// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-15.03 SEC-002 (CWE-73, external control of file name/path) —
//! `--path`/`--registry` CLI argument allowlisting.
//!
//! `cli.rs`'s `Migrate { path: Some(p), .. }`, `Rotate { path, .. }`, and
//! `Command::Register { registry }` used to accept an arbitrary `PathBuf`
//! with zero validation. This suite exercises the COMPILED binary (matching
//! `bc_10_13_001_cli_check_mode_test.rs`'s existing pattern) to prove: an
//! in-scope path is accepted for each of the 3 subcommands, and a
//! traversal/out-of-scope path is rejected for each, before any file I/O
//! against its content occurs.

mod common;

use std::path::PathBuf;
use std::process::Command;

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_last-amended-migrate"))
}

fn run_cli(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(binary_path())
        .args(args)
        .output()
        .expect("spawn last-amended-migrate binary");
    (
        output.status.code().expect("process terminated by signal"),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

// ── migrate --path ──────────────────────────────────────────────────────────

#[test]
fn test_BC_10_13_001_SEC002_migrate_path_in_scope_is_accepted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended = common::clean_current_entry("2026-09-02", "v1.0", "in-scope fixture");
    let content = common::frontmatter_file(
        "behavioral-contract-index",
        "1.0",
        &last_amended,
        Some(&[]),
        "# Fixture BC-INDEX\n",
    );
    let path = common::write_file(
        dir.path(),
        "specs/behavioral-contracts/BC-INDEX.md",
        &content,
    );

    let (exit_code, stdout, stderr) = run_cli(&[
        "migrate",
        "--path",
        path.to_str().expect("utf8 path"),
        "--factory-root",
        dir.path().to_str().expect("utf8 tempdir path"),
        "--check",
    ]);

    assert_eq!(
        exit_code, 0,
        "an in-scope --path resolving under --factory-root must be accepted; \
         stdout={stdout:?} stderr={stderr:?}"
    );
    assert!(!stderr.contains("PathNotAllowed") && !stderr.contains("not an allowed target"));
}

#[test]
fn test_BC_10_13_001_SEC002_migrate_path_out_of_scope_basename_is_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    // A file that is NOT one of the 5 BC-10.13.001 TARGET_FILES at all.
    let content = "---\nlast_amended: \"ok\"\n---\n\nnot a governed file\n";
    let path = common::write_file(dir.path(), "some/other/SECRETS.md", content);
    let before = common::read_file(&path);

    let (exit_code, stdout, stderr) = run_cli(&[
        "migrate",
        "--path",
        path.to_str().expect("utf8 path"),
        "--factory-root",
        dir.path().to_str().expect("utf8 tempdir path"),
    ]);

    assert_ne!(
        exit_code, 0,
        "an out-of-allowlist --path must be rejected before any write; \
         stdout={stdout:?} stderr={stderr:?}"
    );
    assert!(
        stderr.contains("not an allowed target"),
        "rejection must come from the path allowlist, not an unrelated \
         error: stderr={stderr:?}"
    );
    let after = common::read_file(&path);
    assert_eq!(before, after, "a rejected path must never be written to");
}

#[test]
fn test_BC_10_13_001_SEC002_migrate_path_traversal_outside_factory_root_is_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Correct basename (BC-INDEX.md) but NOT under --factory-root's
    // expected `specs/behavioral-contracts/` relative location — simulates
    // an attacker supplying `--path ../../../elsewhere/BC-INDEX.md`-shaped
    // traversal that lands outside the intended scope while still matching
    // a plausible basename.
    let last_amended = common::clean_current_entry("2026-09-02", "v1.0", "wrong location");
    let content = common::frontmatter_file(
        "behavioral-contract-index",
        "1.0",
        &last_amended,
        Some(&[]),
        "# Fixture BC-INDEX\n",
    );
    let path = common::write_file(dir.path(), "unexpected/location/BC-INDEX.md", &content);
    let before = common::read_file(&path);

    let (exit_code, stdout, stderr) = run_cli(&[
        "migrate",
        "--path",
        path.to_str().expect("utf8 path"),
        "--factory-root",
        dir.path().to_str().expect("utf8 tempdir path"),
    ]);

    assert_ne!(
        exit_code, 0,
        "a path outside the factory-root-relative allowlist must be \
         rejected even with a matching basename; stdout={stdout:?} stderr={stderr:?}"
    );
    assert!(stderr.contains("not an allowed target"));
    let after = common::read_file(&path);
    assert_eq!(before, after);
}

// ── rotate --path ────────────────────────────────────────────────────────────

#[test]
fn test_BC_10_13_001_SEC002_rotate_path_under_factory_ancestor_is_accepted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let items: Vec<String> = (1..=5)
        .map(|n| common::changelog_item_block(&format!("2026-08-{n:02}"), &format!("item-{n}")))
        .collect();
    let last_amended = common::clean_current_entry("2026-09-02", "v1.0", "current entry");
    let content = common::frontmatter_file(
        "behavioral-contract-index",
        "1.0",
        &last_amended,
        Some(&items),
        "# Fixture BC-INDEX\n",
    );
    let path = common::write_file(
        dir.path(),
        ".factory/specs/behavioral-contracts/BC-INDEX.md",
        &content,
    );

    let (exit_code, stdout, stderr) = run_cli(&[
        "rotate",
        "--path",
        path.to_str().expect("utf8 path"),
        "--cycle-name",
        "test-cycle",
        "--keep-recent",
        "10",
    ]);

    // `keep_recent` (10) exceeds the fixture's 5 items — EC-004 below-
    // threshold no-op, so apply mode still exits 0 regardless of the path
    // allowlist outcome. What this test actually pins is that acceptance
    // happened: no PathNotAllowed rejection in stderr.
    assert_eq!(
        exit_code, 0,
        "a --path under a .factory/ ancestor must be accepted (and this \
         below-threshold rotation is a no-op, so it succeeds trivially); \
         stdout={stdout:?} stderr={stderr:?}"
    );
    assert!(!stderr.contains("not an allowed target"));
}

#[test]
fn test_BC_10_13_001_SEC002_rotate_path_outside_factory_ancestor_is_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let items: Vec<String> = (1..=5)
        .map(|n| common::changelog_item_block(&format!("2026-08-{n:02}"), &format!("item-{n}")))
        .collect();
    let last_amended = common::clean_current_entry("2026-09-02", "v1.0", "current entry");
    let content = common::frontmatter_file(
        "behavioral-contract-index",
        "1.0",
        &last_amended,
        Some(&items),
        "# Fixture BC-INDEX\n",
    );
    // No `.factory/` ancestor component anywhere in this path.
    let path = common::write_file(dir.path(), "not-factory/BC-INDEX.md", &content);
    let before = common::read_file(&path);

    let (exit_code, stdout, stderr) = run_cli(&[
        "rotate",
        "--path",
        path.to_str().expect("utf8 path"),
        "--cycle-name",
        "test-cycle",
        "--keep-recent",
        "2",
    ]);

    assert_ne!(
        exit_code, 0,
        "a --path with no .factory/ ancestor must be rejected; \
         stdout={stdout:?} stderr={stderr:?}"
    );
    assert!(stderr.contains("not an allowed target"));
    let after = common::read_file(&path);
    assert_eq!(
        before, after,
        "a rejected rotate --path must never be written to"
    );
}

// ── register --registry ─────────────────────────────────────────────────────

#[test]
fn test_BC_10_13_001_SEC002_register_registry_expected_basename_is_accepted() {
    let dir = tempfile::tempdir().expect("tempdir");
    // S-15.03 pr-reviewer S4: the allowlist now checks the full
    // plugins/vsdd-factory/config/ relative shape, not just the basename —
    // this fixture recreates that trailing shape under the tempdir root.
    let registry_path = common::write_file(
        dir.path(),
        "plugins/vsdd-factory/config/artifact-path-registry.yaml",
        "version: 1\nartifacts:\n",
    );

    let (exit_code, stdout, stderr) = run_cli(&[
        "register",
        "--registry",
        registry_path.to_str().expect("utf8 path"),
    ]);

    assert_eq!(
        exit_code, 0,
        "a registry path with the expected plugins/vsdd-factory/config/ \
         shape must be accepted; stdout={stdout:?} stderr={stderr:?}"
    );
    assert!(!stderr.contains("not an allowed target"));
}

#[test]
fn test_BC_10_13_001_SEC002_register_registry_wrong_basename_is_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    // An arbitrary file that is NOT the registry — e.g. an attacker pointing
    // `--registry` at some unrelated file to have arbitrary text appended.
    let target_path = common::write_file(
        dir.path(),
        "plugins/vsdd-factory/config/not-the-registry.yaml",
        "version: 1\n",
    );
    let before = common::read_file(&target_path);

    let (exit_code, stdout, stderr) = run_cli(&[
        "register",
        "--registry",
        target_path.to_str().expect("utf8 path"),
    ]);

    assert_ne!(
        exit_code, 0,
        "a --registry path with the wrong basename must be rejected; \
         stdout={stdout:?} stderr={stderr:?}"
    );
    assert!(stderr.contains("not an allowed target"));
    let after = common::read_file(&target_path);
    assert_eq!(
        before, after,
        "a rejected --registry path must never be read/appended/written to"
    );
}

/// S-15.03 pr-reviewer S4: the pre-fix basename-only check would have
/// accepted THIS fixture — a file that shares the exact
/// `artifact-path-registry.yaml` basename but lives in a completely wrong
/// directory (no `plugins/vsdd-factory/config/` ancestor shape at all).
/// The tightened shape check must reject it.
#[test]
fn test_BC_10_13_001_SEC002_register_registry_same_basename_wrong_directory_is_rejected() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target_path = common::write_file(
        dir.path(),
        "some/other/unrelated/dir/artifact-path-registry.yaml",
        "version: 1\nartifacts:\n",
    );
    let before = common::read_file(&target_path);

    let (exit_code, stdout, stderr) = run_cli(&[
        "register",
        "--registry",
        target_path.to_str().expect("utf8 path"),
    ]);

    assert_ne!(
        exit_code, 0,
        "a same-basename registry file in the wrong directory must be \
         rejected; stdout={stdout:?} stderr={stderr:?}"
    );
    assert!(
        stderr.contains("not an allowed target"),
        "rejection must come from the path allowlist: stderr={stderr:?}"
    );
    let after = common::read_file(&target_path);
    assert_eq!(
        before, after,
        "a rejected same-basename-wrong-directory --registry path must \
         never be read/appended/written to"
    );
}
