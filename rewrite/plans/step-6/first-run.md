# Step 6 plan: the `story-ready` fixture, its loader, and the first headless run (E1 fixture part, E2 first run)

Paths are under `next/` unless they start with `rewrite/`. `E/` is `tests/e2e/`, `U/` is `tests/unit/`, `P` is
`plugins/vsdd-factory`. Spec files are under `rewrite/spec/`. "Item n" is an item of the platform record
(`platform.md`, `topics/platform-proven*.md`). Step 4 plans are cited by file name; `agents.md` is step
5's. "W2" is the other unit of this step (`story-workflow.md`); "ORDER-6" is this step's `ORDER.md`.

## 1. Scope

Built now: the `story-ready` overlay as data (a manifest, one contract, one epic, the story "add
`median()`" and an oracle no agent sees); the loader, which builds a ready project from it through the
real `init` and the stores; the probe form; the story probe; and `E/run.py story`, the one command
that loads, starts `claude -p`, waits, checks and diagnoses. The real run is made by the orchestrator
or the person, outside any sandbox, after the probes of ORDER-6; never by the builder.

Left for when the build widens, each one more probe of the form of 3.3: the three kills (`Probe` gains
`interrupt(evidence) -> bool`; 3.7 already resumes and counts starts per attempt); the eleven-minute
gate; pull-request mode (the loader without `--local`, a bare origin, `E/fakebin/gh`); a
holdout overlay with H2 (D13); `e2e.yml`, collection by `pytest -m e2e`, retry once, the uploaded
JSON, the smoke probes, the other four overlays, measured budgets.

## 2. Files

Owned:

- `E/__init__.py`, `E/fixtures/__init__.py`, `E/probes/__init__.py`: empty, so `tests.e2e.*` imports.
- `E/fixtures/load.py`: `load`, `make_ready`, `set_key`, `main` (3.1); standard library and `vsdd`.
- `E/fixtures/story-ready/`: the five files of 3.2.
- `E/probe.py`: `Probe`, `Evidence`, `Result`, `run_attempt`, `collect`, the seven general checks.
- `E/probes/story.py`: `PROBE` and the five story checks (3.6). `E/run.py`: the command (3.4, 3.5, 3.7).
- `U/e2e_scripted.py`, `U/fixtures/e2e/fake_claude.py`, `U/test_e2e_fixture.py`,
  `U/test_e2e_probe.py` (section 7).

Shared, and all this unit changes in them:

- `U/story_ready.py` (step 3, `survey`): `_cycle`, `_approve_spec` and `_close_plan` go; in their place
  `make_project` calls `make_ready(root, cycle=cycle, title="Sample cycle", waves={"w1": [story]},
  session=SESSION, flush=False)` and keeps its own flush. All else stays (D1), `_install_product` as
  the survey unit's fix leaves it (ORDER-6, section 5).
- `E/sample-project/` (step 1, E1). `.factory/factory.toml` gains `[hooks]` with
  `main_session_wall = true` and `log_retention_days = 30` (S3B-state-core-1). In `README.md`
  "Planned story" goes and the layout line names no function, so the file cannot go stale at
  delivery. `.gitignore` gains `.pytest_cache/`, `.ruff_cache/` and `.mypy_cache/`: gates and delivery
  need a clean worktree, and M3 tells a builder to commit everything.

## 3. Interfaces this unit provides

### 3.1 The loader

`uv run --no-sync python -B tests/e2e/fixtures/load.py <overlay> <directory> --local`; `<overlay>` is a
name under `E/fixtures/` or a path. It prints `{ok, directory, story, cycle, branch, base, state_commit,
spec_digest, mode}` and exits 0.

- Exit 2, nothing created: unknown overlay; no `--local` ("only the local form is built"); the directory
  is not empty, lies inside a git work tree, or its path has a character outside `A-Za-z0-9_@%+=:,./-`
  (the hook approves `vsdd -C <dir>` only for such words, ORCH-2).
- Exit 1, one line `load: <step>: <reason>` on standard error, the directory left for reading: a step fails.

Order (`load(overlay, directory, *, local) -> dict`; raises `LoadRefused` or `LoadFailed(step, reason)`):

1. `sh E/make_sample_repo.sh -n <directory>`: the sample without `.factory/`, one commit on `main`, a local
   identity, signing off, no remote: that is what `--local` means.
2. `vsdd init` in process, as `U/_repo.run_init` does; the answer must be `state: ready`.
3. `.factory/factory.toml` becomes the sample's own, with three keys set by `set_key(text, section, key,
   literal)`, which replaces the one line of that key in that section and fails unless there is exactly
   one: `delivery.mode = "local"`, `delivery.ci = "none"`, `state.push = false` (VSA-15). `config.load`
   must load the result.
4. The overlay's `factory/**` is copied onto `.factory/`; a target that exists is a failure.
5. `make_ready(root, cycle=<id>, title=<title>, waves=<waves>)`.
6. Post-checks, each a failure by name: `artifacts.check_paths` finds nothing; `spec_approved` holds;
   every row of `preconditions.check(root, "story", subject=<story>, cycle=<id>)` is met; the checkout
   is on `main` with an empty `git status --porcelain`.

`make_ready(root, *, cycle, title, waves, session="", flush=True) -> dict` (D1): `cycle_write`
(`kind: product`, `route: standard`); the `brief` decision opened and approved; the
`spec` decision opened with `digests.spec_digest(root)`, computed then (the overlay stores none), and
approved; the plan closure set to `state: closed` with `waves`; then, with `flush`, `commits.flush`,
which must answer `committed` or `clean` with nothing rejected.

### 3.2 The overlay `E/fixtures/story-ready/`

| File | Content rule |
|---|---|
| `overlay.toml` | `sample = "sample-project"`, `story = "S-1.01"`, `[cycle]` with `id = "c-1"` and `title`, `[plan.waves]` with `"W-01" = ["S-1.01"]`. No record, digest or configuration (D2, D3) |
| `factory/specs/product/contracts/1.01/BC-1.01.001.md` | Kind `contract`: `id`, `title`, `status: draft`, `subsystem: SS-01`, `capability: CAP-001`, `origin: new`; the six headings; the text below |
| `factory/stories/E-1/epic.md` | `id: E-1`, `title: Order statistics`, `cycle: c-1`, `capabilities: [CAP-001]`, `status: open`; one sentence |
| `factory/stories/E-1/S-1.01.md` | `id: S-1.01`, `title: Add median() to tally.stats`, `status: draft`, `cycle: c-1`, `kind: feature`, `priority: P1`, `criticality: low`, `contracts: [BC-1.01.001]`, `demo: false`; the text below |
| `oracle.py` | Never copied into the project. Run in the checkout: imports `median` from `tally`, checks every test vector of the contract, the unchanged list and the generator; exit 1 on a miss |

The contract. Behavior: `tally.stats.median(values)`, exported from `tally` beside `mean` and `variance`
and listed in `__all__`; an odd count gives the middle element of the ascending values, as it is; an even
count gives `(a + b) / 2` of the two middle elements. Preconditions: any iterable of `int` or `float`, read
once. Postconditions: a list passed in is unchanged; no value raises `ValueError("median() requires at
least one value")`, as `mean` and `variance` word theirs. Edge cases: one value; repeats; a generator.
Test vectors: `[3, 1, 2]` gives `2`; `[5]` gives `5`; `[4, 1, 3, 2]` gives `2.5`; `[2, 2, 3, 9]` gives
`2.5`; `[1.5, 0.5]` gives `1.0`; `[]` raises that `ValueError`.

The story. Body: "Change only `tally/stats.py`, `tally/__init__.py` and test files under `tests/`. No
dependency, no manifest, no `CHANGELOG.md`, no change to `README.md` or `tools/`." Then `## Acceptance
criteria` with exactly four headings `### AC-<N> (BC-1.01.001)`: AC-1, an odd count gives the middle
element of the sorted values (`median([3, 1, 2])` is `2`, `median([5])` is `5`); AC-2, an even count gives
the mean of the two middle elements (`median([4, 1, 3, 2])` is `2.5`); AC-3, no value raises `ValueError`
with the message `median() requires at least one value`; AC-4, any iterable is accepted, a generator
included, and a list passed in is left unchanged.

Each names a value a test asserts, so its test is red against a stub; tests named `ac<N>` give the
demo gate its table. `feature` runs the whole path, `low` keeps one review lens, `demo: false` leaves
no evidence stage, and no manifest or changelog exists for delivery's row 11.

### 3.3 The probe form (`E/probe.py`)

`Probe(name, prompt, overlay, workflow, allowed_tools, budget_usd, timeout_minutes, agent_budget, env,
checks)`; the budget and the time limit hold a value per tier. A check is `fn(evidence) -> Result(name,
group, ok, text, counted=True)`; a missing file never raises: it is `ok: false`, "not measurable: ...".

`Evidence`: `run_dir`, `project`, `plugin`, `tier`, and per attempt `result` (the session result or `None`),
`exit_code`, `killed`, `session_id`, `records`, `payloads` (what the hook runner captured) and
`transcripts`. `records` are the platform's run records, copied to `platform/` from `<claude
home>/projects/*/<session_id>/`: `workflows/wf_*.json`, and the step transcripts
`subagents/workflows/<wf>/agent-*.jsonl`. Of a record the driver reads `script`, `scriptPath`,
`status`, `result`, `agentCount` and each `workflow_agent` entry of `workflowProgress` (`agentId`,
`agentType`, `model` as a full id, `state`), not its `workflow_phase` entries; a `killed` record has
`result: null` and an `error` key (item 55). The rest is read through the stores of the plugin copy.

`run_attempt(probe, run_dir, options)` starts one session (3.5) and collects; `options` names the
plugin source (ORDER-6, rule 7). `collect(run_dir)` only collects, for `--assert-only` and for tests.

### 3.4 The command

`cd next && uv run --no-sync python -B tests/e2e/run.py story [--dir D] [--tier cheap|roster|<model[:effort]>]
[--budget USD] [--total USD] [--timeout MIN] [--assert-only] [--user-settings] [--refresh-plugin]
[--claude BIN] [--claude-home H]`

Exit 0: every counted check passed. Exit 1: the session or a check failed. Exit 2: refused to start.
One run of the command is one attempt: the driver never starts a second by itself (D14).

Without `--dir` the run directory is new: `~/.cache/vsdd-factory/e2e/story-<UTC stamp>-<4 hex>/` (D5). It
holds `run.json` (tier, budget, the `claude` version, the attempts and their cost), `plugin/` (a copy of
the plugin, made once, D6), `project/` (the loaded fixture), and per attempt `attempt-<n>/` with
`command.json`, `pid`, `result.json`, `stderr.txt`, `hooks/`, `platform/`, `report.json`.

Refusals (exit 2), in this order, before every attempt:

1. `claude --version` is below the roster's `min_claude_code`. A version the platform record was not
   made on is a recorded warning.
2. Fresh run or `--refresh-plugin`: `tools/build.py --check` reports drift, or the workflow file is missing.
3. `--dir` is one 3.1 refuses, unless it holds a `run.json`.
4. The sandbox, this process: no file can be created in `git -C project/.factory rev-parse
   --absolute-git-dir`, a directory under `.git/worktrees/` (item 41). The message has the word `sandbox`.
5. The sandbox, the session: a settings file the session will load (the project's two; with
   `--user-settings` also `~/.claude/settings.json`) sets `sandbox.enabled`.
6. `sh plugin/bin/vsdd -C project validate --json` does not exit 0.
7. Resume: the last attempt's `pid` is alive; or the run lock is held and `vsdd lock release --force`,
   run with this terminal's standard input, is refused (its message is printed).
8. The cost recorded for the earlier attempts plus this attempt's cap is over the total: twice the
   cap of 3.5, or `--total` (D14).

### 3.5 The session: every setting, and what it rests on

Working directory `project/`, standard input `/dev/null`, a new process group. The environment is the
person's, less every `VSDD_*`, `CLAUDE_PLUGIN_ROOT` and `CLAUDE_PROJECT_DIR`, plus the rows marked "env".

| Setting | Value | Rests on |
|---|---|---|
| env `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS` | `0` | Item 47; PR4C-12 |
| env `VSDD_TEST_TIER` | `sonnet:low` for `cheap`; unset for `roster`; else the text given | The survey as built. The run settles item 18 |
| env `VSDD_FACTORY_HOOKS_CAPTURE`, `VSDD_FACTORY_HOOKS_DEBUG` | `attempt-<n>/hooks`; `1` | The runner as built; payload fields of items 36 and 48 |
| prompt; `--plugin-dir` | `/vsdd-factory:story S-1.01`; `plugin/` | Item 50. Every probe ran a copy outside the repository |
| `--permission-mode` | `default` | Items 36 to 40 were seen in it; `auto` admits what nothing approved (item 7) |
| `--allowedTools` | `Workflow Agent Bash Read Write Edit Glob Grep` | Item 30 (`Workflow`); item 38 (an unallowed Bash call is refused part by part, so no pattern list covers a builder); round 4 allowed `Agent`. `Read` under the plugin copy, writes in a worktree: item 52 |
| `--settings` | `{"sandbox":{"enabled":false}}` | Item 41 set the sandbox through this flag |
| `--output-format`; `--model`, `--effort` | `json`; `sonnet`, `medium`: no tier's effort is `medium`, so a payload can tell a step's effort from the session's (D11) | Item 53 |
| `--max-budget-usd` | 6 for `cheap`, 30 for any other tier, or `--budget` | Item 54: a soft cap; two shapes of stop (3.7) |
| `--setting-sources project,local`, `--strict-mcp-config` | On; `--user-settings` drops both | Item 51 (2.1.332, default mode); D8 |

`--no-session-persistence` is never passed (D9). The driver's own limit is 45 minutes for `cheap`, 120
otherwise: then, and on Ctrl-C, `SIGTERM` to the group (item 46), `SIGKILL` after 30 s, `killed`
recorded.

Models. The default is `cheap`: every step after the survey on `sonnet` at `low`, through the override
the survey returns. `--tier roster` adds that `xhigh` and each alias are accepted, and gives item 22
its first figure; run it once, after `cheap` has passed. Expected cost: ORDER-6, section 7.

### 3.6 The checks

All read files (`result.json`, `hooks/`, `platform/`, the stores, the event log) and git; model prose
is printed in a diagnosis, never compared. `H` is the closure's `delivery.head`, `M` its `merged_sha`.

| Check (group) | Passes when |
|---|---|
| `session_ended` (low touch) | Exit code 0; `subtype` `success`; `is_error` false; `num_turns` at least 1 (0 is a block: item 46); no `permission_denials`, and no refused step call (item 52) in the step transcripts, one per `workflow_agent` entry; every payload has `permission_mode: default` |
| `named_workflow_ran` | Each attempt's session has exactly one run record, and its `script` and the file at its `scriptPath` equal `plugin/workflows/story.js` byte for byte; the last attempt's `status` is `completed` (item 49; W2 3.1) |
| `hooks_healthy` | The event log has lines with the session's id and none of type `rule.error`, `rule.timeout`, `runner.launch_failed`, `runner.disabled`, `config.invalid` or `policy.invalid` |
| `tier_applied` | (a) The survey clerk's captured `StructuredOutput` payload has `config.tier_override` equal to the tier asked for (`null` for `roster`), which settles item 18. (b) Each `workflowProgress` agent after that survey step (W2 D20) has a `model` id with its tier's alias as a `-` segment: the override's, else its `agentType`'s roster tier. (c) Each payload of such an agent, joined by `agent_id`, has that tier's `effort.level`: counted only with `STEP_EFFORT_PROVEN` (D11) |
| `agent_budget` (lean) | Agent starts over all attempts are at most 12: distinct `agent_id`s among the `PreToolUse` payloads (D10). `agentCount`, the result's `agents` and the cost are recorded; an attempt over its cap fails |
| `caps` (lean) | `vsdd validate` finds nothing; every record is within its store's cap constant, every event line within `MAX_LINE_BYTES`. A state file no cap claims is listed |
| `holdout_wall` (reported, not counted: D13) | No call of a `builder`-tier agent names a path under `.factory/holdout`: Bash, Edit, Write from the payloads; Read, Grep, Glob from `tool_use` entries in the transcripts' `assistant` records |
| `run_closed` (low touch) | The run record's `result.status` is `done`; the last `vsdd` run has ended; no lock entry; the cycle's decisions are `brief` and `spec` only; no `.worktrees/S-1.01`, no branch `story/S-1.01` |
| `tests_first` (enforceable) | `R`, the earliest passing `red` entry in the step records' `gates`, is a strict ancestor of `G`, the earliest passing `green` entry, and `G` is `H` or its ancestor; `base..R` changes a `paths.tests` file, `R..G` a `paths.source` file; R's `step.recorded` line precedes G's |
| `merged_is_reviewed` (enforceable) | The verdict is `converged`; the `review` stage is at `H`, or at a commit with H's patch id; the last stored review result has that `reviewed_sha`; the tree of `M` equals the tree of `H`; the `pr.merged` line names `H` |
| `closure_delivered` | The story closure has `state: closed`, `position: delivered`, `delivery.state: merged`; `M` is the head of `main`; `.factory` has nothing uncommitted |
| `integration_holds_change` | `main` is the base plus one commit, its message `S-1.01: <title>`, a blank line, `Factory-Story: S-1.01`; it changes only files under `tally/` and `tests/`, `tally/stats.py` among them; the checkout is clean at `M`; the configured `test`, `lint` and `format_check` exit 0 there with `UV_OFFLINE=1`; `oracle.py` exits 0 |

### 3.7 Diagnosis and resumption

After every attempt `report.json` is written: `{probe, tier, ok, attempts: [{n, session_id, exit_code,
killed, num_turns, cost_usd, agent_starts, seconds, cause}], checks, totals}`; `totals` sums cost and
agent starts over all attempts. On failure the driver prints, from files alone: the failed checks; the
session's exit code, `subtype`, turns, cost and denials; each step of the run record (label, agent
type, model, state) and the result's `verdicts`; the `vsdd` run and the lock holder; the story's
position, `delivery.state` and `detail`; open decisions' packet paths; the worktree's head and
uncommitted files; the last ten `block` and `error` events; one cause; the resume line.

Cause, the first that applies: `sandbox` (the word, or `Operation not permitted`, in a survey error, a
step result or an event); `blocked_before_first_turn`; `budget_stop` (`subtype` `error_max_budget_usd`;
no `result` key, so the driver reads `result.get("result")`; the run record gives the shape of item 54
and is printed: `completed` with its `result`, or `killed` with the step left in `progress`);
`no_workflow`; `inline_script`; `hooks_not_live` (the survey's `hooks_alive` row: "run again with
`--user-settings`"); `denied` (a `permission_denials` entry or a refused step call: agent, tool and
input are named); `ceiling` (standard error names the ten-minute stop); `timeout`; `tier_not_applied`;
`spike_limit` (a step record with status `needs_spike`; W2 D18); `workflow_<status>`.

The run directory is always kept. Resume is the person's: the same command with
`--dir <run directory>`. `run.json` gives the tier (another is refused), nothing is loaded again, the
plugin copy is reused unless `--refresh-plugin`, and `attempt-<n+1>` starts a new session with the
same prompt. The survey takes the run over; the checks then run over all attempts.

## 4. Interfaces this unit needs

Built, under `next/`: `dispatch.main`; `config.load`; `cycles.cycle_write`, `closure_update`, `closure_read`;
`decisions.decision_open`, `settle_decision`, `spec_approved`, `decisions_list`; `digests.spec_digest`;
`commits.flush`; `artifacts.check_paths`; `preconditions.check`; `runs_list`; `steps_list`; `lock_entries`;
`gate_evidence.patch_id`; the stores' cap constants; the runner's capture and debug switches; `vsdd
validate` and `vsdd lock release --force`; `E/make_sample_repo.sh -n`; the roster; `tools/build.py`.

Step 4: gates.md 3.1 to 3.3 (the gate commands; a `gates` entry `{gate, passed, sha}`;
`junit.read_report`); rules.md 3.4 (a result stored with `agent_type`; `step.recorded` with the
identity); survey-2.md 3.2, 3.5 (a delivered closure: `state: closed`, `position`, the `review` and
`deliver` stages, `verdict`); delivery.md 3.2 to 3.5 (`delivery.head`, `merged_sha`, `state`, `detail`;
the message; `pr.merged`; `needs_person`; worktree and branch removed); `U/story_progress.py`,
`U/delivery_fixtures.py`.

Step 5: agents.md section 2 (each agent file's `tools` line; `failing-tests` names tests `ac<N>`; M5).

W2 (`story-workflow.md`). 3.1: `/vsdd-factory:story <id>` as the whole prompt starts the built
`P/workflows/story.js` with the typed text as `args` (items 31, 50: no `Skill`); no skill and no hook
on `Workflow` is built, and the proof that the shipped script ran is `named_workflow_ran`, worded there
as here. 3.4:
standalone, the first step relays `vsdd survey story --subject <id> --wait 540 --json` and
the last `vsdd checkpoint story --run <id> --json`, after a failure too. 3.3: the result has `status`
(the checkpoint's), `subject`, `merged_sha`, `agents` and `verdicts`. D20: every step but the first
survey takes the survey's `tier_override`. D18: the script serves no spike, so the fixture sets no
spike limit.

## 5. Design decisions

- D1. **One description of a ready project: `make_ready`.** The loader owns the record-writing steps
  and `story_ready.py` calls them, as its docstring promises: steps 3 and 4 test the loader's recipe.
- D2. **The overlay has no `factory.toml`**, though the spec gives it one: the sample ships its
  configuration and a second copy would drift. The loader sets the three keys a remote decides.
- D3. **Records are made, not copied**, though the spec lists them as overlay files: hand-written
  record JSON rots (step 4 `ORDER.md`, rule 5).
- D4. **The fixture keeps the shipped `merge_strategy = "squash"`.** The branch is then deleted and `R`,
  `G`, `H` are unreachable but still objects: the checks run at once, and a missing one fails by name.
  Nothing durable shows the test-first order later: the person's question (ORDER-6, section 8).
- D5. **The run directory is under `$HOME`**: probes ran there and under Claude Code's own temp tree
  only (PR3-1). D6. **The session runs a copy of the plugin**: builders editing `next/` cannot change
  a run or its resume.
- D7. **The sandbox is made sure of three ways** (3.4 rows 4 and 5, the `--settings` flag); a miss
  is loud in the survey's first flush.
- D8. **Isolation is on by default**: otherwise the developer's plugins, hooks, allow rules and `auto`
  mode enter the run. A `--plugin-dir` plugin loads whole under the two flags (item 51). The person's
  user-level `CLAUDE.md` still reaches the main session and every step.
- D9. **No `--no-session-persistence`**, against `packaging/tests-e2e.md`: whether the run record
  survives it is unproven.
- D10. **Agent starts are counted from captured payloads**: the event log's `agent.start`, which the
  tier's spec names, needs H9. A step that dies before a guarded call is missed; `agentCount` shows it.
- D11. **The model is asserted per step from the platform's run record, the effort from the
  payloads.** `workflowProgress` gives each agent's `agentType` and full `model` id, joined to the
  payloads by `agentId`; `modelUsage` is only recorded. A step's payload carries the effort the step
  was dispatched with, over the session's and the agent file's (item 53, with its check's rerun):
  `STEP_EFFORT_PROVEN = True` in `E/probe.py`, and the effort leg counts.
- D12. **A check that cannot measure fails**; the oracle is outside the project.
- D13. **`holdout_wall` is reported, not counted** ("not applicable: the overlay has no holdout and H2
  is not built"): counted, it would pass on nothing, or fail for a rule nothing enforces yet.
- D14. **An attempt is the person's act, and spend is bounded over all attempts**: `--max-budget-usd`
  caps one session, refusal 8 the run directory. Both are soft (item 54): after a budget stop a
  resume at the same cap needs `--total`.

## 6. Findings

- Fixed here, each with its test in section 7: VSA-15 (D2, 3.1); P3BLD-22, P3COV-14 and SL1-26 (the
  sample tests); SL1-28 (G2's `read_report` reads the runner's file); SL1-29 (the script has a
  caller, the loader); S3B-state-core-1 (the `[hooks]` block); PR4C-12 (3.5); PR4-3 and PR4C-17
  (`named_workflow_ran`); PR4C-16 (`session_ended`, `test_block_is_loud`); PR5C-14 (a refused step
  call); PR5-1, PR5C-16 (`budget_stop`).
- Not at slice depth: AU-P2-14 (E1's second part); PR4-2 (the eleven-minute gate); S5P-N12
  (`demo: false`; B8 keeps both probes); S5P-N7 (A3, S6, H8, then a spike probe; until then the
  diagnosis names `spike_limit`).
- W2's: S3P-1, probes-4-apply-C2-1, S5P-N1, S5P-N9, S5P-N10, S5P-N11 (the run exercises `bc_paths`,
  but neither N9 nor N10); layer-2a-fix-3-C1-1 (`tier_applied` repeats it on the real platform).

## 7. Tests

`deliver_scripted(tmp_path) -> Evidence` loads the fixture, runs the real survey, commits a stub with
reference tests, runs `gate red`, commits a reference `median`, runs `gate green`, stores step and review
results as `U/delivery_fixtures.py` does, runs `story deliver` and the checkpoint, and writes a session
result, payloads, and a run record and step transcripts of the shape of 3.3, as `fake_claude.py` does.

`U/test_e2e_fixture.py`:

- `test_overlay_artifacts_validate` (`vsdd validate` on the loaded project);
  `test_loader_builds_a_ready_project` (one commit on `main`, state committed, every precondition met).
- `test_survey_admits_the_fixture_story`: the real survey's unit is `feature` with four criteria, and
  with `VSDD_TEST_TIER` in its environment the answer carries the override.
- `test_spec_digest_is_computed_not_stored`: editing the contract makes `spec_approved` false.
- `test_loader_local_sets_mode_ci_push`; `test_loader_refuses_without_local`;
  `test_loader_refuses_nonempty_repo_and_unsafe_directories`; `test_loader_failure_is_loud` (a story
  naming a missing contract exits 1 naming the step).
- `test_story_touches_nothing_delivery_refuses`: `needs_person` is empty for the reference change.
- `test_sample_commands_run_offline_and_leave_nothing` (and with the three cache directories made,
  `git status --porcelain` stays empty); `test_sample_report_is_read_by_the_gate_reader`;
  `test_sample_config_states_every_key`; `test_oracle_holds_every_contract_vector`;
  `test_oracle_fails_on_the_sample_and_passes_on_the_reference`.
- `test_scripted_delivery_passes_the_real_gates`; `test_story_ready_uses_the_loader_recipe`
  (`story_ready.py` calls no record writer itself).

`U/test_e2e_probe.py`, the driver run with `fake_claude.py`:

- `test_command_line_and_environment`: 3.5 exactly.
- `test_block_is_loud`, the loud-failure test: `num_turns: 0` with `is_error: false` gives exit 1,
  the cause `blocked_before_first_turn`, the failed check in `report.json` and the resume line.
- `test_no_workflow_and_inline_script_fail`; `test_all_checks_pass_on_scripted_delivery`.
- `test_budget_stop_shapes`: both shapes of 3.7, no `result` key: exit 1, `budget_stop`, the record
  printed, `--total` in the resume line.
- `test_each_check_fails_when_its_fact_is_broken`: one broken fact per counted check (green before
  red; a second commit on `main`; an open closure; an oversize record; thirteen starts; a `rule.error`
  line; a denial; a step's refused call in a transcript, each form of item 52, with no denial
  (`denied` names agent, tool and input; a hook's `reject` passes); an agent with no transcript; no
  payload; two run records in one session; for `tier_applied` a survey payload
  without the override, an `opus` agent under `cheap` and an `xhigh` payload under `cheap`).
- `test_tier_check_exempts_the_first_survey_step` (`--tier opus:low`, the survey agent on `sonnet`);
  `test_holdout_wall_is_reported_not_counted` (a builder reading `.factory/holdout`: `ok` stays true).
- `test_timeout_kills_and_resume_continues`: the first call starts one attempt only; the second loads
  nothing, releases the dead attempt's lock and writes `attempt-2`. `test_refuses_to_start`: each row
  of 3.4, with the word `sandbox`.
- `test_allowed_tools_cover_story_agents`: 3.5's list holds every built-in tool of the ten agent
  files but the architect's web tools.

## 8. Order of work

Before the builder starts: steps 3, 4 and 5 are built and reported, the survey unit's fix of
`story_ready.py` among them (ORDER-6, section 5). The real run needs W2 too, and the probes.

1. The three sample edits; `make_ready` and the change to `story_ready.py`; the whole pytest suite passes.
2. The overlay, `load.py`, `oracle.py`; the fixture tests but the scripted one.
3. `e2e_scripted.py` and `test_scripted_delivery_passes_the_real_gates`.
4. `probe.py`, `probes/story.py`, `run.py`, `fake_claude.py`; the driver tests.
5. Every check of ORDER-6, section 6.

The builder must not: start a real `claude` session; edit a file of steps 3 to 5 other than
`U/story_ready.py`; edit `P/`, `workflow-lib/`, `tests/workflows/`, `tools/`, `pyproject.toml` or
`rewrite/`; commit, branch or push. An interface that differs from section 4 is reported, not worked
around.
