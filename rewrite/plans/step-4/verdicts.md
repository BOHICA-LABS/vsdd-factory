# Step 4 plan: verdicts (package G3)

Paths are under `next/`. `P` is `plugins/vsdd-factory`, `S` is `P/py/vsdd/state`, `T` is `tests`. Spec files
are under `rewrite/spec/`. Brief: `packages/G3.md`. Findings: `rewrite/findings/by-package/G3.md`.
`ORDER.md` is this directory's.

## 1. Scope

Built now, for the perimeter `story` only: the convergence rule in Python (`judge`, and a replay of the
prelude's `converge()` over recorded passes); the verdict rebuilt from a story's stored step records
(`topics/verdicts.md`); whether a converged verdict covers a commit; the closure form of a verdict; the
unit's waivers and passes used; the one definition of "the files changed"; the review lookup's
delegation in records' `evidence` (section 4); and the conformance cases with a Node and a pytest
runner (`packaging/tests-conformance.md`). Grounding is not here: the rules unit owns the one check
(rules.md 3.5), and the verdict trusts the stored mark (D5; PL4-1).

Left for when the build widens:

- `vsdd review show` and `review prepare`: nothing on the story path calls them (K2, the `review` skill;
  PL4-18).
- `holdout_findings` in Python and the two holdout-score cases (G6a). The perimeters `spec`, `plan`, `wave`
  and `release`, with their review methods and items (G5, G6a, G6b).
- Walled findings in full: the evaluation file, a waiver that names a walled key (S2a widened, H3).
  Records refuses to store a walled finding today.
- The flag item F-ITEMS (F packages). `direct` after a cap (P3OPS-09: B4a, W2, S3).

## 2. Files

All owned: no new schema, and no change to the prelude, the harness or `tools/`. One edit is made in a
step 3 file, the review lookup of section 4 (approved: `ORDER.md` section 4). There is no grounding
module or `test_grounding.py` here: both are the rules unit's.

| File | Holds |
|---|---|
| `S/convergence.py` | `HARD`, `REACH`, `judge`, `replay`. Pure, no I/O. `__all__ = []` |
| `S/verdicts.py` | `REVIEW_METHODS`, `ARBITER`, `rebuild` (pure) and the functions of 3.3. `__all__ = ["verdict"]` |
| `T/conformance/cases/<name>.json` | One case per file (3.5; names in section 7) |
| `T/conformance/conformance.test.mjs` | The Node runner |
| `T/conformance/test_conformance.py` | The pytest runner; it also starts the Node runner |
| `T/unit/test_verdicts.py` | Section 7 |

## 3. Interfaces this unit provides

`project` is what records.md section 3 allows: a `Project`, an object with `.root`, a `str` or a `Path`.

### 3.1 `vsdd.state.convergence`

- `judge(findings, perimeter, waived) -> dict` with `blocking`, `advisory`, `deferred`, `spec_gaps`: the
  prelude's `judge()`, same order of findings. `KeyError` for an unknown perimeter.
- `replay(loop, trace=None) -> dict`: a verdict, every field of `P/schemas/verdict.schema.json` and
  `rounds_used`. `loop` is

  ```
  {perimeter, subject, cap, used, waived: [key], arbiter: bool,
   passes: [{review: {ok, blocked, head, findings, claim_checks, coverage_missing} | null,
             fix: {ok, resolutions: [{key, resolution, commit, note}]} | null}, ...],
   arbitration: {rulings: [{key, ruling, reason}]} | null}
  ```

  `review` and `fix` are what the script's `c.review` and `c.fix` return (`topics/prelude-converge.md`).
  With `trace` a list, one entry per pass is appended: `{pass, claims, gaps}`, plus `fix: [key]` when the
  fixer is asked and `arbitrate: {keys: [key], notes: [[key, note]]}` when the arbiter is.

It is `converge()` of `workflow-lib/prelude.js` statement for statement, with the same names, and three
differences:

1. `c.review(pass, …)` is `passes[pass-1].review`, `c.fix(…)` is `passes[pass-1].fix`, and `c.arbitrate`
   exists when `arbiter` is true and returns `arbitration` (`null` reads as no rulings).
2. It walks every recorded pass. Where the script returns, the replay notes that outcome and, if a pass
   follows, goes on: with no claims after a pass that left nothing open, otherwise with the claims built from
   the open findings and the pass's fix (a missing fix reads as no resolutions). A pass whose review is
   `null`, not `ok` or `blocked` changes only the outcome. The arbiter is applied once, at the first pass
   where `used + pass >= cap` with findings open. The verdict is the state after the last pass.
3. If the last pass leaves findings open where the script goes on to a fix, the verdict is `failed`,
   `no_result` with them in `blocking`: the fix failed or the loop was cut off. With no pass at all it is
   `blocked`, `cap` when `used >= cap` (as the script), else `failed`, `no_result`.

A claim holds `path`, `line_start` and `line_end` only where the location has them (the script's undefined
members vanish in JSON), and a `holdout` claim none.

### 3.2 Grounding: not in this unit (PL4-1)

`vsdd.grounding.ground(finding, *, tree, project_root)` is the rules unit's (rules.md 3.5): its
`step-results` rule calls it when a result is stored and sets `grounded: false` on a finding that still
fails. No code here grounds a finding: `judge` reads the stored mark.

### 3.3 `vsdd.state.verdicts`

`REVIEW_METHODS = {"review-story-diff": ("adversary", True), "security-review": ("security-reviewer", False)}`:
method, owner, and whether its `coverage` accounts for the items. `ARBITER = ("arbitrate", "architect")`. A
stored type owns a method when it equals `"vsdd-factory:" + owner`.

- `verdict(project, subject) -> dict`, also `vsdd.state.verdict`: the verdict of the story's last loop, valid
  against `verdict.schema.json`; findings keep their stored `grounded` mark. With no counted reviewer record:
  `passes` 0, `rounds_used` 0, `outcome` `failed`, `reason` `no_result`, `head` `""`, empty lists. It never
  returns `None`: "no pass is stored" is tested as `summary(...) is None` (PL4-4). Raises records'
  `StoreCorrupt` for an unreadable step record and S1's `ArtifactError` when loops exist and the story
  file is gone. Any id is read as a story id.
- `covering(project, subject, *, head=None) -> dict | None`: the review lookup of `topics/evidence-lookup.md`,
  and what `evidence(project, "review", subject)` returns (section 4).
  `head` defaults to the head of `.worktrees/<subject>/`. A verdict covers `head` when its `head` is that
  commit, or `patch_id(tree, base, its head)` is non-empty and equals `patch_id(tree, base, head)`; `base` is
  the closure's `delivery.base`. If the computed verdict covers `head`: its closure form when it is
  `converged` and `chain` holds, else `None`. Otherwise the closure record's `verdict` when that is
  `converged` and covers `head`. `None` without a worktree, a closure or a base.
- `summary(project, subject) -> dict | None`: the closure form of the computed verdict, which the checkpoint
  stores as the closure's `verdict`: `perimeter`, `head`, `patch_id` (`""` when it cannot be computed),
  `passes`, `rounds_used` (the value of `rounds_used` below), `outcome`, and the counts `advisory`,
  `dismissed`, `waived`. `None` exactly when the story has no counted reviewer record in any run: the one
  test of "no pass is stored", and the closure then keeps what it has.
- `waivers(project, subject) -> list[str]`: from every decision record with id `finding.<subject>`, status
  `approved`: choice `waive` gives each word of `values["key"]`, `waive-coverage` gives
  `coverage:<subject>:<item>` for each word of `values["item"]`. The story unit's `waived` is this list;
  no other unit builds one (PL4-5).
- `passes_used(project, subject) -> int`: the number of distinct `(run, pass)` pairs of loop 1 that hold a
  reviewer record, over every run.
- `rounds_used(project, subject) -> int`: the larger of `passes_used` and the stored closure verdict's
  `rounds_used` (0 without one), so a clone with no run records never lowers it. It is the story unit's
  `rounds_used`, which the script passes as `used`, and the closure form's.
- `changed_files(project, since, head) -> list[str] | None`: `git diff --name-only --no-renames <since> <head>`
  at the project root, sorted; `None` when git cannot say. The one definition of "the files changed"
  (VSA-03, PL4-7): G2's `--since` result and delivery call it and write none of their own.
- `rebuilt(project, subject) -> dict`: `{verdict, loops: [{run, loop, input, verdict}], skipped: [{run, step,
  why}], chain: bool, passes_used, walled: [key]}`; `step` is `<name>.<attempt>`.
- `public_verdict(project, subject) -> dict`: `verdict()` with each walled finding (one returned by
  `vsdd-factory:scenario-reviewer` or located under `.factory/holdout/`) replaced by
  `{key: "walled:<12 hex of sha256(key)>", severity, walled: true}`: all a packet or a closure may carry.
- `rebuild(records, *, subject, cap, waived, acs, changed) -> dict`: the pure core, same result as `rebuilt`.
  `records` are step records in stored order; `changed(since, head)` returns a list or `None`.

How `rebuild` reads records (`topics/verdicts.md`; it never parses a step name):

1. It takes records whose `step.workflow` is `story` and `step.subject` the subject.
2. Reviewer record: it carries `step.pass`, its method is in `REVIEW_METHODS` and its `agent_type` owns it.
   `done` with `data` valid against `review-data`: a review record. `blocked`: its pass is blocked. `done`
   with invalid data: its pass is not `ok`. Any other status is ignored. Of several for one run, loop, pass
   and method, the highest `step.attempt` counts.
3. Arbitration record: `done`, method and owner of `ARBITER`, data valid against `ruling-data`; per run and
   loop the highest attempt.
4. Fix record: `done`, carries `step.pass`, not a method of 2 or 3, data valid against `fix-data`, and one
   entry of `gates` has `passed` true and `sha` equal to the record's `head`. A pass's fix is the
   resolutions of its fix records joined in the script's order, those with method `test-fixes` first,
   then the rest; for a key named more than once the last entry wins (D6).
5. A record with `step.pass` that 2 to 4 leave out is listed in `skipped` with `why`: `agent_type`, `data`,
   `gate` or `attempt`.
6. A loop is `(run, loop)`, an absent `step.loop` reading as 1; loops are ordered by the run's first record,
   then `loop`; passes by number.
7. A pass's `review`: `blocked` if a reviewer record is; `ok` when it has a review record, none invalid, and
   one `reviewed_sha`, which is `head`; `findings` and `claim_checks` joined in `REVIEW_METHODS` order;
   `coverage_missing` the loop's items that no record of an accounting method marks `checked` or
   `not_checkable`.
8. Items: `acs` for a loop 1. For a later loop `changed(a, b)`, `a` the head of the verdict of the loop
   before it and `b` the head of its own first pass; `None`, or no loop before it, makes its passes not `ok`.
9. `cap` for a loop 1, 2 later; `used` is the loop-1 passes of earlier runs for a loop 1, 0 later;
   `arbiter` true. `chain` is true when, from the last loop back to the nearest loop 1, every verdict is
   `converged`.

### 3.4 What the other units take from this

- S3, second part: a story is at `reviewed` when `evidence(project, "review", subject)`, that is
  `covering(...)`, is not `None`; the closure's `verdict` is `summary(...)`, and the stored one stays when
  that is `None`; the unit's `waived` is `waivers(...)` and its `rounds_used` is `rounds_used(...)`; a
  packet's findings come from `public_verdict`; the verdict's `reason` maps to a decision as
  `topics/human-stops.md` says.
- G4a: delivery merges only when `covering(project, story, head=<head>)` is not `None`. Otherwise `last` is
  `verdict()` when `summary()` is not `None`, else the closure's `verdict`. `review_stale` needs `last`
  converged with its `head` an ancestor of the head and not the head; then `reviewed_sha` is that `head`
  and `changed_files` is `changed_files(project, reviewed_sha, <head>)`. Anything else is `refused`.
- W2 (step 6): `coverage_missing` comes from the `review-story-diff` result alone; a later loop's `must` is
  the list at the head its first pass reviews; `used` is the unit's `rounds_used`.

### 3.5 Conformance cases

`T/conformance/cases/<name>.json`, one object: `name` (the file stem), `about` (one line), `kind`, `input`,
`expect`. Whole objects are compared after a JSON round trip.

| `kind` | Run by | `input` | `expect` |
|---|---|---|---|
| `judge` | Node, Python | `{findings, perimeter, waived}` | `{result}`: what `judge` returns |
| `loop` | Node, Python | the `loop` of 3.1 | `{verdict}`, and `trace` when the case checks it |
| `records` | Python | `{subject, cap, waived, acs, changed: {"<since>..<head>": [path]}, records}` | `{verdict, loops: [{run, loop, outcome, reason}], skipped: [{step, why}], chain, passes_used}` |

A `loop` case ends where `converge()` returns. `records` holds step records as records.md stores them.

## 4. Interfaces this unit needs

Under `next/` today: `vsdd.schema` (`load_registry`, `validate`) with the `review-data`, `fix-data`,
`ruling-data`, `finding` and `verdict` schemas; `vsdd.config` (`convergence.passes`); `judge` and `converge`
of `workflow-lib/prelude.js`; `T/workflows/harness.mjs` (`runWorkflowFile`); `T/lint/roster.toml`, in a test.

Step 3:

- state-core.md 3.2: the `__all__` facade. 3.3: `gitrun.git`, `GitError`. 3.5:
  `artifacts.load_artifact(root, <story id>)` (`body`). Tests: `T/unit/_repo.py`.
- records.md section 3: `steps.steps_list(project, workflow="story", subject=<id>)` (every run, by run
  start; `StoreCorrupt`) and `store_step` in tests; `cycles.closure_find`; `decisions.decisions_list(project,
  kind="finding")`, and in tests `decision_open` and `settle_decision`; `gate_evidence.patch_id`.
- survey.md 3.3: `worktrees.story_path`, `worktrees.head`. `T/unit/story_ready.py`. D15: `acs` are the ids of
  the `### AC-N` headings.
- **The review lookup (PL4-2).** Records' `gate_evidence.evidence(project, "review", subject)` reads the
  closure only (PL3-7), and step 3's `ORDER.md` section 7 gives G3 the job of putting the computed verdict
  ahead of it. This unit's builder makes exactly that edit in records' file (approved: step 4 `ORDER.md`
  section 4): the `review` branch becomes `from vsdd.state.verdicts import covering; return
  covering(project, subject)`, imported inside the function, with no `ImportError` fallback, and the
  closure-only code it replaces is removed (`covering` reads the closure itself, so there is no cycle).
  The review case of records' `test_evidence_lookup` is updated to the same rule. Nothing else in that
  file or its tests changes.

Step 4, as the spec gives them:

- H7 (`rules/step-results.md`): every step result is stored through `store_step` with the payload's
  `agent_type`; a passing `gates` entry is stored only when its evidence record matches (item 3); a finding
  that fails the rules unit's `ground` on the step's second return is stored, and returned to the script,
  with `grounded: false`. A typed step's payload carries exactly `<plugin>:<agent>` (platform item 36;
  PL4-22), seen with `--plugin-dir` only: a pass of any other type is in `skipped` with `agent_type`.
- G2: a gate result's `sha` is the full commit id. `gate red --refreeze` passes, with `sha` the head, when no
  frozen test changed (as step 5 assumes), or a test-writer's disputes are not counted (rule 4). For VSA-03,
  `gate green --since <sha>` returns `changed_files` by calling this unit's function.
- S3, second part: a `finding` decision has the id `finding.<subject>`. Records' `decision_open` replaces the
  record of an id, so a second `finding` decision for a story must carry the keys and items already waived,
  or they are lost.

## 5. Design decisions

- D1. The tool replays `converge()`; P3BLD-27's shorter rule is not taken. A claim the next reviewer did not
  confirm keeps its finding open, and a dismissal needs the dispute and the ruling, so the last pass alone
  cannot give the verdict.
- D2. No shipped table says who owns a method or which tier an agent has: the roster is under `T/lint/`, the
  method lists in the agent files of step 5. `REVIEW_METHODS` and `ARBITER` are constants beside their one
  reader, held to both by `test_review_methods_table`.
- D3. Items are accounted for by the lens that owns them: for a story, `review-story-diff`. The security
  reviewer marks `checked` only what it examined for security defects (step 5, `security-review`), another
  statement, so its entries neither cover an item nor leave a gap (S5P-N9).
- D4. A fix record counts when a gate it returned passed at its `head`, which is what `gated()` accepts. A
  retry's first attempt failed its gate, so the attempt the script used is the one counted, with no step
  name parsed (VSA-05).
- D5. Grounding runs once, when the result is stored, and the verdict trusts the mark: later fixes change
  the worktree, and the findings of early passes would stop grounding (VSA-07).
- D6. Fix records are joined as the script joins them: `fix()` asks the test-writer first, and
  `converge()` builds a `Map` over the joined list, so the last entry for a key wins (PL4-17). Stored
  order is by path and would put the implementer first, so the method decides the order.
- D7. A later loop's items are fixed at its first pass, as a loop 1's are; files a fix touches inside the
  loop are checked through claims. `--no-renames` so that two callers cannot differ.
- D8. `covering`: the verdict about the commit asked about wins, computed before stored. A pass recorded
  after the checkpoint overrules the closure; a clone with old or no records follows a newer closure.
- D9. `verdicts.py` reads the `### AC-N` headings itself (outside code fences): `story_unit.build` computes
  the position, which calls the verdict. `test_items_equal_unit_acs` holds the two readers together.
- D10. No stored pass, a loop cut off after a fix, and a later loop whose diff git cannot compute are all
  `failed`, `no_result`, never `None` or an exception: callers test one shape, a survey still answers, and
  the next run's full review replaces the loop.
- D11. The Node runner writes a temporary script (meta, the stamped prelude, a driver that turns a case into
  `judge()` or `converge()` calls) and runs it through the harness, as `universal.test.mjs` does; a toy
  would have to pass the universal tests. `test_node_agrees` starts it from pytest: the build's check
  commands run only `universal.test.mjs` and pytest.
- D12. `review show` is not built now (PL4-18). `skipped` is where a person sees why the tool's verdict
  differs from the script's, so it stays in `rebuilt`, and delivery quotes it when it refuses.
- D13. `rounds_used` in the closure form is the loop-1 total across runs and never less than the stored
  one, because the next run's cap reads it. The full verdict keeps the script's `used + pass`.
- D14. A step record that does not parse raises. Skipping it could turn an open loop into a converged one.

## 6. Findings

- **P3OPS-09.** G3's half: the verdict applies `waivers` to every pass and goes on past a pass with nothing
  open, so a waived story reads `reviewed`. Cases `l14-waived`, `l16-waived-coverage`, `r08-waive-after-cap`;
  `test_waivers_from_settled_decision`. `direct` after a cap and guidance in `brief()`: not at slice depth
  (B4a, W2, S3).
- **S5P-N9.** D3. Case `r04-lens-coverage`. The script half is W2's (3.4).
- **VSA-03.** Rule 8 and `changed_files`. Case `r06-repair-new-file`; `test_changed_files_and_later_loop_items`.
  `changed_files` in the `--since` gate result: G2, by this function; the script reading it: W2.
- **P3BLD-27.** Not adopted (D1). Case `l10-claim-not-confirmed` is one the shorter rule gets wrong.
- **VSA-05.** D4 and D6. Cases `r07-fix-retry`, `r16-fix-order-last-wins`.
- **VSA-07.** D5. `test_verdict_trusts_stored_mark`. In `topics/verdicts.md`, "applies the grounding rule"
  should read "honours the stored mark" (spec steward).

## 7. Tests

Cases, each named for what it proves:

- `judge`: `j01-hard-blocks`, `j02-no-anchor-advisory`, `j03-no-location-advisory`,
  `j04-holdout-needs-no-location`, `j05-grounded-false-advisory`, `j06-waived-advisory`,
  `j07-beyond-reach-deferred`, `j08-spec-scope-is-a-gap`, `j09-wave-reaches-integration`.
- `loop`: `l01-clean-first-pass`; `l02-coverage-gap-then-clean` (with `trace`: the fixer is asked);
  `l03-cap-undisputed-kept`; `l04-cap-disputed-dismissed`; `l05-cap-disputed-critical`;
  `l06-cap-disputed-security`; `l07-used-across-runs` (`used` 2, `cap` 3, `arbiter` false: one pass,
  `rounds_used` 3); `l08-reviewer-blocked`; `l09-claims-shape` (with `trace`; one `holdout` claim);
  `l10-claim-not-confirmed`; `l11-claim-withdrawn`; `l12-arbiter-spec-gap`; `l13-fix-returns-nothing`;
  `l14-waived`; `l15-arbiter-silent`; `l16-waived-coverage`.
- `records`: `r01-two-lenses-joined`; `r02-heads-differ`; `r03-builder-echoes-method`; `r04-lens-coverage`;
  `r05-second-loop-open` (`chain` false); `r06-repair-new-file`; `r07-fix-retry`; `r08-waive-after-cap`;
  `r09-used-from-earlier-runs`; `r10-arbitration-record`; `r11-cut-off-after-fix`;
  `r12-invalid-review-data`; `r13-reviewer-blocked-record`; `r14-highest-attempt-counts`;
  `r15-later-loop-converged` (`chain` true); `r16-fix-order-last-wins` (the test-writer says `fixed`, the
  implementer `disputed`: disputed; and the reverse).

`test_conformance.py`: `test_case` (one per file); `test_every_case_is_well_formed` (`name` is the stem,
each expected verdict is valid against the schema, an unknown `kind` fails); `test_spec_rows_have_cases`
(each row of the spec table that section 1 keeps names a file); `test_node_agrees` (runs
`node --test tests/conformance/conformance.test.mjs`; fails with its output, and when `node` is missing).
`conformance.test.mjs`: one test per `judge` and `loop` case; `a case that runs out of passes fails` (the
driver throws when `converge()` asks for a pass the case lacks).

`test_verdicts.py`, on `story_ready` projects with records written by `store_step`:
`test_flat_name_resolves`; `test_no_pass_is_failed_no_result` (and `summary` is `None`);
`test_stored_clean_pass_converges_and_covers`; `test_covering_none_after_a_new_commit`;
`test_covering_by_patch_id` (the head commit amended); `test_computed_overrules_closure`;
`test_closure_used_without_records`; `test_evidence_review_is_covering` (through records' `evidence`,
inside the run that stored the passes and before any checkpoint); `test_items_equal_unit_acs`;
`test_changed_files_and_later_loop_items` (a renamed file gives both names);
`test_waivers_from_settled_decision`; `test_passes_used_counts_run_and_pass_pairs` (3 and 1 give 4);
`test_rounds_used_never_below_closure` (no run records; and a last loop that is a later loop);
`test_summary_is_valid_closure_verdict`; `test_verdict_trusts_stored_mark`;
`test_corrupt_step_record_raises` (the loud failure); `test_unknown_commit_is_not_converged`;
`test_uncounted_agent_type_is_listed_in_skipped`; `test_public_verdict_hides_walled` (through `rebuild`:
records refuses to store one); `test_review_methods_table` (the owners against the roster's tiers and,
where they exist, the agent files).

## 8. Order of work

Before the builder starts: step 3 has reported (`state-core`, `records`, `survey`) and every check passes.
The unit runs beside `gates` and `rules` (`ORDER.md` section 2); S3's second part and G4a need all of it.

1. `verdicts.py` holding `changed_files` alone, with its test: G2's green gate imports it, so it lands
   first.
2. The `judge` and `loop` cases and `conformance.test.mjs`, passing against the prelude as it is. If a case
   that follows the spec table fails, report it (B4a); change neither the case nor the prelude.
3. `convergence.py`, and `test_conformance.py` for those two kinds.
4. `rebuild` in `verdicts.py`, with the `records` cases.
5. The rest of `verdicts.py` with `test_verdicts.py`; then the review-lookup edit of section 4, with
   records' own tests still passing.
6. Every check `ORDER.md` section 2 names.
