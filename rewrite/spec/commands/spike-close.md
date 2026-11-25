---
kind: command
id: spike-close
package: S6
sources: [BLUEPRINT.md#4.9, state.md#2.16, BLUEPRINT.md#6]
---

# `vsdd spike close`

Records the final finding of a named spike and closes its entry.

## Syntax

`vsdd spike close --name <name> --json-file <path>`

## What it does

"Records the final finding of a named spike and closes the entry." The person's way to end a named,
human-led spike: "spike close <name> records its final finding" ([[topic:named-spikes]]). The worktree's
retention then applies, including the branch rule for a named spike
([[topic:worktrees]], [[config:spikes]]).

## Who may call it

`spike-runner` (the spike commands' row: "`spike open`, `spike record [--increment]`, `spike close --name` |
The spike ledger (section 6) | `spike-runner`").

## Writes

The ledger entry: final finding recorded, entry closed ([[artifact:spike-ledger-entry]]).

## Output and failure cases

The closed entry. Failure: no open named entry under that name, or the finding file does not validate
([[schema:spike-finding]]).
