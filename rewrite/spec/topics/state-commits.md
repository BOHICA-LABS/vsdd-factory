---
kind: topic
id: state-commits
sources: [state.md#who-commits-state-and-when, BLUEPRINT.md#5.2]
---

# Who commits state, and when

Nobody is asked to commit state. Flush does it. It is the function `vsdd.state.flush(root, reason)`. There is
no `vsdd flush` command: nothing needs one.

| Called from | When |
|---|---|
| `vsdd survey` | At its start, after preconditions pass, for anything pending. In practice this is a decision a person settled since the last checkpoint: settling writes the record and nothing else, so that it fits inside the hook's time limit. |
| `vsdd checkpoint` | After it writes closures and decisions. This is "saves state" in the pipeline design. It then pushes. |
| `vsdd story deliver`, `vsdd status --fix` | After their own writes. |
| The hook runner on `PreCompact` | Always ([[rule:compaction]]). |

Flush runs at these five points only. `vsdd decide` and plain `vsdd status` do not flush, and `vsdd status`
writes nothing at all. Flushing on every command or on every agent stop would make hundreds of state commits
per wave. Nothing is lost in between: the files are on disk.

## What flush does

1. Takes `.factory/local/flush.lock`. A second flush waits up to 5 seconds and then leaves the work to the
   holder.
2. Checks the mount. If `.factory/` is not a worktree on the state branch it changes nothing and reports
   `partial` ([[topic:state-directory]]).
3. Validates each changed or new file against the registry (the same check as the write hook
   [[table:artifacts]]). A file that fails is left uncommitted and reported, so something written through
   the shell cannot slip in. It reads the changed paths through the state core's `changed_paths`
   helper, which passes `-uall` to `git status`, so an untracked directory is never collapsed to its
   directory name (finding S3B-state-core-3). The record kinds flush validates are its own table
   (`vsdd.state.commits.record_kinds`), and a tracked file under the record folders — the cycles tree
   and `notes.json` — that no row of that table claims is refused and named, not committed unchecked:
   a new tracked record kind must register its validator there in the same step as it first lands in
   the state tree (S3B-records-2).
4. Runs `git -C .factory add` on the valid paths and `git -C .factory commit` with the message
   `factory: <kinds and identifiers changed>` and two trailers, `Run: <run id>` and `Code-Head: <head of
   the integration branch>`. The path-taking git calls (add, `rm --cached`, commit) run with literal
   pathspecs (`GIT_LITERAL_PATHSPECS=1`), because record names may carry glob characters and a pattern
   would widen the commit to files flush just rejected (finding step3-records-fix-3-C1-1), and they force
   the other global pathspec settings off (`0`), because literal is incompatible with an inherited
   `GIT_ICASE_PATHSPECS=1` or `GIT_GLOB_PATHSPECS=1` and every flush with changes would fail (finding
   step3-records-fix-3-C2-1). The commit runs
   with an empty hooks path and without signing, because the
   product's git hooks are written for product code and would run in a tree that has none.
5. Never touches a product branch, a product file or the person's index.

It is idempotent, makes no network call, and takes well under 2 seconds on the 2,000-contract corpus of the
old engine. The hook design gives it 20.

It never raises: every failure comes back as the result, with the state `failed` and the cause in
`error`. A flush called from a sandboxed Bash call cannot write under `.factory/.git` (the platform
probes of 2026-11-24: the Bash sandbox refuses every write under `.git/worktrees/`), so a git failure
of that shape comes back with a cause that names the sandbox; a flush run from a hook is not
sandboxed (finding PR4C-08).

## Push

`vsdd checkpoint` pushes the state branch when `state.push = true` (the default when the repository has a
remote — [[config:state]]). If the remote has moved, which only happens when another machine pushed state,
it rebases the state branch onto the remote and retries up to three times, and only while it holds the run
lock ([[topic:run-lock]]), so no agent is writing. A conflict stops the push, changes nothing, and is
reported as a problem in `vsdd status`. Nothing else pushes state, so nothing replaces
`factory-cas-push.sh` as a separate tool.

## Team use (decision D33; built when the build widens)

Not on the thin slice path. With team use on, the push above is the sync ([[command:sync]]): it also
runs at the start of every survey and after a settlement, and it combines by a commit built without
touching the mount where this file says rebase. It holds `flush.lock` for its local steps only and
never across the network, so a flush, a record write and a typed settlement wait no longer for it than
they do for another flush. A flush can be limited to named paths. A state commit then also carries the
trailers `Clone:` and `Plugin:`, and the flush refuses a clone for which git has no identity
([[table:team-checks]], T-WHO, T-LOCAL). State commits stay unsigned on a team, as step 4 says
([[topic:team-sync]], "A remote with its own rules").

With team use off, a push that finds the remote moved stops and says so, where this file says it
rebases and retries ([[table:sync-checks]], T-SOLO).
