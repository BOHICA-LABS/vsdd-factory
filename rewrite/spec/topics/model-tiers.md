---
kind: topic
id: model-tiers
sources: [agents-skills.md#2.3, BLUEPRINT.md#4.2]
---

# Model tiers

Four tiers; three model settings and one effort exception. `tests/lint/roster.toml` is the single source of
truth for model and effort ([[table:roster]]), and the tier table itself is in the conventions
([[doc:conventions]]). Each agent file must carry its tier's values (lint), so a directly dispatched agent
gets them. The build generates the tier and role tables in the workflow prelude from the same file, so a
workflow cannot disagree with an agent file.

## Workflow steps

Workflow steps set both `model` and `effort`, from the generated table. Whether a step inherits the agent
file's effort is not verified, and a generated table is also where the end-to-end tests switch to cheap
models: when `VSDD_TEST_TIER` is set in the environment, `vsdd survey` returns a tier override, the step
wrapper uses it, and the briefing announces it. Workflow phase metadata names no model, so the step's own
model and effort are the only setting.

## Aliases, not pinned model IDs

`fable`, `sonnet` and `opus` follow the current model of each family (Fable 5.2, Opus 5.5 and Sonnet 5.5 at
the time of writing; all three accept `xhigh`). This is the low-touch choice, adopted as decision D5. The
weekly conformance job records the model ID behind each alias and runs the story tier again when one changes.

## One marked override

The roster's `[overrides]` table lists the only steps that run off their agent's tier ([[table:roster]];
decision D5). The build generates an `OVERRIDE` table into the prelude, `step()` consults it by workflow and
stage, and lint rejects an override the roster does not list; a harness test asserts the tier those two stages
use.

## Cost context for tier choices

At current list prices the planner model costs twice the reviewer model and four times the builder model per
token. Three roles are judgment calls under the user's policy and are raised as open question 1 of the design:
`codebase-analyzer` and `spike-runner` (planner) and `verifier` (builder). Seven agents run on the planner
tier; the tier is one line in each file, so it can be tuned after the end-to-end proof measures it.

## Cross-vendor review is not part of 1.0

The old documents promised a GPT adversary and a Gemini reviewer that never existed on Claude Code. In 1.0 a
reviewer differs from a builder by model (Opus against Sonnet) and, more importantly, by what it is allowed to
see ([[table:walls]]).
