# Step 6 plan review: `story-workflow.md` (W2) and `first-run.md` (E1 fixture, E2 first run)

Reviewer: Opus, after reading both plans in full; the step 4 plans with `ORDER.md`; the step 5 plan;
`rewrite/spec/platform.md`, `topics/platform-proven*.md`, `topics/story-script.md`,
`packaging/e2e-story-tier.md`, `packaging/tests-e2e.md`, `packaging/sandbox-needs.md`; and the code under
`next/` the plans build on (prelude, harness, universal tests, roster lint, survey, story unit, admission,
preconditions, checkpoint, config, artifacts, walls, hook runner capture, launcher, sample project).

Paths are under `next/` unless they start with `rewrite/` or `~`. "W2" is `story-workflow.md`, "FR" is
`first-run.md`, "ORDER-4" is `rewrite/plans/step-4/ORDER.md`, "agents.md" is the step 5 plan.

## Verdict

The clean path of the slice holds end to end on paper: every hand-over from the driver's command to the
merge and the checks matches by name, argument, field and refusal case, and the fixture's recipe is accepted
by the built survey (checked by running it, below). One correction must land in the step 5 build before W2
can be built (PL6-1). Five majors should be settled before the first real run: a spike request has no
deterministic end (PL6-2), nothing tests that the records the script leaves rebuild to the script's own
verdict (PL6-3), the first run rests on platform behaviour neither plan schedules a probe for (PL6-4), the
step 4 gates builder will meet a broken `story_ready(product=...)` (PL6-7), and the checks of the tier on the
real platform cannot tell whether the override applied (PL6-6 is minor only because a wrong tier costs money,
not the delivery).

## What I checked by running something

1. **The fixture recipe against the built survey.** In a scratch directory I ran FR 3.1 by hand:
   `make_sample_repo.sh -n`, the real `vsdd init` (answered `state: ready`), the sample's `factory.toml` with
   `delivery.mode`, `delivery.ci` and `state.push` set and the `[hooks]` block added (`config.load` accepts
   it), a contract, epic and story written to FR 3.2's rules, then the `make_ready` steps through the stores
   (`cycle_write`, `brief` and `spec` decisions opened and approved with `spec_digest` computed after the
   overlay, the plan closure closed with `{"W-01": ["S-1.01"]}`, `flush`). Results: the flush committed with
   nothing rejected; `spec_approved` true; `check_paths` empty (`SS-01` and `CAP-001` pass the constant
   patterns; their existence is not checked because neither kind has an id row); every precondition met; the
   checkout clean; and `vsdd survey story --subject S-1.01` with a caller marker and `VSDD_TEST_TIER=sonnet:low`
   answered `ok: true`, `workflow: "story"`, a run id, the twelve prelude limit keys,
   `config.tier_override = {model: sonnet, effort: low}`, and a unit at `ready` with `kind: feature`,
   `acs: [AC-1..AC-4]`, `tests_agent: test-writer`, `tests_method: failing-tests`, `impl_agent:
   implementer`, `impl_method: tdd`, `red_gate: true`, `demo_required: false`, the worktree and branch
   created. Nothing the fixture gives the story is refused by the survey as built.
2. **A kept platform session folder.** The round-4 control session `1cf1064b-…` under
   `~/.claude/projects/-private-tmp-…-scratchpad-pr4-project/` still exists. Its run record
   `workflows/wf_7042416d-9f1.json` has the keys `runId, timestamp, taskId, script, scriptPath, result,
   agentCount, logs, durationMs, summary, workflowName, status, startTime, phases, defaultModel,
   workflowProgress, totalTokens, totalToolCalls`; for a call by name its `script` and the file at
   `scriptPath` equal the plugin's workflow file byte for byte (`cmp` clean); each `workflowProgress` agent
   entry carries `agentType` and the full `model` id; the step transcript `subagents/workflows/<wf>/agent-*.jsonl`
   carries `tool_use` entries (Glob, Read) with their inputs, and `"effort":"low"`. The hook log of the same
   session shows that the `PostToolUse` input of a named `Workflow` call carries `script` too (the platform
   fills it in); only `PreToolUse` tells a named call from an inline one.
3. **`story_ready(product=<sample>)`**: confirmed what FR reported (PL6-7).

## The walk, hand-over by hand-over

| # | Hand-over | Both sides agree? |
|---|---|---|
| 1 | FR loader to the survey: records, digest, waves, config | Yes (run above). `spikes.per_run` is the one key W2 needs that FR does not and cannot set (PL6-2) |
| 2 | FR driver to the session: prompt `/vsdd-factory:story S-1.01`, `--plugin-dir plugin/`, default mode, `--allowedTools`, `--settings` sandbox off, ceiling variable 0, `VSDD_TEST_TIER` | Matches W2 3.1. Unproven parts: PL6-4 |
| 3 | Typed command to `Workflow({name, args})` | Item 31 (round 3, auto mode). Default mode with the isolation flags: PL6-4 |
| 4 | `parseArgs(args,'story',['repo'])`, `safe()` | Yes (prelude as built) |
| 5 | Survey relay: `vsdd survey story --subject S-1.01 --wait 540 --json`, no `-C` (RUN.dir empty), clerk tier | Yes. Hook approves the single `vsdd` call and adds the marker; survey reads `inv.environ` for the tier |
| 6 | Survey answer to the script: `ok`, `workflow`, `run_id`, `project_dir`, `plugin_root`, `config.tier_override`, `limits`, `units[0]`, `next` | Yes; limit keys are exactly `LIMIT_KEYS`; the three paths come with survey-2 |
| 7 | `tests`: `gated('test-writer','tests',…,'red',{method:'failing-tests', closing:'gate red --story S-1.01 --run <id>'})` | Yes: `brief()` adds `vsdd -C <dir> … --wait 540 --json`; red result `gate: red`, `sha` the head; M4 `head` = gate sha; `gated()` takes the first `red` entry; tests-first lets `source:before-red` write the stub; step-results checks the passing entry against `red.<sha12>.json` |
| 8 | `implement`: `tdd`, `gate green`, `S_IMPLEMENT_DATA` | Yes: green gives `diff_patch`, `test_output`, `changed_files: []`; `VSDD-STUB` must be gone |
| 9 | Review loop 1: `review-adversary-l1-p1`, `review-story-diff`, items `u.acs`, range `u.base...head`, `used` = `u.rounds_used`, cap `limits.passes` | Yes against verdicts.md 3.3 rules 2, 7 to 9: identity carries loop and pass, agent type `vsdd-factory:adversary` (item 36), one `reviewed_sha` equal to head, items are the `### AC-N` headings, coverage from `review-story-diff` alone |
| 10 | Fix steps (if any): `fix-test-writer-l1-p1` (`test-fixes`, re-freeze then green), `fix-implementer-l1-p1` (`fix-findings`, green) | Yes for a feature story; the tool joins `test-fixes` first, as the script does. Other kinds: PL6-1 |
| 11 | `deliver-1`: `vsdd story deliver --story --run --wait 540 --json` | Yes: rows 7 to 11 hold for this story (red record, green at head, `covering()` from the stored pass, demo table, no protected path or manifest) |
| 12 | `close`: `vsdd checkpoint story --run <id> --json` (no `--subject`) | Yes: the command takes `--run`, `--outcome`; delivered closes the closure, drops evidence, status `done` |
| 13 | Workflow result to the main session; result file; checks | `run_closed` reads the platform run record's `result.status` = the checkpoint's `done` (W2 D1). Checks: PL6-5, PL6-6, PL6-11 |

## Findings

### PL6-1 — blocker — W2 cannot be built until step 5 gives three agents `fix-findings`

Plans: `story-workflow.md`, `rewrite/plans/step-5/agents.md`.

W2 D6 sends every fix and repair to the kind's implement agent with method `fix-findings` (S5P-N10), and its
`contract` fails any call whose method is not in the agent's list; `each kind is fixed by its own builder`
runs every shipped `[[story-kind]]` row. agents.md section 3 gives the technical-writer only `docs`, devops
only `repo-setup` and the verifier only `proofs`, and `test_agent_files.py` holds each agent to section 3's
list. So W2's own tests fail on the `docs`, `chore` and `harness` rows, and W2 section 8 tells its builder to
stop. Nothing in `rewrite/STATE.md` next action 5 carries this into the step 5 build, whose notes hold only
ORDER-4 section 7 and probes-4-apply-C2-3. The same goes for `review-story-diff`: its rule "owner
`test-writer` for category `test` or `coverage`" must yield to the owner the task names, or a kind with no
red gate gets findings routed to a test-writer whose re-freeze fails (gates.md D16).

Resolution: put both corrections in the `notes` of the step 5 build: technical-writer, devops and verifier
list `fix-findings` (and `test_agent_files.py`'s method table follows); `fix-findings` says, for a story with
no red gate, that the agent fixes within its own write scope (docs, repository files, harness); and
`review-story-diff` takes each finding's owner from the task when the task names one for every category. If
the orchestrator does not want that in step 5, W2 must narrow `each kind is fixed by its own builder` to the
feature row and record the other rows as left for A5, which is weaker.

### PL6-2 — major — a spike request on the first run has no defined end; the fixture cannot set `spikes.per_run = 0`

Plans: `story-workflow.md`, `first-run.md`.

W2 section 4 needs "a fixture whose `factory.toml` sets `spikes.per_run = 0`" so that a spike request ends
`blocked`, `spike_limit` inside `step()` and never reaches `runSpike()`, whose agent type
`vsdd-factory:spike-runner` has no agent file on the slice (agents.md D20). FR sets three keys only (D2) and
does not mention it. It could not anyway: `config.py` types `spikes.per_run` (and `per_step`) as `INT1`, "an
integer of at least 1" (`plugins/vsdd-factory/py/vsdd/config.py` lines 51 and 162-165), so `config.load`
refuses 0 and the loader's own step 3 would fail. With the shipped 6, any builder or reviewer that returns
`needs_spike` sends the run into `agent()` with an unknown agent type, which is W2's own open question 3: it
may throw (the run ends `failed`), return null (an "inconclusive" finding and a restart), or run untyped
(then `vsdd spike open`, not built, is refused). Each ends the run, but which one, and what it leaves (an
isolated worktree, a lock), is unproven, and FR's diagnosis has no cause for it.

Resolution, in order of preference: (a) allow 0 for `spikes.per_step` and `spikes.per_run` in `config.py`
(an `INT0` kind; the spec's `tables/config-keys.toml` gives no minimum) as a small fix by its owner, FR's
loader sets `spikes.per_run = 0` as a fourth `set_key`, and FR adds a fixture test that the survey answers
`limits.spikes_per_run == 0`; or (b) W2 sets `RUN.limits.spikes_per_run = 0` after `applyLimits` with a
comment naming A3, S6 and H8, removed when they are built. Under either, FR's diagnosis gets the cause
`spike_limit` from a final step record that is `blocked` with that kind. If neither is taken, add the
unknown-`agentType` leg to the probe of PL6-4.

### PL6-3 — major — no test crosses the script's step results and the tool's rebuilt verdict

Plans: `story-workflow.md` (verdicts.md 3.3 provides the function).

Delivery merges only when G3's `covering()` finds a converged verdict rebuilt from the stored step records,
and the checkpoint opens decisions from the same rebuild. The script and the tool meet only there. W2's
scenario tests stub every agent with answers made to fit W2's own reading; verdicts.md's `records` cases are
hand-written records; the `loop` conformance cases compare `converge()` with the Python replay, not the
records the script's steps actually produce (names, loop and pass, method, agent type, the fix's gate entries,
a later loop's items). FR's `deliver_scripted` stores records "as `U/delivery_fixtures.py` does", not as the
script would. A mismatch at this seam (a later loop numbered or itemised differently, a fix whose passing
entry is not at its head, a lens whose method is not in `REVIEW_METHODS`) is first seen as a `refused`
delivery on the real platform. `rewrite/STATE.md` asks for exactly this kind of test: "add a test that
crosses both".

Resolution: W2's `test_story_workflow.py` gains `test_script_records_rebuild_to_the_script_verdict`. For the
scenarios clean, one blocking test finding (test-writer then implementer), a coverage gap, a dispute with a
`not_defect` ruling, the cap, a repair that adds a file (loop 2), and `review_stale`, it runs the shipped
`story.js` in node, turns each returned step result into a record the way the `step-results` rule stores it
(the call's `agentType` as `agent_type`, the result's `head`, in call order), and calls
`vsdd.state.verdicts.rebuild(records, subject=, cap=, waived=, acs=, changed=)` with `changed` answering from
the scenario's `--since` stubs. It asserts the tool's `outcome`, `reason`, `passes`, `rounds_used`,
`coverage_missing` and `chain` equal the verdict the script acted on, and that `skipped` is empty.

### PL6-4 — major — the first run rests on platform behaviour that no plan schedules a probe for

Plans: `first-run.md`, `story-workflow.md` (and the integrator's `ORDER.md`).

Both plans name open questions, but neither makes them a gate before the first story run, and some are not
named at all:

1. `--setting-sources project,local --strict-mcp-config` with a `--plugin-dir` plugin: do its hooks,
   workflows and `bin/` load (FR's own open question 1, D8)? Every round-4 session ran with the user's
   settings loaded, which on this machine include `skipWorkflowUsageWarning`; whether a session without them
   meets a workflow usage warning headless is not recorded either.
2. A typed workflow command with no skill, in the default mode. Item 31 was seen in round 3, which ran in
   `auto` mode (round-3 `CHECK.md`, "every round-3 session ran in the auto permission mode"); every round-4
   session entered through the typed skill `probecall`; and the C30 control saw the model reach for
   `Skill(...)` and be refused. W2 3.1 builds no skill on item 31 alone.
3. In the default mode with bare `Read`, `Edit`, `Write`, `Bash` allowed: a step's `Read` of
   `<plugin copy>/methods/<m>.md`, outside the working directory; an `Edit` and `Write` in a story worktree
   (FR's open question 2).
4. Item 26 with item 48: does a step's payload `effort.level` show the step's effort when the session's
   `--effort` differs? FR's `tier_applied` assumes it does (PL6-6).
5. What a `--max-budget-usd` stop leaves mid-workflow (FR's open question 4).
6. If PL6-2 is not resolved by configuration: `agent()` with an agent type that has no file.

Resolution: one probe round before E2's first run, made gating in `ORDER.md`: one or two headless sessions on
a scratch copy of the spike plugin, each started by the typed workflow command itself (`/<plugin>:<wf>`, no
skill), with FR's exact flags, `--effort medium` on the session, and a one-step workflow whose step (effort
`low`) reads a file under the plugin directory, edits and writes a file in a linked worktree, runs a plain
Bash command, and returns. A second leg sets `--max-budget-usd 0.05` on a three-step workflow. Pass: a
non-empty hook log, a workflow result, empty `permission_denials`, the step payload's `effort.level`
recorded. Well under a dollar. The results become platform items; FR's defaults (D8, the flags) follow them.

### PL6-5 — minor — the session folder is already readable; use it, and drop W2's request for a `Workflow` hook

Plans: `first-run.md`, `story-workflow.md`.

FR's open question 3 asks for a platform item for the session folder; W2's open question 1 asks whether the
file at `scriptPath` equals the plugin's file. Both are answered by the kept round-4 folder (section "What I
checked", item 2): for a call by name, `script` and the file at `scriptPath` equal the plugin file byte for
byte; `workflowProgress` gives each agent's `agentType` and full `model` id; the step transcripts carry
`tool_use` entries (streamed, so each id appears more than once). Two consequences:

- W2 3.1 (a) and its "needs" ask the driver to log the `Workflow` call's `PreToolUse` and `PostToolUse`
  "with a logging hook on `Workflow`". FR builds no such hook (the plugin's matcher is
  `Bash|StructuredOutput`, and rules.md makes it `Bash|Edit|Write|MultiEdit|NotebookEdit|StructuredOutput`,
  never `Workflow`), and does not need one: item 49 itself prescribes the run-record check that FR's
  `named_workflow_ran` makes. Note also that the `PostToolUse` input of a named call carries `script`, so a
  "no `script`" test is only valid on `PreToolUse`.
- FR D11 says the model is seen only in `modelUsage`. It is in `workflowProgress[].model` per agent, with
  the agent type beside it, which attributes a model to each step; `modelUsage` cannot, and FR gives no
  mapping from the roster's aliases (`sonnet`, `opus`, `fable`) to the full ids it holds.

Resolution: the orchestrator records the folder's shape as a platform item (2.1.331, default mode, kept
evidence path). FR's `tier_applied` takes each step's model from `workflowProgress` (matching by `agentId` to
the payloads' `agent_id`), maps an alias to an id by the alias being a segment of the id, and keeps
`modelUsage` only for the main session. FR's `fake_claude.py` and `deliver_scripted` copy that record shape.
W2 drops the hook request from its needs and keeps (b), the byte check, as the driver's.

### PL6-6 — minor — `tier_applied` cannot tell whether the override reached the steps

Plans: `first-run.md`, `story-workflow.md`.

Under the default `cheap` tier the override is `sonnet:low`, the session runs `--model sonnet --effort low`,
and the clerk tier is `sonnet`/`low`. Every payload's `effort.level` is then `low` whether or not the override
was applied, if the payload reports the session's effort (item 26 is open; item 48 was seen with both `low`).
So the run that is meant to settle item 18 passes `tier_applied` vacuously on effort. Second, the first clerk
step, the survey, runs before the script knows the override, at the clerk tier: with any `--tier <model[:effort]>`
other than `sonnet:low`, `tier_applied` ("the override's" effort for every payload with an `agent_id`) fails
on it, and W2's `a tier override reaches every step` must exempt it too.

Resolution: FR reads `config.tier_override` from the captured `PreToolUse` payload of the survey clerk's
`StructuredOutput` (it is the survey's output, verbatim) and asserts it equals the tier asked for: that settles
item 18 directly. The session runs at an effort no probe tier uses (`--effort medium` with `cheap`), so the
payloads' effort distinguishes the step's effort from the session's once PL6-4 item 4 is answered. Both
`tier_applied` and W2's contract exempt the first survey step (the clerk before `RUN.tier` is set), and say so.

### PL6-7 — major — `story_ready(product=<sample>)` copies the sample's `factory.toml` over the project's (step 4 gates)

Plans: `rewrite/plans/step-4/gates.md`, `rewrite/plans/step-4/survey-2.md`, `first-run.md`.

FR reported it; I confirmed it. `tests/unit/story_ready.py` `_install_product` (lines 132-150) copies every
child of the product directory, `.factory/` included, then sets `paths.source` to every top-level name. On
`tests/e2e/sample-project` the result is `delivery.mode = "pr"`, `state.push = true` and
`paths.source = [".factory/**", ".gitignore/**", "README.md/**", "tally/**", "tests/**", "tools/**"]`, so
`tests/**` is both a tests and a source class. G2's `tests/unit/gate_project.py` is built on exactly this call
(gates.md section 7), and survey-2's real-gate tests and delivery's fixtures build on `gate_project`. A red
gate that does not count a source-class test (`test_source_class_test_does_not_count`) and a tests-first rule
that blocks a `source` write after red would both misread the sample's tests; delivery would refuse at row 4.

Resolution: before stage A of step 4, the survey unit (owner of `story_ready.py`, R1) skips `.factory` when it
copies a product directory and sets `[paths]` from the product's own `factory.toml` when it has one (the sample's
is `tests = ["tests/**"]`, `source = ["tally/**", "tools/**"]`); `gate_project.py` asserts both. FR's later edit of
`story_ready.py` (only the three record writers become `make_ready`) keeps that code as it is.

### PL6-8 — minor — the clerk would again get two texts that disagree about a call moved to the background

Plans: `story-workflow.md`, `rewrite/plans/step-5/agents.md`.

W2 D11 adds to `relay()`: "If the call is moved to the background, run the same command again", for every
relayed command, on the ground that every one is safe to repeat. The ledger entry probes-4-apply-C2-3, which
`rewrite/STATE.md` (next action 5) sends into the step 5 build, limits that sentence to job commands
(`story deliver` here): `survey` and `checkpoint` return directly, and a moved call of theirs is waited for,
as the platform's notice says ("do not start a second copy while it runs"). Safe to repeat in sequence is not
safe to run beside a copy still running. With both built as planned, the clerk file and the relay prompt
disagree, which is the fault probes-4-apply-C2-1 was raised for. On the first run neither command should run
near 600 s, so this is not expected to fire.

Resolution: one text in both places: "If a `vsdd story deliver` call is moved to the background, run the same
command again: it re-attaches to its job. A moved call of any other command: wait for it, as the notice says."
W2's `every clerk prompt carries both sentences` tests that text, and step 5's clerk file carries it.

### PL6-9 — minor — W2 D4 rests on survey behaviour the built code no longer has

Plans: `story-workflow.md`.

W2 D4 and its "needs" say the built "could not build the story unit" path keeps the lock and sets `run_id`,
so W2 closes such answers with `--outcome failed`. As built (`py/vsdd/state/survey.py` lines 151-164), that
path releases the lock and empties `run_id` (finding S3R-survey-C1-1); survey-2's re-measure failure does the
same. The branch is harmless as a defence, but the residual case is the other one: a release that itself fails
leaves the lock held with `run_id` empty, and the error says "the lock release left a fault". W2 cannot close
it, and nothing in W2 says so.

Resolution: W2 restates D4 against the code (an `ok: false` survey with a `run_id` is not expected; the branch
stays as a defence, with its test), and names the release-fault case as the survey's, recovered by the next
survey after the lock goes stale or, in the driver, by FR's refusal 7 (`vsdd lock release --force`).

### PL6-10 — minor — a `--since` green result without `changed_files` would read as "nothing to review"

Plans: `story-workflow.md`, `rewrite/plans/step-4/gates.md`.

W2 D8 takes a later loop's items from the `--since` result's `changed_files`. gates.md 3.1 promises the list
on every green result, but `gate-result.schema.json` keeps it optional, and the `step-results` rule checks
only `gate`, `subject`, `sha` and `passed` of an entry against its record: a model's copy that drops the list
passes every check, and an empty item list makes the script's loop converge on nothing while the tool's
`changed()` names files. Also, W2's table does not say that the test-writer's green command in a later loop
carries `--since <from>` (D8 says the fixes close with it; the test-writer's green is in its task text, not its
closing line).

Resolution: W2 treats a `--since` entry with no `changed_files` array as an unusable result (the step is
`failed`, with a summary naming the field), and its table and fix task give the test-writer the green command
with `--since <from>` in a later loop; one scenario test each. gates.md may make `changed_files` and
`test_output` required on a green result in the shipped schema, the way the wave gate's schema has them.

### PL6-11 — minor — `holdout_wall` measures nothing on this fixture

Plans: `first-run.md`.

The overlay has no `.factory/holdout/`, and read walls (H2) are not on the slice (rules.md section 1). So the
check either passes because no file exists to read, or, with a holdout file added, fails for a rule the slice
does not enforce. It costs a transcript reader and a payload scan for no evidence.

Resolution: keep the reader (the transcripts carry `tool_use` entries, PL6-5), but report `holdout_wall` as
"not applicable: the overlay has no holdout and H2 is not built" without counting it toward `ok`, and make it a
pass condition when E2 widens with a holdout overlay and H2.

### PL6-12 — minor — ORDER-4's sentence on evidence after a repair contradicts gates.md D14

Plans: `rewrite/plans/step-4/ORDER.md`, `rewrite/plans/step-4/gates.md`, `story-workflow.md`.

ORDER-4 section 7 says the script does not record evidence again after a repair, "so a `demo: true` story is
refused at the demo row and the next run finishes it". gates.md D14's demo pass rule for `demo: true` is a
file check (report, a script and a transcript per criterion, caps) plus green covering the head: the old
recording passes at the new head, so delivery would merge a story whose demo predates the repair. W2 D9
records again (`evidence-<r+1>`), which is right either way. Not on the slice's path (`demo: false`).

Resolution: the integrator strikes ORDER-4's sentence in favour of W2 D9, and gates.md D14 (or G2 when
widened) records the commit a recording was made at and refuses a recording older than the head.

### PL6-13 — minor — the sample's `.gitignore` does not cover tool caches a builder may create

Plans: `first-run.md` (it already edits two sample files).

The story worktree must be clean at every gate and at delivery row 6 (`git status --porcelain
--untracked-files=all`), and M3 tells builders to "commit everything". The sample ignores `__pycache__/`,
`build/`, `.venv/`, but not `.pytest_cache/`, `.ruff_cache/` or `.mypy_cache/`. A Sonnet builder that runs
`uvx pytest` or `ruff` in the worktree out of habit either fails its gate on dirt or commits the cache, and then
FR's `integration_holds_change` ("changes only files under `tally/` and `tests/`") fails a delivered story.

Resolution: FR adds the three caches to `tests/e2e/sample-project/.gitignore` beside its README and
`factory.toml` edits, and `test_sample_commands_run_offline_and_leave_nothing` covers the file.

### PL6-14 — minor — attempts and spend across attempts have no stated bound

Plans: `first-run.md`.

3.7 makes a resume the person's re-run with `--dir`; open question 4 says "until then the driver treats any
subtype other than success as a failed attempt and resumes". If that means automatic, nothing bounds the
number of attempts, and `--max-budget-usd` caps one session only, so the spend across attempts is unbounded.

Resolution: say that the driver never starts an attempt by itself; `report.json` sums the cost over attempts,
and the driver refuses an attempt (exit 2) when the recorded spend plus the next cap exceeds a total for the
tier (cheap 12, roster 60, or `--budget`).

### PL6-15 — minor — scenario tests that pass with the script wrong

Plans: `story-workflow.md`.

Beyond PL6-3, which is the important one: there is no test that a reviewer whose `reviewed_sha` is not the
head ends the run `failed` with `no_result` (path 6 states it); none that a later loop's fix steps and the
test-writer's green command carry `--since <from>` (D8; PL6-10); the default step stub answers "a passing entry
of the gate its closing line names", so no scenario ever returns a green result shaped as the real gate prints
it after `--since` (with `changed_files` and the range patch) unless a test overrides it; and the `contract`'s
model and effort assertion must exempt the first survey call under an override (PL6-6).

Resolution: add `a reviewer on another commit ends failed`, `a later loop fixes close with --since`, and make
the default green stub for a `--since` closing return `changed_files` and a range `diff_patch` built from the
scenario's changed files.

### PL6-16 — minor — nothing durable shows a delivered story's test-first order

Plans: `first-run.md`, `rewrite/plans/step-4/survey-2.md`, `rewrite/plans/step-4/delivery.md`.

For a story delivered in one run the checkpoint writes only the `review` and `deliver` stage entries, then
drops the evidence (survey-2 3.2, 3.5); the squash merge deletes the branch (delivery D12). FR therefore proves
test-first from the run's local step records and from unreachable loose objects read at once after the session
(FR D4). That works for the first run, and loose objects are not pruned for two weeks by default, but a clone,
or the same machine later, cannot show it. FR asks survey-2 to also write `tests` and `implement` entries from
the run's passing gate entries for a delivered story.

Resolution: this is the person's question FR raises, extending `rewrite/STATE.md` "Waiting on the user" item
6; until it is answered the first run proceeds as FR plans. If the answer is to record it, survey-2's
`assess` writes the two entries for a delivered story from the run's step records (their passing `red` and
`green` entries carry the shas), which needs no evidence.

## Built beyond one story on one machine

Little. FR's `Probe` form, its general checks and its diagnosis are sized for one probe and stated as the
shape later probes reuse; that is reasonable. `holdout_wall` is the one check that buys nothing here (PL6-11).
W2 handles `pr`-mode states, the child form and the conflict and `review_stale` paths because the spec's
playbook has them; their scenario tests are cheap and keep the shipped file honest, so I would not cut them.

## For the person

1. PL6-2: may `spikes.per_run = 0` become a valid setting (no spikes for this project), or should the story
   script ignore spike requests until the spike-runner is built?
2. PL6-16 (FR's question, extending waiting item 6): should a delivered story keep a durable record of its red
   and green commits (in the closure, or as a kept ref)?
3. W2's own question stands: should the product refuse an inline `Workflow` script by a hook rule? Until then
   only the headless driver can tell which script ran.
