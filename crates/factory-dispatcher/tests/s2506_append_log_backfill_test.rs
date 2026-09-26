// Test files use .expect()/.unwrap()/.panic!() for failure reporting,
// matching bc_1_18_011_b2_migration_test.rs's own established convention.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-25.06 / BC-1.18.013 RED-Gate coverage for the `backfill-append-logs`
//! governed one-time migration (mechanism-A's backfill-split, activated via
//! ADR-052's sanctioned execution path).
//!
//! # BC-5.38.001 Red Gate discipline — RED
//!
//! As of stub-architect's S-25.06 commit, every non-trivial function this
//! file exercises panics via `todo!()`:
//! `append_log_markers::artifact_stem_for_target_file`,
//! `append_log_markers::record_boundary_offsets_for_target_file`,
//! `executor::append_log_backfill_admission_precheck`,
//! `executor::append_log_backfill_reservation_release`, and
//! `shard_manager::run_backfill_append_logs_cli`. Each test below asserts
//! the REAL post-implementation expected outcome (mirroring
//! `bc_1_18_011_b2_migration_test.rs`'s own methodology), so the same
//! assertion shape is correct both during Red Gate (fails against the stub)
//! and once BC-1.18.013 is implemented. A handful of tests exercise
//! ALREADY-REAL code (`canonical_path_for_target_file`'s pure path
//! arithmetic; the `AppendLogMigrationError`/schema types' `Display`/serde
//! impls) and are explicitly marked as GREEN-today controls, mirroring
//! `bc_1_18_011_b2_migration_test.rs`'s own "negative control" convention.
//!
//! # Coverage map (AC-001..AC-014, BC-1.18.013 EC-001..EC-008)
//!
//! - AC-001 / EC-008 / VP-145 — closed-grammar rejection
//! - AC-002 / AC-003 / VP-144 — per-file content-preservation + census,
//!   delegated to BC-1.18.008 (differential coverage at the
//!   `append_log_markers` layer; content-preservation coverage at the CLI
//!   layer via byte-exact reconstruction)
//! - AC-004 / AC-006 / EC-001 — four-file all-or-nothing atomicity /
//!   whole-migration rollback
//! - AC-005 — pre-commit source-fingerprint TOCTOU recheck
//! - AC-007 / EC-004 — two-layer idempotency
//! - AC-008 — fixed four-file scope, never a wildcard
//! - AC-009 — `completed.json` as the BC-7.08.001 gating signal
//! - AC-010 — ShardRegistry enrollment is out of the migration binary's own
//!   write-target allowlist
//! - AC-011 / EC-002 / EC-003 — crash / forward-recovery
//! - AC-012 / EC-005 — missing/unreadable target file abort
//! - AC-013 / EC-006 — oversized single-record passthrough
//! - AC-014 / EC-007 — empty-boundary-oracle hard failure
//!
//! # Genuine ambiguities flagged (not guessed at silently)
//!
//! 1. **No granular resume/recovery functions are exposed.** Unlike
//!    BC-1.18.011's sibling (`stage_new_generation`,
//!    `commit_current_generation_pointer`, `execute_canonical_path_moves`,
//!    `resume_from_staging`, `write_txn_record`, `append_intent_log_record`,
//!    `read_active_txn_record`, ...), this story's stub exposes ONLY the
//!    monolithic `run_backfill_append_logs_cli` entry point plus the data
//!    types. `AppendLogIntentLogRecord` does not derive
//!    `Serialize`/`Deserialize` (framed via an as-yet-unimplemented
//!    checksummed writer), so this file cannot pre-seed a precise
//!    mid-lifecycle intent-log fixture the way the B2 test file does.
//!    AC-005's fingerprint-mismatch coverage below therefore pre-seeds only
//!    the (serde-round-trippable) `AppendLogMigrationTxnRecord` at the BC-
//!    documented path (`.factory/migration-state/txn-<activation_id>.json`)
//!    and ASSUMES the implementation discovers an existing STAGING txn
//!    record there at startup and resumes toward the fingerprint recheck.
//!    If the real discovery mechanism differs, this test's setup will need
//!    revision — flagged here rather than guessed at silently.
//! 2. **Shared `migration_fs::Fs` failpoint names.** The crash-injection
//!    test for AC-011/EC-002 assumes the governed multi-file envelope reuses
//!    B2's own `shard_manager::migration_fs::{Fs, StdFs}` seam (per this
//!    story's own `shard_manager.rs`/`executor.rs` module doc comments,
//!    which explicitly call for reusing "the SAME machinery" B2's migration
//!    uses) — meaning the `migration_fs::rename` failpoint name fires
//!    identically regardless of which migration's code path reaches it. If
//!    the implementer instead introduces a separate Fs seam, the
//!    failpoint will never fire and the crash-injection child will exit via
//!    a sentinel "did not abort" code rather than hanging — see that test's
//!    own doc comment.

use std::path::{Path, PathBuf};

use factory_dispatcher::append_log_markers::{
    APPEND_LOG_CYCLE_DIR, APPEND_LOG_TARGET_FILES_IN_ORDER, AppendLogMarkerError,
    artifact_stem_for_target_file, canonical_path_for_target_file,
    record_boundary_offsets_for_target_file,
};
use factory_dispatcher::executor::{
    append_log_backfill_admission_precheck, append_log_backfill_reservation_release,
};
use factory_dispatcher::payload::HookPayload;
use factory_dispatcher::shard_manager::{
    AppendLogAdmissionGateState, AppendLogCompletedMigrationRecord,
    AppendLogCurrentGenerationPointer, AppendLogMigrationError, AppendLogMigrationTxnRecord,
    AppendLogMigrationTxnState, AppendLogPendingCanonicalMove,
    BackfillAppendLogsActivationManifest, ShardEntry, ShardIndex, ShardShape,
    mechanism_a_backfill_already_migrated, mechanism_a_record_boundary_offsets,
    run_backfill_append_logs_cli, run_mechanism_a_backfill_split,
};
use vsdd_hook_sdk::HookResult;

// ---------------------------------------------------------------------------
// Fixture content — each fixture starts EXACTLY at its first record marker
// (zero preamble bytes) so preamble-handling nuance never confounds these
// tests' byte-exact reconstruction assertions.
// ---------------------------------------------------------------------------

const DECISION_LOG_CONTENT: &str = "\
| D-1 | Decision one | rationale one |
| D-2 | Decision two | rationale two |
| D-3 | Decision three | rationale three |
";

const BURST_LOG_CONTENT: &str = "\
## Burst 1

burst one body

## Burst 2

burst two body

## Burst 3

burst three body
";

const LESSONS_CONTENT: &str = "\
## L-TAG-001

lesson one body

## L-TAG-002

lesson two body
";

const SESSION_CHECKPOINTS_CONTENT: &str = "\
## Checkpoint 1

checkpoint one body

## Checkpoint 2

checkpoint two body
";

// ---------------------------------------------------------------------------
// Shared fixture helpers
// ---------------------------------------------------------------------------

fn write_target_file(cwd: &Path, relative_name: &str, content: &str) {
    let path = canonical_path_for_target_file(cwd, relative_name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, content).unwrap();
}

fn read_target_file(cwd: &Path, relative_name: &str) -> Vec<u8> {
    std::fs::read(canonical_path_for_target_file(cwd, relative_name)).unwrap()
}

/// Writes all four fixed target files, each independently splittable
/// (non-empty, well-formed record boundaries per BC-1.18.008's marker
/// table), under the fixed `v1.0-brownfield-backfill` cycle directory.
fn setup_all_four_valid(cwd: &Path) {
    write_target_file(cwd, "decision-log.md", DECISION_LOG_CONTENT);
    write_target_file(cwd, "burst-log.md", BURST_LOG_CONTENT);
    write_target_file(cwd, "lessons.md", LESSONS_CONTENT);
    write_target_file(cwd, "session-checkpoints.md", SESSION_CHECKPOINTS_CONTENT);
}

fn snapshot_all_four(cwd: &Path) -> Vec<(&'static str, Vec<u8>)> {
    APPEND_LOG_TARGET_FILES_IN_ORDER
        .iter()
        .map(|&name| (name, read_target_file(cwd, name)))
        .collect()
}

fn migration_state_dir(cwd: &Path) -> PathBuf {
    cwd.join(".factory/migration-state")
}

fn completed_json_path(cwd: &Path) -> PathBuf {
    migration_state_dir(cwd).join("completed.json")
}

fn write_completed_json(cwd: &Path, record: &AppendLogCompletedMigrationRecord) {
    let dir = migration_state_dir(cwd);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("completed.json"),
        serde_json::to_string_pretty(record).unwrap(),
    )
    .unwrap();
}

/// Reconstructs a target file's ORIGINAL (pre-migration) content by reading
/// back, in `seq` order, every sealed shard its `<stem>.shard-index.toml`
/// names (if any — an under-cap fixture legitimately produces zero sealed
/// shards, per BC-1.18.008 EC-016), followed by the current canonical file.
/// BC-1.18.013 Postcondition 1 delegates content-preservation entirely to
/// BC-1.18.008 Postcondition 6(a)'s own "concatenation reproduces the
/// original byte-for-byte" contract — this helper performs that exact
/// reconstruction end-to-end through real on-disk artifacts the governed
/// migration produced, independent of whatever internal shard_cap_bytes it
/// used.
fn reconstruct_from_split_or_unsplit(cwd: &Path, relative_name: &str) -> Vec<u8> {
    let stem = artifact_stem_for_target_file(relative_name)
        .expect("relative_name must be one of the four fixed target files");
    let canonical_path = canonical_path_for_target_file(cwd, relative_name);
    let index_path = canonical_path.with_file_name(format!("{stem}.shard-index.toml"));
    let mut reconstructed = Vec::new();
    if let Ok(index_toml) = std::fs::read_to_string(&index_path) {
        let index: ShardIndex =
            toml::from_str(&index_toml).expect("a shard-index, if present, must be valid TOML");
        let mut entries: Vec<_> = index.shards.iter().collect();
        entries.sort_by_key(|e| e.seq);
        for entry in entries {
            let shard_path = canonical_path.with_file_name(&entry.path);
            reconstructed.extend_from_slice(
                &std::fs::read(&shard_path)
                    .unwrap_or_else(|e| panic!("sealed shard {shard_path:?} must exist: {e}")),
            );
        }
    }
    reconstructed.extend_from_slice(
        &std::fs::read(&canonical_path).unwrap_or_else(|e| {
            panic!("canonical current file {canonical_path:?} must exist: {e}")
        }),
    );
    reconstructed
}

fn decision_log_shard_entry(shard_cap_bytes: u64) -> ShardEntry {
    ShardEntry {
        artifact_stem: "decision-log".to_string(),
        artifact_path: ".factory/cycles/v1.0-brownfield-backfill/decision-log.md".to_string(),
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

fn append_log_target_payload(
    cwd: &Path,
    tool_name: &str,
    target_file: &str,
    extra: serde_json::Value,
) -> HookPayload {
    let target = canonical_path_for_target_file(cwd, target_file);
    let mut tool_input = extra;
    if let Some(map) = tool_input.as_object_mut() {
        map.insert(
            "file_path".to_string(),
            serde_json::Value::String(target.to_string_lossy().into_owned()),
        );
    }
    HookPayload {
        event_name: "PreToolUse".to_string(),
        tool_name: tool_name.to_string(),
        session_id: "sess-s2506".to_string(),
        tool_input,
        tool_response: None,
        extra: Default::default(),
    }
}

fn write_append_log_migration_txn(cwd: &Path, state: &str) {
    let msd = migration_state_dir(cwd);
    std::fs::create_dir_all(&msd).unwrap();
    std::fs::write(
        msd.join("txn-s2506-sample.json"),
        format!(
            r#"{{"txn_id":"txn-s2506","activation_id":"activation-s2506","fencing_generation":1,"state":"{state}","generation_id":null,"source_sha256":null,"intent_log_path":null,"pending_canonical_moves":[],"created_at":"2026-09-25T00:00:00Z","updated_at":"2026-09-25T00:00:00Z"}}"#
        ),
    )
    .unwrap();
}

// ---------------------------------------------------------------------------
// append_log_markers — AC-002 / AC-003 / AC-014 / EC-007 / VP-144
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_013_PC6_artifact_stem_for_target_file_maps_all_four_fixed_files() {
    let expected = [
        ("decision-log.md", "decision-log"),
        ("burst-log.md", "burst-log"),
        ("lessons.md", "lessons"),
        ("session-checkpoints.md", "session-checkpoints"),
    ];
    for (relative, expected_stem) in expected {
        assert_eq!(
            artifact_stem_for_target_file(relative),
            Ok(expected_stem),
            "BC-1.18.013 Postcondition 6: {relative:?} must resolve to stem {expected_stem:?}"
        );
    }
}

#[test]
fn test_BC_1_18_013_PC6_INV5_artifact_stem_for_target_file_rejects_unknown_file() {
    for bad in ["STATE.md", "decision-log.MD", "../decision-log.md", ""] {
        let result = artifact_stem_for_target_file(bad);
        assert_eq!(
            result,
            Err(AppendLogMarkerError::UnknownTargetFile {
                relative_path: bad.to_string()
            }),
            "BC-1.18.013 Postcondition 6 / Invariant 5: {bad:?} is not one of the four ratified \
             target files and must be rejected — never a wildcard or auto-discovery; got \
             {result:?}"
        );
    }
}

#[test]
fn test_BC_1_18_013_AC002_AC003_VP144_record_boundary_offsets_for_target_file_matches_direct_bc_1_18_008_oracle_for_all_four_stems()
 {
    let fixtures = [
        ("decision-log.md", DECISION_LOG_CONTENT),
        ("burst-log.md", BURST_LOG_CONTENT),
        ("lessons.md", LESSONS_CONTENT),
        ("session-checkpoints.md", SESSION_CHECKPOINTS_CONTENT),
    ];
    for (relative, content) in fixtures {
        let stem = artifact_stem_for_target_file(relative).unwrap();
        let reference = mechanism_a_record_boundary_offsets(stem, content.as_bytes());
        assert!(
            !reference.is_empty(),
            "fixture sanity: {relative} must have at least one real record boundary"
        );
        let via_wrapper = record_boundary_offsets_for_target_file(relative, content.as_bytes());
        assert_eq!(
            via_wrapper,
            Ok(reference.clone()),
            "VP-144 / AC-002 / AC-003: append_log_markers' wrapper for {relative:?} must return \
             byte-identical offsets to BC-1.18.008's own mechanism_a_record_boundary_offsets \
             oracle — no divergent or duplicated verification logic (Architecture Compliance \
             Rule 3); got {via_wrapper:?}, reference={reference:?}"
        );
    }
}

#[test]
fn test_BC_1_18_013_AC014_EC007_record_boundary_offsets_for_target_file_raises_empty_boundary_oracle_on_nonempty_content_with_no_markers()
 {
    // lessons.md content with a heading that does NOT match any of the
    // ID-tagged `## L-<tag>-NNN` / `## LESSON (D-NNN)` /
    // `## RECURRENCE NOTE (D-NNN)` marker forms — zero real record
    // boundaries despite non-empty content (EC-007).
    let content = b"## Untagged heading with no lesson id\n\nsome prose body, never a record\n";
    let result = record_boundary_offsets_for_target_file("lessons.md", content);
    assert_eq!(
        result,
        Err(AppendLogMarkerError::EmptyBoundaryOracle {
            relative_path: "lessons.md".to_string()
        }),
        "AC-014/EC-007: non-empty content yielding zero record boundaries must raise \
         EmptyBoundaryOracle, never a silent single-record fallback; got {result:?}"
    );
}

// GREEN-today control: canonical_path_for_target_file is already real,
// pure path arithmetic (per the stub-architect commit report). Locks the
// schema this whole file relies on.
#[test]
fn test_BC_1_18_013_canonical_path_for_target_file_resolves_under_the_fixed_cycle_dir() {
    let repo_root = Path::new("/repo");
    let path = canonical_path_for_target_file(repo_root, "decision-log.md");
    assert_eq!(
        path,
        Path::new("/repo/.factory/cycles/v1.0-brownfield-backfill/decision-log.md")
    );
    assert_eq!(APPEND_LOG_CYCLE_DIR, "v1.0-brownfield-backfill");
    assert_eq!(
        APPEND_LOG_TARGET_FILES_IN_ORDER,
        [
            "decision-log.md",
            "burst-log.md",
            "lessons.md",
            "session-checkpoints.md"
        ]
    );
}

// ---------------------------------------------------------------------------
// executor::append_log_backfill_admission_precheck /
// append_log_backfill_reservation_release — Precondition 6(b)/(c), the
// writer-exclusion infrastructure AC-004/AC-006's atomicity relies on.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_013_PRECOND6B_admission_precheck_blocks_write_when_txn_staging() {
    let dir = tempfile::tempdir().unwrap();
    setup_all_four_valid(dir.path());
    write_append_log_migration_txn(dir.path(), "STAGING");
    let payload = append_log_target_payload(
        dir.path(),
        "Write",
        "decision-log.md",
        serde_json::json!({ "content": "a non-participating writer's mutation" }),
    );
    let result = append_log_backfill_admission_precheck(&payload, dir.path());
    assert!(
        matches!(result, Some(HookResult::Block { .. })),
        "BC-1.18.013 Precondition 6(b): a Write dispatch targeting a .factory/cycles/ path while \
         a backfill-append-logs txn is STAGING must be BLOCKED (HookResult::Block, never \
         HookResult::Error); got {result:?}"
    );
}

#[test]
fn test_BC_1_18_013_PRECOND6B_admission_precheck_blocks_multi_edit_when_txn_committing() {
    let dir = tempfile::tempdir().unwrap();
    setup_all_four_valid(dir.path());
    write_append_log_migration_txn(dir.path(), "COMMITTING");
    let payload = append_log_target_payload(
        dir.path(),
        "MultiEdit",
        "burst-log.md",
        serde_json::json!({ "edits": [] }),
    );
    let result = append_log_backfill_admission_precheck(&payload, dir.path());
    assert!(
        matches!(result, Some(HookResult::Block { .. })),
        "BC-1.18.013 Precondition 6(b): the gate must block MultiEdit while COMMITTING, \
         regardless of PID liveness; got {result:?}"
    );
}

#[test]
fn test_BC_1_18_013_PRECOND6_admission_precheck_returns_none_when_no_migration_state_dir() {
    let dir = tempfile::tempdir().unwrap();
    setup_all_four_valid(dir.path());
    let payload = append_log_target_payload(
        dir.path(),
        "Write",
        "decision-log.md",
        serde_json::json!({ "content": "ordinary write" }),
    );
    let result = append_log_backfill_admission_precheck(&payload, dir.path());
    assert!(
        result.is_none(),
        "with no .factory/migration-state/ directory present at all, this gate must be a \
         zero-cost no-op (None)"
    );
}

#[test]
fn test_BC_1_18_013_PRECOND6_admission_precheck_returns_none_for_post_tool_use_event() {
    let dir = tempfile::tempdir().unwrap();
    setup_all_four_valid(dir.path());
    write_append_log_migration_txn(dir.path(), "STAGING");
    let mut payload = append_log_target_payload(
        dir.path(),
        "Write",
        "decision-log.md",
        serde_json::json!({ "content": "x" }),
    );
    payload.event_name = "PostToolUse".to_string();
    let result = append_log_backfill_admission_precheck(&payload, dir.path());
    assert!(
        result.is_none(),
        "this gate is PreToolUse-only, mirroring shard_cap_precheck / \
         bc_index_migration_admission_precheck"
    );
}

#[test]
fn test_BC_1_18_013_admission_precheck_returns_none_for_bash_tool_leg_shipped_separately() {
    // BC-1.18.013 Precondition 6(b): "the Edit/Write/MultiEdit legs ship
    // with cluster-5 F4 TDD in executor.rs; the Bash leg ships separately
    // as part of [D-1232-OBL-4]" — mirrors BC-1.18.011's own Ruling 3
    // (^Bash$ pre-shell classifier is a SEPARATE, not-yet-scheduled guard).
    let dir = tempfile::tempdir().unwrap();
    setup_all_four_valid(dir.path());
    write_append_log_migration_txn(dir.path(), "STAGING");
    let payload = append_log_target_payload(
        dir.path(),
        "Bash",
        "decision-log.md",
        serde_json::json!({ "command": "echo hi" }),
    );
    let result = append_log_backfill_admission_precheck(&payload, dir.path());
    assert!(
        result.is_none(),
        "a Bash dispatch must be out of THIS function's scope for this story — the Bash leg \
         ships separately per Precondition 6(b)'s own delivery cross-reference; got {result:?}"
    );
}

#[test]
fn test_BC_1_18_013_PRECOND6C_reservation_release_removes_reservation_file() {
    let dir = tempfile::tempdir().unwrap();
    let msd = migration_state_dir(dir.path());
    std::fs::create_dir_all(msd.join("reservations")).unwrap();
    std::fs::write(msd.join("reservations/tool-use-s2506.reservation"), b"").unwrap();
    append_log_backfill_reservation_release(dir.path(), "tool-use-s2506")
        .expect("releasing an existing writer reservation must succeed");
    assert!(
        !msd.join("reservations/tool-use-s2506.reservation").exists(),
        "BC-1.18.013 Precondition 6(c): releasing a reservation must remove its file"
    );
}

// ---------------------------------------------------------------------------
// AC-001 / EC-008 / VP-145 — closed-grammar rejection
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_013_AC001_EC008_VP145_run_backfill_append_logs_cli_rejects_argv_outside_closed_grammar()
 {
    let dir = tempfile::tempdir().unwrap();
    setup_all_four_valid(dir.path());

    let cases: Vec<(&str, Vec<String>)> = vec![
        (
            "a path argument",
            vec!["/some/other/cycle/decision-log.md".to_string()],
        ),
        (
            "a --cycle flag",
            vec![
                "--cycle".to_string(),
                "v1.0-brownfield-backfill".to_string(),
            ],
        ),
        ("an unrecognized extra flag", vec!["--verbose".to_string()]),
        (
            "--census with a trailing extra token",
            vec!["--census".to_string(), "extra".to_string()],
        ),
        ("a short -h flag", vec!["-h".to_string()]),
        (
            "the subcommand name re-specified as an argument",
            vec!["backfill-append-logs".to_string()],
        ),
        (
            "a shell-metacharacter-looking single token",
            vec!["--census; rm -rf /".to_string()],
        ),
    ];

    let mut failures = Vec::new();
    for (label, argv) in &cases {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_backfill_append_logs_cli(dir.path(), argv)
        }));
        match result {
            Ok(0) => failures.push(format!("{label}: exit_code=0 (must be non-zero)")),
            Ok(_) => {}
            Err(_) => failures.push(format!("{label}: panicked (todo!() not yet implemented)")),
        }
    }
    assert!(
        failures.is_empty(),
        "AC-001/EC-008/VP-145: every invocation form outside the closed grammar \
         (`backfill-append-logs` / `backfill-append-logs --census`) must be rejected by the \
         binary's own defense-in-depth check with a non-zero exit code and zero filesystem \
         mutation; failures:\n{}",
        failures.join("\n")
    );
    assert!(
        !migration_state_dir(dir.path()).exists(),
        "a rejected invocation must never create .factory/migration-state/"
    );
}

// ---------------------------------------------------------------------------
// AC-002 / AC-003 / AC-009 — happy-path four-file split + completed.json
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_013_PC1_AC002_run_backfill_append_logs_cli_splits_all_four_files_reproducing_original_content_byte_for_byte()
 {
    let dir = tempfile::tempdir().unwrap();
    setup_all_four_valid(dir.path());
    let originals = snapshot_all_four(dir.path());

    let exit_code = run_backfill_append_logs_cli(dir.path(), &[]);
    assert_eq!(
        exit_code, 0,
        "a clean happy-path run over four valid, independently-splittable files must exit 0"
    );
    assert!(
        completed_json_path(dir.path()).exists(),
        "Postcondition 5 / AC-009: completed.json must exist after a successful migration"
    );

    for (name, original_bytes) in &originals {
        let reconstructed = reconstruct_from_split_or_unsplit(dir.path(), name);
        assert_eq!(
            &reconstructed, original_bytes,
            "AC-002 / Postcondition 1 (delegated to BC-1.18.008 Postcondition 6(a)): {name}'s \
             (sealed shards, in seq order, + current file) must reproduce the original content \
             byte-for-byte"
        );
    }
}

#[test]
fn test_BC_1_18_013_PC5_AC009_run_backfill_append_logs_cli_writes_completed_json_as_the_bc_7_08_001_gating_signal()
 {
    let dir = tempfile::tempdir().unwrap();
    setup_all_four_valid(dir.path());

    let exit_code = run_backfill_append_logs_cli(dir.path(), &[]);
    assert_eq!(exit_code, 0);

    let completed_json = std::fs::read_to_string(completed_json_path(dir.path()))
        .expect("AC-009/Postcondition 7/Invariant 4: completed.json must exist and be readable");
    let record: AppendLogCompletedMigrationRecord = serde_json::from_str(&completed_json)
        .expect("completed.json must deserialize as AppendLogCompletedMigrationRecord");
    assert_eq!(
        record.file_count, 4,
        "AC-009: the completed.json record must attest all four canonical files, the fixed \
         scope this migration's completion gates BC-7.08.001's Cohort-B flip on"
    );
    assert!(!record.txn_id.is_empty());
    assert!(!record.generation_id.is_empty());
    assert!(!record.completed_at.is_empty());
}

#[test]
fn test_BC_1_18_013_PC8_AC010_run_backfill_append_logs_cli_never_writes_shard_config_toml() {
    let dir = tempfile::tempdir().unwrap();
    setup_all_four_valid(dir.path());
    let shard_config_path = dir.path().join(".factory/shard-config.toml");
    assert!(!shard_config_path.exists());

    let exit_code = run_backfill_append_logs_cli(dir.path(), &[]);
    assert_eq!(exit_code, 0);

    assert!(
        !shard_config_path.exists(),
        "AC-010/Postcondition 8: `.factory/shard-config.toml` is NOT part of this migration \
         binary's ADR-052 §Decision 8 write-target allowlist — ShardRegistry enrollment is a \
         SEPARATE, ordinary Edit/Write step outside this binary's authority. The migration \
         binary must never create or modify it."
    );
}

#[test]
fn test_BC_1_18_013_PC6_INV5_AC008_run_backfill_append_logs_cli_never_touches_files_outside_the_fixed_four()
 {
    let dir = tempfile::tempdir().unwrap();
    setup_all_four_valid(dir.path());

    // A 5th file in the SAME cycle directory — never touched (no wildcard).
    let extra_path = canonical_path_for_target_file(dir.path(), "extra-notes.md");
    std::fs::create_dir_all(extra_path.parent().unwrap()).unwrap();
    std::fs::write(&extra_path, "unrelated sentinel content\n").unwrap();
    let extra_snapshot = std::fs::read(&extra_path).unwrap();

    // The SAME four filenames, under a DIFFERENT cycle directory — never
    // touched without a separate ADR-052 allowlist amendment.
    let other_cycle_dir = dir
        .path()
        .join(".factory/cycles/v1.0-feature-engine-discipline-pass-1");
    std::fs::create_dir_all(&other_cycle_dir).unwrap();
    let mut other_cycle_snapshots = Vec::new();
    for name in APPEND_LOG_TARGET_FILES_IN_ORDER {
        let path = other_cycle_dir.join(name);
        let content = format!("sentinel content for other-cycle {name}\n");
        std::fs::write(&path, &content).unwrap();
        other_cycle_snapshots.push((path, content));
    }

    let exit_code = run_backfill_append_logs_cli(dir.path(), &[]);
    assert_eq!(
        exit_code, 0,
        "the fixed four-file happy path must still succeed alongside unrelated files"
    );

    assert_eq!(
        std::fs::read(&extra_path).unwrap(),
        extra_snapshot,
        "AC-008: a 5th file in the SAME cycle directory must never be touched (no wildcard)"
    );
    for (path, content) in &other_cycle_snapshots {
        assert_eq!(
            std::fs::read(path).unwrap(),
            content.as_bytes(),
            "AC-008/Postcondition 6: another cycle's same-named files must never be touched \
             without a separate ADR-052 allowlist amendment"
        );
    }
}

// ---------------------------------------------------------------------------
// AC-013 / EC-006 — oversized single-record passthrough (tolerated, not an
// abort) vs. AC-014 / EC-007 — empty-boundary-oracle (hard failure).
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_013_EC006_AC013_run_backfill_append_logs_cli_tolerates_an_oversized_single_record_without_aborting()
 {
    let dir = tempfile::tempdir().unwrap();
    // burst-log.md: 3 records; the middle one deliberately far larger than
    // any plausible real shard_cap_bytes (ADR-051's real F4-measured caps
    // for these four artifacts sit in the hundreds-of-KB range) — BC-
    // 1.18.008's own oversized-record exception (EC-006/EC-017) must seal
    // it whole, never split mid-record, and this must NOT abort the
    // migration (distinguishing it from AC-014/EC-007's hard failure below).
    let oversized_body = "x".repeat(2 * 1024 * 1024);
    let burst_log_content = format!(
        "## Burst 1\n\nsmall body\n\n## Burst 2 (oversized)\n\n{oversized_body}\n\n## Burst 3\n\nsmall body\n"
    );
    write_target_file(dir.path(), "burst-log.md", &burst_log_content);
    write_target_file(dir.path(), "decision-log.md", DECISION_LOG_CONTENT);
    write_target_file(dir.path(), "lessons.md", LESSONS_CONTENT);
    write_target_file(
        dir.path(),
        "session-checkpoints.md",
        SESSION_CHECKPOINTS_CONTENT,
    );
    let originals = snapshot_all_four(dir.path());

    let exit_code = run_backfill_append_logs_cli(dir.path(), &[]);
    assert_eq!(
        exit_code, 0,
        "AC-013/EC-006: an oversized single record must NOT abort the migration — BC-1.18.008's \
         own oversized_record exception applies and the census-agreement gate tolerates it"
    );
    assert!(completed_json_path(dir.path()).exists());

    for (name, original_bytes) in &originals {
        let reconstructed = reconstruct_from_split_or_unsplit(dir.path(), name);
        assert_eq!(
            &reconstructed, original_bytes,
            "Postcondition 1, even across an oversized-record shard: {name}'s (shards + \
             current) reconstruction must reproduce the original content byte-for-byte"
        );
    }
}

#[test]
fn test_BC_1_18_013_EC007_AC014_PC4_AC006_run_backfill_append_logs_cli_whole_migration_aborts_when_one_file_has_zero_record_boundaries()
 {
    let dir = tempfile::tempdir().unwrap();
    write_target_file(dir.path(), "decision-log.md", DECISION_LOG_CONTENT);
    write_target_file(dir.path(), "burst-log.md", BURST_LOG_CONTENT);
    // lessons.md: non-empty content, but no ID-tagged/LESSON/RECURRENCE-NOTE
    // heading anywhere — zero real record boundaries despite non-empty
    // content (EC-007), even though the OTHER three files would stage
    // cleanly on their own.
    write_target_file(
        dir.path(),
        "lessons.md",
        "## Untagged heading\n\nprose with no lesson id at all\n",
    );
    write_target_file(
        dir.path(),
        "session-checkpoints.md",
        SESSION_CHECKPOINTS_CONTENT,
    );
    let originals = snapshot_all_four(dir.path());

    let exit_code = run_backfill_append_logs_cli(dir.path(), &[]);
    assert_eq!(
        exit_code, 2,
        "AC-014/EC-007: a zero-boundary-oracle file must surface CONTENT_PRESERVATION_ABORT \
         (exit 2) for the WHOLE migration, never a silent empty-oracle partition"
    );
    assert!(
        !completed_json_path(dir.path()).exists(),
        "AC-006/Postcondition 4: an abort must never write completed.json"
    );
    for (name, original_bytes) in &originals {
        assert_eq!(
            &read_target_file(dir.path(), name),
            original_bytes,
            "AC-004/AC-006/EC-001: a single file's failure must leave ALL FOUR files — \
             including {name}, which itself would have staged cleanly — completely untouched, \
             not merely the failing one"
        );
    }
}

// ---------------------------------------------------------------------------
// AC-012 / EC-005 — missing / unreadable target file abort
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_013_EC005_AC012_run_backfill_append_logs_cli_aborts_before_any_state_when_a_target_file_is_missing()
 {
    let dir = tempfile::tempdir().unwrap();
    setup_all_four_valid(dir.path());
    std::fs::remove_file(canonical_path_for_target_file(dir.path(), "lessons.md")).unwrap();

    let exit_code = run_backfill_append_logs_cli(dir.path(), &[]);
    assert_ne!(
        exit_code, 0,
        "AC-012/EC-005: a missing target file must abort before acquiring the flock or building \
         any staging generation"
    );
    assert!(
        !migration_state_dir(dir.path()).exists(),
        "AC-012/EC-005: no partial migration state may be written for any of the four files \
         when one is missing at activation time"
    );
    assert!(
        canonical_path_for_target_file(dir.path(), "decision-log.md").exists(),
        "the three present files must remain untouched"
    );
}

// Chmod-based unreadability check assumes the test process does not run as
// root (root bypasses POSIX permission bits) — a well-known caveat of this
// style of fixture, documented rather than worked around with an added
// dependency.
#[cfg(unix)]
#[test]
fn test_BC_1_18_013_EC005_AC012_run_backfill_append_logs_cli_aborts_when_a_target_file_is_unreadable()
 {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    setup_all_four_valid(dir.path());
    let burst_log_path = canonical_path_for_target_file(dir.path(), "burst-log.md");
    std::fs::set_permissions(&burst_log_path, std::fs::Permissions::from_mode(0o000)).unwrap();

    let exit_code = run_backfill_append_logs_cli(dir.path(), &[]);

    // Restore permissions before any assertion can panic and leave an
    // unreadable file behind for the tempdir's own Drop cleanup.
    std::fs::set_permissions(&burst_log_path, std::fs::Permissions::from_mode(0o644)).unwrap();

    assert_ne!(
        exit_code, 0,
        "AC-012/EC-005: an unreadable target file must abort before acquiring the flock or \
         building any staging generation"
    );
    assert!(!migration_state_dir(dir.path()).exists());
}

// ---------------------------------------------------------------------------
// AC-007 / EC-004 — two-layer idempotency
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_013_PC5_EC004_run_backfill_append_logs_cli_already_migrated_short_circuits_with_zero_mutation()
 {
    let dir = tempfile::tempdir().unwrap();
    setup_all_four_valid(dir.path());
    write_completed_json(
        dir.path(),
        &AppendLogCompletedMigrationRecord {
            generation_id: "gen-already-done".to_string(),
            txn_id: "txn-already-done".to_string(),
            completed_at: "2026-09-20T00:00:00Z".to_string(),
            file_count: 4,
        },
    );
    let originals = snapshot_all_four(dir.path());
    let msd_before: Vec<_> = std::fs::read_dir(migration_state_dir(dir.path()))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();

    let exit_code = run_backfill_append_logs_cli(dir.path(), &[]);
    assert_eq!(
        exit_code, 0,
        "EC-004: ALREADY_MIGRATED must be exit 0, never an error"
    );
    for (name, original_bytes) in &originals {
        assert_eq!(
            &read_target_file(dir.path(), name),
            original_bytes,
            "EC-004/Postcondition 5: a completed.json short-circuit must perform ZERO \
             filesystem mutation on {name}"
        );
    }
    let msd_after: Vec<_> = std::fs::read_dir(migration_state_dir(dir.path()))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(
        msd_before, msd_after,
        "EC-004: no lock is acquired and no new migration-state artifact is created on the \
         idempotent short-circuit path"
    );
}

#[test]
fn test_BC_1_18_013_CENSUS_run_backfill_append_logs_cli_census_flag_is_read_only_after_completion()
{
    let dir = tempfile::tempdir().unwrap();
    setup_all_four_valid(dir.path());
    write_completed_json(
        dir.path(),
        &AppendLogCompletedMigrationRecord {
            generation_id: "gen-already-done".to_string(),
            txn_id: "txn-already-done".to_string(),
            completed_at: "2026-09-20T00:00:00Z".to_string(),
            file_count: 4,
        },
    );
    let originals = snapshot_all_four(dir.path());

    let exit_code = run_backfill_append_logs_cli(dir.path(), &["--census".to_string()]);
    assert_eq!(
        exit_code, 0,
        "Canonical Test Vector: `--census` against an already-completed migration must succeed \
         read-only, exit 0"
    );
    for (name, original_bytes) in &originals {
        assert_eq!(
            &read_target_file(dir.path(), name),
            original_bytes,
            "`--census` must never mutate {name}"
        );
    }
}

#[test]
fn test_BC_1_18_013_PC5_AC007_run_backfill_append_logs_cli_tolerates_a_pre_existing_bc_1_18_008_manifest_on_one_file()
 {
    // Postcondition 5's Composability clause: a file may ALREADY carry a
    // BC-1.18.008 manifest from an earlier BC-1.18.006 roll (roll-before-
    // backfill ordering) BEFORE this governed migration ever runs. This
    // test seeds that condition using the REAL, already-implemented
    // `run_mechanism_a_backfill_split` directly against decision-log.md,
    // then drives the governed migration through the stub under test.
    let dir = tempfile::tempdir().unwrap();
    setup_all_four_valid(dir.path());

    let canonical_path = canonical_path_for_target_file(dir.path(), "decision-log.md");
    let original_bytes = std::fs::read(&canonical_path).unwrap();
    let offsets = mechanism_a_record_boundary_offsets("decision-log", &original_bytes);
    assert!(!offsets.is_empty(), "fixture sanity check");
    let cap = (original_bytes.len() as u64 / 2).max(1);
    let entry = decision_log_shard_entry(cap);
    run_mechanism_a_backfill_split(&entry, &canonical_path, &offsets, 10).expect(
        "pre-seeding a real BC-1.18.008 split via the already-implemented function must succeed",
    );
    assert!(
        mechanism_a_backfill_already_migrated(&canonical_path, "decision-log").unwrap(),
        "fixture sanity: decision-log.md must now carry a real BC-1.18.008 backfill manifest"
    );

    let exit_code = run_backfill_append_logs_cli(dir.path(), &[]);
    assert_eq!(
        exit_code, 0,
        "Postcondition 5: a pre-existing per-file BC-1.18.008 manifest on ONE of the four files \
         (from an earlier roll-before-backfill) must not abort the governed migration — it must \
         complete normally for all four files, re-running BC-1.18.008's own per-file idempotency \
         check rather than assuming none have a pre-existing manifest"
    );
    assert!(
        completed_json_path(dir.path()).exists(),
        "the governed migration's own completed.json must still be written"
    );
}

// ---------------------------------------------------------------------------
// AC-005 — pre-commit source-fingerprint TOCTOU recheck (genuine ambiguity
// flagged in this file's module doc comment: no granular resume function is
// exposed, so this test pre-seeds only the serde-round-trippable txn record
// at its BC-documented path and assumes startup-time STAGING discovery).
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_013_PC3A_AC005_run_backfill_append_logs_cli_aborts_with_fingerprint_mismatch_when_a_resumed_staging_txn_source_hash_no_longer_matches()
 {
    let dir = tempfile::tempdir().unwrap();
    setup_all_four_valid(dir.path());
    let originals = snapshot_all_four(dir.path());

    let concatenated: Vec<u8> = originals
        .iter()
        .flat_map(|(_, bytes)| bytes.clone())
        .collect();
    let wrong_source_sha256 = {
        use sha2::{Digest, Sha256};
        let mut mutated = concatenated.clone();
        mutated.push(b'!');
        format!("{:x}", Sha256::digest(&mutated))
    };

    let txn = AppendLogMigrationTxnRecord {
        txn_id: "txn-s2506-ac005".to_string(),
        activation_id: "activation-s2506-ac005".to_string(),
        fencing_generation: 1,
        state: AppendLogMigrationTxnState::Staging,
        generation_id: Some("gen-s2506-ac005".to_string()),
        source_sha256: Some(wrong_source_sha256),
        intent_log_path: None,
        pending_canonical_moves: vec![],
        created_at: "2026-09-25T00:00:00Z".to_string(),
        updated_at: "2026-09-25T00:00:00Z".to_string(),
    };
    let msd = migration_state_dir(dir.path());
    std::fs::create_dir_all(&msd).unwrap();
    std::fs::write(
        msd.join(format!("txn-{}.json", txn.activation_id)),
        serde_json::to_string_pretty(&txn).unwrap(),
    )
    .unwrap();

    let exit_code = run_backfill_append_logs_cli(dir.path(), &[]);
    assert_ne!(
        exit_code, 0,
        "AC-005/Postcondition 3a: resuming a STAGING txn whose recorded source_sha256 no longer \
         matches the four files' current concatenated content must abort \
         (FINGERPRINT_MISMATCH_ABORT), never silently proceed to the pointer swap"
    );
    for (name, original_bytes) in &originals {
        assert_eq!(
            &read_target_file(dir.path(), name),
            original_bytes,
            "AC-005: a fingerprint-mismatch abort must leave {name} completely untouched"
        );
    }
}

// ---------------------------------------------------------------------------
// AC-011 / EC-002 — crash / forward-recovery via genuine process crash
// injection. `--features factory-dispatcher/failpoints` only; both tests
// below are no-ops under the default feature set.
//
// `fail::cfg`/`fail::cfg_callback` set PROCESS-GLOBAL state, and this file
// runs 30+ OTHER tests concurrently by default (`cargo test` multithreads
// within one binary). Configuring an abort-on-hit callback IN-PROCESS here
// would race every other concurrently-running test that also reaches
// `Fs::rename`. Mirroring `bc_1_18_011_b2_migration_crash_injection_test.rs`,
// the actual crash-injection call happens in a genuinely separate OS
// process (re-execing this same test binary via `std::env::current_exe()`,
// `--exact` + `--test-threads=1`), invisible to every other concurrently
// running test in this binary.
// ---------------------------------------------------------------------------

// Both constants below are consumed ONLY by the `#[cfg(feature =
// "failpoints")]`-gated child-process entrypoint and its parent test further
// down this section; `#[cfg(feature = "failpoints")]` on the declarations
// themselves keeps `cargo clippy --workspace --all-targets -- -D warnings`
// (the default, non-failpoints feature set) from flagging them as dead code
// -- a mechanical compile-gate fix, not a change to any test assertion or
// coverage.
#[cfg(feature = "failpoints")]
const S2506_ENV_CWD: &str = "VSDD_S2506_CRASH_CWD";
/// Sentinel exit code the child uses when `run_backfill_append_logs_cli`
/// returned WITHOUT the failpoint ever firing an abort — distinguishes "the
/// failpoint name never fired" (see this section's shared-seam ASSUMPTION,
/// module doc comment) from a genuine crash (no ordinary exit code — the
/// process dies by signal).
#[cfg(feature = "failpoints")]
const S2506_CHILD_DID_NOT_ABORT_EXIT_CODE: i32 = 66;

/// The child-process entrypoint. A no-op under ordinary `cargo test`
/// execution (the env var is absent, so this returns immediately); becomes
/// the crash-injection child only when spawned by the parent test below,
/// which sets it.
#[cfg(feature = "failpoints")]
#[test]
fn test_s2506_crash_injection_child_entrypoint() {
    let Ok(cwd) = std::env::var(S2506_ENV_CWD) else {
        return; // not the child -- ordinary `cargo test` run, no-op
    };
    // True no-unwind crash semantics — never `fail`'s own `"panic"` action
    // (which unwinds and runs destructors), mirroring the B2 sibling
    // harness's own documented rationale.
    fail::cfg_callback("migration_fs::rename", || {
        std::process::abort();
    })
    .expect("configuring the migration_fs::rename crash-injection callback must succeed");

    // Real production execution -- the SAME entry point the
    // `backfill-append-logs` CLI invokes.
    let _ = run_backfill_append_logs_cli(Path::new(&cwd), &[]);
    std::process::exit(S2506_CHILD_DID_NOT_ABORT_EXIT_CODE);
}

#[cfg(feature = "failpoints")]
#[test]
fn test_BC_1_18_013_INV3_AC011_EC002_run_backfill_append_logs_cli_forward_recovery_resumes_cleanly_after_a_crashed_canonical_move()
 {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    let dir = tempfile::tempdir().unwrap();
    setup_all_four_valid(dir.path());
    let originals = snapshot_all_four(dir.path());

    let exe = std::env::current_exe().expect("current_exe must resolve for a test binary");
    let mut child = Command::new(exe)
        .arg("--exact")
        .arg("test_s2506_crash_injection_child_entrypoint")
        .arg("--test-threads=1")
        .arg("--nocapture")
        .env(S2506_ENV_CWD, dir.path().as_os_str())
        .env("RUST_BACKTRACE", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawning the crash-injection child process must succeed");

    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if child
            .try_wait()
            .expect("polling the crash-injection child must succeed")
            .is_some()
        {
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!(
                "AC-011/EC-002 crash-injection child did not exit within the 30s timeout — \
                 killed. Either the migration_fs::rename failpoint never fired (see this \
                 section's shared-seam ASSUMPTION, module doc comment), or the migration \
                 deadlocked."
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let output = child
        .wait_with_output()
        .expect("collecting crash-injection child output must succeed");

    #[cfg(unix)]
    let aborted = {
        use std::os::unix::process::ExitStatusExt;
        output.status.signal() == Some(6 /* SIGABRT */)
    };
    #[cfg(not(unix))]
    let aborted = !output.status.success();

    assert!(
        aborted,
        "AC-011/EC-002 fixture setup: the child process must genuinely crash (SIGABRT) via the \
         migration_fs::rename failpoint mid-migration, not exit normally — got status={:?}, \
         stdout={:?}, stderr={:?}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    // Recovery pass -- ordinary, unmodified production code, in THIS
    // (parent) process, which never itself configures a failpoint.
    let recovery_exit_code = run_backfill_append_logs_cli(dir.path(), &[]);
    assert_eq!(
        recovery_exit_code, 0,
        "AC-011/EC-002/Invariant 3: forward recovery after a crash mid canonical-path-move must \
         reach a clean COMPLETED state on the next invocation, resuming from the intent log's \
         first uncompleted move rather than re-moving an already-migrated file or deadlocking"
    );
    assert!(
        completed_json_path(dir.path()).exists(),
        "AC-011/EC-002: forward recovery must reach completed.json"
    );

    for (name, original_bytes) in &originals {
        let reconstructed = reconstruct_from_split_or_unsplit(dir.path(), name);
        assert_eq!(
            &reconstructed, original_bytes,
            "Postcondition 1, generalized across the crash boundary: {name}'s (shards + \
             current) reconstruction must reproduce the original content byte-for-byte \
             regardless of whether its own canonical move happened before or after the crash"
        );
    }
}

// ---------------------------------------------------------------------------
// GREEN-today controls — real (already-implemented) Display/serde impls.
// Not Red-Gate coverage; regression-locks for error-taxonomy conformance
// (S-25.06 Previous Story Intelligence: error-taxonomy display drift is a
// recurring finding class) and the migration schema types' wire shape.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_013_error_taxonomy_display_strings_carry_their_documented_exit_codes() {
    assert!(
        AppendLogMigrationError::ContentPreservationAbort {
            relative_path: "lessons.md".to_string()
        }
        .to_string()
        .contains("CONTENT_PRESERVATION_ABORT, exit 2"),
        "this migration's E-xxx Display strings must carry their documented exit-code suffix \
         verbatim, matching error-taxonomy.md's Message Format convention"
    );
    assert!(
        AppendLogMigrationError::FingerprintMismatchAbort
            .to_string()
            .contains("FINGERPRINT_MISMATCH_ABORT")
    );
    assert!(
        AppendLogMigrationError::ClosedGrammarRejected
            .to_string()
            .contains("CLOSED_GRAMMAR_REJECTED")
    );
}

#[test]
fn test_BC_1_18_013_PRECOND4_activation_manifest_schema_nulls_b2_only_fields() {
    let manifest = BackfillAppendLogsActivationManifest {
        activation_id: "activation-1".to_string(),
        migration_id: "backfill-append-logs".to_string(),
        repo_root_sha: "deadbeef".to_string(),
        approved_by: "human-F4-interactive".to_string(),
        expires_after_hours: 24,
        approved_arch_index_sha: None,
        expected_total_bcs: None,
    };
    let json = serde_json::to_string(&manifest).unwrap();
    let round_tripped: BackfillAppendLogsActivationManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(round_tripped, manifest);
    assert_eq!(manifest.migration_id, "backfill-append-logs");
    assert!(manifest.approved_arch_index_sha.is_none());
    assert!(manifest.expected_total_bcs.is_none());
}

#[test]
fn test_BC_1_18_013_migration_schema_types_round_trip_and_match_documented_shapes() {
    let pending = AppendLogPendingCanonicalMove {
        staging_path: "/tmp/staging/decision-log.md".to_string(),
        canonical_path: "/tmp/.factory/cycles/v1.0-brownfield-backfill/decision-log.md".to_string(),
    };
    let txn = AppendLogMigrationTxnRecord {
        txn_id: "txn-schema".to_string(),
        activation_id: "activation-schema".to_string(),
        fencing_generation: 1,
        state: AppendLogMigrationTxnState::Staging,
        generation_id: None,
        source_sha256: None,
        intent_log_path: None,
        pending_canonical_moves: vec![pending.clone()],
        created_at: "2026-09-25T00:00:00Z".to_string(),
        updated_at: "2026-09-25T00:00:00Z".to_string(),
    };
    let json = serde_json::to_string(&txn).unwrap();
    let round_tripped: AppendLogMigrationTxnRecord = serde_json::from_str(&json).unwrap();
    assert_eq!(round_tripped, txn);
    assert_eq!(round_tripped.pending_canonical_moves, vec![pending]);

    let pointer = AppendLogCurrentGenerationPointer {
        generation_id: "gen-schema".to_string(),
        status: "committing".to_string(),
        txn_id: "txn-schema".to_string(),
    };
    let pointer_json = serde_json::to_string(&pointer).unwrap();
    let pointer_rt: AppendLogCurrentGenerationPointer =
        serde_json::from_str(&pointer_json).unwrap();
    assert_eq!(pointer_rt, pointer);

    assert_eq!(
        serde_json::to_string(&AppendLogAdmissionGateState::Open).unwrap(),
        "\"OPEN\""
    );
    assert_eq!(
        serde_json::to_string(&AppendLogMigrationTxnState::Committing).unwrap(),
        "\"COMMITTING\""
    );
}
