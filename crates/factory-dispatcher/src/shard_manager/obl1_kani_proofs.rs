//! OBL-1 (D-1232-OBL-1) Kani model-checking harnesses for the B2 BC-INDEX
//! shard-migration crash-recovery state machine (BC-1.18.011; ADR-052
//! §Decision 4e/5a/7a/7b/7c).
//!
//! # What this module is
//!
//! Seven `#[kani::proof]` harnesses covering research report §5a items 1-6
//! (item 3 — state-machine totality + inductive invariant — is split into
//! an inductive-step proof and a bounded-sequence proof), mapped onto the
//! OBL-1 refactor design §6.2, plus the in-memory two-namespace
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
    BcIndexMigrationTxnState, CompletedMigrationRecord, ManifestStatus, QuarantineReason,
    RecoveryDecision, is_bc_index_admission_open, recover,
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
    // Skip drop-glue modeling of the String-heavy records/decision at scope
    // end: CBMC unrolls the Vec/String destructors, which dominates solve
    // time here for zero verification value (a harness never really runs).
    std::mem::forget(records);
    std::mem::forget(decision);
    std::mem::forget(completed);
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
}

fn any_txn_event() -> TxnEvent {
    let v: u8 = kani::any();
    kani::assume(v < 4);
    match v {
        0 => TxnEvent::AssignGeneration,
        1 => TxnEvent::BeginCommitting,
        2 => TxnEvent::Complete,
        _ => TxnEvent::Abort,
    }
}

/// The ADR-052 §7a legal transition relation. TOTAL by construction: every
/// (state, event) pair maps to a defined next state; an event that is not
/// legal from the current state is a no-op (the state is unchanged), never
/// a panic and never a backwards move. Terminal states (Completed/Aborted)
/// are absorbing.
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
        s = next;
    }
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
    }
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
    // Skip drop-glue modeling of the String-heavy records at scope end (see
    // h1/h2). Harmless here (h6 is already fast), kept for consistency.
    std::mem::forget(records);
    std::mem::forget(completed);
}
