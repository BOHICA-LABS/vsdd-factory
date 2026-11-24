# vsdd-factory 1.0 blueprint

Status: decided, with the second review pass applied. Written 2026-11-23 on branch `rewrite/v1` from five
subsystem designs and three review reports in `rewrite/design/`. Amended 2026-11-24 after a second pass by six
reviewers (`rewrite/design/pass2-*.md`). You adopted every recommendation in section 2 on 2026-11-23. Where a
design document and this blueprint differ, this blueprint wins, and section 4 is the authority on every shared
name. The design documents have been amended to agree with it.

How to read it: section 1 says what 1.0 is. Section 2 records what you decided. Sections 3 to 7 describe the
design. Section 8 accounts for every first-pass review finding and section 11 for every second-pass finding.
Section 9 is the build plan, and section 10 lists what has been tested on the platform and what has not.

---

## 1. Summary

vsdd-factory 1.0 is a Claude Code plugin that takes a product from a brief to a released, verified change with as
little human effort as the work allows. It is rewritten from scratch as plain text: 20 agents, 13 skills,
12 JavaScript workflows, about 75 method files, one `hooks.json`, and one standard-library Python package that is
both the hook runner (17 rules) and a command-line tool named `vsdd` (about 48 subcommands). Nothing is compiled.

Three layers each do one job. Workflow scripts order the work. The `vsdd` tool measures facts (it runs the
project's own build and test commands, inspects git and computes review verdicts) and writes the evidence. Hooks
refuse any action the evidence does not support, and keep each agent inside its information wall.

Project state lives in one directory, `.factory/`, on its own git branch. Code mounts, validates, commits and
pushes it, and no agent keeps books. Every workflow starts by asking the tool what is already done, so running the
same command again continues the work in any session.

A person makes three routine decisions in a product cycle: approve the brief, approve the specification, and
authorise the release. Each is one typed command. Everything else runs unattended, and anything only a person can
settle is queued while other work continues. A review loop ends on one clean pass by a fresh reviewer, at each of
three widening perimeters (story, wave, release), with a cap of three passes, one arbitration, and then a person.
Any person, agent, skill or workflow can start a bounded research spike.

The same pipeline serves existing code. A codebase the factory did not build is taken in once with `ingest`,
which ends with one decision of its own: approving the recovered baseline. After that, and after any release, a
change is one `feature` command. A bug fix or a small change goes straight to a story; a larger change goes
through a specification amendment first (section 3.4).

Two review passes have been applied. In the first, three reviewers found 96 problems in the five designs. All 96
are fixed: 94 in the first pass, and the two that waited on decision D4 now that you have answered it (section
8). In the second, six reviewers checked those fixes, read the design cold, traced it against your requirements
and looked for cuts. They raised 118 findings. The 100 defects are all fixed. Of the 18 proposed cuts, 16 are
applied, one is offered to you and one is declined on its own reviewer's advice (section 11). The cuts removed
one policy file, the stage and route data files, five record types, about a dozen commands and three decision
kinds. No capability was removed. A close-out check of those fixes found 5 major and 16 minor problems, and
a check of that round found 1 major and 4 minor. All 26 are fixed (11.4).

---

## 2. Decisions (adopted 2026-11-23)

On 2026-11-23 you adopted every recommendation in this section, D1 to D30. Each recommendation is now a fixed
decision, and this section is the record of it. In the tables, the "Recommendation" column is what was decided
and the last column records what the other answer would have changed. Sections 3 to 10 are written to the
decisions as adopted; section 2.4 says where each one is built. The second pass reopened none of them. It
found three that the design had not fully built (D4, D5 and D6), and those are now built as section 2.4 says.

### 2.1 Decided before the build starts

These change what is built.

| # | Question | Recommendation | What the answer changes |
|---|---|---|---|
| D1 | Keep project state on its own git branch (`factory-artifacts`), mounted at `.factory/`? The alternative is a tracked directory on the integration branch. | Keep the separate branch. Mounting, committing and pushing move into code, so it costs you one `/vsdd-factory:setup` per clone. | The alternative puts state commits in product history and hidden test scenarios in the product tree, and it forces your checkout to stay on the integration branch. It would change the state design, the hook walls and the importer. |
| D2 | May the plugin commit and push the state branch on its own? State commits skip the product's git hooks and signing. No product branch is touched. | Yes, for the state branch only. `state.push = false` gives a project manual pushes. | "Commit or push only when asked" applied to state would put you back into every step. |
| D3 | Should the eight projects on the old plugin get an importer? | Yes. `vsdd import` is mechanical, runs in place after tagging the old tip `factory-pre-1.0`, and keeps contract, story and decision identifiers. `vsdd init --fresh` gives a clean start where you prefer one. | Without it, build package S5 is dropped and each project starts again from nothing or stays on rc.25. |
| D4 | May the plugin approve its own routine tool calls, so that unattended runs do not stop on permission prompts? Plugin agents cannot set a permission mode, so in the default mode every edit, build, test and commit inside a workflow asks you. | Yes, offered once per project at setup and scoped: edits inside story and spike worktrees and the state directory by the agent whose role may write them; the project's configured build and test commands; `git add`, `commit`, `status`, `diff` and `log` inside those worktrees; and `vsdd` commands the caller's role allows. A call is approved only when every other hook rule passes. | Two findings waited on this answer (COV-04, OPS-12) and are closed now that it is yes. If no: you run pipeline sessions in a permission mode of your choosing, a six-story wave raises dozens to hundreds of prompts, and scheduled runs cannot work. The design is the same either way; only the default of one hook rule changes. |
| D5 | Model policy details. (a) A fourth tier, "clerk" (Sonnet at low effort), for steps that only run one `vsdd` command and relay its output. (b) Opus at extra-high effort for reviewers, which you have not confirmed. (c) Tiers for roles your policy does not name: `codebase-analyzer` and `spike-runner` on the planner tier (Fable), `verifier` on the builder tier (Sonnet). (d) Model aliases (`fable`, `sonnet`, `opus`) instead of pinned model IDs. | Accept all four. Clerk steps make no judgment and are about a third of all agent starts. Let the broad inventory passes of ingestion run on the builder tier as a marked override, because ingestion is the largest planner-tier cost. Re-run the end-to-end proof after any model release. | Each is one line in `tests/lint/roster.toml`, from which agent files and workflow scripts are checked or generated. Refusing (a) makes clerk steps slower and dearer with nothing gained, because they make no judgment. |
| D6 | Is the main session walled like an agent? It could not read hidden scenarios or edit specification files directly; you use an editor or `/vsdd-factory:author`. | Yes, on by default, with a switch in project configuration. | It closes the last unenforced path around the walls. If no, the model you talk to can read scenarios and pass them to builders in prompts. |
| D7 | Is a typed command the only way to settle a decision in a session? Saying "approved" in prose records nothing, and the model is refused if it tries to record a decision for you. | Accept. It is `/vsdd-factory:decide <id> <choice>`, one short line, and the one act a model must not be able to perform. | If no, approvals rest on what a model says you said. |
| D8 | Is `/vsdd-factory:spike` a workflow, with no model-visible spike skill? The two designs each shipped `spike` and the name can only exist once. | Yes, the workflow. Limits and the ledger are in code, it shares one implementation with the wrapper inside every workflow, and it carries named spikes. The main session can start it, or dispatch the spike agent directly. | If you want a model-visible skill, the workflow is renamed `spike-run` and a small skill starts it; that adds one skill listing to every agent start. |
| D9 | Do you accept one clean review pass on the final commit at each perimeter (story, wave, release), with a cap of three passes, in place of three consecutive clean passes? | Accept, with two review lenses on critical and high modules. The end-to-end proof measures how many findings are first caught at a later perimeter; tighten if that rate is high. | The old rule could not terminate. A stricter rule multiplies reviewer cost by about three. |
| D10 | Is one `verify` workflow covering the old phases 4 to 7 acceptable? Your requirement says each phase is its own workflow. | Accept. The four check the same commit, share one fix loop and have no human decision between them. | Splitting them serialises the checks and adds three commands per cycle. |
| D11 | Do the old git rules hold as default gates: no AI attribution in commits or pull requests, no `--no-verify`, no force push, no direct push or commit to protected branches? | Yes, on by default, each relaxable per project by a person. Setup also switches off Claude Code's own attribution trailer so builders are not blocked on their first commit. | `rewrite/STATE.md` listed them as an unconfirmed assumption; adopting this confirmed them. |
| D12 | Which gates may a project relax, and are two human-only escape switches acceptable? | Lock the walls, write scopes, gate-guard, decision capture and step storage. Let a person set any other gate to advise or off by editing the project file. Keep `VSDD_FACTORY_HOOKS=off` (everything off, announced) and add `VSDD_FACTORY_HOOKS_QUARANTINE=<rule>` (one crashing rule off, announced), both environment variables only a person can set. | Without the switches, a bug in the hook runner locks a project until the plugin is uninstalled. |
| D13 | Research and browser servers (Perplexity, Context7, Tavily, Playwright): bundled, your own, or both? | Bundle all four and have agents name both spellings of each tool; leave keys blank on machines that already run the servers. | Bundled servers need a one-time approval per project. |
| D14 | Two skills need tools the old plugin never shipped: a Markdown-to-PDF converter and a diagram renderer. Both need Node (`npx`), which the bundled servers already need. | Keep both as `vsdd export pdf` and `vsdd render diagram`, with Node as an optional prerequisite that `vsdd doctor` checks. Build package T5 starts by proving the engines. | Dropping either skill would have removed T5 or half of it. |

### 2.2 Defaults in project configuration

These are defaults a project can change later.

| # | Question | Recommendation | What the answer changes |
|---|---|---|---|
| D15 | Should a story merge without a person when every gate passes? | Yes: `delivery.merge = "auto"`. Three things always wait for you: a story that touches a protected path (`.github/**`, `.env*`, `CODEOWNERS`, release configuration); a change that adds a dependency or upgrades one beyond what the approved architecture lists; and anything a project configures. | With `.github/**` protected, the story that first sets up CI waits for you once per project. Dependency changes in a scheduled maintenance run are batched into one decision. |
| D16 | Should a person approve the story plan? | No stop by default. Coverage, cycles and size are checked by code. A project can opt in, and a stop opens automatically when the plan's estimate exceeds a configured ceiling. | One fewer stop per cycle. |
| D17 | Must a person authorise every release, including bug-fix patches? | Yes for 1.0. | Automatic patch releases can follow once the rate of escaped defects is known. |
| D18 | May an agent arbiter (the architect) dismiss a high-severity finding that the builder disputed and that is still open at the cap? | Yes, but never a critical or security finding, never a finding the builder did not dispute, and every dismissal is listed in the release packet. | The alternative is a human decision for every finding still open at the cap. |
| D19 | Do all changes to an approved specification found during build need your approval? | Yes, batched once per wave through an amendment run. | Letting agents apply "clarifications" unapproved cannot be policed by code. |
| D20 | Should a market assessment run for every feature request? | Off by default. Run `/vsdd-factory:discover` in assess mode when you want one. | The old flow ran it, with a stop, for every feature. |
| D21 | Are the spike limits right: 2 per step, 6 per run, 20 per project per day, 20 minutes each, 3 named spikes open, 14 days of worktree retention? | Start with these and adjust after the end-to-end proof. | Configuration only. |
| D22 | Is a demo script plus its text transcript enough as committed demo evidence, with screenshots capped at 300 KB and no committed video? | Yes. Rendered media stays in `.factory/local/media/`. | The old repository carries 110 MB of committed recordings. |
| D23 | When a cycle closes, should evaluations and demo files leave the working tree? They stay in the state branch's history. | Yes. `state.prune_closed_cycles = false` keeps them for projects with audit duties. | The tree stays bounded by the size of the specification. |
| D24 | Should hooks be silent in projects that are not initialised? | Silent, except one migration line in a project with pre-1.0 state. | Unrelated projects pay nothing. |

### 2.3 Decided for cutover

| # | Question | Recommendation | What the answer changes |
|---|---|---|---|
| D25 | Collapse to one long-lived branch at cutover? | Yes. Fast-forward `main`, make it the default, delete `develop`. | Only the branch name CI watches. |
| D26 | Pay for end-to-end tests in CI with an `ANTHROPIC_API_KEY` secret? | Yes: the smoke tier on pull requests that touch the plugin, the story tier at each release and weekly. | If no, the tests run locally before each release and the gate is a checklist item. |
| D27 | What does each of the eight projects do? | Run `tools/cutover.py plan`, pilot on `vantage`, migrate all eight within two weeks, add no legacy marketplace entry. | The order and timing of the last cutover step, migrating the projects. |
| D28 | Should this repository run the factory on itself after 1.0? | No for 1.0. Use ordinary pull requests and the CI checks. | The old engine did, and bookkeeping swamped delivery. |
| D29 | Use "Bohica Labs" and `bohica-labs` URLs in the manifest, and submit to Anthropic's directory? | Yes to the names. No submission for 1.0, so the icon can be a placeholder. | Manifest fields. |
| D30 | Delete `docs/demo-evidence` (110 MB) and the 364 KB changelog from the tree at cutover? | Yes. Both remain in history and under the tag `archive/rc-line`. | Repository size. |

### 2.4 Where each decision is built

| Decision | Built in |
|---|---|
| D1, D2 | State on its own branch, committed and pushed by code: 4.7, 5.2. `state.push = false` gives manual pushes. |
| D3 | `vsdd import`, package S5; the importer also writes the baseline and an approved `spec` decision so that `feature` can start (3.4, 4.9). |
| D4 | The `auto-allow` hook rule is the one mechanism (5.4). The person's answer is stored as `[hooks.rules.auto-allow] scope` in `factory.toml`, written by `vsdd init --auto-allow <scope>` from what `setup` asks. The platform probe passed in the default permission mode (section 10, item 7). The scope you adopted is the value `core`. Setup also offers `extended`, described in 5.4, because the second pass showed that `core` alone still raises prompts in an unattended run. |
| D5 | Four tiers and aliases in `tests/lint/roster.toml` (4.2). The builder-tier override for ingestion's inventory passes is `[overrides]` in the same file. The weekly conformance job reruns the story tier when a model alias moves. |
| D6 | The `main` row of the wall table (4.3, 5.4). The switch is `[hooks] main_session_wall` in `factory.toml`, which only a person can edit. It relaxes the `main` row only and is announced in every briefing while it is off. |
| D7 | The `decide` skill and the `decisions` hook rule (4.11, 5.4). |
| D8 | `/vsdd-factory:spike` is a workflow; there is no spike skill (section 6). |
| D9 | The convergence rule (5.1); the proof measures the escape rate (section 10, item 21). |
| D10 | One `verify` workflow (5.1). |
| D11, D12 | Default gates and which may be relaxed (5.4). The walls stay locked for every agent row; the one stated exception is the `main` row under D6. |
| D13 | Four bundled servers, both tool spellings (5.3). |
| D14 | `vsdd export pdf` and `vsdd render diagram`, package T5. |
| D15 to D24 | Keys and defaults in 4.12; decisions `merge`, `plan` and `release` in 4.11; spike limits in section 6. Release configuration files are among the protected paths (D15), and the plan ceiling is `stops.plan_ceiling_agents` (D16). |
| D25 to D30 | Cutover steps, package CO2. |

---

## 3. Architecture at a glance

### 3.1 The parts

| Part | What it is | Its one job |
|---|---|---|
| Workflows | 12 JavaScript scripts in `<plugin>/workflows/`, run as `/vsdd-factory:<name>` | Order the work: sequence, fan out, loop, branch on result fields. A script cannot read files or run commands, so it is never trusted with truth. |
| Agents | 20 Markdown files in `<plugin>/agents/` | Judgment: write a specification, write code, find defects. An agent never reports whether its own work passed. |
| Method files | About 75 Markdown files in `<plugin>/methods/` | The procedure for one kind of task. An agent reads one only when its task names it, so it costs nothing until used. |
| The `vsdd` tool | One standard-library Python package in `<plugin>/py/vsdd/`, on the Bash tool's `PATH` through `<plugin>/bin/vsdd` | Measure and record: run the project's build and test commands, inspect git, compute verdicts, write evidence, merge, and keep project state. |
| The hook runner | The same Python package, started by `<plugin>/hooks/hooks.json` through a guarded shell launcher and `uv` | Enforce: refuse an action the evidence does not support, keep each agent inside its wall, record each step result, print the session briefing. |
| Project state | `.factory/` in each project, on the branch `factory-artifacts` | Hold specifications, stories, decisions and records. Code commits it. |
| Skills | 13 directories in `<plugin>/skills/` | Conversation with a person, one-off specialist jobs, and local tools. No skill re-implements a workflow. |

Terms used below. A **unit** is the thing a workflow works on: a story, a wave (a group of stories built in
parallel), the specification set, a release candidate. A **gate** is a fact the tool measures, such as "the new
tests fail" or "the suite passes on this commit". An **evidence record** is the file a gate writes. A **step** is
one agent run inside a workflow, and a **step record** is the stored copy of what it returned. A **clerk step**
runs one `vsdd` command and relays its JSON output. A **decision** is a question only a person can answer, stored
with a packet a person can read. A **perimeter** is the scope of a review: story, wave or release. The
**integration branch** is the branch stories merge into; its name comes from project configuration. A **cycle**
is one pass of the pipeline over a body of work, from a brief or a change request to a release. A **baseline** is
what a cycle starts from: a release, an ingest or an import, together with the measurements taken at that point.

### 3.2 How the layers check each other

```
person ── types a command ──> main session (thin: holds a few hundred bytes per run)
                                   │ Workflow tool
                                   v
   workflow script ── step ──> agent ── tool calls ──> hook runner ── allow / block ──> files, git
        ^                        │                          │
        │ step result            │ runs `vsdd gate ...`     │ stores the step record
        │                        v                          v
        └────────────── `vsdd` tool: measures, writes evidence, merges, checkpoints ──> .factory/
```

At every point of no return (merge, closing a wave, opening a decision, publishing) the tool checks the evidence
again and ignores what the script believes. A fabricated step result can waste a pass. It cannot merge code.

### 3.3 How one story travels from a request to a merged change

The example is a bug fix in a product that already has an approved specification, which is the common case after
a first release or an ingest.

1. A person types `/vsdd-factory:feature "CSV export drops the last row"`. The workflow's first step is a clerk
   running `vsdd survey feature`, with the request passed as a percent-encoded argument. The tool checks
   preconditions (a baseline exists), opens a run record and the cycle, takes the run lock and returns what is
   already done.
2. The architect analyses the change against the approved specification and finishes with
   `vsdd feature route`. The tool derives the route from the stored analysis and records it in `cycle.json`.
   Here the request is a bug, and an existing contract already says the export contains every row, so the
   route is "bugfix": one fix story, no specification change.
3. The story-writer writes the story file. Its acceptance criteria cite the existing contract clause and include
   a regression test. It finishes with `vsdd gate plan --story S-9.01`, which checks coverage, size and
   traceability in code. When the story passes, the same command admits it to the run: it records the story in
   the run and the cycle, creates the branch `story/S-9.01` and the worktree `.worktrees/S-9.01/`, and returns
   the story unit.
4. The `story` workflow runs as a child with that unit. Every later gate checks that the story belongs to the
   run, so a garbled unit cannot deliver anything.
5. **Tests.** The test-writer writes a failing regression test for each criterion and finishes with
   `vsdd gate red`. The tool runs the project's test command, reads the JUnit report, confirms the new tests fail
   on assertions, and records their identities as frozen. Until that evidence exists, a hook blocks every write
   to product source in the worktree by the implementer.
6. **Implementation.** The implementer makes the tests pass one at a time and finishes with `vsdd gate green`
   (build, full suite, lint, format, frozen tests unchanged since the red record). A hook stops it from editing
   any test.
7. **Review.** A fresh adversary reviews the diff against the story and its contracts, and a security reviewer
   joins when the module is critical or high. Each finding must cite a file, a line range and a quote, which the
   hook checks. If there are blocking findings, the owner of each fixes or disputes it: the implementer for code,
   the test-writer for a weak test (it then records the changed tests with `vsdd gate red --refreeze`). A new
   fresh reviewer checks those claims. One clean pass ends the loop. At the cap the architect arbitrates, and
   after that a person is asked.
8. **Evidence.** If the story has user-visible criteria, the demo-recorder records a script and transcript and
   runs `vsdd gate demo`. Otherwise the tool builds the criterion-to-test table itself.
9. **Delivery.** A clerk runs `vsdd story deliver`. The tool recomputes the review verdict from the stored
   passes, confirms green evidence on the head commit, pushes, opens the pull request, waits for CI, merges, and
   removes the worktree. No agent can merge or push to the integration branch any other way.
10. The workflow's last step is a clerk running `vsdd checkpoint feature`, which closes the units the evidence
    supports, commits and pushes state, releases the lock and returns the next command: `/vsdd-factory:verify`.

A story inside a wave travels the same way from step 4, with the unit made by the wave's survey. The wave then
runs its own gate (the suite on the integration head, an integration review, the hidden scenarios) before it
closes.

### 3.4 Existing code and finished builds

You asked whether the factory can add features to a project it did not build, and to a project whose first build
is finished. Both are first-class routes, and both end in the same `verify` and `release` as a new product.

| Starting point | What runs | Human decisions |
|---|---|---|
| A codebase the factory did not build | `/vsdd-factory:setup`, then `/vsdd-factory:ingest`. Ingest maps and analyses the code; recovers contracts, architecture, conventions, the dependency list and the toolchain; measures a baseline (which tests fail today, lint and format findings, mutation scores, scanner findings); and ends at the decision `spec` on that recovered baseline. | One: approve the baseline |
| Any project with a baseline: after an ingest, an import or a release | `/vsdd-factory:feature "<request>"`. A bug fix, or a small change that needs no contract change, goes straight to one story and then to `verify` in delta scope. A larger change goes through a specification amendment, `plan` and waves. A request typed while a cycle is open is queued and starts when that cycle closes. | The release; and the specification for a larger change |
| A project on the old plugin | `/vsdd-factory:setup` shows the import dry run and the person runs `vsdd import`. The importer writes the baseline and an approved `spec` decision marked as imported, so `feature` can start: the route names it at once when the importer queued a request for an epic in flight, and otherwise the person starts it. | The import command |
| A codebase with no tests, no JUnit report or no CI | The same routes. The first run schedules chore stories in the standing epic `E-0` for the missing test harness and CI before any other story. Delivery treats "this repository has no CI" as a recorded fact (`delivery.ci = "none"`), so a missing check is never a pass by accident. | The first CI story touches `.github/**`, so it waits once for a merge approval |
| Several repositories | `/vsdd-factory:setup --hub` proposes the member list. Work runs from the hub with the `repo` argument. A change that crosses repositories becomes one story per repository, joined by dependencies. | As for one repository |

Inherited debt does not block new work. On a project with a baseline the measured gates compare against it. A
test that already failed, a lint or scanner finding that already existed, and a module the cycle did not touch do
not fail a gate; anything new does. `vsdd report debt` lists what was inherited, and `maintain` pays it down.
Plan coverage and traceability apply to the contracts a cycle added or changed, not to every recovered contract.
Package E4 proves these routes end to end before cutover (section 9.6).

---

## 4. Shared conventions

This section settles every name, path, schema and identifier on which the five designs disagreed. It is
authoritative.

### 4.1 Names and paths

| Thing | Value |
|---|---|
| Plugin name and namespace | `vsdd-factory`. Agents are `vsdd-factory:<name>`; skills and workflows are `/vsdd-factory:<name>`. |
| The command-line tool | `vsdd`. The pipeline design's working name `factory` is retired. |
| Plugin layout | `<plugin>/agents/`, `skills/`, `workflows/`, `methods/`, `templates/`, `schemas/`, `assets/` (`design-system/`, `demo/`, `scaffold/`), `hooks/hooks.json`, `py/` (`launch.sh`, `find-root.sh`, `hook.py`, `cli.py`, `vsdd/`), `bin/vsdd`, `observability/`, `.mcp.json`. There is no `pipeline/` directory: stage structure lives in the workflow scripts and the route table is code (4.14). |
| Python package | `py/vsdd/` with `project.py`, `config.py`, `frontmatter.py`, `events.py`, `schema.py` (a small JSON Schema checker), and the sub-packages `hooks/`, `rules/`, `policy/`, `state/`, `gates/`, `delivery/`, `cli/`. |
| Policy tables | `py/vsdd/policy/`: `artifacts.toml` (artifact kinds, including the story kinds), `classes.toml` (path classes), `walls.toml` (one row per caller, the command groups a row may run, and what each read command reveals). |
| Prelude source and markers | `workflow-lib/prelude.js` at the repository root. Stamped between `// >>> stamped:prelude` and `// <<< stamped:prelude`. |
| Shared agent text | `workflow-lib/agent-shared.md` (the block every agent file carries) and `workflow-lib/severity.md` (the severity table), stamped into agent files by the build. |
| State directory | `.factory/` at the project root, a worktree of the branch `factory-artifacts`. Exactly one per project, and never present inside a story or spike worktree. |
| Local store | `.factory/local/`, untracked: runs, step records, evidence, jobs, the lock, the driver flag, hook records, the event log. |
| Project configuration | `.factory/factory.toml`. Created by `vsdd init`, edited by a person. Code writes it in three stated cases only (4.7). |
| Worktrees | `.worktrees/<story-id>/` for a story, `.worktrees/_integration/` for wave and release gates (`.worktrees/_integration/<repo>/` in a hub), `.worktrees/spike-<name>/` for a named spike. Quick spikes use a platform worktree. |
| Branches | `story/<id>`, `spike/<name>`, and `hotfix/<version>` for a critical fix cut from a release tag; the integration and release branch names come from `[branches]`. |
| Event log | `.factory/local/events/<UTC date>.jsonl`. Only the hook runner and the tool write it. |
| Person's terminal | `~/.local/bin/vsdd`, a shim that `setup` offers to install. It finds the plugin version enabled for the current project in `~/.claude/plugins/installed_plugins.json` (the project entry first, then the user entry) and prints the reinstall command if that copy has no `bin/vsdd`. Every message addressed to a person names the in-session form first (`/vsdd-factory:<skill>`) and the terminal form second. |
| Python and uv | Python 3.11 or later, standard library only, started through `uv` with no downloads and no network on both the hook path and the tool path. Nothing in the plugin installs a Python: when none is found, the launcher's message names the terminal command that does (`uv python install 3.12`). |
| Minimum Claude Code | 2.1.331. |

### 4.2 Model tiers

`tests/lint/roster.toml` is the single source. Agent files must match it (lint), and the build generates the
tier and role tables in the workflow prelude from it. Every workflow step sets both model and effort from the
generated table, because it is unverified that a step inherits the agent file's effort.

| Tier | Model and effort | Used for |
|---|---|---|
| planner | `fable`, `xhigh` | Planning, thinking and creative roles |
| builder | `sonnet`, `xhigh` | Tests, implementation, verification harnesses, demos, documentation, operations |
| reviewer | `opus`, `xhigh` | Every independent reviewer |
| clerk | `sonnet`, `low` | Steps that run one `vsdd` command and relay its output (decision D5) |

One marked override exists, as decision D5 allows. `[overrides]` in the roster lists the steps that run off
their agent's tier: the inventory passes of ingestion, `ingest/map` and `ingest/analyze`, run on the builder
tier. The build generates an `OVERRIDE` table from it, `step()` consults that table by workflow and stage, lint
rejects any override the roster does not list, and a harness test asserts the tier those two stages use.

Workflow phase metadata names no model, so the step's own model and effort are the only setting. The models are
aliases (`fable`, `sonnet`, `opus`). The weekly conformance job records the model ID behind each alias and runs
the story tier again when one changes.

End-to-end tests run on cheap models: when `VSDD_TEST_TIER` is set in the environment, `vsdd survey` returns a
tier override, the step wrapper uses it, and the briefing announces it.

### 4.3 The agent roster

Twenty agents. An agent type exists only where the tier, the tool list, the wall or the isolation differs.

| # | Agent | Tier | Role | Shell | May write |
|---|---|---|---|---|---|
| 1 | `product-owner` | planner | The brief's final text, domain specification, requirements, behavioral contracts, hidden scenarios; recovered contracts after an ingest; intake of existing documents | `vsdd` only | `spec-product`, `holdout` |
| 2 | `architect` | planner | Architecture, decision records, verification properties, assessments, the dependency list, the toolchain, conventions and review rules, change analysis, arbitration | `vsdd` only | `spec-arch` |
| 3 | `ux-designer` | planner | UX specification, flows, design system, design variants | `vsdd` only | `spec-ux` |
| 4 | `story-writer` | planner | Epics and stories with traced acceptance criteria and dependencies. Waves are computed by code. | `vsdd` only | `stories` |
| 5 | `codebase-analyzer` | planner | Reads an existing codebase and records behaviour, architecture and conventions | yes | `analysis` |
| 6 | `researcher` | planner | Outside evidence with citations, market and discovery signals, retrospectives with improvement proposals | no | `research` |
| 7 | `spike-runner` | planner | Runs one spike in an isolated worktree | yes | its spike worktree only |
| 8 | `test-writer` | builder | Failing tests with the stubs they need; regression, equivalence, UI and end-to-end tests; fixes to tests | yes | `tests` (including frozen tests), `manifests` (test dependencies), `source` only before the red gate |
| 9 | `implementer` | builder | Makes failing tests pass; fixes findings; ports; migrations; service clones | yes | `source` only after the red gate, `manifests`; never `tests` |
| 10 | `verifier` | builder | Builds verification harnesses (proofs, fuzzing, mutation, benchmarks, clone fidelity) in harness stories | yes | `harness`, `tests` |
| 11 | `demo-recorder` | builder | Records demo evidence by driving the built product | yes | `demo` |
| 12 | `technical-writer` | builder | Documentation of current behaviour, release notes | yes | `docs` |
| 13 | `devops` | builder | Chore stories: project scaffold, CI, repository and toolchain configuration; the observability stack | yes | `ci`, `repo-config`, `manifests`; in a chore story with method `repo-setup` also new files in `source` and `tests` |
| 14 | `adversary` | reviewer | Attacks specifications, stories and implementations against the specification with fresh context | no | `reviews`, create only, on-demand reviews only |
| 15 | `code-reviewer` | reviewer | Reviews a diff with no access to specifications or factory state | no | `reviews`, create only |
| 16 | `security-reviewer` | reviewer | Security review, triage of scanner output | no | `reviews` and `analysis`, create only |
| 17 | `holdout-evaluator` | reviewer | Scores the running product against hidden scenarios without seeing source, tests or specifications | yes, to start and exercise the product | nothing |
| 18 | `ui-reviewer` | reviewer | Reviews the interface through a browser: accessibility, visual fidelity, responsive behaviour, heuristics | yes | `reviews`, create only |
| 19 | `scenario-reviewer` | reviewer | Reviews hidden scenarios against the contracts. It never reviews code and its output never reaches a builder. | no | nothing |
| 20 | `clerk` | clerk | Runs one `vsdd` command and returns its JSON | `vsdd` only | nothing |

Rules that apply across the roster:

- Builders write project code only inside a story worktree, and only in the classes the table gives. A path
  in the product tree belongs to exactly one class (4.7). A lint rule checks that the implement agent of every
  story kind can write every class its method's output needs.
- A "`vsdd` only" shell means the agent has the Bash tool and the hook lets it run nothing but a single `vsdd`
  call. The four planners use it to finish a step with the check for their own work (`vsdd gate spec --only
  <part>`, `vsdd gate plan --story`, `vsdd feature route`), so a failed check costs another turn of the agent
  that has the context, not a new agent. They may run read and measure commands only. The researcher, which
  reads untrusted pages, has no shell at all.
- Read walls. `holdout` (hidden scenarios, their evaluations, and review findings that quote them) is readable
  only by `product-owner`, `scenario-reviewer` and `holdout-evaluator`. Reviewers cannot read `reviews`. No
  caller can read `runs` (stored step records), the main session included; what a step needs from an earlier
  step arrives in its prompt, and people read findings through `vsdd review show`. `code-reviewer` reads nothing
  in the state directory except `evidence` and `guidance` (the conventions and review rules it is given).
  `holdout-evaluator` cannot read source, tests, CI, stories, or any
  specification except the brief. `scenario-reviewer` cannot read source or tests. Agents with no shell or a
  `vsdd`-only shell, the clerk and the spike-runner cannot read outside the project, the plugin and the
  temporary directory.
- The main session is walled like an agent (decision D6): it cannot read `holdout` or `runs`; inside the state
  directory it writes only `brief` and `deferred`; and it cannot commit on the integration or release branch.
  When it dispatches an agent that can read hidden scenarios, the hook checks that agent's final message for
  scenario text before the message reaches the main session. `[hooks] main_session_wall = false`, which only a
  person can set, relaxes this row alone and is announced in every briefing.
- Every agent file sets `omitClaudeMd: true`. Project conventions reach agents through
  `.factory/specs/architecture/conventions.md` and build commands through `[commands]`.
- Every agent file lists its tools. `clerk` has `Bash, StructuredOutput`. No agent has the Skill tool.
- Every agent may ask for a spike (section 6).

### 4.4 Names the pipeline design used, and what replaces them

The pipeline design was written with provisional agent names. Every workflow uses roster names, and each step
names a method file.

| Pipeline design name | Roster agent | Method |
|---|---|---|
| `business-analyst` (domain stage of `spec`; project context in `ingest` and `multi`) | `product-owner` | `domain-spec` |
| `business-analyst` (synthesis and idea briefs in `discover`; assessment) | `researcher` | `discovery`, `market-intel` |
| `business-analyst` (risks and assumptions sweep) | `researcher` | `risk-sweep` |
| `research-agent` | `researcher` | `research`, `market-intel`, `discovery` |
| `data-engineer` | `implementer` | `data-migration` |
| `devops-engineer`, `dx-engineer` | `devops`, inside chore stories | `ci`, `repo-setup`, `toolchain` |
| `formal-verifier`, `performance-engineer`, `dtu-validator` | `verifier`, inside harness stories; the measuring itself is the `formal`, `perf` and clone-fidelity checks of `vsdd gate release` and `vsdd gate wave`, run by a clerk | `proofs`, `fuzzing`, `mutation`, `performance`, `clone-fidelity` |
| `e2e-tester` (`ui-verify`) | The `ui` check inside `vsdd gate wave` and `vsdd gate release`; the tests are written by `test-writer` | `e2e-tests` |
| `spec-reviewer` | `adversary` for specifications and stories; `scenario-reviewer` for hidden scenarios | `review-specs`, `review-stories`, `review-scenarios` |
| `validate-extraction` | `adversary` | `review-extraction` |
| `accessibility-auditor`, `visual-reviewer` | `ui-reviewer` | `ui-review` (lens named in the task) |
| `session-reviewer` | `researcher` | `retrospective` |
| arbitration step | `architect` | `arbitrate` |
| idea scoring | `product-owner`, `architect`, `adversary` | `score-ideas` |

### 4.5 Skills and workflows

Thirteen skills. Only `status` is visible to the model; a person types the rest.

| Skill | Purpose |
|---|---|
| `status` | Runs `vsdd status [--fix]`: where the project stands, what is wrong with the command that fixes it, the exact next command. Saves a standing instruction with `vsdd note`. |
| `setup` | Makes a project a factory project. It first asks whether the plugin may approve its own routine tool calls (decision D4: `off`, `core` or `extended`) and, for a hub, shows the member list from `vsdd repos scan`. It then runs `vsdd init` with those answers, shows the detected configuration for you to correct, runs `vsdd doctor`, and offers the terminal shim and the attribution setting. With your confirmation it dispatches the architect to move build conventions out of the project's `CLAUDE.md` into `conventions.md`, and moves sections that describe the old process to `docs/archive/CLAUDE-pre-1.0.md`. On pre-1.0 state `init` converts nothing: setup runs `vsdd import --dry-run`, shows the report, and gives you the one command that performs the import, which only a person may run. On a project that is already initialised it shows the line to add for D4 instead of writing it. |
| `brief` | Guided creation of the product brief. It saves the draft after each section, so a compaction or a crash loses nothing. When you say the draft is ready it dispatches the `product-owner` to write the final text, then runs `vsdd checkpoint brief`, which opens the cycle and the `brief` decision. |
| `decide` | The person's decision command. A hook records the decision before any model reads the message; the skill body only relays what the hook returns. If the driver was paused at that decision in this session, what the hook returns tells the main session to resume it. |
| `drive` | The low-touch driver: runs `vsdd driver on`, then starts the next workflow, reads `next` from its result and repeats. At a human decision it runs `vsdd driver pause` and stops; when you settle that decision the driver carries on without being typed again. It turns itself off at done, or after two runs with no progress. |
| `research`, `review`, `author`, `retro`, `defer`, `observe`, `preview`, `export-pdf` | On-demand research; an independent review outside the pipeline; having the owning specialist change one artifact (including `conventions` and `review-rules`, which is how a retrospective's proposal is adopted); a retrospective; the deferred-work register; the observability stack; the design preview server; Markdown to PDF. |

Twelve workflows: `ingest`, `discover`, `spec`, `feature`, `plan`, `wave`, `story`, `verify`, `release`,
`maintain`, `multi`, `spike`. Only `story` is ever called by another workflow, and it calls none. No skill has a
workflow's name. A workflow accepts its arguments as an object or as the text a person typed after the command;
one prelude function turns the text into the object (`feature <request> [intent=...]`,
`spike <question>`, `spike open <name> -- <mission>`, `spike close <name>`).

### 4.6 Story kinds

One enumeration, used by the state registry, the workflows and the hooks. A story may also carry `method`, which
picks another method owned by the same agent (for example `data-migration` or `ui-implementation`).

| Kind | Tests stage | Implement stage | Red gate |
|---|---|---|---|
| `feature` | `test-writer`: `failing-tests` (the method includes the stubs the tests need) | `implementer`: `tdd` | yes |
| `fix` | `test-writer`: `regression-test` | `implementer`: `fix-findings` | yes |
| `port` | `test-writer`: `equivalence-tests`, against the reference implementation | `implementer`: `port-translation` | yes |
| `facade` (service clones and fakes) | none | `implementer`: `service-clone` | no; clone fidelity against the recorded behaviour of the real service is measured at the wave gate |
| `docs` | none | `technical-writer`: `docs` | no; the green gate runs the documentation check (`commands.docs_check`) |
| `chore` (scaffold, test harness, CI, toolchain configuration) | none | `devops`: `repo-setup`, `ci` or `toolchain` | no |
| `harness` (proofs, fuzz targets, benchmarks) | none | `verifier`: `proofs`, `fuzzing`, `mutation`, `performance` or `clone-fidelity` | no |

The kinds, with whether each has a red gate, are one table in `artifacts.toml`; the hook rule that enforces
tests before code reads its exemptions from there.

Rule: every change to the product tree is a story. Steps that only measure are clerk steps and write nothing but
evidence. Plan readiness, verification harnesses and documentation fixes are therefore chore, harness and docs
stories in the standing epic `E-0`, with the same gates as any story.

A story that does not come from the plan (the one story of a fast route, a fix story from a gate, a readiness
chore) is admitted by the tool, not by the script: `vsdd gate plan --story` and `vsdd story new-fix` check the
story, record it in the run and the cycle, create its branch and worktree under the run's lock, and return the
story unit the child `story` workflow needs.

An empty repository. The architect records the toolchain (`[commands]`, the JUnit report, `[paths]`) in
`specs/architecture/toolchain.toml`, which is part of what the `spec` decision approves. The first survey after
that approval writes those values into the keys of `factory.toml` that are still unset, and the briefing
announces it. `vsdd plan schedule` puts the scaffold chore story alone in the first wave, and its green gate is
the first run of those commands. A project never needs a person to edit `factory.toml` between `spec` and
`wave`.

### 4.7 State layout and path classes

The state design's class table (`state.md` 2.4) is the only content of `classes.toml`. The layout, as amended:

```
.factory/
├── factory.toml                     project configuration; a person edits it
├── notes.json                       up to five notes a person asked to keep
├── specs/product/  specs/architecture/  specs/ux/
├── stories/E-<N>/                   epic.md, S-<N>.<NN>.md
├── holdout/                         HS-<NNN>.md; evaluations/<cycle>/<unit>-<n>.json, including review
│                                    findings that quote a scenario (written by code)
├── reviews/                         on-demand review reports only
├── demo/<cycle>/<story>/            scripts, transcripts, small screenshots
├── analysis/<scope>/                brownfield and port analysis; security.md, disposition.md, port.md
├── research/                        reports, retrospectives and idea briefs; inputs/<source>/<date>.* (by code)
├── requests/CR-<NNN>.md             every change request, queued or running (written by code)
├── ideas/  spikes/  deferred/
├── cycles/<cycle>/                  cycle.json, units/*.json, decisions/*
├── cycles/_standing/                units and decisions of work outside a product cycle, including the
│                                    ingest or import baseline and its `spec` decision
└── local/                           untracked: runs/, evidence/, jobs/, lock.json, driver.json, hooks/,
                                     events/, media/
```

Changes from the state design, first pass: notes get their own tracked file; pipeline reviews write no prose
file; `cycles/_standing/` holds decisions and closure records for `ingest`, `discover`, `maintain` and spikes;
`requests/` is new; the research cache is dropped.

Changes in the second pass. There is no status record: the briefing and `vsdd status` are one function that
computes its answer from the records each time, so no copy can be missing or stale. There is no stored verdict
record: a verdict is computed from the step records, and after the checkpoint it is in the closure record. Each
change request is stored once, as `requests/CR-<NNN>.md`, and `cycle.json` names it (or `brief`); the release
checkpoint adds the close date, the version and the state commit before pruning to `cycle.json`, so there is no
separate request copy or summary file. The lessons register is gone: a retrospective writes its proposals into
its own report under `research/`, and adopting one is an edit of `conventions.md` or `review-rules.md`, the two
files every agent is already given. There is no compaction snapshot file.

Status values live with the record that decides them. An agent sets only the statuses its artifact kind lists
(`draft`, `retired`, `dropped`, `stale` and the like). Whether a specification file is approved, a story ready or done, or a property
proven is read from the decision or closure record that made it so, and code never rewrites tracked files to
say it. `vsdd list <kind>` shows the computed status.

Machine records, written only by code. There are 14 kinds.

| Record | Path | Tracked |
|---|---|---|
| Cycle record | `cycles/<cycle>/cycle.json` | yes |
| Closure record, one per unit; a baseline is a closure record | `cycles/<cycle>/units/<workflow>[.<subject>].json` | yes |
| Decision record and its packet | `cycles/<cycle>/decisions/<id>.json`, `<id>.md` | yes |
| Notes | `notes.json` | yes |
| Holdout evaluation, with any walled findings | `holdout/evaluations/<cycle>/<unit>-<n>.json` | yes |
| Run record | `local/runs/<run>/run.json` | no |
| Step record | `local/runs/<run>/steps/...` | no |
| Evidence record | `local/evidence/<subject>/<gate>.<sha12>.json` | no |
| Job record (a detached gate, a delivery, a started product) | `local/jobs/` | no |
| Run lock | `local/lock.json` | no |
| Driver flag | `local/driver.json` | no |
| Caller marker (who made a `vsdd` call) | `local/hooks/calls/` | no |
| Spike admission (who asked for a spike) | `local/hooks/spikes/` | no |
| Configuration fingerprint | `local/hooks/` | no |

The first pass had 19 kinds. The five removed are the status record, the verdict record, the cycle's copy of
its request, the cycle summary and the compaction snapshot; the per-session marker became the per-call caller
marker.

Path classes, which decide who may read and write what:

| Class | Paths | Who writes |
|---|---|---|
| `config` | `.factory/factory.toml` | A person. No agent tool call can write it. Code writes it in three cases: `vsdd init` and `vsdd import` create it; `vsdd repos split` appends member repositories after a `spec` decision; and the survey fills unset `[commands]` and `[paths]` keys from the toolchain file a `spec` decision approved. Each is announced in the next briefing. |
| `control` | `.factory/notes.json`, `.factory/cycles/**` | Code |
| `decisions` | `.factory/cycles/*/decisions/**` | Code: the checkpoint opens, the person's command settles |
| `brief` | `.factory/specs/product/brief.md` | `product-owner`; the main session in the `brief` skill |
| `spec-product`, `spec-arch`, `spec-ux` | `.factory/specs/product/**`, `architecture/**`, `ux/**` | `product-owner`, `architect`, `ux-designer` |
| `guidance` | `conventions.md` and `review-rules.md`, inside `spec-arch` | `architect`. Named apart for their lock, the decision digest (5.2) and the `code-reviewer`'s read wall (4.3) |
| `stories` | `.factory/stories/**` | `story-writer` |
| `holdout` | `.factory/holdout/**` | `product-owner` (scenarios) |
| `holdout-evaluations` | `.factory/holdout/evaluations/**` | Code: scenario scores, and the full form of any finding that quotes or locates a hidden scenario |
| `reviews` | `.factory/reviews/**` | The reviewer of an on-demand review, create only |
| `demo` | `.factory/demo/**` | `demo-recorder` |
| `analysis` | `.factory/analysis/**` | `codebase-analyzer`; `security-reviewer` create only |
| `research` | `.factory/research/**` except `inputs/` | `researcher` |
| `research-inputs` | `.factory/research/inputs/**` | Code (`vsdd discover collect`) |
| `requests`, `ideas`, `spikes` | `.factory/requests/**`, `ideas/**`, `spikes/**` | Code |
| `deferred` | `.factory/deferred/**` | The main session in the `defer` skill |
| `runs`, `evidence`, `local` | under `.factory/local/` | Code |
| `reference` | `.reference/**` | Nobody |
| `tests`, `harness`, `ci`, `repo-config`, `manifests`, `docs`, `source` | from `[paths]` in project configuration | Builders, by role (4.3) |

The seven product classes cover the whole product tree. `manifests` holds build manifests and lock files
(`Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod` and their locks). `repo-config` holds toolchain and
repository configuration (`rust-toolchain.toml`, `.gitignore`, `.editorconfig`, linter configuration).
`harness` holds proofs, fuzz targets and benchmarks (`fuzz/**`, `benches/**`, `proofs/**`, and in-source proof
files such as `**/proofs.rs`, which the verifier could not write as `source`). A path that matches
the globs of more than one class belongs to the first that matches in the order given in the table, so
`src/parser/tests.rs` is in `tests` even though `src/**` is a source glob. The hook rules and the red gate all
use that one rule. `vsdd init` detects the globs, `vsdd doctor` lists tracked files that fall in no class, and a
write to such a file is blocked with the line to add under `[paths]`.

### 4.8 Identifiers

Identifiers are the state design's (`state.md` 2.4): `BC-S.SS.NNN` for a behavioral contract, `VP-NNN` for a
verification property, `ADR-NNN`, `SS-NN` for a subsystem, `E-N` and `S-N.NN` for epics and stories, `AC-N` for an
acceptance criterion, `HS-NNN` for a hidden scenario, `W-NN` for a wave, `SP-NNN` for a spike, `ID-NNN` for an
idea, and `CR-NNN` for a queued change request (new). A path is a function of its identifier, identifiers are
never reused, and nothing stores a reverse link, a count or a version.

A finding's identity is its `key`, `<category>:<path>:<symbol or clause>`, the same in every pass (outside the
holdout wall a walled finding is named by a hash of its key, 4.10). A finding
built from hidden-scenario scores has the key `holdout:<HS id>`, or `holdout:<unit>:mean` or
`holdout:<unit>:spread`. A decision's identifier is its kind (`brief`, `spec`, `release`, `plan`, `ideas`) or
`<kind>.<subject>` (`finding.S-7.04`, `merge.S-7.04`, `blocked.S-7.04`). When two cycles are open at once (a
critical fix beside the main cycle) and both hold a decision with the same identifier, each is written
`<id>@<cycle>`, for example `release@hotfix-1.2.1`. The briefing prints the qualified form, and a bare
identifier that is ambiguous is refused with the list of choices. A step is identified by run, workflow,
subject, name and attempt.

### 4.9 The `vsdd` command

One tool, two owners. The state design owns everything the pipeline calls (`py/vsdd/state/`, `gates/`,
`delivery/`); the pipeline design supplies each gate's pass condition. The packaging design owns the operator
tools (`py/vsdd/cli/`). Every command prints one JSON object with `ok` under `--json`.

| Command | What it does | Who may run it |
|---|---|---|
| `init [--fresh] [--auto-allow <scope>] [--hub]`, `import [--dry-run] [--auto-allow <scope>]` | `init` creates or mounts state and writes `factory.toml` by detection. On pre-1.0 state it converts nothing: it reports `legacy` and prints the dry-run command and the import command. `import` converts pre-1.0 state in place and writes what `feature` needs to start: the baseline, an approved `spec` decision marked as imported, a queued change request for each epic that was in flight, and `factory.toml`. | Main session in `setup` for `init` and `import --dry-run`; `init --fresh` and the real `import` are run by the person |
| `status [--json] [--wave W]`, `list <kind>`, `refs`, `coverage`, `path`, `validate`, `config --json`, `explain`, `rules`, `review show <unit>` | Read-only views; they write nothing. `list`, `refs`, `coverage` and `review show` leave walled content out unless given `--include-walled`, which the hook allows only for a caller whose wall permits the class. A list kind is an artifact kind from `artifacts.toml`; an unknown kind is treated as revealing every walled class. In `validate`, a register over its limit is a warning and never a failure. | Any caller, subject to its wall |
| `status --fix` | Repair what is safe: remount, release a lock whose holder is gone, remove expired worktrees and evidence, clone missing member repositories, commit pending state | Main session; `clerk` |
| `survey <workflow> [--subject] [--repo] [--cycle] [--request-uri] [--guidance-uri] [--question-uri]` | Check preconditions, then open a run, take the lock, make worktrees and diff patches, and return what is done, the limits, the plugin root and `next`. Opens a cycle when a change request starts one; `maintain` opens one when it is about to deliver fixes and none is open. Free text reaches the tool as a percent-encoded argument, because a script cannot write a file and a clerk's command takes no standard input. | `clerk` |
| `checkpoint <workflow> --run <id> [--outcome failed]`; `checkpoint brief` | Close what the evidence supports, open decisions, commit and push state, release the lock, return `next`. `checkpoint brief` takes no run and opens the cycle and the `brief` decision; the checkpoint of `discover` in assess mode does the same through the same code. The release checkpoint closes the cycle. | `clerk`; the main session for `brief` |
| `gate <name> ... [--only <check>] [--wait <seconds>]` with the 11 names `red`, `green`, `demo`, `spec`, `plan`, `wave`, `release`, `ingest`, `contracts`, `integration`, `sweep` | Measure one gate and write its evidence record. `wave`, `release`, `ingest` and `sweep` each run every check that is due as one job (`baseline`, `ui`, `formal`, `perf`, `audit` and the rest are checks, selectable with `--only`). `gate red --refreeze` records changed frozen tests after a test fix. `gate plan --story` admits a story to the run when it passes (4.6). | The agent whose step ends with it; the planners for `gate spec --only <part>` and `gate plan --story`; `clerk` |
| `feature route --run <id>` | Derive the route from the stored change analysis, write it into `cycle.json`, and mark an amendment run | `architect`; `clerk` |
| `plan schedule` | Compute waves, assign scenarios, run the plan gate, record the baseline check, estimate | `clerk` |
| `review prepare <lens> [base]` | For an on-demand review: write the diff patch and return the next free report path | Main session |
| `story deliver --story <id>`, `story new-fix ...` | The only merge onto the integration branch; fix stories from blocking findings, each admitted with its worktree | `clerk` |
| `release --run <id>` | One idempotent command that does the next missing part: prepare (version files, change log), then publish (tag, publish, merge a hotfix back, record the new baseline). The only route onto the release branch. | `clerk` |
| `env start --subject <id>`, `env stop` | Start the product on its own port for a demo, a UI review or scenario evaluation. The process is recorded with the caller, and is stopped when that agent stops or at the checkpoint. | `demo-recorder`, `ui-reviewer`, `holdout-evaluator`, `clerk` |
| `spike open`, `spike record [--increment]`, `spike close --name` | The spike ledger (section 6) | `spike-runner` |
| `idea register`, `report debt`, `discover collect`, `repos split` | Idea registry; derived debt report; pull configured feedback sources; create member repositories | `clerk` |
| `new <kind>`, `note`, `defer add/list`, `retire <path>`, `driver on/pause/off`, `lock release` | Skeletons, notes, the deferred-work register, deleting a file in a class that allows deletion (`deferred`, `research`), the driver flag, the lock | Main session |
| `decide <id> <choice> [key=value ...] [-- note]`, `config set`, `cycle close`, `lock release --force` | Acts only a person may perform | The person: through the typed command for `decide`, otherwise in their own terminal |
| `doctor [--install-shim]`, `events ...`, `obs ...`, `repos scan`, `export pdf`, `render diagram`, `hooks selftest`, `--version` | Operator tools | Main session |

Every command also takes the global option `-C <dir>`, which names the project directory, so a clerk's command
stays one segment with no `cd` in front.

Removed in the first pass: `vsdd pr-merge` and `vsdd worktree` (library functions inside `story deliver` and
`survey`, so there is one merge route), `vsdd cache`, `vsdd record-test`, and `vsdd stale --update`. Removed in
the second pass: `job wait` and `job status` (a long command re-attaches, below); `review verdict` (a function,
not a command); `stale` (4.14); `flush` and `lock status` (state is committed at the points in 5.2, and
`status` shows the lock); `hooks canary` and exit code 3; `events emit`; `step record` (the platform probe it
was a fallback for has passed); `lesson retire`. `release prepare` and `release publish` are one `release`;
`review diff` and `review path` are one `review prepare`; `defer resolve` and `research retire` are one
`retire`. The tool has about 48 subcommands, down from about 60, and 11 gate names, down from 15.

The hook runner enforces the last column from the `[callers]` table in `walls.toml` (which groups of
subcommands each row may run, with `status --fix` and `doctor` granted apart from the read group) and treats a
read command as a read of the classes `[reveals]` gives for it. Calling the Python package directly instead of
`bin/vsdd` is blocked. The commands only a person may run, the two governed merge commands and the story kinds
exempt from tests before code are constants in code.

How the tool knows who is calling. A `vsdd` process cannot see the hook payload, so before the hook lets a
`vsdd` call through it writes a caller marker (session, agent id, agent type, working directory) and rewrites
the command to carry that marker's one-time key in the environment. The tool reads its own marker and nothing
else. This is how the survey knows the hooks are alive for this session, how the lock records the right
session when two are open, how `spike open` knows which runner is calling, and how `vsdd doctor`, run through
the Bash tool, confirms the hooks are active. A caller cannot forge it: an agent's own `VSDD_*` assignment is
blocked, and markers are in a class only code writes. Section 10, item 27 tests the rewrite; the fallback keys
the marker by the Claude Code process and the caller's working directory.

Outside an initialised project every command exits 2 except `init`, `import`, `doctor`, `obs`,
`hooks selftest` and `--version`. Exit codes: 0 success or "yes"; 1 a negative answer; 2 a usage or environment
error.

Long commands. The Bash tool stops a foreground command after two minutes by default and ten at most. Every gate
that builds or tests, `story deliver` and `release` therefore run their work as a detached job, keyed by the
command, its subject and the commit, and the command itself re-attaches: the first call starts the job, and
every call waits for it and returns either the result or `{state: "pending"}`. The caller runs the same command
again until the result arrives. There is no job id to carry. By default a call waits 100 seconds, which is
inside the Bash default; the closing command in a step prompt adds `--wait 540` and tells the agent to set a
Bash timeout of 600000. A call killed by a timeout loses nothing, because the job is detached. One clerk covers
a whole CI wait. `limits.parallel_gates` caps how many jobs run at once.

### 4.10 Result schemas

The files in `<plugin>/schemas/` are the single source. The build generates the schema literals in the prelude
from them, and the tool and hook runner load them. The pipeline design's section 2.10 defines them, with these
amendments:

| Schema | Amendment |
|---|---|
| Step result | `data` is required (it may be empty). The script never reads a field the schema does not require. `blocked.kind` adds `reviewer_blocked`. The hook stores the calling agent's type with every step record. |
| Finding | Adds `owner`, the roster agent that should fix it (the list includes `demo-recorder`), and the category `coverage`. `location.path` is relative to the root of the reviewed tree. The tool adds `grounded` when it stores the finding. |
| Finding built from scenario scores | Built by the tool, never by an agent: key `holdout:<HS id>` or `holdout:<unit>:mean` or `:spread`; category `holdout`; scope `integration` or `system`; `spec_ref` set to the contracts the scenario cites; `repro` set to the scenario id and the rotation seed; `title` a fixed sentence built from the key and the score, never the scenario's own title; no location. The convergence rule exempts category `holdout` from the location and grounding conditions, so a must-pass scenario under its floor, or a mean under the threshold, blocks. |
| Walled finding | A finding returned by the `scenario-reviewer`, or one located under `.factory/holdout/`. Its full form is stored only in the `holdout-evaluations` class. Its key holds the scenario's path, so outside that class it is named by its walled key, `walled:` and the first 12 hex characters of the SHA-256 of the key. Decision packets, closure records and `vsdd review show` carry the walled key and the severity and nothing else, unless the reader may read `holdout`; a waiver names the walled key. |
| Review data | `coverage` items have the state `checked`, `not_checkable` (with a reason) or `skipped`. |
| Claim (given to the next reviewer) | Structured only: `key`, the location's path and line range, `resolution` (`fixed` or `disputed`), `commit`. No quote and none of the fixer's free text is passed on. A claim about a `holdout` finding carries the key alone. |
| Holdout data | `scores` (`scenario`, `satisfaction`, `must_pass`, `ran`) and `symptoms` (observed and expected behaviour, no location, no scenario text). |
| Verdict | Adds the reason `reviewer_blocked` and `rounds_used`, which persists across runs. A review pass counts toward a verdict only when the stored agent type is the reviewer that owns the pass's method. |
| Gate output | Either the gate result, or `{ok, state: "pending"}` while the job runs (4.9). The green gate result and delivery data carry `diff_patch`, the path of the latest patch, which the next reviewer is given. A passing `gate plan --story` result carries `unit`, the admitted story unit. The story-writer runs that command once for each story it wrote and returns the units as `data.units`, which its step's data schema requires, so the script may pass them to the child stories. Every story's entry must pass with its unit present: a story that fails is retried, and the step is blocked if it still fails. |
| Workflow result and `vsdd status --json` | `next` is an object `{kind, command, args}` in both. The briefing renders it as text of at most 200 characters. For the second of two open cycles, `args` carries `cycle`. |
| Spike finding and ledger | One verdict list: `confirmed`, `refuted`, `answered`, `inconclusive`, `refused`. The asker's agent type in the ledger comes from a record the hook wrote, never from the runner's own argument (section 6). |
| Intake data | `covers`: the stage names whose output the intake filled, which the checkpoint records as done. |

Severity has one definition and one text. The text is `workflow-lib/severity.md`, which holds the table of the
pipeline design's section 2.7; the build stamps it into every reviewer file, so lint compares bytes against one
source. The table:

| Severity | Definition |
|---|---|
| `critical` | The change breaks a stated contract in normal use, loses or corrupts data, opens an exploitable security hole, or breaks the build or the suite. |
| `high` | A stated contract clause, acceptance criterion or listed edge case is unmet or untested, or a test cannot fail. |
| `medium` | A real defect with a concrete consequence that no contract covers. |
| `low` | Style, naming, wording, preference. |

### 4.11 Decisions

Grammar: `/vsdd-factory:decide <id> <choice> [key=value ...] [-- note]`. Every value is named, a key may
repeat, and free text goes after `--`. There are no positional values and no quoted strings, so the parser built
to this grammar accepts every documented choice.

Nine kinds.

| Kind | Opened when | Choices |
|---|---|---|
| `brief` | The brief skill ends, or `discover` in assess mode ends | `approve`, `revise -- <guidance>`, `drop` |
| `spec` | `spec` ends; `feature` on the standard route or as an amendment ends; `ingest` ends (the recovered baseline); `multi` for a project-level specification | `approve [variant=<id>] [topology=single or split]`, `revise [stage=<name>] -- <guidance>`, `drop` |
| `release` | `verify` ends | `approve`, `hold -- <guidance>` |
| `ideas` | `discover` in scan mode ends | `select pursue=<ids> research=<ids> defer=<ids> reject=<ids>`, one command for the whole list |
| `merge` | A delivered story waits for a person: protected path, dependency change, or `delivery.merge = "human"` | `approve`: the next `story deliver` performs the merge, and a dependency it names is recorded as approved. `reject`: the pull request is closed and the story is descoped. |
| `plan` | Configured, or the estimate exceeds `stops.plan_ceiling_agents` | `approve`, `revise -- <guidance>` |
| `finding` | A review loop reached its cap with an upheld blocking finding or an uncovered item, or a critical or security finding | `waive key=<key> [key=<key> ...] -- <reason>`, `waive-coverage item=<item> [item=<item> ...] -- <reason>`, `direct -- <guidance>` (one more round), `descope` |
| `spec-gap` | A reviewer showed the approved specification is silent or wrong | `amend`, `not-a-gap -- <reason>`, `descope` |
| `blocked` | A unit cannot continue for one of four reasons, which the packet and the briefing line state: `spike_limit` (a step needs more spikes than allowed), `environment` (a credential, access right or tool is missing), `stuck` (the unit failed in two runs), `reviewer` (a reviewer refused or could not review; opened the first time) | `retry`, `direct -- <guidance>`, `descope`; also `open-spike name=<name>` for `spike_limit`, and `skip` for an optional environment check |

`approve` maps to the status `approved`, `revise` and `hold` to `revise`, `drop`, `reject` and `descope` to
`rejected`; the state design holds the full table. An approval is bound to a digest of exactly what was approved:
the specification file set for `spec` (without the brief, which has its own decision, and the two guidance
files, which have their own lock; 5.2), the commit for `release`. Approval changes no file. The specification is
locked from that moment because the latest `spec` decision is approved, and it stays locked until a run holds an
amendment (a `feature` run on the standard route or with `amend`, or a `spec` run that starts while an earlier
approval stands); an accepted decision record stays frozen even then. If the content changes any other way the
decision reads as stale and the stop returns. Settling in a session writes the decision record (and, for
`spec-gap amend`, the small amendment request file) and nothing else: no file is promoted and git is not run,
so it always fits inside the hook's time limit. The next survey commits it.

Reopening work is explicit. `revise` and `hold` mark stages as not done in the unit's closure record, from the
stage named by `stage=` or, by default, from the review stage, whose fix loop then works on the guidance.

Exceptional decisions are queued. A story that waits on a decision is set aside (it joins a trailing wave when
it is released), its wave gates and closes without it, and later waves start while holding back any story that
depends on it. There is one exception: a wave whose own gate is waiting on a decision blocks the waves after
it, because nothing more should be built on an integration head that has failed its gate. The route and the
survey use one function for "can this unit run", so they cannot disagree.

A specification gap found during a wave. Choosing `amend` on a `spec-gap` decision records an amendment request
against the open cycle. When the open wave has nothing else that can run, the route returns `feature` as an
amendment: it runs inside the open cycle without being queued, takes every `spec-gap` settled `amend` as its
request (so one run covers the wave's gaps, decision D19), and ends at the decision `spec`. After that approval
`plan` runs in delta scope to re-trace the affected stories, and the waiting stories are released.

A second cycle. A critical bug fix may open a second cycle beside the main one. Its stories are cut from the
release tag and delivered to `hotfix/<version>`; its `next` carries `cycle`, which `verify` and `release` accept;
and when it is released the merge back into the integration branch waits for the integration lock, opening a
`merge` decision if the merge conflicts.

### 4.12 Limits and configuration keys

The survey returns limits under the prelude's names. A test fails on any unknown limit key.

| Prelude name | `factory.toml` key | Default |
|---|---|---|
| `agents` | `limits.agents_per_run` | 900 (the platform stops at 1,000) |
| `story_agents` | `limits.agents_per_story` | 40; each child story receives this as its own budget |
| `stories_per_run` | `limits.stories_per_wave_run` | 24 |
| `attempts` | `limits.step_attempts` | 2 |
| `passes`, `rounds` | `convergence.passes`, `convergence.rounds` | 3, 3 |
| `delivery_rounds` | `limits.delivery_rounds` | 3 |
| `gate_minutes`, `ci_minutes` | `limits.gate_minutes`, `delivery.ci_wait_minutes` | 60, 30 (total waits, taken in slices of nine minutes) |
| `spikes_per_step`, `spikes_per_run`, `spike_minutes` | `spikes.per_step`, `per_run`, `minutes` | 2, 6, 20 |

Budgets across a parent and its children. One prelude function, `runStories()`, starts child stories for `wave`,
`verify`, `feature`, `maintain` and `multi`. Before it starts a batch it checks that the agents already used,
plus `story_agents` for each child, plus a reserve for the close, fit under the run's limit; if they do not, the
run returns `partial`. Each child receives `limits.agents = story_agents`, and a child that throws or returns
nothing is charged its full budget. A child also receives its share of the spikes the run has left (at least
one) and the count used so far, so it refuses a spike itself before starting an agent; `vsdd spike open` remains
the backstop.

The full key table is `state.md` section 2.12; it is the one list, and an unknown key is an error. Keys added or
changed by the reviews:

| Key | Default | Meaning |
|---|---|---|
| `commands.test_report` | detected | `junit:<path>`. The red gate needs per-test results. |
| `commands.docs_check` | empty | The documentation check the green gate runs for a `docs` story. |
| `commands.run` | empty | How to start the product. It must take its port from `{port}` or `$VSDD_PORT`; `vsdd init` and `vsdd doctor` check that. |
| `[commands.env]` | empty | Environment for every configured command, for example a shared build cache. |
| `paths.manifests`, `paths.repo_config`, `paths.harness` | detected | The three product classes added to `source`, `tests`, `docs` and `ci` (4.7). |
| `limits.parallel_gates` | 4 | Jobs that run at once. |
| `holdout.batch` | 12 | Scenarios per evaluator step. |
| `delivery.dependency_changes` | `human` | A new dependency, or an upgrade beyond `dependencies.toml`, needs a `merge` decision. |
| `delivery.protected_paths` | `.github/**`, `.env*`, `CODEOWNERS`, and the files `[release]` names | Paths whose stories always wait for a person. |
| `delivery.mode`, `delivery.ci` | detected | `pr` or `local` (no remote); `required` or `none` (the repository has no CI). With `none`, "no checks" counts as a pass and a CI chore story is scheduled. |
| `stops.plan`, `stops.plan_ceiling_agents` | `auto`, 600 | When the `plan` decision opens: always, or when the estimate passes the ceiling. |
| `[release]` | detected | `version_files`, `changelog`, `tag_format`, `publish`, `github_release`. A `[[repo]]` entry may carry its own `[release]` and `[delivery]`. |
| `[[discover.source]]` | none | `name` and `command` for feedback and analytics. |
| `[hooks] main_session_wall` | `true` | Decision D6. |
| `[hooks.rules.auto-allow] scope` | set at setup | Decision D4: `off`, `core` or `extended` (5.4). |

Missing commands are a state, not an error. When `test` or `test_report` is not set, `vsdd status` says
"commands not configured", workflows that would measure refuse to start, and the route says what supplies them:
the `spec` or `ingest` approval for a project that has none yet (4.6), or the line to add.

Dropped: `limits.ci_waits`, `spikes.max_per_session`, `spikes.max_per_requester`. `spikes.max_open` (3) and
`spikes.per_day` (20) are enforced by `vsdd spike open`.

### 4.13 Evidence, the lock and liveness

- **Evidence lookup.** `red`: the latest passing red record (or re-freeze record) on any commit between the
  story's base and its head; it carries the frozen test identities. `green` and `demo`: the record for the head
  commit, or for a commit with the same `git patch-id` after a clean rebase. Review verdict: computed by the
  tool from the stored passes of any run for that unit, so a verdict outlives the run that produced it; after
  the checkpoint it is read from the closure record.
- **The baseline.** A project that began with an ingest or an import has a baseline record: the tests that
  failed then, lint, format and scanner findings, mutation scores per module, and the recovered contract set.
  The green, wave and release gates fail only on what is new against it, and apply mutation and coverage
  thresholds to modules the cycle touched. An inherited test that fails and then passes when the gate runs
  the suite once more does not fail the gate; the gate result lists it in `flaky`, and the list is copied
  from the evidence into the unit's closure record, where `vsdd report debt` reads it. A cycle with no
  release, ingest or import baseline is a first release, and `verify` runs it in full scope; every other
  cycle runs in delta scope.
- **The run lock.** One lock per repository and branch a run can write, in `.factory/local/lock.json`, taken with
  a file lock around read, check and write. The survey takes it only after preconditions pass. Every return after
  a successful survey goes through the checkpoint, which releases it, including on failure. The `spike`
  workflow takes no run lock, because a spike writes only its own worktree and the ledger; `discover` and a
  report-only `maintain` take none either.
- **Liveness.** The hook runner marks the session as seen (at most once a minute) on every guarded tool call from
  the session that holds the lock, or from a working directory inside a worktree that run owns, and releases
  the lock when that session ends. The lock is stale after 20 minutes without a mark.
- **Hooks must be alive.** The survey accepts only the caller marker written for its own call (4.9). With two
  sessions open, one session's marker cannot stand in for the other's, so a workflow never runs with the gates
  switched off.
- **A story branch on the remote.** At delivery the tool pushes the story branch and opens the pull request with
  a trailer naming the story. A remote story branch with such a pull request can be adopted by any clone, which
  is what lets a fresh clone or a crashed session continue a delivery. A remote story branch without one is
  owned elsewhere: the unit is reported as blocked with the reason `conflict`.

### 4.14 Single sources and generated copies

| Fact | Single source | Generated or checked copies |
|---|---|---|
| Agents, skills, workflows, tiers, overrides, budgets | `tests/lint/roster.toml` | Agent frontmatter (lint); `TIER`, `ROLE` and `OVERRIDE` in the prelude (build) |
| Result schemas | `<plugin>/schemas/*.json` | Schema literals in the prelude (build); loaded by the tool and the hook runner |
| Hook wiring | `py/vsdd/hooks/registry.py` | `hooks/hooks.json` (build, checked in) |
| Shared workflow code, including `runStories()` and the argument parser | `workflow-lib/prelude.js` | The stamped region of every workflow (build) |
| Stage structure | The workflow scripts | None. The tool keeps no list of stages: the survey returns which stage names are recorded as done, and the script skips those. A harness test checks that every step names a method that its agent's file lists. The method of every step is given in `pipeline.md` section 2.9.13. |
| Routes and workflow preconditions | An ordered list of (condition, next) and a list of named preconditions per workflow, in Python beside their condition functions | None; scripts receive `next`. Each row and each precondition has a fixture. |
| Artifact kinds and story kinds, path classes, walls with callers and reveals | The three policy tables | Read by the hook rules and `vsdd validate` |
| Convergence rule | `judge()` and `converge()` in the prelude | The tool's Python copy, held to it by shared fixtures run in Node and Python |
| The agents' shared block; the severity table | `workflow-lib/agent-shared.md`; `workflow-lib/severity.md` | Every agent file (build, checked byte for byte by lint) |

`tools/build.py --check` fails the build when any generated copy differs.

When a stage is done. A stage is done when the unit's closure record says so (or, inside a run that has not yet
reached its checkpoint, a done step record exists), the artifacts its result listed still exist and validate,
and the gate it must pass has passing evidence for the current commit. Nothing is re-run because a digest of
inputs changed: stages reopen only when a person's `revise` or `hold`, or an amendment, says so (4.11). After
approval a contract changes only through an amendment, whose change analysis names the affected stories.

---

## 5. The subsystems

### 5.1 Pipeline workflows, human stops and convergence

Design document: `rewrite/design/pipeline.md`.

Final design. Twelve workflows share one shape: a survey step, the stages that are not yet done, and a
checkpoint step. A route is not a script: the tool computes the next command from what is on disk, the briefing
prints it, every workflow returns it, and the `drive` skill follows it. Human stops
are workflow boundaries. The convergence rule is one function used at every perimeter: a finding blocks only when
it is critical or high, cites a contract clause or a reproduction, lies within the perimeter's reach, and is
grounded in a real file and quote.

Amendments made here:

- Roster names and a method on every step (4.4). `step()` takes a required `method`, and the prompt begins
  `Method: <name>. Read <plugin root>/methods/<name>.md first.` The plugin root comes from the survey.
- Long gates and deliveries run as jobs (4.9). The delivery loop no longer spends one clerk per CI poll.
- Every script wraps its body in `try`/`finally` so the checkpoint always runs. `relay()` retries once.
- The survey checks preconditions before it takes the lock and refuses when the hook marker is missing.
- Reviewer prompts carry structured claims only. Wave reviewers receive deferred findings as scope to check.
- Coverage gaps become findings of category `coverage`, routed to the test-writer, so a pass with an unchecked
  criterion can be fixed or waived instead of running to the cap.
- An arbiter may dismiss only a finding the builder disputed. The choice `direct` grants one more round, not a
  new cap. Gate rounds are stored in the wave's closure record and counted across runs.
- The route table gains two rows: "every runnable unit waits on a decision" sits above "a wave is open", so the
  driver stops instead of looping; and a queued change request starts `feature` when the cycle closes.
- Delivery rebases only when the pull request conflicts. A verdict and green evidence follow the `git patch-id`,
  so a clean rebase keeps them and needs only a build check. The wave gate runs the full suite on the
  integration head, and breakage found there is fixed forward as stories.
- In `feature` the tool, not the script, derives the route, and the plan gate runs after the story-writer. An
  amendment run unlocks the whole specification set for the planner roles, and the next `spec` decision
  approves the amended set.
- `spec` gains a `sources` argument and an `intake` stage for existing documents, and a `scenario-review` step.
  `ingest` with `as: reference` gains `disposition` and `port-analysis` stages. `discover` and the post-release
  sweep gain a `collect` stage. `release` prepares and publishes through one mechanical command.
- Multi-repository projects use the hub model of the state design: one state directory, stories that carry
  `repo`, one cycle.
- Child stories get their own agent budget, and the wave keeps a reserve for its gate and its checkpoint.

Amendments made by the second pass:

- The route table is scoped to the open cycle. Every row except setup, the lock and the multi-repository row
  applies only while a cycle is open, and "no cycle is open and a change request is queued" sits directly below
  the lock rows. A brownfield project with a queued request therefore gets `feature`, a released greenfield
  project gets "done" and not `verify` again, and a standing decision does not hide a queued request. The
  `plan` row applies only to a cycle that began with a brief or is on the standard route, or after an approved
  amendment, so a delivered fast route and a maintenance cycle with delivered fixes get `verify`. The route is
  read from `cycle.json`, a tracked record: a cycle opened with a brief is `standard` from the start, and
  `vsdd feature route` writes it for a change request, so a fresh clone routes the same way. New rows:
  the brief is in draft; commands are not configured; an ingest baseline waits for approval; an amendment is
  requested; the second cycle has work that can run.
- Workflow preconditions are a named list per workflow, evaluated by the same code as the route (4.14).
  "Specification approved" means the latest `spec` decision, from `spec`, `feature`, `ingest` or an import, is
  approved and its digest is current.
- Stories written after the survey are admitted by the tool (4.6), which fixes every fast route and every fix
  round. Fast routes also write the readiness chore stories a project needs, and a change that crosses
  repositories becomes one story per repository.
- Measured checks are combined. `vsdd gate wave` runs the UI check itself. In `verify` one clerk runs the
  baseline check and one runs every other due measured check as a single job beside the reviewers. `maintain`
  runs its measured sweeps as one `vsdd gate sweep` job. `plan schedule` records the baseline in the same job.
  This removes about nine clerk starts from a greenfield cycle.
- The four planners finish their own steps with the check for their work (4.3), which removes two clerk starts
  from every fast-route feature and the separate fix-round starts of the specification and plan checks.
- The scenario review is its own loop. Its findings and claims go only to the `product-owner` and never enter
  the shared specification loop, a decision packet in readable form, or any prompt of an agent that may not
  read hidden scenarios (4.10).
- A weak-test finding can be fixed: the test-writer changes the frozen test and records it with
  `vsdd gate red --refreeze`, which only the test-writer may run and which requires the changed tests to exist
  and to pass or fail on an assertion. When a strengthened test fails, the implementer makes it pass in the
  same fix pass.
- `ingest` ends at the decision `spec` on the recovered baseline, records the measured baseline, and writes
  `conventions.md`, `review-rules.md`, `dependencies.toml` and the toolchain file. Its inventory stages run on
  the builder tier.
- `maintain` delivers fixes inside a cycle: it opens a maintenance cycle when it is about to deliver and none
  is open, so the fixes pass `verify` and a release decision like any other change.
- `runStories()` is one stamped function with the budget check of 4.12, shared by the five workflows that call
  `story`.
- The script skeletons in `pipeline.md` section 2.11 are corrected in place, and the corrected behaviours are
  conformance fixtures (5.5).

### 5.2 State, artifacts and resume

Design document: `rewrite/design/state.md`.

Final design. One state directory on its own branch, mounted, validated, committed and pushed by code. Every
artifact kind has one path, an identifier, flat frontmatter, one writer and a size cap checked before the write,
all as data in one registry. Each fact is stored once and every index, count and reverse link is computed on
demand. Progress lives in records only code writes: closure records per unit and decision records with
packets. The briefing and the status report are computed from those records each time. One entry point,
`/vsdd-factory:status`, reports and repairs.

Amendments made by the first pass that still hold:

- The state design owns the whole pipeline-facing tool: survey, checkpoint, gates, jobs, verdicts, delivery
  and the registers (4.9).
- `factory.toml` cannot be written by any agent tool call. A change to any section that bears on a gate is
  announced in the next briefing.
- References between artifacts are checked for syntax at the write and for existence at the gates and at
  approval, so a contract can name a subsystem that the next stage will write.
- Evidence lookup is defined per gate (4.13).
- Notes are a tracked file of their own. `cycles/_standing/`, `requests/`, `research/inputs/` and
  `analysis/<scope>/security.md` are added. Pipeline reviews write no prose report; `vsdd review show` renders
  findings for people. Evaluations are JSON written by code.
- Register caps are reported as status problems and never fail a gate.
- The lock is taken atomically, its scope is the repository and branch, and liveness belongs to the hook runner.
- Story kinds follow 4.6. The architecture gains `dependencies.toml`, the list of dependencies a `spec` decision
  approves.
- The red gate reads a JUnit report and freezes tests by test identity.

Amendments made by the second pass:

- Status values are read from the record that decides them (4.7). Code no longer promotes files to `approved`,
  `ready`, `done`, `proven`, `accepted` or `active`, so the registry column for code-only values, the hook check
  on it, the digest's special case for the status line, and the promotion with its repair path are all gone.
  The specification lock is one rule over the whole set: locked while the latest `spec` decision is approved
  and no live run holds an amendment. That also locks the files that never carried a status (the domain files,
  the architecture overview, `modules.toml`, `dependencies.toml`, the UX files). The two guidance files,
  `conventions.md` and `review-rules.md` (the class `guidance`), are locked with the set while a cycle is open
  and writable by the architect between cycles, which is when a retrospective's proposals are adopted. They
  are outside the `spec` decision's digest, as is the brief, so adopting one, or drafting the next brief, does
  not make the approval stale.
- The status record and the verdict record are removed (4.7). The five project conditions stay (not a factory
  project, pre-1.0 state, ready, partial, state not mounted), and "partial" now has one cause, a broken mount.
- The stage registry, the per-stage input digests, the `built_against` field and `vsdd stale` are removed
  (4.14). The decision digest stays.
- State is committed at five points: the start of a survey (anything pending, such as a settled decision), the
  checkpoint (which also pushes), `story deliver`, `status --fix`, and before compaction. Plain `vsdd status`
  writes nothing.
- Path classes: the product tree has seven classes with one precedence rule (4.7). A test in a file that both
  a source glob and a tests glob match is a test, for the write scopes, the tests-first rule and the red gate
  alike.
- A baseline record holds what an ingest or an import measured, and the gates compare against it (4.13).
- `vsdd init` never converts pre-1.0 state; only the person's `vsdd import` does. The importer also writes the
  baseline, an approved `spec` decision marked as imported, queued requests and `factory.toml`.
- Cycles are opened by `checkpoint brief`, by the `discover` assess checkpoint, by the `feature` survey and by
  `maintain` when it delivers fixes. Each change request is stored once.
- Approved dependencies: a `merge` decision that approved a dependency records it, and dependency detection
  reads those records until the next `spec` decision folds them into `dependencies.toml`.
- Deletion is one command, `vsdd retire <path>`, which deletes only in the classes that allow it. Superseded
  research reports named in a step result are retired by the checkpoint.
- Multi-repository projects: wave closure records, gate rounds and integration worktrees are kept per
  repository, and a wave closes when every repository's part has closed. A `[[repo]]` entry may carry its own
  release and delivery settings. `vsdd status --fix` clones a missing member repository.
- The sandbox needs of the tool are listed in `packaging.md` section 2.10 and each is tested by `vsdd doctor`.

### 5.3 Agents, skills and models

Design document: `rewrite/design/agents-skills.md`.

Final design. Twenty short agent files (at most 60 lines of body), each with an explicit tool list, a shared
block checked byte for byte, and pointers to method files. Thirteen skills, one visible to the model. Walls and
write scopes are tables keyed on the calling agent's type.

Amendments made here:

- Two agents are added: `clerk` and `scenario-reviewer` (4.3). The lint allows low effort for the clerk tier
  only.
- The spike skill is dropped in favour of the workflow; `decide` and `drive` are added (4.5).
- `devops` no longer merges, opens pull requests or makes worktrees; the tool does. The story-writer no longer
  plans waves.
- Reviewer files carry the pipeline's severity table, the finding key rule and the schema's field names. They
  accept structured claims from an earlier pass and nothing else from it.
- Pipeline reviewers return findings only. A report file is written only by the on-demand `review` skill.
- Every agent omits the project `CLAUDE.md`.
- Every agent may request a spike. For `code-reviewer` and `holdout-evaluator` the spike runner inherits the
  asker's walls (section 6).
- Methods added: `arbitrate`, `score-ideas`, `review-scenarios`, `risk-sweep`, `intake`. Methods removed:
  `wave-plan`, and the `pull-request`, `worktrees` and `gates` methods of `devops`.
- The skill mapping is re-keyed to the twelve workflow names.

Amendments made by the second pass:

- The four planners get a `vsdd`-only shell (4.3). The researcher stays without one.
- Write scopes follow the seven product classes (4.3, 4.7), so the scaffold story, a manifest edit and a
  harness story can all be written.
- The "given, writes, returns" table of every agent is rewritten in the schema's field names. No pipeline
  reviewer is given a report path, the implementer is given findings inline, `devops` returns nothing about
  pull requests or merges, the story-writer writes no wave plan, and the holdout-evaluator writes nothing.
- Methods added: `brief` and `recover-contracts` (product-owner); `verification-assessment` and `conventions`
  (architect); `equivalence-tests` and `test-fixes` (test-writer); `pattern-sweep` (code-reviewer);
  `docs-drift` (technical-writer). `stubs` is merged into `failing-tests`. Every step of every workflow has a
  named method (`pipeline.md` 2.9.13), including the three maintenance sweeps that had none.
- The scenario-freshness sweep is split along the wall: the holdout-evaluator re-runs the scenarios, and the
  scenario-reviewer, which may read contracts, judges staleness and coverage.
- The lessons register is removed. The retrospective method writes proposals into its report, and
  `/vsdd-factory:author conventions` or `author review-rules` (both the architect's) adopts one.
- `setup`, `brief`, `decide` and `drive` behave as 4.5 says: setup asks the D4 question before `init`; brief
  saves its draft and has the product-owner write the final text; the driver pauses at a decision and resumes
  when it is settled.
- The severity table and the shared block are stamped from one source each, so the example agent file and the
  lint cannot disagree.
- The spike runner is denied reading outside the project, like the agents without a shell. No walled reviewer
  has a spike run "on its behalf" by the main session: an agent dispatched in a normal session asks by
  dispatching the spike runner itself, and the hook records who asked (section 6).

### 5.4 Hook runner and rules

Design document: `rewrite/design/hooks.md`.

Final design. One standard-library Python program behind a guarded shell launcher, wired by one generated
`hooks.json`. Seventeen rules. In a project with no factory configuration the launcher exits before Python
starts. Every launch failure and every crash of a gate rule blocks and says why.

Amendments made here:

- The `hooks.json` command guards itself, and stays inert where it should. If the launcher file cannot be read,
  the command walks up from the project directory looking for `.factory/factory.toml`, in a few lines of shell
  that mirror `find-root.sh`. Only if it finds one does it exit 2 with a reinstall message; in a project that
  is not initialised it exits 0, so a broken install can neither switch the gates off in a factory project nor
  block tool calls in an unrelated one. In briefing mode it prints the "gates are down" briefing.
- The matcher also covers the Playwright navigation and upload tools, `WebFetch`, and the research tools. A
  `file:` address is treated as a read of that path, so browser tools cannot read around a wall.
- `read-walls` treats a `vsdd` read command as a read of the classes it prints. `gate-guard` enforces which
  `vsdd` subcommands each role may run, blocks direct calls to the Python package, blocks settings writes that
  set `VSDD_*` variables or disable the plugin, and blocks `claude plugin disable`.
- `artifacts` checks reference existence only at gates, and treats a specification file as frozen by the
  whole-set rule of 5.2. `write-scopes` blocks state writes while the mount is broken and any `.factory` path
  inside a linked worktree.
- `tests-first` uses the per-gate evidence lookup. `merge-guard` also blocks commits on a checked-out
  integration or release branch.
- `step-results` stores a record only for a live run and stores the calling agent's type with it. The tool
  counts a review pass only when that type is the reviewer that owns the pass's method, so a builder cannot
  forge a review, and no table of step names has to be generated. It blocks an ungrounded review once, then
  stores the ungrounded findings as advisory. Paths resolve against the unit's worktree with three lines of
  drift allowed. It stores walled findings in the holdout class (4.10).
- `secrets` covers `.mcp.json`, home credential files, key files, `gh auth token`, `git credential fill`, bare
  `env` and `printenv`. Web tool calls are logged, and a URL or query that contains a known secret value or token
  format is blocked.
- `spike-bounds` keys a spike by agent id in every check, lets the runner read plugin files before it opens its
  ledger entry, and treats the ledger's worktree as the runner's own.
- `lifecycle-log` keeps the lock alive and releases it at session end, and stops product processes an agent
  left running. `compaction` flushes state and writes different instructions for a step agent.
- The `decisions` rule parses the grammar of 4.11 and writes the decision record only.

Amendments made by the second pass:

- `auto-allow` is the one mechanism for decision D4; static permission rules are only the fallback of section
  10. The switch is `[hooks.rules.auto-allow] scope`, written by `vsdd init --auto-allow` from the answer `setup`
  collects. A call is approved only when every other rule passes. `core` is what you adopted, made to work
  unattended: edits inside story and spike worktrees and the state directory by the agent whose row may write
  them; the project's configured commands, matched as a prefix so that an added argument (one test by name)
  still passes; `git add`, `commit`, `status`, `diff` and `log` inside those worktrees; `vsdd` commands the
  caller's row grants, which covers the driver's own calls; and a start of one of this plugin's workflows by
  the main session. `extended` adds what a real run also does: a fixed list of read-only commands (`ls`, `cat`,
  `head`, `wc`, `rg`, `find` without `-exec` or `-delete`, and the reading git commands) inside the caller's own
  worktree; calls to the loopback port of a product that `vsdd env start` started, for the roles that may start
  it; the spike runner's shell commands inside its own spike worktree; and the web and research tools of the
  roles that list them. Setup explains both and recommends `extended` for unattended runs.
- Caller markers replace the per-session marker (4.9).
- The policy tables are three. `[callers]` and `[reveals]` move into `walls.toml`; `[reveals]` is generated
  from the class of each artifact kind; person-only commands, the governed merge commands and the exempt story
  kinds are constants.
- The `PostToolUse` wiring is removed, which leaves seven wired events. The tool writes `pr.created` and
  `pr.merged`, and the gates write `commit.made` for the commits they measure.
- `compaction` (renamed from `compaction-snapshot`) keeps the flush and the instruction to the summariser and
  writes no snapshot file. The briefing after a compaction is computed from the lock and the run record.
- `tests-first` and `write-scopes` use the class precedence of 4.7.
- `secrets`: loopback uploads are allowed. `curl` and `wget` upload flags stay blocked for every host except a
  loopback port that `vsdd env start` recorded for the current run, so the evaluator and the demo-recorder can
  exercise `POST` and `PUT` endpoints. Web addresses in shell commands are logged and checked for secret values
  as web tool calls are, and `export -p` and `declare -x` join the blocked environment dumps.
- Spike walls rest on hook-written facts: `vsdd spike open` takes the asker's type from the stored step record
  of the asking step, or from the admission the hook recorded when the spike runner was dispatched, and
  refuses when neither exists (section 6).
- A step that returns a schema result cannot be held by refusing its stop (section 10, item 3), so "no stop
  without a record" is enforced on the `StructuredOutput` call for workflow steps and on the stop for agents
  dispatched in a session.
- The configuration fingerprint also covers `[[component]]`, `[stops]`, `[release]`, `[spikes]` and `[caps]`,
  and the briefing announces a change to any gate-bearing section.
- `status --fix` and `doctor` are no longer in the read group every agent holds: the main session and the clerk
  may run the first, the main session the second.
- The bundled browser server's entry carries the option that refuses `file:` addresses, and lint checks it.

### 5.5 Packaging, tests, release and cutover

Design document: `rewrite/design/packaging.md`.

Final design. The repository is one plugin directory plus tests, tools and documents. One build step stamps
generated content, and a check fails on drift. Tests come in layers: pytest for Python, a Node harness that runs
workflow scripts with stubbed agents, lint over every shipped file, conformance fixtures, and headless
end-to-end runs. A release is an immutable tag pinned by the marketplace entry. The eight existing projects move
with an idempotent cutover tool.

Amendments made here:

- `tools/build.py` also generates the schema literals, the tier and role tables, and `hooks.json`.
- A conformance layer runs shared fixtures through the convergence rule in Node and in Python.
- The layout gains `methods/`, `schemas/`, `assets/`, `py/vsdd/hooks/`, `policy/`, `gates/` and `delivery/`,
  with lint for method files, templates against the registry, the shared block and method existence.
- The command table follows 4.9. `vsdd doctor --install-shim` installs the terminal shim. `vsdd export pdf` and
  `vsdd render diagram` are added.
- The harness models the agent counter that parent and child share, and gains universal tests: every step names
  an existing method; a run in which every agent returns an empty result still reaches its checkpoint; no
  reviewer prompt contains a builder's free text.
- End-to-end tests add kills in the middle of a step and of a CI wait, an eleven-minute gate, and a run in the
  default permission mode that counts prompts.
- The documentation gains a tested recipe for running `maintain` and `discover` on a schedule.
- The build plan is section 9 of this blueprint.

Amendments made by the second pass:

- The build also stamps the shared agent block and the severity table into agent files, and generates the
  `OVERRIDE` table. It no longer generates a step-name table for the hook, and there is no `pipeline/`
  directory to ship.
- Conformance fixtures are written for the behaviours the first pass changed, so the build cannot freeze the
  old ones: an arbiter dismisses only a finding the builder disputed; a coverage gap calls the fixer; the cap
  counts rounds used across runs; a reviewer that returns blocked gives `reviewer_blocked`; a `holdout` finding
  with no location blocks; children receive `story_agents`; a thrown child is charged its budget; findings are
  carried across attempts; every spike is isolated.
- Universal harness tests added: no prompt to an agent that may not read `holdout` contains text from a walled
  finding; every free-text value in a clerk's command passes through the percent-encoder; every workflow
  accepts its documented argument text.
- Fixture tests added to the Python layer: approve, survey, plan (the approval is still current); assess, brief
  approved, the `spec` survey sees an open cycle; each route row, including brownfield with a queued request,
  greenfield after release, and a wave gate at its cap; the decision grammar; import, then one fast-route
  feature.
- The sample project writes a JUnit report from its `unittest` tests through a small standard-library runner
  that ships with the sample, so the red gate works in the end-to-end tiers with no extra package.
- The proof is two packages. E3 starts from an empty repository and adds a trivial feature after its release.
  E4 proves the brownfield routes: an ingest of a project with tests and one known lint failure, then a
  feature and a bug fix through to release; the same project with no tests and no CI; an import cut from a
  real project, then a feature; and a two-repository hub with a change that crosses both. Cutover waits for
  both.
- The story budget in the story tier is 12 agent starts until package E2 has measured a real figure.
- `vsdd doctor` checks each thing a sandboxed Bash tool must allow: writing `.factory/`, `.worktrees/`, the
  `uv` cache and `~/.cache/vsdd-factory/`, and reaching the git remote and `api.github.com`. It no longer has a
  `--fix` flag; a missing Python is fixed by the terminal command the launcher names.
- The cutover report points each project at the setup offer to archive old process sections of its
  `CLAUDE.md`.

---

## 6. Research spikes

A spike is an isolated, bounded experiment or piece of research that answers one question. There is one protocol
and one agent, `vsdd-factory:spike-runner`.

1. **Ask.** One question, why it blocks, and optionally a method and a starting commit.
2. **Open.** The runner's first command is `vsdd spike open`. The tool establishes who asked from a record the
   hook wrote (below) and refuses if it cannot. It returns an earlier finding for the same question in the same
   unit (matched by a hash of the normalised question) only when the earlier runner worked under walls at least
   as strict as this asker's. It refuses when a limit is reached. Otherwise it writes a ledger entry.
3. **Experiment.** The runner works only inside its own worktree. Tests-before-code does not apply there. It
   cannot push, merge, dispatch agents or write anywhere else.
4. **Record.** The runner's last command is `vsdd spike record`. A hook refuses to let it finish without one:
   for a workflow step it blocks the result call, and for an agent dispatched in a session it refuses the stop.
5. **Return.** The finding goes back to whoever asked. Nothing from a spike reaches the product except by being
   rebuilt through the story playbook.

Who asked is a fact the hook records, never something the runner says. Inside a workflow the asking step's
`needs_spike` result is already stored with the asking agent's type, and `vsdd spike open --run <id> --step
<step>` reads the question and the asker from that record. For a dispatch with the Agent tool, the hook records
an admission (the dispatching agent's type) when it sees the dispatch, attaches it to the runner when the
runner starts, and `vsdd spike open` reads it. An `--asker-type` argument is a cross-check only: a mismatch is
refused. Every agent except the spike runner and the clerk has the Agent tool, limited by the hook to the
spike runner, so no walled reviewer needs anyone to run a spike on its behalf.

| Who decides a spike is needed | How it starts |
|---|---|
| A person | Types `/vsdd-factory:spike <question>`, which runs the `spike` workflow. `spike open <name> -- <mission>` opens or advances a named, human-led spike on the branch `spike/<name>` with `.spike/STATE.md` and `.spike/TASKS.md`; `spike close <name>` records its final finding. |
| The main session, on its own judgment | Dispatches the spike agent with the Agent tool. In the platform probe the model never started a workflow unprompted, and did dispatch the agent, so this is the entrance it uses. |
| An agent dispatched in a normal session | Dispatches `vsdd-factory:spike-runner` with the Agent tool. The agent file sets worktree isolation, which the platform honours for a dispatch. |
| A skill | A skill that instructs the main session starts the workflow. A skill that runs in a forked agent dispatches the spike agent like any agent. |
| An agent running as a workflow step | Returns `needs_spike` with a spike request. It has no Agent tool. The wrapper stamped into every workflow runs the spike as a step of the same workflow and starts the asking step again with the finding. |
| A workflow | Calls the same `runSpike()` function as a step. This works in a parent and in a child, because the spike is never a third workflow. |

The test an agent applies is "do I have the Agent tool": if yes, dispatch; if no, return the request.

What bounds a spike:

| Bound | Default | Enforced by |
|---|---|---|
| Spikes per step, per run | 2, 6 | The wrapper, which refuses before it starts an agent; the survey reports how many the run has used |
| Spikes per project per day; named spikes open | 20; 3 | `vsdd spike open` |
| Time box | 20 minutes | The hook runner: after it, every call is refused except `vsdd spike record` |
| Nesting | none | The spike runner has no Agent tool, and its finding schema has no spike request |
| Containment | its worktree only | Write scopes and git rules in the hook runner |
| Walls | `holdout`, `reviews`, `runs` and everything outside the project, plus the asker's walls | The ledger entry records the asker's agent type from the hook's record and the set of classes the runner was denied; the hook applies the union. This is what lets `code-reviewer`, `holdout-evaluator` and `scenario-reviewer` ask safely. |
| Retention | 14 days, or until the asking unit is delivered | `vsdd checkpoint` and `vsdd status --fix` |

Every spike is isolated in a worktree, including research spikes, so each runner has its own working directory
and its own ledger entry, matched by agent id. A workflow step passes the isolation itself, because the
platform ignores the agent file's setting for a step. Findings are carried across retries of the asking step. A
child story receives its share of the run's remaining spikes, so it refuses locally before it starts an agent.
A step that still needs an experiment at the limit becomes a `blocked` decision with the reason `spike_limit`,
and the person can open a named spike, give direction, or descope.

Named spikes. The runner opens a named spike itself: `vsdd spike open --name <name>` creates the branch, the
worktree and the two ledger files, or attaches this runner to the entry that is already open; attaching does
not count against the limit of open named spikes and is never answered from an earlier finding. A named spike
spans many sessions, so each session's work ends with `vsdd spike record --increment`, which the stop check
accepts, and `vsdd spike close --name` records the final finding.

When a spike can run. The `spike` workflow takes no run lock, so a spike can start while a wave is running.
There is one limit, stated plainly: a spike needs an initialised project. Hooks are silent in a project that
has not been set up (decision D24), so nothing could bound a spike there, and the `spike` workflow answers with
"run `/vsdd-factory:setup` first". Named spikes use the same branch, worktree and `.spike/` files as your own
`resume-spike` convention, which keeps working in any repository.

---

## 7. Compaction, warm-up and cold start

The design principle is that nothing important lives only in a conversation. The main session holds a few
hundred bytes per workflow run; orchestration lives in scripts and in agents with their own contexts; and the
tool writes state at every step boundary.

| Case | What is on disk | What happens |
|---|---|---|
| A session starts in an initialised project (warm-up) | Tracked state and the local store | The `SessionStart` hook prints a briefing of at most 1,200 characters: the project and cycle, the position, decisions waiting on you with the command to settle each, what is running, the exact next command, open named spikes, notes, and any relaxed gate or configuration change. |
| The main session compacts while a workflow runs | Everything. The `PreCompact` hook commits pending state. | The workflow keeps running. The briefing after compaction is computed from the lock and the run record and says which run is live. If the driver was active or paused in this session, the briefing tells the main session to continue it and how. |
| A step agent compacts | Its worktree, its commits, earlier step records | The hook gives the summariser instructions written for a step: keep the step identity, the worktree, the story, the closing gate command and the findings word for word. The step prompt also repeats the identity and closing command at its end. |
| A session ends normally in the middle of a wave | Step records and evidence up to the last finished step; story branches and worktrees; any detached gate or delivery job, which keeps running | The hook releases the lock at session end. The next session's briefing gives the same next command. Its survey skips what is done, continues each story from its position, tells the next agent when a worktree holds uncommitted work, adopts an open pull request, re-attaches to a job that is still running or has finished, and recomputes a converged verdict from the stored passes without reviewing again. |
| A session crashes | The same | The lock goes stale after 20 minutes without activity, or the person runs `/vsdd-factory:status --fix` in a new session (or `vsdd lock release` in a terminal). Then as above. |
| A new session, days later (cold start) | Tracked state and the local store | The briefing gives the position and the next command. Typing it, or `/vsdd-factory:drive`, continues. |
| A fresh clone on another machine | Tracked state only | `/vsdd-factory:setup` mounts the state branch. The survey resumes from the last checkpoint, measures gates again where evidence is missing, and adopts a pushed story branch whose pull request the tool opened (4.13). |
| The mount is broken | Decision and closure records; the state branch | The briefing says so, state writes are blocked, and `/vsdd-factory:status --fix` remounts. There is no status record to lose: the briefing is computed. |
| `uv` or Python is missing | n/a | Guarded tool calls are blocked with the terminal commands that fix it. The briefing says the gates are down. |

One entry point, `/vsdd-factory:status`, replaces the old wrap, wave-handoff, wave-reset, rehydrate-wave,
recover-state and compact-state skills. To stop, stop: there is nothing to save by hand.

The rewrite itself follows the same rule. `rewrite/STATE.md` is the resume record, workflow results are written
under `rewrite/` before a step is marked done, and each build package in section 9 is a unit that can be re-run.

---

## 8. Review findings and their resolution

This table is the record of the first pass. "Fixed" names the change. Where a reviewer's proposed resolution was
replaced by another, the row says so. The second pass then changed how some of these are met; section 11 is the
record of that, and where a row here and sections 3 to 7 differ, sections 3 to 7 hold. The rows the second pass
replaced a mechanism in are SEAM-01 (no `step record` fallback), SEAM-09 (one `release` command, not `release
publish`), SEAM-10 and SEAM-40 (no `gates.toml`, `stages.toml` or `routes.toml`), SEAM-17 ("partial" means
only a broken mount), SEAM-21 (more cycle openers), SEAM-22, OPS-07 and OPS-14 (a caller marker per call, not
a per-session marker), SEAM-24 (the driver also has `pause`), SEAM-26 (no canary, no
`stale`), SEAM-30 (one `retire` command, no lessons register), SEAM-31 (`review prepare`), SEAM-44 and OPS-02
(no status promotion), COV-03 and OPS-01 (re-attaching commands, no `job wait`), COV-16 (the guard stays inert),
OPS-06 (no verdict record), OPS-15 (the agent type is checked when a pass is counted, and `events emit` is
gone), OPS-22 (no `doctor --fix`), and OPS-29 (no status record).

| ID | Severity | What was done |
|---|---|---|
| SEAM-01 | blocker | Fixed. The state design owns the pipeline-facing tool (4.9); `record-test` is dropped; `step record` is specified as the fallback of section 10; build packages G1 to G7 precede the workflows. |
| SEAM-02 | blocker | Fixed. `clerk` is an agent with `Bash, StructuredOutput`, a fourth tier, a lint exception for low effort and a wall row with `tool_only`. The tier is decision D5. |
| SEAM-03 | blocker | Fixed. Workflows use roster names (4.4); the role table is generated from the roster, so a name outside it cannot be written. |
| SEAM-04 | blocker | Fixed. `step()` requires a method; the survey returns the plugin root, the review-rules path and the diff patch; methods `arbitrate`, `score-ideas` and `review-scenarios` are added; pipeline reviewers need no report path. |
| SEAM-05 | blocker | Fixed. Evidence lookup is defined per gate (4.13), with hook cases for edits after a commit. |
| SEAM-06 | blocker | Fixed. The workflow is kept and the skill dropped; one model-visible skill remains. Decision D8. |
| SEAM-07 | blocker | Fixed. The hub model is adopted; the pipeline and hook designs are rewritten to it. |
| SEAM-08 | major | Fixed. No tool call writes `factory.toml`; `vsdd config set` is person-only; changes to gate-bearing sections are announced. |
| SEAM-09 | major | Fixed. Only `story deliver` and `release publish` merge; `pr-merge` and `worktree` become library functions; `devops` loses its merge, pull-request and worktree methods. |
| SEAM-10 | major | Fixed. `[reveals]` and `[callers]` in `gates.toml`, enforced by the hook runner. |
| SEAM-11 | major | Fixed. Browser navigation and upload tools are matched; `file:` addresses are reads; the lint knows these tools are inspected. |
| SEAM-12 | major | Fixed. The `scenario-reviewer` agent is added. |
| SEAM-13 | major | Fixed. Claims are structured data without the fixer's text; the exception is stated in the reviewer files; deferred findings are passed as scope. |
| SEAM-14 | major | Fixed. The pipeline schemas are the contract; `owner` is added; the severity table and key rule are copied into reviewer files; field names follow the schema. |
| SEAM-15 | major | Fixed. The roster is the single source and the prelude tables are generated. The spike runner has one tier. Decision D5. |
| SEAM-16 | major | Fixed. The build generates schema literals and `hooks.json`; a conformance layer is added. |
| SEAM-17 | major | Fixed. The hook design answers the state design's needs H-2 to H-5 with cases and adopts its five conditions; flushing on agent stop is dropped. |
| SEAM-18 | major | Fixed. Every change to the product tree is a story (4.6); measuring steps are clerk gates; the ingest audit and gap analysis get writable homes; idea briefs go under `research/`. |
| SEAM-19 | major | Fixed. For the spike row, the worktree in the open ledger entry counts as its own. |
| SEAM-20 | major | Fixed. One verdict list; the ledger is written only through `vsdd`; the runner returns the finding schema; every spike is isolated; plugin files are readable before `spike open`. |
| SEAM-21 | major | Fixed. `cycles/_standing/` holds work outside a cycle; the brief checkpoint and the feature survey open a cycle; the release checkpoint closes it. |
| SEAM-22 | major | Fixed. Lock after preconditions; checkpoint on every return; the survey requires a fresh hook marker. |
| SEAM-23 | major | Fixed. The `decide` skill is added; the brief skill ends with `vsdd checkpoint brief`. |
| SEAM-24 | major | Fixed. The `drive` skill and `vsdd driver on/off` are added; the briefing carries the continue instruction. |
| SEAM-25 | major | Fixed. The terminal shim, a probe of `! vsdd`, and messages that name a form that works. |
| SEAM-26 | major | Fixed. The exempt subcommands are listed; `stale` has no `--update`; `status` takes `--fix`, `--json` and `--wave`; exit 3 is the canary's alone. |
| SEAM-27 | major | Fixed. The plan gate after the story-writer is a clerk step. |
| SEAM-28 | major | Fixed. One enumeration (4.6); `data` is dropped. |
| SEAM-29 | major | Fixed. `requests/CR-NNN.md`, listed in the briefing, with a route row. |
| SEAM-30 | major | Fixed. `vsdd defer resolve`, `lesson retire` and `research retire`. |
| SEAM-31 | major | Fixed. The tool writes the diff patch at the survey and after each fix; `vsdd review diff` serves the review skill. |
| SEAM-32 | major | Fixed. `vsdd export pdf` and `vsdd render diagram`, with Node optional. Decision D14. |
| SEAM-33 | major | Fixed. Finding paths are relative to the reviewed tree; the hook resolves the tree from the run record. |
| SEAM-34 | minor | Fixed. `vsdd`, the packaging markers and `workflow-lib/prelude.js`; lint checks every command named in a prompt. |
| SEAM-35 | minor | Fixed. The state design's class table is the only source (4.7). |
| SEAM-36 | minor | Fixed. The mapping in 4.12, a test for unknown keys, phantom keys dropped, `max_open` enforced. |
| SEAM-37 | minor | Fixed, differently. The grammar and a choice-to-status table are adopted. The idea list is one decision with a list per choice, not one decision per idea, to keep it to one typed line. |
| SEAM-38 | minor | Fixed. The twelve workflow names; `wave-plan` dropped; the status skill body follows the state design. |
| SEAM-39 | minor | Fixed. The layout and lint rows are added. |
| SEAM-40 | minor | Fixed. `stages.toml` carries input and output globs; `routes.toml` holds the route table; `next` is an object everywhere. |
| SEAM-41 | minor | Fixed. `VSDD_TEST_TIER` through the survey; a documented and tested schedule recipe. |
| SEAM-42 | minor | Fixed. Ancestor searches are allowed once a probe shows the search tool skips the excluded state directory; until then prompts pass explicit paths. |
| SEAM-43 | minor | Fixed. Holdout symptoms have their own shape without a location and are not grounded. |
| SEAM-44 | minor | Fixed, by the alternative. An amendment covers the whole specification set; every edited file returns to `draft` and the next `spec` decision approves it. |
| COV-01 | blocker | Fixed. The `decide` skill; a probe of the prompt-expansion event before the rule is built; a stated fallback; the terminal shim. |
| COV-02 | blocker | Fixed. One owner and build packages G1 to G7. The owner is the state design, not packaging as proposed, because survey, checkpoint and the stores are already there. |
| COV-03 | major | Fixed. Detached jobs with bounded waits (4.9). |
| COV-04 | major | Fixed. Decision D4 is yes. The `auto-allow` hook rule is the mechanism, the answer is stored as `[hooks.rules.auto-allow] scope` by `vsdd init --auto-allow`, and `setup` asks before it runs `init` (5.4; second pass P2COV-02). |
| COV-05 | major | Fixed. The `drive` skill, `vsdd driver`, and a probe that a skill can start a workflow. The driver lives in the main session because of the one-level nesting limit. |
| COV-06 | major | Fixed. Every step names a method; story kinds select agent and method (4.6); the survey returns the plugin root. |
| COV-07 | major | Fixed. Roster names, methods as lenses, and the clerk. |
| COV-08 | major | Fixed. `disposition` and `port-analysis` stages for reference ingests. |
| COV-09 | major | Fixed. A `collect` stage run by code from configured source commands; a schedule recipe. `vsdd cache` is dropped: research reports are the cache, and the researcher has no shell. |
| COV-10 | major | Fixed. The `[release]` section, detected by `vsdd init`. |
| COV-11 | major | Fixed. Both roles can ask; the runner inherits the asker's walls. |
| COV-12 | major | Fixed. The two commands, and `<plugin>/assets/` for the design-system starter, demo templates and scaffold files. |
| COV-13 | major | Fixed. The terminal shim, reported by `vsdd doctor`. |
| COV-14 | minor | Fixed. The skill mapping is re-keyed. |
| COV-15 | minor | Fixed. The two rows are re-marked: findings are validated by schema, and the pull request body by a unit test of the tool's template. |
| COV-16 | minor | Fixed. The hook command guards itself; the platform's failure setting is probed. |
| COV-17 | minor | Fixed. Every spike is isolated and keyed by agent id. |
| COV-18 | minor | Fixed. Child budgets, a reserve for the close, and a harness that models the shared counter. |
| COV-19 | minor | Fixed. The `sources` argument and `intake` stage. |
| COV-20 | minor | Fixed by the resolution of SEAM-06. The reviewer's variant (a model-visible skill and a renamed workflow) is the stated fallback if a probe shows the model cannot start the workflow itself. |
| COV-21 | minor | Fixed. Both probes run before agent files are written; the clerk lists `StructuredOutput`. |
| OPS-01 | blocker | Fixed. Detached jobs, one clerk per CI wait, and an eleven-minute probe. |
| OPS-02 | blocker | Fixed. The digest leaves out status; the decision is written before the promotion; a fixture test covers approve, survey, plan. |
| OPS-03 | blocker | Fixed. `decide` and `drive` skills, `vsdd driver`, the shim. The driver is person-invoked; the briefing carries the instruction that lets a compacted session continue. |
| OPS-04 | blocker | Fixed. Reference existence is checked at gates and approval, syntax at the write. |
| OPS-05 | major | Fixed. Rounds persist; a unit with an open decision is waiting; the route row is reordered. |
| OPS-06 | major | Fixed. Verdict records, `dirty` and `open_pr` in the survey, pull request adoption, a definition of "owned elsewhere", and mid-step kills in the tests. |
| OPS-07 | major | Fixed. Hook-owned liveness, the session marker, one scope rule, an atomic take. The optional cross-machine lease is not built: two machines are reconciled when state is pushed. |
| OPS-08 | major | Fixed. Step-aware compaction instructions and a repeated identity; a probe of compaction inside a step. |
| OPS-09 | major | Fixed. Required `data`, `try`/`finally`, one relay retry, deferred findings kept in step records, a `review-blocked` decision, and a universal test. |
| OPS-10 | major | Fixed. Block once, then advisory; paths resolve to the unit's worktree; three lines of drift. |
| OPS-11 | major | Fixed. As SEAM-04, with field names pinned in the agents' result sections. |
| OPS-12 | major | Fixed. Decision D4 is yes and is built as the hook permit (5.4). `-C <dir>` is a global option of the tool, and the clerk's prompt passes it, so clerk commands stay one segment (4.9; second pass P2OPS-02). |
| OPS-13 | major | Fixed, adapted. No rebase unless the pull request conflicts; verdicts follow the patch id; a gate semaphore; `[commands.env]`; a port per product start. The wave gate is the batch check on the integration head. |
| OPS-14 | major | Fixed. The configuration is not tool-writable; gate-bearing sections are fingerprinted; settings and plugin-disable routes are blocked; the survey needs the hook marker. |
| OPS-15 | major | Fixed. A subcommand allow-list per role, more person-only commands, step records accepted only from the right agent type in a live run and validated against the schema before they count, reserved event types, and no direct calls to the package. |
| OPS-16 | major | Fixed. Wider secret globs and commands, an outside-project read wall for agents without a shell, web calls logged and checked for known secret values and token formats (in place of a general entropy test, which would block ordinary URLs), and upload flags blocked. |
| OPS-17 | major | Fixed. As SEAM-11, plus the bundled browser server is started with file access off. |
| OPS-18 | major | Fixed. A JUnit report is required; tests freeze by identity; tests inside source files do not count. |
| OPS-19 | major | Fixed. Every agent omits the project `CLAUDE.md`; setup offers to move conventions and archive old process sections. |
| OPS-20 | major | Fixed. Dependency additions and out-of-range upgrades need a `merge` decision, batched per maintenance run. |
| OPS-21 | major | Fixed. Delivery state is recorded and becomes a `merge` decision with the link; readiness is a chore story; a run returns `needs_human` when all that is left waits on decisions; a trailing wave can be created. |
| OPS-22 | major | Fixed. The block message gives terminal commands; one no-download policy; `vsdd doctor --fix` installs Python; the sandbox needs are stated and checked inside a Bash tool call. |
| OPS-23 | major | Fixed. Coverage findings, `not_checkable`, `waive-coverage`, dismissal only of disputed findings, and one round for `direct`. |
| OPS-24 | minor | Fixed. The quarantine variable, valid only after repeated errors of that rule, announced in every briefing. |
| OPS-25 | minor | Fixed. One clerk per CI wait, fix units returned by a failing wave gate, batches of 12 scenarios, no project `CLAUDE.md` in agents, the budget set from measured counts. Tiers are decision D5. |
| OPS-26 | minor | Fixed. A per-child budget enforced by the wrapper and a unit test that derives the bound. |
| OPS-27 | minor | Fixed. Findings carried across attempts, refusal before an agent starts, a question hash, isolation, phantom keys dropped. |
| OPS-28 | minor | Fixed. No prose reports from pipeline reviews, caps as status problems, flush at fewer points. |
| OPS-29 | minor | Fixed. The status record is local and notes are a tracked file. |
| OPS-30 | minor | Fixed. Commits, amends and branch-moving resets are blocked on a checked-out integration or release branch, with the route named. |
| OPS-31 | minor | Fixed. Setup switches off the attribution setting and `vsdd doctor` checks it. |

---

## 9. Build plan

Each package is sized for one builder and one reviewer. Builders work only under `next/`, the staging root that
mirrors the final repository. "Design" names the document a package is built from; the blueprint's section 4
overrides it where they differ.

### 9.1 Foundations

| ID | Package | Delivers | Depends on |
|---|---|---|---|
| B0 | Rewrite-mode root | A fifteen-line root `CLAUDE.md`, the `next/` skeleton, a `tools/check.py` that passes | Blueprint approved |
| B1 | Manifest and shell | `plugin.json`, plugin README, licence, icon, `.mcp.json`, empty component folders, lint for both | B0 |
| B2 | Launcher and entries | `py/launch.sh`, `find-root.sh`, a stub `hook.py`, `cli.py --version`, `bin/vsdd`, shell lint | B1 |
| B8 | Platform probes | The items of section 10 that a toy probe can settle (those still open among 3, 5, 6, 9, 10, 12, 13, 16, 18 and 27 to 35), results written to `rewrite/STATE.md`. Items 1, 2, 4, 7, 8, 11, 14, 15 and 17 are already proven. Items that need the product (7 at wave scale, and 19 to 25) belong to R0, E1 to E4 and T5. Its answers can change H, S and K packages, so it runs early. | B2 |
| R0 | Marketplace install check | `install-check` in the marketplace repository | none |
| B3 | Shared core | `project.py`, `config.py`, `frontmatter.py`, `events.py`, `schema.py`, unit tests | B2 |
| C1 | Schemas | `<plugin>/schemas/*.json` as amended in 4.10, with fixtures | B3 |
| C2 | Policy tables | `artifacts.toml` (with the story kinds), `classes.toml` (with the class precedence), `walls.toml` (rows, callers, reveals) and their policy tests | B3, C1 |
| B6 | Lint and roster | The lint suite and `tests/lint/roster.toml`, including `[overrides]` and the rule that every story kind's implement agent can write what its method produces | B1 |
| B4a | Prelude build | `workflow-lib/prelude.js` (with `runStories()`, the argument parser and the percent-encoder), `agent-shared.md`, `severity.md`; `tools/build.py` for the prelude, schema literals, the tier, role and override tables and the stamped agent text, with `--check` | B0, C1, B6 |
| B4b | Hook wiring build | `tools/build.py` generating `hooks/hooks.json` from the rule registry, with `--check` | B4a, H1 |
| B5 | Workflow harness | `harness.mjs`, the universal tests, the shared agent counter, two toy workflows | B4a |
| B7 | CI | `ci.yml` with static, unit, workflows and validate jobs | B2, B5, B6 |

C3 (stages and routes) no longer exists as a package: there is no stage registry, and the route function and
the preconditions are part of S3.

### 9.2 State and the measuring tool

| ID | Package | Delivers | Depends on |
|---|---|---|---|
| S1 | State core | `detect`, mount and `init` (detection, `--auto-allow`, `--hub`), the configuration schema with the full key table, registry validation, skeleton templates, `new`, `list`, `refs`, `coverage`, `path`, `validate`, `config --json`, `config set`, the global `-C` option | B3, C2 |
| S2a | Record stores | Run, step, evidence, closure, decision, cycle, request and note stores, with `vsdd note`; the digest | S1 |
| S2b | Lock, caller identity and flush | The run lock; reading caller markers; flush; the driver flag (`on`, `pause`, `off`) | S2a |
| S3 | Survey, checkpoint and route | The done test, the preconditions and the route function with a fixture per row, unit positions, the worktree library, story admission, decisions opened, cycles opened and closed (with the person-only `vsdd cycle close`), the standing cycle, applying an approved toolchain to `factory.toml` | S2a, S2b, C1 |
| S2c | Status and briefing | The one function behind the briefing and `vsdd status`; the repairs of `status --fix` | S3 |
| S4 | Decisions | The grammar with parser fixtures, the choice table, qualified identifiers, settle (the record only), the approve, survey, plan fixture | S2a |
| S5 | Importer | `vsdd import` with fixtures cut from the eight real layouts; the baseline, the imported `spec` decision, queued requests and `factory.toml`; the fixture "import, then one fast-route feature" | S1, S2a |
| S6 | Registers | Spike ledger commands (`open` with the asker taken from hook records, `record`, `close`), ideas, `defer`, `retire`, `report debt`, `discover collect` | S2a, S2b |
| G1 | Jobs and product environment | Detached, re-attaching jobs keyed by command, subject and commit; the semaphore; `env start/stop` with ports, the caller record and cleanup | B3 |
| G2 | Story gates | `gate red` (JUnit, frozen tests, `--refreeze`), `green` (with the baseline comparison), `demo`, the diff patch writer | S2a, G1 |
| G3 | Verdicts | The verdict function, grounding, `review show` with redaction of walled findings, `review prepare`, and the conformance fixtures in Node and Python | S2a, B5 |
| G4a | Delivery | `story deliver`: the verdict check, push, pull request with the story trailer, CI wait, patch id, merge with the head-commit match, adoption, local mode, `delivery.ci`; the `gh` test double | G2, G3, S3 |
| G4b | Governed delivery and fix stories | Protected paths, dependency detection and approved dependencies, `merge` decisions, the hotfix branch, `story new-fix` with admission | G4a |
| G5 | Planning gates | `gate spec`, `gate plan` (and `--story` with admission), `plan schedule` with the baseline check, `feature route` | S3, G1 |
| G6a | Wave gate | `gate wave`: the integration worktree per repository, mutation, clone fidelity, settle, rotation, the UI check, fix units | G2, G5 |
| G6b | Release gate | `gate release` with its checks (`baseline`, `formal`, `perf`, `ui`) and `--only` | G6a |
| G7 | Remaining gates and release | `gate ingest` with the baseline record, `contracts`, `integration`, `sweep`; `vsdd release`; `repos split` | G4b, G6b |

### 9.3 Hooks

| ID | Package | Delivers | Depends on |
|---|---|---|---|
| H1 | Runner core | Dispatch, context, output, failure behaviour, the inert path, the caller marker and the command rewrite, the event log, the self-guarding command that stays inert | B3 |
| H2 | Safety rules | `bash-destructive`, `bash-git`, `secrets` (with loopback uploads), web call checks, with the harvested old cases | H1 |
| H3 | Walls | `read-walls` (reveals, `--include-walled`, `file:` addresses, searches, the final-message check for the main session), `write-scopes` (class precedence, scaffold and manifest cases), dispatch | H1, C2 |
| H4 | Artifacts | The `artifacts` rule on the registry, with the whole-set specification lock | H1, S1, S2a |
| H5 | Delivery order | `tests-first`, `merge-guard` | H1, S2a |
| H6 | Gate guard and allow | `gate-guard` (callers, person-only commands, settings), `auto-allow` (`core` and `extended`) | H1, C2 |
| H7 | Decisions and step results | `decisions`, `step-results` (agent type stored, walled findings) | H1, S2a, S4 |
| H8 | Spike bounds | `spike-bounds`, with admissions and the asker's walls | H1, S6 |
| H9 | Session rules | `session-briefing`, `compaction`, `lifecycle-log`, `purity-advice`, `hooks selftest`, `explain`, `rules` | H1, S2c |

### 9.4 Agents, methods and skills

| ID | Package | Delivers | Depends on |
|---|---|---|---|
| A1 | Agent frame | The agent template, the shared block, the severity table, `clerk`, agent lint | B6, B8, B4a |
| A2a | Product and UX agents | `product-owner`, `ux-designer` and their methods | A1, S1 |
| A2b | Architecture and planning agents | `architect`, `story-writer` and their methods | A1, S1 |
| A3 | Analysis and research agents | `codebase-analyzer`, `researcher`, `spike-runner` and their methods | A1 |
| A4 | Builder agents | `test-writer`, `implementer`, `verifier` and their methods | A1 |
| A5 | Delivery support agents | `demo-recorder`, `technical-writer`, `devops` and their methods | A1 |
| A6a | Code-facing reviewers | `adversary`, `code-reviewer`, `security-reviewer` and their methods | A1, C1 |
| A6b | Product-facing reviewers | `holdout-evaluator`, `ui-reviewer`, `scenario-reviewer` and their methods | A1, C1 |
| K1 | Control skills | `status`, `setup`, `decide`, `drive` | S2c, S4, S5, H6, H7, T1 |
| K2 | Authoring skills | `brief`, `author`, `review`, `defer` | A2a, A2b, A5, A6a, S6, G3, T5 |
| K3a | Preview skill | `preview` and its server | B3 |
| K3b | Tool skills | `research`, `retro`, `observe`, `export-pdf` | A3, T3, T5 |

### 9.5 Workflows

Each package is one script with its scenario tests, unless its row says otherwise.

| ID | Package | Depends on |
|---|---|---|
| W1 | `spike` | B5, S3, A3, S6, H8 |
| W2 | `story` | B5, S3, A4, A5, A6a, G2, G3, G4a |
| W3 | `wave` | W2, G4b, G6a, A6b |
| W4 | `spec` | B5, A2a, A2b, A6a, A6b, G5 |
| W5 | `plan` | B5, A2b, A6a, G5 |
| W6 | `feature`, with a scenario test for each fast route and for the amendment run | W2, W4, G5 |
| W7 | `verify` and `release` (the former W8: release is five steps and closes the cycle `verify` checked) | W2, G6b, G7, A3 |
| W9 | `ingest` | A2a, A2b, A3, A6a, G7 |
| W10 | `discover` | B5, S3, A2a, A2b, A3, A6a, S6 |
| W11 | `maintain` | W2, G7, S6, A3 |
| W12 | `multi`, with `vsdd repos scan` (the former T4, which serves only multi-repository projects) | W2, G7, A2a, A2b |

### 9.6 Operator tools, proof, documents, release and cutover

| ID | Package | Delivers | Depends on |
|---|---|---|---|
| T1 | `vsdd doctor` | Checks, the terminal shim, the sandbox needs each tested, the hooks-alive check through the caller marker | B3, H9 |
| T3 | Events and observability | `vsdd events` (queries, replay, reports, durations; the former T2), the stack, `vsdd obs`, dashboards | H1 |
| T5 | Export and render | `vsdd export pdf`, `vsdd render diagram`, `<plugin>/assets/`; begins by proving the engines (section 10, item 24) | B3 |
| E1 | Smoke tier | The sample project with its JUnit runner, fixtures, the probe runner, the smoke probes, including a prompt count for one spike and for one builder step in the default permission mode | H1 to H9, S3, W1, A4, A6b, G2 |
| E2 | Story tier | One story delivered; kills at a step boundary, mid-step and mid-CI-wait; an eleven-minute gate; budgets from measured counts (section 10, items 6 and 22) | W2, G4a, E1 |
| E3 | The greenfield proof | A run on the sample project from an empty repository and a brief, driven by `/vsdd-factory:drive`, through release, then one trivial feature after the release; the wave-scale prompt count; the escape rate (section 10, items 7, 21 and 22) | W1 to W7, K1, K2 |
| E4 | The brownfield proof | Four runs: an ingest of the sample with tests and one known lint failure, the baseline approval, a feature and a bug fix through to release; the same sample with no tests and no CI; an import fixture cut from `vantage` or `opnrynkr`, then one feature; a two-repository hub with a change that crosses both | E3, W9, W12, S5 |
| D2 | Documents for users | `tools/gen_reference.py` and the generated reference (the former D1); the hand-written pages, including the schedule recipe and troubleshooting | A, K, W, H, T packages |
| D3 | Contributor documents | Final `CLAUDE.md`, `CONTRIBUTING.md`, `RELEASING.md`, `CHANGELOG.md`, `README.md` | B7, R1 |
| R1 | Release tooling | `tools/release.py`, `release.yml` | B7, E2 |
| CO1 | Cutover tooling | `tools/cutover.py`, `docs/migrating-from-rc.md` | S1, S5 |
| CO2 | Cutover | Steps X0 to X8 of the packaging design: rehearse, pin the marketplace to the old tag, tag the archive, promote `next/`, merge, release 1.0.0, collapse the branches, migrate the eight projects, delete `rewrite/` | Everything above; E3 and E4 passed |

There are 74 packages. The second pass merged five small ones into a neighbour (C3 into S3, T2 into T3, T4
into W12, W8 into W7, D1 into D2), split six that were too large for one builder and one reviewer (S2, G4, G6,
A2, A6, K3), split B4 to break a dependency cycle with H1, and added E4.

Order of work: B0 to B3, B6, B8, R0, C1 and C2 first, then B4a and B5. Then S, G, H and A packages run in
parallel; B4b follows H1. Workflows follow the packages they call. E3 and E4 are the gate for cutover.

---

## 10. Platform facts: proven and not yet proven

Everything in the second table is documented by the platform or assumed by a design. Since the blueprint was
first written, a round of probes has been run against it with the spike plugin (2026-11-23, Claude Code
2.1.331, recorded in `rewrite/STATE.md`). The first table gives what they showed. Package B8 tests each item
that is still open before the packages that depend on it, and records the result in `rewrite/STATE.md`.

| # | Result of the probe |
|---|---|
| 1 | Proven. The hook receives the whole result, and a block makes the agent correct it and return again. The `step record` fallback is removed from the design. |
| 2 | Proven, headless. The event fires with the command name and arguments, and a block ends the prompt before any model reads it. |
| 3 | Partly. A refused stop keeps an agent running when it returns text. It does not hold a workflow step that has already returned a schema result. The design therefore blocks the result call for workflow steps (item 1) and refuses the stop only for agents dispatched in a session. |
| 4 | Proven, headless, for a skill a person invokes. Left to its own judgment the model never started a workflow; it dispatched the spike agent. Section 6 is written to that. |
| 5 | Proven in the main session, a dispatched agent and a workflow step, headless. `! vsdd` typed by a person is untested; the shim covers the person. |
| 6 | Partly. A 150-second call with an explicit timeout ran inside a step, and a detached job outlived the call that started it. The full ten minutes is tested in E2. |
| 7 | Proven in the default permission mode, in the main session, a dispatched agent and a workflow step: the approved command ran and the control was refused. In the `auto` permission mode, which your sessions use, the control was allowed anyway, so there the rule changes little. The count over a whole wave is measured in E3. |
| 8 | Proven. |
| 11 | `omitClaudeMd` is honoured, and a forked skill can name a plugin agent. `isolation: worktree` is honoured for a dispatch and ignored for a workflow step, so every step that needs isolation passes it itself, as the design does. |
| 14, 15 | Proven. |
| 17 | Measured. An agent with an explicit tool list starts with about 2,700 to 3,400 tokens of context; an agent with no restriction starts with about 19,000. |
| 9, 10, 12, 13, 16, 18 to 35 | Not yet probed. |

One more observation to check before the end-to-end tests rely on it: a project `CLAUDE.md` in a scratch
directory under `/private/tmp` was not loaded by a headless session, while the user-level file was.

| # | The design relies on | How to test | If it fails |
|---|---|---|---|
| 1 | The hook on the `StructuredOutput` call sees the whole step result, and a block makes the agent correct it and return again | A workflow step returns a review with a bad quote; inspect the payload and the retry | Proven. Should a later release break it: the hook stores the result at agent stop from the subagent's transcript, which needs no tool in the agent, and grounding happens only when the verdict is computed |
| 2 | The prompt-expansion event fires for a typed `/vsdd-factory:decide`, carries the arguments, and a block keeps the text from the model; also in headless mode | Type the command in a probe session with a recording hook | Use the prompt-submit event with a test that the prompt begins with the command, and document that it is weaker |
| 3 | A refused stop keeps an agent running | A spike step tries to stop without a ledger record | Observed: it holds a dispatched agent and not a workflow step. Workflow steps are held by blocking the result call instead; the tool's own checks at delivery remain |
| 4 | A skill can make the main session start a plugin workflow, interactively and headless, and the model can start a workflow on its own judgment | A probe skill that says "start workflow X" | The briefing prints the exact command and a person types each one; for spikes, the fallback in decision D8 |
| 5 | A file in `<plugin>/bin/` is on `PATH` in the main session, an Agent-tool subagent, a workflow step and headless mode; and `! vsdd` typed by a person | `vsdd --version` in each | Prompts name the tool by the plugin root the survey returns; the shim covers the person |
| 6 | A Bash call inside a workflow step accepts a ten-minute timeout, and a detached job outlives the call that started it | An eleven-minute gate | Shorter wait slices; the job design is unchanged |
| 7 | A hook's permission approval is honoured for calls made inside agents, so a wave in the default permission mode raises no prompts (decision D4) | Proven for single calls. E1 counts prompts for a builder step and a spike; E3 counts them for a wave | Setup writes static allow rules into the project's local settings instead. Static rules cannot be scoped by agent role, so this is the fallback only |
| 8 | An agent that lists the Agent tool still has none as a workflow step; an agent with `Bash, StructuredOutput` returns a schema result | Two toy steps | Adjust the clerk's tool list. A step that can dispatch is still bounded, because every spike must pass `vsdd spike open`. |
| 9 | Hooks fire for MCP tools and `WebFetch` when the matcher names them | A recording hook and one browser call | Walled agents lose browser tools and drive Playwright through the shell, where the walls already apply |
| 10 | Agents that list both spellings of an MCP tool can call it; an unresolved name is dropped silently | One call from a step, a dispatch and a forked skill | Agents with MCP tools, and only those, inherit the session's tools |
| 11 | `isolation: worktree` and `omitClaudeMd` are honoured in a plugin agent's frontmatter; a forked skill can name a plugin agent | One probe each | Callers pass isolation; the hook walls `CLAUDE.md` for the holdout evaluator; the two forked skills dispatch from the main session |
| 12 | Compaction fires the pre-compaction hook inside a step agent | A long step | The repeated identity in the prompt is the only protection |
| 13 | The platform's "block on hook failure" setting covers a missing file and the prompt-expansion event | Remove the launcher in a probe | The self-guarding command already covers it |
| 14 | The search tool skips a directory listed in `.git/info/exclude` | A search from the project root | Ancestor searches stay blocked for walled callers and prompts pass explicit paths |
| 15 | The hook payload's working directory is the worktree for an isolated agent; a spike worktree can check out the asker's commit | A spike probe | Story and spike detection use the target path only |
| 16 | A hook can run `git` in the state worktree before compaction within its time limit; platform worktree commands do not disturb the `.factory` mount | A compaction probe in a project with state | Flush only inside `vsdd` commands; `vsdd status --fix` remounts |
| 17 | Agents without the Skill tool do not receive skill listings; the fixed cost per agent start on your machine | Measure token use per start with your plugins enabled | Document running pipeline sessions with only this plugin enabled |
| 18 | Cheap models can be forced for tests through the survey's tier override | A smoke run with `VSDD_TEST_TIER` | The smoke tier costs several times more |
| 19 | The packaging assumptions N2 to N8: the version floor is enforced; plugin commands run in CI without credentials; a tag-and-commit pin installs; `claude plugin tag` works in a subdirectory; a server with no key is quiet; a stale `agent` setting is harmless; the shared cache survives a partial migration | `install-check` and step X0 | Each has a fallback in `packaging.md` 2.13 |
| 20 | The Node harness matches the platform's workflow rules, including the agent counter shared by parent and child | The weekly conformance tier | Fix the harness |
| 21 | One clean pass per perimeter lets through few enough defects | In E3, count findings first caught at a later perimeter | Raise the cap or add a lens (D9) |
| 22 | A realistic story costs 8 to 12 agent starts and a six-story wave 70 to 90 | Measure in E2 and E3 | Set the budgets from the measured numbers; cut clerk steps further |
| 23 | JUnit reports can be detected for the toolchains in use | `vsdd init` on the eight projects in dry-run mode | The red gate opens a `blocked` decision with the reason `environment`, naming the missing report |
| 24 | The PDF and diagram engines work through `npx` | The first task of T5 | Decision D14: drop the skill |
| 25 | `maintain` and `discover` start from a scheduled headless command | The schedule recipe's probe | They run on demand only |
| 26 | Whether a step inherits the agent file's effort | It cannot be observed | Not relied on: every step sets effort |
| 27 | A hook can rewrite a Bash command through `updatedInput`, in the main session, a dispatched agent and a workflow step, so a `vsdd` call can carry its caller marker's key | A probe hook that adds an environment assignment; the command prints it | The marker is keyed by the Claude Code process for the session (item 29) and matched to an agent by working directory, which is unique for every isolated agent |
| 28 | Hook payloads for tool calls inside workflow steps carry the main session's session id | Compare the ids in a recording hook | The lock is kept alive by calls whose working directory lies in a worktree the run owns, which the design already does |
| 29 | A long-lived Claude Code process can be found among the ancestors of a hook and of a Bash call, on macOS and Linux, with the Bash sandbox on | Walk the process tree in both | The fallback of item 27 is unavailable, so item 27 must pass; and the offered cut P2LEAN-17 is not taken |
| 30 | A hook's permission approval is honoured for the `Workflow` tool | The driver in the default permission mode | Setup writes one static allow rule for this plugin's workflows |
| 31 | Text typed after `/vsdd-factory:<workflow>` reaches the script's `args`, as a string or as an object | Type `/vsdd-factory:spike what is the uv cold start` in a probe workflow that returns `args` | The prelude's parser already accepts both; if nothing arrives, the person-facing entrances become small skills that start the workflow with an object |
| 32 | A forked skill's agent holds the Agent tool and can dispatch the spike runner | A forked probe skill | The `research` and `retro` skills instruct the main session to dispatch instead of forking |
| 33 | What happens to the bundled browser server when the user runs their own; and which option of the pinned server refuses `file:` addresses (the probe names it for `.mcp.json` and L-MCP) | Start a session with both configured, and open a `file:` address with the option set | The hook's check of navigation addresses applies to both spellings and to `browser_tabs`; a click cannot move a Chromium page from `http` to a `file:` address |
| 34 | `encodeURIComponent` exists in the workflow script runtime | A one-line probe workflow | A hexadecimal encoder written in the prelude |
| 35 | The hook can pair a spike runner that starts with the dispatch that started it | Two concurrent dispatches by different agent types | The runner is given the union of the walls of every admission pending in the session, which is the stricter result |

---

## 11. Second pass

On 2026-11-23, after you adopted the decisions, six reviewers read the blueprint and the five designs again.
Three checked whether the first-pass fixes had really been made (ids `P2SEAM-`, `P2COV-`, `P2OPS-`). One read
the whole design cold, as a builder would (`P2COLD-`). One traced it against your requirements, including the
brownfield question (`P2REQ-`). One looked for mechanism that could be removed (`P2LEAN-`). Their reports are
`rewrite/design/pass2-*.md`.

They raised 118 findings: 4 blockers, 43 major and 71 minor. The 100 defects are all fixed in this blueprint and
in the design documents. Of the 18 proposed cuts, 15 are applied in full and one in part, one is offered to you,
and one is declined as its own reviewer recommended. Where two findings proposed different fixes for one
problem, the choice followed the design goals in order: correct and enforceable, low touch, restartable, lean.

### 11.1 Every finding

"Fixed" names the change and where it is. "Cut applied" means the mechanism is gone from the design.

| ID | Severity | Outcome | What was done |
|---|---|---|---|
| P2SEAM-01 | blocker | Fixed | The product tree gains the classes `manifests`, `repo-config` and `harness`, with one precedence rule (4.7), granted by role (4.3). `devops` may create source and test files in a `repo-setup` chore story. The toolchain reaches `factory.toml` through the `spec` approval instead of a person's edit, and the scaffold story runs alone in the first wave (4.6). Hook cases cover a scaffold create and a manifest edit. |
| P2SEAM-02 | major | Fixed | The scenario review is its own loop, routed only to the product-owner. Claims carry no quote. Walled findings are stored in the holdout class and appear elsewhere as key and severity (4.10). `review show` redacts them. A universal test checks every prompt (5.5). |
| P2SEAM-03 | major | Fixed | The probe passed (section 10, item 1), so the `step record` fallback is removed from every document. The fallback named for a later regression needs no tool in the agent. |
| P2SEAM-04 | major | Fixed | `spike open`, `record` and `close` are specified in `state.md` 2.16 with file inputs. The runner opens or attaches to a named spike itself. `record --increment` satisfies the stop check. Every check matches by agent id (section 6). |
| P2SEAM-05 | major | Fixed | The `discover` assess checkpoint opens the cycle through the same code as `checkpoint brief` (4.9, 5.2), with a fixture. |
| P2SEAM-06 | major | Fixed | Route rows are scoped to the open cycle, and the queued-request row sits directly below the lock rows (5.1). The three named fixtures are added (5.5). |
| P2SEAM-07 | major | Fixed | A finding built from scenario scores has a defined shape and is exempt from the location and grounding conditions (4.10), with fixtures for both thresholds. |
| P2SEAM-08 | minor | Fixed | `pipeline.md` 2.9.13 gives the method of every step. Eight methods are added (5.3). Scenario freshness is split between the evaluator and the scenario-reviewer. |
| P2SEAM-09 | minor | Fixed | The table in `agents-skills.md` 2.6 is rewritten in the schema's field names, with no report-path or merge entries. |
| P2SEAM-10 | minor | Fixed | `workflow-lib/severity.md` is the one text. It is reproduced in 4.10 and stamped into agent files. `demo-recorder` joins the owner list. |
| P2SEAM-11 | minor | Fixed | The example rows in `hooks.md` 2.7 are corrected: every row denies `runs`, builders get the read and measure groups, the main row is explicit. The stale paragraph in `agents-skills.md` 2.9 is replaced by a pointer. |
| P2SEAM-12 | minor | Fixed | List kinds are the artifact kinds. `[reveals]` is generated from each kind's class. An unknown kind is treated as revealing everything walled. `--include-walled` is in the command tables (4.9). |
| P2SEAM-13 | minor | Fixed | Every choice uses named values, with text after `--` (4.11), in all four documents, with parser fixtures. |
| P2SEAM-14 | minor | Fixed | No lessons register remains (P2LEAN-09). The checkpoint retires superseded research reports that a step names. The `defer` skill names `defer add` and `retire`. |
| P2SEAM-15 | minor | Fixed | A caller marker binds each `vsdd` call to its session and agent (4.9). Probe 27 tests the mechanism, with a fallback. |
| P2SEAM-16 | minor | Fixed | The "uv cannot run" row names `uv --version`, `uv cache dir` and the reinstall commands, none of which needs the plugin. |
| P2SEAM-17 | minor | Fixed | The green gate result and the delivery data carry `diff_patch`, and each reviewer is given the latest one (4.10). |
| P2SEAM-18 | minor | Fixed | `next` is an object wherever it is shown. "Route rule" is removed from the pipeline design's P4, R2 and 2.5. There is no status record to describe. |
| P2SEAM-19 | minor | Fixed | With no status promotion (P2LEAN-01), an amended file cannot keep an approval: the whole-set lock and the digest decide. Both places are reworded. |
| P2SEAM-20 | minor | Fixed | `hooks.md` S7 matches `state.md` S7, the conditions table matches, and guard G1 exempts `facade`, `docs`, `chore` and `harness`. |
| P2COV-01 | major | Fixed | The asker's type comes from records the hook wrote. `spike open` refuses without one. The "on behalf" relay is gone. A cached finding is returned only to an asker whose walls the earlier runner's covered (section 6). |
| P2COV-02 | major | Fixed | One mechanism, the hook permit. The switch is `[hooks.rules.auto-allow] scope`, written by `vsdd init --auto-allow`, and setup asks before it runs `init`. Spike commands take files, not standard input (5.4). |
| P2COV-03 | major | Fixed | Every step has a method (`pipeline.md` 2.9.13). `docs-drift` and `pattern-sweep` are added. Scenario freshness uses `holdout-eval` for the re-run and `review-scenarios` for staleness and coverage. |
| P2COV-04 | minor | Fixed | `runStories()` is stamped with the reserve check, a failed child is charged its full budget, and children receive `story_agents` (4.12). |
| P2COV-05 | minor | Fixed | Commands re-attach and wait 100 seconds by default (4.9). The closing command in a prompt carries the longer wait. `stallMs` is dropped. |
| P2COV-06 | minor | Fixed | Intake returns `covers`, and the checkpoint records those stages as done. The claim about hand-placed documents is deleted: they go through `sources`. |
| P2COV-07 | minor | Fixed | The guard walks up for `factory.toml` and stays inert in other projects (5.4), with a smoke case. |
| P2COV-08 | minor | Fixed | The two signatures are aligned, and the runner's agent id reaches the tool through the caller marker. |
| P2COV-09 | minor | Fixed | The shim resolves the enabled version from `installed_plugins.json` (4.1). Messages give the in-session form first. |
| P2COV-10 | minor | Fixed | The three stale passages in `state.md` are corrected. |
| P2COV-11 | minor | Fixed | The pipeline design's S1 cites 4.12 and `state.md` 2.12 as the one key list. |
| P2OPS-01 | major | Fixed | As P2SEAM-15, with probes 27 to 29, and the lock kept alive by calls from a worktree the run owns (4.13). |
| P2OPS-02 | major | Fixed | As P2COV-02. `-C` is a global option of the tool. The smoke tier counts prompts for a builder's own shell calls. The wider scope is a choice at setup (5.4). |
| P2OPS-03 | major | Fixed | One precedence rule for classes (4.7), used by the write scopes, the tests-first rule and the red gate, with the three hook cases. |
| P2OPS-04 | major | Fixed | A story's decision holds back only its dependents. A wave's own gate decision blocks the waves after it. One function serves the route and the survey (4.11), with the scenario test. |
| P2OPS-05 | minor | Fixed | Settling writes the decision record only. There is no promotion. The fixture is in package S4. |
| P2OPS-06 | minor | Fixed | The driver pauses at a decision and resumes when it is settled (4.5). |
| P2OPS-07 | minor | Fixed | The sentence in `state.md` 2.11 is replaced, and adoption of the tool's own pull requests is defined (4.13). |
| P2OPS-08 | minor | Fixed | A reviewer that returns blocked opens `blocked` with the reason `reviewer` at once, and the paragraph says so. |
| P2OPS-09 | minor | Fixed | As P2SEAM-09: the implementer is given findings inline, and reviewer report paths are for on-demand reviews only. |
| P2OPS-10 | minor | Fixed | R8 is rewritten. The merge passes the current head whose patch id matches the verdict. Started products are recorded with their caller and stopped. `commands.run` takes a port (4.12). |
| P2OPS-11 | minor | Fixed | The fingerprint covers five more sections, and the briefing announces a change to any gate-bearing section (5.4). |
| P2OPS-12 | minor | Fixed | `status --fix` leaves the read group (4.9). |
| P2OPS-13 | minor | Fixed | The spike runner is denied reads outside the project. Web addresses in shell commands are logged and checked. Two more environment dumps are blocked (5.4). |
| P2OPS-14 | minor | Fixed | The bundled entry carries the option and lint checks it. The two-server case is probe 33. |
| P2OPS-15 | minor | Fixed | Setup offers to archive old process sections, and the cutover report points at the offer (4.5, 5.5). |
| P2OPS-16 | minor | Fixed | The sandbox needs are listed and each is tested by `vsdd doctor`. A missing Python is fixed by a terminal command, and `doctor --fix` is removed. |
| P2OPS-17 | minor | Fixed | The figures are 8 to 12 agent starts per story and 70 to 90 per six-story wave, and the test budget is 12 per story. |
| P2OPS-18 | minor | Fixed | Children receive a share of the run's remaining spikes (4.12). |
| P2OPS-19 | minor | Fixed | The flush points, the review path row and the status of register limits in `validate` are corrected. |
| P2OPS-20 | minor | Fixed | The skeletons in `pipeline.md` 2.11 are corrected in place, and the amended behaviours are conformance fixtures (5.5). |
| P2LEAN-01 | major | Cut applied | Status values are read from the record that decides them. Code no longer rewrites tracked files (4.7, 5.2). |
| P2LEAN-02 | major | Cut applied | Measured checks are combined, the gate names fall to 11, and `release` is one command (4.9, 5.1). |
| P2LEAN-03 | major | Cut applied | The four planners get a `vsdd`-only shell (4.3). |
| P2LEAN-04 | major | Cut applied | There is no status record and no verdict record (4.7). |
| P2LEAN-05 | major | Cut applied | Long commands re-attach. The job commands and the second output shape are gone (4.9). |
| P2LEAN-06 | major | Cut applied | There is no stage registry, no per-stage digest and no `vsdd stale` (4.14). The decision digest stays. |
| P2LEAN-07 | major | Cut applied | About 48 subcommands (4.9). |
| P2LEAN-08 | minor | Cut applied | One `blocked` kind with a reason. Nine decision kinds (4.11). |
| P2LEAN-09 | minor | Cut applied | No lessons register. Proposals live in the retrospective report and are adopted through `author` (4.7, 5.3). See 11.2. |
| P2LEAN-10 | minor | Cut applied | The agent type is stored with every step record and checked when a pass is counted (5.4). |
| P2LEAN-11 | minor | Cut applied | Each request is stored once. There is no summary file (4.7). |
| P2LEAN-12 | minor | Cut applied | There is no compaction snapshot file (5.4). |
| P2LEAN-13 | minor | Cut applied | Three policy tables. The route table is code (4.14). |
| P2LEAN-14 | minor | Cut applied in part | Five of the seven merges are made (section 9). The importer and the spike rule keep their own packages, because this pass enlarged both. |
| P2LEAN-15 | minor | Cut applied | The `PostToolUse` wiring is removed (5.4). |
| P2LEAN-16 | minor | Cut applied | `vsdd events emit` is removed (4.9). |
| P2LEAN-17 | minor | Offered to you | See 11.2. |
| P2LEAN-18 | minor | Declined | As its reviewer recommends, the survey and checkpoint clerks stay. It is recorded so that they are known to be deliberate. |
| P2COLD-01 | blocker | Fixed | The tool admits a story written after the survey and returns its unit (4.6). Section 3.3 is corrected. |
| P2COLD-02 | blocker | Fixed | As P2SEAM-01. A facade is measured by clone fidelity. A lint rule checks that each story kind's implement agent can write what its method produces (4.3). |
| P2COLD-03 | major | Fixed | The toolchain file approved with `spec` fills the unset keys (4.6). "Commands not configured" is a route state (4.12). E3 starts from an empty repository. |
| P2COLD-04 | major | Fixed | `vsdd gate red --refreeze`, for the test-writer only (3.3, 5.1), with a harness scenario. |
| P2COLD-05 | major | Fixed | Caller markers (4.9), and an asker type taken from hook records (section 6). |
| P2COLD-06 | major | Fixed | Free text travels as a percent-encoded argument produced by a prelude function, with a lint (4.9). Spike inputs are files the runner writes. |
| P2COLD-07 | major | Fixed | As P2SEAM-02. No caller reads `runs`. A policy test checks that no record outside the holdout class holds a scenario path or quote. |
| P2COLD-08 | major | Fixed | The amendment run inside the open cycle, its route row, and `plan` in delta scope afterwards (4.11). |
| P2COLD-09 | major | Fixed | The setting is defined and written by `init`. The wider matching is the `extended` scope a person chooses at setup (5.4). No decision kind was added for it. |
| P2COLD-10 | major | Fixed | `[overrides]` in the roster, generated into the prelude and held by lint (4.2). |
| P2COLD-11 | major | Fixed | `[hooks] main_session_wall`, the one stated exception to the locked walls (4.3). |
| P2COLD-12 | major | Fixed | The baseline record and the comparison rule (4.13). "First release" is defined. Ingest ends at a `spec` decision. Plan coverage applies to the cycle's own contracts (3.4). |
| P2COLD-13 | major | Fixed | Package E4. E3 is extended. Cutover waits for both. |
| P2COLD-14 | major | Fixed | `init` converts nothing on pre-1.0 state (4.9). |
| P2COLD-15 | major | Fixed | Uploads to a loopback port of a recorded product are allowed (5.4), with allow and block cases. |
| P2COLD-16 | major | Fixed | B4 is split, the worktree library is in S3, the missing edges are added, and B8 is limited to what a toy probe can settle (section 9). |
| P2COLD-17 | minor | Fixed | Each command that remains has a package (S1, S2c, G3, H3). The removed ones are listed in 4.9. |
| P2COLD-18 | minor | Fixed | Six packages are split (section 9). |
| P2COLD-19 | minor | Fixed | Adoption and "owned elsewhere" are defined (4.13). Jobs are keyed and re-attached, so a resumed run starts no duplicate (4.9). |
| P2COLD-20 | major | Fixed | The lock is defined by the decision, over the whole set. No status promotion exists, so no promotion map or code-only list is needed (5.2). |
| P2COLD-21 | minor | Fixed | Operator and author groups for the main session. `doctor` leaves the read group. Reveals are keyed with `--include-walled`. The main row loses the pipeline group (4.9). |
| P2COLD-22 | minor | Fixed | The full key table is in `state.md` 2.12. Release files are protected. The two shared texts are named (4.1, 4.12). |
| P2COLD-23 | minor | Fixed | Named preconditions per workflow, in code beside the route, each with a fixture (4.14; `pipeline.md` 2.5). |
| P2COLD-24 | minor | Fixed | As P2SEAM-06. |
| P2COLD-25 | minor | Fixed | What `approve` and `reject` do, and where an approved dependency is recorded (4.11, 5.2). |
| P2COLD-26 | minor | Fixed | `equivalence-tests` is added, `stubs` is merged into `failing-tests`, and the reference to a `release` method is removed. |
| P2COLD-27 | minor | Fixed | By P2LEAN-10: no generated table is needed. |
| P2COLD-28 | minor | Fixed | The sample project ships a standard-library JUnit runner (5.5). |
| P2COLD-29 | minor | Fixed | Qualified identifiers, `<id>@<cycle>` (4.8). |
| P2COLD-30 | minor | Fixed | The walk-through uses a bug fix that an existing contract covers, says "critical or high", and the check that could not fail is replaced (3.3; `pipeline.md` 2.9.4). |
| P2COLD-31 | minor | Fixed | `author conventions` and `author review-rules`. Setup dispatches the architect (4.5). |
| P2COLD-32 | minor | Fixed | Each passage is corrected in place, and `reviewer_blocked` joins the blocked kinds. |
| P2REQ-01 | blocker | Fixed | As P2COLD-01, with a harness scenario for each fast route. A hotfix story is cut from the release tag. |
| P2REQ-02 | major | Fixed | As P2COLD-12. Inherited debt is listed by `vsdd report debt`. |
| P2REQ-03 | major | Fixed | Missing commands are a state. Readiness chore stories are scheduled on every route. `delivery.mode` and `delivery.ci` are detected (3.4, 4.12). |
| P2REQ-04 | major | Fixed | The importer writes the baseline, the imported `spec` decision, queued requests and `factory.toml` (4.9), with a fixture. |
| P2REQ-05 | major | Fixed | One story per repository on a fast route. `setup --hub`. Wave records and integration worktrees per repository. Release and delivery settings per repository. Missing members are cloned (5.2). |
| P2REQ-06 | major | Fixed | As P2COLD-08. |
| P2REQ-07 | major | Fixed | The second cycle carries `cycle` in `next`, has qualified decision identifiers, delivers to `hotfix/<version>`, and merges back under the integration lock (4.11). |
| P2REQ-08 | major | Fixed | The `extended` scope covers spike experiments and web tools. `core` covers workflow starts and the driver's own commands (5.4). The smoke tier counts prompts for a spike. |
| P2REQ-09 | major | Fixed | Package E4 with the four cases, and harness scenarios for each fast route and for the amendment run. |
| P2REQ-10 | minor | Fixed | The admission record. Reviewers dispatch the spike runner themselves (section 6). |
| P2REQ-11 | minor | Fixed | The spike workflow takes no run lock. A spike needs an initialised project, and the workflow says so (section 6). |
| P2REQ-12 | minor | Fixed | Probes 31 and 32, and a string grammar parsed by the prelude (4.5). |
| P2REQ-13 | minor | Fixed | The override is built. The product-owner writes the final brief. Phases name no model. The conformance job watches the model aliases (4.2, 4.5). |
| P2REQ-14 | minor | Fixed | The switch exists. The final message of an agent that can read scenarios is checked before it reaches the main session (4.3). |
| P2REQ-15 | minor | Fixed | Release files are protected by default, and the plan ceiling is `stops.plan_ceiling_agents` (4.12). |
| P2REQ-16 | minor | Fixed | Route rows are reordered. `maintain` opens a maintenance cycle. The brief skill saves its draft, and "the brief is in draft" is a route row. |
| P2REQ-17 | minor | Fixed | Ingest writes conventions, review rules, the dependency list and the toolchain file. Setup dispatches the architect to move conventions. |

### 11.2 Cuts offered to you, and applied cuts you can reverse

1. **Offered: lock liveness by process, not by heartbeat (P2LEAN-17).** The run lock would record the Claude
   Code process that holds it and be stale the moment that process is gone. It saves the once-a-minute write on
   the hook's hot path, the `state.lock_stale_minutes` key, and the 20-minute wait after a crash. It loses
   nothing if probe 29 passes. Recommendation: take it if probe 29 passes on macOS and Linux with the Bash
   sandbox on; keep the heartbeat otherwise. Until you say, the heartbeat is the design.
2. **Applied, reversible: no lessons register (P2LEAN-09).** The capability, capturing what a cycle taught and
   proposing improvements, stays in the retrospective, and an adopted proposal now lands in the two files
   agents actually read. What is gone is a separate capped list of one-sentence rules. If you count that list
   as a capability in its own right, say so: it returns as a register with one added step, the architect
   folding adopted lessons into conventions at release.
3. **Not merged: the importer and the spike rule (part of P2LEAN-14).** Merging S5 into CO1 and H8 into W1
   would save two packages. Both were kept apart because this pass added to each. Say so if you prefer the
   merge.

### 11.3 Decisions only a person can make

No new decision blocks the build, and no adopted decision was reopened. Three consequences of this pass are
choices you make at the point of use, and you should know them now.

1. **The scope of decision D4 is chosen at setup, per project.** `core` is what you adopted, with the two
   things it needs in order to work as you meant it: a configured command still counts when a builder adds an
   argument to it, and the main session may start this plugin's own workflows. `extended` also approves
   read-only shell commands in the caller's own worktree, calls to a product the tool started, a spike runner's
   experiment commands inside its spike worktree, and the web and research tools. Two reviewers showed
   that an unattended run in the default permission mode still raises prompts without it. It goes beyond the
   list you adopted, so the plugin does not turn it on by itself: setup asks.
2. **An ingest ends with one decision.** Approving the recovered baseline is what makes it the locked
   specification and writes the toolchain into the configuration. It is one typed command, once per ingested
   project.
3. **A spike needs a project that has been set up.** The requirement was that a spike be available at any
   point. Hooks are silent before setup (decision D24), so no bound could be enforced there. The nearest thing
   that can be built is what the design does: one `setup` command first, and named spikes that share their
   files with your own `resume-spike` convention, which works anywhere.

### 11.4 Close-out

A check of the second-pass fixes (`rewrite/design/pass2-closeout.md`, 2026-11-24) found no blocker, 5 major
problems and 16 minor ones. All 21 are fixed in this blueprint and the design documents. No command, record
type, rule, agent, skill or build package was added, and no decision was reopened. Two policy names are new:
the path class `guidance` (M3) and the `[callers]` group `refreeze` (m8).

A check of that round (`rewrite/design/pass2-closeout-2.md`, 2026-11-24) found that five of those fixes had
each left one mismatch: 1 major and 4 minor, N1 to N5 at the end of the table. All five are fixed, again with
no new command, record type, rule, agent, skill or build package, and with no decision reopened. One field is
new, `flaky`, in the gate result and in a closure record (N4).

| ID | Severity | What was done |
|---|---|---|
| M1 | major | The `plan` route row applies only to a cycle that began with a brief or is on the standard route, or after an approved amendment. "The plan is ready" is defined. The `verify` row and precondition name the cycles with no plan (a fast route, a maintenance cycle). Two route fixtures are added (5.1; `pipeline.md` 2.4, 2.5; `packaging.md` 2.6). |
| M2 | major | The `spec` decision's digest leaves out the two guidance files, which keep their own lock, so adopting a retrospective's proposal, or moving conventions after an import, leaves the approval current (4.11, 5.2; `state.md` 2.4, 2.8). |
| M3 | major | The two guidance files are a path class of their own, `guidance`, inside `spec-arch`, and the code-reviewer's read wall allows it. It can now read the review rules it is given and, for `pattern-sweep`, the conventions (4.3, 4.7; `state.md` 2.4; `agents-skills.md` 2.9). |
| M4 | major | `gate red --refreeze` records a changed frozen test that passes or that fails on an assertion. When one fails, the implementer follows the test-writer in the same fix pass and ends with the green gate (5.1; `pipeline.md` 2.9.1 and `fix()` in 2.11.2; `state.md` 2.16; `agents-skills.md` 2.8). |
| M5 | major | The gate result schema has `unit`. The story-draft step's data schema requires `units`, which is what the script reads, and the step's prompt asks for the closing command once per story (4.10; `pipeline.md` 2.9.4, 2.10.6, 2.10.8, 2.11.1). |
| m1 | minor | Outside the holdout class a walled finding is named by a walled key, a hash of its key, so no record carries a scenario path (4.8, 4.10; `pipeline.md` 2.7; `state.md` 2.5). |
| m2 | minor | The title of a finding built from scenario scores is a fixed sentence made from the key and the score (4.10; `pipeline.md` 2.7). |
| m3 | minor | A final message that quotes a scenario is refused every time, not only the first (`hooks.md` 2.11.2). |
| m4 | minor | `parseArgs()` treats a `key=value` word as a named argument only when the key is one of the workflow's own arguments (`pipeline.md` 2.3, 2.11.1). |
| m5 | minor | `maintain` is report-only while a product cycle is open, with or without a lock, in every place that states it (`pipeline.md` 2.5, 2.9.10). |
| m6 | minor | E1 depends on A4, A6b and G2. W4 and W5 depend on B5, and so does W10, which had the same gap (9.5, 9.6). |
| m7 | minor | `vsdd note` is delivered by S2a and `vsdd cycle close` by S3 (9.2). |
| m8 | minor | `gate red --refreeze` has its own `[callers]` entry, `refreeze`, which only the test-writer's row holds (`hooks.md` 2.7). |
| m9 | minor | In the example rows the clerk holds the `env` group and the main session does not, as 4.9 says (`hooks.md` 2.7). |
| m10 | minor | The `verifier`, `devops` and `story-writer` example rows carry `dispatch`, and an absent key means none (`hooks.md` 2.7). |
| m11 | minor | An imported file over its cap is shortened by `author conform` when it is not locked, and in the next amendment run when it is in the specification set. After an import the route is `feature` with the queued request, or steady state when none was queued (`state.md` 2.15; `packaging.md` 2.6). |
| m12 | minor | The default `harness` globs include an in-source proof file, `**/proofs.rs`, so the verifier can write proofs kept beside the code (4.7; `state.md` 2.4, 2.12). |
| m13 | minor | The `feature` evolve row lists one step per method, with `holdout-scenarios`, `verification-properties` and the two assessment methods (`pipeline.md` 2.9.13). |
| m14 | minor | An inherited test that fails is run once more. One that passes in either run is recorded as flaky, listed as debt, and does not fail the gate (4.13; `pipeline.md` 2.9.8; `state.md` 2.8). |
| m15 | minor | The Playwright option stays a placeholder until probe 33 names and confirms it, and the probe's row says so (section 10; `packaging.md` 2.4). |
| m16 | minor | Stale text is corrected: D4, D11 and D14 in 2.1; the list of replaced rows in section 8; `pipeline.md` amendments 5 and 16; `agents-skills.md` 3.2 row 16 and the write-scopes table of 2.9; `hooks.md` K7; the test-writer's `source` grant in the `state.md` 2.4 class table; `packaging.md` 2.10. |
| N1 | major | The cycle's route is kept in `cycle.json`, which is tracked, and nowhere else: a cycle opened with a brief is `standard` from its opening, and `vsdd feature route` writes the route it derives. The `plan` row, the `verify` row, the `verify` precondition and the `verify` check list read that field, and "a cycle with no plan" is defined positively (route `trivial`, `bugfix` or `hotfix`, or kind `maintenance`), so a missing route cannot send a standard-route feature to `verify`. One route fixture is added: a standard-route feature with its delta specification approved and no run records on disk gives `plan` (3.3, 4.9, 5.1; `pipeline.md` 2.4, 2.5, 2.9.4, 2.9.6; `state.md` 2.7, 2.8, 2.16; `packaging.md` 2.6). |
| N2 | minor | The `spec` decision's digest leaves out the brief as well as the two guidance files, so drafting the next brief leaves the approval current and the set locked. A `spec` run that starts while an earlier approval stands holds an amendment, marked by its survey, which is how a later product cycle writes a set that an earlier approval locks (4.11, 5.2; `state.md` 2.4, 2.8, 2.16; `pipeline.md` 2.6, 2.9.3; `hooks.md` 2.11.3). |
| N3 | minor | `gated()` has an `each` mode for the fast-route story-draft step: every story's plan entry must pass and have its unit in `data.units`, the retry note names each story that did not, and after the last attempt the step is blocked instead of delivering with a story missing. `gate plan --story` run again for an admitted story returns the same unit. A two-story harness scenario is added in which the second story fails once (4.10; `pipeline.md` 2.9.4, 2.11.1; `state.md` 2.16; `packaging.md` 2.6). |
| N4 | minor | A gate lists flaky inherited tests in its result (`flaky`), and the command that writes the unit's closure record copies them there, because evidence is not kept. `vsdd report debt` reads them from closure records (4.13; `pipeline.md` 2.9.8, 2.9.10, 2.10.6; `state.md` 2.8, 2.16; `packaging.md` 2.6). |
| N5 | minor | The import row in 3.4 no longer says the next command is `feature`: the route names `feature` at once when the importer queued a request for an epic in flight, and otherwise the person starts it. |
