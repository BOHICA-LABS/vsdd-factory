---
kind: topic
id: team-git-facts
package: M1
sources: [decisions.md#D33]
---

# The git facts team use rests on

Built when the build widens past the thin slice ([[packaging:build-plan]]). Decision D33 binds team use
to what git can do, so each mechanism of [[topic:team-sync]], [[topic:team-claims]] and
[[topic:team-delivery]] rests on a fact below, and nothing rests on one that is not here.

Each was run on 2026-11-24 in scratch repositories: one bare repository and two or three clones. The
clones that stand for clones running the factory had remotes not named `origin`. Each was run with git
2.34.1 on Linux, which is the floor (T-VERSION in [[table:team-checks]]), and all but fact 10 with git
2.50.1 on macOS as well; where the two versions differ the fact says so. Fact 10 was run on Linux
with OpenSSH 8.9 against a throwaway sshd, and only its last sentence on macOS too. Package M1 keeps
every fact as a test, and CI runs those tests on Linux with a git at the floor.

1. **A refused push says little.** A push the remote does not take exits with status 1, and
   `git push --porcelain` prints a line beginning `!`. The reason on that line follows the timing, not
   the cause. `[rejected] (fetch first)` appears when the remote had moved before the push connected.
   The loser of two pushes that were both connected got `[remote rejected] (failed to update ref)`, in
   20 rounds of 20 with a pre-receive hook that sleeps. A push the remote's own rule declined got
   `[remote rejected] (pre-receive hook declined)`. What does tell them apart is a fetch afterwards:
   after a lost race the remote's head is no longer the commit the push was built on, and after a
   declined push it is.
2. **One winner.** Of two clones that push different commits to one branch at the same moment, the
   remote accepts exactly one: 20 rounds, one winner each, never both and never neither.
3. **An answer can be lost.** A push whose client was killed at a two-second limit, while a
   post-receive hook on the remote was still running, had been accepted. After a fetch,
   `git merge-base --is-ancestor <pushed commit> <remote head>` exits 0.
4. **Upstreams.** `git for-each-ref --format='%(upstream:remotename) %(upstream:remoteref)'` gives a
   branch's remote and ref whatever the remote is called. A branch cut from a remote-tracking branch, by
   `git branch` or by `git worktree add --track -b`, gets that branch as its upstream, and a plain
   `git push` under `push.default=upstream` then moved the integration branch. A branch created at a
   commit id, or with `--no-track`, has no upstream. `git push -u <remote>
   refs/heads/<b>:refs/heads/<b>` moved `<b>` alone and set the upstream to it.
5. **A commit without a worktree.** A commit can be built on the remote's head with a temporary index
   and `git commit-tree`, and pushed by its id, without moving the local branch or touching its
   worktree. After a refused push there is nothing to undo.
6. **Merging one file, and why not the tree.** `git merge-file -p` merges one file from three versions:
   exit 0 when the changes lie in different parts, a positive status and conflict markers when both
   changed one line. A merge of the whole tree is not used. `git merge-tree --write-tree`, which git
   2.34 does not have, merged two edits to different lines of one JSON record with exit 0 into a record
   neither side wrote. The sync reads which paths each side changed instead, with rename detection off.
   One commit removed `claims/story.S-1.json` and added `claims/story.S-2.json`, each written one key to
   a line. `git diff --name-status` and `git show --name-status` listed the two as one line, `R092` with
   both paths. `git diff-tree -r --no-renames --name-status` listed a `D` and an `A`. `git diff-tree -r`
   listed them apart without that option too, whatever `diff.renames` was set to, and joined them only
   with `-M`. With the two claims written on three lines each, and so less alike, every one of these
   commands listed a `D` and an `A`: whether a rename is reported follows how alike the two files are.
   With `-z` a path that holds a space and a non-ASCII letter is listed whole; without it git quotes it.
7. **Fast-forward over uncommitted files.** `git merge --ff-only` in a worktree keeps an uncommitted
   file it does not touch. It refuses, with nothing changed, when it would overwrite one, and also when
   that file's content equals the target's, whether the file is tracked or not.
8. **Ancestry.** `git merge-base --is-ancestor` answers with exit status 0 or 1. In a shallow clone,
   where `git rev-parse --is-shallow-repository` prints `true`, it exited 128 for a commit outside the
   shallow history, and a first-parent count gave 2 where the true count was 27.
9. **A branch rewound or deleted on the remote.** After a forced push that moved the state branch back,
   once to its parent with another commit on top and once to a bare ancestor,
   `git fetch <remote> +refs/heads/<b>:refs/remotes/<remote>/<b>` exits 0 and reports a forced update,
   and the head seen before is no ancestor of the fetched head. The `+` is what does it: the same fetch
   without it exits 1 with `[rejected] (non-fast-forward)` and leaves the remote-tracking ref where it
   was. For a deleted branch `git ls-remote --exit-code <remote> refs/heads/<branch>` exits 2,
   where a remote that is not there gives 128; `git fetch` gives 128 for both; and a push creates the
   branch again.
10. **Prompts.** With `GIT_TERMINAL_PROMPT=0` git does not ask for a credential on the terminal and
    fails. It still runs an askpass program named by `GIT_ASKPASS` or `SSH_ASKPASS`, unless `GIT_ASKPASS`
    and `core.askPass` are empty. The variable does not govern ssh. Started from a terminal with
    inherited standard input, `git ls-remote` over ssh to a host with an unknown key printed ssh's
    question on the terminal; when git was killed at a four-second limit the ssh process stayed.
    Started with standard input from `/dev/null` in a session of its own, the same call failed at once,
    with "Host key verification failed" and, where a password was wanted, "Permission denied". With
    `SSH_ASKPASS` and `DISPLAY` set ssh then ran the askpass program, and with
    `SSH_ASKPASS_REQUIRE=never` it did not. The git manual names `credential.interactive=false` as the
    setting that keeps git from asking and that some credential helpers honour. A credential helper
    that waits is ended only by the time limit, and only when the whole process group is killed:
    killing git alone left the helper running.
11. **Identity.** With nothing configured, `git -c user.useConfigOnly=true var GIT_AUTHOR_IDENT` exits
    128, where `git config user.name` exits 1. With `GIT_AUTHOR_NAME` and `GIT_AUTHOR_EMAIL` in the
    environment it prints that identity, also when the configuration names another person.
12. **A forced push after a fetch.** A teammate pushed a commit to a story branch, and the holder
    fetched and rewrote its own branch. `--force-with-lease=refs/heads/<b>:<the head the holder had
    pushed>` was refused with `[rejected] (stale info)` and the teammate's commit stayed. A bare
    `--force-with-lease` was accepted and the commit was gone from the remote.
13. **Which commits arrived.** `git rev-list <remote head> --not <pushed head> <local head>` listed
    nothing after the holder's own rebase that was not pushed yet, although the remote head was then no
    ancestor of the local one, and listed exactly the teammate's commit after a teammate's push.
14. **Two repositories.** Two repositories with one history, a mirror standing in for a fork, each
    accepted a push of a claim for the same path.
15. **A fetch must not name a local branch.** `git fetch <remote> refs/heads/<b>:refs/heads/<b>` for the
    state branch, run in the mount where it is checked out, exits 128 on both versions and changes
    nothing. Run from the main checkout, with the branch checked out in the mount, it exits 128 on
    2.50.1 ("refusing to fetch into branch … checked out at …"), but on 2.34.1 it exits 0 and moves the
    branch under the mount, where `git status` then shows the file that arrived as a staged deletion.
    For a target branch that is not checked out it exits 0 and moves the person's local branch; where
    that branch holds a commit of its own it exits 1 with `[rejected] (non-fast-forward)`.
    `git fetch <remote> +refs/heads/<b>:refs/remotes/<remote>/<b>` exits 0 in each of these cases and
    moves the remote-tracking ref alone: the local branch, `HEAD`, the index and the files stay, whether
    the branch is checked out or not. It also works in a clone whose configured refspec does not cover
    the branch (`git clone --single-branch`). `git fetch <remote>` with no refspec brought in every
    branch the remote has.
16. **A push that names the head it was built on.** A clone built a commit on the head H of the state
    branch, and the remote's branch was then moved back to H's parent. A plain
    `git push <remote> <commit>:refs/heads/<b>` exits 0, as a fast-forward from the rewound head, and
    the remote holds the removed commit again. The same push with
    `--force-with-lease=refs/heads/<b>:<H>` exits 1 with `[rejected] (stale info)` and leaves the remote
    as it was; a fetch then shows a head that does not contain H. With the remote still at H the push
    is accepted. With the remote moved forward by another clone it is refused in the same words, and
    the fetch shows H contained and the pushed commit not, which is a lost race. Of two clones that
    push at once, each naming H, while a pre-receive hook sleeps, exactly one is accepted, in 20 rounds
    of 20; the other gets `[remote rejected] (failed to update ref)`. A remote with
    `receive.denyNonFastForwards` and `receive.denyDeletes` set takes the push. A pre-receive hook that
    declines gives `[remote rejected] (pre-receive hook declined)` with the remote's head still H. A
    push whose client is killed at two seconds while a post-receive hook sleeps had been accepted, by
    the test of fact 3. Where the branch was moved back and given another commit, a plain push is
    refused with `[rejected] (fetch first)`, and the fetch shows neither the pushed commit nor H
    contained.
17. **The holder's own rebase, pushed or not.** A holder pushed a story branch at head P, rebased it
    onto a target that had moved, and pushed with the lease of fact 12 while a post-receive hook slept
    and the client was killed at two seconds. The remote had taken the push: after a fetch the pushed
    head is contained in the remote head. That remote head is the holder's own head, the listing of
    fact 13 is empty, and yet P is no ancestor of it, so that last test alone does not show that
    somebody else replaced the branch. Where the remote never took the rebase and a teammate pushed a
    commit onto P, the listing of fact 13 gives that commit alone, but merging it into the rebased
    branch exits 0 and leaves each of the holder's commits in the history twice. Putting the branch
    back at P first and then fast-forwarding gives each once, with the teammate's commit under its own
    id. `git cherry <P> <local head> <merge base of the local head and the target>` marks each rebased
    copy `-` and a commit made after the rebase `+`.
18. **The ref rule, walked.** One full sync, one claim taken by two clones at the same moment and one
    hand-off of a story between them were run command by command, with the two forms of T-REMOTE
    ([[table:sync-checks]]) and the push of fact 16 and no other fetch or push, three times on each
    version, the claim won by either clone. Every command exited as the design says. The mounted state
    branch moved only by the flush's commit and by `git merge --ff-only` in the mount, which kept an
    agent's uncommitted file. The loser of the claim had nothing to undo. The new holder's story
    branch, created at the fetched commit id, had no upstream until its first push with `-u`, and then
    its own name; the old holder's later push was refused with `[rejected] (fetch first)`. The remote's
    integration branch, and each clone's local one, checked out in one clone and not in the other, were
    where they began. The loser of the claim listed what arrived from the head its push was built on
    and found the winner's claim there (fact 20).
19. **A listing matches at the end of a name.** With `story/S-6.02` and `keep/refs/heads/story/S-6` on
    the remote and no `story/S-6`, `git ls-remote --exit-code <remote> refs/heads/story/S-6` exits 0 and
    prints one line, for `refs/heads/keep/refs/heads/story/S-6`; `story/S-6.02` is not listed. With no
    such branch either it exits 2. So the exit status alone does not say that the branch asked for is
    there: the ref name on the line does.
20. **What arrived between a push's base and the head that beat it.** Clone B stood at head H of the
    state branch. A commit made by hand added a record, another clone pushed a claim on top of it, and
    B's push of a claim built on H, naming H as in fact 16, was refused with `[rejected] (stale info)`.
    After B's fetch the new head F contains H and not B's commit. `git diff-tree -r --no-renames
    --name-status -z H F` lists the record and the other clone's claim; the same listing from F to F is
    empty. B's claim built again on F was accepted, and `git merge --ff-only <that commit>` in B's
    mount exits 0 and puts the record made by hand into the mount, though B had made no listing that
    names it. What arrived is therefore listed from H, the head the mount last took in, and never from
    a head that a fetch made to judge a push has seen. The same holds after a lost answer. B's push was
    taken while a post-receive hook slept and its client was killed at two seconds, and a commit made
    by hand was pushed on top before B fetched. The fetched head F contains B's commit and is another
    commit. `git merge --ff-only <B's commit>` in the mount moves it to that commit and not to F, and
    the listing from B's commit to F names the file made by hand, where the listing from F to F is empty.
21. **Whether a story branch was pushed is answered without the network.** For a story branch created
    at a commit id and never pushed, `git for-each-ref --format='%(upstream:remotename)|%(upstream:remoteref)'
    refs/heads/story/<id>` prints `|` and exits 0, also while the remote's address leads nowhere, where
    `git ls-remote` exits 128. For a branch the clone does not have it prints nothing. After the first
    push with `-u` (fact 4) it prints the remote and the branch's own name. It still does after the
    branch was deleted on the remote and the remote-tracking ref removed, where `git rev-parse -q
    --verify story/<id>@{upstream}` exits 1; the listing then exits 2 and a fetch of the branch 128.
    A clone that has not fetched the branch does not hold the commit another clone pushed:
    `git cat-file -e <id>^{commit}` exits 128 there, and 0 after the fetch.
