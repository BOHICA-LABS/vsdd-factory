---
kind: command
id: claim-release
package: M3
sources: [decisions.md#D33]
---

# `vsdd claim release`

Built when the build widens past the thin slice ([[packaging:build-plan]]). Decision D33.

Gives up a work claim this clone holds, after publishing the work it covers.

## Syntax

`vsdd claim release <scope> [--cycle <id>]`. The scopes are [[table:work-claim-scopes]].

## What it does

A checkpoint removes every claim its run no longer needs, and a full sync removes what a run or a
command left behind. This command is for the claims both keep: a story in flight, when its holder stops
working on it, and a claim a person took over and no run has entered (T-RELEASE in
[[table:work-claim-checks]]).

For a story it first makes the hand-over complete (T-HANDOFF): the red record and the story branch are
on the remote, and nothing in the story's worktree is left uncommitted. Then it removes the claim file
and syncs, naming the claim and the paths its scope covers, so the removal and the story's records
reach the remote together ([[command:sync]]). The story is free: the next survey on any clone takes it
with no person-only step and continues from what was pushed ([[topic:team-records]]).

It is built with the hand-off it depends on, in package M3.

## Who may call it

The main session and a person in a terminal, as for `vsdd lock release`.

## Writes

The story's closure record ([[record:closure]]) and the tracked copy of its red record
([[record:evidence]]), the story branch on the remote, the removal of the claim file
([[record:work-claim]]).

## Output and failure cases

The released claim and what was pushed. `ok: false`, with nothing removed, when another clone holds the
claim (a person uses [[command:claim-take]]), when a live run on this clone still works on the unit
(finish it, or end it with `vsdd lock release`), when the story's worktree holds uncommitted changes,
which are named, when the story branch is not accepted by the remote, and when the sync does not end
`synced`.
