# Step 4: build order, concurrency and file ownership

Step 4 of the thin slice is G2 and G3, the second part of S3, G4a, and H5 with H7
(`rewrite/spec/packaging/build-plan.md`, "The thin slice"). It is built as five units, each from one plan
in this directory, each by one run of `rewrite/workflows/slice-build-unit.js`. The plans were corrected
against `REVIEW.md` (findings PL4-1 to PL4-23); where a plan and the review differ, the plan holds. What
was decided for each finding is in `RESOLUTIONS.md`.

Every builder and reviewer reads this file. In the five plans `ORDER.md` means this file; the step 3
file is written "step 3's `ORDER.md`". Paths are under `next/`; `P` is `plugins/vsdd-factory`.

## 1. The units

| Key (`args.key`) | Plan | Packages (`args.packages`) | Title (`args.title`) |
|---|---|---|---|
| `verdicts` | `verdicts.md` | G3 | Verdicts: the convergence rule in Python, the verdict from stored passes, cover, conformance cases |
| `gates` | `gates.md` | G2 | The story gates: red, re-freeze, green, demo, the diff patch |
| `rules` | `rules.md` | H5, H7 | Rules: tests-first, merge-guard, step-results, and the grounding check |
| `survey-2` | `survey-2.md` | S3 (second part) | Story positions, measuring a gate again, the checkpoint mapping, decisions |
| `delivery` | `delivery.md` | G4a | `vsdd story deliver` in local mode |

Launch a unit with `args: {step: 4, key, title, packages, concurrent}`, where `concurrent` lists the keys
in flight at the same time, as section 2 gives them.

## 2. Order

| Stage | Units, at the same time | A unit may start when |
|---|---|---|
| 0 | none | All seven step 3 units have reported with nothing blocker or major open, requests R1 and R2 of section 5 are in their builds, and every check below passes on a quiet machine |
| A | `verdicts`, `gates`, `rules` | Stage 0 holds |
| B | `survey-2` | `verdicts`, `gates` and `rules` have all reported |
| C | `delivery` | `survey-2` has reported |

"Reported" is step 3's meaning: the whole run returned (build, review, fixes, confirmation) with no
blocker or major finding open. A unit that is not reported is not a base to build on.

What forces the order:

1. Step 3 before everything: every unit starts from `story_ready.py`, the stores and the dispatcher.
2. `verdicts` and `gates` before `survey-2`: the survey calls G2's registered gate functions and G3's
   `verdict`, `summary`, `waivers`, `rounds_used` and `public_verdict`, and its real-gate tests use G2's
   `gate_project.py`.
3. `rules` before `survey-2`: `tests/hooks/rule_projects.py` runs the real survey in process, and
   `survey-2` rewrites `survey.py`, `story_unit.py` and `checkpoint.py`. Beside each other, the rules
   tests would break and pass on the other builder's half-made edits (PL4-10).
4. `gates` before `survey-2` for a second reason: both change a schema and regenerate the prelude.
5. `survey-2` before `delivery`: delivery's fixtures run the real survey and `story_unit.build`. It also
   calls G2's `demo` function, `dirty_files` and `write_patch`, and G3's `covering`, `verdict`, `summary`
   and `changed_files`.

Inside stage A the three units own disjoint files. Two things join them, and both are in the plans:

- `gates` imports `vsdd.state.verdicts.changed_files` in its green gate (its step 5). `verdicts` builds
  that function as its first step. If it is not there when `gates` reaches step 5, the gates builder
  builds `gate_demo.py` first and comes back; it never writes a copy.
- `gates` (two schemas) and `rules` (three registry rows) both run `tools/build.py`: rule 3 of section 3.

`verdicts` makes its one edit in records' `gate_evidence.py` (section 4) as a single write, in its
step 5, after its own tests pass.

The checks, run by every unit before it reports and once more by the orchestrator on a quiet machine
after each stage: `uv run --no-sync python tools/build.py`, then `tools/check.py`, the whole pytest
suite, the node harness (`tests/workflows/universal.test.mjs`), and `tools/build.py --check`. Take a
snapshot of the tree before each stage. `tests/hooks/test_latency.py` is sensitive to load, as in step
3; after `rules` reports it is run again on a quiet machine, because the four edit tools now start the
launcher too.

## 3. Rules that cross units

Step 3's rules (step 3's `ORDER.md`, section 4) all still hold: a command is a `cmd_<family>[_<x>].py`
module, nobody edits `cli.py` or `vsdd/state/__init__.py`, the caller is read with `callers.caller`, a
person is checked with `inv.require_person`, git goes through `gitrun.git`. These are added.

1. **One definition each.** A second copy is a defect, whoever writes it.

   | Thing | The one definition | Owner | Callers |
   |---|---|---|---|
   | Grounding a finding | `vsdd.grounding.ground(finding, *, tree, project_root)`, tested in `tests/unit/test_grounding.py` | `rules` | the `step-results` rule only; the verdict trusts the stored mark |
   | The verdict, "no pass is stored", cover, waivers, rounds used | `vsdd.state.verdicts`: `verdict` (always an object), `summary` (`None` when no pass is stored), `covering`, `waivers`, `rounds_used` | `verdicts` | `survey-2`, `delivery` |
   | "The files changed" | `vsdd.state.verdicts.changed_files(project, since, head)`, with `--no-renames` | `verdicts` | `gates` (`--since`), `delivery` |
   | A patch file | `vsdd.gates.diffpatch.write_patch(project, subject, base, head, *, tree, since="")`: `diff.<head12>.patch` for base to head, `diff.<since12>-<head12>.patch` for a range | `gates` | green, `delivery` |
   | The clean-worktree rule | `vsdd.gates.gatelib.dirty_files(tree, config)` | `gates` | the gates, `delivery` |
   | "The demo gate holds" | `passed` of the registered `demo` function, for every story | `gates` | `delivery`, `survey-2` |
   | The review lookup | `evidence(project, "review", subject)` is `verdicts.covering(project, subject)` | `verdicts` (section 4) | `survey-2`'s ladder |
   | A story's stage names | `story_unit.STAGES` | `survey-2` | the survey, the checkpoint |
   | The last delivery state | the story closure's `delivery.state`, `run`, `detail`, `needs_person`, `at` | `delivery` writes | the checkpoint reads |

2. **The gate registry contract.** A registered function is
   `fn(project, subject, run, *, wait=None, retry="") -> dict`. Red and green attach to the gate
   command's own job and wait at most `wait` seconds (`None` is 100); `demo` is direct. The survey
   always passes the seconds it has left and its own retry line. Records only stores the callable.
3. **Generated files.** `workflow-lib/prelude.js`, the stamped regions of `tests/workflows/toys/*.js`
   and `P/hooks/hooks.json` are a function of the schema files and the hook registry. A unit that
   changes a schema or a registry row runs `tools/build.py` in the same step, and every unit runs it
   once more immediately before its final checks. In stage A, a `--check` drift or a `build_is_current`
   failure that names a file whose source another unit in flight changed is not a finding: run the
   build and go on. Nobody edits a generated file by hand.
4. **A result made for another run is not served.** A job key holds no run. Gates and delivery put the
   run into the worker's result, remove it before printing, and attach again when it is not theirs.
5. **Test projects.** `story_ready.py` (step 3) builds the project; `gate_project.py` (`gates`) is the
   one builder with the tally sample in the story's base; `story_progress.py` (`survey-2`),
   `rule_projects.py` (`rules`) and `delivery_fixtures.py` (`delivery`) build on those. Nobody writes
   record JSON by hand.
6. **No step 4 unit edits another unit's file** except as section 4 lists. A builder that finds a step
   3 interface missing stops and reports; the orchestrator has the owning step 3 unit fix it. It is
   never worked around with a second copy.
7. **Git the tool runs: the Bash sandbox and the project's hooks** (platform items 41 and 45,
   `rewrite/spec/topics/platform-proven-2.md`). A git failure is reported with its cause, in words a
   test asserts, and nothing is undone, so the next run finishes.
   - **The sandbox.** Step 3's `ORDER.md`, rule 11, holds here: git output with `Operation not
     permitted` is reported with the word `sandbox`, git's own line and the fix. No step 4 unit
     catches and rewords it. A gate's worker lets `WorktreeError("not_permitted")` out, so the job
     layer's error line carries its message; the survey passes a flush's error on; delivery answers
     `refused` with it (delivery.md D7).
   - **A project's git hooks.** A commit the tool makes in a linked worktree runs the project's
     `pre-commit` and `commit-msg` hooks with cwd = that worktree and that worktree's own `GIT_DIR`.
     In `.worktrees/_integration` they see tracked files only, so a hook that needs `.env`,
     `node_modules`, a virtualenv or a generated file fails there. When git refuses such a commit and
     the sandbox is not the cause, the error names `git hook <names>`: those of `pre-commit`,
     `prepare-commit-msg` and `commit-msg` that are executable files in the worktree's hooks directory
     (`git rev-parse --git-path hooks`), or plain `git` when there is none. `pre-merge-commit` never
     runs: the tool makes no automatic merge commit (delivery.md D6).

   Only delivery commits in a linked worktree today. A later unit that does (G5, G6) follows this rule.

## 4. Edits of other units' files (PL4-21, PL4-2)

The step 4 brief says never to change a step 3 file. These are the exceptions, each named by an
earlier plan or needed for a field no earlier package could have known. No two units edit the same
file, and nothing outside this table is edited.

| Unit | File | What changes | Sanctioned by |
|---|---|---|---|
| `verdicts` | `P/py/vsdd/state/gate_evidence.py` (records) | Only the `review` branch of `evidence`: it becomes `from vsdd.state.verdicts import covering; return covering(project, subject)`, imported inside the function, with no `ImportError` fallback; the closure-only code it replaces goes. The review case of records' evidence-lookup test is updated if it no longer holds | Step 3's `ORDER.md` section 7 gives G3 this job; PL4-2 |
| `verdicts` | `P/py/vsdd/state/story_unit.py` (survey) | `acs(body)` is the one reader of a story's acceptance criteria: it skips headings inside code fences, and `verdicts._story_acs` delegates to it (`test_items_equal_unit_acs` holds the two together). `survey-2`, which rewrites this file, keeps `acs` as it is | S4R-verdicts-7, sanctioned after the fact by the orchestrating session (S4R-verdicts-C1-4) |
| `survey-2` | `P/py/vsdd/state/story_unit.py`, `survey.py`, `checkpoint.py`, and the first part's tests that pin a whole unit or the provisional stage entries | As survey-2.md section 2 | survey.md section 1 and step 3's `ORDER.md` section 7 ("provisional") |
| `survey-2` | `P/schemas/story-unit.schema.json` (C1), its two fixtures, the `storyUnit` literal in `tests/workflows/universal.test.mjs` | `bc_paths`, `conventions`, `demo_dir` | S5P-N1, S5P-N11 |
| `rules` | `P/py/vsdd/hooks/registry.py` (hook-runner) | Three rows between `# >>> rules` and `# <<< rules` | hook-runner.md 3.3 |
| `rules` | `tests/lint/test_hooks_wiring.py` (hook-runner) | The wiring `test_shape` expects, which the new rows change | hook-runner.md 3.3; hook-runner-b.md edits it the same way |
| `gates` | `P/schemas/gate-result.schema.json`, `wave-gate-data.schema.json` (C1), with fixtures | `test_output`, `changed_files` | P3BLD-16, VSA-03 |
| `gates` | `tests/e2e/sample-project/tools/junit_runner.py` (E1) | `file` on each test case, `type` on a failure or error | VSA-12 |
| `gates`, `survey-2`, `rules` | `workflow-lib/prelude.js`, the toys' stamped regions, `P/hooks/hooks.json` | By `tools/build.py` only | Rule 3 |

Without the first row there is no covering verdict inside the run that reviewed the story (the closure
verdict is written at the checkpoint), and the first delivery would never merge. If the orchestrator
does not allow it, `verdicts` cannot report and stage B must not start.

## 5. Asked of step 3

Records and the survey were not built when these plans were corrected. Each request goes into the
build of the unit named; if that unit has already reported without it, the orchestrator has it added
by a fix run of that unit before the stage that needs it.

| Id | Unit | What | Needed by | Blocking |
|---|---|---|---|---|
| R1 | `survey` | `tests/unit/story_ready.py` takes `product=<dir or callable>`: it copies and commits the product on the integration branch and sets `[paths]` before the story is admitted, so the tally sample is in the story's base (PL4-9). It does not copy the product's `.factory/`, and takes `[paths]` from the product's own `.factory/factory.toml` when it has one (PL6-7; built 2026-11-24, `tests/unit/test_story_ready.py`) | `gates`, then `survey-2` | Yes, stage A |
| R2 | `records` | `evidence_dir(project, "_base")` is accepted: the path-segment rule allows `_base` beside `_standing`, with a test (PL4-13) | `gates` | Yes, stage A |
| R3 | `records` | `closure-record.schema.json` names the `delivery` keys `state`, `detail`, `needs_person`, `run`, `at`, `head`, `patch_id` (PL4-14). They validate today | `delivery` | No |
| R4 | `records` | The closure rule of the `review` branch sits in one function, `_review_lookup(project, subject)`, so the edit of section 4 replaces one body | `verdicts` | No |
| R5 | `hook-runner` | Nothing to add. `tests/hooks/test_rule_cases.py` keeps reading `tests/hooks/fixture_cases/` only, as built: `tests/hooks/cases/` is run by the rules unit's own `test_rule_tables.py` (PL4-3) | `rules` | Only if it changes |

Asked for by the review and not needed after all:

- PL4-11, `gitrun.git(env=)` as a whole environment: delivery runs git that can start a project's
  hooks through the job's `ctx.run`, which already runs in `command_env(root)`. S1 is not changed.
- PL4-14, forcing the deletion of a squashed branch and a `set_integration` that survives a merge in
  progress: delivery deletes the branch itself with `git update-ref -d` and resets the integration
  worktree before `set_integration`.
- PL4-8, the registry signature: records stores the callable as given, so its code needs no change.

## 6. Files each unit owns

A unit creates and edits only its own files, and the files of section 4. `S` is `P/py/vsdd/state`.

| Unit | Creates |
|---|---|
| `verdicts` | `S/convergence.py`, `S/verdicts.py`; `tests/conformance/cases/*.json`, `tests/conformance/conformance.test.mjs`, `tests/conformance/test_conformance.py`; `tests/unit/test_verdicts.py` |
| `gates` | `P/py/vsdd/gates/junit.py`, `diffpatch.py`, `gatelib.py`, `gate_red.py`, `gate_green.py`, `gate_demo.py`, `cmd_gate_story.py`; `tests/unit/gate_project.py`, `tests/unit/fixtures/junit/`, `tests/unit/test_junit.py`, `test_diffpatch.py`, `test_gate_red.py`, `test_gate_refreeze.py`, `test_gate_green.py`, `test_gate_demo.py`, `test_gate_cli.py`, `test_gate_registry.py`; the fixture `valid_green-since.json` |
| `rules` | `P/py/vsdd/rules/tests_first.py`, `merge_guard.py`, `step_results.py`; `P/py/vsdd/hooks/writes.py`; `P/py/vsdd/grounding.py`; `tests/unit/test_grounding.py`; `tests/hooks/rule_projects.py`, `test_tests_first.py`, `test_merge_guard.py`, `test_step_results.py`, `test_writes.py`, `test_rules_fail_closed.py`, `test_rule_tables.py`; `tests/hooks/cases/tests-first.toml`, `merge-guard.toml`, `step-results.toml`; new payload files under `tests/hooks/payloads/` |
| `survey-2` | `S/remeasure.py`, `S/unit_outcome.py`, `S/packets.py`; `tests/unit/story_progress.py`, `tests/unit/test_ladder.py`, `test_remeasure.py`, `test_unit_outcome.py`, `test_packets.py`, `test_survey_decisions.py`, `test_checkpoint_mapping.py` |
| `delivery` | `P/py/vsdd/delivery/__init__.py`, `cmd_story_deliver.py`, `deliver.py`, `checks.py`, `local.py`; `tests/unit/delivery_fixtures.py`, `test_delivery_checks.py`, `test_delivery_local.py`, `test_delivery_cmd.py` |

Two directories take files from more than one step 4 unit: `S` (`verdicts`, `survey-2`) and
`tests/unit/` (all five). Each unit adds its own files and none edits another's. No step 4 unit edits
`tools/check.py`, `tools/build.py`, `P/py/cli.py`, `S/__init__.py` or `tests/hooks/test_rule_cases.py`.

Not built by anyone in step 4, though an earlier draft of a plan had it: a grounding module under
`S/` (PL4-1), `S/cmd_review.py` (PL4-18), `P/py/vsdd/rules/decisions.py` (PL4-18), a `changed_files` in
`gates` or `delivery` (PL4-7), a patch writer in `delivery` (PL4-6).

## 7. For the orchestrating session

**Scope changes to record** (`rewrite/STATE.md`, and the briefs where the registry allows):

- G3's "grounding" deliverable moved to H7: the rules unit owns `vsdd/grounding.py` (PL4-1).
- Deferred, because the story path never reaches them (PL4-18): `vsdd review show` (G3; with the
  `review` skill, K2); the `decisions` rule and its `UserPromptExpansion` wiring (H7; with K1's `decide`
  skill; until then a person settles with `vsdd decide`); sending a notification (S3 when widened;
  `github-issue` with G4b). A set `notify.channel` is logged as `notify.failed`, never silent.
- Deferred to G4b (PL4-12): the `merge` decision and the checkpoint status `needs_human`. In local
  mode a story that needs a person is refused every run and opens `blocked.<story>` (`stuck`) after
  `limits.failed_runs_before_stuck` runs; the packet names `needs_person` and the delivery detail.

**Probes before E2's first run** (B8), beside those step 3 left open. Round 4 and its independent check
answered all three on 2026-11-24 (Claude Code 2.1.331, a `--plugin-dir` plugin, default mode). Item
numbers are those of `rewrite/spec/topics/platform-proven-2.md`.

- P1 (PL4-22). Does the `PreToolUse` payload of a workflow step's `StructuredOutput` carry
  `agent_type: "vsdd-factory:<agent>"`? G3 counts a review pass only then. **Answered, item 36:** a
  typed step's payload carries `agent_type` exactly `<plugin>:<agent>`, an untyped step's
  `workflow-subagent`, and one `agent_id` on every call of the step. Main-session payloads have neither
  key, so both are read with a default, as the built runner does. Only a `--plugin-dir` plugin was
  probed: each uncounted pass stays in `rebuilt()["skipped"]` with `agent_type`, and delivery's refusal
  keeps saying so, until one marketplace-installed copy has been checked
  (`rewrite/spec/platform.md`, open leg 36).
- P2. Is `updatedInput` honoured on `StructuredOutput` when the rule adds `grounded: false`? **Answered,
  item 37:** yes. The key reaches the script under the shipped open `finding.schema.json`, declared or
  not. No `grounded` property is added to any schema, which closes the question the H7 planner left
  open (`RESULT.json`). A schema that closes the finding object makes the rewrite unreturnable and the
  step fails; rules.md's `test_finding_schemas_stay_open` guards that.
- P3. Is `allow` with a rewrite honoured in the default permission mode? **Answered, item 38:** yes,
  inside a step, for Bash and `StructuredOutput`. Item 39 adds that a rewrite with no decision is
  applied but grants nothing, so the rule's `Rewrite` keeps travelling with `allow` (HD7).

The same round answered three questions the planners left in `RESULT.json`:

- The hook sees a result before the platform's schema check, which then reads the input as the hook
  left it (item 40). `schema_invalid` can therefore fire, and the platform can still refuse a result
  the rule stored. A block reaches the agent inside the platform's wrapper (rules.md D16).
- Exit 2 from a `UserPromptExpansion` hook stops a typed plugin command (item 46), for the `decisions`
  rule when it is built. The blocked result reports `is_error: false`.
- A project's git hooks do run in the integration worktree, with tracked files only (item 45; rule 7).

Still open before E2's first run: the `clear` half of P3OPS-20 (item 46), the detached worker on Linux
(item 43) and an installed plugin's `agent_type` (item 36). That run is headless, and what it must set
is in `rewrite/spec/platform.md`, "Headless runs".

**Corrections to `rewrite/plans/step-5/agents.md`** (PL4-23). Its assumptions of section 8 are met: a
re-freeze result is named `red` and passes when no frozen test changed; `gate green --since` is named
`green`; `sha` is the head. What round 4 changed for the agents (a commit the sandbox refuses, a call
moved to the background, a block read inside the platform's wrapper) is already in that plan. These
differ and are not yet in it:

- `test-fixes`: `--refreeze` works only as the whole single-segment command of the test-writer's own
  call, never inside `cd x && ...`, and is refused without the caller marker. A changed frozen test
  that is skipped fails the re-freeze.
- `fix-findings` step 5: when delivery's `detail` says the tool concluded the merge, there is no marker
  to resolve; run the green gate, do not return `blocked`.
- `demo`: the evidence directory is the unit's `demo_dir` (`.factory/demo/<cycle>/<story>/`), which is
  the directory the demo gate reads.
- M1 and the review methods: contracts arrive as `bc_paths`. A finding's `location.path` under
  `.factory/` is relative to the project root, any other path to the tree root.

**Inputs to W2's plan (step 6):**

- Pass `u.demo_dir`, `u.bc_paths`, `u.conventions` and `u.guidance` to the steps (S5P-N1, S5P-N11).
- Take `coverage_missing` from the `review-story-diff` result alone (S5P-N9).
- After a repair, take the later loop's items from the `--since` gate result's `changed_files`, not
  from `d.changed_files`, and its patch from that result's `diff_patch`, the range patch (VSA-03,
  PL4-6). For `review_stale` both come from the delivery data.
- Refresh `test_output` wherever the patch is refreshed (P3BLD-16).
- A repair whose changed files touch a frozen test needs a test-writer step with `--refreeze`; today
  that repair ends `blocked` (PL4-19, accepted for the slice).
- After a repair the script runs the evidence stage again (step 6 `story-workflow.md` D9; PL6-12), so
  delivery's demo row sees a recording made at the head.
- `used` for loop 1 is the unit's `rounds_used`; a later loop passes 0.

**Ledger.** Outcomes these plans claim, to record when each unit reports: VSA-03 (G3, G2, G4a), VSA-05
and VSA-07 (G3, H7), VSA-09 (G4a, S3), VSA-10 and VSA-18 (G4a), VSA-11 (H7), VSA-12, P3SEAM-15,
P3SEAM-16 and P3BLD-16's gate half (G2), P3REQ-06 (S3 with G2), S5P-N1 and S5P-N11's tool half (S3),
S5P-N9's tool half (G3), P3OPS-09's tool halves (G3, S3). Not at slice depth after these corrections:
P3COV-06's notification half. New, from the plans: red evidence cannot be restored at an earlier
commit; opening `finding.<story>` again loses earlier waivers; results the script makes itself are not
stored and step records carry no time; PL4-19.

**For the spec steward.** `commands/gate-green.md` gives one patch name for both ranges (now two);
`topics/verdicts.md` says the tool "applies the grounding rule" (it honours the stored mark);
`schemas/closure-record.schema.json` lacks the delivery keys of R3; `topics/state-api.md` should say
`verdict` always returns an object; `commands/story-deliver.md` says "Git hooks run" without saying
which (rule 7). The spec files round 4 left behind the platform record are ledger findings, not a
list here: `probes-4-apply-C1-1` (items 27, 28 and 30 called unproven in `rules/step-results.md`,
`topics/hook-platform-facts.md` and `topics/caller-identity.md`), PR4C-16 and PR4C-12
(`packaging/tests-e2e.md`: `is_error`, the ceiling variable) and PR4C-17 (`topics/spikes.md`). Until
they are edited, the platform record holds over them.

## 8. Open question for the person (PL4-20)

`config.py` defaults `delivery.dependency_changes = "human"` and protects `CHANGELOG.md`. Until governed
delivery (G4b) is built, local delivery refuses, with `needs_person` set, any story whose diff touches a
manifest file or the changelog. E2's sample story must therefore touch neither, and the tally sample
lists no manifest.

The question: is that refusal acceptable until G4b, or should local delivery in the slice merge such a
story (for example by setting `dependency_changes = "auto"` and dropping the changelog from
`protected_paths` in the sample's `factory.toml`)? The plans build the refusal.
