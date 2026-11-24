# vsdd-factory 1.0 design: pipeline workflows, human stops, convergence and spikes

> Amended by `rewrite/BLUEPRINT.md` on 2026-11-23, and again on 2026-11-24 after the second review pass
> (blueprint section 11). Where this document and the blueprint differ, the blueprint wins.

## Amendments from the blueprint

The text below has been edited in place where a table, a name or a schema changed. This list is the complete set
of changes; where a passage further down still reads differently, this list and section 4 of the blueprint apply.
Items 1 to 19 are the first pass, with the items the second pass changed corrected here. Items 20 to 38 are the
second pass.

1. **Tool name.** The command-line tool is `vsdd`, not `factory`. The prelude source is `workflow-lib/prelude.js`
   and its markers are `// >>> stamped:prelude` and `// <<< stamped:prelude` (SEAM-34).
2. **Agent names and methods.** Every step uses one of the 20 roster agents (blueprint 4.3, 4.4) and names a method
   file. `step()` takes a required `method`; `brief()` writes `Method: <name>. Read <plugin root>/methods/<name>.md
   first.` as the first line, and repeats the step identity and the closing command as the last lines. The survey
   returns `plugin_root`. Clerk steps have no method (SEAM-03, SEAM-04, COV-06, COV-07, OPS-08, OPS-11).
3. **Tiers.** `TIER` and `ROLE` in the prelude are generated from `tests/lint/roster.toml`. `runSpike()` uses the
   spike runner's own tier and always passes `isolation: 'worktree'`. When the survey returns a tier override
   (`VSDD_TEST_TIER`), `step()` uses it (SEAM-15, SEAM-41, COV-17).
4. **Long commands.** Every gate that builds or tests, `vsdd story deliver` and `vsdd release` run their work as a
   detached job keyed by command, subject and commit, and the command itself re-attaches: it returns the result
   or `{state: "pending"}`, and the caller runs the same command again. There is no `--start`, no `vsdd job
   wait` and no job id to carry. A call waits 100 seconds by default and up to 540 with `--wait 540` and a Bash
   timeout of 600000. One clerk covers a whole CI wait; the `ci_pending` loop and the six-wait limit are gone.
   `stallMs` is not an `agent()` option and is not used (COV-03, OPS-01, P2COV-05, P2LEAN-05).
5. **Survey and checkpoint.** The survey checks preconditions before it takes the lock, and refuses when the
   caller marker the hook runner writes for its call is missing. Every script wraps its body after a
   successful survey in `try`/`finally`, so the checkpoint always runs (with `--outcome failed` on an
   exception). `relay()` retries once. `data` is a required field and scripts read only required fields
   (SEAM-22, OPS-09).
6. **Story units.** The survey returns, per unit, `tree` (the root findings are relative to), `diff_patch`,
   `dirty`, `open_pr`, `rounds_used`, `spikes_used` and `review_rules`. The wrapper adds the earlier-work line
   whenever `dirty` is true. A converged verdict is computed from the stored passes of any run, so a new run
   does not review a story again (SEAM-31, SEAM-33, OPS-05, OPS-06).
7. **Budgets.** Each child story receives `limits.agents = story_agents` (default 40) and `step()` returns
   `agent_budget` when it is reached. Before each fix round the wave checks that the units fit beside a reserve
   for its close (COV-18, OPS-26).
8. **Review.** Pipeline reviewers return findings only and write no report file. From the second pass a reviewer
   receives structured claims (`key`, `location`, `resolution`, `commit`) without the fixer's text. Wave
   reviewers receive deferred findings as scope to check. A coverage gap is a finding of category `coverage`
   routed to the test-writer; a coverage item may be `not_checkable` with a reason; the choice `waive-coverage`
   exists. A reviewer that returns blocked opens a `blocked` decision (reason `reviewer`) at once. An arbiter may dismiss only
   a finding the builder disputed. `direct` grants one more round. Gate rounds persist in the closure record and
   count across runs (SEAM-13, OPS-09, OPS-23, OPS-28).
9. **Findings.** The finding schema gains `owner` (the roster agent that should fix it) and the category
   `coverage`; spec and story fixes are routed on `owner`. `location.path` is relative to the unit's `tree`. The
   hook blocks an ungrounded review once and then stores ungrounded findings as advisory; the tool applies the same
   rule. Holdout symptoms have their own shape with no location (SEAM-14, SEAM-33, SEAM-43, OPS-10).
10. **Non-story work.** Every change to the product tree is a story. Plan readiness, verification harnesses,
    benchmarks and documentation fixes are `chore`, `harness` and `docs` stories in `E-0`. Steps that measure are
    clerk steps; since the second pass the measured checks of one commit run as one job (`vsdd gate wave`,
    `vsdd gate release`, `vsdd gate sweep`). `release` prepares and publishes through one `vsdd release` command.
    Story kinds are `feature`, `fix`, `port`, `facade`, `docs`, `chore`, `harness`; `data` is dropped (SEAM-18,
    SEAM-28, OPS-21).
11. **Delivery.** `vsdd story deliver` and `vsdd release` are the only merges. Delivery rebases only when
    the pull request conflicts; a verdict and green evidence follow the `git patch-id`, so a clean rebase keeps
    them and needs a build check only. It adopts an open pull request. A story that waits for a person becomes a
    `merge` decision with the link. A change that adds a dependency, or upgrades one beyond
    `specs/architecture/dependencies.toml`, needs a `merge` decision, batched per `maintain` run (SEAM-09, OPS-13,
    OPS-20, OPS-21).
12. **Routes.** `next` is an object in results and in `vsdd status --json`. Two rows are added: "every runnable
    unit waits on an open decision" above "a wave is open", and "a change request is queued and no cycle is
    open". The driver is the `drive` skill. `vsdd feature route` derives the route; the script no longer computes
    it. The table is an ordered list in the tool's code; the second pass dropped `routes.toml` (SEAM-24,
    SEAM-29, SEAM-40, OPS-05, P2LEAN-13).
13. **Decisions.** The grammar is `/vsdd-factory:decide <id> <choice> [key=value ...] [-- note]`, captured on the
    prompt-expansion event through the `decide` skill. `ideas` is one decision with a list per choice. An
    amendment run unlocks the whole specification set for the planner roles, and the next `spec` decision
    approves the amended set. The cycle is opened by `vsdd checkpoint brief`, by the `discover` assess
    checkpoint, by the `feature` survey, or by `maintain` when it delivers fixes, and closed by the `release`
    checkpoint. Decisions of `ingest`, `discover`, `maintain` and spikes live under
    `cycles/_standing/` (SEAM-21, SEAM-23, SEAM-37, SEAM-44, COV-01).
14. **Stages added.** `spec`: a `sources` argument with an `intake` stage, and a `scenario-review` step by the
    `scenario-reviewer`. `ingest` with `as: reference`: `disposition` and `port-analysis`. `discover` and the
    post-release sweep: a `collect` clerk step (`vsdd discover collect`). `feature` fast routes: `vsdd gate plan
    --story` after the story file is written; since the second pass the story-writer runs it as its closing
    command (SEAM-12, SEAM-27, COV-08, COV-09, COV-19, P2LEAN-03).
15. **Multi-repository.** The hub model of `state.md` 2.14 applies: one state directory, one cycle, stories that
    carry `repo`. `repo` selects a `[[repo]]` entry. Section 2.9.11 is rewritten (SEAM-07).
16. **Spikes.** Every agent may request one, and the runner inherits the asker's walls; the asker's agent type
    is read from a hook-written record, never from the request (item 30). The wrapper refuses before starting
    an agent when the run's limit is used, carries findings across `gated()` attempts, and the tool
    deduplicates by a hash of the normalised question within the unit. The slash command stays the `spike`
    workflow; there is no spike skill (SEAM-06, COV-11, OPS-27).
17. **Stage registry.** Withdrawn by the second pass. There is no `stages.toml`: the scripts are the one source of
    stage structure, and section 2.9.13 gives the agent and method of every step (SEAM-40, P2LEAN-06).
18. **Limits.** The survey returns limits under the prelude's names (blueprint 4.12); a harness test fails on an
    unknown key (SEAM-36).
19. **Open questions.** Section 6 is superseded by section 2 of the blueprint, whose recommendations the user
    adopted on 2026-11-23.

Second pass (2026-11-24):

20. **The done test.** A stage is done when the unit's closure record (or, inside a run, a done step record)
    says so, the artifacts its result listed still validate, and its gate has passing evidence for the current
    commit. There are no input digests and no `stale` state. A stage reopens only when a person's `revise` or
    `hold`, or an amendment, marks it not done (section 2.5; P2LEAN-06).
21. **Routes and preconditions.** The route table is scoped to the open cycle, reordered, and gains rows. Each
    workflow has a named list of preconditions. Both are code in the tool, each row with a fixture (sections
    2.4 and 2.5; P2SEAM-06, P2COLD-23, P2COLD-24, P2REQ-16, P2LEAN-13).
22. **Decisions.** Nine kinds: `spike-limit`, `environment`, `stuck` and `review-blocked` are one kind,
    `blocked`, with a reason. Every choice uses named values and text after `--`. An identifier is qualified as
    `<id>@<cycle>` when two cycles are open. `merge` choices have defined effects (section 2.6; P2LEAN-08,
    P2SEAM-13, P2OPS-08, P2COLD-25, P2COLD-29).
23. **Stories written after the survey** are admitted by the tool: `vsdd gate plan --story` and `vsdd story
    new-fix` record the story in the run, create its branch and worktree, and return the story unit (sections
    2.9.1 and 2.9.4; P2COLD-01, P2REQ-01).
24. **Fewer clerk steps.** The four planners finish a step with the check for their own work. Measured checks
    on one commit run as one job, and the gate names are 11 (sections 2.9.2 to 2.9.10; P2LEAN-02, P2LEAN-03).
25. **Amendments inside an open cycle**, and a second cycle that carries `cycle` in its `next` (section 2.6;
    P2COLD-08, P2REQ-06, P2REQ-07).
26. **Waves and decisions.** A story's decision holds back only its dependents; a wave gate's own decision
    blocks the waves after it; the route and the survey share one function (section 2.9.2; P2OPS-04).
27. **The holdout wall.** The scenario review is a separate loop. Claims carry no quote. Findings built from
    scenario scores have a defined shape and are exempt from grounding. Walled findings are stored in the
    holdout class (sections 2.7, 2.9.3 and 2.10; P2SEAM-02, P2SEAM-07, P2COLD-07).
28. **Frozen tests can be fixed.** The test-writer records changed frozen tests with `vsdd gate red
    --refreeze` (section 2.9.1; P2COLD-04).
29. **Existing code.** `ingest` records a measured baseline, writes conventions, review rules, the dependency
    list and the toolchain file, and ends at a `spec` decision. Gates compare against the baseline. "First
    release" is defined. Plan coverage applies to the cycle's own contracts. A project with no tests or CI gets
    readiness chore stories on every route (sections 2.9.4 to 2.9.8; P2COLD-03, P2COLD-12, P2REQ-02, P2REQ-03,
    P2REQ-17).
30. **Spikes.** The asker's type comes from hook-written records. The command forms are fixed. The runner opens
    a named spike itself. The `spike` workflow takes no run lock and needs an initialised project. Children
    receive a share of the run's spikes (section 2.8; P2COV-01, P2SEAM-04, P2COV-08, P2OPS-18, P2REQ-10,
    P2REQ-11, P2REQ-12).
31. **Budgets.** `runStories()` is one stamped function with a reserve check; a failed child is charged its
    budget (section 2.5; P2COV-04).
32. **Free text** reaches the tool as a percent-encoded argument, and clerk commands use `vsdd -C <dir>`
    (section 2.5; P2COLD-06, P2OPS-02).
33. **Model policy.** `ingest` map and analyze run on the builder tier through the roster's `[overrides]`;
    phase metadata names no model (P2COLD-10, P2REQ-13).
34. **Multi-repository.** Per-repository wave records and integration worktrees; one story per repository on a
    fast route (section 2.9.11; P2REQ-05).
35. **`maintain`** runs its measured sweeps as one job, names a method for every sweep, and opens a maintenance
    cycle when it delivers fixes (section 2.9.10; P2COV-03, P2REQ-16).
36. **Records.** There is no status record and no stored verdict record; `vsdd review verdict` is a function of
    the tool, not a command (P2LEAN-04).
37. **Skeletons.** Section 2.11 is corrected in place to the amended behaviour (P2OPS-20).
38. **Step results.** The platform probe passed, so there is no `vsdd step record` fallback; the hook stores
    the calling agent's type with every step record (P2SEAM-03, P2LEAN-10).

This is one of five subsystem designs for step 2 of the rewrite (see `rewrite/STATE.md`). It covers the
workflows that run the pipeline, the points where a person must decide, the rule that ends review loops, and
research spikes. Agent names, the command-line tool, state paths and hook rules belong to the other four
designers. Where this design depends on them the name used here is provisional and the exact need is stated in
section 5.

## 1. Summary

In 1.0 the pipeline is twelve JavaScript workflows in `<plugin>/workflows/`: `ingest`, `discover`, `spec`,
`feature`, `plan`, `wave`, `story`, `verify`, `release`, `maintain`, `multi` and `spike`. Each starts with a
survey step that asks a deterministic command what is already done, runs only the remaining stages, and ends
with a checkpoint step that records state and names the next command. Nesting is one level: only `story` is
ever called by another workflow. Scripts steer, tools measure and hooks enforce, so every gate is a measured
fact with an evidence record, and merging, closing a wave and publishing are refused without it. Routine human
stops fall from 37 to three (brief, specification, release). They are workflow boundaries, recorded from the
person's own keystroke, and other decisions queue while work continues around them. Convergence needs one
clean pass by a fresh reviewer on the final commit, at each of three perimeters (story, wave, release), with a
cap of three passes, one arbitration, then a person. Any step may return a spike request; a wrapper stamped
into every script runs the experiment in an isolated worktree within limits and restarts the step with the
finding.

## 2. The design

### 2.1 Terms

| Term | Meaning |
|---|---|
| Workflow | A JavaScript script in `<plugin>/workflows/<name>.js`, run as `/vsdd-factory:<name>`. |
| Stage | A named part of a workflow that produces one set of artifacts. A stage that is already done is skipped. Stages are the unit of resume. |
| Step | One agent run inside a stage. Every step returns a step result (section 2.10.1). |
| Clerk step | A step that runs one `vsdd` command and returns its JSON output unchanged. For a long command it runs the same command again while the output says `pending`. It makes no judgment. |
| `vsdd` | The plugin's deterministic command-line tool (this document first called it `factory`). It measures facts, writes state and evidence, and performs git and pull-request operations. Section 4.9 of the blueprint is the command table. |
| Survey | The first step of every workflow: a clerk step running `vsdd survey <workflow>`. It returns what is done, what is not, and the limits and configuration for the run. |
| Checkpoint | The last step of every workflow: a clerk step running `vsdd checkpoint <workflow>`. It closes what the evidence supports, opens decisions, saves state and returns the next command. |
| Gate | A fact measured by `vsdd gate <name>`, for example "the new tests fail" or "the suite passes on this commit". |
| Evidence record | The file a gate writes. It names the gate, the commit, the result and the measurements. Only the tool writes it. |
| Step record | The stored copy of a step result. The hook runner writes it when the step returns. |
| Unit | The thing a workflow works on: a story, a wave, a specification set, a release candidate. |
| Position | How far a story has got: `ready`, `red`, `green`, `reviewed`, `evidenced`, `delivered`. |
| Perimeter | The scope at which review happens: `spec`, `plan`, `story`, `wave`, `release`. |
| Pass, round | One fresh review of the current commit at a perimeter. At the wave and release perimeters a round is one run of every check that is due. |
| Finding | One defect reported by a reviewer or a gate, in the shape of section 2.10.2. |
| Decision | A question only a person can answer. It is stored with a packet (what is being decided, the evidence, the choices). |
| Route | The order in which workflows run for a given project state. The tool computes it and reports it as `next`. |
| Integration branch | The branch stories merge into. Its name comes from project configuration (the old system called it `develop`). |
| Tiers | planner = Fable at extra-high effort; builder = Sonnet at extra-high effort; reviewer = Opus at extra-high effort; clerk = Sonnet at low effort, used only for clerk steps. |

### 2.2 The rule that makes gates enforceable: scripts steer, tools measure, hooks enforce

A workflow script cannot read a file or run a command. Everything it knows arrives in a step result, and a
step result is something an agent said. The old system had the same weakness everywhere: an agent announced
that a gate had passed, and another agent believed it. The design therefore gives each layer one job.

| Layer | Its job | What it must not be trusted with |
|---|---|---|
| Script | Order the work: sequence, fan out, loop, branch on result fields. | Truth. A script gate is flow control only. |
| Agent | Judgment: write a spec, write code, find defects. | Reporting whether its own work passed. |
| Tool (`vsdd`) | Measure: run the project's build and test commands, inspect git, check artifacts, compute verdicts, and write the evidence record. | Judging quality. |
| Hook runner | Refuse an action the evidence does not support, and keep agents inside their information walls. | Starting work. |

Two consequences follow, and the rest of the design relies on them.

1. An agent that produces something finishes by running the gate command for it and returns the gate's JSON
   in its result. The script branches on that. At the point of no return (merge, wave close, opening a
   decision, publish) the tool or a hook checks the evidence record again and ignores what the script
   believes. A fabricated result can waste a pass. It cannot merge code.
2. Build, test, lint, format, run, mutation, scan, benchmark and proof commands come from project
   configuration. No workflow, agent prompt or gate names `cargo` or any other toolchain.

### 2.3 The workflow set

| Workflow | Purpose | Arguments (all optional unless marked) | Calls | Ends at |
|---|---|---|---|---|
| `ingest` | Read an existing codebase and record what it is: module map, recovered contracts, architecture, conventions, the dependency list, the toolchain, security and verification gaps, and a measured baseline. | `source` (path or URL; default the project), `as` (`project` or `reference`), `guidance` | none | Decision `spec` on the recovered baseline (`as: project`); a report (`as: reference`). |
| `discover` | `scan`: research opportunities and rank ideas. `assess`: market check for one idea, brief or request. | `mode` (`scan` or `assess`), `subject` (idea id, brief path or text) | none | Decision `ideas` (scan) or `brief` (assess). |
| `spec` | Turn an approved brief into the specification set and review it to convergence. | `sources` (paths of existing documents to take in), `guidance`, `variant`, `only` (stage names) | none | Decision `spec`. |
| `feature` | Take a change request for an existing product: analyse the delta, choose the route, then either deliver one story or evolve the specification. | `request` (required on first run, unless `amend`), `intent`, `severity`, `guidance`, `amend` (run the amendment the open cycle is waiting for), `cycle` | `story` | Fast routes: nothing, next is `verify`. Standard route and amendment: decision `spec`. |
| `plan` | Decompose the approved specification into stories, schedule waves, assign hidden scenarios, prove the toolchain and CI work. | `guidance` | none | Nothing by default; next is `wave`. |
| `wave` | Build one wave: deliver its stories in parallel, then run the wave gate to convergence. | `wave` (default: the next open wave) | `story` | Nothing; next is the next wave or `verify`. |
| `story` | The one per-story playbook: failing tests, implementation, review to convergence, evidence, merge. | `story` (required) | none | Nothing, or a queued decision if blocked. |
| `verify` | Check the release candidate: full suite and scans, system review, security, formal proofs, hidden scenarios, performance, UI, docs. | `only` (check names), `cycle` | `story` | Decision `release`. |
| `release` | Ship the authorised commit, record the new baseline, write the retrospective. | `retro` (`false` to skip), `cycle` | none | Done. |
| `maintain` | Run maintenance sweeps and deliver the fixes that are mechanical. | `sweeps` (names), `report_only` | `story` | A report; when it delivered fixes, next is `verify` for the maintenance cycle. |
| `multi` | Coordinate a project that spans several repositories. | `stage` | `story` | Per-repository commands, or decisions `spec` and `release` at project level. |
| `spike` | Run one research spike for a person, or open, advance or close a named spike. | `question`, `kind`, `ref`, or `open` / `close` with a name | none | A finding. |

Every workflow also accepts `repo`, the name of a repository in a multi-repository project (section 2.9.11).

Arguments as text. A person types text after the command, and whether the platform hands a script that text
as a string or as an object is not yet proven (blueprint section 10, item 31). Every script therefore passes
`args` through one prelude function, `parseArgs()`, which accepts an object unchanged and parses a string by a
small grammar: a `key=value` word sets a named argument only when `key` is one of that workflow's own argument
names (the table above), and every other word is part of the main argument. A request such as "make
timeout=30 configurable" therefore keeps `timeout=30`. So
`/vsdd-factory:feature CSV export drops the last row intent=bug-fix` sets `request` and `intent`;
`/vsdd-factory:spike what is the cold start of uv` sets `question`; `/vsdd-factory:spike open uv-startup --
measure cold and warm start` opens a named spike; and `/vsdd-factory:spike close uv-startup` closes it. The
harness runs every workflow with its documented argument text.

Free text never goes into a command line raw. A script cannot write a file and a clerk's command takes no
standard input, so the prelude function `enc()` percent-encodes the text (`encodeURIComponent`) and the tool
decodes it: `vsdd survey feature --request-uri <encoded>`, `--guidance-uri`, `--question-uri`. A lint test
checks that every free-text value in a `relay()` command went through `enc()`. `safe()` is for identifiers
only.

Nesting rule: `wave`, `feature`, `verify`, `maintain` and `multi` may call `story`. Nothing else is ever
called as a child, and `story` contains no `workflow()` call. A static test enforces both.

Why these twelve and not one per old file. The old phases 4, 5, 6 and 7 become one workflow, `verify`. They
check the same commit, share one fix loop and have no human decision between them, so splitting them only
forced the checks to run in sequence and made a person or the main session bounce between four commands. The
old greenfield, brownfield, feature and planning files are not workflows at all in 1.0. They were sequences
of phases, and a sequence is now a route computed from project state. STATE.md says "each phase is its own
workflow". This design keeps that with the 1.0 phase list: Ingest, Spec, Plan, Build (waves), Verify, Release.
That reading is flagged in section 6.

### 2.4 Routes: how the old modes run now

A route is not a script. `vsdd status` computes the next command from what is on disk, the session
briefing prints it, and every workflow returns it. The table shows the route each old mode becomes. "Stop"
marks a human decision.

| Old mode or path | 1.0 route |
|---|---|
| Greenfield | brief (interactive skill) → stop `brief` → `spec` → stop `spec` → `plan` → `wave` for each wave → `verify` → stop `release` → `release` |
| Brownfield | `ingest` → stop `spec` (the recovered baseline) → the person states intent → `feature "<request>"` (delta against the approved baseline), or a brief and then the greenfield route. A project imported from the old plugin starts at `feature`: the importer writes the baseline and an approved `spec` decision |
| Feature, standard (F1 to F7) | `feature` → stop `spec` (delta) → `plan` → `wave` for each wave → `verify` (delta scope) → stop `release` → `release` |
| Feature, trivial | `feature` (delta analysis, one story delivered) → `verify` (suite and traceability only) → stop `release` → `release` |
| Feature, bug fix | `feature` (delta analysis, one fix story with a regression test) → `verify` (suite, scoped hidden scenarios, one delta review, security if the module is critical or high, affected proofs) → stop `release` → `release` |
| Feature, critical bug | As bug fix, with `severity: critical`: the story branches from the last release tag, `verify` runs the suite and one delta review, and `release` merges the fix back into the integration branch |
| Maintenance | `maintain` on a schedule or on demand |
| Discovery | `discover` → stop `ideas` → brief or `feature` for each idea pursued |
| Planning (adaptive front end) | No workflow. Detection and routing are the route table; existing documents are taken in by the `intake` stage of `spec`, through its `sources` argument |
| Multi-repository | `multi` at the project root, and the routes above with `repo` for each repository |
| Per-story delivery | `story`, as a child or on its own |

Route table. The tool evaluates it top to bottom and the first match is `next`.

| Project state | `next` |
|---|---|
| The project is not initialised | human: run `/vsdd-factory:setup` |
| A run lock is held by a live run | human: wait, or inspect the run |
| The root holds a multi-repository manifest and project-level work is due | `multi` |
| No cycle is open and a change request is queued | `feature` with the oldest queued request |
| A second cycle (a critical fix) is open and has work that can run | That cycle's next workflow, with `cycle` in `args` |
| No cycle is open, an `ingest` has finished and its `spec` decision is open | human: that decision |
| No cycle is open, source exists, and there is no baseline (no ingest, import or release) and no approved brief | `ingest` |
| No cycle is open, the brief exists as a draft | human: continue `/vsdd-factory:brief` |
| No cycle is open and none of the above | done (steady state: write a brief, or `feature`, `maintain`, `discover` on demand) |
| A cycle is open: a `spec-gap` decision was settled `amend` and nothing else in the open wave can run | `feature` with `amend` |
| A cycle is open: a change request is in flight and neither delivered nor awaiting a decision | `feature` |
| A cycle is open: the brief is approved and the specification is not converged, or its decision asked for revision | `spec` |
| A cycle is open: the specification is approved and `[commands]` has no `test` or `test_report` | human: the lines to add to `factory.toml` (this arises only when the approved toolchain file did not supply them) |
| A cycle is open that needs a plan (the `route` in its `cycle.json` is `standard`, which a cycle that began with a brief has from its opening): the specification is approved and the `plan` unit is not closed. Or, in any open cycle, an amendment was approved and the plan has not been re-traced | `plan` |
| A cycle is open: every unit that could run waits on an open decision | human: the list of decisions |
| A cycle is open: a wave is open, or a later wave has runnable stories | `wave` |
| A cycle is open: all waves closed (or, in a cycle with no plan, which is one whose route is `trivial`, `bugfix` or `hotfix` or whose kind is `maintenance`, every admitted story delivered) and `verify` has not converged on the current commit | `verify` |
| A cycle is open: decision `release` is approved for the current commit | `release` |
| A cycle is open: only open decisions remain | human: the list of decisions |

Every row below the first three is scoped: it names whether a cycle is open. After a release closes the cycle,
no `verify` or `release` row can match, so the answer is "done" or the queued request, never `verify` again.
A project that was ingested or imported and has no brief is in steady state, and a queued request starts
because its row sits directly below the lock and multi-repository rows, above every row that would send a
person away. "Could run" in the two decision rows is the same function the survey's preconditions use, so the
route never names a command the survey would refuse. A story waiting on its own decision does not stop later
waves; a wave whose gate is waiting on a decision does (section 2.9.2). The `plan` unit is closed when a
`plan` run ends with the plan gate passed, and a `revise` on a `plan` decision reopens it. "The plan is
ready", the precondition of `wave`, means that unit is closed and no `plan` decision is open. A cycle's route
is the `route` field of its `cycle.json`, a tracked record: `standard` from the opening of a cycle that began
with a brief, and written by `vsdd feature route` for a cycle a change request opened (section 2.9.4). So a
fresh clone, or a resume after the run records are pruned, routes the same way, and a cycle whose change
request has no route yet matches neither the `plan` row nor the no-plan case of the `verify` row.

The table is an ordered list of (condition, next) in the tool's Python, beside the condition functions. Only
the tool evaluates it, and each row has a fixture. Six fixtures are named because they failed before: a
brownfield project with a queued request gives `feature` with that request; a greenfield project after its
release gives done; a standing `ideas` decision plus a queued request gives `feature`; a fast route whose
story is delivered gives `verify`, not `plan`; a maintenance cycle with delivered fixes gives `verify`; a
standard-route feature whose delta specification is approved, with no run records on disk, gives `plan`.

The driver. A person can type each command, because the briefing always shows the next one. For low touch
the plugin ships a small driver, the person-invoked `drive` skill: it runs `vsdd driver on`; reads `next`; if
it is a workflow, starts it with the Workflow tool; when it returns, reads `next` from its result; and repeats
until `next` is a human decision or done, or until a run returns `progress: false` twice in a row. At a human
decision it runs `vsdd driver pause` with the decision's id and stops; when the person settles that decision
in the same session, the context the `decide` hook returns tells the main session to resume the loop, so the
person does not type `/vsdd-factory:drive` again after each approval. At done, or after two runs without
progress, it runs `vsdd driver off`. It lives in
the main session because a workflow may nest only one level. After a compaction the briefing tells the main
session that the driver is active or paused and how to continue. The driver only steers. If the main session starts the wrong workflow, that workflow's survey refuses,
so enforcement does not depend on the driver behaving. With the driver, a greenfield project needs three
human actions after the brief conversation: approve the brief, approve the specification, authorise the
release.

### 2.5 The common shape of every workflow

Every workflow is the same three parts.

1. Survey. One clerk step runs `vsdd -C <dir> survey <workflow> [--subject <id>] [--repo <name>] --json`. The
   command first checks that the hook runner is alive for this very call (the caller marker the hook wrote for
   it; blueprint 4.9) and that every precondition is met. Only then does it commit any pending state, open a
   run record, take the run lock, and make the worktrees and diff patches the units need. It returns the
   survey object: project directory, plugin root, the run id, preconditions with whether each is met, the
   units and their positions, each stage name recorded as `done` (a stage not listed is `todo`), the limits
   and thresholds from project configuration, open and settled decisions, and `next`. If a precondition is
   unmet no lock is taken and the script returns at once with the reason and `next`.
2. Stages. The script runs each stage that is not `done`, in order, in parallel where the stage table says
   so. Every step goes through the `step()` wrapper (section 2.11.1), which sets model and effort from the
   tier table, retries an unusable result, serves spike requests and counts agents.
3. Checkpoint. One clerk step runs `vsdd checkpoint <workflow> --run <id> --json`. The command reads the
   step records and evidence of the run, closes the units the evidence supports, opens a decision packet for
   each blocked unit, saves state, releases the lock, removes expired spike worktrees and returns `status`,
   `progress`, the decisions it opened and `next`. That object is the workflow's return value, so the main
   session receives a few hundred bytes per run.

How a workflow knows what is already done. The script never reads state. The survey command does, and it is
code. A stage is `done` when three things hold: the unit's closure record says so, or, inside a run that has
not reached its checkpoint, a step record with status `done` exists for the stage and unit; the artifacts
that result listed still exist and pass their structural check; and any gate the stage must pass has a
passing evidence record for the current commit (the survey measures the gate again when only the evidence
record is missing). The tool keeps no list of stages. It records the stage names the scripts report, and each
script skips the names the survey returns as done; the stage tables in section 2.9 are the specification the
scripts are written from. Nothing reopens by itself: a stage is marked not done only when a person settles
`revise` or `hold` (from the stage named by `stage=`, or from the review stage by default) or when an
amendment is opened. A person who edits a draft artifact by hand between runs names the stages to run again
with the workflow's `only` argument. Running the same command in a new session therefore continues the work.
In the same session the platform's own resume replays finished steps from its cache, which is faster but is
not relied on.

Preconditions. Each workflow has a named list, evaluated by the survey before the lock is taken and by the
route's "could run" test, in one implementation with a fixture per predicate.

| Workflow | Preconditions, beyond "the project is ready and the hooks are alive for this call" |
|---|---|
| `spec` | A cycle is open; decision `brief` is approved |
| `feature` | A baseline exists (a release, an ingest whose `spec` decision is approved, or an import). With `amend`: a cycle is open and a `spec-gap` decision was settled `amend` |
| `plan` | A cycle is open; the specification is approved |
| `wave` | The plan is ready; the baseline check passed; every earlier wave is closed; `[commands]` is configured |
| `story` (standalone) | The specification is approved; the story is admitted (it is in an open wave, or a fast route or a fix round admitted it); its dependencies are delivered; `[commands]` is configured |
| `verify` | A cycle is open; every wave is closed, or the cycle has no plan (its route is `trivial`, `bugfix` or `hotfix`, or its kind is `maintenance`) and every story admitted to it is delivered |
| `release` | Decision `release` is approved for the verified commit |
| `ingest`, `discover`, `maintain` | None beyond the common one. `maintain` runs report-only while a product cycle is open |
| `multi` | The project layout is `hub` |
| `spike` | None beyond the common one, and it takes no lock |

"The specification is approved" has one meaning everywhere: the latest `spec` decision, whether it came from
`spec`, `feature`, `ingest` or an import, is approved and its digest matches the specification set as it
stands.

What every step is given. In this order: the method to read, as an absolute path; its task in one or two
sentences; the unit's identifiers and file paths, never file contents (for a reviewer: the tree root, the
diff patch, the evidence paths and the review-rules path); the command it must finish with, if any; the
findings or structured claims it must act on; spike findings already obtained for the step; and, last, the
step identity it must echo back and the closing command again, so both survive a compaction of the step. Agents read the files
themselves, and the walls in section 5 decide what each agent type can open.

What is on disk at each step boundary.

| Boundary | On disk afterwards | Written by |
|---|---|---|
| A producing step returns | Its artifacts: files, and commits on the unit's branch | The agent |
| | The evidence record for the commit it left | `vsdd gate`, run by the agent as its last act |
| | The step record | The hook runner (need H1) |
| A review step returns | The step record holding its findings, claim checks and coverage | The hook runner |
| A clerk step returns | Whatever the command writes: run record, worktrees, evidence, fix stories, the pull request, the merge | The tool |
| The workflow returns | Decision packets, closed units, saved state, released lock, `next` | `vsdd checkpoint` |

Nothing that matters lives only in a script variable. Where the script computes something that later code
must trust (a convergence verdict), the tool recomputes it from the stored records with the same rule, and a
fixture test keeps the two implementations in step (need P4). The route exists once, in the tool.

Limits. Every loop is bounded, and the survey supplies the bounds from project configuration.

| Limit | Default | Enforced by | At the limit |
|---|---|---|---|
| Agents per run | 900 (the platform stops at 1,000; a parent and its children share the platform's counter) | Script counter | Stop starting work, return `partial`; the same command continues |
| Agents per child story | 40 | The child's own counter in `step()` | The story returns blocked with kind `agent_budget` |
| Stories per `wave` run | 24 | Script | `partial` |
| Attempts per step (unusable result, or the step's gate fails) | 2 | Script | Unit `failed`, or blocked with kind `gate` |
| Review passes: story, spec | 3, plus 1 after arbitration | Script; the tool recomputes | Section 2.7 |
| Review rounds: wave, verify | 3, plus 1 after arbitration | Script; the tool recomputes | Section 2.7 |
| Review passes: plan, ingest | 2 | Script | Open findings go to the packet or report |
| Delivery rounds per story | 3; each CI wait is one job of at most `delivery.ci_wait_minutes` (30), served by one clerk | Script and tool | Blocked |
| Gate run time | `limits.gate_minutes` (60), waited in slices of nine minutes | Tool | The gate result is `ok: false` |
| Fix stories per `maintain` run | 10 | Script | The rest are reported |
| Spikes per step, per run, per day | 2, 6, 20 | Script and tool | Section 2.8 |
| Spike time box | 20 minutes | Hook | Section 2.8 |
| Runs in which a unit failed without a human-decidable reason | 2 | Tool | Decision `blocked` with the reason `stuck` |
| Token budget, if the person set one | none | Platform `budget` | `partial` |

Failure handling. `agent()` returns `null` when a step dies or is skipped; the wrapper retries once and then
marks the unit `failed`. A failed unit does not stop its siblings. A `partial` or `failed` run is repaired by
running the same command again, because the survey starts from disk. After a successful survey every script
runs its body inside `try`/`finally`, so the checkpoint runs on every path, including an exception
(`--outcome failed`), and the lock is always released.

Child stories and the shared budget. `wave`, `verify`, `feature`, `maintain` and `multi` all start child
stories through one stamped prelude function, `runStories(units)`. Before it starts a batch it checks
`RUN.agents + units.length * story_agents + CLOSE_RESERVE <= limits.agents` and, if the batch does not fit,
starts the units that do and reports the rest, so the caller returns `partial`. Each child receives
`limits.agents = story_agents` and its share of the run's remaining spikes (at least one) with the count
already used. A child that throws or returns nothing is charged its full `story_agents`, because it may have
spent them. The platform's 1,000-agent counter is shared and hard, so this check is what keeps the reserve
for the checkpoint, which releases the lock and pushes state.

Concurrency. Once preconditions pass, the survey takes a lock scoped to the repository and branch the run
writes. A second run that
would write the same branch is refused. `discover`, `maintain` with `report_only` and `spike` take no lock: a
spike writes only its own worktree and the ledger, so one can start while a wave runs. A critical
bug fix takes a separate lock because it works from the release tag, on the branch `hotfix/<version>`.

### 2.6 Human stops

A human stop exists only where a person must decide. It is always a workflow boundary: the workflow's
checkpoint opens a decision, returns, and nothing waits inside a running script.

Routine stops, three per product cycle.

| Decision | Opened by | Why only a person can decide | Choices and what follows |
|---|---|---|---|
| `brief` | The brief skill, or `discover` in `assess` mode. Either way the checkpoint that opens the decision also opens the cycle. | What to build, for whom, and whether it is worth the money. The market assessment is in the packet. | `approve` → `spec`. `revise -- <guidance>` → the brief is reworked. `drop`. |
| `spec` | `spec`; `feature` on the standard route or as an amendment; `multi` for a project-level specification | The specification is the contract for what the product will do. Accepting it, choosing between design variants, and deciding whether to split into repositories are product and organisation decisions. For an empty repository the packet also shows the toolchain lines (`[commands]`, `[paths]`) that approval will write into `factory.toml`. | `approve [variant=<id>] [topology=single or split]` → the specification set is locked, next is `plan`. `revise [stage=<name>] -- <guidance>` → `spec` or `feature` runs again from that stage (by default from the review stage) with the guidance. `drop`. |
| `release` | `verify` | Shipping to users. The person accepts the residual risk shown in the convergence report: waivers, dismissed findings, advisory counts, scenario scores. | `approve` → `release`. `hold -- <guidance>` → `verify` runs again with the guidance. |

The grammar is `<id> <choice> [key=value ...] [-- note]` for every choice in this section. Values are named,
a key may repeat, and free text follows `--`. There are no positional values and no quoted strings.

Conditional stops, present only when configured or when the situation arises.

| Decision | When | Why only a person can decide |
|---|---|---|
| `ideas` | End of `discover` in `scan` mode | Which opportunities to pursue is strategy. One command settles the list: `select pursue=<ids> research=<ids> defer=<ids> reject=<ids>`. Rejected and deferred ideas are remembered so they are not proposed again. |
| `merge` | A story touched a protected path (by default `.github/**`, `.env*`, `CODEOWNERS` and the files `[release]` names); it adds a dependency or upgrades one beyond what `specs/architecture/dependencies.toml` and earlier approved `merge` decisions list; or project configuration sets `delivery.merge = "human"`. The packet carries the pull request link. In `maintain`, dependency changes are batched into one decision per run. `approve`: the next `vsdd story deliver` for that story performs the governed merge, and a dependency the decision names is recorded in the decision record as approved, so the next story that uses it does not ask again. `reject`: the next survey closes the pull request and the story is descoped. | The project owner has reserved these changes for human review. |
| `plan` | Project configuration sets `stops.plan = "always"`, or the plan's estimate exceeds `stops.plan_ceiling_agents` (default 600) | Authorising the spend. |
| `spec` (the recovered baseline) | End of `ingest` with `as: project` | Whether the recovered contracts, architecture and toolchain describe the system well enough to build on. Approval locks them as the specification and, where `[commands]` or `[paths]` are unset, writes the recovered toolchain into `factory.toml`. It is one decision, once per ingested project. |

Exceptional stops. These are queued, not blocking: the blocked unit waits, and the route continues with every
other unit that does not depend on it. The one exception is a wave whose own gate waits on a decision, which
holds the waves after it (section 2.9.2).

| Decision | Raised when | Choices |
|---|---|---|
| `finding` | A review loop reached its cap with an upheld blocking finding or an uncovered item, or a critical or security finding that an agent may not dismiss | `waive key=<key> [key=<key> ...] -- <reason>` (risk acceptance), `waive-coverage item=<item> [item=<item> ...] -- <reason>`, `direct -- <guidance>` (exactly one more round with the guidance), `descope` |
| `spec-gap` | A reviewer showed that the approved specification is silent or wrong on a point the story depends on | `amend` (the gap joins the amendment run described below, which ends at decision `spec`), `not-a-gap -- <reason>`, `descope` |
| `blocked` | A unit cannot continue. The reason is in the record, the packet and the briefing line: `reviewer` (a reviewer returned blocked, for example because it declined the material or an input was missing; opened the first time it happens); `spike_limit` (a step still needs an experiment after the spike limits); `environment` (a credential, access right or tool is missing and cannot be supplied by an agent); `stuck` (the unit failed in two runs for no reason a person was asked about) | `retry` (also "I have supplied it" for `environment`), `direct -- <guidance>`, `descope`. For `spike_limit` also `open-spike name=<name>` (a named, human-led spike). For an optional environment check also `skip`. |

The four reasons were four decision kinds in the first pass. They are one kind because their choices are the
same; the identifier is `blocked.<subject>`.

Which decision a blocked step becomes. The checkpoint maps the stored result, not the script. A review loop
that ended at its cap opens `finding`. A review loop whose verdict reason is `reviewer_blocked`, or a step
blocked with kind `reviewer_blocked`, opens `blocked` with the reason `reviewer` at once, the first time. A
step blocked with kind `spec_gap` opens `spec-gap`; `spike_limit` opens `blocked` with the reason
`spike_limit`; `environment` opens `blocked` with the reason `environment`. A step blocked with kind `gate`,
`conflict` or `other`, and a unit that `failed`, open nothing the first time: the next run retries, and the
second failure opens `blocked` with the reason `stuck`. Kind `agent_budget` is not a decision; the run
returns `partial`.

Amendments inside an open cycle. A specification gap is always found while a cycle is open, so the amendment
cannot be a queued request. When a person settles a `spec-gap` decision with `amend`, the settle function
writes an amendment request for the open cycle (`requests/CR-NNN.md` with `cycle` and `amendment = true`).
The stories that wait on the gap stay blocked. When nothing else in the open wave can run, the route returns
`feature` with `amend: true`. That run is not queued: its survey takes every `spec-gap` decision settled
`amend` in the cycle as its request, so one run covers the wave's gaps, holds the amendment that unlocks the
specification set for the planners, and ends at decision `spec`. When the amended specification is approved,
`plan` runs in delta scope: the story-writer re-traces the stories the change analysis names, the plan gate
runs again, and the waiting stories are released into the open or a trailing wave.

A second cycle. A critical bug fix may open a second cycle while the main one is open. Its `next` carries
`cycle` in `args`, and `verify`, `release` and `feature` accept `cycle`. Where both cycles hold a decision
with the same identifier, each is written `<id>@<cycle>` (for example `release@hotfix-1.2.1`); the briefing
prints that form and a bare ambiguous identifier is refused with the choices. Its stories are cut from the
release tag and delivered to the branch `hotfix/<version>`. When it is released, the merge back into the
integration branch waits for the integration lock; a conflict opens a `merge` decision.

How a decision is recorded, and why an agent cannot forge one.

- A decision is opened only by `vsdd checkpoint`, which is code. For the brief, the interactive brief
  skill runs `vsdd checkpoint brief` when the person says the draft is ready. The packet is a file a
  person can read: what is being decided, the evidence, the choices, and the recommendation where there is
  one.
- A decision is settled only by a person. They type
  `/vsdd-factory:decide <id> <choice> [key=value ...] [-- note]`, which is the person-only `decide` skill. The
  hook runner sees the typed command on the prompt-expansion event and writes the decision record itself,
  before any model reads the message (need H5). A person can also run `vsdd decide` in their own terminal,
  through the shim that `setup` installs. Every agent is blocked from that command and from writing the
  decision store.
- An approval is bound to a digest of what was approved: the specification file set for `spec`, the commit
  for `release`. The brief, which has its own decision, and the two guidance files, `conventions.md` and
  `review-rules.md`, which have their own lock, are outside that set (`state.md` 2.8). Approval changes no
  file, so the digest is over the files exactly as they are. If the content changes afterwards the decision
  reads as `stale` and the stop returns. In a session, settling writes the decision record and nothing else;
  the next survey commits it.
- The approved specification set is locked as a whole: while the latest `spec` decision is approved and no
  live run holds an amendment, no file in the specification classes can be written, whether or not it carries
  a status. It changes only inside a `feature` or `spec` run that holds an amendment, and that run ends at
  decision `spec` again, whose packet lists each changed file. An accepted decision record stays frozen even
  under an amendment. Specification gaps found during a wave are batched: the stories wait, the wave
  finishes, and one amendment run covers them all, inside the open cycle (above).
- When a decision opens, the tool notifies through the channel in project configuration, and the session
  briefing lists open decisions.

What happened to the old 37 approval steps is in section 3.5.

### 2.7 Convergence

Why the old rule could not end. It required three consecutive passes with nothing above a nitpick, and any
finding reset the count. A fresh reviewer told to find flaws nearly always finds something. If a pass is
clean with probability one half, three in a row takes fourteen passes on average, and the cap was ten. At the
cycle level each fix also created new bookkeeping for the next pass to fault, so the finding count never fell.

The 1.0 rule has four definitions and one loop. It is the `judge()` and `converge()` code in section 2.11.1
and is used unchanged at every perimeter.

Severity. Reviewers assign one of four levels, defined so that a level can be checked against evidence.

| Severity | Definition |
|---|---|
| `critical` | The change breaks a stated contract in normal use, loses or corrupts data, opens an exploitable security hole, or breaks the build or the suite. |
| `high` | A stated contract clause, acceptance criterion or listed edge case is unmet or untested, or a test cannot fail. |
| `medium` | A real defect with a concrete consequence that no contract covers. |
| `low` | Style, naming, wording, preference. |

Blocking finding. A finding blocks only when all of these hold. The script applies the first three and the
hook or tool applies the fourth.

1. Its severity is `critical` or `high`.
2. It is anchored: it cites the contract clause or acceptance criterion it violates (`spec_ref`), or gives a
   command or scenario that shows the defect (`repro`). A severe finding with no anchor is advisory.
3. Its scope is within reach of the perimeter. A story review reaches `unit` findings. A wave reaches `unit`
   and `integration`. Release reaches those and `system`. A finding beyond reach is deferred to the perimeter
   that can judge it and does not block here. A finding whose scope is `spec` cannot be fixed by a builder and
   becomes a `spec-gap` decision.
4. It is grounded: the file exists, the line range is inside it and the quoted text is found there. An
   ungrounded finding is rejected when the reviewer returns it (need H2).

Clean pass. A pass is clean when every reviewer in it returned a result for the current commit; every item
the reviewers were told to account for (each acceptance criterion at a story, each changed file after a
repair, each contract at the specification) is marked `checked`, or `not_checkable` with a reason when
another evidence route covers it; there is no blocking finding (an item left `skipped` is itself a finding of
category `coverage`, routed to the test-writer, so the fixer is called and the loop can end); and every claim
from the previous pass ("this was fixed", "this is disputed") was checked and came back resolved or withdrawn.

Converged. A unit has converged when its latest pass is clean and nothing has changed since. One clean pass
is enough. It must be the last word: any later commit that touches source or tests needs a scoped pass over
the new commits before the unit counts as reviewed.

The loop.

1. Run a pass with fresh reviewers. From the second pass they receive the list of claims to verify and
   nothing else from earlier passes. A claim is structured data only: the finding key, the path and line
   range of its location (never the quote), the resolution (`fixed` or `disputed`) and the commit. A claim
   about a finding of category `holdout` carries the key alone. The fixer's free text stays in the step record
   for the arbiter and is never shown to a reviewer.
2. If the pass is clean, stop: converged.
3. Otherwise hand the blocking findings to the fixer. For each one the fixer either fixes it or disputes it
   with evidence. Advisory findings are handed over once, to fix where cheap, and never extend the loop.
4. Repeat until the cap (three passes at story and specification, three rounds at wave and release, two at
   plan and ingest).
5. At the cap, an arbiter rules on each open finding: `defect`, `not_defect` or `spec_gap`. The arbiter is
   the architect at the planner tier, a third role that neither wrote the code nor raised the finding.
   A `not_defect` ruling dismisses the finding only if the builder disputed it, and never if it is `critical`
   or in the security category. Defects that remain get one more fix and one more pass. The number of passes
   and rounds used is stored in the unit's closure record, so the cap counts across runs.
6. If findings are still open, the unit is blocked and a `finding` or `spec-gap` decision is opened. The
   pipeline carries on with other units.

At the specification perimeter there is no arbiter, because a person is about to read the packet anyway:
findings still open at the cap are listed in it as open issues with both positions.

Why it terminates. The loop runs at most cap plus one passes, each pass is a fixed number of steps, and each
step is bounded by the attempt and spike limits. A story review therefore uses at most 4 passes, 4 fixes and
one arbitration.

Why one clean pass is enough. The old rule asked for three clean looks at the same perimeter. This design
gets at least three independent fresh looks at every change before release, one at each perimeter, each with
a wider view: the story review against its contracts, the wave review of how stories fit together, and the
system review in `verify`. Stories in critical or high modules get two lenses in every story pass (adversary
and security reviewer). Repeating a look at the same perimeter mostly re-finds the same things; widening the
view finds different ones.

How a finding can leave the blocking list. By a fix that the next reviewer confirms. By a dispute that the
next reviewer accepts (it withdraws the finding). By an arbiter's `not_defect` ruling, which needs the
builder's dispute first, so two independent agents have disagreed with the reviewer. By a person's waiver.
There is no other way, and every dismissal is listed in the release packet.

Checks that are not reviews. At the wave and release perimeters a round also contains measured checks
(suite, mutation score, scans, proofs, scenario scores). A failing check produces findings in the same shape,
so one loop handles both. The hidden-scenario thresholds carry over from the old system and live in project
configuration: mean satisfaction at least 0.85, every must-pass scenario at least 0.6, standard deviation
below 0.15, and an 80 percent rotation chosen by the tool from a seed derived from the commit. Mutation
thresholds by module criticality also carry over: 95, 90, 80 and 70 percent.

Findings built from scenario scores. The evaluator returns scores and symptoms, never findings. The tool
(and `holdoutFindings()` in the prelude, held to it by fixtures) builds the findings, and their shape is
fixed so that they block. One finding per must-pass scenario under its floor: key `holdout:<HS id>`. One
aggregate finding when the mean or the spread misses its threshold: key `holdout:<unit>:mean` or
`holdout:<unit>:spread`. Each has category `holdout`, severity `high`, scope `integration` at a wave and
`system` at release, `spec_ref` set to the contracts the scenario cites (the tool resolves them, because it
may read the scenario), `repro` set to the scenario id and the rotation seed, the evaluator's symptom as
`observed` and `expected`, and no `location`. `title` is a fixed sentence built from the key and the score
("Hidden scenario HS-014 scored 0.50, under its floor of 0.60"), never the scenario's own title. `judge()`
and its Python copy exempt category `holdout` from the location condition and from grounding, because there
is no file to point at and the scenario path must not appear. Without that exemption a failing must-pass
scenario would be advisory and the hidden-scenario gate would be silently off. Two conformance fixtures hold
it: a must-pass scenario at 0.5 blocks, and a mean of 0.8 blocks.

Walled findings. A finding returned by the `scenario-reviewer`, or any finding whose location lies under
`.factory/holdout/`, quotes or names a hidden scenario. Such a finding is walled: its full form is stored
only in the `holdout-evaluations` class. Its key holds the scenario's path and clause, so everywhere else it
is named by its walled key: `walled:` and the first 12 hex characters of the SHA-256 of the key, which is as
stable across passes and reveals nothing. Decision packets, closure records, claims and `vsdd review show`
carry the walled key and the severity and nothing else for a reader who may not read `holdout`, and a waiver
names the walled key. The scenario review therefore runs as its own convergence loop (section 2.9.3), with
its own claims, routed only to the `product-owner`. A universal harness test asserts that no prompt sent to
an agent that is denied `holdout` contains text from a walled finding.

One text for severity. The severity table above is the file `workflow-lib/severity.md`; the build stamps it
into every reviewer file, so the byte-for-byte lint has one source. The blueprint's section 4.10 reproduces
it.

What is dropped from the old convergence document, and what replaces it.

| Old mechanism | 1.0 |
|---|---|
| Three consecutive clean passes, minimum three, maximum ten | One clean pass on the final commit, cap three, arbitration, then a person |
| Novelty score, similarity by embeddings, convergence index, decay curve, cost-benefit formula | Dropped. None was implementable without embeddings and cost data, and none ended a loop. A finding key that returns after a fix is the only repeat signal, and it goes to arbitration at the cap |
| Zero findings on a first pass is suspicious; rerun | Coverage accounting: a clean first pass is accepted only if every required item is marked checked |
| Reviewer confidence trend and hallucination fingerprints | Grounding check on every finding when it is returned, and the dispute path |
| Autonomy levels 2 to 4 with a composite score | Two switches in project configuration: `delivery.merge` and `stops.*` |
| A person may override any assessment | Kept: `waive` and `hold` |

### 2.8 Research spikes

A spike is an isolated, bounded experiment that answers one question. There are two sizes of the same
protocol.

- A quick spike is one run of the `spike-runner` agent in a throwaway worktree. Agents, workflows and people
  can all start one, and no person has to approve it.
- A named spike is a human-led effort that lasts across sessions. It follows the user's existing convention:
  branch `spike/<name>`, worktree `.worktrees/spike-<name>/`, and the ledger files `.spike/STATE.md` and
  `.spike/TASKS.md` inside that worktree.

The protocol, the same from every entrance.

1. Ask. Someone states one question, why it blocks, and optionally a method and a commit to start from.
2. Open. The spike runner's first command is `vsdd spike open`. The tool first establishes who asked, from a
   record the hook wrote and never from the runner's word (below), and refuses if it cannot. It then checks
   the ledger. If the same question already has a finding in the same unit, and the runner that produced it
   worked under a read-deny set that covers this asker's, it returns that finding and the spike ends. If a
   limit is reached it refuses. Otherwise it writes a ledger entry: question, the asker's agent type, the
   effective read-deny set, the runner's agent id and working directory, the run, the starting commit, the
   time box.
3. Experiment. The runner works only inside its own worktree. Pipeline gates such as tests-before-code do
   not apply there. It cannot push, merge or write anywhere else.
4. Record. The runner's last command is `vsdd spike record`, which validates the finding against the
   schema and appends method, result, verdict and the worktree path to the ledger entry. The hook runner
   refuses to let a spike runner finish without a ledger record (need H6): for a workflow step it blocks the
   `StructuredOutput` call, and for a runner dispatched with the Agent tool it refuses the stop. The platform
   probe showed that a refused stop does not hold a step that has already returned a schema result, which is
   why the step case uses the result call.
5. Return. The finding goes back to whoever asked.

Who asked. The asker's agent type decides which read walls the runner inherits, so it must be a fact and not
a claim.

| Entrance | Where the asker comes from | The runner's opening command |
|---|---|---|
| A workflow step returned `needs_spike` | The stored step record of the asking step. The hook wrote it and stored the calling agent's type with it; it also holds the question | `vsdd spike open --run <id> --step <workflow>/<subject>/<name>` |
| The `spike` workflow, started by a person or a skill | The run record, where the survey stored the question it was given as `--question-uri`. The asker is the main session | `vsdd spike open --run <id>` |
| A dispatch with the Agent tool | An admission the hook records at `PreToolUse` on `Agent` when the target is the spike runner (the dispatcher's agent type), and attaches to the runner when it starts | `vsdd spike open --question-file <path>`, a file the runner writes in the temporary directory |

`--asker-type <type>` is accepted as a cross-check only: a value that disagrees with the record is refused.
`vsdd spike record --json-file <path>` and `vsdd spike close --name <name> --json-file <path>` take the
finding as a file the runner writes in the temporary directory. No spike command reads standard input, so
each is a single command the hook can approve.

The three entrances.

| Who decides a spike is needed | How it starts | What handles it |
|---|---|---|
| An agent running as a workflow step | It returns status `needs_spike` with a `spike_request` in its step result. It cannot dispatch an agent, and it is told so in every step prompt. | The `step()` wrapper: it runs the spike as a step of the same workflow and starts the asking step again with the finding. This works in a parent and in a child, because the spike is a step and never a third workflow. |
| An agent dispatched in a normal session | It dispatches `vsdd-factory:spike-runner` with the Agent tool. | The spike runner's own file sets `isolation: worktree`. The open and record commands and the hooks apply the same limits and ledger. |
| A person, or a skill on a person's behalf | `/vsdd-factory:spike <question>` | The `spike` workflow, which calls the same `runSpike()` function. There is no spike skill. The main session on its own judgment dispatches the spike agent directly (the second row): in the platform probe the model never started a workflow unprompted. |

Every spike runs in its own worktree, research spikes included, so each runner has its own working directory
and its own ledger entry, matched by agent id. A workflow step passes `isolation: 'worktree'` itself, because
the platform ignores the agent file's setting for a step. Every agent type may ask. The ledger entry records
the asker's agent type, taken as the table above says, and the hook runner gives the spike runner the asker's
read walls in addition to its own, which is what makes a request from the `code-reviewer`, the
`holdout-evaluator` or the `scenario-reviewer` safe. Every agent except the spike runner and the clerk has
the Agent tool, limited by the hook to the spike runner, so in a normal session a walled reviewer asks by
dispatching the runner itself. Nobody runs a spike "on behalf of" a walled agent, because that would record
the wrong asker.

This improves on the reading in STATE.md in three ways. First, the slash command is a workflow and not a
skill. A skill would ask the main session to keep a ledger; the workflow gets isolation, the schema and the
limits from code, shares one implementation with the wrapper, and adds no model-visible skill to every
agent's fixed context. Second, limits and the ledger are enforced by the open and record commands, which
every entrance must pass through, so the direct-dispatch entrance is bounded too. Third, the request carries
`ref`, the commit the experiment starts from. A platform worktree starts from the main checkout's head, which
does not contain the asking story's work, so the runner checks out `ref` first.

The shared wrapper. The JavaScript is `brief()`, `runSpike()` and `step()` in section 2.11.1. Scripts cannot
import, so the build stamps that block into every workflow file, and a test fails if any copy differs from
the source (need P2). In short: `step()` runs the agent; if the result asks for a spike and the limits allow,
it calls `runSpike()`, appends the finding to the list the step is given, and runs the step again; if the
limits do not allow, it returns the step as blocked with kind `spike_limit`.

Limits.

| Limit | Default | Enforced by | When exceeded |
|---|---|---|---|
| Spikes per step | 2 | The wrapper | The step returns blocked, kind `spike_limit` |
| Spikes per run | 6 | The wrapper (per script) and `vsdd spike open` (per run id, so parent and children share it). A parent passes each child its share of what remains (at least 1) and the count used, so the child's wrapper refuses before it starts an agent | As above |
| Spikes per project per day | 20 | `vsdd spike open`, from the ledger | The open command refuses; the finding's verdict is `refused` |
| Time box | 20 minutes | The hook runner: after the time box every tool call by that spike runner is refused except `vsdd spike record` | The runner records what it has, usually `inconclusive` |
| Nesting | None | The spike finding schema has no spike request, and the hook runner refuses a spike runner that dispatches agents | Not possible |
| Questions per spike | 1 | Schema | Not possible |

A blocked step with kind `spike_limit` becomes a `blocked` decision with the reason `spike_limit` at the
checkpoint. The packet holds the questions asked and the findings so far, and the person can open a named
spike (`open-spike name=<name>`), give direction or descope.

How the finding returns to the step that asked. The asking agent stops when it returns `needs_spike`; any
partial work stays on disk. The wrapper starts a new agent for the same step with the same task, plus the
findings, plus a line saying earlier work may exist. A finding is small (section 2.10.5): verdict, answer,
method, up to ten evidence lines, confidence, and the paths of anything the runner built. The asker may read
those paths as reference. Nothing from a spike worktree reaches the product branch except by being rebuilt
through the story playbook, tests first.

Ledger and retention. One ledger per project holds every spike from every entrance (need S6). A worktree the
platform kept because it has changes is listed in its ledger entry. `vsdd checkpoint` removes spike
worktrees older than 14 days and those whose asking unit has been delivered, so no agent step is spent on
cleanup.

Named spikes. `/vsdd-factory:spike open <name> -- <mission>` runs one spike-runner step. The runner's first
command is `vsdd spike open --name <name> --run <id>`, which creates the branch, the worktree and the two
ledger files, or, when the named entry is already open, attaches this runner's agent id to it. Attaching is
never answered from an earlier finding and does not count against `spikes.max_open`; creating does count. The
runner then advances the mission by one bounded increment inside the named worktree, updates the ledger
files, and ends with `vsdd spike record --increment`, which the stop check accepts for a named entry: a named
spike has no verdict until it is closed. Running the same command again continues from the ledger, in any
session. A person can also work in the worktree by hand. `/vsdd-factory:spike close <name>` records the final
finding in the project ledger. Named spikes are listed in the session briefing.

Where a spike can run. The `spike` workflow takes no run lock, so it is available while any other workflow
runs. It does need an initialised project: hooks are silent before setup, so no bound could be enforced
there, and the workflow answers "run `/vsdd-factory:setup` first". The user's own `resume-spike` convention
keeps working in any repository and can resume a named spike, because the branch, worktree and ledger file
names are the same.

### 2.9 Workflow specifications

Each table lists stages in order. "Agent (tier)" names the provisional agent type. "Gate" is the condition
the script tests on the result; `g(r, "x")` means the entry of `r.gates` whose `gate` is `x`. "Done when" is
what the stage's result must show for the survey to count it done (section 2.5). These tables are the
specification the scripts are written from; there is no separate stage registry. Section 2.9.13 gives the
agent and the method of every step in every workflow.

#### 2.9.1 `story`

Purpose: deliver one story. Arguments: `story` (required), `repo`. When called by a parent it also receives
`position` (the story unit), `run_id`, `project_dir` and `limits` (with `agents` set to `story_agents`, and
the child's share of spikes), and skips its own survey and checkpoint.

Where a child's unit comes from. A story that was planned gets its unit from the parent's survey. A story
written after the survey (the one story of a fast route, a readiness chore, a fix story from a gate) gets it
from the command that admits it: `vsdd gate plan --story <id> --run <id>` for a story the story-writer just
wrote, and `vsdd story new-fix` (or the `fix_units` of a failing `vsdd gate wave`) for fix stories. Admission
checks the story, records it in the run and in the cycle's closure records, creates the branch `story/<id>`
from the cycle's base (the release tag, on `hotfix/<version>`, for a critical fix) and the worktree, all
under the parent's lock, and returns the full story unit of section 2.10.8. A parent may pass only a unit the
tool returned to this run: every gate and `vsdd story deliver` check that the story is recorded in the run
they are given, that its worktree exists at the recorded path and that its branch descends from the recorded
base, and refuse otherwise. A garbled unit therefore stops at the first gate and delivers nothing.

| Stage | Runs when the position is before | Step and agent (tier) | Given | Returns | Gate | Checked again by |
|---|---|---|---|---|---|---|
| survey | (standalone only) | `survey`, clerk | Story id | Survey | Every precondition met (section 2.5): the specification is approved, the story is admitted, its dependencies are delivered, `[commands]` is configured | The command refuses otherwise |
| tests | `red` | `tests`, test-writer (builder) | Worktree, story file path, contract ids, the red-gate command | Step result with the red gate | `r.status == "done" && g(r,"red").passed && g(r,"red").sha == r.head`. On failure one retry with the gate's failure text, then blocked | Hook: source files in the story worktree cannot be written until red evidence exists |
| implement | `green` | `implement`, the agent and method the story's kind selects (implementer, technical-writer, devops or verifier; all builder) | As above, and the green-gate command | Step result with the green gate | Same test with `green` | Hook: tests frozen at the red gate are read-only to builders. Tool: delivery needs green evidence for the head commit |
| review | `reviewed` | `review-<lens>-p<k>`: adversary, plus security-reviewer when the story's module is critical or high or the story is flagged security (reviewer). `fix-p<k>`: the finding's `owner`, which is the test-writer for findings of category `test` or `coverage` and otherwise the implementer (builder); when a pass has both, the test-writer runs first, and the implementer also follows it when a re-frozen test fails on the current code. `arbitrate`: architect (planner) | The method, the tree root, the latest diff patch path (from the survey for the first pass, then the `diff_patch` of the last green gate result), the test output path, the review-rules path, story file, contract ids, the acceptance criteria to account for, the structured claims from the previous pass. A fixer is given the findings inline | Review data; fix data; ruling data (section 2.10.8) | `converge()` returns `converged` | Tool: recomputes the verdict from the stored passes before it merges, counting a pass only when the stored agent type is the reviewer that owns the pass's method |
| evidence | `evidenced`, only when the story has user-visible acceptance criteria | `evidence`, demo-recorder (builder) | Story file, the criteria to show, the demo-gate command | Step result with the demo gate | `g(r,"demo").passed` | Tool, at delivery |
| deliver | `delivered` | `deliver-<n>`: clerk running `vsdd story deliver`. `repair-<n>`: implementer (builder). Then a scoped review of the new commits | Story id | Delivery data | `d.state == "merged"` | Tool: refuses to merge without green evidence on the head, a converged verdict covering the head, the demo gate and green CI. Hook: agents cannot merge or push to the integration branch any other way |
| close | (standalone only) | `close`, clerk | Run id | Close | none | |

Details.

- The red gate passes when the stubs build, the new tests fail on assertions and not on build errors, and at
  least half of the non-exempt new tests are red. The exemptions carry over: tests that are green by design
  and wiring tests. The gate reads per-test results from the JUnit report named by `commands.test_report`. New
  tests are the test identities missing from the report of the base commit. The gate records the identity and
  file hash of every new test; those are the frozen tests. Which file is a test file follows the class
  precedence of blueprint 4.7: a path that matches a tests glob is in `tests` even when a source glob also
  matches, so the tests in `src/parser/tests.rs` count and the test-writer may still edit that file after
  red. A test defined in a file classed as `source` does not count.
- Fixing a frozen test. A review finding of category `test` (for example "this test cannot fail") is the
  test-writer's to fix, and the hook lets it change a frozen test. The green gate would then fail on "frozen
  tests unchanged". So the test-writer's fix step ends with `vsdd gate red --refreeze --story <id>`, which
  only the test-writer may run: it requires every changed frozen test to still exist and to either pass or
  fail on an assertion, as the red gate does, and records the new hashes as a re-freeze record on the head
  commit. A strengthened test that now fails has found the defect the weak one hid, so in the same fix pass
  the implementer follows the test-writer and makes it pass, ending with the green gate. The green gate
  compares against the latest red or re-freeze record. A harness scenario covers a weak-test finding fixed
  this way, with a strengthened test that fails until the implementer changes the code.
- The green gate passes when the configured build, full suite, lint and format commands pass on the head
  commit, no stub marker remains, the frozen tests are unchanged since the latest red or re-freeze record,
  and for a UI story the configured UI check passes. On a project with a baseline (section 2.9.8), "pass"
  means no new failure: a test, lint finding or format finding that the baseline lists does not fail the
  gate, and anything else does. Its result carries `diff_patch`, the path of the patch it wrote for the head.
- For a story with no user-visible criteria the demo gate is satisfied by the tool itself: it builds the
  table of acceptance criterion to test to result at delivery, with no agent.
- `vsdd story deliver` does everything mechanical, as one detached job keyed by the story that a single
  clerk waits on by running the same command again: push, open the pull request with a body rendered from
  the story and its evidence and a trailer naming the story (or adopt one that is already open), wait for
  CI, merge under a file lock, remove the worktree. It rebases only when the pull request conflicts with the
  integration branch. The merge passes `--match-head-commit` with the current head, and only when that
  head's patch id equals the one the verdict covers. Its `state` is one of `merged`, `awaiting_merge` (a
  person must approve; the checkpoint opens a `merge` decision with the link, and once it is approved the
  next `vsdd story deliver` for the story merges), `ci_failed`, `conflict`, `review_stale` (the story's
  patch changed since the converged review) or `refused` (evidence missing). Its data carries `diff_patch`
  for the range from the reviewed commit to the head, which the scoped review after a repair is given. With
  `delivery.mode = "local"` (no remote) it merges locally and skips the pull request and CI. With
  `delivery.ci = "none"` (the repository has no CI, recorded by `vsdd init`) it does not wait for checks;
  only then does "no checks" count as a pass.
- The verdict and the green evidence are bound to the head commit and to the `git patch-id` of the story's
  diff. After a clean rebase the patch id is unchanged, so both still hold and the tool runs the build
  command only. After a repair, or when a rebase changed the patch, the review loop runs once more with a
  cap of 2 over only the new commits.
- Story kinds change single stages, not the playbook (blueprint 4.6). `fix`: created from findings, with a
  regression test among its criteria. `facade` (service clones, fakes): there is no tests stage and no red
  gate, and the clone's fidelity to the recorded behaviour of the real service is measured at the wave gate
  by the fidelity harness. `port`: the tests are behavioural-equivalence tests against the reference
  implementation (method `equivalence-tests`). `docs`: starts at `red`, the implement step uses technical-writer and the
  green gate runs the documentation checks. `chore` (scaffold, CI, toolchain configuration): starts at `red`
  and the implement step uses devops. `harness` (proofs, fuzz targets, benchmarks): starts at `red` and the
  implement step uses verifier. A story's optional `method` key picks another method of the same agent, for
  example `data-migration`.
- Review findings beyond a story's reach are returned to the parent in `deferred`.
- Delivery never force-pushes the integration or release branch. After a rebase it pushes the story's own
  branch with `--force-with-lease`. It never skips git hooks, and its commits carry no AI attribution.

Ends: `done` with the merged commit; `needs_human` when a person must merge; `blocked` with a decision
opened at the checkpoint; or `failed`, which the next run retries.

#### 2.9.2 `wave`

Purpose: build one wave. Arguments: `wave` (default: the next open wave), `repo`.

| Stage | Step and agent (tier) | Given | Returns | Gate |
|---|---|---|---|---|
| survey | `survey`, clerk | Wave id or none | Survey: each story of the wave with its position, worktree, criticality, flags and any settled decision | Preconditions (section 2.5): the specification is approved, the plan is ready, the baseline check passed, every earlier wave is closed, `[commands]` is configured |
| stories | `story` as a child through `runStories()`, one per story that is not delivered, deferred or waiting on an open decision, all in parallel | The unit from the survey | Story result | None here. If the run could not start every story (story or agent limit), checkpoint and return `partial` |
| gate | `integration-r<k>`: clerk running `vsdd gate wave`, which for a UI wave runs the UI check as part of the same job. Then in parallel: `integration-review-r<k>`, adversary (reviewer); `security-r<k>`, security-reviewer (reviewer), when the wave has critical, high or security stories; `holdout-r<k>-b<i>`, holdout-evaluator (reviewer), one per batch of up to `holdout.batch` (12) scenarios; for UI waves `ux-review-r<k>`, ui-reviewer (reviewer) | Integration worktree, commit range of the wave, story ids, the gate's `diff_patch`, findings deferred by the story reviews, claims | Wave gate data; review data; holdout data | `converge()` at the `wave` perimeter returns `converged` |
| fix (inside the gate loop) | `new-fix-r<k>`: clerk running `vsdd story new-fix`, which writes the fix stories and admits each (section 2.9.1). Then `story` as a child for each fix story, through `runStories()` and its reserve check | The round number | Units; story results | A finding counts as fixed when its fix story was delivered |
| close | `close`, clerk | Run id | Close | The command closes the wave only if the evidence supports it |

Details.

- `vsdd gate wave` runs the suite, lint and build on the integration head in a worktree the tool owns
  (`.worktrees/_integration/`), so the person's own checkout is never switched. It also runs mutation testing
  for stories that carry the requirement and the fidelity check for service clones the wave touched. With
  `--settle` it first moves stories that wait on a person to a later wave, together with their dependents,
  when no delivered story depends on them, creating a trailing wave if this is the last one; it reports any
  story that still blocks the gate in `pending`. When the measured gate fails, its output already carries the
  fix units, so no separate `new-fix` clerk is needed in that case. When everything left in the wave waits
  on decisions the run returns `needs_human` and the route names them.
- From the second round, reviews are scoped to the fix commits and the claims, and scenarios are re-run only
  if a fix changed product source.
- Scenario scores become findings in the shape section 2.7 fixes: one high finding per must-pass scenario
  under its floor (key `holdout:<HS id>`), and one aggregate high finding if the mean or the spread misses its
  threshold (key `holdout:<wave>:mean` or `:spread`), with no location and exempt from grounding. The
  evaluator's text for a failing scenario describes the symptom and must not quote the scenario; the tool
  checks that before a fix story is written from it.
- A wave-level demo is no longer part of the gate. It can be switched on in project configuration.
- In a multi-repository project the gate, its rounds and its integration worktree are per repository
  (section 2.9.11).

Ends: `done` (next is the next wave or `verify`); `partial`; or `needs_human` with decisions queued.

Waves and open decisions. One rule, written here, in the route table and in blueprint 4.11.

- A story that waits on its own decision (`finding.<story>`, `merge.<story>`, `spec-gap`, `blocked.<story>`)
  does not hold its wave: `--settle` moves it, with its dependents, to a later or trailing wave, the wave
  gates and closes without it, and the next wave starts with every story that does not depend on it.
- A wave whose own gate waits on a decision (`finding.<wave>`, opened when the gate loop reached its cap)
  is not closed, and no later wave starts, because nothing more should be built on an integration head that
  has failed its gate. The route's first answer is then "human" with that decision; it does not return
  `wave` for a survey that would refuse.

The survey precondition "every earlier wave is closed" and the route's "could run" test are one function. A
scenario test holds the second case: a wave gate at its cap gives a route of "human" with that decision on
the first try.

#### 2.9.3 `spec`

Purpose: produce and review the specification set from an approved brief. Arguments: `guidance`, `variant`,
`only`, `repo`. Stages run in the order listed, except that assessments, scenarios and ux run in parallel
once the architecture is settled.

| Stage | Step and agent (tier) | Produces | Done when |
|---|---|---|---|
| survey | clerk | Survey; preconditions: a cycle is open and decision `brief` is approved. When the latest `spec` decision is approved (an earlier cycle's, an ingest's or an import's), the survey marks the run as holding an amendment, as `vsdd feature route` does for the standard route, so the planners can write the set that approval locks | |
| intake (only with `sources`) | product-owner, and architect for architecture documents (planner), method `intake` | Existing documents named by the `sources` argument, converted into registry artifacts. The step returns `data.covers`, the names of the later stages whose output it filled | Artifacts valid. The checkpoint, and within the run the survey's done test, record each stage in `covers` as done once its artifacts validate, so those stages are skipped and nothing that was converted is regenerated |
| domain | product-owner (planner), method `domain-spec` | Domain specification: capabilities, entities, invariants, edge cases, assumptions, risks, failure modes | Artifacts valid |
| contracts | product-owner (planner) | Requirements document and behavioural contracts, non-functional requirements, error taxonomy | As above |
| architecture | architect (planner), two steps: `architecture`, then `properties` | Architecture, decision records, the dependency list, the toolchain file `specs/architecture/toolchain.toml` (the `[commands]`, test report and `[paths]` lines the project will use; for an empty repository this is where they are chosen), purity boundary map, module criticality, verification tooling, deployment topology, and `data.feasibility` (`ok`, `issues`); then the verification properties | As above, and feasibility settled |
| feasibility | product-owner then architect (planner), at most 2 rounds, only when `feasibility.ok` is false | Revised contracts | `feasibility.ok`, or the open issues are recorded for the packet |
| assessments | architect (planner) | Service-clone assessment, reference-implementation assessment | Artifacts valid |
| scenarios | product-owner (planner) | Hidden scenarios, each citing the contracts it exercises | Artifacts valid. Only product-owner, scenario-reviewer and holdout-evaluator can read them |
| ux (UI products) | ux-designer (planner) | UX specification, design system (bootstrapped, or extracted after an ingest), two or three variants for complex screens with a scored recommendation | Artifacts valid |
| checks | One clerk running the cross-set `vsdd gate spec`. Fixes by the owning planner, at most 2 rounds; each fix step ends with `vsdd gate spec --only <part>`, run by the planner itself, so a failed check is another turn of the agent that has the context | Gate result whose findings carry the owning role | `gate.passed` |
| review | Two separate loops. The specification loop: adversary once per review unit, and ui-reviewer on the UX specification (reviewer); fixes by the planner each finding names in `owner`. The scenario loop: scenario-reviewer on the hidden scenarios; its findings and claims go only to the product-owner and never into the specification loop | Review data | `converge()` at the `spec` perimeter, cap 3, once for each loop. At the cap, open findings go to the packet; a walled finding appears there as its walled key and severity |
| close | clerk | Decision `spec` with the packet: summary, digest, open issues, recommended variant and topology, and the toolchain lines that approval will write where `[commands]` or `[paths]` are unset | |

Intake of existing documents. A person who has a requirements document or an architecture document
elsewhere passes its path in the `sources` argument, and the `intake` stage converts it into registry
artifacts and reports which later stages it covered. Documents a person has placed by hand in the 1.0 layout
are not picked up silently: nothing records them as done, so they are passed through `sources` like any
other document, and `intake` validates them where they sit. References between artifacts are
checked for existence by `vsdd gate spec` and at approval, not when a draft is written, so contracts may name
subsystems that the architecture stage has yet to write. CI setup, toolchain preflight and service-clone creation, which the old Phase 1 gate demanded, move to
`plan` and to ordinary stories.

Ends: decision `spec`.

#### 2.9.4 `feature`

Purpose: handle a change request against an existing product. Arguments: `request` (required on the first
run), `intent` (`feature`, `enhancement`, `bug-fix`, `deprecation`, `port`), `severity`, `guidance`, `repo`.

| Stage | Step and agent (tier) | Produces | Gate |
|---|---|---|---|
| survey | clerk; the request travels as `--request-uri` | Survey: the change request in flight if any, the baseline, the base branch (the release tag for a critical bug) | A baseline exists: a release, an ingest whose `spec` decision is approved, or an import. Every request is recorded once, as `.factory/requests/CR-NNN.md`. If another cycle is active and the request is not critical, it stays queued and the run returns. Otherwise the survey opens the cycle, whose `cycle.json` names the request. With `amend: true` the survey opens nothing: it runs inside the open cycle, takes every `spec-gap` decision settled `amend` as its request, and marks the run as holding the amendment |
| delta and route | architect (planner), method `delta-analysis`. Its closing command is `vsdd feature route --run <id>`: the tool derives the route from the delta record the step just wrote, writes it into `cycle.json`, and for the standard route marks the run as an amendment. No clerk step | Delta record and delta data: intent, severity, contracts changed and added, modules, whether any is critical, UI, cross-repository, estimated files. The route: `trivial` when the intent is not a bug fix, no contract changes or additions, the estimate is within the story size limit and no critical module is touched. `bugfix` when the intent is a bug fix and no contract changes; `hotfix` when its severity is critical. Otherwise `standard`. With `cross_repo` a fast route keeps its name and yields one story per repository, joined by `depends_on`. The route result also lists `readiness`: the chore stories the project still needs (a test harness and JUnit report when the baseline has no suite; CI when `delivery.ci = "none"` and there is a remote) | The script uses the route in the gate result the step returned. On a fast route the specification stays locked, so no contract can change; the plan gate refuses a story whose criteria cite a contract outside the approved set, and the green gate fails a `trivial`-route story whose diff touches a critical module |
| story-draft (fast routes) | story-writer (planner), method `story`. Its closing command is `vsdd gate plan --story <id> --run <id>` for each story it wrote; at most 2 attempts. No clerk step | One story per repository touched, and first the readiness chore stories the route listed. For a bug the story must include a regression test criterion. When a story passes, the same command admits it and returns its unit (section 2.9.1) in the gate result's `unit`, and run again for a story already admitted it returns the same unit; the step returns the units as `data.units` (section 2.10.8) | Through `gated()` with `each`: every story's `plan` entry passed, and the unit whose `id` is that entry's `subject` is in `data.units`. A story that fails is named in the retry note; if one still fails after the last attempt the step is blocked with kind `gate`, and no story is delivered while one is missing |
| deliver (fast routes) | `story` as a child through `runStories()`, readiness stories first | Story result | `status == "done"` |
| evolve (standard route) | product-owner, architect, and ux-designer for UI (planner), contracts first and the others in parallel | Contract, architecture, verification-property and UX deltas; new hidden scenarios; assessments again if a new external service or reference implementation appears | Artifacts valid |
| checks, review (standard route) | As `spec`, in delta scope | | As `spec` |
| close | clerk | Fast routes: `next` is `verify`. Standard route: decision `spec` with the delta packet | |

A deprecation is a standard-route feature whose delta removes contracts. A port from a reference
implementation is a standard-route feature whose stories have kind `port`. A market assessment is not part
of this workflow by default; `discover` in `assess` mode can be run on the request first.

Ends: fast routes end with nothing to decide; the standard route ends at decision `spec`.

#### 2.9.5 `plan`

Purpose: stories, waves and readiness. Arguments: `guidance`, `repo`. The scope (full or delta) comes from
the survey.

| Stage | Step and agent (tier) | Produces | Gate |
|---|---|---|---|
| survey | clerk | Survey; preconditions: a cycle is open and the specification is approved (section 2.5). The scope is `delta` after an amendment or on a project with a baseline | |
| outline | story-writer (planner), method `decompose` | Epics and the story list with contract references and dependencies (outline data) | `r.status == "done"` |
| write | story-writer (planner), method `story`, one per epic, in parallel | Story files: acceptance criteria traced to contract clauses, size estimate, criticality, kind, flags. Service clones are stories of kind `facade` | `r.status == "done"` for every epic |
| schedule and readiness | One clerk running `vsdd plan schedule`, as one job | The wave schedule by topological layering with priority and the stories-per-wave limit, the assignment of each hidden scenario to the earliest wave that covers its contracts, the plan gate result, the baseline check, and an estimate of stories, waves and agents. The baseline check records the suite on the integration branch; for an empty repository it records "no suite yet" and requires the scaffold story | `gate.passed`; fixes by story-writer, at most 2 rounds, each ending with `vsdd gate plan --story <id>` for the stories it changed |
| review | adversary (reviewer), method `review-stories`; fixes by story-writer | Review data | `converge()` at the `plan` perimeter, cap 2 |
| close | clerk | `next` is `wave`. Decision `plan` only when configured or when the estimate is over `stops.plan_ceiling_agents` | |

Readiness. No agent writes product files in this workflow. The plan gate requires `chore` stories in `E-0`
for what the project lacks, and devops builds them through the story playbook: the project scaffold for an
empty repository; a test harness with a JUnit report when the project has no suite; the CI pipeline when
`delivery.ci = "none"` and there is a remote. `vsdd plan schedule` puts the scaffold story alone in the first
wave, so every other story starts after the build and test commands exist. The scaffold's green gate is the
first run of the commands that the approved toolchain file wrote into `factory.toml`, and a wrong command
shows there as a `blocked` decision with the reason `environment`, not later in every story. A missing tool
on the machine is the same decision.

The plan gate checks coverage and shape in code: every contract the cycle added or changed is covered by a
story criterion (on a first release that is every contract; on a project with a baseline it is the cycle's
delta, so recovered or already-built contracts need no new story), every criterion traces to a clause, the
graph has no cycle, no story is over the size limit, no placeholder text, every wave has scenarios or a
recorded reason.

Ends: normally nothing to decide.

#### 2.9.6 `verify`

Purpose: check the release candidate and produce the convergence report. Arguments: `only`, `cycle`, `repo`.
The scope and the list of required checks come from the survey. The scope is `full` for a first release and
`delta` otherwise. A first release is a cycle whose baseline is empty: there was no earlier release, no
ingest and no import. So the first cycle on an ingested or imported codebase runs in delta scope, and the
whole legacy system is not put through full verification because one feature was added.

| Check | Step and agent (tier) | Runs | Blocking when |
|---|---|---|---|
| baseline | `baseline-r<k>`: clerk running `vsdd gate release --only baseline` | Every round, first, alone | The suite, lint or build fails; a traceability link is missing; a module's mutation score is under its criticality threshold; a scan reports critical or high; documentation links or references are stale. On a project with a baseline record these are compared against it: only what is new blocks, and the thresholds apply to modules the cycle touched |
| measured | `measured-r<k>`: one clerk running `vsdd gate release`, which runs every measured check that is due as one job: `formal` (for every verification property in scope whose module changed), `perf` (when budgets are configured), `ui` (UI products). The checks run in parallel inside the tool; `--only <check>` selects one | Beside the reviewers, after the baseline passes | A proof fails, a fuzz target crashes, or a property has no harness (fixed by a `harness` story that the verifier builds); a regression beyond the budget; a UI check fails |
| adversarial | `system-review-<unit>-r<k>`, adversary (reviewer): one per subsystem in full scope, one for the delta otherwise | When the unit changed since its last clean pass | The convergence rule, with `system` findings in reach |
| security | `security-r<k>`, security-reviewer (reviewer): triage of the scan output and review of the security boundaries | As above | The convergence rule |
| scenarios | `holdout-r<k>-b<i>`, holdout-evaluator (reviewer) | When product source changed since the last scenario evidence | The thresholds in section 2.7 |
| ui review | `ux-review-r<k>`, ui-reviewer (reviewer), including the final demo when configured; a finding about the final demo has the owner `demo-recorder` | UI products | The convergence rule |
| docs | No step of its own. The baseline check reports stale documentation as findings | Every round, with the baseline | The findings become a fix story of kind `docs`, written by technical-writer |

The loop is `converge()` at the `release` perimeter with a cap of 3 rounds. In each round the baseline runs
first; if it passes, the measured job and every due review run in parallel. Blocking findings become fix
stories (`vsdd story new-fix --verify`, which admits each) delivered through `story` by `runStories()`, whose
reserve check applies here as it does in `wave`: a fix round that does not fit beside the reserve for the
close returns `partial`. Evidence is tied to the commit, so after a fix round the tool reports which checks
are out of date and only those run again.

Required checks by the cycle's route (section 2.4): standard and first release, all that apply; `trivial`, baseline only; `bugfix`,
baseline, one adversarial pass over the delta, scenarios for the regression subset, security when the module
is critical or high, formal for affected properties; `hotfix`, baseline and one adversarial pass.

Ends: the checkpoint writes the convergence report from the evidence and opens decision `release`. If the
loop did not converge the packet says so and lists what is open, and the person can hold or accept the risk.

#### 2.9.7 `release`

| Stage | Step and agent (tier) | Does | Gate |
|---|---|---|---|
| survey | clerk | Precondition: decision `release` approved for the verified commit (for the cycle named by `cycle`, when two are open) | |
| release | One clerk running `vsdd release --run <id>`, a single idempotent command that does the next missing part each time it is run | Prepare: the release steps from `[release]` in project configuration, which are the version (computed from the route) written to `version_files`, the change log (rendered from the delivered stories), and the release branch or pull request; it ends with the check that the only changes since the approved commit are release files. Publish: tag, publish, and for a critical bug fix the merge of `hotfix/<version>` back into the integration branch, which waits for the integration lock and opens a `merge` decision if it conflicts. It records the new baseline that later delta runs compare against | `ok` |
| retro | researcher (planner), method `retrospective`, skipped with `retro: false` | A retrospective report under `research/` (kind `retrospective`) from the cycle's step records: pass counts, findings by perimeter, spikes, failures, and proposals, each naming the line it would add to `conventions.md` or `review-rules.md`. A person adopts one with `/vsdd-factory:author conventions` or `author review-rules`. There is no lessons register. The step returns `data.superseded`, the paths of earlier reports this one replaces, and the checkpoint retires them. It never blocks and needs no approval | none |
| close | clerk | The checkpoint closes the cycle, recording the close date, the version and the state commit in `cycle.json`; `next` is done, or `feature` when a change request is queued | |

In a multi-repository project the same workflow, run at the hub after one `release` decision, releases the
repositories in the order their stories' dependencies give.

#### 2.9.8 `ingest`

Purpose: record what an existing codebase is. Arguments: `source`, `as`, `guidance`, `repo`.

| Stage | Step and agent (tier) | Produces | Gate |
|---|---|---|---|
| survey | clerk | Survey with a file inventory by directory and language | |
| map | codebase-analyzer, method `ingest-broad`, on the builder tier by the roster's `[overrides]` (decision D5) | The module map (module-map data), which the script fans out over | `r.status == "done"` and at least one module |
| analyze | codebase-analyzer, method `ingest-deepen`, on the builder tier by the same override, one per module, as a pipeline | Per-module analysis: behaviours with file citations, conventions, dependencies, existing tests | |
| validate | adversary (reviewer), method `review-extraction`, the second pipeline stage for each module; corrections by codebase-analyzer (planner tier, method `ingest-deepen`), at most 2 rounds | Confirmed or corrected analysis | No claim contradicts the code |
| coverage and baseline | clerk running `vsdd gate ingest` as one job; a second analyze round over the files no analysis cited, at most 2 rounds | Coverage evidence, and the measured baseline: the suite result with the identities of the tests that fail today, lint and format findings, the mutation score per module, scanner findings in existing dependencies, and whether a suite, a JUnit report and CI exist | Cited source files at or over the configured share |
| synthesize | In parallel: architect, method `architecture` (recovered architecture, module criticality, `conventions.md`, `review-rules.md`, `dependencies.toml` with the current dependencies as the approved list, and `toolchain.toml` with the commands the project uses or, where it has none, the ones a harness story will provide); architect, method `verification-assessment` (the verification gap analysis, an assessment of kind `verification`); product-owner, method `recover-contracts` (recovered contracts marked `origin: recovered`, scenario seeds); security-reviewer, method `security-review` (security and dependency audit, written once to `analysis/<scope>/security.md`). Then product-owner (project context, method `domain-spec`) | The baseline that `feature` and `spec` build on | Artifacts valid |
| review | adversary (reviewer), method `review-specs`; fixes by the owning planner | Review data | `converge()` at the `spec` perimeter, cap 2 |
| close | clerk | The baseline closure record in `cycles/_standing/`, and decision `spec` on the recovered baseline. The packet is the ingest report, with low-confidence areas and any critical security finding at the top, the recovered contract set, the measured baseline and the toolchain lines | |

Measured file coverage replaces the old "deepen until novelty is low". With `as: "reference"` the map,
analyze and validate stages run, followed by two stages of their own: `disposition` (codebase-analyzer,
method `disposition`, judged against the approved brief or the `guidance` argument: model, reimplement,
enhance or leave) writes `analysis/<ref>/disposition.md`, and `port-analysis` (codebase-analyzer, method
`port-analysis`) writes `analysis/<ref>/port.md`. The assessments stage of `spec` and the delta step of a
`feature` with intent `port` read both.

Ends: with `as: project`, decision `spec` on the recovered baseline; with `as: reference`, a report.
Recovered contracts describe what the code does. Approving the baseline makes them the approved, locked
specification, so "the specification is approved" has a meaning for every later `feature`, and applies the
toolchain file to any `[commands]` and `[paths]` keys that are still unset.

What the baseline is for. Every later cycle on this project is a delta against it (section 2.9.6). The
measured gates compare against the baseline record: the green gate, the wave gate and the release gate fail
only on a test failure, lint finding, format finding or scanner finding that the baseline does not list, and
on a frozen test that fails; mutation and coverage thresholds apply to modules the cycle touched. A flaky
legacy test must not fail an unrelated story: when an inherited test that the baseline has passing fails, the
gate runs the test command once more, and a test that passes in either run does not fail the gate. The gate
result lists it in `flaky`, so it is in the gate's evidence and, because evidence is not kept, is copied into
the unit's closure record, which is where `vsdd report debt` reads it. A test the cycle added or changed gets
no second run. What the project inherited is listed by `vsdd report debt` and paid down through `maintain`.
Plan coverage and traceability apply to the contracts a cycle adds or changes, so approving several hundred
recovered contracts does not demand a story for each. A project imported from the old plugin has the same
kind of baseline, written by the importer, with the measurements taken by the first survey that needs them.

#### 2.9.9 `discover`

`scan` mode.

| Stage | Step and agent (tier) | Produces |
|---|---|---|
| survey | clerk | Discovery configuration, and the idea registry with rejected and deferred ideas |
| collect | clerk running `vsdd discover collect`: the tool runs each command configured under `[[discover.source]]` (issue trackers, feedback, analytics exports) and stores the output | Files under `.factory/research/inputs/<source>/<date>.*`, which the researcher reads. The researcher has no shell and never holds a credential |
| research | researcher (planner), up to four in parallel, each only if configured: market, customer feedback, competitors, usage analytics; methods `market-intel` and `discovery` | Signal files |
| synthesize | researcher (planner), method `discovery` | Candidate ideas |
| score | In parallel, each scoring every idea in one step with method `score-ideas`: product-owner (value, alignment, urgency), architect (feasibility, effort), adversary at the reviewer tier (novelty and challenge). The script takes the geometric mean | Scores |
| register | clerk running `vsdd idea register` | Duplicates and ideas in cool-down removed; the rest registered |
| briefs | researcher (planner), one per idea above the threshold | Idea briefs, stored as `.factory/research/<date>-idea-<ID>.md` |
| close | clerk | Decision `ideas` with the ranked report |

A schedule is a headless command (`claude -p "/vsdd-factory:discover"` with the Workflow tool allowed) run by
cron or launchd; the packaging design documents and tests the recipe.

`assess` mode: survey, one research step, one synthesis step by researcher giving a recommendation of
go, caution or stop with its evidence (and a draft brief when the subject was raw text), then a checkpoint
that attaches the assessment to the brief, opens the cycle and opens decision `brief`. It opens the cycle
through the same code as `vsdd checkpoint brief`, so that when the brief is approved, `spec`, `plan`, `wave`,
`verify` and `release` have a cycle directory for their closures, decisions and evidence. A fixture test
holds it: assess, approve the brief, and the `spec` survey sees an open cycle.

#### 2.9.10 `maintain`

Purpose: scheduled sweeps. Arguments: `sweeps`, `report_only`, `repo`. Sweeps run in parallel and each
returns sweep data: its findings and which of them are mechanically fixable.

| Sweep | Step and agent (tier) | Runs when |
|---|---|---|
| measured sweeps | One clerk running `vsdd gate sweep`, a single job that runs every measured sweep that is due: the dependency audit, the benchmarks (when configured), clone fidelity (when service clones exist), spec coherence (the full `spec` check), the debt report (waivers, advisories, flaky inherited tests and the inherited baseline; this replaces the technical-debt register sweep) and the collection of configured feedback sources. `--only <check>` selects one | Always |
| dependency triage | security-reviewer (reviewer), method `security-review`, on the audit output | The audit reported something |
| docs drift | technical-writer (builder), method `docs-drift` | Always |
| pattern consistency | code-reviewer (reviewer), method `pattern-sweep`, which reads the code against `conventions.md` and needs no diff | Always |
| scenario freshness | Two steps, split along the wall. holdout-evaluator (reviewer), method `holdout-eval`: re-runs the scenarios against the product. scenario-reviewer (reviewer), method `review-scenarios`: judges staleness and coverage against the contracts, which the evaluator may not read | Hidden scenarios exist |
| accessibility | ui-reviewer (reviewer), method `ui-review`, lens accessibility | UI products |
| design drift | ui-reviewer (reviewer), method `ui-review`, lens design drift | A design system exists |
| risks and assumptions | researcher (planner), method `risk-sweep` | The domain specification has them |
| post-release signals | researcher (planner), method `discovery`, on what the sweep job collected | Analytics or feedback sources configured |

Then `vsdd story new-fix --maintain` writes fix stories for the fixable findings, at most ten per run, admits
each, and each is delivered through `story` by `runStories()`, with the same gates as any story. While a
product cycle is open, whether or not a run holds its lock, the workflow runs in `report_only` mode. The
checkpoint writes the report. A critical vulnerability that cannot be fixed mechanically opens a `finding`
decision.

Fixes are delivered inside a cycle. When the run is about to deliver fix stories and no cycle is open, the
tool opens a maintenance cycle (kind `maintenance`, its request written by code from the sweep findings), so
the fixes go through `verify` in delta scope and a `release` decision like any other change, and the route
after the run is `verify`. When a product cycle is open the run is report-only, so the question does not
arise. A run with nothing to deliver opens no cycle.

Ends: a report; no routine stop of its own. When fixes were delivered, the cycle it opened ends at decision
`release`.

#### 2.9.11 `multi` and the `repo` argument

A multi-repository project follows the hub model of `state.md` section 2.14. One repository, the hub, holds
the single state directory; its `factory.toml` has `layout = "hub"` and one `[[repo]]` entry per member
repository, with that repository's path, branches and commands. Member repositories hold code and nothing
from the factory. There is one cycle, one identifier space and one status report. A story belongs to one
repository and carries `repo`; work that spans two repositories is two stories joined by `depends_on`.

Per-repository work uses the ordinary workflows with `repo: "<name>"`, run from the hub. The survey resolves
the name to its `[[repo]]` entry, filters the units to stories with that `repo`, and returns the member
directory as `project_dir`; the wrapper puts it in every step prompt, and the hook runner finds the hub's
state by walking up from the path being touched (need H7). That one argument replaces the old sub-workflows
with a changed working directory. Release order follows story dependencies.

`multi` does the work that belongs to no single repository.

| Stage | Step and agent (tier) | Runs when |
|---|---|---|
| survey | clerk | Always: progress per repository, read from each story's `repo`; the order the story dependencies give; which project-level stages are due |
| split | clerk running `vsdd repos split` | A `spec` decision chose `topology=split`: the tool creates the member repositories from the `repo` key of the architect's subsystem files and appends their `[[repo]]` entries. This is the one case in which code adds to `factory.toml` after `init`, and the next briefing announces it |
| synthesize | architect (method `multi-repo-synthesis`) and product-owner (planner) | Every existing repository is ingested and the project context is stale |
| contracts | clerk running `vsdd gate contracts`; generated client libraries are rebuilt through fix stories in their repositories | A provider's published interface changed since its consumers were last built |
| integrate | clerk running `vsdd gate integration` (bring the services up together, cross-repository end-to-end tests); then adversary, security-reviewer and holdout-evaluator (reviewer) at the `release` perimeter; fixes through `story` with `repo` set to the owner | The repositories of one dependency layer have closed their waves, and again before release |
| close | clerk | `next` is the next per-repository command, or decision `spec` or `release` |

A cross-repository change request starts with `feature` at the hub; its delta analysis names the affected
repositories (`cross_repo`), and each gets its own story: through the plan on the standard route, and on a
fast route as one story per repository joined by `depends_on`, because a story belongs to one repository.

What is kept per repository. Waves are project-wide (the plan's wave map covers every story), but a wave is
built and gated repository by repository, since each `wave` run takes the lock of one repository and branch.
The wave's closure record, its gate rounds and its integration worktree are therefore per repository:
`cycles/<cycle>/units/wave.<W-NN>.<repo>.json` and `.worktrees/_integration/<repo>/`. A wave is closed when
every repository that has stories in it has closed its part, and the survey's "every earlier wave is closed"
reads that. A `[[repo]]` entry may carry its own `[release]` and `[delivery]` tables, so the hub release can
ship each repository by its own rules, in the order story dependencies give.

Setting a hub up. `vsdd init` does not guess a hub. `/vsdd-factory:setup --hub` runs `vsdd repos scan`, shows
the repositories it found, and passes the confirmed list to `vsdd init --hub`, which writes the `[[repo]]`
entries when it creates `factory.toml`; on a project that is already initialised it shows the block for the
person to paste. On a fresh clone of the hub, `vsdd status --fix` clones each member repository that is
missing from the `url` in its entry.

#### 2.9.12 `spike`

| Call | Steps | Returns |
|---|---|---|
| `{question, kind?, ref?}`, or the text `<question>` | A survey that stores the question in the run record (it takes no lock), then one `runSpike()` call | The spike finding |
| `{open: name, question}`, or the text `open <name> -- <mission>` | The same survey; then one spike-runner step whose first command is `vsdd spike open --name <name> --run <id>` and whose last is `vsdd spike record --increment`. No clerk opens the entry | The ledger's current state and next action |
| `{close: name}`, or the text `close <name>` | One spike-runner step that writes the final finding and runs `vsdd spike close --name <name>` | The final finding |

The workflow needs an initialised project and says so when there is none (section 2.8).

#### 2.9.13 The method of every step

`step()` takes a required `method`, and universal test 6 checks that the method exists and that the step's
agent file lists it. This table is the source the scripts are written from. Clerk steps have no method. Where
a stage has more than one agent step, each is listed.

| Workflow | Stage or step | Agent | Method |
|---|---|---|---|
| `story` | tests | test-writer | By story kind: `failing-tests` (`feature`; it includes the stubs), `regression-test` (`fix`), `equivalence-tests` (`port`); `ui-tests` or `e2e-tests` when the story's `method` key says so. No tests stage for `facade`, `docs`, `chore`, `harness` |
| `story` | implement | By story kind | `tdd` (implementer, `feature`), `fix-findings` (implementer, `fix`), `port-translation` (implementer, `port`), `service-clone` (implementer, `facade`), `docs` (technical-writer, `docs`), `repo-setup`, `ci` or `toolchain` (devops, `chore`), `proofs`, `fuzzing`, `mutation`, `performance` or `clone-fidelity` (verifier, `harness`); or the story's `method` key |
| `story` | review | adversary; security-reviewer | `review-story-diff`; `security-review` |
| `story` | fix | test-writer; implementer | `test-fixes` (findings of category `test` or `coverage`; ends with `vsdd gate red --refreeze`); `fix-findings` |
| `story` | arbitrate | architect | `arbitrate` |
| `story` | evidence | demo-recorder | `demo` |
| `story` | repair after a failed delivery | implementer | `fix-findings` |
| `wave` | integration review; security; hidden scenarios; UX review; arbitrate | adversary; security-reviewer; holdout-evaluator; ui-reviewer; architect | `review-wave`; `security-review`; `holdout-eval`; `ui-review`; `arbitrate` |
| `spec` | intake | product-owner; architect | `intake` |
| `spec` | domain | product-owner | `domain-spec` |
| `spec` | contracts | product-owner | `requirements` |
| `spec` | architecture | architect | `architecture`, then `verification-properties` |
| `spec` | feasibility | product-owner; architect | `requirements`; `feasibility` |
| `spec` | assessments | architect | `clone-assessment` and `port-assessment`, one step each, only when the product has external services or a reference implementation |
| `spec` | scenarios | product-owner | `holdout-scenarios` |
| `spec` | ux | ux-designer | `ux-spec`; then `design-system` when none exists and `design-variants` for complex screens, one step each |
| `spec` | fixes after checks or review | The owning planner | The method that wrote the artifact |
| `spec` | review | adversary; ui-reviewer; scenario-reviewer (its own loop) | `review-specs`; `ui-review`; `review-scenarios` |
| `feature` | delta and route | architect | `delta-analysis` |
| `feature` | story-draft | story-writer | `story` |
| `feature` | evolve | product-owner; architect; ux-designer | One step per method. product-owner: `spec-delta`, then `holdout-scenarios` when the delta needs new hidden scenarios. architect: `architecture`, then `verification-properties` when properties change, and `clone-assessment` or `port-assessment` when a new external service or reference implementation appears. ux-designer: `ux-spec` |
| `feature` | checks and review | As `spec` | As `spec` |
| `plan` | outline; write and fixes | story-writer | `decompose`; `story` |
| `plan` | review | adversary | `review-stories` |
| `verify` | adversarial; security; scenarios; UI review; arbitrate | adversary; security-reviewer; holdout-evaluator; ui-reviewer; architect | `review-system`; `security-review`; `holdout-eval`; `ui-review`; `arbitrate` |
| `release` | retro | researcher | `retrospective` |
| `ingest` | map; analyze and corrections | codebase-analyzer | `ingest-broad`; `ingest-deepen` |
| `ingest` | validate | adversary | `review-extraction` |
| `ingest` | synthesize | architect; architect; product-owner; security-reviewer; product-owner | `architecture`; `verification-assessment`; `recover-contracts`; `security-review`; `domain-spec` |
| `ingest` | review | adversary | `review-specs` |
| `ingest` (`as: reference`) | disposition; port-analysis | codebase-analyzer | `disposition`; `port-analysis` |
| `discover` | research; synthesize; briefs | researcher | `market-intel` or `discovery`; `discovery`; `discovery` |
| `discover` | score | product-owner; architect; adversary | `score-ideas` |
| `discover` (assess) | research and synthesis | researcher | `market-intel` |
| `maintain` | dependency triage; docs drift; pattern consistency | security-reviewer; technical-writer; code-reviewer | `security-review`; `docs-drift`; `pattern-sweep` |
| `maintain` | scenario freshness | holdout-evaluator; scenario-reviewer | `holdout-eval`; `review-scenarios` |
| `maintain` | accessibility; design drift | ui-reviewer | `ui-review` |
| `maintain` | risks and assumptions; post-release signals | researcher | `risk-sweep`; `discovery` |
| `multi` | synthesize | architect; product-owner | `multi-repo-synthesis`; `domain-spec` |
| `multi` | integrate reviews | adversary; security-reviewer; holdout-evaluator | `review-system`; `security-review`; `holdout-eval` |
| `spike` | every spike | spike-runner | `spike` |

Methods this table needs that the first pass did not have: `equivalence-tests` and `test-fixes`
(test-writer), `recover-contracts` (product-owner), `verification-assessment` (architect), `docs-drift`
(technical-writer) and `pattern-sweep` (code-reviewer). Outside the workflows, the `brief` skill uses
`brief` (product-owner) and the `author` skill uses `conventions` (architect). `stubs` is merged into
`failing-tests`, because one step takes one method.

### 2.10 Shared result schemas

The schemas live as files in `<plugin>/schemas/` and are the single source for the scripts, the tool and the
hook runner (need P3). The scripts hold them as object literals composed without `$ref`, because the platform
only promises a root object with `properties`.

#### 2.10.1 Step result

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "step-result.schema.json",
  "type": "object",
  "properties": {
    "step": {
      "type": "object",
      "description": "Identity given in the prompt, returned unchanged. The step record is stored under it.",
      "properties": {
        "run": { "type": "string" },
        "workflow": { "type": "string" },
        "name": { "type": "string" },
        "subject": { "type": "string" },
        "attempt": { "type": "integer", "minimum": 1 }
      },
      "required": ["run", "workflow", "name", "subject", "attempt"]
    },
    "status": { "type": "string", "enum": ["done", "needs_spike", "blocked", "failed"] },
    "summary": { "type": "string", "maxLength": 600 },
    "head": { "type": "string", "description": "Commit the step left its branch at; empty if it made none." },
    "artifacts": { "type": "array", "items": { "type": "string" }, "maxItems": 50 },
    "gates": { "type": "array", "items": { "$ref": "gate-result.schema.json" } },
    "blocked": {
      "type": "object",
      "properties": {
        "kind": { "type": "string", "enum": ["spec_gap", "environment", "spike_limit", "gate", "agent_budget", "conflict", "reviewer_blocked", "other"] },
        "reason": { "type": "string", "maxLength": 600 },
        "question": { "type": "string", "maxLength": 400 }
      },
      "required": ["kind", "reason"]
    },
    "spike_request": { "$ref": "spike-request.schema.json" },
    "data": { "type": "object", "description": "Step-specific payload; shapes in section 2.10.8. Required; it may be empty. Every field a script reads is required by that step's data schema." }
  },
  "required": ["step", "status", "summary", "head", "data"]
}
```

#### 2.10.2 Review finding

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "finding.schema.json",
  "type": "object",
  "properties": {
    "key": { "type": "string", "maxLength": 200, "description": "Stable identity, <category>:<path>:<symbol or clause>. The same defect has the same key in every pass." },
    "title": { "type": "string", "maxLength": 160 },
    "severity": { "type": "string", "enum": ["critical", "high", "medium", "low"] },
    "category": { "type": "string", "enum": ["spec-fidelity", "test", "code", "security", "integration", "performance", "accessibility", "ux", "docs", "spec", "traceability", "holdout", "formal", "coverage"] },
    "scope": { "type": "string", "enum": ["unit", "integration", "system", "spec"] },
    "owner": { "type": "string", "enum": ["product-owner", "architect", "ux-designer", "story-writer", "test-writer", "implementer", "verifier", "technical-writer", "devops", "codebase-analyzer", "demo-recorder"], "description": "The roster agent that should fix it. Fixes are routed on this field." },
    "location": {
      "type": "object",
      "properties": {
        "path": { "type": "string", "description": "Relative to the root of the reviewed tree: the unit's worktree for a story, the integration worktree for a wave or a release, the project root for files under .factory/." },
        "line_start": { "type": "integer" },
        "line_end": { "type": "integer" },
        "quote": { "type": "string", "maxLength": 400 }
      },
      "required": ["path"]
    },
    "spec_ref": { "type": "string", "description": "Contract clause or acceptance criterion violated; empty if none." },
    "repro": { "type": "string", "maxLength": 800, "description": "Command or scenario that shows the defect; empty if none." },
    "observed": { "type": "string", "maxLength": 600 },
    "expected": { "type": "string", "maxLength": 600 },
    "suggestion": { "type": "string", "maxLength": 600 },
    "confidence": { "type": "number", "minimum": 0, "maximum": 1 }
  },
  "required": ["key", "title", "severity", "category", "scope", "owner", "location", "spec_ref", "repro", "observed", "expected"]
}
```

When the tool or the hook runner stores a finding it adds `grounded` (true or false). An ungrounded finding
is advisory. The hook blocks a review with ungrounded findings once per step, so the reviewer can correct
them, and stores the result on the second return. A quote may sit up to three lines from the cited range.

Two kinds of finding are treated apart (section 2.7). A finding of category `holdout` is built by the tool
from scenario scores, has an empty `location` (the schema's required `location` is then `{"path": ""}`), and
is exempt from the location condition and from grounding. A walled finding (one returned by the
`scenario-reviewer`, or located under `.factory/holdout/`) is stored in full only in the holdout class;
everywhere else it is its walled key and `severity`.

A finding against the final demo has the owner `demo-recorder`.

#### 2.10.3 Verdict

The script computes it with `converge()`; the tool computes the same object from the stored passes, as a
function that `vsdd gate demo`, `vsdd story deliver`, the survey and the checkpoint call. It is not a command
and it is not stored as a record of its own: after the checkpoint the verdict is in the unit's closure
record. The tool's result is the one guards read. The tool counts a pass only when the agent type stored with
the step record is the reviewer that owns the pass's method.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "verdict.schema.json",
  "type": "object",
  "properties": {
    "perimeter": { "type": "string", "enum": ["spec", "plan", "story", "wave", "release"] },
    "subject": { "type": "string" },
    "head": { "type": "string", "description": "Commit the last pass reviewed." },
    "passes": { "type": "integer", "minimum": 0 },
    "outcome": { "type": "string", "enum": ["converged", "blocked", "failed"] },
    "reason": { "type": "string", "enum": ["clean", "cap", "spec_gap", "no_result", "reviewer_blocked"] },
    "rounds_used": { "type": "integer", "minimum": 0, "description": "Passes or rounds used across every run for this unit." },
    "blocking": { "type": "array", "items": { "$ref": "finding.schema.json" } },
    "advisory": { "type": "array", "items": { "$ref": "finding.schema.json" } },
    "deferred": { "type": "array", "items": { "$ref": "finding.schema.json" } },
    "dismissed": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "key": { "type": "string" },
          "by": { "type": "string", "enum": ["reviewer", "arbiter", "human"] },
          "reason": { "type": "string" }
        },
        "required": ["key", "by", "reason"]
      }
    },
    "coverage_missing": { "type": "array", "items": { "type": "string" } }
  },
  "required": ["perimeter", "subject", "head", "passes", "outcome", "reason", "blocking", "advisory", "deferred", "dismissed", "coverage_missing"]
}
```

#### 2.10.4 Spike request

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "spike-request.schema.json",
  "type": "object",
  "properties": {
    "question": { "type": "string", "maxLength": 400, "description": "One question an experiment can answer." },
    "why_blocking": { "type": "string", "maxLength": 400 },
    "kind": { "type": "string", "enum": ["experiment", "research"], "description": "experiment runs code; research reads documentation and sources. Both run in an isolated worktree, so every runner has its own working directory." },
    "method_hint": { "type": "string", "maxLength": 600 },
    "ref": { "type": "string", "description": "Branch or commit to start from; empty means the commit the asking step left." },
    "done_when": { "type": "string", "maxLength": 300 }
  },
  "required": ["question", "why_blocking", "kind"]
}
```

#### 2.10.5 Spike finding

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "spike-finding.schema.json",
  "type": "object",
  "properties": {
    "ledger_id": { "type": "string", "description": "Id returned by vsdd spike open; empty only when the runner returned nothing." },
    "question": { "type": "string" },
    "verdict": { "type": "string", "enum": ["confirmed", "refuted", "answered", "inconclusive", "refused"], "description": "The spike ledger in the state design uses this same list." },
    "answer": { "type": "string", "maxLength": 1200 },
    "method": { "type": "string", "maxLength": 800 },
    "evidence": { "type": "array", "items": { "type": "string", "maxLength": 300 }, "maxItems": 10 },
    "confidence": { "type": "number", "minimum": 0, "maximum": 1 },
    "cached": { "type": "boolean", "description": "True when the ledger already held this answer and no experiment ran." },
    "artifacts": { "type": "array", "items": { "type": "string" }, "description": "Paths in the kept worktree, for reference only." },
    "follow_up": { "type": "string", "maxLength": 400 }
  },
  "required": ["ledger_id", "question", "verdict", "answer", "method", "evidence", "confidence", "cached"]
}
```

#### 2.10.6 Gate result

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "gate-result.schema.json",
  "type": "object",
  "properties": {
    "ok": { "type": "boolean", "description": "False when the command itself could not run." },
    "state": { "type": "string", "enum": ["done", "pending"], "description": "pending: the job is still running; run the same command again. Absent means done." },
    "diff_patch": { "type": "string", "description": "Green and wave gates: path of the patch the gate wrote for the commit it measured. Given to the next reviewer." },
    "gate": { "type": "string" },
    "subject": { "type": "string" },
    "passed": { "type": "boolean" },
    "sha": { "type": "string", "description": "Commit the gate measured." },
    "evidence": { "type": "string", "description": "Path of the evidence record." },
    "metrics": { "type": "object" },
    "failures": { "type": "array", "items": { "type": "string", "maxLength": 400 }, "maxItems": 20 },
    "flaky": { "type": "array", "items": { "type": "string" }, "description": "Identities of inherited tests that failed and then passed on the second run (section 2.9.8). Copied into the unit's closure record." },
    "findings": { "type": "array", "items": { "$ref": "finding.schema.json" } },
    "unit": { "type": "object", "description": "Only from vsdd gate plan --story, and always when it passes: the admitted story unit (section 2.10.8)." },
    "error": { "type": "string" }
  },
  "required": ["ok", "gate", "subject", "passed", "sha", "evidence", "failures"]
}
```

#### 2.10.7 Workflow result

What every workflow returns to the main session or to its parent. It is kept under 2 KB.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "workflow-result.schema.json",
  "type": "object",
  "properties": {
    "workflow": { "type": "string" },
    "subject": { "type": "string" },
    "status": { "type": "string", "enum": ["done", "partial", "needs_human", "blocked", "failed"] },
    "summary": { "type": "string", "maxLength": 600 },
    "progress": { "type": "boolean", "description": "False when the run changed nothing on disk." },
    "decisions": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": { "id": { "type": "string" }, "kind": { "type": "string" }, "summary": { "type": "string" } },
        "required": ["id", "kind", "summary"]
      }
    },
    "next": {
      "type": "object",
      "properties": {
        "kind": { "type": "string", "enum": ["workflow", "human", "done"] },
        "command": { "type": "string" },
        "args": { "type": "object" }
      },
      "required": ["kind"]
    },
    "counts": { "type": "object", "properties": { "agents": { "type": "integer" }, "spikes": { "type": "integer" } } }
  },
  "required": ["workflow", "subject", "status", "summary"]
}
```

#### 2.10.8 Step-specific data and tool outputs

| Name | Returned by | Fields |
|---|---|---|
| Review data | Reviewers | `reviewed_sha`; `findings` (findings); `claim_checks` (`key`, `outcome`: `resolved`, `not_resolved` or `withdrawn`, `note`); `coverage` (`item`, `state`: `checked`, `not_checkable` or `skipped`, `note`). All four are required |
| Claim | Given to the next reviewer | `key`, `path`, `line_start`, `line_end`, `resolution` (`fixed` or `disputed`), `commit`. No quote and no free text from the fixer. For a finding of category `holdout`: `key`, `resolution` and `commit` only. Claims from the scenario review go only to the scenario-reviewer's next pass |
| Fix data | Fixers | `resolutions` (`key`, `resolution`: `fixed` or `disputed`, `commit`, `note`). The `note` is kept in the step record for the arbiter only |
| Ruling data | Arbiter | `rulings` (`key`, `ruling`: `defect`, `not_defect` or `spec_gap`, `reason`) |
| Holdout data | holdout-evaluator | `scores` (`scenario`, `satisfaction` from 0 to 1, `must_pass`, `ran`); `symptoms` (`scenario`, `observed`, `expected`, with no location and no scenario text). These are not findings and are not grounded; `holdoutFindings()` and the tool build findings from the scores, in the shape section 2.7 fixes |
| Pending output (tool) | Any long command (`vsdd gate ...`, `vsdd story deliver`, `vsdd release`) while its job runs | `ok: true`, `state: "pending"`, and `retry`, the same command to run again. When the job is done the command returns its ordinary result |
| Delta data | `feature` delta step | `intent`, `severity`, `contracts_changed`, `contracts_added`, `modules`, `critical_modules`, `ui`, `cross_repo` (the repositories touched), `files_estimate` |
| Route data (tool) | `vsdd feature route` | `ok`, `route`, `amendment`, `stories` (one per repository touched), `readiness` (chore stories the project still needs) |
| Story-draft data | `feature` story-draft step | `units`: one story unit for each story the step wrote, copied from the `unit` of that story's passing `vsdd gate plan --story` result. Required, so the script may read it; the script passes each unit to `runStories()` |
| Intake data | `spec` intake step | `covers` (names of the stages whose output the intake filled) |
| Outline data | `plan` outline step | `epics` (`id`, `title`, `contracts`, `stories` with `id`, `title`, `depends_on`) |
| Module-map data | `ingest` map step | `modules` (`name`, `paths`, `language`, `files`) |
| Score data | `discover` scorers | `scores` (`idea`, `dimension`, `value` from 1 to 10, `evidence`) |
| Sweep data | `maintain` sweeps | `sweep`, `findings`, `fixable` (finding keys) |
| Survey (tool) | `vsdd survey` | `ok`, `error`, `run_id`, `workflow`, `subject`, `cycle`, `scope` (`full` or `delta`), `project_dir`, `plugin_root`, `head`, `preconditions` (`name`, `met`, `detail`), `done` (the stage names recorded as done), `units`, `config` (including `review_rules` and any `tier_override`), `limits` (under the prelude's names), `spikes_used`, `decisions`, `waived`, `next` |
| Story unit | `vsdd survey story` and `wave`; `vsdd gate plan --story` (in the gate result's `unit`); `vsdd story new-fix` and the `fix_units` of `vsdd gate wave` | `id`, `position`, `worktree`, `tree`, `branch`, `base`, `head`, `story_path`, `bcs`, `acs`, `criticality`, `kind`, `method`, `repo`, `security`, `ui`, `demo_required`, `waived`, `guidance`, `deferred`, `diff_patch`, `test_output`, `dirty`, `open_pr`, `rounds_used`, `blocked` (`kind`, `decision`, `open`), `fixes` (for a fix story) |
| Wave gate data (tool) | `vsdd gate wave` | The gate result fields, and `pending`, `worktree`, `base`, `stories`, `needs` (`security`, `ui`), `holdout_batches`, `diff_patch`, and `fix_units` (admitted story units) when the measured gate failed |
| Delivery data (tool) | `vsdd story deliver` | `ok`, `error`, `state`, `pr`, `merged_sha`, `detail`, `reviewed_sha`, `changed_files`, `diff_patch` (the patch from `reviewed_sha` to the head, for the scoped review after a repair), `needs_person` (`protected_path`, `dependency` or `configured`) |
| Close (tool) | `vsdd checkpoint` | `ok`, `status`, `progress`, `decisions`, `next` |

### 2.11 Script skeletons

These are illustrations of shape, not the product. Blueprint 4.14 names `judge()` and `converge()` in the
prelude as the single source of the convergence rule, and these blocks are the only rendering of that code in
the design, so on 2026-11-24 they were corrected in place to the behaviour both review passes fixed. A
builder ports what is written here. The corrections: a required `method` on every step, with the identity and
the closing command repeated at the end of the prompt; `data` required; the earlier-work line whenever the
unit is dirty, not only on a retry; `relay()` passes `vsdd -C <dir>`, retries once, and has the clerk run a
pending command again; no `ci_pending` loop; `try`/`catch` around the body so the checkpoint always runs;
spikes always isolated, on the spike runner's own tier, opened from the stored step record and never with an
asker on the command line or text on standard input; spike findings carried across `gated()` attempts;
`judge()` exempts `holdout` findings from the location condition; a coverage gap is a finding and calls the
fixer; an arbiter dismisses only a finding the builder disputed; the cap counts rounds used in earlier runs;
a reviewer that returns blocked gives `reviewer_blocked`, not `no_result`; claims carry no title, quote or
note; `runStories()` is in the prelude with the reserve check, children receive `story_agents`, and a child
that throws is charged its budget; phases name no model. The same behaviours are conformance fixtures
(`packaging.md` 2.6, layer 2b), so the build cannot freeze the earlier ones. The close-out (blueprint 11.4)
added four: argument names in `parseArgs()`, a per-item closing command in `brief()` with the matching
`each` mode of `gated()`, and the implementer following a failing re-freeze in `fix()`.

They follow the platform limits: `meta` is a pure
literal; there is no import, file access, shell access, clock or random number; the only child call is
`workflow('vsdd-factory:story', …)` from a top-level script; every step sets model and effort itself, because
whether a step inherits the agent file's effort is unverified. Before the corrections the three blocks were
parsed with Node and run against a fake `agent()` through six story cases (clean first pass, one finding
fixed, a spike request, the cap reached, the cap reached with an arbiter dismissal, a CI failure) and three
wave cases (clean gate, one integration finding fixed through a fix story, a run stopped by the story limit).
The corrected text was parsed with Node again and run against a fake `agent()` for twenty checks of the
corrected behaviours: a clean story as a child (four agents, every prompt opening with its method and ending
with its closing command, the clerk's command using `vsdd -C`); an exception in a step, which returns
`failed` through the checkpoint path; a research spike request, which is isolated, runs on the planner tier
and is opened from the step record; a clean two-story wave whose children receive `story_agents`; a thrown
child charged its full budget; and the convergence cases (a coverage gap that calls the fixer, an undisputed
finding the arbiter may not dismiss, a disputed one it may, the cap counted across runs, a blocked reviewer,
claims with no title, quote or note, and a `holdout` finding that blocks without a location). Package B5
turns these and the original nine cases into harness tests. They have not been run on the platform.

#### 2.11.1 The stamped prelude: tiers, the spike wrapper, gates and the convergence rule

```js
// >>> stamped:prelude  (generated from workflow-lib/prelude.js by tools/build.py; a test fails if this block differs)
const P = 'vsdd-factory:'
// TIER and ROLE are generated by tools/build.py from tests/lint/roster.toml. They are never edited by hand.
const TIER = {
  planner: { model: 'fable', effort: 'xhigh' },
  builder: { model: 'sonnet', effort: 'xhigh' },
  reviewer: { model: 'opus', effort: 'xhigh' },
  clerk: { model: 'sonnet', effort: 'low' },
}
const ROLE = {
  clerk: 'clerk', 'spike-runner': 'planner', 'test-writer': 'builder', implementer: 'builder',
  verifier: 'builder', 'demo-recorder': 'builder', adversary: 'reviewer',
  'security-reviewer': 'reviewer', 'holdout-evaluator': 'reviewer', architect: 'planner',
  // … one generated row per agent in the roster (20 rows)
}
// OVERRIDE is generated from [overrides] in the roster: the only steps that run off their agent's tier.
const OVERRIDE = { 'ingest/map': 'builder', 'ingest/analyze': 'builder' }
const RUN = {
  id: '', dir: '', root: '', wf: '', tier: null, agents: 0, spikes: 0,
  limits: {
    agents: 900, story_agents: 40, stories_per_run: 24, attempts: 2, passes: 3, rounds: 3, delivery_rounds: 3,
    spikes_per_step: 2, spikes_per_run: 6, spike_minutes: 20,
  },
}
const CLOSE_RESERVE = 30 // agents kept back so the checkpoint can always run
const safe = (s) => (/^[A-Za-z0-9._\/-]+$/.test(String(s || '')) ? String(s) : '') // identifiers only
const enc = (s) => encodeURIComponent(String(s || '')) // free text on a command line; the tool decodes it
// A person's typed text, or an object, becomes the argument object. A key=value word is a named argument only
// when key is in names, the workflow's own argument names; any other such word stays in the main argument.
function parseArgs(raw, main, names = []) {
  if (raw && typeof raw === 'object') return raw
  const out = {}, rest = []
  for (const w of String(raw || '').trim().split(/\s+/).filter(Boolean)) {
    const m = /^([a-z_]+)=(.+)$/.exec(w)
    if (m && names.includes(m[1])) out[m[1]] = m[2]
    else rest.push(w)
  }
  if (rest.length) out[main] = rest.join(' ')
  return out
}
const str = { type: 'string' }
const S_FINDING = { type: 'object', properties: { key: str, title: str, severity: str, category: str, scope: str /* … section 2.10.2 */ }, required: ['key', 'title', 'severity', 'category', 'scope'] }
const S_SPIKE_REQUEST = { type: 'object', properties: { question: str, why_blocking: str, kind: str, method_hint: str, ref: str, done_when: str }, required: ['question', 'why_blocking', 'kind'] }
const S_SPIKE_FINDING = { type: 'object', properties: { ledger_id: str, question: str, verdict: str, answer: str, method: str, evidence: { type: 'array', items: str }, confidence: { type: 'number' }, cached: { type: 'boolean' }, artifacts: { type: 'array', items: str }, follow_up: str }, required: ['ledger_id', 'question', 'verdict', 'answer', 'method', 'evidence', 'confidence', 'cached'] }
const S_GATE = { type: 'object', properties: { gate: str, subject: str, passed: { type: 'boolean' }, sha: str, evidence: str, failures: { type: 'array', items: str }, findings: { type: 'array', items: S_FINDING }, unit: { type: 'object', properties: {} } }, required: ['gate', 'subject', 'passed', 'sha', 'evidence', 'failures'] }
const stepSchema = (data) => ({
  type: 'object',
  properties: {
    step: { type: 'object', properties: { run: str, workflow: str, name: str, subject: str, attempt: { type: 'integer' } }, required: ['run', 'workflow', 'name', 'subject', 'attempt'] },
    status: { type: 'string', enum: ['done', 'needs_spike', 'blocked', 'failed'] },
    summary: str, head: str,
    artifacts: { type: 'array', items: str },
    gates: { type: 'array', items: S_GATE },
    blocked: { type: 'object', properties: { kind: str, reason: str, question: str }, required: ['kind', 'reason'] },
    spike_request: S_SPIKE_REQUEST,
    data: data || { type: 'object', properties: {} },
  },
  required: ['step', 'status', 'summary', 'head', 'data'],
})
const made = (id, status, kind, reason) => ({ step: { ...id, attempt: 0 }, status, summary: reason, head: '', blocked: { kind, reason }, data: {} })

// What every step is told. The method comes first. The step identity and the closing command come last, so
// both survive a compaction of the step. o.closing is a vsdd command without the leading "vsdd". With o.each
// (the fast-route story-draft step: 'story you wrote') the command holds <id> and is run once per item, and
// gated() requires every item's entry to pass.
function brief(id, n, task, spikes, o) {
  return [
    `Method: ${o.method}. Read ${RUN.root}/methods/${o.method}.md first.`,
    task,
    `Project directory: ${RUN.dir}.`,
    'You are running as a workflow step. If you cannot finish without an experiment, stop, return status ' +
      '"needs_spike" and fill spike_request with one question. A spike will be run for you in isolation and ' +
      'this step will be started again with the finding.',
    spikes.length ? `Spike findings already obtained for this step: ${JSON.stringify(spikes)}` : '',
    n > 1 || o.dirty ? 'Earlier work on this step may be on disk, some of it uncommitted. Check before redoing it.' : '',
    `Return step = ${JSON.stringify({ ...id, attempt: n })} unchanged.`,
    o.closing
      ? `Finish by running${o.each ? `, once for each ${o.each}` : ''}: vsdd -C ${RUN.dir} ${o.closing} --wait 540 --json  with a Bash timeout of 600000. ` +
        'If it prints state "pending", run the same command again. Put its final output in gates.'
      : '',
  ].filter(Boolean).join('\n')
}

// One research spike: an isolated, bounded experiment that answers one question. Nothing about the asker is
// passed on the command line: vsdd spike open reads the question and the asking agent's type from the
// needs_spike step record that the hook stored, named by stepKey. Inputs are files, never standard input.
async function runSpike(req, stepKey, phaseTitle) {
  RUN.spikes++
  RUN.agents++
  const tier = RUN.tier || TIER[ROLE['spike-runner']] // the spike runner's own tier, for every kind of spike
  const f = await agent(
    `Method: spike. Read ${RUN.root}/methods/spike.md first.\n` +
      `Question: ${req.question}\nWhy it blocks: ${req.why_blocking}\n` +
      `Suggested method: ${req.method_hint || 'your choice'}\nDone when: ${req.done_when || 'the question has an answer with evidence'}\n` +
      `First run: vsdd -C ${RUN.dir} spike open --run ${RUN.id} --step ${stepKey} --ref ${safe(req.ref) || 'integration'} --json\n` +
      `If it returns an existing finding, return that finding with cached = true and do nothing else. If it refuses, return ` +
      `a finding with verdict "refused" and its reason. Otherwise run the experiment inside your own worktree only. Write the ` +
      `finding as JSON to a file in the temporary directory, then run: vsdd -C ${RUN.dir} spike record --id <ledger_id> ` +
      `--json-file <that file>  and return the same finding.`,
    {
      agentType: P + 'spike-runner', model: tier.model, effort: tier.effort, schema: S_SPIKE_FINDING, phase: phaseTitle,
      label: `spike ${RUN.spikes}: ${req.question.slice(0, 48)}`,
      isolation: 'worktree', // always, research included: a step does not inherit the agent file's isolation
    },
  )
  return f || { ledger_id: '', question: req.question, verdict: 'inconclusive', answer: 'The spike agent returned nothing.', method: '', evidence: [], confidence: 0, cached: false }
}

// Run one step. o.method is required. Retries an unusable result, and serves spike requests within the limits.
// o.spikes, when given, is the list gated() carries across its attempts.
async function step(type, name, subject, task, o = {}) {
  const tier = RUN.tier || TIER[OVERRIDE[`${RUN.wf}/${o.stage || name}`] || ROLE[type]]
  const id = { run: RUN.id, workflow: RUN.wf, name, subject }
  const spikes = o.spikes || []
  let bad = 0
  for (let n = 1; ; n++) {
    if (RUN.agents >= RUN.limits.agents || (budget.total && budget.remaining() < 20000))
      return made(id, 'blocked', 'agent_budget', 'The agent or token budget for this run is spent.')
    RUN.agents++
    const r = await agent(brief(id, n, task, spikes, o), {
      agentType: P + type, model: tier.model, effort: tier.effort, schema: stepSchema(o.data),
      phase: o.phase, label: `${subject} · ${name} #${n}`,
    })
    if (!r || !r.step || r.step.name !== name || r.step.subject !== subject) {
      if (++bad >= RUN.limits.attempts) return made(id, 'failed', 'other', 'The step returned no usable result.')
      continue
    }
    if (r.status !== 'needs_spike') return r
    const q = r.spike_request
    if (!q || !q.question) return made(id, 'failed', 'other', 'The step asked for a spike without a question.')
    const over = spikes.length >= RUN.limits.spikes_per_step || RUN.spikes >= RUN.limits.spikes_per_run
    if (over) return { ...r, status: 'blocked', blocked: { kind: 'spike_limit', reason: 'The spike limit for this step or run is reached.', question: q.question } }
    // Refused above before any agent starts. The tool also counts per run id, so parent and children share the limit.
    const f = await runSpike({ ...q, ref: q.ref || r.head || '' }, `${RUN.wf}/${subject}/${name}.${n}`, o.phase)
    if (f.verdict === 'refused') return { ...r, status: 'blocked', blocked: { kind: 'spike_limit', reason: f.answer, question: q.question } }
    spikes.push(f)
  }
}

// Run a step until the gate it must finish with passes on the commit it left behind. o.closing is that gate's
// command. Spike findings are carried from one attempt to the next, so a retry does not ask again. With o.each
// the step returns one entry for the gate per item: every entry must pass and have its unit in data.units, and
// the retry note names each one that did not.
async function gated(type, name, subject, task, gate, o = {}) {
  let note = ''
  const spikes = []
  for (let i = 1; i <= RUN.limits.attempts; i++) {
    const r = await step(type, i === 1 ? name : `${name}-retry${i - 1}`, subject, task + note, { ...o, spikes })
    if (r.status !== 'done') return r
    const all = (r.gates || []).filter((x) => x.gate === gate)
    const gs = o.each ? all : all.slice(0, 1)
    const units = (r.data && r.data.units) || []
    const ok = (g) => g.passed && g.sha === r.head
    const bad = gs.filter((g) => !ok(g) || (o.each && !units.some((u) => u.id === g.subject)))
    if (gs.length && !bad.length) return r
    const why = bad.map((g) => (o.each ? `${g.subject}: ` : '') + (ok(g) ? 'passed, but its unit is not in data.units' : g.failures.join('; ')))
    note = `\nThe ${gate} gate did not pass last time: ${gs.length ? why.join(' | ').slice(0, 1500) : 'no gate result was returned'}. Deal with that first.` +
      (o.each ? ` Then run the closing command again for every ${o.each}, and return every unit.` : '')
  }
  return made({ run: RUN.id, workflow: RUN.wf, name, subject }, 'blocked', 'gate', `The ${gate} gate still fails after ${RUN.limits.attempts} attempts.`)
}

// Run one vsdd command through the clerk and return its JSON output. The command is a single segment,
// "vsdd -C <dir> ...", with no cd in front and no redirect, so the clerk's wall and the auto-allow rule accept
// it. A long command returns state "pending" while its job runs; the clerk runs the same command again. One
// retry if the clerk returns nothing.
async function relay(name, subject, command, schema, phaseTitle) {
  const full = RUN.dir ? command.replace(/^vsdd /, `vsdd -C ${RUN.dir} `) : command
  for (let n = 1; n <= 2; n++) {
    RUN.agents++
    const r = await agent(
      `Run exactly this command, with a Bash timeout of 600000, and return the JSON it prints as your result. ` +
        `If the JSON has state "pending", run the same command again, and repeat until it does not. ` +
        `Do not interpret, repair or summarise it. If it prints no JSON, return {"ok": false, "error": "<what it printed>"}.\n${full}`,
      {
        agentType: P + 'clerk', model: (RUN.tier || TIER.clerk).model, effort: (RUN.tier || TIER.clerk).effort, schema,
        phase: phaseTitle, label: `${subject} · ${name}${n > 1 ? ' (retry)' : ''}`,
      },
    )
    if (r) return r
  }
  return { ok: false, error: 'The clerk step returned nothing, twice.' }
}

// Start child stories. Used by wave, verify, feature, maintain and multi. A batch starts only if it fits
// beside the reserve for the close. Each child gets its own agent budget and its share of the run's spikes.
// A child that throws or returns nothing is charged its full budget, because it may have spent it.
async function runStories(units, extra = {}) {
  const per = RUN.limits.story_agents
  const fit = Math.max(0, Math.floor((RUN.limits.agents - RUN.agents - CLOSE_RESERVE) / per))
  const batch = units.slice(0, Math.min(fit, RUN.limits.stories_per_run))
  const share = Math.max(1, Math.floor((RUN.limits.spikes_per_run - RUN.spikes) / Math.max(1, batch.length)))
  const lost = (u, why) => ({ subject: u.id, status: 'failed', summary: why, deferred: [], agents: per, spikes: 0 })
  const rs = await parallel(batch.map((u) => async () => {
    try {
      return await workflow(`${P}story`, {
        ...extra, story: u.id, position: u, run_id: RUN.id, project_dir: RUN.dir, plugin_root: RUN.root, tier: RUN.tier,
        limits: { ...RUN.limits, agents: per, spikes_per_run: share },
      })
    } catch (e) {
      return lost(u, String(e).slice(0, 300))
    }
  }))
  const results = rs.map((r, i) => r || lost(batch[i], 'The story run returned nothing.'))
  results.forEach((r) => { RUN.agents += r.agents || per; RUN.spikes += r.spikes || 0 })
  return { results, skipped: units.slice(batch.length) }
}

// The convergence rule (section 2.7).
const HARD = ['critical', 'high']
const REACH = {
  spec: ['unit', 'integration', 'system', 'spec'], plan: ['unit', 'integration', 'system', 'spec'],
  story: ['unit'], wave: ['unit', 'integration'], release: ['unit', 'integration', 'system'],
}
function judge(findings, perimeter, waived) {
  const v = { blocking: [], advisory: [], deferred: [], spec_gaps: [] }
  for (const f of findings) {
    // A holdout finding is built from scenario scores and has no file to point at, so it needs no location.
    const placed = f.category === 'holdout' || Boolean(f.location && f.location.path)
    const hard = HARD.includes(f.severity) && Boolean(f.spec_ref || f.repro) && placed
    if (waived.includes(f.key)) v.advisory.push(f)
    else if (REACH[perimeter].includes(f.scope)) (hard ? v.blocking : v.advisory).push(f)
    else if (f.scope === 'spec') (hard ? v.spec_gaps : v.advisory).push(f)
    else v.deferred.push(f)
  }
  return v
}
// c.review(pass, claims, gaps) -> { ok, head, findings, claim_checks, coverage_missing },
//                                 or { ok: false, blocked: true } when a reviewer returned blocked
// c.fix(open, advisory, pass)  -> { ok, resolutions: [{ key, resolution: 'fixed' | 'disputed', commit, note }] }
// c.arbitrate(open)            -> { rulings: [{ key, ruling: 'defect' | 'not_defect' | 'spec_gap', reason }] }   (optional)
// c.used                       -> passes or rounds this unit already used in earlier runs (from the survey)
async function converge(c) {
  const raised = new Map()
  const waived = (c.waived || []).slice()
  const dismissed = []
  const disputed = new Set()
  const used = c.used || 0
  let claims = [], open = [], advisory = [], deferred = [], gaps = [], head = '', pass = 0, extra = 0
  const verdict = (outcome, reason) => ({
    perimeter: c.perimeter, subject: c.subject, head, passes: pass, rounds_used: used + pass, outcome, reason,
    blocking: open, advisory, deferred, dismissed, coverage_missing: gaps,
  })
  // An item that no reviewer marked checked is itself a finding, so the fixer is called and the loop can end.
  const uncovered = (item) => ({
    key: `coverage:${c.subject}:${item}`, title: `Not accounted for: ${item}`, severity: 'high', category: 'coverage',
    scope: REACH[c.perimeter][0], owner: 'test-writer', location: { path: '' }, spec_ref: item, repro: '',
    observed: 'No reviewer marked this item checked.', expected: 'A test or another evidence route covers it.',
  })
  while (used + pass < c.cap + extra) { // the cap counts across runs
    pass++
    const r = await c.review(pass, claims, gaps)
    if (r && r.blocked) return verdict('blocked', 'reviewer_blocked') // a decision opens at once; not a failed run
    if (!r || !r.ok) return verdict('failed', 'no_result')
    head = r.head
    const j = judge(r.findings, c.perimeter, waived)
    j.blocking.forEach((f) => raised.set(f.key, f))
    const checked = new Map((r.claim_checks || []).map((k) => [k.key, k.outcome]))
    for (const k of claims) if (checked.get(k.key) === 'withdrawn') dismissed.push({ key: k.key, by: 'reviewer', reason: 'Withdrawn on re-review.' })
    const still = claims.filter((k) => !['resolved', 'withdrawn'].includes(checked.get(k.key))).map((k) => raised.get(k.key))
    gaps = (r.coverage_missing || []).filter((item) => !waived.includes(`coverage:${c.subject}:${item}`))
    const cov = gaps.map(uncovered)
    cov.forEach((f) => raised.set(f.key, f))
    open = j.blocking.concat(still.filter((f) => f && f.category !== 'coverage' && !j.blocking.some((b) => b.key === f.key)), cov)
    advisory = j.advisory
    deferred = deferred.concat(j.deferred)
    if (j.spec_gaps.length) { open = j.spec_gaps; return verdict('blocked', 'spec_gap') }
    if (!open.length) return verdict('converged', 'clean')
    if (used + pass >= c.cap + extra) {
      if (extra || !c.arbitrate) break
      const a = await c.arbitrate(open)
      const ruled = new Map(((a && a.rulings) || []).map((x) => [x.key, x]))
      const upheld = []
      for (const f of open) {
        const x = ruled.get(f.key)
        if (x && x.ruling === 'spec_gap') { open = [f]; return verdict('blocked', 'spec_gap') }
        // Only a finding the builder disputed can be dismissed, and never a critical or security finding.
        if (x && x.ruling === 'not_defect' && disputed.has(f.key) && f.severity !== 'critical' && f.category !== 'security') {
          dismissed.push({ key: f.key, by: 'arbiter', reason: x.reason })
          waived.push(f.key)
        } else upheld.push(f)
      }
      open = upheld
      if (!open.length) return verdict('converged', 'clean')
      extra = 1
    }
    const fx = await c.fix(open, advisory, pass)
    if (!fx || !fx.ok) return verdict('failed', 'no_result')
    const how = new Map((fx.resolutions || []).map((x) => [x.key, x]))
    // A claim is structured data: no title, no quote, none of the fixer's text. A holdout claim is the key alone.
    claims = open.map((f) => {
      const h = how.get(f.key) || {}
      if (h.resolution === 'disputed') disputed.add(f.key)
      const where = f.category === 'holdout' ? {} : { path: f.location.path, line_start: f.location.line_start, line_end: f.location.line_end }
      return { key: f.key, ...where, resolution: h.resolution || 'fixed', commit: h.commit || '' }
    })
  }
  return verdict('blocked', 'cap')
}
// <<< stamped:prelude
```

The prelude also holds the remaining schema literals (`S_SURVEY`, `S_CLOSE`, `S_REVIEW`, `S_FIXES`,
`S_RULINGS`, `S_DELIVERY`, `S_WAVE_GATE`, `S_HOLDOUT`) and `holdoutFindings()`, which turns scenario scores
into findings in the shape section 2.7 fixes (category `holdout`, key `holdout:<HS id>` or
`holdout:<unit>:mean` or `:spread`, no location). They are left out here.

#### 2.11.2 `story.js`

```js
export const meta = {
  name: 'story',
  description: 'Deliver one story: failing tests, implementation, review to convergence, evidence, merge',
  whenToUse: 'Run for one story id. Normally started by the wave, feature, verify, maintain or multi workflows.',
  phases: [
    { title: 'Survey' }, { title: 'Tests' }, { title: 'Implement' },
    { title: 'Review' }, { title: 'Evidence' }, { title: 'Deliver' },
  ],
}

// >>> prelude … <<< prelude   (section 2.11.1, stamped here)

RUN.wf = 'story'
const a = parseArgs(args, 'story', ['repo'])
const sid = safe(a.story)
if (!sid) return { workflow: 'story', subject: '', status: 'failed', summary: 'No valid story id was given.' }

phase('Survey')
let u = a.position // set when a parent passes a unit the tool returned: from its survey, or from an admission
let surveyNext = null
if (u) {
  RUN.id = a.run_id; RUN.dir = a.project_dir; RUN.root = a.plugin_root; RUN.tier = a.tier || null
  RUN.limits = { ...RUN.limits, ...(a.limits || {}) } // the parent passes agents = story_agents and this child's share of spikes
} else {
  const s = await relay('survey', sid, `vsdd survey story --subject ${sid}${a.repo ? ` --repo ${safe(a.repo)}` : ''} --json`, S_SURVEY, 'Survey')
  const unmet = s.ok ? s.preconditions.filter((p) => !p.met) : []
  if (!s.ok || unmet.length || !s.units.length)
    return { workflow: 'story', subject: sid, status: 'blocked', summary: s.error || unmet.map((p) => `${p.name}: ${p.detail}`).join('; '), next: s.next }
  RUN.id = s.run_id; RUN.dir = s.project_dir; RUN.root = s.plugin_root; RUN.tier = s.config.tier_override || null
  RUN.limits = { ...RUN.limits, ...s.limits }
  u = s.units[0]
  surveyNext = s.next
}

const ORDER = ['ready', 'red', 'green', 'reviewed', 'evidenced', 'delivered']
const before = (p) => ORDER.indexOf(u.position) < ORDER.indexOf(p)
const out = { workflow: 'story', subject: sid, passes: 0, advisory: 0, deferred: [], pr: '', merged_sha: '' }
const end = async (status, summary, failed) => {
  let next = surveyNext
  if (!a.position) { // standalone run: the parent's checkpoint does this otherwise
    const c = await relay('close', sid, `vsdd checkpoint story --run ${RUN.id} --subject ${sid}${failed ? ' --outcome failed' : ''} --json`, S_CLOSE, 'Deliver')
    next = c.next || next
  }
  return { ...out, status, summary, next, agents: RUN.agents, spikes: RUN.spikes }
}
const where = `Story ${sid}. Work only in the worktree ${u.worktree}. Story file: ${u.story_path}. Contracts: ${u.bcs.join(', ')}.`
const gate = (name) => `gate ${name} --story ${sid} --run ${RUN.id}` // a closing command; brief() adds "vsdd -C <dir>" and the wait
const pick = (r, name) => (r.gates || []).find((x) => x.gate === name) || {}
const KIND = { // story kind -> the agent and method of each stage (blueprint 4.6); a story's own `method` key wins
  feature: { tests: ['test-writer', 'failing-tests'], impl: ['implementer', 'tdd'] },
  fix: { tests: ['test-writer', 'regression-test'], impl: ['implementer', 'fix-findings'] },
  port: { tests: ['test-writer', 'equivalence-tests'], impl: ['implementer', 'port-translation'] },
  facade: { impl: ['implementer', 'service-clone'] },
  docs: { impl: ['technical-writer', 'docs'] },
  chore: { impl: ['devops', 'repo-setup'] },
  harness: { impl: ['verifier', 'proofs'] },
}
const K = KIND[u.kind] || KIND.feature
let head = u.head
let patch = u.diff_patch // the latest diff patch; each green gate result replaces it

try { // everything after a successful survey is inside try, so the checkpoint always runs (the body is not re-indented here)

if (before('red') && K.tests) {
  phase('Tests')
  const r = await gated(K.tests[0], 'tests', sid,
    `${where} Write a failing test for every acceptance criterion, with the stubs the tests need to build, and commit them.`,
    'red', { phase: 'Tests', method: K.tests[1], closing: gate('red'), dirty: u.dirty })
  if (r.status !== 'done') return end(r.status === 'failed' ? 'failed' : 'blocked', r.summary)
  head = r.head
}

if (before('green')) {
  phase('Implement')
  const r = await gated(K.impl[0], 'implement', sid,
    `${where} ` + (K.tests
      ? 'Make the failing tests pass one at a time with the least code. The tests frozen at the red gate are read-only for you.'
      : 'Build what the story asks for.'),
    'green', { phase: 'Implement', method: u.method || K.impl[1], closing: gate('green'), dirty: u.dirty })
  if (r.status !== 'done') return end(r.status === 'failed' ? 'failed' : 'blocked', r.summary)
  head = r.head; patch = pick(r, 'green').diff_patch || patch
}

// One review loop, used for the full diff and again for any commits added after it converged.
const lenses = [['adversary', 'review-story-diff']]
  .concat(['critical', 'high'].includes(u.criticality) || u.security ? [['security-reviewer', 'security-review']] : [])
const testOwned = (f) => f.owner === 'test-writer' // findings of category test or coverage
const review = (from, cap, must, used) => converge({
  perimeter: 'story', subject: sid, cap, used, waived: u.waived || [],
  review: async (pass, claims, gaps) => {
    const rs = await parallel(lenses.map(([l, m]) => () => step(l, `review-${l}-p${pass}`, sid,
      `${where} Tree root: ${u.tree}. Review the diff patch ${patch} (${from}...${head}) against the story and its contracts. ` +
        `Test output: ${u.test_output}. Review rules: ${u.review_rules || 'none'}. Account for each of: ${must.join(', ')}. ` +
        `Claims to verify from the last pass: ${JSON.stringify(claims)}. Not yet covered: ${gaps.join(', ') || 'nothing'}.`,
      { phase: 'Review', method: m, data: S_REVIEW })))
    if (rs.some((r) => r && r.status === 'blocked')) return { ok: false, blocked: true } // a reviewer refused, or lacked an input
    if (rs.some((r) => !r || r.status !== 'done' || r.data.reviewed_sha !== head)) return { ok: false }
    const covered = (item) => rs.some((r) => r.data.coverage.some((c) => c.item === item && c.state !== 'skipped'))
    return {
      ok: true, head, findings: rs.flatMap((r) => r.data.findings), claim_checks: rs.flatMap((r) => r.data.claim_checks),
      coverage_missing: must.filter((item) => !covered(item)),
    }
  },
  fix: async (open, advisory, pass) => {
    // Routed on owner. The test-writer goes first: only it may change a frozen test, and it then records the
    // change with the re-freeze, so the green gate that follows does not fail on "frozen tests unchanged".
    // A strengthened test may fail on the current code. The re-freeze accepts that, and the implementer then
    // runs even with no finding of its own, to make the re-frozen tests pass.
    let resolutions = [], red = false
    for (const who of ['test-writer', 'implementer']) {
      const tw = who === 'test-writer'
      const mine = open.filter((f) => testOwned(f) === tw)
      if (!mine.length && (tw || !red)) continue
      const r = await gated(who, `fix-${who}-p${pass}`, sid,
        `${where} Fix each blocking finding, or dispute it with evidence: ${JSON.stringify(mine)}. ` +
          (red && !tw ? 'Tests the test-writer strengthened and re-froze now fail on this code: make them pass. ' : '') +
          `Fix these too where it is cheap: ${JSON.stringify(advisory.filter((f) => testOwned(f) === tw).slice(0, 10))}.` +
          (tw ? ` After the re-freeze below, also run: vsdd -C ${RUN.dir} ${gate('green')} --wait 540 --json  and add its output to gates.` : ''),
        tw ? 'red' : 'green',
        { phase: 'Review', method: tw ? 'test-fixes' : 'fix-findings', closing: tw ? `${gate('red')} --refreeze` : gate('green'), data: S_FIXES })
      if (r.status !== 'done') return { ok: false }
      head = r.head; patch = pick(r, 'green').diff_patch || patch
      if (tw) red = !pick(r, 'green').passed // no passing green result after the re-freeze: the code must change
      resolutions = resolutions.concat(r.data.resolutions)
    }
    return { ok: true, resolutions }
  },
  arbitrate: async (open) => {
    const r = await step('architect', 'arbitrate', sid,
      `${where} A reviewer and a builder still disagree after ${cap} passes. Rule on each finding: ${JSON.stringify(open)}`,
      { phase: 'Review', method: 'arbitrate', data: S_RULINGS })
    return r.status === 'done' ? r.data : null
  },
})

if (before('reviewed')) {
  phase('Review')
  const v = await review(u.base, RUN.limits.passes, u.acs, u.rounds_used || 0)
  out.passes = v.passes; out.deferred = v.deferred; out.advisory = v.advisory.length
  if (v.outcome !== 'converged') return end(v.outcome === 'failed' ? 'failed' : 'blocked', `Review did not converge (${v.reason}).`)
}

if (before('evidenced') && u.demo_required) {
  phase('Evidence')
  const r = await gated('demo-recorder', 'evidence', sid,
    `${where} Record evidence for each user-visible acceptance criterion.`,
    'demo', { phase: 'Evidence', method: 'demo', closing: gate('demo') })
  if (r.status !== 'done') return end('blocked', r.summary)
}

if (before('delivered')) {
  phase('Deliver')
  for (let round = 1; ; round++) {
    // One clerk covers the whole delivery, the CI wait included: the command re-attaches to its job while it is pending.
    const d = await relay(`deliver-${round}`, sid, `vsdd story deliver --story ${sid} --run ${RUN.id} --wait 540 --json`, S_DELIVERY, 'Deliver')
    if (d.state === 'merged') { out.pr = d.pr; out.merged_sha = d.merged_sha; break }
    if (d.state === 'awaiting_merge') return end('needs_human', `PR ${d.pr} passed every gate and waits for a person's merge decision.`)
    if (!d.ok || round >= RUN.limits.delivery_rounds || !['ci_failed', 'conflict', 'review_stale'].includes(d.state))
      return end('blocked', d.error || `Delivery stopped in state ${d.state}.`)
    patch = d.diff_patch || patch // the patch from the reviewed commit to the head
    if (d.state !== 'review_stale') {
      const r = await gated('implementer', `repair-${round}`, sid, `${where} Delivery failed (${d.state}): ${d.detail} Repair it.`,
        'green', { phase: 'Deliver', method: 'fix-findings', closing: `${gate('green')} --since ${safe(d.reviewed_sha)}` })
      if (r.status !== 'done') return end('blocked', r.summary)
      head = r.head; patch = pick(r, 'green').diff_patch || patch // with --since the gate writes the patch from that commit
    }
    const v = await review(d.reviewed_sha, 2, d.changed_files, 0) // only the commits added since the converged review
    if (v.outcome !== 'converged') return end('blocked', `Review of the repair did not converge (${v.reason}).`)
  }
}
return end('done', `Story ${sid} is merged at ${out.merged_sha || u.head}.`)

} catch (e) {
  return end('failed', `The script stopped on an error: ${String(e).slice(0, 300)}`, true)
}
```

A pass with both test findings and code findings runs the test-writer first and the implementer second, as
the `fix` function shows. The implementer also runs when the pass had test findings only and a re-frozen test
fails on the current code.

#### 2.11.3 `wave.js`

```js
export const meta = {
  name: 'wave',
  description: 'Build one wave: deliver its stories in parallel, then run the wave gate to convergence',
  whenToUse: 'Run after the plan is ready. With no argument it takes the next open wave.',
  phases: [{ title: 'Survey' }, { title: 'Stories' }, { title: 'Gate' }, { title: 'Fix' }, { title: 'Close' }],
}

// >>> prelude … <<< prelude   (section 2.11.1, stamped here)

RUN.wf = 'wave'
const a = parseArgs(args, 'wave', ['repo'])

phase('Survey')
const s = await relay('survey', 'wave', `vsdd survey wave${a.wave ? ` --subject ${safe(a.wave)}` : ''}${a.repo ? ` --repo ${safe(a.repo)}` : ''} --json`, S_SURVEY, 'Survey')
const unmet = s.ok ? s.preconditions.filter((p) => !p.met) : []
if (!s.ok || unmet.length)
  return { workflow: 'wave', subject: a.wave || '', status: 'blocked', summary: s.error || unmet.map((p) => `${p.name}: ${p.detail}`).join('; '), next: s.next }
RUN.id = s.run_id; RUN.dir = s.project_dir; RUN.root = s.plugin_root; RUN.tier = s.config.tier_override || null
RUN.limits = { ...RUN.limits, ...s.limits }
const W = safe(s.subject)

const close = async (status, summary, failed) => {
  phase('Close')
  const c = await relay('close', W, `vsdd checkpoint wave --run ${RUN.id} --subject ${W}${failed ? ' --outcome failed' : ''} --json`, S_CLOSE, 'Close')
  return {
    workflow: 'wave', subject: W, status: c.ok ? c.status || status : 'failed', summary, progress: Boolean(c.progress),
    decisions: c.decisions || [], next: c.next, counts: { agents: RUN.agents, spikes: RUN.spikes },
  }
}
let short = false // set when a fix round did not fit beside the reserve for the close

try { // everything after a successful survey is inside try, so the checkpoint always runs (the body is not re-indented here)

phase('Stories')
const todo = s.units.filter((u) => u.position !== 'delivered' && !u.deferred && !(u.blocked && u.blocked.open))
// runStories() is in the prelude: each child gets story_agents, and a batch starts only if it fits beside the reserve.
const first = todo.length ? await runStories(todo, { repo: a.repo }) : { results: [], skipped: [] }
const built = first.results
if (first.skipped.length) {
  log(`${first.skipped.length} stories of ${W} are left for the next run of this command.`)
  return close('partial', `${built.filter((r) => r.status === 'done').length} of ${todo.length} open stories delivered in this run.`)
}

phase('Gate')
// --settle moves stories that wait on a person to a later wave when the plan allows it, and reports what still blocks the gate.
// For a UI wave the same job runs the UI check.
const gateCmd = (settle) => `vsdd gate wave --wave ${W} --run ${RUN.id}${settle ? ' --settle' : ''}${a.repo ? ` --repo ${safe(a.repo)}` : ''} --wait 540 --json`
let g = await relay('integration-r1', W, gateCmd(true), S_WAVE_GATE, 'Gate')
if (!g.ok || g.pending.length)
  return close(g.ok ? 'partial' : 'failed', g.ok ? `Stories not yet delivered: ${g.pending.join(', ')}.` : g.error)

const carried = built.flatMap((r) => r.deferred || [])
const v = await converge({
  perimeter: 'wave', subject: W, cap: RUN.limits.rounds, used: s.rounds_used || 0, waived: s.waived || [],
  review: async (round, claims) => {
    if (round > 1) g = await relay(`integration-r${round}`, W, gateCmd(false), S_WAVE_GATE, 'Gate')
    if (!g.ok) return { ok: false }
    if (!g.passed) return { ok: true, head: g.sha, findings: g.findings, claim_checks: [], coverage_missing: [] }
    const ctx = `Wave ${W}. Integration worktree: ${g.worktree}. Diff patch: ${g.diff_patch} (${g.base}...${g.sha}). Stories: ${g.stories.join(', ')}. Claims to verify: ${JSON.stringify(claims)}.`
    const checks = [
      () => step('adversary', `integration-review-r${round}`, W,
        `${ctx} Review how these stories work together. Findings the story reviews passed up, as scope to check: ${JSON.stringify(carried)}`,
        { phase: 'Gate', method: 'review-wave', data: S_REVIEW }),
      ...(g.needs.security ? [() => step('security-reviewer', `security-r${round}`, W, `${ctx} Review the security of the combined change.`,
        { phase: 'Gate', method: 'security-review', data: S_REVIEW })] : []),
      ...(g.needs.ui ? [() => step('ui-reviewer', `ux-review-r${round}`, W, `${ctx} Review the interface. Start the product with: vsdd -C ${RUN.dir} env start --subject ${W}`,
        { phase: 'Gate', method: 'ui-review', data: S_REVIEW })] : []),
      // The evaluator is given scenario ids and nothing about the code. It returns scores and symptoms, never findings.
      ...g.holdout_batches.map((ids, i) => () => step('holdout-evaluator', `holdout-r${round}-b${i + 1}`, W,
        `Evaluate the hidden scenarios ${ids.join(', ')} against the running product. Start it with: vsdd -C ${RUN.dir} env start --subject ${W}`,
        { phase: 'Gate', method: 'holdout-eval', data: S_HOLDOUT })),
    ]
    const rs = await parallel(checks)
    if (rs.some((r) => r && r.status === 'blocked')) return { ok: false, blocked: true }
    if (rs.some((r) => !r || r.status !== 'done')) return { ok: false }
    return {
      ok: true, head: g.sha, coverage_missing: [],
      // holdoutFindings() builds findings of category holdout from the scores: no location, key holdout:<HS id>.
      findings: rs.flatMap((r) => r.data.findings || []).concat(holdoutFindings(rs, s.config.holdout, W)),
      claim_checks: rs.flatMap((r) => r.data.claim_checks || []),
    }
  },
  fix: async (open, advisory, round) => {
    phase('Fix')
    // When the measured gate failed, its output already holds the admitted fix units. Otherwise the tool recomputes the
    // verdict from the stored pass records, writes one fix story per group of blocking findings and admits each.
    let units = g.fix_units || []
    if (!units.length) {
      const nf = await relay(`new-fix-r${round}`, W, `vsdd story new-fix --wave ${W} --round ${round} --run ${RUN.id} --json`, S_SURVEY, 'Fix')
      if (!nf.ok || !nf.units.length) return { ok: false }
      units = nf.units
    }
    const { results: rs, skipped } = await runStories(units, { repo: a.repo })
    if (skipped.length) { short = true; return { ok: false } } // the round did not fit: return partial, keep the reserve
    phase('Gate')
    const merged = new Map(rs.filter((r) => r.status === 'done').flatMap((r) => ((units.find((u) => u.id === r.subject) || {}).fixes || []).map((k) => [k, r.merged_sha || ''])))
    // Every open finding gets a claim, so none is lost. A fix story that was not delivered is never recorded as a
    // dispute: its claim is checked by the next round and comes back unresolved.
    return { ok: true, resolutions: open.map((f) => ({ key: f.key, resolution: 'fixed', commit: merged.get(f.key) || '', note: '' })) }
  },
  arbitrate: async (open) => {
    const r = await step('architect', 'arbitrate', W, `Wave ${W}. Rule on each finding that is still open after ${RUN.limits.rounds} rounds: ${JSON.stringify(open)}`,
      { phase: 'Gate', method: 'arbitrate', data: S_RULINGS })
    return r.status === 'done' ? r.data : null
  },
})
if (short) return close('partial', `A fix round for ${W} did not fit in this run. Run the same command again.`)
return close(v.outcome === 'converged' ? 'done' : v.outcome === 'failed' ? 'failed' : 'needs_human',
  v.outcome === 'converged' ? `Wave ${W} passed its gate after ${v.passes} round(s).` : `Wave ${W} gate did not converge (${v.reason}); ${v.blocking.length} finding(s) open.`)

} catch (e) {
  return close('failed', `The script stopped on an error: ${String(e).slice(0, 300)}`, true)
}
```

`verify`, `feature`, `maintain` and `multi` call the same `runStories()` and treat its `skipped` list the same
way, which is how the reserve check of section 2.5 reaches every workflow that runs child stories.

Agent counts from the simulation, for scale: a story with a clean first review used 4 agents as a child
(tests, implement, review, deliver) and 6 standalone; with one finding fixed, 6 and 8. A two-story wave with
a clean gate used 13 agents, and 21 with one integration finding fixed through a fix story. Those are the
floor, with every step succeeding first time. The planning figure for a realistic story, with a second lens,
a fix round and an evidence step, is 8 to 12 agent starts, and 70 to 90 for a six-story wave (blueprint
section 10, item 22); the end-to-end tests budget 12 per story until they have measured it. The old
`code-delivery.lobster` had 32 named steps before its loops and its state backups.

### 2.12 What changed from the old flow, and why

| # | Old flow | What was wrong | 1.0 |
|---|---|---|---|
| 1 | 17 `.lobster` files read by an agent and dispatched by hand | No runtime: order, loops, gates and walls were on the honour system | JavaScript workflows execute the control flow |
| 2 | Mode file calls phase file calls code delivery, three levels | The platform allows one level | Routes are computed; only `story` is a child |
| 3 | 37 approval steps | Most approved things code can check, and seven only approved a retrospective | Three routine stops; others are conditional or queued |
| 4 | Approvals were prompts inside a run | A run cannot take input, and nothing stopped an agent recording "approved" | Stops are workflow boundaries; the record is written from the person's keystroke |
| 5 | A state-manager "backup" step after almost every step (about sixty) | It was the bookkeeping that swamped delivery | The hook runner stores each step result; one checkpoint per run |
| 6 | Resume by reading a 231 KB state file | Unbounded, and stale | Survey computed from artifacts, evidence and git on every start |
| 7 | Stubs, then tests, then an orchestrator-run Red Gate with `cargo` | Two dispatches, a gate run by the party that wants to proceed, one toolchain | One step that ends with the red-gate command from project configuration; a hook blocks source edits until the evidence exists |
| 8 | The implementer could edit the tests it had to pass | Tests could be weakened to go green | Tests are frozen by hash at the red gate; only the test-writer may change them |
| 9 | Three review loops per story: adversary (three clean passes), pull-request reviewer (ten rounds), security | The same diff reviewed three ways with three verdicts, and the pull-request loop needed agents to dispatch agents, which a step cannot do | One loop with one or two lenses and one verdict bound to the head commit |
| 10 | Three consecutive clean passes, novelty and decay metrics | Could not terminate; the metrics were never computable | One clean pass on the final commit, anchored blocking findings, cap, arbitration, person |
| 11 | "Zero findings on a first pass is suspicious; rerun" | Punished good work and invited invented findings | Coverage accounting |
| 12 | Secondary "Gemini" review loops and "different model family" gate criteria | Those models were never wired in | Diversity comes from lenses and perimeters; the tiers are set on every step |
| 13 | Wave gate: six gates in sequence, run by an agent typing commands | Slow, and unverifiable | The tool measures; reviews and scenarios run in parallel; one loop |
| 14 | Wave gate switched the main checkout to `develop` | It took over the person's working tree | The tool owns an integration worktree |
| 15 | Integration findings fixed by an implementer on the merged branch, and fix deliveries that skipped the Red Gate | A second, weaker path to the product branch | Fixes are stories and go through the one playbook with a regression test |
| 16 | One blocked story held its wave, and the wave held the pipeline | High touch | The story is deferred with its dependents; the decision is queued |
| 17 | Phases 4, 5, 6, 7 in sequence, each feeding back to another, "re-run only failing checks" | Checks of one commit were serialised, and skipping passed checks after a fix is unsound | One `verify` workflow; due checks in parallel; evidence tied to the commit decides what reruns |
| 18 | Scenario rotation and wave scheduling done by agents | Not reproducible | The tool does both; rotation is seeded from the commit |
| 19 | Phase 2 and F3 human approval of stories | Coverage, cycles and size are checkable by code | No stop by default; the plan gate enforces them |
| 20 | F1 human approval of scope and route | The route follows from the delta facts | Computed, stored, and re-checked at the green gate |
| 21 | Market intelligence as its own stop in four modes | A stop with no separate decision | Part of the `brief` decision; off by default for features |
| 22 | Story split recovery with two human approvals after the pull request was too large | Late and expensive | Size limit at the plan gate; large diffs are reviewed in parts |
| 23 | A recorded demo for every acceptance criterion, committed to the product branch (110 MB) | Bloat with little signal | A criterion-to-test table by default; recordings only for user-visible criteria, kept out of the product branch |
| 24 | pr-manager with nine steps, github-ops, and devops cleanup | Mechanical work done by three agents | One `vsdd story deliver` command through a clerk step |
| 25 | Autonomy levels with a composite score; budgets in dollars through a proxy | Neither was implemented | Two configuration switches; counters for agents, passes and spikes; the platform token budget |
| 26 | Specification changes during build made by whichever agent noticed | Unapproved changes to the contract | Locked specification; changes only through an amendment that ends at decision `spec` |
| 27 | Service clones built by a separate skill with a pre-phase gate | A parallel delivery path | Clones are stories of kind `facade` in the earliest wave |
| 28 | Planning workflow with intake approval and five routing levels | A workflow whose job was to decide which workflow to run | The route table, and the `intake` stage of `spec` for existing documents |
| 29 | Multi-repository sub-workflows with a changed working directory | Three levels of nesting | The `repo` argument and a thin coordinator |
| 30 | Session review as a blocking step with an approval in every mode | A stop that decided nothing about the product | A non-blocking stage of `release` |
| 31 | No provision for experiments; unknowns were guessed or escalated | Wrong guesses became findings many passes later | Spikes from any step, bounded and recorded |

## 3. Mapping from the old system

Marks: ported (same job, new form), merged (folded into something else), replaced (job done a different
way), dropped (gone, with the reason).

### 3.1 Workflow files

| Old file | Mark | 1.0 |
|---|---|---|
| `greenfield.lobster` (116 steps) | replaced | The greenfield route in section 2.4 |
| `feature.lobster` (121 steps) | replaced | `feature`, then the shared `plan`, `wave`, `verify`, `release` |
| `brownfield.lobster` (26 steps) | replaced | `ingest`, then `feature` or `spec` |
| `code-delivery.lobster` (32 steps) | ported | `story` |
| `discovery.lobster` (29 steps) | ported | `discover` in `scan` mode |
| `maintenance.lobster` (36 steps) | ported | `maintain` |
| `multi-repo.lobster` (62 steps) | replaced | `multi` and the `repo` argument |
| `planning.lobster` (24 steps) | dropped as a workflow | Its parts: environment check → `plan` readiness; artifact detection and routing → survey and route table; market intelligence → `discover` assess; brainstorming and brief → the brief skill; validation of existing documents → the `intake` stage of `spec`, through its `sources` argument. Reason: a workflow that only chooses a workflow is a route |
| `phases/phase-0-codebase-ingestion.lobster` | ported | `ingest` |
| `phases/phase-1-spec-crystallization.lobster` | ported | `spec` |
| `phases/phase-2-story-decomposition.lobster` | ported | `plan` |
| `phases/phase-3-tdd-implementation.lobster` | ported | `wave` and `story` |
| `phases/phase-4-holdout-evaluation.lobster` | merged | The scenarios check in the wave gate and in `verify` |
| `phases/phase-5-adversarial-refinement.lobster` | merged | The adversarial check in `verify` |
| `phases/phase-6-formal-hardening.lobster` | merged | The baseline and formal checks in `verify` |
| `phases/phase-7-convergence.lobster` | merged | The `verify` loop and its report; `release` |
| `phases/per-story-delivery.md` and `agents/orchestrator/per-story-delivery.md` | merged | `story`. The two disagreed; there is now one |

### 3.2 Modes and the fourteen documented paths

| Old path | Mark | 1.0 |
|---|---|---|
| 1 Greenfield | replaced | Route |
| 2 Greenfield to multi-repo | merged | Choice `topology=split` in decision `spec`, then `multi` stage `split` |
| 3 Brownfield | replaced | `ingest` then `feature` or `spec` |
| 4 Brownfield multi-repo | merged | `ingest` with `repo` for each repository, then `multi` stage `synthesize` |
| 5 Brownfield to multi-repo discovery | merged | Same as path 2 |
| 6 Feature, standard | ported | `feature` standard route |
| 7 Feature, trivial | ported | `feature` route `trivial` |
| 8 Feature, bug fix | ported | `feature` route `bugfix` |
| 9 Feature, critical bug; the hotfix flow | ported | `feature` route `hotfix`; merge-back in `release` |
| 10 Maintenance | ported | `maintain` |
| 11 Discovery | ported | `discover` |
| 12 Multi-repo | replaced | `multi` and `repo` |
| 13 Code delivery | ported | `story` |
| 14 Planning | dropped as a path | Route table (see 3.1) |
| Feature deprecation path | merged | `feature` with intent `deprecation` |
| Steady-state handoff, cycle archive | merged | `release` publish stage records the baseline |
| Product backlog in the state file | replaced | Queued change requests recorded by `feature`'s survey; storage is the state designer's |

### 3.3 Phases

| Old phase | 1.0 |
|---|---|
| 0 Codebase ingestion | `ingest` |
| 1 Spec crystallization, steps 1a to 1d | `spec` stages domain, contracts, architecture, ux, review |
| 2 Story decomposition | `plan` |
| 3 TDD implementation | `wave`, `story` |
| 4 Holdout evaluation | Scenarios check: wave gate and `verify` |
| 5 Adversarial refinement | Adversarial and security checks in `verify` |
| 6 Formal hardening | Baseline and formal checks in `verify` |
| 7 Convergence | `verify` loop and report, decision `release` |
| Release | `release` |
| F1 Delta analysis | `feature` stage delta and the route |
| F2 Spec evolution | `feature` stage evolve, checks, review |
| F3 Incremental stories | `plan` in delta scope |
| F4 Delta implementation | `wave` |
| F5 Scoped adversarial, F6 Targeted hardening, F7 Delta convergence | `verify` in delta scope |

### 3.4 Step kinds and recurring steps

| Old item | Mark | 1.0 |
|---|---|---|
| `type: agent` (280) | ported | `step()` |
| `type: skill` (130) | replaced | Agent steps. A step cannot rely on a skill that dispatches agents |
| `type: gate` (38) with prose criteria | replaced | `vsdd gate` commands and script conditions on their results |
| `type: human-approval` (37) | replaced | Decisions (section 2.6) |
| `type: sub-workflow` (25) | replaced | `workflow('vsdd-factory:story')` only |
| `type: loop` (24) | replaced | `converge()`, `gated()`, and bounded `for` loops |
| `parallel`, `parallel-foreach`, `compound` | ported | `parallel()` and `pipeline()` |
| `multi-repo` step type with `cwd` | replaced | `repo` argument |
| `context.include` / `exclude` walls (49 steps) | replaced | Hook walls keyed on agent type (need H3) |
| `model_tier` (24 steps, never interpreted) | replaced | `TIER` and `ROLE` tables; model and effort set on every step |
| `condition` pseudo-expressions | replaced | JavaScript conditions on result fields |
| `timeout`, `on_failure`, `max_retries` | replaced | Attempt limits, CI wait limit, the platform's stall restart |
| `cost_monitoring` | dropped | It had no implementation. Counters and the platform budget bound a run |
| state-manager backup steps after each step | dropped | Step records and the checkpoint make them unnecessary |
| `input-hash-drift-check` (five phase files) | replaced | The decision digest (a changed approved specification makes its decision stale) and explicit reopening through `revise`, `hold` or an amendment. There are no per-stage input hashes |
| `session-review`, `session-review-approval`, `process-review-decisions` (seven files) | merged | `release` stage retro; no approval |
| `scenario-rotation` by state-manager | replaced | Tool, seeded |
| `dx-engineer-preflight`, `pre-phase-4-cicd-gate`, `pre-phase-4-dtu-gate` | merged | `plan` readiness; clones are stories |
| `establish-demo-baseline`, bug-fix before and after comparison | merged | The evidence stage of a UI fix story records before (from the base commit) and after |
| `brownfield-full-regression`, `brownfield-codeowners-check` | merged | The green gate always runs the full suite; protected paths cover code owners |
| `dependency-merge-check` | dropped | Dependencies never sit inside one wave, and the survey refuses a story whose dependencies are not delivered |
| `post-feature-validation` | merged | `maintain` sweep post-release signals |
| `discovery-notifications`, notification table | replaced | The tool notifies when a decision opens |

### 3.5 The 37 approval steps

| Old approval | Count | Where | Mark | 1.0 |
|---|---|---|---|---|
| Brief approval | 1 | planning | ported | Decision `brief` |
| Market-intelligence review | 4 | planning, feature, brownfield, multi-repo | merged | In the `brief` packet; not run for features by default |
| Intake approval | 1 | planning | dropped | Routing is computed; nothing to decide |
| Phase 0 approval | 3 | brownfield, phase-0 file, multi-repo | replaced | `ingest` ends with a report and waits for the person's intent |
| Design-system approval | 2 | greenfield, brownfield | merged | Decision `spec` |
| Design-variant selection | 2 | greenfield, feature | merged | Choice `variant=` in decision `spec` |
| Phase 1 spec approval | 3 | greenfield, phase-1 file, multi-repo wave 0 | ported | Decision `spec` |
| Multi-repo confirmation | 1 | greenfield | merged | Choice `topology=` in decision `spec` |
| Phase 2 story approval | 2 | greenfield, phase-2 file | dropped by default | Plan gate; opt-in decision `plan` |
| F1 scope approval | 1 | feature | dropped | Route computed and re-checked |
| F2 spec-evolution approval | 1 | feature | ported | Decision `spec` on the delta |
| F3 stories approval | 1 | feature | dropped by default | As Phase 2 |
| Delivery approval | 1 | code delivery | ported, conditional | Decision `merge` |
| Release or convergence approval | 6 | greenfield, phase-7 file, feature F7, quick-dev, bug fix, multi-repo | ported | Decision `release` |
| Discovery review | 1 | discovery | ported | Decision `ideas` |
| Session-review approval | 7 | every top-level file | dropped | The retrospective is a report. Reason: it decides nothing about the product, and proposals about the plugin belong in the plugin's issue tracker |

### 3.6 Orchestrator files, loops and convergence material

| Old item | Mark | 1.0 |
|---|---|---|
| `orchestrator.md` operating loop, routing table, phase gate table | replaced | Scripts, the `ROLE` table, the route table |
| `greenfield-sequence.md`, `brownfield-sequence.md`, `feature-sequence.md`, `discovery-sequence.md`, `maintenance-sequence.md`, `multi-repo.md` | replaced | The workflow specifications in section 2.9 |
| `steady-state.md`: hotfix flow, deprecation, concurrency rules | merged | `feature` routes; the run lock; `maintain` in report-only mode during a cycle |
| `HEARTBEAT.md` | dropped | It polled an OpenClaw runtime that does not exist here. The briefing and `vsdd status` report health |
| Architecture feasibility loop (three rounds) | ported | `spec` stage feasibility, two rounds |
| Adversarial loops at spec, story, wave, phase 5, F5 (ten passes each) | replaced | `converge()` with a cap of three |
| "Gemini" secondary loops at spec, stories, F2, F3 | merged | The adversary's `review-specs` and `review-stories` methods, and the scenario-reviewer, in the same pass |
| Pull-request review loop (ten rounds) and security review cycles | merged | The story review loop |
| Wait-for-CI loop (three rounds) | ported | Delivery rounds |
| Storybook self-healing loop (ten rounds), per-story UI quality gate | merged | The configured UI check inside the green gate |
| Wave integration fix loop (ten rounds) | replaced | Wave gate rounds with fix stories |
| F7 convergence loop (ten cycles, escalate at five) | replaced | `verify` rounds |
| Task sizing rules, token budget check | replaced | Story size limit at the plan gate |
| Main-checkout sync protocol | dropped | Nothing in 1.0 pulls or merges in the person's checkout. Whether the state directory can collide with product paths is the state designer's question |
| Verification discipline ("run the command yourself") | replaced | The tool measures |
| Seven convergence dimensions | merged | The `verify` checks. Visual and documentation become the ui and docs checks |
| Module criticality guide and mutation thresholds | ported | Configuration read by the baseline gate; criticality also selects review lenses |
| Hidden-scenario thresholds | ported | Section 2.7 |
| Progressive autonomy, maximum-viable-refinement formula | dropped | Never implemented, and the inputs (override rates over twenty runs, dollar cost per pass) do not exist |
| Feedback integration loop (route a flaw back to the phase that owns it) | ported | Finding category selects the fixer; scope `spec` becomes a `spec-gap` decision |

### 3.7 Orchestration skills and tools (function only; the skills themselves are the roster designer's)

| Old item | Mark | Where its job went |
|---|---|---|
| `run-phase`, `next-step`, `validate-workflow`, `bin/lobster-parse` | dropped | The workflow runtime, `next`, and the static tests replace them |
| Fifteen phase entry skills (`phase-0` to `phase-7`, `phase-f1` to `phase-f7`) | dropped | Workflows are the entry points |
| `deliver-story`, `code-delivery`, `fix-pr-delivery`, `pr-create`, `pr-review-triage` | merged | `story` and `vsdd story deliver` |
| `wave-gate`, `wave-scheduling`, `wave-status`, `bin/wave-state` | replaced | `wave`, `vsdd plan schedule`, the survey |
| `adversarial-review`, `phase-1d-adversarial-spec-review`, `holdout-eval` | merged | Review and scenario steps |
| `convergence-check`, `convergence-tracking` | replaced | `converge()`, the tool's verdict function, the `verify` report |
| `formal-verify`, `perf-check` | merged | `verify` checks |
| `quick-dev-routing`, `mode-decision-guide`, `artifact-detection`, `feature-mode-scoping-rules` | replaced | The route rule in `feature` and the route table |
| `maintenance-sweep`, `discovery-engine`, `market-intelligence-assessment`, `release`, `session-review` | ported | `maintain`, `discover`, `release` |
| `bin/check-stale-verdict.sh`, `bin/enforce-merge-strategy.sh`, `bin/resolve-worktree-identity.sh` | merged | `vsdd story deliver` checks that the verdict covers the head and performs the only merge |

## 4. Interfaces I provide

| # | Interface | Consumers | Definition |
|---|---|---|---|
| I1 | Twelve workflow commands | Skills, the driver, people | Names, arguments and end states in section 2.3. Each returns a workflow result (2.10.7) |
| I2 | Stage structure and methods | Workflow authors; the harness | The tables in section 2.9 and the method table in 2.9.13. There is no stage registry file: the scripts hold the structure, the survey returns the stage names recorded as done, and universal test 6 checks every step's method |
| I3 | The route table, the preconditions and the fast-route rule | `vsdd status`, the briefing, the driver, the survey | Sections 2.4 and 2.5, implemented once, as ordered lists in the tool's Python with a fixture per row; the fast-route rule in section 2.9.4, in `vsdd feature route`. `next` is the object of section 2.10.7 everywhere, including `vsdd status --json` |
| I4 | Result schemas | Agents, tool, hook runner | Section 2.10, as files in `<plugin>/schemas/` |
| I5 | The prelude | Packaging (stamping), every workflow | Section 2.11.1, kept in `workflow-lib/prelude.js` and stamped between the markers `// >>> stamped:prelude` and `// <<< stamped:prelude` |
| I6 | The convergence rule | The tool's verdict function, reviewers | Section 2.7 and the `judge()` / `converge()` code |
| I7 | Step contracts | Agents designer | Section 2.9: what each agent type is given and what it returns; severity definitions in 2.7 |
| I8 | Gate catalogue | Tool | Eleven gates: `red` (with `--refreeze`), `green`, `demo`, `spec`, `plan` (with `--story` and admission), `wave`, `release`, `ingest`, `contracts`, `integration`, `sweep`. `wave`, `release`, `ingest` and `sweep` run every due check as one job; the checks `baseline`, `ui`, `formal`, `perf`, `audit` and the rest are selectable with `--only`. Pass conditions in 2.9 and thresholds in 2.7. The state design owns the commands; this design owns the pass conditions |
| I9 | Decision kinds and choices | State and hook designers | Section 2.6 |
| I10 | The spike protocol | Agents, tool, hook runner | Section 2.8; schemas 2.10.4 and 2.10.5 |
| I11 | Limits | Tool (as configuration defaults) | The tables in sections 2.5 and 2.8 |
| I12 | Reserved slash names | Skills designer | The twelve workflow names. `decide` and `drive` are skills in the agents-skills roster |

## 5. Interfaces I need

### 5.1 From the state designer

| # | Need | Detail that must match |
|---|---|---|
| S1 | Project configuration keys | The one key list is the table in `state.md` section 2.12, summarised in blueprint 4.12; an unknown key is an error, so there is no second list here. The keys this design relies on: `commands.{build, test, test_report, lint, format_check, run, ui_check, docs_check, mutation, scan, audit, bench, proofs}`; `branches.{integration, release}`; `delivery.{mode, ci, merge, protected_paths, dependency_changes, ci_wait_minutes}`; `convergence.{passes, rounds}`; `holdout.{mean_min, must_pass_min, stddev_max, rotation, batch}`; `mutation.kill_rate` by criticality; `limits.*` as in 2.5 and 2.8; `stops.{plan, plan_ceiling_agents}`; `evidence.{wave_demo, final_demo}`; `notify.channel`; `maintain.sweeps`; `release.*`; `[[discover.source]]` |
| S2 | `vsdd survey <workflow> [--subject] [--repo] [--cycle] [--request-uri] [--guidance-uri] [--question-uri] --json` | Returns the survey fields in 2.10.8. Applies the preconditions and the three-part "done" test in 2.5. Idempotent. Creates story worktrees at `.worktrees/<story-id>/` and the integration worktree at `.worktrees/_integration/` (per repository in a hub). The worktree library it uses also serves the admission commands |
| S3 | Step record store | Every step result is retrievable by `(run, workflow, subject, name, attempt)` before the next step starts |
| S4 | Evidence store | One record per gate and commit, with `gate`, `subject`, `sha`, `passed`, measurements. Writable only by the tool |
| S5 | Decision store | Records with `id`, `kind`, `status` (`open`, `approved`, `revise`, `rejected`, `stale`), `choice`, `note`, `digest`, and a readable packet. Opened only by the checkpoint |
| S6 | Spike ledger | One per project: `id`, `question`, `asker`, `asker_type` (from a hook-written record), `read_deny` (the classes the runner was denied), `agent` (the runner's agent id), `run`, `ref`, `opened`, `minutes`, `verdict`, `answer`, `method`, `worktree`. Named spikes keep `.spike/STATE.md` and `.spike/TASKS.md` in their worktree. The command forms are in section 2.8 |
| S7 | `vsdd checkpoint <workflow> --run <id> --json` | Does what 2.5 lists and returns the Close fields. It decides from disk and takes no status from the script |
| S8 | Run lock | Scoped to repository and branch; expires; a refused survey returns `ok: false` with the holder |
| S9 | `vsdd status --json` and the session briefing | One function, computed from the records each time; there is no stored status record. They include `next` computed from the route table (I3), open decisions, named spikes, and whether the driver is active or paused so a session continues it after compaction |
| S10 | Story facts | The tool can read from a story: id, wave, dependencies, criticality, kind, security and UI flags, whether evidence recordings are required, acceptance criteria ids, contract ids |
| S11 | Demo evidence location | Somewhere the demo gate can check that is not bulk media on the product branch |

### 5.2 From the agents and skills designer

| # | Need | Detail that must match |
|---|---|---|
| A1 | Agent types | The 20 roster agents of the blueprint, section 4.3. planner: `product-owner`, `architect`, `ux-designer`, `story-writer`, `codebase-analyzer`, `researcher`, `spike-runner`. builder: `test-writer`, `implementer`, `verifier`, `demo-recorder`, `technical-writer`, `devops`. reviewer: `adversary`, `code-reviewer`, `security-reviewer`, `holdout-evaluator`, `ui-reviewer`, `scenario-reviewer`. clerk: `clerk`. Section 4.4 of the blueprint maps each provisional name this document used to its roster agent and method. The `ROLE` table is generated from the roster |
| A2 | Every agent works as a workflow step | It needs no Agent tool, reads its inputs by path, runs the named gate command last, and returns a step result |
| A3 | `clerk` | Tools: `Bash, StructuredOutput`. System prompt of a few lines: run the one command with a Bash timeout of 600000, run it again while its output says `pending`, return its JSON |
| A4 | `spike-runner` | `isolation: worktree` in its file (honoured for a dispatch; a workflow step passes isolation itself). First command `vsdd spike open`, last command `vsdd spike record`, both with file inputs |
| A8 | Planners with a `vsdd`-only shell | `product-owner`, `architect`, `ux-designer` and `story-writer` have Bash limited to single `vsdd` calls in the read and measure groups (and `feature route` for the architect), so they can finish a step with the check for their own work |
| A5 | One shared paragraph in every agent file | "If you need an experiment: with the Agent tool, dispatch `vsdd-factory:spike-runner`; without it, return `needs_spike` with a spike request." Every agent carries it. No model-visible spike skill |
| A6 | Skills | The `brief` skill, which ends by running `vsdd checkpoint brief` (this opens the cycle and decision `brief`); `setup`; `decide`; `drive`, the driver described in 2.4; `status`. No skill reuses a name in I12, and no skill re-implements a workflow |
| A7 | Reviewer discipline | Stable finding keys, the severity definitions in 2.7, an anchor for every severe finding, and a coverage entry for every item they are told to account for |

### 5.3 From the hooks designer

| # | Need | Detail that must match |
|---|---|---|
| H1 | Store step results | When a `StructuredOutput` call carries `step.run`, write the step record, with the calling agent's type, before the call completes. Pass other `StructuredOutput` calls through. Store the full form of a walled finding in the holdout class |
| H2 | Ground findings | Reject a review result whose finding names a missing file, a line range outside the file, or a quote not found there; give the reason so the agent corrects it. The platform probe showed this works. Findings of category `holdout` have no location and are not grounded |
| H3 | Walls by agent type | W1: only `product-owner`, `scenario-reviewer` and `holdout-evaluator` can read hidden scenarios or their evaluations; every other agent type is refused. W2: `holdout-evaluator` cannot read product source, tests, specifications, stories or reviews. W3: no caller, the main session included, can read stored step records, so a reviewer never sees an earlier pass or a builder's record, and scenario scores never reach a builder. W1 also covers walled findings: the full form of a finding that quotes a scenario is stored only in the holdout class. W4: builders other than `test-writer` cannot write tests frozen at the red gate. W5: builders cannot write product source outside `.worktrees/<story-id>/`. W6: `spike-runner` cannot write outside its own worktree, push or merge |
| H4 | Guards | G1: source files in a story worktree are not writable until red evidence exists (kinds `facade`, `docs`, `chore` and `harness` excepted, which are the kinds with no red gate). G2: no agent merges into or pushes to the integration or release branch except through `vsdd story deliver` and `vsdd release`. G3: while the latest `spec` decision is approved, no specification file is writable outside a run that holds an amendment. G4: the evidence store and the decision store are not writable by any agent tool call |
| H5 | Decision capture | On the prompt-expansion event for `/vsdd-factory:decide <id> <choice> [key=value ...] [-- note]`, write the decision record. Block `vsdd decide` when called by any agent |
| H6 | Spike rules | Tests-before-code and the other pipeline gates do not apply inside a spike worktree. After the time box, refuse every tool call by that spike runner except `vsdd spike record`. Refuse a spike runner that dispatches agents. Refuse a spike runner that finishes with no ledger record: block its `StructuredOutput` call when it is a workflow step, and refuse `SubagentStop` when it was dispatched. Record an admission (the dispatcher's agent type) when an agent dispatches the spike runner, and give the runner the asker's read walls |
| H7 | Project resolution by path | Find the governing project by walking up from the file or directory being touched to the state directory, so story worktrees and `repo` targets resolve correctly |
| H8 | `clerk` restriction | A `clerk` may run only `vsdd` commands |

### 5.4 From the packaging designer

| # | Need | Detail that must match |
|---|---|---|
| P1 | The `vsdd` tool on the agents' path | Commands: `survey`, `checkpoint`, `status`, `gate <name>` (the eleven of I8), `feature route`, `story deliver`, `story new-fix`, `plan schedule`, `review prepare`, `env start` and `env stop` (replacing `holdout env`), `spike open`, `spike record`, `spike close`, `idea register`, `report debt`, `discover collect`, `release`, `repos split`, `decide`, with the global option `-C <dir>`. Each prints one JSON object with `ok`; a long command prints `state: "pending"` while its job runs. Git operations that touch shared refs take a repository lock. Owner: the state design (blueprint 4.9); built by packages G1 to G7 |
| P2 | Prelude stamping and static tests | Stamp the block between the markers into every workflow file; fail the build if any copy differs. Tests: `meta` is a literal; no `import`, `require`, `Date.now`, `Math.random` or argument-less `new Date`; `workflow(` appears only in `wave`, `feature`, `verify`, `maintain`, `multi` and only with `story`; `story.js` has none |
| P3 | One schema source | `<plugin>/schemas/*.schema.json`, stamped into the prelude and loaded by the tool and the hook runner |
| P4 | Conformance fixtures | The same input cases run through the JavaScript `judge()` / `converge()` / `holdoutFindings()` and through the tool's implementations, and must agree. The route is not duplicated: only the tool computes it |
| P5 | A script test harness | Runs each workflow against a fake `agent()` in CI, as was done for the skeletons here |
| P6 | Headless recipe | How to run `maintain` and `discover` on a schedule with the Workflow tool allowed |

## 6. Open questions for the user

These questions are carried into section 2 of the blueprint (decisions D5, D8 to D10 and D15 to D21), which
holds the current wording and recommendations.

1. Merging without a person. Recommendation: `delivery.merge = "auto"`, so a story merges when every gate
   passes, with a short protected list (`.github/**`, `.env*`, `CODEOWNERS`, release configuration) that
   always waits for a person. The old default list included lock files and package manifests, which nearly
   every early story touches.
2. Approval of the story plan. Recommendation: no stop. Coverage, cycles and size are enforced by the plan
   gate, and a stop can be switched on per project or triggered by a cost ceiling.
3. Authorising patch releases. Recommendation: keep the `release` stop for every release in 1.0, including
   bug-fix patches. Auto-release of patches can be added once the escape rate is known.
4. Dismissal by an arbiter. Recommendation: allow the architect, as arbiter, to dismiss a high finding that
   the builder disputed, never a critical or security finding, with every dismissal listed in the release
   packet. The alternative is a human decision for every finding still open at the cap.
5. Specification changes found during build. Recommendation: every change to an approved contract needs your
   approval, batched once per wave. The alternative, letting agents apply "clarifications" unapproved, cannot
   be policed by code.
6. Convergence defaults. Recommendation: one clean pass on the final commit per perimeter, cap of three, two
   lenses for critical and high modules. Please confirm you accept one clean pass in place of three.
7. Clerk tier. Steps that only run one command use Sonnet at low effort, which departs from "Sonnet at
   extra-high is the default". Recommendation: accept; these steps make no judgment and there are two or more
   in every run.
8. Phases 4 to 7 as one workflow. STATE.md says each phase is its own workflow. Recommendation: accept
   `verify` as one phase and one workflow, for the reasons in section 2.3.
9. Market assessment for feature requests. Recommendation: off by default; run `discover` in `assess` mode
   when you want it. The old flow ran it, with a stop, for every non-bug feature.
10. Spike defaults. Recommendation: 2 per step, 6 per run, 20 per day, 20 minutes each, and 14 days of
    worktree retention.

## 7. Risks

| # | Risk | Effect | Mitigation |
|---|---|---|---|
| R1 | Settled. The platform probe of 2026-11-23 showed that the hook on `StructuredOutput` receives the whole result and that a block makes the agent correct it and return again | None now. A later platform release could change it | The weekly conformance tier re-checks it. There is no `vsdd step record` command: nine of the twenty agents have no shell, so that fallback could not have worked. If the behaviour ever changes, the hook stores the result at agent stop from the subagent's transcript, which needs no tool in the agent, and grounding moves into the verdict computation |
| R2 | The convergence rule exists twice, in JavaScript and in the tool | They drift, and a script thinks a unit converged when the tool does not | Conformance fixtures (P4). The tool's answer always wins, so drift costs a wasted run, not a wrong merge. The route exists once, in the tool |
| R3 | One clean pass may let more defects through than three | Defects reach the wave or release perimeter, or users | Three perimeters, coverage accounting, two lenses on critical modules. Measure the rate of findings first caught at a later perimeter during the end-to-end proof and raise the cap or add a lens if it is high |
| R4 | The driver depends on the main session following `next`, and "a skill can start a workflow" is unconfirmed | More typing | The briefing prints the exact command, so a person can drive by hand; enforcement does not depend on the driver |
| R5 | An agent could fabricate a gate result or write an evidence file through a shell command | A script takes a wrong branch | Guards read the evidence store, hooks deny agent writes to it, and the tool re-measures at delivery. A determined agent with a shell is the hook designer's residual risk |
| R6 | The 1,000-agent ceiling, 16 concurrent agents and same-session resume | A large wave cannot finish in one run, and a long run dies with its session | Story and agent budgets with `partial` results; the survey resumes from disk in any session |
| R7 | The `vsdd` tool is now the critical component, with about sixteen commands | A bug there is a pipeline bug; a schema mismatch breaks a script | Build and test it first; schemas from one source; the script harness (P5) runs against recorded tool outputs |
| R8 | Parallel stories merging into one branch | A change that passed alone breaks the branch after another merge | Delivery does not rebase because the base moved: it rebases only when the pull request conflicts, and then a clean rebase (same `git patch-id`) keeps the verdict and the green evidence and needs a build check only, while a changed patch gets a scoped review. The merge passes the current head, whose patch id must match the verdict's. The wave gate runs the full suite on the integration head, and breakage found there is fixed forward as stories |
| R9 | The hidden-scenario wall for the evaluator | An evaluator with a shell can read source it should not see | The evaluator's tools are restricted to starting the product and exercising it; the leak check on its text before fix stories are written. Residual for interpreted products |
| R10 | Spike worktrees start from the main checkout's head | The experiment misses the asking story's work | The request carries `ref` and the runner checks it out first. Needs confirming on the platform |
| R11 | Auto-deferral of blocked stories changes the wave plan without a person | Scope appears to slip | Nothing is dropped: the release gate requires every planned story to be delivered or descoped by a person, and deferrals are listed in the wave result |
| R12 | Reviewer cost: Opus at extra-high effort on every pass, plus a fixed cost per agent start | Spend | A realistic story is 8 to 12 agent starts, and a six-story wave 70 to 90, against well over twenty per story before; the lens count follows criticality; clerk steps run at low effort, and the second pass removed about nine of them from a greenfield cycle. The end-to-end proof measures the real figures |
| R13 | Named spikes overlap with the user's own `resume-spike` skill | Two ways to resume | The workflow uses the same branch, worktree and ledger file names, so either can resume the other's work |
