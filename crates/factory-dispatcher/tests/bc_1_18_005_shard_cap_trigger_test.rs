// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! BC-1.18.005 (S-25.02 F4 BC-cluster 1 "cap+trigger") integration coverage
//! for the `executor.rs` -> `shard_manager.rs` wiring path.
//!
//! `executor.rs::shard_cap_precheck` (private) is the native shard-cap gate
//! invocation point, called from `execute_tiers` BEFORE the registry-driven
//! tier loop (Invariant 1). It applies two cheap, real guards before ever
//! reaching the live gate logic:
//!
//! 1. Tool-name filter — only `Edit`/`Write`/`MultiEdit` PreToolUse calls are
//!    candidates (Precondition 1).
//! 2. Config-presence filter — a no-op when no `[[shard]]` config file exists
//!    at the well-known relative path `.factory/shard-config.toml`.
//!
//! This file supplies a REAL `[[shard]]` config fixture at that exact path
//! (relative to a tempdir `cwd`) to drive a matching Edit/Write/MultiEdit
//! call PAST both guards and into `ShardRegistry::load` / `shard_cap_gate_check`
//! — both fully implemented. Every "drives the gate" test below asserts the
//! real post-implementation outcome and is green. The two negative-control
//! tests (no config present; a non-mutating tool name) exercise the
//! independent guard logic in `executor.rs` and lock in the exact wiring
//! conditions the positive tests above depend on, so a regression in either
//! guard is caught independently of the gate logic itself. The F-001 tests
//! further down additionally cover `execute_tiers`'s translation of a
//! `HookResult::Error` gate verdict into a blocking dispatch outcome
//! (non-zero `exit_code`).

use std::sync::Arc;

use factory_dispatcher::engine::build_engine;
use factory_dispatcher::executor::{ExecutorInputs, execute_tiers, shard_cap_precheck};
use factory_dispatcher::host::HostContext;
use factory_dispatcher::internal_log::InternalLog;
use factory_dispatcher::payload::HookPayload;
use factory_dispatcher::plugin_loader::PluginCache;
use factory_dispatcher::registry::Registry;
use factory_dispatcher::resolver::ResolverRegistry;

/// A well-formed `"flat"`-shaped `[[shard]]` config entry, using the BC's
/// own provisional calibration constants (BC-1.18.005 Postcondition 6).
const FLAT_SHARD_CONFIG: &str = "\
[[shard]]
artifact_stem = \"decision-log\"
artifact_path = \"decision-log.md\"
practical_fuel_ceiling = 8000000
worst_case_fuel_per_byte = 106.36
max_single_record_bytes = 16384
safety_margin = 8192
shard_cap_bytes = 49152
shape = \"flat\"
";

fn empty_registry() -> Registry {
    Registry {
        schema_version: 1,
        defaults: Default::default(),
        hooks: vec![],
    }
}

/// Writes a real `.factory/shard-config.toml` under `cwd`, at the EXACT
/// relative path `executor.rs::SHARD_CONFIG_RELATIVE_PATH` resolves against
/// `base_host_ctx.cwd` — this is the fixture the task requires to drive
/// `shard_cap_precheck` past its config-presence guard.
fn write_shard_config(cwd: &std::path::Path, body: &str) {
    let factory_dir = cwd.join(".factory");
    std::fs::create_dir_all(&factory_dir).expect("create .factory dir");
    std::fs::write(factory_dir.join("shard-config.toml"), body).expect("write shard-config.toml");
}

/// MAJOR-3 (S-25.02 cluster-2 PR #824 pr-review cycle 3; ADR-051 §Decision
/// 17): `shard_cap_precheck` is no longer computed INSIDE `execute_tiers` —
/// `main::run` now computes it once, before `execute_tiers` runs, and
/// threads the result in as a parameter. This helper mirrors that: it
/// builds the SAME `ExecutorInputs` this file's tests always used, plus the
/// `Option<HookResult>` a caller must now separately compute (via the
/// decoupled `shard_cap_precheck(&HookPayload, &Path)`) and pass alongside
/// it. `event_name` defaults to `"PreToolUse"` — this file's own tests all
/// target the PreToolUse-scoped native gate and never populated an explicit
/// `event_name` under the pre-MAJOR-3 signature either (backward
/// compatibility for exactly these fixtures was that signature's own
/// documented default).
#[allow(clippy::too_many_arguments)]
fn inputs_for<'a>(
    engine: &'a wasmtime::Engine,
    cache: &'a PluginCache,
    registry: &'a Registry,
    internal_log: &Arc<InternalLog>,
    cwd: &std::path::Path,
    tool_name: &str,
    target_path: &std::path::Path,
    tool_input_extra: serde_json::Value,
) -> (ExecutorInputs<'a>, Option<vsdd_hook_sdk::HookResult>) {
    let mut base = HostContext::new("", "0.0.1", "sess-bc-1-18-005", "trace-bc-1-18-005");
    base.cwd = cwd.to_path_buf();
    base.internal_log = Some(internal_log.clone());

    let mut tool_input = tool_input_extra;
    if let Some(map) = tool_input.as_object_mut() {
        map.insert(
            "file_path".to_string(),
            serde_json::Value::String(target_path.to_string_lossy().into_owned()),
        );
    }

    let payload = HookPayload {
        event_name: "PreToolUse".to_string(),
        tool_name: tool_name.to_string(),
        session_id: "sess-bc-1-18-005".to_string(),
        tool_input: tool_input.clone(),
        tool_response: None,
        extra: Default::default(),
    };
    let precheck_result = shard_cap_precheck(&payload, cwd);

    let inputs = ExecutorInputs {
        engine,
        cache,
        registry,
        payload_value: serde_json::json!({
            "tool_name": tool_name,
            "tool_input": tool_input,
        }),
        base_host_ctx: base,
        internal_log: internal_log.clone(),
        resolver_registry: Arc::new(ResolverRegistry::new()),
    };
    (inputs, precheck_result)
}

// ---------------------------------------------------------------------------
// Real config present + a candidate tool name drives execution through
// executor.rs's guards and into the live shard_manager gate
// (ShardRegistry::load is the first function hit).
// ---------------------------------------------------------------------------

// NOTE (BC-5.38.001 Red Gate discipline): these three tests deliberately do
// NOT use `#[should_panic]`. Each test asserts the real expected outcome (a
// within-cap call MUST Continue: `exit_code == 0`, `block_intent == false`),
// which is the correct assertion shape both during Red Gate (where it fails
// against the former `todo!()` stub) and now that `shard_manager.rs` is
// fully implemented (where it passes unchanged).

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_005_INV1_write_with_real_shard_config_reaches_native_gate() {
    let dir = tempfile::tempdir().unwrap();
    write_shard_config(dir.path(), FLAT_SHARD_CONFIG);
    let target = dir.path().join("decision-log.md");
    std::fs::write(&target, "x".repeat(5_000)).unwrap();

    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());
    let registry = empty_registry();
    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));

    let (inputs, precheck) = inputs_for(
        &engine,
        &cache,
        &registry,
        &internal_log,
        dir.path(),
        "Write",
        &target,
        serde_json::json!({"content": "x".repeat(5_000)}),
    );

    // BC-1.18.005 Invariant 1 / T-2: with a real [[shard]] config present at
    // `.factory/shard-config.toml` and a matching mutating tool name, the
    // native shard-cap gate check MUST be invoked BEFORE the (here, empty)
    // registry-driven tier loop, and a within-cap Write (5,000 <= 49,152)
    // MUST Continue — zero block intent, zero exit code.
    let summary = execute_tiers(inputs, vec![], precheck).await;
    assert_eq!(
        summary.exit_code, 0,
        "a within-cap Write against a matched [[shard]] entry MUST Continue (exit_code 0)"
    );
    assert!(!summary.block_intent);
}

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_005_INV1_edit_with_real_shard_config_reaches_native_gate() {
    let dir = tempfile::tempdir().unwrap();
    write_shard_config(dir.path(), FLAT_SHARD_CONFIG);
    let target = dir.path().join("decision-log.md");
    std::fs::write(&target, "y".repeat(45_000)).unwrap();

    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());
    let registry = empty_registry();
    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));

    let (inputs, precheck) = inputs_for(
        &engine,
        &cache,
        &registry,
        &internal_log,
        dir.path(),
        "Edit",
        &target,
        serde_json::json!({"old_string": "a", "new_string": "aaaaa"}),
    );

    // Precondition 1's tool-name filter includes Edit, not just Write.
    // current shard 45,000 + net_delta (+4) = 45,004 <= 49,152 -> Continue.
    let summary = execute_tiers(inputs, vec![], precheck).await;
    assert_eq!(
        summary.exit_code, 0,
        "a within-cap Edit against a matched [[shard]] entry MUST Continue (exit_code 0)"
    );
    assert!(!summary.block_intent);
}

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_005_INV1_multi_edit_with_real_shard_config_reaches_native_gate() {
    let dir = tempfile::tempdir().unwrap();
    write_shard_config(dir.path(), FLAT_SHARD_CONFIG);
    let target = dir.path().join("decision-log.md");
    std::fs::write(&target, "z".repeat(48_000)).unwrap();

    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());
    let registry = empty_registry();
    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));

    let (inputs, precheck) = inputs_for(
        &engine,
        &cache,
        &registry,
        &internal_log,
        dir.path(),
        "MultiEdit",
        &target,
        serde_json::json!({
            "edits": [
                {"old_string": "a", "new_string": "aa"},
                {"old_string": "b", "new_string": ""},
            ]
        }),
    );

    // Precondition 1's tool-name filter includes MultiEdit too. net delta =
    // (+1) + (-1) = 0; projected = 48,000 + 0 = 48,000 <= 49,152 -> Continue.
    let summary = execute_tiers(inputs, vec![], precheck).await;
    assert_eq!(
        summary.exit_code, 0,
        "a within-cap MultiEdit against a matched [[shard]] entry MUST Continue (exit_code 0)"
    );
    assert!(!summary.block_intent);
}

// ---------------------------------------------------------------------------
// Negative controls — REAL executor.rs guard behavior. These MUST pass:
// they document and lock in the exact two conditions (config-presence,
// tool-name match) the positive tests above rely on to reach the live gate
// at all. A regression in either guard would otherwise silently make the
// positive tests above "pass for the wrong reason" (e.g. no-op bypass
// rather than reaching the gate).
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_005_PC1_no_shard_config_present_bypasses_native_gate_no_panic() {
    let dir = tempfile::tempdir().unwrap();
    // Deliberately do NOT write .factory/shard-config.toml.
    let target = dir.path().join("decision-log.md");
    std::fs::write(&target, "x".repeat(5_000)).unwrap();

    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());
    let registry = empty_registry();
    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));

    let (inputs, precheck) = inputs_for(
        &engine,
        &cache,
        &registry,
        &internal_log,
        dir.path(),
        "Write",
        &target,
        serde_json::json!({"content": "x".repeat(5_000)}),
    );

    let summary = execute_tiers(inputs, vec![], precheck).await;
    assert_eq!(
        summary.exit_code, 0,
        "no [[shard]] config file present MUST be a complete no-op — never reaches the live gate"
    );
    assert!(!summary.block_intent);
}

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_005_PC1_non_mutating_tool_name_bypasses_native_gate_no_panic() {
    let dir = tempfile::tempdir().unwrap();
    write_shard_config(dir.path(), FLAT_SHARD_CONFIG);
    let target = dir.path().join("decision-log.md");
    std::fs::write(&target, "x".repeat(5_000)).unwrap();

    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());
    let registry = empty_registry();
    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));

    // "Read" is not in {Edit, Write, MultiEdit} — Precondition 1's tool-name
    // filter MUST reject it even though a real [[shard]] config is present.
    let (inputs, precheck) = inputs_for(
        &engine,
        &cache,
        &registry,
        &internal_log,
        dir.path(),
        "Read",
        &target,
        serde_json::json!({}),
    );

    let summary = execute_tiers(inputs, vec![], precheck).await;
    assert_eq!(
        summary.exit_code, 0,
        "a non-Edit/Write/MultiEdit tool call MUST bypass the native gate entirely, even with a matching config present"
    );
    assert!(!summary.block_intent);
}

// ---------------------------------------------------------------------------
// F-001 (HIGH, S-25.02 Phase F4 LOCAL adversary pass-1 cluster-1) — a
// malformed [[shard]] config entry MUST fail-loud all the way to the
// dispatch outcome (non-zero exit_code). `executor.rs`'s `execute_tiers`
// matches the `HookResult` returned by `shard_cap_precheck` and sets
// `block_intent = true` for `HookResult::Error` (EC-009 missing `shape`;
// EC-011 `low_water_mark >= N`) and `HookResult::Block` alike, BEFORE the
// registry-driven tier loop runs — the same fail-loud translation
// `plugin_requests_block` / `plugin_fail_closed` / `bim_fired` already apply
// to a WASM tier plugin's verdict, extended to the native gate's own
// verdict. The final `exit_code: if block_intent { 2 } else { 0 }` therefore
// reflects the native gate's decision, not just downstream tier plugins.
// Both tests below assert that resulting non-zero `exit_code` and are green —
// post-v1.12 MATCH-FIRST restructure (F-C1-P6-001), `ShardRegistry::load` is
// structural-TOML-parse-only and does NOT itself reject either malformed
// shape (both `shape` and `low_water_mark` are `Option`-typed, so they
// deserialize fine); fail-loud enforcement for both cases now happens in
// `validate_entry`, called by `shard_cap_gate_check` at entry-MATCH time on
// the entry `find_matching_entry` resolves for the dispatch (see the
// shard_manager.rs unit tests
// `test_BC_1_18_005_EC_009_matched_entry_missing_shape_field_is_fail_loud`
// and `test_BC_1_18_005_EC_011_validate_entry_rejects_low_water_mark_equal_to_n`),
// and `execute_tiers` now propagates that verdict into the dispatch outcome.
// ---------------------------------------------------------------------------

/// Shared driver for the two F-001 malformed-config cases: writes the given
/// (malformed) `[[shard]]` config body to `dir/.factory/shard-config.toml`,
/// then drives a single Edit/Write/MultiEdit call for `target` through
/// `execute_tiers` with an empty tier list — exactly the same "reaches the
/// native gate, nothing else runs" shape the existing positive/negative
/// Red Gate tests above use.
async fn run_shard_gate_for_config(
    dir: &std::path::Path,
    shard_config_body: &str,
    target: &std::path::Path,
    tool_name: &str,
    tool_input_extra: serde_json::Value,
) -> factory_dispatcher::executor::TierExecutionSummary {
    write_shard_config(dir, shard_config_body);

    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());
    let registry = empty_registry();
    let internal_log = Arc::new(InternalLog::new(dir.join("logs")));

    let (inputs, precheck) = inputs_for(
        &engine,
        &cache,
        &registry,
        &internal_log,
        dir,
        tool_name,
        target,
        tool_input_extra,
    );

    execute_tiers(inputs, vec![], precheck).await
}

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_005_F001_malformed_config_missing_shape_ec009_blocks_dispatch_outcome() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("decision-log.md");
    std::fs::write(&target, "x".repeat(100)).unwrap();

    // EC-009: entry omits `shape` entirely — fail-loud MissingShape at
    // ShardRegistry::load() time.
    let malformed_missing_shape: &str = "\
[[shard]]
artifact_stem = \"decision-log\"
artifact_path = \"decision-log.md\"
practical_fuel_ceiling = 8000000
worst_case_fuel_per_byte = 106.36
max_single_record_bytes = 16384
safety_margin = 8192
shard_cap_bytes = 49152
";

    let summary = run_shard_gate_for_config(
        dir.path(),
        malformed_missing_shape,
        &target,
        "Write",
        serde_json::json!({"content": "x".repeat(100)}),
    )
    .await;

    assert_ne!(
        summary.exit_code, 0,
        "F-001 (HIGH): a [[shard]] entry omitting `shape` (EC-009) MUST fail-loud all the way to \
         the dispatch outcome (non-zero exit_code) — executor.rs's execute_tiers translates the \
         HookResult::Error ShardRegistry::load returns into block_intent = true before the tier \
         loop runs, so exit_code is 2 here, not 0"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_005_F001_malformed_config_low_water_mark_ge_n_ec011_blocks_dispatch_outcome()
{
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("BC-INDEX.md");
    std::fs::write(
        &target,
        "---\ntitle: \"BC-INDEX\"\nchangelog:\n  - version: \"1.0\"\n---\n\n# Body\n",
    )
    .unwrap();

    // EC-011: low_water_mark (50) >= N (50) — the degenerate `== N` boundary
    // — fail-loud InvalidLowWaterMark at ShardRegistry::load() time.
    let malformed_low_water_mark: &str = "\
[[shard]]
artifact_stem = \"BC-INDEX\"
artifact_path = \"BC-INDEX.md\"
practical_fuel_ceiling = 8000000
worst_case_fuel_per_byte = 106.36
max_single_record_bytes = 16384
safety_margin = 8192
shard_cap_bytes = 49152
shape = \"frontmatter-changelog-array\"
n = 50
low_water_mark = 50
";

    let summary = run_shard_gate_for_config(
        dir.path(),
        malformed_low_water_mark,
        &target,
        "Edit",
        serde_json::json!({"old_string": "a", "new_string": "ab"}),
    )
    .await;

    assert_ne!(
        summary.exit_code, 0,
        "F-001 (HIGH): a [[shard]] entry with low_water_mark >= N (EC-011) MUST fail-loud all the \
         way to the dispatch outcome (non-zero exit_code) — executor.rs's execute_tiers \
         translates the HookResult::Error ShardRegistry::load returns into block_intent = true \
         before the tier loop runs, so exit_code is 2 here, not 0"
    );
}

// ---------------------------------------------------------------------------
// F-C1-P2-001 (MEDIUM, S-25.02 Phase F4 LOCAL adversary pass-2 cluster-1) —
// F-001 above pins that a malformed [[shard]] config's fail-loud
// HookResult::Error becomes a blocking dispatch outcome (non-zero exit_code).
// This finding additionally required that the operator-facing *reason
// text* — the same artifact_stem-naming, failure-kind-naming message
// `ShardConfigError`'s own `Display` impl already produces (see
// `shard_manager.rs`'s `MissingShape` / `InvalidLowWaterMark` /
// `CapExceedsFormulaCeiling` error variants) — actually reaches somewhere an
// operator (or `main.rs::extract_block_info`, which scans
// `TierExecutionSummary::per_plugin_results` for a blocking entry's reason)
// can see it.
//
// FIXED: `executor.rs`'s `shard_gate_block_outcome` helper, called from the
// `execute_tiers` match arm on the native gate's verdict, now synthesizes a
// `PluginOutcome` (shaped like a real WASM plugin's advisory-block verdict,
// `plugin_name = "shard-cap-gate"`, `stdout` carrying
// `{"outcome":"block","reason":"..."}`) and appends it to `all_outcomes` for
// both `HookResult::Error` and `HookResult::Block`. `summary
// .per_plugin_results` is therefore no longer always `[]` for the native
// gate's own fail-loud verdict — `main.rs::extract_block_info`'s scan over
// it now has something to find, so an operator seeing
// `block_intent=true exit_code=2` gets a non-empty `block_reason` naming
// both the offending `artifact_stem` and the BC-1.18.005 failure-kind
// marker.
//
// Each test below is GREEN, asserting against
// `format!("{:?}", summary.per_plugin_results)` (the same
// `Vec<PluginOutcome>` structure `main.rs::extract_block_info` reads) rather
// than against a specific new field name, since `extract_block_info` itself
// is a private `main.rs` fn not reachable from this integration-test crate,
// and the exact plumbing shape (a synthetic `PluginOutcome` entry vs. some
// other channel) was the implementer's call — these tests only pin that the
// message ends up SOMEWHERE inside the structure the operator-facing
// surfacing path already scans.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_005_P2001_missing_shape_block_reason_names_artifact_stem_and_failure_kind() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("decision-log.md");
    std::fs::write(&target, "x".repeat(100)).unwrap();

    // Same EC-009 malformed body as the F-001 missing-shape test above.
    let malformed_missing_shape: &str = "\
[[shard]]
artifact_stem = \"decision-log\"
artifact_path = \"decision-log.md\"
practical_fuel_ceiling = 8000000
worst_case_fuel_per_byte = 106.36
max_single_record_bytes = 16384
safety_margin = 8192
shard_cap_bytes = 49152
";

    let summary = run_shard_gate_for_config(
        dir.path(),
        malformed_missing_shape,
        &target,
        "Write",
        serde_json::json!({"content": "x".repeat(100)}),
    )
    .await;

    assert_ne!(
        summary.exit_code, 0,
        "precondition: this config MUST still block (see F-001)"
    );

    let per_plugin_debug = format!("{:?}", summary.per_plugin_results);
    assert!(
        per_plugin_debug.contains("decision-log"),
        "F-C1-P2-001: the block_reason surfacing path (per_plugin_results, which \
         main.rs::extract_block_info scans) MUST name the offending artifact_stem \
         (\"decision-log\") somewhere so an operator can locate the bad config entry without \
         opening the internal log. Got: {per_plugin_debug}"
    );
    assert!(
        per_plugin_debug.contains("EC-009"),
        "F-C1-P2-001: the block_reason surfacing path MUST name the failure kind (EC-009 \
         missing `shape`) so an operator knows WHICH BC-1.18.005 fail-loud condition fired, not \
         just that something blocked. Got: {per_plugin_debug}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_005_P2001_low_water_mark_block_reason_names_artifact_stem_and_failure_kind() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("BC-INDEX.md");
    std::fs::write(
        &target,
        "---\ntitle: \"BC-INDEX\"\nchangelog:\n  - version: \"1.0\"\n---\n\n# Body\n",
    )
    .unwrap();

    // Same EC-011 malformed body as the F-001 low-water-mark test above.
    let malformed_low_water_mark: &str = "\
[[shard]]
artifact_stem = \"BC-INDEX\"
artifact_path = \"BC-INDEX.md\"
practical_fuel_ceiling = 8000000
worst_case_fuel_per_byte = 106.36
max_single_record_bytes = 16384
safety_margin = 8192
shard_cap_bytes = 49152
shape = \"frontmatter-changelog-array\"
n = 50
low_water_mark = 50
";

    let summary = run_shard_gate_for_config(
        dir.path(),
        malformed_low_water_mark,
        &target,
        "Edit",
        serde_json::json!({"old_string": "a", "new_string": "ab"}),
    )
    .await;

    assert_ne!(
        summary.exit_code, 0,
        "precondition: this config MUST still block (see F-001)"
    );

    let per_plugin_debug = format!("{:?}", summary.per_plugin_results);
    assert!(
        per_plugin_debug.contains("BC-INDEX"),
        "F-C1-P2-001: the block_reason surfacing path MUST name the offending artifact_stem \
         (\"BC-INDEX\") so an operator can locate the bad config entry without opening the \
         internal log. Got: {per_plugin_debug}"
    );
    assert!(
        per_plugin_debug.contains("EC-011"),
        "F-C1-P2-001: the block_reason surfacing path MUST name the failure kind (EC-011 \
         low_water_mark >= N) so an operator knows WHICH BC-1.18.005 fail-loud condition fired. \
         Got: {per_plugin_debug}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_005_P2001_cap_exceeds_ceiling_block_reason_names_artifact_stem_and_failure_kind()
 {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("over-cap-log.md");
    std::fs::write(&target, "x".repeat(10)).unwrap();

    // Postcondition 9 / EC-013: declared shard_cap_bytes (100,000) exceeds
    // compute_shard_cap_bytes(these four inputs) = 50,640 — same worked
    // example as shard_manager.rs's
    // test_BC_1_18_005_PC9_EC013_validate_entry_rejects_cap_greater_than_formula_ceiling.
    let malformed_cap_exceeds_ceiling: &str = "\
[[shard]]
artifact_stem = \"over-cap-log\"
artifact_path = \"over-cap-log.md\"
practical_fuel_ceiling = 8000000
worst_case_fuel_per_byte = 106.36
max_single_record_bytes = 16384
safety_margin = 8192
shard_cap_bytes = 100000
shape = \"flat\"
";

    let summary = run_shard_gate_for_config(
        dir.path(),
        malformed_cap_exceeds_ceiling,
        &target,
        "Write",
        serde_json::json!({"content": "x".repeat(10)}),
    )
    .await;

    assert_ne!(
        summary.exit_code, 0,
        "Postcondition 9 / EC-013: a [[shard]] entry declaring shard_cap_bytes GREATER than its \
         own formula-derived ceiling MUST fail-loud all the way to the dispatch outcome \
         (non-zero exit_code), the same as the EC-009/EC-011 F-001 cases above"
    );

    let per_plugin_debug = format!("{:?}", summary.per_plugin_results);
    assert!(
        per_plugin_debug.contains("over-cap-log"),
        "F-C1-P2-001: the block_reason surfacing path MUST name the offending artifact_stem \
         (\"over-cap-log\") so an operator can locate the bad config entry without opening the \
         internal log. Got: {per_plugin_debug}"
    );
    assert!(
        per_plugin_debug.contains("EC-013"),
        "F-C1-P2-001: the block_reason surfacing path MUST name the failure kind (Postcondition \
         9 / EC-013 cap-exceeds-ceiling) so an operator knows WHICH BC-1.18.005 fail-loud \
         condition fired. Got: {per_plugin_debug}"
    );
}

// ---------------------------------------------------------------------------
// PR #818 cycle-2 review finding B-2 — an absent/non-string
// tool_input.file_path MUST fail loud AT THE shard_cap_precheck LEVEL
// (executor.rs), never silently default to an empty PathBuf that lets
// find_matching_entry resolve None (no stem) and Continue as if the
// dispatch matched no [[shard]] entry at all. The pre-existing malformed-
// payload tests (m3, above and in shard_manager.rs's own unit tests) all
// exercise this ONE layer down, inside shard_cap_gate_check itself (which
// already has a valid target_path by the time it runs) — this test drives
// the FULL execute_tiers -> shard_cap_precheck path with a payload that
// omits "file_path" entirely, pinning the bug at the layer it actually
// lives in.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_005_B2_shard_cap_precheck_missing_file_path_fails_loud() {
    let dir = tempfile::tempdir().unwrap();
    write_shard_config(dir.path(), FLAT_SHARD_CONFIG);

    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());
    let registry = empty_registry();
    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));

    let mut base = HostContext::new("", "0.0.1", "sess-bc-1-18-005-b2", "trace-bc-1-18-005-b2");
    base.cwd = dir.path().to_path_buf();
    base.internal_log = Some(internal_log.clone());

    // Deliberately malformed payload: tool_input carries NO "file_path" key
    // at all — the exact shape shard_cap_precheck's pre-B-2
    // `.unwrap_or_default()` silently coerced into an empty PathBuf, which
    // has no file_stem(), so find_matching_entry resolved None and the
    // whole gate Continued as if this dispatch matched no [[shard]] entry.
    let inputs = ExecutorInputs {
        engine: &engine,
        cache: &cache,
        registry: &registry,
        payload_value: serde_json::json!({
            "tool_name": "Write",
            "tool_input": {"content": "x".repeat(5_000)},
        }),
        base_host_ctx: base,
        internal_log: internal_log.clone(),
        resolver_registry: Arc::new(ResolverRegistry::new()),
    };

    // MAJOR-3: `shard_cap_precheck` is computed by the caller (mirroring
    // `main::run`'s own call site) and threaded into `execute_tiers` — the
    // SAME malformed (missing "file_path") payload the ExecutorInputs above
    // carries, built as a real HookPayload this time so the decoupled
    // function can read it.
    let payload = HookPayload {
        event_name: "PreToolUse".to_string(),
        tool_name: "Write".to_string(),
        session_id: "sess-bc-1-18-005-b2".to_string(),
        tool_input: serde_json::json!({"content": "x".repeat(5_000)}),
        tool_response: None,
        extra: Default::default(),
    };
    let precheck = shard_cap_precheck(&payload, dir.path());

    let summary = execute_tiers(inputs, vec![], precheck).await;

    assert_ne!(
        summary.exit_code, 0,
        "B-2: a Write dispatch whose tool_input is missing \"file_path\" entirely MUST fail loud \
         (non-zero exit_code), never silently Continue as if the dispatch matched no [[shard]] \
         entry — a malformed payload must never let the shard-cap gate go silently unguarded."
    );
    assert!(
        summary.block_intent,
        "B-2: block_intent MUST be set for a Write dispatch with a missing file_path"
    );
    let per_plugin_debug = format!("{:?}", summary.per_plugin_results);
    assert!(
        per_plugin_debug.contains("file_path"),
        "B-2: the block_reason surfacing path MUST name the missing \"file_path\" field so an \
         operator can diagnose the malformed payload. Got: {per_plugin_debug}"
    );
}

// ---------------------------------------------------------------------------
// PR #818 fix-burst finding m5 — the synthesized shard-cap-gate
// PluginOutcome MUST carry an accurate, non-empty plugin_version (the
// dispatcher's own version, same as every other native/sentinel outcome in
// executor.rs), never a hardcoded empty string.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_005_m5_shard_gate_block_outcome_carries_accurate_plugin_version() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("decision-log.md");
    std::fs::write(&target, "x".repeat(10)).unwrap();

    // Same EC-013 malformed config as the AC-023 test above — any fail-loud
    // verdict from shard_cap_precheck exercises shard_gate_block_outcome.
    let malformed_cap_exceeds_ceiling: &str = "\
[[shard]]
artifact_stem = \"decision-log\"
artifact_path = \"decision-log.md\"
practical_fuel_ceiling = 8000000
worst_case_fuel_per_byte = 106.36
max_single_record_bytes = 16384
safety_margin = 8192
shard_cap_bytes = 100000
shape = \"flat\"
";

    let summary = run_shard_gate_for_config(
        dir.path(),
        malformed_cap_exceeds_ceiling,
        &target,
        "Write",
        serde_json::json!({"content": "x".repeat(10)}),
    )
    .await;

    let shard_gate_outcome = summary
        .per_plugin_results
        .iter()
        .find(|o| o.plugin_name == "shard-cap-gate")
        .expect("m5: the native shard-cap gate's own fail-loud verdict MUST be present in per_plugin_results");

    // `inputs_for`'s fixture HostContext is constructed with plugin_version
    // "0.0.1" (see `HostContext::new("", "0.0.1", ...)` above) — before this
    // fix, `shard_gate_block_outcome` hardcoded `plugin_version:
    // String::new()` regardless of the dispatcher's actual version.
    assert_eq!(
        shard_gate_outcome.plugin_version, "0.0.1",
        "m5: the synthesized shard-cap-gate PluginOutcome MUST carry the dispatcher's actual \
         plugin_version (\"0.0.1\" in this fixture), not a hardcoded empty string — got {:?}",
        shard_gate_outcome.plugin_version
    );
}

// ---------------------------------------------------------------------------
// F-C1-P2-004 (LOW, S-25.02 Phase F4 LOCAL adversary pass-2 cluster-1) —
// negative control proving the native shard-cap gate is PreToolUse-scoped
// ONLY (BC-1.18.005 Precondition 1). `shard_cap_precheck` (executor.rs) now
// reads `event_name` and classifies it via `EventType` (mirroring `main.rs`'s
// `EventType::from_event_str(&payload.event_name)`) BEFORE the `tool_name` /
// `[[shard]]` config-presence guards, and short-circuits with `None` (no
// gate check performed at all) for any dispatch whose `event_name`
// classifies as something other than `PreToolUse`. A dispatch that omits
// `event_name` entirely — as the F-001 tests above do — is treated as
// `PreToolUse`-equivalent for backward compatibility with those
// pre-existing fixtures; only an EXPLICIT non-`PreToolUse` `event_name`
// opts a dispatch out. This test is GREEN: a PostToolUse Edit/Write/
// MultiEdit call against a matched, malformed `[[shard]]` entry now
// short-circuits before ever reaching `ShardRegistry::load`, and therefore
// does NOT block, unlike the PreToolUse-equivalent case the F-001 tests
// exercise.
// ---------------------------------------------------------------------------

/// Same shape as `inputs_for`, plus an explicit `event_name` field in the
/// payload envelope — mirroring the exact `EventType` classification
/// `main.rs` performs via `EventType::from_event_str(&payload.event_name)`
/// (see `main.rs`'s `event_is_advisory_only` computation), so this fixture's
/// `payload_value` is shaped exactly like a real harness envelope's
/// `event_name` field (`HookPayload::event_name`), not an ad hoc string.
#[allow(clippy::too_many_arguments)]
fn inputs_for_event<'a>(
    engine: &'a wasmtime::Engine,
    cache: &'a PluginCache,
    registry: &'a Registry,
    internal_log: &Arc<InternalLog>,
    cwd: &std::path::Path,
    event_name: &str,
    tool_name: &str,
    target_path: &std::path::Path,
    tool_input_extra: serde_json::Value,
) -> (ExecutorInputs<'a>, Option<vsdd_hook_sdk::HookResult>) {
    let mut base = HostContext::new("", "0.0.1", "sess-bc-1-18-005", "trace-bc-1-18-005");
    base.cwd = cwd.to_path_buf();
    base.internal_log = Some(internal_log.clone());

    let mut tool_input = tool_input_extra;
    if let Some(map) = tool_input.as_object_mut() {
        map.insert(
            "file_path".to_string(),
            serde_json::Value::String(target_path.to_string_lossy().into_owned()),
        );
    }

    // MAJOR-3: computed via the decoupled `shard_cap_precheck`, using the
    // SAME explicit `event_name` this fixture carries — this is the exact
    // point of this test (a non-PreToolUse event must short-circuit to
    // `None` here, before `execute_tiers` ever sees it).
    let payload = HookPayload {
        event_name: event_name.to_string(),
        tool_name: tool_name.to_string(),
        session_id: "sess-bc-1-18-005".to_string(),
        tool_input: tool_input.clone(),
        tool_response: None,
        extra: Default::default(),
    };
    let precheck_result = shard_cap_precheck(&payload, cwd);

    let inputs = ExecutorInputs {
        engine,
        cache,
        registry,
        payload_value: serde_json::json!({
            "event_name": event_name,
            "tool_name": tool_name,
            "tool_input": tool_input,
        }),
        base_host_ctx: base,
        internal_log: internal_log.clone(),
        resolver_registry: Arc::new(ResolverRegistry::new()),
    };
    (inputs, precheck_result)
}

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_005_PC1_post_tool_use_event_does_not_run_shard_gate_negative_control() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("decision-log.md");
    std::fs::write(&target, "x".repeat(100)).unwrap();

    // Deliberately the SAME EC-009 malformed body the F-001
    // `test_BC_1_18_005_F001_malformed_config_missing_shape_ec009_blocks_dispatch_outcome` test
    // proves blocks a PreToolUse-equivalent dispatch (that test's payload omits `event_name`
    // entirely) — if this PostToolUse call is ALSO blocked, the native gate is not honoring
    // Precondition 1's PreToolUse scoping.
    let malformed_missing_shape: &str = "\
[[shard]]
artifact_stem = \"decision-log\"
artifact_path = \"decision-log.md\"
practical_fuel_ceiling = 8000000
worst_case_fuel_per_byte = 106.36
max_single_record_bytes = 16384
safety_margin = 8192
shard_cap_bytes = 49152
";
    write_shard_config(dir.path(), malformed_missing_shape);

    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());
    let registry = empty_registry();
    let internal_log = Arc::new(InternalLog::new(dir.path().join("logs")));

    let (inputs, precheck) = inputs_for_event(
        &engine,
        &cache,
        &registry,
        &internal_log,
        dir.path(),
        factory_dispatcher::invoke::EventType::PostToolUse.as_str(),
        "Edit",
        &target,
        serde_json::json!({"old_string": "a", "new_string": "ab"}),
    );

    let summary = execute_tiers(inputs, vec![], precheck).await;

    assert_eq!(
        summary.exit_code, 0,
        "BC-1.18.005 Precondition 1: the native shard-cap gate is PreToolUse-scoped ONLY. A \
         PostToolUse Edit call against the SAME malformed [[shard]] config that \
         test_BC_1_18_005_F001_malformed_config_missing_shape_ec009_blocks_dispatch_outcome \
         proves blocks a PreToolUse-equivalent dispatch MUST NOT be blocked here. \
         shard_cap_precheck's event_name guard classifies this PostToolUse call and \
         short-circuits with None BEFORE the tool_name/config-presence guards and BEFORE \
         ShardRegistry::load ever runs, so the malformed config is never even loaded for this \
         event (exit_code=0)."
    );
    assert!(
        !summary.block_intent,
        "PC1: a PostToolUse dispatch MUST NOT have block_intent set by the shard-cap gate"
    );
}

// ---------------------------------------------------------------------------
// EC-018 / EC-019 (BC-1.18.005 v1.12 MATCH-FIRST restructure, F-C1-P6-001,
// LOW pending-intent, product-owner adjudication) — "Blast-radius scoping".
//
// Pre-v1.12, `executor.rs::shard_cap_precheck` calls `ShardRegistry::load()`
// — which parses AND semantically validates EVERY `[[shard]]` entry
// (fail-loud on EC-009/EC-011/EC-013/EC-015/EC-016/EC-017 for ANY entry) —
// BEFORE `shard_cap_gate_check` ever matches the dispatch's target path to a
// specific entry. Consequence: a SINGLE malformed SIBLING entry causes
// `HookResult::Error` for EVERY Edit/Write/MultiEdit dispatch in the repo
// whose config file exists, INCLUDING dispatches whose target matches NO
// entry at all, or matches a DIFFERENT, well-formed entry.
//
// v1.12 ADJUDICATED (B) MATCH-FIRST: `ShardRegistry::load()` becomes
// structural-TOML-deserialization-only; a NEW `validate_entry(&ShardEntry)`
// carries the semantic checks, invoked ONLY on the entry `find_matching_entry`
// resolves for the current dispatch. EC-018 pins the RESOLVED blast-radius
// scenario (`Continue`/normal gate logic, sibling never validated). EC-019
// pins the ONE residual, unavoidable exception: a config file that fails
// STRUCTURAL TOML deserialization still blocks every dispatch regardless of
// match, since `find_matching_entry` cannot run without a successfully
// deserialized registry.
//
// The two EC-018 tests below drive the FULL public gate path
// (`execute_tiers` -> `shard_cap_precheck` -> `ShardRegistry::load` ->
// `shard_cap_gate_check`, via the same `run_shard_gate_for_config` helper the
// F-001/P2001 tests above use) — this is deliberate: `shard_cap_gate_check`
// alone never eagerly validates siblings (it only ever looks at the entry
// `find_matching_entry` resolves), so exercising it in isolation would
// trivially return `Continue` and prove nothing about blast-radius scoping.
// Only the full `shard_cap_precheck` path exercises the ordering guarantee
// end to end: `ShardRegistry::load()` (structural-parse-only, post-v1.12)
// runs first, `find_matching_entry` resolves the dispatch's target to (at
// most) one entry, and ONLY that resolved entry — never a sibling — is ever
// passed to `validate_entry`. These two tests pin that ordering: the
// malformed `lessons` sibling entry MUST NOT block a dispatch this
// Postcondition promises is entirely unaffected by it.
//
// The matched-to-the-malformed-entry-itself scenario (BC-1.18.005's third
// Canonical Test Vector for this ruling — "UNCHANGED from pre-v1.12
// behavior") is already covered by the existing F-001/P2001 tests above
// (`test_BC_1_18_005_F001_malformed_config_missing_shape_ec009_...`,
// `..._low_water_mark_ge_n_ec011_...`,
// `test_BC_1_18_005_P2001_cap_exceeds_ceiling_...`) — each of those already
// targets the SAME artifact_stem as the single malformed entry in its
// fixture, so they already pin "matched malformed entry still fails loud"
// and require no changes here.
// ---------------------------------------------------------------------------

/// A `[[shard]]` config entry for `lessons` that OMITS `shape` entirely
/// (EC-009-style malformation) — the "malformed SIBLING entry" fixture for
/// the EC-018 pinning tests below. THE F-C1-P6-001 scenario's own malformed
/// entry (BC-1.18.005's Postcondition 1 "Blast-radius scoping" sub-paragraph
/// and its first two new Canonical Test Vectors both use this exact
/// `lessons`-omits-`shape` fixture).
const MALFORMED_LESSONS_SIBLING_ENTRY: &str = "\
[[shard]]
artifact_stem = \"lessons\"
artifact_path = \"lessons.md\"
practical_fuel_ceiling = 8000000
worst_case_fuel_per_byte = 106.36
max_single_record_bytes = 16384
safety_margin = 8192
shard_cap_bytes = 49152
";

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_005_EC_018_malformed_sibling_unmatched_target_continues() {
    let dir = tempfile::tempdir().unwrap();
    // Target does NOT exist on disk and its stem ("foo") matches NO
    // `[[shard]]` entry in the config below — mirroring BC-1.18.005's own
    // "THE F-C1-P6-001 scenario" Canonical Test Vector exactly (`Edit` to
    // `src/foo.rs`).
    let target = dir.path().join("foo.rs");

    // The config's ONLY entry ("lessons") omits `shape` entirely — it would
    // fail EC-009 if matched, but this dispatch's target does not match it.
    let summary = run_shard_gate_for_config(
        dir.path(),
        MALFORMED_LESSONS_SIBLING_ENTRY,
        &target,
        "Edit",
        serde_json::json!({"old_string": "a", "new_string": "ab"}),
    )
    .await;

    // Postcondition 1's "Blast-radius scoping" ruling (v1.12): `Continue`
    // MUST fire — `find_matching_entry` returns `None` for `foo.rs` BEFORE
    // `validate_entry` (or any semantic check) is ever invoked on the
    // malformed `lessons` entry, so its `shape` omission is NEVER evaluated
    // for this dispatch. `ShardRegistry::load()` is structural-TOML-parse-
    // only (post-v1.12 MATCH-FIRST restructure) and never validates entry
    // semantics itself, so the malformed sibling's `shape` omission has no
    // opportunity to surface for a dispatch that never matches it.
    assert_eq!(
        summary.exit_code, 0,
        "EC-018: a dispatch whose target matches NO [[shard]] entry MUST Continue \
         (exit_code 0) even when a SIBLING entry (\"lessons\") is malformed — the malformed \
         sibling must NEVER be validated for this dispatch. Got exit_code={} (per_plugin_results: \
         {:?}) — if this fired, `ShardRegistry::load()` is still eagerly validating every entry \
         BEFORE find_matching_entry runs, reproducing the pre-v1.12 blast-radius bug \
         (BC-1.18.005 Postcondition 1's \"Blast-radius scoping\" ruling, F-C1-P6-001).",
        summary.exit_code, summary.per_plugin_results
    );
    assert!(
        !summary.block_intent,
        "EC-018: block_intent MUST NOT be set by a malformed SIBLING entry for an unmatched target"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_005_EC_018_malformed_sibling_matched_different_well_formed_entry_continues() {
    let dir = tempfile::tempdir().unwrap();
    // Target MATCHES the DIFFERENT, well-formed `decision-log` entry —
    // mirroring BC-1.18.005's own matching Canonical Test Vector exactly
    // (`Write` to `decision-log.md`, `shard_cap_bytes=49,152`, `content`
    // length 5,000 bytes).
    let target = dir.path().join("decision-log.md");

    // Two-entry config: the malformed `lessons` sibling (omits `shape`)
    // PLUS the well-formed `decision-log` entry this dispatch actually
    // targets.
    let config = format!("{MALFORMED_LESSONS_SIBLING_ENTRY}\n{FLAT_SHARD_CONFIG}");

    let summary = run_shard_gate_for_config(
        dir.path(),
        &config,
        &target,
        "Write",
        serde_json::json!({"content": "x".repeat(5_000)}),
    )
    .await;

    // `projected_size = len(content) = 5,000 <= 49,152` -> normal
    // Postcondition 3 gate logic against `decision-log.md`'s OWN
    // well-formed entry proceeds entirely unaffected by the `lessons`
    // entry's malformation (EC-018). `ShardRegistry::load()`'s structural
    // parse (post-v1.12) never validates entry semantics, so the malformed
    // `lessons` entry never fails anything at load time; `validate_entry`
    // is invoked ONLY on the resolved `decision-log` entry that
    // `find_matching_entry` matches, so `lessons`'s malformation is never
    // examined for this dispatch at all.
    assert_eq!(
        summary.exit_code, 0,
        "EC-018: a dispatch matching a DIFFERENT, well-formed [[shard]] entry MUST proceed with \
         normal gate logic (Continue here: 5,000 <= 49,152) even when a SIBLING entry \
         (\"lessons\") is malformed — the malformed sibling must NEVER be validated for this \
         dispatch, and must NEVER cause this dispatch's own well-formed entry to be skipped or \
         its verdict overridden. Got exit_code={} (per_plugin_results: {:?})",
        summary.exit_code, summary.per_plugin_results
    );
    assert!(
        !summary.block_intent,
        "EC-018: block_intent MUST NOT be set by a malformed SIBLING entry for a dispatch \
         matching a different, well-formed entry"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_005_EC_019_structurally_invalid_toml_syntax_blocks_regardless_of_match() {
    let dir = tempfile::tempdir().unwrap();
    // Target does NOT match anything in the (structurally broken) config —
    // mirroring BC-1.18.005's own EC-019 residual-exception Canonical Test
    // Vector exactly (`Edit` to `src/foo.rs`, config file has invalid TOML
    // syntax).
    let target = dir.path().join("foo.rs");

    // Invalid TOML syntax (unterminated array-of-tables header) — `toml::
    // from_str` cannot deserialize this AT ALL, regardless of how many
    // (if any) `[[shard]]` entries it was meant to contain.
    let invalid_toml_syntax = "[[shard]\nartifact_stem = \"decision-log\"\n";

    let summary = run_shard_gate_for_config(
        dir.path(),
        invalid_toml_syntax,
        &target,
        "Edit",
        serde_json::json!({"old_string": "a", "new_string": "ab"}),
    )
    .await;

    // EC-019: `find_matching_entry` cannot run at all without a
    // successfully deserialized registry — this is the ONE residual case
    // where a config defect retains whole-file blast radius, regardless of
    // match. This is a REGRESSION PIN: `toml::from_str` already rejects
    // syntactically-invalid TOML at the very top of `ShardRegistry::load`
    // TODAY (before any per-entry loop), so this assertion is GREEN now and
    // MUST remain GREEN after the match-first restructure — the v1.12
    // ruling explicitly does NOT weaken this residual exception.
    assert_ne!(
        summary.exit_code, 0,
        "EC-019: a [[shard]] config FILE that fails STRUCTURAL TOML deserialization MUST \
         fail-loud (HookResult::Error) for EVERY Edit/Write/MultiEdit dispatch while the file \
         exists, matched or not — find_matching_entry cannot run without a successfully \
         deserialized registry. Got exit_code=0 (Continue) instead."
    );
    assert!(
        summary.block_intent,
        "EC-019: block_intent MUST be set for a structurally-invalid [[shard]] config file"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_005_EC_019_missing_required_non_option_field_blocks_regardless_of_match() {
    let dir = tempfile::tempdir().unwrap();
    // Target does NOT match anything relevant — mirroring EC-019's
    // "matched or not" scope (the residual exception applies regardless of
    // match).
    let target = dir.path().join("foo.rs");

    // `shard_cap_bytes: u64` (non-`Option`) is omitted entirely — `toml::
    // from_str` cannot deserialize a `Vec<ShardEntry>` with a missing
    // required field; this is a STRUCTURAL parse failure (EC-019), not a
    // semantic one (contrast with EC-009's `shape: Option<ShardShape>`,
    // which deserializes fine when omitted and is instead a MATCH-TIME
    // semantic failure).
    let missing_required_field: &str = "\
[[shard]]
artifact_stem = \"decision-log\"
practical_fuel_ceiling = 8000000
worst_case_fuel_per_byte = 106.36
max_single_record_bytes = 16384
safety_margin = 8192
shape = \"flat\"
";

    let summary = run_shard_gate_for_config(
        dir.path(),
        missing_required_field,
        &target,
        "Edit",
        serde_json::json!({"old_string": "a", "new_string": "ab"}),
    )
    .await;

    // REGRESSION PIN (same rationale as the invalid-syntax EC-019 test
    // above): `toml::from_str` already rejects a missing non-`Option`
    // field TODAY (a `serde`-level deserialize error, still surfaced via
    // `ShardConfigError::Toml`'s `#[from] toml::de::Error`), so this
    // assertion is GREEN now and MUST remain GREEN after the match-first
    // restructure.
    assert_ne!(
        summary.exit_code, 0,
        "EC-019: a [[shard]] entry omitting a non-Option field required for toml::from_str to \
         succeed (here, shard_cap_bytes) MUST fail-loud (HookResult::Error) for EVERY \
         Edit/Write/MultiEdit dispatch while the file exists, matched or not. Got exit_code=0 \
         (Continue) instead."
    );
    assert!(
        summary.block_intent,
        "EC-019: block_intent MUST be set for a [[shard]] config file missing a required \
         non-Option field"
    );
}

// ---------------------------------------------------------------------------
// MAJOR-3 (S-25.02 cluster-2 PR #824 pr-review cycle 3; ADR-051 §Decision
// 17) — real-binary regression: the PreToolUse shard-cap gate MUST still
// fire when NEITHER the sync NOR async matched-plugin group has any
// entries at all. Before MAJOR-3, `shard_cap_precheck` ran only INSIDE
// `execute_tiers`, which `main::run` skips ENTIRELY via its
// `sync_tiers.is_empty() && partition.async_group.is_empty()` early-return
// guard — so an empty matched-plugin set silently defeated the gate. Every
// test above drives `execute_tiers` (or a helper wrapping it) directly, so
// none of them can exercise `main::run`'s own guard — this is the ONE test
// in this suite that spawns the REAL compiled binary end-to-end, the
// concrete falsifier the pre-MAJOR-3 architecture made impossible to
// exercise (MINOR-4, PR #824 pr-review cycle 3).
// ---------------------------------------------------------------------------

/// Path to the compiled `factory-dispatcher` binary — `CARGO_BIN_EXE_factory-dispatcher`
/// is set by Cargo for integration tests (see `bc_3_08_001_s19_05.rs`'s own
/// identical helper).
fn binary_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_factory-dispatcher"))
}

/// MAJOR-3 concrete falsifier: a PreToolUse Edit against a matched, EC-009-malformed
/// `[[shard]]` entry (omits `shape`) MUST still surface `HookResult::Error`/`Block`
/// and `final_exit_code == 2` even when `CLAUDE_PLUGIN_ROOT` points at a registry
/// with ZERO `[[hooks]]` entries — the exact "both sync_tiers and
/// partition.async_group empty" configuration that previously made the native
/// shard-cap gate unreachable from `main::run`.
#[test]
fn test_MAJOR3_shard_cap_gate_fires_via_real_binary_when_no_plugin_matched() {
    use std::io::Write as _;

    let plugin_root = tempfile::tempdir().expect("tempdir for plugin_root");
    // schema_version=2, ZERO [[hooks]] entries — match_plugins resolves an
    // empty matched set for every dispatch, so both partition.sync_group
    // and partition.async_group are empty and main::run's early-return
    // guard would (pre-MAJOR-3) have skipped execute_tiers — and with it,
    // shard_cap_precheck — entirely.
    std::fs::write(
        plugin_root.path().join("hooks-registry.toml"),
        "schema_version = 2\n",
    )
    .expect("write empty hooks-registry.toml");

    let project_dir = tempfile::tempdir().expect("tempdir for project cwd");
    let target = project_dir.path().join("decision-log.md");
    std::fs::write(&target, "x".repeat(100)).expect("write target fixture");

    // Same EC-009 malformed body (omits `shape`) the library-level F-001
    // test above (`test_BC_1_18_005_F001_malformed_config_missing_shape_ec009_blocks_dispatch_outcome`)
    // proves fails loud when driven directly through `execute_tiers`.
    let malformed_missing_shape = "\
[[shard]]
artifact_stem = \"decision-log\"
artifact_path = \"decision-log.md\"
practical_fuel_ceiling = 8000000
worst_case_fuel_per_byte = 106.36
max_single_record_bytes = 16384
safety_margin = 8192
shard_cap_bytes = 49152
";
    write_shard_config(project_dir.path(), malformed_missing_shape);

    let payload = serde_json::json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Edit",
        "session_id": "sess-major3-real-binary",
        "tool_input": {
            "file_path": target.to_string_lossy(),
            "old_string": "a",
            "new_string": "ab",
        },
    })
    .to_string();

    let mut child = std::process::Command::new(binary_path())
        .env("CLAUDE_PLUGIN_ROOT", plugin_root.path())
        .env("CLAUDE_PROJECT_DIR", project_dir.path())
        .env("VSDD_LOG_DIR", project_dir.path().join("logs"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn factory-dispatcher binary");

    child
        .stdin
        .take()
        .expect("child stdin must be available")
        .write_all(payload.as_bytes())
        .expect("failed to write payload to binary stdin");

    let output = child
        .wait_with_output()
        .expect("failed to wait for factory-dispatcher output");

    assert_eq!(
        output.status.code(),
        Some(2),
        "MAJOR-3: a PreToolUse Edit against a matched, EC-009-malformed [[shard]] entry MUST \
         still exit 2 even when NO plugin matched at all (empty hooks-registry.toml) — if this \
         is 0, the native shard-cap gate was silently skipped because main::run's early-return \
         guard fired before shard_cap_precheck ever ran, reproducing the exact bug MAJOR-3 \
         fixed.\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}
