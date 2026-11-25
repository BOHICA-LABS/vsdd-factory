---
kind: topic
id: platform-proven
sources: [BLUEPRINT.md#10]
---

# Platform facts proven by probe

Settled items of [[doc:platform]], with what the probe showed and the evidence it rests on. Rounds 1 and 2 ran
2026-11-23, round 3 on 2026-11-24, all against Claude Code 2.1.331 on macOS (Darwin 25.5.0), headless
(`claude -p ... --plugin-dir ... --output-format json`), with the throwaway plugin now kept in
`rewrite/spike/probes-round-3/` (extended from `rewrite/spike/plugin/`). Where the evidence lives: the round-2
results rest on raw outputs that survive only in the session scratchpad, not in the repository; the round-3
results rest on the result JSON and hook logs in `rewrite/spike/probes-round-3/logs/`, and — for items 6, 13, 31
and 35 and the project-CLAUDE.md observation — on session transcripts kept only under `~/.claude/projects`,
whose decisive excerpts are quoted in `rewrite/spike/probes-round-3/CHECK.md`. Every round-3 probe session ran
in the `auto` permission mode; the default-mode results below come from the check's reruns.

Method notes that travel with every result: a `PreToolUse` hook response carries `hookSpecificOutput` with
`permissionDecision` and `updatedInput`; the nested `updatedInput` slot alone is honoured (probe R11). Hook
payloads carry `session_id` everywhere, including inside workflow-step subagents (R1). Workflow scripts can use
`args` and the globals `phase()`, `agent()`, `parallel()`.

## Item 1 — step result in the hook, block and retry

Proven, round 2. The hook sees the whole `StructuredOutput` call as `tool_input`; a block makes the agent correct
the result and return again. The `step record` fallback is removed from the design.

## Item 2 — typed decision command

Proven, round 2, headless. `UserPromptExpansion` fires with `command_name` and `command_args`; a block ends the
prompt with zero model turns, so the text never reaches a model.

## Item 3 — refused stop

Partly, round 2. A refused `SubagentStop` keeps a step running when it returns text; not when it has already
returned a schema result. The design blocks the result call (item 1) and refuses the stop only for agents
dispatched in a session.

## Item 4 — a skill starts a workflow

Proven, round 2, headless, for a skill a person invokes. Left to its own judgment the model never started a
workflow; it dispatched the spike agent. [[topic:spikes]] is written to that.

## Item 5 — plugin bin on PATH

Proven, round 2, in the main session, a dispatched agent and a workflow step, headless. A person typing
`! vsdd` is untested; the shim covers the person.

## Item 6 — long Bash calls and detached jobs

Partly in rounds 2 and 3. Round 2: a 150-second call with an explicit timeout ran inside a step and a detached job outlived the
call that started it. Round 3: the step's `sleep 240 && echo LONGDONE` call with timeout 600000 was refused by
the platform's sleep guard ("Blocked: sleep 240 ... use run_in_background: true"), so the 600000 ms value was
never tested; the agent re-ran the job detached and polled it with an until-loop at timeout 300000, which ran
236 seconds (step transcript, quoted in CHECK.md). What the round shows: a foreground call with timeout 300000
runs about four minutes inside a step, and the Bash tool refuses commands that begin with `sleep`. Round 4
completes the item in [[topic:platform-proven-2]]: the 600000 ms call is item 42, and what the tool does to
a call past its timeout is item 43.

## Item 7 — hook permission approval

Proven, round 2, in the default permission mode, in the main session, a dispatched agent and a workflow step:
the approved command ran and the control was refused. In the `auto` mode the control was allowed anyway. Wave-scale
prompt counts stay with E3.

## Item 8 — tool lists

Proven, round 2. An agent whose file lists the Agent tool still has none as a workflow step; an agent limited to
Bash still returns a schema result.

## Item 9 — hooks fire for MCP tools and WebFetch

Proven, round 3 (R5, `logs/r5.jsonl`, R5b), for a catch-all hook: `PreToolUse` fired for `mcp__tiny__ping`
called from the main session, a dispatched agent and a workflow step, and for a `WebFetch` call. Named matchers
are answered by the check's rerun C9 (default mode, main session only): a regex matcher (`mcp__tiny__.*`) and a
design-style alternation (`Bash|Read|mcp__(plugin_fspike_)?tiny__ping|WebFetch`) fired for the MCP tool and for
WebFetch; controls that do not name them (`mcp__nosuchserver__.*`, `Bash`) stayed silent. Untested: the
plugin-bundled server spelling, and named matchers in the dispatch and step contexts. Platform facts found on
the way: WebFetch refuses undotted hostnames outright ("WebFetch cannot fetch localhost or other hostnames
without a dot. To reach a local server, use Bash with curl instead."), and it upgrades an `http` address to
`https` — a plain-http loopback fixture is unreachable through WebFetch (C9: `http://127.0.0.1:8139` drew a TLS
ClientHello on the plain-http server and returned HTTP 502). Recorded as PR3-4.

## Item 10 — MCP tool lists

Proven, round 3 (R5, R5b, `logs/r5-result.json`). An agent whose explicit tool list names `mcp__tiny__ping` sees
and can call that tool from a dispatch and a workflow step; the project-scoped server came from `--mcp-config`.
The main-session call succeeded in a session with no tool list, not an agent with the tool named. A second,
nonexistent spelling (`mcp__nosuchserver__ping`) in the same list was dropped silently: the agent started and
worked with no error anywhere. Caveat: the "exactly one mcp tool visible" count is the agent's own account of
its tool list. The forked-skill leg was not exercised this round.

## Item 11 — agent frontmatter

Round 2: `isolation: worktree` honoured for a dispatch, ignored for a workflow step (steps pass isolation
themselves); a forked skill can name a plugin agent. `omitClaudeMd` removes the injected CLAUDE.md, shown for
the user-level file in the round-2 transcripts. Caveat: that probe project lived under `/private/tmp`, where no
project CLAUDE.md is ever loaded, so the project-file leg — the one the holdout-evaluator wall relies on — was
not observable; re-test it with the project under `$HOME`.

## Item 13 — missing hook launcher

Settled, negative, round 3 (R6). With every hook command pointing at a missing file (`sh` exits 127), a headless
session ran normally: the model answered in one turn and nothing blocked. The result files do not show the hooks
ran; the session transcript does, recording each failure as a non-blocking hook error with exit code 127 on
`SessionStart` and `UserPromptSubmit` (excerpt in CHECK.md) — the failures are visible, and the platform's
default for a missing launcher is fail-open. Untested in this probe: `UserPromptExpansion` (the prompt was plain
text) and `PreToolUse` (no tool was called). The self-guarding command of [[topic:hook-wiring]] is therefore
load-bearing, as designed. Recorded as PR3-2.

## Item 14 — search tools and git exclude

Proven, round 2. `Grep` skipped a directory listed in `.git/info/exclude`.

## Item 15 — hook working directory

Proven, round 2. The payload's `cwd` is the worktree for an isolated agent.

## Item 16 — worktree commands and the state mount

Partly, round 3 (R2, `logs/r2-result.json`). In a scratch project with `.factory` mounted as a git worktree
(orphan-branch stand-in), the platform created an agent worktree under `.claude/worktrees/agent-<hash>` for an
`isolation: worktree` dispatch and left the `.factory` worktree listed, mounted and intact after the agent
worktree was cleanly removed. Evidence note: the post-run `git worktree list` is not in the run's logs; the
check re-ran it on the probe project and confirms both the listing and the sentinel file. Not probed: a dispatch
whose worktree is kept, and a `PreCompact` hook running git inside its time limit before an actual compaction;
the latter needs a compaction to fire and stays open.

## Item 17 — fixed cost of an agent start

Measured, round 2. About 2,700 to 3,400 tokens of context for an agent with an explicit tool list, about 19,000
without one.

## Item 27 — hook rewrite through updatedInput

Proven, round 3 (R1, R2, R11). A `PreToolUse` hook returned `permissionDecision: "allow"` plus
`hookSpecificOutput.updatedInput` rewriting the call:

- Main session (R2): the rewritten Bash command ran; `probe.sh` printed `CALLER=ticket-42`, the value the hook
  injected, where the unrewritten call prints `CALLER=unset`.
- Dispatched agent (R2): the same, `CALLER=ticket-42`.
- Workflow step (R1): the same, `CALLER=ticket-42`.
- `StructuredOutput` call (R1, `logs/r1-result.json` P27-so-rewrite): the hook rewrote one field of the step's
  schema result (`ORIGVAL` to `ORIGVAL-HOOKREWRITE`) in `updatedInput`, and the awaited step result — the value
  the workflow script received — carried the rewritten field.

Caveats: every run was in the `auto` permission mode, so allow-plus-rewrite is shown here for auto mode only.
The default mode the design deploys in is item 38 of [[topic:platform-proven-2]]. Spelling: R11 ran with `updatedInput` only in the nested
`hookSpecificOutput` slot and the rewrite was still applied, so the nested slot is the honoured one. The runner
R1 to R10 used is not preserved — the copy in the spike plugin is the R11 version — so what the earlier rounds
emitted cannot be checked; R11 alone shows the nested slot is honoured.

The rewrite with `ask` instead of `allow` (P3COV-07) does not run the call headless in the `auto` permission
mode: see PR3-3.

## Item 28 — session id inside steps

Proven, round 3 (R1, `logs/r1.jsonl`). All 29 logged events of the run, including `PreToolUse`,
`SubagentStart`/`SubagentStop` and `StructuredOutput` events inside workflow-step subagents, carry the main
session's `session_id`.

## Item 29 — the platform process among ancestors

Partly, round 3 (R1, `logs/r1-result.json` P29). Walking `ps -axo pid,ppid,comm` upward from a Bash call inside
a workflow step on macOS, with no sandbox setting, reached, uninterrupted:
`python3 -> zsh -> claude -> zsh -> claude -> zsh -> login shell -> iTerm2 -> launchd`. Reading: the workflow
step runs inside the headless session process, so the nearer `claude` is that session's; the outer `claude` is
the long-lived interactive session that launched the probe. Supported: on macOS with no sandbox, a Bash call
inside a step finds the session's `claude` process as its nearest `claude` ancestor. The design risk sits in
nested setups, where a walk looking for "a long-lived claude" can hit the unrelated outer session; the nearest
`claude` ancestor is the anchor. Untested: the same walk from a hook, the sandbox-on case on macOS, and the
Linux leg.

## Item 30 — hook approval for the Workflow tool

Proven by the check's rerun C30 (recorded in CHECK.md). The round-3 run (R3, `logs/r3-result.json`) does not
show it: its transcript records `permissionMode: "auto"`, auto mode admits the control anyway, and no run
without the hook's allow was made. C30, in a fresh scratch project with `--permission-mode default` and
`Workflow` absent from `--allowedTools`: without hooks the Workflow call was refused ("Review dynamic workflow
before running") and listed in `permission_denials`; with the probe hook's `allow` on the `PreToolUse` event for
tool `Workflow`, the call ran and returned its result to the main session. Also seen: `PreToolUse` fires for
`ToolSearch`.

## Item 31 — typed arguments reach the workflow script

Proven, round 3 (R4, `logs/r4-result.json`). Typing `/fspike:args ARGTEXT-XYZZY-42 hello world` headless, the
script's `args` was the string `"ARGTEXT-XYZZY-42 hello world"` (`typeof args === "string"`). How it travels:
the skill expansion told the model to run `Invoke: Workflow({ name: "fspike:args", args: "..." })` and the model
made exactly that call, so the text reaches the script through the model copying it; no tokenising or wrapping
holds for this plain input, and quotes, newlines and long input are untested. The prelude's parser accepting
both a string and an object holds by the code; the probe says nothing about a programmatic object form.

## Item 32 — a forked skill's agent can dispatch

Proven, round 3 (R7, `logs/r7-result.json`). The forked skill's agent (tool list: `Agent`) dispatched another
plugin agent and its reply came back as the skill's result. The fork returned synchronously in the same run,
which also settles the third assumption of P3COV-09.

## Item 34 — encodeURIComponent

Proven, round 3 (R1, R3, `logs/r1-result.json`). `encodeURIComponent('a b&c/d')` in a workflow script returned
`a%20b%26c%2Fd`; no prelude encoder is needed.

## Item 35 — pairing a dispatched runner with its dispatch

Proven, round 3 (R5, `logs/r5.jsonl`). Three background dispatches overlapped: one dispatch's agent was still
pending when another made a guarded call, and each `SubagentStart` carries the session's `session_id`, its
dispatch's `subagent_type` and a distinct `agent_id`. Mechanics: `SubagentStart` carries no `tool_use_id`, and
`agent_id` exists only once the agent has started, so a pending admission created at the dispatch keys on
`(session_id, agent_type)`, with `agent_id` attaching at `SubagentStart`. The ordering case of probe 35
(P3COV-08) remains open at E-time.

## The project CLAUDE.md observation

Confirmed, with its scope narrowed, round 3 (R9a-c). A project `CLAUDE.md` in a project root under Claude Code's
own temp tree (`/private/tmp/claude-<uid>/`) is not loaded into a headless session: the transcripts'
injected-context lists show the global user `CLAUDE.md` only (the result files record the model's own account of
its context). The same scratch project under `$HOME` had its project `CLAUDE.md` loaded and quoted verbatim.
Untested: `/private/tmp` locations outside the temp tree, and `/tmp` as a separate case. Cause not probed (looks
like trust scoping by root); the behaviour is location-dependent. Fixture and sample projects for unattended
runs must live under `$HOME`, or their briefing must travel by hook. Recorded as PR3-1.

## Open after round 3

Items 12 (compaction hooks inside a step), 18 to 26, 33 — they need the product, a scheduler, CI, a browser, or
an event that cannot be forced headlessly; and item 29's untested legs (the walk from a hook, the sandbox-on
case, Linux). [[doc:platform]] carries the list and
where each is tested.

## Round 4

Round 4 (2026-11-24, same platform) and its independent check are items 36 to 49 of
[[topic:platform-proven-2]], with the evidence in `rewrite/spike/probes-round-4/`. They complete item 6
(items 42 and 43) and lift item 27's auto-mode caveat (item 38); what they leave open is listed in
[[doc:platform]]. This file keeps items 1 to 35 only.
