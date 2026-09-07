# VHS Demo Tape — S-25.02 Phase F4 cluster-1 "cap+trigger"
# EC-018 (BC-1.18.005 v1.12 MATCH-FIRST, Postcondition 1 "Blast-radius
#   scoping"): a SIBLING [[shard]] entry ("lessons") omits `shape` entirely
#   (would fail EC-009 if matched) but the current dispatch's target
#   ("foo.rs") matches NO entry at all -> Continue. find_matching_entry
#   resolves (or fails to resolve) BEFORE validate_entry ever runs, so the
#   malformed sibling is never validated for this dispatch.
#
# NOTE: fixed Sleep durations used instead of Wait+Line — see
# AC-002a-under-cap-write-continue.tape's header note for why.
#
# TD-VSDD-091 (anti-volatile-pin): source snippet is extracted by an awk
# range anchored on the enclosing test function's signature
# (`test_BC_1_18_005_EC_018_malformed_sibling_unmatched_target_continues`)
# through its own closing brace, NOT by pinned `file.rs:NNN` line numbers.

# Output paths are relative to THIS file's own directory — run vhs from
# inside docs/demo-evidence/S-25.02/cluster-1-cap-trigger/ (see README.md's
# Reproduction section). This avoids the doubled-path bug that results from
# running vhs from a different cwd while a tape carries a full repo-relative
# Output path.
Output EC-018-match-first-blast-radius.gif
Output EC-018-match-first-blast-radius.webm

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

Type "echo 'BC-1.18.005 EC-018: fn test_BC_1_18_005_EC_018_malformed_sibling_unmatched_target_continues'"
Enter
Sleep 1.5s

Type "awk '/^async fn test_BC_1_18_005_EC_018_malformed_sibling_unmatched_target_continues/,/^}/' crates/factory-dispatcher/tests/bc_1_18_005_shard_cap_trigger_test.rs"
Enter
Sleep 3.5s

Type "cargo test -p factory-dispatcher --test bc_1_18_005_shard_cap_trigger_test test_BC_1_18_005_EC_018_malformed_sibling_unmatched_target_continues -- --exact --nocapture"
Enter
Sleep 5s

Sleep 2s
