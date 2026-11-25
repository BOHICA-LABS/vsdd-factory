# Second audit: prose fidelity and the earlier findings

Written by the second audit step. It compares the files under `rewrite/spec/` with the six originals
(`rewrite/BLUEPRINT.md`, `rewrite/design/{pipeline,state,agents-skills,hooks,packaging}.md`) after the
repair. It reviews the conversion only, not the design, and fixes nothing. Line numbers are from the files
as they stand on `rewrite/v1` at the time of this audit.

## Verdict

Not yet faithful enough to build from, but much closer. Of the 35 findings of `conflicts/audit.md`, 28 are
fixed as asked; 7 are not, or were fixed in a way that left a new defect (F08, F14, F22, F23, F26, F27,
F35). Two new blockers were found, both in machine tables: the roster's `[workflows]` table says that
`feature`, `verify`, `maintain` and `multi` call nothing, and the artifact registry's only `[story]` row is
the hook design's illustrative excerpt. Prose and code skeletons are otherwise copied closely; the launcher
skeleton, the three `rule-model` code blocks and the annotated `factory.toml` are byte-identical to the
originals.

Counts: 2 blockers, 9 major, 12 minor. Sections compared: 108; faithful: 86.

Both checks pass: `spec_lint.py` reports `0 problem(s)`; `sections.py coverage` reports `sections: 240;
cited by a spec file: 181; retired: 39; uncovered: 0`.

## Method

Each original section was split into sentences and table cells (cells of six characters or more), and each
fragment was looked up, punctuation and link syntax removed, first in the files whose frontmatter or
`Sources:` header cites the section, then in the whole spec. Unmatched fragments were read in context. About
50 sections were read side by side in full (marked * below); the rest were compared fragment by fragment
with every unmatched fragment read against the new file. Code blocks were compared with `difflib`. The
earlier findings were checked one by one by reading the files they name and searching for their siblings.

## Status of the 35 earlier findings

Fixed: F01, F02, F03, F04, F05, F06 (its `[platform]` and row-format halves), F07, F09, F10, F11, F12, F13,
F15, F16 (in substance; see P2-11), F17, F18, F19 (see P2-12 for a header residue), F20, F21, F24, F25 (except
the `[story]` row, P2-02), F28 (its two named items; see P2-15), F29, F30, F31, F32, F33, F34.

Not fixed or fixed with a new defect: F08 (P2-08), F14 (P2-09), F22 (P2-03), F23 (P2-04), F26 (P2-01),
F27 (P2-06), F35 (P2-12).

## Findings

### P2-01. The roster says four workflows that call `story` call nothing (blocker, altered; F26)

- Originals: `agents-skills.md#2.12` line 901: `wave = { calls = ["story"] }    # and so on for feature,
  verify, maintain and multi; the others call nothing`. `pipeline.md#2.3` lines 219-230, column "Calls":
  `story` for `feature`, `wave`, `verify`, `maintain`, `multi`; line 251: "Nesting rule: `wave`, `feature`,
  `verify`, `maintain` and `multi` may call `story`." `BLUEPRINT.md#4.12` line 700: "`runStories()`, starts
  child stories for `wave`, `verify`, `feature`, `maintain` and `multi`".
- New: `tables/roster.toml` lines 64, 68, 70, 71 give `feature`, `verify`, `maintain` and `multi`
  `{ calls = [] }`; only `wave` (line 66) calls `story`. The header (line 58) cites `BLUEPRINT.md#4.5`
  ("Only `story` is ever called by another workflow"), which names the callee, not the callers.
  `conflicts/people-repair.md` P-8 (lines 63-67) claims 4.5 says "no other call edges"; it does not.
- Effect: L-ROSTER and L-XREF built from this table reject four shipped workflows, or a builder removes
  their child-story calls. `workflows/story.md` line 14 and `topics/workflow-shape.md` state the opposite.
- Proposed resolution: `calls = ["story"]` for `feature`, `verify`, `maintain` and `multi`, citing
  `pipeline.md#2.3`; correct P-8.

### P2-02. The registry's `[story]` row is the illustrative excerpt (blocker, altered; F25 residue)

- Originals: `state.md#2.5` line 565 gives the story frontmatter: `id`, `title`, `status` (draft, dropped),
  `cycle`, `kind`, `method`, `depends_on`, `contracts`, `properties`, `points`, `priority` (P0, P1, P2),
  `criticality`, `security`, `ui`, `demo`, `screens`, `repo`, `source`. `hooks.md#2.11.3` line 1057 shows a
  `[story]` row with five of them, marked `# illustrative; the content is state.md section 2.5`.
- New: `tables/artifacts.toml` lines 42-54 hold only the illustrative row (`frontmatter = { id, title,
  status, kind, contracts }`); the "every kind" part (from line 59) has no `[story]` row. The header (lines
  8-9) says the kind cells are the tables of `state.md#2.5`. `artifacts/story.md` lines 20-30 list all 18
  keys, so the two statements disagree.
- Effect: `frontmatter` is the allowed-key list; a registry built from this row makes the `artifacts` rule
  refuse every planned story that carries `depends_on`, `criticality` or `points`.
- Proposed resolution: build `[story]` from `state.md#2.5` line 565, in the format of the `[contract]` row;
  link `artifacts/story.md` to the row instead of restating the keys.

### P2-03. Facts are still defined in two to five files (major, earlier-finding-not-fixed; F22)

- Green gate, red gate, re-freeze and delivery: `workflows/story.md` lines 63-70, 83-87, 89-99, 143-157 and
  again, as quotations, in `commands/gate-green.md` line 19, `commands/gate-red.md` lines 19-37 and
  `commands/story-deliver.md` lines 20-40 (source `pipeline.md#2.9.1` lines 838-870).
- Decision choices: `topics/human-stops.md` lines 17-75 restate every choice that `tables/decision-kinds.toml`
  holds (source `pipeline.md#2.6`, `BLUEPRINT.md#4.11`).
- Ambiguous decision id refused: `conventions.md` 150, `commands/decide.md` 32, `skills/decide.md` 39,
  `topics/hotfix-cycle.md` 15, `topics/decisions.md` 31 (RR11 fixed only the records group's file).
- Spike limits: `topics/spikes.md` lines 92-99 and `tables/limits.toml` lines 85-98.
- Holdout thresholds: `topics/convergence.md` line 101 and `tables/config-keys.toml` from line 361.
- The tier override: `conventions.md` lines 119-122 and `topics/model-tiers.md` lines 31-35.
- The `vsdd explain` example: `topics/event-log.md` lines 46-53 and `commands/explain.md` from line 21.
- Proposed resolution: keep each in its owner file and link from the others.

### P2-04. History is still copied into design files (major, earlier-finding-not-fixed; F23)

- "Replaces ..." (the old-agent column of `agents-skills.md#2.4`): `agents/code-reviewer.md` 10,
  `agents/codebase-analyzer.md` 10, `agents/holdout-evaluator.md` 10 (F23 named only `agents/clerk.md`).
- Design-pass history: `rules/spike-bounds.md` 107 ("The first pass let ..."), `topics/event-log.md` 15
  ("the first pass's list of reserved event types"), `topics/caller-identity.md` 17, `topics/rule-tuning.md`
  47, `agents/test-writer.md` 46, `packaging/tests-conformance.md` 40 ("The second pass names these"),
  `topics/run-lock.md` 54 ("The blueprint's second pass offered ... (P2LEAN-17)").
- Converter commentary about the conversion inside design files: `topics/computed-status.md` 26-30,
  `topics/state-layout.md` 60-63, `tables/story-kinds.toml` 7-9 ("recorded as a disagreement").
- Proposed resolution: strike them; keep the commentary in the conflicts files.

### P2-05. The per-mode route table of `pipeline.md#2.4` was dropped as history (major, lost)

- Original: `pipeline.md#2.4` lines 268-280, column "1.0 route": the greenfield sequence; brownfield;
  standard feature; trivial ("`verify` (suite and traceability only)"); bug fix ("suite, scoped hidden
  scenarios, one delta review, security if the module is critical or high, affected proofs"); critical bug
  ("the story branches from the last release tag, `verify` runs the suite and one delta review"); discovery
  ("stop `ideas` → brief or `feature` for each idea pursued"); multi-repository; per-story delivery.
- New: `conflicts/workflows.md` lines 118-120 treat the whole table as an old-system mapping. Only the
  importer sentence survives (`topics/brownfield.md` 59-60). The left column is a mapping; the right column
  is design.
- Unrecorded disagreement: trivial route "suite and traceability only" (2.4) against `pipeline.md#2.9.6`
  line 1051 "`trivial`, baseline only", which `workflows/verify.md` 88 carries.
- Proposed resolution: carry the right column (in `topic:brownfield` or the `table:routes` header) and
  record the 2.4/2.9.6 disagreement.

### P2-06. The flaky-test and first-release rules are stated two ways (major, earlier-finding-not-fixed; F27)

- Originals: `BLUEPRINT.md#4.13` lines 746-750: "An inherited test that fails and then passes when the gate
  runs the suite once more does not fail the gate" and "A cycle with no release, ingest or import baseline
  is a first release". `state.md#2.8` lines 786-789: "passed at the baseline and fails now is run once more
  ...; one that passes in either run does not fail the gate" and "A cycle whose baseline is empty".
- New: `records/baseline.md` lines 28-31 use `state.md#2.8`; `commands/gate-green.md` line 41 uses the
  blueprint's wording. `conflicts/records-repair.md` RR10 chose the detail document, against the rule that
  section 4 of the blueprint wins.
- Proposed resolution: use `BLUEPRINT.md#4.13` in `record:baseline`, link from `command:gate-green`.

### P2-07. A word added to the docs-story green gate (major, fixed-during-conversion)

- Originals: `BLUEPRINT.md#4.6` line 395: "the green gate runs the documentation check
  (`commands.docs_check`)"; `pipeline.md#2.9.1` line 882: "the green gate runs the documentation checks".
- New: `commands/gate-green.md` lines 23-24: "A `docs` story's green gate 'runs the documentation check
  (`commands.docs_check`)' instead". No original says the check replaces the build, suite, lint and format
  commands.
- Proposed resolution: drop "instead".

### P2-08. The prelude constraints are now in two files, one reworded (major, defined-twice; F08)

- Original: `packaging.md#2.5` lines 464-467.
- New: `packaging/build-step.md` lines 61-65 and `topics/prelude.md` lines 91-97. `build-step.md` line 64
  changes "one of those names" to "one of the prelude's top-level names".
- Proposed resolution: one copy, as written, in `packaging:build-step`; `topic:prelude` links to it.

### P2-09. Links still send readers to `table:callers` for `[callers]` and `[reveals]` (major, other; F14)

- After F14, `[callers]` and `[reveals]` are in `tables/walls.toml` and `tables/callers.toml` holds only the
  three constants. Yet `rules/read-walls.md` 74 and `topics/information-walls.md` 54 write
  "[[table:callers]] `[reveals]`", and `agents/architect.md` 42, `agents/story-writer.md` 44,
  `agents/ux-designer.md` 38, `agents/researcher.md` 40, `agents/product-owner.md` 46,
  `agents/codebase-analyzer.md` 41, `rules/gate-guard.md` 30 and 48, `tables/command-families.toml` 23 and
  `topics/command-conventions.md` 30 link `table:callers` for the subcommand groups. The registry summary
  (`packages/C2.md` line 51) still says the table holds the groups and the reveals.
- Proposed resolution: link `table:walls` for groups and reveals; ask the registry owner to update the
  summary.

### P2-10. The event log's example line is gone (minor, lost)

- Original: `hooks.md#2.12` lines 1462-1467, the example `rule.block` line.
- New: in no file. `schemas/event.schema.json` line 5 calls itself a "machine copy of the documented line"
  that the spec no longer shows; its properties omit `cwd`, an extra field of `agent.start`/`agent.stop`.
- Proposed resolution: copy the example into `topic:event-log`; add `cwd` to the schema's properties.

### P2-11. Two open offers retired; the third is quoted without its source (major, wrongly-retired)

- Original: `BLUEPRINT.md#11.2` lines 1668-1675: item 2 (no lessons register, "If you count that list as a
  capability in its own right, say so") and item 3 (importer and spike rule kept apart, "Say so if you
  prefer the merge") are offers a person can still take.
- New: `retired.toml` retires 11.2 as history; items 2 and 3 are in no file. Item 1 is quoted in
  `topics/run-lock.md` 52-60, whose `sources` omit `BLUEPRINT.md#11.2`.
- Proposed resolution: record the three offers once, where item 1 now is, cite 11.2, and stop retiring it.

### P2-12. Pointers into the original documents remain (major, earlier-finding-not-fixed; F35)

- `packaging/cli-entry.md` 36: "Which subcommands exist is in `BLUEPRINT.md#4.9`".
- `topics/rule-testing.md` 11: "hooks.md section 2.14 supplies the formats"; `topics/risks-hooks.md` 9:
  "Copied in substance from hooks.md section 7"; `topics/information-walls.md` 74; `topics/hook-runner.md`
  28 ("follows `packaging.md` section 2.3"); `rules/artifacts.md` 32; `rules/secrets.md` 65.
- "blueprint section 10, item N" instead of `[[doc:platform]]`: `packaging/plugin-manifest.md` 119 and 128,
  `packaging/e2e-story-tier.md` 20 and 37, `packaging/proof-greenfield.md` 23-24.
- `tables/limits.toml` lines 5-6: "the three spike rows" (there are two) and "its sections 2.7 and 2".
- `commands/status.md` 18: "(2.10)" inside a quotation.
- Proposed resolution: replace each with a `[[kind:id]]` link.

### P2-13. A quotation was edited inside its quotation marks (minor, altered)

- `commands/story-deliver.md` line 21 inserts "(`Factory-Story: S-7.03`)" into a sentence quoted from
  `pipeline.md#2.9.1` line 858, which has no such text (the trailer is `state.md#2.11` line 1019).
- Proposed resolution: move the trailer outside the quotation.

### P2-14. The end-to-end tier costs were dropped (minor, lost)

- Original: `packaging.md#layer-4` lines 735-739, column "Rough cost": "a few dollars", "tens of dollars;
  measured in step 4 of the plan", "about a dollar".
- New: `packaging/tests-e2e.md` lines 30-62 have no cost per tier.
- Proposed resolution: copy the column.

### P2-15. The spike definition and the `ref` rule follow the detail document (minor, altered; F28 scope)

- `BLUEPRINT.md#6` lines 1126-1127: "an isolated, bounded experiment or piece of research ... There is one
  protocol and one agent". `topics/spikes.md` 14-15: "an isolated, bounded experiment ... There are two
  sizes of the same protocol" (`pipeline.md#2.8`). Not recorded.
- `pipeline.md#2.8` lines 747-749: "A platform worktree starts from the main checkout's head ... so the
  runner checks out `ref` first." Now only a risk (`topics/risks-pipeline.md` 73-77); `conflicts/workflows.md`
  retired the paragraph as history.
- The person row of the blueprint's entrance table (`topics/spikes.md` 59) drops the named-spike sentence.
- Proposed resolution: use the blueprint's wording; carry the `ref` sentence in `command:spike-open`.

### P2-16. The state storage cost table was paraphrased (minor, altered)

- Original: `state.md#decision-keep-the-separate-branch-and-take-every` lines 240-249, column "In 1.0".
- New: `topics/state-directory.md` 56-62 compresses it. Lost: "No questions are asked"; "Two machines are
  reconciled by git when state is pushed"; "Nobody composes or reads state commit messages"; "CI mounting
  the branch for validators → Optional. A project that wants `vsdd validate` in CI adds two lines."
- Proposed resolution: copy the column as written.

### P2-17. The list of commands that are not built is gone (minor, lost)

- Original: `state.md#2.16` lines 1399-1402: `vsdd flush`, `lock status`, `stale`, `review verdict`/`diff`/
  `path`, `defer resolve`, `lesson retire`, `research retire`, `job wait`, `job status`, `step record`, each
  with its reason.
- New: only `lock status` (`topics/run-lock.md` 41) and the retire trio (`commands/retire.md` 26) survive;
  `rules/step-results.md` 64-65 replaces the `step record` sentence with a converter-written one ("so no
  fallback that needs a tool in the agent can stand in").
- Proposed resolution: copy the list into `topic:command-conventions`; restore the original sentence of
  `hooks.md#2.11.7` lines 1300-1302 without its history clause.

### P2-18. `rules/secrets.md` adds a claim and a misplaced sentence (minor, invented)

- Line 69: "Enforcement data such as `secret_globs` is project configuration owned by the state design" -
  no original says this; `secret_globs` is not in the annotated `factory.toml` of `state.md#2.12`, which is
  "the whole key list". The gap is not recorded.
- Lines 19-20 carry the `git commit -F` attribution sentence of `hooks.md#2.11.1`, which belongs to
  `bash-git`.
- Proposed resolution: strike both; record the missing `secret_globs` key as a gap.

### P2-19. `tables/rules.toml` coins tool names and drops a column (minor, invented)

- Lines 71, 101, 115: `any-guarded`, `any-guarded-for-lock-liveness`, `web-and-research` are not names in
  any original (`hooks.md#2.10` lines 925, 930, 932 give prose).
- The "What it checks" column of `hooks.md#2.10` is in no file as written.
- Proposed resolution: keep the original cell text as the value, or record the encoding; copy the column.

### P2-20. The default of `hooks.rules.auto-allow.scope` is a paraphrase (minor, altered)

- `BLUEPRINT.md#4.12` line 726: "set at setup". `tables/config-keys.toml` line 619: "written once by vsdd
  init, from the answer setup collected", which RR3 calls the blueprint's meaning but is not its text.
- Proposed resolution: use "set at setup"; keep RR3's record of the two example values.

### P2-21. Two smaller losses (minor, lost)

- `pipeline.md#2.11.3` lines 2190-2191: "`verify`, `feature`, `maintain` and `multi` call the same
  `runStories()` and treat its `skipped` list the same way" is not in `topics/wave-script.md`.
- `agents-skills.md#2.6`, security-reviewer row: "Nothing in the pipeline" is replaced in
  `agents/security-reviewer.md` line 30 by a link to the wall row; the pipeline/on-demand distinction that
  the row states is gone.
- Proposed resolution: copy both sentences.

### P2-22. `source-index.json` is stale (minor, other)

- 20 sections differ from the files' own `sources`: for example `pipeline.md#2.11` lacks
  `packaging/tests-workflows.md`, `BLUEPRINT.md#4.4` shows `[]` though `tables/methods.toml` cites it, and
  `state.md#2.5` lacks `records/closure.md`.
- Proposed resolution: regenerate it with `sections.py`.

### P2-23. `tables/artifacts.toml` mixes two row formats (minor, other)

- Lines 29-54 use the hook format (`glob`, `id`, inline `frontmatter`, `max_bytes`); lines 61-280 use prose
  strings (`path`, `frontmatter = "..."`, `cap`). The header (lines 13-21) documents only the first.
- Proposed resolution: one format, the hook design's, for every row; record any key the originals do not
  give.

## Sections compared

Not faithful (22): `agents-skills.md#2.12`*, `pipeline.md#2.3`*, `state.md#2.5`, `hooks.md#2.11.3`*,
`pipeline.md#2.4`*, `pipeline.md#2.9.1`*, `state.md#2.16`, `BLUEPRINT.md#4.13`, `state.md#2.8`,
`packaging.md#2.5`*, `hooks.md#2.12`*, `BLUEPRINT.md#11.2`*, `packaging.md#layer-4-end-to-end-headless`*,
`pipeline.md#2.8`, `BLUEPRINT.md#6`*, `state.md#decision-keep-the-separate-branch-and-take-every`*,
`hooks.md#2.11.1`*, `hooks.md#2.10`*, `hooks.md#2.11.7`*, `pipeline.md#2.11.3`, `BLUEPRINT.md#4.12`*,
`agents-skills.md#2.6`.

Faithful (86):

- `BLUEPRINT.md`: `#2.3`, `#3.2`, `#3.3`, `#4.2`, `#4.3`*, `#4.5`*, `#4.11`*, `#4.14`, `#5.3`*, `#7`*,
  `#9.1`, `#9.6`.
- `pipeline.md`: `#2.2`, `#2.5`*, `#2.6`*, `#2.7`*, `#2.9.2`, `#2.9.3`, `#2.9.4`, `#2.9.5`*, `#2.9.6`,
  `#2.9.7`*, `#2.9.8`, `#2.9.9`, `#2.9.10`, `#2.9.11`, `#2.9.12`, `#2.9.13`, `#2.10.8`*, `#2.11`*, `#2.11.2`,
  `#7`.
- `state.md`: `#2.2`, `#2.6`, `#2.7`, `#2.9`, `#2.10`*, `#2.11`, `#2.12`* (the example; see P2-20 for the
  key table), `#2.13`, `#2.14`, `#2.15`*, `#2.17`*, `#2.18`, `#4`*, `#mounting`, `#layout`,
  `#who-commits-state-and-when`, `#7`.
- `agents-skills.md`: `#2.2`, `#2.3`, `#2.4`*, `#2.5`, `#2.7`, `#2.8`, `#2.9`*, `#2.10`, `#7`.
- `hooks.md`: `#1`, `#2.1`*, `#2.2`*, `#2.3`, `#2.4`, `#2.5`*, `#2.6`*, `#2.7`*, `#2.8`, `#2.9`*,
  `#2.11.2`*, `#2.11.4`*, `#2.11.5`*, `#2.11.6`*, `#2.11.8`*, `#2.11.9`*, `#2.11.10`*, `#2.13`, `#2.14`,
  `#2.16`, `#7`.
- `packaging.md`: `#2.6`, `#layer-1-python-tests-pytest-through-uv`,
  `#layer-2-workflow-tests-node-built-ins-only`*, `#layer-2b-conformance-fixtures`,
  `#layer-3-lint-over-every-shipped-file-pytest-test`*, `#2.7`*, `#2.8`.

"Faithful" here means every normative sentence and table cell arrived, sometimes reformatted from a table
into a list, with history clauses removed; a faithful section can still carry a duplicate (P2-03) or a
history phrase (P2-04), which are counted against the file, not the section.
