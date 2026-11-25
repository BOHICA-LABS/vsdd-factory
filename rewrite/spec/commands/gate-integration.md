---
kind: command
id: gate-integration
package: G7
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.9.11]
---

# `vsdd gate integration`

The multi-repository integration gate: brings the services up together and runs cross-repository
end-to-end tests.

## Syntax

`vsdd gate integration --run <id>`. A job command ([[topic:long-commands]]).

## What it does

The `integrate` stage of the `multi` workflow: "clerk running `vsdd gate integration` (bring the services up
together, cross-repository end-to-end tests); then adversary, security-reviewer and holdout-evaluator
(reviewer) at the `release` perimeter; fixes through `story` with `repo` set to the owner. Runs when: The
repositories of one dependency layer have closed their waves, and again before release."

## Who may call it

`clerk` (the `integrate` stage of `multi`).

## Writes

Evidence ([[record:evidence]]); the integration worktree per repository in a hub
(`.worktrees/_integration/<repo>/`, [[topic:worktrees]]).

## Output and failure cases

"Gate result" ([[schema:gate-result]]); review findings at the `release` perimeter flow into the
convergence loop, with fixes through [[command:story-new-fix]].
