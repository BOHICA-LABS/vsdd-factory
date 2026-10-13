// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-25.08 PR #848 SEC-002 (CWE-59 link following / CWE-377 insecure temp
//! file): `atomic_write::write_atomic` and `write_atomic_strict_durable`
//! created their sibling temp file with `File::create` at the PREDICTABLE name
//! `.<basename>.tmp-<pid>` / `.<basename>.strict-tmp-<pid>`. A symlink planted
//! at that name was followed: the write truncated and overwrote the symlink's
//! victim, and the final rename then installed the symlink itself as the
//! target. The reservation write in the dispatcher's admission path
//! (`create_writer_reservation`) goes through `write_atomic`, so this is the
//! shared choke point.
//!
//! Required behavior (fix: `create_new` / `O_EXCL` + unpredictable suffix):
//! with a symlink planted at the legacy predictable temp name, the victim file
//! is NEVER modified, and the call either succeeds through a different temp
//! name (target holds the new content as a regular file, not a symlink) or
//! fails closed leaving the target untouched.

#![cfg(unix)]

use std::os::unix::fs::symlink;
use std::path::Path;

use last_amended_migrate::atomic_write::{write_atomic, write_atomic_strict_durable};

const VICTIM_ORIGINAL: &str = "VICTIM-ORIGINAL-CONTENT\n";

fn assert_symlink_not_followed(
    label: &str,
    tmp_suffix: &str,
    dangling: bool,
    write: fn(&Path, &str) -> Result<(), last_amended_migrate::error::MigrateError>,
) {
    let dir = tempfile::tempdir().expect("tempdir");
    let victim_dir = tempfile::tempdir().expect("victim tempdir");
    let victim = victim_dir.path().join("victim.txt");
    if !dangling {
        std::fs::write(&victim, VICTIM_ORIGINAL).expect("seed victim");
    }

    let target = dir.path().join("reservation.json");
    let planted = dir.path().join(format!(
        ".reservation.json.{tmp_suffix}-{}",
        std::process::id()
    ));
    symlink(&victim, &planted).expect("plant symlink at predictable temp name");

    let result = write(&target, "ATTACKER-CONTROLLED-NEW-CONTENT\n");

    // The victim must be untouched (dangling: must not have been created).
    if dangling {
        assert!(
            !victim.exists(),
            "{label}: write followed a dangling symlink at the predictable temp name and \
             CREATED the victim file (CWE-59)"
        );
    } else {
        assert_eq!(
            std::fs::read_to_string(&victim).expect("read victim"),
            VICTIM_ORIGINAL,
            "{label}: write followed a symlink at the predictable temp name and overwrote \
             the victim file (CWE-59/CWE-377)"
        );
    }

    match result {
        Ok(()) => {
            let md = std::fs::symlink_metadata(&target).expect("target must exist after Ok");
            assert!(
                !md.file_type().is_symlink(),
                "{label}: target must be a regular file, not the planted symlink"
            );
            assert_eq!(
                std::fs::read_to_string(&target).expect("read target"),
                "ATTACKER-CONTROLLED-NEW-CONTENT\n",
                "{label}: a successful write must land the new content via a different temp name"
            );
        }
        Err(_) => {
            // Fail-closed: the target must not have been installed as a symlink.
            if let Ok(md) = std::fs::symlink_metadata(&target) {
                assert!(
                    !md.file_type().is_symlink(),
                    "{label}: fail-closed path must not leave the planted symlink as the target"
                );
            }
        }
    }
}

#[test]
fn test_S_25_08_SEC002_write_atomic_does_not_follow_symlink_at_predictable_temp_name() {
    assert_symlink_not_followed("write_atomic/existing-victim", "tmp", false, write_atomic);
}

#[test]
fn test_S_25_08_SEC002_write_atomic_does_not_create_through_dangling_symlink_at_temp_name() {
    assert_symlink_not_followed("write_atomic/dangling", "tmp", true, write_atomic);
}

#[test]
fn test_S_25_08_SEC002_write_atomic_strict_durable_does_not_follow_symlink_at_predictable_temp_name()
 {
    assert_symlink_not_followed(
        "write_atomic_strict_durable/existing-victim",
        "strict-tmp",
        false,
        write_atomic_strict_durable,
    );
}

#[test]
fn test_S_25_08_SEC002_write_atomic_strict_durable_does_not_create_through_dangling_symlink() {
    assert_symlink_not_followed(
        "write_atomic_strict_durable/dangling",
        "strict-tmp",
        true,
        write_atomic_strict_durable,
    );
}

/// Positive control: with no planted symlink both primitives still write the
/// target and leave no temp sibling (guards against a fix that fails closed
/// unconditionally). Passes before and after the fix by design.
#[test]
fn test_S_25_08_SEC002_control_clean_directory_writes_succeed_and_leave_no_temp() {
    for write in [write_atomic, write_atomic_strict_durable] {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("reservation.json");
        write(&target, "content\n").expect("clean write must succeed");
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "content\n");
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["reservation.json".to_string()], "no temp left");
    }
}
