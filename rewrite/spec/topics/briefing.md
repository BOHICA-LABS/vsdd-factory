---
kind: topic
id: briefing
sources: [state.md#2.9, BLUEPRINT.md#7]
---

# The session briefing

The hook rule `session-briefing` prints it on every `SessionStart` (startup, resume, clear, compact and
fork). This design owns the text and the limit; the function is `vsdd.state.briefing(root, session_id,
source)`, which `vsdd status` also calls, so the two cannot disagree.

How it is computed. `briefing()` renders the status object of [[topic:status-report]], which is computed
from the records each time: `factory.toml`, `local/lock.json`, `local/driver.json`, the open cycle's
`cycle.json`, its closure records and open decision records, the open named spikes, `notes.json`, and then
the route. That is tens of small file reads and no subprocess. The mount test of
[[topic:state-directory]] adds two more reads. It returns in milliseconds; the hook design gives the
briefing eight seconds.

Content, in this order. Lines are dropped from the bottom if the limit would be passed.

```
vsdd-factory: acme-api. Cycle auth-sso (feature).
Build: wave W-02, 3 of 5 stories delivered, 1 blocked. Specification and plan are closed.
Waiting on you (1): finding.S-7.04  Waive or direct: lockout counter race
  Read .factory/cycles/auth-sso/decisions/finding.S-7.04.md, then /vsdd-factory:decide finding.S-7.04 <choice>
Running: wave W-02, started 14:02 in this session. Do not start it again. The driver is active: continue it.
Next: /vsdd-factory:wave
Named spikes open: uv-startup
Note: PR 41 needs the staging secret before it can merge.
Full report: /vsdd-factory:status
```

| Line | Shown when | Source |
|---|---|---|
| Project and cycle | Always. With no cycle: "No cycle is open." | `cycle.json` |
| Position | A cycle is open | The cycle's closure records |
| Waiting on you | A decision is open. At most three, then a count. Identifiers are qualified with `@<cycle>` when two cycles are open. A `blocked` decision's line states its reason. | The open decision records |
| Running | The run lock is held by a live run. It says whether that run belongs to this session, and whether the driver is active or paused (`pipeline.md` S9). | `local/lock.json`, the run record, `local/driver.json` |
| Next | Always | The route, rendered from the `next` object |
| Named spikes | Any are open | The spike ledger |
| Note | Any exist. At most two. | `notes.json` |
| State problems | The condition is `partial` (the mount is broken); the last push failed; `[commands]` is not configured; a rule has repeated `rule.error` events ([[topic:hook-failure]]) | The mount test; the push result the checkpoint recorded in the run record; `factory.toml`; the event log |
| Full report | Always | |

## The driver lines

When `local/driver.json` names this session, the "Running" line is followed by the instruction that lets a
compacted main session carry on without the person. When the driver is on: "The driver is active. When the
running workflow returns, start the command in `next` with the Workflow tool and repeat; at a decision for
a person run `vsdd driver pause <id>` and stop; at done, or after two runs with no progress, run `vsdd
driver off`." When the driver is paused at a decision: "The driver is paused at `<id>`. When that decision
is settled, resume the driver loop." The context the `decide` hook returns carries the same instruction, so
after an approval the person does not type `/vsdd-factory:drive` again. `vsdd driver on`, `pause` and `off`
write that file ([[command:driver-on]], [[command:driver-pause]], [[command:driver-off]]). Queued change
requests are shown as a count on the position line.

## After a compaction

After a compaction the first line is preceded by: "Context was compacted. This briefing is computed from
disk and is current." The running workflow, if any, is named by the "Running" line from the lock and the
run record; there is no compaction snapshot to read. The hook rule may then add its own lines (relaxed
gates, a changed gate-bearing section).

Limit: 1,200 characters and 12 lines for this design's part, inside the hook rule's own ceiling of 2,000.
A subagent never receives it. It contains no file contents and nothing from the holdout class.

## Team use (decision D33; built when the build widens)

Not on the thin slice path. Two lines are added after "Running", both read from files with no fetch:
"Yours:" with the work claims this clone holds, and "Elsewhere:" with those other clones hold, each with
its person and its clone's label, at most three and then a count. "State problems" gains the last
sync's outcome when it is not `synced`, anything unpublished, unknown or set aside, a push whose lost
answer a sync has since settled, a claim this clone lost, and a count of hand commits. A decision settled here and not yet published is listed as that, and
not under "Waiting on you" ([[record:clone]], [[record:work-claim]]). The limit is unchanged, so lines
still drop from the bottom.
