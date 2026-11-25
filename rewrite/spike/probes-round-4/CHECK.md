# Check of the round-4 probe results

2026-11-24, Claude Code 2.1.331, git 2.50.1, macOS. Checked: `RESULTS.md`, `COMMANDS.md`, every log and
result file in this folder, `rewrite/findings/sources/extra-probes-4.json`, items 36 to 46 of
`rewrite/spec/topics/platform-proven-2.md`, the round-4 paragraphs of `topics/platform-proven.md` and
`spec/platform.md`. I also read the probe sessions' own run records and step transcripts, which the
platform keeps under `~/.claude/projects/<probe project>/<session>/` (`workflows/wf_*.json`,
`subagents/workflows/*/agent-*.jsonl`, `tasks/*.output`). Those files show what the agents were told,
which the hook log does not.

Reruns (section 3) used a fresh scratch copy under the session scratchpad (`<scratch>/chk4/`): a
throwaway plugin `chk4` whose hook runner logs the **whole** payload with a timestamp, a fresh git
project with a `.factory` worktree of an orphan branch (gitignored), headless
`claude -p "/chk4:probecall chk4:<wf>" --plugin-dir ... --model sonnet --effort low --max-budget-usd 2`,
generic prompts only. The scratchpad is temporary, so the lines that matter are quoted below.

## 1. Verdicts

| Item | Reported | Supported by the recorded evidence? | Rerun |
|---|---|---|---|
| Q1 agent_type / agent_id on a step | proven | **Yes** | Confirms; whole payload recorded |
| Q2 rewrite adds an undeclared key | proven | **No, as recorded.** Q2A holds. The "declared optional" leg (Q2B) was never run: `main.js` gives Q2B the same `FIND` schema as Q2A, with no `grounded` property | Both legs hold; a **closed** schema refuses the rewritten result |
| Q3 allow + rewrite, default mode | proven | **No, as recorded.** The positive leg (s1) ran with `Bash(sh *)` allowlisted, and the same-session control ran (`CALLER=unset`). The refusing control (s1c) ran in another session that had no positive leg | Confirms, with both legs in one session and nothing allowlisted |
| Q4 rewrite with no decision | partly | **No.** s2b cannot tell "rewrite ignored" apart from "rewrite applied, then refused by the permission check", and the reading given ("not honoured in default mode", "asymmetric by mode") is wrong | **Contradicts**: the rewrite is applied in the default mode; the rewritten command is then permission-checked as written |
| Q5 hook before schema check; exit 2 heard | proven | **Yes.** The ordering is in the hook log. The full message reached the agent: the step transcript shows it, though the folder's result file holds only its first 40 characters | Confirms; the agent acted on a token that only the message carried |
| Q6 sandbox and git | partly | **Yes, for what it states**, but it missed that the refused `worktree add` still created `story/x` (`.git/refs/heads/story/x`, 17:26:43) | Widens it: **every** write under `.git/worktrees/` is refused, so no commit is possible in any linked worktree |
| Q7 ten-minute foreground call | proven | **Yes** for the call. The note about the session misreads what happened: the workflow was **killed** by headless print mode's ten-minute ceiling | not rerun |
| Q8 detached worker against the timeout | proven | **No.** The tool never killed a call at its timeout. Off: the worker wrote at 17:27:13, before the backgrounded call was killed at 17:27:14 (`tasks/bld8kh3zz.output` = `[killed]`). On: the parent was never killed. Item 43's "the backgrounded parent call itself still completed" is contradicted by the sandbox-off leg | Worker survival now shown, sandbox off and on |
| Q9 Glob and Read on the `.factory` mount | proven | **Yes** (PostToolUse Glob `filenames`, Read `content`) | not rerun |
| Q10 git hooks in the integration worktree | proven | **Yes in its core** (`hooklog-repoC`). The folder is untidy, though: `repos-script.sh` is a paraphrase (wrong paths); no repoA log; repoB's log is not "empty" (one main-checkout entry); repoE's worktree entry lacks the tracked file; one repoD entry has `GIT_DIR=unset`; no rc is recorded. The stated consequence for delivery is wrong (section 4) | Confirms, and adds `pre-merge-commit` |
| Q11 expansion block; workflow lifetime | proven | **Half.** Q11a: yes (`num_turns: 0`, cost 0, message quoted). Q11b tested `kill -TERM`, not a session end with reason `clear`, and the process-table samples it cites are not recorded | not rerun; the `clear` leg is still open |

Checked 11, supported 6 (Q1, Q5, Q6, Q7, Q9, Q10). Q2, Q3, Q4, Q8 and Q11 are not supported as recorded.
Reruns confirm the conclusions of Q2 and Q3 and Q8's worker survival. They overturn Q4's reading.

## 2. Per item

**Q1.** s1.jsonl lines 17/25/29/31 show `agent_type: fs4:clerk` and `agent_id: ace09d2d8d20ce481` on the
step's Bash and StructuredOutput calls. The run record `wf_6a3d678f-71c.json` gives the same `agentId` and
`agentType`. Untyped `agent()` steps give `workflow-subagent` (s1 lines 2-13, the inline-script run).

The question asked for the whole payload, and the original runner logged selected fields only. The rerun
records it. A step's PreToolUse carries `session_id, transcript_path` (the main session's), `cwd,
prompt_id, permission_mode, agent_id, agent_type, effort {level}, hook_event_name, tool_name,
tool_input, tool_use_id`. PostToolUse adds `tool_response, duration_ms`. Main-session events have no
`agent_id`/`agent_type` keys at all.

Residual: only an `--plugin-dir` plugin (`fs4@inline`) was probed, not a marketplace install.

**Q2.** Q2A: hook log lines 19/20 and the journal show `findings: [{text, grounded: false}]` reaching the
script. Q2B: the script that ran (`workflows/scripts/main-wf_6a3d678f-71c.js`, identical to `main.js`)
passes `schema: FIND` to Q2B, and FIND's items declare only `text`. The claim "grounded:true when declared
optional (Q2B)" in item 37 and `extra-probes-4.json` is therefore not what was tested.

**Q3.** s1 allowlisted `Bash(sh *)`, and its own control `RUNME3ACTRL` ran (`CALLER=unset`, s1 line 28),
so s1 shows that the rewrite runs but not that the hook's `allow` admitted anything. s1c's control was
refused (`This command requires approval`, transcript confirms), but s1c had no positive leg. Only the
rerun puts both cases in one configuration.

**Q4.** s2b allowlisted `Bash(sh *)` and the hook prefixed `VSDD_CALLER=ticket-42 `. If the platform had
ignored the rewrite, the original `sh ...` matched the allowlist and would have run. It did not, so the
platform checked something else. The rerun shows it checks the rewritten command (section 3).

**Q5.** s1c line 5: the hook saw `{"label":"Q5C","output":"NOCANDO"}` with `note` missing. The step
transcript then shows the platform's own refusal `Output does not match required schema: root: must have
required property 'note'`.

Q5B's transcript shows the exact text the agent got. It is `PreToolUse:StructuredOutput hook error: [sh
"<plugin>/hooks/run.sh"]: FS4-HOOK-GATE-9: ... ` followed by a blank line and `This hook comes from the
fs4@inline plugin.` The cited result file holds only the first 40 characters. The message is wrapped:
the hook's command line comes before it and the plugin line after it.

**Q6.** s4b output as stated. The worktree add created branch `story/x` before it failed. The rerun's
retry shows what that costs: `fatal: a branch named 'story/y3' already exists` (rc 255).

**Q7.** s7 line 14: `stdout "P7START\nP7LONGDONE"`, `interrupted: false`, input `timeout: 600000`.
`s7-stderr.txt` reads: "Background tasks still running 10m after the last turn (workflow ...); stopping
them. Set CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0 to wait indefinitely." The run record
`wf_85928115-fea.json` has `status: killed`, `durationMs: 604564`. The step transcript ends in `[Request
interrupted by user]` nine seconds after the Bash result. The step never returned its result.

**Q8.** Both original legs carry `backgroundEndsWithFinalResponse: true`. In the sandbox-off leg the
agent returned at once, and the backgrounded call was killed in that second (`[killed]`), one second
after the worker had already written. In the sandbox-on leg the agent polled the background output until
`CALL-END [exited with code 0]`. Neither leg shows a worker outliving the kill of its starting call.

**Q9.** s6b lines 3/5/7: both Glob shapes return `.factory/specs/contract.md`; Read returns
`CONTRACT-MARKER-9`. Agent `fs4:glober`, tools Glob and Read only, default mode, in a workflow step.

**Q10.** `hooklog-repoC` shows the core: pre-commit in `.worktrees/_integration` with cwd = the worktree
and `GIT_DIR=<repo>/.git/worktrees/_integration`. Untracked `data.gen` is missing, so the hook fails; once
the file is placed there, the hook passes. The rerun (section 3) confirms this cleanly.

**Q11.** Q11a as stated. The blocked result has `subtype: success`, `is_error: false`, so a driver must
test `num_turns`/`result`, not `is_error`. Q11b: SIGTERM to the process is a different event from `/clear`
(after `/clear` the process lives on). The design's open question is about `clear` (step-3 RESOLUTIONS,
P3OPS-20), and it is still open. Headless `-p` cannot type `/clear` mid-run, so it needs an interactive
(tmux) probe.

## 3. Rerun evidence (`<scratch>/chk4/logs/`)

- **A** (default mode; allowedTools `Workflow`, `Bash(echo *)` only; workflow `r123`):
  - R1 `chk4:clerk`, `agent_id af3d5b8e29dda7af7` on the Bash call and the StructuredOutput call.
  - R2A (open schema) and R2B (`grounded: boolean` declared): the script gets
    `findings:[{text, grounded:false}]`.
  - **R2C (finding items `additionalProperties:false`)**: hook allow + rewrite. No PostToolUse. The agent got
    `Output does not match required schema: /findings/0: must NOT have additional properties ('grounded' is
    not allowed)` four times for a key it never sent. The step failed with `agent({schema}): subagent
    completed without calling StructuredOutput (after in-conversation nudge)`, after about 190 s. So the
    platform's check runs after the hook, on the rewritten input.
  - R3A (`sh` not allowlisted; hook allow + `VSDD_CALLER=t42 ` prefix): PostToolUse
    `VSDD_CALLER=t42 sh <proj>/probe.sh RUN3A` → `CALLER=t42 ARGS=RUN3A`.
  - R3CTRL (same session, hook silent): `This command requires approval`, no PostToolUse.
  - R3B: `R3BMARK-HOOKREWRITE`.
  - R5B: after the exit-2 refusal the agent returned `output: ZEBRA-31`, which only the message asked
    for, and `note` = the full wrapped message.
- **B** (default mode; `Bash(sh *)` allowlisted; workflow `r4`):
  - R4ARG (decisionless rewrite of an argument): PostToolUse `sh <proj>/probe.sh RUN4ARG-REWRITTEN` →
    `ARGS=RUN4ARG-REWRITTEN`. The rewrite is honoured with no decision.
  - R4ENV (decisionless `VSDD_CALLER=t42 ` prefix): `This command requires approval`, no PostToolUse.
  - R4ALLOWENV (allow + prefix): `CALLER=t42`.
- **C/D** (Q8; `spawncall.py out 60 45`, Bash timeout 15000, agent told to return at once; D with
  `--settings '{"sandbox":{"enabled":true}}'`):

  | Event | Off | On |
  |---|---|---|
  | spawn | 4442.0 | 4445.6 |
  | moved to background (`timedOutAfterMs: 15000`, `backgroundEndsWithFinalResponse: true`) | 4457 | 4460 |
  | final response | 4464.0 | 4464.2 |
  | task output reads `[killed]` | 4464 | 4464 |
  | session exit | 4468 | 4475 |
  | worker writes `DETACHED-WROTE` | 4502.1 | 4505.7 |

  The worker survives both the kill at the step's end and the session's exit, sandbox off and on.
- **D/E2** (sandbox on; `sbx2.sh`, `sbx3.sh`; worktree `.worktrees/z` made beforehand without the
  sandbox):
  - Allowed: `git branch story/y`, `git update-ref`, `mkdir .git/probe-dir`, a commit in the main checkout
    with signing off.
  - Refused with `Operation not permitted`:
    - `mkdir .git/worktrees/probe-dir`;
    - `touch .git/worktrees/-factory/probe-file`;
    - `git add` and `git commit` in `.worktrees/z` (`Unable to create '.git/worktrees/z/index.lock'`);
    - the same in `.factory` (`.git/worktrees/-factory/index.lock`);
    - `git worktree add`.
  - A commit with the person's global signing on failed: gpg could not reach its agent or write its home
    directory.
  - Session F also listed `"excludedCommands":["sh"]`. That did not take the call out of the sandbox:
    the home write was still refused. Not pursued.
  - Session E's first try never ran: through the typed skill, the main session still passed its own
    inline `script` (`return {}`). It is kept as `E-inline-script.jsonl`.
- **Q10** (`<scratch>/chk4/q10/hooklog.txt`; `.git/hooks` and `core.hooksPath .githooks`, both
  pre-commit and pre-merge-commit):
  - An automatic `git merge --no-ff --no-edit` in `.worktrees/_integration` runs **pre-merge-commit, not
    pre-commit**. A pre-merge-commit hook that needs the untracked `data.gen` fails and the merge is
    refused (rc 1).
  - `--no-commit` followed by `git commit` runs pre-commit: rc 1 with `data.gen` absent, rc 0 once it is
    placed.
  - `git -C .worktrees/_integration commit` from the main checkout also runs pre-commit, with cwd = the
    worktree.
  - In every linked-worktree run: `GIT_DIR` and `GIT_INDEX_FILE` are set to `.git/worktrees/_integration`
    and `GIT_WORK_TREE` is unset. The tracked `scripts/version` is present.
  - Main-checkout hooks (original repoE line 1) have `GIT_DIR` unset, so a hook should use
    `git rev-parse`.

## 4. What the spec files say beyond the evidence

- **Item 37**:
  - "whether the step schema declares it (optional) or not" was not tested by round 4; the rerun shows it.
  - Add: a schema that closes the finding object (`additionalProperties: false`) makes the rewritten
    result fail the platform's check, and the step fails.
- **Item 38**: true, but its evidence should be the same-session pair (rerun A: R3A, R3CTRL). s1 plus s1c
  do not show it.
- **Item 39 is wrong**:
  - Default mode: a decisionless `updatedInput` *is* applied, and the rewritten command then goes through
    the ordinary permission check. An argument rewrite that still matched `Bash(sh *)` ran rewritten; an
    env-prefix rewrite matched no rule and needed approval.
  - "A rewrite alone never admits a call" should read "a rewrite grants nothing; the rewritten command must
    pass the permission check on its own".
  - Auto mode: the classifier admitted the rewritten command. That is not an asymmetry in how
    `updatedInput` is treated.
- **Item 40**: true. It should give the wrapper the agent sees (hook command line before the message,
  plugin line after) and cite the transcript or rerun, not `s1c-result.json`.
- **Item 41**: understated.
  - All of `.git/worktrees/` is unwritable with the sandbox on. That covers commits in story worktrees, in
    the integration worktree and in the `.factory` state mount.
  - A refused `worktree add` leaves its branch behind.
  - Signing fails under the sandbox.
- **Item 42**: the note "the headless session may return while the workflow runs" should say that
  headless print mode **kills** background workflows ten minutes after the last turn unless
  `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0`; the q7 run was killed.
- **Item 43**:
  - Should read: a call past its timeout is backgrounded, then **killed when the step agent gives its
    final response**.
  - "The backgrounded parent call itself still completed" holds only when the agent waits.
  - Worker survival rests on the rerun, not on round 4. Linux is untested.
- **Item 45**: add pre-merge-commit on the automatic merge path, and that a failing pre-merge-commit hook
  blocks the merge the same way.
- **Item 46**: the second bullet is SIGTERM, not `clear`. Say that the `clear` leg is open, and that a
  blocked prompt reports `is_error: false`.
- **Side finding** ("through a typed skill the platform resolves the file"): not reliable. In rerun E the
  skill route still produced an inline script.
- **`platform.md`**:
  - Item 27's row still says "auto mode only" with no pointer to item 38.
  - The index table has no rows for items 36 to 46.
  - Item 26 "cannot be observed" is no longer true: every payload carries `effort.level`. It was
    confounded here (the session flag and the agent file were both `low`), but a cheap probe can now
    settle it.
- **`extra-probes-4.json`**:
  - PR4-1 understates its scope (Item 41 above).
  - PR4-2's "a runaway call is not reaped by the timeout and keeps consuming while backgrounded" is
    wrong: it is reaped at the step's end.
  - PR4-3's resolution (always enter through a typed skill) is not sufficient.

## 5. Consequences for the build

| Plan / unit | What must change |
|---|---|
| step-4 `ORDER.md` §7 P1, `verdicts.md` (PL4-22), `rules.md` §3.4 promise | Answered: a typed step's payload carries `agent_type` exactly `<plugin>:<agent>`, and an untyped one `workflow-subagent`. Mark P1 done. Keep G3's loud `skipped` listing until an installed (marketplace) copy is checked once, since only `--plugin-dir` was probed. Main-session payloads lack the keys, so read them with a default. |
| step-4 `ORDER.md` P2, `rules.md` §3.4 step 6 / D13, C1 schemas | Answered: the `grounded: false` rewrite reaches the script under the shipped open `finding.schema.json`, so no schema change is needed (close the step-4 RESULT.json open item). Add a rule-side test or lint that `finding.schema.json` and every result schema embedding findings (`review-data`, `delivery-data`, any prelude-inlined copy) never set `additionalProperties: false` on a finding, or else declare `grounded`. Otherwise the rule's rewrite makes every ungrounded return unreturnable and the step fails after the platform's retries. |
| step-4 `ORDER.md` P3; step-3 `RESOLUTIONS.md` (records' VSDD_CALLER probe, hook-runner's allow+rewrite probe); `hook-runner.md` HD7 | Answered yes, in the default mode inside a workflow step, for Bash and StructuredOutput. HD7 stands as written. |
| step-3 `RESOLUTIONS.md` hook-runner "updatedInput with no decision"; `hook-runner.md` HD7 and `test_decision_pairing` | It *is* honoured, but the pairing cannot be dropped: the rewritten `VSDD_CALLER=... vsdd ...` matches no allow rule, so without `allow` it waits for approval (default mode). Keep HD7. Add to HD7 that a rewrite without a decision changes what runs (no rule may return one as advisory), and a test case for it. |
| step-3 `RESOLUTIONS.md` records probe "hook before the schema check"; `records.md` D4 / `schema_invalid` | Answered: the hook sees schema-invalid results first, and the platform checks the (possibly rewritten) input after the hook. The store must keep its own schema check against the step's schema. D4's same-`agent_id` replacement is needed, not a flat `step_exists`: the agent's corrected retry arrives with the same `agent_id` (Q1), and a stored result can still be refused by the platform afterwards. |
| H7 rules (block texts), step-5 method prompts that quote gate messages | The agent sees `PreToolUse:StructuredOutput hook error: [<hook command line>]: <message>` plus `This hook comes from the <plugin>@<source> plugin.` Write block messages to stand inside that wrapper; never rely on them being the first characters. The hook's absolute command line is visible to agents. |
| `spec/packaging/sandbox-needs.md` and T1 doctor; PR4-1 | Add a row "write access to `.git/worktrees/` (every linked worktree's index, HEAD and logs)" and, where commits are signed, "the gpg home and agent socket". The doctor's test must be a commit inside an existing linked worktree, not only `worktree add`. Until a working allowance is shown, the deployment default for factory runs is sandbox off (a person's decision). `excludedCommands` was not shown to help. |
| step-3 `survey.md` (admission, `admit`) | With the sandbox on, `git worktree add -b` fails after creating the branch. `admit` must detect a branch it created whose worktree is missing (delete it or reuse it with `worktree add <path> <branch>`), and name the sandbox as the cause on `Operation not permitted`. |
| step-3 `records.md` (`commits.flush`, D11) | A flush run from a sandboxed Bash call cannot commit in `.factory` (`index.lock` refused) and so ends `failed`. The plan should say so, give `failed` a cause that names the sandbox, and note that hooks (not sandboxed) still flush. |
| step-4 `delivery.md` D6, D7, D10; step-5 `agents.md` (implementer, test-writer, `fix-findings` step 5) | Sandbox on: every commit, merge and merge-in in `.worktrees/<story>` or `.worktrees/_integration` is refused. D7's `refused` text should name the sandbox when git reports `Operation not permitted` under `.git/worktrees`. Q10 (the probe's "no finding" is wrong for this): D6's `merge` strategy (`git merge --no-ff -F`) runs pre-merge-commit and commit-msg, **not** pre-commit; `squash` runs pre-commit. Either state per strategy which hooks run, or make both use `--no-commit` + `git commit -F` so the project's pre-commit runs uniformly. D7 should add that hooks there run with cwd = the worktree and `GIT_DIR=.git/worktrees/_integration`, and see tracked files only: a hook that needs untracked or generated files (`.env`, `node_modules`, virtualenvs, generated sources) fails and the story is `refused`. The same holds for D10's clean merge-in commit in the story worktree. |
| step-3 `jobs.md` D10, `RESOLUTIONS.md` (G1 ten-minute probe) | `MAX_WAIT` may go to 540: a 590 s foreground call with timeout 600000 ran to its end in a step. |
| step-3 `jobs.md` D1/D2, `RESOLUTIONS.md` G1 survival probe; `spec/topics/long-commands.md`; PR4-2 | Answered for macOS: the double-forked, `setsid` worker survives the kill of its starting call, sandbox off and on (rerun C/D); Linux is still open. Correct the premise: past its Bash timeout a call is backgrounded and then **killed at the step's final response**, so detachment (D1, D2) is necessary, not just insurance. PR4-2's resolution changes: the platform does reap at the step's end; the job record covers what outlives the step. Step prompts for job commands should say: if the call is moved to the background, run the same command again (it re-attaches) rather than waiting on the background output, as the s4b agent did. |
| Every headless driver: E2's run (step-4 `ORDER.md` §7–8 inputs), the story-ready fixture and W2 (step 6), the scheduler recipe (platform item 25), probe harnesses | `claude -p` kills background workflows ten minutes after the last turn unless `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0` (or a larger value) is set. Every driver must set it, or a factory run dies at ten minutes with no result. Nothing in spec or plans mentions it. |
| PR4-3 / `spec/topics/spikes.md` / the same drivers | A typed skill does not guarantee the named workflow runs (rerun E passed an inline `return {}`). Add a check: a PreToolUse rule that refuses a `Workflow` call carrying `script` in driver sessions, or the driver verifies the run record's `scriptPath` and `script` against the plugin file. |
| step-5 `RESOLUTIONS.md` PL5-2 / N12 | N12's Glob probe is answered for a step whose cwd is the project root. A reviewer whose cwd is a story worktree, using an absolute `<project dir>/.factory/specs` path, is not yet probed; it is a cheap B8 follow-up if N11 does not land first. |
| step-3 `RESOLUTIONS.md` (records "workflow survives /clear"), `ORDER.md` §7 (P3OPS-20 `clear` half), `hook-runner-b.md` (lock_release on every reason) | Still open: round 4 tested SIGTERM, not `clear`. It needs an interactive (tmux) probe. Until then hook-runner-b's release-on-every-reason remains the unproven choice. |
| Typed-command gates (item 2, K1 `decide`) and any headless driver that reads the result | A blocked prompt reports `subtype: success`, `is_error: false`, `num_turns: 0`. Detect a block by `num_turns == 0` plus the result text. |

## 6. Housekeeping for the orchestrator

- `rewrite/s11b.jsonl` is a stray copy of `q11-expansion-block/s11b.jsonl` left at the `rewrite/` root
  (identical bytes).
- Missing from this folder: `plugin/skills/probecall/SKILL.md` and `plugin/workflows/q11.js` (both ran),
  Q11b's process-table samples, and Q10's actual script. `repos-script.sh` uses `/tmp/pr4-q10/...` paths
  that do not match the logs.
- COMMANDS.md says the Q10 repositories are in the scratchpad. They are in `/private/tmp/pr4-q10.YhrECl/`
  (still present today, not durable). repoA has no hook log there either.
