---
kind: topic
id: event-log
sources: [hooks.md#2.12, BLUEPRINT.md#4.1]
---

# The event log

Where it is: daily files, one JSON object per line, in the directory the project configuration names
(working name `.factory/local/events/<UTC date>.jsonl`; `VSDD_EVENTS_DIR` overrides it for tests).
BLUEPRINT.md#4.1: `.factory/local/events/<UTC date>.jsonl`, and only the hook runner and the tool
write it.

`vsdd/events.py` is the one writer in Python, used by the runner and by the tool, in process. The launcher
writes two event types itself, in shell, because Python has not started when they happen:
`runner.launch_failed` and `runner.disabled` (the emergency switch, written on every call it skips). A
launcher-written line carries `v`, `ts` in whole seconds, `type`, `severity`, `session_id` as `null` (the
launcher does not parse the hook input), `code` and `mode`. It goes to the same directory, and honours
`VSDD_EVENTS_DIR` the same way. The schema admits both forms, so a reader needs no special case. There is no
`vsdd events emit`: no 1.0 agent, skill or workflow used it, and the reserved event-type list existed
only to protect the log from that command. With no shell entry point, only
the runner and the tool can write the log, whose directory is in a class only code writes, so the
quarantine switch can trust the `rule.error` events it reads.

Each line is one `write` on a file opened with `O_APPEND`, so concurrent agents do not interleave.
Lines are capped at 4 KB. Calls from worktrees log to the main project's file, because the root is
resolved to the main checkout.

## Format

The line format is [[schema:event]]; every event type with its extra fields is
[[table:event-types]]. Every line has `v`, `ts` (ISO 8601, UTC, to the millisecond from Python), `type`
(dotted), `severity` (`info`, `warn`, `block`, `error`) and `session_id`. Lines about a tool call add `agent_type`, `agent_id`,
`event`, `tool` and `ms`. These are the field names `packaging.md` assumes in its need H3.

One `rule.block` line, whole, as the design's example (the fixture
`tests/unit/fixtures/schemas/event/valid_rule-block.json` is this line, spelled as the writer
emits it, and the `story` and `cwd` fields ride in the schema's properties):

```json
{"v":1,"ts":"2026-11-24T09:31:12.512Z","type":"rule.block","severity":"block",
 "session_id":"sess_9f83ab","agent_type":"vsdd-factory:implementer","agent_id":"agent-7",
 "event":"PreToolUse","tool":"Edit","ms":9,"rule":"tests-first","code":"no_red_evidence",
 "id":"b-7f3a2c","reason":"src/lib.rs is product source in story S-1.02, and no failing test
 run is recorded for that story.","fix":"write the failing test, then run the red gate: vsdd
 gate red --story S-1.02","target":"/repo/.worktrees/S-1.02/src/lib.rs","story":"S-1.02",
 "cwd":"/repo/.worktrees/S-1.02"}
```

Allowed calls are not logged one by one; that would be most of the file and nobody reads it. Generic
tool and cost figures come from Claude Code's own telemetry. Setting `VSDD_FACTORY_HOOKS_DEBUG=1` logs
every call as `runner.call` with its duration, for diagnosis.

## Hygiene

File contents are never logged. Commands are cut to 200 characters and passed through a redactor that
masks `NAME=value` pairs with secret-shaped names and common token formats. Files older than
`log_retention_days` (default 30) are deleted at session end. If a day's file passes 20 MB, only
blocks and errors are written for the rest of the day.

## Explaining a block

Every block has an id that appears in the message the model sees and in the log.

```
$ vsdd explain b-7f3a2c
Blocked: Edit of .worktrees/S-1.02/src/lib.rs by vsdd-factory:implementer at 21:40:03
Rule:    tests-first (gate, can be relaxed in [hooks.rules.tests-first])
Why:     no failing test run is recorded for story S-1.02
Fix:     write the failing test, then run the red gate: vsdd gate red --story S-1.02
Looked at: evidence store, gate "red", subject S-1.02 (no passing record); paths.source = ["src/**"]
```

`vsdd explain --last` shows the most recent block of the current project. `vsdd events schema` prints
the JSON Schema that the writer validates against, so the documented schema cannot drift from the
code.

The event log directory is one of the runner's needs on the state design (need S2): both it and the
runner's own records directory must be in a path class with `write = "code"`
([[table:path-classes]]).
