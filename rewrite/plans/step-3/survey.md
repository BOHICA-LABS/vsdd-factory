# Step 3 plan: survey, checkpoint and story admission for one story (S3, first part)

Paths: spec files are under `rewrite/spec/`, code is under `next/`, and `ST/` stands for
`plugins/vsdd-factory/py/vsdd/state/`.

## 1. Scope

This unit builds what one standalone `story` run needs from S3 (`packages/S3.md`): `vsdd survey story`
(caller marker, the story's preconditions, flush, run and lock through records' `run_begin`, admission of
a planned story, the story unit with its position); the worktree library; story admission as one library
function, with the check every gate and delivery repeats; the route rows a one-story project can reach;
and `vsdd checkpoint story` (closes a delivered story, records positions as stage entries, flushes, ends
the run, releases the lock, returns `next`). `position()` and the checkpoint's stage mapping are
provisional: the second part replaces them with the mapping of step records.

Left for the second part (build-plan step 4, after G2 and G3):

- re-measuring a gate whose evidence is missing, through the gate registry and G1's jobs, with a pending
  survey answer; `reviewed` from the verdict G3 computes;
- the checkpoint's mapping of step records and evidence to stage entries with their own commits and
  artifacts, replacing `position()`'s part in it; the verdict and `flaky` copied into the closure;
- decisions: blocked steps to `finding`, `spec-gap`, `blocked.<subject>` and `merge`; `stuck`;
  `checkout_dirty`; the statuses `needs_human` and `blocked`; packets; notification; the unit fields that
  read decisions (`waived`, `guidance`, `blocked`, `deferred`);
- adoption of a remote story branch and the `conflict` unit.

Left for when the build widens: the other eleven surveys and checkpoints, with their preconditions and
done test; `checkpoint brief`, `cycle close`, opening and closing cycles, the standing cycle; toolchain
application; request queueing; route rows 3 to 11 and 17 to 19; hub layout and `--repo`; hotfix cycles;
spike worktrees and retention; holdout evaluations; fetching state before the survey. Each is refused out
loud today (D3, D12).

## 2. Files

Owned:

- `ST/worktrees.py`: the git layer of the worktree library. No record access.
- `ST/admission.py`: `admissible`, `admit`, `require_story`, `record_base`.
- `ST/story_unit.py`: the story unit from frontmatter, closure and git; `position`; `story_kind`,
  `kind_has_red_gate`.
- `ST/preconditions.py`: one function per predicate, and `check`.
- `ST/route.py`: the ordered row list and `next_step`.
- `ST/survey.py`, `ST/checkpoint.py`: the bodies of the two commands; `ST/cmd_survey.py`,
  `ST/cmd_checkpoint.py`: their `COMMANDS` (3.2).
- `tests/unit/story_ready.py`: test helper that builds a story-ready project.
- `tests/unit/test_{worktrees,admission,story_unit,preconditions,route,survey,checkpoint,survey_cli}.py`.

Shared: none; `py/cli.py` and `ST/__init__.py` are S1's. `__all__` (state-core.md 3.2): `worktrees.py`
lists `unit_tree`, `story_unit.py` lists `story_kind` and `kind_has_red_gate`, every other module `[]`.
No schema is edited: the shipped `story-unit.schema.json` already has D14's fields (layer 2a).

## 3. Interfaces this unit provides

### 3.1 `vsdd survey` (spec: `commands/survey.md`; output: the built `survey.schema.json`)

`vsdd [-C <dir>] survey <workflow> [--subject <id>] [--repo <name>] [--cycle <id>] [--request-uri <t>]
[--guidance-uri <t>] [--question-uri <t>] [--json] [--wait <s>]`. Built for `story` with `--subject`. The
cycle is the one open cycle; `--cycle`, when given, must name it.

Order: usage checks; read the caller marker with `caller(inv.caller_key, project=inv.root)`; evaluate
every precondition and stop, with nothing written, if one is unmet; flush (D11); `run_begin` (D7), which
settles the lock and writes the run record; admit the story; print `ok: true`, exit 0.

- Exit 1: `ok: false` and a one-line `error`, for no caller marker; an unmet precondition; a flush that
  failed; a lock held by a live run other than this session's run of the same workflow and subject (the
  error names its workflow, subject, run, session and `seen`); or a failed admission (the lock is
  released first, which ends the run).
- Exit 2 (`raise Refuse`; the dispatcher prints the whole empty object): a usage error, or anything not
  built (D3).

What the spec leaves open, as built:

- `preconditions`, always all six, in this order: `project_ready` (`detect` is `ready`); `hooks_alive`
  (the marker was read); `spec_approved` (records' `decisions.spec_approved(project)`, with its detail);
  `story_admitted` (`admissible`); `dependencies_delivered` (the closure of every `depends_on` story has
  `delivery.merged_sha`); `commands_configured` (`commands.test` and `commands.test_report` are set).
- `head`: the commit of the branch the run can write. `scope`: `full` when the cycle's `baseline` is empty,
  else `delta`. `done`: the closure's stage names whose state is `done`. `decisions`: the ids in the cycle.
- `config`: `{tier_override, review_rules, holdout: {mean_min, must_pass_min, stddev_max}}`;
  `tier_override` is `null` or `{model, effort}` (D13). `limits`: `Config.prelude_limits()`.
- `spikes_used`, `rounds_used`: 0; `waived`: `[]` (second part).
- `units`: one story unit, every field real (`tree` is the worktree; `rounds_used` is the closure
  verdict's, else 0; the kind fields as D14) except four that stay empty until the second part: `waived`,
  `guidance`, `deferred` and `blocked` (`{kind: "", decision: "", open: false}`).
- On exit 1: `run_id` and `units` empty, the rest as far as it was computed, `next` from the route.

### 3.2 `vsdd checkpoint` (spec: `commands/checkpoint.md`; output: the built `checkpoint-result.schema.json`)

`vsdd [-C <dir>] checkpoint <workflow> --run <id> [--subject <id>] [--outcome <status>] [--json]
[--wait <s>]`. Built for `story`. `--outcome failed` closes the run as failed; `done`, `partial`,
`blocked` and `needs_human` change nothing (the status is computed); any other word is exit 2.

For each story in the run record: compute the position; add a stage entry for each position reached and
not yet recorded (D9); when `delivery.merged_sha` is present set `state: closed`, `position: delivered`
and `closed`. Then stop each story's product (`vsdd.gates.jobs.stop_products(root, subject=<story>)`),
flush and push. In `finally`: `run_set(agents=<n>)` (D18), then `lock_release(run=<id>)`, which ends the
run; `run_end` is called only when no lock entry names the run. `next` is computed after the release.

- `status`: `failed` with `--outcome failed`; `done` when every story of the run is delivered; else
  `partial`. `decisions`: `[]`. `progress`: true when the run has a step record or a closure changed.
- Exit 0 with `ok: true`. Exit 1 with `ok: false` when flush or push failed; the lock is still released.
- Exit 2 (`Refuse`), nothing changed, for an unknown run, a workflow or `--subject` that differs from the
  run record, or a workflow not built.
- A run whose `ended` is set: writes nothing; returns `ok: true`, `progress: false`, the status the
  closures give, and `next` (D8).

`cmd_survey.py` and `cmd_checkpoint.py` each hold one `Command` (state-core.md 3.1) needing `initialised`
(a broken mount must reach the precondition list and must not strand a lock), with `output` `"survey"`
and `"checkpoint-result"` and `failure` `{"next": {"kind": "human", "command": "", "args": {}}}`, plus
`"status": "failed"` for the checkpoint. `run` is `Reply.of(*survey.run_survey(inv, args))` or
`Reply.of(*checkpoint.run_checkpoint(inv, args))`: a body returns `(object, 0 | 1)`, prints nothing,
checks the object against its shipped schema, and raises `Refuse` for a violation and for exit 2.

### 3.3 `vsdd.state.worktrees` (spec: `topics/worktrees.md`)

`WorktreeError(code, message)`, codes `git`, `occupied`, `branch_mismatch`, `not_descendant`, `missing`,
`not_permitted` (D20).
In 3.3 to 3.5 the first parameter, written `root`, takes a `Project`, any object with `.root`, or a
`str` or `Path` (records' `store.root_of`).

- `story_path(root, story) -> Path`: `<root>/.worktrees/<story>`. No git, so safe on the hook path. A
  run's worktrees are this path for each id in its run record's `stories`.
- `integration_path(root) -> Path`: `<root>/.worktrees/_integration`. Both take the layout from
  `vsdd.project.classify_worktree`, as jobs does. `branch_name(config, story) -> str`: from
  `branches.story`.
- `target_head(root, config) -> tuple[str, str]`: the branch a story is delivered to, and its commit now.
  `branches.integration`; in `pr` mode it runs `git fetch origin <branch>` and reads the remote head.
- `create_story(root, story, branch, base) -> Path`: creates the branch at exactly `base`, then the
  worktree, each when absent and by its own git command (D20). Idempotent. Raises `occupied`,
  `branch_mismatch`, `not_descendant`, `git`.
- `verify_story(root, story, branch, base) -> str`: the head. Raises `missing`, `branch_mismatch`,
  `not_descendant`.
- `remove_story(root, story, *, delete_branch=False) -> bool`: idempotent; true when it removed something.
- `set_integration(root, commit) -> Path`: creates the integration worktree, or forces it, detached, to
  `commit` and cleans it. Moves no branch.
- `detached(root, commit, name)`: context manager giving a temporary detached worktree at
  `.worktrees/_tmp-<name>/`, removed on exit and on error.
- `head(path) -> str`, `is_dirty(path) -> bool`, `is_ancestor(root, a, b) -> bool`.
- `unit_tree(root, workflow, subject) -> Path`: for `story` the worktree; any other workflow raises
  `WorktreeError("missing", ...)` until it is built.

### 3.4 `vsdd.state.admission` (spec: `topics/story-admission.md`)

`AdmissionError(reason)`.

- `admissible(root, cycle, story) -> tuple[bool, str]`: the "story is admitted" predicate (D4).
- `admit(root, *, cycle, story, run, admitted_by) -> dict`: the story unit, valid against
  `story-unit.schema.json`. `admitted_by` is `plan` or the admitting run's id. It checks the story file
  (exists, `id` matches, `status: draft`, a known `kind`, a valid `criticality`); requires `run` to be live
  and to hold the lock; writes the closure (`state: open`, `position: ready`, `delivery.branch`,
  `delivery.base`, `admitted_by`) before it creates the branch; creates branch and worktree; adds the
  story to the run record's `stories`. Run again it changes nothing, except that it adds the story to a
  new run. For a delivered story it creates nothing. Raises `AdmissionError` or `WorktreeError`; whatever a
  failed call left is completed by the next call.
- `require_story(root, run, story) -> dict`: the check of `commands/story-deliver.md`, first bullet.
  Returns `{cycle, closure, worktree, branch, target, base, head}`. It is records'
  `run_require(root, run, story=story)`, the one membership check, then `verify_story`; `AdmissionError`
  when either fails.
- `record_base(root, cycle, story, branch, base) -> None`: the one writer of `delivery.branch` and
  `delivery.base`; delivery calls it after a rebase (VSA-01).

### 3.5 `vsdd.state.story_unit`, `preconditions`, `route`

- `story_unit.build(root, cycle, story) -> dict`; `story_unit.position(root, cycle, story) -> str`
  (`topics/story-life.md`; D6); `story_kind(root, story) -> str`; `kind_has_red_gate(kind) -> bool`.
- `preconditions.check(root, workflow, *, subject="", cycle="", hooks_alive=None) -> list[dict]` of
  `{name, met, detail}`; `hooks_alive=None` leaves that row out, which is the route's "could run" form.
- `route.next_step(root) -> dict`: `{kind, command, args}`; never raises for a readable project.

## 4. Interfaces this unit needs

Built: `vsdd.project` (`classify_worktree`, `read_branch`, the story grammar), `vsdd.config`,
`vsdd.frontmatter`, `vsdd.schema`, `py/launch.sh cli`, and `vsdd.policy.load("artifacts")` (the `[story]`
row; `[[story-kind]]` with `tests_agent`, `tests_method`, `implement_agent`, `implement_method`,
`red_gate`).

- S1 (state-core.md): `Command`, `Invocation`, `Reply.of`, `Refuse`; `vsdd.state.detect(root)`; from
  `artifacts`, `path_for`, `load_artifact` and `registry().raw`; `gitrun.git`; for tests,
  `tests/unit/_repo.make_project` and `set_keys`.
- Records (records.md), imported from their modules: `runs` (`run_read`, `run_require`, `run_admit`,
  `run_set`, `run_end`); `steps` (`run_has_steps`, `steps_list`); `cycles` (`cycles_open`, `cycle_read`,
  `closure_read`, `closure_update`, `closures_list`); `decisions` (`decisions_list`, `decision_status`,
  `spec_approved`); `gate_evidence` (`evidence_dir`, and `evidence(project, gate, subject)`, which for
  `review` returns the closure's converged verdict when it covers the head, else `None`);
  `callers.caller`; `locks` (`lock_scope`, `run_begin`, `lock_release`, `lock_entries`, `lock_is_stale`);
  `commits.flush`; `store.root_of`. Records ships the seven record schemas (C1's by the registry), and
  no state push (D19).
- Jobs: `vsdd.gates.jobs.stop_products(project, subject=<id>)`. `jobs.py` imports no state code, so the
  import has no back edge ("S3 imports no gate package" concerns gate modules).
- The hook runner writes the marker (`records/caller-marker.md`); the tests write that file themselves.

## 5. Design decisions

- D1. `checkpoint` takes `[--subject <id>]`: `topics/story-script.md` passes it, `commands/checkpoint.md`
  and the toy omit it. It must equal the run record's subject, so a strict parser does not strand the
  lock and a garbled id is caught. `--outcome` takes any status word for the same reason (3.2).
- D2. An unmet precondition is `ok: false`, exit 1, with every predicate listed. Nothing is written; only
  the marker is consumed.
- D3. Only `story` is built. Another workflow, `--repo`, a hub layout, a hotfix cycle, two open cycles or a
  `-uri` option is exit 2 with "not built yet", never a guess.
- D4. The open wave is the first wave, in id order, of the plan closure's `waves` whose closure is not
  `closed`. A story is admitted when it is in that wave or its own closure has `admitted_by`; a story in
  a later wave is not.
- D5. `topics/worktrees.md` has the library record the base. The write is `admission.record_base`, so the
  git layer needs no stores. Order: read the target head, write the closure, create the branch at that
  exact commit; if the branch is missing on a later call the head is read and recorded again.
- D6. `position()` returns `delivered` whenever `delivery.merged_sha` is set, before it reads any
  evidence: delivery removes the worktree, and `evidence()` answers `None` without one. Otherwise it is
  the ladder of `topics/story-life.md` over `evidence()`. A kind with no red gate starts at `red`; a
  story whose `demo` is false passes `evidenced`.
- D7. Re-entry (`topics/run-lock.md`) is `run_begin`'s. The survey calls
  `run_begin(root, scope=lock_scope(root), workflow=, subject=, session=<the marker's>, cycle=)` once:
  `ok: false` is exit 1 naming the `holder`; otherwise `run` is the run, whether new, re-entered or
  taken over.
- D8. A checkpoint on an ended run writes nothing: the relay retries a lost answer with a second clerk,
  and a run that was taken over must change nothing.
- D9. Until the second part maps step records, stage entries come from positions: `tests`, `implement`,
  `review`, `evidence` (state `done`, with the unit's head) for an undelivered story, and `deliver` (with
  `merged_sha`) for a delivered one. A story delivered inside one run gets only `deliver`.
- D10. `next` of kind `human`: `command` is empty unless one typed command is the whole answer;
  `args.reason` is the line for the person; `args.decisions` lists ids.
- D11. The opening flush: `committed`, `clean` and `busy` continue; `failed` or `partial` is `ok: false`,
  exit 1, with the flush's error, before any lock. Rejected files stay uncommitted; the checkpoint's
  flush reports them.
- D12. The route holds rows 1, 2 and 12 to 16 of `tables/routes.toml` as an ordered list of named
  predicate functions. After row 2 a guard returns kind `human`, reason "this project state needs a route
  row that is not built yet", when no cycle or more than one is open, the layout is hub, a request file
  exists, or the cycle is not a product cycle on the standard route. A state no built row matches gets
  the same answer. Row 15 needs an undelivered story in the open wave, and an open decision on each.
- D13. `VSDD_TEST_TIER` is `<model>[:<effort>]`, effort `low` by default (the prelude reads `.model` and
  `.effort`; the spec gives no format).
- D14. The unit carries the fields `topics/story-script.md` reads as `K.*`, from the story's
  `[[story-kind]]` row: `tests_agent` (the row's); `tests_method` (the story's `tests_method`, else the
  row's; empty when the kind has no tests stage); `impl_agent` (the row's `implement_agent`);
  `impl_method` (the story's `impl_method`, else the row's `implement_method`); `red_gate` (the row's).
- D15. `acs` holds ids (`AC-1`) read from the criterion headings, because the verdict compares them as
  items. `demo_required` is the story's `demo` key. `fixes` is the story's `source`.
- D16. `diff_patch` is the green record's `diff_patch` and `test_output` the raw output beside it; both
  are empty until green evidence covers the head. On a first run reviewers therefore get no test output:
  W2 should take it from the gate result, as it does the patch.
- D17. The integration and `_tmp-*` worktrees are the tool's own: forced and cleaned without asking.
  `remove_story` keeps the branch unless told otherwise; G4a decides.
- D18. The run record's `agents` is the number of the run's step records. It is written before the lock
  is released: `lock_release` ends the run, and `run_set` refuses an ended run.
- D19. With `state.push` true and no push function shipped: skip when the state branch has no remote;
  otherwise `ok: false` naming the gap, after the lock is released.
- D20. The Bash sandbox (`ORDER.md` rule 11). A git call of `worktrees.py` that fails with `Operation not
  permitted` raises `WorktreeError("not_permitted")` with rule 11's message and undoes nothing. A refused
  `worktree add -b` leaves a branch that its retry fails on, so branch and worktree are made apart: the
  next `admit` finds the branch and adds the worktree.

## 6. Findings (`rewrite/findings/by-package/S3.md`)

- P3COV-10, P3SEAM-10: not claimed (D1; `ORDER.md` section 7). `test_checkpoint_subject_must_match_run`.
- P3BLD-23: the survey records each unit it returns in the run's `stories`; `require_story` and the
  checkpoint take a live run to be one with `ended` empty. `test_survey_records_unit_in_run`,
  `test_require_story_refusals`.
- P3REQ-07, P3OPS-07: resolved on the story path: `admit` called again records the story in the new run
  and returns the same unit. `test_admit_again_same_unit_new_run`. The other surveys: S3 when widened.
- P3OPS-08: resolved for stories: position reads the evidence of any run.
  `test_position_survives_new_run`. Non-story stages: S3 when widened.
- P3REQ-02: no cycle base (D5). `test_admit_base_is_target_head_at_creation`. Readiness ordering: G5 and
  W-feature.
- P3OPS-10: the precondition shows records' `stale` detail (`test_spec_stale_after_edit`). The route row
  and the new decision: not at slice depth, S3 when widened.

Not at slice depth:

- S3 second part: P3COV-06 (decisions); P3REQ-06 (with G1); VSA-09 (with G4a: step 4's
  `ORDER.md`, rule 1).
- S3 when widened: AU-P2-05, P3REQ-01, CL-R2, CL-R5, P3OPS-24, P3SEAM-19, P3REQ-08 (route rows; the
  feature, spec and plan surveys); P3OPS-13 (with S2b); P3REQ-04, P3REQ-05 (with ingest); P3REQ-12 (with
  S5); AU-S2-05 (a spec repair first).
- G4a: VSA-01 (local mode never rebases; delivery calls `record_base`; the lookup is S2a's).

## 7. Tests

`story_ready.py` builds its project with S1's `_repo.make_project` (the real init) and the stores, never
hand-written record JSON. Init leaves `[commands]` empty, so it then writes the sample's `commands.test`
and `commands.test_report` with `_repo.set_keys` and flushes. When E1's `story-ready` loader exists it
becomes a call to it. Commands run in process through S1's `main`, and every output is checked against
its shipped schema.

Worktrees: `test_create_story_at_exact_base`; `test_create_story_idempotent`;
`test_create_story_refuses_occupied_and_foreign_branch`; `test_target_head_local_and_pr`;
`test_verify_story_refusals`; `test_remove_story_keeps_branch`;
`test_set_integration_detached_moves_no_branch`; `test_detached_removed_on_exit_and_error`;
`test_unit_tree_story_and_unbuilt`; `test_refused_add_names_the_sandbox` (git patched to fail once
the branch is made; the next call finishes).

Admission: `test_admissible_open_wave_later_wave_dropped`; `test_admit_writes_closure_run_branch_worktree`;
`test_admit_base_is_target_head_at_creation`; `test_admit_again_same_unit_new_run`;
`test_admit_completes_after_crash`; `test_admit_requires_live_run_holding_lock` (raises, nothing
written); `test_admit_rejects_bad_story`; `test_admit_delivered_creates_nothing`;
`test_require_story_refusals` (not in the run, run ended, worktree gone, history rewritten).

Story unit: `test_unit_fields_and_kind_defaults` (the five kind fields; a story's own methods win);
`test_position_ladder`; `test_position_delivered_without_worktree`;
`test_position_reviewed_from_closure_verdict`; `test_position_falls_back_after_new_commit`;
`test_position_survives_new_run`; `test_kind_has_red_gate_matches_policy`.

Preconditions, one per predicate: `test_project_ready`, `test_hooks_alive`, `test_spec_approved`,
`test_spec_stale_after_edit`, `test_story_admitted`, `test_dependencies_delivered`,
`test_commands_configured`, `test_check_without_hooks_row`.

Route, one per row built: `test_row_not_initialised`, `test_row_live_lock`, `test_row_spec`,
`test_row_missing_commands`, `test_row_plan`, `test_row_waiting_on_decisions`, `test_row_wave`; and
`test_unbuilt_state_is_loud` (no cycle; a queued request; hub; every wave closed).

Survey: `test_survey_story_ready` (run record, lock under the caller's session, worktree, unit);
`test_survey_without_marker_takes_nothing`; `test_survey_unmet_lists_all_takes_no_lock`;
`test_survey_lock_held_by_other_session`; `test_reentry_before_first_step_same_run`;
`test_reentry_after_first_step_takes_over`; `test_stale_lock_taken_over_old_run_ended`;
`test_survey_records_unit_in_run`; `test_survey_flushes_settled_decision`;
`test_survey_limits_config_tier_override`; `test_survey_releases_lock_when_admission_fails`;
`test_survey_failed_flush_takes_no_lock`; `test_survey_unbuilt_is_exit_2_with_whole_object`;
`test_survey_delivered_story`.

Checkpoint: `test_checkpoint_closes_delivered_story` (closure, `deliver` stage, agent count, run ended,
lock gone, state committed, `next` wave); `test_checkpoint_undelivered_is_partial` (stage entries);
`test_checkpoint_outcome_values` (`failed` acts, `done` changes nothing, an unknown word exits 2);
`test_checkpoint_subject_must_match_run`; `test_checkpoint_twice_is_harmless`;
`test_checkpoint_releases_lock_when_flush_fails` (`ok: false`, the error);
`test_checkpoint_broken_mount_still_releases`; `test_checkpoint_unknown_run` (exit 2, `status: failed`);
`test_checkpoint_stops_products`.

Through the launcher: `test_cli_survey_then_checkpoint` (exit 0 twice, with a marker and `VSDD_CALLER`);
`test_cli_survey_no_marker_exits_1_with_full_object`.

## 8. Order of work

Before the builder starts (`ORDER.md`): `state-core`, `records` and `jobs` have reported. The hook runner
is not needed (section 4).

1. `worktrees.py` with its tests.
2. `story_ready.py`, against the real init and stores.
3. `story_unit.py`, then `admission.py`, with tests.
4. `preconditions.py`, then `route.py`, with tests.
5. `survey.py` and `cmd_survey.py`, then `checkpoint.py` and `cmd_checkpoint.py`.
6. The launcher tests; then every check.
