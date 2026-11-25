# Repair record: agents, skills, artifacts, packaging, the four documents, and the tables and topics of the agents, state, packaging and index groups

Written by the repair pass for those files. It lists every disagreement between the original documents met
while repairing, and every gap (a value the originals do not give). Nothing here is resolved beyond the
precedence rule: section 4 of the blueprint, then the rest of the blueprint, then the detail document that
owns the subject. Disagreements already recorded in the conversion's conflicts files (`conflicts/agents.md`,
`conflicts/rules.md`, and the rest) are referenced, not restated.

## Disagreements between originals

### P-1. The security reviewer's `analysis` grant (audit F13, `conflicts/agents.md` A-1)

`BLUEPRINT.md#4.3` row 16 grants "`reviews` and `analysis`, create only";
`agents-skills.md#2.9` and `#2.6` grant one named file, `analysis/<scope>/security.md`, once, during an
ingest. The repair follows section 4 of the blueprint: the `write` key of the
`vsdd-factory:security-reviewer` row of `tables/walls.toml` is the one statement, and
`agents/security-reviewer.md` links to it. The "Used" note of A-1 was changed to match.

### P-2. The tier table appears in both the blueprint and the agents design (audit F22)

`BLUEPRINT.md#4.2` and `agents-skills.md#2.12` both carry the tier table. They agree on every value
(`fable`/`sonnet`/`opus` at `xhigh`, clerk `low`), so no choice was needed: `doc:conventions` carries the
blueprint's table verbatim (with its "Used for" column), `tables/roster.toml` `[tiers]` carries the data,
and `topic:model-tiers` links to both instead of listing values again.

### P-3. The prelude constraints are restated in two files

The sentence of `packaging.md#2.5` (audit F08) is carried verbatim in `packaging/build-step.md`, per the
audit's resolution. `topics/prelude.md` also quotes it. The prelude topic is not in this pass's area; if
one of the two should yield, the topic is the one to change, keeping the constraints in
`packaging:build-step`, which the audit named.

### P-4. `state.md#2.17` is history and design in one section (audit F15, F10)

The section's retirement was dropped because its design content is now carried and its id is cited by
`topics/worktrees.md`, which quotes the retention rule with
the named-spike branch sentence and reaches the project-ledger reason through `[[config:spikes]]`. The
remaining passages of 2.17 (how the design departed from `rewrite/STATE.md` readings) are history and
remain uncarried. Recorded because the section cannot be half-retired by the tooling: it is now wholly
accounted for through the citation.

### P-5. `BLUEPRINT.md#4.4` is a mapping, not history (audit F15)

The retirement was wrong: the provisional-names table is design, and `conflicts/agents.md` A-6 relies on
it. `tables/methods.toml` carries it row for row as `[[replaced-name]]`, and `topics/method-files.md`
copies the section's two framing sentences and cites `BLUEPRINT.md#4.4`.

## Gaps: values the originals do not give

### P-6. The `ux-designer` closing command is nowhere stated (`conflicts/agents.md` A-5)

The tool table gives the set VSDD to all four planners, names a closing command for three
(`vsdd gate spec --only <part>`, `vsdd gate plan --story`, `vsdd feature route`) and none for the
ux-designer. Carried as written; the `[callers]` grant for its row decides what the agent may run, or
leaves it empty.

### P-7. The check names `formal` and `perf` are never defined (`conflicts/agents.md` A-6)

`BLUEPRINT.md#4.4` names them as checks of `vsdd gate release` and `vsdd gate wave`; no method or gate
check of either name is defined anywhere in the six documents. The row is carried verbatim in
`tables/methods.toml` (`replaced-name`). The names need defining or renaming in a review pass.

### P-8. The roster's `[workflows]` source is abbreviated (audit F26)

`agents-skills.md#2.12` lists `story` and `wave` and stops at "and so on for feature, verify, maintain and
multi". The complete statement is the Calls column of `pipeline.md#2.3`, which the shipment carries;
`BLUEPRINT.md#4.5` adds the nesting rule, that only `story` is ever called by another workflow. The
shipped roster's `[workflows]` rows are the one copy of the call edges (README rule 7). (F26 read 4.5's
"only story is ever called by another workflow" as "no other call edges", which it does not say.)

### P-9. The artifact identifier for `toolchain` points at a shape, not a schema

The cell in `state.md#2.5` reads "TOML: `[commands]`, `test_report`, `[paths]`, in the shapes of section
2.12" — a pointer in the original. `tables/artifacts.toml` now renders it as "in the shapes of
[[config:commands]] and [[config:paths]]", the files that carry those shapes. No JSON-Schema-strength
definition of the shape exists in the originals.

### P-10. Two cells in `packaging.md` point into the same document

The `RELEASING.md` row of the documentation table says "Section 2.8, about fifty lines" (2.8 is titled
"Release"); `packaging/documentation.md` renders that as "The release procedure, about fifty lines" and
links [[packaging:plugin-release]]. The `scripts/`, `ci/`, `benches/` deletion row says "See the mapping in
section 3"; `packaging/cutover-deletions.md` says the old-system mapping is kept under `rewrite/history/`,
because the mapping is retired history here. Both substitutions replace a pointer with the text or home of
the thing, per the no-pointer rule.

## What was restored in this pass (audit finding, where)

- F06/F26 `tables/roster.toml`: `[platform]` (`min_claude_code = "2.1.331"`, `python_floor = "3.11"`),
  `name = tier` agent rows, the twelve workflows with the one call edge.
- F25 `tables/artifacts.toml`: one row per kind from the tables of `state.md#2.5`.
- F02/F13/F14 `tables/walls.toml` and the two agent files: one wall statement per fact, agent files
  linking to [[table:walls]] (`agents/holdout-evaluator.md`, `agents/security-reviewer.md`).
- F03 `topics/state-api.md`: the S7 signatures and contracts, `path_class`, `verdict`.
- F05 `tables/routes.toml` header: the "the plan is ready" definition, copied from `pipeline.md#2.4`.
- F08 `packaging/build-step.md`: the prelude constraints, copied from `packaging.md#2.5`.
- F16 `topics/run-lock.md`: the open offer P2LEAN-17, quoted, so `doc:platform` probe 29 has a referent.
- F15/F31 `retired.toml`: `BLUEPRINT.md#4.4` and `state.md#2.17` un-retired (see P-4, P-5); the five
  open-question sections, `BLUEPRINT.md#11.3` and `packaging.md#2.12` given reason "duplicate" with the
  id that holds the text.
- F20 `packaging/tests-workflows.md`: the six story cases, three wave cases and twenty checks of
  `pipeline.md#2.11` as the B5 test list.
- F33 `tables/step-methods.toml` header: the brief/author method sentences and the `stubs` merger;
  `tables/severity.toml`, `tables/lint-checks.toml` and `tables/methods.toml` verified cell by cell
  against `agents-skills.md#2.7`, `packaging.md` layer 3, `agents-skills.md#2.8` and `BLUEPRINT.md#4.4`.
- F35 the pointer prose of `topics/artifact-rules.md`, `topics/run-lock.md`,
  `packaging/documentation.md`, `packaging/cutover-deletions.md` and `tables/artifacts.toml` replaced
  with links or text (P-9, P-10).
- F22/HISTORY the history passages struck from the agents, artifact and packaging files of this group
  (the `Replaces the old` lines, the conversion-pass remarks), keeping the design's own
  convergence-pass vocabulary, which is content, distinct from conversion history.
- `topics/state-directory.md`: the seven rules of storage of `state.md#2.2` restored verbatim (rules 2,
  4, 5 and 7 had been dropped behind links).
