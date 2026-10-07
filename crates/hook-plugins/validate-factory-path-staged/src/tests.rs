// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
// Test-local recording helper uses Rc<RefCell<Vec<(String, Vec<String>)>>> for the
// injected exec_subprocess call-recorder; matches the established codebase convention
// for test files (see e.g. validate-artifact-path/src/tests.rs, validate-stable-anchors/
// src/tests.rs) rather than a production-code type alias for a test-only helper type.
#![allow(clippy::type_complexity)]
//! Unit tests for validate-factory-path-staged (BC-4.16.002 / BC-5.38.001).
//!
//! Exercises the production functions declared in `lib.rs` via injectable
//! callbacks (`HookCallbacks`) and direct pure-function calls.
//! `is_factory_path`, `find_staged_factory_path`, and `hook_logic` are fully
//! implemented (shipped in S-25.04). `is_product_branch` is a real 1-line
//! implementation and is deliberately NOT given a standalone unit test here
//! (a direct test of an already-trivial 1-line function would add no
//! meaningful coverage); its behavior is instead exercised through the
//! `hook_logic` integration tests below, which cover every branch outcome
//! (block on a product branch / pass on `factory-artifacts`) that depends on
//! `is_product_branch`'s result.
//!
//! # BC traces (BC-4.16.002)
//! - PC1: detect `.factory/` path staged on a product branch (block,
//!   `FactoryPathStagedOnProductBranch`)
//! - PC2: pass — no `.factory/` path staged, or branch is `factory-artifacts`
//! - PC3: INDETERMINATE trigger on cannot-complete (AC-001 closure criterion;
//!   dispatcher-side — verified here via the `hooks-registry.toml` structural
//!   reachability gate, not via `hook_logic`'s own Rust control flow)
//! - PC4: fail-open on branch-detection failure (mirrors BC-4.16.001
//!   Invariant 3)
//! - PC5: advisory-only on non-resource-exhaustion crash (`on_error =
//!   "continue"`; dispatcher-side, verified via the registry gate)
//! - Invariant 4: case-insensitive `.factory/` path-matching predicate,
//!   reused verbatim from BC-4.16.001
//! - Invariant 7: unconditional BROAD trigger scope — no command-text
//!   pre-filter
//! - EC-001..EC-008, T-1..T-9: see inline test-level BC trace comments

use std::cell::{Cell, RefCell};
use std::panic;
use std::rc::Rc;

use serde_json::json;
use vsdd_hook_sdk::{HookPayload, HookResult};

use crate::{
    BRANCH_DETECTION_MAX_OUTPUT_BYTES, FACTORY_PATH_STAGED_ON_PRODUCT_BRANCH, HookCallbacks,
    STAGED_PATH_LISTING_MAX_OUTPUT_BYTES, find_staged_factory_path, hook_logic, is_factory_path,
    max_output_bytes_for,
};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// A single mocked `exec_subprocess` return value: `(exit_code, stdout, stderr)`.
type ExecResult = Result<(i32, String, String), String>;

fn exec_ok(exit_code: i32, stdout: &str, stderr: &str) -> ExecResult {
    Ok((exit_code, stdout.to_string(), stderr.to_string()))
}

fn exec_err(message: &str) -> ExecResult {
    Err(message.to_string())
}

/// Builds a completed `PostToolUse` `Bash` payload with the given command
/// text in `tool_input.command`. BC-4.16.002 Precondition 1: the plugin
/// fires strictly after the Bash command has completed.
fn make_post_tool_use_payload(command: &str) -> HookPayload {
    let v = json!({
        "event_name": "PostToolUse",
        "session_id": "test-session",
        "dispatcher_trace_id": "test-trace",
        "tool_name": "Bash",
        "tool_input": { "command": command },
        "tool_response": { "exit_code": 0 }
    });
    serde_json::from_value(v).expect("fixture must deserialize")
}

/// Builds a completed `PostToolUse` `Bash` payload with NO `command` field
/// in `tool_input` at all (malformed/absent). Used to prove Invariant 7:
/// the unconditional check does not depend on being able to read command
/// text — a narrow text-gated implementation would need `command` to decide
/// whether to run at all; the BROAD implementation must not.
fn make_post_tool_use_payload_no_command() -> HookPayload {
    let v = json!({
        "event_name": "PostToolUse",
        "session_id": "test-session",
        "dispatcher_trace_id": "test-trace",
        "tool_name": "Bash",
        "tool_input": {},
        "tool_response": { "exit_code": 0 }
    });
    serde_json::from_value(v).expect("fixture must deserialize")
}

/// Runs `hook_logic` with a mocked two-call `exec_subprocess`: the FIRST
/// call (staged-path listing, `git diff --cached --name-only`) returns
/// `diff_result`; the SECOND call (branch detection, `git branch
/// --show-current`) returns `branch_result`. Mirrors BC-4.16.002
/// Precondition 3's documented call order (diff, then branch).
fn run_hook(
    payload: HookPayload,
    diff_result: ExecResult,
    branch_result: ExecResult,
) -> std::thread::Result<HookResult> {
    let call_index = Cell::new(0u32);
    panic::catch_unwind(move || {
        hook_logic(
            payload,
            HookCallbacks {
                exec_subprocess: move |_bin: &str, _args: &[&str]| {
                    let n = call_index.get();
                    call_index.set(n + 1);
                    if n == 0 {
                        diff_result.clone()
                    } else {
                        branch_result.clone()
                    }
                },
                emit_event: |_, _| {},
                log: |_, _| {},
            },
        )
    })
}

/// Like `run_hook`, but also records every `exec_subprocess` invocation
/// (binary + args) so tests can assert BOTH calls were actually made,
/// unconditionally, regardless of the triggering command's own text
/// (BC-4.16.002 Precondition 2 / Invariant 7 BROAD scope).
fn run_hook_recording_calls(
    payload: HookPayload,
    diff_result: ExecResult,
    branch_result: ExecResult,
) -> (std::thread::Result<HookResult>, Vec<(String, Vec<String>)>) {
    let calls: Rc<RefCell<Vec<(String, Vec<String>)>>> = Rc::new(RefCell::new(Vec::new()));
    let calls_for_closure = Rc::clone(&calls);
    let call_index = Cell::new(0u32);
    let result = panic::catch_unwind(panic::AssertUnwindSafe(move || {
        hook_logic(
            payload,
            HookCallbacks {
                exec_subprocess: move |bin: &str, args: &[&str]| {
                    calls_for_closure.borrow_mut().push((
                        bin.to_string(),
                        args.iter().map(|s| s.to_string()).collect(),
                    ));
                    let n = call_index.get();
                    call_index.set(n + 1);
                    if n == 0 {
                        diff_result.clone()
                    } else {
                        branch_result.clone()
                    }
                },
                emit_event: |_, _| {},
                log: |_, _| {},
            },
        )
    }));
    let recorded = calls.borrow().clone();
    (result, recorded)
}

/// Walk up from `CARGO_MANIFEST_DIR` until `hooks-registry.toml` is found.
fn hooks_registry_toml_path() -> std::path::PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR must be set during cargo test");
    let mut dir = std::path::PathBuf::from(&manifest_dir);
    loop {
        let candidate = dir.join("plugins/vsdd-factory/hooks-registry.toml");
        if candidate.exists() {
            return candidate;
        }
        if !dir.pop() {
            panic!(
                "could not locate plugins/vsdd-factory/hooks-registry.toml walking up from \
                 CARGO_MANIFEST_DIR ({manifest_dir})"
            );
        }
    }
}

/// Extracts the `[[hooks]]` stanza whose `name = "<name>"` line matches, as
/// a raw text slice from that `[[hooks]]` marker up to (but not including)
/// the next `[[hooks]]` marker or EOF. Returns `None` if no stanza with that
/// name exists (the RED-gate case before implementation registers the
/// entry).
fn extract_hook_stanza<'a>(content: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("name = \"{name}\"");
    let name_idx = content.find(&needle)?;
    let stanza_start = content[..name_idx].rfind("[[hooks]]")?;
    let after = &content[stanza_start..];
    // Skip the "[[hooks]]" marker itself before searching for the NEXT one.
    let marker_len = "[[hooks]]".len();
    let rel_end = after[marker_len..]
        .find("[[hooks]]")
        .map(|i| i + marker_len)
        .unwrap_or(after.len());
    Some(&after[..rel_end])
}

// ---------------------------------------------------------------------------
// is_factory_path (BC-4.16.002 PC1 / Invariant 4 — reused verbatim predicate)
// ---------------------------------------------------------------------------

#[test]
fn test_bc4_16_002_is_factory_path_matches_prefix() {
    assert!(is_factory_path(".factory/STATE.md"));
}

#[test]
fn test_bc4_16_002_is_factory_path_matches_prefix_case_insensitive() {
    // BC-4.16.002 PC1: "e.g., .Factory/STATE.md matches"
    assert!(is_factory_path(".Factory/STATE.md"));
    assert!(is_factory_path(".FACTORY/STATE.md"));
}

#[test]
fn test_bc4_16_002_is_factory_path_matches_interior_path_component() {
    // BC-4.16.002 PC1: "or containing /.factory/ as a path component"
    assert!(is_factory_path("some/nested/.factory/STATE.md"));
}

#[test]
fn test_bc4_16_002_is_factory_path_matches_interior_component_case_insensitive() {
    assert!(is_factory_path("some/nested/.Factory/STATE.md"));
}

#[test]
fn test_bc4_16_002_is_factory_path_rejects_non_factory_path() {
    assert!(!is_factory_path("src/main.rs"));
    assert!(!is_factory_path("crates/hook-sdk/src/lib.rs"));
}

#[test]
fn test_bc4_16_002_is_factory_path_rejects_missing_leading_dot() {
    // "factory/" (no leading dot) is a different directory entirely.
    assert!(!is_factory_path("factory/STATE.md"));
}

#[test]
fn test_bc4_16_002_is_factory_path_rejects_similar_but_distinct_dir_name() {
    // "myfactory/" is not ".factory/" — must not false-positive on substring.
    assert!(!is_factory_path("myfactory/STATE.md"));
    assert!(!is_factory_path("src/myfactory/STATE.md"));
}

// ---------------------------------------------------------------------------
// find_staged_factory_path (BC-4.16.002 PC1 / PC2 / EC-002 / EC-007 — BROAD
// unconditional scan of git diff --cached --name-only stdout)
// ---------------------------------------------------------------------------

#[test]
fn test_bc4_16_002_find_staged_factory_path_returns_match_single_line() {
    assert_eq!(
        find_staged_factory_path(".factory/STATE.md\n"),
        Some(".factory/STATE.md".to_string())
    );
}

#[test]
fn test_bc4_16_002_find_staged_factory_path_returns_match_among_multiple_lines() {
    // BC-4.16.002 T-9-style: .factory/ path is not the only staged path.
    let stdout = "src/main.rs\nCargo.toml\n.factory/stories/S-25.04.md\ncrates/foo/lib.rs\n";
    assert_eq!(
        find_staged_factory_path(stdout),
        Some(".factory/stories/S-25.04.md".to_string())
    );
}

#[test]
fn test_bc4_16_002_find_staged_factory_path_returns_none_when_no_match() {
    // BC-4.16.002 EC-002: staging entirely outside .factory/.
    let stdout = "src/main.rs\nCargo.toml\nREADME.md\n";
    assert_eq!(find_staged_factory_path(stdout), None);
}

#[test]
fn test_bc4_16_002_find_staged_factory_path_returns_none_for_empty_stdout() {
    // BC-4.16.002 EC-007: nothing staged at all.
    assert_eq!(find_staged_factory_path(""), None);
}

#[test]
fn test_bc4_16_002_find_staged_factory_path_case_insensitive() {
    assert_eq!(
        find_staged_factory_path(".Factory/STATE.md\n"),
        Some(".Factory/STATE.md".to_string())
    );
}

#[test]
fn test_bc4_16_002_find_staged_factory_path_ignores_blank_lines() {
    let stdout = "\n\n.factory/STATE.md\n\n";
    assert_eq!(
        find_staged_factory_path(stdout),
        Some(".factory/STATE.md".to_string())
    );
}

// ---------------------------------------------------------------------------
// hook_logic PC1 — detect + block (T-1, T-9, case-insensitivity, block msg)
// ---------------------------------------------------------------------------

#[test]
fn test_bc4_16_002_t1_blocks_factory_path_staged_on_develop() {
    // BC-4.16.002 T-1: `.factory/STATE.md` staged, `git add .factory/STATE.md`
    // just completed, branch=develop → DETECTED PC1.
    let payload = make_post_tool_use_payload("git add .factory/STATE.md");
    let result = run_hook(
        payload,
        exec_ok(0, ".factory/STATE.md\n", ""),
        exec_ok(0, "develop", ""),
    );
    assert!(
        result.is_ok(),
        "T-1 / BC-4.16.002 PC1: hook_logic panicked for '.factory/STATE.md' staged on \
         develop. Must return HookResult::Block with FactoryPathStagedOnProductBranch."
    );
    if let Ok(hook_result) = result {
        assert_eq!(
            hook_result.exit_code(),
            2,
            "T-1 / BC-4.16.002 PC1: must exit 2 (block_intent=true). Got exit code {}.",
            hook_result.exit_code()
        );
        match &hook_result {
            HookResult::Block { reason } => {
                assert!(
                    reason.contains(FACTORY_PATH_STAGED_ON_PRODUCT_BRANCH),
                    "T-1 / BC-4.16.002 PC1 Error variant: block reason must contain \
                     '{}'. Got: '{}'",
                    FACTORY_PATH_STAGED_ON_PRODUCT_BRANCH,
                    reason
                );
                assert!(
                    reason.contains("develop"),
                    "T-1 / BC-4.16.002 PC1: block reason must include the branch name \
                     'develop'. Got: '{}'",
                    reason
                );
                assert!(
                    reason.contains("factory-artifacts"),
                    "T-1 / BC-4.16.002 PC1: canonical block message must mention the \
                     'factory-artifacts' worktree as the fix guidance. Got: '{}'",
                    reason
                );
            }
            other => panic!(
                "T-1 / BC-4.16.002 PC1: expected HookResult::Block, got {:?}",
                other
            ),
        }
    }
}

#[test]
fn test_bc4_16_002_t9_blocks_factory_path_staged_on_release_branch() {
    // BC-4.16.002 T-9: `.factory/stories/S-25.04.md` staged, branch =
    // release/v1.0.0-rc.25 (a product branch; mirrors BC-4.16.001 T-5).
    let payload = make_post_tool_use_payload("git add .factory/stories/S-25.04.md");
    let result = run_hook(
        payload,
        exec_ok(0, ".factory/stories/S-25.04.md\n", ""),
        exec_ok(0, "release/v1.0.0-rc.25", ""),
    );
    assert!(
        result.is_ok(),
        "T-9 / BC-4.16.002 PC1: hook_logic panicked for release branch. Production \
         unimplemented."
    );
    if let Ok(hook_result) = result {
        assert_eq!(
            hook_result.exit_code(),
            2,
            "T-9 / BC-4.16.002 PC1: release/* is a product branch — must exit 2."
        );
    }
}

#[test]
fn test_bc4_16_002_pc1_blocks_case_insensitive_factory_path_on_develop() {
    // BC-4.16.002 PC1: "e.g., .Factory/STATE.md matches" — case-insensitive
    // match reused verbatim from BC-4.16.001 Invariant 4.
    let payload = make_post_tool_use_payload("python3 fix_index.py");
    let result = run_hook(
        payload,
        exec_ok(0, ".Factory/STATE.md\n", ""),
        exec_ok(0, "develop", ""),
    );
    assert!(
        result.is_ok(),
        "BC-4.16.002 PC1: hook_logic panicked for case-varied '.Factory/STATE.md'. \
         Production unimplemented."
    );
    if let Ok(hook_result) = result {
        assert_eq!(
            hook_result.exit_code(),
            2,
            "BC-4.16.002 PC1: '.Factory/STATE.md' (mixed case) on develop must exit 2 \
             (case-insensitive predicate, Invariant 4)."
        );
    }
}

#[test]
fn test_bc4_16_002_pc1_item3_does_not_claim_automatic_unstage() {
    // BC-4.16.002 PC1 item 3: the plugin is detective, not preventive — the
    // block reason must not falsely claim the path was auto-unstaged.
    let payload = make_post_tool_use_payload("git add .factory/STATE.md");
    let result = run_hook(
        payload,
        exec_ok(0, ".factory/STATE.md\n", ""),
        exec_ok(0, "develop", ""),
    );
    assert!(
        result.is_ok(),
        "BC-4.16.002 PC1 item 3: hook_logic panicked. Production unimplemented."
    );
    if let Ok(HookResult::Block { reason }) = result {
        assert!(
            !reason
                .to_ascii_lowercase()
                .contains("automatically unstaged")
                && !reason.to_ascii_lowercase().contains("reverted"),
            "BC-4.16.002 PC1 item 3 / Invariant 6: block reason must NOT claim the path \
             was automatically unstaged or reverted — a PostToolUse detective check \
             cannot retroactively undo a completed git operation. Got: '{}'",
            reason
        );
    }
}

// ---------------------------------------------------------------------------
// hook_logic PC2 — pass (T-2, T-3, T-8, EC-002, EC-006)
// ---------------------------------------------------------------------------

#[test]
fn test_bc4_16_002_t2_passes_factory_path_staged_on_factory_artifacts() {
    // BC-4.16.002 T-2: `.factory/STATE.md` staged, branch=factory-artifacts
    // → PASSED PC2.
    let payload = make_post_tool_use_payload("git add .factory/STATE.md");
    let result = run_hook(
        payload,
        exec_ok(0, ".factory/STATE.md\n", ""),
        exec_ok(0, "factory-artifacts", ""),
    );
    assert!(
        result.is_ok(),
        "T-2 / BC-4.16.002 PC2: hook_logic panicked for factory-artifacts branch. \
         Production unimplemented."
    );
    if let Ok(hook_result) = result {
        assert_eq!(
            hook_result,
            HookResult::Continue,
            "T-2 / BC-4.16.002 PC2: .factory/ staged on factory-artifacts must Continue. \
             Got: {:?}",
            hook_result
        );
    }
}

#[test]
fn test_bc4_16_002_t3_passes_no_factory_path_staged_on_feature_branch() {
    // BC-4.16.002 T-3: `git add src/lib.rs`, branch=feature/S-25.04 →
    // PASSED PC2 (no .factory/ path).
    let payload = make_post_tool_use_payload("git add src/lib.rs");
    let result = run_hook(
        payload,
        exec_ok(0, "src/lib.rs\n", ""),
        exec_ok(0, "feature/S-25.04", ""),
    );
    assert!(
        result.is_ok(),
        "T-3 / BC-4.16.002 PC2: hook_logic panicked. Production unimplemented."
    );
    if let Ok(hook_result) = result {
        assert_eq!(
            hook_result,
            HookResult::Continue,
            "T-3 / BC-4.16.002 PC2: no .factory/ path staged must Continue."
        );
    }
}

#[test]
fn test_bc4_16_002_t8_passes_nothing_staged_unrelated_command() {
    // BC-4.16.002 T-8: `npm test`, nothing staged, branch=develop →
    // PASSED PC2 (unconditional check ran, found nothing relevant — NOT a
    // skipped fast-pass).
    let payload = make_post_tool_use_payload("npm test");
    let result = run_hook(payload, exec_ok(0, "", ""), exec_ok(0, "develop", ""));
    assert!(
        result.is_ok(),
        "T-8 / BC-4.16.002 PC2: hook_logic panicked for 'npm test' with nothing staged. \
         Production unimplemented."
    );
    if let Ok(hook_result) = result {
        assert_eq!(
            hook_result,
            HookResult::Continue,
            "T-8 / BC-4.16.002 PC2: nothing staged must Continue."
        );
    }
}

#[test]
fn test_bc4_16_002_ec006_deep_factory_path_on_factory_artifacts_passes() {
    // BC-4.16.002 EC-006: .factory/ path staged, branch=factory-artifacts
    // (legitimate state-manager commit) → PC2 passes unconditionally.
    let payload = make_post_tool_use_payload("git add .factory/cycles/pass-1/burst-log.md");
    let result = run_hook(
        payload,
        exec_ok(0, ".factory/cycles/pass-1/burst-log.md\n", ""),
        exec_ok(0, "factory-artifacts", ""),
    );
    assert!(
        result.is_ok(),
        "EC-006 / BC-4.16.002 PC2: hook_logic panicked. Production unimplemented."
    );
    if let Ok(hook_result) = result {
        assert_eq!(
            hook_result,
            HookResult::Continue,
            "EC-006 / BC-4.16.002 PC2: deep .factory/ path on factory-artifacts must \
             Continue."
        );
    }
}

// ---------------------------------------------------------------------------
// BROAD trigger scope (Precondition 2 / Invariant 7) — T-4, EC-002, EC-007
//
// The exact case this closure exists to close: staging performed via a
// non-`git add`/`stage`-text command (git plumbing, wrapper script, alias)
// is STILL detected, because the check runs against actual git index state,
// not against the triggering command's payload text. A narrow, text-gated
// re-implementation of BC-4.16.001's own `git\s+(add|stage)` detector against
// THIS plugin's payload would fail every test in this section.
// ---------------------------------------------------------------------------

#[test]
fn test_bc4_16_002_t4_broad_scope_detects_staging_via_non_git_add_text_command() {
    // BC-4.16.002 T-4: `.factory/` staged via a wrapper script
    // (`python3 fix_index.py`) whose command text contains no recognizable
    // `git add`/`git stage` substring — DETECTED anyway (BROAD unconditional
    // check catches it; this is exactly the case a text-gated internal
    // filter would have missed).
    let payload = make_post_tool_use_payload("python3 fix_index.py");
    let result = run_hook(
        payload,
        exec_ok(0, ".factory/STATE.md\n", ""),
        exec_ok(0, "develop", ""),
    );
    assert!(
        result.is_ok(),
        "T-4 / BC-4.16.002 Precondition 2 / Invariant 7: hook_logic panicked for a \
         non-git-add-text wrapper-script command that staged .factory/STATE.md via git \
         plumbing. BROAD scope requires detection regardless of command text. Production \
         unimplemented."
    );
    if let Ok(hook_result) = result {
        assert_eq!(
            hook_result.exit_code(),
            2,
            "T-4 / BC-4.16.002 Invariant 7: BROAD unconditional scope MUST detect \
             .factory/ staged via 'python3 fix_index.py' (no git-add-text substring) on \
             develop — exit 2. A narrow text-gated internal filter would incorrectly \
             Continue here because the triggering command text contains no 'git', 'add', \
             or 'stage' substring. Got exit code {}.",
            hook_result.exit_code()
        );
    }
}

#[test]
fn test_bc4_16_002_ec007_check_still_executes_for_completely_unrelated_command() {
    // BC-4.16.002 EC-007: the Bash command was not git-related at all
    // (e.g., `npm test`) — the unconditional check STILL executes (both
    // exec_subprocess calls issued); this resolves via PC2 only because
    // nothing relevant is staged, NOT because the command text was
    // recognized as irrelevant and the check was skipped.
    let payload = make_post_tool_use_payload("npm test");
    let (result, calls) =
        run_hook_recording_calls(payload, exec_ok(0, "", ""), exec_ok(0, "develop", ""));
    assert!(
        result.is_ok(),
        "EC-007 / BC-4.16.002 Precondition 2: hook_logic panicked for 'npm test'. \
         Production unimplemented."
    );
    assert_eq!(
        calls.len(),
        2,
        "EC-007 / BC-4.16.002 Invariant 7: exactly 2 exec_subprocess calls (staged-path \
         listing + branch detection) MUST be issued for 'npm test' — an entirely \
         unrelated, non-git command. A text-gated fast-pass-skip implementation would \
         issue 0 calls. Got {} call(s): {:?}",
        calls.len(),
        calls
    );
}

#[test]
fn test_bc4_16_002_precondition3_exec_subprocess_call_shape_and_order() {
    // BC-4.16.002 Precondition 3: exactly two calls, in order — staged-path
    // listing (`git diff --cached --name-only`) THEN branch detection
    // (`git branch --show-current`).
    let payload = make_post_tool_use_payload("git add .factory/STATE.md");
    let (result, calls) = run_hook_recording_calls(
        payload,
        exec_ok(0, ".factory/STATE.md\n", ""),
        exec_ok(0, "develop", ""),
    );
    assert!(
        result.is_ok(),
        "BC-4.16.002 Precondition 3: hook_logic panicked. Production unimplemented."
    );
    assert_eq!(
        calls,
        vec![
            (
                "git".to_string(),
                vec![
                    "diff".to_string(),
                    "--cached".to_string(),
                    "--name-only".to_string()
                ]
            ),
            (
                "git".to_string(),
                vec!["branch".to_string(), "--show-current".to_string()]
            ),
        ],
        "BC-4.16.002 Precondition 3: exec_subprocess must be called exactly twice, in \
         order: 'git diff --cached --name-only' then 'git branch --show-current'. Got: \
         {:?}",
        calls
    );
}

#[test]
fn test_bc4_16_002_invariant7_ignores_malformed_or_missing_tool_input() {
    // BC-4.16.002 Invariant 7 / Precondition 2: the check does not depend on
    // being able to read `tool_input.command` at all — it is unconditional.
    // A malformed/absent command field must not cause the check to be
    // skipped or to panic differently than the normal path.
    let payload = make_post_tool_use_payload_no_command();
    let (result, calls) = run_hook_recording_calls(
        payload,
        exec_ok(0, ".factory/STATE.md\n", ""),
        exec_ok(0, "develop", ""),
    );
    assert!(
        result.is_ok(),
        "BC-4.16.002 Invariant 7: hook_logic panicked when tool_input.command was absent. \
         The check must run unconditionally without needing command text at all. \
         Production unimplemented."
    );
    if let Ok(hook_result) = result {
        assert_eq!(
            hook_result.exit_code(),
            2,
            "BC-4.16.002 Invariant 7: absent tool_input.command must not suppress \
             detection — .factory/STATE.md staged on develop must still block."
        );
    }
    assert_eq!(
        calls.len(),
        2,
        "BC-4.16.002 Invariant 7: both exec_subprocess calls must be issued even when \
         tool_input.command is absent/malformed. Got {} call(s).",
        calls.len()
    );
}

#[test]
fn test_bc4_16_002_ec002_broad_check_runs_but_no_factory_path_found() {
    // BC-4.16.002 EC-002: `git add src/main.rs` completes — the
    // unconditional check still runs, finds no .factory/ path → PC2 passes;
    // no block, no marker.
    let payload = make_post_tool_use_payload("git add src/main.rs");
    let (result, calls) = run_hook_recording_calls(
        payload,
        exec_ok(0, "src/main.rs\n", ""),
        exec_ok(0, "develop", ""),
    );
    assert!(
        result.is_ok(),
        "EC-002 / BC-4.16.002 PC2: hook_logic panicked. Production unimplemented."
    );
    if let Ok(hook_result) = result {
        assert_eq!(
            hook_result,
            HookResult::Continue,
            "EC-002 / BC-4.16.002 PC2: non-.factory/ staging must Continue."
        );
    }
    assert_eq!(
        calls.len(),
        2,
        "EC-002 / BC-4.16.002 Invariant 7: unconditional check must still issue both \
         exec_subprocess calls even for an ordinary non-.factory/ git add."
    );
}

// ---------------------------------------------------------------------------
// PC4 — fail-open on branch-detection failure (T-5, EC-005, Invariant 3)
// ---------------------------------------------------------------------------

#[test]
fn test_bc4_16_002_t5_fail_open_on_detached_head() {
    // BC-4.16.002 T-5: branch detection fails (detached HEAD) → PASSED
    // fail-open per PC4, regardless of the triggering command.
    let payload = make_post_tool_use_payload("git add .factory/STATE.md");
    let result = run_hook(
        payload,
        exec_ok(0, ".factory/STATE.md\n", ""),
        exec_ok(128, "", "fatal: not a git repository"),
    );
    assert!(
        result.is_ok(),
        "T-5 / BC-4.16.002 PC4: hook_logic panicked when branch detection failed \
         (non-zero exit). BC-4.16.002 Invariant 3: uncertain branch state is NOT a \
         blocking condition — must fail-open (Continue). Production unimplemented."
    );
    if let Ok(hook_result) = result {
        assert_eq!(
            hook_result,
            HookResult::Continue,
            "T-5 / BC-4.16.002 PC4: branch-detection failure (non-zero exit) must \
             fail-open to Continue, even though a .factory/ path IS staged. Got: {:?}",
            hook_result
        );
    }
}

#[test]
fn test_bc4_16_002_pc4_fail_open_on_exec_subprocess_err() {
    // git unavailable — exec_subprocess itself returns Err for the branch
    // detection call.
    let payload = make_post_tool_use_payload("git add .factory/STATE.md");
    let result = run_hook(
        payload,
        exec_ok(0, ".factory/STATE.md\n", ""),
        exec_err("git: command not found"),
    );
    assert!(
        result.is_ok(),
        "BC-4.16.002 PC4 / Invariant 3: hook_logic panicked when exec_subprocess \
         returned Err for branch detection. Must fail-open. Production unimplemented."
    );
    if let Ok(hook_result) = result {
        assert_eq!(
            hook_result,
            HookResult::Continue,
            "BC-4.16.002 PC4: exec_subprocess Err on branch detection must fail-open to \
             Continue. Got: {:?}",
            hook_result
        );
    }
}

#[test]
fn test_bc4_16_002_pc4_fail_open_on_empty_branch_output() {
    // Detached HEAD sometimes manifests as exit 0 with empty stdout rather
    // than a non-zero exit — must also fail-open (mirrors BC-4.16.001
    // Invariant 3 handling of this same edge case).
    let payload = make_post_tool_use_payload("git add .factory/STATE.md");
    let result = run_hook(
        payload,
        exec_ok(0, ".factory/STATE.md\n", ""),
        exec_ok(0, "", ""),
    );
    assert!(
        result.is_ok(),
        "BC-4.16.002 PC4: hook_logic panicked for empty branch-detection stdout \
         (detached HEAD). Production unimplemented."
    );
    if let Ok(hook_result) = result {
        assert_eq!(
            hook_result,
            HookResult::Continue,
            "BC-4.16.002 PC4: empty branch output (detached HEAD) must fail-open to \
             Continue. Got: {:?}",
            hook_result
        );
    }
}

#[test]
fn test_bc4_16_002_ec005_branch_detection_failure_is_advisory_not_blocking() {
    // BC-4.16.002 EC-005: branch detection fails during the post-hoc check
    // → fail-open (PC4); advisory warning only, mirrors BC-4.16.001 EC-006.
    let payload = make_post_tool_use_payload("git add .factory/STATE.md");
    let result = run_hook(
        payload,
        exec_ok(0, ".factory/STATE.md\n", ""),
        exec_err("git: command not found"),
    );
    assert!(
        result.is_ok(),
        "EC-005 / BC-4.16.002 PC4: hook_logic panicked. Production unimplemented."
    );
    if let Ok(hook_result) = result {
        assert_ne!(
            hook_result.exit_code(),
            2,
            "EC-005 / BC-4.16.002 PC4: branch-detection failure must never produce a \
             blocking outcome (exit 2)."
        );
    }
}

// ---------------------------------------------------------------------------
// PC6 — fail-open on staged-path-listing failure (T-10, EC-009, Invariant 9)
// ---------------------------------------------------------------------------

#[test]
fn test_bc4_16_002_t10_fail_open_on_staged_path_listing_non_zero_exit() {
    // BC-4.16.002 T-10: the staged-path-listing call (`git diff --cached
    // --name-only`) fails with a non-zero exit → PASSED fail-open per PC6,
    // mirroring T-5's fail-open style but applied to the FIRST
    // exec_subprocess call instead of the second (branch detection). The
    // branch-detection call still runs unconditionally (Invariant 7) and
    // reports an ordinary product branch, so this test isolates the
    // staged-path-listing failure arm specifically: if `find_staged_factory_path`
    // were reached with a bogus stdout instead of short-circuiting to `None`,
    // the branch below (a product branch) would incorrectly block.
    // Note: stdout is a REAL staged `.factory/` path, not empty. This makes
    // the fixture mutation-resistant: if the exit-code fail-open guard were
    // buggy or deleted and the code fell through to parsing `stdout`
    // unconditionally, `find_staged_factory_path` would detect
    // `.factory/STATE.md` as staged and (branch "develop" being a product
    // branch) the hook would incorrectly return `HookResult::Block` instead
    // of `Continue` — causing this test to fail and catch the regression.
    // A blank stdout could not distinguish "correctly fail-open on non-zero
    // exit" from "trivially passes because there was nothing to detect".
    let payload = make_post_tool_use_payload("git add .factory/STATE.md");
    let result = run_hook(
        payload,
        exec_ok(128, ".factory/STATE.md\n", "fatal: not a git repository"),
        exec_ok(0, "develop", ""),
    );
    assert!(
        result.is_ok(),
        "T-10 / BC-4.16.002 PC6: hook_logic panicked when the staged-path-listing call \
         failed (non-zero exit). BC-4.16.002 Invariant 9: an inconclusive staged-path \
         answer is NOT a blocking condition — must fail-open (Continue)."
    );
    if let Ok(hook_result) = result {
        assert_eq!(
            hook_result,
            HookResult::Continue,
            "T-10 / BC-4.16.002 PC6: staged-path-listing failure (non-zero exit) must \
             fail-open to Continue, even on a product branch. Got: {:?}",
            hook_result
        );
    }
}

#[test]
fn test_bc4_16_002_pc6_fail_open_on_staged_path_listing_exec_subprocess_err() {
    // BC-4.16.002 EC-009: the `exec_subprocess` call itself returns a host
    // `Err` (e.g. subprocess spawn failure) for the staged-path-listing call
    // → fail-open per PC6 / Invariant 9. Distinct from PC3's fuel-exhaustion/
    // epoch-timeout/OutputTooLarge cannot-complete path — this is a clean,
    // completed `PluginResult::Ok` with an inconclusive answer.
    let payload = make_post_tool_use_payload("git add .factory/STATE.md");
    let result = run_hook(
        payload,
        exec_err("git: command not found"),
        exec_ok(0, "develop", ""),
    );
    assert!(
        result.is_ok(),
        "BC-4.16.002 PC6 / Invariant 9: hook_logic panicked when exec_subprocess \
         returned Err for the staged-path-listing call. Must fail-open."
    );
    if let Ok(hook_result) = result {
        assert_eq!(
            hook_result,
            HookResult::Continue,
            "BC-4.16.002 PC6: exec_subprocess Err on staged-path-listing must fail-open \
             to Continue. Got: {:?}",
            hook_result
        );
    }
}

// ---------------------------------------------------------------------------
// NEW-1 regression guard: STAGED_PATH_LISTING_MAX_OUTPUT_BYTES must cover
// BC-4.16.002's own >= 500-staged-path resource model (git index
// cardinality driver, per the Verification Properties calibration-corpus
// note) — a revert to the original undersized 512-BYTE copy-paste value
// (correct only for the sibling branch-detection call's single short line)
// must fail this test.
// ---------------------------------------------------------------------------

#[test]
// Both operands are `const`, so clippy statically proves this assertion's
// truth value at lint time (`clippy::assertions_on_constants`). That is
// deliberate, not an oversight: the guard is *intentionally*
// compile-time-derivable, but is kept as a `#[test]` (rather than a
// top-level `const _: () = assert!(...)`) specifically so a revert to the
// undersized 512-byte cap surfaces as a named, reportable failure in
// `cargo test -p validate-factory-path-staged` output — the CI-visible
// regression signal this NEW-1 fix requires — rather than only a build
// failure elsewhere.
#[allow(clippy::assertions_on_constants)]
fn test_bc4_16_002_staged_path_listing_max_output_bytes_covers_bc_worst_case() {
    // BC-4.16.002's own resource model specifies a >= 500-simultaneously-
    // staged-path worst case for THIS validator specifically (its resource
    // driver is git index cardinality, not `.factory/` artifact byte size —
    // see BC-4.16.002 Verification Properties section). A conservative
    // per-path-length lower bound of 256 bytes/path (comfortably covering
    // deeply nested `.factory/cycles/<cycle-name>/<file>.md`-style paths
    // plus the trailing newline) over 500 paths yields 128,000 bytes as the
    // minimum acceptable cap. The production constant (131_072 = 128 KiB)
    // clears this bound with headroom; the original defect value (512 bytes)
    // could not hold even ~8 typical staged paths and would fail this
    // assertion outright.
    const BC_DERIVED_WORST_CASE_LOWER_BOUND_BYTES: u32 = 500 * 256; // 128_000 bytes
    assert!(
        STAGED_PATH_LISTING_MAX_OUTPUT_BYTES >= BC_DERIVED_WORST_CASE_LOWER_BOUND_BYTES,
        "NEW-1 regression guard: STAGED_PATH_LISTING_MAX_OUTPUT_BYTES ({}) must be >= the \
         BC-4.16.002-derived >= 500-staged-path worst-case lower bound ({} bytes = 500 \
         paths * 256 bytes/path average). A revert to the original undersized 512-byte cap \
         (or any cap below this bound) must fail this test.",
        STAGED_PATH_LISTING_MAX_OUTPUT_BYTES,
        BC_DERIVED_WORST_CASE_LOWER_BOUND_BYTES
    );
}

// ---------------------------------------------------------------------------
// NEW-3 regression guard (pass-3 LOCAL adversary): the cap-SELECTION branch
// (not just the two constants' own values) must be independently covered.
// Prior to this test, only `STAGED_PATH_LISTING_MAX_OUTPUT_BYTES`'s value
// was pinned (see the NEW-1 guard above); the ternary routing `"diff"` vs.
// everything-else to the correct constant had zero coverage. A regression
// that hardcoded 512 for both calls, or swapped the two arms, would
// reintroduce the HIGH NEW-1 self-wedge defect while every prior test
// stayed green. This test asserts BOTH arms of `max_output_bytes_for`,
// giving `BRANCH_DETECTION_MAX_OUTPUT_BYTES` its first assertion anywhere
// in this suite.
// ---------------------------------------------------------------------------

#[test]
fn test_bc4_16_002_max_output_bytes_for_selects_staged_path_listing_cap_for_diff() {
    assert_eq!(
        max_output_bytes_for(&["diff", "--cached", "--name-only"]),
        STAGED_PATH_LISTING_MAX_OUTPUT_BYTES,
        "NEW-3 regression guard: max_output_bytes_for(&[\"diff\", ...]) must select \
         STAGED_PATH_LISTING_MAX_OUTPUT_BYTES (the staged-path-listing cap), not the \
         undersized branch-detection cap."
    );
}

#[test]
fn test_bc4_16_002_max_output_bytes_for_selects_branch_detection_cap_for_branch() {
    assert_eq!(
        max_output_bytes_for(&["branch", "--show-current"]),
        BRANCH_DETECTION_MAX_OUTPUT_BYTES,
        "NEW-3 regression guard: max_output_bytes_for(&[\"branch\", ...]) must select \
         BRANCH_DETECTION_MAX_OUTPUT_BYTES, not the oversized staged-path-listing cap."
    );
}

// ---------------------------------------------------------------------------
// Structural reachability — hooks-registry.toml (AC-001, PC3, PC5, Invariant
// 5 reuse, T-6, T-7, EC-001, EC-004, EC-008, SDK Grounding Evidence Grep 2/3)
//
// PC3 (INDETERMINATE trigger on cannot-complete) and PC5 (crash advisory)
// are dispatcher-side classification outcomes that occur OUTSIDE
// hook_logic's own Rust control flow (per lib.rs module docs: "that
// classification happens outside this pure-core function's control flow, at
// the WASM host boundary"). The structurally-reachable trigger path this
// story's AC-001 requires IS the PostToolUse + failure_policy=fail-closed
// registry registration itself — so these tests verify that registration
// exists with the exact required shape, which is the crate-local,
// deterministically-testable proxy for "PC3/PC5 are reachable for this
// plugin."
// ---------------------------------------------------------------------------

#[test]
fn test_bc4_16_002_ac001_registry_entry_exists() {
    let path = hooks_registry_toml_path();
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));
    assert!(
        extract_hook_stanza(&content, "validate-factory-path-staged").is_some(),
        "AC-001 / BC-4.16.002 Architecture Anchors: plugins/vsdd-factory/hooks-registry.toml \
         must contain a [[hooks]] stanza with name = \"validate-factory-path-staged\". \
         Entry is absent — implementer must add it per Task 4."
    );
}

#[test]
fn test_bc4_16_002_ac001_pc3_registry_entry_is_post_tool_use_bash() {
    // AC-001 / BC-4.16.002 Precondition 4 / PC3: MUST be registered
    // PostToolUse ^Bash$ — a PreToolUse registration would repeat the exact
    // structural gap this story closes (BC-1.18.001 Invariant 4).
    let path = hooks_registry_toml_path();
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));
    let stanza = extract_hook_stanza(&content, "validate-factory-path-staged")
        .expect("AC-001 prerequisite: registry entry must exist before this gate can pass");
    assert!(
        stanza.contains("event = \"PostToolUse\""),
        "AC-001 / BC-4.16.002 Precondition 4: validate-factory-path-staged MUST be \
         registered event = \"PostToolUse\" (not PreToolUse — that would repeat the \
         structural gap this story closes). Stanza:\n{}",
        stanza
    );
    assert!(
        stanza.contains("tool = \"^Bash$\""),
        "AC-001 / BC-4.16.002 Precondition 4: validate-factory-path-staged MUST be \
         registered tool = \"^Bash$\". Stanza:\n{}",
        stanza
    );
}

#[test]
fn test_bc4_16_002_t6_pc3_registry_entry_is_failure_policy_fail_closed() {
    // BC-4.16.002 T-6 / PC3 / AC-001 closure criterion: failure_policy =
    // "fail-closed" is what makes the INDETERMINATE marker-write path
    // structurally reachable for THIS plugin's own cannot-complete case.
    let path = hooks_registry_toml_path();
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));
    let stanza = extract_hook_stanza(&content, "validate-factory-path-staged")
        .expect("AC-001 prerequisite: registry entry must exist before this gate can pass");
    assert!(
        stanza.contains("failure_policy = \"fail-closed\""),
        "T-6 / BC-4.16.002 PC3: validate-factory-path-staged MUST be registered \
         failure_policy = \"fail-closed\" for the INDETERMINATE marker-write path \
         (write_indeterminate_marker) to be reachable on fuel exhaustion / epoch \
         timeout / OutputTooLarge. Stanza:\n{}",
        stanza
    );
}

#[test]
fn test_bc4_16_002_t7_pc5_registry_entry_is_on_error_continue() {
    // BC-4.16.002 T-7 / PC5 / EC-008: on_error = "continue" is what makes a
    // non-resource-exhaustion crash advisory-only (no block, no marker).
    let path = hooks_registry_toml_path();
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));
    let stanza = extract_hook_stanza(&content, "validate-factory-path-staged")
        .expect("AC-001 prerequisite: registry entry must exist before this gate can pass");
    assert!(
        stanza.contains("on_error = \"continue\""),
        "T-7 / BC-4.16.002 PC5 / EC-008: validate-factory-path-staged MUST be registered \
         on_error = \"continue\" so a non-resource-exhaustion crash is advisory-only. \
         Stanza:\n{}",
        stanza
    );
}

#[test]
fn test_bc4_16_002_registry_entry_priority_161_no_collision() {
    // BC-4.16.002 SDK Grounding Evidence Grep 2 (architect F2 §2.2): 161 is
    // the lowest free priority slot above the occupied 150-160 band, and
    // MUST NOT collide with any other stanza's priority.
    let path = hooks_registry_toml_path();
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));
    let stanza = extract_hook_stanza(&content, "validate-factory-path-staged")
        .expect("AC-001 prerequisite: registry entry must exist before this gate can pass");
    assert!(
        stanza.contains("priority = 161"),
        "BC-4.16.002 Architecture Anchors: validate-factory-path-staged MUST be \
         registered priority = 161. Stanza:\n{}",
        stanza
    );
    let occurrences = content.matches("priority = 161").count();
    assert_eq!(
        occurrences, 1,
        "BC-4.16.002 SDK Grounding Evidence Grep 2: priority = 161 MUST be used by \
         exactly one [[hooks]] stanza (no collision). Found {} occurrence(s) in \
         hooks-registry.toml.",
        occurrences
    );
}

#[test]
fn test_bc4_16_002_registry_entry_timeout_and_capability() {
    let path = hooks_registry_toml_path();
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));
    let stanza = extract_hook_stanza(&content, "validate-factory-path-staged")
        .expect("AC-001 prerequisite: registry entry must exist before this gate can pass");
    assert!(
        stanza.contains("timeout_ms = 5000"),
        "BC-4.16.002 Architecture Anchors: validate-factory-path-staged MUST be \
         registered timeout_ms = 5000. Stanza:\n{}",
        stanza
    );
    assert!(
        stanza.contains("binary_allow"),
        "BC-4.16.002 Precondition 3: validate-factory-path-staged MUST declare an \
         exec_subprocess capability with binary_allow permitting 'git'. Stanza:\n{}",
        stanza
    );
    assert!(
        stanza.contains("git"),
        "BC-4.16.002 Precondition 3: exec_subprocess capability binary_allow must permit \
         'git'. Stanza:\n{}",
        stanza
    );
}

#[test]
fn test_bc4_16_002_ec004_registry_entry_name_distinguishes_from_sibling() {
    // BC-4.16.002 EC-004: "(plugin_name, artifact_path) pairs distinguish
    // the two validators' own INDETERMINATE events by construction" — the
    // registered `name` must be the exact, distinct "validate-factory-path-
    // staged" (past participle), never colliding with the sibling
    // "validate-factory-path-staging" (gerund).
    let path = hooks_registry_toml_path();
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));
    let stanza = extract_hook_stanza(&content, "validate-factory-path-staged")
        .expect("AC-001 prerequisite: registry entry must exist before this gate can pass");
    assert!(
        stanza.contains("name = \"validate-factory-path-staged\""),
        "EC-004 / BC-4.16.002: the registered plugin name must be the exact, distinct \
         string 'validate-factory-path-staged' (past participle) — construction-level \
         disambiguation from the sibling 'validate-factory-path-staging' (gerund) that \
         (plugin_name, artifact_path) pair-keying depends on. Stanza:\n{}",
        stanza
    );
    // Also verify the sibling's own entry is untouched / still separately
    // present (AC-003: no behavioral change to validate-factory-path-staging).
    assert!(
        extract_hook_stanza(&content, "validate-factory-path-staging").is_some(),
        "AC-003 / BC-4.16.002 Invariant 1: the sibling validate-factory-path-staging \
         entry must remain present and untouched — this story ADDS a companion, it does \
         not replace the existing preventive guard."
    );
}
