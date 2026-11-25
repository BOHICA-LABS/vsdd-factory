---
kind: record
id: baseline
package: G7
sources: [BLUEPRINT.md#4.13, state.md#2.8, pipeline.md#2.9.8]
---

# Baseline

The baseline is the closure record of an ingest or an import:
`cycles/_standing/units/ingest.json` (or `import.json`), a closure record with a `baseline` object (fields
in [[schema:baseline]]). Tracked. A cycle's `cycle.json` names its baseline.

## What it holds

The integration head it was measured on; any earlier release tag; the suite result with the identities of
the tests that failed; the lint and format findings; the mutation score per module; the scanner findings in
existing dependencies; the recovered or imported contract ids; and whether the project has a suite, a JUnit
report and CI.

## Written by

`vsdd gate ingest` measures it; for an import, the first survey that needs the measurements takes them.

## What it is for

The green, wave and release gates read it and fail only on what it does not list, and they apply mutation
and coverage thresholds to the modules the cycle touched. An inherited test that fails and then passes when
the gate runs the suite once more does not fail the gate; the gate result lists it in `flaky`, and the list
is copied from the evidence into the unit's closure record, where `vsdd report debt` reads it. A cycle whose
baseline is empty is a first release and runs `verify` in full scope; every other cycle runs in delta scope.
`vsdd report debt` lists what the baseline recorded and the flaky tests the closure records hold.

## Team use (decision D33; built when the build widens)

A commit made by hand never changes a recorded baseline; what it broke fails the next gate as new
([[topic:team-delivery]]).
