// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! BC-1.18.006 (S-25.02 cluster-2 "roll") integration coverage for the
//! `executor.rs` -> `shard_manager.rs` roll wiring AND the `invoke.rs`
//! Postcondition 7 catch-point (i) qualifying wrapper -> `main::run` wiring.
//!
//! # BC-5.38.001 Red Gate discipline — every test below MUST currently FAIL
//!
//! `shard_manager.rs`'s BC-1.18.006 functions (`execute_roll`,
//! `read_canonical_content`, `publish_sealed_shard`,
//! `truncate_canonical_to_empty`, `publish_shard_index_update`,
//! `reconcile_post_write_replace_all_overcap`,
//! `reconcile_leading_probe_backstop`, and the two self-heal functions) are
//! ALL `todo!()` as of the stub-architect's cluster-2 burst (commit
//! `e04ab76f`). Every test below drives a REAL call path into one of these
//! stubs and therefore panics today. Each test asserts the REAL,
//! post-implementation expected outcome (never `#[should_panic]`) — the same
//! methodology `bc_1_18_005_shard_cap_trigger_test.rs`'s own header comment
//! already establishes for this crate: a test written this way is RED today
//! (fails via panic) and turns GREEN, unmodified, once implementer replaces
//! the stub with real logic.
//!
//! # BC ambiguity flagged for product-owner (not resolved by this burst)
//!
//! BC-1.18.006 v1.4's own "Canonical Test Vectors" table's first row ("Write
//! to decision-log.md, current shard 45,000 bytes, content length 5,000
//! bytes, cap 49,152") is internally inconsistent with Postcondition 3's
//! CORRECTED `Write` formula (`projected_size = len(content)` ALONE —
//! `current_shard_bytes` is NEVER added for `Write`, per the BC's own
//! Postcondition 3 text and this story's already-implemented, already-tested
//! AC-002). Under that formula, `len(content) = 5,000 <= 49,152`, so the
//! vector's own inputs would NOT trigger a roll at all — it appears to be a
//! stale carry-over from the WITHDRAWN pre-F-P2-002 uniform
//! `current_shard_bytes + payload_bytes` formula, never updated to a
//! self-consistent example after that fix-burst corrected the `Write`
//! formula. Tests below use independently self-consistent worked numbers
//! (content length ALONE exceeding cap for `Write`) rather than copying that
//! vector's contradictory raw numbers verbatim. Routing: product-owner
//! (BC-1.18.006 owner) should amend the vector's numeric example in a future
//! burst; this is a documentation-only inconsistency, not a code defect, and
//! does not block this cluster's TDD.

use std::sync::Arc;

use factory_dispatcher::engine::build_engine;
use factory_dispatcher::executor::{ExecutorInputs, execute_tiers};
use factory_dispatcher::host::HostContext;
use factory_dispatcher::internal_log::InternalLog;
use factory_dispatcher::invoke::reconcile_replace_all_overcap_if_qualifying;
use factory_dispatcher::payload::HookPayload;
use factory_dispatcher::plugin_loader::PluginCache;
use factory_dispatcher::registry::Registry;
use factory_dispatcher::resolver::ResolverRegistry;

/// A well-formed `"flat"`-shaped `[[shard]]` config entry — identical
/// calibration constants to `bc_1_18_005_shard_cap_trigger_test.rs`'s own
/// `FLAT_SHARD_CONFIG` (cap 49,152, formula ceiling 50,640), reused here so
/// this cluster's over-cap fixtures are self-consistent with cluster-1's
/// already-validated formula ceiling.
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

fn write_shard_config(cwd: &std::path::Path, body: &str) {
    let factory_dir = cwd.join(".factory");
    std::fs::create_dir_all(&factory_dir).expect("create .factory dir");
    std::fs::write(factory_dir.join("shard-config.toml"), body).expect("write shard-config.toml");
}

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
) -> ExecutorInputs<'a> {
    let mut base = HostContext::new("", "0.0.1", "sess-bc-1-18-006", "trace-bc-1-18-006");
    base.cwd = cwd.to_path_buf();
    base.internal_log = Some(internal_log.clone());

    let mut tool_input = tool_input_extra;
    if let Some(map) = tool_input.as_object_mut() {
        map.insert(
            "file_path".to_string(),
            serde_json::Value::String(target_path.to_string_lossy().into_owned()),
        );
    }

    ExecutorInputs {
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
    }
}

/// Drives a single Edit/Write/MultiEdit PreToolUse dispatch through
/// `execute_tiers` with an empty tier list — the same "reaches the native
/// gate, nothing else runs" shape `bc_1_18_005_shard_cap_trigger_test.rs`'s
/// own `run_shard_gate_for_config` helper uses.
async fn run_roll_gate(
    dir: &std::path::Path,
    target: &std::path::Path,
    tool_name: &str,
    tool_input_extra: serde_json::Value,
) -> factory_dispatcher::executor::TierExecutionSummary {
    write_shard_config(dir, FLAT_SHARD_CONFIG);

    let engine = build_engine().unwrap();
    let cache = PluginCache::new(engine.clone());
    let registry = empty_registry();
    let internal_log = Arc::new(InternalLog::new(dir.join("logs")));

    let inputs = inputs_for(
        &engine,
        &cache,
        &registry,
        &internal_log,
        dir,
        tool_name,
        target,
        tool_input_extra,
    );

    execute_tiers(inputs, vec![]).await
}

fn sealed_path_for(dir: &std::path::Path, stem: &str, seq: u32) -> std::path::PathBuf {
    dir.join(format!("{stem}.{seq:04}.md"))
}

// ---------------------------------------------------------------------------
// AC-006 — staged, crash-recoverable roll-before-write sequence, driven
// end-to-end through the real PreToolUse dispatch path.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_006_AC006_over_cap_write_dispatch_stages_full_roll_sequence() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("decision-log.md");
    let pre_roll_content = "y".repeat(3_000);
    std::fs::write(&target, &pre_roll_content).unwrap();

    // Write's own formula is `projected_size = len(content)` ALONE
    // (Postcondition 3 CORRECTED) — 50,000 > 49,152 triggers regardless of
    // the pre-existing 3,000-byte canonical content.
    let summary = run_roll_gate(
        dir.path(),
        &target,
        "Write",
        serde_json::json!({"content": "x".repeat(50_000)}),
    )
    .await;

    // AC-006 / Invariant 1: the fired trigger MUST resolve to Block (never
    // silent Continue) — a blocking dispatch outcome (non-zero exit_code).
    assert_ne!(
        summary.exit_code, 0,
        "AC-006/Invariant 1: an over-cap Write must resolve to HookResult::Block, a blocking \
         dispatch outcome (exit_code != 0) — never a silent Continue"
    );
    assert!(
        summary.block_intent,
        "AC-006: block_intent must be set for a fired roll trigger"
    );

    // Postcondition 1 step (b): the sealed shard is a byte-for-byte copy of
    // the PRE-ROLL canonical content (never the blocked Write's own,
    // never-applied 50,000-byte payload).
    let sealed_path = sealed_path_for(dir.path(), "decision-log", 1);
    let sealed_content = std::fs::read_to_string(&sealed_path).expect(
        "AC-006: the sealed shard file must exist on disk in THE SAME native-gate invocation \
         that returned Block (Postcondition 1 executes fully before any HookResult returns)",
    );
    assert_eq!(
        sealed_content, pre_roll_content,
        "AC-006: the sealed shard must contain the canonical file's PRE-ROLL content, never the \
         blocked call's own (never-applied) payload"
    );

    // Postcondition 1 step (c) / Invariant 6: canonical is exactly 0 bytes.
    let canonical_len = std::fs::metadata(&target).unwrap().len();
    assert_eq!(
        canonical_len, 0,
        "AC-006/AC-008: the canonical file must be exactly 0 bytes immediately after the roll — \
         the blocked Write's own content is never applied"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_006_AC006_over_cap_edit_dispatch_stages_full_roll_sequence() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("decision-log.md");
    let pre_roll_content = "y".repeat(48_000);
    std::fs::write(&target, &pre_roll_content).unwrap();

    // Edit/MultiEdit's formula is `current_shard_bytes + net_delta_bytes`
    // (UNCHANGED leg) — old_string 10 bytes, new_string 2,000 bytes ->
    // net_delta = +1,990; 48,000 + 1,990 = 49,990 > 49,152 triggers.
    let summary = run_roll_gate(
        dir.path(),
        &target,
        "Edit",
        serde_json::json!({
            "old_string": "x".repeat(10),
            "new_string": "x".repeat(2_000),
        }),
    )
    .await;

    assert_ne!(
        summary.exit_code, 0,
        "AC-006: an over-cap Edit must also resolve to a blocking Block outcome, identically to \
         the Write case"
    );

    let sealed_path = sealed_path_for(dir.path(), "decision-log", 1);
    let sealed_content = std::fs::read_to_string(&sealed_path)
        .expect("AC-006: the sealed shard must exist for the Edit case too");
    assert_eq!(
        sealed_content, pre_roll_content,
        "AC-006: the sealed shard must be the pre-roll canonical content for the Edit trigger \
         path, identically to the Write case"
    );
    assert_eq!(
        std::fs::metadata(&target).unwrap().len(),
        0,
        "AC-006: canonical must be 0 bytes after an Edit-triggered roll too"
    );
}

// ---------------------------------------------------------------------------
// AC-007 — unified, single-template retry-instruction Block message
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_006_AC007_write_block_reason_contains_unified_retry_template() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("decision-log.md");
    std::fs::write(&target, "y".repeat(3_000)).unwrap();

    let summary = run_roll_gate(
        dir.path(),
        &target,
        "Write",
        serde_json::json!({"content": "x".repeat(50_000)}),
    )
    .await;

    assert_ne!(
        summary.exit_code, 0,
        "precondition: this call must block (see AC-006)"
    );

    // Mirrors bc_1_18_005's own F-C1-P2-001 methodology: assert against the
    // Debug-formatted per_plugin_results structure `main.rs::extract_block_info`
    // scans, rather than a specific private field name.
    let per_plugin_debug = format!("{:?}", summary.per_plugin_results);
    for expected_substring in [
        "decision-log",
        "49152",
        "now empty",
        "reissue as a fresh",
        "recompute",
    ] {
        assert!(
            per_plugin_debug.contains(expected_substring),
            "AC-007: the unified Block retry-instruction message must contain \"{expected_substring}\" \
             (Postcondition 2's required substrings — artifact name, cap reached, empty-shard \
             notice, and per-tool retry guidance for BOTH tool branches embedded in ONE template). \
             Got: {per_plugin_debug}"
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_006_AC007_edit_block_reason_uses_same_unified_template_as_write() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("decision-log.md");
    std::fs::write(&target, "y".repeat(48_000)).unwrap();

    let summary = run_roll_gate(
        dir.path(),
        &target,
        "Edit",
        serde_json::json!({
            "old_string": "x".repeat(10),
            "new_string": "x".repeat(2_000),
        }),
    )
    .await;

    assert_ne!(summary.exit_code, 0, "precondition: this call must block");

    let per_plugin_debug = format!("{:?}", summary.per_plugin_results);
    // Invariant 4: the SAME fixed template — never a divergent per-tool-name
    // wording — so the Edit case's message must ALSO name the Write-specific
    // guidance text (both branches are embedded in the ONE template).
    for expected_substring in [
        "decision-log",
        "49152",
        "now empty",
        "reissue as a fresh",
        "recompute",
    ] {
        assert!(
            per_plugin_debug.contains(expected_substring),
            "AC-007/Invariant 4: the Edit-triggered Block message must use the SAME unified \
             template as the Write case — never a per-tool-name-divergent wording. Missing \
             \"{expected_substring}\". Got: {per_plugin_debug}"
        );
    }
}

// ---------------------------------------------------------------------------
// AC-008 — no shard ever observed over cap; same-invocation shard+index
// atomicity.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_006_AC008_sealed_shard_capped_and_index_published_same_invocation() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("decision-log.md");
    std::fs::write(&target, "y".repeat(3_000)).unwrap();

    let summary = run_roll_gate(
        dir.path(),
        &target,
        "Write",
        serde_json::json!({"content": "x".repeat(50_000)}),
    )
    .await;
    assert_ne!(summary.exit_code, 0, "precondition: this call must block");

    // AC-008 first half: the sealed shard's final size (3,000, the pre-roll
    // content) is always <= shard_cap_bytes for a normal (prospective) roll.
    let sealed_path = sealed_path_for(dir.path(), "decision-log", 1);
    let sealed_len = std::fs::metadata(&sealed_path)
        .expect("AC-008: the sealed shard must exist")
        .len();
    assert!(
        sealed_len <= 49_152,
        "AC-008: a normal (non-retroactive) roll's sealed shard must never exceed \
         shard_cap_bytes — got {sealed_len}"
    );

    // AC-008 second half / AC-004 structural guarantee: the shard-index TOML
    // write and the sealed-shard publish are BOTH filesystem writes present
    // in the SAME native-gate invocation, before any git commit — i.e. both
    // must be visible on disk by the time this ONE dispatch call returns.
    let index_path = dir.path().join("decision-log.shard-index.toml");
    assert!(
        index_path.exists(),
        "AC-008: the shard-index TOML must be durably published in the SAME PreToolUse \
         invocation that sealed the shard — TD-VSDD-053 single-commit-per-burst structurally \
         depends on both writes landing together in the working tree before the next git commit"
    );
}

// ---------------------------------------------------------------------------
// AC-009 — shard-index schema and stable-current-filename addressing
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_006_AC009_shard_index_schema_records_seal_event() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("decision-log.md");
    std::fs::write(&target, "y".repeat(3_000)).unwrap();

    let summary = run_roll_gate(
        dir.path(),
        &target,
        "Write",
        serde_json::json!({"content": "x".repeat(50_000)}),
    )
    .await;
    assert_ne!(summary.exit_code, 0, "precondition: this call must block");

    let index_path = dir.path().join("decision-log.shard-index.toml");
    let index_toml =
        std::fs::read_to_string(&index_path).expect("AC-009: the shard-index file must exist");
    let index: factory_dispatcher::shard_manager::ShardIndex = toml::from_str(&index_toml).expect(
        "AC-009: the shard-index must be valid TOML matching \
                                              the ShardIndex schema",
    );

    assert_eq!(
        index.schema_version, 1,
        "Postcondition 5: schema_version must be 1"
    );
    assert_eq!(index.artifact_stem, "decision-log");
    assert_eq!(
        index.shards.len(),
        1,
        "AC-009: exactly one [[shard]] entry after one roll"
    );
    let entry = &index.shards[0];
    assert_eq!(entry.seq, 1, "AC-009: seq starts at 1");
    assert_eq!(
        entry.path, "decision-log.0001.md",
        "AC-009: path must name the sealed file using the <stem>.<seq:04>.md pattern"
    );
    assert_eq!(
        entry.bytes_at_seal, 3_000,
        "AC-009: bytes_at_seal must record the sealed shard's exact final byte count"
    );
    assert!(
        !entry.sealed_retroactively,
        "AC-009/AC-008: a normal (prospective) roll's entry must not set sealed_retroactively"
    );
}

// ---------------------------------------------------------------------------
// AC-024 — Postcondition 7 catch point (i): immediate post-write
// reconciliation, driven through the REAL `invoke.rs` qualifying wrapper
// (`reconcile_replace_all_overcap_if_qualifying`), which is wired
// unconditionally into `main::run` (ADR-051 §Decision 15 point 4).
// ---------------------------------------------------------------------------

fn replace_all_post_tool_use_payload(target: &std::path::Path) -> HookPayload {
    let value = serde_json::json!({
        "hook_event_name": "PostToolUse",
        "tool_name": "Edit",
        "session_id": "sess-bc-1-18-006-ac024",
        "tool_input": {
            "file_path": target.to_string_lossy(),
            "old_string": "some old text",
            "new_string": "some new text",
            "replace_all": true,
        },
        "tool_response": {"success": true},
    });
    serde_json::from_value(value).expect(
        "HookPayload must deserialize from a well-formed \
                                           PostToolUse replace_all Edit envelope",
    )
}

#[test]
fn test_BC_1_18_006_AC024_EC014_real_invoke_wiring_retroactively_seals_qualifying_replace_all_overcap_write()
 {
    let dir = tempfile::tempdir().unwrap();
    write_shard_config(dir.path(), FLAT_SHARD_CONFIG);
    let target = dir.path().join("decision-log.md");
    // Simulates the under-projected replace_all write already having been
    // APPLIED (EC-014): the canonical file is genuinely over cap on disk by
    // the time this PostToolUse event fires.
    let over_cap_content = "z".repeat(49_500);
    std::fs::write(&target, &over_cap_content).unwrap();

    let payload = replace_all_post_tool_use_payload(&target);

    // AC-024/Postcondition 7 catch point (i): this call routes through the
    // REAL `detect_replace_all_overcap_candidate` qualification filter
    // (event=="PostToolUse", tool in {Edit,MultiEdit}, replace_all:true,
    // config-match) into the now-implemented
    // `shard_manager::reconcile_post_write_replace_all_overcap`, which
    // executes `execute_roll`'s full four-step sequence RETROACTIVELY
    // (`sealed_retroactively = true`) against the content already durably on
    // disk. Assert the REAL side effects (never weakened to a mere
    // "didn't panic" check):
    reconcile_replace_all_overcap_if_qualifying(&payload, dir.path());

    // Step (b): the sealed shard is a byte-for-byte copy of the already-
    // applied over-cap content, published as the first sealed shard.
    let sealed_path = sealed_path_for(dir.path(), "decision-log", 1);
    let sealed_content = std::fs::read_to_string(&sealed_path).expect(
        "AC-024: catch point (i) must publish a sealed shard for the already-applied over-cap \
         content",
    );
    assert_eq!(
        sealed_content, over_cap_content,
        "AC-024: the sealed shard must be a byte-for-byte copy of the over-cap content already \
         on disk at the time the PostToolUse event fired"
    );

    // Step (c) / Invariant 6: canonical is exactly 0 bytes.
    assert_eq!(
        std::fs::metadata(&target).unwrap().len(),
        0,
        "AC-024/Invariant 6: the canonical file must be exactly 0 bytes after catch point (i)'s \
         retroactive roll reconciles the over-cap state"
    );

    // Step (d) / Postcondition 5 + Postcondition 7: the shard-index entry
    // records this seal with `sealed_retroactively = true` (Postcondition
    // 7's documented, narrowly-scoped exception to Postcondition 3's
    // `bytes_at_seal <= shard_cap_bytes` bound).
    let index_path = dir.path().join("decision-log.shard-index.toml");
    let index_toml =
        std::fs::read_to_string(&index_path).expect("AC-024: the shard-index file must exist");
    let index: factory_dispatcher::shard_manager::ShardIndex = toml::from_str(&index_toml)
        .expect("AC-024: the shard-index must be valid TOML matching the ShardIndex schema");
    assert_eq!(
        index.shards.len(),
        1,
        "AC-024: exactly one [[shard]] entry after one retroactive roll"
    );
    let entry = &index.shards[0];
    assert_eq!(entry.seq, 1);
    assert_eq!(entry.path, "decision-log.0001.md");
    assert_eq!(
        entry.bytes_at_seal, 49_500,
        "AC-024/Postcondition 7: bytes_at_seal exceeds shard_cap_bytes (49,152) for a \
         retroactive seal — the documented exception to the normal <= cap bound"
    );
    assert!(
        entry.sealed_retroactively,
        "AC-024/Postcondition 7: a catch-point-(i) seal must record sealed_retroactively = true"
    );
}

// EC-016 (no agent-facing signal change for catch point (i)):
// `reconcile_replace_all_overcap_if_qualifying`'s own
// `fn(&HookPayload, &std::path::Path)` return type is `()`, not `HookResult`
// (see the `use` import above) — it is structurally incapable of surfacing a
// Block/Error to the agent, by construction, regardless of what its
// implementation does. The AC-024 test above calls this exact function and
// observes only its filesystem side effects (sealed shard, truncated
// canonical, updated index) — never a `HookResult` of any kind — which is
// the load-bearing evidence for EC-016: the PostToolUse dispatch that
// triggered this reconciliation already returned `Continue` to the agent
// before this leg ever runs, and this leg has no channel through which to
// retroactively surface a Block or Error even if it wanted to.

// ---------------------------------------------------------------------------
// AC-025 — Postcondition 7 catch point (ii): next-dispatch leading-probe
// backstop, driven through the REAL PreToolUse dispatch path (the SAME
// `execute_tiers` -> `shard_cap_gate_check` wiring AC-006 uses), against a
// canonical file already left over cap by a simulated missed catch point (i)
// (EC-015).
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_006_AC025_EC015_leading_probe_backstop_reached_on_next_edit_dispatch() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("decision-log.md");
    // EC-015: catch point (i) is simulated as crashed/never-run — the
    // canonical file is ALREADY over cap on disk when the artifact's NEXT
    // matched dispatch arrives.
    std::fs::write(&target, "z".repeat(50_000)).unwrap();

    // The leading-probe backstop (AC-025) is wired into the `ShardShape::Flat`
    // Edit/MultiEdit arms, reusing THAT arm's own `current_shard_bytes_flat`
    // stat() read — so any Edit dispatch against this already-over-cap
    // artifact reaches it. `old_string`/`new_string` here are net-zero
    // (irrelevant — the backstop fires BEFORE BC-1.18.005's own trigger is
    // even evaluated, per Postcondition 7's "before evaluating the new
    // call's own trigger" ordering).
    // `old_string`/`new_string` are net-zero-length-delta so that, once the
    // backstop has reconciled the pre-existing over-cap state, BC-1.18.005's
    // OWN trigger (evaluated against the now-empty, post-backstop canonical)
    // does not ALSO fire: 0 (post-backstop) + 0 (net delta) = 0 <= cap.
    let summary = run_roll_gate(
        dir.path(),
        &target,
        "Edit",
        serde_json::json!({"old_string": "a", "new_string": "a"}),
    )
    .await;

    // AC-025/EC-015: the backstop must have ALREADY reconciled the
    // pre-existing over-cap state BEFORE BC-1.18.005's own trigger is
    // evaluated — so THIS dispatch's own outcome is Continue (the post-
    // backstop canonical, 0 bytes + a net-zero Edit, is well within cap),
    // never a second Block for the SAME already-resolved over-cap condition.
    assert_eq!(
        summary.exit_code, 0,
        "AC-025: once the leading-probe backstop reconciles the pre-existing over-cap state, \
         THIS dispatch's own Edit (net-zero delta against the now-empty canonical) must Continue \
         — the backstop is a silent janitor for the PRIOR condition, not a second block for it"
    );

    // Postcondition 7's bounded-window guarantee: the backstop's retroactive
    // roll published a sealed shard (over cap, sealed_retroactively=true)
    // and left the canonical file at exactly 0 bytes.
    let sealed_path = sealed_path_for(dir.path(), "decision-log", 1);
    let sealed_len = std::fs::metadata(&sealed_path)
        .expect(
            "AC-025: the backstop must have published a sealed shard for the pre-existing \
                 over-cap content",
        )
        .len();
    assert_eq!(
        sealed_len, 50_000,
        "AC-025: the retroactively-sealed shard must capture the FULL pre-existing over-cap \
         content (50,000 bytes) — the sealed-shard cap exception applies since \
         sealed_retroactively is true"
    );
    assert_eq!(
        std::fs::metadata(&target).unwrap().len(),
        0,
        "AC-025/Invariant 6: the canonical file must be exactly 0 bytes after the backstop \
         reconciles — the over-cap window is closed before this dispatch's own trigger runs"
    );
}

// ---------------------------------------------------------------------------
// BC-1.18.006 Postcondition 1 / ADR-051 §Decision 11 — self-heal recovery
// (E-SHD-006, E-SHD-007) MUST run automatically on the artifact's next
// matched dispatch, BEFORE evaluating any new size trigger. `shard_manager::
// self_heal_resume_from_truncate` and `self_heal_reconcile_missing_index_
// entries` are both fully implemented and unit-tested in isolation
// (`shard_manager.rs`'s own `test_BC_1_18_006_ESHD006_*` / `test_BC_1_18_006_
// ESHD007_*` tests), but NEITHER is invoked from `shard_cap_gate_check`
// (verified by inspection: the `ShardShape::Flat` arm only ever calls
// `reconcile_leading_probe_backstop`, gated on `current_bytes >
// entry.shard_cap_bytes` — an on-disk-over-cap condition, which is NOT what
// either self-heal crash signature requires). Crash recovery is therefore
// currently DORMANT: a crashed roll leaves the artifact stuck until some
// out-of-band remediation runs the self-heal functions directly, which no
// dispatch path does today.
//
// Red Gate: both tests below drive a REAL, ordinary matched dispatch (a
// net-zero-delta Edit, well under cap on its own) through the SAME
// `execute_tiers` -> `shard_cap_gate_check` wiring AC-006/AC-025 use, against
// a canonical file left in one of the two crash-state signatures. Today,
// with no self-heal call site wired in, the dispatch's own trigger
// evaluation is the ONLY thing that runs — the crash state is left
// completely unremediated — so every assertion below that expects the
// self-heal's side effects FAILS. Each test's implementer fix is to wire a
// call to the corresponding `self_heal_*` function into `shard_cap_gate_check`
// (BEFORE the `ShardShape::Flat` trigger-evaluation logic, per Postcondition
// 1 / ADR-051 §Decision 11's ordering requirement), not to change these
// assertions.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_006_PC1_ADR051_D11_ESHD006_self_heal_resumes_truncate_on_next_matched_dispatch()
 {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("decision-log.md");
    let stuck_content = "d".repeat(2_500);

    // E-SHD-006 crash signature: step (b) (publish sealed shard) succeeded,
    // step (c) (truncate canonical) never ran — the canonical file is STILL
    // byte-identical to the already-durable sealed shard. Note this is
    // DELIBERATELY under `shard_cap_bytes` (49,152): the leading-probe
    // backstop (AC-025), which only fires when `current_bytes >
    // entry.shard_cap_bytes`, must NOT be the mechanism that reconciles this
    // — only a genuine self-heal call site can, since a stuck-but-under-cap
    // artifact is exactly the case the leading-probe backstop ignores.
    let sealed_path = sealed_path_for(dir.path(), "decision-log", 1);
    std::fs::write(&sealed_path, &stuck_content).expect("seed sealed shard (step (b) done)");
    std::fs::write(&target, &stuck_content).expect("seed duplicate (un-truncated) canonical");
    // No shard-index file exists yet (step (d) never ran either, since it
    // follows step (c) in the sequence).

    // An ordinary, unrelated, net-zero-delta Edit dispatch against this
    // artifact — nothing about this dispatch's OWN payload should trigger a
    // roll (2,500 + 0 well under the 49,152 cap either before or after a
    // correct self-heal).
    let summary = run_roll_gate(
        dir.path(),
        &target,
        "Edit",
        serde_json::json!({"old_string": "a", "new_string": "a"}),
    )
    .await;

    // Postcondition 1 / ADR-051 §Decision 11: self-heal runs BEFORE the new
    // trigger is evaluated, and this dispatch's own delta is net-zero, so the
    // outcome must be Continue regardless of whether self-heal ran — this
    // assertion alone would pass even under today's dormant wiring; the
    // assertions below are the ones that actually distinguish "self-heal ran"
    // from "self-heal never ran".
    assert_eq!(
        summary.exit_code, 0,
        "precondition: a net-zero-delta Edit against a well-under-cap artifact must Continue"
    );

    // The load-bearing assertion: the self-heal must have completed step (c)
    // as part of THIS dispatch, leaving the canonical file empty — never
    // still holding the stuck duplicate content.
    let canonical_len = std::fs::metadata(&target)
        .expect("canonical file must still exist")
        .len();
    assert_eq!(
        canonical_len, 0,
        "BC-1.18.006 Postcondition 1 / ADR-051 §Decision 11 / E-SHD-006: the self-heal must \
         AUTOMATICALLY resume from step (c) (truncate) on the artifact's next matched dispatch, \
         BEFORE evaluating this dispatch's own trigger — the canonical file must be empty, not \
         still holding the stuck pre-crash content, once this dispatch returns"
    );

    // The already-durable sealed shard must never be rewritten by the
    // self-heal (it resumes from step (c) ALONE).
    let sealed_after =
        std::fs::read_to_string(&sealed_path).expect("sealed shard must remain on disk");
    assert_eq!(
        sealed_after, stuck_content,
        "E-SHD-006: the self-heal must never re-publish the already-correct sealed shard"
    );

    // Step (d): the self-heal must also publish the missing index entry for
    // the seal it just resumed from — a consistent recovered state, not a
    // truncate with no corresponding index record.
    let index_path = dir.path().join("decision-log.shard-index.toml");
    let index_toml = std::fs::read_to_string(&index_path).expect(
        "BC-1.18.006 Postcondition 1 / E-SHD-006: the self-heal must publish the shard-index \
         entry for the resumed seal as part of leaving a CONSISTENT recovered state",
    );
    let index: factory_dispatcher::shard_manager::ShardIndex =
        toml::from_str(&index_toml).expect("the shard-index must be valid TOML");
    assert_eq!(
        index.shards.len(),
        1,
        "exactly one reconciled [[shard]] entry"
    );
    let entry = &index.shards[0];
    assert_eq!(entry.seq, 1);
    assert_eq!(entry.path, "decision-log.0001.md");
    assert_eq!(
        entry.bytes_at_seal, 2_500,
        "bytes_at_seal must reflect the resumed seal's actual sealed content length"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_006_PC1_ADR051_D11_ESHD007_self_heal_reconciles_missing_index_on_next_matched_dispatch()
 {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("decision-log.md");

    // E-SHD-007 crash signature: steps (b) and (c) both succeeded — the
    // sealed shard is durably published AND the canonical file is correctly
    // truncated to empty — but step (d) (publish the shard-index entry)
    // never ran, so the sealed shard is orphaned: present on disk, absent
    // from the index.
    std::fs::write(&target, "").expect("seed already-truncated (post-step-(c)) canonical");
    let sealed_path = sealed_path_for(dir.path(), "decision-log", 1);
    std::fs::write(&sealed_path, "f".repeat(1_200)).expect("seed orphaned sealed shard");
    // No shard-index file exists at all yet (step (d) never ran).

    // An ordinary, unrelated, net-zero-delta Edit dispatch against the
    // (correctly empty) canonical — this dispatch's own trigger must never
    // fire on its own (0 + 0 well under cap).
    let summary = run_roll_gate(
        dir.path(),
        &target,
        "Edit",
        serde_json::json!({"old_string": "a", "new_string": "a"}),
    )
    .await;

    assert_eq!(
        summary.exit_code, 0,
        "precondition: a net-zero-delta Edit against an empty, well-under-cap canonical must \
         Continue"
    );

    // The load-bearing assertion: the self-heal must have discovered the
    // orphaned sealed shard and appended its missing index entry as part of
    // THIS dispatch — a consistent recovered state, never a permanently
    // orphaned shard file.
    let index_path = dir.path().join("decision-log.shard-index.toml");
    let index_toml = std::fs::read_to_string(&index_path).expect(
        "BC-1.18.006 Postcondition 1 / ADR-051 §Decision 11 / E-SHD-007: the self-heal must \
         AUTOMATICALLY reconcile a missing index entry for an orphaned sealed shard on the \
         artifact's next matched dispatch, BEFORE evaluating this dispatch's own trigger — the \
         shard-index file must exist once this dispatch returns",
    );
    let index: factory_dispatcher::shard_manager::ShardIndex =
        toml::from_str(&index_toml).expect("the shard-index must be valid TOML");
    assert_eq!(
        index.shards.len(),
        1,
        "E-SHD-007: exactly one reconciled [[shard]] entry for the previously-orphaned sealed \
         shard"
    );
    let entry = &index.shards[0];
    assert_eq!(entry.seq, 1);
    assert_eq!(entry.path, "decision-log.0001.md");
    assert_eq!(
        entry.bytes_at_seal, 1_200,
        "bytes_at_seal must reflect the orphaned sealed shard's actual on-disk byte count"
    );

    // The orphaned sealed shard content itself must never be touched by the
    // reconciliation (it only appends an index record, never rewrites shard
    // content).
    let sealed_after =
        std::fs::read_to_string(&sealed_path).expect("sealed shard must remain on disk");
    assert_eq!(sealed_after, "f".repeat(1_200));
}
