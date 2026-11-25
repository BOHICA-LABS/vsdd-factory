---
kind: rule
id: session-briefing
package: H9
sources: [hooks.md#2.10, hooks.md#2.11.8, hooks.md#2.6]
---

# session-briefing

Composes and prints the session briefing at session start.

| Registration | |
|---|---|
| Kind | recorder (it owns how the briefing is produced and delivered) |
| Runs on | `SessionStart`, every source: startup, resume, clear, compact and fork (mode `brief`, timeout 15 s) |
| Result | `Context` (`additionalContext`); logs `session.start` |
| Fails | open, announced — "blocking a session start helps nobody; the fallback text says the briefing failed" |
| Relax | locked (decision D12) |

The spike proved that `additionalContext` reaches the model for startup, resume and compact.

## Ownership split

The state design owns what the briefing says and its length limit, and `packaging.md` asks that the
same function back `vsdd status` (its need S5). This rule calls the state design's briefing function
(need S4), which computes the status from the records (there is no stored status record) and returns
the briefing lines: phase, what is in flight, open decisions and named spikes, the exact next command,
and whether the driver is active or paused. See [[topic:briefing]].

## What this rule adds

After a compaction it adds nothing of its own about what was running. The briefing's "Running" line
names the live workflow from the lock and the run record, and the survey reports worktrees with
uncommitted work when the workflow continues.

It adds runner lines only when there is something to say:

- gates that are relaxed;
- the main-session wall switched off;
- the `auto-allow` scope;
- a change to any gate-bearing section of the configuration since the last session (the fingerprint
  list of [[topic:rule-tuning]], not only `[hooks]`), including a write the tool made;
- a configuration or policy fault;
- another session that started in this project in the last 30 minutes and has not ended.

When all is normal it adds nothing.

It enforces the state design's length limit, and a hard ceiling of 2,000 characters of its own. It
logs `session.start`.

In a project with no configuration file none of this runs (see [[topic:hooks-inert]]).
