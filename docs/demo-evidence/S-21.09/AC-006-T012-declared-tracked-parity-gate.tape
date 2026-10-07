# VHS Demo — S-21.09 AC-006 (and AC-007)
# The T-012 end-to-end declared-vs-tracked parity gate (run_t012_gate) runs
# against the REAL workspace root — both production registries and the real
# git-tracked WASM set. Demonstrates the gate PASSES on the converged tree,
# with T-012 itself (the whole-suite gate) highlighted among the 51 passing
# tests in bundle_orphan_check.rs.
#
# Story: S-21.09 | AC: AC-006, AC-007

Output "AC-006-T012-declared-tracked-parity-gate.gif"
Output "AC-006-T012-declared-tracked-parity-gate.webm"

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

# === Demo: AC-006 T-012 — declared-set-subset-of-tracked-set parity gate ===
# run_t012_gate(&workspace_root()) exercised against the real production
# registries (hooks-registry.toml + resolvers-registry.toml) and the real
# git-tracked hook-plugins/ set -- 51 tests total in bundle_orphan_check.rs.
Type "cargo test -p factory-dispatcher --test bundle_orphan_check --release 2>&1 | grep -E 'T012_declared_set_subset_of_tracked_set|test result'"
Enter
Wait+Screen@30s /test result:/
Sleep 2s
