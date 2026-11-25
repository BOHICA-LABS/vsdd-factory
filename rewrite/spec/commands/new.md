---
kind: command
id: new
package: S1
sources: [BLUEPRINT.md#4.9, state.md#2.16, state.md#2.13, state.md#2.4]
---

# `vsdd new`

Stamps an artifact skeleton with the next free identifier.

## Syntax

`vsdd new <kind> ...`

## What it does

"Stamps a skeleton with the next free identifier." Skeletons live in `<plugin>/templates/<kind>.md`, one
per agent-written kind — "the frontmatter keys with their starting values and the required headings, each
with one line saying what belongs under it" — and "`vsdd new <kind>` stamps one with the next free
identifier" ([[artifact:story]] and the other artifact kinds).

Identifier allocation: "Identifiers are never renumbered and never reused. A retired artifact keeps its
file with `status: retired`, which is what keeps the number occupied. Because there is one state directory,
an agent that picks a number already taken is blocked at the write and told the next free one.
`vsdd new <kind>` allocates for callers that have a shell" ([[table:identifiers]]).

## Who may call it

Main session (the read/write row of the command table: "`new <kind>`, `note`, `defer add/list`,
`retire <path>`, `driver on/pause/off`, `lock release` | Main session").

## Writes

The new artifact file under the state layout, stamped from the skeleton.

## Failure cases

A kind with no skeleton, or a state-directory write rule refusal, surface as the hook's write block; the
identifier the write would take is already covered by the collision rule above.

## Team use (decision D33; built when the build widens)

"One state directory" is true of one clone. With team use on, an id that more than one claim could mint
is taken inside a sync, and a number an agent picked for itself that another clone also used shows at
the sync as one path added twice ([[table:work-claim-checks]], T-MINT).
