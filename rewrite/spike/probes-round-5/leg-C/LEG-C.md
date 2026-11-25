# Leg C — the platform's session folder (no session of its own)

2026-11-25, Claude Code 2.1.332, macOS. Compared: leg A's folder (session `4265e389-8597-4ead-b675-cfdb38acdbfa`,
project `/Users/jmagady/.vsdd-probes/round-5.QkGwwK/project`) with the kept round-4 folder
`~/.claude/projects/-private-tmp-claude-501--Users-jmagady-dev2-vsdd-factory-f468fc16-2190-4e04-8bbb-50654bb31906-scratchpad-pr4-project/1cf1064b-4e2f-49c6-92e9-e93287927cfa/`
(workflow `q9`). Copies of the leg-A and leg-B platform files are in `../leg-A/platform/` and `../leg-B/platform/`.

## Where the folder is found

`<claude home>/projects/<sanitized project path>/<session id>/`, where the sanitized project path is the
project's absolute path with every character outside `A-Za-z0-9_-` replaced by `-`:

- `/Users/jmagady/.vsdd-probes/round-5.QkGwwK/project` → `-Users-jmagady--vsdd-probes-round-5-QkGwwK-project`
  (`/.` before `vsdd` becomes `--`, the dot in `round-5.QkGwwK` becomes `-`).
- `/private/tmp/claude-501/-Users-jmagady/.../scratchpad/pr4-project` →
  `-private-tmp-claude-501--Users-jmagady-...-scratchpad-pr4-project` (round 4).

A sibling file `<session id>.jsonl` (the main-session transcript) sits beside the folder.

## Shape, both folders

- `workflows/wf_<run id>.json` — one per workflow run, holding `script` (the full text that ran),
  `scriptPath` (a saved copy under the session's `workflows/scripts/<name>-<run id>.js`), `status`
  (`completed` in both), `result` (the workflow's return value), `agentCount`, `durationMs`,
  `defaultModel` (a full id, `claude-sonnet-5-5`), `workflowName`, `startTime`, `totalTokens`,
  `totalToolCalls`, `phases`, `logs`, `summary`, `taskId`, `timestamp`, and `workflowProgress`: first
  entries `type: workflow_phase` (`index`, `title`), then one `type: workflow_agent` per agent with
  `label`, `phaseIndex`, `agentId`, `agentType` (`<plugin>:<agent>`), `model` (the full id), `state`
  (`done`; leg D shows `error` too), `attempt`, `startedAt`, `queuedAt`, `lastToolName`,
  `lastToolSummary`, `promptPreview`, `promptFramed`, `lastProgressAt`, `tokens`, `toolCalls`,
  `durationMs`, `resultPreview`.
- `workflows/scripts/<workflow name>-<run id>.js` — the saved script; identical bytes to `script`.
- `subagents/workflows/<run id>/agent-<agent id>.jsonl` — the step's transcript, records with
  `agentId`, `isSidechain: true`, `promptId`, and `message.content` holding the `tool_use` entries
  (tool name, input) and their results; plus `agent-<agent id>.meta.json` (`agentType`, `model`,
  `inWorkflowRun: true`, `spawnDepth`, ...) and `journal.jsonl` (one `launched` line per agent, a
  `result` line per agent return).

Identical shape on both versions probed (2.1.331 round 4, 2.1.332 round 5).

## By-name calls

For a call by name the record's `script` and the file at its `scriptPath` equal the plugin's file byte
for byte, checked with cmp-equivalent comparison in Python:

- Round 4, `q9`: `script == plugin/workflows/q9.js` and `script == scriptPath file` — both true.
- Round 5 leg A, `one`: `script == ../leg-A/plugin/workflows/one.js` and `script == scriptPath file`
  — both true (leg B `three` likewise).

So a typed command with no skill resolves the named plugin workflow and persists exactly its text.
The step transcripts carry `tool_use` entries: leg A's `agent-a3338c7a916b05723.jsonl` shows the Bash,
Read, Write, Edit and StructuredOutput calls the hook log recorded.
