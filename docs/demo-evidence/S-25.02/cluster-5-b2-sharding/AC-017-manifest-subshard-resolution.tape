# VHS Demo Tape — S-25.02 cluster-5 AC-017 manifest-keyed second-level addressing
#
# Demonstrates BC-1.18.010 VP-128 (part 1): manifest-keyed second-level
# addressing. `resolve_bc_shard_path` routes to `first_level_shard_path` for
# a non-sub-sharded subsystem (SS-01, SS-07) but for an over-cap
# sub-sharded subsystem (SS-05) consults the subsystem's own
# `BC-INDEX-SS-05.manifest.toml` sub-manifest to find the single
# authoritative sub-shard range covering the requested BC id -- and returns
# a named SubShardRangeNotFound error (never a silent guess) when no range
# covers the id.
#
# Tests: `resolve_bc_shard_path` filter matches 4 tests:
#   test_BC_1_18_010_EC001_EC020_resolve_bc_shard_path_ss05_subsharded_resolves_correct_sub_shard
#   test_BC_1_18_010_EC002_resolve_bc_shard_path_ss01_not_subsharded_delegates_to_first_level
#   test_BC_1_18_010_PC4_resolve_bc_shard_path_returns_sub_shard_range_not_found_when_uncovered
#   test_BC_1_18_010_EC004_resolve_bc_shard_path_ss07_whole_corpus_style_lookup_needs_no_sub_manifest
#
# TD-VSDD-091 (anti-volatile-pin): no pinned file.rs:NNN line numbers.

Output AC-017-manifest-subshard-resolution.gif
Output AC-017-manifest-subshard-resolution.webm

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

Type "echo 'AC-017/VP-128: manifest-keyed second-level sub-shard resolution (SS-05)'"
Enter
Sleep 1s

Type "cargo test -p factory-dispatcher --test bc_1_18_010_b2_addressing_test resolve_bc_shard_path -- --nocapture"
Enter
Sleep 6s
