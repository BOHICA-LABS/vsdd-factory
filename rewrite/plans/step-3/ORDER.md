# Step 3: build order, concurrency and file ownership

Step 3 of the thin slice is S1, S2a, S2b, S4, the first part of S3, H1 then B4b, and G1
(`rewrite/spec/packaging/build-plan.md`, "The thin slice"). It is built as seven units, each from one plan
in this directory, each by one run of `rewrite/workflows/slice-build-unit.js`. The plans were corrected
against `REVIEW.md`; where a plan and the review differ, the plan holds.

Every builder and reviewer reads this file. It says when a unit may start, what may run beside it, which
files it owns, and the rules that cross units.

## 1. The units

| Key (`args.key`) | Plan | Packages (`args.packages`) | Title (`args.title`) |
|---|---|---|---|
| `state-core` | `state-core.md` | S1 | The state core: command dispatch, the `vsdd.state` facade, mount, registry, init |
| `hook-runner` | `hook-runner.md` | H1, B4b | Hook runner core and hook wiring |
| `jobs` | `jobs.md` | G1 | Jobs and the product environment |
| `records` | `records.md` | S2a, S2b, S4 | Record stores, lock and caller identity, decisions |
| `survey` | `survey.md` | S3 (first part) | Survey, checkpoint and story admission for one story |
| `hook-runner-b` | `hook-runner-b.md` | H1 (second part) | Hook runner: path classes and lock liveness |
| `jobs-b` | `jobs-b.md` | G1 (second part) | Jobs: the `env` commands |

`hook-runner-b` and `jobs-b` are the parts of H1 and G1 that need S1 and records. They have their own
plans so that each run has one plan (the script's default is `rewrite/plans/step-3/<key>.md`) and so
that the first parts can be built beside S1.

Launch a unit with `args: {step: 3, key, title, packages, concurrent}`, where `concurrent` lists the
keys of the units in flight at the same time, as section 2 gives them.

## 2. Order

| Stage | Units, at the same time | A unit may start when |
|---|---|---|
| 0 | none | `slice-layer-2a` has reported (catch-up, C2, B6) and every check passes under `next/` |
| 1 | `state-core`, `hook-runner`, `jobs` | stage 0 holds |
| 2 | `records` | `state-core` and `hook-runner` have both reported |
| 3 | `survey`, `hook-runner-b`, `jobs-b` | `survey`: `state-core`, `records` and `jobs` have reported. `hook-runner-b`: `hook-runner`, `state-core` and `records` have reported. `jobs-b`: `jobs`, `state-core` and `records` have reported |

"Reported" means the unit's whole run returned: build, review, fixes and confirmation. A unit whose run
left blocker or major findings open is not a base to build on: fix it first.

`records` need not wait for `jobs`. If `jobs` is still running when stage 2 starts, launch `records` with
`concurrent: ["jobs"]`; their files are disjoint.

What forces the order:

1. Layer 2a before `state-core`: S1 edits `vsdd/config.py`, which C2 changed.
2. `state-core` before `records`: records' commands register through S1's `Command`, its names publish
   through S1's `__all__` facade, its flush calls S1's `check_paths` and `mount.mounted`, and its tests
   start from S1's `_repo.make_project`.
3. `hook-runner` before `records`: both add schema names to the same two sets in `tools/check.py` and
   `tools/build.py`. The check needs an exact match, so the edits are made one unit after the other.
4. `records` before `survey`: the survey is built on `run_begin`, `closure_update`, the decision store,
   `spec_approved`, `evidence`, `caller` and `flush`.
5. `jobs` before `survey`: the checkpoint calls `vsdd.gates.jobs.stop_products`.
6. `state-core` and `records` before `jobs-b`: `gates/cmd_env.py` registers through S1's dispatcher and
   reads records' `caller`.
7. `state-core` and `records` before `hook-runner-b`: path classes need S1's `path_class` and
   `glob_regex`; lock liveness needs records' `lock_seen` and `lock_release`.
8. H1 before B4b, inside `hook-runner`: `hooks.json` is generated from H1's registry (its step 7).

After the last unit of each stage reports, run every check once on a quiet machine before the next
stage starts: `tools/check.py`, the whole pytest suite, the node harness, `tools/build.py --check`.
`tests/hooks/test_latency.py` (a 120 ms median over real launches) is sensitive to load: a failure
while other builders run is not a finding until it repeats on a quiet machine. Take a snapshot of the
tree before each stage.

## 3. Files each unit owns

Paths are under `next/`; `P` is `plugins/vsdd-factory`. A unit creates and edits only its own files. A
file not listed here belongs to an earlier package: read it, do not edit it, and report what was missing.

| Unit | Creates | Edits (shared files it alone may change in step 3) |
|---|---|---|
| `state-core` | `P/py/vsdd/cli/` (`__init__.py`, `dispatch.py`); `P/py/vsdd/state/__init__.py`, `gitrun.py`, `mount.py`, `detection.py`, `artifacts.py`, `cmd_init.py`, `cmd_config.py`, `cmd_validate.py`, `cmd_path.py`; `P/py/vsdd/globs.py`; `tests/unit/_repo.py`, `test_dispatch.py`, `test_state_facade.py`, `test_project_paths.py`, `test_mount.py`, `test_init.py`, `test_config_keys.py`, `test_artifacts.py`; `tests/unit/fixtures/artifacts/` | `P/py/cli.py`; `P/py/vsdd/config.py`; `P/py/vsdd/project.py` |
| `hook-runner` | `P/py/vsdd/hooks/` (`__init__.py`, `model.py`, `registry.py`, `main.py`, `emit.py`, `caller.py`, `shell.py`, `gitfs.py`, `policy.py`); `P/py/vsdd/rules/__init__.py`; `P/schemas/caller-marker.schema.json`; `P/hooks/hooks.json`; `tests/hooks/runner_helpers.py`, `fixture_rules/`, `payloads/`, `fixture_cases/`, `test_runner.py`, `test_emit.py`, `test_ctx.py`, `test_shell.py`, `test_gitfs.py`, `test_caller.py`, `test_rule_cases.py`, `test_latency.py`; `tests/lint/test_hooks_wiring.py`; `tests/unit/fixtures/schemas/caller-marker/`, `.../event/valid_rule-block.json`. It removes `P/hooks/.gitkeep` | `P/py/hook.py`; `P/schemas/event.schema.json`; `tools/build.py`; `tools/check.py` |
| `jobs` | `P/py/vsdd/gates/` (`__init__.py`, `jobs.py`, `job_worker.py`, `env.py`); `tests/unit/test_jobs.py`, `test_env.py`, `_jobs_helper.py` | none |
| `records` | `P/py/vsdd/state/` (`store.py`, `runs.py`, `steps.py`, `gate_evidence.py`, `cycles.py`, `digests.py`, `decisions.py`, `callers.py`, `locks.py`, `commits.py`, `cmd_decide.py`, `cmd_lock.py`); the seven schemas `P/schemas/{run-record,step-record,evidence-record,closure-record,cycle-record,decision-record,lock}.schema.json` with their fixture directories; `tests/unit/state_fixtures.py`, `test_state_*.py` | `tools/check.py` and `tools/build.py`: its seven schema names only, in the two sets `hook-runner` edited |
| `survey` | `P/py/vsdd/state/` (`worktrees.py`, `admission.py`, `story_unit.py`, `preconditions.py`, `route.py`, `survey.py`, `checkpoint.py`, `cmd_survey.py`, `cmd_checkpoint.py`); `tests/unit/story_ready.py`, `test_worktrees.py`, `test_admission.py`, `test_story_unit.py`, `test_preconditions.py`, `test_route.py`, `test_survey.py`, `test_checkpoint.py`, `test_survey_cli.py` | none |
| `hook-runner-b` | `P/py/vsdd/hooks/paths.py`, `liveness.py`; `tests/hooks/test_classes.py`, `test_liveness.py` | Files of `hook-runner` only: `hooks/model.py`, `registry.py`, `main.py`; `P/hooks/hooks.json` (by `tools/build.py`); `tests/lint/test_hooks_wiring.py`; `tests/hooks/test_ctx.py` |
| `jobs-b` | `P/py/vsdd/gates/cmd_env.py`; `tests/unit/test_cmd_env.py` | none |

Three directories take modules from more than one unit: `P/py/vsdd/state/` (`state-core`, `records`,
`survey`), `P/py/vsdd/gates/` (`jobs`, `jobs-b`) and `P/schemas/` (`hook-runner`, `records`). Each unit
adds its own files; none edits another's.

Files with exactly one step 3 writer, although several units depend on them:

| File | Writer | What the others do |
|---|---|---|
| `P/py/cli.py` | `state-core` | Nothing. A command is a `cmd_<family>.py` module. |
| `P/py/vsdd/state/__init__.py` | `state-core` | Nothing. A flat name is an entry in a module's own `__all__`. No other unit creates this file. |
| `P/py/vsdd/project.py` | `state-core` | `hook-runner` handed over its `home` default change; it passes `home` itself. |
| `P/py/vsdd/config.py` | `state-core` | `hook-runner` reads `Config.get("hooks", "rules")` and `Config.hook_problems`, which S1 keeps. |
| `tools/check.py`, `tools/build.py` | `hook-runner`, then `records` | Each adds its schema names in the same step as its schema files and keeps the other's names. No other unit edits either file. |
| `P/hooks/hooks.json` | `hook-runner`, then `hook-runner-b` | Generated by `tools/build.py`; nobody edits it by hand. |
| `P/schemas/story-unit.schema.json` | none in step 3 | See section 5. |
| `workflow-lib/prelude.js`, `tests/workflows/` | none in step 3 | No step 3 unit changes a schema the prelude carries, so no unit regenerates or edits them. |

## 4. Rules that cross units

Each is stated in the plan of the unit that owns it; this is the list, so that no builder writes a second
form.

1. **Commands** (state-core.md 3.1). One module `cmd_<family>[_<x>].py` holding
   `COMMANDS = (Command(...),)`; `run(inv, args) -> Reply` with code 0 or 1; exit 2 is `raise Refuse`. No
   unit edits `cli.py` or a command table. `decide` uses `raw=True`. `needs`: `decide` is `mounted`;
   `survey`, `checkpoint`, `lock release`, `env start` and `env stop` are `initialised`.
2. **Relayed commands** (state-core.md 3.1). `survey` and `checkpoint` set `Command.output` and
   `failure`, so every exit 2 still prints every field of the schema.
3. **Flat names** (state-core.md 3.2). A module's literal `__all__` holds the names of
   `topics/state-api.md` it implements and nothing else. Any other name is imported from its module. A
   module is never named after an exported name. Hook code imports the submodule, never the facade.
4. **Who is calling** (records.md). `vsdd.state.callers.caller(inv.caller_key, project=inv.root)`. Nothing
   reads `os.environ` for it. The hook runner only writes markers; `caller` reads, deletes and prunes.
5. **Person only** (state-core.md, decision 2). `inv.require_person(what)`: no `VSDD_CALLER` and a
   terminal on standard input, exit 2. No unit writes a second check.
6. **The project argument** (records.md 3). A function that takes a project takes a
   `vsdd.project.Project`, any object with `.root`, or a `str` or `Path`.
7. **Git** (state-core.md 3.3). `vsdd.state.gitrun.git`, called or wrapped. **Globs** (3.4):
   `vsdd.globs.glob_regex`. **Path classes** (3.4): `vsdd.project.path_class`. **Worktree layout**:
   `vsdd.project.classify_worktree`, used by the survey's worktree library and by jobs.
8. **Run membership** (records.md): `runs.run_require`; the survey's `require_story` calls it.
   **Re-entry and the lock scope** (records.md): `locks.run_begin` and `locks.lock_scope`; the survey
   only calls them. **"The specification is approved"** (records.md):
   `decisions.spec_approved(project) -> tuple[bool, str]`.
9. **Test projects** (state-core.md 2). Every unit's tests start from
   `tests/unit/_repo.make_project`, which runs the real `init`. Records' `state_fixtures.make_project`
   and the survey's `story_ready.py` build on it. The hook runner's `runner_helpers.make_project` writes
   only a `factory.toml`, which is all a hook needs.
10. **A schema file and its name land together.** A unit that adds a schema adds its name to
    `tools/check.py` and `tools/build.py` in the same step, so the shared check never sees one without
    the other.
11. **The Bash sandbox** (`rewrite/spec/topics/platform-proven-2.md`, item 41). With the sandbox on, a
    Bash call cannot write under `.git/worktrees/`: git cannot commit in `.factory`, in a story worktree
    or in the integration worktree, and cannot add a worktree. A unit whose git call fails with
    `Operation not permitted` in git's output reports that as its own failure, with an error that holds
    the word `sandbox` and git's own line and says to run outside the sandbox. For a person that means
    turning the Bash sandbox off for the project, or granting the write
    (`rewrite/spec/packaging/sandbox-needs.md`), and running the same command again. The unit undoes
    nothing, so the next run finishes from what the refused one left. Who does it:
    - `records`: `flush` returns `state: "failed"` with that `error` (PR4C-08;
      `test_flush_never_raises`). A flush started from a sandboxed Bash call can commit nothing in
      `.factory`.
    - `survey`: `worktrees.py` raises `WorktreeError("not_permitted")` (survey.md D20), and the opening
      flush's error is passed on as the survey's (survey.md D11). Both are exit 1 with the word in `error`.
    - Step 4 builds on the same words: a gate's worker lets the `WorktreeError` out, and delivery answers
      `refused` (step 4's `ORDER.md`, rule 7).

    Each unit has a test that asserts the word. No unit tests for the sandbox itself or guesses at it
    from anything but git's `Operation not permitted`.

## 5. The story unit (PL3-10)

`topics/story-script.md` reads `tests_agent`, `tests_method`, `impl_agent`, `impl_method` and `red_gate`
from the unit. When the plans were reviewed the shipped `story-unit.schema.json` had `method` and
`tests_method` only. The layer 2a workflow has since changed the shipped schema, its fixtures and the
harness's `storyUnit` literal: the five fields are there, all required, and `method` is gone. No step 3
unit edits that schema. The survey fills the fields from the story's `[[story-kind]]` row (survey.md
D14). If a builder finds the schema without those fields, it stops and reports; it does not add them.

## 6. Open question for the person (PL3-21)

The hook runner approves an agent's single-segment `vsdd` call (`permissionDecision: allow` with the
command rewrite that carries the caller marker), whatever `hooks.rules.auto-allow.scope` says. That key
defaults to `off`, and decision D4 makes approving the plugin's own calls something a person opts into
at setup. It is built this way because the marker needs the rewrite, and a rewrite paired with `ask`
does not run in an unattended session (PR3-3). It covers `vsdd` calls only, the main session is still
asked, and H6's caller rows will narrow which subcommands an agent may run.

The question: should `auto-allow.scope = "off"` stop this approval, which would make every unattended
run prompt, or are the plugin's own `vsdd` calls outside that key?

**Answered by the person on 2026-11-24: the plugin's own `vsdd` calls are outside that key.** Setting it
to `off` does not stop the approval that carries the caller marker. The plan as written (hook-runner.md
HD7) is therefore the decision, not a stopgap; it will be recorded as decision D32 in
`rewrite/spec/decisions.md`. Build and review it as decided, and test that the approval is given with
the key `off`.

## 7. For the orchestrating session

- **Resolutions.** The integrator's decision on each of PL3-1 to PL3-32 is in its structured result;
  save it as `RESOLUTIONS.md` here.
- **`rewrite/STATE.md`.** Step 3 now has seven plan files and seven build runs.
- **Scope changes to record.** S1 no longer builds `vsdd new`, its skeleton templates, or `vsdd list` in
  step 3: nothing on the story path calls them (PL3-30). `vsdd path`, `vsdd config --json` and the `env`
  commands stay, because the step 5 method files call them. Records ships the seven record schemas the
  registry gives to C1 (PL3-32).
- **Ledger.**
  - P3COV-10 and P3SEAM-10: the catch-up outcome says the story script already omitted `--subject`.
    `topics/story-script.md` passes it. The outcome should say: the toy dropped it, the command accepts
    an optional `--subject` that must equal the run's (survey.md D1), and W2 settles what the script
    sends (PL3-22).
  - P3BLD-14, P3OPS-20, P3SEAM-24: `records` records them left-for-builder; `hook-runner-b` closes
    them, except the `clear` half of P3OPS-20, which waits for a probe (B8). **Still open after round
    4** (platform item 46): that round tested SIGTERM, not a session end with reason `clear`; the probe
    needs an interactive session.
  - P3SEAM-07: left-for-builder (H4, with `spec_locked`); only `run_live` is built (PL3-11).
  - A new finding, met while deciding PL3-4, for B4a and W2: the prelude's `emptyOf()` gives `status: ""`
    and `next: {}`, and neither passes the shipped `checkpoint-result` or `survey` schema (`status` has
    an enum without `""`; `next` requires `kind`). `relay()` tells the clerk to return that object when
    the command prints no JSON, which the platform's schema check would refuse. The Python side avoids
    it with `Command.failure`; the prelude needs the same per-schema values.
- **Probes.** What the round-4 probes answered, and what is still open, is marked on each question in
  `RESOLUTIONS.md`, with the item number of the platform record.
- **Step 4 planners** build on these:
  - `position()` and the checkpoint's stage mapping in survey.md are provisional (PL3-29).
  - A gate module is named `gate_<name>.py` and registers its measuring function with
    `vsdd.state.gate_evidence.register_gate` at import (PL3-20). G3 puts the computed verdict ahead of the
    closure's in `evidence(project, "review", subject)`.
  - A gate checks its story with `admission.require_story`, attaches with
    `attach(..., retry=inv.retry_command(), wait=inv.wait)`, and a relayed command sets `Command.output`.
  - Rules import `vsdd.state.steps`, `gate_evidence` and `locks` by module. `ctx.classes` exists once
    `hook-runner-b` is built. H9 calls `stop_products(..., grace=2.0)`, inside its 4 s deadline (PL3-28).
  - The note store must read `{"notes": []}` as no notes, and the S4 "approve, survey, plan" fixture
    comes with the plan survey (PL3-24).
