// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! BC-1.18.011 (S-25.02 cluster-5 "shard-b2", T-11, AC-018) RED-Gate
//! coverage for the B2 governed one-time migration surface in
//! `shard_manager.rs`, plus the `executor.rs` native admission-gate
//! precedence tests the architect's Ruling 1/2/3 (task-level, this burst)
//! require.
//!
//! # BC-5.38.001 Red Gate discipline — RED (every function under test is
//! `todo!()` as of stub-architect's cluster-5 commit `adbc795a`)
//!
//! Every function this file exercises panics today via `todo!()`. Each test
//! asserts the REAL post-implementation expected outcome, mirroring
//! `bc_1_18_006_roll_test.rs`'s own established methodology: the same
//! assertion shape is correct both during Red Gate (fails against the stub)
//! and once BC-1.18.011 is implemented (should pass unchanged).
//!
//! VP classification (BC-1.18.011 §Verification Properties): VP-132
//! (proptest; content-preservation structured-row-equivalence — this file's
//! PC1 tests are example-based, not full proptest harnesses, per the
//! story's task-scope note that Red-Gate authorship here covers AC-018's
//! example vectors; a full `proptest`/golden-file harness is implementer's
//! to add alongside the real implementation). VP-133 (integration;
//! independent-census + crash-atomicity + idempotency + SS-05/SS-06
//! sub-split census). VP-134 (static-check; no-new-Cohort-B-dependency).
//!
//! # Architect Rulings baked into this file (do not deviate)
//!
//! 1. **Native-gate precedence.** `bc_index_migration_admission_precheck`
//!    must be evaluated BEFORE `shard_cap_precheck` for a BC-INDEX-path
//!    dispatch, and `shard_cap_precheck` must be structurally SKIPPED (never
//!    invoked at all) when migration-admission blocks. The gate-precedence
//!    tests below enforce this via control flow (`shard_cap_precheck` is
//!    only reachable in the `None` arm of a `match` on the migration
//!    verdict) and assert the load-bearing filesystem side effect (no
//!    roll/truncation), not merely the returned `HookResult`.
//! 2. `bc_index_migration_admission_precheck`'s refusal path returns
//!    `HookResult::Block` (E-MAINTENANCE-001) — NEVER `HookResult::Error`.
//! 3. The `^Bash$` full-command pre-shell classifier (ADR-052 §5c) and the
//!    4 dispatcher-guard amendments (§5b) are OUT OF SCOPE — this file
//!    tests Edit/Write/MultiEdit admission only, plus one negative control
//!    confirming Bash is correctly out of THIS function's scope.
//!
//! # Genuine ambiguity flagged for implementer/architect (not guessed at)
//!
//! `resume_from_staging(_txn_record, _migration_state_dir)`'s signature
//! carries neither a canonical `BC-INDEX.md` path/body nor an explicit
//! "original census" parameter, and `BcIndexMigrationTxnRecord` has no
//! field that stores the pre-split census ID set itself (only
//! `source_sha256`/`source_body_row_sha256`, both hashes, not the set). It
//! is therefore unclear whether the EC-060 mandatory census re-run
//! re-derives its baseline by re-reading the (pre-pivot, still-untouched)
//! canonical `BC-INDEX.md` at a well-known path relative to
//! `_migration_state_dir`'s parent, or from a txn-record field not yet
//! specified. This file's EC-060 tests assert only the OBSERVABLE contract
//! (a staged generation corrupted with a duplicate row must fail on resume;
//! a still-valid one must pass) without assuming the internal data source —
//! flagging this for confirmation once T-11 is implemented.
//!
//! Separately: `pre_commit_fingerprint_recheck`'s hash encoding (lowercase
//! hex vs. another encoding) is not pinned by the stub; this file assumes
//! lowercase-hex SHA-256 (`{:x}` via the `sha2` crate), consistent with the
//! rest of this crate's `sha2`-based hashing convention — flagging as an
//! inferred, not confirmed, convention.
//!
//! **RESOLVED (F-C5-P1-006, S-25.02 cluster-5 fix-burst T-11 follow-up):**
//! the `main.rs`-wiring ordering defect flagged in this paragraph's earlier
//! revision has been fixed by extracting the precedence decision into
//! `executor::resolve_shard_gate_precedence(migration_verdict, shard_cap)`
//! — a pure, lib-testable helper (`shard_cap` is a lazy `FnOnce` closure
//! specifically so "never invoked at all," not merely "outcome discarded,"
//! is directly observable by a test). `main::run`'s `shard_gate_precheck_result`
//! call site now delegates to this helper with an unchanged effective
//! outcome. The gate-precedence tests immediately below (against the real
//! precheck functions, filesystem side effects) remain load-bearing
//! integration coverage; the `resolve_shard_gate_precedence_*` tests further
//! below are the regression lock against the pure helper itself, closing
//! F-C5-P1-006 (the helper was previously inlined at the binary-crate call
//! site and unreachable from any integration test).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use factory_dispatcher::executor::{
    bc_index_migration_admission_precheck, resolve_shard_gate_precedence, shard_cap_precheck,
};
use factory_dispatcher::payload::HookPayload;
use factory_dispatcher::shard_manager::{
    BcId, BcIndexAdmissionGateState, BcIndexMigrationError, BcIndexMigrationOutcome,
    BcIndexMigrationTxnRecord, BcIndexMigrationTxnState, CompletedMigrationRecord,
    CurrentGenerationPointer, IntentLogRecord, IntentLogRecordType, IntentLogRecoveryDecision,
    PendingCanonicalMove, admit_or_block_bc_index_writer, append_intent_log_record,
    commit_current_generation_pointer, compute_body_row_sha256, compute_independent_census,
    decide_intent_log_recovery, execute_canonical_path_moves, extract_and_sort_bc_rows,
    is_bc_index_admission_open, migration_process_exit_code, pre_commit_fingerprint_recheck,
    read_active_txn_record, read_intent_log, resume_from_staging, run_bc_index_migration,
    stage_new_generation, try_acquire_migration_lock, verify_content_preservation,
    verify_independent_census, write_completed_record, write_txn_record,
};
use vsdd_hook_sdk::HookResult;

// ---------------------------------------------------------------------------
// Shared fixture helpers
// ---------------------------------------------------------------------------

fn bc_id(subsystem_major: u32, capability_minor: u32, sequence: u32) -> BcId {
    BcId {
        subsystem_major,
        capability_minor,
        sequence,
    }
}

/// A realistic (mirrors the live `BC-INDEX.md` row shape) pre-split body:
/// two subsystems, two rows each, plus a `§Summary` table whose own
/// `BC-1`/`BC-5`-style prefix cells must NOT be mistaken for full
/// `BC-X.YY.NNN` rows by the extractor.
const ORIGINAL_BODY: &str = "\
## Summary

| Subsystem | BC-S Prefix | Count | Directory |
|-----------|------------|-------|-----------|
| SS-01 Hook Dispatcher Core | BC-1 | 2 | ss-01/ |
| SS-05 Pipeline Orchestration | BC-5 | 2 | ss-05/ |

## Index by subsystem

### SS-01 — Hook Dispatcher Core (BC-1) — 2 BCs

| BC ID | Title | Status | Capability | Stories |
|-------|-------|--------|-----------|---------|
| [BC-1.01.001](ss-01/BC-1.01.001.md) | Registry rejects unknown schema version | draft | CAP-TBD | S-15.01 |
| [BC-1.02.001](ss-01/BC-1.02.001.md) | Second SS-01 row | draft | CAP-TBD | TBD |

### SS-05 — Pipeline Orchestration (BC-5) — 2 BCs

| BC ID | Title | Status | Capability | Stories |
|-------|-------|--------|-----------|---------|
| [BC-5.01.001](ss-05/BC-5.01.001.md) | First SS-05 row | draft | CAP-TBD | TBD |
| [BC-5.02.001](ss-05/BC-5.02.001.md) | Second SS-05 row | draft | CAP-TBD | TBD |
";

/// The staged per-subsystem shard bodies content-identical to
/// `ORIGINAL_BODY`'s own two subsystems' rows (order: SS-01, SS-05).
fn staged_bodies_matching_original() -> Vec<String> {
    vec![
        "### SS-01\n\n\
         | BC ID | Title | Status | Capability | Stories |\n\
         |-------|-------|--------|-----------|---------|\n\
         | [BC-1.01.001](ss-01/BC-1.01.001.md) | Registry rejects unknown schema version | draft | CAP-TBD | S-15.01 |\n\
         | [BC-1.02.001](ss-01/BC-1.02.001.md) | Second SS-01 row | draft | CAP-TBD | TBD |\n"
            .to_string(),
        "### SS-05\n\n\
         | BC ID | Title | Status | Capability | Stories |\n\
         |-------|-------|--------|-----------|---------|\n\
         | [BC-5.01.001](ss-05/BC-5.01.001.md) | First SS-05 row | draft | CAP-TBD | TBD |\n\
         | [BC-5.02.001](ss-05/BC-5.02.001.md) | Second SS-05 row | draft | CAP-TBD | TBD |\n"
            .to_string(),
    ]
}

fn original_census() -> BTreeSet<BcId> {
    [
        bc_id(1, 1, 1),
        bc_id(1, 2, 1),
        bc_id(5, 1, 1),
        bc_id(5, 2, 1),
    ]
    .into_iter()
    .collect()
}

fn sample_txn_record(state: BcIndexMigrationTxnState) -> BcIndexMigrationTxnRecord {
    BcIndexMigrationTxnRecord {
        txn_id: "txn-sample".to_string(),
        activation_id: "activation-sample".to_string(),
        fencing_generation: 1,
        state,
        generation_id: None,
        source_sha256: None,
        source_body_row_sha256: None,
        intent_log_path: None,
        pending_canonical_moves: vec![],
        created_at: "2026-09-13T00:00:00Z".to_string(),
        updated_at: "2026-09-13T00:00:00Z".to_string(),
    }
}

fn sha256_hex_of_file(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

// ---------------------------------------------------------------------------
// Postcondition 1 (PC1) — structured per-BC-row content-preservation.
// The v1.7 `source_body_row_sha256` model — NEVER whole-concat vs.
// `source_sha256` (explicitly UNSATISFIABLE per the BC).
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_011_PC1_extract_and_sort_bc_rows_excludes_summary_and_sorts_canonical_order() {
    let rows = extract_and_sort_bc_rows(ORIGINAL_BODY)
        .expect("a well-formed body with BC rows must extract");
    let ids: Vec<BcId> = rows.iter().map(|(id, _)| *id).collect();
    assert_eq!(
        ids,
        vec![
            bc_id(1, 1, 1),
            bc_id(1, 2, 1),
            bc_id(5, 1, 1),
            bc_id(5, 2, 1)
        ],
        "extraction must exclude the §Summary table's non-row BC-S-prefix cells (e.g. bare \
         \"BC-1\"/\"BC-5\") and sort the real rows by canonical BC-ID order"
    );
}

#[test]
fn test_BC_1_18_011_PC1_compute_body_row_sha256_is_deterministic_for_identical_sorted_rows() {
    let rows_a = extract_and_sort_bc_rows(ORIGINAL_BODY).expect("must extract");
    let rows_b = extract_and_sort_bc_rows(ORIGINAL_BODY).expect("must extract");
    assert_eq!(
        compute_body_row_sha256(&rows_a),
        compute_body_row_sha256(&rows_b),
        "hashing the same sorted row content twice must be deterministic"
    );
}

#[test]
fn test_BC_1_18_011_PC1_verify_content_preservation_passes_when_staged_rows_match_source() {
    let source_rows = extract_and_sort_bc_rows(ORIGINAL_BODY).expect("must extract source rows");
    let source_body_row_sha256 = compute_body_row_sha256(&source_rows);

    let staged_bodies = staged_bodies_matching_original();
    let result = verify_content_preservation(&staged_bodies, &source_body_row_sha256);
    assert!(
        result.is_ok(),
        "staged shard bodies whose extracted+sorted rows match the source hash must pass PC1: \
         {result:?}"
    );
}

#[test]
fn test_BC_1_18_011_PC1_verify_content_preservation_returns_content_preservation_abort_on_mismatch()
{
    let source_rows = extract_and_sort_bc_rows(ORIGINAL_BODY).expect("must extract source rows");
    let source_body_row_sha256 = compute_body_row_sha256(&source_rows);

    // Corrupt one staged row's title text — the staged content no longer
    // byte-matches the source.
    let mut staged_bodies = staged_bodies_matching_original();
    staged_bodies[0] = staged_bodies[0].replace("Registry rejects", "CORRUPTED TITLE");

    let result = verify_content_preservation(&staged_bodies, &source_body_row_sha256);
    assert!(
        matches!(
            result,
            Err(BcIndexMigrationError::ContentPreservationAbort { .. })
        ),
        "a byte-level staged-row mismatch must fail CONTENT_PRESERVATION_ABORT, got {result:?}"
    );
}

#[test]
fn test_BC_1_18_011_PC1_verify_content_preservation_ignores_added_subsystem_shard_manifest_section()
{
    // The staged lean body legitimately ADDS a brand-new `§Subsystem Shard
    // Manifest` section absent from the original — a whole-concatenation
    // hash comparison against a whole-file `source_sha256` would ALWAYS
    // mismatch by construction (explicitly UNSATISFIABLE per BC-1.18.011
    // v1.7). This proves PC1 is immune to that extra content: appending a
    // §Subsystem Shard Manifest-shaped block to one staged body must NOT
    // change the PC1 verdict, because the row-extractor excludes it.
    let source_rows = extract_and_sort_bc_rows(ORIGINAL_BODY).expect("must extract source rows");
    let source_body_row_sha256 = compute_body_row_sha256(&source_rows);

    let mut staged_bodies = staged_bodies_matching_original();
    staged_bodies.push(
        "## Subsystem Shard Manifest\n\nSee `shards/BC-INDEX.shard-manifest.toml`.\n".to_string(),
    );

    let result = verify_content_preservation(&staged_bodies, &source_body_row_sha256);
    assert!(
        result.is_ok(),
        "PC1 must compare structured per-BC-row content only, excluding the new §Subsystem \
         Shard Manifest section — a whole-concatenation model would spuriously fail here: \
         {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Postcondition 2 (PC2) — independent census vs. total_bcs; EC-001 dup ->
// ABORT; EC-004 SS-05/SS-06 sub-split scoped census.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_011_PC2_compute_independent_census_enumerates_ids_matching_total_bcs() {
    let census = compute_independent_census(ORIGINAL_BODY, 4)
        .expect("a fresh census whose cardinality matches total_bcs must succeed");
    assert_eq!(census, original_census());
}

#[test]
fn test_BC_1_18_011_PC2_compute_independent_census_errors_when_cardinality_diverges_from_total_bcs()
{
    // total_bcs (the independent count-oracle) claims 5; the body's own
    // fresh enumeration finds only 4 — a genuine divergence the sanity
    // bound must catch.
    let result = compute_independent_census(ORIGINAL_BODY, 5);
    assert!(
        result.is_err(),
        "a mismatch between the fresh enumeration and the total_bcs sanity bound must fail \
         loud, got {result:?}"
    );
}

#[test]
fn test_BC_1_18_011_PC2_verify_independent_census_passes_when_every_id_in_exactly_one_shard() {
    let staged_bodies = staged_bodies_matching_original();
    let staged_bc_index_body = "## Summary\n\n## Subsystem Shard Manifest\n";
    let result =
        verify_independent_census(&original_census(), &staged_bodies, staged_bc_index_body);
    assert!(
        result.is_ok(),
        "every census ID present exactly once must pass PC2: {result:?}"
    );
}

#[test]
fn test_BC_1_18_011_EC001_verify_independent_census_returns_census_mismatch_abort_on_duplicated_row()
 {
    let mut staged_bodies = staged_bodies_matching_original();
    // Duplicate BC-1.01.001's row into the SS-05 shard body too.
    staged_bodies[1].push_str(
        "| [BC-1.01.001](ss-01/BC-1.01.001.md) | Registry rejects unknown schema version | draft | CAP-TBD | S-15.01 |\n",
    );
    let staged_bc_index_body = "## Summary\n\n## Subsystem Shard Manifest\n";
    let result =
        verify_independent_census(&original_census(), &staged_bodies, staged_bc_index_body);
    assert!(
        matches!(
            result,
            Err(BcIndexMigrationError::CensusMismatchAbort { .. })
        ),
        "a row duplicated across two shard files must fail CENSUS_MISMATCH_ABORT (EC-001), got \
         {result:?}"
    );
}

#[test]
fn test_BC_1_18_011_PC2_verify_independent_census_returns_census_mismatch_abort_on_dropped_row() {
    let mut staged_bodies = staged_bodies_matching_original();
    // Drop BC-5.02.001 entirely from the staged SS-05 body.
    staged_bodies[1] = staged_bodies[1]
        .lines()
        .filter(|line| !line.contains("BC-5.02.001"))
        .collect::<Vec<_>>()
        .join("\n");
    let staged_bc_index_body = "## Summary\n\n## Subsystem Shard Manifest\n";
    let result =
        verify_independent_census(&original_census(), &staged_bodies, staged_bc_index_body);
    assert!(
        matches!(
            result,
            Err(BcIndexMigrationError::CensusMismatchAbort { .. })
        ),
        "a census ID present in ZERO staged shards must fail CENSUS_MISMATCH_ABORT, got {result:?}"
    );
}

#[test]
fn test_BC_1_18_011_PC2_verify_independent_census_fails_when_staged_bc_index_body_retains_rows() {
    let staged_bodies = staged_bodies_matching_original();
    // BC-1.18.010 Invariant 3: the staged lean BC-INDEX.md body itself must
    // carry ZERO per-BC rows. Here it still retains one.
    let staged_bc_index_body = "## Summary\n\n\
        | [BC-1.01.001](ss-01/BC-1.01.001.md) | Registry rejects unknown schema version | draft | CAP-TBD | S-15.01 |\n";
    let result =
        verify_independent_census(&original_census(), &staged_bodies, staged_bc_index_body);
    assert!(
        result.is_err(),
        "a staged BC-INDEX.md body that still retains a per-BC row must fail PC2 (BC-1.18.010 \
         Invariant 3), got {result:?}"
    );
}

#[test]
fn test_BC_1_18_011_EC004_verify_independent_census_ss05_sub_split_scoped_count() {
    // SS-05's own sub-split census verifies its BC-5.* total INDEPENDENTLY
    // of the whole-corpus census: here SS-05 alone (2 rows) is split across
    // sub-shards `.a` and `.b` (1 row each), still summing to exactly 2.
    let ss05_census: BTreeSet<BcId> = [bc_id(5, 1, 1), bc_id(5, 2, 1)].into_iter().collect();
    let staged_sub_shards = vec![
        "| [BC-5.01.001](ss-05/BC-5.01.001.md) | First SS-05 row | draft | CAP-TBD | TBD |\n"
            .to_string(),
        "| [BC-5.02.001](ss-05/BC-5.02.001.md) | Second SS-05 row | draft | CAP-TBD | TBD |\n"
            .to_string(),
    ];
    let staged_bc_index_body = "## Summary\n";
    let result = verify_independent_census(&ss05_census, &staged_sub_shards, staged_bc_index_body);
    assert!(
        result.is_ok(),
        "an SS-05-scoped census against its own two sub-shards must pass independently of the \
         whole-corpus census: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Postcondition 3 / Invariant 3 — crash-atomicity envelope: staging, the
// CURRENT.json sole commit-point, canonical path moves, completed.json.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_011_PC3_stage_new_generation_creates_directory_and_returns_uuid() {
    let dir = tempfile::tempdir().unwrap();
    let migration_state_dir = dir.path().join(".factory/migration-state");
    std::fs::create_dir_all(&migration_state_dir).unwrap();

    let generation_id = stage_new_generation(&migration_state_dir)
        .expect("staging a new generation against a fresh migration-state dir must succeed");
    assert!(
        migration_state_dir
            .join(format!("gen-{generation_id}"))
            .is_dir(),
        "stage_new_generation must durably create gen-<uuid>/ before returning"
    );
}

#[test]
fn test_BC_1_18_011_PC3_commit_current_generation_pointer_writes_current_json_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let migration_state_dir = dir.path().join(".factory/migration-state");
    std::fs::create_dir_all(&migration_state_dir).unwrap();
    let pointer = CurrentGenerationPointer {
        generation_id: "gen-abc".to_string(),
        status: "committing".to_string(),
        txn_id: "txn-abc".to_string(),
    };
    commit_current_generation_pointer(&migration_state_dir, &pointer)
        .expect("the sole commit-point write must succeed");
    let written = std::fs::read_to_string(migration_state_dir.join("CURRENT.json")).unwrap();
    assert!(
        written.contains("gen-abc") && written.contains("committing"),
        "CURRENT.json must carry the pointer's generation_id and committing status: {written}"
    );
}

#[test]
fn test_BC_1_18_011_INV3_execute_canonical_path_moves_halts_not_aborts_on_single_move_failure() {
    let dir = tempfile::tempdir().unwrap();
    let intent_log_path = dir.path().join("intent-abc.log");
    std::fs::create_dir_all(dir.path().join("gen-abc/shards")).unwrap();
    std::fs::write(
        dir.path().join("gen-abc/shards/BC-INDEX-SS-01.md"),
        "content",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("shards")).unwrap();

    let pending = vec![
        PendingCanonicalMove {
            staging_path: dir
                .path()
                .join("gen-abc/shards/BC-INDEX-SS-01.md")
                .to_string_lossy()
                .into_owned(),
            canonical_path: dir
                .path()
                .join("shards/BC-INDEX-SS-01.md")
                .to_string_lossy()
                .into_owned(),
        },
        // Second move's staging_path deliberately does not exist — this
        // move must fail WITHOUT aborting (rolling back) the whole
        // operation.
        PendingCanonicalMove {
            staging_path: dir
                .path()
                .join("gen-abc/shards/MISSING.md")
                .to_string_lossy()
                .into_owned(),
            canonical_path: dir
                .path()
                .join("shards/MISSING.md")
                .to_string_lossy()
                .into_owned(),
        },
    ];

    let completed_count = execute_canonical_path_moves(&pending, &intent_log_path).expect(
        "a single move's failure must HALT (not roll back/ABORT) — this function still returns \
         Ok with the count of moves completed before the halt",
    );
    assert_eq!(
        completed_count, 1,
        "exactly the first move must have completed before the second move's failure halted \
         further renames"
    );
    assert!(
        dir.path().join("shards/BC-INDEX-SS-01.md").exists(),
        "the first (successful) move's canonical path must remain moved — halt, not rollback"
    );
    assert!(
        !dir.path().join("shards/MISSING.md").exists(),
        "the failed second move must not have produced a canonical file"
    );
}

#[test]
fn test_BC_1_18_011_PC3_write_completed_record_writes_permanent_completed_json() {
    let dir = tempfile::tempdir().unwrap();
    let migration_state_dir = dir.path().join(".factory/migration-state");
    std::fs::create_dir_all(&migration_state_dir).unwrap();
    let record = CompletedMigrationRecord {
        generation_id: "gen-abc".to_string(),
        txn_id: "txn-abc".to_string(),
        completed_at: "2026-09-13T00:00:00Z".to_string(),
        canonical_paths_count: 11,
    };
    write_completed_record(&migration_state_dir, &record)
        .expect("writing the permanent terminal record must succeed");
    assert!(migration_state_dir.join("completed.json").exists());
}

// ---------------------------------------------------------------------------
// Postcondition 3a — exactly-once TOCTOU pre-commit source-fingerprint
// recheck, immediately before the pointer swap; mismatch -> ABORT, no
// renames.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_011_PC3a_pre_commit_fingerprint_recheck_passes_when_source_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let source_path = dir.path().join("BC-INDEX.md");
    std::fs::write(&source_path, ORIGINAL_BODY).unwrap();
    let expected = sha256_hex_of_file(&source_path);

    let result = pre_commit_fingerprint_recheck(std::slice::from_ref(&source_path), &expected);
    assert!(
        result.is_ok(),
        "an unchanged source must pass the TOCTOU recheck: {result:?}"
    );
}

#[test]
fn test_BC_1_18_011_PC3a_pre_commit_fingerprint_recheck_returns_fingerprint_mismatch_abort_on_drift()
 {
    let dir = tempfile::tempdir().unwrap();
    let source_path = dir.path().join("BC-INDEX.md");
    std::fs::write(&source_path, ORIGINAL_BODY).unwrap();
    let stale_expected_hash = "0000000000000000000000000000000000000000000000000000000000000000";
    let canonical_snapshot_before = std::fs::read(&source_path).unwrap();

    let result =
        pre_commit_fingerprint_recheck(std::slice::from_ref(&source_path), stale_expected_hash);
    assert!(
        matches!(result, Err(BcIndexMigrationError::FingerprintMismatchAbort)),
        "a source-content drift since quiescence must fail FINGERPRINT_MISMATCH_ABORT, got \
         {result:?}"
    );
    // No renames have occurred at this point — the source file itself must
    // be byte-identical to what it was before the recheck ran.
    assert_eq!(
        std::fs::read(&source_path).unwrap(),
        canonical_snapshot_before,
        "PC3a: a fingerprint-mismatch abort must leave the source file completely untouched — \
         no renames occur before this check runs"
    );
}

// ---------------------------------------------------------------------------
// Dual writer-exclusion — advisory flock + txn-record admission gate,
// independent of PID liveness (Precondition 6(a)/(b)).
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_011_PRECOND6A_try_acquire_migration_lock_succeeds_when_free() {
    let dir = tempfile::tempdir().unwrap();
    let lock_path = dir.path().join("exclusive.lock");
    std::fs::write(&lock_path, b"").unwrap(); // stable, pre-created inode

    let guard = try_acquire_migration_lock(&lock_path)
        .expect("acquiring a free advisory flock must succeed");
    assert!(
        guard.is_some(),
        "the lock must be granted when no other holder exists"
    );
}

#[test]
fn test_BC_1_18_011_PRECOND6A_try_acquire_migration_lock_returns_none_when_already_held() {
    let dir = tempfile::tempdir().unwrap();
    let lock_path = dir.path().join("exclusive.lock");
    std::fs::write(&lock_path, b"").unwrap();

    let first_guard = try_acquire_migration_lock(&lock_path)
        .expect("the first acquisition must succeed")
        .expect("the first acquisition must return Some(guard)");
    let second_attempt = try_acquire_migration_lock(&lock_path)
        .expect("a non-blocking contended attempt must not itself error");
    assert!(
        second_attempt.is_none(),
        "a SECOND, independent flock attempt on the SAME still-held lock path must return \
         Ok(None) (non-blocking), never block or error"
    );
    drop(first_guard);
}

#[test]
fn test_BC_1_18_011_PRECOND6BC_is_bc_index_admission_open_true_when_gate_open_and_no_active_txn() {
    assert!(is_bc_index_admission_open(
        BcIndexAdmissionGateState::Open,
        None
    ));
}

#[test]
fn test_BC_1_18_011_PRECOND6BC_is_bc_index_admission_open_false_when_txn_staging_regardless_of_gate()
 {
    let txn = sample_txn_record(BcIndexMigrationTxnState::Staging);
    assert!(
        !is_bc_index_admission_open(BcIndexAdmissionGateState::Open, Some(&txn)),
        "a STAGING txn must block admission even when gate_state == Open — PID liveness is \
         irrelevant"
    );
}

#[test]
fn test_BC_1_18_011_PRECOND6BC_is_bc_index_admission_open_false_when_txn_committing() {
    let txn = sample_txn_record(BcIndexMigrationTxnState::Committing);
    assert!(!is_bc_index_admission_open(
        BcIndexAdmissionGateState::Open,
        Some(&txn)
    ));
}

#[test]
fn test_BC_1_18_011_PRECOND6BC_is_bc_index_admission_open_true_when_txn_completed_or_aborted() {
    let completed = sample_txn_record(BcIndexMigrationTxnState::Completed);
    let aborted = sample_txn_record(BcIndexMigrationTxnState::Aborted);
    assert!(is_bc_index_admission_open(
        BcIndexAdmissionGateState::Open,
        Some(&completed)
    ));
    assert!(is_bc_index_admission_open(
        BcIndexAdmissionGateState::Open,
        Some(&aborted)
    ));
}

#[test]
fn test_BC_1_18_011_INV1_write_txn_record_and_read_active_txn_record_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let migration_state_dir = dir.path().join(".factory/migration-state");
    std::fs::create_dir_all(&migration_state_dir).unwrap();
    let record = sample_txn_record(BcIndexMigrationTxnState::Staging);
    write_txn_record(&migration_state_dir, &record).expect("writing the txn record must succeed");
    let read_back = read_active_txn_record(&migration_state_dir)
        .expect("reading it back must succeed")
        .expect("a just-written txn record must be found");
    assert_eq!(read_back.txn_id, record.txn_id);
    assert_eq!(read_back.state, BcIndexMigrationTxnState::Staging);
}

#[test]
fn test_BC_1_18_011_INV1_read_active_txn_record_returns_none_when_absent() {
    let dir = tempfile::tempdir().unwrap();
    let migration_state_dir = dir.path().join(".factory/migration-state");
    std::fs::create_dir_all(&migration_state_dir).unwrap();
    let result = read_active_txn_record(&migration_state_dir)
        .expect("an absent txn file must not itself be an error");
    assert!(result.is_none());
}

// ---------------------------------------------------------------------------
// Intent log — framed, checksummed, per-target expected-hash recovery
// (ADR-052 §Decision 7b).
// ---------------------------------------------------------------------------

fn sample_intent_record(
    expected_post_hash: &str,
    expected_pre_state: Option<&str>,
) -> IntentLogRecord {
    IntentLogRecord {
        txn_id: "txn-sample".to_string(),
        fencing_generation: 1,
        record_type: IntentLogRecordType::Intent,
        target_canonical: PathBuf::from("shards/BC-INDEX-SS-01.md"),
        staging_path: PathBuf::from("gen-abc/shards/BC-INDEX-SS-01.md"),
        expected_post_hash: expected_post_hash.to_string(),
        expected_pre_state: expected_pre_state.map(|s| s.to_string()),
        timestamp_utc: "2026-09-13T00:00:00Z".to_string(),
        record_checksum: "checksum-placeholder".to_string(),
    }
}

#[test]
fn test_BC_1_18_011_intent_log_append_and_read_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let intent_log_path = dir.path().join("intent-abc.log");
    let record = sample_intent_record("hash-post", Some("hash-pre"));
    append_intent_log_record(&intent_log_path, &record)
        .expect("appending a well-formed record must succeed");
    let read_back = read_intent_log(&intent_log_path).expect("reading it back must succeed");
    assert_eq!(read_back.len(), 1);
    assert_eq!(read_back[0].expected_post_hash, "hash-post");
}

#[test]
fn test_BC_1_18_011_intent_log_torn_trailing_record_treated_as_absent_never_partial() {
    let dir = tempfile::tempdir().unwrap();
    let intent_log_path = dir.path().join("intent-abc.log");
    let record = sample_intent_record("hash-post", Some("hash-pre"));
    append_intent_log_record(&intent_log_path, &record).expect("append must succeed");

    // Truncate the file mid-record to simulate a crash during the append's
    // own write — a torn record MUST be treated as absent, never parsed as
    // a partial INTENT/DONE.
    let full_bytes = std::fs::read(&intent_log_path).unwrap();
    let torn_len = full_bytes.len().saturating_sub(5).max(1);
    std::fs::write(&intent_log_path, &full_bytes[..torn_len]).unwrap();

    let read_back = read_intent_log(&intent_log_path)
        .expect("a torn trailing record must not itself be a parse error");
    assert!(
        read_back.is_empty(),
        "a single record truncated mid-write must be discarded entirely, not surfaced as a \
         partial record"
    );
}

#[test]
fn test_BC_1_18_011_intent_log_decide_recovery_treat_done_when_canonical_matches_expected_post_hash()
 {
    let record = sample_intent_record("hash-post", Some("hash-pre"));
    let decision =
        decide_intent_log_recovery(Some("hash-post"), Some("irrelevant-staging"), Some(&record));
    assert_eq!(decision, IntentLogRecoveryDecision::TreatDone);
}

#[test]
fn test_BC_1_18_011_intent_log_decide_recovery_redo_rename_when_staging_has_post_hash_and_canonical_has_pre_state()
 {
    let record = sample_intent_record("hash-post", Some("hash-pre"));
    let decision = decide_intent_log_recovery(Some("hash-pre"), Some("hash-post"), Some(&record));
    assert_eq!(decision, IntentLogRecoveryDecision::RedoRename);
}

#[test]
fn test_BC_1_18_011_intent_log_decide_recovery_fail_closed_on_absent_record() {
    let decision = decide_intent_log_recovery(Some("some-hash"), Some("some-hash"), None);
    assert!(matches!(
        decision,
        IntentLogRecoveryDecision::FailClosed { .. }
    ));
}

#[test]
fn test_BC_1_18_011_intent_log_decide_recovery_fail_closed_on_ambiguous_canonical_state() {
    let record = sample_intent_record("hash-post", Some("hash-pre"));
    // Canonical matches NEITHER expected_post_hash NOR expected_pre_state —
    // an untrustworthy state the recovery table has no safe row for.
    let decision =
        decide_intent_log_recovery(Some("hash-unexpected"), Some("hash-post"), Some(&record));
    assert!(matches!(
        decision,
        IntentLogRecoveryDecision::FailClosed { .. }
    ));
}

// ---------------------------------------------------------------------------
// EC-059 — crash mid-staging: original untouched, restart discards partial.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_011_EC059_crash_mid_staging_leaves_original_untouched_and_restart_discards_partial()
{
    let dir = tempfile::tempdir().unwrap();
    let migration_state_dir = dir.path().join(".factory/migration-state");
    std::fs::create_dir_all(&migration_state_dir).unwrap();
    let canonical_path = dir.path().join("BC-INDEX.md");
    std::fs::write(&canonical_path, ORIGINAL_BODY).unwrap();
    let original_snapshot = std::fs::read(&canonical_path).unwrap();

    // Simulate "6 of 10" shard files staged, then the process crashes —
    // nothing beyond stage_new_generation + partial file writes has run;
    // commit_current_generation_pointer (the sole commit point) was never
    // reached.
    let generation_id = stage_new_generation(&migration_state_dir)
        .expect("staging the first generation must succeed");
    let gen_dir = migration_state_dir.join(format!("gen-{generation_id}"));
    std::fs::create_dir_all(gen_dir.join("shards")).unwrap();
    for n in 1..=6 {
        std::fs::write(
            gen_dir.join(format!("shards/BC-INDEX-SS-{n:02}.md")),
            format!("partial shard {n}"),
        )
        .unwrap();
    }

    // Postcondition 3's atomicity guarantee: canonical BC-INDEX.md is
    // untouched — nothing in the sequence above ever wrote to it.
    assert_eq!(
        std::fs::read(&canonical_path).unwrap(),
        original_snapshot,
        "EC-059: BC-INDEX.md's original body must be byte-identical after a mid-staging crash \
         — staging never touches the canonical path"
    );

    // Restart: the next attempt stages a FRESH generation rather than
    // resuming the abandoned partial one — the partial staged output is
    // discarded, not healed.
    let restart_generation_id = stage_new_generation(&migration_state_dir)
        .expect("a restart after an abandoned partial staging attempt must still succeed");
    assert_ne!(
        restart_generation_id, generation_id,
        "EC-059: a fresh restart must stage a NEW generation id, discarding the abandoned \
         partial one — never silently resuming or merging into it"
    );
}

// ---------------------------------------------------------------------------
// EC-060 — resume-from-STAGING MUST re-run the full census (H3 fix), even
// from a previously-verified-complete staged state.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_011_EC060_resume_from_staging_fails_when_staged_content_now_has_a_duplicate() {
    let dir = tempfile::tempdir().unwrap();
    let migration_state_dir = dir.path().join(".factory/migration-state");
    std::fs::create_dir_all(&migration_state_dir).unwrap();
    let generation_id = "resume-test".to_string();
    let gen_dir = migration_state_dir.join(format!("gen-{generation_id}"));
    std::fs::create_dir_all(gen_dir.join("shards")).unwrap();

    // Staged content is corrupted to duplicate BC-1.01.001 across two
    // shards. EC-060/H3 requires the FULL census to be re-run on resume,
    // against THESE (corrupted) staged files — never trusted from a stale
    // prior verification.
    std::fs::write(
        gen_dir.join("shards/BC-INDEX-SS-01.md"),
        "| [BC-1.01.001](ss-01/BC-1.01.001.md) | Registry rejects unknown schema version | draft | CAP-TBD | S-15.01 |\n",
    )
    .unwrap();
    std::fs::write(
        gen_dir.join("shards/BC-INDEX-SS-05.md"),
        "| [BC-1.01.001](ss-01/BC-1.01.001.md) | Registry rejects unknown schema version | draft | CAP-TBD | S-15.01 |\n",
    )
    .unwrap();

    let mut txn = sample_txn_record(BcIndexMigrationTxnState::Staging);
    txn.generation_id = Some(generation_id);

    let result = resume_from_staging(&txn, &migration_state_dir);
    assert!(
        result.is_err(),
        "EC-060 (H3 fix): resume-from-STAGING must re-run the full census against the staged \
         generation's CURRENT on-disk content — a duplicated row must still be caught, got \
         {result:?}"
    );
}

#[test]
fn test_BC_1_18_011_EC060_resume_from_staging_succeeds_when_staged_content_still_valid() {
    let dir = tempfile::tempdir().unwrap();
    let migration_state_dir = dir.path().join(".factory/migration-state");
    std::fs::create_dir_all(&migration_state_dir).unwrap();
    let generation_id = "resume-ok".to_string();
    let gen_dir = migration_state_dir.join(format!("gen-{generation_id}"));
    std::fs::create_dir_all(gen_dir.join("shards")).unwrap();
    std::fs::write(
        gen_dir.join("shards/BC-INDEX-SS-01.md"),
        "| [BC-1.01.001](ss-01/BC-1.01.001.md) | Registry rejects unknown schema version | draft | CAP-TBD | S-15.01 |\n",
    )
    .unwrap();

    let mut txn = sample_txn_record(BcIndexMigrationTxnState::Staging);
    txn.generation_id = Some(generation_id);

    let result = resume_from_staging(&txn, &migration_state_dir);
    assert!(
        result.is_ok(),
        "a still-valid staged generation must pass the mandatory resume census re-run: \
         {result:?}"
    );
}

// ---------------------------------------------------------------------------
// EC-061 — idempotent re-run no-op (already-migrated body detected).
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_011_EC061_run_bc_index_migration_idempotent_noop_when_completed_json_present() {
    let dir = tempfile::tempdir().unwrap();
    let migration_state_dir = dir.path().join(".factory/migration-state");
    std::fs::create_dir_all(&migration_state_dir).unwrap();
    std::fs::write(
        migration_state_dir.join("completed.json"),
        r#"{"generation_id":"gen-done","txn_id":"txn-done","completed_at":"2026-09-13T00:00:00Z","canonical_paths_count":11}"#,
    )
    .unwrap();
    let canonical_path = dir.path().join("BC-INDEX.md");
    std::fs::write(&canonical_path, ORIGINAL_BODY).unwrap();
    let snapshot_before = std::fs::read(&canonical_path).unwrap();

    let outcome = run_bc_index_migration(dir.path())
        .expect("a completed.json marker must short-circuit to AlreadyMigrated, never an error");
    assert_eq!(
        outcome,
        BcIndexMigrationOutcome::AlreadyMigrated,
        "EC-061/EC-006: a re-run after successful completion must be an idempotent no-op"
    );
    assert_eq!(
        std::fs::read(&canonical_path).unwrap(),
        snapshot_before,
        "an idempotent no-op must never rewrite BC-INDEX.md's canonical body"
    );
}

// ---------------------------------------------------------------------------
// Abort codes are migration-binary PROCESS EXIT CODES — never
// HookResult/E-SHD-005 (that code is scoped exclusively to the
// steady-state native admission gate, BC-1.18.006/BC-1.18.010).
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_011_process_exit_code_expiry_abort_maps_to_exit_1() {
    assert_eq!(BcIndexMigrationError::ExpiryAbort.process_exit_code(), 1);
}

#[test]
fn test_BC_1_18_011_process_exit_code_content_and_census_aborts_map_to_exit_2() {
    let content_err = BcIndexMigrationError::ContentPreservationAbort {
        detail: "x".to_string(),
    };
    let census_err = BcIndexMigrationError::CensusMismatchAbort {
        bc_id: "BC-1.01.001".to_string(),
        detail: "x".to_string(),
    };
    assert_eq!(content_err.process_exit_code(), 2);
    assert_eq!(census_err.process_exit_code(), 2);
}

#[test]
fn test_BC_1_18_011_migration_process_exit_code_ok_variants_map_to_exit_0() {
    assert_eq!(
        migration_process_exit_code(&Ok(BcIndexMigrationOutcome::AlreadyMigrated)),
        0
    );
    assert_eq!(
        migration_process_exit_code(&Ok(BcIndexMigrationOutcome::Completed {
            canonical_paths_count: 11
        })),
        0
    );
}

#[test]
fn test_BC_1_18_011_migration_process_exit_code_err_delegates_to_process_exit_code() {
    let err = BcIndexMigrationError::CensusMismatchAbort {
        bc_id: "BC-1.01.001".to_string(),
        detail: "x".to_string(),
    };
    assert_eq!(migration_process_exit_code(&Err(err)), 2);
}

// ---------------------------------------------------------------------------
// Postcondition 7 / Invariant 4 / VP-134 — no new Cohort-B dependency
// (static-check; always-green control against the REAL project registry).
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_011_PC7_VP134_hooks_registry_has_no_bc_index_migration_dependency_for_cohort_b() {
    let registry_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../plugins/vsdd-factory/hooks-registry.toml");
    let content = std::fs::read_to_string(&registry_path)
        .expect("hooks-registry.toml must be readable from the workspace");
    for needle in ["migrate-bc-index", "bc_index_migration", "BC-1.18.011"] {
        assert!(
            !content.contains(needle),
            "hooks-registry.toml must never reference {needle:?} as a Cohort B \
             (regression-gate/convergence-tracker) gating dependency (Postcondition 7 / \
             Invariant 4)"
        );
    }
}

// ---------------------------------------------------------------------------
// Gate-precedence Ruling-1/2/3 — the native OPEN/DRAINING writer-admission
// gate takes precedence over the (destructive) shard-cap-gate roll, with
// shard_cap_precheck structurally SKIPPED, never merely outcome-discarded.
// ---------------------------------------------------------------------------

const BC_INDEX_SHARD_CONFIG: &str = "\
[[shard]]
artifact_stem = \"BC-INDEX\"
artifact_path = \".factory/specs/behavioral-contracts/BC-INDEX.md\"
practical_fuel_ceiling = 8000000
worst_case_fuel_per_byte = 106.36
max_single_record_bytes = 16384
safety_margin = 8192
shard_cap_bytes = 100
shape = \"flat\"
";

fn write_shard_config(cwd: &Path, body: &str) {
    let factory_dir = cwd.join(".factory");
    std::fs::create_dir_all(&factory_dir).unwrap();
    std::fs::write(factory_dir.join("shard-config.toml"), body).unwrap();
}

fn write_migration_txn(cwd: &Path, state: &str) {
    let migration_state_dir = cwd.join(".factory/migration-state");
    std::fs::create_dir_all(&migration_state_dir).unwrap();
    std::fs::write(
        migration_state_dir.join("txn-activation-ruling1.json"),
        format!(
            r#"{{"txn_id":"txn-ruling1","activation_id":"activation-ruling1","fencing_generation":1,"state":"{state}","generation_id":null,"source_sha256":null,"source_body_row_sha256":null,"intent_log_path":null,"pending_canonical_moves":[],"created_at":"2026-09-13T00:00:00Z","updated_at":"2026-09-13T00:00:00Z"}}"#
        ),
    )
    .unwrap();
}

fn bc_index_target(cwd: &Path) -> PathBuf {
    cwd.join(".factory/specs/behavioral-contracts/BC-INDEX.md")
}

fn bc_index_payload(
    cwd: &Path,
    tool_name: &str,
    tool_input_extra: serde_json::Value,
) -> HookPayload {
    let target = bc_index_target(cwd);
    let mut tool_input = tool_input_extra;
    if let Some(map) = tool_input.as_object_mut() {
        map.insert(
            "file_path".to_string(),
            serde_json::Value::String(target.to_string_lossy().into_owned()),
        );
    }
    HookPayload {
        event_name: "PreToolUse".to_string(),
        tool_name: tool_name.to_string(),
        session_id: "sess-bc-1-18-011".to_string(),
        tool_input,
        tool_response: None,
        extra: Default::default(),
    }
}

#[test]
fn test_BC_1_18_011_PC6_RULING1_gate_precedence_staging_blocks_shard_cap_precheck_never_runs_no_roll()
 {
    let dir = tempfile::tempdir().unwrap();
    write_shard_config(dir.path(), BC_INDEX_SHARD_CONFIG);
    write_migration_txn(dir.path(), "STAGING");
    let target = bc_index_target(dir.path());
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    // Genuinely over-cap: cap is 100 bytes; the canonical content already on
    // disk (50 bytes) plus this Write's own content (5,000 bytes) is far
    // over cap under BC-1.18.005's own Write formula (projected_size =
    // len(content) alone).
    std::fs::write(&target, "y".repeat(50)).unwrap();
    let canonical_snapshot_before = std::fs::read(&target).unwrap();

    let payload = bc_index_payload(
        dir.path(),
        "Write",
        serde_json::json!({ "content": "x".repeat(5_000) }),
    );

    // Ruling 1: bc_index_migration_admission_precheck MUST be evaluated
    // BEFORE shard_cap_precheck for a BC-INDEX-path dispatch, and
    // shard_cap_precheck MUST be structurally SKIPPED (never invoked at
    // all) when migration-admission blocks. shard_cap_precheck is only
    // reachable in the `None` arm below — it is NEVER called when the
    // migration-admission gate fires, which IS this test's central
    // assertion.
    let migration_verdict = bc_index_migration_admission_precheck(&payload, dir.path());
    let verdict = match migration_verdict {
        Some(v) => v,
        None => shard_cap_precheck(&payload, dir.path())
            .expect("a genuinely over-cap dispatch with no migration in flight must fire"),
    };

    assert!(
        matches!(verdict, HookResult::Block { .. }),
        "Ruling 2: a migration-admission refusal must surface as HookResult::Block \
         (E-MAINTENANCE-001), never HookResult::Error, got {verdict:?}"
    );
    if let HookResult::Block { reason } = &verdict {
        let lower = reason.to_lowercase();
        assert!(
            lower.contains("retry") || lower.contains("migrat"),
            "the Block reason must be retry-actionable (name the migration window and/or retry \
             guidance), got: {reason}"
        );
    }

    // The load-bearing filesystem assertion: NO roll side effect occurred,
    // even though the write was genuinely over-cap. execute_roll (reached
    // only via shard_cap_gate_check, which shard_cap_precheck calls) would
    // have truncated the canonical file to 0 bytes and published a sealed
    // shard — neither may happen here because shard_cap_precheck was never
    // reached.
    assert_eq!(
        std::fs::read(&target).unwrap(),
        canonical_snapshot_before,
        "no roll/truncation side effect may occur on a BC-INDEX-path write while a txn record \
         is STAGING, even when the write is genuinely over-cap"
    );
    assert!(
        !dir.path()
            .join(".factory/specs/behavioral-contracts/BC-INDEX.0001.md")
            .exists(),
        "no sealed shard file may be published while migration admission is blocking"
    );
}

#[test]
fn test_BC_1_18_011_PC6_RULING1_gate_precedence_committing_blocks_shard_cap_precheck_never_runs_no_roll()
 {
    // Identical to the STAGING case above but for the COMMITTING state —
    // Precondition 6(b) names BOTH STAGING and COMMITTING as blocking
    // states.
    let dir = tempfile::tempdir().unwrap();
    write_shard_config(dir.path(), BC_INDEX_SHARD_CONFIG);
    write_migration_txn(dir.path(), "COMMITTING");
    let target = bc_index_target(dir.path());
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&target, "y".repeat(50)).unwrap();
    let canonical_snapshot_before = std::fs::read(&target).unwrap();

    let payload = bc_index_payload(
        dir.path(),
        "Write",
        serde_json::json!({ "content": "x".repeat(5_000) }),
    );

    let migration_verdict = bc_index_migration_admission_precheck(&payload, dir.path());
    let verdict = match migration_verdict {
        Some(v) => v,
        None => shard_cap_precheck(&payload, dir.path())
            .expect("a genuinely over-cap dispatch with no migration in flight must fire"),
    };

    assert!(matches!(verdict, HookResult::Block { .. }));
    assert_eq!(
        std::fs::read(&target).unwrap(),
        canonical_snapshot_before,
        "no roll/truncation side effect may occur while a txn record is COMMITTING either"
    );
}

// ---------------------------------------------------------------------------
// F-C5-P1-006 (process-gap) regression lock — `resolve_shard_gate_precedence`
// ---------------------------------------------------------------------------
//
// The two filesystem-level `RULING1` tests immediately above exercise
// Ruling 1 end-to-end against the real `bc_index_migration_admission_precheck`
// / `shard_cap_precheck` functions and a real on-disk canonical file. They
// are valuable integration coverage, but they do NOT — and structurally
// cannot — pin the precedence *contract* itself: their `match` block
// hand-composes `bc_index_migration_admission_precheck` before
// `shard_cap_precheck` inline, so a future regression that reintroduces the
// pass-1-era `main.rs` defect (both prechecks computed eagerly, precedence
// applied only via `.or(...)` on the two already-computed `Option`s) would
// leave these two tests green — they never call the shared
// `resolve_shard_gate_precedence` helper `main::run` actually delegates to.
//
// These tests close that gap directly: they call
// `executor::resolve_shard_gate_precedence` itself and use a call-counter
// closure to prove the `shard_cap` argument is STRUCTURALLY never invoked
// (not merely "invoked but its result discarded") whenever the migration
// verdict is `Some(_)`. This is the load-bearing guarantee `execute_roll`'s
// destructive seal-and-truncate safety depends on.

#[test]
fn test_BC_1_18_011_PC6_RULING1_resolve_shard_gate_precedence_migration_block_shard_cap_never_invoked()
 {
    let migration_verdict = Some(HookResult::Block {
        reason: "BC-1.18.011 E-MAINTENANCE-001: migration in flight".to_string(),
    });

    let shard_cap_call_count = AtomicUsize::new(0);
    let result = resolve_shard_gate_precedence(migration_verdict, || {
        shard_cap_call_count.fetch_add(1, Ordering::SeqCst);
        // Deliberately a DIFFERENT verdict than the migration one, so a
        // passing assertion on the returned value below can only be
        // explained by the migration verdict winning, not by the closure's
        // return value happening to coincide with it.
        Some(HookResult::Block {
            reason: "shard-cap: this must never be observed".to_string(),
        })
    });

    assert_eq!(
        shard_cap_call_count.load(Ordering::SeqCst),
        0,
        "Ruling 1: the shard_cap closure must be STRUCTURALLY SKIPPED — never invoked at all — \
         when the migration verdict is Some(_). A nonzero count here is exactly the pass-1-era \
         defect (both prechecks computed eagerly, precedence applied only via `.or(...)` after \
         the destructive shard_cap_precheck/execute_roll path had already run) reintroduced."
    );
    match result {
        Some(HookResult::Block { reason }) => {
            assert_eq!(
                reason, "BC-1.18.011 E-MAINTENANCE-001: migration in flight",
                "the migration verdict must win unconditionally and be returned verbatim, not \
                 the shard_cap closure's verdict"
            );
        }
        other => {
            panic!("expected the migration Block verdict to be returned verbatim, got {other:?}")
        }
    }
}

#[test]
fn test_BC_1_18_011_PC6_RULING1_resolve_shard_gate_precedence_migration_error_variant_also_short_circuits()
 {
    // Ruling 1's "wins unconditionally" guarantee is on `Some(_)` generally,
    // not merely the `Block` variant — pin an `Error` verdict too, since a
    // future refactor narrowing the short-circuit to `matches!(_, Block)`
    // would silently let a fired migration `Error` fall through to the
    // destructive shard_cap path.
    let migration_verdict = Some(HookResult::Error {
        message: "BC-1.18.011: failed to read the active BC-INDEX migration txn record".to_string(),
    });

    let shard_cap_call_count = AtomicUsize::new(0);
    let result = resolve_shard_gate_precedence(migration_verdict, || {
        shard_cap_call_count.fetch_add(1, Ordering::SeqCst);
        None
    });

    assert_eq!(
        shard_cap_call_count.load(Ordering::SeqCst),
        0,
        "an Error-variant migration verdict must short-circuit shard_cap exactly like a Block \
         verdict does — `Some(_)` wins unconditionally regardless of which HookResult variant it \
         carries"
    );
    assert!(
        matches!(result, Some(HookResult::Error { .. })),
        "the migration Error verdict must be returned verbatim, got {result:?}"
    );
}

#[test]
fn test_BC_1_18_011_PC6_RULING1_resolve_shard_gate_precedence_migration_none_invokes_shard_cap_once_returns_some()
 {
    let shard_cap_call_count = AtomicUsize::new(0);
    let result = resolve_shard_gate_precedence(None, || {
        shard_cap_call_count.fetch_add(1, Ordering::SeqCst);
        Some(HookResult::Block {
            reason: "shard-cap: genuinely over cap".to_string(),
        })
    });

    assert_eq!(
        shard_cap_call_count.load(Ordering::SeqCst),
        1,
        "when no migration is in flight (migration_verdict = None), the shard_cap closure must \
         be invoked exactly once — never skipped, never invoked more than once"
    );
    match result {
        Some(HookResult::Block { reason }) => {
            assert_eq!(reason, "shard-cap: genuinely over cap");
        }
        other => panic!(
            "expected the shard_cap closure's verdict to be returned verbatim, got {other:?}"
        ),
    }
}

#[test]
fn test_BC_1_18_011_PC6_RULING1_resolve_shard_gate_precedence_migration_none_invokes_shard_cap_once_returns_none()
 {
    let shard_cap_call_count = AtomicUsize::new(0);
    let result = resolve_shard_gate_precedence(None, || {
        shard_cap_call_count.fetch_add(1, Ordering::SeqCst);
        None
    });

    assert_eq!(
        shard_cap_call_count.load(Ordering::SeqCst),
        1,
        "the shard_cap closure must still be invoked exactly once even when it has nothing to \
         report (the common case — no migration in flight AND the dispatch is under cap)"
    );
    assert!(
        result.is_none(),
        "with no migration verdict and a non-firing shard_cap check, the overall precedence \
         result must be None (allow the dispatch to proceed), got {result:?}"
    );
}

#[test]
fn test_BC_1_18_011_PRECOND6_admission_precheck_returns_none_when_no_migration_state_dir() {
    // Negative control — this guard is REAL (already-implemented) code per
    // stub-architect's own Red-Gate safety note, so this test is expected
    // to be GREEN today, locking in the exact zero-cost-bypass condition
    // the positive gate-precedence tests above depend on.
    let dir = tempfile::tempdir().unwrap();
    let target = bc_index_target(dir.path());
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    std::fs::write(&target, "content").unwrap();
    let payload = bc_index_payload(dir.path(), "Write", serde_json::json!({ "content": "x" }));
    let result = bc_index_migration_admission_precheck(&payload, dir.path());
    assert!(
        result.is_none(),
        "with no .factory/migration-state/ directory present at all, this gate must be a \
         zero-cost no-op (None)"
    );
}

#[test]
fn test_BC_1_18_011_RULING3_admission_precheck_returns_none_for_bash_tool_out_of_scope() {
    // Ruling 3: the ^Bash$ full-command pre-shell classifier (ADR-052
    // §Decision 5c) is a SEPARATE guard, not implemented by this function.
    // This function's own tool-kind guard covers Edit/Write/MultiEdit ONLY
    // — also real (already-implemented) code, GREEN today.
    let dir = tempfile::tempdir().unwrap();
    write_migration_txn(dir.path(), "STAGING");
    let payload = bc_index_payload(
        dir.path(),
        "Bash",
        serde_json::json!({ "command": "echo hi" }),
    );
    let result = bc_index_migration_admission_precheck(&payload, dir.path());
    assert!(
        result.is_none(),
        "a Bash dispatch must be out of THIS function's scope regardless of migration state — \
         the Bash classifier is a separate, not-yet-scheduled guard"
    );
}

#[test]
fn test_BC_1_18_011_admission_precheck_returns_none_for_post_tool_use_event() {
    // Negative control — real (already-implemented) event-type guard,
    // mirroring shard_cap_precheck's own PreToolUse-only scoping.
    let dir = tempfile::tempdir().unwrap();
    write_migration_txn(dir.path(), "STAGING");
    let mut payload = bc_index_payload(dir.path(), "Write", serde_json::json!({ "content": "x" }));
    payload.event_name = "PostToolUse".to_string();
    let result = bc_index_migration_admission_precheck(&payload, dir.path());
    assert!(
        result.is_none(),
        "this gate is PreToolUse-only, mirroring shard_cap_precheck"
    );
}

// Silence an unused-import warning for `admit_or_block_bc_index_writer` if a
// future edit trims the PreToolUse admission-check-entry-point test below it
// — kept intentionally adjacent to its own dedicated test.
#[test]
fn test_BC_1_18_011_PRECOND6_admit_or_block_bc_index_writer_creates_reservation_on_admission() {
    let dir = tempfile::tempdir().unwrap();
    let migration_state_dir = dir.path().join(".factory/migration-state");
    std::fs::create_dir_all(migration_state_dir.join("reservations")).unwrap();
    // No gate-state file and no txn record present — the OPEN default case.
    admit_or_block_bc_index_writer(&migration_state_dir, "tool-use-1")
        .expect("an admissible dispatch (OPEN gate, no active txn) must be admitted");
    assert!(
        migration_state_dir
            .join("reservations/tool-use-1.reservation")
            .exists(),
        "admission must create the writer reservation file for this tool_use_id"
    );
}

// ---------------------------------------------------------------------------
// F-C5-P1-001 (cycle-5 pass-1 adversary finding) — BC-1.18.010 Postcondition
// 1: the post-migration canonical BC-INDEX.md lean body MUST retain the
// ORIGINAL YAML frontmatter (`total_bcs` et al.), the real `## Summary`
// subsystem-registry table, and any cross-cutting invariants — it inserts
// `## Subsystem Shard Manifest` and removes ONLY the per-subsystem
// `### SS-NN` BC tables. `run_bc_index_migration`'s `staged_bc_index_body`
// is currently a hardcoded literal (`"## Summary\n\n## Subsystem Shard
// Manifest\n\nSee \`shards/BC-INDEX.shard-manifest.toml\`.\n"`) that
// discards EVERYTHING before the first `### SS-NN` heading wholesale —
// frontmatter, the real §Summary table, and any cross-cutting invariants —
// replacing it with a 4-line stub.
//
// This is a fresh-run END-TO-END test against a realistic fixture, driving
// `run_bc_index_migration` for real — NOT the PC1/PC2 unit helpers in
// isolation above, which cannot see this defect: `verify_content_preservation`
// and `verify_independent_census` only ever compare STRUCTURED PER-BC ROWS,
// never the frontmatter/§Summary/invariant prose surrounding them, so the
// migration completes with `Ok(Completed)` both before and after this fix —
// the defect is only observable by reading the actual post-migration
// canonical file content, which is what this test does.
// ---------------------------------------------------------------------------

/// A realistic pre-split `BC-INDEX.md`: YAML frontmatter with a known
/// `total_bcs`, a real `## Summary` subsystem-registry table, a
/// cross-cutting-invariant line, and two `### SS-NN` sections — SS-01 (1
/// row, deliberately small) and SS-05 (5 rows, deliberately large) — so the
/// same fixture exercises both the flat first-level split (SS-01, stays
/// under `shard_cap_bytes`) and the ADR-051 §Decision 18 second-level
/// sub-split (SS-05, exceeds `shard_cap_bytes`).
const FC5P1001_ORIGINAL_CONTENT: &str = "\
---
document_type: bc-index
version: \"1.7\"
total_bcs: 6
---

## Summary

| Subsystem | BC-S Prefix | Count | Directory |
|-----------|------------|-------|-----------|
| SS-01 Hook Dispatcher Core | BC-1 | 1 | ss-01/ |
| SS-05 Pipeline Orchestration | BC-5 | 5 | ss-05/ |

## Cross-Cutting Invariants

- INV-BC-INDEX-001: Every BC ID referenced in a story file MUST exist in this index.

## Index by subsystem

### SS-01 — Hook Dispatcher Core (BC-1) — 1 BC

| BC ID | Title | Status | Capability | Stories |
|-------|-------|--------|-----------|---------|
| [BC-1.01.001](ss-01/BC-1.01.001.md) | Registry rejects unknown schema version | draft | CAP-TBD | S-15.01 |

### SS-05 — Pipeline Orchestration (BC-5) — 5 BCs

| BC ID | Title | Status | Capability | Stories |
|-------|-------|--------|-----------|---------|
| [BC-5.01.001](ss-05/BC-5.01.001.md) | First SS-05 row with a longer descriptive title text | draft | CAP-TBD | TBD |
| [BC-5.02.001](ss-05/BC-5.02.001.md) | Second SS-05 row with a longer descriptive title text | draft | CAP-TBD | TBD |
| [BC-5.03.001](ss-05/BC-5.03.001.md) | Third SS-05 row with a longer descriptive title text | draft | CAP-TBD | TBD |
| [BC-5.04.001](ss-05/BC-5.04.001.md) | Fourth SS-05 row with a longer descriptive title text | draft | CAP-TBD | TBD |
| [BC-5.05.001](ss-05/BC-5.05.001.md) | Fifth SS-05 row with a longer descriptive title text | draft | CAP-TBD | TBD |
";

/// A `[[shard]]` config entry whose `shard_cap_bytes` (400) sits strictly
/// between the fixture's captured SS-01 section size (~265 bytes, under cap
/// — stays a single flat shard) and its captured SS-05 section size (~755
/// bytes, over cap — triggers the second-level sub-split), so this single
/// fixture exercises BOTH split paths in the same migration run.
const FC5P1001_SHARD_CONFIG: &str = "\
[[shard]]
artifact_stem = \"BC-INDEX\"
artifact_path = \".factory/specs/behavioral-contracts/BC-INDEX.md\"
practical_fuel_ceiling = 8000000
worst_case_fuel_per_byte = 106.36
max_single_record_bytes = 16384
safety_margin = 8192
shard_cap_bytes = 400
shape = \"flat\"
";

#[test]
fn test_BC_1_18_010_PC1_FC5P1001_run_bc_index_migration_preserves_frontmatter_summary_and_invariants()
 {
    let dir = tempfile::tempdir().unwrap();
    write_shard_config(dir.path(), FC5P1001_SHARD_CONFIG);
    let canonical_path = bc_index_target(dir.path());
    std::fs::create_dir_all(canonical_path.parent().unwrap()).unwrap();
    std::fs::write(&canonical_path, FC5P1001_ORIGINAL_CONTENT).unwrap();

    let outcome = run_bc_index_migration(dir.path())
        .expect("a well-formed fresh-run migration against this fixture must succeed");
    assert!(
        matches!(outcome, BcIndexMigrationOutcome::Completed { .. }),
        "expected a Completed outcome (the migration itself succeeds even with today's \
         content-destroying bug — PC1/PC2 only check structured BC rows, not \
         frontmatter/§Summary/invariants), got {outcome:?}"
    );

    let lean_body = std::fs::read_to_string(&canonical_path)
        .expect("the canonical BC-INDEX.md must still exist post-migration");

    // --- YAML frontmatter must survive verbatim (BC-1.18.010 PC1) ---
    assert!(
        lean_body.starts_with("---\n"),
        "F-C5-P1-001: the post-migration lean body must still open with the original YAML \
         frontmatter delimiter, not a bare \"## Summary\" stub; got:\n{lean_body}"
    );
    assert!(
        lean_body.contains("total_bcs: 6"),
        "F-C5-P1-001: the frontmatter's total_bcs field must survive VERBATIM into the lean \
         body, not be discarded by the hardcoded staged_bc_index_body stub; got:\n{lean_body}"
    );
    assert!(
        lean_body.contains("document_type: bc-index"),
        "F-C5-P1-001: the frontmatter's document_type field must survive; got:\n{lean_body}"
    );

    // --- the real `## Summary` subsystem-registry table must survive ---
    assert!(
        lean_body.contains("SS-01 Hook Dispatcher Core"),
        "F-C5-P1-001: the §Summary subsystem-registry table's real row content must survive \
         into the lean body, not be replaced by a bare \"## Summary\\n\\n\" heading with no \
         rows; got:\n{lean_body}"
    );
    assert!(
        lean_body.contains("SS-05 Pipeline Orchestration"),
        "F-C5-P1-001: the §Summary table's SS-05 registry row must survive; got:\n{lean_body}"
    );

    // --- cross-cutting invariants must survive ---
    assert!(
        lean_body.contains("INV-BC-INDEX-001"),
        "F-C5-P1-001: cross-cutting invariant content must survive into the lean body, not be \
         discarded along with everything else before the first ### SS-NN heading; \
         got:\n{lean_body}"
    );

    // --- the inserted §Subsystem Shard Manifest section must be present ---
    assert!(
        lean_body.contains("## Subsystem Shard Manifest"),
        "the migration must still insert the §Subsystem Shard Manifest section; got:\n{lean_body}"
    );

    // --- the per-subsystem BC tables must be REMOVED from the lean body
    // (BC-1.18.010 Invariant 3 removes ONLY these, per this test's other
    // assertions that everything else survives) ---
    assert!(
        !lean_body.contains("[BC-1.01.001]"),
        "the per-subsystem BC row for BC-1.01.001 must have MOVED to its shard file, not \
         remain in the lean canonical body; got:\n{lean_body}"
    );
    assert!(
        !lean_body.contains("[BC-5.01.001]"),
        "the per-subsystem BC row for BC-5.01.001 must have MOVED to its shard file, not \
         remain in the lean canonical body; got:\n{lean_body}"
    );

    // --- and the rows must have actually landed in SS-01's shard file, not
    // simply vanished (SS-01 stays flat — under shard_cap_bytes) ---
    let ss01_shard = std::fs::read_to_string(
        dir.path()
            .join(".factory/specs/behavioral-contracts/shards/BC-INDEX-SS-01.md"),
    )
    .expect("SS-01's shard file must exist post-migration");
    assert!(
        ss01_shard.contains("[BC-1.01.001]"),
        "SS-01's BC row content must have moved into its own shard file, confirming the row \
         was relocated (not lost) even though it is correctly absent from the lean body above"
    );
}
