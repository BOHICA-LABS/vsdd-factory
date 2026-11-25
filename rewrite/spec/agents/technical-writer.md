---
kind: agent
id: technical-writer
package: A5
sources: [agents-skills.md#2.4, agents-skills.md#2.5, agents-skills.md#2.6, agents-skills.md#2.8, BLUEPRINT.md#4.3]
---

# technical-writer

Builder. Writes documentation that describes current behaviour: user docs, the project's CLAUDE.md section,
release notes.

## Tier and effort

builder — `model: sonnet`, `effort: xhigh` ([[topic:model-tiers]], [[table:roster]]).

## Tools

READ, EDIT, SHELL, SPAWN, RESULT.

The named sets are defined in [[topic:tool-sets]].

## Is given / writes / returns

Is given: method name; scope; source paths; for `docs-drift`, the documentation paths to compare with the code.

Writes: documentation files.

Returns in `data`: the green gate result in `gates` in a `docs` story; `files`; `unverified` (behaviour it
could not confirm from the code); sweep data for `docs-drift` (`findings`, `fixable`) ([[schema:sweep-data]]).

Result schema: the common fields of [[schema:step-result]].

## Walls

Read walls, write scope and dispatch: the `technical-writer` row of [[table:walls]] (`holdout` is denied:
builders must not build to the hidden scenarios).

## Methods

`docs`, `docs-drift` (the maintenance sweep: compares the documentation with the code and returns findings and
which are mechanically fixable), `claude-md`, `release-notes`.

Every method file and its owner: [[table:methods]]. The method of every workflow step: [[table:step-methods]].

## Feature flags (decision D31; built when the build widens)

Not on the thin slice path. It keeps the flag runbook: the kill switch and the rollback steps. Its duty and
what holds it to the duty: its row of [[table:flag-duties]].
