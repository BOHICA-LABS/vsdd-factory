# Evidence check: platform probes, round 3

Checked 2026-11-24 against the files in this directory and the session transcripts the probe runs left behind
(`~/.claude/projects/-private-tmp-claude-501--…-scratchpad-probe3-project/` and
`~/.claude/projects/-Users-jmagady--probe3-claudemd-scratch/`). The copies in `logs/` are byte-identical to the
probe's scratch logs. The transcripts are not in the repository and will not survive cleanup, so the excerpts
this check relies on are quoted below.

## Verdict

17 results examined, 13 supported, 4 not supported as written (items 6, 9, 29 and 30). Two of the four now have
an answer from a rerun made during this check (items 9 and 30). Two findings in `extra-probes.json` state more
than their evidence: PR3-3 names a permission mode that was never run, and PR3-4 proposes a fix that does not
work.

One fact affects several items: **every round-3 session ran in the `auto` permission mode.** Every transcript
records `permissionMode: "auto"`, which is the user-level `defaultMode`. No round-3 run used the default mode.

Scope: this report covers the 15 results that rest on round-3 evidence (6, 9, 10, 13, 15, 16, 27 to 32, 34, 35
and the CLAUDE.md observation). It also spot-checks the round-2 results 7 and 11 against the round-2 raw outputs,
which survive only in the scratchpad (`scratchpad/spike/out/`). The other round-2 carry-overs (1 to 5, 8, 14, 17)
were compared with the STATE.md summary and, roughly, with those raw outputs. They look consistent, but they
were not re-audited and are not counted here. Their only record in the repository is the STATE.md narrative.

## Per-item results

| Item | Reported | Verdict | Why |
|---|---|---|---|
| 6 long Bash calls | Partly | **Not supported** | The 600000 ms call was refused by the platform's sleep guard. LONGDONE came from a detached job plus a 236-second poll loop with timeout 300000. |
| 7 hook approval (round 2) | Proven | Supported | Spot-checked the step leg: default mode; `allowme` ran, the control returned "This command requires approval". |
| 9 hooks fire for MCP and WebFetch | Proven | **Not supported as worded** | The question is "when the matcher names them". The probe used a matcher-less hook. Rerun C9 answers it for the main session. |
| 10 MCP tool lists | Proven | Supported, with caveats | The calls succeeded in all three contexts. The unresolved name caused no error. "Exactly one mcp tool visible" is the agent's own claim. |
| 11 agent frontmatter (round 2) | Proven | Supported, with caveat | `omitClaudeMd` removed the injected global CLAUDE.md. The project-CLAUDE.md leg could not be observed (project under /private/tmp). |
| 13 missing launcher | Disproven | Supported, with caveats | The transcript records exit 127 as a non-blocking error on SessionStart and UserPromptSubmit. The prompt-expansion leg was never fired. |
| 15 hook working directory | Proven | Supported | r2 hook payload `cwd` is `…/.claude/worktrees/agent-a1086f85dbfc17f4a` for the isolated agent. |
| 16 worktree commands and state mount | Partly | Supported (partly) | One clean isolated dispatch left `.factory` listed and intact. The post-run listing was not saved; this check re-ran it. |
| 27 rewrite through updatedInput | Proven | Supported, with caveats | Direct tool results show `CALLER=ticket-42` in all three contexts. The journal shows the step submitted `ORIGVAL` and the workflow received `ORIGVAL-HOOKREWRITE`. Auto mode only. |
| 28 session id inside steps | Proven | Supported | All 29 r1 events carry `fa2ed106-…`. |
| 29 platform process among ancestors | Partly ("proven on macOS") | **Not supported as written** | The process chain is misread. The hook leg was not run. The sandbox was off. |
| 30 hook approval for Workflow | Proven | **Not supported by the recorded run** | r3 ran in auto mode, not default mode, and had no refused control. Rerun C30 in default mode supports the conclusion. |
| 31 typed workflow arguments | Proven | Supported, with caveat | `args` was the string. The text reached the script through the model, which copied an `Invoke: Workflow({…, args: "…"})` line. |
| 32 forked skill dispatches | Proven | Supported | Fork agent called Agent(`fspike:clerk`); clerk handed back `DISPATCH-OK-77`; skill stdout `DISPATCH-OK-77`. |
| 34 encodeURIComponent | Proven | Supported | Runtime-recorded workflow results: `a%20b%26c%2Fd` (r1), `x%20y` (r3). |
| 35 runner-dispatch pairing | Proven | Supported, but by r5, not the cited r2b | The r2b dispatches did not overlap, and r1's mixed-type pair failed to start. The three background dispatches in r5 do overlap. |
| CLAUDE.md observation | Proven | Supported, with scope caveat | The transcripts' injected context shows it. The cited result files are the model's own account of its context, answering a question about a marker the file does not contain. Only `/private/tmp/claude-501/…` was tested. |

## Details

### Item 6: the ten-minute timeout was never accepted

From the P6 step transcript (`…/wf_02bdaa2e-c47/agent-afe9056df08efb6f0.jsonl`):

```
15:53:55 Bash {"command": "sleep 240 && echo LONGDONE", "timeout": 600000}
         -> error: Blocked: sleep 240 followed by: echo LONGDONE. To wait for a condition, use Monitor with an
            until-loop ... To wait for a command you started, use run_in_background: true.
15:54:00 Bash {same, "timeout": 600000, "run_in_background": true}  -> "Command running in background with ID ..."
15:54:06 Bash {"command": "until grep -q LONGDONE <task output file>; do sleep 2; done; cat ...", "timeout": 300000}
15:58:02 -> LONGDONE [exited with code 0]
```

The 600000 ms call was refused because the command starts with a sleep; the timeout value was never tested. The
only foreground call that completed lasted 236 s with timeout 300000. So `platform.md` ("600000 ms timeout
accepted, 240 s call ran in a step") and `platform-proven.md` ("the ten-minute ceiling is accepted by the tool")
are both unsupported. The reported consequence ("the wait-slice fallback is not needed so far") is the reverse of
what happened: the agent ran a background job and polled it, which is the wait-slice pattern.

What the run does show: inside a step, a foreground Bash call with timeout 300000 runs for about four minutes.
It also shows a new fact: the Bash tool refuses commands that start with `sleep N && …`. The E2 ten-minute proof
and any long-running test command must not begin with a sleep.

### Item 9: matcher-less hook; rerun C9

`hooks.json` gives `PreToolUse` no matcher, and `platform-proven.md` says so honestly. But the table states the
item as proven, and the design's real matcher (`topics/hook-wiring.md`) depends on regex alternation over MCP
names (`mcp__(plugin_vsdd-factory_)?(perplexity|tavily|context7)__.*`). That matcher behaviour was never tested.

**Rerun C9.** Scratch project, default mode, main session only. Six tagged `PreToolUse` entries; one call to
`mcp__tiny__ping` and one WebFetch. Hook log:

```
catch-all                 PreToolUse mcp__tiny__ping   (permission_mode default)
regex-mcp                 PreToolUse mcp__tiny__ping   matcher "mcp__tiny__.*"
alternation-design-style  PreToolUse mcp__tiny__ping   matcher "Bash|Read|mcp__(plugin_fspike_)?tiny__ping|WebFetch"
alternation-design-style  PreToolUse WebFetch
webfetch-exact            PreToolUse WebFetch          matcher "WebFetch"
catch-all                 PreToolUse WebFetch
(no line for "mcp__nosuchserver__.*" or "Bash": both controls stayed silent)
```

So regex and alternation matchers that name an MCP tool do fire, and controls that do not name it stay silent.
This was run for the main session only. The plugin-bundled spelling `mcp__plugin_<plugin>_<server>__…` was not
exercised, because the server came from `--mcp-config`.

**PR3-4's fix fails.** The same run sent WebFetch to `http://127.0.0.1:8139/idx.html`, where a plain-http server
was listening. The hostname rule did not fire, but WebFetch upgraded the request to HTTPS. The server logged a
TLS ClientHello (`code 400, message Bad request version …`), and the tool returned "The server returned HTTP
502 Bad Gateway. The response body was not retrieved." So "localhost fixtures must use 127.0.0.1" does not work
for an http fixture. Of the two routes PR3-4 offers, only the shell is left, and it was not tested either.

### Item 10: MCP tool lists

The tool results in the subagent transcripts are direct: `TINY-PONG {}` for the main session, `fspike:mcpping`
(dispatch), `fspike:mcpboth` and the r5b workflow step. The `mcpboth` agent, which also listed
`mcp__nosuchserver__ping`, started and worked with no error anywhere; that is the "silently" half, and it is
direct. "Exactly one mcp tool visible" is the agent's own account of its tool list (its own words: "mcp__tiny__ping,
mcp__tiny__SubagentHandback? No—only mcp__tiny__ping"). The main-session leg is a session with no tool list, not
"an agent whose explicit tool list names" the tool. The reported consequence ("MCP-capable agents (and only
those) can keep inherited session tools") mixes things up. Per `topics/research-and-browser-tools.md`, dropping
the `tools` list is the *fallback* for when listing both spellings fails, and this probe shows that listing both
works.

`tiny_mcp.py` reads `params.args` where MCP sends `params.arguments`, so every echo came back as `{}`. That does
not affect the result.

### Item 13: fail-open, but the cited file does not show it

`r6-result.json` and the empty stderr do not, on their own, show that the hooks ran at all. The transcript
(`128bbfb6-….jsonl`) does:

```
attachment hook_non_blocking_error  SessionStart:startup  exitCode 127
  "Failed with non-blocking status code: sh: …/plugin-nolauncher/hooks/missing-launcher.sh: No such file or directory"
attachment hook_non_blocking_error  UserPromptSubmit      exitCode 127  (same text)
```

The negative result holds. Three corrections: the hooks were not "silently absent", since their failures are in
the transcript; the prompt was a plain "Reply with exactly READY", so `UserPromptExpansion` (named in the
question) never fired; and `PreToolUse` never fired either, since no tool was called.

### Item 16: mount survival

The r2 isolated dispatch created and cleanly removed `.claude/worktrees/agent-a1086f85dbfc17f4a` (meta:
`"worktreeCleanlyRemoved": true`). The `git worktree list` output the result cites is not in `logs/`. This check
re-ran it on the probe project: `.factory 102efa2 [factory-artifacts]` is still listed, and
`.factory/sentinel.txt` reads `FACTORY-SENTINEL-42`. Only the clean-removal path was exercised. A dispatch that
leaves changes behind, so that its worktree is kept, was not.

### Item 27: supported; caveats on mode and on the preserved runner

The direct tool results are `CALLER=ticket-42` in the main session and the dispatched agent (r2 transcripts),
in the workflow step (r1), and again in r11. The r1 journal shows the P27SO step's one `StructuredOutput` call
carried `"output": "ORIGVAL"`, while the awaited result was `{"label": "P27SO", "output": "ORIGVAL-HOOKREWRITE"}`.
Caveats:

- Every run was in auto mode. Allow plus rewrite was never exercised in the default mode the design deploys in.
- `plugin-additions/hooks/runner.py` is the r11 version (modified 10:04:53, after r10c). It emits only the nested
  slot, although its docstring still says "both". The runner that r1 to r10 used is not preserved, so the claim
  that "R1 to R2 emitted both" rests on narrative. r11 alone does show that the nested slot is honoured.
- **PR3-3 overstates.** It says the ask-with-rewrite case fails "in either the default or the auto permission
  mode". All three r10 sessions (`4cc42318`, `78e33f23`, `06da155a`) ran in auto mode. The failure itself is
  direct: `permission_denials` lists the rewritten command, and the side-effect file of `probe-ask2.sh` was never
  created. Only the mode claim is unsupported.

### Item 29: process chain misread; two legs not run

The P29 step printed:
`13051:python3 -> 13050:zsh -> 10651:claude -> 10649:zsh -> 21191:claude -> 20856:zsh -> 41179:-zsh -> login -> iTermServer -> iTerm2 -> launchd`.

`platform-proven.md` reads this as "claude (workflow step) -> zsh -> claude (headless session)". That is wrong.
Workflow steps run inside the session process. 10651 is the headless `claude -p` session, and its parent 10649
is a Bash-tool shell of PID 21191. At check time, 21191 was the long-lived interactive session that launched the
probe, with 13h45m elapsed. The design risk is the reverse of the reading: in nested setups, a walk that looks
for "a long-lived claude" can find an unrelated outer session. The nearest `claude` ancestor is the right one.

Two legs of the question were not run. The ancestor walk ran only from a Bash call, not from a hook. And the
sandbox was off: there is no sandbox setting, and the text itself says "no sandbox override". Yet `platform.md`
attaches the "with the Bash sandbox on" condition only to the Linux leg. What is supported: on macOS with no
sandbox, a Bash call inside a step finds the session's `claude` process as its nearest `claude` ancestor.

### Item 30: auto mode, no control; rerun C30

r3's transcript records `permissionMode: "auto"`, and round 2 had already found that auto mode lets the control
through. No run without the hook's allow was made. So r3 shows nothing about the hook's approval.

**Rerun C30.** Fresh scratch project, `--permission-mode default` (confirmed `"default"` in all three
transcripts), the same prompt shape as r3, and Workflow not in `--allowedTools`:

| Run | Plugin | Result |
|---|---|---|
| control (`09fdc159`) | no hooks | Workflow refused: tool error "Review dynamic workflow before running"; `permission_denials` lists `Workflow {"name":"fspike:perm30"}` |
| allow (`b4118853`) | the probe hook (allows Workflow) | hook verdict `workflow-allowed`; workflow returned `PERM30-WORKFLOW-RAN`; no denials |

The conclusion holds in default mode, but only on this rerun's evidence. A side note: on a first control attempt
(`3899aee8`), the model went through the Skill tool (`Skill(fspike:perm30)`), was refused, and never called
Workflow. In default mode a by-name entrance can therefore also need a `Skill(…)` approval.

### Item 31: the model relays the arguments

The transcript (`8f77e400-….jsonl`) shows the typed `/fspike:args ARGTEXT-XYZZY-42 hello world` expanded into a
prompt containing `Invoke: Workflow({ name: "fspike:args", args: "ARGTEXT-XYZZY-42 hello world" })`. The model
then made exactly that call, and the runtime's record shows `args: 'ARGTEXT-XYZZY-42 hello world'`. "String" is
supported. "Verbatim, no tokenising" holds for this simple input, but it depends on a model copying the text:
quotes, newlines and long text are untested. The sentence "the object form is what the Workflow tool receives
programmatically" has no evidence behind it.

### Item 35: right result, wrong citation

r2b: both Agent calls were in one assistant message, but the plain agent had already handed back before the
rewriter dispatch's `PreToolUse` fired, so the two never overlapped. r1: the planned mixed-type pair failed,
"agent type 'fspike:plain' not found" (`plain.md` is dated 09:58:50, after r1 started), so r1's parallel steps
were four `fspike:clerk` and one untyped `workflow-subagent`. The real evidence is r5, which has three background
dispatches. There, `mcpboth`'s dispatch is pending (line 6) while `mcpping` makes a guarded call (line 7), before
`mcpboth`'s `SubagentStart` (line 8). Each `SubagentStart.agent_type` matches its dispatch's `subagent_type`, and
the `agent_id`s are distinct.

A correction to the reasoning: `SubagentStart` carries no `tool_use_id`, and `agent_id` exists only once the
agent has started. So a pending admission created at the dispatch can be keyed only on `(session_id,
agent_type)`, and `agent_id` attaches at `SubagentStart`. The triple key, as worded, cannot be formed on the
dispatch side.

### The CLAUDE.md observation: right, but the probe could not tell

r9a-c asked whether the context held `CLAUDEMD-PROJ-MARKER-123`. The project `CLAUDE.md` contains
`STATUS-MARKER-123`, so NO was the right answer in all three runs whatever was loaded. The cited result files
rest on the model's free-text description of its own context. The transcripts settle the question directly:

```
631bae21 (r9a, /private/tmp)   injected: /Users/jmagady/.claude/CLAUDE.md
3c02f94c (r9b, /private/tmp)   injected: /Users/jmagady/.claude/CLAUDE.md
130b9379 (r9c, $HOME)          injected: /Users/jmagady/.claude/CLAUDE.md, ~/.probe3-claudemd-scratch/CLAUDE.md
fa2ed106, 4d98dd0a (r1, r2 under /private/tmp): global only
```

Scope caveat: both /private/tmp runs were inside `/private/tmp/claude-501/…`, Claude Code's own per-user temp
tree, and both transcripts record the same resolved cwd. "Under /private/tmp" in general, and "reached through
/tmp" as a separate case, were not shown. PR3-1 and `platform-proven.md` should say "under Claude Code's temp
tree (`/private/tmp/claude-<uid>/`); other /tmp locations untested". The design consequence (fixtures under
`$HOME`, briefings by hook) is safe either way.

### Items 7 and 11 (round 2, spot-checked)

- Item 7: `scratchpad/spike/out/p-perm.json` is a default-mode session (`ede8d696`). Its workflow returned
  `[{"label":"allow","output":"ALLOWME-RAN"},{"label":"control","output":"This command requires approval"}]`, so
  the refused control is there. Supported.
- Item 11: the round-2 session `431f0c3d` (auto mode) injected `/Users/jmagady/.claude/CLAUDE.md` into the
  `fspike:plain` agent and no CLAUDE.md into `fspike:nomd`. So `omitClaudeMd` works, directly shown. That
  project lived under `/private/tmp`, where no project CLAUDE.md is ever loaded, so withholding a *project*
  CLAUDE.md, which is what the holdout-evaluator wall relies on, was not observable. Re-run that leg with the
  project under `$HOME`.

## What platform.md and platform-proven.md should say instead

- Item 6 (table and topic): "Partly. A foreground call with timeout 300000 ran about 240 s in a step. The tool
  refuses commands that begin with `sleep`, so the 600000 ms call was never tried. Ten minutes stays with E2,
  with no leading sleep."
- Item 9: "Proven for a catch-all hook in main, dispatch and step. Named matchers (regex and design-style
  alternation) fire for an MCP tool and for WebFetch in the main session, with controls silent (check rerun C9).
  The plugin-bundled spelling is untested." Drop "localhost fixtures must use 127.0.0.1": WebFetch upgrades
  http to https, so a plain-http loopback fixture is unreachable.
- Item 13: say the failures are recorded as non-blocking hook errors in the transcript, and that the
  prompt-expansion and `PreToolUse` legs were not exercised.
- Item 27: say auto mode only. Drop "R1 to R2 emitted both" unless that runner is recovered.
- Item 29: correct the chain (the step runs inside the headless session; the second `claude` is the outer
  interactive session). Say the hook leg and the sandbox-on case are untested on macOS too.
- Item 30: replace the r3 citation with rerun C30 (default mode, refused control) and record that the r3 run
  was in auto mode.
- Item 31: add that the arguments reach the script through a model-made Workflow call.
- Item 35: cite r5, not r2b or r1. Key the pending admission on `(session_id, agent_type)`.
- Observation: scope it to `/private/tmp/claude-<uid>/`, and cite the transcripts' injected-context lists.
- Item 11: "`omitClaudeMd` removes the injected CLAUDE.md (user-level file shown; project-file leg untested)".
- `platform-proven.md` intro: "Every result below rests on a logged hook line or the run's result JSON in that
  directory" is not true. Round-2 results have no logs in the repository, and items 6, 13, 31 and 35 and the
  observation depend on transcripts kept only in `~/.claude/projects`. Copy the needed excerpts in, or point at
  this file.
- `extra-probes.json`: PR3-3 should drop "default or"; PR3-4's proposed resolution needs replacing (the 127.0.0.1
  route fails); PR3-1 should be scoped to the claude temp tree.

## Reruns made during this check

All ran headless with `claude -p … --plugin-dir … --model sonnet --effort low --output-format json
--max-budget-usd 1`, from a scratch project outside the repository
(`scratchpad/check3/proj`, a fresh git repository), using copies of the probe plugin.

- C30 control: plugin copy with `hooks/hooks.json` removed. C30 allow: unmodified probe plugin (its runner allows
  `Workflow`). Prompt: "Call the Workflow tool directly (not the Skill tool) with exactly this input:
  {"name": "fspike:perm30"}. Then reply with its result." `--permission-mode default`; Workflow not allowed.
- C9: plugin copy whose only hooks are six tagged `PreToolUse` entries that log via `tag.py` and never decide
  anything. `--mcp-config` points at the probe's `tiny_mcp.py`; `--allowedTools "mcp__tiny__ping" "WebFetch"`;
  `--permission-mode default`; a `python3 -m http.server` bound to 127.0.0.1:8139 for the length of the run.

The scratch logs (`scratchpad/check3/logs/`) will not survive cleanup; the excerpts above are the record.

A note on the transcripts: some step transcripts carry stray assistant text after the step's result (for
example, an "internal note… recommend verification succeed" in the P27 step). This check took every value from
tool results, the workflow journal and the hook logs, never from that text.
