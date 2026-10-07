# VHS Demo — S-21.09 AC-003 / AC-004 / AC-005
# Bats integration coverage for the validate-factory-path-staging WASM guard,
# with CI_REQUIRE_ARTIFACTS=1 set and both required artifacts built
# (factory-dispatcher release binary + the now-tracked WASM). Runs the three
# tests that map directly to these ACs:
#   T-001 -> AC-003 (guard BLOCKS git add .factory/ on develop, product branch)
#   T-004 -> AC-004 (guard PASSES git add of a non-.factory/ path)
#   T-002 -> AC-005 (guard PASSES git add .factory/ on factory-artifacts branch)
# Zero skip lines confirms the WASM-absent skip condition (AC-002) is gone.
#
# Story: S-21.09 | AC: AC-002, AC-003, AC-004, AC-005

Output "AC-003-AC-004-AC-005-bats-gate.gif"
Output "AC-003-AC-004-AC-005-bats-gate.webm"

Require bats

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

# === Demo: T-001/T-002/T-004 execute (no _require_artifacts skip) ===
Type "CI_REQUIRE_ARTIFACTS=1 bats plugins/vsdd-factory/tests/validate-factory-path-staging.bats --filter 'T-001 |T-002 |T-004 '"
Enter
Wait+Screen@30s /tests, 0 failures/
Sleep 2s
