# VHS Demo — S-21.09 AC-006 (core "gate fires on an orphan" behavior)
# Read-only demonstration (no mutation of real registries/tree): shows the
# T-015 fixture-driven negative control, which builds an isolated synthetic
# declared/tracked set (NOT the real repo) with one declared-but-untracked
# artifact ("hooks-only.wasm") and asserts check_declared_subset_tracked()
# returns Err with a "MISSING: <name>" identifier. This is the same
# classification machinery run_t012_gate uses against the real tree in
# AC-006-T012-declared-tracked-parity-gate. First greps the exact assertion
# out of the test source, then runs the test live to confirm it fires.
#
# Story: S-21.09 | AC: AC-006

Output "AC-006-T015-orphan-missing-classification.gif"
Output "AC-006-T015-orphan-missing-classification.webm"

Require cargo

Set Shell "zsh"
Set FontFamily "Menlo"
Set FontSize 18
Set Width 1200
Set Height 600
Set Theme "Catppuccin Mocha"
Set WindowBar Colorful
Set BorderRadius 8
Set Padding 20
Set Framerate 30
Set TypingSpeed 50ms

Hide
Type "cd $(git rev-parse --show-toplevel)"
Enter
Sleep 1s
Type "clear"
Enter
Sleep 500ms
Show

# === Demo: AC-006 T-015 — declared-but-untracked fixture fires MISSING ===
# Step 1: show the load-bearing assertion in the test source (read-only).
Type "grep -n 'MISSING: hook-plugins/hooks-only.wasm' crates/factory-dispatcher/tests/bundle_orphan_check.rs"
Enter
Wait+Screen@10s /1941:/
Sleep 2s

Type "clear"
Enter
Sleep 500ms

# Step 2: run the isolated fixture test live — confirms the MISSING
# classification fires exactly as asserted, with zero mutation of the real
# registries or git-tracked tree.
Type "cargo test -p factory-dispatcher --test bundle_orphan_check --release test_S_21_09_ac006_T015_declared_but_untracked_arm_names_artifact -- --exact 2>&1 | tail -6"
Enter
Wait+Screen@30s /test result:/
Sleep 2s
