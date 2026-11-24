# Pass 2: verification of the coverage findings (COV-01 to COV-21)

Reviewer task: check that the first-pass resolution of each finding in `review-coverage.md`, as stated in
section 8 of `rewrite/BLUEPRINT.md`, was carried out in the blueprint and in every design document where the
problem appeared. Decisions D1 to D30 are treated as adopted. Inputs read: `rewrite/STATE.md`, the blueprint in
full, `review-coverage.md`, the amendment lists of all five designs, and every body section each original
finding cited (plus the old `maintenance-sweep` skill and the platform's workflow-authoring reference, to check
two claims).

## Verdict

Nine of the 21 fixes are verified in every place the problem appeared: COV-01, 02, 05, 07, 08, 14, 15, 20 and 21.
Both blockers hold. The `decide` skill exists in the roster, in `roster.toml`, in the hook rule and in the
pipeline. The measuring and delivery commands have one owner (`state.md` 2.16), a command table, `gates.toml`
entries and build packages G1 to G7. Twelve are not sound. Two regressed:
- COV-11: the inherited spike walls rest on an asker type that the spike runner types itself, and the
  deduplication cache can hand a finding across a wall.
- COV-16: the self-guarding hook command runs before the inert check, so a broken install blocks unrelated
  projects.

Two are partial at major severity:
- COV-04: D4 is adopted, but its switch has no home that setup can write, and the packaging design describes
  the fallback mechanism instead.
- COV-06: many stage rows name no method, and three maintenance sweeps have no method at all.

The other eight are minor partials. Most are text that still says the old thing: the script skeletons, three
rows of `state.md`, need rows in `hooks.md` and `pipeline.md`, and person-facing messages. Two are real gaps:
the agent reserve protects only `wave` (COV-18), and intake output never satisfies the done test (COV-19).
None needs the architecture to change.

On the brownfield question in the user's message: the two coverage items that bear on it are COV-08, which is
verified (reference ingests run `disposition` and `port-analysis`), and COV-19, which is partial (existing
documents can be converted, but the later `spec` stages still re-run). The routes themselves are in place:
`ingest` then `feature` for an existing codebase, and `feature` alone after a release (`pipeline.md` 2.4 and
2.9.4). They are walked end to end by the requirements reviewer.

## Findings

### P2COV-01: The asker's walls for a spike rest on a value the spike runner types (COV-11, regressed)

- Severity: major
- Locations: `BLUEPRINT.md` section 6 (step 2 "Open", and the "Walls" row of the bounds table); `pipeline.md`
  2.8 (paragraph "Every spike runs in its own worktree ... The request carries the asker's agent type, the
  ledger entry records it") and 2.11.1 `runSpike()` (`--asker ${asker}`, where `step()` passes
  `${RUN.wf}/${name}/${subject}`); `state.md` 2.5 (spike ledger entry, field `asker_type`) and 2.16
  (`vsdd spike open`); `hooks.md` 2.11.6 (the `read-walls` bullet of "Containment"); `agents-skills.md` 2.9
  (wall row `spike-runner`) and 2.11 (last paragraph: "dispatched directly, they return blocked with the
  question, and the main session may run the spike on their behalf").
- Problem: The fix lets `code-reviewer` and `holdout-evaluator` (and `scenario-reviewer`) ask for spikes by
  giving the spike runner the union of its own and the asker's read walls. The union depends on `asker_type`
  in the ledger entry. That value reaches the entry only as an argument the spike runner types from its prompt.
  No design checks it against anything the hook observed, and no design makes `spike open` refuse without it.
  The skeleton passes a step path, not an agent type. Three paths cross the wall:
  1. The value is wrong or left out. The runner then has only its own walls (`holdout`, `reviews`), so it can
     read source, tests and specifications.
  2. The "main session runs the spike on their behalf" path records the main session, or nothing, as the
     asker. The main session then passes the finding into the reviewer's next prompt.
  3. `spike open` returns an earlier finding for the same question in the same unit, matched by question hash
     only (blueprint section 6, step 2). At the wave perimeter the adversary (which reads source and
     specifications) and the holdout evaluator or scenario reviewer can ask the same question, and the second
     asker receives a finding produced under weaker walls.
- What breaks: The information walls that make holdout evaluation and the code reviewer independent. `STATE.md`
  requires walls "enforced by hooks keyed on the calling agent, not by instructions". The leak is silent.
- Resolution:
  - `vsdd spike open` derives `asker_type` from a record the hook runner wrote. For a workflow step, that is
    the stored `needs_spike` step record of the asking step in the same run, matched by question hash.
    `step-results` already verifies the calling agent type before it stores that record. For an Agent-tool
    dispatch, it is the dispatcher's `agent_type`, which the hook records at `PreToolUse` on `Agent` together
    with a hash of the question.
  - `--asker` becomes a cross-check only. `spike open` refuses when no asker can be established.
  - Remove the "on their behalf" relay for the three walled roles. Outside a workflow they report the open
    question in their result and do not receive the finding.
  - Store each entry's effective read-deny set. Return a cached finding only to an asker whose deny set the
    earlier runner's set covers; otherwise run a new spike.
  - Fix the skeleton to pass the step's agent type.

### P2COV-02: D4 is adopted, but its switch has no home and two designs describe different mechanisms (COV-04, partial)

- Severity: major
- Locations: `BLUEPRINT.md` D4, 4.5 (`setup`), 4.7 (class `config`: "No tool call can write it"), section 10
  item 7; `hooks.md` 2.6 (the `[hooks]` table has no allow-list key), 2.10 rule list row 17 ("Project list off
  by default | new (question 2)"), 2.11.10 (`auto-allow`: "Its default is decision D4 of the blueprint");
  `packaging.md` amendment 12 ("`setup` offers to write, into the project's local settings, the permission
  allow rules of decision D4") and 2.6 layer 4, smoke probe 7 ("with the project allow-list on");
  `agents-skills.md` 2.12 (`setup`); `pipeline.md` 2.11.1 `runSpike()` (`--question-file -`,
  `--json-file -`).
- Problem: D4 as adopted is a hook mechanism. The plugin approves its own routine calls, scoped "by the agent
  whose role may write them", and "only the default of one hook rule changes". Nothing defines where the
  person's acceptance at setup is recorded:
  - The hook reads only `[hooks]`. That table has no key for the allow-list, and no tool call, `setup`
    included, may write `factory.toml`.
  - `packaging.md` instead has `setup` write static permission rules into `.claude/settings.local.json`. That
    is the blueprint's section 10 fallback, and static rules cannot be scoped by agent role.
  - The hook design still describes the default as pending.
  - Separately, the only spike recipe passes its inputs on standard input. The built-in part of `auto-allow`
    refuses any call with a redirect or pipe, so every spike's `spike open` and `spike record` raise a prompt.
- What breaks: Packages H6 (`auto-allow`) and K1 (`setup`) would be built to incompatible mechanisms. If the
  hook list stays off, a default-mode wave raises dozens to hundreds of prompts, and scheduled `claude -p` runs
  have their tool calls denied. Those are the outcomes D4 was adopted to prevent (user decision 7).
- Resolution:
  - Make the hook permit the one mechanism. Static settings rules stay as the fallback, used only if probe 7
    fails.
  - Give it a switch, `[hooks.rules.auto-allow] project = true`. `vsdd init` writes it from the answer `setup`
    collects before init, since init is the one writer of `factory.toml`. For a project that is already
    initialised, `setup` shows the person the one line to add.
  - Update `hooks.md` 2.10 row 17 and 2.11.10, `packaging.md` amendment 12 and smoke probe 7 to match.
  - Replace `--question-file -` and `--json-file -` with a file the runner writes under the temporary
    directory, as `hooks.md` 2.11.10 already requires.

### P2COV-03: Many workflow steps still name no method, and three maintenance sweeps have none to name (COV-06, partial)

- Severity: major
- Locations: `pipeline.md` 2.9.3 (`contracts`, `architecture`, `feasibility`, `assessments`, `scenarios`,
  `ux`), 2.9.4 (`delta`, `story-draft`, `evolve`), 2.9.5 (`outline`, `write`), 2.9.2 (`integration-review`,
  `holdout`), 2.9.6 (`adversarial`, `security`, `scenarios`), 2.9.8 (`map`, `analyze`, and the architect,
  first product-owner and security-reviewer parts of `synthesize`), 2.9.10 (`docs drift`,
  `pattern consistency`, `scenario freshness`); pipeline amendment 17 (`stages.toml` carries `method` per
  stage); `agents-skills.md` 2.8 (method catalogue) and 3.2 row 53 (`maintenance-sweep` → `maintain`, no
  method); `packaging.md` 2.6 layer 2, universal test 6.
- Problem: The fix makes `method` required on every step. It is enforced by universal test 6: the method must
  exist and be owned by the step's agent. Its source is the stage tables of 2.9, which become `stages.toml`.
  - Most rows still name no method. Some have an obvious one. Others are ambiguous: `map` and `analyze` could
    be any of `ingest-broad`, `ingest-deepen` or `ingest-synthesis`. The architect's "verification gap
    analysis" in `ingest` has no matching method. `assessments` needs two methods in one step.
  - Three maintenance sweeps have no method in the catalogue at all: `docs drift` (technical-writer),
    `pattern consistency` (code-reviewer, whose only method `review-diff` needs a diff) and `scenario
    freshness` (holdout-evaluator). The old `maintenance-sweep` skill held those procedures, and its mapping
    row points at no method.
  - The old freshness sweep also checks which features have no hidden scenario. That needs the contracts,
    which the holdout evaluator's wall forbids it to read.
- What breaks: To satisfy test 6, a builder must attach a wrong but existing method, so those agents run
  without the ported procedure. That is the failure COV-06 described, now limited to these steps. Three of
  the eleven sweeps the user named under "port all of it" lose their content.
- Resolution:
  - Add a method column to every stage table in 2.9.
  - Add three methods: `docs-drift` (technical-writer), `pattern-sweep` (code-reviewer) and
    `scenario-freshness` (holdout-evaluator, re-running scenarios only). Move the coverage half of the
    freshness sweep to code (`vsdd coverage`, run by a clerk) or to `scenario-reviewer` with `review-scenarios`.
  - Point mapping row 53 at the three methods.

### P2COV-04: Only `wave` keeps an agent reserve; every other parent of `story` can still exhaust the shared counter (COV-18, partial)

- Severity: minor
- Locations: `pipeline.md` amendment 7 ("Before each fix round the wave checks ..."), 2.5 (limits table), 2.9.6
  (`verify` fix rounds through `vsdd story new-fix --verify`), 2.9.10 (`maintain`, up to ten fix stories), 2.9.11
  (`multi` integrate fixes), 2.11.3 `wave.js` (`runStories` passes `limits: RUN.limits`, so children get
  `agents: 900`; `STORY_WORST = 30` against `story_agents = 40`; `fix()` has no reserve check; the `catch`
  returns a result with no `agents`); `BLUEPRINT.md` 5.1 (last bullet).
- Problem: The reserve rule is stated for the wave only. `verify` runs up to three rounds of fix stories through
  `story` children, each child with a budget of 40, with no check before a round. The same holds for `maintain`
  and `multi`. A parent learns a child's count only from its result, so a child that throws or returns `null` is
  counted as zero although it may have used its whole budget. The platform counter is shared and hard at 1,000
  (confirmed in the workflow-authoring reference). Beyond that point, `agent()` throws, including in the
  checkpoint relay.
- What breaks: The guarantee COV-18 protected. In a large `verify`, the run can end without its checkpoint,
  with the lock held until it goes stale and the state unpushed.
- Resolution:
  - Move `runStories` into the stamped prelude with the reserve check
    (`RUN.agents + units * story_agents + CLOSE_RESERVE <= limit`, else return `partial`), so `wave`,
    `verify`, `feature`, `maintain` and `multi` share it.
  - Charge a thrown or empty child its full `story_agents`.
  - Pass each child `limits.agents = story_agents`, derive the worst case from it, and update the skeleton.

### P2COV-05: The story skeleton still waits for CI in a loop, runs gates in the foreground, and names a step option the platform does not have (COV-03, partial)

- Severity: minor
- Locations: `pipeline.md` amendment 4 ("Steps that wait set `stallMs` above the wait"), 2.11.1 `relay()`
  ("Run exactly this command once"), 2.11.2 `story.js` (`greenCmd`, the red and demo gate commands without
  `--start`, and the deliver loop `if (d.state === 'ci_pending' && waits < 6)`); `state.md` 2.16 (a job
  command runs in the foreground unless given `--start`); `agents-skills.md` 2.7 (shared block) and 2.8 (no
  mention of `--start`, `job wait` or the 600000 timeout anywhere in the document); `BLUEPRINT.md` section 10.
- Problem:
  - The red and green gates are jobs, and the test-writer and the implementer run them themselves. The only
    concrete prompt text tells them to run `vsdd gate red|green --story <id> --json`, in the foreground, under
    the Bash default of two minutes. The skeleton disclaimer names "jobs in place of the `ci_pending` loop" but
    says nothing about builder gates, and the agent design never tells builders about jobs.
  - `stallMs` is not an `agent()` option. The workflow-authoring reference lists `label`, `phase`, `schema`,
    `model`, `effort`, `isolation` and `agentType`, and section 10 does not probe it.
- What breaks: A builder that follows the prompt verbatim is killed at two minutes on any real project. The
  step retries, then the unit becomes `stuck`, which is the original COV-03 failure on the builders' own gates.
- Resolution:
  - Make job commands safe by default. Without `--start`, a job command starts its job and waits at most
    100 seconds, under the default limit, then returns `pending` with the job id and the exact `job wait`
    command to run next.
  - Put the wait loop and `timeout: 600000` into the closing-command text that `brief()` generates, and into
    the clerk's body.
  - Update the `story.js` deliver loop and gate commands.
  - Drop `stallMs`, or add it to section 10 as a probe.

### P2COV-06: Converted documents never satisfy the done test, and four passages still describe intake as plain resume (COV-19, partial)

- Severity: minor
- Locations: `pipeline.md` 2.9.3 (row `intake`: "the later stages they cover then read as `done`"; paragraph
  "Documents that already sit in the 1.0 layout ... the survey reports those stages as `done`"), 2.5 (the
  four-part done test), 2.3 (`spec` arguments list `guidance`, `variant`, `only` and no `sources`), 2.4 row
  "Planning (adaptive front end)" ("intake of existing documents is ordinary resume in `spec`"), 3.1 row
  `planning.lobster` ("validation of existing documents → resume in `spec`"); `state.md` 2.8 (the refined done
  test); `agents-skills.md` 3.2 row 5 (`artifact-detection` → `status` only).
- Problem: A stage is done only when its closure entry says `done` with the same input digest, or when a live
  run holds a done step record for that stage. After an intake run, the `domain`, `contracts` and
  `architecture` stages have neither, because only the `intake` step ran. Documents placed in the layout by
  hand have neither either. Nothing writes those closures.
- What breaks: `spec` runs the domain, contracts and architecture stages anyway, and the planners regenerate
  the artifacts the intake converted. That is the outcome COV-19 named ("ignored and regenerated").
- Resolution:
  - The `intake` step returns `covers`, the stage names whose output globs its artifacts fill. The checkpoint
    writes `done` closure entries for those stages with their input digests.
  - Either add the same rule for valid hand-placed documents, or delete that claim and route such documents
    through `sources`.
  - Add `sources` to the `spec` row of 2.3, and correct the 2.4 row, the 3.1 row and agents-skills row 5.

### P2COV-07: The self-guarding hook command blocks projects that are not factory projects (COV-16, regressed)

- Severity: minor
- Locations: `hooks.md` 2.3 (the generated command `sh -c 'f=...; [ -r "$f" ] || { ...; exit 2; }; exec sh "$f"
  hook gate'`, and the note "the only defence if the plugin's files are missing"), 2.2 and 2.8 (the inert check
  lives in `find-root.sh`, sourced by `launch.sh`), 2.9 (row "the hook cannot start at all → Blocked by
  `onFailure`"), 2.16 (row "The launcher already blocks without `onFailure`"); `STATE.md` ("Hooks do nothing in
  a project that has not been initialised"); `BLUEPRINT.md` D24; `packaging.md` 2.13 N8 (cache cleanup).
- Problem: The guard runs before the inert check. When `launch.sh` is unreadable, every guarded tool call
  (`Bash`, `Read`, `Edit`, `Write`, `Grep`, `Glob`, `Agent`, and the rest) is blocked in every project where the
  plugin is enabled, whether or not it has `.factory/factory.toml`. The case the guard exists for, a cache
  directory removed under a running session, is exactly the case that reaches unrelated projects. The guard
  for `brief` mode is unspecified, and the three notes still describe `onFailure` as the defence for missing
  files.
- What breaks: D24 and the `STATE.md` requirement that hooks stay silent in projects that are not initialised.
  A broken install stops all work in unrelated projects until a reinstall.
- Resolution:
  - Inline the inert test in the guard. When the launcher is unreadable, walk up from
    `${CLAUDE_PROJECT_DIR:-$PWD}` for `.factory/factory.toml` (a few lines of `sh`, mirroring `find-root.sh`).
    Block only if one is found; otherwise exit 0.
  - In `brief` mode, print a down-briefing with `systemMessage`.
  - Correct the three notes.
  - Add a smoke case: a missing launcher in a project that is not initialised stays inert.

### P2COV-08: Two designs define the spike lookup differently, and how `spike open` learns the agent id is unspecified (COV-17, partial)

- Severity: minor
- Locations: `hooks.md` 5 S7 (`spike_entry(project, worktree)`), 5 P5 ("writes the ledger entry with the
  runner's worktree path ... so the hook can match"), 2.11.6 (row "No work without an open ledger entry": "the
  rule records [the agent id] ... and `spike open` writes into the entry"; row "No stop without a finding":
  "the ledger entry for the runner's worktree"), amendment 8 (one marker per session id); `state.md` 5 S7
  (`spike_entry(project, agent_id, worktree)`); `pipeline.md` 2.10.4 (`kind` description: "experiment runs code
  in an isolated worktree; research reads documentation and sources") and 2.11.1 `runSpike()` (isolation only
  when `kind` is not `research`).
- Problem:
  - The hook design's need rows still key spikes by worktree, and its S7 signature differs from the one the
    state design implements.
  - `spike open` is a child process of the Bash tool and never sees the hook payload that carries `agent_id`.
    The only hook-to-tool channel the designs define is the per-session marker, which every parallel agent in
    the session shares. If the agent id travels that way, two spike runners that open at once can be swapped,
    which is the misattribution COV-17 described.
  - The schema description and the skeleton still treat research spikes as unisolated.
- What breaks: Time boxes, open-entry checks and the stop check can attach to the wrong ledger entry, and
  packages H8 and S6 would be built to different signatures.
- Resolution:
  - `hooks.md` S7 and P5 adopt the state design's signature.
  - Specify the channel. At a spike runner's first guarded call, the hook writes
    `local/hooks/spikes/<agent_id>.json` with its `cwd`. `spike open` takes the one record whose `cwd` equals
    its own working directory (unique, because every spike is isolated) and refuses on none or several.
  - Correct 2.10.4 and the skeleton.

### P2COV-09: The terminal shim can run the wrong plugin version, and two messages still name commands that do not work (COV-13, partial)

- Severity: minor
- Locations: `packaging.md` 2.10 ("the recorded path, or the newest `vsdd-factory` directory in the plugin
  cache"); `state.md` 2.11 ("the command a person runs ...: `vsdd lock release`") and 2.10 resume table ("the
  briefing names `vsdd lock release`"); `hooks.md` 2.9 (row "uv cannot run ...: Exit 2, naming the exit code and
  `vsdd doctor`"); `BLUEPRINT.md` 4.1 (row "Person's terminal").
- Problem:
  - The recorded path is a versioned cache path that changes with every update. "Newest directory" is not
    the version enabled for the project the person is in. Projects can pin different tags, and during cutover
    the cache holds rc.25 directories, which have no `bin/vsdd`. Person-only commands (`decide`, `import`,
    `init --fresh`, `config set`, `lock release --force`) could run another version's code against this
    project's state.
  - State 2.11 gives a bare `vsdd` form, which fails without the shim. The blueprint's rule is that every
    message names a form that works.
  - The "uv cannot run" message names `vsdd doctor`, which runs through the same launcher and `uv`, so it
    cannot run in that state. `hooks.md` 2.2's own rule forbids that.
- What breaks: The human escape hatches COV-13 was about, at the moments they are needed.
- Resolution:
  - The shim resolves the plugin from Claude Code's record of installed plugins
    (`~/.claude/plugins/installed_plugins.json`, as COV-13 proposed): the project-scope entry for the current
    directory, then the user scope. It checks that `bin/vsdd` exists and otherwise prints the reinstall
    command.
  - Messages give the in-session form first (`/vsdd-factory:status --fix`, which releases a dead holder's lock)
    and the shim form second.
  - The "uv cannot run" row names terminal commands that need nothing from the plugin.

### P2COV-10: `state.md` still carries three statements the fixes removed (COV-12, COV-09, and a COV-02 leftover; partial)

- Severity: minor
- Locations: `state.md` 2.13 (row "Something copied into the product ... | 27 | Scaffold files owned by the
  skill that copies them, under that skill's folder"); 2.18 (row "packaging": "the research cache is
  `.factory/local/cache/research/`"); 5, need P-5 ("the survey or `vsdd worktree ensure`").
- Problem:
  - The design-system starter, demo templates and scaffold files now live in `<plugin>/assets/` (blueprint
    4.1, `agents-skills.md` P-7, `packaging.md` 2.3 and L-TEMPLATE). No skill copies them; the `ux-designer`
    and `demo-recorder` methods use them.
  - The research cache and `vsdd cache` are dropped (state amendment 6, blueprint 4.7 and 4.9).
  - `vsdd worktree` does not exist (blueprint 4.9).
- What breaks: Package S1 (skeleton templates) and the A2 and A5 builders read contradictory homes for the 27
  scaffold files. A builder reading the seam table may create a research cache path that nothing uses.
- Resolution: Change the 2.13 row to `<plugin>/assets/{design-system,demo,scaffold}/`, cited by the methods and
  checked by L-TEMPLATE. Delete the cache clause in 2.18. Change P-5 to "made by `vsdd survey`".

### P2COV-11: The pipeline's own list of configuration keys omits the sections the fixes added (COV-10 and COV-09, partial)

- Severity: minor
- Locations: `pipeline.md` 5.1 S1 (the key list has no `release.*`, no `discover.source`, no `holdout.batch`,
  no `delivery.dependency_changes`); `state.md` 2.12 (the hosted-sections table lists `[release]` and
  `[[discover.source]]` with owner "pipeline", keys "As shown above"); `BLUEPRINT.md` 4.12.
- Problem: COV-10 cited S1 for having no release keys, and it still has none. The section's owner and its host
  now disagree. The state design rejects unknown keys as errors.
- What breaks: If the configuration schema is built from the owner's list, `[release]` and
  `[[discover.source]]` are rejected as unknown keys. Release publishing and feedback collection then cannot
  be configured.
- Resolution: Make S1 point to blueprint 4.12 and `state.md` 2.12 as the key list, or add the four keys with
  their defaults.

## Checked and found sound

- COV-01 (blocker): `decide` is in the skill roster, `roster.toml`, P-9, `hooks.md` 2.11.5 and 2.3 (matcher
  `^vsdd-factory:decide$`), `pipeline.md` 2.6 and A6. The prompt-expansion probe is item 2 of section 10. The
  fallback and its weakness are stated in `hooks.md` 2.11.5 and 2.16. One need row (`hooks.md` 5 P4) still shows
  the pre-SEAM-37 grammar `[note]`; that is SEAM-37's concern, not this finding's.
- COV-02 (blocker): `state.md` 2.16 lists every measuring and delivery command with arguments, outputs, writers
  and callers. Blueprint 4.9, `gates.toml` `[governed]` and `[callers]`, packaging amendment 5 (P5 answered:
  merge and worktree are library functions), pipeline P1, and packages G1 to G7 with the conformance fixtures in
  G3 all agree.
- COV-05: The `drive` skill, `vsdd driver on/off`, `local/driver.json`, the briefing line in `state.md` 2.9,
  probe 4, and the nesting rationale appear consistently in four designs.
- COV-07: The pipeline body uses only roster names; `ROLE` is generated; the clerk has a roster row, a tier, a
  wall row with `tool_only` and a lint exception. The "no relays" design rule in `agents-skills.md` 2.2 reads
  oddly beside the clerk but changes nothing that is built.
- COV-08: `disposition` and `port-analysis` stages for `as: "reference"` (`pipeline.md` 2.9.8), their methods
  (`agents-skills.md` 2.8), their output paths in the state layout, and their readers (`spec` assessments,
  `feature` with intent `port`).
- COV-14: Section 3.2 is re-keyed to the twelve workflow names. Rows 54 and 115 are corrected. The disposition
  totals (6, 33, 64, 14, 10) sum to 127, which a count of the table confirms.
- COV-15: `hooks.md` 3.1 rows 16 and 20 are re-marked as the schema check and the template unit test.
- COV-20: There is no spike skill in any roster. P-8 forbids a skill with a workflow's name, and the fallback
  (D8) is stated.
- COV-21: The clerk lists `Bash, StructuredOutput` in `agents-skills.md` 2.5, `pipeline.md` A3 and blueprint 4.3.
  Both probes are item 8 of section 10, which package B8 runs before A1.
- Platform facts re-checked against the workflow-authoring reference: children share the parent's agent
  counter and concurrency cap; the cap is 1,000; the `agent()` options are `label`, `phase`, `schema`, `model`,
  `effort`, `isolation` and `agentType`.
- Counts: 21 findings checked; 9 verified; 2 regressed (COV-11, COV-16); 10 partial (COV-03, 04, 06, 09, 10,
  12, 13, 17, 18, 19); none left unfixed.
