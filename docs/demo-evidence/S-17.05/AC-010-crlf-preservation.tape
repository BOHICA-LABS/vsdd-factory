Output docs/demo-evidence/S-17.05/AC-010-crlf-preservation.gif
Output docs/demo-evidence/S-17.05/AC-010-crlf-preservation.webm

Require bash

Set FontFamily "Menlo"
Set Theme "Catppuccin Mocha"
Set Width 1200
Set Height 700
Set FontSize 13
Set Padding 24
Set Framerate 30

Hide
Type "cd $(git rev-parse --show-toplevel)"
Enter
Sleep 500ms
Show

Type "bash docs/demo-evidence/S-17.05/demo-runner.sh ac010-crlf"
Enter
Sleep 6s
