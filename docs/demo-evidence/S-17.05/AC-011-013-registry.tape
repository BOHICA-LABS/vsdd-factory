Output docs/demo-evidence/S-17.05/AC-011-013-registry.gif
Output docs/demo-evidence/S-17.05/AC-011-013-registry.webm

Require bash

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

Type "bash docs/demo-evidence/S-17.05/demo-runner.sh registry"
Enter
Sleep 4s
