// Test files use .expect()/.unwrap()/.panic!() for failure reporting,
// matching bc_1_18_011_b2_migration_test.rs's own established convention.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! OBL-1 (D-1232-OBL-1) systematic fault-injection / crash-consistency
//! suite for the B2 BC-INDEX shard migration
//! (`shard_manager::run_bc_index_migration` / `shard_manager::recover`).
//!
//! `--features factory-dispatcher/failpoints` only -- this whole file is a
//! no-op (empty test binary body, aside from the always-present harmless
//! `#[test]` child entrypoint below) under the default feature set, exactly
//! like `bc_1_18_011_b2_migration_obl1_failpoint_smoke_test.rs`.
//!
//! # Harness: true no-unwind crash semantics
//!
//! `fail::cfg_callback(name, closure)` registers a plain Rust closure (NOT
//! one of `fail`'s built-in action strings) that this suite always sets to
//! call `std::process::abort()` directly. `abort()` raises `SIGABRT`
//! immediately with **no stack unwinding and no destructors run** -- a
//! genuine crash. This is deliberately NEVER `fail`'s own `"panic"` action
//! (which unwinds the stack and runs destructors -- not a power-loss-style
//! crash) per the OBL-1 research blueprint's explicit caveat.
//!
//! Each scenario spawns a **separate OS process** (`std::process::Command`,
//! re-execing this same test binary via `std::env::current_exe()` with
//! `--exact test_OBL1_crash_injection_child_entrypoint`), which configures
//! the named `migration_fs::*` failpoint to abort on its `occurrence`-th
//! reach (a counting `AtomicUsize` inside the callback -- `fail`'s own
//! per-call counting only supports a max-count-then-`off` transition, not
//! "fire exactly once, at reach N, on a callback action", so this suite
//! rolls its own counter) and then calls the REAL `run_bc_index_migration`
//! entry point. The child's stdin is explicitly closed (`Stdio::null()`)
//! and the parent enforces a 30s wall-clock timeout with `kill()` on
//! expiry, per this project's subprocess-testing discipline -- a
//! miscounted occurrence must never hang CI.
//!
//! After the child aborts, the PARENT process runs "recovery" by calling
//! `run_bc_index_migration` again, in-process, against the same crashed
//! directory -- since `fail`'s failpoint registry is per-process and the
//! parent never itself calls `fail::cfg`/`fail::cfg_callback`, this
//! recovery call executes as ordinary, unmodified production code (the
//! SAME entry point the `migrate-bc-index` CLI invokes), which now
//! dispatches through `recover()` as its single resume-decision authority.
//!
//! # Honest limitation (documented per the research blueprint's own
//! caveat, not silently omitted)
//!
//! Killing a child process via `abort()` does **not** emulate real power
//! loss: dirty page-cache data the kernel has not yet flushed to the
//! physical device may still land on disk after the process dies (whereas
//! a genuine power cut loses it). This harness therefore under-approximates
//! write-reordering / partial-durability interleavings. Per the research
//! blueprint's own recommended split, that class of coverage belongs to
//! Kani's bounded abstract-`Fs` crash-trace-atomicity proof (formal-
//! verifier's separate OBL-1 deliverable, "part 2 of 2"), not this
//! integration suite. This suite is the CI-gating, deterministic,
//! process-level layer; it reproduces and closes the four pass-2-era
//! defect scenarios empirically and is complementary to, not a
//! replacement for, the Kani proof.
//!
//! # `migration_fs::*` occurrence tables (this suite's 2-subsystem,
//! 4-pending-move fixture: SS-01 shard, SS-02 shard, top-level shard
//! manifest, lean `BC-INDEX.md` body -- in that push order)
//!
//! `write_temp` (10 occurrences on an uninterrupted fresh run):
//!  1. initial txn record write (STAGING, `generation_id=None`)
//!  2. txn record write (STAGING, `generation_id=Some`, after
//!     `stage_new_generation`)
//!  3. SS-01 shard staging write
//!  4. SS-02 shard staging write
//!  5. top-level shard-manifest.toml staging write
//!  6. staged lean `BC-INDEX.md` body staging write
//!  7. txn record write (`pending_canonical_moves` populated) -- AFTER the
//!     WAL-boundary intent-log INTENT append (occurrences 1-4 of `append`,
//!     below) and the Postcondition 3a fingerprint recheck
//!  8. `CURRENT.json` commit -- the sole commit point
//!  9. txn record write (`state=Committing`)
//!  10. txn record write (`state=Completed`) -- AFTER `completed.json` is
//!      ALREADY durably written (that write is NOT `Fs`-seamed -- see
//!      FINDING 2 below)
//!
//! `fsync_file` (5 occurrences, paired 1:1 with the 5 `write_txn_record`
//! calls above -- occurrences 1/2/7/9/10 in `write_temp`'s own numbering):
//!  1..5
//!
//! `append` (8 occurrences): 1-4 = INTENT records for the 4 pending moves
//! (same order as the fixture list above), durable BEFORE
//! `pending_canonical_moves` is persisted, BEFORE the fingerprint recheck,
//! and BEFORE the pointer swap (the WAL-ordering fix, research finding
//! #4); 5-8 = DONE records, one per successful canonical rename
//! (post-swap), same order.
//!
//! `rename` / `fsync_dir` (post-swap, 4 occurrences each, same order) plus
//! ONE extra `fsync_dir` occurrence (#1) inside `stage_new_generation`
//! (the generation-directory sync, BEFORE `generation_id` is persisted to
//! the txn record) -- so `fsync_dir` has 5 occurrences total, `rename` has
//! 4.
//!
//! `remove` (0 occurrences on the happy path): reachable only via
//! `abort_staging`'s (PC1/PC2 verification failure) or
//! `discard_incomplete_staging`'s (STAGING-resume verification failure)
//! gen-dir cleanup -- see the chained two-stage test below.
//!
//! `pointer_swap` (0 occurrences, EVER): see FINDING 1 below.
//!
//! # FINDINGS -- real crash-consistency defects this suite discovered
//! (NOT weakened or papered over; see the module's final report to the
//! orchestrator for full detail and suggested routing)
//!
//! **FINDING 1 (dead seam):** `Fs::pointer_swap` is defined, unit-tested
//! in isolation, and documented as "the SOLE commit-point", but is NEVER
//! called by `run_bc_index_migration` or any other production code path --
//! `commit_current_generation_pointer` performs the actual `CURRENT.json`
//! commit-rename through `Fs::write_temp` instead (per that function's own
//! doc comment). `migration_fs::pointer_swap` is therefore unreachable from
//! real execution; `test_BC_1_18_011_obl1_FINDING1_*` below documents this
//! empirically rather than fabricating a test against a boundary that can
//! never fire.
//!
//! **FINDING 2 (real defect, HIGH severity -- silent false-success):** a
//! crash ANYWHERE between "the 4 pending canonical moves are computed /
//! staged" and "`pending_canonical_moves` is persisted to the durable txn
//! record" (i.e. during the `append`#1-4 INTENT-record loop, the
//! Postcondition 3a fingerprint recheck, or `write_temp`#6/#7) leaves the
//! on-disk txn record's `pending_canonical_moves` field at its stale,
//! EMPTY value. On the next invocation, `recover()` correctly classifies
//! this as `ResumeFromStaging` (staging content re-verifies fine -- it's
//! all durably present), but `run_bc_index_migration`'s `ResumeFromStaging`
//! arm feeds the STALE (empty) `txn.pending_canonical_moves` into
//! `finish_committing_migration` instead of recomputing it, so
//! `execute_canonical_path_moves` iterates zero moves, `completed_count
//! (0) < pending.len() (0)` is FALSE (vacuously), and the migration writes
//! `completed.json` with `canonical_paths_count: 0` and transitions the
//! txn to COMPLETED -- **while NONE of the shard files were ever created
//! and the canonical `BC-INDEX.md` was never split.** This is a genuine
//! "old-or-new, never torn" violation at the SYSTEM level: `completed.json`
//! (the reader-integration "is the split done" signal) says NEW while
//! every canonical file on disk still says OLD. `test_BC_1_18_011_obl1_
//! FINDING2_*` tests below reproduce this at the `write_temp`#6/#7 and
//! `append`#1/#4 boundaries and assert the CORRECT invariant (which
//! currently FAILS).
//!
//! **FINDING 3 (real defect, MEDIUM severity -- permanent gate lockout):**
//! a crash between `write_completed_record` (NOT `Fs`-seamed, so not
//! itself fault-injectable, but sequenced immediately before `write_temp`
//! occurrence #10 / `fsync_file` occurrence #5) succeeding and the
//! subsequent `write_admission_gate_state(Open)` call leaves the on-disk
//! writer-admission gate at LOCKED forever: the NEXT (and every
//! subsequent) `run_bc_index_migration` invocation hits the
//! `completed.json`-presence short-circuit (`BcIndexMigrationOutcome::
//! AlreadyMigrated`, checked "no other file consulted", ADR-052 §7c step
//! 8) at the very top of the function, BEFORE `recover()`'s dispatch and
//! BEFORE `finish_committing_migration`'s gate-reset call are ever reached
//! again. `reconcile_stale_admission_gate` cannot self-heal this either --
//! its Branch A requires `active_txn == None`, but a live COMMITTING txn
//! record still exists on disk in this exact scenario. The migration
//! itself is 100% correct and complete; only the gate is permanently
//! stuck, blocking every future writer via `admit_or_block_bc_index_writer`
//! with no automatic recovery path. `test_BC_1_18_011_obl1_FINDING3_*`
//! tests below reproduce this and assert the CORRECT invariant (which
//! currently FAILS).
//!
//! All three findings are newly surfaced by this suite (distinct from, and
//! in addition to, the four pass-2-era defects this refactor was built to
//! close) and are NOT papered over: the `test_BC_1_18_011_obl1_FINDING*`
//! tests assert the correct behavior and are therefore expected to FAIL
//! under the current implementation, by design -- see this suite's final
//! report for routing.
#![cfg(feature = "failpoints")]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use factory_dispatcher::shard_manager::{
    self, BcIndexAdmissionGateState, BcIndexMigrationError, BcIndexMigrationOutcome,
    BcIndexMigrationTxnRecord, BcIndexMigrationTxnState, run_bc_index_migration,
};

// ---------------------------------------------------------------------------
// Fixture: two subsystems (so the migration produces 4 pending canonical
// moves -- 2 per-subsystem shards + 1 top-level manifest + 1 lean
// BC-INDEX.md body -- giving every multi-occurrence boundary real
// per-target variation to crash between).
// ---------------------------------------------------------------------------

const SHARD_CONFIG: &str = "\
[[shard]]
artifact_stem = \"BC-INDEX\"
artifact_path = \".factory/specs/behavioral-contracts/BC-INDEX.md\"
practical_fuel_ceiling = 8000000
worst_case_fuel_per_byte = 106.36
max_single_record_bytes = 16384
safety_margin = 8192
shard_cap_bytes = 100000
shape = \"flat\"
";

const ORIGINAL_CONTENT: &str = "\
---
document_type: bc-index
version: \"1.0\"
total_bcs: 2
---

## Summary

| Subsystem | BC-S Prefix | Count | Directory |
|-----------|------------|-------|-----------|
| SS-01 Hook Dispatcher Core | BC-1 | 1 | ss-01/ |
| SS-02 Shard Manager | BC-2 | 1 | ss-02/ |

## Index by subsystem

### SS-01 — Hook Dispatcher Core (BC-1) — 1 BC

| BC ID | Title | Status | Capability | Stories |
|-------|-------|--------|-----------|---------|
| [BC-1.01.001](ss-01/BC-1.01.001.md) | Registry rejects unknown schema version | draft | CAP-TBD | S-15.01 |

### SS-02 — Shard Manager (BC-2) — 1 BC

| BC ID | Title | Status | Capability | Stories |
|-------|-------|--------|-----------|---------|
| [BC-2.01.001](ss-02/BC-2.01.001.md) | Shard manager splits oversized index | draft | CAP-TBD | S-25.02 |
";

fn bc_index_target(cwd: &Path) -> PathBuf {
    cwd.join(".factory/specs/behavioral-contracts/BC-INDEX.md")
}

fn migration_state_dir(cwd: &Path) -> PathBuf {
    cwd.join(".factory/migration-state")
}

fn shard1_path(cwd: &Path) -> PathBuf {
    cwd.join(".factory/specs/behavioral-contracts/shards/BC-INDEX-SS-01.md")
}

fn shard2_path(cwd: &Path) -> PathBuf {
    cwd.join(".factory/specs/behavioral-contracts/shards/BC-INDEX-SS-02.md")
}

fn setup_fixture(cwd: &Path) {
    let factory_dir = cwd.join(".factory");
    std::fs::create_dir_all(&factory_dir).unwrap();
    std::fs::write(factory_dir.join("shard-config.toml"), SHARD_CONFIG).unwrap();
    let canonical_path = bc_index_target(cwd);
    std::fs::create_dir_all(canonical_path.parent().unwrap()).unwrap();
    std::fs::write(&canonical_path, ORIGINAL_CONTENT).unwrap();
}

// ---------------------------------------------------------------------------
// Child-process crash harness
// ---------------------------------------------------------------------------

const ENV_BOUNDARY: &str = "VSDD_OBL1_CRASH_BOUNDARY";
const ENV_OCCURRENCE: &str = "VSDD_OBL1_CRASH_OCCURRENCE";
const ENV_CWD: &str = "VSDD_OBL1_CRASH_CWD";

/// Sentinel exit code the child uses when `run_bc_index_migration` returned
/// WITHOUT the configured failpoint ever firing an abort at the requested
/// occurrence -- distinguishes "the boundary/occurrence count is wrong" (a
/// suite bug) from a genuine abort (no ordinary exit code -- the process
/// dies by signal) or an ordinary migration error.
const CHILD_DID_NOT_ABORT_EXIT_CODE: i32 = 66;

/// The child-process entrypoint. A no-op under ordinary `cargo test`
/// execution (the env vars are absent, so this returns immediately);
/// becomes the crash-injection child only when spawned by
/// [`spawn_crash_child`] below, which sets them. Each invocation is a
/// freshly spawned, single-test process (see [`spawn_crash_child`]'s
/// `--exact`/`--test-threads=1` invocation), so `fail`'s process-global
/// failpoint registry is never shared across scenarios or raced by
/// concurrently running tests -- the same process-isolation discipline
/// `bc_1_18_011_b2_migration_obl1_failpoint_smoke_test.rs` documents for
/// the same reason.
#[test]
fn test_OBL1_crash_injection_child_entrypoint() {
    let Ok(boundary) = std::env::var(ENV_BOUNDARY) else {
        return; // not the child -- ordinary `cargo test` run, no-op
    };
    let occurrence: usize = std::env::var(ENV_OCCURRENCE)
        .expect("occurrence env var must be set alongside boundary")
        .parse()
        .expect("occurrence must be a valid usize");
    let cwd =
        PathBuf::from(std::env::var(ENV_CWD).expect("cwd env var must be set alongside boundary"));

    // True no-unwind crash semantics: a plain callback action, never
    // `fail`'s own `"panic"` action (which unwinds and runs destructors --
    // see this suite's header comment).
    let counter = AtomicUsize::new(0);
    fail::cfg_callback(boundary.clone(), move || {
        let n = counter.fetch_add(1, Ordering::SeqCst) + 1;
        if n == occurrence {
            std::process::abort();
        }
    })
    .expect("configuring the crash-injection callback must succeed");

    // Real production execution -- the SAME entry point `migrate-bc-index`
    // invokes.
    let _ = run_bc_index_migration(&cwd);
    std::process::exit(CHILD_DID_NOT_ABORT_EXIT_CODE);
}

struct CrashResult {
    aborted: bool,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Spawn the crash-injection child as a genuinely separate OS process
/// (`std::process::Command`, re-execing this same test binary), configured
/// to abort inside the named `migration_fs::*` boundary on its
/// `occurrence`-th reach. Stdin is explicitly closed (the child never
/// reads stdin) and a hard wall-clock timeout is enforced via polling +
/// `kill()` -- this suite must never hang a CI run if a boundary/occurrence
/// combination is miscounted and the child spins or blocks.
fn spawn_crash_child(boundary: &str, occurrence: usize, cwd: &Path) -> CrashResult {
    let exe = std::env::current_exe().expect("current_exe must resolve for a test binary");
    let mut child = Command::new(exe)
        .arg("--exact")
        .arg("test_OBL1_crash_injection_child_entrypoint")
        .arg("--test-threads=1")
        .arg("--nocapture")
        .env(ENV_BOUNDARY, boundary)
        .env(ENV_OCCURRENCE, occurrence.to_string())
        .env(ENV_CWD, cwd.as_os_str())
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
                "crash-injection child for boundary={boundary} occurrence={occurrence} did not \
                 exit within the 30s timeout -- killed. This indicates the failpoint never fired \
                 the requested number of times (boundary unreachable or occurrence count wrong \
                 for this fixture shape), or the migration itself deadlocked."
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    let output = child
        .wait_with_output()
        .expect("collecting crash-injection child output must succeed");

    #[cfg(unix)]
    let (aborted, exit_code) = {
        use std::os::unix::process::ExitStatusExt;
        match output.status.signal() {
            Some(sig) => (sig == 6 /* SIGABRT */, None),
            None => (false, output.status.code()),
        }
    };
    #[cfg(not(unix))]
    let (aborted, exit_code): (bool, Option<i32>) = (false, output.status.code());

    CrashResult {
        aborted,
        exit_code,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// Assert the child genuinely aborted (`SIGABRT`) at the intended boundary
/// -- never a silent `CHILD_DID_NOT_ABORT_EXIT_CODE` (meaning the boundary
/// was never reached the requested number of times, invalidating the whole
/// scenario) and never an ordinary migration `Err` return.
fn assert_child_aborted(result: &CrashResult, boundary: &str, occurrence: usize) {
    assert!(
        result.aborted,
        "crash-injection child for boundary={boundary} occurrence={occurrence} did not abort \
         (SIGABRT) -- exit_code={:?}. A CHILD_DID_NOT_ABORT_EXIT_CODE \
         ({CHILD_DID_NOT_ABORT_EXIT_CODE}) means the boundary was never reached the requested \
         number of times for this fixture shape. stdout={:?} stderr={:?}",
        result.exit_code, result.stdout, result.stderr,
    );
}

// ---------------------------------------------------------------------------
// Post-crash state readers -- direct filesystem reads using the SAME
// wire-schema types `shard_manager` exposes as `pub` (BcIndexMigrationTxnRecord
// derives Deserialize), since this is an external integration-test crate
// with no access to shard_manager's crate-private readers.
// ---------------------------------------------------------------------------

fn read_live_txn_record(msd: &Path) -> Option<BcIndexMigrationTxnRecord> {
    let entries = std::fs::read_dir(msd).ok()?;
    let mut found: Option<BcIndexMigrationTxnRecord> = None;
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        // `.json.archived` deliberately does not match `ends_with(".json")`.
        if name.starts_with("txn-") && name.ends_with(".json") {
            let content = std::fs::read_to_string(entry.path()).unwrap();
            let record: BcIndexMigrationTxnRecord =
                serde_json::from_str(&content).unwrap_or_else(|e| {
                    panic!("malformed txn record at {}: {e}", entry.path().display())
                });
            found = Some(record);
        }
    }
    found
}

fn completed_json_exists(msd: &Path) -> bool {
    msd.join("completed.json").exists()
}

fn read_gate_state(msd: &Path) -> BcIndexAdmissionGateState {
    let path = msd.join("gate-state.json");
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("malformed gate-state.json: {e}")),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            BcIndexAdmissionGateState::Open
        }
        Err(source) => panic!("failed to read gate-state.json: {source}"),
    }
}

// ---------------------------------------------------------------------------
// Invariant assertions (the four properties every scenario below checks)
// ---------------------------------------------------------------------------

/// Invariant "no fail-open": while the migration is crashed/recovering
/// (txn live and/or gate non-OPEN), a concurrent BC-INDEX writer MUST be
/// blocked. Calls the REAL production admission-check function --
/// `admit_or_block_bc_index_writer` -- rather than reimplementing its
/// predicate, so this assertion exercises the exact code path a real
/// PreToolUse dispatch would hit.
fn assert_admission_blocked(cwd: &Path, probe_id: &str) {
    let msd = migration_state_dir(cwd);
    let result = shard_manager::admit_or_block_bc_index_writer(&msd, probe_id);
    assert!(
        matches!(
            result,
            Err(BcIndexMigrationError::WriterAdmissionRefused { .. })
        ),
        "no-fail-open invariant violated: admit_or_block_bc_index_writer succeeded (or failed \
         with an unexpected error) while the migration was crashed/recovering -- a concurrent \
         writer would have been silently admitted. result={result:?}"
    );
}

/// Run `run_bc_index_migration` repeatedly (bounded) until it reaches a
/// terminal outcome (`Completed` or `AlreadyMigrated`) or `max_attempts` is
/// exhausted. Several `RecoveryDecision` arms (`DiscardPreGeneration`,
/// `CleanAbortExpiredStaging`, a failed `ResumeFromStaging`) return an
/// `Err` from the SAME call that performs the safe discard/abort action --
/// forward progress resumes on the NEXT invocation, not within the same
/// call. This mirrors exactly what an operator/CI re-running the
/// `migrate-bc-index` CLI after a reported failure would do.
fn run_recovery_to_convergence(
    cwd: &Path,
    max_attempts: usize,
) -> Result<BcIndexMigrationOutcome, BcIndexMigrationError> {
    let mut last = Err(BcIndexMigrationError::BinaryIntegrityFailure {
        message: "run_recovery_to_convergence called with max_attempts=0".to_string(),
    });
    for _ in 0..max_attempts {
        last = run_bc_index_migration(cwd);
        if matches!(
            last,
            Ok(BcIndexMigrationOutcome::Completed { .. })
                | Ok(BcIndexMigrationOutcome::AlreadyMigrated)
        ) {
            return last;
        }
    }
    last
}

/// The CORRECT terminal-state invariant: after recovery converges, the
/// migration is genuinely, completely done -- both shard files exist with
/// the right content, the canonical `BC-INDEX.md` no longer carries the
/// per-subsystem tables, the txn record is COMPLETED, the admission gate
/// is back to OPEN, and a fresh writer is admitted again. This is the
/// "old-or-new, never torn" + "terminal state" + "no permanent lockout"
/// properties combined, asserted the strong way (content-level, not just
/// "some Ok came back").
fn assert_genuinely_fully_migrated(cwd: &Path) {
    let msd = migration_state_dir(cwd);
    assert!(
        completed_json_exists(&msd),
        "completed.json must exist once the migration has genuinely converged"
    );
    let shard1 = shard1_path(cwd);
    let shard2 = shard2_path(cwd);
    assert!(
        shard1.exists(),
        "SS-01 canonical shard file must exist after a genuinely completed migration"
    );
    assert!(
        shard2.exists(),
        "SS-02 canonical shard file must exist after a genuinely completed migration"
    );
    let shard1_content = std::fs::read_to_string(&shard1).unwrap();
    let shard2_content = std::fs::read_to_string(&shard2).unwrap();
    assert!(
        shard1_content.contains("BC-1.01.001"),
        "SS-01 shard must contain its BC row (content-preservation)"
    );
    assert!(
        shard2_content.contains("BC-2.01.001"),
        "SS-02 shard must contain its BC row (content-preservation)"
    );
    let canonical_content = std::fs::read_to_string(bc_index_target(cwd)).unwrap();
    assert!(
        !canonical_content.contains("### SS-01") && !canonical_content.contains("### SS-02"),
        "canonical BC-INDEX.md must no longer carry per-subsystem BC tables once genuinely split \
         (BC-1.18.010 Invariant 3) -- got: {canonical_content:?}"
    );
    assert!(
        !canonical_content.contains("BC-1.01.001") && !canonical_content.contains("BC-2.01.001"),
        "independent census: canonical BC-INDEX.md must carry ZERO per-BC rows post-split"
    );

    let txn = read_live_txn_record(&msd)
        .expect("a live/most-recent txn record must exist after a genuinely completed migration");
    assert_eq!(
        txn.state,
        BcIndexMigrationTxnState::Completed,
        "the txn record must reach the COMPLETED terminal state, never permanently stuck in \
         STAGING/COMMITTING"
    );

    assert_eq!(
        read_gate_state(&msd),
        BcIndexAdmissionGateState::Open,
        "the writer-admission gate must return to OPEN once the migration genuinely completes -- \
         a stuck LOCKED/DRAINING gate here is a permanent-lockout defect (see FINDING 3)"
    );
    let admit = shard_manager::admit_or_block_bc_index_writer(&msd, "post-recovery-probe");
    assert!(
        admit.is_ok(),
        "a fresh writer must be admitted once the migration has genuinely, fully completed; got \
         {admit:?}"
    );
}

/// Idempotence: a subsequent recovery call is a no-op (`AlreadyMigrated`)
/// that does not disturb any of the already-converged invariants.
fn assert_recovery_is_idempotent(cwd: &Path) {
    let before = std::fs::read_to_string(bc_index_target(cwd)).unwrap();
    let outcome = run_bc_index_migration(cwd);
    assert!(
        matches!(outcome, Ok(BcIndexMigrationOutcome::AlreadyMigrated)),
        "re-running recovery after genuine completion must be a pure no-op via the \
         completed.json short-circuit -- got {outcome:?}"
    );
    let after = std::fs::read_to_string(bc_index_target(cwd)).unwrap();
    assert_eq!(
        before, after,
        "idempotent recovery must not further mutate the already-converged canonical content"
    );
    assert_genuinely_fully_migrated(cwd);
}

// ===========================================================================
// `write_temp` boundary
// ===========================================================================

#[test]
fn test_BC_1_18_011_obl1_crash_write_temp_occ1_before_any_txn_record_clean_restart() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    let crash = spawn_crash_child("migration_fs::write_temp", 1, dir.path());
    assert_child_aborted(&crash, "migration_fs::write_temp", 1);
    assert_admission_blocked(dir.path(), "probe");

    let outcome = run_recovery_to_convergence(dir.path(), 3);
    assert!(
        matches!(
            outcome,
            Ok(BcIndexMigrationOutcome::Completed {
                canonical_paths_count: 4
            })
        ),
        "crashing before ANY txn record exists must converge to a clean, complete fresh run; \
         got {outcome:?}"
    );
    assert_genuinely_fully_migrated(dir.path());
    assert_recovery_is_idempotent(dir.path());
}

#[test]
fn test_BC_1_18_011_obl1_crash_write_temp_occ2_generation_id_not_yet_persisted_discards() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    let crash = spawn_crash_child("migration_fs::write_temp", 2, dir.path());
    assert_child_aborted(&crash, "migration_fs::write_temp", 2);
    assert_admission_blocked(dir.path(), "probe");

    let txn = read_live_txn_record(&migration_state_dir(dir.path())).unwrap();
    assert_eq!(txn.state, BcIndexMigrationTxnState::Staging);
    assert!(
        txn.generation_id.is_none(),
        "the on-disk record must still show no generation_id (the crash landed before the SECOND \
         write persisted it), even though stage_new_generation's gen-dir already durably exists \
         on disk -- recover()'s DiscardPreGeneration arm is keyed on the RECORD, not the physical \
         gen-dir, per the v1.13 LOW-1 ordering invariant"
    );

    // DiscardPreGeneration returns Err from the SAME call that discards --
    // forward progress (a brand new fresh run) happens on the NEXT call.
    let outcome = run_recovery_to_convergence(dir.path(), 3);
    assert!(
        matches!(
            outcome,
            Ok(BcIndexMigrationOutcome::Completed {
                canonical_paths_count: 4
            })
        ),
        "got {outcome:?}"
    );
    assert_genuinely_fully_migrated(dir.path());
    assert_recovery_is_idempotent(dir.path());
}

#[test]
fn test_BC_1_18_011_obl1_crash_write_temp_occ3_partial_staging_discards_via_census_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    // Aborts right before the SS-02 shard staging write -- only the SS-01
    // shard is durably staged.
    let crash = spawn_crash_child("migration_fs::write_temp", 3, dir.path());
    assert_child_aborted(&crash, "migration_fs::write_temp", 3);
    assert_admission_blocked(dir.path(), "probe");

    let txn = read_live_txn_record(&migration_state_dir(dir.path())).unwrap();
    assert_eq!(txn.state, BcIndexMigrationTxnState::Staging);
    assert!(txn.generation_id.is_some());

    // resume_from_staging's mandatory EC-003/EC-060 census re-run finds
    // the staged generation incomplete and fails closed; run_bc_index_
    // migration's ResumeFromStaging-failure arm discards it (never
    // silently accepts a partial staged generation as complete).
    let outcome = run_recovery_to_convergence(dir.path(), 3);
    assert!(
        matches!(
            outcome,
            Ok(BcIndexMigrationOutcome::Completed {
                canonical_paths_count: 4
            })
        ),
        "got {outcome:?}"
    );
    assert_genuinely_fully_migrated(dir.path());
    assert_recovery_is_idempotent(dir.path());
}

/// FINDING 2, occurrence A: crash right after ALL staging content is
/// durable (all 4 artifacts staged) but before the INTENT-record append
/// loop starts. `pending_canonical_moves` is still empty on disk.
#[test]
fn test_BC_1_18_011_obl1_FINDING2_crash_write_temp_occ6_staged_but_pending_moves_empty() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    let crash = spawn_crash_child("migration_fs::write_temp", 6, dir.path());
    assert_child_aborted(&crash, "migration_fs::write_temp", 6);
    assert_admission_blocked(dir.path(), "probe");

    let txn = read_live_txn_record(&migration_state_dir(dir.path())).unwrap();
    assert_eq!(txn.state, BcIndexMigrationTxnState::Staging);
    assert_eq!(
        txn.pending_canonical_moves.len(),
        0,
        "precondition for FINDING 2: pending_canonical_moves must still be empty on disk at this \
         crash point"
    );

    let outcome = run_recovery_to_convergence(dir.path(), 3);
    // FINDING 2: this SHOULD converge to a genuine, content-correct
    // completion (or a fail-closed error) -- it currently, incorrectly,
    // reports Ok(Completed { canonical_paths_count: 0 }) with NEITHER
    // shard file ever created. Asserting the CORRECT invariant here (not
    // weakened) -- this assertion is expected to currently FAIL.
    assert_genuinely_fully_migrated(dir.path());
    let _ = outcome;
}

/// FINDING 2, occurrence B (primary repro): crash exactly at the boundary
/// between "intent log durable for every target" (the WAL-ordering fix's
/// own guarantee) and "pending_canonical_moves persisted to the txn
/// record".
#[test]
fn test_BC_1_18_011_obl1_FINDING2_crash_write_temp_occ7_pending_moves_not_yet_persisted() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    let crash = spawn_crash_child("migration_fs::write_temp", 7, dir.path());
    assert_child_aborted(&crash, "migration_fs::write_temp", 7);
    assert_admission_blocked(dir.path(), "probe");

    let txn = read_live_txn_record(&migration_state_dir(dir.path())).unwrap();
    assert_eq!(txn.state, BcIndexMigrationTxnState::Staging);
    assert_eq!(txn.pending_canonical_moves.len(), 0);

    let outcome = run_recovery_to_convergence(dir.path(), 3);
    // FINDING 2 (see module header): currently produces
    // Ok(Completed { canonical_paths_count: 0 }) with completed.json
    // present, canonical BC-INDEX.md UNCHANGED (still the monolithic
    // original), and neither shard file created -- a false-success /
    // "old-or-new, never torn" violation at the system level. This
    // assertion is expected to currently FAIL; it is intentionally left
    // asserting the CORRECT behavior, per this suite's mandate to not
    // weaken assertions around real findings.
    assert_genuinely_fully_migrated(dir.path());
    let _ = outcome;
}

/// FINDING 2, occurrence C: same defect class via the `append` boundary
/// directly (crash during the first INTENT-record append, before ANY of
/// the 4 targets has a durable intent record).
#[test]
fn test_BC_1_18_011_obl1_FINDING2_crash_append_occ1_first_intent_record() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    let crash = spawn_crash_child("migration_fs::append", 1, dir.path());
    assert_child_aborted(&crash, "migration_fs::append", 1);
    assert_admission_blocked(dir.path(), "probe");

    let outcome = run_recovery_to_convergence(dir.path(), 3);
    assert_genuinely_fully_migrated(dir.path());
    let _ = outcome;
}

/// FINDING 2, occurrence D: crash during the LAST INTENT-record append
/// (3 of 4 targets already have a durable intent record) -- still before
/// `pending_canonical_moves` is persisted, so the same false-success
/// defect reproduces even with the intent log almost entirely durable.
#[test]
fn test_BC_1_18_011_obl1_FINDING2_crash_append_occ4_last_intent_record() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    let crash = spawn_crash_child("migration_fs::append", 4, dir.path());
    assert_child_aborted(&crash, "migration_fs::append", 4);
    assert_admission_blocked(dir.path(), "probe");

    let outcome = run_recovery_to_convergence(dir.path(), 3);
    assert_genuinely_fully_migrated(dir.path());
    let _ = outcome;
}

#[test]
fn test_BC_1_18_011_obl1_crash_write_temp_occ8_at_sole_commit_point_resumes_correctly() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    let crash = spawn_crash_child("migration_fs::write_temp", 8, dir.path());
    assert_child_aborted(&crash, "migration_fs::write_temp", 8);
    assert_admission_blocked(dir.path(), "probe");

    let txn = read_live_txn_record(&migration_state_dir(dir.path())).unwrap();
    assert_eq!(txn.state, BcIndexMigrationTxnState::Staging);
    assert_eq!(
        txn.pending_canonical_moves.len(),
        4,
        "by occurrence 8, pending_canonical_moves (occurrence 7) has already durably persisted -- \
         this crash lands exactly AT the sole commit point, before CURRENT.json itself lands"
    );

    let outcome = run_recovery_to_convergence(dir.path(), 3);
    assert!(
        matches!(
            outcome,
            Ok(BcIndexMigrationOutcome::Completed {
                canonical_paths_count: 4
            })
        ),
        "a crash exactly at the sole commit point (before it lands) must safely resume from \
         ResumeFromStaging and complete; got {outcome:?}"
    );
    assert_genuinely_fully_migrated(dir.path());
    assert_recovery_is_idempotent(dir.path());
}

#[test]
fn test_BC_1_18_011_obl1_crash_write_temp_occ9_commit_landed_label_stale_staging() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    // CURRENT.json (occurrence 8) succeeded; the txn record's
    // state=Committing write (occurrence 9) is what gets aborted -- a
    // "physical ahead of label" mismatch this scenario proves recover()
    // handles safely without needing to consult CURRENT.json's own
    // content (per its own doc comment / OBL-1 design §0.3).
    let crash = spawn_crash_child("migration_fs::write_temp", 9, dir.path());
    assert_child_aborted(&crash, "migration_fs::write_temp", 9);
    assert_admission_blocked(dir.path(), "probe");

    let txn = read_live_txn_record(&migration_state_dir(dir.path())).unwrap();
    assert_eq!(
        txn.state,
        BcIndexMigrationTxnState::Staging,
        "the on-disk txn record is stale (still says STAGING) even though CURRENT.json already \
         durably names the new generation -- recover() must not need to consult CURRENT.json's \
         content to recover safely"
    );

    let outcome = run_recovery_to_convergence(dir.path(), 3);
    assert!(
        matches!(
            outcome,
            Ok(BcIndexMigrationOutcome::Completed {
                canonical_paths_count: 4
            })
        ),
        "got {outcome:?}"
    );
    assert_genuinely_fully_migrated(dir.path());
    assert_recovery_is_idempotent(dir.path());
}

/// FINDING 3, occurrence A: `completed.json` is already durable (the
/// migration is, in substance, fully and correctly done) but the txn
/// record's own COMPLETED-state write is what gets aborted.
#[test]
fn test_BC_1_18_011_obl1_FINDING3_crash_write_temp_occ10_completed_json_durable_gate_stuck() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    let crash = spawn_crash_child("migration_fs::write_temp", 10, dir.path());
    assert_child_aborted(&crash, "migration_fs::write_temp", 10);

    let msd = migration_state_dir(dir.path());
    assert!(
        completed_json_exists(&msd),
        "precondition for FINDING 3: completed.json must already be durable at this crash point \
         (it is written, via a non-Fs-seamed call, immediately before the write_temp occurrence \
         this test targets)"
    );

    let outcome = run_recovery_to_convergence(dir.path(), 3);
    assert!(
        matches!(outcome, Ok(BcIndexMigrationOutcome::AlreadyMigrated)),
        "recovery must report AlreadyMigrated once completed.json exists, per ADR-052 §7c step 8 \
         (\"no other file consulted\") -- got {outcome:?}"
    );
    // FINDING 3 (see module header): the admission gate should self-heal
    // to OPEN once the migration is genuinely, fully complete -- content
    // is correct (shards exist, census holds) but the gate is currently
    // left permanently LOCKED, because the AlreadyMigrated short-circuit
    // at the top of run_bc_index_migration never reaches
    // finish_committing_migration's gate-reset call, on this call OR any
    // future call. Asserting the CORRECT invariant (expected to currently
    // FAIL) rather than weakening it to match the observed stuck-LOCKED
    // behavior.
    assert_genuinely_fully_migrated(dir.path());
}

// ===========================================================================
// `fsync_file` boundary (5 occurrences, paired 1:1 with write_txn_record's
// 5 calls)
// ===========================================================================

#[test]
fn test_BC_1_18_011_obl1_crash_fsync_file_occ1_redundant_barrier_is_harmless() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    let crash = spawn_crash_child("migration_fs::fsync_file", 1, dir.path());
    assert_child_aborted(&crash, "migration_fs::fsync_file", 1);
    assert_admission_blocked(dir.path(), "probe");

    // fsync_file is a harmless, best-effort re-fsync in production --
    // write_temp's own bundled primitive already fully durably wrote the
    // content by the time fsync_file is reached (see migration_fs.rs's
    // own "production-granularity note"). A crash here is equivalent, in
    // terms of on-disk durable state, to a crash at write_temp occurrence
    // 1 -- proving the redundant barrier really is a no-op from a
    // recovery-correctness standpoint.
    let outcome = run_recovery_to_convergence(dir.path(), 3);
    assert!(
        matches!(
            outcome,
            Ok(BcIndexMigrationOutcome::Completed {
                canonical_paths_count: 4
            })
        ),
        "got {outcome:?}"
    );
    assert_genuinely_fully_migrated(dir.path());
    assert_recovery_is_idempotent(dir.path());
}

/// FINDING 3, occurrence B: same defect as the write_temp#10 case above,
/// reached via the fsync_file boundary instead (the redundant re-fsync of
/// the already-durable final COMPLETED-state txn record).
#[test]
fn test_BC_1_18_011_obl1_FINDING3_crash_fsync_file_occ5_completed_json_durable_gate_stuck() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    let crash = spawn_crash_child("migration_fs::fsync_file", 5, dir.path());
    assert_child_aborted(&crash, "migration_fs::fsync_file", 5);

    let msd = migration_state_dir(dir.path());
    assert!(completed_json_exists(&msd));

    let outcome = run_recovery_to_convergence(dir.path(), 3);
    assert!(
        matches!(outcome, Ok(BcIndexMigrationOutcome::AlreadyMigrated)),
        "got {outcome:?}"
    );
    assert_genuinely_fully_migrated(dir.path());
}

// ===========================================================================
// `append` boundary -- post-swap DONE records (green: exercises
// decide_intent_log_recovery's redo/skip table via the ForwardRecovery arm)
// ===========================================================================

#[test]
fn test_BC_1_18_011_obl1_crash_append_occ5_first_done_record_post_swap_resumes() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    // Occurrence 5 = the FIRST DONE record, i.e. right after the first
    // canonical rename (post-swap) already landed -- the pointer swap and
    // all 4 INTENT records are already durable at this point.
    let crash = spawn_crash_child("migration_fs::append", 5, dir.path());
    assert_child_aborted(&crash, "migration_fs::append", 5);
    assert_admission_blocked(dir.path(), "probe");

    let txn = read_live_txn_record(&migration_state_dir(dir.path())).unwrap();
    assert_eq!(txn.state, BcIndexMigrationTxnState::Committing);

    let outcome = run_recovery_to_convergence(dir.path(), 3);
    assert!(
        matches!(
            outcome,
            Ok(BcIndexMigrationOutcome::Completed {
                canonical_paths_count: 4
            })
        ),
        "got {outcome:?}"
    );
    assert_genuinely_fully_migrated(dir.path());
    assert_recovery_is_idempotent(dir.path());
}

// ===========================================================================
// `rename` boundary (post-swap canonical moves)
// ===========================================================================

#[test]
fn test_BC_1_18_011_obl1_crash_rename_occ1_first_canonical_move_resumes() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    let crash = spawn_crash_child("migration_fs::rename", 1, dir.path());
    assert_child_aborted(&crash, "migration_fs::rename", 1);
    assert_admission_blocked(dir.path(), "probe");

    let txn = read_live_txn_record(&migration_state_dir(dir.path())).unwrap();
    assert_eq!(txn.state, BcIndexMigrationTxnState::Committing);

    let outcome = run_recovery_to_convergence(dir.path(), 3);
    assert!(
        matches!(
            outcome,
            Ok(BcIndexMigrationOutcome::Completed {
                canonical_paths_count: 4
            })
        ),
        "got {outcome:?}"
    );
    assert_genuinely_fully_migrated(dir.path());
    assert_recovery_is_idempotent(dir.path());
}

#[test]
fn test_BC_1_18_011_obl1_crash_rename_occ3_mid_sequence_resumes() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    let crash = spawn_crash_child("migration_fs::rename", 3, dir.path());
    assert_child_aborted(&crash, "migration_fs::rename", 3);
    assert_admission_blocked(dir.path(), "probe");

    let outcome = run_recovery_to_convergence(dir.path(), 3);
    assert!(
        matches!(
            outcome,
            Ok(BcIndexMigrationOutcome::Completed {
                canonical_paths_count: 4
            })
        ),
        "got {outcome:?}"
    );
    assert_genuinely_fully_migrated(dir.path());
    assert_recovery_is_idempotent(dir.path());
}

// ===========================================================================
// `fsync_dir` boundary
// ===========================================================================

#[test]
fn test_BC_1_18_011_obl1_crash_fsync_dir_occ1_generation_dir_sync_discards() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    // Occurrence 1 = stage_new_generation's own gen-dir sync, BEFORE
    // generation_id is persisted to the txn record.
    let crash = spawn_crash_child("migration_fs::fsync_dir", 1, dir.path());
    assert_child_aborted(&crash, "migration_fs::fsync_dir", 1);
    assert_admission_blocked(dir.path(), "probe");

    let txn = read_live_txn_record(&migration_state_dir(dir.path())).unwrap();
    assert_eq!(txn.state, BcIndexMigrationTxnState::Staging);
    assert!(
        txn.generation_id.is_none(),
        "the gen dir may already physically exist (mkdir succeeded before the fsync_dir call), \
         but generation_id is not yet persisted to the record -- DiscardPreGeneration must fire"
    );

    let outcome = run_recovery_to_convergence(dir.path(), 3);
    assert!(
        matches!(
            outcome,
            Ok(BcIndexMigrationOutcome::Completed {
                canonical_paths_count: 4
            })
        ),
        "got {outcome:?}"
    );
    assert_genuinely_fully_migrated(dir.path());
    assert_recovery_is_idempotent(dir.path());
}

#[test]
fn test_BC_1_18_011_obl1_crash_fsync_dir_occ2_post_rename_barrier_resumes_via_treat_done() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    // Occurrence 2 = the FIRST post-rename directory-fsync barrier: the
    // rename already landed (the canonical file physically already holds
    // the NEW content), but the durability barrier and the DONE record
    // are not yet confirmed. Exercises decide_intent_log_recovery's
    // TreatDone arm (canonical hash already == expected_post_hash) rather
    // than a redundant re-rename.
    let crash = spawn_crash_child("migration_fs::fsync_dir", 2, dir.path());
    assert_child_aborted(&crash, "migration_fs::fsync_dir", 2);
    assert_admission_blocked(dir.path(), "probe");

    let outcome = run_recovery_to_convergence(dir.path(), 3);
    assert!(
        matches!(
            outcome,
            Ok(BcIndexMigrationOutcome::Completed {
                canonical_paths_count: 4
            })
        ),
        "got {outcome:?}"
    );
    assert_genuinely_fully_migrated(dir.path());
    assert_recovery_is_idempotent(dir.path());
}

// ===========================================================================
// `remove` boundary -- reachable only via abort_staging / discard_incomplete_
// staging's gen-dir cleanup. Chained two-stage scenario: stage 1 is a real
// partial-staging crash (via write_temp), stage 2 injects an abort inside
// the SECOND invocation's own `fs.remove` call (reached from
// discard_incomplete_staging, triggered by resume_from_staging's mandatory
// census re-run failing on the still-incomplete staged generation).
// ===========================================================================

#[test]
fn test_BC_1_18_011_obl1_crash_remove_during_discard_incomplete_staging_eventually_converges() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());

    let stage1 = spawn_crash_child("migration_fs::write_temp", 3, dir.path());
    assert_child_aborted(&stage1, "migration_fs::write_temp", 3);

    let stage2 = spawn_crash_child("migration_fs::remove", 1, dir.path());
    assert_child_aborted(&stage2, "migration_fs::remove", 1);
    assert_admission_blocked(dir.path(), "probe");

    let txn = read_live_txn_record(&migration_state_dir(dir.path())).unwrap();
    assert_eq!(
        txn.state,
        BcIndexMigrationTxnState::Staging,
        "fs.remove's own doc comment: gen-dir removal is best-effort, but the SURROUNDING \
         txn-state-to-Aborted write must still happen -- here the whole PROCESS aborted mid-way \
         through fs.remove, so that surrounding write genuinely never ran either; the txn is \
         still STAGING, and this MUST still be safely recoverable on a subsequent attempt (the \
         orphaned/partially-removed gen dir is inert per Fs::remove's doc comment)"
    );

    // discard_incomplete_staging's own txn-state write is not swallowed on
    // a genuinely graceful (non-crash) fs.remove failure, but a genuine
    // PROCESS abort loses that too -- a further attempt or two converges.
    let outcome = run_recovery_to_convergence(dir.path(), 4);
    assert!(
        matches!(
            outcome,
            Ok(BcIndexMigrationOutcome::Completed {
                canonical_paths_count: 4
            })
        ),
        "got {outcome:?}"
    );
    assert_genuinely_fully_migrated(dir.path());
    assert_recovery_is_idempotent(dir.path());
}

// ===========================================================================
// `pointer_swap` boundary -- FINDING 1 (dead seam)
// ===========================================================================

#[test]
fn test_BC_1_18_011_obl1_FINDING1_pointer_swap_is_unreachable_dead_seam() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    // If pointer_swap were reachable, the child would abort on its first
    // reach and CHILD_DID_NOT_ABORT_EXIT_CODE would never be observed. It
    // is never called by any production code path today (the actual
    // CURRENT.json commit-rename goes through Fs::write_temp instead --
    // see commit_current_generation_pointer's own doc comment) -- the
    // migration runs to completion and the child hits the sentinel exit
    // code instead of aborting.
    let crash = spawn_crash_child("migration_fs::pointer_swap", 1, dir.path());
    assert!(
        !crash.aborted,
        "migration_fs::pointer_swap fired -- if this assertion ever fails, FINDING 1 has been \
         fixed (the seam is reachable again) and this test should be rewritten as an ordinary \
         crash-injection scenario"
    );
    assert_eq!(
        crash.exit_code,
        Some(CHILD_DID_NOT_ABORT_EXIT_CODE),
        "expected the child to run the full migration to completion without ever reaching \
         migration_fs::pointer_swap; stdout={:?} stderr={:?}",
        crash.stdout,
        crash.stderr,
    );
}

// ===========================================================================
// Per-boundary `return(Err)` graceful (non-crash) failure-path variants.
//
// NOTE: `fail`'s own `"return"` action cannot be used here. Every
// `migration_failpoint!` call site in migration_fs.rs uses the SINGLE-ARG
// `fail_point!($name)` macro form (confirmed from the `fail` 0.5.1 source,
// `fail_point!($name) => { $crate::eval($name, |_| { panic!("Return is not
// supported for the fail point \"{}\"", $name); }); }`) -- configuring a
// `"return"` action against a single-arg fail point does not gracefully
// return an Err from that Fs operation; it PANICS with that literal message
// instead. Genuine `return(Err)` semantics would require upgrading each
// `migration_failpoint!` call site to the TWO-ARG `fail_point!($name, $e)`
// form (with `$e` mapping the configured string to the right
// `BcIndexMigrationError` variant) -- a production-code change outside
// test-writer scope. Surfaced here, not silently worked around with a
// panic-based substitute mislabeled as "graceful".
//
// Substituted mechanism: REAL filesystem faults (a pre-existing non-empty
// directory colliding with a target the migration expects to be a
// plain file/missing) at the two FIXED (non-UUID-bearing) target paths
// where this is deterministically constructible without any call-order
// timing dependency: `CURRENT.json` and a canonical shard path. This
// requires no `failpoints`-feature machinery at all -- these are genuine
// `std::io::Error`s the production `StdFs` returns from ordinary syscalls.
//
// The other 5 named boundaries (`fsync_file`, `fsync_dir`, `append`,
// `remove`, and `write_temp` for the per-run UUID-named staging artifacts
// `gen-<uuid>/`, `intent-<uuid>.log`, `txn-<uuid>.json`) do NOT have fixed,
// predictable target paths a fixture can pre-collide with before the child
// process even starts -- full graceful-Err coverage for those requires the
// same two-arg-macro code hook. Recommended routing: implementer/architect,
// upgrading migration_fs.rs's 7 `migration_failpoint!` call sites to the
// two-arg form.
// ===========================================================================

/// Real I/O fault: `CURRENT.json` pre-exists as a non-empty directory, so
/// the commit-point `Fs::write_temp` call's underlying rename-into-place
/// fails with a genuine `IsADirectory` `io::Error` -- BEFORE any bytes of
/// the pointer file land, so the txn stays STAGING and the fault is
/// entirely non-destructive to everything already durably staged. Confirms
/// the graceful (non-crash) `?`-propagated error path exits cleanly and
/// that the SAME transaction resumes and completes correctly once the
/// transient fault clears (mirrors a real disk-full/permission-denied
/// condition being fixed and the writer retrying).
#[test]
fn test_BC_1_18_011_obl1_graceful_err_current_json_directory_collision_at_commit() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    let msd = migration_state_dir(dir.path());
    std::fs::create_dir_all(&msd).unwrap();
    let current_json_blocker = msd.join("CURRENT.json");
    std::fs::create_dir_all(&current_json_blocker).unwrap();
    std::fs::write(current_json_blocker.join("dummy"), b"x").unwrap();

    let outcome = run_bc_index_migration(dir.path());
    assert!(
        matches!(outcome, Err(BcIndexMigrationError::Io { .. })),
        "expected a genuine graceful Io error from the CURRENT.json directory collision, got \
         {outcome:?}"
    );
    assert_admission_blocked(dir.path(), "probe");
    let txn = read_live_txn_record(&msd).unwrap();
    assert_eq!(
        txn.state,
        BcIndexMigrationTxnState::Staging,
        "a graceful (non-crash) error at the commit-point write must never advance the txn state \
         past STAGING -- the ? propagation exits before txn.state = Committing is ever set"
    );

    // Clear the transient fault and retry -- the SAME transaction resumes
    // and completes.
    std::fs::remove_dir_all(&current_json_blocker).unwrap();
    let outcome2 = run_recovery_to_convergence(dir.path(), 3);
    assert!(
        matches!(
            outcome2,
            Ok(BcIndexMigrationOutcome::Completed {
                canonical_paths_count: 4
            })
        ),
        "got {outcome2:?}"
    );
    assert_genuinely_fully_migrated(dir.path());
    assert_recovery_is_idempotent(dir.path());
}

/// Real I/O fault: a canonical shard target pre-exists as a non-empty
/// directory. This trips `Fs::read`'s error path inside
/// `append_intent_records_for_pending_moves` (computing `expected_pre_state`
/// for that target) -- i.e. the PRE-swap WAL-construction phase, not the
/// later `rename` call itself (a directory collision on the canonical
/// target is read-incompatible before it is ever rename-incompatible).
/// This is still a genuine graceful (non-crash) `Fs` error-path test with
/// real production cleanup: `abort_staging` fires (gen dir removed, txn
/// ABORTED, gate reset to OPEN) since this fault occurs before the pointer
/// swap.
#[test]
fn test_BC_1_18_011_obl1_graceful_err_canonical_shard_directory_collision_pre_swap() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    let shards_root = dir
        .path()
        .join(".factory/specs/behavioral-contracts/shards");
    let blocker = shards_root.join("BC-INDEX-SS-01.md");
    std::fs::create_dir_all(&blocker).unwrap();
    std::fs::write(blocker.join("dummy"), b"x").unwrap();

    let outcome = run_bc_index_migration(dir.path());
    assert!(
        matches!(outcome, Err(BcIndexMigrationError::Io { .. })),
        "expected a genuine graceful Io error from the canonical shard directory collision, got \
         {outcome:?}"
    );

    let msd = migration_state_dir(dir.path());
    let txn = read_live_txn_record(&msd).unwrap();
    assert_eq!(
        txn.state,
        BcIndexMigrationTxnState::Aborted,
        "abort_staging's cleanup must run for a pre-swap graceful I/O failure: gen dir removed, \
         txn ABORTED"
    );
    assert_eq!(
        read_gate_state(&msd),
        BcIndexAdmissionGateState::Open,
        "abort_staging must reset the admission gate to OPEN on a pre-swap graceful failure -- \
         unlike FINDING 3's post-completion scenario, this path correctly self-heals"
    );
    // A fresh writer is admitted again immediately (no crash occurred, and
    // abort_staging's cleanup already ran to completion). Release the
    // probe's own writer reservation immediately afterward -- otherwise
    // it would sit in the reservations dir and cause the NEXT migration
    // attempt's own drain procedure (below) to time out waiting for
    // quiescence, which would be a self-inflicted test artifact, not a
    // product defect.
    let admit = shard_manager::admit_or_block_bc_index_writer(&msd, "post-abort-probe");
    assert!(admit.is_ok(), "got {admit:?}");
    shard_manager::release_bc_index_writer_reservation(&msd, "post-abort-probe")
        .expect("releasing the probe's own writer reservation must succeed");

    // Clear the transient fault and retry -- a brand NEW migration attempt
    // (the prior one is terminally ABORTED) completes cleanly.
    std::fs::remove_dir_all(&blocker).unwrap();
    let outcome2 = run_recovery_to_convergence(dir.path(), 3);
    assert!(
        matches!(
            outcome2,
            Ok(BcIndexMigrationOutcome::Completed {
                canonical_paths_count: 4
            })
        ),
        "got {outcome2:?}"
    );
    assert_genuinely_fully_migrated(dir.path());
    assert_recovery_is_idempotent(dir.path());
}
