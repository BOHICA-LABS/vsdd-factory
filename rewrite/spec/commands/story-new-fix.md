---
kind: command
id: story-new-fix
package: G4b
sources: [BLUEPRINT.md#4.9, state.md#2.16, BLUEPRINT.md#4.6]
---

# `vsdd story new-fix`

Writes fix stories in E-0 from blocking findings and admits each with its worktree.

## Syntax

`vsdd story new-fix --wave <id>`, `--verify` or `--maintain`, `--round <k>`, `--run <id>`

## What it does

"Fix stories in `E-0` from blocking findings, each admitted with its worktree as `gate plan --story` does."
Admission is the tool's, not the script's: "a story that does not come from the plan (the one story of a
fast route, a fix story from a gate, a readiness chore) is admitted by the tool ... check[s] the story,
record[s] it in the run and the cycle, create[s] its branch and worktree under the run's lock, and return[s]
the story unit the child `story` workflow needs" ([[topic:story-admission]], [[table:story-kinds]]).

Modes, from the workflows that call it: in `wave`, the fix round inside the gate loop ("Then
`story` as a child for each fix story"); in `verify`, "Blocking findings become fix stories
(`vsdd story new-fix --verify`, which admits each)"; in `maintain`, "writes fix stories for the fixable
findings, at most ten per run, admits each". "With `--maintain` and no open cycle, a maintenance cycle and
its request" — "the tool opens a maintenance cycle (kind `maintenance`, its request written by code from the
sweep findings)" ([[record:cycle]]).

A failing [[command:gate-wave]] carries the fix units in its own output, "so no separate `new-fix` clerk is
needed in that case".

## Who may call it

`clerk`.

## Writes

Fix story files, run-record and closure admission entries, branch and worktree per fix story
([[record:run]], [[record:closure]], [[topic:worktrees]]); with `--maintain`, a maintenance cycle and its
request.

## Output and failure cases

Units ([[schema:story-unit]]); a reserve check applies: "a fix round that does not fit beside the reserve
for the close returns `partial`."

## Feature flags (decision D31; built when the build widens)

With `--maintain` it also writes the cleanup story for a flag whose removal is due, once a person has
approved the removal where the project's setting asks for that ([[topic:flag-lifecycle]]; F-REMOVE in
[[table:flag-release-checks]]).

## Team use (decision D33; built when the build widens)

Each fix story gets its id inside a sync and is claimed when it is admitted
([[table:work-claim-checks]], T-MINT, T-CLAIM).
