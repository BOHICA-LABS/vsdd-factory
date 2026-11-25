---
kind: topic
id: gates
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.2]
---

# Gates

A gate is one measurement the tool makes and records. "Measure one gate and write its evidence record".
Why a measurement by the tool is the point: scripts steer, tools measure, hooks enforce —
see [[topic:trust-layers]]. "Build, test, lint, format, run, mutation, scan, benchmark and proof commands
come from project configuration. No workflow, agent prompt or gate names `cargo` or any other toolchain."
An agent that produces something finishes by running the gate command for it and returns the gate's JSON in
its result.

## The eleven gate names

"The gate names are eleven: `red`, `green`, `demo`, `spec`, `plan`, `wave`, `release`, `ingest`,
`contracts`, `integration`, `sweep`."

[[command:gate-red]], [[command:gate-green]], [[command:gate-demo]], [[command:gate-spec]],
[[command:gate-plan]], [[command:gate-wave]], [[command:gate-release]], [[command:gate-ingest]],
[[command:gate-contracts]], [[command:gate-integration]], [[command:gate-sweep]].

## Checks inside a gate

"`wave`, `release`, `ingest` and `sweep` each run every check that is due as one job (`baseline`, `ui`,
`formal`, `perf`, `audit` and the rest are checks, selectable with `--only`)."
"`vsdd gate baseline`, `ui`, `formal`, `perf` and `audit` are no longer commands of
their own: they are checks inside `wave`, `release`, `ingest` and `sweep`, selectable with `--only`."

What each job runs: "`wave` (suite, lint, build, mutation, clone fidelity, UI), `release` (`baseline`,
`formal`, `perf`, `ui`), `ingest` (coverage, baseline), `sweep` (audit, benchmarks, clone fidelity, spec
coherence, the debt report, collection). A failing check is reported with the others."

## A story gate measures a commit

`red`, `green` and `demo` bind their evidence to the head commit of the story's worktree, so they measure
only a clean worktree. When a tracked file is modified, or an untracked file is not ignored, the gate
measures nothing and returns `passed: false` with the failure `uncommitted changes: commit them, then run
the gate again`, naming the files; it writes no record. The JUnit report at the path `commands.test_report`
names is the one untracked file that does not count. A passing record for the head is returned without
measuring again; a failing record never is.

## Evidence

Every gate writes an evidence record, one per gate and commit, carrying the `git patch-id` of the subject's
diff; look-up after a clean rebase is by patch id ([[record:evidence]], [[topic:evidence-lookup]]). The
output shape is the gate result schema ([[schema:gate-result]]). Gates that build or test run detached and
re-attach ([[topic:long-commands]]). Against a baseline record the measured gates "fail only on what is new
against it" ([[record:baseline]]).

## Feature flags (decision D31; built when the build widens)

Not on the thin slice path. The gate names stay eleven. Decision D31 adds checks inside the existing
gates, each a row of [[table:flag-checks]], [[table:flag-story-checks]] or
[[table:flag-release-checks]]; in `wave`, `release` and `sweep` they are one check, `flags`.

## Team use (decision D33; built when the build widens)

Not on the thin slice path. The gate names stay eleven. The wave and release gates gain the list of hand
commits ([[table:team-delivery-checks]]).
