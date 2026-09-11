// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! BC-1.18.009 (S-25.02 cluster-4 "B1 rotation") integration coverage for the
//! PreToolUse `FrontmatterChangelogArray` artifact-shape rotate-and-retry path
//! in `shard_manager.rs`, exercised through the `executor.rs` ->
//! `shard_cap_precheck` -> `shard_cap_gate_check` wiring identical to the one
//! `bc_1_18_006_roll_test.rs` uses for the `Flat` shape.
//!
//! # Implementation state — GREEN
//!
//! The `FrontmatterChangelogArray` trigger-fired branch in `shard_manager.rs`
//! and `rotate_changelog_at` in `last-amended-migrate/src/rotate.rs` are both
//! fully implemented. All 10 tests in this file pass GREEN against the current
//! implementation.
//!
//! # Always-passing tests (below-threshold and pure-logic paths)
//!
//! Two tests exercise paths that never enter the rotation branch and therefore
//! pass regardless of rotation-logic changes:
//!
//! * `test_BC_1_18_009_AC015_EC002_CTV3_below_n_continues_without_rotation`:
//!   tests the below-threshold `Continue` path (item count < N=50); the
//!   rotation branch is not taken.
//!
//! * `test_BC_1_18_009_AC015_build_b1_block_reason_format_pinned_verbatim`:
//!   tests the pure string-template `build_b1_block_reason` helper — zero
//!   branching, no I/O.
//!
//! # BC ambiguity / Postcondition 2 step 3 verbatim text
//!
//! `test_BC_1_18_009_AC015_build_b1_block_reason_format_pinned_verbatim`
//! asserts the exact retry-instruction message against BC-1.18.009
//! Postcondition 2 step 3's prescribed text, per the L-BB-D1179 verbatim-pin
//! lesson (no weak `.contains()` substring checks — a prior sibling template
//! diverged from its spec text while a substring-only test stayed green through
//! the divergence, F-C2-P5-002/F-C2-P6-002).
//!
//! # VP coverage
//!
//! * VP-125 (bounded-live-sequence + no-history-loss): exercised by
//!   `test_BC_1_18_009_AC015_CTV1_*` (live sequence trimmed to low_water_mark)
//!   and `test_BC_1_18_009_AC016_VP125_single_evergreen_archive_no_history_loss`
//!   (same archive file accumulates across two rotations).
//! * VP-126 (zero `prepend_changelog_item` call sites in B1 handler): exercised
//!   by `test_BC_1_18_009_AC015_INV1_VP126_zero_prepend_changelog_item_callsites_in_shard_manager`.
//! * VP-131 (fail-loud E-SHD-004 + pre-rotation state preserved): exercised by
//!   `test_BC_1_18_009_AC016_VP131_CTV4_rotation_failure_returns_e_shd_004_state_preserved`.

use std::sync::Arc;

use factory_dispatcher::engine::build_engine;
use factory_dispatcher::executor::{ExecutorInputs, execute_tiers, shard_cap_precheck};
use factory_dispatcher::host::HostContext;
use factory_dispatcher::internal_log::InternalLog;
use factory_dispatcher::invoke::PluginResult;
use factory_dispatcher::payload::HookPayload;
use factory_dispatcher::plugin_loader::PluginCache;
use factory_dispatcher::registry::Registry;
use factory_dispatcher::resolver::ResolverRegistry;
use factory_dispatcher::shard_manager::build_b1_block_reason;

/// Well-formed `"frontmatter-changelog-array"`-shaped `[[shard]]` config
/// entry for `BC-INDEX.md`, calibrated to N=50 items and low_water_mark=25
/// (BC-1.18.009 CTV canonical test vectors).
///
/// `shard_cap_bytes = 49152` satisfies the BC-1.18.005 Postcondition 9
/// cap-vs-formula inequality check (same calibration constants used by
/// `bc_1_18_005_shard_cap_trigger_test.rs`'s `FLAT_SHARD_CONFIG`; the
/// formula ceiling is 50,640, and 49,152 < 50,640).
const B1_SHARD_CONFIG: &str = "\
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
low_water_mark = 25
";

/// The expected `low_water_mark` trim floor (CTV canonical: floor(50/2) = 25).
const LOW_WATER_MARK: usize = 25;
/// The expected `N` cap (CTV canonical).
const N_CAP: u64 = 50;

fn empty_registry() -> Registry {
    Registry {
        schema_version: 1,
        defaults: Default::default(),
        hooks: vec![],
    }
}

/// Write a real `.factory/shard-config.toml` under `cwd`, at the exact
/// relative path `executor.rs::SHARD_CONFIG_RELATIVE_PATH` resolves against
/// `base_host_ctx.cwd` — identical to the same helper in
/// `bc_1_18_006_roll_test.rs`.
fn write_shard_config(cwd: &std::path::Path, body: &str) {
    let factory_dir = cwd.join(".factory");
    std::fs::create_dir_all(&factory_dir).expect("create .factory dir");
    std::fs::write(factory_dir.join("shard-config.toml"), body).expect("write shard-config.toml");
}

/// Write `BC-INDEX.md` at `path` with exactly `n_items` `changelog:` sequence
/// items in its frontmatter. Items are written newest-first:
///   `  - date: 2026-01-<NN>\n    change: "item-<i>"\n`
/// (modular date so the fixture stays valid YAML regardless of `n_items`).
/// The frontmatter also carries `document_type`, `version`, and `last_amended`
/// fields, mirroring the real BC-INDEX.md shape.
fn write_bc_index_fixture(path: &std::path::Path, n_items: usize) {
    let mut changelog_block = String::new();
    if n_items > 0 {
        changelog_block.push_str("changelog:\n");
        for i in 0..n_items {
            // newest-first: item index descends
            let item_num = n_items - i;
            let day = (item_num % 28) + 1;
            let month = (item_num % 12) + 1;
            changelog_block.push_str(&format!(
                "  - date: 2026-{month:02}-{day:02}\n    change: \"item-{item_num}\"\n"
            ));
        }
    }
    let content = format!(
        "---\n\
         document_type: behavioral-contract-index\n\
         version: \"1.0\"\n\
         last_amended: \"2026-09-01 (v1.0) — test fixture\"\n\
         {changelog_block}\
         ---\n\n\
         # BC-INDEX Test Fixture\n"
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create BC-INDEX.md parent");
    }
    std::fs::write(path, &content).expect("write BC-INDEX.md fixture");
}

/// Count the number of `changelog:` items in the file at `path`.
///
/// Two-branch dispatch:
/// * If the file starts with `---\n` (a frontmatter-fenced source file such as
///   `BC-INDEX.md`): count `  - ` lines **inside** the frontmatter block only,
///   using the same approach as `read_changelog_item_count` in
///   `shard_manager.rs` but implemented here independently so this test module
///   does not gain a dependency on a private function.
/// * Otherwise (a fence-less archive file such as
///   `BC-INDEX-changelog-archive.md`): count `  - ` lines across the **whole**
///   file. Archive files contain only raw YAML sequence items appended by
///   `rotate_changelog_at` (no frontmatter, no false-positive `  - ` lines),
///   so counting globally gives a correct item total.
fn count_changelog_items(path: &std::path::Path) -> usize {
    let content = std::fs::read_to_string(path).expect("read fixture for changelog item count");
    if let Some(after_open) = content.strip_prefix("---\n") {
        // Frontmatter-fenced source file: count `  - ` lines inside the
        // frontmatter block only — adequate for the well-formed fixtures this
        // test file's own `write_bc_index_fixture` produces.
        let end = after_open
            .find("\n---")
            .expect("frontmatter-fenced file must have closing frontmatter fence");
        let block = &after_open[..end];
        // Count sequence item lines (lines starting with "  - ") — correct for
        // the flat YAML map items this test's own `write_bc_index_fixture` produces.
        block.lines().filter(|l| l.starts_with("  - ")).count()
    } else {
        // Fence-less archive file: count all `  - ` lines across the whole file.
        content.lines().filter(|l| l.starts_with("  - ")).count()
    }
}

/// Mirrors `inputs_for` in `bc_1_18_006_roll_test.rs` (MAJOR-3 shape:
/// `shard_cap_precheck` is computed externally and passed to `execute_tiers`).
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
    let mut base = HostContext::new("", "0.0.1", "sess-bc-1-18-009", "trace-bc-1-18-009");
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
        session_id: "sess-bc-1-18-009".to_string(),
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

/// Drive a single Edit/Write/MultiEdit PreToolUse dispatch through
/// `execute_tiers` with the B1 shard config and an empty tier list — the same
/// "reaches the native gate, nothing else runs" shape `bc_1_18_006_roll_test.rs`
/// uses for mechanism-A roll tests.
async fn run_b1_gate(
    dir: &std::path::Path,
    target: &std::path::Path,
    tool_name: &str,
    tool_input_extra: serde_json::Value,
) -> factory_dispatcher::executor::TierExecutionSummary {
    write_shard_config(dir, B1_SHARD_CONFIG);

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

/// Extract the exact (un-truncated, un-`Debug`-escaped) `reason` string from a
/// blocking dispatch outcome — mirrors `exact_block_reason` in
/// `bc_1_18_006_roll_test.rs` (which is also private to that test binary).
/// Panics if no blocking outcome carries a JSON `reason` field; callers only
/// invoke this after asserting the dispatch actually blocked.
fn exact_block_reason(summary: &factory_dispatcher::executor::TierExecutionSummary) -> String {
    for outcome in &summary.per_plugin_results {
        if let PluginResult::Ok { stdout, .. } = &outcome.result
            && stdout.contains(r#""outcome":"block""#)
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(stdout)
            && let Some(reason) = value.get("reason").and_then(|r| r.as_str())
        {
            return reason.to_string();
        }
    }
    panic!(
        "exact_block_reason: no per_plugin_results outcome carried a \
         {{\"outcome\":\"block\",\"reason\":...}} stdout payload. Got: {:?}",
        summary.per_plugin_results
    );
}

// ---------------------------------------------------------------------------
// AC-015 / CTV #1 — Happy-path block-and-retry: N=50 items, low_water_mark=25
// (EC-001: changelog: at N items, agent's write arrives)
// ---------------------------------------------------------------------------

/// AC-015, CTV #1 (Edit variant): `BC-INDEX.md` at exactly N=50 `changelog:`
/// items; an Edit dispatch triggers rotation (`item_count_trigger_fires` is
/// true), `rotate_changelog_at` trims the live sequence to `low_water_mark=25`
/// items and appends the 25 overflow items to the SINGLE evergreen archive
/// `BC-INDEX-changelog-archive.md`, then the gate returns `HookResult::Block`
/// with the Postcondition 2 step-3 retry instruction.
#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_009_AC015_CTV1_over_n_edit_dispatch_rotates_and_blocks() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("BC-INDEX.md");
    write_bc_index_fixture(&target, N_CAP as usize);

    // Precondition check: fixture must have exactly N items.
    assert_eq!(
        count_changelog_items(&target),
        N_CAP as usize,
        "precondition: fixture must have exactly N={N_CAP} changelog items"
    );

    let summary = run_b1_gate(
        dir.path(),
        &target,
        "Edit",
        serde_json::json!({"old_string": "test", "new_string": "test-new"}),
    )
    .await;

    // BC-1.18.009 Invariant 4: Block is the ONLY outcome on a successful
    // rotation — never Continue.
    assert_ne!(
        summary.exit_code, 0,
        "AC-015/Invariant 4: an over-N Edit must resolve to HookResult::Block, a blocking \
         dispatch outcome (exit_code != 0) — never a silent Continue after rotation"
    );
    assert!(
        summary.block_intent,
        "AC-015: block_intent must be set for a fired item-count rotation trigger"
    );

    // BC-1.18.009 Postcondition 2 step 3: Block reason must name the archive
    // path and the post-rotation item count (low_water_mark).
    let reason = exact_block_reason(&summary);
    let expected_archive_name = "BC-INDEX-changelog-archive.md";
    assert!(
        reason.contains(expected_archive_name),
        "AC-015/PC2: Block reason must name the evergreen archive file \
         '{expected_archive_name}'. Got: {reason}"
    );
    assert!(
        reason.contains(&LOW_WATER_MARK.to_string()),
        "AC-015/PC2: Block reason must state the post-rotation item count \
         (low_water_mark={LOW_WATER_MARK}). Got: {reason}"
    );

    // BC-1.18.009 Postcondition 2: the single evergreen archive file must
    // exist (rotate_changelog_at appended the 25 overflow items to it).
    let archive_path = dir.path().join("BC-INDEX-changelog-archive.md");
    assert!(
        archive_path.exists(),
        "AC-015/PC2: the single evergreen archive file must exist after rotation: {}",
        archive_path.display()
    );

    // VP-125 (bounded-live-sequence facet): live sequence trimmed to
    // low_water_mark=25, never to N-1=49 (the withdrawn design).
    let live_count = count_changelog_items(&target);
    assert_eq!(
        live_count,
        LOW_WATER_MARK,
        "AC-015/VP-125: live sequence must be trimmed to low_water_mark={LOW_WATER_MARK}, \
         NEVER to N-1={} — the withdrawn N-1 framing is explicitly unsound \
         (BC-1.18.009 Postcondition 1, fix-burst pass-3 F-P3-005)",
        N_CAP - 1
    );

    // VP-125 (no-history-loss facet): archive must contain the 25 overflow
    // items (the oldest items beyond the retained low_water_mark).
    let archive_count = count_changelog_items(&archive_path);
    assert_eq!(
        archive_count,
        N_CAP as usize - LOW_WATER_MARK,
        "AC-015/VP-125: archive must contain exactly {} overflow items (N={N_CAP} - \
         low_water_mark={LOW_WATER_MARK}). Live={live_count}, archive={archive_count}",
        N_CAP as usize - LOW_WATER_MARK
    );
}

/// AC-015, CTV #1 (Write variant): same contract as the Edit variant above but
/// for a `Write` tool call — BC-1.18.009 Postcondition 2 applies identically
/// regardless of whether the originating call is an `Edit` or `Write`.
#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_009_AC015_CTV1_over_n_write_dispatch_rotates_and_blocks() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("BC-INDEX.md");
    write_bc_index_fixture(&target, N_CAP as usize);

    let summary = run_b1_gate(
        dir.path(),
        &target,
        "Write",
        serde_json::json!({"content": "---\ndocument_type: x\n---\n"}),
    )
    .await;

    assert_ne!(
        summary.exit_code, 0,
        "AC-015: an over-N Write must also resolve to HookResult::Block, identically \
         to the Edit case — BC-1.18.009 Postcondition 2 does not distinguish tool kind"
    );
    assert!(
        summary.block_intent,
        "AC-015: block_intent must be set for a Write-triggered item-count rotation"
    );

    let archive_path = dir.path().join("BC-INDEX-changelog-archive.md");
    assert!(
        archive_path.exists(),
        "AC-015/PC2: the single evergreen archive must exist after a Write-triggered rotation"
    );

    let live_count = count_changelog_items(&target);
    assert_eq!(
        live_count, LOW_WATER_MARK,
        "AC-015/VP-125: live sequence must be trimmed to low_water_mark={LOW_WATER_MARK} \
         by a Write-triggered rotation, identically to the Edit case"
    );
}

// ---------------------------------------------------------------------------
// AC-015 / EC-002 / CTV #3 — Below-threshold Continue
// The gate returns Continue when item count < N=50; the rotation branch is
// never taken for below-threshold dispatches.
// ---------------------------------------------------------------------------

/// AC-015, EC-002, CTV #3: `BC-INDEX.md` at 10 items, N=50. The item-count
/// trigger does NOT fire (`10 + 1 = 11 <= 50`). The gate returns `Continue`
/// and the agent's own original call is allowed to proceed unmodified.
/// No rotation takes place; the archive file is NOT created.
#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_009_AC015_EC002_CTV3_below_n_continues_without_rotation() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("BC-INDEX.md");
    write_bc_index_fixture(&target, 10);

    let summary = run_b1_gate(
        dir.path(),
        &target,
        "Write",
        serde_json::json!({"content": "---\ndocument_type: x\n---\n"}),
    )
    .await;

    assert_eq!(
        summary.exit_code, 0,
        "AC-015/EC-002/CTV#3: a below-N dispatch (10 items, N=50) must return \
         HookResult::Continue (exit_code == 0) — no rotation, no block"
    );
    assert!(
        !summary.block_intent,
        "AC-015/EC-002: block_intent must NOT be set for a below-threshold dispatch"
    );

    // No archive file must have been created.
    let archive_path = dir.path().join("BC-INDEX-changelog-archive.md");
    assert!(
        !archive_path.exists(),
        "AC-015/EC-002: the archive file must NOT be created for a below-threshold dispatch \
         (no rotation occurred)"
    );

    // Fixture must be unchanged (no rotation ran).
    assert_eq!(
        count_changelog_items(&target),
        10,
        "AC-015/EC-002: the fixture's changelog: item count must be unchanged (10) after a \
         below-threshold Continue"
    );
}

// ---------------------------------------------------------------------------
// AC-016 / VP-131 / CTV #4 — Rotation failure: E-SHD-004 + pre-rotation state
// preserved (EC-003 fail-loud pattern)
// ---------------------------------------------------------------------------

/// AC-016, VP-131, CTV #4, EC-003: When `rotate_changelog_at` itself fails
/// (here: the archive path is pre-created as a directory, so `write_atomic`
/// cannot rename a temp file over a directory target → EISDIR / permission
/// error), the gate must return `HookResult::Error` with message starting
/// `"E-SHD-004: rotate_changelog invocation failed for \"BC-INDEX\": ..."` —
/// NEVER `E-SHD-001` (which is BC-1.18.006's distinct mechanism-A error code,
/// Postcondition 6 CORRECTED fix-burst pass-3 F-P3-001). The frontmatter's
/// `changelog:` sequence must be left byte-identical to its pre-rotation state.
#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_009_AC016_VP131_CTV4_rotation_failure_returns_e_shd_004_state_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("BC-INDEX.md");
    write_bc_index_fixture(&target, N_CAP as usize);

    // Capture the pre-rotation frontmatter content for byte-identity check
    // (BC-1.18.009 Postcondition 6 / VP-131: pre-rotation state preserved).
    let pre_rotation_content = std::fs::read_to_string(&target)
        .expect("VP-131 precondition: read pre-rotation fixture content");

    // Inject rotation failure: pre-create the archive path AS A DIRECTORY so
    // `write_atomic`'s rename-over-directory attempt fails with EISDIR.
    // `rotate_changelog_at` computes `archive_path = target.parent() /
    // "BC-INDEX-changelog-archive.md"` — creating that path as a dir before
    // the dispatch ensures the write step cannot succeed.
    let archive_as_dir = dir.path().join("BC-INDEX-changelog-archive.md");
    std::fs::create_dir_all(&archive_as_dir)
        .expect("VP-131 fixture: pre-create archive path as a directory");
    assert!(
        archive_as_dir.is_dir(),
        "VP-131 fixture precondition: archive path must be a directory before dispatch"
    );

    let summary = run_b1_gate(
        dir.path(),
        &target,
        "Edit",
        serde_json::json!({"old_string": "test", "new_string": "test-new"}),
    )
    .await;

    // VP-131: rotation failure MUST return HookResult::Error, never Block or
    // Continue.
    assert_ne!(
        summary.exit_code, 0,
        "AC-016/VP-131: a rotation failure must result in a non-zero exit_code"
    );

    // Check for an Error outcome (vs a Block outcome).
    let mut found_error = false;
    let mut error_message = String::new();
    for outcome in &summary.per_plugin_results {
        if let PluginResult::Ok { stdout, .. } = &outcome.result
            && stdout.contains(r#""outcome":"error""#)
        {
            found_error = true;
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(stdout)
                && let Some(msg) = value.get("message").and_then(|m| m.as_str())
            {
                error_message = msg.to_string();
            }
        }
    }
    assert!(
        found_error,
        "AC-016/VP-131: rotation failure must produce HookResult::Error (\"outcome\":\"error\" \
         in per_plugin_results), never Block or Continue. Got: {:?}",
        summary.per_plugin_results
    );

    // BC-1.18.009 Postcondition 6 (CORRECTED fix-burst pass-3 F-P3-001):
    // error code must be E-SHD-004 (this BC's OWN rotate_changelog failure
    // code) — NEVER E-SHD-001 (BC-1.18.006's distinct mechanism-A error code).
    assert!(
        error_message.starts_with("E-SHD-004: rotate_changelog invocation failed for"),
        "AC-016/VP-131: error message must start with \
         \"E-SHD-004: rotate_changelog invocation failed for\" — NOT E-SHD-001 \
         (which is BC-1.18.006's different mechanism-A shard-seal-write failure code). \
         Got: {error_message}"
    );
    assert!(
        error_message.contains("BC-INDEX"),
        "AC-016/VP-131: error message must name the artifact stem (\"BC-INDEX\"). \
         Got: {error_message}"
    );

    // VP-131 (pre-rotation state preserved): the frontmatter must be
    // byte-identical to its pre-rotation state — no partial rotation was
    // written to disk.
    let post_attempt_content =
        std::fs::read_to_string(&target).expect("VP-131: read post-attempt fixture content");
    assert_eq!(
        post_attempt_content, pre_rotation_content,
        "AC-016/VP-131: the frontmatter changelog: sequence must be byte-identical to its \
         pre-rotation state after a rotate_changelog_at failure — fail-loud, no partial state \
         left authoritative (BC-1.18.009 Postcondition 6 / EC-003)"
    );
}

// ---------------------------------------------------------------------------
// AC-015 / EC-007 / CTV #2 — Amortized rotation cadence (EC-007 is NOT a
// defect): after a rotation that trims to low_water_mark=25, the next
// N - low_water_mark - 1 = 24 writes land via plain Continue before the
// item count reaches N=50 again and re-triggers Block+rotation.
// ---------------------------------------------------------------------------

/// AC-015, EC-007, CTV #2: Demonstrates the amortized rotation cadence that
/// BC-1.18.009 Postcondition 1's fix-burst (F-P3-005) introduces, replacing
/// the withdrawn every-write-rotation pathology.
///
/// Scenario:
///   1. File at N=50 items → dispatch → Block + rotation (live trimmed to 25)
///   2. Agent's retried write lands (file at 26 items) → Continue
///   3. Items 27, 28, ..., 49 → Continue (23 more writes, 24 total)
///   4. File at 50 items (second rotation boundary) → Block (EC-007 re-trigger)
///
/// EC-007 explicitly states this re-trigger is "NOT a defect — the SAME
/// single-actor block-and-retry contract applies identically to this
/// re-trigger as to the first-ever rotation."
#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_009_AC015_EC007_CTV2_amortized_cadence_24_continues_then_retriggers() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("BC-INDEX.md");

    // Step 1: file at N=50 items → dispatch → Block + rotation.
    write_bc_index_fixture(&target, N_CAP as usize);
    let summary_block_1 = run_b1_gate(
        dir.path(),
        &target,
        "Edit",
        serde_json::json!({"old_string": "test", "new_string": "test-new"}),
    )
    .await;

    assert_ne!(
        summary_block_1.exit_code, 0,
        "AC-015/EC-007/CTV#2 step 1: first rotation trigger (N=50 items) must Block"
    );
    assert_eq!(
        count_changelog_items(&target),
        LOW_WATER_MARK,
        "AC-015/EC-007 step 1: after first rotation, live sequence must be \
         trimmed to low_water_mark={LOW_WATER_MARK}"
    );

    // Step 2: agent's retried write lands (low_water_mark + 1 = 26 items).
    // In the real flow, the agent re-reads the post-rotation file and
    // prepends ONE item. We simulate this by writing a 26-item fixture.
    write_bc_index_fixture(&target, LOW_WATER_MARK + 1);

    // Steps 2–25: 24 successive writes land via Continue (items 26..49).
    // `items_before_dispatch` counts what's in the file BEFORE each dispatch;
    // the gate checks CURRENT (pre-write) item count → trigger fires when
    // current + 1 > N, i.e. current >= N = 50.
    for items_before in (LOW_WATER_MARK + 1)..=(N_CAP as usize - 1) {
        write_bc_index_fixture(&target, items_before);
        let summary_continue = run_b1_gate(
            dir.path(),
            &target,
            "Write",
            serde_json::json!({"content": "---\ndocument_type: x\n---\n"}),
        )
        .await;
        assert_eq!(
            summary_continue.exit_code,
            0,
            "AC-015/EC-007/CTV#2: dispatch on {items_before} items (< N={N_CAP}) must Continue \
             — amortized cadence: only once per N - low_water_mark = {} writes is rotation \
             re-triggered, not on every write",
            N_CAP - LOW_WATER_MARK as u64
        );
    }

    // Step 4 (EC-007 re-trigger): file at N=50 items again → Block.
    write_bc_index_fixture(&target, N_CAP as usize);
    let summary_block_2 = run_b1_gate(
        dir.path(),
        &target,
        "Edit",
        serde_json::json!({"old_string": "test", "new_string": "test-new2"}),
    )
    .await;

    assert_ne!(
        summary_block_2.exit_code, 0,
        "AC-015/EC-007: the amortized re-trigger (file back at N=50 items) must ALSO Block — \
         EC-007 explicitly states this is NOT a defect but the EXPECTED amortized re-trigger \
         (BC-1.18.009 EC-007)"
    );
    assert!(
        summary_block_2.block_intent,
        "AC-015/EC-007: block_intent must be set on the amortized re-trigger, identically \
         to the first rotation"
    );
}

// ---------------------------------------------------------------------------
// AC-015 / CTV #5 — Stale-payload caller compliance (documented boundary)
// ---------------------------------------------------------------------------

/// AC-015, CTV #5: After a rotation that trims to low_water_mark=25 items,
/// a correctly-retried write (agent re-reads post-rotation file, prepends one
/// new item → file has 26 items before dispatch) returns `Continue` — the gate
/// sees `26 < N=50` and correctly allows the retry to proceed.
///
/// BC-1.18.009 CTV #5 documents the STALE payload scenario separately: "if [a
/// stale payload] lands, [it] re-introduces the just-rotated tail item —
/// this is a caller-compliance failure, not a gate defect." The gate itself
/// cannot and does not validate payload content for the
/// FrontmatterChangelogArray shape (it reads the CURRENT file state, not the
/// payload). This test validates the CORRECT retry path; the stale-payload
/// hazard is outside the gate's responsibility.
///
/// The gate sees 26 items (< N=50) and returns `Continue` — the rotation
/// branch is not taken for below-threshold item counts.
#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_009_AC015_CTV5_post_rotation_correct_retry_continues() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("BC-INDEX.md");

    // Simulate the post-rotation state: agent retried and prepended one item
    // to the low_water_mark=25 file → file now has 26 items.
    write_bc_index_fixture(&target, LOW_WATER_MARK + 1);

    let summary = run_b1_gate(
        dir.path(),
        &target,
        "Write",
        serde_json::json!({"content": "---\ndocument_type: x\n---\n"}),
    )
    .await;

    assert_eq!(
        summary.exit_code,
        0,
        "AC-015/CTV#5: a correctly-retried write (post-rotation file at \
         low_water_mark+1={} items < N={N_CAP}) must Continue — \
         single-actor model: the gate makes room (rotation), then the agent's \
         retry lands unimpeded",
        LOW_WATER_MARK + 1
    );
    assert!(
        !summary.block_intent,
        "AC-015/CTV#5: block_intent must NOT be set for a correctly-retried write \
         at low_water_mark+1 items"
    );
}

// ---------------------------------------------------------------------------
// AC-016 / VP-125 — Single evergreen archive: no-history-loss across multiple
// rotations (Postcondition 5 / CTV #6)
// ---------------------------------------------------------------------------

/// AC-016, VP-125 (no-history-loss facet), CTV #6: two successive rotations
/// BOTH append to the SAME single evergreen archive file at the sibling path
/// `BC-INDEX-changelog-archive.md`. The second rotation's appended items appear
/// AFTER the first rotation's items in the archive file (oldest-to-newest
/// append order). No prior archive content is ever overwritten or truncated.
///
/// This tests BC-1.18.009 Postcondition 5's "every `rotate_changelog`
/// invocation APPENDS to the SAME single destination file — no prior appended
/// content is ever overwritten, truncated, or deleted by a subsequent
/// rotation."
#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_009_AC016_VP125_single_evergreen_archive_no_history_loss() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("BC-INDEX.md");
    let archive_path = dir.path().join("BC-INDEX-changelog-archive.md");

    // First rotation: 50 items → Block + 25 archived, 25 retained.
    write_bc_index_fixture(&target, N_CAP as usize);
    let summary1 = run_b1_gate(
        dir.path(),
        &target,
        "Edit",
        serde_json::json!({"old_string": "a", "new_string": "b"}),
    )
    .await;
    assert_ne!(
        summary1.exit_code, 0,
        "VP-125 precondition: first rotation must Block"
    );
    assert!(
        archive_path.exists(),
        "VP-125 precondition: archive must exist after first rotation"
    );
    let archive_after_rotation_1 =
        std::fs::read_to_string(&archive_path).expect("VP-125: read archive after first rotation");
    let archive_items_1 = count_changelog_items(&archive_path);
    assert_eq!(
        archive_items_1,
        N_CAP as usize - LOW_WATER_MARK,
        "VP-125: first rotation must archive exactly {} items",
        N_CAP as usize - LOW_WATER_MARK
    );

    // Second rotation: reset the live file to 50 items (simulates
    // low_water_mark + 25 agent prepends landing between rotations).
    write_bc_index_fixture(&target, N_CAP as usize);
    let summary2 = run_b1_gate(
        dir.path(),
        &target,
        "Edit",
        serde_json::json!({"old_string": "a", "new_string": "b"}),
    )
    .await;
    assert_ne!(
        summary2.exit_code, 0,
        "VP-125: second rotation must also Block"
    );

    // The archive must contain BOTH the first and second rotation's items —
    // the second rotation APPENDED to the archive, never overwrote it.
    let archive_after_rotation_2 =
        std::fs::read_to_string(&archive_path).expect("VP-125: read archive after second rotation");
    assert!(
        archive_after_rotation_2.starts_with(&archive_after_rotation_1),
        "AC-016/VP-125/PC5: the second rotation must APPEND to the existing archive — \
         the first rotation's content must be preserved verbatim at the start of the \
         archive file (no overwrite or truncation). \
         Archive-after-1 prefix check FAILED.\n\
         After rotation 1: {archive_after_rotation_1:?}\n\
         After rotation 2: {archive_after_rotation_2:?}"
    );

    let archive_items_2 = count_changelog_items(&archive_path);
    assert_eq!(
        archive_items_2,
        2 * (N_CAP as usize - LOW_WATER_MARK),
        "AC-016/VP-125: archive after TWO rotations must contain exactly {} items \
         ({} per rotation × 2 — no history lost, no items deduplicated)",
        2 * (N_CAP as usize - LOW_WATER_MARK),
        N_CAP as usize - LOW_WATER_MARK
    );
}

// ---------------------------------------------------------------------------
// AC-015 / INV-1 / VP-126 — Zero `prepend_changelog_item` call sites in the
// B1 handler (static source scan, CI-enforceable)
// ---------------------------------------------------------------------------

/// AC-015, Invariant 1, VP-126: verifies that `shard_manager.rs` contains
/// ZERO call sites for `prepend_changelog_item` — the function must remain
/// exclusively agent-side tooling (per ADR-049 §Decision 2), never called by
/// the gate itself. This is a load-bearing, CI-enforceable static check.
///
/// `prepend_changelog_item` has never appeared in `shard_manager.rs`
/// (confirmed by the F1 delta analysis §A Verified-Current-State Greps and
/// by the fully-implemented B1 handler). This static scan enforces that
/// invariant stays true as the codebase evolves.
///
/// Note: VP-126 also requires "no reimplemented rotation/trim/validate/write
/// logic other than a call into `rotate_changelog_at`" in the B1 handler.
/// That facet is a code-review / formal-verifier concern (requires structural
/// analysis beyond a grep); this test covers only the static-scan facet
/// (`prepend_changelog_item` call-site count = 0) which is directly
/// expressible as a source-scan assertion (CI-enforceable without external
/// tooling, analogous to the grep pattern cited in BC-1.18.009 §Verification
/// Properties VP-126 row).
#[test]
fn test_BC_1_18_009_AC015_INV1_VP126_zero_prepend_changelog_item_callsites_in_shard_manager() {
    let shard_manager_src = include_str!("../src/shard_manager.rs");

    let callsite_count = shard_manager_src.matches("prepend_changelog_item").count();

    assert_eq!(
        callsite_count, 0,
        "AC-015/INV-1/VP-126: `prepend_changelog_item` must appear ZERO times in \
         `shard_manager.rs` — the gate's B1 handler calls ONLY `rotate_changelog_at`, \
         never `prepend_changelog_item` (BC-1.18.009 Invariant 1; the function is \
         imported/used exclusively by agent-side tooling per ADR-049 §Decision 2, never \
         by the gate). Found {} occurrence(s).",
        callsite_count
    );
}

// ---------------------------------------------------------------------------
// AC-015 — `build_b1_block_reason` message format pinned verbatim
// Pure string template, zero branching — tests the exact BC-prescribed text.
// ---------------------------------------------------------------------------

/// AC-015: Pins `build_b1_block_reason`'s output byte-for-byte against
/// BC-1.18.009 Postcondition 2 step 3's prescribed retry-instruction text.
///
/// Per the L-BB-D1179 verbatim-pin lesson (no weak `.contains()` substring
/// checks — a sibling template `build_empty_roll_retry_block_reason` diverged
/// from its spec text while a substring-only test stayed green through the
/// divergence, F-C2-P5-002/F-C2-P6-002): asserts the FULL string verbatim, not
/// just individual substrings.
///
/// `build_b1_block_reason` is a pure, zero-branching string template function.
#[test]
fn test_BC_1_18_009_AC015_build_b1_block_reason_format_pinned_verbatim() {
    // Use a canonical archive path matching the gate's expected sibling path
    // derivation (target.parent() / "BC-INDEX-changelog-archive.md").
    let archive_path = std::path::Path::new(
        "/tmp/bc-1-18-009-test/.factory/specs/behavioral-contracts/BC-INDEX-changelog-archive.md",
    );

    let actual = build_b1_block_reason("BC-INDEX", archive_path, LOW_WATER_MARK);

    // Expected text per BC-1.18.009 Postcondition 2 step 3 (verbatim
    // blockquote), with `artifact_stem`="BC-INDEX", `keep_recent`=25, and
    // `archive_path` rendered via `Display` (Unix: the path string directly).
    let expected = format!(
        "`BC-INDEX`'s `changelog:` sequence was rotated to make room \
         (oldest item(s) appended to `{}`); the frontmatter now has {LOW_WATER_MARK} items. \
         Retry your write: if you used `Edit`, reissue as a fresh `Write` or a fresh \
         `Edit` re-read against the current (post-rotation) file, since your original \
         `old_string`/`new_string` pair may no longer match; if you used `Write`, \
         recompute your `content` payload against the current (post-rotation) file before \
         retrying — do not resubmit your original payload unchanged, since it reflects \
         pre-rotation state.",
        archive_path.display()
    );

    assert_eq!(
        actual, expected,
        "AC-015: build_b1_block_reason's output must match BC-1.18.009 Postcondition 2 \
         step 3's prescribed retry-instruction text VERBATIM (L-BB-D1179 verbatim-pin \
         lesson: no substring-only check — the sibling template build_empty_roll_retry_\
         block_reason diverged from its own spec text while a substring-only test stayed \
         green, F-C2-P5-002/F-C2-P6-002; this pins the FULL string)"
    );
}

// ---------------------------------------------------------------------------
// EC-008 / Invariant 5 (v1.6 hardening) — counter-divergence guard:
// trigger fires but rotate_changelog_at returns mutated=false → Error(E-SHD-014)
// ---------------------------------------------------------------------------

/// Write `BC-INDEX.md` at `path` with `n_items` items in YAML INLINE (flow)
/// sequence form: `changelog: [1, 2, 3, ..., n_items]`.
///
/// This deliberately diverges from canonical `  - date: / change:` block form
/// to exercise EC-008's counter-divergence scenario:
///
/// * `read_changelog_item_count` (serde_norway path): sees a valid YAML
///   `Vec<Value>` with `n_items` elements → count = `n_items` → trigger fires
///   when `n_items >= N`.
/// * `parse_frontmatter`'s line-scan path (`changelog_sequence_bounds`):
///   requires `line == "changelog:\n"` (exact match with NO inline value) —
///   `"changelog: [1, 2, ...]\n"` does NOT match → `None` → `extract_changelog`
///   falls through to `(true, Vec::new())` → `changelog_items_raw.len() = 0`.
/// * After implementation: `rotate_changelog_at` calls `parse_frontmatter`,
///   gets `total = 0 <= keep_recent = 25` → returns `RotationReport {
///   mutated: false, items_moved: 0 }`.
/// * EC-008 guard: gate MUST return `Error(E-SHD-014)`, never `Block`.
fn write_bc_index_inline_seq_fixture(path: &std::path::Path, n_items: usize) {
    let items: Vec<String> = (1..=n_items).map(|i| i.to_string()).collect();
    let inline = items.join(", ");
    let content = format!(
        "---\n\
         document_type: behavioral-contract-index\n\
         version: \"1.0\"\n\
         last_amended: \"2026-09-01 (v1.0) — test fixture\"\n\
         changelog: [{inline}]\n\
         ---\n\n\
         # BC-INDEX Inline-Sequence Test Fixture (EC-008 counter-divergence)\n"
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create inline fixture parent");
    }
    std::fs::write(path, &content).expect("write inline-sequence fixture");
}

/// EC-008, Invariant 5 (v1.6 hardening): the item-count trigger fires
/// (`read_changelog_item_count` serde count = 50 >= N = 50) but
/// `rotate_changelog_at` returns `Ok(report)` with `report.mutated == false`
/// (counter-method divergence: `parse_frontmatter`'s line-scan counts 0 items
/// in the YAML-inline-sequence fixture). The gate MUST return
/// `HookResult::Error` (a BLOCKING fail-loud error with block_intent=true,
/// exit_code=2, on_error=Block) carrying a message beginning `"E-SHD-014:"`.
/// It MUST NEVER return `HookResult::Block` (the retry-instruction variant).
///
/// "Never Block" (BC-1.18.009 v1.6 Invariant 5) means never the
/// `HookResult::Block` VARIANT (the retry instruction), NOT "never blocks the
/// dispatch." E-SHD-014 IS blocking — block_intent=true — but via the Error
/// variant, not the Block variant. Emitting the Block variant on a
/// `mutated=false` report would send the retrying agent into a permanent
/// self-DoS block+retry loop on `BC-INDEX.md` for the session.
///
/// The frontmatter must be byte-identical to the pre-attempt state (no partial
/// rotation was written).
///
/// No test seam is required: the divergence is induced naturally by the
/// YAML-inline-sequence fixture format (counter method difference between
/// serde_norway deserialization and the `  - date:` line-scan in
/// `changelog_sequence_bounds`). The `FrontmatterChangelogArray` trigger-fired
/// branch in `shard_manager.rs` returns a BLOCKING
/// `HookResult::Error { message: "E-SHD-014: ..." }` (on_error=Block →
/// exit_code=2, block_intent=true) when `report.mutated == false`.
#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_009_EC008_INV5_mutated_false_returns_e_shd_014_error_variant_still_blocking()
{
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("BC-INDEX.md");

    // Inline-sequence fixture: serde counts N=50 items (trigger fires);
    // line-scan counts 0 (changelog_sequence_bounds exact-match fails for
    // inline form) → rotate_changelog_at returns mutated=false after
    // implementation.
    write_bc_index_inline_seq_fixture(&target, N_CAP as usize);

    let pre_attempt_content =
        std::fs::read_to_string(&target).expect("EC-008: read pre-attempt fixture content");

    let summary = run_b1_gate(
        dir.path(),
        &target,
        "Edit",
        serde_json::json!({"old_string": "test", "new_string": "test-new"}),
    )
    .await;

    // EC-008 / Invariant 5: the gate MUST return Error(E-SHD-014) — BLOCKING
    // (block_intent=true, exit_code=2) — when rotate_changelog_at returns
    // mutated=false. MUST NOT return HookResult::Block (the retry variant).
    assert_eq!(
        summary.exit_code, 2,
        "EC-008/Inv-5: E-SHD-014 is a BLOCKING fail-loud Error — exit_code must be 2 \
         (same blocking treatment as E-SHD-004; on_error=Block)"
    );
    assert!(
        summary.block_intent,
        "EC-008/Inv-5: E-SHD-014 is a BLOCKING fail-loud Error (block_intent=true, \
         on_error=Block) — 'never Block' (BC-1.18.009 v1.6 Inv-5) means never the \
         HookResult::Block VARIANT (the retry instruction), NOT 'never blocks the \
         dispatch.' The Error variant with on_error=Block also sets block_intent=true."
    );

    // Verify an Error outcome (not a Block outcome) is reported.
    let mut found_error = false;
    let mut error_message = String::new();
    for outcome in &summary.per_plugin_results {
        if let PluginResult::Ok { stdout, .. } = &outcome.result {
            if stdout.contains(r#""outcome":"error""#) {
                found_error = true;
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(stdout)
                    && let Some(msg) = value.get("message").and_then(|m| m.as_str())
                {
                    error_message = msg.to_string();
                }
            }
            // Explicitly assert no Block outcome was emitted
            assert!(
                !stdout.contains(r#""outcome":"block""#),
                "EC-008/Inv-5: a Block outcome MUST NOT be emitted when rotate_changelog_at \
                 returns mutated=false — emitting Block would send the retrying agent into a \
                 permanent self-DoS retry loop (BC-1.18.009 v1.6 Invariant Inv-5)"
            );
        }
    }
    assert!(
        found_error,
        "EC-008/Inv-5: an Error outcome (\"outcome\":\"error\") MUST be present in \
         per_plugin_results when rotate_changelog_at returns mutated=false. Got: {:?}",
        summary.per_plugin_results
    );

    // BC-1.18.009 v1.6 Postcondition 6 + Invariant 5: error code must be
    // E-SHD-014 (NOT E-SHD-004 which is the rotate_changelog Err-arm failure;
    // NOT E-SHD-001 which belongs to BC-1.18.006's mechanism-A).
    assert!(
        error_message.starts_with("E-SHD-014:"),
        "EC-008/Inv-5: error message must start with \"E-SHD-014:\" — the counter-divergence \
         guard uses BC-1.18.009 v1.6's own distinct error code; NEVER E-SHD-004 (the Err-arm \
         rotate_changelog invocation failure code) or E-SHD-001 (BC-1.18.006 mechanism-A code). \
         Got: {error_message}"
    );

    // Frontmatter byte-identity: the inline-sequence content is unchanged
    // (no partial rotation wrote to disk — the guard fires before any mutation).
    let post_attempt_content =
        std::fs::read_to_string(&target).expect("EC-008: read post-attempt fixture content");
    assert_eq!(
        post_attempt_content, pre_attempt_content,
        "EC-008/Inv-5: the frontmatter must be byte-identical to the pre-attempt state — \
         no partial rotation was written to disk (the Error(E-SHD-014) guard fires before \
         any file mutation)"
    );

    // No archive file must have been created.
    let archive_path = dir.path().join("BC-INDEX-changelog-archive.md");
    assert!(
        !archive_path.exists(),
        "EC-008/Inv-5: the archive file must NOT be created when the guard fires \
         (mutated=false means no rotation happened)"
    );
}
