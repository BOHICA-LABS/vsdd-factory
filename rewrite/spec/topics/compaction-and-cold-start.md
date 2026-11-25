---
kind: topic
id: compaction-and-cold-start
sources: [BLUEPRINT.md#7, state.md#2.10]
---

# Compaction, warm-up and cold start

The design principle is that nothing important lives only in a conversation. The main session holds a few
hundred bytes per workflow run; orchestration lives in scripts and in agents with their own contexts; and
the tool writes state at every step boundary.

## What is flushed before compaction

The hook rule `compaction` calls flush with reason `precompact` ([[topic:state-commits]]). There is usually
little to commit, because flush already ran at the last flush point: the start of a survey, a checkpoint, a
delivery. Nothing else needs saving at that moment, by construction:

| What a main session knows | Where it already is |
|---|---|
| Which workflow is running and for what | The run lock and the run record, written by the survey |
| What each finished step returned | Step records, written by the hook runner as each step returned |
| What a person decided | Decision records, written when the person typed the command |
| Something a person said that must survive | Only if it was saved with `vsdd note`. The `status` skill tells the main session to do that when a person gives a standing instruction. |

## What happens in each case

**A session starts in an initialised project (warm-up).** On disk: tracked state and the local store. What
happens: the briefing ([[topic:briefing]]) — the project and cycle, the position, decisions waiting on you
with the command to settle each, what is running, the exact next command, open named spikes, notes, and any
relaxed gate or configuration change.

**A step agent compacts.** On disk: its worktree, its commits, earlier step records. What happens: the hook
gives the summariser instructions written for a step: keep the step identity, the worktree, the story, the
closing gate command and the findings word for word. The step prompt also repeats the identity and closing
command at its end.

**The main session compacts while a workflow runs.** On disk: everything; flush ran in `PreCompact`. What
happens: the workflow keeps running. The briefing after compaction says which run is live and that the
driver was active, and tells the main session to continue it and how.

**The session dies in the middle of a wave.** On disk: step records and evidence up to the last finished
step; closures up to the last checkpoint; story branches and worktrees. What happens: the next briefing
says a run started and did not finish. If the old session ended normally its lock is already released.
After a hard crash the briefing names `/vsdd-factory:status --fix` (and, for a terminal, `vsdd lock
release`), or the lock lapses after 20 minutes ([[topic:run-lock]]). The same command then starts a new
run; its survey skips what is done, continues each story from its position, and re-attaches to any gate or
delivery job the dead session left running, because a job is keyed by its command, subject and commit and
so is never started twice.

**A new session, days later.** On disk: tracked state and the local store. What happens: the briefing gives
the position and the next command.

**A fresh clone on another machine.** On disk: tracked state only, after `setup` mounts it. What happens:
the survey resumes from the last checkpoint and measures gates again where evidence is missing. A story
branch that was pushed has a pull request the tool opened, with the story trailer, and is adopted
([[topic:run-lock]]).

**The mount is broken.** On disk: the state branch. What happens: the condition is `partial`. `vsdd status
--fix` remounts. Uncommitted files in the old directory are kept and reported.

**`uv` or Python is missing.** On disk: n/a. What happens: guarded tool calls are blocked with the terminal
commands that fix it. The briefing says the gates are down.

One entry point, `/vsdd-factory:status` ([[command:status]]), reports, repairs and names the next command;
it replaces the old wrap, wave-handoff, wave-reset, rehydrate-wave, recover-state and compact-state skills.
To stop, stop: there is nothing to save by hand.

## The rewrite itself follows the same rule

`rewrite/STATE.md` is the resume record, workflow results are written under `rewrite/` before a step is
marked done, and each build package is a unit that can be re-run.

## Team use (decision D33; built when the build widens)

Not on the thin slice path. "A fresh clone on another machine" is, on a team, a clone that joins
([[table:team-checks]], T-JOIN) and then takes a story by claim; what it continues from is in
[[topic:team-records]]. With team use off it mounts as before and is told that another clone wrote the
state. The briefing gains the claim lines and the outcome of the last sync, read from files
([[topic:briefing]]).
