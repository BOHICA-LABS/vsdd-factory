---
kind: command
id: sync
package: M1
sources: [decisions.md#D33]
---

# `vsdd sync`

Built when the build widens past the thin slice ([[packaging:build-plan]]). Decision D33.

Brings this clone's state and the remote's together: fetch, combine, push. It refuses out loud when it
cannot.

## Syntax

`vsdd sync [--label <text>] [--json] [--wait <seconds>]`. A job command, so a second call attaches to
a sync that is running, and the `decisions` rule can start one without waiting; how the job is keyed,
and why a finished sync is never served to a later call, is in [[topic:long-commands]]. `--label` sets
the clone's label ([[record:clone]]).

## What it does

It is the command face of `vsdd.state.sync(root, reason, paths)`, the one function through which state
leaves or enters a clone. The survey, the checkpoint, delivery, the claim commands, `vsdd decide` in a
terminal and `vsdd status --fix` call the function; a person or the main session runs the command when
the briefing says state is unpublished or behind. Each check named below is a row of
[[table:sync-checks]], [[table:team-checks]] or [[table:work-claim-checks]]; the reasons are in
[[topic:team-sync]].

In order:

1. The preflight (T-VERSION, T-REMOTE, T-WHO), and `local/sync.lock`, a file lock that is free again the
   moment the process holding it ends or dies: one sync runs in a clone at a time.
   A second waits for the first, inside its own time budget, and then runs its own; when the budget
   ends first it answers `busy` and has changed nothing. A command that takes a claim, mints an id or
   opens a cycle holds the same lock for the whole of its act, the syncs it calls included, so no
   other sync of the clone runs in the middle of one (T-PUBLISH, T-MINT, T-RELEASE).
2. Flush, under `flush.lock` (T-LOCAL). A sync is **full**, and flushes everything pending, when its
   caller is a survey, a checkpoint, `vsdd status --fix` or `vsdd sync` and no run but the caller's own
   is live on this clone. Every other sync is **partial**: its flush commits only the paths its caller
   names, and may name none. A settlement names the decision's record; `vsdd claim release` names the
   claim and every changed path its scope covers; a command that opens a cycle inside a run names the
   new cycle record and the removal of its `open` claim; the sync the `decisions` rule starts names
   the record of the decision it settled. The half-written work of a live run is therefore published
   only by that run's checkpoint.
3. List the remote's state branch. The sync ends `synced` here, without a fetch, only when the later
   steps have nothing to do: the listed head is `sync.remote_head`, the mount's head is that commit
   too, `sync.unknown` is empty and, in a full sync, this clone holds no claim that T-RELEASE has a
   sync remove. Otherwise fetch, into the remote-tracking ref and nowhere else (T-REMOTE), and judge
   the fetched head (T-HISTORY).
4. Under `flush.lock`: check what arrived since `sync.remote_head` (T-INCOMING), settle the commits
   whose push had got no answer (T-PUSH), combine (T-COMBINE), withhold what another clone holds
   (T-HOLD, T-SETTLE), remove the claims a full sync removes (T-RELEASE), fast-forward the mount and
   then, not before, write the clone record with the fetched head as `sync.remote_head` (T-HISTORY).
5. Push the mount's head, naming the head it was built on, and judge the answer (T-PUSH). A lost race
   goes back to step 3 with `sync.remote_head` as it was, so step 4 checks all that arrived.
6. Under `flush.lock`: write the outcome to the clone record ([[record:clone]]).

`flush.lock` is held for the local steps only, each of which takes about as long as a flush, and never
across a listing, a fetch or a push. The record stores and the hook that settles a decision take the
same lock with a wait of a few seconds, and the network can take far longer. If the mount's head moved
between two local steps, the fast-forward fails without changing anything and the step is repeated.

While team use is off the command does the state push of [[topic:state-commits]] and nothing else of
the above. Which checks act on that push all the same is stated at the head of [[table:team-checks]].

## Time, and nothing that waits for a person

Each network call has a time limit of 30 seconds. Called inside a foreground command (the survey, the
checkpoint, `vsdd decide`, the claim commands) a sync has 60 seconds for its wait for `local/sync.lock`
and all its network calls together, retries included, so that the command stays inside the Bash tool's
default limit (T-FRESH) also while a long sync job holds the lock. Run as this command it is a job, with
ten minutes for its wait for the lock and for each call, which is what a clone far behind uses. Every
network call is made so that it cannot wait for a person ([[topic:team-sync]], "Credentials"); one that
would have asked fails, and the failure is `unreachable` with git's own line.

## Who may call it

The main session and a person in a terminal. The clerk never needs it: the survey and the checkpoint call
the function themselves.

## Writes

State commits on the state branch (the flush's and, when there is one, the combined commit); the
remote-tracking ref of the state branch; the clone record; files under `.factory/local/aside/`. It
never touches a product branch.

## Output and failure cases

`ok`, `state`, `remote`, `head`, `pulled` and `pushed` (counts of commits), `aside` (what was set aside,
with whose version stands), `lost` (claims this clone no longer holds), `hand_edits` (what T-INCOMING
reports), `settled` and `error`. `settled` lists each commit whose push had got no answer and that this
sync measured: the claim or the minted file it carried, and whether the remote had taken it. The clone
record keeps the list, so `vsdd status` and the briefing show it too ([[record:clone]]).

| `state` | Meaning | What a person does |
|---|---|---|
| `synced` | The clone and the remote hold the same state | Nothing |
| `unpushed` | Combined here; the push got no answer and the remote could not be asked | Run it again |
| `unreachable` | The listing or the fetch failed or ran out of time; nothing changed | Restore the network or the credential; `vsdd sync` for a clone far behind |
| `missing` | The remote has no state branch; nothing changed | Restore the branch with the push the message gives |
| `rewritten` | The remote's state branch lost commits this clone built on; nothing changed | Push the old head back, or move the mount onto the remote's history |
| `diverged` | A text conflict, or a combined file that fails its check; nothing changed | Merge in `.factory/` with git, then run it again |
| `blocked` | The mount could not move without overwriting an uncommitted file, which is named | Correct or remove that file |
| `refused` | The preflight, T-SOLO or T-SWITCH refused, or the remote did not take the push though nobody else had moved the branch; `error` names which, with the line to change or git's own line | Change that line, or the rule of the remote |
| `busy` | Five pushes in a row lost to other clones, or another sync of this clone held the lock until this call's time ran out | Run it again; `vsdd sync` attaches to a sync that is running |

`ok` is true for `synced` only.
