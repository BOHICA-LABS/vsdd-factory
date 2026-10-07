# VHS Demo Tape — S-25.02 cluster-5 AC-018 baseline
#
# Demonstrates BC-1.18.011 (AC-018, B2 one-time governed migration) — the
# full 55-test integration suite for `run_bc_index_migration`'s txn state
# machine, WAL-ordered intent log, content-preservation gate, independent
# census gate, and admission-control wiring, all green. Precondition every
# focused AC-018 clip in this directory is drawn from. This suite runs
# against SYNTHETIC temp-dir BC-INDEX fixtures -- the real one-time
# migration is not yet activated (activation-boundary scope); this is the
# same harness the crate's own test suite uses to exercise the migration
# engine ahead of activation.
#
# TD-VSDD-091 (anti-volatile-pin): no pinned file.rs:NNN line numbers.

Output AC-018-suite-all-green-migration.gif
Output AC-018-suite-all-green-migration.webm

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

Type "echo 'AC-018: BC-1.18.011 B2 one-time governed migration — full suite'"
Enter
Sleep 1s

Type "cargo test -p factory-dispatcher --test bc_1_18_011_b2_migration_test"
Enter
Sleep 6s
