// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-25.10 RED tests -- WRITER DURABILITY observed through the `Fs` seam (red test T12).
//!
//! Authority: ADR-054 v1.0 Decision 1.9 steps 2-6 and "`Fs` seam additions"
//! (`append_durable(path, bytes) -> created: bool`, `truncate_durable(path, len)`), Decision
//! 5 (fault between every step, including tail truncation and log-creation directory sync);
//! ADR-052 v1.24 section Decision 7d (platform barriers: `F_FULLFSYNC` on macOS, `fsync`
//! elsewhere, no silent downgrade); BC-1.18.011 v1.21 Postcondition 15(c), EC-065; story
//! S-25.10 AC-008.
//!
//! | Story AC / red test | Test |
//! |---|---|
//! | AC-008 / T12 | `..._15c_append_durable_syncs_the_directory_when_it_creates_the_log` |
//! | AC-008 / T12 | `..._15c_a_batch_is_one_append_durable_call_and_a_non_creating_call_does_no_directory_sync` |
//! | AC-008 / T12 | `..._15c_std_fs_append_durable_reports_created_and_appends` |
//! | AC-008 / T12 | `..._15c_std_fs_truncate_durable_shortens_the_file_to_the_requested_length` |
//! | AC-008 / T2 | `..._EC065_tail_truncation_goes_through_truncate_durable_before_the_first_append` |
//! | AC-008 / T12 | `..._15c_macos_uses_the_full_sync_primitive_without_downgrade` (`cfg(target_os = "macos")`) |
//! | AC-008 / T12 | `..._15c_failing_barrier_is_an_io_error_and_the_boundary_is_not_reached` |
//! | AC-008 / T12 | `..._15c_failing_directory_sync_after_creation_is_an_io_error` |
//! | AC-008 / T12 | `..._15c_failing_tail_truncation_is_an_io_error_and_nothing_is_appended` |
//! | AC-008 / T12 (`failpoints`) | `..._15c_failpoint_*` (the real `StdFs` seams are injectable per step) |
//!
//! Interpretation recorded for the implementer (ADR-054 1.9 step 5): `append_durable` returns
//! `created: bool` precisely so that the WRITER can sync the parent directory when this call
//! created the log, via the existing `Fs::fsync_dir` seam (a separate, separately injectable
//! step, as Decision 5 requires). The recording double below therefore does NOT sync the
//! directory inside `append_durable`; `StdFs::append_durable` itself only appends, barriers
//! and reports `created`. The double, like `StdFs`, performs the REAL filesystem effect so
//! on-disk assertions are meaningful.
//!
//! The module under test and the new `Fs` methods do not exist yet: this file fails to
//! COMPILE (the accepted Red).

#[path = "s2510_support/api.rs"]
mod api;
#[path = "s2510_support/mod.rs"]
mod support;

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};

use api::{IntentLogWriter, to_intent};
use factory_dispatcher::shard_manager::BcIndexMigrationError;
use factory_dispatcher::shard_manager::migration_fs::{Fs, StdFs};
use support::{Rec, TXN, assert_no_failures, concat, encode};

// ---------------------------------------------------------------------------
// Recording / fault-injecting double over the REAL filesystem
// ---------------------------------------------------------------------------

#[derive(Default)]
struct RecFs {
    ops: RefCell<Vec<String>>,
    /// `append_durable` writes, then fails (the barrier failed): bytes may be in the page
    /// cache but the call reports `Err`.
    fail_barrier: Cell<bool>,
    fail_dir_sync: Cell<bool>,
    fail_truncate: Cell<bool>,
}

fn io_err(path: &Path, what: &str) -> BcIndexMigrationError {
    BcIndexMigrationError::Io {
        path: path.to_path_buf(),
        source: std::io::Error::other(format!("injected: {what}")),
    }
}

impl RecFs {
    fn mutating_ops(&self) -> Vec<String> {
        self.ops.borrow().clone()
    }
    fn clear(&self) {
        self.ops.borrow_mut().clear();
    }
}

impl Fs for RecFs {
    fn write_temp(&self, path: &Path, content: &[u8]) -> Result<(), BcIndexMigrationError> {
        StdFs.write_temp(path, content)
    }
    fn fsync_file(&self, path: &Path) -> Result<(), BcIndexMigrationError> {
        StdFs.fsync_file(path)
    }
    fn rename(&self, from: &Path, to: &Path) -> Result<(), BcIndexMigrationError> {
        StdFs.rename(from, to)
    }
    fn fsync_dir(&self, dir: &Path) -> Result<(), BcIndexMigrationError> {
        self.ops
            .borrow_mut()
            .push(format!("fsync_dir {}", dir.display()));
        if self.fail_dir_sync.get() {
            return Err(io_err(dir, "directory sync"));
        }
        StdFs.fsync_dir(dir)
    }
    fn read(&self, path: &Path) -> Result<Option<Vec<u8>>, BcIndexMigrationError> {
        StdFs.read(path)
    }
    fn exists(&self, path: &Path) -> bool {
        StdFs.exists(path)
    }
    fn pointer_swap(&self, tmp: &Path, target: &Path) -> Result<(), BcIndexMigrationError> {
        StdFs.pointer_swap(tmp, target)
    }
    fn remove(&self, path: &Path) -> Result<(), BcIndexMigrationError> {
        StdFs.remove(path)
    }
    fn append_durable(&self, path: &Path, content: &[u8]) -> Result<bool, BcIndexMigrationError> {
        self.ops.borrow_mut().push(format!(
            "append_durable {} {}",
            path.display(),
            content.len()
        ));
        let created = !path.exists();
        {
            use std::io::Write as _;
            let mut f = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|source| BcIndexMigrationError::Io {
                    path: path.to_path_buf(),
                    source,
                })?;
            f.write_all(content)
                .map_err(|source| BcIndexMigrationError::Io {
                    path: path.to_path_buf(),
                    source,
                })?;
        }
        if self.fail_barrier.get() {
            return Err(io_err(path, "file barrier"));
        }
        Ok(created)
    }
    fn truncate_durable(&self, path: &Path, len: u64) -> Result<(), BcIndexMigrationError> {
        self.ops
            .borrow_mut()
            .push(format!("truncate_durable {} {len}", path.display()));
        if self.fail_truncate.get() {
            return Err(io_err(path, "tail truncation"));
        }
        let f = std::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .map_err(|source| BcIndexMigrationError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        f.set_len(len).map_err(|source| BcIndexMigrationError::Io {
            path: path.to_path_buf(),
            source,
        })
    }
}

fn setup() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("intent-gen-1.log");
    (dir, log)
}

fn r(i: usize) -> Rec {
    Rec::sample(TXN, i, "INTENT", 1)
}

// ===========================================================================
// creation: directory sync
// ===========================================================================

/// ADR-054 1.9 step 5: when the append CREATED the log, the parent directory is synced
/// BEFORE the WAL boundary counts as reached. Observed through the seam: the batch is ONE
/// `append_durable` call carrying every byte, followed by `fsync_dir(<parent>)`.
#[test]
fn test_BC_1_18_011_15c_append_durable_syncs_the_directory_when_it_creates_the_log() {
    let (dir, log) = setup();
    let fs = RecFs::default();
    let mut w = IntentLogWriter::open(&fs, &log, TXN).expect("open on an absent log");
    assert!(
        fs.mutating_ops().is_empty(),
        "open on an absent log mutates nothing: {:?}",
        fs.mutating_ops()
    );
    w.append_batch(&[to_intent(&r(0)), to_intent(&r(1))])
        .expect("first batch");

    let total = encode(&r(0)).len() + encode(&r(1)).len();
    assert_eq!(
        fs.mutating_ops(),
        vec![
            format!("append_durable {} {total}", log.display()),
            format!("fsync_dir {}", dir.path().display()),
        ],
        "the creating batch must be ONE append_durable of every byte, THEN a sync of the parent directory"
    );
}

/// One `write_all` per batch (ONE `append_durable` call however many records), and a call
/// that did NOT create the log performs NO directory sync.
#[test]
fn test_BC_1_18_011_15c_a_batch_is_one_append_durable_call_and_a_non_creating_call_does_no_directory_sync()
 {
    let (_dir, log) = setup();
    let fs = RecFs::default();
    {
        let mut w = IntentLogWriter::open(&fs, &log, TXN).unwrap();
        w.append_batch(&[to_intent(&r(0))]).unwrap();
    }
    fs.clear();
    let mut w = IntentLogWriter::open(&fs, &log, TXN).expect("reopen a clean log");
    assert!(
        fs.mutating_ops().is_empty(),
        "reopening a clean log must not truncate: {:?}",
        fs.mutating_ops()
    );
    let batch = [to_intent(&r(1)), to_intent(&r(2)), to_intent(&r(3))];
    w.append_batch(&batch).unwrap();
    let total: usize = (1..=3).map(|i| encode(&r(i)).len()).sum();
    assert_eq!(
        fs.mutating_ops(),
        vec![format!("append_durable {} {total}", log.display())],
        "a three-record batch is ONE append_durable call and, the log already existing, NO directory sync"
    );
    assert_eq!(
        std::fs::read(&log).unwrap(),
        concat(&(0..=3).map(|i| encode(&r(i))).collect::<Vec<_>>())
    );
}

/// `StdFs::append_durable` reports whether THIS call created the file, appends to it, and
/// never truncates existing content.
#[test]
fn test_BC_1_18_011_15c_std_fs_append_durable_reports_created_and_appends() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("intent.log");
    let first = StdFs
        .append_durable(&path, b"first-")
        .expect("creating append");
    let second = StdFs
        .append_durable(&path, b"second")
        .expect("second append");
    assert!(
        first,
        "the call that creates the file returns created == true"
    );
    assert!(
        !second,
        "a call on an existing file returns created == false"
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"first-second");
}

/// `StdFs::truncate_durable` shortens the file to exactly `len` bytes.
#[test]
fn test_BC_1_18_011_15c_std_fs_truncate_durable_shortens_the_file_to_the_requested_length() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("intent.log");
    std::fs::write(&path, b"0123456789").unwrap();
    StdFs.truncate_durable(&path, 4).expect("truncate");
    assert_eq!(std::fs::read(&path).unwrap(), b"0123");
    StdFs.truncate_durable(&path, 0).expect("truncate to empty");
    assert_eq!(std::fs::read(&path).unwrap(), b"");
}

// ===========================================================================
// tail repair
// ===========================================================================

/// ADR-054 1.9 step 2: a torn tail is truncated to `valid_prefix_len` THROUGH the seam
/// (`truncate_durable`) when the writer is OPENED -- before the first append -- and the
/// append that follows needs no directory sync (the log already existed).
#[test]
fn test_BC_1_18_011_EC065_tail_truncation_goes_through_truncate_durable_before_the_first_append() {
    let (_dir, log) = setup();
    let enc1 = encode(&r(0));
    let mut original = enc1.clone();
    original.extend_from_slice(&[0u8; 512]);
    std::fs::write(&log, &original).unwrap();

    let fs = RecFs::default();
    let mut w = IntentLogWriter::open(&fs, &log, TXN).expect("open + repair");
    assert_eq!(
        fs.mutating_ops(),
        vec![format!("truncate_durable {} {}", log.display(), enc1.len())],
        "open must truncate the torn tail to valid_prefix_len BEFORE any append"
    );
    assert_eq!(
        std::fs::read(&log).unwrap(),
        enc1,
        "the tail is gone after open"
    );

    w.append_batch(&[to_intent(&r(2))]).unwrap();
    assert_eq!(
        fs.mutating_ops().last().unwrap(),
        &format!("append_durable {} {}", log.display(), encode(&r(2)).len())
    );
    assert_eq!(
        fs.mutating_ops().len(),
        2,
        "truncate then one append, nothing else: {:?}",
        fs.mutating_ops()
    );
    assert_eq!(std::fs::read(&log).unwrap(), concat(&[enc1, encode(&r(2))]));
}

// ===========================================================================
// macOS strict barrier
// ===========================================================================

/// Body of `fn <name>(` inside `migration_fs.rs` (brace-matched), or panic.
#[cfg(target_os = "macos")]
fn fn_body(src: &str, name: &str) -> String {
    let needle = format!("fn {name}(");
    let start = src
        .find(&needle)
        .unwrap_or_else(|| panic!("migration_fs.rs must define `{needle}`"));
    let open = start + src[start..].find('{').expect("fn body");
    let mut depth = 0usize;
    for (i, c) in src[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return src[open..open + i + 1].to_string();
                }
            }
            _ => {}
        }
    }
    panic!("unbalanced braces after `{needle}`");
}

/// ADR-052 Decision 7d / ADR-054 1.9 step 5 (macOS): `append_durable` and `truncate_durable`
/// use the strict `F_FULLFSYNC` barrier -- the existing `*_strict_durable` primitive family
/// (`sync_dir_strict_durable`, `write_atomic_strict_durable` of `last_amended_migrate`) --
/// and NEVER a bare `sync_all` / `sync_data` (which on Apple platforms does not flush the
/// drive cache): NO silent downgrade. The barrier itself is not observable from a test
/// process, so this is a source gate over the two `StdFs` bodies plus a live call.
#[cfg(target_os = "macos")]
#[test]
fn test_BC_1_18_011_15c_macos_uses_the_full_sync_primitive_without_downgrade() {
    let src = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shard_manager/migration_fs.rs"),
    )
    .unwrap();
    // Only the production `impl Fs for StdFs` (before the unit-test module).
    let prod = src.split("#[cfg(test)]").next().unwrap();
    let prod = &prod[prod
        .find("impl Fs for StdFs")
        .expect("migration_fs.rs must keep `impl Fs for StdFs`")..];
    let mut failures = Vec::new();
    for name in ["append_durable", "truncate_durable"] {
        let body = fn_body(prod, name);
        for bad in [".sync_all(", ".sync_data(", "sync_all()", "sync_data()"] {
            if body.contains(bad) {
                failures.push(format!(
                    "`StdFs::{name}` must not call `{bad}` directly (macOS: not durable to media); \
                     use the shared strict barrier primitive"
                ));
            }
        }
        if !(body.contains("strict_durable") || body.contains("F_FULLFSYNC")) {
            failures.push(format!(
                "`StdFs::{name}` must use the strict full-sync barrier (a `*_strict_durable` \
                 primitive or F_FULLFSYNC); body was:\n{body}"
            ));
        }
        if body.contains("let _ =") || body.contains(".ok();") {
            failures.push(format!(
                "`StdFs::{name}` must propagate every barrier error, not discard it"
            ));
        }
    }
    // Live: the strict path works on the host filesystem and appends durably.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("intent.log");
    match StdFs.append_durable(&path, b"abc") {
        Ok(true) => {}
        other => failures.push(format!(
            "live append_durable on APFS: expected Ok(true), got {other:?}"
        )),
    }
    assert_no_failures("macOS strict barrier", failures);
}

// ===========================================================================
// failing steps
// ===========================================================================

/// ADR-054 1.9 step 5: a failing file barrier is an `Io` error and the WAL boundary is NOT
/// reached -- the writer reports the error, makes exactly ONE append attempt (no retry, no
/// downgrade to a weaker barrier) and does NOT proceed to the directory sync.
#[test]
fn test_BC_1_18_011_15c_failing_barrier_is_an_io_error_and_the_boundary_is_not_reached() {
    let (_dir, log) = setup();
    let fs = RecFs::default();
    fs.fail_barrier.set(true);
    let mut w = IntentLogWriter::open(&fs, &log, TXN).unwrap();
    let err = w
        .append_batch(&[to_intent(&r(0))])
        .expect_err("a failing barrier must fail the batch");
    assert!(
        matches!(err, BcIndexMigrationError::Io { .. }),
        "a failing barrier is an Io error, got {err}"
    );
    assert_eq!(err.process_exit_code(), 2);
    assert_eq!(
        fs.mutating_ops(),
        vec![format!(
            "append_durable {} {}",
            log.display(),
            encode(&r(0)).len()
        )],
        "exactly one append attempt and no directory sync after a failed barrier"
    );
}

/// ADR-054 1.9 step 5: if the call created the log and the parent-directory sync fails,
/// the result is an `Io` error (the boundary is not reached) -- never swallowed.
#[test]
fn test_BC_1_18_011_15c_failing_directory_sync_after_creation_is_an_io_error() {
    let (dir, log) = setup();
    let fs = RecFs::default();
    fs.fail_dir_sync.set(true);
    let mut w = IntentLogWriter::open(&fs, &log, TXN).unwrap();
    let err = w
        .append_batch(&[to_intent(&r(0))])
        .expect_err("a failing directory sync after creation must fail the batch");
    assert!(matches!(err, BcIndexMigrationError::Io { .. }), "got {err}");
    assert_eq!(
        fs.mutating_ops(),
        vec![
            format!("append_durable {} {}", log.display(), encode(&r(0)).len()),
            format!("fsync_dir {}", dir.path().display()),
        ]
    );
}

/// ADR-054 1.9 step 2 + Decision 5: a failing tail truncation is an `Io` error from
/// `open`; the log is unchanged and nothing is appended (never append behind garbage).
#[test]
fn test_BC_1_18_011_15c_failing_tail_truncation_is_an_io_error_and_nothing_is_appended() {
    let (_dir, log) = setup();
    let mut original = encode(&r(0));
    original.extend_from_slice(b"garbage-tail");
    std::fs::write(&log, &original).unwrap();
    let fs = RecFs::default();
    fs.fail_truncate.set(true);
    let res = IntentLogWriter::open(&fs, &log, TXN);
    match res {
        Err(BcIndexMigrationError::Io { .. }) => {}
        Err(e) => panic!("a failing truncation must be an Io error, got {e}"),
        Ok(_) => panic!("open must not succeed when the torn tail cannot be removed"),
    }
    assert_eq!(
        std::fs::read(&log).unwrap(),
        original,
        "the log must be unchanged"
    );
    assert!(
        fs.mutating_ops()
            .iter()
            .all(|o| o.starts_with("truncate_durable")),
        "no append may follow a failed truncation: {:?}",
        fs.mutating_ops()
    );
}

// ===========================================================================
// real StdFs seams are injectable per step (failpoints builds only)
// ===========================================================================

#[cfg(feature = "failpoints")]
mod failpoints {
    use super::*;

    fn fail_point_scope() -> fail::FailScenario<'static> {
        fail::FailScenario::setup()
    }

    /// OBL-1: `migration_fs::append_durable` is its own injectable boundary. A graceful
    /// `return(storage_full)` surfaces as an `Io` of that kind from the writer, the batch
    /// is not reported durable, and no byte reached the log.
    #[test]
    fn test_BC_1_18_011_15c_failpoint_append_durable_return_is_an_io_error_and_appends_nothing() {
        let _fp = fail_point_scope();
        let (_dir, log) = setup();
        let mut w = IntentLogWriter::open(&StdFs, &log, TXN).unwrap();
        fail::cfg("migration_fs::append_durable", "return(storage_full)").unwrap();
        let res = w.append_batch(&[to_intent(&r(0))]);
        fail::cfg("migration_fs::append_durable", "off").unwrap();
        match res {
            Err(BcIndexMigrationError::Io { source, .. }) => {
                assert_eq!(source.kind(), std::io::ErrorKind::StorageFull);
            }
            other => panic!("expected an injected StorageFull Io error, got {other:?}"),
        }
        assert!(
            !log.exists() || std::fs::metadata(&log).unwrap().len() == 0,
            "the failed append must leave no bytes"
        );
    }

    /// `migration_fs::truncate_durable` is its own injectable boundary: a failing tail
    /// truncation fails `open`, leaves the log untouched.
    #[test]
    fn test_BC_1_18_011_15c_failpoint_truncate_durable_return_fails_open_and_leaves_the_log_untouched()
     {
        let _fp = fail_point_scope();
        let (_dir, log) = setup();
        let mut original = encode(&r(0));
        original.extend_from_slice(&[0u8; 64]);
        std::fs::write(&log, &original).unwrap();
        fail::cfg("migration_fs::truncate_durable", "return(write_zero)").unwrap();
        let res = IntentLogWriter::open(&StdFs, &log, TXN);
        fail::cfg("migration_fs::truncate_durable", "off").unwrap();
        match res {
            Err(BcIndexMigrationError::Io { source, .. }) => {
                assert_eq!(source.kind(), std::io::ErrorKind::WriteZero);
            }
            Err(e) => panic!("expected an injected WriteZero Io error, got {e}"),
            Ok(_) => panic!("open must fail when the truncation fails"),
        }
        assert_eq!(std::fs::read(&log).unwrap(), original);
    }

    /// The log-creation directory sync is its own injectable boundary
    /// (`migration_fs::fsync_dir`): failing it after the creating append is an `Io` error.
    #[test]
    fn test_BC_1_18_011_15c_failpoint_log_creation_directory_sync_return_is_an_io_error() {
        let _fp = fail_point_scope();
        let (_dir, log) = setup();
        let mut w = IntentLogWriter::open(&StdFs, &log, TXN).unwrap();
        fail::cfg("migration_fs::fsync_dir", "return(permission_denied)").unwrap();
        let res = w.append_batch(&[to_intent(&r(0))]);
        fail::cfg("migration_fs::fsync_dir", "off").unwrap();
        match res {
            Err(BcIndexMigrationError::Io { source, .. }) => {
                assert_eq!(source.kind(), std::io::ErrorKind::PermissionDenied);
            }
            other => panic!("expected an injected PermissionDenied Io error, got {other:?}"),
        }
    }
}
