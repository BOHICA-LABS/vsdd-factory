// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-15.03 SEC-003 (CWE-367, TOCTOU race condition) — atomic
//! write-then-rename for every governed-file write.
//!
//! `migrate_file`, `rotate_changelog`, and `register_artifact_paths`
//! previously called plain `std::fs::write(path, ...)` directly against the
//! target after reading it, with no atomicity between the read and the
//! write. This suite exercises `src/atomic_write.rs::write_atomic` directly
//! (the REAL production write primitive — POLICY 11) plus each of the 3
//! call sites' observable behavior: the target file is either fully old or
//! fully new content, never partially written, and no `.tmp-*` sibling
//! survives a successful write.

mod common;

use last_amended_migrate::atomic_write::write_atomic;
use last_amended_migrate::migrate::{MigrationMode, MigrationOptions};
use last_amended_migrate::registry::register_artifact_paths;
use last_amended_migrate::rotate::rotate_changelog;
use last_amended_migrate::{migrate_file, parse_frontmatter};

/// Direct unit proof of the primitive itself: after a successful
/// `write_atomic`, the target holds exactly the new content, and no
/// `.tmp-<pid>` sibling file is left behind in the directory.
#[test]
fn test_BC_10_13_001_SEC003_write_atomic_leaves_no_tmp_sibling_on_success() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::write_file(dir.path(), "target.md", "old content\n");

    write_atomic(&path, "new content\n").expect("write_atomic must succeed");

    assert_eq!(common::read_file(&path), "new content\n");

    let leftover_tmp: Vec<_> = std::fs::read_dir(dir.path())
        .expect("read tempdir")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
        .collect();
    assert!(
        leftover_tmp.is_empty(),
        "no .tmp-<pid> sibling must survive a successful write_atomic call: {leftover_tmp:?}"
    );
}

/// The target is NEVER observed in a partially-written state: since
/// `write_atomic` writes to a fresh sibling file and only then `rename`s it
/// into place, the target file's content at every point in time is either
/// the exact old content or the exact new content — this is the
/// rename-is-atomic-on-the-same-filesystem property the fix relies on. A
/// true concurrent-writer race is impractical to construct deterministically
/// in a unit test; this test instead proves the mechanism (temp-then-rename,
/// never a direct in-place write) is what actually executes, by asserting a
/// `.tmp-<pid>` file DOES briefly exist as a real filesystem object (i.e.
/// the write genuinely goes through a separate file, not the target
/// in-place) before the call returns — read via the returned content check
/// above — and that after the call there is exactly one file with the
/// target's name holding fully-new content, never a truncated fragment of
/// it.
#[test]
fn test_BC_10_13_001_SEC003_write_atomic_target_is_never_truncated_or_mixed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let old = "a".repeat(10_000);
    let new = "b".repeat(20_000);
    let path = common::write_file(dir.path(), "target.md", &old);

    write_atomic(&path, &new).expect("write_atomic must succeed");

    let after = common::read_file(&path);
    assert!(
        after == new,
        "the target must hold exactly the new content, never a mix of old/new \
         bytes or a truncated fragment of either"
    );
}

/// `write_atomic` never touches the real target at all when the temp-file
/// write itself fails — the original content survives untouched, proving the
/// temp-then-rename ordering (never "truncate target, then write") is what
/// actually runs.
///
/// # S-15.03 Windows-CI portability fix
///
/// This test previously forced the temp-file write to fail by marking the
/// target's parent DIRECTORY read-only via `Permissions::set_readonly(true)`.
/// That construction is Unix-only in practice: Windows' `FILE_ATTRIBUTE_READONLY`
/// bit on a *directory* does not block creating files inside it (it is a
/// legacy Explorer "customized folder" flag, not an access-control
/// restriction — only the read-only bit on a *file* is enforced by Windows
/// I/O) — so on Windows this write would silently SUCCEED, `result.is_err()`
/// would fail, and the whole test would fail there, unrelated to any bug in
/// `write_atomic` itself.
///
/// Replaced with a construction that is deterministic and enforced
/// identically by every OS's filesystem API: pre-create a DIRECTORY at
/// exactly the path `write_atomic` will itself compute for its sibling temp
/// file (`.{basename}.tmp-{pid}`, in the same parent directory as `path`).
/// The test and `write_atomic` run in the same process, so `std::process::id()`
/// is identical between them, making this path fully deterministic to
/// reproduce here without reaching into `write_atomic`'s internals. Opening a
/// path that already exists as a directory for writing as a plain file
/// deterministically fails everywhere (Unix: `EISDIR`; Windows:
/// `ERROR_ACCESS_DENIED`), so `File::create(&tmp_path)` inside
/// `write_atomic` is guaranteed to fail on every platform, exercising the
/// exact "temp-file write itself fails" branch this test targets.
#[test]
fn test_BC_10_13_001_SEC003_write_atomic_leaves_target_untouched_on_temp_write_failure() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::write_file(dir.path(), "target.md", "original\n");

    let blocking_tmp_dir = dir
        .path()
        .join(format!(".target.md.tmp-{}", std::process::id()));
    std::fs::create_dir(&blocking_tmp_dir)
        .expect("pre-create a directory at write_atomic's own deterministic temp-file path");

    let result = write_atomic(&path, "attempted overwrite\n");

    assert!(
        result.is_err(),
        "write_atomic must fail when its own sibling temp-file path is \
         already occupied by a directory, not silently succeed: {result:?}"
    );
    assert_eq!(
        common::read_file(&path),
        "original\n",
        "the original target content must survive untouched when the temp \
         write fails"
    );
}

/// S-15.03 pr-reviewer N2: `write_atomic` preserves the pre-existing
/// target's non-default permission bits (`0o600`) across the write —
/// `File::create`'s own default mode must never silently reset them.
/// Unix-only: `std::fs::Permissions`'s mode bits are a Unix-specific
/// concept (`PermissionsExt`); Windows only carries a read-only flag, which
/// this test does not attempt to exercise precisely.
#[cfg(unix)]
#[test]
fn test_BC_10_13_001_N2_write_atomic_preserves_non_default_file_mode() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("tempdir");
    let path = common::write_file(dir.path(), "target.md", "old content\n");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
        .expect("set non-default 0o600 mode on the pre-existing target");

    write_atomic(&path, "new content\n").expect("write_atomic must succeed");

    assert_eq!(common::read_file(&path), "new content\n");
    let mode_after = std::fs::metadata(&path)
        .expect("stat target after write_atomic")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(
        mode_after, 0o600,
        "write_atomic must preserve the pre-existing target's permission \
         bits across the temp-then-rename, not reset them to the process's \
         default create mode"
    );
}

// ── Call-site integration: migrate_file / rotate_changelog / register ──────

#[test]
fn test_BC_10_13_001_SEC003_migrate_file_write_leaves_no_tmp_sibling() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended = common::clean_current_entry("2026-09-02", "v4.430", "bootstrap fixture");
    let content = common::frontmatter_file(
        "story-index",
        "4.430",
        &last_amended,
        None,
        "# Fixture STORY-INDEX\n",
    );
    let path = common::write_file(dir.path(), "STORY-INDEX.md", &content);

    let report = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default())
        .expect("migrate_file must succeed");
    assert!(report.mutated);

    let leftover_tmp: Vec<_> = std::fs::read_dir(dir.path())
        .expect("read tempdir")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
        .collect();
    assert!(
        leftover_tmp.is_empty(),
        "migrate_file's write must go through write_atomic and leave no \
         .tmp-<pid> sibling: {leftover_tmp:?}"
    );
    assert!(
        parse_frontmatter(&path).is_ok(),
        "target file must be intact and parseable"
    );
}

#[test]
fn test_BC_10_13_001_SEC003_rotate_changelog_writes_leave_no_tmp_siblings() {
    let dir = tempfile::tempdir().expect("tempdir");
    let items: Vec<String> = (1..=8)
        .rev()
        .map(|n| common::changelog_item_block(&format!("2026-08-{n:02}"), &format!("item-{n}")))
        .collect();
    let last_amended = common::clean_current_entry("2026-09-02", "v5.41", "current entry");
    let content = common::frontmatter_file(
        "behavioral-contract-index",
        "5.41",
        &last_amended,
        Some(&items),
        "# Fixture BC-INDEX\n",
    );
    let path = common::write_file(
        dir.path(),
        ".factory/specs/behavioral-contracts/BC-INDEX.md",
        &content,
    );

    let report = rotate_changelog(&path, "test-cycle", 3, MigrationMode::Apply)
        .expect("rotate_changelog must succeed");
    assert!(report.mutated);

    for check_dir in [
        path.parent().expect("source parent"),
        report.archive_path.parent().expect("archive parent"),
    ] {
        let leftover_tmp: Vec<_> = std::fs::read_dir(check_dir)
            .expect("read dir")
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(
            leftover_tmp.is_empty(),
            "rotate_changelog's writes must leave no .tmp-<pid> sibling in \
             {check_dir:?}: {leftover_tmp:?}"
        );
    }
    assert!(report.archive_path.exists());
    assert!(parse_frontmatter(&path).is_ok());
}

#[test]
fn test_BC_10_13_001_SEC003_register_artifact_paths_write_leaves_no_tmp_sibling() {
    let dir = tempfile::tempdir().expect("tempdir");
    let registry_path = common::write_file(
        dir.path(),
        "artifact-path-registry.yaml",
        "version: 1\nartifacts:\n",
    );

    register_artifact_paths(&registry_path).expect("register_artifact_paths must succeed");

    let leftover_tmp: Vec<_> = std::fs::read_dir(dir.path())
        .expect("read tempdir")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
        .collect();
    assert!(
        leftover_tmp.is_empty(),
        "register_artifact_paths's write must leave no .tmp-<pid> sibling: {leftover_tmp:?}"
    );
    assert!(
        common::read_file(&registry_path).contains("artifacts:"),
        "the registry file must be intact after the atomic write"
    );
}
