// Allow #[cfg(kani)] without triggering unexpected_cfgs warning.
// This inner attribute must appear at the top of the module.
#![cfg_attr(not(kani), allow(unexpected_cfgs))]

//! Pure aggregation of sync_group plugin results into the dispatcher exit code.
//!
//! Per BC-1.14.001 PC5, the dispatcher exit code is computed solely from sync_group
//! plugin results. Async-group results do NOT influence exit code (Invariant 3 +
//! EC-012). This separation is enforced by `aggregate_exit_code` taking
//! `&[PluginResult]` (caller responsible for passing only sync_group results).
//!
//! # BC traces
//! - BC-1.14.001 PC5 — sync_group exit-code aggregation
//! - BC-1.14.001 Invariant 3 — async group excluded from exit-code model
//! - VP-077 H5 — exit-code independence proof (structural: async not accepted)
//! - VP-077 H6 — aggregation correctness proof
//! - ADR-019 — async semantics at registry layer
//! - DI-019 — ASYNC_DRAIN_WINDOW_MS (invariant anchor for async-group exclusion)

use crate::registry::OnError;

/// A sync_group plugin result as produced by the sync dispatch loop.
///
/// This type is intentionally minimal and Kani-friendly: only the two fields
/// that participate in block-intent detection are present. Diagnostic fields
/// (plugin_name, stdout, stderr, etc.) live in [`crate::invoke::PluginResult`]
/// and [`crate::executor::PluginOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginResult {
    /// WASI exit code returned by the plugin process.
    pub exit_code: u8,
    /// Error-handling policy declared in the registry entry for this plugin.
    pub on_error: OnError,
}

/// Aggregate sync_group results into the dispatcher exit code
/// per BC-1.14.001 PC5 and ADR-019 async-semantics invariant.
///
/// Returns `2` if any sync_group plugin returned (`exit_code == 2`, `on_error == Block`).
/// Returns `0` otherwise.
///
/// async_group results are NOT accepted as a parameter and are NOT considered.
/// This structural exclusion is the mechanistic proof of BC-1.14.001 Invariant 3
/// ("Async group plugins are excluded from the exit-code aggregation").
///
/// # Purity contract
///
/// This function is **pure**: no I/O, no globals, no async executor calls,
/// no logging side effects. The function takes a slice and returns a `u8`.
/// Any future modification that introduces side effects must remove the Kani
/// proof harnesses H5/H6 (VP-077) or restructure to maintain a pure inner function.
///
/// # BC traces
/// - BC-1.14.001 PC5 — dispatcher exit code determined by sync_group only
/// - BC-1.14.001 Invariant 3 — async group excluded
/// - VP-077 H5 — exit-code independence (structural)
/// - VP-077 H6 — aggregation correctness
/// - ADR-019 — async semantics at registry layer
/// - DI-019 — ASYNC_DRAIN_WINDOW_MS (drain window constant; cited as invariant
///   source for async-group exclusion semantics)
pub fn aggregate_exit_code(sync_results: &[PluginResult]) -> u8 {
    // Returns 2 iff any sync_group plugin had exit_code==2 AND on_error==Block.
    // Semantically equivalent to VP-077 Appendix A's `.then_some(2).unwrap_or(0)` form;
    // rewritten as if-else for clippy::obfuscated_if_else compatibility.
    if sync_results
        .iter()
        .any(|r| r.exit_code == 2 && r.on_error == OnError::Block)
    {
        2
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::OnError;

    #[test]
    fn empty_sync_results_returns_0() {
        assert_eq!(aggregate_exit_code(&[]), 0);
    }

    #[test]
    fn single_block_result_returns_2() {
        let results = vec![PluginResult {
            exit_code: 2,
            on_error: OnError::Block,
        }];
        assert_eq!(aggregate_exit_code(&results), 2);
    }

    #[test]
    fn exit_2_with_continue_policy_returns_0() {
        // exit_code=2 alone is not a block intent without on_error=Block
        let results = vec![PluginResult {
            exit_code: 2,
            on_error: OnError::Continue,
        }];
        assert_eq!(aggregate_exit_code(&results), 0);
    }

    #[test]
    fn exit_0_with_block_policy_returns_0() {
        // on_error=Block without exit_code=2 is not a block intent
        let results = vec![PluginResult {
            exit_code: 0,
            on_error: OnError::Block,
        }];
        assert_eq!(aggregate_exit_code(&results), 0);
    }

    #[test]
    fn mixed_results_any_block_returns_2() {
        let results = vec![
            PluginResult {
                exit_code: 0,
                on_error: OnError::Continue,
            },
            PluginResult {
                exit_code: 2,
                on_error: OnError::Block,
            },
            PluginResult {
                exit_code: 0,
                on_error: OnError::Block,
            },
        ];
        assert_eq!(aggregate_exit_code(&results), 2);
    }

    #[test]
    fn no_blocking_result_returns_0() {
        let results = vec![
            PluginResult {
                exit_code: 0,
                on_error: OnError::Continue,
            },
            PluginResult {
                exit_code: 1,
                on_error: OnError::Continue,
            },
        ];
        assert_eq!(aggregate_exit_code(&results), 0);
    }

    /// BC-3.08.001 Invariant 5 (trace_id wire format) — zero-occurrence test:
    /// aggregate_exit_code is a pure function; it produces a u8, not a JSON string.
    /// This test asserts the function is not doing any serialization (it cannot
    /// accidentally produce "dispatcher_trace_id" in its output).
    #[test]
    fn aggregate_exit_code_is_pure_u8_no_serialization() {
        let result = aggregate_exit_code(&[PluginResult {
            exit_code: 2,
            on_error: OnError::Block,
        }]);
        // The output is a raw u8 — not a JSON string that could contain field names.
        assert_eq!(result, 2u8);
    }
}

// ---------------------------------------------------------------------------
// Kani proof harnesses — VP-077 H5 + H6
// These run under `cargo kani` only (not `cargo test`).
// ---------------------------------------------------------------------------

#[cfg(kani)]
mod kani_proofs {
    use super::*;
    use crate::registry::OnError;

    // Kani Arbitrary implementations for the minimal types used by H5/H6.
    // PluginResult is a plain struct over Copy types, so kani::any() can
    // generate all combinations exhaustively within the bounded input sizes.
    impl kani::Arbitrary for OnError {
        fn any() -> Self {
            if kani::any::<bool>() {
                OnError::Block
            } else {
                OnError::Continue
            }
        }
    }

    impl kani::Arbitrary for PluginResult {
        fn any() -> Self {
            PluginResult {
                exit_code: kani::any(),
                on_error: kani::any(),
            }
        }
    }

    /// VP-077 H5: proof_vp077_exit_code_independent_of_async_group
    ///
    /// Properties: 5 (Exit-code independence) — the dispatcher exit code is
    /// computed from sync_group results only.  `aggregate_exit_code` takes
    /// `&[PluginResult]` (sync results only); async results are NOT accepted
    /// as a parameter at all.
    ///
    /// Independence is **structurally enforced by the function signature**: the
    /// async group results are not even reachable by the function.  This harness
    /// proves the determinism property — calling `aggregate_exit_code` twice on
    /// the same sync_group slice always returns the same value, regardless of
    /// what async results could hypothetically exist in the environment.
    ///
    /// Two independent async result instances (`_async_result_a`, `_async_result_b`)
    /// are constructed with `kani::any()` to demonstrate that they play no role
    /// in the computation — the function never sees them.
    ///
    /// # BC traces
    /// - BC-1.14.001 PC5 — sync_group exit-code aggregation
    /// - BC-1.14.001 Invariant 3 — async group excluded from exit-code model
    /// - VP-077 Property 5 — exit-code independence
    /// - ADR-019 — async semantics at registry layer
    /// - DI-019 — ASYNC_DRAIN_WINDOW_MS (invariant anchor; async-group exclusion)
    #[kani::proof]
    #[kani::unwind(8)]
    fn proof_vp077_exit_code_independent_of_async_group() {
        let n: usize = kani::any();
        kani::assume(n <= 5);

        // Build a bounded sync_group with arbitrary PluginResult values.
        let mut sync_group: Vec<PluginResult> = Vec::new();
        for _ in 0..n {
            let r: PluginResult = kani::any();
            // Bound exit_code to {0, 1, 2} for tractability.
            kani::assume(r.exit_code <= 2);
            sync_group.push(r);
        }

        // Two independent async_group results — neither is passed to
        // aggregate_exit_code.  Structurally enforced by signature.
        let _async_result_a: PluginResult = kani::any();
        let _async_result_b: PluginResult = kani::any();

        // Call aggregate_exit_code twice on the same sync_group.
        // The async results are NOT accepted as input — independence is structural.
        let r1 = aggregate_exit_code(&sync_group);
        let r2 = aggregate_exit_code(&sync_group);

        // Determinism: identical inputs → identical outputs.
        kani::assert(
            r1 == r2,
            "VP-077 H5: aggregate_exit_code must be deterministic; \
             same sync_group must always produce same exit code",
        );

        // Independence: return value is only 0 or 2 (no other values possible).
        kani::assert(
            r1 == 0 || r1 == 2,
            "VP-077 H5: aggregate_exit_code must return 0 or 2 only",
        );
    }

    /// VP-077 H6: proof_vp077_aggregation_correctness
    ///
    /// Properties: 6 (Aggregation correctness) — `aggregate_exit_code` returns
    /// 2 if and only if at least one sync_group plugin has exit_code == 2 AND
    /// on_error == OnError::Block; otherwise returns 0.
    ///
    /// Uses `kani::assume(r.exit_code <= 2)` to bound the state space for
    /// tractability while covering the full semantically relevant range.
    ///
    /// # BC traces
    /// - BC-1.14.001 PC5 — dispatcher exit code determined by sync_group only
    /// - BC-1.14.001 Invariant 3 — async group excluded
    /// - VP-077 Property 6 — aggregation correctness
    /// - ADR-019 — async semantics at registry layer
    /// - DI-019 — ASYNC_DRAIN_WINDOW_MS (drain window invariant anchor)
    #[kani::proof]
    #[kani::unwind(8)]
    fn proof_vp077_aggregation_correctness() {
        let n: usize = kani::any();
        kani::assume(n <= 5);

        let mut sync_group: Vec<PluginResult> = Vec::new();
        for _ in 0..n {
            let r: PluginResult = kani::any();
            // Bound exit_code to {0, 1, 2} for Kani tractability while
            // covering all semantically relevant values.
            kani::assume(r.exit_code <= 2);
            sync_group.push(r);
        }

        // Compute the expected result from spec definition:
        //   block_intent := (exit_code == 2 AND on_error == Block)
        let any_blocking = sync_group
            .iter()
            .any(|r| r.exit_code == 2 && r.on_error == OnError::Block);

        let exit_code = aggregate_exit_code(&sync_group);

        if any_blocking {
            kani::assert(
                exit_code == 2,
                "VP-077 H6: any sync block (exit_code=2, on_error=Block) => dispatcher exit 2",
            );
        } else {
            kani::assert(
                exit_code == 0,
                "VP-077 H6: no sync block => dispatcher exit 0",
            );
        }
    }
}
