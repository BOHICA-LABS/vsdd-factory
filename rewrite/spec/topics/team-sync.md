---
kind: topic
id: team-sync
package: M1
sources: [decisions.md#D33]
---

# How a clone meets the remote

Built when the build widens past the thin slice ([[packaging:build-plan]]). Decision D33; the capability
from end to end is [[topic:team-use]].

One function, `vsdd.state.sync`, is the only way state leaves or enters a clone. Its order and its
outcomes are in [[command:sync]]; its checks are the rows of [[table:sync-checks]]. This file gives the
reason for each choice. The git behaviour it rests on is [[topic:team-git-facts]].

## Freshness: no run without a fetch

Decision D33 has a run start from what the team has pushed, not from a stale local copy. So every survey
syncs before it reads a precondition, and a run starts only when that sync ended `synced` (T-FRESH).
There is no mode in which a clone that cannot reach the remote goes on with what it holds: it could not
see a teammate's settlement, a take-over or a raised plugin floor, and a claim is made by a push in any
case. What this costs is the time in which the remote cannot be reached.

A clone is never refused for being behind: it catches up in the same step. The briefing never fetches,
because it runs inside a hook's time limit; it prints the last sync's outcome from the clone record
([[record:clone]]).

## How two clones' state combines

The layout was already one file per record ([[topic:state-layout]]), and with claims each unit has one
writer at a time, so two clones' commits normally change different files. What this clone publishes is
first committed by the flush, through the mount's own index. Only the combined commit is built without
touching the mount, and the mount is then fast-forwarded to it. That is why state can be combined while
an agent has uncommitted files there: a fast-forward keeps the files it does not touch. It also refuses
over a file that is already there, even one with the same content, which is why a claim or a minted
file is never written in the mount first and reaches it with its commit (T-PUBLISH, T-MINT).

The sync decides every path itself, from two listings of the paths each side changed since the merge
base, before it builds anything (T-COMBINE). The listings are read with rename detection off, because
git's everyday diff reports a claim released and another taken as one renamed file. It does not hand
the tree to git's merge, because git merges by line: two edits to
different lines of one decision record come out as a valid record that neither clone wrote. Records are
therefore never merged, only chosen; text a person or an agent wrote is merged one file at a time.

This replaces the rebase of [[topic:state-commits]], which needs a clean worktree and so was limited to
moments when the run lock was held. A sync also runs outside a run, for a settlement or a claim.

## The conflicts that remain

Each ends in a named outcome that a person sees. None is settled silently.

| Both sides changed | Decided by | Why |
|---|---|---|
| A claim | T-PUBLISH | The push is the claim: the attempt that lost never became one |
| A record of a unit another clone now holds | T-HOLD | The holder is the one writer |
| A decision's settlement | T-SETTLE | First pushed is first settled, and only a published settlement is acted on |
| `notes.json` | T-COMBINE | Notes are independent entries a person asked for, so both sides' are kept |
| Any other record code writes | T-COMBINE | No rule can merge two JSON records, so one stands whole and the other is kept aside |
| A text a person or agent wrote | T-COMBINE | A person's words are merged by git or left for the person, never dropped |

"Set aside" means saved under `.factory/local/aside/`, listed in the clone record, and shown by the sync
result, `vsdd status` and the briefing with the author of the version that stands.

## When someone edits state by hand

An artifact may be edited by hand: a person uses an editor or `/vsdd-factory:author`. The edit is checked
when it is flushed and again on every clone that fetches it, and an approval bound to the file reads
`stale` everywhere, as it does on one clone ([[topic:digests]]). A record edited by hand, or a commit
made on the state branch without the plugin, is checked when it arrives and listed as made outside the
factory (T-INCOMING). What no program can tell is whether a record that passes its check is true.

What arrived is measured from the last head of the remote that the mount took in. A fetch that only
judges a push does not move that mark: the commits that beat the push would otherwise reach the mount
with the next fast-forward and never be checked (T-HISTORY; [[topic:team-git-facts]], fact 20).

## Remotes

- **No name.** The remote of a branch is the one its upstream names, which git records per clone. One
  clone may call it `origin` and another `upstream`. The tool takes the remote from there and nothing
  else, and writes every ref out itself, in the two forms T-REMOTE gives. A push goes to the branch's
  own name: a story branch cut from the integration branch's remote head would otherwise inherit that
  branch as its upstream, and a push "to its upstream" would land the story on the shared branch with
  no pull request. A fetch goes into the remote-tracking ref and never into a local branch: the state
  branch is checked out in the mount and a target branch is the person's own, and git either refuses
  such a fetch or moves the branch under whoever has it ([[topic:team-git-facts]], fact 15).
- **One shared repository.** A claim excludes only among clones that push to the same repository. Every
  clone therefore pushes its state branch and its story branches to the one repository the team shares.
  A way of working in which a person can push only to a fork of their own is not supported: such a clone
  cannot take a claim the others see. No program can measure that two clones mean the same repository,
  so `setup` asks which remote is the shared one when a clone has several (T-JOIN), and `vsdd doctor`
  prints the addresses for people to compare.
- **No special refs.** Claims and state live on the state branch and stories on `story/<id>`. The design
  asks nothing of the remote beyond accepting pushes to branches, and creates no other kind of ref there.
- **Credentials.** Any network call may fail for want of them. The tool never asks, stores or prints
  one. Setting `GIT_TERMINAL_PROMPT=0` is not enough: it does not govern ssh, an askpass program or a
  credential helper. Every network call therefore runs with standard input from `/dev/null`, in a
  session of its own so that it has no terminal to ask on, with `GIT_ASKPASS` and `core.askPass` empty,
  `SSH_ASKPASS_REQUIRE=never` and `credential.interactive=false`, under a time limit at which the whole
  process group is killed. What would have asked fails, and the failure is reported with git's line.
- **A remote with its own rules.** Decision D33 leaves who may push, approve and merge to the
  repository. The factory needs two things of those rules and says so in `vsdd doctor`: every clone
  can push the state branch directly, and state commits are unsigned (decision D2), so a rule that
  requires a pull request or a signature on the state branch declines every push and no claim can be
  taken there. A declined push is told from a lost race by measurement and is not retried (T-PUSH). The
  repository should forbid forced pushes and deletion of the state branch; where it does not, the sync
  notices both and stops (T-HISTORY). Such a rule does not refuse the tool's own push: it names the
  head it was built on so that it can only add to that head, and a remote that forbids every update
  but a fast-forward took it (fact 16). What that buys is that a branch moved back between a clone's
  fetch and its push is noticed too, where a plain push would quietly put the removed commits back.
- **Clocks.** Nothing compares a time from one clone with a time from another (T-ORDER).

Versions and settings are in [[config:team]].

## A state branch that grows

The state branch's history is the record of who held what and when, so nothing prunes it, and every
claim and every combine adds a commit. The sync's cost is kept independent of that length: it lists
before it fetches, checks the tree that arrived and not each commit (T-INCOMING), and combines from two
diffs. The proof measures a sync on a history of 50,000 commits, and `vsdd doctor` reports the branch's
length ([[packaging:proof-team]]).

## One clone, or no remote

Which of the checks this file explains act while `team.enabled` is false is stated once, at the head
of [[table:team-checks]]. With no remote there is nothing to sync and nothing changes.
