---
kind: command
id: gate-wave
package: G6a
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.9.2]
---

# `vsdd gate wave`

Runs every due wave check on the integration head as one job: suite, lint, build, mutation, clone fidelity
and UI.

## Syntax

`vsdd gate wave --wave <id> [--settle] [--repo <name>] --run <id>`. A job command
([[topic:long-commands]]).

## What it does

"Runs the suite, lint and build on the integration head in a worktree the tool owns
(`.worktrees/_integration/`), so the person's own checkout is never switched. It also runs mutation testing
for stories that carry the requirement and the fidelity check for service clones the wave touched." The
checks run "in parallel inside the tool"; the check list is in [[topic:gates]]. "With `--settle` it first
moves stories that wait on a person to a later wave, together with their dependents, when no delivered
story depends on them, creating a trailing wave if this is the last one; it reports any story that still
blocks the gate in `pending`. When the measured gate fails, its output already carries the fix units, so no
separate `new-fix` clerk is needed in that case."

The wave's fix rounds come through [[command:story-new-fix]]; "a finding counts as fixed when its fix story
was delivered". In a multi-repository project "the gate, its rounds and its integration worktree are per
repository" ([[topic:multi-repo]]). "A wave-level demo is no longer part of the gate. It can be switched on
in project configuration."

## Who may call it

`clerk` ("integration-r<k>: clerk running `vsdd gate wave`").

## Writes

"Evidence; the integration worktree (per repository in a hub); closure rounds for `wave`"
([[record:evidence]], [[record:closure]]). The wave gate data feeds the convergence loop's `wave` perimeter
([[schema:wave-gate-data]]).

## Output and failure cases

"Gate result; wave gate data for `wave`" ([[schema:gate-result]]). Measured gates fail only on what the
baseline does not list ([[record:baseline]]). "When everything left in the wave waits on decisions the run
returns `needs_human` and the route names them"; a wave whose own gate waits on a decision is not closed,
and no later wave starts.

## Feature flags (decision D31; built when the build widens)

The job gains one check, `flags`: the rows of [[table:flag-story-checks]] whose `form` names this job.

## Team use (decision D33; built when the build widens)

Not on the thin slice path. Wherever the target branch has an upstream, with team use on or off, the
gate lists the hand commits in the wave's range as `hand_commits`, each with its patch, and fails with
`history_rewritten` or `history_unmeasured` when it cannot measure that range. With team use on it puts
a story another clone holds in `pending` ([[table:team-delivery-checks]], T-NOTICE, T-WAVE).
