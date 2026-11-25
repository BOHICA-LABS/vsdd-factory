---
kind: topic
id: team-records
package: M3
sources: [decisions.md#D33]
---

# What the records say on a team

Built when the build widens past the thin slice ([[packaging:build-plan]]). Decision D33; the capability
from end to end is [[topic:team-use]].

Who acted, how a decision opened on one clone is settled on another, how records are ordered without a
shared clock, and what a hand-off continues from. The rules are rows of [[table:team-checks]] and
[[table:work-claim-checks]]; this file gives the reasons.

## Who

The factory records two things about every act: the **person**, as the identity git stamps on a commit
made in that clone, and the **clone**, by the id and label in its clone record ([[record:clone]]). Where
each act is recorded, and how the identity is read, is T-WHO. The record of who started a run needs no
field of its own: it is the author of the run's first state commit, which is its claim.

It checks no authority. No key lists people or roles, no check compares a name with anything, and any
person may claim, settle, take over, merge and release. Whether the name is true, and whether that person
may approve or merge, is for the repository's own rules, as decision D33 says. The one thing refused is
no identity at all.

## A decision opened on one clone and settled on another

A checkpoint opens a decision and pushes it with the rest of its state. Another clone has it after its
next sync, and its briefing then lists it under "Waiting on you" from the files; the project's
notification channel, where one is set, is what tells a person to look ([[topic:human-stops]]). Nothing
reads that channel, and nothing depends on it.

Any person on any clone may settle it. When two people settle the same decision, the one whose push
arrives first has settled it, and the other is told who did and what each chose. What makes that safe is
that a settlement counts only once the remote holds it: until then every reader on the clone that typed
it sees the decision as open (T-SETTLE). That holds however the settlement is published, at once from a
terminal or a moment later for one typed in a session ([[rule:decisions]]).

## Order without clocks

Two machines' clocks need not agree, so nothing orders two tracked records by their times (T-ORDER).
Where the design says "the latest `spec` decision" or "a `merge` decision newer than it", the later
record stores a link to the earlier one, and the order is read from the links. Times stay in the records
for people to read, each beside the clone that wrote it.

## What moves and what stays

What is shared and what belongs to one clone is the table in [[config:team]]. A run does not move
between machines. A unit does: another clone starts a new run on it, from tracked state and the branches
on the remote.

## What a hand-off continues from

From what was pushed, and nothing else (T-HANDOFF). The checkpoint, and `vsdd claim release` for one
story, push the story's branch and the red record. Nothing is pushed more often: a push inside a run
would publish half-written work, and the checkpoint is already the boundary every workflow ends on. A
clone that is lost takes with it what it did after its last checkpoint and whatever it never committed;
the next holder repeats that part. If that proves costly, the remedy is the one [[topic:risks-state]]
names, an extra checkpoint inside a long workflow.

The next holder's survey then places the story as it places any story ([[topic:story-life]]), from the
branch on the remote and these records. If nothing was pushed, the story starts again from its base.

## Is a gate result from another machine trusted?

A unit that a checkpoint closed is trusted on every clone, as tracked state is on one clone. For a unit
in flight, evidence does not travel: green, demo and the wave and release gates are measured again on the
clone that continues, which the survey already does wherever evidence is missing ([[topic:resume]]). The
review's cover comes from the closure's verdict when it covers the head. The red record is the one
exception, because it cannot be measured again once the implementation exists: it travels as a tracked
copy, written by the same command that measured it, with the cap of the record it copies, so a record
that could be written can always travel ([[record:evidence]]).

## Ids

Two clones must not give two files the same id. Inside the `spec` claim, and for the stories of an epic
that one cycle's run owns, that cannot happen. An id that more than one claim could mint is minted
inside a sync, and the command returns it only once the remote has accepted the file (T-MINT). A run id
is local to its clone; where one is written into tracked state it is read together with the clone that
wrote it.
