---
kind: workflow
id: multi
package: W12
sources: [pipeline.md#2.3, pipeline.md#2.5, pipeline.md#2.9.11, pipeline.md#2.9.13, BLUEPRINT.md#4.5, BLUEPRINT.md#5.1]
---

# `multi`

Coordinates a project that spans several repositories.

Arguments: `stage`, `repo`. Preconditions: the project layout is `hub` ([[table:preconditions]]).
This workflow defines the hub model, which [[topic:multi-repo]] and [[topic:multi-repo-state]]
summarise: one repository, the hub, holds the single state directory; its `factory.toml` has
`layout = "hub"` and one `[[repo]]` entry per member repository. Member repositories hold code and
nothing from the factory. There is one cycle, one identifier space and one status report. A story
belongs to one repository and carries `repo`; work that spans two repositories is two stories joined
by `depends_on`.

## Per-repository work

Per-repository work uses the ordinary workflows with `repo: "<name>"`, run from the hub. The survey
resolves the name to its `[[repo]]` entry, filters the units to stories with that `repo`, and returns
the member directory as `project_dir`; the wrapper puts it in every step prompt, and the hook runner
finds the hub's state by walking up from the path being touched. Release order follows story
dependencies.

## Stages in order

### survey

- Step and agent: clerk.
- Runs when: always. Produces: progress per repository, read from each story's `repo`; the order the
  story dependencies give; which project-level stages are due.

### split

- Step and agent: clerk running `vsdd repos split` ([[command:repos-split]]).
- Runs when: a `spec` decision chose `topology=split`: the tool creates the member repositories from
  the `repo` key of the architect's subsystem files and appends their `[[repo]]` entries. This is the
  one case in which code adds to `factory.toml` after `init`, and the next briefing announces it.

### synthesize

- Step and agent: architect (method `multi-repo-synthesis`) and product-owner (planner).
- Runs when: every existing repository is ingested and the project context is stale.

### contracts

- Step and agent: clerk running `vsdd gate contracts` ([[command:gate-contracts]]); generated client
  libraries are rebuilt through fix stories in their repositories.
- Runs when: a provider's published interface changed since its consumers were last built.

### integrate

- Step and agent: clerk running `vsdd gate integration` ([[command:gate-integration]]) — bring the
  services up together, cross-repository end-to-end tests; then adversary, security-reviewer and
  holdout-evaluator (reviewer) at the `release` perimeter; fixes through `story` with `repo` set to
  the owner.
- Runs when: the repositories of one dependency layer have closed their waves, and again before
  release.

### close

- Step and agent: clerk.
- Runs when: always. Produces: `next` is the next per-repository command, or decision `spec` or
  `release`.

## What is kept per repository

A cross-repository change request starts with `feature` at the hub; its delta analysis names the
affected repositories (`cross_repo`), and each gets its own story: through the plan on the standard
route, and on a fast route as one story per repository joined by `depends_on`, because a story
belongs to one repository.

Waves are project-wide (the plan's wave map covers every story), but a wave is built and gated
repository by repository, since each `wave` run takes the lock of one repository and branch. The
wave's closure record, its gate rounds and its integration worktree are therefore per repository:
`cycles/<cycle>/units/wave.<W-NN>.<repo>.json` and `.worktrees/_integration/<repo>/`. A wave is
closed when every repository that has stories in it has closed its part, and the survey's "every
earlier wave is closed" reads that. A `[[repo]]` entry may carry its own `[release]` and `[delivery]`
tables, so the hub release can ship each repository by its own rules, in the order story dependencies
give.

## Setting a hub up

`vsdd init` does not guess a hub. `/vsdd-factory:setup --hub` runs `vsdd repos scan`
([[command:repos-scan]]), shows the repositories it found, and passes the confirmed list to
`vsdd init --hub`, which writes the `[[repo]]` entries when it creates `factory.toml`; on a project
that is already initialised it shows the block for the person to paste. On a fresh clone of the hub,
`vsdd status --fix` clones each member repository that is missing from the `url` in its entry.

Child stories are started through `runStories()` with the shared reserve check
([[topic:prelude-run-stories]]). Ends: per-repository commands, or decisions `spec` and `release` at
project level. The return value is a workflow result ([[schema:workflow-result]]).
