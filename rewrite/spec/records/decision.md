---
kind: record
id: decision
package: S2a
sources: [BLUEPRINT.md#4.7, state.md#2.8, BLUEPRINT.md#4.11, state.md#5]
---

# Decision record

`cycles/<cycle>/decisions/<id>.json` and its packet `<id>.md`. Tracked. A decision is a question only a
person can answer, stored as a record and a packet a person can read. Fields in
[[schema:decision-record]]; the nine kinds and their choices are [[table:decision-kinds]]; see also
[[topic:decisions]].

## Written by

`vsdd checkpoint` opens; `vsdd decide` settles; `vsdd import` writes one approved `spec` decision. A decision
is settled only by a person, through the typed `decide` command or in their own terminal.

## How a decision is protected

The record holds `id`, `kind`, `status` (open, approved, revise, rejected, stale), `choice`, `values` (the
`key=value` pairs), `note`, `digest`, `opened`, `settled`, `settled_by` (`session`, `terminal` or `import`),
`settled_by_session`, and for a `blocked` decision its `reason`. A `spec` decision also holds `files` (the
paths the digest covers) and `accepted` (the ids of the decision records it accepted). A `merge` decision
holds `dependencies` (the names and versions it approved). `vsdd checkpoint` computes the digest of what is
being decided (the specification file set for `spec`, the commit for `release`), over the files exactly as
they are. The survey recomputes the digest, and a decision whose digest no longer matches reads as `stale`
and cannot be settled. See [[topic:digests]].

## Settling changes no artifact

`settle_decision` ([[topic:state-api]]) writes the decision record and nothing else (with one small
exception: for `spec-gap amend` it also writes the amendment request file): it does not promote any file, it
does not run git, and it returns in a few milliseconds, which is what lets the hook that captures a typed
decision stay inside its four-second deadline on a specification of any size. It is also the only writer:
the person's `/vsdd-factory:decide` is recorded only through `settle_decision`, so a settled decision cannot
be listed as open. The next survey commits the record.

## Cap and lifetime

Cap 4,000 bytes for the record and 12,000 for the packet. Kept permanently. The schemas for step results,
verdicts and decisions in `<plugin>/schemas/` stay within these caps (a step record of 64 KB, a packet of
12 KB), and a write over a cap is refused.

## Team use (decision D33; built when the build widens)

Not on the thin slice path. Three fields are added: `settled_by_person` (the identity git gives in the
clone that settled), `settled_on` (that clone's id) and, in a decision whose id carries no subject,
`follows`; a `merge` decision also stores `spec` ([[table:team-checks]], T-WHO, T-ORDER). A settlement
counts only once the remote holds it ([[table:work-claim-checks]], T-SETTLE). The schema stays open, so
a record written before them still validates.
