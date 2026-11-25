---
kind: topic
id: story-life
sources: [BLUEPRINT.md#3.3, pipeline.md#2.9.1]
---

# Story life

How one story travels from a request to a merged change, and the positions it passes through. The
story playbook itself is [[workflow:story]].

Positions, in order: `ready`, `red`, `green`, `reviewed`, `evidenced`, `delivered`. The survey computes a
story's position from records and git, never from what a script reports: the story is at the last position
for which the condition below holds for it and for every position before it.

- `ready`: the story is admitted and has its branch and worktree ([[topic:story-admission]]).
- `red`: a passing red or re-freeze record exists on a commit between the base and the head; a kind with
  no red gate is here at once ([[table:story-kinds]]).
- `green`: green evidence covers the head, by commit or by patch id ([[topic:evidence-lookup]]).
- `reviewed`: the tool's verdict is converged and covers the head ([[topic:verdicts]]).
- `evidenced`: demo evidence covers the head, or the story needs none ([[command:gate-demo]]).
- `delivered`: the story's closure record holds `delivery.merged_sha` ([[record:closure]]).

A commit added after a position was reached moves the story back to the last position that still holds,
which is how a repair after a failed delivery is reviewed again.

The example is a bug fix in a product that already has an approved specification, which is the common
case after a first release or an ingest.

1. A person types `/vsdd-factory:feature "CSV export drops the last row"`. The workflow's first step
   is a clerk running `vsdd survey feature`, with the request passed as a percent-encoded argument.
   The tool checks preconditions (a baseline exists), opens a run record and the cycle, takes the run
   lock and returns what is already done.
2. The architect analyses the change against the approved specification and finishes with
   `vsdd feature route`. The tool derives the route from the stored analysis and records it in
   `cycle.json`. Here the request is a bug, and an existing contract already says the export contains
   every row, so the route is "bugfix": one fix story, no specification change.
3. The story-writer writes the story file. Its acceptance criteria cite the existing contract clause
   and include a regression test. It finishes with `vsdd gate plan --story S-9.01`, which checks
   coverage, size and traceability in code. When the story passes, the same command admits it to the
   run: it records the story in the run and the cycle, creates the branch `story/S-9.01` and the
   worktree `.worktrees/S-9.01/`, and returns the story unit.
4. The `story` workflow runs as a child with that unit. Every later gate checks that the story
   belongs to the run, so a garbled unit cannot deliver anything.
5. Tests. The test-writer writes a failing regression test for each criterion and finishes with
   `vsdd gate red`. The tool runs the project's test command, reads the JUnit report, confirms the
   new tests fail on assertions, and records their identities as frozen. Until that evidence exists,
   a hook blocks every write to product source in the worktree by the implementer.
6. Implementation. The implementer makes the tests pass one at a time and finishes with
   `vsdd gate green` (build, full suite, lint, format, frozen tests unchanged since the red record).
   A hook stops it from editing any test.
7. Review. A fresh adversary reviews the diff against the story and its contracts, and a security
   reviewer joins when the module is critical or high. Each finding must cite a file, a line range
   and a quote, which the hook checks. If there are blocking findings, the owner of each fixes or
   disputes it: the implementer for code, the test-writer for a weak test (it then records the
   changed tests with `vsdd gate red --refreeze`). A new fresh reviewer checks those claims. One
   clean pass ends the loop. At the cap the architect arbitrates, and after that a person is asked.
8. Evidence. If the story has user-visible criteria, the demo-recorder records a script and
   transcript and runs `vsdd gate demo`. Otherwise the tool builds the criterion-to-test table
   itself.
9. Delivery. A clerk runs `vsdd story deliver`. The tool recomputes the review verdict from the
   stored passes, confirms green evidence on the head commit, pushes, opens the pull request, waits
   for CI, merges, and removes the worktree. No agent can merge or push to the integration branch
   any other way.
10. The workflow's last step is a clerk running `vsdd checkpoint feature`, which closes the units the
    evidence supports, commits and pushes state, releases the lock and returns the next command:
    `/vsdd-factory:verify`.

A story inside a wave travels the same way from step 4, with the unit made by the wave's survey. The
wave then runs its own gate (the suite on the integration head, an integration review, the hidden
scenarios) before it closes.
