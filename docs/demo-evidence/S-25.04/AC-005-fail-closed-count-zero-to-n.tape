Output docs/demo-evidence/S-25.04/AC-005-fail-closed-count-zero-to-n.gif
Output docs/demo-evidence/S-25.04/AC-005-fail-closed-count-zero-to-n.webm

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
Set TypingSpeed 40ms

Hide
Type "cd $(git rev-parse --show-toplevel)"
Enter
Sleep 500ms
Type "clear"
Enter
Sleep 500ms
Show

# AC-005: Layer-1 effective fail-closed count moves from ZERO (S-25.01 merge,
# BC-1.18.004 v1.4 PC4 historical anchor, unchanged) to N>=1 at S-25.04 merge
# -- documented in the CHANGELOG under [Unreleased] > Added.
Type "grep -n 'S-25.04' CHANGELOG.md | head -1"
Enter
Sleep 2s
Sleep 1s

Type "clear"
Enter
Sleep 500ms

# Live count in the production registry: exactly 4 LIVE (non-comment)
# failure_policy = \"fail-closed\" entries -- the 3 unchanged Cohort A members
# plus this story's seventh ADR-039 member. N>=1, attributable entirely to
# validate-factory-path-staged (validate-factory-path-staging itself stays
# at ZERO observable enforcement per its own unchanged PreToolUse registration).
Type "grep -c '^failure_policy = .fail-closed.$' plugins/vsdd-factory/hooks-registry.toml"
Enter
Sleep 2s
Sleep 1s

Type "clear"
Enter
Sleep 500ms

# The dispatcher-side regression gate pins this count structurally.
Type "cargo test -p factory-dispatcher --lib test_BC_1_01_016_production_registry_all_entries_default_to_fail_open 2>&1 | tail -3"
Enter
Sleep 3s
Sleep 2s
