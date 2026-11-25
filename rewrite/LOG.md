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

**The user said yes on 2026-11-24** to both parts of the proposal below: a local commit (no push) and the
restructure before the amend step.
- `rewrite/` was committed locally as `2a23827b` on `rewrite/v1` (no upstream, not pushed). `next/` and the new
  root `CLAUDE.md` are committed when slice layer 1 finishes writing. No commit beyond those two has been
  authorised.
- Tools written and run: `rewrite/tools/sections.py` (a manifest of the 240 sections of the six original
  documents, in `rewrite/history/section-manifest.json`, and a coverage check), `rewrite/tools/spec_lint.py`
  (checks the new layout), `rewrite/tools/gen_packages.py` (generates the 74 package briefs from section 9 of
  the blueprint and the registry). The rules of the new layout are in `rewrite/spec/README.md`.
- **The restructure workflow was launched on 2026-11-24** (script at `rewrite/workflows/restructure.js`): one
  Fable indexer writes `rewrite/spec/registry.toml` and the four top-level documents; seven Sonnet extractors
  write the files for commands, records and schemas, hook rules, agents and skills, workflows and topics, state
  and artifacts, and packaging; one Sonnet closer makes the two checks pass; one Opus auditor samples for lost
  or altered content. Extractors copy text and record every disagreement between the old documents in
  `rewrite/spec/conflicts/`; they do not fix review findings. The six originals stay where they are until
  slice layer 1 has finished reading them, then move to `rewrite/history/design-v1/`.
- **The restructure finished on 2026-11-24** (ten agents, none failed, about 100 minutes). It wrote 381
  registered things as about 460 files under `rewrite/spec/` (62 commands, 15 records, 47 schemas, 22 config
  groups, 17 rules, 20 agents, 13 skills, 12 workflows, 31 artifacts, 21 tables, 85 topics, 32 packaging files,
  4 top-level documents, 74 generated package briefs) and recorded 109 disagreements between the old documents
  in `rewrite/spec/conflicts/`. Both checks pass: the layout check reports no problem, and all 240 original
  sections are accounted for (179 cited, 41 retired as history).
- **But the audit says it is not yet faithful enough to build from** (`rewrite/spec/conflicts/audit.md`). Of 100
  sections compared, 80 were carried faithfully. 35 findings: 10 blockers, 16 major, 9 minor. Prose and code
  skeletons were mostly copied closely. The damage is concentrated: machine tables where the converters wrote
  rows instead of copying them (the read walls, the roster, configuration defaults, limits, the artifact table);
  schemas whose list fields are not arrays; dropped interface text (the shared state function signatures, the
  rule `check()` signature, the definition of "the plan is ready", the prelude constraints, the spike
  question-matching rule, the launcher script, the harness test cases); two wrong retirements; history copied
  in; and about a dozen facts defined in more than one file. The spec is also no smaller than what it carries
  (about 1,040 KB against about 795 KB of design text), which fits the duplicate findings.
- **A repair run was launched on 2026-11-24** (script at `rewrite/workflows/restructure-repair.js`): five Sonnet
  repairers, one area each (hook rules and walls; commands; records, schemas and config; workflows, prelude and
  spikes; agents, skills, artifacts, state and packaging), each fixing the audit findings in its area and
  re-deriving every machine table and schema cell by cell from the originals; then three Opus auditors (every
  table and schema, exhaustively; prose fidelity on sections the first audit did not sample; duplicates,
  history and retirements). The layout check now also reads `rewrite/spec/registry.d/*.toml`, one file per
  agent, so that agents working at once do not edit the same registry file.
- After it is clean: the amend step applies the 104 pass-3 findings, R1 to R5, the slice findings and the 109
  recorded disagreements to the new files, one place each.

**Slice layer 1 finished on 2026-11-24** (nine agents, none failed, about 4.7 hours) and is committed locally as
`486b6835` together with the new root `CLAUDE.md`.
- Built under `next/`: the skeleton, plugin manifest, MCP config, `uv` launcher, root finder and `vsdd` entry
  point; the shared Python core and result schemas with fixtures; the workflow prelude, build step with drift
  check and Node harness; the sample project `tally`.
- Reviews found 67 code problems (6 blockers, 37 major, 24 minor) and all were fixed. Examples: the repository
  `.gitignore` would have silently dropped the plugin's `.mcp.json` from any commit; launch failures gave the
  wrong cause; an unknown hook mode made the gates go quiet; the build step's guard flagged every correctly
  stamped workflow.
- State at the commit, re-run by the assistant: `tools/check.py` 24 of 24, pytest 332 passed and 1 expected
  failure, 21 workflow harness tests passed.
- 55 design findings (12 major, 43 minor) are in `rewrite/design/slice-findings.md`, for the amend step.
- Left for later packages: `uv.lock` (the sandbox cannot reach the package index without `--system-certs`),
  the hook rules, `hooks/hooks.json`, and the `vsdd` subcommands.
- The unit 3 builder read the pass-3 review reports and applied two of their findings in code (limits
  `gate_minutes` and `ci_minutes`, and the build order). The amend step must make the design agree.

**The repair run finished on 2026-11-24** (eight agents, none failed, about 113 minutes). It changed 73 files,
restored 50 dropped things by copying, removed 4 invented values and replaced 16 duplicates with links. The
second audit (`rewrite/spec/conflicts/audit-2-{tables,prose,structure}.md`) says the spec is much closer but
still not fully faithful: 74 findings, 7 blockers (5 distinct, all in machine tables), 34 major, 33 minor.
- Tables and schemas: 33 of 68 machine files are fully faithful. Blockers: the roster says four workflows do
  not call `story` when they do; the artifact table's story row is an excerpt the original calls illustrative;
  the code reviewer's wall leaves `.factory/local` readable; a limit name survives only in a comment; one exact
  block message is lost.
- Prose: 86 of 108 sections compared are faithful; 28 of the first audit's 35 findings are fixed.
- Structure: one fact in one place is mostly not achieved yet. Workflow files restate command files, topics
  restate records and rules, every artifact kind is defined in two files. Most copies agree; three do not.
  About forty history phrases remain. The spec is 1,060 KB in 463 files.
Both checks still pass (layout: no problem; coverage: 181 cited, 39 retired, none uncovered).

**The six originals and all review reports were moved on 2026-11-24** to `rewrite/history/design-v1/` and
`rewrite/history/reviews/` (staged as renames, not committed). `rewrite/design/` no longer exists; a path
such as `rewrite/design/pass3-builder.md` in older text now means `rewrite/history/reviews/pass3-builder.md`.

**Findings ledger (2026-11-24).** `rewrite/tools/findings.py build` reads the raw results saved in
`rewrite/findings/sources/` and writes `rewrite/findings/ledger.json` and one list per build package in
`rewrite/findings/by-package/`. It holds 238 open findings: 104 from review pass 3, 5 left by the pass-2
close-out, 55 from slice layer 1, 74 from the second restructure audit. 10 blockers, 97 major, 131 minor; 111
on the slice path. A finding closes only through an entry in `rewrite/findings/outcomes/*.json`. The 109
disagreements the extractors recorded are in `rewrite/spec/conflicts/` and are not itemised in the ledger.

**Change of plan, decided by the assistant on 2026-11-24 and reported to the user (no answer yet).**
Four rounds of fixing prose have each left about a fifth of what they touched wrong, while slice layer 1,
built straight from the unamended design with a reviewer and tests behind each unit, produced working code and
fixed all 67 problems its reviews found. So the remaining findings are closed where they can be checked:
1. Now: a triage run assigns every finding to the one to four spec files where its fix belongs and to the
   packages that must act on it; a repairer fixes the audit's table blockers and majors; a planner applies
   the slice-path blockers and majors (about 41, the ones that span packages) to their owning spec files; a
   verifier checks them. Script at `rewrite/workflows/slice-amend.js`.
2. Then slice layer 2 is built package by package. Each builder gets its package brief, the files it lists,
   and its open findings from the ledger, resolves each in code with a test, and reports the outcome. When a
   package is built, its spec files shrink to pointers at the code.
3. Findings that are not on the slice path stay open in the ledger against their packages and are closed the
   same way when those packages are built. Nothing is dropped.
The alternative, which the user may prefer, is to keep repairing and amending all of the prose until an audit
is clean before building anything more.

**The slice-amend run finished on 2026-11-24** (seven agents, none failed, about two hours).
- Triage: all 238 findings were assigned to their owning spec files and packages
  (`rewrite/findings/triage/*.json`).
- Tables: 38 table and schema findings fixed.
- Amend: the 41 slice-path blockers and majors were decided: 38 applied to the spec, 3 left for the builder
  because the code under `next/` must change (the `vsdd` entry's exit mapping in `launch.sh`, the event schema
  in `events.py`, and generating every prelude schema literal from schema files). Decisions made, each in one
  owning file: local delivery makes the merge commit in a detached integration worktree, then moves the branch
  with `update-ref`, or with `git merge --ff-only` in the checkout that holds it (the one stated exception to
  never touching the person's checkout; a refusal returns `checkout_dirty`); a step identity carries `method`,
  `loop` and `pass` so the tool can recompute a verdict; the green gate and delivery require a red record; the
  merge guard blocks ref-moving git commands; gates measure a clean worktree only; a story records the commit
  it was cut from as `delivery.base`.
- Verify: 61 of 79 entries confirmed. Not clean: 9 major and 17 minor remain, none on the slice's happy path
  (one story, local mode, no repair). Twelve fixes were reopened as incomplete, mostly prose that still
  restates a repaired table. Four new majors came from the amendment itself (a rebase in pull-request mode
  loses the red record; `ok` is false for a failed CI so the repair loop never runs; the tool and the script
  count different files after a repair; a rule about the caller's worktree widens the spike runner's write
  grant).
- Ledger after it: 256 findings; 64 fixed, 3 left for the builder, 12 reopened, 177 open (2 blockers, 58
  major, 132 minor); 109 active on the slice path. Lists per package are in `rewrite/findings/by-package/`.
  Builders and reviewers add findings by writing `rewrite/findings/sources/extra-<name>.json` and record
  outcomes in `rewrite/findings/outcomes/<name>.json`.

**Slice layer 2, in the order `rewrite/spec/packaging/build-plan.md` ("The thin slice") gives:**
1. done: foundations (layer 1).
2. B8 probes; C2 policy tables; B6 roster and lint. Before these, a catch-up unit brings the layer 1 code into
   line with the amended design (the three left-for-builder findings and the open findings against B3, B4a,
   B5 and C1).
3. S1, S2a, S2b, S4 and the first part of S3; H1, then B4b; G1.
4. G2 and G3; the second part of S3; G4a; H5 and H7.
5. The agents `story.js` names (A1, A4, A6a, the architect from A2b, three from A5), each with only the methods
   the story script gives it.
6. W2, the `story-ready` fixture, then E2's first run: one story, headless.
Each step is its own workflow of fewer than ten agents: a Sonnet builder per unit, an Opus reviewer, a fix.

**Launched on 2026-11-24:** step 2 as two workflows. `rewrite/workflows/slice-layer-2a.js` builds the catch-up
unit, then C2, then B6, each reviewed and fixed. `rewrite/workflows/slice-probes.js` runs the platform probes
the slice depends on (package B8) and writes the results into `rewrite/spec/platform.md`.
**The probe run finished on 2026-11-24** (16 headless sessions; evidence in `rewrite/spike/probes-round-3/`,
with the independent check in `CHECK.md` there). The check supports 13 of 17 results.
- Proven: a hook can rewrite a tool call through `updatedInput` (the nested `hookSpecificOutput.updatedInput`
  spelling), in the main session, a dispatched agent and a workflow step, shown in the auto permission mode;
  every event inside workflow-step subagents carries the same `session_id`; a hook's approval lets a `Workflow`
  call through in the default permission mode (the checker's rerun); matchers fire for MCP tools and `WebFetch`
  (the checker's rerun, main session only); a forked skill's agent can dispatch another agent; two dispatches
  get distinct `agent_id`s.
- Disproven: a missing hook launcher does not block. The platform logs a non-blocking error with exit 127 and
  carries on, so the self-guarding launcher is load-bearing.
- Not established: a ten-minute Bash timeout (the platform's sleep guard refused the test command; package E2
  tests it with a real command), and finding the Claude Code process among a command's ancestors (the chain was
  misread, and it was tried only from a Bash call with the sandbox off), so the offered cut that judges a
  stale lock by process stays unproven.
- Other facts: `WebFetch` refuses `localhost` and upgrades `http://127.0.0.1` to HTTPS, so a plain-http local
  fixture cannot be reached through it; typed workflow arguments reach the script through the model copying
  them; a project `CLAUDE.md` is not loaded when the project is under `/private/tmp`, so `omitClaudeMd` was
  never observed withholding one.
- A correction agent then made `rewrite/spec/platform.md`, `rewrite/spec/topics/platform-proven.md` and
  `rewrite/findings/sources/extra-probes.json` say exactly what the check supports (done 2026-11-24; the
  layout check is clean).

**User instruction, 2026-11-24:** "you should use fable extra high to plan, sonnet extra high to write code, and
opus extra high for review. always use workflows when you can". From step 3 on, every unit of work is shaped
plan (Fable) then build (Sonnet) then review (Opus) then fix (Sonnet), inside a workflow, with model and effort
set on every step. `slice-layer-2a`, already running, uses Sonnet builders and Opus reviewers with no separate
planning step and was left to finish.

**Step 3 planning was launched on 2026-11-24** (script at `rewrite/workflows/slice-step-3-plan.js`): five Fable
planners write one build plan each under `rewrite/plans/step-3/` for S1; S2a, S2b and S4; the first part of S3;
H1 and B4b; G1. One Opus reviewer checks the seams between the plans and against the code, and one Fable
integrator corrects the plans. The plans state which files each unit owns, so that units whose files do not
overlap can be built at the same time. Planners only read `next/`; they do not write there.
**Next action: when `slice-layer-2a` and the planning run have both finished, launch the step 3 build
workflows from the plans (Sonnet build, Opus review, Sonnet fix per unit).**

Ledger tool change (2026-11-24): outcome files now apply in the order they were recorded, kept in
`rewrite/findings/outcomes/ORDER`, so a later fix or a later reopening wins whatever the file is called.
Progress seen on disk while `slice-layer-2a` runs: the catch-up unit has recorded 26 findings fixed and 6 left
for later packages, reported 2 new minor findings, and is in review and fix; C2 and B6 have not started.

Reading taken when the user asked how close the rewrite is (2026-11-24): 7 of 74 packages built; `next/`
passes 24 of 24 checks and 348 tests; the ledger has 262 findings, 90 fixed and 172 to act on. The assistant's
estimate: the first story delivered end to end in two to four days of continuous running, full 1.0 in two to
three weeks, soft until the first end-to-end run. The assistant suggested continuing in a fresh session at the
next step boundary, because this one is very long.

**Next action: when `slice-layer-2a` finishes, rebuild the ledger, run the checks under `next/`, record the
result here, and launch step 3. Nothing beyond the two approved commits is committed.**

Commits so far (local, not pushed): `2a23827b` (`rewrite/`), `486b6835` (`next/` and `CLAUDE.md`). The user
approved those two. The assistant asked whether it may commit at each step boundary and has had no answer, so
`rewrite/spec/`, `rewrite/tools/`, `rewrite/history/` and the later workflow scripts are uncommitted.

The proposal as made: the user asked whether the prose should be broken into
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
| 2d | Review pass 3 (full, read-only) | done 2026-11-24; 104 findings, not yet applied |
| 2e | Restructure the design into one file per thing under `rewrite/spec/` | converted 2026-11-24; audit found it not faithful enough; **repair run running** |
| 2f | Amend step on the new files: pass-3 findings, R1 to R5, slice findings, recorded disagreements | pending a clean repair audit |
| 3a | Thin slice, layer 1: foundations under `next/` | done 2026-11-24; committed `486b6835` |
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


## 2026-11-24, after the compaction: what reported between 12:30 and 18:15

Moved here from rewrite/STATE.md on 2026-11-24 at 18:20, unchanged.

Finished since the compaction:

- **Step 2 build (`slice-layer-2a`), reported 13:10 on 2026-11-24.** Its result is saved in
  `rewrite/findings/sources/slice-layer-2a-result.json`. Its three reviewers found 52 problems (4 blockers, 21
  major, 27 minor), and its two fixers reported every one fixed, but nobody had checked that. The 52 are in the
  ledger (`rewrite/findings/sources/extra-layer-2a-review.json`, ids `CATCHUP-R<n>`, `C2-R<n>`, `B6-R<n>`), and
  the confirmation workflow above records an outcome for each.
- **Step 5 planning (`slice-step-5-plan`), reported 13:20.** `rewrite/plans/step-5/` holds `agents.md` (ten
  agent files, fifteen method files, 35 files in all), `REVIEW.md` (18 findings, all decided), `RESOLUTIONS.md`
  and `RESULT.json`. Its new findings are in the ledger as `S5P-N1` to `S5P-N12`. Two things it settled:
  the step 5 build needs steps 3 and 4 built first, because its tests check that every `vsdd` subcommand an
  agent file names is registered; and the first story run has no spike scenario, because the spike-runner,
  `vsdd spike` (S6) and the spike-bounds rule (H8) are outside the slice, so spikes come when the build widens
  (S5P-N7). Its eight open questions for the gates, the survey and the story workflow are in
  `RESOLUTIONS.md`; the step 4 planners are told to read them.
- **Step 3 planning (`slice-step-3-plan`), reported 14:00.** The seam review found the five plans not
  buildable as written: 32 findings, 3 blockers, all about the three seams S1 owns (command registration, the
  `vsdd.state` facade, the registry check that flush calls). The integrator fixed 31 and left one for the
  user (PL3-21, below). `rewrite/plans/step-3/` now holds seven plans (the parts of H1 and G1 that need S1 and
  the record stores became `hook-runner-b.md` and `jobs-b.md`), `REVIEW.md`, `ORDER.md`, `RESOLUTIONS.md` and
  `RESULT.json`. Scope changes: S1 does not build `vsdd new`, its skeleton templates or `vsdd list` in step 3,
  because nothing on the story path calls them; the records unit ships the seven record schemas the registry
  gave to C1. The planners' open questions (probes for the first headless run, mostly) are at the end of
  `RESOLUTIONS.md`. Ledger entries made from it: `S3P-1`, and corrected outcomes for P3COV-10 and P3SEAM-10.
- **Step 2 confirmation (`slice-layer-2a-confirm`), reported 14:40.** Its result is saved in
  `rewrite/findings/sources/slice-layer-2a-confirm-result.json`. Of the 52 review findings, 42 were confirmed
  fixed at once; the confirmers added 5 new minor problems; a second fixer took the 15 then open and a second
  confirmation passed 10 of them. Five are still open and went to a third round (the workflow above):
  CATCHUP-R10 (major: the toy story closes without `--outcome failed` when a step throws, and AU-P2-12 is
  routed to the wrong packages), B6-R7 (major: the roster lint still lets a hand-written region pin the run's
  tier, and its stripper hides calls inside template literals), C2-R8, L2C-c2-b6-1 and L2C-catchup-2 (minor).
  `rewrite/workflows/slice-fix-open.js` is the script for this: it takes a list of open findings, fixes,
  confirms, and repeats once.
- **Step 3 unit `jobs` (G1), reported 18:00.** Built, reviewed (20 findings, 10 major, no blocker), fixed
  and confirmed twice. Nothing blocker or major is open, so it is a base to build on. Five minor findings
  went to the third fix round above: three in the jobs code (a job lock that an escaped descendant can keep
  holding, a race between two `env start` calls, a refusal message that names the wrong limit) and two that
  are one cleanup of conversion commentary left in design files (AU-S2-12). Its result is saved in
  `rewrite/findings/results/step3-jobs.json`; results of build runs go in that folder from now on.
- **Step 4 planning (`slice-step-4-plan`), reported 17:05.** `rewrite/plans/step-4/` holds `gates.md` (G2),
  `verdicts.md` (G3), `rules.md` (H5, H7), `survey-2.md` (the second part of S3), `delivery.md` (G4a, local
  mode), `REVIEW.md`, `ORDER.md`, `RESOLUTIONS.md` and `RESULT.json`. The seam review found 23 problems (3
  blockers, 8 major); the integrator fixed 22 and left PL4-20 for the user. What the orchestrating session
  must carry from `ORDER.md` there:
  - **Order:** stage A is `verdicts`, `gates` and `rules` together; stage B is `survey-2` alone; stage C is
    `delivery` alone. Stage A starts only when all seven step 3 units have reported with nothing blocker or
    major open and requests R1 and R2 are in their builds.
  - **Requests to step 3 units not yet built (section 5):** R1 for `survey` (`tests/unit/story_ready.py`
    takes `product=`), R2 for `records` (`evidence_dir(project, "_base")` is accepted), R3 and R4 for
    `records` (the delivery keys in the closure schema; the review lookup in one function
    `_review_lookup`). Put them in the `notes` of those two launches.
  - **Approved by the orchestrating session:** the `verdicts` builder makes one edit in the records unit's
    `gate_evidence.py`, so that the review evidence lookup returns the computed verdict (section 4). Step
    3's `ORDER.md` section 7 already gave G3 that job.
  - **Scope changes:** grounding moved from G3 to H7 (the rules unit owns `vsdd/grounding.py`); `vsdd review
    show`, the `decisions` rule and sending notifications are deferred because the story path never
    reaches them; the `merge` decision and `needs_human` wait for G4b.
  - **Corrections to the step 5 plan** (section 7, PL4-23) go into the `notes` of the step 5 launch.
  - **Inputs to W2's plan** (section 7) go into the step 6 planning script.
  - **Probes** P1 to P3 of section 7, with the ones the step 3 and step 5 planners asked for, are being run
    now as round 4 (the workflow above), before stage A, because a wrong assumption there would cost a
    rebuild of `verdicts` and `rules`. The first question decides whether any delivery can pass: what agent
    type a hook sees on a workflow step's result.
- **Step 2 is closed (third fix round, `slice-fix-open` as `layer-2a-fix-3`, reported 17:00).** Result saved
  in `rewrite/findings/sources/layer-2a-fix-3-result.json`. Of the 58 findings its reviews and confirmations
  raised, 57 are confirmed fixed. The last, `layer-2a-fix-3-C1-1` (minor), is left for B6, B5 and W2 by a
  decision of the orchestrating session: the roster lint over hand-written workflow code cannot be made
  complete by patterns (each round closed the spellings named and the next check found others), so the lint
  stays as the first check and the check that counts is behavioural: W2 and each later workflow package
  asserts in its scenario tests that the model and effort of every dispatched step equal the roster's.
- **Feature flag design (`d31-feature-flags`), reported about 16:35.** Its result is saved in
  `rewrite/findings/sources/d31-feature-flags-result.json`. D31 is in the design: `rewrite/spec/decisions.md`
  (D31), `topics/feature-flags.md` (the whole of it), `topics/flag-system-choice.md`,
  `topics/flag-lifecycle.md`, `config/flags.md`, `artifacts/flag.md`, five tables `tables/flag-*.toml`, short
  linked passages in 46 existing files, and six packages F1 to F6 registered in `registry.d/d31.toml` (80
  packages in all; F comes after the workflows and before the proofs; none is on the thin slice). The two
  reviewers raised 50 findings (20 major, no blocker); the integrator fixed 48; the confirmation passed 46 and
  left 11 open, two of them major: the on-state check would refuse the main use, a feature added to existing
  code, because existing tests of replaced behaviour fail with the flag on (D31R-C-2), and the ledger entries
  for the built packages described the design before its corrections (D31R-C-1). Ten went to the second
  correction round above; the orchestrating session closed the eleventh (D31R-F-17) and D31-7.
  `rewrite/workflows/spec-fix-open.js` is the script for a design fix round: Fable corrects, Opus confirms,
  repeated once.
- **Decision D32 is written** (the answer to PL3-21): `rewrite/spec/decisions.md`, `topics/decisions-built.md`
  and `config/hooks.md`. The spec lint now also applies the line limit to files in `registry.d/`, and reports
  a malformed one instead of crashing.
- While several agents work in the tree, `tools/check.py` can fail on a `__pycache__` folder under
  `next/plugins/` left by an agent that imported the plugin's code with plain `python3`. It is not a defect in
  the product. The build and fix scripts now tell agents to run Python with `-B` and to delete such a folder.
- Checks at 14:08, before the step 3 builds started: `tools/build.py --check` clean, 30 of 30 checks, 508
  tests passed with 1 skipped, spec lint clean. A snapshot was taken then (see Commits).

## 2026-11-24, evening: what reported between 18:15 and 22:40

The state of each piece is in `rewrite/STATE.md`; this is the order in which things closed.

- **Step 3 units closed one by one.** `state-core` (30 findings, all fixed), `records` (four fix rounds;
  every finding confirmed fixed, `D33-9` included), `jobs` (31 findings over four rounds, `MAX_WAIT` raised to
  540 after the probe of a ten-minute foreground call; two minor races between overlapping `env start` calls
  left for G1), `jobs-b` and `hook-runner-b` (17 findings fixed; one symlink case left for H4 and the
  reference class for H5). Results are in `rewrite/findings/results/step3-*.json`.
- **The hook runner's approval of a `vsdd` call changed approach (`ORCH-2`).** Four review rounds each found a
  shell construct the parser let through, so approval now also needs the whole raw command text to match an
  allow-list. Rounds 3 to 5 confirmed it (6,150 shell runs of accepted texts ran only `vsdd`). Round 6
  (reported 22:31) pinned the grammar in the tests with an independent checker and a differential test, and
  made the bare `/vsdd` program word accepted as the spec said. One minor is left for H1
  (`step3-hook-runner-fix-6-C2-1`: the tests do not pin non-ASCII characters; the code is right on all).
- **Platform probes, round 4**, run and independently checked; the checker corrected five of eleven readings.
  Applied to `rewrite/spec/platform.md`, `topics/platform-proven-2.md` (items 36 to 49) and the step 4 and 5
  plans. New facts: the Bash sandbox refuses every write under `.git/worktrees/`; a headless run is stopped
  ten minutes after its last turn unless `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS` is set; which git hooks a
  merge runs.
- **Team use (D33)** adopted by the user and written into the design by `design-decision.js`: 56 review
  findings, 52 confirmed fixed, a second correction round launched for the rest. Packages M1 to M6; 86
  packages in all. Decisions from D31 on moved to `rewrite/spec/decisions-later.md`.
- **Feature flags (D31)** closed for now: 62 findings fixed, 12 left for F1 to F6.
- **The `survey` unit** (the last of step 3) was launched after the 21:35 snapshot, built by 22:07 and reviewed by 22:28
  (19 findings, 6 major); its fix pass followed.
- **The session was compacted a second time at 22:30.** After it, the step 6 plan was launched
  (`rewrite/workflows/slice-step-6-plan.js`), to run while step 3 finishes and before steps 4 and 5 are built,
  so that a walk of one story through all the plans finds mismatches before code is written from them.
- **Step 3 is built (23:40).** The `survey` unit reported at 23:34: 19 review findings and 2 from its first
  confirmation, all confirmed fixed. The team-use second correction round reported at 23:30 and D33 is
  closed for now (105 entries: 70 fixed, 35 left for M1 to M6 and the slice units). On a quiet tree:
  `tools/build.py --check` clean, 30 of 30 checks, 1,039 tests passed with 1 skipped, 27 harness tests, spec
  lint clean. Snapshot `rewrite-next-1124-2341.tar.gz`. Stage A of step 4 (`verdicts`, `gates`, `rules`) was
  launched at 23:42.

## 2026-11-25, 00:00 to 07:20: stage A of step 4, the step 6 plan, probe round 5

- **Step 6 planned (reported 00:16).** Two plans (`story-workflow`, `first-run`), a review that walked one
  story through every plan and the built code (16 findings, 1 blocker), corrected. The blocker was in the
  step 5 plan: three agents need `fix-findings`. `rewrite/plans/step-5/ORDER.md` now holds the seven
  corrections that apply over the step 5 plan.
- **`PL6-7`, found by that review, fixed at once** (confirmed 00:50): `tests/unit/story_ready.py` copied the
  sample's own `.factory/` over the project's and classed `tests/**` as source.
- **Probe round 5 (reported 00:52), the gate before the first run: passed in its primary form** on Claude
  Code 2.1.332. Applied to the platform record (items 50 to 55, `topics/platform-proven-3.md`) and to the
  step 6 plans by `probes-5-apply` (reported 01:43). The check corrected the prober on the budget stop and
  found that a refused step call is not in the session result's `permission_denials`.
- **`gates` (G2) built and confirmed (02:57)**, with a third round on test support (04:36).
  **`verdicts` (G3) built and confirmed (03:35).**
- **`rules` (H5, H7): three rounds, a fourth running.** The first two fixed 37 findings. The reviews kept
  finding spellings of a git command that `merge-guard` passes, so the third round (reported 07:05) changed
  how it decides. Its confirmation found both new spellings and ordinary commands blocked, which led to
  decision `ORCH-8`: a text rule blocks what it can positively read, and the guarantee is measured by the
  gates. `ORCH-7` is the open question to the user about a guard at the ref itself.
- **Checks on a quiet tree at 07:16:** 30 of 30, 1,326 tests passed with 1 skipped, both node suites, spec
  lint clean. Snapshot `rewrite-next-1125-0716.tar.gz`. `survey-2` and the last `rules` round were launched
  together at 07:18.
