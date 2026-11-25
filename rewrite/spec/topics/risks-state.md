---
kind: topic
id: risks-state
sources: [state.md#7]
---

# Risks the state design names

1. **The mount is one more thing that can break.** A deleted directory, a pruned worktree or a person
   checking out another branch inside `.factory/` leaves the project `partial`. Mitigation: the mount test
   is five file reads and runs on every hook call and every `vsdd` command; in that condition state writes
   are blocked instead of landing in the wrong place; `vsdd status --fix` remounts and keeps any
   uncommitted files ([[topic:state-directory]]).
2. **A fresh clone is silent until `setup` runs.** With no `.factory/`, the hooks are inert, so nothing
   announces that the project is a factory project. No pipeline work can start there, because every
   workflow's survey fails, but direct agent work would be ungated. Mitigation: `vsdd doctor` and `setup`
   detect the unmounted state branch; a one-line briefing hint is possible at the cost of the hook design's
   "silent when not initialised" rule, and is left to that design's open question
   ([[topic:hooks-inert]]).
3. **Specification and code have separate histories.** Nobody can check out one commit and get both.
   Mitigation: each state commit carries the integration head as a trailer, and closure records carry the
   product commit each gate passed on. This is recorded once and never updated, so it cannot become the
   citation bookkeeping of the old system ([[topic:state-commits]], [[record:closure]]).
4. **Resume across machines is only as good as the last checkpoint.** Step records and evidence are local.
   A run that dies before its checkpoint is repeated on another machine, and gates are measured again.
   Mitigation: every workflow ends in a checkpoint, a wave run checkpoints per wave, and repeating is safe
   because the survey starts from disk. If this proves costly, the fix is an extra checkpoint inside long
   workflows, not tracking step records ([[topic:compaction-and-cold-start]]).
5. **The run lock depends on liveness signals.** If the hook runner does not refresh `seen`, a quiet 20
   minutes inside one long tool call makes a live run look dead. Mitigation: the default is longer than
   the longest foreground command Claude Code allows; the take-over is logged; and a second run on the same
   story would still be stopped at delivery by the evidence check ([[topic:run-lock]]).
6. **Caps will sometimes block legitimate content.** A contract with a large table of test vectors or an
   architecture overview of a large system can exceed its cap. Mitigation: the message says how to split; a
   project may raise a cap to twice its default, visibly ([[config:caps]]); imported files over the cap are
   waived by name ([[topic:importer]]). The numbers should be checked against the end-to-end proof and the
   first import ([[topic:artifact-rules]]).
7. **The importer meets eight different layouts.** The projects drifted: `prism` has flat contract files
   with slugs and free-form story identifiers, others have subsystem directories whose names do not follow
   from the identifiers. Mitigation: a dry run that only reports; fixtures cut from the real projects; a
   tag before anything changes; aliases for identifiers that cannot be kept ([[topic:importer]]).
8. **Records are only as true as the commands that write them.** A closure says a gate passed because
   `vsdd` measured it, which is sound. A verdict says findings were resolved because a reviewer's
   structured result said so, which is judgment. Mitigation: that line is the pipeline design's rule
   ([[topic:trust-layers]]); this design keeps the two kinds of fact in different stores, so a reader can
   tell which is which.
9. **A shared tool, four owners.** The survey, the checkpoint and the stores are specified here and in
   `pipeline.md`; validation is specified here and in `hooks.md`. If the two descriptions of one command
   differ, builders will implement two. Mitigation: the differences found while writing are listed in
   [[topic:seams-state]], and the build plan should give the state package and the policy tables one owner.

## Platform behaviour this design relies on and has not seen

Each needs a recorded run before the build depends on it.

| Assumption | If it fails |
|---|---|
| A hook may run `git` in the state worktree within its time limit on `PreCompact`, and in the background on `SubagentStop`. | Flush runs only inside `vsdd` commands, which already covers every step boundary. |
| The Grep tool honours `.git/info/exclude`. | The state design adds a `.ignore` file at the project root naming `.factory/`, which the hook design already anticipated. |
| Agents can read files under `.factory/` by absolute path from a story worktree without a permission prompt. | The hook design's `auto-allow` list covers the state directory ([[topic:tool-call-approval]]). |
| `git worktree` commands by Claude Code's own isolation do not prune or disturb a worktree mounted at `.factory`. | `vsdd status --fix` remounts; the mount test catches it at the next call. |

## Team use (decision D33; built when the build widens)

Risk 4 is answered for a team in [[topic:team-records]]: the checkpoint also pushes the branches of
stories in flight and the red record, and a hand-off continues from there. Two clones no longer meet
only at the push ([[topic:team-claims]]).
