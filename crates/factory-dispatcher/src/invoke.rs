//! Plugin invocation: create a Store, set the epoch deadline and fuel
//! budget, pipe the payload through WASI stdin, run `_start`, capture
//! stdout, and classify the outcome.
//!
//! Everything except the bounded cost enforcement (epoch interruption,
//! fuel consumption) happens synchronously. The per-invocation timeout
//! is honored by the shared [`EpochTicker`]; each invocation just sets
//! its own deadline before calling `_start`.
//!
//! ## S-18.00: EventType enum
//!
//! [`EventType`] is the closed enum of Claude Code harness event types that the
//! dispatcher recognises. It enumerates `PreToolUse`, `PostToolUse`, `PreCompact`,
//! and `PostCompact` as first-class variants (BC-1.15.001 INV1). An unknown-event
//! fallback that silently discards PreCompact/PostCompact is a specification
//! violation.
//!
//! PreCompact/PostCompact routing runs through `main.rs → execute_tiers`. The
//! block-intent decision for PostCompact is suppressed by
//! `EventType::from_event_str().is_advisory_only()` (BC-1.15.001 PC2).
//! `dispatch_precompact` and `dispatch_postcompact` are intentional public symbol
//! anchors for the routing arms; the complete dispatch path lives in main.rs.

use std::time::Instant;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use wasmtime::{Engine, Module, Store, Trap};
use wasmtime_wasi::p1::{self, WasiP1Ctx};
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};
use wasmtime_wasi::{DirPerms, FilePerms, I32Exit, WasiCtxBuilder};

use crate::engine::timeout_ms_to_epochs;
use crate::host::{HostContext, setup_linker};

// ---------------------------------------------------------------------------
// S-18.00: EventType enum (BC-1.15.001 INV1)
//
// Closed enum of harness event types the dispatcher recognises. All event
// types — including PreCompact and PostCompact added by S-18.00 — MUST be
// enumerated here rather than handled via string fallback. An unknown-event
// fallback path that silently discards PreCompact/PostCompact is a
// specification violation (BC-1.15.001 INV1).
//
// The complete dispatch path for PreCompact/PostCompact runs through
// main.rs → execute_tiers. `dispatch_precompact` and `dispatch_postcompact`
// are public symbol anchors for the routing arms (BC-1.15.001 PC1/PC2/PC3/PC4/PC5).
// ---------------------------------------------------------------------------

/// Closed enum of harness event types the dispatcher routes.
///
/// Adding a new event type to the Claude Code harness protocol requires a
/// new variant here — string-fallback dispatch is a specification violation
/// per BC-1.15.001 INV1.
///
/// # S-18.00 additions
///
/// `PreCompact` and `PostCompact` are added as first-class variants alongside
/// the existing `PreToolUse` and `PostToolUse` (BC-1.15.001 INV1).
/// `PostCompact` is advisory-only at the harness level: the dispatcher
/// propagates exit codes but never sets `block_intent=true` for PostCompact
/// (BC-1.15.001 PC2).
///
/// # Serde
///
/// `EventType` can be round-tripped from the `event_name` / `event` string
/// fields in the harness payload and hooks-registry.toml via
/// [`EventType::from_event_str`] and [`EventType::as_str`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum EventType {
    /// Claude Code pre-tool-use hook event (gates tool execution).
    PreToolUse,
    /// Claude Code post-tool-use hook event (advisory / observability).
    PostToolUse,
    /// Claude Code pre-compact hook event (gates context compaction).
    /// Introduced in harness >= v2.1.105. Block-intent propagation is
    /// supported (BC-1.15.001 PC1/PC4).
    PreCompact,
    /// Claude Code post-compact hook event (advisory-only; no block-intent).
    /// Introduced in harness >= v2.1.105 (BC-1.15.001 PC2).
    PostCompact,
    /// Other harness events not enumerated above (session lifecycle, etc.).
    /// The dispatcher routes these identically to `PostToolUse` semantics
    /// (advisory, no block-intent) unless a future story adds a specific arm.
    Other,
}

impl EventType {
    /// Convert this variant to the canonical event-name string used in
    /// hooks-registry.toml `event` fields and harness payload `event_name` fields.
    ///
    /// # GREEN-BY-DESIGN
    ///
    /// Pure match on enum variants; zero branching beyond pattern, no I/O,
    /// no helpers. Body ≤ 3 lines per variant. BC-5.38.002 criteria all satisfied.
    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::PreToolUse => "PreToolUse",
            EventType::PostToolUse => "PostToolUse",
            EventType::PreCompact => "PreCompact",
            EventType::PostCompact => "PostCompact",
            EventType::Other => "Other",
        }
    }

    /// Returns `true` if this event type is advisory-only at the harness level,
    /// meaning the dispatcher MUST NOT propagate `block_intent=true` regardless
    /// of plugin exit codes or `on_error` settings.
    ///
    /// # BC-1.15.001 PC2 (S-18.00)
    ///
    /// `PostCompact` is the only advisory-only event type. The harness does not
    /// honour `block_intent` on PostCompact; attempting to set it would be a
    /// specification violation.
    ///
    /// All other event types (`PreToolUse`, `PostToolUse`, `PreCompact`, `Other`)
    /// return `false` — they support block-intent propagation via `on_error=block`
    /// and exit-code 2 semantics.
    pub fn is_advisory_only(&self) -> bool {
        matches!(self, EventType::PostCompact)
    }

    /// Parse an event-name string from the harness payload or hooks-registry.toml
    /// into an `EventType` variant.
    ///
    /// Returns `EventType::Other` for unrecognised event names so the dispatcher
    /// never silently drops an unknown event — callers can inspect `EventType::Other`
    /// and handle gracefully.
    ///
    /// # BC-1.15.001 INV1
    ///
    /// `"PreCompact"` and `"PostCompact"` are first-class event types; they MUST NOT
    /// return `EventType::Other`. An unknown-event fallback that silently discards
    /// these events is a specification violation.
    pub fn from_event_str(event: &str) -> Self {
        match event {
            "PreToolUse" => EventType::PreToolUse,
            "PostToolUse" => EventType::PostToolUse,
            "PreCompact" => EventType::PreCompact,
            "PostCompact" => EventType::PostCompact,
            _ => EventType::Other,
        }
    }
}

/// Dispatch a `PreCompact` event to a set of matched plugins.
///
/// PreCompact supports block-intent propagation: a plugin that exits 2
/// causes the dispatcher to return `block_intent=true` (BC-1.15.001 PC1/PC4).
/// On-error semantics mirror PreToolUse (BC-1.15.001 PC5):
/// - `on_error = "block"` crash → block_intent=true (fail-closed)
/// - `on_error = "continue"` crash → advisory only (fail-open)
///
/// When no plugins are registered for this event type, returns without error
/// and produces no block intent (BC-1.15.001 PC3).
///
/// # Integration path
///
/// The full dispatch with plugin invocation, priority ordering, and block-intent
/// aggregation lives in `main.rs` → `executor::execute_tiers`. This function is
/// the unit-level anchor for the `EventType::PreCompact` routing arm; the integration
/// path reaches it via `match_plugins` + `execute_tiers` using `event_name = "PreCompact"`.
///
/// # BC-1.15.001 PC1/PC3/PC4/PC5
pub fn dispatch_precompact() {
    // No-op: this function is the named anchor for the PreCompact routing arm.
    // The complete dispatch (plugin invocation, exit-2 aggregation, on_error semantics)
    // runs through main.rs → execute_tiers when `event_name = "PreCompact"` is matched.
    // BC-1.15.001 PC3: zero registered plugins → block_intent=false, exit 0 (no-op correct).
}

/// Dispatch a `PostCompact` event to a set of matched plugins.
///
/// PostCompact is advisory-only at the harness level: the dispatcher invokes
/// registered plugins and propagates exit codes in the response, but NEVER
/// sets `block_intent=true` regardless of plugin exit code (BC-1.15.001 PC2).
///
/// When no plugins are registered for this event type, returns without error
/// (BC-1.15.001 PC3).
///
/// # Integration path
///
/// The full dispatch path runs through `main.rs` → `executor::execute_tiers` when
/// `event_name = "PostCompact"`. The advisory-only constraint (never block_intent)
/// is enforced in `main.rs` by only propagating exit-2 block intent for PreCompact
/// events; PostCompact results are handled as advisory.
///
/// # BC-1.15.001 PC2/PC3
pub fn dispatch_postcompact() {
    // No-op: this function is the named anchor for the PostCompact routing arm.
    // The complete dispatch runs through main.rs → execute_tiers when
    // `event_name = "PostCompact"` is matched.
    // BC-1.15.001 PC2: block_intent is NEVER set for PostCompact (advisory-only).
    // BC-1.15.001 PC3: zero registered plugins → no-op (correct).
}

/// Outcome of a single `invoke_plugin` call.
///
/// `fuel_consumed` is always populated so operators can see how close
/// normal plugins came to their budget. `elapsed_ms` is wall-clock from
/// just before `_start` to just after the result is classified.
///
/// `stderr` is the plugin's WASI stderr captured during invocation,
/// truncated to [`STDERR_CAP_BYTES`]. It's the diagnostic signal the
/// dispatcher emits on `plugin.completed` / `plugin.crashed` /
/// `plugin.timeout` events; without it, operators see only `exit_code`
/// and have to re-run with a manual capture to find out why a plugin
/// exited 1. Field added in v1.0.0-beta.4 after the S-2.7 dogfood loop
/// ran into exactly that diagnostic gap.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginResult {
    Ok {
        /// Process exit code the plugin's `fn main()` returned.
        exit_code: i32,
        /// Raw stdout (UTF-8 `HookResult` JSON in well-behaved plugins).
        stdout: String,
        /// Captured stderr, truncated to STDERR_CAP_BYTES.
        stderr: String,
        elapsed_ms: u64,
        fuel_consumed: u64,
    },
    Timeout {
        cause: TimeoutCause,
        /// Captured stderr, truncated to STDERR_CAP_BYTES. May contain
        /// a partial message because the plugin was interrupted.
        stderr: String,
        elapsed_ms: u64,
        /// Instructions executed before the interrupt. Useful telemetry for
        /// the event log. Note: on `Trap::OutOfFuel`, `fuel_consumed_from_store`
        /// returns `cap.saturating_sub(remaining)` where `remaining == 0`, so
        /// `fuel_consumed == fuel_cap` on every fuel trap — it does not convey
        /// demand independently of the cap. Use `fuel_cap` for the operator
        /// message when communicating what limit was hit.
        fuel_consumed: u64,
        /// The configured fuel budget at the time of invocation. Always
        /// reflects the actual cap set on the wasmtime Store, independent of
        /// whether `get_fuel()` succeeds. Use this field — not `fuel_consumed`
        /// — when constructing operator-facing messages about which cap to raise.
        fuel_cap: u64,
    },
    Crashed {
        trap_string: String,
        /// Captured stderr, truncated to STDERR_CAP_BYTES. Often the
        /// most useful field for diagnosing a crash since wasmtime's
        /// trap_string only surfaces the trap kind, not the plugin's
        /// own pre-trap diagnostics.
        stderr: String,
        elapsed_ms: u64,
        fuel_consumed: u64,
    },
}

/// Truncation cap on per-plugin stderr captured into `PluginResult`.
/// Operators see this value on `plugin.completed`/`plugin.crashed`/
/// `plugin.timeout` events. 4 KiB is generous for diagnostic lines
/// while keeping the internal-log per-event payload bounded.
pub const STDERR_CAP_BYTES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeoutCause {
    /// Wall-clock budget exhausted (epoch interruption fired).
    Epoch,
    /// Fuel budget exhausted.
    Fuel,
}

/// ADR-042 §Decision 1: global fuel cap raised 10M → 20M (measurement-validated).
/// Worst-case legacy-bash-adapter payload consumed 10,406,058 fuel
/// (ARCH-INDEX + 50 KB last_assistant_message, 377,109 payload bytes),
/// exhausting the former 10M cap. 20M leaves ~9.6M headroom (92% margin).
///
/// This constant is the single source of truth for the ADR-042 fuel cap.
/// Both `InvokeLimits::default()` and `RegistryDefaults::default()` reference it,
/// so any future deliberate cap change is made in one place and propagates
/// atomically to both — structural enforcement rather than test-only enforcement.
/// Any change to this value requires updating the ADR-042 §Decision 1 rationale.
pub const DEFAULT_FUEL_CAP: u64 = 20_000_000;

/// Per-invocation budget. Defaults live in
/// `RegistryDefaults`; callers usually get these from a
/// `RegistryEntry` with fallback.
#[derive(Debug, Clone, Copy)]
pub struct InvokeLimits {
    pub timeout_ms: u32,
    pub fuel_cap: u64,
}

impl Default for InvokeLimits {
    fn default() -> Self {
        Self {
            timeout_ms: 5_000,
            fuel_cap: DEFAULT_FUEL_CAP,
        }
    }
}

#[derive(Debug, Error)]
pub enum InvokeError {
    #[error("wasmtime store setup failed: {0}")]
    Setup(String),
    #[error("linker instantiation failed: {0}")]
    Instantiate(String),
    #[error("plugin has no `_start` export")]
    MissingStart,
    #[error("host linker build failed: {0}")]
    HostLinker(String),
    #[error("wasi setup failed: {0}")]
    Wasi(String),
}

/// Invoke a pre-compiled plugin module against the given
/// [`HostContext`]. The `payload_json` is written to the plugin's
/// stdin; the plugin is expected to write a `HookResult` JSON line to
/// stdout, which the caller is responsible for parsing.
/// Invoke a plugin and return the raw result plus the per-invocation
/// `host_output_too_large_seen` flag (S-25.01 AC-003).
///
/// The `bool` is `true` iff any host function returned `OUTPUT_TOO_LARGE(-3)`
/// during this invocation. Callers pass it to `classify_outcome` to detect
/// the Indeterminate(OutputTooLarge) case (BC-1.18.001 invariant 5).
///
/// The flag is only meaningful when `PluginResult::Ok { exit_code: 0, .. }`;
/// for Timeout and Crashed results it is always `false`.
pub fn invoke_plugin(
    engine: &Engine,
    module: &Module,
    host_ctx: HostContext,
    payload_json: &[u8],
    limits: InvokeLimits,
) -> Result<(PluginResult, bool), InvokeError> {
    // Set up wasmtime store with both host context and WASI context.
    // We use a wrapper type so both live in the store's data slot.
    let stdout = MemoryOutputPipe::new(64 * 1024);
    let stderr = MemoryOutputPipe::new(64 * 1024);

    // Preopen the project directory (host_ctx.cwd) as "." in the WASI guest.
    // This enables std::fs operations from WASM plugins that perform filesystem
    // I/O relative to the project root (e.g. session-learning appending to
    // .factory/sidecar-learning.md). Plugins without filesystem needs are
    // unaffected — they simply ignore the preopened handle.
    // If the cwd path cannot be opened (e.g. missing dir in tests), the WASI
    // context is built without a preopen and std::fs calls will return EBADF.
    let mut wasi_builder = WasiCtxBuilder::new();
    wasi_builder
        .stdin(MemoryInputPipe::new(payload_json.to_vec()))
        .stdout(stdout.clone())
        .stderr(stderr.clone());
    if host_ctx.cwd.as_os_str().is_empty() {
        // No project dir — build without filesystem preopen.
    } else if let Err(e) = wasi_builder.preopened_dir(
        // W-15 wave gate (SEC-001 / CRIT-W15-003): WASI preopens grant
        // DirPerms::all() | FilePerms::all() to plugins. This is the sandbox
        // boundary; capability-gated host functions (e.g., write_file) provide
        // ADDITIONAL bounded mechanisms but do not constrain native WASI calls.
        // See crates/hook-sdk/HOST_ABI.md "Filesystem Access Model". v1.1 will
        // tighten preopens to read-only by default with explicit write capability
        // declarations.
        &host_ctx.cwd,
        ".",
        DirPerms::all(),
        FilePerms::all(),
    ) {
        // Non-fatal: log and continue without filesystem access.
        // Plugin may still function if it doesn't need std::fs.
        tracing::debug!(
            cwd = %host_ctx.cwd.display(),
            err = %e,
            "wasi preopen failed; plugin std::fs calls will fail"
        );
    }
    let wasi_ctx = wasi_builder.build_p1();

    let store_data = StoreData {
        host: host_ctx,
        wasi: wasi_ctx,
        // S-25.01 AC-003: per-invocation reset happens before func.call() (not only here).
        // This initialization sets the starting state; the implementer adds the pre-call reset.
        host_output_too_large_seen: false,
    };
    let mut store = Store::new(engine, store_data);

    // Wall-clock budget → epoch deadline.
    store.set_epoch_deadline(timeout_ms_to_epochs(limits.timeout_ms as u64));
    store
        .set_fuel(limits.fuel_cap)
        .map_err(|e| InvokeError::Setup(e.to_string()))?;

    // Build per-invocation linker: host imports (S-1.4) + WASI.
    let host_linker = setup_linker(engine).map_err(|e| InvokeError::HostLinker(e.to_string()))?;
    let mut linker: wasmtime::Linker<StoreData> = wasmtime::Linker::new(engine);
    // Copy host imports from the HostContext-typed linker into our
    // StoreData-typed linker. For every Extern the host linker holds,
    // register a proxy that dereferences store_data.host.
    proxy_host_imports(&mut linker, &host_linker, engine)
        .map_err(|e| InvokeError::HostLinker(e.to_string()))?;
    p1::add_to_linker_sync(&mut linker, |d: &mut StoreData| &mut d.wasi)
        .map_err(|e| InvokeError::Wasi(e.to_string()))?;

    let instance = linker
        .instantiate(&mut store, module)
        .map_err(|e| InvokeError::Instantiate(e.to_string()))?;

    let start_export = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .map_err(|_| InvokeError::MissingStart)?;

    // S-25.01 AC-003: per-invocation reset immediately before func.call().
    // A new Store is constructed per invocation so the field is already false,
    // but we reset explicitly here to make the invariant unambiguous and to
    // guard against any future Store-reuse refactors.
    store.data_mut().host_output_too_large_seen = false;

    let started = Instant::now();
    let call_result = start_export.call(&mut store, ());
    let elapsed_ms = started.elapsed().as_millis() as u64;
    let fuel_consumed = fuel_consumed_from_store(&store, limits.fuel_cap);

    // S-25.01 AC-003: capture the flag AFTER func.call() (before next reset).
    // This is the authoritative read for the per-invocation OutputTooLarge state.
    let output_too_large = store.data().host_output_too_large_seen;

    // WASI command convention: `_start` returns () on exit(0); any
    // other exit code arrives as a trap whose downcast yields an
    // `I32Exit(code)`.
    match call_result {
        Ok(()) => {
            let out = stdout_to_string(&stdout);
            let err_text = stderr_to_string(&stderr);
            Ok((
                PluginResult::Ok {
                    exit_code: 0,
                    stdout: out,
                    stderr: err_text,
                    elapsed_ms,
                    fuel_consumed,
                },
                output_too_large,
            ))
        }
        Err(err) => classify_trap(
            anyhow::Error::from(err),
            &stdout,
            &stderr,
            elapsed_ms,
            fuel_consumed,
            limits.fuel_cap,
        )
        // For Timeout/Crashed results, output_too_large is irrelevant:
        // classify_outcome ignores the flag for non-Ok results (BC-1.18.001).
        .map(|r| (r, false)),
    }
}

fn classify_trap(
    err: anyhow::Error,
    stdout: &MemoryOutputPipe,
    stderr: &MemoryOutputPipe,
    elapsed_ms: u64,
    fuel_consumed: u64,
    fuel_cap: u64,
) -> Result<PluginResult, InvokeError> {
    let stderr_text = stderr_to_string(stderr);
    // WASI `exit(n)` propagates as an `I32Exit` in wasmtime-wasi's
    // preview-1 glue; non-zero exit is still "Ok" from our POV since
    // the plugin ran to a controlled finish.
    if let Some(exit) = err.downcast_ref::<I32Exit>() {
        let out = stdout_to_string(stdout);
        return Ok(PluginResult::Ok {
            exit_code: exit.0,
            stdout: out,
            stderr: stderr_text,
            elapsed_ms,
            fuel_consumed,
        });
    }
    if let Some(trap) = err.downcast_ref::<Trap>() {
        return Ok(match trap {
            Trap::Interrupt => PluginResult::Timeout {
                cause: TimeoutCause::Epoch,
                stderr: stderr_text,
                elapsed_ms,
                fuel_consumed,
                fuel_cap,
            },
            Trap::OutOfFuel => PluginResult::Timeout {
                cause: TimeoutCause::Fuel,
                stderr: stderr_text,
                elapsed_ms,
                fuel_consumed,
                fuel_cap,
            },
            other => PluginResult::Crashed {
                trap_string: other.to_string(),
                stderr: stderr_text,
                elapsed_ms,
                fuel_consumed,
            },
        });
    }
    // Any other wasmtime error surfaces as Crashed — preserves the
    // exact diagnostic for operators.
    Ok(PluginResult::Crashed {
        trap_string: format!("{err:#}"),
        stderr: stderr_text,
        elapsed_ms,
        fuel_consumed,
    })
}

fn fuel_consumed_from_store(store: &Store<StoreData>, cap: u64) -> u64 {
    match store.get_fuel() {
        Ok(remaining) => cap.saturating_sub(remaining),
        Err(_) => 0,
    }
}

/// Read the captured stderr pipe and truncate to STDERR_CAP_BYTES so
/// the per-event payload stays bounded. Truncation appends an ellipsis
/// marker so operators can see they only have a partial view.
fn stderr_to_string(pipe: &MemoryOutputPipe) -> String {
    let bytes = pipe.contents();
    if bytes.len() <= STDERR_CAP_BYTES {
        return String::from_utf8_lossy(&bytes).into_owned();
    }
    let mut s = String::from_utf8_lossy(&bytes[..STDERR_CAP_BYTES]).into_owned();
    s.push_str("\n…(stderr truncated)");
    s
}

fn stdout_to_string(pipe: &MemoryOutputPipe) -> String {
    let bytes = pipe.contents();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Bridge from the `Linker<HostContext>` built by S-1.4 to a
/// `Linker<StoreData>` the invoke path uses. The bridge re-registers
/// each host import under the StoreData data type by wrapping it in a
/// proxy that delegates into `store_data.host`.
///
/// In practice we just rebuild the linker from scratch here — wasmtime
/// doesn't support cloning Func between different Store types. The
/// S-1.4 `setup_linker` exists for the integration-test story; at
/// runtime the invoke path owns its own linker build.
fn proxy_host_imports(
    linker: &mut wasmtime::Linker<StoreData>,
    _host_linker_reference: &wasmtime::Linker<HostContext>,
    engine: &Engine,
) -> Result<(), String> {
    // Keep the parameter signature stable so the call site from
    // `invoke_plugin` reads naturally — swap the internal
    // implementation to re-register against the StoreData data type.
    let _ = engine;
    setup_host_on_store_data(linker).map_err(|e| e.to_string())
}

/// Re-register every `vsdd::*` host function directly against a
/// `Linker<StoreData>`. Semantics mirror the `HostContext`-flavored
/// registrations in `host/` submodules, but access host state through
/// `store.data().host`.
fn setup_host_on_store_data(
    linker: &mut wasmtime::Linker<StoreData>,
) -> Result<(), crate::host::HostCallError> {
    use crate::host::HostCallError;
    use crate::host::codes;
    use crate::internal_log::InternalEvent;
    use serde_json::{Map, Value};
    use wasmtime::Caller;

    // log
    linker
        .func_wrap(
            "vsdd",
            "log",
            |mut caller: Caller<'_, StoreData>, level: u32, msg_ptr: u32, msg_len: u32| {
                if let Ok(msg) = read_wasm_string_sd(&mut caller, msg_ptr, msg_len) {
                    let level_str = match level {
                        0 => "trace",
                        1 => "debug",
                        2 => "info",
                        3 => "warn",
                        4 => "error",
                        _ => "info",
                    };
                    let host = &caller.data().host;
                    let ev = InternalEvent::now("plugin.log")
                        .with_trace_id(&host.dispatcher_trace_id)
                        .with_session_id(&host.session_id)
                        .with_plugin_name(&host.plugin_name)
                        .with_plugin_version(&host.plugin_version)
                        .with_field("level", Value::String(level_str.to_string()))
                        .with_field("message", Value::String(msg));
                    host.emit_internal(ev);
                }
            },
        )
        .map_err(|e| HostCallError::Linker(e.to_string()))?;

    // emit_event
    linker
        .func_wrap(
            "vsdd",
            "emit_event",
            |mut caller: Caller<'_, StoreData>,
             type_ptr: u32,
             type_len: u32,
             fields_ptr: u32,
             fields_len: u32| {
                let event_type = match read_wasm_string_sd(&mut caller, type_ptr, type_len) {
                    Ok(s) => s,
                    Err(_) => return,
                };
                let fields_buf = match read_wasm_bytes_sd(&mut caller, fields_ptr, fields_len) {
                    Ok(b) => b,
                    Err(_) => return,
                };
                let pairs = crate::host::emit_event::decode_fields(&fields_buf).unwrap_or_default();
                let host = &caller.data().host;
                let mut ev = InternalEvent::now(&event_type)
                    .with_trace_id(&host.dispatcher_trace_id)
                    .with_session_id(&host.session_id)
                    .with_plugin_name(&host.plugin_name)
                    .with_plugin_version(&host.plugin_version);
                for (k, v) in pairs {
                    // Reserved fields: plugins cannot override dispatcher-owned identity fields.
                    // BC-3.08.001 v1.7 Invariant 5: both "trace_id" (canonical wire name) and
                    // "dispatcher_trace_id" (legacy, defense-in-depth) are reserved.
                    if [
                        "trace_id",
                        "dispatcher_trace_id",
                        "session_id",
                        "plugin_name",
                        "plugin_version",
                        "ts",
                        "ts_epoch",
                        "schema_version",
                        "type",
                    ]
                    .contains(&k.as_str())
                    {
                        continue;
                    }
                    ev = ev.with_field(&k, Value::String(v));
                }
                host.emit_internal(ev);
            },
        )
        .map_err(|e| HostCallError::Linker(e.to_string()))?;

    // context getters: session_id / dispatcher_trace_id / plugin_root / plugin_version / cwd
    linker
        .func_wrap(
            "vsdd",
            "session_id",
            context_reader(|h| h.session_id.clone()),
        )
        .map_err(|e| HostCallError::Linker(e.to_string()))?;
    linker
        .func_wrap(
            "vsdd",
            "dispatcher_trace_id",
            context_reader(|h| h.dispatcher_trace_id.clone()),
        )
        .map_err(|e| HostCallError::Linker(e.to_string()))?;
    linker
        .func_wrap(
            "vsdd",
            "plugin_root",
            context_reader(|h| h.plugin_root.to_string_lossy().into_owned()),
        )
        .map_err(|e| HostCallError::Linker(e.to_string()))?;
    linker
        .func_wrap(
            "vsdd",
            "plugin_version",
            context_reader(|h| h.plugin_version.clone()),
        )
        .map_err(|e| HostCallError::Linker(e.to_string()))?;
    linker
        .func_wrap(
            "vsdd",
            "cwd",
            context_reader(|h| h.cwd.to_string_lossy().into_owned()),
        )
        .map_err(|e| HostCallError::Linker(e.to_string()))?;

    // env — simplified; full capability check lives in crate::host::env
    linker
        .func_wrap(
            "vsdd",
            "env",
            |mut caller: Caller<'_, StoreData>,
             name_ptr: u32,
             name_len: u32,
             out_ptr: u32,
             out_cap: u32|
             -> i32 {
                let name = match read_wasm_string_sd(&mut caller, name_ptr, name_len) {
                    Ok(s) => s,
                    Err(_) => return codes::INVALID_ARGUMENT,
                };
                let host = &caller.data().host;
                if !host.capabilities.env_allow.iter().any(|n| n == &name) {
                    let mut details = Map::new();
                    details.insert("variable".into(), Value::String(name.clone()));
                    host.emit_internal(host.denial_event("env", "env_not_on_allow_list", details));
                    return codes::CAPABILITY_DENIED;
                }
                let value = host.env_view.get(&name).cloned();
                match value {
                    None => 0,
                    Some(v) => {
                        let bytes = v.into_bytes();
                        match write_wasm_bytes_sd(&mut caller, out_ptr, out_cap, &bytes) {
                            Ok(n) => n as i32,
                            Err(_) => codes::INVALID_ARGUMENT,
                        }
                    }
                }
            },
        )
        .map_err(|e| HostCallError::Linker(e.to_string()))?;

    // read_file: real implementation. Uses output-pointer protocol:
    // the host reads the file, grows WASM memory to hold the bytes,
    // writes them there, then writes the address and length back to
    // the guest-provided out-param pointers.
    //
    // S-8.07: first in-tree plugin (warn-pending-wave-gate) to use this path.
    linker
        .func_wrap(
            "vsdd",
            "read_file",
            |mut caller: Caller<'_, StoreData>,
             path_ptr: u32,
             path_len: u32,
             max_bytes: u32,
             _timeout_ms: u32,
             out_ptr_out: u32,
             out_len_out: u32|
             -> i32 {
                let path = match read_wasm_string_sd(&mut caller, path_ptr, path_len) {
                    Ok(s) => s,
                    Err(_) => return codes::INVALID_ARGUMENT,
                };

                // Capability check + file read (host-side logic, no WASM memory).
                let body = {
                    let ctx = caller.data().host.clone();
                    match crate::host::read_file::prepare(&ctx, &path, max_bytes) {
                        Ok((bytes, _)) => bytes,
                        Err(code) => {
                            // S-25.01 AC-003: flag OutputTooLarge for the
                            // post-call classify_outcome path.
                            if code == codes::OUTPUT_TOO_LARGE {
                                caller.data_mut().host_output_too_large_seen = true;
                            }
                            return code;
                        }
                    }
                };

                if body.is_empty() {
                    // Empty file: write ptr=0, len=0.  SDK read_owned_bytes guards
                    // ptr==0 → returns Vec::new(), which is correct for empty files.
                    let _ = write_wasm_u32_sd(&mut caller, out_ptr_out, 0);
                    let _ = write_wasm_u32_sd(&mut caller, out_len_out, 0);
                    return codes::OK;
                }

                // Find the current end of WASM linear memory, then grow by
                // enough pages to hold `body`.  Writing at the old end gives
                // us a valid, unused address (the SDK copies the bytes
                // immediately via `read_owned_bytes`, so the page is never
                // reused for anything else during this call).
                let memory = match get_memory_sd(&mut caller) {
                    Ok(m) => m,
                    Err(_) => return codes::INTERNAL_ERROR,
                };
                let current_bytes = memory.data_size(&caller);
                let pages_needed = body.len().div_ceil(65536) as u64;
                if memory.grow(&mut caller, pages_needed).is_err() {
                    return codes::INTERNAL_ERROR;
                }

                let write_offset = current_bytes as u32;

                // Write file bytes at the newly allocated offset.
                // `out_cap` = body.len() because we just grew enough memory.
                if write_wasm_bytes_sd(&mut caller, write_offset, body.len() as u32, &body).is_err()
                {
                    return codes::INTERNAL_ERROR;
                }

                // Return (ptr, len) to the guest via the out-params.
                if write_wasm_u32_sd(&mut caller, out_ptr_out, write_offset).is_err() {
                    return codes::INVALID_ARGUMENT;
                }
                if write_wasm_u32_sd(&mut caller, out_len_out, body.len() as u32).is_err() {
                    return codes::INVALID_ARGUMENT;
                }
                codes::OK
            },
        )
        .map_err(|e| HostCallError::Linker(e.to_string()))?;

    // read_prefix: bounded partial read (head-c semantics). Uses the same
    // output-pointer protocol as read_file: grow WASM linear memory by
    // ceil(body_len / 65536) pages, write body at current_bytes (always > 0
    // for non-empty files), and return the write offset via out_ptr_out.
    // Capability enforcement delegates to crate::host::read_prefix::prepare,
    // which checks ctx.capabilities.read_prefix.path_allow (deny-by-default;
    // independent of read_file capability per BC-1.17.001 Invariant 3).
    //
    // ADR-025 §Decision 16: production-path fill for vsdd::read_prefix.
    // S-19.06 registered read_prefix in setup_linker (Linker<HostContext>,
    // test path in host/mod.rs). This block makes it available on the
    // Linker<StoreData> production dispatch path called by proxy_host_imports.
    linker
        .func_wrap(
            "vsdd",
            "read_prefix",
            |mut caller: Caller<'_, StoreData>,
             path_ptr: u32,
             path_len: u32,
             max_bytes: u32,
             _timeout_ms: u32,
             out_ptr_out: u32,
             out_len_out: u32|
             -> i32 {
                let path = match read_wasm_string_sd(&mut caller, path_ptr, path_len) {
                    Ok(s) => s,
                    Err(_) => return codes::INVALID_ARGUMENT,
                };

                // Capability check + bounded file read (host-side logic, no WASM memory).
                let body = {
                    let ctx = caller.data().host.clone();
                    match crate::host::read_prefix::prepare(&ctx, &path, max_bytes) {
                        Ok((bytes, _)) => bytes,
                        Err(code) => return code,
                    }
                };

                if body.is_empty() {
                    // Empty file or max_bytes=0: write ptr=0, len=0.  SDK
                    // read_owned_bytes guards ptr==0 → returns Vec::new().
                    let _ = write_wasm_u32_sd(&mut caller, out_ptr_out, 0);
                    let _ = write_wasm_u32_sd(&mut caller, out_len_out, 0);
                    return codes::OK;
                }

                // Find the current end of WASM linear memory, then grow by
                // enough pages to hold `body`.  Writing at current_bytes gives
                // a valid, unused address (> 0 for any non-empty WASM module).
                let memory = match get_memory_sd(&mut caller) {
                    Ok(m) => m,
                    Err(_) => return codes::INTERNAL_ERROR,
                };
                let current_bytes = memory.data_size(&caller);
                let pages_needed = body.len().div_ceil(65536) as u64;
                if memory.grow(&mut caller, pages_needed).is_err() {
                    return codes::INTERNAL_ERROR;
                }

                let write_offset = current_bytes as u32;

                // Write prefix bytes at the newly allocated offset.
                if write_wasm_bytes_sd(&mut caller, write_offset, body.len() as u32, &body).is_err()
                {
                    return codes::INTERNAL_ERROR;
                }

                // Return (ptr, len) to the guest via the out-params.
                if write_wasm_u32_sd(&mut caller, out_ptr_out, write_offset).is_err() {
                    return codes::INVALID_ARGUMENT;
                }
                if write_wasm_u32_sd(&mut caller, out_len_out, body.len() as u32).is_err() {
                    return codes::INVALID_ARGUMENT;
                }
                codes::OK
            },
        )
        .map_err(|e| HostCallError::Linker(e.to_string()))?;

    // write_file: real implementation rooted at cwd (CLAUDE_PROJECT_DIR).
    // Relative paths in the request and path_allow are resolved against
    // ctx.cwd so plugins can write project-relative files (e.g.
    // `.factory/wave-state.yaml`). Uses input-pointer protocol: the host
    // reads the byte slice from guest memory (read_wasm_bytes protocol).
    // First consumer: update-wave-state-on-merge (S-8.04 BC-7.03.085/086).
    linker
        .func_wrap(
            "vsdd",
            "write_file",
            |mut caller: Caller<'_, StoreData>,
             path_ptr: u32,
             path_len: u32,
             contents_ptr: u32,
             contents_len: u32,
             max_bytes: u32,
             _timeout_ms: u32|
             -> i32 {
                let path = match read_wasm_string_sd(&mut caller, path_ptr, path_len) {
                    Ok(s) => s,
                    Err(_) => return codes::INVALID_ARGUMENT,
                };
                let contents = match read_wasm_bytes_sd(&mut caller, contents_ptr, contents_len) {
                    Ok(b) => b,
                    Err(_) => return codes::INVALID_ARGUMENT,
                };
                // Delegate to the shared host-side gate (F-S1903-P6-001 dedup).
                // Mirrors the read_file delegation above: `prepare` enforces
                // deny-by-default capability check, path resolution, byte cap,
                // allowlist check (shared path_util::check_path_allowed), and
                // emit_denial with the correct reason tokens
                // (path_resolution_failed / path_not_allowed / output_too_large).
                // BC-2.02.011 postconditions 1-5.
                let host = caller.data().host.clone();
                match crate::host::write_file::prepare(&host, &path, &contents, max_bytes) {
                    Ok(()) => codes::OK,
                    Err(code) => {
                        // S-25.01 AC-003: flag OutputTooLarge for the
                        // post-call classify_outcome path.
                        if code == codes::OUTPUT_TOO_LARGE {
                            caller.data_mut().host_output_too_large_seen = true;
                        }
                        code
                    }
                }
            },
        )
        .map_err(|e| HostCallError::Linker(e.to_string()))?;

    // exec_subprocess: real implementation that delegates to the
    // crate::host::exec_subprocess policy + executor. The legacy-bash-
    // adapter (S-2.1) needs this path live so it can shell out to bash
    // hooks; tests in crate::host::exec_subprocess cover the policy.
    linker
        .func_wrap(
            "vsdd",
            "exec_subprocess",
            |mut caller: Caller<'_, StoreData>,
             cmd_ptr: u32,
             cmd_len: u32,
             args_ptr: u32,
             args_len: u32,
             stdin_ptr: u32,
             stdin_len: u32,
             timeout_ms: u32,
             max_output_bytes: u32,
             result_buf_ptr: u32,
             result_buf_cap: u32|
             -> i32 {
                let cmd = match read_wasm_string_sd(&mut caller, cmd_ptr, cmd_len) {
                    Ok(s) => s,
                    Err(_) => return codes::INVALID_ARGUMENT,
                };
                let args_buf = match read_wasm_bytes_sd(&mut caller, args_ptr, args_len) {
                    Ok(b) => b,
                    Err(_) => return codes::INVALID_ARGUMENT,
                };
                let args = match crate::host::exec_subprocess::decode_args(&args_buf) {
                    Some(a) => a,
                    None => return codes::INVALID_ARGUMENT,
                };
                let stdin_bytes = if stdin_len == 0 {
                    Vec::new()
                } else {
                    match read_wasm_bytes_sd(&mut caller, stdin_ptr, stdin_len) {
                        Ok(b) => b,
                        Err(_) => return codes::INVALID_ARGUMENT,
                    }
                };

                let envelope = match crate::host::exec_subprocess::run(
                    &caller.data().host,
                    &cmd,
                    &args,
                    &stdin_bytes,
                    timeout_ms,
                    max_output_bytes,
                ) {
                    Ok(env) => env,
                    Err(code) => return code,
                };

                // Write the envelope into the guest-provided buffer.
                // Returns bytes written (positive) on success or a
                // negative error code. Mirrors host/exec_subprocess.rs.
                if envelope.len() as u32 > result_buf_cap {
                    // S-25.01 AC-003: flag OutputTooLarge for the
                    // post-call classify_outcome path.
                    caller.data_mut().host_output_too_large_seen = true;
                    return codes::OUTPUT_TOO_LARGE;
                }
                match write_wasm_bytes_sd(&mut caller, result_buf_ptr, result_buf_cap, &envelope) {
                    Ok(written) => written as i32,
                    Err(_) => codes::INVALID_ARGUMENT,
                }
            },
        )
        .map_err(|e| HostCallError::Linker(e.to_string()))?;

    Ok(())
}

fn context_reader<F>(
    extract: F,
) -> impl Fn(Caller<'_, StoreData>, u32, u32) -> u32 + Send + Sync + 'static + Copy
where
    F: Fn(&HostContext) -> String + Send + Sync + Copy + 'static,
{
    move |mut caller: Caller<'_, StoreData>, out_ptr: u32, out_cap: u32| -> u32 {
        let s = extract(&caller.data().host);
        let bytes = s.into_bytes();
        write_wasm_bytes_sd(&mut caller, out_ptr, out_cap, &bytes).unwrap_or_default()
    }
}

// StoreData-typed memory helpers — mirror host::memory but against the
// wider `StoreData` so the invoke linker can share them.
use wasmtime::Caller;

use crate::host::HostCallError;

fn get_memory_sd(caller: &mut Caller<'_, StoreData>) -> Result<wasmtime::Memory, HostCallError> {
    caller
        .get_export("memory")
        .and_then(|e| e.into_memory())
        .ok_or(HostCallError::MissingMemory)
}

fn read_wasm_bytes_sd(
    caller: &mut Caller<'_, StoreData>,
    ptr: u32,
    len: u32,
) -> Result<Vec<u8>, HostCallError> {
    let memory = get_memory_sd(caller)?;
    let data = memory.data(&caller);
    let start = ptr as usize;
    let end = start
        .checked_add(len as usize)
        .ok_or(HostCallError::MemoryOverflow)?;
    if end > data.len() {
        return Err(HostCallError::OutOfBounds {
            ptr,
            len,
            memory_size: data.len(),
        });
    }
    Ok(data[start..end].to_vec())
}

fn read_wasm_string_sd(
    caller: &mut Caller<'_, StoreData>,
    ptr: u32,
    len: u32,
) -> Result<String, HostCallError> {
    let bytes = read_wasm_bytes_sd(caller, ptr, len)?;
    String::from_utf8(bytes).map_err(|_| HostCallError::InvalidUtf8)
}

fn write_wasm_bytes_sd(
    caller: &mut Caller<'_, StoreData>,
    out_ptr: u32,
    out_cap: u32,
    bytes: &[u8],
) -> Result<u32, HostCallError> {
    let needed = bytes.len() as u32;
    if needed > out_cap {
        return Ok(needed);
    }
    let memory = get_memory_sd(caller)?;
    let start = out_ptr as usize;
    let end = start
        .checked_add(bytes.len())
        .ok_or(HostCallError::MemoryOverflow)?;
    let data_len = memory.data(&caller).len();
    if end > data_len {
        return Err(HostCallError::OutOfBounds {
            ptr: out_ptr,
            len: needed,
            memory_size: data_len,
        });
    }
    memory
        .write(caller, start, bytes)
        .map_err(|_| HostCallError::OutOfBounds {
            ptr: out_ptr,
            len: needed,
            memory_size: data_len,
        })?;
    Ok(needed)
}

/// Write a single little-endian u32 into guest memory.
/// Used for `read_file`'s out-param protocol (`out_ptr_out`, `out_len_out`).
fn write_wasm_u32_sd(
    caller: &mut Caller<'_, StoreData>,
    out_ptr: u32,
    value: u32,
) -> Result<(), HostCallError> {
    let bytes = value.to_le_bytes();
    write_wasm_bytes_sd(caller, out_ptr, bytes.len() as u32, &bytes)?;
    Ok(())
}

/// Per-invocation store data: the HostContext S-1.4 populates plus the
/// wasmtime-wasi preview-1 context the SDK needs to talk to stdin /
/// stdout.
///
/// # S-25.01: host_output_too_large_seen (BC-1.18.001 invariant 5)
///
/// `host_output_too_large_seen` is DISPATCHER-INTERNAL state — it MUST reside here
/// and MUST NOT be exposed in `crates/hook-sdk/` or any public hook-sdk ABI type.
/// Plugin WASM binaries observe the `OutputTooLarge(-3)` return code from host
/// functions but cannot read or write this flag directly.
///
/// Per-invocation reset invariant (BC-1.18.001 invariant 1; AC-003): the flag
/// MUST be reset to `false` immediately BEFORE each `func.call()` invocation —
/// NOT only at Store creation. The reset ensures that a prior invocation's OTL
/// event does not bleed into the next invocation's classification.
///
/// No HOST_ABI_VERSION bump is required for this field (dispatcher-internal only).
pub struct StoreData {
    pub host: HostContext,
    pub wasi: WasiP1Ctx,
    /// Set to `true` by any host function that returns `OutputTooLarge(-3)` to
    /// the WASM guest during the current invocation. Reset to `false` before
    /// each `func.call()`. Read after `func.call()` to supply the `output_too_large`
    /// parameter to `classify_outcome`.
    ///
    /// S-25.01 AC-003/AC-018 — BC-1.18.001 invariant 1 + invariant 5.
    pub host_output_too_large_seen: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::build_engine;

    fn compile(engine: &Engine, wat: &str) -> Module {
        let bytes = wat::parse_str(wat).expect("wat should parse");
        Module::from_binary(engine, &bytes).expect("module should compile")
    }

    fn bare_ctx() -> HostContext {
        HostContext::new("plugin", "0.0.1", "sess", "trace")
    }

    // ADR-042 §Decision 1: global fuel cap raised 10M → 20M (measurement-validated).
    // 838 fuel-exhaustion events across 35 plugins confirmed by perf-engineer;
    // worst-case payload (ARCH-INDEX + 50 KB assistant message) measured 10,406,058 fuel.
    #[test]
    fn invoke_limits_default_fuel_cap_is_20m() {
        assert_eq!(InvokeLimits::default().fuel_cap, DEFAULT_FUEL_CAP);
    }

    // Proves the dispatcher enforces the fuel ceiling at the boundary: a synthetic WAT
    // workload of ~10.8M fuel crosses from trapped to completing when the ceiling
    // moves from 10M to 20M. This is a ceiling-enforcement test, not a production
    // model: the WAT loop runs with an empty payload and no host calls, so it does
    // not reflect the regression formula measured by the perf-engineer
    // (fuel = 29,452 + 27.514 × payload_bytes). The ~10.8M estimate is based on
    // the loop structure (1,200,000 iterations × ~9 WASM instructions each); the
    // actual observed value is asserted in the 20M branch below.
    #[test]
    fn fuel_workload_exhausts_10m_completes_at_20m() {
        let engine = build_engine().unwrap();
        // Arithmetic loop: 1,200,000 iterations x ~9 instructions ≈ 10.8M fuel.
        let module = compile(
            &engine,
            r#"
            (module
              (memory (export "memory") 1)
              (func (export "_start")
                (local $i i32)
                (local.set $i (i32.const 0))
                (block $done
                  (loop $l
                    (local.set $i (i32.add (local.get $i) (i32.const 1)))
                    (br_if $done (i32.ge_u (local.get $i) (i32.const 1200000)))
                    (br $l)))))
            "#,
        );

        // At the former 10M cap: fuel exhaustion.
        let (res_10m, _) = invoke_plugin(
            &engine,
            &module,
            bare_ctx(),
            b"",
            InvokeLimits {
                timeout_ms: 30_000,
                fuel_cap: 10_000_000,
            },
        )
        .unwrap();
        assert!(
            matches!(
                res_10m,
                PluginResult::Timeout {
                    cause: TimeoutCause::Fuel,
                    ..
                }
            ),
            "expected Timeout{{Fuel}} at 10M cap, got {res_10m:?}"
        );

        // At the new 20M cap: completes successfully.
        let (res_20m, _) = invoke_plugin(
            &engine,
            &module,
            bare_ctx(),
            b"",
            InvokeLimits {
                timeout_ms: 30_000,
                fuel_cap: DEFAULT_FUEL_CAP,
            },
        )
        .unwrap();
        // Assert both the exit code and the observed fuel so a future wasmtime
        // accounting change fails with a message stating the actual cost rather
        // than an opaque pattern-match mismatch.
        match res_20m {
            PluginResult::Ok {
                exit_code,
                fuel_consumed,
                ..
            } => {
                assert_eq!(exit_code, 0, "expected exit_code=0 at 20M cap");
                assert!(
                    fuel_consumed > 10_000_000,
                    "fuel_consumed should be > 10M (workload is ~10.8M), got {fuel_consumed}"
                );
                assert!(
                    fuel_consumed < DEFAULT_FUEL_CAP,
                    "fuel_consumed should be < cap ({DEFAULT_FUEL_CAP}), got {fuel_consumed}"
                );
            }
            other => panic!("expected Ok(exit_code=0) at 20M cap, got {other:?}"),
        }
    }

    #[test]
    fn invoke_normal_plugin_returns_ok() {
        // Minimal WASI command that just returns successfully.
        let engine = build_engine().unwrap();
        let module = compile(
            &engine,
            r#"
            (module
              (memory (export "memory") 1)
              (func (export "_start")))
            "#,
        );
        let (res, _) =
            invoke_plugin(&engine, &module, bare_ctx(), b"", InvokeLimits::default()).unwrap();
        match res {
            PluginResult::Ok { exit_code, .. } => assert_eq!(exit_code, 0),
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[test]
    fn invoke_with_infinite_loop_times_out_on_epoch() {
        // Start the ticker so the engine's epoch actually advances.
        let engine = build_engine().unwrap();
        let _ticker = crate::engine::EpochTicker::start(engine.clone());

        // Infinite loop at a yield point — br 0 checkpoints the epoch
        // on every iteration.
        let module = compile(
            &engine,
            r#"
            (module
              (memory (export "memory") 1)
              (func (export "_start")
                (loop (br 0))))
            "#,
        );
        let (res, _) = invoke_plugin(
            &engine,
            &module,
            bare_ctx(),
            b"",
            InvokeLimits {
                timeout_ms: 50,
                fuel_cap: u64::MAX,
            },
        )
        .unwrap();
        match res {
            PluginResult::Timeout {
                cause: TimeoutCause::Epoch,
                ..
            } => {}
            other => panic!("expected Timeout{{Epoch}}, got {other:?}"),
        }
    }

    #[test]
    fn invoke_fuel_hog_runs_out_of_fuel() {
        let engine = build_engine().unwrap();
        // Tight arithmetic loop — each iteration consumes a few fuel
        // units via wasmtime's instruction-count metering.
        let module = compile(
            &engine,
            r#"
            (module
              (memory (export "memory") 1)
              (func (export "_start")
                (local $i i32)
                (local.set $i (i32.const 0))
                (loop $l
                  (local.set $i (i32.add (local.get $i) (i32.const 1)))
                  (br $l))))
            "#,
        );
        let (res, _) = invoke_plugin(
            &engine,
            &module,
            bare_ctx(),
            b"",
            InvokeLimits {
                timeout_ms: 60_000,
                fuel_cap: 10_000,
            },
        )
        .unwrap();
        match res {
            PluginResult::Timeout {
                cause: TimeoutCause::Fuel,
                ..
            } => {}
            other => panic!("expected Timeout{{Fuel}}, got {other:?}"),
        }
    }

    #[test]
    fn invoke_panic_plugin_reports_crashed() {
        let engine = build_engine().unwrap();
        let module = compile(
            &engine,
            r#"
            (module
              (memory (export "memory") 1)
              (func (export "_start")
                unreachable))
            "#,
        );
        let (res, _) =
            invoke_plugin(&engine, &module, bare_ctx(), b"", InvokeLimits::default()).unwrap();
        match res {
            PluginResult::Crashed { .. } => {}
            other => panic!("expected Crashed, got {other:?}"),
        }
    }

    #[test]
    fn invoke_records_elapsed_and_fuel_on_ok() {
        let engine = build_engine().unwrap();
        let module = compile(
            &engine,
            r#"
            (module
              (memory (export "memory") 1)
              (func (export "_start")))
            "#,
        );
        let (res, _) = invoke_plugin(
            &engine,
            &module,
            bare_ctx(),
            b"",
            InvokeLimits {
                timeout_ms: 1000,
                fuel_cap: 1_000_000,
            },
        )
        .unwrap();
        if let PluginResult::Ok { fuel_consumed, .. } = res {
            assert!(
                fuel_consumed < 1_000_000,
                "fuel_consumed should be well under cap for trivial plugin"
            );
        } else {
            panic!("expected Ok");
        }
    }

    // -----------------------------------------------------------------------
    // S-19.09 T-001 — AC-001 (D19 RED gate)
    //
    // A WASM module importing vsdd::read_prefix must instantiate without a
    // link error via setup_host_on_store_data (the production dispatch path).
    //
    // RED today: read_prefix is absent from setup_host_on_store_data (0-hit
    // grep at develop 9787c056); instantiation fails with wasmtime link error
    // "unknown import: `vsdd::read_prefix`".
    //
    // GREEN after D19: setup_host_on_store_data registers read_prefix,
    // satisfying the import and allowing instantiation to succeed.
    //
    // AC trace: AC-001; ADR-025 §Decision 16; BC-1.17.001.
    // -----------------------------------------------------------------------
    #[test]
    fn t001_s19_09_read_prefix_instantiates_without_link_error_via_production_linker() {
        let engine = build_engine().unwrap();

        // Module imports only vsdd::read_prefix — no WASI imports needed.
        // WAT requires all imports before memory declarations.
        let module = compile(
            &engine,
            r#"(module
              (import "vsdd" "read_prefix" (func (param i32 i32 i32 i32 i32 i32) (result i32)))
              (memory (export "memory") 2)
              (func (export "_start"))
            )"#,
        );

        let mut linker: wasmtime::Linker<StoreData> = wasmtime::Linker::new(&engine);
        // RED gate: setup_host_on_store_data does not register read_prefix today;
        // linker.instantiate below returns Err("unknown import: vsdd::read_prefix").
        setup_host_on_store_data(&mut linker)
            .expect("setup_host_on_store_data must not error on registration");

        let wasi_ctx = WasiCtxBuilder::new().build_p1();
        let store_data = StoreData {
            host: bare_ctx(),
            wasi: wasi_ctx,
            host_output_too_large_seen: false,
        };
        let mut store = Store::new(&engine, store_data);
        store
            .set_fuel(1_000_000)
            .expect("engine has fuel metering enabled");
        store.set_epoch_deadline(u64::MAX);

        // The load-bearing assertion: instantiation MUST succeed.
        // Fails today with InvokeError::Instantiate (unresolved import).
        linker.instantiate(&mut store, &module).expect(
            "T-001 AC-001: read_prefix import must be satisfied by setup_host_on_store_data \
             (production dispatch path, ADR-025 §Decision 16); \
             got link error — read_prefix not registered today (0-hit grep at 9787c056)",
        );
    }

    // -----------------------------------------------------------------------
    // S-19.09 T-002 — AC-002 (D19 RED gate)
    //
    // Round-trip: production path reads an allowlisted file; returned bytes
    // match expected content; out_ptr written to WASM memory is > 0 (the
    // memory-grow protocol at current_bytes, ADR-025 §Decision 16).
    //
    // RED today: setup_host_on_store_data does not register read_prefix;
    // instantiation fails before the round-trip can be exercised.
    //
    // GREEN after D19: read_prefix production binding registered; memory-grow
    // protocol writes bytes at current_bytes (> 0 for non-empty files).
    //
    // AC trace: AC-002; ADR-025 §Decision 16 (memory-grow at current_bytes);
    // BC-1.17.001 PC-1 + PC-2.
    // -----------------------------------------------------------------------
    #[test]
    fn t002_s19_09_read_prefix_round_trip_bytes_correct_and_out_ptr_nonzero_via_production_path() {
        use crate::registry::{Capabilities, ReadPrefixCaps};

        // Write a tmp file with known content; tempdir gives per-test isolation
        // (F-P2-002: replace fixed std::env::temp_dir path with tempfile::tempdir).
        let content = b"hello-read-prefix-content";
        let dir = tempfile::tempdir().expect("create temp dir for T-002");
        let tmp_path = dir.path().join("s19_09_t002_read_prefix.txt");
        std::fs::write(&tmp_path, content).expect("write tmp file for T-002");
        let path_str = tmp_path.to_str().expect("path to str").to_string();
        let path_bytes = path_str.as_bytes();

        // HostContext with read_prefix capability allowing the tmp path.
        let mut ctx = bare_ctx();
        ctx.capabilities = Capabilities {
            read_prefix: Some(ReadPrefixCaps {
                path_allow: vec![path_str.clone()],
            }),
            ..Capabilities::default()
        };

        let engine = build_engine().unwrap();
        let mut linker: wasmtime::Linker<StoreData> = wasmtime::Linker::new(&engine);
        // RED gate: fails today — read_prefix not in setup_host_on_store_data.
        setup_host_on_store_data(&mut linker).expect("setup_host_on_store_data must not error");

        // WAT module layout:
        //   memory[0:4]   — out_ptr_out (written by host)
        //   memory[4:8]   — out_len_out (written by host)
        //   memory[128..] — path bytes (written by test before call)
        //
        // call_rp(path_ptr, path_len) → host return code (0 = codes::OK)
        // get_out_ptr()               → i32 loaded from memory[0:4]
        // WAT requires all imports before memory declarations.
        let module = compile(
            &engine,
            r#"(module
              (import "vsdd" "read_prefix" (func $rp (param i32 i32 i32 i32 i32 i32) (result i32)))
              (memory (export "memory") 2)
              (func (export "call_rp") (param $path_ptr i32) (param $path_len i32) (result i32)
                (call $rp
                  (local.get $path_ptr)
                  (local.get $path_len)
                  (i32.const 64)
                  (i32.const 0)
                  (i32.const 0)
                  (i32.const 4)
                )
              )
              (func (export "get_out_ptr") (result i32)
                (i32.load (i32.const 0))
              )
            )"#,
        );

        let wasi_ctx = WasiCtxBuilder::new().build_p1();
        let store_data = StoreData {
            host: ctx,
            wasi: wasi_ctx,
            host_output_too_large_seen: false,
        };
        let mut store = Store::new(&engine, store_data);
        store
            .set_fuel(1_000_000)
            .expect("engine has fuel metering enabled");
        store.set_epoch_deadline(u64::MAX);

        let instance = linker.instantiate(&mut store, &module).expect(
            "T-002: instantiation must succeed via production path (setup_host_on_store_data)",
        );

        // Write path bytes to WASM memory at offset 128.
        let memory = instance
            .get_memory(&mut store, "memory")
            .expect("module exports memory");
        memory
            .write(&mut store, 128, path_bytes)
            .expect("write path bytes to WASM memory");

        // Call read_prefix via the WAT wrapper; assert return code == 0 (OK).
        let call_rp = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "call_rp")
            .expect("module exports call_rp");
        let ret = call_rp
            .call(&mut store, (128, path_bytes.len() as i32))
            .expect("call_rp must not trap");
        assert_eq!(
            ret, 0,
            "T-002 AC-002: read_prefix must return codes::OK (0) for allowed file; got {}",
            ret
        );

        // Assert out_ptr > 0 (production memory-grow protocol writes at current_bytes > 0).
        let get_out_ptr = instance
            .get_typed_func::<(), i32>(&mut store, "get_out_ptr")
            .expect("module exports get_out_ptr");
        let out_ptr = get_out_ptr
            .call(&mut store, ())
            .expect("get_out_ptr must not trap");
        assert!(
            out_ptr > 0,
            "T-002 AC-002: out_ptr must be > 0 for non-empty file via production path \
             (ADR-025 §Decision 16: memory-grow protocol writes at current_bytes > 0); \
             got out_ptr={}",
            out_ptr
        );

        // Read out_len from memory[4:8] (little-endian u32; host writes at out_len_out=4).
        // F-P2-001: extend T-002 to verify both out_len and byte content, not just out_ptr.
        let mem_data: Vec<u8> = memory.data(&store).to_vec();
        let out_len = u32::from_le_bytes(
            mem_data[4..8]
                .try_into()
                .expect("memory[4:8] must be 4 bytes"),
        );
        assert_eq!(
            out_len,
            content.len() as u32,
            "T-002 AC-002: out_len must equal fixture content length ({} bytes); got {}; \
             production marshalling correctness (ADR-025 §Decision 16)",
            content.len(),
            out_len
        );

        // Read back memory[out_ptr..out_ptr+out_len] and assert byte equality.
        // Mutation-check (TD-VSDD-059): this assertion is load-bearing — changing
        // `content` to any other byte string causes an immediate assertion failure,
        // proving the host correctly marshalled the fixture bytes into WASM linear memory.
        let start = out_ptr as u32 as usize;
        let end = start + out_len as usize;
        assert_eq!(
            &mem_data[start..end],
            content.as_slice(),
            "T-002 AC-002: bytes read back from WASM memory[{}..{}] must equal fixture content; \
             production marshalling correctness (ADR-025 §Decision 16)",
            start,
            end
        );
        // dir goes out of scope here; tempfile::TempDir::drop auto-cleans the directory.
    }

    // -----------------------------------------------------------------------
    // S-19.09 T-002b — AC-002 head-c bound (D19 GREEN gate)
    //
    // read_prefix with a file LARGER than max_bytes must:
    //   1. Return codes::OK (0).
    //   2. Write out_len == max_bytes exactly (head-c truncation at the bound).
    //   3. Write the first max_bytes bytes of the file content to WASM memory.
    //
    // ADR-025 §Decision 16 / BC-1.17.001 PC-2: head-c semantics — the function
    // behaves like `head -c max_bytes`; content beyond the bound is discarded.
    //
    // AC trace: AC-002; ADR-025 §Decision 16; BC-1.17.001 PC-2.
    // -----------------------------------------------------------------------
    #[test]
    fn t002b_s19_09_read_prefix_head_c_bound_clamps_out_len_to_max_bytes() {
        use crate::registry::{Capabilities, ReadPrefixCaps};

        const MAX_BYTES: usize = 10;
        // 40-byte content — deliberately larger than max_bytes = 10.
        let content: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMN";
        let dir = tempfile::tempdir().expect("create temp dir for T-002b");
        let tmp_path = dir.path().join("s19_09_t002b_read_prefix.txt");
        std::fs::write(&tmp_path, content).expect("write tmp file for T-002b");
        let path_str = tmp_path.to_str().expect("path to str").to_string();
        let path_bytes = path_str.as_bytes();

        let mut ctx = bare_ctx();
        ctx.capabilities = Capabilities {
            read_prefix: Some(ReadPrefixCaps {
                path_allow: vec![path_str.clone()],
            }),
            ..Capabilities::default()
        };

        let engine = build_engine().unwrap();
        let mut linker: wasmtime::Linker<StoreData> = wasmtime::Linker::new(&engine);
        setup_host_on_store_data(&mut linker).expect("setup_host_on_store_data must not error");

        // WAT layout same as T-002 but max_bytes = 10 (hardcoded in i32.const).
        let module = compile(
            &engine,
            r#"(module
              (import "vsdd" "read_prefix" (func $rp (param i32 i32 i32 i32 i32 i32) (result i32)))
              (memory (export "memory") 2)
              (func (export "call_rp") (param $path_ptr i32) (param $path_len i32) (result i32)
                (call $rp
                  (local.get $path_ptr)
                  (local.get $path_len)
                  (i32.const 10)
                  (i32.const 0)
                  (i32.const 0)
                  (i32.const 4)
                )
              )
            )"#,
        );

        let wasi_ctx = WasiCtxBuilder::new().build_p1();
        let store_data = StoreData {
            host: ctx,
            wasi: wasi_ctx,
            host_output_too_large_seen: false,
        };
        let mut store = Store::new(&engine, store_data);
        store
            .set_fuel(1_000_000)
            .expect("engine has fuel metering enabled");
        store.set_epoch_deadline(u64::MAX);

        let instance = linker
            .instantiate(&mut store, &module)
            .expect("T-002b: instantiation must succeed");

        let memory = instance
            .get_memory(&mut store, "memory")
            .expect("module exports memory");
        memory
            .write(&mut store, 128, path_bytes)
            .expect("write path bytes to WASM memory");

        let call_rp = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "call_rp")
            .expect("module exports call_rp");
        let ret = call_rp
            .call(&mut store, (128, path_bytes.len() as i32))
            .expect("call_rp must not trap");
        assert_eq!(
            ret, 0,
            "T-002b AC-002: read_prefix must return codes::OK (0) for file > max_bytes; got {}",
            ret
        );

        // Read out_ptr (memory[0:4]) and out_len (memory[4:8]) directly.
        let mem_data: Vec<u8> = memory.data(&store).to_vec();
        let out_ptr = u32::from_le_bytes(
            mem_data[0..4]
                .try_into()
                .expect("memory[0:4] must be 4 bytes"),
        );
        let out_len = u32::from_le_bytes(
            mem_data[4..8]
                .try_into()
                .expect("memory[4:8] must be 4 bytes"),
        );

        assert!(
            out_ptr > 0,
            "T-002b AC-002: out_ptr must be > 0 for non-empty truncated result; got {}",
            out_ptr
        );
        assert_eq!(
            out_len,
            MAX_BYTES as u32,
            "T-002b AC-002: out_len must be clamped to max_bytes ({}) not full content length ({}); \
             head-c semantics per ADR-025 §Decision 16 / BC-1.17.001 PC-2",
            MAX_BYTES,
            content.len()
        );

        // Byte equality: returned bytes must match the FIRST max_bytes of content.
        let start = out_ptr as usize;
        let end = start + out_len as usize;
        assert_eq!(
            &mem_data[start..end],
            &content[..MAX_BYTES],
            "T-002b AC-002: read-back bytes must equal the first {} bytes of fixture content; \
             head-c truncation correctness (ADR-025 §Decision 16 / BC-1.17.001 PC-2)",
            MAX_BYTES
        );
        // dir goes out of scope here; tempfile::TempDir::drop auto-cleans the directory.
    }

    // -----------------------------------------------------------------------
    // S-19.09 T-003 — AC-003 (D19 RED gate)
    //
    // read_prefix call with no read_prefix capability block must return
    // CAPABILITY_DENIED (-1) via setup_host_on_store_data (production path),
    // matching the deny-by-default behavior of read_file on the same path.
    //
    // RED today: instantiation fails with link error (read_prefix not in
    // setup_host_on_store_data); the CAPABILITY_DENIED assertion is never reached.
    //
    // GREEN after D19: instantiation succeeds; the function returns -1 because
    // the HostContext has no capabilities.read_prefix block.
    //
    // AC trace: AC-003; ADR-025 §Decision 16; BC-1.17.001 PC-4.
    // -----------------------------------------------------------------------
    #[test]
    fn t003_s19_09_read_prefix_capability_absent_returns_capability_denied_via_production_path() {
        // bare_ctx() → Capabilities::default() → read_prefix: None (deny-by-default).
        let ctx = bare_ctx();

        let engine = build_engine().unwrap();
        let mut linker: wasmtime::Linker<StoreData> = wasmtime::Linker::new(&engine);
        // RED gate: fails today — read_prefix not in setup_host_on_store_data.
        setup_host_on_store_data(&mut linker).expect("setup_host_on_store_data must not error");

        // WAT requires all imports before memory declarations.
        let module = compile(
            &engine,
            r#"(module
              (import "vsdd" "read_prefix" (func $rp (param i32 i32 i32 i32 i32 i32) (result i32)))
              (memory (export "memory") 2)
              (func (export "call_rp") (param $path_ptr i32) (param $path_len i32) (result i32)
                (call $rp
                  (local.get $path_ptr)
                  (local.get $path_len)
                  (i32.const 64)
                  (i32.const 0)
                  (i32.const 0)
                  (i32.const 4)
                )
              )
            )"#,
        );

        let wasi_ctx = WasiCtxBuilder::new().build_p1();
        let store_data = StoreData {
            host: ctx,
            wasi: wasi_ctx,
            host_output_too_large_seen: false,
        };
        let mut store = Store::new(&engine, store_data);
        store
            .set_fuel(1_000_000)
            .expect("engine has fuel metering enabled");
        store.set_epoch_deadline(u64::MAX);

        let instance = linker.instantiate(&mut store, &module).expect(
            "T-003: instantiation must succeed via production path (setup_host_on_store_data)",
        );

        let path = b"/some/path.txt";
        let memory = instance
            .get_memory(&mut store, "memory")
            .expect("module exports memory");
        memory
            .write(&mut store, 128, path)
            .expect("write path to WASM memory");

        let call_rp = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "call_rp")
            .expect("module exports call_rp");
        let ret = call_rp
            .call(&mut store, (128, path.len() as i32))
            .expect("call_rp must not trap");

        assert_eq!(
            ret, -1,
            "T-003 AC-003: read_prefix with no capabilities.read_prefix block must return \
             CAPABILITY_DENIED (-1) via production path (setup_host_on_store_data); \
             deny-by-default per BC-1.17.001 PC-4; got {}",
            ret
        );
    }

    // -----------------------------------------------------------------------
    // S-19.09 T-015 — AC-002 (F-P6-001 closure: EC-002 coverage gap)
    //
    // read_prefix on a 0-byte (empty) allowlisted file via setup_host_on_store_data
    // (the D19 production path) must:
    //   1. Return codes::OK (0).
    //   2. Write out_ptr == 0 to memory[0:4]  (empty-body fast path skips grow).
    //   3. Write out_len == 0 to memory[4:8]  (no bytes to transfer).
    //   4. Leave WASM linear memory size unchanged (no grow call on empty body).
    //
    // The `if body.is_empty()` branch in setup_host_on_store_data's read_prefix
    // closure (EC-002) writes ptr=0, len=0 and returns codes::OK immediately —
    // it does NOT call memory.grow.  t001/t002/t002b/t003 cover only
    // link/non-empty/clamp/denial; none exercise this branch.
    //
    // Mutation-check (TD-VSDD-059): if the `if body.is_empty()` early-return
    // were removed, the code falls through to the memory-grow protocol.
    // grow(0 pages) succeeds but write_offset = current_bytes (nonzero for
    // any loaded WASM module), so out_ptr would be written as current_bytes
    // rather than 0 — the assert_eq!(out_ptr, 0) assertion flips to FAIL,
    // demonstrating this test is load-bearing for the EC-002 branch.
    //
    // AC trace: AC-002; BC-1.17.001 PC-1; ADR-025 §Decision 16; EC-002.
    // -----------------------------------------------------------------------
    #[test]
    fn t015_s19_09_read_prefix_empty_file_returns_ok_with_zero_ptr_len_no_grow() {
        use crate::registry::{Capabilities, ReadPrefixCaps};

        // Write a 0-byte file; tempdir gives per-test isolation (sibling convention).
        let dir = tempfile::tempdir().expect("create temp dir for T-015");
        let tmp_path = dir.path().join("s19_09_t015_empty.txt");
        std::fs::write(&tmp_path, b"").expect("write empty file for T-015");
        let path_str = tmp_path.to_str().expect("path to str").to_string();
        let path_bytes = path_str.as_bytes();

        // HostContext with read_prefix capability allowing the empty file.
        let mut ctx = bare_ctx();
        ctx.capabilities = Capabilities {
            read_prefix: Some(ReadPrefixCaps {
                path_allow: vec![path_str.clone()],
            }),
            ..Capabilities::default()
        };

        let engine = build_engine().unwrap();
        let mut linker: wasmtime::Linker<StoreData> = wasmtime::Linker::new(&engine);
        setup_host_on_store_data(&mut linker).expect("setup_host_on_store_data must not error");

        // WAT layout (mirrors T-002):
        //   memory[0:4]   — out_ptr_out location (host writes the returned pointer here)
        //   memory[4:8]   — out_len_out location (host writes the returned length here)
        //   memory[128..] — path bytes (written by test before call)
        //
        // call_rp(path_ptr, path_len) → host return code (0 = codes::OK)
        let module = compile(
            &engine,
            r#"(module
              (import "vsdd" "read_prefix" (func $rp (param i32 i32 i32 i32 i32 i32) (result i32)))
              (memory (export "memory") 2)
              (func (export "call_rp") (param $path_ptr i32) (param $path_len i32) (result i32)
                (call $rp
                  (local.get $path_ptr)
                  (local.get $path_len)
                  (i32.const 64)
                  (i32.const 0)
                  (i32.const 0)
                  (i32.const 4)
                )
              )
            )"#,
        );

        let wasi_ctx = WasiCtxBuilder::new().build_p1();
        let store_data = StoreData {
            host: ctx,
            wasi: wasi_ctx,
            host_output_too_large_seen: false,
        };
        let mut store = Store::new(&engine, store_data);
        store
            .set_fuel(1_000_000)
            .expect("engine has fuel metering enabled");
        store.set_epoch_deadline(u64::MAX);

        let instance = linker.instantiate(&mut store, &module).expect(
            "T-015: instantiation must succeed via production path (setup_host_on_store_data)",
        );

        let memory = instance
            .get_memory(&mut store, "memory")
            .expect("module exports memory");
        memory
            .write(&mut store, 128, path_bytes)
            .expect("write path bytes to WASM memory");

        // Capture memory size BEFORE the read_prefix call so we can assert
        // that the empty-body fast path skips the grow call.
        let memory_size_before = memory.data_size(&store);

        let call_rp = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "call_rp")
            .expect("module exports call_rp");
        let ret = call_rp
            .call(&mut store, (128, path_bytes.len() as i32))
            .expect("call_rp must not trap");

        // Capture memory size AFTER the read_prefix call.
        let memory_size_after = memory.data_size(&store);

        // Assert 1: return code must be codes::OK (0).
        assert_eq!(
            ret, 0,
            "T-015 AC-002: read_prefix on an empty file must return codes::OK (0); \
             EC-002 empty-body fast path; got {}",
            ret
        );

        // Read out_ptr (memory[0:4]) and out_len (memory[4:8]).
        let mem_data: Vec<u8> = memory.data(&store).to_vec();
        let out_ptr = u32::from_le_bytes(
            mem_data[0..4]
                .try_into()
                .expect("memory[0:4] must be 4 bytes"),
        );
        let out_len = u32::from_le_bytes(
            mem_data[4..8]
                .try_into()
                .expect("memory[4:8] must be 4 bytes"),
        );

        // Assert 2: out_ptr must be 0 — empty-body fast path writes ptr=0, skips grow.
        // Mutation-check: removing the `if body.is_empty()` early-return causes the code
        // to fall through to memory-grow.  grow(0 pages) succeeds but write_offset =
        // current_bytes (nonzero for any loaded WASM module), so out_ptr would equal
        // current_bytes rather than 0 — this assertion flips to FAIL, proving the test
        // detects regression of the EC-002 branch.
        assert_eq!(
            out_ptr, 0,
            "T-015 AC-002: out_ptr must be 0 for empty file — empty-body fast path writes \
             ptr=0, skips grow (ADR-025 §Decision 16; EC-002); got out_ptr={}",
            out_ptr
        );

        // Assert 3: out_len must be 0 (no bytes to transfer).
        assert_eq!(
            out_len, 0,
            "T-015 AC-002: out_len must be 0 for empty file (EC-002: 0 bytes to transfer); \
             got out_len={}",
            out_len
        );

        // Assert 4: WASM linear memory size must be unchanged — empty-body fast path
        // exits before the grow call, so no page allocation occurs.
        assert_eq!(
            memory_size_before, memory_size_after,
            "T-015 AC-002: WASM linear memory must NOT grow for empty file — empty-body fast \
             path skips the grow call (EC-002 no-memory-growth invariant); \
             before={} after={}",
            memory_size_before, memory_size_after
        );
        // dir goes out of scope here; tempfile::TempDir::drop auto-cleans the directory.
    }
}

// S-18.04b-prereq: git_context payload injection (ADR-029)

/// The seven-field git_context schema injected into `payload.extra` on qualifying
/// PostToolUse Bash git-commit events (ADR-029 §Decision 2 / ADR-032-AC021-prereq).
///
/// All fields are `String`. Empty string means the field could not be populated
/// (e.g. `head_parent_sha` when factory-artifacts has only one commit).
/// The dispatcher MUST NOT use `null` — empty string is the sentinel per AC-006.
///
/// This struct is `pub` so integration tests can construct expected values and
/// compare against the injected `serde_json::Value::Object`.
///
/// # ADR-032-AC021-prereq fields (three new fields)
///
/// `head_state_timestamp`, `head_parent_state_timestamp`, and `state_md_in_commit`
/// are required by the AC-021 advisory plugin to detect stale timestamp commits
/// without invoking `exec_subprocess` in the WASM sandbox (ADR-032 §Prerequisite
/// deliverable). All three use the same empty-string sentinel convention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitContext {
    /// Subject line of HEAD commit in the factory-artifacts worktree.
    pub head_subject: String,
    /// Full 40-character SHA of HEAD commit.
    pub head_sha: String,
    /// Subject line of HEAD^ commit. Empty string if HEAD^ does not exist
    /// (initial commit case — AC-006, AC-011).
    pub head_parent_subject: String,
    /// Full 40-character SHA of HEAD^ commit. Empty string if HEAD^ does not
    /// exist (initial commit case).
    pub head_parent_sha: String,
    /// The `timestamp:` frontmatter value extracted from `HEAD:STATE.md`
    /// in the factory-artifacts worktree (ADR-032-AC021-prereq).
    /// Empty string if git show fails or the field is absent.
    pub head_state_timestamp: String,
    /// The `timestamp:` frontmatter value extracted from `HEAD^:STATE.md`.
    /// Empty string on initial commit or git error.
    pub head_parent_state_timestamp: String,
    /// Serialized bool: `"true"` if `STATE.md` appears in the diff between
    /// HEAD^ and HEAD; `"false"` if it does not; `""` on git error (initial
    /// commit with no HEAD^, git not found, etc.) — ADR-032-AC021-prereq.
    pub state_md_in_commit: String,
}

impl GitContext {
    /// Return the all-empty `GitContext` used as the fail-open sentinel
    /// (BC-1.16.001 PC2 / AC-002 / AC-009).
    ///
    /// # GREEN-BY-DESIGN
    ///
    /// Pure field initialisation; zero branching, no I/O, no helpers.
    /// Body is trivial struct construction only — BC-5.38.002 criteria all satisfied.
    pub fn empty() -> Self {
        Self {
            head_subject: String::new(),
            head_sha: String::new(),
            head_parent_subject: String::new(),
            head_parent_sha: String::new(),
            head_state_timestamp: String::new(),
            head_parent_state_timestamp: String::new(),
            state_md_in_commit: String::new(),
        }
    }

    /// Serialize this context to a `serde_json::Value::Object` suitable for
    /// insertion into `payload.extra["git_context"]` (ADR-029 §Decision 2).
    ///
    /// All seven fields are explicitly named in the `json!` literal — the macro
    /// does NOT serialize struct fields automatically (ADR-032 §Prerequisite
    /// deliverable item 5).
    ///
    /// # GREEN-BY-DESIGN
    ///
    /// Builds a JSON object from the seven string fields; zero branching,
    /// no I/O, no non-trivial helpers. BC-5.38.002 satisfied.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "head_subject": self.head_subject,
            "head_sha": self.head_sha,
            "head_parent_subject": self.head_parent_subject,
            "head_parent_sha": self.head_parent_sha,
            "head_state_timestamp": self.head_state_timestamp,
            "head_parent_state_timestamp": self.head_parent_state_timestamp,
            "state_md_in_commit": self.state_md_in_commit,
        })
    }
}

/// Detect whether a hook payload qualifies for `git_context` injection
/// (ADR-029 §Decision 1 + §Decision 3; BC-1.16.001 PC3/PC4; AC-003/AC-004/AC-010).
///
/// Returns `true` iff ALL of:
/// 1. `payload.event_name == "PostToolUse"` (AC-004: non-PostToolUse events never qualify).
/// 2. `payload.tool_name == "Bash"` (AC-004: Edit/Write/Agent never qualify).
/// 3. `payload.tool_input.command` contains `"git commit"` as a substring (AC-010).
/// 4. `payload.tool_input.command` contains `".factory"` as an indicator of the
///    factory-artifacts worktree (AC-010 heuristic; minimises spurious injection).
///
/// # Implementer notes
///
/// - Check `tool_name` BEFORE inspecting `tool_input.command` (AC-004: non-Bash events
///   MUST NOT have their command inspected at all).
/// - False positives (e.g. `echo "git commit"` with `.factory` path in args) are
///   acceptable per ADR-029 §Decision 3 Negative consequence note: `git_context` will
///   be valid git state (whatever HEAD of factory-artifacts is), and WASM plugins treat
///   valid-but-irrelevant context as "pass" (fail-open).
/// - Detection is heuristic (AC-010); exactness is not required.
pub fn detect_git_commit_event(payload: &crate::payload::HookPayload) -> bool {
    // AC-004: non-PostToolUse events never qualify.
    if payload.event_name != "PostToolUse" {
        return false;
    }
    // AC-004: non-Bash tools never qualify; do NOT inspect command for non-Bash.
    if payload.tool_name != "Bash" {
        return false;
    }
    // AC-010: heuristic detection — command invokes git with the "commit" subcommand
    // AND contains a ".factory" factory-artifacts worktree indicator.
    //
    // Detection: the command must contain "git" AND " commit" (space-prefixed to anchor
    // "commit" as a git subcommand token rather than part of a -m "message" argument
    // that merely mentions "commit"). The ".factory" indicator scopes to factory-artifacts.
    //
    // Examples that QUALIFY:
    //   "git -C .factory commit -m ..."     → "git" + " commit" + ".factory" ✓
    //   "git commit -C .factory -m ..."     → "git" + " commit" + ".factory" ✓
    //
    // Examples that do NOT qualify (EC-007/EC-008):
    //   "git commit -m ..."                 → no ".factory" indicator ✗
    //   "echo \"git commit\""               → no ".factory" indicator ✗
    //   "git -C .factory push ..."          → no " commit" token ✗
    let command = match payload.tool_input.get("command").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return false,
    };
    // CR-001: A command such as `git -C .factory commit -m "force commit"` contains
    // " commit" twice; this is acceptable — it is still a qualifying git-commit event.
    // False positives are spec-sanctioned per ADR-029 §Decision 3.
    command.contains("git") && command.contains(" commit") && command.contains(".factory")
}

/// Execute the seven git commands/operations against the factory-artifacts worktree at
/// `factory_dir` and return a populated `GitContext` (ADR-029 §Decision 3 / ADR-032-AC021-prereq).
///
/// Commands executed (in order) via `Command::new("git").current_dir(factory_dir)`:
/// 1. `git log --format=%s -1 HEAD` → `head_subject`
/// 2. `git rev-parse HEAD` → `head_sha`
/// 3. `git log --format=%s -1 HEAD^` → `head_parent_subject`
///    (empty string if HEAD^ does not exist — exit non-zero)
/// 4. `git rev-parse HEAD^` → `head_parent_sha`
///    (empty string if HEAD^ does not exist — exit non-zero)
/// 5. `git show HEAD:STATE.md` → `head_state_timestamp`
///    (extract `timestamp:` field via `extract_yaml_string_value`; empty on error)
/// 6. `git show HEAD^:STATE.md` → `head_parent_state_timestamp`
///    (empty string if HEAD^ does not exist — exit non-zero)
/// 7. `git diff --name-only HEAD^ HEAD` → `state_md_in_commit`
///    (`"true"` if STATE.md is listed; `"false"` otherwise; empty string on error)
///
/// # Fail-open contract (BC-1.16.001 PC2 / AC-002 / AC-009)
///
/// On ANY git command failure (non-zero exit, git binary not found, I/O error,
/// permission denied), the function MUST:
/// 1. Emit `tracing::warn!` describing the failure.
/// 2. Return `GitContext::empty()` (all seven fields `""`).
///
/// The dispatcher MUST NOT block, abort, or fail-closed on a git error.
///
/// # Initial commit handling (AC-006, AC-011, EC-009)
///
/// Commands 3 and 4 (HEAD^) exit non-zero when factory-artifacts has only one
/// commit. This is NOT a general git error — commands 1 and 2 (HEAD) must still
/// be populated normally. Only `head_parent_subject` and `head_parent_sha` are
/// set to `""` (not null, not absent).
pub fn build_git_context(factory_dir: &std::path::Path) -> GitContext {
    // SEC-001 (CWE-117): Strip ASCII control characters from any git-derived string
    // before it is used in a tracing field. JSON payload values (via serde) are already
    // safe; only the tracing log emission path requires sanitization.
    let sanitize_for_log = |s: &str| -> String { s.chars().filter(|c| !c.is_control()).collect() };

    // Helper: run a git command and return trimmed stdout, or Err on failure.
    let run_git = |args: &[&str]| -> Result<String, String> {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(factory_dir)
            .output()
            .map_err(|e| format!("git exec failed: {e}"))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(format!(
                "git {} exited {}: {}",
                args.join(" "),
                output.status,
                // Sanitize stderr for log safety (SEC-001/CWE-117): only tracing path.
                sanitize_for_log(String::from_utf8_lossy(&output.stderr).trim())
            ))
        }
    };

    // Step 1: HEAD subject (log --format=%s -1 HEAD).
    let head_subject = match run_git(&["log", "--format=%s", "-1", "HEAD"]) {
        Ok(s) => s,
        Err(e) => {
            // e is already sanitized (control chars stripped in run_git error path).
            tracing::warn!(
                factory_dir = %factory_dir.display(),
                error = %e,
                "build_git_context: git log HEAD failed; fail-open with empty git_context"
            );
            return GitContext::empty();
        }
    };

    // Step 2: HEAD SHA (rev-parse HEAD).
    let head_sha = match run_git(&["rev-parse", "HEAD"]) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(
                factory_dir = %factory_dir.display(),
                error = %e,
                "build_git_context: git rev-parse HEAD failed; fail-open with empty git_context"
            );
            return GitContext::empty();
        }
    };

    // Steps 3+4: HEAD^ subject and SHA. Non-zero exit on initial commit is expected;
    // it is NOT a general git error — only the parent fields are empty (AC-006, AC-011, EC-009).
    let head_parent_subject = run_git(&["log", "--format=%s", "-1", "HEAD^"]).unwrap_or_default();
    let head_parent_sha = run_git(&["rev-parse", "HEAD^"]).unwrap_or_default();

    // ADR-032-AC021-prereq Steps 5+6+7: timestamp fields + STATE.md discriminator.

    // Step 5: HEAD state timestamp — extract timestamp: from HEAD:STATE.md.
    // factory_lock_parse::extract_yaml_string_value scans a single YAML line.
    // Fail-open (consistent with HEAD^ handling above): any error → String::new().
    let head_state_timestamp = match run_git(&["show", "HEAD:STATE.md"]) {
        Ok(content) => content
            .lines()
            .find_map(|line| factory_lock_parse::extract_yaml_string_value(line, "timestamp"))
            .unwrap_or_default(),
        Err(_) => String::new(),
    };

    // Step 6: HEAD^ state timestamp. Error on initial commit → String::new().
    let head_parent_state_timestamp = match run_git(&["show", "HEAD^:STATE.md"]) {
        Ok(content) => content
            .lines()
            .find_map(|line| factory_lock_parse::extract_yaml_string_value(line, "timestamp"))
            .unwrap_or_default(),
        Err(_) => String::new(),
    };

    // Step 7: Whether STATE.md was changed in HEAD relative to HEAD^.
    // git diff --name-only HEAD^ HEAD lists all filenames changed in the HEAD commit.
    // Error (e.g. initial commit with no HEAD^, or git not found) → String::new()
    // (fail-open; AC-021 Pre-condition 2 treats "" as fail-open Continue).
    let state_md_in_commit = match run_git(&["diff", "--name-only", "HEAD^", "HEAD"]) {
        Ok(files) => {
            if files.lines().any(|f| f.trim() == "STATE.md") {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Err(_) => String::new(),
    };

    GitContext {
        head_subject,
        head_sha,
        head_parent_subject,
        head_parent_sha,
        head_state_timestamp,
        head_parent_state_timestamp,
        state_md_in_commit,
    }
}

/// Orchestrate git_context detection, construction, and injection into
/// `payload_value` before it is routed to registered plugins.
///
/// # Contract (ADR-029 §Decision 1–3; BC-1.16.001 PC1–PC6)
///
/// 1. Call `detect_git_commit_event(&original_payload)` to determine if this
///    PostToolUse Bash event is a qualifying git-commit event.
/// 2. If non-qualifying: return immediately without mutating `payload_value`
///    (AC-003, AC-004 — no injection on non-qualifying events).
/// 3. If qualifying: call `build_git_context(factory_dir)` to obtain the seven-field context,
///    inject `git_context` as a `serde_json::Value::Object` into `payload_value` at key
///    `"git_context"` (rides in the `extra` flatten map — ADR-029 §Decision 2 / AC-005),
///    with all seven fields present as strings; null fields are forbidden (AC-006, AC-011).
///
/// # Arguments
///
/// - `original_payload`: the parsed `HookPayload` (used for detection only).
/// - `payload_value`: the mutable `serde_json::Value` that will be passed to
///   `ExecutorInputs` — injection mutates this value in place.
/// - `factory_dir`: path to the factory-artifacts worktree (typically
///   `<CLAUDE_PROJECT_DIR>/.factory`; derived from `CLAUDE_PROJECT_DIR` env var
///   at the call site in `main.rs`).
///
/// # Wiring site
///
/// This function is called in `main.rs` immediately after `dispatcher_trace_id`
/// is injected into `payload_value` and before `ExecutorInputs` is constructed.
/// See the `// S-18.04b-prereq: git_context injection site` comment in main.rs.
pub fn inject_git_context_if_qualifying(
    original_payload: &crate::payload::HookPayload,
    payload_value: &mut serde_json::Value,
    factory_dir: &std::path::Path,
) {
    // Step 1: detection — if non-qualifying, evict any caller-supplied "git_context"
    // key and return. git_context is dispatcher-authoritative (BC-1.16.001 INV1 /
    // SEC-002 / CWE-345): a caller-supplied key must not pass through to plugins.
    if !detect_git_commit_event(original_payload) {
        if let Some(map) = payload_value.as_object_mut() {
            map.remove("git_context");
        }
        return;
    }

    // Step 2: build git_context (fail-open: git errors produce GitContext::empty()).
    let git_ctx = build_git_context(factory_dir);

    // Step 3: inject into payload_value["git_context"] (rides in the extra flatten map).
    // No new named HookPayload field — git_context is a top-level key in the JSON value
    // (deserialized into HookPayload.extra via #[serde(flatten)]). AC-005.
    if let Some(map) = payload_value.as_object_mut() {
        map.insert("git_context".to_string(), git_ctx.to_json());
    }
}

// ---------------------------------------------------------------------------
// S-25.02 BC-1.18.006 Postcondition 7 catch point (i) qualifying wrapper
// (story AC-024; ADR-051 §Decision 15)
// ---------------------------------------------------------------------------
//
// `true` iff `tool_input` is an `Edit` call with `replace_all: true`, or a
// `MultiEdit` call whose `edits` array contains at least one block with
// `replace_all: true` (BC-1.18.006 Postcondition 7's scope predicate,
// mirrored from Precondition 4). Real, cheap, structural — see
// `detect_replace_all_overcap_candidate`'s own doc comment for why this
// stays outside this burst's `todo!()` boundary.
fn tool_input_has_replace_all_true(tool_input: &serde_json::Value) -> bool {
    if tool_input.get("replace_all").and_then(|v| v.as_bool()) == Some(true) {
        return true;
    }
    tool_input
        .get("edits")
        .and_then(|v| v.as_array())
        .is_some_and(|edits| {
            edits
                .iter()
                .any(|e| e.get("replace_all").and_then(|v| v.as_bool()) == Some(true))
        })
}

/// Structural (real, non-`todo!()`) pre-filter for BC-1.18.006 Postcondition
/// 7 catch point (i). Mirrors [`detect_git_commit_event`]'s own cheap-
/// detect-then-act shape (used by [`inject_git_context_if_qualifying`]
/// above) and `executor.rs::shard_cap_precheck`'s zero-cost-bypass
/// precedent — both already-shipped, non-stub native checks reached
/// unconditionally from `main::run`.
///
/// Every check here is cheap and structural (event/tool/`replace_all`/
/// config-match) — this is BC-1.18.006 Postcondition 7's OWN "zero added
/// cost outside the narrow case" requirement, not implementer business
/// logic invented for this stub burst. It is DELIBERATELY real (not
/// `todo!()`): if this filter itself were `todo!()`, EVERY PostToolUse
/// dispatch of ANY kind would panic against ADR-051 §Decision 15 point 4's
/// mandatory unconditional call site in `main::run` (see that call site's
/// own doc comment) — a catastrophic regression of the ENTIRE cluster-1
/// BC-1.18.005 suite plus every other integration/bats test that drives the
/// dispatcher at all. The BC's own tested `stat()`-and-retroactive-roll
/// behavior lives entirely inside
/// `shard_manager::reconcile_post_write_replace_all_overcap` (fully
/// `todo!()`) — this function never touches that behavior, only decides
/// whether to call into it.
fn detect_replace_all_overcap_candidate(
    original_payload: &crate::payload::HookPayload,
    cwd: &std::path::Path,
) -> Option<(crate::shard_manager::ShardEntry, std::path::PathBuf)> {
    if original_payload.event_name != "PostToolUse" {
        return None;
    }
    if !matches!(original_payload.tool_name.as_str(), "Edit" | "MultiEdit") {
        return None;
    }
    if !tool_input_has_replace_all_true(&original_payload.tool_input) {
        return None;
    }
    let shard_config_path = cwd.join(crate::executor::SHARD_CONFIG_RELATIVE_PATH);
    if !shard_config_path.exists() {
        return None;
    }
    let registry = crate::shard_manager::ShardRegistry::load(&shard_config_path).ok()?;
    let target_path = original_payload
        .tool_input
        .get("file_path")
        .and_then(|v| v.as_str())
        .map(std::path::PathBuf::from)?;
    let entry = crate::shard_manager::find_matching_entry(&registry, &target_path)
        .ok()
        .flatten()?
        .clone();
    Some((entry, target_path))
}

/// BC-1.18.006 Postcondition 7 catch point (i) qualifying wrapper (story
/// AC-024; ADR-051 §Decision 15). Called unconditionally from `main::run`
/// BEFORE its `sync_tiers.is_empty() && partition.async_group.is_empty()`
/// early-return guard (Decision 15 point 4's load-bearing placement
/// caveat) — mirrors [`inject_git_context_if_qualifying`]'s own
/// detect-then-act call shape. Silent filesystem side effect: emits NO
/// `HookResult` of its own (Decision 15 point 2 — this leg is a janitor,
/// not a gate) and never influences the caller's own dispatch outcome.
///
/// The qualification filter ([`detect_replace_all_overcap_candidate`]) is
/// real; the actual reconciliation behavior it delegates into
/// ([`crate::shard_manager::reconcile_post_write_replace_all_overcap`]) is
/// entirely `todo!()` — see that function's own doc comment. A failure
/// there is logged (fail-open, matching this leg's own "no `HookResult`
/// signaling" contract — this leg never blocks or errors the calling
/// dispatch), not propagated.
pub fn reconcile_replace_all_overcap_if_qualifying(
    original_payload: &crate::payload::HookPayload,
    cwd: &std::path::Path,
) {
    let Some((entry, target_path)) = detect_replace_all_overcap_candidate(original_payload, cwd)
    else {
        return;
    };
    if let Err(e) =
        crate::shard_manager::reconcile_post_write_replace_all_overcap(&entry, &target_path)
    {
        tracing::warn!(
            artifact_stem = %entry.artifact_stem,
            error = %e,
            "BC-1.18.006 Postcondition 7 catch point (i): retroactive reconciliation failed"
        );
    }
}

// ---------------------------------------------------------------------------
// S-25.02 cluster-2 LOCAL adversary pass-1 finding F-C2-P1-006 — coverage
// unit tests for `tool_input_has_replace_all_true`'s `MultiEdit` `edits[]`-
// array branch (BC-1.18.006 Precondition 4 / Postcondition 7 scope
// predicate: "a `MultiEdit` call containing at least one edit block whose
// `replace_all` field is `true`"). `tool_input_has_replace_all_true` is a
// real (non-`todo!()`), already-shipped structural pre-filter — this is a
// COVERAGE test asserting the branch behaves correctly, not a Red Gate test
// for unimplemented behavior. If either assertion below fails, that is a
// latent bug in the already-shipped `MultiEdit` `edits[]` branch, not an
// expected-red TDD state.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod f_c2_p1_006_replace_all_multi_edit_tests {
    use super::*;

    /// A `MultiEdit` call whose `edits[]` array contains a per-block
    /// `replace_all: true` in one of its entries (here, the SECOND block,
    /// so the branch's `.any(...)` scan must not stop at the first,
    /// non-qualifying block) qualifies — `tool_input_has_replace_all_true`
    /// must return `true`.
    #[test]
    fn test_F_C2_P1_006_multi_edit_edits_array_per_block_replace_all_true_qualifies() {
        let tool_input = serde_json::json!({
            "file_path": "decision-log.md",
            "edits": [
                {
                    "old_string": "first old",
                    "new_string": "first new",
                    "replace_all": false
                },
                {
                    "old_string": "second old",
                    "new_string": "second new",
                    "replace_all": true
                }
            ]
        });

        assert!(
            tool_input_has_replace_all_true(&tool_input),
            "F-C2-P1-006: a MultiEdit whose edits[] array contains a per-block \
             `replace_all: true` in ANY block must qualify (BC-1.18.006 Precondition 4's \
             MultiEdit scope: \"a MultiEdit call containing at least one edit block whose \
             replace_all field is true\")"
        );
    }

    /// A negative case: a `MultiEdit` whose `edits[]` array has NO block with
    /// `replace_all: true` (either omitted entirely, or explicitly `false`)
    /// must NOT qualify — `tool_input_has_replace_all_true` must return
    /// `false`.
    #[test]
    fn test_F_C2_P1_006_multi_edit_edits_array_without_replace_all_does_not_qualify() {
        let tool_input = serde_json::json!({
            "file_path": "decision-log.md",
            "edits": [
                {
                    "old_string": "first old",
                    "new_string": "first new"
                },
                {
                    "old_string": "second old",
                    "new_string": "second new",
                    "replace_all": false
                }
            ]
        });

        assert!(
            !tool_input_has_replace_all_true(&tool_input),
            "F-C2-P1-006: a MultiEdit whose edits[] array contains NO block with \
             `replace_all: true` must NOT qualify — a plain MultiEdit without replace_all is \
             unaffected by BC-1.18.006 Precondition 4/Postcondition 7 (per Precondition 4's own \
             closure-scope statement)"
        );
    }
}

// ---------------------------------------------------------------------------
// ADR-032-AC021-prereq: dispatcher git_context extension tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod git_context_prereq_tests {
    use super::*;

    /// Initialize a minimal git repo at `dir` suitable for build_git_context tests.
    /// Sets user.name and user.email locally so tests pass in CI environments
    /// where the global git config may not be set.
    fn git_init_and_configure(dir: &std::path::Path) {
        let run = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(dir)
                .output()
                .expect("git command failed")
        };
        run(&["init", "-b", "factory-artifacts"]);
        run(&["config", "user.email", "test@vsdd.local"]);
        run(&["config", "user.name", "VSDD Test"]);
    }

    fn git_run(dir: &std::path::Path, args: &[&str]) {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .expect("git command failed");
        assert!(
            out.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    // -----------------------------------------------------------------------
    // ADR-032-AC021-prereq Dispatcher Test 1
    //
    // build_git_context(factory_dir).to_json() contains all three new keys
    // (head_state_timestamp, head_parent_state_timestamp, state_md_in_commit)
    // with correct values when STATE.md is committed with a known timestamp.
    //
    // Construct a temp git repo with:
    //   Commit 1: STATE.md containing timestamp: "2026-07-20T09:00:00Z" (parent)
    //   Commit 2: STATE.md updated to timestamp: "2026-07-20T10:00:00Z" (HEAD)
    // Expected:
    //   head_state_timestamp = "2026-07-20T10:00:00Z"
    //   head_parent_state_timestamp = "2026-07-20T09:00:00Z"
    //   state_md_in_commit = "true" (STATE.md changed in HEAD)
    // -----------------------------------------------------------------------
    #[test]
    fn adr032_ac021_prereq_test1_all_three_new_keys_populated() {
        let dir = tempfile::tempdir().expect("create tmpdir for test 1");
        let d = dir.path();

        git_init_and_configure(d);

        // Commit 1: parent STATE.md with timestamp TS_PARENT.
        std::fs::write(
            d.join("STATE.md"),
            "---\ntimestamp: \"2026-07-20T09:00:00Z\"\nphase: test\n---\n\n# STATE\n",
        )
        .unwrap();
        git_run(d, &["add", "STATE.md"]);
        git_run(d, &["commit", "-m", "parent commit with STATE.md"]);

        // Commit 2 (HEAD): STATE.md updated with timestamp TS_HEAD.
        std::fs::write(
            d.join("STATE.md"),
            "---\ntimestamp: \"2026-07-20T10:00:00Z\"\nphase: complete\n---\n\n# STATE\n",
        )
        .unwrap();
        git_run(d, &["add", "STATE.md"]);
        git_run(d, &["commit", "-m", "HEAD commit with updated STATE.md"]);

        let ctx = build_git_context(d);
        let json = ctx.to_json();

        assert_eq!(
            json["head_state_timestamp"],
            serde_json::json!("2026-07-20T10:00:00Z"),
            "ADR-032-AC021-prereq Test 1: head_state_timestamp must be the timestamp \
             from HEAD:STATE.md ('2026-07-20T10:00:00Z')"
        );
        assert_eq!(
            json["head_parent_state_timestamp"],
            serde_json::json!("2026-07-20T09:00:00Z"),
            "ADR-032-AC021-prereq Test 1: head_parent_state_timestamp must be the timestamp \
             from HEAD^:STATE.md ('2026-07-20T09:00:00Z')"
        );
        assert_eq!(
            json["state_md_in_commit"],
            serde_json::json!("true"),
            "ADR-032-AC021-prereq Test 1: state_md_in_commit must be 'true' — \
             STATE.md changed in HEAD commit"
        );
        // Confirm all 7 keys are present in the JSON object (ADR-032 §Prerequisite item 5).
        for key in &[
            "head_subject",
            "head_sha",
            "head_parent_subject",
            "head_parent_sha",
            "head_state_timestamp",
            "head_parent_state_timestamp",
            "state_md_in_commit",
        ] {
            assert!(
                json.get(key).is_some(),
                "ADR-032-AC021-prereq Test 1: to_json() must include key '{key}'"
            );
        }
    }

    // -----------------------------------------------------------------------
    // ADR-032-AC021-prereq Dispatcher Test 2
    //
    // inject_git_context_if_qualifying with a qualifying PostToolUse Bash
    // git-commit event where the committed file set includes STATE.md produces
    // a payload where git_context["state_md_in_commit"] == "true".
    //
    // Same two-commit temp repo as Test 1. The qualifying payload simulates:
    //   event_name = PostToolUse, tool_name = Bash,
    //   command = "git -C .factory commit -m 'update'"
    // -----------------------------------------------------------------------
    #[test]
    fn adr032_ac021_prereq_test2_inject_qualifying_state_md_commit_sets_state_md_in_commit_true() {
        let dir = tempfile::tempdir().expect("create tmpdir for test 2");
        let d = dir.path();

        git_init_and_configure(d);

        // Commit 1 (parent).
        std::fs::write(
            d.join("STATE.md"),
            "---\ntimestamp: \"2026-07-20T09:00:00Z\"\n---\n\n# STATE\n",
        )
        .unwrap();
        git_run(d, &["add", "STATE.md"]);
        git_run(d, &["commit", "-m", "parent"]);

        // Commit 2 (HEAD): STATE.md updated.
        std::fs::write(
            d.join("STATE.md"),
            "---\ntimestamp: \"2026-07-20T10:00:00Z\"\n---\n\n# STATE\n",
        )
        .unwrap();
        git_run(d, &["add", "STATE.md"]);
        git_run(d, &["commit", "-m", "state: update STATE.md"]);

        // Build a qualifying PostToolUse Bash git-commit payload.
        // The command must contain "git", " commit", and ".factory" to pass
        // detect_git_commit_event. inject_git_context_if_qualifying then calls
        // build_git_context(d) which reads from the actual temp git repo.
        let payload_json = serde_json::json!({
            "event_name": "PostToolUse",
            "session_id": "test-session",
            "tool_name": "Bash",
            "tool_input": { "command": "git -C .factory commit -m 'state: update STATE.md'" },
            "tool_response": {},
        });
        let original_payload: crate::payload::HookPayload =
            serde_json::from_value(payload_json.clone()).expect("parse HookPayload");
        let mut payload_value = payload_json;

        inject_git_context_if_qualifying(&original_payload, &mut payload_value, d);

        let state_md_flag = payload_value
            .get("git_context")
            .and_then(|gc| gc.get("state_md_in_commit"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        assert_eq!(
            state_md_flag, "true",
            "ADR-032-AC021-prereq Test 2: inject_git_context_if_qualifying must set \
             git_context[state_md_in_commit] = 'true' when STATE.md was committed at HEAD; \
             got: {state_md_flag:?}"
        );
    }

    // -----------------------------------------------------------------------
    // ADR-032-AC021-prereq Dispatcher Test 3 (negative branch)
    //
    // When the HEAD commit does NOT include STATE.md (e.g. a burst-log-only
    // commit), build_git_context(factory_dir).to_json() produces
    // state_md_in_commit == "false".
    //
    // Setup:
    //   Commit 1: STATE.md (parent commit)
    //   Commit 2: only burst-log.md committed (HEAD does NOT include STATE.md)
    // Expected: state_md_in_commit = "false"
    // -----------------------------------------------------------------------
    #[test]
    fn adr032_ac021_prereq_test3_negative_branch_no_state_md_in_commit_returns_false() {
        let dir = tempfile::tempdir().expect("create tmpdir for test 3");
        let d = dir.path();

        git_init_and_configure(d);

        // Commit 1 (parent): commit STATE.md.
        std::fs::write(
            d.join("STATE.md"),
            "---\ntimestamp: \"2026-07-20T09:00:00Z\"\n---\n\n# STATE\n",
        )
        .unwrap();
        git_run(d, &["add", "STATE.md"]);
        git_run(d, &["commit", "-m", "initial STATE.md commit"]);

        // Commit 2 (HEAD): commit only burst-log.md — STATE.md NOT changed.
        std::fs::write(d.join("burst-log.md"), "## Burst log entry\n").unwrap();
        git_run(d, &["add", "burst-log.md"]);
        git_run(d, &["commit", "-m", "state: burst-log only (no STATE.md)"]);

        let json = build_git_context(d).to_json();

        assert_eq!(
            json["state_md_in_commit"],
            serde_json::json!("false"),
            "ADR-032-AC021-prereq Test 3: state_md_in_commit must be 'false' when \
             the HEAD commit does not include STATE.md (burst-log-only commit); \
             got: {:?}",
            json["state_md_in_commit"]
        );
    }
}
