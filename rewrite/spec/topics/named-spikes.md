---
kind: topic
id: named-spikes
sources: [BLUEPRINT.md#6, pipeline.md#2.8, state.md#2.5]
---

# Named spikes

Named, human-led spikes that span sessions: their branch, worktree, files and commands. The shared
protocol and the limits are in [[topic:spikes]].

`/vsdd-factory:spike open <name> -- <mission>` ([[workflow:spike]]) runs one spike-runner step. The
runner's first command is `vsdd spike open --name <name> --run <id>`
([[command:spike-open]]), which creates the branch, the worktree and the two ledger files, or, when
the named entry is already open, attaches this runner's agent id to it. Attaching is never answered
from an earlier finding and does not count against `spikes.max_open`; creating does count.

The runner then advances the mission by one bounded increment inside the named worktree, updates the
ledger files, and ends with `vsdd spike record --increment` ([[command:spike-record]]), which the
stop check accepts for a named entry: a named spike has no verdict until it is closed. Running the
same command again continues from the ledger, in any session. A person can also work in the worktree
by hand. `/vsdd-factory:spike close <name>` records the final finding in the project ledger
([[command:spike-close]]). Named spikes are listed in the session briefing.

A named spike follows the user's existing convention: branch `spike/<name>`, worktree
`.worktrees/spike-<name>/`, and the ledger files `.spike/STATE.md` and `.spike/TASKS.md` inside that
worktree. Those two files belong to the spike and are never read by the pipeline; the project ledger
entry ([[artifact:spike-ledger-entry]]) is the only thing that crosses over. The `resume-spike`
convention keeps working in any repository and can resume a named spike, because the branch, worktree
and ledger file names are the same.

The cap on open named spikes (`spikes.max_open`, 3) and the daily cap are enforced by
`vsdd spike open`.
