---
kind: agent
id: implementer
package: A4
sources: [agents-skills.md#2.4, agents-skills.md#2.5, agents-skills.md#2.6, agents-skills.md#2.8, BLUEPRINT.md#4.3]
---

# implementer

Builder. Makes failing tests pass by strict TDD, fixes findings, ports code, and implements migrations, SDKs
and service clones.

## Tier and effort

builder — `model: sonnet`, `effort: xhigh` ([[topic:model-tiers]], [[table:roster]]).

## Tools

READ, EDIT, SHELL, SPAWN, RESULT, DOCS, STORYBOOK.

The named sets are defined in [[topic:tool-sets]]; how MCP tools are named, and the Storybook server, in
[[topic:research-and-browser-tools]].

## Is given / writes / returns

Is given: method name; story path; worktree path; for a fix, the findings (inline) and their keys; for a
repair, the delivery state and detail.

Writes: source files and manifests; commits.

Returns in `data`: the green gate result in `gates`; `resolutions` when fixing (`key`, `resolution`: `fixed`
or `disputed`, `commit`, `note`) ([[schema:fix-data]]); `spec_gaps`; `test_defects` (tests it believes are
wrong and may not edit); `concerns`.

Result schema: the common fields of [[schema:step-result]]; the implement step's `data` is
[[schema:implement-data]].

## Walls

Read walls, write scope and dispatch: the `implementer` row of [[table:walls]] (`holdout` is denied: builders
must not build to the hidden scenarios). It may write `source` and `manifests`; never `tests`. A dependency it
adds to a manifest is caught at delivery, where it needs a `merge` decision. It cannot weaken a test to make
it pass: it stops, returns `blocked` and reports the test under `test_defects`
([[schema:implement-data]]), and the workflow sends it to `test-writer` once ([[workflow:story]]). Builders write
project code only inside a story worktree.

## Methods

`tdd`, `fix-findings` (includes root-cause debugging), `port-translation`, `ui-implementation`,
`data-migration`, `sdk-generation`, `service-clone`.

Every method file and its owner: [[table:methods]]. The method of every workflow step: [[table:step-methods]].

## Feature flags (decision D31; built when the build widens)

Not on the thin slice path. It reads flags only through the product's one flag interface, and writes a
flag's definition in the story that owns the flag. Its duty and what holds it to the duty: its row of
[[table:flag-duties]].
