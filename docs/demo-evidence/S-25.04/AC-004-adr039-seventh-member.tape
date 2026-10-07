Output docs/demo-evidence/S-25.04/AC-004-adr039-seventh-member.gif
Output docs/demo-evidence/S-25.04/AC-004-adr039-seventh-member.webm

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

# AC-004: ADR-039 SS2 seventh member -- the dispatcher's own sanctioned
# fail-closed allowlist names validate-factory-path-staged as a distinct
# second authority (ADR-039), NOT a fourth Cohort A (ADR-047) member.
Type "grep -n -A6 'sanctioned_fail_closed: std::collections::HashSet' crates/factory-dispatcher/src/registry.rs"
Enter
Sleep 2s
Sleep 1s

Type "clear"
Enter
Sleep 500ms

# The dispatcher-side regression gate: exactly 4 fail-closed entries
# (3 Cohort A + this seventh ADR-039 member), no more, no less.
Type "cargo test -p factory-dispatcher --lib test_BC_1_01_016_production_registry_all_entries_default_to_fail_open -- --nocapture"
Enter
Sleep 3s
Sleep 2s
