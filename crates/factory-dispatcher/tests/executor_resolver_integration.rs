// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Integration tests: ResolverRegistry wired into executor.rs dispatch (F-001 / AC-002 / AC-003).
//!
//! Verifies that `execute_tiers` and `spawn_async_plugin` both:
//! - AC-002: short-circuit resolver invocation when `needs_context` is empty
//!   (zero overhead — BC-1.13.001 PC3).
//! - AC-003: invoke the resolver and merge its output when `needs_context` is
//!   non-empty (BC-1.13.001 PC4 / BC-4.12.005).
//!
//! Uses inline WAT fixtures + an in-process `ContextResolver` mock; no WASM
//! I/O inspection is required — the resolver's call count is the observable.
//!
//! BC traces: BC-1.13.001 PC3/PC4, BC-4.12.005, AC-002, AC-003.
//! Story: S-12.03 (F-001 adversary finding).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use factory_dispatcher::engine::build_engine;
use factory_dispatcher::executor::{ExecutorInputs, execute_tiers, spawn_async_plugin};
use factory_dispatcher::host::HostContext;
use factory_dispatcher::internal_log::InternalLog;
use factory_dispatcher::plugin_loader::PluginCache;
use factory_dispatcher::registry::{Capabilities, Registry, RegistryEntry};
use factory_dispatcher::resolver::{
    ContextResolver, ResolverError, ResolverInput, ResolverOutput, ResolverRegistry,
};
use factory_dispatcher::routing::group_by_priority;

// ---------------------------------------------------------------------------
// Minimal WAT fixture: returns immediately (successful exit).
// ---------------------------------------------------------------------------

const WAT_NORMAL: &str = r#"
(module
  (memory (export "memory") 1)
  (func (export "_start")))
"#;

// ---------------------------------------------------------------------------
// Counting resolver: records every invocation; returns a fixed output.
// ---------------------------------------------------------------------------

struct CountingResolver {
    call_count: Arc<Mutex<usize>>,
    output_key: String,
}

impl CountingResolver {
    fn new(key: &str) -> (Self, Arc<Mutex<usize>>) {
        let count = Arc::new(Mutex::new(0usize));
        (
            Self {
                call_count: count.clone(),
                output_key: key.to_string(),
            },
            count,
        )
    }
}

impl ContextResolver for CountingResolver {
    fn name(&self) -> &str {
        &self.output_key
    }

    fn resolve(&self, _input: &ResolverInput) -> Result<Option<ResolverOutput>, ResolverError> {
        *self.call_count.lock().unwrap() += 1;
        Ok(Some(ResolverOutput {
            key: self.output_key.clone(),
            value: Some(serde_json::json!({ "injected": true })),
        }))
    }
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn compile_wasm(dir: &std::path::Path, name: &str) -> PathBuf {
    let bytes = wat::parse_str(WAT_NORMAL).expect("WAT parse");
    let path = dir.join(format!("{name}.wasm"));
    std::fs::write(&path, bytes).unwrap();
    path
}

fn make_entry(path: &std::path::Path, name: &str, needs_context: Vec<String>) -> RegistryEntry {
    RegistryEntry {
        name: name.to_string(),
        event: "PreToolUse".to_string(),
        tool: None,
        plugin: path.to_path_buf(),
        priority: Some(100),
        enabled: true,
        timeout_ms: Some(2_000),
        fuel_cap: Some(1_000_000_000),
        on_error: None,
        capabilities: Some(Capabilities::default()),
        config: toml::Value::Table(toml::Table::new()),
        async_flag: false,
        needs_context,
        failure_policy: Default::default(),
    }
}

fn make_registry(entries: Vec<RegistryEntry>) -> Registry {
    Registry {
        schema_version: 2,
        defaults: Default::default(),
        hooks: entries,
    }
}

fn make_executor_inputs<'a>(
    engine: &'a wasmtime::Engine,
    cache: &'a PluginCache,
    registry: &'a Registry,
    internal_log: &Arc<InternalLog>,
    resolver_registry: Arc<ResolverRegistry>,
) -> ExecutorInputs<'a> {
    let mut base = HostContext::new("", "0.0.1", "sess-resolver-test", "trace-resolver-test");
    base.internal_log = Some(internal_log.clone());
    ExecutorInputs {
        engine,
        cache,
        registry,
        payload_value: serde_json::json!({
            "event_name": "PreToolUse",
            "tool_name": "Write",
            "session_id": "resolver-test-session",
            "dispatcher_trace_id": "resolver-test-trace"
        }),
        base_host_ctx: base,
        internal_log: internal_log.clone(),
        resolver_registry,
    }
}

// ---------------------------------------------------------------------------
// AC-002: zero-overhead short-circuit
// ---------------------------------------------------------------------------

/// AC-002 / BC-1.13.001 PC3:
/// When a hook entry has `needs_context: []`, the resolver registry MUST NOT
/// be consulted — the counting resolver's call count remains zero.
///
/// This test exercises the `needs_context.is_empty()` short-circuit in
/// `build_plugin_config` inside `executor::execute_tiers` (F-001).
#[tokio::test(flavor = "current_thread")]
async fn ac002_execute_tiers_zero_overhead_when_needs_context_empty() {
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());
    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));

    let wasm = compile_wasm(dir.path(), "ok");
    // Hook with empty needs_context — resolver must NOT be called.
    let entry = make_entry(&wasm, "no-context-hook", vec![]);
    let registry = make_registry(vec![entry]);
    let matched: Vec<&RegistryEntry> = registry.hooks.iter().collect();
    let tiers = group_by_priority(&registry, matched);

    let (resolver, call_count) = CountingResolver::new("foo");
    let mut resolver_registry = ResolverRegistry::new();
    resolver_registry
        .register(Box::new(resolver))
        .expect("first registration must succeed");
    let resolver_registry = Arc::new(resolver_registry);

    let inputs = make_executor_inputs(&engine, &cache, &registry, &internal_log, resolver_registry);
    let summary = execute_tiers(inputs, tiers, None).await;

    // Plugin must have run without error
    assert_eq!(
        summary.per_plugin_results.len(),
        1,
        "AC-002: exactly one plugin must have run"
    );
    assert_eq!(
        summary.exit_code, 0,
        "AC-002: exit_code must be 0 (normal run)"
    );

    // Resolver must NOT have been invoked (zero overhead — BC-1.13.001 PC3).
    assert_eq!(
        *call_count.lock().unwrap(),
        0,
        "AC-002 / BC-1.13.001 PC3: resolver must NOT be invoked when \
         needs_context is empty (zero-overhead short-circuit)"
    );
}

// ---------------------------------------------------------------------------
// AC-003: resolver invoked when needs_context is non-empty
// ---------------------------------------------------------------------------

/// AC-003 / BC-1.13.001 PC4:
/// When a hook entry has `needs_context: ["foo"]` and a resolver named "foo"
/// is registered, `execute_tiers` MUST invoke the resolver exactly once.
///
/// This is the load-bearing integration verification: the counting resolver
/// demonstrates that `build_plugin_config` in `executor.rs` consults the
/// `ResolverRegistry` before splicing `plugin_config` into the payload
/// (F-001 — the executor wiring is active).
#[tokio::test(flavor = "current_thread")]
async fn ac003_execute_tiers_invokes_resolver_when_needs_context_non_empty() {
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());
    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));

    let wasm = compile_wasm(dir.path(), "ok");
    // Hook declares needs_context: ["foo"] — resolver "foo" must be invoked.
    let entry = make_entry(&wasm, "context-hook", vec!["foo".to_string()]);
    let registry = make_registry(vec![entry]);
    let matched: Vec<&RegistryEntry> = registry.hooks.iter().collect();
    let tiers = group_by_priority(&registry, matched);

    let (resolver, call_count) = CountingResolver::new("foo");
    let mut resolver_registry = ResolverRegistry::new();
    resolver_registry
        .register(Box::new(resolver))
        .expect("first registration must succeed");
    let resolver_registry = Arc::new(resolver_registry);

    let inputs = make_executor_inputs(&engine, &cache, &registry, &internal_log, resolver_registry);
    let summary = execute_tiers(inputs, tiers, None).await;

    assert_eq!(
        summary.per_plugin_results.len(),
        1,
        "AC-003: exactly one plugin must have run"
    );
    assert_eq!(summary.exit_code, 0, "AC-003: exit_code must be 0");

    // Resolver MUST have been invoked exactly once (BC-1.13.001 PC4).
    assert_eq!(
        *call_count.lock().unwrap(),
        1,
        "AC-003 / BC-1.13.001 PC4: resolver 'foo' must be invoked exactly once \
         when needs_context = [\"foo\"] (executor.rs wiring — F-001)"
    );
}

// ---------------------------------------------------------------------------
// AC-002 via spawn_async_plugin: zero-overhead on async path
// ---------------------------------------------------------------------------

/// AC-002 (async path) / BC-1.13.001 PC3:
/// `spawn_async_plugin` also uses `build_plugin_config` — verify the
/// short-circuit applies equally to the async dispatch path.
#[tokio::test(flavor = "current_thread")]
async fn ac002_spawn_async_plugin_zero_overhead_when_needs_context_empty() {
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let cache = Arc::new(PluginCache::new(engine.clone()));
    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));

    let wasm = compile_wasm(dir.path(), "ok_async");
    let mut entry = make_entry(&wasm, "no-context-async", vec![]);
    entry.async_flag = true;
    let registry = make_registry(vec![entry.clone()]);

    let (resolver, call_count) = CountingResolver::new("bar");
    let mut resolver_registry = ResolverRegistry::new();
    resolver_registry
        .register(Box::new(resolver))
        .expect("first registration must succeed");
    let resolver_registry = Arc::new(resolver_registry);

    let mut base_ctx = HostContext::new("", "0.0.1", "sess-async-resolver", "trace-async-resolver");
    base_ctx.internal_log = Some(internal_log.clone());

    let handle = spawn_async_plugin(
        engine.clone(),
        cache.clone(),
        registry.defaults.clone(),
        entry,
        serde_json::json!({
            "event_name": "PreToolUse",
            "session_id": "async-resolver-session",
            "dispatcher_trace_id": "async-resolver-trace"
        }),
        base_ctx,
        internal_log.clone(),
        resolver_registry,
    );

    let outcome = tokio::time::timeout(std::time::Duration::from_secs(10), handle)
        .await
        .expect("JoinHandle join must not timeout")
        .expect("JoinHandle must not panic");

    assert!(
        matches!(
            outcome.result,
            factory_dispatcher::invoke::PluginResult::Ok { .. }
        ),
        "AC-002 (async): plugin must complete successfully"
    );

    // Resolver must NOT have been called (needs_context is empty).
    assert_eq!(
        *call_count.lock().unwrap(),
        0,
        "AC-002 (async path) / BC-1.13.001 PC3: resolver must NOT be invoked \
         when needs_context is empty on the async dispatch path"
    );
}

// ---------------------------------------------------------------------------
// F-P2-007 (test 3): ErroringResolver causes resolver.error event in InternalLog
// ---------------------------------------------------------------------------

/// An in-process resolver that always returns `Err(ResolverError::Trap)`.
struct ErroringResolver {
    key: String,
}

impl ContextResolver for ErroringResolver {
    fn name(&self) -> &str {
        &self.key
    }

    fn resolve(&self, _input: &ResolverInput) -> Result<Option<ResolverOutput>, ResolverError> {
        Err(ResolverError::Trap {
            name: self.key.clone(),
            detail: "injected test failure".to_string(),
        })
    }
}

// ---------------------------------------------------------------------------
// F-P3-003: resolver.not_found event written to InternalLog when resolver missing
// ---------------------------------------------------------------------------

/// F-P3-003 / BC-4.12.005 PC4 / SOUL #4:
/// When a hook declares `needs_context: ["unknown"]` and NO resolver named
/// "unknown" is registered, `execute_tiers` must:
/// 1. Complete without panicking (dispatch continues — BC-4.12.005 INV3).
/// 2. Write a `resolver.not_found` event to InternalLog, with `resolver_name`
///    equal to "unknown" and the correct hook name.
#[tokio::test(flavor = "current_thread")]
async fn ac005_resolver_not_found_event_appears_in_internal_log() {
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());
    let log_dir = dir.path().join("logs");
    let internal_log = Arc::new(InternalLog::new(log_dir.clone()));

    let wasm = compile_wasm(dir.path(), "ok_not_found");
    // Hook declares needs_context: ["unknown"] — no resolver named "unknown" exists.
    let entry = make_entry(&wasm, "not-found-hook", vec!["unknown".to_string()]);
    let registry = make_registry(vec![entry]);
    let matched: Vec<&factory_dispatcher::registry::RegistryEntry> =
        registry.hooks.iter().collect();
    let tiers = factory_dispatcher::routing::group_by_priority(&registry, matched);

    // Register zero resolvers — "unknown" will not be found.
    let resolver_registry = Arc::new(ResolverRegistry::new());

    let inputs = make_executor_inputs(&engine, &cache, &registry, &internal_log, resolver_registry);
    // execute_tiers must NOT panic — dispatch continues even with missing resolver.
    let summary = execute_tiers(inputs, tiers, None).await;

    assert_eq!(
        summary.per_plugin_results.len(),
        1,
        "F-P3-003: exactly one plugin must have run despite missing resolver"
    );

    // Flush InternalLog and verify resolver.not_found event is present.
    drop(internal_log);

    let all_log_content: String = std::fs::read_dir(&log_dir)
        .expect("log dir must exist after dispatch")
        .filter_map(|e| e.ok())
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .collect::<Vec<_>>()
        .join(
            "
",
        );

    assert!(
        all_log_content.contains("resolver.not_found"),
        "F-P3-003 / SOUL #4: InternalLog must contain a 'resolver.not_found' event \
         when needs_context names a resolver that is not registered. \
         Log content: {all_log_content:?}"
    );
    assert!(
        all_log_content.contains("unknown"),
        "F-P3-003: 'resolver.not_found' event must carry resolver_name == 'unknown'. \
         Log content: {all_log_content:?}"
    );
    assert!(
        all_log_content.contains("not-found-hook"),
        "F-P3-003: 'resolver.not_found' event must carry the correct hook_name. \
         Log content: {all_log_content:?}"
    );

    // F-S12.04-P7-003: positive-coverage for trace_id + session_id provenance triplet.
    // Catches silent regression of with_trace_id() / with_session_id() in the resolver.not_found
    // event constructor. Mirrors the f_p2_007_resolver_error_test pattern (POL-11).
    // make_executor_inputs sets: session_id = "sess-resolver-test", trace_id = "trace-resolver-test"
    assert!(
        all_log_content.contains("resolver-test-trace")
            || all_log_content.contains("trace-resolver-test"),
        "F-P7-003: resolver.not_found event must include trace_id literal. Log content: {all_log_content:?}"
    );
    assert!(
        all_log_content.contains("sess-resolver-test"),
        "F-P7-003: resolver.not_found event must include session_id literal. Log content: {all_log_content:?}"
    );
    // plugin_name positive-coverage (entry.name)
    assert!(
        all_log_content.contains("not-found-hook"),
        "F-P7-003: resolver.not_found event must include hook plugin_name (entry.name). Log content: {all_log_content:?}"
    );
}

// ---------------------------------------------------------------------------
// F-P2-007 (test 3): ErroringResolver causes resolver.error event in InternalLog
// ---------------------------------------------------------------------------

/// F-P2-007 (integration test) / SOUL #4:
/// When a hook declares `needs_context: ["boom"]` and the resolver "boom"
/// returns `Err(ResolverError::Trap)`, `execute_tiers` must:
/// 1. Complete without panicking (dispatch continues — BC-4.12.005 INV3).
/// 2. Write a `resolver.error` event to InternalLog (no silent failures).
///
/// Verifies that executor.rs wires the error callback correctly in production.
#[tokio::test(flavor = "current_thread")]
async fn f_p2_007_erroring_resolver_causes_resolver_error_event_in_internal_log() {
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());
    let log_dir = dir.path().join("logs");
    let internal_log = Arc::new(InternalLog::new(log_dir.clone()));

    let wasm = compile_wasm(dir.path(), "ok_boom");
    // Hook declares needs_context: ["boom"] — resolver "boom" will error.
    let entry = make_entry(&wasm, "boom-hook", vec!["boom".to_string()]);
    let registry = make_registry(vec![entry]);
    let matched: Vec<&factory_dispatcher::registry::RegistryEntry> =
        registry.hooks.iter().collect();
    let tiers = factory_dispatcher::routing::group_by_priority(&registry, matched);

    let mut resolver_registry = ResolverRegistry::new();
    resolver_registry
        .register(Box::new(ErroringResolver {
            key: "boom".to_string(),
        }))
        .expect("first registration must succeed");
    let resolver_registry = Arc::new(resolver_registry);

    let inputs = make_executor_inputs(&engine, &cache, &registry, &internal_log, resolver_registry);
    // execute_tiers must NOT panic — dispatch continues even with erroring resolver.
    let summary = execute_tiers(inputs, tiers, None).await;

    assert_eq!(
        summary.per_plugin_results.len(),
        1,
        "F-P2-007: exactly one plugin must have run despite erroring resolver"
    );

    // Flush InternalLog and verify resolver.error event is present.
    drop(internal_log);

    // Read the NDJSON log file and look for a resolver.error event.
    let log_entries: Vec<String> = std::fs::read_dir(&log_dir)
        .expect("log dir must exist")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .map(|x| x == "ndjson" || x == "jsonl" || x == "log")
                .unwrap_or(false)
        })
        .flat_map(|e| {
            std::fs::read_to_string(e.path())
                .unwrap_or_default()
                .lines()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect();

    // Also check any file in log_dir (InternalLog may use different extension).
    let all_log_content: String = std::fs::read_dir(&log_dir)
        .expect("log dir must exist after dispatch")
        .filter_map(|e| e.ok())
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .collect::<Vec<_>>()
        .join(
            "
",
        );

    let _ = log_entries; // checked via all_log_content below
    assert!(
        all_log_content.contains("resolver.error"),
        "F-P2-007 / SOUL #4: InternalLog must contain a 'resolver.error' event \
         when a registered resolver returns Err — no silent failures. \
         Log content: {all_log_content:?}"
    );
    // F-P4-001A / F-P5-001: structured error_kind field must be present with
    // the snake_case variant tag matching HOST_ABI.md line 1095.
    assert!(
        all_log_content.contains("error_kind"),
        "F-P4-001A: 'resolver.error' event must contain structured 'error_kind' field \
         (serde tag from ResolverError). Log content: {all_log_content:?}"
    );
    assert!(
        all_log_content.contains("\"error_kind\":\"trap\""),
        "F-P5-001: 'resolver.error' event 'error_kind' must equal 'trap' (snake_case) \
         for a ResolverError::Trap variant — matches HOST_ABI.md line 1095. \
         Log content: {all_log_content:?}"
    );
    // F-P5-002: error_detail (singular) must be present as a Display string.
    assert!(
        all_log_content.contains("error_detail"),
        "F-P5-002: 'resolver.error' event must contain 'error_detail' field (singular). \
         Log content: {all_log_content:?}"
    );
    // F-P6-002: event_type (Claude Code envelope event type) must be present per HOST_ABI.md line 1097.
    // Renamed from hook_event_name (Path A: eliminates dual-semantics with ResolverInput.hook_event_name).
    assert!(
        all_log_content.contains("event_type"),
        "F-P6-002: 'resolver.error' event must contain 'event_type' field (the Claude Code \
         envelope event type, e.g. 'PreToolUse') per HOST_ABI.md line 1097. \
         Log content: {all_log_content:?}"
    );
    // F-P4-005: trace_id positive-coverage assertion. The dispatch context in
    // make_executor_inputs sets dispatcher_trace_id = "resolver-test-trace" (line 119).
    // The resolver.error event MUST include this value — if with_trace_id() is ever
    // accidentally removed, this assertion catches the regression (POL-11 tautology gap).
    assert!(
        all_log_content.contains("resolver-test-trace"),
        "F-P4-005: resolver.error event must include the dispatch trace_id \
         ('resolver-test-trace'). Positive coverage check — ensures with_trace_id() \
         wiring isn't dropped silently in a regression. \
         Log content: {all_log_content:?}"
    );
    // F-S12.04-P5-002: session_id positive-coverage assertion. The base_host_ctx in
    // make_executor_inputs is constructed via HostContext::new("", "0.0.1",
    // "sess-resolver-test", "trace-resolver-test") (line 119). executor.rs:487
    // captures base_host_ctx.session_id as session_id_err and wires it into the
    // resolver.error event via with_session_id(). This assertion ensures that wiring
    // isn't dropped silently in a regression (positive coverage check).
    assert!(
        all_log_content.contains("sess-resolver-test"),
        "F-S12.04-P5-002: resolver.error event must include the dispatch session_id \
         ('sess-resolver-test', from base_host_ctx). Positive coverage check — ensures \
         with_session_id() wiring isn't dropped silently in a regression. \
         Log content: {all_log_content:?}"
    );
    // F-S12.04-P6-002: plugin_name positive-coverage assertion. The hook entry name
    // (set from entry.name and emitted via with_plugin_name()) MUST appear in the
    // resolver.error event. Catches silent regression of .with_plugin_name() being
    // dropped (POL-11 tautology gap).
    let expected_hook_name = "boom-hook";
    assert!(
        all_log_content.contains(expected_hook_name),
        "F-S12.04-P6-002: resolver.error event must include the hook plugin_name (from entry.name). \
         Log content: {all_log_content:?}"
    );
}

// ---------------------------------------------------------------------------
// F-P4-001B / F-P5-003: resolver.merge_collision event carries resolver_name
// ---------------------------------------------------------------------------

/// A resolver whose registry name is distinct from the output key it produces.
/// Used by F-P4-001B / F-P5-003 / F-P2-002 to verify that `resolver_name` in the
/// `resolver.merge_collision` event reflects the declared context_key (merge key).
///
/// F-P2-002: merge uses context_key(), not output.key. This struct overrides
/// context_key() to enable testing a resolver whose declared merge key ("collision-key")
/// differs from its registry name ("test_resolver_alpha") — proving that output.key
/// is informational only.
struct NamedKeyResolver {
    /// Registry name of this resolver (what `name()` returns).
    registry_name: String,
    /// The registry-declared context_key (what `context_key()` returns — the MERGE key).
    /// F-P2-002: this is what determines where output is stored in plugin_config.
    declared_context_key: String,
    /// The output key this resolver produces (informational only per F-P2-002).
    output_key: String,
}

impl ContextResolver for NamedKeyResolver {
    fn name(&self) -> &str {
        &self.registry_name
    }

    fn context_key(&self) -> &str {
        &self.declared_context_key
    }

    fn resolve(&self, _input: &ResolverInput) -> Result<Option<ResolverOutput>, ResolverError> {
        Ok(Some(ResolverOutput {
            key: self.output_key.clone(),
            value: Some(serde_json::json!({ "resolved": true })),
        }))
    }
}

/// F-P4-001B / F-P5-003 / F-P3-001 (integration test):
/// When a resolver output key collides with a static config key,
/// `execute_tiers` must emit a `resolver.merge_collision` event that
/// carries the `resolver_name` field reflecting the REGISTRY NAME of the
/// resolver — proving F-P3-001 resolver name threading works end-to-end
/// (BC-4.12.004 wire format).
///
/// Setup:
/// - Resolver registry name:          "test_resolver_alpha"
/// - Resolver declared context_key:   "collision-key" (the merge key per F-P2-002)
/// - Resolver output.key:             "collision-key-output" (informational only)
/// - Static config key:               "collision-key" (causes collision with context_key)
///
/// F-P3-001: resolver_name in collision event = registry NAME ("test_resolver_alpha"),
/// NOT context_key ("collision-key"). The collision key field carries the context_key.
#[tokio::test(flavor = "current_thread")]
async fn f_p4_001b_merge_collision_event_carries_resolver_name() {
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());
    let log_dir = dir.path().join("logs");
    let internal_log = Arc::new(InternalLog::new(log_dir.clone()));

    let wasm = compile_wasm(dir.path(), "ok_collision");
    // Build entry with static config key "collision-key" so the resolver output collides.
    let mut toml_map = toml::Table::new();
    toml_map.insert(
        "collision-key".to_string(),
        toml::Value::String("static".to_string()),
    );
    let entry = RegistryEntry {
        name: "collision-hook".to_string(),
        event: "PreToolUse".to_string(),
        tool: None,
        plugin: wasm,
        priority: Some(100),
        enabled: true,
        timeout_ms: Some(2_000),
        fuel_cap: Some(1_000_000_000),
        on_error: None,
        capabilities: Some(Capabilities::default()),
        config: toml::Value::Table(toml_map),
        async_flag: false,
        // Declare "test_resolver_alpha" in needs_context — matches the registry name below.
        needs_context: vec!["test_resolver_alpha".to_string()],
        failure_policy: Default::default(),
    };
    let registry = make_registry(vec![entry]);
    let matched: Vec<&factory_dispatcher::registry::RegistryEntry> =
        registry.hooks.iter().collect();
    let tiers = factory_dispatcher::routing::group_by_priority(&registry, matched);

    // F-P5-003 / F-P2-002: resolver registry name "test_resolver_alpha", declared
    // context_key "collision-key" (the merge key), output_key "collision-key" (informational).
    // The declared context_key determines the merge key; output.key is informational.
    let resolver = NamedKeyResolver {
        registry_name: "test_resolver_alpha".to_string(),
        declared_context_key: "collision-key".to_string(), // merge key = context_key (F-P2-002)
        output_key: "collision-key-output".to_string(),    // informational only (F-P2-002)
    };
    let mut resolver_registry = ResolverRegistry::new();
    resolver_registry
        .register(Box::new(resolver))
        .expect("registration must succeed");
    let resolver_registry = Arc::new(resolver_registry);

    let inputs = make_executor_inputs(&engine, &cache, &registry, &internal_log, resolver_registry);
    let summary = execute_tiers(inputs, tiers, None).await;

    assert_eq!(
        summary.per_plugin_results.len(),
        1,
        "F-P4-001B: one plugin must have run"
    );

    drop(internal_log);

    let all_log_content: String = std::fs::read_dir(&log_dir)
        .expect("log dir must exist after dispatch")
        .filter_map(|e| e.ok())
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .collect::<Vec<_>>()
        .join(
            "
",
        );

    assert!(
        all_log_content.contains("resolver.merge_collision"),
        "F-P4-001B: InternalLog must contain a 'resolver.merge_collision' event. \
         Log content: {all_log_content:?}"
    );
    assert!(
        all_log_content.contains("resolver_name"),
        "F-P4-001B: 'resolver.merge_collision' event must carry 'resolver_name' field \
         (BC-4.12.004 wire format). Log content: {all_log_content:?}"
    );
    // F-P3-001: assert resolver_name VALUE is "test_resolver_alpha" (the registry NAME).
    // With F-P3-001, resolver_name in the collision event = registry name (ContextResolver::name()),
    // NOT the context_key. The context_key is "collision-key" (the merge key);
    // the registry name is "test_resolver_alpha" — these are distinct.
    assert!(
        all_log_content.contains("\"resolver_name\":\"test_resolver_alpha\""),
        "F-P3-001: 'resolver_name' in merge_collision event must equal \
         'test_resolver_alpha' (the registry NAME, not the context_key 'collision-key'). \
         Log content: {all_log_content:?}"
    );
    // The collision key itself must be "collision-key" (the context_key used for merge).
    assert!(
        all_log_content.contains("\"key\":\"collision-key\""),
        "F-P3-001: 'key' in merge_collision event must equal 'collision-key' \
         (the context_key = merge key). Log content: {all_log_content:?}"
    );
    // F-P8-002: provenance-triplet positive-coverage for resolver.merge_collision.
    // make_executor_inputs sets trace_id = "trace-resolver-test", session_id = "sess-resolver-test".
    // entry.name = "collision-hook" (plugin_name in the event).
    // Matches the pattern established by F-P7-003 for resolver.not_found.
    assert!(
        all_log_content.contains("trace-resolver-test"),
        "F-P8-002: resolver.merge_collision event must include trace_id literal \
         ('trace-resolver-test'). Positive coverage check — ensures with_trace_id() \
         wiring is not silently dropped. Log content: {all_log_content:?}"
    );
    assert!(
        all_log_content.contains("sess-resolver-test"),
        "F-P8-002: resolver.merge_collision event must include session_id literal \
         ('sess-resolver-test'). Positive coverage check — ensures with_session_id() \
         wiring is not silently dropped. Log content: {all_log_content:?}"
    );
    assert!(
        all_log_content.contains("collision-hook"),
        "F-P8-002: resolver.merge_collision event must include plugin_name ('collision-hook', \
         from entry.name). Positive coverage check — ensures with_plugin_name() \
         wiring is not silently dropped. Log content: {all_log_content:?}"
    );
}

// ---------------------------------------------------------------------------
// F-P5-005: ResolverInput shape test (spy resolver captures ResolverInput)
// ---------------------------------------------------------------------------

/// Spy resolver: captures the `ResolverInput` it receives for inspection.
struct SpyResolver {
    captured: Arc<Mutex<Option<ResolverInput>>>,
}

impl ContextResolver for SpyResolver {
    fn name(&self) -> &str {
        "spy"
    }

    fn resolve(&self, input: &ResolverInput) -> Result<Option<ResolverOutput>, ResolverError> {
        *self.captured.lock().unwrap() = Some(input.clone());
        Ok(Some(ResolverOutput {
            key: "spy_data".into(),
            value: Some(serde_json::json!({"ok": true})),
        }))
    }
}

/// F-P5-005: Verify the shape of `ResolverInput` received by a resolver.
///
/// The test confirms:
/// - `event_type` is the Claude Code event type from the payload (e.g. "PreToolUse").
/// - `hook_event_name` is the registry entry name (executor.rs convention: entry.name).
/// - `agent_type` is None when absent from the payload.
/// - `project_dir` is derived from base_host_ctx.cwd.
///
/// F-P5-005 resolution: `hook_event_name == entry.name` (registry entry name).
/// This is documented in executor.rs `build_plugin_config` and here as the
/// canonical convention: the hook_event_name in ResolverInput carries the
/// hooks-registry entry name, NOT the Claude Code envelope event type.
/// (The Claude Code envelope event type is in `event_type`.)
#[tokio::test(flavor = "current_thread")]
async fn f_p5_005_resolver_receives_correct_resolverinput_shape() {
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());
    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));

    let wasm = compile_wasm(dir.path(), "ok_spy");
    let entry = make_entry(&wasm, "my-hook-entry", vec!["spy".to_string()]);
    let registry = make_registry(vec![entry]);
    let matched: Vec<&RegistryEntry> = registry.hooks.iter().collect();
    let tiers = group_by_priority(&registry, matched);

    let captured = Arc::new(Mutex::new(None::<ResolverInput>));
    let spy = SpyResolver {
        captured: captured.clone(),
    };
    let mut resolver_registry = ResolverRegistry::new();
    resolver_registry
        .register(Box::new(spy))
        .expect("spy registration must succeed");
    let resolver_registry = Arc::new(resolver_registry);

    // Payload uses "hook_event_name" (canonical Claude Code envelope field).
    let mut base_ctx = HostContext::new("/test/project", "0.0.1", "sess-spy", "trace-spy");
    base_ctx.internal_log = Some(internal_log.clone());
    let inputs = ExecutorInputs {
        engine: &engine,
        cache: &cache,
        registry: &registry,
        payload_value: serde_json::json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Write",
            "session_id": "spy-session",
            "dispatcher_trace_id": "spy-trace"
        }),
        base_host_ctx: base_ctx.clone(),
        internal_log: internal_log.clone(),
        resolver_registry,
    };
    execute_tiers(inputs, tiers, None).await;

    let captured_input = captured.lock().unwrap().clone();
    let input = captured_input.expect("SpyResolver must have been invoked");

    // F-P5-005: event_type comes from hook_event_name fallback (no event_name in payload).
    assert_eq!(
        input.event_type, "PreToolUse",
        "F-P5-005: ResolverInput.event_type must be the Claude Code event type \
         from the payload (extracted from 'hook_event_name' when 'event_name' is absent)"
    );

    // F-P5-005 convention: hook_event_name in ResolverInput == registry entry name.
    assert_eq!(
        input.hook_event_name, "my-hook-entry",
        "F-P5-005: ResolverInput.hook_event_name must be the registry entry name \
         (entry.name == 'my-hook-entry'), NOT the Claude Code event type. \
         This is the executor.rs convention documented in build_plugin_config."
    );

    // agent_type absent from payload → None.
    assert_eq!(
        input.agent_type, None,
        "F-P5-005: ResolverInput.agent_type must be None when absent from payload"
    );

    // project_dir from base_host_ctx.cwd.
    assert_eq!(
        input.project_dir,
        base_ctx.cwd.to_str().unwrap_or(""),
        "F-P5-005: ResolverInput.project_dir must equal base_host_ctx.cwd"
    );
}

// ---------------------------------------------------------------------------
// F-P5-006: payload field extraction order (hook_event_name fallback)
// ---------------------------------------------------------------------------

/// F-P5-006: Verify executor.rs extracts event_type correctly when only
/// `hook_event_name` is present in the payload (no `event_name`).
///
/// This exercises the fallback path in executor.rs lines 432-437.
/// The current extraction order is: `event_name` OR `hook_event_name`.
/// When only `hook_event_name` is set (canonical Claude Code envelope),
/// the fallback must produce the correct value.
///
/// If this test passes, the fallback works for real Claude Code envelopes.
/// If it fails, it reveals a bug in the extraction order — which must then
/// be fixed in executor.rs.
#[tokio::test(flavor = "current_thread")]
async fn f_p5_006_payload_field_extraction_falls_back_to_hook_event_name() {
    let dir = tempfile::tempdir().unwrap();
    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());
    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));

    let wasm = compile_wasm(dir.path(), "ok_spy2");
    let entry = make_entry(&wasm, "extract-hook", vec!["spy2".to_string()]);
    let registry = make_registry(vec![entry]);
    let matched: Vec<&RegistryEntry> = registry.hooks.iter().collect();
    let tiers = group_by_priority(&registry, matched);

    let captured = Arc::new(Mutex::new(None::<ResolverInput>));
    struct SpyResolver2 {
        captured: Arc<Mutex<Option<ResolverInput>>>,
    }
    impl ContextResolver for SpyResolver2 {
        fn name(&self) -> &str {
            "spy2"
        }
        fn resolve(&self, input: &ResolverInput) -> Result<Option<ResolverOutput>, ResolverError> {
            *self.captured.lock().unwrap() = Some(input.clone());
            Ok(Some(ResolverOutput {
                key: "spy2_data".into(),
                value: Some(serde_json::json!({"ok": true})),
            }))
        }
    }
    let spy2 = SpyResolver2 {
        captured: captured.clone(),
    };
    let mut resolver_registry = ResolverRegistry::new();
    resolver_registry
        .register(Box::new(spy2))
        .expect("spy2 registration");
    let resolver_registry = Arc::new(resolver_registry);

    // Payload with ONLY hook_event_name (no event_name) — real Claude Code envelope.
    let mut base_ctx = HostContext::new("", "0.0.1", "sess-extract", "trace-extract");
    base_ctx.internal_log = Some(internal_log.clone());
    let inputs = ExecutorInputs {
        engine: &engine,
        cache: &cache,
        registry: &registry,
        payload_value: serde_json::json!({
            "hook_event_name": "PostToolUse",
            "tool_name": "Read",
            "session_id": "extract-session",
            "dispatcher_trace_id": "extract-trace"
        }),
        base_host_ctx: base_ctx,
        internal_log: internal_log.clone(),
        resolver_registry,
    };
    execute_tiers(inputs, tiers, None).await;

    let captured_input = captured.lock().unwrap().clone();
    let input = captured_input.expect("SpyResolver2 must have been invoked");

    // F-P5-006: fallback extraction from hook_event_name works correctly.
    assert_eq!(
        input.event_type, "PostToolUse",
        "F-P5-006: executor.rs event_type extraction must fall back to \
         'hook_event_name' when 'event_name' is absent in the payload. \
         Proves the fallback path works for real Claude Code envelopes."
    );
}
