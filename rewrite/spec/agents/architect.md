---
kind: agent
id: architect
package: A2b
sources: [agents-skills.md#2.4, agents-skills.md#2.5, agents-skills.md#2.6, agents-skills.md#2.8, BLUEPRINT.md#4.3]
---

# architect

Planner. Designs the architecture and records decisions, verification properties, service-clone and port
assessments, data models, and the impact of a change.

## Tier and effort

planner — `model: fable`, `effort: xhigh` ([[topic:model-tiers]], [[table:roster]]).

## Tools

READ, EDIT, VSDD, SPAWN, RESULT, WEB, LOOKUP, DOCS.

What it deliberately lacks: no general shell: an experiment is a spike. It ends the delta step with
`vsdd feature route`, and a fix step with `vsdd gate spec --only <part>`. The named sets are defined in
[[topic:tool-sets]]; how MCP tools are named in [[topic:research-and-browser-tools]].

## Is given / writes / returns

Is given: method name; product spec paths; for delta analysis, the change request path; for a fix, the findings
inline; for arbitration, the open findings and the builder's dispute notes.

Writes: architecture sections, decision records, verification properties, assessments, `dependencies.toml`,
`toolchain.toml`, `conventions.md`, `review-rules.md`.

Returns in `data`: `decisions`, `properties`; `feasibility` (`ok`, `issues`); delta data for `delta-analysis`
([[schema:delta-data]]), with the route gate result in `gates`; `rulings` for `arbitrate` (`key`, `ruling`,
`reason`) ([[schema:ruling-data]]); `requests`; `clones_required`, `port_candidates`.

Result schema: the common fields of [[schema:step-result]].

## Walls

Read walls, write scope and dispatch: the `architect` row of [[table:walls]] (`holdout` is denied, so its
output cannot carry scenario content to a builder). The `vsdd` command groups it may run: [[table:walls]].

## Methods

`architecture` (including `dependencies.toml` and `toolchain.toml`, and after an ingest the recovered
architecture), `decision-record`, `verification-properties`, `feasibility`, `clone-assessment` (the old DTU
assessment), `port-assessment` (the old gene-transfusion assessment), `verification-assessment` (the
verification gap analysis of existing code), `conventions` (`conventions.md` and `review-rules.md`: writing
them after an ingest, moving build conventions out of a project's `CLAUDE.md`, and adopting a retrospective's
proposal), `delta-analysis` (ends with `vsdd feature route`), `data-model`, `multi-repo-synthesis`, `diagram`,
`arbitrate`, `intake`, `score-ideas`.

Every method file and its owner: [[table:methods]]. The method of every workflow step: [[table:step-methods]].

## Feature flags (decision D31; built when the build widens)

Not on the thin slice path. In `architecture` it proposes the product's flag system, which a person confirms
in the `spec` decision ([[topic:flag-system-choice]]). Its duty and what holds it to the duty: its row of
[[table:flag-duties]].
