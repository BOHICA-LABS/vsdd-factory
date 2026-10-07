# VHS Demo Tape — S-25.02 cluster-4 AC-015/EC-008 counter-divergence guard
#
# Demonstrates the EC-008 / Invariant 5 counter-divergence BLOCKING guard
# (BC-1.18.009 v1.6 hardening):
#
#   The item-count trigger fires (read_changelog_item_count serde count = 50
#   >= N=50) BUT rotate_changelog_at returns Ok(report) with report.mutated==false
#   (counter-method divergence: YAML inline-sequence fixture passes serde count
#   but fails the line-scan path's exact "changelog:\n" match, so
#   changelog_sequence_bounds returns None and rotate_changelog_at reports 0
#   items moved, mutated=false).
#
#   The gate MUST return HookResult::Error with message starting "E-SHD-014:"
#   (a BLOCKING fail-loud error: block_intent=true, exit_code=2, on_error=Block).
#   It MUST NEVER return HookResult::Block (the retry variant) — emitting Block
#   on a mutated=false report would send the retrying agent into a permanent
#   self-DoS block+retry loop on BC-INDEX.md for the session.
#
# Test: test_BC_1_18_009_EC008_INV5_mutated_false_returns_e_shd_014_error_variant_still_blocking
#
# Assertions verified by the test:
#   - exit_code == 2, block_intent = true (BLOCKING Error, not Block retry variant)
#   - "outcome":"error" present in per_plugin_results (NOT "outcome":"block")
#   - Error message starts with "E-SHD-014:" (NOT E-SHD-004 or E-SHD-001)
#   - Frontmatter byte-identical to pre-attempt state (no partial rotation)
#   - Archive file NOT created (guard fires before any file mutation)
#
# TD-VSDD-091 (anti-volatile-pin): no pinned file.rs:NNN line numbers.

Output AC-015-ec008-counter-divergence.gif
Output AC-015-ec008-counter-divergence.webm

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

Type "echo 'AC-015/EC-008/INV-5: counter-divergence guard -> E-SHD-014 blocking Error'"
Enter
Sleep 1s

Type "cargo test -p factory-dispatcher --test bc_1_18_009_b1_rotate_test EC008_INV5 -- --nocapture"
Enter
Sleep 20s

Sleep 2s
