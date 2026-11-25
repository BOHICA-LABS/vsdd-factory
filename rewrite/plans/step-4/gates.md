# Step 4 plan: the story gates (package G2)

Brief: `rewrite/spec/packages/G2.md`. Design, under `rewrite/spec/`: `commands/gate-red.md`, `gate-green.md`,
`gate-demo.md`, `topics/gates.md`, `evidence-lookup.md`, `long-commands.md`, `records/evidence.md`,
`baseline.md`. Step 3 plans are cited by file name (`rewrite/plans/step-3/`); `ORDER.md` is this
directory's. Paths below are under `next/`; `P` is `plugins/vsdd-factory`, `G` is `P/py/vsdd/gates`.

## 1. Scope

Built now: `vsdd gate red` (JUnit reader, base report, the pass rule, frozen files, criteria map) and
`--refreeze`; `vsdd gate green` (red record present, frozen files unchanged, no stub marker, the configured
commands, the diff patch, `--since`); `vsdd gate demo` (the criterion-to-test table, and a file check of
recorded evidence); the diff patch writer; and the three gates' entries in records' gate registry. Red and
green do all measuring inside a G1 job, so no call, foreground or survey, ever waits on a suite. Every
path on which a gate cannot measure returns `passed: false` with a reason.

Left for when the build widens:

- The baseline comparison and the flaky rerun (`records/baseline.md`): the baseline's fields are G7's.
  Until then `flaky` is `[]`, and a cycle that names a baseline is measured with no exemption and says
  so (D12).
- Feature-flag checks (F2); hub projects and `--repo`; identities for lint and format findings (P3OPS-11).
- The red lookup after a rebase (VSA-01: G4a and S2a); the `commit.made` event; `init` and `doctor`
  warnings for a report that names no files (S1).
- A red gate, or a re-freeze, for a story kind with none: it fails out loud today (D16; S5P-N10 is W2's).

## 2. Files

Owned:

- `G/junit.py` — `read_report(path) -> dict[str, Case]`, the not-implemented type list, the file of a
  test, the criteria map.
- `G/diffpatch.py` — `write_patch` (3.5), the one writer of patch files.
- `G/gatelib.py` — what the three gates share: story lookup, report path, `dirty_files` (3.5), the cached
  record, the suite run, the result builder, the job attach. Not named `gate_*`: the registry imports those.
- `G/gate_red.py`, `G/gate_green.py`, `G/gate_demo.py` — one gate each: `measure`, the job's `work`, and
  `register_gate("<name>", measure)` as the last line. The stub marker is written `"VSDD-" + "STUB"`.
- `G/cmd_gate_story.py` — `COMMANDS` for `("gate","red")`, `("gate","green")`, `("gate","demo")`.
- `tests/unit/gate_project.py` (the tally sample with one admitted feature story; the one sample-based
  builder, reused by S3's second part), `tests/unit/fixtures/junit/*.xml`, `tests/unit/test_junit.py`,
  `test_diffpatch.py`, `test_gate_red.py`, `test_gate_refreeze.py`, `test_gate_green.py`,
  `test_gate_demo.py`, `test_gate_cli.py`, `test_gate_registry.py`.

Shared (approved in `ORDER.md` section 4; no other step 4 unit edits these files):

| File | Owner | This unit changes |
|---|---|---|
| `P/schemas/gate-result.schema.json` | C1 | Adds optional `test_output` (string) and `changed_files` (array of strings); a new fixture `valid_green-since.json` |
| `P/schemas/wave-gate-data.schema.json`, its two valid fixtures | C1 | The same two properties, also in `required` (`tools/check.py` holds the two schemas equal and this one fully required) |
| `workflow-lib/prelude.js`, stamped regions of `tests/workflows/toys/*.js` | B4a | Nothing by hand: `tools/build.py` regenerates them from the schema files (`ORDER.md` rule 3) |
| `tests/e2e/sample-project/tools/junit_runner.py` | E1 | Writes `file` (path relative to the project root) on each `<testcase>` and `type` (the exception class name) on `<failure>` and `<error>` |

No edit to `tools/check.py`, `tools/build.py`, `cli.py`, `G/__init__.py`, `G/jobs.py` or any `state/` file.

## 3. Interfaces this unit provides

### 3.1 The result

Every gate returns the shipped `gate-result` object. What the spec leaves open:

- `sha`: the full head commit of the story worktree when the call began; `""` only when the story could
  not be resolved. `evidence`, `diff_patch`, `test_output`: absolute paths; `""` when not written.
- `failures`: at least one line whenever `passed` is false (the dispatcher's exit 2 object alone has
  only `error`); at most 20 of 400 characters, the last `and <n> more`. `error`: set when `ok` is
  false, equal to the first failure.
- Green adds `diff_patch` (the base-to-head patch; with `--since` the range patch, 3.5), `test_output`
  (`green.<sha12>.log`), `flaky` (`[]`) and `changed_files`: `[]` without `--since`, with it G3's
  `verdicts.changed_files(project, since, head)`, the one definition (PL4-7).
- `metrics`: red `{new, red, exempt, base_tests, no_file, seconds}`; re-freeze `{changed}`; green
  `{tests, failed, skipped, seconds, baseline}`; demo `{mode, uncovered, recorded}`.

| Case | `ok` | `passed` | Exit |
|---|---|---|---|
| Measured and passed, or a passing record for this head returned | true | true | 0 |
| Job still running: `{ok: true, state: "pending", retry}` (`pending-output`) | true | - | 0 |
| Measured and failed; dirty worktree; no report written; head moved; no red record; kind has no red gate | true | false | 1 |
| Not attempted: admission refused; `--refreeze` caller without the grant; `--since` not an ancestor of the head; `commands.test` empty or `test_report` not `junit:<path inside the worktree>`; the job failed (limit, crash, lost twice) | false | false | 1 |
| Usage error (argparse, `Refuse`): the dispatcher prints the whole object | false | false | 2 |

### 3.2 Commands

`vsdd [-C <dir>] gate red --story <id> --run <id> [--refreeze]`, `gate green --story <id> --run <id>
[--since <commit>]`, `gate demo --story <id> --run <id>`; each takes the dispatcher's `--json` and
`--wait`; a `--story` outside the story grammar is exit 2. Each `Command` has `needs="mounted"`,
`output="gate-result"`, `failure={"gate": "<name>"}`, and prints with `Reply.of(result, 0 | 1)`. Red and
green are jobs; demo returns directly. `--refreeze` prints `gate: "red"`. Pass conditions: the three
command files; what they leave open is section 5.

### 3.3 Evidence (for H5, H7, S3, G4a); fields are records' `evidence-record` schema

- Red: `red.<sha12>.json` with `frozen` (`file` relative to the worktree root, POSIX; `sha256` of the
  bytes at the head), `tests` (`id` is `<classname>::<name>`), `criteria` (`AC-<N>` to ids), and the
  report as `.xml`. The base report is `evidence_dir(project, "_base")/<base12>.xml`.
- Re-freeze: a `red` record with `refreeze: true`, always `passed: true`, carrying the **complete** frozen
  set, tests and criteria at the head. A refused re-freeze writes no record, so the earlier one stands.
- Green: `green.<sha12>.json` with `diff_patch` (always the base-to-head patch, also after `--since`) and
  `flaky`; `.log` (every command's output) and `.xml`.
- Demo: `demo.<sha12>.json`; `.log` holds the table, one line per criterion and test.
- No record for: a dirty worktree, a refusal, a moved head, a kind without the gate, a job that failed.
- **H7's check always holds**: a result with `passed: true` has a record at
  `evidence_read(project, subject, gate, sha)` with the same four values.
- A failing record never replaces a passing one: a passing record for the head is returned first.

### 3.4 The gate registry

Each `G/gate_<name>.py` ends with `register_gate("<name>", measure)`; records imports the modules by file
name on the first `registered_gate` lookup (records.md, `gate_evidence`), so state code imports no gate
module.

`measure(project, subject, run, *, wait: int | None = None, retry: str = "") -> dict`, plus
`refreeze=False` for red and `since=""` for green: the registry contract (records only stores the
callable). It returns a gate result, or for red and green the pending object: it **attaches to the
gate's job** (the command's own key) with `attach(..., wait=wait, retry=retry)` and never measures in
the caller's process. `wait=0` answers pending at once; `None` is `clamp_wait`'s 100 s. S3's second part
calls `fn(root, story, run, wait=<seconds left of its own wait>, retry=inv.retry_command())` and answers
pending itself (PL4-8). `retry` defaults to the gate's command line. It raises for nothing a caller can
cause; a refusal is an `ok: false` result. Two rules for callers: call it from a foreground process, not
from inside another job (two slots, and with `parallel_gates = 1` a deadlock); and red measures the head
only, so red evidence lost after implementation commits returns `passed: false`, never a pass.

### 3.5 Functions for G4a

- `vsdd.gates.gate_demo.measure(project, subject, run) -> dict`: direct, never pending. Delivery's "the
  demo gate holds" is `passed` of this call, through `registered_gate("demo")`, for every story: the
  passing record for the head when there is one, else a measurement (the table when `demo` is false).
- `vsdd.gates.diffpatch.write_patch(project, subject, base, head, *, tree, since="") -> str`: writes
  `git diff --no-color --no-ext-diff <since or base> <head>` into
  `evidence_dir(project, subject)` by rename and returns the absolute path; an empty diff is an empty
  file; `GitError` for an unknown commit. The name says the range: `diff.<head12>.patch` for base to
  head, `diff.<since12>-<head12>.patch` with `since`, so a `--since` call never replaces the whole patch
  that the next run's full review is given (PL4-6). Green and delivery write patches no other way.
- `vsdd.gates.gatelib.dirty_files(tree, config) -> list[str]`: D5's rule, the one copy; delivery's
  clean check calls it (PL4-15).

### 3.6 What step 5 assumed (`rewrite/plans/step-5/agents.md`, sections 6 and 8)

Met: a re-freeze result is named `red` and passes when no frozen test changed; `gate green --since` is
named `green`; `sha` is the head; failures are always given; a stub's not-implemented error is red;
`ac<N>` then a non-digit; `green_by_design` and `wiring`. The demo gate reads
`.factory/demo/<cycle>/<story>/`, which S3's second part gives the unit as `demo_dir`. Two differences
for the methods (`ORDER.md` section 7): `--refreeze` is refused unless it is the whole command of a
test-writer's call; a changed frozen test that is skipped fails the re-freeze.

## 4. Interfaces this unit needs

- **state-core.md**: 3.1 `Command`, `Invocation` (`root`, `wait`, `caller_key`, `config`,
  `retry_command()`), `Reply.of`, `Refuse`, `main`; 3.3 `gitrun.git`, `GitError`; 3.4
  `vsdd.project.path_class(path, root=, config=)`; 3.5 `artifacts.load_artifact` (the story's `demo`,
  `ui`, and its `### AC-<N>` headings); `tests/unit/_repo.make_project`, `set_keys`.
- **records.md**: `gate_evidence.evidence_write(project, record, log=, report=)`, `evidence_read`,
  `evidence`, `patch_id`, `evidence_dir`, `register_gate`, `registered_gate`; `store.locked`, `root_of`,
  `StoreError`; `callers.caller`; `cycles.cycle_read` (the `baseline` field).
- **survey.md**: 3.4 `admission.require_story(root, run, story)` and its seven fields, `AdmissionError`;
  3.3 `worktrees.detached(root, commit, name)`, which must replace a `_tmp-<name>` a killed worker left
  (its D17), `worktrees.head`, `is_ancestor`; 3.5 `story_kind`, `kind_has_red_gate`;
  `tests/unit/story_ready.py`.
- **Asked of step 3** (`ORDER.md` section 5; a miss is reported, not worked around). R1, survey:
  `story_ready(tmp, *, product=<dir or callable>)` copies and commits the product on the integration
  branch and sets `[paths]` before admission; committed later, the sample is in the story's diff, the
  base report is empty and every test is new (PL4-9). R2, records: `evidence_dir(project, "_base")` is
  accepted, not `unsafe_id` (PL4-13).
- **jobs.md**: 3.2 `JobSpec`, `attach`, `Outcome`; 3.3 `JobContext` (`root`, `key`, `args`, `config`,
  `remaining()`, `run(command, cwd=, output=)`), `JobTimeout`.
- **Under `next/`**: `Config.command(name)`, `Config.get("paths")`; `vsdd.policy.load("walls")` (the
  `refreeze` group) and `load("artifacts")` (the `[demo-evidence]` row); `vsdd.schema` for tests; the sample.
- **Step 4 units**: G3's `vsdd.state.verdicts.changed_files`, imported inside green's `--since` path; G3
  builds it first (section 8). H5 reads `frozen`; H7 reads records; S3 calls 3.4; G4a calls 3.5.

## 5. Design decisions

- **D1. The body is the job.** The command does only what is cheap and decides the key: admission, the
  caller check, the clean check, the cached record, then `attach(..., wait=inv.wait,
  retry=inv.retry_command())`. The worker repeats those checks and measures. The marker's session id goes
  into `JobSpec.session_id`, so the job's events are written. Key: `gate red`,
  `gate red --refreeze`, `gate green` or `gate green --since <full sha>`; the story; the head. The key
  holds no run (jobs.md D11), so the result carries the private key `run`; the command and `measure`
  remove it, serve no result made for another run, and attach again, once (PL4-16).
- **D2. The registry function attaches too** (3.4): no caller then waits on a suite in its own process,
  and the survey and an agent share one measurement.
- **D3. `ok` and exits** as the table of 3.1: exit 2 is usage only. A job-layer
  failure has its `error` copied into `failures`, so an agent always has a line to act on.
- **D4. The cached record** is the passing record for exactly this head (`evidence_read`), never one
  found by patch id: `gated()` needs `sha` to be the head. Green also needs its `.xml` to exist. On every call green writes the whole patch again, and with `--since` the range patch too.
- **D5. Clean check** (`gatelib.dirty_files`). `git status --porcelain -z --untracked-files=all` in the
  worktree, less the report path; the failure names the files. After measuring, a head that moved is a
  failure with no record.
- **D6. One measurer per worktree.** The worker holds `store.locked(project, "gate-<story>.lock",
  wait=ctx.remaining())` while it runs commands there; the base report runs in `_tmp-base-<job key>`.
  Command output goes to `.factory/local/tmp/<job key>.<name>.out`, then into the record's `.log`.
- **D7. Never an old report.** The report file is deleted before the test command runs; none afterwards
  is `the test command wrote no JUnit report at <path>`. A report that does not parse is the same
  failure. At the base, no report means an empty base set and nothing is kept.
  Base reports older than `state.retain_runs_days` are deleted when a new one is written.
- **D8. Reading a report.** A repeated identity is failed if any occurrence is. `<skipped>` is not red and
  not passed. A test's file is its `file` attribute made relative to the worktree, else the longest
  prefix of the classname (`.` and `::` as `/`) that equals, or ends, the extension-less path of exactly
  one `tests`-class file (VSA-12); else `""`, and the test counts. Criterion: `ac(\d+)(?!\d)`, no case.
- **D9. Red's failures**: `no new tests`; `build or harness error in <id>: <type>` (an `<error>` whose
  `type` is absent or not a not-implemented type); fewer than half of the non-exempt new tests red; no
  red test. The test command's exit code is not read.
- **D10. Re-freeze** (the spec is silent on "nothing changed"; P3SEAM-15). Needs a red record (`evidence`),
  else `no red record`. A passing record for this head is returned as it is. Otherwise the frozen set is
  computed afresh from base to head; a file the earlier record froze that is no longer in it is
  `frozen test file removed`. No file changed or added: the record is written at the head with the
  earlier tests and criteria, and the suite does not run (an unchanged test broken by half-done code is
  not the test-writer's to mend). Otherwise the suite runs
  once: each earlier identity must still be reported, and each new test in a changed or unknown file
  must pass or be red in D9's sense.
- **D11. Who may re-freeze.** H6 is not in the slice, and a re-freeze by the wrong agent defeats the
  frozen-test wall. The command reads the marker (`callers.caller`) and refuses unless the row of
  `walls.toml` for its `agent_type` holds `refreeze`; no marker is refused, naming the one-segment form.
- **D12. Green's commands.** `test` is required; `build`, `lint`, `format_check` run when set, and an
  empty one is listed in `metrics.skipped`. A story with `ui` also needs `ui_check`; a `docs` story
  needs `docs_check` as well as the others (AU-P2-07). The
  suite passes on exit 0 and a report with no failed case. With a baseline named by the cycle,
  `metrics.baseline` is `"not applied"` and a failing result adds `the cycle has a baseline, and the
  comparison is not built: every failure counts`.
- **D13. Green's static checks run first**; if one fails, no command runs and the failing record is
  written: no red record (a kind with a red gate); a frozen file whose SHA-256 differs; `VSDD-STUB` in a
  tracked `source`-class file. After the suite, every test of the red record must be in the report and
  passed (`frozen test did not run: <id>`).
- **D14. Demo** (the spec gives no pass rule). Always: green evidence covers the head, else `no green
  evidence covers the head`. Story `demo` false: for a kind with a red gate a red record must exist, and
  each test in its `criteria` must be passed in the green `.xml`; a criterion with no test is listed in
  `metrics.uncovered` and in the table, and does not fail.
  `demo` true: `report.md` is not empty; at least one criterion has one script (`.sh`, `.tape`,
  `.spec.ts`) and a transcript `AC-<k>.txt`; every `AC-<k>` file names a criterion; sizes within the
  row's caps.
- **D15. A red record over records' 8,000-byte cap** is `the red record is over its cap (<n> new tests):
  split the story`; nothing is cut, because a cut record would unfreeze tests.
- **D16. No red gate for the kind**: `gate red` returns `story kind <kind> has no red gate`, no record.
- **D17. `--since`** is resolved to a commit that is an ancestor of the head; the range patch and
  `changed_files` use it. The record's `diff_patch` and `patch_id` are always base to head (VSA-03's
  definition). A `changed_files` that answers `None` is `ok: false`.

## 6. Findings (`rewrite/findings/by-package/G2.md`)

- **P3SEAM-15**: D10 and 3.3: the complete set, `gate: "red"`. `test_refreeze_records_changed_and_added`.
- **P3SEAM-16**: the marker is the constant `VSDD-STUB`, searched in `source`-class files at the head.
  `test_stub_marker_fails`.
- **VSA-12**: D8, and the sample runner writes `file`. `test_criteria_need_a_non_digit`,
  `test_file_from_attribute_then_classname`, `test_sample_runner_writes_file_and_type`.
- **P3BLD-16**: `test_output` in the green result and schema. `test_green_passes_and_writes_patch`.
  Reading it in `story.js` is W2's.
- **AU-P2-07**: D12. `test_docs_and_ui_checks_required`.
- **AU-S2-12**: the code has one re-freeze rule, D10. `test_refreeze_accepts_a_failing_assertion`.
- Not at slice depth: **AU-P2-06** (G7, then G2: the flaky rule); **VSA-01** (G4a's `record_base` after
  a rebase, S2a's lookup).
- **AU-P2-03**, **AU-S2-01**: no code: each pass condition is one function here; the duplicate prose is
  the spec steward's.

Not on the list, settled here: VSA-03's gate half (`changed_files` by G3's function; the range patch
has its own name: spec steward); the two gaps of `verify-slice-amend.md` (base report retention, D7; an
unchanged re-freeze, D10).

## 7. Tests

Gate tests run the real command (`main([...])`) on the tally sample (`gate_project.py`, built on
`story_ready(product=...)`) with `commands.test` set to `<sys.executable> tools/junit_runner.py`; every
result is checked against `gate-result`.

`test_junit.py`: `test_reads_cases_and_outcomes`; `test_not_implemented_types`; `test_file_from_attribute_then_classname`
(`tests.test_stats.TestMean` gives `tests/test_stats.py`; two candidates give `""`);
`test_criteria_need_a_non_digit`; `test_broken_report_raises`. `test_sample_runner_writes_file_and_type`.

`test_diffpatch.py`: `test_write_patch_names_by_range`; `test_unknown_commit_raises`.

`test_gate_red.py`: `test_red_passes_and_freezes` (exit 0; `frozen`, `tests`, `criteria`, the `.xml` and
the base report); `test_base_report_measured_once`; `test_stub_error_counts_as_red`;
`test_harness_error_fails` (an import error is named); `test_half_rule_and_exemptions` (one red of
three fails; exempt tests alone never pass); `test_no_new_tests_fails`;
`test_source_class_test_does_not_count`; `test_dirty_worktree_writes_nothing` (a modified and an
untracked file are named, no record, no job; the report file alone is not dirt);
`test_passing_record_returned_without_measuring`; `test_failing_record_measured_again`;
`test_no_report_is_loud` and `test_stale_report_is_not_read` (the loud-failure tests: never a pass);
`test_kind_without_red_gate_fails_loudly`; `test_story_not_in_run_refused` (`ok: false`, exit 1);
`test_head_moved_writes_nothing`; `test_record_over_cap_is_loud`.

`test_gate_refreeze.py`: `test_refreeze_nothing_changed_passes_without_suite` (named `red`, record at the
head, command not run); `test_refreeze_same_head_returns_the_record`;
`test_refreeze_records_changed_and_added`;
`test_refreeze_accepts_a_failing_assertion`; `test_refreeze_refuses_removed_errored_skipped` (no record;
`evidence` still returns the earlier one); `test_refreeze_needs_a_red_record`;
`test_refreeze_only_for_the_test_writer` (implementer's marker and no marker refused).

`test_gate_green.py`: `test_green_passes_and_writes_patch` (`diff_patch`, `test_output`, `flaky: []`,
`skipped`); `test_green_needs_red_record`; `test_frozen_test_changed_fails`; `test_stub_marker_fails`
(not in a test file); `test_failing_suite_lint_format_each_named`;
`test_frozen_test_missing_from_report_fails`; `test_exit_zero_without_report_fails`;
`test_since_narrows_patch_and_lists_changed_files` (a renamed file gives both names);
`test_since_patch_leaves_whole_patch` (the file and the record's `diff_patch` still hold base to head);
`test_since_not_an_ancestor_refused`; `test_cached_green_rewrites_patch`;
`test_baseline_named_is_strict_and_says_so`; `test_docs_and_ui_checks_required`;
`test_job_failure_keeps_gate_shape` (`ok: false`, `failures` set).

`test_gate_demo.py`: `test_table_built_without_recording`;
`test_demo_needs_green_evidence`; `test_demo_missing_report_copy_fails`; `test_recording_checked`
(each missing part fails, the whole set passes); `test_demo_returns_directly` (`gate_demo.measure`, as
G4a calls it: never pending).

`test_gate_cli.py`: `test_three_gates_register`; `test_pending_then_result` (`--wait 0` is the
`pending-output` object, the next call the result, one run); `test_usage_error_prints_gate_shape`;
`test_result_of_another_run_is_not_served` (no `run` key is printed); `test_through_launcher` (exits 0
and 1).

`test_gate_registry.py`: `test_gates_register_by_file_name` (a fresh interpreter, no gate import);
`test_registered_function_attaches_and_can_be_pending` (shares the command's job; `wait=0` answers at
once); `test_state_imports_no_gate_module`.

## 8. Order of work

Before the builder starts: all seven step 3 units have reported, with R1 and R2 in, and every check
passes. The unit runs beside `verdicts` and `rules` (`ORDER.md` section 2). Step 5 imports
`verdicts.changed_files`, which G3 builds first; if it is not there yet, build `gate_demo.py` first.

1. The two schema edits and fixtures, then `tools/build.py`, as one step and before anything else
   (`rules` runs the same build later); the sample runner and its test.
2. `junit.py` with its fixtures; `diffpatch.py`.
3. `gate_project.py`; `gatelib.py`.
4. `gate_red.py` and the red command; then `--refreeze`.
5. `gate_green.py`; `gate_demo.py`.
6. The command-line and registry tests.
7. `tools/build.py` once more, then `cd next && uv run --no-sync python tools/check.py`, the pytest
   suite, the node harness, `tools/build.py --check`.
