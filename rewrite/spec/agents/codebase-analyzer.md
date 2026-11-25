---
kind: agent
id: codebase-analyzer
package: A3
sources: [agents-skills.md#2.4, agents-skills.md#2.5, agents-skills.md#2.6, agents-skills.md#2.8, BLUEPRINT.md#4.3]
---

# codebase-analyzer

Planner. Reads an existing codebase and extracts its behaviour, architecture and conventions.

## Tier and effort

planner — `model: fable`, `effort: xhigh` ([[topic:model-tiers]], [[table:roster]]). One of the three roles
raised as a cost judgment call under the user's policy (open question 1 of the design).

## Tools

READ, EDIT, SHELL, SPAWN, RESULT, DOCS.

What it deliberately lacks: no web search: its claims must come from the code. The named sets are defined in
[[topic:tool-sets]]; how MCP tools are named in [[topic:research-and-browser-tools]].

## Is given / writes / returns

Is given: method name; codebase path; the module or the targets, verbatim from the workflow; paths of earlier
output.

Writes: analysis files.

Returns in `data`: module-map data for `ingest-broad` (`modules`) ([[schema:module-map-data]]);
`files_covered`, `files_total`; `low_confidence` (areas it could not establish).

Result schema: the common fields of [[schema:step-result]].

## Walls

Read walls, write scope and dispatch: the `codebase-analyzer` row of [[table:walls]] (`holdout` is denied, so
its output cannot carry scenario content to a builder). The `vsdd` command groups it may run:
[[table:walls]].

## Methods

`ingest-broad` (the module map), `ingest-deepen` (per-module analysis, and corrections after validation),
`ingest-synthesis`, `port-analysis` (the old semport analysis), `disposition`.

Every method file and its owner: [[table:methods]]. The method of every workflow step: [[table:step-methods]].

## Feature flags (decision D31; built when the build widens)

Each module's analysis says whether the module uses feature flags, so that an existing flag system is kept
and not doubled. Its duty and what holds it to the duty: its row of [[table:flag-duties]].
