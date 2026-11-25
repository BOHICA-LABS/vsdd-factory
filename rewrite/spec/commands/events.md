---
kind: command
id: events
package: T3
sources: [BLUEPRINT.md#4.9, packaging.md#2.10]
---

# `vsdd events`

Operator tool for the event log ([[topic:event-log]]): canned queries, replay, reports and agent durations. Reads
take `--json` like every command that reads ([[topic:command-conventions]]).

- `vsdd events top`, `recent`, `grep`, `rules`, `stats`, `reasons` — canned queries over the event log.
- `vsdd events replay [SESSION]` — plays back one session's hook activity in order.
- `vsdd events report daily`, `weekly`, `range` — Markdown reports.
- `vsdd events durations`, `summary`, `open` — agent run times from start and stop pairs.

There is no `vsdd events emit`. Nothing in 1.0 called it: the hook runner and the tool write events in process,
through one writer. With no shell entry point, only they can write the log, so no list of reserved event types is
needed to protect it.

## Who may call it

Run by the main session.

Built by package T3 ([[package:T3]]) together with the observability stack
([[packaging:observability-stack]], [[command:obs]]).
