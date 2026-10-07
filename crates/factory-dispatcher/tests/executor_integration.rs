// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Integration coverage for the tokio executor (S-1.6).
//!
//! Exercises the full `execute_tiers` path against real wasmtime
//! modules. Uses inline WAT fixtures for normal / hang / panic
//! scenarios; the hang scenario relies on an [`EpochTicker`] to fire
//! the epoch interrupt.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use factory_dispatcher::engine::{EpochTicker, build_engine};
use factory_dispatcher::executor::{ExecutorInputs, execute_tiers};
use factory_dispatcher::host::HostContext;
use factory_dispatcher::internal_log::InternalLog;
use factory_dispatcher::plugin_loader::PluginCache;
use factory_dispatcher::registry::{Capabilities, Registry, RegistryEntry};
use factory_dispatcher::resolver::ResolverRegistry;
use factory_dispatcher::routing::group_by_priority;

/// Minimal WASI command: returns cleanly.
const WAT_NORMAL: &str = r#"
(module
  (memory (export "memory") 1)
  (func (export "_start")))
"#;

/// Infinite loop at an epoch yield point.
const WAT_HANG: &str = r#"
(module
  (memory (export "memory") 1)
  (func (export "_start") (loop (br 0))))
"#;

/// Unreachable instruction — wasmtime traps.
const WAT_CRASH: &str = r#"
(module
  (memory (export "memory") 1)
  (func (export "_start") unreachable))
"#;

fn compile_to(dir: &std::path::Path, name: &str, wat: &str) -> PathBuf {
    let bytes = wat::parse_str(wat).expect("wat parse");
    let path = dir.join(format!("{name}.wasm"));
    std::fs::write(&path, bytes).unwrap();
    path
}

fn entry_at(path: &std::path::Path, name: &str, priority: u32) -> RegistryEntry {
    RegistryEntry {
        name: name.to_string(),
        event: "PreToolUse".to_string(),
        tool: None,
        plugin: path.to_path_buf(),
        priority: Some(priority),
        enabled: true,
        timeout_ms: Some(2_000),
        fuel_cap: Some(1_000_000_000),
        on_error: None,
        capabilities: Some(Capabilities::default()),
        config: toml::Value::Table(toml::Table::new()),
        // S-15.01 stub: async_flag defaults to false (sync_group) per BC-7.06.001
        // postcondition 3 (serde-default semantics). Test fixture updated for
        // compilation; the implementer updates schema_version to 2 in T-3a.
        async_flag: false,
        needs_context: vec![],
        failure_policy: Default::default(),
    }
}

fn registry_with(entries: Vec<RegistryEntry>) -> Registry {
    // Hand-build — parsing from TOML would be overkill for fixture setup.
    Registry {
        schema_version: 1,
        defaults: Default::default(),
        hooks: entries,
    }
}

fn inputs<'a>(
    engine: &'a wasmtime::Engine,
    cache: &'a PluginCache,
    registry: &'a Registry,
    internal_log: &Arc<InternalLog>,
) -> ExecutorInputs<'a> {
    let mut base = HostContext::new("", "0.0.1", "sess-integration", "trace-integration");
    base.internal_log = Some(internal_log.clone());
    ExecutorInputs {
        engine,
        cache,
        registry,
        payload_value: serde_json::json!({}),
        base_host_ctx: base,
        internal_log: internal_log.clone(),
        // S-12.03: empty registry — tests don't exercise resolver path
        resolver_registry: Arc::new(ResolverRegistry::new()),
    }
}

#[tokio::test(flavor = "current_thread")]
async fn parallel_happy_path_five_plugins_one_tier() {
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());

    let mut entries = Vec::new();
    for i in 0..5 {
        let path = compile_to(dir.path(), &format!("ok{i}"), WAT_NORMAL);
        entries.push(entry_at(&path, &format!("ok{i}"), 100));
    }
    let registry = registry_with(entries);
    let matched: Vec<&RegistryEntry> = registry.hooks.iter().collect();
    let tiers = group_by_priority(&registry, matched);

    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));
    let summary = execute_tiers(
        inputs(&engine, &cache, &registry, &internal_log),
        tiers,
        None,
    )
    .await;

    assert_eq!(summary.per_plugin_results.len(), 5);
    assert_eq!(summary.exit_code, 0);
    assert!(!summary.block_intent);
    for outcome in summary.per_plugin_results {
        assert!(matches!(
            outcome.result,
            factory_dispatcher::invoke::PluginResult::Ok { .. }
        ));
    }
}

#[tokio::test(flavor = "current_thread")]
async fn crash_does_not_affect_siblings() {
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());

    let ok_path = compile_to(dir.path(), "ok", WAT_NORMAL);
    let crash_path = compile_to(dir.path(), "crash", WAT_CRASH);
    let entries = vec![
        entry_at(&ok_path, "ok-a", 100),
        entry_at(&crash_path, "crash", 100),
        entry_at(&ok_path, "ok-b", 100),
    ];
    let registry = registry_with(entries);
    let matched: Vec<&RegistryEntry> = registry.hooks.iter().collect();
    let tiers = group_by_priority(&registry, matched);

    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));
    let summary = execute_tiers(
        inputs(&engine, &cache, &registry, &internal_log),
        tiers,
        None,
    )
    .await;

    assert_eq!(summary.per_plugin_results.len(), 3);
    let by_name: std::collections::HashMap<String, _> = summary
        .per_plugin_results
        .iter()
        .map(|o| (o.plugin_name.clone(), o.result.clone()))
        .collect();
    assert!(matches!(
        by_name["ok-a"],
        factory_dispatcher::invoke::PluginResult::Ok { .. }
    ));
    assert!(matches!(
        by_name["ok-b"],
        factory_dispatcher::invoke::PluginResult::Ok { .. }
    ));
    assert!(matches!(
        by_name["crash"],
        factory_dispatcher::invoke::PluginResult::Crashed { .. }
    ));
    assert_eq!(summary.exit_code, 0);
}

#[tokio::test(flavor = "current_thread")]
async fn parallel_timeout_does_not_cascade() {
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let _ticker = EpochTicker::start(engine.clone());
    let cache = PluginCache::new(engine.clone());

    let ok_path = compile_to(dir.path(), "ok", WAT_NORMAL);
    let hang_path = compile_to(dir.path(), "hang", WAT_HANG);

    let mut hang_entry = entry_at(&hang_path, "hanger", 100);
    hang_entry.timeout_ms = Some(120);
    let entries = vec![
        entry_at(&ok_path, "ok-a", 100),
        hang_entry,
        entry_at(&ok_path, "ok-b", 100),
        entry_at(&ok_path, "ok-c", 100),
    ];
    let registry = registry_with(entries);
    let matched: Vec<&RegistryEntry> = registry.hooks.iter().collect();
    let tiers = group_by_priority(&registry, matched);

    let started = Instant::now();
    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));
    let summary = execute_tiers(
        inputs(&engine, &cache, &registry, &internal_log),
        tiers,
        None,
    )
    .await;
    let wall_ms = started.elapsed().as_millis() as u64;

    // Four plugins; one hangs for ~120ms. Sequential execution would
    // push wall-clock past 500ms even on fast hardware (the 3 normal
    // plugins are ~ms each but wasmtime cold-compile varies). Parallel
    // execution should stay under ~1s with plenty of margin.
    assert!(
        wall_ms < 2_000,
        "wall_ms={wall_ms} suggests plugins ran sequentially"
    );

    let by_name: std::collections::HashMap<String, _> = summary
        .per_plugin_results
        .iter()
        .map(|o| (o.plugin_name.clone(), o.result.clone()))
        .collect();
    assert!(matches!(
        by_name["hanger"],
        factory_dispatcher::invoke::PluginResult::Timeout { .. }
    ));
    for name in ["ok-a", "ok-b", "ok-c"] {
        assert!(
            matches!(
                by_name[name],
                factory_dispatcher::invoke::PluginResult::Ok { .. }
            ),
            "plugin {name} did not complete ok",
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn multi_tier_runs_in_priority_order() {
    // Two tiers. Tier 10 has one plugin; tier 100 has two. Verify
    // the summary orders results tier-10-first (execution
    // ordering is observable via the per_plugin_results vec).
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());

    let ok_path = compile_to(dir.path(), "ok", WAT_NORMAL);
    let entries = vec![
        entry_at(&ok_path, "late-a", 100),
        entry_at(&ok_path, "early", 10),
        entry_at(&ok_path, "late-b", 100),
    ];
    let registry = registry_with(entries);
    let matched: Vec<&RegistryEntry> = registry.hooks.iter().collect();
    let tiers = group_by_priority(&registry, matched);

    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));
    let summary = execute_tiers(
        inputs(&engine, &cache, &registry, &internal_log),
        tiers,
        None,
    )
    .await;

    assert_eq!(summary.per_plugin_results[0].plugin_name, "early");
    // Tier 2 has both late plugins; order within tier is unspecified.
    let tier2_names: std::collections::HashSet<_> = summary.per_plugin_results[1..]
        .iter()
        .map(|o| o.plugin_name.as_str())
        .collect();
    assert_eq!(tier2_names.len(), 2);
    assert!(tier2_names.contains("late-a"));
    assert!(tier2_names.contains("late-b"));
}

#[tokio::test(flavor = "current_thread")]
async fn empty_tier_set_returns_zero_exit_code() {
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());

    let registry = registry_with(vec![]);
    let tiers: Vec<Vec<&RegistryEntry>> = vec![];

    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));
    let summary = execute_tiers(
        inputs(&engine, &cache, &registry, &internal_log),
        tiers,
        None,
    )
    .await;

    assert!(summary.per_plugin_results.is_empty());
    assert_eq!(summary.exit_code, 0);
    assert!(!summary.block_intent);
}
