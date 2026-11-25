---
kind: packaging
id: proof-team
package: M5
sources: [decisions.md#D33]
---

# The team proof (M5)

Built when the build widens past the thin slice ([[packaging:build-plan]]). Decision D33:
"its proof is a test on two clones of one remote". The capability is [[topic:team-use]].

## The harness

One bare repository and two clones of it, A and B, made by a fixture from the sample project
([[packaging:sample-project]]). The remotes are named `shared` in A and `upstream` in B, so no test
passes by a remote being called `origin`. The clones have different git identities and their own clone
records, and B's clock runs three hours behind A's in every scenario. The real commands run in process in
each clone; pull requests go through the delivery double, which both clones share. A third working copy,
C, has no plugin and commits with plain git. The harness can put a hook on the bare repository that
delays, declines or outlasts a push, which is what makes a race, a declined push and a lost answer
happen every time and not by luck. No scenario needs a model: what is proved is the tool.

The suite lives in `tests/team/` and is part of the Python test layer ([[packaging:tests-python]]), so CI
runs it wherever it runs that layer ([[packaging:ci]]), on Linux with a git at the floor.

## Scenarios, each with what must be true at the end

Every check of the four tables is run by at least one scenario; its name is in brackets.

**Claims and the sync**

1. *One story, two clones at once*, twenty times, with both pushes connected. Each time exactly one
   survey starts a run; the other names the holder and has written nothing; the remote holds one claim
   [T-CLAIM, T-PUBLISH].
2. *Two stories at once.* Both clones end holding their story: the loser of the push retried and was not
   reported as refused. Then the push that beats B's is C's, a commit that breaks the closure of a
   third story: B's `vsdd status` names that record with C's commit and author, and B holds its story.
   Then B's push is taken while a hook outlasts its limit, and C pushes such a commit on top before B
   can ask: B holds its claim, its `sync.remote_head` is its own commit, and its next sync names the
   record [T-PUSH, T-HISTORY, T-INCOMING].
3. *A push the remote declines.* `refused` with git's line, one attempt, no claim [T-PUSH].
4. *A lost answer.* The remote accepts a claim, the client is killed at its limit and the remote then
   cannot be reached. No run starts and the commit is listed as unknown; with the remote back, the next
   sync finds the claim accepted, lists the commit under `settled`, and removes the claim, since nothing
   was begun under it. The same with an `open` claim whose command was killed before it wrote the cycle
   record: the next full sync removes it, and B then opens a cycle [T-PUSH, T-RELEASE].
5. *The remote out of reach.* Every survey on B is refused `unreachable` and writes nothing, also for a
   story B holds; with the remote back the same command starts the run [T-FRESH].
6. *A stale clone.* A checkpoints a delivered story; B, which has not fetched since, surveys and starts
   nothing for it [T-FRESH].
7. *The state branch rewound.* C forces it back past A's claim. Both syncs end `rewritten` and name
   the heads and the vanished commit; B takes no claim; after the old head is pushed back both sync.
   Then C rewinds it between A's opening sync and A's claim push, once to a commit that is no ancestor
   and once to a bare ancestor: both times A's claim is refused `rewritten`, the removed commit is not
   back on the remote, and A's `sync.remote_head` is unchanged [T-HISTORY, T-PUSH].
8. *The state branch deleted.* `missing`; no sync creates it again; after a person restores it the
   sync goes on [T-HISTORY].
9. *A claim removed by hand.* C deletes A's claim. A's sync reports it lost with C's commit, and A's
   run writes nothing more to the unit [T-INCOMING, T-HOLD].
10. *A record edited by hand.* A commit that breaks a closure stops its unit on both clones, with the
    path, commit and author, until a repair arrives; a valid one is listed as made outside the factory
    [T-INCOMING].
11. *One record changed on two clones*, in lines far apart. The remote's version stands whole, the
    other is set aside and named, and no record exists that neither clone wrote [T-COMBINE].
12. *Text edited on both sides.* Different parts of `conventions.md` combine; the same line ends
    `diverged` with nothing pushed or lost, and after a merge by hand `vsdd sync` publishes [T-COMBINE].
13. *Two notes.* Each clone adds one; both are in `notes.json` on both clones [T-COMBINE].
14. *A slow remote.* While a sync's fetch is held open, a decision is settled inside the hook's deadline
    and a record write in a live run does not fail busy [[command:sync]].
15. *A long history.* On a state branch of 50,000 commits the sync's time is measured, reported, and
    inside the survey's budget [T-FRESH, T-INCOMING].
16. *Prompts.* A fetch over ssh to a local sshd with an unknown host key, started from a
    pseudo-terminal, fails at once and leaves no process [T-REMOTE].

**Setups**

17. *Joining.* With team use off B mounts and is given the notice, and its first push after A pushed
    stops. A turns team use on and syncs: the story A had in flight is claimed by A, and B's next push
    joins it with a clone record. The story B had in flight is claimed by B in that same sync, so A's
    next survey lists it under `elsewhere` and does not start it; a story both had begun is reported
    for a person and stays A's [T-JOIN, T-SOLO, T-SWITCH].
18. *Two remotes.* `vsdd init` on a clone whose two remotes both hold the state branch mounts nothing
    until `--remote` names one; where B's remote is a mirror, `vsdd doctor` on A and B prints two
    addresses [T-JOIN, T-REMOTE].
19. *What must not be on.* Turning team use on is refused in `local` mode, with `delivery.ci = "none"`,
    with a `[[repo]]` entry, in a shallow clone and with a host the delivery double says it cannot
    serve [T-SWITCH, T-REMOTE].
20. *Versions.* With `team.min_plugin` above B's version, or a git below the floor, B's survey and push
    are refused with both versions and state untouched; a file from a newer plugin stops its unit with
    "upgrade" as the cause; an unknown `schema` refuses every command [T-VERSION, T-INCOMING].
21. *Identity.* With none, the flush, a claim and a settlement are refused; with one in the environment
    only, they succeed and the records name that person [T-WHO].
22. *One machine's data.* A `factory.toml` with an absolute path is not pushed and the key is named; a
    path under `local/` in a commit C pushed is refused [T-LOCAL].
23. *One clone.* With `team.enabled` false, and again with no remote, the story-ready tests of the slice
    run under a git wrapper that counts calls: no fetch, no listing of the remote, no claim push, no
    clone record [T-SOLO, T-BRANCH].

**Decisions**

24. *A decision across clones.* A's checkpoint opens `finding.<story>`; B's briefing lists it; B settles
    it in a terminal; A's next survey reads it settled, with B's person and clone [T-SETTLE, T-WHO].
25. *Two settlements.* A settles `finding.<story>` while the remote cannot be reached; B settles it
    differently and pushes. On A the decision reads open to `vsdd status`, the route and every gate; with
    the remote back A's sync keeps B's, sets A's aside and names both; no run acted on A's [T-SETTLE].
26. *A settlement during a run.* The run reads the decision as open until the detached sync's push is
    accepted, and that sync's commit holds the decision record alone [T-SETTLE].
27. *Order.* Two `merge` decisions for two stories are opened at the same moment on A and B: no state
    problem is reported, and the latest `spec` decision is the one the links give, with B's clock behind
    [T-ORDER].

**Hand-off**

28. *A clone that is gone.* A checkpoints a story after its green gate and stops. B's survey is refused
    and names A; `vsdd claim take` from a tool call is refused; a person takes the claim. B's
    `vsdd status --fix` and the opening sync of B's survey leave that claim in place with its
    `taken_from`, also where A had pushed nothing [T-RELEASE]. B continues from A's pushed branch, reads the red record's tracked copy and measures green again. When A
    returns, its sync reports the claim lost, its write to the unit is refused, and its push of the
    story branch is refused by git with nothing forced [T-TAKEOVER, T-HOLD, T-HANDOFF].
29. *A hand-over.* `vsdd claim release` in the middle of a story is refused while the worktree holds an
    uncommitted change; after the commit, the remote has the branch and the red record, the integration
    branch is where it was, and B takes the story with a plain survey [T-HANDOFF, T-RELEASE, T-REMOTE].
30. *Minted ids.* A and B each file a change request: two files, two ids, also when one answer is lost.
    The command run again after the sync that settled the lost answer returns the id minted the first
    time and adds no file. A different change request filed in its place gets a new id and a new file,
    and `vsdd status` still lists the settled entry [T-MINT].
31. *Two openings.* A and B each open a cycle: one cycle exists, and the second request is queued
    [T-PUBLISH, T-RELEASE].

**Delivery and commits made by hand**

32. *Two stories in parallel.* Both are merged through pull requests; state holds two closed closures,
    no claim and nothing set aside; each `merged_by` names its person and clone [T-DELIVERY].
33. *A wave on a team.* With four free stories and a limit of two, A's wave run claims two and B takes
    the others; with the agent budget set so that one story fits, A claims one and starts that one; a run with nothing left but stories held elsewhere returns `partial` and names the
    holder; the clone that holds the cycle claim when all are in runs the gate [T-WAVE].
34. *A hand commit on the integration branch.* C pushes one that breaks a test. The wave gate lists it,
    fails on the new failure and offers a fix unit that names it; the verdict stays open until
    `hand:<sha>` is checked, and the reviewer was given its patch; the release packet lists it; the
    commit is on the branch under its own id and nothing reverts it. A second one adds behaviour no
    contract covers, and `spec-gap` opens [T-NOTICE, T-ITEMS, T-GAP, T-NEVER].
35. *A hand commit on a story branch.* C changes a frozen test on A's story branch. A's survey merges it
    in under its own id and records it; green fails naming the test and the commit; after the re-freeze
    the story is delivered [T-BRANCH].
36. *The holder's own rebase.* A's rebase is not pushed and A surveys: no hand commit is recorded and
    nothing is merged twice. Then C pushes between A's fetch and A's forced push: the push is refused,
    C's commit is on the remote, and A's branch is put back and takes it in by a merge [T-BRANCH, T-NEVER].
37. *Merged outside the tool.* The same patch is adopted; a different one closes the story `unreviewed`
    and becomes a `hand:` item of the wave; a pull request the host queues gives `queued`, opens no
    decision and is adopted later [T-DELIVERY].
38. *Team use off.* With one clone in `pr` mode and `team.enabled` false, scenarios 34 and 35 end the
    same way, with no claim, no clone record and no sync, and a delivery records `merged_by` with a
    person and no clone. Before a story's first push no listing names its branch [T-NOTICE, T-BRANCH,
    T-DELIVERY].
39. *A release on a protected branch.* The bare repository declines direct pushes to the integration
    branch: the merge of the hotfix branch back lands through a pull request, is recorded in the release
    closure and is not counted as a hand commit [T-NEVER, T-NOTICE].

**The refs, and what must not come back**

40. *No fetch moves a local branch.* With the integration branch checked out in A's checkout and not in
    B's, a survey, a claim, a checkpoint, a hand-off and a delivery in `pr` mode run on both. Each
    clone's local integration branch, its checkout and the mount's index are where they were, and a
    git wrapper saw no fetch with a destination under `refs/heads/`, none without a refspec, and no
    push of the state branch that did not name the head it was built on [T-REMOTE, T-PUSH].
41. *A claim released and another taken.* Between two syncs of B, A's commits remove one claim file and
    add another like it. B's combine lists a removal and an addition, and both reach B's mount
    [T-COMBINE].
42. *A slow sync job.* While a `vsdd sync` job holds `local/sync.lock` with its fetch held open, a
    survey on the same clone answers `busy` inside its 60 seconds and has written nothing; afterwards
    the same survey starts the run [T-FRESH].
43. *An old sync result.* A sync the `decisions` rule started ends and nobody collects it; C pushes a
    state commit; the next `vsdd sync` runs a sync of its own, and the commit is in the mount. A
    settlement typed while a `vsdd sync` job runs is published by a job of its own
    ([[topic:long-commands]]).
44. *A lost answer on a story branch.* A's forced push after its rebase is accepted while a hook on the
    bare repository outlasts the client's limit: A records the pushed head, puts nothing back, and its
    next survey records no hand commit. With the remote out of reach as well, the next survey finds
    the remote head to be A's own and goes on. With a rebase the remote never took and a commit of C on
    the old head, A's branch is put back, takes C's commit in, and holds each of A's commits once
    [T-BRANCH, T-NEVER].

Every refusal of the four check tables also has its own failing and passing case in the package that
builds it; these scenarios run the checks together.

## One run through the platform

One headless run at the story tier ([[packaging:e2e-story-tier]]): two sessions in two clones deliver two
stories of one wave at the same time, and the second clone then runs the wave gate. *End:* the claims,
closures and merges are those of scenarios 32 and 33, and the event log of each session shows no refused
write to the other's unit. It runs on demand and before a release, not on every pull request.

## What it gates

A person is offered team use in two places: the passage of the `setup` skill that shows the line to
add ([[skill:setup]]), and the user documents. Package M5 writes the first, and only when this proof
passes; the documents packages come after M5 in the order of work and describe team use only if it has
passed ([[packaging:build-plan]]). Before that nothing a person reads names `[team]`. Cutover still
waits for E3 and E4 only ([[packaging:cutover-steps]]).

Delivered by package M5, which depends on M1 to M4 and M6, B5, K1, the first part of E1 and the story
tier of E2 ([[package:M5]]).
