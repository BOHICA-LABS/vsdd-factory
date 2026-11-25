---
kind: workflow
id: wave
package: W3
sources: [pipeline.md#2.3, pipeline.md#2.5, pipeline.md#2.9.2, pipeline.md#2.9.13, BLUEPRINT.md#4.5, BLUEPRINT.md#5.1]
---

# `wave`

Builds one wave: delivers its stories in parallel, then runs the wave gate to convergence.

Arguments: `wave` (default: the next open wave), `repo`. Preconditions: the plan is ready; the
baseline check passed; every earlier wave is closed; `[commands]` is configured
([[table:preconditions]]). Script skeleton: [[topic:wave-script]].

## Stages in order

### survey

- Step and agent: `survey`, clerk.
- Given: wave id or none.
- Returns: survey; each story of the wave with its position, worktree, criticality, flags and any
  settled decision.
- Gate: preconditions — the specification is approved, the plan is ready, the baseline check passed,
  every earlier wave is closed, `[commands]` is configured.

### stories

- Step and agent: `story` as a child through `runStories()` ([[topic:prelude-run-stories]]), one per
  story that is not delivered, deferred or waiting on an open decision, all in parallel.
- Given: the unit from the survey.
- Returns: story result.
- Gate: none here. If the run could not start every story (story or agent limit), checkpoint and
  return `partial`.

### gate

- Step and agent: `integration-r<k>`: clerk running `vsdd gate wave`
  ([[command:gate-wave]]), which for a UI wave runs the UI check as part of the same job. Then in
  parallel: `integration-review-r<k>`, adversary (reviewer); `security-r<k>`, security-reviewer
  (reviewer), when the wave has critical, high or security stories; `holdout-r<k>-b<i>`,
  holdout-evaluator (reviewer), one per batch of up to `holdout.batch` (12) scenarios; for UI waves
  `ux-review-r<k>`, ui-reviewer (reviewer).
- Given: integration worktree, commit range of the wave, story ids, the gate's `diff_patch`, findings
  deferred by the story reviews, claims.
- Returns: wave gate data; review data; holdout data.
- Gate: `converge()` at the `wave` perimeter returns `converged` ([[topic:convergence]]).

### fix (inside the gate loop)

- Step and agent: `new-fix-r<k>`: clerk running `vsdd story new-fix`
  ([[command:story-new-fix]]), which writes the fix stories and admits each. Then `story` as a child
  for each fix story, through `runStories()` and its reserve check.
- Given: the round number.
- Returns: units; story results.
- Gate: a finding counts as fixed when its fix story was delivered.

### close

- Step and agent: `close`, clerk.
- Given: run id.
- Returns: close.
- Gate: the command closes the wave only if the evidence supports it.

## Details

- `vsdd gate wave` runs the suite, lint and build on the integration head in a worktree the tool owns
  (`.worktrees/_integration/`), so the person's own checkout is never switched. It also runs mutation
  testing for stories that carry the requirement and the fidelity check for service clones the wave
  touched. With `--settle` it first moves stories that wait on a person to a later wave, together
  with their dependents, when no delivered story depends on them, creating a trailing wave if this is
  the last one; it reports any story that still blocks the gate in `pending`. When the measured gate
  fails, its output already carries the fix units, so no separate `new-fix` clerk is needed in that
  case. When everything left in the wave waits on decisions the run returns `needs_human` and the
  route names them.
- From the second round, reviews are scoped to the fix commits and the claims, and scenarios are
  re-run only if a fix changed product source.
- Scenario scores become findings in the shape [[topic:holdout-findings]] fixes: one high finding per
  must-pass scenario under its floor (key `holdout:<HS id>`), and one aggregate high finding if the
  mean or the spread misses its threshold (key `holdout:<wave>:mean` or `:spread`), with no location
  and exempt from grounding. The evaluator's text for a failing scenario describes the symptom and
  must not quote the scenario; the tool checks that before a fix story is written from it.
- A wave-level demo is not part of the gate. It can be switched on in project configuration.
- In a multi-repository project the gate, its rounds and its integration worktree are per repository
  ([[topic:multi-repo]]).

## Waves and open decisions

One rule, written here, in the route table ([[table:routes]]) and in [[topic:human-stops]].

- A story that waits on its own decision (`finding.<story>`, `merge.<story>`, `spec-gap`,
  `blocked.<story>`) does not hold its wave: `--settle` moves it, with its dependents, to a later or
  trailing wave, the wave gates and closes without it, and the next wave starts with every story that
  does not depend on it.
- A wave whose own gate waits on a decision (`finding.<wave>`, opened when the gate loop reached its
  cap) is not closed, and no later wave starts, because nothing more should be built on an
  integration head that has failed its gate. The route's first answer is then "human" with that
  decision; it does not return `wave` for a survey that would refuse.

The survey precondition "every earlier wave is closed" and the route's "could run" test are one
function. A scenario test holds the second case: a wave gate at its cap gives a route of "human" with
that decision on the first try.

## Ends

`done` (next is the next wave or `verify`); `partial`; or `needs_human` with decisions queued. The
return value is a workflow result ([[schema:workflow-result]]); the last step is a clerk running
`vsdd checkpoint wave --run <id>` ([[command:checkpoint]]).

## Feature flags (decision D31; built when the build widens)

`vsdd gate wave` runs one more check, `flags` ([[command:gate-wave]]), and the product the reviewers and
the evaluator are given is started with the live temporary flags on ([[topic:flag-lifecycle]]).

## Team use (decision D33; built when the build widens)

Not on the thin slice path. The stories of one wave can be held by different clones. This run claims
the free ones it can start and leaves the others free, returns `partial` when only stories held
elsewhere are left, and runs the gate once they are in. The gate's review accounts for each hand commit
in the wave's range ([[table:team-delivery-checks]], T-WAVE, T-ITEMS).
