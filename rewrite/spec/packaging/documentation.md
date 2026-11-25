---
kind: packaging
id: documentation
package: D2
sources: [packaging.md#2.11, BLUEPRINT.md#5.5]
---

# The documentation set

Principles: hand-written pages never state a count of agents, skills, hooks or anything else; everything that lists
shipped parts is generated from those parts; nothing documents a mechanism that does not exist.

- `README.md` — for everyone. What it is, prerequisites, install, the first command, a map of the docs. Under 150
  lines.
- `plugins/vsdd-factory/README.md` — for someone looking at an installed copy. Thirty lines and a link.
- `docs/getting-started.md` — for a new user. Claude Code at the floor, `uv`, git, `gh`. Install. Initialise a
  project. What the session briefing tells you. `vsdd doctor` ([[command:doctor]]).
- `docs/concepts.md` — for a new user. The method in 1.0: the specification chain, stories and waves, tests before
  code, adversarial review and the convergence rule, holdout scenarios, spikes, and a glossary.
- `docs/running-the-pipeline.md` — for a user. Each workflow command, when to use it, where it stops for a decision,
  how to resume, how to start a spike, the `drive` skill, and a tested recipe for running `maintain` and `discover`
  on a schedule (cron or launchd running `claude -p "/vsdd-factory:maintain"` in the project with the Workflow tool
  allowed).
- `docs/configuration.md` — for a user. The project configuration file, the three API keys, turning a rule off or
  tuning it ([[topic:project-configuration]], [[topic:rule-tuning]]).
- `docs/observability.md` — for a user. The event log, `vsdd events`, `vsdd obs`, the dashboards, sending events
  elsewhere ([[command:events]], [[command:obs]], [[packaging:observability-stack]]).
- `docs/troubleshooting.md` — for a user. "A tool call was blocked": how to read the reason. `uv` missing. The plugin
  did not load. Old state found.
- `docs/migrating-from-rc.md` — for release-candidate users. The cutover steps for a project
  ([[packaging:cutover-projects]]).
- `docs/reference/agents.md`, `skills.md`, `workflows.md`, `rules.md`, `cli.md`, `configuration-keys.md` — for lookup.
  **Generated** by `tools/gen_reference.py` from agent and skill frontmatter, workflow `meta`, the rule registry, the
  `vsdd` argument parser and the configuration schema. Checked for drift ([[packaging:reference-pages]]).
- `docs/architecture.md` — for a contributor. The parts, how one story travels through them, and a short list of the
  decisions that shaped 1.0, each with its reason. Distilled from the blueprint.
- `CONTRIBUTING.md` — for a contributor. The layout, the one check command, how to add an agent, a skill, a workflow,
  a rule or a subcommand, and the four test layers ([[packaging:check-command]]).
- `RELEASING.md` — for the releaser. The release procedure, about fifty lines, and why the marketplace
  is a separate repository ([[packaging:plugin-release]]).
- `CHANGELOG.md` — for everyone. Starts at 1.0.0.
- `CLAUDE.md` — for agents working on this repository. [[packaging:repo-claude-md]]

Delivered by package D2, together with the generated reference (the former D1), the schedule recipe and
troubleshooting ([[package:D2]]). The contributor documents (final `CLAUDE.md`, `CONTRIBUTING.md`, `RELEASING.md`,
`CHANGELOG.md`, `README.md`) are package D3.
