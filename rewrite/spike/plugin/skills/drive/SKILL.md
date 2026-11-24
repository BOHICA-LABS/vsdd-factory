---
name: drive
description: Probe skill that tells the main session to start a plugin workflow.
disable-model-invocation: true
---

Start the saved workflow named fspike:story with the Workflow tool, passing args {"id": "DRIVE-1"}.
When it has finished, print one line: DRIVE-RESULT followed by the JSON it returned.
If the tool refuses, quote the exact refusal.
