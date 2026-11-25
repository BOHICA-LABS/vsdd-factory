---
kind: topic
id: walled-findings
sources: [pipeline.md#2.7, BLUEPRINT.md#4.10, state.md#2.5]
---

# Walled findings

What makes a finding walled, where its full form lives, and what everything outside the holdout wall
is shown. The wall itself is [[topic:information-walls]]; the findings built from scenario scores are
[[topic:holdout-findings]].

A finding returned by the `scenario-reviewer`, or any finding whose location lies under
`.factory/holdout/`, quotes or names a hidden scenario. Such a finding is walled: its full form is
stored only in the `holdout-evaluations` class — the evaluation file
`holdout/evaluations/<cycle>/<unit>-<n>.json`, inside the holdout wall, which also holds
`walled_findings` (the full form of scenario-review findings and of any finding located under
`holdout/`), written by `vsdd checkpoint` from the evaluator's and the scenario-reviewer's step
records ([[record:holdout-evaluation]]).

Its key holds the scenario's path and clause, so everywhere else it is named by its walled key:
`walled:` and the first 12 hex characters of the SHA-256 of the key, which is as stable across passes
and reveals nothing.

Decision packets, closure records, claims and `vsdd review show` ([[command:review-show]]) carry the
walled key and the severity and nothing else for a reader who may not read `holdout`, and a waiver
names the walled key. The closure record, the decision packet and `vsdd review show` keep the walled
key and severity only, with the line "details are inside the holdout wall";
`vsdd review show <unit> --include-walled` prints the full form for a caller whose wall allows
`holdout`, and for a person in a terminal.

The scenario review therefore runs as its own convergence loop ([[workflow:spec]]), with its own
claims, routed only to the `product-owner`. A universal harness test asserts that no prompt sent to
an agent that is denied `holdout` contains text from a walled finding. A policy test asserts that no
record outside the holdout class and `runs` contains a scenario path or a quote from one.
