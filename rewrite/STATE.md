# vsdd-factory 1.0 rewrite — resume record

A session starting cold reads this file first. It is the single source of truth for the rewrite effort.
It is independent of `.factory/`, the `factory-artifacts` branch, and the process rules in the repo `CLAUDE.md`,
all of which describe the pre-rewrite system and are historical reference only.

Last updated: 2026-11-23 (session 1).

## Resume in one breath

We are rewriting vsdd-factory from scratch as version 1.0: a native Claude Code plugin with JavaScript workflows
for orchestration, a small Python hook runner launched through `uv`, and every agent and skill rewritten.
The survey of the old system is done and saved under `rewrite/reference/`. The plan is agreed and the platform
spike passed (results below; the spike plugin is kept in `rewrite/spike/plugin/`).
**The design workflow (step 2) ran to completion on 2026-11-23** (nine agents, none failed, about two hours,
about 4.5 million subagent tokens; script at `rewrite/workflows/design.js`). It produced:
- `rewrite/BLUEPRINT.md` (997 lines): the document the user approves. Section 2 lists 30 decisions, 14 of which
  block the build. Section 9 is a build plan of about 70 packages. Section 10 lists 26 untested platform assumptions.
- `rewrite/design/{pipeline,state,agents-skills,hooks,packaging}.md`: the five detail designs, amended to match.
- `rewrite/design/review-{seams,coverage,operations}.md`: 96 findings (13 blocker, 56 major, 27 minor).
  The synthesizer marked 94 fixed and 2 deferred to the user (both are decision D4).

The design in one line: 20 agents, 13 skills, 12 workflows, about 70 method files, 17 hook rules, and one
standard-library Python package that is both the hook runner and a `vsdd` command. Workflows order the work, the
tool measures and records evidence, hooks refuse what the evidence does not support. Three routine human
decisions per cycle (brief, specification, release).

Caveats recorded by the assistant after checking the result:
- The 94 fixes were applied by one agent in one pass and have not been reviewed again.
- The design is large (843 KB of documents) and the operations reviewer noted bookkeeping creeping back (OPS-28).
- Two facts found on disk: the marketplace repository is now `bohica-labs/claude-mp`, and all eight old installs
  carry a stale `agent` setting.

**Decisions: on 2026-11-23 the user adopted every recommendation in section 2 of the blueprint (D1 to D30).**
Each recommendation is now a fixed decision. That includes: state on its own branch, committed and pushed by the
plugin (D1, D2); an importer for the eight old projects (D3); the plugin approving its own routine tool calls
(D4); a fourth "clerk" tier of Sonnet at low effort, Opus at extra-high effort for reviewers, and model aliases
(D5); the main session walled like an agent (D6); `/vsdd-factory:spike` as a workflow (D8); one clean review pass
per perimeter with a cap of three (D9).

The user also asked whether brownfield is covered: adding features to an existing project the factory did not
build, and adding features after the greenfield build is finished. It is (`ingest` then `feature`, and `feature`
alone), and both are now explicit cases the second pass must walk end to end.

**The second pass (step 2b) ran to completion on 2026-11-24** (eight agents, none failed, about two hours, about
4.1 million subagent tokens; script at `rewrite/workflows/second-pass.js`). Results:
- Only 41 of the 96 first-pass fixes were fully in place; the rest were partial and three had regressed.
- 118 new findings: 4 blockers, 43 major, 71 minor. The blockers came from two root problems: a story written
  after a workflow's opening survey got no worktree (which stalled every brownfield path and every bug fix), and
  no agent was allowed to write build manifests or repository configuration (which stalled the first wave of a
  new product). The requirements reviewer found that, as first written, none of five brownfield cases reached
  a release.
- The amender fixed 100 defects and applied 16 cuts. Brownfield is now explicit in blueprint section 3.4, and a
  new build package, E4, proves the brownfield cases before cutover. The platform probe results are folded into
  section 10. Counts after: 20 agents, 13 skills, 12 workflows, 17 hook rules, about 48 `vsdd` subcommands
  (was about 60), 14 record types (was 19), 74 build packages. The blueprint is now 1,676 lines.
- The close-out check was **not clean**: no blocker, 5 major and 16 minor problems remain
  (`rewrite/design/pass2-closeout.md`). Three of the majors are new seams between second-pass fixes.
- One new question for the user: the scope of decision D4 at setup, `core` (what was adopted) or `extended`
  (also approves read-only commands and spike experiments; recommended for unattended runs). One cut is offered
  pending a probe (decide lock staleness by whether the holding process exists).

**A close-out fix run was launched on 2026-11-24** to resolve those 21 items: one Fable fixer, one Opus verifier,
at most two rounds. Script at `rewrite/workflows/closeout-fix.js`; the verifier writes
`rewrite/design/pass2-closeout-2.md`.

**User instruction, 2026-11-24:** "run one more full review pass and then go to the thin slice next. make sure we
fix what we find during both methods." The user also asked whether the two must run one after the other or can
run in parallel. The assistant's answer and plan (the user has not yet replied to it):

1. The close-out fix run (2c) finishes first. Everything else reads the six design documents it is editing.
2. Then two things run at the same time:
   - **Review pass 3**: a full fresh review by six reviewers, who only read the documents.
   - **Thin slice, layer 1**: the foundations no review has disputed, built under `next/`: the skeleton,
     manifest, launcher and entries (blueprint packages B0, B1, B2), the shared core and schemas (B3, C1), the
     workflow prelude, build step and harness (B4a, B5), and a sample project.
3. One amend step folds the review findings and anything layer 1 found into the documents, with a fix-and-verify
   loop. The design documents have one writer at a time.
4. **Thin slice, layer 2**, built against the amended design: only as much of state (S1, S2a, S2b, S3), gates
   (G1, G2, G3, G4a in local mode), hooks (H1, H5, H7, B4b), agents (A1, A4, the adversary from A6a) and the
   `story` workflow (W2) as one story needs. Then one story is delivered on the sample project through the real
   platform, headless.
5. Whatever the slice run finds is fixed in the code and in the design documents, then the build widens.

Rule for both methods: every finding gets a recorded outcome, in the documents and in the code where it applies.
Slice findings go to `rewrite/design/slice-findings.md`.

**The close-out fix run (2c) finished on 2026-11-24** (four agents, two rounds, about an hour). Round one fixed
the 21 items and its check found 5 new problems, one major. Round two fixed those and its check was clean of
blockers and majors. Five minor problems remain, each caused by a round-two fix (R1 to R5 in
`rewrite/design/pass2-closeout-3.md`): they concern which run may write a cycle's route, an earlier
specification approval counting in a later cycle, a retry that drops a story, and two undefined phrases.
They go into the next amend step.

**Launched together on 2026-11-24:**
- **Review pass 3**: five Opus reviewers (seams; coverage and platform truth; operations; requirements; a
  builder's cold read of the slice path and build plan). Read-only. They write `rewrite/design/pass3-*.md`.
  Script at `rewrite/workflows/review-pass-3.js`.
- **Thin slice, layer 1**: Sonnet builders and Opus reviewers build the foundations under `next/`:
  unit 1 the skeleton, manifest, launcher and entries (packages B0, B1, B2), unit 2 the shared core and schemas
  (B3, C1), unit 3 the workflow prelude, build step and harness (B4a, B5), unit 4 a sample project. Unit 1
  also replaces the repository root `CLAUDE.md` on this branch with the short rewrite-mode version the
  blueprint specifies. Script at `rewrite/workflows/slice-layer-1.js`.

**Review pass 3 finished on 2026-11-24** (five reviewers, none failed, about 25 minutes). 104 findings: 3
blockers, 51 major, 50 minor; 56 of them lie on the thin-slice path (2 blockers, 27 major). Reports are
`rewrite/design/pass3-{seams,coverage,operations,requirements,builder}.md`. The problems several reviewers
found independently:
- **Local delivery has nowhere to merge.** With no remote, the integration branch is the one checked out in the
  person's own checkout; the design both requires merging into it and forbids touching that checkout (blocker,
  on the slice path).
- **The tool cannot recompute a review verdict.** No record stores which method a review pass belonged to, nothing
  says how passes, disputes and rulings are found among step records, and a scoped re-review overwrites the
  first review's records (blocker, on the slice path).
- **The feature route's delta record has no path or class**, and the specification lock refuses the write (blocker).
- **Brownfield still does not reliably reach a release**: an ingest can measure its baseline before its commands
  are known; the hand-off of a queued request is undefined; the no-tests case branches a fix story before the
  harness exists; a hub cannot ingest its second repository.
- **Resume gaps**: a resumed run loses stories an earlier run admitted; a lock orphaned inside a live session
  is never released.
- **Two enforcement holes**: delivery never checks red evidence, and in local mode git ref commands can move the
  integration branch past the merge guard.
- **The slice plan itself was wrong**: the package list above omits B6, C2 and B8 and the decision package S4
  that H7 needs; the sample project sits inside E1; S3 is too large for one builder. The layer 2 definition
  must be corrected from `pass3-builder.md` before it is built.

Slice layer 1 is still running. Its first unit has replaced the root `CLAUDE.md` with an 18-line rewrite-mode
file, which the assistant read and found sound.

**Proposal awaiting the user's answer (2026-11-24).** The user asked whether the prose should be broken into
smaller files. Measured: the design proper is 983 KB (about 250,000 tokens) in six files of 119 to 217 KB, with
single lines up to 2,094 characters; review reports are another 626 KB. The same thing is described in several
files (`vsdd story deliver` is in all six, 45 times), which is why fixes kept landing in some places and not
others. The assistant recommended restructuring by thing, not by size, before the amend step:
one file per command, record, hook rule, agent, skill and workflow; real JSON schemas; machine-readable tables
(roster, walls, routes, limits, configuration keys); short topic pages that link instead of restating; one brief
per build package naming the files it implements; history (reviews, finding tables, old-system mappings) moved
out; a lint that checks every reference resolves and nothing is defined twice. The original six documents are
kept untouched under `rewrite/history/`, and a script checks that every old section is accounted for.
The assistant also asked for permission to make a local commit of `rewrite/` and `next/` first, because none
of this work is in git yet.

**Next action: when layer 1 finishes, write the slice's design findings to `rewrite/design/slice-findings.md`,
then run one amend step (a fixer and a verifier, at most two rounds) over the 104 pass-3 findings, R1 to R5 and
the slice findings, with the slice-path findings first. Then correct the layer 2 package list and build it.
Nothing is committed.**

## Heads

| Thing | Value |
|---|---|
| Rewrite branch | `rewrite/v1`, created from `develop` @ `150bc36c`; local only, nothing committed or pushed yet |
| Old plugin version | `1.0.0-rc.25` (the rc line ends here; the rewrite ships as `1.0.0`) |
| Old pipeline state | `origin/factory-artifacts` @ `3f9be6d6` (not mounted in this clone; reference only) |
| Old plugin installs on this machine | project-scoped in `~/Dev/{secops-factory,prism,pregolya,flowloom,vantage,hylo,opnrynkr,vitrand}`; not installed for this clone |

## Decisions made by the user (2026-11-23)

1. Full rewrite, shipping as version 1.0. The existing corpus (code, `.factory` artifacts, policies, decision log) is
   historical reference only. No compatibility with the `1.0.0-rc.N` line.
2. Scope: port all of it. Every capability carries over, including market discovery, the design/UX suite,
   multi-repo, service clones (DTU), the observability stack, and maintenance sweeps.
3. Platforms: macOS and Linux only.
4. Models: Sonnet at extra-high effort is the default. Fable at extra-high effort for planning, thinking and
   creative roles. Opus for the adversary, the security reviewer, and all other independent reviewers.
5. Hook runner: Python, launched through `uv`.
6. Orchestration: Claude Code Workflows. The pipeline workflows are redesigned and improved, not transcribed.
7. Low touch: as little human effort as possible to run it and keep it running. An activation step is acceptable
   only if genuinely needed.
8. The rewrite itself is carried out with multi-agent workflows, on a new branch, with the new plugin in a fresh
   directory beside the old one until cutover.
9. The rewrite and the new pipeline must both survive compaction, warm a fresh context, and resume from a cold start.

Assumptions of mine that the user has not confirmed:
- Opus runs at extra-high effort like the other tiers.
- "Port all of it" means capabilities, so duplicate skills merge, and hooks that only police the old bookkeeping
  formats are dropped because those formats are gone.
- The git rules from the old `CLAUDE.md` still hold: no AI attribution in commits, no `--no-verify`,
  no force-push to `main`, commit or push only when asked.

## What the survey found (one-paragraph version)

The old system is a Claude Code plugin ported in April 2026 from an OpenClaw system called dark-factory.
It has 35 agents, 127 skills, 17 `.lobster` YAML workflow files with about 560 steps and no runtime (an LLM reads
them and dispatches by hand), 79 registered hooks behind a Rust/WASM dispatcher (about 195,000 lines of Rust across
55 crates), and 2,279 bats tests. The cross-vendor adversary in the docs (GPT-5.4, Gemini) never existed on
Claude Code. Bookkeeping swamped delivery: 3,489 artifact commits against 883 code commits, and a 231 KB `STATE.md`.
Full detail is in `rewrite/reference/`.

## Verified platform facts

From the workflow authoring reference and the plugin documentation:
- A plugin ships workflows as `workflows/*.js`, invoked as `/<plugin>:<name>`.
- A workflow step can run a named agent type and set `model`, `effort` and worktree isolation per step.
- A workflow can call another saved workflow, one level deep only.
- Workflow scripts cannot read files, call the shell, import modules, or take user input mid-run.
  Limits: 16 concurrent agents, 1,000 agents per run. Resume works only inside the same session.
- Agent frontmatter accepts `model` (`sonnet`, `opus`, `fable`, `haiku`, `inherit`, or a full ID) and
  `effort` (`low` to `max`). Plugin-shipped agents ignore `permissionMode`, `hooks`, `mcpServers`, `initialPrompt`.
- Plugin hooks are commands in `hooks/hooks.json`. Exit 2 blocks; any other non-zero exit is non-blocking.
- Plugins need no activation step.

Measured on the user's Mac (uv 0.12.23):
- `uv run --script` with inline script metadata starts in about 49 ms warm when standard-library only,
  and 65–110 ms with one third-party package.
- It leaves the target project's own `pyproject.toml` untouched.
- `uv` missing exits 127, which Claude Code treats as non-blocking: gates would silently switch off.
- A failed package download exits 2, which Claude Code treats as a block. A PyPI fetch failed in the
  assistant's sandboxed shell with a TLS "UnknownIssuer" error.
- Consequence: keep the runner standard-library only, use `uv` to pin the interpreter, and guard the hook
  command so a missing `uv` fails loudly.

Proven by the platform spike on 2026-11-23 (Claude Code 2.1.331, headless `claude -p --plugin-dir`):
- A workflow step can run a plugin agent by its qualified name (`agentType: 'fspike:builder'`). With no model set
  on the step, the agent file's `model` is used. A `model` set on the step overrides the agent file, and the
  agent still gets its own system prompt. A step with no agent type runs on the session model.
- A plugin workflow can call another plugin workflow by qualified name (`workflow('fspike:story', args)`),
  and the arguments and return value pass through.
- A `uv`-launched Python hook blocks a tool call with exit 2, and the model is shown the runner's reason.
  The shell guard for a missing `uv` exits 2 with a clear message.
- Hook input carries `agent_type` for calls made inside a subagent (`fspike:builder`, `workflow-subagent`;
  empty for the main session). A read wall keyed on it worked: the builder was blocked from a holdout file
  and the reviewer read it.
- A `SessionStart` hook's `additionalContext` reaches the model with `source` = `startup`, `resume` and `compact`.
  `PreCompact` and `PostCompact` both fire. After compaction the model has only a summary plus the fresh briefing.
- Workflows run headless when `Workflow` is in `--allowedTools`. `CLAUDE_PLUGIN_DATA` is set for hooks.

Also observed:
- Hooks fire for the `Workflow` and `StructuredOutput` tools, so the runner must pass unknown tools through.
- Seven trivial agents cost about $1.06, almost all of it fixed per-agent context. Every agent start carries the
  session's loaded plugins and skill listings, so the number of model-visible skills is a cost multiplier.
- `claude plugin validate` wants a README, a licence, an icon, and a `network.hosts` declaration for hooks.

Still not verified:
- Whether a step inherits the agent file's `effort` (there is no way to observe effort from inside a run).
  Set effort on the step, or accept the agent file's value on trust.

Platform probes, round 2 (2026-11-23, against section 10 of the blueprint; plugin in `rewrite/spike/plugin/`):

| Blueprint item | Result |
|---|---|
| 1. Hook on the `StructuredOutput` call | Proven. The hook receives the whole result as `tool_input`, and a block makes the agent correct it and return again. |
| 2. Typed decision command | Proven, headless. `UserPromptExpansion` fires with `command_name` and `command_args`; a hook can add context; a block ends the prompt with zero model turns, so the text never reaches a model. |
| 3. Refused stop | Partly. A refused `SubagentStop` keeps a step running when the step returns text. It does not when the step has already returned a schema result. Use a block on the `StructuredOutput` call instead (item 1), which is proven. |
| 4. Skill starts a workflow | Proven, headless: a skill a person invokes makes the main session start a plugin workflow and relay its result. Left to its own judgment, the main session used a model-visible spike skill when there was one, and dispatched the spike agent directly when there was not. It never started a workflow unprompted. |
| 5. `<plugin>/bin` on `PATH` | Proven in the main session, a dispatched agent and a workflow step, headless. A person typing `! vsdd` was not tested. |
| 6. Long Bash call and detached job | A 150-second call with an explicit timeout ran in a workflow step, and a detached job outlived the call that started it. The full ten minutes was not tested. |
| 7. Hook approval of a tool call | Proven in the default permission mode in the main session, a dispatched agent and a workflow step: the approved command ran and the control was refused. The user's sessions default to the `auto` permission mode, where the control was allowed anyway. |
| 8. Tool lists | Proven. An agent whose file lists the Agent tool still has none as a workflow step. An agent limited to `Bash` still returns a schema result. |
| 11. Agent frontmatter | `isolation: worktree` is honoured when the agent is dispatched with the Agent tool and ignored when it runs as a workflow step, so workflow steps must pass isolation themselves. `omitClaudeMd` is honoured (indirect evidence: about 600 fewer tokens, and the agent no longer listed the global `CLAUDE.md`). A forked skill can name a plugin agent. |
| 14. Search tool and git exclude | Proven. `Grep` skipped a directory listed in `.git/info/exclude`. `Grep` and `Glob` exist in this build. |
| 15. Hook working directory | Proven earlier: the payload's `cwd` is the worktree for an isolated agent. |
| 17. Fixed cost of an agent start | Measured. An agent with an explicit tool list starts with about 2,700 to 3,400 tokens of context. An agent with no tool restriction starts with about 19,000. Explicit tool lists cut the fixed cost by about 85 percent. |
| 9, 10, 12, 13, 16, 18 to 25 | Not probed: they need MCP servers, an interactive session, or the product itself. |

Also seen: a project `CLAUDE.md` in a scratch directory under `/private/tmp` was not loaded by headless sessions,
even in the main session, while the user-level `CLAUDE.md` was. Check this before end-to-end tests depend on it.

## Requirements the blueprint must meet

Pipeline workflows:
- Flatten nesting to one level: each phase is its own workflow; a wave calls story delivery.
- Human stops are workflow boundaries; prune the old 37 approval steps to real decisions.
- Convergence needs a workable rule: severity threshold, cap, escalation path.
- Information walls are enforced by hooks keyed on the calling agent, not by instructions.
- Build and test commands come from project config, not hardcoded `cargo`.
- One per-story playbook, with the adversary loop in it.

Compaction, warm-up and cold start (applies to the new pipeline):
- The main session stays thin: orchestration lives in workflow scripts and in subagents with their own contexts.
- All state is on disk, written at each step boundary. Nothing important lives only in a conversation.
- Every workflow starts by working out what is already done from the artifacts and git, so re-running the same
  command in a new session continues instead of restarting.
- A `SessionStart` hook prints a short briefing (project initialised or not, current phase, what is in flight,
  the exact next command) on startup, on resume, and after compaction.
- A pre-compaction hook flushes anything pending. The old pre- and post-compaction hooks were registered but
  never wired, so that mechanism has never run.
- State files have size caps. The old `STATE.md` grew to 231 KB against a 200-line target.
- One status-and-resume entry point replaces the old wrap, wave-handoff, wave-reset, rehydrate-wave,
  recover-state and compact-state skills.

Spikes (user requirement, 2026-11-23: "include provisions for running spikes as well and allow workflows/agents
to run spikes when they think they need to"; and "make sure vsdd has the ability to run spikes, from agents, to
skills, to workflows, etc. it should be selectable anytime a human or agent thinks a research spike is warranted"):
- A spike is a first-class capability of the new plugin: an isolated, bounded experiment that answers one question.
- It must be reachable from every level: a person, any agent, any skill, any workflow, at any point.
- Agents and workflows may start one on their own judgment, without a human stop.
- The user's existing convention is the starting point: a `spike/<name>` branch, a `.spike/STATE.md` and
  `.spike/TASKS.md` ledger, a `.worktrees/spike-<name>/` worktree, and full isolation from pipeline state
  (see `~/.claude/skills/resume-spike/SKILL.md`).
- Entry points tested on 2026-11-23 with the spike plugin (`rewrite/spike/plugin/`):

  | Who decides a spike is needed | How it starts | Result |
  |---|---|---|
  | A person | Runs the spike skill as a slash command; the skill opens a ledger, dispatches the spike agent, records the finding | works |
  | An agent dispatched in a normal session | Dispatches the spike agent itself with the Agent tool (main session, then agent, then spike agent) | works |
  | An agent running as a workflow step | Returns a spike request in its structured result; the workflow runs the spike agent in an isolated worktree and re-runs the step with the finding | works end to end |
  | An agent running as a workflow step, directly | It has no Agent tool, so it cannot spawn a spike agent. It can invoke a skill, but a skill that dispatches an agent fails at that step | does not work |
  | A top-level workflow | Runs a spike step, or calls a spike workflow | works |
  | A workflow that is already a child | Runs the spike as a step of its own. Calling a third workflow throws "nesting is limited to one level" | works as a step only |

- Worktree isolation held: the spike ran in `.claude/worktrees/<run-id>` on its own branch, the file it wrote
  never appeared in the main checkout, and the worktree was kept because it had changes.
- The hook runner sees every spike start and which agent asked (`PreToolUse` on `Agent` with the spike agent
  type, and `SubagentStart`), so limits can be enforced in code.
- Consequences for the blueprint (assistant's reading, not yet confirmed by the user):
  - One spike protocol with three entrances: slash command for people, direct dispatch for agents in normal
    sessions, structured request for agents inside workflows. The rule for agents: in a workflow, ask; elsewhere, dispatch.
  - Every workflow step's result schema carries an optional spike request, handled by one shared wrapper.
    Scripts cannot import, so the wrapper is stamped into each workflow file at build time or checked by a test.
  - Limits per step and per run keep autonomous spikes bounded; exceeding them escalates to a person.
  - Pipeline gates such as tests-before-code must not block work inside a spike worktree, and nothing from a
    spike reaches the product branch except through the pipeline.
  - A durable spike ledger records question, who asked, method, result and verdict, so findings survive a cold
    start and are not repeated. Kept spike worktrees need a retention rule.

Hook runner:
- One checked-in `hooks/hooks.json`, no per-platform variants, no activation.
- Hooks do nothing in a project that has not been initialised for the factory.
- Standard-library Python; config in TOML; a small frontmatter reader ships inside the plugin.
- Known defects not to carry over: the old dispatcher never wrote to stdout, so `protect-bc` and `protect-vp`
  did not block; the hook timeout of 10000 was read as seconds.

## Plan and status

| Step | What | Status |
|---|---|---|
| 0 | Survey the old system and the platform; agree direction | done |
| 1 | Platform spike: throwaway plugin proving the unverified facts above | done 2026-11-23 |
| 2 | Design workflow: Fable designers per subsystem, Opus reviewers on the seams, one blueprint for approval | done 2026-11-23; blueprint awaiting the user's decisions and approval |
| 2b | Second review pass on the blueprint and the platform probes (blueprint package B8) | done 2026-11-24; close-out found 5 major and 16 minor problems |
| 2c | Close-out fix run for those 21 problems | done 2026-11-24; clean of blockers and majors, 5 minor remain |
| 2d | Review pass 3 (full, read-only), then one amend step | **running**, launched 2026-11-24 |
| 3a | Thin slice, layer 1: foundations under `next/` | **running**, launched 2026-11-24 |
| 3b | Thin slice, layer 2: one story delivered on a sample project | pending the amend step |
| 3 | Build workflows per subsystem: conventions, hook runner, agents, skills, workflows, docs and tests | pending |
| 4 | End-to-end proof on a small sample project | pending |
| 5 | Cutover: replace the old plugin directory, release 1.0.0, decide what the eight existing installs do | pending |

Design subsystems for step 2: pipeline workflows, human stops and spikes; state and artifact model (including
compaction, warm-up and cold-start resume); agent roster and models; skill roster; hook rules and runner;
packaging, release and tests.

## How this record stays current

Update this file at the end of every step and before ending any session: the status table, the next action,
new decisions, and newly verified facts. Workflow results are written to files under `rewrite/` before the
step is marked done, because workflow output and background-agent reports do not survive a session.

## Reference index

| File | Contents |
|---|---|
| `rewrite/reference/old-agents-inventory.md` | All 35 old agents: frontmatter, roles, boilerplate, contradictions |
| `rewrite/reference/old-skills-inventory.md` | All 127 old skills: categories, call graph, duplicates, missing files |
| `rewrite/reference/old-hook-runner-map.md` | Dispatcher flow, all 79 registry entries, every plugin's rule, tests |
| `rewrite/reference/old-workflows-packaging-map.md` | `.lobster` schema, tooling, templates, CI, release, pipeline concepts |
| `rewrite/reference/claude-code-plugin-platform-facts.md` | Plugin components, agent and skill frontmatter, hook events, distribution |
| `rewrite/spike/plugin/` | The spike plugin: two agents, parent and child workflows, `hooks.json`, the `uv` guard script, the Python runner |
| `plugins/vsdd-factory/docs/{VSDD,FACTORY,CONVERGENCE,AGENT-SOUL}.md` | The old methodology docs (still the best statement of intent) |
| `git show origin/factory-artifacts:<path>` | Old specs, stories, ADRs, policies |
