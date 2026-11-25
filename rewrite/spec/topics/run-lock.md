---
kind: topic
id: run-lock
sources: [state.md#2.11, BLUEPRINT.md#4.13]
---

# The run lock and liveness

The run lock is `.factory/local/lock.json` ([[record:lock]]).

```json
{ "locks": [ { "scope": "acme-api:develop", "run": "r-20261126-1402-7q", "workflow": "wave", "subject": "W-02",
               "session": "0c1d…", "taken": "2026-11-26T14:02:00Z", "seen": "2026-11-26T14:31:10Z" } ] }
```

## Taking and releasing

- `vsdd survey` takes it once its preconditions pass ([[table:preconditions]]), scoped to the repository and
  the branch the run can write. Every run that can merge into the integration branch takes the same scope,
  so a standalone `story` run and its wave exclude each other. A second run on the same scope gets
  `ok: false` with the holder, and `stale_at`, the moment the holder goes stale: `seen` plus
  `state.lock_stale_minutes`, not the last-seen time itself. The take is atomic: a file lock on
  `local/lock.flock` is held around the read, the check and the write.
- `vsdd checkpoint` releases it. So does the hook runner when the holder's session ends, which covers a
  person closing a session in the middle of a run.
- A run is live from its survey until its checkpoint, or until its lock is released or taken over. The
  command that ends it writes `ended` in the run record ([[record:run]]). The step store and every
  pipeline command given `--run` refuse a run that is not live, so a run that was taken over can store and
  measure nothing more. A stale lock does not by itself end its run: it only lets the next survey take it.
- Re-entry. A survey from the session that holds the lock, for the same workflow and subject, is not
  refused. When the holding run has no step record, the survey returns that same run, so a clerk's retry
  after a lost answer changes nothing. Otherwise the holding run was interrupted: the survey ends it and
  takes the lock for a new run, which continues from what is on disk.
- The `spike` workflow, `discover` and a report-only `maintain` take no lock.
- It is local, so it does not stop two machines. They meet at the state push: the second fetches, rebases
  and retries ([[topic:state-commits]]), and a real conflict in the same record is reported to a person.

## Liveness

- The holder counts as alive while `seen` is recent. The hook runner owns liveness: it refreshes `seen`, at
  most once a minute, on a call that belongs to the locked run, and on no other. Four kinds of call belong
  to it: a `vsdd` call whose arguments carry `--run` with the lock's run; a `StructuredOutput` call whose
  `step.run` is that run; a call whose target path or effective directory ([[topic:rule-model]]) lies in a
  worktree the run owns; and a call made inside a subagent of the holder's session. A call by the main
  session itself never refreshes it, so a run that stopped inside a session that stays open goes stale
  like any other.
- After `state.lock_stale_minutes` (default 20 — [[config:state]]) without one, the lock is stale, its
  holder counts as dead, and the next survey takes it over. A foreground command can run for ten minutes at
  most, so a live run is not mistaken for a dead one.
- The runner does this through two core duties wired beside the rules, not through a rule (the finding
  P3BLD-14 moved them out of lifecycle-log): the refresh on every guarded `PreToolUse` call, passing the
  call's facts to `lock_seen`, and the release on `SessionEnd` whatever its reason, giving the session's
  entries to `lock_release` with a 0.5 s wait inside the event's deadline. The release logs each released
  entry as `lock.released` with the run and the session-end reason; when the store signals a busy flush,
  it logs what it can and releases nothing by other means — the lock then goes stale by time. A fault in
  either duty is logged as `runner.liveness_error` and never blocks a call or changes its output.
- After a hard crash there is no session end, so the lock stands until it goes stale. The briefing and
  `vsdd status` then say which session holds it, when it was last seen, and what a person runs if they know
  that session is gone: `/vsdd-factory:status --fix` in a session, which releases a stale lock, or
  `vsdd lock release` ([[command:lock-release]]), which also releases a lock the caller's own session
  holds. `vsdd status` shows the lock at any time; there is no separate `lock status` command.

The state tests hold the re-entry rule with two cases: a run interrupted straight after its survey, and one
interrupted after its first step; in both, the same command run again in the same session continues the
work with no other command.

## Caller identity

The survey refuses to start a run when its call has no marker, because that means the hooks are not active
for this session; without that check a `vsdd` process with two sessions open could take the other session's
marker and record the lock under the wrong session. The caller marker itself — how `vsdd` learns who is
calling, and why one marker is written per call — is [[topic:caller-identity]] and [[record:caller-marker]].
The same marker gives `vsdd spike open` the runner's agent id, `vsdd env start` the agent to clean up after,
and `vsdd doctor` the proof that hooks are live.

## Liveness by process: the open offer

Lock liveness by process, rather than by heartbeat, is an open offer to the person running the project:

> The run lock would record the Claude Code process that holds it and be stale the moment that process is
> gone. It saves the once-a-minute write on the hook's hot path, the `state.lock_stale_minutes` key, and the
> 20-minute wait after a crash. It loses nothing if probe 29 passes. Take it if probe 29
> passes on macOS and Linux with the Bash sandbox on; keep the heartbeat otherwise. Until you say, the
> heartbeat is the design.

Probe 29 is in [[doc:platform]].

## A story branch on the remote

`vsdd story deliver` pushes the story branch and opens the pull request with a trailer in its body naming
the story (`Factory-Story: S-7.03`). The survey treats a remote story branch in one of two ways:

- It is **adoptable** when its pull request carries that trailer and the tracked state shows the story
  undelivered in the open cycle: the survey fetches it and continues the delivery from the open pull
  request. That is this machine's own branch after a crash, and equally a fresh clone picking up where
  another left off, which is safe because the merge is one idempotent, head-matched act.
- Otherwise it is **owned elsewhere**: the remote head is neither the local branch's head nor an ancestor of
  it, no local run record names the branch, and it has no pull request the tool opened. The survey then
  reports the story as a blocked unit with kind `conflict`, which the checkpoint turns into a `blocked`
  decision if it persists.

Writes to machine records are atomic (write a temporary file, then rename) under `flush.lock`, so two
sessions that legitimately run at once (a wave and a report-only sweep) cannot corrupt a record.

## Team use (decision D33; built when the build widens)

Not on the thin slice path. The lock stays on the clone and keeps one clone to one run; nothing in it
changes. What two clones see of each other is the claim ([[topic:team-claims]]): the survey takes it
after the lock, and a claim another clone holds refuses the run as a held lock does. Where this file
says two machines meet at the state push and the second fetches, rebases and retries: with team use off
that push now stops and says so, and with it on they meet at the claim and the push is the sync
([[table:sync-checks]], T-SOLO).

The adoption rule for a remote story branch gives way to the claim, which is why it is built behind one
function that is given the claim's state. A story branch belongs to the clone whose claim names it; a
story another clone holds is listed as held elsewhere, not as a `conflict`; and commits that reach a
branch from elsewhere are hand commits ([[table:team-delivery-checks]], T-BRANCH).
