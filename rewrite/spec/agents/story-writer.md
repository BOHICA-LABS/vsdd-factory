---
kind: agent
id: story-writer
package: A2b
sources: [agents-skills.md#2.4, agents-skills.md#2.5, agents-skills.md#2.6, agents-skills.md#2.8, BLUEPRINT.md#4.3]
---

# story-writer

Planner. Splits specs into epics and stories with traced acceptance criteria and dependencies. Waves are
computed by `vsdd plan schedule`.

## Tier and effort

planner — `model: fable`, `effort: xhigh` ([[topic:model-tiers]], [[table:roster]]).

## Tools

READ, EDIT, VSDD, SPAWN, RESULT.

What it deliberately lacks: no web: stories derive from the specs only. It ends a story draft with
`vsdd gate plan --story <id>`, which admits the story when it passes. The named sets are defined in
[[topic:tool-sets]].

## Is given / writes / returns

Is given: method name; spec paths; existing stories; for a fast route, the delta record path and the readiness
stories to write; for a fix, the findings inline.

Writes: epics and stories. It writes `depends_on` and `priority`; it writes no wave plan, because
`vsdd plan schedule` computes the waves.

Returns in `data`: outline data for `decompose` (`epics` with `stories`) ([[schema:outline-data]]);
`stories` (identifiers); `oversized` (stories it could not split); `untraced` (contracts no story covers); on
a fast route, where it ran `vsdd gate plan --story` once for each story it wrote, those gate results in
`gates` and `units`, the admitted story unit of each story, copied from its gate result's `unit` (required by
the step's data schema, [[schema:story-unit]]).

Result schema: the common fields of [[schema:step-result]].

## Walls

Read walls, write scope and dispatch: the `story-writer` row of [[table:walls]] (`holdout` is denied, so its
output cannot carry scenario content to a builder). The `vsdd` command groups it may run: [[table:walls]].

## Methods

`decompose`, `story` (create, refine, split; the completeness checklist; on a fast route it ends with
`vsdd gate plan --story`, and it writes the readiness chore stories the route lists and one story per
repository touched). There is no `wave-plan`: the story-writer writes `depends_on` and `priority`, and
`vsdd plan schedule` computes the waves.

Every method file and its owner: [[table:methods]]. The method of every workflow step: [[table:step-methods]].

## Feature flags (decision D31; built when the build widens)

It applies the setting `flags.require` to each story and writes the flag records ([[artifact:flag]]). Its
duty and what holds it to the duty: its row of [[table:flag-duties]].
