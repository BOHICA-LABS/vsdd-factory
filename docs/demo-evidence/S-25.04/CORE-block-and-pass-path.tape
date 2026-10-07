Output docs/demo-evidence/S-25.04/CORE-block-and-pass-path.gif
Output docs/demo-evidence/S-25.04/CORE-block-and-pass-path.webm

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

# CORE BEHAVIOR -- the canonical block message this plugin emits when a
# .factory/ path is staged on a product branch (post-hoc check).
Type "grep -A5 'DETECTED: .factory' crates/hook-plugins/validate-factory-path-staged/src/lib.rs"
Enter
Sleep 2s
Sleep 1s

Type "clear"
Enter
Sleep 500ms

# BLOCK PATH (T-1): `git add .factory/STATE.md` completes on branch=develop
# -- staged-path listing + branch-current both mocked; hook_logic must
# return HookResult::Block with FactoryPathStagedOnProductBranch.
Type "cargo test -p validate-factory-path-staged t1_blocks -- --nocapture"
Enter
Sleep 3s
Sleep 1s

Type "clear"
Enter
Sleep 500ms

# CLEAN-PASS PATH (T-3): `git add src/lib.rs` on branch=feature/S-25.04 --
# no .factory/ path staged -- hook_logic must return HookResult::Continue.
Type "cargo test -p validate-factory-path-staged t3_passes -- --nocapture"
Enter
Sleep 3s
Sleep 2s
