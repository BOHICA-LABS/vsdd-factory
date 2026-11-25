---
kind: rule
id: compaction
package: H9
sources: [hooks.md#2.10, hooks.md#2.11.9, BLUEPRINT.md#5.4]
---

# compaction

Runs on `PreCompact` (manual and automatic). It flushes state and tells the summariser what to keep;
it writes no snapshot file.

| Registration | |
|---|---|
| Kind | recorder |
| Runs on | `PreCompact` (mode `notify`, timeout 30 s, runner deadline 25 s of which the state flush may use 20 s) |
| Result | `Context`: plain stdout on `PreCompact`, where Claude Code uses it as instructions for the summary |
| Fails | open, announced — "blocking automatic compaction can fail the request outright (hooks documentation). The next briefing reports the failed flush." |
| Relax | no |

## What it does

1. It runs the state flush (need S8), with a 20 second limit.
2. It prints a short instruction for the summariser on stdout, which Claude Code uses as instructions
   for the compaction summary. The instruction depends on who is compacting. For the main session
   (empty `agent_type`): keep the project root, the current phase, the in-flight item and the exact
   next command, and note that a fresh briefing will follow. For a subagent or workflow step, which
   receives no briefing: keep word for word the step identity block, the worktree path, the story id,
   the closing gate command and the findings or claims the step was given. The workflow prompt also
   repeats the identity and the closing command at its end, so they survive even if this event does not
   fire for a step.

It never blocks compaction. If the flush fails, the event `compaction.flush_failed` is logged and the
next briefing says so. `PostCompact` is not wired: the briefing after compaction arrives through
`SessionStart` with source `compact`, which also logs `compaction.done`.

See also [[topic:compaction-and-cold-start]].
