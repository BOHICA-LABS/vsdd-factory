// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-25.08: registry-error exit-code mapping (BC-7.06.001
//! §Fail-Closed Symmetry; BC-1.08.001 Invariants 1/2).
//!
//! The former inline `match` in `main.rs::run` ended in `_ => 0`, so a FUTURE
//! `RegistryError` variant would have silently failed OPEN. The testable seam is
//! the pure `registry_error_exit_code(&RegistryError) -> i32` in `registry.rs`,
//! which is implemented, and `main.rs::run` delegates to it
//! (`factory_dispatcher::registry::registry_error_exit_code(&e)`).
//!
//! "Unknown variant => non-zero" cannot be exercised for a variant that does not
//! exist, so it is pinned STRUCTURALLY: `expected_exit` below matches EVERY
//! current `RegistryError` variant explicitly with NO wildcard arm. Adding a
//! variant to `RegistryError` breaks THIS test's build until the author decides
//! (here, and in `registry_error_exit_code`) whether it fails open or closed.
//! `registry_error_exit_code` keeps its fail-open arm an explicit variant list
//! (no `_`).

use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use factory_dispatcher::registry::{RegistryError, registry_error_exit_code};

/// Expected exit code per variant; the match is exhaustive WITHOUT a wildcard.
fn expected_exit(e: &RegistryError) -> i32 {
    match e {
        RegistryError::NotFound(_) => 0,
        RegistryError::Io(_) => 0,
        RegistryError::Toml(_) => 0,
        RegistryError::ToolRegex { .. } => 0,
        RegistryError::SchemaVersion { .. } => 2,
        RegistryError::AsyncBlockConflict { .. } => 2,
        RegistryError::DuplicateEntry { .. } => 2,
    }
}

fn every_variant() -> Vec<RegistryError> {
    let toml_err = toml::from_str::<toml::Value>("not [[[ toml").unwrap_err();
    let bad_pattern = String::from("(");
    let regex_err = regex::Regex::new(&bad_pattern).unwrap_err();
    vec![
        RegistryError::NotFound(PathBuf::from("/nonexistent/hooks-registry.toml")),
        RegistryError::Io(std::io::Error::other("io")),
        RegistryError::Toml(toml_err),
        RegistryError::ToolRegex {
            name: "p".into(),
            pattern: "(".into(),
            source: regex_err,
        },
        RegistryError::SchemaVersion {
            got: 99,
            expected: 2,
        },
        RegistryError::AsyncBlockConflict {
            name: "p".into(),
            on_error: "block".into(),
        },
        RegistryError::DuplicateEntry {
            name: "p".into(),
            event: "PreToolUse".into(),
            tool: None,
        },
    ]
}

#[test]
fn test_BC_7_06_001_registry_error_exit_code_fail_open_variants_exit_0() {
    for e in every_variant() {
        if expected_exit(&e) == 0 {
            assert_eq!(
                registry_error_exit_code(&e),
                0,
                "fail-open variant must map to exit 0 (BC-1.08.001): {e:?}"
            );
        }
    }
}

#[test]
fn test_BC_7_06_001_registry_error_exit_code_fail_closed_variants_exit_2() {
    for e in every_variant() {
        if expected_exit(&e) == 2 {
            assert_eq!(
                registry_error_exit_code(&e),
                2,
                "fail-closed variant must map to exit 2 (ADR-019 Decision 2): {e:?}"
            );
        }
    }
}

#[test]
fn test_BC_7_06_001_registry_error_exit_code_covers_every_variant_explicitly() {
    // Structural pin: `every_variant` + `expected_exit` enumerate each variant, so a
    // new `RegistryError` variant breaks the build here. Run the whole table once.
    let table = every_variant();
    assert_eq!(
        table.len(),
        7,
        "update the table AND registry_error_exit_code when a variant is added"
    );
    for e in &table {
        assert_eq!(registry_error_exit_code(e), expected_exit(e), "{e:?}");
    }
}

// ---- real-binary pins (they lock the wired behavior that
// `registry_error_exit_code` preserves) ----

fn run_binary(registry_body: Option<&str>) -> i32 {
    let root = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    if let Some(b) = registry_body {
        std::fs::write(root.path().join("hooks-registry.toml"), b).unwrap();
    }
    let payload = serde_json::json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "session_id": "sess-reg",
        "tool_input": { "command": "true" },
    })
    .to_string();
    let mut child = Command::new(env!("CARGO_BIN_EXE_factory-dispatcher"))
        .env("CLAUDE_PLUGIN_ROOT", root.path())
        .env("CLAUDE_PROJECT_DIR", project.path())
        .env("VSDD_LOG_DIR", project.path().join("logs"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(payload.as_bytes()).unwrap();
    drop(stdin);
    child.wait_with_output().unwrap().status.code().unwrap()
}

/// REGRESSION PIN (green at Red Gate): a missing registry fails OPEN.
#[test]
fn test_BC_1_08_001_real_binary_missing_registry_fails_open_exit_0_regression_pin() {
    assert_eq!(run_binary(None), 0);
}

/// REGRESSION PIN (green at Red Gate): a schema-version mismatch fails CLOSED.
#[test]
fn test_BC_7_06_001_real_binary_schema_mismatch_fails_closed_exit_2_regression_pin() {
    assert_eq!(run_binary(Some("schema_version = 99\n")), 2);
}
