// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Resolver error-isolation tests (S-12.04 AC-010, AC-011).
//!
//! Verifies that a trapping WASM resolver does not abort the dispatch
//! cycle (AC-010) and that a `resolver.error` telemetry event is emitted for
//! each trap (AC-011), per BC-4.12.004 crash isolation contract.
//!
//! Uses the compiled `tests/fixtures/trapping_resolver.wasm` artifact whose
//! `resolve()` function immediately executes `unreachable`. This fixture was
//! compiled from `tests/fixtures/trapping_resolver.wat` during S-12.04 Step 2.
//!
//! All test bodies are fully authored per S-12.04 Step 2. Every test MUST
//! FAIL before Step 3 implementation (Red Gate per BC-5.38.001) because the
//! production WASM invocation path is not yet wired — the in-process
//! `ContextResolver` trait does not yet dispatch through wasmtime.
//!
//! Architecture anchors:
//! - BC-4.12.004 — resolver crash isolation contract
//! - VP-074 — formal verification target for crash isolation
//! - S-12.04 AC-010 (trap isolation), AC-011 (trap event emission)

use std::sync::{Arc, Mutex};

use factory_dispatcher::engine::build_engine;
use factory_dispatcher::executor::{ExecutorInputs, execute_tiers};
use factory_dispatcher::host::HostContext;
use factory_dispatcher::internal_log::InternalLog;
use factory_dispatcher::plugin_loader::PluginCache;
use factory_dispatcher::registry::{Capabilities, Registry, RegistryEntry};
use factory_dispatcher::resolver::{
    ContextResolver, ResolverError, ResolverInput, ResolverOutput, ResolverRegistry,
};
use factory_dispatcher::resolver_loader::ResolverLoader;
use factory_dispatcher::routing::group_by_priority;

// ---------------------------------------------------------------------------
// WAT fixture: a minimal working plugin (does not trap; returns exit 0)
// ---------------------------------------------------------------------------

/// Minimal WASM plugin for dispatcher integration. Not the resolver fixture —
/// this is the *hook plugin* that runs after resolver context is injected.
const WAT_OK_PLUGIN: &str = r#"
(module
  (memory (export "memory") 1)
  (func (export "_start")))
"#;

// ---------------------------------------------------------------------------
// Path to the trapping resolver WASM fixture
// ---------------------------------------------------------------------------

fn trapping_resolver_wasm() -> std::path::PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    std::path::Path::new(manifest).join("tests/fixtures/trapping_resolver.wasm")
}

// F-P2-004: TrappingWasmResolver (in-process proxy) and GoodResolver were removed.
// The AC-006/008/009 integration tests now use real wasmtime invocation via
// ResolverLoader::load_registry + execute_tiers end-to-end. The proxy was only
// needed before the real WASM invocation path was wired in Step 3.

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a `ResolverRegistry` by loading `trapping_resolver.wasm` via the real
/// `ResolverLoader::load_registry` path. The registry name is "trap-resolver" and
/// the context_key is "trap-output" — matching the `needs_context` declaration.
///
/// F-P2-004: AC-006/008/009 integration tests must use the real wasmtime trap path,
/// not the in-process TrappingWasmResolver proxy.
fn load_real_trapping_resolver_registry(
    engine: &wasmtime::Engine,
    tempdir: &std::path::Path,
) -> Arc<ResolverRegistry> {
    let fixture = trapping_resolver_wasm();
    assert!(
        fixture.exists(),
        "F-P2-004: trapping_resolver.wasm must exist at {:?}",
        fixture
    );

    // Windows: PathBuf::display() produces backslashes which TOML parses as escape sequences.
    // Forward-slash the path so it is valid TOML on all platforms (F-WIN-001).
    let fixture_toml = fixture.to_string_lossy().replace('\\', "/");
    let toml_content = format!(
        r#"schema_version = 1

[[resolvers]]
name = "trap-resolver"
plugin = "{fixture_toml}"
context_key = "trap-output"
"#
    );
    let registry_path = tempdir.join("trapping-resolvers-registry.toml");
    std::fs::write(&registry_path, &toml_content).expect("write trapping resolver registry TOML");

    let loader = ResolverLoader::new(engine.clone());
    let (registry, _warnings) = loader
        .load_registry(&registry_path)
        .expect("F-P2-004: load_registry must succeed (loading does not invoke resolve)");
    Arc::new(registry)
}

fn compile_ok_plugin(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let bytes = wat::parse_str(WAT_OK_PLUGIN).expect("WAT parse");
    let path = dir.join(format!("{name}.wasm"));
    std::fs::write(&path, bytes).unwrap();
    path
}

fn make_hook_entry(
    plugin_path: &std::path::Path,
    entry_name: &str,
    needs_context: Vec<String>,
) -> RegistryEntry {
    RegistryEntry {
        name: entry_name.to_string(),
        event: "PreToolUse".to_string(),
        tool: None,
        plugin: plugin_path.to_path_buf(),
        priority: Some(100),
        enabled: true,
        timeout_ms: Some(5_000),
        fuel_cap: Some(1_000_000_000),
        on_error: None,
        capabilities: Some(Capabilities::default()),
        config: toml::Value::Table(toml::Table::new()),
        async_flag: false,
        needs_context,
        failure_policy: Default::default(),
    }
}

fn make_registry_with_hooks(hooks: Vec<RegistryEntry>) -> Registry {
    Registry {
        schema_version: 2,
        defaults: Default::default(),
        hooks,
    }
}

fn make_executor_inputs<'a>(
    engine: &'a wasmtime::Engine,
    cache: &'a PluginCache,
    registry: &'a Registry,
    internal_log: &Arc<InternalLog>,
    resolver_registry: Arc<ResolverRegistry>,
) -> ExecutorInputs<'a> {
    let mut base = HostContext::new("", "0.0.1", "sess-trap-test", "trace-trap-test");
    base.internal_log = Some(internal_log.clone());
    // F-P3-004: project_dir must not be empty (BC-4.12.003 INV4).
    // Use /tmp as the test project dir — a real path that always exists.
    base.cwd = std::path::PathBuf::from("/tmp");
    ExecutorInputs {
        engine,
        cache,
        registry,
        payload_value: serde_json::json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Write",
            "session_id": "trap-test-session",
            "dispatcher_trace_id": "trap-test-trace"
        }),
        base_host_ctx: base,
        internal_log: internal_log.clone(),
        resolver_registry,
    }
}

// ---------------------------------------------------------------------------
// AC-010 (integration) — trapping resolver does NOT abort dispatch
//
// BC-4.12.004 postconditions 1–2 / VP-074:
// A resolver that traps must not propagate the trap to the dispatch cycle.
// The dispatch must complete (return Ok), and the trapping resolver's key
// must be ABSENT from plugin_config.
// ---------------------------------------------------------------------------

/// test_BC_4_12_004_trapping_resolver_does_not_abort_dispatch
///
/// F-P2-004: Loads `trapping_resolver.wasm` via `ResolverLoader::load_registry`,
/// dispatches a hook that declares `needs_context: ["trap-resolver"]`, and asserts:
/// 1. `execute_tiers` returns without panicking (dispatch completes).
/// 2. The plugin runs (per_plugin_results has one entry).
/// 3. The real wasmtime trap path (not the in-process proxy) is exercised.
///
/// This test uses the real WASM invocation path (BC-4.12.004 crash isolation).
#[tokio::test(flavor = "current_thread")]
async fn test_BC_4_12_004_trapping_resolver_does_not_abort_dispatch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let engine = build_engine().expect("build_engine");
    let cache = PluginCache::new(engine.clone());
    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));

    // F-P2-004: load via real ResolverLoader::load_registry (not in-process proxy).
    let resolver_registry = load_real_trapping_resolver_registry(&engine, dir.path());

    let plugin = compile_ok_plugin(dir.path(), "trap_dispatch_plugin");
    let entry = make_hook_entry(
        &plugin,
        "trap-resolver-hook",
        vec!["trap-resolver".to_string()],
    );
    let registry = make_registry_with_hooks(vec![entry]);
    let matched: Vec<&RegistryEntry> = registry.hooks.iter().collect();
    let tiers = group_by_priority(&registry, matched);

    let inputs = make_executor_inputs(&engine, &cache, &registry, &internal_log, resolver_registry);

    // execute_tiers must NOT panic — BC-4.12.004 crash isolation contract.
    // The real wasmtime trap from unreachable must be caught, not propagated.
    let summary = execute_tiers(inputs, tiers, None).await;

    assert_eq!(
        summary.per_plugin_results.len(),
        1,
        "AC-010 / BC-4.12.004 PC1 / F-P2-004: exactly one hook plugin must have run — \
         a real wasmtime trap must not prevent hook dispatch from completing"
    );
}

// ---------------------------------------------------------------------------
// AC-011 (integration) — trapping resolver emits resolver.error event
//
// BC-4.12.004 postconditions 3–5 / VP-074:
// Every resolver trap must be observable: the dispatcher must emit a
// resolver.error event with error_kind="trap", resolver_name, and error_detail.
// ---------------------------------------------------------------------------

/// test_BC_4_12_004_trapping_resolver_emits_resolver_error_event
///
/// F-P2-004: Loads `trapping_resolver.wasm` via `ResolverLoader::load_registry`,
/// dispatches a hook with the real trapping resolver, and asserts the InternalLog
/// contains a `resolver.error` event with:
/// - `error_kind == "trap"`
/// - `resolver_name` matching the registered resolver name
/// - `error_detail` non-empty (carries trap context)
///
/// This test verifies the executor.rs `emit_resolver_error` callback wiring
/// through the real wasmtime trap path (not the in-process proxy).
#[tokio::test(flavor = "current_thread")]
async fn test_BC_4_12_004_trapping_resolver_emits_resolver_error_event() {
    let dir = tempfile::tempdir().expect("tempdir");
    let engine = build_engine().expect("build_engine");
    let cache = PluginCache::new(engine.clone());
    let log_dir = dir.path().join("logs");
    let internal_log = Arc::new(InternalLog::new(log_dir.clone()));

    // F-P2-004: load via real ResolverLoader::load_registry.
    let resolver_registry = load_real_trapping_resolver_registry(&engine, dir.path());

    let plugin = compile_ok_plugin(dir.path(), "trap_event_plugin");
    let entry = make_hook_entry(
        &plugin,
        "trap-event-hook",
        vec!["trap-resolver".to_string()],
    );
    let registry = make_registry_with_hooks(vec![entry]);
    let matched: Vec<&RegistryEntry> = registry.hooks.iter().collect();
    let tiers = group_by_priority(&registry, matched);

    let inputs = make_executor_inputs(&engine, &cache, &registry, &internal_log, resolver_registry);
    let _summary = execute_tiers(inputs, tiers, None).await;

    // Flush the InternalLog to disk.
    drop(internal_log);

    // Read all log files from the log directory.
    let all_log_content: String = std::fs::read_dir(&log_dir)
        .expect("AC-011: log dir must exist after dispatch")
        .filter_map(|e| e.ok())
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .collect::<Vec<_>>()
        .join(
            "
",
        );

    // AC-011 assertion 1: resolver.error event present.
    assert!(
        all_log_content.contains("resolver.error"),
        "AC-011 / BC-4.12.004 PC4 / SOUL #4: InternalLog must contain a \
         'resolver.error' event when a resolver traps — no silent failures. \
         Log content: {all_log_content:?}"
    );

    // AC-011 assertion 2: error_kind == "trap" (snake_case serde tag from ResolverError::Trap).
    assert!(
        all_log_content.contains("\"error_kind\":\"trap\""),
        "AC-011 / BC-4.12.004 PC4: 'resolver.error' event must carry \
         'error_kind': 'trap' (serde snake_case tag for ResolverError::Trap). \
         Log content: {all_log_content:?}"
    );

    // AC-011 assertion 3: resolver_name present and correct.
    assert!(
        all_log_content.contains("trap-resolver"),
        "AC-011 / BC-4.12.004 PC4: 'resolver.error' event must carry the \
         resolver name 'trap-resolver' in the resolver_name field. \
         Log content: {all_log_content:?}"
    );

    // AC-011 assertion 4: error_detail present and non-empty.
    assert!(
        all_log_content.contains("error_detail"),
        "AC-011 / BC-4.12.004 PC4: 'resolver.error' event must carry \
         'error_detail' (non-empty trap context string). \
         Log content: {all_log_content:?}"
    );

    // F-S12.04-P9-001: provenance-triplet positive-coverage parallel to F-P7-003 + F-P8-002.
    // Catches silent regression of with_trace_id() / with_session_id() / with_plugin_name()
    // in the resolver.error event constructor at executor.rs:553-560.
    // make_executor_inputs (line 146) sets: session_id = "sess-trap-test", trace_id = "trace-trap-test".
    // entry.name = "trap-event-hook" (set at line 250-253).
    assert!(
        all_log_content.contains("trace-trap-test"),
        "F-P9-001: resolver.error event must include trace_id literal \
         ('trace-trap-test'). Positive coverage check — ensures with_trace_id() \
         wiring is not silently dropped. Log content: {all_log_content:?}"
    );
    assert!(
        all_log_content.contains("sess-trap-test"),
        "F-P9-001: resolver.error event must include session_id literal \
         ('sess-trap-test'). Positive coverage check — ensures with_session_id() \
         wiring is not silently dropped. Log content: {all_log_content:?}"
    );
    assert!(
        all_log_content.contains("trap-event-hook"),
        "F-P9-001: resolver.error event must include plugin_name ('trap-event-hook', \
         from entry.name). Positive coverage check — ensures with_plugin_name() \
         wiring is not silently dropped. Log content: {all_log_content:?}"
    );
}

// ---------------------------------------------------------------------------
// AC-008 (integration) — failed resolver key ABSENT from plugin_config
//
// BC-4.12.004 postcondition 3:
// When a resolver fails (trap), the dispatcher must NOT write the resolver's
// key into plugin_config. The key must be absent — not null, not {}.
//
// Note: plugin_config is passed into the hook WASM as part of the dispatch
// payload. This test verifies the merge step (executor.rs build_plugin_config)
// does not include the failed resolver's output.
// ---------------------------------------------------------------------------

/// test_BC_4_12_004_failed_resolver_key_absent_from_plugin_config
///
/// Dispatches a hook with a trapping resolver. The trapping resolver is
/// named "trap-resolver". After dispatch, the test verifies the dispatch
/// completed (no panic), and that the resolver's key was excluded from the
/// merged plugin_config (verified via a SpyResolver that inspects the config
/// it receives — the spy should NOT see "trap-resolver" as a plugin_config key
/// because the trapping resolver's output was never merged).
///
/// Red Gate: fails because the resolver error path (key-absent contract) is
/// not yet wired in the stub implementation.
/// test_BC_4_12_004_failed_resolver_key_absent_from_plugin_config
///
/// F-P2-004: Loads `trapping_resolver.wasm` via `ResolverLoader::load_registry`,
/// dispatches a hook with BOTH the real trapping resolver and an in-process
/// good-resolver. Asserts:
/// 1. "trap-output" key (the context_key of the trapping resolver) is ABSENT.
/// 2. The good-resolver was invoked (resolver B after A failed — AC-009).
/// 3. The hook plugin ran (dispatch not aborted — AC-008).
///
/// Uses real wasmtime trap path per F-P2-004 requirement.
#[tokio::test(flavor = "current_thread")]
async fn test_BC_4_12_004_failed_resolver_key_absent_from_plugin_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let engine = build_engine().expect("build_engine");
    let cache = PluginCache::new(engine.clone());
    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));

    let plugin = compile_ok_plugin(dir.path(), "absent_key_plugin");
    // Hook requests both "trap-resolver" (will fail via real wasmtime trap) and
    // "good-resolver" (in-process, will succeed). AC-009: B executes after A fails.
    let entry = make_hook_entry(
        &plugin,
        "absent-key-hook",
        vec!["trap-resolver".to_string(), "good-resolver".to_string()],
    );
    let registry = make_registry_with_hooks(vec![entry]);
    let matched: Vec<&RegistryEntry> = registry.hooks.iter().collect();
    let tiers = group_by_priority(&registry, matched);

    // Track what plugin_config the good resolver sees — it will NOT include
    // "trap-output" key (the trapping resolver's context_key is "trap-output",
    // and it trapped so no output was produced).
    let captured_plugin_config: Arc<Mutex<Option<serde_json::Value>>> = Arc::new(Mutex::new(None));
    let config_capture = captured_plugin_config.clone();

    struct CapturingGoodResolver {
        captured: Arc<Mutex<Option<serde_json::Value>>>,
    }
    impl ContextResolver for CapturingGoodResolver {
        fn name(&self) -> &str {
            "good-resolver"
        }
        fn resolve(&self, input: &ResolverInput) -> Result<Option<ResolverOutput>, ResolverError> {
            // Capture what plugin_config the executor passed in (pre-merge static config).
            *self.captured.lock().unwrap() = Some(input.plugin_config.clone());
            Ok(Some(ResolverOutput {
                key: "good-key".to_string(),
                value: Some(serde_json::json!(42)),
            }))
        }
    }

    let good = CapturingGoodResolver {
        captured: config_capture,
    };

    // Build a combined registry: real trapping resolver + in-process good-resolver.
    // We can't mutate Arc<ResolverRegistry>, so re-build from the loaded resolvers.
    let fixture = trapping_resolver_wasm();
    // Windows: PathBuf::display() produces backslashes which TOML parses as escape sequences.
    // Forward-slash the path so it is valid TOML on all platforms (F-WIN-001).
    let fixture_toml = fixture.to_string_lossy().replace('\\', "/");
    let toml_content = format!(
        r#"schema_version = 1

[[resolvers]]
name = "trap-resolver"
plugin = "{fixture_toml}"
context_key = "trap-output"
"#
    );
    let registry_path = dir.path().join("combined-resolvers-registry.toml");
    std::fs::write(&registry_path, &toml_content).expect("write registry TOML");
    let loader = ResolverLoader::new(engine.clone());
    let (mut resolver_registry, _warnings) = loader
        .load_registry(&registry_path)
        .expect("F-P2-004: load_registry must succeed");
    resolver_registry
        .register(Box::new(good))
        .expect("good-resolver registration");
    let resolver_registry = Arc::new(resolver_registry);

    let inputs = make_executor_inputs(&engine, &cache, &registry, &internal_log, resolver_registry);
    let summary = execute_tiers(inputs, tiers, None).await;

    assert_eq!(
        summary.per_plugin_results.len(),
        1,
        "AC-008: hook plugin must have run despite the trapping resolver"
    );

    // The good resolver must have been invoked (AC-009: resolver B executes after A fails).
    let config = captured_plugin_config.lock().unwrap().clone();
    let config = config.expect(
        "AC-008 / AC-009: good-resolver must have been invoked — \
         resolver B must execute even when resolver A fails (BC-4.12.004 PC6)",
    );

    // AC-008: "trap-resolver" key must NOT appear in plugin_config.
    // It was never merged because the trapping resolver returned Err.
    let config_obj = config
        .as_object()
        .expect("plugin_config must be a JSON object");

    // AC-008: "trap-output" (the context_key declared in resolvers-registry.toml) must
    // NOT appear in plugin_config — the trapping resolver produced no output (it trapped).
    // F-P2-002: merge key = context_key = "trap-output", not the resolver name "trap-resolver".
    assert!(
        config_obj.get("trap-output").is_none(),
        "AC-008 / BC-4.12.004 PC3 / F-P2-004: 'trap-output' (the resolver's context_key) \
         must be ABSENT from plugin_config after the real wasmtime trap — not null, not {{}}, \
         not any default value. Found: {:?}",
        config_obj.get("trap-output")
    );

    // AC-009: good-resolver was invoked (resolver B executed after resolver A failed).
    //
    // The `config.expect(...)` call above already proves invocation — if
    // good-resolver had NOT been called, `captured_plugin_config` would be
    // `None` and the test would have failed there.
    //
    // Additionally: "good-key" must NOT be present in the input config that
    // good-resolver received, because "good-key" is good-resolver's own output
    // value — it cannot appear in the input passed TO good-resolver.
    assert!(
        config_obj.get("good-key").is_none(),
        "AC-009 / BC-4.12.004 PC6: 'good-key' is good-resolver's own output key — \
         it must NOT be present in the input plugin_config passed to good-resolver \
         (a resolver cannot see its own output as input). \
         good-resolver was invoked (proven by config.expect above). \
         Found: {:?}",
        config_obj.get("good-key")
    );
}

// ---------------------------------------------------------------------------
// F-P1-001 (CRITICAL) — end-to-end test: real WASM trap fixture via
// ResolverLoader → invoke_resolver_wasm → ResolverError::Trap
//
// BC-4.12.004 postconditions 1–4 / VP-074:
// Loading trapping_resolver.wasm via ResolverLoader::load_registry and
// then invoking it via ResolverRegistry::invoke_resolver_wasm must:
// 1. Return Err(ResolverError::Trap) — the wasmtime Unreachable trap
//    is classified and returned, not propagated.
// 2. NOT panic.
// 3. Carry the resolver name and a non-empty detail string.
// ---------------------------------------------------------------------------

/// test_F_P1_001_invoke_resolver_wasm_real_wasm_fixture_surfaces_trap
///
/// Loads `trapping_resolver.wasm` via `ResolverLoader::load_registry` using
/// an in-memory TOML, then calls `ResolverRegistry::invoke_resolver_wasm`
/// with the resolver name and asserts:
/// 1. The call returns `Err(ResolverError::Trap)` — the wasmtime trap
///    from `unreachable` is caught, classified, and returned.
/// 2. The function does NOT panic (trap isolation — BC-4.12.004 INV1).
/// 3. The `Trap.name` field equals the registered resolver name.
/// 4. The `Trap.detail` field is non-empty (VP-074: observable trap context).
///
/// This is the end-to-end proof that `invoke_resolver_wasm` exercises the
/// real wasmtime boundary and surfaces `ResolverError::Trap`.
#[test]
fn test_F_P1_001_invoke_resolver_wasm_real_wasm_fixture_surfaces_trap() {
    use factory_dispatcher::resolver::ResolverInput;
    use factory_dispatcher::resolver_loader::ResolverLoader;

    let fixture = trapping_resolver_wasm();
    assert!(
        fixture.exists(),
        "F-P1-001: trapping_resolver.wasm must exist at {:?}",
        fixture
    );

    let engine = build_engine().expect("F-P1-001: build_engine must succeed");

    // Write a minimal resolvers-registry.toml pointing at the fixture.
    let dir = tempfile::tempdir().expect("tempdir");
    // Windows: PathBuf::display() produces backslashes which TOML parses as escape sequences.
    // Forward-slash the path so it is valid TOML on all platforms (F-WIN-001).
    let fixture_toml = fixture.to_string_lossy().replace('\\', "/");
    let toml_content = format!(
        r#"schema_version = 1

[[resolvers]]
name = "trap-wasm-resolver"
plugin = "{fixture_toml}"
context_key = "trap_context"
"#
    );
    let registry_path = dir.path().join("resolvers-registry.toml");
    std::fs::write(&registry_path, toml_content).expect("write registry TOML");

    // Load the registry — this compiles trapping_resolver.wasm.
    let loader = ResolverLoader::new(engine);
    let (registry, _warnings) = loader
        .load_registry(&registry_path)
        .expect("F-P1-001: load_registry must succeed (load does not invoke resolve)");

    assert_eq!(
        registry.len(),
        1,
        "F-P1-001: registry must have exactly 1 resolver after loading trapping_resolver.wasm"
    );

    // Build a minimal ResolverInput.
    let input = ResolverInput {
        event_type: "PreToolUse".to_string(),
        hook_event_name: "test-hook".to_string(),
        agent_type: None,
        project_dir: dir.path().to_str().unwrap_or("").to_string(),
        plugin_config: serde_json::json!({}),
    };

    // Invoke via invoke_resolver_wasm — the trapping resolver executes
    // `unreachable` and the trap must be caught and returned as ResolverError::Trap.
    let result = registry.invoke_resolver_wasm_for_testing("trap-wasm-resolver", &input);

    match result {
        Err(factory_dispatcher::resolver::ResolverError::Trap { name, detail }) => {
            assert_eq!(
                name, "trap-wasm-resolver",
                "F-P1-001 / VP-074: Trap.name must equal the registered resolver name"
            );
            assert!(
                !detail.is_empty(),
                "F-P1-001 / VP-074: Trap.detail must be non-empty — carry wasmtime trap context"
            );
        }
        Err(other) => {
            panic!(
                "F-P1-001 / BC-4.12.004: expected ResolverError::Trap from trapping_resolver.wasm \
                 but got {:?} — trap must be caught and classified, not returned as a different error",
                other
            );
        }
        Ok(_) => {
            panic!(
                "F-P1-001 / BC-4.12.004: invoke_resolver_wasm on trapping_resolver.wasm must \
                 return Err(ResolverError::Trap), not Ok — the WASM executes `unreachable`"
            );
        }
    }
}
