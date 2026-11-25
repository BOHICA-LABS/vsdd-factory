---
kind: doc
id: conventions
sources: [BLUEPRINT.md#4, BLUEPRINT.md#4.1, BLUEPRINT.md#4.2, BLUEPRINT.md#4.5, BLUEPRINT.md#4.8]
---

# Shared conventions

Section 4 of the blueprint settles every name, path, schema and identifier on which the five designs disagreed.
It is authoritative. This file carries its names and paths, its model tiers and its identifier rules. The last
part of this file says where the rest of that section is.

## Names and paths

### Plugin name and namespace

`vsdd-factory`. Agents are `vsdd-factory:<name>`; skills and workflows are `/vsdd-factory:<name>`.

### The command-line tool

`vsdd`. The pipeline design's working name `factory` is retired.

### Plugin layout

`<plugin>/agents/`, `skills/`, `workflows/`, `methods/`, `templates/`, `schemas/`, `assets/` (`design-system/`,
`demo/`, `scaffold/`), `hooks/hooks.json`, `py/` (`launch.sh`, `find-root.sh`, `hook.py`, `cli.py`, `vsdd/`),
`bin/vsdd`, `observability/`, `.mcp.json`. There is no `pipeline/` directory: stage structure lives in the
workflow scripts and the route table is code ([[topic:single-sources]]).

### Python package

`py/vsdd/` with `project.py`, `config.py`, `frontmatter.py`, `events.py`, `schema.py` (a small JSON Schema
checker), and the sub-packages `hooks/`, `rules/`, `policy/`, `state/`, `gates/`, `delivery/`, `cli/`.

### Policy tables

`py/vsdd/policy/`: `artifacts.toml` (artifact kinds, including the story kinds), `classes.toml` (path classes),
`walls.toml` (one row per caller, the command groups a row may run, and what each read command reveals).

### Prelude source and markers

`workflow-lib/prelude.js` at the repository root. Stamped between `// >>> stamped:prelude` and
`// <<< stamped:prelude`.

### Shared agent text

`workflow-lib/agent-shared.md` (the block every agent file carries) and `workflow-lib/severity.md` (the severity
table), stamped into agent files by the build.

### State directory

`.factory/` at the project root, a worktree of the branch `factory-artifacts`. Exactly one per project, and
never present inside a story or spike worktree.

### Local store

`.factory/local/`, untracked: runs, step records, evidence, jobs, the lock, the driver flag, hook records, the
event log.

### Project configuration

`.factory/factory.toml`. Created by `vsdd init`, edited by a person. Code writes it in three stated cases only
([[topic:project-configuration]]).

### Worktrees

`.worktrees/<story-id>/` for a story, `.worktrees/_integration/` for wave and release gates
(`.worktrees/_integration/<repo>/` in a hub), `.worktrees/spike-<name>/` for a named spike. Quick spikes use a
platform worktree.

### Branches

`story/<id>`, `spike/<name>`, and `hotfix/<version>` for a critical fix cut from a release tag; the integration
and release branch names come from `[branches]`.

### Event log

`.factory/local/events/<UTC date>.jsonl`. Only the hook runner and the tool write it.

### Person's terminal

`~/.local/bin/vsdd`, a shim that `setup` offers to install. It finds the plugin version enabled for the current
project in `~/.claude/plugins/installed_plugins.json` (the project entry first, then the user entry) and prints
the reinstall command if that copy has no `bin/vsdd`. Every message addressed to a person names the in-session
form first (`/vsdd-factory:<skill>`) and the terminal form second.

### Python and uv

Python 3.11 or later, standard library only, started through `uv` with no downloads and no network on both the
hook path and the tool path. Nothing in the plugin installs a Python: when none is found, the launcher's message
names the terminal command that does (`uv python install 3.12`).

### Minimum Claude Code

2.1.331.

Where each is defined in full: the repository and plugin layout in [[packaging:repo-layout]]; the Python package
in [[packaging:python-package]]; the policy tables in [[table:artifacts]], [[table:path-classes]],
[[table:walls]] and [[table:callers]]; the prelude in [[topic:prelude]]; the shared agent text in
[[topic:agent-shared-block]] and [[table:severity]]; the state directory in [[topic:state-directory]] and
[[topic:state-layout]]; the project configuration in [[topic:project-configuration]]; worktrees in
[[topic:worktrees]]; branches in [[config:branches]]; the event log in [[topic:event-log]]; the terminal shim in
[[packaging:terminal-shim]]; the launcher in [[topic:hook-launcher]] and [[packaging:cli-entry]]; the minimum
Claude Code version in [[packaging:plugin-manifest]].

## Model tiers

`tests/lint/roster.toml` is the single source.

| Tier | Used for |
|---|---|
| planner | Planning, thinking and creative roles |
| builder | Tests, implementation, verification harnesses, demos, documentation, operations |
| reviewer | Every independent reviewer |
| clerk | Steps that run one `vsdd` command and relay its output (decision D5) |

The tiers' model, effort and colour are `tests/lint/roster.toml`'s `[tiers]` table ([[table:roster]]), the
single source. Agent files must match it (lint), and the build generates the
tier and role tables in the workflow prelude from it. Every workflow step sets both model and effort from the
generated table, because it is unverified that a step inherits the agent file's effort.

One marked override exists, as decision D5 allows. `[overrides]` in the roster lists the steps that run off
their agent's tier. The build generates an `OVERRIDE` table from it, `step()` consults that table by workflow
and stage, lint rejects any override the roster does not list, and a harness test asserts the tier those two
stages use.

Workflow phase metadata names no model, so the step's own model and effort are the only setting. The models are
aliases (`fable`, `sonnet`, `opus`). The weekly conformance job records the model ID behind each alias and runs
the story tier again when one changes.

End-to-end tests run on cheap models: when `VSDD_TEST_TIER` is set in the environment, `vsdd survey` returns a
tier override, the step wrapper uses it, and the briefing announces it.

The roster is [[table:roster]]. The generated tables and `step()` are in [[topic:prelude]] and
[[topic:prelude-steps]]; the build that generates them is [[packaging:build-step]]. How a tier reaches an agent
file, and the cost context, are in [[topic:model-tiers]]. Decision D5 is in [[doc:decisions]].

## Identifiers

Identifiers are the state design's ([[table:identifiers]]): `BC-S.SS.NNN` for a behavioral contract, `VP-NNN` for a
verification property, `ADR-NNN`, `SS-NN` for a subsystem, `E-N` and `S-N.NN` for epics and stories, `AC-N` for an
acceptance criterion, `HS-NNN` for a hidden scenario, `W-NN` for a wave, `SP-NNN` for a spike, `ID-NNN` for an
idea, and `CR-NNN` for a queued change request (new). A path is a function of its identifier, identifiers are
never reused, and nothing stores a reverse link, a count or a version.

A finding's identity is its `key`, `<category>:<path>:<symbol or clause>`, the same in every pass (outside the
holdout wall a walled finding is named by a hash of its key, [[topic:walled-findings]]). A finding
built from hidden-scenario scores has the key `holdout:<HS id>`, or `holdout:<unit>:mean` or
`holdout:<unit>:spread`. A decision's identifier is its kind (`brief`, `spec`, `release`, `plan`, `ideas`) or
`<kind>.<subject>` (`finding.S-7.04`, `merge.S-7.04`, `blocked.S-7.04`). When two cycles are open at once (a
critical fix beside the main cycle) and both hold a decision with the same identifier, each is written
`<id>@<cycle>`, for example `release@hotfix-1.2.1`. The briefing prints the qualified form, and a bare
identifier that is ambiguous is refused with the list of choices. A step is identified by run, workflow,
subject, name and attempt.

The full list of identifier schemes, with patterns, examples and how each is allocated, is
[[table:identifiers]]. The finding key is a field of [[schema:finding]]. Decision identifiers are explained in
[[topic:decisions]].

## A naming rule from the skills and workflows

No skill has a workflow's name. The skills and the workflows are listed in [[table:roster]].

## Where the rest of section 4 is

- 4.3, the agent roster: [[table:roster]], one file per agent, [[topic:tool-sets]], [[topic:information-walls]]
  and [[table:walls]].
- 4.4, the provisional names the pipeline design used, mapped to their replacements: [[table:methods]]. The
  agent and method of every step are [[table:step-methods]], and the owner of every method is [[table:methods]].
- 4.5, skills and workflows: one file per skill and per workflow, and [[topic:workflow-arguments]].
- 4.6, story kinds: [[table:story-kinds]], [[topic:story-admission]], [[artifact:toolchain]].
- 4.7, state layout and path classes: [[topic:state-layout]], [[topic:computed-status]],
  [[topic:machine-records]] and one file per record, [[table:path-classes]].
- 4.9, the `vsdd` command: [[topic:command-conventions]], one file per command, [[topic:caller-identity]],
  [[topic:long-commands]].
- 4.10, result schemas: [[topic:result-schemas]], one file per schema, [[topic:holdout-findings]],
  [[topic:walled-findings]], [[table:severity]].
- 4.11, decisions: [[table:decision-kinds]], [[topic:decisions]], [[topic:human-stops]], [[topic:amendments]],
  [[topic:hotfix-cycle]].
- 4.12, limits and configuration keys: [[table:limits]], [[table:config-keys]], [[topic:prelude-run-stories]],
  [[topic:project-configuration]].
- 4.13, evidence, the lock and liveness: [[topic:evidence-lookup]], [[record:baseline]], [[topic:run-lock]],
  [[topic:caller-identity]], [[command:story-deliver]].
- 4.14, single sources and generated copies: [[topic:single-sources]], [[topic:resume]].
