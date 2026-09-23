# VHS Demo Tape — S-25.02 cluster-5 AC-018 crash-atomicity: txn state machine + WAL-ordered intent log
#
# Demonstrates BC-1.18.011's crash-atomicity mechanism: a durable txn record
# (write_txn_record / read_active_txn_record round-trips; a live txn is
# always distinguishable from a stale terminal one) plus a WAL-ordered
# intent log (append/read round-trip) that records each canonical move
# BEFORE it happens. A torn trailing record (partial write from a crash
# mid-append) is treated as ABSENT, never as partial/corrupt data — the
# recovery decision logic (`decide_recovery`) then reads that log to decide
# whether a given canonical move should be treated as done (post-hash
# already matches canonical), redone (staging has the post-hash but
# canonical still shows pre-state), or fails closed on an absent or
# ambiguous record rather than ever guessing.
#
# Tests: first command's `INV1` filter matches the txn-record round-trip (2
# tests); second command's `intent_log` filter matches the WAL append/read
# and recovery-decision suite (6 tests):
#   test_BC_1_18_011_INV1_write_txn_record_and_read_active_txn_record_round_trip
#   test_BC_1_18_011_INV1_read_active_txn_record_returns_none_when_absent
#   test_BC_1_18_011_intent_log_append_and_read_round_trip
#   test_BC_1_18_011_intent_log_torn_trailing_record_treated_as_absent_never_partial
#   test_BC_1_18_011_intent_log_decide_recovery_treat_done_when_canonical_matches_expected_post_hash
#   test_BC_1_18_011_intent_log_decide_recovery_redo_rename_when_staging_has_post_hash_and_canonical_has_pre_state
#   test_BC_1_18_011_intent_log_decide_recovery_fail_closed_on_absent_record
#   test_BC_1_18_011_intent_log_decide_recovery_fail_closed_on_ambiguous_canonical_state
#
# TD-VSDD-091 (anti-volatile-pin): no pinned file.rs:NNN line numbers.

Output AC-018-crash-atomicity-txn-intent-log.gif
Output AC-018-crash-atomicity-txn-intent-log.webm

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

Type "echo 'AC-018: durable txn record round-trip -- live txn never confused with stale terminal'"
Enter
Sleep 1s

Type "cargo test -p factory-dispatcher --test bc_1_18_011_b2_migration_test INV1 -- --nocapture"
Enter
Sleep 5s

Type "echo 'AC-018: WAL-ordered intent log -- torn record = absent, fail-closed recovery decisions'"
Enter
Sleep 1s

Type "cargo test -p factory-dispatcher --test bc_1_18_011_b2_migration_test intent_log -- --nocapture"
Enter
Sleep 6s
