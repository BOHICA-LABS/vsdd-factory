# VHS Demo — S-21.09 AC-001
# Confirms the tracked-artifact premise: validate-factory-path-staging.wasm
# is present in the git INDEX (not merely on disk) under the gitignored
# hook-plugins/ directory, and is declared in the production
# hooks-registry.toml. Read-only git/grep commands against the real tree.
#
# Story: S-21.09 | AC: AC-001

Output "AC-001-artifact-tracked-and-declared.gif"
Output "AC-001-artifact-tracked-and-declared.webm"

Require git

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

# === Demo: AC-001 — WASM artifact is tracked in git (not just on disk) ===
Type "git ls-files plugins/vsdd-factory/hook-plugins/validate-factory-path-staging.wasm | sed 's/^/TRACKED: /'"
Enter
Wait+Screen@10s /TRACKED: /
Sleep 2s

Type "clear"
Enter
Sleep 500ms

# === Registry declares the same artifact (hooks-registry.toml) ===
Type "grep -n validate-factory-path-staging.wasm plugins/vsdd-factory/hooks-registry.toml | sed 's/^/FOUND: /'"
Enter
Wait+Screen@10s /FOUND: /
Sleep 2s
