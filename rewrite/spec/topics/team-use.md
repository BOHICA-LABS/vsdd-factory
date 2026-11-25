---
kind: topic
id: team-use
sources: [decisions.md#D33]
---

# Team use

Built when the build widens past the thin slice ([[packaging:build-plan]]). Nothing on the slice path reads
this file or changes because of it.

Decision D33 ([[doc:decisions-later]]): several people, each with their own machine, clone and Claude
Code session, run the factory against one shared repository, and the result is as sound as when one
person runs it. The Git remote is the only thing they share. Nothing here promises more than git gives: a
fetch, a push, and a remote that updates a branch for exactly one of two clones that push at once.

It adds no workflow, agent, skill, gate name, hook rule or decision kind. It adds three commands, two
records, one configuration section and the checks of four tables. This file links the parts and gives
the reason for each choice; each fact is in the file the table names, and nowhere else.

| Part | Defined in | Built by |
|---|---|---|
| The setting; what is shared and what belongs to one clone; versions; joining | [[config:team]], [[table:team-config-keys]], [[record:clone]], [[table:team-checks]] | [[package:M6]] |
| Sync: fetching, combining, pushing; conflicts; remotes | [[topic:team-sync]], [[command:sync]], [[table:sync-checks]], [[topic:team-git-facts]] | [[package:M1]] |
| Work claims: scopes, taking, releasing, taking over; a wave on a team | [[topic:team-claims]], [[record:work-claim]], [[table:work-claim-scopes]], [[table:work-claim-checks]], [[command:claim-take]] | [[package:M2]] |
| Who is recorded; decisions across clones; order; hand-off; ids | [[topic:team-records]], [[command:claim-release]] | [[package:M3]] |
| Delivery through pull requests; commits made by hand | [[topic:team-delivery]], [[table:team-delivery-checks]] | [[package:M4]] |
| The proof on two clones; offering team use | [[packaging:proof-team]] | [[package:M5]] |

## What is new, and why nothing existing could carry it

- **`vsdd sync`.** State entered a clone only at mount and left it only at a checkpoint. A settlement
  and a claim must be published outside a run, and `vsdd status --fix` also releases locks and removes
  worktrees, which a job started by a hook must not do. `status --fix` calls it.
- **The work claim.** The run lock is untracked and says nothing to another clone, and a field in the
  closure record would exist only once a unit has a closure. A file of its own gives two clones one
  path to collide on before either writes anything else.
- **`vsdd claim take` and `vsdd claim release`.** `vsdd lock release` repairs one clone's own lock. A
  command that changes what every clone sees must not share its name.
- **The clone record.** Nothing named a clone: a session id means something on one machine only.
- **`[team]`.** A project with one clone must not pay for any of this, so something has to say that there
  is more than one.

Everything else is reused: the survey and the checkpoint carry the new steps, `finding`, `spec-gap` and
`blocked` carry what a person must decide, the verdict's items carry the review of a hand commit, and
`pr` mode is the delivery.

## From one clone to a team

1. **Turn it on.** A person adds `enabled = true` under `[team]` on the clone that has the project and
   syncs. That clone claims what it has in flight (T-SWITCH).
2. **Join.** `setup` on another clone mounts the state, gives the clone its record and shows the shared
   settings (T-JOIN, [[skill:setup]]).
3. **Start.** Every survey syncs first and starts a run only from fetched state, then claims what the
   run will work on. A unit another clone holds is refused or left out, with the holder's name (T-FRESH,
   T-CLAIM).
4. **Work.** As on one clone: the worktrees, gates and evidence are local.
5. **Checkpoint.** It pushes state and the branches of stories in flight, and gives up the run's claims
   (T-RELEASE, T-HANDOFF).
6. **Decide.** Any person on any clone settles an open decision; the first settlement pushed stands, and
   only a published one is acted on (T-SETTLE).
7. **Deliver.** Through a pull request and CI, by the clone that holds the story (T-DELIVERY).
8. **Hand over.** The holder releases a story, or a person takes a claim whose clone is gone
   (T-TAKEOVER).
9. **Close the wave and release.** Whoever holds the cycle claim runs the gate. Commits made by hand in
   the range are listed and must be reviewed (T-NOTICE, T-ITEMS).

## What runs in parallel, and what the claims put in a row

| At the same time, on different clones | Why it is safe |
|---|---|
| Different stories, of one wave or of different ones | Each is one `story.<id>` claim, one branch and one closure record |
| A story being built while another clone drives its wave, plans, verifies or amends the specification | A standalone `story` run takes no `cycle` claim |
| A hotfix cycle or standing work beside the product cycle | Each cycle has its own `cycle` claim |
| `spike`, `discover` and a report-only `maintain`, at any time | They take no claim for their run; the rule and its one exception are in [[table:work-claim-scopes]] |
| Settling decisions, at any time | No claim covers a settlement |

One thing is put in a row: the cycle-level runs of one cycle (`spec`, `plan`, `feature`, `wave`,
`verify`, `release`), which all take that cycle's `cycle` claim. That is a choice, and it costs no
parallel work: the pipeline's own preconditions already let only one of them run at a time in a cycle,
since each wave needs the one before it closed and `verify` needs them all. The claim only makes sure
that the one that runs is on one clone.

## What was decided, and why

- **The claim.** A file on the state branch, made by a push that the remote accepts from one clone
  only; everyone sees it after a fetch. How a claim ends when its clone is gone is T-TAKEOVER's rule;
  the reason for it is that git gives no sign of a live clone. ([[topic:team-claims]])
- **Freshness.** A run starts only after a sync that fetched and pushed; a clone that cannot reach the
  remote starts none, which is what clause 4c asks. Nothing waits for a network or a password, and the
  briefing never fetches. ([[topic:team-sync]])
- **The state branch under several writers.** One file per record and one holder per unit. The sync
  decides each path itself: a record is chosen whole, never merged, and what was not taken is set aside
  and named; text a person wrote is merged by git or left for the person. ([[topic:team-sync]])
- **Git's answers.** Nothing is decided by a reason text git prints or by a push "failing": the tool
  fetches and compares commits. Which refs a fetch and a push name is one rule (T-REMOTE), under which
  no fetch moves a local branch. A state branch that was rewound or deleted stops the sync, also when
  that happens between a fetch and a push (T-PUSH). ([[topic:team-git-facts]])
- **Shared and local.** Tracked state is shared, and what belongs to one clone stays there
  ([[config:team]] has the list). A unit moves, a run does not. A hand-off continues from the last
  checkpoint; gates are measured again, except red, which travels. ([[topic:team-records]])
- **Who.** The person as git would stamp a commit there, and the clone, in the claim, the decision, the
  delivery and every state commit. No authority is checked. ([[topic:team-records]])
- **Delivery.** Pull requests and CI only, for stories and for what a release puts on a shared branch.
  ([[topic:team-delivery]])
- **Hand commits.** Noticed on the integration branch and on a story branch, listed, made items the
  review must check, never rewritten or reverted; with team use on or off. ([[topic:team-delivery]])
- **Versions and settings.** The `schema` number refuses an unknown state format; `team.min_plugin`
  refuses an old plugin; git has a floor; all of `factory.toml` is shared and nothing machine-bound may
  sit in it. ([[config:team]])
- **Remotes.** The remote a branch's upstream names, whatever it is called, and one repository that
  every clone pushes to; ordinary branches only; credentials never asked for; no clock compared.
  ([[topic:team-sync]])
- **One clone.** With `team.enabled` false nothing here runs but the checks that the head of
  [[table:team-checks]] lists, with the reason for each. ([[config:team]])
- **The proof.** Scenarios on two clones of one bare repository, and one run through the platform.
  Team use is offered to people only once it passes. ([[packaging:proof-team]])
- **The thin slice.** What the units being built now must not assume is in the ledger as `D33-<n>`, one
  entry for each owning package; none changes what the slice does on one machine
  ([[packaging:build-plan]]).

## What nothing here measures

Each check states its own limit, once, in its `unmeasured` cell. The widest are these: whether every
clone pushes to the same repository (T-REMOTE); whether a record that passes its check and was edited by
hand is true (T-INCOMING); whether a name is the person's own and whether that person was entitled to
act (T-WHO); and what a project's CI and its repository rules require (T-DELIVERY). Decision D33 gives
the last three to the repository.

## Where each clause of D33 ends

| Clause | Ends as |
|---|---|
| 1. Several clones, as sound as one | T-CLAIM, T-PUBLISH, T-PUSH, T-HOLD and T-COMBINE; the proof ([[packaging:proof-team]]) |
| 2. The remote is the only channel | Claims, decisions, state and hand-off travel only as fetches and pushes of branches (T-REMOTE); the shipped Python may import no network module ([[table:lint-checks]]). Delivery opens and merges the pull request clause 4f asks for through one host module, which carries nothing else |
| 3. Bound by what git can do | How a claim ends (T-TAKEOVER); no compared clocks (T-ORDER); a hand-off from what was pushed (T-HANDOFF); one rule for the refs of every fetch and push (T-REMOTE); every push judged by a fetch (T-PUSH); the git facts, each a test ([[topic:team-git-facts]]) |
| 4a. Parallel work | The scopes of [[table:work-claim-scopes]]; one file per record; T-COMBINE; T-WAVE; the table above |
| 4b. No double work | T-CLAIM, T-PUBLISH, T-MINT; T-SWITCH and T-SOLO for work already in flight; the claim lines of `vsdd status` and the briefing, with each clone's label |
| 4c. Current state | T-FRESH, T-HISTORY |
| 4d. Shared decisions | T-SETTLE |
| 4e. Attribution | T-WHO |
| 4f. Team delivery | T-DELIVERY, T-SWITCH, T-NEVER |
| 4g. Matching setups | T-VERSION, T-LOCAL, T-JOIN; [[config:team]] |
| 5. Records, not permissions | The fields of T-WHO; no key names a person or a role; the proof scenario in which one person settles another's decision |
| 6. Teammates who commit by hand | T-NOTICE, T-BRANCH, T-ITEMS, T-GAP, T-NEVER, with team use on or off |
| 7. Not one session or one working copy; not the multi-repository layout | The run lock, unchanged ([[topic:run-lock]]); T-SWITCH refuses team use in a hub ([[topic:multi-repo]]) |
| 8. Designed now, built later | Packages M1 to M6, none on the thin slice; the `D33-<n>` ledger entries for the slice's units |
