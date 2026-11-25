---
kind: topic
id: decisions
package: S4
sources: [BLUEPRINT.md#4.11, state.md#2.8, BLUEPRINT.md#4.8]
---

# Decisions

Decisions as records. The nine kinds, when each opens and the choices each accepts, are [[table:decision-kinds]];
the record and its packet are [[record:decision]]. How a stop works inside a workflow is
[[topic:human-stops]].

## Grammar

`/vsdd-factory:decide <id> <choice> [key=value ...] [-- note]`. Every value is named, a key may repeat, and
free text goes after `--`. There are no positional values and no quoted strings, so the parser built to this
grammar accepts every documented choice.

## How choices map to statuses

`approve` maps to the status `approved`, `revise` and `hold` to `revise`, `drop`, `reject` and `descope` to
`rejected`. Which choices each of the nine kinds accepts is [[table:decision-kinds]], one row per kind.

## Qualified identifiers

A decision's identifier is its kind (`brief`, `spec`, `release`, `plan`, `ideas`) or `<kind>.<subject>`
(`finding.S-7.04`, `merge.S-7.04`, `blocked.S-7.04`). When two cycles are open at once (a critical fix
beside the main cycle) and both hold a decision with the same identifier, each is written `<id>@<cycle>`,
for example `release@hotfix-1.2.1`. The briefing prints the qualified form, and a bare identifier that is
ambiguous is refused with the list of choices.

## What settling writes, and approved dependencies

Settling writes the decision record and nothing else (for `spec-gap amend`, also the amendment request
file): no file is promoted and git is not run, so it always fits inside the hook's time limit. The next
survey commits it. Dependencies approved by a `merge` decision are readable from the record:
dependency detection in `vsdd story deliver` reads `dependencies.toml` ([[artifact:dependencies]]) and the
`dependencies` of every approved `merge` decision that is newer than the latest approved `spec` decision.
The next `spec` or amendment run has the architect fold them into `dependencies.toml`.

## Team use (decision D33; built when the build widens)

Not on the thin slice path. Settling gains the person and the clone (`settled_by_person`, `settled_on`),
and "latest" and "newer" are read from a link the later record stores, never from two times
([[table:team-checks]], T-WHO, T-ORDER). A settlement is published by a sync, counts only once the
remote holds it, and of two the first pushed stands ([[table:work-claim-checks]], T-SETTLE).
