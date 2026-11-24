# Review: coverage and platform truth

Reviewer lens: (a) does every capability of the old system port, or is it dropped with a reason; (b) does each
design rely only on platform behaviour that `rewrite/STATE.md` lists as verified, or flag what it does not.
Inputs read in full: `rewrite/STATE.md`, the five designs, the four old-system inventories and the platform-facts
survey. The workflow script API was also checked against the platform's own workflow-authoring reference
(`budget`, `whenToUse`, `phases[].model`, `agentType`, `isolation`, shared child counters). Old files were sampled
where a mapping row made a claim that could be checked (the maintenance sweeps, the discovery schedule, the
feedback and analytics skills, the release skill, the dashboards' event types).

## Verdict

The mapping tables are unusually complete: every old workflow file, mode, phase, approval step, agent, skill,
registry entry, `bin/` tool and template has a row, and the totals add up. The design breaks down in a smaller
number of places where a capability is named in one design and then never built by another. The two blockers are
in this class. No design ships the `/vsdd-factory:decide` command that every human stop depends on. No design and
no build package owns the measuring commands (`gate <name>`, `story deliver`, `story new-fix`, `review verdict`,
`holdout env`, `release publish`), which carry most of the old phases 3 to 7. On platform truth, the designs
respect the hard limits they were told about: one level of nesting, no mid-run input, no reliance on resume
from a new session, plugin agents without hooks or MCP or permission mode, and timeouts in seconds. They miss
limits that `STATE.md` does not list. A foreground Bash call stops at ten minutes, which breaks long gates and the
twenty-minute CI wait. Permission prompts pause an unattended run, and no design sets the permission posture. The
child workflows share the parent's agent counter. Several capabilities the user named under "port all of it"
reach 1.0 only on paper: the low-touch driver, disposition and semantic port, feedback and analytics ingestion,
scheduled sweeps, config-driven release, diagrams and PDF export. Each has a short, concrete fix. None needs the
architecture to change.

## Findings

### COV-01: No design ships the `decide` command that settles every human stop

- Severity: blocker
- Documents and sections: `pipeline.md` 2.6 (how a decision is recorded), 4 I12, 5.3 H5; `hooks.md` 2.11.5,
  2.16, 5 A4 and P4; `agents-skills.md` 2.12 (the twelve-skill roster and `roster.toml`), 4 P-9; `packaging.md`
  2.6 L-ROSTER, 2.10; `state.md` 2.7, 2.16.
- Problem: The pipeline reserves `/vsdd-factory:decide` and has the hook runner record the decision from the
  typed prompt. The hooks design goes further: it records the decision only from `UserPromptExpansion`, which
  "requires `decide` to be a registered plugin command" with `disable-model-invocation: true` (need A4). The
  agents-skills roster has twelve skills, and `decide` is not one of them. The packaging lint (L-ROSTER) fails the
  build if a skill on disk is missing from the roster. The stated fallback is "a person can run `vsdd decide` in
  their own terminal". But `bin/vsdd` is only on the Bash tool's `PATH` (packaging 2.10), not on the person's shell
  `PATH` (see COV-13). There is also a second, unverified dependency: `hooks.md` 2.16 marks it as not verified
  that `UserPromptExpansion` fires for a typed plugin command, in interactive and in `-p` sessions.
- What breaks: The brief, spec and release stops, plus every queued `finding`, `spec-gap`, `stuck` and
  `environment` decision, cannot be settled inside a session. The pipeline stops at the first approval.
- Proposed resolution: Add `decide` to the agents-skills roster and `roster.toml` as a person-only skill
  (`disable-model-invocation: true`, `argument-hint: "<id> <choice> [note]"`). Its body tells the model to relay
  the context the hook returned and to name the next command. Put a probe in B8 (packaging 2.12) before the hook
  rule is built: does `UserPromptExpansion` fire for `/vsdd-factory:decide`, does it carry `command_args`, and
  does a block keep the text from the model? Keep the `UserPromptSubmit` fallback, and state its weakness. Make
  the terminal route real (COV-13).

### COV-02: The measuring and delivery commands have no owner and no build package

- Severity: blocker
- Documents and sections: `pipeline.md` 2.2, 2.9.1 to 2.9.7 (gate pass conditions, `story deliver`), 4 I8,
  5.4 P1, 7 R7; `packaging.md` 2.10 (the `vsdd` command table), 4 I4, 2.12 (build plan); `state.md` 2.16 (state
  commands); `hooks.md` 2.7 `gates.toml`.
- Problem: The pipeline needs sixteen tool commands, and its risk R7 calls the tool "the critical component".
  `state.md` 2.16 takes `survey`, `checkpoint`, `decide`, `cycle`, `plan schedule`, `spike`, `idea register` and
  `report debt`. Nobody takes `gate <name>` (red, green, demo, spec, plan, baseline, wave, ui, release, formal,
  ingest, audit, contracts, integration), `story deliver`, `story new-fix`, `review verdict`, `holdout env` or
  `release publish`. The packaging command table lists none of them. Its build plan has no package for them
  (T1 to T6 are doctor, events, observability, pr-merge and worktree, status and lock, repos and cache). Packaging
  does build `vsdd pr-merge` and `vsdd worktree`, which no workflow calls (its need P5 is unanswered).
- What breaks: The red gate, the green gate, frozen-test hashing, the wave integration gate (suite, mutation,
  clone fidelity), hidden-scenario rotation, formal proofs, CI polling and the governed merge, the release
  publish, the convergence verdict that guards merge, and fix-story creation. These are the old phases 3 to 7 and
  `code-delivery`. As planned they are never built, so no workflow can pass its first gate.
- Proposed resolution: Give one design ownership of a "measure and deliver" package. Packaging is the natural
  owner, with behaviour as specified in `pipeline.md` 2.9 and stores from `state.md` 2.8. Add the commands to
  packaging 2.10 and I4. Add build packages T7 (gates and evidence), T8 (`story deliver`, `story new-fix`,
  `review verdict`, `holdout env`) and T9 (`release publish`). Decide that `story deliver` calls `worktree ensure`
  and `pr-merge` internally, which answers packaging P5. Put the governed names in `hooks.md` `gates.toml`. The
  conformance fixtures of pipeline P4 (`review verdict` against `converge()`) belong in T8.

### COV-03: Long gates and the CI wait exceed the platform's foreground command limit

- Severity: major
- Documents and sections: `pipeline.md` 2.2 point 1 (an agent finishes by running its gate), 2.9.1 (`story
  deliver` polls CI), 2.9.2 (`factory gate wave` runs the suite, mutation testing and clone fidelity), 2.11.1
  `relay()` ("run exactly this command once"), 2.11.2 (`ci_pending` loop); `state.md` 2.12 (`ci_wait_minutes =
  20`), 2.11 (lock staleness "longer than the longest single tool call").
- Problem: Every gate and every delivery step runs as one Bash call by an agent. A foreground Bash call in Claude
  Code defaults to a two-minute timeout and is capped at ten minutes. None of the designs mentions this. The
  clerk is told to run the command once and not to interpret it, so it will not raise the timeout or move the
  command to the background unless told to. A green gate runs the configured build, the full suite, lint and
  format check. On a medium project, and on this repository's own Rust workspace, that takes longer than ten
  minutes. A mutation run takes hours. The configured CI wait is twenty minutes per call.
- What breaks: The command is killed mid-run. The clerk returns `ok: false`, the step retries and fails, and
  after two runs the unit becomes `stuck`. This happens exactly on the projects that most need the gates.
- Proposed resolution: Make every measuring command safe under the Bash limit. Long work runs detached:
  `vsdd gate <name> --start` returns a job id at once and writes the evidence record when it finishes, and
  `vsdd gate wait <job> --max 540` returns `pending` or the result within nine minutes. Scripts loop over
  `pending` with a bounded count, as `story.js` already does for `ci_pending`. Recast `ci_wait_minutes` as
  "per-call wait at most 9" times `ci_waits`. Tell every clerk and builder prompt to pass an explicit Bash
  timeout of 600000. Add a conformance probe (packaging B8) for the Bash timeout and for background commands
  inside a workflow step.

### COV-04: Permission prompts will pause unattended runs, and no design sets the permission posture

- Severity: major
- Documents and sections: `pipeline.md` 2.4 (the driver; "three human actions"), 7 R6; `hooks.md` 2.11.10
  (`auto-allow`: built-in part narrow, project allow-list "ships off"), 6 question 2; `agents-skills.md`
  N-HOOK-6; `packaging.md` 2.6 layer 4 (end-to-end runs pass `--allowedTools`, so they never meet a prompt);
  `state.md` 7 risk 10.
- Problem: The platform pauses a workflow run on a permission prompt, and that is the only thing besides usage
  limits that pauses it. Workflow steps run under the session's permission mode. In default mode, each builder's
  edits in a story worktree, `git commit`, configured build and test commands, and each clerk call with a
  redirect prompt the person. The redirect case covers `spike open --question-file -` and
  `spike record --json-file -`, and `auto-allow` permits only a bare `vsdd` call "with no pipe, redirect or
  second segment". A plugin cannot ship permission settings (`settings` is not a confirmed plugin component). The
  only lever in the designs is the hooks allow-list, and it is off by default.
- What breaks: User decision 7 (low touch). A wave of twenty-four stories in default mode raises hundreds of
  prompts, and an overnight run stalls at the first one. The end-to-end tests cannot see this, because they
  pre-allow every tool.
- Proposed resolution: Decide the posture in the blueprint. Recommended: `setup` writes project permission allow
  rules for edits under `.worktrees/**` and `.factory/**`, the configured `[commands]`, `vsdd *` and the git
  commit family, and explains auto mode as the alternative. The hook gates stay the safety layer either way. Ship
  the hooks allow-list on for story and spike worktrees. Replace stdin redirects in tool recipes with arguments
  or a file under the temporary class. Add a smoke probe in default permission mode that counts prompts.

### COV-05: The low-touch driver is in no roster, and its platform premise is unverified

- Severity: major
- Documents and sections: `pipeline.md` 2.4 (the driver), 5.2 A6, 7 R4, S9; `agents-skills.md` 2.2 rule 7,
  2.12 ("no skill exists to start a workflow"); `state.md` 2.3 layout (`local/driver.json`), 2.9 (briefing line
  "The driver is active: continue it"); `reference/claude-code-plugin-platform-facts.md` section 2 (a skill
  causing a Workflow call is UNCONFIRMED).
- Problem: The pipeline's answer to low touch is a twenty-line driver skill that reads `next`, starts the
  workflow and repeats. The agents-skills roster leaves it out by rule. The state design writes and briefs a
  "driver active" flag that no component sets. Whether a skill's instructions can make the main session call the
  Workflow tool was never probed. A driver cannot be a workflow either: it would call `wave`, which calls
  `story`, and that is two levels.
- What breaks: User decision 7. Without the driver a person types every `spec`, `plan`, `wave`, `verify` and
  `release` command. The briefing's "continue it" line refers to something that does not exist.
- Proposed resolution: Add a person-invoked `drive` skill, or `status --go`, to the roster. It runs
  `vsdd driver on`, loops on `next`, and stops on a human decision, on done, or on two runs with
  `progress: false`. Give `vsdd driver on|off` to the state commands so that `driver.json` has a writer. Add a
  B8 probe, interactive and `-p`, that a skill can start a plugin workflow. State in the blueprint that the driver
  must live in the main session because of the nesting limit.

### COV-06: Workflow steps never name a method file, so the ported procedures are never loaded

- Severity: major
- Documents and sections: `pipeline.md` 2.5 ("What every step is given"), 2.11.1 `brief()`, 2.11.2 and 2.11.3
  (step prompts), 2.9.1 (story kinds `facade`, `port`, `docs`, `data`, `fix`); `agents-skills.md` 2.7 (template:
  "Your task names a method"; "If a required input is missing, return blocked"), 2.8, 3.2 (66 skills mapped to
  "workflow" plus a method), 5 N-PIPE-3.
- Problem: In 1.0 almost all old skill content lives in method files: stubs, failing-tests, tdd, fix-findings
  (systematic debugging), service-clone (the old dtu-creation), port-translation, sdk-generation,
  data-migration, ui-implementation, review-story-diff, holdout-eval, and others. Agents read a method only when
  the task names it. The pipeline's step prompts never name one, and its "what every step is given" list has no
  method field.
- What breaks: Agents either return `blocked` for a missing input, as their own rule says, or improvise without
  the procedure. The coverage claims of `agents-skills.md` 3.2 for about seventy skills hold only if the method is
  named. Story kinds such as `facade` and `port` reach the implementer with no service-clone or port-translation
  procedure.
- Proposed resolution: Add a `METHOD` table to the prelude, keyed by agent type, stage and story kind (for
  example implementer with `facade` maps to `service-clone`, and adversary at the story perimeter maps to
  `review-story-diff`). `step()` puts "Method: <name>" first in every prompt. Since scripts get no path
  substitution, the survey returns `plugin_root` and the line gives the absolute path, which also covers probe V1
  if it fails. Add an L-XREF check that every step names an existing method, and a harness assertion.

### COV-07: Agent types the workflows dispatch are not in the agent roster

- Severity: major
- Documents and sections: `pipeline.md` 2.1 (tiers), 2.9 stage tables, 2.11.1 `ROLE`, 5.2 A1 and A3;
  `agents-skills.md` 2.4 (18 agents), 3.1, 5 N-PIPE-2; `hooks.md` 4.2 (noted for `clerk` and `spec-reviewer`).
- Problem: The pipeline dispatches `clerk`, `business-analyst`, `spec-reviewer`, `validate-extraction`,
  `research-agent`, `devops-engineer`, `dx-engineer`, `formal-verifier`, `e2e-tester`, `performance-engineer`,
  `dtu-validator`, `accessibility-auditor`, `visual-reviewer`, `session-reviewer` and `data-engineer`. None of
  them exists in the 18-agent roster. The platform resolves `agentType` from the agent registry, so an unknown
  type fails the call, and the packaging harness's universal test 3 rejects the script. This goes beyond names in
  two cases. The `clerk` (Sonnet, low effort, `tool_only`) contradicts the roster's rule "no relays; mechanical
  steps use `devops`". And the pipeline's lens design (a spec-reviewer lens beside the adversary, an
  accessibility and visual pair at UX review) assumes roles the roster merged into one agent with method lenses.
- What breaks: Every workflow as specified.
- Proposed resolution: The pipeline adopts the 18 names in `ROLE` and expresses lenses as method names (this
  depends on COV-06). Settle the clerk explicitly. Either add `clerk` to the roster with a `walls.toml` row that
  has `tool_only = true` (recommended, for cost and containment), or run clerk steps as `devops` with a
  `tool_only` flag that the wrapper selects.

### COV-08: The disposition pass and semantic port analysis have no pipeline stage

- Severity: major
- Documents and sections: `agents-skills.md` 2.8 (codebase-analyzer methods `disposition`, `port-analysis`), 3.2
  rows 33 and 100; `pipeline.md` 2.9.8 (`ingest`: with `as: "reference"` only map, analyze and validate run),
  2.9.4 (intent `port`), 3.7 (silent on `semport-analyze` and `disposition-pass`); old `semport-analyze` and
  `disposition-pass` skills.
- Problem: Two brownfield capabilities are mapped to `ingest` by the agents-skills design, but `ingest` has no
  stage that runs them. One is the per-reference-repository verdict (model, reimplement, enhance or leave)
  against the project's own intent. The other is the semantic port analysis (extract intent, design the
  translation strategy). A `feature` with intent `port` then starts from raw module analysis.
- What breaks: User decision 2 for brownfield and porting work, which drives the user's multi-repository
  projects. The architect's reference-implementation assessment has nothing to consume.
- Proposed resolution: Add two stages to `ingest` with `as: "reference"`, registered in `stages.toml`.
  `disposition` (codebase-analyzer, method `disposition`) runs against the approved brief or the `guidance`
  argument and writes `analysis/<ref>/disposition.md`. `port-analysis` (method `port-analysis`) writes
  `analysis/<ref>/port.md`. The `spec` assessments stage and the `feature` delta step for intent `port` read both.

### COV-09: Discovery inputs need a shell or credentials that no assigned agent has, and nothing schedules sweeps

- Severity: major
- Documents and sections: `pipeline.md` 2.9.9 (research step by research-agent), 2.9.10 (post-release signals),
  5.4 P6 (headless schedule recipe); `agents-skills.md` 2.5 (`researcher`: "No shell"), 3.2 rows 4, 25, 52 (jira
  becomes a `devops` method, but no discovery step runs `devops`); `packaging.md` 2.10 (`vsdd cache`, which needs
  a shell), no answer to P6; `state.md` 2.3 layout (no place for analytics exports or surveys); old
  `customer-feedback-ingestion` (`gh issue list`, `gh api graphql`, Slack API), `analytics-integration`
  (`.factory/analytics/*.csv`, authenticated API), `discovery.lobster` `schedule:` (daily feedback, weekly market,
  competitors and analytics), and `maintenance.lobster` ("run on a schedule").
- Problem: The agent assigned to feedback and analytics research cannot run `gh`, jira-cli or an authenticated
  API call, and the method that knows issue trackers belongs to an agent no discovery step uses. Nobody answers
  the pipeline's request for a scheduled headless run. The research cache command needs a shell the researcher
  lacks.
- What breaks: Feedback ingestion from GitHub, Jira and Slack, analytics ingestion, the post-release signals
  sweep, and every cadence ("maintenance: scheduled, 11 sweeps"). These are parts of market discovery and
  maintenance that the user named.
- Proposed resolution: Add a `collect` stage to `discover` and to the signals sweep. A clerk running
  `vsdd discover collect` or `devops` with method `issue-trackers` pulls each configured source into a new
  registry kind, `research/inputs/<source>/<date>.*`, which the researcher reads. Add `[discover.sources]` keys,
  owned by the pipeline and hosted in `factory.toml`. Packaging answers P6 with a documented recipe (cron or
  launchd running `claude -p` with `--allowedTools Workflow`, or Claude Code scheduled tasks if a probe shows
  workflows start there) and an end-to-end probe. Either give `vsdd cache` a clerk caller or drop it with a
  reason.

### COV-10: Config-driven release has no configuration home

- Severity: major
- Documents and sections: `pipeline.md` 2.9.7 ("Runs the release steps from project configuration"), 5.1 S1
  (no release keys); `state.md` 2.12 (`factory.toml` has no `[release]` section, and the table of hosted sections
  omits it), 2.13 (configuration templates); old `release` skill (reads `.factory/release-config.yaml`: version
  locations per manifest, publish command such as `cargo publish` or `npm publish`, bootstrap mode that detects
  them).
- Problem: The `release` workflow's prepare and publish stages run "release steps from project configuration",
  but no design defines those keys. The old capability was the configuration: which files carry the version, what
  publishes, and whether a GitHub Release is made.
- What breaks: `release` can tag, but it cannot bump version files or publish to a package registry. That loses
  the old release capability for every project that ships a package.
- Proposed resolution: Add `[release]` to `factory.toml`: `version_files` (path plus key), `changelog`,
  `tag_format`, `publish` (a list of commands), `github_release` (true or false). The pipeline owns it and the
  state design hosts it. `vsdd init` detects it the way the old bootstrap mode did, and `release publish` runs it.

### COV-11: Two agents cannot request a spike, against the "any agent" requirement, and the user was not asked

- Severity: major
- Documents and sections: `agents-skills.md` 2.7 (shared block, first bullet omitted for three agents), 2.11
  ("Two roles cannot ask for a spike"), 6 (not among the open questions); `pipeline.md` 2.11.1 `brief()` (every
  step is told it may return `needs_spike`); `STATE.md` spike requirement ("reachable from every level: a person,
  any agent ...").
- Problem: `code-reviewer` and `holdout-evaluator` are barred from spikes so that a finding cannot carry walled
  material back to them. The reason is sound, but the change departs from a stated user requirement and is not
  raised as a question. The two designs also contradict each other. The pipeline wrapper tells every step it may
  return `needs_spike` and serves the request, so the wall the roster meant to protect is crossed through the
  wrapper anyway.
- What breaks: The user requirement, and either the holdout wall (the pipeline serves the request) or the step
  (it is told two incompatible things).
- Proposed resolution: Keep the capability and the wall together. The spike request carries the asker's type,
  `vsdd spike open` records `walls_of = <asker>`, and the hook applies the union of the spike runner's and the
  asker's `read_deny` to that runner. Then both roles can ask. If that is rejected, list the exclusion as an open
  question for the user, and have `brief()` omit the spike line for these two types.

### COV-12: Tools and assets behind retained skills and methods have no home

- Severity: major
- Documents and sections: `agents-skills.md` 2.12 (`export-pdf`; `author diagram`; `preview`), 5 N-PKG-4
  ("the last two were missing from the old plugin and are not in the packaging draft"); `packaging.md` 2.3
  layout, 2.10, 2.12 (no package); `state.md` 2.13 (27 scaffold files, including the 23-file design-system
  starter, demo `.tape`, `.spec.ts`, the CI job and the justfile, "owned by the skill that copies them, under that
  skill's folder", but no skill in the roster copies them; methods are flat files).
- Problem: Two shipped skills drive tools no design provides: a diagram renderer for Excalidraw, Mermaid and
  draw.io exports, and a Markdown-to-PDF converter. A standard-library Python package cannot render a PDF, so this
  needs an external engine, which is a prerequisite decision. The design-system starter and the demo templates
  that the `ux-designer` and `demo-recorder` methods would use have no directory.
- What breaks: `export-pdf` and `author diagram` ship as entry points that cannot work, which repeats the old
  plugin's broken state for both. Design-system bootstrap loses its starter. Demo recording loses its
  templates.
- Proposed resolution: Packaging adds `vsdd render diagram` and `vsdd export pdf`. Both can run on Chromium
  through the bundled Playwright, or on pandoc if present, declared as an optional prerequisite that
  `vsdd doctor` checks. Packaging also adds `<plugin>/assets/{design-system,demo,scaffold}/`, which methods cite
  by path and L-XREF checks. If either tool is not wanted in 1.0, drop the skill with a reason the user approves.

### COV-13: `vsdd` is not on the person's shell PATH, but recovery instructions tell the person to run it

- Severity: major
- Documents and sections: `packaging.md` 2.10 (`bin/vsdd` "puts it on the Bash tool's PATH"), 2.11
  (getting-started and troubleshooting tell the person to run `vsdd doctor`); `state.md` 2.10 and 2.11 ("the
  command a person runs ...: `vsdd lock release`"); `pipeline.md` 2.6 ("a person can also run `factory decide`
  in their own terminal"); `hooks.md` 2.2 (block message "Run: vsdd doctor"), 2.12 (`$ vsdd explain`), 2.11.5.
- Problem: The platform puts a plugin's `bin/` on the Bash tool's `PATH` only. In the person's terminal, `vsdd`
  is "command not found" unless they type the versioned cache path. Yet the person-only routes depend on it:
  deciding outside a session, releasing a crashed run's lock, diagnosing gates that are down, and explaining a
  block.
- What breaks: The human escape hatches, at exactly the moments a person needs them. The `decide` fallback in
  COV-01 does not exist in practice.
- Proposed resolution: `setup` offers to install a stable shim (`~/.local/bin/vsdd`) that resolves the current
  plugin root from `~/.claude/plugins/installed_plugins.json` and then runs the launcher. `vsdd doctor` reports
  whether the shim is present. Also probe whether `! vsdd ...` typed in a session gets the plugin `bin/` on
  `PATH`, and whether hooks see it. If both hold, documentation can phrase every human instruction that way.

### COV-14: The agents-skills mapping cites workflows and gates the pipeline does not define

- Severity: minor
- Documents and sections: `agents-skills.md` 2.12 (assumed workflow names), 3.2 rows 16, 27, 35, 45, 46, 49,
  50, 54, 57, 58, 62, 67, 69 to 72, 86, 96, 108, 112 to 115, 123; `pipeline.md` 2.3.
- Problem: About thirty rows of the skill mapping point at `stories`, `holdout`, `refine`, `harden`,
  `converge`, `fix` and `multi-repo`, none of which exists. Two rows (54 `market-intelligence-assessment` and 115
  `validate-brief`) promise "the first gate of `spec`". The pipeline has no such gate, and it makes market
  assessment off by default, run through `discover` in `assess` mode.
- What breaks: The mapping cannot serve as the build's coverage checklist. A builder following row 54 would add
  a gate the pipeline rejected.
- Proposed resolution: Re-key every row to the pipeline's names: `stories` becomes `plan`; `holdout`, `refine`,
  `harden` and `converge` become `verify`; `fix` becomes the `feature` routes `trivial`, `bugfix` and `hotfix`, or
  story kind `fix`; `multi-repo` becomes `multi`. Correct rows 54 and 115.

### COV-15: Two hook mapping rows claim coverage by artifact kinds that do not exist

- Severity: minor
- Documents and sections: `hooks.md` 3.1 rows 16 (`validate-finding-format`) and 20
  (`validate-pr-description-completeness`); `state.md` 2.5 (no finding-file kind and no pull request description
  kind), 2.6 and section 3 (the pull request description is derived).
- Problem: Both rows say "merged into `artifacts`" via an artifact kind. The state registry has neither kind:
  findings live in step records, and the pull request body is rendered by the tool.
- What breaks: Two old checks look covered but are enforced nowhere, so the hooks mapping overstates coverage.
- Proposed resolution: Re-mark row 16 as "replaced: `finding.schema.json` validated by the workflow runtime".
  Re-mark row 20 as "replaced: the tool renders the body from a fixed template; a unit test asserts its required
  sections".

### COV-16: `onFailure: "block"` is relied on beyond its documented scope, so missing plugin files would fail open

- Severity: minor
- Documents and sections: `hooks.md` 2.3 (notes on wiring: "the only defence if the plugin's files are
  missing"), 2.9 (row "the hook cannot start at all"), 2.16, and its use on `UserPromptExpansion`;
  `reference/claude-code-plugin-platform-facts.md` section 5 (`onFailure` "applies to `PreToolUse` timeouts").
- Problem: The documentation shows `onFailure` covering `PreToolUse` timeouts. The design assumes it also turns
  "cannot start" and "unexpected exit code" into a block, and applies it to another event. If `py/launch.sh` is
  absent (a broken install, or the cache cleanup in packaging risk 6), `sh` exits 127, which is non-blocking.
- What breaks: Every gate switches off silently, which is the failure `STATE.md` says must never happen.
- Proposed resolution: Make the `hooks.json` command guard itself. For example:
  `sh -c 'f="$CLAUDE_PLUGIN_ROOT/py/launch.sh"; [ -r "$f" ] || { echo "vsdd-factory: plugin files missing, reinstall" >&2; exit 2; }; exec sh "$f" hook gate'`,
  with exit 0 in `notify` mode. Keep `onFailure` as an extra layer. Add a conformance probe for `onFailure` on a
  missing file and on `UserPromptExpansion`.

### COV-17: Research spikes run without a worktree, but the hook matches spikes by worktree

- Severity: minor
- Documents and sections: `pipeline.md` 2.11.1 `runSpike()` (no isolation when `kind` is `research`);
  `hooks.md` 2.11.6 (ledger entry matched "for the runner's worktree"; a spike runner in the main checkout can
  write nothing), 5 S7 `spike_entry(project, worktree)` and P5; `agents-skills.md` 2.11 (a dispatch without
  isolation is rejected).
- Problem: Parallel review lenses can each request a research spike. Those spikes all run in the main checkout,
  so they share one "worktree" key. The hook then attaches the time box, the open-entry test and the stop test to
  the wrong ledger entry. The agents-skills design forbids non-isolated spikes outright.
- What breaks: Misattributed time boxes and refused or wrongly allowed stops when two research spikes overlap.
  It also leaves the designs in disagreement about whether isolation is mandatory.
- Proposed resolution: Always isolate spike runners. The documented cost is half a second, and the worktree is
  removed when unchanged. Also key ledger entries by the `agent_id` that `spike-bounds` already records on first
  sight, with the worktree as a secondary field.

### COV-18: Child workflows share the parent's agent counter, so the wave's reserve is not safe

- Severity: minor
- Documents and sections: `pipeline.md` 2.5 (limits), 2.11.3 (`STORY_WORST`, `GATE_RESERVE`, `fix()` calling
  `runStories` with no reserve check; children receive `limits.agents = 900`); platform workflow reference
  (`workflow()` children share the run's concurrency cap and agent counter; the 1,000-agent cap is hard).
- Problem: Each child story checks only its own count against 900. Fix rounds start more story children without
  checking what is left. When the shared cap is reached, `agent()` throws. That includes the checkpoint relay
  inside `close()`.
- What breaks: The run ends without a checkpoint. The lock is held for twenty minutes, state is not pushed, and
  counts are lost. Re-running recovers, but the "every workflow ends with a checkpoint" guarantee (`state.md`
  P-4) is broken.
- Proposed resolution: Pass each child `limits.agents = STORY_WORST`. Before each fix round, check
  `RUN.agents + units * STORY_WORST + CLOSE_RESERVE <= limit`, and return `partial` if it fails. Hold a fixed
  reserve for `close()`. Model the shared counter in the packaging harness (its table says "counted", not
  "shared").

### COV-19: Existing planning documents outside the 1.0 layout have no intake path

- Severity: minor
- Documents and sections: `pipeline.md` 2.9.3 ("Intake of existing documents needs no special path"), 3.1
  (`planning.lobster` row); `agents-skills.md` 3.2 row 5 (`artifact-detection` merged into `status`); old
  `artifact-detection` and `planning.lobster` (detect, validate and route existing briefs, PRDs and architecture).
- Problem: The survey treats a stage as done only when registry-valid files exist under `.factory/specs/`. A
  person who arrives with a PRD or architecture document elsewhere in the repository has no way to bring it in.
  The `brief` skill accepts notes, and nothing else does.
- What breaks: The old adaptive planning intake. Existing documents are ignored and regenerated from scratch.
- Proposed resolution: Add a `sources` argument (a list of paths) to `spec`, and an `intake` stage
  (product-owner, plus architect for architecture documents) that converts them into registry artifacts before
  the domain stage. The survey then reports the converted stages as done.

### COV-20: `spike` is both a workflow and a skill in the same namespace

- Severity: minor
- Documents and sections: `pipeline.md` 2.3 (workflow `spike`), 2.8 ("the slash command is a workflow and not a
  skill"), 5.2 A5 ("No model-visible spike skill"); `agents-skills.md` 2.12 (model-visible skill `spike`), 4 P-9,
  P-8 lint ("no skill has a workflow's name").
- Problem: Both designs claim `/vsdd-factory:spike`. The platform's behaviour when a workflow and a skill share a
  qualified name is unverified, and the agents-skills lint fails the build on the collision.
- What breaks: One entrance becomes unreachable, or the build fails.
- Proposed resolution: Keep the model-visible skill `spike` for quick spikes, because the main session needs a
  listed entry it can choose on its own. Rename the workflow `spike-named` for open, advance and close of named
  spikes. Record the decision in both designs.

### COV-21: Explicit tool lists in workflow steps interact with the platform in two unverified ways

- Severity: minor
- Documents and sections: `agents-skills.md` 2.5 (SPAWN listed on most agents; "a workflow step never has this
  tool, whatever the file says"; RESULT listed defensively), 7 V2; `pipeline.md` 5.2 A3 (clerk "Tools: Bash
  only"); `STATE.md` (the verified "no Agent tool" fact was observed with agents that had no `tools` list).
- Problem: Two interactions were never observed. First, whether an agent that lists `Agent` explicitly still
  loses it when run as a workflow step. Second, whether an agent whose list omits `StructuredOutput`, like the
  pipeline's Bash-only clerk, can still return a schema result.
- What breaks: If `Agent` survives, workflow steps dispatch spike runners directly. That bypasses the wrapper's
  per-step accounting, and nested dispatch from a step may simply fail and waste the attempt. If
  `StructuredOutput` is hidden, every clerk step returns nothing and every workflow stops at its survey.
- Proposed resolution: Add both checks to the probes that run before the agent files are written
  (agents-skills section 7, packaging B8). Make the clerk's list `Bash, StructuredOutput` now, matching the
  defensive RESULT set.

## Areas checked and found sound

- Old workflow files: all 17 are mapped in `pipeline.md` 3.1, the 14 documented paths in 3.2, and the 37
  approval steps in 3.5, whose counts sum to 37.
- Old agents: all 35 have a disposition in `agents-skills.md` 3.1.
- Old skills: all 127 have a row in `agents-skills.md` 3.2, and the totals (6, 33, 66, 12, 10) sum to 127. Only
  the target names are wrong (COV-14).
- Hook registry: all 79 entries are mapped in `hooks.md` 3.1, and the counts by event match the old map
  (PreToolUse 22, PostToolUse 41, SubagentStop 6, and the rest). The two errors are COV-15.
- `bin/` tools (21), observability files (15), and templates (136, accounted in `state.md` 2.13) are all mapped.
- The 11 old maintenance sweeps are all present among the 12 in `pipeline.md` 2.9.10.
- The observability stack, its dashboards (event names re-pointed), the `watched-factories` registry and Claude
  telemetry wiring carry over.
- Nesting: only `story` is ever a child, it calls no workflow, and spikes inside a child run as steps. This
  matches the spike results.
- No design takes user input mid-run, relies on resuming a run from a new session, or has a plugin agent declare
  hooks, MCP servers or permission mode. The lint forbids those keys.
- Hook timeouts are in seconds, with a test that fails above 60. The uv guard and the exit-code mapping match the
  measured facts.
- The workflow script API used in the skeletons (`budget`, `meta.whenToUse`, `phases[].model`, `agentType`,
  `isolation`, `effort`, `schema`, `parallel`, `workflow`, top-level `return`) matches the platform reference
  and the spike scripts.
- The designers already flagged the remaining unverified items correctly: the `StructuredOutput` payload and
  retry, `SubagentStop` refusal for workflow steps, Grep and git exclusions, `${CLAUDE_PLUGIN_ROOT}` in agent
  bodies, `isolation` and `omitClaudeMd` in agent frontmatter, and `bin/` on `PATH` inside workflow steps.
