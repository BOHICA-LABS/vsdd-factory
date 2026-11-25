---
kind: topic
id: platform-proven-3
sources: [BLUEPRINT.md#10]
---

# Round-5 platform probes (items 50 to 55)

Continuation of [[topic:platform-proven-2]] (items 36 to 49). Round 5 ran on 2026-11-25, and an
independent check reran leg A whole and leg B at two caps the same day. What holds for every item below
unless the item says otherwise:

- **Version.** Claude Code 2.1.332, git 2.50.1, macOS (Darwin 25.5.0, arm64), headless `claude -p`.
- **Install.** A throwaway plugin loaded with `--plugin-dir`, with no skill in it, and a scratch project
  under `$HOME` with no settings file. No marketplace-installed plugin was probed.
- **Mode and flags.** Permission mode `default`. Each session was started by a typed workflow command and
  carried `--allowedTools "Workflow Agent Bash Read Write Edit Glob Grep"`, `--settings
  '{"sandbox":{"enabled":false}}'`, `--setting-sources project,local`, `--strict-mcp-config`, `--model
  sonnet`, `--effort medium`, `--output-format json`, standard input from `/dev/null`, and
  `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0` in its environment. Leg D differed (below).
- **Evidence.** `rewrite/spike/probes-round-5/`: one folder per leg, `COMMANDS.md`, the prober's
  `RESULTS.md`, and `CHECK.md`, the check's report, which holds where `RESULTS.md` differs. "Rerun" means a
  rerun by the check (`CHECK.md`, section 3): the same command in a fresh scratch directory with the
  calling session's `CLAUDE_*` variables unset. Its logs are outside the repository; `CHECK.md` quotes the
  lines that decide.

## Leg D — the round-4 rerun A, run again on 2.1.332

Leg D ran the workflow of the round-4 check's rerun A (items 36 to 38 and 40 of
[[topic:platform-proven-2]]) once; the round-5 check read its logs and did not rerun it. Its script is the
round-4 check record's with the plugin prefix and the project path changed. Three things differ from the
round-4 rerun:

- it was entered by the typed workflow command, where the round-4 rerun went through the skill `probecall`;
- it ran under the two isolation flags, the sandbox-off `--settings` flag and the ceiling variable, none
  of which the round-4 rerun carried; its tool list was `Workflow Bash(echo *)` and its session effort `low`;
- R3B's rewrite was not rebuilt, so on this version item 38's `StructuredOutput` leg rests on R2A and R2B
  (`allow` plus `updatedInput` on a `StructuredOutput` call).

Held again: item 36 (`agent_type` exactly `<plugin>:<agent>`, one `agent_id` per step, the run record's
ids; main-session events have neither key); item 37's open and declared legs; item 38's Bash leg and its
refused control, in one session; item 40 (the hook sees a result first; the wrapped block message is heard
whole); item 48 (every step payload carries `effort.level`). Changed: item 37's closed-schema leg still
fails the step, with another count, error and time; item 37 gives both versions.

## Item 50 — the typed workflow command starts the named workflow with no skill

Proven. Seen: the mode and flags above; rerun A the same. Evidence: `leg-A/A-result.json`,
`leg-A/A-part1.jsonl` (lines 1 to 3), `leg-A/platform/wf_bb1971a5-1cd.json`.

- The whole prompt `/p5:one <plugin dir>` was expanded by the platform (a `UserPromptExpansion` event,
  `command_name: p5:one`, `command_source: plugin`) into an instruction to call `Workflow` by that name.
- The main session then made exactly one tool call, `Workflow {"name": "p5:one", "args": "<plugin dir>"}`:
  `num_turns` 1 and no `Skill` call. So `Skill` is not needed in a driver's tool list in the default mode.
- `permission_denials` is empty, which is evidence for a main-session call: a refused one is listed there
  (item 30). For a step it is not evidence (item 52).
- The typed text reached the script's `args` as one string, through the model's own call (item 31).
- The run record ended `completed`, and its `script` and the file at its `scriptPath` equal the plugin's
  `one.js` byte for byte; the same held for `three` and for both workflows of the rerun. The inline
  script of item 49 did not occur in these sessions. That does not rule it out: a driver still checks the
  run record.

## Item 51 — a `--plugin-dir` plugin loads whole under the two isolation flags

Proven. Seen: `--setting-sources project,local --strict-mcp-config`, a project with no settings file;
rerun A the same. Evidence: `leg-A/A-part1.jsonl`, `leg-A/platform/wf_bb1971a5-1cd.json`; `CHECK.md`,
sections 2 (A1) and 3.

- Loaded: the plugin's hooks fired (prompt expansion, then `PreToolUse` and `PostToolUse` in the main
  session and in the step); its workflow ran to `completed`; its `bin/` tool ran by bare name in a step's
  Bash call (`BIN-MARKER-R5`; the tool is on no other `PATH`).
- Kept out: the session's skill listing held the probe plugin's commands and built-ins only, none of the
  ten plugins the person's `~/.claude/settings.json` enables; no `mcp__` tool appeared; every payload says
  `permission_mode: default`, where the person's settings set `defaultMode: auto`.
- Not kept out (rerun A): the person's user-level `~/.claude/CLAUDE.md` was still injected, into the main
  session and into the step's transcript.
- The sandbox-off `--settings` flag was honoured: the step's commit in a linked worktree wrote under
  `.git/worktrees/x/` and succeeded, which the sandbox refuses (item 41).

## Item 52 — bare tool names admit a step's read, write, edit and commit; a step's refusal is not in `permission_denials`

Proven. Seen: bare names only in `--allowedTools` (no `Bash(...)` pattern), one step; rerun A the same
calls. Evidence: the `PostToolUse` lines of `leg-A/A-part1.jsonl` and `A-part2.jsonl`, each with the tool's
own response, and git in the probe project.

What one workflow step did, each call with its `PostToolUse` line, and no more than this was shown:

- a `Read` of a file under the plugin, outside the working directory (line 9: `METHOD-MARKER-R5`);
- a `Write` that created a file in a linked worktree under the project (line 11, `type: create`), then a
  `Read` and an `Edit` of another file there (lines 13 and 15);
- one compound `Bash` call, `git -C <worktree> add -A && git -C <worktree> commit -m leg5` (line 17):
  `[story/x e2b67c6] leg5`. Git shows that commit on `story/x` with both files, the worktree clean.

Rerun A: the same calls, commit `a61f0a6`. Not shown: `Glob`, `Grep` or `Agent` under a bare name; a
write outside the project; a second step.

**A refused call of a workflow step does not appear in the session result's `permission_denials`.** Seen:
leg D (tool list `Workflow Bash(echo *)`), not rerun. The R3CTRL step's `Bash` call was refused: its step
transcript holds a `tool_result` with `is_error: true` and the text `This command requires approval`; the
hook log holds the call's `PreToolUse` payload and no `PostToolUse` for its `tool_use_id`; and
`leg-D/D-result.json` has `permission_denials: []` with `subtype: success`. The round-4 rerun A shows the
same on 2.1.331.

The transcript record that holds a `tool_result` (`type: user`) also carries a key `permissionDecision`.
`CHECK.md` does not cite the key. Evidence: `step-transcripts/`, which holds, for each step transcript the table names
(`agent-<id>.jsonl`, then the record's line), every record with a `tool_use` or `tool_result` entry, whole,
with its line number:

| The call | Transcript, line | `is_error` | `permissionDecision` | `toolDenialKind` |
|---|---|---|---|---|
| Refused for want of an allow rule (leg D, R3CTRL) | `a5caa4d535b10df94`, 15 | `true` | `decision: reject`, `source: config`, `reasonCode: bashMiss:no-rule-match` | `user-rejected` |
| Blocked by a hook's exit 2 (leg D, R5B) | `a4bd7d5edd93227d3`, 15 | `true` | `decision: reject`, `source: hook`, `reasonCode: hook:PreToolUse` | `permission-rule` |
| Admitted by an allowed tool name (leg A, rerun A) or a hook's `allow` (leg D, R3A) | `a3338c7a916b05723` and `afbe9cef34f0bc21c`, every result in a `type: user` record; `a48bb837317fa3653`, 16 | `false` or absent | `decision: accept`, `source: config` or `hook` | absent |
| Admitted, then refused by the schema check (leg D, R2C) | `ac674367fa72bb811`, 16, 23, 30, 37, 44 | `true` | `decision: accept`, `source: hook` | absent |
| Cut off by a budget stop (item 54, the rerun at 2 cents) | `aeb3de9ef0ed5cf90`, 15 | `true` | absent | `user-rejected` |

- So a driver finds a step's refusal in the step transcripts, in their records of `type: user` only: a
  `tool_result` with `is_error: true` whose record has `permissionDecision.decision` `reject` from a
  `source` other than `hook`, or whose text has `requires approval`. The transcript is the agent's, and the
  `tool_use` entry with the result's `tool_use_id` names the tool and its input.
- Records of other types are left out: a result can be repeated in one with none of these keys (item 55),
  and the text leg would name a repeated refusal twice.
- `toolDenialKind` alone does not mark a refusal: a call cut off by a budget stop carries `user-rejected`
  too, with the text `The user doesn't want to proceed with this tool use.`
- A `PreToolUse` payload with no `PostToolUse` is not by itself a refusal either: a hook's block (item
  40), a result the schema check refused (item 37) and a call cut off by a budget stop leave that trace.
- Not shown: a refused `Read`, `Write` or `Edit` in a step, so neither its text nor its `reasonCode`.

## Item 53 — a step's payloads carry the effort the step was dispatched with

Proven. Seen: session `--effort medium`; the prober's step had `effort: low` in its agent file and
`effort: 'low'` in its `agent()` call, the rerun's `xhigh` in the agent file and `low` in the call.
Evidence: `leg-A/A-part1.jsonl`, `leg-A/A-part2.jsonl`; `CHECK.md`, section 3 (rerun A).

- Prober's run: all 16 payloads of the step carry `effort: {"level": "low"}`. That run cannot say whose
  value it is, the agent file's or the call's.
- Rerun A separates them: all 14 payloads of the step carry `low`. A step's payload carries the effort the
  script dispatched the step with, not the session's and not the agent file's.
- A main-session tool payload carries the session's effort: the `PreToolUse` and `PostToolUse` of the
  `Workflow` call carry `medium` in leg A and in the rerun, and `low` in leg D, whose session ran at `low`.

So a payload tells a step's effort from the session's when the two differ, and a check can read the
effort a step ran under from its payloads (item 48). Item 26's own question was not probed: no step was
dispatched without an effort, so whether such a step takes its agent file's is not shown. The design does
not rely on it: every step is dispatched with its effort ([[doc:conventions]]).

## Item 54 — a `--max-budget-usd` stop: two shapes, and the cap is soft

Proven, each shape seen with three small steps in a row (workflow `three`). Evidence: `leg-B/B-result.json`,
`leg-B/B-rc.txt`, `leg-B/platform/wf_402584c1-f2f.json` (cap `0.05`); `CHECK.md`, section 3 (rerun B at
`0.05`, and at `0.02`).

The session result is the same in both shapes: exit code 1, `subtype: error_max_budget_usd`, `is_error:
true`, `errors: ["Reached maximum budget ($<cap>)"]`, `terminal_reason: budget_exhausted`, and **no
`result` key at all**. The run record and standard error tell the shapes apart:

- **The cap was passed after the workflow ended** (cap `0.05`; the prober's run and rerun B). The spend
  crossed the cap only with the main loop's second answer. The run record reads `status: completed` with
  the workflow's whole return value and every agent `done`; standard error is empty; `num_turns` 1.
- **The cap was passed while a step ran** (cap `0.02`; rerun only). Standard error: `Budget limit reached
  ($0.05 of $0.02); stopping background agents.` The run record reads `status: killed`, `result: null` and
  a key `error` (`Error: Workflow aborted`, with a stack); the first step is left `state: progress` with
  `lastToolName: StructuredOutput` and no `durationMs`; the later steps never started; the journal has
  `launched` and `started` and no `result`; `num_turns` 2. The hook log holds that step's `PreToolUse
  StructuredOutput` payload with no `PostToolUse`: a hook saw a result the run record never got.

The cap is soft: the stop comes after the cap is passed, and what is in flight is charged. Spend was
$0.0578 and $0.0651 on a 5-cent cap, and $0.0544 on a 2-cent cap (2.7 times). How far a large step
overshoots was not measured.

So a driver reads `result.get("result")`, reads the run record on this `subtype`, and treats a cap as a
ceiling that one session's in-flight calls can pass. Recorded as PR5-1.

## Item 55 — where a session's platform records live, and their shape

Proven. Seen: the kept round-4 folder (2.1.331), leg A's folder and the folders of the three reruns
(2.1.332), all of one shape; the run records and journals of legs B and D agree. Evidence:
`leg-C/LEG-C.md`, `leg-A/platform/`, `leg-B/platform/`, `leg-D/platform/`, `step-transcripts/`; `CHECK.md`,
section 2 (C).

**Finding the folder.** It is `<claude home>/projects/<a name made from the project path>/<session id>/`,
and every hook payload's `transcript_path` is that folder's path plus `.jsonl` (the main transcript beside
it). A reader takes `transcript_path` without `.jsonl`, or globs `projects/*/<session id>/`: no naming rule
is needed. Of the name, only this was shown: each `/` and each `.` of the absolute project path is
replaced by `-`. No probed path held any other character outside letters, digits and `-`.

**In the folder.**

- `workflows/wf_<run id>.json`, one per workflow run: `script`, `scriptPath`, `status`, `result`,
  `agentCount`, `workflowProgress`, and more. `workflowProgress` starts with `workflow_phase` entries; then
  one `workflow_agent` entry per agent that started holds `agentId`, `agentType` (`<plugin>:<agent>`), `model` as a full
  id (`claude-sonnet-5-5`), `state` (`done`, `error`, or `progress` in a killed run) and `label`.
- A `killed` record has `result: null` and adds a key `error` (item 54). A `completed` record can hold a
  step that failed: that agent's `state` is `error` (leg D).
- `workflows/scripts/<workflow name>-<run id>.js`, the file `scriptPath` names: the bytes of `script`. For
  a call by name both equal the plugin's file (item 50).
- `subagents/workflows/<run id>/agent-<agent id>.jsonl`, a step's transcript, with `agent-<agent
  id>.meta.json` and one `journal.jsonl`. The journal has one `launched` line for the run, then a `started`
  line and a `result` line per agent (`failed` for a failed step; none for a step cut off).
- A step transcript's `tool_use` entries (tool name and input) are in its records of `type: assistant`.
  Some are repeated in records of `type: api-request-blob` (leg A: 8 ids in 10 entries, each id also in an
  `assistant` record). A reader takes the `assistant` records only, or dedupes by id.
- A `tool_result` is in a record of `type: user` (the record's own `type` key), with its `tool_use_id`,
  and that record carries the keys of item 52. Some results are repeated too, in `api-request-blob`
  records whose `message.role` is `user`, and a copy carries none of those keys. Leg A's transcript
  repeats at line 30 the results of its lines 25 and 27 (an admitted `Write` and `Read`; 8 ids in 10
  entries); rerun A's repeats at line 28 those of its lines 23 and 25. No result of item 52's table with
  `is_error: true` has a copy.
