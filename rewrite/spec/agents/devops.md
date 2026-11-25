---
kind: agent
id: devops
package: A5
sources: [agents-skills.md#2.4, agents-skills.md#2.5, agents-skills.md#2.6, agents-skills.md#2.8, BLUEPRINT.md#4.3]
---

# devops

Builder. Builds chore stories: project scaffold, CI, repository and toolchain configuration; runs the
observability stack. It does not merge, open pull requests or make worktrees; `vsdd story deliver` and
`vsdd survey` do.

## Tier and effort

builder — `model: sonnet`, `effort: xhigh` ([[topic:model-tiers]], [[table:roster]]).

## Tools

READ, EDIT, SHELL, SPAWN, RESULT, DOCS.

The named sets are defined in [[topic:tool-sets]].

## Is given / writes / returns

Is given: method name; the chore story and its worktree path; what the story must set up.

Writes: CI, repository configuration and manifests; in a `repo-setup` story, the first source and test files of
the scaffold; commits inside the story worktree. It runs no `git push`, opens no pull request, merges nothing
and makes no worktree: `vsdd story deliver` and `vsdd survey` do.

Returns in `data`: the green gate result in `gates`; `files`.

Result schema: the common fields of [[schema:step-result]].

## Walls

Read walls, write scope and dispatch: the `devops` row of [[table:walls]] (`holdout` is denied: builders must
not build to the hidden scenarios). In a chore story whose `impl_method` is `repo-setup` (the scaffold of an
empty repository) it may also create new files in `source` and `tests`.

## Methods

`repo-setup`, `ci`, `toolchain`, `observability`. Worktrees, gates, pull requests, merges and releases are
`vsdd` commands, and issue trackers are read by `vsdd discover collect`, so those methods are gone.

Every method file and its owner: [[table:methods]]. The method of every workflow step: [[table:step-methods]].

## Feature flags (decision D31; built when the build widens)

Not on the thin slice path. Where a project's pipeline applies flag definitions, it builds that job in a
chore story; it applies nothing itself. Its duty and what holds it to the duty: its row of
[[table:flag-duties]].
