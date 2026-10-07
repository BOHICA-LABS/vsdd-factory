//! WASI command entry point for validate-unvalidated-mutation-marker.
//!
//! Reads the PreToolUse JSON payload from stdin via the SDK's `__internal::run`
//! trampoline, calls `on_pre_tool_use`, and exits.
//!
//! Both Arm 1 (`^Agent$`) and Arm 2 (`^Bash$` git filter) dispatch to the same
//! `on_pre_tool_use` function because both hooks-registry.toml entries reference
//! this IDENTICAL compiled binary (BC-1.18.002 invariant 1; AC-019).
//!
//! # Compliance notes (BC-1.18.002)
//!
//! - HOST_ABI_VERSION = 1 (no new host functions introduced by S-25.01).
//! - `failure_policy = "fail-open"` for BOTH registry entries (invariant 2).
//!   If this plugin itself crashes or fuel-exhausts, it does NOT write a new marker
//!   (no self-lock via a new quarantine; `failure_policy = "fail-open"` governs
//!   marker-WRITING only). A pre-existing NON-EXPIRED marker still blocks via the
//!   dispatcher's native `on_error = "block_if_marker"` check (ADR-048 §D1):
//!   EC-031 (non-expired marker present → Block / PC5),
//!   EC-009/EC-032 (marker absent or TTL expired → Allow / PC6).
//! - No dependency on factory-dispatcher or other workspace crates (forbidden
//!   for WASM hook plugins).

use validate_unvalidated_mutation_marker::on_pre_tool_use;

fn main() {
    vsdd_hook_sdk::__internal::run(on_pre_tool_use);
}
