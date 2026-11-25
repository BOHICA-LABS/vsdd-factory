---
kind: topic
id: team-delivery
package: M4
sources: [decisions.md#D33]
---

# Delivery on a team, and commits made by hand

Built when the build widens past the thin slice ([[packaging:build-plan]]). Decision D33; the capability
from end to end is [[topic:team-use]]. The rules are the rows of [[table:team-delivery-checks]], which
also defines a hand commit; this file gives the reasons.

## Pull requests and CI

On a team a story reaches the shared branch through a pull request and the project's CI, never through
a merge made on one clone (T-DELIVERY). That is the `pr` mode of [[command:story-deliver]]: the tool
pushes the story branch, opens the pull request, waits for CI and merges at the reviewed head. Team use
cannot be turned on in `local` mode or with `delivery.ci = "none"` (T-SWITCH). CI is required and not
only preferred, because on a team the target moves under teammates' merges while a story is built, and
the pull request's checks are what measures the merged result before it lands.

Decision D33 adds four things to `pr` mode: the delivery refuses unless this clone holds the story's
claim; it records who merged; it has a state for a pull request the host will merge by itself later;
and it deals with a pull request merged outside the tool.

A pull request is not git, so one part of the tool talks to the host: the delivery's host module. It is
the one exception to "nothing names a host", and it carries no coordination: claims, decisions and
state never pass through it ([[table:team-checks]], the head rules).

Who may merge is not the factory's to enforce. A repository that wants a review before a merge, or a
current branch, says so in its own rules; a merge the remote refuses is reported with the remote's
message and waits for a person, as `awaiting_merge` does today ([[topic:governed-delivery]]).

The same holds for `vsdd release`: a repository set up as decision D33 expects protects its shared
branches, so in `pr` mode the release moves them through pull requests as well ([[command:release]]).

## A target branch that moves

Teammates' merges move the integration branch while a story is being built. The design already holds
for that, and nothing is added:

- a story's base is the remote head read when its branch is created, and every gate measures against
  that recorded commit, so later merges do not change what the story is measured against
  ([[topic:worktrees]]);
- the verdict and the green evidence are bound to the story's patch, so a story whose pull request still
  applies merges without another review ([[topic:verdicts]]);
- when the pull request conflicts, delivery answers `conflict`, the target is brought into the story
  branch, the new commits are reviewed and the story is delivered again ([[workflow:story]]);
- the merged result is measured by the project's CI on the pull request and, for the whole wave, by the
  wave gate on the integration head ([[command:gate-wave]]).

## Commits made by hand

A teammate may commit to the product without the plugin. The factory notices, never undoes, and brings
the commit under the same review as its own work. This does not depend on team use being on: a project
in which one person runs the factory and the others commit by hand has the same need, so these checks
do not read `team.enabled`. When each acts, and when the remote is asked about a branch at all, is
the `runs` cell of its row in [[table:team-delivery-checks]] and is stated nowhere else.

**On the integration branch** (T-NOTICE). Every survey lists hand commits; the wave gate and the release
gate make the list binding for the range they cover.

- *Baselines.* A recorded baseline is never changed. A story cut after a hand commit has it in its base.
  What the commit broke is new since the baseline, so the next green or wave gate fails on it, and the
  remedy is a fix story that changes the code forward.
- *Review.* Each hand commit is an item the wave's review, or the release's, must mark as checked before
  the verdict can converge, and the reviewer is given the commit's patch (T-ITEMS).
- *A specification that has fallen behind.* The reviewer that accounts for the item says whether the
  commit changes behaviour a contract fixes or adds behaviour no contract covers. If so its result opens
  `spec-gap`, and `amend` brings the specification up to the code (T-GAP). The release packet lists every
  hand commit of the cycle, so the person who authorises the release sees what entered outside the
  factory.

**On a story branch** (T-BRANCH). The holder's next survey brings the commit into the story branch under
its own id and records it; the story is then built, tested and reviewed again from there. To tell a
teammate's commit from the holder's own, the tool keeps the head it last pushed or checked: after its
own rebase the remote and local branches differ in every commit, and without that head the holder's
work would be taken for a stranger's and merged in twice. For the same reason a push of the story
branch is judged as a push of state is, by a fetch and not by its exit status: a push the remote took
while its answer was lost would otherwise be undone on the clone, and the next survey would meet the
holder's own rebased commits on the remote as if a teammate had made them. And where a rebase never
reached the remote and a teammate's commit arrived meanwhile, the branch is put back before that commit
is brought in, because a merge into the rebased branch leaves the holder's commits in the history
twice ([[topic:team-git-facts]], fact 17). A hand commit that changes a frozen test goes to the
test-writer, who judges it and re-freezes it; no builder may quietly restore the old one.

**What the factory never does** (T-NEVER) is rewrite or undo a person's commit. The one push that can
replace commits on the remote, after its own rebase of a branch with no hand commit, names the head it
expects there, because a lease without that value overwrites whatever arrived since the last fetch
([[topic:team-git-facts]], fact 12).

What it cannot see is a hand commit made in the holder's own story worktree: that is a commit of the
story, and the review loop covers it as it covers any new commit.
