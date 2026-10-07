// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! BC-10.13.001 PC6 / §Architecture Anchors / S-15.03 AC-006 — CLI-surface
//! completeness gap: `registry::register_artifact_paths` (PC6) has no
//! subcommand wiring it to the CLI (`src/cli.rs` only exposes `migrate` and
//! `rotate`). This is a binary-boundary integration test against the
//! COMPILED `last-amended-migrate` binary (via
//! `CARGO_BIN_EXE_last-amended-migrate`), matching the existing
//! `bc_10_13_001_cli_check_mode_test.rs` pattern.
//!
//! Argument style matches the existing `Migrate`/`Rotate` subcommands
//! (`--registry <path>`, long-flag clap-derive convention) and the
//! `compute-input-hash` `--path`/target-flag convention.

mod common;

use std::path::PathBuf;
use std::process::Command;

const REGISTRY_HEADER: &str = "version: 1\nartifacts:\n";

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

/// `register --registry <path>` must exit 0 and append all 5 D-1149 sidecar
/// basenames to the registry file (AC-006/PC6), exercised through the
/// COMPILED binary (not the library function directly — that's already
/// covered by `bc_10_13_001_pc6_registry_test.rs`; this test's job is to
/// prove the CLI subcommand exists and wires through to it).
#[test]
fn test_BC_10_13_001_cli_register_subcommand_adds_all_five_sidecars() {
    let dir = tempfile::tempdir().expect("tempdir");
    // S-15.03 pr-reviewer S4: the CLI's --registry allowlist now checks the
    // full plugins/vsdd-factory/config/ relative shape, not just the
    // basename — this fixture recreates that trailing shape under the
    // tempdir root.
    let registry_path = common::write_file(
        dir.path(),
        "plugins/vsdd-factory/config/artifact-path-registry.yaml",
        REGISTRY_HEADER,
    );
    for basename in common::expected_sidecar_basenames() {
        assert!(
            !common::read_file(&registry_path).contains(&basename),
            "fixture sanity: {basename} must not be pre-registered"
        );
    }

    let (exit_code, stdout, stderr) = run_cli(&[
        "register",
        "--registry",
        registry_path.to_str().expect("utf8 path"),
    ]);

    assert_eq!(
        exit_code, 0,
        "register subcommand must exit 0 on success; stdout={stdout:?} stderr={stderr:?}"
    );
    assert!(
        !stderr.contains("panicked"),
        "success must come from controlled dispatch, not a panic: stderr={stderr:?}"
    );

    let after = common::read_file(&registry_path);
    for basename in common::expected_sidecar_basenames() {
        assert!(
            after.contains(&basename),
            "expected the registry to reference the D-1149 sidecar \
             {basename:?} after `register`, got: {after:?}"
        );
    }
}

/// Idempotency at the CLI boundary: running `register` twice against the
/// same registry file must not duplicate entries (mirrors PC6's idempotency
/// invariant, exercised through the binary rather than the library call).
#[test]
fn test_BC_10_13_001_cli_register_subcommand_is_idempotent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let registry_path = common::write_file(
        dir.path(),
        "plugins/vsdd-factory/config/artifact-path-registry.yaml",
        REGISTRY_HEADER,
    );
    let registry_arg = registry_path.to_str().expect("utf8 path");

    let (exit_code_1, stdout_1, stderr_1) = run_cli(&["register", "--registry", registry_arg]);
    assert_eq!(
        exit_code_1, 0,
        "first register call must succeed; stdout={stdout_1:?} stderr={stderr_1:?}"
    );
    let after_first = common::read_file(&registry_path);

    let (exit_code_2, stdout_2, stderr_2) = run_cli(&["register", "--registry", registry_arg]);
    assert_eq!(
        exit_code_2, 0,
        "second register call must also succeed; stdout={stdout_2:?} stderr={stderr_2:?}"
    );
    let after_second = common::read_file(&registry_path);

    assert_eq!(
        after_first, after_second,
        "re-running `register` must not add duplicate entries"
    );
    for basename in common::expected_sidecar_basenames() {
        assert_eq!(
            after_second.matches(&basename).count(),
            1,
            "{basename} must appear exactly once, not duplicated across 2 CLI runs"
        );
    }
}
