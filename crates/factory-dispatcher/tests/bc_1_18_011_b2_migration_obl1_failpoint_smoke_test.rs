// Test files use .expect()/.unwrap()/.panic!() for failure reporting,
// matching bc_1_18_011_b2_migration_test.rs's own established convention.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! OBL-1 (D-1232-OBL-1) Fs-seam reachability smoke test — proves a
//! `migration_fs::*` failpoint is reachable from REAL execution
//! (`run_bc_index_migration`'s fresh-run path), not merely from
//! `migration_fs`'s own unit tests against a bare `StdFs`.
//!
//! `--features factory-dispatcher/failpoints` only — this whole file is a
//! no-op (empty test binary) under the default feature set, since the
//! `fail_point!` macro compiles to nothing without `failpoints` and this
//! file's single test is itself `#[cfg(feature = "failpoints")]`.
//!
//! # Why this is its OWN test binary, not a test added to
//! # `bc_1_18_011_b2_migration_test.rs`
//!
//! `fail::cfg` sets PROCESS-GLOBAL failpoint state. `bc_1_18_011_b2_
//! migration_test.rs` has 55+ tests that `cargo test` runs concurrently
//! (multithreaded within one binary) by default, and several of them call
//! `run_bc_index_migration` (which reaches `Fs::rename` via
//! `execute_canonical_path_moves`). A `panic`-action failpoint configured
//! from a test in that same binary would race every concurrently-running
//! test that also happens to hit `Fs::rename`, causing flaky, seemingly
//! unrelated test failures elsewhere in the suite (confirmed empirically —
//! adding this exact test to that file made 2 unrelated tests fail
//! non-deterministically). Each `tests/*.rs` file compiles to its own
//! process, so a separate file cannot race that file's tests no matter how
//! `cargo test` schedules threads within either binary.
//!
//! This file duplicates a MINIMAL fixture (a single-subsystem BC-INDEX.md
//! + matching `[[shard]]` config) rather than importing
//! `bc_1_18_011_b2_migration_test.rs`'s own fixture helpers, because
//! integration test files are independent crates — nothing in that file is
//! `pub`, so nothing is importable from here.

// Gates the ENTIRE file, not just the one test function: the fixture
// helpers/consts below are unused (dead code) under the default feature
// set, which `cargo clippy -D warnings` would otherwise flag.
#![cfg(feature = "failpoints")]

use std::path::{Path, PathBuf};

use factory_dispatcher::shard_manager::run_bc_index_migration;

const ORIGINAL_CONTENT: &str = "\
---
document_type: bc-index
version: \"1.0\"
total_bcs: 1
---

## Summary

| Subsystem | BC-S Prefix | Count | Directory |
|-----------|------------|-------|-----------|
| SS-01 Hook Dispatcher Core | BC-1 | 1 | ss-01/ |

## Index by subsystem

### SS-01 — Hook Dispatcher Core (BC-1) — 1 BC

| BC ID | Title | Status | Capability | Stories |
|-------|-------|--------|-----------|---------|
| [BC-1.01.001](ss-01/BC-1.01.001.md) | Registry rejects unknown schema version | draft | CAP-TBD | S-15.01 |
";

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

fn write_shard_config(cwd: &Path, body: &str) {
    let factory_dir = cwd.join(".factory");
    std::fs::create_dir_all(&factory_dir).unwrap();
    std::fs::write(factory_dir.join("shard-config.toml"), body).unwrap();
}

fn bc_index_target(cwd: &Path) -> PathBuf {
    cwd.join(".factory/specs/behavioral-contracts/BC-INDEX.md")
}

#[cfg(feature = "failpoints")]
#[test]
fn test_OBL1_migration_fs_rename_failpoint_reachable_from_run_bc_index_migration() {
    // The single-arg `fail_point!(name)` form `StdFs::rename` uses only
    // supports non-return actions (`panic`/`sleep`/`print`/...) — see the
    // `fail` 0.5.1 crate's own macro definition ("Return is not supported
    // for the fail point"). `panic` is exactly what proves REACHABILITY:
    // if `fs.rename(..)` inside `execute_canonical_path_moves` (called from
    // `run_bc_index_migration`'s fresh-run -> `finish_committing_migration`
    // path) is never actually invoked, this panic never fires and
    // `catch_unwind` observes `Ok(..)` instead of `Err(..)` — the test then
    // FAILS, distinguishing "reachable" from "wired but dead". Safe to
    // leave the failpoint configured process-wide for this call (never
    // reset) — this file's single test is the ONLY test in this process.
    fail::cfg("migration_fs::rename", "panic").expect("configuring the failpoint must succeed");

    // Suppress the default panic-hook's stderr backtrace dump for this
    // EXPECTED, caught panic.
    std::panic::set_hook(Box::new(|_| {}));

    let dir = tempfile::tempdir().unwrap();
    write_shard_config(dir.path(), SHARD_CONFIG);
    let canonical_path = bc_index_target(dir.path());
    std::fs::create_dir_all(canonical_path.parent().unwrap()).unwrap();
    std::fs::write(&canonical_path, ORIGINAL_CONTENT).unwrap();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_bc_index_migration(dir.path())
    }));

    let panic_payload = result.expect_err(
        "run_bc_index_migration must panic via the migration_fs::rename failpoint when it is \
         configured to panic -- an Ok(..)/Err(..) return here means execute_canonical_path_moves \
         never actually called Fs::rename, i.e. the seam is wired but NOT reachable from real \
         execution",
    );
    let message = panic_payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| panic_payload.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_default();
    assert!(
        message.contains("migration_fs::rename"),
        "the panic must be attributable to the migration_fs::rename failpoint specifically \
         (proving THAT call site fired, not some unrelated panic); got: {message:?}"
    );
}
