# Review pass 3: coverage and platform truth

Date: 2026-11-24. Branch `rewrite/v1`. Reviewer prefix `P3COV-`. Read in full: `rewrite/STATE.md`, the blueprint
sections 1 to 7, 9, 10 and 11.4, and the parts of the five designs that carry the mechanisms below; the four
surveys in `rewrite/reference/` and the platform facts file; and, to sample the mapping tables, the old
`agents/orchestrator/steady-state.md`, `maintenance-sequence.md` and `skills/repo-initialization/SKILL.md`.
R1 to R5 of `pass2-closeout-3.md` are not repeated.

## Verdict

The design respects every platform limit the probes established: no script reads a file or runs a command,
only top-level workflows call `story`, no step relies on the Agent tool, steps are held on their
`StructuredOutput` call and never on a refused stop, spikes pass isolation themselves, hook timeouts are in
seconds, and long commands re-attach inside the ten-minute cap. The problems found are of two kinds. On the
slice path, three mechanisms rest on things that are not true or not yet known: local delivery merges into a
branch the person's own checkout holds, which git will not let the tool do without touching that checkout
(P3COV-01). The fallbacks for the two unproven caller-identity probes identify a caller by its working
directory, which is the shared project root for every story step (P3COV-02). And the slice plan leaves out the
probe package and two other packages that its own packages depend on (P3COV-03). On coverage, four old
capabilities are designed but cannot run as written: measured checks with no command or threshold key
(P3COV-04), test-writer methods no step can select (P3COV-05), notifications with no package (P3COV-06), and a
few smaller gaps. One blocker, five majors, eight minors.

## Findings

### P3COV-01. Local-mode delivery has no way to merge into an integration branch the person's checkout holds

- Severity: blocker. On the slice path: yes (G4a in local mode, the slice's delivery step).
- Locations: `pipeline.md` 2.9.1, Details, the `vsdd story deliver` bullet ("With `delivery.mode = "local"` (no
  remote) it merges locally and skips the pull request and CI"); `state.md` 2.12 `[delivery] mode` and 2.16,
  `vsdd story deliver` and `vsdd release` rows; blueprint 4.12 (`delivery.mode`), 9.2 G4a ("local mode");
  `hooks.md` 2.11.4 (`merge-guard`: "in local delivery mode, where the integration branch is the product
  branch"); `state.md` 2.3 table and `pipeline.md` 3.6 ("Nothing in 1.0 pulls or merges in the person's
  checkout"); the first-pass finding OPS-30, which states the same situation.
- Problem: in local mode the integration branch is the branch the person's main checkout has checked out (a
  repository with no remote, as in the slice and in every new greenfield project). The design says only that
  the tool "merges locally". Git refuses to check that branch out in `.worktrees/_integration/` ("already
  checked out"). Building the merge in a detached worktree and moving the ref with `update-ref` leaves the
  person's index and working tree at the old commit, so `git status` there shows the story's changes reversed as
  staged edits, and the next commit from that checkout reverts the story. Merging in the main checkout breaks
  the rule that the person's checkout is never touched. No rule says which of these happens.
- What breaks: the first story of the slice cannot be delivered as specified. A builder must choose between a
  git error, a corrupted-looking checkout, or breaking a stated rule. `vsdd release` in local mode has the same
  gap for the release branch.
- Resolution: state one rule in `pipeline.md` 2.9.1, `state.md` 2.16 (`story deliver`, `release`) and blueprint
  4.12. The smallest that keeps low touch: the tool makes the merge commit in `.worktrees/_integration/`
  (detached), then, when the person's checkout holds the integration branch and is clean, fast-forwards it with
  `git -C <root> merge --ff-only <merge sha>`; when it is dirty, delivery returns `awaiting_merge` with the
  reason and the one command that finishes it. Record this as the one stated exception in the `state.md` 2.3
  table and `pipeline.md` 3.6, and add a G4a fixture with the main checkout on the integration branch, clean
  and dirty.

### P3COV-02. The fallbacks for the caller-identity probes key on the working directory, which is the project root for every story step

- Severity: major. On the slice path: yes (H1 caller markers, S2b lock, S3 survey's hooks-alive check).
- Locations: blueprint 4.9 ("How the tool knows who is calling", last sentence), 4.13 ("Liveness"), section 10
  items 27, 28 and 29; `hooks.md` 2.11.10 (caller markers: "matched to an agent by `cwd`, which is unique for
  every isolated agent"; `lifecycle-log`), 2.11.10 `extended` scope ("inside the caller's own worktree"), 2.16
  rows for items 27 and 28; `state.md` 2.11 (marker fallback; "from a working directory inside a worktree the
  locked run owns"); `pipeline.md` 2.11.1 `step()` (no `isolation` option is passed).
- Problem: story stages run as ordinary steps. `step()` passes no isolation; the tool's own worktree
  `.worktrees/<id>/` is named in prose. So the payload `cwd` of every test-writer, implementer, reviewer and
  clerk step is the project root, the same for every story running in parallel, and a `cd` inside a command
  does not change it. Only spike runners are platform-isolated. Three fallbacks depend on a unique `cwd`:
  - item 27 failing: markers are matched by process and `cwd`, so concurrent steps in a wave cannot be told
    apart (`env start` records the wrong agent; the lock and the asker are attributed wrongly);
  - item 28 failing: the lock is refreshed by calls "from a working directory inside a worktree the run owns",
    which no story step makes, and the main session makes no tool calls while the Workflow tool runs, so the
    lock goes stale after 20 minutes in the middle of a wave and a second session's survey takes it over;
  - item 29 failing as well: blueprint section 10 says "item 27 must pass". There is then no fallback, and the
    survey refuses every run.
  The `extended` scope's "inside the caller's own worktree" has the same flaw for builders.
- What breaks: if probe 27 or 28 fails, which B8 has not yet tested, the slice's survey or lock is built on a
  premise that does not hold, and two runs can write the same scope at once.
- Resolution: in `hooks.md` 2.5 (`Ctx`) define "the caller's worktree" as the worktree that holds the call's
  target path, or the directory of its `cd`, `git -C` or `vsdd -C` prefix, never the payload `cwd` alone, and
  use that definition in `lifecycle-log`, `auto-allow` and the marker fallback. Add to the liveness rule (4.13,
  `state.md` 2.11, `hooks.md` 2.11.10): any `vsdd` call carrying `--run <id>` of the locked run refreshes the
  lock. Give items 27 and 29 together a fallback that needs neither a rewrite nor a process walk: the hook
  writes the marker keyed by a hash of the session id and the tokenised command; the tool hashes its own argv
  (`vsdd` plus `sys.argv[1:]`, which matches exactly because clerk commands are single segments with
  percent-encoded text) and takes the newest unread marker with that hash, or the strictest one when several
  match. Change the fallback cells of items 27, 28 and 29 to match.

### P3COV-03. The slice plan omits the probes, policy tables, roster and decision store its packages depend on

- Severity: major. On the slice path: yes.
- Locations: `rewrite/STATE.md`, "Thin slice, layer 1" (units 1 to 4: B0, B1, B2, B3, C1, B4a, B5) and plan item
  4, layer 2 (S1, S2a, S2b, S3, G1, G2, G3, G4a, H1, H5, H7, B4b, A1, A4, A6a, W2); blueprint 9.1 (B4a depends on
  B6; B8), 9.2 (S1 depends on C2), 9.3 (H7 depends on S4; `hooks.md` 2.4 `policy.py` loads the three policy
  tables that H1 and H5 read), 9.4 (A1 depends on B8).
- Problem: B8 is in neither layer, yet A1 depends on it and its open items decide slice code: 27 to 29 (H1's
  rewrite and marker, S2b's lock), 31 (the argument parser in B4a), 34 (the percent-encoder in B4a), 6 (the
  ten-minute call) and 30. C2 (`artifacts.toml` with the story kinds, `classes.toml` with the precedence) is
  needed by S1 and by `tests-first`; B6 (`tests/lint/roster.toml`) is what B4a generates the tier, role and
  override tables from; H7 depends on S4.
- What breaks: layer 1 builds the prelude without its source table, layer 2 builds the caller-marker and lock
  design before learning whether its platform premise holds, and P3COV-02's failure cases are discovered only
  in the end-to-end run.
- Resolution: add to the slice the B8 probes for items 6, 27, 28, 29, 30, 31 and 34, run before H1 and S2b; add
  C2 and the roster part of B6 to layer 1; either add S4 or split H7 so `step-results` ships without the
  `decisions` rule. Record the change in `STATE.md` and blueprint 9.

### P3COV-04. Several measured checks have no command or threshold key, so the tool has nothing to run

- Severity: major. On the slice path: no.
- Locations: `state.md` 2.12 (`[commands]`; "The example above is the whole key list ... an unknown key is an
  error"), 2.16 (`gate wave`, `release`, `ingest`, `sweep`, `integration` row); `pipeline.md` 2.9.2 Details
  (clone fidelity), 2.9.6 `measured` row (`perf` "when budgets are configured"; "a fuzz target crashes"),
  2.9.8 ("mutation and coverage thresholds"), 2.9.10 measured sweeps, 2.9.11 `integrate` stage; blueprint 4.6
  (`facade` row), 4.12, 4.13 ("apply mutation and coverage thresholds"). Old sources: `maintenance-sequence.md`
  sweeps 5 (10 percent regression threshold) and 6 (clones checked "against real APIs"); skills
  `formal-verify` (cargo-fuzz), `perf-check`, `dtu-validate`; `multi-repo.lobster` cross-repository gate; agent
  `performance-engineer` (Core Web Vitals, bundle budgets).
- Problem: the closed key list has `build`, `test`, `lint`, `format_check`, `run`, `ui_check`, `docs_check`,
  `mutation`, `scan`, `audit`, `bench` and `proofs`. Nothing names how to run the clone-fidelity harness, a fuzz
  target (with its time bound), the cross-repository integration suite, or a coverage measurement, and there is
  no key for a coverage threshold, a performance budget or a regression percentage. Clone fidelity is compared
  only against "the recorded behaviour of the real service"; nothing records that behaviour or re-measures the
  real service, which was the point of old sweep 6.
- What breaks: the wave gate's fidelity check, the release gate's fuzz and perf checks, the coverage thresholds
  and `gate integration` cannot be implemented, and a project cannot add the missing keys because unknown keys
  are errors. The DTU, fuzzing, performance-budget and cross-repository capabilities do not port as checks.
- Resolution: add to `state.md` 2.12 and blueprint 4.12 `commands.fidelity`, `commands.fuzz` (taking
  `{seconds}`) with `limits.fuzz_seconds`, `commands.coverage` with `coverage.min` by criticality (or delete the
  coverage thresholds from 4.13, 2.9.8 and `state.md` 2.8), `[perf] regression_pct` with a budget file under
  `specs/architecture/`, and a hub-level `commands.integration`; say in `pipeline.md` 2.9.2 and 2.9.10 who
  records the real service's behaviour (a `clone-fidelity` harness story with a `blocked` decision of reason
  `environment` when credentials are missing), and that the maintenance sweep re-records it to detect drift.

### P3COV-05. Test-writer methods cannot be selected, and four methods are reachable from nowhere

- Severity: major. On the slice path: yes (`story.js`, W2), though the slice's own story does not hit it.
- Locations: `pipeline.md` 2.9.13 `story` tests row ("`ui-tests` or `e2e-tests` when the story's `method` key
  says so"), 2.11.2 `story.js` (tests stage passes `method: K.tests[1]`; only the implement stage uses
  `u.method`), 2.9.7 (no technical-writer step); `state.md` 2.5 Story row (`method`: "another method of the
  kind's implement agent"), 2.12 `[paths] docs` (no `CLAUDE.md`); blueprint 4.4 (`e2e-tester` row: tests written
  by `test-writer`, method `e2e-tests`), 4.6; `agents-skills.md` 2.8 (test-writer `ui-tests`, `e2e-tests`;
  technical-writer `claude-md`, `release-notes`; devops `observability`), 3.2 rows 98 (`scaffold-claude-md` to
  `setup`, method `claude-md`) and 107 (`storybook-mcp-integration`, method `ui-tests`); `packaging.md` L-METHOD.
- Problem: a story has one `method` key, defined as an implement-agent method, and `story.js` never applies it
  to the tests stage, so `ui-tests` and `e2e-tests` are never run. A UI story cannot name both `ui-tests` and
  `ui-implementation`. `claude-md` is named by no step, skill or `author` type, `setup` does not run it, and
  `CLAUDE.md` falls in no path class, so the write would be blocked. `release-notes` is named by no step,
  because `vsdd release` renders the change log itself. `observability` has no caller, because `observe` runs
  `vsdd obs` in the main session. L-METHOD only checks that some agent file names each method.
- What breaks: end-to-end journey tests (old `e2e-tester`) and UI component tests are never written, and the
  mappings for `scaffold-claude-md` and `storybook-mcp-integration` claim a port that does not exist.
- Resolution: add a story key `tests_method` (a test-writer method) to `state.md` 2.5 and blueprint 4.6, and use
  it in `story.js` for the tests stage. Add a lint or harness check that every method is reachable from a
  2.9.13 row, an `author` type, a `review` lens or a story key. For `claude-md`, either add `author claude-md`
  and put `CLAUDE.md` in the `docs` default globs, or drop the method and correct row 98 with the reason
  (agents omit `CLAUDE.md`; conventions live in `conventions.md`). Either add a technical-writer
  `release-notes` step to `release` or drop the method. Drop `observability` from `devops`.

### P3COV-06. Notification when a decision opens is designed but no package builds it

- Severity: major. On the slice path: no.
- Locations: `pipeline.md` 2.6 (last bullet: "the tool notifies through the channel in project configuration"),
  3.4 (last row: the old notification table is "replaced"), 5.1 S1 (`notify.channel`); `state.md` 2.12
  (`[notify] channel`, `command`), 2.16 `vsdd checkpoint` row (does not mention it); blueprint 4.9 `checkpoint`
  row, 4.12 and 9.2 (no mention, no package). Old source: `agents/orchestrator/steady-state.md`, "Human
  Notification System"; `maintenance-sequence.md`, "Notifications".
- Problem: the only replacement for the old notification system is one sentence in `pipeline.md` and two keys
  in `state.md`. The blueprint, which is the authority, does not mention it; no command or function performs
  it; and no build package delivers it.
- What breaks: scheduled `maintain` and `discover` runs, and an unattended `drive`, open decisions that nobody
  sees until the next session starts. That defeats the low-touch requirement for exactly the runs it is meant
  for, and a capability of the old system is dropped without a stated reason.
- Resolution: in `state.md` 2.16 add to the checkpoint row: when it opens a decision it runs the configured
  channel (`github-issue`: one issue per decision; `command`: the configured command, detached, with the
  decision id, kind and summary in `VSDD_*` variables, and any failure logged). Name the channel keys in
  blueprint 4.12, assign the work to S3 in 9.2, and add a fixture.

### P3COV-07. Which permission decision travels with the rewritten command is not stated

- Severity: minor. On the slice path: yes (H1).
- Locations: `hooks.md` 2.5 (`Permit`; "the runner may return `updatedInput` for a `vsdd` Bash call"),
  2.11.10 (built-in `auto-allow`: "for the main session only read-only subcommands ... anything that changes
  state still asks"); blueprint section 10 items 7 (static-rule fallback) and 27 (test: "the command prints
  it").
- Problem: the platform documents `updatedInput` as paired with `permissionDecision` `allow` (approve) or `ask`
  (show the person). The design emits the rewrite on every `vsdd` call but says nothing for calls it does not
  permit, or where another rule advised. With `allow` it would approve state-changing main-session calls under
  scope `off`, against D4. With `ask` it forces a prompt in `auto` mode. With no decision the rewrite may be
  ignored, and then the survey refuses. The rewritten text `VSDD_CALLER=<key> vsdd ...` also no longer matches
  the static `vsdd` allow rules that item 7's fallback writes, or a person's own rules.
- What breaks: the D4 scope is either widened or produces unexpected prompts, depending on a choice nobody made.
- Resolution: state in `hooks.md` 2.5 that the rewrite goes with `allow` when `auto-allow` permits the call and
  with `ask` otherwise. Extend probe 27 to the three cases in the default and `auto` modes, and write item 7's
  static fallback rules in a form that matches the prefixed command.

### P3COV-08. Spike admissions are paired on an asynchronous event that nothing orders before `spike open`

- Severity: minor. On the slice path: no.
- Locations: `hooks.md` 2.3 (`SubagentStart`, `async: true`, "`spike-bounds` (pairs a starting spike runner
  with its admission)"), 2.11.6 "Who asked" row; `state.md` 2.16 `vsdd spike open` ("refuses when no asker can
  be established"); blueprint 6 and section 10 item 35. Platform facts: async hooks "run in background, no
  timeout enforced".
- Problem: for a spike runner dispatched with the Agent tool, `spike_attach` runs in an async hook, which may
  finish after the runner's first `vsdd spike open`. That command then finds no admission and refuses.
- What breaks: spikes requested by agents in a normal session fail at random. This is the entrance that walled
  reviewers use.
- Resolution: let `vsdd spike open` claim the oldest unclaimed admission of the caller's session itself when the
  marker's agent id has none, or pair in the synchronous `PreToolUse` rule on the runner's first guarded call.
  Add the ordering case to probe 35.

### P3COV-09. Platform assumptions the design uses that section 10 does not list

- Severity: minor. On the slice path: no (workflow steps carry the plugin root in the prompt).
- Locations: `agents-skills.md` 2.7 (`Read ${CLAUDE_PLUGIN_ROOT}/methods/<name>.md`), 7 probe V1 (fallback "adds
  the plugin root ... at `SubagentStart`") and V3; `packaging.md` 2.13 N1 (fallback states the substitution as
  fact); `hooks.md` 2.3 (`SubagentStart` is async), 2.11.2 (final-message check); `agents-skills.md` 2.12
  (`research`, `retro`: `context: fork` with no `background`); blueprint section 10 items 10 and 32; platform
  facts section 4 (`background`: "fork only; default true").
- Problem: four assumptions are relied on but are neither proven nor listed:
  - `${CLAUDE_PLUGIN_ROOT}` is substituted in an agent body, which every agent dispatched outside a workflow
    needs. V1's fallback cannot work, because output from an async `SubagentStart` hook is ignored.
  - The final message is available to the `SubagentStop` rule.
  - Forked skills return synchronously, whereas the docs default them to background. Both `research` and
    `retro` rely on the reply reaching the main session (retiring superseded reports, showing the adopt
    command).
  - Agents with explicit tool lists can call MCP tools without `ToolSearch`. The platform's workflow reference
    says workflow agents reach MCP tools through it. Item 10 and V3 give different fallbacks, and the
    researcher's "fall back to `WebSearch` and `WebFetch`" may be deferred in the same way.
  The workflow concurrency cap is also min(16, CPUs minus 2), not 16, which matters only for timing on CI
  runners.
- What breaks: on-demand reviews, `author` and spikes in a normal session may not find their method file;
  research may run with no source tools; forked skills may not complete as described.
- Resolution: add these four to section 10 and B8. Set `background: false` on the two forked skills. Make V1's
  fallback "the dispatching prompt names the plugin root", as step prompts already do. Choose one fallback for
  item 10, and include `WebSearch` and `WebFetch` in its probe.

### P3COV-10. Scripts call `vsdd checkpoint` with `--subject`, which the command does not take

- Severity: minor. On the slice path: yes (the standalone story's checkpoint releases the lock).
- Locations: `pipeline.md` 2.11.2 `end()` and 2.11.3 (`vsdd checkpoint story|wave --run ... --subject ...`);
  blueprint 4.9 `checkpoint` row; `state.md` 2.16; `pipeline.md` 5.1 S7 (all without `--subject`).
- Problem: the authoritative signature is `checkpoint <workflow> --run <id> [--outcome failed]`.
- What breaks: a strict parser exits 2, the clerk returns `ok: false`, and the run's lock is not released.
- Resolution: drop `--subject` from both scripts, since the run record knows its subject, or add
  `[--subject <id>]` to the signature in all three places.

### P3COV-11. Mapping rows that still describe removed mechanisms

- Severity: minor. On the slice path: no.
- Locations and problems:
  - `agents-skills.md` 3.2 row 8 (`check-input-drift`: "Staleness check in the lint tool, shown by `status`")
    and `hooks.md` 3.1 row 18 ("reported by `vsdd stale`"). The second pass removed staleness and `vsdd stale`
    (blueprint 4.14, 5.2).
  - `pipeline.md` 3.5, "Phase 0 approval" row ("`ingest` ends with a report and waits for the person's
    intent"). `ingest` now ends at decision `spec` (2.9.8).
  - `hooks.md` 3.1 row 60 ("Lessons are an artifact written by a workflow step"). There is no lessons register.
- What breaks: the mapping is the record that every capability ports, and these rows point builders at things
  that do not exist.
- Resolution: rewrite rows 8 and 18 to "dropped: the decision digest and the specification lock make drift
  impossible after approval" (as `packaging.md` 3 does for `compute-input-hash`), correct the Phase 0 row to
  decision `spec`, and point row 60 at the retrospective report.

### P3COV-12. The Storybook server step promised to UI projects is not in `setup`

- Severity: minor. On the slice path: no.
- Locations: `agents-skills.md` 2.10 ("The `setup` skill tells the person to register it under the name
  `storybook`"), 3.2 row 107 ("`setup` registers the server"); `agents-skills.md` 2.12 `setup` row (steps 1 to 5)
  and blueprint 4.5 `setup` row, which have no such step.
- Problem: four agents list `mcp__storybook__*` tools, but nothing in the specified `setup` tells a UI project to
  provide the server.
- What breaks: the Storybook-backed UI methods silently fall back to `index.json` on every UI project.
- Resolution: add the step to the `setup` row in both places, with a `vsdd doctor` check when `project.ui` is
  true.

### P3COV-13. The remote half of `repo-initialization` is dropped without a reason

- Severity: minor. On the slice path: no.
- Locations: `agents-skills.md` 3.2 row 93 (`repo-initialization`, merged into `setup` and method
  `repo-setup`); the old `skills/repo-initialization/SKILL.md` (`gh repo create`, branch protection on the
  integration branch); `state.md` 2.12 (`delivery.mode` is detected once, by `vsdd init`).
- Problem: neither `setup` nor the `repo-setup` chore story creates the remote repository or protects its
  branches. A greenfield project therefore stays in `local` mode, and nothing tells the person that adding a
  remote later means editing `delivery.mode` themselves.
- What breaks: a capability of the old system disappears with no stated reason, and a project that gains a
  remote keeps delivering locally.
- Resolution: either state the drop in row 93 (a person creates the remote), with `vsdd doctor` reporting
  "remote found, `delivery.mode` is `local`" and the line to change; or have `setup` offer, with confirmation,
  to create the remote and set branch protection.

### P3COV-14. The sample project's commands may fetch packages, which the end-to-end tiers forbid

- Severity: minor. On the slice path: yes (the sample, unit 4 of layer 1; the red and green gates run its
  commands).
- Locations: `packaging.md` 2.6 Layer 4 ("Its test command is `uv run python tools/junit_runner.py`"; "the
  end-to-end tiers must not need a package download"); E4 ("one known lint failure"); `STATE.md` measured facts
  (only `uv run --script` was measured).
- Problem: `uv run` in a project directory is not `--script`. It creates `.venv` in each story worktree and, if
  `pyproject.toml` declares a build system, builds and installs the project, which downloads the build backend.
  A PyPI fetch failed in the sandbox. The sample's lint, format and build commands are not specified, and the
  usual Python linters are downloads too.
- What breaks: the slice's red and green gates can fail on a network error, or on `.venv` landing in no path
  class.
- Resolution: specify that the sample has no `[build-system]` (or sets `[tool.uv] package = false`), that its
  commands are `uv run --no-project --offline python tools/...`, that it ships a small standard-library
  `tools/lint.py` and format check like its JUnit runner, and that `.venv/` and `build/` are in its
  `.gitignore`. Add a smoke assertion that no network fetch happens.

## Checked and found sound

1. No workflow script reads a file, runs a command, imports, or uses the clock or random numbers. All reading
   goes through clerk steps, and `meta` is a literal (`pipeline.md` 2.11.1).
2. Nesting is one level. Only top-level workflows call `story`, `story` calls nothing, and `runSpike()` is a
   step, not a workflow (blueprint 4.5, 6).
3. No workflow step relies on the Agent tool. The shared block's test is "do I have the Agent tool", and
   `needs_spike` is served by the wrapper (`agents-skills.md` 2.7, 2.11).
4. A step is held by blocking its `StructuredOutput` call, never by a refused stop. Only dispatched agents are
   held at `SubagentStop` (`hooks.md` 2.11.6, 2.11.7).
5. Every spike step passes `isolation: 'worktree'`, so the agent file's setting is not relied on in a step
   (`runSpike()`).
6. Hook timeouts are in seconds, and lint caps them at 60 (`hooks.md` 2.3; `packaging.md` L-HOOK).
7. The ten-minute cap on a foreground call is met: jobs re-attach, `--wait 540` runs with a Bash timeout of
   600000, and nothing carries a job id (blueprint 4.9).
8. Grounding blocks once and then stores findings as advisory, so a bad citation cannot use up the retries.
9. The decision command is recorded on `UserPromptExpansion` before any model reads it (item 2).
10. The session briefing arrives on the startup, resume and compact sources (proven).
11. The pre-compaction flush has a fallback for item 16.
12. The self-guarding hook command is inert in projects that are not initialised and blocks in factory projects.
    A missing `uv` exits 2.
13. `bin/vsdd` is on `PATH` in every context (item 5), and the shim covers the person's terminal.
14. Every step sets its effort (item 26), so effort inheritance is not relied on.
15. `omitClaudeMd` is set on every agent (item 11).
16. Every agent has an explicit tool list (item 17).
17. Ancestor searches rely on `Grep` skipping the excluded state directory (item 14), with a stated fallback.
18. The `encodeURIComponent` (item 34) and argument-text (item 31) assumptions have workable fallbacks in the
    prelude.
19. The old agents table (35 rows) is complete. Sampled: `business-analyst`, `data-engineer`, `dtu-validator`,
    `pr-manager`, `stub-architect`.
20. The old hooks (79 entries) are mapped. Sampled: destructive, secrets, red-gate, `protect-bc` and
    `protect-vp`, brownfield discipline, purity, harness version, worktree hooks.
21. The old `bin/` tools (21) are mapped, including the event-query tools that move to `vsdd events`.
22. The observability stack, its seven dashboards, the registry and the telemetry wiring are ported.
23. The fourteen documented paths, the hotfix flow and the deprecation path are mapped.
24. Maintenance sweeps 1 to 5 and 7 to 11 are mapped, with sweep 6 covered under P3COV-04. So is the limit of
    ten fixes per run.
25. The 37 approval steps are mapped, apart from the stale row in P3COV-11.
26. The spike entrances match the probe table in `STATE.md`, including "ask in a workflow, dispatch elsewhere".
27. The holdout thresholds, the 80 percent rotation and the per-wave scenario assignment are carried over.
28. The demo evidence policy follows decision D22.
29. The headless end-to-end harness uses only proven invocation (`claude -p --plugin-dir`, `Workflow` allowed).
30. No design relies on platform resume. Resume is the survey's job.
