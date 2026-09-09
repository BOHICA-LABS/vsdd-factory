// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! BC-1.18.006 (S-25.02 cluster-2 "roll") integration coverage for the
//! `executor.rs` -> `shard_manager.rs` roll wiring AND the `invoke.rs`
//! Postcondition 7 catch-point (i) qualifying wrapper -> `main::run` wiring.
//!
//! # BC-5.38.001 Red Gate discipline — GREEN (all tests pass)
//!
//! `shard_manager.rs`'s BC-1.18.006 functions (`execute_roll`,
//! `read_canonical_content`, `publish_sealed_shard`,
//! `truncate_canonical_to_empty`, `publish_shard_index_update`,
//! `reconcile_post_write_replace_all_overcap`,
//! `reconcile_leading_probe_backstop`, and the two self-heal functions) were
//! ALL `todo!()` as of the stub-architect's original cluster-2 burst (commit
//! `e04ab76f`); implementer has since replaced every one of them with real
//! logic, and every test below drives a real call path into that logic and
//! passes. Each test asserts the REAL, post-implementation expected outcome
//! (never `#[should_panic]`) — the same methodology
//! `bc_1_18_005_shard_cap_trigger_test.rs`'s own header comment already
//! establishes for this crate.
//!
//! # BC ambiguity flagged for product-owner — RESOLVED (BC-1.18.006 v1.5, F-C2-P1-004)
//!
//! BC-1.18.006 v1.4's own "Canonical Test Vectors" table's first row ("Write
//! to decision-log.md, current shard 45,000 bytes, content length 5,000
//! bytes, cap 49,152") was internally inconsistent with Postcondition 3's
//! CORRECTED `Write` formula (`projected_size = len(content)` ALONE —
//! `current_shard_bytes` is NEVER added for `Write`) — a stale carry-over
//! from the WITHDRAWN pre-F-P2-002 uniform `current_shard_bytes +
//! payload_bytes` formula. Product-owner corrected this in v1.5
//! (cluster-2 LOCAL adversary pass-1 finding F-C2-P1-004): the row now reads
//! `content` length 50,000 bytes, cap 49,152 — a self-consistent example
//! that actually triggers the roll under the current formula, and matches
//! the worked numbers this file's own tests below already used
//! independently before that fix landed.

use std::sync::Arc;

use factory_dispatcher::engine::build_engine;
use factory_dispatcher::executor::{ExecutorInputs, execute_tiers};
use factory_dispatcher::host::HostContext;
use factory_dispatcher::internal_log::InternalLog;
use factory_dispatcher::invoke::{PluginResult, reconcile_replace_all_overcap_if_qualifying};
use factory_dispatcher::payload::HookPayload;
use factory_dispatcher::plugin_loader::PluginCache;
use factory_dispatcher::registry::Registry;
use factory_dispatcher::resolver::ResolverRegistry;
use factory_dispatcher::shard_manager::{
    ShardEntry, ShardRegistry, ShardShape, execute_roll, find_matching_entry, validate_entry,
};

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

/// Parses the `reason` field out of a `{"outcome":"block","reason":"..."}`
/// stdout JSON payload in `summary.per_plugin_results` and returns the
/// EXACT (un-truncated, un-`Debug`-escaped) string — the same extraction
/// `main.rs`'s own `extract_reason_from_outcome` (TD #71 surfacing path)
/// performs, reimplemented here because that function is private to the
/// `factory-dispatcher` BINARY crate and unreachable from this integration
/// test file (which links only the LIBRARY crate). Existing AC-007 tests in
/// this file assert against `format!("{:?}", summary.per_plugin_results)`
/// substrings, which cannot catch wording drift outside the asserted
/// fragments (F-C2-P6-002) — this helper exists so a verbatim, whole-string
/// comparison is possible instead. Panics if no blocking outcome carries a
/// JSON `reason` field; callers only invoke this after asserting the
/// dispatch actually blocked.
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

/// F-C2-P6-002 (MINOR, cluster-2 LOCAL adversary pass-6) — companion to the
/// two substring-only tests above. `test_BC_1_18_006_AC007_write_block_reason_contains_unified_retry_template`
/// and `..._edit_block_reason_uses_same_unified_template_as_write` only ever
/// assert `.contains(substring)` against `format!("{:?}", per_plugin_results)`
/// — the SAME weak-guard shape the pass-5 fix-burst (F-C2-P5-002) already
/// found and closed for the SIBLING `build_empty_roll_retry_block_reason`
/// template (a prior revision paraphrased that template's wording and a
/// single-substring test stayed green through the divergence). This test
/// pins `build_roll_retry_block_reason`'s UNIFIED template — the one
/// actually used here — to BC-1.18.006 v1.8 Postcondition 2's verbatim
/// blockquote, table-driven over the SAME two tool shapes the substring
/// tests above cover (`Write`, and `Edit` with `replace_all: true` — the
/// prospective PreToolUse trigger formula does not branch on `replace_all`
/// for `Edit`, per `shard_manager.rs`'s `ToolKind::Edit` arm, so this only
/// exercises whether an incidental `replace_all` field on the payload could
/// somehow perturb the emitted template; it does not).
#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_006_FC2P6_002_unified_retry_block_reason_pinned_verbatim() {
    struct Case {
        name: &'static str,
        tool_name: &'static str,
        tool_input: serde_json::Value,
        pre_roll_content: String,
    }

    let cases = vec![
        Case {
            name: "Write",
            tool_name: "Write",
            tool_input: serde_json::json!({"content": "x".repeat(50_000)}),
            pre_roll_content: "y".repeat(3_000),
        },
        Case {
            name: "Edit{replace_all:true}",
            tool_name: "Edit",
            tool_input: serde_json::json!({
                "old_string": "x".repeat(10),
                "new_string": "x".repeat(2_000),
                "replace_all": true,
            }),
            pre_roll_content: "y".repeat(48_000),
        },
    ];

    for case in cases {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("decision-log.md");
        std::fs::write(&target, &case.pre_roll_content).unwrap();

        let summary = run_roll_gate(dir.path(), &target, case.tool_name, case.tool_input).await;
        assert_ne!(
            summary.exit_code, 0,
            "{}: precondition: this call must block (see AC-006)",
            case.name
        );

        let actual_reason = exact_block_reason(&summary);

        // BC-1.18.006 v1.8 Postcondition 2's unified rotate-and-retry
        // template, read VERBATIM from the spec's own blockquote.
        // `<artifact>` and `<sealed-path>` carry LITERAL backticks in the
        // emitted text (the established convention `build_empty_roll_retry_block_reason`'s
        // own doc comment cites THIS template as precedent for); the `<N>`
        // numeric placeholder's backticks are doc-only spec markup — a bare
        // number is emitted. `<artifact>` = "decision-log", `<N>` = 49152
        // (`FLAT_SHARD_CONFIG`'s `shard_cap_bytes`), `<sealed-path>` =
        // "decision-log.0001.md" (each case rolls a FRESH tempdir, so
        // `next_seal_seq` = 1 both times).
        let expected_reason = "Shard `decision-log` rotated (cap 49152 bytes reached); the \
             current shard is now empty. Retry your write against the CURRENT (post-roll, \
             empty) file — do not resubmit your original payload unchanged: if you used \
             `Edit` or `MultiEdit`, your `old_string` will no longer match (the content it \
             targeted is now in `decision-log.0001.md`) — reissue as a fresh `Write` \
             containing ONLY your new entry; if you used `Write`, recompute `content` to \
             contain ONLY your new entry (not your original full pre-roll payload, which \
             reflects discarded state and will exceed the cap again if resubmitted).";

        assert_eq!(
            actual_reason, expected_reason,
            "{}: F-C2-P6-002: build_roll_retry_block_reason's UNIFIED rotate-and-retry \
             template must match BC-1.18.006 v1.8 Postcondition 2's verbatim blockquote \
             byte-for-byte — a sibling template (build_empty_roll_retry_block_reason) was \
             previously found to have diverged from its own spec text (F-C2-P5-002) while a \
             substring-only test stayed green through that divergence; this pins the UNIFIED \
             template's exact text so the same class of drift cannot hide here undetected.",
            case.name
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

/// MAJOR-2 (S-25.02 cluster-2 PR #824 pr-review cycle 3): a `[[shard]]`
/// entry with `artifact_path = "."` normalizes to an EMPTY registered
/// component vector — `path_falls_under_or_equals`'s suffix-match leg is
/// then vacuously true for ANY target sharing `artifact_stem`, so
/// `find_matching_entry` matches this entry against a target path it never
/// legitimately governed. `validate_entry` refuses this shape loud
/// (EC-022), and `shard_cap_gate_check` (the PreToolUse leg) never reaches
/// its own `execute_roll` without calling `validate_entry` first — but
/// PRIOR to MAJOR-2's fix, `detect_replace_all_overcap_candidate` (this
/// leg) called `find_matching_entry` alone and routed straight into the
/// destructive `reconcile_post_write_replace_all_overcap` ->
/// `execute_roll`, which seals and truncates the canonical to 0 bytes.
const EC022_ARTIFACT_PATH_ALL_CURDIR_SHARD_CONFIG: &str = "\
[[shard]]
artifact_stem = \"decision-log\"
artifact_path = \".\"
practical_fuel_ceiling = 8000000
worst_case_fuel_per_byte = 106.36
max_single_record_bytes = 16384
safety_margin = 8192
shard_cap_bytes = 49152
shape = \"flat\"
";

#[test]
fn test_MAJOR2_reconcile_replace_all_overcap_never_reaches_execute_roll_for_entry_failing_validate_entry()
 {
    let dir = tempfile::tempdir().unwrap();
    write_shard_config(dir.path(), EC022_ARTIFACT_PATH_ALL_CURDIR_SHARD_CONFIG);
    let target = dir.path().join("decision-log.md");
    let over_cap_content = "z".repeat(49_500);
    std::fs::write(&target, &over_cap_content).unwrap();

    // Precondition: confirm this fixture actually reproduces MAJOR-2's
    // exact bypass shape — find_matching_entry MUST match this entry (the
    // vacuously-true suffix match an all-CurDir artifact_path produces),
    // while validate_entry MUST independently refuse it (EC-022). If either
    // precondition fails, this test would pass for the wrong reason (no
    // bypass to catch at all).
    let registry_path = dir.path().join(".factory").join("shard-config.toml");
    let registry = ShardRegistry::load(&registry_path)
        .expect("precondition: the fixture shard-config.toml must parse as a valid ShardRegistry");
    let matched = find_matching_entry(&registry, &target)
        .expect("precondition: find_matching_entry must not itself error for this fixture")
        .expect(
            "precondition: find_matching_entry MUST match this EC-022-invalid entry against the \
             target — MAJOR-2's bypass is exercised only when the matcher itself succeeds \
             despite the entry being invalid",
        );
    assert!(
        validate_entry(matched).is_err(),
        "precondition: this entry MUST fail validate_entry (EC-022 empty/CurDir-only \
         artifact_path) — otherwise this fixture does not reproduce MAJOR-2's bypass shape at \
         all"
    );

    let payload = replace_all_post_tool_use_payload(&target);
    reconcile_replace_all_overcap_if_qualifying(&payload, dir.path());

    // MAJOR-2: the destructive execute_roll path must NEVER be reached for
    // an entry that fails validate_entry, even though find_matching_entry
    // matched it. Assert the REAL absence of every destructive side effect
    // execute_roll would have produced (never a mere "didn't panic" check):
    assert_eq!(
        std::fs::read_to_string(&target).expect("MAJOR-2: the canonical file must still exist"),
        over_cap_content,
        "MAJOR-2: an entry failing validate_entry must never reach execute_roll — the canonical \
         must be left COMPLETELY UNTOUCHED, never sealed-and-truncated, for a config entry the \
         PreToolUse gate would have refused"
    );
    let sealed_path = sealed_path_for(dir.path(), "decision-log", 1);
    assert!(
        !sealed_path.exists(),
        "MAJOR-2: no sealed shard must be published for an entry failing validate_entry"
    );
    let index_path = dir.path().join("decision-log.shard-index.toml");
    assert!(
        !index_path.exists(),
        "MAJOR-2: no shard-index must be created for an entry failing validate_entry — \
         reconciliation must never have been attempted at all"
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
// AC-025 / EC-017 — BC-1.18.006 v1.5 (F-C2-P1-002, MAJOR): catch point
// (ii)'s Write-arm DEDICATED backstop `stat()`. Prior to v1.5, the
// `ShardShape::Flat` `Write` arm performed NO stat() of the canonical file
// at all (its Postcondition-3 formula, `projected_size = len(content)`,
// never reads current_shard_bytes) — so a crash-orphaned, un-sealed,
// over-cap canonical file left behind by a missed catch point (i) (EC-015)
// would be silently destroyed by the NEXT Write whose own content happens
// to be under cap, with no roll ever having started for self-heal to catch.
//
// Red Gate: this test seeds exactly that crash-orphaned state (canonical
// file over cap, un-sealed, no roll ever attempted) and drives a `Write`
// dispatch whose own `content` is under cap. It currently FAILS because the
// `Write` arm has no backstop at all: `projected_size_write(3_000) = 3_000
// <= 49_152` returns `Continue` immediately, with no sealed shard ever
// published and the canonical file left untouched at 50,000 bytes — the
// exact data-loss gap F-C2-P1-002 identifies. Once implementer adds the
// Write arm's own dedicated, bounded `stat()` backstop (reusing
// `reconcile_leading_probe_backstop`, mirroring the already-wired Edit/
// MultiEdit arms), this test turns GREEN unmodified.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_006_AC025_EC017_write_arm_backstop_seals_orphaned_overcap_before_applying_undercap_write()
 {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("decision-log.md");
    // EC-017: catch point (i) crashed without running (as EC-015); the
    // canonical file is left over cap, un-sealed, with NO roll ever having
    // started. The artifact's NEXT matched dispatch is specifically a
    // `Write` (not an `Edit`/`MultiEdit`) whose own `content` is under cap
    // on its own.
    let orphaned_content = "z".repeat(50_000);
    std::fs::write(&target, &orphaned_content).unwrap();

    let summary = run_roll_gate(
        dir.path(),
        &target,
        "Write",
        serde_json::json!({"content": "x".repeat(3_000)}),
    )
    .await;

    // F-C2-P1-002 / Postcondition 7 catch point (ii): the Write arm's own
    // DEDICATED stat() must detect the pre-existing over-cap canonical
    // BEFORE the Write is applied, and reconcile it via the retroactive
    // roll — so THIS dispatch's own outcome (evaluated against the
    // post-backstop, now-empty canonical) is Continue: 0 (post-backstop) +
    // 3,000 (this Write's own content) <= 49,152.
    assert_eq!(
        summary.exit_code, 0,
        "AC-025/EC-017: once the Write-arm backstop reconciles the pre-existing over-cap state, \
         THIS dispatch's own under-cap Write must Continue — never a stale Block for an \
         already-closed condition, and never a silent pass-through that skips reconciliation"
    );

    let sealed_path = sealed_path_for(dir.path(), "decision-log", 1);
    let sealed_content = std::fs::read_to_string(&sealed_path).expect(
        "AC-025/EC-017/F-C2-P1-002: the Write arm's dedicated stat()-based backstop must \
         publish a sealed shard for the crash-orphaned, un-sealed, over-cap canonical content \
         BEFORE the Write is ever applied — otherwise this un-sealed history is silently \
         destroyed by the under-cap Write, which is exactly the data-loss gap F-C2-P1-002 \
         identifies",
    );
    assert_eq!(
        sealed_content, orphaned_content,
        "AC-025/EC-017: the sealed shard must be a byte-for-byte copy of the orphaned over-cap \
         content that existed BEFORE this Write dispatch — never the Write's own (not-yet-\
         applied) under-cap content"
    );

    // Invariant 6: the canonical file's zero-bytes-after-roll guarantee
    // holds unconditionally, even under this retroactive backstop path —
    // the Write's own content is applied AFTER, by the caller, against this
    // now-empty canonical, never against the orphaned over-cap content.
    assert_eq!(
        std::fs::metadata(&target).unwrap().len(),
        0,
        "AC-025/EC-017/Invariant 6: the canonical file must be exactly 0 bytes after the \
         Write-arm backstop reconciles the pre-existing over-cap state"
    );

    let index_path = dir.path().join("decision-log.shard-index.toml");
    let index_toml = std::fs::read_to_string(&index_path).expect(
        "AC-025/EC-017: the shard-index TOML must be published for the Write-arm backstop's \
         seal, in the SAME native-gate invocation",
    );
    let index: factory_dispatcher::shard_manager::ShardIndex =
        toml::from_str(&index_toml).expect("the shard-index must be valid TOML");
    assert_eq!(
        index.shards.len(),
        1,
        "AC-025/EC-017: exactly one [[shard]] entry after the Write-arm backstop's retroactive \
         roll"
    );
    let entry = &index.shards[0];
    assert_eq!(entry.seq, 1);
    assert_eq!(entry.path, "decision-log.0001.md");
    assert_eq!(
        entry.bytes_at_seal, 50_000,
        "AC-025/EC-017: bytes_at_seal must record the FULL pre-existing orphaned over-cap \
         content's exact size — the sealed-shard cap exception applies since this seal is \
         retroactive"
    );
    assert!(
        entry.sealed_retroactively,
        "F-C2-P1-002: a seal produced by the Write-arm backstop's retroactive roll must record \
         sealed_retroactively = true, identically to the Edit/MultiEdit-arm backstop path"
    );
}

// ---------------------------------------------------------------------------
// AC-024 / EC-018 — BC-1.18.006 v1.5 Invariant 7 (F-C2-P1-001, MAJOR):
// `sealed_retroactively` recovery-by-inference on a crashed RETROACTIVE
// roll's own `E-SHD-007` self-heal.
//
// A retroactive roll (Postcondition 7 catch point (i) or (ii)) that crashes
// AFTER its own step (c) (canonical truncated) but BEFORE its own step (d)
// (index publish) leaves the exact same filesystem signature as a crashed
// PROSPECTIVE roll — an orphaned sealed shard, present on disk, absent from
// the index — with no in-flight roll context surviving to record that THIS
// particular seal was retroactive. Invariant 7 requires the index
// reconciliation to INFER `sealed_retroactively` from the recovered entry's
// own `bytes_at_seal` vs. `shard_cap_bytes` (`bytes_at_seal >
// shard_cap_bytes ⇒ sealed_retroactively = true`, since a prospective roll
// can never seal over-cap content per Postcondition 3), never a hardcoded
// `false`.
//
// Red Gate: `self_heal_reconcile_missing_index_entries` (shard_manager.rs)
// currently hardcodes every recovered entry's `sealed_retroactively` to
// `false` unconditionally. This test seeds an orphaned sealed shard whose
// content is OVER shard_cap_bytes (the retroactive-roll signature) and
// currently FAILS on the `sealed_retroactively` assertion. Once implementer
// applies Invariant 7's inference rule, this test turns GREEN unmodified.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_006_AC024_EC018_self_heal_infers_sealed_retroactively_true_for_overcap_orphaned_shard()
 {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("decision-log.md");

    // E-SHD-007 crash signature for a RETROACTIVE roll: steps (b) and (c)
    // both succeeded — the sealed shard is durably published, OVER CAP
    // (consistent ONLY with a retroactive seal; Postcondition 3 makes an
    // over-cap PROSPECTIVE seal structurally impossible), and the canonical
    // file is correctly truncated to empty — but step (d) never ran, so the
    // sealed shard is orphaned: present on disk, absent from the index, with
    // no surviving record of which roll produced it.
    std::fs::write(&target, "").expect("seed already-truncated (post-step-(c)) canonical");
    let sealed_path = sealed_path_for(dir.path(), "decision-log", 1);
    let over_cap_sealed_content = "q".repeat(49_500);
    std::fs::write(&sealed_path, &over_cap_sealed_content)
        .expect("seed orphaned OVER-CAP sealed shard (retroactive-roll signature)");
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

    let index_path = dir.path().join("decision-log.shard-index.toml");
    let index_toml = std::fs::read_to_string(&index_path).expect(
        "BC-1.18.006 Postcondition 1 / E-SHD-007: self-heal must reconcile the orphaned sealed \
         shard's missing index entry on this next matched dispatch",
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
        entry.bytes_at_seal, 49_500,
        "bytes_at_seal must reflect the orphaned sealed shard's actual on-disk byte count"
    );

    // BC-1.18.006 v1.5 Invariant 7 / F-C2-P1-001: bytes_at_seal (49,500) >
    // shard_cap_bytes (49,152) is proof-by-construction that this seal was
    // produced by a RETROACTIVE roll — self-heal MUST infer
    // sealed_retroactively = true, never hardcode false regardless of size.
    assert!(
        entry.sealed_retroactively,
        "BC-1.18.006 v1.5 Invariant 7 (F-C2-P1-001, MAJOR): E-SHD-007 self-heal reconciliation \
         of an orphaned sealed shard whose bytes_at_seal (49,500) exceeds shard_cap_bytes \
         (49,152) MUST infer sealed_retroactively = true — a hardcoded `false` mislabels a \
         genuinely over-cap shard as cap-guaranteed, the exact ambiguity Postcondition 5's \
         sealed_retroactively field exists to prevent"
    );

    // The orphaned sealed shard's content itself must never be touched by
    // the reconciliation (it only appends an index record, never rewrites
    // shard content).
    let sealed_after =
        std::fs::read_to_string(&sealed_path).expect("sealed shard must remain on disk");
    assert_eq!(sealed_after, over_cap_sealed_content);
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

/// F-C2-P6-003 (ADVISORY, cluster-2 LOCAL adversary pass-6) — sealed-shard
/// filenames use `{seq:04}` (Rust's MIN-width, not FIXED-width, formatting
/// spec), so `seq >= 10,000` produces a 5-digit filename
/// (`decision-log.10000.md`) exactly like this test's sibling
/// `..._ESHD007_...` test above produces a 4-digit one
/// (`decision-log.0001.md`) for `seq == 1`. But
/// `self_heal_reconcile_missing_index_entries` hard-rejects any candidate
/// whose `seq_str.len() != 4` (`shard_manager.rs`) — a 5-digit orphan is
/// silently skipped forever, identically to (and indistinguishable from,
/// by that guard) an unrelated file that merely happens to share this
/// artifact's prefix. This reproduces that orphan directly (mirroring the
/// sibling ESHD-007 test's own fixture shape) and asserts it IS reconciled
/// — this assertion currently FAILS: the guard skips the 5-digit candidate,
/// so no `[[shard]]` index entry (and, since this is the artifact's ONLY
/// orphan, no shard-index file at all) is ever produced.
#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_006_FC2P6_003_self_heal_reconciles_5_digit_seq_orphan() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("decision-log.md");
    std::fs::write(&target, "").expect("seed already-truncated (post-step-(c)) canonical");

    // Same orphan shape as the E-SHD-007 test above (a durably-published,
    // non-empty sealed shard absent from the index — no shard-index file
    // exists at all yet), but at `seq = 10,000`, whose `{seq:04}` filename
    // is 5 digits wide (`10000`, not `0001`).
    let sealed_path = sealed_path_for(dir.path(), "decision-log", 10_000);
    std::fs::write(&sealed_path, "f".repeat(1_200))
        .expect("seed 5-digit-seq orphaned sealed shard");

    // An ordinary, unrelated, net-zero-delta Edit dispatch against the
    // (correctly empty) canonical — this dispatch's own trigger must never
    // fire on its own (0 + 0 well under cap); it exists only to drive
    // self-heal reconciliation for the pre-seeded orphan above.
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
    // 5-digit-seq orphaned sealed shard and appended its missing index
    // entry as part of THIS dispatch, identically to a 4-digit orphan — a
    // consistent recovered state, never a permanently orphaned shard file
    // merely because its `seq` happened to overflow 4 digits.
    let index_path = dir.path().join("decision-log.shard-index.toml");
    let index_toml = std::fs::read_to_string(&index_path).expect(
        "F-C2-P6-003: self_heal_reconcile_missing_index_entries's `seq_str.len() != 4` guard \
         must not silently skip a 5-digit seq (>= 10,000, produced by the `{seq:04}` MIN-width \
         formatting convention) as though it were an unrelated file — the shard-index file \
         must exist once this dispatch returns",
    );
    let index: factory_dispatcher::shard_manager::ShardIndex =
        toml::from_str(&index_toml).expect("the shard-index must be valid TOML");
    assert_eq!(
        index.shards.len(),
        1,
        "F-C2-P6-003: exactly one reconciled [[shard]] entry for the previously-orphaned \
         5-digit-seq sealed shard"
    );
    let entry = &index.shards[0];
    assert_eq!(entry.seq, 10_000);
    assert_eq!(entry.path, "decision-log.10000.md");
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

// ---------------------------------------------------------------------------
// S-25.02 cluster-2 LOCAL adversary pass-2 — three findings (F-C2-P2-001
// MAJOR, F-C2-P2-002 MAJOR, F-C2-P2-006 ADVISORY). Red Gate tests below pin
// the CORRECT behavior and currently FAIL against the pre-fix code.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// F-C2-P2-001 (MAJOR): `shard_manager::self_heal_resume_from_truncate` uses
// `let Ok(sealed_content) = std::fs::read_to_string(&sealed_path) else {
// return Ok(None) };`. When a sealed shard genuinely EXISTS at the index's
// next-expected seq (the plausibility probe already confirmed this) but its
// bytes cannot be read as a `String` (e.g. the sealed content is not valid
// UTF-8), the `else` branch returns `Ok(None)` — INDISTINGUISHABLE from "no
// sealed shard exists at all". `run_self_heal_if_plausible` then falls
// through to `self_heal_reconcile_missing_index_entries`, which appends a
// "sealed" index entry for the orphaned file WITHOUT EVER truncating the
// canonical — a permanent inconsistent state (canonical still holds the
// duplicate, un-truncated pre-roll content; the index falsely records the
// artifact as sealed), violating Invariant 6.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_006_ESHD006_F_C2_P2_001_self_heal_must_not_silently_mis_recover_on_unreadable_sealed_shard()
 {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("decision-log.md");

    // Genuine E-SHD-006 crash signature: step (b) (publish sealed shard)
    // succeeded and step (c) (truncate canonical) never ran — the canonical
    // file is STILL byte-identical to the already-durable sealed shard —
    // EXCEPT the shared bytes are deliberately NOT valid UTF-8 (a leading
    // 0xFF/0xFE/0xFD sequence), so `std::fs::read_to_string` fails on both
    // files even though the sealed shard genuinely exists on disk at the
    // index's next-expected seq path (the plausibility probe's own `.exists()`
    // check does not care about UTF-8 validity).
    let stuck_bytes: Vec<u8> = vec![0xFF, 0xFE, 0xFD, b'x', b'x', b'x', b'x', b'x'];
    let sealed_path = sealed_path_for(dir.path(), "decision-log", 1);
    std::fs::write(&sealed_path, &stuck_bytes)
        .expect("seed non-UTF-8 sealed shard (step (b) done)");
    std::fs::write(&target, &stuck_bytes)
        .expect("seed byte-identical non-UTF-8 canonical (step (c) never ran)");
    // No shard-index file exists yet (step (d) never ran either).

    // An ordinary, unrelated, net-zero-delta Edit dispatch against this
    // artifact — nothing about this dispatch's OWN payload should trigger a
    // roll on its own.
    let summary = run_roll_gate(
        dir.path(),
        &target,
        "Edit",
        serde_json::json!({"old_string": "a", "new_string": "a"}),
    )
    .await;

    // The self-heal probe's own I/O failures must never surface as an
    // uncaught panic in this dispatch; whatever HookResult comes back, the
    // filesystem state is what this test actually pins.
    let _ = summary;

    let index_path = dir.path().join("decision-log.shard-index.toml");
    let index_claims_sealed = match std::fs::read_to_string(&index_path) {
        Ok(toml_str) => {
            match toml::from_str::<factory_dispatcher::shard_manager::ShardIndex>(&toml_str) {
                Ok(index) => !index.shards.is_empty(),
                Err(_) => false,
            }
        }
        Err(_) => false,
    };
    let canonical_bytes = std::fs::read(&target).expect("canonical file must still exist on disk");
    let canonical_still_untruncated = !canonical_bytes.is_empty();

    // F-C2-P2-001: the self-heal must NEVER land in the state where the
    // index claims the artifact is sealed (an index entry exists) WHILE the
    // canonical file remains un-truncated (still holding the duplicate
    // pre-roll content) — that combination is the exact permanent
    // inconsistency this finding identifies. The correct behavior is EITHER
    // a full, correct resume (canonical truncated to 0 bytes via a
    // bytes-level read, with a matching index entry) OR a loud failure (no
    // index entry at all, canonical left untouched) — never both "index says
    // sealed" and "canonical still has the duplicate content" at once.
    assert!(
        !(index_claims_sealed && canonical_still_untruncated),
        "F-C2-P2-001 (MAJOR): E-SHD-006 self-heal must not silently mis-recover on a read \
         error of an EXISTING sealed shard. `self_heal_resume_from_truncate`'s `let Ok(sealed_\
         content) = std::fs::read_to_string(&sealed_path) else {{ return Ok(None) }}` treats a \
         read failure (e.g. invalid UTF-8) identically to \"no sealed shard exists\", falling \
         through to `self_heal_reconcile_missing_index_entries`, which appends a \"sealed\" \
         index entry for the orphaned file WITHOUT ever truncating the canonical — a permanent \
         inconsistent state (canonical still holds the duplicate un-truncated pre-roll content; \
         index falsely claims the artifact is sealed), violating Invariant 6. Got: index_claims_\
         sealed={index_claims_sealed}, canonical_len={}",
        canonical_bytes.len()
    );
}

// ---------------------------------------------------------------------------
// F-C2-P2-002 (MAJOR): `invoke.rs::detect_replace_all_overcap_candidate`
// filters on event/tool/replace_all/config-match but NOT `entry.shape`.
// BC-1.18.006 Postcondition 7 catch point (i) is scoped to the FLAT
// byte-size roll mechanism ONLY — the `replace_all` occurrence-multiplicity
// under-projection gap it exists to catch is a defect of the byte-size
// formula alone, never the `"frontmatter-changelog-array"` item-count
// mechanism. Without a shape check, a `replace_all` Edit against a matched
// `"frontmatter-changelog-array"`-shaped entry that happens to be byte-
// over-cap incorrectly calls `execute_roll`, byte-truncating the canonical
// to empty — cross-mechanism data corruption (e.g. silently emptying a real
// index-style artifact like BC-INDEX.md, whose rotation is item-count-driven
// and has nothing to do with byte size).
// ---------------------------------------------------------------------------

const FRONTMATTER_SHARD_CONFIG: &str = "\
[[shard]]
artifact_stem = \"BC-INDEX\"
artifact_path = \"BC-INDEX.md\"
practical_fuel_ceiling = 8000000
worst_case_fuel_per_byte = 106.36
max_single_record_bytes = 16384
safety_margin = 8192
shard_cap_bytes = 49152
shape = \"frontmatter-changelog-array\"
n = 500
";

#[test]
fn test_BC_1_18_006_PC7_F_C2_P2_002_catch_point_i_must_be_shape_aware_and_no_op_for_frontmatter_changelog_array()
 {
    let dir = tempfile::tempdir().unwrap();
    write_shard_config(dir.path(), FRONTMATTER_SHARD_CONFIG);
    let target = dir.path().join("BC-INDEX.md");
    // Byte-over-cap on disk (> 49,152 shard_cap_bytes), but this entry's
    // shape is "frontmatter-changelog-array" — an item-count trigger, which
    // this catch point never governs. Catch point (i) must be SHAPE-AWARE
    // and simply no-op for this artifact, never byte-roll it.
    let over_cap_content = "z".repeat(49_500);
    std::fs::write(&target, &over_cap_content).unwrap();

    let payload = replace_all_post_tool_use_payload(&target);

    reconcile_replace_all_overcap_if_qualifying(&payload, dir.path());

    let canonical_after = std::fs::read_to_string(&target)
        .expect("F-C2-P2-002: the canonical file must still exist on disk");
    assert_eq!(
        canonical_after, over_cap_content,
        "F-C2-P2-002 (MAJOR): catch point (i) must be SHAPE-AWARE (flat-only per BC-1.18.006 \
         Postcondition 7) — it must NOT byte-roll a \"frontmatter-changelog-array\"-shaped \
         artifact just because it happens to be byte-over-cap. The canonical must be left \
         completely untouched"
    );

    let sealed_path = sealed_path_for(dir.path(), "BC-INDEX", 1);
    assert!(
        !sealed_path.exists(),
        "F-C2-P2-002: no byte-shard must ever be published for a \
         \"frontmatter-changelog-array\"-shaped artifact via catch point (i) — the byte-size \
         roll mechanism does not apply to this shape at all"
    );

    let index_path = dir.path().join("BC-INDEX.shard-index.toml");
    assert!(
        !index_path.exists(),
        "F-C2-P2-002: no shard-index file should be created either — catch point (i) must \
         fully no-op (zero side effects) for a non-flat-shaped entry"
    );
}

// ---------------------------------------------------------------------------
// F-C2-P2-006 (ADVISORY, being fixed in-scope per CLAUDE.md's production-
// grade default): after the `Write`-arm's own Postcondition 7 catch point
// (ii) backstop truncates a crash-orphaned over-cap canonical to 0 bytes, if
// the SAME dispatch's own `Write` `content` alone ALSO exceeds cap,
// BC-1.18.005's own trigger fires a SECOND time for the SAME dispatch — and
// `execute_roll` unconditionally seals whatever the canonical currently
// holds, which is now 0 bytes (the backstop just truncated it). This
// accumulates a useless, permanent EMPTY (0-byte) sealed shard file plus a
// matching shard-index row, purely as an artifact of the backstop-then-
// trigger double-fire sequence — never a real seal of real content.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_006_PC7_F_C2_P2_006_write_backstop_then_trigger_must_not_seal_empty_shard() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("decision-log.md");

    // EC-017 crash-orphan signature: canonical is already over cap,
    // un-sealed, with no roll ever having started.
    let orphaned_content = "z".repeat(50_000);
    std::fs::write(&target, &orphaned_content).unwrap();

    // This dispatch's own Write `content` is ALSO over cap on its own —
    // once the Write-arm backstop truncates the orphaned content away
    // (leaving the canonical at 0 bytes), BC-1.18.005's own trigger,
    // evaluated against THIS Write's 60,000-byte content, fires a SECOND
    // time in the SAME dispatch.
    let summary = run_roll_gate(
        dir.path(),
        &target,
        "Write",
        serde_json::json!({"content": "x".repeat(60_000)}),
    )
    .await;

    // Precondition: an over-cap Write must still resolve to a blocking
    // outcome — F-C2-P2-006 is about NOT accumulating a useless empty seal,
    // never about suppressing the legitimate block for this Write's own
    // genuinely over-cap content.
    assert_ne!(
        summary.exit_code, 0,
        "precondition: this Write's own 60,000-byte content is over the 49,152 cap and must \
         still resolve to Block"
    );

    let index_path = dir.path().join("decision-log.shard-index.toml");
    let index_toml = std::fs::read_to_string(&index_path)
        .expect("the backstop's own legitimate retroactive seal must have published an index");
    let index: factory_dispatcher::shard_manager::ShardIndex =
        toml::from_str(&index_toml).expect("the shard-index must be valid TOML");

    // The backstop's own retroactive seal of the real 50,000-byte orphaned
    // content is legitimate and expected — this assertion is NOT what
    // F-C2-P2-006 is about.
    assert!(
        index.shards.iter().any(|e| e.bytes_at_seal == 50_000),
        "precondition: the Write-arm backstop must have sealed the real, pre-existing 50,000- \
         byte orphaned content. Got shards: {:?}",
        index.shards
    );

    // F-C2-P2-006: no EMPTY (0-byte) sealed shard must ever be recorded in
    // the index — sealing must be skipped once the canonical content is
    // already 0 bytes (i.e. immediately after the backstop's own truncate).
    assert!(
        !index.shards.iter().any(|e| e.bytes_at_seal == 0),
        "F-C2-P2-006 (ADVISORY): an over-cap Write, whose own trigger fires a second time \
         against a canonical the Write-arm backstop JUST truncated to 0 bytes, must NOT \
         accumulate a useless EMPTY (0-byte) sealed shard + index row before blocking — \
         sealing must be skipped when the canonical content is already 0 bytes. Got shards: \
         {:?}",
        index.shards
    );

    // Cross-check directly against the filesystem too — no sealed shard
    // FILE on disk (referenced by the index or not) may be 0 bytes.
    for shard in &index.shards {
        let shard_path = dir.path().join(&shard.path);
        let len = std::fs::metadata(&shard_path)
            .unwrap_or_else(|_| panic!("indexed shard file {} must exist on disk", shard.path))
            .len();
        assert_ne!(
            len, 0,
            "F-C2-P2-006: sealed shard file {} must never be 0 bytes on disk",
            shard.path
        );
    }
}

// ---------------------------------------------------------------------------
// Red Gate: cluster-2 LOCAL adversary PASS 4 findings (BC-1.18.006 v1.8).
// Every test below is written to FAIL against the CURRENT implementation —
// test-writer per BC-5.38.001; implementer follows to make these green.
// ---------------------------------------------------------------------------

/// A well-formed `"flat"`-shaped [`ShardEntry`], mirroring
/// `shard_manager.rs`'s own private `flat_entry` test fixture (identical
/// calibration constants — cap 49,152, formula ceiling 50,640) so this
/// file's directly-constructed fixtures stay self-consistent with the rest
/// of this cluster's already-validated formula ceiling.
fn flat_entry(stem: &str, shard_cap_bytes: u64) -> ShardEntry {
    ShardEntry {
        artifact_stem: stem.to_string(),
        artifact_path: format!("{stem}.md"),
        practical_fuel_ceiling: 8_000_000,
        worst_case_fuel_per_byte: 106.36,
        max_single_record_bytes: 16_384,
        safety_margin: 8_192,
        shard_cap_bytes,
        shape: Some(ShardShape::Flat),
        n: None,
        low_water_mark: None,
    }
}

// ---------------------------------------------------------------------------
// F-C2-P4-001 (MAJOR, data-loss): catch point (i) must self-heal a
// pre-existing E-SHD-006 orphan BEFORE calling `execute_roll` for a NEW
// over-cap payload — otherwise `execute_roll`'s own `next_seal_seq`
// computation (driven off an index that has not yet recorded the orphan)
// collides with the orphan's own seq, and `publish_sealed_shard`'s
// unconditional `write_atomic` SILENTLY OVERWRITES the durable orphan with
// the new payload's content, permanently destroying the orphaned history
// catch point (i) exists to preserve.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_006_FC2P4_001_catch_point_i_self_heals_orphan_before_execute_roll() {
    let dir = tempfile::tempdir().unwrap();
    write_shard_config(dir.path(), FLAT_SHARD_CONFIG);
    let target = dir.path().join("decision-log.md");

    // E-SHD-006 crash state: the sealed shard is ALREADY durably published
    // (step (b) succeeded) holding content X (<= cap, the legitimate
    // non-retroactive-seal shape), the canonical file STILL holds that same
    // content X (step (c)/truncate never ran), and NO shard-index file
    // exists at all yet (step (d) never ran either).
    let orphan_content = "o".repeat(3_000);
    let sealed_seq1_path = sealed_path_for(dir.path(), "decision-log", 1);
    std::fs::write(&sealed_seq1_path, &orphan_content).unwrap();
    std::fs::write(&target, &orphan_content).unwrap();
    let index_path = dir.path().join("decision-log.shard-index.toml");
    assert!(
        !index_path.exists(),
        "precondition: no shard-index file exists yet (step (d) never ran)"
    );

    // Simulate a DIFFERENT, already-applied `replace_all: true` Edit
    // ballooning the canonical to X' — over cap, and content-changing (X'
    // != X) — by the time this PostToolUse event fires.
    let new_content = "n".repeat(50_000);
    std::fs::write(&target, &new_content).unwrap();

    let payload = replace_all_post_tool_use_payload(&target);

    // Drive this through the REAL catch-point-(i) wiring end-to-end.
    reconcile_replace_all_overcap_if_qualifying(&payload, dir.path());

    // The durable E-SHD-006 orphan at seq=1 must be left byte-for-byte
    // UNCHANGED — never overwritten with X'.
    let sealed_seq1_after = std::fs::read_to_string(&sealed_seq1_path)
        .expect("F-C2-P4-001: the durable orphan sealed shard at seq=1 must still exist on disk");
    assert_eq!(
        sealed_seq1_after, orphan_content,
        "F-C2-P4-001 (MAJOR, data-loss): catch point (i) must self-heal the pre-existing \
         E-SHD-006 orphan BEFORE running execute_roll for the new over-cap payload — the \
         orphan's durable content must never be silently overwritten by execute_roll's own \
         next_seal_seq collision"
    );

    // The shard-index must now record BOTH the self-healed orphan (seq=1)
    // AND the new payload's own retroactive seal (seq=2) — never a single
    // entry that clobbers the orphan's slot.
    let index_toml = std::fs::read_to_string(&index_path)
        .expect("F-C2-P4-001: the shard-index must exist after self-heal + roll");
    let index: factory_dispatcher::shard_manager::ShardIndex =
        toml::from_str(&index_toml).expect("the shard-index must be valid TOML");
    assert_eq!(
        index.shards.len(),
        2,
        "F-C2-P4-001: exactly two [[shard]] entries — the self-healed orphan (seq=1) and the \
         new payload's own retroactive seal (seq=2). Got: {:?}",
        index.shards
    );

    let orphan_entry = &index.shards[0];
    assert_eq!(orphan_entry.seq, 1);
    assert_eq!(orphan_entry.path, "decision-log.0001.md");
    assert_eq!(
        orphan_entry.bytes_at_seal, 3_000,
        "F-C2-P4-001: the self-healed orphan's index entry must record its ACTUAL on-disk byte \
         count (3,000), never the new payload's byte count"
    );

    let new_entry = &index.shards[1];
    assert_eq!(
        new_entry.seq, 2,
        "F-C2-P4-001: the new payload must seal to seq=2 (the orphan having already claimed \
         seq=1 via self-heal), never seq=1 again"
    );
    assert_eq!(new_entry.path, "decision-log.0002.md");
    assert_eq!(new_entry.bytes_at_seal, 50_000);
    assert!(
        new_entry.sealed_retroactively,
        "F-C2-P4-001: the new payload's own seal is catch-point-(i)'s retroactive roll"
    );

    // X' must be sealed to a NEW file at seq=2 — never overwriting seq=1.
    let sealed_seq2_path = sealed_path_for(dir.path(), "decision-log", 2);
    let sealed_seq2_content = std::fs::read_to_string(&sealed_seq2_path).expect(
        "F-C2-P4-001: the new payload's content must be sealed to a NEW file (seq=2), not \
         overwrite decision-log.0001.md",
    );
    assert_eq!(sealed_seq2_content, new_content);

    // Canonical is exactly 0 bytes after the retroactive roll completes.
    assert_eq!(
        std::fs::metadata(&target).unwrap().len(),
        0,
        "F-C2-P4-001/Invariant 6: canonical must be exactly 0 bytes after catch point (i)'s \
         retroactive roll reconciles the new over-cap content"
    );
}

// ---------------------------------------------------------------------------
// F-C2-P4-002 (MINOR, E-SHD-009): `publish_sealed_shard` must be write-once
// — a sealed shard, once durably published at a given seq, is IMMUTABLE.
//
// This is a DIRECT unit-level test of `publish_sealed_shard` itself, NOT a
// full-pipeline dispatch test (mirroring `shard_manager.rs`'s own
// `test_BC_1_18_006_ESHD001_publish_sealed_shard_failure_maps_to_seal_write_failed`
// ELOOP/missing-parent-dir-style direct calls) — deliberately, per the
// F-C2-P4-001 fix landed this same burst (catch point (i) now self-heals a
// pre-existing E-SHD-006 orphan BEFORE calling `execute_roll`), the
// collision this guard defends against is NO LONGER REACHABLE through the
// real dispatch pipeline: `run_self_heal_if_plausible`/catch-point-(i)'s own
// self-heal absorbs any stray sealed-shard file into the index FIRST, so
// `next_seal_seq` always advances past it before `execute_roll` ever runs,
// and the pipeline-level collision this test originally drove through
// `run_roll_gate` can never occur. That is correct-by-design — the
// self-heal-first root-cause fix (F-C2-P4-001) closes the only path that
// used to produce a same-seq collision. `publish_sealed_shard`'s own
// write-once guard (E-SHD-009) is the RESIDUAL defense-in-depth layer for
// every other way a destination could already be occupied (a stray file
// dropped outside this dispatcher's own roll sequence, a corrupted/
// hand-edited index, concurrent-process races, etc.) — it can only be
// exercised by calling the primitive directly.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_006_FC2P4_002_publish_sealed_shard_refuses_to_overwrite_existing_seq() {
    let dir = tempfile::tempdir().expect("tempdir");
    let sealed_path = dir.path().join("decision-log.0001.md");

    // Pre-create a sealed-shard file holding content Y — simulating an
    // already-durable seal occupying this exact path.
    let preexisting_content = b"PRESEED".repeat(200);
    std::fs::write(&sealed_path, &preexisting_content).expect("seed pre-existing sealed shard");

    // Attempt to publish a DIFFERENT payload (content Z) at the SAME
    // destination path.
    let new_content = b"z".repeat(3_000);
    let err = factory_dispatcher::shard_manager::publish_sealed_shard(&sealed_path, &new_content)
        .expect_err(
            "F-C2-P4-002 (MINOR): publish_sealed_shard must refuse to overwrite an \
             already-sealed shard — a write-once immutability violation must fail loud, never \
             silently succeed",
        );
    assert!(
        matches!(
            err,
            factory_dispatcher::shard_manager::ShardRollError::SealedShardAlreadyExists { .. }
        ),
        "F-C2-P4-002: the failure MUST be ShardRollError::SealedShardAlreadyExists (E-SHD-009) \
         specifically, not a generic error or a different named variant — got: {err:?}"
    );
    assert!(
        err.to_string().contains("E-SHD-009"),
        "F-C2-P4-002: the error's Display text must name the E-SHD-009 code — got: {err}"
    );

    // The pre-existing sealed shard must be left byte-for-byte UNCHANGED —
    // Z (the new content) must never have been written.
    let on_disk =
        std::fs::read(&sealed_path).expect("F-C2-P4-002: the sealed shard must remain on disk");
    assert_eq!(
        on_disk, preexisting_content,
        "F-C2-P4-002: the pre-existing sealed shard must be left byte-for-byte UNCHANGED (still \
         holding Y) — publish_sealed_shard must never silently overwrite an already-sealed \
         shard file with the new content Z"
    );
}

// ---------------------------------------------------------------------------
// F-C2-P4-003 (MINOR, BC-1.18.006 v1.8): the F-C2-P2-006 empty-canonical
// Block message must match Postcondition 2's "Empty-canonical retry
// template" VERBATIM — not merely contain the `(<N> bytes)` fragment.
//
// STRENGTHENED (cluster-2 LOCAL adversary PASS 5, F-C2-P5-002, weak-test
// finding): the original version of this test asserted only
// `.contains("(60000 bytes)")` — a single substring so weak it stayed GREEN
// through a real emitter-vs-spec divergence: the shipped
// `build_empty_roll_retry_block_reason` never emits the spec's own "no roll
// was performed... rotate away... the shard remains exactly as it was
// before this call" framing at all, and additionally appends a spurious
// "reached" after the cap-bytes parenthetical that Postcondition 2's own
// text does not contain. A single-substring assertion is structurally blind
// to both defects. This version pins the FULL literal template text (BC
// v1.8 Postcondition 2's "Empty-canonical retry template" blockquote,
// cross-checked against EC-021's abbreviated Canonical Test Vector row,
// which — like the sibling unified-template CTV row immediately above it —
// elides the literal backticks/period-vs-ellipsis punctuation for table
// brevity only; Postcondition 2's own quoted blockquote is the byte-exact
// source, and the ALREADY-SHIPPED, ALREADY-GREEN `build_roll_retry_block_
// reason` unified template establishes the precedent that backticks around
// the artifact-name placeholder ARE literal output characters while
// backticks around bare numeric placeholders are doc-only markup — this
// test follows that same convention for the empty-canonical template's
// `<artifact>` placeholder).
// ---------------------------------------------------------------------------

/// BC-1.18.006 v1.8 Postcondition 2 "Empty-canonical retry template",
/// pinned VERBATIM (byte-exact modulo the three named substitutions). Any
/// wording drift in the shipped `build_empty_roll_retry_block_reason` —
/// missing phrases, reordered clauses, or spurious additions — must fail
/// this exact-text assertion, not merely a fragment-substring check.
fn expected_empty_canonical_block_reason(
    artifact_stem: &str,
    payload_len_bytes: u64,
    shard_cap_bytes: u64,
) -> String {
    format!(
        "Shard `{artifact_stem}` is already empty; your own payload alone ({payload_len_bytes} \
         bytes) exceeds the cap ({shard_cap_bytes} bytes). Recompute or split your payload into \
         multiple smaller calls — no roll was performed, because there is no existing content \
         to rotate away; the shard remains exactly as it was before this call."
    )
}

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_006_FC2P4_003_empty_canonical_block_message_matches_verbatim_template() {
    struct Case {
        tool_name: &'static str,
        tool_input: serde_json::Value,
        // The trigger-fire payload length N BC-1.18.005's own formula
        // computes for this call (Write: `len(content)`; Edit{replace_all}:
        // `current_shard_bytes(0) + net_delta_bytes(len(new) - len(old))`) —
        // both cases are constructed so N works out to 60,000, matching the
        // BC's own EC-021 Canonical Test Vector numbers exactly.
        expected_payload_len_bytes: u64,
    }
    let cases = [
        Case {
            tool_name: "Write",
            tool_input: serde_json::json!({"content": "x".repeat(60_000)}),
            expected_payload_len_bytes: 60_000,
        },
        Case {
            tool_name: "Edit",
            tool_input: serde_json::json!({
                "old_string": "",
                "new_string": "z".repeat(60_000),
                "replace_all": true,
            }),
            expected_payload_len_bytes: 60_000,
        },
    ];

    for case in cases {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("decision-log.md");
        // Canonical is ALREADY empty (F-C2-P2-006's own empty-canonical
        // trigger case) — the trigger fires purely because of the incoming
        // payload's own size (60,000 bytes > 49,152 cap).
        std::fs::write(&target, "").unwrap();

        let summary = run_roll_gate(dir.path(), &target, case.tool_name, case.tool_input).await;

        assert_ne!(
            summary.exit_code, 0,
            "precondition ({}): an over-cap payload against an already-empty canonical must \
             still resolve to Block (F-C2-P2-006)",
            case.tool_name
        );

        let per_plugin_debug = format!("{:?}", summary.per_plugin_results);
        let expected = expected_empty_canonical_block_reason(
            "decision-log",
            case.expected_payload_len_bytes,
            49_152,
        );
        assert!(
            per_plugin_debug.contains(&expected),
            "F-C2-P5-002 (strengthened from F-C2-P4-003, BC-1.18.006 v1.8 Postcondition 2's \
             \"Empty-canonical retry template\") tool=\"{}\": the empty-canonical Block message \
             must match the spec's VERBATIM template text exactly, not merely contain the \
             \"({} bytes)\" fragment. Expected the message to contain:\n  {expected}\nGot \
             per_plugin_results debug:\n  {per_plugin_debug}",
            case.tool_name,
            case.expected_payload_len_bytes,
        );
    }
}

// ---------------------------------------------------------------------------
// F-C2-P7-001 (MAJOR, cluster-2 LOCAL adversary pass-7): a first-ever
// over-cap Write against a MISSING canonical (BC-1.18.006 Precondition 2 /
// BC-1.18.005 EC-004 — "treated as a zero-byte current shard") must resolve
// to the SAME empty-canonical Block outcome Postcondition 1's Ok(None)
// short-circuit already produces for an EXISTING 0-byte canonical — NEVER
// `ShardRollError::SealWriteFailed` (E-SHD-001)'s `HookResult::Error`. The
// shipped `execute_roll` step (a) currently maps ANY `read_canonical_content`
// I/O error — including `NotFound` — uniformly to E-SHD-001, misreading
// Precondition 2's explicit "or is treated as a zero-byte current shard...
// if this is the artifact's first-ever write" clause as applying only to
// BC-1.18.005's own trigger formula, not to BC-1.18.006's own roll-execution
// read.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_006_FC2P7_001_missing_canonical_first_ever_write_blocks_via_empty_template() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("decision-log.md");
    // Deliberately NEVER created — this is the artifact's first-ever write
    // (BC-1.18.005 EC-004), the exact scenario BC-1.18.006 Precondition 2
    // names: "the current shard file... exists (or is treated as a
    // zero-byte current shard... if this is the artifact's first-ever
    // write)".
    assert!(
        !target.exists(),
        "test setup: the canonical file must NOT exist before this dispatch"
    );

    let summary = run_roll_gate(
        dir.path(),
        &target,
        "Write",
        serde_json::json!({"content": "x".repeat(60_000)}),
    )
    .await;

    assert_ne!(
        summary.exit_code, 0,
        "Invariant 1: a fired over-cap trigger against a missing (first-ever-write) canonical \
         must resolve to HookResult::Block — never a silent Continue"
    );

    let reason = exact_block_reason(&summary);
    let expected = expected_empty_canonical_block_reason("decision-log", 60_000, 49_152);
    assert_eq!(
        reason, expected,
        "F-C2-P7-001 (MAJOR): a first-ever over-cap Write against a MISSING canonical must \
         resolve to Postcondition 1's empty-canonical Ok(None) short-circuit — the SAME \
         `build_empty_roll_retry_block_reason` template an EXISTING 0-byte canonical produces \
         (BC-1.18.006 Precondition 2's explicit 'treated as a zero-byte current shard' clause) \
         — NOT ShardRollError::SealWriteFailed's (E-SHD-001) generic error message. Got: \
         {reason:?}"
    );

    assert!(
        !target.exists(),
        "F-C2-P7-001: the empty-canonical short-circuit performs ZERO writes — no canonical \
         file may be fabricated as a side effect of a missing-canonical roll attempt"
    );
    let sealed_path = sealed_path_for(dir.path(), "decision-log", 1);
    assert!(
        !sealed_path.exists(),
        "F-C2-P7-001: no sealed shard may be published for the missing-canonical short-circuit \
         — there is no pre-existing content to preserve"
    );
}

// ---------------------------------------------------------------------------
// F-C2-P4-004 (ADVISORY): `execute_roll` must seal byte-for-byte — including
// non-UTF-8 canonical content. Step (a) currently reads the canonical via
// `std::fs::read_to_string`, which fails with `InvalidData` (mapped to
// `ShardRollError::SealWriteFailed`/E-SHD-001) on ANY non-UTF-8 byte
// sequence, even though nothing about sealing requires UTF-8 validity — a
// roll must succeed and preserve arbitrary bytes exactly.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_006_FC2P4_004_execute_roll_seals_non_utf8_content_byte_for_byte() {
    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    let entry = flat_entry("decision-log", 49_152);

    // Over-cap content containing a non-UTF-8 byte (0xFF is never a valid
    // UTF-8 lead or continuation byte).
    let mut raw_bytes = vec![b'a'; 50_000];
    raw_bytes[12_345] = 0xFF;
    std::fs::write(&canonical_path, &raw_bytes).unwrap();

    let result = execute_roll(&entry, &canonical_path, false);

    let new_entry = match result {
        Ok(Some(entry)) => entry,
        other => panic!(
            "F-C2-P4-004 (ADVISORY): execute_roll must succeed and seal non-UTF-8 canonical \
             content byte-for-byte — it must NOT fail with E-SHD-001 (SealWriteFailed) merely \
             because the content is not valid UTF-8. Got: {other:?}"
        ),
    };

    let sealed_path = sealed_path_for(dir.path(), "decision-log", new_entry.seq);
    let sealed_bytes = std::fs::read(&sealed_path)
        .expect("F-C2-P4-004: the sealed shard must exist on disk after a successful roll");
    assert_eq!(
        sealed_bytes, raw_bytes,
        "F-C2-P4-004: the sealed shard must be a byte-for-byte copy of the non-UTF-8 canonical \
         content — no lossy/failable UTF-8 decoding step may sit between the canonical file and \
         the sealed shard"
    );

    assert_eq!(
        std::fs::metadata(&canonical_path).unwrap().len(),
        0,
        "F-C2-P4-004/Invariant 6: canonical must be exactly 0 bytes after a successful roll, \
         even for non-UTF-8 content"
    );
}

// ---------------------------------------------------------------------------
// Red Gate: cluster-2 LOCAL adversary PASS 8 finding F-C2-P8-004 (MINOR,
// BC-1.18.006 v1.9 Postcondition 2's "Double-fire exception" / Invariant 4's
// Case B1/B2 split, EC-026).
//
// "Double-fire" sequence: Postcondition 7 catch point (ii)'s leading
// backstop probe retroactively seals+truncates a PRE-EXISTING orphaned
// over-cap canonical BEFORE this dispatch's own trigger is evaluated (the
// SAME fixture shape `test_BC_1_18_006_PC7_F_C2_P2_006_write_backstop_then_
// trigger_must_not_seal_empty_shard` above already drives, which asserts
// only the index/no-empty-seal side of this sequence); this SAME dispatch's
// own payload then ALSO exceeds the cap against the now-freshly-emptied
// canonical, so Postcondition 1's `Ok(None)` empty-canonical short-circuit
// fires a SECOND time within the SAME dispatch. Per BC v1.9, the gate MUST
// select the distinct Case B2 template (`build_empty_roll_retry_block_reason`
// parameterized with `preceded_by_backstop_roll: true`), which OMITS the "no
// roll was performed"/"remains exactly as it was before this call" clauses
// (FALSE on this path — a roll DID occur) while keeping the identical
// split-payload guidance — never Case B1's wording, never the unified Case A
// template.
//
// RED GATE: `build_empty_roll_retry_block_reason` currently takes no
// `preceded_by_backstop_roll` parameter at all (`shard_manager.rs`, private
// fn, 3 params: artifact_stem, shard_cap_bytes, payload_len_bytes) and
// `shard_cap_gate_check`'s `Ok(None)` arm unconditionally emits Case B1's
// wording regardless of whether catch point (ii) fired earlier in the same
// dispatch — so this test currently FAILS at RUNTIME (wrong Block reason
// text), never at compile time: this integration test only asserts against
// the dispatch's OBSERVABLE `reason` string via `exact_block_reason`, never
// calls the private `build_empty_roll_retry_block_reason` fn directly (which
// is unreachable from this file regardless, being private to
// `shard_manager.rs`). **Production signature the implementer must add**
// (per BC-1.18.006 v1.9 Postcondition 2's own "CODE CHANGE ROUTED"
// text): `build_empty_roll_retry_block_reason` gains a fourth parameter,
// `preceded_by_backstop_roll: bool`, and `shard_cap_gate_check`'s `Ok(None)`
// call site must thread through whether `reconcile_leading_probe_backstop`
// fired earlier in the SAME dispatch (a fact already available in
// dispatch-local control flow — no new tracking state required).
// ---------------------------------------------------------------------------

/// BC-1.18.006 v1.9 Postcondition 2's Case B2 ("Double-fire exception")
/// template, pinned VERBATIM (byte-exact modulo the three named
/// substitutions) — mirrors this file's own
/// `expected_empty_canonical_block_reason` (Case B1) helper above, which
/// this template deliberately DROPS the "no roll was performed... the shard
/// remains exactly as it was before this call" clauses from, while keeping
/// the identical split-payload guidance.
fn expected_case_b2_double_fire_block_reason(
    artifact_stem: &str,
    payload_len_bytes: u64,
    shard_cap_bytes: u64,
) -> String {
    format!(
        "Shard `{artifact_stem}` is now empty (a prior over-cap shard was retroactively rotated \
         by this same call before your payload was evaluated); your own payload alone \
         ({payload_len_bytes} bytes) exceeds the cap ({shard_cap_bytes} bytes). Recompute or \
         split your payload into multiple smaller calls."
    )
}

#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_006_EC026_FC2P8_004_double_fire_emits_case_b2_not_case_b1() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("decision-log.md");

    // EC-017/EC-015 crash-orphan signature: canonical is already over cap,
    // un-sealed, with no roll ever having started — catch point (i) never
    // ran (or crashed before running) for this pre-existing content.
    let orphaned_content = "z".repeat(60_000);
    std::fs::write(&target, &orphaned_content).unwrap();

    // This dispatch's own Write `content` is ALSO over cap on its own
    // (55,000 > 49,152, matching BC-1.18.006 v1.9's own EC-026 Canonical
    // Test Vector numbers exactly) — once the Write-arm backstop (catch
    // point (ii)) retroactively seals+truncates the pre-existing orphaned
    // content (leaving the canonical at 0 bytes), BC-1.18.005's own trigger,
    // evaluated against THIS Write's 55,000-byte content, fires a SECOND
    // time in the SAME dispatch, hitting Postcondition 1's Ok(None)
    // empty-canonical short-circuit a second time.
    let summary = run_roll_gate(
        dir.path(),
        &target,
        "Write",
        serde_json::json!({"content": "x".repeat(55_000)}),
    )
    .await;

    assert_ne!(
        summary.exit_code, 0,
        "precondition: this Write's own 55,000-byte content is over the 49,152 cap and must \
         still resolve to Block"
    );

    // Precondition (already covered by the sibling F-C2-P2-006 test above,
    // re-asserted here so this test is self-contained): the backstop's own
    // legitimate retroactive seal of the real 60,000-byte orphaned content
    // must have landed at seq=1 before this dispatch's own trigger re-fired.
    let index_path = dir.path().join("decision-log.shard-index.toml");
    let index_toml = std::fs::read_to_string(&index_path)
        .expect("the backstop's own legitimate retroactive seal must have published an index");
    let index: factory_dispatcher::shard_manager::ShardIndex =
        toml::from_str(&index_toml).expect("the shard-index must be valid TOML");
    assert!(
        index
            .shards
            .iter()
            .any(|e| e.bytes_at_seal == 60_000 && e.sealed_retroactively),
        "precondition: the Write-arm backstop must have retroactively sealed the real, \
         pre-existing 60,000-byte orphaned content BEFORE this dispatch's own trigger re-fired. \
         Got shards: {:?}",
        index.shards
    );
    assert_eq!(
        index.shards.len(),
        1,
        "precondition: the SECOND (double-fire) Ok(None) short-circuit must NOT have appended \
         its own [[shard]] entry — only the backstop's one legitimate retroactive seal. Got: \
         {:?}",
        index.shards
    );

    let reason = exact_block_reason(&summary);

    let case_b1_forbidden_fragment = "no roll was performed";
    assert!(
        !reason.contains(case_b1_forbidden_fragment),
        "BC-1.18.006 v1.9 Invariant 4 Case B1/B2 split (EC-026, F-C2-P8-004, MINOR): a \
         double-fire dispatch (catch point (ii)'s backstop retroactively rolled a pre-existing \
         orphan earlier in THIS SAME dispatch) must NEVER emit Case B1's \"no roll was \
         performed... the shard remains exactly as it was before this call\" wording — that \
         claim is FALSE on this path (a roll DID occur). Got reason:\n  {reason}"
    );
    assert!(
        !reason.contains("the shard remains exactly as it was before this call"),
        "BC-1.18.006 v1.9 Invariant 4 Case B1/B2 split (EC-026, F-C2-P8-004, MINOR): the \
         double-fire path must not claim the shard \"remains exactly as it was before this \
         call\" — a roll (the backstop's own retroactive roll) DID occur earlier in this same \
         dispatch. Got reason:\n  {reason}"
    );

    let expected = expected_case_b2_double_fire_block_reason("decision-log", 55_000, 49_152);
    assert_eq!(
        reason, expected,
        "BC-1.18.006 v1.9 Postcondition 2's \"Double-fire exception\" (EC-026, F-C2-P8-004, \
         MINOR): a double-fire dispatch must emit Case B2's VERBATIM template — \
         build_empty_roll_retry_block_reason parameterized with preceded_by_backstop_roll: true \
         — dropping the \"no roll was performed\"/\"remains exactly as it was before this call\" \
         clauses while keeping the identical split-payload guidance. Got reason:\n  {reason}\n\
         Expected:\n  {expected}"
    );
}

/// Companion regression guard (F-C2-P8-004): the PURE empty-canonical Case
/// B1 path — no roll of any kind occurred during this dispatch, the
/// canonical was ALREADY 0 bytes when the dispatch began — must remain
/// UNCHANGED by the Case B2 addition above. This duplicates
/// `test_BC_1_18_006_FC2P4_003_empty_canonical_block_message_matches_verbatim_template`'s
/// own assertion deliberately (same verbatim template, same helper), as the
/// pass-8-scoped regression pin: the fix for EC-026 must thread
/// `preceded_by_backstop_roll: false` for this case, never flip it to `true`
/// merely because SOME dispatch-local backstop code path exists in the
/// binary.
#[tokio::test(flavor = "current_thread")]
async fn test_BC_1_18_006_FC2P8_004_pure_empty_canonical_case_b1_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("decision-log.md");
    // Canonical is ALREADY empty BEFORE this dispatch begins — no backstop,
    // no retroactive roll of any kind occurs during this dispatch.
    std::fs::write(&target, "").unwrap();

    let summary = run_roll_gate(
        dir.path(),
        &target,
        "Write",
        serde_json::json!({"content": "x".repeat(60_000)}),
    )
    .await;

    assert_ne!(
        summary.exit_code, 0,
        "precondition: over-cap payload must still Block"
    );

    let reason = exact_block_reason(&summary);
    let expected = expected_empty_canonical_block_reason("decision-log", 60_000, 49_152);
    assert_eq!(
        reason, expected,
        "F-C2-P8-004 regression guard: the PURE empty-canonical Case B1 path (no roll occurred \
         this dispatch) must be UNCHANGED by the Case B2 addition — still Case B1's \"no roll \
         was performed... the shard remains exactly as it was before this call\" wording. Got \
         reason:\n  {reason}\nExpected:\n  {expected}"
    );
}
