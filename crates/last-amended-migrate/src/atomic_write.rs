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
//! 1. **Permission preservation.** Creating a brand-new temp file
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
//!
//! # PR #842 — Windows CI transient-rename-denial mitigation
//!
//! `windows-x64` CI on PR #842 failed 5/5 `bc_1_18_011_b2_migration_test`
//! tests with `PermissionDenied` (`os error 5`, `ERROR_ACCESS_DENIED`) on
//! the final rename of a first-time write (destination did not yet exist in
//! any of the 5 failures). Code inspection confirmed neither `write_atomic`
//! nor `write_atomic_strict_durable` ever holds its own temp-file `File`
//! handle open across the rename — both close it via `Drop` at a
//! function-local scope boundary strictly before the rename call. The
//! verified cause is therefore a THIRD PARTY (GitHub Actions' Windows
//! runners run Windows Defender real-time protection by default, including
//! against `%TEMP%`) transiently opening the just-written file for a
//! post-write scan in the narrow window between our `Drop` and our next
//! syscall — a well-documented class of Windows filesystem flakiness, not a
//! handle leak in this module. Every rename in this module (and its
//! sibling call sites in `factory-dispatcher`, swept per TD-VSDD-060) now
//! goes through [`rename_with_retry`], which absorbs that transient window
//! with a short bounded backoff while still propagating a genuine,
//! persistent permission failure unchanged.

use crate::error::MigrateError;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Bounded attempt count for [`rename_with_retry`]'s Windows-transient-lock
/// mitigation (initial attempt + up to 4 retries = 5 total).
///
/// Production code only reaches this constant via the `#[cfg(windows)]` arm
/// of [`rename_with_retry`] (Finding 4) — on every other target it is only
/// referenced by this module's own cross-platform unit tests (exercised
/// through [`rename_with_retry_impl`]), hence `allow(dead_code)` on
/// non-Windows targets rather than a false "unused" warning.
#[cfg_attr(not(windows), allow(dead_code))]
const RENAME_RETRY_MAX_ATTEMPTS: u32 = 5;

/// Base backoff delay for [`rename_with_retry`]'s exponential backoff:
/// 20ms, 40ms, 80ms, 160ms between the 5 attempts (~300ms worst-case total),
/// short enough to be invisible in normal operation but long enough to
/// outlast a transient Windows Defender/indexer post-write scan handle.
///
/// See [`RENAME_RETRY_MAX_ATTEMPTS`]'s doc comment for why this is
/// `allow(dead_code)`-gated on non-Windows targets.
#[cfg_attr(not(windows), allow(dead_code))]
const RENAME_RETRY_BASE_DELAY: Duration = Duration::from_millis(20);

/// `std::fs::rename` wrapped in a bounded retry-with-backoff for
/// `io::ErrorKind::PermissionDenied` — every write-then-rename call site in
/// this module (and its downstream callers in `factory-dispatcher`) routes
/// its final rename through this helper rather than calling
/// `std::fs::rename` directly (TD-VSDD-060 sibling-site sweep, S-25.02
/// cluster-5 PR #842 Windows-CI fix).
///
/// # Why this exists — verified root cause, not speculation
///
/// PR #842's `windows-x64` CI leg failed 5/5 `bc_1_18_011_b2_migration_test`
/// tests with `Io { PermissionDenied, os error 5 ("Access is denied.") }` on
/// the rename step of a **first-time** write (the destination path did not
/// exist yet in any of the 5 failures — confirmed against the CI log, not
/// assumed). `os error 5` (`ERROR_ACCESS_DENIED`) on `MoveFileExW` is the
/// textbook Windows symptom of SOME process holding an open handle to the
/// source or destination path, without `FILE_SHARE_DELETE`, at the instant
/// of the rename — Windows (unlike POSIX, see this module's own top-of-file
/// doc comment) refuses to rename a file out from under an open handle.
///
/// This module's own writers (`write_atomic`, `write_atomic_strict_durable`)
/// already close their own temp-file `File` handle deterministically via
/// Rust's `Drop` — the handle is a closure-local/function-local binding
/// whose owning scope ends, and is dropped, strictly before either
/// function's own rename call — so the open handle triggering this failure
/// is never this process's own write handle (verified by inspection: no
/// code path retains a `File` across the rename in either function). On
/// GitHub Actions' Windows-hosted runners, Windows Defender real-time
/// protection is enabled by default (including for `%TEMP%`), and its
/// well-documented behavior is to open a transient post-write scan handle
/// on a just-closed file asynchronously, in the narrow window between this
/// process's `CloseHandle` and its very next syscall — exactly the gap
/// between our `File::drop` and our `rename` call. That handle clears
/// itself within single-digit milliseconds once the scan completes, which
/// is precisely the class of failure a short bounded retry resolves without
/// masking a genuine, persistent permission problem: a real, non-transient
/// access-denied condition (a read-only ACL, a directory permission
/// problem, or a caller-held handle that never closes) still fails after
/// every retry is exhausted and propagates the final, unmodified error.
///
/// # Windows-only — `#[cfg(windows)]`-gated, not applied on Unix (Finding 4)
///
/// This retry mechanism exists ONLY for the Windows AV/indexer transient-
/// lock symptom documented above; it is scoped to `#[cfg(windows)]` and, on
/// every other target, this function is a direct, zero-overhead passthrough
/// to `std::fs::rename` with no retry loop and no sleep at all — see the
/// `#[cfg(not(windows))]` definition below.
///
/// An earlier revision of this fix applied the retry loop uniformly on
/// every platform, reasoning that `rename(2)` never returns `EACCES` for
/// the "open handle" reason on Unix (Unix permits renaming a file with
/// open handles unconditionally) so a genuine Unix `PermissionDenied`
/// would only be retried, never masked. That reasoning missed a real,
/// measurable cost: Unix `EACCES`/`EPERM` (e.g. a caller-induced read-only
/// directory, or an unrelated permission misconfiguration) DOES map to
/// `io::ErrorKind::PermissionDenied` in Rust's std, so the uniform version
/// silently retried genuine Unix permission failures too — adding up to
/// ~300ms of pointless delay before a real, non-transient permission error
/// ever reached the caller, for a race this module's own doc comment (see
/// above) establishes is Windows-only. Scoping the mechanism to
/// `#[cfg(windows)]` removes that latency entirely on Unix, which has no
/// use for this retry: a Unix rename either succeeds on the first attempt
/// or fails for a reason retrying can never fix.
#[cfg(windows)]
pub fn rename_with_retry(from: &Path, to: &Path) -> std::io::Result<()> {
    rename_with_retry_impl(
        from,
        to,
        |f: &Path, t: &Path| std::fs::rename(f, t),
        std::thread::sleep,
    )
}

/// Non-Windows: a direct, zero-overhead passthrough to `std::fs::rename`.
/// This platform has no equivalent of the Windows AV/indexer transient-lock
/// race [`rename_with_retry`] exists to absorb (see the `#[cfg(windows)]`
/// sibling's doc comment above), so there is no retry loop, no sleep, and
/// no added latency on any Unix (or other non-Windows target's) hot path —
/// same public signature as the Windows arm, so none of this module's own
/// or `factory-dispatcher`'s 6 call sites need to change.
#[cfg(not(windows))]
pub fn rename_with_retry(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::rename(from, to)
}

/// Testable core of [`rename_with_retry`]: identical retry-with-backoff
/// logic, but with the rename operation and the sleep function injected
/// rather than hardcoded to `std::fs::rename`/`std::thread::sleep`, so the
/// retry loop itself (attempt counting, backoff sequence, which errors are
/// retried) can be unit-tested deterministically and instantly — without a
/// real filesystem race or real `Duration`-length sleeps.
///
/// Deliberately compiled on every target, not just `#[cfg(windows)]`: this
/// keeps the retry logic itself unit-testable cross-platform (Finding 1/3),
/// even though production code on non-Windows targets never calls it (the
/// `#[cfg(not(windows))]` arm of [`rename_with_retry`] bypasses it entirely
/// — Finding 4) — hence `allow(dead_code)` there.
#[cfg_attr(not(windows), allow(dead_code))]
fn rename_with_retry_impl<R, S>(
    from: &Path,
    to: &Path,
    mut rename_fn: R,
    mut sleep_fn: S,
) -> std::io::Result<()>
where
    R: FnMut(&Path, &Path) -> std::io::Result<()>,
    S: FnMut(Duration),
{
    let mut attempt = 0u32;
    loop {
        match rename_fn(from, to) {
            Ok(()) => return Ok(()),
            Err(source)
                if is_retryable_rename_error(&source)
                    && attempt + 1 < RENAME_RETRY_MAX_ATTEMPTS =>
            {
                sleep_fn(RENAME_RETRY_BASE_DELAY * (1 << attempt));
                attempt += 1;
            }
            Err(source) => return Err(source),
        }
    }
}

/// Returns `true` when `err` represents the class of transient Windows
/// AV/indexer lock contention [`rename_with_retry`] absorbs with a bounded
/// retry, rather than a genuine, persistent failure that must propagate
/// immediately.
///
/// # Finding 3 — PR #842 fix-burst
///
/// The original condition checked only `io::ErrorKind::PermissionDenied`
/// (the `ERROR_ACCESS_DENIED` / `os error 5` symptom verified in CI). On
/// Windows, `ERROR_SHARING_VIOLATION` (raw OS error 32) and
/// `ERROR_LOCK_VIOLATION` (raw OS error 33) are the same class of transient
/// AV/indexer lock race, but Rust's std does not map either one to
/// `ErrorKind::PermissionDenied` — they fall into an uncategorized kind, so
/// the original condition silently missed them. `raw_os_error()` is not a
/// meaningful or portable check off Windows (the numeric codes are a Win32
/// convention), so that half of the condition is `#[cfg(windows)]`-gated;
/// the `PermissionDenied` check continues to apply on every platform (its
/// own scope — whether it should apply on non-Windows targets at all — is
/// addressed separately by [`rename_with_retry`]'s own `cfg(windows)` split,
/// Finding 4).
///
/// See [`rename_with_retry_impl`]'s doc comment for why this is
/// `allow(dead_code)`-gated on non-Windows targets.
#[cfg_attr(not(windows), allow(dead_code))]
fn is_retryable_rename_error(err: &std::io::Error) -> bool {
    if err.kind() == std::io::ErrorKind::PermissionDenied {
        return true;
    }
    #[cfg(windows)]
    {
        matches!(err.raw_os_error(), Some(32) | Some(33))
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Bounded number of fresh-suffix attempts [`create_exclusive_temp`] makes
/// when `create_new` reports the candidate name already exists, before it
/// fails closed with `ErrorKind::AlreadyExists`.
const TEMP_CREATE_MAX_ATTEMPTS: u32 = 16;

/// Unpredictable 64-bit suffix for a temp-file name (S-25.08 PR #848
/// SEC-002, CWE-377).
///
/// Source choice: `std::hash::RandomState`. Its SipHash keys are seeded
/// from the OS CSPRNG at first use (per thread), so a same-uid attacker
/// who can read the pid and the clock still cannot predict the output;
/// the hasher additionally mixes the pid, the wall-clock nanoseconds and
/// a process-wide counter so successive calls within one thread never
/// repeat. This avoids adding a direct `getrandom`/`rand`/`uuid`
/// dependency to this standalone CLI crate for one 64-bit nonce. The
/// security property does NOT rest on unpredictability alone: the file
/// is opened with `create_new` (`O_EXCL`), so a pre-planted file or
/// symlink at a guessed name is refused rather than followed.
#[doc(hidden)]
pub fn unpredictable_suffix() -> u64 {
    use std::hash::{BuildHasher, Hasher, RandomState};
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(COUNTER.fetch_add(1, Ordering::Relaxed));
    hasher.write_u32(std::process::id());
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    hasher.write_u64(nanos);
    hasher.finish()
}

/// Exclusively create (`create_new`, i.e. `O_EXCL` on Unix and
/// `CREATE_NEW` on Windows) a fresh sibling temp file
/// `.<basename>.<kind>-<pid>-<nonce:016x>` in `parent`, returning the open
/// handle and its path. A name collision (`AlreadyExists`, including a
/// planted file or symlink, dangling or not) is retried with a new
/// suffix, bounded by `TEMP_CREATE_MAX_ATTEMPTS` (16); exhaustion or any
/// other error is returned as-is (fail closed, nothing is written
/// through a pre-existing path). On Unix the file is created with mode
/// `0o600`, so it is never briefly readable by others.
///
/// Writers that replace a governed target should use
/// [`create_exclusive_temp_for_target`], which also picks the correct
/// creation mode for a target that does not exist yet.
pub fn create_exclusive_temp(
    parent: &Path,
    basename: &str,
    kind: &str,
) -> std::io::Result<(File, PathBuf)> {
    create_temp_with_mode(
        parent,
        basename,
        kind,
        TEMP_MODE_OWNER_ONLY,
        unpredictable_suffix,
    )
}

/// Suffix-injectable core of [`create_exclusive_temp`] for this crate's unit
/// tests only: `suffix_fn` is called exactly once per attempt (at most
/// `TEMP_CREATE_MAX_ATTEMPTS`). Only `AlreadyExists` is retried; any other
/// error is returned immediately, and exhaustion returns the last
/// `AlreadyExists` error. No cross-crate caller needs it (the dispatcher
/// marker writer drives `create_exclusive_temp_for_target_with`), so it is
/// not part of the public surface.
#[cfg(test)]
pub(crate) fn create_exclusive_temp_with<F: FnMut() -> u64>(
    parent: &Path,
    basename: &str,
    kind: &str,
    suffix_fn: F,
) -> std::io::Result<(File, PathBuf)> {
    create_temp_with_mode(parent, basename, kind, TEMP_MODE_OWNER_ONLY, suffix_fn)
}

/// Owner-only creation mode for a temp that will replace an existing target.
const TEMP_MODE_OWNER_ONLY: u32 = 0o600;
/// `File::create`'s requested mode; the process umask narrows it.
const TEMP_MODE_DEFAULT_CREATE: u32 = 0o666;

/// Like [`create_exclusive_temp`], with the creation mode chosen from
/// `target` (the file the temp will be renamed over):
///
/// - `target` EXISTS: the temp is created `0o600`. Pair with
///   [`apply_target_permissions`] on the open handle before closing it, so
///   the temp is owner-only until it takes the target's mode.
/// - `target` does NOT exist: the temp is created with `0o666` so the
///   process umask applies, exactly the semantics of `File::create`. No
///   `umask(2)` call, probe file or fixed fallback is involved.
///
/// # Creation-mode trust analysis (SEC-002-N3)
///
/// The `0o600` birth mode closes the window in which a temp holding the
/// content of a restrictive target (e.g. a `0o600` file) would be briefly
/// readable at the umask default. For a NEW target the temp is born
/// `0o666 & !umask`, the very mode `File::create` gives the final file, so
/// the temp is never more permissive than that result for as long as this
/// writer is the file's only creator. Caveat: if another process creates
/// the target between the existence check and the rename, the temp (holding
/// this writer's content) was born at the umask default while the file it
/// replaces may be more restrictive; the exposure is bounded by the umask
/// default, i.e. what a plain `File::create` of the same content would
/// have exposed, and closing it would need a lock or `O_TMPFILE`/`linkat`,
/// which this crate does not use.
///
/// # Trust boundary (SEC-002-N1)
///
/// Only the temp LEAF is protected (`O_EXCL`, unpredictable suffix). The
/// parent directory of `target` is trusted: a symlinked or attacker-writable
/// parent redirects every path beneath it and is out of scope.
pub fn create_exclusive_temp_for_target(
    parent: &Path,
    basename: &str,
    kind: &str,
    target: &Path,
) -> std::io::Result<(File, PathBuf)> {
    create_exclusive_temp_for_target_with(parent, basename, kind, target, unpredictable_suffix)
}

/// Suffix-injectable core of [`create_exclusive_temp_for_target`].
#[doc(hidden)]
pub fn create_exclusive_temp_for_target_with<F: FnMut() -> u64>(
    parent: &Path,
    basename: &str,
    kind: &str,
    target: &Path,
    suffix_fn: F,
) -> std::io::Result<(File, PathBuf)> {
    let mode = if target.exists() {
        TEMP_MODE_OWNER_ONLY
    } else {
        TEMP_MODE_DEFAULT_CREATE
    };
    create_temp_with_mode(parent, basename, kind, mode, suffix_fn)
}

/// SEC-R3-004: `basename` and `kind` are interpolated into a sibling file
/// name, so each must be a single plain name component: non-empty, no `/`
/// (nor `\` on Windows), not `.` or `..`, no NUL. Rejected with
/// `InvalidInput` before anything is created.
fn validate_temp_name_component(what: &str, value: &str) -> std::io::Result<()> {
    let has_separator = value.contains('/') || (cfg!(windows) && value.contains('\\'));
    if value.is_empty() || value == "." || value == ".." || value.contains('\0') || has_separator {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("unsafe temp-file {what} {value:?}: must be a single plain name component"),
        ));
    }
    Ok(())
}

fn create_temp_with_mode<F: FnMut() -> u64>(
    parent: &Path,
    basename: &str,
    kind: &str,
    #[cfg_attr(not(unix), allow(unused_variables))] mode: u32,
    mut suffix_fn: F,
) -> std::io::Result<(File, PathBuf)> {
    validate_temp_name_component("basename", basename)?;
    validate_temp_name_component("kind", kind)?;
    let mut last_err = None;
    for _ in 0..TEMP_CREATE_MAX_ATTEMPTS {
        let candidate = parent.join(format!(
            ".{basename}.{kind}-{}-{:016x}",
            std::process::id(),
            suffix_fn()
        ));
        let mut opts = OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(mode);
        }
        match opts.open(&candidate) {
            Ok(file) => return Ok((file, candidate)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => last_err = Some(e),
            Err(e) => return Err(e),
        }
    }
    Err(last_err.unwrap_or_else(|| std::io::Error::from(std::io::ErrorKind::AlreadyExists)))
}

/// Give the temp its FINAL permissions through the OPEN handle (`fchmod`
/// on Unix, no path-based `chmod`, so no path race), before the handle is
/// closed and the temp renamed.
///
/// - `target` exists: the handle takes the target's permission bits
///   (`& 0o777`; setuid/setgid/sticky are never copied) (S-15.03 N2: a
///   rename must not change a file's permission bits).
/// - `target` does not exist: the temp already carries the `File::create`
///   default from [`create_exclusive_temp_for_target`]; a temp born
///   `0o600` for a target that has since vanished is moved to that same
///   default (see `apply_vanished_target_default`).
/// - Any other metadata error is returned.
///
/// A failed `fchmod` is RETURNED (never swallowed): the caller removes the
/// temp and fails the write, so a file never lands with permissions other
/// than the ones this function was asked to apply (NB-B).
pub fn apply_target_permissions(file: &File, target: &Path) -> std::io::Result<()> {
    match std::fs::metadata(target) {
        Ok(existing_meta) => {
            #[cfg(unix)]
            {
                // SEC-R3-002: copy the permission bits only. setuid/setgid/
                // sticky (`0o7000`) are never inherited from the target.
                use std::os::unix::fs::PermissionsExt;
                let mode = existing_meta.permissions().mode() & 0o777;
                file.set_permissions(std::fs::Permissions::from_mode(mode))?;
            }
            #[cfg(not(unix))]
            file.set_permissions(existing_meta.permissions())?;
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            #[cfg(unix)]
            apply_vanished_target_default(file)?;
            Ok(())
        }
        // Any other failure to read the target's metadata (EACCES, ...):
        // fail closed rather than guess the mode.
        Err(e) => Err(e),
    }
}

/// SEC-R3-001: the target does not exist at apply time. A temp born for a
/// NEW target already carries the `File::create` default and is left alone.
/// A temp born `0o600` because the target existed at creation (which has
/// since vanished) must not stay stuck at `0o600`: it takes the umask
/// default `0o666 & !umask`, the mode `File::create` would have produced.
///
/// The umask is READ, never changed: [`process_umask`] uses
/// `/proc/self/status` on Linux and otherwise a child `sh` that inherits
/// it. No `umask(2)` call (process-global, racy against other threads) and
/// no probe file. If the umask cannot be determined the temp is left at
/// owner-only `0o600`, the restrictive direction (fail-safe).
#[cfg(unix)]
fn apply_vanished_target_default(file: &File) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let current = file.metadata()?.permissions().mode() & 0o777;
    if current != TEMP_MODE_OWNER_ONLY {
        return Ok(()); // born with the File::create default already
    }
    if let Some(umask) = process_umask() {
        let default = TEMP_MODE_DEFAULT_CREATE & !umask;
        if default != current {
            file.set_permissions(std::fs::Permissions::from_mode(default))?;
        }
    }
    Ok(())
}

/// The process umask, read without `umask(2)` and without a probe file.
#[cfg(unix)]
fn process_umask() -> Option<u32> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        if let Some(v) = status.lines().find_map(|l| l.strip_prefix("Umask:")) {
            if let Ok(mask) = u32::from_str_radix(v.trim(), 8) {
                return Some(mask & 0o777);
            }
        }
    }
    // POSIX: a child process inherits the parent's umask; `sh`'s `umask`
    // builtin prints it in octal.
    let out = std::process::Command::new("/bin/sh")
        .args(["-c", "umask"])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    u32::from_str_radix(text.trim(), 8).ok().map(|m| m & 0o777)
}

/// Write `content` to `path` atomically: write to an exclusively-created
/// sibling `.<basename>.tmp-<pid>-<nonce>` file in the same directory
/// (on Unix created `0o600` when `path` already exists, then `fchmod`ed to
/// `path`'s mode on the open handle; created `0o666 & !umask`, i.e.
/// `File::create` semantics, when `path` is new; `fsync`ed before rename —
/// S-15.03 N2; see [`create_exclusive_temp_for_target`]), then
/// `rename` it into place. On success, no `.<basename>.tmp-*` sibling
/// survives — the rename consumes it. On failure to write or fsync the temp file, no
/// attempt is made to write `path` at all, so `path` is left completely
/// untouched, and the temp file is best-effort removed.
///
/// # Trust boundary (SEC-002-N1)
///
/// The PARENT directory of `path` is trusted. The temp-file protection
/// (`create_new`/`O_EXCL`, unpredictable suffix) covers the temp LEAF only:
/// it refuses a planted file or symlink at the temp name, but it does not
/// and cannot defend against a symlinked or attacker-writable parent
/// directory, which redirects every path beneath it. Callers must only
/// pass governed paths under directories they control.
///
/// # Crash behaviour (N-4)
///
/// A crash (SIGKILL, power loss) between temp creation and the rename can
/// orphan a `.<basename>.tmp-<pid>-<nonce>` file in the parent directory.
/// Orphans are inert (never read as `path`) and no sweep in this crate
/// removes them; an orphan is only removed by the best-effort cleanup on a
/// handled write/rename error, or manually.
pub fn write_atomic(path: &Path, content: &str) -> Result<(), MigrateError> {
    write_atomic_with_perms_fn(path, content, apply_target_permissions)
}

/// Permission-apply-injectable core of [`write_atomic`] (S-25.08 PR #848
/// NB-B test seam): `perms_fn` is called once on the open temp handle with
/// the final `path`. Production always passes [`apply_target_permissions`];
/// tests inject a failing closure to prove an `fchmod` error removes the
/// temp, fails the write and leaves `path` untouched.
#[doc(hidden)]
pub fn write_atomic_with_perms_fn<P>(
    path: &Path,
    content: &str,
    perms_fn: P,
) -> Result<(), MigrateError>
where
    P: FnOnce(&File, &Path) -> std::io::Result<()>,
{
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let basename = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "last-amended-migrate-output".to_string());
    let (file, tmp_path) = create_exclusive_temp_for_target(parent, &basename, "tmp", path)
        .map_err(|source| MigrateError::Io {
            // N-3: the candidate name is not known when creation fails (it is
            // random and may have exhausted all attempts), so report the one
            // concrete path that is: the directory the temp file could not be
            // created in — never a non-existent glob.
            path: parent.to_path_buf(),
            source,
        })?;

    if let Err(e) = write_and_sync_temp(file, &tmp_path, path, content, perms_fn) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e);
    }

    // `rename_with_retry` (not a bare `std::fs::rename`) — see its own doc
    // comment for the verified Windows CI root cause (transient AV/indexer
    // handle, never this function's own write handle, which
    // `write_and_sync_temp` already closed via `Drop` before returning).
    rename_with_retry(&tmp_path, path).map_err(|source| {
        // Best-effort cleanup of the orphaned temp file — the rename failure
        // itself is still reported; a leftover `.tmp-<pid>-<nonce>` here is a
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

/// Write `content` through `file` (the exclusively-created handle for
/// `tmp_path`, consumed so it is dropped at return) and `fsync` it before
/// returning, so the caller's subsequent `rename` never lands ahead of the
/// data actually being durable on disk (S-15.03 N2).
fn write_and_sync_temp<P>(
    mut file: File,
    tmp_path: &Path,
    target: &Path,
    content: &str,
    perms_fn: P,
) -> Result<(), MigrateError>
where
    P: FnOnce(&File, &Path) -> std::io::Result<()>,
{
    file.write_all(content.as_bytes())
        .map_err(|source| MigrateError::Io {
            path: tmp_path.to_path_buf(),
            source,
        })?;
    perms_fn(&file, target).map_err(|source| MigrateError::Io {
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

/// Non-Unix (Windows, and any other non-Unix target such as
/// `wasm32-wasip1`): see the doc comment on the `#[cfg(unix)]` sibling
/// above for the full Windows rationale — a directory cannot be opened via
/// `std::fs::File::open` on Windows, and NTFS's `$LogFile` metadata journal
/// already durably covers directory-entry mutations without a separate
/// flush operation, so this is a documented no-op rather than a hard
/// failure or an unsound weakening of a guarantee NTFS provides some other
/// way. Gated on `not(unix)` rather than `windows` specifically so every
/// non-Unix compilation target — including the `wasm32-wasip1` hook-plugin
/// target, which is neither `unix` nor `windows` — still has a function
/// body; a bare `#[cfg(windows)]` complement to `#[cfg(unix)]` leaves
/// `wasm32-wasip1` (and any other future non-Unix, non-Windows target)
/// with no definition of `sync_dir_durable` at all, a hard compile error
/// for every caller (here, `write_atomic_strict_durable`).
#[cfg(not(unix))]
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
///
/// The same trust boundary (parent directory trusted, SEC-002-N1) and
/// crash behaviour (a crash can orphan a `.<basename>.strict-tmp-<pid>-<nonce>`
/// file, which no sweep removes, N-4) documented on [`write_atomic`] apply.
pub fn write_atomic_strict_durable(path: &Path, content: &str) -> Result<(), MigrateError> {
    write_atomic_strict_durable_with_perms_fn(path, content, apply_target_permissions)
}

/// Permission-apply-injectable core of [`write_atomic_strict_durable`]
/// (NB-B test seam; see [`write_atomic_with_perms_fn`]).
#[doc(hidden)]
pub fn write_atomic_strict_durable_with_perms_fn<P>(
    path: &Path,
    content: &str,
    perms_fn: P,
) -> Result<(), MigrateError>
where
    P: FnOnce(&File, &Path) -> std::io::Result<()>,
{
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let basename = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "last-amended-migrate-strict-output".to_string());
    let (file, tmp_path) = create_exclusive_temp_for_target(parent, &basename, "strict-tmp", path)
        .map_err(|source| {
            MigrateError::Io {
                // N-3: concrete directory, not a glob (see `write_atomic`).
                path: parent.to_path_buf(),
                source,
            }
        })?;

    if let Err(source) = write_and_sync_temp_strict(file, path, content, perms_fn) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(MigrateError::Io {
            path: tmp_path,
            source,
        });
    }

    // `rename_with_retry` (not a bare `std::fs::rename`) — see its own doc
    // comment for the verified Windows CI root cause (transient AV/indexer
    // handle, never this function's own write handle, which
    // `write_and_sync_temp_strict` already closed via `Drop` before
    // returning — PR #842).
    if let Err(source) = rename_with_retry(&tmp_path, path) {
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

/// Write `content` through `file` (the exclusively-created handle for the
/// temp file) and durably sync it — `F_FULLFSYNC` on macOS, plain `fsync` elsewhere, via
/// [`sync_file_durable`] — before returning, so the caller's subsequent
/// rename never lands ahead of the data actually being durable on disk
/// (D-1232-OBL-2(a)).
///
/// `file` is moved into THIS function's own stack frame and never
/// returned or exposed to the caller: it is dropped (its OS handle closed
/// via `Drop`) at this function's return, strictly BEFORE
/// [`write_atomic_strict_durable`]'s subsequent `rename_with_retry` call —
/// named as its own function (mirroring [`write_and_sync_temp`]'s identical
/// shape for [`write_atomic`]) rather than an inline closure specifically
/// so this scoping is unambiguous on inspection (PR #842 Windows-CI
/// investigation).
fn write_and_sync_temp_strict<P>(
    mut file: File,
    target: &Path,
    content: &str,
    perms_fn: P,
) -> std::io::Result<()>
where
    P: FnOnce(&File, &Path) -> std::io::Result<()>,
{
    file.write_all(content.as_bytes())?;
    perms_fn(&file, target)?;
    sync_file_durable(&file)
}

// ---------------------------------------------------------------------------
// PR #842 fix-burst (pr-reviewer REQUEST_CHANGES) — unit tests for
// `rename_with_retry`'s retry-with-backoff mechanism, exercised through the
// injectable `rename_with_retry_impl` core so the retry loop itself (attempt
// counting, backoff sequence, which error conditions are retried) is
// verified deterministically and instantly, with no real filesystem race and
// no real `Duration`-length sleeps.
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// A scripted rename error: either a plain `ErrorKind` (portable, what a
    /// non-Windows caller would actually observe) or a raw OS error code
    /// (used to simulate Windows-specific codes like `ERROR_SHARING_VIOLATION`
    /// (32) / `ERROR_LOCK_VIOLATION` (33) regardless of the host platform this
    /// test suite actually runs on — `std::io::Error::from_raw_os_error` is a
    /// portable *constructor*, even though the numeric code it wraps is only
    /// ever produced by a real Windows syscall in production).
    #[derive(Clone, Copy)]
    enum ScriptedError {
        Kind(std::io::ErrorKind),
        Raw(i32),
    }

    impl ScriptedError {
        fn make(self) -> std::io::Error {
            match self {
                ScriptedError::Kind(kind) => std::io::Error::from(kind),
                ScriptedError::Raw(code) => std::io::Error::from_raw_os_error(code),
            }
        }
    }

    /// Builds a `rename_fn` double that fails with `err` for the first
    /// `fail_count` calls, then succeeds on every call after that. Returns
    /// the closure plus a shared call counter the test can inspect
    /// afterward, since the closure itself is moved into
    /// `rename_with_retry_impl`.
    fn scripted_rename(
        fail_count: u32,
        err: ScriptedError,
    ) -> (
        impl FnMut(&Path, &Path) -> std::io::Result<()>,
        Rc<RefCell<u32>>,
    ) {
        let calls = Rc::new(RefCell::new(0u32));
        let calls_inner = Rc::clone(&calls);
        let rename_fn = move |_from: &Path, _to: &Path| -> std::io::Result<()> {
            let mut n = calls_inner.borrow_mut();
            *n += 1;
            if *n <= fail_count {
                Err(err.make())
            } else {
                Ok(())
            }
        };
        (rename_fn, calls)
    }

    /// Finding 1: fails `PermissionDenied` twice, then succeeds — the retry
    /// loop must absorb both failures and return `Ok`, having called the
    /// underlying rename exactly 3 times (2 failures + 1 success), never
    /// more.
    #[test]
    fn test_rename_with_retry_impl_retries_then_succeeds() {
        let (rename_fn, calls) =
            scripted_rename(2, ScriptedError::Kind(std::io::ErrorKind::PermissionDenied));
        let mut sleeps = Vec::new();

        let result = rename_with_retry_impl(Path::new("from"), Path::new("to"), rename_fn, |d| {
            sleeps.push(d)
        });

        assert!(
            result.is_ok(),
            "must succeed once the underlying rename does: {result:?}"
        );
        assert_eq!(
            *calls.borrow(),
            3,
            "must call the underlying rename exactly N+1 times (2 failures + 1 success)"
        );
    }

    /// Finding 1: a `PermissionDenied` that NEVER clears must propagate the
    /// original error after exactly `RENAME_RETRY_MAX_ATTEMPTS` attempts —
    /// not more (bounded), not fewer (the full retry budget is spent before
    /// giving up).
    #[test]
    fn test_rename_with_retry_impl_exhausts_retries_then_propagates_original_error() {
        let (rename_fn, calls) = scripted_rename(
            RENAME_RETRY_MAX_ATTEMPTS + 5,
            ScriptedError::Kind(std::io::ErrorKind::PermissionDenied),
        );

        let result = rename_with_retry_impl(Path::new("from"), Path::new("to"), rename_fn, |_| {});

        let err = result.expect_err("must propagate an error once retries are exhausted");
        assert_eq!(
            err.kind(),
            std::io::ErrorKind::PermissionDenied,
            "the original error kind must be propagated unchanged"
        );
        assert_eq!(
            *calls.borrow(),
            RENAME_RETRY_MAX_ATTEMPTS,
            "must attempt exactly RENAME_RETRY_MAX_ATTEMPTS times total, not more or fewer"
        );
    }

    /// Finding 1: a non-retried error kind (e.g. `NotFound`) must propagate
    /// immediately after exactly one attempt — a genuinely different failure
    /// reason must never be masked behind the transient-lock retry budget.
    #[test]
    fn test_rename_with_retry_impl_does_not_retry_non_retried_error_kinds() {
        let (rename_fn, calls) = scripted_rename(
            RENAME_RETRY_MAX_ATTEMPTS + 5,
            ScriptedError::Kind(std::io::ErrorKind::NotFound),
        );

        let result = rename_with_retry_impl(Path::new("from"), Path::new("to"), rename_fn, |_| {});

        let err = result.expect_err("must propagate a non-retried error");
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
        assert_eq!(
            *calls.borrow(),
            1,
            "a non-retried error kind must return immediately after exactly 1 attempt"
        );
    }

    /// Finding 1: the sleep durations passed to the injected `sleep_fn` must
    /// follow the documented backoff exactly: `20ms * (1 << attempt)` for
    /// attempts 0..3, i.e. 20ms, 40ms, 80ms, 160ms between the 5 permitted
    /// attempts.
    #[test]
    fn test_rename_with_retry_impl_backoff_sequence_matches_documented_formula() {
        let (rename_fn, _calls) = scripted_rename(
            RENAME_RETRY_MAX_ATTEMPTS - 1,
            ScriptedError::Kind(std::io::ErrorKind::PermissionDenied),
        );
        let mut sleeps = Vec::new();

        let result = rename_with_retry_impl(Path::new("from"), Path::new("to"), rename_fn, |d| {
            sleeps.push(d)
        });

        assert!(result.is_ok());
        assert_eq!(
            sleeps,
            vec![
                Duration::from_millis(20),
                Duration::from_millis(40),
                Duration::from_millis(80),
                Duration::from_millis(160),
            ],
            "backoff must follow 20ms * (1 << attempt) for attempts 0..3"
        );
    }

    /// Finding 3: on Windows, raw OS error 32 (`ERROR_SHARING_VIOLATION`) is
    /// the same class of transient AV/indexer lock contention as
    /// `PermissionDenied`, but Rust's std does not categorize it as
    /// `ErrorKind::PermissionDenied` — it must still be retried.
    /// Windows-only: `is_retryable_rename_error`'s raw-os-error check is
    /// itself `#[cfg(windows)]`-gated (raw_os_error is not a meaningful,
    /// portable check off Windows — Finding 3's own rationale), so this
    /// assertion only holds on that platform.
    #[cfg(windows)]
    #[test]
    fn test_rename_with_retry_impl_retries_raw_os_error_32_then_succeeds() {
        let (rename_fn, calls) = scripted_rename(2, ScriptedError::Raw(32));

        let result = rename_with_retry_impl(Path::new("from"), Path::new("to"), rename_fn, |_| {});

        assert!(
            result.is_ok(),
            "raw OS error 32 must be retried: {result:?}"
        );
        assert_eq!(*calls.borrow(), 3);
    }

    /// Finding 3: raw OS error 33 (`ERROR_LOCK_VIOLATION`) is likewise
    /// retried, and exhausts the same bounded retry budget as any other
    /// retryable condition when it never clears. Windows-only, see above.
    #[cfg(windows)]
    #[test]
    fn test_rename_with_retry_impl_exhausts_retries_on_raw_os_error_33() {
        let (rename_fn, calls) =
            scripted_rename(RENAME_RETRY_MAX_ATTEMPTS + 5, ScriptedError::Raw(33));

        let result = rename_with_retry_impl(Path::new("from"), Path::new("to"), rename_fn, |_| {});

        assert!(result.is_err(), "must propagate once retries are exhausted");
        assert_eq!(*calls.borrow(), RENAME_RETRY_MAX_ATTEMPTS);
    }

    /// Finding 3: a raw OS error that is NOT 32 or 33 (and not
    /// `PermissionDenied`-kind) must NOT be retried — the widened predicate
    /// is specific to the two documented lock-contention codes, not "any
    /// raw OS error." This holds on every platform: off Windows, the
    /// raw-os-error branch of `is_retryable_rename_error` never applies at
    /// all (Finding 4), so a non-32/33 raw code is unretried everywhere.
    #[test]
    fn test_rename_with_retry_impl_does_not_retry_unrelated_raw_os_error() {
        let (rename_fn, calls) = scripted_rename(
            RENAME_RETRY_MAX_ATTEMPTS + 5,
            ScriptedError::Raw(2), // ERROR_FILE_NOT_FOUND — unrelated code
        );

        let result = rename_with_retry_impl(Path::new("from"), Path::new("to"), rename_fn, |_| {});

        assert!(result.is_err());
        assert_eq!(
            *calls.borrow(),
            1,
            "an unrelated raw OS error must not be retried"
        );
    }

    // -----------------------------------------------------------------
    // S-25.08 PR #848 round-2 B-1 (TD-VSDD-059): the SEC-002 integration
    // tests plant a symlink at the OLD predictable temp name, which the fix
    // no longer uses, so they pass even if `create_new(true)` is reverted.
    // These unit tests drive the crate-private suffix-injection seam
    // `create_exclusive_temp_with(parent, basename, kind, suffix_fn)` so a
    // collision is planted at the EXACT candidate name the function will try
    // first, exercising the `AlreadyExists` retry branch and the bounded
    // fail-closed exit. They live in this src-level module (not an
    // integration file) because the seam is crate-private.
    // -----------------------------------------------------------------

    /// The exact candidate name `create_exclusive_temp_with` must build for
    /// a given suffix (documented format `.<basename>.<kind>-<pid>-<suffix:016x>`).
    fn candidate_for(parent: &Path, basename: &str, kind: &str, suffix: u64) -> PathBuf {
        parent.join(format!(
            ".{basename}.{kind}-{}-{suffix:016x}",
            std::process::id()
        ))
    }

    /// A deterministic suffix source yielding `values` in order, then
    /// panicking if over-consumed; the shared counter records how many
    /// suffixes were requested (== attempts made).
    fn scripted_suffixes(values: Vec<u64>) -> (impl FnMut() -> u64, Rc<RefCell<usize>>) {
        let calls = Rc::new(RefCell::new(0usize));
        let calls_in = Rc::clone(&calls);
        let f = move || {
            let mut n = calls_in.borrow_mut();
            let v = *values
                .get(*n)
                .unwrap_or_else(|| panic!("suffix_fn over-consumed: asked for call #{}", *n + 1));
            *n += 1;
            v
        };
        (f, calls)
    }

    const VICTIM_ORIGINAL: &str = "VICTIM-ORIGINAL-CONTENT\n";

    /// Shared body for (a): plant a symlink at the FIRST candidate; the
    /// second candidate must be used and the victim left untouched.
    #[cfg(unix)]
    fn assert_first_candidate_symlink_retries_to_second(dangling: bool) {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let victim_dir = tempfile::tempdir().unwrap();
        let victim = victim_dir.path().join("victim.txt");
        if !dangling {
            std::fs::write(&victim, VICTIM_ORIGINAL).unwrap();
        }
        let first = candidate_for(dir.path(), "reservation.json", "tmp", 1);
        let second = candidate_for(dir.path(), "reservation.json", "tmp", 2);
        symlink(&victim, &first).unwrap();

        let (suffix_fn, calls) = scripted_suffixes(vec![1, 2]);
        let (mut file, path) =
            create_exclusive_temp_with(dir.path(), "reservation.json", "tmp", suffix_fn)
                .expect("a symlink at the first candidate must be refused and the retry succeed");
        file.write_all(b"NEW-CONTENT\n").unwrap();
        drop(file);

        assert_eq!(
            path, second,
            "retry must proceed to the second candidate name"
        );
        assert_eq!(*calls.borrow(), 2, "exactly two suffixes consumed");
        assert_eq!(std::fs::read_to_string(&second).unwrap(), "NEW-CONTENT\n");
        assert!(
            std::fs::symlink_metadata(&first)
                .unwrap()
                .file_type()
                .is_symlink(),
            "the planted symlink must be left in place, not replaced or removed"
        );
        if dangling {
            assert!(
                !victim.exists(),
                "create_new must not create the victim through a dangling symlink (CWE-59)"
            );
        } else {
            assert_eq!(
                std::fs::read_to_string(&victim).unwrap(),
                VICTIM_ORIGINAL,
                "create_new must not follow the symlink and truncate the victim (CWE-59/CWE-377)"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_S_25_08_SEC002_create_exclusive_temp_retries_past_symlink_to_existing_victim() {
        assert_first_candidate_symlink_retries_to_second(false);
    }

    #[cfg(unix)]
    #[test]
    fn test_S_25_08_SEC002_create_exclusive_temp_retries_past_dangling_symlink() {
        assert_first_candidate_symlink_retries_to_second(true);
    }

    /// (b): every one of the 16 candidates collides (a symlink planted at
    /// each) -> typed `AlreadyExists`, exactly 16 attempts, nothing written
    /// through any symlink, the target untouched, no new files created.
    #[cfg(unix)]
    #[test]
    fn test_S_25_08_SEC002_create_exclusive_temp_all_16_candidates_collide_fails_closed() {
        use std::os::unix::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        let victim_dir = tempfile::tempdir().unwrap();
        let victim = victim_dir.path().join("victim.txt");
        std::fs::write(&victim, VICTIM_ORIGINAL).unwrap();
        let target = dir.path().join("reservation.json");
        std::fs::write(&target, "TARGET-ORIGINAL\n").unwrap();

        assert_eq!(TEMP_CREATE_MAX_ATTEMPTS, 16, "documented attempt bound");
        let suffixes: Vec<u64> = (100..100 + u64::from(TEMP_CREATE_MAX_ATTEMPTS)).collect();
        for s in &suffixes {
            symlink(
                &victim,
                candidate_for(dir.path(), "reservation.json", "tmp", *s),
            )
            .unwrap();
        }
        let entries_before = std::fs::read_dir(dir.path()).unwrap().count();

        let (suffix_fn, calls) = scripted_suffixes(suffixes);
        let err = create_exclusive_temp_with(dir.path(), "reservation.json", "tmp", suffix_fn)
            .expect_err("all candidates colliding must fail closed");

        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(
            *calls.borrow(),
            16,
            "must try exactly 16 candidates, then stop"
        );
        assert_eq!(std::fs::read_to_string(&victim).unwrap(), VICTIM_ORIGINAL);
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "TARGET-ORIGINAL\n"
        );
        assert_eq!(
            std::fs::read_dir(dir.path()).unwrap().count(),
            entries_before,
            "fail-closed must create nothing"
        );
    }

    /// (b) cross-platform: all 16 candidates collide via a pre-created
    /// regular file at each name (no symlink needed) -> AlreadyExists after
    /// exactly 16 attempts; the pre-existing files keep their content.
    #[test]
    fn test_S_25_08_SEC002_create_exclusive_temp_all_16_regular_file_collisions_fail_closed() {
        let dir = tempfile::tempdir().unwrap();
        let suffixes: Vec<u64> = (200..216).collect();
        for s in &suffixes {
            std::fs::write(
                candidate_for(dir.path(), "t.md", "strict-tmp", *s),
                "OCCUPIED\n",
            )
            .unwrap();
        }
        let (suffix_fn, calls) = scripted_suffixes(suffixes.clone());
        let err = create_exclusive_temp_with(dir.path(), "t.md", "strict-tmp", suffix_fn)
            .expect_err("must fail closed");
        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(*calls.borrow(), 16);
        for s in suffixes {
            assert_eq!(
                std::fs::read_to_string(candidate_for(dir.path(), "t.md", "strict-tmp", s))
                    .unwrap(),
                "OCCUPIED\n",
                "an occupied candidate must never be truncated"
            );
        }
    }

    /// N-1 (cross-platform replacement for the unix-only, root-skipping
    /// permission test): force a creation failure by pre-creating a
    /// DIRECTORY at every injected candidate path. `create_new` refuses a
    /// directory on every platform and as any uid (including root), so the
    /// failure is deterministic. The target must be untouched.
    #[test]
    fn test_S_25_08_SEC002_create_exclusive_temp_directory_at_candidate_fails_and_target_untouched()
    {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.md");
        std::fs::write(&target, "original\n").unwrap();
        // Same suffix every call -> the one (directory) path is hit 16 times.
        std::fs::create_dir(candidate_for(dir.path(), "target.md", "tmp", 7)).unwrap();

        let (suffix_fn, calls) = scripted_suffixes(vec![7; 16]);
        let result = create_exclusive_temp_with(dir.path(), "target.md", "tmp", suffix_fn);

        let err = result.expect_err("a directory at the candidate path must force a failure");
        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(*calls.borrow(), 16);
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "original\n");
    }

    /// A non-`AlreadyExists` creation error is NOT retried: it is returned
    /// after the first attempt (a missing parent directory yields NotFound
    /// on every platform).
    #[test]
    fn test_S_25_08_SEC002_create_exclusive_temp_non_collision_error_is_not_retried() {
        let dir = tempfile::tempdir().unwrap();
        let missing_parent = dir.path().join("no-such-dir");
        let (suffix_fn, calls) = scripted_suffixes(vec![1, 2, 3]);
        let err = create_exclusive_temp_with(&missing_parent, "t.md", "tmp", suffix_fn)
            .expect_err("missing parent must fail");
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
        assert_eq!(*calls.borrow(), 1, "only AlreadyExists is retried");
    }

    /// Positive control: with no collision the first candidate is used and
    /// has the documented `.<basename>.<kind>-<pid>-<suffix:016x>` shape.
    #[test]
    fn test_S_25_08_SEC002_create_exclusive_temp_first_candidate_name_format() {
        let dir = tempfile::tempdir().unwrap();
        let (suffix_fn, calls) = scripted_suffixes(vec![0xABCD]);
        let (_file, path) = create_exclusive_temp_with(dir.path(), "x.json", "tmp", suffix_fn)
            .expect("clean directory must create on first attempt");
        assert_eq!(path, candidate_for(dir.path(), "x.json", "tmp", 0xABCD));
        assert_eq!(*calls.borrow(), 1);
    }

    /// SEC-002-N3 / NB-7: the temp file is born `0o600`, never wider.
    #[cfg(unix)]
    #[test]
    fn test_S_25_08_SEC002_N3_create_exclusive_temp_is_owner_only_0o600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let (file, path) = create_exclusive_temp(dir.path(), "t.md", "tmp").unwrap();
        drop(file);
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "temp file must be created owner-only");
    }

    /// SEC-002-N3 follow-up: a NEW file keeps the platform default creation
    /// mode (what `File::create` yields); only the temp is owner-only.
    #[cfg(unix)]
    #[test]
    fn test_S_25_08_SEC002_N3_new_file_via_write_atomic_has_default_creation_mode() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let probe = dir.path().join("probe");
        File::create(&probe).unwrap();
        let expected = std::fs::metadata(&probe).unwrap().permissions().mode() & 0o777;

        let target = dir.path().join("new.md");
        write_atomic(&target, "x\n").unwrap();
        let mode = std::fs::metadata(&target).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode, expected,
            "new file must have the default creation mode"
        );

        let strict = dir.path().join("new-strict.md");
        write_atomic_strict_durable(&strict, "x\n").unwrap();
        let mode = std::fs::metadata(&strict).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode, expected,
            "strict: new file must have the default creation mode"
        );
    }

    /// CR-R2-003 (PR #848 round 3): an OVERWRITE keeps the target's existing
    /// mode through `write_atomic` (the temp is born 0o600; the pre-existing
    /// 0o640 must survive the rename). The earlier tests only covered the
    /// new-file case.
    #[cfg(unix)]
    #[test]
    fn test_S_25_08_CR_R2_003_write_atomic_overwrite_preserves_existing_mode_0o640() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("existing.md");
        std::fs::write(&target, "old\n").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o640)).unwrap();

        write_atomic(&target, "new\n").unwrap();

        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new\n");
        let mode = std::fs::metadata(&target).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o640,
            "write_atomic overwrite must preserve the existing 0o640 mode"
        );
    }

    /// CR-R2-003: same overwrite contract for `write_atomic_strict_durable`.
    #[cfg(unix)]
    #[test]
    fn test_S_25_08_CR_R2_003_write_atomic_strict_durable_overwrite_preserves_existing_mode_0o640()
    {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("existing-strict.md");
        std::fs::write(&target, "old\n").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o640)).unwrap();

        write_atomic_strict_durable(&target, "new\n").unwrap();

        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new\n");
        let mode = std::fs::metadata(&target).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o640,
            "write_atomic_strict_durable overwrite must preserve the existing 0o640 mode"
        );
    }

    // -----------------------------------------------------------------------
    // S-25.08 PR #848 round-3 hardening tests (R3-B1, NB-B, SEC-R3-001/002/004).
    // -----------------------------------------------------------------------

    /// `Debug` rendering of any value; lets a test inspect a result whose type
    /// changes from `()` to `io::Result<()>` without a compile break.
    #[cfg(unix)]
    fn debug_of<T: std::fmt::Debug>(value: T) -> String {
        format!("{value:?}")
    }

    /// `File::create` probe mode in `dir` (umask-aware default).
    #[cfg(unix)]
    fn file_create_probe_mode(dir: &Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        let probe = dir.join("mode-probe");
        let f = File::create(&probe).unwrap();
        let mode = f.metadata().unwrap().permissions().mode() & 0o777;
        drop(f);
        std::fs::remove_file(&probe).unwrap();
        mode
    }

    /// R3-B1(a): a temp for an EXISTING target is born `0o600`, observed on the
    /// handle BEFORE any `fchmod` (`apply_target_permissions`). Kills mutation
    /// M4 (existing-target branch uses the default creation mode).
    #[cfg(unix)]
    #[test]
    fn test_S_25_08_R3_B1_temp_for_existing_target_is_born_0o600_before_any_fchmod() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("t.md");
        std::fs::write(&target, "old\n").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644)).unwrap();

        let (file, tmp) =
            create_exclusive_temp_for_target_with(dir.path(), "t.md", "tmp", &target, || 1)
                .unwrap();
        let mode = file.metadata().unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "temp for an existing 0o644 target must be born owner-only (0o600), pre-fchmod"
        );
        assert!(tmp.exists());
    }

    /// R3-B1(b): a temp for an ABSENT target is born with `File::create`
    /// semantics (umask default), equal to a same-dir probe.
    #[cfg(unix)]
    #[test]
    fn test_S_25_08_R3_B1_temp_for_absent_target_is_born_with_file_create_default_mode() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("absent.md");
        let expected = file_create_probe_mode(dir.path());

        let (file, _tmp) =
            create_exclusive_temp_for_target_with(dir.path(), "absent.md", "tmp", &target, || 2)
                .unwrap();
        let mode = file.metadata().unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode, expected,
            "temp for an absent target must match the File::create probe mode"
        );
    }

    /// NB-B: `apply_target_permissions` must SURFACE a failed fchmod instead of
    /// swallowing it. Failure injection: a read-only handle to `/dev/null`
    /// (owned by root, so fchmod by an unprivileged user fails EPERM). The
    /// result is inspected through its `Debug` form so this compiles against
    /// both the current `()` return (red: prints `()`) and the required
    /// `io::Result<()>` (green: prints `Err(..)`). Skips only when running as
    /// a user permitted to chmod `/dev/null` (root).
    #[cfg(unix)]
    #[test]
    #[allow(clippy::unit_arg)] // `()` today, `io::Result<()>` after NB-B
    fn test_S_25_08_R3_NB_B_apply_target_permissions_surfaces_fchmod_error() {
        use std::os::unix::fs::MetadataExt;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("t.md");
        std::fs::write(&target, "x\n").unwrap();
        let dev_null = File::open("/dev/null").unwrap();
        let me = std::fs::metadata(dir.path()).unwrap().uid();
        let owner = dev_null.metadata().unwrap().uid();
        assert!(
            me != 0 && owner != me,
            "precondition: fchmod on /dev/null must be refused (uid {me}, /dev/null owner {owner})"
        );

        let shown = debug_of(apply_target_permissions(&dev_null, &target));
        assert!(
            shown.starts_with("Err("),
            "a failed fchmod must be returned as Err, got {shown}"
        );
    }

    /// SEC-R3-002: setuid is NOT inherited from the target; final mode is the
    /// permission bits only.
    #[cfg(unix)]
    #[test]
    fn test_S_25_08_SEC_R3_002_write_atomic_masks_setuid_from_target_mode() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("suid.md");
        std::fs::write(&target, "old\n").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o4755)).unwrap();
        let pre = std::fs::metadata(&target).unwrap().permissions().mode() & 0o7777;
        assert_eq!(pre, 0o4755, "precondition: setuid set on own file");

        write_atomic(&target, "new\n").unwrap();
        let mode = std::fs::metadata(&target).unwrap().permissions().mode() & 0o7777;
        assert_eq!(mode, 0o755, "special bits must be masked, got {mode:o}");
    }

    /// SEC-R3-002 (strict-durable writer): same masking.
    #[cfg(unix)]
    #[test]
    fn test_S_25_08_SEC_R3_002_write_atomic_strict_durable_masks_setuid_from_target_mode() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("suid-strict.md");
        std::fs::write(&target, "old\n").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o4755)).unwrap();

        write_atomic_strict_durable(&target, "new\n").unwrap();
        let mode = std::fs::metadata(&target).unwrap().permissions().mode() & 0o7777;
        assert_eq!(mode, 0o755, "special bits must be masked, got {mode:o}");
    }

    /// SEC-R3-002: setgid/sticky are masked too where the platform lets an
    /// unprivileged owner set them (cases the OS refuses are skipped; setuid
    /// above is the mandatory case).
    #[cfg(unix)]
    #[test]
    fn test_S_25_08_SEC_R3_002_write_atomic_masks_setgid_and_sticky_where_settable() {
        use std::os::unix::fs::PermissionsExt;
        let mut exercised = 0;
        for (special, expected) in [(0o2755u32, 0o755u32), (0o1777, 0o777)] {
            let dir = tempfile::tempdir().unwrap();
            let target = dir.path().join("sp.md");
            std::fs::write(&target, "old\n").unwrap();
            let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(special));
            let got = std::fs::metadata(&target).unwrap().permissions().mode() & 0o7777;
            if got != special {
                continue; // platform refused the special bit for this user
            }
            exercised += 1;
            write_atomic(&target, "new\n").unwrap();
            let mode = std::fs::metadata(&target).unwrap().permissions().mode() & 0o7777;
            assert_eq!(mode, expected, "{special:o} must mask to {expected:o}");
        }
        // setuid is covered by the mandatory tests; do not pass vacuously on
        // a platform where neither optional bit is settable without saying so.
        eprintln!("setgid/sticky cases exercised: {exercised}");
    }

    /// SEC-R3-001: the target vanished after the exists-check (temp born
    /// `0o600`): `apply_target_permissions` must fall back to the `File::create`
    /// umask default, not leave the final file stuck at `0o600`.
    #[cfg(unix)]
    #[test]
    #[allow(clippy::unit_arg)] // `()` today, `io::Result<()>` after NB-B
    fn test_S_25_08_SEC_R3_001_vanished_target_falls_back_to_umask_default_mode() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let expected = file_create_probe_mode(dir.path());
        assert_ne!(
            expected, 0o600,
            "precondition: umask default must differ from 0o600 for this test to discriminate"
        );
        let (file, _tmp) = create_exclusive_temp(dir.path(), "v.md", "tmp").unwrap();
        assert_eq!(
            file.metadata().unwrap().permissions().mode() & 0o777,
            0o600,
            "precondition: temp born 0o600"
        );
        let gone = dir.path().join("vanished-target.md");

        // Return type is `()` today, `io::Result<()>` after NB-B; only the
        // resulting handle mode is asserted here.
        let _ = debug_of(apply_target_permissions(&file, &gone));
        let mode = file.metadata().unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode, expected,
            "vanished target must fall back to the umask default ({expected:o}), got {mode:o}"
        );
    }

    /// SEC-R3-004: public `create_exclusive_temp*` reject an unvalidated
    /// basename/kind (separator, `..`, NUL, empty) with `InvalidInput` and
    /// create nothing.
    #[test]
    fn test_S_25_08_SEC_R3_004_create_exclusive_temp_rejects_unsafe_basename_and_kind() {
        let mut bad: Vec<&str> = vec!["a/b", "/abs", "..", "", "a\0b"];
        if cfg!(windows) {
            bad.push("a\\b");
        }
        for value in bad {
            for (basename, kind) in [(value, "tmp"), ("ok.md", value)] {
                let dir = tempfile::tempdir().unwrap();
                let target = dir.path().join("t.md");
                let results = [
                    (
                        "create_exclusive_temp",
                        create_exclusive_temp(dir.path(), basename, kind),
                    ),
                    (
                        "create_exclusive_temp_with",
                        create_exclusive_temp_with(dir.path(), basename, kind, || 7),
                    ),
                    (
                        "create_exclusive_temp_for_target",
                        create_exclusive_temp_for_target(dir.path(), basename, kind, &target),
                    ),
                    (
                        "create_exclusive_temp_for_target_with",
                        create_exclusive_temp_for_target_with(
                            dir.path(),
                            basename,
                            kind,
                            &target,
                            || 7,
                        ),
                    ),
                ];
                for (name, res) in results {
                    match res {
                        Ok((_, p)) => {
                            panic!("{name}({basename:?}, {kind:?}) must be rejected, created {p:?}")
                        }
                        Err(e) => assert_eq!(
                            e.kind(),
                            std::io::ErrorKind::InvalidInput,
                            "{name}({basename:?}, {kind:?}) must fail InvalidInput, got {e:?}"
                        ),
                    }
                }
                let leftovers: Vec<_> = std::fs::read_dir(dir.path())
                    .unwrap()
                    .map(|e| e.unwrap().path())
                    .collect();
                assert!(
                    leftovers.is_empty(),
                    "rejected ({basename:?}, {kind:?}) must create nothing, found {leftovers:?}"
                );
            }
        }
    }
}
