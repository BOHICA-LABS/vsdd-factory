# Check of the round-5 probe results

2026-11-25, Claude Code 2.1.332 (`~/.local/bin/claude`), git 2.50.1, macOS (Darwin 25.5.0, arm64).
Checked: `COMMANDS.md`, `RESULTS.md`, every file under `leg-A/` to `leg-D/`, the scratch directory
`~/.vsdd-probes/round-5.QkGwwK/` (scripts, logs, plugins, projects), the platform folders of the five
round-5 sessions under `~/.claude/projects/` (main transcripts, run records, step transcripts, journals),
the kept round-4 folder and the round-4 check's `r123` record, `rewrite/findings/sources/extra-probes-5.json`,
`spec/platform.md`, `spec/topics/platform-proven-3.md` and items 36 to 40 and 48 of `platform-proven-2.md`.
Against: `rewrite/plans/step-6/ORDER.md` 8.2, `first-run.md` 3.3 to 3.7 and D8/D11, `story-workflow.md` 3.1.

The check reran leg A whole, and leg B twice (section 3), in a fresh scratch directory
`~/.vsdd-probes/round-5-check.Y3aUf7/` (writable from this shell; nothing sandboxed it). The scratch is kept:
`legA.sh`, `legB.sh`, `legBm.sh`, `logs/`, `platform/` (copies of the three sessions' run records, step
transcripts and journals). Prompts carried probe text only. Spend: A $0.104, B $0.065, B at 2 cents $0.054.

## 1. Verdicts

| Leg | Reported | Supported by the recorded evidence? | Check rerun |
|---|---|---|---|
| A1 | pass | **Yes** | Confirms |
| A2 | pass | **Yes** | Confirms |
| A3 | pass | **Yes**, on the hook log and git; the cited empty `permission_denials` is no evidence for a step | Confirms |
| A4 | pass | **Yes for the answer** (`low`); two statements around it are wrong | Confirms, and separates call from agent file |
| B | pass | **No, as recorded.** The stop never fell inside a workflow; `result: ""` is wrong | Reproduced at 5 cents; at 2 cents the workflow is killed |
| C | pass | **Yes**, with two slips (the `_` rule, the journal) | Shape seen again in three more sessions |
| D | pass | **Yes**; item 37's failure changed shape on this version, and three deviations are under-reported | not rerun |

Checked 7, supported 6 (A1, A2, A3, A4, C, D). B is not supported as recorded: it answers the easier case
(a stop after the workflow ended), and the question was the stop in the middle of one.

**Flags, one by one, against ORDER-6 8.2** (`leg-A/run-leg.sh`, `leg-B/run-leg.sh`, identical to the
scratch `legA.sh`/`legB.sh`): `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0` yes; logger variable `P5_LOG=<S>/logs/<leg>.jsonl`
yes; prompt `/p5:one <S>/plugin` (typed command, absolute plugin directory) yes; `--plugin-dir <S>/plugin` yes;
`--permission-mode default` yes; `--allowedTools "Workflow Agent Bash Read Write Edit Glob Grep"` yes;
`--settings '{"sandbox":{"enabled":false}}'` yes; `--setting-sources project,local --strict-mcp-config` yes;
`--model sonnet --effort medium` yes; `--max-budget-usd 1` (A) and `0.05` (B) yes; `--output-format json
< /dev/null` yes; scratch under `$HOME` yes. Deviations, none of which weakens A1 to A3: the project has a
second commit (`1eb182d notes`, on `story/x` only; `main` has one); the round-4 runner was replaced by a
whole-payload logger (the round-4 one logs selected fields and no `effort`, so this was needed for A4);
`worker.md` sets `effort: low`, the same value as the `agent()` call, which is why A4 could not tell the
two apart; the scripts do not clear the calling agent session's environment, which in a subagent's shell
holds `CLAUDE_EFFORT`, `CLAUDE_CODE_CHILD_SESSION`, a custom-header variable and others (the prober's own
values were not recorded). The check's rerun cleared all of these and got the same answers, so they did
not change the result.

## 2. Per leg

**A1.** `leg-A/A-part1.jsonl` + `A-part2.jsonl` equal the scratch `logs/A.jsonl` byte for byte. Line 1
`UserPromptExpansion` (`command_name: p5:one`, `command_source: plugin`), lines 2-3 `Workflow`, lines 4-19
the step (`agent_type: p5:worker`, `agent_id: a3338c7a916b05723`). Line 5, `PostToolUse Bash` for the bare
`probe-marker`: `stdout "BIN-MARKER-R5"`; the tool is on no `PATH` of this shell, so the plugin's `bin/`
put it there. Run record `wf_bb1971a5-1cd.json` (equal to the platform's copy): `status: completed`,
`agentCount: 1`. Isolation is shown directly too, which `RESULTS.md` does not cite: the session's skill
listing (main transcript, `skill_listing` attachment) holds `p5:one`, `p5:three` and built-ins only, none
of the ten plugins the person's `~/.claude/settings.json` enables; no `mcp__` tool appears; payloads say
`permission_mode: default`, where the person's settings say `defaultMode: auto`. The sandbox-off reading
stands: line 17's commit wrote under `.git/worktrees/x/`.

**A2.** The main transcript has exactly one tool call, `Workflow {"name": "p5:one", "args": "<S>/plugin"}`,
made after the platform's own expansion of the typed command (transcript line 5: "Run the "p5:one"
workflow. ... Invoke: Workflow({ name: "p5:one", args: ... })"). No `Skill` call; `num_turns: 1`; `script`
and the file at `scriptPath` equal
`plugin/workflows/one.js` byte for byte (checked here in Python, and against the platform's own copy).
`permission_denials: []` is evidence here: a main-session denial is listed there (item 30).

**A3.** Supported by `PostToolUse` lines, each with the tool's own response: line 9 `Read
<S>/plugin/methods/probe.md` (outside `project/`) content `METHOD-MARKER-R5`; 11 `Write` `type: create`;
15 `Edit` with its patch; 17 `Bash` `git -C <S>/project/.worktrees/x add -A && git -C ... commit -m leg5`,
stdout `[story/x e2b67c6] leg5 / 2 files changed`. Git now: `e2b67c6 leg5` on `story/x` above `1eb182d`
and `2f7e7cf init` (`main`), both files with their new text, the worktree clean. The cited
`permission_denials: []` proves nothing about a step: in leg D a step's Bash call was refused by the
platform (step transcript `agent-a5caa4d535b10df94.jsonl`: `tool_result` `is_error: true`, "This command
requires approval") and `D-result.json` still has `permission_denials: []`; so does round 4's rerun A,
whose R3CTRL was refused the same way. Item 52 should not cite it.

**A4.** All 16 step payloads carry `effort: {"level": "low"}`; the session ran `--effort medium`. Two
statements are wrong. (1) "The main session's own Workflow events carry no `effort` key" (`RESULTS.md`,
item 53, the `extra-probes-5.json` evidence line): lines 2 and 3 carry `effort: {"level": "medium"}`, the
session's, as do the check's rerun lines 2 and 3 and leg D's lines 2 and 3 (`low`, its session). (2) "The
agent file's value": `worker.md` and the `agent()` call both said `low`, so the run cannot say which one
the payload reports. The check's rerun separates them (section 3): agent file `xhigh`, call `low`, session
`medium` → every step payload `low`. The payload carries the effort the script dispatched the step with,
and that wins over the agent file. This is the case the first run is in: builder files say `xhigh`
(`tests/lint/roster.toml`), the cheap tier dispatches `low`.

**B.** The recorded facts hold except one: exit 1, `subtype: error_max_budget_usd`, `is_error: true`,
`num_turns: 1`, $0.0578, no denials, record `completed` with the whole return value, journal with three
results. But `B-result.json` has **no `result` key** (not `result: ""`); it has `errors: ["Reached maximum
budget ($0.05)"]` and `terminal_reason: budget_exhausted`. And the leg did not reach its question. The main
transcript shows when the cap was crossed: budget `0.014` after the launch; the workflow's notification was
queued at 06:29:07.034; the main loop's second answer ("The workflow is running ...") was written at
06:29:07.128; budget `0.0578` at 06:29:07.140. The spend crossed 5 cents only with that answer, after the
workflow had ended, so nothing was "in the middle" to stop. Item 54's "only a larger workload would strand
a mid-flight step; untested" is honest, but the finding (PR5-1) and the plan change it proposes ("instead
of assuming a killed workflow") generalise from the one case seen. The check's run at 2 cents shows the
other case: the workflow is killed (section 3).

**C.** Leg A's folder, the kept round-4 folder `-private-tmp-claude-501--Users-...-scratchpad-pr4-project/
1cf1064b-4e2f-49c6-92e9-e93287927cfa/` (`q9`), and the check's three folders all have the stated shape:
`workflows/wf_*.json` with `script`, `scriptPath` (under the session's `workflows/scripts/`), `status`,
`result`, `agentCount`, `workflowProgress` (`workflow_phase` entries first, then `workflow_agent` entries
with `agentId`, `agentType`, `model` as a full id, `state`); `subagents/workflows/<wf>/agent-*.jsonl` with
`tool_use` entries, `agent-*.meta.json`, `journal.jsonl`. Byte equality for calls by name holds for `q9`
(saved script = `probes-round-4/plugin/workflows/q9.js`, `cmp`), `one`, `three` and the check's `one`
and `three`. Slips: (1) the rule "every character outside `A-Za-z0-9_-` becomes `-`" is shown for `/` and
`.` only; no probed path, and no folder under `~/.claude/projects/`, has an underscore, so whether `_`
survives is not shown. (2) `journal.jsonl` holds one `launched` line per run, then a `started` and a
`result` line per agent (LEG-C says one `launched` per agent). Not noted: a step transcript repeats some
`tool_use` entries in `api-request-blob` records (leg A: 8 ids, 10 entries; every id also has an
`assistant` record), so a reader takes `type: assistant` records or dedupes by id; and a killed record
adds `error` and has `result: null` (section 3). An exact route from a session to its folder needs no
naming rule: every hook payload's `transcript_path` is `<folder>.jsonl`.

**D.** The reconstruction is faithful: leg D's `r123.js` equals the round-4 check record's `script`
(`wf_0d0146ac-d6b.json`) with `chk4:` → `chk5:` and `PROJ` changed, nothing else. The hook log shows each
item directly. 36: `agent_type` `chk5:clerk`/`chk5:returner`, one `agent_id` per step, the run record's
eight `agentId`s; main-session events have neither key. 37: `PostToolUse` lines 9 and 10 hold the
rewritten `grounded: false`/`true`, and the record's result has them; R2C's `PreToolUse` appears five times
(lines 7, 19, 24, 25, 26), no `PostToolUse`, step `state: error`. 38: line 13 `sh .../probe.sh RUN3A`, line
15 `PostToolUse` `VSDD_CALLER=t42 sh ...` → `CALLER=t42 ARGS=RUN3A`, with only `Bash(echo *)` allowed;
R3CTRL refused (step transcript, above). 40: line 5 the empty-note result, then line 22 the corrected one,
line 23 its `PostToolUse`; the record's `note` is the 289-character wrapped message and `output` `ZEBRA-31`.
Under-reported: (1) item 37's failure is not the 2.1.331 one. Round 4: four refusals, `subagent completed
without calling StructuredOutput (after in-conversation nudge)`, about 190 s. Here: five attempts in about
10 s, `TelemetrySafeError: agent({schema}): StructuredOutput retry cap (5) exceeded — 5 failed calls with
no valid output — last StructuredOutput error: ... must NOT have additional properties`. The conclusion
holds (a closed object fails the step); item 37's text is version-specific and not updated. (2) R3B's
rewrite was not rebuilt, so item 38's `StructuredOutput` leg rests on R2A/R2B (allow plus `updatedInput`
on `StructuredOutput`), which is enough. (3) Leg D entered by the typed command under the isolation flags,
the sandbox flag and the ceiling variable; the round-4 rerun A entered by the skill `probecall` with none
of them. `COMMANDS.md` names the entry route only; `platform-proven-3.md` names neither.

## 3. Check reruns

**Rerun A** (`legA.sh`): the exact command of ORDER-6 8.2, with the calling session's `CLAUDE_*`,
`CLAUDECODE` and custom-header variables unset (only the two token limits from the shell profile remain):

```
cd <S>/project
CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0 C5_LOG=<S>/logs/A.jsonl \
claude -p "/c5:one <S>/plugin" --plugin-dir <S>/plugin \
  --permission-mode default --allowedTools "Workflow Agent Bash Read Write Edit Glob Grep" \
  --settings '{"sandbox":{"enabled":false}}' --setting-sources project,local --strict-mcp-config \
  --model sonnet --effort medium --max-budget-usd 1 --output-format json < /dev/null > <S>/logs/A-result.json
```

The plugin `c5` is a copy of the round-4 probe plugin (`hooks.json`, `run.sh` unchanged; `runner.py`
logs the whole payload and decides nothing), with `bin/probe-marker` (`BIN-CHECK-Q7`), `methods/probe.md`
(`METHOD-CHECK-Z4`), `agents/worker.md` (tools `Bash, Read, Write, Edit, Glob, Grep`, `model: sonnet`,
`effort: xhigh`) and `workflows/one.js` (one step, `agentType: 'c5:worker'`, `model: 'sonnet'`,
`effort: 'low'`). The project: `git init -b main`, one commit holding `README.md` and `notes.md` (`TODO-C`),
signing off, `git worktree add .worktrees/x -b story/x`, no settings file. Session
`fb028848-21c3-4b64-aaae-ad61c33ddbe6`:

- rc 0, `subtype: success`, `is_error: false`, `num_turns: 1`, `permission_denials: []`, $0.1038.
- Hook log: line 1 `UserPromptExpansion` (`command_name: c5:one`, `command_source: plugin`); line 2
  `PreToolUse Workflow {"name": "c5:one", "args": "<S>/plugin"}`, effort `medium`; lines 4-17 the step,
  `agent_type: c5:worker`, `agent_id: afbe9cef34f0bc21c`, effort `low` on all 14. Line 5 `PostToolUse Bash`
  `probe-marker` → `BIN-CHECK-Q7`; 7 `Read <S>/plugin/methods/probe.md` → `METHOD-CHECK-Z4`; 9 `Write
  .worktrees/x/check-write.md` `type: create`; 13 `Edit .worktrees/x/notes.md` `TODO-C` → `DONE-C`; 15 `Bash`
  `git -C <S>/project/.worktrees/x add -A && git -C ... commit -m check5` → `[story/x a61f0a6] check5`.
- Main transcript: one tool call, `Workflow`; no `Skill`. Skill listing: `c5:one` and built-ins; no
  `mcp__` tool. The person's `~/.claude/CLAUDE.md` is still injected, into the main session and into the
  step transcript: the isolation flags do not keep it out.
- Run record `wf_6aa63f05-61b.json`: `status: completed`, `agentCount: 1`, `script` = `one.js` = the file
  at `scriptPath`, byte for byte; agent `c5:worker`, `claude-sonnet-5-5`, `done`.
- Git: `a61f0a6 check5` on `story/x`, parent `14f2c4f init` (`main`, one commit); `check-write.md` and
  `notes.md` changed; worktree clean.

A1, A2, A3 pass in the form specified; A4 gives `low` with the agent file at `xhigh`.

**Rerun B** (`legB.sh`, the same command with `/c5:three` and `--max-budget-usd 0.05`; `three.js` is the
prober's with `c5:worker`). Session `aa3f5a35-12d0-4017-bb83-da71b1e1b029`: rc 1, `error_max_budget_usd`,
`is_error: true`, `num_turns: 1`, no `result` key, `errors: ["Reached maximum budget ($0.05)"]`, $0.0651,
standard error empty. Record `wf_3035e1c2-c2c`: `completed`, three agents `done`, the whole result. The
workflow ended at 06:44:36.981, the main loop's second answer at 06:44:38.199, the budget read 0.0651 at
06:44:38.209: the round-5 shape again, for the same reason.

**Rerun B at 2 cents** (`legBm.sh`, `--max-budget-usd 0.02`, so the cap falls inside the workflow).
Session `9e960d9f-36b4-4f79-b4fa-ccc2876a2626`:

- rc 1, `error_max_budget_usd`, `is_error: true`, `num_turns: 2`, no `result` key, `errors: ["Reached
  maximum budget ($0.02)"]`, $0.0544. Standard error: `Budget limit reached ($0.05 of $0.02); stopping
  background agents.`
- Record `wf_d0f5214c-a14.json`: `status: killed`, `result: null`, a new key `error: "Error: Workflow
  aborted\n    at ..."`, `agentCount: 1`; T1 `state: progress`, `lastToolName: StructuredOutput`, no
  `durationMs`. T2 and T3 never started. Journal: `launched`, `started` T1, no `result`.
- Hook log line 4: T1's `PreToolUse StructuredOutput {"n": 1, "t": "T1"}`, with no `PostToolUse`: a hook
  saw a result the run record never got.
- Timing: budget 0.015 after the launch; the main loop's second call started 06:44:46.310 and has no
  answer recorded; T1's model response 06:44:47.787; the record closed 06:44:47.809.

So a `--max-budget-usd` stop has two shapes. Crossed after the workflow ends: the record is `completed`
with the whole result. Crossed while a step runs: background agents are stopped, the record is `killed`,
the running step is left `progress` and its result is lost. The session result looks the same in both
(rc 1, `error_max_budget_usd`, no `result` key) and standard error tells them apart. The stop came after
the cap was passed, not at it ($0.054 on a $0.02 cap): what is in flight is charged. Seen once each, with
tiny steps; how far a large step overshoots is not measured.

## 4. The spec files against the evidence

`platform-proven-3.md`'s header names the version, the mode, the flags and leg D's two flag differences;
each item names its mode. Where the items say more than the evidence:

- **Header.** "Items 36 to 38, 40 and 48 all re-held ... including the closed-schema failure": it
  re-held as a failure, with a new message, count and time (section 2, D). Leg D's typed-command entry
  and R3B's missing rewrite are not mentioned.
- **Item 52.** Cites `permission_denials` empty as evidence; for a step it is not. Add: a refused step
  call is not listed in the session's `permission_denials` (leg D R3CTRL; round 4 rerun A). "The first
  run's tool list covers the story step's write scope" says more than one Read, Write, Edit and commit.
- **Item 53.** "The main session's own Workflow events carry no `effort` key" is contradicted: they carry
  the session's. "The agent file's value" was not tested; the check's rerun shows the dispatched effort
  wins over the agent file. `platform.md` row 26 repeats "the agent file's effort".
- **Item 54.** "Empty `result` text": there is no `result` key. Only the completed shape is described,
  and the killed one is called untested; the check's run at 2 cents shows it. `platform.md`'s headless
  bullet "A budget stop is not a dead run" holds only for the first shape.
- **Item 55.** The `_` in the naming rule is not shown. The rule is not needed: the folder is
  `transcript_path` without `.jsonl`, or `<claude home>/projects/*/<session id>/`.
- **`platform-proven-2.md` item 37** still gives the 2.1.331 failure text and time with no note that
  2.1.332 fails differently.
- **`extra-probes-5.json` PR5-1** says "an empty result text"; its `problem` says `STEP_EFFORT_PROVEN`
  "can ship true" and its `proposed_resolution` says it needs "no change": it ships false, so it changes.

## 5. What follows for the first run

- **A1, the isolation default.** No change to the default: `first-run.md` 3.5 keeps
  `--setting-sources project,local --strict-mcp-config` on; its "Rests on" cell becomes item 51 (round 5,
  2.1.332, default mode) in place of "the probe of ORDER-6 decides the default (D8)"; D8's "Whether a
  `--plugin-dir` plugin still loads under the two flags is unproven" becomes "shown (item 51)". D8 should
  also say what the flags do not keep out: the person's `~/.claude/CLAUDE.md` reaches the main session and
  every step. ORDER-6 section 7's "The two isolation flags, and `Skill` in the tool list, follow the probe
  answers" becomes: both flags on, no `Skill`. The `--user-settings` fallback is not taken.
- **A2, the entry route.** No change to the prompt. `first-run.md` 3.5's prompt row ("Item 31, seen in
  `auto` mode only: the probe of ORDER-6") cites item 50; section 4's sentence "If the probe of ORDER-6
  shows the typed command needs the `Skill` tool, `Skill` joins 3.5's list" is settled: it does not.
  `story-workflow.md` 3.1, "where unproven, probed before the first run", cites items 50 to 52.
- **A3, the allowed-tools list.** No change: `Workflow Agent Bash Read Write Edit Glob Grep`; 3.5's row
  cites item 52. But the checks that read denials must change, because a step's refusal is not in the
  session's `permission_denials`. `first-run.md` 3.6 `session_ended`: "no `permission_denials`" stays and
  covers the main session only; add "and no step call refused: no `tool_result` with `is_error: true` and
  `requires approval` in the step transcripts". 3.7's cause `denied` and ORDER-6 section 7's row `denied`
  ("`result.json`, `permission_denials`") read the step transcripts as well, naming the agent, the tool
  and its input from the matching `PreToolUse` payload (a `PreToolUse` with no `PostToolUse` for its
  `tool_use_id`).
- **A4, `STEP_EFFORT_PROVEN`.** Set it `True` in `E/probe.py` (ORDER-6 8.2 row A4 and the E2 line of
  section 8; `first-run.md` D11 and 3.6 `tier_applied` (c)). D11's "Whether a step's payload carries its own
  effort or the session's is unproven (items 26, 48)" becomes: proven (item 53, with the check's rerun): the
  payload carries the effort the step was dispatched with, over both the session and the agent file. Leg
  (c)'s rule ("that tier's `effort.level`", the override's under `cheap`) is right as written and needs no
  change; `--effort medium` keeps its reason. The PR5-1 resolution's "no change for STEP_EFFORT_PROVEN" is
  wrong: it changes from false to true.
- **B, the budget-stop diagnosis.** `first-run.md` 3.7's cause `budget_stop` and ORDER-6 section 7's
  row (`result.json`, `subtype` and cost; "probe leg B says what that leaves") become, in words: "The
  session reports `error_max_budget_usd` (exit 1, `is_error: true`, no `result` key, `errors` naming the
  cap). Read the run record: `completed` with the whole result when the cap was passed after the workflow
  ended; `killed`, with `error` 'Workflow aborted', `result: null` and the step that was running left in
  `progress`, when it was passed during a step (standard error: 'Budget limit reached ...; stopping
  background agents.'). Name that step; its result may be in the hook capture with no `PostToolUse`.
  Resume with `--dir`." The driver reads `result.get("result")`, never `result["result"]`. The spend can
  pass the cap (2.7 times a 2-cent cap here), so ORDER-6's "Cap for one session" and refusal 8's total are
  ceilings that one attempt's in-flight calls can exceed; 3.5's `--max-budget-usd` row ("No probe reached
  one; the probe of ORDER-6 does") cites item 54, corrected. `named_workflow_ran` already fails a `killed`
  last attempt, as it should.
- **C, the folder the checks read.** No change to the fields `first-run.md` 3.3 reads, or to finding the
  folder by `<claude home>/projects/*/<session_id>/` (no naming rule needed). Two readings to state:
  `workflowProgress` starts with `workflow_phase` entries (3.3 reads `workflow_agent` entries only); a
  `killed` record has `result: null` and an `error` key. `holdout_wall`'s reading of the step transcripts
  takes `type: assistant` records or dedupes `tool_use` ids, since `api-request-blob` records repeat some.
- **D.** No first-run change. The spec notes item 37's 2.1.332 failure (retry cap 5, about 10 s).

## 6. The gate

Legs A1, A2 and A3 pass as specified, in the primary form: the plugin loads under both isolation flags (no
repeat without them was needed) and the typed command works with no skill (no repeat with `Skill`). The
prober's session and the check's independent rerun agree. The gate is passed. Before the run, the plans
take the changes of section 5: `STEP_EFFORT_PROVEN = True`, the two-shape `budget_stop`, and denial
detection from the step transcripts.
