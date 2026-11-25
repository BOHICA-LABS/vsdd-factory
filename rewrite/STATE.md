# vsdd-factory 1.0 rewrite — resume record

A session starting cold, or one that has just compacted, reads this file first. It wins over anything
remembered. The full dated history of how the work got here is in `rewrite/LOG.md`; this file holds only
what is needed to continue.

Last updated: 2026-11-25 07:18: step 4 has `gates` and `verdicts` built, `rules` in its last round and `survey-2` being built; steps 5 and 6 are planned and the probe gate passed.

## Resume in one breath

vsdd-factory is being rewritten from scratch as version 1.0 on branch `rewrite/v1`: a native Claude Code
plugin with JavaScript workflows, a standard-library Python tool and hook runner started through `uv`, and
every agent and skill rewritten. The design is in `rewrite/spec/` (one small file per thing; 86 build
packages). Working code is under `next/`. The current milestone is the thin slice: one story delivered on
the sample project through the real platform. Of its 6 steps, steps 1 to 3 are built and closed; step 4 is
being built: of its five units, `gates` and `verdicts` are built and confirmed, `rules` is in its last
round, `survey-2` is being built, and `delivery` follows; step 5 is planned, with its corrections in
`rewrite/plans/step-5/ORDER.md`; step 6 is planned and its probe round passed. Two decisions the user adopted on
2026-11-24 are in the design and are built after the slice: feature flags (D31) and team use (D33).

## In flight when this was written

Two workflows are running in the background of the session that launched them (the same session, after its
second compaction). A workflow keeps running through a compaction of that session and reports back with a
notification. It cannot be resumed from a new session; there, see which outputs exist on disk and relaunch
what is missing from the saved script with the same `args`.

| Workflow | Task id | Script and args | Done when | When it reports |
|---|---|---|---|---|
| Step 4, stage B: `survey-2` (the second part of S3). Launched 07:18 on 2026-11-25, beside the round below (a departure from `ORDER.md`, which has it wait for `rules`; each is told what the other changes) | `w9dxokz40` | `rewrite/workflows/slice-build-unit.js`, `{step: 4, key: "survey-2"}`; title, packages, `concurrent` and `notes` as saved in the session scratchpad, `launch/stage-b-args.json`, and in the launch call of the session transcript. The notes carry what stage A built, `D33-21`, `D33-33`, `S3B-survey-2` and what the story workflow's plan relies on | `rewrite/findings/outcomes/` holds `step4-survey-2.json`, `-fix.json` and `-confirm.json` | Next action 1 |
| Step 4, `rules` (H5, H7): the fourth and last round. It applies decision `ORCH-8` (a rule over command text blocks what it can positively read; tests-first passes what it cannot read, because the green and red gates measure it; merge-guard keeps blocking an unreadable git command where position does not show it harmless) and takes five findings on the path of an ordinary run: the here-document parser bug (blocker), ordinary commands falsely blocked, `git commit -am`, `bash -c`, and a checkout of a guarded branch followed by a commit. Its confirmation is told what counts. Launched 07:18 on 2026-11-25 | `woio9h8nm` | `rewrite/workflows/slice-fix-open.js`, `{name: "step4-rules-fix-4"}`; args saved in the session scratchpad, `launch/rules-fix-4-args.json` | `rewrite/findings/outcomes/` holds `step4-rules-fix-4.json` and `-confirm.json` | Save the result. No fifth round: a blocker or major it leaves that is an ordinary command blocked or a plain direct command passing goes into the `notes` of the `delivery` build as a fix it must not build on, and is fixed by a small round after `survey-2` reports; anything else is triaged to H5 under `ORCH-7` |

When any workflow reports: copy its result to `rewrite/findings/results/<name>.json` (the notification
gives the output file; the result is its `result` key), run `python3 rewrite/tools/findings.py build`, and
read `still_open`.

## Where each piece stands

| Piece | State at 22:30 on 2026-11-24 | Record |
|---|---|---|
| Step 1: foundations | Built | `rewrite/LOG.md` |
| Step 2: catch-up, C2, B6 | Built and closed: 57 of 58 findings confirmed fixed; `layer-2a-fix-3-C1-1` left for B6, B5 and W2 | `rewrite/findings/sources/slice-layer-2a-result.json`, `slice-layer-2a-confirm-result.json`, `layer-2a-fix-3-result.json` |
| Step 3 plans | Seven plans, corrected; `ORDER.md` gives order, owners and cross-unit rules | `rewrite/plans/step-3/` |
| Step 3 `state-core` (S1) | Built, reviewed (30 findings), fixed, confirmed; nothing open | `rewrite/findings/results/step3-state-core.json` |
| Step 3 `hook-runner` (H1, B4b) | Built and closed for the slice. The allow-list for approving a `vsdd` call (`ORCH-2`) holds: 6,150 shell runs of texts it accepts, across ten shell setups, ran only `vsdd`. The sixth round pinned the grammar in the tests with an independent checker and a differential test (of 1,163 changes to the grammar tried in a scratch copy, every ASCII one fails a test) and made the bare `/vsdd` program word accepted, as the spec said. One minor is left for H1: the tests do not pin non-ASCII characters, though the code is right on every code point (`step3-hook-runner-fix-6-C2-1`, with the test to add) | `rewrite/findings/results/step3-hook-runner.json` and `step3-hook-runner-fix-3.json` to `-fix-6.json` |
| Step 3 `jobs` (G1) | Built and closed for the slice: 31 findings fixed over four rounds, `MAX_WAIT` raised to 540. Two minor races between overlapping `env start` calls are left for G1 when the build widens, with the instruction to rewrite that logic as one loop under one rule | `rewrite/findings/results/step3-jobs.json`, `step3-jobs-fix-3.json`, `step3-jobs-fix-4.json` |
| Step 3 `records` (S2a, S2b, S4) | Built and closed: every finding of its review and of its four fix rounds is confirmed fixed, `D33-9` included. Nothing open | `rewrite/findings/results/step3-records.json`, `step3-records-fix-3.json`, `step3-records-fix-4.json` |
| Step 3 `jobs-b` (G1, second part) | Built and confirmed; nothing open | `rewrite/findings/results/step3-jobs-b.json` |
| Step 3 `hook-runner-b` (H1, second part) | Built and confirmed: 17 findings fixed, nothing blocker or major open. One symlink case is left for H4 and the reference class for H5 | `rewrite/findings/results/step3-hook-runner-b.json` |
| Step 3 `survey` (S3, first part) | Built and confirmed: its review raised 19 findings (6 major) and the first confirmation 2 more; all 21 are confirmed fixed. Six older findings it built only for the story path are recorded again as left for the packages that widen them. Its builder's three minors have owners (`S3B-survey-2` goes to `survey-2`). A third round (reported 00:50 on 2026-11-25) fixed the `product=` option of `tests/unit/story_ready.py` (`PL6-7`): it no longer copies the product's `.factory/` and takes `[paths]` from the product's own `factory.toml`; five tests in `tests/unit/test_story_ready.py`; confirmed, nothing open | `rewrite/findings/results/step3-survey.json`, `step3-survey-fix-3.json` |
| Step 3 as a whole | Built. Checked on a quiet tree at 23:40: `tools/build.py --check` clean, 30 of 30 checks, 1,039 tests passed with 1 skipped (`tests/hooks/test_latency.py` among them), 27 workflow harness tests, spec lint clean. Snapshot `rewrite-next-1124-2341.tar.gz` | |
| Step 4 `gates` (G2) | Built and confirmed (reported 02:57 on 2026-11-25): its review raised 21 findings (1 blocker, 11 major) and its first confirmation 3 more; all 24 are confirmed fixed. Nothing blocker or major open. A third round on test support (reported 04:36) made the sample's test runner collect plain test functions and keep its report on an import error, and made `gate_project.py` build on the fixed `story_ready` and assert the project's delivery mode and path classes (`ORCH-5`). Two edge cases of the runner (a module that exits or skips at import) are left for the step 6 `first-run` unit. Two step 3 tests of the survey's provisional story position (`test_position_ladder`, `test_position_reviewed_from_closure_verdict`) fail now that real gates are registered: `survey-2` updates them, as `ORDER.md` section 4 gives it | `rewrite/findings/results/step4-gates.json`, `step4-gates-fix-3.json` |
| Step 4 `verdicts` (G3) | Built and confirmed (reported 03:35 on 2026-11-25): its review raised 16 findings (8 major) and its confirmations 5 more; nothing blocker or major is open. Three minors are left for later packages: the replay's round count on a clone with no local records (`S4R-verdicts-15`, for M3 with S3 and G3), one conformance case and a step-name lint. `acs()` in `story_unit.py` is now the one reader of acceptance criteria; `survey-2` keeps it (`ORDER.md` section 4 has the row) | `rewrite/findings/results/step4-verdicts.json` |
| Step 4 `rules` (H5, H7) | Built: its review raised 27 findings (16 major) and three rounds of confirmation 42 more; 62 are confirmed fixed. `tests-first`, `step-results` and the grounding check have nothing blocker or major open that is not in the last round. The third round (reported 07:05 on 2026-11-25) rebuilt `merge-guard` to pass a git command only on a positive reading, made the hook runner inert outside a factory project (`ORCH-6`) and repaired the twelve step 3 tests; its confirmation ran about 165 commands against real git and left twelve findings, five of them on the path of an ordinary run (in the round above) and seven unusual spellings assigned to H5 | `rewrite/findings/results/step4-rules.json`, `step4-rules-fix-3.json` |
| Step 4 stage A as a whole | Checked on a quiet tree at 07:16 on 2026-11-25, with the five `rules` findings above still open: `tools/build.py --check` clean, 30 of 30 checks, 1,326 tests passed with 1 skipped, 27 workflow harness tests and 26 conformance tests, `tests/hooks/test_latency.py` alone in 1.5 seconds, spec lint clean. Snapshot `rewrite-next-1125-0716.tar.gz` | |
| Step 4 plans | Five plans (`gates`, `verdicts`, `rules`, `survey-2`, `delivery`), corrected twice: for their seam review and for the probe results | `rewrite/plans/step-4/` |
| Step 6 plans | Two plans (`story-workflow` for W2; `first-run` for the `story-ready` fixture and the driver of the first headless run), reviewed by a walk of one story through the plans of steps 3 to 6 and the built code (16 findings: 1 blocker, 4 major), corrected. `ORDER.md` gives the build order, the exact command of the first run, the changes asked of the step 4 and 5 builds (section 8.1), the probe round (8.2), the questions for the person (8.3) and the ledger entries its build records (8.4) | `rewrite/plans/step-6/`, `rewrite/findings/results/step6-plan.json` |
| Step 5 plan | One plan for ten agent files and fifteen method files, corrected for the probe results | `rewrite/plans/step-5/` |
| Platform probes, round 4 | Run, independently checked, and applied to the platform record and to the plans | `rewrite/spike/probes-round-4/CHECK.md`, `rewrite/findings/results/probes-round-4.json`, `probes-4-apply.json` |
| Platform probes, round 5 | Run and independently checked (the checker reran the main leg whole). **The gate before the first run passed in its primary form**, on Claude Code 2.1.332 in the default permission mode: the plugin's hooks, workflows and `bin/` load with the user's settings kept out; the typed workflow command starts the shipped script with no skill; bare tool names admit a step's read of the plugin's files and its write and commit in a story worktree; a step's hook payload carries the effort it was dispatched with; items 36 to 38 and 40 still hold. Cost about 35 cents plus the check's reruns. The check corrected the prober on leg B (a budget stop mid-workflow kills the workflow and loses the running step; the cap is soft) and on wording; a correction round applied the check to the platform record and to the step 6 plans (18 findings fixed, 3 minors from its confirmation fixed by the orchestrating session) | `rewrite/spike/probes-round-5/CHECK.md`, `rewrite/findings/results/probes-round-5.json`, `probes-5-apply.json` |
| Feature flags (D31) | In the design and closed for now: 62 findings fixed, 12 left for the packages F1 to F6 that build it. No further round of design fixes | `rewrite/findings/sources/d31-feature-flags-result.json`, `rewrite/findings/results/d31-fix-2.json` |
| Team use (D33) | In the design and closed for now: `rewrite/spec/topics/team-use.md` explains it and links the rest; packages M1 to M6. Of 105 ledger entries, 70 are fixed and 35 are left for the packages that build it, each with a resolution. The second correction round (reported 23:30) settled which refs a fetch and a push name (T-REMOTE in `tables/sync-checks.toml`), after running each git claim on git 2.34.1 and 2.50.1. No further round of design fixes. Of its entries for the slice units, `D33-9`, `D33-13` and `D33-25` are built; `D33-21` is built for the survey and goes to `delivery`; `D33-33` (a missing `+` in the built `target_head` refspec, pr mode only) goes to `survey-2` | `rewrite/findings/results/d33-team-use.json`, `d33-fix-2.json` |
| Decision D32 | Written into the design | `rewrite/spec/decisions-later.md` |

Readings at 07:16 on 2026-11-25, on a quiet tree before `survey-2` started: the checks as in the row "Step 4
stage A as a whole". Ledger: 908 findings, 691 fixed, 140 left for a named package, 4 rejected, 71 open (2
blockers: the old `P3SEAM-01` outside the slice, and the here-document parser bug that the `rules` round is
fixing), and every open one has an owner. Latest snapshot: `rewrite-next-1125-0716.tar.gz`.

## Next actions, in order

1. **When `survey-2` reports:** save its result (`rewrite/findings/results/step4-survey-2.json`, as
   `{logs, result}`), rebuild the ledger, give its builder's own findings (`S4B-survey-2-<n>`) owners, and
   send anything blocker or major still open to a `slice-fix-open` round (two passes; a further round only
   for a blocker or major; otherwise triage to the owning package as left-for-builder with a concrete
   resolution, in an outcome file and a triage file named `orchestrator-<unit>.json`). Record the outcomes
   the plans claim for older findings (`rewrite/plans/step-4/ORDER.md` section 7, "Ledger"). When it and
   the `rules` round have both reported with nothing blocker or major open on the path of an ordinary run:
   run every check on a quiet machine (`tools/build.py`, `tools/check.py`, the whole pytest suite, both
   node suites, `tools/build.py --check`; `tests/hooks/test_latency.py` again by itself), take a snapshot,
   and launch two builds at the same time, each named in the other's `concurrent`: stage C, `delivery`
   (G4a; `D33-21`'s delivery half in its notes), and step 5, `agents` (next action 5). Keys, titles and
   packages: `rewrite/plans/step-4/ORDER.md` section 1.
2. (Done at 01:44 on 2026-11-25: probe round 5 is applied. The platform record, items 50 to 55 in
   `rewrite/spec/topics/platform-proven-3.md`, says what the evidence shows, and the step 6 plans carry the
   answers. Its evidence extract is in `rewrite/spike/probes-round-5/step-transcripts/`. Only `PR5-1` stays
   open, for the `first-run` unit to close.)
3. (Done at 23:46: the 24 older open findings with no owner are settled. Ten are fixed or were already
   fixed, two are rejected as history, and twelve have owners, in `orchestrator-old.json` under `outcomes/`
   and `triage/`. No open finding is without an owner.)
4. (Step 4's last stage is in action 1.)
5. **Step 5 build** from `rewrite/plans/step-5/agents.md` (`args: {step: 5, key: "agents", ...}`), after
   step 4. Its `notes` carry the corrections of `rewrite/plans/step-4/ORDER.md` section 7 (PL4-23) and
   `probes-4-apply-C2-3` (the sentence about re-running a command moved to the background applies only to
   job commands). It may start beside step 4's later stages if waiting would hold up step 6.
   Also in its `notes`: `L2-c2-b6-fix-1` (a tools line that holds an MCP tool name is written quoted).
   And rows 4 to 7 of `rewrite/plans/step-6/ORDER.md` section 8.1, which block the W2 build: the
   technical-writer, devops and verifier each list `fix-findings`, with its new head and its sentence for a
   kind with no red gate; `review-story-diff` takes a finding's owner from the task when the task names one;
   the clerk file carries the two background sentences word for word; universal tests 3 and 6 never name
   `toy-steps`, and the toy uses the closings `story.js` uses. All of these, with the step 4 corrections,
   are written out in `rewrite/plans/step-5/ORDER.md` section 3, which holds over the plan; the launch
   `notes` only point at it and say what step 4 built. Launch it when `survey-2` has reported, beside
   step 4's last unit `delivery`, and name each in the other's `concurrent`. Not earlier: its own test
   needs every `vsdd` subcommand its files name to be registered (`story deliver` comes with `delivery`),
   it changes `tools/build.py` and `tools/check.py`, which `survey-2` runs, and a half-filled `agents/`
   folder fails a harness test (`S5P-N6`).
6. **Step 6 build**, when steps 4 and 5 have reported and every check passes on a quiet machine: take a
   snapshot and launch `story-workflow` (W2) and `first-run` (E1's fixture part, E2's first run) at the
   same time with `slice-build-unit.js` (`step: 6`; `rewrite/plans/step-6/ORDER.md` gives keys, owners and
   cross-unit rules). Their `notes` carry the answers of probe round 5 and three older findings:
   `L2-catchup-fix-1` (type and require the members of `survey.config` a script reads), `PR3-1` (the
   driver's project lives outside Claude Code's temporary tree) and `VSA-15` (the values the loader writes
   with and without `--local`). The `first-run` notes also carry two edge cases of the sample's test runner
   (`step4-gates-fix-3-C2-1` and `-C2-2`: a test module that exits or skips at import). After both report: every check (the node line becomes
   `node --test tests/workflows/*.test.mjs`), then the loader once as a model-free smoke check. Then the
   first run, cheap tier: `cd next && uv run --no-sync python -B tests/e2e/run.py story`, from a shell
   outside the Bash sandbox (if this session cannot get one, the user runs that one command). It is
   capped at 6 dollars a session and 12 over all attempts. The roster tier (capped at 30 and 60 dollars)
   is run only after the user says so. Fix what the run finds, in the code and in the spec. When the
   `gates` unit of step 4 gets a fix round, add row 2 of section 8.1 (`gate_project.py` asserts
   `delivery.mode`, `paths.tests` and `paths.source` after building its project).
7. Then widen to all 86 packages (the 74 of the blueprint, F1 to F6 for feature flags in the order F1, F5,
   F2, F6, F3, F4, and M1 to M6 for team use with M6 first), run the greenfield and brownfield proofs (E3,
   E4) and the two-clone proof, write the documents, build the release tooling and cut over.

The authoritative order for the slice is the section "The thin slice" in
`rewrite/spec/packaging/build-plan.md`.

## Standing rules from the user

- **Models:** Fable at extra-high effort plans. Sonnet at extra-high effort writes code. Opus at extra-high
  effort reviews. Set model and effort explicitly on every workflow step.
- **Workflows:** always use a workflow when one applies. This is a standing opt-in; do not ask first.
  The session guideline is under ten agents per workflow, so run several workflows instead of one large one.
- **Git:** commit or push only when asked. No AI attribution in commit messages, and that includes the
  session trailer the harness suggests. Never `--no-verify`. Never force-push `main`.
- **Scope:** full rewrite as 1.0; every capability of the old system ports; macOS and Linux only; Python
  through `uv`, standard library only at run time; as low touch as possible; research spikes reachable by a
  person, an agent, a skill and a workflow; brownfield (existing code, and features after a finished build).
- **Feature flags (decision D31, adopted 2026-11-24 with the word "adopt"):** the factory treats feature
  flags as a first-class concern of the software it builds, not of itself. Both halves are in scope: the
  product gets a sound flag system, and features are delivered through it. It is opt-in per project. The
  factory uses an established flag system and the person confirms which one, in this order: the one already
  in the code, the one the person or brief names, the one the architect recommends after research, and
  rolling our own only as a last resort with explicit approval and a recorded reason. The user's words:
  "it should probably also confirm which feature flag system we are using (launchdarkly, unleash, etc, etc,
  etc), rolling our own should be a last resort." Code reaches flags through one interface, OpenFeature where
  the language supports it. The factory writes flag definitions into the repository and never calls the
  outside service; keys stay out of the repository; tests never reach the live service. Operating flags in
  production is out of scope. No new workflow. Built when the build widens past the thin slice. The nine
  clauses as mirrored to the user are in the `DECISION` text of `rewrite/workflows/d31-feature-flags.js`.
- **Team use (decision D33, adopted 2026-11-24 after the user answered five questions).** Several people,
  each with their own machine, clone and session, run the factory against one shared repository like a
  normal development team, and the result must be as sound as with one person. The Git remote is the only
  coordination channel: no server. The user's words on hand-off between machines: "this should be bound
  within the physics achievable with git"; the assistant reads that as coordination and hand-off resting on
  fetch, push and the atomic update of a ref, with work picked up from what has been pushed, and told the
  user so. The factory records who did what and does not enforce who may approve, merge or release: "a
  properly setup repo should enforce this". It must cope with teammates who commit by hand without the
  plugin. Designed now, so that the state code does not bake in one machine; the thin slice stays one
  story on one machine; built when the build widens, with a two-clone test as proof. The eight clauses are
  the `decision` argument of the D33 launch, saved in `rewrite/spec/decisions-later.md` in short and in the
  session transcript in full.
- **The user asks for a mirror before a new capability is designed:** restate what was asked, say what the
  design has today, ask the questions whose answers change the design, and start only after the answer.
- **Findings:** everything found by review or by building gets a recorded outcome. Nothing is dropped.
- **Resume:** the rewrite and the product must both survive compaction and a cold start.
- The user adopted every recommendation in the blueprint's decisions D1 to D30 (`rewrite/spec/decisions.md`),
  and D32: the hook runner approves an agent's own single `vsdd` call, to attach the caller marker, whatever
  `hooks.rules.auto-allow.scope` says.

## Commits

Two local commits exist on `rewrite/v1`, both approved by the user, neither pushed:
`2a23827b` (the first snapshot of `rewrite/`) and `486b6835` (`next/` foundations and the rewrite-mode root
`CLAUDE.md`). Everything since is uncommitted: about 180 changed or new paths under `rewrite/` and `next/`.

Because so much is uncommitted while several agents write to the tree, snapshot archives of `rewrite/`,
`next/` (without the virtual environment) and the root `CLAUDE.md` are saved in
`/private/tmp/claude-501/-Users-jmagady-dev2-vsdd-factory/f468fc16-2190-4e04-8bbb-50654bb31906/scratchpad/snapshots/`,
named `rewrite-next-<month day>-<hour minute>.tar.gz`. The latest is `rewrite-next-1125-0716.tar.gz`. They
are a safety net against an agent's mistake only: a reboot can clear that folder. Take a new one before each
stage of builds:

```
SNAP=/private/tmp/claude-501/-Users-jmagady-dev2-vsdd-factory/f468fc16-2190-4e04-8bbb-50654bb31906/scratchpad/snapshots
tar czf "$SNAP/rewrite-next-$(date +%m%d-%H%M).tar.gz" --exclude=.venv --exclude=__pycache__ --exclude=.pytest_cache --exclude=node_modules rewrite next CLAUDE.md
```

Every builder is told never to use git to discard or restore files.

## Waiting on the user

Nothing here blocks the next actions. Each default stands until the user says otherwise.

1. **Local commits.** May the assistant commit locally at each step boundary? Asked several times, not yet
   answered, so no further commit has been made. A yes would protect the work and let builds run in
   isolated worktrees.
2. **The Bash sandbox.** With Claude Code's Bash sandbox on, every write under `.git/worktrees/` is refused,
   so the factory cannot commit or create a story worktree. Working assumption, recommended by the
   assistant: the factory requires the sandbox off for the sessions that run it, `doctor` and the first
   survey say so loudly, and a later probe looks for an allowance that works. The first story run is made
   with the sandbox off.
3. **Team use (D33), three questions.** Should each person who joins be asked for their own machine which
   tool calls the plugin approves by itself (D4 was answered once per project)? The assistant recommends
   yes. Is a team on a Git host other than the one whose command-line tool pull-request mode uses in scope
   for 1.0? The assistant recommends no, with the host operations behind one module (`D33-18`). Does cutover
   wait for the two-clone proof? As designed it does not; the assistant recommends making it a gate.
4. **Team use, six limits the design chose:** with team use on, no run starts without a successful fetch (no
   offline runs); team use needs pull-request delivery with CI required; it cannot be turned on in a
   multi-repository hub yet; a fork-only workflow is not supported; the state branch must accept direct,
   unsigned pushes; git 2.34 or later.
5. **Feature flags, four defaults:** `flags.require` defaults to `high-risk` (the assistant asked the user to
   confirm this one; `existing` would flag every feature story on an existing product); `flags.removal`
   defaults to `human`; F1 to F6 are built before the proofs, so they are on the path to cutover; the
   flagged-story run is not added to CI.
6. **Step 4, four defaults for the slice:** until governed delivery (G4b) is built, local delivery refuses a
   story that touches a dependency manifest or `CHANGELOG.md`; a project with `delivery.merge = "human"`
   cannot deliver until G4b; after a local merge the story branch is deleted, so with a squash merge the
   story's individual commits become unreachable (the assistant asked the user to confirm this one); a
   decision is settled with `vsdd decide` in a terminal until the `decide` skill is built.
7. Not blocking: the blueprint's offer (section 11.2) to replace the lock heartbeat with liveness by process
   is still open; the heartbeat is built. The assistant changed the plan for old findings: they are closed
   in code as each package is built. The user was told and has not objected.

8. **Step 6, four questions** (`rewrite/plans/step-6/ORDER.md` section 8.3), each with the default that
   stands: (a) may `spikes.per_run = 0` become a valid setting meaning "this project runs no spikes"?
   Default: no change; `story.js` sets its own spike limit to zero until the spike packages are built.
   (b) Should a delivered story keep a durable record of its red and green commits (in the closure, or a
   kept ref), since a squash merge makes them unreachable? Default: no; this extends item 6. (c) Should
   the product refuse, by a hook rule, a `Workflow` call that carries an inline script, so that an
   interactive run is known to be the shipped script? It is a new capability, to be mirrored first.
   Default: no rule. (d) Spending on the real platform: the probe round costs under a dollar and the
   first run at the cheap tier one to three dollars; the assistant runs both. The roster-tier run (six to
   fifteen dollars expected) waits for the user's word.

9. **A guard at the ref for the integration branch** (`ORCH-7`). `merge-guard` reads the text of a tool
   call. However strictly it reads, it cannot see git run by a script, by make or by an interpreter, so it
   is not a guarantee that only delivery moves the integration branch. Git's `reference-transaction` hook,
   installed by `vsdd init`, could refuse any move of that branch unless the tool makes it. That is a new
   mechanism in the product's repository (it must leave a person's own terminal alone, live with a project
   that sets its own hooks path, and needs git 2.28), so it is mirrored to the user before anyone designs
   it. Default: no such guard; the text rule, delivery's own checks, and the listing of hand commits that
   D33 designs for the wave and release gates.

Decisions the assistant made and told the user, recorded in the ledger:

- **`ORCH-2`: a `vsdd` call is approved by an allow-list, not by parsing the shell.** The hook runner
  approves an agent's own `vsdd` call. It did so when a shell parser found nothing dangerous in the command,
  and four review rounds each found a new construct that slipped through. Approval now also needs a match of
  the whole raw command text: `vsdd`, then arguments that are bare words of letters, digits and
  `_ @ % + = : , . / -`, or single-quoted strings holding no quote and no control character. Anything else
  passes unmarked and unapproved. The same rule is recorded for the auto-allow rule of package H6
  (`ORCH-3`).
- **`merge-guard` passes a git command only when it can read the whole of it** (told to the user on
  2026-11-25). Three review passes each found new spellings that slipped through on the integration branch
  (`env -S`, an expansion in git's own arguments, an exported `GIT_DIR`, a wrapper not in the table). The
  rule now blocks a git-bearing command it cannot read, unless position alone shows it harmless (a story
  worktree, no other branch or directory named). The same change of approach as `ORCH-2`.
- **A rule over command text blocks what it can positively read; the guarantee is measured (`ORCH-8`, told
  to the user on 2026-11-25).** Treating every unreadable command as the violation it could be blocked
  ordinary work (`grep -rn rm src`, `git mv`, a commit message in a here-document) and still never became
  complete. tests-first now passes what it cannot read: the green gate fails on any frozen test whose
  content changed since it was frozen, and the red gate fails if the new tests do not fail. merge-guard
  has no measured backstop until `ORCH-7` is decided, so it still blocks an unreadable git command where
  position does not show it harmless; seven findings about unusual spellings are left for H5.
- **`survey-2` was launched beside the last `rules` round**, against the planned order, to save two to
  three hours: that round touches only the shell reader and the two rules, not the fixtures `survey-2`
  builds on, and each run is told what the other changes.
- **A fix round runs two passes; after that only a blocker or major gets another round.** A minor that is
  left goes to the package that must settle it, with a concrete resolution.

## How the work is done

- **The design** is `rewrite/spec/`. `rewrite/spec/README.md` gives the layout: one file per thing, a
  registry, links written `[[kind:id]]`, files under 16 KB. Each build package has a generated brief in
  `rewrite/spec/packages/<ID>.md`. The spec was converted from six larger documents kept in
  `rewrite/history/design-v1/`; a builder reads the original section when a spec file is silent. When a
  package is built, its spec files shrink to pointers at the code.
- **Adding to the design.** A new build package is registered in a `rewrite/spec/registry.d/` file as
  `[package.<ID>]` with `summary`, `delivers`, `depends` and `source`, and
  `python3 rewrite/tools/gen_packages.py` writes its brief. Decisions D1 to D30 are in
  `rewrite/spec/decisions.md` and D31 onwards in `rewrite/spec/decisions-later.md`; a file that comes from
  a later decision takes `sources: [decisions.md#D<n>]`.
- **The ledger** is `rewrite/findings/`. `python3 rewrite/tools/findings.py build` reads the raw findings in
  `sources/`, the owners in `triage/` and the outcomes in `outcomes/` (applied in the order listed in
  `outcomes/ORDER`, so a later file wins), and writes `ledger.json` and one list per package in
  `by-package/`. Whole workflow results are saved in `results/`.
- **The workflow scripts** are in `rewrite/workflows/` and are launched with the Workflow tool's `scriptPath`
  and `args`. The ones to reuse:
  - `slice-build-unit.js`, `{step, key, title, packages, concurrent, notes}`: one unit from its plan. Sonnet
    builds, Opus reviews, Sonnet fixes every finding, Opus confirms, with a second fix and confirmation if
    needed. It records everything in the ledger under the name `step<step>-<key>`.
  - `slice-fix-open.js`, `{name, about, others, findings}`: code findings an earlier round left open.
    Sonnet fixes, Opus confirms, twice at most.
  - `spec-fix-open.js`, `{name, about, findings, may_write, guidance, forbid}`: the same for the design and
    the plans, with Fable correcting and Opus confirming.
  - `design-decision.js`, `{id, slug, decision, read_first, must_answer, soundness_lens, package_prefix,
    extra}`: writes an adopted decision into the design. Fable designs, two Opus reviewers (fit; soundness
    in practice), Fable integrates, Opus confirms.
  - `slice-step-4-plan.js` is the model for a planning run (five Fable planners, one Opus seam review, one
    Fable integrator that writes `ORDER.md` and `RESOLUTIONS.md`); `slice-probes-4.js` for a probe round
    (Sonnet probes, Opus checks the evidence and reruns what matters).
- **Before a stage of builds:** every check on a quiet tree, then a snapshot. **In a launch:** `concurrent`
  names what else is writing to the tree, and `notes` carries what was learned after the plan was written;
  the note says it holds over the plan.
- **Checks under `next/`:** `cd next && uv run --no-sync python tools/build.py --check`, then
  `uv run --no-sync python tools/check.py`, then `uv run --no-sync python -m pytest -q` (about three
  minutes), then `node --test tests/workflows/universal.test.mjs`. If `uv` cannot reach the package index,
  add `--system-certs`. A `__pycache__` folder under `next/plugins/` makes two checks fail; it comes from an
  agent that imported the plugin with plain `python3`, and deleting it is safe.
- **Checks on the spec:** `python3 rewrite/tools/spec_lint.py`, `python3 rewrite/tools/sections.py coverage`
  and `python3 rewrite/tools/gen_packages.py`.
- **Platform probes** use the throwaway plugin in `rewrite/spike/plugin/`, a scratch project outside the
  repository, and headless `claude -p --plugin-dir`. Results are in `rewrite/spec/platform.md`,
  `rewrite/spec/topics/platform-proven.md` and `platform-proven-2.md`; evidence for the last round is in
  `rewrite/spike/probes-round-4/`.

## What has been learned that changes how to work

- Fixing prose does not converge; building with a reviewer and tests behind each unit does. A design round
  gets one correction pass and one more for what its confirmation finds; the rest goes to the packages.
- A reviewer that reruns the checks and breaks the code in a scratch copy finds what tests miss. Every
  step 3 unit passed its own tests when built, and every review found majors.
- A fix often opens the next gap. An independent confirmation after every fix is what catches it; two of the
  step 3 majors were introduced by fixes.
- When rounds keep finding new cases of one kind, change the approach, do not patch again: an allow-list in
  place of a deny-list for the approved command; one loop under one rule for a state machine; a differential
  test against an independent checker in place of hand-picked cases.
- Probe a platform assumption before building on it. Round 4 cost under two dollars, confirmed the verdict
  design could work, and found three things no plan knew (the sandbox, the ten-minute stop of headless
  background work, which git hooks a merge runs). The prober misread five of eleven results; the independent
  check caught each.
- Units built at the same time meet at their seams. Give each builder what the other is changing, in
  `notes`, and add a test that crosses both.
- An agent with an explicit tool list starts with about 3,000 tokens of context; one without starts with
  about 19,000. A workflow step cannot spawn agents; a workflow may call another one level deep only; a
  script cannot read files or run commands. A missing hook launcher does not block anything, so the launch
  command guards itself.
- The assistant's own shell runs sandboxed: it cannot reach the package index without `--system-certs`, and
  a project `CLAUDE.md` is not loaded for a project under `/private/tmp`.

## Where things are

| Path | Contents |
|---|---|
| `rewrite/STATE.md` | This record |
| `rewrite/LOG.md` | The dated history, in two parts: up to the first compaction, and what reported on 2026-11-24 |
| `rewrite/WARMUP.md` | The prompt that reloads a session after compaction or from cold |
| `rewrite/spec/` | The design; `README.md`, `conventions.md`, `decisions.md`, `decisions-later.md`, `platform.md`, `packaging/build-plan.md` are the places to start |
| `rewrite/plans/step-<n>/` | Build plans, one per unit, with `ORDER.md` (order, owners, cross-unit rules), `REVIEW.md` and `RESOLUTIONS.md` |
| `rewrite/findings/` | The ledger: `sources/`, `triage/`, `outcomes/`, `results/`, `ledger.json`, `by-package/` |
| `rewrite/workflows/` | Every workflow script run so far |
| `rewrite/tools/` | `spec_lint.py`, `sections.py`, `gen_packages.py`, `findings.py` |
| `rewrite/history/design-v1/` | The six original design documents, unchanged |
| `rewrite/spike/` | The probe plugin and the evidence of each probe round |
| `next/` | The product being built; the plugin is `next/plugins/vsdd-factory/` |
| `plugins/`, `crates/` | The old system: read for reference, never edit |

## Keeping this record current

Update this file at the end of every step and before ending or compacting a session: what is in flight, the
next actions, the commit list, and what is waiting on the user. Put history in `rewrite/LOG.md`, not here.
Workflow results are written to files under `rewrite/` before a step is marked done, because workflow output
does not survive a session.
