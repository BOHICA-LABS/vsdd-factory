---
kind: workflow
id: story
package: W2
sources: [pipeline.md#2.3, pipeline.md#2.5, pipeline.md#2.9.1, pipeline.md#2.9.13, BLUEPRINT.md#4.5, BLUEPRINT.md#4.6, BLUEPRINT.md#5.1]
---

# `story`

The one per-story playbook: failing tests, implementation, review to convergence, evidence, merge.

Arguments: `story` (required), `repo`. When called by a parent it also receives `position` (the story
unit), `run_id`, `project_dir` and `limits` (with `agents` set to `story_agents`, and the child's
share of spikes), and skips its own survey and checkpoint. Nesting rule: `wave`, `feature`, `verify`,
`maintain` and `multi` may call `story`. Nothing else is ever called as a child, and `story` contains
no `workflow()` call. A static test enforces both. Only `story` is ever called by another workflow,
and it calls none.

Preconditions (standalone): the specification is approved; the story is admitted (it is in an open
wave, or a fast route or a fix round admitted it); its dependencies are delivered; `[commands]` is
configured. See [[table:preconditions]].

Where a child's unit comes from. A story that was planned gets its unit from the parent's survey. A
story written after the survey (the one story of a fast route, a readiness chore, a fix story from a
gate) gets it from the command that admits it: `vsdd gate plan --story <id> --run <id>` for a story
the story-writer just wrote, and `vsdd story new-fix` (or the `fix_units` of a failing
`vsdd gate wave`) for fix stories. Admission checks the story, records it in the run and in the
cycle's closure records, creates the branch `story/<id>` from its base ([[topic:worktrees]]) and the
worktree, all under the parent's lock, and returns the full story unit of [[schema:story-unit]]. A parent may pass only a unit the tool returned to this run:
every gate and `vsdd story deliver` check that the story is recorded in the run they are given, that
its worktree exists at the recorded path and that its branch descends from the recorded base, and
refuse otherwise. A garbled unit therefore stops at the first gate and delivers nothing. See
[[topic:story-admission]].

## Stages in order

Position order: `ready`, `red`, `green`, `reviewed`, `evidenced`, `delivered`; what puts a story at each
is defined in [[topic:story-life]]. Each stage runs when the position is before the named position. The
script skeleton is [[topic:story-script]].

### survey (standalone only)

- Step and agent: `survey`, clerk.
- Given: story id.
- Returns: survey.
- Gate: every precondition met — the specification is approved, the story is admitted, its
  dependencies are delivered, `[commands]` is configured.
- Checked again by: the command refuses otherwise.

### tests (when the position is before `red`)

- Step and agent: `tests`, test-writer (builder). Method by story kind, from [[table:step-methods]]:
  `failing-tests` (`feature`; it includes the stubs), `regression-test` (`fix`),
  `equivalence-tests` (`port`); `ui-tests` or `e2e-tests` when the story's `tests_method` key says so.
  No tests stage for `facade`, `docs`, `chore`, `harness`.
- Given: worktree, story file path, contract ids, the red-gate command.
- Returns: step result with the red gate.
- Gate: `r.status == "done" && g(r,"red").passed && g(r,"red").sha == r.head`. On failure one retry
  with the gate's failure text, then blocked.
- Checked again by hook: source files in the story worktree cannot be written until red evidence
  exists.

The red gate passes when the stubs build, the new tests fail on assertions and not on build errors,
and at least half of the non-exempt new tests are red. What counts as a new test, a red test and an
exempt test, where the base commit's report comes from, and what the gate freezes are in
[[command:gate-red]].

### implement (when the position is before `green`)

- Step and agent: `implement`, the agent and method the story's kind selects (implementer,
  technical-writer, devops or verifier; all builder), from [[table:story-kinds]].
- Given: as above, and the green-gate command.
- Returns: step result with the green gate.
- Gate: same test with `green`.
- Checked again by: hook (tests frozen at the red gate are read-only to builders) and tool (delivery
  needs green evidence for the head commit).
- A wrong frozen test. The implementer may not edit a frozen test. When it finds one wrong it returns
  `blocked` and lists the test in `test_defects` ([[schema:implement-data]]). The script then runs one
  test-writer step, `test-defects`, with the method `test-fixes`, ending with `vsdd gate red --refreeze`,
  and the implement step once more with the test-writer's answer. A second `blocked` ends the run
  blocked. A harness scenario covers it.

The green gate passes when the configured build, full suite, lint and format commands pass on the head
commit, no stub marker remains, and the frozen tests are unchanged since the latest red or re-freeze
record, which must exist; the full condition is in [[command:gate-green]]. Its result carries
`diff_patch`, the path of the patch it wrote for the head.

Fixing a frozen test. A review finding of category `test` (for example "this test cannot fail") is the
test-writer's to fix, and the hook lets it change a frozen test. The green gate would then fail on
"frozen tests unchanged". So the test-writer's fix step ends with `vsdd gate red --refreeze --story <id>`,
which only the test-writer may run: it requires every changed frozen test to still exist and to either
pass or fail on an assertion, as the red gate does, and records the new hashes as a re-freeze record on
the head commit. A strengthened test that now fails has found the defect the weak one hid, so in the
same fix pass the implementer follows the test-writer and makes it pass, ending with the green gate.
The green gate compares against the latest red or re-freeze record. A harness scenario covers a
weak-test finding fixed this way, with a strengthened test that fails until the implementer changes
the code.

### review (when the position is before `reviewed`)

- Step and agent: `review-<lens>-l<n>-p<k>`: adversary, plus security-reviewer when the story's module
  is critical or high or the story is flagged security (reviewer). `fix-<who>-l<n>-p<k>`: the
  finding's `owner`, which is the test-writer for findings of category `test` or `coverage` and
  otherwise the implementer (builder); when a pass has both, the test-writer runs first, and the
  implementer also follows it when a re-frozen test fails on the current code. `arbitrate-l<n>`:
  architect (planner). `<n>` is the review loop (1 for the full diff, then one per delivery round) and
  `<k>` the pass; both are also in each step's identity ([[topic:verdicts]]).
- Given: the method, the tree root, the latest diff patch path (from the survey for the first pass,
  then the `diff_patch` of the last green gate result), the test output path, the review-rules path,
  story file, contract ids, the acceptance criteria to account for, the structured claims from the
  previous pass. A fixer is given the findings inline.
- Returns: review data; fix data; ruling data ([[schema:review-data]], [[schema:fix-data]],
  [[schema:ruling-data]]).
- Gate: `converge()` returns `converged` ([[topic:convergence]]).
- Checked again by tool: recomputes the verdict from the stored passes before it merges
  ([[topic:verdicts]]).

A pass with both test findings and code findings runs the test-writer first and the implementer
second. The implementer also runs when the pass had test findings only and a re-frozen test fails on
the current code.

### evidence (when the position is before `evidenced`, only when the story has user-visible acceptance criteria)

- Step and agent: `evidence`, demo-recorder (builder).
- Given: story file, the criteria to show, the demo-gate command.
- Returns: step result with the demo gate.
- Gate: `g(r,"demo").passed`.
- Checked again by tool, at delivery.

For a story with no user-visible criteria the demo gate is satisfied by the tool itself: it builds the
table of acceptance criterion to test to result at delivery, with no agent.

### deliver (when the position is before `delivered`)

- Step and agent: `deliver-<n>`: clerk running `vsdd story deliver` ([[command:story-deliver]]).
  `repair-<n>`: implementer (builder). Then a scoped review of the new commits.
- Given: story id.
- Returns: delivery data ([[schema:delivery-data]]).
- Gate: `d.state == "merged"`.
- Checked again by: tool (the refusals of [[command:story-deliver]]) and hook (agents cannot merge
  or push to the integration branch any other way, [[rule:merge-guard]]).

`vsdd story deliver` does everything mechanical, as one detached job keyed by the story that a single
clerk waits on by running the same command again. What it checks before it merges, its states, and how
it merges with no remote are in [[command:story-deliver]]. The script repairs and reviews again on
`ci_failed`, `conflict` and `review_stale`, returns `needs_human` on `awaiting_merge`, and ends
`blocked` on every other state that is not `merged`.

### close (standalone only)

- Step and agent: `close`, clerk.
- Given: run id.
- Returns: close.

## Details

- The verdict and the green evidence are bound to the head commit and to the `git patch-id` of the
  story's diff. After a clean rebase the patch id is unchanged, so both still hold and the tool runs
  the build command only. After a repair, or when a rebase changed the patch, the review loop runs
  once more with a cap of 2 over only the new commits.
- Story kinds change single stages, not the playbook ([[table:story-kinds]]). A story's optional
  `impl_method` key picks another method of the implement agent, for example `data-migration`, and its
  optional `tests_method` key another method of the test-writer for the tests stage.
- Review findings beyond a story's reach are returned to the parent in `deferred`.
- Delivery never force-pushes the integration or release branch. After a rebase it pushes the story's
  own branch with `--force-with-lease`. It never skips git hooks, and its commits carry no AI
  attribution.

## Where it stops for a person

`awaiting_merge`: the delivery data carries the pull request link and the checkpoint opens a `merge`
decision; once it is approved the next `vsdd story deliver` for the story merges. A blocked step
becomes a decision at the checkpoint as [[topic:human-stops]] defines.

## Resumption and what it returns

A stage is `done` when the three-part done test of [[topic:workflow-shape]] holds; running the same
command in a new session continues the work. Ends: `done` with the merged commit; `needs_human` when
a person must merge; `blocked` with a decision opened at the checkpoint; or `failed`, which the next
run retries. The return value is a workflow result ([[schema:workflow-result]]); standalone, the last
step is a clerk running `vsdd checkpoint story --run <id>`
([[command:checkpoint]]).

## Feature flags (decision D31; built when the build widens)

Not on the thin slice path: no stage, step or prompt of this workflow changes. For a story that names a
feature flag the red and green gates and the items the review accounts for follow the rows of
[[table:flag-story-checks]]. An existing test that the green gate's run with the flag on names reaches the
test-writer through the `test-defects` step, as a wrong frozen test does ([[table:flag-duties]]). The
capability is [[topic:feature-flags]].

## Team use (decision D33; built when the build widens)

Not on the thin slice path: no stage or prompt of this workflow changes. The survey claims the story,
and a story another clone holds is refused with its holder ([[topic:team-claims]]). A commit a teammate
pushed to the story branch by hand is merged in, moves the position back and, where it changed a frozen
test, reaches the test-writer through the `test-defects` step. A delivery the host will complete later
ends the run `partial` and opens no decision ([[topic:team-delivery]]).
