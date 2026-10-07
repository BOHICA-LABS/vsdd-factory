# VHS Demo Tape — S-25.02 Phase F4 cluster-1 "cap+trigger"
# AC-002 (under-cap leg): Write, projected_size = len(content) = 5,000 bytes,
#   shard_cap_bytes = 49,152 -> HookResult::Continue (no trigger fires)
#
# BC-1.18.005 Postcondition 3 / Invariant 1 — drives the REAL public gate
# end-to-end: execute_tiers -> executor.rs::shard_cap_precheck ->
# ShardRegistry::load -> shard_cap_gate_check, against a real
# `.factory/shard-config.toml` fixture and a real target file on disk.
#
# NOTE: this tape uses fixed Sleep durations rather than Wait+Line/Wait+Screen
# — in this recording environment, VHS's Wait matcher timed out even though
# Type/Enter delivery to the pty and command execution both worked correctly
# (verified independently: a plain Sleep-gated tape shows the typed command's
# real output on screen). Sleep durations below are generously sized above
# each command's measured real run time (every command here completes in
# well under 1s on a warm target dir).
#
# TD-VSDD-091 (anti-volatile-pin): source snippet is extracted by an awk
# range anchored on the enclosing test function's signature
# (`test_BC_1_18_005_INV1_write_with_real_shard_config_reaches_native_gate`)
# through its own closing brace, NOT by pinned `file.rs:NNN` line numbers —
# the extraction survives unrelated line-number drift elsewhere in the file.

# Output paths are relative to THIS file's own directory — run vhs from
# inside docs/demo-evidence/S-25.02/cluster-1-cap-trigger/ (see README.md's
# Reproduction section). This avoids the doubled-path bug that results from
# running vhs from a different cwd while a tape carries a full repo-relative
# Output path.
Output AC-002a-under-cap-write-continue.gif
Output AC-002a-under-cap-write-continue.webm

Require cargo

Set Shell "zsh"
Set FontFamily "Menlo"
Set FontSize 18
Set Width 1200
Set Height 700
Set Theme "Catppuccin Mocha"
Set WindowBar Colorful
Set BorderRadius 8
Set Padding 20
Set Framerate 30
Set TypingSpeed 50ms

Hide
Type "cd $(git rev-parse --show-toplevel) && clear"
Enter
Sleep 1.5s
Show

Type "echo 'BC-1.18.005 AC-002 (under-cap leg): fn test_BC_1_18_005_INV1_write_with_real_shard_config_reaches_native_gate'"
Enter
Sleep 1.5s

Type "awk '/^async fn test_BC_1_18_005_INV1_write_with_real_shard_config_reaches_native_gate/,/^}/' crates/factory-dispatcher/tests/bc_1_18_005_shard_cap_trigger_test.rs"
Enter
Sleep 3s

Type "cargo test -p factory-dispatcher --test bc_1_18_005_shard_cap_trigger_test test_BC_1_18_005_INV1_write_with_real_shard_config_reaches_native_gate -- --exact --nocapture"
Enter
Sleep 5s

Sleep 2s
