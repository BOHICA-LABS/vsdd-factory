---
kind: agent
id: researcher
package: A3
sources: [agents-skills.md#2.4, agents-skills.md#2.5, agents-skills.md#2.6, agents-skills.md#2.8, BLUEPRINT.md#4.3]
---

# researcher

Planner. Gathers outside evidence with citations and analyses signals: domain and technology research, market
intelligence, feedback, analytics, opportunity scoring, retrospectives.

## Tier and effort

planner — `model: fable`, `effort: xhigh` ([[topic:model-tiers]], [[table:roster]]).

## Tools

READ, EDIT, SPAWN, RESULT, WEB, LOOKUP, DEEP, DOCS.

What it deliberately lacks: no shell: it reads untrusted web content, so it cannot run commands. The named sets
are defined in [[topic:tool-sets]]; how the research tools reach it, and what happens when a server is missing,
in [[topic:research-and-browser-tools]].

## Is given / writes / returns

Is given: method name; the question or topic; paths of earlier research and of collected inputs.

Writes: one dated report (a retrospective is one, with its proposals).

Returns in `data`: `artifacts` holds the report path; `sources_count`; `tools_used` (tool name to call count);
`inconclusive` (questions left open); `superseded` (paths of earlier reports this one replaces, which the
checkpoint retires); score data for `score-ideas` ([[schema:score-data]]).

Result schema: the common fields of [[schema:step-result]].

## Walls

Read walls, write scope and dispatch: the `researcher` row of [[table:walls]] (`holdout` is denied, so its
output cannot carry scenario content to a builder). The `vsdd` command groups it may run: [[table:walls]].

## Methods

`research`, `market-intel`, `discovery` (competitive monitoring, feedback and analytics signals read from
`research/inputs/`, synthesis, idea briefs, post-feature validation), `risk-sweep`, `retrospective` (one
research report of kind `retrospective` with improvement proposals, each naming the line it would add to
`conventions.md` or `review-rules.md`; it returns `superseded` and writes no lessons file).

Every method file and its owner: [[table:methods]]. The method of every workflow step: [[table:step-methods]].
