//! F-S2508-L4-001 (LOCAL adversary pass 4, MEDIUM) -- the release leg's
//! `.factory/migration-state` presence probe must not collapse a stat ERROR
//! into "absent".
//!
//! BC-1.18.013 EC-021 / AC-011: the writer-reservation release is best-effort;
//! a NON-ENOENT release error is a NON-FATAL `tracing::warn!` (never a verdict).
//! `bc_index_migration_reservation_release` currently guards with
//! `if !migration_state_dir.exists() { return; }`; `Path::exists()` is false on
//! EACCES / EIO / ELOOP too, so a stat error is silently treated as the
//! no-op ENOENT path (the sibling of F-S2508-L3-009's `.factory` fix).
//!
//! OBSERVABILITY: S-25.08 has no event channel (S-25.09 adds the
//! `migration.release_failed` advisory), and `reservation_release` returns `()`
//! and only logs through `tracing`. The warn-vs-silent distinction is NOT
//! observable through the real binary's exit code or filesystem effects -- the
//! black-box vector (`s2508_admission_blackbox_test.rs`, ELOOP `migration-state`
//! symlink) pins only that externally visible half (exit 0, nothing deleted),
//! which passes today. It IS observable in-process: `s2508_diagnostics_test.rs`
//! (F-S2508-L5-001) installs a capturing `tracing` subscriber around the `pub`
//! `bc_index_migration_reservation_release` and asserts exactly one WARN
//! (`error_kind` != NotFound, `path` naming migration-state) for a stat error and
//! none for ENOENT.
//! The error-path classification is pinned HERE against the factored helper the
//! implementer must introduce in `executor.rs`:
//!
//! ```ignore
//! /// Ok(true)  -- stat succeeded (the release proceeds);
//! /// Ok(false) -- ENOENT: migration-state/ genuinely absent (silent no-op);
//! /// Err(e)    -- any OTHER stat error (EACCES/EIO/ELOOP/...): the caller emits
//! ///              the non-fatal tracing::warn! (EC-021) instead of returning silently.
//! pub fn probe_release_migration_state_dir(
//!     migration_state_dir: &std::path::Path,
//! ) -> Result<bool, std::io::Error>;
//! ```
//!
//! and `bc_index_migration_reservation_release` must branch on it
//! (`Ok(false)` => return; `Err(e)` => `tracing::warn!` + return).
//! RED at authoring time = compile failure (the helper does not exist yet).

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use factory_dispatcher::executor::probe_release_migration_state_dir;

#[test]
fn test_BC_1_18_013_L4_001_probe_absent_dir_is_ok_false() {
    let dir = tempfile::tempdir().unwrap();
    let r = probe_release_migration_state_dir(&dir.path().join(".factory/migration-state"));
    assert!(
        matches!(r, Ok(false)),
        "ENOENT must be Ok(false) (genuinely absent => silent no-op), got {r:?}"
    );
}

#[test]
fn test_BC_1_18_013_L4_001_probe_present_dir_is_ok_true() {
    let dir = tempfile::tempdir().unwrap();
    let ms = dir.path().join("migration-state");
    std::fs::create_dir_all(&ms).unwrap();
    let r = probe_release_migration_state_dir(&ms);
    assert!(
        matches!(r, Ok(true)),
        "present dir must be Ok(true), got {r:?}"
    );
}

#[cfg(unix)]
#[test]
fn test_BC_1_18_013_L4_001_probe_eloop_is_err_not_absent() {
    // Self-referential symlink: stat => ELOOP for EVERY uid (root-safe).
    let dir = tempfile::tempdir().unwrap();
    let ms = dir.path().join("migration-state");
    std::os::unix::fs::symlink("migration-state", &ms).unwrap();
    let r = probe_release_migration_state_dir(&ms);
    match r {
        Err(e) => assert_ne!(
            e.kind(),
            std::io::ErrorKind::NotFound,
            "an ELOOP stat error must surface as a non-NotFound Err, got {e:?}"
        ),
        other => panic!(
            "a stat error (ELOOP) must be Err(..) so the release emits the non-fatal \
             warn (EC-021) instead of a silent no-op; got {other:?}"
        ),
    }
}

#[cfg(unix)]
#[test]
fn test_BC_1_18_013_L4_001_probe_eacces_is_err_not_absent() {
    use std::os::unix::fs::PermissionsExt;
    // Root bypasses permission checks; ELOOP covers the root case above.
    if unsafe { libc_geteuid() } == 0 {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path().join("factory");
    std::fs::create_dir_all(parent.join("migration-state")).unwrap();
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o000)).unwrap();
    let r = probe_release_migration_state_dir(&parent.join("migration-state"));
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        matches!(&r, Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied),
        "EACCES must be Err(PermissionDenied), never Ok(false); got {r:?}"
    );
}

#[cfg(unix)]
unsafe extern "C" {
    #[link_name = "geteuid"]
    fn libc_geteuid() -> u32;
}
