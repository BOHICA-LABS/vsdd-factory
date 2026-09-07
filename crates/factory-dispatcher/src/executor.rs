//! Tiered parallel execution of matched plugins (S-1.6).
//!
//! Consumers of this module hand it the ordered tiers produced by
//! [`crate::routing::group_by_priority`] and get back a
//! [`TierExecutionSummary`] that records every plugin's outcome plus a
//! dispatcher-level exit code.
//!
//! Within a tier: plugins run concurrently via tokio tasks; each task
//! wraps the synchronous [`crate::invoke::invoke_plugin`] in
//! `spawn_blocking` so wasmtime is never blocking the runtime. Between
//! tiers: the dispatcher awaits every task before the next tier begins,
//! preserving priority ordering.
//!
//! Advisory-block semantics (per Q3 resolution + W-15 gate fix CRIT-PR59-001):
//! a plugin that writes `{"outcome":"block","reason":"..."}` to stdout
//! records a dispatcher-level block intent regardless of `on_error` setting.
//! The `on_error` field governs fail-closed semantics for crash and timeout:
//! a sync-group plugin that Crashes or times out with `on_error=Block` triggers
//! fail-closed exit 2 (ADR-019 §Decision 2). Async hooks never trigger fail-closed
//! (async block verdicts are advisory-only per ADR-019). The summary's
//! `exit_code` is 2 iff any block intent was recorded.

use std::sync::Arc;
use std::time::Instant;

use wasmtime::Engine;

use crate::host::HostContext;
use crate::indeterminate_marker::{
    MarkerFields, UNVALIDATED_MUTATION_MARKER_TTL_SECONDS, block_if_marker_check,
    check_and_clear_expired_marker, delete_marker_if_pass, emit_marker_cleared,
    emit_write_tied_audit_events, read_all_marker_fields, read_marker_plugin_name,
    reconcile_raw_delete, should_write_marker, write_indeterminate_marker,
};
use crate::internal_log::{
    InternalEvent, InternalLog, PLUGIN_COMPLETED, PLUGIN_CRASHED, PLUGIN_INDETERMINATE,
    PLUGIN_INVOKED, PLUGIN_TIMEOUT,
};
use crate::invoke::{EventType, InvokeLimits, PluginResult, TimeoutCause, invoke_plugin};
use crate::plugin_loader::PluginCache;
use crate::registry::{FailurePolicy, OnError, Registry, RegistryEntry};
use crate::resolver::{ResolverInput, ResolverRegistry, merge_resolver_outputs};

// ---------------------------------------------------------------------------
// S-25.01: INDETERMINATE outcome class (BC-1.18.001, ADR-047 Layer 1)
// ---------------------------------------------------------------------------

/// Root cause of an INDETERMINATE dispatch outcome.
///
/// Distinct values for each resource-exhaustion class so operators can
/// distinguish fuel exhaustion, epoch timeout, and OutputTooLarge events
/// in the `plugin.indeterminate` event (BC-3.08.001 Event 8).
///
/// VP-102 proof harness covers the full cause-mapping table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndeterminateCause {
    /// WASM execution ran out of fuel budget (Trap::OutOfFuel).
    Fuel,
    /// WASM execution exceeded epoch/wall-clock budget (Trap::Interrupt).
    Epoch,
    /// A host function returned OutputTooLarge(-3) and the plugin subsequently
    /// exited with exit_code=0 — the plugin did not detect the truncation.
    OutputTooLarge,
}

/// Three-valued dispatch outcome for a single plugin invocation.
///
/// This is a first-class, distinct type from the existing `PluginResult` taxonomy.
/// `PluginResult` captures the raw invocation result; `DispatchOutcome` is the
/// CLASSIFIED semantic outcome after applying INDETERMINATE detection rules.
///
/// - **Pass**: exit_code=0 AND host_output_too_large_seen=false.
/// - **Fail**: non-zero exit_code.
/// - **Indeterminate**: could not validate — fuel/epoch/OutputTooLarge.
///
/// S-25.01 BC-1.18.001 postcondition 2: these three variants are the complete,
/// non-overlapping trichotomy. VP-102 proof harness verifies exhaustive coverage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchOutcome {
    /// Plugin validated successfully: exit_code=0 and no OutputTooLarge flag set.
    Pass,
    /// Plugin returned a non-zero exit code (explicit validation failure).
    Fail {
        /// The non-zero exit code returned by the plugin.
        exit_code: i32,
    },
    /// Plugin could not complete validation (fuel, epoch, or OutputTooLarge).
    ///
    /// For fail-closed plugins: triggers durable `.factory/unvalidated-mutation.marker` write.
    /// For fail-open plugins: advisory `plugin.indeterminate` event only (BC-1.18.004).
    Indeterminate {
        /// Root cause distinguishing the three INDETERMINATE sub-cases.
        cause: IndeterminateCause,
    },
}

/// Classify a raw `PluginResult` + `FailurePolicy` + `output_too_large` flag
/// into the three-valued `DispatchOutcome` trichotomy.
///
/// This is the pure-core function covering the INDETERMINATE detection logic
/// (BC-1.18.001). It does NOT write the marker or emit events — those are
/// effectful operations in the invocation loop.
///
/// # Fuel detection
///
/// Fuel exhaustion MUST be detected via `Trap::OutOfFuel` downcast on the
/// `PluginResult::Timeout { cause: TimeoutCause::Fuel }` variant, NOT via
/// `get_fuel()`. `get_fuel()` is unreliable after a Trap (BC-1.18.001 §Architecture
/// Anchors; ADR-047 §D1 implementation note). AC-001.
///
/// # Non-exhaustive Trap wildcard (BC-1.18.001 invariant 2)
///
/// `Trap` is `#[non_exhaustive]`. The match arm for unrecognised Trap variants
/// MUST use `_ => { /* route to on_error, NOT Indeterminate */ }`. Future unknown
/// Trap variants MUST NOT be silently bucketed as INDETERMINATE. AC-004.
///
/// # OutputTooLarge invariant
///
/// The `output_too_large` flag MUST be captured from `StoreData` AFTER `func.call()`
/// completes (before the per-invocation reset). It is the caller's responsibility
/// to pass the captured value. BC-1.18.001 invariant 5: the flag is dispatcher-
/// internal StoreData; never exposed in hook-sdk ABI. AC-003/AC-018.
///
/// # VP
///
/// VP-102 proof harness covers this function's 5 unit test cases.
///
/// # BC-5.38.001
///
/// Non-trivial body — contains branching, pattern matching, conditional returns.
/// todo!() per Red Gate discipline. Implementer fills in.
pub fn classify_outcome(
    plugin_result: PluginResult,
    _policy: FailurePolicy,
    output_too_large: bool,
) -> DispatchOutcome {
    // NOTE (S-25.01 orchestrator ruling): `_policy` is genuinely unused inside
    // classify_outcome — classification is independent of policy. Policy is only
    // used downstream by `should_write_marker`. The parameter is retained in the
    // spec-mandated signature (spec-wins; BC-1.18.001 AC-004 signature). Surface
    // to product-owner as possible spec-signature refinement (per orchestrator note).
    //
    // BC-1.18.001 postcondition 1 + postcondition 2 + postcondition 5 + invariant 2.
    // VP-102 proof harness covers this function's 5 unit test cases.
    match plugin_result {
        // Timeout variants: fuel or epoch exhaustion → INDETERMINATE.
        // Fuel detection MUST use the Timeout{cause:Fuel} variant (mapped from
        // Trap::OutOfFuel in invoke.rs::classify_trap), NOT get_fuel(). AC-001/AC-002.
        PluginResult::Timeout {
            cause: crate::invoke::TimeoutCause::Fuel,
            ..
        } => DispatchOutcome::Indeterminate {
            cause: IndeterminateCause::Fuel,
        },
        PluginResult::Timeout {
            cause: crate::invoke::TimeoutCause::Epoch,
            ..
        } => DispatchOutcome::Indeterminate {
            cause: IndeterminateCause::Epoch,
        },

        // Ok variant: trichotomy depends on exit_code + output_too_large flag.
        // AC-003: exit_code=0 + output_too_large=true → Indeterminate(OutputTooLarge).
        // AC-004: exit_code=0 + output_too_large=false → Pass.
        // AC-004: exit_code≠0 → Fail{exit_code} (policy-orthogonal).
        PluginResult::Ok { exit_code, .. } => {
            if output_too_large && exit_code == 0 {
                DispatchOutcome::Indeterminate {
                    cause: IndeterminateCause::OutputTooLarge,
                }
            } else if exit_code == 0 {
                DispatchOutcome::Pass
            } else {
                DispatchOutcome::Fail { exit_code }
            }
        }

        // Crashed variant: unrecognized/future Trap variant (wildcard `_ =>` arm in
        // invoke.rs::classify_trap). MUST NOT yield INDETERMINATE per BC-1.18.001
        // invariant 2 (`Trap` is `#[non_exhaustive]`). Route to existing on_error
        // handling by returning Fail (non-zero). AC-004/AC-011.
        PluginResult::Crashed { .. } => {
            // BC-1.18.001 invariant 2: wildcard Trap arm routes to on_error, not INDETERMINATE.
            // Existing plugin_fail_closed() checks Crashed+on_error=Block → exit 2.
            // classify_outcome returns Fail so the executor can apply on_error logic.
            DispatchOutcome::Fail { exit_code: 1 }
        }
    }
}

/// Owned per-plugin outcome with a name attached so callers don't have
/// to zip with the original tier vec. `RegistryEntry` is cloned here —
/// it's small (no WASM binary), and cloning sidesteps borrow-plumbing
/// through the tokio tasks.
#[derive(Debug, Clone)]
pub struct PluginOutcome {
    pub plugin_name: String,
    pub plugin_version: String,
    pub on_error: OnError,
    pub result: PluginResult,
    /// Set to `true` by `execute_tiers` when `plugin_block_if_marker` returned `true`
    /// for this outcome — i.e., `on_error == BlockIfMarker` AND the
    /// `.factory/unvalidated-mutation.marker` was present and non-expired at dispatch time.
    ///
    /// Used by `extract_block_info` (TD #71 surfacing path) to populate
    /// `blocking_plugins` + `block_reason` for the recoverable-block case
    /// (ADR-048 §Decision 1 / BC-1.18.002 PC5).
    ///
    /// `false` for all other outcomes (advisory block, fail-closed Block,
    /// async plugins, load failures).
    pub block_if_marker_fired: bool,
    /// The marker's parsed fields, populated by `execute_tiers` when
    /// `block_if_marker_fired` is `true` (best-effort read of
    /// `.factory/unvalidated-mutation.marker` at the moment the crash-path
    /// block was confirmed).
    ///
    /// Used by `extract_block_info` (TD #71 surfacing path) / `main.rs`'s
    /// crash-path BLOCK message construction to populate the mandatory
    /// `plugin_name`, `artifact_path`, `cause`, `expires_at` fields required
    /// by BC-1.18.002 v1.6 PC5 — the message MUST name the concrete marker
    /// so the operator/agent can act on it, not just assert a marker exists.
    ///
    /// `None` when `block_if_marker_fired` is `false` (no marker to read),
    /// or on the defensive fallback where the marker could not be re-read
    /// (e.g., concurrently deleted between the crash-path check and the
    /// field read — the marker's prior presence already satisfied PC5's
    /// block decision; a `None` here degrades the message gracefully
    /// rather than fabricating field values).
    ///
    /// Boxed: `MarkerFields` carries six owned `String`s, and `PluginOutcome`
    /// is stored inline in `JoinWrap::Ready` alongside a bare `JoinHandle`
    /// (8 bytes) — an unboxed `Option<MarkerFields>` would blow up
    /// `clippy::large_enum_variant` on `JoinWrap`. `Option<Box<_>>` keeps
    /// this field pointer-sized regardless of `MarkerFields`' own size.
    pub block_if_marker_fields: Option<Box<MarkerFields>>,
}

/// Aggregated result of running every tier in order.
#[derive(Debug, Clone)]
pub struct TierExecutionSummary {
    pub per_plugin_results: Vec<PluginOutcome>,
    pub total_elapsed_ms: u64,
    pub block_intent: bool,
    pub exit_code: i32,
}

/// Inputs into a single `execute_tiers` call. Borrowing the engine and
/// cache; owning the registry + payload so the caller can construct
/// them fresh per invocation.
///
/// `payload_value` is the *base* envelope (with `dispatcher_trace_id`
/// already injected by main.rs). The executor deep-clones it per plugin
/// and splices in `plugin_config` from the registry entry before
/// serializing to bytes for invoke. Per-plugin spliced bytes mean two
/// plugins in the same tier never see each other's config — exactly
/// what the legacy-bash-adapter (S-2.1) needs to multiplex over a
/// single shared adapter wasm.
///
/// `resolver_registry` is the in-process context-resolver registry built
/// from `resolvers-registry.toml`. It is consulted for each hook entry's
/// `needs_context` list before `plugin_config` is spliced in.
/// An empty registry (no resolvers registered) is valid and produces
/// zero overhead via the `needs_context.is_empty()` short-circuit
/// (BC-1.13.001 PC3 / AC-002).
pub struct ExecutorInputs<'a> {
    pub engine: &'a Engine,
    pub cache: &'a PluginCache,
    pub registry: &'a Registry,
    pub payload_value: serde_json::Value,
    pub base_host_ctx: HostContext,
    /// Mirror of the dispatcher's internal log, used to emit plugin
    /// lifecycle events. Held in an `Arc` so per-plugin tasks can
    /// reach it without cloning the whole log.
    pub internal_log: Arc<InternalLog>,
    /// In-process context-resolver registry. Queried per hook entry
    /// for each name in `entry.needs_context`. Pass
    /// `Arc::new(ResolverRegistry::new())` when no resolvers are
    /// configured (BC-1.13.001 INV2).
    pub resolver_registry: Arc<ResolverRegistry>,
}

/// Well-known path (relative to the project cwd) of the `[[shard]]` config
/// file the native shard-cap gate check reads (BC-1.18.005 Precondition 2).
/// TBD-at-F4 per BC-1.18.005's Architecture Anchors — a sibling
/// `shard-config.toml` rather than `hooks-registry.toml` itself, so the
/// gate's config surface can evolve independently of the WASM plugin
/// registry schema.
const SHARD_CONFIG_RELATIVE_PATH: &str = ".factory/shard-config.toml";

/// Native (non-WASM) shard-cap gate invocation point (S-25.02 BC-1.18.005
/// T-2). Called from `execute_tiers` BEFORE the registry-driven tier loop
/// (Invariant 1's placement requirement — architecturally analogous to the
/// `block_if_marker_check` native-check precedent in `indeterminate_marker.rs`).
///
/// # Guarded call site (BC-5.38.001 Red Gate discipline)
///
/// [`crate::shard_manager::shard_cap_gate_check`] and `ShardRegistry::load`
/// are fully implemented (S-25.02 F4 BC-cluster 1 — no longer stubs). This
/// function still applies TWO cheap, real guards before reaching them —
/// themselves a direct extension of Postcondition 1's zero-cost-bypass
/// spirit, not the BC's tested formula/trigger logic — so that every dispatch
/// that isn't a candidate for BC-1.18.005's check pays zero cost:
///
/// 1. **Tool-name filter.** Only `Edit`/`Write`/`MultiEdit` PreToolUse calls
///    are candidates at all (Precondition 1). A cheap string compare, no I/O.
/// 2. **Config-presence filter.** The native gate is a no-op when no
///    `[[shard]]` config file exists at [`SHARD_CONFIG_RELATIVE_PATH`] —
///    none of this crate's pre-existing test fixtures ship one, so this
///    guard is what keeps every pre-existing dispatch unaffected by the
///    gate. The test-writer stage's BC-1.18.005 Red Gate fixtures place a
///    real `[[shard]]` config file under a test's `cwd`, which is exactly
///    what drives a matching call past the guard and into the live
///    `ShardRegistry::load` / `shard_cap_gate_check` gate logic.
///
/// Returns `None` when any guard short-circuits (no gate check performed);
/// `Some(HookResult)` when the gate was actually invoked.
///
/// # PreToolUse-scoped (F-C1-P2-004, S-25.02 Phase F4 LOCAL adversary pass-2
/// cluster-1, LOW; BC-1.18.005 Precondition 1)
///
/// The native shard-cap gate is a `PreToolUse`-only check — it exists to stop
/// a mutation BEFORE it lands, not to audit one after the fact. The
/// `event_name` guard below reads the harness's `EventType` classification
/// (mirroring `main.rs`'s `EventType::from_event_str(&payload.event_name)`)
/// and short-circuits for any event that is explicitly NOT `PreToolUse` —
/// e.g. a `PostToolUse` Edit/Write/MultiEdit call against the exact same
/// matched, malformed `[[shard]]` entry that would legitimately block a
/// PreToolUse call. A dispatch that omits `event_name` entirely (as this
/// crate's own PreToolUse-only test fixtures above do, since they predate
/// this guard) is treated as `PreToolUse`-equivalent for backward
/// compatibility with those fixtures — only an EXPLICIT non-PreToolUse
/// `event_name` opts a dispatch out.
fn shard_cap_precheck(inputs: &ExecutorInputs<'_>) -> Option<vsdd_hook_sdk::HookResult> {
    // PR #818 fix-burst finding m4: named, documented default rather than an
    // implicit `.unwrap_or(...)` fallback that reads as ad hoc fixture
    // convenience. See this function's own doc comment above for the full
    // backward-compatibility rationale (this crate's pre-existing
    // PreToolUse-only test fixtures predate the `event_name` guard and never
    // populate this field) — this constant makes that a deliberate,
    // named choice rather than an accidental one.
    const DEFAULT_EVENT_TYPE_WHEN_ABSENT: EventType = EventType::PreToolUse;
    let event_type = inputs
        .payload_value
        .get("event_name")
        .and_then(|v| v.as_str())
        .map(EventType::from_event_str)
        .unwrap_or(DEFAULT_EVENT_TYPE_WHEN_ABSENT);
    if event_type != EventType::PreToolUse {
        return None;
    }

    let tool_name = inputs
        .payload_value
        .get("tool_name")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !matches!(tool_name, "Edit" | "Write" | "MultiEdit") {
        return None;
    }

    let shard_config_path = inputs.base_host_ctx.cwd.join(SHARD_CONFIG_RELATIVE_PATH);
    if !shard_config_path.exists() {
        return None;
    }

    // A `[[shard]]` config file is present and this is a candidate tool —
    // from here on, real BC-1.18.005 gate logic runs: structural TOML
    // config load, followed by `shard_cap_gate_check`'s entry-match-time
    // semantic validation (EC-009/EC-010/EC-011/EC-012/Postcondition 9, via
    // `validate_entry`) and the trigger-boundary check itself.
    let registry = match crate::shard_manager::ShardRegistry::load(&shard_config_path) {
        Ok(reg) => reg,
        Err(e) => return Some(e.into()),
    };
    let target_path = inputs
        .payload_value
        .get("tool_input")
        .and_then(|v| v.get("file_path"))
        .and_then(|v| v.as_str())
        .map(std::path::PathBuf::from)
        .unwrap_or_default();
    let tool_input = inputs
        .payload_value
        .get("tool_input")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    Some(crate::shard_manager::shard_cap_gate_check(
        &registry,
        tool_name,
        &target_path,
        &tool_input,
    ))
}

/// Synthesize a blocking [`PluginOutcome`] for the native shard-cap gate's
/// own fail-loud verdict (F-C1-P2-001, S-25.02 Phase F4 LOCAL adversary
/// pass-2 cluster-1, MEDIUM).
///
/// Before this fix, `execute_tiers`'s shard-gate match arm only flipped the
/// local `block_intent` bool for a `HookResult::Error`/`HookResult::Block`
/// verdict — it never appended anything to `all_outcomes` (the
/// `Vec<PluginOutcome>` that becomes `TierExecutionSummary::per_plugin_results`).
/// `main.rs::extract_block_info` (the TD #71 operator-facing surfacing path)
/// derives `block_reason` EXCLUSIVELY by scanning `per_plugin_results`, so an
/// empty tier list (the native gate's own "nothing else runs" shape) meant
/// `block_reason` was always `""` for a malformed `[[shard]]` config —
/// silently dropping the artifact_stem + failure-kind diagnostic
/// `ShardConfigError`'s own `Display` impl already carries.
///
/// This synthesizes a `PluginOutcome` shaped exactly like a real WASM
/// plugin's advisory-block verdict — `stdout` carrying
/// `{"outcome":"block","reason":"..."}`, `on_error = Block`, `exit_code = 2`
/// — so it flows through `extract_block_info`'s existing `is_blocking`
/// check (the `advisory` arm) and `extract_reason_from_outcome`'s existing
/// JSON-reason parse exactly the way a real plugin's verdict would, with no
/// parallel surfacing path required. `plugin_name` is `"shard-cap-gate"` so
/// an operator can distinguish this native gate's own verdict from a WASM
/// plugin's in `blocking_plugins`. `message` is either `ShardConfigError`'s
/// own `Display` text (`HookResult::Error` case — already names the
/// offending `artifact_stem` and the `EC-009`/`EC-011`/`EC-013` BC-1.18.005
/// failure-kind marker) or a `HookResult::Block`'s `reason` (reserved for a
/// later cluster's fired-trigger block outcome; `shard_cap_gate_check` does
/// not construct one today — see `shard_cap_precheck`'s doc comment).
///
/// `plugin_version` (PR #818 fix-burst finding m5, corrected from an
/// earlier revision that hardcoded `String::new()`): callers pass
/// `inputs.base_host_ctx.plugin_version.clone()` — the same dispatcher-own
/// version every OTHER `PluginOutcome` in this file populates via
/// `host_ctx.plugin_version`/`base_host_ctx.plugin_version` (this native
/// gate is dispatcher code, not a versioned WASM plugin, so there is no
/// separate "shard-cap-gate plugin version" to report; the dispatcher's own
/// version is the accurate value, consistent with how this file already
/// reports `plugin_version` for its OTHER native/sentinel outcomes, e.g. the
/// `payload serialize`/`plugin load failed` crash paths above).
///
/// On `plugins_run` counting (m5's second half): this function's outcome
/// intentionally DOES count toward `TierExecutionSummary::per_plugin_results
/// .len()` (and therefore the operator-facing `plugins_run` telemetry field
/// in `main.rs`) exactly like this file's other native/sentinel outcomes —
/// the `spawn_blocking` join-error `"<unknown>"` crash outcome, the
/// `"payload serialize"` crash outcome, and the `"plugin load failed"` crash
/// outcome all count identically despite none of them representing a
/// successfully-executed WASM plugin. `plugins_run` has always meant "number
/// of outcome-producing checks this dispatch ran," not "number of WASM
/// modules that executed successfully" — this native gate check fits that
/// established, pre-existing semantics precisely. (Reviewed against a fix
/// suggestion to exclude this outcome from the count: doing so would
/// actually be the INCONSISTENT choice, singling out this one native check
/// while leaving the other three sentinel-outcome sites uncorrected.)
fn shard_gate_block_outcome(message: String, plugin_version: String) -> PluginOutcome {
    let stdout = serde_json::json!({ "outcome": "block", "reason": message }).to_string();
    PluginOutcome {
        plugin_name: "shard-cap-gate".to_string(),
        plugin_version,
        on_error: OnError::Block,
        result: PluginResult::Ok {
            exit_code: 2,
            stdout,
            stderr: String::new(),
            elapsed_ms: 0,
            fuel_consumed: 0,
        },
        block_if_marker_fired: false,
        block_if_marker_fields: None,
    }
}

/// Run every tier and return the aggregated summary.
pub async fn execute_tiers(
    inputs: ExecutorInputs<'_>,
    tiers: Vec<Vec<&RegistryEntry>>,
) -> TierExecutionSummary {
    let started = Instant::now();
    let mut all_outcomes: Vec<PluginOutcome> = Vec::new();
    let mut block_intent = false;

    // S-25.02 BC-1.18.005 T-2 — native shard-cap gate, BEFORE the
    // registry-driven tier loop below (Invariant 1). See
    // `shard_cap_precheck`'s doc comment for the guard rationale; this call
    // is a no-op (`None`) for every dispatch that isn't both an
    // Edit/Write/MultiEdit PreToolUse call AND has a `[[shard]]` config file
    // present, which covers 100% of this crate's pre-existing test fixtures.
    // F-001 fix (S-25.02 Phase F4 LOCAL adversary pass-1 cluster-1, HIGH):
    // a fail-loud `HookResult::Error` from `shard_cap_gate_check`'s
    // entry-match-time `validate_entry` call (EC-009 missing `shape`;
    // EC-011 `low_water_mark >= N`) is BC-1.18.005's OWN postcondition and
    // MUST become a BLOCKING dispatch outcome here — the
    // same way `plugin_fail_closed`/`plugin_requests_block` translate a
    // WASM plugin's fail-closed verdict into `block_intent` below. A
    // `HookResult::Block` is likewise translated identically, though
    // `shard_cap_gate_check` does not construct one today: a fired
    // size/item-count trigger is BC-1.18.006's/BC-1.18.009's own observable
    // roll/rotate-and-retry outcome (out of scope for this cluster), so the
    // gate itself still returns `Continue` + a non-fatal `tracing::warn!`
    // advisory for a fired trigger (see `shard_manager.rs`) — this match
    // arm is wired now so that hand-off requires no further executor.rs
    // change when those later clusters land.
    // F-C1-P2-001 fix (S-25.02 Phase F4 LOCAL adversary pass-2 cluster-1,
    // MEDIUM): a fail-loud verdict here MUST also be appended to
    // `all_outcomes` (via `shard_gate_block_outcome`) — not just flip
    // `block_intent` — so `main.rs::extract_block_info`'s scan over
    // `TierExecutionSummary::per_plugin_results` has something to find. See
    // `shard_gate_block_outcome`'s doc comment for the full rationale.
    if let Some(shard_gate_result) = shard_cap_precheck(&inputs) {
        match shard_gate_result {
            vsdd_hook_sdk::HookResult::Error { message } => {
                block_intent = true;
                all_outcomes.push(shard_gate_block_outcome(
                    message,
                    inputs.base_host_ctx.plugin_version.clone(),
                ));
            }
            vsdd_hook_sdk::HookResult::Block { reason } => {
                block_intent = true;
                all_outcomes.push(shard_gate_block_outcome(
                    reason,
                    inputs.base_host_ctx.plugin_version.clone(),
                ));
            }
            vsdd_hook_sdk::HookResult::Continue => {}
        }
    }

    for tier in tiers {
        let mut tier_outcomes = execute_tier(&inputs, tier).await;
        for outcome in tier_outcomes.iter_mut() {
            let bim_fired = plugin_block_if_marker(
                &outcome.result,
                outcome.on_error,
                &inputs.base_host_ctx.cwd,
                chrono::Utc::now(),
            );
            // Record per-plugin whether block_if_marker fired so extract_block_info
            // can surface a non-empty reason for the BlockIfMarker crash-block case
            // (TD #71 / ADR-048 §Decision 1 / BC-1.18.002 PC5).
            outcome.block_if_marker_fired = bim_fired;
            // F-P2-001 fix: when the crash-path block fired, read the marker's
            // concrete fields so the block message can name plugin_name,
            // artifact_path, cause, and expires_at instead of asserting a
            // marker exists without saying which one (BC-1.18.002 v1.6 PC5).
            // Best-effort: a read failure here (e.g. the marker was cleared by
            // a racing T1 re-validation between the crash-path check and this
            // read) leaves the field `None` — the message construction in
            // main.rs degrades gracefully rather than fabricating values.
            outcome.block_if_marker_fields = if bim_fired {
                let marker_path = inputs
                    .base_host_ctx
                    .cwd
                    .join(".factory")
                    .join("unvalidated-mutation.marker");
                read_all_marker_fields(&marker_path)
                    .ok()
                    .flatten()
                    .map(Box::new)
            } else {
                None
            };
            if plugin_requests_block(&outcome.result)
                || plugin_fail_closed(&outcome.result, outcome.on_error)
                || bim_fired
            {
                block_intent = true;
            }
        }
        all_outcomes.extend(tier_outcomes);
    }

    TierExecutionSummary {
        total_elapsed_ms: started.elapsed().as_millis() as u64,
        exit_code: if block_intent { 2 } else { 0 },
        block_intent,
        per_plugin_results: all_outcomes,
    }
}

/// Execute every plugin in a single tier concurrently.
async fn execute_tier<'a>(
    inputs: &ExecutorInputs<'a>,
    tier: Vec<&'a RegistryEntry>,
) -> Vec<PluginOutcome> {
    let mut join_handles = Vec::with_capacity(tier.len());

    for entry in tier {
        let engine = inputs.engine.clone();
        let entry_clone = entry.clone();
        let limits = InvokeLimits {
            timeout_ms: entry_clone.timeout_ms(&inputs.registry.defaults),
            fuel_cap: entry_clone.fuel_cap(&inputs.registry.defaults),
        };
        let on_error = entry_clone.on_error(&inputs.registry.defaults);

        // ADR-048 §Decision 4 v1.2 (S-25.01 LOCAL adversary pass 2 F-P2-002 HIGH +
        // F-P2-003 MED): dispatcher-native pre-check, run BEFORE invoking any
        // `on_error = "block_if_marker"` (Arm 1/Arm 2) plugin on the normal
        // (non-crash) path. TTL_EXPIRED detection + auto-clear + emission and
        // OPERATOR_OVERRIDE RAW_DELETE_DETECTED reconciliation both happen here,
        // dispatcher-native — never via the WASM `emit_event` host ABI, whose
        // RESERVED_FIELDS enrichment cannot carry the marker's foreign
        // trace_id/plugin_name (see `indeterminate_marker::check_and_clear_expired_marker`
        // doc comment). By the time the WASM gate plugin's `evaluate_gate` runs
        // below, the marker is guaranteed absent-or-non-expired.
        if on_error == OnError::BlockIfMarker {
            match check_and_clear_expired_marker(
                &inputs.base_host_ctx.cwd,
                chrono::Utc::now(),
                &inputs.base_host_ctx,
            ) {
                Ok(Some(_)) => {
                    // TTL_EXPIRED cleared and emitted — no raw-delete reconciliation
                    // needed; the marker's own quarantine trail is now closed.
                }
                Ok(None) => {
                    // Not TTL-cleared (absent, non-expired, or legacy-conservative).
                    // Best-effort: if genuinely absent, reconcile a possible operator
                    // out-of-band raw delete. `reconcile_raw_delete` itself re-checks
                    // marker absence and is a no-op when the marker is still present.
                    if let Err(e) =
                        reconcile_raw_delete(&inputs.base_host_ctx.cwd, &inputs.base_host_ctx)
                    {
                        tracing::warn!(
                            error = %e,
                            "reconcile_raw_delete: I/O error on normal-path pre-check; \
                             continuing (ADR-048 §D4 v1.2 best-effort, never gates dispatch)"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "check_and_clear_expired_marker: I/O error on normal-path pre-check; \
                         continuing without TTL clear (ADR-048 §D4 v1.2 fail-safe)"
                    );
                }
            }
        }

        // Build the merged plugin_config from static config + resolver outputs.
        // AC-002: zero-overhead short-circuit when needs_context is empty.
        // AC-003: invoke resolver and merge outputs when needs_context is non-empty.
        let trace_id = inputs
            .payload_value
            .get("dispatcher_trace_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let plugin_config = build_plugin_config(
            &entry_clone,
            &inputs.payload_value,
            &inputs.base_host_ctx,
            &inputs.resolver_registry,
            &inputs.internal_log,
            &trace_id,
        );

        // Splice this entry's per-plugin config onto the base envelope.
        // Cheap clone since `payload_value` is a small JSON tree, and
        // doing it per-plugin guarantees one entry never sees another's
        // config — even when several entries share the same wasm
        // (e.g. multiple legacy-bash-adapter registrations).
        let mut per_plugin_value = inputs.payload_value.clone();
        if let Some(map) = per_plugin_value.as_object_mut() {
            map.insert("plugin_config".to_string(), plugin_config);
        }
        let payload = match serde_json::to_vec(&per_plugin_value) {
            Ok(v) => v,
            Err(e) => {
                let result = PluginResult::Crashed {
                    trap_string: format!("payload serialize: {e}"),
                    stderr: String::new(),
                    elapsed_ms: 0,
                    fuel_consumed: 0,
                };
                emit_lifecycle(
                    &inputs.internal_log,
                    &inputs.base_host_ctx,
                    &entry_clone,
                    &result,
                );
                join_handles.push(JoinWrap::Ready(PluginOutcome {
                    plugin_name: entry_clone.name.clone(),
                    plugin_version: inputs.base_host_ctx.plugin_version.clone(),
                    on_error,
                    result,
                    block_if_marker_fired: false,
                    block_if_marker_fields: None,
                }));
                continue;
            }
        };
        let internal_log = inputs.internal_log.clone();

        let mut host_ctx = inputs.base_host_ctx.clone();
        host_ctx.plugin_name = entry_clone.name.clone();
        host_ctx.capabilities = entry_clone.capabilities.clone().unwrap_or_default();

        let module = match inputs.cache.get_or_compile(&entry_clone.plugin) {
            Ok(m) => m,
            Err(e) => {
                let result = PluginResult::Crashed {
                    trap_string: format!("plugin load failed: {e}"),
                    stderr: String::new(),
                    elapsed_ms: 0,
                    fuel_consumed: 0,
                };
                emit_lifecycle(&internal_log, &inputs.base_host_ctx, &entry_clone, &result);
                let outcome = PluginOutcome {
                    plugin_name: entry_clone.name.clone(),
                    plugin_version: host_ctx.plugin_version.clone(),
                    on_error,
                    result,
                    block_if_marker_fired: false,
                    block_if_marker_fields: None,
                };
                join_handles.push(JoinWrap::Ready(outcome));
                continue;
            }
        };

        emit_invoked(&internal_log, &inputs.base_host_ctx, &entry_clone);
        let base_ctx_for_event = inputs.base_host_ctx.clone();

        // MEDIUM-5: extract artifact_path from tool_input.file_path before the closure
        // moves inputs away (BC-1.18.001 PC4 — marker must record the artifact path).
        // Empty string when no file_path is present (e.g. non-file-mutation tool events).
        //
        // LOW-3 (S-25.01): BC-1.18.001 PC4 requires artifact_path to be an absolute path.
        // The Claude Code harness always emits absolute file_path values in tool event
        // payloads (enforced by the harness itself — the Edit/Write tools require an
        // absolute path per CLAUDE.md and the harness rejects relative paths at entry).
        // No normalization is needed; we store the value verbatim and trust the harness
        // invariant. If the harness ever changes this behavior, the marker will contain a
        // relative path (degraded but non-failing — best-effort per BC-1.18.001 PC4).
        let artifact_path_for_marker = inputs
            .payload_value
            .get("tool_input")
            .and_then(|ti| ti.get("file_path"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let handle = tokio::task::spawn_blocking(move || {
            let started = Instant::now();
            // S-25.01: invoke_plugin now returns (PluginResult, bool) where bool
            // = host_output_too_large_seen (AC-003).
            let (result, output_too_large) =
                invoke_plugin(&engine, &module, host_ctx.clone(), &payload, limits).unwrap_or_else(
                    |e| {
                        (
                            PluginResult::Crashed {
                                trap_string: format!("invoke setup error: {e}"),
                                stderr: String::new(),
                                elapsed_ms: started.elapsed().as_millis() as u64,
                                fuel_consumed: 0,
                            },
                            false,
                        )
                    },
                );
            emit_lifecycle(&internal_log, &base_ctx_for_event, &entry_clone, &result);

            // S-25.01: classify outcome + emit plugin.indeterminate event + marker write.
            let failure_policy = entry_clone.failure_policy;
            let outcome = classify_outcome(result.clone(), failure_policy, output_too_large);

            // BLOCKER-1: BC-1.18.003 PC1 + INV2 — PASS from the named plugin MUST clear
            // the marker, but ONLY if the marker's plugin_name matches this plugin (scoped).
            // MEDIUM-1 fix (S-25.01): BC-1.18.003 PC1 requires the clear happen ONLY when the
            // named plugin is dispatched in a PostToolUse hook and produces Pass. A PreToolUse
            // PASS from the named plugin MUST NOT clear the marker.
            if let DispatchOutcome::Pass = outcome {
                let marker_path = base_ctx_for_event
                    .cwd
                    .join(".factory")
                    .join("unvalidated-mutation.marker");
                // Best-effort read; if the marker is absent or unreadable, no-op.
                // Scoped clear: only this plugin's PostToolUse PASS clears its own marker.
                // M-1 fix (S-25.01): pass artifact_path_for_marker so delete_marker_if_pass
                // enforces BC-1.18.003 INV2 artifact-scoped clear internally.
                // Log (but do not propagate) errors — a clear failure must not fail the dispatch.
                if let Ok(Some(marker_plugin)) = read_marker_plugin_name(&marker_path)
                    && marker_plugin == entry_clone.name
                    && entry_clone.event == "PostToolUse"
                {
                    // ADR-048 v1.1: read all marker fields BEFORE delete so we have the
                    // trace_id and other provenance fields for the marker.cleared event.
                    let all_fields = read_all_marker_fields(&marker_path).ok().flatten();
                    match delete_marker_if_pass(&marker_path, &artifact_path_for_marker) {
                        Ok(true) => {
                            // Marker was actually removed — emit marker.cleared(REVALIDATED).
                            if let Some(ref fields) = all_fields {
                                emit_marker_cleared(
                                    &base_ctx_for_event,
                                    fields,
                                    "REVALIDATED",
                                    "validator",
                                    None,
                                );
                            }
                        }
                        Ok(false) => {}
                        Err(e) => {
                            tracing::warn!(
                                plugin = %entry_clone.name,
                                marker_path = %marker_path.display(),
                                error = %e,
                                "best-effort marker clear failed on PASS; dispatch continues"
                            );
                        }
                    }
                }
            }

            if let DispatchOutcome::Indeterminate { ref cause } = outcome {
                // Emit plugin.indeterminate for EVERY indeterminate outcome (AC-006).
                // HIGH-1: pass real artifact_path so event and marker record the same path.
                emit_indeterminate(
                    &base_ctx_for_event,
                    &entry_clone,
                    cause,
                    &artifact_path_for_marker,
                );
                // Write durable marker only for fail-closed plugins (AC-005/AC-015).
                // BLOCKER-2: BC-1.18.001 INV4 — marker write is PostToolUse-only.
                // PreToolUse INDETERMINATE → advisory event only; NO marker written.
                if should_write_marker(&outcome, failure_policy)
                    && entry_clone.event == "PostToolUse"
                {
                    let marker_path = base_ctx_for_event
                        .cwd
                        .join(".factory")
                        .join("unvalidated-mutation.marker");
                    let marker_now = chrono::Utc::now();
                    let fields = MarkerFields {
                        timestamp: marker_now.to_rfc3339(),
                        plugin_name: entry_clone.name.clone(),
                        // MEDIUM-5: thread artifact_path from tool_input.file_path (AC-007).
                        artifact_path: artifact_path_for_marker,
                        cause: cause_to_str(cause).to_string(),
                        trace_id: base_ctx_for_event.dispatcher_trace_id.clone(),
                        // ADR-048 §Decision 2: 24-hour deadman TTL.
                        expires_at: (marker_now
                            + chrono::Duration::seconds(
                                UNVALIDATED_MUTATION_MARKER_TTL_SECONDS as i64,
                            ))
                        .to_rfc3339(),
                    };
                    // F-P3-002 (ADR-048 §D4 v1.3): read the pre-existing marker BEFORE
                    // the overwrite so its fields are captured prior to being clobbered
                    // by the rename. F-P9-001 (ADR-048 §D4 v1.5, symmetric to the v1.4
                    // marker.written fix): emit marker.cleared(SUPERSEDED) for it ONLY
                    // immediately after a confirmed successful write — never before the
                    // write is attempted, never on Err. Emitting SUPERSEDED
                    // unconditionally (before the write) falsely records the old marker
                    // as overwritten even when write_indeterminate_marker returns Err
                    // and the old marker is still on disk untouched — otherwise
                    // reconcile_raw_delete would later mis-attribute the superseded
                    // pair's clearance to a human OPERATOR_OVERRIDE that never happened
                    // (BC-1.18.001 INV3 last-writer-wins requires the audit trail to
                    // reflect what actually happened, not what was attempted).
                    let existing_marker = read_all_marker_fields(&marker_path).ok().flatten();
                    // HIGH-2: log marker-write failures instead of silently swallowing them.
                    // Best-effort: write failure does NOT fail the dispatch result.
                    // The plugin.indeterminate event was already emitted above.
                    //
                    // TD-VSDD-060 (F-P12-001 pre-req): the tied emission decision
                    // (SUPERSEDED-then-written on Ok, nothing on Err) is delegated
                    // to emit_write_tied_audit_events — the single source of truth
                    // for this discipline, shared with spawn_async_plugin below.
                    let write_result = write_indeterminate_marker(&fields, &marker_path);
                    emit_write_tied_audit_events(
                        &base_ctx_for_event,
                        write_result,
                        &marker_path,
                        existing_marker.as_ref(),
                        &fields,
                    );
                }
            }

            PluginOutcome {
                plugin_name: entry_clone.name.clone(),
                plugin_version: host_ctx.plugin_version.clone(),
                on_error,
                result,
                // block_if_marker_fired is set post-hoc by execute_tiers after the
                // tier completes, so this initial value is always false.
                block_if_marker_fired: false,
                block_if_marker_fields: None,
            }
        });
        join_handles.push(JoinWrap::Pending(handle));
    }

    let mut outcomes = Vec::with_capacity(join_handles.len());
    for wrap in join_handles {
        match wrap {
            JoinWrap::Ready(o) => outcomes.push(o),
            JoinWrap::Pending(h) => match h.await {
                Ok(outcome) => outcomes.push(outcome),
                Err(join_err) => {
                    // A tokio JoinError here means spawn_blocking itself
                    // panicked in its harness — extremely rare, but treat
                    // as a plugin crash so the dispatcher stays up.
                    outcomes.push(PluginOutcome {
                        plugin_name: "<unknown>".into(),
                        plugin_version: "".into(),
                        on_error: OnError::Continue,
                        result: PluginResult::Crashed {
                            trap_string: format!("spawn_blocking join error: {join_err}"),
                            stderr: String::new(),
                            elapsed_ms: 0,
                            fuel_consumed: 0,
                        },
                        block_if_marker_fired: false,
                        block_if_marker_fields: None,
                    });
                }
            },
        }
    }
    outcomes
}

/// Execute a single async-group plugin as an independent tokio task.
///
/// Returns a `JoinHandle<PluginOutcome>` so the caller can collect results
/// via a channel and `tokio::select!` drain timer (BC-1.14.001 PC4 + EC-012).
///
/// # Async-group spawn pattern (BC-1.14.001 v1.9 PC4 + Invariant 3)
///
/// - Each async-group plugin MUST be spawned via `tokio::spawn` (independent task).
/// - Results MUST be collected via a channel (not all-or-nothing `execute_tiers`).
/// - `group_by_priority` MUST NOT be called on async-group plugins.
/// - The caller uses `tokio::select!` over the channel and a drain timer.
///
/// # BC traces
/// - BC-1.14.001 PC4 — per-plugin tokio::spawn spawn pattern
/// - BC-1.14.001 Invariant 3 — async group excluded from tier ordering
/// - EC-012 — partial completions: completed events MUST emit; in-flight MAY be lost
/// - BC-1.13.001 PC3/PC4/PC5 — resolver step mirrors execute_tier behavior
#[allow(clippy::too_many_arguments)]
pub fn spawn_async_plugin(
    engine: wasmtime::Engine,
    cache: Arc<crate::plugin_loader::PluginCache>,
    registry_defaults: crate::registry::RegistryDefaults,
    entry: RegistryEntry,
    payload_value: serde_json::Value,
    base_host_ctx: HostContext,
    internal_log: Arc<InternalLog>,
    resolver_registry: Arc<ResolverRegistry>,
) -> tokio::task::JoinHandle<PluginOutcome> {
    tokio::spawn(async move {
        let limits = InvokeLimits {
            timeout_ms: entry.timeout_ms(&registry_defaults),
            fuel_cap: entry.fuel_cap(&registry_defaults),
        };
        let on_error = entry.on_error(&registry_defaults);

        // Build the merged plugin_config from static config + resolver outputs.
        // AC-002: zero-overhead short-circuit when needs_context is empty.
        let trace_id = payload_value
            .get("dispatcher_trace_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let plugin_config = build_plugin_config(
            &entry,
            &payload_value,
            &base_host_ctx,
            &resolver_registry,
            &internal_log,
            &trace_id,
        );

        // MEDIUM-5: extract artifact_path from tool_input.file_path before payload_value
        // is moved into per_plugin_value (BC-1.18.001 PC4 — marker must record artifact path).
        //
        // LOW-3 (S-25.01): BC-1.18.001 PC4 requires artifact_path to be an absolute path.
        // The Claude Code harness always emits absolute file_path values in tool event
        // payloads (enforced by the harness itself — the Edit/Write tools require an
        // absolute path per CLAUDE.md and the harness rejects relative paths at entry).
        // No normalization is needed; we store the value verbatim and trust the harness
        // invariant. If the harness ever changes this behavior, the marker will contain a
        // relative path (degraded but non-failing — best-effort per BC-1.18.001 PC4).
        let artifact_path_for_marker = payload_value
            .get("tool_input")
            .and_then(|ti| ti.get("file_path"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Splice this entry's per-plugin config onto the base envelope.
        let mut per_plugin_value = payload_value;
        if let Some(map) = per_plugin_value.as_object_mut() {
            map.insert("plugin_config".to_string(), plugin_config);
        }
        let payload = match serde_json::to_vec(&per_plugin_value) {
            Ok(v) => v,
            Err(e) => {
                let result = PluginResult::Crashed {
                    trap_string: format!("payload serialize: {e}"),
                    stderr: String::new(),
                    elapsed_ms: 0,
                    fuel_consumed: 0,
                };
                emit_lifecycle(&internal_log, &base_host_ctx, &entry, &result);
                return PluginOutcome {
                    plugin_name: entry.name.clone(),
                    plugin_version: base_host_ctx.plugin_version.clone(),
                    on_error,
                    result,
                    block_if_marker_fired: false,
                    block_if_marker_fields: None,
                };
            }
        };

        let module = match cache.get_or_compile(&entry.plugin) {
            Ok(m) => m,
            Err(e) => {
                let result = PluginResult::Crashed {
                    trap_string: format!("plugin load failed: {e}"),
                    stderr: String::new(),
                    elapsed_ms: 0,
                    fuel_consumed: 0,
                };
                emit_lifecycle(&internal_log, &base_host_ctx, &entry, &result);
                return PluginOutcome {
                    plugin_name: entry.name.clone(),
                    plugin_version: base_host_ctx.plugin_version.clone(),
                    on_error,
                    result,
                    block_if_marker_fired: false,
                    block_if_marker_fields: None,
                };
            }
        };

        emit_invoked(&internal_log, &base_host_ctx, &entry);
        let base_ctx_for_event = base_host_ctx.clone();

        let mut host_ctx = base_host_ctx;
        host_ctx.plugin_name = entry.name.clone();
        host_ctx.capabilities = entry.capabilities.clone().unwrap_or_default();

        let (result, output_too_large) = tokio::task::spawn_blocking(move || {
            let started = std::time::Instant::now();
            invoke_plugin(&engine, &module, host_ctx, &payload, limits).unwrap_or_else(|e| {
                (
                    PluginResult::Crashed {
                        trap_string: format!("invoke setup error: {e}"),
                        stderr: String::new(),
                        elapsed_ms: started.elapsed().as_millis() as u64,
                        fuel_consumed: 0,
                    },
                    false,
                )
            })
        })
        .await
        .unwrap_or_else(|join_err| {
            (
                PluginResult::Crashed {
                    trap_string: format!("spawn_blocking join error: {join_err}"),
                    stderr: String::new(),
                    elapsed_ms: 0,
                    fuel_consumed: 0,
                },
                false,
            )
        });

        emit_lifecycle(&internal_log, &base_ctx_for_event, &entry, &result);

        // S-25.01: classify + emit plugin.indeterminate + marker write (mirrors execute_tier).
        let failure_policy = entry.failure_policy;
        let outcome = classify_outcome(result.clone(), failure_policy, output_too_large);

        // BLOCKER-1: BC-1.18.003 PC1 + INV2 — PASS from the named plugin MUST clear
        // the marker, scoped to only the plugin named in the marker (INV2).
        // MEDIUM-1 fix (S-25.01): BC-1.18.003 PC1 requires the clear happen ONLY when the
        // named plugin is dispatched in a PostToolUse hook and produces Pass. A PreToolUse
        // PASS from the named plugin MUST NOT clear the marker.
        if let DispatchOutcome::Pass = outcome {
            let marker_path = base_ctx_for_event
                .cwd
                .join(".factory")
                .join("unvalidated-mutation.marker");
            // Scoped clear: only this plugin's PostToolUse PASS clears its own marker.
            // M-1 fix (S-25.01): pass artifact_path_for_marker so delete_marker_if_pass
            // enforces BC-1.18.003 INV2 artifact-scoped clear internally.
            // Log (but do not propagate) errors — a clear failure must not fail the dispatch.
            if let Ok(Some(marker_plugin)) = read_marker_plugin_name(&marker_path)
                && marker_plugin == entry.name
                && entry.event == "PostToolUse"
            {
                // ADR-048 v1.1: read all marker fields BEFORE delete for marker.cleared event.
                let all_fields = read_all_marker_fields(&marker_path).ok().flatten();
                match delete_marker_if_pass(&marker_path, &artifact_path_for_marker) {
                    Ok(true) => {
                        // Marker was actually removed — emit marker.cleared(REVALIDATED).
                        if let Some(ref fields) = all_fields {
                            emit_marker_cleared(
                                &base_ctx_for_event,
                                fields,
                                "REVALIDATED",
                                "validator",
                                None,
                            );
                        }
                    }
                    Ok(false) => {}
                    Err(e) => {
                        tracing::warn!(
                            plugin = %entry.name,
                            marker_path = %marker_path.display(),
                            error = %e,
                            "best-effort marker clear failed on PASS; dispatch continues"
                        );
                    }
                }
            }
        }

        if let DispatchOutcome::Indeterminate { ref cause } = outcome {
            // HIGH-1: pass real artifact_path so event and marker record the same path.
            emit_indeterminate(
                &base_ctx_for_event,
                &entry,
                cause,
                &artifact_path_for_marker,
            );
            // BLOCKER-2: BC-1.18.001 INV4 — marker write is PostToolUse-only.
            // PreToolUse INDETERMINATE → advisory event only; no marker written.
            if should_write_marker(&outcome, failure_policy) && entry.event == "PostToolUse" {
                let marker_path = base_ctx_for_event
                    .cwd
                    .join(".factory")
                    .join("unvalidated-mutation.marker");
                let marker_now = chrono::Utc::now();
                let fields = MarkerFields {
                    timestamp: marker_now.to_rfc3339(),
                    plugin_name: entry.name.clone(),
                    // MEDIUM-5: thread artifact_path from tool_input.file_path (AC-007).
                    artifact_path: artifact_path_for_marker,
                    cause: cause_to_str(cause).to_string(),
                    trace_id: base_ctx_for_event.dispatcher_trace_id.clone(),
                    // ADR-048 §Decision 2: 24-hour deadman TTL.
                    expires_at: (marker_now
                        + chrono::Duration::seconds(
                            UNVALIDATED_MUTATION_MARKER_TTL_SECONDS as i64,
                        ))
                    .to_rfc3339(),
                };
                // F-P3-002 (ADR-048 §D4 v1.3): read the pre-existing marker BEFORE
                // the overwrite so its fields are captured prior to being clobbered
                // by the rename. F-P9-001 (ADR-048 §D4 v1.5, symmetric to the v1.4
                // marker.written fix): emit marker.cleared(SUPERSEDED) for it ONLY
                // immediately after a confirmed successful write — never before the
                // write is attempted, never on Err. Emitting SUPERSEDED
                // unconditionally (before the write) falsely records the old marker
                // as overwritten even when write_indeterminate_marker returns Err
                // and the old marker is still on disk untouched — otherwise
                // reconcile_raw_delete would later mis-attribute the superseded
                // pair's clearance to a human OPERATOR_OVERRIDE that never happened
                // (BC-1.18.001 INV3 last-writer-wins requires the audit trail to
                // reflect what actually happened, not what was attempted).
                let existing_marker = read_all_marker_fields(&marker_path).ok().flatten();
                // HIGH-2: log marker-write failures instead of silently swallowing them.
                // Best-effort: write failure does NOT fail the dispatch result.
                // The plugin.indeterminate event was already emitted above.
                //
                // TD-VSDD-060 (F-P12-001 pre-req): the tied emission decision
                // (SUPERSEDED-then-written on Ok, nothing on Err) is delegated
                // to emit_write_tied_audit_events — the single source of truth
                // for this discipline, shared with execute_tier above.
                let write_result = write_indeterminate_marker(&fields, &marker_path);
                emit_write_tied_audit_events(
                    &base_ctx_for_event,
                    write_result,
                    &marker_path,
                    existing_marker.as_ref(),
                    &fields,
                );
            }
        }

        PluginOutcome {
            plugin_name: entry.name.clone(),
            plugin_version: base_ctx_for_event.plugin_version.clone(),
            on_error,
            result,
            // Async plugins are not processed through execute_tiers' block_if_marker loop;
            // block_if_marker semantics are not applied to async-group outcomes
            // (BC-1.14.001 Invariant 3 — async group excluded from tier ordering).
            block_if_marker_fired: false,
            block_if_marker_fields: None,
        }
    })
}

/// Internal helper: either an already-resolved outcome (load failure
/// short-circuit) or a pending tokio JoinHandle. Keeps the per-plugin
/// fan-out loop uniform.
enum JoinWrap {
    Ready(PluginOutcome),
    Pending(tokio::task::JoinHandle<PluginOutcome>),
}

/// Build the merged `plugin_config` for one hook entry.
///
/// AC-002: if `entry.needs_context` is empty, returns the static config
/// unchanged with zero resolver invocations.
///
/// AC-003: if `entry.needs_context` is non-empty, invokes each resolver
/// via `resolver_registry` and merges outputs onto the static config
/// using `merge_resolver_outputs`. Emits `resolver.not_found` and
/// `resolver.error` via `internal_log` for observability (BC-1.13.001
/// PC6 / SOUL #4 — no silent failures).
///
/// The returned `Value` is always a JSON Object ready to be inserted at
/// the `"plugin_config"` key of the per-plugin envelope.
// CONVENTION (P-005 / S-7.01 Sibling-Coverage):
//
// When adding a new resolver-tier event type to the dispatcher
// (whether emitted from executor.rs::build_plugin_config OR from
// main.rs startup OR from any other dispatcher code path):
//
// 1. Add a field table to HOST_ABI.md listing ALL emitted fields,
//    INCLUDING the provenance triplet (trace_id, session_id, plugin_name)
//    when applicable.
// 2. Update the BC that owns the event with the corresponding PC field
//    list (BC-1.13.001 / BC-4.12.004 / etc).
// 3. Add positive-coverage assertions in the integration test for EVERY
//    provenance field (POL-11).
// 4. Audit ALL sibling resolver-tier events for the same gaps. The current
//    enumerated list (NOT exhaustive — verify by `grep -rn 'InternalEvent::now("resolver\.'`):
//      - resolver.not_found      (executor.rs)
//      - resolver.error          (executor.rs)
//      - resolver.merge_collision (executor.rs)
//      - resolver.registry_loaded (main.rs)
//      - resolver.load_warning   (main.rs)
//      - resolver.load_error     (main.rs)
//
// This pattern was hard-learned across 4+ adversarial passes; codified
// to prevent recurrence. The S-7.01 sibling-blast-radius rule applies
// across SOURCE FILES not just within a single function.
fn build_plugin_config(
    entry: &RegistryEntry,
    payload_value: &serde_json::Value,
    base_host_ctx: &HostContext,
    resolver_registry: &ResolverRegistry,
    internal_log: &InternalLog,
    trace_id: &str,
) -> serde_json::Value {
    // AC-002: zero-overhead short-circuit (BC-1.13.001 PC3).
    if entry.needs_context.is_empty() {
        return entry.config_as_json();
    }

    // Hoist single config_as_json() call — avoids three separate allocations below.
    // Placed inside the non-empty branch so the zero-overhead short-circuit at the
    // top of this function (needs_context.is_empty() → return early) is preserved.
    let static_json = entry.config_as_json();

    // Build the ResolverInput from the current dispatch context.
    let event_type = payload_value
        .get("event_name")
        .or_else(|| payload_value.get("hook_event_name"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    // agent_type is reserved for future Claude Code envelope evolution per BC-4.12.002.
    // Standard PreToolUse/PostToolUse envelopes do not carry this field today; the
    // extraction defaults to None for resolver inputs. Forward-compat with potential
    // `subagent_type` or `agent_type` envelope additions.
    let agent_type = payload_value
        .get("agent_type")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let project_dir = base_host_ctx.cwd.to_str().unwrap_or("").to_string();

    let resolver_input = ResolverInput {
        event_type: event_type.clone(),
        hook_event_name: entry.name.clone(),
        agent_type,
        project_dir,
        plugin_config: static_json.clone(),
    };

    // Coerce the static config into a Map for merge_resolver_outputs (F-006).
    //
    // After Registry::parse_str + config_as_json(), plugin_config is guaranteed to be
    // Value::Object by TOML structure semantics: TOML tables always deserialize to JSON
    // objects, and Registry::parse_str rejects non-object plugin_config at load time.
    // The non-Object arm is therefore unreachable in production. The debug_assert
    // documents this invariant; any violation is a programming error, not a runtime fault.
    debug_assert!(
        matches!(static_json, serde_json::Value::Object(_)),
        "plugin_config must be a JSON object after Registry::parse_str — TOML table \
         semantics guarantee this; a non-Object value indicates a bypass of parse_str"
    );
    let static_map = match static_json {
        serde_json::Value::Object(m) => m,
        _ => serde_json::Map::new(),
    };

    let hook_name = entry.name.clone();
    let hook_name_nf = hook_name.clone();
    let hook_name_err = hook_name.clone();
    let trace_id_nf = trace_id.to_string();
    let trace_id_err = trace_id.to_string();
    // F-P4-001: carry session_id into resolver event closures for parity with
    // emit_invoked (line 617) and emit_lifecycle (line 704) which both include
    // with_session_id(&base_ctx.session_id).
    let session_id_nf = base_host_ctx.session_id.clone();
    let session_id_err = base_host_ctx.session_id.clone();
    // event_type is the Claude Code envelope event (e.g. "PreToolUse") emitted as
    // the event_type field in resolver.error events (HOST_ABI.md line 1097).
    let event_type_for_log = event_type.clone();

    // AC-005: emit resolver.not_found when a named resolver is absent.
    // Clone InternalLog (PathBuf wrapper) so the closure captures by value — no unsafe needed.
    //
    // Note: resolver.not_found event field table now documented in HOST_ABI
    // (F-P7-002 burst). Wire format (per implementation): { resolver_name, trace_id,
    // session_id, plugin_name }. F-P7-002 closes the prior deferral.
    let emit_not_found = {
        let log = internal_log.clone();
        move |missing_name: &str| {
            let ev = InternalEvent::now("resolver.not_found")
                .with_trace_id(&trace_id_nf)
                .with_session_id(&session_id_nf)
                .with_plugin_name(&hook_name_nf)
                .with_field(
                    "resolver_name",
                    serde_json::Value::String(missing_name.to_string()),
                );
            log.write(&ev);
        }
    };

    // AC-007 / SOUL #4: emit resolver.error when a resolver returns Err.
    // F-P4-001A / F-P5-001: error_kind uses snake_case serde tag (HOST_ABI line 1095).
    // F-P5-002: error_detail (singular) is the Display string (HOST_ABI line 1096).
    //           event_type carries the Claude Code envelope event type (HOST_ABI line 1097).
    //           This is distinct from ResolverInput.hook_event_name (registry entry name).
    let emit_resolver_error = {
        let log = internal_log.clone();
        move |err_name: &str, err: &crate::resolver::ResolverError| {
            let err_json = serde_json::to_value(err)
                .unwrap_or_else(|_| serde_json::json!({"kind": "unknown"}));
            let error_kind = err_json
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let ev = InternalEvent::now("resolver.error")
                .with_trace_id(&trace_id_err)
                .with_session_id(&session_id_err)
                .with_plugin_name(&hook_name_err)
                .with_field(
                    "resolver_name",
                    serde_json::Value::String(err_name.to_string()),
                )
                .with_field("error_kind", serde_json::Value::String(error_kind))
                .with_field("error_detail", serde_json::Value::String(format!("{err}")))
                .with_field(
                    "event_type",
                    serde_json::Value::String(event_type_for_log.clone()),
                );
            log.write(&ev);
        }
    };

    // resolve_context_for_entry returns Vec<ResolvedContext> in declaration order
    // (BC-1.13.001 PC7 / F-P5-003 Option A / F-P2-002 / F-P3-001).
    // Each ResolvedContext carries context_key (merge key), resolver_name
    // (registry name for telemetry), and output.
    let resolver_outputs = resolver_registry.resolve_context_for_entry(
        &entry.needs_context,
        &resolver_input,
        emit_not_found,
        emit_resolver_error,
    );

    // AC-007: merge_resolver_outputs is pure (BC-4.12.005 INV1, architect Path B).
    // Collisions are returned as Vec<CollisionInfo>; caller emits telemetry for each.
    // F-P5-003 / F-P2-002 / F-P3-001: ResolvedContext carries both context_key (merge key)
    // and resolver_name (registry name) so CollisionInfo.resolver_name is the registry NAME,
    // not the context_key.
    let (merged_map, collisions) = merge_resolver_outputs(static_map, &resolver_outputs);

    // F-P4-001B / F-P2-002 / F-P3-001: emit resolver_name (registry NAME, not context_key)
    // in each merge_collision event for per-resolver traceability (BC-4.12.004 wire format).
    for collision in collisions {
        let ev = InternalEvent::now("resolver.merge_collision")
            .with_trace_id(trace_id)
            .with_session_id(&base_host_ctx.session_id)
            .with_plugin_name(&hook_name)
            .with_field("key", serde_json::Value::String(collision.key))
            .with_field(
                "resolver_name",
                serde_json::Value::String(collision.resolver_name),
            )
            .with_field("static_value", collision.old_value)
            .with_field("resolver_value", collision.new_value);
        internal_log.write(&ev);
    }

    serde_json::Value::Object(merged_map)
}

fn plugin_requests_block(result: &PluginResult) -> bool {
    let stdout = match result {
        PluginResult::Ok { stdout, .. } => stdout.as_str(),
        _ => return false,
    };
    // Plugins serialize `HookResult::Block { reason }` as
    // `{"outcome":"block","reason":"..."}`. The simplest durable
    // detector is a substring check against the tagged serde output —
    // we're not committing to parsing the full JSON here because the
    // contract is stable (HOST_ABI.md) and a fuller parse can be
    // layered on when the internal log needs the reason.
    stdout.contains(r#""outcome":"block""#)
}

/// Fail-closed semantics for sync-group gate hooks (ADR-019 §Decision 2).
///
/// Returns `true` when a sync-group plugin Crashed or timed out AND its
/// registry entry declared `on_error = block`. In this case the dispatcher
/// must exit 2 even though the crashed plugin never emitted stdout.
///
/// **Async hooks MUST NOT call this path.** `execute_tiers` is called only
/// for sync-group plugins; async hooks go through `spawn_async_plugin` and
/// are excluded from gate decisions by the structural partition (ADR-019
/// async semantics — async verdicts are advisory-only).
///
/// # BC traces
/// - ADR-019 §Decision 2 — fail-closed semantics
/// - BC-1.14.001 Error Paths — Crashed+on_error=Block exits 2
/// - BC-7.06.001 Invariant 1 — sync gate hooks must not silently fail open
fn plugin_fail_closed(result: &PluginResult, on_error: OnError) -> bool {
    if on_error != OnError::Block {
        return false;
    }
    matches!(
        result,
        PluginResult::Crashed { .. } | PluginResult::Timeout { .. }
    )
}

/// Conditional crash-path gate for `on_error = "block_if_marker"` (ADR-048 §Decision 1).
///
/// Returns `true` (block) iff:
/// 1. `on_error == BlockIfMarker`, AND
/// 2. the plugin crashed or timed out, AND
/// 3. a non-expired `.factory/unvalidated-mutation.marker` exists under `cwd`.
///
/// All other combinations return `false` (allow). I/O errors reading the marker are
/// treated as allow (fail-open on infra fault — CWE-636 balance).
fn plugin_block_if_marker(
    result: &PluginResult,
    on_error: OnError,
    cwd: &std::path::Path,
    now: chrono::DateTime<chrono::Utc>,
) -> bool {
    if on_error != OnError::BlockIfMarker {
        return false;
    }
    if !matches!(
        result,
        PluginResult::Crashed { .. } | PluginResult::Timeout { .. }
    ) {
        return false;
    }
    block_if_marker_check(cwd, now)
}

fn emit_invoked(log: &InternalLog, base_ctx: &HostContext, entry: &RegistryEntry) {
    let ev = InternalEvent::now(PLUGIN_INVOKED)
        .with_trace_id(&base_ctx.dispatcher_trace_id)
        .with_session_id(&base_ctx.session_id)
        .with_plugin_name(&entry.name)
        .with_plugin_version(&base_ctx.plugin_version)
        .with_field("event", serde_json::Value::String(entry.event.clone()));
    log.write(&ev);
}

fn emit_lifecycle(
    log: &InternalLog,
    base_ctx: &HostContext,
    entry: &RegistryEntry,
    result: &PluginResult,
) {
    let (event_type, elapsed, fuel, mut extra_fields) = match result {
        PluginResult::Ok {
            exit_code,
            elapsed_ms,
            fuel_consumed,
            stderr,
            ..
        } => (
            PLUGIN_COMPLETED,
            *elapsed_ms,
            *fuel_consumed,
            vec![
                ("exit_code".to_string(), serde_json::Value::from(*exit_code)),
                (
                    "stderr".to_string(),
                    serde_json::Value::String(stderr.clone()),
                ),
            ],
        ),
        PluginResult::Timeout {
            cause,
            stderr,
            elapsed_ms,
            fuel_consumed,
            fuel_cap,
        } => {
            let cause_str = match cause {
                TimeoutCause::Epoch => "epoch",
                TimeoutCause::Fuel => "fuel",
            };
            (
                PLUGIN_TIMEOUT,
                *elapsed_ms,
                *fuel_consumed,
                vec![
                    (
                        "cause".to_string(),
                        serde_json::Value::String(cause_str.to_string()),
                    ),
                    (
                        "stderr".to_string(),
                        serde_json::Value::String(stderr.clone()),
                    ),
                    ("fuel_cap".to_string(), serde_json::Value::from(*fuel_cap)),
                ],
            )
        }
        PluginResult::Crashed {
            trap_string,
            stderr,
            elapsed_ms,
            fuel_consumed,
        } => (
            PLUGIN_CRASHED,
            *elapsed_ms,
            *fuel_consumed,
            vec![
                (
                    "trap".to_string(),
                    serde_json::Value::String(trap_string.clone()),
                ),
                (
                    "stderr".to_string(),
                    serde_json::Value::String(stderr.clone()),
                ),
            ],
        ),
    };
    // Drop empty stderr from the lifecycle event payload — it's the
    // common case (well-behaved plugins write nothing to stderr) and a
    // dangling "stderr": "" field is just visual noise in the log.
    extra_fields.retain(|(k, v)| k != "stderr" || !v.as_str().map(str::is_empty).unwrap_or(false));

    let mut ev = InternalEvent::now(event_type)
        .with_trace_id(&base_ctx.dispatcher_trace_id)
        .with_session_id(&base_ctx.session_id)
        .with_plugin_name(&entry.name)
        .with_plugin_version(&base_ctx.plugin_version)
        .with_field("elapsed_ms", serde_json::Value::from(elapsed))
        .with_field("fuel_consumed", serde_json::Value::from(fuel));
    for (k, v) in extra_fields {
        ev = ev.with_field(&k, v);
    }
    log.write(&ev);
}

/// Convert `IndeterminateCause` to the canonical BC-3.08.001 wire string.
fn cause_to_str(cause: &IndeterminateCause) -> &'static str {
    match cause {
        IndeterminateCause::Fuel => "fuel",
        IndeterminateCause::Epoch => "epoch",
        IndeterminateCause::OutputTooLarge => "output-too-large",
    }
}

/// Emit a `plugin.indeterminate` event to the internal log (BC-3.08.001 Event 8).
///
/// Mandatory 8 fields: type, trace_id, session_id, plugin_name, artifact_path,
/// cause, failure_policy, timestamp (provided by `InternalEvent::now`).
///
/// `artifact_path` MUST be the envelope `file_path` from `tool_input` when one
/// is present; callers pass an empty string only when there is genuinely no
/// artifact context (e.g. non-file-mutation tool events). Never hardcoded empty.
///
/// Called for EVERY INDETERMINATE outcome — both fail-closed and fail-open paths.
///
/// Routes through `base_ctx.emit_internal` (ADR-048 §D4 v1.3 F-P3-001) — the same
/// dual-sink primitive (durable `InternalLog` write + `ctx.events` queue) every
/// other dispatcher-native BC-3.08.001 event uses — rather than a raw
/// `InternalLog::write` call. No new parameter: `base_ctx: &HostContext` was
/// already threaded to every call site.
fn emit_indeterminate(
    base_ctx: &HostContext,
    entry: &RegistryEntry,
    cause: &IndeterminateCause,
    artifact_path: &str,
) {
    let cause_str = cause_to_str(cause);
    let policy_str = match entry.failure_policy {
        FailurePolicy::FailClosed => "fail-closed",
        FailurePolicy::FailOpen => "fail-open",
    };
    let ev = InternalEvent::now(PLUGIN_INDETERMINATE);
    // BC-3.08.001 wire format: mandatory `timestamp` field distinct from `ts` (DI-017).
    // `with_field("timestamp", ...)` adds the BC-required `timestamp` alias for `ts`.
    let ts = ev.ts.clone();
    let ev = ev
        .with_trace_id(&base_ctx.dispatcher_trace_id)
        .with_session_id(&base_ctx.session_id)
        .with_plugin_name(&entry.name)
        .with_field("timestamp", ts.as_str())
        .with_field(
            "artifact_path",
            serde_json::Value::String(artifact_path.to_string()),
        )
        .with_field("cause", serde_json::Value::String(cause_str.to_string()))
        .with_field(
            "failure_policy",
            serde_json::Value::String(policy_str.to_string()),
        );
    base_ctx.emit_internal(ev);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indeterminate_marker::{
        MarkerFields, delete_marker_if_pass, should_write_marker, write_indeterminate_marker,
    };
    use crate::invoke::DEFAULT_FUEL_CAP;
    use crate::registry::FailurePolicy;

    // ── S-25.01 Red Gate tests — BC-1.18.001/002/003/004 ─────────────────────
    // Each test calls a todo!() production function, compiles, and MUST FAIL
    // at runtime until the implementer fills in the production logic (T-1..T-3).
    //
    // BC-5.38.005 self-check: "If I include this real implementation, will the
    // test pass trivially without implementer work?" — YES for all tests below.
    // Therefore all production bodies remain todo!() per BC-5.38.001.

    // ── S-25.01 Red Gate stub 1 ───────────────────────────────────────────────
    #[test]
    fn test_BC_1_18_001_fuel_exhaustion_yields_indeterminate_for_fail_closed_plugin() {
        // AC-001 / BC-1.18.001 postcondition 1 (fuel cause):
        // Timeout { cause: Fuel } + fail-closed → Indeterminate { cause: Fuel }.
        // Fuel detection MUST use Trap::OutOfFuel downcast, NOT get_fuel() (ADR-047 §D1).
        let result = PluginResult::Timeout {
            cause: TimeoutCause::Fuel,
            stderr: String::new(),
            elapsed_ms: 5_000,
            fuel_consumed: DEFAULT_FUEL_CAP,
            fuel_cap: DEFAULT_FUEL_CAP,
        };
        let outcome = classify_outcome(result, FailurePolicy::FailClosed, false);
        assert_eq!(
            outcome,
            DispatchOutcome::Indeterminate {
                cause: IndeterminateCause::Fuel
            },
            "AC-001: Fuel timeout + fail-closed MUST yield Indeterminate(Fuel), not Pass or Fail"
        );
    }

    // ── S-25.01 Red Gate stub 2 ───────────────────────────────────────────────
    #[test]
    fn test_BC_1_18_001_epoch_timeout_yields_indeterminate_for_fail_closed_plugin() {
        // AC-002 / BC-1.18.001 postcondition 1 (epoch timeout cause):
        // Timeout { cause: Epoch } + fail-closed → Indeterminate { cause: Epoch }.
        let result = PluginResult::Timeout {
            cause: TimeoutCause::Epoch,
            stderr: String::new(),
            elapsed_ms: 5_000,
            fuel_consumed: 0,
            fuel_cap: DEFAULT_FUEL_CAP,
        };
        let outcome = classify_outcome(result, FailurePolicy::FailClosed, false);
        assert_eq!(
            outcome,
            DispatchOutcome::Indeterminate {
                cause: IndeterminateCause::Epoch
            },
            "AC-002: Epoch timeout + fail-closed MUST yield Indeterminate(Epoch)"
        );
    }

    // ── S-25.01 Red Gate stub 3 ───────────────────────────────────────────────
    #[test]
    fn test_BC_1_18_001_output_too_large_then_ok_yields_indeterminate_for_fail_closed_plugin() {
        // AC-003 / BC-1.18.001 postcondition 1 + invariant 1:
        // output_too_large=true + exit_code=0 + fail-closed → Indeterminate(OutputTooLarge).
        // Per-invocation reset: host_output_too_large_seen MUST be reset to false BEFORE each
        // func.call() invocation — not only at Store creation (BC-1.18.001 INV1).
        // The flag is captured AFTER call() and passed to classify_outcome (AC-018).
        let result = PluginResult::Ok {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            elapsed_ms: 10,
            fuel_consumed: 100,
        };
        // output_too_large=true: a host function returned OutputTooLarge(-3) during this invocation
        let outcome = classify_outcome(result, FailurePolicy::FailClosed, true);
        assert_eq!(
            outcome,
            DispatchOutcome::Indeterminate {
                cause: IndeterminateCause::OutputTooLarge
            },
            "AC-003: exit_code=0 + output_too_large=true MUST yield Indeterminate(OutputTooLarge)"
        );
    }

    // ── S-25.01 Red Gate stub 4 ───────────────────────────────────────────────
    #[test]
    fn test_BC_1_18_001_indeterminate_is_distinct_from_pass_and_fail() {
        // AC-004 / BC-1.18.001 postcondition 2 + postcondition 5:
        // DispatchOutcome is a strict trichotomy: Pass, Fail, Indeterminate.
        // PASS: exit_code=0 AND output_too_large=false.
        // FAIL: non-zero exit_code.
        // INDETERMINATE: the third, distinct gap (fuel/epoch/OTL).

        // Pass path
        let ok_pass = PluginResult::Ok {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            elapsed_ms: 1,
            fuel_consumed: 10,
        };
        let pass = classify_outcome(ok_pass, FailurePolicy::FailClosed, false);
        assert_eq!(
            pass,
            DispatchOutcome::Pass,
            "exit_code=0 + no OTL MUST be Pass"
        );

        // Fail path
        let ok_fail = PluginResult::Ok {
            exit_code: 1,
            stdout: String::new(),
            stderr: String::new(),
            elapsed_ms: 1,
            fuel_consumed: 10,
        };
        let fail = classify_outcome(ok_fail, FailurePolicy::FailClosed, false);
        assert_eq!(
            fail,
            DispatchOutcome::Fail { exit_code: 1 },
            "exit_code=1 MUST be Fail{{exit_code: 1}}"
        );

        // Indeterminate is distinct from both Pass and Fail
        let indet = DispatchOutcome::Indeterminate {
            cause: IndeterminateCause::Fuel,
        };
        assert_ne!(
            indet,
            DispatchOutcome::Pass,
            "Indeterminate MUST NOT equal Pass"
        );
        assert_ne!(
            indet,
            DispatchOutcome::Fail { exit_code: 1 },
            "Indeterminate MUST NOT equal Fail"
        );
    }

    // ── S-25.01 Red Gate stub 5 ───────────────────────────────────────────────
    #[test]
    fn test_BC_1_18_001_indeterminate_writes_marker_to_factory_path() {
        // AC-005 / BC-1.18.001 postcondition 4 + invariant 3 + invariant 4:
        // Atomic marker write via write-to-temp + rename (O_CREAT|O_WRONLY|O_TRUNC then rename).
        // PostToolUse-only: marker write is only valid for PostToolUse events (INV4).
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");
        let fields = MarkerFields {
            timestamp: "2026-08-30T00:00:00Z".to_string(),
            plugin_name: "validate-factory-path-staging".to_string(),
            artifact_path: String::new(), // empty string; MUST NOT be omitted (BC-1.18.001 PC4)
            cause: "fuel".to_string(),
            trace_id: "test-trace-001".to_string(),
            expires_at: "2099-01-01T00:00:00Z".to_string(),
        };
        write_indeterminate_marker(&fields, &marker_path)
            .expect("write_indeterminate_marker MUST succeed for a writable path");
        assert!(
            marker_path.exists(),
            "AC-005: marker file MUST exist after write_indeterminate_marker returns Ok(())"
        );
        // Verify no stale .tmp file remains after atomic rename
        let tmp_path = dir
            .path()
            .join(format!("{}.tmp", "unvalidated-mutation.marker"));
        assert!(
            !tmp_path.exists(),
            "AC-005: temp file MUST be renamed to final path; no .tmp file should remain"
        );
    }

    // ── S-25.01 Red Gate stub 6 ───────────────────────────────────────────────
    #[test]
    fn test_BC_1_18_001_marker_contains_required_fields() {
        // AC-005 / BC-1.18.001 postcondition 4:
        // Marker MUST contain all five required TOML fields:
        //   timestamp (RFC 3339), plugin_name, artifact_path (empty ok, never omit),
        //   cause ("fuel"|"epoch"|"output-too-large"), trace_id.
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");
        let fields = MarkerFields {
            timestamp: "2026-08-30T12:00:00Z".to_string(),
            plugin_name: "validate-factory-path-staging".to_string(),
            artifact_path: "/path/to/.factory/STATE.md".to_string(),
            cause: "epoch".to_string(),
            trace_id: "deadbeef-0001-0001-0001-000000000001".to_string(),
            expires_at: "2099-01-01T00:00:00Z".to_string(),
        };
        write_indeterminate_marker(&fields, &marker_path).expect("write must succeed");
        let contents =
            std::fs::read_to_string(&marker_path).expect("must be able to read marker back");

        // All five required field names must appear in the TOML
        assert!(
            contents.contains("timestamp"),
            "AC-005: marker MUST contain 'timestamp' TOML key"
        );
        assert!(
            contents.contains("plugin_name"),
            "AC-005: marker MUST contain 'plugin_name' TOML key"
        );
        assert!(
            contents.contains("artifact_path"),
            "AC-005: marker MUST contain 'artifact_path' TOML key (even when empty)"
        );
        assert!(
            contents.contains("cause"),
            "AC-005: marker MUST contain 'cause' TOML key"
        );
        assert!(
            contents.contains("trace_id"),
            "AC-005: marker MUST contain 'trace_id' TOML key"
        );
        // Verify specific values were persisted
        assert!(
            contents.contains("validate-factory-path-staging"),
            "AC-005: marker plugin_name MUST reflect the plugin name"
        );
        assert!(
            contents.contains("epoch"),
            "AC-005: marker cause MUST reflect the cause value"
        );
        assert!(
            contents.contains("deadbeef-0001-0001-0001-000000000001"),
            "AC-005: marker trace_id MUST reflect the trace_id value"
        );
    }

    // ── S-25.01 Red Gate stub 7 ───────────────────────────────────────────────
    #[test]
    fn test_BC_1_18_003_successful_revalidation_deletes_marker() {
        // AC-012 / BC-1.18.003 postcondition 1 + invariant 2:
        // delete_marker_if_pass deletes the marker file.
        // Scoping: ONLY the named plugin's PASS clears the marker; caller must enforce this.
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");

        // Pre-condition: write a marker to simulate the INDETERMINATE state
        std::fs::write(
            &marker_path,
            "timestamp = \"2026-08-30T00:00:00Z\"\nplugin_name = \"test-plugin\"\n\
             artifact_path = \"\"\ncause = \"fuel\"\ntrace_id = \"trace-test\"\n",
        )
        .expect("test setup: write marker");
        assert!(
            marker_path.exists(),
            "pre-condition: marker must exist before delete"
        );

        // marker has artifact_path = "" (empty) → M-1 predicate: empty artifact_path →
        // vacuously satisfied → delete regardless of current_artifact_path.
        let deleted = delete_marker_if_pass(&marker_path, "")
            .expect("AC-012: delete_marker_if_pass MUST return Ok(_) when file exists");
        assert!(
            deleted,
            "AC-012: delete_marker_if_pass MUST return Ok(true) when the marker was removed"
        );

        assert!(
            !marker_path.exists(),
            "AC-012: marker MUST be deleted after delete_marker_if_pass succeeds"
        );
    }

    // ── S-25.01 Red Gate stub 8 (DO NOT DELETE per ADR-047 §D7 alias) ─────────
    #[test]
    fn test_BC_1_18_004_fail_open_indeterminate_writes_no_marker() {
        // Canonical backward-compat guard test. Cross-reference: BC-1.18.004 PC5 canonical name is
        // test_BC_1_18_004_fail_open_default_preserves_advisory_behavior. Both names acceptable per
        // BC-1.18.004 PC5 allowance — this is the canonical alias. DO NOT DELETE. ADR-047 §D7.
        //
        // AC-015 / BC-1.18.004 postcondition 1 + postcondition 2 + postcondition 3:
        // fail-open INDETERMINATE → advisory event only; NO marker written; NO gate triggered.
        // should_write_marker(Indeterminate, FailOpen) MUST be false.
        // ~76 existing fail-open plugins MUST be completely unaffected by S-25.01.
        let outcome = DispatchOutcome::Indeterminate {
            cause: IndeterminateCause::Fuel,
        };
        let write = should_write_marker(&outcome, FailurePolicy::FailOpen);
        assert!(
            !write,
            "AC-015 / BC-1.18.004 PC1: should_write_marker(Indeterminate, FailOpen) MUST return \
             false — ~76 existing fail-open plugins MUST NOT have a marker written (DO NOT DELETE)"
        );
    }

    // ── S-25.01 Red Gate stub 9 (DO NOT DELETE per ADR-047 §D7 canonical) ────
    #[test]
    fn test_BC_1_18_004_fail_open_default_preserves_advisory_behavior() {
        // Canonical backward-compat guard test per ADR-047 §Decision 7. DO NOT DELETE.
        // Cross-reference: test_BC_1_18_004_fail_open_indeterminate_writes_no_marker above.
        //
        // AC-015 / BC-1.18.004 postcondition 1–3; S-21.10 canonical:
        // FailurePolicy::default() MUST equal FailOpen (ADR-039 §Decision 1).
        // Validates: all four should_write_marker(_, FailOpen) cases return false.

        // Fuel cause
        let indet_fuel = DispatchOutcome::Indeterminate {
            cause: IndeterminateCause::Fuel,
        };
        assert!(
            !should_write_marker(&indet_fuel, FailurePolicy::FailOpen),
            "AC-015: should_write_marker(Indeterminate(Fuel), FailOpen) MUST be false"
        );

        // Epoch cause
        let indet_epoch = DispatchOutcome::Indeterminate {
            cause: IndeterminateCause::Epoch,
        };
        assert!(
            !should_write_marker(&indet_epoch, FailurePolicy::FailOpen),
            "AC-015: should_write_marker(Indeterminate(Epoch), FailOpen) MUST be false"
        );

        // OTL cause
        let indet_otl = DispatchOutcome::Indeterminate {
            cause: IndeterminateCause::OutputTooLarge,
        };
        assert!(
            !should_write_marker(&indet_otl, FailurePolicy::FailOpen),
            "AC-015: should_write_marker(Indeterminate(OTL), FailOpen) MUST be false"
        );

        // FailurePolicy::default() MUST equal FailOpen (S-21.10 canonical; ADR-039 §Decision 1)
        assert!(
            !should_write_marker(&indet_fuel, FailurePolicy::default()),
            "AC-015 / S-21.10: FailurePolicy::default() MUST equal FailOpen; \
             should_write_marker(Indeterminate, default()) MUST be false (DO NOT DELETE)"
        );

        // Pass and Fail with FailOpen MUST also be false (symmetry check)
        assert!(
            !should_write_marker(&DispatchOutcome::Pass, FailurePolicy::FailOpen),
            "should_write_marker(Pass, FailOpen) MUST be false"
        );
        assert!(
            !should_write_marker(
                &DispatchOutcome::Fail { exit_code: 1 },
                FailurePolicy::FailOpen
            ),
            "should_write_marker(Fail, FailOpen) MUST be false"
        );
    }

    // ── S-25.01 Red Gate stub 10 ──────────────────────────────────────────────
    #[test]
    fn test_backward_compat_pass_fail_on_error_semantics_unchanged() {
        // AC-004 / BC-1.18.001 postcondition 5 (backward-compat anchor):
        // Existing PASS/FAIL semantics for PluginResult::Ok UNCHANGED by S-25.01.
        // The policy parameter MUST NOT change the PASS/FAIL classification —
        // policy only affects the marker-write path via should_write_marker.

        // Pass path (exit_code=0, output_too_large=false) — fail-open context
        let ok_pass = PluginResult::Ok {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            elapsed_ms: 1,
            fuel_consumed: 10,
        };
        let pass = classify_outcome(ok_pass, FailurePolicy::FailOpen, false);
        assert_eq!(
            pass,
            DispatchOutcome::Pass,
            "backward compat: exit_code=0 + output_too_large=false MUST remain Pass"
        );

        // Fail path (exit_code=2) — fail-open context
        let ok_fail = PluginResult::Ok {
            exit_code: 2,
            stdout: String::new(),
            stderr: String::new(),
            elapsed_ms: 1,
            fuel_consumed: 10,
        };
        let fail = classify_outcome(ok_fail, FailurePolicy::FailOpen, false);
        assert_eq!(
            fail,
            DispatchOutcome::Fail { exit_code: 2 },
            "backward compat: exit_code=2 MUST remain Fail{{exit_code: 2}}"
        );

        // Policy MUST NOT change the PASS classification (policy-orthogonality)
        let ok_pass_closed = PluginResult::Ok {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            elapsed_ms: 1,
            fuel_consumed: 10,
        };
        let pass_closed = classify_outcome(ok_pass_closed, FailurePolicy::FailClosed, false);
        assert_eq!(
            pass_closed,
            DispatchOutcome::Pass,
            "AC-004: policy=FailClosed MUST NOT change a Pass outcome to anything else"
        );
    }

    // ── S-25.01 Red Gate stub 11 ──────────────────────────────────────────────
    #[test]
    fn test_BC_1_18_001_unrecognized_trap_routes_to_on_error_not_indeterminate() {
        // AC-004 / BC-1.18.001 invariant 2 (non-exhaustive Trap wildcard):
        // An unrecognized (future) Trap variant MUST NOT yield INDETERMINATE.
        // In invoke.rs, unrecognized Trap variants become PluginResult::Crashed (the `_ =>` arm).
        // classify_outcome MUST NOT classify Crashed as INDETERMINATE.
        // The `_ =>` wildcard arm routes to existing on_error handling.
        // Trap is `#[non_exhaustive]` — future unknown variants must be safe.
        let crash_result = PluginResult::Crashed {
            trap_string: "wasm trap: out of bounds memory access".to_string(),
            stderr: String::new(),
            elapsed_ms: 1,
            fuel_consumed: 0,
        };
        let outcome = classify_outcome(crash_result, FailurePolicy::FailClosed, false);
        assert!(
            !matches!(outcome, DispatchOutcome::Indeterminate { .. }),
            "AC-004 / BC-1.18.001 INV2: unrecognized Trap (PluginResult::Crashed) MUST NOT yield \
             INDETERMINATE — wildcard `_ =>` arm routes to on_error handling, not INDETERMINATE"
        );
    }

    // ── S-25.01 Red Gate stub 12 (upgraded to full event-body assertion) ────────
    #[test]
    fn test_BC_1_18_001_emits_plugin_indeterminate_event_with_required_fields() {
        // AC-006 / BC-3.08.001 Event 8:
        // emit_indeterminate MUST write a plugin.indeterminate JSONL event carrying
        // all 8 mandatory fields, including artifact_path == the passed-in envelope
        // file_path (HIGH-1 fix).
        //
        // Mandatory event fields (BC-3.08.001 Event 8):
        //   type           "plugin.indeterminate"
        //   trace_id       dispatcher_trace_id
        //   session_id     session ID from the dispatch context
        //   plugin_name    registry name of the plugin
        //   artifact_path  envelope file_path (never hardcoded empty)
        //   cause          "fuel" | "epoch" | "output-too-large"
        //   failure_policy "fail-closed" | "fail-open"
        //   timestamp      RFC 3339 (ts field in the event)
        //
        // Pattern mirrors emit_lifecycle_timeout_carries_fuel_cap_and_consumed.
        let dir = tempfile::tempdir().expect("tempdir");
        let log = InternalLog::new(dir.path().join("logs"));

        let mut base_ctx = crate::host::HostContext::new(
            "validate-factory-path-staging",
            "0.1.0",
            "sess-abc",
            "trace-xyz",
        );
        // ADR-048 §D4 v1.3 F-P3-001: emit_indeterminate now routes through
        // base_ctx.emit_internal, which requires internal_log wired for the
        // durable-sink assertion below (matches production main.rs wiring).
        base_ctx.internal_log = Some(Arc::new(log));
        let entry = RegistryEntry {
            name: "validate-factory-path-staging".to_string(),
            event: "PostToolUse".to_string(),
            tool: None,
            plugin: std::path::PathBuf::from("validate-factory-path-staging.wasm"),
            priority: None,
            enabled: true,
            timeout_ms: None,
            fuel_cap: None,
            on_error: None,
            capabilities: None,
            config: toml::Value::Table(toml::Table::new()),
            async_flag: false,
            needs_context: vec![],
            failure_policy: FailurePolicy::FailClosed,
        };

        // The artifact_path is the envelope file_path from tool_input.file_path
        // (populated by execute_tier / spawn_async_plugin before calling emit_indeterminate).
        let artifact_path = "/Users/dev/project/.factory/STATE.md";
        let cause = IndeterminateCause::Fuel;

        emit_indeterminate(&base_ctx, &entry, &cause, artifact_path);

        // Read back the JSONL event and assert all 8 BC-3.08.001 Event 8 fields.
        let log_dir = dir.path().join("logs");
        let files: Vec<_> = std::fs::read_dir(&log_dir)
            .expect("log dir must exist after emit_indeterminate write")
            .map(|e| e.expect("dir entry").path())
            .collect();
        assert_eq!(files.len(), 1, "expected exactly one log file");
        let content = std::fs::read_to_string(&files[0]).expect("read log file");
        let event: serde_json::Value =
            serde_json::from_str(content.trim_end()).expect("log line must be valid JSON");

        // Field 1: type
        assert_eq!(
            event["type"].as_str(),
            Some(PLUGIN_INDETERMINATE),
            "BC-3.08.001 Event 8: type must be 'plugin.indeterminate'"
        );
        // Field 2: trace_id (BC-3.08.001 v1.7 Invariant 5 — wire key is 'trace_id')
        assert_eq!(
            event["trace_id"].as_str(),
            Some("trace-xyz"),
            "BC-3.08.001 Event 8: trace_id must equal dispatcher_trace_id"
        );
        // Field 3: session_id
        assert_eq!(
            event["session_id"].as_str(),
            Some("sess-abc"),
            "BC-3.08.001 Event 8: session_id must be present and match"
        );
        // Field 4: plugin_name
        assert_eq!(
            event["plugin_name"].as_str(),
            Some("validate-factory-path-staging"),
            "BC-3.08.001 Event 8: plugin_name must match registry entry name"
        );
        // Field 5: artifact_path — HIGH-1: must be the real envelope file_path,
        // NOT a hardcoded empty string.
        assert_eq!(
            event["artifact_path"].as_str(),
            Some(artifact_path),
            "BC-3.08.001 Event 8 / HIGH-1: artifact_path MUST equal the envelope \
             file_path passed to emit_indeterminate, not a hardcoded empty string"
        );
        // Field 6: cause
        assert_eq!(
            event["cause"].as_str(),
            Some("fuel"),
            "BC-3.08.001 Event 8: cause must be 'fuel' for IndeterminateCause::Fuel"
        );
        // Field 7: failure_policy
        assert_eq!(
            event["failure_policy"].as_str(),
            Some("fail-closed"),
            "BC-3.08.001 Event 8: failure_policy must be 'fail-closed' for FailClosed"
        );
        // Field 8a: the common 'ts' field (always present on every InternalEvent).
        assert!(
            event["ts"].as_str().map(|s| !s.is_empty()).unwrap_or(false),
            "BC-3.08.001 Event 8: ts (timestamp) field must be present and non-empty"
        );
        // Field 8b (F-P10-002): BC-3.08.001 §Event 8 Wire format + Mandatory fields
        // ALSO declares a distinct top-level `timestamp` field (ISO-8601), separate
        // from the common `ts`/`ts_epoch` fields every InternalEvent carries — the
        // same convention the seven sibling BC-3.08.001 emitters in
        // `host/emit_event.rs` follow via `.with_field("timestamp", ts.as_str())`
        // (see e.g. `test_s19_09_t013_emit_plugin_completed_async_has_timestamp_field`).
        // No prior test asserted this field on `plugin.indeterminate`, which let the
        // real `emit_indeterminate` implementation ship without it (F-P10-002).
        let timestamp_value = event.get("timestamp");
        assert!(
            timestamp_value.is_some(),
            "F-P10-002 / BC-3.08.001 Event 8: emit_indeterminate must emit a distinct \
             'timestamp' field (Wire format + Mandatory fields list), separate from the \
             common 'ts' field; field is absent"
        );
        let timestamp_str = timestamp_value.and_then(|v| v.as_str()).unwrap_or("");
        assert!(
            !timestamp_str.is_empty(),
            "F-P10-002 / BC-3.08.001 Event 8: 'timestamp' field value must be non-empty; \
             got empty string"
        );

        // ADR-048 §D4 v1.3 F-P3-001 dual-write assertion: emit_indeterminate MUST
        // route through base_ctx.emit_internal — proof is that the SAME event also
        // landed on the drained-events queue, not just the durable InternalLog file.
        let drained = base_ctx.drain_events();
        assert_eq!(
            drained.len(),
            1,
            "ADR-048 §D4 v1.3: exactly one plugin.indeterminate event must reach \
             base_ctx.events (drain_events) — proves emit_internal, not a raw InternalLog::write"
        );
        assert_eq!(drained[0].type_, PLUGIN_INDETERMINATE);
        assert_eq!(drained[0].dispatcher_trace_id.as_deref(), Some("trace-xyz"));
        // F-P10-002: the drained (queue-sink) copy of the event must ALSO carry the
        // distinct 'timestamp' field, not just the durable-log copy asserted above.
        let drained_timestamp = drained[0]
            .fields
            .get("timestamp")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(
            !drained_timestamp.is_empty(),
            "F-P10-002 / BC-3.08.001 Event 8: drained ctx.events copy of plugin.indeterminate \
             must also carry a non-empty 'timestamp' field"
        );

        // F-P14-001 / BC-3.08.001 §Common Fields: "plugin_version is NOT emitted
        // by Events 1, 4, 5, 7, and 8." Event 8's mandatory-fields list (asserted
        // field-by-field above) is exactly the 8 fields checked above;
        // plugin_version is NOT among them and MUST be absent from the wire
        // event on BOTH sinks emit_indeterminate writes through.
        assert!(
            event.get("plugin_version").is_none(),
            "F-P14-001 / BC-3.08.001 §Common Fields: plugin_version is NOT emitted by \
             Event 8 (plugin.indeterminate) — the durable-log JSON must not carry a \
             'plugin_version' field, but it does"
        );
        assert!(
            drained[0].plugin_version.is_none(),
            "F-P14-001 / BC-3.08.001 §Common Fields: plugin_version is NOT emitted by \
             Event 8 (plugin.indeterminate) — the drained ctx.events copy must not carry \
             a plugin_version value, but it does"
        );
    }

    // ── End S-25.01 Red Gate stubs 1–12 ──────────────────────────────────────

    // ── S-25.01 additional Red Gate tests for uncovered ACs ──────────────────

    /// AC-015 / BC-1.18.004 postcondition 1–3 + BC-1.18.001 INV3:
    /// should_write_marker(Indeterminate, FailClosed) == true (the ONLY true case).
    /// should_write_marker(Pass/Fail, FailClosed) == false (non-INDETERMINATE outcomes never write).
    #[test]
    fn test_BC_1_18_001_should_write_marker_true_only_for_fail_closed_indeterminate() {
        // AC-015 / BC-1.18.004 PC1-3 + BC-1.18.001 INV3 (single-marker policy):
        // The only case where should_write_marker returns true:
        //   outcome=Indeterminate AND policy=FailClosed.

        // True cases (all three INDETERMINATE causes with FailClosed)
        let indet_fuel = DispatchOutcome::Indeterminate {
            cause: IndeterminateCause::Fuel,
        };
        assert!(
            should_write_marker(&indet_fuel, FailurePolicy::FailClosed),
            "BC-1.18.001: should_write_marker(Indeterminate(Fuel), FailClosed) MUST be true"
        );
        let indet_epoch = DispatchOutcome::Indeterminate {
            cause: IndeterminateCause::Epoch,
        };
        assert!(
            should_write_marker(&indet_epoch, FailurePolicy::FailClosed),
            "BC-1.18.001: should_write_marker(Indeterminate(Epoch), FailClosed) MUST be true"
        );
        let indet_otl = DispatchOutcome::Indeterminate {
            cause: IndeterminateCause::OutputTooLarge,
        };
        assert!(
            should_write_marker(&indet_otl, FailurePolicy::FailClosed),
            "BC-1.18.001: should_write_marker(Indeterminate(OTL), FailClosed) MUST be true"
        );

        // False cases: Pass/Fail with FailClosed must NOT write marker
        assert!(
            !should_write_marker(&DispatchOutcome::Pass, FailurePolicy::FailClosed),
            "should_write_marker(Pass, FailClosed) MUST be false (only INDETERMINATE writes)"
        );
        assert!(
            !should_write_marker(
                &DispatchOutcome::Fail { exit_code: 2 },
                FailurePolicy::FailClosed
            ),
            "should_write_marker(Fail, FailClosed) MUST be false (only INDETERMINATE writes)"
        );
    }

    /// AC-013 / BC-1.18.003 postcondition 2 — idempotent delete NotFound → Ok(()).
    /// `delete_marker_if_pass` MUST return Ok(()) when the file is already absent.
    /// `io::ErrorKind::NotFound` MUST be silently swallowed; all other errors propagated.
    #[test]
    fn test_BC_1_18_003_idempotent_delete_not_found_returns_ok() {
        // AC-013 / BC-1.18.003 postcondition 2:
        // delete_marker_if_pass is idempotent: absent file → Ok(()) (no-op).
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");
        // Verify the file is NOT present (no pre-write in this test)
        assert!(
            !marker_path.exists(),
            "pre-condition: marker must NOT exist for idempotent-delete test"
        );
        // First delete: absent file must return Ok(()) not Err(NotFound).
        // M-1: pass empty current_artifact_path; NotFound path returns before the read.
        let result1 = delete_marker_if_pass(&marker_path, "");
        assert!(
            result1.is_ok(),
            "AC-013: delete_marker_if_pass on absent path MUST return Ok(()) — \
             io::ErrorKind::NotFound MUST be swallowed, not propagated"
        );
        // Second delete: still absent → still Ok(()) (truly idempotent)
        let result2 = delete_marker_if_pass(&marker_path, "");
        assert!(
            result2.is_ok(),
            "AC-013: second delete_marker_if_pass call MUST also return Ok(()) (idempotent)"
        );
    }

    // ── End S-25.01 additional Red Gate tests ────────────────────────────────

    // ── CRIT-PR59-001 regression tests: advisory-block gate ──────────────────

    /// Advisory block fires when on_error=Continue and stdout contains outcome:block.
    /// Regression for CRIT-PR59-001: the AND-gate `on_error==Block &&` was removed;
    /// stdout `{"outcome":"block"}` is now sufficient regardless of on_error.
    #[test]
    fn advisory_block_fires_with_on_error_continue() {
        let outcome = PluginOutcome {
            plugin_name: "test-plugin".to_string(),
            plugin_version: "0.1.0".to_string(),
            on_error: OnError::Continue,
            result: PluginResult::Ok {
                exit_code: 0,
                stdout: r#"{"outcome":"block","reason":"test"}"#.to_string(),
                stderr: String::new(),
                elapsed_ms: 1,
                fuel_consumed: 10,
            },
            block_if_marker_fired: false,
            block_if_marker_fields: None,
        };
        assert!(
            plugin_requests_block(&outcome.result),
            "stdout outcome:block must be detected regardless of on_error"
        );
        // Aggregate to verify exit_code=2 path
        let block = plugin_requests_block(&outcome.result);
        assert!(block, "block_intent must be true");
    }

    /// Regression: on_error=Continue with no stdout block → no block intent.
    #[test]
    fn advisory_block_absent_with_on_error_continue_and_no_block_stdout() {
        let outcome = PluginOutcome {
            plugin_name: "test-plugin".to_string(),
            plugin_version: "0.1.0".to_string(),
            on_error: OnError::Continue,
            result: PluginResult::Ok {
                exit_code: 0,
                stdout: "all good, no block here".to_string(),
                stderr: String::new(),
                elapsed_ms: 1,
                fuel_consumed: 5,
            },
            block_if_marker_fired: false,
            block_if_marker_fields: None,
        };
        assert!(
            !plugin_requests_block(&outcome.result),
            "no outcome:block stdout → block_intent must be false"
        );
    }

    #[test]
    fn plugin_requests_block_detects_tagged_json() {
        let r = PluginResult::Ok {
            exit_code: 2,
            stdout: r#"{"outcome":"block","reason":"policy 9"}"#.to_string(),
            stderr: String::new(),
            elapsed_ms: 3,
            fuel_consumed: 10,
        };
        assert!(plugin_requests_block(&r));
    }

    #[test]
    fn plugin_requests_block_false_for_continue() {
        let r = PluginResult::Ok {
            exit_code: 0,
            stdout: r#"{"outcome":"continue"}"#.to_string(),
            stderr: String::new(),
            elapsed_ms: 2,
            fuel_consumed: 5,
        };
        assert!(!plugin_requests_block(&r));
    }

    #[test]
    fn plugin_requests_block_false_for_crash() {
        let r = PluginResult::Crashed {
            trap_string: "unreachable".to_string(),
            stderr: "panicked at 'unreachable'".to_string(),
            elapsed_ms: 1,
            fuel_consumed: 0,
        };
        assert!(!plugin_requests_block(&r));
    }

    #[test]
    fn plugin_requests_block_false_for_timeout() {
        let r = PluginResult::Timeout {
            cause: TimeoutCause::Epoch,
            stderr: String::new(),
            elapsed_ms: 5_000,
            fuel_consumed: 0,
            fuel_cap: DEFAULT_FUEL_CAP,
        };
        assert!(!plugin_requests_block(&r));
    }

    // ── ADR-019 §Decision 2 fail-closed tests: plugin_fail_closed ────────────

    /// Crashed + on_error=Block → fail-closed (exit 2).
    /// This is the TC-8 root cause: WASI trap doesn't set exit_code; the
    /// aggregator must detect Crashed+Block independently.
    ///
    /// ADR-019 §Decision 2, BC-1.14.001 Error Paths, BC-7.06.001 Invariant 1.
    #[test]
    fn fail_closed_crashes_with_on_error_block() {
        let r = PluginResult::Crashed {
            trap_string: "unreachable".to_string(),
            stderr: String::new(),
            elapsed_ms: 1,
            fuel_consumed: 0,
        };
        assert!(
            plugin_fail_closed(&r, OnError::Block),
            "Crashed + on_error=Block must trigger fail-closed"
        );
    }

    /// Crashed + on_error=Continue → NOT fail-closed (fail-open, normal advisory path).
    #[test]
    fn fail_closed_crash_with_on_error_continue_is_open() {
        let r = PluginResult::Crashed {
            trap_string: "unreachable".to_string(),
            stderr: String::new(),
            elapsed_ms: 1,
            fuel_consumed: 0,
        };
        assert!(
            !plugin_fail_closed(&r, OnError::Continue),
            "Crashed + on_error=Continue must NOT trigger fail-closed"
        );
    }

    /// Timeout + on_error=Block → fail-closed (exit 2).
    /// A timed-out gate hook also cannot emit stdout; fail-closed must apply.
    ///
    /// ADR-019 §Decision 2.
    #[test]
    fn fail_closed_timeout_with_on_error_block() {
        let r = PluginResult::Timeout {
            cause: TimeoutCause::Epoch,
            stderr: String::new(),
            elapsed_ms: 5_000,
            fuel_consumed: 0,
            fuel_cap: DEFAULT_FUEL_CAP,
        };
        assert!(
            plugin_fail_closed(&r, OnError::Block),
            "Timeout + on_error=Block must trigger fail-closed"
        );
    }

    /// Timeout + on_error=Continue → NOT fail-closed.
    #[test]
    fn fail_closed_timeout_with_on_error_continue_is_open() {
        // fuel_consumed == fuel_cap on Trap::OutOfFuel (remaining=0 → cap.saturating_sub(0)=cap).
        let r = PluginResult::Timeout {
            cause: TimeoutCause::Fuel,
            stderr: String::new(),
            elapsed_ms: 5_000,
            fuel_consumed: DEFAULT_FUEL_CAP,
            fuel_cap: DEFAULT_FUEL_CAP,
        };
        assert!(
            !plugin_fail_closed(&r, OnError::Continue),
            "Timeout + on_error=Continue must NOT trigger fail-closed"
        );
    }

    /// Ok result + on_error=Block → NOT fail-closed (advisory path handles this).
    #[test]
    fn fail_closed_ok_result_is_not_fail_closed() {
        let r = PluginResult::Ok {
            exit_code: 0,
            stdout: r#"{"outcome":"continue"}"#.to_string(),
            stderr: String::new(),
            elapsed_ms: 10,
            fuel_consumed: 100,
        };
        assert!(
            !plugin_fail_closed(&r, OnError::Block),
            "Ok result + on_error=Block must NOT trigger fail-closed (advisory path handles Ok)"
        );
    }

    // ── emit_lifecycle telemetry field coverage ────────────────────────────

    /// `plugin.timeout` events must carry both `fuel_cap` and `fuel_consumed`
    /// so operators can see which budget was hit without opening the full trace.
    /// CLAUDE.md Factory Hook Diagnostics Step 2 sends operators to grep
    /// `.factory/logs/dispatcher-internal-*.jsonl` for exactly these fields.
    #[test]
    fn emit_lifecycle_timeout_carries_fuel_cap_and_consumed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let log = InternalLog::new(dir.path().join("logs"));
        let base_ctx = crate::host::HostContext::new("test-plugin", "0.1.0", "sess-1", "trace-1");
        let entry = RegistryEntry {
            name: "test-plugin".to_string(),
            event: "PreToolUse".to_string(),
            tool: None,
            plugin: std::path::PathBuf::from("test.wasm"),
            priority: None,
            enabled: true,
            timeout_ms: None,
            fuel_cap: None,
            on_error: None,
            capabilities: None,
            config: toml::Value::Table(toml::Table::new()),
            async_flag: false,
            needs_context: vec![],
            failure_policy: crate::registry::FailurePolicy::FailOpen,
        };
        let cap: u64 = DEFAULT_FUEL_CAP;
        let consumed: u64 = 12_345_678;
        let result = PluginResult::Timeout {
            cause: TimeoutCause::Fuel,
            stderr: String::new(),
            elapsed_ms: 10,
            fuel_consumed: consumed,
            fuel_cap: cap,
        };
        emit_lifecycle(&log, &base_ctx, &entry, &result);

        // Read back the JSONL and verify both fields are present.
        let log_dir = dir.path().join("logs");
        let files: Vec<_> = std::fs::read_dir(&log_dir)
            .expect("log dir must exist after write")
            .map(|e| e.expect("dir entry").path())
            .collect();
        assert_eq!(files.len(), 1, "expected exactly one log file");
        let content = std::fs::read_to_string(&files[0]).expect("read log file");
        let event: serde_json::Value =
            serde_json::from_str(content.trim_end()).expect("log line must be valid JSON");
        assert_eq!(
            event["type"].as_str(),
            Some(PLUGIN_TIMEOUT),
            "event type must be plugin.timeout"
        );
        assert_eq!(
            event["fuel_consumed"].as_u64(),
            Some(consumed),
            "plugin.timeout event must carry fuel_consumed"
        );
        assert_eq!(
            event["fuel_cap"].as_u64(),
            Some(cap),
            "plugin.timeout event must carry fuel_cap (CLAUDE.md Diagnostics Step 2)"
        );
    }

    // ── ADR-048 §Decision 1: plugin_block_if_marker unit tests (BC-1.18.002) ──

    /// BC-1.18.002 PC5: Crashed + on_error=BlockIfMarker + active marker (future expires_at)
    /// → plugin_block_if_marker returns true (Block).
    #[test]
    fn test_BC_1_18_002_block_if_marker_crashed_with_marker_blocks() {
        let dir = tempfile::tempdir().expect("tempdir");
        let factory_dir = dir.path().join(".factory");
        std::fs::create_dir_all(&factory_dir).expect("create .factory subdir");
        let marker_path = factory_dir.join("unvalidated-mutation.marker");
        let fields = MarkerFields {
            timestamp: "2026-08-31T00:00:00Z".to_string(),
            plugin_name: "p".to_string(),
            artifact_path: String::new(),
            cause: "fuel".to_string(),
            trace_id: "trace-bim-unit-1".to_string(),
            expires_at: "2099-01-01T00:00:00Z".to_string(),
        };
        write_indeterminate_marker(&fields, &marker_path).expect("write marker");
        let r = PluginResult::Crashed {
            trap_string: "unreachable".to_string(),
            stderr: String::new(),
            elapsed_ms: 10,
            fuel_consumed: 0,
        };
        let now = chrono::Utc::now();
        assert!(
            plugin_block_if_marker(&r, OnError::BlockIfMarker, dir.path(), now),
            "BC-1.18.002 PC5: Crashed + BlockIfMarker + active marker MUST return true (Block)"
        );
    }

    /// BC-1.18.002: Crashed + on_error=BlockIfMarker + no marker → false (Allow).
    #[test]
    fn test_BC_1_18_002_block_if_marker_crashed_no_marker_allows() {
        let dir = tempfile::tempdir().expect("tempdir");
        let factory_dir = dir.path().join(".factory");
        std::fs::create_dir_all(&factory_dir).expect("create .factory subdir");
        // No marker written.
        let r = PluginResult::Crashed {
            trap_string: "unreachable".to_string(),
            stderr: String::new(),
            elapsed_ms: 10,
            fuel_consumed: 0,
        };
        let now = chrono::Utc::now();
        assert!(
            !plugin_block_if_marker(&r, OnError::BlockIfMarker, dir.path(), now),
            "BC-1.18.002: Crashed + BlockIfMarker + absent marker MUST return false (Allow)"
        );
    }

    /// BC-1.18.002 PC6: Crashed + on_error=BlockIfMarker + expired marker → false (Allow).
    ///
    /// TTL elapsed: expired marker is treated as absent per ADR-048 §Decision 2.
    #[test]
    fn test_BC_1_18_002_block_if_marker_crashed_expired_marker_allows() {
        let dir = tempfile::tempdir().expect("tempdir");
        let factory_dir = dir.path().join(".factory");
        std::fs::create_dir_all(&factory_dir).expect("create .factory subdir");
        let marker_path = factory_dir.join("unvalidated-mutation.marker");
        let fields = MarkerFields {
            timestamp: "2020-01-01T00:00:00Z".to_string(),
            plugin_name: "p".to_string(),
            artifact_path: String::new(),
            cause: "fuel".to_string(),
            trace_id: "trace-bim-expired".to_string(),
            expires_at: "2020-01-02T00:00:00Z".to_string(),
        };
        write_indeterminate_marker(&fields, &marker_path).expect("write marker");
        let r = PluginResult::Crashed {
            trap_string: "unreachable".to_string(),
            stderr: String::new(),
            elapsed_ms: 10,
            fuel_consumed: 0,
        };
        let now = chrono::Utc::now();
        assert!(
            !plugin_block_if_marker(&r, OnError::BlockIfMarker, dir.path(), now),
            "BC-1.18.002 PC6: Crashed + BlockIfMarker + expired marker MUST return false (Allow)"
        );
    }

    /// BC-1.18.002: Timeout + on_error=BlockIfMarker + active marker → true (Block).
    ///
    /// Both Crashed and Timeout variants trigger the conditional block gate.
    #[test]
    fn test_BC_1_18_002_block_if_marker_timeout_with_marker_blocks() {
        let dir = tempfile::tempdir().expect("tempdir");
        let factory_dir = dir.path().join(".factory");
        std::fs::create_dir_all(&factory_dir).expect("create .factory subdir");
        let marker_path = factory_dir.join("unvalidated-mutation.marker");
        let fields = MarkerFields {
            timestamp: "2026-08-31T00:00:00Z".to_string(),
            plugin_name: "p".to_string(),
            artifact_path: String::new(),
            cause: "epoch".to_string(),
            trace_id: "trace-bim-timeout".to_string(),
            expires_at: "2099-01-01T00:00:00Z".to_string(),
        };
        write_indeterminate_marker(&fields, &marker_path).expect("write marker");
        let r = PluginResult::Timeout {
            cause: TimeoutCause::Epoch,
            stderr: String::new(),
            elapsed_ms: 5_000,
            fuel_consumed: 0,
            fuel_cap: DEFAULT_FUEL_CAP,
        };
        let now = chrono::Utc::now();
        assert!(
            plugin_block_if_marker(&r, OnError::BlockIfMarker, dir.path(), now),
            "BC-1.18.002: Timeout + BlockIfMarker + active marker MUST return true (Block)"
        );
    }

    /// BC-1.18.002 ADR-048 §D1: Ok result + on_error=BlockIfMarker + active marker → false.
    ///
    /// Only Crashed and Timeout results can trigger block_if_marker; Ok success never blocks.
    #[test]
    fn test_BC_1_18_002_block_if_marker_ok_result_never_blocks() {
        let dir = tempfile::tempdir().expect("tempdir");
        let factory_dir = dir.path().join(".factory");
        std::fs::create_dir_all(&factory_dir).expect("create .factory subdir");
        let marker_path = factory_dir.join("unvalidated-mutation.marker");
        let fields = MarkerFields {
            timestamp: "2026-08-31T00:00:00Z".to_string(),
            plugin_name: "p".to_string(),
            artifact_path: String::new(),
            cause: "fuel".to_string(),
            trace_id: "trace-bim-ok".to_string(),
            expires_at: "2099-01-01T00:00:00Z".to_string(),
        };
        write_indeterminate_marker(&fields, &marker_path).expect("write marker");
        let r = PluginResult::Ok {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            elapsed_ms: 10,
            fuel_consumed: 100,
        };
        let now = chrono::Utc::now();
        assert!(
            !plugin_block_if_marker(&r, OnError::BlockIfMarker, dir.path(), now),
            "BC-1.18.002 ADR-048 §D1: Ok result + BlockIfMarker MUST return false \
             (no block on successful plugin invocation)"
        );
    }

    /// BC-1.18.002: on_error=Continue + Crashed + active marker → false.
    ///
    /// block_if_marker is an exclusive gate for on_error=BlockIfMarker only;
    /// on_error=Continue never gates on marker presence.
    #[test]
    fn test_BC_1_18_002_block_if_marker_on_error_continue_never_blocks() {
        let dir = tempfile::tempdir().expect("tempdir");
        let factory_dir = dir.path().join(".factory");
        std::fs::create_dir_all(&factory_dir).expect("create .factory subdir");
        let marker_path = factory_dir.join("unvalidated-mutation.marker");
        let fields = MarkerFields {
            timestamp: "2026-08-31T00:00:00Z".to_string(),
            plugin_name: "p".to_string(),
            artifact_path: String::new(),
            cause: "fuel".to_string(),
            trace_id: "trace-bim-continue".to_string(),
            expires_at: "2099-01-01T00:00:00Z".to_string(),
        };
        write_indeterminate_marker(&fields, &marker_path).expect("write marker");
        let r = PluginResult::Crashed {
            trap_string: "unreachable".to_string(),
            stderr: String::new(),
            elapsed_ms: 10,
            fuel_consumed: 0,
        };
        let now = chrono::Utc::now();
        assert!(
            !plugin_block_if_marker(&r, OnError::Continue, dir.path(), now),
            "BC-1.18.002: on_error=Continue + Crashed + active marker MUST return false \
             (block_if_marker only triggers for on_error=BlockIfMarker)"
        );
    }
}
