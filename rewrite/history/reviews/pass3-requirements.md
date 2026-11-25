# Review pass 3: the user's decisions and requirements

Date: 2026-11-24. Branch `rewrite/v1`. Reviewer lens: the user's decisions and requirements
(`rewrite/STATE.md`, "Decisions made by the user" and "Requirements the blueprint must meet") and the 30
adopted decisions in section 2 of the blueprint. Read: `rewrite/STATE.md`, `rewrite/BLUEPRINT.md`, the five
detail designs, `pass2-closeout-3.md`, and the old-system surveys where a brownfield or import claim depended
on them. Line numbers are as of this reading. R1 to R5 of `pass2-closeout-3.md` are not reported again.

## Verdict

Most of what the user asked for is in the design and is built where section 2.4 of the blueprint says: the
model policy, the hook runner, the walls, the convergence rule, the three routine stops, the spike protocol
and its entrances, compaction and warm start. The weak point is the user's direct brownfield question.
Walking the five starting points of blueprint 3.4 end to end, none reaches a release reliably as the design is
written. The codebase the factory did not build can record a baseline before its test commands are known, so
inherited failures later fail every gate. The queued-request hand-off that the second-feature case and the
import case both depend on is not defined. The no-tests, no-CI case stalls: its fix story's branch is cut
before the test harness exists, and it schedules a CI story again in every later cycle. The two-repository
case cannot ingest its second repository. Resume from cold is partly met. Three paths do not continue in a
new session: the survey runs long measurements with no job, a resumed `feature`, `verify` or `maintain` run
loses the stories an earlier run admitted, and a resumed standard-route `feature` holds no amendment. There
are two spike problems. The spike ledger is an unwalled channel out of the holdout wall. The person's own
quick spike, as the skeleton is written, opens with the wrong command form. One problem is on the slice path:
local-mode delivery has nowhere to merge without touching the person's checkout, which the design promises it
never does. I found 10 major and 4 minor problems and no blocker. Each needs a stated rule in a few places,
not a redesign, except P3REQ-05 (hub ingest), which needs per-repository baseline records.

## Requirements table

"Met" means the design satisfies it as written. "Partly" and "not met" rows have findings.

| # | Requirement or decision | Where the design meets it | Judgement |
|---|---|---|---|
| U1 | Full rewrite as 1.0; old corpus is reference only | Blueprint 1, 9; `next/` staging | Met |
| U2 | Port all of it (discovery, design/UX, multi-repo, DTU, observability, sweeps) | `discover`, `ux-designer` and `preview`, `multi`, `facade` stories and clone fidelity, `vsdd obs`, `maintain` | Met |
| U3 | macOS and Linux only | POSIX `sh` launcher (`hooks.md` 2.2), probes 29 on both, no Windows paths | Met |
| U4 | Model policy: Sonnet xhigh default, Fable xhigh planning and creative, Opus for every independent reviewer | Blueprint 4.2, 4.3; roster with `[overrides]` (D5); every step sets model and effort | Met |
| U5 | Python hook runner through `uv`, standard library only | `hooks.md` 2.2 (`--offline --no-python-downloads --no-config`, 3.11 floor); blueprint 4.1 | Met |
| U6 | Claude Code Workflows, redesigned | 12 workflows, one level of nesting, route computed by the tool | Met |
| U7 | Low touch | Driver, D4, three routine stops; but see P3REQ-01, 03, 10 | Partly |
| U8 | Rewrite done by multi-agent workflows on a new branch, in a fresh directory | `rewrite/workflows/`, `next/`, branch `rewrite/v1` | Met |
| U9 | Survive compaction, warm a fresh context, resume from cold | Blueprint 7; but see P3REQ-06, 07, 08, 13 | Partly |
| R1 | Flatten nesting to one level; a wave calls story delivery | `pipeline.md` 2.3 nesting rule and its static test | Met |
| R2 | Human stops are workflow boundaries; the 37 approvals pruned | `pipeline.md` 2.6, 3.5 | Met |
| R3 | Convergence rule with threshold, cap and escalation | `pipeline.md` 2.7; D9 | Met |
| R4 | Information walls enforced by hooks keyed on the caller | `read-walls`, `write-scopes`, D6; but see P3REQ-09 | Partly |
| R5 | Build and test commands from project configuration | `[commands]`; no workflow names a toolchain | Met |
| R6 | One per-story playbook with the adversary loop in it | `story` workflow, `pipeline.md` 2.9.1, 2.11.2 | Met |
| R7 | The main session stays thin | Blueprint 3.2, 7 | Met |
| R8 | All state on disk at each step boundary | `step-results` hook; survey and checkpoint; `pipeline.md` 2.5 table | Met |
| R9 | Every workflow works out what is done, so re-running continues | Done test in `pipeline.md` 2.5 and `state.md` 2.8; but see P3REQ-06, 07, 08 | Partly |
| R10 | `SessionStart` briefing on startup, resume and compaction | `state.md` 2.9, `hooks.md` 2.11.8; but see P3REQ-13 | Partly |
| R11 | Pre-compaction flush actually wired | `compaction` rule on `PreCompact`; `hooks.json` generated from the rule table | Met (probe 16 still open) |
| R12 | State files have size caps | `state.md` 2.4, 2.5, 2.8 caps checked before the write | Met |
| R13 | One status-and-resume entry point | `/vsdd-factory:status`, `state.md` 2.10 | Met |
| R14 | A spike is a first-class capability | Blueprint 6; `spike-runner`; `spike` workflow; ledger | Met |
| R15 | Spikes reachable from every level, at any point | Six entrances in blueprint 6; but see P3REQ-11. The need for an initialised project is stated in 11.3 and not reported | Partly |
| R16 | Agents and workflows start spikes on their own judgment, with no human stop | `needs_spike` and the wrapper; dispatch with the Agent tool. Without prompts this needs the `extended` scope, which is the open D4 scope question in `rewrite/STATE.md`, not a defect | Met |
| R17 | The user's `spike/<name>`, `.spike/`, `.worktrees/spike-<name>/` convention is the starting point | Named spikes, `pipeline.md` 2.8 | Met |
| R18 | One protocol with three entrances matched to what the platform allows | Blueprint 6 table against STATE probe table; isolation passed by steps; admissions for dispatches | Met |
| R19 | Every step result can carry a spike request, served by one stamped wrapper | `step-result.schema.json`, `step()` and `runSpike()` in the prelude | Met |
| R20 | Limits per step and per run; beyond them a person decides | Wrapper and `spike open`; `blocked` with reason `spike_limit` | Met |
| R21 | Pipeline gates do not block inside a spike; nothing from a spike reaches the product except through the pipeline | `spike_exempt` rules, `bash-git` spike rules | Met |
| R22 | Durable ledger that survives a cold start and is not repeated; a retention rule; full isolation | `state.md` 2.16 spike commands, cached findings; but see P3REQ-09, 12 | Partly |
| R23 | One checked-in `hooks.json`, no per-platform variants, no activation | `hooks.md` 2.3 | Met |
| R24 | Hooks do nothing in a project not initialised | `hooks.md` 2.8; D24 | Met |
| R25 | Standard-library Python; TOML config; a small frontmatter reader in the plugin | `py/vsdd/frontmatter.py`, `tomllib` (3.11 floor) | Met |
| R26 | Old defects not carried over (no stdout, timeout read as seconds) | `hooks.md` 2.3 notes, 2.15 | Met |
| R27 | Setup once per clone and nothing more | `state.md` 2.3 "run setup once per clone"; but see P3REQ-13, and P3REQ-10 for local mode | Partly |
| B1 | Brownfield: a codebase the factory did not build | `ingest`, baseline, decision `spec`; but see P3REQ-04 | Partly |
| B2 | Brownfield: a second feature, a bug fix and a queued third request after a first release | Route rows, queuing, delta scope; but see P3REQ-01, 07, 08 | Partly |
| B3 | Brownfield: a project imported from the old plugin | `vsdd import`, `state.md` 2.15; but see P3REQ-01, 06, 14 | Partly |
| B4 | Brownfield: a codebase with no tests and no CI | Readiness chores; `delivery.ci`; see P3REQ-02, 03, 04 | Not met as written |
| B5 | Brownfield: a change across two repositories | Hub model; one story per repository; see P3REQ-05 | Not met for existing repositories |
| D1, D2 | State on its own branch, committed and pushed by code | `state.md` 2.3 flush and push | Met |
| D3 | Importer for the eight old projects | `vsdd import`; see P3REQ-01, 06, 14 | Partly |
| D4 | Plugin approves its own routine calls, scoped | `auto-allow`, `hooks.md` 2.11.10 | Met |
| D5 | Clerk tier, Opus xhigh reviewers, unnamed roles' tiers, aliases | Blueprint 4.2, roster | Met |
| D6 | Main session walled | `main` row; `main_session_wall` | Met |
| D7 | Typed decision command only | `decide` skill and `decisions` rule; probe 2 proven | Met |
| D8 | `/vsdd-factory:spike` is a workflow; no spike skill | `pipeline.md` 2.9.12 | Met |
| D9 | One clean pass per perimeter, cap three, two lenses on critical and high | `converge()`; `story.js` lenses | Met |
| D10 | One `verify` | `pipeline.md` 2.9.6 | Met |
| D11 | Old git rules as default gates; attribution off | `bash-git`, `merge-guard`; setup offers the attribution setting and `doctor` checks it | Met |
| D12 | Locked rules and two human-only switches | `hooks.md` 2.6, 2.2 | Met |
| D13 | Four bundled servers, both spellings | `agents-skills.md` 2.5, 2.10 | Met |
| D14 | `export pdf`, `render diagram`, Node optional | Package T5 | Met |
| D15 | Auto merge, three things wait for a person | `story deliver`, protected paths, dependency detection; but see P3REQ-10 for local mode | Partly |
| D16 | No plan stop by default; ceiling | `stops.plan`, `stops.plan_ceiling_agents` | Met |
| D17 | A person authorises every release | `release` decision on every route | Met |
| D18 | Architect may dismiss a disputed high finding at the cap | `converge()` arbitration | Met |
| D19 | Spec changes approved, batched per wave | Amendment run inside the open cycle | Met |
| D20 | Market assessment off by default | `discover assess` on demand | Met |
| D21 | Spike limits including 14 days of worktree retention | `[spikes]`; see P3REQ-12 | Partly |
| D22 | Demo script and transcript; screenshots at most 300 KB; media local | `state.md` 2.5 demo row | Met |
| D23 | Closed cycles prune evaluations and demos | `state.prune_closed_cycles` | Met |
| D24 | Hooks silent in uninitialised projects | `hooks.md` 2.8 | Met |
| D25 to D30 | Cutover choices | `packaging.md` cutover steps X0 to X8, package CO2 (not walked in depth) | Met |

71 rows examined, 55 met.

## The two walks

### Research spikes, entrance by entrance (blueprint section 6)

| Entrance | What the person or agent does | What runs | Where it stops | On disk after | Platform check |
|---|---|---|---|---|---|
| A person | Types `/vsdd-factory:spike <question>` | `spike` workflow: survey stores the question, one `runSpike()` step in a platform worktree | Never, unless refused | Ledger entry in `.factory/spikes/`; kept worktree if it changed | Slash command proven; text reaching `args` is probe 31 with a fallback. As written, `runSpike()` opens with `--step`, which this entrance has no record for (P3REQ-11) |
| A person, named spike | `spike open <name> -- <mission>`; later `spike close <name>` | One runner step per increment; branch `spike/<name>`, `.spike/STATE.md`, `.spike/TASKS.md` | Never | Named worktree and ledger files; project ledger entry | Sound, except that the age-based retention rule can remove an open named spike's worktree (P3REQ-12) |
| The main session on its own judgment | Dispatches `spike-runner` with the Agent tool | Admission recorded at `PreToolUse` on `Agent`; runner in its own worktree | Prompts under `core` in default mode; none under `extended` | Ledger entry | Proven: the model dispatched the spike agent unprompted (probe 4); isolation honoured for a dispatch (item 11) |
| An agent dispatched in a session | Dispatches `spike-runner` itself | As above; asker type from the admission | As above | Ledger entry | Proven chain main → agent → spike agent (STATE table); pairing under concurrency is probe 35, with the stricter union as fallback |
| A skill | Instructs the main session, or a forked agent dispatches | As above | As above | Ledger entry | Skill starting a workflow proven; a forked agent holding the Agent tool is probe 32, with a fallback |
| An agent in a workflow step | Returns `needs_spike` | `step()` wrapper runs the runner as a step with isolation and restarts the asker | At the limit, `blocked` with `spike_limit` | Ledger entry; finding in the retried prompt | Proven end to end; step has no Agent tool (item 8) |
| A workflow | Calls `runSpike()` as a step | Same | Same | Same | Works in a parent and in a child (STATE table) |

The entrances match what the platform allows. The two defects are in what a spike carries out. A spike asked
by an agent that may read hidden scenarios can write scenario text into a ledger that builders read
(P3REQ-09). And the person's quick spike opens with the wrong command form (P3REQ-11).

### Brownfield, the five starting points (blueprint 3.4)

1. **A codebase the factory did not build.** `setup` → `ingest` → `spec` decision on the baseline →
   steady state → `feature "<request>"` → standard or fast route → `verify` (delta) → `release`. It stalls
   when `vsdd init` did not detect the test command, its JUnit report or the lint command. `vsdd gate ingest`
   then measures the baseline with no commands. The architect recovers the commands only afterwards, and
   nothing measures the baseline again, so every inherited failure counts as new (P3REQ-04). With commands
   detected, it reaches a release.
2. **A second feature, a bug fix and a queued third request after a release.** The second feature opens a
   cycle. The bug fix and the third request typed while it is open are queued. When the cycle closes, the
   route says "`feature` with the oldest queued request", but nothing defines how the request is handed to
   `feature`, so the queued requests never start cleanly (P3REQ-01). A run of either that dies after
   admitting its story, or after its delta step, does not continue in a new session (P3REQ-07, P3REQ-08).
3. **A project imported from the old plugin.** The person runs `setup`, then `vsdd import` in a terminal.
   The baseline, the imported `spec` decision and queued requests are written, and the route offers
   `feature` with a queued request. It meets the same hand-off gap (P3REQ-01). The first survey must then
   take the whole baseline (suite, lint, format, mutation per module, scanners) inside one clerk call with no
   job (P3REQ-06). An in-flight story re-planned under its old identifier can collide with its pre-1.0
   worktree at the same path (P3REQ-14).
4. **A codebase with no tests and no CI.** `setup` → `ingest` (baseline "no suite") → approval → `feature`
   bug fix → readiness lists the harness and CI chores. The story-writer writes and admits all three
   stories at once, so the fix story's branch is cut before the harness exists. Its red gate cannot produce a
   JUnit report, and it ends `stuck` (P3REQ-02). After the CI chore merges, `delivery.ci` stays `none`. Every
   later cycle schedules another CI story, each needing a merge approval, and delivery never waits for the CI
   the factory added (P3REQ-03).
5. **A change across two repositories.** `setup --hub` writes the member list. Both members are existing
   code, so each needs an ingest. The second ingest collides with the first: one `ingest.json`, one `spec`
   decision identifier, a done test that sees the first as done, and a specification lock that refuses its
   writes once the first is approved. The route does not name it either (P3REQ-05). Until both have
   baselines, the cross-repository `feature` cannot be judged against the second repository.

---

## Findings

### P3REQ-01. The hand-off of a queued change request to `feature` is not defined

Severity: major. On the slice path: no.

Where:
- `pipeline.md` 2.3, `feature` row: `request` is "required on first run, unless `amend`".
- `pipeline.md` 2.4, route row "No cycle is open and a change request is queued → `feature` with the oldest
  queued request" (line 289); 2.9.7 close row, "`next` is done, or `feature` when a change request is queued"
  (line 1065).
- `pipeline.md` 2.9.4 survey row (line 981): "Every request is recorded once... If another cycle is active
  and the request is not critical, it stays queued and the run returns."
- `state.md` 2.5, change request row (line 599): "When the cycle closes the route starts `feature` with the
  oldest"; 2.7, `next` field (line 705), whose `args` are not given for this row.
- `state.md` 2.16, survey row (line 1379): "For `feature` with a request it writes the request file and opens
  a cycle, or leaves the request queued when a cycle is active."
- Blueprint 3.4 row 2 ("A request typed while a cycle is open is queued and starts when that cycle closes");
  4.9 survey row; `packaging.md` 2.6 fixtures (lines 516, 519), which assert only the route's answer.

The problem. The route names `feature` for a queued request, but nothing says what `next.args` carries, and
the `feature` survey has only one way to receive a request: free text in `--request-uri`, which always writes
a new request file. All three readings fail:
- `args.request` holds the request's text. The survey writes a second request file and opens a cycle for
  it. The original `CR-<NNN>` stays `queued`, so after this cycle closes the route offers it again, and the
  work is done twice.
- `args.request` holds `CR-<NNN>`. The survey stores the text "CR-002" as a new request, and the architect
  analyses that string.
- `args` is empty. `request` is required on a first run, so the workflow has nothing to work on.

A second gap: the survey takes the run lock "once its preconditions pass" (`state.md` 2.11). If a wave or
fast-route run holds the lock when the person types the next request, the order between "write the request
file" and "take the lock" decides whether the typed request is recorded at all. The design does not state
it, and queuing must not depend on a lock it does not need.

What breaks. Starting points 2 and 3 of blueprint 3.4. The queued bug fix and the third request after a
release, and every request the importer queued for an epic in flight, either run twice or never run as
themselves. The driver repeats the same `next`.

Proposed resolution:
- State that the route's `next` for a queued request is `{kind: "workflow", command: "/vsdd-factory:feature",
  args: {request: "CR-<NNN>"}}`.
- State that a `request` argument that is exactly the identifier of a queued request is passed as `--subject
  CR-<NNN>`, and the survey activates that file instead of writing a new one. `feature` with no `request`
  and no `amend` takes the request in flight, or else the oldest queued one.
- State that the survey writes a new request file before it checks the lock. When a cycle is active it
  returns `queued` without taking the lock.
- Apply this in `pipeline.md` 2.3, 2.4 and 2.9.4, `state.md` 2.5, 2.7 and 2.16, and blueprint 4.9. Add the
  fixture "a request typed while a run holds the lock is queued, and after the release the same `CR` is
  activated, not duplicated".

### P3REQ-02. On a fast route, stories are branched before the readiness chores they need are delivered

Severity: major. On the slice path: no.

Where:
- `pipeline.md` 2.9.4, story-draft row (line 983): the story-writer writes "first the readiness chore stories
  the route listed" and the change's own stories in the same step, and `vsdd gate plan --story` admits each.
- `pipeline.md` 2.9.1, "Where a child's unit comes from" (lines 813 to 822), and `state.md` 2.16, `gate plan`
  row (line 1430): admission "creates `story/<id>` from the cycle's base" at once. "The cycle's base" is not
  defined anywhere for a normal cycle. `cycle.json` (`state.md` 2.8) has no base field.
- `pipeline.md` 2.9.4, deliver row (line 984): "readiness stories first".
- `pipeline.md` 2.9.5, readiness (lines 1009 to 1016): only "the scaffold story" is put "alone in the first
  wave". `state.md` 2.12 (lines 1241 to 1245) says the test harness chore is "scheduled first on every route,
  fast routes included".
- Blueprint 3.4 row 4 and 4.6 (lines 406 to 416).

The problem. On a codebase with no suite, the bug-fix route admits the harness chore, the CI chore and the fix
story in one step. Each gets its branch and worktree from the same commit, before the harness exists. Running
the readiness stories first does not help: the fix story's worktree was cut earlier and is never refreshed,
because delivery "rebases only when the pull request conflicts". The fix story's test-writer works in a tree
with no harness. Its red gate runs the approved `commands.test`, which needs the harness, and gets no JUnit
report. Two attempts give `blocked` (kind `gate`), and the second run gives `stuck`. On the standard route,
`plan schedule` places only the scaffold alone in the first wave. A harness chore can share wave 1 with
stories whose red gate needs it.

What breaks. Starting point 4 ("a codebase with no tests") does not reach a release on any fast route, and
may not on the standard route. That is the case blueprint 3.4 promises "the same routes" for.

Proposed resolution:
- Define "the cycle's base" as the integration head at the moment of admission (the release tag on
  `hotfix/<version>`).
- When the route lists readiness stories, the fast route runs story-draft and deliver twice: first the
  readiness chores alone, then the change's own stories, so their branches are cut from a head that has the
  harness.
- `plan schedule` puts the test-harness chore in a first wave of its own, as it does the scaffold.
- Apply this in `pipeline.md` 2.9.1, 2.9.4 and 2.9.5, `state.md` 2.12 and 2.16, and blueprint 4.6. Extend
  the E4 no-tests run to assert that the fix story's red gate reads a JUnit report.

### P3REQ-03. `delivery.ci` never leaves `none`, so a CI story is scheduled in every cycle and the new CI is never waited on

Severity: major. On the slice path: no.

Where:
- `state.md` 2.12, the `[delivery] ci` comment (lines 1091 to 1092) and the bullet "until it merges
  `delivery.ci = "none"` records that there are no checks to wait for" (lines 1244 to 1245).
- `state.md` 2.12 (lines 1035 to 1038), 2.4 `config` row (line 482), and blueprint 4.7 `config` row (line
  488): code writes `factory.toml` in three cases only, none of which is this key.
- `pipeline.md` 2.9.4 route row (line 982): readiness lists "CI when `delivery.ci = "none"` and there is a
  remote"; 2.9.5 (line 1012), the same for the plan gate.
- `pipeline.md` 2.9.1, delivery bullet (lines 870 to 872): with `none`, delivery "does not wait for checks".
- Blueprint 3.4 row 4 ("waits once for a merge approval") and 4.12 `delivery.ci` row.

The problem. The design treats `none` as true "until it merges", but nothing changes the value when the CI
chore story merges. No agent can write the file, code may write it only in three stated cases, and the
briefing does not ask the person to. So after the first CI story is delivered:
- Every later `feature` route lists a CI readiness chore again, and every later `plan` gate demands one. Each
  touches `.github/**`, a protected path, and opens a `merge` decision for a person.
- `story deliver` keeps skipping the CI wait, so a red check on the CI the factory itself added is ignored.

What breaks. Starting point 4 ("waits once") becomes "waits every cycle". D15's merge rule then lets stories
merge past failing CI.

Proposed resolution:
- Derive the fact instead of storing it. `delivery.ci = "none"` stops counting once a chore story with method
  `ci` has a delivery closure in the project. From then on delivery waits for checks, readiness lists no CI
  chore, and the briefing names the obsolete line once.
- State this in `state.md` 2.12, `pipeline.md` 2.9.1, 2.9.4 and 2.9.5, and blueprint 4.12. Add an E4
  assertion: the second cycle on the no-CI sample schedules no CI story.

### P3REQ-04. `ingest` measures the baseline before it knows the project's commands, and nothing measures it again

Severity: major. On the slice path: no.

Where:
- `pipeline.md` 2.9.8: the "coverage and baseline" stage (line 1080), `vsdd gate ingest`, measures the suite
  with failing-test identities, lint and format. It runs before "synthesize" (line 1081), where the architect
  writes `toolchain.toml` "with the commands the project uses".
- `state.md` 2.12, "Missing commands are a state" (lines 1230 to 1246): the measuring workflows that refuse
  without commands are `wave`, `story`, `verify` and `maintain`, not `ingest`. The commands "arrive by
  approval", in the first survey after the `spec` decision.
- `state.md` 2.8, the baseline record (lines 779 to 790): for an import "the first survey that needs the
  measurements takes them". There is no such rule for an ingest.
- Blueprint 4.12 (lines 729 to 731) and 4.13, the baseline bullet; `packaging.md` E4 run (1) (line 763), which
  asserts "the inherited lint failure does not fail any gate".

The problem. When `vsdd init` cannot detect the test command, its JUnit report or the lint command, the
baseline records no failing-test identities and no lint findings. `init` detects only nextest, `pytest
--junitxml`, jest-junit and go-junit-report, so a custom runner, plain `cargo test` or Gradle all fall
outside it. The commands then arrive by approval, and the gates compare against a baseline that lists
nothing. Every test that already failed, and every existing lint finding, is "new". The green gate fails on
inherited debt the story cannot touch: the implementer may not edit tests, and the failures are unrelated.
That gives `blocked`, then `stuck`, and no command measures the baseline again. Readiness also reads "the
baseline has no suite" and schedules a harness chore for a project that has a suite but lacked only a JUnit
flag.

What breaks. Starting point 1, for any codebase whose toolchain `init` does not fully detect. That includes
the E4 sample `tally`: its `tools/junit_runner.py` is not among the detected reports, so E4 run (1) would
fail its own assertion.

Proposed resolution:
- When `gate ingest` runs with `test`, `test_report`, `lint` or `format_check` unset, it records each of those
  measurements as "not taken" rather than as empty.
- The first survey after the approval applies the toolchain and takes the missing measurements, as it does
  for an import (as a job, see P3REQ-06).
- Readiness reads "has a suite" from the taken measurement.
- Apply this in `pipeline.md` 2.9.8, `state.md` 2.8 and 2.12, and blueprint 4.13.

### P3REQ-05. A hub of existing repositories cannot ingest its second repository

Severity: major. On the slice path: no.

Where:
- `pipeline.md` 3.2 (line 2273): old mode 4 "Brownfield multi-repo" becomes "`ingest` with `repo` for each
  repository, then `multi` stage `synthesize`"; 2.9.11 synthesize row (line 1187), "Every existing repository
  is ingested".
- `state.md` 2.8: the closure row (line 737) and the baseline record (line 779) name one
  `cycles/_standing/units/ingest.json`. The ingest's decision is `spec` in `cycles/_standing/decisions/`
  (`pipeline.md` 2.9.8 close row; blueprint 4.8 identifiers).
- `pipeline.md` 2.4, the two ingest route rows (lines 291 and 292): "there is no baseline (no ingest...)".
- `pipeline.md` 2.6, the whole-set lock (lines 540 to 545); `state.md` 2.16, survey row: only `spec`,
  `feature` and `--amend` runs hold an amendment.
- `state.md` 2.5, toolchain row (line 554): one `[commands]` table; the survey fills top-level keys only.
- `state.md` 2.14 and `pipeline.md` 2.9.11 (lines 1196 to 1202): per-repository records exist for waves
  only.
- Blueprint 3.4 row "Several repositories" and 4.13.

The problem. Run per repository as the mapping says, the second ingest fails at several points:
- Its survey's done test reads the closure `ingest.json`, which the first ingest closed, so every stage
  reads as done.
- If it did run, its baseline would overwrite the first one's, and its decision would have the same
  identifier, `spec`, in the same cycle. Qualified identifiers exist only for two open cycles.
- Once the first baseline is approved, the whole specification set is locked and an ingest holds no
  amendment, so the second ingest's recovered contracts are refused.
- After the first approval "a baseline exists", so the route never names the second ingest.
- The gates compare every story against one baseline, measured with one repository's commands. The
  toolchain file cannot carry per-repository commands into `[[repo]]` entries.
- Which run opens the hub's single `release` decision is not stated: per-repository `verify` runs, or
  `multi`. `verify.json` and the decision `release` are per cycle.

What breaks. Starting point 5 for repositories the factory did not build. E4 run (4) passes only because its
hub fixture never ingests.

Proposed resolution:
- In a hub, `ingest` without `repo` covers every member in one run, with its stages fanned out per
  repository. It stores one `baseline` object per repository inside the ingest closure, each with that
  repository's integration head and measurements, which the gates read for stories with that `repo`.
- It writes `toolchain.toml` with one `[[repo]]` section per member, whose approval fills that entry's unset
  commands, and it ends at one `spec` decision.
- A member added later is ingested with `repo` in a run that holds an amendment.
- The route's ingest row reads "a member repository has no baseline".
- `verify` in a hub runs once at the hub over the repositories the cycle touched and opens the one `release`
  decision.
- Apply this in `pipeline.md` 2.4, 2.9.8, 2.9.11 and 3.2, `state.md` 2.5, 2.8 and 2.14, and blueprint 3.4 and
  4.13. E4 run (4) starts from two existing repositories and ingests them.

### P3REQ-06. The survey runs long measurements in a plain clerk call, with no job and no `pending` state

Severity: major. On the slice path: no.

Where:
- Blueprint 4.9, "Long commands" (lines 593 to 600), and `pipeline.md` amendment 4 (lines 22 to 27): gates
  that build or test, `story deliver` and `release` run as re-attaching jobs. The survey is not among them.
- `state.md` 2.16: the survey row (line 1379) and the job paragraph (lines 1414 to 1422).
- `pipeline.md` 2.10.8, survey fields (line 1537): no `state`.
- `pipeline.md` 2.5 (lines 362 to 367) and `state.md` 2.8 (lines 801 to 802): "the survey measures the gate
  again" whenever a closure names a passed gate and the evidence is missing.
- `state.md` 2.8 (lines 783 to 784) and 2.15 step 4 (lines 1326 to 1327): an import's whole baseline is taken
  "by the first survey that needs" it, including the mutation score per module and the scanners.
- Blueprint 7, fresh clone row (line 1207); `state.md` 2.10, fresh clone row (line 972).

The problem. On a fresh clone, or after evidence retention, the survey of a wave in flight re-measures the
green gate of every story whose closure names one, plus the plan's baseline check. Each is a build and a full
suite. After an import, the first survey runs the whole suite, lint, format, mutation testing per module and
the scanners. All of this happens inside one clerk Bash call that is stopped at ten minutes. The survey holds
the lock by then, because it takes the lock before the done test, and it has no job to re-attach to and no
`pending` answer. The retry finds its own run's lock, or measures from scratch again.

What breaks. "Resume from a cold start" (user decision 9) and "a fresh clone resumes from the last
checkpoint" (blueprint 7) on any project whose suite is not quick, and the first `feature` after every import.

Proposed resolution:
- The survey's measurements run as the same keyed detached jobs the gate commands use. `vsdd survey`
  re-attaches like any long command and returns `{ok: true, state: "pending"}` until they finish; the clerk
  already re-runs a pending command.
- Add `state` to the survey fields, and list `survey` among the long commands in blueprint 4.9, `pipeline.md`
  amendment 4 and `state.md` 2.16.

### P3REQ-07. A resumed `feature`, `verify`, `maintain` or `wave` run cannot continue a story an earlier run admitted

Severity: major. On the slice path: no. The standalone `story` survey on the slice path must record its unit in
its own run for the same reason.

Where:
- `pipeline.md` 2.9.1 (lines 819 to 822): "A parent may pass only a unit the tool returned to this run: every
  gate and `vsdd story deliver` check that the story is recorded in the run they are given."
- `state.md` 2.8, run record (line 739): "the stories admitted to it".
- `state.md` 2.16, `gate plan` row (line 1430): "Run again for a story it has admitted, it changes nothing
  and returns the same unit"; `story new-fix` row (line 1435).
- `pipeline.md` 2.10.8, Story unit row (line 1538): returned only by the `story` and `wave` surveys and the
  admission commands. The `feature` survey row (2.9.4, line 981) returns "the change request in flight, the
  baseline, the base branch", and the deliver stage takes units only from the story-draft step's
  `data.units`.
- `state.md` 2.8, the done test (lines 797 to 799): step records count only inside the run that wrote them.

The problem. Admission records a story in the run that admitted it. Suppose the session ends while a bug-fix
story is in review: the run is never checkpointed and the lock is released at session end. The person types
the same `feature` command and gets a new run. Story-draft is not done in any closure, so the story-writer
runs again. `gate plan --story` "changes nothing" for the admitted story, so it is not recorded in the new
run. The child's gates then refuse it, and it is `blocked`, then `stuck`. If the earlier run did reach its
checkpoint, for example with `failed` from the `finally` path, story-draft is closed and skipped. The new run
then has no units, because the `feature` survey returns none. Fix stories admitted by an earlier `verify`,
`maintain` or `wave` round are in the same position, since wave membership is the plan's map and fix stories
are not in it.

What breaks. "Re-running the same command in a new session continues" for every fast route, which is the bug
fix in starting point 2, and for every fix round.

Proposed resolution:
- The survey of each workflow that calls `story` returns, as `units`, every story the open cycle admitted for
  that workflow (and for a wave, its fix stories) that is not delivered, and records each in the new run.
- `gate plan --story`, run again for a story an earlier run admitted, records it in this run and returns its
  unit.
- When story-draft is done, the `feature` deliver stage takes its units from the survey.
- Apply this in `pipeline.md` 2.9.1, 2.9.4 and 2.10.8 and `state.md` 2.8 and 2.16 (R3's rewording of the
  same `gate plan` sentence should carry it). Add a harness scenario that kills a fast route after admission.

### P3REQ-08. A resumed standard-route `feature` run holds no amendment, so its writes are refused

Severity: major. On the slice path: no.

`pass2-closeout-3.md` noted this under "Seen while checking". It is not one of R1 to R5, so it is reported
here to make sure it gets a recorded outcome.

Where:
- `state.md` 2.16: the `feature route` row (line 1432) writes "the amendment flag in the run record". The
  survey row (line 1379) marks an amendment only for `spec` and for `--amend`.
- `state.md` 2.8, run record (line 739); S7 `spec_locked` (line 1606): locked "when the latest `spec`
  decision is approved and no live run record holds an amendment".
- `pipeline.md` 2.9.4, delta row (line 982); 2.6 lock paragraph (lines 540 to 545).

The problem. Only `vsdd feature route`, the delta step's closing command, marks a `feature` run as holding
the amendment. Suppose a standard-route run checkpoints with its delta stage done and the rest not done, for
example `failed` from the `finally` path or `partial` at the agent budget. The next run skips the delta stage,
so its run record has no flag. Every `evolve`, check-fix and review-fix write by the product-owner, the
architect or the ux-designer is then refused by the specification lock.

What breaks. Resume of the standard route, which is how a second feature (starting point 2) and every
larger brownfield change are delivered.

Proposed resolution:
- The `feature` survey marks the run as holding the amendment when the cycle's `route` is `standard` and
  the cycle has no approved `spec` decision of its own. This is the predicate closeout-3 proposes for R2.
- List `vsdd feature route` and the `feature` survey among the run record's writers.
- Apply this in `state.md` 2.8 and 2.16 and `pipeline.md` 2.9.4.

### P3REQ-09. The spike ledger carries hidden-scenario text out of the holdout wall

Severity: major. On the slice path: no.

Where:
- Blueprint 6, walls row (line 1169): the union of walls "is what lets `code-reviewer`, `holdout-evaluator`
  and `scenario-reviewer` ask safely".
- `pipeline.md` 2.8: the ledger entry holds the question (lines 697 to 699), and `spike record` appends the
  result (lines 702 to 703). "Ledger and retention" (lines 778 to 781) keeps one project ledger for every
  entrance.
- `pipeline.md` 2.11.1, `runSpike()` (lines 1674 to 1676), puts the asker's question into the runner's
  prompt.
- `agents-skills.md` 2.9, read walls (lines 567 to 581): the `spikes` class is denied only to the
  `holdout-evaluator`, so builders, the story-writer and the architect read it.
- `hooks.md` 2.11.6, read-walls bullet (lines 1244 to 1251), protects what the runner reads and the cached
  findings, not what the entry holds.
- `packaging.md` 2.6, universal test 10 (lines 589 to 591), plants its marker only in findings.

The problem. The walls protect what a spike runner can read. They do not protect what the asker gives it.
The `product-owner`, the `scenario-reviewer` and the `holdout-evaluator` read hidden scenarios, and any of
them may ask, with a question of up to 400 characters. That question, and the runner's answer to it, are
written into `.factory/spikes/`, which the test-writer and the implementer may read. The question is also
written into the prompt of a runner that is itself denied `holdout`, and the runner may copy it into a kept
worktree that builders can open. The design walls every finding the `scenario-reviewer` returns, but not its
spike requests.

What breaks. The holdout wall (user requirement: walls enforced by hooks), on exactly the path that section 6
calls safe.

Proposed resolution:
- A spike whose asker may read `holdout` is walled. Its ledger entry is stored in the `holdout-evaluations`
  class, and elsewhere it is named by its ledger id only. Its kept worktree is denied to every caller denied
  `holdout`. Its finding is returned from the cache only to an asker that may read `holdout`.
- Universal test 10 also plants its marker in a spike question.
- Apply this in blueprint 6, `pipeline.md` 2.8, `hooks.md` 2.11.6, `state.md` (spike ledger row and 2.16) and
  `packaging.md` 2.6.

### P3REQ-10. Local-mode delivery has nowhere to merge without touching the person's checkout

Severity: major. On the slice path: yes. The slice delivers "in local mode with no remote".

Where:
- `pipeline.md` 2.9.1, delivery bullet (lines 870 to 872): "With `delivery.mode = "local"` (no remote) it
  merges locally", and nothing more.
- `pipeline.md` 3.6 (line 2373): "Nothing in 1.0 pulls or merges in the person's checkout". `state.md` 1
  (line 160) and 2.3 (line 221): the person's checkout is "never switched, pulled or committed to".
- `pipeline.md` 2.9.2 (lines 907 to 908): the tool's `.worktrees/_integration/` worktree.
- `hooks.md` 2.11.4, merge-guard (lines 1149 to 1155), "which matters most in local delivery mode, where the
  integration branch is the product branch".
- `state.md` 2.16, `story deliver` row (line 1434); blueprint 4.12, `delivery.mode` row.

The problem. With no remote, the integration branch is normally the branch checked out in the person's main
checkout. `vsdd init` reads "the current branch names". Git does not let the tool's integration worktree
check out a branch that another worktree has checked out. That leaves two options:
- Merge in the person's checkout. This breaks the stated rule, and it fails or mixes changes when that
  checkout has uncommitted work.
- Move the branch ref from elsewhere. The person's index and files then disagree with their `HEAD`.

The design picks neither, so the G4a builder must choose, and either choice breaks a stated promise.

What breaks. The slice's delivery step on the sample project. Beyond the slice, D15's unattended merge, and
D1's promise that the person's checkout is free.

Proposed resolution:
- In local mode the integration branch is checked out only in `.worktrees/_integration/`, and `story deliver`
  merges there, with git hooks running as usual.
- `vsdd init` detects a person's checkout on that branch, and `setup` offers the one command that moves it to
  another branch. `vsdd doctor` reports it.
- `story deliver` returns `refused` with that reason instead of merging elsewhere.
- Apply this in `pipeline.md` 2.9.1, `state.md` 2.12 (`[delivery] mode`) and 2.16, `hooks.md` 2.11.4 and
  blueprint 4.12. The slice's sample fixture leaves its main checkout on a non-integration branch.

### P3REQ-11. The person's quick spike opens with the wrong command form

Severity: minor. On the slice path: no.

Where:
- `pipeline.md` 2.11.1, `runSpike()` (line 1677): the opening command is always `spike open --run <id>
  --step ${stepKey}`.
- `pipeline.md` 2.8, the who-asked table (line 716), and `state.md` 2.16, `spike open` row (line 1410): the
  `spike` workflow opens with `--run <id>` alone and reads the question from the run record.
- `pipeline.md` 2.9.12 (line 1214): the `spike` workflow makes "one `runSpike()` call".
- `hooks.md` 2.11.6, "Who asked" row (line 1220): names only the step record and the admission.

The problem. Ported as written, the `spike` workflow passes either an empty step key, which makes `--step`
consume `--ref`, or an invented one, which `spike open` cannot find. Either way the person's own spike
returns `refused`. `hooks.md` also omits the run-record source, so the H8 and S6 builders read different
rules.

What breaks. The first entrance in blueprint 6, the one the user named first.

Proposed resolution:
- `runSpike()` omits `--step` when the step key is empty, and the `spike` workflow calls it that way.
- `hooks.md` 2.11.6 adds the third source: the run record of a `spike` run, whose asker is the main session.

### P3REQ-12. Spike retention can delete an open named spike's worktree

Severity: minor. On the slice path: no.

Where:
- Blueprint 6, Retention row (line 1170): "14 days, or until the asking unit is delivered".
- `pipeline.md` 2.8, "Ledger and retention" (lines 778 to 781): remove "spike worktrees older than 14 days".
- `state.md` 2.17 (lines 1455 to 1458) and 2.10, `--fix` (line 948): the same, while only "the branch
  `spike/<name>` of a named spike is kept until the spike is closed".
- `state.md` 2.12, `retain_days` comment (line 1122): "how long a finished spike's worktree is kept".

The problem. Three texts count retention by age, and only the configuration comment limits it to finished
spikes. A named spike "spans many sessions". Its worktree holds `.spike/STATE.md`, `.spike/TASKS.md` and any
work a person did by hand. On day 15 `vsdd checkpoint` or `status --fix` removes that worktree while the
spike is still open. Attaching with `spike open --name` is not said to recreate it. The user's
`resume-spike` convention then finds no worktree.

Proposed resolution:
- Count retention from the entry's verdict, or from delivery of the asking unit. Never remove the worktree
  of an open named spike.
- Use the same wording in blueprint 6, `pipeline.md` 2.8 and `state.md` 2.10, 2.12 and 2.17.

### P3REQ-13. A fresh clone of a factory project starts silent; nothing tells the person to run setup

Severity: minor. On the slice path: no.

Where:
- `hooks.md` 2.8, condition table, `unmounted` row (line 844): "Inert and silent".
- Blueprint 7, fresh clone row (line 1207); `state.md` 2.10, fresh clone row (line 972); `state.md` 2.3,
  "What is left for a person: run `/vsdd-factory:setup` once per clone".
- `state.md` 2.16 (line 1368): outside an initialised project `vsdd status` exits 2.

The problem. The requirement is a briefing that says whether the project is initialised. On a fresh clone the
state branch exists but is not mounted, so the session prints nothing. `/vsdd-factory:status` exits 2, and
the person or the model has to know to type `setup`. The legacy case already prints one fixed line from
shell. The unmounted case can be recognised the same way, by reading `.git/packed-refs` or
`.git/refs/remotes/*/factory-artifacts` without starting a process.

What breaks. The cold-start and "setup once per clone" experience on a second machine.

Proposed resolution:
- In `brief` mode the launcher recognises `unmounted` by that file read and prints one fixed line naming
  `/vsdd-factory:setup`, as it does for `legacy`.
- `vsdd status` reports `unmounted` with the same line instead of exiting 2.
- Apply this in `hooks.md` 2.8 and `state.md` 2.9, 2.10 and 2.16.

### P3REQ-14. Pre-1.0 story worktrees stay at the paths 1.0 will use for the same story identifiers

Severity: minor. On the slice path: no.

Where:
- `state.md` 2.15, steps 1 to 3 (lines 1317 to 1320) and the stories row (line 1344): identifiers are kept,
  an in-flight story comes back through a queued request, and the import refuses only worktrees with
  uncommitted work.
- `packaging.md` cutover `apply` step 1 (lines 899 to 900).
- Blueprint 4.1, worktrees row (line 255): `.worktrees/<story-id>/`.
- The old layout is `.worktrees/S-N.MM/` on `feature/S-N.MM-*` branches
  (`reference/old-workflows-packaging-map.md` line 310). Two of the eight projects have such directories
  today: `prism` has `.worktrees/S-3.09`, and `pregolya` has `.worktrees/S-1.02`.

The problem. After an import, clean old story worktrees remain. When the queued request re-plans an
in-flight story under its kept identifier, the worktree library tries to create `.worktrees/S-3.09/` where the
old worktree sits on a `feature/` branch. Admission or the survey then fails. Until then, the hook's
path-based story mapping also treats the old directory as that story's worktree.

Proposed resolution:
- `vsdd import` removes every clean pre-1.0 story worktree under `.worktrees/`, keeps their branches, and
  lists them in its report and dry run.
- Apply this in `state.md` 2.15 and the cutover description in `packaging.md`.

---

## Checked and found sound

- Model policy end to end: four tiers from one roster, D5's override for ingest's inventory stages, every
  step setting model and effort, reviewers on Opus, the arbiter on the architect as D18 adopted, aliases
  watched weekly.
- The hook launcher: no download and no network, a 3.11 floor, a guard for a missing `uv` and missing plugin
  files, inert outside factory projects, timeouts in seconds, one generated `hooks.json` with no variants and
  no activation.
- Standard library only on both the hook and tool paths. Node and Docker appear only for D14, D13 and the
  optional observability stack.
- macOS and Linux: POSIX shell, and the two-platform probes named.
- Compaction and warm start: the `PreCompact` flush is wired, the briefing is computed from records on every
  source, and the driver continues after compaction and after a decision.
- State on its own branch, committed and pushed by code with empty hooks and no signing (D1, D2). Size caps
  are checked before the write. One status entry point replaces six skills.
- Pipeline requirements: one level of nesting with a static test, three routine stops, a terminating
  convergence rule with one arbitration, commands from configuration, one per-story playbook.
- Spikes: the six entrances against the probe results, isolation passed by steps, asker types from
  hook-written records, the limits from D21, no nesting, containment by write scopes and git rules, the
  shared stamped wrapper, and findings carried across retries.
- Decisions D1 to D30 as listed in the table, including D7 (the typed `decide` captured before any model
  reads it), D9's two lenses, D15's three waiting cases, D16 to D20, D22 to D24.
