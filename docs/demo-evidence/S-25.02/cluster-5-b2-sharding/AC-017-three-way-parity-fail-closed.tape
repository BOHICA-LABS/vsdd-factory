# VHS Demo Tape — S-25.02 cluster-5 AC-017 three-way ARCH-INDEX parity
#
# Demonstrates BC-1.18.010 VP-128 (part 2) / INV-2: mapping-source-of-truth
# three-way parity. `check_arch_index_parity` compares the SHA the running
# binary was compiled against, the SHA recorded in the
# SubsystemPrefixSnapshot (`.factory/specs/architecture/shard-prefix-map.toml`),
# and ARCH-INDEX.md's live SHA. All three must agree; any single divergence
# (a stale binary against a current manifest, or a manifest that has drifted
# from ARCH-INDEX.md) is a FAIL-CLOSED error -- the addressing layer never
# silently trusts a partially-stale mapping.
#
# Tests: `INV2` filter matches 4 tests:
#   test_BC_1_18_010_INV2_check_arch_index_parity_passes_when_all_three_shas_match
#   test_BC_1_18_010_INV2_check_arch_index_parity_fails_closed_when_manifest_sha_diverges
#   test_BC_1_18_010_INV2_check_arch_index_parity_fails_closed_for_stale_binary_against_current_manifest
#   test_BC_1_18_010_INV2_load_subsystem_prefix_snapshot_parses_valid_toml
#
# TD-VSDD-091 (anti-volatile-pin): no pinned file.rs:NNN line numbers.

Output AC-017-three-way-parity-fail-closed.gif
Output AC-017-three-way-parity-fail-closed.webm

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

Type "echo 'AC-017/INV-2: three-way ARCH-INDEX parity, fail-closed on any divergence'"
Enter
Sleep 1s

Type "cargo test -p factory-dispatcher --test bc_1_18_010_b2_addressing_test INV2 -- --nocapture"
Enter
Sleep 6s
