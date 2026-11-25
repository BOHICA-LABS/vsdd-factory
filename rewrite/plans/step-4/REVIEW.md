# Step 4 plans: cross-plan review

Reviewed: `gates.md` (G2), `verdicts.md` (G3), `survey-2.md` (S3, second part), `delivery.md` (G4a), `rules.md`
(H5, H7). Read against `rewrite/plans/step-3/` (ORDER.md, RESOLUTIONS.md and the seven plans), the step 5 plan
(`agents.md` sections 2, 4, 6 and 8, `RESOLUTIONS.md`), the spec files at the seams (`topics/story-script.md`,
`workflows/story.md`, `commands/story-deliver.md`, `gate-green.md`, `gate-demo.md`, `rules/step-results.md`,
`rules/tests-first.md`, `topics/verdicts.md`, `topics/evidence-lookup.md`, `topics/state-api.md`, the closure,
evidence and finding schemas), the ledger entries triaged to these packages, and the code under `next/`.

State of `next/` when read: S1 (`cli/dispatch.py`, `state/gitrun.py`, `mount`, `artifacts`, `cmd_*`), G1
(`gates/jobs.py`, `job_worker.py`, `env.py`) and most of the first part of H1 (`hooks/model.py`, `registry.py`,
`shell.py`, `gitfs.py`, `main.py`, `emit.py`) are built. Records, the survey, hook-runner-b and jobs-b are not:
`vsdd/state/` has no `store.py`, `gate_evidence.py` or `story_unit.py`, and `tests/hooks/` has no
`test_rule_cases.py`. Where this review asks a step 3 unit for something, that unit has not been built yet, so
the request can still go into its build.

## Verdict

Not ready to build as written. Three blockers:

- grounding is planned twice, with two different rules (PL4-1);
- nobody is allowed to make the review-lookup edit that three plans rely on (PL4-2);
- the rules unit cannot pass the case-coverage test it inherits (PL4-3).

There are also eight major mismatches. Four of them are where the second part of S3 and G4a read G3 (PL4-4,
PL4-5), the patch file (PL4-6) and the changed-file list (PL4-7). PL4-6 and PL4-7 are both silent or nearly
silent on the repair path.

Once those are settled the order is: G2, G3 and H5/H7 at the same time; then S3's second part; then G4a.

## Findings

### PL4-1 (blocker) Grounding is defined twice, with different rules, and both plans own the same test file

- **verdicts.md (G3)** owns `vsdd/state/grounding.py`, with `ground(tree, finding)` and
  `ungrounded(tree, findings)`, and `tests/unit/test_grounding.py`. It says "H7 calls `ungrounded(...)` ... No
  other code grounds a finding."
- **rules.md (H7)** owns `vsdd/grounding.py`, with `ground(location, *, tree, project_root)`, and the same
  `tests/unit/test_grounding.py`.

The two differ on what counts as grounded:

| Point | G3 | H7 |
|---|---|---|
| Reason texts | `path is outside the reviewed tree: <p>`, `file not found: <p>`, `lines <a>-<b> are outside <p> (<n> lines)`, `quote not found in <p>` | `no location`, `path not found in <tree>`, `lines <a>-<b> outside <path>`, `quote not found in <path>` |
| Empty `location.path` | Exempt (G3 D6) | `no location` is a failure |
| A path starting `.factory` | Resolved against the story worktree only | Resolved against the project root |

`finding.schema.json` (`location.path`: "the project root for files under .factory/") and
`rules/step-results.md` item 4 ("the project root for specification files") both side with H7 on the last row.
The story worktree has no `.factory/` in it. So with G3's function, a finding that cites the story file or a
contract is marked `grounded: false` on its second return, which `judge()` turns advisory. A spec gap or a
contract violation found by the adversary would then drop out of the verdict silently.

The two builders would also write the same test file.

**Resolution.** Keep one module with one test file. Recommended: the rules unit owns `vsdd/grounding.py`.
- It is the only caller: G3 D5 has the verdict trust the stored mark, so G3 needs no grounding code at all.
- This keeps H7 buildable beside G3.
- The orchestrator records that G3's "grounding" deliverable moved to H7.

Details:
- Adopt H7's `.factory` rule and G3's exemption for category `holdout`.
- Settle the empty path one way, in that one module and its test. Exempt is cheaper: `judge()` already holds
  such a finding advisory, so a block only costs the reviewer one retry.
- G3 drops `state/grounding.py`, `ungrounded`, `test_grounding.py` and its section 3.2. G3's `test_grounding`
  cases move into the one test file.

If G3 keeps ownership instead, G3 takes H7's signature and `.factory` rule, H7 drops its copy and waits for G3's
step 3 (grounding) before its step 4 (step-results).

### PL4-2 (blocker) The review lookup's delegation to G3 is unassigned, and three plans depend on it

- ORDER.md section 7 gives G3 the job of putting the computed verdict ahead of the closure's in
  `gate_evidence.evidence(project, "review", subject)`.
- That file belongs to the records unit (step 3), so G3 correctly refuses to edit it (verdicts.md section 4).
  Its `test_evidence_review_is_covering` fails until the edit is made.
- survey-2.md (the `reviewed` rung of `ladder`) and delivery.md (D3, refusal row 9) both assume the edit is made.

Without it, `evidence("review")` reads only the closure, and the closure's verdict is written at the
checkpoint. Inside the run that reviewed the story there is no covering verdict when `vsdd story deliver` runs.
Delivery D3 then falls to `verdict()`: a converged verdict whose head equals the story head is "an ancestor", so
the result is `review_stale` with an empty `changed_files`. The script then runs a pointless second loop with
nothing to account for, and the first run never merges.

**Resolution.** The orchestrator authorises exactly one cross-unit edit, by G3's builder:
- the `review` branch of `gate_evidence.evidence` becomes
  `from vsdd.state.verdicts import covering; return covering(project, subject)`, imported inside the function;
- records' `test_evidence_lookup` review case is updated to match.

Do not use an `ImportError` fallback to the closure: a missing module would then silently read the closure
alone. Record the edit in ORDER.md as sanctioned. Records is not built yet, so its builder can instead be told
now to leave a one-line seam (a `_review_lookup` function) that G3's builder replaces.

### PL4-3 (blocker) The rules unit cannot pass `test_gate_case_coverage`

hook-runner.md's `test_gate_case_coverage` requires every gate row to have a blocking and an allowing launcher
case in `tests/hooks/cases/*.toml`.

The case format (`fixture_cases/probe.toml`) names a payload and dotted `set` overrides only. No step 3 plan
promises a way for a case to build a project that has:
- `[paths]` globs;
- a story worktree;
- a story file with its kind;
- a branch checked out;
- records.

rules.md section 4 says this for `tests-first` and plans to "report the gap". The same limit hits the
`merge-guard` blocking case, which needs `develop` checked out in a git repository, unless
`runner_helpers.make_project(branch=)` is reachable from a case.

The unit would end with a failing test, and ORDER.md section 2 does not count such a unit as reported.

**Resolution.** `test_rule_cases.py` is not built yet. Ask the hook-runner builder to add a `project` table to
the case format, with:
- `factory.toml` keys by section;
- `branch`;
- `story` (makes `.worktrees/<id>/`);
- optionally `fixture = "<name>"`, naming a builder function in `tests/hooks/rule_projects.py`, so rules can
  set up evidence and records.

Otherwise authorise the rules unit to extend `test_rule_cases.py` that way. Either way, state it in ORDER.md.

### PL4-4 (major) `vsdd.state.verdict` returns `dict | None` in two plans and never `None` in the third

- **G3 (3.3, D10):** `verdict()` always returns an object. With no counted pass it is `failed`/`no_result`,
  with `passes: 0` and `rounds_used: 0`.
- **survey-2 (section 4, 3.5):** expects `None` when no pass is stored, and keeps the stored closure verdict
  then.
- **delivery (section 4, D3):** falls back to the closure's verdict "when that is None".

Who loses what:
- **survey-2.** On a fresh clone, run records are local and absent while the closure verdict is tracked. The
  first checkpoint would overwrite a converged closure verdict with `failed`, 0 passes and `rounds_used: 0`.
  That drops the story from `reviewed` and resets the cap the next run reads.
- **delivery.** On a clone, the closure fallback is never reached, so a story that should be `review_stale` is
  `refused` instead.

delivery.md's `test_computed_verdict_wins_over_closure` only holds when the computed verdict's head is the story
head. With an older head, G3's `covering` lets the closure win (G3 D8). That test must be written to G3's rule.

**Resolution.** Keep G3's always-an-object contract (one shape for callers) and give "no pass stored" one test:
`verdicts.summary(...) is None`, which G3 already defines. survey-2 and delivery use that test, not `is None`
on `verdict()`. Delivery's review_stale fallback becomes:
- the computed verdict when its `passes > 0`, else the closure's;
- with the head an ancestor and not equal to the story head. An equal head with no cover is row 9, not an empty
  review_stale.

### PL4-5 (major) S3's second part re-implements three G3 functions with different rules

| What | G3 provides | survey-2 builds instead | Difference |
|---|---|---|---|
| Closure form of the verdict | `verdicts.summary()` | its own in `unit_outcome.assess` (3.5) | survey-2's form is never `None` (PL4-4); its `rounds_used` is the last loop's |
| The unit's `rounds_used` | `max(passes_used(), closure verdict's rounds_used)` | "the tool verdict's, else the closure verdict's, else 0" | see below |
| Waivers | `verdicts.waivers(project, subject)` | `story_unit.waivers(root, cycle, story)` | survey-2 says "G3 may call it", but G3 is built first and cannot |

The `rounds_used` difference matters because `converge()` sets `rounds_used = used + pass` and a later loop has
`used = 0`. After a delivery repair, the tool verdict's `rounds_used` is the loop-2 count, say 1, while loop 1
used 3. survey-2 would hand the next run's loop 1 `used = 1`, giving it two passes the cap does not allow. G3
D13 exists to prevent exactly this.

**Resolution.** survey-2 calls G3:
- `summary()` for the closure's `verdict`;
- `waivers()` for `waived`;
- `max(passes_used(), closure rounds_used)` for `rounds_used`, then its own owed-pass cap (`direct`) on top.

survey-2 defines no second `waivers` and no closure form. Its `tool_verdict` stays the one entry point to
`vsdd.state.verdict`. Add a survey-2 test of a story whose last loop is a later loop.

### PL4-6 (major, silent) `gate green --since` overwrites the whole-story patch that the next run's full review reads

- gates.md 3.5 writes the patch to `diff.<head12>.patch`, whatever the range.
- G2 D4 rewrites it "for the range asked on every call".
- The green record keeps that path in `diff_patch`.
- survey.md D16 sets the unit's `diff_patch` from the green record.

The failure:
1. In run 1, a conflict repair ends with `gate green --since <reviewed_sha>` at H2. `diff.<H2>.patch` now holds
   `<reviewed>..H2` only.
2. The loop-2 review does not converge, and the run ends.
3. In run 2 the story is at `green`, and the script runs a full loop 1 with `patch = u.diff_patch`. That is the
   narrowed patch, labelled `(base...head)`.

The reviewers review a fraction of the story and can converge on it. Nothing fails out loud.

delivery.md adds a third writer of patch files: it writes its own scoped patch (`diff.<reviewed12>-<head12>.patch`)
in `checks.py`, while gates.md 3.5 and G2's report say G4a calls `diffpatch.write_patch`.

**Resolution.** One patch writer, G2's `diffpatch.write_patch`, naming the file by its range:
- `diff.<head12>.patch` for base to head;
- `diff.<since12>-<head12>.patch` for `--since`.

The green record's `diff_patch` always names the base-to-head patch. A `--since` result's `diff_patch` names the
range patch. Delivery calls `write_patch(project, story, reviewed_sha, head, tree=...)` and drops its own writer.
This follows the spec's intent ("from the story's base, or from --since") without one name for two contents. Add
G2 `test_since_patch_leaves_whole_patch`.

### PL4-7 (major) "The files changed" has three definitions, and delivery's differs, so VSA-03 is not closed

- **G2** `diffpatch.changed_files`: `git diff --name-only --no-renames`.
- **G3** `verdicts.changed_files`: `git diff --name-only --no-renames`. Its D7 says this is "so that two callers
  cannot differ".
- **G4a** `checks.changed_files` (3.2): `git diff --name-only <reviewed_sha> <head>`, without `--no-renames`.

With rename detection on (git's default), a renamed file appears in delivery's list under its new name only. G3's
later-loop items hold both names. The script reviews delivery's list (`review(round + 1, d.reviewed_sha, 2,
d.changed_files, 0)`), so the tool's verdict always shows the old name in `coverage_missing`, never converges,
and delivery refuses every round of a `review_stale` that involves a rename.

**Resolution.** One definition. The state side may not import gates, so G3's `verdicts.changed_files` is the one
definition:
- delivery calls it;
- G2's `diffpatch.changed_files` either calls it (if G3 lands first) or runs the identical command, with a test
  in G3 or G4a that holds the two equal.

Delivery's conflict prediction (D10, the union with `<merge-base>..<old>`) is built on the same function.

### PL4-8 (major) The survey calls the gate functions without the wait and retry that G2's functions take

- survey-2 3.4 calls `fn(root, story, run)` and polls "every 2 s until `wait`".
- gates.md 3.4's `measure` takes `wait=None` and `retry=""` and attaches to the job. `jobs.attach` applies
  `clamp_wait(None) = 100`.

So each call blocks up to 100 s whatever the survey's remaining budget is. `--wait 0` blocks 100 s. Three gates
measured again can hold the survey for about 300 s before it answers pending. The pending answer's `retry` also
comes from G2's default, not from the survey's command line (survey-2 replaces it, which is fine).

survey-2's tests use `fake_gate`, so they would not catch this. records.md documents the registry signature as
`fn(project, subject, run)` alone.

P3REQ-06 (survey-2 section 6) is therefore resolved only for a slow suite, not inside the survey's `--wait`.

**Resolution.**
- survey-2 calls `fn(root, story, run, wait=<seconds left>, retry=inv.retry_command())`.
- The registry contract in records.md (and G2 3.4) gains the two keyword arguments.
- survey-2 adds one test that passes a real G2 function with `--wait 0` and gets the pending object at once.

### PL4-9 (major) The gate tests need the sample product in the story's base, and no helper provides it

G2 (section 4) needs `tests/unit/story_ready.py` to "let a caller commit product files to the integration
branch before the story is admitted". survey.md section 7 does not promise this.

`_repo.make_project` (built) commits `README.md`, `src/module.py` and `tests/test_module.py`, not the tally
sample. `story_ready` points `commands.test` at the sample runner, which is not in that project. Who needs the
real sample:
- G2's whole gate suite;
- survey-2's `test_real_gates_move_position` and `test_survey_remeasures_with_real_green_gate`.

Committing the sample after admission puts it in the story's diff: the base report then has no tests and every
test is "new".

**Resolution.**
- Ask the survey builder (not built yet) for `story_ready(tmp, *, product=<dir or callable>)`. It copies and
  commits the product, and sets `[paths]`, before the survey admits the story.
- G2's `gate_project.py` is the one sample-based builder on top of it.
- survey-2 reuses `gate_project.py` rather than writing its own.

### PL4-10 (major) The rules unit's sanctioned edits and its tests collide with S3's second part if run beside it

rules.md section 8 says the unit "can run beside the other four step 4 units". Two things stop that:
- `rule_projects.py` is built on `story_ready.py` and an in-process `vsdd survey story`, and survey-2 rewrites
  `survey.py`, `story_unit.py` and `checkpoint.py` (and the first part's tests). Running beside survey-2, the
  rules tests would break and pass with the other builder's half-made edits.
- Both G2 (schemas, then `tools/build.py`) and rules (registry rows, then `tools/build.py`) regenerate
  `prelude.js`, the toys and `hooks.json` in one run. `tools/build.py --check` and `check.py`'s
  `build_is_current` fail for whichever unit checks while the other has an edit not yet rebuilt.

**Resolution.**
- Run rules beside G2 and G3 only, and let it report before survey-2 starts.
- G2's step 1 (schemas and the build) lands before the rules unit's step 5. Failing that, each builder runs
  `tools/build.py` immediately before its own checks.

### PL4-11 (major) The delivery environment for git hooks does not exist in the built `gitrun.git`

- delivery.md (section 4) needs `gitrun.git(env=)` to take a whole environment, so the merge commit's hooks run
  in `command_env(root)` (jobs D12).
- The built `state/gitrun.py` does `full = dict(os.environ); full.update(env)`. It only adds.

So `VIRTUAL_ENV`, `PYTHONSAFEPATH`, `PYTHONUTF8`, `UV_RUN_RECURSION_DEPTH` and every `VSDD_*` name come back from
the tool's own environment. Only `PATH` is replaced, because it is a key `command_env` sets. A project's
pre-commit or commit-msg hook that runs Python then sees the tool's environment. Delivery D7 turns that into
`refused`.

**Resolution.** S1 adds a keyword to `gitrun.git`, for example `env_only: bool = False`, to use `env` as the
whole environment plus `LC_ALL` and `GIT_TERMINAL_PROMPT`. This is a small change to a built file and needs the
orchestrator's word. If that is refused, delivery passes `command_env(root)` with each scrubbed name set to `""`
and says so in its report. Only the first form is exact.

### PL4-12 (minor) Survey-2 and delivery disagree on the closure's delivery keys and on merge decisions

| Point | delivery.md | survey-2.md | Effect |
|---|---|---|---|
| Name of the delivery error | `delivery.error` (3.4), holding the `detail` for conflict and review_stale | reads `delivery.detail` (section 4, the stuck packet); its `story_progress.delivery` helper writes `detail` | each unit's tests pass alone; together, the stuck packet loses the delivery's error |
| Approved `merge` decisions | refuses every case that needs a person (D9) and never reads one | "Delivery merges a story whose `merge.<s>` is approved" | none in local mode: no `awaiting_merge` is ever written, so row 2 of 3.5 never opens a `merge` decision |

A story with `delivery.merge = "human"`, a protected path or a manifest change is refused every run and opens
`blocked.<s>` (stuck) after `failed_runs_before_stuck` runs.

**Resolution.**
- One key name: `delivery.detail`, the delivery data's own field name. Then `error` is not stored twice.
- survey-2 drops the merge-decision assumption for step 4.
- The `stuck` packet names `needs_person` and the error when `delivery.state` is `refused`.

### PL4-13 (minor) The records plan's id rule rejects `_base`, which G2's base reports use

- records.md section 3: a path segment must match `^[A-Za-z0-9]...` "or is `_standing`; else `unsafe_id`".
- The same plan gives `evidence_dir(project, subject)` "(`_base` for base reports)".
- G2 D7 and 3.3 write base reports under `evidence_dir(project, "_base")`.

**Resolution.** Tell the records builder to allow `_base` in `evidence_dir`, and test it.

### PL4-14 (minor) Delivery's requests of step 3 are not in any step 3 plan

Delivery depends on three things:
- `worktrees.remove_story(delete_branch=True)` forcing the deletion (`-D`), because a squashed branch is
  unmerged to git;
- `worktrees.set_integration` succeeding when the integration worktree holds `MERGE_HEAD`, which needs
  `git merge --abort` or `reset --merge` as well as the forced checkout;
- `closure-record.schema.json` naming the extra delivery keys.

survey.md and records.md promise none of these. The third validates today, because `delivery` has no
`additionalProperties`. Survey and records are not built yet.

**Resolution.** Hand the first two to the survey builder and the third to the records builder, now.

### PL4-15 (minor) G2 and G4a each write the clean-worktree rule

- G2 D5 (`gatelib`): `git status --porcelain -z --untracked-files=all`, less the report path.
- delivery.md (row 6): "if G2 exports its clean-worktree check, row 6 calls it; otherwise" `checks.py` writes
  its own.
- gates.md lists no such export.

There is the same split over demo:
- delivery row 10 reads `evidence("demo")` for a `demo: true` story and calls the registered `demo` function
  otherwise;
- gates.md 3.5 says "the demo gate holds" is `passed` of `gate_demo.measure` for every story.

**Resolution.**
- G2 exports `gatelib.dirty_files(tree, config) -> list[str]` as an interface, and delivery calls it.
- Delivery takes "the demo gate holds" from the registered `demo` function for both kinds. The function returns
  the passing record for the head when there is one.

### PL4-16 (minor) G2's jobs can answer one run with another run's result

- The job key has no run (jobs D11), and a G2 worker repeats `require_story` with the run of the call that
  started the job.
- If that run was taken over while the job was queued, the new run's agent gets `ok: false` "run ended". That
  costs one of `gated()`'s two attempts.
- Delivery D1 handles the same case by dropping a result made for another run.

**Resolution.** G2's command does the same as delivery D1: a result whose `run` differs from `--run` is not
served, and the command attaches again. This needs `run` inside the result, removed before printing.

### PL4-17 (minor) G3 replays two details differently from the script

- **Resolutions per key.** `converge()` builds `new Map(resolutions.map(...))` over the test-writer's entries
  followed by the implementer's, so the last entry for a key wins. G3 rule 4 says "disputed only when both say
  so". The two differ when the test-writer says fixed and the implementer says disputed.
- **`passes_used`.** G3 counts "distinct pass numbers with a reviewer record" over every run. Read literally,
  this undercounts across runs: three passes in run 1 and one in run 2 give 3, not 4.

**Resolution.**
- Join fix records in the script's order (test-writer, then implementer) and let the last entry win, with one
  conformance case.
- Count distinct `(run, pass)` pairs of loop 1.

### PL4-18 (minor) Built now, though the slice path never reaches it

- **`vsdd review show`** (G3 D12): no step calls it.
- **The `decisions` rule** (H7), with `UserPromptExpansion` wiring: it cannot fire until K1 ships the `decide`
  skill. A person settles with `vsdd decide`, which records builds.
- **`packets.notify` with `notify.channel = "command"`** (survey-2 3.6): the default channel is `none`.

Each is small and tested. The build plan says a package is finished for the slice "when the files the story
path touches pass their tests", and these touch none.

**Resolution.** The orchestrator either defers the three or accepts them as named exceptions. If `review show`
stays, its `--include-walled` check uses `inv.require_person`, or says why it downgrades rather than refuses.
G3 3.4 writes its own `caller_key is None and stdin_tty` test, a second person check that ORDER.md rule 5
forbids.

### PL4-19 (minor) A conflict repair can strand on frozen tests

- After delivery D10 merges the target into the story worktree, a frozen test file the target changed fails
  G2's frozen-hash check (D13).
- The repair step runs only the implementer, with `gate green --since`, which cannot re-freeze.
- tests-first blocks the implementer from resolving conflict markers inside a frozen test file.

**Resolution.** Leave this as it is for the slice. Record it for W2 and G4a: a repair whose `changed_files`
touch frozen tests needs a test-writer step with `--refreeze`.

### PL4-20 (minor) Delivery's default configuration refuses some ordinary stories in the slice

- `config.py` defaults `delivery.dependency_changes = "human"` and puts `CHANGELOG.md` in `protected_paths`.
- With delivery D9, any story whose diff touches a manifest file or the changelog is `refused`, with
  `needs_person` set, until G4b.

**Resolution.** Record that E2's sample story must touch neither, and keep delivery's open question for the
person.

### PL4-21 (minor) Step 3 files edited by step 4, beyond what the step 4 brief allows

- **survey-2:** `story_unit.py`, `survey.py`, `checkpoint.py` and the first part's tests. This is sanctioned by
  survey.md section 1 and ORDER.md section 7 ("provisional ... replaced by the second part").
- **rules:** four rows between the anchors of `hooks/registry.py` and `test_shape` in
  `tests/lint/test_hooks_wiring.py`. This is sanctioned by hook-runner.md 3.3, and the plan's own matcher pins it.
- **G2:** C1's `gate-result` and `wave-gate-data` schemas and E1's `junit_runner.py`. These are not step 3 files,
  but they are other packages' files, and G2 asks for that permission itself.

Each plan states exactly what it adds, and no two plans touch the same file.

**Resolution.** The orchestrator records these as approved exceptions to "never change a step 3 file". It also
confirms G2's three edits: no other unit edits them, and the generated prelude follows from the build.

### PL4-22 (minor) The `agent_type` a workflow step's hook payload carries is still unproven

- G3 counts a review pass only when the stored `agent_type` is exactly `vsdd-factory:<owner>`.
- H7 stores the payload's value unchanged.
- The prelude dispatches with `agentType: 'vsdd-factory:' + type`.
- No probe shows the `StructuredOutput` payload of a workflow step carrying that string.

If it differs (for example `workflow-subagent`, which `hooks/model.py` already maps to `other`), no pass counts
and every delivery is refused. That failure is loud, but it is total.

**Resolution.** Add it to B8's probes before E2's first run, beside H7's open probes (a rewrite adding
`grounded`, and allow-plus-rewrite in the default mode).

### PL4-23 (minor) Corrections the step 5 and step 6 plans must take

**For `rewrite/plans/step-5/agents.md` (the methods):**

- `test-fixes`: `--refreeze` works only as the whole single-segment command of the test-writer's call, never
  wrapped as `cd x && vsdd ...`. It is refused without the caller marker (G2 D11). A changed frozen test that is
  skipped fails the re-freeze. A re-freeze with nothing changed passes, named `red`, which meets agents.md
  section 8.
- `fix-findings` step 5: when delivery's `detail` says the tool concluded the merge (delivery D10), there is no
  marker to resolve. Run the green gate; do not return `blocked`.
- `demo`: the evidence directory is the unit's `demo_dir` (`.factory/demo/<cycle>/<story>/`, survey-2 D8), the
  directory G2's demo gate reads.
- M1 and the review methods: contracts come as `bc_paths` (survey-2), and `location.path` under `.factory/` is
  relative to the project root (`finding.schema.json`; PL4-1).

**For W2 (step 6), from these plans:**

- Pass `u.demo_dir`, `u.bc_paths`, `u.conventions` and `u.guidance` (S5P-N1, S5P-N11).
- Take `coverage_missing` from the `review-story-diff` result alone (S5P-N9, G3 D3).
- After a repair, take the later loop's items from the `--since` result's `changed_files`, not `d.changed_files`
  (VSA-03).
- Refresh `test_output` wherever it refreshes `patch` (P3BLD-16).
- Use the range patch of PL4-6.

The orchestrator should record these as ledger outcomes.

## Seams checked and found to fit

- **Command registration.** Every new command is a `cmd_<family>[_x].py` in `vsdd/state`, `vsdd/gates` or
  `vsdd/delivery`, which are in `COMMAND_PACKAGES` in the built dispatcher. No two share words. `failure`
  objects are schema-valid with `empty_of`: `{"gate": name}` for gate-result, `{}` for delivery-data, where
  `state: ""` is in the enum.
- **Pending answers.** The dispatcher does not validate output against `Command.output`. So gates, delivery and
  the survey may print the `pending-output` object. The relay prompt re-runs on `state: "pending"`. survey-2
  correctly changes the survey's own schema check for that one case.
- **Gate registry.** Gate modules are named `gate_<name>.py` and register at import (records, ORDER.md section 7).
  The demo function is direct and never attaches, so delivery's job may call it. Red and green are never called
  from inside another job.
- **The step-results rule and G2.** The rule looks records up by subject, gate and sha (VSA-11). G2 writes a
  record for every passing result, including the no-change re-freeze and `--since`. A re-freeze is a `red`
  record holding the complete frozen set, which records' `refreeze_incomplete` enforces. `frozen[].file` is
  POSIX and relative to the worktree, which is what tests-first compares.
- **Store signatures.** `store_step(project, result, agent_type, head, *, agent_id="")`, `settle_decision`,
  `parse_decide`, `decision_open`, `steps_list`, `closure_update`, `closure_find`, `run_require` and `patch_id`
  are used with records.md's signatures throughout.
- **Jobs.** `JobSpec`, `attach(..., wait=, retry=)`, `JobContext.run(command, cwd=, output=)`, `command_env`,
  `stop_products(root, subject=)` and `clamp_wait` are used as built in `gates/jobs.py`.
- **Hooks.** `Ctx` fields (`raw`, `tool_input`, `target_literal`, `row`, `story_at`, `branch_at`, `project_at`,
  `project.hooks_dir`), `shell.Segment` and `gitfs.head_sha` and `branch` exist as rules.md uses them.
  `TOOL_ATOMS` and the `UserPromptExpansion` row with `^vsdd-factory:decide$` are in `registry.py`.
  `ctx.classes_of` comes with hook-runner-b.
- **Policy.** `walls.toml` gives the test-writer `source:before-red`, `tests:frozen` and the `refreeze` vsdd group,
  which G2 D11 and tests-first read. The `[demo-evidence]` row gives the directory G2 D14 checks.
- **Delivery and the checkpoint.** The checkpoint closes the unit from `merged_sha` and reads `delivery.state`
  where `delivery.run` is its own run (VSA-09). Delivery never writes the closure's `state`, `position` or
  `stages`. The lock order is `deliver.lock`, then `flush.lock`.
- **Step 5 assumptions.** Met: a re-freeze result is named `red` and passes with nothing changed;
  `gate green --since` is named `green`; `sha` is the full head, so `gated()`'s `g.sha === r.head` holds;
  failures are always given; the conflict merge-in leaves markers in the story worktree (VSA-18, step 5 D18).
- **Ledger claims that hold as stated.** P3SEAM-15, P3SEAM-16, VSA-12, P3BLD-16 (the gate half), VSA-05 (with
  PL4-17's detail), VSA-07 (G3 D5 with H7 D13), VSA-10, VSA-11, VSA-18, S5P-N1 and S5P-N11 (the tool half).
- **Ledger claims that do not yet hold.** VSA-03 (PL4-7, and PL4-6 for the patch); P3REQ-06 (PL4-8); VSA-09
  (PL4-12); P3OPS-09's G3 half (PL4-5: the waivers must be G3's, applied in one place).

## Build order

Before any step 4 unit: all seven step 3 units have reported, and the step 3 requests above are in their builds:
- PL4-9 (survey);
- PL4-13 and PL4-14 (records and survey);
- PL4-3 (hook-runner);
- PL4-11 (S1, if approved).

The orchestrator has also settled PL4-1 and PL4-2.

| Stage | Units | Why |
|---|---|---|
| A | `gates` (G2), `verdicts` (G3), `rules` (H5, H7), at the same time | Disjoint files once PL4-1 gives grounding one owner. G2's schema step and its `tools/build.py` run land before rules' step 5 (PL4-10). G3's builder makes the PL4-2 edit. |
| B | `survey-2` (S3, part 2) | It calls G2's registered functions and G3's `summary`, `waivers`, `passes_used` and `verdict`, and edits the survey files that the rules unit's tests run. So stage A must have reported. It regenerates the prelude after its `story-unit` schema edit, after G2's. |
| C | `delivery` (G4a) | Its fixtures run the real survey and `story_unit.build`, which survey-2 changes. It calls G2's `demo` function and patch writer, and G3's verdict, cover and `changed_files`. |
