//! OBL-1 (D-1232-OBL-1) filesystem seam for the B2 migration's crash-
//! atomicity logic (BC-1.18.011; ADR-052 §Decision 7a/7b/7c).
//!
//! Kani cannot model real I/O, and the fault-injection integration suite
//! (test-writer's scope) needs a way to crash the process at a specific
//! point inside the rename+intent+swap sequence. Both need the same seam:
//! an [`Fs`] trait the migration's decision logic calls through, with
//! [`StdFs`] (real `std::fs` + the existing D-1232-OBL-2(a) durable-write
//! primitives) as the production implementation.
//!
//! # Trait surface — deliberately scoped to exactly the ops the migration
//! uses (research report §1c "keep abstract domains tiny"): no
//! open-with-fd-handle API, no generic directory-listing op.
//!
//! # Production-granularity note (implementer scope-boundary, honestly
//! documented rather than silently papered over)
//!
//! [`StdFs::rename`] and [`StdFs::fsync_dir`] map 1:1 onto the two already-
//! separate real primitive calls `execute_canonical_path_moves` performs
//! today (`std::fs::rename` then `sync_dir_durable`) — genuinely two
//! distinct, independently fault-injectable steps in production.
//!
//! [`StdFs::write_temp`] and [`StdFs::fsync_file`], by contrast, do NOT map
//! onto two separate real primitive calls for the migration's STAGING-
//! publish writes: `last_amended_migrate::atomic_write::
//! write_atomic_strict_durable` (which [`super::migration_durable_write`]
//! delegates to) performs write+`F_FULLFSYNC`+rename-into-place+
//! `F_FULLFSYNC`(dir) as ONE bundled FFI call, with no smaller primitive
//! `last_amended_migrate` currently exposes. `StdFs::write_temp` therefore
//! performs the FULL bundled durable write (content is durably staged by
//! the time `write_temp` returns); `StdFs::fsync_file` is a redundant,
//! harmless best-effort re-fsync of the same already-durable file — NOT a
//! silent weakening of the D-1232-OBL-2(a) STRICT durability mandate (the
//! `F_FULLFSYNC`-strength guarantee still unconditionally happens, just
//! inside `write_temp` rather than split across two calls). This means
//! production fault injection cannot land a crash strictly BETWEEN
//! `write_temp` and `fsync_file` for a staging-publish target the way it
//! can for `rename`/`fsync_dir` — a real limitation, not a design defect;
//! Kani's abstract two-namespace model (formal-verifier's harness) is
//! unaffected, since it models `write_temp`/`fsync_file` as genuinely
//! separate steps regardless of production's bundling (a conservative
//! superset of production's actual interleaving space, never unsound).
//! Splitting the underlying `last_amended_migrate` primitive into a true
//! two-step write-then-fsync API, so production fault injection can reach
//! this boundary too, is a legitimate follow-up but requires a
//! cross-crate API change to `last-amended-migrate` and is out of THIS
//! burst's scope — flagged here rather than silently narrowed.
//!
//! # Call-graph wiring status
//!
//! This module defines the trait and its production adapter as a
//! compiling, independently unit-testable seam. Threading `&impl Fs`
//! through the full existing call graph (`execute_canonical_path_moves`,
//! `commit_current_generation_pointer`, `stage_new_generation`,
//! `append_intent_log_record`, `read_intent_log`, `read_active_txn_record`,
//! `write_txn_record`, `run_bc_index_migration`'s own body) is NOT done in
//! this burst — those functions still call `std::fs`/
//! `migration_durable_write` directly, unchanged, zero behavioral
//! difference from before this module existed. `recover()`
//! ([`super::recover`]) is built as a pure function over already-gathered
//! data (no `Fs` parameter) rather than requiring the full thread-through,
//! so the fail-open structural fix (finding #1) does not depend on this
//! wiring gap. See the OBL-1 discharge report for the explicit remaining-
//! work boundary.

use std::path::Path;

use super::BcIndexMigrationError;

/// Filesystem seam for the B2 migration's crash-atomicity logic. See the
/// module doc comment for the production-granularity note on
/// `write_temp`/`fsync_file`.
pub trait Fs {
    /// Write `content` durably to `path` (production: the full
    /// `F_FULLFSYNC(temp) -> rename -> F_FULLFSYNC(dir)` sequence via
    /// [`super::migration_durable_write`] — see the module doc comment's
    /// production-granularity note). Model (Kani, formal-verifier's
    /// harness): writes to the abstract `live` namespace only, NOT yet
    /// `durable` until [`Fs::fsync_file`].
    fn write_temp(&self, path: &Path, content: &[u8]) -> Result<(), BcIndexMigrationError>;

    /// Durability barrier for file CONTENT. Production: see the module
    /// doc comment's production-granularity note (a harmless redundant
    /// re-fsync for a staging-publish write, since `write_temp` already
    /// performed the full durable write in production). Model: promotes
    /// `path`'s content from `live` to `durable`.
    fn fsync_file(&self, path: &Path) -> Result<(), BcIndexMigrationError>;

    /// Atomic rename, same filesystem (staging and canonical targets are
    /// both under `.factory/`). Production: `std::fs::rename`. Model:
    /// atomic namespace move within `live`; NOT durable until the parent
    /// dir is fsynced.
    fn rename(&self, from: &Path, to: &Path) -> Result<(), BcIndexMigrationError>;

    /// Durability barrier for a DIRECTORY's entries. Production:
    /// [`super::sync_dir_durable`] (`F_FULLFSYNC`-strength, STRICT, no
    /// silent fallback — D-1232-OBL-2(a)). Model: promotes namespace/
    /// existence changes from `live` to `durable`.
    fn fsync_dir(&self, dir: &Path) -> Result<(), BcIndexMigrationError>;

    /// Read full file content. Production: `std::fs::read`. `Ok(None)`
    /// represents a clean not-found (never conflated with a genuine I/O
    /// error). Model: reads from `live`.
    fn read(&self, path: &Path) -> Result<Option<Vec<u8>>, BcIndexMigrationError>;

    /// Existence check without reading content.
    fn exists(&self, path: &Path) -> bool;

    /// Atomic pointer swap — the SOLE commit-point for the whole
    /// multi-file migration (ADR-052 §Decision 7c step 6). Kept distinct
    /// from [`Fs::rename`] so a Kani harness can assert the commit
    /// predicate fires on exactly this call, never on a canonical-path
    /// move rename. Production: identical `rename(2)` syscall to
    /// [`Fs::rename`] — the distinction is in the CALLER's protocol role,
    /// not the syscall.
    fn pointer_swap(&self, tmp: &Path, target: &Path) -> Result<(), BcIndexMigrationError>;

    /// Remove a file or empty/non-empty directory tree (staging-generation
    /// cleanup on abort/quarantine). Best-effort at the call site — a
    /// failure here must never be treated as migration-correctness
    /// failure (ADR-052 §7c step 9: cleanup is optional housekeeping).
    fn remove(&self, path: &Path) -> Result<(), BcIndexMigrationError>;
}

// ---------------------------------------------------------------------------
// Fault-injection facade (OBL-1 §6.1 item 6 / §6.3). Compiles to nothing
// without the `failpoints` cargo feature — `fail_point!` never runs, and
// `fail` is not even a compiled dependency (`dep:fail` is optional). Named
// per the boundary list in the OBL-1 design's §6.3: one failpoint per `Fs`
// mutating operation, so the test-writer's crash-injection suite can target
// each by name via `fail::cfg("migration_fs::<op>", "abort")` /
// `FailScenario`.
// ---------------------------------------------------------------------------

#[cfg(feature = "failpoints")]
macro_rules! migration_failpoint {
    ($name:expr) => {
        fail::fail_point!($name)
    };
}

#[cfg(not(feature = "failpoints"))]
macro_rules! migration_failpoint {
    ($name:expr) => {};
}

/// Production implementation of [`Fs`] — delegates to the EXISTING
/// `migration_durable_write`/`sync_dir_durable`/`std::fs::rename`/
/// `std::fs::read` primitives already in `shard_manager.rs`. Pure
/// extraction: no behavioral change to production I/O beyond this module's
/// own doc-commented `write_temp`/`fsync_file` granularity note.
#[derive(Debug, Clone, Copy, Default)]
pub struct StdFs;

impl Fs for StdFs {
    fn write_temp(&self, path: &Path, content: &[u8]) -> Result<(), BcIndexMigrationError> {
        migration_failpoint!("migration_fs::write_temp");
        super::migration_durable_write(path, content)
    }

    fn fsync_file(&self, path: &Path) -> Result<(), BcIndexMigrationError> {
        migration_failpoint!("migration_fs::fsync_file");
        // See the module doc comment's production-granularity note: this
        // is a harmless best-effort re-fsync, not the sole durability
        // barrier (that already happened inside `write_temp`).
        match std::fs::File::open(path) {
            Ok(file) => file.sync_all().map_err(|source| BcIndexMigrationError::Io {
                path: path.to_path_buf(),
                source,
            }),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(BcIndexMigrationError::Io {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), BcIndexMigrationError> {
        migration_failpoint!("migration_fs::rename");
        std::fs::rename(from, to).map_err(|source| BcIndexMigrationError::Io {
            path: to.to_path_buf(),
            source,
        })
    }

    fn fsync_dir(&self, dir: &Path) -> Result<(), BcIndexMigrationError> {
        migration_failpoint!("migration_fs::fsync_dir");
        super::sync_dir_durable(dir)
    }

    fn read(&self, path: &Path) -> Result<Option<Vec<u8>>, BcIndexMigrationError> {
        match std::fs::read(path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(BcIndexMigrationError::Io {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn pointer_swap(&self, tmp: &Path, target: &Path) -> Result<(), BcIndexMigrationError> {
        migration_failpoint!("migration_fs::pointer_swap");
        std::fs::rename(tmp, target).map_err(|source| BcIndexMigrationError::Io {
            path: target.to_path_buf(),
            source,
        })
    }

    fn remove(&self, path: &Path) -> Result<(), BcIndexMigrationError> {
        migration_failpoint!("migration_fs::remove");
        if path.is_dir() {
            match std::fs::remove_dir_all(path) {
                Ok(()) => Ok(()),
                Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(source) => Err(BcIndexMigrationError::Io {
                    path: path.to_path_buf(),
                    source,
                }),
            }
        } else {
            match std::fs::remove_file(path) {
                Ok(()) => Ok(()),
                Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(source) => Err(BcIndexMigrationError::Io {
                    path: path.to_path_buf(),
                    source,
                }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_OBL1_std_fs_write_temp_then_read_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.txt");
        let fs = StdFs;
        fs.write_temp(&path, b"hello").unwrap();
        assert_eq!(fs.read(&path).unwrap(), Some(b"hello".to_vec()));
        assert!(fs.exists(&path));
    }

    #[test]
    fn test_OBL1_std_fs_read_missing_file_returns_ok_none_not_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing.txt");
        let fs = StdFs;
        assert_eq!(fs.read(&path).unwrap(), None);
        assert!(!fs.exists(&path));
    }

    #[test]
    fn test_OBL1_std_fs_rename_moves_file_and_fsync_dir_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let from = dir.path().join("a.txt");
        let to = dir.path().join("b.txt");
        let fs = StdFs;
        fs.write_temp(&from, b"content").unwrap();
        fs.rename(&from, &to).unwrap();
        assert!(!fs.exists(&from));
        assert_eq!(fs.read(&to).unwrap(), Some(b"content".to_vec()));
        fs.fsync_dir(dir.path()).unwrap();
    }

    #[test]
    fn test_OBL1_std_fs_pointer_swap_atomically_replaces_target() {
        let dir = tempfile::tempdir().unwrap();
        let tmp = dir.path().join("CURRENT.tmp.json");
        let target = dir.path().join("CURRENT.json");
        let fs = StdFs;
        fs.write_temp(&target, b"old").unwrap();
        fs.write_temp(&tmp, b"new").unwrap();
        fs.pointer_swap(&tmp, &target).unwrap();
        assert_eq!(fs.read(&target).unwrap(), Some(b"new".to_vec()));
        assert!(!fs.exists(&tmp));
    }

    #[test]
    fn test_OBL1_std_fs_remove_file_and_missing_path_is_a_noop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f.txt");
        let fs = StdFs;
        fs.write_temp(&path, b"x").unwrap();
        fs.remove(&path).unwrap();
        assert!(!fs.exists(&path));
        // Removing an already-gone path is a no-op, not an error.
        fs.remove(&path).unwrap();
    }

    #[test]
    fn test_OBL1_std_fs_remove_directory_tree() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir_all(sub.join("nested")).unwrap();
        std::fs::write(sub.join("nested/f.txt"), b"x").unwrap();
        let fs = StdFs;
        fs.remove(&sub).unwrap();
        assert!(!fs.exists(&sub));
    }
}
