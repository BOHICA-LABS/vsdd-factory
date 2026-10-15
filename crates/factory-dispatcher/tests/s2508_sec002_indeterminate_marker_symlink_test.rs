// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-25.08 PR #848 round-2 SEC-002-N4 (CWE-59 link following / CWE-377
//! insecure temp file; TD-VSDD-060 sibling of the `atomic_write` SEC-002 fix).
//!
//! `indeterminate_marker::write_indeterminate_marker` staged its marker at
//! `<marker>.<plugin>.<pid>.<nonce>.tmp` using `std::fs::write`, which
//! follows symlinks and truncates. The name is predictable: `pid` is
//! observable and `nonce` is a process-wide counter that starts at 0. A
//! same-uid attacker who plants a symlink at that name made the writer
//! truncate and overwrite the symlink's victim, then rename the symlink
//! itself into place as the marker.
//!
//! This file deliberately holds a SINGLE `#[test]` so the process-wide nonce
//! counter is deterministic (first call uses nonce 0, second nonce 1): the
//! test plants symlinks at the candidate names for nonces 0..16 (even nonces
//! to an existing victim, odd nonces dangling), then drives the production
//! writer through its public entry point twice. Required behavior: the victim
//! is NEVER modified, a dangling victim is NEVER created, and the marker, if
//! the write succeeds, is a regular file (never the planted symlink). Whether
//! the fixed writer retries past the collision or fails closed is
//! implementation-defined; both leave the victim untouched.

#![cfg(unix)]

use std::os::unix::fs::symlink;

use factory_dispatcher::indeterminate_marker::{MarkerFields, write_indeterminate_marker};

const VICTIM_ORIGINAL: &str = "VICTIM-ORIGINAL-CONTENT\n";

fn fields() -> MarkerFields {
    MarkerFields {
        timestamp: "2026-10-14T00:00:00Z".to_string(),
        plugin_name: "sec002-plugin".to_string(),
        artifact_path: "a.md".to_string(),
        cause: "fuel".to_string(),
        trace_id: "trace-1".to_string(),
        expires_at: "2026-10-15T00:00:00Z".to_string(),
    }
}

#[test]
fn test_S_25_08_SEC002_N4_write_indeterminate_marker_does_not_follow_symlink_at_predictable_temp_name()
 {
    let dir = tempfile::tempdir().expect("tempdir");
    let victim_dir = tempfile::tempdir().expect("victim tempdir");
    let existing_victim = victim_dir.path().join("existing-victim.txt");
    let dangling_victim = victim_dir.path().join("dangling-victim.txt");
    std::fs::write(&existing_victim, VICTIM_ORIGINAL).expect("seed victim");

    let marker = dir.path().join("unvalidated-mutation.marker");
    let pid = std::process::id();
    // Legacy predictable name: `<file_name>.<safe_plugin>.<pid>.<nonce>.tmp`.
    for nonce in 0..16u64 {
        let planted = dir.path().join(format!(
            "unvalidated-mutation.marker.sec002-plugin.{pid}.{nonce}.tmp"
        ));
        let victim = if nonce % 2 == 0 {
            &existing_victim
        } else {
            &dangling_victim
        };
        symlink(victim, &planted).expect("plant symlink at predictable temp name");
    }

    // Call 1 (legacy nonce 0 -> existing victim) and call 2 (nonce 1 -> dangling).
    let results = [
        write_indeterminate_marker(&fields(), &marker),
        write_indeterminate_marker(&fields(), &marker),
    ];

    assert_eq!(
        std::fs::read_to_string(&existing_victim).expect("read victim"),
        VICTIM_ORIGINAL,
        "write_indeterminate_marker followed a symlink at its predictable temp name and \
         overwrote the victim file (CWE-59/CWE-377)"
    );
    assert!(
        !dangling_victim.exists(),
        "write_indeterminate_marker followed a dangling symlink at its predictable temp name \
         and CREATED the victim file (CWE-59)"
    );

    for result in &results {
        if result.is_ok() {
            let md = std::fs::symlink_metadata(&marker).expect("marker must exist after Ok");
            assert!(
                !md.file_type().is_symlink(),
                "the marker must be a regular file, not the planted symlink"
            );
            let body = std::fs::read_to_string(&marker).expect("read marker");
            assert!(
                body.contains("plugin_name") && body.contains("sec002-plugin"),
                "a successful write must land the marker TOML, got: {body}"
            );
        }
    }
    if let Ok(md) = std::fs::symlink_metadata(&marker) {
        assert!(
            !md.file_type().is_symlink(),
            "the planted symlink must never be installed as the marker"
        );
    }
}
