//! WASI command entry point for stamp-state-timestamp.
//!
//! Reads the PostToolUse JSON payload from stdin via the SDK's
//! `__internal::run` trampoline, calls `on_post_tool_use`, and exits.
//!
//! Deserialization failures (malformed JSON, missing required fields) are
//! caught by the trampoline: the hook logs a best-effort warning and exits 0
//! (graceful degradation — PostToolUse hook, on_error=continue per BC-4.17.001 PC3).
//!
//! Unit-testable logic lives in `src/lib.rs` (`guard_logic`); this file wires
//! the pure logic to real host function calls via `on_post_tool_use`.
//!
//! # Compliance notes (BC-4.17.001 / ADR-046)
//! - `on_post_tool_use` uses `host::read_file` / `host::write_file` / `host::exec_subprocess`.
//! - No `serde_yaml` / `serde_norway` (Architecture Compliance Rule 6).
//! - No `regex` crate (Architecture Compliance Rule 6).
//! - HOST_ABI_VERSION = 1 (BC-4.17.001 architecture compliance).
//! - `async = false` required (ADR-019; ADR-046).

use stamp_state_timestamp::on_post_tool_use;

fn main() {
    vsdd_hook_sdk::__internal::run(on_post_tool_use);
}
