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
//! `&impl Fs` is now threaded through the real migration call graph:
//! `execute_canonical_path_moves`, `commit_current_generation_pointer`,
//! `stage_new_generation`, `append_intent_log_record`, `read_intent_log`,
//! `read_active_txn_record`, `write_txn_record`,
//! `append_intent_records_for_pending_moves`, `discard_incomplete_staging`,
//! `finish_committing_migration`, `admit_or_block_bc_index_writer`,
//! `reconcile_stale_admission_gate`, and `run_bc_index_migration`'s own
//! body all take (or, for `run_bc_index_migration`, construct and thread)
//! an `&impl Fs` — production call sites pass `&StdFs`. This makes every
//! `migration_fs::*` failpoint reachable from real execution (see this
//! module's own `failpoints`-feature smoke test in
//! `bc_1_18_011_b2_migration_test.rs`, which exercises
//! `migration_fs::rename` via `run_bc_index_migration`).
//!
//! Deliberately left OUTSIDE the `Fs` seam (per the OBL-1 design's own
//! "keep abstract domains tiny" guidance, §1.2/§1.4): directory-listing
//! enumeration (`read_active_txn_record`'s `std::fs::read_dir` glob scan —
//! the design's own §1.2 exclusion note), `mkdir` calls
//! (`std::fs::create_dir_all` for the migration-state/reservations/gen
//! directories — idempotent, not itself a crash-recovery decision input;
//! only the subsequent directory-entry `fsync_dir` durability barrier is
//! safety-critical, and that IS seamed), one-time infra bootstrap (the
//! `exclusive.lock` sentinel file, the reservations directory), and reads
//! of the pre-migration canonical source file
//! (`run_bc_index_migration`'s initial `std::fs::read_to_string` of
//! `BC-INDEX.md` — the migration's INPUT, not its own crash-atomicity
//! state). `pre_commit_fingerprint_recheck` and `resume_from_staging` are
//! also left unthreaded — neither is named in the OBL-1 implementer scope
//! (§6.1 item 1's explicit function list), and both read the same
//! pre-existing canonical/staged content rather than mutating migration
//! state.
//!
//! `write_txn_record` is threaded onto `Fs::write_temp` +
//! `Fs::fsync_file`, which — because `StdFs::write_temp` delegates to the
//! STRICT `F_FULLFSYNC`-class `super::migration_durable_write` primitive —
//! is a deliberate STRENGTHENING of the txn record's durability barrier
//! from `last_amended_migrate::atomic_write::write_atomic` (the lighter,
//! Unix-best-effort-dir-fsync primitive it used before this burst) to the
//! same STRICT primitive `CURRENT.json`/the intent log/`completed.json`
//! already use. Surfaced explicitly here rather than silently changed: the
//! txn record is exactly as crash-recovery-critical as those three
//! artifacts (the WHOLE `recover()` decision tree is keyed off it), so
//! leaving it on the weaker primitive while everything else in the same
//! recovery protocol uses the STRICT one was itself an inconsistency: this
//! is a correctness improvement, not a silent behavioral regression — it
//! makes the on-disk txn record durable, never fsync-weaker than the
//! CURRENT.json pointer whose commit it gates. Every other threaded
//! function keeps its EXACT prior underlying primitive (zero behavioral
//! change) — this is the one intentional exception, and it is additive
//! (stronger guarantee), never weaker.
//!
//! `recover()` ([`super::recover`]) remains a pure function over
//! already-gathered data (no `Fs` parameter) as designed — it performs no
//! I/O itself.

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

    /// Durably append `content` to `path` (creating it if absent) — the
    /// intent log's own I/O pattern (ADR-052 §Decision 7b), which is
    /// fundamentally an append-only WAL, not a whole-file replace. Added
    /// alongside [`Fs::write_temp`]/[`Fs::fsync_file`] rather than modeled
    /// as one of them: the OBL-1 design's own fault-injection boundary list
    /// (research §6.3) names `append+fsync(intent, ...)` as ITS OWN
    /// boundary, distinct from `write(temp) -> fsync(temp)`, so collapsing
    /// it onto `write_temp` would both be semantically wrong (an append is
    /// never a temp-then-rename publish) and would silently merge two
    /// fault-injection boundaries the design treats as separate. Production
    /// bundles the write+fsync into one call (mirroring the
    /// `write_temp`/`fsync_file` granularity note above) — delegates to the
    /// SAME `OpenOptions::append(true)` + `write_all` + `sync_all` sequence
    /// [`super::append_intent_log_record`] already performed before this
    /// seam existed. Model (Kani, formal-verifier's harness): appends to
    /// the abstract `live` namespace's content for `path`'s `FileId`, not
    /// yet `durable` until a harness-level promotion (mirrors
    /// `write_temp`'s own live/durable split).
    fn append(&self, path: &Path, content: &[u8]) -> Result<(), BcIndexMigrationError>;
}

// ---------------------------------------------------------------------------
// Fault-injection facade (OBL-1 §6.1 item 6 / §6.3). Compiles to nothing
// without the `failpoints` cargo feature — `fail_point!` never runs, and
// `fail` is not even a compiled dependency (`dep:fail` is optional). Named
// per the boundary list in the OBL-1 design's §6.3: one failpoint per `Fs`
// mutating operation, so the test-writer's crash-injection suite can target
// each by name via `fail::cfg("migration_fs::<op>", "abort")` /
// `FailScenario`.
//
// Two-arg upgrade (test-writer's `return(Err)` coverage requirement): every
// call site below now uses `fail_point!($name, $path)` rather than the
// single-arg `fail_point!($name)` form. The single-arg form's own doc
// comment (`fail` 0.5.1) states plainly: "Return is not supported for the
// fail point" -- configuring a `"return"` action against a single-arg fail
// point PANICS with that literal message rather than gracefully returning
// an `Err`, which is useless for exercising this trait's ordinary `?`-
// propagated error paths (disk-full, permission-denied, etc). The two-arg
// form passes `fail::eval`'s `Option<String>` (the configured `return(tag)`
// action's `tag` argument, `None` for a non-`Task::Return` action such as
// `off`/`sleep`/`panic`/`callback`) through [`migration_failpoint_error`],
// which maps `tag` to a concrete `BcIndexMigrationError::Io` variant
// carrying `path`'s context. `fail`'s own two-arg macro only actually
// returns early when `eval` yields `Some(..)` (i.e. only for a configured
// `Task::Return` action) -- this suite's `abort()`-based crash-injection
// scenarios (`cfg_callback`, a `Task::Callback`) are UNCHANGED by this
// upgrade: `Callback` actions never produce a `Return` value, so `eval`
// still yields `None` for them and this macro is still a no-op in that
// configuration, exactly as the single-arg form was.
// ---------------------------------------------------------------------------

/// Maps a `fail_point!` two-arg `return(tag)` action's configured `tag`
/// string to a concrete [`BcIndexMigrationError::Io`], carrying `path` for
/// caller context. `None` (a fail point configured with a non-`Task::Return`
/// action, e.g. `off`) is a `fail`/`eval` implementation detail this
/// function never actually sees in practice (the two-arg `fail_point!`
/// macro only invokes its closure argument -- this function -- when `eval`
/// already yielded `Some(tag)`), but is accepted so the closure's signature
/// matches `FnOnce(Option<String>) -> R` exactly; it maps to a generic
/// `Other`-kind error rather than being treated as unreachable, so a future
/// change to `fail`'s own evaluation semantics fails loud instead of
/// panicking. An unrecognized `tag` also fails loud (a descriptive `Other`-
/// kind error naming the bad tag) rather than silently defaulting to some
/// specific `io::ErrorKind` that might mask a typo in test configuration.
#[cfg(feature = "failpoints")]
fn migration_failpoint_error(
    name: &str,
    tag: Option<String>,
    path: &Path,
) -> BcIndexMigrationError {
    let kind = match tag.as_deref() {
        Some("not_found") => std::io::ErrorKind::NotFound,
        Some("permission_denied") => std::io::ErrorKind::PermissionDenied,
        Some("already_exists") => std::io::ErrorKind::AlreadyExists,
        Some("interrupted") => std::io::ErrorKind::Interrupted,
        Some("out_of_memory") => std::io::ErrorKind::OutOfMemory,
        Some("write_zero") => std::io::ErrorKind::WriteZero,
        Some("unexpected_eof") => std::io::ErrorKind::UnexpectedEof,
        Some("storage_full") => std::io::ErrorKind::StorageFull,
        None => std::io::ErrorKind::Other,
        Some(other) => {
            return BcIndexMigrationError::Io {
                path: path.to_path_buf(),
                source: std::io::Error::other(format!(
                    "migration_failpoint {name}: unrecognized return() tag {other:?} -- expected \
                     one of not_found/permission_denied/already_exists/interrupted/\
                     out_of_memory/write_zero/unexpected_eof/storage_full"
                )),
            };
        }
    };
    BcIndexMigrationError::Io {
        path: path.to_path_buf(),
        source: std::io::Error::new(
            kind,
            format!("migration_failpoint {name}: injected graceful return({tag:?})"),
        ),
    }
}

#[cfg(feature = "failpoints")]
macro_rules! migration_failpoint {
    ($name:expr, $path:expr) => {
        fail::fail_point!($name, |tag: Option<String>| Err(migration_failpoint_error(
            $name, tag, $path
        )))
    };
}

#[cfg(not(feature = "failpoints"))]
macro_rules! migration_failpoint {
    ($name:expr, $path:expr) => {
        let _ = $path;
    };
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
        migration_failpoint!("migration_fs::write_temp", path);
        super::migration_durable_write(path, content)
    }

    fn fsync_file(&self, path: &Path) -> Result<(), BcIndexMigrationError> {
        migration_failpoint!("migration_fs::fsync_file", path);
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
        migration_failpoint!("migration_fs::rename", to);
        std::fs::rename(from, to).map_err(|source| BcIndexMigrationError::Io {
            path: to.to_path_buf(),
            source,
        })
    }

    fn fsync_dir(&self, dir: &Path) -> Result<(), BcIndexMigrationError> {
        migration_failpoint!("migration_fs::fsync_dir", dir);
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
        migration_failpoint!("migration_fs::pointer_swap", target);
        std::fs::rename(tmp, target).map_err(|source| BcIndexMigrationError::Io {
            path: target.to_path_buf(),
            source,
        })
    }

    fn remove(&self, path: &Path) -> Result<(), BcIndexMigrationError> {
        migration_failpoint!("migration_fs::remove", path);
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

    fn append(&self, path: &Path, content: &[u8]) -> Result<(), BcIndexMigrationError> {
        migration_failpoint!("migration_fs::append", path);
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|source| BcIndexMigrationError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        use std::io::Write as _;
        file.write_all(content)
            .map_err(|source| BcIndexMigrationError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        // WAL boundary (ADR-052 §Decision 7b step 2/5): fsync before
        // returning, same discipline `write_temp`'s bundled durable-write
        // primitive already provides for staging publishes.
        file.sync_all().map_err(|source| BcIndexMigrationError::Io {
            path: path.to_path_buf(),
            source,
        })
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

    #[test]
    fn test_OBL1_std_fs_append_creates_file_then_appends_subsequent_calls() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("intent.log");
        let fs = StdFs;
        fs.append(&path, b"first-").unwrap();
        fs.append(&path, b"second").unwrap();
        assert_eq!(fs.read(&path).unwrap(), Some(b"first-second".to_vec()));
    }
}
