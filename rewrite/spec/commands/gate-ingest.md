---
kind: command
id: gate-ingest
package: G7
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.9.8]
---

# `vsdd gate ingest`

Measures ingest coverage and the baseline record of an existing codebase.

## Syntax

`vsdd gate ingest --run <id>`. A job command ([[topic:long-commands]]).

## What it does

The `ingest` workflow's "coverage and baseline" stage: "clerk running `vsdd gate ingest` as one job; a
second analyze round over the files no analysis cited, at most 2 rounds". It produces "Coverage evidence,
and the measured baseline: the suite result with the identities of the tests that fail today, lint and
format findings, the mutation score per module, scanner findings in existing dependencies, and whether a
suite, a JUnit report and CI exist". The gate's own pass test: "Cited source files at or over the
configured share" of the codebase.

The baseline record is what every later delta cycle compares against: "`vsdd gate ingest` measures it; for
an import, the first survey that needs the measurements takes them" — "The green, wave and release gates
read it and fail only on what it does not list, and they apply mutation and coverage thresholds to the
modules the cycle touched" ([[record:baseline]], [[topic:brownfield]]).

## Who may call it

`clerk` (the `coverage and baseline` stage).

## Writes

"Closure rounds for `ingest`; the baseline record for `ingest`" — the baseline closure record in
`cycles/_standing/units/ingest.json` ([[record:closure]]).

## Output and failure cases

"Gate result" ([[schema:gate-result]]); coverage under the configured share sends the run back through the
analyze stage, at most 2 rounds.
