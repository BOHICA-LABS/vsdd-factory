# VHS Demo Tape — S-25.02 Phase F4 cluster-1 "cap+trigger"
# AC-023 / EC-013: a [[shard]] entry declares shard_cap_bytes (100,000)
#   GREATER than its own formula-derived ceiling compute_shard_cap_bytes(..)
#   (50,640) -> ShardRegistry entry validation fails loud, HookResult::Error
#   propagates all the way to a non-zero dispatch exit_code, and the
#   block_reason names BOTH the offending artifact_stem ("over-cap-log")
#   AND the failure kind (EC-013) so an operator can locate the bad entry.
#
# NOTE: fixed Sleep durations used instead of Wait+Line — see
# AC-002a-under-cap-write-continue.tape's header note for why.
#
# TD-VSDD-091 (anti-volatile-pin): source snippet is extracted by an awk
# range anchored on the enclosing test function's signature
# (`test_BC_1_18_005_P2001_cap_exceeds_ceiling_block_reason_names_artifact_stem_and_failure_kind`)
# through its own closing brace, NOT by pinned `file.rs:NNN` line numbers.

# Output paths are relative to THIS file's own directory — run vhs from
# inside docs/demo-evidence/S-25.02/cluster-1-cap-trigger/ (see README.md's
# Reproduction section). This avoids the doubled-path bug that results from
# running vhs from a different cwd while a tape carries a full repo-relative
# Output path.
Output AC-023-malformed-config-fail-loud.gif
Output AC-023-malformed-config-fail-loud.webm

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

Type "echo 'BC-1.18.005 AC-023/EC-013: fn test_BC_1_18_005_P2001_cap_exceeds_ceiling_block_reason_names_artifact_stem_and_failure_kind'"
Enter
Sleep 1.5s

Type "awk '/^async fn test_BC_1_18_005_P2001_cap_exceeds_ceiling_block_reason_names_artifact_stem_and_failure_kind/,/^}/' crates/factory-dispatcher/tests/bc_1_18_005_shard_cap_trigger_test.rs"
Enter
Sleep 4s

Type "cargo test -p factory-dispatcher --test bc_1_18_005_shard_cap_trigger_test test_BC_1_18_005_P2001_cap_exceeds_ceiling_block_reason_names_artifact_stem_and_failure_kind -- --exact --nocapture"
Enter
Sleep 5s

Sleep 2s
