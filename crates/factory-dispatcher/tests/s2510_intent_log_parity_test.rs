// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-25.10 RED tests -- the PARITY contract for S-25.06 (red test T11, writer-bytes half).
//!
//! Authority: ADR-054 v1.0 Decision 4 (ONE module for both migrations; "contains no
//! migration-specific names"); BC-1.18.013 v1.13 Precondition 5 (shared module, N fixed at 4
//! for `backfill-append-logs`) and EC-050..EC-052; BC-1.18.011 v1.21 Postcondition 15(a);
//! story S-25.10 AC-009.
//!
//! | Story AC / red test | Test |
//! |---|---|
//! | AC-009 / T11 | `..._5_parity_both_parameter_sets_yield_identical_writer_bytes` |
//! | AC-009 / T11 | `..._5_parity_the_same_logical_inputs_differ_only_in_path_and_checksum_lines` |
//! | AC-009 / AC-001 | `..._5_parity_module_source_names_no_migration_specific_type_or_subcommand` |
//!
//! The fixture is `s2510_support/parity.rs` (importable by S-25.06). The assertion that the
//! `DONE` FIELD POPULATION (copy from the INTENT, live txn parameters) is identical for both
//! parameter sets is S-25.11 AC-005's, added to the same fixture once that logic exists.
//!
//! The module under test does not exist yet: this file fails to COMPILE (the accepted Red).

#[path = "s2510_support/api.rs"]
mod api;
#[path = "s2510_support/parity.rs"]
mod parity;
#[path = "s2510_support/mod.rs"]
mod support;

use api::{IntentLogWriter, encode_record, read_log, to_intent, to_intents};
use factory_dispatcher::shard_manager::BcIndexMigrationError;
use factory_dispatcher::shard_manager::migration_fs::StdFs;
use support::{assert_no_failures, concat, encode};

/// Write a set's full run through the module the way the coordinators do: ONE batch for all
/// INTENTs, then one `DONE` per move.
fn write_run(set: &parity::ParitySet, log: &std::path::Path) -> Result<(), BcIndexMigrationError> {
    let recs = set.records();
    let n = set.n();
    let mut w = IntentLogWriter::open(&StdFs, log, &set.txn_id)?;
    w.append_batch(&to_intents(&recs[..n]))?;
    for r in &recs[n..] {
        w.append_batch(std::slice::from_ref(&to_intent(r)))?;
    }
    Ok(())
}

/// BC-1.18.013 Precondition 5 / EC-050..EC-052; ADR-054 Decision 4: the SAME module yields
/// the SAME writer bytes for the `migrate-bc-index` set (N = staged shards = 5) and the
/// `backfill-append-logs` set (N = 4): each log is byte-identical to the independent
/// reference encoding of the logical records, reads back in full with L1-L4 holding, and
/// its INTENT count equals N (the number of distinct targets).
#[test]
fn test_BC_1_18_013_5_parity_both_parameter_sets_yield_identical_writer_bytes() {
    let mut failures = Vec::new();
    for set in parity::sets() {
        let recs = set.records();
        let want = concat(&recs.iter().map(encode).collect::<Vec<_>>());
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("intent-gen.log");
        if let Err(e) = write_run(&set, &log) {
            failures.push(format!("[{}] writer failed: {e}", set.name));
            continue;
        }
        let got = std::fs::read(&log).unwrap();
        if got != want {
            failures.push(format!(
                "[{}] writer bytes differ from the reference encoding ({} vs {} bytes)",
                set.name,
                got.len(),
                want.len()
            ));
        }
        match read_log(&got, &set.txn_id) {
            Ok(lr) => {
                if lr.records != to_intents(&recs) || lr.valid_prefix_len != got.len() {
                    failures.push(format!("[{}] the log must read back in full", set.name));
                }
                let intents = lr
                    .records
                    .iter()
                    .filter(|r| r.record_type == api::RecordType::Intent)
                    .count();
                if intents != set.n() {
                    failures.push(format!(
                        "[{}] INTENT count {intents} != N {}",
                        set.name,
                        set.n()
                    ));
                }
            }
            Err(e) => failures.push(format!("[{}] read_log failed: {e:?}", set.name)),
        }
    }
    let sets = parity::sets();
    if sets[0].n() != 5 || sets[1].n() != 4 {
        failures.push(
            "fixture sanity: N is 5 for migrate-bc-index and the fixed 4 for backfill-append-logs"
                .into(),
        );
    }
    assert_no_failures("parity writer bytes", failures);
}

/// The module treats the two parameter sets uniformly: for the SAME logical INTENT/DONE
/// inputs (same hashes, timestamps, fencing, and -- here -- the same txn id) the encoded
/// bytes of the two sets differ ONLY in the target, staging and checksum lines (the
/// checksum covers the path lines), line for line.
#[test]
fn test_BC_1_18_013_5_parity_the_same_logical_inputs_differ_only_in_path_and_checksum_lines() {
    let a = parity::migrate_bc_index_set();
    let mut b = parity::backfill_append_logs_set();
    b.txn_id = a.txn_id.clone(); // identical logical inputs apart from the targets
    let (ra, rb) = (a.records(), b.records());
    let mut failures = Vec::new();
    // Compare the first INTENT..DONE pair index-by-index over the 4 indices both sets have:
    // INTENT i is at i, DONE i is at N + i.
    for i in 0..4 {
        for (kind, ia, ib) in [("INTENT", i, i), ("DONE", a.n() + i, b.n() + i)] {
            let (Ok(ea), Ok(eb)) = (
                encode_record(&to_intent(&ra[ia])),
                encode_record(&to_intent(&rb[ib])),
            ) else {
                failures.push(format!(
                    "{kind} {i}: both records are valid and must encode"
                ));
                continue;
            };
            let (ta, tb) = (
                String::from_utf8(ea).unwrap(),
                String::from_utf8(eb).unwrap(),
            );
            let (la, lb): (Vec<&str>, Vec<&str>) = (ta.lines().collect(), tb.lines().collect());
            if la.len() != 11 || lb.len() != 11 {
                failures.push(format!("{kind} {i}: both must be 11 lines"));
                continue;
            }
            for (n, (x, y)) in la.iter().zip(&lb).enumerate() {
                // zero-based lines 4, 5 and 9 are target_canonical, staging_path and
                // record_checksum (the checksum covers the path lines).
                let must_differ = matches!(n, 4 | 5 | 9);
                if must_differ != (x != y) {
                    failures.push(format!(
                        "{kind} {i}: line {} must {} between the two parameter sets: {x:?} vs {y:?}",
                        n + 1,
                        if must_differ { "differ" } else { "be identical" }
                    ));
                }
            }
        }
    }
    assert_no_failures("parity line-level uniformity", failures);
}

/// ADR-054 Decision 4 / "Forbidden Dependencies": `shard_manager/intent_log.rs` contains no
/// migration-specific name -- neither subcommand string, nor a txn-record type, nor the
/// plan field, nor a diagnostics/executor dependency. (`BcIndexMigrationError` is the `Fs`
/// seam's error type and necessarily appears in signatures; it is allowed.)
#[test]
fn test_BC_1_18_013_5_parity_module_source_names_no_migration_specific_type_or_subcommand() {
    let src_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shard_manager/intent_log.rs");
    let src = std::fs::read_to_string(&src_path)
        .unwrap_or_else(|e| panic!("ADR-054 Decision 4: {} must exist: {e}", src_path.display()));
    // Strip line comments so prose cannot hide or cause a hit.
    let code: String = src
        .lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    let forbidden = [
        "migrate-bc-index",
        "backfill-append-logs",
        "BcIndexMigrationTxn",
        "PendingCanonicalMove",
        "PlannedCanonicalMove",
        "pending_canonical_moves",
        "canonical_move_plan",
        "internal_log",
        "executor",
    ];
    let hits: Vec<&str> = forbidden
        .iter()
        .copied()
        .filter(|t| code.contains(t))
        .collect();
    assert!(
        hits.is_empty(),
        "intent_log.rs must contain no migration-specific name (ADR-054 Decision 4); found: {hits:?}"
    );
    // Sanity that the gate scanned a real module: it owns the format and the writer.
    for needle in [
        "INTENT_LOG_RECORD_V1",
        "END_INTENT_LOG_RECORD",
        "IntentLogWriter",
        "read_log",
    ] {
        assert!(
            src.contains(needle),
            "intent_log.rs must define/own `{needle}`"
        );
    }
}
