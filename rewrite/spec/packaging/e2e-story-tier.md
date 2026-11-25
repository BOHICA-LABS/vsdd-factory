---
kind: packaging
id: e2e-story-tier
package: E2
sources: [packaging.md#layer-4-end-to-end-headless, BLUEPRINT.md#9.6, BLUEPRINT.md#5.5]
---

# The story tier

Runs in the release workflow, weekly, and on demand. From the `story-ready` fixture, one story is delivered ("add
`median()`"). Then the same run is killed three ways and started again as a new process with the same command:

- at a step boundary;
- in the middle of a step (the worktree holds uncommitted work);
- in the middle of a CI wait (a pull request is open).

The second process must not review a converged story again.

The tier also carries an eleven-minute gate: the probe for the platform fact that a Bash call inside a workflow step
accepts a ten-minute timeout and a detached job outlives the call that started it ([[doc:platform]], item 6;
"The full ten minutes is tested in E2").

## What the story tier asserts

One group per design goal:

- **Enforceable:** the failing-test commit precedes the implementation commit; the merged commit is the one the
  reviewer approved; no builder-tier agent read a holdout path.
- **Low touch:** the run reached its first designed human stop with no input.
- **Restartable:** after the kill, the second process finishes; the end state satisfies the same assertions; and it
  starts fewer agents than a full run, counted from the event log.
- **Lean:** agent starts and total cost are recorded and fail the run above a budget; every state file is under its
  cap.

## The budget

The budget is 12 agent starts for a story until package E2 has measured a real figure ([[doc:platform]], items 6
and 22). The pipeline design's planning figure is 8 to 12 starts for a realistic story and 70 to 90 for a six-story
wave; its "4 to 6" is the floor of a simulation in which every step succeeded first time, and a budget of 1.5 times
that would fail ordinary runs.

Delivered by package E2 ([[package:E2]]): story delivery, kill and resume, the budgets, `e2e.yml`
([[packaging:ci]]). The tier is a model-driven gate in the release workflow ([[packaging:plugin-release]]).
