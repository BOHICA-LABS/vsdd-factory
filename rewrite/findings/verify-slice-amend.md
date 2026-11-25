# Verification of the tables repair and the slice amendment

Checked against `rewrite/findings/ledger.json`, the six originals in `rewrite/history/design-v1/`, the platform
facts in `rewrite/STATE.md`, and the built code under `next/` where an outcome cites it. Nothing but this file was
written.

## The checks

```
$ python3 rewrite/tools/spec_lint.py
registry: 20 agent, 31 artifact, 62 command, 22 config, 4 doc, 74 package, 32 packaging, 15 record, 17 rule, 48 schema, 13 skill, 21 table, 86 topic, 12 workflow
0 problem(s)

$ python3 rewrite/tools/sections.py coverage
sections: 240; cited by a spec file: 181; retired: 39; uncovered: 0
text: 972 KB total; 786 KB cited; 185 KB retired
```

Both pass. Neither check looks at meaning, so they cannot catch the problems below.
`rewrite/spec/source-index.json` (written 05:22) predates both passes. For example, it lists `BLUEPRINT.md#4.4`
with no citing file, but `topics/method-files.md` cites it. It needs regenerating.

## Counts

79 entries examined: 38 from `outcomes/tables.json`, 41 from `outcomes/slice-amend.json`.

- 61 are sound.
- 12 table entries were only partly done: the tables were fixed, but the prose copies the finding also names were
  not.
- 6 amendment entries are sound in themselves, but caused or left a problem that a builder would hit.
- The three entries marked left-for-builder (SL1-03, SL1-05, SL1-06) were correctly left. In each case the design
  now says the right thing, and I confirmed that the code under `next/` differs exactly as the outcome says.

Verdict: **not clean.** 7 major problems remain from the entries, and 4 new major problems were caused or left
by the fixes. None of them is on the thin slice's happy path: one story, local mode, no repair.

## Entries not confirmed

| Id | Severity | What remains |
|---|---|---|
| AU-T2-08 | major | The outcome says "all 31 artifact files link [[table:artifacts]] instead of restating the row". They do add the link, but every `artifacts/*.md` still restates its row as bullets (path, frontmatter keys, writer, cap, lifecycle: 4 or 5 cells each). One copy already disagrees: `artifacts/demo-evidence.md` gives the lifecycle "write-once. Removed with holdout/evaluations/<cycle>/ when the cycle closes", which the table row does not say. `conflicts/tables-repair.md` also claims this was done. |
| AU-S2-03, with AU-T2-13, AU-T2-15 and P3SEAM-06 | major | There are still two key lists that both call themselves the list. `topics/project-configuration.md` says on one line that "The one key list is [[table:config-keys]]". Lower down it says "The example below is the whole key list: every section and every key the loader accepts appears in it, with its default. It is the one key list". The header of `tables/config-keys.toml` points back to that example as "the whole key list". The example and the table give different defaults: `delivery.mode` "pr" against "detected"; `delivery.ci` "required" against "detected"; `protected_paths` with a literal `CHANGELOG.md` against the blueprint default that T2-15 chose; `auto-allow.scope` "off" against "set at setup"; `paths.docs` a fixed list against "detected". The example also leaves out the per-rule `[hooks]` keys that P3SEAM-06 added, although it claims to hold every key the loader accepts. This is the S1 loader's source, on the slice path. |
| AU-T2-09 | major | The mismatch is recorded in `conflicts/tables-repair.md` but not resolved. The kind ids are `holdout-scenario` and `project-conventions`, while `[reveals]` uses `list scenario`. `[reveals]` is "generated at build time from the class of each artifact kind", so the generated key and the hand-written table disagree. Choosing one vocabulary is put off until "when the kinds are named for the build", which is deferring the decision. |
| AU-P2-12, AU-S2-11 | major | Only the table part was done. Prose pointers into the originals remain: "blueprint section 10, item N" in `packaging/e2e-story-tier.md`, `plugin-manifest.md` and `proof-greenfield.md`; "open question 1 of the design" in `agents/verifier.md`, `codebase-analyzer.md`, `spike-runner.md` and `topics/model-tiers.md`; "Section 2.15." in `commands/import.md`; "Which subcommands exist is in `BLUEPRINT.md#4.9`" in `packaging/cli-entry.md`; "pipeline.md guard G3" in `rules/artifacts.md`; and the other files the findings list. |
| AU-S2-09 | major | Partly done. The spike and convergence topics now link the table. Still defined in a second place: the event types (the full list sits in `x-documented-types` in `schemas/event.schema.json` as well as in `tables/event-types.toml`); the tiers (`conventions.md` lines 114 to 116, and the tier line of every agent file); `GOVERNED` (`rules/merge-guard.md`); the kinds exempt from tests-first (`rules/tests-first.md`); the `lock_stale_minutes` default (`topics/run-lock.md`); the spike reuse rule (`tables/artifacts.toml` and `artifacts/spike-ledger-entry.md`); and the `pure_paths` default (`rules/purity-advice.md`). All copies currently agree. |
| AU-S2-12 | minor | Only the three table headers were cleaned. "(see `rewrite/spec/conflicts/commands.md`)" commentary remains in `commands/env-stop.md`, `validate.md`, `cycle-close.md`, `defer-add.md`, `review-prepare.md`, `coverage.md` and `idea-register.md`, and in the descriptions of the driver-flag, config-fingerprint and spike-admission schemas. `env-stop` and `validate` still give two syntaxes. |
| AU-P2-10 | minor | `cwd` was added to the schema, but the example `rule.block` line from `hooks.md#2.12` is still in no spec file, although the schema calls itself the machine copy of that line. |
| AU-T2-01 | minor | The table is right. `conflicts/people-repair.md` P-8 still says the source has "no other call edges" and that the roster was completed from that. |
| AU-T2-22 | minor | The verdict description reads "perimeter, head, passes, rounds_used, outcome …". The original (state.md 2.5, Convergence record row) says "perimeter, commit, passes, outcome, and the counts …". It is a paraphrase, not a quote. |

## New problems caused or left by the fixes

### VSA-01 (major). A rebase in pr mode breaks the recorded base, the red lookup and the patch-id claim

Caused by P3SEAM-03 and P3OPS-02. `topics/worktrees.md` now measures everything from the recorded base commit,
"never a branch name". `records/evidence.md` defines the patch id as the diff "from its base". P3OPS-02 made
delivery and the green gate require a red or re-freeze record "on a commit between the base and the head".

`commands/story-deliver.md` still rebases the story branch when the pull request conflicts, and still says
"After a clean rebase the patch id is unchanged, so both still hold". After a rebase:

- the red record's commit is no longer an ancestor of the new head, so delivery refuses and the green gate fails
  with `no red record`;
- the diff from the old recorded base now includes the stories merged in the meantime, so the patch id changes,
  green evidence is not found by patch id, and the story always goes `review_stale`.

Local mode merges and never rebases, so the slice is not affected.

Resolution:
- When delivery rebases, it records the commit it rebased onto as the new `delivery.base`.
- The red lookup also accepts the latest red or re-freeze record whose frozen files have the same SHA-256 at the
  head and whose patch id equals the head's diff from the new base.
- State this in `topics/worktrees.md` and `topics/evidence-lookup.md`.
- Add a G4a fixture: pr mode, the integration branch moved, a clean rebase, then a merge with no re-review.

### VSA-02 (major). The delivery `ok` field contradicts the repair loop in story.js

`schemas/delivery-data.schema.json`, rewritten in this pass, describes `ok` as "False when the story was not merged
and no person is being waited for". That makes `ok` false for `ci_failed`, `conflict` and `review_stale`. The
loop in `topics/story-script.md` tests `!d.ok` first and returns `blocked`, so it would never repair. Only the
merged path, which the slice uses, works.

Resolution: define `ok` as "the command ran and reached a state". Make it false only for `refused`,
`checkout_dirty` and a command that could not run. Say in `commands/story-deliver.md` which states carry
`ok: true`.

### VSA-03 (major). In a later review loop, the tool and the script account for different items

`topics/verdicts.md` says that in a later loop the items to account for are "the files changed between the
commit the loop before it converged on and the head it reviewed". story.js passes `d.changed_files`, which
delivery computed before the repair commits. `changed_files` is defined nowhere in the delivery schema.

After a `ci_failed` or `conflict` repair, the tool's list includes files the reviewers were never asked about.
The tool's verdict is then not converged while the script's is, and delivery refuses.

Resolution:
- Define `changed_files` in `delivery-data` as the files changed from `reviewed_sha` to the head at delivery.
- After a repair, story.js takes the list from the `gate green --since` result (add `changed_files` to that
  result).
- The tool uses the same definition: from `reviewed_sha` to the reviewed head.
- Add a conformance fixture in which a repair touches a new file.

### VSA-04 (major). The new `where` rule empties `own-worktree`

Caused by P3BLD-12 and P3COV-02. `topics/rule-model.md` now says that "the caller's worktree" and the related
phrases "in any rule mean the worktree that contains `where`". For a file tool, `where` is the target's own
directory. If `own-worktree` (the spike runner's only write grant) is read that way, every target lies in its
own worktree, and a spike runner could write into story and integration worktrees.

Two files still define `own-worktree` by the caller's working directory: `tables/path-classes.toml` (the
computed-class comment) and `rules/spike-bounds.md` (containment). The texts disagree.

Resolution: exclude `own-worktree` from the `where` sentence and define it once, as `spike-bounds` does (the
worktree that holds the payload's `cwd`, or the one recorded in the open ledger entry). Add a write-scopes case:
a spike runner writing into `.worktrees/S-1.01/` is blocked.

### Minor problems

- **VSA-05.** A `gated()` retry produces several fix records for the same loop and pass:
  `fix-implementer-l1-p1` and `fix-implementer-l1-p1-retry1` both carry the same `loop` and `pass`.
  `topics/verdicts.md` joins every fix record's `resolutions`, but the script uses only the last attempt. The
  `disputed` sets diverge. Count only the last done attempt of each fixer per pass.
- **VSA-06.** The verdict "covers a head when … each [loop] reviewed from the commit the one before it converged
  on". No record carries the commit a review started from: the identity has `method`, `loop` and `pass`, and
  review data has only `reviewed_sha`. Either add `from` to the review step's identity, or drop the condition.
- **VSA-07.** `rules/step-results.md` item 5 and `topics/verdicts.md` re-apply grounding "before it counts the
  record". They do not say against which tree. Against the current worktree, the findings of early passes no
  longer ground after later fixes. Ground against the record's `reviewed_sha`, or trust the stored `grounded`
  mark.
- **VSA-08.** `schemas/step-record.schema.json` now composes step-result, which has `head` ("commit the step left
  its branch at"), with the hook's `head` ("head of the unit's worktree when stored"). That is one key with two
  meanings. Rename the hook's field and update every place that uses it.
- **VSA-09.** `commands/story-deliver.md` says the checkpoint treats `checkout_dirty` "as a blocked step of kind
  `other`". A relay result carries no step identity, so no step record holds it, and nothing says where the last
  delivery state is recorded for the checkpoint to read. Name the record (the delivery job record, or a
  `delivery.state` in the closure) and the mapping.
- **VSA-10.** Local delivery, steps 2 to 4: a person who commits between the read and `merge --ff-only` makes git
  refuse, and step 4 reports that as `checkout_dirty` instead of going back to step 1. After a refusal, re-read
  the branch first. The text also does not name the commit message of the local merge commit (the default
  strategy is `squash`) or the path of the delivery file lock.
- **VSA-11.** `rules/step-results.md` item 3 reads "the evidence record the entry names", which is a path an agent
  supplied (`tmp` is writable). Look the record up by subject, gate and sha in the evidence class instead.
- **VSA-12.** `commands/gate-red.md`:
  - "contains `ac<N>`" makes `test_ac12_x` cover AC-1. Require a non-digit after N.
  - Mapping a classname to a file fails for Python `module.Class` classnames, which the sample uses. Allow the
    longest prefix that names a test file, or have the sample runner write `file`.
- **VSA-13.** Conflict records now contradict the amended tables:
  - P-8 (above);
  - `conflicts/rules.md` R2 and the derivation in `conflicts/hooks-repair.md`, which say an absent `commands`
    key means "*" and the verifier has no `commands` key;
  - `conflicts/tables-repair.md`, which says "patch_id … gone" (P3BLD-06 restored it), "the two lists no longer
    disagree", and that the artifact files no longer restate their rows;
  - `conflicts/records-repair.md`, which has two different entries both numbered RR10.
- **VSA-14.** The rule "every field of a relayed tool output is required" is not applied throughout:
  - `survey.rounds_used` is optional;
  - `story-unit` leaves `repo`, `ui`, `guidance`, `deferred`, `open_pr`, `blocked` and `fixes` optional;
  - `release-result` has no types and no required list.
- **VSA-15.** `packaging/sample-project.md`: the overlay's `factory.toml` carries "`delivery.mode`" but no value.
  With `--local` it must be `local`, and `delivery.ci` must be `none`.
- **VSA-16.** `records/closure.md` "Written by" leaves out `vsdd survey`. The survey now writes `delivery.branch`
  and `delivery.base` through the worktree library.
- **VSA-17.** P3COV-05: "CLAUDE.md joins the default docs globs" holds only in the topic's example. `paths.docs`
  defaults to "detected", and detection is not said to include `CLAUDE.md`.
- **VSA-18** (left by P3SEAM-05). Builders hold only `git-commit` and `git-read`. After `conflict`, the repair
  step cannot bring the integration head into the story branch (that needs `git-integrate` or `git-other`), so
  the automated conflict repair can only fail. Either have the tool merge the integration head into the story
  branch before the repair step, or grant a scoped merge.

## Read as the builder of the five subjects

- **Local delivery** (`commands/story-deliver.md`, `topics/worktrees.md`, `topics/state-directory.md`,
  `schemas/delivery-data.schema.json`, `commands/release.md`): buildable. The detached integration worktree,
  `update-ref <new> <old>`, the fast-forward exception and `checkout_dirty` are coherent. Every git behaviour
  relied on is real: `--ff-only` refuses atomically when it would overwrite a local change. Gaps are VSA-02
  (`ok`), VSA-09 (the checkpoint mapping), VSA-10 (the race, the message and the lock path). `release-result` has
  no state for a refused fast-forward (VSA-14).
- **Verdict recomputation** (`topics/verdicts.md`, `topics/prelude-converge.md`, `topics/story-script.md`, the
  step schemas, `records/step.md`): the identity fields, the reviewer-tier and owner test, loop order across runs,
  and `used` can all be built. Blocking: VSA-03. Minor: VSA-05 to VSA-08. On the wave side, a round whose
  measured gate failed returns findings with no stored review record. The replay cannot rebuild that pass, and
  "the items its gate names" is not defined. This is outside the slice.
- **Red evidence and the merge guard** (`rules/step-results.md`, `rules/tests-first.md`, `rules/merge-guard.md`,
  `tables/command-families.toml`, `tables/walls.toml`): buildable. The fabricated-red path is closed. The Bash
  write forms and the ref-moving commands are listed, and the closed `git-read` with the `git-other` catch-all
  is sound. Gaps are VSA-01 (rebase), VSA-04 (`own-worktree`), VSA-11 (the evidence path) and VSA-18.
- **Gate behaviour** (`topics/gates.md`, `commands/gate-red.md`, `commands/gate-green.md`,
  `commands/gate-demo.md`, `records/evidence.md`, `schemas/evidence-record.schema.json`): the clean-worktree
  rule, the cache rule, the base report, red, error and exempt tests, the frozen files, the stub marker and the
  criteria map are all defined. Gaps: VSA-12; how long `local/evidence/_base/*.xml` is kept is not stated; and
  what `--refreeze` does when the test-writer changes nothing is not stated.
- **The cycle's base** (`topics/worktrees.md`, `records/cycle.md`, `schemas/closure-record.schema.json`,
  `schemas/story-unit.schema.json`): it is defined once, and the restatements now link to it. Two gaps: it is
  never re-recorded after a rebase (VSA-01), and the closure record's writer list is incomplete (VSA-16).

## Entries confirmed, with notes

- **Tables:** AU-T2-02, 03, 04, 05, 06, 07, 10, 11, 12, 14, 16, 17, 18, 19, 20, 21, 23, 24, 25, 26; AU-P2-02,
  P2-19, P2-20, P2-23; AU-S2-16, S2-17. Values checked against hooks.md 2.7, 2.10 and 2.11.3, state.md 2.4, 2.5
  and 2.8, pipeline.md 2.5, and BLUEPRINT.md 4.8, 4.11 and 4.12. Nits:
  - the `[story]` comment says "eighteen keys", but the row now has nineteen (with `tests_method`);
  - the story lifecycle drops the original's "480 KB" sentence;
  - the "glob" cells of artifacts.toml are path patterns (`<section>.md`, `nfr.md, errors.md, …`), not globs;
  - `timeout_seconds = "platform budget of 1.5"` is copied as written, as the finding asked;
  - the `default = ""` cells of `attribution` and `allow_pr_close` (a boolean) leave the loader to treat ""
    as "no default".
- **Amendment:** P3SEAM-02, 04, 05; P3COV-01, 02, 03, 05; P3OPS-01, 03, 04, 05, 06; P3REQ-10; P3BLD-02, 03, 04,
  05, 06, 08, 09, 10, 11, 13; SL1-01, 02, 04, 07, 08, 09, 10, 11, 12.
  - Decisions that differ from what the finding proposed: `checkout_dirty` instead of `awaiting_merge`; the
    fast-forward exception instead of moving the person off the integration branch; no E0 package and no S3
    split, with ordering rules instead; no `integration_moved` refusal; no second block on ungrounded findings.
    Each is reasoned and breaks no user decision (D1 to D30). The unproven `updatedInput` on `StructuredOutput`
    is correctly put under probe 27 with a stated fallback.
  - Each proposal that was rejected is replaced by something that works.
- **Left for the builder:** SL1-03 (`next/plugins/vsdd-factory/py/launch.sh` sends exit 1 to the `*)` branch),
  SL1-05 (`next/plugins/vsdd-factory/py/vsdd/events.py` requires a non-empty `session_id`) and SL1-06
  (`next/workflow-lib/prelude.js` writes the `S_*` literals by hand, and six schema files are missing). All three
  were confirmed in the code. They belong to the B2, B3 and C1 rework, which nothing in the ledger yet assigns.

## For the orchestrator

- `packaging/build-plan.md` now gives the slice's packages and their order: B8 (probes 6, 10, 27 to 31 and 34),
  C2, B6, S4, B4b, the first part of E1, and the agent files A2b (architect) and A5 (demo-recorder,
  technical-writer, devops) that story.js names. That list differs from the one in `rewrite/STATE.md` and in this
  task's description. P3COV-03 left copying it into STATE.md to you.
- `registry.d/amend.toml` registers `implement-data` under C1, and `packages/C1.md` already lists it.
