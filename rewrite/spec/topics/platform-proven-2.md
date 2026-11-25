---
kind: topic
id: platform-proven-2
sources: [BLUEPRINT.md#10]
---

# Platform facts proven by probe, round 4

Continuation of [[topic:platform-proven]] (items 1 to 35). Round 4 ran on 2026-11-24, and an independent
check reran its important legs the same day. What holds for every item below unless the item says otherwise:

- **Version.** Claude Code 2.1.331, git 2.50.1, macOS (Darwin 25.5.0), headless `claude -p`.
- **Install.** A throwaway plugin loaded with `--plugin-dir`. No marketplace-installed plugin was probed.
- **Mode.** Each item's first line names the permission mode it was seen in.
- **Evidence.** `rewrite/spike/probes-round-4/`: one folder of hook logs and result files per question, and
  `CHECK.md`, the check's report, which holds where `RESULTS.md` beside it differs. "Rerun" means a rerun
  by the check (`CHECK.md`, section 3). Its logs are kept in `check-reruns/logs/`, with the script that ran
  them, and `CHECK.md` quotes the lines that decide.

## Item 36 — a step's hook payload names the plugin agent

Proven. Seen: default mode. Evidence: `q1-q5-followup/s1.jsonl`; rerun A (R1), which logged the whole payload.

- A step dispatched with `agentType: '<plugin>:<agent>'` (`fs4:clerk`, `chk4:clerk`) carries `agent_type`
  exactly that string on its `Bash` and `StructuredOutput` calls, and one `agent_id` on every call of the
  step, the id its run record holds.
- An untyped step (`agent()` with no `agentType`) carries `agent_type: "workflow-subagent"`.
- Main-session events have no `agent_id` or `agent_type` key at all: a reader gives both a default.
- A step's `PreToolUse` payload holds `session_id` and `transcript_path` (the main session's), `cwd`,
  `prompt_id`, `permission_mode`, `agent_id`, `agent_type`, `effort` (item 48), `hook_event_name`,
  `tool_name`, `tool_input` and `tool_use_id`. `PostToolUse` adds `tool_response` and `duration_ms`.

Not shown: the prefix a marketplace-installed plugin gives ([[doc:platform]], open leg 36).

## Item 37 — a rewrite may add a key, unless the schema closes the object

Proven. Seen: default mode. Evidence: rerun A (R2A, R2B, R2C). Round 4's own run (`s1.jsonl`) covered the
undeclared leg only. On 2.1.332: leg D of round 5 ([[topic:platform-proven-3]]), run once and not rerun.

`permissionDecision: "allow"` with `updatedInput` adding `grounded: false` inside `findings[0]` of a step's
result:

- finding object open, key undeclared (R2A): the script receives the key;
- key declared as an optional boolean (R2B): the same;
- finding items closed with `additionalProperties: false`, key undeclared (R2C): the platform checks the
  rewritten input after the hook and refuses it, for a key the agent never sent: `Output does not match
  required schema: /findings/0: must NOT have additional properties ('grounded' is not allowed)`. No
  `PostToolUse` fires and the step fails. How it fails differs by version:
  - 2.1.331 (rerun A): four refusals, then after about 190 s `agent({schema}): subagent completed without
    calling StructuredOutput (after in-conversation nudge)`;
  - 2.1.332 (round 5, leg D): five attempts in about 10 s, each seen by the hook, then
    `agent({schema}): StructuredOutput retry cap (5) exceeded — 5 failed calls with no valid output — last
    StructuredOutput error:` and the schema message above. The step's `state` is `error`, and the
    workflow went on to `completed`.

The open and declared legs held unchanged on 2.1.332. So the grounding mark of [[rule:step-results]] needs no schema change while the finding object stays open,
and a schema that closes it must declare `grounded`.

## Item 38 — allow plus rewrite holds in the default mode

Proven. Seen: default mode, both legs in one session, `sh` not allowlisted. Evidence: rerun A (R3A, R3CTRL,
R3B). Round 4's own logs do not show it: the positive leg ran with `Bash(sh *)` allowlisted, and the refused
control was in another session.

- Bash in a step: `allow` plus a `VSDD_CALLER=t42 ` prefix ran the rewritten command (`CALLER=t42`).
- Control, hook silent, same session: `This command requires approval`, and no `PostToolUse`.
- `StructuredOutput` in a step: the rewritten field reached the script (`R3BMARK-HOOKREWRITE`).

This lifts item 27's caveat. Also seen (`q7-long`): a compound command is permission-checked part by part.

## Item 39 — a rewrite with no decision is applied, and grants nothing

Proven. Seen: default mode with `Bash(sh *)` allowlisted (rerun B); auto mode (`q4-auto/s3b.jsonl`).

A hook that returns `updatedInput` with no `permissionDecision` has its rewrite applied, and the rewritten
command then meets the ordinary permission check:

- R4ARG: an argument rewrite that still matched `Bash(sh *)` ran rewritten (`ARGS=RUN4ARG-REWRITTEN`);
- R4ENV: a `VSDD_CALLER=t42 ` prefix matched no allow rule: `This command requires approval`, no
  `PostToolUse`;
- R4ALLOWENV: the same prefix with `allow` ran (`CALLER=t42`);
- auto mode: the classifier admitted the prefixed command. The modes do not differ in how `updatedInput`
  is treated.

Two consequences. A rewrite grants nothing, so the caller-marker rewrite ([[topic:caller-identity]]) travels
with `allow`. And a rewrite always changes what runs, so no rule may return one as advice. PR3-3 stands:
`ask` with a rewrite never runs unattended.

## Item 40 — the hook sees a result before the platform's schema check, and its block arrives wrapped

Proven. Seen: default mode. Evidence: `q1-q5-followup/s1c.jsonl` with the step transcripts quoted in
`CHECK.md`; rerun A (R5B, R2C).

- Order: the `PreToolUse` hook on `StructuredOutput`, then the platform's schema check, which reads the
  input as the hook left it (item 37). A result missing a required key reached the hook; the agent then
  got the platform's own `Output does not match required schema: root: must have required property 'note'`.
- A result a hook has stored can therefore still be refused by the platform, and the agent's corrected
  return arrives with the same `agent_id` (item 36).
- Exit 2 with a message, on a valid result: the agent reads `PreToolUse:StructuredOutput hook error: [<the
  hook's command line>]: <message>`, a blank line, then `This hook comes from the <plugin>@<source>
  plugin.`, and returns again. In the rerun it returned a token only the message carried (`ZEBRA-31`).

So a block message is heard whole, but it is never the first characters the agent reads, and the hook's
absolute command line is visible to the agent.

## Item 41 — the Bash sandbox refuses every write under `.git/worktrees/`

Proven. Seen: default mode, the sandbox switched on with `--settings '{"sandbox":{"enabled":true}}'`, Bash
calls in a workflow step. Evidence: `q8-sandbox/s4b.jsonl`; reruns D and E2.

- Allowed: reads, `git branch`, `git update-ref`, `mkdir` directly under `.git/`, and a commit in the main
  checkout with signing off.
- Refused with `Operation not permitted`: `mkdir .git/worktrees/x`; `touch` inside an existing
  `.git/worktrees/<name>/`; `git add` and `git commit` in an existing linked worktree (`Unable to create
  '.git/worktrees/z/index.lock'`) and in the `.factory` mount; `git worktree add`; a write to `$HOME`.
- So nothing can be committed in a story worktree, in the integration worktree or in the `.factory` state
  mount, and no worktree can be added.
- A refused `git worktree add <path> -b <branch>` has already created the branch: the retry fails with
  `fatal: a branch named '<branch>' already exists` (rc 255).
- A commit with the person's global signing on fails too: gpg cannot reach its agent or write its home.

Not shown: any setting that grants the write (`excludedCommands: ["sh"]` did not, in one session); the
sandbox set in the project's settings file; a write under `.git/worktrees/` from a hook command, which the
platform documents as running outside the Bash sandbox, or from a detached job worker. The needs are
[[packaging:sandbox-needs]]. Recorded as PR4-1.

## Item 42 — a 600000 ms foreground call runs to its end

Proven. Seen: default mode. Evidence: `q7-long/s7.jsonl`. A step's foreground Bash call of 590 s with
`timeout: 600000` ran to its last marker (`interrupted: false`, no `timedOutAfterMs`), which closes item 6's
open leg. The session of this probe was stopped before the step returned its result: item 47.

## Item 43 — a call past its timeout is backgrounded, then killed at the step's final response

Proven on macOS. Seen: default mode, sandbox off and on. Evidence: reruns C and D (a worker that writes 60 s
after its spawn, Bash timeout 15000). Round 4's own legs (`q8-detach`, `q8-sandbox`) do not show survival: in
one the worker wrote before the kill, in the other the call was never killed.

- At its timeout the tool does not kill the call. It moves it to the background (`timedOutAfterMs`, a
  `backgroundTaskId`, `backgroundEndsWithFinalResponse: true`).
- The Bash result then carries a notice to the agent, after the output so far: "Command did not complete
  within its 15s timeout and was moved to the background (ID: bqle2o00k). Output is being written to:
  `<task output file>`. If it exits while you are still working you will be notified, but it is terminated
  when you give your final response and no notification can follow that — so do not end your turn to wait
  for it; if you need its result, wait for it before giving your final response. It is running now: do not
  start a second copy while it runs. If it is no longer needed, stop it with TaskStop. To check interim
  output, use Read on that file path."
- Evidence for the notice: in rerun C the step copied the Bash result into its own result, so the hook log
  of its `StructuredOutput` call holds it (`check-reruns/logs/C.jsonl`, line 5; `D.jsonl`, line 10, reads
  the same with the sandbox on).
- When the step agent gives its final response, the backgrounded call is killed: its task output reads
  `[killed]`.
- A double-forked worker in its own session (fork, `setsid`, fork) survived that kill and the session's
  exit, and wrote its file about 40 s after the final response, sandbox off and on.

So a job outlives its step only because it is detached, and the notice tells the agent to wait and not to
repeat the command, where a job command is made to be repeated ([[topic:long-commands]]). Not shown: Linux
([[doc:platform]], open leg 43). Recorded as PR4-2.

## Item 44 — Glob and Read see the `.factory` mount from the project root

Proven. Seen: default mode. Evidence: `q9-glob/s6b.jsonl`. An agent with Glob and Read only, in a step whose
working directory is the project root, found `.factory/specs/contract.md` (a gitignored worktree of another
branch) with pattern `.factory/specs/**/*.md`, and with pattern `**/*.md` and path `.factory/specs`, and read
it. Not shown: a working directory inside a story worktree, with an absolute path ([[doc:platform]], open
leg 44).

## Item 45 — git hooks in a linked worktree (plain git)

Proven. Seen: git 2.50.1 alone, no platform involved. Evidence: `q10-git-hooks/hooklog-repoC`; the rerun,
with hooks in `.git/hooks` and under a tracked `core.hooksPath`.

- An automatic `git merge --no-ff` in `.worktrees/_integration` runs `pre-merge-commit` and not
  `pre-commit`. A failing `pre-merge-commit` refuses the merge (rc 1).
- `git merge --no-commit` followed by `git commit` runs `pre-commit` and not `pre-merge-commit`. So does
  `git -C <worktree> commit` from the main checkout.
- In the linked worktree a hook runs with cwd = the worktree root, `GIT_DIR` and `GIT_INDEX_FILE` under
  `<repo>/.git/worktrees/_integration`, and `GIT_WORK_TREE` unset. In the main checkout `GIT_DIR` is unset,
  so a hook should ask `git rev-parse`.
- Tracked files are present. Untracked and generated files of the main checkout are not: a hook that needs
  one exits 1, and the commit is refused until the file is in the worktree.

## Item 46 — exit 2 on expansion stops a typed plugin command; SIGTERM ends a workflow

Partly. Seen: default mode. Evidence: `q11-expansion-block/`, `q11a-control/`.

- Exit 2 from a `UserPromptExpansion` hook on a typed plugin command ends the headless prompt with
  `num_turns: 0` and no cost; the result text names the event and quotes the hook's message. This extends
  item 2 to plugin commands in the default mode.
- That blocked result reports `subtype: success` and `is_error: false`. A driver detects a block by
  `num_turns == 0` and the result text, never by `is_error`.
- `SIGTERM` to a session whose step sat in a long Bash call: session, step agent and call were gone, and
  the result file was never written. The process samples behind the timing were not kept.

Not shown: a session end with reason `clear`. After `/clear` the process lives on, so the SIGTERM leg says
nothing about it ([[doc:platform]], open leg 46).

## Item 47 — headless print mode stops background workflows ten minutes after the last turn

Proven. Seen: default mode. Evidence: `q7-long/s7-stderr.txt` and the run record quoted in `CHECK.md`.
`claude -p` stops background tasks, a running workflow included, ten minutes after the last turn:
"Background tasks still running 10m after the last turn (workflow ...); stopping them. Set
CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0 to wait indefinitely." The run record ended `killed` at 604564 ms and
the step never returned its result. Every headless driver sets the variable ([[doc:platform]], "Headless
runs").

## Item 48 — hook payloads carry the effort level

Seen, default mode, rerun A. Every hook payload of a step carries `effort: {level}`. That makes item 26
observable. The rerun could not say whose effort it is: the session flag and the agent file both said
`low`. Round 5 could: a step's payload carries the effort the step was dispatched with, a main-session
payload the session's (item 53 of [[topic:platform-proven-3]]).

## Item 49 — the Workflow tool accepts an inline script

Proven. Seen: default mode. Evidence: two lost round-4 runs; rerun E (`E-inline-script`). A main session
told to start a named workflow can pass its own `script` instead, and the named plugin workflow never runs.
A typed skill that names the workflow does not prevent it: in rerun E the main session still passed `return
{}`. A driver therefore checks that the named workflow ran, by the run record's `scriptPath` and `script`,
or a hook refuses a `Workflow` call that carries `script`. Recorded as PR4-3.
