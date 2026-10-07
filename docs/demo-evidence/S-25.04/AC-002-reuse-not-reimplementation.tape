Output docs/demo-evidence/S-25.04/AC-002-reuse-not-reimplementation.gif
Output docs/demo-evidence/S-25.04/AC-002-reuse-not-reimplementation.webm

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

# AC-002: no second marker file / gate mechanism / duplicate classify_outcome
# logic. Grep the new crate: no write_indeterminate_marker CALL, only doc/test
# references to the name it reuses verbatim (dispatcher-side, unchanged).
Type "grep -rn 'write_indeterminate_marker(' crates/hook-plugins/validate-factory-path-staged/src/ || echo 'NO CALL SITE -- marker write stays dispatcher-owned (S-25.01, unchanged)'"
Enter
Sleep 2s
Sleep 1s

Type "clear"
Enter
Sleep 500ms

# Architecturally impossible to reimplement: the crate has zero dependency on
# factory-dispatcher (where the marker mechanism lives) -- reuse is structural,
# not just a convention.
Type "grep -A6 '^\[dependencies\]' crates/hook-plugins/validate-factory-path-staged/Cargo.toml"
Enter
Sleep 2s
Sleep 2s
