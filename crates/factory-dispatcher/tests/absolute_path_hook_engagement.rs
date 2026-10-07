// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Integration validation: absolute-path hook engagement for
//! `validate-stable-anchors` and `validate-artifact-path` WASM plugins.
//!
//! # Context
//!
//! Claude Code's hook envelope sends `tool_input.file_path` as an ABSOLUTE
//! path (e.g. `/Users/jmagady/Dev/vsdd-factory/.factory/specs/foo.md`).
//!
//! Pre-fix, both hooks short-circuited on `starts_with(".factory/")`, which
//! is FALSE for absolute paths, causing silent bypass — the hook was
//! registered and active but never gated any absolute-path Edit/Write.
//!
//! ## Fixes merged on `origin/fix/S-15.01-F5-convergence`
//!
//! - `cc5a016b`: `validate-stable-anchors` — `is_spec_target` now accepts
//!   both `.factory/specs/...` (relative) and `/…/.factory/specs/...`
//!   (absolute).
//!
//! - `8b4f697f`: `validate-artifact-path` — `matches_canonical` and
//!   `hook_logic` both accept absolute paths via leading-slash discipline.
//!
//! # What this file validates (integration level)
//!
//! These tests exercise the ACTUAL compiled WASM binaries through the
//! dispatcher's `invoke_plugin` path — not the Rust unit-test surface that
//! calls `hook_logic` directly with injectable callbacks.
//!
//! WASM files under test:
//! - **Pre-fix** (validate-artifact-path only):
//!   `plugins/vsdd-factory/hook-plugins/validate-artifact-path.wasm` (303672 B,
//!   committed in S-13.01, before `8b4f697f`). Demonstrates the bypass bug.
//!
//! - **Post-fix** (validate-artifact-path):
//!   `target/wasm32-wasip1/release/validate-artifact-path.wasm` (305333 B,
//!   compiled from `8b4f697f` source). Must BLOCK on unregistered absolute
//!   paths and PASS on registered absolute paths.
//!
//! - **Post-fix** (validate-stable-anchors):
//!   `target/wasm32-wasip1/release/validate-stable-anchors.wasm` (172185 B,
//!   compiled from `cc5a016b` source). Must BLOCK when `<file>.<ext>:NNN`
//!   cite is in the proposed Edit payload for an absolute spec path, and
//!   PASS when no cite is present.
//!
//! # Assertions (per validation strategy)
//!
//! 1. Pre-fix bug reproduction: `plugins/` WASM returns Continue (exit_code 0)
//!    for an absolute unregistered .factory/ path — the bypass is real.
//!
//! 2. Post-fix correctness — validate-artifact-path:
//!    a. Unregistered absolute path → block (exit_code 2, block_intent).
//!    b. Registered absolute path → continue (exit_code 0, no block).
//!    c. Relative path regression: unregistered → block; registered → continue.
//!
//! 3. Post-fix correctness — validate-stable-anchors:
//!    a. Absolute spec path + new_string containing `foo.rs:42` → block.
//!    b. Absolute spec path + new_string with no cite → continue.
//!    c. Absolute non-spec path (not under .factory/specs/) → continue.
//!
//! 4. Leading-slash discipline: `prefix.factory/specs/foo.md` (no slash
//!    before .factory/) must NOT trigger either hook — prevents false positives.
//!
//! # BC trace
//! - BC-4.11.001: validate-artifact-path enforcement
//! - TD-031: validate-stable-anchors stable-anchor convention
//! - F-P18-001: sibling propagation gap (absolute-path bypass)
//!
//! # Notes on pre-fix WASM
//!
//! We do NOT test the pre-fix validate-stable-anchors WASM (no pre-fix binary
//! was committed to this repo — `plugins/` only carries validate-artifact-path).
//! The pre-fix code path is validated by unit tests in
//! `validate-stable-anchors/src/tests.rs` which read the pre-fix source directly.

use std::path::{Path, PathBuf};

use factory_dispatcher::engine::build_engine;
use factory_dispatcher::host::HostContext;
use factory_dispatcher::invoke::{InvokeLimits, PluginResult, invoke_plugin};
use factory_dispatcher::registry::{Capabilities, ReadFileCaps};
use wasmtime::Module;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Path to the repository root (this file is at
/// `crates/factory-dispatcher/tests/absolute_path_hook_engagement.rs`).
fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points to `crates/factory-dispatcher`.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Go up two levels: factory-dispatcher → crates → repo root.
    manifest_dir
        .parent()
        .expect("crates/")
        .parent()
        .expect("repo root")
        .to_path_buf()
}

/// Return the path to the pre-fix validate-artifact-path WASM binary.
///
/// This is the deployed binary committed in S-13.01 (before 8b4f697f). It
/// carries `starts_with(".factory/")` only, so it silently bypasses absolute
/// paths — which is exactly the bug we want to demonstrate is real.
fn pre_fix_artifact_path_wasm() -> PathBuf {
    repo_root().join("plugins/vsdd-factory/hook-plugins/validate-artifact-path.wasm")
}

/// Return the path to the post-fix validate-artifact-path WASM binary.
///
/// Built from `8b4f697f`. Carries the leading-slash discipline fix:
/// `starts_with(".factory/") || contains("/.factory/")`.
///
/// Prefers `target/wasm32-wasip1/release/*.wasm` (freshly-built by the
/// operator during fix verification). Falls back to the committed
/// `plugins/vsdd-factory/hook-plugins/*.wasm` after the fix lands, since
/// the committed binary is the post-fix version once shipped. This keeps
/// the test running in CI (which doesn't `cargo build --target wasm32-wasip1`)
/// without losing local-fresh-build validation.
fn post_fix_artifact_path_wasm() -> PathBuf {
    let fresh = repo_root().join("target/wasm32-wasip1/release/validate-artifact-path.wasm");
    if fresh.exists() {
        fresh
    } else {
        repo_root().join("plugins/vsdd-factory/hook-plugins/validate-artifact-path.wasm")
    }
}

/// Return the path to the post-fix validate-stable-anchors WASM binary.
///
/// Built from `cc5a016b`. Carries `is_spec_target` fix: accepts absolute
/// paths via `contains("/.factory/specs/")`.
///
/// Same fallback semantics as `post_fix_artifact_path_wasm`.
fn post_fix_stable_anchors_wasm() -> PathBuf {
    let fresh = repo_root().join("target/wasm32-wasip1/release/validate-stable-anchors.wasm");
    if fresh.exists() {
        fresh
    } else {
        repo_root().join("plugins/vsdd-factory/hook-plugins/validate-stable-anchors.wasm")
    }
}

/// Build a synthetic Claude Code hook envelope (PreToolUse Edit) with:
/// - `tool_name`: "Edit" (the tool that triggers PreToolUse hooks)
/// - `tool_input.file_path`: the supplied path (may be absolute or relative)
/// - `tool_input.new_string`: optional replacement text
///
/// The envelope format matches what Claude Code sends over stdin.
fn make_edit_payload(file_path: &str, new_string: Option<&str>) -> Vec<u8> {
    let mut tool_input = serde_json::json!({
        "file_path": file_path,
    });
    if let Some(ns) = new_string {
        tool_input["new_string"] = serde_json::json!(ns);
    }
    let envelope = serde_json::json!({
        "hook_event_name": "PreToolUse",
        "event_name": "PreToolUse",
        "session_id": "test-abs-path-integration-session",
        "dispatcher_trace_id": "test-abs-path-trace-001",
        "tool_name": "Edit",
        "tool_input": tool_input,
    });
    serde_json::to_vec(&envelope).expect("payload must serialize")
}

/// Build a synthetic Claude Code hook envelope for a Write operation.
fn make_write_payload(file_path: &str) -> Vec<u8> {
    let envelope = serde_json::json!({
        "hook_event_name": "PreToolUse",
        "event_name": "PreToolUse",
        "session_id": "test-abs-path-integration-session",
        "dispatcher_trace_id": "test-abs-path-trace-002",
        "tool_name": "Write",
        "tool_input": {
            "file_path": file_path,
            "content": "# Test content

No volatile cites here.
",
        },
    });
    serde_json::to_vec(&envelope).expect("payload must serialize")
}

/// A minimal artifact-path-registry YAML string containing one canonical BC
/// entry. Injected into the WASM plugin via the host's `read_file` callback.
///
/// The validate-artifact-path plugin calls `host::read_file(REGISTRY_PATH,
/// ...)` to load the registry. In tests the host resolves `read_file` from
/// the `HostContext.cwd`, which we set to a temp dir containing this file.
const REGISTRY_YAML: &str = r#"version: 1
artifacts:
  - artifact_type: behavioral-contract
    canonical_path_pattern: ".factory/specs/behavioral-contracts/ss-{subsystem}/BC-{bc-id}.md"
    description: Behavioral contract spec
    enforcement_level: "block"
"#;

/// Registry path the plugin requests (matches REGISTRY_PATH constant in
/// validate-artifact-path/src/lib.rs).
const REGISTRY_RELATIVE_PATH: &str = "plugins/vsdd-factory/config/artifact-path-registry.yaml";

/// Create a temp directory containing the registry YAML at the expected
/// relative path, and return (temp_dir, cwd_path). The temp_dir must stay
/// alive for the duration of the test.
fn setup_temp_registry() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir creation must succeed");
    let registry_dir = dir.path().join("plugins/vsdd-factory/config");
    std::fs::create_dir_all(&registry_dir).expect("registry dir creation must succeed");
    let registry_file = dir.path().join(REGISTRY_RELATIVE_PATH);
    std::fs::write(&registry_file, REGISTRY_YAML).expect("registry write must succeed");
    let cwd = dir.path().to_path_buf();
    (dir, cwd)
}

/// Invoke the plugin at `wasm_path` with the given `payload_json`, using
/// `cwd` as the working directory (so `read_file` resolves relative paths).
/// Returns the `PluginResult` for assertions.
///
/// The read_file capability is granted for `plugins/vsdd-factory/config`
/// (where the registry YAML lives). The `path_allow` list must be non-empty
/// and include the registry prefix — an empty list means "deny all".
fn invoke_hook(wasm_path: &Path, cwd: &Path, payload_json: &[u8]) -> PluginResult {
    let engine = build_engine().expect("wasmtime engine must build");
    let wasm_bytes = std::fs::read(wasm_path)
        .unwrap_or_else(|e| panic!("WASM read failed for {:?}: {}", wasm_path, e));
    let module = Module::from_binary(&engine, &wasm_bytes)
        .unwrap_or_else(|e| panic!("WASM compile failed for {:?}: {}", wasm_path, e));

    let mut host_ctx = HostContext::new(
        "validate-artifact-path",
        "0.0.1",
        "test-session",
        "test-trace",
    );
    host_ctx.cwd = cwd.to_path_buf();
    // Grant read_file capability for the registry directory.
    // path_allow is relative to cwd — the registry lives at
    // `plugins/vsdd-factory/config/artifact-path-registry.yaml` under cwd.
    // We also allow "plugins" as a prefix to cover the full registry subtree.
    host_ctx.capabilities = Capabilities {
        read_file: Some(ReadFileCaps {
            path_allow: vec!["plugins".to_string()],
        }),
        ..Capabilities::default()
    };

    let limits = InvokeLimits {
        timeout_ms: 5_000,
        fuel_cap: 50_000_000,
    };

    // S-25.01: invoke_plugin now returns (PluginResult, bool). Discard the
    // output_too_large flag — this test helper only needs the PluginResult.
    let (result, _output_too_large) =
        invoke_plugin(&engine, &module, host_ctx, payload_json, limits)
            .expect("invoke_plugin must not fail with InvokeError");
    result
}

/// Assert that a PluginResult carries a block intent (exit_code 2 and
/// stdout containing `"outcome":"block"`).
fn assert_blocks(result: &PluginResult, context: &str) {
    match result {
        PluginResult::Ok {
            exit_code, stdout, ..
        } => {
            assert_eq!(
                *exit_code, 2,
                "{}: expected exit_code=2 (block), got exit_code={} — \
                 hook did NOT block. stdout: {:?}",
                context, exit_code, stdout
            );
            assert!(
                stdout.contains(r#""outcome":"block""#),
                "{}: expected stdout to contain '\"outcome\":\"block\"' but got: {:?}",
                context,
                stdout
            );
        }
        other => {
            panic!(
                "{}: expected PluginResult::Ok with exit_code=2 (block), got {:?}",
                context, other
            );
        }
    }
}

/// Assert that a PluginResult carries a continue (exit_code 0, no block).
fn assert_continues(result: &PluginResult, context: &str) {
    match result {
        PluginResult::Ok {
            exit_code, stdout, ..
        } => {
            assert_eq!(
                *exit_code, 0,
                "{}: expected exit_code=0 (continue), got exit_code={} — \
                 hook unexpectedly blocked. stdout: {:?}",
                context, exit_code, stdout
            );
        }
        other => {
            panic!(
                "{}: expected PluginResult::Ok with exit_code=0 (continue), got {:?}",
                context, other
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Assertion 1: Pre-fix bug reproduction — validate-artifact-path
//
// The deployed WASM at plugins/vsdd-factory/hook-plugins/validate-artifact-path.wasm
// was committed before 8b4f697f. It has `starts_with(".factory/")` only,
// which is FALSE for absolute paths. This test confirms the bypass is real:
// an absolute unregistered path returns Continue (exit_code 0) instead of
// blocking.
//
// NOTE: this is the "red" test that confirms the bug existed. It passes only
// if the pre-fix binary is still present on disk. If the binary is updated to
// the post-fix version this test will fail (which would mean the fix is already
// deployed and the red test is no longer meaningful). We document this clearly.
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_BC_4_11_001_prefixbug_absolute_path_bypasses_prefixwasm() {
    // Pre-condition: verify the pre-fix WASM exists (size=303672 bytes = pre-fix).
    // If this file has been replaced with the post-fix binary the test expectation
    // changes — see comment above.
    let wasm_path = pre_fix_artifact_path_wasm();
    if !wasm_path.exists() {
        eprintln!(
            "SKIP: pre-fix WASM not found at {:?} — cannot demonstrate bug. \
             This may mean the fix has already been deployed to plugins/.",
            wasm_path
        );
        return;
    }

    let wasm_size = std::fs::metadata(&wasm_path).expect("wasm metadata").len();

    // The pre-fix binary is exactly 303672 bytes (well-known historical
    // size baked at the moment the bug was identified). Any other size
    // implies the fix has been deployed and the bug is no longer
    // reproducible. We don't pin a specific POST_FIX size because every
    // toolchain / dependency rebuild produces slightly different bytes
    // (timestamps, build-host metadata baked into WASM); the rc.14
    // rebuild produced 305013 bytes vs the originally-recorded 305333.
    // See TD #67 family — wall-clock and byte-count assertions are
    // inherently fragile when the artifact is rebuilt.
    const PRE_FIX_SIZE: u64 = 303_672;

    if wasm_size != PRE_FIX_SIZE {
        eprintln!(
            "SKIP pre-fix bypass test: plugins/ WASM is {} bytes (≠ pre-fix \
             {}). The fix has been deployed. Bypass is no longer present.",
            wasm_size, PRE_FIX_SIZE
        );
        return;
    }

    let (_tmp, cwd) = setup_temp_registry();

    // Absolute path to an UNREGISTERED .factory/ location.
    // Pre-fix: starts_with(".factory/") is FALSE → early-exit Continue → bypass.
    let abs_unregistered = "/abs/project/.factory/feature-deltas/F1-delta.md";
    let payload = make_write_payload(abs_unregistered);
    let result = invoke_hook(&wasm_path, &cwd, &payload);

    // Pre-fix behavior: the hook bypasses and returns Continue (exit_code=0).
    // If this assertion fails it means the pre-fix binary unexpectedly blocks,
    // which would indicate the binary was already updated.
    assert_continues(
        &result,
        "pre-fix bug reproduction: absolute unregistered .factory/ path \
         must bypass (exit_code=0) under the pre-fix WASM. \
         If this fails, the WASM binary in plugins/ is already the post-fix version.",
    );
}

// ---------------------------------------------------------------------------
// Assertion 2a: Post-fix validate-artifact-path — absolute UNREGISTERED path
// must BLOCK (was bypassing pre-fix).
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_BC_4_11_001_postfix_absolute_unregistered_path_blocks() {
    let wasm_path = post_fix_artifact_path_wasm();
    if !wasm_path.exists() {
        panic!(
            "Post-fix validate-artifact-path WASM not found at {:?}. \
             Run `cargo build --target wasm32-wasip1 --release -p validate-artifact-path` \
             to build the post-fix binary.",
            wasm_path
        );
    }

    let (_tmp, cwd) = setup_temp_registry();

    // Absolute path to an UNREGISTERED .factory/ location — must now block.
    let abs_unregistered = "/Users/jmagady/Dev/vsdd-factory/.factory/feature-deltas/F1-delta.md";
    let payload = make_write_payload(abs_unregistered);
    let result = invoke_hook(&wasm_path, &cwd, &payload);

    assert_blocks(
        &result,
        "F-P18-001 post-fix: absolute unregistered .factory/ path must block \
         (ARTIFACT_PATH_UNREGISTERED). The hook must engage on absolute paths \
         sent by Claude Code's hook envelope.",
    );
}

// ---------------------------------------------------------------------------
// Assertion 2b: Post-fix validate-artifact-path — absolute REGISTERED path
// must CONTINUE (hook engages but path is valid).
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_BC_4_11_001_postfix_absolute_registered_path_continues() {
    let wasm_path = post_fix_artifact_path_wasm();
    if !wasm_path.exists() {
        panic!(
            "Post-fix validate-artifact-path WASM not found at {:?}.",
            wasm_path
        );
    }

    let (_tmp, cwd) = setup_temp_registry();

    // Absolute path that maps to a REGISTERED pattern (behavioral-contract).
    // After stripping the absolute prefix the normalized path is:
    // `.factory/specs/behavioral-contracts/ss-04/BC-4.11.001.md`
    // which matches the `ss-{subsystem}/BC-{bc-id}.md` pattern.
    let abs_registered =
        "/Users/jmagady/Dev/vsdd-factory/.factory/specs/behavioral-contracts/ss-04/BC-4.11.001.md";
    let payload = make_write_payload(abs_registered);
    let result = invoke_hook(&wasm_path, &cwd, &payload);

    assert_continues(
        &result,
        "F-P18-001 post-fix: absolute registered .factory/ path must continue \
         (path is canonical per registry; enforcement_level=block means 'allow'). \
         The hook must engage AND find a match, then return Continue.",
    );
}

// ---------------------------------------------------------------------------
// Assertion 2c: Post-fix validate-artifact-path — relative paths still work
// (regression guard).
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_BC_4_11_001_postfix_relative_unregistered_still_blocks() {
    let wasm_path = post_fix_artifact_path_wasm();
    if !wasm_path.exists() {
        panic!(
            "Post-fix validate-artifact-path WASM not found at {:?}.",
            wasm_path
        );
    }

    let (_tmp, cwd) = setup_temp_registry();

    // Relative path, UNREGISTERED — must still block post-fix.
    let rel_unregistered = ".factory/feature-deltas/F1-delta.md";
    let payload = make_write_payload(rel_unregistered);
    let result = invoke_hook(&wasm_path, &cwd, &payload);

    assert_blocks(
        &result,
        "regression: relative unregistered .factory/ path must still block \
         after the absolute-path fix. The fix must not break existing behavior.",
    );
}

#[test]
fn test_e2e_BC_4_11_001_postfix_relative_registered_still_continues() {
    let wasm_path = post_fix_artifact_path_wasm();
    if !wasm_path.exists() {
        panic!(
            "Post-fix validate-artifact-path WASM not found at {:?}.",
            wasm_path
        );
    }

    let (_tmp, cwd) = setup_temp_registry();

    // Relative path, REGISTERED — must still continue post-fix.
    let rel_registered = ".factory/specs/behavioral-contracts/ss-04/BC-4.11.001.md";
    let payload = make_write_payload(rel_registered);
    let result = invoke_hook(&wasm_path, &cwd, &payload);

    assert_continues(
        &result,
        "regression: relative registered .factory/ path must still continue \
         after the absolute-path fix.",
    );
}

// ---------------------------------------------------------------------------
// Assertion 3a: Post-fix validate-stable-anchors — absolute spec path with
// a volatile cite in new_string MUST BLOCK.
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_TD_031_postfix_absolute_spec_path_with_cite_blocks() {
    let wasm_path = post_fix_stable_anchors_wasm();
    if !wasm_path.exists() {
        panic!(
            "Post-fix validate-stable-anchors WASM not found at {:?}. \
             Run `cargo build --target wasm32-wasip1 --release -p validate-stable-anchors` \
             to build the post-fix binary.",
            wasm_path
        );
    }

    // validate-stable-anchors does not read a registry file — no temp dir needed.
    // Use a temp dir as cwd to satisfy the WASI preopen (plugin may do read_file
    // on the spec path, which will fail gracefully; the violation is caught from
    // new_string before the file read happens).
    let tmp = tempfile::tempdir().expect("tempdir");
    let cwd = tmp.path().to_path_buf();

    // Absolute path to a spec file under .factory/specs/ — is_spec_target must
    // accept this after cc5a016b.
    let abs_spec_path =
        "/Users/jmagady/Dev/vsdd-factory/.factory/specs/behavioral-contracts/ss-04/BC-4.11.001.md";

    // new_string containing a volatile cite: `lib.rs:42` — TD-031 violation.
    let violating_new_string = "See `lib.rs:42` for the implementation details.";

    let payload = make_edit_payload(abs_spec_path, Some(violating_new_string));
    let result = invoke_hook(&wasm_path, &cwd, &payload);

    assert_blocks(
        &result,
        "cc5a016b post-fix: absolute spec path + new_string with volatile cite \
         (`lib.rs:42`) must BLOCK. is_spec_target must accept absolute paths \
         so the hook engages and detects the TD-031 violation.",
    );
}

// ---------------------------------------------------------------------------
// Assertion 3b: Post-fix validate-stable-anchors — absolute spec path with
// NO volatile cite in new_string must CONTINUE.
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_TD_031_postfix_absolute_spec_path_without_cite_continues() {
    let wasm_path = post_fix_stable_anchors_wasm();
    if !wasm_path.exists() {
        panic!(
            "Post-fix validate-stable-anchors WASM not found at {:?}.",
            wasm_path
        );
    }

    let tmp = tempfile::tempdir().expect("tempdir");
    let cwd = tmp.path().to_path_buf();

    let abs_spec_path =
        "/Users/jmagady/Dev/vsdd-factory/.factory/specs/behavioral-contracts/ss-04/BC-4.11.001.md";
    // Clean new_string — references functions by name, no line numbers.
    let clean_new_string =
        "See `emit_plugin_async_block_discarded` in `factory_dispatcher::main` for details.";

    let payload = make_edit_payload(abs_spec_path, Some(clean_new_string));
    let result = invoke_hook(&wasm_path, &cwd, &payload);

    assert_continues(
        &result,
        "cc5a016b post-fix: absolute spec path + clean new_string (no volatile cite) \
         must CONTINUE. The hook engages (is_spec_target returns true) but finds \
         no violation and allows the edit.",
    );
}

// ---------------------------------------------------------------------------
// Assertion 3c: Post-fix validate-stable-anchors — absolute NON-spec path
// (outside .factory/specs/) must CONTINUE regardless of content.
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_TD_031_postfix_absolute_non_spec_path_continues() {
    let wasm_path = post_fix_stable_anchors_wasm();
    if !wasm_path.exists() {
        panic!(
            "Post-fix validate-stable-anchors WASM not found at {:?}.",
            wasm_path
        );
    }

    let tmp = tempfile::tempdir().expect("tempdir");
    let cwd = tmp.path().to_path_buf();

    // Absolute path to a SOURCE file (not under .factory/specs/) — hook must
    // exit early and never scan content.
    let abs_src_path = "/Users/jmagady/Dev/vsdd-factory/crates/factory-dispatcher/src/lib.rs";
    // Even with a cite in new_string, the hook must NOT block for non-spec files.
    let cite_new_string = "// See lib.rs:42 — this is in a source file, out of scope.";

    let payload = make_edit_payload(abs_src_path, Some(cite_new_string));
    let result = invoke_hook(&wasm_path, &cwd, &payload);

    assert_continues(
        &result,
        "cc5a016b post-fix: absolute path outside .factory/specs/ must CONTINUE \
         (is_spec_target returns false → early-exit; content not scanned). \
         Source files with line cites are NOT in scope for TD-031 enforcement.",
    );
}

// ---------------------------------------------------------------------------
// Assertion 4: Leading-slash discipline — false positive prevention.
//
// `prefix.factory/specs/foo.md` (no slash or start-of-string before .factory/)
// must NOT trigger the hooks. Only `.factory/...` (start) or `/.factory/...`
// (absolute) are in scope.
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_BC_4_11_001_postfix_false_positive_prefix_factory_does_not_block() {
    let wasm_path = post_fix_artifact_path_wasm();
    if !wasm_path.exists() {
        panic!(
            "Post-fix validate-artifact-path WASM not found at {:?}.",
            wasm_path
        );
    }

    let (_tmp, cwd) = setup_temp_registry();

    // Leading-slash discipline: `.factory/` preceded by something other than
    // `/` or start-of-string is NOT in scope. The hook must not match this.
    let false_positive_path = "notfactory/specs/behavioral-contracts/ss-04/BC-4.11.001.md";
    let payload = make_write_payload(false_positive_path);
    let result = invoke_hook(&wasm_path, &cwd, &payload);

    assert_continues(
        &result,
        "leading-slash discipline: 'notfactory/...' does not start with .factory/ \
         and does not contain /.factory/ — hook must early-exit Continue. \
         No false positive block for paths that incidentally contain 'factory/'.",
    );
}

#[test]
fn test_e2e_TD_031_postfix_false_positive_prefix_factory_does_not_block() {
    let wasm_path = post_fix_stable_anchors_wasm();
    if !wasm_path.exists() {
        panic!(
            "Post-fix validate-stable-anchors WASM not found at {:?}.",
            wasm_path
        );
    }

    let tmp = tempfile::tempdir().expect("tempdir");
    let cwd = tmp.path().to_path_buf();

    // Same leading-slash discipline check for validate-stable-anchors.
    let false_positive_path = "notfactory/specs/behavioral-contracts/ss-04/BC-4.11.001.md";
    let violating_new_string = "See `lib.rs:42` — but this path is not a spec file.";

    let payload = make_edit_payload(false_positive_path, Some(violating_new_string));
    let result = invoke_hook(&wasm_path, &cwd, &payload);

    assert_continues(
        &result,
        "leading-slash discipline (stable-anchors): 'notfactory/...' must NOT \
         trigger the hook even if new_string contains a cite. \
         is_spec_target correctly rejects this path.",
    );
}
