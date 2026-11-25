---
kind: workflow
id: ingest
package: W9
sources: [pipeline.md#2.3, pipeline.md#2.5, pipeline.md#2.9.8, pipeline.md#2.9.13, BLUEPRINT.md#4.5, BLUEPRINT.md#5.1]
---

# `ingest`

Reads an existing codebase and records what it is: module map, recovered contracts, architecture,
conventions, the dependency list, the toolchain, security and verification gaps, and a measured
baseline — ending at the spec decision on the recovered baseline.

Arguments: `source` (path or URL; default the project), `as` (`project` or `reference`), `guidance`,
`repo`. Preconditions: none beyond the common one ([[table:preconditions]]).

## Stages in order

### survey

- Step and agent: clerk.
- Produces: survey with a file inventory by directory and language.

### map

- Step and agent: codebase-analyzer, method `ingest-broad`, on the builder tier by the roster's
  `[overrides]` ([[table:roster]]).
- Produces: the module map (module-map data), which the script fans out over.
- Gate: `r.status == "done"` and at least one module.

### analyze

- Step and agent: codebase-analyzer, method `ingest-deepen`, on the builder tier by the same
  override, one per module, as a pipeline.
- Produces: per-module analysis — behaviours with file citations, conventions, dependencies, existing
  tests.

### validate

- Step and agent: adversary (reviewer), method `review-extraction`, the second pipeline stage for
  each module; corrections by codebase-analyzer (planner tier, method `ingest-deepen`), at most 2
  rounds.
- Produces: confirmed or corrected analysis.
- Gate: no claim contradicts the code.

### coverage and baseline

- Step and agent: clerk running `vsdd gate ingest` ([[command:gate-ingest]]) as one job; a second
  analyze round over the files no analysis cited, at most 2 rounds.
- Produces: coverage evidence, and the measured baseline — the suite result with the identities of
  the tests that fail today, lint and format findings, the mutation score per module, scanner
  findings in existing dependencies, and whether a suite, a JUnit report and CI exist.
- Gate: cited source files at or over the configured share.

### synthesize

- Step and agent, in parallel: architect, method `architecture` (recovered architecture, module
  criticality, `conventions.md`, `review-rules.md`, `dependencies.toml` with the current dependencies
  as the approved list, and `toolchain.toml` with the commands the project uses or, where it has
  none, the ones a harness story will provide); architect, method `verification-assessment` (the
  verification gap analysis, an assessment of kind `verification`); product-owner, method
  `recover-contracts` (recovered contracts marked `origin: recovered`, scenario seeds);
  security-reviewer, method `security-review` (security and dependency audit, written once to
  `analysis/<scope>/security.md`). Then product-owner (project context, method `domain-spec`).
- Produces: the baseline that `feature` and `spec` build on.
- Gate: artifacts valid.

### review

- Step and agent: adversary (reviewer), method `review-specs`; fixes by the owning planner.
- Produces: review data.
- Gate: `converge()` at the `spec` perimeter, cap 2 ([[topic:convergence]]).

### close

- Step and agent: clerk.
- Produces: the baseline closure record in `cycles/_standing/`, and decision `spec` on the recovered
  baseline. The packet is the ingest report, with low-confidence areas and any critical security
  finding at the top, the recovered contract set, the measured baseline and the toolchain lines.

## `as: reference`

With `as: "reference"` the map, analyze and validate stages run, followed by two stages of their own:
`disposition` (codebase-analyzer, method `disposition`, judged against the approved brief or the
`guidance` argument: model, reimplement, enhance or leave) writes `analysis/<ref>/disposition.md`,
and `port-analysis` (codebase-analyzer, method `port-analysis`) writes
`analysis/<ref>/port.md`. The assessments stage of `spec` and the delta step of a `feature` with
intent `port` read both.

## What the baseline is for

Ends: with `as: project`, decision `spec` on the recovered baseline; with `as: reference`, a report.
Recovered contracts describe what the code does. Approving the baseline makes them the approved,
locked specification, so "the specification is approved" has a meaning for every later `feature`, and
applies the toolchain file to any `[commands]` and `[paths]` keys that are still unset
([[topic:brownfield]]).

Every later cycle on this project is a delta against the baseline. The measured gates compare against
the baseline record: the green gate, the wave gate and the release gate fail only on a test failure,
lint finding, format finding or scanner finding that the baseline does not list, and on a frozen test
that fails; mutation and coverage thresholds apply to modules the cycle touched. The flaky rule for an
inherited test is the one [[record:baseline]] states; the gate result lists such a test in `flaky`,
the evidence carries it to the unit's closure record, and `vsdd report debt` reads it there
([[command:report-debt]]). A test the cycle added or changed gets no second run. What the project
inherited is listed by `vsdd report debt` and paid down through `maintain`. Plan coverage and
traceability apply to the contracts a cycle adds or changes, so approving several hundred recovered
contracts does not demand a story for each. A project imported from the old plugin has the same kind
of baseline, written by the importer, with the measurements taken by the first survey that needs them.

## Feature flags (decision D31; built when the build widens)

The analyze stage says whether each module uses feature flags. In the synthesize stage the architect
writes `flags.detected` and, where a flag system was found, proposes keeping it; the `spec` decision on
the recovered baseline confirms it ([[topic:flag-system-choice]], [[table:flag-duties]]).

## Team use (decision D33; built when the build widens)

With `as: project` the run takes the `spec` claim ([[table:work-claim-scopes]]).
