# Second pass: cold read of the blueprint as its builder

Reviewer: cold-read (pass 2). Read on 2026-11-23: `rewrite/STATE.md`, `rewrite/BLUEPRINT.md` in full, and the five
design documents (`pipeline.md`, `state.md`, `agents-skills.md`, `hooks.md`, `packaging.md`) in full. Decisions D1 to
D30 are treated as fixed.

## Verdict

The blueprint can be built in outline. The shared names in section 4 mostly hold across the five designs: the 20
agents, 13 skills, 12 workflows, 17 rules, four policy tables and four tiers are named the same way everywhere. Read
as a builder, though, it fails in two places. First, the walk-through in section 3.3 cannot run as written: a story
drafted in the middle of a run gets no worktree and no survey unit, so the `feature` fast routes, and every fix story
in `wave`, `verify` and `maintain`, have nothing to start from (P2COLD-01). Second, the write scopes give no agent the
right to write build manifests, root configuration or harness directories, so chore, harness and facade stories
cannot be built, and nor can any story that adds a dependency (P2COLD-02). Greenfield fails at its first wave because
of both. The major findings follow. Some leave an adopted decision with no home: the D4 allow-list, the D5
ingestion override, the D6 switch (which D12 also contradicts) and the D19 amendment flow. Others are mechanisms that
do not work as specified: re-freezing tests after a test fix, how the tool learns who is calling it, passing a
request's text to a clerk command, holdout content leaking through packets and verdict records, and a spec lock that
covers only files that carry an `approved` status. On the user's aside: brownfield is designed, as `ingest` then
`feature`, and `feature` alone after a release. But the gates assume a green, fully traced codebase that the factory
built itself (P2COLD-12), and no build package ever proves a brownfield or post-release feature run end to end
(P2COLD-13). The build plan also has a dependency cycle and several missing dependencies (P2COLD-16). In all: 2
blockers, 15 major findings (P2COLD-03 to 16 and 20) and 15 minor findings that tidy names, keys, schemas and packages.

---

## P2COLD-01: Stories created during a run have no worktree, no survey unit and no valid preconditions

- **Severity:** blocker
- **Locations:** `BLUEPRINT.md` 3.3 steps 1, 3 and 4 ("The survey has already made a worktree at `.worktrees/S-9.01/`"),
  4.9 (survey row; "`vsdd worktree` (now library functions inside `story deliver` and `survey`)"); `pipeline.md`
  2.9.1 (a child "skips its own survey and checkpoint"; survey precondition "the story's wave open"), 2.9.4
  (story-draft, then deliver as a child), 2.9.2 fix row, 2.9.6 (`vsdd story new-fix --verify`), 2.9.10, 2.11.3
  (`wave.js` passes `nf.units` from `story new-fix` as `position`); `state.md` 2.5 (story is `ready` "when the plan
  closes"), 2.11 (one lock scope for every run that merges), 2.16 (`story new-fix` writes fix stories only);
  `hooks.md` 2.11.4 ("the tool's survey creates a story's worktree only when the story is ready").
- **Problem:** The survey makes worktrees and units, and it runs first. In `feature`, the story-writer writes S-9.01
  after the survey (step 3), so the claim in step 4 cannot be true. The child `story` needs `position` (worktree,
  branch, base, acs, bcs, criticality and so on) from a parent survey that never saw the story. The child cannot run
  its own survey either. The parent holds the lock on the same scope, so the child's survey is refused. Even without
  the lock, the precondition "the story's wave open" fails, because fast-route stories are in no wave (waves come only
  from `plan schedule`). The story also stays `draft`: only the plan's close sets `ready`, and the hook design makes
  worktrees only for ready stories. The same gap affects fix stories. `vsdd story new-fix` is specified to write story
  files, yet `wave.js` treats its output as survey units with worktrees, and no command is assigned to make them.
- **What breaks:** Every `feature` trivial, bugfix and hotfix route, which is the 3.3 walk-through itself, and every
  fix round in the `wave`, `verify` and `maintain` loops. Those loops then either fail or run to the cap.
- **Resolution:** Make the tool return a complete unit for any story it learns about mid-run. `vsdd gate plan --story
  <id>` (for the fast routes) and `vsdd story new-fix` (and `gate wave` when it returns fix units) each check the story,
  set `ready`, record its membership (the feature run's closure, or the wave's closure for a fix), create the branch
  and worktree with the worktree library, and return the story-unit object of `pipeline.md` 2.10.8. Define the
  preconditions a parent-supplied `position` must satisfy, so a child never needs its own survey. Correct 3.3 step 4
  to: "The plan gate has made a worktree at `.worktrees/S-9.01/` and returned the story unit".

## P2COLD-02: Manifests, root configuration and harness directories are writable by no agent, so chore, harness and facade stories cannot be built

- **Severity:** blocker
- **Locations:** `BLUEPRINT.md` 4.3 rows 9 (implementer: `source`), 10 (verifier: `tests (harness directories)`), 13
  (devops: `ci`), 4.6 (rows `facade`, `chore`, `harness`), 4.7 last row; `state.md` 2.12 `[paths]` (only `source`,
  `tests`, `docs`, `ci`); `agents-skills.md` 2.9 write scopes ("`source`, including build manifests"; "`ci` and
  repository configuration files"); `hooks.md` 2.7 (`walls.toml` rows), 2.11.2 `write-scopes` check 2;
  `pipeline.md` 2.9.5 readiness (scaffold and CI are `chore` stories built by devops), 2.9.1 (`facade`: mutation
  requirement at the wave gate).
- **Problem:** Write-scopes check 2 blocks any path that lies in none of the caller's granted classes. Build
  manifests and lockfiles (`Cargo.toml`, `package.json`, `pyproject.toml`), toolchain files (`rust-toolchain.toml`,
  `.nvmrc`), `.gitignore`, `Makefile` or `justfile`, `build.rs`, `benches/`, `fuzz/`, `examples/` and `migrations/` are
  in no class. So:
  - The scaffold chore that `plan` puts first for an empty repository needs to create source and manifests, but
    devops may write only `ci`.
  - Harness stories need `benches/`, `fuzz/` and proof harnesses (often in `src/` for Kani). The verifier may write
    only `tests`.
  - The implementer cannot add the dependency that `dependencies.toml` approves, because the manifest is not
    `source`.
  - A `facade` story has no tests stage and its implementer may never write tests. Yet the wave gate measures a
    mutation score for it, which cannot pass with no tests.
- **What breaks:** The greenfield route at its first wave, every harness story raised by `verify`'s formal and
  performance checks, every service clone, and every story whose change needs a manifest edit. Agents told "do not
  reach the same effect another way" will either stall or route around the wall with `sed`.
- **Resolution:** Add two path classes from configuration and detect both at `vsdd init`:
  - `manifests` (`[paths] manifests`: manifests, lockfiles, toolchain and build files);
  - `harness` (`[paths] harness`, for example `benches/**`, `fuzz/**`, `proofs/**`).

  Then change the grants:
  - implementer: `source` and `manifests`;
  - verifier: `harness` and `tests`, plus `source:` restricted to the property's module for proofs that must live
    there;
  - devops: `ci` and `manifests`, plus `source` and `tests` only for a `chore` story with method `repo-setup`.

  Give `facade` a tests stage (test-writer, method `failing-tests`, no red requirement), or replace its mutation
  requirement with the clone-fidelity check. Add a lint rule: for every story kind and method, the implement agent can
  write every path class the method's output names.

## P2COLD-03: Greenfield on an empty repository has no path to configured build and test commands

- **Severity:** major
- **Locations:** `state.md` 2.12 ("A missing `test` command or `test_report` is an error"; `factory.toml` is written
  only at `init` and then only by a person), 2.3 mounting; `BLUEPRINT.md` 1 ("three routine decisions"), 4.1
  (`factory.toml` "edited only by a person"), 4.12 (`test_report` required); `pipeline.md` 2.4 route table, 2.9.5
  readiness; `BLUEPRINT.md` 9.6 E3 ("A greenfield run on the sample project from a brief").
- **Problem:** `vsdd init` runs before `spec`, and on an empty repository it can detect no toolchain. The toolchain is
  chosen later, by the architect. After that no tool call may write `[commands]` or `[paths]`. The scaffold story's
  green gate and `gate baseline` need those commands, so a person must edit `factory.toml` by hand between `spec` and
  `wave`. No route row, decision or briefing line says so.
- **What breaks:** Every greenfield project gets an unannounced fourth human stop. The driver loops or stops with
  `stuck`, and the "three routine decisions" claim is false. E3 fails unless its sample project already has a
  toolchain, in which case it does not test greenfield.
- **Resolution:** The architecture names the toolchain in a registry artifact (`specs/architecture/toolchain.toml`:
  commands, test report and path globs), approved with the `spec` decision. When the person approves `spec`,
  `settle_decision` writes those values into `factory.toml`. That is a second sanctioned code write, like the
  `[[repo]]` append of `repos split`, and the next briefing announces it. Add a route row "commands not configured →
  human: the lines to add, or approve `spec`", and make E3 start from an empty repository.

## P2COLD-04: A test-writer fix to a frozen test always fails the green gate

- **Severity:** major
- **Locations:** `BLUEPRINT.md` 3.3 step 6 ("frozen tests unchanged"), 4.3 row 8; `pipeline.md` 2.9.1 (green gate:
  "the frozen tests are unchanged"), 2.7 (`high` includes "a test cannot fail"), 2.11.2 (`fix` runs
  `gated(who, …, 'green')` with `who = 'test-writer'` for test findings); `state.md` 2.8 evidence lookup; `hooks.md`
  2.11.4 (`tests:frozen` lets the test-writer change them).
- **Problem:** Review findings in the `test` category, including the common high finding "a test cannot fail", go to
  the test-writer. The hook allows that edit, but the green gate then sees a changed frozen test and fails. The red
  gate cannot be re-recorded either, because once the implementation exists the tests pass. Nothing re-freezes the
  set.
- **What breaks:** Every story review that finds a weak test spends its attempts on green failures, runs to the cap
  and opens a `finding` decision for something the design meant to fix automatically.
- **Resolution:** Give the gate a way to accept test changes made by the test-writer. Either:
  - `vsdd gate red --refreeze --story <id>` (test-writer only): it records new hashes for the changed frozen tests,
    and their identities must still exist and still pass;
  - or the green gate accepts a frozen-test change when the commit that made it is the head of a stored test-writer
    step record.

  Add a harness scenario for "test finding fixed by the test-writer".

## P2COLD-05: The tool cannot learn reliably who is calling it, so the lock, the spike ledger and spike walls rest on guesses or self-declaration

- **Severity:** major
- **Locations:** `BLUEPRINT.md` 4.13 (liveness; "Hooks must be alive"), 4.10 spike row ("asker's agent type, filled in by
  the wrapper"), 6 (Walls row: "The ledger entry records the asker's agent type, and the hook applies the union");
  `state.md` amendments 9 and 12, 2.11 ("`vsdd` learns which session is calling from the marker"); `hooks.md`
  amendments 8 and 13, 2.11.6 (entry "matched by the runner's `agent_id`, which … `spike open` writes into the
  entry"), 2.11.10; `pipeline.md` 2.11.1 `runSpike()` (`--asker ${RUN.wf}/${name}/${subject}` passed in the
  runner's prompt).
- **Problem:**
  - Session markers are per session (`sessions/<id>.json`). With two sessions live, which `state.md` 2.11 supports,
    a `vsdd` process has no way to tell which marker is its own.
  - With up to 16 concurrent agents in one session, a `spike open` process cannot learn its runner's `agent_id` at
    all.
  - Worst, the asker's agent type that decides the spike runner's walls is an argument the spike runner types itself.
    A prompt-injected runner can claim `--asker vsdd-factory:product-owner` and read specifications on behalf of the
    `code-reviewer`, or source on behalf of the `holdout-evaluator`.
- **What breaks:**
  - Locks get attributed to the wrong session, and the wrong lock is released at `SessionEnd`.
  - `spike-bounds` blocks the right runner, or admits the wrong one.
  - The wall inheritance that makes it safe for the walled reviewers to ask is enforced by instruction, against the
    requirement "information walls are enforced by hooks … not by instructions".
- **Resolution:** Have the hook pass identity to the tool. On `PreToolUse` for a `vsdd` Bash call, the hook rewrites
  the command through `updatedInput` so it is prefixed with `VSDD_CALLER_SESSION`, `VSDD_CALLER_AGENT_ID` and
  `VSDD_CALLER_TYPE`, overwriting any value the caller supplied. Add this as a section 10 probe in B8; if it fails,
  write the marker per agent id and key spikes by the worktree, the one thing the tool sees as its own working
  directory. Derive `asker_type` in code, never from an argument:
  - in a workflow, from the hook-stored step record of the asking step, named by run and step identity;
  - for a direct dispatch, from the hook's record of the `Agent` call that started the runner.

## P2COLD-06: A change request's text and other free text cannot reach a clerk-run command

- **Severity:** major
- **Locations:** `BLUEPRINT.md` 4.9 survey row (`[--request-file]`); `state.md` 2.16 (same); `hooks.md` 2.11.1
  (`tool_only`: one `vsdd` segment), 2.11.10 (inputs "as arguments or as a file under the temporary directory, never
  on standard input"); `pipeline.md` 2.11.1 (`safe()` keeps only `[A-Za-z0-9._/-]`; `runSpike` uses
  `--question-file -` and `--json-file -`), 2.3 (`feature request`, `guidance`, `discover subject` text, named spike
  `question`).
- **Problem:** A workflow script cannot write a file. The clerk has only Bash, restricted to a single `vsdd` segment,
  so it cannot write a file either, and standard input is ruled out. Nobody can create the request file that
  `--request-file` expects. No other way to pass free text (requests, guidance, a named spike's mission) is defined,
  and the `safe()` filter would erase it.
- **What breaks:** `/vsdd-factory:feature "<request>"`, the entry point of the walk-through, of brownfield and of
  post-release work, cannot open its cycle. `guidance` arguments to `spec`, `plan` and `feature`, and a named spike's
  question, are lost.
- **Resolution:** Define one encoding for free text: `--text-uri <percent-encoded>` (JavaScript `encodeURIComponent`,
  decoded by the tool), produced by a prelude helper `enc()`. Add `--request-uri`, `--guidance-uri` and
  `--question-uri` to the commands that take text. Lint every `relay()` command so free text goes through `enc()`.
  Replace the stdin forms in `runSpike` with a file under the temporary directory, which the spike runner can write.

## P2COLD-07: Holdout content leaks through decision packets, closure records, verdict records and `vsdd review show`

- **Severity:** major
- **Locations:** `pipeline.md` 2.7 (at the spec perimeter, open findings "are listed in [the packet] as open issues
  with both positions"), 2.9.3 (scenario-reviewer in the `review` stage), 2.9.2 (`holdoutFindings()`); `state.md`
  amendment 7 ("the closure record keeps the dismissed and waived findings in full"), 2.5 last table (open findings
  copied into the decision packet), 2.8 verdict record in `local/evidence/`; `BLUEPRINT.md` 4.3 walls, 4.13; `hooks.md`
  2.7 `[reveals]` (`"review show" = ["runs"]`) and the `[main]` row (denies `holdout` only).
- **Problem:**
  - The scenario-reviewer's findings quote hidden scenarios and point their `location` at `holdout/HS-*.md`.
    Findings built from wave and verify scores carry scenario identities and titles.
  - These findings are copied into decision packets (`cycles/**`), closure records (`cycles/**`) and verdict records
    (`local/evidence/**`). Builders may read all three, and the code-reviewer may read `evidence`.
  - `vsdd review show` reveals only `runs`, which the main session may read, although D6 walls the main session off
    from `holdout`.
- **What breaks:** The holdout wall, a stated requirement, is crossed by code. Builders, and the main session that
  composes their prompts, can learn scenario content.
- **Resolution:**
  - Mark every finding from a scenario-reviewer or holdout step, or with a location in `holdout`, as `walled`, and
    store its full form only inside the `holdout` class (for example `holdout/evaluations/<cycle>/findings.json`).
  - Packets, closures and verdicts carry a redacted line, checked by the same leak check used before fix stories are
    written.
  - Make `[reveals]` for `review show` include `holdout` whenever the unit has walled findings.
  - Add a policy test that no record outside the `holdout` class contains a scenario path or quote.

## P2COLD-08: The D19 amendment flow has no mechanism during an active cycle

- **Severity:** major
- **Locations:** `BLUEPRINT.md` D19, 4.11 (`spec-gap`: `amend`); `pipeline.md` 2.6 (`amend` "runs `feature` as an
  amendment and ends at decision `spec`"; "Specification gaps found during a wave are batched"), 2.9.4 survey gate
  (a request during an active cycle is queued), 2.4 route table; `state.md` 2.16 (`vsdd survey feature` "queues the
  request when one is active"; `feature route` marks an amendment only on the standard route).
- **Problem:** Choosing `amend` creates no request and no run. If the person types `/vsdd-factory:feature` for the gap,
  the survey sees an active cycle and queues it until the cycle closes. The cycle cannot close while the stories wait
  on the gap. Nothing says how the waiting stories come back after the amendment's `spec` approval: who re-traces
  their criteria, and whether `plan` runs again.
- **What breaks:** The first specification gap found during a wave deadlocks the cycle. D19 ("batched once per wave
  through an amendment run") cannot happen.
- **Resolution:**
  - `settle_decision` for `spec-gap amend` writes an in-cycle amendment request (`requests/CR-NNN.md` with `cycle`
    and `amendment = true`).
  - The `feature` survey runs an amendment request for the open cycle at once and marks the run as an amendment.
  - Add the route row "an amendment request exists and the open wave has finished → `feature`".
  - After the amendment's `spec` approval, `plan` runs in delta scope to re-trace the waiting stories and release
    them.
  - Put these rows in `routes.toml` and the precondition in `stages.toml`.

## P2COLD-09: The D4 project allow-list has no stored home, and as scoped it does not remove the prompts it is for

- **Severity:** major
- **Locations:** `BLUEPRINT.md` D4, 4.5 `setup` row, 5.4 last bullet; `hooks.md` 2.6 (`[hooks]` keys: no allow-list
  key), 2.10 row 17, 2.11.10 (`auto-allow`), 6 question 2; `packaging.md` amendment 12 ("`setup` offers to write, into
  the project's local settings, the permission allow rules"); `BLUEPRINT.md` 10 item 7 (static rules are the
  *fallback*).
- **Problem:**
  - The hook reads "a project allow-list" whose key and file are never defined. `setup` runs in the main session and
    cannot write `factory.toml`. Packaging instead writes static rules into `.claude/settings.local.json`, the
    mechanism section 10 calls the fallback.
  - As scoped, the list permits "the project's configured build and test commands". Builders run those commands with
    extra arguments, such as one test by name, which is the TDD loop of step 6. They also run read-only shell
    commands, and the evaluator and demo-recorder exercise the product. Every one of these still prompts.
- **What breaks:** D4's purpose ("unattended runs do not stop on permission prompts") is not met. Smoke probe 7, "no
  permission prompt", fails, and scheduled runs stall.
- **Resolution:** Pick one mechanism and give it a home. Have `vsdd init` open a `setup.allow` decision, settled by
  the person's typed `/vsdd-factory:decide setup.allow approve`, and record the answer under `[hooks.auto_allow]`
  (a sanctioned code write) or in a control record the hook reads. Widen the matching:
  - configured commands match as prefixes with additional arguments;
  - a fixed list of read-only commands (`ls`, `cat`, `head`, `wc`, `rg`, `find` without `-exec` or `-delete`, the
    `git-read` family) is allowed inside the caller's own worktree;
  - calls to the started product's port are allowed for the roles that hold the `env` grant.

  Keep the prompt-count probe as the acceptance test.

## P2COLD-10: The ingestion tier override adopted in D5 is not implemented anywhere

- **Severity:** major
- **Locations:** `BLUEPRINT.md` D5 ("Let the broad inventory passes of ingestion run on the builder tier as a marked
  override"), 4.2 ("Every workflow step sets both model and effort from the generated table"), 4.14; `pipeline.md`
  2.9.8 (`map` and `analyze` on the planner tier), 2.11.1 `step()` (tier only from `ROLE[type]` or the test override);
  `agents-skills.md` N-PIPE-2 ("except for an override marked as one in the script"); `packaging.md` 2.6 roster format.
- **Problem:** No roster key, prelude table, `step()` option or ingest stage carries the override. The only override
  is `VSDD_TEST_TIER`.
- **What breaks:** The adopted D5 ruling is lost. Ingestion runs every pass at the planner tier, the largest
  planner-tier cost the design identifies.
- **Resolution:** Add `[overrides]` to `tests/lint/roster.toml` (`"ingest/map" = "builder"`, `"ingest/analyze" =
  "builder"`). Generate it into the prelude as `OVERRIDE` and have `step()` consult it by `<workflow>/<stage>`. Lint
  allows only listed overrides, and a harness test asserts the model and effort the two ingest stages use.

## P2COLD-11: The D6 switch does not exist, and D12 forbids it

- **Severity:** major
- **Locations:** `BLUEPRINT.md` D6 ("on by default, with a switch in project configuration"), D12 ("Lock the walls,
  write scopes …"), 5.4; `hooks.md` 2.6 (locked rules `read-walls` and `write-scopes` ignore `strength`; setting it is a
  configuration error), 2.7 `[main]` row; `state.md` 2.12 (no such key).
- **Problem:** Two adopted decisions conflict, and neither design document defines the switch.
- **What breaks:** A builder must either break D12 by making the walls relaxable or break D6 by shipping no switch.
  Whichever is chosen silently overrides an adopted decision.
- **Resolution:** State the exception in 4.12 and `hooks.md` 2.6. Add one key, `[hooks] main_session_wall = true`
  (default). It relaxes only the `main` row (`holdout` reads and specification writes) and is announced in every
  briefing. Every agent row stays locked.

## P2COLD-12: The gates assume a factory-built, all-green codebase, so the brownfield route stalls

- **Severity:** major
- **Locations:** `pipeline.md` 2.9.1 green gate (build, full suite, lint and format pass on the head), 2.9.4 survey
  ("A baseline exists: a release or an ingest"), 2.9.5 readiness and the plan gate ("every contract is covered by a
  story criterion"), 2.9.6 ("`full` for a first release"), 2.9.8 Ends (recovered contracts "become approved contracts
  only through a later `spec` decision"); `BLUEPRINT.md` 3.3, 4.13; `state.md` 2.8 (`spec` approval promotes only the
  files in its digest).
- **Problem:** On a codebase the factory did not build:
  - Any pre-existing failing test, lint warning or format difference fails every story's green gate. That includes
    the gate of a fix story, because the other failures remain. No known-failures baseline exists.
  - The first `verify` after an ingest is a "first release", so it runs at full scope. It applies a system review per
    subsystem, criticality mutation thresholds and traceability to the whole legacy codebase, and turns each finding
    into a fix story.
  - The recovered contracts are never approved unless some `spec` digest covers them, so "specification approved"
    has no defined meaning for fast routes after an ingest.
  - The plan gate's full coverage rule would ask for stories covering every recovered contract.
- **What breaks:** `ingest` followed by `feature` (the user's brownfield case) cannot deliver a story in a typical
  legacy project, and its first `verify` cannot converge.
- **Resolution:**
  - At the end of `ingest`, `gate baseline` records the known-failing test identities and the lint and format
    findings as baseline evidence. The green, wave and release gates then require no new failures against that
    baseline, plus every frozen test passing. Known failures are reported by `vsdd report debt` and paid down through
    `maintain`.
  - Define "first release" as the first release since the baseline (an ingest counts), so the first verify runs at
    delta scope.
  - Make `ingest` end with a `spec` decision on the recovered baseline, or define the story precondition as "the
    contracts the story cites exist", so fast routes work after an ingest.
  - State that the plan gate's coverage rule applies to contracts added or changed in the cycle.

## P2COLD-13: No package proves brownfield or a post-release feature end to end

- **Severity:** major
- **Locations:** `BLUEPRINT.md` 9.6 (E2 one story; E3 "A greenfield run … from a brief", depending on W1 to W8 only,
  so not W9 `ingest`), CO2 ("E3 passed"); `rewrite/STATE.md` (the user asked; "both are now explicit cases the second
  pass must walk end to end").
- **Problem:** The two brownfield cases the user asked about, "existing project then `ingest` then `feature`" and
  "`feature` after the greenfield build", are never exercised before cutover. The eight real projects are brownfield,
  so these are the routes they will take first.
- **What breaks:** The faults in P2COLD-12, P2COLD-01 and P2COLD-06 reach users. Cutover is gated only on greenfield.
- **Resolution:** Add E4: run `ingest` on a sample with an existing suite and one known lint failure, then a `feature`
  standard route through to `release`, then a `feature` bugfix. Extend E3 so that after its release it runs one
  `feature` trivial route on the built product. Make CO2 depend on E4.

## P2COLD-14: `vsdd init` runs the importer, which only a person may run

- **Severity:** major
- **Locations:** `state.md` 2.3 Mounting ("With pre-1.0 state it runs the importer"), 2.16 `vsdd init` row ("Runs the
  importer when it finds pre-1.0 state"), contradicted by 2.15 and 2.16 `import` row; `BLUEPRINT.md` D3, 4.5 `setup`
  row, 4.9 first row; `hooks.md` 2.7 `[human_only]` (`import`, `init --fresh`; `init` is in the `author` group the main
  session holds).
- **Problem:** The main session runs `vsdd init` inside `setup`. As state.md specifies `init`, it performs the import
  that the blueprint reserves for the person.
- **What breaks:** A model-run command rewrites a project's state tree, bypassing the person-only rule that D3 and
  4.9 set. Builders of S1 and S5 will implement different behaviour.
- **Resolution:** `init` on pre-1.0 state performs no conversion. It reports `legacy` and prints
  `vsdd import --dry-run`, then `vsdd import` for the person. Correct `state.md` 2.3 and 2.16.

## P2COLD-15: Blocking `curl` upload flags for every agent stops the evaluator and the demo-recorder from exercising API products

- **Severity:** major
- **Locations:** `hooks.md` amendment 16, 2.11.1 `secrets` row ("For agents, `curl` and `wget` with an upload flag
  (`-d`, `--data*`, `-F`, `-T` …) are blocked"); `BLUEPRINT.md` 5.4 (`secrets`), 8 row OPS-16, 4.3 row 17 (evaluator
  has a shell "to start and exercise the product"); `pipeline.md` 2.9.2 and 2.9.6 (scenario checks).
- **Problem:** Scenarios for an HTTP API need POST and PUT requests. The guard against sending data out blocks them
  even against the product on localhost that `vsdd env start` launched.
- **What breaks:** Hidden-scenario evaluation and demo evidence for any API product. The evaluator scores scenarios as
  not run and the wave gate fails on thresholds.
- **Resolution:** Allow upload flags when every target URL is a loopback address on a port recorded by `vsdd env start`
  for the current run (written under `local/`). Keep the block for every other host. Add allow and block cases to the
  `secrets` case table.

## P2COLD-16: The build plan has a dependency cycle and missing dependencies

- **Severity:** major
- **Locations:** `BLUEPRINT.md` 9.1 to 9.6.
- **Problem:**
  - **Cycle.** B4 generates `hooks.json` from `py/vsdd/hooks/registry.py`, which H1 delivers, and H1 depends on B4.
  - **B4** also needs `tests/lint/roster.toml` (B6) for `TIER` and `ROLE`.
  - **Worktree library, two owners.** S3 delivers "worktrees", and G4 delivers "the worktree library", which S3
    needs and which comes later.
  - **S2** writes the status record's `next` but does not depend on C3, which holds the route table.
  - **G5** runs `gate baseline`, a job, without G1.
  - **Workflows:**
    - W2 has no S3, yet a standalone story needs survey and checkpoint; W1 and W10 also lack S3.
    - W11 lacks S6 (`report debt`, `discover collect`) and A3 (researcher sweeps).
    - W12 lacks A2 (architect `multi-repo-synthesis`).
  - **Skills:**
    - K1 lacks T1 (`doctor`, `--install-shim`), S5 (`import --dry-run`) and H6 (the allow-list).
    - K2 lacks A5 (`author doc`), T5 (`author diagram`) and G3 (`review diff` and `review path`).
  - **T1** (`doctor` runs `hooks selftest` and the canary) lacks H9.
  - **E1** smoke probe 7 (a two-story wave) needs W3 and its agents, but E1 depends only on W1.
  - **B8** claims "every testable item of section 10", but items 7 and 21 to 25 need much later packages.
- **What breaks:** A scheduler driven by this table deadlocks on B4 and H1, starts packages before what they call
  exists, or builds the worktree library twice.
- **Resolution:**
  - Split B4: B4a (prelude, schema literals and tier tables; depends on B0, C1, B6) and B4b (`hooks.json` generation;
    depends on H1).
  - Move the worktree library into S3 and have G4 use it.
  - Add the missing edges: S2→C3, G5→G1, W1/W2/W10→S3, W11→S6 and A3, W12→A2, K1→T1, S5 and H6, K2→A5, T5 and G3,
    T1→H9, E1→W3.
  - In B8, list the section 10 items it can settle with toy probes, and move items 7 and 21 to 25 to E1 to E3 and
    T5 by name.

## P2COLD-17: Commands from 4.9 that no package delivers

- **Severity:** minor
- **Locations:** `BLUEPRINT.md` 4.9, 9.2 to 9.6; `state.md` 2.16 (`step record`); `hooks.md` 2.7 (`--include-walled`),
  2.11.1 (`vsdd -C <dir>`).
- **Problem:** No package owns:
  - `stale`, `config --json` and `config set` (person-only);
  - `review path <lens>`;
  - `hooks canary`;
  - the repair half of `status --fix`;
  - the global `-C <dir>` option that the clerk and `auto-allow` rely on, and which is missing from 4.9;
  - `--include-walled`;
  - the conditional `step record` fallback.
- **What breaks:** The `review` skill (needs `review path`), `vsdd doctor` (needs the canary), the clerk's
  single-segment calls (need `-C`), and the section 10 item 1 fallback have no builder.
- **Resolution:** Assign them:
  - S1: `config --json`, `config set`, `stale`, `-C`;
  - S2: `status --fix`, plus `step record`, built if B8 item 1 fails;
  - G3: `review path`;
  - H1: `hooks canary`;
  - H3: `--include-walled`.

  Add `-C` and `--include-walled` to 4.9.

## P2COLD-18: Some packages are too large for one builder and one reviewer

- **Severity:** minor
- **Locations:** `BLUEPRINT.md` 9.2 S2, G4, G6; 9.4 A2, A6, K3.
- **Problem:**
  - S2 holds eight stores, the digest, flush, the lock, the status record, the briefing and the driver.
  - G4 holds the worktree library and delivery (pull request, CI wait, patch id, protected paths, dependency
    detection, adoption), plus `new-fix` and the `gh` double.
  - G6 holds five gates, including the wave gate with mutation, settle and rotation.
  - A2 holds four agents and about 24 method files.
  - A6 holds six agents and about 12 method files.
  - K3 holds five skills, including the preview server application.
- **What breaks:** Review depth drops on the most critical code, the tool, which `pipeline.md` R7 names as the
  critical component.
- **Resolution:** Split them:
  - S2a (stores, digest, flush), S2b (lock and liveness), S2c (status record, briefing, driver);
  - G4a (worktree library and `new-fix`), G4b (`story deliver` and the `gh` double);
  - G6a (`gate wave` and settle), G6b (`release`, `ui`, `formal`, `perf`);
  - A2 into product-owner and story-writer, then architect and ux-designer;
  - A6 into adversary and scenario-reviewer, then the other four;
  - K3 into `preview`, then the rest.

## P2COLD-19: Two resume rows of section 7 rely on undefined behaviour

- **Severity:** minor
- **Locations:** `BLUEPRINT.md` 7 rows "A session ends normally …", "A session crashes" and "A fresh clone on another
  machine"; `state.md` amendment 16 and 2.11 ("owned elsewhere"); `BLUEPRINT.md` 4.9 jobs, 10 item 6 (jobs outlive
  their caller).
- **Problem:**
  - What the survey *does* with a story that is "owned elsewhere" is never defined. On a fresh clone there is no
    local branch and no local run record, so by the definition every story whose branch was pushed is owned
    elsewhere, including the person's own open pull requests.
  - Detached jobs, such as a `story deliver` waiting on CI, survive a session that ends or crashes. Nothing finds
    them, and `--start` does not return the running job for the same subject and gate, so a resumed run starts a
    second one.
- **What breaks:** The fresh-clone resume row cannot continue stories that have open pull requests. Duplicate
  deliveries and gate jobs compete for `parallel_gates` slots and race to merge, protected only by the merge file lock
  and `--match-head-commit`.
- **Resolution:**
  - Define "owned elsewhere" as a `blocked` unit with kind `conflict`, and treat a pushed branch whose open pull
    request was opened by this repository's `vsdd` (a trailer in the body) as adoptable.
  - Make `--start` idempotent per (gate or deliver, subject, commit), returning the live job's id.
  - Have the survey report orphaned jobs and wait on them.

## P2COLD-20: The specification lock and status promotion work only for files that carry `status: approved`

- **Severity:** major
- **Locations:** `state.md` 2.4 (`frozen_when`; the `code_values` list including `active` and `accepted`), 2.5 (domain
  and supplement files have only `title`; subsystem, screen, flow and scenario statuses are `active` or `retired`; ADR
  `proposed`/`accepted`; VP `draft`/`active`/`proven`; `modules.toml` and `dependencies.toml` have no frontmatter),
  2.8 (`settle_decision` "sets `status: approved` on every draft specification artifact"), S7 (`spec_locked` "true
  when the file's status is `approved`"); `hooks.md` 2.11.3 (`artifact_frozen` is keyed on the `spec` decision), list
  of `code_values`; `BLUEPRINT.md` 5.4 (a third list: `approved`, `ready`, `done`, `proven`).
- **Problem:**
  - Most of the approved specification set has no `approved` status, so `spec_locked` is never true for it. That
    covers the domain files, supplements, architecture overview, subsystems, `modules.toml`, `dependencies.toml`,
    conventions, review rules, UX files and the design system.
  - The promotion writes `approved` into kinds whose enumerations have no such value (VP, ADR).
  - Three different `code_values` lists exist. Read globally, they make subsystem, screen, flow and scenario files
    unwritable, because `active` is their only live status.
- **What breaks:** D19's lock does not hold. During build, an architect, for example at arbitration, can edit the
  approved architecture or add a line to `dependencies.toml`, silently avoiding the D15 dependency decision. VPs and
  ADRs never reach `active` or `accepted`.
- **Resolution:**
  - Define the lock by the decision: a file is locked when it is in the digest of the latest approved `spec`
    decision and no live run holds an amendment. Make the hooks wording the definition and change `spec_locked` to
    match.
  - Put a per-kind promotion map in `artifacts.toml` (contract `draft→approved`, VP `draft→active`, ADR
    `proposed→accepted`, brief `draft→approved`).
  - Make `code_values` strictly per kind and delete the global lists in `state.md` 2.4 and 5.4.

## P2COLD-21: Gaps in `[callers]` and `[reveals]`

- **Severity:** minor
- **Locations:** `hooks.md` 2.7 `[callers]`, `[reveals]`, 2.11.1; `BLUEPRINT.md` 4.9 "Who may run it" column.
- **Problem:**
  - `obs`, `export`, `render`, `hooks`, `repos scan`, `import --dry-run` and `--version` belong to no group, so the
    `observe`, `export-pdf`, `author diagram` and `setup` skills may be refused.
  - `doctor` is in `read`, which every builder holds, so any builder can run `doctor --fix` (installs a Python) and
    `--install-shim` (writes `~/.local/bin`).
  - The main row grants the whole `pipeline` group (`survey`, `checkpoint <workflow>`, `story deliver`, `release
    publish`), against the clerk-only column of 4.9.
  - `[reveals] "refs"` lists `holdout`, so every builder is refused `vsdd refs` even though the tool leaves walled
    content out unless `--include-walled` is given.
- **What breaks:** Skills fail, an agent can change the person's machine, the main session can take over a run's lock
  or checkpoint, and builders lose the reference query their methods use.
- **Resolution:**
  - Add an `operator` group granted to the main row only, and move `doctor --fix` and `--install-shim` into it.
  - Key `[reveals]` on `refs --include-walled` and `coverage --include-walled`.
  - Remove `survey`, `checkpoint <workflow>`, `story deliver` and `release publish` from the main row, or state the
    deviation in 4.9.

## P2COLD-22: Undefined configuration keys and unnamed single sources

- **Severity:** minor
- **Locations:** `pipeline.md` 2.6 (`plan` "estimate exceeds the configured ceiling"), 2.9.1 (`docs` kind: "the green
  gate runs the documentation checks"), S1 (`mutation.kill_rate`, `evidence.*`, `maintain.sweeps`, `stops.plan`);
  `state.md` 2.12 (unknown keys are errors; those sections are not shown), `delivery.protected_paths` default;
  `BLUEPRINT.md` D15 ("release configuration" always waits), 4.10 and 4.14 ("One text each"); `pipeline.md` 2.7;
  `agents-skills.md` 2.7 adversary example.
- **Problem:**
  - The plan ceiling key, the documentation-check command and the shapes of `[mutation]`, `[evidence]`, `[stops]`
    and `[maintain]` are named but never specified. Because unknown keys are errors, S1 cannot write the schema.
  - The default `protected_paths` omits the release configuration that D15 lists.
  - The "one text" for the shared agent block and for the severity table has no file. Three versions of the severity
    text already differ (blueprint 4.10: "loses data … exploitable hole"; pipeline 2.7: "loses or corrupts data …
    exploitable security hole … the suite"; the agents-skills example has a third). Byte-for-byte lint cannot pass
    against any one of them.
- **What breaks:** Configuration validation, the D16 automatic plan stop, the D15 default and the reviewer lint.
- **Resolution:**
  - Add the full key table to `state.md` 2.12, including `stops.plan_ceiling_agents` and `commands.docs_check`.
  - Add release configuration globs to `protected_paths` (`[release] version_files`, the changelog, release
    workflow files).
  - Name `workflow-lib/agent-shared.md` and `workflow-lib/severity.md` as the single sources, stamped by
    `tools/build.py` into agent files. Adopt the pipeline 2.7 wording.

## P2COLD-23: Workflow preconditions have no single source

- **Severity:** minor
- **Locations:** `BLUEPRINT.md` 4.9 survey row, 4.14; `pipeline.md` 2.5, 2.9.1 to 2.9.12 (preconditions stated for
  `story`, `wave`, `spec`, `plan` and `release` only), I2 (`stages.toml` fields lack preconditions); `state.md` 2.16.
- **Problem:** The survey is to check preconditions before it locks, but the predicates are prose scattered through
  tables. None is given for `verify`, `feature` (except the baseline), `ingest`, `discover`, `maintain`, `multi` or
  `spike`, and "specification approved" has no meaning across cycles or after an ingest.
- **What breaks:** S3 and C3 builders invent the predicates, and the harness has nothing to test against.
- **Resolution:** Add `[[precondition]]` tables per workflow to `stages.toml` (C3), using named predicates the survey
  implements (`decision_approved(kind, cycle)`, `waves_closed`, `baseline_exists` and so on), with a fixture per
  predicate.

## P2COLD-24: The route table is not scoped to a cycle

- **Severity:** minor
- **Locations:** `pipeline.md` 2.4 route table; `BLUEPRINT.md` 5.1 (two added rows).
- **Problem:** The rows test facts such as "brief approved", "specification approved", "plan ready" and "decision
  `release` approved for the current commit" without naming a cycle. After `release` closes the cycle, the release
  decision is still approved for the same commit, so `next` is `release` again. The queued-request row also sits
  below "Only open decisions remain", so a standing `ideas` decision hides a queued change request.
- **What breaks:** The briefing and the driver point at the wrong workflow after a release, and the driver stops only
  after two runs make no progress.
- **Resolution:** Scope every row except setup and the lock to the open cycle. Place "no cycle is open and a change
  request is queued" directly after the lock rows, and give the conformance fixtures a case for after-release.

## P2COLD-25: What a `merge` decision does is undefined

- **Severity:** minor
- **Locations:** `BLUEPRINT.md` 4.11 (`merge`: `approve`, `reject`), D15; `pipeline.md` 2.6, 2.9.1 (`awaiting_merge`);
  `state.md` 2.8 choice table.
- **Problem:** Nothing says who merges after `approve` (the next `story deliver`, or the person on GitHub), what
  `reject` does to the story and its pull request, or whether an approved dependency is recorded. `dependencies.toml`
  is locked, so the next story using the same dependency asks again.
- **What breaks:** Stories that wait for a person cannot complete through the designed path, and dependency decisions
  repeat.
- **Resolution:**
  - `approve` lets the next `story deliver` perform the governed merge, with the head-commit match.
  - `reject` closes the pull request and blocks the story with kind `conflict` for re-planning.
  - An approved dependency is appended by code to an approved-dependency list in the decision record, which
    dependency detection reads until the next `spec` folds it into `dependencies.toml`.

## P2COLD-26: Methods named by story kinds that do not exist

- **Severity:** minor
- **Locations:** `BLUEPRINT.md` 4.6 (`port`: "test-writer: equivalence tests against the reference"; `feature`: tests
  stage "`stubs`, `failing-tests`"); `agents-skills.md` 2.8 (test-writer methods: `stubs`, `failing-tests`,
  `ui-tests`, `e2e-tests`, `regression-test`), 3.2 row 91 (method `release`, absent from 2.8); `pipeline.md` amendment
  2 (`step()` takes one required `method`); `packaging.md` 2.6 universal test 6.
- **Problem:** The `port` tests stage names no method, and none exists. The `feature` tests stage is one step that
  names two methods. A `release` method is cited but not defined.
- **What breaks:** Universal test 6 fails for port stories, or a builder invents a method. The tests step cannot pass
  one `method`.
- **Resolution:** Add `equivalence-tests` to the test-writer. Merge `stubs` into `failing-tests` (or let `step()` take
  a list). Remove the `release` method from 3.2.

## P2COLD-27: The source of the `step-results` sender table is undefined

- **Severity:** minor
- **Locations:** `hooks.md` amendment 12, 2.11.7 step 2 ("a table generated from the workflows at build time");
  `BLUEPRINT.md` 5.4; `pipeline.md` 2.11 (step names are built at run time, for example `review-${l}-p${pass}`).
- **Problem:** Scripts compute step names. No declared list exists from which the build could generate "allowed agent
  types per step name".
- **What breaks:** The guard that stops a builder forging a review cannot be built as specified.
- **Resolution:** Declare a step-name pattern and the allowed agent types per stage in `stages.toml`. Generate both
  the hook's table and a prelude constant from it, and add a harness test that every `step()` name matches a declared
  pattern with the declared agent type.

## P2COLD-28: The sample project produces no JUnit report

- **Severity:** minor
- **Locations:** `packaging.md` 2.6 Layer 4 (`tally`, "standard-library `unittest` tests"); `BLUEPRINT.md` 4.12
  (`test_report` required); `state.md` 2.12.
- **Problem:** Standard-library `unittest` writes no JUnit XML, and the red gate needs one.
- **What breaks:** E1 to E3 fail at the red gate, or configuration validation fails on the sample.
- **Resolution:** Run the sample's tests with `pytest --junitxml` from a development dependency through `uv`, or ship
  a 30-line standard-library JUnit runner inside the sample.

## P2COLD-29: Decision identifiers are ambiguous when two cycles are open

- **Severity:** minor
- **Locations:** `BLUEPRINT.md` 4.8 (a decision's id is its kind or `<kind>.<subject>`), 4.11; `state.md` 2.7 `others`
  (a critical fix runs as a second cycle), 2.8.
- **Problem:** A hotfix cycle and the main cycle can each hold an open `release` (or `spec`) decision, and
  `/vsdd-factory:decide release approve` does not say which.
- **What breaks:** The wrong release could be authorised, or the command is refused with no way to name the target.
- **Resolution:** When more than one cycle is open, qualify ids as `<kind>@<cycle>`. The briefing prints the
  qualified id, and the `decisions` rule refuses an ambiguous bare id with the list of choices.

## P2COLD-30: Errors in the text of the 3.3 walk-through

- **Severity:** minor
- **Locations:** `BLUEPRINT.md` 3.3 steps 2, 3 and 7; D9; `pipeline.md` 2.9.4 route row (the green gate "fails a
  fast-route story that touches a contract file").
- **Problem:**
  - "Add CSV export" is new behaviour that touches no contract, yet step 3 requires every criterion to cite a
    contract clause. A trivial `feature` must be covered by existing contracts, or it is really `standard`.
  - Step 7 adds the security reviewer only for critical modules, but D9 says critical *and high*.
  - Contract files live on the state branch, so a story's diff can never touch one, and that half of the green-gate
    check is vacuous.
- **What breaks:** Builders and test authors copy a wrong example and a vacuous check.
- **Resolution:** Use an example already covered by a contract (for example "add a `--json` flag to an existing
  report, covered by BC-x"). Say "critical or high" in step 7. Replace the contract-file test with "the story cites a
  contract changed in this cycle".

## P2COLD-31: `setup` cannot move conventions, and `author` has no type for them

- **Severity:** minor
- **Locations:** `BLUEPRINT.md` 4.5 `setup` row ("offers to move build conventions out of the project's `CLAUDE.md`"),
  D6; `agents-skills.md` 2.9 main-session write scope (`brief`, `deferred`, the project), 2.12 `author` types;
  `state.md` 2.5 (lessons "folded into `conventions.md` or `review-rules.md` by the architect").
- **Problem:** `conventions.md` is in `spec-arch`, which the main session may not write, and `author` has no
  `conventions` or `review-rules` type. Lessons also have no route into conventions.
- **What breaks:** The OPS-19 remedy offered at setup is refused by the walls, and adopted lessons never reach agents.
- **Resolution:** Add `author` types `conventions` and `review-rules` (architect, method `architecture`). Have `setup`
  invoke them, and have the `retro` output name `/vsdd-factory:author conventions` for adopted lessons.

## P2COLD-32: Stale passages in the design documents that the amendment lists do not cover

- **Severity:** minor
- **Locations and problems:**
  - `agents-skills.md` 2.6:
    - devops still writes `git` and `gh` operations and returns `worktree_path`, `pr_number` and `merge_sha`;
    - story-writer writes a "wave plan" and returns `waves`;
    - code-reviewer, security-reviewer, ui-reviewer and holdout-evaluator are "given a report path" and write
      reports;
    - the spike runner's verdicts are three, against five in the schema.
  - `agents-skills.md` 2.9 translates the walls into `roles.toml` and `zones.toml` with flags `write_source_before_red`
    and `amend_tests`, against `walls.toml` and the `:before-red` and `:frozen` grants in `hooks.md` 2.7.
  - `state.md` P-5 names `vsdd worktree ensure`, and 2.18 names `.factory/local/cache/research/`.
  - `hooks.md` S7 gives `spike_entry(project, worktree)` without the agent id, and 4.1 item 7 gives the old decide
    grammar.
  - `pipeline.md`:
    - H4 lists the exempt kinds as `facade` and `docs` only;
    - R8 says delivery "rebases and reruns the green gate when the base moved";
    - 2.10.1 `blocked.kind` has no value for a reviewer that refused, although amendment 8 opens `review-blocked`
      "at once" and 2.6 maps unknown kinds to "nothing the first time".
- **What breaks:** A builder who reads the section rather than the amendment list builds the old behaviour.
  Precedence rules do not stop a table from being copied.
- **Resolution:** Correct each passage in place. Add `reviewer_blocked` to `blocked.kind` and map it to
  `review-blocked` in the checkpoint table.

---

## Checked and found sound

- The 20 agent names, their tiers and the tier values (planner Fable, builder Sonnet, reviewer Opus, all at `xhigh`;
  clerk Sonnet at `low`) agree across `BLUEPRINT.md` 4.2 and 4.3, `agents-skills.md` 2.3 to 2.5 and 2.12,
  `pipeline.md` A1, and `packaging.md` amendment 4 and the roster.
- The 13 skills, their invocation modes (one model-visible) and the 12 workflow names are consistent, with no name
  shared between a skill and a workflow. Only `story` is a child, and the nesting rule holds in every script sketch.
- The 17 hook rules are the same set in `hooks.md` 2.10 and the blueprint, and all are delivered by H2 to H9. The four
  policy tables are delivered by C2, and the schemas and stage and route files by C1 and C3.
- All 15 gate names in 4.9 match `pipeline.md` I8 and `state.md` 2.16, and each is assigned to G2, G5, G6 or G7.
- The 20 agents are covered by A1 to A6, the 13 skills by K1 to K3 and the 12 workflows by W1 to W12.
- The state layout, the path-class table (one copy), the identifier schemes, the decision grammar and the
  choice-to-status table agree across the blueprint, `state.md` and `hooks.md`.
- The limits keys and prelude names in 4.12 agree with `state.md` 2.12 and the pipeline limits tables.
- The launch chain (the self-guarding command, mode-dependent failure, exit 10 mapped to 2, no downloads, the inert
  path), the briefing source and size, flush points, the lock's take, scope and stale time, and the hook-liveness
  marker agree across the documents for the single-session case.
- D1, D2, D7, D8, D9 (rule and cap), D10, D11, D12 (the quarantine variable), D13, D14, D17, D18, D20 to D24 and D25
  to D30 are carried into sections 3 to 7 and the designs without contradiction.
- Walk-through steps 5, 8, 9 and 10 and the wave sentence are supported by `pipeline.md` 2.9.1 and 2.9.2 and
  `state.md` 2.16.
- In section 6: steps 3 to 5, the person, main-session, workflow and in-workflow-agent entrances, the time box, no
  nesting, containment and retention are supported by `pipeline.md` 2.8, `hooks.md` 2.11.6 and `state.md` 2.17.
- In section 7: the warm-up, main-session compaction, step-agent compaction, cold start days later, lost status record
  and missing `uv` rows are supported by `state.md` 2.9 and 2.10 and `hooks.md` 2.8, 2.9 and 2.11.8 to 2.11.10.
