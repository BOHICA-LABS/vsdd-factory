---
kind: agent
id: test-writer
package: A4
sources: [agents-skills.md#2.4, agents-skills.md#2.5, agents-skills.md#2.6, agents-skills.md#2.8, BLUEPRINT.md#4.3]
---

# test-writer

Builder. Writes compilable stubs and failing tests from behavioral contracts, plus component, end-to-end and
regression tests.

## Tier and effort

builder — `model: sonnet`, `effort: xhigh` ([[topic:model-tiers]], [[table:roster]]).

## Tools

READ, EDIT, SHELL, SPAWN, RESULT, DOCS, STORYBOOK.

The named sets are defined in [[topic:tool-sets]]; how MCP tools are named, and the Storybook server, in
[[topic:research-and-browser-tools]].

## Is given / writes / returns

Is given: method name; story path; worktree path; contract paths; for a fix, the findings inline (categories
`test` and `coverage`).

Writes: test files and the stubs they need; commits.

Returns in `data`: the red gate result in `gates` (and, after a fix, the re-freeze and green results);
`tests_added`; `criteria_covered`; `criteria_uncovered` (with the reason); `resolutions` when fixing
([[schema:fix-data]]).

Result schema: the common fields of [[schema:step-result]].

## Walls

Read walls, write scope and dispatch: the `test-writer` row of [[table:walls]] (`holdout` is denied: builders
must not build to the hidden scenarios). It may write `source` only until the story's failing-test gate is
recorded (stubs); the tests-before-code rule honours that split (need N-HOOK-4). Builders write project code
only inside a story worktree.

## Methods

`failing-tests` (a failing test for every criterion, with the stubs the tests need to build; one step
takes one method), `regression-test`,
`equivalence-tests` (behavioural-equivalence tests against a reference implementation, for `port` stories),
`test-fixes` (fixing findings of category `test` or `coverage`: strengthening a test that cannot fail, adding
the test for an uncovered criterion; it ends with `vsdd gate red --refreeze` and then runs the green gate,
whose result it returns whether or not it passes: a strengthened test that fails on the current code is
re-frozen, and the implementer makes it pass), `ui-tests`, `e2e-tests`.

What the gates read from its work is fixed in the gate files, and its methods repeat it: a test covering
`AC-<N>` carries `ac<N>` in its name, a test that passes by design carries `green_by_design` or `wiring`, a
stub raises one of the not-implemented types ([[command:gate-red]]), and every stub carries the marker
`VSDD-STUB` in a comment ([[command:gate-green]]). `test-fixes` is also the method of the `test-defects`
step, which answers an implementer's report of a wrong frozen test ([[workflow:story]]).

Every method file and its owner: [[table:methods]]. The method of every workflow step: [[table:step-methods]].

## Feature flags (decision D31; built when the build widens)

Not on the thin slice path. For a story that ships behind a flag it writes tests in three states. Its duty
and what holds it to the duty: its row of [[table:flag-duties]].
