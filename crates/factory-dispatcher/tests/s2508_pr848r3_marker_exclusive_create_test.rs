// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-25.08 PR #848 round-3 CR-R2-004 (CWE-59/CWE-377).
//!
//! The round-2 marker symlink test plants links only at the LEGACY temp names
//! (`<marker>.<plugin>.<pid>.<nonce>.tmp`), so it only guards against reverting
//! the naming scheme: a writer that kept the new unpredictable names but went
//! back to non-exclusive `std::fs::write` would still pass it. These tests plant
//! symlinks at the names the CURRENT naming produces
//! (`.<marker>.<plugin>.tmp-<pid>-<suffix:016x>`) via an injectable suffix
//! source, so they FAIL if the marker writer stops using exclusive create.
//!
//! TEST SEAMS EXERCISED (both exist; `#[doc(hidden)] pub`, no behaviour change):
//!   1. `last_amended_migrate::atomic_write::create_exclusive_temp_for_target_with`
//!      (suffix-injectable core the marker writer calls; the temp is created
//!      exclusively, `0o600` when the marker already exists, `File::create`
//!      semantics otherwise);
//!   2. `factory_dispatcher::indeterminate_marker::write_indeterminate_marker_with_suffix_fn<F: FnMut() -> u64>(
//!          fields: &MarkerFields, marker_path: &Path, suffix_fn: F) -> io::Result<()>`
//!      with `write_indeterminate_marker` delegating to it using the default
//!      (unpredictable) suffix source.

#![cfg(unix)]

use std::cell::Cell;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use factory_dispatcher::indeterminate_marker::{
    MarkerFields, write_indeterminate_marker_with_suffix_fn,
};

const VICTIM_ORIGINAL: &str = "VICTIM-ORIGINAL-CONTENT\n";

fn fields() -> MarkerFields {
    MarkerFields {
        timestamp: "2026-10-14T00:00:00Z".to_string(),
        plugin_name: "sec002-plugin".to_string(),
        artifact_path: "a.md".to_string(),
        cause: "fuel".to_string(),
        trace_id: "trace-excl".to_string(),
        expires_at: "2026-10-15T00:00:00Z".to_string(),
    }
}

/// The CURRENT temp name for `suffix`: `.<file>.<plugin>.tmp-<pid>-<suffix:016x>`.
fn current_candidate(dir: &Path, suffix: u64) -> PathBuf {
    dir.join(format!(
        ".unvalidated-mutation.marker.sec002-plugin.tmp-{}-{:016x}",
        std::process::id(),
        suffix
    ))
}

/// Symlink planted at the FIRST candidate name (existing victim or dangling):
/// exclusive create must refuse it, retry with the second suffix, leave the
/// victim untouched and the planted link in place.
fn run_first_candidate_case(dangling: bool) {
    let dir = tempfile::tempdir().expect("tempdir");
    let victim_dir = tempfile::tempdir().expect("victim dir");
    let victim = victim_dir.path().join("victim.txt");
    if !dangling {
        std::fs::write(&victim, VICTIM_ORIGINAL).expect("seed victim");
    }
    let marker = dir.path().join("unvalidated-mutation.marker");
    let first = current_candidate(dir.path(), 1);
    symlink(&victim, &first).expect("plant symlink at CURRENT first candidate");

    let calls = Cell::new(0u32);
    let suffixes = [1u64, 2u64];
    let result = write_indeterminate_marker_with_suffix_fn(&fields(), &marker, || {
        let i = calls.get() as usize;
        calls.set(calls.get() + 1);
        *suffixes.get(i).expect("suffix source over-consumed")
    });

    result.expect("retry past the planted symlink must succeed");
    assert_eq!(calls.get(), 2, "first candidate refused, second used");
    if dangling {
        assert!(
            !victim.exists(),
            "writer followed a dangling symlink at its CURRENT temp name and created the \
             victim (non-exclusive create, CWE-59)"
        );
    } else {
        assert_eq!(
            std::fs::read_to_string(&victim).expect("read victim"),
            VICTIM_ORIGINAL,
            "writer followed a symlink at its CURRENT temp name and overwrote the victim \
             (non-exclusive create, CWE-59/CWE-377)"
        );
    }
    assert!(
        std::fs::symlink_metadata(&first)
            .expect("planted link must remain")
            .file_type()
            .is_symlink(),
        "the planted symlink must be left in place, not consumed/renamed"
    );
    let md = std::fs::symlink_metadata(&marker).expect("marker exists");
    assert!(
        !md.file_type().is_symlink(),
        "marker must be a regular file"
    );
    assert!(
        std::fs::read_to_string(&marker)
            .expect("read marker")
            .contains("sec002-plugin")
    );
}

#[test]
fn test_S_25_08_PR848R3_CR_R2_004_marker_writer_refuses_symlink_at_current_temp_name_existing_victim()
 {
    run_first_candidate_case(false);
}

#[test]
fn test_S_25_08_PR848R3_CR_R2_004_marker_writer_refuses_symlink_at_current_temp_name_dangling_victim()
 {
    run_first_candidate_case(true);
}

/// All 16 candidates collide: fail closed (typed error), nothing written through
/// any link, no marker created.
#[test]
fn test_S_25_08_PR848R3_CR_R2_004_marker_writer_fails_closed_when_all_current_candidates_collide() {
    let dir = tempfile::tempdir().expect("tempdir");
    let victim_dir = tempfile::tempdir().expect("victim dir");
    let victim = victim_dir.path().join("victim.txt");
    std::fs::write(&victim, VICTIM_ORIGINAL).expect("seed victim");
    let marker = dir.path().join("unvalidated-mutation.marker");
    for s in 100..116u64 {
        symlink(&victim, current_candidate(dir.path(), s)).expect("plant");
    }
    let calls = Cell::new(0u64);
    let result = write_indeterminate_marker_with_suffix_fn(&fields(), &marker, || {
        let v = 100 + calls.get();
        calls.set(calls.get() + 1);
        v
    });
    let err = result.expect_err("exhausted candidates must fail closed");
    assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(calls.get(), 16, "exactly 16 bounded attempts");
    assert_eq!(
        std::fs::read_to_string(&victim).expect("read victim"),
        VICTIM_ORIGINAL,
        "nothing may be written through any planted link"
    );
    assert!(!marker.exists(), "no marker on fail-closed");
}
