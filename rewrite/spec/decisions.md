---
kind: doc
id: decisions
sources: [BLUEPRINT.md#2, BLUEPRINT.md#2.1, BLUEPRINT.md#2.2, BLUEPRINT.md#2.3]
---

# Decisions

On 2026-11-23 you adopted the recommendations recorded here as D1 to D30. A decision you adopted later is
under "Decided after 2026-11-23", with its own date. Each is a fixed decision, and this file is the record
of it.

Under each decision, "Decided" is what was decided (for D1 to D30, the blueprint's "Recommendation" column)
and "What the answer changes" records what the other answer would have changed. The rest of the design is written to the
decisions as adopted; [[topic:decisions-built]] says where each one is built.

## Decided before the build starts

These change what is built.

### D1

- **Question.** Keep project state on its own git branch (`factory-artifacts`), mounted at `.factory/`? The
  alternative is a tracked directory on the integration branch.
- **Decided.** Keep the separate branch. Mounting, committing and pushing move into code, so it costs you one
  `/vsdd-factory:setup` per clone.
- **What the answer changes.** The alternative puts state commits in product history and hidden test scenarios
  in the product tree, and it forces your checkout to stay on the integration branch. It would change the state
  design, the hook walls and the importer.

### D2

- **Question.** May the plugin commit and push the state branch on its own? State commits skip the product's git
  hooks and signing. No product branch is touched.
- **Decided.** Yes, for the state branch only. `state.push = false` gives a project manual pushes.
- **What the answer changes.** "Commit or push only when asked" applied to state would put you back into every
  step.

### D3

- **Question.** Should the eight projects on the old plugin get an importer?
- **Decided.** Yes. `vsdd import` is mechanical, runs in place after tagging the old tip `factory-pre-1.0`, and
  keeps contract, story and decision identifiers. `vsdd init --fresh` gives a clean start where you prefer one.
- **What the answer changes.** Without it, build package S5 is dropped and each project starts again from
  nothing or stays on rc.25.

### D4

- **Question.** May the plugin approve its own routine tool calls, so that unattended runs do not stop on
  permission prompts? Plugin agents cannot set a permission mode, so in the default mode every edit, build, test
  and commit inside a workflow asks you.
- **Decided.** Yes, offered once per project at setup and scoped: edits inside story and spike worktrees and the
  state directory by the agent whose role may write them; the project's configured build and test commands;
  `git add`, `commit`, `status`, `diff` and `log` inside those worktrees; and `vsdd` commands the caller's role
  allows. A call is approved only when every other hook rule passes.
- **What the answer changes.** Two findings waited on this answer (COV-04, OPS-12) and are closed now that it is
  yes. If no: you run pipeline sessions in a permission mode of your choosing, a six-story wave raises dozens to
  hundreds of prompts, and scheduled runs cannot work. The design is the same either way; only the default of
  one hook rule changes.

### D5

- **Question.** Model policy details. (a) A fourth tier, "clerk" (Sonnet at low effort), for steps that only run
  one `vsdd` command and relay its output. (b) Opus at extra-high effort for reviewers, which you have not
  confirmed. (c) Tiers for roles your policy does not name: `codebase-analyzer` and `spike-runner` on the
  planner tier (Fable), `verifier` on the builder tier (Sonnet). (d) Model aliases (`fable`, `sonnet`, `opus`)
  instead of pinned model IDs.
- **Decided.** Accept all four. Clerk steps make no judgment and are about a third of all agent starts. Let the
  broad inventory passes of ingestion run on the builder tier as a marked override, because ingestion is the
  largest planner-tier cost. Re-run the end-to-end proof after any model release.
- **What the answer changes.** Each is one line in `tests/lint/roster.toml`, from which agent files and workflow
  scripts are checked or generated. Refusing (a) makes clerk steps slower and dearer with nothing gained,
  because they make no judgment.

### D6

- **Question.** Is the main session walled like an agent? It could not read hidden scenarios or edit
  specification files directly; you use an editor or `/vsdd-factory:author`.
- **Decided.** Yes, on by default, with a switch in project configuration.
- **What the answer changes.** It closes the last unenforced path around the walls. If no, the model you talk to
  can read scenarios and pass them to builders in prompts.

### D7

- **Question.** Is a typed command the only way to settle a decision in a session? Saying "approved" in prose
  records nothing, and the model is refused if it tries to record a decision for you.
- **Decided.** Accept. It is `/vsdd-factory:decide <id> <choice>`, one short line, and the one act a model must
  not be able to perform.
- **What the answer changes.** If no, approvals rest on what a model says you said.

### D8

- **Question.** Is `/vsdd-factory:spike` a workflow, with no model-visible spike skill? The two designs each
  shipped `spike` and the name can only exist once.
- **Decided.** Yes, the workflow. Limits and the ledger are in code, it shares one implementation with the
  wrapper inside every workflow, and it carries named spikes. The main session can start it, or dispatch the
  spike agent directly.
- **What the answer changes.** If you want a model-visible skill, the workflow is renamed `spike-run` and a
  small skill starts it; that adds one skill listing to every agent start.

### D9

- **Question.** Do you accept one clean review pass on the final commit at each perimeter (story, wave,
  release), with a cap of three passes, in place of three consecutive clean passes?
- **Decided.** Accept, with two review lenses on critical and high modules. The end-to-end proof measures how
  many findings are first caught at a later perimeter; tighten if that rate is high.
- **What the answer changes.** The old rule could not terminate. A stricter rule multiplies reviewer cost by
  about three.

### D10

- **Question.** Is one `verify` workflow covering the old phases 4 to 7 acceptable? Your requirement says each
  phase is its own workflow.
- **Decided.** Accept. The four check the same commit, share one fix loop and have no human decision between
  them.
- **What the answer changes.** Splitting them serialises the checks and adds three commands per cycle.

### D11

- **Question.** Do the old git rules hold as default gates: no AI attribution in commits or pull requests, no
  `--no-verify`, no force push, no direct push or commit to protected branches?
- **Decided.** Yes, on by default, each relaxable per project by a person. Setup also switches off Claude Code's
  own attribution trailer so builders are not blocked on their first commit.
- **What the answer changes.** `rewrite/STATE.md` listed them as an unconfirmed assumption; adopting this
  confirmed them.

### D12

- **Question.** Which gates may a project relax, and are two human-only escape switches acceptable?
- **Decided.** Lock the walls, write scopes, gate-guard, decision capture and step storage. Let a person set any
  other gate to advise or off by editing the project file. Keep `VSDD_FACTORY_HOOKS=off` (everything off,
  announced) and add `VSDD_FACTORY_HOOKS_QUARANTINE=<rule>` (one crashing rule off, announced), both environment
  variables only a person can set.
- **What the answer changes.** Without the switches, a bug in the hook runner locks a project until the plugin
  is uninstalled.

### D13

- **Question.** Research and browser servers (Perplexity, Context7, Tavily, Playwright): bundled, your own, or
  both?
- **Decided.** Bundle all four and have agents name both spellings of each tool; leave keys blank on machines
  that already run the servers.
- **What the answer changes.** Bundled servers need a one-time approval per project.

### D14

- **Question.** Two skills need tools the old plugin never shipped: a Markdown-to-PDF converter and a diagram
  renderer. Both need Node (`npx`), which the bundled servers already need.
- **Decided.** Keep both as `vsdd export pdf` and `vsdd render diagram`, with Node as an optional prerequisite
  that `vsdd doctor` checks. Build package T5 starts by proving the engines.
- **What the answer changes.** Dropping either skill would have removed T5 or half of it.

## Defaults in project configuration

These are defaults a project can change later.

### D15

- **Question.** Should a story merge without a person when every gate passes?
- **Decided.** Yes: `delivery.merge = "auto"`. Three things always wait for you: a story that touches a
  protected path (`.github/**`, `.env*`, `CODEOWNERS`, release configuration); a change that adds a dependency
  or upgrades one beyond what the approved architecture lists; and anything a project configures.
- **What the answer changes.** With `.github/**` protected, the story that first sets up CI waits for you once
  per project. Dependency changes in a scheduled maintenance run are batched into one decision.

### D16

- **Question.** Should a person approve the story plan?
- **Decided.** No stop by default. Coverage, cycles and size are checked by code. A project can opt in, and a
  stop opens automatically when the plan's estimate exceeds a configured ceiling.
- **What the answer changes.** One fewer stop per cycle.

### D17

- **Question.** Must a person authorise every release, including bug-fix patches?
- **Decided.** Yes for 1.0.
- **What the answer changes.** Automatic patch releases can follow once the rate of escaped defects is known.

### D18

- **Question.** May an agent arbiter (the architect) dismiss a high-severity finding that the builder disputed
  and that is still open at the cap?
- **Decided.** Yes, but never a critical or security finding, never a finding the builder did not dispute, and
  every dismissal is listed in the release packet.
- **What the answer changes.** The alternative is a human decision for every finding still open at the cap.

### D19

- **Question.** Do all changes to an approved specification found during build need your approval?
- **Decided.** Yes, batched once per wave through an amendment run.
- **What the answer changes.** Letting agents apply "clarifications" unapproved cannot be policed by code.

### D20

- **Question.** Should a market assessment run for every feature request?
- **Decided.** Off by default. Run `/vsdd-factory:discover` in assess mode when you want one.
- **What the answer changes.** The old flow ran it, with a stop, for every feature.

### D21

- **Question.** Are the spike limits right: 2 per step, 6 per run, 20 per project per day, 20 minutes each, 3
  named spikes open, 14 days of worktree retention?
- **Decided.** Start with these and adjust after the end-to-end proof.
- **What the answer changes.** Configuration only.

### D22

- **Question.** Is a demo script plus its text transcript enough as committed demo evidence, with screenshots
  capped at 300 KB and no committed video?
- **Decided.** Yes. Rendered media stays in `.factory/local/media/`.
- **What the answer changes.** The old repository carries 110 MB of committed recordings.

### D23

- **Question.** When a cycle closes, should evaluations and demo files leave the working tree? They stay in the
  state branch's history.
- **Decided.** Yes. `state.prune_closed_cycles = false` keeps them for projects with audit duties.
- **What the answer changes.** The tree stays bounded by the size of the specification.

### D24

- **Question.** Should hooks be silent in projects that are not initialised?
- **Decided.** Silent, except one migration line in a project with pre-1.0 state.
- **What the answer changes.** Unrelated projects pay nothing.

## Decided for cutover

### D25

- **Question.** Collapse to one long-lived branch at cutover?
- **Decided.** Yes. Fast-forward `main`, make it the default, delete `develop`.
- **What the answer changes.** Only the branch name CI watches.

### D26

- **Question.** Pay for end-to-end tests in CI with an `ANTHROPIC_API_KEY` secret?
- **Decided.** Yes: the smoke tier on pull requests that touch the plugin, the story tier at each release and
  weekly.
- **What the answer changes.** If no, the tests run locally before each release and the gate is a checklist
  item.

### D27

- **Question.** What does each of the eight projects do?
- **Decided.** Run `tools/cutover.py plan`, pilot on `vantage`, migrate all eight within two weeks, add no
  legacy marketplace entry.
- **What the answer changes.** The order and timing of the last cutover step, migrating the projects.

### D28

- **Question.** Should this repository run the factory on itself after 1.0?
- **Decided.** No for 1.0. Use ordinary pull requests and the CI checks.
- **What the answer changes.** The old engine did, and bookkeeping swamped delivery.

### D29

- **Question.** Use "Bohica Labs" and `bohica-labs` URLs in the manifest, and submit to Anthropic's directory?
- **Decided.** Yes to the names. No submission for 1.0, so the icon can be a placeholder.
- **What the answer changes.** Manifest fields.

### D30

- **Question.** Delete `docs/demo-evidence` (110 MB) and the 364 KB changelog from the tree at cutover?
- **Decided.** Yes. Both remain in history and under the tag `archive/rc-line`.
- **What the answer changes.** Repository size.

## Decided after 2026-11-23

D31 onwards are in [[doc:decisions-later]], each with its own date. A file that comes from one of them
cites it as `decisions.md#D<number>`.
