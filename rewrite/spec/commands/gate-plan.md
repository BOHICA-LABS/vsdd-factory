---
kind: command
id: gate-plan
package: G5
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.9.5, pipeline.md#2.9.4, BLUEPRINT.md#4.6]
---

# `vsdd gate plan`

Checks the plan; with `--story` checks one story and, when it passes, admits it to the run.

## Syntax

`vsdd gate plan [--story <id> --run <id>]`

## What it does

The plan gate, "checked in code": "every contract the cycle added or changed is covered by a story
criterion (on a first release that is every contract; on a project with a baseline it is the cycle's delta,
so recovered or already-built contracts need no new story), every criterion traces to a clause, the graph
has no cycle, no story is over the size limit, no placeholder text, every wave has scenarios or a recorded
reason." On a fast route it refuses "a story whose criteria cite a contract outside the approved set", and
the green gate fails a `trivial`-route story whose diff touches a critical module.

Admission ([[topic:story-admission]]). "With `--story` and a pass it admits the story: records it in the
run record and in a closure (`admitted_by` this run), creates `story/<id>` [...] and the
worktree with the worktree library, under the run's lock. Run again for a story it has admitted, it changes
nothing and returns the same unit" ([[record:run]], [[record:closure]]). The commit the branch is cut from,
and where it is recorded, is the base of [[topic:worktrees]]. A parent may
pass only a unit the tool returned to this run: "every gate and `vsdd story deliver` check that the story is
recorded in the run they are given, that its worktree exists at the recorded path and that its branch
descends from the recorded base, and refuse otherwise."

## Who may call it

"`clerk`; the `story-writer` for `--story`." On a fast route the story-writer's closing command is
`vsdd gate plan --story <id> --run <id>` for each story it wrote, at most 2 attempts.

## Writes

Evidence; with `--story` and a pass, the run record, the admission closure, the branch `story/<id>` and
the worktree.

## Output and failure cases

"Gate result whose findings carry `owner`; with `--story`, also `unit` when it passes"
([[schema:gate-result]], the unit of the story — [[schema:story-unit]]). "A story that fails is named in the
retry note; if one still fails after the last attempt the step is blocked with kind `gate`, and no story is
delivered while one is missing."

## Feature flags (decision D31; built when the build widens)

The gate also applies F-REQUIRE, F-RECORD and F-READY of [[table:flag-checks]], for the whole plan and
with `--story`, so a story cannot be admitted against the project's flag setting.

## Team use (decision D33; built when the build widens)

With `--story`, admission also claims the story ([[table:work-claim-checks]], T-CLAIM).
