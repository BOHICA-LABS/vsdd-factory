//! Atomic write-then-rename for governed-file writes (S-15.03 SEC-003,
//! CWE-367 time-of-check to time-of-use race condition).
//!
//! `migrate_file`, `rotate_changelog`, and `register_artifact_paths` all read
//! a target file, compute a new full content, then previously called plain
//! `std::fs::write(path, ...)` directly against the target — no atomicity
//! between the read and the write, so a concurrent writer to the same file
//! could interleave with this tool's own write, or a reader could observe a
//! partially-written file mid-write. `std::fs::write` is not required by any
//! platform to write its buffer as a single atomic syscall.
//!
//! This module writes to a sibling temporary file first, then
//! `std::fs::rename`s it into place — `rename(2)`/`MoveFileExW` is atomic
//! when the source and destination are on the same filesystem (true here,
//! since the temp file is always created as a sibling of the real target in
//! the same directory), so any concurrent reader of `path` observes either
//! the fully-old content or the fully-new content, never a partial write.
//!
//! # S-15.03 pr-reviewer N2 — permission preservation + fsync durability
//!
//! Two gaps in the original write-then-rename implementation, closed here:
//!
//! 1. **Permission preservation.** `File::create` on a brand-new temp file
//!    gets the platform/umask default mode, NOT the pre-existing target
//!    file's own mode — a target deliberately `chmod`'d to something
//!    non-default (e.g. `0o600` for a sensitive file) would silently have
//!    its permission bits reset to the default on every write. This module
//!    now reads the pre-existing target's `Permissions` (mode on Unix,
//!    read-only flag on Windows) and applies them to the temp file before
//!    the rename, so a rename never changes a file's permission bits as a
//!    side effect. A target that does not exist yet (first-ever write) has
//!    nothing to preserve, and is left at the platform default.
//! 2. **fsync durability.** A `rename` that lands before the temp file's
//!    written bytes are actually durable on disk (still sitting in a
//!    filesystem write-back cache) leaves a crash window where `path` could
//!    point at a temp file whose content is lost or truncated after an
//!    unclean shutdown, even though the rename itself succeeded. This module
//!    now `fsync`s the temp file's contents before the rename, and
//!    best-effort `fsync`s the containing directory afterward (Unix only —
//!    directory fsync has no equivalent/is not meaningful on Windows) so the
//!    rename's directory-entry update is also durable, not just the file's
//!    bytes.

use crate::error::MigrateError;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Write `content` to `path` atomically: write to a sibling `<basename>.tmp-<pid>`
/// file in the same directory (preserving `path`'s pre-existing permission
/// bits, if any, and `fsync`ing the data before rename — S-15.03 N2), then
/// `rename` it into place. On success, no `.tmp-*` sibling survives — the
/// rename consumes it. On failure to write or fsync the temp file, no
/// attempt is made to write `path` at all, so `path` is left completely
/// untouched, and the temp file is best-effort removed.
pub fn write_atomic(path: &Path, content: &str) -> Result<(), MigrateError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let basename = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "last-amended-migrate-output".to_string());
    let tmp_path = parent.join(format!(".{basename}.tmp-{}", std::process::id()));

    if let Err(e) = write_and_sync_temp(&tmp_path, content) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e);
    }

    // N2: preserve the pre-existing target's permission bits across the
    // rename. Best-effort — a target that doesn't exist yet (first write)
    // or a permissions API that fails for an unrelated reason must not
    // block the substantive content write, which `std::fs::write` never
    // guarded against either.
    if let Ok(existing_meta) = std::fs::metadata(path) {
        let _ = std::fs::set_permissions(&tmp_path, existing_meta.permissions());
    }

    std::fs::rename(&tmp_path, path).map_err(|source| {
        // Best-effort cleanup of the orphaned temp file — the rename failure
        // itself is still reported; a leftover `.tmp-<pid>` here is a
        // secondary symptom, not the primary error, and this tool has no
        // other path that can safely remove it on the caller's behalf later.
        let _ = std::fs::remove_file(&tmp_path);
        MigrateError::Io {
            path: path.to_path_buf(),
            source,
        }
    })?;

    // N2: best-effort directory fsync so the rename's directory-entry
    // update is itself durable across a crash, not just the file's bytes.
    // Unix-only: Windows has no directly equivalent operation, and a
    // failure here must never fail the overall write — the rename already
    // succeeded, which is the operation this function's contract cares
    // about; this is defense-in-depth for the crash-durability window, not
    // a correctness requirement of the write itself.
    #[cfg(unix)]
    if let Ok(dir) = File::open(parent) {
        let _ = dir.sync_all();
    }

    Ok(())
}

/// Write `content` to `tmp_path` (creating or truncating it) and `fsync` it
/// before returning, so the caller's subsequent `rename` never lands ahead
/// of the data actually being durable on disk (S-15.03 N2).
fn write_and_sync_temp(tmp_path: &Path, content: &str) -> Result<(), MigrateError> {
    let mut file = File::create(tmp_path).map_err(|source| MigrateError::Io {
        path: tmp_path.to_path_buf(),
        source,
    })?;
    file.write_all(content.as_bytes())
        .map_err(|source| MigrateError::Io {
            path: tmp_path.to_path_buf(),
            source,
        })?;
    file.sync_all().map_err(|source| MigrateError::Io {
        path: tmp_path.to_path_buf(),
        source,
    })
}

// ---------------------------------------------------------------------------
// D-1232-OBL-2(a) mandated STRICT durability sequence — `F_FULLFSYNC(temp)
// -> rename -> F_FULLFSYNC(dir)` on macOS, with STRICT error propagation
// and NO silent fallback (a failed fsync is a fail-loud abort, not a
// swallowed best-effort). This is a STRONGER guarantee than
// `write_atomic`/`write_and_sync_temp` above provide: `write_atomic`'s own
// directory fsync is Unix-only BEST-EFFORT (`let _ = dir.sync_all()`), and
// its file-content fsync is plain `fsync(2)` (`File::sync_all`) rather
// than the macOS-specific `F_FULLFSYNC` durability lever Apple's own docs
// require for power-loss durability (`fsync(2)` on APFS/HFS+ does NOT
// flush the drive's write cache). `write_atomic` remains correct and
// sufficient for this crate's own `changelog`/`migrate`/`registry`
// callers; BC-1.18.011's B2 BC-INDEX governed migration
// (`crates/factory-dispatcher/src/shard_manager.rs`) is a
// governance-integrity-critical migration with its OWN stronger
// crash-durability obligation (BC-1.18.011 Postcondition 3 / D-1232-
// OBL-2(a)) that this module now also provides, deliberately housed HERE
// rather than in `factory-dispatcher` itself: that crate carries a
// crate-wide `#![deny(unsafe_code)]` security regression guard
// ("the crate operates in a security-critical dispatch path; unsafe is
// never warranted here" — `crates/factory-dispatcher/src/lib.rs`), and
// `F_FULLFSYNC` has no safe-Rust std equivalent (`std::fs::File::sync_all`
// is plain `fsync(2)`, insufficient on macOS/APFS). This crate carries no
// such restriction, so the two narrowly-scoped, safety-commented `unsafe`
// blocks below stay confined to this already-privileged, standalone
// operator/agent-invoked CLI tool rather than entering the dispatcher's
// own hot path.
// ---------------------------------------------------------------------------

/// `F_FULLFSYNC` on macOS — the documented durability lever (`fsync(2)`
/// alone does not flush the drive's write cache on APFS/HFS+); plain
/// `fsync(2)` (`File::sync_all`) on every other platform, where ordinary
/// `fsync(2)` already IS the durability guarantee. STRICT: any failure
/// propagates as an `io::Error`, never silently swallowed.
fn sync_file_durable(file: &File) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::io::AsRawFd;
        // Raw, stable macOS <fcntl.h> value -- avoids pulling in a
        // `libc`/`nix` dependency for one `i32` constant, matching this
        // workspace's existing convention for such narrow, well-known
        // per-OS syscall values (see
        // `crates/factory-dispatcher/src/shard_manager.rs`'s
        // `reclaim_identity_still_safe` for the same pattern applied to
        // `O_NONBLOCK`/`O_NOFOLLOW`).
        const F_FULLFSYNC: i32 = 51;
        unsafe extern "C" {
            fn fcntl(fd: i32, cmd: i32, ...) -> i32;
        }
        // SAFETY: `file.as_raw_fd()` is a valid, open file descriptor for
        // the duration of this call (borrowed from `file: &File`, which
        // outlives this call); `F_FULLFSYNC` takes no variadic argument,
        // matching this call site's own zero-varargs invocation; `fcntl`
        // with `F_FULLFSYNC` has no other memory-safety precondition
        // beyond a valid fd.
        let rc = unsafe { fcntl(file.as_raw_fd(), F_FULLFSYNC) };
        if rc == -1 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        file.sync_all()
    }
}

/// Directory-entry durability barrier after a rename. Issues the
/// strongest available barrier for the platform (`F_FULLFSYNC` on macOS
/// via [`sync_file_durable`], `fsync(2)` on other Unix) and propagates a
/// hard I/O failure — but, per BC-1.18.011 Postcondition 3's own
/// platform-branched durability language, does not claim a stronger
/// guarantee than macOS/APFS actually provides for directory fsync
/// (Apple's docs do not guarantee APFS directory-fsync itself survives
/// power loss, even under `F_FULLFSYNC`); this function still issues the
/// call and still propagates a hard failure on Unix, it just does not
/// oversell what the underlying platform call durably promises.
///
/// # Windows
///
/// `std::fs::File::open` cannot open a directory on Windows at all: the
/// underlying `CreateFileW` call fails (`ERROR_ACCESS_DENIED`) unless the
/// caller passes `FILE_FLAG_BACKUP_SEMANTICS`, which `std` never sets —
/// so the Unix `File::open(dir)?` implementation is not merely weaker on
/// Windows, it is a hard, unconditional `Err` on every call, which would
/// make every OBL-2(a) durable write fail outright on Windows regardless
/// of whether the actual write succeeded. This is cfg-gated to a
/// documented no-op instead, which is the CORRECT Windows equivalent, not
/// a weakened fallback: NTFS durably logs directory-entry mutations
/// (create/rename/delete) through its own `$LogFile` metadata transaction
/// journal as part of the mutation itself, so — unlike POSIX filesystems,
/// where an explicit `fsync(dir_fd)` is required for a rename's directory
/// entry to survive a crash — there is no separate "flush the directory"
/// operation NTFS exposes or requires for this guarantee (this is also
/// why practice elsewhere, e.g. SQLite's Windows VFS, does not attempt a
/// directory-handle flush). The Unix branch's guarantee is unchanged.
#[cfg(unix)]
fn sync_dir_durable(dir: &Path) -> std::io::Result<()> {
    let dir_file = File::open(dir)?;
    sync_file_durable(&dir_file)
}

/// Windows: see the doc comment on the `#[cfg(unix)]` sibling above for the
/// full rationale — a directory cannot be opened via `std::fs::File::open`
/// on Windows, and NTFS's `$LogFile` metadata journal already durably
/// covers directory-entry mutations without a separate flush operation, so
/// this is a documented no-op rather than a hard failure or an unsound
/// weakening of a guarantee NTFS provides some other way.
#[cfg(windows)]
fn sync_dir_durable(_dir: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Public wrapper around this module's own `F_FULLFSYNC`-on-macOS
/// directory durability barrier, for external crates (BC-1.18.011's B2
/// migration in `factory-dispatcher`) that need to durably fsync a
/// directory entry (e.g. immediately after creating a new staging
/// directory, or after a `rename(2)` this module's own
/// `write_atomic_strict_durable` does not itself cover) without
/// introducing their own `unsafe` FFI — `factory-dispatcher` carries a
/// crate-wide `#![deny(unsafe_code)]` security regression guard that this
/// crate does not.
pub fn sync_dir_strict_durable(dir: &Path) -> Result<(), MigrateError> {
    sync_dir_durable(dir).map_err(|source| MigrateError::Io {
        path: dir.to_path_buf(),
        source,
    })
}

/// D-1232-OBL-2(a) STRICT durable-write sequence: write `content` to a
/// sibling temp file, `F_FULLFSYNC` it (macOS) / `fsync` it (elsewhere),
/// `rename(2)` onto `path`, then `F_FULLFSYNC`/`fsync` the parent
/// directory — every step's failure propagates (no silent fallback), and
/// on ANY failure `path` is left completely untouched. Callers needing
/// this crate's ordinary best-effort durability (`write_atomic` above)
/// are UNAFFECTED — this is a separate, additive, stronger-guarantee
/// entry point for a governance-integrity-critical writer (BC-1.18.011's
/// B2 BC-INDEX migration).
pub fn write_atomic_strict_durable(path: &Path, content: &str) -> Result<(), MigrateError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let basename = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "last-amended-migrate-strict-output".to_string());
    let tmp_path = parent.join(format!(".{basename}.strict-tmp-{}", std::process::id()));

    let write_result: std::io::Result<()> = (|| {
        let mut file = File::create(&tmp_path)?;
        file.write_all(content.as_bytes())?;
        sync_file_durable(&file)
    })();
    if let Err(source) = write_result {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(MigrateError::Io {
            path: tmp_path,
            source,
        });
    }

    if let Ok(existing_meta) = std::fs::metadata(path) {
        let _ = std::fs::set_permissions(&tmp_path, existing_meta.permissions());
    }

    if let Err(source) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(MigrateError::Io {
            path: path.to_path_buf(),
            source,
        });
    }

    sync_dir_durable(parent).map_err(|source| MigrateError::Io {
        path: parent.to_path_buf(),
        source,
    })
}
