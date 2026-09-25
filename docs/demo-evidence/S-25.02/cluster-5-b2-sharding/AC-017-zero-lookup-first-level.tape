# VHS Demo Tape — S-25.02 cluster-5 AC-017 zero-lookup first-level addressing
#
# Demonstrates BC-1.18.010 VP-127: zero-lookup, pure-function first-level
# addressing. `first_level_shard_path(bc_id, prefixes)` computes the shard
# path for a subsystem (e.g. SS-01) from the in-memory SubsystemPrefixSnapshot
# alone -- no filesystem I/O, no directory scan -- and returns a named
# UnmappedPrefix error (INV-1) for an unknown subsystem major instead of
# silently guessing.
#
# Tests: `first_level_shard_path` filter matches 2 tests:
#   test_BC_1_18_010_VP127_EC002_first_level_shard_path_computes_ss01_from_prefix_snapshot_alone
#   test_BC_1_18_010_INV1_first_level_shard_path_returns_unmapped_prefix_error_for_unknown_subsystem
#
# TD-VSDD-091 (anti-volatile-pin): no pinned file.rs:NNN line numbers.

Output AC-017-zero-lookup-first-level.gif
Output AC-017-zero-lookup-first-level.webm

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

Type "echo 'AC-017/VP-127: zero-lookup pure-function first-level addressing'"
Enter
Sleep 1s

Type "cargo test -p factory-dispatcher --test bc_1_18_010_b2_addressing_test first_level_shard_path -- --nocapture"
Enter
Sleep 6s
