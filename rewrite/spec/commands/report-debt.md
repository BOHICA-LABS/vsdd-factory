---
kind: command
id: report-debt
package: S6
sources: [BLUEPRINT.md#4.9, state.md#2.16, BLUEPRINT.md#3.4]
---

# `vsdd report debt`

Derived report of waivers, advisory findings, flaky inherited tests and the debt a baseline recorded.

## Syntax

`vsdd report debt`

## What it does

"the derived report of waivers, advisory findings and flaky inherited tests, all read from closure records,
and the debt a baseline recorded." "`vsdd report debt` lists what the baseline recorded and the flaky tests
the closure records hold" — the flaky identities are copied from gate evidence, which is not kept, into the
unit's closure record, "which is where `vsdd report debt` reads it" ([[record:closure]],
[[record:baseline]]).

Inherited debt does not block new work: "A test that already failed, a lint or scanner finding that already
existed, and a module the cycle did not touch do not fail a gate; anything new does. `vsdd report debt`
lists what was inherited, and `maintain` pays it down" ([[topic:brownfield]]). It also appears as the debt
figure inside [[command:gate-sweep]]'s job, "this replaces the technical-debt register sweep".

## Who may call it

`clerk` (derived view; read-only).

## Writes

Nothing: it is a derived view ([[topic:stored-and-derived]]).

## Output and failure cases

The report; a project with no baseline and no closures has nothing to report.

## Feature flags (decision D31; built when the build widens)

The report also lists the flag debt: the figures and the flags that [[topic:flag-lifecycle]] names under
"The release".
