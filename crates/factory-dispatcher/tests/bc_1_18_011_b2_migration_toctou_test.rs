// Test files use .expect()/.unwrap()/.panic!() for failure reporting,
// matching bc_1_18_011_b2_migration_test.rs's own established convention.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! TOCTOU regression suite for `migrate-bc-index`: `completed.json` (and every other
//! state file) is read ONLY under `flock(exclusive.lock)`.
//!
//! Spec anchors: ADR-052 v1.23 item 11(c); BC-1.18.011 v1.20 EC-052 and Postcondition
//! 9(e)(a0); BC-1.18.013 v1.12 EC-054 (lock-then-read precedence).
//!
//! `--features factory-dispatcher/failpoints` only (the whole file is cfg-gated).
//!
//! # Seams
//!
//! `run_bc_index_migration_core` carries two `failpoints`-gated fail points, both using
//! the single-argument `fail::fail_point!` form (so only `cfg_callback` / `sleep` /
//! `pause` / `panic` actions are valid here; a `return(..)` action would panic):
//!
//! * `bc_index_migration_core::before_lock` -- after `exclusive.lock` exists, immediately
//!   BEFORE the flock attempt: the exact point where the retired pre-lock
//!   `completed.json` probe used to sit. A callback here simulates "a coordinator
//!   finished between what used to be the probe and the lock".
//! * `bc_index_migration_core::after_lock_before_state_read` -- lock HELD, before the
//!   first state read.
//!
//! # Mutant record (both proven red against the production code, then reverted)
//!
//! * (a) mutant M1: reintroduce a pre-lock `completed.json` existence probe immediately
//!   BEFORE the `before_lock` fail point (the retired code shape) and replace the
//!   under-lock read with a hard-coded "absent". The probe sees "absent", the callback
//!   then materializes `completed.json` + a COMPLETED txn, the lock is acquired, and the
//!   run starts a FRESH migration on the already-migrated tree => test (a) fails.
//! * (b) mutant M2: move the `completed.json` existence read to BEFORE
//!   `try_acquire_migration_lock` (ahead of both fail points). The read happens
//!   pre-lock, so the state the callback injects under the lock is never observed and a
//!   fresh migration starts => test (b) fails.

#![cfg(feature = "failpoints")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use factory_dispatcher::shard_manager::{
    BcIndexMigrationError, BcIndexMigrationOutcome, run_bc_index_migration,
};

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

type TreeSnapshot = BTreeMap<String, Vec<u8>>;

const BEFORE_LOCK: &str = "bc_index_migration_core::before_lock";
const AFTER_LOCK: &str = "bc_index_migration_core::after_lock_before_state_read";

/// Fail-point isolation: `fail`'s registry is process-global, so every test takes this
/// scope as its first statement. `FailScenario::setup()` holds the crate's global
/// scenario mutex for the whole test (serializing in-process scenarios) and clears every
/// fail point on drop, including on a panic unwind.
fn fail_point_scope() -> fail::FailScenario<'static> {
    fail::FailScenario::setup()
}

fn migration_state_dir(cwd: &Path) -> PathBuf {
    cwd.join(".factory/migration-state")
}

fn canonical_bc_index(cwd: &Path) -> PathBuf {
    cwd.join(".factory/specs/behavioral-contracts/BC-INDEX.md")
}

fn setup_fixture(cwd: &Path) {
    let factory_dir = cwd.join(".factory");
    std::fs::create_dir_all(&factory_dir).unwrap();
    std::fs::write(factory_dir.join("shard-config.toml"), SHARD_CONFIG).unwrap();
    let canonical = canonical_bc_index(cwd);
    std::fs::create_dir_all(canonical.parent().unwrap()).unwrap();
    std::fs::write(&canonical, ORIGINAL_CONTENT).unwrap();
}

/// Materialize exactly what a finishing coordinator leaves behind: `completed.json` and
/// a COMPLETED txn record of this migration. Nothing else is created.
fn inject_finished_coordinator_state(msd: &Path) {
    std::fs::write(
        msd.join("completed.json"),
        r#"{"generation_id":"gen-done","txn_id":"txn-done","completed_at":"2026-10-08T00:00:00Z","canonical_paths_count":4}"#,
    )
    .unwrap();
    std::fs::write(
        msd.join("txn-00000000-0000-4000-8000-000000000001.json"),
        r#"{"state":"COMPLETED","migration_id":"migrate-bc-index","txn_id":"txn-done"}"#,
    )
    .unwrap();
}

/// Byte-level snapshot of every file under `root` (relative path -> bytes), plus
/// directories (as empty-bytes entries with a trailing `/`), so a stray created
/// directory is detected too.
fn tree_snapshot(root: &Path) -> TreeSnapshot {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            let rel = path.strip_prefix(root).unwrap().display().to_string();
            if path.is_dir() {
                out.insert(format!("{rel}/"), Vec::new());
                walk(root, &path, out);
            } else {
                out.insert(rel, std::fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

/// A non-blocking exclusive flock attempt on a FRESH open file description of
/// `exclusive.lock` (flock conflicts between distinct open file descriptions, even
/// within one process). Returns `true` if the lock is held by someone else.
fn lock_is_held_elsewhere(lock_path: &Path) -> bool {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(lock_path)
        .expect("exclusive.lock must exist at both fail points");
    match file.try_lock() {
        Ok(()) => {
            // We took it: release immediately so the run under test is unaffected.
            drop(file);
            false
        }
        Err(std::fs::TryLockError::WouldBlock) => true,
        Err(std::fs::TryLockError::Error(e)) => panic!("unexpected flock error: {e}"),
    }
}

/// BC-1.18.011 EC-052 / ADR-052 v1.23 item 11(c): `completed.json` + a COMPLETED txn
/// materialize at the `before_lock` seam (after the point a pre-lock probe would have
/// run, before the lock). The late process reads under the lock, starts NO fresh run,
/// exits `ALREADY_MIGRATED` (exit 0 -- a state read under the lock; clean steady state
/// => zero writes), and touches nothing beyond the injected files.
#[test]
fn test_BC_1_18_011_toctou_completed_json_materializing_before_lock_is_honored_under_lock() {
    let _fp = fail_point_scope();
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    let msd = migration_state_dir(dir.path());
    let lock_path = msd.join("exclusive.lock");

    // (snapshot taken inside the callback, i.e. AFTER injection) and the flock
    // observation at that moment.
    let at_injection: Arc<Mutex<Option<(TreeSnapshot, bool)>>> = Arc::new(Mutex::new(None));
    {
        let at_injection = Arc::clone(&at_injection);
        let msd = msd.clone();
        let lock_path = lock_path.clone();
        let root = dir.path().to_path_buf();
        fail::cfg_callback(BEFORE_LOCK, move || {
            let held = lock_is_held_elsewhere(&lock_path);
            inject_finished_coordinator_state(&msd);
            *at_injection.lock().unwrap() = Some((tree_snapshot(&root), held));
        })
        .unwrap();
    }

    let outcome = run_bc_index_migration(dir.path());

    let (snapshot_after_injection, lock_held_at_before_lock) = at_injection
        .lock()
        .unwrap()
        .take()
        .expect("the before_lock fail point must fire exactly on the production path");
    assert!(
        !lock_held_at_before_lock,
        "contrast check: at before_lock the flock must NOT yet be held (the seam sits \
         strictly before the lock attempt)"
    );
    assert!(
        matches!(outcome, Ok(BcIndexMigrationOutcome::AlreadyMigrated)),
        "EC-052: completed.json read under the lock must yield ALREADY_MIGRATED, not a \
         fresh run -- got {outcome:?}"
    );
    assert_eq!(
        tree_snapshot(dir.path()),
        snapshot_after_injection,
        "EC-052: no fresh run -- gate untouched (gate-state.json not created), no new txn, \
         no gen-*/ directory, no shards, BC-INDEX.md unchanged: the tree must be \
         byte-identical to the moment right after the injected files appeared"
    );
    assert!(
        !msd.join("gate-state.json").exists(),
        "clean steady state (absent gate reads OPEN) performs zero gate writes"
    );
    assert_eq!(
        std::fs::read_to_string(canonical_bc_index(dir.path())).unwrap(),
        ORIGINAL_CONTENT,
        "the canonical BC-INDEX.md must not be re-read-and-split by a late process"
    );
}

/// ADR-052 v1.23 item 11(c) / EC-052 (lock-then-read): at the
/// `after_lock_before_state_read` seam (a) the flock IS held (a non-blocking flock from
/// another fd gets EWOULDBLOCK) and (b) no state file has been read yet -- proven by
/// mutating state in the callback (completed.json + COMPLETED txn appear only now) and
/// showing the run observes the mutated state (ALREADY_MIGRATED, no fresh run). A state
/// read made before the lock would have seen the pre-mutation tree and started a fresh
/// migration.
#[test]
fn test_BC_1_18_011_toctou_flock_held_and_no_state_read_before_after_lock_seam() {
    let _fp = fail_point_scope();
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    let msd = migration_state_dir(dir.path());
    let lock_path = msd.join("exclusive.lock");

    let observed: Arc<Mutex<Option<(bool, TreeSnapshot)>>> = Arc::new(Mutex::new(None));
    {
        let observed = Arc::clone(&observed);
        let msd = msd.clone();
        let lock_path = lock_path.clone();
        let root = dir.path().to_path_buf();
        fail::cfg_callback(AFTER_LOCK, move || {
            let held = lock_is_held_elsewhere(&lock_path);
            inject_finished_coordinator_state(&msd);
            *observed.lock().unwrap() = Some((held, tree_snapshot(&root)));
        })
        .unwrap();
    }

    let outcome = run_bc_index_migration(dir.path());

    let (held, snapshot_after_injection) = observed
        .lock()
        .unwrap()
        .take()
        .expect("the after_lock_before_state_read fail point must fire");
    assert!(
        held,
        "the flock on exclusive.lock must be HELD (EWOULDBLOCK for another fd) at the \
         after_lock_before_state_read seam"
    );
    assert!(
        matches!(outcome, Ok(BcIndexMigrationOutcome::AlreadyMigrated)),
        "state injected after the lock was taken must be what the run observes (no state \
         read precedes the lock) -- got {outcome:?}"
    );
    assert_eq!(
        tree_snapshot(dir.path()),
        snapshot_after_injection,
        "observing the injected state means a no-op: nothing beyond the injected files"
    );
}

/// Negative control: the same seam with NO injection performs the real fresh run, so the
/// two tests above are not vacuously "AlreadyMigrated regardless". Also re-proves the
/// lock is held at the seam on the fresh path and released after the run returns.
#[test]
fn test_BC_1_18_011_toctou_control_without_injection_runs_fresh_migration_and_releases_lock() {
    let _fp = fail_point_scope();
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    let msd = migration_state_dir(dir.path());
    let lock_path = msd.join("exclusive.lock");

    let held_at_seam = Arc::new(Mutex::new(None::<bool>));
    {
        let held_at_seam = Arc::clone(&held_at_seam);
        let lock_path = lock_path.clone();
        fail::cfg_callback(AFTER_LOCK, move || {
            *held_at_seam.lock().unwrap() = Some(lock_is_held_elsewhere(&lock_path));
        })
        .unwrap();
    }

    let outcome = run_bc_index_migration(dir.path());

    assert_eq!(*held_at_seam.lock().unwrap(), Some(true));
    assert!(
        matches!(
            outcome,
            Ok(BcIndexMigrationOutcome::Completed {
                canonical_paths_count: 4
            })
        ),
        "no injected state: a genuine fresh migration must complete -- got {outcome:?}"
    );
    assert!(
        !lock_is_held_elsewhere(&lock_path),
        "the flock must be released when the run returns"
    );
}

/// Contention variant of EC-052/EC-051: when another holder owns the lock, the run exits
/// `MigrationLockContention` even though `completed.json` has materialized -- it must
/// not fall back to a pre-lock hint of `completed.json` (never exit 0 without the lock).
#[test]
fn test_BC_1_18_011_toctou_lock_not_acquired_with_completed_json_is_contention_not_already_migrated()
 {
    let _fp = fail_point_scope();
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    let msd = migration_state_dir(dir.path());
    let lock_path = msd.join("exclusive.lock");

    // A competing holder that keeps the flock from before_lock until the run returns.
    let holder: Arc<Mutex<Option<std::fs::File>>> = Arc::new(Mutex::new(None));
    {
        let holder = Arc::clone(&holder);
        let msd = msd.clone();
        let lock_path = lock_path.clone();
        fail::cfg_callback(BEFORE_LOCK, move || {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&lock_path)
                .unwrap();
            file.try_lock().expect("test holder must win the free lock");
            inject_finished_coordinator_state(&msd);
            *holder.lock().unwrap() = Some(file);
        })
        .unwrap();
    }

    let outcome = run_bc_index_migration(dir.path());

    assert!(
        matches!(outcome, Err(BcIndexMigrationError::MigrationLockContention)),
        "item 11(c): lock not acquired => MIGRATION_LOCK_CONTENTION (exit 1) even with \
         completed.json present -- got {outcome:?}"
    );
    assert!(
        !msd.join("gate-state.json").exists(),
        "contention: no read-driven reconciliation, no gate write"
    );
    drop(holder.lock().unwrap().take());
}
