---
kind: rule
id: lifecycle-log
package: H9
sources: [hooks.md#2.10, hooks.md#2.11.10, BLUEPRINT.md#5.4]
---

# lifecycle-log

Records agent and session lifecycle, keeps the run lock alive, releases it at session end and stops
products an agent left running. It records no commit or pull-request event: the tool writes those.

| Registration | |
|---|---|
| Kind | recorder |
| Runs on | `SubagentStart`, `SubagentStop`, `SessionEnd`, and every guarded call for the lock's liveness |
| Result | records only (event log; `Record` in the rule table) |
| Fails | open — recording only |
| Relax | no |

## What it does

- It records `agent.start` and `agent.stop` for factory agents and workflow steps, and `session.end`.
- It keeps the run lock alive: on a call that belongs to the locked run, as [[topic:run-lock]] defines
  it, it refreshes the lock's `seen` time through the state design's `lock_seen`, at most once a
  minute, and on `SessionEnd` it releases a lock that session holds through `lock_release`.
- On `SubagentStop` it stops any product process that agent started with `vsdd env start` and left
  running; the checkpoint stops whatever is left.
- On `SessionEnd` it also deletes event files older than the retention period. Claude Code gives plugin
  `SessionEnd` hooks 1.5 seconds, which a directory listing fits easily.
- It records no commit or pull-request event: the tool writes `pr.created` and `pr.merged` when it
  delivers a story, and the gates write `commit.made` for the commit they measure.

The run lock and liveness are [[topic:run-lock]]; who is calling is [[topic:caller-identity]].
