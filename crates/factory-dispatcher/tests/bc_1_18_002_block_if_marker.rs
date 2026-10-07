// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Integration tests for ADR-048 §Decision 1: `on_error = "block_if_marker"` crash gate.
//!
//! Drives the real `execute_tiers` path against a compiled WAT_CRASH plugin registered
//! with `on_error = "block_if_marker"`. Verifies:
//!
//! - BC-1.18.002 PC5: Crash + active marker → `TierExecutionSummary::block_intent = true`
//! - BC-1.18.002: Crash + no marker → `TierExecutionSummary::block_intent = false`
//!
//! These are RED Gate tests: both will fail until the implementer wires
//! `plugin_block_if_marker` into `execute_tiers`.

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
// WAT fixture
// ---------------------------------------------------------------------------

/// Unreachable instruction — wasmtime traps with PluginResult::Crashed.
const WAT_CRASH: &str = r#"
(module
  (memory (export "memory") 1)
  (func (export "_start") unreachable))
"#;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn compile_to(dir: &std::path::Path, name: &str, wat: &str) -> PathBuf {
    let bytes = wat::parse_str(wat).expect("wat parse");
    let path = dir.join(format!("{name}.wasm"));
    std::fs::write(&path, bytes).unwrap();
    path
}

fn make_crash_block_if_marker_entry(path: &std::path::Path, name: &str) -> RegistryEntry {
    // Plugin that will crash (WAT_CRASH) with on_error = BlockIfMarker.
    // Ample fuel so the crash is from `unreachable`, not fuel exhaustion.
    RegistryEntry {
        name: name.to_string(),
        event: "PreToolUse".to_string(),
        tool: None,
        plugin: path.to_path_buf(),
        priority: Some(100),
        enabled: true,
        timeout_ms: Some(5_000),
        fuel_cap: Some(1_000_000_000),
        on_error: Some(OnError::BlockIfMarker),
        capabilities: Some(Capabilities::default()),
        config: toml::Value::Table(toml::Table::new()),
        async_flag: false,
        needs_context: vec![],
        failure_policy: FailurePolicy::FailClosed,
    }
}

fn make_registry(entries: Vec<RegistryEntry>) -> Registry {
    Registry {
        schema_version: 1,
        defaults: Default::default(),
        hooks: entries,
    }
}

fn executor_inputs_with_cwd<'a>(
    engine: &'a wasmtime::Engine,
    cache: &'a PluginCache,
    registry: &'a Registry,
    internal_log: &Arc<InternalLog>,
    cwd: PathBuf,
) -> ExecutorInputs<'a> {
    let mut base = HostContext::new("", "0.0.1", "sess-bim-integ", "trace-bim-integ");
    base.cwd = cwd;
    base.internal_log = Some(internal_log.clone());
    ExecutorInputs {
        engine,
        cache,
        registry,
        payload_value: serde_json::json!({}),
        base_host_ctx: base,
        internal_log: internal_log.clone(),
        resolver_registry: Arc::new(ResolverRegistry::new()),
    }
}

fn write_active_6field_marker(marker_path: &std::path::Path) {
    std::fs::write(
        marker_path,
        "timestamp = \"2026-08-31T00:00:00Z\"\n\
         plugin_name = \"crash-gate\"\n\
         artifact_path = \"\"\n\
         cause = \"fuel\"\n\
         trace_id = \"trace-bim-integ\"\n\
         expires_at = \"2099-01-01T00:00:00Z\"\n",
    )
    .expect("test setup: write 6-field active marker");
}

// ---------------------------------------------------------------------------
// Integration tests
// ---------------------------------------------------------------------------

/// BC-1.18.002 PC5 (RED): Crash + on_error=BlockIfMarker + active marker →
/// `execute_tiers` returns `block_intent = true`.
///
/// Exercises VP-105 through the full executor dispatch path (not a unit call
/// to `plugin_block_if_marker`). Confirms ADR-048 §Decision 1 is wired into
/// the production `execute_tiers` loop.
#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_002_execute_tiers_block_if_marker_crash_with_marker_blocks() {
    let dir = tempfile::tempdir().expect("tempdir");
    let engine = build_engine().expect("build_engine");
    let _ticker = EpochTicker::start(engine.clone());
    let cache = PluginCache::new(engine.clone());

    // Compile the crashing WASM plugin.
    let crash_path = compile_to(dir.path(), "crash-bim", WAT_CRASH);
    let entry = make_crash_block_if_marker_entry(&crash_path, "crash-gate");
    let registry = make_registry(vec![entry]);

    // Write an active 6-field marker at cwd/.factory/unvalidated-mutation.marker.
    let factory_dir = dir.path().join(".factory");
    std::fs::create_dir_all(&factory_dir).expect("create .factory dir");
    write_active_6field_marker(&factory_dir.join("unvalidated-mutation.marker"));

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
        ),
        tiers,
        None,
    )
    .await;

    assert!(
        summary.block_intent,
        "BC-1.18.002 PC5 / VP-105: Crash + BlockIfMarker + active marker MUST set \
         block_intent=true in execute_tiers (ADR-048 §Decision 1 wiring)"
    );
    assert_eq!(
        summary.exit_code, 2,
        "BC-1.18.002 PC5: block_intent=true MUST yield exit_code=2"
    );
}

/// BC-1.18.002 (RED): Crash + on_error=BlockIfMarker + no marker →
/// `execute_tiers` returns `block_intent = false` (conditional — no block without quarantine).
///
/// The crash itself does NOT block when no marker is present. This is the
/// "allow on absent marker" postcondition of ADR-048 §Decision 1.
#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_002_execute_tiers_block_if_marker_crash_no_marker_allows() {
    let dir = tempfile::tempdir().expect("tempdir");
    let engine = build_engine().expect("build_engine");
    let _ticker = EpochTicker::start(engine.clone());
    let cache = PluginCache::new(engine.clone());

    // Compile the crashing WASM plugin.
    let crash_path = compile_to(dir.path(), "crash-bim-nomarker", WAT_CRASH);
    let entry = make_crash_block_if_marker_entry(&crash_path, "crash-gate-nomarker");
    let registry = make_registry(vec![entry]);

    // Create .factory dir but write NO marker — absent marker → Allow.
    let factory_dir = dir.path().join(".factory");
    std::fs::create_dir_all(&factory_dir).expect("create .factory dir");

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
        ),
        tiers,
        None,
    )
    .await;

    assert!(
        !summary.block_intent,
        "BC-1.18.002 / VP-105: Crash + BlockIfMarker + absent marker MUST NOT set \
         block_intent — conditional gate requires active marker (ADR-048 §Decision 1)"
    );
}
