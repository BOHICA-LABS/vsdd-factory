Output docs/demo-evidence/S-17.05/AC-003-006-identity-gate.gif
Output docs/demo-evidence/S-17.05/AC-003-006-identity-gate.webm

Require bash

Set FontFamily "Menlo"
Set Theme "Catppuccin Mocha"
Set Width 1200
Set Height 750
Set FontSize 14
Set Padding 24
Set Framerate 30

Hide
Type "cd $(git rev-parse --show-toplevel)"
Enter
Sleep 500ms
Show

Type "bash docs/demo-evidence/S-17.05/demo-runner.sh ac003-self-renewal"
Enter
Sleep 6s

Type "bash docs/demo-evidence/S-17.05/demo-runner.sh ac006-foreign-no-renewal"
Enter
Sleep 6s
