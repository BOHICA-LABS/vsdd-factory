//! WASI command entry point for validate-factory-path-staged.
//!
//! Reads the PostToolUse JSON payload from stdin via the SDK's
//! `__internal::run` trampoline, calls `on_post_tool_use`, and exits.
//!
//! Deserialization failures (malformed JSON, missing required fields) are
//! caught by the trampoline: the hook logs a best-effort warning and exits 0
//! (graceful degradation — `on_error = "continue"` per BC-4.16.002 PC5).
//!
//! Unit-testable logic lives in `src/lib.rs` (`hook_logic`);
//! this file wires the pure logic to real host function calls.
//!
//! # Compliance notes (BC-4.16.002)
//! - HOST_ABI_VERSION = 1 (no new host functions introduced).
//! - Uses only host::exec_subprocess, host::log_*, host::emit_event (ABI v1).
//! - Block messages use HookResult::block_with_fix (canonical Why/Fix/Code),
//!   with `code = FACTORY_PATH_STAGED_ON_PRODUCT_BRANCH` (BC-4.16.002 PC1
//!   Error variant).
//! - No dependency on factory-dispatcher or other workspace crates
//!   (forbidden).
//! - registered PostToolUse `^Bash$`, priority 161, `failure_policy =
//!   "fail-closed"`, `on_error = "continue"` — the `hooks-registry.toml`
//!   entry itself is added at implementation/integration time, not by the
//!   stub-architect stage (BC-4.16.002 Architecture Anchors).

use validate_factory_path_staged::on_post_tool_use;

fn main() {
    vsdd_hook_sdk::__internal::run(on_post_tool_use);
}
