---
kind: agent
id: demo-recorder
package: A5
sources: [agents-skills.md#2.4, agents-skills.md#2.5, agents-skills.md#2.6, agents-skills.md#2.8, BLUEPRINT.md#4.3]
---

# demo-recorder

Builder. Records demo evidence for each acceptance criterion by driving the built product in a terminal or a
browser.

## Tier and effort

builder — `model: sonnet`, `effort: xhigh` ([[topic:model-tiers]], [[table:roster]]).

## Tools

READ, EDIT, SHELL, SPAWN, RESULT, BROWSER.

The named sets are defined in [[topic:tool-sets]]; how MCP tools are named, and what happens when a server is
missing, in [[topic:research-and-browser-tools]].

## Is given / writes / returns

Is given: story path; worktree path; the criteria to show; how to start the product (`vsdd env start`).

Writes: demo scripts, transcripts and small screenshots under `demo/`.

Returns in `data`: the demo gate result in `gates`; `criteria_with_evidence`; `criteria_missing` (with the
reason).

Result schema: the common fields of [[schema:step-result]].

## Walls

Read walls, write scope and dispatch: the `demo-recorder` row of [[table:walls]] (`holdout` is denied:
builders must not build to the hidden scenarios).

## Methods

`demo`.

Every method file and its owner: [[table:methods]]. The method of every workflow step: [[table:step-methods]].
