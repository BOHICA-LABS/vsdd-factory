---
kind: topic
id: team-claims
package: M2
sources: [decisions.md#D33]
---

# The work claim

Built when the build widens past the thin slice ([[packaging:build-plan]]). Decision D33; the capability
from end to end is [[topic:team-use]].

A work claim says which clone has a piece of work, so that two clones never take the same one. The
record is [[record:work-claim]], what can be claimed is [[table:work-claim-scopes]], and the checks are
[[table:work-claim-checks]]. This file says what was decided and why; the rules themselves are in those
rows and are not repeated here.

## What can be claimed

Five scopes, in two kinds. A **story** is held for as long as it is in flight, because a story in flight
has a branch, a worktree and evidence that tracked state does not hold. Everything else is held for one
run or one act: the **cycle** claim by a cycle-level run, the **spec** claim by a run that may write the
specification, and the two **open** claims for the moment a cycle is opened. Those runs leave all they
produced in tracked state at their checkpoint, so the next run can be anybody's.

The things decision D33 names map onto them: a story is `story.<id>`; a wave and a release are the
`cycle` claim, whose record names the workflow and the wave; a specification change is `spec`.

The run lock is unchanged and stays on the clone ([[topic:run-lock]]). It keeps one clone to one run;
the claim is what two clones see of each other.

## How exactly one of two clones wins

A claim is a file on the state branch, and making it is a push (T-PUBLISH). A remote updates a branch
for one of two pushes that arrive together ([[topic:team-git-facts]], fact 2), and the push names the
head it was built on, so the remote takes it only if nothing has changed since the clone looked (T-PUSH
in [[table:sync-checks]]; fact 16). The clone's own state branch moves only after the remote has
accepted, so the loser has nothing to undo, and a clone that cannot reach the remote cannot hold a
claim at all.

Two things about that push are not left to how git words its answer. Whether a refused push lost a race
or was declined by the remote, and whether a push that got no answer was accepted after all, are both
measured by a fetch (T-PUSH in [[table:sync-checks]]). Without that the loser of every real race would
be reported as blocked by a rule, and a clone could hold a claim it believed it had failed to take.

Why a file on the state branch, and not a ref of its own or a branch per claim: the state branch is the
one shared thing every clone already fetches, so a claim is seen by the same fetch that brings the rest;
it needs nothing of the remote beyond an ordinary branch; and the branch's history then says who held
what and when, with no second record.

## Where everyone sees it

In `.factory/cycles/<cycle>/claims/` after any sync. `vsdd status` lists every claim with its person,
its clone's label and the time its holder wrote; the briefing has one line for what this clone holds and
one for what is held elsewhere; a survey leaves stories held elsewhere out of its units and lists them
in `elsewhere` ([[command:survey]]). All of it is read from files, with no fetch.

## How it ends

By the checkpoint, by a full sync once the run or the command that took it is gone, by its holder with
[[command:claim-release]], or by another person with [[command:claim-take]] (T-RELEASE, T-TAKEOVER).
T-RELEASE says exactly which claims a sync removes and when a story counts as begun. Two reasons shape
it: the work of a begun story is on a branch and not in tracked state, so its claim has to outlive the
run; and a claim a person took over was left behind by no run of this clone, so a sync that removed it
would undo what the person just decided. A claim's removal never reaches the remote ahead of the
records the claim covered, so no clone can step into work that is half published.

## A claim whose clone is gone

Git has no notion of a live clone, and the clocks of two machines cannot be compared. That is the
reason for the rule of T-TAKEOVER, which is stated there and nowhere else: a person decides, with
`vsdd claim take`.
The act is recorded and never silent: the taker is shown the holder and what was pushed before anything
changes, and the old holder's clone is told at its next sync and can write nothing more to the unit
(T-HOLD). The cost of a wrong take-over is therefore one clone's unpushed work on one unit, kept on that
clone and reported, and never a mixed record.

## A wave on a team

The stories of one wave can be claimed by different people: a standalone `story` run takes one story,
and a `wave` run takes the batch it will start, which [[table:work-claim-scopes]] defines, and leaves
the others free, so that it holds no story it does not work on. One clone at a
time drives the wave, because the `wave` run holds the cycle claim; the gate is run by whichever clone
holds it once every story is delivered, descoped or moved (T-WAVE in [[table:team-delivery-checks]]).
Nothing waits inside a run: the next `wave` run looks again.

## Turning team use on

The first sync that sees `team.enabled = true` on the clone that set it claims the stories that clone
has in flight (T-SWITCH), so those claims are on the remote before any other clone syncs as a member of
the team. A clone that was mounted before the team was formed has work in flight too, so it claims in
the same way at the sync in which it joins (T-SOLO in [[table:sync-checks]]); otherwise another clone
would take its story as free and build it a second time.
