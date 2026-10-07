# VHS Demo Tape — S-25.02 cluster-5 AC-018 fresh migration + content preservation
#
# Demonstrates BC-1.18.011 Postcondition 1: a FRESH `run_bc_index_migration`
# run against a synthetic multi-subsystem BC-INDEX.md fixture splits every
# BC row into its correct first-level per-subsystem shard file, preserves
# the frontmatter summary and invariants section verbatim, and the
# content-preservation gate (`verify_content_preservation`) proves
# per-BC-row equivalence between the staged shards and the original source
# -- byte-for-byte row content is never lost, reordered-and-hidden, or
# silently dropped during the split. The gate also correctly IGNORES the
# newly-added subsystem-shard-manifest section (that section is new
# structure the migration itself adds, not a content-preservation
# violation) while still catching a genuine row mismatch as a
# ContentPreservationAbort.
#
# Tests: `FC5P1001` filter matches the flagship fresh-split test; the
# second command's `content_preservation` filter matches the 3-test
# preservation-gate suite:
#   test_BC_1_18_010_PC1_FC5P1001_run_bc_index_migration_preserves_frontmatter_summary_and_invariants
#   test_BC_1_18_011_PC1_verify_content_preservation_passes_when_staged_rows_match_source
#   test_BC_1_18_011_PC1_verify_content_preservation_returns_content_preservation_abort_on_mismatch
#   test_BC_1_18_011_PC1_verify_content_preservation_ignores_added_subsystem_shard_manifest_section
#
# TD-VSDD-091 (anti-volatile-pin): no pinned file.rs:NNN line numbers.

Output AC-018-fresh-split-content-preservation.gif
Output AC-018-fresh-split-content-preservation.webm

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

Type "echo 'AC-018/PC1: fresh split on synthetic fixture -- preserves frontmatter+summary+invariants'"
Enter
Sleep 1s

Type "cargo test -p factory-dispatcher --test bc_1_18_011_b2_migration_test FC5P1001 -- --nocapture"
Enter
Sleep 5s

Type "echo 'AC-018/PC1: content-preservation gate — per-BC-row equivalence'"
Enter
Sleep 1s

Type "cargo test -p factory-dispatcher --test bc_1_18_011_b2_migration_test content_preservation -- --nocapture"
Enter
Sleep 6s
