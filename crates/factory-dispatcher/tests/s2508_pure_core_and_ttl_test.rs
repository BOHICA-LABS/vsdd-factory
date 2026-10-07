// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-25.08 (T-1 / T-2) RED-GATE tests for the PURE decision cores and the
//! reservation TTL (AC-007 pure core, AC-008, AC-010 core semantics).
//!
//! # BC-5.38.001 Red Gate -- STUB SURFACE this file compiles against
//!
//! All four symbols below were added by the test-writer to
//! `shard_manager.rs` as minimal compilable stubs with `todo!()` bodies (names
//! and shapes are pinned by VP-146 v1.2 / VP-147 v1.1 / ADR-052 v1.18 §5a):
//!
//! * `decide_terminal_record_reconciliation(&TerminalReconcileInputs) -> TerminalReconcileDecision`
//! * `reservation_is_stale(created_at: Option<u64>, mtime: u64, now: u64, ttl: u64) -> bool`
//!   (epoch seconds; `created_at = None` models absent/unparseable)
//! * `validate_production_reservation_ttl(Duration) -> Result<Duration, BcIndexMigrationError>`
//!   and `MIN_PRODUCTION_RESERVATION_TTL` (1,800 s) -- the PRODUCTION entry
//!   point's TTL floor (the injectable test seam `drain_bc_index_writers` is
//!   NOT bound by the floor). The validator's NAME is the test-writer's
//!   proposal (the spec names the behavior, not the symbol); the implementer
//!   may rename it IF the production entry point is kept a distinct, testable
//!   function.
//!
//! The existing `DEFAULT_MAX_RESERVATION_TTL` is left at its merged 120 s
//! value (that is defect B2-3).
//!
//! # Traceability
//!
//! | Test | AC | BC clause | VP |
//! |------|----|-----------|----|
//! | `test_BC_1_18_011_EC009_production_ttl_constant_floor_and_created_at_judgment` | AC-008 | BC-1.18.011 Pre 6(c) TTL, EC-009; BC-1.18.013 Pre 6(c), EC-019 | VP-133 f7 |
//! | `test_BC_1_18_011_EC009_reservation_is_stale_*` | AC-008 | same | VP-133 f7, VP-146 a4 / VP-147 h4 (S8) |
//! | `test_BC_1_18_011_PC9_terminal_reconcile_*` | AC-007 / AC-010 | BC-1.18.011 Postcondition 9, EC-011..EC-013; BC-1.18.013 Post 5a, EC-009/010/016 | VP-147 h1 totality, VP-146 a1 |

use std::path::Path;
use std::time::Duration;

use factory_dispatcher::shard_manager::{
    BcIndexMigrationError, BcIndexMigrationTxnState, DEFAULT_MAX_RESERVATION_TTL,
    MIN_PRODUCTION_RESERVATION_TTL, TerminalReconcileDecision, TerminalReconcileInputs,
    decide_terminal_record_reconciliation, drain_bc_index_writers, reservation_is_stale,
    validate_production_reservation_ttl,
};

// ---------------------------------------------------------------------------
// AC-008 -- TTL constant, production floor, created_at-first staleness
// ---------------------------------------------------------------------------

fn write_reservation(dir: &Path, id: &str, created_at: Option<&str>, mtime_age_secs: u64) {
    std::fs::create_dir_all(dir).unwrap();
    let body = match created_at {
        Some(c) => format!(r#"{{"created_at":"{c}","tool_use_id":"{id}"}}"#),
        None => format!(r#"{{"tool_use_id":"{id}"}}"#),
    };
    let path = dir.join(format!("{id}.reservation"));
    std::fs::write(&path, body).unwrap();
    let mtime = std::time::SystemTime::now() - Duration::from_secs(mtime_age_secs);
    filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(mtime)).unwrap();
}

fn iso_ago(secs: i64) -> String {
    (chrono::Utc::now() - chrono::Duration::seconds(secs))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// AC-008 (BC-1.18.011 Precondition 6(c) "TTL", EC-009; BC-1.18.013 Precondition 6(c), EC-019;
/// VP-133 facet 7). The TTL and the drain timeout are the INJECTABLE seams
/// (`drain_bc_index_writers(dir, drain_timeout, ttl)`); no test sleeps beyond a
/// 150 ms drain timeout.
#[test]
fn test_BC_1_18_011_EC009_production_ttl_constant_floor_and_created_at_judgment() {
    let mut failures: Vec<String> = Vec::new();

    // (a) static production constants.
    if DEFAULT_MAX_RESERVATION_TTL != Duration::from_secs(3600) {
        failures.push(format!(
            "DEFAULT_MAX_RESERVATION_TTL must be 3,600 s (merged value 120 s violates the \
             1,800 s floor), got {:?}",
            DEFAULT_MAX_RESERVATION_TTL
        ));
    }
    if MIN_PRODUCTION_RESERVATION_TTL != Duration::from_secs(1800) {
        failures.push(format!(
            "production floor must be 1,800 s, got {MIN_PRODUCTION_RESERVATION_TTL:?}"
        ));
    }

    // (b) production entry point rejects < 1,800 s as a configuration error and
    // accepts >= 1,800 s unchanged. (todo!() panics are caught per vector so every
    // vector is reported.)
    for secs in [0u64, 1, 120, 1799] {
        let r = std::panic::catch_unwind(|| {
            validate_production_reservation_ttl(Duration::from_secs(secs))
        });
        match r {
            Ok(Err(_)) => {}
            Ok(Ok(v)) => failures.push(format!(
                "production TTL {secs}s must be REJECTED, accepted as {v:?}"
            )),
            Err(_) => failures.push(format!(
                "production TTL validator panicked (unimplemented) for {secs}s"
            )),
        }
    }
    for secs in [1800u64, 3600, 86_400] {
        let r = std::panic::catch_unwind(|| {
            validate_production_reservation_ttl(Duration::from_secs(secs))
        });
        match r {
            Ok(Ok(v)) if v == Duration::from_secs(secs) => {}
            Ok(other) => failures.push(format!(
                "production TTL {secs}s must be accepted unchanged, got {other:?}"
            )),
            Err(_) => failures.push(format!(
                "production TTL validator panicked (unimplemented) for {secs}s"
            )),
        }
    }

    // (c) created_at-first staleness through the REAL drain GC (TTL seam = 3,600 s).
    // (label, created_at, mtime_age_secs, expect_reclaimed)
    let vectors: [(&str, Option<String>, u64, bool); 6] = [
        // BC canonical vector row 1: created_at = now-4000 => reclaimed (mtime FRESH).
        (
            "created_at old, mtime fresh => reclaimed",
            Some(iso_ago(4000)),
            0,
            true,
        ),
        // row 2: no created_at, mtime = now-4000 => reclaimed (mtime fallback).
        (
            "created_at absent, mtime old => reclaimed",
            None,
            4000,
            true,
        ),
        // row 3: created_at = now => NOT reclaimed.
        (
            "created_at now, mtime fresh => kept",
            Some(iso_ago(0)),
            0,
            false,
        ),
        // created_at WINS over mtime: fresh created_at + ancient mtime => kept.
        (
            "created_at now, mtime OLD => kept (created_at wins)",
            Some(iso_ago(0)),
            4000,
            false,
        ),
        // unparseable created_at => mtime fallback.
        (
            "created_at unparseable, mtime old => reclaimed",
            Some("not-a-timestamp".to_string()),
            4000,
            true,
        ),
        (
            "created_at unparseable, mtime fresh => kept",
            Some("not-a-timestamp".to_string()),
            0,
            false,
        ),
    ];
    for (label, created_at, mtime_age, expect_reclaimed) in vectors {
        let dir = tempfile::tempdir().unwrap();
        let res_dir = dir.path().join("reservations");
        write_reservation(&res_dir, "T1", created_at.as_deref(), mtime_age);
        let r = drain_bc_index_writers(
            &res_dir,
            Duration::from_millis(150),
            Duration::from_secs(3600),
        );
        let still_there = res_dir.join("T1.reservation").exists();
        let ok = if expect_reclaimed {
            r.is_ok() && !still_there
        } else {
            matches!(r, Err(BcIndexMigrationError::DrainTimeoutAbort)) && still_there
        };
        if !ok {
            failures.push(format!(
                "[{label}] expected reclaimed={expect_reclaimed}; drain result={r:?}, \
                 reservation_still_present={still_there}"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "AC-008: {} failure(s):\n  - {}",
        failures.len(),
        failures.join("\n  - ")
    );
}

/// AC-008: the single pure staleness predicate (no PID input): created_at wins,
/// mtime ONLY when created_at is absent/unparseable; strict `>` boundary.
#[test]
fn test_BC_1_18_011_EC009_reservation_is_stale_created_at_first_mtime_fallback() {
    let now = 1_000_000u64;
    let ttl = 3600u64;
    // created_at-first (mtime ignored when created_at is Some).
    assert!(
        reservation_is_stale(Some(now - 4000), now, now, ttl),
        "created_at old => stale even with fresh mtime"
    );
    assert!(
        !reservation_is_stale(Some(now), now - 4000, now, ttl),
        "created_at fresh => NOT stale even with old mtime"
    );
    // mtime fallback only when created_at is None.
    assert!(
        reservation_is_stale(None, now - 4000, now, ttl),
        "no created_at + old mtime => stale"
    );
    assert!(
        !reservation_is_stale(None, now, now, ttl),
        "no created_at + fresh mtime => not stale"
    );
    // strict boundary: age == ttl is NOT stale; ttl + 1 IS.
    assert!(
        !reservation_is_stale(Some(now - ttl), 0, now, ttl),
        "age == ttl is not stale (strict >)"
    );
    assert!(
        reservation_is_stale(Some(now - ttl - 1), now, now, ttl),
        "age == ttl + 1 is stale"
    );
    // a created_at in the future never underflows / is never stale.
    assert!(
        !reservation_is_stale(Some(now + 500), 0, now, ttl),
        "future created_at must not be stale"
    );
    assert!(
        !reservation_is_stale(None, now + 500, now, ttl),
        "future mtime must not be stale"
    );
}

// ---------------------------------------------------------------------------
// AC-007 / AC-010 -- the ONE pure terminal-record reconciliation core
// ---------------------------------------------------------------------------

const STATES: [Option<BcIndexMigrationTxnState>; 5] = [
    None,
    Some(BcIndexMigrationTxnState::Staging),
    Some(BcIndexMigrationTxnState::Committing),
    Some(BcIndexMigrationTxnState::Completed),
    Some(BcIndexMigrationTxnState::Aborted),
];

fn all_inputs() -> Vec<TerminalReconcileInputs> {
    let mut v = Vec::new();
    for lock_acquired in [false, true] {
        for record_present in [false, true] {
            for txn_state in STATES {
                for bits in 0u32..(1 << 4) {
                    for hbits in 0u32..(1 << 4) {
                        for own in [false, true] {
                            v.push(TerminalReconcileInputs {
                                lock_acquired,
                                record_present,
                                txn_state,
                                record_parses: bits & 1 != 0,
                                txn_id_eq: bits & 2 != 0,
                                generation_id_eq: bits & 4 != 0,
                                count_eq_n: bits & 8 != 0,
                                hashes_eq: [
                                    hbits & 1 != 0,
                                    hbits & 2 != 0,
                                    hbits & 4 != 0,
                                    hbits & 8 != 0,
                                ],
                                txn_is_own_migration: own,
                            });
                        }
                    }
                }
            }
        }
    }
    v
}

fn live(s: Option<BcIndexMigrationTxnState>) -> bool {
    matches!(
        s,
        Some(BcIndexMigrationTxnState::Staging | BcIndexMigrationTxnState::Committing)
    )
}

/// The decision table from VP-146 v1.2 a1 / VP-147 v1.1 h1, stated independently
/// of any implementation (the oracle).
fn oracle(i: &TerminalReconcileInputs) -> TerminalReconcileDecision {
    use TerminalReconcileDecision::*;
    if !i.lock_acquired || !live(i.txn_state) {
        return NoOp; // live coordinator, or gate-only repair (Branch A) -- not this core
    }
    if !i.txn_is_own_migration {
        return RefuseForeignMigration; // precedence over every record check
    }
    if !i.record_present {
        return NoOp; // record ABSENT => nothing to reconcile (BC-1.18.011 v1.13 Pre 6(d) table row 3)
    }
    let all_ok = i.record_parses
        && i.txn_id_eq
        && i.generation_id_eq
        && i.count_eq_n
        && i.hashes_eq.iter().all(|b| *b);
    if i.txn_state == Some(BcIndexMigrationTxnState::Committing) && all_ok {
        FinalizeThenOpenGate
    } else {
        FailClosedMismatch // includes STAGING + record: ALWAYS fail-closed, no verification
    }
}

/// AC-007 / AC-010 (BC-1.18.011 Postcondition 9; BC-1.18.013 Postcondition 5a;
/// VP-147 h1 totality): over the full bounded product (10,240 inputs)
/// the core is total (never panics) and equals the decision table.
#[test]
fn test_BC_1_18_011_PC9_terminal_reconcile_core_total_and_matches_decision_table() {
    let mut mismatches = Vec::new();
    let mut count = 0usize;
    for i in all_inputs() {
        count += 1;
        let got = std::panic::catch_unwind(|| decide_terminal_record_reconciliation(&i));
        match got {
            Ok(d) if d == oracle(&i) => {}
            Ok(d) => mismatches.push(format!("{i:?} => {d:?}, expected {:?}", oracle(&i))),
            Err(_) => mismatches.push(format!("{i:?} => PANIC (core must be total)")),
        }
        if mismatches.len() >= 8 {
            break;
        }
    }
    assert!(
        mismatches.is_empty(),
        "decision core diverges from the VP-146/VP-147 decision table over {count}+ inputs \
         (first {} shown):\n  - {}",
        mismatches.len(),
        mismatches.join("\n  - ")
    );
}

/// Foreign-migration guard (BC-1.18.011 EC-013 / BC-1.18.013 EC-016; VP-146 a1):
/// RefuseForeignMigration for EVERY lock_acquired + live + not-own input, never
/// Finalize / FailClosedMismatch (label discipline: a foreign txn is refused, not
/// a "mismatch").
#[test]
fn test_BC_1_18_011_PC9_terminal_reconcile_foreign_txn_is_refused_never_finalized_or_mismatch() {
    for i in all_inputs() {
        if i.lock_acquired && live(i.txn_state) && !i.txn_is_own_migration {
            let d = decide_terminal_record_reconciliation(&i);
            assert_eq!(
                d,
                TerminalReconcileDecision::RefuseForeignMigration,
                "foreign live txn must be refused over EVERY record check: {i:?} => {d:?}"
            );
        }
    }
}

/// STAGING + terminal record is ALWAYS fail-closed (no verification attempted),
/// even when every record check is true (BC-1.18.011 Precondition 6(d)/Postcondition
/// 9(d); BC-1.18.013 Postcondition 5a; taxonomy v1.34 clause (c)).
#[test]
fn test_BC_1_18_011_PC9_terminal_reconcile_staging_plus_record_always_fail_closed() {
    let i = TerminalReconcileInputs {
        lock_acquired: true,
        record_present: true,
        txn_state: Some(BcIndexMigrationTxnState::Staging),
        record_parses: true,
        txn_id_eq: true,
        generation_id_eq: true,
        count_eq_n: true,
        hashes_eq: [true; 4],
        txn_is_own_migration: true,
    };
    assert_eq!(
        decide_terminal_record_reconciliation(&i),
        TerminalReconcileDecision::FailClosedMismatch,
        "STAGING + a fully-verifying terminal record must STILL fail closed (never Finalize)"
    );
}

/// FinalizeThenOpenGate ONLY on lock + own + record + COMMITTING + EVERY check
/// (BC-1.18.011 EC-011; VP-146 a1): flipping any single check false must demote it
/// to FailClosedMismatch.
#[test]
fn test_BC_1_18_011_PC9_terminal_reconcile_finalize_only_on_full_verification() {
    let full = TerminalReconcileInputs {
        lock_acquired: true,
        record_present: true,
        txn_state: Some(BcIndexMigrationTxnState::Committing),
        record_parses: true,
        txn_id_eq: true,
        generation_id_eq: true,
        count_eq_n: true,
        hashes_eq: [true; 4],
        txn_is_own_migration: true,
    };
    assert_eq!(
        decide_terminal_record_reconciliation(&full),
        TerminalReconcileDecision::FinalizeThenOpenGate,
        "a fully verifying own COMMITTING txn must finalize-then-open-gate"
    );
    let single_flips: Vec<(&str, TerminalReconcileInputs)> = vec![
        (
            "record_parses",
            TerminalReconcileInputs {
                record_parses: false,
                ..full
            },
        ),
        (
            "txn_id_eq",
            TerminalReconcileInputs {
                txn_id_eq: false,
                ..full
            },
        ),
        (
            "generation_id_eq",
            TerminalReconcileInputs {
                generation_id_eq: false,
                ..full
            },
        ),
        (
            "count_eq_n",
            TerminalReconcileInputs {
                count_eq_n: false,
                ..full
            },
        ),
        (
            "hashes_eq[0]",
            TerminalReconcileInputs {
                hashes_eq: [false, true, true, true],
                ..full
            },
        ),
        (
            "hashes_eq[3]",
            TerminalReconcileInputs {
                hashes_eq: [true, true, true, false],
                ..full
            },
        ),
    ];
    for (name, i) in single_flips {
        assert_eq!(
            decide_terminal_record_reconciliation(&i),
            TerminalReconcileDecision::FailClosedMismatch,
            "a single failed check ({name}) must demote Finalize to FailClosedMismatch"
        );
    }
    // a live coordinator (lock not acquired) never causes any action.
    let no_lock = TerminalReconcileInputs {
        lock_acquired: false,
        ..full
    };
    assert_eq!(
        decide_terminal_record_reconciliation(&no_lock),
        TerminalReconcileDecision::NoOp,
        "lock not acquired (live coordinator) => NoOp"
    );
}

/// BC-1.18.011 v1.13 NoOp cell: own-migration live txn (STAGING and, separately,
/// COMMITTING), lock acquired, terminal record ABSENT => `NoOp` (no txn write, no
/// gate write).
#[test]
fn test_BC_1_18_011_PC9_terminal_reconcile_record_absent_is_noop_for_staging_and_committing() {
    for state in [
        BcIndexMigrationTxnState::Staging,
        BcIndexMigrationTxnState::Committing,
    ] {
        // every other check input true AND false: record absence alone decides.
        for others in [true, false] {
            let i = TerminalReconcileInputs {
                lock_acquired: true,
                record_present: false,
                txn_state: Some(state),
                record_parses: others,
                txn_id_eq: others,
                generation_id_eq: others,
                count_eq_n: others,
                hashes_eq: [others; 4],
                txn_is_own_migration: true,
            };
            assert_eq!(
                decide_terminal_record_reconciliation(&i),
                TerminalReconcileDecision::NoOp,
                "own live txn {state:?} with NO terminal record must be NoOp ({i:?})"
            );
        }
    }
}

/// BC-1.18.011 v1.13 Pre 6(d) row 3: Branch B has NO gate condition. With the
/// gate OPEN (an absent gate-state.json reads as OPEN), an own live
/// null-generation STAGING txn, lock acquired and terminal record absent must
/// still plan the Branch B abort-then-reopen; a lock-held (live coordinator)
/// control plans no action.
#[test]
fn test_BC_1_18_011_PC6d_plan_branch_b_applies_with_gate_open() {
    use factory_dispatcher::shard_manager::{
        BcIndexAdmissionGateState, StaleGateReconciliationPlan, plan_stale_gate_reconciliation,
    };
    let inputs = TerminalReconcileInputs {
        lock_acquired: true,
        record_present: false,
        txn_state: Some(BcIndexMigrationTxnState::Staging),
        record_parses: false,
        txn_id_eq: false,
        generation_id_eq: false,
        count_eq_n: false,
        hashes_eq: [false; 4],
        txn_is_own_migration: true,
    };
    for gate in [
        BcIndexAdmissionGateState::Open,
        BcIndexAdmissionGateState::Draining,
        BcIndexAdmissionGateState::Locked,
    ] {
        assert_eq!(
            plan_stale_gate_reconciliation(gate, &inputs, true),
            StaleGateReconciliationPlan::AbortNullGenerationThenReopenGate,
            "Branch B must apply for gate {gate:?} (no gate condition)"
        );
    }
    // generation assigned => not Branch B, regardless of gate.
    assert_eq!(
        plan_stale_gate_reconciliation(BcIndexAdmissionGateState::Open, &inputs, false),
        StaleGateReconciliationPlan::NothingToReconcile
    );
    // live coordinator => no action.
    let held = TerminalReconcileInputs {
        lock_acquired: false,
        ..inputs
    };
    assert_eq!(
        plan_stale_gate_reconciliation(BcIndexAdmissionGateState::Open, &held, true),
        StaleGateReconciliationPlan::LiveCoordinator
    );
}

/// F-S2508-L1-005 (BC-1.18.011 Precondition 6(c): the coordinator waits for ALL
/// in-flight admitted writers; since v1.7 `reservations/` exists before draining,
/// so every read error is genuine): an UNREADABLE `reservations/` holding a live
/// writer reservation must NOT be treated as quiescent. The drain must fail
/// closed (not `Ok`), so the coordinator never proceeds to LOCKED/snapshot.
/// Skipped (loudly) only as root, where chmod 000 does not restrict reads.
#[cfg(unix)]
#[test]
fn test_BC_1_18_011_PC6c_drain_fails_closed_on_reservations_read_error() {
    use std::os::unix::fs::PermissionsExt;

    let uid = std::process::Command::new("id")
        .arg("-u")
        .stdin(std::process::Stdio::null())
        .output()
        .expect("id -u");
    if String::from_utf8_lossy(&uid.stdout).trim() == "0" {
        eprintln!(
            "SKIP: running as root; chmod 000 does not restrict reads, so the read-error path is unobservable"
        );
        return;
    }

    struct Restore(std::path::PathBuf);
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
        }
    }

    let dir = tempfile::tempdir().unwrap();
    let ms = dir.path().join("migration-state");
    let res_dir = ms.join("reservations");
    // A live (fresh) admitted-writer reservation.
    write_reservation(&res_dir, "T1", Some(&iso_ago(0)), 0);
    // The coordinator has flipped DRAINING; the drain must never lead to LOCKED.
    std::fs::write(ms.join("gate-state.json"), "\"DRAINING\"").unwrap();
    let _restore = Restore(res_dir.clone());
    std::fs::set_permissions(&res_dir, std::fs::Permissions::from_mode(0o000)).unwrap();

    let r = drain_bc_index_writers(
        &res_dir,
        Duration::from_millis(150),
        Duration::from_secs(3600),
    );

    assert!(
        r.is_err(),
        "an unreadable reservations/ with a live writer present must NOT be reported \
         quiescent (drain returned {r:?}) -- the coordinator would proceed to LOCKED + \
         snapshot while an admitted writer may be in flight"
    );
    // The drain itself never advances the gate, and the live reservation survives.
    assert_eq!(
        std::fs::read_to_string(ms.join("gate-state.json")).unwrap(),
        "\"DRAINING\"",
        "gate must never leave DRAINING (never LOCKED) on a failed drain"
    );
    std::fs::set_permissions(&res_dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        res_dir.join("T1.reservation").exists(),
        "the live writer's reservation must be untouched"
    );
}
