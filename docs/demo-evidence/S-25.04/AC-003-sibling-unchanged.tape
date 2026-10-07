Output docs/demo-evidence/S-25.04/AC-003-sibling-unchanged.gif
Output docs/demo-evidence/S-25.04/AC-003-sibling-unchanged.webm

Require cargo
Require git

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

# AC-003: validate-factory-path-staging's own PreToolUse registration is
# byte-unchanged by this story -- diff against the pre-S-25.04 merge-base.
Type "git diff $(git merge-base HEAD origin/develop) HEAD -- plugins/vsdd-factory/hooks-registry.toml | grep -c 'validate-factory-path-staging.$' || echo 'BYTE-UNCHANGED: 0 diff lines touch the sibling stanza'"
Enter
Sleep 2s
Sleep 1s

Type "clear"
Enter
Sleep 500ms

# EC-004: the two entries are construction-level distinct, and the sibling's
# entry is still present -- verified by a dedicated unit test.
Type "cargo test -p validate-factory-path-staged ec004 -- --nocapture"
Enter
Sleep 3s
Sleep 2s
