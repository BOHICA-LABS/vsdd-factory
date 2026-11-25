# Round-5 results, one block per leg

Every claim rests on the cited file in this folder (hook-log line, result JSON, run record, git output).
Modes and flags as COMMANDS.md states. Claude Code 2.1.332, macOS, 2026-11-25, permission mode `default`
in every session, `--setting-sources project,local --strict-mcp-config`, sandbox set off with `--settings`.

## A1 — the plugin loads under the two isolation flags: PASS

Session `4265e389-8597-4ead-b675-cfdb38acdbfa`, project with no settings file. The hook log `A-part1.jsonl`
opens with a `UserPromptExpansion` line and `PreToolUse`/`PostToolUse` for tool `Workflow`: a
`--plugin-dir` plugin's hooks are live under `--setting-sources project,local --strict-mcp-config`. The run
record `platform/wf_bb1971a5-1cd.json` has `workflowName: one`, `status: completed`. The step returned the
bin/ tool's marker: run record `result.bin_bare = "BIN-MARKER-R5"` — the bare name resolved, so the
plugin's `bin/` is on the step's PATH; `bin_abs` (explicit path) matches. The sandbox flag was honoured: the
step's compound `git -C .worktrees/x add -A && git -C .worktrees/x commit -m leg5` wrote
`.git/worktrees/x/index.lock` and committed (`PostToolUse` stdout `[story/x e2b67c6] leg5`), which item 41
shows is impossible with the sandbox on. No repeat without the flags was needed.

## A2 — the typed workflow command with no skill, default mode: PASS

Same session. `num_turns: 1`; `permission_denials` empty; the run record's `script` equals
`plugin/workflows/one.js` byte for byte (`platform/one-wf_bb1971a5-1cd.js` is the saved copy; compared in
Python, equal), `status: completed`. The prompt `/p5:one <S>/plugin` alone started the named workflow: the
model made a `Workflow` call with the name (PreToolUse line 2), no `Skill` call occurred, nothing was
denied, and the typed text reached the script as `args` (the script trims it and its built prompt —
`plugin/methods/probe.md` — appears in `promptPreview`). The model never reached for a skill, so the
contingency ("repeat with Skill allowed") did not arise.

## A3 — bare tool names admit the step's cross-boundary work: PASS

Same session; `permission_denials` is empty in `A-result.json`. From the `PostToolUse` lines in the hook
log: `Read /Users/jmagady/.vsdd-probes/round-5.QkGwwK/plugin/methods/probe.md` (outside the working
directory) returned `METHOD-MARKER-R5`; `Write .worktrees/x/leg5-write.md` created the file
(`WRITEMARKER-R5`); `Edit .worktrees/x/notes.md` changed `TODO-5` to `DONE-5`; the commit output names
`2 files changed`. Git confirms: `git -C .worktrees/x log` has `e2b67c6 leg5` on branch `story/x`, both
files hold their new text, `git status --porcelain` in the worktree is empty. Bare tool names
(`Bash Read Write Edit Glob Grep` and `Workflow Agent`) in `--allowedTools` admitted all of it, including a
compound Bash command.

## A4 — a step's payload carries the step's effort: the agent file's, not the session's

Every step payload in the hook log (16 records of `p5:worker`) carries `effort: {"level": "low"}`, while
the session was started with `--effort medium` and the agent file says `effort: low`. The main session's
own Workflow events carry no `effort` field at all. So the payload names the step's effective effort
(agent file wins over the session flag), and `STEP_EFFORT_PROVEN = True` is earned. Item 26 is settled.

## B — a --max-budget-usd stop: recorded

Session `654e64c3-4863-4d4f-af29-556e11e815f9`, workflow `three`, cap `0.05`, `B-result.json`:

- exit code 1 (B-rc.txt); `subtype: error_max_budget_usd`, `is_error: true`, `num_turns: 1`,
  `total_cost_usd: 0.0578`, `result: ""` (empty text); no denials.
- The run record `platform/wf_402584c1-f2f.json` reads `status: completed` and holds the workflow's whole
  return value: `{"a": {"n": 1, "t": "T1"}, "b": {"n": 2, "t": "T2"}, "c": {"n": 3, "t": "T3"}}`; all three
  agents `state: done`, full model id `claude-sonnet-5-5` each.
- Nothing was left mid-flight: the hook log shows three completed StructuredOutput round trips, the journal
  records all three returns, and no step was running when the session stopped. At this cap the stop fell
  after the workflow had completed — the main loop's budget accounting stopped the turn that would have
  delivered the result, not the workflow. A driver must read the run record (it may hold a usable result)
  and not treat `error_max_budget_usd` as a killed workflow. Filed as PR5-1.

## C — the session folder: described

See `leg-C/LEG-C.md`. Location rule
`~/.claude/projects/<project path with every char outside A-Za-z0-9_- replaced by ->/<session id>/`; the
folder shape (run record fields, saved script, step transcripts, journal, meta) is identical in the kept
round-4 folder and leg A's; for a call by name, `script` and the file at `scriptPath` equal the plugin's
file byte for byte in both records checked (round-4 `q9`, round-5 `one`).

## D — items 36 to 38 and 40 of the round-4 check, re-held on 2.1.332: PASS

Session `9e2b33bb-77e5-492a-a52b-def85effd91b` (workflow `r123`, 8 agents in parallel, default mode,
allowedTools `Workflow Bash(echo *)` only). All four re-hold:

- **Item 36**: every step payload carries `agent_type` exactly `chk5:clerk`/`chk5:returner` and one
  `agent_id` per step (`D-part1/part2.jsonl`); the run record's `workflowProgress` agentIds are the same
  ids (R1 `a05ec0ef83a75a20f` and seven more). Item 36's `agent_type` half is thereby also re-shown on the
  new version.
- **Item 37**: the run record's result holds R2A `findings: [{"text": "R2ATEXT", "grounded": false}]`
  (key undeclared, open schema) and R2B with `grounded: true` (declared) — the rewritten key reached the
  script. R2C (item object `additionalProperties: false`) errored:
  `agent({schema}): StructuredOutput retry cap (5) exceeded`, last platform refusal `Output does not match
  required schema: /findings/0: must NOT have additional properties`; the hook log shows the hook saw all
  five attempts. A closed schema makes the rewritten result unreturnable and fails the step.
- **Item 38**: R3A `PreToolUse Bash` saw `sh <S>/projd/probe.sh RUN3A`; the hook allowed and prefixed; the
  `PostToolUse` shows the rewritten command `VSDD_CALLER=t42 sh ...` and the awaited result is
  `CALLER=t42 ARGS=RUN3A` — with `sh` absent from allowedTools. R3CTRL, the same session, hook silent:
  `This command requires approval`. Both legs in one session, as the check demanded.
- **Item 40**: the hook logged R5B's first result (note empty) before any refusal; the agent's recovery is
  the run record's R5B `note` = the full wrapped message (`PreToolUse:StructuredOutput hook error: [sh
  "<hook path>"]: CHK5-GATE-9: ...`, 289 characters) and `output: ZEBRA-31`, a token only the message
  carried; the accepted second return appears as `PostToolUse`.

R3B returned `R3BMARK` (its rewrite was not rebuilt; see COMMANDS.md), so the StructuredOutput half of
item 38 rests here on R2A/R2B. Item 48's `effort.level` also re-observed (all step payloads `low`, session
`low`) — leg A is the discriminating run for effort.
