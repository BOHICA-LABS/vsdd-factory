# Second pass: requirements trace

Reviewer: requirements trace (P2REQ). Written 2026-11-23 against `rewrite/STATE.md`, `rewrite/BLUEPRINT.md` and the
five amended designs in `rewrite/design/`. Decisions D1 to D30 are treated as adopted.

## Verdict

Most of what the user asked for is in the design and can be traced to a section. That covers the nine user
decisions, the hook-runner requirements, compaction and cold-start resume, and the spike protocol with its three
entrances and its limits. The greenfield route is complete. Brownfield is not. Every brownfield path, and every
bug fix after a release, goes through `feature`'s fast route. That route cannot get a worktree for the story it has
just written: the child `story` needs a unit from the parent's survey, and the survey ran before the story existed
(P2REQ-01, blocker). Behind that gap there are four more. After an ingest, the design never says what the baseline
or the scope is (P2REQ-02). It has no way to add a test harness or CI to a project that lacks them and then use
their commands (P2REQ-03). An imported old project has no next step (P2REQ-04). A change that spans two
repositories cannot be expressed on the fast route, and the gates of each repository are not designed (P2REQ-05).
Decision D19 has no path for running its amendment inside an open cycle (P2REQ-06). A critical fix that arrives
while a cycle is open is only partly designed (P2REQ-07). Autonomous spikes, and the `drive` loop, still stop on
permission prompts that the adopted allow-list does not cover (P2REQ-08). No end-to-end test exercises any
brownfield case (P2REQ-09). Eight minor gaps follow. Asked plainly, "did we include brownfield?": the intent is
there, as `ingest` then `feature`, or `feature` alone. But as written, none of the five brownfield cases runs to a
release without stalling.

## Requirement table

"Met" means a section of the design satisfies it as written. "Partly" or "not met" have a finding.

### Decisions made by the user (`rewrite/STATE.md`)

| # | Requirement | Result | Where |
|---|---|---|---|
| U1 | Full rewrite as 1.0, no compatibility with the rc line | met | Blueprint 1; packaging 2.3, 2.8 |
| U2 | Port every capability (discovery, design/UX, multi-repo, DTU, observability, maintenance) | met | agents-skills 3.1, 3.2; pipeline 3.1 to 3.7; hooks 3.1; packaging 3 |
| U3 | macOS and Linux only | met | packaging 2.6 (unit on Ubuntu and macOS), 2.7; hooks 2.2 (POSIX `sh`) |
| U4 | Fable for planning and creative work, Opus for every independent reviewer, Sonnet for the rest | partly | Blueprint 4.2, 4.3; agents-skills 2.3; P2REQ-13 |
| U5 | Python hook runner launched through `uv` | met | hooks 2.2; Blueprint 4.1 |
| U6 | Claude Code Workflows, redesigned rather than transcribed | met | pipeline 2.3, 2.12 |
| U7 | Low touch; activation only if genuinely needed | partly | state 2.3 (one `setup` per clone) met; P2REQ-03, P2REQ-08 |
| U8 | Rewrite done by workflows, on a new branch, in a fresh directory | met | Blueprint 9 (`next/`) |
| U9 | Survive compaction, warm a fresh context, resume from cold | partly | Blueprint 7; state 2.9, 2.10; P2REQ-16(c) |

### Requirements the blueprint must meet (`rewrite/STATE.md`)

| # | Requirement | Result | Where |
|---|---|---|---|
| P1 | Nesting flattened to one level | met | pipeline 2.3 nesting rule; packaging P2 static test |
| P2 | Human stops are workflow boundaries; the 37 approvals pruned | met | pipeline 2.6, 3.5 |
| P3 | Convergence: severity threshold, cap, escalation | met | pipeline 2.7 |
| P4 | Walls enforced by hooks keyed on the calling agent | partly | hooks 2.7, 2.11.2; P2REQ-10, P2REQ-14 |
| P5 | Build and test commands come from project configuration | partly | state 2.12 `[commands]`; P2REQ-03 |
| P6 | One per-story playbook with the adversary loop | partly | pipeline 2.9.1, 2.11.2; P2REQ-01 |
| C1 | The main session stays thin | met | Blueprint 7; pipeline 2.5 |
| C2 | All state on disk at every step boundary | partly | pipeline 2.5; hooks 2.11.7; P2REQ-16(c) |
| C3 | Every workflow starts by working out what is done | met | pipeline 2.5; state 2.8 |
| C4 | `SessionStart` briefing on startup, resume and compaction | met | state 2.9; hooks 2.11.8 |
| C5 | Pre-compaction flush | met | hooks 2.11.9; state 2.10 |
| C6 | Size caps on state files | met | state 2.4, 2.5, 2.7 |
| C7 | One status-and-resume entry point | met | state 2.10 |
| S1 | Spikes are first-class | met | Blueprint 6; pipeline 2.8 |
| S2 | Reachable from every level, at any point | partly | Blueprint 6; P2REQ-11, P2REQ-12 |
| S3 | Agents and workflows start spikes on their own judgment, with no human stop | partly | pipeline 2.8; P2REQ-08 |
| S4 | The user's convention (`spike/<name>`, `.spike/`, `.worktrees/spike-<name>/`) | met | pipeline 2.8; state 2.5 |
| S5 | Entrances consistent with the spike results | partly | Blueprint 6 matches the STATE table; P2REQ-12 |
| S6 | The hook runner sees spike starts and enforces limits in code | met | hooks 2.11.6 |
| S7 | One protocol; "in a workflow, ask; elsewhere, dispatch" | met | agents-skills 2.7 shared block, 2.11 |
| S8 | The wrapper is stamped into every workflow and checked by a test | met | packaging 2.5; pipeline P2 |
| S9 | Limits per step and per run; escalation to a person | met | pipeline 2.8; Blueprint 4.11 `spike-limit` |
| S10 | Pipeline gates off inside a spike; nothing reaches the product except through the pipeline | met | hooks 2.11.6 |
| S11 | Durable ledger; retention rule | met | state 2.5, 2.17; pipeline 2.8 |
| H1 | One checked-in `hooks.json`, no variants, no activation | met | hooks 2.3 |
| H2 | Inert in an uninitialised project | met | hooks 2.8; D24 |
| H3 | Standard-library Python, TOML configuration, frontmatter reader | met | Blueprint 4.1; packaging L-PY |
| H4 | Old defects (no stdout, timeout read as seconds) not carried over | met | hooks 2.3 notes, 2.15 |

### Section 2 of the blueprint, adopted as decisions

| # | Decision | Result | Where |
|---|---|---|---|
| D1 | State on its own branch, mounted at `.factory/` | met | state 2.3 |
| D2 | The plugin commits and pushes the state branch; `state.push = false` | met | state 2.3, 2.12 `[state]` |
| D3 | Importer for pre-1.0 projects | partly | state 2.15; P2REQ-04 |
| D4 | Scoped self-approval of routine tool calls | partly | hooks 2.11.10 `auto-allow`; P2REQ-08 |
| D5 | Clerk tier, Opus xhigh, tiers for unnamed roles, aliases, ingestion override, re-proof after a model release | partly | Blueprint 4.2; P2REQ-13 |
| D6 | Main session walled like an agent, with a configuration switch | partly | hooks 2.7 `[main]`; P2REQ-14 |
| D7 | A typed command is the only way to settle a decision | met | pipeline 2.6; hooks 2.11.5; agents-skills 2.12 `decide` |
| D8 | `/vsdd-factory:spike` is a workflow, with no spike skill | met | pipeline 2.9.12; agents-skills 2.11 |
| D9 | One clean pass per perimeter, cap of three, two lenses on critical and high | met | pipeline 2.7, 2.11.2 lenses |
| D10 | One `verify` for the old phases 4 to 7 | met | pipeline 2.9.6 |
| D11 | Old git rules as default gates; attribution setting switched off | met | hooks 2.10 rules 7 and 9; agents-skills 2.12 `setup` |
| D12 | Locked rule set; two person-only escape switches | met | hooks 2.6, 2.2 |
| D13 | Bundle four servers; name both tool spellings | met | agents-skills 2.10 |
| D14 | `vsdd export pdf`, `vsdd render diagram`, Node optional | met | Blueprint 4.9; package T5 |
| D15 | Auto-merge, except protected paths (including release configuration), dependency changes and configured cases | partly | state 2.12 `[delivery]`; P2REQ-15 |
| D16 | No plan stop, except opt-in or an estimate above a ceiling | partly | pipeline 2.6; P2REQ-15 |
| D17 | A person authorises every release | met | pipeline 2.6 |
| D18 | The arbiter may dismiss only disputed findings that are neither critical nor security | met | pipeline 2.7 and amendment 8 |
| D19 | Spec changes found during build: batched once per wave through an amendment run | not met | pipeline 2.6; P2REQ-06 |
| D20 | Market assessment off by default | met | pipeline 2.9.4 |
| D21 | Spike limits 2, 6, 20, 20 min, 3 open, 14 days | met | state 2.12 `[spikes]` |
| D22 | Demo script and transcript; screenshots at most 300 KB; no committed video | met | state 2.5 |
| D23 | Prune closed cycles; switch to keep them | met | state 2.5, 2.12 |
| D24 | Silent when uninitialised; one migration line | met | hooks 2.8 |
| D25 to D30 | Cutover choices (one branch, CI e2e key, migration plan, no self-hosting, names, deletions) | met | packaging 2.3, 2.7, 2.9, 2.11 |

### Walkthroughs the user asked for, and the other checks

| # | Item | Result | Where |
|---|---|---|---|
| W1 | Spike: a person | partly | P2REQ-11, P2REQ-12 |
| W2 | Spike: the main session on its own judgment | met | Blueprint 6; section 10 probe 4 with fallback |
| W3 | Spike: an agent dispatched in a normal session | partly | P2REQ-10 |
| W4 | Spike: a skill | partly | P2REQ-12(b) |
| W5 | Spike: an agent running as a workflow step | met | pipeline 2.8, 2.11.1 `step()` (prompts: P2REQ-08) |
| W6 | Spike: a workflow | met | pipeline 2.8 |
| W7 | Spike: a workflow that is already a child | met | pipeline 2.8; `story` uses `step()` |
| B-a | Brownfield: existing project with tests | partly | P2REQ-01, P2REQ-02, P2REQ-17 |
| B-b | Brownfield: no tests, no CI | not met | P2REQ-03 |
| B-c | Factory-built project after release: second feature, bug fix, third feature | partly | P2REQ-01, P2REQ-06, P2REQ-07, P2REQ-16 |
| B-d | Imported old-plugin project, then a feature | not met | P2REQ-04 |
| B-e | Multi-repository feature spanning two repositories | partly | P2REQ-05 |
| X1 | Model policy, including the clerk tier | partly | P2REQ-13 |
| X2 | macOS and Linux only | met | see U3 |
| X3 | Python through `uv`, standard library only | met | hooks 2.2 (`--offline --no-python-downloads`); packaging L-PY |
| X4 | No activation step beyond one setup per clone | met | state 2.3 (multi-repo clones: P2REQ-05) |
| X5 | Resume after compaction and from cold | met | Blueprint 7 |
| X6 | Every old capability carried forward | met | mappings in all five designs |

## Walkthroughs

### Spikes, entrance by entrance

1. **A person.** They type `/vsdd-factory:spike "<question>"`. The `spike` workflow calls `runSpike()`, a
   `spike-runner` (Fable) starts in a platform worktree, runs `vsdd spike open`, experiments, and runs
   `vsdd spike record`. Nothing stops for the person. On disk afterwards: `.factory/spikes/SP-NNN.md` and a kept
   worktree, listed in the ledger. A named spike (`open`) also gives `spike/<name>`, `.worktrees/spike-<name>/`
   and `.spike/STATE.md` with `TASKS.md`. Two problems. How typed text arrives in `args` was never tested; the
   STATE spike tested this entrance through a skill (P2REQ-12). And the entrance fails before `setup`, while the
   workflow's lock behaviour is undefined (P2REQ-11).
2. **The main session on its own judgment.** It starts the workflow or dispatches `spike-runner` with the Agent
   tool, which its `[main]` row allows. This works, subject to probe 4, which has a fallback.
3. **An agent dispatched in a normal session**, for example the architect under `/vsdd-factory:author`. It has
   SPAWN and dispatches `spike-runner`, and the STATE spike showed this works. The asker's type, though, comes
   from the prompt (`--asker`), not from the hook. `code-reviewer` and `holdout-evaluator` cannot dispatch; they
   return blocked, and the main session runs the spike "on their behalf" without their walls (P2REQ-10).
4. **A skill.** A main-session skill starts the workflow. The two forked skills (`research`, `retro`) run the
   researcher, which lists SPAWN. No probe checks that a forked skill's agent has the Agent tool (P2REQ-12(b)).
5. **An agent as a workflow step.** It returns `needs_spike`. `step()` checks the limits, runs `runSpike()` as a
   step and starts the asking step again with the finding. This matches the spike result that works end to end.
   In the default permission mode the runner's ad-hoc experiment commands raise prompts (P2REQ-08).
6. **A workflow.** It calls `runSpike()` as a step. Met.
7. **A workflow that is already a child.** `story` uses the same `step()`, so the spike runs as a step and never
   as a third workflow. This matches the "works as a step only" result. The run's spike count is shared through
   `vsdd spike open --run`. Met.

### Brownfield cases

- **(a) Existing project with tests.** The person types `/vsdd-factory:setup`. `vsdd init` creates and mounts
  `factory-artifacts`, writes `factory.toml` by detection and needs a JUnit report; the person corrects the file
  in an editor. Then `/vsdd-factory:ingest` runs with no stop and leaves `analysis/codebase/`, recovered
  contracts (`origin: recovered`, draft), the architecture, `modules.toml`, `analysis/codebase/security.md` and
  an ingest closure in `cycles/_standing/`. Next is "human: … run `feature`". Then
  `/vsdd-factory:feature "add X"`: the survey opens a cycle (baseline: ingest), the architect writes the delta,
  the tool picks the route, and the story-writer writes the story. **It stalls here**, because no step creates
  the story's unit (P2REQ-01). On the standard route it gets as far as the `spec` decision. After that, plan
  coverage, the verify scope and the inherited suite, mutation and scanner debt are all undefined (P2REQ-02).
  The project's conventions are not carried over (P2REQ-17).
- **(b) No tests, no CI.** `init` leaves `test` and `test_report` empty, which is an error. On a fast route no
  chore story ever adds a harness. On the standard route the scaffold chore story cannot pass its own green gate.
  Nobody but the person can set the commands, and nothing tells them when. The CI wait has no defined behaviour
  when the repository reports no checks or has no remote. **It stalls at the first red or green gate**
  (P2REQ-03).
- **(c) After the first release.** A second feature on the standard route works: `spec` decision, `plan`,
  `wave`, `verify`, `release` decision, `release`. A bug fix after that release uses the `bugfix` fast route and
  stalls the same way as (a) (P2REQ-01). A third feature that arrives during the second is queued as
  `requests/CR-NNN.md` and starts when the release checkpoint closes the cycle. Three gaps remain. A non-critical
  bug fix during the second feature is also only queued. A critical fix during it opens a second cycle that the
  route, `verify`, `release` and `decide` cannot address (P2REQ-07). A spec gap found in a wave has no amendment
  path (P2REQ-06). Fixes from `maintain` outside a cycle have no release path (P2REQ-16).
- **(d) Old-plugin project.** `setup` finds legacy state, runs `vsdd import --dry-run` and hands the person
  `vsdd import`, which they run through the shim. The tree is rewritten in place and tagged `factory-pre-1.0`.
  **Then there is no next step.** No baseline, cycle or decision records exist, so `feature` refuses. The route
  can point at `ingest` or `spec`, and in-flight stories are orphaned (P2REQ-04).
- **(e) Two repositories.** A hub needs `layout = "hub"` and `[[repo]]` entries that the person writes by hand.
  Each member is ingested with `repo`, then `multi` synthesises. A cross-repository feature is classed
  `trivial` if it changes no contract, which yields one story for two repositories. On the standard route,
  waves, integration worktrees and per-repository release settings are undefined (P2REQ-05).

## Findings

### P2REQ-01 — `feature`'s fast routes cannot obtain a unit for the story they write (blocker)

**Where.** `rewrite/design/pipeline.md` 2.9.4: the `story-draft (fast routes)` and `deliver (fast routes)` rows.
`pipeline.md` 2.9.1: `story` takes `position`, "the unit from the parent's survey", and then skips its own survey.
`pipeline.md` 2.11.2: `let u = a.position`; without it the story runs its own `vsdd survey story`, whose
preconditions include "the story's wave open". `rewrite/design/state.md` 2.11: "Every run that can merge into the
integration branch takes the same scope, so a standalone `story` run and its wave exclude each other."
`rewrite/BLUEPRINT.md` 3.3 steps 1 to 4: "The survey has already made a worktree at `.worktrees/S-9.01/`", but
the story-writer only writes S-9.01 in step 3, after the survey in step 1.

**Problem.** The parent's survey runs before the story file exists, so it cannot return the story's unit
(worktree, branch, base, position, acceptance criteria). Calling `story` without a unit makes it standalone, and
then two things refuse it. The parent `feature` run already holds the lock for the integration branch, and a
fast-route story belongs to no wave. No clerk step between `vsdd gate plan --story` and the child call creates
the unit. `wave`, `verify` and `maintain` do not have this problem: their mid-run stories come from
`vsdd story new-fix`, which returns units.

**What breaks.** Every `trivial`, `bugfix` and `hotfix` route: the first feature after an ingest, every bug fix
after a release, and every critical fix. Re-running `feature` may happen to work, because the next survey sees
the story, but the first run always fails.

**Resolution.** Have `vsdd gate plan --story <id>` return the story unit once it passes, or add a clerk step
`vsdd survey story --subject <id> --run <parent run>`. The tool creates `story/<id>` from the cycle's base (the
release tag for a hotfix) under the parent's lock and records the unit in the run. The feature script passes
that unit as `position`. Add one harness scenario per fast route.

### P2REQ-02 — After an ingest, the baseline and the scope of spec, plan and verify are undefined (major)

**Where.** `pipeline.md` 2.9.6: "scope (`full` for a first release, `delta` afterwards)" and "standard and first
release, all that apply". The baseline row there blocks on "a traceability link is missing; a module's mutation
score is under its criticality threshold; a scan reports critical or high". `pipeline.md` 2.9.5: the plan gate
requires "every contract is covered by a story criterion". `pipeline.md` 2.9.1: the green gate needs the "full
suite" to pass. `pipeline.md` 2.9.8: recovered contracts "become approved contracts only through a later `spec`
decision". `state.md` 2.8: `baseline` is a release tag or an ingest, and the `spec` digest covers "the
specification file set".

**Problem.** Nothing says whether the first cycle after an ingest is a "first release". Nothing says whether a
`spec` decision on a delta approves the whole recovered set, or how measured gates treat debt the factory did not
create: recovered contracts with no story, modules below the mutation thresholds, scanner findings in existing
dependencies, and existing tests that already fail or flake.

**What breaks.** Brownfield case (a) on the standard route. Approving the delta can promote every recovered
contract. The plan gate then demands stories for all of them, and the verify baseline fails traceability for each
one. On any route, one failing inherited test fails every story's green gate. All of it ends in `finding` and
`stuck` decisions about code nobody touched.

**Resolution.** End `ingest` with `vsdd gate baseline`. Its evidence becomes the ingest baseline: the suite
result with the identities of tests already failing, mutation per module where configured, scanner findings and
the recovered contract set. Define "first release" as a cycle with neither a release baseline nor an ingest
baseline; an ingest-based cycle runs in delta scope. On inherited code, measured gates block only on regressions
against the baseline (a ratchet) and on modules the cycle changed. The green gate's "full suite" means no new
failures against the baseline's known failures. A `feature` `spec` decision approves only the delta files;
recovered contracts outside the delta stay `draft` and are excluded from coverage and traceability until a cycle
adopts them. List the inherited debt in `vsdd report debt`.

### P2REQ-03 — No path to add a test harness, a JUnit report or CI and then use their commands (major)

**Where.** `state.md` 2.12: `init` detects commands, and "a missing `test` command or `test_report` is an error";
`[delivery] mode = "pr"`; the file is written by no tool call. `pipeline.md` 2.9.5 readiness creates scaffold and
CI chore stories, but only inside `plan`. `pipeline.md` 2.9.1: the green gate runs the configured build, suite,
lint and format. `pipeline.md` 2.9.4: fast routes have no readiness step. Blueprint section 10, item 23: the
fallback is an `environment` decision. `hooks.md` 2.6: configuration is person-only.

**Problem.** A codebase with no tests (case b), no JUnit output, or no CI cannot move. On a fast route nothing
creates the harness story. On the standard route the chore story that adds the harness cannot pass its own green
gate, because `[commands] test` is empty, and only a person can fill it in, with nothing prompting them when the
story lands. The same chicken-and-egg hits a greenfield empty repository after its scaffold story, which puts the
greenfield proof (E3) at risk. What `story deliver` does is unspecified when the repository has no CI checks (a
30-minute wait, then?) or no remote at all.

**What breaks.** Case (b) stalls at the first red gate (an `environment` decision with no fix the factory can
make) or at the scaffold's green gate (`stuck`). Low touch (U7) fails for the most common brownfield starting
point.

**Resolution.**
1. When `test` or `test_report` is missing, the `feature` survey and `plan` both schedule an `E-0` chore story,
   "test harness and JUnit report", ahead of any other story, fast routes included.
2. A chore story that establishes commands returns the proposed `[commands]` lines. Its green gate runs those
   lines from the story worktree. The checkpoint opens a `config` decision whose packet shows the exact lines,
   and the person's `decide config.commands approve` writes them. Because it is the person's keystroke, the rule
   "no tool call writes `factory.toml`" holds.
3. `init` sets `delivery.mode = "local"` when there is no remote. `delivery.ci = "none"` is detected when the
   repository has no CI. Only then does "no checks reported" count as a pass, and the CI chore story is
   scheduled.

### P2REQ-04 — An imported project has no next step (major)

**Where.** `state.md` 2.15 (the import table and steps 1 to 4). `state.md` 2.3 Mounting and the 2.16 `init` row:
"With pre-1.0 state it runs the importer", which contradicts `import` being person-only (`hooks.md` 2.7
`human_only`). `pipeline.md` 2.9.4: the `feature` survey needs "a baseline: a release or an ingest". `pipeline.md`
2.4 route rows 4 and 6. `packaging.md` 2.9, the eight projects, step 4.

**Problem.** The importer writes specifications and stories but no cycle, decision, closure or baseline records,
and nothing says whether it writes `factory.toml`. Afterwards:
- `feature` refuses, because there is no baseline.
- The route reads "source exists, nothing ingested, no approved brief" and says `ingest`, which would recover
  contracts over 86 to 278 approved ones.
- If the brief's status counts as approved, the route says `spec`, whose survey then needs a `brief` decision
  record that does not exist.
- Stories imported as `ready` have no plan record and no wave, so no workflow can run them.

**What breaks.** Decision D3. The eight projects exist to continue their work. After a successful import, none of
them can take a feature.

**Resolution.** The importer writes:
- `cycles/_standing/units/import.json` as a baseline (the integration head, plus the old project's last release
  tag if it has one), which `feature` accepts as it accepts an ingest;
- `brief` and `spec` decision records marked `settled_by: import`, with digests of the imported set, so the
  route reads steady state;
- one queued change request per open epic, in place of orphaned `ready` stories, or a plan closure record that
  lists them;
- `factory.toml`, by the same detection as `init`.

`init` stops on legacy state and names the import command; it does not run the importer. Add a fixture test:
import a cut-down `vantage`, then run one fast-route feature (see P2REQ-09).

### P2REQ-05 — A feature that spans two repositories is not carried through (major)

**Where.** `pipeline.md` 2.9.4, the route row: the rule for `trivial` ignores `cross_repo`, which the delta data
in 2.10.8 carries. `pipeline.md` 2.9.11, 2.9.2 (one `.worktrees/_integration/`) and 2.9.7. `state.md` 2.14;
`state.md` 2.12 `[[repo]]` (only `branches`, `commands`, `paths`); `state.md` 2.8 (`units/wave.<W-NN>.json`);
`state.md` 2.11 (lock scope "repository and branch"). `agents-skills.md` 2.12 `setup` (`argument-hint: "[--repo]"`,
behaviour undefined). `packaging.md` 2.10 `vsdd repos scan` (reads only).

**Problem.**
1. A cross-repository change with no contract change is classed `trivial`, which yields one story, and a story
   belongs to one repository.
2. Nothing builds a hub for repositories that already exist: `init` does not detect one, and `repos scan` prints
   a graph but writes nothing.
3. Waves are planned for the whole project, but `wave` and its gate run per `repo`. Two runs for the same wave
   in different repositories share one closure record, `wave.W-NN.json`, and one integration worktree path,
   while taking different locks.
4. A `[[repo]]` entry cannot carry `[release]` or `[delivery]` (version files, publish commands, protected
   paths, CI), so the hub release cannot release each repository by its own rules.
5. On a fresh clone of the hub, nothing says who clones the member repositories.

**What breaks.** Case (e) stalls at the route (one story cannot change two repositories), at wave close, or at
release. Point 5 adds a manual activation step for every member, beyond the one `setup` per clone.

**Resolution.**
- `vsdd feature route`: `cross_repo = true` means `standard`, or a fast route that writes one story per
  repository joined by `depends_on`.
- `setup --hub` runs `vsdd repos scan` and shows the proposed `layout = "hub"` and `[[repo]]` block for the
  person to paste.
- Key the wave closure records, gate rounds and integration worktrees by repository
  (`wave.W-NN.<repo>.json`, `.worktrees/_integration-<repo>/`). A wave closes when every repository's part has
  closed.
- Allow `release` and `delivery` tables inside `[[repo]]`.
- `vsdd status --fix` clones any member that is missing, from its `url`.

### P2REQ-06 — Decision D19's amendment run has no path inside an open cycle (major)

**Where.** `pipeline.md` 2.6: the `spec-gap` row ("`amend` (runs `feature` as an amendment …)") and "Specification
gaps found during a wave are batched: … one amendment run covers them all". `pipeline.md` 2.9.4: the survey queues
any non-critical request while a cycle is active. `pipeline.md` 2.4 route table. `state.md` 2.8 choice table.
Blueprint 4.11.

**Problem.** The spec gap is always raised inside an open cycle, and in that state `feature` queues the request
until the cycle closes. `feature` has no argument that marks an in-cycle amendment, and no route row starts one.
Once a person settles `amend`, the blocked story counts as runnable again, because the wave filter only skips
units whose decision is still open. The story then re-runs against the unchanged specification, hits the same gap
and ends `stuck`.

**What breaks.** D19, in every wave of a greenfield cycle and of any feature cycle (case c).

**Resolution.** Add `feature {amend: true}` (or `spec {amend: true}`), which runs inside the open cycle without
queuing. It takes every `spec-gap` decision settled `amend` as its request set and ends at decision `spec`. Add a
route row: "a spec-gap was settled `amend` and nothing else in the wave can run → the amendment run". The survey
keeps a story blocked on that gap until the amended `spec` decision is approved. Add a harness test.

### P2REQ-07 — A critical fix while a cycle is open is only partly designed (major)

**Where.** `pipeline.md` 2.4 (the critical bug row), 2.5 ("a critical bug fix takes a separate lock"), 2.9.4 and
2.9.7 (the merge-back). `state.md` 2.7 (`others`, "a critical fix is the only second line of work"). `state.md`
2.8 (decisions under `cycles/<cycle>/decisions/<id>`). Blueprint 4.8 (decision ids are kinds) and 4.9
(`story deliver` is "the only merge onto the integration branch").

**Problem.** A second cycle is allowed. But `next` is a single value that ignores cycles. `verify`, `release` and
`decide release …` take no cycle argument. The branch a hotfix story merges into (it is cut from the release tag)
is never named. And the merge-back into the integration branch can run while a wave holds that branch's lock.

**What breaks.** Case (c) with a critical fix. The person cannot tell which cycle `/vsdd-factory:verify` or
`decide release approve` acts on, the release checkpoint can close the wrong cycle, and the merge-back races the
wave.

**Resolution.**
- Every `next` for the second cycle carries `args.cycle`, and `verify` and `release` accept `cycle`.
- Decision ids in a second cycle are qualified, for example `release.<cycle>`.
- `story deliver` for a hotfix targets `hotfix/<version>`, cut from the tag.
- The merge-back in `release publish` waits for the integration lock, or opens a `merge` decision.
- Add route rows for runnable work in the second cycle.

### P2REQ-08 — The adopted allow-list leaves autonomous spikes and the driver on permission prompts (major)

**Where.** Blueprint D4. `hooks.md` 2.11.10 `auto-allow`: edits, the configured build and test commands, and five
`git` verbs only; the main session gets read-only `vsdd` subcommands only. `hooks.md` 2.3: `Workflow` never
reaches the runner. `agents-skills.md` 2.12 `drive`, `brief`. `packaging.md` 2.6 smoke probe 7, which covers a
wave and no spike.

**Problem.** An experiment spike runs commands nobody configured. A research spike calls `WebFetch`, `WebSearch`
and the MCP research tools. The `drive` skill starts each workflow with the `Workflow` tool, and the main session
runs `vsdd driver on` and `vsdd checkpoint brief`. None of these is covered, so each raises a prompt in the
default permission mode.

**What breaks.** S3 ("without a human stop") and the purpose of D4. A workflow that starts a spike stalls on the
spike's first ad-hoc command, and an unattended `drive` session waits at every workflow boundary.

**Resolution.** Widen what `setup` offers:
- Bash by `spike-runner` whose working directory and every path argument lie in its own spike worktree, and
  which passes `bash-destructive`, `secrets` and `bash-git`.
- The web and research tools, for the roles that list them, still subject to `secrets`.
- Static allow rules for `Workflow` (scoped to `vsdd-factory:*`) and for the main session's `vsdd driver`,
  `checkpoint brief` and `note`.

Add a smoke probe in which a step returns `needs_spike` for an experiment, and count the prompts.

### P2REQ-09 — The end-to-end proof covers greenfield only (major)

**Where.** Blueprint 9.6: E1 to E3 ("A greenfield run on the sample project from a brief") and CO2 ("E3 passed").
`packaging.md` 2.6, Layer 4: the story tier starts from the `story-ready` fixture, and "Step 4 … is the story tier
plus one greenfield run".

**Problem.** No probe and no scenario test exercises ingest then feature, any fast route, import then feature, a
codebase without tests, or a hub.

**What breaks.** The user asked directly about brownfield, and cutover is gated on a proof that never runs it.
P2REQ-01 to 05 would first surface on the eight real projects after cutover.

**Resolution.** Add an E4 that gates CO2, with these runs:
1. The `tally` fixture with tests and no factory state: `setup`, `ingest`, a trivial `feature`, a bug fix,
   `release`.
2. The same fixture with its tests and CI removed.
3. An import fixture cut from `vantage` or `opnrynkr`: import, then one feature.
4. A two-repository hub with one cross-repository feature.

Add harness scenarios for each fast route and for the amendment run.

### P2REQ-10 — Outside workflows, the asker of a spike reports its own identity (minor)

**Where.** `agents-skills.md` 2.11, last paragraphs ("the main session may run the spike on their behalf").
`hooks.md` 2.11.6, `read-walls` (applies `asker_type` from the ledger). `state.md` 2.5 (the ledger records
`asker_type`). `pipeline.md` 2.11.1 `runSpike()` (`--asker ${asker}`).

**Problem.** Inside a workflow, the wrapper fills in the asker. On direct dispatch, the asker's type reaches
`vsdd spike open` only through the prompt. When `code-reviewer` or `holdout-evaluator` returns blocked in a normal
session (for example under the `review` skill), the main session runs the spike and is recorded as the asker. The
runner may then read specifications or source and hand them back to the walled agent.

**What breaks.** Requirement P4 (walls are enforced by hooks, not by instructions) holds only at the workflow
entrance.

**Resolution.** On `PreToolUse` for `Agent` with a `spike-runner` target, the hook records an admission with the
caller's type. `vsdd spike open` takes `asker_type` from that admission and refuses a mismatched `--asker`. For a
spike run on behalf of a blocked agent, the `review` skill passes the blocked agent's type, and the tool accepts it
only from the main session.

### P2REQ-11 — Spikes are not available "at any point" (minor)

**Where.** Blueprint 4.9: outside an initialised project every command exits 2, except a short list that leaves
out `spike` and `survey`. `pipeline.md` 2.5 concurrency: only `discover` and report-only `maintain` take no lock.
`pipeline.md` 2.9.12 shows no survey or checkpoint for `spike`.

**Problem.** A person cannot run a spike before `setup`; the user's own `resume-spike` convention works in any
repository. And it is undefined whether the `spike` workflow takes the integration-branch lock. If it does, no
spike can start while a wave runs, in this session or another.

**What breaks.** S2, "selectable anytime".

**Resolution.** State that the `spike` survey takes no run lock, or one scoped to `spike/<name>`. Choose and
document the behaviour before setup: either run with a `.spike/`-only ledger (no `.factory/`, as the user's
convention does), or answer "run `/vsdd-factory:setup` first".

### P2REQ-12 — Two spike entrances rest on platform behaviour nobody has tested (minor)

**Where.** Blueprint section 10 (neither item is listed). `pipeline.md` 2.9.12: arguments as objects
(`{open: name, question}`). `rewrite/STATE.md`: the spike table tested the person's entrance through a skill.
`agents-skills.md` 2.12: `research` and `retro` are `context: fork`.

**Problem.** (a) It is unknown whether the text after `/vsdd-factory:spike` (or `/vsdd-factory:feature "<request>"`)
reaches `args` as a string or as an object. (b) It is unknown whether the agent of a forked skill holds the Agent
tool and can dispatch `spike-runner`.

**What breaks.** A wrong guess about (a) breaks the person's entrance to named spikes, and `feature`'s required
`request`. A wrong guess about (b) breaks the forked-skill entrance.

**Resolution.** Add both to package B8. Specify a string grammar that the scripts parse (`<question>`,
`open <name> -- <mission>`, `close <name>`; `feature <request> [intent=…]`), and add a harness test.

### P2REQ-13 — Parts of the adopted model policy did not reach the design (minor)

**Where.** Blueprint D5 and 4.2. `pipeline.md` 2.9.8 (`map` and `analyze` on the planner tier), 2.11.1 (`step()`
has no tier option), and 2.11.2 and 2.11.3 (`meta.phases` with `model: 'opus'` on Review and Gate).
`agents-skills.md` 2.12 `brief`; N-PIPE-2 ("an override marked as one in the script"). `packaging.md` 2.6.

**Problem.**
- (a) D5's marked builder-tier override for ingestion's broad inventory passes does not exist anywhere: not in
  a stage, not in `roster.toml`, not in `step()`.
- (b) The product brief, a planning and creative artifact, is written by the main session on whatever model the
  person runs. A skill's `model` key lasts one turn only.
- (c) Phase-level `model: 'opus'` covers phases in which clerk and builder steps run, and nothing says whether it
  outranks the step's own model. Section 10 does not list it.
- (d) D5's "re-run the end-to-end proof after any model release" has no trigger.

**What breaks.** U4 and D5, in cost (a) and in tier (b, c).

**Resolution.**
- (a) Add an override row in `roster.toml` (`ingest-broad = "builder"`) and a `tier` option on `step()`
  that lint allows only on those steps.
- (b) Have `brief` end by dispatching `product-owner` to write the final `brief.md` from the conversation.
- (c) Drop `model` from phase metadata.
- (d) Have the weekly conformance job record the model IDs behind each alias, and run the story tier when they
  change.

### P2REQ-14 — Decision D6 is only partly implemented (minor)

**Where.** Blueprint D6 ("with a switch in project configuration") and D12 (walls locked). `hooks.md` 2.6 (locked
rules) and 2.7 (`[main]`: `dispatch = ["*"]`). `agents-skills.md` 2.9 dispatch table.

**Problem.** No configuration key exists for the switch D6 promises. And the main session can dispatch
`product-owner`, `scenario-reviewer` or `holdout-evaluator` and receive scenario text in the reply. That is the
path D6 says it closes.

**Resolution.** Either add a person-only `[hooks] main_session_wall` that is announced in the briefing, or record
that D12 removes the switch. When the main session dispatches an agent that can read hidden scenarios, run the
evaluator's no-scenario-text check on that agent's final message at `SubagentStop`.

### P2REQ-15 — D15 and D16 defaults are incomplete (minor)

**Where.** Blueprint D15 and D16. `state.md` 2.12: `[delivery] protected_paths` has no release configuration, and
the owner table lists only `stops.plan`. `pipeline.md` 2.6: the `plan` row, "the configured ceiling".

**Problem.** D15 lists release configuration among protected paths, but the default list leaves it out. D16's
estimate ceiling has no key and no default.

**Resolution.** Add the files `[release]` names (version files, changelog, release workflow) to the default
protected paths. Define the ceiling key (for example `stops.plan_ceiling_agents`) with a default, and list it in
Blueprint 4.12 and in the unknown-key test.

### P2REQ-16 — The route table ignores which cycle is open, ignores brownfield, and loses a draft brief (minor)

**Where.** `pipeline.md` 2.4 (rows 5, 11 and 14), 2.9.10 (`maintain` delivers fixes outside a cycle) and 2.9.7
(the release close returns `feature` when a request is queued). `state.md` 2.7. `agents-skills.md` 2.12 `brief`.

**Problem.**
- (a) In an ingested project with no brief, the row "no approved brief and no change request in flight → human"
  sits above "a change request is queued → `feature`". The briefing therefore never names the queued request,
  although the checkpoint returned it.
- (b) A fix that `maintain` merges outside a cycle moves the integration head, so `next` becomes `verify` with no
  cycle to hold a `release` decision. How maintenance fixes get released is undefined.
- (c) The `brief` skill does not save its draft as it goes, and a draft brief is not a route state. A compaction
  or crash mid-conversation loses it (C2, U9).

**Resolution.**
- (a) Move the queued-request row above the no-brief row.
- (b) Scope the "verify not converged" rows to an open cycle. Have `maintain`, when it delivers fixes, open a
  maintenance cycle (baseline check plus a `release` decision).
- (c) The `brief` skill writes `brief.md` as a draft after each section, and a route row "brief in draft →
  `/vsdd-factory:brief`" continues it.

### P2REQ-17 — An ingested project's conventions never reach the builders (minor)

**Where.** `pipeline.md` 2.9.8 synthesize: the outputs include no `conventions.md`, `review-rules.md` or
`dependencies.toml`. `agents-skills.md` amendment 10 and 2.12 `setup` (offers to move conventions). `hooks.md` 2.7
`[main]` (cannot write `spec-arch`). `state.md` 2.5 (`conventions.md` is written by the architect).

**Problem.** Every agent omits `CLAUDE.md`, but ingest writes no conventions file. The `setup` offer to move
conventions has no writer that may do it, since the main session cannot write `spec-arch`. With no
`dependencies.toml`, the "dependency beyond the approved list" check has no baseline.

**What breaks.** In case (a), builders work without the project's own conventions, and every dependency change is
measured against an empty list.

**Resolution.** Add `conventions.md`, `review-rules.md` and `dependencies.toml` (the current dependencies as the
approved baseline) to the architect's outputs at the ingest synthesize stage. Have `setup` dispatch the architect
to move conventions out of `CLAUDE.md`.

## Checked and found sound

- The spike protocol: open, experiment, record and return. The limits are enforced in the wrapper, in
  `vsdd spike open` and in the hook, and the per-run count is shared by parent and child through `--run`. Spikes
  are kept out of the product (`bash-git`, `write-scopes`); the ledger dedupes by a question hash; retention is
  14 days; named spikes follow the user's convention.
- The workflow-step, workflow and child-workflow spike entrances match what the platform spike proved: no Agent
  tool in a step, one level of nesting, spikes always run as steps.
- One level of nesting, enforced by a static test. Human stops sit at workflow boundaries. Three routine
  decisions. Decisions are captured from the person's typed command.
- The convergence rule (severity, anchoring, reach, grounding, a cap of three, arbitration that dismisses only
  disputed findings, then a person) at every perimeter, with two lenses on critical and high modules.
- The tiers have one source (`roster.toml`), every step sets model and effort, there is a clerk tier, and every
  reviewer runs on Opus at extra-high effort.
- The hook runner: one generated `hooks.json`, a self-guarding command, inert and silent without
  `factory.toml`, a fail-closed launcher, timeouts in seconds, `uv --offline --no-python-downloads`, and a
  standard-library-only lint.
- Compaction and resume: step records written by the hook, a survey done-test computed from disk, a briefing on
  every `SessionStart` source, a pre-compaction flush and snapshot, a step-aware summariser instruction, and one
  `status` entry point that replaces six old skills.
- The state branch with code-owned mount, flush and push (D1, D2); size caps on every artifact kind; nothing
  derived is stored.
- A factory-built project after its first release: a second standard-route feature, and a third feature queued
  as a change request during it.
- Capability mapping: every old agent, skill, hook entry and `bin/` tool is ported, merged, replaced or dropped
  with a reason, and none of the dropped items is a capability the user named.
- macOS and Linux only. Cutover decisions D25 to D30.

Checked: 85 items (9 user decisions, 28 requirements, 30 blueprint decisions, 7 spike entrances, 5 brownfield
cases, 6 cross-cutting checks). Sound: 59.
