//! OBL-1 (D-1232-OBL-1) Kani model-checking harnesses for the B2 BC-INDEX
//! shard-migration crash-recovery state machine (BC-1.18.011; ADR-052
//! §Decision 4e/5a/7a/7b/7c).
//!
//! # What this module is
//!
//! Ten `#[kani::proof]` harnesses (VP-147). The original seven cover
//! research report §5a items 1-6 (item 3 — state-machine totality +
//! inductive invariant — is split into an inductive-step proof and a
//! bounded-sequence proof), mapped onto the OBL-1 refactor design §6.2. The
//! ADR-052 v1.18 re-baseline (S-25.08 AC-010) adds three over the SHARED
//! admission/reconciliation pure cores —
//! `proof_obl1_h1_terminal_reconcile_totality`
//! ([`decide_terminal_record_reconciliation`](super::decide_terminal_record_reconciliation)),
//! `proof_obl1_h4_reservation_quiescence_and_selfheal` (reserve-then-verify,
//! drain, TTL GC, release-on-block and step-3.5 Branches A/B/C via
//! [`plan_stale_gate_reconciliation`](super::plan_stale_gate_reconciliation)
//! and [`reservation_is_stale`](super::reservation_is_stale)) and
//! `proof_obl1_h6_terminal_reconcile_idempotence` — and gives the h3
//! transition relation the `FinalizeFromTerminalRecord` event. Every harness
//! carries `kani::cover!` non-vacuity witnesses that the CI job requires to
//! be SATISFIED. The harnesses model the REAL production functions; no
//! decision logic is re-implemented here. Also here: the in-memory two-namespace
//! [`Fs`](super::migration_fs::Fs) model that is their crash-atomicity
//! substrate (research §1c). The harnesses are gated behind `#[cfg(kani)]`
//! and are therefore compiled ONLY under `cargo kani` — a normal
//! `cargo build`/`cargo test`/`cargo clippy` never sees them (`cfg(kani)`
//! is the allowlisted expected-cfg in the workspace `Cargo.toml`
//! `[workspace.lints.rust.unexpected_cfgs]` block), so the normal build is
//! completely unaffected whether or not Kani is installed.
//!
//! # Solve-time note (`mem::forget` for the recover-based harnesses)
//!
//! The recover-based harnesses (h1 totality, h2 safety) construct
//! `String`-heavy [`super::BcIndexMigrationTxnRecord`] values. CBMC's
//! dominant cost for them is not `recover()` itself but the DROP GLUE at
//! scope end — unrolling the `Vec`/`String` destructors — which drove the
//! solve past 13 minutes (the same characteristic the pre-existing
//! `partition.rs`/`aggregator.rs` String-bearing harnesses exhibit;
//! research §1d). Each such harness therefore `std::mem::forget`s its
//! records/decision at the end: a Kani harness never really runs, so
//! skipping destructor modeling is sound and cuts the solve to well under a
//! minute. The state-machine (h3), admission-gate (h4) and crash-atomicity
//! (h5) harnesses use no heap-heavy owned values and are fast regardless.
//!
//! # What Kani proves here vs. what it cannot (research §1b)
//!
//! Kani "does not model I/O": it has no semantics for `rename(2)`,
//! `F_FULLFSYNC`, the page cache, torn sectors, or power loss. Every
//! harness therefore proves a property of the migration's *pure decision
//! logic* ([`recover`](super::recover),
//! [`classify_txn_records`](super::classify_txn_records),
//! [`is_bc_index_admission_open`](super::is_bc_index_admission_open)) or of
//! an *explicit abstract filesystem model* whose axioms
//! ([`InMemoryFs`]'s `crash`/`fsync_dir`/`rename` behaviour) are documented
//! below and are the separate refinement obligation discharged empirically
//! by the `fail`-based fault-injection integration suite (test-writer
//! scope). The defensible claim form is: *"under abstract filesystem model
//! M, every modeled op/crash sequence of length <= N preserves the
//! old-or-new invariant and recovery is total and fail-closed."*
//!
//! # Kani-generated inputs
//!
//! [`super::BcIndexMigrationTxnRecord`] contains `String`/`Vec` fields that
//! do not implement `kani::Arbitrary`, so — following this crate's existing
//! `partition.rs`/`aggregator.rs` harness convention — records are built
//! with EMPTY strings and `kani::any()` is used only for the
//! decision-relevant fields (`state`, `generation_id.is_some()`). This is
//! sound: [`recover`](super::recover)'s only branches are over exactly
//! those fields plus the four scalar/enum parameters, so varying them
//! covers the function's entire decision surface; the empty strings keep
//! CBMC's heap modeling of `recover()`'s internal `String` clones as cheap
//! as the type allows.

use std::cell::RefCell;
use std::path::Path;

use super::migration_fs::Fs;
use super::{
    BcIndexAdmissionGateState, BcIndexMigrationError, BcIndexMigrationTxnRecord,
    BcIndexMigrationTxnState, CompletedMigrationRecord, DEFAULT_MAX_RESERVATION_TTL,
    ManifestStatus, QuarantineReason, RESERVATION_CLOCK_SKEW_TOLERANCE_SECS, RecoveryDecision,
    StaleGateReconciliationPlan, TerminalReconcileDecision, TerminalReconcileInputs,
    decide_terminal_record_reconciliation, is_bc_index_admission_open,
    plan_stale_gate_reconciliation, recover, reservation_is_stale,
};

// ---------------------------------------------------------------------------
// Symbolic-value helpers over the migration's small finite domains.
// ---------------------------------------------------------------------------

/// A nondeterministic [`BcIndexMigrationTxnState`] covering all four
/// variants (research §5a-1: "arbitrary record-set shape"). Uses
/// `assume(v < 4)` rather than `any() % 4`: the modulo would force CBMC to
/// model all 256 `u8` values and is a known SAT-cost pitfall; the `assume`
/// constrains the symbolic value to exactly the 4 relevant cases.
fn any_txn_state() -> BcIndexMigrationTxnState {
    let v: u8 = kani::any();
    kani::assume(v < 4);
    match v {
        0 => BcIndexMigrationTxnState::Staging,
        1 => BcIndexMigrationTxnState::Committing,
        2 => BcIndexMigrationTxnState::Completed,
        _ => BcIndexMigrationTxnState::Aborted,
    }
}

/// A nondeterministic [`ManifestStatus`] covering all four variants (see
/// [`any_txn_state`] for the `assume(v < 4)` rationale).
fn any_manifest_status() -> ManifestStatus {
    let v: u8 = kani::any();
    kani::assume(v < 4);
    match v {
        0 => ManifestStatus::StillValid,
        1 => ManifestStatus::ExpiredOrAbsent,
        2 => ManifestStatus::CompletionOnly,
        _ => ManifestStatus::Unknown,
    }
}

/// A nondeterministic [`BcIndexAdmissionGateState`] (all three variants).
fn any_gate_state() -> BcIndexAdmissionGateState {
    let v: u8 = kani::any();
    kani::assume(v < 3);
    match v {
        0 => BcIndexAdmissionGateState::Open,
        1 => BcIndexAdmissionGateState::Draining,
        _ => BcIndexAdmissionGateState::Locked,
    }
}

/// A nondeterministic `Option<BcIndexMigrationTxnState>` (no txn, or any of
/// the four states).
fn any_optional_txn_state() -> Option<BcIndexMigrationTxnState> {
    if kani::any() {
        Some(any_txn_state())
    } else {
        None
    }
}

/// A fully nondeterministic [`TerminalReconcileInputs`] — every field of the
/// shared pure core's canonical input record (VP-146 v1.2 / VP-147 v1.1) is
/// free, so a proof over it covers the core's ENTIRE input space (2^13 x 5
/// points; no field is left fixed).
fn any_terminal_reconcile_inputs() -> TerminalReconcileInputs {
    TerminalReconcileInputs {
        lock_acquired: kani::any(),
        record_present: kani::any(),
        txn_state: any_optional_txn_state(),
        record_parses: kani::any(),
        txn_id_eq: kani::any(),
        generation_id_eq: kani::any(),
        count_eq_n: kani::any(),
        hashes_eq: kani::any(),
        txn_migration_known: kani::any(),
    }
}

/// Every Branch C verification check passes (the full-verification
/// conjunction of BC-1.18.011 Postcondition 9).
fn all_terminal_checks_pass(i: &TerminalReconcileInputs) -> bool {
    i.record_parses
        && i.txn_id_eq
        && i.generation_id_eq
        && i.count_eq_n
        && i.hashes_eq[0]
        && i.hashes_eq[1]
        && i.hashes_eq[2]
        && i.hashes_eq[3]
}

fn optional_state_is_live(s: Option<BcIndexMigrationTxnState>) -> bool {
    matches!(
        s,
        Some(BcIndexMigrationTxnState::Staging | BcIndexMigrationTxnState::Committing)
    )
}

/// Build one txn record whose only decision-relevant fields (`state`,
/// `generation_id.is_some()`) are the caller's; every other field is fixed
/// and is never branched on by [`recover`](super::recover) (see this
/// module's doc comment). All `String` fields are EMPTY on purpose: their
/// concrete values are irrelevant to every proof here (the assertions match
/// on `RecoveryDecision` discriminants / the unit variant, never on a
/// carried id), and empty strings keep CBMC's heap modeling of the
/// `String` clones `recover()` performs near-free — the difference between
/// a tractable proof and an intractable SAT instance (research §1d).
fn make_txn_record(
    state: BcIndexMigrationTxnState,
    has_generation_id: bool,
) -> BcIndexMigrationTxnRecord {
    BcIndexMigrationTxnRecord {
        txn_id: String::new(),
        activation_id: String::new(),
        fencing_generation: 1,
        state,
        generation_id: if has_generation_id {
            Some(String::new())
        } else {
            None
        },
        source_sha256: None,
        source_body_row_sha256: None,
        intent_log_path: None,
        pending_canonical_moves: Vec::new(),
        created_at: String::new(),
        updated_at: String::new(),
    }
}

/// Is this decision one that authorizes the writer-admission gate to be
/// OPEN (i.e. treats the store as having no unresolved migration)? These
/// are exactly the "no active transaction" decisions — the fail-OPEN
/// hazard the safety predicate (harness 2) guards.
fn decision_permits_admission_open(d: &RecoveryDecision) -> bool {
    matches!(
        d,
        RecoveryDecision::NoActiveTransaction
            | RecoveryDecision::AlreadyMigrated
            | RecoveryDecision::AbortedTerminal { .. }
    )
}

/// Is this decision a forward/mutating action (redo a rename, resume
/// staging)? These are the decisions that MUST only be reached when the
/// on-disk cross-checks hold (`gen_dir_exists` + a valid manifest).
fn decision_is_forward_action(d: &RecoveryDecision) -> bool {
    matches!(
        d,
        RecoveryDecision::ForwardRecovery { .. } | RecoveryDecision::ResumeFromStaging { .. }
    )
}

/// A txn state `classify_txn_records` treats as LIVE (STAGING/COMMITTING).
/// The harnesses reason about liveness through this allocation-free
/// predicate (an iterator `filter`/`find`) rather than re-calling
/// `classify_txn_records` — recover() already calls classify internally, so
/// a second `Vec<&record>` build in the harness would only double CBMC's
/// (dominant) allocator-modeling cost for no added coverage.
fn state_is_live(s: BcIndexMigrationTxnState) -> bool {
    matches!(
        s,
        BcIndexMigrationTxnState::Staging | BcIndexMigrationTxnState::Committing
    )
}

// ===========================================================================
// Harness 1 — recover() TOTALITY (research §5a-1; design §2.2)
//
// A defined RecoveryDecision for EVERY combination of classify_txn_records
// inputs: no panic, no unreachable!(), no arithmetic fault. Kani's built-in
// checks (which fire automatically for every #[kani::proof]) prove the
// absence of panic/overflow/OOB; the explicit assertions pin the three
// adversary-finding-derived regression cases (stale-terminal+live,
// mid-rename COMMITTING, incomplete-STAGING) so a future refactor that
// reintroduces any of findings #1/#2/#3 fails this proof.
// ===========================================================================

/// Bounded record-slice generator: 0, 1, or 2 records, each with arbitrary
/// state + arbitrary generation-id presence. Bound is 2 because
/// `recover()`'s only cardinality branch is `live.len() > 1`, which two
/// live records already exercise — so this one generator lets h1/h2 cover
/// the multiple-live (finding #1) case inline (research §1d: keep domains
/// tiny). Built with a fixed two-slot `if` structure, not a symbolic-length
/// loop. Callers `std::mem::forget` the returned records at scope end — see
/// the module-level "Solve-time note" for why skipping the String/Vec drop
/// glue is what keeps the recover-based harnesses tractable.
fn any_bounded_records() -> Vec<BcIndexMigrationTxnRecord> {
    let count: u8 = kani::any();
    kani::assume(count <= 2);
    let mut records = Vec::with_capacity(2);
    if count >= 1 {
        records.push(make_txn_record(any_txn_state(), kani::any()));
    }
    if count >= 2 {
        records.push(make_txn_record(any_txn_state(), kani::any()));
    }
    records
}

#[kani::proof]
#[kani::unwind(4)]
fn proof_obl1_h1_recover_totality() {
    let records = any_bounded_records();
    let completed_present: bool = kani::any();
    let completed = CompletedMigrationRecord {
        generation_id: String::new(),
        txn_id: String::new(),
        completed_at: String::new(),
        canonical_paths_count: 1,
    };
    let completed_ref = if completed_present {
        Some(&completed)
    } else {
        None
    };
    let gen_dir_exists: bool = kani::any();
    let manifest_status = any_manifest_status();

    // Totality: this call must return a defined RecoveryDecision for every
    // input combination — Kani's automatic panic/unreachable/overflow
    // checks are the actual totality proof; a non-total function (a stray
    // `unreachable!()` arm, an overflow, an index-out-of-bounds) would be
    // flagged here.
    let decision = recover(
        &records,
        None,
        completed_ref,
        gen_dir_exists,
        manifest_status,
    );

    // Regression assertions for the three adversary findings the total
    // function structurally closes (design §2.2). Liveness is derived with
    // allocation-free iterators (see `state_is_live`), NOT a second
    // `classify_txn_records` call.
    let live_count = records.iter().filter(|r| state_is_live(r.state)).count();

    if completed_present {
        // completed.json presence is sufficient, unconditionally (ADR-052
        // §7c step 8) — checked FIRST, before any txn-record inspection.
        kani::assert(
            decision == RecoveryDecision::AlreadyMigrated,
            "H1: completed.json present => AlreadyMigrated regardless of txn records",
        );
    } else if live_count > 1 {
        // Finding #1 generalized: >1 coexisting LIVE records => fail-CLOSED
        // Quarantine, never a silent pick (holds regardless of how many
        // terminal records also coexist).
        kani::assert(
            matches!(
                decision,
                RecoveryDecision::Quarantine {
                    reason: QuarantineReason::MultipleLiveTxnRecords { .. }
                }
            ),
            "H1: >1 live txn records => Quarantine (never fail-open pick)",
        );
    } else if let Some(live) = records.iter().find(|r| state_is_live(r.state)) {
        // Exactly one live record (live_count == 1). Finding #1: a stale
        // terminal record coexisting with it never suppresses it.
        // Finding #2: mid-rename COMMITTING is a defined decision (never
        // dead recovery code / permanent-stuck).
        if live.state == BcIndexMigrationTxnState::Committing {
            kani::assert(
                decision != RecoveryDecision::NoActiveTransaction,
                "H2/finding#2: a live COMMITTING record is never treated as no-active-txn",
            );
        }
        // Finding #3: incomplete-STAGING (generation_id=None) is DISCARDED,
        // never permanently stuck.
        if live.state == BcIndexMigrationTxnState::Staging && live.generation_id.is_none() {
            kani::assert(
                matches!(decision, RecoveryDecision::DiscardPreGeneration { .. }),
                "H3/finding#3: STAGING with no generation_id => DiscardPreGeneration",
            );
        }
    }
    kani::cover!(
        !completed_present && live_count > 1,
        "H1 non-vacuity: the multiple-live Quarantine case is reachable"
    );
    kani::cover!(
        matches!(decision, RecoveryDecision::DiscardPreGeneration { .. }),
        "H1 non-vacuity: DiscardPreGeneration is reachable"
    );
    // Skip drop-glue modeling of the String-heavy records/decision at scope
    // end: CBMC unrolls the Vec/String destructors, which dominates solve
    // time here for zero verification value (a harness never really runs).
    std::mem::forget(records);
    std::mem::forget(decision);
    std::mem::forget(completed);
}

// ===========================================================================
// Harness 1b (VP-147 v1.18 extension) — TERMINAL-RECORD RECONCILIATION
// TOTALITY over the REAL shared pure core
// `decide_terminal_record_reconciliation` (ADR-052 v1.18 §5a step 3.5
// Branch C; BC-1.18.011 v1.13 Precondition 6(d) five-row decision table,
// Precondition 6(e), Postcondition 9; EC-010..EC-014).
//
// Every one of the core's inputs is free (`any_terminal_reconcile_inputs`),
// so this is exhaustive over its whole input space. The four outcome
// biconditionals below fully characterize the function against the BC
// table, row by row (they are a specification, not a copy of the
// implementation's control flow: each outcome is pinned to the exact input
// region the BC assigns it), and the named row assertions restate the
// normative cells verbatim. Row numbers are BC-1.18.011 v1.17 /
// BC-1.18.013 v1.9 Precondition 6(d) numbering: 1 NoOp (lock not acquired /
// no live txn), 2 RefuseForeignMigration (`migration_id ∉ K`), 3 NoOp (known
// migration, its terminal record ABSENT), 4 FinalizeThenOpenGate, 5
// FailClosedMismatch. K = {`migrate-bc-index`, `backfill-append-logs`} (an
// absent `migration_id` reads `migrate-bc-index`); the core's input
// `txn_migration_known` is `migration_id ∈ K` — the shared core serves BOTH
// migrations and has no "evaluating migration".
// ===========================================================================

#[kani::proof]
fn proof_obl1_h1_terminal_reconcile_totality() {
    let i = any_terminal_reconcile_inputs();
    // Totality: a defined decision for every input, no panic (Kani's
    // automatic checks).
    let d = decide_terminal_record_reconciliation(&i);

    let live = optional_state_is_live(i.txn_state);
    let staging = i.txn_state == Some(BcIndexMigrationTxnState::Staging);
    let committing = i.txn_state == Some(BcIndexMigrationTxnState::Committing);
    let checks = all_terminal_checks_pass(&i);

    // Row 1 (lock not acquired / no live txn) and row 3 (known migration,
    // its terminal record ABSENT — STAGING and COMMITTING alike): NoOp, and
    // ONLY there.
    kani::assert(
        (d == TerminalReconcileDecision::NoOp)
            == (!i.lock_acquired || !live || (i.txn_migration_known && !i.record_present)),
        "H1b rows 1/3: NoOp iff lock not acquired, no live txn, or known-migration live txn with its terminal record absent",
    );
    // Row 2 (Precondition 6(e)): a live txn whose `migration_id ∉ K` under
    // the lock is refused — with precedence over every record check.
    kani::assert(
        (d == TerminalReconcileDecision::RefuseForeignMigration)
            == (i.lock_acquired && live && !i.txn_migration_known),
        "H1b row 2: RefuseForeignMigration iff lock acquired, live txn, migration_id not in K",
    );
    // Row 4 (Postcondition 9): FINALIZE only on full verification of the
    // known migration's own present terminal record against a COMMITTING txn
    // under the lock.
    kani::assert(
        (d == TerminalReconcileDecision::FinalizeThenOpenGate)
            == (i.lock_acquired
                && i.txn_migration_known
                && i.record_present
                && committing
                && checks),
        "H1b row 4: FinalizeThenOpenGate iff lock + known migration + its record present + COMMITTING + every check passes",
    );
    // Row 5: every other known-migration, record-present live case fails
    // closed.
    kani::assert(
        (d == TerminalReconcileDecision::FailClosedMismatch)
            == (i.lock_acquired
                && live
                && i.txn_migration_known
                && i.record_present
                && !(committing && checks)),
        "H1b row 5: FailClosedMismatch iff known-migration live txn + its record present and not (COMMITTING and fully verified)",
    );

    // Named normative cells.
    if i.lock_acquired && i.txn_migration_known && i.record_present && staging {
        kani::assert(
            d == TerminalReconcileDecision::FailClosedMismatch,
            "H1b: STAGING + terminal record is ALWAYS fail-closed (no verification consulted)",
        );
    }
    if !i.txn_migration_known {
        kani::assert(
            d != TerminalReconcileDecision::FinalizeThenOpenGate
                && d != TerminalReconcileDecision::FailClosedMismatch,
            "H1b/S7: an unknown-migration_id txn is never finalized and never a completion-record mismatch",
        );
    }
    if i.lock_acquired && live && i.txn_migration_known && !i.record_present {
        kani::assert(
            d == TerminalReconcileDecision::NoOp,
            "H1b row 3: known-migration live txn + lock + its terminal record ABSENT => NoOp (STAGING and COMMITTING alike)",
        );
    }
    if !i.lock_acquired {
        kani::assert(
            d == TerminalReconcileDecision::NoOp,
            "H1b/S4: no decision other than NoOp without the exclusive lock (live coordinator)",
        );
    }

    // Non-vacuity: every outcome and every named cell is reachable.
    kani::cover!(
        d == TerminalReconcileDecision::FinalizeThenOpenGate,
        "H1b non-vacuity: FinalizeThenOpenGate reachable"
    );
    kani::cover!(
        d == TerminalReconcileDecision::RefuseForeignMigration && i.record_present && checks,
        "H1b non-vacuity: row-2 refusal (migration_id not in K) wins over a present, fully verified record"
    );
    kani::cover!(
        d == TerminalReconcileDecision::FailClosedMismatch && staging && checks,
        "H1b non-vacuity: STAGING + record fails closed even with every check passing"
    );
    kani::cover!(
        d == TerminalReconcileDecision::FailClosedMismatch && committing && !checks,
        "H1b non-vacuity: COMMITTING + record with a failed check fails closed"
    );
    kani::cover!(
        d == TerminalReconcileDecision::NoOp
            && i.lock_acquired
            && committing
            && i.txn_migration_known
            && !i.record_present,
        "H1b non-vacuity: row 3 — known COMMITTING + lock + record ABSENT => NoOp"
    );
    kani::cover!(
        d == TerminalReconcileDecision::NoOp
            && i.lock_acquired
            && staging
            && i.txn_migration_known
            && !i.record_present,
        "H1b non-vacuity: row 3 — known STAGING + lock + record ABSENT => NoOp"
    );
}

// ===========================================================================
// Harness 2 — recovery SAFETY PREDICATE (research §5a-2; design §6.2 item 2)
//
// (a) fail-CLOSED, never fail-OPEN: admission may be treated as open ONLY
//     when the unresolved-live-record set is empty (or completed.json is
//     present). Any live STAGING/COMMITTING record with no completed.json
//     forces a non-"admission-open" decision.
// (b) forward/mutating actions (ForwardRecovery / ResumeFromStaging) are
//     reached ONLY when the physical cross-check holds (gen_dir_exists);
//     an unreachable/ambiguous physical state routes to Quarantine /
//     RequiresReauthorization (fail-closed), never to a forward action.
// ===========================================================================

#[kani::proof]
#[kani::unwind(4)]
fn proof_obl1_h2_recovery_safety_predicate() {
    let records = any_bounded_records();
    let gen_dir_exists: bool = kani::any();
    let manifest_status = any_manifest_status();

    // No completed.json in this harness — we are proving the fail-open
    // hazard is closed for the ACTIVE-migration case specifically.
    let decision = recover(&records, None, None, gen_dir_exists, manifest_status);

    // Liveness via allocation-free iterators (see `state_is_live`), not a
    // second `classify_txn_records` call.
    let live_count = records.iter().filter(|r| state_is_live(r.state)).count();
    let single_live = records.iter().find(|r| state_is_live(r.state));

    // (a) Fail-CLOSED: a single unresolved LIVE record must NOT yield an
    // admission-open decision.
    if live_count == 1 {
        kani::assert(
            !decision_permits_admission_open(&decision),
            "H2(a): an unresolved live txn record never yields an admission-open decision",
        );
    }
    // (a') >1 live records => Quarantine (fail-closed), never admission-open.
    if live_count > 1 {
        kani::assert(
            !decision_permits_admission_open(&decision)
                && matches!(decision, RecoveryDecision::Quarantine { .. }),
            "H2(a'): multiple live records => Quarantine, never admission-open",
        );
    }

    // (b) A forward/mutating action is only ever reached when the staged
    // generation directory physically exists (the §7c-step-1 corruption
    // cross-check). generation_id-without-gen-dir must route to Quarantine.
    if decision_is_forward_action(&decision) {
        kani::assert(
            gen_dir_exists,
            "H2(b): a forward-recovery/resume action is never authorized without gen_dir_exists",
        );
    }

    // (b') A live record with generation_id set but gen dir absent is the
    // §7c "Corruption case": ALWAYS fail-closed (Quarantine or, for the
    // COMMITTING+expired arm, RequiresReauthorization) — never a forward
    // action, never admission-open.
    if let Some(live) = single_live
        && live_count == 1
        && live.generation_id.is_some()
        && !gen_dir_exists
    {
        kani::assert(
            !decision_is_forward_action(&decision) && !decision_permits_admission_open(&decision),
            "H2(b'): generation_id set but gen dir absent => fail-closed, never forward/open",
        );
    }
    kani::cover!(
        decision_is_forward_action(&decision),
        "H2 non-vacuity: a forward-recovery/resume action is reachable"
    );
    kani::cover!(
        live_count == 1
            && !gen_dir_exists
            && single_live.is_some_and(|r| r.generation_id.is_some()),
        "H2 non-vacuity: the generation-id-without-gen-dir corruption case is reachable"
    );
    // Skip drop-glue modeling of the String-heavy records/decision at scope
    // end: CBMC unrolls the Vec/String destructors, which dominates solve
    // time here for zero verification value (a harness never really runs).
    std::mem::forget(records);
    std::mem::forget(decision);
}

// ===========================================================================
// Harness 3 — txn state-machine TOTALITY + INDUCTIVE INVARIANT preservation
// (research §5a-3; design §6.2 item 3)
//
// Production performs its STAGING -> COMMITTING -> COMPLETED/ABORTED
// transitions inline in `run_bc_index_migration` (there is no standalone
// `transition()` symbol). This harness therefore models the ADR-052
// §Decision 7a legal-transition relation explicitly and proves it is (a)
// TOTAL (defined, panic-free for every (state, event) pair) and (b)
// preserves the "no turning back" invariant (Invariant 3): a terminal
// state is absorbing, and the state rank is monotonic non-decreasing. The
// one-step property below IS the inductive step; with the base case
// (initial state = Staging, rank 0) it establishes the invariant for
// UNBOUNDED transition sequences (a bounded companion sequence harness
// follows).
// ===========================================================================

#[derive(Clone, Copy)]
enum TxnEvent {
    AssignGeneration,
    BeginCommitting,
    Complete,
    Abort,
    /// VP-147 v1.18 extension (BC-1.18.011 Postcondition 9; ADR-052 §5a
    /// Branch C): finalize from a present terminal record. The event carries
    /// the Branch C facts; whether it fires is decided by the REAL pure core
    /// [`decide_terminal_record_reconciliation`] evaluated against the
    /// CURRENT state (see [`txn_transition`]), never by a harness-local rule.
    FinalizeFromTerminalRecord(TerminalReconcileInputs),
}

fn any_txn_event() -> TxnEvent {
    let v: u8 = kani::any();
    kani::assume(v < 5);
    match v {
        0 => TxnEvent::AssignGeneration,
        1 => TxnEvent::BeginCommitting,
        2 => TxnEvent::Complete,
        3 => TxnEvent::Abort,
        _ => TxnEvent::FinalizeFromTerminalRecord(any_terminal_reconcile_inputs()),
    }
}

/// The ADR-052 §7a legal transition relation. TOTAL by construction: every
/// (state, event) pair maps to a defined next state; an event that is not
/// legal from the current state is a no-op (the state is unchanged), never
/// a panic and never a backwards move. Terminal states (Completed/Aborted)
/// are absorbing.
///
/// `FinalizeFromTerminalRecord` moves the txn to COMPLETED iff the real
/// Branch C core, fed the event's facts with `txn_state` bound to the
/// CURRENT state `s`, decides `FinalizeThenOpenGate`; any other decision
/// (NoOp, FailClosedMismatch, RefuseForeignMigration) leaves the txn as is
/// (no txn write).
fn txn_transition(s: BcIndexMigrationTxnState, e: TxnEvent) -> BcIndexMigrationTxnState {
    use BcIndexMigrationTxnState::*;
    match (s, e) {
        // Staging may self-loop on generation assignment, advance to
        // Committing, or abort.
        (Staging, TxnEvent::AssignGeneration) => Staging,
        (Staging, TxnEvent::BeginCommitting) => Committing,
        (Staging, TxnEvent::Abort) => Aborted,
        // Committing may complete or abort ("no turning back" to Staging).
        (Committing, TxnEvent::Complete) => Completed,
        (Committing, TxnEvent::Abort) => Aborted,
        // Every other event from a non-terminal state is rejected (no-op).
        (Staging, TxnEvent::Complete) => Staging,
        (Committing, TxnEvent::AssignGeneration) => Committing,
        (Committing, TxnEvent::BeginCommitting) => Committing,
        // Terminal-record finalize: decided by the REAL pure core.
        (Staging | Committing, TxnEvent::FinalizeFromTerminalRecord(facts)) => {
            let inputs = TerminalReconcileInputs {
                txn_state: Some(s),
                ..facts
            };
            if decide_terminal_record_reconciliation(&inputs)
                == TerminalReconcileDecision::FinalizeThenOpenGate
            {
                Completed
            } else {
                s
            }
        }
        // Terminal states are absorbing under ALL events.
        (Completed, _) => Completed,
        (Aborted, _) => Aborted,
    }
}

/// Progress rank: Staging < Committing < {Completed, Aborted}. Monotonic
/// non-decrease of this rank is the machine's "no turning back" invariant.
fn txn_rank(s: BcIndexMigrationTxnState) -> u8 {
    match s {
        BcIndexMigrationTxnState::Staging => 0,
        BcIndexMigrationTxnState::Committing => 1,
        BcIndexMigrationTxnState::Completed | BcIndexMigrationTxnState::Aborted => 2,
    }
}

fn txn_is_terminal(s: BcIndexMigrationTxnState) -> bool {
    matches!(
        s,
        BcIndexMigrationTxnState::Completed | BcIndexMigrationTxnState::Aborted
    )
}

/// The `FinalizeFromTerminalRecord` safety facts shared by both h3 harnesses:
/// the event only ever moves COMMITTING -> COMPLETED (STAGING + terminal
/// record is ALWAYS fail-closed), and only for a known migration
/// (`migration_id ∈ K`) with the lock acquired, its own terminal record
/// present and every check passing (BC Precondition 6(d) row 4). Returns the non-vacuity witness "this step finalized COMMITTING
/// -> COMPLETED" for the caller's `kani::cover!`.
fn assert_finalize_event_safety(
    s: BcIndexMigrationTxnState,
    e: TxnEvent,
    next: BcIndexMigrationTxnState,
) -> bool {
    if let TxnEvent::FinalizeFromTerminalRecord(facts) = e {
        if s == BcIndexMigrationTxnState::Staging {
            kani::assert(
                next == BcIndexMigrationTxnState::Staging,
                "H3: FinalizeFromTerminalRecord never moves a STAGING txn (STAGING + terminal record is always fail-closed)",
            );
        }
        if next != s {
            kani::assert(
                s == BcIndexMigrationTxnState::Committing
                    && next == BcIndexMigrationTxnState::Completed,
                "H3: FinalizeFromTerminalRecord only ever moves COMMITTING -> COMPLETED",
            );
            kani::assert(
                facts.lock_acquired
                    && facts.txn_migration_known
                    && facts.record_present
                    && facts.record_parses
                    && facts.txn_id_eq
                    && facts.generation_id_eq
                    && facts.count_eq_n
                    && facts.hashes_eq.iter().all(|ok| *ok),
                "H3: FinalizeFromTerminalRecord fires only for a known migration + lock + its present, fully verified record",
            );
        }
        return s == BcIndexMigrationTxnState::Committing
            && next == BcIndexMigrationTxnState::Completed;
    }
    false
}

#[kani::proof]
fn proof_obl1_h3_transition_inductive_step() {
    let s = any_txn_state();
    let e = any_txn_event();
    let next = txn_transition(s, e);

    // Totality is proven by this call returning without panic for every
    // (s, e) — Kani's automatic checks. The inductive-step invariant:
    // (1) rank never decreases ("no turning back").
    kani::assert(
        txn_rank(next) >= txn_rank(s),
        "H3: txn transition never decreases progress rank (no turning back)",
    );
    // (2) terminal states are absorbing.
    if txn_is_terminal(s) {
        kani::assert(
            next == s,
            "H3: a terminal txn state is absorbing under every event",
        );
    }
    // (3) VP-147 v1.18: the FinalizeFromTerminalRecord event.
    let finalized = assert_finalize_event_safety(s, e, next);
    kani::cover!(
        finalized,
        "H3 non-vacuity: FinalizeFromTerminalRecord moves COMMITTING -> COMPLETED"
    );
    kani::cover!(
        matches!(e, TxnEvent::FinalizeFromTerminalRecord(_))
            && s == BcIndexMigrationTxnState::Committing
            && next == BcIndexMigrationTxnState::Committing,
        "H3 non-vacuity: an unverified terminal record leaves COMMITTING unchanged (fail-closed)"
    );
    kani::cover!(
        matches!(e, TxnEvent::FinalizeFromTerminalRecord(f) if f.lock_acquired && f.txn_migration_known && f.record_present)
            && s == BcIndexMigrationTxnState::Staging,
        "H3 non-vacuity: FinalizeFromTerminalRecord evaluated against STAGING + known migration's present record"
    );
}

/// Bounded companion: apply an arbitrary sequence of events from the base
/// state and assert the invariant holds at every step. Complements the
/// one-step inductive proof with a concrete finite-trajectory check.
#[kani::proof]
#[kani::unwind(7)]
fn proof_obl1_h3_transition_bounded_sequence() {
    let steps: usize = kani::any();
    kani::assume(steps <= 6);
    // Base case: the machine starts in Staging (rank 0).
    let mut s = BcIndexMigrationTxnState::Staging;
    let mut finalized_in_run = false;
    for _ in 0..steps {
        let e = any_txn_event();
        let next = txn_transition(s, e);
        kani::assert(
            txn_rank(next) >= txn_rank(s),
            "H3-seq: progress rank monotonic across the whole sequence",
        );
        if txn_is_terminal(s) {
            kani::assert(
                next == s,
                "H3-seq: terminal state stays terminal for the rest of the run",
            );
        }
        if assert_finalize_event_safety(s, e, next) {
            finalized_in_run = true;
        }
        s = next;
    }
    kani::cover!(
        finalized_in_run && s == BcIndexMigrationTxnState::Completed,
        "H3-seq non-vacuity: a run reaches COMPLETED via FinalizeFromTerminalRecord"
    );
    kani::cover!(
        s == BcIndexMigrationTxnState::Aborted,
        "H3-seq non-vacuity: a run reaches ABORTED"
    );
}

// ===========================================================================
// Harness 4 — ADMISSION-GATE invariant via a finite scheduler
// (research §5a-4; design §6.2 item 4)
//
// Proves INV-GATE-TXN: no writer is admitted (and hence no writer can
// mutate a BC-INDEX path) while a migration is in flight — i.e. while the
// gate is DRAINING/LOCKED and/or the txn is STAGING/COMMITTING. The
// admission decision at each step is the REAL production predicate
// `is_bc_index_admission_open`; the scheduler is a finite nondeterministic
// sequence of protocol actions modeling the ADR-052 §5a drain/reopen
// lifecycle. Invariant asserted after EVERY step.
// ===========================================================================

#[kani::proof]
#[kani::unwind(7)]
fn proof_obl1_h4_admission_gate_invariant() {
    let mut gate = BcIndexAdmissionGateState::Open;
    // Modeled migration txn: None (no migration) or a live/terminal state.
    let mut txn_state: Option<BcIndexMigrationTxnState> = None;
    let mut active_writers: u32 = 0;

    let steps: usize = kani::any();
    kani::assume(steps <= 6);

    for _ in 0..steps {
        // Build the txn record (if any) the real admission predicate sees.
        let txn_record = txn_state.map(|st| make_txn_record(st, true));

        let action: u8 = kani::any();
        kani::assume(action < 5);
        match action {
            0 => {
                // AdmitWriter — gated by the REAL production predicate.
                if is_bc_index_admission_open(gate, txn_record.as_ref()) {
                    // Bound the counter to keep the domain finite.
                    if active_writers < 3 {
                        active_writers += 1;
                    }
                }
            }
            1 => {
                // ReleaseWriter.
                if active_writers > 0 {
                    active_writers -= 1;
                }
            }
            2 => {
                // BeginDrain: close admission FIRST (Open -> Draining).
                if gate == BcIndexAdmissionGateState::Open {
                    gate = BcIndexAdmissionGateState::Draining;
                }
            }
            3 => {
                // CompleteDrain: only once writers have quiesced
                // (active == 0), Draining -> Locked and the migration txn
                // becomes live (Staging).
                if gate == BcIndexAdmissionGateState::Draining && active_writers == 0 {
                    gate = BcIndexAdmissionGateState::Locked;
                    txn_state = Some(BcIndexMigrationTxnState::Staging);
                }
            }
            _ => {
                // Advance/finish the migration and reopen: Staging ->
                // Committing -> Completed, then gate reopens.
                match txn_state {
                    Some(BcIndexMigrationTxnState::Staging) => {
                        txn_state = Some(BcIndexMigrationTxnState::Committing);
                    }
                    Some(BcIndexMigrationTxnState::Committing) => {
                        txn_state = Some(BcIndexMigrationTxnState::Completed);
                    }
                    _ => {
                        // Terminal or no migration: safe to reopen.
                        gate = BcIndexAdmissionGateState::Open;
                        txn_state = None;
                    }
                }
            }
        }

        // INV-GATE-TXN (safety): a writer is only ever active while the
        // gate is OPEN. Because CompleteDrain requires active==0 before it
        // can move to Locked and start the migration, and admission is
        // gated by the real predicate (which returns false unless
        // gate==Open AND txn is None/terminal), no writer can be active
        // while a migration is in flight.
        // INV-GATE-TXN (the actual safety property): NO writer is active
        // while a migration txn is in flight (STAGING/COMMITTING). This is
        // upheld because CompleteDrain requires `active_writers == 0` before
        // it may move to Locked and start the migration (txn=Staging), and
        // admission after that point is refused by the REAL predicate
        // (gate is Locked AND txn is live). A writer admitted while the gate
        // was OPEN may still be draining (active>0) during the DRAINING
        // phase — that is correct and precisely why the drain step waits;
        // no migration txn exists yet during draining.
        if matches!(
            txn_state,
            Some(BcIndexMigrationTxnState::Staging) | Some(BcIndexMigrationTxnState::Committing)
        ) {
            kani::assert(
                active_writers == 0,
                "H4/INV-GATE-TXN: no writer is active while a migration txn is STAGING/COMMITTING",
            );
            // A live migration also implies the admission gate is closed.
            kani::assert(
                gate != BcIndexAdmissionGateState::Open,
                "H4: an in-flight migration implies the admission gate is closed",
            );
        }
        // A writer is only ever admitted while the gate is OPEN, so it can
        // only be active while the gate is OPEN or DRAINING (never LOCKED —
        // LOCKED is reached only after quiescence).
        if active_writers > 0 {
            kani::assert(
                gate != BcIndexAdmissionGateState::Locked,
                "H4: no writer is active once the gate is LOCKED (drain waited for quiescence)",
            );
        }
        kani::cover!(
            active_writers > 0 && gate == BcIndexAdmissionGateState::Draining,
            "H4 non-vacuity: a writer admitted while OPEN is still draining"
        );
        kani::cover!(
            txn_state == Some(BcIndexMigrationTxnState::Committing),
            "H4 non-vacuity: a migration reaches COMMITTING"
        );
    }
}

// ===========================================================================
// Harness 4b (VP-147 v1.18 extension) — RESERVATION QUIESCENCE + SELF-HEAL
// (ADR-052 v1.18 §5a step 0 reserve-then-verify, drain steps 1-6, step 3.5
// Branches A/B/C, release-on-block; BC-1.18.011 Precondition 6(b)/(c)/(d)/
// (e), EC-007..EC-014)
//
// A finite nondeterministic scheduler over the product state
//   gate x txn{state, generation null?, migration_id in K / not in K} x terminal
//   record x exclusive.lock (held by a live coordinator) x 2 writers x 2
//   reservation slots {absent, present{created_at parsed (possibly future) |
//   unusable, mtime available (possibly future) | unavailable}} x clock
// with ops (each an atomic step; the scheduler interleaves them freely):
//   writer:  W1 reserve | W2a read gate | W2b read txns + decide (REAL
//            `is_bc_index_admission_open`) | step-3.5 reconcile (REAL
//            `plan_stale_gate_reconciliation`, itself over the REAL
//            `decide_terminal_record_reconciliation`) | PostToolUse /
//            later-stage release | crash (reservation leaks);
//   coordinator: start (acquire lock; fresh run or resume) | C1 DRAINING |
//            drain TTL GC (REAL `reservation_is_stale`, the SHIPPED
//            `DEFAULT_MAX_RESERVATION_TTL`) | C2 read reservations ->
//            LOCKED | quiescence snapshot + txn STAGING | assign generation
//            | COMMITTING | write terminal record | txn COMPLETED | gate
//            OPEN + release | crash (lock released, state left) | drain
//            timeout (gate OPEN);
//   clock tick.
// The writer's verification is split exactly as production reads it
// (`AdmissionSnapshot::read`: gate FIRST, then the txn scan), so the Dekker
// argument (W1 < W2, C1 < C2: never both miss) is exercised against
// arbitrary interleavings — including a writer that read OPEN before the
// coordinator's C1.
//
// The initial state is an arbitrary post-crash state constrained only by
// "no coordinator / no writer running": any gate, any txn (incl. a
// FOREIGN-migration txn), any terminal record, any leaked reservations of
// any age — including gate OPEN beside a live txn (an absent/lost
// `gate-state.json` reads OPEN), so S1 is checked as an inductive property
// and the admission dual check's txn half is load-bearing. This covers the stuck states
// self-heal exists for (crashed coordinator in DRAINING / LOCKED,
// null-generation STAGING, COMMITTING + terminal record, ...).
//
// Model choices (honest boundary):
// * The Branch C verification facts are FREE at each reconcile, so the model
//   includes the S-25.06 verified-finalize effect (txn COMPLETED, then gate
//   OPEN) as well as this build's fail-closed seam (all checks unverified =>
//   no write). Safety is proven over that superset.
// * TTL assumption (BC-1.18.011 Precondition 6(c) rationale for 3,600 s):
//   a live writer's tool call outlives no TTL — the clock may only advance
//   while every live writer's reservation stays non-stale under the REAL
//   predicate. A tool call longer than the TTL is the documented residual
//   (outside any model).
// * The coordinator's fresh run requires no live txn and no terminal record
//   (production's completed.json short-circuit runs before the lock and is
//   outside the shared-core scope; S-25.06 B2-2); resume is modeled for the
//   coordinator's own STAGING-with-generation / COMMITTING txn only.
// * Reconcile and each effect are atomic (production: under
//   `exclusive.lock`, txn write before gate write).
// Real flock, wall-clock time, PostToolUse delivery and filesystem
// visibility are carried by the black-box suites (VP-133 facets 6/7).
//
// Safety, asserted after EVERY step / at the named op:
//   S1 INV-GATE-TXN: gate OPEN => no live txn, inductively (once it holds
//      it keeps holding); no writer admitted while a txn is
//      STAGING/COMMITTING or the gate is LOCKED (unconditionally, from every
//      initial state).
//   S2 quiescence precedes snapshot: at the snapshot no writer is admitted
//      and no writer holds an OPEN gate observation that could still admit.
//   S3 no admit while live: admission happens only with a reservation
//      already present (reserve-then-verify) and no live txn.
//   S4 no self-heal under a live lock: LiveCoordinator, no write.
//   S5 mismatch stable: FailClosedMismatch writes nothing and re-planning
//      the unchanged state yields the same mismatch.
//   S6 bounded progress / no permanent self-lock: a reconcile that holds the
//      lock never leaves the gate closed with no live txn, nor a
//      null-generation known-migration STAGING txn stuck without its
//      terminal record.
//   S7 (VP-147 v1.4) a live txn whose `migration_id ∉ K` (unknown to this
//      build; `txn_migration_known = false`) is never finalized, aborted or
//      reconciled over.
//   S8 created_at-first staleness: a TRUSTED created_at (<= now + 300 s)
//      decides and the mtime is ignored; with created_at absent the mtime
//      is judged exactly as a created_at of that value; the drain GC never
//      reclaims a live writer's reservation.
//   S10 (VP-147 v1.4; ADR-052 v1.20 "Reservation timestamp rules", the REAL
//      `reservation_is_stale` with the shipped 300 s skew) staleness
//      retention: a reservation of UNKNOWN age (no usable created_at and no
//      mtime) is never reclaimed; a created_at beyond now + 300 s is
//      untrusted and falls back to the mtime (never clamped); a future
//      created_at within the skew ages as 0 (never stale). Leaked
//      reservations in the initial state carry arbitrary, possibly future,
//      possibly absent stamps. RFC 3339 parsing / pre-epoch / u64-overflow
//      classification (all mapping to `None`) and the coordinator-drain
//      stderr advisory tokens (BC-1.18.011 v1.18 Precondition 6(c) rule (5))
//      are string/IO concerns outside Kani (VP-133 facet 7(c)).
//   S9 (VP-147 v1.2) the reservation namespace exists from the first
//      protected write: the initial state may be the never-migrated project
//      (no `.factory/migration-state/`); the admitter creates the namespace
//      before its reservation; every admitted writer has a reservation
//      inside an existing namespace, so a subsequent drain sees it.
//      Claim boundary: this is a property of the MODELED admitter protocol.
//      The scope guard that once short-circuited on an absent namespace sits
//      in the effectful `executor::migration_writer_admission` (std::fs),
//      outside Kani's domain; that call site, real `create_dir_all`
//      idempotence, concurrent first-ever creation and EACCES/EROFS/ENOSPC
//      fail-closed are carried by the black-box suite (VP-133 facet 7(d):
//      `test_BC_1_18_013_EC020_*`).
// ===========================================================================

/// The SHIPPED reservation TTL (3,600 s) — the model checks the production
/// constant, not a toy value.
const H4X_TTL_SECS: u64 = DEFAULT_MAX_RESERVATION_TTL.as_secs();

/// The SHIPPED clock-skew tolerance (300 s; ADR-052 v1.20 §5a "Reservation
/// timestamp rules").
const H4X_SKEW_SECS: u64 = RESERVATION_CLOCK_SKEW_TOLERANCE_SECS;

#[derive(Clone, Copy, PartialEq, Eq)]
enum H4xWriterPhase {
    /// No tool call in flight (a leaked reservation may still exist).
    Idle,
    /// W1 done (own reservation present); gate not yet read.
    Reserved,
    /// W2a done: the gate value read is in `gate_seen`.
    GateSeen,
    /// First verification failed: step-3.5 reconciliation pending.
    Reconcile,
    /// Admitted: the write is in flight until PostToolUse.
    Admitted,
}

#[derive(Clone, Copy)]
struct H4xWriter {
    phase: H4xWriterPhase,
    gate_seen: BcIndexAdmissionGateState,
    reconciled: bool,
}

#[derive(Clone, Copy)]
struct H4xReservation {
    present: bool,
    /// `None` = the `created_at` field is absent / unparseable / pre-epoch
    /// (the RFC 3339 parse is string I/O outside Kani; its `None` outcome is
    /// what the predicate sees). A `Some` may lie in the FUTURE (skew).
    created_at: Option<u64>,
    /// `None` = the file mtime is unavailable (incl. pre-epoch).
    mtime: Option<u64>,
}

#[derive(Clone, Copy)]
struct H4xTxn {
    state: BcIndexMigrationTxnState,
    generation_id_is_null: bool,
    /// The txn's `migration_id ∈ K` (`txn_migration_known`: one of the two
    /// ids the build recognises, so its own terminal record can be selected);
    /// `false` = unknown `migration_id` (BC row 2).
    migration_known: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum H4xCoord {
    /// No coordinator; `exclusive.lock` free.
    Absent,
    /// Lock acquired; C1 not yet done.
    LockHeld,
    /// C1 done (gate durably DRAINING); draining.
    Draining,
    /// C2 saw no reservation; gate LOCKED; snapshot next.
    Locked,
    StagingNullGeneration,
    StagingWithGeneration,
    Committing,
    TerminalRecordWritten,
    TxnCompleted,
}

fn h4x_txn_live(txn: Option<H4xTxn>) -> bool {
    txn.is_some_and(|t| state_is_live(t.state))
}

/// A nondeterministic timestamp `<= bound`.
fn h4x_time_at_most(bound: u64) -> u64 {
    let t: u64 = kani::any();
    kani::assume(t <= bound);
    t
}

#[kani::proof]
#[kani::unwind(11)]
fn proof_obl1_h4_reservation_quiescence_and_selfheal() {
    // ---- Arbitrary post-crash initial state (base case of S1) -------------
    let mut now: u64 = h4x_time_at_most(4 * H4X_TTL_SECS);
    let mut gate = any_gate_state();
    let mut txn: Option<H4xTxn> = if kani::any() {
        Some(H4xTxn {
            state: any_txn_state(),
            generation_id_is_null: kani::any(),
            migration_known: kani::any(),
        })
    } else {
        None
    };
    let mut terminal_record: bool = kani::any();
    // NO S1 assumption on the initial state: `gate-state.json` absent
    // (deleted / lost) reads OPEN (`read_admission_gate_state`), so "gate
    // OPEN beside a live txn" is a real starting state. The admission dual
    // check's txn half and Branch B must cope with it; S1 is asserted
    // INDUCTIVELY (once it holds it must keep holding) below.
    let mut s1_holds = !(gate == BcIndexAdmissionGateState::Open && h4x_txn_live(txn));
    let lost_gate_file_initially = !s1_holds;
    let mut coord = H4xCoord::Absent;
    let idle = H4xWriter {
        phase: H4xWriterPhase::Idle,
        gate_seen: BcIndexAdmissionGateState::Open,
        reconciled: false,
    };
    let mut writers = [idle, idle];
    let mut res = [
        H4xReservation {
            present: kani::any(),
            created_at: if kani::any() {
                Some(h4x_time_at_most(now + 2 * H4X_SKEW_SECS))
            } else {
                None
            },
            mtime: if kani::any() {
                Some(h4x_time_at_most(now + 2 * H4X_SKEW_SECS))
            } else {
                None
            },
        },
        H4xReservation {
            present: kani::any(),
            created_at: if kani::any() {
                Some(h4x_time_at_most(now + 2 * H4X_SKEW_SECS))
            } else {
                None
            },
            mtime: if kani::any() {
                Some(h4x_time_at_most(now + 2 * H4X_SKEW_SECS))
            } else {
                None
            },
        },
    ];
    // S9 (VP-147 v1.2): `.factory/migration-state/` (and so `reservations/`)
    // may not exist yet — the never-migrated project. Then nothing under it
    // exists either: no reservation, no txn, no terminal record, and the
    // absent gate file reads OPEN.
    let mut namespace_exists: bool = kani::any();
    let never_migrated_initially = !namespace_exists;
    if never_migrated_initially {
        kani::assume(
            !res[0].present
                && !res[1].present
                && txn.is_none()
                && !terminal_record
                && gate == BcIndexAdmissionGateState::Open,
        );
    }

    // Ghost state for the non-vacuity witnesses.
    let mut first_ever_admission = false;
    let mut drain_after_first_ever_admission = false;
    let mut ever_admitted = false;
    let mut admitted_after_selfheal = false;
    let mut snapshot_after_admission = false;
    let mut coordinator_waited = false;
    let mut released_on_block = false;
    let mut blocked_by_draining_observation = false;
    let mut stale_gc_by_created_at = false;
    let mut stale_gc_by_mtime_fallback = false;
    let mut unknown_age_retained = false;
    let mut future_created_at_fell_back_to_mtime = false;
    let mut within_skew_future_retained = false;
    let mut plan_reopen = false;
    let mut plan_abort_null = false;
    let mut plan_finalize = false;
    let mut plan_mismatch = false;
    let mut plan_foreign = false;
    let mut plan_live_coordinator = false;
    let mut blocked_by_txn_half_of_dual_check = false;
    let mut lost_gate_state_healed = false;

    let steps: usize = kani::any();
    kani::assume(steps <= 10);
    for _ in 0..steps {
        let w: usize = if kani::any() { 1 } else { 0 };
        let action: u8 = kani::any();
        kani::assume(action < 13);
        match action {
            // ---- W1: reserve FIRST ---------------------------------------
            0 => {
                if writers[w].phase == H4xWriterPhase::Idle && !res[w].present {
                    // S9: the admitter creates the namespace idempotently
                    // (`create_dir_all`) BEFORE inserting its reservation;
                    // there is no "namespace absent => admit without a
                    // reservation" transition.
                    namespace_exists = true;
                    res[w] = H4xReservation {
                        present: true,
                        created_at: if kani::any() { Some(now) } else { None },
                        mtime: Some(now),
                    };
                    writers[w] = H4xWriter {
                        phase: H4xWriterPhase::Reserved,
                        gate_seen: BcIndexAdmissionGateState::Open,
                        reconciled: false,
                    };
                }
            }
            // ---- W2a: read gate-state.json --------------------------------
            1 => {
                if writers[w].phase == H4xWriterPhase::Reserved {
                    writers[w].phase = H4xWriterPhase::GateSeen;
                    writers[w].gate_seen = gate;
                }
            }
            // ---- W2b: scan txn-*.json and decide (REAL predicate) ---------
            2 => {
                if writers[w].phase == H4xWriterPhase::GateSeen {
                    let record = txn.map(|t| make_txn_record(t.state, !t.generation_id_is_null));
                    let open = is_bc_index_admission_open(writers[w].gate_seen, record.as_ref());
                    std::mem::forget(record);
                    if open {
                        // S3.
                        kani::assert(
                            res[w].present,
                            "H4b/S3: admission only after the admitter's own reservation exists (reserve-then-verify)",
                        );
                        kani::assert(
                            !h4x_txn_live(txn),
                            "H4b/S3: no writer is admitted while a txn is STAGING/COMMITTING",
                        );
                        if never_migrated_initially && !ever_admitted {
                            first_ever_admission = true;
                        }
                        writers[w].phase = H4xWriterPhase::Admitted;
                        ever_admitted = true;
                        if writers[w].reconciled {
                            admitted_after_selfheal = true;
                        }
                    } else if !writers[w].reconciled {
                        if writers[w].gate_seen == BcIndexAdmissionGateState::Open {
                            // Gate read OPEN, yet refused: only the txn half
                            // of the dual check can have decided this.
                            blocked_by_txn_half_of_dual_check = true;
                        }
                        writers[w].phase = H4xWriterPhase::Reconcile;
                    } else {
                        // Release-on-block: the admitter removes its OWN
                        // reservation before returning E-MAINTENANCE-001.
                        if writers[w].gate_seen == BcIndexAdmissionGateState::Draining {
                            blocked_by_draining_observation = true;
                        }
                        res[w].present = false;
                        writers[w].phase = H4xWriterPhase::Idle;
                        released_on_block = true;
                    }
                }
            }
            // ---- Step-3.5 reconciliation (REAL planner) -------------------
            3 => {
                if writers[w].phase == H4xWriterPhase::Reconcile {
                    let lock_acquired = coord == H4xCoord::Absent;
                    let live = h4x_txn_live(txn);
                    let known = live && txn.is_some_and(|t| t.migration_known);
                    let inputs = TerminalReconcileInputs {
                        lock_acquired,
                        record_present: known && terminal_record,
                        txn_state: if live { txn.map(|t| t.state) } else { None },
                        record_parses: kani::any(),
                        txn_id_eq: kani::any(),
                        generation_id_eq: kani::any(),
                        count_eq_n: kani::any(),
                        hashes_eq: kani::any(),
                        txn_migration_known: known,
                    };
                    let gen_null = live && txn.is_some_and(|t| t.generation_id_is_null);
                    let plan = plan_stale_gate_reconciliation(gate, &inputs, gen_null);
                    let gate_before = gate;
                    let txn_state_before = txn.map(|t| t.state);
                    match plan {
                        StaleGateReconciliationPlan::ReopenGate => {
                            gate = BcIndexAdmissionGateState::Open;
                            plan_reopen = true;
                        }
                        StaleGateReconciliationPlan::AbortNullGenerationThenReopenGate => {
                            if let Some(t) = txn.as_mut() {
                                t.state = BcIndexMigrationTxnState::Aborted;
                            }
                            gate = BcIndexAdmissionGateState::Open;
                            plan_abort_null = true;
                        }
                        StaleGateReconciliationPlan::FinalizeThenOpenGate => {
                            if let Some(t) = txn.as_mut() {
                                t.state = BcIndexMigrationTxnState::Completed;
                            }
                            gate = BcIndexAdmissionGateState::Open;
                            plan_finalize = true;
                        }
                        StaleGateReconciliationPlan::FailClosedMismatch => plan_mismatch = true,
                        StaleGateReconciliationPlan::RefuseForeignMigration => plan_foreign = true,
                        StaleGateReconciliationPlan::LiveCoordinator => {
                            plan_live_coordinator = true;
                        }
                        StaleGateReconciliationPlan::NothingToReconcile => {}
                    }
                    let unchanged = gate == gate_before && txn.map(|t| t.state) == txn_state_before;
                    // S4.
                    if !lock_acquired {
                        kani::assert(
                            plan == StaleGateReconciliationPlan::LiveCoordinator && unchanged,
                            "H4b/S4: no self-heal of any kind while a live coordinator holds the lock",
                        );
                    }
                    // S7.
                    if live && !known {
                        kani::assert(
                            unchanged,
                            "H4b/S7: a live txn whose migration_id is not in K is never finalized, aborted or reconciled over",
                        );
                    }
                    // S5.
                    if plan == StaleGateReconciliationPlan::FailClosedMismatch {
                        kani::assert(
                            unchanged,
                            "H4b/S5: a completion-record mismatch writes neither the txn nor the gate",
                        );
                        kani::assert(
                            plan_stale_gate_reconciliation(gate, &inputs, gen_null)
                                == StaleGateReconciliationPlan::FailClosedMismatch,
                            "H4b/S5: the mismatch is stable on re-evaluation",
                        );
                    }
                    // S6.
                    if lock_acquired {
                        kani::assert(
                            gate == BcIndexAdmissionGateState::Open || h4x_txn_live(txn),
                            "H4b/S6: reconciliation under the lock never leaves the gate closed with no live txn",
                        );
                        kani::assert(
                            !(h4x_txn_live(txn)
                                && txn.is_some_and(|t| {
                                    t.migration_known
                                        && t.state == BcIndexMigrationTxnState::Staging
                                        && t.generation_id_is_null
                                })
                                && !terminal_record),
                            "H4b/S6: a known-migration null-generation STAGING txn without its terminal record never survives reconciliation",
                        );
                    }
                    writers[w].phase = H4xWriterPhase::Reserved;
                    writers[w].reconciled = true;
                }
            }
            // ---- PostToolUse / later-stage block: release -----------------
            4 => {
                if writers[w].phase == H4xWriterPhase::Admitted {
                    res[w].present = false;
                    writers[w].phase = H4xWriterPhase::Idle;
                }
            }
            // ---- Writer crash / harness denial: reservation leaks ---------
            5 => {
                writers[w].phase = H4xWriterPhase::Idle;
            }
            // ---- Coordinator start (acquire exclusive.lock) ---------------
            6 => {
                if coord == H4xCoord::Absent && !terminal_record {
                    // `run_bc_index_migration` creates the namespace (and
                    // `reservations/`) before draining.
                    namespace_exists = true;
                    if !h4x_txn_live(txn) {
                        coord = H4xCoord::LockHeld; // fresh run
                    } else if let Some(t) = txn
                        && t.migration_known
                        && !t.generation_id_is_null
                    {
                        coord = if t.state == BcIndexMigrationTxnState::Committing {
                            H4xCoord::Committing
                        } else {
                            H4xCoord::StagingWithGeneration
                        };
                    }
                }
            }
            // ---- Coordinator protocol step --------------------------------
            7 => match coord {
                H4xCoord::Absent => {}
                H4xCoord::LockHeld => {
                    // C1: durable DRAINING flip.
                    if first_ever_admission {
                        drain_after_first_ever_admission = true;
                    }
                    gate = BcIndexAdmissionGateState::Draining;
                    coord = H4xCoord::Draining;
                }
                H4xCoord::Draining => {
                    // C2: read reservations/; LOCKED only when empty.
                    // A reservation is visible to the drain only inside the
                    // namespace.
                    if !(namespace_exists && res[0].present)
                        && !(namespace_exists && res[1].present)
                    {
                        gate = BcIndexAdmissionGateState::Locked;
                        coord = H4xCoord::Locked;
                    } else {
                        coordinator_waited = true;
                    }
                }
                H4xCoord::Locked => {
                    // S2: the quiescence snapshot.
                    for wr in writers.iter() {
                        kani::assert(
                            wr.phase != H4xWriterPhase::Admitted,
                            "H4b/S2: no writer is admitted when the quiescence snapshot is taken",
                        );
                        kani::assert(
                            !(wr.phase == H4xWriterPhase::GateSeen
                                && wr.gate_seen == BcIndexAdmissionGateState::Open),
                            "H4b/S2: no in-flight writer holds an OPEN gate observation at the snapshot (never both miss)",
                        );
                    }
                    if ever_admitted {
                        snapshot_after_admission = true;
                    }
                    txn = Some(H4xTxn {
                        state: BcIndexMigrationTxnState::Staging,
                        generation_id_is_null: true,
                        migration_known: true,
                    });
                    coord = H4xCoord::StagingNullGeneration;
                }
                H4xCoord::StagingNullGeneration => {
                    if let Some(t) = txn.as_mut() {
                        t.generation_id_is_null = false;
                    }
                    coord = H4xCoord::StagingWithGeneration;
                }
                H4xCoord::StagingWithGeneration => {
                    if let Some(t) = txn.as_mut() {
                        t.state = BcIndexMigrationTxnState::Committing;
                    }
                    coord = H4xCoord::Committing;
                }
                H4xCoord::Committing => {
                    terminal_record = true;
                    coord = H4xCoord::TerminalRecordWritten;
                }
                H4xCoord::TerminalRecordWritten => {
                    if let Some(t) = txn.as_mut() {
                        t.state = BcIndexMigrationTxnState::Completed;
                    }
                    coord = H4xCoord::TxnCompleted;
                }
                H4xCoord::TxnCompleted => {
                    gate = BcIndexAdmissionGateState::Open;
                    coord = H4xCoord::Absent;
                }
            },
            // ---- Drain step 1: TTL GC (REAL predicate, shipped TTL) -------
            8 => {
                if coord == H4xCoord::Draining {
                    for (i, slot) in res.iter_mut().enumerate() {
                        if !slot.present {
                            continue;
                        }
                        let stale =
                            reservation_is_stale(slot.created_at, slot.mtime, now, H4X_TTL_SECS);
                        // Spec regions (BC-1.18.011 v1.17 / BC-1.18.013 v1.9
                        // Precondition 6(c) "Reservation timestamp rules").
                        let skew_bound = now + H4X_SKEW_SECS;
                        let created_at_trusted = slot.created_at.filter(|c| *c <= skew_bound);
                        let created_at_beyond_skew =
                            slot.created_at.is_some_and(|c| c > skew_bound);
                        let alt_mtime: Option<u64> =
                            if kani::any() { Some(kani::any()) } else { None };
                        // S8: a TRUSTED created_at (<= now + skew) decides;
                        // the mtime is ignored.
                        if let Some(c) = created_at_trusted {
                            kani::assert(
                                stale
                                    == reservation_is_stale(Some(c), alt_mtime, now, H4X_TTL_SECS),
                                "H4b/S8: a trusted created_at decides staleness; the mtime is ignored",
                            );
                        }
                        // S8: with created_at absent, a trusted-range mtime is
                        // judged exactly as a created_at of that value.
                        if slot.created_at.is_none()
                            && let Some(m) = slot.mtime
                            && m <= skew_bound
                        {
                            kani::assert(
                                stale == reservation_is_stale(Some(m), None, now, H4X_TTL_SECS),
                                "H4b/S8: with created_at absent the mtime is judged exactly as a created_at of that value",
                            );
                        }
                        // S10(a): age UNKNOWN (no usable created_at, no
                        // mtime) => never reclaimed (retention direction).
                        if created_at_trusted.is_none() && slot.mtime.is_none() {
                            kani::assert(
                                !stale,
                                "H4b/S10: a reservation of unknown age (no usable created_at, no mtime) is never reclaimed",
                            );
                            unknown_age_retained = true;
                        }
                        // S10(b): a created_at beyond now + 300 s is untrusted
                        // and falls back to the mtime (no clamp).
                        if created_at_beyond_skew {
                            kani::assert(
                                stale == reservation_is_stale(None, slot.mtime, now, H4X_TTL_SECS),
                                "H4b/S10: a created_at beyond the skew tolerance falls back to the mtime",
                            );
                            if stale {
                                future_created_at_fell_back_to_mtime = true;
                            }
                        }
                        // S10(c): a future created_at within the skew ages
                        // as 0 => never stale.
                        if let Some(c) = created_at_trusted
                            && c > now
                        {
                            kani::assert(
                                !stale,
                                "H4b/S10: a future created_at within the skew tolerance ages as 0 (never stale)",
                            );
                            within_skew_future_retained = true;
                        }
                        if stale {
                            kani::assert(
                                writers[i].phase == H4xWriterPhase::Idle,
                                "H4b/S8: the drain's TTL GC never reclaims a live writer's reservation",
                            );
                            if created_at_trusted.is_some() {
                                stale_gc_by_created_at = true;
                            } else {
                                stale_gc_by_mtime_fallback = true;
                            }
                            slot.present = false;
                        }
                    }
                }
            }
            // ---- Coordinator crash: lock released, state left -------------
            9 => {
                coord = H4xCoord::Absent;
            }
            // ---- Drain timeout: gate OPEN, abort (no txn yet) -------------
            10 => {
                if coord == H4xCoord::Draining {
                    gate = BcIndexAdmissionGateState::Open;
                    coord = H4xCoord::Absent;
                }
            }
            // ---- Clock tick (TTL assumption: live writers stay fresh) -----
            _ => {
                let delta: u64 = kani::any();
                kani::assume(delta >= 1 && delta <= 2 * H4X_TTL_SECS);
                let later = now + delta;
                let mut live_writers_stay_fresh = true;
                for (i, slot) in res.iter().enumerate() {
                    if writers[i].phase != H4xWriterPhase::Idle
                        && slot.present
                        && reservation_is_stale(slot.created_at, slot.mtime, later, H4X_TTL_SECS)
                    {
                        live_writers_stay_fresh = false;
                    }
                }
                if live_writers_stay_fresh {
                    now = later;
                }
            }
        }

        // ---- S1 / INV-GATE-TXN after every step (inductive) --------------
        let s1_now = !(gate == BcIndexAdmissionGateState::Open && h4x_txn_live(txn));
        if s1_holds {
            kani::assert(
                s1_now,
                "H4b/S1 INV-GATE-TXN (inductive): once the gate is never-OPEN-while-live it stays so",
            );
        }
        if lost_gate_file_initially && s1_now {
            lost_gate_state_healed = true;
        }
        s1_holds = s1_now;
        for (i, wr) in writers.iter().enumerate() {
            if wr.phase == H4xWriterPhase::Admitted {
                kani::assert(
                    res[i].present && namespace_exists,
                    "H4b/S9: every admitted writer has a reservation inside an existing namespace, visible to a subsequent drain",
                );
                kani::assert(
                    !h4x_txn_live(txn),
                    "H4b/S1 INV-GATE-TXN: no writer is admitted while a txn is STAGING/COMMITTING",
                );
                kani::assert(
                    gate != BcIndexAdmissionGateState::Locked,
                    "H4b/S1: no writer is admitted once the gate is LOCKED (the drain waited for quiescence)",
                );
            }
        }
    }

    // ---- Non-vacuity witnesses -------------------------------------------
    kani::cover!(ever_admitted, "H4b non-vacuity: a writer is admitted");
    kani::cover!(
        drain_after_first_ever_admission,
        "H4b/S9 non-vacuity: first-ever admission on a never-migrated project, then the coordinator's DRAINING flip"
    );
    kani::cover!(
        blocked_by_txn_half_of_dual_check,
        "H4b non-vacuity: a writer that read OPEN was refused by the txn half of the dual check"
    );
    kani::cover!(
        lost_gate_state_healed,
        "H4b non-vacuity: a lost-gate-file state (OPEN beside a live txn) is resolved"
    );
    kani::cover!(
        admitted_after_selfheal,
        "H4b non-vacuity: a writer is admitted after a step-3.5 self-heal"
    );
    kani::cover!(
        snapshot_after_admission,
        "H4b non-vacuity: the coordinator reaches the snapshot after a writer was admitted and released (Dekker interleave)"
    );
    kani::cover!(
        coordinator_waited,
        "H4b non-vacuity: C2 observed a reservation and waited"
    );
    kani::cover!(
        released_on_block,
        "H4b non-vacuity: release-on-block removed an admitter's own reservation"
    );
    kani::cover!(
        blocked_by_draining_observation,
        "H4b non-vacuity: a writer that observed DRAINING was blocked"
    );
    kani::cover!(
        stale_gc_by_created_at,
        "H4b non-vacuity: the drain GC reclaimed a reservation stale by created_at"
    );
    kani::cover!(
        stale_gc_by_mtime_fallback,
        "H4b non-vacuity: the drain GC reclaimed a reservation stale by the mtime fallback"
    );
    kani::cover!(
        unknown_age_retained,
        "H4b/S10 non-vacuity: the drain GC met a reservation of unknown age and retained it"
    );
    kani::cover!(
        future_created_at_fell_back_to_mtime,
        "H4b/S10 non-vacuity: a created_at beyond the skew fell back to a stale mtime and was reclaimed"
    );
    kani::cover!(
        within_skew_future_retained,
        "H4b/S10 non-vacuity: a future created_at within the skew aged as 0 and was retained"
    );
    kani::cover!(
        plan_reopen,
        "H4b non-vacuity: Branch A reopened a stuck gate"
    );
    kani::cover!(
        plan_abort_null,
        "H4b non-vacuity: Branch B aborted a null-generation STAGING txn"
    );
    kani::cover!(
        plan_finalize,
        "H4b non-vacuity: Branch C finalized a verified COMMITTING txn"
    );
    kani::cover!(
        plan_mismatch,
        "H4b non-vacuity: Branch C failed closed on a completion-record mismatch"
    );
    kani::cover!(
        plan_foreign,
        "H4b non-vacuity: Branch C refused a txn whose migration_id is not in K"
    );
    kani::cover!(
        plan_live_coordinator,
        "H4b non-vacuity: reconciliation found a live coordinator (EWOULDBLOCK)"
    );
}

// ===========================================================================
// In-memory two-namespace Fs model — the crash-atomicity substrate
// (research §1c; design §1.3). Small FileId enum (not PathBuf), Content as
// a generation tag (one byte, not real bytes), live/durable namespaces,
// crash() = live<-durable collapse, rename/pointer_swap as atomic-in-live
// axioms, fsync_file promotes content, fsync_dir promotes the directory's
// entries (name/existence bindings).
// ===========================================================================

/// Bounded file identity — collapses the PathBuf state space to a handful
/// of slots (research §1c/§1d).
#[derive(Clone, Copy, PartialEq, Eq)]
enum FileId {
    CurrentJson,
    CurrentTmp,
    CompletedJson,
    StagingShard,
    CanonicalShard,
    IntentLog,
    Other,
}

const FILE_SLOTS: usize = 7;

fn file_id_index(id: FileId) -> usize {
    match id {
        FileId::CurrentJson => 0,
        FileId::CurrentTmp => 1,
        FileId::CompletedJson => 2,
        FileId::StagingShard => 3,
        FileId::CanonicalShard => 4,
        FileId::IntentLog => 5,
        FileId::Other => 6,
    }
}

/// Concrete path -> FileId mapping. The harness uses SINGLE-CHARACTER
/// concrete path constants (`Path::new("c")` etc.) deliberately: `Path`
/// component parsing + UTF-8 validation are byte-length loops, so a long
/// path would force an unwinding bound proportional to its string length
/// (research §1d state-explosion guidance). One-byte names keep those loops
/// at ~1 iteration. The comparison is fully concrete (no symbolic strings).
fn file_id_of(path: &Path) -> FileId {
    match path.as_os_str().to_str() {
        Some("c") => FileId::CurrentJson,
        Some("t") => FileId::CurrentTmp,
        Some("p") => FileId::CompletedJson,
        Some("s") => FileId::StagingShard,
        Some("n") => FileId::CanonicalShard,
        Some("i") => FileId::IntentLog,
        _ => FileId::Other,
    }
}

/// One abstract inode: its content generation tag in each namespace
/// (`None` = the file does not exist in that namespace).
#[derive(Clone, Copy)]
struct Slot {
    live: Option<u8>,
    durable: Option<u8>,
}

impl Slot {
    const EMPTY: Slot = Slot {
        live: None,
        durable: None,
    };
}

/// In-memory two-namespace filesystem model implementing the production
/// [`Fs`](super::migration_fs::Fs) trait. `&self` methods mutate through a
/// `RefCell` (the trait takes `&self` because the production `StdFs` is a
/// zero-sized handle to the real OS).
struct InMemoryFs {
    slots: RefCell<[Slot; FILE_SLOTS]>,
}

impl InMemoryFs {
    fn new() -> Self {
        InMemoryFs {
            slots: RefCell::new([Slot::EMPTY; FILE_SLOTS]),
        }
    }

    /// Seed a file as fully durable (exists in both namespaces) — used to
    /// establish the pre-migration OLD committed state.
    fn seed_durable(&self, id: FileId, gen_tag: u8) {
        let mut slots = self.slots.borrow_mut();
        slots[file_id_index(id)] = Slot {
            live: Some(gen_tag),
            durable: Some(gen_tag),
        };
    }

    /// Power-loss crash: everything only in `live` is lost; the process
    /// restarts observing exactly the `durable` namespace (research §1c).
    fn crash(&self) {
        let mut slots = self.slots.borrow_mut();
        for slot in slots.iter_mut() {
            slot.live = slot.durable;
        }
    }

    /// What a restarted process observes for `id` (its post-crash `live`,
    /// which crash() has set equal to `durable`).
    fn observe(&self, id: FileId) -> Option<u8> {
        self.slots.borrow()[file_id_index(id)].live
    }
}

impl Fs for InMemoryFs {
    fn write_temp(&self, path: &Path, content: &[u8]) -> Result<(), BcIndexMigrationError> {
        // Abstract model: content is a one-byte generation tag; the write
        // lands in `live` only (NOT durable until fsync_file) — a
        // conservative SUPERSET of production's bundled durable write
        // (migration_fs.rs granularity note), never unsound.
        let gen_tag = content.first().copied().unwrap_or(0);
        let mut slots = self.slots.borrow_mut();
        slots[file_id_index(file_id_of(path))].live = Some(gen_tag);
        Ok(())
    }

    fn fsync_file(&self, path: &Path) -> Result<(), BcIndexMigrationError> {
        // Promote CONTENT live -> durable for this file.
        let mut slots = self.slots.borrow_mut();
        let slot = &mut slots[file_id_index(file_id_of(path))];
        slot.durable = slot.live;
        Ok(())
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), BcIndexMigrationError> {
        // Atomic namespace move within LIVE (POSIX rename atomicity axiom);
        // NOT durable until a subsequent fsync_dir.
        let mut slots = self.slots.borrow_mut();
        let from_i = file_id_index(file_id_of(from));
        let to_i = file_id_index(file_id_of(to));
        slots[to_i].live = slots[from_i].live;
        slots[from_i].live = None;
        Ok(())
    }

    fn fsync_dir(&self, _dir: &Path) -> Result<(), BcIndexMigrationError> {
        // Promote the directory's entry bindings (name/existence + the
        // content reachable through them) live -> durable. This is the
        // barrier that makes a preceding rename/pointer_swap durable
        // (research §1c). Since all modeled files live in one directory,
        // this promotes every slot.
        let mut slots = self.slots.borrow_mut();
        for slot in slots.iter_mut() {
            slot.durable = slot.live;
        }
        Ok(())
    }

    fn read(&self, path: &Path) -> Result<Option<Vec<u8>>, BcIndexMigrationError> {
        // Reads observe the LIVE namespace.
        Ok(self
            .slots
            .borrow()
            .get(file_id_index(file_id_of(path)))
            .and_then(|s| s.live)
            .map(|g| vec![g]))
    }

    fn exists(&self, path: &Path) -> bool {
        self.slots.borrow()[file_id_index(file_id_of(path))]
            .live
            .is_some()
    }

    fn pointer_swap(&self, tmp: &Path, target: &Path) -> Result<(), BcIndexMigrationError> {
        // Same underlying atomic-rename axiom as `rename` (the distinction
        // is the caller's protocol role — the SOLE commit point).
        self.rename(tmp, target)
    }

    fn remove(&self, path: &Path) -> Result<(), BcIndexMigrationError> {
        let mut slots = self.slots.borrow_mut();
        slots[file_id_index(file_id_of(path))].live = None;
        Ok(())
    }

    fn append(&self, path: &Path, content: &[u8]) -> Result<(), BcIndexMigrationError> {
        // The intent-log append is modeled at generation-tag granularity:
        // the latest tag written to `live`, durable only after its own
        // fsync (production bundles write+fsync; the model keeps them
        // separable — a conservative superset).
        let gen_tag = content.first().copied().unwrap_or(0);
        let mut slots = self.slots.borrow_mut();
        slots[file_id_index(file_id_of(path))].live = Some(gen_tag);
        Ok(())
    }
}

// ===========================================================================
// Harness 5 — BOUNDED CRASH-TRACE ATOMICITY (research §5a-5; design §6.2 item 5)
//
// Over the two-namespace InMemoryFs, execute the durable pointer-swap
// publish protocol (the sole commit point) with a NONDETERMINISTIC crash
// index and assert that, after crash + recovery-read, the CURRENT pointer
// resolves to exactly OLD or NEW — never torn, never missing. This covers
// the write-reordering / partial-durability interleavings a process-kill
// fault-injection test cannot reach (research §5b): a crash before the
// commit's dir-fsync loses the swap (CURRENT stays OLD); a crash after it
// keeps the swap (CURRENT is NEW); no intermediate/torn value is reachable.
// ===========================================================================

const GEN_OLD: u8 = 10;
const GEN_NEW: u8 = 20;

// unwind(12): the InMemoryFs's `crash()`/`fsync_dir()` iterate the
// FILE_SLOTS (7) array, and single-character path constants keep the
// `Path`-parse/UTF-8-validation loops in `file_id_of` at ~1 iteration; 12
// comfortably clears every unwinding assertion (the abstract u8 model is
// cheap, so a generous bound has no cost).
#[kani::proof]
#[kani::unwind(12)]
fn proof_obl1_h5_pointer_swap_crash_atomicity() {
    let fs = InMemoryFs::new();
    // Single-character paths (see file_id_of): c=CURRENT, t=CURRENT.tmp,
    // i=intent log, s=staging shard. Long paths would blow the unwinding
    // bound (Path parsing is a byte-length loop).
    let current = Path::new("c");
    let current_tmp = Path::new("t");
    let intent = Path::new("i");
    let staging = Path::new("s");

    // Pre-migration OLD committed state is durable.
    fs.seed_durable(FileId::CurrentJson, GEN_OLD);

    // The ordered durable publish protocol (ADR-052 §7b/§7c, WAL-ordered:
    // intent durable BEFORE the commit): 6 ops. `crash_at` truncates the
    // sequence at a nondeterministic point, then crash() collapses live to
    // durable (loses anything not yet fsynced).
    let crash_at: usize = kani::any();
    kani::assume(crash_at <= 6);

    // op 0: stage the NEW shard content (live)
    if crash_at > 0 {
        let _ = fs.write_temp(staging, &[GEN_NEW]);
    }
    // op 1: fsync the staged content (durable)
    if crash_at > 1 {
        let _ = fs.fsync_file(staging);
    }
    // op 2: WAL boundary — intent record durable BEFORE any commit/rename
    if crash_at > 2 {
        let _ = fs.append(intent, &[GEN_NEW]);
        let _ = fs.fsync_file(intent);
    }
    // op 3: write the new CURRENT pointer to a temp (live)
    if crash_at > 3 {
        let _ = fs.write_temp(current_tmp, &[GEN_NEW]);
        let _ = fs.fsync_file(current_tmp);
    }
    // op 4: the SOLE commit point — atomic pointer swap (live only)
    if crash_at > 4 {
        let _ = fs.pointer_swap(current_tmp, current);
    }
    // op 5: dir-fsync makes the commit durable
    if crash_at > 5 {
        let _ = fs.fsync_dir(Path::new("d"));
    }

    // Power loss at the chosen point.
    fs.crash();

    // Recovery observes CURRENT. It must resolve to exactly OLD or NEW.
    let observed = fs.observe(FileId::CurrentJson);
    kani::assert(
        observed == Some(GEN_OLD) || observed == Some(GEN_NEW),
        "H5: after a crash at ANY point, CURRENT resolves to exactly OLD or NEW (never torn)",
    );
    // CURRENT is never lost/missing across the whole protocol (the swap
    // replaces it atomically; it always exists in the durable namespace).
    kani::assert(
        observed.is_some(),
        "H5: CURRENT is never missing after a crash (atomic replace, never unlink-then-write)",
    );

    // WAL-ordering (finding #4): if the commit became durable (CURRENT ==
    // NEW after crash), the intent record for NEW was already durable
    // (it precedes the commit in the protocol), so recovery always has the
    // record explaining the published state — never an unexplained mutation.
    kani::cover!(
        observed == Some(GEN_OLD),
        "H5 non-vacuity: a crash leaves CURRENT at OLD"
    );
    kani::cover!(
        observed == Some(GEN_NEW),
        "H5 non-vacuity: a crash after the durable commit leaves CURRENT at NEW"
    );
    if observed == Some(GEN_NEW) {
        kani::assert(
            fs.observe(FileId::IntentLog) == Some(GEN_NEW),
            "H5: a durable NEW commit always has its durable intent record (WAL ordering, finding #4)",
        );
    }
}

// ===========================================================================
// Harness 6 — RECOVERY IDEMPOTENCE (research §5a-6; design §6.2 item 6)
//
// The §5a-6 property is "applying recover()+action twice reaches the same
// terminal state as once". Its core is the terminal FIXED POINT: once the
// permanent completed.json record is present, recover() returns
// AlreadyMigrated for EVERY txn-record/gen/manifest shape (ADR-052 §7c step
// 8 — completed.json's mere presence is checked FIRST, unconditionally,
// before any txn-record inspection), and every subsequent recovery pass
// returns that SAME terminal decision. So a completed migration
// re-recovered never triggers a second destructive action, and any forward
// action whose completion durably wrote completed.json converges to
// AlreadyMigrated on the next pass rather than re-doing (that a forward
// action is itself always a DEFINED decision is proven by harness 1's
// totality; this harness proves the completion is an ABSORBING fixed point).
//
// Records are generated with arbitrary state/generation shape to prove the
// absorption holds regardless of what txn records remain on disk. The
// assertions compare against the unit variant `AlreadyMigrated`, and the
// `completed`-present recover() calls short-circuit before `classify_txn_
// records` even runs — so this harness stays CBMC-cheap (no full structural
// `String`/`Vec` equality of two symbolic heap-bearing decision values; a
// determinism check over those is not a model-checking-worthy property,
// since `recover()` is a pure `fn` of its `&`-params with no interior
// mutability or globals).
// ===========================================================================

#[kani::proof]
#[kani::unwind(4)]
fn proof_obl1_h6_recovery_idempotence() {
    let records = any_bounded_records();
    let gen_dir_exists: bool = kani::any();
    let manifest_status = any_manifest_status();
    let completed = CompletedMigrationRecord {
        generation_id: String::new(),
        txn_id: String::new(),
        completed_at: String::new(),
        canonical_paths_count: 1,
    };

    // Terminal fixed point: completed.json present => AlreadyMigrated for
    // EVERY record/gen/manifest shape (the absorbing recovery state).
    let first = recover(
        &records,
        None,
        Some(&completed),
        gen_dir_exists,
        manifest_status,
    );
    kani::assert(
        first == RecoveryDecision::AlreadyMigrated,
        "H6: completed.json present => AlreadyMigrated for every record/gen/manifest shape",
    );

    // Idempotence: a SECOND recovery pass over the same completed state is a
    // stable no-op — still AlreadyMigrated, never oscillating, never a
    // repeated destructive action. This is the §5a-6 fixed point: applying
    // recovery twice reaches the same terminal state as applying it once.
    let second = recover(
        &records,
        None,
        Some(&completed),
        gen_dir_exists,
        manifest_status,
    );
    kani::assert(
        second == RecoveryDecision::AlreadyMigrated,
        "H6: re-running recovery on a completed migration stays AlreadyMigrated (idempotent)",
    );
    kani::cover!(
        records.iter().any(|r| state_is_live(r.state)),
        "H6 non-vacuity: absorption holds with a live txn record still on disk"
    );
    // Skip drop-glue modeling of the String-heavy records at scope end (see
    // h1/h2). Harmless here (h6 is already fast), kept for consistency.
    std::mem::forget(records);
    std::mem::forget(completed);
}

// ===========================================================================
// Harness 6b (VP-147 v1.18 extension) — TERMINAL-RECORD RECONCILIATION
// IDEMPOTENCE over the REAL pure cores (`decide_terminal_record_reconciliation`
// and the step-3.5 planner `plan_stale_gate_reconciliation` built on it).
//
// Apply the effect a decision names, then decide again over the post-state:
// a writing decision (Branch A reopen, Branch B abort-then-reopen, Branch C
// finalize-then-reopen) is followed by NothingToReconcile / NoOp — never a
// repeated destructive action; a non-writing decision (NoOp, mismatch,
// foreign refusal, live coordinator) is STABLE (the same decision again).
// The finalize effect modeled is the S-25.06 intended one (txn COMPLETED,
// then gate OPEN); this build's fail-closed seam writes nothing, which is
// the stable non-writing case. Also proves the planner's contract the
// effectful shell relies on: a txn-bearing plan is only ever produced for a
// LIVE txn.
// ===========================================================================

#[kani::proof]
fn proof_obl1_h6_terminal_reconcile_idempotence() {
    let gate = any_gate_state();
    let i = any_terminal_reconcile_inputs();
    let gen_null: bool = kani::any();
    let live = optional_state_is_live(i.txn_state);

    // ---- The Branch C core ------------------------------------------------
    let d1 = decide_terminal_record_reconciliation(&i);
    let i_after_core = if d1 == TerminalReconcileDecision::FinalizeThenOpenGate {
        TerminalReconcileInputs {
            txn_state: Some(BcIndexMigrationTxnState::Completed),
            ..i
        }
    } else {
        i
    };
    let d2 = decide_terminal_record_reconciliation(&i_after_core);
    if d1 == TerminalReconcileDecision::FinalizeThenOpenGate {
        kani::assert(
            d2 == TerminalReconcileDecision::NoOp,
            "H6b: after a finalize the second decision is NoOp (never a second finalize)",
        );
    } else {
        kani::assert(
            d2 == d1,
            "H6b: a non-writing decision (NoOp / mismatch / foreign refusal) is stable on re-evaluation",
        );
    }

    // ---- The step-3.5 planner (Branches A/B/C) ----------------------------
    let p1 = plan_stale_gate_reconciliation(gate, &i, gen_null);
    if matches!(
        p1,
        StaleGateReconciliationPlan::AbortNullGenerationThenReopenGate
            | StaleGateReconciliationPlan::FailClosedMismatch
            | StaleGateReconciliationPlan::FinalizeThenOpenGate
            | StaleGateReconciliationPlan::RefuseForeignMigration
    ) {
        kani::assert(
            live && i.lock_acquired,
            "H6b: a txn-bearing plan is produced only for a live txn under the lock (shell contract)",
        );
    }
    // BC-1.18.011 v1.17 Precondition 6(d) row-3 caller contract: Branch B
    // applies IF AND ONLY IF the lock is held and the live txn is a known
    // migration's (`migration_id ∈ K`) STAGING with `generation_id = null`
    // and its own terminal record absent — with NO gate condition (an absent
    // gate file reads OPEN).
    kani::assert(
        (p1 == StaleGateReconciliationPlan::AbortNullGenerationThenReopenGate)
            == (i.lock_acquired
                && i.txn_state == Some(BcIndexMigrationTxnState::Staging)
                && gen_null
                && i.txn_migration_known
                && !i.record_present),
        "H6b: Branch B iff lock + known-migration null-generation STAGING + its terminal record absent (no gate condition)",
    );
    let (gate2, state2) = match p1 {
        StaleGateReconciliationPlan::ReopenGate => (BcIndexAdmissionGateState::Open, i.txn_state),
        StaleGateReconciliationPlan::AbortNullGenerationThenReopenGate => (
            BcIndexAdmissionGateState::Open,
            Some(BcIndexMigrationTxnState::Aborted),
        ),
        StaleGateReconciliationPlan::FinalizeThenOpenGate => (
            BcIndexAdmissionGateState::Open,
            Some(BcIndexMigrationTxnState::Completed),
        ),
        _ => (gate, i.txn_state),
    };
    let i2 = TerminalReconcileInputs {
        txn_state: state2,
        ..i
    };
    let p2 = plan_stale_gate_reconciliation(gate2, &i2, gen_null);
    let p1_writes = matches!(
        p1,
        StaleGateReconciliationPlan::ReopenGate
            | StaleGateReconciliationPlan::AbortNullGenerationThenReopenGate
            | StaleGateReconciliationPlan::FinalizeThenOpenGate
    );
    if p1_writes {
        kani::assert(
            p2 == StaleGateReconciliationPlan::NothingToReconcile,
            "H6b: after a self-heal write the second plan is NothingToReconcile (no repeated destructive action)",
        );
    } else {
        kani::assert(
            p2 == p1,
            "H6b: a non-writing plan (nothing / mismatch / foreign / live coordinator) is stable",
        );
    }

    // Non-vacuity.
    kani::cover!(
        d1 == TerminalReconcileDecision::FinalizeThenOpenGate,
        "H6b non-vacuity: finalize then NoOp"
    );
    kani::cover!(
        d1 == TerminalReconcileDecision::FailClosedMismatch,
        "H6b non-vacuity: stable mismatch"
    );
    kani::cover!(
        p1 == StaleGateReconciliationPlan::ReopenGate,
        "H6b non-vacuity: Branch A then nothing"
    );
    kani::cover!(
        p1 == StaleGateReconciliationPlan::AbortNullGenerationThenReopenGate,
        "H6b non-vacuity: Branch B then nothing"
    );
    kani::cover!(
        p1 == StaleGateReconciliationPlan::AbortNullGenerationThenReopenGate
            && gate == BcIndexAdmissionGateState::Open,
        "H6b non-vacuity: Branch B with the gate reading OPEN (absent gate file)"
    );
    kani::cover!(
        p1 == StaleGateReconciliationPlan::FinalizeThenOpenGate,
        "H6b non-vacuity: Branch C finalize then nothing"
    );
    kani::cover!(
        p1 == StaleGateReconciliationPlan::RefuseForeignMigration,
        "H6b non-vacuity: stable row-2 refusal (migration_id not in K)"
    );
    kani::cover!(
        p1 == StaleGateReconciliationPlan::LiveCoordinator,
        "H6b non-vacuity: stable live coordinator"
    );
}
