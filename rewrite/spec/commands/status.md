---
kind: command
id: status
package: S2c
sources: [BLUEPRINT.md#4.9, state.md#2.16, state.md#2.7, state.md#2.10, packaging.md#2.10]
---

# `vsdd status`

The read-only status report with the exact next command; with `--fix`, the safe repairs and a flush.

## Syntax

`vsdd status [--json] [--wave W]` and `vsdd status --fix`.

## What it does

"The status entry point. Read-only: it computes the report and writes nothing." It "prints the
pipeline status: the same text as the session briefing, plus wave readiness, the lock and its holder ...
computed from the records" — a thin face on the state design's status function
([[topic:status-report]], [[schema:status]]). "With `--json` it gives the same as data."

Three things in order, and it writes nothing:

1. "Checks: the mount; that `factory.toml` and the open closure and decision records parse; `vsdd validate`
   on files changed since the last flush; uncommitted and unpushed state; the run lock and whether its
   holder is alive; jobs left running by an earlier session; spike worktrees against the ledger; raised
   caps and waived files; tracked product files that fall in no path class; a member repository that is not
   cloned."
2. "Reports: the cycle; each unit with its state; the open wave's stories with their positions and computed
   statuses, taken from closure records and evidence; open decisions with their packets; the lock and its
   holder; open spikes; problems, each with the command that fixes it."
3. "Names the next command."

With `--fix` "it also repairs what is safe to repair, and only then does it write: it remounts the state
directory; releases a lock whose holder is dead; removes spike worktrees past retention and worktrees of
delivered stories; prunes runs, jobs and evidence past retention; clones a member repository that is
missing from the `url` in its `[[repo]]` entry; rebuilds the catalogue cache; and flushes. It never deletes
a tracked file and never changes a decision."

## Who may call it

Read-only form: "Any caller, subject to its wall." "Because it releases locks and removes worktrees,
`status --fix` is not in the read group that every agent holds: the main session and the clerk may run it."
`/vsdd-factory:status [--fix]` is the `status` skill; it "tells the main session to run `vsdd status
[--fix]` and relay the output".

## Messages and failure cases

On the `partial` condition (broken mount) the briefing names `/vsdd-factory:status --fix`, which remounts;
uncommitted files in the old directory are kept and reported. It reports "commands not configured" when
`[commands]` keys are unset — "a gate with no command still never reads as a pass". It reports a waived
cap file count and each missing member repository. After a hard crash it names `/vsdd-factory:status
--fix` (and, for a terminal, `vsdd lock release`), or the lock lapses after 20 minutes.

## Team use (decision D33; built when the build widens)

Not on the thin slice path. The report gains: every work claim with its person, clone and label, and
which are this clone's (`work_claims`); the outcome of the last sync, with what is unpublished, unknown,
settled after a lost answer and set aside (`sync`); decisions settled here and not yet published, shown apart from those waiting
for an answer; a claim this clone lost; the hand commits noticed on the target branch; the lines of the
preflight; and a plugin older than the one that last wrote state. All of it is read from files
([[record:clone]], [[record:work-claim]]). With team use on, `--fix` also runs the sync
([[command:sync]]).
