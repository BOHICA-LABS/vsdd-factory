Output docs/demo-evidence/S-17.05/AC-014-bats-suite.gif
Output docs/demo-evidence/S-17.05/AC-014-bats-suite.webm

Require bats

Set FontFamily "Menlo"
Set Theme "Catppuccin Mocha"
Set Width 1200
Set Height 500
Set FontSize 14
Set Padding 24
Set Framerate 30

Hide
Type "cd $(git rev-parse --show-toplevel)"
Enter
Sleep 500ms
Show

Type "bats plugins/vsdd-factory/tests/stamp-state-timestamp.bats"
Enter
Sleep 12s
