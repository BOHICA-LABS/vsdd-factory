# Audit 2 of the restructure: one fact in one place, history, retirements, registry

Written by the second audit step, after the five repair passes. It compares the files under `rewrite/spec/`
with the six originals (`rewrite/BLUEPRINT.md`, `rewrite/design/{pipeline,state,agents-skills,hooks,packaging}.md`).
It reviews the conversion only, not the design, and changes nothing. Line numbers in the originals are from
the files as they stand on `rewrite/v1`; line numbers in the new files are from the files as they stand now.
Findings that repeat a class from `conflicts/audit.md` name that finding (F-number) so it can be traced.

## Verdict

Not yet one fact in one place. The repairs fixed the specific single-definition cases they were pointed at
(the `spike open --step` syntax, the holdout and security wall rows, `HUMAN_ONLY`, the state functions, the
rule `check()` signature, "the plan is ready", the named-spike branch rule), and the registry is clean. But the
F22 pattern is largely unfixed: workflow files still restate the command files' pass conditions and
behaviour, topics still restate the records and rules they summarise, every artifact kind is defined both in
`tables/artifacts.toml` and in its own file, and the decision rules are stated three times. Most copies agree.
Three do not: the configuration defaults of `delivery.mode`, `delivery.ci` and
`hooks.rules.auto-allow.scope` (table against annotated example), the flaky-test rule (two wordings in five
places), and the hub model's `[[repo]]` overrides. History phrases ("the first pass", "the second pass",
finding id P2LEAN-17) and pointers into the original documents remain in about forty places, a few of them
added by the repairs. `packaging/build-plan.md` rewrites the build plan's own paragraphs and adds an
ordering step. Of the 39 retirements, 38 are sound; `BLUEPRINT.md#11.2` is retired while its first item is
carried uncited. All 456 registry entries have their file and a valid, sensible package.

Counts: 0 blockers, 13 major, 12 minor (25 findings).

## The two checks

- `python3 rewrite/tools/spec_lint.py`: `registry: 20 agent, 31 artifact, 62 command, 22 config, 4 doc, 74
  package, 32 packaging, 15 record, 17 rule, 47 schema, 13 skill, 21 table, 86 topic, 12 workflow` and
  `0 problem(s)`.
- `python3 rewrite/tools/sections.py coverage`: `sections: 240; cited by a spec file: 181; retired: 39;
  uncovered: 0` and `text: 972 KB total; 786 KB cited; 185 KB retired`.

Both pass. Neither checks single definition, history or retirement correctness.

## Method

- Single definition: every sentence of 50 characters or more (markup and links normalised) was indexed across
  all design files under `rewrite/spec/` (not `conflicts/`, `packages/`, the registry or the indexes); every
  sentence found in two or more files was read in context, and near-identical sentence pairs (similarity
  0.6 to 0.99) were diffed to find copies that disagree. Then 73 names were searched by their defining text
  (syntax, default, pass condition, list members) in the whole folder; the table at the end gives each.
- History: every occurrence of first/second/third pass, close-out, repair, audit, amend, replaces the old,
  renamed, no longer, was/were, review finding ids, `<doc>.md`, "section N.N", "the X design's" and
  "blueprint section" was extracted with 30 characters of context and looked up in the normalised originals,
  to separate text copied from an original from text the converters or repairers wrote.
- Retirements: all 39 entries of `retired.toml` were read against the original section; the five amendment
  lists were spot-checked item by item (pipeline items 1, 2, 4, 6, 11, 12, 16, 17, 20, 31, 36, 38) for content
  missing from the body text.
- Registry: `registry.toml` and `registry.d/*.toml` were loaded as the linter does; for every entry the file,
  the frontmatter `package` and the package id were checked, and each package was compared with what
  `BLUEPRINT.md#9` says that package delivers. `source-index.json` was compared with the live citations.

## Findings

### S2-01. Workflow files restate the command files' pass conditions and behaviour (major, earlier finding F22 not fixed)

- Originals: `pipeline.md#2.9.1` lines 836-866 (red gate, frozen-test fix, green gate, `story deliver`) and
  `state.md#2.16` lines 1426-1434 (the command rows); `pipeline.md#2.9.2`, `#2.9.6`, `#2.9.4` for wave,
  verify and feature.
- New, second copies: `workflows/story.md` lines 63-72 (red gate), 83-88 (green gate), 89-100 (re-freeze),
  143-157 (`story deliver` states and modes), 165-174 (verdict binding, force-push rule), against
  `commands/gate-red.md` 19-35, `commands/gate-green.md` 19-25, `commands/story-deliver.md` 20-44.
  Also `workflows/wave.md` 67-75 against `commands/gate-wave.md`; `workflows/verify.md` 44-50 against
  `commands/gate-release.md` 26-30; `workflows/feature.md` 38-42 against `commands/feature-route.md` 18-22;
  `workflows/plan.md` against `commands/gate-plan.md` and `commands/plan-schedule.md`; `workflows/spec.md`
  against `commands/gate-spec.md`.
- Do they agree: yes, sentence for sentence. The only wording difference is `gate-wave.md` "a wave-level demo
  is no longer part of the gate" against `wave.md` "is not part of the gate".
- Proposed resolution: keep the pass condition, states and modes in the command file; the workflow's stage
  keeps its gate expression and links `[[command:...]]`.

### S2-02. The flaky inherited test rule is stated five times, in two wordings (major, defined twice, disagreeing)

- Originals: `BLUEPRINT.md#4.13` lines 746-748: "An inherited test that fails and then passes when the gate
  runs the suite once more does not fail the gate". `state.md#2.8` lines 786-788 and `pipeline.md#2.9.8`
  lines 1101-1103: "An inherited test that passed at the baseline and fails now is run once more before it
  counts as new; one that passes in either run does not fail the gate". `BLUEPRINT.md#11.4` row m14 (line
  1729) shows the close-out meant the second wording, but 4.13 was not changed.
- New: `records/baseline.md` 28-31 (state wording), `commands/gate-green.md` 40-42 (blueprint wording),
  `workflows/ingest.md` 101-104 (pipeline wording), `packaging/tests-python.md` 59 ("fails once and passes
  on the second run"), and the `flaky` description in `schemas/gate-result.schema.json` line 16.
- Do they agree: no, narrowly. The blueprint wording reruns and records any inherited test; the state
  wording only one that passed at the baseline. `conflicts/records-repair.md` RR10 chose `state.md#2.8`
  "the detail document that owns the subject", against the precedence rule (section 4 of the blueprint
  first).
- Proposed resolution: one statement in `record:baseline`, worded per the precedence rule
  (`BLUEPRINT.md#4.13`), with the disagreement kept in RR10 and RR10's "Used" line corrected; the gate,
  workflow and test files link to it.

### S2-03. Configuration defaults are stated in two key lists that disagree on three keys (major, defined twice, disagreeing)

- Originals: `state.md#2.12` lines 1043-1194, the annotated `factory.toml` that "is the whole key list ...
  with its default": `mode = "pr"` (1090), `ci = "required"` (1091), `scope = "off"` (1190).
  `BLUEPRINT.md#4.12` (lines 685-735): `delivery.mode`, `delivery.ci` "detected"; the auto-allow scope "set
  at setup". `hooks.md#2.6` line 506: `scope = "core"`.
- New: `tables/config-keys.toml` (96 keys; `delivery.mode` and `delivery.ci` default "detected",
  `hooks.rules.auto-allow.scope` line 617-621 "written once by vsdd init ...") and the verbatim example in
  `topics/project-configuration.md` lines 63-211 (`mode = "pr"` 110, `ci = "required"` 111,
  `scope = "off"` 210). Both files call themselves the one key list (`config-keys.toml` line 3 points to the
  example; the topic, line 61: "It is the one key list"). `schemas/config.schema.json` is a third list of the
  same keys (they agree on key names). `tables/limits.toml` repeats the defaults of 14 limit keys (they agree).
- Do they agree: no, on the three keys above; yes on every other default compared (81 keys).
- Proposed resolution: one statement of each default. Since F07 required the example to be copied once,
  keep the example as the text and make the table carry key, type and owner only, or mark the three rows
  as recorded disagreements (RR2, RR3) instead of giving a second default. `limits.toml` links to the
  config-key rows for its defaults.

### S2-04. The decision kinds and the qualified identifier are defined in three places (major, earlier finding F22 not fixed)

- Originals: `BLUEPRINT.md#4.11` lines 642-652 (kinds and choices), `state.md#2.8` lines 846-848 (choice to
  status), `pipeline.md#2.6` (stops); `BLUEPRINT.md#4.8` line 532 and `state.md#2.8` line 836 (qualified
  identifier); `pipeline.md#2.6` lines 516-517 (hotfix).
- New: `tables/decision-kinds.toml` holds kinds, opened-when and choices; `topics/human-stops.md` lines 17-80
  restates every kind's opened-when and choices. The qualified identifier and the refusal of an ambiguous
  bare id are written out in `conventions.md` 146-150, `topics/decisions.md` 26-31 and
  `topics/hotfix-cycle.md` 12-16, and repeated in part in `commands/decide.md` 32, `skills/decide.md` 39 and
  `rules/decisions.md` 32-38.
- Do they agree: yes in substance. Wording differs: the `blocked` reason `reviewer` is "a reviewer refused or
  could not review" in the table and "a reviewer returned blocked, for example because it declined the
  material or an input was missing" in the topic; `merge` reject is "the pull request is closed" against
  "the next survey closes the pull request".
- Proposed resolution: choices in the table only, the identifier rule in `doc:conventions` only (it is
  section 4.8 of the blueprint); the topics and command files link.

### S2-05. Every artifact kind is defined twice: in `tables/artifacts.toml` and in its own file (major, earlier findings F22 and F25 not fixed)

- Original: `state.md#2.5` lines 534-629, one table row per kind; `hooks.md#2.11.3` lines 1049-1093, the
  worked `[contract]`, `[story]` and `[_unknown]` rows.
- New: `tables/artifacts.toml` now has all 31 kinds (fixing F25), but every `artifacts/<kind>.md` still
  restates path, frontmatter, writer, cap and lifecycle as a list (for example `artifacts/brief.md` 10-17,
  `artifacts/contract.md` 10-19). `rules/artifacts.md` lines 30-46 still carries the `[story]` and
  `[_unknown]` rows. `record:holdout-evaluation` and the `[holdout-evaluation]` row repeat each other.
- Do they agree: yes; caps and paths of all 31 kinds match.
- Proposed resolution: the artifact files keep only what the table does not hold and link
  `[[table:artifacts]]`; `rules/artifacts.md` links instead of showing the rows.

### S2-06. Records and rules are restated by the topics that explain them (major, earlier finding F22 not fixed)

- `records/lock.md` 18-45 and `topics/run-lock.md` 16-42 and 79-80 both define taking, refreshing, staleness, release,
  two machines and atomic writes (original `state.md#2.11` lines 975-1030). Agree, apart from
  `records/lock.md` "refreshes `seen` through `lock_seen`" against the topic's "refreshes `seen`".
- `records/caller-marker.md` 13-29 and `topics/caller-identity.md` 11-31 (`hooks.md#2.8`): the prefix
  rewrite, the three users of the marker, pruning after ten minutes. Agree.
- `records/config-fingerprint.md` and `topics/rule-tuning.md` (`hooks.md#2.6`). Agree.
- `rules/auto-allow.md` 43-90 and `topics/tool-call-approval.md` 28-45 (`hooks.md#2.11.10`): the switch,
  the setup question, the static-rules fallback, the prompt counts. Agree ("an agent whose row" against
  "the agent whose role").
- `topics/workflow-shape.md` 47-52 and `topics/resume.md` 13-31 (`BLUEPRINT.md#4.14`, `pipeline.md#2.5`,
  `state.md#2.8`): the done test, which `resume.md` then states a second time as "Refinement". Agree.
- Proposed resolution: the record or rule file defines; the topic explains and links.

### S2-07. Command files and topics restate each other (major, earlier finding F22 not fixed)

- `commands/import.md` 18-40 and `topics/importer.md` 15-75 (`state.md#2.15`, `#2.16`). They agree but each
  has a clause the other lacks: the command has "whose measurements are taken by the first survey that
  needs them" and "whose stories are imported as draft"; the topic has "naming those stories".
- `commands/init.md` and `topics/project-configuration.md` 39-48 and `topics/state-directory.md` (detection,
  `--auto-allow`, mounting). Agree.
- `commands/survey.md` 30-34 and `commands/checkpoint.md` against `topics/workflow-shape.md` 24-40. Agree.
- `commands/explain.md` and `topics/event-log.md` (the block example). Agree.
- `commands/retire.md` and `topics/artifact-rules.md`. Agree.
- `packaging/build-step.md` 61-71 and `topics/prelude.md` 89-97, the prelude constraints of `packaging.md#2.5`
  (lines 464-467). Agree; recorded by `conflicts/people-repair.md` P-3 but left in both.
- Proposed resolution: one owner each (command, `topic:importer`, `packaging:build-step`), links elsewhere.

### S2-08. The hub model is written out three times, and the copies differ on what a `[[repo]]` entry may carry (major, defined twice, disagreeing; F22 not fixed)

- Originals: `pipeline.md#2.9.11` lines 1167-1209 and `state.md#2.14` lines 1270-1304.
- New: `workflows/multi.md` 12-18 (says it "defines the hub model"), `topics/multi-repo.md` 14-18 (an "In one
  view" summary the converter wrote), `topics/multi-repo-state.md` 10-30.
- Do they agree: mostly. `topics/multi-repo-state.md` line 21: an entry "may carry its own `branches`,
  `commands`, `paths`, `release` and `delivery`"; `workflows/multi.md` line 81: "its own `[release]` and
  `[delivery]` tables". The state wording is the fuller one; the two are not reconciled anywhere.
- Proposed resolution: one statement of the hub model and of the per-repository overrides, in the owner the
  workflows repair chose (`workflow:multi`) or in `topic:multi-repo-state` for the configuration side; record
  the override-list difference; remove the "In one view" paraphrase.

### S2-09. Lists and values repeated in a second file (major, earlier findings F22 and F30 not fixed)

All of these agree; each is a second definition.

- Spike limits: `topics/spikes.md` 90-99 (its own table, from `pipeline.md#2.8`) and `tables/limits.toml`
  spike rows (from `pipeline.md#2.5`), plus `config-keys.toml` and the example.
- Event types: `tables/event-types.toml` and the `x-documented-types` list in `schemas/event.schema.json`
  (now equal, 30 types each; F30 asked for one list).
- Tiers: `tables/roster.toml` 12-16, `conventions.md` 111-117 (with the override and test-tier paragraphs
  again in `topics/model-tiers.md` 14-36), and line 15 of every agent file (`model: sonnet`, `effort: xhigh`).
- `GOVERNED`: `tables/callers.toml` 12 and `rules/merge-guard.md` 34.
- Kinds exempt from tests before code: `tables/story-kinds.toml`, `tables/callers.toml` 13-14 ("which today
  exempts facade, docs, chore and harness"), `rules/tests-first.md` 47, `workflows/story.md` 54-55.
- Holdout thresholds: `config-keys.toml`, the example, `topics/convergence.md` 100-102.
- `state.lock_stale_minutes` (20): `config-keys.toml`, the example, `records/lock.md` 33,
  `topics/run-lock.md` 34, `config/state.md` 10.
- The earlier-finding reuse rule of a spike: `commands/spike-open.md` 27-29, `topics/spikes.md` 27-30,
  `rules/spike-bounds.md` 103-105, `artifacts/spike-ledger-entry.md`, `tables/artifacts.toml` `[spike-ledger-entry]`.
- Proposed resolution: keep each in its table and link from prose.

### S2-10. History phrases remain in design files (major, earlier finding F23 not fixed)

Copied from an original (the rule forbids them even so): `topics/caller-identity.md` 17-20 ("The first pass
wrote one marker per session", `hooks.md` 1373); `topics/rule-tuning.md` 47 ("the first pass had no key for
it", `hooks.md` 537); `rules/spike-bounds.md` 107-110 ("The first pass let the main session run a spike",
`hooks.md` 1252); `topics/event-log.md` 15-16 ("the first pass's list of reserved event types", `hooks.md`
1455); `agents/test-writer.md` 46-47 ("the first pass's separate `stubs` method", `agents-skills.md` 504);
`packaging/tests-conformance.md` 40 ("The second pass names these ... as first designed", `packaging.md` 632);
`packaging/tests-workflows.md` 53-66 ("Before the corrections", "The corrected text", `pipeline.md` 1566-1579);
`platform.md` 266 ("the offered cut P2LEAN-17", `BLUEPRINT.md` 1513).

Written by the converters or repairers: `topics/run-lock.md` 54-55 ("The blueprint's second pass offered
lock liveness by process, not by heartbeat (P2LEAN-17)"); `topics/computed-status.md` 29-31 ("Its summary of
the second-pass amendment lists `active` ..."); `rules/secrets.md` 65 ("The blueprint amendment
(BLUEPRINT.md#5.4)"); `topics/seams-state.md` 9 ("Every interface the state design stated is now defined
by"); `tables/methods.toml` 345-347 ("Provisional agent names the pipeline design was written with, and ...
that replace each"); `packaging/build-plan.md` 21-23 (see S2-13); `tables/callers.toml` 14 ("today").

- Proposed resolution: strike the history clause and keep the design clause where the sentence has both
  (for example keep "With a marker per call:" and its list); quote 11.2 item 1 in `topic:run-lock` without
  the framing sentence and without the finding id, and cite `BLUEPRINT.md#11.2` (see S2-15).

### S2-11. Pointers into the original documents remain in place of links or text (major, earlier finding F35 not fixed)

- Pointer as the place to find the text: `commands/import.md` 18 ("Section 2.15.", copied from
  `state.md#2.16` line 1376; the text is `topic:importer`); `commands/config.md` 20 ("the one list in state design
  section 2.12"); `rules/artifacts.md` 30 ("the content is state.md section 2.5"); `tables/limits.toml` 6
  ("left to its sections 2.7 and 2.", truncated); `agents/verifier.md` 16, `agents/codebase-analyzer.md` 16,
  `agents/spike-runner.md` 15 and `topics/model-tiers.md` 40 ("open question 1 of the design", a retired
  section, `agents-skills.md#6`; the answer is decision D5); `topics/risks-state.md` 18.
- Written by converters, naming the blueprint where a link exists: `packaging/ci.md` 10,
  `packaging/cutover-steps.md` 21 and 28, `packaging/cutover-deletions.md` 11, `packaging/plugin-manifest.md`
  62 ("decision Dnn in `BLUEPRINT.md#2.3`", for `[[doc:decisions]]`); `packaging/e2e-story-tier.md` 20 and 37,
  `packaging/proof-greenfield.md` 23-24, `packaging/plugin-manifest.md` 119 and 128,
  `packaging/packaging-assumptions.md` 14 ("blueprint section 10, item N", for `[[doc:platform]]`);
  `packaging/cli-entry.md` 36; `packaging/proof-brownfield.md` 28; `packaging/plugin-assets.md` 11;
  `topics/event-log.md` 11; `topics/information-walls.md` 29; `topics/tool-call-approval.md` 16 and 21;
  `rules/decisions.md` 11 and 27; `tables/artifacts.toml` 122, 217, 232, 278.
- Proposed resolution: replace each with the `[[kind:id]]` link of the thing, or copy the text.

### S2-12. Conversion commentary and pointers to conflicts files sit inside design files (minor, other; F29 class)

- `tables/walls.toml` 3, `tables/story-kinds.toml` 7-9, `tables/command-families.toml` 29,
  `artifacts/analysis.md` 21, `topics/state-layout.md` 60-63, `topics/computed-status.md` 29-31,
  `commands/cycle-close.md` 20-22 and 38, `commands/env-stop.md` 14-16, `commands/validate.md` 14-15,
  `commands/review-prepare.md` 31, `commands/coverage.md` 32, `commands/idea-register.md` 15,
  `commands/defer-add.md` 37, `commands/gate-red.md` 45-47 ("The state design's own wording of the same
  rule"), `commands/gate-green.md` 38 ("The state design's row adds"), and the descriptions of
  `schemas/run-record`, `driver-flag`, `config-fingerprint` and `spike-admission` ("see conflicts/records.md").
- `commands/env-stop.md` and `commands/validate.md` each present two syntaxes instead of the one the
  precedence rule selects.
- Proposed resolution: move the commentary into the conflicts files; a design file states the chosen text.

### S2-13. `packaging/build-plan.md` rewrites the build plan's paragraphs and adds an ordering step (major, altered)

- Original `BLUEPRINT.md#9.1` lines 1358-1359 and `#9.6` lines 1452-1457: "C3 (stages and routes) no longer
  exists as a package ..."; "There are 74 packages. The second pass merged five small ones ... split six
  ... split B4 to break a dependency cycle with H1, and added E4. Order of work: B0 to B3, B6, B8, R0, C1 and
  C2 first, then B4a and B5. Then S, G, H and A packages run in parallel; B4b follows H1. Workflows follow the
  packages they call. E3 and E4 are the gate for cutover."
- New, lines 17-23: "The packages were sized so that one builder and one reviewer can carry each, except
  where a dependency cycle made a split necessary: B4 is split for that reason ..., and E4"; "Two packages
  from the plan's first table no longer exist as such: C3 ... and, from the packaging design's own earlier
  table, the ids the blueprint reused were kept and the others renamed" (one package is named, and the tail
  comes from the retired `packaging.md#2.12`). Lines 32-34: a fifth order-of-work step, "Operator tools,
  proofs, documents, release and cutover: T1, T3, T5, E1 to E4, D2, D3, R1, CO1, CO2", which no original
  states (T1 is a dependency of K1). Lines 38-41 add a converter summary of the foundation packages.
- Proposed resolution: copy the order-of-work paragraph as written; drop the history sentence rather than
  paraphrase it; remove step 5 and the summary.

### S2-14. The reason for one project ledger outside spike worktrees is carried nowhere (major, earlier finding F15 not fixed)

- Original `state.md#2.17` lines 1452-1454: "added one project ledger outside any spike worktree, because a
  ledger that lives in a throwaway worktree cannot stop a question being asked twice".
- New: no file has it. `conflicts/people-repair.md` P-4 says `topics/worktrees.md` "reaches the
  project-ledger reason through `[[config:spikes]]`"; `config/spikes.md` does not contain it, nor does
  `topics/spikes.md` 109-111 ("One ledger per project holds every spike from every entrance").
- Proposed resolution: copy the reason into `artifact:spike-ledger-entry` or `topic:spikes`, and cite
  `state.md#2.17` there.

### S2-15. `BLUEPRINT.md#11.2` is retired as history while its first item is carried, uncited (minor, wrongly retired)

- Original lines 1661-1676. Item 1 (lock liveness by process, "Until you say, the heartbeat is the design")
  is still open and is what probe 29 refers to. Items 2 and 3 are open offers to the person: the lessons
  register can return "with one added step", and S5/CO1 and H8/W1 could be merged.
- New: item 1 is quoted in `topics/run-lock.md` 52-62, whose sources (`state.md#2.11`, `BLUEPRINT.md#4.13`)
  do not include 11.2; `retired.toml` lines 17-19 still retire the section as history. Items 2 and 3 appear in
  no file.
- Proposed resolution: cite `BLUEPRINT.md#11.2` from `topic:run-lock` and drop its retirement, or record the
  two other offers with the open offer in `doc:decisions`.

### S2-16. `conventions.md` says section 4.4 is not carried, but it is (minor, other)

- `conventions.md` line 165: "4.4, the provisional names the pipeline design used: not carried over."
- `tables/methods.toml` 345-420 carries 4.4 as `[[replaced-name]]` and `topics/method-files.md` cites it;
  4.4 is no longer retired (`conflicts/people-repair.md` P-5). The `[[replaced-name]]` rows are a mapping
  from provisional names (history-like) and hold lists as comma-joined strings (`method = "discovery,
  market-intel"`).
- Proposed resolution: correct the conventions line to link `[[table:methods]]`; decide once whether the
  mapping is design (keep, with arrays) or history (retire 4.4 and drop the rows).

### S2-17. `tables/story-kinds.toml` says its own table lives in `artifacts.toml` (minor, other)

- Line 5: "The kinds, with whether each has a red gate, are one table in artifacts.toml". The copied
  sentence fits the original layout (`BLUEPRINT.md#9.1` C2: "`artifacts.toml` (with the story kinds)"), but
  in the spec `red_gate` is in `story-kinds.toml` (lines 15-55) and `artifacts.toml` has no red-gate data.
- Proposed resolution: say where the table is in the spec, or link `[[table:story-kinds]]`.

### S2-18. `source-index.json` is stale (minor, other)

- Twelve sections map to fewer files than the live citations: `BLUEPRINT.md#4.4` gives `[]` (live:
  `topics/method-files.md`), `state.md#2.17` gives `[]` (live: `topics/worktrees.md`); `state.md#4`,
  `state.md#2.8` and `hooks.md#5` lack `topics/state-api.md`; `packaging.md#2.5` lacks `topics/prelude.md`;
  `pipeline.md#2.11` lacks `packaging/tests-workflows.md`; `state.md#2.4`, `#2.5`, `#2.7`, `#2.16` and `#5`
  lack `records/cycle.md`, `records/closure.md`, `records/driver-flag.md`, `records/evidence.md` and
  `records/decision.md`.
- Proposed resolution: regenerate with `python3 rewrite/tools/sections.py coverage --write-index`.

### S2-19. Two registry summaries no longer match their tables (minor, other)

- `registry.toml` `[table.callers]`: "The groups of vsdd subcommands a wall row may hold, what each read command
  reveals, and the person-only and governed command lists". `[callers]` and `[reveals]` moved into
  `tables/walls.toml` (lines 205 and 222); `tables/callers.toml` holds only `[constants]`.
- `[table.roster]`: "each agent's tier, tools and package". After F06 the roster rows are `name = tier`.
- Proposed resolution: the registry owner corrects the two summaries (repairers may not edit `registry.toml`).

### S2-20. `registry.d/` files carry repair history (minor, history leaked)

- `registry.d/people.toml` lines 1-6 ("this pass's audit (F03)", "the state and rules repair passes"),
  `registry.d/records.toml` lines 1-3 ("audit F03"), `registry.d/hooks.toml` lines 1-2 ("added while
  repairing the hook/rules area").
- Proposed resolution: remove the comments; empty additions files can be deleted.

### S2-21. The README names a folder that does not exist (minor, other)

- `rewrite/spec/README.md` line 3 says the six documents are "kept unchanged under `rewrite/history/design-v1/`".
  They are at `rewrite/BLUEPRINT.md` and `rewrite/design/`; `rewrite/history/` holds only
  `section-manifest.json`.
- Proposed resolution: for the person or the README's owner; the repair rules forbid editing it.

### S2-22. The security reviewer's agent file restates its read walls incompletely (minor, defined twice)

- `agents/security-reviewer.md` line 39: "Read walls: `holdout` and `reviews`". `tables/walls.toml` line 160
  denies `holdout`, `reviews`, `runs` and `outside` (no shell, `BLUEPRINT.md#4.3` line 308; every row denies
  `runs`, `hooks.md#2.7`).
- Proposed resolution: link the row, as `agents/holdout-evaluator.md` 40-41 now does.

### S2-23. The B5 test list was reworded (minor, altered)

- Original `pipeline.md#2.11` lines 1566-1579 ends "Package B5 turns these and the original nine cases into
  harness tests. They have not been run on the platform."
- New `packaging/tests-workflows.md` 54-66 moves the B5 sentence to the front, changes "the three blocks" to
  "the three script skeletons" and drops "They have not been run on the platform."
- Proposed resolution: copy the passage as written, without its history clauses (see S2-10).

### S2-24. A retirement reason names files, not the section that holds the text (minor, other)

- `retired.toml` lines 21-23: `BLUEPRINT.md#11.3` is `duplicate` with the comment "carried by
  topics/tool-call-approval.md, workflows/ingest.md and topics/hooks-inert.md". The README requires the id
  of the section that holds the same text. The three consequences are carried (`rules/auto-allow.md` 50-80
  for the scopes, `workflows/ingest.md` 12 and 92 for the one decision, `topics/spikes.md` 114-118 for setup),
  but not in `topics/hooks-inert.md`.
- Proposed resolution: name the sections (`hooks.md#2.11.10`, `pipeline.md#2.9.8`, `pipeline.md#2.8`).

### S2-25. A repair record claims a fix that is not there (minor, other)

- `conflicts/people-repair.md` P-4 (S2-14) and the hook, command and workflow repair summaries ("struck
  remaining original-document pointers and history phrasing"; "de-duplicated the hub model and ambiguity
  rule") describe work that S2-04, S2-08, S2-10 and S2-11 show is incomplete.
- Proposed resolution: correct the records when the fixes land.

## Seventy-three names checked for a single definition

"Agree" means the copies say the same thing. Sound means defined once and linked elsewhere.

| # | Name | Defined in | Also stated in | Result |
|---|---|---|---|---|
| 1 | `vsdd gate red` | `commands/gate-red.md` | `workflows/story.md` | Twice, agree (S2-01) |
| 2 | `vsdd gate green` | `commands/gate-green.md` | `workflows/story.md` | Twice, agree (S2-01) |
| 3 | `vsdd story deliver` | `commands/story-deliver.md` | `workflows/story.md` | Twice, agree (S2-01) |
| 4 | `vsdd gate wave` | `commands/gate-wave.md` | `workflows/wave.md` | Twice, agree (S2-01) |
| 5 | `vsdd gate release` | `commands/gate-release.md` | `workflows/verify.md` | Twice, agree (S2-01) |
| 6 | `vsdd feature route` | `commands/feature-route.md` | `workflows/feature.md` | Twice, agree (S2-01) |
| 7 | `vsdd gate plan` | `commands/gate-plan.md` | `workflows/plan.md`, `story.md`, `feature.md` | Twice, agree (S2-01) |
| 8 | `vsdd gate spec` | `commands/gate-spec.md` | `workflows/spec.md` | Twice, agree (S2-01) |
| 9 | `vsdd plan schedule` | `commands/plan-schedule.md` | `workflows/plan.md` | Twice, agree (S2-01) |
| 10 | `vsdd survey` | `commands/survey.md` | `topics/workflow-shape.md` | Twice, agree (S2-07) |
| 11 | `vsdd checkpoint` | `commands/checkpoint.md` | `topics/workflow-shape.md` | Twice, agree (S2-07) |
| 12 | `vsdd import` | `commands/import.md` | `topics/importer.md` | Twice, complementary clauses (S2-07) |
| 13 | `vsdd init` | `commands/init.md` | `topics/project-configuration.md`, `state-directory.md` | Twice, agree (S2-07) |
| 14 | `vsdd explain` | `commands/explain.md` | `topics/event-log.md` | Twice, agree (S2-07) |
| 15 | `spike open --step` syntax | `commands/spike-open.md` | linked from `topics/spikes.md` | Sound (F17 fixed) |
| 16 | Spike earlier-finding reuse | `commands/spike-open.md` | four other files | Five times, agree (S2-09) |
| 17 | `vsdd decide` settling | `commands/decide.md` | `records/decision.md` | Twice, agree |
| 18 | `vsdd retire` | `commands/retire.md` | `topics/artifact-rules.md` | Twice, agree (S2-07) |
| 19 | `vsdd cycle close` | `commands/cycle-close.md` | none | Sound; commentary (S2-12) |
| 20 | `vsdd env stop` syntax | `commands/env-stop.md` | none | Two syntaxes in one file (S2-12) |
| 21 | `vsdd validate` syntax | `commands/validate.md` | none | Two syntaxes in one file (S2-12) |
| 22 | `vsdd lock release` | `commands/lock-release.md` | linked from `topics/run-lock.md` | Sound |
| 23 | Run lock | `records/lock.md` | `topics/run-lock.md` | Twice, agree (S2-06) |
| 24 | Caller marker | `records/caller-marker.md` | `topics/caller-identity.md` | Twice, agree (S2-06) |
| 25 | Configuration fingerprint | `records/config-fingerprint.md` | `topics/rule-tuning.md` | Twice, agree (S2-06) |
| 26 | Flaky inherited test | `records/baseline.md` | four other files | Five times, disagree (S2-02) |
| 27 | Holdout evaluation | `records/holdout-evaluation.md` | `tables/artifacts.toml` | Twice, agree (S2-05) |
| 28 | Decision packet cap (12 KB) | `records/decision.md` | none | Sound |
| 29 | State functions (S7) | `topics/state-api.md` | linked from records | Sound (F03 fixed) |
| 30 | Spike admission | `records/spike-admission.md` | `rules/spike-bounds.md` | One sentence repeated, agree |
| 31 | `auto-allow` scopes | `rules/auto-allow.md` | `topics/tool-call-approval.md` | Twice, agree (S2-06) |
| 32 | `HUMAN_ONLY` | `tables/callers.toml` | linked from `rules/gate-guard.md` | Sound |
| 33 | `GOVERNED` | `tables/callers.toml` | `rules/merge-guard.md` | Twice, agree (S2-09) |
| 34 | Kinds exempt from tests first | `tables/story-kinds.toml` | three other files | Four times, agree (S2-09) |
| 35 | `[story]`, `[_unknown]` rows | `tables/artifacts.toml` | `rules/artifacts.md` | Twice, agree (S2-05) |
| 36 | Rule `check()` signature | `topics/rule-model.md` | none | Sound (F04 fixed) |
| 37 | Launcher skeleton, uv message | `topics/hook-launcher.md` | none | Sound (F21 fixed) |
| 38 | Spike time box (20 minutes) | `tables/config-keys.toml` | `limits.toml`, `spikes.md`, `spike-bounds.md`, example | Several, agree (S2-09) |
| 39 | Event types | `tables/event-types.toml` | `schemas/event.schema.json` | Twice, agree (S2-09) |
| 40 | Holdout evaluator wall | `tables/walls.toml` | linked from agent file | Sound (F02 fixed) |
| 41 | Security reviewer write grant | `tables/walls.toml` | linked from agent file | Sound (F13 fixed) |
| 42 | Security reviewer read walls | `tables/walls.toml` | `agents/security-reviewer.md` | Twice, incomplete (S2-22) |
| 43 | Tier table | `tables/roster.toml` | `conventions.md`, 20 agent files | Several, agree (S2-09) |
| 44 | Tier override, `VSDD_TEST_TIER` | `conventions.md` | `topics/model-tiers.md` | Twice, agree (S2-09) |
| 45 | `product-owner` wall row | `tables/walls.toml` | none | Sound (F01 fixed) |
| 46 | `demo-recorder` wall row | `tables/walls.toml` | none | Sound (F01 fixed) |
| 47 | `technical-writer` wall row | `tables/walls.toml` | none | Sound (F01 fixed) |
| 48 | `branches.protected` | `tables/config-keys.toml` | example | Twice, agree |
| 49 | `state.push` | `tables/config-keys.toml` | example, `config/state.md`, `state-commits.md` | Several, agree |
| 50 | `delivery.mode` | `tables/config-keys.toml` | example | Twice, disagree (S2-03) |
| 51 | `delivery.ci` | `tables/config-keys.toml` | example | Twice, disagree (S2-03) |
| 52 | `hooks.rules.auto-allow.scope` | `tables/config-keys.toml` | example | Twice, disagree (S2-03) |
| 53 | `spikes.retain_days` | `tables/config-keys.toml` | example, `topics/worktrees.md` | Several, agree |
| 54 | `state.lock_stale_minutes` | `tables/config-keys.toml` | four other files | Several, agree (S2-09) |
| 55 | `stops.plan_ceiling_agents` | `tables/config-keys.toml` | example, `topics/human-stops.md` | Several, agree |
| 56 | Holdout thresholds | `tables/config-keys.toml` | example, `topics/convergence.md` | Several, agree (S2-09) |
| 57 | The configuration key list | `tables/config-keys.toml` | example, `schemas/config.schema.json` | Three lists, keys agree (S2-03) |
| 58 | Limit defaults | `tables/limits.toml` | `config-keys.toml`, example | Several, agree (S2-03) |
| 59 | Spike limits | `tables/limits.toml` | `topics/spikes.md` | Twice, agree (S2-09) |
| 60 | `convergence.passes`, `rounds` | `tables/config-keys.toml` | `limits.toml`, example | Several, agree |
| 61 | Decision choices | `tables/decision-kinds.toml` | `topics/human-stops.md` | Twice, agree (S2-04) |
| 62 | Choice-to-status mapping | `topics/decisions.md` | linked from the table | Sound |
| 63 | Qualified id, ambiguity refusal | `conventions.md` | `decisions.md`, `hotfix-cycle.md`, three more | Several, agree (S2-04) |
| 64 | Story-kind table | `tables/story-kinds.toml` | none | Once, header misplaces it (S2-17) |
| 65 | Story-kind enumeration | `tables/story-kinds.toml` | `artifacts.toml` `[story]`, `rules/artifacts.md` | Several, agree |
| 66 | `analysis` path class | `tables/path-classes.toml` | none | Sound |
| 67 | `research`, `local` globs | `tables/path-classes.toml` | none | Sound (F34 fixed) |
| 68 | Hub model | `workflows/multi.md` | `topics/multi-repo.md`, `multi-repo-state.md` | Three times, differ (S2-08) |
| 69 | Done test | `topics/workflow-shape.md` | `topics/resume.md` | Twice, agree (S2-06) |
| 70 | Prelude constraints | `packaging/build-step.md` | `topics/prelude.md` | Twice, agree (S2-07) |
| 71 | "The plan is ready" | `tables/routes.toml` | none | Sound (F05 fixed) |
| 72 | Artifact kinds (31 rows) | `tables/artifacts.toml` | `artifacts/<kind>.md` | Twice each, agree (S2-05) |
| 73 | Named-spike branch rule | `topics/worktrees.md` | none | Sound (F10 fixed) |

Sound: 18 of 73 (15, 19, 22, 28, 29, 32, 36, 37, 40, 41, 45, 46, 47, 62, 66, 67, 71, 73).

## The 39 retired entries

| Section | Reason | Verdict |
|---|---|---|
| `BLUEPRINT.md#8` | history | Right: first-pass finding table |
| `BLUEPRINT.md#11` | history | Right: narrative of the second pass |
| `BLUEPRINT.md#11.1` | history | Right: finding table |
| `BLUEPRINT.md#11.2` | history | Wrong in part: item 1 carried uncited, items 2-3 open offers recorded nowhere (S2-15) |
| `BLUEPRINT.md#11.3` | duplicate | Right in substance; reason names files, one of them wrong (S2-24) |
| `BLUEPRINT.md#11.4` | history | Right: close-out rounds |
| `pipeline.md#amendments-from-the-blueprint` | history | Right: twelve items spot-checked, each is in the body text |
| `pipeline.md#2.12` | history | Right: old flow against 1.0 |
| `pipeline.md#3.1` to `#3.7` (7 entries) | history | Right: mappings; the 3.5 market-intelligence row is also in `pipeline.md#2.6` and D20 |
| `pipeline.md#6` | duplicate | Right: D5, D8-D10, D15-D21 |
| `state.md#amendments-from-the-blueprint` | history | Right |
| `state.md#3` | history | Right: old-system mapping |
| `state.md#6` | duplicate | Right: D1-D3, D22, D23 |
| `agents-skills.md#amendments-from-the-blueprint` | history | Right |
| `agents-skills.md#3.1` to `#3.4` (4 entries) | history | Right: mappings |
| `agents-skills.md#6` | duplicate | Right: D5, D6, D13; four files still point at "open question 1" (S2-11) |
| `hooks.md#amendments-from-the-blueprint` | history | Right |
| `hooks.md#2.15` | history | Right: each prevention is carried |
| `hooks.md#2.17` | history | Right: each changed reading is carried (launcher, `spike open`, `spike_exempt`, no PostCompact, silent briefing) |
| `hooks.md#3.1`, `#3.2` | history | Right: mappings |
| `hooks.md#6` | duplicate | Right: D4, D6, D7, D11, D12, D24 |
| `packaging.md#amendments-from-the-blueprint` | history | Right |
| `packaging.md#2.12` | duplicate | Right: `BLUEPRINT.md#9`; a fragment leaked into `build-plan.md` (S2-13) |
| `packaging.md#packaging-and-manifests`, `#bin-21-tools`, `#scripts-ci-benches`, `#tests`, `#ci-and-release` | history | Right: mappings |
| `packaging.md#6` | duplicate | Right: D25-D30 |

Sound: 38 of 39. The two un-retired sections are now cited: `BLUEPRINT.md#4.4` by `topics/method-files.md`
(S2-16) and `state.md#2.17` by `topics/worktrees.md`, which still lacks one design sentence (S2-14).

## Registry

- 456 entries (455 in `registry.toml`, 1 in `registry.d/hooks.toml`; `people.toml` and `records.toml` add
  none). Every entry has exactly its file; no file is unregistered; every `package` names one of the 74
  packages, and every Markdown file's frontmatter `package` equals its registry entry.
- Every package was compared with `BLUEPRINT.md#9`. All are sensible: commands follow the package that
  delivers them (for example `gate-red` G2, `spike-open` S6, `cycle-close` S3, `repos-scan` W12, `version`
  B2, `events-schema` T3), rules follow H2-H9, agents A1-A6b, skills K1-K3b, workflows W1-W12, configuration
  S1, schemas C1 except `status` S2c, `config` S1 and the hook records H1/H8.
- Sound: 454 of 456; the summaries of `table.callers` and `table.roster` are stale (S2-19).

## Totals

Checked: 73 names, 39 retired entries and 456 registry entries, 568 in all. Sound: 18 names, 38 retired
entries and 454 registry entries, 510 in all.
