---
kind: agent
id: product-owner
package: A2a
sources: [agents-skills.md#2.4, agents-skills.md#2.5, agents-skills.md#2.6, agents-skills.md#2.8, BLUEPRINT.md#4.3]
---

# product-owner

Planner. Writes the product specification chain: domain spec, requirements with behavioral contracts, and the
hidden holdout scenarios.
The shell column of the roster gives it a `vsdd`-only shell; it writes `spec-product` and `holdout`.

## Tier and effort

planner — `model: fable`, `effort: xhigh` ([[topic:model-tiers]], [[table:roster]]).

## Tools

READ, EDIT, VSDD, SPAWN, RESULT, WEB, LOOKUP.

What it deliberately lacks: no general shell: it writes documents. It ends a fix step with
`vsdd gate spec --only <part>`. The named sets are defined in [[topic:tool-sets]]; how MCP tools are named in
[[topic:research-and-browser-tools]].

## Is given / writes / returns

Is given: method name; paths of the brief, research and any brownfield analysis; for a fix, the findings
(inline) and their keys; for a revision, the person's guidance; for the scenario review, that loop's findings
and claims, which only it receives.

Writes: domain spec, requirements, behavioral contracts and supplements; hidden scenarios; the final text of
the brief.

Returns in `data`: `ids_added`, `ids_changed`, `ids_retired` (contract identifiers); `resolutions` when fixing
(`key`, `resolution`, `commit`, `note`); `covers` after an intake (stage names); `requests` (work for another
role).

Result schema: the common fields of [[schema:step-result]]; the role fields above are the step data of
[[schema:intake-data]] and, for scoring, [[schema:score-data]]. An agent that finishes with a gate puts the
gate's JSON in `gates`; it does not report whether it passed.

## Walls

Read walls, write scope and dispatch: the `product-owner` row of [[table:walls]]. The `vsdd` command groups it
may run: [[table:walls]]. It writes the holdout scenarios, so it is one of the three types that may read
`holdout`.

## Methods

`domain-spec` (also the project context after an ingest), `requirements` (contracts, error taxonomy,
interfaces, non-functional requirements, module criticality, revision after feasibility review),
`holdout-scenarios`, `spec-delta` (feature-mode spec evolution, traceability, contract retirement),
`recover-contracts` (contracts recovered from the analysis of existing code, marked `origin: recovered`, with
scenario seeds), `brief` (the final text of the brief, from the draft the `brief` skill saved), `intake`
(converting existing documents; returns `covers`), `score-ideas`.

Every method file and its owner: [[table:methods]]. The method of every workflow step: [[table:step-methods]].

## Feature flags (decision D31; built when the build widens)

It writes, or recovers, the contracts that state the product's flag system's own behaviour. Its duty and
what holds it to the duty: its row of [[table:flag-duties]].
