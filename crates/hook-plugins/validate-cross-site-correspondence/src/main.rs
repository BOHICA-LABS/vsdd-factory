//! WASI command entry point for validate-cross-site-correspondence.
//!
//! Reads the PostToolUse JSON payload from stdin via the SDK's
//! `__internal::run` trampoline, calls `on_post_tool_use`, and exits.
//!
//! Deserialization failures (malformed JSON, missing required fields) are
//! caught by the trampoline: the hook logs a best-effort warning and exits 0
//! (graceful degradation).
//!
//! Unit-testable logic lives in `src/lib.rs` (`on_post_tool_use`) and the
//! arm modules (`arm_a1`, `arm_a2`, `arm_b`, `arm_e`); this file
//! wires the pure logic to real host function calls. Class D (`arm_d`) is
//! DEFERRED per BC-5.39.010 v1.6 / D-953 (§File Structure / Task 12; target: S-21.08)
//! and does not exist in this crate.
//!
//! # Compliance notes (BC-5.39.010)
//! - HOST_ABI_VERSION = 1 (no new host functions introduced).
//! - Uses only host::read_file, host::log_warn (ABI v1).
//! - All block messages use HookResult::block_with_fix (canonical Why/Fix/Code).
//! - No dependency on factory-dispatcher or other workspace crates (forbidden).
//! - `event = "PostToolUse"`, `tool = "^(Edit|Write|MultiEdit)$"` (BC-5.39.010 §Gate Spec).
//! - `on_error = "continue"` — fuel exhaustion → silent non-finding, not spurious block.

// WIRING-EXEMPT: This function body is required by the WASM entry-point contract.
// The entire body is a single delegation to the SDK's run trampoline with the
// hook's `on_post_tool_use` function as the argument. No domain logic here.
// BC-5.38.003: WIRING-EXEMPT wiring type = WASI command entry point.

use validate_cross_site_correspondence::on_post_tool_use;

fn main() {
    vsdd_hook_sdk::__internal::run(on_post_tool_use);
}
