---
kind: agent
id: holdout-evaluator
package: A6b
sources: [agents-skills.md#2.4, agents-skills.md#2.5, agents-skills.md#2.6, agents-skills.md#2.8, BLUEPRINT.md#4.3]
---

# holdout-evaluator

Reviewer. Scores the running product against hidden scenarios without seeing source, tests or specs.

## Tier and effort

reviewer — `model: opus`, `effort: xhigh` ([[topic:model-tiers]], [[table:roster]]).

## Tools

`Read`, SHELL, SPAWN, RESULT, BROWSER.

What it deliberately lacks: no `Grep` or `Glob`, so it cannot trawl the source tree. No Write: its evaluation
file is written by code from its result. Its shell is to start and exercise the product. A spike it asks for
inherits its walls. The named sets are defined in [[topic:tool-sets]].

## Is given / writes / returns

Is given: the scenario identifiers to run; how to start the product (`vsdd env start`). No source, test or
specification path.

Writes: nothing. Its evaluation file is written by code from its result.

Returns in `data`: holdout data ([[schema:holdout-data]]): `scores` (`scenario`, `satisfaction` from 0 to 1,
`must_pass`, `ran`); `symptoms` (`scenario`, `observed`, `expected`, with no scenario text and no location).
It returns no findings: the tool builds them from the scores, and the workflow never asks it for a gate.

Result schema: the common fields of [[schema:step-result]].

## Walls

Read walls: it judges the product from outside. Its deny list is the row in [[table:walls]]; it may read
`holdout`, `brief` and `docs`.
Write scope: nothing.

## Methods

`holdout-eval` (also the re-run half of the scenario-freshness sweep; the sweep's two halves follow from the
wall, and are described with [[topic:method-files]]).

Every method file and its owner: [[table:methods]]. The method of every workflow step: [[table:step-methods]].
