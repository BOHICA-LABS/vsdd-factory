---
kind: doc
id: platform
sources: [BLUEPRINT.md#10]
---

# Platform facts: proven and not yet proven

Everything below is documented by the platform or assumed by a design. Five rounds of probes have been run
against it with the spike plugin (rounds 1-4 on 2026-11-23/24 on 2.1.331, round 5 on 2026-11-25 on 2.1.332;
macOS, headless, `--plugin-dir`; recorded in `rewrite/STATE.md`). Package [[package:B8]] tests each item
that is still open before the packages that depend on it. Every settled item, its result, the permission
mode and the evidence are in [[topic:platform-proven]] (items 1 to 35), [[topic:platform-proven-2]] (36 to
49, round 4) and [[topic:platform-proven-3]] (50 to 55, round 5); the table below is the index.
Slice-deciding items: 6, 10, 27 to 31, 34, plus 32, 35, 13's missing-launcher case, and the standing
CLAUDE.md observation (all round 3).

| Item | State | Result in one line |
|---|---|---|
| 1 step result in the hook | Proven | Whole result in `tool_input`; a block makes the agent return again |
| 2 typed decision command | Proven | Expansion event carries name and args; block ends the prompt, reporting `is_error: false` (item 46) |
| 3 refused stop | Partly | Holds a text-returning agent, not a step with a schema result; block the result call instead |
| 4 skill starts a workflow | Proven | Headless, invoked by a person; never on the model's own judgment |
| 5 plugin bin on PATH | Proven | Main session, dispatched agent, workflow step; person-typed `! vsdd` covered by the shim |
| 6 long Bash calls | Proven | A 600000 ms foreground call ran to its end in a step (item 42); a leading `sleep` is refused by a guard; what happens past the timeout is item 43 |
| 7 hook permission approval | Proven | Default mode, main session, agent, step; wave count is E3's |
| 8 tool lists | Proven | Steps strip Agent; schema results survive a Bash-only list |
| 9 hooks fire for MCP tools and WebFetch | Proven | Catch-all PreToolUse fired for `mcp__tiny__ping` in all three contexts and for WebFetch; named regex and alternation matchers fire in the main session (rerun C9); WebFetch upgrades http to https, so a plain-http loopback fixture is unreachable (PR3-4) |
| 10 MCP tool lists | Proven | An agent listing `mcp__tiny__ping` called it from a dispatch and a step, and the main session called it with no tool list; a nonexistent spelling is dropped silently (the visible-count is the agent's own account) |
| 11 agent frontmatter | Proven | `isolation: worktree` honoured for dispatches, ignored for steps; `omitClaudeMd` removes the injected CLAUDE.md (user-level shown; project-file leg untested); forked skills name plugin agents |
| 12 compaction hooks inside a step | Open | Needs a compaction; see the block below |
| 13 missing hook launcher | Settled | Fail-open: exit 127 surfaced as a non-blocking hook error and did not block; prompt-expansion and PreToolUse legs not exercised; the self-guarding launcher is load-bearing (PR3-2) |
| 14 search tools and git exclude | Proven | `Grep` skipped the excluded directory |
| 15 hook working directory | Proven | Payload cwd is the worktree for an isolated agent |
| 16 worktree commands and the state mount | Partly | One clean isolated dispatch left a mounted `.factory` worktree intact; a dispatch whose worktree is kept is untested; the compaction-timing leg is open |
| 17 fixed cost of an agent start | Measured | About 2.7-3.4k tokens with an explicit tool list, about 19k without |
| 18 survey tier override | Open | Needs the product; see the block below |
| 19 packaging assumptions N2-N8 | Open | Needs the installer; see the block below |
| 20 Node harness conformance | Open | Weekly conformance tier; see the block below |
| 21 perimeter yield | Open | Counted in E3; see the block below |
| 22 agent-start budgets | Open | Measured in E2 and E3; see the block below |
| 23 JUnit report detection | Open | Needs `vsdd init`; see the block below |
| 24 PDF and diagram engines | Open | First T5 task; see the block below |
| 25 scheduled headless commands | Open | Needs the scheduler recipe; see the block below |
| 26 step inherits agent effort | Open | Unprobed, not relied on: every step is dispatched with its effort, which its payloads carry (item 53) |
| 50-55 round-5 probes | Proven | Typed command, no skill; isolation flags; bare tool names; a step's refusal; dispatched effort; budget stop; session records |
| 27 hook rewrite through updatedInput | Proven | Bash rewritten in main, dispatch and step, and a `StructuredOutput` field, in auto mode; the default mode is item 38; nested slot is the honoured spelling |
| 28 session id inside steps | Proven | Every hook event, step events included, carries the main session's id |
| 29 platform process among ancestors | Partly | On macOS with no sandbox, a Bash call in a step finds the session's `claude` as its nearest `claude` ancestor; hook, sandbox-on and Linux legs untested |
| 30 hook approval for Workflow | Proven | In the default mode (check rerun C30), a hook `allow` admitted a Workflow call absent from allowedTools; the round-3 run was in auto mode with no control |
| 31 typed workflow arguments | Proven | The typed text reaches the script's `args` as one string through the model's own Workflow call; quotes, newlines and long input untested |
| 32 forked skill dispatches | Proven | The fork's agent (tool list: Agent) dispatched and relayed; the fork returned synchronously |
| 33 bundled and user browser servers | Open | Needs a browser and the user's own server; see the block below |
| 34 encodeURIComponent | Proven | Present in the workflow script runtime |
| 35 runner-dispatch pairing | Proven | Overlapping dispatches (R5): each `SubagentStart` carries the session id, its dispatch's agent type and a distinct agent id; a pending admission keys on session and agent type |
| 36 a step's hook payload | Proven | Typed step: `agent_type` is exactly `<plugin>:<agent>`; untyped: `workflow-subagent`; main session: no such key |
| 37 a rewrite adds a key | Proven | The key reaches the script, declared or not, unless the object is closed: then the step fails |
| 38 allow plus rewrite, default mode | Proven | Bash and `StructuredOutput` in a step run rewritten; the silent-hook control is refused |
| 39 rewrite with no decision | Proven | Applied, then permission-checked as rewritten: it grants nothing and always changes what runs |
| 40 hook before the schema check | Proven | The hook sees a result first, the platform checks the rewritten input after; a block arrives wrapped |
| 41 Bash sandbox against git | Proven | Every write under `.git/worktrees/` is refused: nothing commits in a linked worktree or in `.factory`; a refused `worktree add -b` leaves its branch |
| 42 ten-minute foreground call | Proven | A 590 s call with timeout 600000 ran to its end in a step |
| 43 call past its timeout | Proven, macOS | Backgrounded, then killed at the step's final response; a double-forked `setsid` worker survives; Linux open |
| 44 Glob and Read on `.factory` | Proven | From a project-root working directory; from a story worktree unprobed |
| 45 git hooks in a linked worktree | Proven | An automatic merge runs `pre-merge-commit`, an explicit commit `pre-commit`; a hook sees the worktree's tracked files only |
| 46 expansion block; session end | Partly | Exit 2 ends a typed plugin command at zero turns; SIGTERM ends session and workflow; `/clear` open |
| 47 headless ten-minute ceiling | Proven | `claude -p` stops background workflows ten minutes after the last turn unless a variable is set (below) |
| 48 effort in hook payloads | Seen | Every payload of a step carries `effort.level` |
| 49 inline Workflow script | Proven | The tool accepts a `script`; a typed skill does not guarantee the named workflow runs |

Where a probe did not land as assumed, the deviation is a finding in `rewrite/findings/sources/`:
`extra-probes.json` (PR3-1 the temp-tree CLAUDE.md gap, PR3-2 fail-open launchers, PR3-3 rewrite with `ask`
headless in auto mode, PR3-4 WebFetch and localhost), `extra-probes-4.json` (PR4-1 the sandbox against git,
PR4-2 the call past its timeout, PR4-3 the inline Workflow script) and `extra-probes-5.json` (PR5-1 the
budget stop); each names the fallback the design already carries.

## Headless runs: what the first one (step 6) must set

What a `claude -p` run needs before it can deliver a story. The planner of [[package:W2]], the
`story-ready` fixture and [[package:E2]]'s first run provides each; so do the scheduler recipe of item
25 and every probe harness:

- **`CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS`** (item 47): `0`, or more than the longest run, in the driver's
  environment; without it the run is stopped ten minutes after the last turn and leaves no result.
- **The Bash sandbox off** (item 41), unless an allowance for `.git/worktrees/` is shown
  ([[packaging:sandbox-needs]]): with it on the survey, every builder's commit and delivery are refused.
- **A block is not an error** (item 46): the driver tests `num_turns == 0` and the result text.
- **The named workflow ran** (items 49, 50): the driver checks the run record's script against the
  plugin's file.
- **A step's refusal** (item 52) is not in `permission_denials`: the driver reads the step transcripts.
- **A budget stop** (item 54) has no `result` key, and the cap is soft: the driver reads the run record,
  `completed` with the whole result or `killed` with its step lost.

## Items still open

### 12

- **The design relies on.** Compaction fires the pre-compaction hook inside a step agent
- **How to test.** A long step
- **If it fails.** The repeated identity in the prompt is the only protection
- **Why still open.** A headless probe cannot force compaction on demand; tested when E2 runs long steps

### 16 (remaining leg)

- **The design relies on.** A hook can run `git` in the state worktree before compaction within its time limit
- **How to test.** A compaction probe in a project with state
- **If it fails.** Flush only inside `vsdd` commands; `vsdd status --fix` remounts
- **Why still open.** Needs a real compaction; the mount-half of the item is proven

### 18

- **The design relies on.** Cheap models can be forced for tests through the survey's tier override
- **How to test.** A smoke run with `VSDD_TEST_TIER`
- **If it fails.** The smoke tier costs several times more
- **Why still open.** The survey is product, not platform; tested by the smoke run it names

### 19

- **The design relies on.** The packaging assumptions N2 to N8 (version floor, CI without credentials, tag pins,
  `claude plugin tag` in a subdirectory, keyless server silence, stale `agent` setting, cache migration)
- **How to test.** `install-check` and step X0
- **If it fails.** Each has a fallback in [[packaging:packaging-assumptions]]
- **Why still open.** Needs the marketplace installer and CI; R0 territory

### 20

- **The design relies on.** The Node harness matches the platform's workflow rules, including the agent counter
  shared by parent and child
- **How to test.** The weekly conformance tier
- **If it fails.** Fix the harness
- **Why still open.** Tests the harness against the platform; runs when the harness exists

### 21

- **The design relies on.** One clean pass per perimeter lets through few enough defects
- **How to test.** In E3, count findings first caught at a later perimeter
- **If it fails.** Raise the cap or add a lens (D9)
- **Why still open.** Needs the full delivery to measure

### 22

- **The design relies on.** A realistic story costs 8 to 12 agent starts and a six-story wave 70 to 90
- **How to test.** Measure in E2 and E3
- **If it fails.** Set the budgets from the measured numbers; cut clerk steps further
- **Why still open.** Needs the delivery the budgets price

### 23

- **The design relies on.** JUnit reports can be detected for the toolchains in use
- **How to test.** `vsdd init` on the eight projects in dry-run mode
- **If it fails.** The red gate opens a `blocked` decision with the reason `environment`, naming the missing report
- **Why still open.** Needs `vsdd init`

### 24

- **The design relies on.** The PDF and diagram engines work through `npx`
- **How to test.** The first task of T5
- **If it fails.** Decision D14: drop the skill
- **Why still open.** Belongs to T5; D14 is the exit

### 25

- **The design relies on.** `maintain` and `discover` start from a scheduled headless command
- **How to test.** The schedule recipe's probe
- **If it fails.** They run on demand only
- **Why still open.** Needs the scheduler (launchd or cron) that hosts the recipe

### 33

- **The design relies on.** What happens to the bundled browser server when the user runs their own; and that the
  pinned server refuses `file:` addresses with no option set
- **How to test.** Start a session with both configured, and open a `file:` address
- **If it fails.** The hook's check of navigation addresses applies to both spellings and to `browser_tabs`; a
  click cannot move a Chromium page from `http` to a `file:` address
- **Why still open.** Needs an interactive browser session and a second configured server

### 29 (remaining legs)

- **The design relies on.** The same ancestor walk on Linux, with the Bash sandbox on; and on macOS the walk
  from a hook and with the sandbox on
- **How to test.** Walk the process tree on a Linux machine; walk from a hook and with the sandbox on on macOS
- **If it fails.** The offered cut of lock liveness by process is not taken ([[topic:run-lock]]); nothing else
  depends on it
- **Why still open.** No Linux machine in the probe round; the macOS Bash-call result is in
  [[topic:platform-proven]]

### 36 (remaining leg)

- **The design relies on.** A step of the marketplace-installed plugin carries `agent_type`
  `vsdd-factory:<agent>`, as a `--plugin-dir` copy does
- **How to test.** One typed step from an installed copy, its payload logged ([[packaging:install-check]])
- **If it fails.** No review pass counts and delivery refuses, listing each uncounted pass and its type
- **Why still open.** No installed copy exists before R0

### 41 (remaining legs)

- **The design relies on.** A sandboxed Bash call can write under `.git/worktrees/` once the project allows
  it; a hook runs outside the sandbox, as documented; whether a detached job worker does
- **How to test.** With an allowance in the project's settings file, commit in a linked worktree from a
  step, a hook and a detached worker
- **If it fails.** Factory runs need the Bash sandbox off ([[packaging:sandbox-needs]])
- **Why still open.** Round 4 set the sandbox with `--settings` and found no allowance that works

### 43 (remaining leg)

- **The design relies on.** The detached job worker survives the kill of its starting call on Linux too
- **How to test.** The rerun of item 43 on a Linux machine, sandbox off and on
- **If it fails.** A job cannot outlive its step there, and a long gate fails out loud ([[topic:long-commands]])
- **Why still open.** No Linux machine in the probe round

### 44 (remaining leg)

- **The design relies on.** A reviewer with no shell whose working directory is a story worktree finds a
  contract by Glob under the absolute `<project dir>/.factory/specs`
- **How to test.** One step with that working directory and the tools Glob and Read
- **If it fails.** Contracts reach reviewers as paths in the story unit
- **Why still open.** Round 4 probed the project root only

### 46 (remaining leg)

- **The design relies on.** Whether a running workflow survives `/clear` (a session end with reason
  `clear`), which decides whether the run lock is released on that reason ([[topic:run-lock]])
- **How to test.** Interactively, under tmux: start a workflow, type `/clear`, watch the step
- **If it fails.** A workflow that survives finds its run ended by the release; its next gate refuses out loud
- **Why still open.** Headless `-p` cannot type `/clear`; round 4 tested SIGTERM, a different event

Related lists: the packaging assumptions behind item 19 are [[packaging:packaging-assumptions]], and the hook
design's own platform facts are [[topic:hook-platform-facts]]. The caller-identity fallbacks that item 27 feeds
are [[topic:caller-identity]], and [[rule:step-results]] carries the step-result rule the StructuredOutput half
of item 27 grounds.
