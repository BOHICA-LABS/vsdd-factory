---
kind: topic
id: spec-lock
sources: [state.md#2.4, BLUEPRINT.md#4.11, BLUEPRINT.md#5.2]
---

# The specification lock

`spec_locked(project, path)` is true when the file is approved and no live run holds an amendment. An
amendment covers the whole specification set. More precisely, in the form the state design states it: while
the latest `spec` decision is approved and no live run holds an amendment, no file in the classes
`spec-product`, `spec-arch` and `spec-ux` can be written, whether or not the file carries a status.

This is what locks the domain files, the architecture overview, `modules.toml`, `dependencies.toml`, the
toolchain file, conventions and the UX files, which never had an `approved` status to test.

## The guidance files

Two files, the class `guidance`, are treated a little differently, because they are guidance and not a
statement of what the product does: `conventions.md` and `review-rules.md` are locked with the rest while a
cycle is open, so nobody changes the rules in the middle of a build, and are writable by the architect
between cycles (`guidance_locked`), which is how a retrospective's proposal is adopted through
`/vsdd-factory:author` without an amendment run. They are outside the `spec` decision's digest
([[topic:digests]]), so that edit does not make the approval stale.

## The brief

The `brief` class is outside the lock and outside that digest; it has its own decision.

## Kinds that stay frozen even under an amendment

Three kinds stay frozen even under an amendment, because a change to them is a new artifact that supersedes
the old one:

- a decision record that an approved `spec` decision lists as accepted;
- a property that a `verify` closure lists as proven;
- a story whose delivery closure exists.

These are the files for which `artifact_locked(project, path)` is true.

## When the lock starts and ends

From the blueprint: the specification is locked from the moment a `spec` decision is approved, and it stays
locked until a run holds an amendment (a `feature` run on the standard route or with `amend`, or a `spec`
run that starts while an earlier approval stands); an accepted decision record stays frozen even then. If
the content changes any other way the decision reads as stale and the stop returns
([[topic:computed-status]], [[topic:digests]]).

Reopening work is explicit: settling `revise` or `hold` clears the entries of the closure record from the
stage the person named with `stage=` or, by default, from the entry named `review` onward
([[topic:amendments]]). An amendment run, not a direct edit, changes an approved specification
([[topic:amendments]]).

The hook-side enforcement of the lock — reason code `artifact_frozen`, one record read per guarded write —
is the hook rule `artifacts`.
