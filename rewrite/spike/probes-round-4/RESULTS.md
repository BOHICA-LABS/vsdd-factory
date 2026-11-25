# Round-4 results, one block per question

Every claim below rests on the cited hook-log line or result JSON in the same folder. Mode named per session.
Claude Code 2.1.331, macOS, 2026-11-24.

## Q1 — who a hook sees on a workflow step's result

Default mode, workflow `main` (s1.jsonl). A step dispatched with
`agent(prompt, {agentType: 'fs4:clerk', schema})` produced, on its `StructuredOutput` call, a `PreToolUse`
payload with `agent_type` exactly `fs4:clerk` (`<plugin>:<agent>`) and an `agent_id`; the same `agent_id`
appears on that step's `Bash` call (`echo Q1MARKER-77`) and its result call. A step dispatched with no
`agentType` carries `agent_type: "workflow-subagent"` (both in s1.jsonl and again in round 3's r1.jsonl).
Corroboration: s1c.jsonl (`fs4:q5returner`), s2b/s3b/s4b/s5b/s6b (`fs4:clerk`, `fs4:glober`). Consequence: the
verdict plan's keying on `vsdd-factory:<owner>` can work (PL4-22's uncertainty is resolved affirmatively by
parity of shape; the runner compares the literal string the design dispatches).

## Q2 — a rewrite that adds a key the schema does not declare

Default mode, workflow `main` (s1.jsonl, s1-result.json). The hook returned allow plus
`hookSpecificOutput.updatedInput` with `grounded` added inside `findings[0]`:

- Q2A, `grounded` undeclared in the step schema: the awaited result the workflow received is
  `findings: [{"text": "Q2ARESULT", "grounded": false}]` (WF-RESULT line of s1-result.json); the
  `PostToolUse` response is "Structured output provided successfully". Not dropped, not refused.
- Q2B, `grounded` declared optional: the added `grounded: true` reaches the awaited result the same way.

## Q3 — allow with a rewrite outside the auto mode

Default mode (s1.jsonl, s1c.jsonl):

- Bash, hook answers allow + `updatedInput` prefixing `VSDD_CALLER=ticket-42 `: the `PostToolUse` line shows
  the rewritten command (`VSDD_CALLER=ticket-42 sh .../probe.sh RUNME3A`) and stdout `CALLER=ticket-42`.
- `StructuredOutput`, hook answers allow + rewrite of `output`: the awaited result carries
  `Q3BMARKER-HOOKREWRITE` (s1-result.json).
- Control, no hook decision (s1c.jsonl, `sh probe.sh RUNME3ACTRL2` with the command absent from
  allowedTools): the agent's result is `This command requires approval` and no `PostToolUse Bash` line
  follows — the call would wait/be refused headless.
- Corollary seen on the way (s7.jsonl, first attempt): a compound command is split for permission; the part
  outside the allowlist (`python3 -c ...` after `echo ...`) drew the approval requirement even though the
  session's allowlist covered the other part.

## Q4 — a rewrite with no decision

`s2b.jsonl` (default) and `s3b.jsonl` (auto), both with `sh ...` allowlisted so permission is out of the
picture:

- Default: the `PreToolUse` hook logged `q4-rewrite-no-decision` and sent `updatedInput` alone; the call was
  still refused (`This command requires approval`, no `PostToolUse`). A rewrite without a decision does not
  admit the call in the default mode.
- Auto: the same hook output ran the rewritten command (`CALLER=ticket-42` in the awaited result). In auto
  mode the decisionless rewrite is honoured.

## Q5 — order of hook and schema check; refusing a result call

Default mode, workflow `followup` (s1c.jsonl):

- Ordering: the step submitted `{"label": "Q5C", "output": "NOCANDO"}` with the required `note` missing. The
  hook logged `q5c-hook-saw-missing-required-key` on exactly that invalid input — `PreToolUse` fires before
  the platform's own schema check. The agent then retried and its recorded refusal text is the platform's own
  ("Output does not match required schema: root"), so the platform check still answers the agent after the
  hook has spoken.
- Refusal: the hook answered exit 2 with a message on a schema-valid result missing `note`
  (`q5b-refuse-empty-note`); the agent's retry quotes `PreToolUse:StructuredOutput hook error:` and the
  message, and the awaited result is the corrected one (`output: "SECONDTRY"`). The agent answers again; the
  gate can refuse a step's result and be heard.

## Q6 — the sandbox and git

`s4b.jsonl` (default mode, `--settings '{"sandbox":{"enabled":true}}'`, step ran `sbx.sh`):

- Sandbox-on signal: `HOME_WRITE: /Users/jmagady/fs4-canary.txt: Operation not permitted`.
- `git worktree add .worktrees/x -b story/x` was REFUSED:
  `fatal: could not create directory of '.git/worktrees/x': Operation not permitted` (then, in cascade, the
  add and commit inside `.worktrees/x` failed because the directory was never created).
- `git update-ref refs/heads/probe-ref HEAD` SUCCEEDED (`REF: 5ee7939...` — a loose ref write under `.git/refs`)
  and `git worktree list` ran.
- So: with the Bash sandbox on, creating `.git/worktrees/` is refused while writing a loose ref works. The
  open probe of step-3 `RESOLUTIONS.md` (admission with the sandbox on) is answered negatively for
  worktree creation, and `sandbox-needs.md`'s table lacks the `.git/` write it must name.

## Q7 — a ten-minute foreground call

Default mode, workflow `q7` (q7-long/s7.jsonl): the step's `PreToolUse Bash` carries
`{"command": "python3 .../wait590.py", "timeout": 600000}`; the `PostToolUse` line answers
`{"stdout": "P7START\nP7LONGDONE", "interrupted": false}` — the 590 s call ran to its end under the explicit
600000 ms timeout inside a workflow step; no `timedOutAfterMs`. The first (superseded) attempt shows the
600000 value accepted in the input of the refused compound command too. Platform fact on the way: the ten-
minute value is accepted and honoured; the round-3 blocker was only the leading `sleep`.

## Q8 — a detached worker against the timeout

- Sandbox off (s5b.jsonl): the 15 s timeout did not kill the call — the `PostToolUse` response carries
  `"timedOutAfterMs": 15000` and a `backgroundTaskId`: the platform moved the call to background. The
  double-forked grandchild wrote `detach-off.out` (`DETACHED-WROTE`).
- Sandbox on (s4b.jsonl): the same backgrounding; the agent's polling loop then read `CALL-END [exited with
  code 0]` from the backgrounded call, and `detach-on.out` was written.
- Both legs: the detached worker survives and writes its file. The platform's timeout backgrounds a
  foreground call rather than killing it, which is gentler than the premise "the tool kills it".

## Q9 — finding state files under the .factory mount

Default mode, workflow `q9` (s6b.jsonl), agent `fs4:glober` (tools Glob, Read only). Both shapes found the
file: pattern `.factory/specs/**/*.md` from the project root and pattern `**/*.md` with path
`.factory/specs` both returned `.factory/specs/contract.md`, and Read returned `CONTRACT-MARKER-9`. The
gitignored worktree mount is visible to Glob and Read despite `.gitignore`. Reproduced a second time by the
Q11a control session (q11a-control/s11ac.jsonl result text).

## Q10 — git hooks in the integration worktree (plain git)

Repositories `repoA`/`repoB` (auto-merge), `repoC` (`core.hooksPath .githooks`) and `repoE` (`.git/hooks`),
kept in the scratchpad; hook output captured in `hooklog-*.txt`:

- An auto `git merge --no-ff --no-edit` merge commit runs NO `pre-commit` (repoA: merge succeeded, hook log
  empty; repoB the same). The hook a tool can expect on an automatic merge is `pre-merge-commit`.
- With the merge staged (`--no-commit`) and committed explicitly in `.worktrees/_integration`, `pre-commit`
  ran (repoC and repoE): `pwd` = the worktree root
  (`.../repoC/.worktrees/_integration`), `GIT_DIR` = `<repo>/.git/worktrees/_integration`, `GIT_WORK_TREE`
  unset, `git rev-parse --show-toplevel` = the worktree.
- A relative `core.hooksPath` (`.githooks`) resolved for the worktree commit because `.githooks/` is tracked
  and therefore checked out in the worktree.
- A hook needing a file that exists only in the main checkout: a TRACKED file (`scripts/version`) is present
  in the worktree; an UNTRACKED file (`data.gen`) is absent, the hook exited 1 and the commit was BLOCKED
  (`commit-rc=1`); after the file was placed in the worktree the same hook passed.

## Q11 — typed-command block and workflow lifetime

- Expansion block (s11a.jsonl, s11a-blocked.json): `UserPromptExpansion` exit 2 on the typed
  `/fs4:probecall BLOCKME11` ended the prompt with `num_turns: 0` and cost 0; the result text is
  `UserPromptExpansion operation blocked by hook: ... Q11A-BLOCK: ...`. The control session ran normally.
  Tested headless; both legs observed.
- Workflow lifetime (s11b.jsonl): `kill -TERM` to a session whose workflow step sat in a 120 s Bash call
  removed the session process, the step's Bash process and the step agent together within 4 s; no workflow
  state was left in the project (`.claude/` empty of run files), the result file was never written. A running
  workflow does not survive its session's death, headless, under SIGTERM.

## Side finding — the Workflow tool takes an inline script

s1 (q1-q5-followup, the superseded first run) and the first s7 attempt: a main session told to "call the
Workflow tool with name X" can pass its own `script` (the tool accepts it; `PostToolUse Workflow` shows the
inline text and the named file never runs). With the typed skill (`probecall`) the model resolves the named
workflow and the platform injects the file's text (`PostToolUse Workflow` shows `"script": "export const
meta = { name: 'main'...`). Driver skills are load-bearing for headless runs.
