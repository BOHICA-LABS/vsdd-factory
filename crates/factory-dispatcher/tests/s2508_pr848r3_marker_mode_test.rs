// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-25.08 PR #848 round-3 BLOCKING-1 (TD-VSDD-059 paper fix).
//!
//! Pins the final permissions of the marker written by
//! `write_indeterminate_marker` on the SUCCESS path:
//!
//! - a NEW marker gets the platform default creation mode a plain
//!   `File::create` would give (not the exclusive temp's `0o600` birth mode);
//! - an OVERWRITTEN marker keeps the mode it had before.
//!
//! The writer picks the temp's creation mode from the target (existing mode
//! when the target exists, default creation mode otherwise) and applies the
//! target permissions on the open fd before the rename. Both tests drive the
//! real public writer.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;

use factory_dispatcher::indeterminate_marker::{MarkerFields, write_indeterminate_marker};

fn fields() -> MarkerFields {
    MarkerFields {
        timestamp: "2026-10-14T00:00:00Z".to_string(),
        plugin_name: "mode-plugin".to_string(),
        artifact_path: "a.md".to_string(),
        cause: "fuel".to_string(),
        trace_id: "trace-mode".to_string(),
        expires_at: "2026-10-15T00:00:00Z".to_string(),
    }
}

fn mode_of(p: &std::path::Path) -> u32 {
    std::fs::metadata(p).expect("metadata").permissions().mode() & 0o777
}

/// (a) A NEW marker gets the default creation mode (probed with `File::create`
/// in the same directory), not the temp's `0o600`.
#[test]
fn test_S_25_08_PR848R3_B1_new_marker_has_default_creation_mode_not_0o600() {
    let dir = tempfile::tempdir().expect("tempdir");
    let probe = dir.path().join("mode-probe");
    std::fs::File::create(&probe).expect("probe");
    let expected = mode_of(&probe);

    let marker = dir.path().join("unvalidated-mutation.marker");
    write_indeterminate_marker(&fields(), &marker).expect("write marker");

    assert_eq!(
        mode_of(&marker),
        expected,
        "a new marker must land with the File::create default mode ({expected:o}), \
         not the exclusive temp's 0o600 birth mode"
    );
}

/// (b) An OVERWRITTEN marker keeps its previous mode (last-writer-wins must not
/// silently change permissions).
#[test]
fn test_S_25_08_PR848R3_B1_overwritten_marker_keeps_previous_mode_0o640() {
    let dir = tempfile::tempdir().expect("tempdir");
    let marker = dir.path().join("unvalidated-mutation.marker");
    std::fs::write(&marker, "stale = true\n").expect("pre-create");
    std::fs::set_permissions(&marker, std::fs::Permissions::from_mode(0o640)).expect("chmod");

    write_indeterminate_marker(&fields(), &marker).expect("overwrite marker");

    let body = std::fs::read_to_string(&marker).expect("read marker");
    assert!(
        body.contains("mode-plugin"),
        "overwrite must have landed: {body}"
    );
    assert_eq!(
        mode_of(&marker),
        0o640,
        "an overwritten marker must keep its previous 0o640 mode"
    );
}
