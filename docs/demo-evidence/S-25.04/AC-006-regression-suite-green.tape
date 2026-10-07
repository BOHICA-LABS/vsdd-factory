Output docs/demo-evidence/S-25.04/AC-006-regression-suite-green.gif
Output docs/demo-evidence/S-25.04/AC-006-regression-suite-green.webm

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

# AC-006: the full validate-factory-path-staged regression suite (45 tests:
# 40 unit + 5 proptest) stays green, including the BROAD-scope test (T-4)
# proving a non-`git add`-text wrapper command still gets caught.
Type "cargo test -p validate-factory-path-staged 2>&1 | grep -E 'test result:|t4_broad'"
Enter
Sleep 3s
Sleep 1s

Type "clear"
Enter
Sleep 500ms

# The DO-NOT-DELETE backward-compat guard from S-25.01 (BC-1.18.004 PC5)
# stays present and green, unmodified by this story.
Type "cargo test -p factory-dispatcher --lib test_BC_1_18_004_fail_open_default_preserves_advisory_behavior -- --nocapture"
Enter
Sleep 3s
Sleep 2s
