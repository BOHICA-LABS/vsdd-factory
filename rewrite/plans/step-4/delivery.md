# Step 4 plan: delivery (package G4a)

Paths: spec files are under `rewrite/spec/`, code under `next/`. `P` is `plugins/vsdd-factory`, `D/` is
`P/py/vsdd/delivery/`, `T/` is `tests/unit/`. Step 3 plans are in `rewrite/plans/step-3/`; `ORDER.md` is
this directory's.

## 1. Scope

This unit builds `vsdd story deliver` (`commands/story-deliver.md`) for `delivery.mode = "local"`, whole:
the command as a job keyed by the story; the refusal list in its stated order, with the review verdict
matched to the head by commit or patch id; the merge of the checked commit in the detached integration
worktree; the move of the target branch from the head that was read and from no other; `checkout_dirty`;
`conflict`, with the target brought into the story branch for the repair; `review_stale`; the record
(the closure's `delivery`, the event, the flush); worktree removal; and resumption after a kill at any
point, including adoption of a merge already made.

Left for when the build widens; each is refused out loud today (3.3):

- `pr` mode: push, the pull request with the trailer, adoption of an open pull request or remote branch,
  the CI wait, `delivery.ci`, `ci_failed`, rebase with `--force-with-lease` and `record_base`,
  `gh pr merge --match-head-commit`, `pr.created`, the `gh` test double.
- Governed delivery (G4b): `awaiting_merge`, reading an approved `merge` decision, dependency detection.
- `merge_strategy = "rebase"` in local mode; hub layout and `[[repo]]` delivery tables; the hotfix
  branch; the feature-flag sections; `status --fix` removing a delivered story's leftover worktree (S5).

## 2. Files

Owned (this unit alone):

- `D/__init__.py`: a docstring.
- `D/cmd_story_deliver.py`: `COMMANDS`; attaches to the job, prints, picks the exit code (3.1).
- `D/deliver.py`: `work(ctx)`, the job body: resumption, checks, merge, record; `EMPTY`, the empty output.
- `D/checks.py`: the refusal list, the verdict match, `needs_person`.
- `D/local.py`: the local merge, `hook_git`, `move_branch`, the conflict merge-in, the intent file,
  `TRAILER`.
- `T/delivery_fixtures.py`: `deliverable(...)` and `job_context(...)`, a real `JobContext` built by hand.
- `T/test_delivery_checks.py`, `T/test_delivery_local.py`, `T/test_delivery_cmd.py`.

Shared: none. No schema, `tools/` file or `py/cli.py` is edited: `delivery-data` and `pending-output`
are shipped, and the command registers by its file name (state-core.md 3.1).

## 3. Interfaces this unit provides

### 3.1 The command

`vsdd [-C <dir>] story deliver --story <id> --run <id> [--json] [--wait <s>]`. One `Command` with words
`("story", "deliver")`, `output="delivery-data"`, `failure={}`, and `needs="mounted"`, because it writes
tracked state and may simply refuse (state-core.md 3.1).

- It reads the caller marker for the session id only, builds
  `JobSpec(command="story deliver", subject=<story>, commit="", work="vsdd.delivery.deliver:work",
  args={story, run, session}, error_shape=EMPTY, session_id=<session or None>)` and calls
  `attach(root, spec, wait=inv.wait, retry=inv.retry_command())`.
- Pending: prints exactly the `pending-output` object, exit 0.
- A result: prints every field of `delivery-data`; exit 0 when `ok`, else 1.
- Exit 2 (`Refuse`, the whole empty object): `--story` or `--run` missing, a story id outside the story
  grammar, an unmounted state directory.
- A result made for another run is not served (D1).

### 3.2 Output, by state (local mode)

| `state` | `ok` | exit | Fields set besides `state` |
|---|---|---|---|
| `merged` | true | 0 | `merged_sha` (the commit on the target), `reviewed_sha` |
| `conflict` | true | 0 | `detail`, `reviewed_sha`, `changed_files` (D10) |
| `review_stale` | true | 0 | `detail`, `reviewed_sha`, `changed_files`, `diff_patch` |
| `refused` | false | 1 | `error`; `needs_person` when that is the reason |
| `checkout_dirty` | false | 1 | `error`: `the merge of <id> would overwrite uncommitted changes in <checkout>: commit or stash them there, then run the story again` |
| `""` | false | 1 | `error` from the job layer (crash, time limit), naming the log |

`pr` is always empty. `awaiting_merge` and `ci_failed` are never returned in local mode (D7, D9).
`reviewed_sha` is a full commit id. `changed_files` is G3's
`verdicts.changed_files(root, reviewed_sha, <head>)` (PL4-7). `diff_patch` is the
range patch `diff.<reviewed12>-<head12>.patch`, written by G2's `diffpatch.write_patch(root, story,
base, <head>, tree=<worktree>, since=reviewed_sha)`; the whole patch is never touched (PL4-6).

### 3.3 Refusals

Checked in this order; the first that fails is the `error`, word for word (`<..12>` is 12 hex digits).
Nothing is written before row 2 passes.

| # | Condition that fails | `error` |
|---|---|---|
| 1 | a story closure exists in an open cycle | `story <id> is not admitted: no closure record in an open cycle` |
| 2 | the story is in the live run given | `run <run>: <run_unknown / run_ended / story_not_in_run>` |
| 3 | worktree at the recorded path, branch descends from the base | `story <id>: <the AdmissionError reason>` |
| 4 | local mode | `delivery.mode = "pr" is not built yet: only local delivery is` |
| 5 | strategy is `squash` or `merge` | `delivery.merge_strategy = "<x>" is not built for local delivery: use "squash" or "merge"` |
| 6 | the story worktree is clean (G2's `gatelib.dirty_files`) | `uncommitted changes in <worktree>: commit them, then run the gates again: <files>` |
| 7 | red, for a kind with a red gate | `no passing red or re-freeze record between <base12> and <head12>` |
| 8 | green covers the head | `green evidence does not cover the head <head12>` |
| 9 | a converged verdict covers the head (D3) | `no converged review verdict for <id>` (plus `: <outcome>, <reason>`, and `; <n> results not counted: <why>` from G3's `skipped`); or `the reviewed commit <sha12> is not in the story's history` |
| 10 | the demo gate holds: `passed` of the registered `demo` function, for every story | `the demo gate did not pass at <head12>: <first failure>`; `the demo gate is not registered` |
| 11 | no person is needed (D9) | `<id> needs a person's merge decision (<kind>: <what>); governed delivery is not built yet` |
| 12 | the target branch exists | `target branch <b> does not exist` |
| 13 | the merge commit is made | `<git hook <names> or git> refused the merge commit in <path>: <last line>; log: <job log>` (D7); `<id> has no changes against <b>` |
| 14 | the branches hold still | `<b> moved 5 times during delivery: run the story again`; `story/<id> moved twice during delivery` |
| 15 | the delivery lock is had | `another delivery held .factory/local/deliver.lock for <n> s` |

Row 9 is `review_stale`, not a refusal, when D3 finds a converged verdict at an earlier commit.

### 3.4 What it writes (for the checkpoint, the survey and status)

- The story closure, through records' `closure_update`, only inside `delivery`: on a merge `merged_sha`,
  `pr: ""`, `head` (the story commit delivered), `patch_id`; on every outcome after row 2 `state`, `detail`
  (at most 400 characters: the output's `detail`, else its `error`; the one key the checkpoint reads,
  PL4-12), `needs_person`, `run`, `at`.
  Delivery never sets the closure's `state`, `position`, `closed` or `stages`: the checkpoint closes the
  unit from `delivery.merged_sha` (survey.md 3.2), and reads `delivery.state` where `delivery.run` is
  its run (VSA-09).
- The event `pr.merged` with `head`, `branch`, `story`, `url: ""`, when the job has a session id (D14).
- `.factory/local/deliver.lock` (a file lock); `.factory/local/delivery/<story>.json`, the intent (D5),
  present only between the merge commit and the record; the range patch of 3.2, through G2's writer.
- A flush after the merge is recorded, reason `story deliver <id>` (D13); no push. Then D12's cleanup.

### 3.5 Functions

- `vsdd.delivery.deliver.work(ctx: JobContext) -> dict`: the whole delivery; returns an object it has
  checked against `delivery-data`, plus the private key `run`, which the command removes. It raises
  only for a defect, which the job layer turns into the `""` row.
- `vsdd.delivery.local.move_branch(ctx, branch, old, new) -> tuple[str, str]`: steps 2 to 4 of "Local
  mode", with the two git commands that section gives; `ctx` is the job's context (D6). `("moved", "")`;
  `("stale", <head now>)` when the branch is not at `old`, nothing changed; `("dirty", <message naming
  the checkout>)` when git refused the fast-forward and the branch is still at `old`. G6 calls it.
- `vsdd.delivery.local.hook_git(ctx, cwd, *args) -> tuple[int, str]`: D6; the exit code and the last
  output line.
- `vsdd.delivery.checks.needs_person(config, files) -> tuple[str, str]`: `("", "")`, or the kind
  (`configured`, `protected_path`, `dependency`) and what triggered it.
- `vsdd.delivery.local.TRAILER = "Factory-Story"`; `LOCK = "deliver.lock"`. A later gate that sets the
  integration worktree must hold that lock while it does.

## 4. Interfaces this unit needs

This step:

- G3 (verdicts.md 3.3): `verdicts.covering(project, subject, head=)`, the converged verdict that covers a
  commit, in closure form, or `None`; `verdict(project, subject)`, always an object; `summary(...)`,
  `None` when no pass is stored; `changed_files(project, since, head)`;
  `rebuilt(...)["skipped"]`.
- G2 (gates.md 3.5): its gate records, read only through `evidence()`; `registered_gate("demo")(project,
  story, run)`, direct, which returns the passing record for the head or measures;
  `gatelib.dirty_files(tree, config)`; `diffpatch.write_patch` (PL4-15).
- S3's second part reads 3.4. H5 and H7 see nothing: git that the tool starts is not a tool call.

Step 3:

- state-core.md 3.1: `Command`, `Invocation` (`root`, `wait`, `caller_key`, `retry_command()`),
  `Reply.of`, `Refuse`. 3.3: `gitrun.git`, `GitError`. 3.4: `vsdd.globs.glob_regex`. 3.5:
  `artifacts.load_artifact` (the story's `title`).
- records.md 3: `runs.run_require`; `cycles.closure_find`, `closure_update`; `gate_evidence.evidence`,
  `patch_id`, `registered_gate`, `evidence_dir`; `store.locked`, `write_record`, `read_record`, `now`,
  `StoreBusy`, `StoreRefused`; `callers.caller`; `commits.flush`.
- survey.md 3.3: `worktrees.set_integration`, `remove_story`, `head`, `is_ancestor`. 3.4:
  `admission.require_story`, `AdmissionError`. 3.5: `story_unit.build` (`red_gate`, `demo_required`).
  Tests: `T/story_ready.py`.
- jobs.md 3.2 to 3.4: `JobSpec`, `attach`, `Outcome`; `JobContext` (`root`, `args`, `config`, `log`,
  `remaining()`, `run(command, cwd=, env=, output=)`, and its constructor in tests); `JobTimeout`;
  `stop_products(root, subject=)`.

Nothing here waits on a change to step 3 (`ORDER.md` section 5: PL4-11, PL4-14, R3; D2, D6 and D12 do
the work here).

## 5. Design decisions

- D1. **The whole delivery runs in the worker** (jobs.md D1); the command only attaches, so a killed call
  loses nothing. The key holds no run (jobs.md D11), so a job left by a run that was taken over would
  answer `run_ended` to the new run. The result therefore carries its run, and the command drops one
  that is not its own and attaches again. Recomputing is harmless: `merged` and `conflict` are answered
  again from what is on disk.
- D2. **Order in the worker**: rows 1 and 2; resumption (a closure with `merged_sha` goes straight to
  cleanup and answers `merged`; an intent is adopted or dropped, D5); rows 3 to 11; the merge under the
  lock; the record. A story worktree with `MERGE_HEAD` set is answered `conflict` again before row 6,
  so a repair step that died is simply run again. Before `set_integration`, when the integration
  worktree exists, `git reset --hard` runs there: it is the tool's own (survey.md D17), and a merge a
  killed worker left must not fail the next one.
- D3. **The verdict** (PL4-4). `v = verdicts.covering(root, story, head=H)`, `H` the head read at row 3:
  given, the story is covered, by commit or patch id (G3's rule). If `v` is `None`,
  `last` is `verdict(root, story)` when `summary(root, story)` is not `None`, else the closure's
  `verdict` (all a fresh clone has). `last` converged with its `head` an ancestor of `H` and not `H`
  gives `review_stale`, `reviewed_sha = last.head`. Anything else is row 9, a converged verdict at `H`
  that does not cover it included: that must not come back as a `review_stale` with nothing to review.
- D4. **The head-commit match, locally.** The commit `H` that passed rows 3 to 11 is merged by its id,
  never by branch name. After the merge commit is made and before the target moves, the story branch is
  read again: if it is not `H`, the merge is thrown away and the checks run again from row 3, once; a
  second move is row 14.
- D5. **Adoption.** After the merge commit `new` exists and before the target moves, delivery writes
  the intent `{story, run, branch, old, new, head, patch_id, at}`. On the next call,
  if `new` is an ancestor of the target and carries the trailer, it is adopted as `merged_sha` with no
  second merge; otherwise the intent is deleted and the merge is made again. Second route, for a machine
  that lost `local/`: when merging `H` leaves nothing to commit, a commit in `<base>..<target>` whose
  trailer names the story is adopted; with none, row 13's second text.
- D6. **The merge commit** (VSA-10). `squash`: `git merge --squash <H>`. `merge`: `git merge --no-ff
  --no-commit <H>`. Then, for both, `git commit -F <file>`: `pre-commit` and `commit-msg` run whatever
  the strategy, and `pre-merge-commit`, which only an automatic merge commit runs, never does (platform
  item 45). The message is `<id>: <title>`, a blank line, `Factory-Story:
  <id>`, and nothing else, so no squashed message or attribution line is carried over. Hooks and signing
  run as the person configured them. `rebase` is row 5: it would deliver commits with no trailer.
  Git that can run a project's hooks (the merge commit, the commit of a clean merge-in, the
  fast-forward in a checkout) goes through `local.hook_git`: `ctx.run` with shell-quoted arguments and
  `env={"LC_ALL": "C", "GIT_TERMINAL_PROMPT": "0"}`, so the hooks run in `command_env(root)` under the
  job's time limit (jobs.md D12; PL4-11). The built `gitrun.git(env=)` only adds to the tool's
  environment; every other git call uses it.
- D7. **A commit git refuses is `refused`, not `ci_failed`,** and `error` names the cause (`ORDER.md`
  rule 7): the Bash sandbox, when any git write of delivery ends in `Operation not permitted` (a
  `WorktreeError("not_permitted")` included); else the project's git hook, which sees only tracked
  files in the integration worktree and which no implementer can repair. Nothing has moved, and the
  next run merges again.
- D8. **No build is run on the merged tree**: local mode has no CI wait, and the wave gate measures the
  integrated head.
- D9. **What would wait for a person is refused until G4b.** `delivery.merge = "human"` is `configured`;
  a file of the story's change (`changed_files(root, <merge-base of target and H>, H)`) matching
  `delivery.protected_paths` is `protected_path`; with `dependency_changes = "human"`, one matching
  `paths.manifests` is `dependency`, an over-approximation until G4b.
  Merging these unasked breaks decision D15; `awaiting_merge` with no `merge` decision to settle
  strands the person. The shipped defaults therefore refuse a story that touches a manifest or
  `CHANGELOG.md`: E2's sample story touches neither (PL4-20; `ORDER.md` section 8).
- D10. **Conflict** (VSA-18). The integration worktree is reset to `old`. Then, in the story worktree,
  `git merge --no-ff --no-commit <old>`: conflicts stay as markers with the merge in progress, which
  `fix-findings` step 5 of the step 5 plan concludes with `git add` and `git commit`; `detail` names
  the files and says the merge is in progress there. If that direction merges cleanly, the tool commits
  it and says so in `detail`; a commit git refuses there is aborted (`git merge --abort`) and is
  D7's. The recorded base stays an ancestor, so the red record and the descent
  check hold. `changed_files` is made before the repair exists: `changed_files(root, reviewed_sha, H)`
  joined with `changed_files(root, <merge-base>, old)`, which the merge brings in.
- D11. **The lock** is `store.locked(root, "deliver.lock", wait=min(ctx.remaining(), 900))`, around the
  merge, the move and the merge-in. It is the outermost lock: `closure_update` takes `flush.lock` inside
  it, never the reverse. `stale` from `move_branch` starts the attempt again from reading the target,
  five times at most (row 14).
- D12. **After the record**: stop the product, then `remove_story(root, story)`, which keeps the branch,
  then `git update-ref -d refs/heads/<branch> <H>`, which deletes the branch only while it is still at
  `H` and asks no merged check (a squashed branch is unmerged to git). A removal that fails leaves
  `merged` standing, with the reason in `detail`.
- D13. **Flush once, after a merge.** Other outcomes wait for the checkpoint's flush. A failed flush is
  logged and does not change `merged`: the checkpoint reports it.
- D14. **One event type for a merged story**: `pr.merged`, with an empty `url` in local mode.

## 6. Findings (`rewrite/findings/by-package/G4a.md`)

- AU-S2-01: in code the refusal list exists once, `checks.py`, and nothing restates it.
  `test_refusal_order_names_the_first`. The spec text is the orchestrator's, here and in the next.
- AU-P2-13: the trailer is the constant `TRAILER`, used by the message and by adoption.
  `test_merge_message_is_title_and_trailer_only`.
- VSA-09: 3.4, `delivery.state` with `run` and `detail` in the closure.
  `test_every_outcome_is_recorded_in_closure`. The mapping to a blocked step is S3's second part.
- VSA-03: the list is G3's `changed_files`, renames included. `test_rename_lists_both_names`.
- VSA-10: `move_branch` re-reads after a refusal (`stale` against `dirty`); D6 names the message, D11 the
  lock. `test_ff_refused_because_branch_moved_merges_again`, `test_move_branch_three_answers`.
- VSA-18 (left-for-builder): D10. `test_conflict_leaves_merge_in_progress_in_story_worktree`,
  `test_deliver_after_conflict_repair_merges`.
- VSA-01: not at slice depth (G4a with `pr` mode, S2a for the red lookup). Local mode never rebases;
  `test_conflict_merge_in_keeps_base_and_red_record` shows the merge-in does not raise it.
- P3REQ-03: not at slice depth (G4a with the CI wait; S3, G5). Local mode never reads `delivery.ci`.

The step 5 plan's D18 holds (D10); what this plan asks of it and of W2 is in `ORDER.md` section 7.

## 7. Tests

`delivery_fixtures.deliverable(tmp_path, *, kind="feature", demo=False, strategy="squash")` starts from
`story_ready` (real init, real survey), commits a change in the story worktree, and brings the story to
`evidenced` through the stores alone: `evidence_write` for red, green and demo on the head, and a
converged `verdict` in the closure; `passes=[...]` stores review passes with `store_step` instead. No
suite runs. `work()` is called in process with `job_context(...)`; kill and race tests replace the
no-op `local._point(name)`, called at `merged`, `intent`, `moved` and `recorded`.

`test_delivery_checks.py`:

- `test_refuses_unknown_run_ended_run_and_foreign_story`: closure and git untouched.
- `test_refuses_missing_worktree_and_rewritten_history`; `test_refuses_pr_mode_and_rebase_strategy`;
  `test_refuses_dirty_story_worktree_but_not_the_test_report`.
- `test_red_required_only_for_kinds_with_a_red_gate`: a `chore` passes without one.
- `test_refuses_when_green_does_not_cover_head`; `test_refuses_without_converged_verdict` (both texts;
  uncounted results are named).
- `test_review_stale_gives_scoped_fields`: the range patch, and the whole patch untouched.
  `test_rename_lists_both_names`; `test_stale_on_a_clone_uses_the_closure_verdict`.
- `test_verdict_and_green_cover_by_patch_id`: an amended message still merges.
- G3's D8, both ways: `test_computed_verdict_at_head_overrules_closure` (stored passes not converged
  at the head, closure converged there: row 9, not stale); `test_closure_covers_when_computed_is_older`
  (merged).
- Row 10: `test_demo_true_without_evidence_refuses`; `test_demo_table_built_through_registry`;
  `test_demo_failure_and_unregistered_gate_refuse`.
- `test_needs_person_three_kinds_refuse`: `needs_person` is set. `test_refusal_order_names_the_first`.

`test_delivery_local.py` (the first five are the cases `commands/story-deliver.md` lists):

- `test_checkout_on_target_clean_is_fast_forwarded`.
- `test_overlapping_change_is_checkout_dirty`: nothing moved, the checkout is named; after a stash the
  next call merges.
- `test_untouched_change_survives_the_merge`; `test_branch_checked_out_nowhere_uses_update_ref`;
  `test_branch_moved_between_read_and_move_merges_again`.
- `test_ff_refused_because_branch_moved_merges_again`; `test_move_branch_three_answers`.
- `test_branch_that_keeps_moving_refuses`; `test_story_head_moved_during_merge_rechecks`: row 14, D4.
- `test_merge_message_is_title_and_trailer_only`: a `commit-msg` hook ran and saw nothing else.
- `test_merge_hooks_run_in_the_product_environment`: a `pre-commit` hook sees no `VSDD_*` name and not
  the tool's `VIRTUAL_ENV`. `test_both_strategies_run_pre_commit_never_pre_merge_commit`.
- `test_merge_strategy_merge_keeps_story_commits`; `test_hook_rejection_refuses_and_moves_nothing` (naming
  `git hook pre-commit`; then the next call merges);
  `test_merge_left_in_integration_worktree_is_cleared`.
- `test_sandbox_refusal_names_the_sandbox` (git patched: `refused`, nothing moved, then the next call
  merges); `test_refused_merge_in_commit_is_aborted`.
- `test_conflict_leaves_merge_in_progress_in_story_worktree`: markers and `MERGE_HEAD`; the integration
  worktree clean at `old`; the target unmoved; `changed_files`.
- `test_conflict_answered_again_while_merge_in_progress`;
  `test_conflict_merge_in_keeps_base_and_red_record`; `test_deliver_after_conflict_repair_merges`.
- `test_kill_before_intent_merges_once`; `test_kill_after_intent_before_move_merges_once`;
  `test_kill_after_move_adopts_the_recorded_merge`; `test_kill_after_record_finishes_cleanup`: each
  raises at one point, runs again, and finds one merge commit and `merged`.
- `test_lost_intent_adopts_by_trailer`; `test_empty_merge_without_trailer_refuses`;
  `test_lock_held_refuses_loudly`.
- `test_worktree_and_branch_removed_product_stopped` (a squashed branch goes); `test_moved_branch_is_kept`.

`test_delivery_cmd.py`, through S1's `main` and once through `py/launch.sh cli`:

- `test_cli_merges_and_prints_every_field`: exit 0; closure, event and flush are there.
- `test_cli_pending_then_result` (`--wait 0`); `test_cli_refused_exits_1`;
  `test_cli_usage_exits_2_with_whole_object`; `test_cli_again_after_merge_is_merged`;
  `test_result_of_ended_run_is_not_served`.
- `test_job_crash_is_loud`, the loud-failure test: a work function that raises gives `ok: false`,
  `state: ""`, an error naming the log, exit 1, and no branch moved.
- `test_every_outcome_is_recorded_in_closure`; `test_flush_failure_leaves_merged`.

## 8. Order of work

Before the builder starts (`ORDER.md` section 2): every step 3 unit and `gates`, `verdicts`, `rules` and
`survey-2` have reported; no unit runs beside this one. A G2 or G3 name that differs from section 4 is
reported, never copied.

1. `delivery_fixtures.py`; `checks.py` with `test_delivery_checks.py`.
2. `local.py`: `move_branch`, then the merge and the intent, then the conflict merge-in, with tests.
3. `deliver.py`: resumption, the record, cleanup; the kill tests.
4. `cmd_story_deliver.py` and `test_delivery_cmd.py`.
5. Every check of `ORDER.md` section 2.

Each kill point of D2, D5 and D10 has a test. One point has none: a kill inside git's own fast-forward
of the person's checkout, after which the next call answers `checkout_dirty`, naming that checkout.
