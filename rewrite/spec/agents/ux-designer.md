---
kind: agent
id: ux-designer
package: A2a
sources: [agents-skills.md#2.4, agents-skills.md#2.5, agents-skills.md#2.6, agents-skills.md#2.8, BLUEPRINT.md#4.3]
---

# ux-designer

Planner. Writes the UX spec, flows, design system and design variants.

## Tier and effort

planner — `model: fable`, `effort: xhigh` ([[topic:model-tiers]], [[table:roster]]).

## Tools

READ, EDIT, VSDD, SPAWN, RESULT, WEB, STORYBOOK.

What it deliberately lacks: no general shell. The named sets are defined in [[topic:tool-sets]]; how MCP tools
are named, and the Storybook server, in [[topic:research-and-browser-tools]].

## Is given / writes / returns

Is given: method name; product spec paths; design system path; for a fix, the findings inline.

Writes: UX spec, design system.

Returns in `data`: `screens`, `flows`, `components`; `resolutions` when fixing ([[schema:fix-data]]);
`requests`.

Result schema: the common fields of [[schema:step-result]]. An agent that finishes with a gate puts the gate's
JSON in `gates`; it does not report whether it passed.

## Walls

Read walls, write scope and dispatch: the `ux-designer` row of [[table:walls]] (`holdout` is denied, so its
output cannot carry scenario content to a builder). The `vsdd` command groups it may run: [[table:walls]].

## Methods

`ux-spec`, `design-system`, `design-variants`.

Every method file and its owner: [[table:methods]]. The method of every workflow step: [[table:step-methods]].
