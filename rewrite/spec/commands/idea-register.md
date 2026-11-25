---
kind: command
id: idea-register
package: S6
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.9.9]
---

# `vsdd idea register`

Writes ideas into the idea registry.

## Syntax

`vsdd idea register ...`

## What it does

The `register` stage of the `discover` scan mode: "clerk running `vsdd idea register`: Duplicates and ideas
in cool-down removed; the rest registered." It runs after the three scoring steps, whose geometric mean the
script computes; the idea statuses themselves are set later, when a person settles the `ideas` decision
("`select` with `pursue=`, `research=`, `defer=`, `reject=` lists of idea ids; each idea's own status is set
from its list") ([[table:decision-kinds]], [[artifact:idea]]).

## Who may call it

`clerk` (the command table's idea/report/discover/repos row).

## Writes

The idea registry — the registered ideas, minus duplicates and ideas in cool-down ([[artifact:idea]]).

## Output and failure cases

Not specified in the design beyond the registration effect.

## Team use (decision D33; built when the build widens)

Idea ids are taken inside a sync ([[table:work-claim-checks]], T-MINT).
