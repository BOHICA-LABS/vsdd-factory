---
kind: command
id: gate-demo
package: G2
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.9.1]
---

# `vsdd gate demo`

Records a story's demo evidence, or the criterion-to-test table when no recording is required.

## Syntax

`vsdd gate demo --story <id> --run <id>`

## What it does

Records the story's demo evidence ([[artifact:demo-evidence]]). "For a story with no user-visible criteria
the demo gate is satisfied by the tool itself: it builds the table of acceptance criterion to test to result
at delivery, with no agent." The rows of that table come from the `criteria` map of the story's red record
([[command:gate-red]]), each with the test's result in the latest green evidence. A wave-level demo is not part of the wave gate; it "can be switched on in
project configuration" (`evidence.wave_demo`, `evidence.final_demo`).

## Pass condition

Always: green evidence covers the head, else the gate fails with `no green evidence covers the head`.
The gate is direct: it never runs detached and never returns a pending answer.

- Story `demo` false: for a kind with a red gate a red record must exist, and each test in its `criteria`
  map must be passed in the latest green report. A criterion with no test is listed in `metrics.uncovered`
  and in the table, and does not fail.
- Story `demo` true: `report.md` is not empty; at least one criterion has one script (`.sh`, `.tape`,
  `.spec.ts`) and a transcript `AC-<k>.txt`; every file names a criterion; every file is within the
  `[[demo-evidence]]` row's caps (script 8,000; transcript 20,000; screenshot 300,000; 1,500,000 per
  story). The evidence directory is `.factory/demo/<cycle>/<story>/`.

## Who may call it

"`demo-recorder`, `clerk`."

## Writes

"Evidence `demo.<sha12>.json`; the criterion-to-test table when no recording is required"
([[record:evidence]]).

## Output and failure cases

The gate result ([[schema:gate-result]]); the story's `evidence` stage passes on `g(r,"demo").passed`, and
"the tool [checks] at delivery" as the second pair of eyes. Evidence lookup as for the green gate: "For
`green` and `demo` it returns the record for the head commit, or one whose patch id equals the head's after
a clean rebase" ([[topic:evidence-lookup]]).
