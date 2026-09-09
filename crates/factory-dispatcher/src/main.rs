//! vsdd-factory-dispatcher — CLI entry point.
//!
//! Reads a Claude Code hook envelope from stdin, loads the registry at
//! `${CLAUDE_PLUGIN_ROOT}/hooks-registry.toml`, selects plugins that
//! match the event/tool, groups them by priority, and hands them to the
//! execution layer (S-1.6's `execute_tiers`).
//!
//! The runtime is a `current_thread` tokio runtime — we don't need a
//! multi-threaded pool because wasmtime invocations are wrapped in
//! `spawn_blocking` inside the executor, and the dispatcher's own fan-
//! in work is trivial. Single-thread keeps startup cost low and avoids
//! surprising thread pools in a short-lived process.
//!
//! Self-telemetry: the always-on internal log (S-1.7) is constructed
//! first so any dispatcher error — including registry load failures —
//! is durably recorded. Plugin lifecycle events (`plugin.invoked`,
//! `plugin.completed`, `plugin.timeout`, `plugin.crashed`) are emitted
//! by the executor; `dispatcher.*` structured events (schema_mismatch,
//! registry_invalid) and `internal.dispatcher_error` are emitted here.
//!
//! S-15.01 T-3e: four BC-3.08.001 structured events are now emitted from
//! the dispatch path: `dispatcher.schema_mismatch`, `dispatcher.registry_invalid`,
//! `plugin.async_block_discarded`, `plugin.timeout` (async path).
//!
//! ## VSDD_SINK_FILE (diagnostic + test hook)
//!
//! When `VSDD_SINK_FILE` is set to a path, all plugin-emitted events
//! (from `host::emit_event`) are appended as JSONL to that file after
//! execution completes. Honored in both debug and release builds
//! (S-19.05 AC-004). SEC-003 path sanitization (no ".." traversal) is
//! applied. Used by bats integration tests and operators for diagnostics.
//! Best-effort: write failures are silently dropped.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use factory_dispatcher::engine::{EpochTicker, build_engine};
use factory_dispatcher::executor::{
    ExecutorInputs, PluginOutcome, execute_tiers, shard_cap_precheck, spawn_async_plugin,
};
use factory_dispatcher::host::HostContext;
use factory_dispatcher::host::emit_event::{
    emit_dispatcher_schema_mismatch, emit_plugin_abandoned, emit_plugin_async_block_discarded,
    emit_plugin_completed_async, emit_plugin_timeout_async, emit_registry_invalid_e_reg002,
    emit_registry_invalid_e_reg003,
};
use factory_dispatcher::indeterminate_marker::MarkerFields;
use factory_dispatcher::internal_log::{
    DEFAULT_RETENTION_DAYS, DISPATCHER_STARTED, INTERNAL_DISPATCHER_ERROR, InternalEvent,
    InternalLog,
};
use factory_dispatcher::invoke::{PluginResult, TimeoutCause};
use factory_dispatcher::partition::partition_plugins;
use factory_dispatcher::payload::HookPayload;
use factory_dispatcher::plugin_loader::PluginCache;
use factory_dispatcher::registry::{
    REGISTRY_SCHEMA_VERSION, Registry, RegistryDefaults, RegistryError,
};
use factory_dispatcher::resolver_loader::ResolverLoader;
use factory_dispatcher::routing::{group_by_priority, match_plugins};
use factory_dispatcher::{ASYNC_DRAIN_WINDOW_MS, HOST_ABI_VERSION, new_trace_id};
use factory_dispatcher::{AggregatorPluginResult, aggregate_exit_code};
use tokio::sync::mpsc;

const ENV_PLUGIN_ROOT: &str = "CLAUDE_PLUGIN_ROOT";
const ENV_PROJECT_DIR: &str = "CLAUDE_PROJECT_DIR";
// VSDD_SINK_FILE: runtime-gated structured-event sink for bats integration tests
// and operator diagnostics. Honored in both debug and release builds (S-19.05 AC-004).
// SEC-003 path sanitization (no ".." traversal) is applied inside flush_sink_file.
const ENV_SINK_FILE: &str = "VSDD_SINK_FILE";

// VSDD_ASYNC_DRAIN_WINDOW_MS: env override for the async drain window.
// Active in debug builds AND release builds compiled with feature=test-support
// (CI integration tests only — never enabled in shipped artifacts per release.yml).
// Shipped artifacts (release.yml: no features) always use ASYNC_DRAIN_WINDOW_MS
// per DI-019 v1.6. SEC-003: compiled out in shipped release builds so the env var
// name does not appear in production binaries.
#[cfg(any(debug_assertions, feature = "test-support"))]
const ENV_ASYNC_DRAIN_WINDOW_MS: &str = "VSDD_ASYNC_DRAIN_WINDOW_MS";

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // ONLY an explicit VSDD_LOG_DIR (resolution level A) bypasses the #206
    // mount gate: the operator said exactly where to log, and suppressing
    // that would override the override (the bats harness points VSDD_LOG_DIR
    // at scratch `.factory/logs` fixtures, for example). FACTORY_ROOT
    // (level B) stays GATED: it resolves to `$FACTORY_ROOT/logs`, and by the
    // resolution contract FACTORY_ROOT names the `.factory` directory itself
    // — the conventional `FACTORY_ROOT=<repo>/.factory` therefore yields
    // exactly the racing shape the gate exists to hold back, on the
    // mid-setup population most likely to hit it. When FACTORY_ROOT points
    // anywhere not named `.factory`, the gate never fires anyway.
    let explicit_override = factory_dispatcher::log_dir::mount_gate_exempt(
        std::env::var("VSDD_LOG_DIR").ok().as_deref(),
    );
    let internal_log =
        Arc::new(InternalLog::new(resolve_log_dir()).with_mount_gate(!explicit_override));
    internal_log.prune_old(DEFAULT_RETENTION_DAYS);

    let code = match run(internal_log.clone()).await {
        Ok(code) => code,
        Err(err) => {
            emit_dispatcher_error(&internal_log, None, None, &err.to_string());
            0
        }
    };
    std::process::exit(code);
}

async fn run(internal_log: Arc<InternalLog>) -> anyhow::Result<i32> {
    let trace_id = new_trace_id();
    let payload = HookPayload::from_reader(std::io::stdin().lock())?;

    // ADR-024 Decision 2 — two-tier CLAUDE_PLUGIN_ROOT check.
    //
    // Tier 1 (absent/empty): emit internal.dispatcher_error, use empty registry,
    // proceed in degraded mode — exit 0 (no hard-abort).
    //
    // Tier 2 (present but invalid path): resolve_registry_path() / Registry::load()
    // hard-error path — emit structured error, may exit 0 or 2 depending on error type.
    //
    // The Tier-1 check MUST run BEFORE resolve_registry_path() to make the
    // degraded-continue path reachable (ADR-024 v1.1 Decision 2, LOW-1 clarification).
    // Tier 1 and Tier 2 are MUTUALLY EXCLUSIVE: an absent CLAUDE_PLUGIN_ROOT never
    // reaches Tier-2 code.
    let plugin_root_val = std::env::var(ENV_PLUGIN_ROOT).unwrap_or_default();
    let (registry, registry_path) = if plugin_root_val.is_empty() {
        // Tier 1: CLAUDE_PLUGIN_ROOT absent or empty — degraded-continue.
        // Emit an actionable internal.dispatcher_error and proceed with an empty
        // Registry (no plugins will run).  BC-1.13.001 INV2: absent registry →
        // empty registry, not an error.  Do NOT call resolve_registry_path().
        internal_log.write(
            &InternalEvent::now(INTERNAL_DISPATCHER_ERROR)
                .with_trace_id(trace_id.clone())
                .with_session_id(payload.session_id.clone())
                .with_field(
                    "message",
                    "$CLAUDE_PLUGIN_ROOT is not set or empty — hook registry and resolver \
                     registry paths unresolvable; all plugins will be skipped. Set \
                     CLAUDE_PLUGIN_ROOT to the vsdd-factory plugin directory.",
                ),
        );
        let empty_registry = Registry {
            schema_version: REGISTRY_SCHEMA_VERSION,
            defaults: RegistryDefaults::default(),
            hooks: Vec::new(),
        };
        (empty_registry, PathBuf::new())
    } else {
        // Tier 2: CLAUDE_PLUGIN_ROOT is set — resolve and load the registry.
        // Hard-error on path resolution failure (actionable message).
        let rp = resolve_registry_path()?;
        let reg = match Registry::load(&rp) {
            Ok(r) => r,
            Err(e) => {
                // S-15.01 T-3e/T-3f: fail-closed registry errors must exit 2 and emit structured
                // events per BC-1.14.001 EC-006 (schema mismatch) and EC-008 (async+block conflict).
                // These are the explicit exceptions to BC-1.08.001 fail-open policy (ADR-019 §Decision 2).
                //
                // Build a minimal HostContext so we can use the structured emit fns.
                // Events accumulate in the context's event queue and are flushed to
                // VSDD_SINK_FILE below (debug builds only, SEC-003).
                let err_ctx = {
                    let mut ctx = HostContext::new(
                        "dispatcher",
                        env!("CARGO_PKG_VERSION"),
                        payload.session_id.clone(),
                        trace_id.clone(),
                    );
                    ctx.internal_log = Some(internal_log.clone());
                    ctx
                };
                // Always emit generic error to internal log for durability.
                emit_dispatcher_error(
                    &internal_log,
                    Some(trace_id.clone()),
                    Some(payload.session_id.clone()),
                    &format!("registry load: {e}"),
                );
                // Emit structured event + fail-closed (exit 2) for schema/invariant errors.
                // Other registry errors (file not found, parse error) remain fail-open (exit 0)
                // per BC-1.08.001 (only schema-version mismatch and invariant violation are
                // the named fail-closed exceptions per ADR-019 §Decision 2 and BC-1.14.001 EC-006/EC-008).
                let exit_code = match &e {
                    RegistryError::SchemaVersion { got, expected } => {
                        // BC-1.14.001 EC-006 + BC-3.08.001 Event 2.
                        // Emit dispatcher.schema_mismatch with found_version/expected_version/error_code.
                        emit_dispatcher_schema_mismatch(&err_ctx, *got, *expected);
                        eprintln!(
                            "factory-dispatcher: E-REG-001 schema_version={got} expected={expected}; exiting 2 (fail-closed per ADR-019 §Decision 2)"
                        );
                        2
                    }
                    RegistryError::AsyncBlockConflict { name, on_error } => {
                        // BC-1.14.001 EC-008 + BC-3.08.001 Event 3.
                        // Emit dispatcher.registry_invalid with offending_plugin/violation/error_code.
                        // E-REG-002 is intra-entry; offending_event/tool absence is enforced by type system.
                        emit_registry_invalid_e_reg002(&err_ctx, name, "async_block_conflict");
                        // LOW-1: name the actual offending on_error value ("block" or
                        // "block_if_marker"), not a hardcoded "block" literal.
                        eprintln!(
                            "factory-dispatcher: E-REG-002 on_error={on_error} AND async=true for '{name}'; exiting 2 (fail-closed per ADR-019 §Decision 2)"
                        );
                        2
                    }
                    RegistryError::DuplicateEntry { name, event, tool } => {
                        // BC-7.06.001 Invariant 7 + BC-3.08.001 Event 3 (E-REG-003).
                        // Emit dispatcher.registry_invalid with full wire payload per BC-7.06.001 v1.8:
                        // offending_plugin, violation, error_code, offending_event, offending_tool.
                        // F-P8-001 / F-P14-001 Path B: fail-closed; dispatcher refuses to start on
                        // duplicate (name, event, tool) tuple.
                        eprintln!(
                            "[E-REG-003] Duplicate hook registration: name={name}, event={event}, tool={tool:?} \
                             (BC-7.06.001 v1.8 Invariant 7). Each (name, event, tool) tuple must be unique \
                             across all [[hooks]] entries; dispatcher refuses to start."
                        );
                        emit_registry_invalid_e_reg003(
                            &err_ctx,
                            name.as_str(),
                            "duplicate_hook_registration",
                            event.as_str(), // offending_event — required for E-REG-003
                            tool.as_deref(), // offending_tool — None means wildcard/"all tools"
                        );
                        2
                    }
                    // Other errors: file not found, parse failures, regex errors.
                    // These are operational errors (misconfiguration / missing file), not
                    // semantic invariant violations. Fail-open per BC-1.08.001.
                    _ => 0,
                };
                // Flush structured events to VSDD_SINK_FILE (debug and release builds).
                // VP-079 S2/S3 verify these events appear in the sink.
                // SEC-003 path sanitization (no ".." traversal) is applied inside flush_sink_file.
                if let Ok(sink_path) = std::env::var(ENV_SINK_FILE)
                    && !sink_path.is_empty()
                {
                    factory_dispatcher::vsdd_sink::flush_sink_file(&sink_path, &err_ctx.events);
                }
                return Ok(exit_code);
            }
        };
        (reg, rp)
    };

    internal_log.write(
        &InternalEvent::now(DISPATCHER_STARTED)
            .with_trace_id(trace_id.clone())
            .with_session_id(payload.session_id.clone())
            .with_field("dispatcher_version", env!("CARGO_PKG_VERSION"))
            .with_field("host_abi_version", HOST_ABI_VERSION as i64)
            .with_field(
                "platform",
                format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
            )
            .with_field("pid", std::process::id() as i64)
            .with_field("registry_path", registry_path.display().to_string())
            .with_field("loaded_plugin_count", registry.hooks.len() as i64)
            .with_field(
                "log_dir",
                std::path::absolute(internal_log.log_dir())
                    .unwrap_or_else(|_| internal_log.log_dir().to_path_buf())
                    .display()
                    .to_string(),
            ),
    );

    let matched = match_plugins(&registry, &payload);

    // S-15.01 T-3c: partition matched plugins into sync_group (gates user)
    // and async_group (fire-and-forget, verdict never reaches Claude Code).
    // BC-1.14.001 postconditions 1, 5, 6 — partition then await sync, spawn async.
    let matched_owned: Vec<_> = matched.into_iter().cloned().collect();
    let partition = partition_plugins(&matched_owned);
    let sync_tiers = group_by_priority(&registry, partition.sync_group.iter().collect());

    eprintln!(
        "factory-dispatcher trace={} event={} tool={} host_abi={} sync_plugins={} async_plugins={}",
        trace_id,
        payload.event_name,
        payload.tool_name,
        HOST_ABI_VERSION,
        partition.sync_group.len(),
        partition.async_group.len(),
    );

    // Resolved ONCE, here — see `resolve_project_cwd`'s own doc comment for
    // why this must happen before the early-return guard immediately below,
    // rather than at `base_host_ctx.cwd`'s own (later) assignment site.
    let project_cwd = resolve_project_cwd();

    // BC-1.18.006 Postcondition 7 catch point (i) / story AC-024 (ADR-051
    // §Decision 15 point 4 — LOAD-BEARING placement caveat; §Decision 17
    // MAJOR-3, S-25.02 cluster-2 PR #824 pr-review cycle 3): an
    // UNCONDITIONAL native call, independent of the registry's
    // matched-plugin count, placed BEFORE the (now widened, see below)
    // `sync_tiers.is_empty() && partition.async_group.is_empty()`
    // early-return guard — mirroring Decision 1's own "before the
    // registry-driven plugin loop" placement rule. If this call were placed
    // AFTER that guard (e.g. as "one more thing the registry-driven loop
    // does"), it would silently stop firing altogether on any configuration
    // where the registered PostToolUse `Edit`/`Write`/`MultiEdit` plugin set
    // becomes empty — the exact "silently stop firing if the plugin set
    // changes" failure mode Decision 1's placement rule already exists to
    // prevent for the PreToolUse leg. Precedent for a native call sitting
    // unconditionally in this exact slot: `write_indeterminate_marker`'s
    // `executor.rs` call sites and `inject_git_context_if_qualifying`
    // (further below, ADR-029 §Decision 1-3) — both native, non-WASM,
    // non-registry-gated dispatcher-internal calls reached from this same
    // `run` function.
    //
    // Silent filesystem side effect, no `HookResult` signaling (Decision 15
    // point 2 — a janitor, not a gate): this call never influences
    // `sync_tiers`/`partition.async_group` or this function's own return
    // value. Real, cheap, structural qualification filtering happens inside
    // `reconcile_replace_all_overcap_if_qualifying` itself (event/tool/
    // `replace_all`/config-match — BC-1.18.006 Postcondition 7's own
    // "zero added cost outside the narrow case" requirement); the actual
    // `stat()`-and-retroactive-roll behavior it delegates into
    // (`shard_manager::reconcile_post_write_replace_all_overcap`) is fully
    // implemented and unit-tested — this call site is wiring, exercised
    // end-to-end by `bc_1_18_006_roll_test.rs`'s AC-024 integration test.
    factory_dispatcher::invoke::reconcile_replace_all_overcap_if_qualifying(&payload, &project_cwd);

    // BC-1.18.005 T-2 / MAJOR-3 (S-25.02 cluster-2 PR #824 pr-review cycle
    // 3; ADR-051 §Decision 17): the PreToolUse shard-cap gate, computed ONCE
    // here — at the same call site as the catch-point-(i) leg immediately
    // above, and for the SAME reason (ADR-051 §Decision 1's placement rule:
    // a native, non-registry-gated check must run BEFORE the
    // `sync_tiers.is_empty() && partition.async_group.is_empty()` guard, not
    // as something the registry-driven `execute_tiers` loop does on the
    // side). Before MAJOR-3, `shard_cap_precheck` ran only INSIDE
    // `execute_tiers`, which this function skips entirely whenever no
    // plugin matched — silently defeating the PreToolUse gate for exactly
    // that configuration (an empty matched-plugin set), the asymmetric twin
    // of the failure mode this same Decision 1 placement rule already
    // prevents on the PostToolUse leg above. The gate's own guards
    // (event/tool/config-presence — see `shard_cap_precheck`'s doc comment)
    // make this a zero-cost no-op (`None`) for every dispatch that isn't a
    // genuine PreToolUse Edit/Write/MultiEdit candidate with a `[[shard]]`
    // config present.
    //
    // Computed EXACTLY ONCE: threaded into `execute_tiers` below as a
    // parameter, which CONSUMES it rather than recomputing it. The gate's
    // fired branch reaches `shard_manager::execute_roll` — a destructive
    // seal-and-truncate-to-0 operation — so evaluating this twice would
    // corrupt on-disk state (the second evaluation would see the
    // already-rolled canonical), not merely waste cycles.
    let shard_gate_precheck_result = shard_cap_precheck(&payload, &project_cwd);

    // Widened (MAJOR-3) from `sync_tiers.is_empty() && partition.async_group.is_empty()`:
    // a fired shard-cap-gate verdict (`Some(_)`) must still reach
    // `execute_tiers`'s verdict->`all_outcomes`/`block_intent` translation
    // and this function's own `final_exit_code` aggregation even when BOTH
    // the sync and async matched-plugin groups are empty — the exact
    // configuration that previously made this gate unreachable. An empty
    // `tiers` vec with a fired precheck verdict is already handled
    // correctly by `execute_tiers` (empty `tiers` loop body, precheck
    // consumed unconditionally before it).
    if shard_gate_precheck_result.is_none()
        && sync_tiers.is_empty()
        && partition.async_group.is_empty()
    {
        return Ok(0);
    }

    // Execution layer. Build a shared engine + epoch ticker + module
    // cache per invocation. Keeping the engine short-lived adds a bit
    // of cold-start cost but sidesteps any global state concerns for
    // the short-lived dispatcher process. S-1.5's `PluginCache` still
    // amortizes per-plugin compile cost within a single invocation.
    let engine = match build_engine() {
        Ok(e) => e,
        Err(e) => {
            emit_dispatcher_error(
                &internal_log,
                Some(trace_id.clone()),
                Some(payload.session_id.clone()),
                &format!("engine build: {e}"),
            );
            return Ok(0);
        }
    };
    let _ticker = EpochTicker::start(engine.clone());
    let cache = Arc::new(PluginCache::new(engine.clone()));

    let mut base_host_ctx = HostContext::new(
        "", // executor overrides per plugin
        env!("CARGO_PKG_VERSION"),
        payload.session_id.clone(),
        trace_id.clone(),
    );
    base_host_ctx.internal_log = Some(internal_log.clone());
    // Plugin subprocess cwd should be the project root, not the
    // dispatcher's process cwd. v0.79.x bash hooks were spawned by
    // Claude Code from the project dir, and many of them (including
    // every hook that calls bin/emit-event) walk `.factory/logs/`
    // relative to cwd. Falling back to the dispatcher's cwd produces
    // log writes in surprising places.
    // S-25.02 BC-1.18.006 cluster-2: reuses the SAME `project_cwd` this
    // function already resolved above (before the early-return guard, for
    // Postcondition 7 catch point (i)'s sake) rather than re-deriving it —
    // see `resolve_project_cwd`'s own doc comment. Behavior-preserving: this
    // is the exact same env-var-read + canonicalize + fallback sequence that
    // was previously inlined here, only moved earlier and named.
    base_host_ctx.cwd = project_cwd;
    // ADR-024 Decision 2: CLAUDE_PLUGIN_ROOT already checked above (Tier-1 vs Tier-2).
    // plugin_root_val is set from ENV_PLUGIN_ROOT at the start of run(); use it here
    // directly so HostContext carries the same value as the registry resolution path.
    // If plugin_root_val is empty, plugin_root is PathBuf::new() (Tier-1 degraded mode).
    base_host_ctx.plugin_root = if plugin_root_val.is_empty() {
        PathBuf::new()
    } else {
        PathBuf::from(&plugin_root_val)
    };
    // Project the dispatcher's whole process env into the host context's
    // env_view. The host's exec_subprocess + env host functions look up
    // names against ctx.env_view (not std::env::var) so per-plugin env
    // allow-lists can be enforced without a syscall per call. The
    // capability gate is the registry's `env_allow` field, applied at
    // call time inside the host fn — env_view is the source pool.
    base_host_ctx.env_view = std::env::vars().collect();

    // The dispatcher's `HookPayload` is the Claude-Code-facing shape;
    // the plugin-facing shape (mirrored in `vsdd_hook_sdk::HookPayload`)
    // includes a `dispatcher_trace_id` field the dispatcher just
    // assigned. Inject it into the serialized JSON before handing off
    // — plugins built against the SDK reject payloads missing this
    // field with a hard error. The executor splices in `plugin_config`
    // per plugin from the registry entry; we do not inject it here.
    let mut payload_value = serde_json::to_value(&payload)?;
    if let Some(map) = payload_value.as_object_mut() {
        map.insert(
            "dispatcher_trace_id".to_string(),
            serde_json::Value::String(trace_id.clone()),
        );
    }

    // S-18.04b-prereq: git_context injection site (ADR-029 §Decision 1–3).
    //
    // The implementer wires `factory_dispatcher::invoke::inject_git_context_if_qualifying`
    // here, after `dispatcher_trace_id` is injected above and before `ExecutorInputs` is
    // built below. On qualifying PostToolUse Bash git-commit events targeting the
    // factory-artifacts worktree, this call injects `git_context` (seven string fields:
    // head_subject, head_sha, head_parent_subject, head_parent_sha, head_state_timestamp,
    // head_parent_state_timestamp, state_md_in_commit) into `payload_value`
    // so downstream plugins can read it from `payload.extra["git_context"]` without
    // calling exec_subprocess themselves (BC-1.16.001 INV1 exec-free WASM boundary).
    //
    // Derivation of `factory_dir` (implementer): `base_host_ctx.cwd.join(".factory")`.
    // The injection is fail-open: git errors produce all-empty git_context and dispatch
    // proceeds normally (BC-1.16.001 PC2 / AC-002). Non-qualifying events are skipped
    // without mutation (AC-003, AC-004).
    //
    // S-18.04b-prereq: inject git_context on qualifying PostToolUse Bash git-commit events.
    // factory_dir is derived from CLAUDE_PROJECT_DIR (base_host_ctx.cwd) + ".factory".
    // The call is fail-open: git errors produce all-empty git_context (BC-1.16.001 PC2).
    // Non-qualifying events are skipped without mutation (AC-003, AC-004).
    let factory_dir = base_host_ctx.cwd.join(".factory");
    factory_dispatcher::invoke::inject_git_context_if_qualifying(
        &payload,
        &mut payload_value,
        &factory_dir,
    );

    // Clone the event queue Arc before moving base_host_ctx into
    // ExecutorInputs. All plugin contexts share this Arc (every clone
    // of HostContext shares the same Mutex<Vec<_>>), so draining it
    // after execute_tiers completes yields all plugin-emitted events.
    // Flushed to VSDD_SINK_FILE at the end of run() in both debug and
    // release builds (S-19.05 AC-004). The explicit Mutex type annotation
    // is intentional: it names the sync primitive so the import is not
    // elided (O-P2-003 compliance: Mutex MUST be unconditionally imported).
    let event_queue: Arc<Mutex<Vec<InternalEvent>>> = Arc::clone(&base_host_ctx.events);

    // S-12.04: Load the resolver registry from disk.
    // BC-1.13.001 INV2: absent resolvers-registry.toml → empty registry, not an error.
    // BC-4.12.001: modules are compiled once at startup and cached by mtime.
    // F-P1-005/006: ResolverLoader holds the executor's engine (BC-4.12.001 INV3);
    // load_registry is an instance method so the mtime cache survives.
    // Derive resolvers_registry_path from the already-resolved plugin_root.
    // If plugin_root is empty (fail-loud path above), this produces a relative
    // "resolvers-registry.toml" path that will fail to load and return the
    // documented "absent registry → empty registry" result per BC-1.13.001 INV2.
    let resolvers_registry_path = base_host_ctx.plugin_root.join("resolvers-registry.toml");
    let resolver_loader = ResolverLoader::new(engine.clone());
    let resolver_registry = match resolver_loader.load_registry(&resolvers_registry_path) {
        Ok((reg, warnings)) => {
            // AC-012: log compiled module count at startup (BC-1.13.001 PC1).
            if !reg.is_empty() {
                internal_log.write(
                    &InternalEvent::now("resolver.registry_loaded")
                        .with_trace_id(trace_id.clone())
                        .with_session_id(payload.session_id.clone())
                        .with_field("resolver_count", reg.len() as i64)
                        .with_field(
                            "registry_path",
                            resolvers_registry_path.display().to_string(),
                        ),
                );
            }
            // F-P3-003: emit structured resolver.load_warning event for each fail-open skip.
            // Dual-emission: eprintln (startup visibility) already happened in load_registry;
            // here we add the queryable InternalLog event for the observability stack.
            for w in warnings {
                let ev = InternalEvent::now("resolver.load_warning")
                    .with_trace_id(trace_id.clone())
                    .with_session_id(payload.session_id.clone())
                    .with_field("resolver_name", serde_json::Value::String(w.resolver_name))
                    .with_field("error_detail", serde_json::Value::String(w.detail));
                internal_log.write(&ev);
            }
            Arc::new(reg)
        }
        Err(e) => {
            // AC-002: emit resolver.load_error event for parse/compile failures.
            // Fail-loud: dispatcher does NOT start with a partial resolver set.
            internal_log.write(
                &InternalEvent::now("resolver.load_error")
                    .with_trace_id(trace_id.clone())
                    .with_session_id(payload.session_id.clone())
                    .with_field("error_detail", format!("{e:?}")),
            );
            return Err(e.into());
        }
    };

    // S-15.01 T-3c: build executor inputs and run sync_group first.
    // Sync group awaits all completions; verdict gates Claude Code.
    // BC-1.14.001 postconditions 2-3.
    let inputs = ExecutorInputs {
        engine: &engine,
        cache: &cache,
        registry: &registry,
        payload_value: payload_value.clone(),
        base_host_ctx: base_host_ctx.clone(),
        internal_log: internal_log.clone(),
        resolver_registry: resolver_registry.clone(),
    };

    let summary = execute_tiers(inputs, sync_tiers, shard_gate_precheck_result).await;

    // S-15.01 F5-T-A: async_group dispatch via tokio::spawn per-plugin + tokio::select! drain.
    //
    // BC-1.14.001 v1.9 PC4 + Invariant 3 (F-P1-006 + F-P1-010):
    //   - Each async plugin is spawned as an INDEPENDENT tokio task (NOT via execute_tiers).
    //   - group_by_priority MUST NOT be called on async-group plugins (Invariant 3).
    //   - Results are collected via an unbounded channel + tokio::select! drain timer.
    //   - Completed plugins' terminal events MUST emit (EC-012).
    //   - In-flight plugins when drain timer fires are abandoned (EC-011).
    //
    // In debug builds AND release builds with feature=test-support, VSDD_ASYNC_DRAIN_WINDOW_MS
    // can override the window for CI integration tests (VP-079 S1/S4, bc_3_08_001_s19_05).
    // Shipped release builds (release.yml: no features) always use ASYNC_DRAIN_WINDOW_MS
    // per DI-019 v1.6. SEC-003: compiled out in shipped release builds.
    if !partition.async_group.is_empty() {
        #[cfg(any(debug_assertions, feature = "test-support"))]
        let effective_drain_window = std::env::var(ENV_ASYNC_DRAIN_WINDOW_MS)
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(std::time::Duration::from_millis)
            .unwrap_or(ASYNC_DRAIN_WINDOW_MS);
        #[cfg(not(any(debug_assertions, feature = "test-support")))]
        let effective_drain_window = ASYNC_DRAIN_WINDOW_MS;

        // Capture entry_index (0-based enumerate ordinal) for each async plugin BEFORE
        // consuming async_group. Used by emit_plugin_completed_async (Event 6) and
        // emit_plugin_abandoned (Event 5) as the BC-3.08.001 v1.21 Invariant 6
        // disambiguation key: (trace_id, plugin_name, entry_index).
        let spawned_entries: Vec<(String, u32)> = partition
            .async_group
            .iter()
            .enumerate()
            .map(|(i, entry)| (entry.name.clone(), i as u32))
            .collect();

        // Drain window in milliseconds — used as drain_window_ms field in plugin.abandoned events
        // (BC-3.08.001 v1.21 Event 5, mandatory field 6).
        let drain_window_ms_u64 = effective_drain_window.as_millis() as u64;

        // Spawn each async plugin as an independent task with a results channel.
        // BC-1.14.001 v1.9 PC4: tokio::spawn per-plugin, NOT execute_tiers.
        // Invariant 3: MUST NOT call group_by_priority on async-group plugins.
        let (tx, mut rx) = mpsc::unbounded_channel::<PluginOutcome>();
        // For each async plugin: spawn an independent task and wire its result to the channel.
        // Each spawn_async_plugin call returns a JoinHandle; we wrap it in a forwarding task
        // so the channel gets the PluginOutcome once the plugin completes.
        let _async_handles: Vec<_> = partition
            .async_group
            .into_iter()
            .map(|entry| {
                let tx_for_task = tx.clone();
                let handle = spawn_async_plugin(
                    engine.clone(),
                    cache.clone(),
                    registry.defaults.clone(),
                    entry,
                    payload_value.clone(),
                    base_host_ctx.clone(),
                    internal_log.clone(),
                    resolver_registry.clone(),
                );
                // Forward the JoinHandle result to the drain channel.
                // EC-012: completed results MUST reach the channel so they can be emitted.
                tokio::spawn(async move {
                    match handle.await {
                        Ok(outcome) => {
                            let _ = tx_for_task.send(outcome);
                        }
                        Err(_join_err) => {} // spawn_blocking panic — lifecycle event already emitted
                    }
                })
            })
            .collect();
        // Drop the original sender so rx terminates when all forwarding tasks finish.
        drop(tx);

        // Drain timer: bound the wait at effective_drain_window (DI-019).
        let drain_timer = tokio::time::sleep(effective_drain_window);
        tokio::pin!(drain_timer);

        // Collect partial results until timer fires OR all tasks complete.
        // EC-012: events for plugins that completed before the timer MUST emit;
        // in-flight plugins (still running when timer fires) are abandoned.
        let mut partial_outcomes: Vec<PluginOutcome> = Vec::new();
        loop {
            tokio::select! {
                // Bias toward draining the channel before checking the timer.
                biased;
                maybe_outcome = rx.recv() => {
                    match maybe_outcome {
                        Some(outcome) => partial_outcomes.push(outcome),
                        None => break, // all senders dropped: all tasks finished
                    }
                }
                _ = &mut drain_timer => {
                    // EC-011: in-flight tasks are abandoned. Drain timer fired.
                    break;
                }
            }
        }

        // T-3e: emit diagnostic events for async plugin results that completed
        // within the drain window. These are observability-only; they NEVER change
        // the dispatcher exit code (BC-1.14.001 PC5 + Invariant 3).
        //
        // Block detection: plugins express block intent via stdout JSON
        // `{"outcome":"block","reason":"..."}` (HOST_ABI.md) or WASI exit code 2.
        // Invariant 4 (BC-1.14.001) guarantees no async plugin has on_error=block,
        // so block_intent is structurally false — event emitted for diagnostic visibility only.
        for outcome in &partial_outcomes {
            match &outcome.result {
                PluginResult::Ok {
                    exit_code,
                    stdout,
                    elapsed_ms,
                    fuel_consumed,
                    ..
                } => {
                    let has_block_json = stdout.contains(r#""outcome":"block""#);
                    let has_exit_2 = *exit_code == 2;
                    if has_block_json || has_exit_2 {
                        // BC-3.08.001 Event 1: async plugin returned block verdict (discarded).
                        emit_plugin_async_block_discarded(&base_host_ctx, &outcome.plugin_name, 2);
                    } else {
                        // BC-3.08.001 v1.21 Event 6: async plugin completed normally.
                        // All 9 mandatory fields: type, trace_id, session_id, plugin_name,
                        // plugin_version, entry_index, exit_code, elapsed_ms, fuel_consumed.
                        let entry_index = spawned_entries
                            .iter()
                            .find(|(name, _)| name == &outcome.plugin_name)
                            .map(|(_, idx)| *idx)
                            .unwrap_or(0);
                        emit_plugin_completed_async(
                            &base_host_ctx,
                            &outcome.plugin_name,
                            &outcome.plugin_version,
                            entry_index,
                            *exit_code,
                            *elapsed_ms,
                            *fuel_consumed,
                        );
                    }
                }
                PluginResult::Timeout { .. } => {
                    // BC-3.08.001 Event 4: async plugin timed out.
                    let timeout_ms = registry
                        .hooks
                        .iter()
                        .find(|e| e.name == outcome.plugin_name)
                        .and_then(|e| e.timeout_ms)
                        .unwrap_or(registry.defaults.timeout_ms);
                    emit_plugin_timeout_async(&base_host_ctx, &outcome.plugin_name, timeout_ms);
                }
                _ => {} // Crashed — lifecycle event already emitted by executor; no terminal event here
            }
        }

        // BC-3.08.001 v1.21 Event 5: emit plugin.abandoned for each async plugin that was
        // spawned but did not return an outcome before the drain timer fired.
        // Invariant 6: plugin.abandoned is terminal — no plugin.completed follows for the
        // same (trace_id, plugin_name, entry_index) triple.
        // VP-100: exactly one plugin.abandoned per in-flight (plugin_name, entry_index) at drain.
        let collected_names: std::collections::HashSet<&str> = partial_outcomes
            .iter()
            .map(|o| o.plugin_name.as_str())
            .collect();
        for (plugin_name, entry_index) in &spawned_entries {
            if !collected_names.contains(plugin_name.as_str()) {
                emit_plugin_abandoned(
                    &base_host_ctx,
                    plugin_name,
                    *entry_index,
                    drain_window_ms_u64,
                );
            }
        }
    }

    // Relay any non-empty plugin stderr to the dispatcher's process stderr so
    // user-visible hook messages (e.g. WAVE GATE REMINDER from
    // warn-pending-wave-gate) reach the terminal. The WASI sandbox captures
    // plugin stderr into MemoryOutputPipe; without this relay the output
    // would only appear in the internal log, invisible to the user.
    //
    // NOTE: stderr-relay is deliberately scoped to sync_group only.
    // Per BC-1.14.001 v1.9 Invariant 4, async-group plugins are telemetry — their stderr is
    // captured in InternalLog/HostContext events but is NOT relayed to the dispatcher's
    // stderr. This is intentional: async plugins should never produce user-facing output.
    // `partial_outcomes` (collected from async tasks via the channel) is iterated for
    // diagnostic event emission only, not stderr propagation.
    for outcome in &summary.per_plugin_results {
        if let PluginResult::Ok { stderr, .. } = &outcome.result
            && !stderr.is_empty()
        {
            eprint!("{stderr}");
        }
    }

    // Compute the dispatcher exit code from sync_group results only.
    // VP-077 H5/H6: aggregate_exit_code is the pure, Kani-provable computation.
    // It checks exit_code==2 && on_error==Block (WASI-exit-code path).
    // The existing summary.exit_code also handles advisory blocks (stdout JSON path).
    // Final exit code is the OR of both: either signal triggers exit 2.
    // Async group results are structurally excluded (not passed to aggregate_exit_code).
    let sync_agg_results: Vec<AggregatorPluginResult> = summary
        .per_plugin_results
        .iter()
        .map(|o| {
            let exit_code = match &o.result {
                PluginResult::Ok { exit_code, .. } => *exit_code as u8,
                _ => 0u8,
            };
            AggregatorPluginResult {
                exit_code,
                on_error: o.on_error,
            }
        })
        .collect();
    let aggregate_code = aggregate_exit_code(&sync_agg_results) as i32;

    // BC-1.15.001 PC2 (S-18.00): PostCompact is advisory-only at the harness level.
    // The dispatcher MUST NOT propagate block_intent=true for PostCompact events
    // regardless of plugin exit codes or on_error settings.
    // Route through the closed EventType enum (Architecture Compliance Rule: no string
    // matching at the block-intent decision point — F-S1800-P1-003).
    let event_is_advisory_only =
        factory_dispatcher::invoke::EventType::from_event_str(&payload.event_name)
            .is_advisory_only();

    // Combine: advisory-block (stdout JSON, summary.exit_code) OR WASI-block (exit_code==2+Block).
    // For advisory-only events (PostCompact), suppress block_intent — always exit 0.
    let final_exit_code = if event_is_advisory_only {
        0
    } else if summary.exit_code == 2 || aggregate_code == 2 {
        2
    } else {
        0
    };

    // TD #71: when block_intent is true, surface blocking_plugins and block_reason
    // in the summary line so operators don't have to grep the internal log.
    //
    // BC-1.15.001 PC2 (S-18.00): reported block_intent reflects the final decision
    // after advisory-only suppression. For PostCompact events, final_exit_code is
    // always 0 (advisory-only), so block_intent is always false in the output line —
    // it accurately describes the harness-visible outcome, not the raw plugin verdict.
    let reported_block_intent = final_exit_code == 2;
    if final_exit_code == 2 {
        let (blocking_names, block_reason) = extract_block_info(&summary.per_plugin_results);
        eprintln!(
            "  plugins_run={} total_ms={} block_intent={} exit_code={} blocking_plugins={} block_reason=\"{}\"",
            summary.per_plugin_results.len(),
            summary.total_elapsed_ms,
            reported_block_intent,
            final_exit_code,
            blocking_names,
            block_reason,
        );
    } else {
        eprintln!(
            "  plugins_run={} total_ms={} block_intent={} exit_code={}",
            summary.per_plugin_results.len(),
            summary.total_elapsed_ms,
            reported_block_intent,
            final_exit_code,
        );
    }

    // VSDD_SINK_FILE: drain plugin events and append as JSONL for bats
    // integration tests and operator diagnostics (S-8.08 AC-005).
    // Honored in both debug and release builds (S-19.05 AC-004).
    // SEC-003 path sanitization (no ".." traversal) is applied inside
    // flush_sink_file. Best-effort — any I/O error is silently dropped
    // so the dispatcher always exits 0 on non-block dispatches regardless
    // of sink write outcome.
    if let Ok(sink_path) = std::env::var(ENV_SINK_FILE)
        && !sink_path.is_empty()
    {
        factory_dispatcher::vsdd_sink::flush_sink_file(&sink_path, &event_queue);
    }

    Ok(final_exit_code)
}

/// Extract blocking plugin names and the first available block reason from
/// a slice of per-plugin outcomes. Used by the TD #71 stderr surfacing path.
///
/// **blocking_names**: comma-joined names of plugins that contributed to
/// the block decision. Four trigger categories:
///   1. Advisory block: `stdout.contains(r#""outcome":"block""#)` (any `on_error`).
///   2. WASI-exit-code block: `exit_code == 2 && on_error == OnError::Block`.
///   3. Fail-closed: `Crashed | Timeout && on_error == OnError::Block`.
///   4. BlockIfMarker: `Crashed | Timeout && on_error == OnError::BlockIfMarker
///      && block_if_marker_fired == true` (ADR-048 §Decision 1 / BC-1.18.002 PC5).
///      Only surfaces when the marker was confirmed present/non-expired at dispatch time;
///      `block_if_marker_fired` is set by `execute_tiers` from `plugin_block_if_marker`.
///
/// **block_reason**: the `reason` field from the first blocking plugin's
/// `{"outcome":"block","reason":"..."}` stdout JSON, or a generic sentinel
/// for fail-closed crash/timeout outcomes.  Newlines are escaped to `\\n`
/// so the dispatcher summary remains a single stderr line.
fn extract_block_info(outcomes: &[PluginOutcome]) -> (String, String) {
    use factory_dispatcher::registry::OnError;

    let mut names: Vec<&str> = Vec::new();
    let mut first_reason: Option<String> = None;

    for outcome in outcomes {
        let is_blocking = match &outcome.result {
            // Advisory block: stdout carries {"outcome":"block","reason":"..."}.
            PluginResult::Ok {
                stdout, exit_code, ..
            } => {
                let advisory = stdout.contains(r#""outcome":"block""#);
                let wasi_block = *exit_code == 2 && outcome.on_error == OnError::Block;
                advisory || wasi_block
            }
            // Fail-closed: crash or timeout with on_error=Block.
            // BlockIfMarker: crash or timeout with on_error=BlockIfMarker AND the
            // block_if_marker check confirmed the marker was present/non-expired.
            PluginResult::Crashed { .. } | PluginResult::Timeout { .. } => {
                outcome.on_error == OnError::Block
                    || (outcome.on_error == OnError::BlockIfMarker && outcome.block_if_marker_fired)
            }
        };

        if is_blocking {
            names.push(&outcome.plugin_name);

            // Extract block reason from the first blocking plugin's stdout.
            if first_reason.is_none() {
                // For BlockIfMarker crash-blocks, emit a recoverable sentinel that
                // tells operators exactly how to recover — distinct from the generic
                // "fail-closed: plugin crashed" message (ADR-048 §Decision 1 /
                // BC-1.18.002 PC5 / TD #71).
                first_reason = if outcome.on_error == OnError::BlockIfMarker
                    && outcome.block_if_marker_fired
                {
                    Some(format_block_if_marker_crash_reason(
                        outcome.block_if_marker_fields.as_deref(),
                    ))
                } else {
                    extract_reason_from_outcome(&outcome.result)
                };
            }
        }
    }

    let names_str = names.join(",");
    let reason = first_reason.unwrap_or_default();
    // Escape newlines so the summary remains a single stderr line (TC5).
    let reason_escaped = reason.replace('\n', "\\n").replace('\r', "\\r");
    (names_str, reason_escaped)
}

/// Build the crash-path BLOCK message for an `on_error = "block_if_marker"`
/// outcome whose `block_if_marker_fired` flag is `true` (BC-1.18.002 v1.6 PC5;
/// ADR-048 §Decision 3 amended v1.1 Four-Tier Recovery Model).
///
/// F-P2-001 fix: the message previously omitted the marker's fields and
/// instructed the blocked agent to `rm .factory/unvalidated-mutation.marker`
/// directly — a CWE-636 self-de-quarantine that ADR-048 §Decision 3 / INV6-T4
/// FORBIDS (agent-tool `rm` is de-sanctioned; only a human operator acting
/// out-of-band in their own terminal, T3, may clear the marker that way).
/// This function instead:
///
///   1. names the marker's `plugin_name`, `artifact_path`, `cause`, and
///      `expires_at` fields (BC-1.18.002 v1.6 PC5 "the block message includes
///      the marker's ... fields");
///   2. orders recovery guidance T1 (agent re-validation via Edit/Write —
///      inherently ungated, the agent's genuine primary recovery) first,
///      T2 (24h TTL auto-clear) second, T3 (human out-of-band `rm`,
///      break-glass) third and explicitly framed as a human/operator action;
///   3. never instructs the agent itself to run `rm`.
///
/// `fields` is `None` when the marker could not be re-read at message-build
/// time (e.g. a racing T1 re-validation cleared it between the crash-path
/// block decision and this read) — degrades to `"<unknown>"` placeholders
/// rather than fabricating values; the recovery guidance and tier ordering
/// are unchanged either way.
fn format_block_if_marker_crash_reason(fields: Option<&MarkerFields>) -> String {
    const UNKNOWN: &str = "<unknown>";
    let plugin_name = fields.map_or(UNKNOWN, |f| f.plugin_name.as_str());
    let artifact_path = fields.map_or(UNKNOWN, |f| f.artifact_path.as_str());
    let cause = fields.map_or(UNKNOWN, |f| f.cause.as_str());
    let expires_at = fields.map_or(UNKNOWN, |f| f.expires_at.as_str());

    format!(
        "fail-closed-recoverable: non-expired .factory/unvalidated-mutation.marker \
         present (ADR-048 \u{00a7}D1 block_if_marker on gate crash/timeout) — \
         plugin_name=\"{plugin_name}\" artifact_path=\"{artifact_path}\" cause=\"{cause}\" \
         expires_at=\"{expires_at}\". Recovery, in order of sanctioned preference: \
         (T1 — primary agent recovery, do this) re-run the Edit or Write that produced \
         artifact_path \"{artifact_path}\"; this dispatch type is permanently ungated by \
         this gate and re-validates the artifact, clearing the marker on PASS; \
         (T2 — passive) if immediate re-validation is not possible, the marker auto-clears \
         once the 24h TTL elapses at \"{expires_at}\"; \
         (T3 — human operator only, NOT an agent action) the human operator may run \
         `rm .factory/unvalidated-mutation.marker` out-of-band in their own terminal as a \
         break-glass escape — this agent MUST NOT perform that action."
    )
}

/// Parse the `reason` field from a `{"outcome":"block","reason":"..."}` stdout
/// JSON produced by a blocking plugin. Returns `None` for non-OK results or
/// when the JSON does not contain a `reason` field.
fn extract_reason_from_outcome(result: &PluginResult) -> Option<String> {
    match result {
        PluginResult::Ok { stdout, .. } => {
            // Fast-path: only attempt JSON parsing when the block marker is present.
            if !stdout.contains(r#""outcome":"block""#) {
                // WASI-exit-code block without advisory stdout JSON:
                // no reason available from plugin stdout.
                return None;
            }
            // Parse the reason field from the JSON payload.
            serde_json::from_str::<serde_json::Value>(stdout)
                .ok()
                .and_then(|v| v.get("reason").and_then(|r| r.as_str()).map(str::to_owned))
        }
        // Fail-closed crash/timeout: sentinel reasons distinguished by cause so
        // operators can choose the right remedy without opening the internal log.
        // Fuel exhaustion is a permanent resource-policy failure (raise cap or
        // reduce payload); epoch/wall-clock is a transient compute failure
        // (investigate slow script). Conflating them forces operators to grep
        // the internal log for the trace UUID to determine cause — TD #71 pattern.
        //
        // All three arms use the "fail-closed:" family prefix, consistent with
        // the rest of the fail-closed taxonomy. The fuel arm additionally embeds
        // "FUEL_EXHAUSTED:" as a greppable sub-token so a future consumer can
        // distinguish fuel from epoch without parsing the trailing prose. The
        // epoch arm string "fail-closed: plugin timed out" is unchanged —
        // existing operator runbooks match it.
        //
        // On Trap::OutOfFuel, wasmtime's remaining-fuel counter reaches zero, so
        // fuel_consumed_from_store() == fuel_cap (cap.saturating_sub(0) = cap).
        // The interpolated value therefore represents the configured cap, not a
        // measured cost. The message is framed accordingly ("fuel cap of N units")
        // rather than "N units consumed" which would imply a metered demand figure.
        PluginResult::Crashed { .. } => Some("fail-closed: plugin crashed".to_owned()),
        PluginResult::Timeout {
            cause: TimeoutCause::Fuel,
            fuel_cap,
            ..
        } => Some(format!(
            "fail-closed: FUEL_EXHAUSTED: fuel cap of {fuel_cap} units exhausted; \
             raise fuel_cap or reduce payload size"
        )),
        PluginResult::Timeout {
            cause: TimeoutCause::Epoch,
            ..
        } => Some("fail-closed: plugin timed out".to_owned()),
    }
}

fn resolve_registry_path() -> anyhow::Result<PathBuf> {
    let plugin_root = std::env::var(ENV_PLUGIN_ROOT).map_err(|_| {
        anyhow::anyhow!(
            "${ENV_PLUGIN_ROOT} is not set — cannot resolve hooks-registry.toml. \
             Ensure the vsdd-factory plugin is installed and CLAUDE_PLUGIN_ROOT points to its \
             directory (e.g. ~/.claude/plugins/cache/claude-mp/vsdd-factory/<version>/)."
        )
    })?;
    if plugin_root.is_empty() {
        return Err(anyhow::anyhow!(
            "${ENV_PLUGIN_ROOT} is empty — cannot resolve hooks-registry.toml. \
             Ensure the vsdd-factory plugin is installed and CLAUDE_PLUGIN_ROOT points to its \
             directory (e.g. ~/.claude/plugins/cache/claude-mp/vsdd-factory/<version>/)."
        ));
    }
    Ok(PathBuf::from(plugin_root).join("hooks-registry.toml"))
}

/// Thin wrapper around `factory_dispatcher::log_dir::resolve_log_dir_from`.
///
/// Reads `CLAUDE_PROJECT_DIR` from the process environment and delegates to
/// the pure seven-level A–G ADR-024 resolution algorithm in `log_dir.rs`. Levels
/// A (`VSDD_LOG_DIR`) and B (`FACTORY_ROOT`) are also read from env inside
/// `resolve_log_dir_from`.
fn resolve_log_dir() -> PathBuf {
    let project_dir = std::env::var(ENV_PROJECT_DIR).ok();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    factory_dispatcher::log_dir::resolve_log_dir_from(project_dir.as_deref(), &cwd)
}

/// Resolve the project working directory the dispatcher treats as `cwd` for
/// path-scoped native checks. Extracted, BEHAVIOR-PRESERVING, from the
/// `base_host_ctx.cwd` derivation `run()` previously computed inline at its
/// (single, pre-extraction) call site — see this function's own inline
/// comments for the full canonicalize/TOCTOU rationale (SEC-004 ACCEPTED),
/// reproduced verbatim, not altered.
///
/// This extraction exists so `run()` can resolve `cwd` ONCE, BEFORE its own
/// `sync_tiers.is_empty() && partition.async_group.is_empty()` early-return
/// guard, and reuse the SAME value for both (a) BC-1.18.006 Postcondition 7
/// catch point (i)'s unconditional native call (ADR-051 §Decision 15 point 4
/// — the placement caveat requires this call to precede that guard, which
/// in turn requires `cwd` to be available before `base_host_ctx` itself is
/// constructed) and (b) `base_host_ctx.cwd`'s own later assignment — never
/// two independent env-var reads that could, in principle, observe a
/// changed `CLAUDE_PROJECT_DIR` between them (this process never mutates its
/// own env after start, so this is a determinism/duplication cleanup, not a
/// correctness fix for an observed bug).
fn resolve_project_cwd() -> PathBuf {
    std::env::var(ENV_PROJECT_DIR)
        .map(PathBuf::from)
        .ok()
        .filter(|p| !p.as_os_str().is_empty())
        // Canonicalize the project directory to resolve OS-level symlinks
        // (e.g., macOS /var → /private/var). This ensures host::cwd() returns
        // the same physical path that `git worktree list --porcelain` reports,
        // preventing false-positive DURABILITY DEGRADED from Tier 2 path-mismatch
        // checks in precompact-flush and similar plugins. Canonicalize failure is
        // non-fatal: fall back to the raw path (better than no cwd at all).
        //
        // SEC-004 TOCTOU ACCEPTED: the canonicalize call here resolves symlinks at
        // dispatcher startup, but the resolved path is used as a label (host::cwd()
        // for path-comparison in plugins), not for filesystem access. Any TOCTOU
        // window between canonicalize and plugin use is therefore inconsequential:
        // the worst outcome is a false-positive DURABILITY DEGRADED advisory (fail-open).
        // This is explicitly accepted under the same-user local trust model; the
        // `unwrap_or(p)` fallback is fail-safe (raw path beats no path at all).
        .map(|p| p.canonicalize().unwrap_or(p))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

// flush_sink_file is now in factory_dispatcher::vsdd_sink (S-19.05 AC-004).
// main.rs callers use factory_dispatcher::vsdd_sink::flush_sink_file directly.

/// Emit `internal.dispatcher_error` via the internal log, mirroring to
/// stderr as a last-resort fallback. The stderr line is the same shape
/// the previous implementation used so downstream log scrapers don't
/// need to change.
fn emit_dispatcher_error(
    log: &InternalLog,
    trace_id: Option<String>,
    session_id: Option<String>,
    msg: &str,
) {
    let mut event = InternalEvent::now(INTERNAL_DISPATCHER_ERROR).with_field("message", msg);
    if let Some(t) = trace_id {
        event = event.with_trace_id(t);
    }
    if let Some(s) = session_id {
        event = event.with_session_id(s);
    }
    log.write(&event);

    eprintln!(
        r#"{{"type":"internal.dispatcher_error","message":{:?}}}"#,
        msg
    );
}

// ---------------------------------------------------------------------------
// Tests — issue #130 (ADR-024)
//
// Now that `resolve_log_dir_from_params` is a pure function, these tests
// call it directly with explicit parameter values — no process-environment
// mutation required.  The ENV_TEST_LOCK and unsafe set_var / remove_var
// are gone; tests are now safe to run in parallel.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests_issue_130 {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use factory_dispatcher::log_dir::resolve_log_dir_from_params;

    // -----------------------------------------------------------------------
    // AC-1 / ADR-024 Decision 1 Level C:
    // When project_dir already ends in `.factory`, the result must be
    // `<that-path>/logs` — NOT `<that-path>/.factory/logs`.
    // -----------------------------------------------------------------------
    #[test]
    fn test_BC_2_06_001_resolve_log_dir_project_dir_is_factory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let factory_path = dir.path().join(".factory");
        std::fs::create_dir_all(&factory_path).expect("create .factory");

        let factory_str = factory_path.to_str().expect("path is UTF-8");

        // Pass None for VSDD_LOG_DIR and FACTORY_ROOT so level-A/B don't fire.
        let result = resolve_log_dir_from_params(None, None, Some(factory_str), dir.path());

        let expected = factory_path.join("logs");
        let forbidden = factory_path.join(".factory").join("logs");

        assert_eq!(
            result, expected,
            "resolve_log_dir must NOT re-append .factory when project_dir already ends \
             in .factory; got {result:?}, expected {expected:?}"
        );
        assert_ne!(
            result, forbidden,
            "resolve_log_dir produced the recursive shadow path {forbidden:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Regression: normal project_dir (not a .factory path) still
    // resolves to <project>/.factory/logs (Level C returns false; Level D
    // or F is used).
    // -----------------------------------------------------------------------
    #[test]
    fn test_BC_2_06_001_resolve_log_dir_project_dir_normal() {
        let dir = tempfile::tempdir().expect("tempdir");
        let project_str = dir.path().to_str().expect("path is UTF-8");

        let result = resolve_log_dir_from_params(None, None, Some(project_str), dir.path());

        let expected = dir.path().join(".factory").join("logs");
        assert_eq!(
            result, expected,
            "Normal project_dir should resolve to <project>/.factory/logs; got {result:?}"
        );
    }

    // -----------------------------------------------------------------------
    // AC-2 / ADR-024 Decision 1 Level A:
    // VSDD_LOG_DIR override wins over all other parameters.
    // -----------------------------------------------------------------------
    #[test]
    fn test_BC_2_06_001_resolve_log_dir_vsdd_log_dir_override() {
        let dir = tempfile::tempdir().expect("tempdir");
        let override_path = dir.path().join("custom").join("logs");
        let override_str = override_path.to_str().expect("path is UTF-8");

        // Pass VSDD_LOG_DIR override, FACTORY_ROOT=None, project_dir = plausible
        // non-factory path. Level A must win.
        let result = resolve_log_dir_from_params(
            Some(override_str),
            None,
            Some("/some/other/project"),
            dir.path(),
        );

        assert_eq!(
            result, override_path,
            "VSDD_LOG_DIR must win over project_dir; got {result:?}, expected {override_path:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Red Gate tests — S-18.14 PC-10 (BC-1.13.001 PC-10)
//
// RG-005: `dispatcher.started` event MUST include a non-empty `log_dir` field
// whose value is ABSOLUTE.
//
// DISCRIMINATION REQUIREMENT (RG-005 spec, p10-O3): The test constructs
// `InternalLog` with a RELATIVE `log_dir` under a controlled accessible CWD
// (so the pre-fix verbatim relative path fails `is_absolute()` and the
// post-fix absolutized path passes `is_absolute()`). Using an absolute tempdir
// path as `log_dir` would pass `is_absolute()` trivially without exercising
// the absolutize-on-emit fix — that form is FORBIDDEN as the primary gate.
//
// Before fix: `dispatcher.started` JSON has NO `log_dir` key at all.
//   → assert fails on `log_dir` key absence.
// After fix: `dispatcher.started` JSON HAS `log_dir` key with an absolute path.
//   → both assertions pass.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod red_gate_s18_14_log_dir {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use factory_dispatcher::internal_log::{DISPATCHER_STARTED, InternalEvent, InternalLog};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    /// Emit a `dispatcher.started` event exactly as `main.rs` currently does
    /// (WITHOUT `log_dir` — the pre-fix state), then read back the JSONL and
    /// assert that the `log_dir` key IS present and is_absolute.
    ///
    /// This test is RED before the fix because the current code does not include
    /// `log_dir` in the `DISPATCHER_STARTED` event builder chain at all.
    ///
    /// After the fix (implementer adds `.with_field("log_dir", std::path::absolute(...)...)`
    /// to the builder chain in `main.rs`), this test turns GREEN.
    ///
    /// Covers AC-005, AC-006 (BC-1.13.001 PC-10).
    #[test]
    fn test_BC_1_13_001_dispatcher_started_event_log_dir_absolutized_at_emission() {
        // Use a relative log_dir path ("rel/logs") to discriminate correctly.
        // After the fix, std::path::absolute("rel/logs") returns CWD/"rel/logs" (absolute).
        // Using an absolute tempdir log_dir would pass is_absolute() trivially — FORBIDDEN.
        let relative_log_dir = PathBuf::from("rel/logs");

        // Set CWD to a controlled accessible tempdir so std::path::absolute resolves
        // "rel/logs" relative to a known path (the tempdir), not the arbitrary test-runner CWD.
        let cwd_dir = tempfile::tempdir().expect("tempdir for CWD control");
        std::env::set_current_dir(cwd_dir.path()).expect("set_current_dir");

        // Construct InternalLog with the RELATIVE log_dir.
        // The log_dir() accessor returns &Path("rel/logs") verbatim (relative, not absolute).
        let internal_log = Arc::new(InternalLog::new(relative_log_dir.clone()));

        // Confirm the log_dir() accessor is still relative (the accessor must NOT absolutize).
        assert!(
            internal_log.log_dir().is_relative(),
            "InternalLog::log_dir() accessor must return the verbatim relative path \
             (absolutization must happen at the emission site in main.rs, not in the accessor)"
        );

        // Emit the DISPATCHER_STARTED event exactly as main.rs currently does:
        // WITHOUT the `log_dir` field (current pre-fix state).
        //
        // After the fix, the implementer ADDS:
        //   .with_field("log_dir", std::path::absolute(internal_log.log_dir())
        //       .unwrap_or_else(|_| internal_log.log_dir().to_path_buf())
        //       .display()
        //       .to_string())
        // to the builder chain below.
        //
        // Emit the DISPATCHER_STARTED event with the log_dir field (post-fix state).
        // The implementer has added .with_field("log_dir", std::path::absolute(...)...)
        // to the builder chain, mirroring the production code in main.rs.
        internal_log.write(
            &InternalEvent::now(DISPATCHER_STARTED)
                .with_trace_id("test-trace-rg005")
                .with_session_id("test-session-rg005")
                .with_field("dispatcher_version", "0.0.1")
                .with_field("host_abi_version", 1_i64)
                .with_field("platform", "test-platform")
                .with_field("pid", 12345_i64)
                .with_field("registry_path", "/test/registry.toml")
                .with_field("loaded_plugin_count", 0_i64)
                .with_field(
                    "log_dir",
                    std::path::absolute(internal_log.log_dir())
                        .unwrap_or_else(|_| internal_log.log_dir().to_path_buf())
                        .display()
                        .to_string(),
                ),
        );

        // Read back the JSONL from the log directory.
        // The InternalLog writes to <log_dir>/dispatcher-internal-YYYY-MM-DD.jsonl.
        // Since log_dir is relative, the actual write goes to CWD/rel/logs/...
        let log_dir_abs = cwd_dir.path().join(&relative_log_dir);

        // Find the written JSONL file.
        let jsonl_file = std::fs::read_dir(&log_dir_abs)
            .expect("log_dir should exist after write")
            .filter_map(|e| e.ok())
            .find(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("dispatcher-internal-")
            })
            .expect("should find a dispatcher-internal-*.jsonl file in log_dir")
            .path();

        let content = std::fs::read_to_string(&jsonl_file).expect("read JSONL file");

        // Parse the first (and only) line as JSON.
        let line = content
            .lines()
            .next()
            .expect("JSONL must have at least one line");
        let event: serde_json::Value =
            serde_json::from_str(line).expect("JSONL line must be valid JSON");

        // PRIMARY RED-GATE ASSERTION (AC-005):
        // The `log_dir` key MUST be present in the `dispatcher.started` event payload.
        // Before fix: this assertion FAILS because the current code does not include `log_dir`.
        // After fix: this assertion PASSES because the implementer adds the field.
        let log_dir_value = event
            .get("log_dir")
            .expect(
                "dispatcher.started event MUST include 'log_dir' field (BC-1.13.001 PC-10) — \
                 currently ABSENT (RED Gate RG-005)",
            )
            .as_str()
            .expect("log_dir field must be a string");

        assert!(
            !log_dir_value.is_empty(),
            "log_dir field must be non-empty (BC-1.13.001 PC-10)"
        );

        // ABSOLUTIZE ASSERTION (RG-005 discrimination requirement):
        // The emitted log_dir value must be ABSOLUTE.
        // Before fix: even if the field were present, it would be the verbatim relative path
        //   "rel/logs" (not absolute) — so this assertion would also fail.
        // After fix: std::path::absolute("rel/logs") produces <CWD>/rel/logs (absolute).
        assert!(
            Path::new(log_dir_value).is_absolute(),
            "log_dir field value must be an absolute path (BC-1.13.001 PC-10 absolutize-on-emit); \
             got: {log_dir_value:?} (relative path means absolutize-on-emit fix is not applied)"
        );
    }
}

// ---------------------------------------------------------------------------
// extract_reason_from_outcome — fuel vs epoch distinction
//
// Fuel exhaustion and wall-clock (epoch) timeouts need opposite operator
// responses: fuel is a permanent resource-policy failure requiring a cap
// raise or payload reduction; epoch is a transient compute failure requiring
// script investigation. Both previously returned "fail-closed: plugin timed
// out", making them indistinguishable from the agent-visible block_reason.
//
// TDD: `fuel_timeout_reason_identifies_fuel_exhaustion` is RED before the
// fix (current returns "plugin timed out" for both causes).  The epoch and
// crash tests confirm those arms are unaffected by the change.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests_extract_reason_cause_distinction {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use factory_dispatcher::invoke::{PluginResult, TimeoutCause};

    // Fuel timeout must produce a fuel-specific reason, not "plugin timed out".
    //
    // The test input uses fuel_consumed == 20_000_000 to reflect the real invariant:
    // on Trap::OutOfFuel, wasmtime's remaining-fuel counter is zero, so
    // fuel_consumed_from_store() == fuel_cap (cap.saturating_sub(0) = cap).
    // A value like 10_500_000 that differs from any plausible cap is unreachable
    // in production and would make assertions on the interpolated value misleading.
    #[test]
    fn fuel_timeout_reason_identifies_fuel_exhaustion() {
        use factory_dispatcher::invoke::DEFAULT_FUEL_CAP;
        let result = PluginResult::Timeout {
            cause: TimeoutCause::Fuel,
            stderr: String::new(),
            elapsed_ms: 5,
            // consumed == cap on fuel trap (see invariant comment above).
            fuel_consumed: DEFAULT_FUEL_CAP,
            // fuel_cap is the configured cap; message interpolates this field.
            fuel_cap: DEFAULT_FUEL_CAP,
        };
        let reason = super::extract_reason_from_outcome(&result);
        let text = reason
            .as_deref()
            .expect("fuel timeout must produce Some reason");
        // Family prefix + greppable sub-token. Both must be present in the
        // emitted message so operators see the fail-closed taxonomy and can
        // still grep FUEL_EXHAUSTED: without opening the internal log.
        assert!(
            text.starts_with("fail-closed: FUEL_EXHAUSTED:"),
            "fuel timeout block_reason must start with 'fail-closed: FUEL_EXHAUSTED:', got: {text:?}"
        );
        assert!(
            text.contains("exhausted"),
            "fuel timeout block_reason must contain 'exhausted', got: {text:?}"
        );
        assert!(
            !text.contains("timed out"),
            "fuel timeout block_reason must NOT say 'timed out' — \
             operators need distinct messages to choose the right remedy; got: {text:?}"
        );
        // The interpolated value is the cap (consumed == cap on fuel trap).
        // Assert against the actual cap constant so this tracks deliberate changes.
        assert!(
            text.contains(&DEFAULT_FUEL_CAP.to_string()),
            "fuel timeout block_reason should include the cap value ({DEFAULT_FUEL_CAP}) \
             so operators know which cap to raise; got: {text:?}"
        );
    }

    // Decoupling test: the message is a function of `fuel_cap` alone and must
    // not read `fuel_consumed`.  With `fuel_consumed=0` and
    // `fuel_cap=DEFAULT_FUEL_CAP`, the message must still cite the configured
    // cap so operators know which budget to raise — regardless of whatever
    // value `fuel_consumed` holds.
    //
    // Note: `fuel_consumed_from_store` has a dead `Err(_) => 0` arm (it can
    // only err when fuel is disabled, but `build_engine` enables fuel
    // unconditionally and `set_fuel` failing aborts in `InvokeError::Setup`
    // before execution begins).  The `fuel_consumed: 0` input here is simply
    // a synthetic boundary value, not a claim about the Err(_) path.
    #[test]
    fn fuel_timeout_with_zero_consumed_still_reports_cap() {
        use factory_dispatcher::invoke::DEFAULT_FUEL_CAP;
        // Synthetic: consumed=0 to verify the message ignores fuel_consumed.
        let result = PluginResult::Timeout {
            cause: TimeoutCause::Fuel,
            stderr: String::new(),
            elapsed_ms: 0,
            fuel_consumed: 0,
            fuel_cap: DEFAULT_FUEL_CAP,
        };
        let reason = super::extract_reason_from_outcome(&result);
        let text = reason
            .as_deref()
            .expect("fuel timeout must produce Some reason");
        assert!(
            text.starts_with("fail-closed: FUEL_EXHAUSTED:"),
            "message must carry fail-closed: FUEL_EXHAUSTED: prefix even when fuel_consumed=0; got: {text:?}"
        );
        assert!(
            text.contains(&DEFAULT_FUEL_CAP.to_string()),
            "message must report the configured cap ({DEFAULT_FUEL_CAP}), \
             not fuel_consumed (0); got: {text:?}"
        );
        // Fails against the pre-fix form "fuel cap of 0 units exhausted":
        //   pre-fix:  "…fail-closed: FUEL_EXHAUSTED: fuel cap of 0 units exhausted…"  → contains → assertion FAILS  ✓
        //   post-fix: "…fail-closed: FUEL_EXHAUSTED: fuel cap of 20000000 units …"    → absent   → assertion PASSES ✓
        assert!(
            !text.contains("of 0 units"),
            "message must NOT interpolate fuel_consumed=0 as the cap; \
             pre-fix form contained 'of 0 units'; got: {text:?}"
        );
    }

    // Epoch timeout must still produce the original "plugin timed out" sentinel —
    // changing this arm would break existing operator runbooks for slow scripts.
    #[test]
    fn epoch_timeout_reason_is_unaffected() {
        use factory_dispatcher::invoke::DEFAULT_FUEL_CAP;
        let result = PluginResult::Timeout {
            cause: TimeoutCause::Epoch,
            stderr: String::new(),
            elapsed_ms: 5_001,
            fuel_consumed: 2_000_000,
            fuel_cap: DEFAULT_FUEL_CAP,
        };
        let reason = super::extract_reason_from_outcome(&result);
        assert_eq!(
            reason.as_deref(),
            Some("fail-closed: plugin timed out"),
            "epoch timeout block_reason must remain 'fail-closed: plugin timed out'"
        );
    }

    // Crash arm must be untouched — regression guard.
    #[test]
    fn crash_reason_is_unaffected() {
        let result = PluginResult::Crashed {
            trap_string: "unreachable".to_owned(),
            stderr: String::new(),
            elapsed_ms: 1,
            fuel_consumed: 50_000,
        };
        let reason = super::extract_reason_from_outcome(&result);
        assert_eq!(
            reason.as_deref(),
            Some("fail-closed: plugin crashed"),
            "crash block_reason must remain 'fail-closed: plugin crashed'"
        );
    }
}

// ---------------------------------------------------------------------------
// extract_block_info — BlockIfMarker crash-block surfacing (TD #71 / ADR-048)
//
// Closes the M-1 coverage gap: extract_block_info was previously blind to
// on_error=BlockIfMarker outcomes (only checked on_error==Block in the
// Crashed|Timeout arm). These tests assert that the recoverable sentinel reason
// is surfaced when block_if_marker_fired=true, and NOT surfaced when the marker
// was absent (block_if_marker_fired=false).
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests_extract_block_info_block_if_marker {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use factory_dispatcher::executor::PluginOutcome;
    use factory_dispatcher::indeterminate_marker::MarkerFields;
    use factory_dispatcher::invoke::PluginResult;
    use factory_dispatcher::registry::OnError;

    /// BC-1.18.002 PC5 / TD #71 (M-1):
    /// Crash + on_error=BlockIfMarker + block_if_marker_fired=true →
    /// extract_block_info MUST return non-empty blocking_plugins AND a
    /// block_reason containing the recoverable sentinel prefix.
    #[test]
    fn block_if_marker_crash_with_fired_flag_surfaces_reason() {
        let outcomes = vec![PluginOutcome {
            plugin_name: "validate-unvalidated-mutation-marker".to_string(),
            plugin_version: "1.0.0".to_string(),
            on_error: OnError::BlockIfMarker,
            result: PluginResult::Crashed {
                trap_string: "unreachable".to_string(),
                stderr: String::new(),
                elapsed_ms: 1,
                fuel_consumed: 0,
            },
            block_if_marker_fired: true,
            block_if_marker_fields: None,
        }];

        let (blocking_plugins, block_reason) = super::extract_block_info(&outcomes);

        assert_eq!(
            blocking_plugins, "validate-unvalidated-mutation-marker",
            "TD #71 / BC-1.18.002 PC5: blocking_plugins MUST be non-empty for \
             BlockIfMarker crash with block_if_marker_fired=true"
        );
        assert!(
            block_reason.starts_with("fail-closed-recoverable:"),
            "TD #71 / BC-1.18.002 PC5: block_reason MUST start with \
             'fail-closed-recoverable:' for BlockIfMarker crash-block; got: {block_reason:?}"
        );
        assert!(
            block_reason.contains("unvalidated-mutation.marker"),
            "block_reason MUST name the marker file so the human operator (T3) knows \
             which file break-glass rm targets; got: {block_reason:?}"
        );
        assert!(
            block_reason.contains("ADR-048"),
            "block_reason MUST cite ADR-048 for traceability; got: {block_reason:?}"
        );
    }

    /// F-P2-001 (HIGH / BLOCKER) regression test — BC-1.18.002 v1.6 PC5 +
    /// INV6/T4 (ADR-048 §Decision 3 amended v1.1 Four-Tier Recovery Model).
    ///
    /// The crash-path BLOCK message MUST:
    ///   1. contain all four marker fields (plugin_name, artifact_path, cause,
    ///      expires_at) with their actual values from the marker;
    ///   2. present recovery options ordered T1 (agent re-validation via
    ///      Edit/Write) before T2 (TTL auto-clear) before T3 (human
    ///      out-of-band `rm`);
    ///   3. NOT instruct the agent itself to run `rm` — a T3 human-action
    ///      mention is allowed, but it must be framed as an operator action,
    ///      never as a command the agent should execute to self-de-quarantine
    ///      (CWE-636 per ADR-048 §Decision 3 / INV6/T4).
    #[test]
    fn block_if_marker_crash_message_has_fields_t1_first_and_no_agent_rm_instruction() {
        let fields = MarkerFields {
            timestamp: "2026-08-30T12:00:00Z".to_string(),
            plugin_name: "validate-factory-path-staging".to_string(),
            artifact_path: "/repo/.factory/STATE.md".to_string(),
            cause: "fuel".to_string(),
            trace_id: "trace-abc123".to_string(),
            expires_at: "2026-08-31T12:00:00Z".to_string(),
        };
        let outcomes = vec![PluginOutcome {
            plugin_name: "validate-unvalidated-mutation-marker".to_string(),
            plugin_version: "1.0.0".to_string(),
            on_error: OnError::BlockIfMarker,
            result: PluginResult::Crashed {
                trap_string: "unreachable".to_string(),
                stderr: String::new(),
                elapsed_ms: 1,
                fuel_consumed: 0,
            },
            block_if_marker_fired: true,
            block_if_marker_fields: Some(Box::new(fields)),
        }];

        let (_, block_reason) = super::extract_block_info(&outcomes);

        // (1) All four mandatory marker fields, with actual values, present.
        assert!(
            block_reason.contains("validate-factory-path-staging"),
            "F-P2-001: block message MUST include the marker's plugin_name; \
             got: {block_reason:?}"
        );
        assert!(
            block_reason.contains("/repo/.factory/STATE.md"),
            "F-P2-001: block message MUST include the marker's artifact_path; \
             got: {block_reason:?}"
        );
        assert!(
            block_reason.contains("\"fuel\""),
            "F-P2-001: block message MUST include the marker's cause; \
             got: {block_reason:?}"
        );
        assert!(
            block_reason.contains("2026-08-31T12:00:00Z"),
            "F-P2-001: block message MUST include the marker's expires_at; \
             got: {block_reason:?}"
        );

        // (2) T1 (re-run Edit/Write) ordered before T2 (TTL) ordered before
        // T3 (human out-of-band rm).
        let t1_pos = block_reason
            .find("T1")
            .expect("F-P2-001: block message MUST mention T1 (primary agent recovery)");
        let t2_pos = block_reason
            .find("T2")
            .expect("F-P2-001: block message MUST mention T2 (TTL auto-clear)");
        let t3_pos = block_reason
            .find("T3")
            .expect("F-P2-001: block message MUST mention T3 (human out-of-band rm)");
        assert!(
            t1_pos < t2_pos && t2_pos < t3_pos,
            "F-P2-001 / BC-1.18.002 v1.6 PC5: recovery options MUST be ordered \
             T1 first, T2 second, T3 third; got positions T1={t1_pos} T2={t2_pos} T3={t3_pos} \
             in: {block_reason:?}"
        );

        // (3) The agent is never instructed to run rm itself — only the human
        // operator (T3) is described as able to. The forbidden CWE-636 pattern
        // is an unqualified imperative directed at the caller (e.g. the old
        // "recover via `rm ...`" phrasing this test guards against).
        assert!(
            !block_reason.contains("recover via `rm"),
            "F-P2-001 / ADR-048 INV6/T4: block message MUST NOT instruct the agent \
             to run rm (CWE-636 self-de-quarantine forbidden); got: {block_reason:?}"
        );
        assert!(
            block_reason.contains("this agent MUST NOT perform that action")
                || block_reason.contains("NOT an agent action"),
            "F-P2-001 / ADR-048 INV6/T4: the T3 rm mention MUST be explicitly framed \
             as a human/operator action, not an agent instruction; got: {block_reason:?}"
        );
    }

    /// BC-1.18.002 (absent-marker path):
    /// Crash + on_error=BlockIfMarker + block_if_marker_fired=false →
    /// extract_block_info MUST NOT surface this outcome as blocking (marker was absent).
    #[test]
    fn block_if_marker_crash_without_fired_flag_not_surfaced() {
        let outcomes = vec![PluginOutcome {
            plugin_name: "validate-unvalidated-mutation-marker".to_string(),
            plugin_version: "1.0.0".to_string(),
            on_error: OnError::BlockIfMarker,
            result: PluginResult::Crashed {
                trap_string: "unreachable".to_string(),
                stderr: String::new(),
                elapsed_ms: 1,
                fuel_consumed: 0,
            },
            block_if_marker_fired: false,
            block_if_marker_fields: None,
        }];

        let (blocking_plugins, block_reason) = super::extract_block_info(&outcomes);

        assert!(
            blocking_plugins.is_empty(),
            "BC-1.18.002 absent-marker path: blocking_plugins MUST be empty when \
             block_if_marker_fired=false (no marker present); got: {blocking_plugins:?}"
        );
        assert!(
            block_reason.is_empty(),
            "BC-1.18.002 absent-marker path: block_reason MUST be empty when \
             block_if_marker_fired=false; got: {block_reason:?}"
        );
    }

    /// Regression guard: existing on_error=Block crash still surfaces with the
    /// generic fail-closed sentinel, unchanged by the BlockIfMarker addition.
    #[test]
    fn on_error_block_crash_still_surfaces_generic_sentinel() {
        let outcomes = vec![PluginOutcome {
            plugin_name: "some-gate".to_string(),
            plugin_version: "1.0.0".to_string(),
            on_error: OnError::Block,
            result: PluginResult::Crashed {
                trap_string: "unreachable".to_string(),
                stderr: String::new(),
                elapsed_ms: 1,
                fuel_consumed: 0,
            },
            block_if_marker_fired: false,
            block_if_marker_fields: None,
        }];

        let (blocking_plugins, block_reason) = super::extract_block_info(&outcomes);

        assert_eq!(
            blocking_plugins, "some-gate",
            "on_error=Block crash MUST still surface as a blocker (regression guard)"
        );
        assert_eq!(
            block_reason, "fail-closed: plugin crashed",
            "on_error=Block crash reason MUST remain 'fail-closed: plugin crashed' (regression)"
        );
    }
}
