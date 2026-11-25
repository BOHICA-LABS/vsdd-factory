---
kind: command
id: decide
package: S4
sources: [BLUEPRINT.md#4.9, state.md#2.16, BLUEPRINT.md#4.11, state.md#2.8]
---

# `vsdd decide`

Settles a decision by writing the decision record and nothing else; refused when called from an agent's
tool call.

## Syntax

`vsdd decide <id> <choice> [key=value ...] [-- note]`

## What it does

"Settles a decision: writes the decision record and nothing else. Refused when called from an agent's tool
call." The grammar and the nine kinds are the decisions design's ([[topic:decisions]],
[[table:decision-kinds]]): "Every value is named, a key may repeat, and free text goes after `--`. There
are no positional values and no quoted strings."

Settling: "`settle_decision` writes the decision record and nothing else (with one small exception: for
`spec-gap amend` it also writes the amendment request file): it does not promote any file, it does not run
git, and it returns in a few milliseconds, which is what lets the hook that captures a typed decision stay
inside its four-second deadline on a specification of any size. The next survey commits the record."
Choices map to statuses ("`approve` maps to the status `approved`, `revise` and `hold` to `revise`, `drop`,
`reject` and `descope` to `rejected`").

Ambiguity: with two open cycles the decision id is written `<id>@<cycle>` ([[topic:decisions]]); the
command refuses a bare id that is ambiguous, with the choices.

## Who may call it

Acts only a person may perform — "the person: through the typed command for `decide`". The hook runner
calls it "when the person types `/vsdd-factory:decide`, or by the person in a terminal; never an agent"
([[rule:decisions]]). The event log records `decision.recorded` and `decision.refused` ("A person typed the
decide command") ([[topic:event-log]]).

## Writes

The decision record and packet — and, for `spec-gap amend` only, the amendment request file
([[record:decision]]).

## Output and failure cases

The settled record; refusal of an agent call, of an ambiguous bare id, and of an undocumented choice.
A decision whose digest no longer matches "reads as `stale` and cannot be settled" ([[topic:digests]]).

## Feature flags (decision D31; built when the build widens)

Not on the thin slice path. Settling gains one refusal, F-APPROVE of [[table:flag-checks]]: approving a
specification that proposes a flag system needs the value `flags`, and rejecting the removal of a flag
needs `until` ([[topic:flag-system-choice]], [[topic:flag-lifecycle]]).

## Team use (decision D33; built when the build widens)

Not on the thin slice path. In a terminal the command syncs before it settles and after, so the answer
is on the remote when it returns; it refuses when the remote already holds the decision settled, and
shows by whom. A settlement the remote does not hold yet is acted on nowhere
([[table:work-claim-checks]], T-SETTLE).
