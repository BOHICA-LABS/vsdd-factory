//! Mechanism-A (`backfill-append-logs`) Kani model-checking harnesses for the
//! S-25.06 / BC-1.18.013 crash-recovery guarantee (ADR-052 §Decision 7a/7b/7c,
//! §Decision 12).
//!
//! # What this module is
//!
//! Seven `#[kani::proof]` harnesses for six obligations (`proof_obl_a1`..
//! `proof_obl_a6`; obligation a3 is split into an inductive-step proof and a
//! bounded-sequence proof). They are the structural sibling of the B2 suite in
//! [`super::obl1_kani_proofs`](../obl1_kani_proofs/index.html), retyped onto
//! mechanism-A's own domain. The harnesses are gated behind `#[cfg(kani)]` and
//! compile ONLY under `cargo kani` (`cfg(kani)` is the allowlisted expected-cfg
//! in the workspace `Cargo.toml` `[workspace.lints.rust.unexpected_cfgs]`
//! block), so a normal `cargo build`/`test`/`clippy` never sees them.
//!
//! # Which production code each harness exercises
//!
//! Every pure decision a harness reasons about is the REAL production function,
//! never a harness-local copy that could drift from it:
//!
//! | Harness | Production code under proof |
//! |---------|------------------------------|
//! | a1, a2, a6 | [`decide_append_log_recovery`](super::decide_append_log_recovery) |
//! | a3 (both) | [`append_log_txn_transition`](super::append_log_txn_transition) — the single legal-transition relation every production txn-state change goes through |
//! | a4 | [`is_append_log_admission_open`](super::is_append_log_admission_open) (the rule `executor::append_log_backfill_admission_precheck` applies) and `append_log_txn_transition` |
//! | a5 | `decide_append_log_recovery` + `append_log_txn_transition`, driven over an explicit crash model of `run_backfill_append_logs`'s durable-write order |
//!
//! a5's operation ORDER is necessarily a model of `run_backfill_append_logs` /
//! `finish_append_log_migration` (those functions are bound to `std::fs`,
//! `chrono` and SHA-256 and cannot be symbolically executed); the order is
//! transcribed step-for-step, with the production call named on every step,
//! so a reviewer can diff the two directly.
//!
//! # Kani-generated inputs (String-field convention)
//!
//! [`AppendLogIntentLogRecord`](super::AppendLogIntentLogRecord) carries
//! `String`/`PathBuf` fields that do not implement `kani::Arbitrary`, so —
//! following this crate's `partition.rs`/`aggregator.rs`/`obl1_kani_proofs.rs`
//! convention — the record is built with EMPTY path/id fields and the only
//! decision-relevant fields (`expected_post_hash`, `expected_pre_state`) are
//! SINGLE-CHARACTER content-hash stand-ins drawn from `{absent, "P", "R",
//! "F"}` (`"P"` = post-image, `"R"` = pre-image, `"F"` = a foreign/stale
//! hash matching neither). This is SOUND and COMPLETE for
//! `decide_append_log_recovery`: its only comparisons are string equalities
//! between the two on-disk hashes and the record's two expectations, so the
//! equal / pre-equal / neither trichotomy that set realizes reaches every
//! branch (each branch's reachability is checked with `kani::cover!`).
//!
//! # Vacuity guard
//!
//! Every harness carries `kani::cover!` statements for the input classes its
//! assertions are conditioned on, so an over-constraining `kani::assume` or an
//! unreachable branch shows up as an UNSATISFIABLE cover instead of a silently
//! vacuous "VERIFICATION SUCCESSFUL". The `kani-mechanism-a` CI job fails on
//! any UNSATISFIABLE / UNREACHABLE cover.
//!
//! # Solve-time note (`mem::forget`)
//!
//! The `FailClosed` arm of `decide_append_log_recovery` builds a `String` via
//! `format!`, and the record is itself `String`-heavy. CBMC's dominant cost is
//! the DROP GLUE at scope end, so each decision-based harness
//! `std::mem::forget`s its record/decision values: a Kani harness never really
//! runs, so skipping destructor modeling is sound.
//!
//! # What Kani proves here vs. what it cannot
//!
//! Kani does not model I/O (no semantics for `rename(2)`, `F_FULLFSYNC`, the
//! page cache, torn sectors or power loss). a5 therefore proves its property
//! over the explicit crash model [`CrashFs`], whose axioms are documented on the
//! type; refinement of those axioms against real filesystems is the separate
//! obligation discharged by the `fail`-based fault-injection integration suite.
//! The defensible claim form is: *"under crash model M, for every crash point,
//! every partial-persistence outcome, every pre-commit abort point and every
//! interleaved foreign write, recovery never publishes a torn pointer, never
//! performs a canonical move without a durable COMMITTING txn record and its
//! durable intent record, and never mistakes a merely-existing canonical path
//! for a completed move."*

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use super::migration_fs::Fs;
use super::{
    AppendLogIntentLogRecord, AppendLogIntentLogRecordType, AppendLogIntentLogRecoveryDecision,
    AppendLogMigrationTxnState, AppendLogTxnEvent, BcIndexMigrationError,
    append_log_txn_transition, decide_append_log_recovery, is_append_log_admission_open,
};

// ---------------------------------------------------------------------------
// Symbolic-value helpers over mechanism-A's small finite domains.
// ---------------------------------------------------------------------------

/// The record's post-image content hash stand-in.
const HASH_POST: &str = "P";
/// The record's pre-image content hash stand-in.
const HASH_PRE: &str = "R";
/// A foreign/stale content hash matching NEITHER expectation.
const HASH_FOREIGN: &str = "F";
/// The hash of TORN content (harness a5's crash model only).
const HASH_TORN: &str = "T";

/// A nondeterministic on-disk hash from `{absent, post, pre, foreign}`. Uses
/// `assume(v < 4)` rather than `any() % 4` (the modulo forces CBMC to model
/// all 256 `u8` values).
fn any_hash() -> Option<&'static str> {
    let v: u8 = kani::any();
    kani::assume(v < 4);
    match v {
        0 => None,
        1 => Some(HASH_POST),
        2 => Some(HASH_PRE),
        _ => Some(HASH_FOREIGN),
    }
}

/// Build one intent-log record with the given `expected_pre_state` (`None` =
/// the `missing` sentinel: the canonical target did not exist at intent-write
/// time) and the fixed post-image hash. All path/id/timestamp fields are EMPTY:
/// `decide_append_log_recovery` never reads them.
fn make_append_log_record(
    record_type: AppendLogIntentLogRecordType,
    expected_pre_state: Option<&'static str>,
) -> AppendLogIntentLogRecord {
    AppendLogIntentLogRecord {
        txn_id: String::new(),
        fencing_generation: 1,
        record_type,
        target_canonical: PathBuf::new(),
        staging_path: PathBuf::new(),
        expected_post_hash: String::from(HASH_POST),
        expected_pre_state: expected_pre_state.map(String::from),
        timestamp_utc: String::new(),
        record_checksum: String::new(),
    }
}

/// A nondeterministic `expected_pre_state` from `{missing, pre-image}` — the
/// two values production can record (`append_intent_records_for_append_log_
/// pending_moves` hashes whatever the canonical path holds at intent time;
/// the `"R"` stand-in names "that content", whatever it was).
fn any_pre_state() -> Option<&'static str> {
    if kani::any() { Some(HASH_PRE) } else { None }
}

/// Stub for `std::fmt::format` (the function every `format!` expands to),
/// installed on the decision-based harnesses with `#[kani::stub]` (run under
/// `cargo kani -Z stubbing`). `decide_append_log_recovery`'s `FailClosed`
/// arms build their human-readable `reason` with `format!`, including a
/// `{:?}` of an `Option<&str>` whose `escape_debug` path drags the Unicode
/// printable/grapheme tables into the CBMC formula — unstubbed, harness a1
/// alone ran for over 16 minutes at 12 GB RSS without finishing. SOUND for
/// every property in this module: no assertion inspects the `reason`
/// string's content (all match on the decision discriminant), and the
/// stubbed-out code is `core::fmt` machinery formatting `&str`/`String`/
/// `Option<&str>` values, which cannot panic. The decision LOGIC — every
/// comparison and every branch of `decide_append_log_recovery` — is still
/// the real production code.
fn stub_format(_args: std::fmt::Arguments<'_>) -> String {
    String::new()
}

// ===========================================================================
// Harness a1 — decide_append_log_recovery() TOTALITY + exact decision table
//
// A defined decision for EVERY (canonical_hash, staging_hash, record present/
// absent, expected_pre_state) combination: no panic, no unreachable!(), no
// arithmetic fault (Kani's automatic checks). The explicit assertions pin the
// full decision table so a regression to the existence-only oracle, or any
// other row change, fails this proof.
// ===========================================================================

#[kani::proof]
#[kani::unwind(4)]
#[kani::stub(std::fmt::format, stub_format)]
fn proof_obl_a1_recover_totality() {
    let record_present: bool = kani::any();
    let pre = any_pre_state();
    let record = make_append_log_record(AppendLogIntentLogRecordType::Intent, pre);
    let record_ref = if record_present { Some(&record) } else { None };

    let canonical_hash = any_hash();
    let staging_hash = any_hash();

    let decision = decide_append_log_recovery(canonical_hash, staging_hash, record_ref);

    let post = Some(HASH_POST);
    if record_ref.is_none() {
        kani::assert(
            matches!(
                decision,
                AppendLogIntentLogRecoveryDecision::FailClosed { .. }
            ),
            "a1: no intent-log record => FailClosed (torn/absent, never a silent success)",
        );
    } else if canonical_hash == post {
        kani::assert(
            decision == AppendLogIntentLogRecoveryDecision::TreatDone,
            "a1: canonical content == expected_post_hash => TreatDone (checked FIRST)",
        );
    } else if staging_hash == post && canonical_hash == pre {
        kani::assert(
            decision == AppendLogIntentLogRecoveryDecision::RedoRename,
            "a1: staging==post && canonical==expected_pre => RedoRename",
        );
    } else {
        kani::assert(
            matches!(
                decision,
                AppendLogIntentLogRecoveryDecision::FailClosed { .. }
            ),
            "a1: no safe recovery row matches => FailClosed (never a silent success)",
        );
    }

    // Reachability of every row (vacuity guard), including each of the three
    // distinct FailClosed arms of the production function.
    kani::cover!(record_ref.is_none(), "a1 cover: record absent");
    kani::cover!(
        decision == AppendLogIntentLogRecoveryDecision::TreatDone,
        "a1 cover: TreatDone row"
    );
    kani::cover!(
        decision == AppendLogIntentLogRecoveryDecision::RedoRename && pre.is_none(),
        "a1 cover: RedoRename onto a missing canonical (new shard file)"
    );
    kani::cover!(
        decision == AppendLogIntentLogRecoveryDecision::RedoRename && pre.is_some(),
        "a1 cover: RedoRename onto the recorded pre-image"
    );
    kani::cover!(
        record_ref.is_some() && canonical_hash.is_none() && staging_hash.is_none(),
        "a1 cover: both-absent FailClosed arm"
    );
    kani::cover!(
        record_ref.is_some()
            && canonical_hash.is_some()
            && staging_hash.is_some()
            && matches!(
                decision,
                AppendLogIntentLogRecoveryDecision::FailClosed { .. }
            ),
        "a1 cover: both-present conflicting-content FailClosed arm"
    );
    kani::cover!(
        record_ref.is_some()
            && canonical_hash.is_some()
            && staging_hash.is_none()
            && matches!(
                decision,
                AppendLogIntentLogRecoveryDecision::FailClosed { .. }
            ),
        "a1 cover: generic catch-all FailClosed arm"
    );

    std::mem::forget(record);
    std::mem::forget(decision);
}

// ===========================================================================
// Harness a2 — recovery SAFETY PREDICATE / FAIL-CLOSED
//
// (a) RedoRename — the ONLY arm that leads its caller to rename over the
//     canonical path (the application-level RENAME_NOREPLACE analogue) — is
//     reached ONLY when staging content == expected_post_hash AND canonical
//     content == the recorded expected_pre_state exactly.
// (b) TreatDone is reached ONLY when canonical content == expected_post_hash.
// (c) A canonical that exists with non-post content is NEVER TreatDone — the
//     precise defect of the pre-fix existence-only oracle.
// ===========================================================================

#[kani::proof]
#[kani::unwind(4)]
#[kani::stub(std::fmt::format, stub_format)]
fn proof_obl_a2_recovery_safety_predicate() {
    let pre = any_pre_state();
    let record = make_append_log_record(AppendLogIntentLogRecordType::Intent, pre);
    let canonical_hash = any_hash();
    let staging_hash = any_hash();

    let decision = decide_append_log_recovery(canonical_hash, staging_hash, Some(&record));

    let post = Some(HASH_POST);
    match decision {
        AppendLogIntentLogRecoveryDecision::RedoRename => {
            kani::assert(
                staging_hash == post && canonical_hash == pre,
                "a2(a): RedoRename requires staging==expected_post AND canonical==expected_pre",
            );
        }
        AppendLogIntentLogRecoveryDecision::TreatDone => {
            kani::assert(
                canonical_hash == post,
                "a2(b): TreatDone requires canonical content == expected_post_hash",
            );
        }
        AppendLogIntentLogRecoveryDecision::FailClosed { .. } => {}
    }

    let canonical_exists_not_post = canonical_hash.is_some() && canonical_hash != post;
    if canonical_exists_not_post {
        kani::assert(
            decision != AppendLogIntentLogRecoveryDecision::TreatDone,
            "a2(c): canonical that exists with non-post content is never TreatDone",
        );
    }
    // RedoRename never fires over foreign destination content.
    if canonical_hash == Some(HASH_FOREIGN) {
        kani::assert(
            decision != AppendLogIntentLogRecoveryDecision::RedoRename,
            "a2(d): RedoRename never overwrites foreign canonical content",
        );
    }

    kani::cover!(
        canonical_exists_not_post && staging_hash == post,
        "a2 cover: existence-only oracle's mistaken 'already moved' input class"
    );
    kani::cover!(
        decision == AppendLogIntentLogRecoveryDecision::RedoRename,
        "a2 cover: RedoRename reachable"
    );
    kani::cover!(
        decision == AppendLogIntentLogRecoveryDecision::TreatDone,
        "a2 cover: TreatDone reachable"
    );

    std::mem::forget(record);
    std::mem::forget(decision);
}

// ===========================================================================
// Harness a3 — txn state-machine TOTALITY + INDUCTIVE INVARIANT
//
// Proves the REAL production relation `append_log_txn_transition` (every
// post-creation txn-state change in `run_backfill_append_logs` /
// `finish_append_log_migration` goes through it, via
// `advance_append_log_txn_state` or the abort-cleanup closure) is
// (a) TOTAL — defined and panic-free for every (state, event) pair — and
// (b) preserves BC-1.18.013 Invariant 3 ("no turning back"): every accepted
// transition strictly increases the progress rank, and terminal states accept
// no event. The one-step property is the inductive step; with the base case
// (records are created in Staging, rank 0) it covers UNBOUNDED sequences. The
// bounded companion replays arbitrary event sequences as production applies
// them (an accepted event updates the state; a rejected one is a hard error,
// leaving the persisted state unchanged).
// ===========================================================================

fn any_txn_state() -> AppendLogMigrationTxnState {
    let v: u8 = kani::any();
    kani::assume(v < 4);
    match v {
        0 => AppendLogMigrationTxnState::Staging,
        1 => AppendLogMigrationTxnState::Committing,
        2 => AppendLogMigrationTxnState::Completed,
        _ => AppendLogMigrationTxnState::Aborted,
    }
}

fn any_txn_event() -> AppendLogTxnEvent {
    let v: u8 = kani::any();
    kani::assume(v < 3);
    match v {
        0 => AppendLogTxnEvent::BeginCommitting,
        1 => AppendLogTxnEvent::Complete,
        _ => AppendLogTxnEvent::Abort,
    }
}

/// Progress rank: Staging < Committing < {Completed, Aborted}.
fn txn_rank(s: AppendLogMigrationTxnState) -> u8 {
    match s {
        AppendLogMigrationTxnState::Staging => 0,
        AppendLogMigrationTxnState::Committing => 1,
        AppendLogMigrationTxnState::Completed | AppendLogMigrationTxnState::Aborted => 2,
    }
}

fn txn_is_terminal(s: AppendLogMigrationTxnState) -> bool {
    matches!(
        s,
        AppendLogMigrationTxnState::Completed | AppendLogMigrationTxnState::Aborted
    )
}

#[kani::proof]
fn proof_obl_a3_transition_inductive_step() {
    let s = any_txn_state();
    let e = any_txn_event();
    let next = append_log_txn_transition(s, e);

    if let Some(n) = next {
        kani::assert(
            txn_rank(n) > txn_rank(s),
            "a3: every accepted txn transition strictly advances (no turning back, no self-loop)",
        );
    }
    if txn_is_terminal(s) {
        kani::assert(
            next.is_none(),
            "a3: a terminal txn state (COMPLETED/ABORTED) accepts no event",
        );
    }
    // COMMITTING can never be left for STAGING, and COMPLETED is reachable
    // only from COMMITTING (never skipping the pointer-swap commit point).
    kani::assert(
        next != Some(AppendLogMigrationTxnState::Staging),
        "a3: no event ever yields STAGING",
    );
    if next == Some(AppendLogMigrationTxnState::Completed) {
        kani::assert(
            s == AppendLogMigrationTxnState::Committing,
            "a3: COMPLETED is entered only from COMMITTING",
        );
    }

    // The four ADR-052 §7a legal edges are each accepted (the relation is not
    // vacuously "reject everything").
    kani::cover!(
        s == AppendLogMigrationTxnState::Staging
            && next == Some(AppendLogMigrationTxnState::Committing),
        "a3 cover: STAGING -> COMMITTING"
    );
    kani::cover!(
        s == AppendLogMigrationTxnState::Staging
            && next == Some(AppendLogMigrationTxnState::Aborted),
        "a3 cover: STAGING -> ABORTED"
    );
    kani::cover!(
        s == AppendLogMigrationTxnState::Committing
            && next == Some(AppendLogMigrationTxnState::Completed),
        "a3 cover: COMMITTING -> COMPLETED"
    );
    kani::cover!(
        s == AppendLogMigrationTxnState::Committing
            && next == Some(AppendLogMigrationTxnState::Aborted),
        "a3 cover: COMMITTING -> ABORTED"
    );
}

#[kani::proof]
#[kani::unwind(7)]
fn proof_obl_a3_transition_bounded_sequence() {
    let steps: usize = kani::any();
    kani::assume(steps <= 6);
    // Base case: production creates every txn record in Staging.
    let mut s = AppendLogMigrationTxnState::Staging;
    let mut reached_completed = false;
    let mut reached_aborted = false;
    for _ in 0..steps {
        let e = any_txn_event();
        let prev = s;
        if let Some(n) = append_log_txn_transition(s, e) {
            s = n;
        }
        kani::assert(
            txn_rank(s) >= txn_rank(prev),
            "a3-seq: progress rank monotonic across the whole sequence",
        );
        if txn_is_terminal(prev) {
            kani::assert(
                s == prev,
                "a3-seq: a terminal state stays terminal for the rest of the run",
            );
        }
        reached_completed |= s == AppendLogMigrationTxnState::Completed;
        reached_aborted |= s == AppendLogMigrationTxnState::Aborted;
    }
    kani::cover!(
        reached_completed,
        "a3-seq cover: COMPLETED reachable from Staging"
    );
    kani::cover!(
        reached_aborted,
        "a3-seq cover: ABORTED reachable from Staging"
    );
}

// ===========================================================================
// Harness a4 — ADMISSION-GATE invariant via a finite scheduler
//
// Production's mechanism-A admission gate is derived ENTIRELY from the active
// txn record's state (there is no gate-state file and no drain step for
// `backfill-append-logs`; see `run_backfill_append_logs`'s abort-cleanup
// comment). The scheduler below therefore models exactly that: writer
// admission attempts are decided by the REAL `is_append_log_admission_open`
// over the current txn state, and the migration's lifecycle is driven by the
// REAL `append_log_txn_transition`. Invariants asserted at EVERY admission
// decision, over every reachable lifecycle state:
//
//   * INV-GATE-TXN (refusal half): no write is ever admitted while the txn is
//     STAGING/COMMITTING;
//   * no self-lock: once the txn is terminal (or absent), every write is
//     admitted again.
//
// NOT claimed: that writers admitted BEFORE the migration started have
// quiesced before STAGING. That requires BC-1.18.013 Precondition 6(c)'s
// drain-with-writer-reservations step, which mechanism-A's production code
// does not implement (reported as a production gap, not modeled here).
// ===========================================================================

/// A txn state is LIVE (an in-flight migration) iff STAGING or COMMITTING —
/// the same classification `read_active_append_log_txn_record` and
/// `run_backfill_append_logs`'s `resuming_live` use (stated independently of
/// the predicate under proof, so the harness checks the predicate against it).
fn txn_is_live(txn_state: Option<AppendLogMigrationTxnState>) -> bool {
    matches!(
        txn_state,
        Some(AppendLogMigrationTxnState::Staging) | Some(AppendLogMigrationTxnState::Committing)
    )
}

#[kani::proof]
#[kani::unwind(9)]
fn proof_obl_a4_admission_gate_invariant() {
    let mut txn_state: Option<AppendLogMigrationTxnState> = None;
    let mut refused_while_live = false;
    let mut readmitted_after_terminal = false;

    let steps: usize = kani::any();
    kani::assume(steps <= 8);

    for _ in 0..steps {
        let action: u8 = kani::any();
        kani::assume(action < 3);
        match action {
            0 => {
                // A PreToolUse Edit/Write/MultiEdit against .factory/cycles/,
                // decided by the REAL predicate over the current txn state.
                let live = txn_is_live(txn_state);
                let admitted = is_append_log_admission_open(txn_state);
                kani::assert(
                    !(admitted && live),
                    "a4/INV-GATE-TXN: no write is admitted while a migration txn is STAGING/COMMITTING",
                );
                kani::assert(
                    admitted || live,
                    "a4: a write is refused only while a txn is STAGING/COMMITTING (no self-lock)",
                );
                refused_while_live |= !admitted && live;
                readmitted_after_terminal |= admitted
                    && matches!(
                        txn_state,
                        Some(AppendLogMigrationTxnState::Completed)
                            | Some(AppendLogMigrationTxnState::Aborted)
                    );
            }
            1 => {
                // run_backfill_append_logs, fresh run: a new txn record is
                // created in STAGING only when no LIVE record exists
                // (otherwise production resumes the live one instead).
                if !txn_is_live(txn_state) {
                    txn_state = Some(AppendLogMigrationTxnState::Staging);
                }
            }
            _ => {
                // A lifecycle event, applied through the REAL relation.
                if let Some(s) = txn_state {
                    if let Some(n) = append_log_txn_transition(s, any_txn_event()) {
                        txn_state = Some(n);
                    }
                }
            }
        }
    }

    kani::cover!(
        refused_while_live,
        "a4 cover: a write is refused during a live txn"
    );
    kani::cover!(
        readmitted_after_terminal,
        "a4 cover: writes are re-admitted after the txn reaches a terminal state"
    );
}

// ===========================================================================
// CrashFs — the crash model for harness a5
//
// Axioms (each a CONSERVATIVE superset of what a POSIX filesystem with an
// ordered metadata journal — ext4 data=ordered, APFS, NTFS — may persist):
//
//   A1 Each name binds a content tag plus a `synced` bit (content flushed by
//      fsync_file). A name whose binding persists while its content was never
//      fsynced is observed TORN after a crash.
//   A2 Namespace/content mutations since the last full barrier persist as a
//      nondeterministic PREFIX of their issue order (ordered metadata
//      journaling) — zero of them, all of them, or any prefix.
//   A3 fsync_dir is a full barrier: everything issued before it is durable.
//      (All modeled files share one directory per production layout:
//      CURRENT.json, the intent log and the txn record live in
//      `.factory/migration-state/`; the staging/canonical pair is a separate
//      directory, but every rename in this protocol is followed by an
//      fsync_dir before anything depends on it, so collapsing them only
//      REMOVES barriers between unrelated directories — still conservative.)
//   A4 fsync_file marks the content of that name synced (in the live state and
//      in every pending prefix snapshot that still binds the same content).
//   A5 rename / pointer_swap is atomic within each persisted prefix.
//   A6 `migration_durable_write` (production's temp + F_FULLFSYNC + rename +
//      dir-fsync primitive, used for the staged files, the txn record and
//      completed.json) is atomic and durable on return — modeled as one
//      synced snapshot followed by a barrier.
// ===========================================================================

#[derive(Clone, Copy, PartialEq, Eq)]
enum FileId {
    CurrentJson,
    CurrentTmp,
    Canonical,
    Staging,
    IntentRecord,
    DoneRecord,
    TxnRecord,
    CompletedJson,
    Other,
}

const FILE_SLOTS: usize = 9;
/// Max mutations between two barriers in harness a5's protocol (checked).
const MAX_PENDING: usize = 6;

fn file_id_index(id: FileId) -> usize {
    match id {
        FileId::CurrentJson => 0,
        FileId::CurrentTmp => 1,
        FileId::Canonical => 2,
        FileId::Staging => 3,
        FileId::IntentRecord => 4,
        FileId::DoneRecord => 5,
        FileId::TxnRecord => 6,
        FileId::CompletedJson => 7,
        FileId::Other => 8,
    }
}

/// Single-character path constants keep `Path` parsing / UTF-8 validation at
/// ~1 loop iteration. The intent log's INTENT and DONE records are modeled as
/// two slots (`i`, `o`) because they are separately appended and separately
/// synced records of one append-only file; a torn trailing record is
/// discarded by `read_append_log_intent_log`, which the model mirrors by
/// ignoring a TORN record slot.
fn file_id_of(path: &Path) -> FileId {
    match path.as_os_str().to_str() {
        Some("c") => FileId::CurrentJson,
        Some("t") => FileId::CurrentTmp,
        Some("n") => FileId::Canonical,
        Some("s") => FileId::Staging,
        Some("i") => FileId::IntentRecord,
        Some("o") => FileId::DoneRecord,
        Some("x") => FileId::TxnRecord,
        Some("p") => FileId::CompletedJson,
        _ => FileId::Other,
    }
}

#[derive(Clone, Copy)]
struct Slot {
    tag: Option<u8>,
    synced: bool,
}

impl Slot {
    const EMPTY: Slot = Slot {
        tag: None,
        synced: true,
    };
}

type Image = [Slot; FILE_SLOTS];

struct CrashState {
    /// What every process observes right now.
    live: Image,
    /// What is guaranteed to survive a crash (state at the last barrier).
    durable: Image,
    /// Snapshots of `live` after each mutation since the last barrier.
    pending: [Image; MAX_PENDING],
    pending_len: usize,
}

struct CrashFs {
    st: RefCell<CrashState>,
}

/// Content-tag of a TORN file (binding persisted, content never fsynced).
const GEN_TORN: u8 = 0xEE;

impl CrashFs {
    fn new() -> Self {
        CrashFs {
            st: RefCell::new(CrashState {
                live: [Slot::EMPTY; FILE_SLOTS],
                durable: [Slot::EMPTY; FILE_SLOTS],
                pending: [[Slot::EMPTY; FILE_SLOTS]; MAX_PENDING],
                pending_len: 0,
            }),
        }
    }

    /// Seed a file as fully durable pre-migration state.
    fn seed_durable(&self, id: FileId, tag: u8) {
        let mut st = self.st.borrow_mut();
        let slot = Slot {
            tag: Some(tag),
            synced: true,
        };
        st.live[file_id_index(id)] = slot;
        st.durable[file_id_index(id)] = slot;
    }

    fn snapshot(st: &mut CrashState) {
        assert!(
            st.pending_len < MAX_PENDING,
            "CrashFs: MAX_PENDING too small"
        );
        let i = st.pending_len;
        st.pending[i] = st.live;
        st.pending_len += 1;
    }

    fn barrier(st: &mut CrashState) {
        st.durable = st.live;
        st.pending_len = 0;
    }

    fn set_live(&self, id: FileId, tag: Option<u8>, synced: bool) {
        let mut st = self.st.borrow_mut();
        st.live[file_id_index(id)] = Slot { tag, synced };
        Self::snapshot(&mut st);
    }

    /// A6: production's `migration_durable_write` — atomic + durable.
    fn durable_write(&self, path: &Path, tag: u8) {
        let mut st = self.st.borrow_mut();
        st.live[file_id_index(file_id_of(path))] = Slot {
            tag: Some(tag),
            synced: true,
        };
        Self::barrier(&mut st);
    }

    /// Power loss: the restarted process observes the durable image plus a
    /// nondeterministic prefix of the pending mutations (A2), with unsynced
    /// content observed TORN (A1).
    fn crash(&self) {
        let mut st = self.st.borrow_mut();
        let m: usize = kani::any();
        kani::assume(m <= st.pending_len);
        let image = if m == 0 {
            st.durable
        } else {
            st.pending[m - 1]
        };
        let mut recovered = [Slot::EMPTY; FILE_SLOTS];
        for (out, slot) in recovered.iter_mut().zip(image.iter()) {
            *out = match slot.tag {
                Some(_) if !slot.synced => Slot {
                    tag: Some(GEN_TORN),
                    synced: true,
                },
                _ => Slot {
                    tag: slot.tag,
                    synced: true,
                },
            };
        }
        st.live = recovered;
        st.durable = recovered;
        st.pending_len = 0;
    }

    fn observe(&self, id: FileId) -> Option<u8> {
        self.st.borrow().live[file_id_index(id)].tag
    }
}

impl Fs for CrashFs {
    fn write_temp(&self, path: &Path, content: &[u8]) -> Result<(), BcIndexMigrationError> {
        // Conservative superset of production's (fully durable) write_temp:
        // content is NOT synced until fsync_file.
        self.set_live(file_id_of(path), content.first().copied(), false);
        Ok(())
    }

    fn fsync_file(&self, path: &Path) -> Result<(), BcIndexMigrationError> {
        // A4.
        let mut st = self.st.borrow_mut();
        let i = file_id_index(file_id_of(path));
        let tag = st.live[i].tag;
        st.live[i].synced = true;
        let len = st.pending_len;
        for snap in st.pending.iter_mut().take(len) {
            if snap[i].tag == tag {
                snap[i].synced = true;
            }
        }
        // Branch-free so the check stays reachable even in protocols that never
        // fsync an already-durable binding.
        let same_inode = st.durable[i].tag == tag;
        st.durable[i].synced |= same_inode;
        Ok(())
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), BcIndexMigrationError> {
        // A5: one atomic mutation.
        let mut st = self.st.borrow_mut();
        let from_i = file_id_index(file_id_of(from));
        let to_i = file_id_index(file_id_of(to));
        st.live[to_i] = st.live[from_i];
        st.live[from_i] = Slot::EMPTY;
        Self::snapshot(&mut st);
        Ok(())
    }

    fn fsync_dir(&self, _dir: &Path) -> Result<(), BcIndexMigrationError> {
        // A3.
        let mut st = self.st.borrow_mut();
        Self::barrier(&mut st);
        Ok(())
    }

    fn read(&self, path: &Path) -> Result<Option<Vec<u8>>, BcIndexMigrationError> {
        Ok(self.observe(file_id_of(path)).map(|g| vec![g]))
    }

    fn exists(&self, path: &Path) -> bool {
        self.observe(file_id_of(path)).is_some()
    }

    fn pointer_swap(&self, tmp: &Path, target: &Path) -> Result<(), BcIndexMigrationError> {
        self.rename(tmp, target)
    }

    fn remove(&self, path: &Path) -> Result<(), BcIndexMigrationError> {
        self.set_live(file_id_of(path), None, true);
        Ok(())
    }

    fn append(&self, path: &Path, content: &[u8]) -> Result<(), BcIndexMigrationError> {
        // Production bundles append + sync_all; the model separates them (the
        // harness calls fsync_file right after, as production's sync_all does).
        self.set_live(file_id_of(path), content.first().copied(), false);
        Ok(())
    }
}

// ===========================================================================
// Harness a5 — POINTER-SWAP + PER-MOVE CRASH ATOMICITY
//
// Drives `run_backfill_append_logs` + `finish_append_log_migration`'s durable
// write order over `CrashFs` for ONE representative canonical-path move (all
// four target files share the single CURRENT.json commit point and the single
// txn record, so per-move recovery is independent across moves — each move is
// decided by `decide_append_log_recovery` from its own intent record), with:
//
//   * a nondeterministic crash point (every step boundary),
//   * a nondeterministic partial-persistence outcome at the crash (A2),
//   * an optional nondeterministic pre-commit abort (any `abort_staging` site),
//   * an optional FOREIGN write to the canonical path at any step (a writer
//     the admission gate did not exclude), and
//   * a canonical pre-image that is either the OLD content (the CURRENT file
//     being replaced) or absent (a brand-new shard file).
//
// After the crash, recovery is routed exactly as production routes it
// (`completed.json` present => ALREADY_MIGRATED; txn COMMITTING =>
// `finish_append_log_migration`, which consults the REAL
// `decide_append_log_recovery`; otherwise no canonical move is performed) and
// the following are asserted:
//
//   (i)   CURRENT.json is exactly OLD or NEW — never torn, never missing;
//   (ii)  WAL ordering: a durable COMMITTING txn record implies CURRENT == NEW
//         and the move's INTENT record are durable;
//   (iii) the migration's NEW content reaches the canonical path only after a
//         durable COMMITTING record (no move outside the commit protocol);
//   (iv)  routed into finish: TreatDone iff canonical holds the NEW content;
//         foreign canonical content is never TreatDone and never overwritten
//         (RedoRename); with no foreign interference, recovery never fails
//         closed (it always resumes or recognizes completion).
// ===========================================================================

const GEN_OLD: u8 = 10;
const GEN_NEW: u8 = 20;
const GEN_FOREIGN: u8 = 30;
const TAG_INTENT: u8 = 40;
const TAG_DONE: u8 = 41;
const TAG_COMPLETED: u8 = 50;
const TXN_STAGING: u8 = 1;
const TXN_COMMITTING: u8 = 2;
const TXN_COMPLETED: u8 = 3;
const TXN_ABORTED: u8 = 4;

/// Map an observed canonical/staging content tag to the hash stand-in
/// `decide_append_log_recovery` compares against.
fn content_hash(g: Option<u8>) -> Option<&'static str> {
    match g {
        None => None,
        Some(GEN_OLD) => Some(HASH_PRE),
        Some(GEN_NEW) => Some(HASH_POST),
        Some(GEN_TORN) => Some(HASH_TORN),
        _ => Some(HASH_FOREIGN),
    }
}

fn txn_tag(s: AppendLogMigrationTxnState) -> u8 {
    match s {
        AppendLogMigrationTxnState::Staging => TXN_STAGING,
        AppendLogMigrationTxnState::Committing => TXN_COMMITTING,
        AppendLogMigrationTxnState::Completed => TXN_COMPLETED,
        AppendLogMigrationTxnState::Aborted => TXN_ABORTED,
    }
}

// Protocol steps (each runs only if `crash_at` is past it).
const STEP_TXN_STAGING: usize = 0; // write_append_log_txn_record (STAGING, generation_id)
const STEP_STAGE: usize = 1; // migration_durable_write(staged file)
const STEP_INTENT: usize = 2; // append_intent_records_for_append_log_pending_moves
const STEP_TMP: usize = 3; // fs.write_temp(CURRENT.tmp) + fs.fsync_file
const STEP_SWAP: usize = 4; // fs.pointer_swap(CURRENT.tmp -> CURRENT.json)
const STEP_SWAP_SYNC: usize = 5; // fs.fsync_dir(migration_state_dir)
const STEP_TXN_COMMITTING: usize = 6; // advance(BeginCommitting) + write txn record
const STEP_RENAME: usize = 7; // finish: fs.rename(staging -> canonical)
const STEP_RENAME_SYNC: usize = 8; // finish: fs.fsync_dir(parent)
const STEP_DONE: usize = 9; // finish: write DONE intent record
const STEP_COMPLETED: usize = 10; // finish: migration_durable_write(completed.json)
const STEP_TXN_COMPLETED: usize = 11; // advance(Complete) + write txn record
const STEP_COUNT: usize = 12;

/// Nondeterministic interference schedule for one a5 run.
struct RunCtx {
    crash_at: usize,
    abort_at: usize,
    foreign_at: usize,
    foreign_happened: bool,
}

/// Runs before production step `step`: stops the run if the crash point has
/// been reached, injects the optional foreign write, and performs the optional
/// pre-commit abort exactly as `run_backfill_append_logs`'s `abort_staging`
/// closure does (remove the staged generation, then ABORTED via the REAL
/// transition relation, durably persisted). Returns `false` to stop the run.
fn before_step(
    fs: &CrashFs,
    step: usize,
    ctx: &mut RunCtx,
    txn: &mut AppendLogMigrationTxnState,
) -> bool {
    if step >= ctx.crash_at {
        return false;
    }
    if ctx.foreign_at == step {
        // A non-participating writer's Edit/Write: not necessarily fsynced
        // (the Edit tool does not fsync), so its content may later be TORN.
        let _ = fs.write_temp(Path::new("n"), &[GEN_FOREIGN]);
        let foreign_synced: bool = kani::any();
        if foreign_synced {
            let _ = fs.fsync_file(Path::new("n"));
        }
        ctx.foreign_happened = true;
    }
    if ctx.abort_at == step && step > STEP_TXN_STAGING && step <= STEP_TMP {
        let _ = fs.remove(Path::new("s"));
        if let Some(n) = append_log_txn_transition(*txn, AppendLogTxnEvent::Abort) {
            *txn = n;
            fs.durable_write(Path::new("x"), txn_tag(*txn));
        }
        return false;
    }
    true
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::stub(std::fmt::format, stub_format)]
fn proof_obl_a5_pointer_swap_and_move_crash_atomicity() {
    let fs = CrashFs::new();
    let current = Path::new("c");
    let current_tmp = Path::new("t");
    let canonical = Path::new("n");
    let staging = Path::new("s");
    let intent = Path::new("i");
    let done = Path::new("o");
    let txn_path = Path::new("x");
    let completed = Path::new("p");
    let dir = Path::new("d");

    // Pre-migration durable state: CURRENT.json = OLD; the canonical target
    // holds the OLD content or does not exist yet (new shard file).
    fs.seed_durable(FileId::CurrentJson, GEN_OLD);
    let canonical_preexists: bool = kani::any();
    if canonical_preexists {
        fs.seed_durable(FileId::Canonical, GEN_OLD);
    }

    let crash_at: usize = kani::any();
    kani::assume(crash_at <= STEP_COUNT);
    let mut ctx = RunCtx {
        crash_at,
        // Optional pre-commit failure at any abort_staging site (after the
        // STAGING record exists, at or before the CURRENT.tmp write);
        // out-of-range values mean "no abort".
        abort_at: kani::any(),
        // Optional foreign write to the canonical path just before step
        // `foreign_at`; out-of-range values mean "no foreign write".
        foreign_at: kani::any(),
        foreign_happened: false,
    };

    let mut txn = AppendLogMigrationTxnState::Staging;
    let mut recorded_pre: Option<&'static str> = None;

    // Straight-line (not a loop over a step index) so each production step —
    // in particular the decide_append_log_recovery call — is encoded ONCE in
    // the CBMC formula rather than once per unrolled iteration.
    'run: {
        if !before_step(&fs, STEP_TXN_STAGING, &mut ctx, &mut txn) {
            break 'run;
        }
        fs.durable_write(txn_path, TXN_STAGING);

        if !before_step(&fs, STEP_STAGE, &mut ctx, &mut txn) {
            break 'run;
        }
        fs.durable_write(staging, GEN_NEW);

        if !before_step(&fs, STEP_INTENT, &mut ctx, &mut txn) {
            break 'run;
        }
        // expected_pre_state = hash of whatever canonical holds NOW.
        recorded_pre = content_hash(fs.observe(FileId::Canonical));
        let _ = fs.append(intent, &[TAG_INTENT]);
        let _ = fs.fsync_file(intent);

        if !before_step(&fs, STEP_TMP, &mut ctx, &mut txn) {
            break 'run;
        }
        let _ = fs.write_temp(current_tmp, &[GEN_NEW]);
        let _ = fs.fsync_file(current_tmp);

        if !before_step(&fs, STEP_SWAP, &mut ctx, &mut txn) {
            break 'run;
        }
        let _ = fs.pointer_swap(current_tmp, current);

        if !before_step(&fs, STEP_SWAP_SYNC, &mut ctx, &mut txn) {
            break 'run;
        }
        let _ = fs.fsync_dir(dir);

        if !before_step(&fs, STEP_TXN_COMMITTING, &mut ctx, &mut txn) {
            break 'run;
        }
        match append_log_txn_transition(txn, AppendLogTxnEvent::BeginCommitting) {
            Some(n) => {
                txn = n;
                fs.durable_write(txn_path, txn_tag(txn));
            }
            None => break 'run,
        }

        if !before_step(&fs, STEP_RENAME, &mut ctx, &mut txn) {
            break 'run;
        }
        // finish_append_log_migration consults the decision before the
        // rename, from the INTENT record it just read back.
        let d = decide_append_log_recovery(
            content_hash(fs.observe(FileId::Canonical)),
            content_hash(fs.observe(FileId::Staging)),
            Some(&make_append_log_record(
                AppendLogIntentLogRecordType::Intent,
                recorded_pre,
            )),
        );
        let proceed = d == AppendLogIntentLogRecoveryDecision::RedoRename;
        std::mem::forget(d);
        if !proceed {
            // TreatDone is impossible here (canonical != NEW before the first
            // move); FailClosed halts the run with BinaryIntegrityFailure.
            break 'run;
        }
        let _ = fs.rename(staging, canonical);

        if !before_step(&fs, STEP_RENAME_SYNC, &mut ctx, &mut txn) {
            break 'run;
        }
        let _ = fs.fsync_dir(dir);

        if !before_step(&fs, STEP_DONE, &mut ctx, &mut txn) {
            break 'run;
        }
        let _ = fs.append(done, &[TAG_DONE]);
        let _ = fs.fsync_file(done);

        if !before_step(&fs, STEP_COMPLETED, &mut ctx, &mut txn) {
            break 'run;
        }
        fs.durable_write(completed, TAG_COMPLETED);

        if !before_step(&fs, STEP_TXN_COMPLETED, &mut ctx, &mut txn) {
            break 'run;
        }
        if let Some(n) = append_log_txn_transition(txn, AppendLogTxnEvent::Complete) {
            txn = n;
            fs.durable_write(txn_path, txn_tag(txn));
        }
    }
    let foreign_happened = ctx.foreign_happened;

    fs.crash();

    let pointer = fs.observe(FileId::CurrentJson);
    let durable_txn = fs.observe(FileId::TxnRecord);
    let canonical_content = fs.observe(FileId::Canonical);
    let staging_content = fs.observe(FileId::Staging);
    let intent_durable = fs.observe(FileId::IntentRecord) == Some(TAG_INTENT);
    let done_durable = fs.observe(FileId::DoneRecord) == Some(TAG_DONE);

    // (i) pointer crash atomicity.
    kani::assert(
        pointer == Some(GEN_OLD) || pointer == Some(GEN_NEW),
        "a5(i): after a crash at ANY point, CURRENT resolves to exactly OLD or NEW (never torn/missing)",
    );

    // (ii) WAL ordering for the recovery-routing record.
    if durable_txn == Some(TXN_COMMITTING) {
        kani::assert(
            pointer == Some(GEN_NEW),
            "a5(ii): a durable COMMITTING txn record implies the pointer swap is durable",
        );
        kani::assert(
            intent_durable,
            "a5(ii): a durable COMMITTING txn record implies the move's INTENT record is durable",
        );
    }

    // (iii) the migration's content reaches canonical only via the protocol.
    if canonical_content == Some(GEN_NEW) {
        kani::assert(
            durable_txn == Some(TXN_COMMITTING) || durable_txn == Some(TXN_COMPLETED),
            "a5(iii): NEW content at canonical implies a durable COMMITTING (or COMPLETED) txn",
        );
    }
    kani::assert(
        durable_txn != Some(TXN_ABORTED) || canonical_content != Some(GEN_NEW),
        "a5(iii): an ABORTED migration never touched the canonical path",
    );

    // (iv) recovery routed exactly as run_backfill_append_logs routes it.
    let completed_present = fs.observe(FileId::CompletedJson).is_some();
    kani::assert(
        !completed_present || fs.observe(FileId::CompletedJson) == Some(TAG_COMPLETED),
        "a5: completed.json is never observed torn",
    );
    if completed_present {
        // ALREADY_MIGRATED: the move had durably completed first.
        if !foreign_happened {
            kani::assert(
                canonical_content == Some(GEN_NEW),
                "a5(iv): completed.json present implies the move is durably complete",
            );
        }
    } else if durable_txn == Some(TXN_COMMITTING) {
        // finish_append_log_migration: latest durable, non-torn record wins.
        let record = if done_durable {
            Some(make_append_log_record(
                AppendLogIntentLogRecordType::Done,
                None,
            ))
        } else if intent_durable {
            Some(make_append_log_record(
                AppendLogIntentLogRecordType::Intent,
                recorded_pre,
            ))
        } else {
            None
        };
        let decision = decide_append_log_recovery(
            content_hash(canonical_content),
            content_hash(staging_content),
            record.as_ref(),
        );

        kani::assert(
            (decision == AppendLogIntentLogRecoveryDecision::TreatDone)
                == (canonical_content == Some(GEN_NEW)),
            "a5(iv): TreatDone iff canonical holds the NEW post-image (content, not existence)",
        );
        // Canonical content that is neither the post-image nor the pre-image
        // the governing record captured (a foreign write landing after the
        // intent record, or a torn one) is never TreatDone and never
        // overwritten by RedoRename.
        let governing_pre = if done_durable { None } else { recorded_pre };
        let canonical_hash = content_hash(canonical_content);
        if canonical_hash != Some(HASH_POST) && canonical_hash != governing_pre {
            kani::assert(
                matches!(
                    decision,
                    AppendLogIntentLogRecoveryDecision::FailClosed { .. }
                ),
                "a5(iv): canonical content matching neither the post-image nor the recorded \
                 pre-image is never TreatDone and never overwritten",
            );
        }
        if !foreign_happened {
            kani::assert(
                !matches!(
                    decision,
                    AppendLogIntentLogRecoveryDecision::FailClosed { .. }
                ),
                "a5(iv): without foreign interference, COMMITTING recovery always resumes or completes",
            );
        }

        kani::cover!(
            decision == AppendLogIntentLogRecoveryDecision::RedoRename,
            "a5 cover: crash after COMMITTING, before the durable move => RedoRename"
        );
        kani::cover!(
            decision == AppendLogIntentLogRecoveryDecision::TreatDone && !done_durable,
            "a5 cover: durable move, DONE record lost => TreatDone via the INTENT record"
        );
        kani::cover!(
            decision == AppendLogIntentLogRecoveryDecision::TreatDone && done_durable,
            "a5 cover: durable move + DONE record => TreatDone"
        );
        kani::cover!(
            canonical_content == Some(GEN_FOREIGN) && recorded_pre != Some(HASH_FOREIGN),
            "a5 cover: foreign canonical content seen by recovery (existence-only oracle's trap)"
        );
        kani::cover!(
            canonical_content == Some(GEN_TORN),
            "a5 cover: torn canonical content seen by recovery"
        );
        std::mem::forget(record);
        std::mem::forget(decision);
    } else {
        // STAGING / ABORTED / no record: production re-stages or starts fresh
        // and performs no canonical move on this path.
        kani::assert(
            canonical_content != Some(GEN_NEW),
            "a5(iv): a non-COMMITTING durable txn never has the migration's content at canonical",
        );
    }

    // Reachability of every crash class (vacuity guard).
    kani::cover!(
        pointer == Some(GEN_OLD) && durable_txn == Some(TXN_STAGING),
        "a5 cover: crash before the commit point"
    );
    kani::cover!(
        pointer == Some(GEN_NEW) && durable_txn == Some(TXN_STAGING),
        "a5 cover: pointer swap durable, COMMITTING record not yet written (re-stage on resume)"
    );
    kani::cover!(
        durable_txn == Some(TXN_ABORTED),
        "a5 cover: pre-commit abort persisted"
    );
    kani::cover!(completed_present, "a5 cover: crash after completed.json");
    kani::cover!(
        durable_txn == Some(TXN_COMPLETED),
        "a5 cover: full run to COMPLETED"
    );
    kani::cover!(
        !canonical_preexists && canonical_content == Some(GEN_NEW),
        "a5 cover: move onto a previously-missing canonical (new shard file)"
    );
}

// ===========================================================================
// Harness a6 — RECOVERY IDEMPOTENCE / CONVERGENCE
//
// For EVERY on-disk (canonical, staging) content shape and record, apply the
// action recovery chooses (TreatDone: nothing; RedoRename: the
// staging -> canonical move `finish_append_log_migration` performs;
// FailClosed: halt), optionally lose that action to a crash before its
// dir-fsync, and re-run recovery. Proves:
//
//   * a non-FailClosed first pass converges: the next pass is TreatDone, and
//     a further pass is still TreatDone (absorbing fixed point — a completed
//     move never re-triggers the destructive RedoRename);
//   * a crash that loses the RedoRename's rename reproduces EXACTLY the first
//     pass's decision (re-running recovery after a crash mid-action is safe);
//   * FailClosed is stable: re-running over unchanged state never upgrades it
//     to a forward action.
// ===========================================================================

#[kani::proof]
#[kani::unwind(4)]
#[kani::stub(std::fmt::format, stub_format)]
fn proof_obl_a6_recovery_idempotence() {
    let pre = any_pre_state();
    let record = make_append_log_record(AppendLogIntentLogRecordType::Intent, pre);
    let canonical0 = any_hash();
    let staging0 = any_hash();

    let first = decide_append_log_recovery(canonical0, staging0, Some(&record));
    let crashed_mid_action: bool = kani::any();

    let (canonical1, staging1) = match first {
        AppendLogIntentLogRecoveryDecision::RedoRename if !crashed_mid_action => (staging0, None),
        _ => (canonical0, staging0),
    };
    let second = decide_append_log_recovery(canonical1, staging1, Some(&record));

    match first {
        AppendLogIntentLogRecoveryDecision::TreatDone => {
            kani::assert(
                second == AppendLogIntentLogRecoveryDecision::TreatDone,
                "a6: TreatDone is an absorbing fixed point",
            );
        }
        AppendLogIntentLogRecoveryDecision::RedoRename => {
            if crashed_mid_action {
                kani::assert(
                    second == AppendLogIntentLogRecoveryDecision::RedoRename,
                    "a6: a crash that loses the rename reproduces the same RedoRename decision",
                );
            } else {
                kani::assert(
                    second == AppendLogIntentLogRecoveryDecision::TreatDone,
                    "a6: after the RedoRename move lands, recovery converges to TreatDone",
                );
                let third = decide_append_log_recovery(canonical1, staging1, Some(&record));
                kani::assert(
                    third == AppendLogIntentLogRecoveryDecision::TreatDone,
                    "a6: a completed move never re-triggers RedoRename",
                );
                std::mem::forget(third);
            }
        }
        AppendLogIntentLogRecoveryDecision::FailClosed { .. } => {
            kani::assert(
                matches!(
                    second,
                    AppendLogIntentLogRecoveryDecision::FailClosed { .. }
                ),
                "a6: FailClosed is stable over unchanged state (never upgraded to a forward action)",
            );
        }
    }

    kani::cover!(
        first == AppendLogIntentLogRecoveryDecision::RedoRename && !crashed_mid_action,
        "a6 cover: RedoRename applied then re-recovered"
    );
    kani::cover!(
        first == AppendLogIntentLogRecoveryDecision::RedoRename && crashed_mid_action,
        "a6 cover: RedoRename lost to a crash then re-recovered"
    );
    kani::cover!(
        first == AppendLogIntentLogRecoveryDecision::TreatDone,
        "a6 cover: TreatDone re-recovered"
    );

    std::mem::forget(record);
    std::mem::forget(first);
    std::mem::forget(second);
}
