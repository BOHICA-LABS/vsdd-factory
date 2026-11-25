---
kind: topic
id: skill-file
sources: [agents-skills.md#2.12]
---

# Skill files

Thirteen skills, in directories `<plugin>/skills/<name>/` with a `SKILL.md` each, invoked as
`/vsdd-factory:<name>`. One, `status`, is model-visible. A workflow is already a slash command
(`/vsdd-factory:<workflow>`), so no skill exists to start one, with one deliberate exception: `drive`, which
follows the route from one workflow to the next so a person does not type each command.

## The two shapes

A skill either instructs the main session or runs in a forked agent:

- **Main-session instruction.** `disable-model-invocation: true`, so only a person can run it; the body tells
  the main session what to run or dispatch, and relays what comes back. Twelve of the thirteen.
- **Forked.** `context: fork` with `agent: vsdd-factory:<name>` naming an existing agent (a lint-checked fact);
  the skill body becomes that agent's task. `research` and `retro`.

The skeletons of both shapes are in the skill files themselves ([[skill:decide]] and [[skill:research]] carry
them verbatim). The frontmatter keys beyond `name` and `description` used by the roster are
`disable-model-invocation`, `argument-hint`, `context` and `agent`.

## Why only one model-visible skill

The platform spike measured about fifteen cents of fixed context per agent start, and noted that skill
listings are part of it. The model needs to find one thing on its own: where the project stands. A question
that needs an experiment is answered by the `spike` workflow or by dispatching the spike agent, neither of
which needs a skill. Everything else is either started by a person or is a workflow. Agents themselves are
found through the agent listing, which is why the 20 descriptions are one sentence each.

Because `defer` cannot be invoked by the model, the old rule that only a person may defer work is enforced by
the platform and not by a paragraph of prose.

The lint budget for skills: exactly one file without `disable-model-invocation: true`, and it is `status`;
its description at most 400 characters; `SKILL.md` at most 150 lines; no skill has a workflow's name
([[topic:seams-agents]]).
