---
kind: topic
id: seams-pipeline
sources: [pipeline.md#4, pipeline.md#5.1, pipeline.md#5.2, pipeline.md#5.3, pipeline.md#5.4]
---

# Pipeline seams

The interfaces the pipeline design provides to and needs from the other designs, each linked to the
thing that now defines it. This topic links; it does not redefine.

## What the pipeline provides

- **I1 — Twelve workflow commands**: names, arguments and end states; each returns a workflow result.
  Consumers: skills, the driver, people. Defined by the twelve workflow files;
  [[schema:workflow-result]].
- **I2 — Stage structure and methods**: the stage tables of the twelve workflow files; there is no
  stage registry file — the scripts hold the structure, the survey returns the stage names recorded
  as done, and universal test 6 checks every step's method. Consumers: workflow authors; the harness.
  Defined by the workflow files; [[table:step-methods]].
- **I3 — The route table, the preconditions and the fast-route rule**: implemented once, as ordered
  lists in the tool's Python with a fixture per row; the fast-route rule in `vsdd feature route`;
  `next` is the workflow-result object everywhere, including `vsdd status --json`. Consumers:
  `vsdd status`, the briefing, the driver, the survey. Defined by [[table:routes]],
  [[table:preconditions]], [[command:feature-route]].
- **I4 — Result schemas** as files in `<plugin>/schemas/`. Consumers: agents, tool, hook runner.
  Defined by [[topic:result-schemas]].
- **I5 — The prelude**, kept in `workflow-lib/prelude.js` and stamped between the markers. Consumers:
  packaging (stamping), every workflow. Defined by [[topic:prelude]].
- **I6 — The convergence rule**, one function used at every perimeter. Consumers: the tool's verdict
  function, reviewers. Defined by [[topic:prelude-converge]].
- **I7 — Step contracts**: what each agent type is given and what it returns; severity definitions.
  Consumer: agents designer. Defined by the workflow files; [[table:step-methods]];
  [[table:severity]].
- **I8 — Gate catalogue**: eleven gates — `red` (with `--refreeze`), `green`, `demo`, `spec`, `plan`
  (with `--story` and admission), `wave`, `release`, `ingest`, `contracts`, `integration`, `sweep`.
  `wave`, `release`, `ingest` and `sweep` run every due check as one job; the checks `baseline`,
  `ui`, `formal`, `perf`, `audit` and the rest are selectable with `--only`. The state design owns
  the commands; this design owns the pass conditions. Consumer: tool. Defined by [[topic:gates]];
  pass conditions in the workflow files.
- **I9 — Decision kinds and choices**. Consumers: state and hook designers. Defined by
  [[topic:human-stops]], [[table:decision-kinds]].
- **I10 — The spike protocol**; schemas for the spike request and finding. Consumers: agents, tool,
  hook runner. Defined by [[topic:spikes]], [[schema:spike-request]], [[schema:spike-finding]].
- **I11 — Limits**, as configuration defaults. Consumer: tool (as configuration defaults). Defined
  by [[table:limits]].
- **I12 — Reserved slash names**: the twelve workflow names. `decide` and `drive` are skills.
  Consumer: skills designer. Defined by the workflow files; [[skill:decide]], [[skill:drive]].

## What the pipeline needs

### From the state designer

- Project configuration: the one key list is the configuration key table — an unknown key is an
  error, so there is no second list. The keys this design relies on: `commands.*`, `branches.*`,
  `delivery.*`, `convergence.*`, `holdout.*`, `mutation.kill_rate` by criticality, `limits.*`,
  `stops.*`, `evidence.*`, `notify.channel`, `maintain.sweeps`, `release.*`, `[[discover.source]]`
  ([[config:commands]], [[config:branches]], [[config:delivery]], [[config:convergence]],
  [[config:holdout]], [[config:mutation]], [[config:limits]], [[config:stops]], [[config:evidence]],
  [[config:notify]], [[config:maintain]], [[config:release]], [[config:discover]]).
- `vsdd survey` ([[command:survey]]): returns the survey fields. Applies the preconditions and the
  three-part done test. Idempotent. Creates story worktrees at `.worktrees/<story-id>/` and the
  integration worktree at `.worktrees/_integration/` (per repository in a hub); the worktree library
  it uses also serves the admission commands ([[topic:worktrees]]).
- Step record store ([[record:step]]): every step result is retrievable by
  `(run, workflow, subject, name, attempt)` before the next step starts.
- Evidence store ([[record:evidence]]): one record per gate and commit; writable only by the tool
  ([[topic:evidence-lookup]]).
- Decision store ([[record:decision]]): records with `id`, `kind`, `status`, `choice`, `note`,
  `digest`, and a readable packet. Opened only by the checkpoint ([[topic:decisions]]).
- Spike ledger ([[artifact:spike-ledger-entry]]): one per project, with the fields
  `id`, `question`, `asker`, `asker_type`, `read_deny`, `agent`, `run`, `ref`, `opened`, `minutes`,
  `verdict`, `answer`, `method`, `worktree`; named spikes keep `.spike/STATE.md` and `.spike/TASKS.md`
  in their worktree.
- `vsdd checkpoint` ([[command:checkpoint]]): does what the checkpoint step lists and returns the
  close fields. It decides from disk and takes no status from the script.
- Run lock ([[record:lock]], [[topic:run-lock]]): scoped to repository and branch; expires; a refused
  survey returns `ok: false` with the holder.
- `vsdd status --json` and the session briefing ([[command:status]], [[topic:briefing]]): one
  function, computed from the records each time; there is no stored status record. They include
  `next` computed from the route table, open decisions, named spikes, and whether the driver is
  active or paused so a session continues it after compaction.
- Story facts ([[artifact:story]]): the tool can read from a story — id, wave, dependencies,
  criticality, kind, security and UI flags, whether evidence recordings are required, acceptance
  criteria ids, contract ids.
- Demo evidence location ([[record:evidence]]): somewhere the demo gate can check that is not bulk
  media on the product branch.

### From the agents and skills designer

- Agent types: the 20 roster agents, planner/builder/reviewer/clerk; the `ROLE` table is generated
  from the roster ([[table:roster]], [[topic:model-tiers]]).
- Every agent works as a workflow step: it needs no Agent tool, reads its inputs by path, runs the
  named gate command last, and returns a step result ([[topic:agent-file]]).
- `clerk` ([[agent:clerk]]): tools `Bash, StructuredOutput`; system prompt of a few lines — run the
  one command with a Bash timeout of 600000, run it again while its output says `pending`, return its
  JSON.
- `spike-runner` ([[agent:spike-runner]]): `isolation: worktree` in its file (honoured for a
  dispatch; a workflow step passes isolation itself). First command `vsdd spike open`, last command
  `vsdd spike record`, both with file inputs.
- Planners with a `vsdd`-only shell: `product-owner`, `architect`, `ux-designer` and `story-writer`
  have Bash limited to single `vsdd` calls in the read and measure groups (and `feature route` for
  the architect), so they can finish a step with the check for their own work
  ([[topic:tool-sets]]).
- One shared paragraph in every agent file: "If you need an experiment: with the Agent tool, dispatch
  `vsdd-factory:spike-runner`; without it, return `needs_spike` with a spike request." No
  model-visible spike skill ([[topic:agent-shared-block]]).
- Skills: `brief` ends by running `vsdd checkpoint brief`; `setup`; `decide`; `drive`; `status`. No
  skill reuses a workflow name, and no skill re-implements a workflow ([[skill:brief]], [[skill:setup]],
  [[skill:decide]], [[skill:drive]], [[skill:status]]).
- Reviewer discipline: stable finding keys, the severity definitions, an anchor for every severe
  finding, and a coverage entry for every item they are told to account for.

### From the hooks designer

- Store step results: when a `StructuredOutput` call carries `step.run`, write the step record, with
  the calling agent's type, before the call completes; pass other calls through; store the full form
  of a walled finding in the holdout class ([[rule:step-results]], [[topic:walled-findings]]).
- Ground findings: reject a review result whose finding names a missing file, a line range outside
  the file, or a quote not found there; give the reason so the agent corrects it. Findings of
  category `holdout` have no location and are not grounded.
- Walls by agent type: W1–W3 keep hidden scenarios, evaluations and stored step records inside their
  walls, the main session included; W4 makes frozen tests read-only to builders other than the
  test-writer; W5 confines builders to their story worktree; W6 confines the spike runner to its own
  worktree ([[topic:information-walls]], [[table:walls]]).
- Guards: G1 source files in a story worktree are not writable until red evidence exists (except the
  kinds with no red gate); G2 no agent merges or pushes to the integration or release branch except
  through `vsdd story deliver` and `vsdd release`; G3 while the latest `spec` decision is approved, no
  specification file is writable outside a run that holds an amendment; G4 the evidence store and the
  decision store are not writable by any agent tool call ([[rule:tests-first]], [[rule:merge-guard]],
  [[rule:gate-guard]]).
- Decision capture: on the prompt-expansion event for the `decide` command, write the decision
  record; block `vsdd decide` when called by any agent ([[rule:decisions]]).
- Spike rules: pipeline gates do not apply inside a spike worktree; after the time box refuse every
  tool call except `vsdd spike record`; refuse a spike runner that dispatches agents; refuse a spike
  runner that finishes with no ledger record; record an admission when an agent dispatches the spike
  runner and give the runner the asker's read walls ([[rule:spike-bounds]]).
- Project resolution by path: find the governing project by walking up from the file or directory
  being touched to the state directory ([[topic:state-directory]]).
- Clerk restriction: a `clerk` may run only `vsdd` commands.

### From the packaging designer

- The `vsdd` tool on the agents' path: `survey`, `checkpoint`, `status`, `gate <name>` (the eleven),
  `feature route`, `story deliver`, `story new-fix`, `plan schedule`, `review prepare`, `env start`
  and `env stop`, `spike open`, `spike record`, `spike close`, `idea register`, `report debt`,
  `discover collect`, `release`, `repos split`, `decide`, with the global option `-C <dir>`. Each
  prints one JSON object with `ok`; a long command prints `state: "pending"` while its job runs
  ([[topic:long-commands]], [[topic:command-conventions]]).
- Prelude stamping and static tests: stamp the block between the markers into every workflow file;
  fail the build if any copy differs ([[packaging:build-step]]).
- One schema source: `<plugin>/schemas/*.schema.json`, stamped into the prelude and loaded by the
  tool and the hook runner.
- Conformance fixtures: the same input cases run through the JavaScript `judge()` /
  `converge()` / `holdoutFindings()` and through the tool's implementations, and must agree. The
  route is not duplicated: only the tool computes it ([[packaging:tests-conformance]]).
- A script test harness: runs each workflow against a fake `agent()` in CI
  ([[packaging:tests-workflows]]).
- Headless recipe: how to run `maintain` and `discover` on a schedule with the Workflow tool allowed
  ([[packaging:observability-stack]]).
