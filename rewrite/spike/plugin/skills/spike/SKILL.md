---
name: spike
description: Run a research spike, an isolated and bounded experiment that answers one question. Use whenever a person or an agent decides a question needs an experiment before work continues.
argument-hint: "<question>"
---

# Spike

Run these steps in order, once each.

1. Use the Write tool to create `.spike/LEDGER.md` in the project root (or the Edit tool to append if it exists),
   adding one line: `OPEN | <the question>`.
2. Dispatch the agent type `fspike:spike-runner` with the Agent tool. Give it the question and this experiment:
   run the Bash command `echo SPIKE-EXPERIMENT-OK` and report what it printed.
3. Append one line to `.spike/LEDGER.md`: `DONE | <the question> | <verdict> | <one-line result>`.
4. Report the finding in one line that starts with `SPIKE-SKILL-FINDING:`.

If a step cannot be done, say exactly which step failed and quote the error.
