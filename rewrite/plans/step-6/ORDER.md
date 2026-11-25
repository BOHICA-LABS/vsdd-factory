# Step 6: build order, file ownership, the first run

Step 6 of the thin slice is W2, then the `story-ready` fixture and its loader, then E2's first run: one
story, headless (`rewrite/spec/packaging/build-plan.md`, "The thin slice"). It is built as two units,
each from one plan in this directory, each by one run of `rewrite/workflows/slice-build-unit.js`. The
plans were corrected against `REVIEW.md` (findings PL6-1 to PL6-16); where a plan and the review differ,
the plan holds. What was decided for each finding is in `RESOLUTIONS.md`.

Every builder and reviewer reads this file. Paths are under `next/` unless they start with `rewrite/` or
`~`; `P` is `plugins/vsdd-factory`, `E` is `tests/e2e`, `U` is `tests/unit`. "ORDER-4" is
`rewrite/plans/step-4/ORDER.md`; "agents.md" is the step 5 plan. "Item n" is an item of the platform
record (`rewrite/spec/platform.md`, `topics/platform-proven*.md`).

## 1. The units

| Key (`args.key`) | Plan | Packages (`args.packages`) | Title (`args.title`) |
|---|---|---|---|
| `story-workflow` | `story-workflow.md` | W2 | The `story` workflow script and its scenario tests |
| `first-run` | `first-run.md` | E1 (the fixture part), E2 (the first run) | The `story-ready` fixture, its loader, and the driver of the first headless run |

Launch a unit with `args: {step: 6, key, title, packages, concurrent, notes}`; `concurrent` names the
other key when both are in flight.

## 2. Order

| Stage | What | May start when |
|---|---|---|
| 0 | nothing is built | Section 5 holds, and every check of section 6 passes on a quiet machine |
| A | `story-workflow` and `first-run`, at the same time | Stage 0 holds |
| P | The probes of section 8.2 | Run and checked on 2026-11-25: the gate passed, and the answers (items 50 to 55) are in `first-run.md` and section 7 |
| R | The first run (section 7) | Both units have reported, stage P has passed and its answers are applied |

"Reported" is the meaning of steps 3 and 4: the whole run returned (build, review, fixes, confirmation)
with no blocker or major finding open.

The two units may be built at the same time. They own disjoint files and neither calls the other's
code: `first-run` stores records the way `U/delivery_fixtures.py` does and stands in for `claude`, and
`story-workflow` stubs every agent. They meet only in the real run and in the rules of section 4. If
they are built one after the other, the order does not matter.

Nothing in step 6 may be built before steps 3, 4 and 5 have reported: `story-workflow`'s `contract`
reads the 25 agent and method files and its cross test calls G3; `first-run`'s scripted delivery runs
the real survey, gates, delivery and checkpoint.

## 3. Files each unit owns

| Unit | Creates | Edits, and nothing else in the file |
|---|---|---|
| `story-workflow` | `P/workflows/story.js`; `tests/workflows/story_scenario.mjs`, `tests/workflows/story.test.mjs`; `tests/lint/test_story_workflow.py` | `workflow-lib/prelude.js`: the prompt string of `relay()` (then `tools/build.py` restamps the toys and `story.js`). `tests/workflows/toys/toy-steps.js`: deleted. `tests/workflows/universal.test.mjs`: lines that name `toy-steps`, if any. `P/workflows/.gitkeep`: removed |
| `first-run` | `E/__init__.py`, `E/fixtures/__init__.py`, `E/probes/__init__.py`; `E/fixtures/load.py`; `E/fixtures/story-ready/` (five files); `E/probe.py`, `E/probes/story.py`, `E/run.py`; `U/e2e_scripted.py`, `U/fixtures/e2e/fake_claude.py`, `U/test_e2e_fixture.py`, `U/test_e2e_probe.py` | `U/story_ready.py`: the three record-writing functions become one call of `make_ready`. `E/sample-project/.factory/factory.toml`: a `[hooks]` block. `E/sample-project/README.md`: the planned story goes. `E/sample-project/.gitignore`: three cache directories |

No unit edits a schema, `roster.toml`, `harness.mjs`, an agent or method file, `tools/`, `pyproject.toml`,
`P/py/`, or anything under `rewrite/`. A builder that finds an interface of steps 3 to 5 missing or
different stops and reports; it is never worked around with a second copy or a narrower test.

## 4. Rules that cross the two units

Step 3's rules (its `ORDER.md`, section 4) and step 4's (ORDER-4, section 3) still hold. These are
added. Where both plans state one of them, they state it in the same words; a difference is a defect
of the plan that differs from this section.

1. **The way in.** The whole prompt is `/vsdd-factory:story <story-id>`, in the project root. The main
   session makes one `Workflow` call by name. No skill, no command file and no hook on `Workflow` is
   built by either unit (`story-workflow.md` 3.1, `first-run.md` 3.5).
2. **Proof that the shipped script ran** is one check, `named_workflow_ran`, owned by `first-run`:
   each attempt's session has exactly one platform run record, and its `script` and the file at its
   `scriptPath` equal the plugin's `workflows/story.js` byte for byte. `story-workflow` guarantees the
   file (a build product, `tools/build.py --check` clean) and asks for nothing else.
3. **The tier.** Every step after the survey takes the survey's `config.tier_override`. The `survey`
   step itself runs at the clerk tier under any override, because the override is its answer. That one
   call, and no other, is exempt in `story-workflow`'s `contract` and in `first-run`'s `tier_applied`.
   The override is asserted where the survey printed it: the survey clerk's captured result.
4. **Spikes.** Nothing on the slice can serve one. `story.js` sets `RUN.limits.spikes_per_run = 0`
   after `applyLimits`, so a spike request ends `blocked`, `spike_limit`. The fixture sets three
   configuration keys and no spike limit (`config.py` refuses 0). The driver's diagnosis names the
   cause `spike_limit` from a step record whose status is `needs_spike`.
5. **The result.** `story.js` returns `status` (standalone: the checkpoint's), `subject`, `merged_sha`,
   `agents` and `verdicts`, one entry per review loop. The driver reads `status` for `run_closed` and
   prints `verdicts` in a diagnosis; it compares no summary text.
6. **A lock nobody closed.** The script closes every run it was given an id for. A survey whose lock
   release failed answers with no `run_id`; the script cannot close that, and the driver's refusal 7
   (`vsdd lock release --force`, with the terminal's standard input) is the way out.
7. **`first-run`'s tests never need W2's file.** `run_attempt` takes the plugin source directory as an
   option (default: the repository's plugin). `U/test_e2e_probe.py` gives it a temporary copy that
   holds a stand-in `workflows/story.js`, and `named_workflow_ran` is tested against that stand-in.
   Only the real run needs the real file.
8. **Generated files** (ORDER-4, rule 3). Only `story-workflow` changes `workflow-lib/prelude.js`,
   and only the relay string. While it is in flight, a `tools/build.py --check` drift or a
   `build_is_current` failure that names the prelude, a toy or `story.js` is not a finding of
   `first-run`: run the build and go on. Nobody edits a generated region by hand.
9. **No builder starts a real `claude` session**, and none runs the probes or the first run. Those are
   the orchestrating session's or the person's, from a shell outside the Bash sandbox.
10. **Python runs with `-B`.** A `__pycache__` under `plugins/` fails two checks. No commit, branch,
    stash or push; no git command that discards or restores a file.

## 5. What must be true of steps 3 to 5 before a unit starts

Both units: all seven step 3 units, the five step 4 units and the step 5 unit have reported, and the
checks of section 6 pass.

| # | Unit that must hold it | What | Needed by |
|---|---|---|---|
| 1 | step 3 `survey` | `tests/unit/story_ready.py`, `_install_product`: a product directory's own `.factory/` is not copied, and `[paths]` is set from the product's `.factory/factory.toml` when it has one. On the sample: `delivery.mode` stays `local`, `tests = ["tests/**"]`, `source = ["tally/**", "tools/**"]` (PL6-7). Needed before step 4's stage A; `first-run` leaves that code as it finds it | step 4 `gates`, then `first-run` |
| 2 | step 4 `gates` | `tests/unit/gate_project.py` asserts both halves of row 1 on the project it builds | `first-run` (its scripted delivery) |
| 3 | step 4 `verdicts` | `vsdd.state.verdicts.rebuild(records, *, subject, cap, waived, acs, changed)` exists as verdicts.md 3.3 gives it | `story-workflow` (the cross test) |
| 4 | step 4 `survey-2` | The stamped `S_STORY_UNIT` holds `bc_paths`, `conventions`, `demo_dir` | `story-workflow` |
| 5 | step 5 `agents` | The technical-writer, devops and verifier files list `fix-findings`, and `test_agent_files.py`'s table says so (PL6-1) | `story-workflow` |
| 6 | step 5 `agents` | `review-story-diff` takes each finding's `owner` from the task when the task names one (PL6-1) | `story-workflow` |
| 7 | step 5 `agents` | `clerk.md` holds, word for word: "If a `vsdd story deliver` call is moved to the background, run the same command again: it re-attaches to its job. A moved call of any other command: wait for it, as the notice says." (PL6-8) | `story-workflow` |
| 8 | step 5 `agents` | Universal tests 3 and 6 reach workflow files by listing the directories, never by the name `toy-steps` | `story-workflow` (it deletes the toy) |

The exact changes asked of the step 4 and step 5 builds are in section 8.1. If row 5, 6 or 7 does not
hold when `story-workflow` starts, its builder stops and reports.

## 6. The checks

Run by each unit before it reports, and once more by the orchestrating session on a quiet machine after
both have reported. Take a snapshot of the tree before stage A.

```
cd next
uv run --no-sync python -B tools/build.py
uv run --no-sync python -B tools/check.py
uv run --no-sync python -B -m pytest -q
node --test tests/workflows/*.test.mjs
uv run --no-sync python -B tools/build.py --check
```

- The node line replaces `node --test tests/workflows/universal.test.mjs` everywhere it is written
  (`rewrite/STATE.md`, ORDER-4): `story.test.mjs` is the second file.
- `tests/hooks/test_latency.py` is sensitive to load, as in steps 3 and 4.
- `story-workflow` is not done until `tests/workflows/toys/toy-steps.js` is gone and
  `every method is dispatched by some scenario` passes.
- After both units report, the orchestrating session runs the loader once from a shell outside the
  sandbox, as a smoke check that needs no model:
  `uv run --no-sync python -B tests/e2e/fixtures/load.py story-ready ~/.cache/vsdd-factory/e2e/smoke-<stamp> --local`.
  It must print `ok: true`. Inside the Bash sandbox it cannot run at all: writes under `$HOME` and
  under `.git/worktrees/` are refused there (item 41).

## 7. The first run

**Before it.** Both units reported; section 6 clean; the probes of section 8.2 run and their answers
applied; the person's answer to question 4 of section 8.3, or its default.

**The command**, from a terminal outside any sandbox, with `claude` signed in:

```
cd next
uv run --no-sync python -B tests/e2e/run.py story
```

That is the cheap tier. The driver makes a new run directory
`~/.cache/vsdd-factory/e2e/story-<UTC stamp>-<4 hex>/`, copies the plugin into it, loads the fixture,
and starts one session in `project/` with exactly this (`first-run.md` 3.5):

```
CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0 VSDD_TEST_TIER=sonnet:low \
VSDD_FACTORY_HOOKS_CAPTURE=<run dir>/attempt-1/hooks VSDD_FACTORY_HOOKS_DEBUG=1 \
claude -p "/vsdd-factory:story S-1.01" --plugin-dir <run dir>/plugin \
  --permission-mode default --allowedTools "Workflow Agent Bash Read Write Edit Glob Grep" \
  --settings '{"sandbox":{"enabled":false}}' --setting-sources project,local --strict-mcp-config \
  --model sonnet --effort medium --max-budget-usd 6 --output-format json < /dev/null
```

Both isolation flags stay on and `Skill` stays out of the tool list (items 50 and 51).

**What it costs.**

| Tier | Command | Expected | Cap for one session | Cap over all attempts | Time limit |
|---|---|---|---|---|---|
| `cheap` (the default) | as above | 6 to 9 agent starts, 1 to 3 dollars | 6 dollars | 12 dollars | 45 minutes |
| `roster` | add `--tier roster` | 6 to 15 dollars | 30 dollars | 60 dollars | 120 minutes |

The figures are estimates: round 4 paid 29 cents for eight trivial steps at the cheap tier, and no
story has been delivered yet. `report.json` holds what was measured. Run `roster` once, after `cheap`
has passed, and only with the person's word (section 8.3, question 4).

Both caps are soft (item 54): what is in flight is charged past a cap (2.7 times a 2-cent cap in the
probe), so one attempt's overshoot can take a session past its cap and the run past its total.

**What a pass looks like.** The command exits 0 and prints the run directory. In it:

- `attempt-1/report.json` has `ok: true`, and every counted check passed: `session_ended`,
  `named_workflow_ran`, `hooks_healthy`, `tier_applied`, `agent_budget`, `caps`, `run_closed`,
  `tests_first`, `merged_is_reviewed`, `closure_delivered`, `integration_holds_change`.
  `holdout_wall` is reported as not applicable and is not counted.
- `project/` is on `main`, clean, one commit past the base: `S-1.01: Add median() to tally.stats`,
  with the trailer `Factory-Story: S-1.01`, changing only files under `tally/` and `tests/`. The
  sample's test, lint and format commands pass there, and the oracle passes.
- The story closure is `closed` at position `delivered`; the lock is free; no story worktree or
  branch is left; the cycle holds the `brief` and `spec` decisions only.
- The platform's run record ended `completed` with `result.status` `done` and one `verdicts` entry,
  `converged`.

A pass settles item 18 (the survey returned the override from `VSDD_TEST_TIER`, and every later step
ran on it) and gives item 22 its first figure. Record both, save `report.json` under
`rewrite/findings/results/`, and keep the run directory until the findings of the run are in the ledger.

**When it fails.** The command exits 1 and prints one cause, the failed checks, each step of the run
record, the lock holder, the story's position and the resume line. Nothing is lost: the whole run
directory is kept. Look first at what the cause names:

| Cause | Look first at | What it has meant so far |
|---|---|---|
| exit 2, the word `sandbox` | The message; the shell the command ran in | The driver or the session is sandboxed: no write under `.git/worktrees/` (item 41) |
| `blocked_before_first_turn` | `attempt-n/result.json`, the `result` text | A hook stopped the typed command; `num_turns` is 0 and `is_error` is false (item 46) |
| `budget_stop` | `attempt-n/stderr.txt`; the run record's `status` | The session reports `error_max_budget_usd`: exit 1, no `result` key, `errors` naming the cap (item 54). Run record `completed` with the whole result: the cap was passed after the workflow ended. `killed`, with `error` "Workflow aborted", `result: null` and a step left in `progress`: it was passed during that step, and standard error says "Budget limit reached ...; stopping background agents." The step is named; its result may be in `attempt-n/hooks/` with no `PostToolUse`. Resume with `--dir` and `--total`: the spend is past the cap, so refusal 8 refuses the default |
| `no_workflow` | `result.json`: `result`, `permission_denials` | The main session never called `Workflow`: the typed command was not made, or a tool was denied (probe leg A) |
| `inline_script` | `attempt-n/platform/workflows/wf_*.json`, the `script` key | The main session passed its own script (item 49). Run again; if it repeats, question 3 of section 8.3 |
| `hooks_not_live` | The survey's `error` in the step list | "no caller marker": the plugin's hooks did not load. Run again with `--user-settings` |
| `denied` | The failed `session_ended` check; the step transcripts in `attempt-n/platform/` | A tool call was refused in the default mode. A step's refusal is never in `permission_denials` (item 52): it is read from the step's transcript, and the diagnosis names the agent, the tool and the input that the tool list of `first-run.md` 3.5 did not admit |
| `ceiling`, `timeout` | `attempt-n/stderr.txt`; the last step in the run record | The ten-minute stop (item 47) or the driver's own limit; the step that was running is named |
| `tier_not_applied` | The survey clerk's payload in `attempt-n/hooks/` | `config.tier_override` is empty: `VSDD_TEST_TIER` did not reach the survey (item 18). The run cost roster prices |
| `spike_limit` | The step record with status `needs_spike`, its `spike_request` | A step asked for an experiment; none can be served yet. The checkpoint opened `blocked.S-1.01` |
| `workflow_<status>` | The run record's steps and `result.verdicts`; then `project/.factory/local/runs/<run>/`; then the last `block` and `error` events | The script ended on its own: a gate that still fails, a review at its cap, a delivery refused (`delivery.detail` says which row) |

Then fix what the run found, in the code and in the spec, and run the same command with
`--dir <run directory>`: it starts `attempt-2` in the same project, and the survey takes the run over.
The driver never starts an attempt by itself, and refuses one that could take the run past its total.

## 8. For the orchestrating session

### 8.1 Changes asked of the step 4 and step 5 builds

None of these plans was edited. Each change goes into the `notes` of the build named, or, where that
unit has already reported, into a fix run of it. Rows 1 to 3 are needed before step 4's stage A; rows 4
to 8 before `story-workflow` starts.

| # | Plan, section | The change | Finding |
|---|---|---|---|
| 1 | ORDER-4, section 5, request R1 (built by step 3's `survey` unit) | Add to R1: "`_install_product` does not copy the product's `.factory/` directory, and sets `[paths]` from the product's own `.factory/factory.toml` when it has one; otherwise as built." A test in the survey unit builds `make_project(product=<the tally sample>)` and asserts `delivery.mode == "local"`, `paths.tests == ["tests/**"]` and `paths.source == ["tally/**", "tools/**"]` | PL6-7 |
| 2 | `gates.md`, section 7 (`gate_project.py`) | After building its project, `gate_project.py` asserts the same three values, so a regression of row 1 fails loudly in the gates unit and not as a misread red gate | PL6-7 |
| 3 | ORDER-4, section 7, "Inputs to W2's plan" | Strike the sentence "After a repair the script does not run the evidence stage again, so a `demo: true` story is refused at the demo row and the next run finishes it." W2 records evidence again (`story-workflow.md` D9). Not on the slice's path | PL6-12 |
| 4 | `agents.md`, section 3 (technical-writer, devops, verifier) and section 7 item 3 (the method table of `test_agent_files.py`) | Each of the three lists `fix-findings` beside its own method. In section 4 the head of `fix-findings` becomes "(`implement`, fix; `fix-<the kind's implement agent>-*`; `repair-<n>`)", and the method gains: "For a kind with no red gate, fix inside your own write scope; there are no re-frozen tests." In section 1 the row `fix-implementer-l<n>-p<k>` is named `fix-<implement agent>-l<n>-p<k>` | PL6-1, S5P-N10 |
| 5 | `agents.md`, section 4, `review-story-diff` | The owner rule becomes: "`owner` as the task says; when the task names none, `test-writer` for category `test` or `coverage`, else `implementer` (for scope `spec`, the spec's owner)." | PL6-1 |
| 6 | `agents.md`, section 3 (the clerk) and section 7 item 3 | In place of "M3's background sentence" the clerk file carries the two sentences of row 7 in section 5, word for word, and the test checks those. This is what `probes-4-apply-C2-3` asks, in the words the relay prompt uses | PL6-8 |
| 7 | `agents.md`, section 7 item 4 | Tests 3 and 6 of `universal.test.mjs` apply their assertions to every file the test already lists (the toys and `workflows/`), and never name `toy-steps`. The toy uses the closings `story.js` uses: a fix step of a later loop closes with `gate green --since <sha>`, a second evidence step is `evidence-<n>`, the survey relay carries `--wait 540` | W2's plan, section 2 |
| 8 | `survey-2.md`, 3.5, `stages` for a delivered story | Only if the person answers yes to question 2 of section 8.3: `assess` also writes the `tests` and `implement` entries of a delivered story, each at the `sha` of the run's passing `red` and `green` gate entry. No evidence is needed for it | PL6-16 |

Not asked, though the review offered it: making `changed_files` and `test_output` required on a green
result in `gate-result.schema.json` (PL6-10). W2 reads a missing list as an unusable result, so the
slice does not depend on the schema.

### 8.2 Probes before the run

One probe round, a gate for stage R. It needs no step 6 code: a scratch copy of the round-4 probe
plugin (`rewrite/spike/probes-round-4/plugin`, with its logging hook), in a directory under `$HOME`,
because that is where the first run's project will be (PR3-1). The copy gains a `bin/` tool that
prints a marker, a `methods/probe.md` that holds another, an agent `worker` with the tools `Bash,
Read, Write, Edit, Glob, Grep`, and two workflows. The scratch project is a git repository with one
commit and a linked worktree `.worktrees/x` on a branch `story/x`, and no settings file. Both sessions
are started by the typed workflow command itself, with no skill, and with the first run's own flags:

```
CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0 <the logger's variable>=<probe>/logs/<leg>.jsonl \
claude -p "/<plugin>:<workflow> <absolute plugin directory>" --plugin-dir <probe>/plugin \
  --permission-mode default --allowedTools "Workflow Agent Bash Read Write Edit Glob Grep" \
  --settings '{"sandbox":{"enabled":false}}' --setting-sources project,local --strict-mcp-config \
  --model sonnet --effort medium --max-budget-usd <cap> --output-format json < /dev/null
```

| Leg | Workflow, cap | It answers | Pass, or what to record | What follows |
|---|---|---|---|---|
| A1 | `one`, 1 dollar: one typed step (`agentType` the plugin's `worker`, `model` `sonnet`, `effort` `low`) | Do a `--plugin-dir` plugin's hooks, workflows and `bin/` load under the two isolation flags? Every round-4 session had the user's settings loaded, `skipWorkflowUsageWarning` among them | The hook log holds the step's payloads; a run record exists and ended `completed`; the step returned the `bin/` tool's marker | Fail: repeat without the two flags. If that passes, `first-run`'s default becomes `--user-settings` (one line) and the run is made that way |
| A2 | the same session | Does the typed workflow command work with no skill in the default mode? Item 31 was seen in `auto` mode; round 4 entered through a skill | `num_turns` at least 1; the run record's `script` equals the plugin's `one.js`; no denial | Fail with the model reaching for `Skill`: repeat with `Skill` allowed; if that passes, `Skill` joins `first-run.md` 3.5's list. Still failing: stop. The way in then needs a skill, a design change to mirror to the person |
| A3 | the same session; the step Reads `<plugin>/methods/probe.md`, Edits one file and Writes another in `.worktrees/x`, commits there with plain Bash | Do bare tool names in `--allowedTools` admit a Read outside the working directory, and Edit, Write and Bash in a linked worktree? | `permission_denials` is empty; the marker came back; the commit is on `story/x` with both files | Fail: the denied tool and input say which allow rule is missing; `first-run.md` 3.5 gains it before the run |
| A4 | the same session | Items 26 and 48: does a step's payload carry the step's effort (`low`) or the session's (`medium`)? | Record `effort.level` of the step's payloads | Answered `low` (item 53): `first-run` writes `STEP_EFFORT_PROVEN = True` in `E/probe.py` |
| B | `three`, 0.05 dollars: three steps in a row | What does a `--max-budget-usd` stop leave in the middle of a workflow? | Record the exit code; whether a result is written, and its `subtype`, `is_error`, `num_turns`; the run record's `status`; what the running step left | The cause `budget_stop` in `first-run`'s diagnosis is made to read exactly that |
| C | no session | The platform's session folder, which three checks read | From leg A's folder and the kept round-4 folder (`~/.claude/projects/-private-tmp-…-pr4-project/1cf1064b-…/`): `workflows/wf_*.json` with `script`, `scriptPath` (under `workflows/scripts/`), `status`, `result`, `agentCount`, `workflowProgress` (per agent `agentId`, `agentType`, full `model` id, `state`); `subagents/workflows/<wf>/agent-*.jsonl` with `tool_use` entries. For a call by name `script` and the file at `scriptPath` equal the plugin's file | A platform item. If leg A's folder differs from the kept one, `first-run.md` 3.3 follows the new shape |

Expected cost: under a dollar. This machine has Claude Code 2.1.332 and the platform record was made
on 2.1.331; leg A's log shows a typed step's `agent_type` again (item 36) on the new version. Rerun A
of the round-4 check (`rewrite/spike/probes-round-4/check-reruns/`: items 36 to 38 and 40) is worth
its 30 cents beside this round, and is not a gate: the first run's survey step exercises item 38 for
cents, and a rewrite that adds `grounded: false` (item 37) is needed only when a finding is ungrounded.

Run on 2026-11-25 on 2.1.332 and checked independently (`rewrite/spike/probes-round-5/CHECK.md`):
items 50 to 55. Legs A1 to A3 passed as specified, so no fallback of the table is taken; what legs A3
and B add is in `first-run.md` 3.6 and 3.7 and in section 7.

Not probed, because the plans no longer need it: `agent()` with an agent type that has no file
(`story.js` never reaches `runSpike()`); `--no-session-persistence` (never passed).

Settled by the first run itself: item 18 (`VSDD_TEST_TIER` reaching `vsdd survey` in a step's Bash
call) and item 22's first figure.

### 8.3 Questions for the person

Each default stands until the person says otherwise; none blocks stage A.

1. **Spikes while nothing can serve one** (PL6-2). `story.js` sets its own spike limit to zero until
   the spike-runner (A3), `vsdd spike` (S6) and the spike-bounds rule (H8) are built, because
   `config.py` refuses `spikes.per_run = 0`. Should 0 become a valid setting, meaning "this project
   runs no spikes"? **Default:** no change to the configuration; the script's line stands and goes
   with those three packages.
2. **A durable record of test-first order** (PL6-16; extends "Waiting on the user" item 6 in
   `rewrite/STATE.md`). After a squash delivery inside one run, the closure holds only the `review`
   and `deliver` stages and the story's commits are unreachable. The first run proves the order from
   the run's local step records and from loose objects, read at once. A clone, or this machine later,
   cannot. Should the checkpoint record the red and green commits in the closure (row 8 of 8.1), or
   delivery keep a ref? **Default:** neither; the run is made as planned.
3. **An inline `Workflow` script in an interactive session.** Only a headless driver can tell which
   script ran (item 49). A hook rule that refuses a `Workflow` call carrying `script` would cover
   interactive runs, and could block a person's own ad-hoc workflows in a factory project. It is a new
   capability, to be mirrored before anyone designs it. **Default:** no rule.
4. **Spending on the real platform.** The probe round costs under a dollar; the first run at the cheap
   tier is capped at 6 dollars a session and 12 over all attempts. **Default:** the orchestrating
   session runs both, as next action 6 of `rewrite/STATE.md` says. The roster tier (capped at 30 and
   60 dollars) is run only after the person says so. The first run needs a shell outside the Bash
   sandbox; if the orchestrating session cannot get one, the person runs the one command of section 7.

The defaults already recorded in `rewrite/STATE.md` stand for this step: the sandbox is required off;
local delivery refuses a story that touches a dependency manifest or `CHANGELOG.md` (the fixture's
story touches neither); a decision is settled with `vsdd decide` in a terminal.

### 8.4 Ledger and the spec

Outcomes these plans claim, to record when each unit reports: S3P-1 (W2's half), S5P-N1 and S5P-N11
(W2's half), S5P-N9 (W2's half), S5P-N10, probes-4-apply-C2-1, layer-2a-fix-3-C1-1 (W2's part), VSA-03
and P3BLD-16 (W2's half), P3BLD-17, P3BLD-18, VSA-15, S3B-state-core-1, SL1-28, SL1-29, PR4C-12, PR4-3,
PR4C-16, PR4C-17, PR5-1.

New findings, each left for the package named:

- A3, with S6 and H8: remove `RUN.limits.spikes_per_run = 0` and its test from `story.js`, and add the
  spike probe to E2 (S5P-N7; `story-workflow.md` D18).
- B4a, C1: the story result carries `agents` and `spikes` beside the schema's `counts` (D15).
- B4a with G3: a full review with `rounds_used` at the cap ends `blocked`, `cap` with no pass until
  `direct` grants one (D16; P3OPS-09).
- G4a's `pr` mode, M4: delivery data names no head, so a `review_stale` loop reviews the commit the
  script knows; add `head` to `delivery-data` before anything but the script's own steps can move it
  (D17). The cross test leaves `review_stale` out for the same reason.
- S3 or G2: the story unit or the green result names the frozen files, so a repair that changed one
  can be re-frozen in the same run (PL4-19).
- G2, when widened: the demo evidence records the commit each recording was made at, and the gate
  refuses a recording older than the head (PL6-12).
- C1, G2, H7: a green result's `changed_files` and `test_output` are optional in the schema and
  unchecked by the `step-results` rule (PL6-10).
- S1 (`config.py`): `spikes.per_step` and `spikes.per_run` are typed "at least 1" where the spec's
  table gives no minimum; question 1 decides.
- E2: `holdout_wall` becomes a counted check with a holdout overlay and H2 (PL6-11). Agent starts are
  counted from captured payloads until H9's `agent.start` exists.
- B8: a refused `Read`, `Write` or `Edit` in a step (item 52 saw Bash only); a large step's overshoot
  of a budget cap (item 54).

For the spec steward, when W2 and E2 shrink their files to pointers: `topics/prelude-steps.md` takes
the relay text of `story-workflow.md` D11; `workflows/story.md` and `topics/story-script.md` say that a
fix goes to the kind's implement agent, that the standalone status is the checkpoint's and that the
result carries `verdicts`; `packaging/tests-e2e.md` lists `Skill` and `--no-session-persistence`, which
the first run does not pass, and reads the event log for agent starts.

`rewrite/STATE.md`: the node check line of section 6; the platform items of 8.2; the four questions.
