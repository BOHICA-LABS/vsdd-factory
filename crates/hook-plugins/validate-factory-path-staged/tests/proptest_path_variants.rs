// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Proptest harness — `.factory/` staged-path variant coverage
//! (BC-4.16.002 PC1 / PC2 / Invariant 4 / Invariant 7).
//!
//! Verifies that `hook_logic` blocks every `.factory/`-rooted staged-path
//! variant returned by the (mocked) `git diff --cached --name-only` call on
//! a product branch (develop), and passes non-`.factory/` staged-path
//! variants — regardless of the triggering command's own text (BROAD
//! scope). Minimum 20 diverse deterministic input variants, per the
//! sibling crate's own AC-006 precedent (S-21.01).
//!
//! All tests are RED at stub time (lib.rs bodies are `todo!()`).
//!
//! # BC traces
//! - BC-4.16.002 PC1: block `.factory/` staging on product branches
//! - BC-4.16.002 PC2: pass on non-`.factory/` staging
//! - BC-4.16.002 Invariant 4: path matching is conservative and
//!   case-insensitive, reused verbatim from BC-4.16.001
//! - BC-4.16.002 Invariant 7: unconditional BROAD trigger scope — the
//!   triggering command's own text is irrelevant; every proptest case below
//!   uses a deliberately unrelated command string to prove this.

use std::cell::Cell;
use std::panic;

use proptest::prelude::*;
use serde_json::json;
use validate_factory_path_staged::{HookCallbacks, hook_logic};
use vsdd_hook_sdk::{HookPayload, HookResult};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

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

/// Runs `hook_logic` with a mocked two-call `exec_subprocess`: first call
/// (staged-path listing) returns `diff_stdout` at exit 0; second call
/// (branch detection) returns `branch` at exit 0.
fn run_hook(command: &str, diff_stdout: &str, branch: &str) -> std::thread::Result<HookResult> {
    let payload = make_post_tool_use_payload(command);
    let diff_stdout = diff_stdout.to_string();
    let branch = branch.to_string();
    let call_index = Cell::new(0u32);
    panic::catch_unwind(move || {
        hook_logic(
            payload,
            HookCallbacks {
                exec_subprocess: move |_bin: &str, _args: &[&str]| {
                    let n = call_index.get();
                    call_index.set(n + 1);
                    if n == 0 {
                        Ok((0, diff_stdout.clone(), String::new()))
                    } else {
                        Ok((0, branch.clone(), String::new()))
                    }
                },
                emit_event: |_, _| {},
                log: |_, _| {},
            },
        )
    })
}

// ---------------------------------------------------------------------------
// Property tests — .factory/ staged-path variants block on develop
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(proptest::test_runner::Config::with_cases(200))]

    /// BC-4.16.002 PC1 + Invariant 4: for any `.factory/`-rooted staged path
    /// component, and a deliberately git-unrelated triggering command (to
    /// prove Invariant 7 BROAD scope), the post-hoc check on develop must
    /// block.
    #[test]
    fn prop_BC_4_16_002_pc1_factory_staged_path_variants_block_on_develop(
        path_suffix in "[a-zA-Z0-9/_.-]{1,60}"
    ) {
        let staged_path = format!(".factory/{}", path_suffix);
        let result = run_hook("echo unrelated-command", &format!("{}\n", staged_path), "develop");
        prop_assert!(
            result.is_ok(),
            "hook_logic panicked for staged path '{}'. Must return HookResult, never \
             panic. Production function is unimplemented (todo!()).",
            staged_path
        );
        if let Ok(hook_result) = result {
            prop_assert_ne!(
                hook_result,
                HookResult::Continue,
                "BC-4.16.002 PC1: staged path '{}' on develop must be BLOCKED \
                 (block_intent=true, exit 2), triggered by an unrelated command \
                 ('echo unrelated-command') — proving the check is unconditional \
                 (Invariant 7). Got Continue.",
                staged_path
            );
        }
    }

    /// BC-4.16.002 PC1: interior `/.factory/` path-component variant also
    /// blocks on develop (not just the leading-prefix form).
    #[test]
    fn prop_BC_4_16_002_pc1_interior_factory_component_blocks_on_develop(
        prefix in "[a-z0-9/]{1,20}",
        suffix in "[a-zA-Z0-9/_.-]{1,40}"
    ) {
        let staged_path = format!("{}/.factory/{}", prefix, suffix);
        let result = run_hook("git commit -m wip", &format!("{}\n", staged_path), "develop");
        prop_assert!(
            result.is_ok(),
            "hook_logic panicked for interior-component staged path '{}'.",
            staged_path
        );
        if let Ok(hook_result) = result {
            prop_assert_ne!(
                hook_result,
                HookResult::Continue,
                "BC-4.16.002 PC1 / Invariant 4: staged path '{}' (interior /.factory/ \
                 component) on develop must be BLOCKED. Got Continue.",
                staged_path
            );
        }
    }

    /// BC-4.16.002 PC2: non-`.factory/` staged-path variants on develop must
    /// NOT be blocked — guards against over-blocking.
    #[test]
    fn prop_BC_4_16_002_pc2_non_factory_staged_path_passes_on_develop(
        path_suffix in "[a-zA-Z0-9/_.-]{1,60}"
    ) {
        let staged_path = format!("src/{}", path_suffix);
        prop_assume!(!staged_path.to_ascii_lowercase().contains(".factory/"));
        let result = run_hook("git add .", &format!("{}\n", staged_path), "develop");
        prop_assert!(
            result.is_ok(),
            "hook_logic panicked for non-.factory/ staged path '{}'.",
            staged_path
        );
        if let Ok(hook_result) = result {
            prop_assert_eq!(
                hook_result,
                HookResult::Continue,
                "BC-4.16.002 PC2: non-.factory/ staged path '{}' on develop must pass \
                 (Continue). Got a blocking result.",
                staged_path
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Deterministic canonical variant vectors (minimum 20 diverse inputs)
// ---------------------------------------------------------------------------

/// Canonical staged-path variants mirroring BC-4.16.002 T-1/T-4/T-9 and
/// BC-4.16.001's own Invariant 4 conservative-forms precedent, adapted to
/// this plugin's file-path (not git-add-argument) matching surface. All are
/// RED at stub time.
#[test]
fn test_bc4_16_002_canonical_factory_staged_path_vectors_block_on_develop() {
    let staged_paths = vec![
        ".factory/STATE.md",
        ".factory/stories/S-25.04.md",
        ".factory/specs/behavioral-contracts/ss-04/BC-4.16.002.md",
        ".factory/specs/architecture/ARCH-INDEX.md",
        ".factory/cycles/v1.0-brownfield-backfill/burst-log.md",
        ".factory/policies.yaml",
        ".factory/tech-debt-register.md",
        ".factory/logs/dispatcher-internal-2026-09-04.jsonl",
        ".factory/specs/verification-properties/VP-INDEX.md",
        ".factory/stories/STORY-INDEX.md",
        // Case-insensitive variants (BC-4.16.002 PC1 example: .Factory/STATE.md)
        ".Factory/STATE.md",
        ".FACTORY/state.md",
        // Interior path-component variants
        "some/nested/dir/.factory/STATE.md",
        "crates/hook-plugins/.factory/README.md",
        // Multi-path stdout: .factory/ path is not the only staged path
        "src/main.rs\n.factory/STATE.md",
        ".factory/specs/domain-spec/entities.md",
        ".factory/specs/prd.md",
        ".factory/stories/epics/E-25-validation-integrity.md",
        ".factory/tech-debt-register.md\n.factory/STATE.md",
        ".factory/specs/architecture/decisions/ADR-039-validator-failure-policy.md",
    ];

    for staged_stdout in &staged_paths {
        let result = run_hook(
            "some unrelated non-git-add command",
            &format!("{}\n", staged_stdout),
            "develop",
        );
        assert!(
            result.is_ok(),
            "hook_logic panicked (todo!()) for staged stdout '{}'. Must return \
             HookResult::Block, not panic. Production unimplemented.",
            staged_stdout
        );
        if let Ok(hook_result) = result {
            assert_ne!(
                hook_result,
                HookResult::Continue,
                "BC-4.16.002 PC1: staged stdout '{}' on develop must be BLOCKED \
                 (exit 2). Got Continue. Production unimplemented.",
                staged_stdout
            );
        }
    }
}

#[test]
fn test_bc4_16_002_factory_artifacts_branch_passes_all_factory_staged_paths() {
    // BC-4.16.002 PC2 / EC-006: factory-artifacts branch must pass
    // unconditionally, regardless of what is staged.
    let staged_paths = vec![".factory/STATE.md", ".factory/stories/S-25.04.md"];
    for staged_stdout in &staged_paths {
        let result = run_hook(
            "git add .factory/STATE.md",
            &format!("{}\n", staged_stdout),
            "factory-artifacts",
        );
        assert!(
            result.is_ok(),
            "BC-4.16.002 PC2: hook_logic panicked (todo!()) for '{}' on \
             factory-artifacts. Must return Continue, not panic. Production \
             unimplemented.",
            staged_stdout
        );
        if let Ok(hook_result) = result {
            assert_eq!(
                hook_result,
                HookResult::Continue,
                "BC-4.16.002 PC2: '{}' on factory-artifacts must return Continue. Got: \
                 {:?}. Production unimplemented.",
                staged_stdout,
                hook_result
            );
        }
    }
}
