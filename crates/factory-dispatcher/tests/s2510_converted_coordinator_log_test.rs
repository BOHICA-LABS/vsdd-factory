// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-25.10 RED tests -- INTERIM SELF-CONSISTENCY of the converted coordinator (AC-011, NEW,
//! split-induced): after the call-site conversion, a full successful `migrate-bc-index` run
//! produces a log that the strict `read_log` returns IN FULL with NO `MidLogCorruption` and
//! NO L1-L4 violation -- for canonical targets both ABSENT and PRESENT at INTENT time.
//!
//! Authority: ADR-054 v1.0 Decision 1.8 (L3: a `DONE` agrees with its `INTENT` on
//! `staging_path`, `expected_post_hash`, `expected_pre_state`) and Decision 3 B-2 item (c)
//! (copy rule); BC-1.18.011 v1.21 Postcondition 15(b)(c), Precondition 5; story S-25.10
//! AC-011 and its "Interim self-consistency" scope note.
//!
//! The pre-S-25.11 executor computes a `DONE`'s `expected_post_hash` from the observed file
//! and hard-codes `expected_pre_state`; the stricter reader would then reject the
//! coordinator's OWN log. The converted call site therefore builds each `DONE` by COPYING
//! `target_canonical`, `staging_path`, `expected_post_hash` and `expected_pre_state` from the
//! INTENT it completes, with the live txn's `txn_id` and the current `fencing_generation`
//! passed by S-25.09 (no log scraping). What stays S-25.11's (B-2 proper): the
//! `sha256(canonical)` COMPARISON with the INTENT, `CANONICAL_MOVE_HALTED`, the live-parameter
//! assertions. None of that is asserted here.
//!
//! | Story AC | Test |
//! |---|---|
//! | AC-011 | `..._15bc_full_run_log_round_trips_through_read_log_with_l1_to_l4_for_absent_and_present_canonical_targets` |
//! | AC-011 | `..._15c_done_copies_the_four_intent_fields` |
//!
//! A real-binary, module-independent twin of both (so the property also fails by ASSERTION
//! today) is `..._15c_full_run_log_is_a_pure_record_sequence_and_every_done_copies_its_intent`
//! in `s2510_coordinator_blackbox_test.rs`.
//!
//! The module under test does not exist yet: this file fails to COMPILE (the accepted Red).

#[path = "s2510_support/api.rs"]
mod api;
#[path = "s2510_support/mod.rs"]
mod support;

use std::collections::BTreeMap;
use std::path::PathBuf;

use api::{IntentRecord, RecordType, read_log};
use factory_dispatcher::shard_manager::{BcIndexMigrationOutcome, run_bc_index_migration};
use support::{ORIGINAL_CONTENT, read_txn, setup_fresh_fixture, sha256_hex};

/// In a failpoints build, serialize against the process-global fail registry.
#[cfg(feature = "failpoints")]
fn fail_point_scope() -> fail::FailScenario<'static> {
    fail::FailScenario::setup()
}

struct Run {
    _dir: tempfile::TempDir,
    activation_id: String,
    fencing: u64,
    log_bytes: Vec<u8>,
    _log: PathBuf,
}

fn full_run() -> Run {
    let dir = tempfile::tempdir().unwrap();
    setup_fresh_fixture(dir.path());
    let outcome = run_bc_index_migration(dir.path());
    assert!(
        matches!(
            outcome,
            Ok(BcIndexMigrationOutcome::Completed {
                canonical_paths_count: 4
            })
        ),
        "a fresh run over the two-subsystem fixture completes 4 moves, got {outcome:?}"
    );
    let ms = dir.path().join(".factory/migration-state");
    let txn = read_txn(&ms);
    let gen_id = txn["generation_id"].as_str().unwrap().to_string();
    let log = ms.join(format!("intent-{gen_id}.log"));
    let log_bytes = std::fs::read(&log).expect("the coordinator's intent log");
    Run {
        activation_id: txn["activation_id"].as_str().unwrap().to_string(),
        fencing: txn["fencing_generation"].as_u64().unwrap(),
        log_bytes,
        _log: log,
        _dir: dir,
    }
}

/// AC-011: `read_log` over the coordinator's own log returns EVERY record -- four INTENT
/// then four DONE -- with no corruption and no invariant violation; the INTENT batch covers
/// both an ABSENT canonical target (`expected_pre_state` = MISSING) and the PRESENT
/// `BC-INDEX.md` (`expected_pre_state` = `sha256(original body)`).
#[test]
fn test_BC_1_18_011_15bc_full_run_log_round_trips_through_read_log_with_l1_to_l4_for_absent_and_present_canonical_targets()
 {
    #[cfg(feature = "failpoints")]
    let _fp = fail_point_scope();
    let run = full_run();
    let lr = read_log(&run.log_bytes, &run.activation_id).unwrap_or_else(|e| {
        panic!(
            "the converted coordinator must not reject its OWN log (L1-L4 / grammar / checksum): \
             {e:?}"
        )
    });
    assert_eq!(
        lr.valid_prefix_len,
        run.log_bytes.len(),
        "no torn tail, no trailing bytes"
    );
    let intents: Vec<&IntentRecord> = lr
        .records
        .iter()
        .filter(|r| r.record_type == RecordType::Intent)
        .collect();
    let dones: Vec<&IntentRecord> = lr
        .records
        .iter()
        .filter(|r| r.record_type == RecordType::Done)
        .collect();
    assert_eq!(intents.len(), 4, "one INTENT per move");
    assert_eq!(dones.len(), 4, "one DONE per move");
    assert_eq!(lr.records.len(), 8);

    let missing = intents
        .iter()
        .filter(|r| r.expected_pre_state.is_none())
        .count();
    let present: Vec<&&IntentRecord> = intents
        .iter()
        .filter(|r| r.expected_pre_state.is_some())
        .collect();
    assert_eq!(
        missing, 3,
        "the SS-01 shard, the SS-02 shard and the manifest were ABSENT at INTENT time"
    );
    assert_eq!(
        present.len(),
        1,
        "exactly BC-INDEX.md was PRESENT at INTENT time"
    );
    assert!(present[0].target_canonical.ends_with("BC-INDEX.md"));
    assert_eq!(
        present[0].expected_pre_state.as_deref(),
        Some(sha256_hex(ORIGINAL_CONTENT.as_bytes()).as_str()),
        "the PRESENT target's pre-state is recorded truthfully at INTENT time"
    );
    for r in &lr.records {
        assert_eq!(
            r.txn_id, run.activation_id,
            "every record carries the live txn id (S-25.09: txn_id == activation_id)"
        );
        assert_eq!(
            r.fencing_generation, run.fencing,
            "fresh run: the current fencing generation"
        );
    }
}

/// AC-011 / Decision 3 B-2 (c) copy rule: every `DONE` copies `target_canonical`,
/// `staging_path`, `expected_post_hash` and `expected_pre_state` from the INTENT it
/// completes (for PRESENT as well as ABSENT targets), carries the live txn's id and the
/// current fencing generation, and is not lower than its INTENT.
#[test]
fn test_BC_1_18_011_15c_done_copies_the_four_intent_fields() {
    #[cfg(feature = "failpoints")]
    let _fp = fail_point_scope();
    let run = full_run();
    let lr = read_log(&run.log_bytes, &run.activation_id).expect("own log reads");
    let mut latest_intent: BTreeMap<PathBuf, &IntentRecord> = BTreeMap::new();
    for r in &lr.records {
        if r.record_type == RecordType::Intent {
            latest_intent.insert(r.target_canonical.clone(), r);
        }
    }
    let mut checked = 0;
    for done in lr
        .records
        .iter()
        .filter(|r| r.record_type == RecordType::Done)
    {
        let intent = latest_intent
            .get(&done.target_canonical)
            .unwrap_or_else(|| {
                panic!("DONE for {} has no INTENT", done.target_canonical.display())
            });
        assert_eq!(done.target_canonical, intent.target_canonical);
        assert_eq!(
            done.staging_path, intent.staging_path,
            "staging_path copied"
        );
        assert_eq!(
            done.expected_post_hash, intent.expected_post_hash,
            "expected_post_hash copied, not recomputed"
        );
        assert_eq!(
            done.expected_pre_state, intent.expected_pre_state,
            "expected_pre_state copied (was hard-coded MISSING)"
        );
        assert_eq!(done.txn_id, run.activation_id);
        assert_eq!(done.fencing_generation, run.fencing);
        assert!(done.fencing_generation >= intent.fencing_generation);
        checked += 1;
    }
    assert_eq!(checked, 4);
}
