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
//!     below); the Postcondition 3a fingerprint recheck now runs LATER
//!     (SEC-001 REDESIGN, see below) -- immediately before each
//!     `Fs::pointer_swap` attempt (occurrence 8's `CURRENT.tmp.json` write
//!     already having happened first)
//!  8. `CURRENT.tmp.json` staging write -- durably stages the new pointer
//!     content at a path DISTINCT from `CURRENT.json` itself (OBL-1
//!     FINDING 1 fix: `commit_current_generation_pointer` now performs
//!     THREE separate steps -- this write, then the actual sole commit-point
//!     rename via `Fs::pointer_swap` below, then a dedicated `Fs::fsync_dir`
//!     durability barrier -- rather than committing `CURRENT.json` directly
//!     through this call the way it used to; see FINDING 1 (RESOLVED). SEC-001
//!     REDESIGN: the Postcondition 3a fingerprint recheck now runs
//!     IMMEDIATELY BEFORE EVERY individual `Fs::pointer_swap` attempt
//!     (including a retry), never after a successful one -- see
//!     `shard_manager::swap_current_generation_pointer_with_precommit_recheck`'s
//!     own doc comment and this file's own SEC-001 test section near the end
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
//! `fsync_dir` (6 occurrences total; `rename` has 4 -- post-swap only):
//!  1. `stage_new_generation`'s own gen-dir sync, BEFORE `generation_id` is
//!     persisted to the txn record
//!  2. NEW (OBL-1 FINDING 1 fix): `commit_current_generation_pointer`'s own
//!     Step 3 durability barrier for the `Fs::pointer_swap` rename's
//!     directory-entry change, immediately after the sole commit-point
//!     rename lands and BEFORE the txn record's `state=Committing` write
//!     (`write_temp` occurrence 9) -- see FINDING 1 (RESOLVED) below
//!  3-6. the 4 post-swap canonical-path-move `rename`+`fsync_dir` barrier
//!     pairs inside `execute_canonical_path_moves` (same order as the
//!     fixture list above) -- occurrence 3 is what this suite's fsync_dir
//!     boundary coverage used to number as occurrence 2, before the new
//!     occurrence 2 above was inserted; see the fsync_dir test section
//!     below for the corrected occurrence-to-scenario mapping
//!
//! `remove` (0 occurrences on the happy path): reachable only via
//! `abort_staging`'s (PC1/PC2 verification failure) or
//! `discard_incomplete_staging`'s (STAGING-resume verification failure)
//! gen-dir cleanup -- see the chained two-stage test below.
//!
//! `pointer_swap` (1 occurrence on an uninterrupted fresh run): the SOLE
//! commit-point for the whole multi-file migration, invoked exactly once
//! per `commit_current_generation_pointer` call -- see FINDING 1
//! (RESOLVED) below.
//!
//! # FINDINGS -- real crash-consistency defects this suite discovered
//! (NOT weakened or papered over; see the module's final report to the
//! orchestrator for full detail and suggested routing)
//!
//! **FINDING 1 (dead seam -- RESOLVED):** `Fs::pointer_swap` was defined,
//! unit-tested in isolation, and documented as "the SOLE commit-point", but
//! was never called by `run_bc_index_migration` or any other production
//! code path -- `commit_current_generation_pointer` performed the actual
//! `CURRENT.json` commit-rename through `Fs::write_temp` directly instead.
//! `migration_fs::pointer_swap` was therefore unreachable from real
//! execution; the ORIGINAL `test_BC_1_18_011_obl1_FINDING1_*` test in this
//! suite documented this empirically rather than fabricating a test against
//! a boundary that could never fire.
//!
//! The implementer has since fixed this (`commit_current_generation_pointer`
//! now performs the three distinct steps its own doc comment always
//! promised: stage the pointer content to `CURRENT.tmp.json` via
//! `Fs::write_temp`, THEN the atomic commit-point rename via the dedicated
//! `Fs::pointer_swap` seam, THEN a `Fs::fsync_dir` durability barrier for
//! that rename). `migration_fs::pointer_swap` is now genuinely reachable
//! from real execution -- the test below (renamed from the original
//! FINDING-1 test) now asserts reachability directly and additionally
//! proves the seam is crash-atomic (old-or-new, never torn) when a crash
//! lands exactly there, rather than merely re-confirming the historical
//! unreachability defect.
//!
//! **FINDING 2 (RESOLVED -- was a real defect, HIGH severity -- silent
//! false-success):** a crash ANYWHERE between "the 4 pending canonical
//! moves are computed / staged" and "`pending_canonical_moves` is
//! persisted to the durable txn record" (i.e. during the `append`#1-4
//! INTENT-record loop, the Postcondition 3a fingerprint recheck, or
//! `write_temp`#6/#7) used to leave the on-disk txn record's
//! `pending_canonical_moves` field at its stale, EMPTY value. On the next
//! invocation, `recover()` correctly classifies this as
//! `ResumeFromStaging` (staging content re-verifies fine -- it's all
//! durably present), but `run_bc_index_migration`'s `ResumeFromStaging`
//! arm used to feed the STALE (empty) `txn.pending_canonical_moves` into
//! `finish_committing_migration` instead of recomputing it, so
//! `execute_canonical_path_moves` iterated zero moves, `completed_count
//! (0) < pending.len() (0)` was FALSE (vacuously), and the migration wrote
//! `completed.json` with `canonical_paths_count: 0` and transitioned the
//! txn to COMPLETED -- **while NONE of the shard files were ever created
//! and the canonical `BC-INDEX.md` was never split.** This was a genuine
//! "old-or-new, never torn" violation at the SYSTEM level: `completed.json`
//! (the reader-integration "is the split done" signal) said NEW while
//! every canonical file on disk still said OLD. The fix
//! (`recompute_pending_canonical_moves_from_staged_generation`) makes the
//! `ResumeFromStaging` arm recompute the pending moves from the staged
//! generation on resume instead of trusting the stale, empty field.
//! `test_BC_1_18_011_obl1_FINDING2_*` tests below reproduce the crash
//! points at the `write_temp`#6/#7 and `append`#1/#4 boundaries and assert
//! the CORRECT invariant, which now PASSES -- these are load-bearing
//! regression guards against FINDING 2 recurring.
//!
//! **FINDING 3 (RESOLVED -- was a real defect, MEDIUM severity --
//! permanent gate lockout):** a crash between `write_completed_record`
//! (NOT `Fs`-seamed, so not itself fault-injectable, but sequenced
//! immediately before `write_temp` occurrence #10 / `fsync_file`
//! occurrence #5) succeeding and the subsequent
//! `write_admission_gate_state(Open)` call used to leave the on-disk
//! writer-admission gate at LOCKED forever: the NEXT (and every
//! subsequent) `run_bc_index_migration` invocation hits the
//! `completed.json`-presence short-circuit (`BcIndexMigrationOutcome::
//! AlreadyMigrated`, checked "no other file consulted", ADR-052 §7c step
//! 8) at the very top of the function, BEFORE `recover()`'s dispatch and
//! BEFORE `finish_committing_migration`'s gate-reset call are ever reached
//! again. `reconcile_stale_admission_gate` cannot self-heal this either --
//! its Branch A requires `active_txn == None`, but a live COMMITTING txn
//! record still exists on disk in this exact scenario. The migration
//! itself was always 100% correct and complete; only the gate used to get
//! permanently stuck, blocking every future writer via
//! `admit_or_block_bc_index_writer` with no automatic recovery path. The
//! fix gives the `completed.json` short-circuit its own best-effort
//! convergence: it now converges the txn record to COMPLETED and resets
//! the gate to OPEN inline, idempotently, every time the short-circuit is
//! hit. `test_BC_1_18_011_obl1_FINDING3_*` tests below reproduce the
//! crash points and assert the CORRECT invariant, which now PASSES --
//! these are load-bearing regression guards against FINDING 3 recurring.
//!
//! All three findings were newly surfaced by this suite (distinct from,
//! and in addition to, the four pass-2-era defects this refactor was
//! built to close) and are NOT papered over: the
//! `test_BC_1_18_011_obl1_FINDING*` tests assert the correct behavior.
//! FINDING 1, FINDING 2, and FINDING 3 are all now RESOLVED -- every
//! `FINDING*` assertion PASSES and serves as a load-bearing regression
//! guard against its respective finding recurring -- see this suite's
//! final report for routing.
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
    // FINDING 2 (RESOLVED): this converges to a genuine, content-correct
    // completion -- prior to the fix it incorrectly reported
    // Ok(Completed { canonical_paths_count: 0 }) with NEITHER shard file
    // ever created. The `ResumeFromStaging` arm now recomputes
    // `pending_canonical_moves` from the staged generation
    // (`recompute_pending_canonical_moves_from_staged_generation`) instead
    // of trusting the stale, empty on-disk field, so this assertion PASSES
    // and is a load-bearing regression guard against FINDING 2 recurring.
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
    // FINDING 2 (RESOLVED -- see module header): prior to the fix, this
    // boundary produced Ok(Completed { canonical_paths_count: 0 }) with
    // completed.json present, canonical BC-INDEX.md UNCHANGED (still the
    // monolithic original), and neither shard file created -- a
    // false-success / "old-or-new, never torn" violation at the system
    // level. The `ResumeFromStaging` arm now recomputes
    // `pending_canonical_moves` from the staged generation on resume, so
    // recovery produces both shards plus the split canonical body and
    // reaches genuine COMPLETED. This assertion PASSES and is a
    // load-bearing regression guard against FINDING 2 recurring, per this
    // suite's mandate to not weaken assertions around real findings.
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
    // FINDING 3 (RESOLVED -- see module header): the admission gate
    // self-heals to OPEN once the migration is genuinely, fully complete
    // -- content is correct (shards exist, census holds), and the
    // completed.json short-circuit now runs its own best-effort
    // convergence of the txn record to COMPLETED and the gate to OPEN
    // inline, rather than leaving both stuck because
    // finish_committing_migration's own gate-reset call is never reached
    // again from this short-circuit. This assertion PASSES and is a
    // load-bearing regression guard against FINDING 3 recurring.
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

/// FINDING 3, occurrence B: same (now-resolved) defect as the
/// write_temp#10 case above, reached via the fsync_file boundary instead
/// (the redundant re-fsync of the already-durable final COMPLETED-state
/// txn record).
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

/// Occurrence 2 was, before the OBL-1 FINDING 1 fix, the FIRST post-rename
/// directory-fsync barrier (the scenario the docstring below used to
/// describe). `commit_current_generation_pointer`'s new Step 3
/// (`Fs::fsync_dir` on `migration_state_dir`, immediately after
/// `Fs::pointer_swap` -- see the module header's corrected `fsync_dir`
/// occurrence table) is now the FIRST `fsync_dir` call reached after
/// `stage_new_generation`'s own occurrence 1, displacing every later
/// occurrence by one. This test now targets THAT boundary; the ORIGINAL
/// post-rename-barrier/TreatDone scenario this test used to cover moved to
/// occurrence 3 (see the next test below), which restores that coverage
/// rather than silently dropping it.
#[test]
fn test_BC_1_18_011_obl1_crash_fsync_dir_occ2_post_pointer_swap_barrier_resumes_via_staging_reinvocation()
 {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    // Occurrence 2 = commit_current_generation_pointer's own Step 3
    // durability barrier: Fs::pointer_swap (the sole commit-point) has
    // ALREADY durably renamed CURRENT.tmp.json onto CURRENT.json -- the
    // new generation is physically live -- but the directory-entry
    // durability barrier for that rename, and the txn record's own
    // state=Committing write (write_temp occurrence 9, which happens
    // strictly AFTER this call returns), have not yet run.
    let crash = spawn_crash_child("migration_fs::fsync_dir", 2, dir.path());
    assert_child_aborted(&crash, "migration_fs::fsync_dir", 2);
    assert_admission_blocked(dir.path(), "probe");

    let txn = read_live_txn_record(&migration_state_dir(dir.path())).unwrap();
    assert_eq!(
        txn.state,
        BcIndexMigrationTxnState::Staging,
        "the on-disk txn record is stale (still says STAGING, since write_temp occurrence 9's \
         state=Committing write happens strictly after commit_current_generation_pointer \
         returns) even though CURRENT.json already durably points at the new generation -- \
         recover()'s ResumeFromStaging arm must safely re-invoke \
         commit_current_generation_pointer (idempotent: re-staging the same tmp content and \
         re-renaming it onto the already-correct target are both no-ops in substance) rather \
         than needing to consult CURRENT.json's own content"
    );
    assert!(
        txn.generation_id.is_some() && txn.pending_canonical_moves.len() == 4,
        "generation_id and pending_canonical_moves are both already durable at this crash point \
         (write_temp occurrences 2 and 7 respectively, both well before pointer_swap ever runs)"
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

/// The ORIGINAL post-rename-barrier/TreatDone scenario, restored at its new
/// occurrence number after the OBL-1 FINDING 1 fix inserted a new
/// `fsync_dir` occurrence ahead of it (see the module header's corrected
/// occurrence table and the occurrence-2 test above).
#[test]
fn test_BC_1_18_011_obl1_crash_fsync_dir_occ3_post_rename_barrier_resumes_via_treat_done() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    // Occurrence 3 = the FIRST post-rename directory-fsync barrier inside
    // execute_canonical_path_moves: the rename already landed (the
    // canonical file physically already holds the NEW content), but the
    // durability barrier and the DONE record are not yet confirmed.
    // Exercises decide_intent_log_recovery's TreatDone arm (canonical hash
    // already == expected_post_hash) rather than a redundant re-rename.
    let crash = spawn_crash_child("migration_fs::fsync_dir", 3, dir.path());
    assert_child_aborted(&crash, "migration_fs::fsync_dir", 3);
    assert_admission_blocked(dir.path(), "probe");

    let txn = read_live_txn_record(&migration_state_dir(dir.path())).unwrap();
    assert_eq!(
        txn.state,
        BcIndexMigrationTxnState::Committing,
        "by occurrence 3, the pointer swap and the txn record's own state=Committing write \
         (write_temp occurrence 9) have both already landed durably -- this crash is strictly \
         post-commit, inside execute_canonical_path_moves' per-target rename+fsync_dir barrier \
         loop"
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
// `pointer_swap` boundary -- FINDING 1 (RESOLVED: was a dead seam, now the
// genuinely wired sole commit-point)
// ===========================================================================

/// The "old-or-new, never torn" property for a crash injected exactly at
/// `Fs::pointer_swap` -- the SOLE commit-point (ADR-052 §Decision 7c step
/// 6; OBL-1 FINDING 1's fix). After recovery converges, the on-disk system
/// state must be EXACTLY ONE of two self-consistent terminal states --
/// never a mix of the two:
///
/// - OLD: the pre-migration canonical `BC-INDEX.md` is byte-identical to
///   the untouched original, NEITHER shard file was ever created, and
///   `completed.json` does not exist -- i.e. recovery safely discarded the
///   staged generation rather than forcing it through (only reachable if
///   the crash landed before `generation_id`/`pending_canonical_moves`
///   were themselves durable -- NOT the case for this suite's specific
///   crash point, several `write_temp` occurrences past both, but this
///   assertion checks the disjunction generically rather than hard-coding
///   that reasoning, per this suite's mandate to never assert a single
///   branch merely because it is the only one a human worked out by hand).
/// - NEW: the migration genuinely, fully completed --
///   [`assert_genuinely_fully_migrated`]'s own strong, content-level check
///   (both shard files present with correct content, canonical
///   `BC-INDEX.md` fully split, COMPLETED txn, OPEN gate, fresh writer
///   admitted).
///
/// Either branch must ALSO satisfy: no live txn record stuck at
/// COMMITTING (a torn, neither-old-nor-new state), the admission gate is
/// OPEN (no permanent lockout), and idempotent re-invocation does not
/// disturb the converged state.
fn assert_pointer_swap_crash_converged_old_or_new_never_torn(dir: &Path) {
    let msd = migration_state_dir(dir);
    let canonical_content = std::fs::read_to_string(bc_index_target(dir)).unwrap();
    let shard1_exists = shard1_path(dir).exists();
    let shard2_exists = shard2_path(dir).exists();
    let completed_exists = completed_json_exists(&msd);

    let looks_old = canonical_content == ORIGINAL_CONTENT && !shard1_exists && !shard2_exists;
    let looks_new = completed_exists && shard1_exists && shard2_exists;

    assert!(
        looks_old ^ looks_new,
        "old-or-new, never torn violated: post-recovery state must be EXACTLY one of the \
         pristine pre-migration original OR the fully-split new content -- never neither, never \
         both. looks_old={looks_old} looks_new={looks_new} completed_exists={completed_exists} \
         shard1_exists={shard1_exists} shard2_exists={shard2_exists} \
         canonical_content={canonical_content:?}"
    );

    let txn = read_live_txn_record(&msd);
    if let Some(txn) = &txn {
        assert_ne!(
            txn.state,
            BcIndexMigrationTxnState::Committing,
            "neither the OLD nor the NEW branch may leave a live txn record stuck at COMMITTING \
             -- that is a torn state, not a converged terminal one"
        );
    }
    assert_eq!(
        read_gate_state(&msd),
        BcIndexAdmissionGateState::Open,
        "recovery from a crash at the sole commit point must never leave the admission gate \
         permanently non-OPEN, regardless of which branch (old/new) it converges to"
    );
    let admit = shard_manager::admit_or_block_bc_index_writer(&msd, "post-recovery-probe");
    assert!(
        admit.is_ok(),
        "a fresh writer must be admitted once recovery from a sole-commit-point crash has \
         converged; got {admit:?}"
    );

    if looks_new {
        assert_genuinely_fully_migrated(dir);
        assert_recovery_is_idempotent(dir);
    } else {
        shard_manager::release_bc_index_writer_reservation(&msd, "post-recovery-probe")
            .expect("releasing the probe's own writer reservation must succeed");
        let before = std::fs::read_to_string(bc_index_target(dir)).unwrap();
        let idempotent_outcome = run_bc_index_migration(dir);
        assert!(
            idempotent_outcome.is_ok(),
            "a further recovery call against an already-converged OLD-branch state must not \
             error; got {idempotent_outcome:?}"
        );
        let after = std::fs::read_to_string(bc_index_target(dir)).unwrap();
        assert_eq!(
            before, after,
            "an already-converged OLD-branch canonical file must not be further mutated by a \
             subsequent call"
        );
    }
}

#[test]
fn test_BC_1_18_011_obl1_pointer_swap_is_the_sole_commit_point_atomic_across_crash() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());

    // (a) Reachability (OBL-1 FINDING 1, now fixed): commit_current_
    // generation_pointer routes the CURRENT.json commit through the
    // dedicated Fs::pointer_swap seam, not the general Fs::write_temp it
    // used before this burst (see that function's own doc comment). A
    // crash injected here on the FIRST reach must genuinely fire (SIGABRT)
    // -- if it instead ran to completion and hit CHILD_DID_NOT_ABORT_
    // EXIT_CODE, the seam would once again be a dead one and this
    // assertion (not a silently-passing no-op) would catch that
    // regression.
    let crash = spawn_crash_child("migration_fs::pointer_swap", 1, dir.path());
    assert_child_aborted(&crash, "migration_fs::pointer_swap", 1);
    assert_admission_blocked(dir.path(), "probe");

    // (b) Crash-atomicity across the sole commit point: recovery, run to
    // convergence exactly as an operator/CI re-running `migrate-bc-index`
    // would, must reach a genuinely terminal outcome...
    let outcome = run_recovery_to_convergence(dir.path(), 3);
    assert!(
        matches!(
            outcome,
            Ok(BcIndexMigrationOutcome::Completed { .. })
                | Ok(BcIndexMigrationOutcome::AlreadyMigrated)
        ),
        "a crash at the sole commit point must converge to a genuinely terminal success outcome \
         (never a permanently stuck error) within a bounded number of operator/CI retries; got \
         {outcome:?}"
    );
    // ...and land on an old-or-new, never-torn state (never a specific
    // branch asserted by fiat -- see the helper's own doc comment for why).
    assert_pointer_swap_crash_converged_old_or_new_never_torn(dir.path());
}

// ===========================================================================
// Per-boundary `return(Err)` graceful (non-crash) failure-path variants.
//
// HISTORICAL NOTE (RESOLVED): `fail`'s own `"return"` action could not be
// used against the ORIGINAL single-arg `fail_point!($name)` macro form every
// `migration_failpoint!` call site in migration_fs.rs used to have --
// configuring a `"return"` action against a single-arg fail point does not
// gracefully return an Err from that Fs operation; it PANICS instead (`fail`
// 0.5.1's own single-arg expansion: `Return is not supported for the fail
// point`). The two tests below (`current_json_directory_collision_at_commit`
// / `canonical_shard_directory_collision_pre_swap`) were written against
// that limitation using REAL filesystem faults (a pre-existing non-empty
// directory colliding with a target the migration expects to be a plain
// file/missing) rather than `fail`-injected ones, at the two FIXED
// (non-UUID-bearing) target paths where that is deterministically
// constructible without any call-order timing dependency.
//
// The implementer has since upgraded every `migration_failpoint!` call site
// to the TWO-ARG `fail_point!($name, $path)` form (see migration_fs.rs's own
// header comment's "Two-arg upgrade" section and `migration_failpoint_error`),
// which maps a configured `return(<tag>)` action's tag to a concrete
// `BcIndexMigrationError::Io` variant. `fail::cfg(name, "1*return(tag)")`
// fires the injected error on exactly the FIRST reach of that named
// boundary in this process (the `1*` count self-neuters after one hit, so
// no explicit teardown is strictly required to avoid leaking into a LATER
// occurrence of the SAME boundary within this same test -- each test below
// still explicitly resets its own failpoint to `"off"` immediately after
// observing the injected error, purely as defense-in-depth against bleeding
// into a DIFFERENT test in this same process should an assertion ever be
// wrong about which occurrence actually got hit). This retires the "requires
// a production-code change outside test-writer scope" limitation this
// comment block used to document -- the two real-I/O-fault tests below are
// KEPT (still valid, complementary coverage exercising genuine OS-level
// errors rather than injected ones); the 7 new tests that follow them cover
// every remaining named `migration_fs::*` boundary via the now-available
// `return(Err)` mechanism.
// ===========================================================================

/// Real I/O fault: `CURRENT.json` pre-exists as a non-empty directory. Post-
/// OBL-1-FINDING-1-fix, the actual commit-point rename is
/// `Fs::pointer_swap` (not `Fs::write_temp` -- `commit_current_generation_
/// pointer`'s Step 1 `Fs::write_temp` call durably stages the new pointer
/// content at the SEPARATE `CURRENT.tmp.json` path first, which does not
/// collide with this fixture's blocker at all and succeeds normally); the
/// directory collision therefore now trips `Fs::pointer_swap`'s underlying
/// `rename(2)` instead, with a genuine `IsADirectory`/`ENOTEMPTY`-class
/// `io::Error` -- still BEFORE any bytes of `CURRENT.json` itself land, so
/// the txn stays STAGING and the fault is entirely non-destructive to
/// everything already durably staged. Confirms the graceful (non-crash)
/// `?`-propagated error path exits cleanly and that the SAME transaction
/// resumes and completes correctly once the transient fault clears (mirrors
/// a real disk-full/permission-denied condition being fixed and the writer
/// retrying).
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
         this path self-heals within the SAME call, unlike FINDING 3's post-completion scenario \
         (now resolved via a dedicated best-effort convergence at the completed.json \
         short-circuit rather than this same-call self-heal)"
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

// ===========================================================================
// `fail::cfg(name, "1*return(tag)")`-injected graceful (non-crash) error-
// path variants, one per named `migration_fs::*` boundary, now that the
// two-arg `fail_point!` upgrade makes genuine `return(Err)` semantics
// available (see the comment block above). `"1*"` fires the injected error
// on exactly the FIRST reach of that named boundary within this process,
// then self-neuters -- each test still explicitly resets its own failpoint
// to `"off"` right after observing the error, as defense-in-depth (this
// suite's own process-isolation discipline -- see the module header comment
// -- means these tests never race a CONCURRENTLY-running test, but multiple
// tests within this SAME `--test-threads=1` binary do still share `fail`'s
// process-global registry across sequential test invocations).
// ===========================================================================

/// `write_temp`, occurrence 1 (the FIRST write_temp reach anywhere in a
/// fresh run: `write_txn_record`'s initial STAGING/`generation_id=None`
/// write). The injected error fires BEFORE `write_temp`'s real bundled
/// durable-write primitive ever runs, so NO txn record lands at all --
/// mirroring the equivalent write_temp-occurrence-1 CRASH test's own
/// "clean restart" shape (no partial state to reconcile, just an
/// unconditional fresh retry).
#[test]
fn test_BC_1_18_011_obl1_graceful_err_write_temp_return_storage_full() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    fail::cfg("migration_fs::write_temp", "1*return(storage_full)")
        .expect("configuring the write_temp return(storage_full) failpoint must succeed");

    let outcome = run_bc_index_migration(dir.path());
    fail::cfg("migration_fs::write_temp", "off")
        .expect("resetting the write_temp failpoint must succeed");
    assert!(
        matches!(
            outcome,
            Err(BcIndexMigrationError::Io { ref source, .. })
                if source.kind() == std::io::ErrorKind::StorageFull
        ),
        "expected a graceful StorageFull Io error injected at the FIRST write_temp reach, got \
         {outcome:?}"
    );

    assert!(
        read_live_txn_record(&migration_state_dir(dir.path())).is_none(),
        "no txn record should exist -- the injected error fired before write_temp's bundled \
         durable-write primitive ever ran, exactly like a crash at this same boundary"
    );
    assert_admission_blocked(dir.path(), "probe");

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

/// `fsync_file`, occurrence 1 (paired with `write_temp` occurrence 1: the
/// SAME initial `write_txn_record` call's redundant re-fsync). Since
/// `write_temp` already performed the FULL bundled durable write before
/// `fsync_file` is even reached (the migration_fs module doc's own
/// production-granularity note), the txn record genuinely IS durable on
/// disk despite the reported error -- proving the "harmless redundant
/// barrier" property holds for the graceful error path too, not just the
/// crash path the equivalent fsync_file-occurrence-1 CRASH test covers.
#[test]
fn test_BC_1_18_011_obl1_graceful_err_fsync_file_return_interrupted() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    fail::cfg("migration_fs::fsync_file", "1*return(interrupted)")
        .expect("configuring the fsync_file return(interrupted) failpoint must succeed");

    let outcome = run_bc_index_migration(dir.path());
    fail::cfg("migration_fs::fsync_file", "off")
        .expect("resetting the fsync_file failpoint must succeed");
    assert!(
        matches!(
            outcome,
            Err(BcIndexMigrationError::Io { ref source, .. })
                if source.kind() == std::io::ErrorKind::Interrupted
        ),
        "expected a graceful Interrupted Io error injected at the FIRST fsync_file reach, got \
         {outcome:?}"
    );

    let msd = migration_state_dir(dir.path());
    let txn = read_live_txn_record(&msd)
        .expect("the txn record must be durable despite the reported fsync_file error");
    assert_eq!(txn.state, BcIndexMigrationTxnState::Staging);
    assert!(
        txn.generation_id.is_none(),
        "recover() must classify this as DiscardPreGeneration on the next call, not \
         'no txn record at all'"
    );
    assert_admission_blocked(dir.path(), "probe");

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

/// `append`, occurrence 1 (the first WAL-boundary INTENT-record append,
/// reached from the fresh-run path after all 4 targets are durably staged
/// but before the pointer swap). Unlike the equivalent process-abort
/// scenario (FINDING 2), a GRACEFUL error here is `?`-propagated through
/// ordinary control flow, so the call site's explicit `abort_staging`
/// cleanup genuinely runs (never skipped the way a killed process skips
/// everything): the staged generation is removed, the txn moves straight
/// to ABORTED, and the gate self-heals to OPEN -- all within this SAME
/// call, no retry needed. This demonstrates FINDING 2 is specific to
/// genuine crash/power-loss semantics, not to ordinary graceful software
/// errors on this exact call path.
#[test]
fn test_BC_1_18_011_obl1_graceful_err_append_return_write_zero_self_heals_via_abort_staging() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    fail::cfg("migration_fs::append", "1*return(write_zero)")
        .expect("configuring the append return(write_zero) failpoint must succeed");

    let outcome = run_bc_index_migration(dir.path());
    fail::cfg("migration_fs::append", "off").expect("resetting the append failpoint must succeed");
    assert!(
        matches!(
            outcome,
            Err(BcIndexMigrationError::Io { ref source, .. })
                if source.kind() == std::io::ErrorKind::WriteZero
        ),
        "expected a graceful WriteZero Io error injected at the FIRST append reach, got \
         {outcome:?}"
    );

    let msd = migration_state_dir(dir.path());
    let txn = read_live_txn_record(&msd)
        .expect("a txn record must exist -- abort_staging's cleanup wrote it to ABORTED");
    assert_eq!(
        txn.state,
        BcIndexMigrationTxnState::Aborted,
        "a graceful (non-crash) append failure must self-heal via abort_staging within the SAME \
         call, unlike a process-killed crash at the same boundary (FINDING 2)"
    );
    assert_eq!(
        read_gate_state(&msd),
        BcIndexAdmissionGateState::Open,
        "abort_staging must reset the admission gate to OPEN immediately, no retry required"
    );
    let admit = shard_manager::admit_or_block_bc_index_writer(&msd, "post-abort-probe");
    assert!(admit.is_ok(), "got {admit:?}");
    shard_manager::release_bc_index_writer_reservation(&msd, "post-abort-probe")
        .expect("releasing the probe's own writer reservation must succeed");

    // A brand NEW migration attempt (the prior one is terminally ABORTED)
    // completes cleanly.
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

/// `rename`, occurrence 1 (the FIRST post-swap canonical-path-move rename
/// inside `execute_canonical_path_moves`, reached from
/// `finish_committing_migration` well after the pointer swap has already
/// durably landed). `execute_canonical_path_moves` does NOT `?`-propagate a
/// rename failure directly -- it halts the loop (never a partial/silent
/// "done" count) and `finish_committing_migration` converts the resulting
/// `completed_count < pending.len()` short-fall into a named
/// `BinaryIntegrityFailure`, never a bare `Io` error and never a silent
/// false-success.
#[test]
fn test_BC_1_18_011_obl1_graceful_err_rename_return_already_exists_forward_recovers() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    fail::cfg("migration_fs::rename", "1*return(already_exists)")
        .expect("configuring the rename return(already_exists) failpoint must succeed");

    let outcome = run_bc_index_migration(dir.path());
    fail::cfg("migration_fs::rename", "off").expect("resetting the rename failpoint must succeed");
    assert!(
        matches!(
            outcome,
            Err(BcIndexMigrationError::BinaryIntegrityFailure { .. })
        ),
        "expected a named BinaryIntegrityFailure forward-recovery error (never a silent \
         false-success) when the FIRST canonical rename gracefully fails, got {outcome:?}"
    );

    let msd = migration_state_dir(dir.path());
    let txn = read_live_txn_record(&msd).unwrap();
    assert_eq!(
        txn.state,
        BcIndexMigrationTxnState::Committing,
        "the txn record correctly stays at COMMITTING -- the pointer swap already landed, and \
         forward recovery (never rollback, Invariant 3) resumes the remaining canonical moves \
         on the next invocation via the intent log's matching-destination-hash rule"
    );
    assert_admission_blocked(dir.path(), "probe");

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

/// `fsync_dir`, occurrence 1 (`stage_new_generation`'s own gen-dir sync,
/// BEFORE `generation_id` is persisted to the txn record) -- mirrors the
/// equivalent fsync_dir-occurrence-1 CRASH test's own DiscardPreGeneration
/// shape via the graceful path instead.
#[test]
fn test_BC_1_18_011_obl1_graceful_err_fsync_dir_return_unexpected_eof_discards() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    fail::cfg("migration_fs::fsync_dir", "1*return(unexpected_eof)")
        .expect("configuring the fsync_dir return(unexpected_eof) failpoint must succeed");

    let outcome = run_bc_index_migration(dir.path());
    fail::cfg("migration_fs::fsync_dir", "off")
        .expect("resetting the fsync_dir failpoint must succeed");
    assert!(
        matches!(
            outcome,
            Err(BcIndexMigrationError::Io { ref source, .. })
                if source.kind() == std::io::ErrorKind::UnexpectedEof
        ),
        "expected a graceful UnexpectedEof Io error injected at the FIRST fsync_dir reach, got \
         {outcome:?}"
    );

    let msd = migration_state_dir(dir.path());
    let txn = read_live_txn_record(&msd).unwrap();
    assert_eq!(txn.state, BcIndexMigrationTxnState::Staging);
    assert!(
        txn.generation_id.is_none(),
        "the gen dir may already physically exist (mkdir succeeded before the fsync_dir call), \
         but generation_id is not yet persisted -- DiscardPreGeneration must fire on the next \
         invocation, mirroring the equivalent CRASH scenario at this same boundary"
    );
    assert_admission_blocked(dir.path(), "probe");

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

/// `pointer_swap`, its ONE occurrence per run -- `commit_current_generation_
/// pointer`'s Step 2, the sole commit-point. The injected error fires
/// BEFORE the real `rename(2)` syscall runs (the macro invocation is the
/// literal first statement of `Fs::pointer_swap`), so `CURRENT.json` is
/// never touched, but `CURRENT.tmp.json` (Step 1) is already durable and
/// `generation_id`/`pending_canonical_moves` were both persisted several
/// `write_temp` occurrences earlier -- the graceful-path counterpart of the
/// `write_temp`-occurrence-8 CRASH test's "exactly at the sole commit
/// point" scenario.
/// SEC-001 REDESIGN: before the redesign, `StdFs::pointer_swap` was backed
/// by `rename_with_retry`, which is a bare, non-retrying passthrough to
/// `std::fs::rename` on every non-Windows target (see that function's own
/// `#[cfg(not(windows))]` arm) -- so on THIS test's platform, a single
/// injected `PermissionDenied` always propagated immediately, with no
/// retry at all, requiring a SECOND, separate `run_bc_index_migration`
/// invocation to resume from STAGING (hence this test's original name).
///
/// Post-redesign, `swap_current_generation_pointer_with_precommit_recheck`
/// drives its OWN retry-with-precommit-recheck loop around
/// `Fs::pointer_swap` -- deliberately platform-uniform, not
/// `#[cfg(windows)]`-gated (see that function's own doc comment for why) --
/// so a single retryable `PermissionDenied` is now transparently retried
/// and self-heals WITHIN the same `run_bc_index_migration` call, on every
/// platform, never surfacing to the caller at all. This is a genuine,
/// intended behavioral improvement (the whole point of moving the retry
/// loop to this layer), not a regression: this test is renamed and rewritten
/// to assert the new, stronger guarantee.
#[test]
fn test_BC_1_18_011_obl1_graceful_err_pointer_swap_return_permission_denied_retries_and_converges_within_one_call()
 {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    fail::cfg("migration_fs::pointer_swap", "1*return(permission_denied)")
        .expect("configuring the pointer_swap return(permission_denied) failpoint must succeed");

    let outcome = run_bc_index_migration(dir.path());
    fail::cfg("migration_fs::pointer_swap", "off")
        .expect("resetting the pointer_swap failpoint must succeed");
    assert!(
        matches!(
            outcome,
            Ok(BcIndexMigrationOutcome::Completed {
                canonical_paths_count: 4
            })
        ),
        "a single transient (retryable) PermissionDenied at the sole commit-point must now be \
         absorbed by swap_current_generation_pointer_with_precommit_recheck's own retry loop \
         WITHIN this one call -- no second invocation should be needed; got {outcome:?}"
    );
    assert_genuinely_fully_migrated(dir.path());
    assert_recovery_is_idempotent(dir.path());
}

/// `remove`, reached (like the chained CRASH scenario above it in this
/// file) only via `discard_incomplete_staging`'s gen-dir cleanup after a
/// genuine partial-staging failure. Stage 1 is a real crash (write_temp
/// occurrence 3, exactly like the chained remove CRASH test); stage 2
/// injects a GRACEFUL `fs.remove` failure into the SECOND invocation's own
/// cleanup call. `Fs::remove`'s own doc comment: "a failure here must
/// never be treated as migration-correctness failure" -- confirms the
/// call site genuinely swallows it (best-effort) rather than propagating
/// it, and that `discard_incomplete_staging`'s own txn-state-to-ABORTED
/// write is NOT swallowed alongside it.
#[test]
fn test_BC_1_18_011_obl1_graceful_err_remove_return_out_of_memory_best_effort_cleanup_still_converges()
 {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());

    let stage1 = spawn_crash_child("migration_fs::write_temp", 3, dir.path());
    assert_child_aborted(&stage1, "migration_fs::write_temp", 3);

    fail::cfg("migration_fs::remove", "1*return(out_of_memory)")
        .expect("configuring the remove return(out_of_memory) failpoint must succeed");
    let outcome = run_bc_index_migration(dir.path());
    fail::cfg("migration_fs::remove", "off").expect("resetting the remove failpoint must succeed");

    assert!(
        outcome.is_err(),
        "the resume attempt must still fail (the staged generation genuinely IS incomplete, per \
         resume_from_staging's own mandatory census re-run) -- got {outcome:?}"
    );
    let msd = migration_state_dir(dir.path());
    let txn = read_live_txn_record(&msd)
        .expect("a txn record must exist -- discard_incomplete_staging still writes it");
    assert_eq!(
        txn.state,
        BcIndexMigrationTxnState::Aborted,
        "discard_incomplete_staging's own txn-state-to-ABORTED write is NOT swallowed by a \
         best-effort fs.remove failure -- only the (optional, non-fatal) gen-dir removal step \
         is best-effort"
    );
    assert_admission_blocked(dir.path(), "probe");

    // A further attempt converges (mirrors the equivalent process-abort
    // chained scenario above) -- the orphaned, non-removed gen dir is
    // inert per Fs::remove's own doc comment and never revisited by a
    // fresh run.
    let outcome2 = run_recovery_to_convergence(dir.path(), 4);
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

// ===========================================================================
// SEC-001 REDESIGN (CWE-367/CWE-362) -- through-`run_bc_index_migration`
// proof that the fingerprint-precheck-before-every-attempt redesign closes
// the permanent-deadlock defect a fresh-eyes pr-reviewer pass found in PR
// #842's original post-swap-recheck fix. See
// `shard_manager::swap_current_generation_pointer_with_precommit_recheck`'s
// own doc comment for the full mechanism, and
// `bc_1_18_011_b2_migration_test.rs`'s own
// `test_BC_1_18_011_SEC001_commit_current_generation_pointer_catches_mutation_between_retried_swap_attempts`
// for the fully deterministic proof (via a hand-rolled `Fs`) that the
// precommit recheck specifically re-runs before a SECOND (retried)
// `Fs::pointer_swap` attempt, catching a mutation landing in that exact
// gap. THIS test instead proves the same abort-routing end-to-end through
// the REAL `run_bc_index_migration` production entry point, which cannot
// inject a custom `Fs` (its `StdFs` is constructed internally).
//
// # Deterministic mutation timing -- no wall-clock race
//
// A first attempt at this test used a background thread with a fixed sleep
// before mutating the canonical file, racing it against
// `run_bc_index_migration`'s own internal timing. That was unreliable in
// practice: the fresh-run path's staging phase (reading the original
// content, splitting sections, running PC1/PC2 census checks, appending
// WAL intent-log records -- several real, individually-fsynced writes)
// has no fixed upper bound, so a short sleep can land the mutation BEFORE
// the migration ever reads its own original content, corrupting the
// fixture instead of exercising the intended race.
//
// This test instead exploits `migration_fs::write_temp`'s own occurrence
// numbering (documented at the top of this file): occurrence 8, for this
// file's standard 2-subsystem fixture, is ALWAYS the `CURRENT.tmp.json`
// staging write -- the LAST write `swap_current_generation_pointer_with_
// precommit_recheck` performs before its retry loop runs its FIRST
// precommit fingerprint recheck. A `fail::cfg_callback` that mutates the
// canonical file on exactly that occurrence lands the mutation
// synchronously, on the SAME thread, guaranteed strictly AFTER every prior
// staging step (which already consumed the untouched original content)
// and strictly BEFORE the first precommit recheck -- zero race, no sleep,
// no thread. This exercises the identical downstream abort-routing code
// path as the retry-widened race (the SAME `pre_commit_fingerprint_recheck`
// call, inside the SAME loop, feeding the SAME `abort_staging` closure) --
// which loop iteration first detects the mismatch does not change what is
// being proven here: that detection anywhere in this loop correctly aborts
// and reopens the gate, rather than deadlocking.
// ===========================================================================

/// A non-participating writer (one bypassing the migration's advisory
/// `exclusive.lock` flock) mutates the canonical `BC-INDEX.md` after every
/// staging step has completed but strictly before the first precommit
/// fingerprint recheck (see this section's own header comment for why
/// `write_temp` occurrence 8 is the deterministic hook). Proves, through
/// the REAL `run_bc_index_migration` entry point (not a direct
/// `commit_current_generation_pointer`/`swap_current_generation_pointer_
/// with_precommit_recheck` unit call): (1) the migration detects this and
/// returns a genuine abort, never a silent success and never a hang; (2)
/// `txn.state` reaches `Aborted`, not stuck at `Staging`; (3) the
/// admission gate is reopened to `Open`, not left `Locked`; (4)
/// `CURRENT.json` never exists at all for the mutated generation (no
/// reader is ever exposed to a partially-committed `status:"committing"`
/// pointer); and (5) -- the property that actually distinguishes this
/// from the broken PR #842 fix -- a SUBSEQUENT `run_bc_index_migration`
/// invocation against the SAME `.factory/` fixture genuinely converges,
/// proving the deadlock this redesign closes is gone, not merely that the
/// mismatch check fires once.
#[test]
fn test_BC_1_18_011_SEC001_run_bc_index_migration_detects_concurrent_writer_mutation_before_pointer_swap()
 {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    let canonical_path = bc_index_target(dir.path());
    let msd = migration_state_dir(dir.path());

    let occurrence = AtomicUsize::new(0);
    let mutate_path = canonical_path.clone();
    fail::cfg_callback("migration_fs::write_temp", move || {
        let n = occurrence.fetch_add(1, Ordering::SeqCst) + 1;
        // Occurrence 8 (this file's own fixture/occurrence table, module
        // doc comment) is the `CURRENT.tmp.json` staging write -- the
        // deterministic hook this section's header comment explains.
        if n == 8 {
            std::fs::write(
                &mutate_path,
                "## a non-participating writer's mutation, bypassing exclusive.lock\n",
            )
            .expect("SEC-001 fixture: simulated concurrent-writer mutation must succeed");
        }
    })
    .expect("configuring the write_temp callback failpoint must succeed");

    let outcome = run_bc_index_migration(dir.path());
    fail::cfg("migration_fs::write_temp", "off")
        .expect("resetting the write_temp failpoint must succeed");

    // (1) genuine abort, never a silent success or a hang.
    assert!(
        matches!(
            outcome,
            Err(BcIndexMigrationError::FingerprintMismatchAbort)
        ),
        "SEC-001: a concurrent writer's mutation landing strictly before the first pointer-swap \
         attempt's precommit recheck must be caught before ANY swap attempt commits -- got \
         {outcome:?}"
    );

    // (2) txn.state reaches Aborted, never stuck at Staging.
    let txn = read_live_txn_record(&msd)
        .expect("a txn record must exist -- the fresh-run path's abort_staging still writes it");
    assert_eq!(
        txn.state,
        BcIndexMigrationTxnState::Aborted,
        "SEC-001 REDESIGN: a fingerprint mismatch detected inside \
         swap_current_generation_pointer_with_precommit_recheck must route through the SAME \
         abort_staging cleanup as every other pre-swap Postcondition 3a gate -- a txn stuck at \
         STAGING here is exactly the permanent-deadlock defect this redesign closes"
    );

    // (3) the admission gate is reopened, never left LOCKED.
    assert_eq!(
        read_gate_state(&msd),
        BcIndexAdmissionGateState::Open,
        "SEC-001 REDESIGN: the writer-admission gate must be reopened to OPEN on this abort path, \
         not left LOCKED forever"
    );
    // Release the probe's own writer reservation immediately afterward --
    // otherwise it would sit in the reservations dir and cause the NEXT
    // migration attempt's own drain procedure (step (5) below) to time out
    // waiting for quiescence, which would be a self-inflicted test
    // artifact, not a product defect (see the equivalent pattern in this
    // file's own `graceful_err_current_json_directory_collision_at_commit`
    // test).
    let admit = shard_manager::admit_or_block_bc_index_writer(&msd, "post-abort-probe");
    assert!(
        admit.is_ok(),
        "a fresh ordinary writer must be admitted once the gate has genuinely reopened -- got \
         {admit:?}"
    );
    shard_manager::release_bc_index_writer_reservation(&msd, "post-abort-probe")
        .expect("releasing the probe's own writer reservation must succeed");

    // (4) CURRENT.json never exists at all for the mutated generation -- no
    // reader was ever exposed to a partially-committed pointer.
    assert!(
        !msd.join("CURRENT.json").exists(),
        "SEC-001: the pointer swap must never have landed for the mutated generation -- \
         CURRENT.json must not exist (readers switch to \"committing\" the instant it does, per \
         the reader-state function's own doc comment, so its mere absence here is the load-\
         bearing assertion, not merely its content)"
    );

    // (5) the property that actually distinguishes this from the broken PR
    // #842 fix: a SUBSEQUENT invocation against the SAME fixture genuinely
    // converges. The concurrent writer's own (deliberately malformed,
    // non-BC-INDEX-shaped) mutation is restored to a well-formed body first
    // -- this test isolates "does the migration SYSTEM (txn/gate state)
    // converge" from "can a fresh migration parse this writer's arbitrary
    // content", which is an orthogonal, already-covered concern elsewhere
    // in this suite.
    std::fs::write(&canonical_path, ORIGINAL_CONTENT).unwrap();
    let outcome2 = run_recovery_to_convergence(dir.path(), 3);
    assert!(
        matches!(
            outcome2,
            Ok(BcIndexMigrationOutcome::Completed {
                canonical_paths_count: 4
            })
        ),
        "SEC-001 REDESIGN: a further run_bc_index_migration invocation against the same \
         .factory/ fixture must genuinely converge -- a permanent deadlock here (every future \
         call hitting the exact same abort forever) is exactly what the broken PR #842 fix \
         produced; got {outcome2:?}"
    );
    assert_genuinely_fully_migrated(dir.path());
    assert_recovery_is_idempotent(dir.path());
}

// ===========================================================================
// SEC-001 v3 (fresh-eyes pr-reviewer finding on PR #842's v2 redesign
// above): the STAGING-resume arm's forward-recovery-vs-abort
// disambiguation. `shard_manager::read_current_generation_pointer_if_
// present` -- called at the very top of `run_bc_index_migration`'s
// `ResumeFromStaging` arm, before `resume_from_staging`'s census re-run and
// before the recompute/recheck/swap sequence -- disambiguates "the swap for
// THIS generation genuinely never happened yet" (the pre-existing
// recheck-before-swap-then-abort-on-mismatch logic remains correct) from
// "a PRIOR (crashed) invocation already drove `Fs::pointer_swap` to success
// before crashing strictly before `Fs::fsync_dir`/`state=Committing`" (which
// must proceed via forward recovery ONLY -- Invariant 3, "no turning back"
// -- never through `discard_incomplete_staging`'s gen-dir deletion).
// ===========================================================================

/// Requirement (1) of the v3 regression suite: a STAGING-resume where the
/// pointer swap has NOT yet happened for this generation (crashed strictly
/// before `write_temp` occurrence 8's `CURRENT.tmp.json` staging write ever
/// ran -- see this file's own module-doc occurrence table; occurrence 7 is
/// the boundary FINDING 2's own `test_BC_1_18_011_obl1_FINDING2_crash_
/// write_temp_occ7_pending_moves_not_yet_persisted` test already proves
/// resumes correctly WITHOUT a mutation), followed by a non-participating
/// writer's mutation of the canonical `BC-INDEX.md` before the resume
/// attempt. `read_current_generation_pointer_if_present` must find NO
/// `CURRENT.json` at all for this generation (the swap was never attempted)
/// and correctly fall through to the pre-existing recheck-before-swap
/// logic, which must still abort exactly as it always has -- this is the
/// REGRESSION GUARD proving the v3 fix's new early-return branch does not
/// accidentally widen to cover the "genuinely not yet committed" case too.
#[test]
fn test_BC_1_18_011_SEC001_v3_resume_mutated_source_no_prior_swap_aborts_and_converges() {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    let canonical_path = bc_index_target(dir.path());
    let msd = migration_state_dir(dir.path());

    // Crash strictly BEFORE the pointer swap is ever attempted (write_temp
    // occurrence 7 -- the txn record write persisting pending_canonical_
    // moves, itself strictly before occurrence 8's CURRENT.tmp.json write).
    let crash = spawn_crash_child("migration_fs::write_temp", 7, dir.path());
    assert_child_aborted(&crash, "migration_fs::write_temp", 7);
    assert_admission_blocked(dir.path(), "probe");

    let txn = read_live_txn_record(&msd).unwrap();
    assert_eq!(txn.state, BcIndexMigrationTxnState::Staging);
    assert!(
        !msd.join("CURRENT.json").exists(),
        "precondition: the pointer swap must never have been attempted at this crash point"
    );

    // Simulate a non-participating writer's mutation landing between the
    // crash and the resume attempt.
    std::fs::write(
        &canonical_path,
        "## a non-participating writer's mutation, bypassing exclusive.lock\n",
    )
    .unwrap();

    let outcome = run_bc_index_migration(dir.path());
    assert!(
        matches!(
            outcome,
            Err(BcIndexMigrationError::FingerprintMismatchAbort)
        ),
        "SEC-001 v3: a STAGING-resume where the swap genuinely never happened yet must still \
         detect the mutated source and abort exactly as before the v3 fix -- got {outcome:?}"
    );

    let txn = read_live_txn_record(&msd)
        .expect("a txn record must exist -- discard_incomplete_staging still writes it");
    assert_eq!(
        txn.state,
        BcIndexMigrationTxnState::Aborted,
        "SEC-001 v3: the not-yet-committed case must still route through discard_incomplete_\
         staging -- txn.state stuck at STAGING here would be the permanent-deadlock defect this \
         module's SEC-001 REDESIGN section already closed"
    );
    assert_eq!(
        read_gate_state(&msd),
        BcIndexAdmissionGateState::Open,
        "SEC-001 v3: the admission gate must be reopened on this genuinely-not-committed abort \
         path"
    );
    assert!(
        !msd.join("CURRENT.json").exists(),
        "SEC-001 v3: no live CURRENT.json for this generation -- the swap never landed"
    );

    // A subsequent invocation, once the mutation is corrected, genuinely
    // converges.
    std::fs::write(&canonical_path, ORIGINAL_CONTENT).unwrap();
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

/// Requirement (2) of the v3 regression suite -- the actual defect under
/// test: a STAGING-resume where the pointer swap ALREADY landed in a prior
/// crashed invocation (crashed strictly between `Fs::pointer_swap`
/// succeeding and `Fs::fsync_dir`/`state=Committing` durably landing -- the
/// SAME crash window `test_BC_1_18_011_obl1_crash_fsync_dir_occ2_post_
/// pointer_swap_barrier_resumes_via_staging_reinvocation` above exercises),
/// followed by a non-participating writer's mutation of the canonical
/// `BC-INDEX.md` before the resume attempt.
///
/// Before the v3 fix, this scenario made the `ResumeFromStaging` arm
/// re-run the pre-swap fingerprint recheck against the now-mutated
/// canonical content, observe a mismatch, and route through
/// `discard_incomplete_staging` -- DELETING the already-committed
/// generation directory while `CURRENT.json` still pointed at it, a
/// rollback-after-the-commit-point Invariant 3 forbids (empirically
/// reproduced by a fresh-eyes pr-reviewer pass on PR #842's v2 redesign).
/// `read_current_generation_pointer_if_present` must detect that
/// `CURRENT.json` already names this transaction's generation and txn, and
/// skip the recheck/swap entirely, proceeding straight to forward recovery.
///
/// # Honest scope note on this test's own final-outcome assertion
///
/// The mutated canonical `BC-INDEX.md` is ALSO one of this migration's own
/// pending canonical-path-move TARGETS (the lean split body). Once forward
/// recovery reaches `execute_canonical_path_moves`, `decide_intent_log_
/// recovery`'s own (separate, pre-existing, correct) fail-closed table
/// legitimately halts THAT one target's rename -- the on-disk canonical
/// content now matches neither the recorded `expected_pre_state` nor
/// `expected_post_hash`, exactly as it would for ANY concurrently-mutated
/// rename target, v3 fix or not (this is `execute_canonical_path_moves`'s
/// own, separate, already-correct fail-closed contract -- not something the
/// v3 fix changes or needs to change). This test therefore asserts the
/// property the v3 fix actually controls on the FIRST resume call (no
/// abort, no gen-dir deletion, `CURRENT.json` stays intact and correct),
/// then clears the interference and asserts genuine, full convergence on a
/// SECOND call -- rather than asserting a same-call full `Completed`
/// outcome that the system's own concurrent-writer-safety contract (a
/// separate, correct behavior) does not promise while the interference is
/// still physically present on disk.
#[test]
fn test_BC_1_18_011_SEC001_v3_resume_mutated_source_after_prior_swap_forward_recovers_never_deletes_generation()
 {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    let canonical_path = bc_index_target(dir.path());
    let msd = migration_state_dir(dir.path());

    let crash = spawn_crash_child("migration_fs::fsync_dir", 2, dir.path());
    assert_child_aborted(&crash, "migration_fs::fsync_dir", 2);
    assert_admission_blocked(dir.path(), "probe");

    let txn_before = read_live_txn_record(&msd).unwrap();
    assert_eq!(
        txn_before.state,
        BcIndexMigrationTxnState::Staging,
        "precondition: the txn record is stale (still STAGING) at this crash point even though \
         CURRENT.json already durably points at the new generation"
    );
    let generation_id = txn_before
        .generation_id
        .clone()
        .expect("precondition: generation_id must already be durable at this crash point");
    let gen_dir = msd.join(format!("gen-{generation_id}"));
    assert!(
        gen_dir.exists(),
        "precondition: the staged generation directory must still exist"
    );
    let current_before: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(msd.join("CURRENT.json"))
            .expect("precondition: CURRENT.json must already exist -- the swap already landed"),
    )
    .unwrap();
    assert_eq!(
        current_before.get("generation_id").and_then(|v| v.as_str()),
        Some(generation_id.as_str()),
        "precondition: CURRENT.json must already name this generation"
    );

    // Simulate a non-participating writer's mutation landing between the
    // crash and the resume attempt.
    std::fs::write(
        &canonical_path,
        "## a non-participating writer's mutation, bypassing exclusive.lock\n",
    )
    .unwrap();

    let outcome = run_bc_index_migration(dir.path());
    assert!(
        !matches!(
            outcome,
            Err(BcIndexMigrationError::FingerprintMismatchAbort)
        ),
        "SEC-001 v3 REGRESSION: a swap that ALREADY committed in a prior crashed invocation must \
         NEVER be re-classified as \"never happened\" just because the canonical source was \
         mutated afterward -- got {outcome:?}"
    );

    // The generation directory must NEVER be deleted -- this is the actual
    // defect the v3 fix closes (a rollback-after-the-commit-point).
    assert!(
        gen_dir.exists(),
        "SEC-001 v3 REGRESSION: the already-committed generation directory must never be \
         deleted just because a later fingerprint recheck against MUTATED content would (if \
         mistakenly re-run) report a mismatch"
    );

    // CURRENT.json must still exist and still correctly name this exact
    // generation and transaction -- never reverted, never left dangling.
    let current_after_content = std::fs::read_to_string(msd.join("CURRENT.json"))
        .expect("SEC-001 v3 REGRESSION: CURRENT.json must still exist after this resume attempt");
    let current_after: serde_json::Value = serde_json::from_str(&current_after_content).unwrap();
    assert_eq!(
        current_after.get("generation_id").and_then(|v| v.as_str()),
        Some(generation_id.as_str()),
        "SEC-001 v3 REGRESSION: CURRENT.json must still correctly point at the already-committed \
         generation"
    );
    assert_eq!(
        current_after.get("txn_id").and_then(|v| v.as_str()),
        Some(txn_before.txn_id.as_str()),
        "SEC-001 v3 REGRESSION: CURRENT.json's txn_id must be unchanged"
    );

    // The txn record must have advanced FORWARD (to Committing), never
    // backward to Aborted -- Invariant 3, "no turning back".
    let txn_after = read_live_txn_record(&msd).unwrap();
    assert_ne!(
        txn_after.state,
        BcIndexMigrationTxnState::Aborted,
        "SEC-001 v3 REGRESSION: forward recovery from an already-committed swap must never reach \
         the Aborted state -- got {:?}",
        txn_after.state
    );

    // Once the interfering mutation is corrected, a further invocation
    // genuinely, fully converges -- proving this is forward recovery, not a
    // stuck halt or a silently-lost generation.
    std::fs::write(&canonical_path, ORIGINAL_CONTENT).unwrap();
    let outcome2 = run_recovery_to_convergence(dir.path(), 3);
    assert!(
        matches!(
            outcome2,
            Ok(BcIndexMigrationOutcome::Completed {
                canonical_paths_count: 4
            })
        ),
        "SEC-001 v3: forward recovery must genuinely converge once the interfering mutation \
         clears -- got {outcome2:?}"
    );
    assert_genuinely_fully_migrated(dir.path());
    assert_recovery_is_idempotent(dir.path());
}

// ===========================================================================
// SEC-004 (CWE-703, LOW/advisory) -- an `Io` error from the fingerprint
// recheck's OWN read of the canonical source (as opposed to a write-side
// `Fs::write_temp`/`Fs::pointer_swap` fault) must be routed through the
// same abort-and-reopen-gate handling as a genuine `FingerprintMismatchAbort`
// -- never left at STAGING with the gate LOCKED forever, restoring parity
// with the pre-v2 baseline's uniform treatment of any fingerprint-recheck
// failure.
// ===========================================================================

/// Deletes the canonical `BC-INDEX.md` (rather than mutating its content)
/// at the exact same deterministic hook this file's own SEC-001 section
/// uses (`write_temp` occurrence 8, the `CURRENT.tmp.json` staging write --
/// the last write before the precommit recheck loop's first iteration), so
/// `pre_commit_fingerprint_recheck`'s own `std::fs::read` fails with a
/// genuine `NotFound` `Io` error rather than a content mismatch. Proves the
/// SEC-004 fix routes this the SAME way `FingerprintMismatchAbort` already
/// is, through the REAL `run_bc_index_migration` fresh-run path.
#[test]
fn test_BC_1_18_011_SEC004_run_bc_index_migration_recheck_source_read_io_error_aborts_like_mismatch()
 {
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    let canonical_path = bc_index_target(dir.path());
    let msd = migration_state_dir(dir.path());

    let occurrence = AtomicUsize::new(0);
    let delete_path = canonical_path.clone();
    fail::cfg_callback("migration_fs::write_temp", move || {
        let n = occurrence.fetch_add(1, Ordering::SeqCst) + 1;
        // Occurrence 8 (this file's own fixture/occurrence table, module
        // doc comment) is the `CURRENT.tmp.json` staging write -- the
        // deterministic hook this section's header comment explains.
        if n == 8 {
            std::fs::remove_file(&delete_path)
                .expect("SEC-004 fixture: deleting the canonical source must succeed");
        }
    })
    .expect("configuring the write_temp callback failpoint must succeed");

    let outcome = run_bc_index_migration(dir.path());
    fail::cfg("migration_fs::write_temp", "off")
        .expect("resetting the write_temp failpoint must succeed");

    assert!(
        matches!(outcome, Err(BcIndexMigrationError::Io { .. })),
        "SEC-004: a source-read I/O fault at the precommit recheck moment must surface as an Io \
         error (never silently swallowed, never a hang) -- got {outcome:?}"
    );
    if let Err(BcIndexMigrationError::Io { path, .. }) = &outcome {
        assert_eq!(
            path, &canonical_path,
            "SEC-004: the Io error must be the recheck's OWN read of the canonical source, not \
             some other path"
        );
    }

    let txn = read_live_txn_record(&msd)
        .expect("a txn record must exist -- the fresh-run path's abort_staging still writes it");
    assert_eq!(
        txn.state,
        BcIndexMigrationTxnState::Aborted,
        "SEC-004: a source-read Io error at the recheck moment must be routed through the SAME \
         abort_staging cleanup as a fingerprint mismatch -- leaving txn.state stuck at STAGING \
         here would wedge the migration with the gate LOCKED forever, exactly the pre-v2-parity \
         gap SEC-004 closes"
    );
    assert_eq!(
        read_gate_state(&msd),
        BcIndexAdmissionGateState::Open,
        "SEC-004: the writer-admission gate must be reopened, not left LOCKED forever"
    );
    assert!(
        !msd.join("CURRENT.json").exists(),
        "SEC-004: the pointer swap must never have landed"
    );

    // A subsequent invocation, once the source is restored, genuinely
    // converges.
    std::fs::write(&canonical_path, ORIGINAL_CONTENT).unwrap();
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
