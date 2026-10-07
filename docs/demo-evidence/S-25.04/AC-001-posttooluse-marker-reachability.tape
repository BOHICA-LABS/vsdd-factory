Output docs/demo-evidence/S-25.04/AC-001-posttooluse-marker-reachability.gif
Output docs/demo-evidence/S-25.04/AC-001-posttooluse-marker-reachability.webm

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

# AC-001: new PostToolUse companion registered event=PostToolUse tool=^Bash$
# priority=161 failure_policy=fail-closed -- structurally reaches the marker-write path.
Type "grep -A9 'name = .validate-factory-path-staged.$' plugins/vsdd-factory/hooks-registry.toml"
Enter
Sleep 2s
Sleep 1s

Type "clear"
Enter
Sleep 500ms

# The two AC-001 structural-reachability tests (event=PostToolUse tool=^Bash$,
# failure_policy=fail-closed -- the exact bits that make write_indeterminate_marker
# reachable per BC-1.18.001 Postcondition 4 / Invariant 4).
Type "cargo test -p validate-factory-path-staged ac001 -- --nocapture"
Enter
Sleep 3s
Sleep 2s
