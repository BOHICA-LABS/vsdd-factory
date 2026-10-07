// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Integration tests for the S-25.01 INDETERMINATE marker lifecycle.
//!
//! Adversary pass-1 findings encoded here (each test references the finding):
//!
//! - BLOCKER-1 (AC-012 / BC-1.18.003 PC1+INV2): marker auto-clear MUST be wired into
//!   execute_tiers dispatch path, not just exposed as a free function.
//! - BLOCKER-2 (AC-005 / BC-1.18.001 INV4 / EC-002 / EC-009): marker write MUST be
//!   PostToolUse-only; PreToolUse INDETERMINATE MUST emit advisory event only.
//! - MEDIUM-5 (AC-005 / AC-007 / BC-1.18.001 PC4): artifact_path MUST be threaded from
//!   the PostToolUse tool_input.file_path, not hardcoded to empty string.
//!
//! All tests drive the REAL execute_tiers / execute_tier path — no direct calls to
//! indeterminate_marker functions (which would constitute a paper-fix). Integration
//! tests are required per the adversary finding ("NOT a direct unit call to
//! delete_marker_if_pass").

use std::path::PathBuf;
use std::sync::Arc;

use factory_dispatcher::engine::{EpochTicker, build_engine};
use factory_dispatcher::executor::{ExecutorInputs, execute_tiers};
use factory_dispatcher::host::HostContext;
use factory_dispatcher::internal_log::InternalLog;
use factory_dispatcher::plugin_loader::PluginCache;
use factory_dispatcher::registry::{Capabilities, FailurePolicy, OnError, Registry, RegistryEntry};
use factory_dispatcher::resolver::ResolverRegistry;
use factory_dispatcher::routing::group_by_priority;

// ---------------------------------------------------------------------------
// WAT fixtures (mirrors executor_integration.rs)
// ---------------------------------------------------------------------------

/// Minimal WASI command: returns cleanly with exit_code=0 (→ DispatchOutcome::Pass).
const WAT_NORMAL: &str = r#"
(module
  (memory (export "memory") 1)
  (func (export "_start")))
"#;

/// Infinite loop at an epoch yield point.
/// With fuel_cap=100, exhausts fuel almost immediately → DispatchOutcome::Indeterminate{Fuel}.
const WAT_HANG: &str = r#"
(module
  (memory (export "memory") 1)
  (func (export "_start") (loop (br 0))))
"#;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn compile_to(dir: &std::path::Path, name: &str, wat: &str) -> PathBuf {
    let bytes = wat::parse_str(wat).expect("wat parse");
    let path = dir.join(format!("{name}.wasm"));
    std::fs::write(&path, bytes).unwrap();
    path
}

fn make_registry(entries: Vec<RegistryEntry>) -> Registry {
    Registry {
        schema_version: 1,
        defaults: Default::default(),
        hooks: entries,
    }
}

fn make_pass_entry(path: &std::path::Path, name: &str, event: &str) -> RegistryEntry {
    // Entry for a WASM that will return PASS (exit_code=0); ample fuel so it completes.
    RegistryEntry {
        name: name.to_string(),
        event: event.to_string(),
        tool: None,
        plugin: path.to_path_buf(),
        priority: Some(100),
        enabled: true,
        timeout_ms: Some(5_000),
        fuel_cap: Some(1_000_000_000), // ample fuel — no fuel exhaustion
        on_error: Some(OnError::Continue),
        capabilities: Some(Capabilities::default()),
        config: toml::Value::Table(toml::Table::new()),
        async_flag: false,
        needs_context: vec![],
        failure_policy: FailurePolicy::FailClosed,
    }
}

fn make_indeterminate_entry(path: &std::path::Path, name: &str, event: &str) -> RegistryEntry {
    // Entry for a WASM that will INDETERMINATE (WAT_HANG + very low fuel → Fuel exhaustion).
    RegistryEntry {
        name: name.to_string(),
        event: event.to_string(),
        tool: None,
        plugin: path.to_path_buf(),
        priority: Some(100),
        enabled: true,
        timeout_ms: Some(5_000), // wall-clock safety net (fuel exhausts first)
        fuel_cap: Some(100),     // very low: exhausts in microseconds → INDETERMINATE(Fuel)
        on_error: Some(OnError::Continue), // don't also trigger on_error=block path
        capabilities: Some(Capabilities::default()),
        config: toml::Value::Table(toml::Table::new()),
        async_flag: false,
        needs_context: vec![],
        failure_policy: FailurePolicy::FailClosed,
    }
}

fn executor_inputs_with_cwd<'a>(
    engine: &'a wasmtime::Engine,
    cache: &'a PluginCache,
    registry: &'a Registry,
    internal_log: &Arc<InternalLog>,
    cwd: PathBuf,
    payload: serde_json::Value,
) -> ExecutorInputs<'a> {
    let mut base = HostContext::new("", "0.0.1", "sess-marker-integ", "trace-marker-integ");
    base.cwd = cwd;
    base.internal_log = Some(internal_log.clone());
    ExecutorInputs {
        engine,
        cache,
        registry,
        payload_value: payload,
        base_host_ctx: base,
        internal_log: internal_log.clone(),
        resolver_registry: Arc::new(ResolverRegistry::new()),
    }
}

fn write_test_marker(path: &std::path::Path, plugin_name: &str) {
    std::fs::write(
        path,
        format!(
            "timestamp = \"2026-08-31T00:00:00Z\"\n\
             plugin_name = \"{plugin_name}\"\n\
             artifact_path = \"\"\n\
             cause = \"fuel\"\n\
             trace_id = \"trace-integ-test\"\n"
        ),
    )
    .expect("test setup: write marker");
}

fn write_test_marker_with_artifact(path: &std::path::Path, plugin_name: &str, artifact_path: &str) {
    std::fs::write(
        path,
        format!(
            "timestamp = \"2026-08-31T00:00:00Z\"\n\
             plugin_name = \"{plugin_name}\"\n\
             artifact_path = \"{artifact_path}\"\n\
             cause = \"fuel\"\n\
             trace_id = \"trace-integ-test\"\n"
        ),
    )
    .expect("test setup: write marker with artifact");
}

// ---------------------------------------------------------------------------
// BLOCKER-1 tests: marker auto-clear via execute_tiers dispatch path
// ---------------------------------------------------------------------------

/// BLOCKER-1 PRIMARY (RED): PASS from the named plugin via execute_tiers MUST delete
/// the marker. BC-1.18.003 PC1 + INV2. AC-012.
///
/// **Why this is an integration test (not a unit test):** The adversary finding is that
/// `delete_marker_if_pass` exists as a free function but is NEVER CALLED from the
/// dispatch path. Writing a unit test that calls `delete_marker_if_pass` directly
/// would be the paper-fix the adversary warned against (TD-VSDD-059). This test
/// drives `execute_tiers → execute_tier → spawn_blocking` so the implementer MUST
/// wire the clear into the dispatch path itself.
///
/// Currently FAILS (RED) because execute_tier has no `delete_marker_if_pass` call
/// in its PASS arm — the marker is never deleted by the dispatch path.
#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_003_named_plugin_pass_clears_marker_via_execute_tiers() {
    // AC-012 / BC-1.18.003 PC1 + INV2
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());

    // Set up .factory dir and pre-write a marker for "plugin-p"
    let factory_dir = dir.path().join(".factory");
    std::fs::create_dir_all(&factory_dir).unwrap();
    let marker_path = factory_dir.join("unvalidated-mutation.marker");
    write_test_marker(&marker_path, "plugin-p");
    assert!(
        marker_path.exists(),
        "pre-condition: marker must exist before dispatch"
    );

    // WAT_NORMAL returns exit_code=0 → DispatchOutcome::Pass
    let pass_wasm = compile_to(dir.path(), "plugin-p", WAT_NORMAL);
    let entry = make_pass_entry(&pass_wasm, "plugin-p", "PostToolUse");
    let registry = make_registry(vec![entry]);
    let matched: Vec<&RegistryEntry> = registry.hooks.iter().collect();
    let tiers = group_by_priority(&registry, matched);

    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));
    let summary = execute_tiers(
        executor_inputs_with_cwd(
            &engine,
            &cache,
            &registry,
            &internal_log,
            dir.path().to_path_buf(),
            serde_json::json!({}),
        ),
        tiers,
        None,
    )
    .await;

    // Plugin produced PASS (WAT_NORMAL exits 0 → no block)
    assert_eq!(
        summary.exit_code, 0,
        "PASS plugin must not set block intent"
    );

    // AC-012 / BC-1.18.003 PC1 + INV2:
    // A PASS result from "plugin-p" MUST cause the dispatcher to call
    // `delete_marker_if_pass` and remove the marker.
    //
    // RED: currently fails because execute_tier has NO delete_marker_if_pass call
    // in its PASS path. The marker persists untouched after dispatch completes.
    assert!(
        !marker_path.exists(),
        "AC-012 / BC-1.18.003 PC1: PASS from 'plugin-p' via execute_tiers MUST delete \
         the marker at .factory/unvalidated-mutation.marker. \
         FAILS because execute_tier has no delete_marker_if_pass in its PASS arm."
    );

    // F-P10-002 / VP-108 PC1 / BC-3.08.001 §Event 9 Wire format + Mandatory fields:
    // `marker.cleared(REVALIDATED)` (emitted by `emit_marker_cleared`, called from the
    // execute_tiers PASS arm in executor.rs after `delete_marker_if_pass` succeeds)
    // MUST carry a distinct top-level `timestamp` field, separate from the common `ts`
    // field. No prior test asserted this on the REVALIDATED clear_mode, which let the
    // real `emit_marker_cleared` implementation ship without it (F-P10-002).
    let log_dir = dir.path().join("logs");
    let cleared_events = read_events_of_type(&log_dir, "marker.cleared");
    assert_eq!(
        cleared_events.len(),
        1,
        "VP-108 PC1: exactly one marker.cleared event must be durably logged for the \
         REVALIDATED clear — got {cleared_events:?}"
    );
    assert_eq!(
        cleared_events[0]["clear_mode"], "REVALIDATED",
        "VP-108 PC1: clear_mode must be REVALIDATED"
    );
    let timestamp_str = cleared_events[0]
        .get("timestamp")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        !timestamp_str.is_empty(),
        "F-P10-002 / BC-3.08.001 Event 9: emit_marker_cleared(REVALIDATED) must emit a \
         non-empty distinct 'timestamp' field; field is absent or empty"
    );

    // O-P18-002 / VP-108 PC1: `marker.cleared(REVALIDATED)` MUST carry the marker's OWN
    // `trace_id` (set via `write_test_marker` above as "trace-integ-test"), not the
    // current dispatch's `dispatcher_trace_id`. `emit_marker_cleared` deliberately links
    // back to the original `plugin.indeterminate` event via `marker_fields.trace_id` —
    // this assertion directly exercises that provenance link at the execute_tiers
    // REVALIDATED callsite.
    assert_eq!(
        cleared_events[0]["trace_id"], "trace-integ-test",
        "VP-108 PC1: marker.cleared(REVALIDATED) trace_id must equal the marker's own \
         trace_id ('trace-integ-test'), linking the clear event back to the marker it \
         cleared — got {:?}",
        cleared_events[0]["trace_id"]
    );
}

/// MEDIUM-1 (PreToolUse-does-not-clear complement): PASS from the NAMED plugin on a
/// **PreToolUse** event MUST NOT clear the marker. BC-1.18.003 INV2 (scoped clear).
///
/// The implementer added a `&& entry.event == "PostToolUse"` guard to the marker-clear
/// path so that only PostToolUse PASS events trigger `delete_marker_if_pass`. A PreToolUse
/// PASS from the same plugin must NOT clear the marker — otherwise a pre-dispatch pass
/// (where nothing has yet been validated) would silently unblock the gate.
///
/// Relation to BLOCKER-1: BLOCKER-1 PRIMARY uses PostToolUse and expects deletion.
/// This test uses PreToolUse and expects NO deletion. Together they verify the full
/// PostToolUse-only scoping requirement.
#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_003_pretooluse_pass_does_not_clear_marker() {
    // AC-012 / BC-1.18.003 INV2 (PostToolUse-only scoping for marker clear)
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());

    // Set up .factory dir and pre-write a marker for "plugin-p".
    let factory_dir = dir.path().join(".factory");
    std::fs::create_dir_all(&factory_dir).unwrap();
    let marker_path = factory_dir.join("unvalidated-mutation.marker");
    write_test_marker(&marker_path, "plugin-p");
    assert!(
        marker_path.exists(),
        "pre-condition: marker must exist before dispatch"
    );

    // WAT_NORMAL returns exit_code=0 → DispatchOutcome::Pass.
    // Event = "PreToolUse": the `&& entry.event == "PostToolUse"` guard MUST prevent
    // delete_marker_if_pass from being called for this event type.
    let pass_wasm = compile_to(dir.path(), "plugin-p", WAT_NORMAL);
    let entry = make_pass_entry(
        &pass_wasm,
        "plugin-p",
        "PreToolUse", // <-- KEY: PreToolUse; marker clear MUST be suppressed
    );
    let registry = make_registry(vec![entry]);
    let matched: Vec<&RegistryEntry> = registry.hooks.iter().collect();
    let tiers = group_by_priority(&registry, matched);

    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));
    let summary = execute_tiers(
        executor_inputs_with_cwd(
            &engine,
            &cache,
            &registry,
            &internal_log,
            dir.path().to_path_buf(),
            serde_json::json!({}),
        ),
        tiers,
        None,
    )
    .await;

    // Plugin produced PASS (WAT_NORMAL exits 0 → no block).
    assert_eq!(
        summary.exit_code, 0,
        "pre-condition: PASS plugin must not set block intent"
    );

    // AC-012 / BC-1.18.003 INV2 (PostToolUse-only scoping):
    // A PASS result from "plugin-p" on a PreToolUse event MUST NOT clear the marker.
    // Only a PostToolUse PASS from the named plugin clears the marker.
    // This exercises the `&& entry.event == "PostToolUse"` guard in execute_tier's
    // PASS arm — the gate is cleared only when both conditions hold simultaneously.
    assert!(
        marker_path.exists(),
        "AC-012 / BC-1.18.003 INV2 (PostToolUse-only scoping): PreToolUse PASS from \
         'plugin-p' MUST NOT delete the marker at .factory/unvalidated-mutation.marker. \
         The clear is gated on PostToolUse event type; PreToolUse PASS must leave marker intact."
    );
}

/// BLOCKER-1 SCOPING CONSTRAINT: PASS from a DIFFERENT plugin MUST NOT clear the
/// marker for plugin-p. BC-1.18.003 INV2. AC-012.
///
/// Currently GREEN (accidentally: no clear call at all → marker stays for wrong plugin
/// too). Must STAY GREEN after fix (implementer must check plugin_name before clearing).
///
/// Included to prevent a naive implementation that clears on ANY pass from ANY plugin.
#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_003_different_plugin_pass_does_not_clear_named_plugin_marker() {
    // AC-012 / BC-1.18.003 INV2 (scoping)
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());

    let factory_dir = dir.path().join(".factory");
    std::fs::create_dir_all(&factory_dir).unwrap();
    let marker_path = factory_dir.join("unvalidated-mutation.marker");
    // Marker records plugin-p; we dispatch plugin-q (DIFFERENT)
    write_test_marker(&marker_path, "plugin-p");

    let pass_wasm = compile_to(dir.path(), "plugin-q", WAT_NORMAL);
    let entry = make_pass_entry(&pass_wasm, "plugin-q", "PostToolUse"); // <-- DIFFERENT plugin
    let registry = make_registry(vec![entry]);
    let matched: Vec<&RegistryEntry> = registry.hooks.iter().collect();
    let tiers = group_by_priority(&registry, matched);

    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));
    execute_tiers(
        executor_inputs_with_cwd(
            &engine,
            &cache,
            &registry,
            &internal_log,
            dir.path().to_path_buf(),
            serde_json::json!({}),
        ),
        tiers,
        None,
    )
    .await;

    // BC-1.18.003 INV2 (scoping): plugin-q's PASS MUST NOT clear plugin-p's marker.
    // The clear is scoped to the plugin named in the marker.
    assert!(
        marker_path.exists(),
        "AC-012 / BC-1.18.003 INV2: PASS from 'plugin-q' MUST NOT delete 'plugin-p's marker \
         (scoped clear — only the named plugin's PASS clears)"
    );
}

// ---------------------------------------------------------------------------
// BLOCKER-2 tests: PostToolUse-only marker write
// ---------------------------------------------------------------------------

/// BLOCKER-2 PRIMARY (RED): INDETERMINATE for a fail-closed plugin dispatched on a
/// PreToolUse event MUST NOT write the marker. BC-1.18.001 INV4. AC-005. EC-002 / EC-009.
///
/// Currently FAILS (RED) because execute_tier has no PostToolUse event-type guard —
/// it calls `write_indeterminate_marker` for BOTH PreToolUse and PostToolUse
/// INDETERMINATE events when `should_write_marker` returns true.
#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_001_pretooluse_indeterminate_does_not_write_marker() {
    // AC-005 / BC-1.18.001 INV4 / EC-002 / EC-009
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let _ticker = EpochTicker::start(engine.clone()); // safety net for epoch timeout
    let cache = PluginCache::new(engine.clone());

    // Pre-create .factory so the write WOULD succeed if the guard is absent.
    // (Without the dir, write_indeterminate_marker silently fails → test passes vacuously.)
    let factory_dir = dir.path().join(".factory");
    std::fs::create_dir_all(&factory_dir).unwrap();
    let marker_path = factory_dir.join("unvalidated-mutation.marker");
    assert!(
        !marker_path.exists(),
        "pre-condition: marker must NOT exist at dispatch start"
    );

    // WAT_HANG + fuel_cap=100 → fuel exhaustion → DispatchOutcome::Indeterminate{Fuel}.
    // Event = "PreToolUse": BC-1.18.001 INV4 says marker write MUST be suppressed.
    let hang_wasm = compile_to(dir.path(), "validate-factory-path-staging", WAT_HANG);
    let entry = make_indeterminate_entry(
        &hang_wasm,
        "validate-factory-path-staging",
        "PreToolUse", // <-- KEY: PreToolUse; marker write MUST be suppressed per INV4
    );
    let registry = make_registry(vec![entry]);
    let matched: Vec<&RegistryEntry> = registry.hooks.iter().collect();
    let tiers = group_by_priority(&registry, matched);

    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));
    execute_tiers(
        executor_inputs_with_cwd(
            &engine,
            &cache,
            &registry,
            &internal_log,
            dir.path().to_path_buf(),
            serde_json::json!({}),
        ),
        tiers,
        None,
    )
    .await;

    // AC-005 / BC-1.18.001 INV4 / EC-009:
    // PreToolUse INDETERMINATE → advisory `plugin.indeterminate` event only.
    // NO marker written (PostToolUse-only invariant).
    //
    // RED: currently fails because execute_tier writes the marker for both event types.
    assert!(
        !marker_path.exists(),
        "AC-005 / BC-1.18.001 INV4 / EC-009: PreToolUse INDETERMINATE for fail-closed \
         plugin MUST NOT write the marker (PostToolUse-only per BC-1.18.001 INV4). \
         FAILS because execute_tier has no event-type guard in its INDETERMINATE arm."
    );
}

/// BLOCKER-2 COMPLEMENT: PostToolUse INDETERMINATE for fail-closed MUST write marker.
/// Currently GREEN (current code writes for both event types, no guard).
/// Must STAY GREEN after the implementer adds the PostToolUse guard.
#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_001_posttooluse_indeterminate_writes_marker_for_fail_closed() {
    // AC-005 / BC-1.18.001 PC4 + INV4 (the write IS expected for PostToolUse)
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let _ticker = EpochTicker::start(engine.clone());
    let cache = PluginCache::new(engine.clone());

    let factory_dir = dir.path().join(".factory");
    std::fs::create_dir_all(&factory_dir).unwrap();
    let marker_path = factory_dir.join("unvalidated-mutation.marker");

    let hang_wasm = compile_to(dir.path(), "validate-factory-path-staging", WAT_HANG);
    let entry = make_indeterminate_entry(
        &hang_wasm,
        "validate-factory-path-staging",
        "PostToolUse", // PostToolUse: marker MUST be written
    );
    let registry = make_registry(vec![entry]);
    let matched: Vec<&RegistryEntry> = registry.hooks.iter().collect();
    let tiers = group_by_priority(&registry, matched);

    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));
    execute_tiers(
        executor_inputs_with_cwd(
            &engine,
            &cache,
            &registry,
            &internal_log,
            dir.path().to_path_buf(),
            serde_json::json!({}),
        ),
        tiers,
        None,
    )
    .await;

    // AC-005 / BC-1.18.001 PC4 + INV4:
    // PostToolUse INDETERMINATE for fail-closed MUST write the marker.
    assert!(
        marker_path.exists(),
        "AC-005 / BC-1.18.001 PC4: PostToolUse INDETERMINATE for fail-closed \
         MUST write the .factory/unvalidated-mutation.marker file"
    );
}

// ---------------------------------------------------------------------------
// MEDIUM-5: artifact_path threaded from tool_input.file_path
// ---------------------------------------------------------------------------

/// MEDIUM-5 (RED): The marker's artifact_path field MUST be threaded from the
/// PostToolUse tool_input.file_path, not hardcoded to empty string. AC-005 / AC-007 /
/// BC-1.18.001 PC4.
///
/// Currently FAILS (RED) because execute_tier hardcodes `artifact_path: String::new()`
/// in the MarkerFields construction inside the spawn_blocking closure.
#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_001_artifact_path_threaded_from_tool_input_file_path() {
    // AC-005 / AC-007 / BC-1.18.001 PC4
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let _ticker = EpochTicker::start(engine.clone());
    let cache = PluginCache::new(engine.clone());

    let factory_dir = dir.path().join(".factory");
    std::fs::create_dir_all(&factory_dir).unwrap();
    let marker_path = factory_dir.join("unvalidated-mutation.marker");

    let expected_artifact_path = "/path/to/.factory/STATE.md";

    // PostToolUse Edit dispatch with file_path set — the INDETERMINATE event must
    // capture this path in the marker's artifact_path field.
    let payload = serde_json::json!({
        "tool_name": "Edit",
        "tool_input": {
            "file_path": expected_artifact_path
        }
    });

    let hang_wasm = compile_to(dir.path(), "validate-factory-path-staging", WAT_HANG);
    let entry =
        make_indeterminate_entry(&hang_wasm, "validate-factory-path-staging", "PostToolUse");
    let registry = make_registry(vec![entry]);
    let matched: Vec<&RegistryEntry> = registry.hooks.iter().collect();
    let tiers = group_by_priority(&registry, matched);

    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));
    execute_tiers(
        executor_inputs_with_cwd(
            &engine,
            &cache,
            &registry,
            &internal_log,
            dir.path().to_path_buf(),
            payload,
        ),
        tiers,
        None,
    )
    .await;

    assert!(
        marker_path.exists(),
        "pre-condition for MEDIUM-5: PostToolUse fail-closed INDETERMINATE MUST write marker"
    );

    let content =
        std::fs::read_to_string(&marker_path).expect("marker file must be readable after write");

    // AC-005 / AC-007 / BC-1.18.001 PC4:
    // The marker MUST record the artifact_path from tool_input.file_path.
    //
    // RED: currently fails because execute_tier uses `artifact_path: String::new()`
    // (hardcoded empty) instead of extracting from payload_value.tool_input.file_path.
    assert!(
        content.contains(expected_artifact_path),
        "AC-005 / AC-007 / BC-1.18.001 PC4: marker artifact_path MUST be threaded from \
         tool_input.file_path '{}'. \
         FAILS because execute_tier hardcodes artifact_path = String::new(). \
         Actual marker content: '{}'",
        expected_artifact_path,
        content
    );
}

// ---------------------------------------------------------------------------
// EC-008 / EC-009: artifact-scoped marker-clear via execute_tiers (VP-106)
// ---------------------------------------------------------------------------

/// EC-008 INTEGRATION (VP-106 / BC-1.18.003 INV2): same plugin, DIFFERENT non-empty
/// artifact → execute_tiers MUST NOT clear the marker (artifact mismatch keeps quarantine).
///
/// Two-phase test:
/// - Phase 1: dispatch plugin "p" PostToolUse PASS with payload file_path="/abs/B.md"
///   while marker records artifact_path="/abs/A.md". Marker MUST persist.
/// - Phase 2 (positive control): dispatch plugin "p" PostToolUse PASS with payload
///   file_path="/abs/A.md" (same artifact). Marker MUST be cleared.
///
/// This exercises the artifact-threading callsite in execute_tier (MEDIUM-5):
/// `artifact_path_for_marker` is extracted from `payload.tool_input.file_path` and
/// forwarded into `delete_marker_if_pass`, not just the leaf function in isolation.
#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_003_EC_008_artifact_mismatch_preserves_marker_via_execute_tiers() {
    // EC-008 / BC-1.18.003 INV2 / VP-106
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());

    let factory_dir = dir.path().join(".factory");
    std::fs::create_dir_all(&factory_dir).unwrap();
    let marker_path = factory_dir.join("unvalidated-mutation.marker");
    // Pre-write marker for "p" recording artifact "/abs/A.md".
    write_test_marker_with_artifact(&marker_path, "p", "/abs/A.md");
    assert!(
        marker_path.exists(),
        "pre-condition: marker must exist before dispatch"
    );

    let pass_wasm = compile_to(dir.path(), "p", WAT_NORMAL);
    let entry = make_pass_entry(&pass_wasm, "p", "PostToolUse");
    let registry = make_registry(vec![entry]);

    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));

    // Phase 1: PASS on "/abs/B.md" (different from marker's "/abs/A.md") → MUST NOT clear.
    let payload_b = serde_json::json!({
        "tool_name": "Edit",
        "tool_input": { "file_path": "/abs/B.md" }
    });
    let tiers1: Vec<Vec<&RegistryEntry>> =
        group_by_priority(&registry, registry.hooks.iter().collect());
    let summary1 = execute_tiers(
        executor_inputs_with_cwd(
            &engine,
            &cache,
            &registry,
            &internal_log,
            dir.path().to_path_buf(),
            payload_b,
        ),
        tiers1,
        None,
    )
    .await;
    assert_eq!(
        summary1.exit_code, 0,
        "pre-condition: PASS plugin must not block"
    );

    // EC-008 / BC-1.18.003 INV2: artifact mismatch → quarantine persists.
    assert!(
        marker_path.exists(),
        "EC-008 / BC-1.18.003 INV2 via execute_tiers: PASS from 'p' with \
         tool_input.file_path='/abs/B.md' MUST NOT clear marker{{artifact_path='/abs/A.md'}}. \
         Quarantine persists across different artifacts."
    );

    // Phase 2 (positive control): PASS on "/abs/A.md" (same artifact) → MUST clear.
    let payload_a = serde_json::json!({
        "tool_name": "Edit",
        "tool_input": { "file_path": "/abs/A.md" }
    });
    let tiers2: Vec<Vec<&RegistryEntry>> =
        group_by_priority(&registry, registry.hooks.iter().collect());
    let summary2 = execute_tiers(
        executor_inputs_with_cwd(
            &engine,
            &cache,
            &registry,
            &internal_log,
            dir.path().to_path_buf(),
            payload_a,
        ),
        tiers2,
        None,
    )
    .await;
    assert_eq!(
        summary2.exit_code, 0,
        "pre-condition: PASS plugin must not block"
    );

    // EC-008 (positive control): same artifact → quarantine lifted.
    assert!(
        !marker_path.exists(),
        "EC-008 (positive control) via execute_tiers: PASS from 'p' with \
         tool_input.file_path='/abs/A.md' MUST clear marker{{artifact_path='/abs/A.md'}} \
         (same artifact — quarantine lifted)."
    );
}

/// EC-009 INTEGRATION (VP-106 / BC-1.18.003 INV2): empty marker `artifact_path` →
/// execute_tiers MUST clear the marker even when the payload carries a non-empty file_path.
///
/// Exercises the artifact-threading callsite: `artifact_path_for_marker="/abs/anything.md"`
/// is extracted from the payload, but since `marker.artifact_path` is empty (non-artifact-
/// scoped validator), `delete_marker_if_pass` clears unconditionally (vacuous satisfaction).
#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_003_EC_009_empty_marker_artifact_path_clears_via_execute_tiers() {
    // EC-009 / BC-1.18.003 INV2 / VP-106
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());

    let factory_dir = dir.path().join(".factory");
    std::fs::create_dir_all(&factory_dir).unwrap();
    let marker_path = factory_dir.join("unvalidated-mutation.marker");
    // Pre-write marker for "p" with empty artifact_path (non-artifact-scoped validator).
    write_test_marker(&marker_path, "p");
    assert!(
        marker_path.exists(),
        "pre-condition: marker must exist before dispatch"
    );

    let pass_wasm = compile_to(dir.path(), "p", WAT_NORMAL);
    let entry = make_pass_entry(&pass_wasm, "p", "PostToolUse");
    let registry = make_registry(vec![entry]);
    let tiers: Vec<Vec<&RegistryEntry>> =
        group_by_priority(&registry, registry.hooks.iter().collect());

    // Dispatch with a non-empty file_path to confirm empty marker artifact_path clears regardless.
    let payload = serde_json::json!({
        "tool_name": "Write",
        "tool_input": { "file_path": "/abs/anything.md" }
    });
    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));
    let summary = execute_tiers(
        executor_inputs_with_cwd(
            &engine,
            &cache,
            &registry,
            &internal_log,
            dir.path().to_path_buf(),
            payload,
        ),
        tiers,
        None,
    )
    .await;
    assert_eq!(
        summary.exit_code, 0,
        "pre-condition: PASS plugin must not block"
    );

    // EC-009 / BC-1.18.003 INV2: empty marker artifact_path → cleared unconditionally.
    assert!(
        !marker_path.exists(),
        "EC-009 / BC-1.18.003 INV2 via execute_tiers: marker{{artifact_path=''}} MUST be \
         cleared by PASS from 'p' even with non-empty current_artifact_path='/abs/anything.md'. \
         Empty artifact_path is the non-artifact-scoped fallback; cleared unconditionally."
    );
}

// ---------------------------------------------------------------------------
// F-P9-001 (human-ratified): marker.cleared(SUPERSEDED) MUST be emitted only
// AFTER a successful marker write — symmetric to the v1.4 marker.written fix
// (F-P6-001). Before this fix, execute_tier / spawn_async_plugin called
// emit_superseded_if_cross_pair UNCONDITIONALLY before attempting the write,
// so a write failure left the OLD marker on disk while a SUPERSEDED audit
// record falsely claimed it had been overwritten.
//
// These tests drive the REAL execute_tiers / execute_tier path (same
// discipline as the BLOCKER-1/BLOCKER-2 tests above) — a direct unit call to
// emit_superseded_if_cross_pair would not exercise the callsite ordering bug
// and would be a paper-fix per TD-VSDD-059.
// ---------------------------------------------------------------------------

/// Reads every internal-log event of `event_type` from today's rotated JSONL
/// file under `log_dir` (`dispatcher-internal-<date>.jsonl`).
///
/// No polling: safe to call immediately after an `.await`ed `execute_tiers`
/// call because `InternalLog::write` appends directly to the file (no
/// buffering, no async queue) before `HostContext::emit_internal` returns —
/// see `wait_for_log_event`'s doc comment in `full_stack_plugin_invocation.rs`
/// for the underlying guarantee this relies on.
fn read_events_of_type(log_dir: &std::path::Path, event_type: &str) -> Vec<serde_json::Value> {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let log_file = log_dir.join(format!("dispatcher-internal-{today}.jsonl"));
    let Ok(content) = std::fs::read_to_string(&log_file) else {
        return Vec::new();
    };
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|v| v.get("type").and_then(|t| t.as_str()) == Some(event_type))
        .collect()
}

/// Reads the ordered sequence of `type` values from today's rotated JSONL
/// file — used to verify emission ORDER (append-only file, single-threaded
/// dispatch in these tests, so line order == emission order).
fn read_event_type_sequence(log_dir: &std::path::Path) -> Vec<String> {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let log_file = log_dir.join(format!("dispatcher-internal-{today}.jsonl"));
    let Ok(content) = std::fs::read_to_string(&log_file) else {
        return Vec::new();
    };
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|v| v.get("type").and_then(|t| t.as_str()).map(str::to_string))
        .collect()
}

/// F-P9-001 REGRESSION (RED before the fix): cross-pair marker present +
/// `write_indeterminate_marker` returns `Err` ⟹ NO `marker.cleared(SUPERSEDED)`
/// and NO `marker.written` are emitted. The pre-existing (OLD pair) marker
/// MUST survive untouched on disk.
///
/// Write failure is forced the same way the existing F-P6-001
/// `test_marker_written_emitted_on_successful_write_only` unit test forces
/// it — by making the write target unwritable — but here via `.factory`
/// directory permissions (0o555, no write bit) rather than an absent parent
/// dir, because the OLD marker file must remain present and READABLE (so
/// `read_all_marker_fields` still succeeds and the cross-pair precondition
/// holds) while the temp-file WRITE inside `.factory` fails with EACCES
/// before ever reaching the rename step.
#[cfg(unix)]
#[tokio::test(flavor = "current_thread")]
async fn test_F_P9_001_cross_pair_write_failure_suppresses_superseded_and_marker_written() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let _ticker = EpochTicker::start(engine.clone());
    let cache = PluginCache::new(engine.clone());

    let factory_dir = dir.path().join(".factory");
    std::fs::create_dir_all(&factory_dir).unwrap();
    let marker_path = factory_dir.join("unvalidated-mutation.marker");
    // Pre-existing marker for a DIFFERENT (plugin_name, artifact_path) pair —
    // the cross-pair precondition emit_superseded_if_cross_pair requires.
    write_test_marker_with_artifact(&marker_path, "old-plugin", "/abs/old.md");
    assert!(
        marker_path.exists(),
        "pre-condition: old (cross-pair) marker must exist before dispatch"
    );

    // Remove write permission on .factory AFTER the marker above was created,
    // so write_indeterminate_marker's temp-file write (inside .factory) hits
    // EACCES before ever reaching the rename step. Read/execute bits remain,
    // so the pre-existing marker stays readable.
    std::fs::set_permissions(&factory_dir, std::fs::Permissions::from_mode(0o555)).unwrap();

    let hang_wasm = compile_to(dir.path(), "new-plugin", WAT_HANG);
    let entry = make_indeterminate_entry(&hang_wasm, "new-plugin", "PostToolUse");
    let registry = make_registry(vec![entry]);
    let tiers: Vec<Vec<&RegistryEntry>> =
        group_by_priority(&registry, registry.hooks.iter().collect());

    let payload = serde_json::json!({
        "tool_name": "Edit",
        "tool_input": { "file_path": "/abs/new.md" }
    });
    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));
    execute_tiers(
        executor_inputs_with_cwd(
            &engine,
            &cache,
            &registry,
            &internal_log,
            dir.path().to_path_buf(),
            payload,
        ),
        tiers,
        None,
    )
    .await;

    // Restore write permission so tempdir Drop-cleanup can remove files.
    std::fs::set_permissions(&factory_dir, std::fs::Permissions::from_mode(0o755)).unwrap();

    // The write must actually have failed: the OLD marker survives untouched.
    assert!(
        marker_path.exists(),
        "pre-condition: a failed overwrite must leave the OLD marker on disk"
    );
    let surviving = std::fs::read_to_string(&marker_path).unwrap();
    assert!(
        surviving.contains("old-plugin"),
        "pre-condition: the marker on disk after the failed write must still be \
         the OLD (pre-existing) marker, not a partially-written new one: {surviving}"
    );

    let log_dir = dir.path().join("logs");
    let superseded_events = read_events_of_type(&log_dir, "marker.cleared");
    assert!(
        superseded_events.is_empty(),
        "F-P9-001 regression: write_indeterminate_marker returned Err, so \
         marker.cleared(SUPERSEDED) MUST NOT be emitted — the old marker was never \
         actually overwritten, so emitting SUPERSEDED here would be a false audit \
         record. Got: {superseded_events:?}"
    );

    let written_events = read_events_of_type(&log_dir, "marker.written");
    assert!(
        written_events.is_empty(),
        "F-P9-001 / F-P6-001: a failed marker write MUST NOT emit marker.written. \
         Got: {written_events:?}"
    );
}

/// F-P9-001 PRESERVE: cross-pair marker present + write SUCCEEDS ⟹ exactly one
/// `marker.cleared(SUPERSEDED)` (carrying the OLD pair's fields) AND exactly one
/// `marker.written` (carrying the NEW pair's fields) are emitted — both AFTER
/// the write, and SUPERSEDED strictly before marker.written within the Ok arm.
#[tokio::test(flavor = "current_thread")]
async fn test_F_P9_001_cross_pair_write_success_emits_superseded_then_marker_written() {
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let _ticker = EpochTicker::start(engine.clone());
    let cache = PluginCache::new(engine.clone());

    let factory_dir = dir.path().join(".factory");
    std::fs::create_dir_all(&factory_dir).unwrap();
    let marker_path = factory_dir.join("unvalidated-mutation.marker");
    write_test_marker_with_artifact(&marker_path, "old-plugin", "/abs/old.md");

    let hang_wasm = compile_to(dir.path(), "new-plugin", WAT_HANG);
    let entry = make_indeterminate_entry(&hang_wasm, "new-plugin", "PostToolUse");
    let registry = make_registry(vec![entry]);
    let tiers: Vec<Vec<&RegistryEntry>> =
        group_by_priority(&registry, registry.hooks.iter().collect());

    let payload = serde_json::json!({
        "tool_name": "Edit",
        "tool_input": { "file_path": "/abs/new.md" }
    });
    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));
    execute_tiers(
        executor_inputs_with_cwd(
            &engine,
            &cache,
            &registry,
            &internal_log,
            dir.path().to_path_buf(),
            payload,
        ),
        tiers,
        None,
    )
    .await;

    assert!(
        marker_path.exists(),
        "pre-condition: the write must have succeeded"
    );
    let new_marker_content = std::fs::read_to_string(&marker_path).unwrap();
    assert!(
        new_marker_content.contains("new-plugin"),
        "post-condition: marker on disk must now be the NEW pair after a \
         successful overwrite: {new_marker_content}"
    );

    let log_dir = dir.path().join("logs");

    let superseded_events = read_events_of_type(&log_dir, "marker.cleared");
    assert_eq!(
        superseded_events.len(),
        1,
        "exactly one marker.cleared(SUPERSEDED) event for the OLD pair — got {superseded_events:?}"
    );
    let sup = &superseded_events[0];
    assert_eq!(
        sup.get("clear_mode").and_then(|v| v.as_str()),
        Some("SUPERSEDED")
    );
    assert_eq!(
        sup.get("plugin_name").and_then(|v| v.as_str()),
        Some("old-plugin"),
        "SUPERSEDED event MUST carry the OLD pair's plugin_name"
    );
    assert_eq!(
        sup.get("artifact_path").and_then(|v| v.as_str()),
        Some("/abs/old.md"),
        "SUPERSEDED event MUST carry the OLD pair's artifact_path"
    );

    let written_events = read_events_of_type(&log_dir, "marker.written");
    assert_eq!(
        written_events.len(),
        1,
        "exactly one marker.written event for the NEW pair — got {written_events:?}"
    );
    let w = &written_events[0];
    assert_eq!(
        w.get("plugin_name").and_then(|v| v.as_str()),
        Some("new-plugin"),
        "marker.written event MUST carry the NEW pair's plugin_name"
    );
    assert_eq!(
        w.get("artifact_path").and_then(|v| v.as_str()),
        Some("/abs/new.md"),
        "marker.written event MUST carry the NEW pair's artifact_path"
    );

    // Sequencing within the Ok arm: SUPERSEDED (closing the old pair's key)
    // is emitted strictly BEFORE marker.written (opening the new pair's key).
    let sequence = read_event_type_sequence(&log_dir);
    let cleared_idx = sequence.iter().position(|t| t == "marker.cleared");
    let written_idx = sequence.iter().position(|t| t == "marker.written");
    assert!(
        matches!((cleared_idx, written_idx), (Some(c), Some(w)) if c < w),
        "F-P9-001: within the Ok arm, marker.cleared(SUPERSEDED) MUST be emitted \
         (and thus logged) strictly before marker.written — sequence: {sequence:?}"
    );
}

/// F-P9-001 PRESERVE: same-pair overwrite (continuous quarantine of the same
/// target) + write SUCCEEDS ⟹ NO `marker.cleared(SUPERSEDED)` is emitted, but
/// `marker.written` still is (the write itself succeeded; only the
/// cross-pair-supersession record is suppressed for a same-pair overwrite).
#[tokio::test(flavor = "current_thread")]
async fn test_F_P9_001_same_pair_overwrite_no_superseded_via_execute_tiers() {
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let _ticker = EpochTicker::start(engine.clone());
    let cache = PluginCache::new(engine.clone());

    let factory_dir = dir.path().join(".factory");
    std::fs::create_dir_all(&factory_dir).unwrap();
    let marker_path = factory_dir.join("unvalidated-mutation.marker");
    // Same (plugin_name, artifact_path) pair as the NEW dispatch below.
    write_test_marker_with_artifact(&marker_path, "same-plugin", "/abs/same.md");

    let hang_wasm = compile_to(dir.path(), "same-plugin", WAT_HANG);
    let entry = make_indeterminate_entry(&hang_wasm, "same-plugin", "PostToolUse");
    let registry = make_registry(vec![entry]);
    let tiers: Vec<Vec<&RegistryEntry>> =
        group_by_priority(&registry, registry.hooks.iter().collect());

    let payload = serde_json::json!({
        "tool_name": "Edit",
        "tool_input": { "file_path": "/abs/same.md" }
    });
    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));
    execute_tiers(
        executor_inputs_with_cwd(
            &engine,
            &cache,
            &registry,
            &internal_log,
            dir.path().to_path_buf(),
            payload,
        ),
        tiers,
        None,
    )
    .await;

    assert!(
        marker_path.exists(),
        "pre-condition: the write must have succeeded"
    );

    let log_dir = dir.path().join("logs");
    let superseded_events = read_events_of_type(&log_dir, "marker.cleared");
    assert!(
        superseded_events.is_empty(),
        "same-pair overwrite (continuous quarantine of the same target) MUST NOT \
         emit marker.cleared(SUPERSEDED) — got {superseded_events:?}"
    );
    let written_events = read_events_of_type(&log_dir, "marker.written");
    assert_eq!(
        written_events.len(),
        1,
        "same-pair overwrite MUST still emit marker.written on a successful \
         write — got {written_events:?}"
    );
}
