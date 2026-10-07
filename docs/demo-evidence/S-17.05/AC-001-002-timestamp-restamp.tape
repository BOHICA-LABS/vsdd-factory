Output docs/demo-evidence/S-17.05/AC-001-002-timestamp-restamp.gif
Output docs/demo-evidence/S-17.05/AC-001-002-timestamp-restamp.webm

Require bash

Set FontFamily "Menlo"
Set Theme "Catppuccin Mocha"
Set Width 1200
Set Height 600
Set FontSize 14
Set Padding 24
Set Framerate 30

Hide
Type "cd $(git rev-parse --show-toplevel)"
Enter
Sleep 500ms
Show

Type "bash docs/demo-evidence/S-17.05/demo-runner.sh ac001-restamp"
Enter
Sleep 6s
