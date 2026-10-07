// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! BC-10.13.001 `--check` CLI mode — binary-boundary integration tests
//! against the COMPILED `last-amended-migrate` binary (via
//! `CARGO_BIN_EXE_last-amended-migrate`), matching the workspace's existing
//! pattern in `crates/policy15-attestation-gate/tests/binary_integration_test.rs`.
//!
//! `src/cli.rs`'s own doc comment states the convention explicitly:
//! "`--check` (report violations without writing, mirroring
//! `compute-input-hash`'s `--check` convention)". `compute-input-hash`'s own
//! usage banner documents its own exit-code contract: "0 — success (or
//! match for --check ...); 2 — mismatch (--check) ... found stale files".
//! These tests pin the same shape for `last-amended-migrate`: **exit 0 on a
//! clean/compliant target, nonzero on drift**, and — the property specific
//! to `--check` regardless of the exact code chosen — **never a write**.
//!
//! `src/main.rs` currently has ZERO tests (its only line, `run(Cli::parse())`,
//! delegates entirely to the `todo!()`-stubbed `cli::run`), so this file also
//! doubles as the Red Gate for that dispatch/exit-code-mapping surface.

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

/// `--check` against a fixture carrying the D-1144 escape defect must exit
/// NONZERO (drift found) and must NOT mutate the file.
///
/// S-15.03 SEC-002: `--path` now goes through an allowlist that requires it
/// to resolve, under `--factory-root`, to one of the 5 real BC-10.13.001
/// `TARGET_FILES` relative paths — so the fixture lives at
/// `<tmp>/specs/behavioral-contracts/BC-INDEX.md` and the CLI invocation
/// passes `--factory-root <tmp>`, matching real usage exactly (rather than a
/// flat basename-only tempdir layout, which the allowlist now correctly
/// rejects as out-of-scope).
#[test]
fn test_BC_10_13_001_cli_check_mode_nonzero_exit_on_defect_without_mutation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended = common::quote_defect_current_entry("2026-09-02", "v5.41");
    let content = common::frontmatter_file(
        "behavioral-contract-index",
        "5.41",
        &last_amended,
        Some(&[common::changelog_item_block("2026-08-01", "an older entry")]),
        "# Fixture BC-INDEX\n",
    );
    let path = common::write_file(
        dir.path(),
        "specs/behavioral-contracts/BC-INDEX.md",
        &content,
    );
    let before = common::read_file(&path);

    let (exit_code, stdout, stderr) = run_cli(&[
        "migrate",
        "--path",
        path.to_str().expect("utf8 path"),
        "--factory-root",
        dir.path().to_str().expect("utf8 tempdir path"),
        "--check",
    ]);

    assert_ne!(
        exit_code, 0,
        "--check must exit nonzero on drift (mirrors compute-input-hash's \
         --check convention); stdout={stdout:?} stderr={stderr:?}"
    );
    // A nonzero exit alone would also (mis)pass on an unhandled panic/crash
    // — assert the process took the CONTROLLED nonzero-exit-on-drift path
    // (`ExitCode::FAILURE` per `cli::run`'s own doc comment), not a
    // panic-driven process abort, so this test cannot pass by coincidence
    // before real dispatch logic exists.
    assert!(
        !stderr.contains("panicked"),
        "the nonzero exit must come from controlled Err-to-ExitCode mapping, \
         not an unhandled panic: stderr={stderr:?}"
    );

    let after = common::read_file(&path);
    assert_eq!(
        before, after,
        "--check must NEVER mutate the target file, regardless of exit code"
    );
}

/// `--check` against an already-fully-compliant fixture must exit 0 and
/// must not mutate the file. S-15.03 SEC-002: same `--factory-root`-relative
/// fixture layout as the sibling test above.
#[test]
fn test_BC_10_13_001_cli_check_mode_zero_exit_on_compliant_fixture() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended = common::clean_current_entry("2026-09-02", "v4.11", "already compliant");
    let content = common::frontmatter_file(
        "architecture-index",
        "4.11",
        &last_amended,
        Some(&[common::changelog_item_block("2026-08-01", "an older entry")]),
        "# Fixture ARCH-INDEX\n",
    );
    let path = common::write_file(dir.path(), "specs/architecture/ARCH-INDEX.md", &content);
    let before = common::read_file(&path);

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
        "--check must exit 0 on an already-compliant fixture; \
         stdout={stdout:?} stderr={stderr:?}"
    );

    let after = common::read_file(&path);
    assert_eq!(before, after, "--check must never mutate, even on success");
}

/// Without `--check` (apply mode, the default), the CLI must actually
/// perform the migration write and exit 0. S-15.03 SEC-002: same
/// `--factory-root`-relative fixture layout as the sibling tests above.
#[test]
fn test_BC_10_13_001_cli_apply_mode_mutates_and_exits_zero() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended = common::clean_current_entry("2026-09-02", "v4.430", "bootstrap fixture");
    let content = common::frontmatter_file(
        "story-index",
        "4.430",
        &last_amended,
        None,
        "# Fixture STORY-INDEX\n",
    );
    let path = common::write_file(dir.path(), "stories/STORY-INDEX.md", &content);

    let (exit_code, stdout, stderr) = run_cli(&[
        "migrate",
        "--path",
        path.to_str().expect("utf8 path"),
        "--factory-root",
        dir.path().to_str().expect("utf8 tempdir path"),
    ]);

    assert_eq!(
        exit_code, 0,
        "apply-mode migration of a valid fixture must exit 0; \
         stdout={stdout:?} stderr={stderr:?}"
    );

    let after = common::read_file(&path);
    assert!(
        after.contains("changelog:"),
        "apply mode must actually perform the migration write: {after:?}"
    );
}
