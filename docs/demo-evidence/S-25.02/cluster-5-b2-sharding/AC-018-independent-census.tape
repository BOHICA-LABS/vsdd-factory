# VHS Demo Tape — S-25.02 cluster-5 AC-018 independent census
#
# Demonstrates BC-1.18.011 Postcondition 2: an INDEPENDENT census gate that
# does not trust the migration's own bookkeeping. `compute_independent_census`
# enumerates BC ids from the staged shard files by direct re-scan and
# asserts the enumerated cardinality matches `total_bcs`;
# `verify_independent_census` asserts every id appears in EXACTLY ONE shard
# -- a duplicated row (double-counted) or a dropped row (silently lost) is a
# CensusMismatchAbort, and a staged BC-INDEX.md body that still retains rows
# post-split (i.e. did not actually get replaced with a stub/summary) is
# also caught. Includes the SS-05 sub-split scoped count check.
#
# Tests: `census` filter matches 8 tests (7 census-specific +
# process_exit_code_content_and_census_aborts_map_to_exit_2):
#   test_BC_1_18_011_PC2_compute_independent_census_enumerates_ids_matching_total_bcs
#   test_BC_1_18_011_PC2_compute_independent_census_errors_when_cardinality_diverges_from_total_bcs
#   test_BC_1_18_011_PC2_verify_independent_census_passes_when_every_id_in_exactly_one_shard
#   test_BC_1_18_011_EC001_verify_independent_census_returns_census_mismatch_abort_on_duplicated_row
#   test_BC_1_18_011_PC2_verify_independent_census_returns_census_mismatch_abort_on_dropped_row
#   test_BC_1_18_011_PC2_verify_independent_census_fails_when_staged_bc_index_body_retains_rows
#   test_BC_1_18_011_EC004_verify_independent_census_ss05_sub_split_scoped_count
#   test_BC_1_18_011_process_exit_code_content_and_census_aborts_map_to_exit_2
#
# TD-VSDD-091 (anti-volatile-pin): no pinned file.rs:NNN line numbers.

Output AC-018-independent-census.gif
Output AC-018-independent-census.webm

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

Type "echo 'AC-018/PC2: independent census — re-scan-based, never trusts migration bookkeeping'"
Enter
Sleep 1s

Type "cargo test -p factory-dispatcher --test bc_1_18_011_b2_migration_test census -- --nocapture"
Enter
Sleep 6s
