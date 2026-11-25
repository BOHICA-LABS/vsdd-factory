# Audit of the restructure into rewrite/spec/

Written by the audit step. It compares the files under `rewrite/spec/` with the six original documents
(`rewrite/BLUEPRINT.md`, `rewrite/design/{pipeline,state,agents-skills,hooks,packaging}.md`). It reviews the
conversion only, not the design. Nothing here is fixed; every finding names both locations and a proposed
resolution for the fixing pass. Line numbers in the originals are from the files as they stand on
`rewrite/v1`; line numbers in the new files are from the files as they stand at the time of this audit.

## Verdict

Not faithful enough to build from. The lint and coverage checks pass, but coverage only counts citations.
Most prose and the code skeletons were copied closely, and the seven result schemas of `pipeline.md#2.10.1`
to `#2.10.7` are byte-equivalent. The machine tables are where the conversion changed the design. In
`tables/walls.toml` the rows the converters wrote themselves deny the wrong classes and grant the wrong
command groups, and the holdout evaluator's row is copied from an excerpt marked "shortened here". The
configuration defaults are restated in two places that disagree, with meanings written by the converter.
`tables/roster.toml` lost its `[platform]` table and changed its row format. Several interface contracts
were dropped: the state functions' signatures, the rule `check()` signature, the definition of "the plan is
ready", the prelude's constraints and the spike question-matching rule. Two retirements are wrong
(`BLUEPRINT.md#4.4`, `state.md#2.17`). History was copied into many rule and topic files, and about a dozen
facts are defined in two or three files.

Counts: 10 blockers, 16 major, 9 minor.

## The two checks

- `python3 rewrite/tools/spec_lint.py`, last lines: `registry: 20 agent, 31 artifact, 62 command, 22 config,
  4 doc, 74 package, 32 packaging, 15 record, 17 rule, 47 schema, 13 skill, 21 table, 85 topic, 12 workflow`
  and `0 problem(s)`.
- `python3 rewrite/tools/sections.py coverage`, last lines: `sections: 240; cited by a spec file: 179;
  retired: 41; uncovered: 0` and `text: 972 KB total; 783 KB cited; 188 KB retired`.

Both pass. `sections.py coverage` checks only that a section is cited or retired. It does not check that
the text arrived, which is why the findings below were not caught.

## Method

Every original section was split into sentences and table cells. Each fragment of 25 characters or more
was looked up, with markup and link syntax normalised, first in the files that cite the section and then in
the whole spec. I read every fragment that matched nowhere, in context, against the new files. About 35
sections were also read side by side in full. A second pass checked every number that carries a unit, such
as minutes, KB or characters. A third pass looked for long sentences that occur in two or more spec files.
Every JSON schema was checked with `jsonschema` (Draft 2020-12 `check_schema`) and for structure. Every
TOML table was parsed, and its contents were compared with what its name and registry entry claim.

## Findings

### F01. Translated wall rows deny and grant the wrong things (blocker, altered)

`hooks.md#2.7` shows wall rows for only 11 of the 20 agents. The other nine rows in `tables/walls.toml` were
written by the converters from `agents-skills.md#2.9` (`conflicts/rules.md` R9). Five of those rows break
the rules that the originals apply to every row:

- `agents-skills.md#2.9` lines 563-565: "every caller is denied `runs` ... and agents with no shell or a
  `vsdd`-only shell, the clerk and the spike runner may not read outside the project".
- `hooks.md#2.7` lines 757-761: the same rule for `outside`, and "Every row denies `runs`".
- `BLUEPRINT.md#4.3` lines 293, 298 and 308 give the shells, and line 322 says "They may run read and
  measure commands only". `hooks.md#2.7` lines 805-809: "A builder row grants `read` and `measure`; a
  planner row the same".
- `state.md#2.16` lists who may run each gate: `gate green` "The story's builder, `clerk`"; `gate demo`
  "`demo-recorder`, `clerk`".

The rows in `rewrite/spec/tables/walls.toml`:

- line 41, `product-owner`: `read_deny = []`. Its shell is `vsdd` only, but the row denies neither `runs`
  nor `outside`. It has no `tool_only` and no `vsdd` key, so it can run only the `read` group and cannot run
  its closing `vsdd gate spec --only <part>`. This also contradicts `rules/gate-guard.md`: "A row with
  `tool_only = true` (the clerk and the four planners)".
- line 72, `researcher`: it has no shell, yet `outside` is not denied. The `hooks.md` note the table copies
  says the researcher must deny it.
- line 148, `security-reviewer`: it has no shell, and `outside` is not denied.
- line 109, `demo-recorder`: `vsdd = ["read", "env"]` has no `measure`, so the agent cannot run
  `vsdd gate demo`.
- line 116, `technical-writer`: there is no `vsdd` key, which means `read` only, so it cannot run the green
  gate that ends a `docs` story.

Proposed resolution: derive the nine rows again from `agents-skills.md#2.9` and `BLUEPRINT.md#4.3`, using
the defaults of `hooks.md#2.7` exactly. Record each derivation in `conflicts/rules.md`.

### F02. The holdout evaluator's row is the excerpt marked "shortened here" (blocker, altered, defined twice)

- Original: `hooks.md#2.7` line 709 ends its row with `# shortened here`. The full deny list is in
  `agents-skills.md#2.9` line 579, and `BLUEPRINT.md#4.3` line 329 says it "cannot read source, tests, CI,
  stories, or any specification except the brief".
- New: `tables/walls.toml` line 160 copies the shortened list, comment included. It leaves out `harness`,
  `manifests`, `repo-config`, `ci`, `evidence`, `stories`, `research`, `analysis`, `spikes` and `spec-ux`.
  `agents/holdout-evaluator.md` lines 40-42 give the full list, so the wall is now stated twice and the two
  statements differ.
- Proposed resolution: put the full list in the table row, and have the agent file link to the row instead
  of restating it.

### F03. The state functions lost their signatures and contracts (blocker, lost)

- Original: `state.md#4` row S7 (line 1606) and `hooks.md#5` row S7 (line 1868), which says "the two are
  built to one signature each". The signatures are `store_step(project, result, agent_type, head)`, which
  refuses a record over its cap; `settle_decision(project, id, choice, values, note, session_id)`, which
  raises with a reason when the decision is not open, the id is ambiguous, the choice is invalid or the
  digest is stale; `spike_entry(project, agent_id, worktree)`, which returns `opened`, `verdict`,
  `asker_type` and `read_deny`; `spike_admit(project, session_id, dispatcher_type)`;
  `spike_attach(project, agent_id, cwd)`; `story_kind(project, story)`; `kind_has_red_gate(kind)`;
  `caller(key)`; `status(root)`. Row 6 of `state.md#4` adds "each returning within one second except
  `flush`". Row S3 adds `vsdd.project.path_class(path)`. `state.md#2.8` adds
  `vsdd.state.verdict(project, subject)`.
- New: `topics/seams-state.md` and `topics/seams-hooks.md` list only the function names, and say they
  "define nothing". No file carries `store_step(`, `settle_decision(`, `spike_entry(`, `spike_admit(`,
  `spike_attach(`, `story_kind(`, `caller(key)`, `path_class(` or `verdict(project`. Only `evidence(`,
  `spec_locked(`, `artifact_locked(`, `guidance_locked(`, `unit_tree(`, `briefing(root`, `status(root`,
  `detect(root` and `flush(root` survive.
- Proposed resolution: give the state API a file of its own (for example, a topic linked from
  `record:step`, `record:decision` and `rule:spike-bounds`) that holds the S7 text as written.

### F04. The rule `check()` signature and the runner's main loop were dropped (blocker, lost)

- Original: `hooks.md#2.5` lines 428-439 give the declaration with
  `def check(ctx: Ctx, cfg: RuleConfig) -> Outcome | list[Outcome] | None`, and lines 484-500 give the main
  loop.
- New: `topics/rule-model.md` turns the declaration into prose and has no signature.
  `rules/tests-first.md` lines 24-31 copy the decorator without the `def` line, and shorten or drop its
  comments (for example, "locked rules cannot be relaxed in project configuration"). No file has
  `RuleConfig` or the main loop; `topics/hook-runner.md` line 63 paraphrases the loop.
- Proposed resolution: copy both code blocks as written into `topic:rule-model`.

### F05. The definition of "the plan is ready" is gone (blocker, lost)

- Original: `pipeline.md#2.4` lines 312-318: "The `plan` unit is closed when a `plan` run ends with the plan
  gate passed, and a `revise` on a `plan` decision reopens it. "The plan is ready", the precondition of
  `wave`, means that unit is closed and no `plan` decision is open." It continues: "a cycle whose change
  request has no route yet matches neither the `plan` row nor the no-plan case of the `verify` row".
  Line 308: an ingested or imported project with no brief is in steady state.
- New: `tables/routes.toml` copies the rows and part of the prose but not these sentences. The phrase "the
  plan is ready" appears only as a precondition (`tables/preconditions.toml` line 22, `workflows/wave.md`
  line 12) and is defined nowhere.
- Proposed resolution: copy the paragraph as written into `table:routes` (in its header comment) or into
  `topic:waves`.

### F06. The roster lost its `[platform]` table and changed its row format (blocker, lost and altered)

- Original: `packaging.md#layer-3-...` lines 668-697 give the roster with
  `[platform] min_claude_code = "2.1.331"` and `python_floor = "3.11"`. `[agents]` is `name = tier`, both
  there (line 679) and in `agents-skills.md#2.12` line 862. The L-MAN check (line 647) reads
  `[platform] min_claude_code`.
- New: `tables/roster.toml` has no `[platform]`, but `tables/lint-checks.toml` line 8 still checks against
  it. Its `[agents]` rows became inline tables, `{ tier, package, tool_sets }`, with keys no source gives
  this file.
- Proposed resolution: restore `[platform]`. Keep `name = tier`. Leave packages to the registry and tool
  sets to `topic:tool-sets`.

### F07. Configuration defaults and meanings were changed (blocker, altered)

- Original: `state.md#2.12` lines 1043-1194 hold the annotated `factory.toml`, which "is the whole key
  list: every section and every key the loader accepts appears in it, with its default". Among its values:
  `protected = ["main"]` (line 1055), `mode = "pr"` (line 1090), `ci = "required"` (line 1091) and
  `scope = "off"` (line 1190). `state.md` line 367: `state.push = true` is "the default when the
  repository has a remote".
- New: `config/branches.md` lines 26-29 and `tables/config-keys.toml` line 51 give `branches.protected` the
  default `"[]"` and the meaning "Branches a story branch is never merged to directly", which no source
  states. `config/state.md` line 15 gives `state.push` the default `true` without the condition.
  `delivery.mode` and `delivery.ci` become `"detected"`, which follows `BLUEPRINT.md#4.12`, but the
  disagreement with `state.md#2.12` is not recorded anywhere. The annotated example itself is copied nowhere.
- Proposed resolution: copy the annotated example once, as written. Record the 4.12 against 2.12
  disagreements in `conflicts/records.md`. Remove the meanings no source gives (see F10).

### F08. The prelude's constraints were dropped behind a link (blocker, lost)

- Original: `packaging.md#2.5` lines 464-467: "declarations only (`const` and `function`); no `import` or
  `export`; no `Date.now()`, `Math.random()` or argument-less `new Date()`; no reference to anything a
  single workflow defines; a header comment listing every top-level name it introduces; at most 400 lines".
- New: `packaging/build-step.md` line 61 says "Constraints on the prelude are the pipeline design's
  ([[topic:prelude]])". `topics/prelude.md` lines 91-96 carry only the platform limits on scripts. "Declarations
  only", the header comment, the ban on names a workflow defines and the 400-line cap are stated in no file.
- Proposed resolution: copy the sentence as written into `packaging:build-step`.

### F09. The rule for matching an earlier spike finding was lost (blocker, lost)

- Original: `BLUEPRINT.md#6` lines 1131-1133: an earlier finding is returned for the same question "matched
  by a hash of the normalised question".
- New: `topics/spikes.md` uses the wording of `pipeline.md#2.8`, which says only "the same question". No
  file mentions a normalised question. The blueprint outranks the detail document, and the difference was
  not recorded.
- Proposed resolution: use the blueprint's wording in `topic:spikes` and `command:spike-open`, and record it.

### F10. Spike retention was rewritten, and the named-spike branch rule was lost (blocker, altered)

- Original: `BLUEPRINT.md#6` line 1170 and `state.md#2.12` line 1122 keep "a finished spike's worktree" for
  14 days, or until the asking unit is delivered. `state.md#2.17` lines 1455-1458 add that "the branch
  `spike/<name>` of a named spike is kept until the spike is closed".
- New: `topics/worktrees.md` lines 30-32 and `commands/spike-close.md` line 20 say "Named-spike worktrees are
  kept `spikes.retain_days` (default 14), or until the asking unit is delivered". `topics/spikes.md` line
  110 still applies the rule to every spike worktree. The branch rule is in no file, because `state.md#2.17`
  was retired (see F15).
- Proposed resolution: restore both rules as written, in `topic:worktrees` only, and link to them elsewhere.

### F11. Configuration keys are defined twice, and the copies disagree (major, defined twice)

- `tables/config-keys.toml`, every `config/*.md` (each has its own TOML block of key, type, default and
  meaning) and `schemas/config.schema.json` all define the same keys.
- Where they disagree: `hooks.rules.auto-allow.scope` has the default "set at setup" in the table (line
  561) but "off (set at setup)" in `config/hooks.md` line 28. The strength key is
  `hooks.rules.<rule>.strength` in the table but `hooks.rules.tests-first.strength` in the config file.
  `caps.<kind>` is `caps.<artifact kind>` in the config file. `paths.*` defaults are "detected" in the table
  but example globs in `config/paths.md`.
- Proposed resolution: one table holds key, type, default and owner. The config files keep prose and link
  to the table rows. The schema becomes a pointer, or the reverse; either way there is one list.

### F12. The config files carry meanings the sources never give (major, altered)

- Examples from `config/*.md`, none found in any original: "Build of the product.", "Lint command.",
  "Scanner.", "Branches a story branch is never merged to directly.", "Push the state branch at the
  checkpoint.", "Remove demo, evaluation, run and evidence files of closed cycles.", "How long event files
  are kept.", "Allow pull-request close.", "Commit messages and pull request text.", "Where decision
  notifications go." In all, 50 of the 96 `meaning` strings contain a sentence that occurs in no original.
- Proposed resolution: keep only meanings quoted from a source, such as the `state.md#2.12` comments and
  the `BLUEPRINT.md#4.12` table. Leave the rest empty.

### F13. The security reviewer's `analysis` grant is stated three ways (major, defined twice)

- Originals: `BLUEPRINT.md#4.3` line 308 says "`reviews` and `analysis`, create only". `agents-skills.md#2.9`
  and `#2.6` allow only `analysis/<scope>/security.md`, once, during an ingest.
- New: `tables/walls.toml` line 150 has `analysis:create`, following the blueprint (`conflicts/rules.md`
  R1). `agents/security-reviewer.md` lines 40-41 follow the narrower text (`conflicts/agents.md` A-1, which
  chose the detail document against the precedence rule). `tables/path-classes.toml` (class `analysis`)
  says "may create security.md".
- Proposed resolution: one statement, chosen by the precedence rule (`BLUEPRINT.md#4.3`) and kept in
  `table:walls`. Change the A-1 resolution to match.

### F14. `walls.toml` changed its own format (major, altered)

- Original format, `hooks.md#2.7` lines 646-754: top-level tables `[main]`, `[other]` and
  `["vsdd-factory:<agent>"]`, with `[callers]` and `[reveals]` in the same file. L-POLICY
  (`packaging.md#layer-3`) requires "`walls.toml` has `[callers]` and `[reveals]`".
- New: `tables/walls.toml` writes `[walls.main]` (nested) next to `["walls.vsdd-factory:..."]` (a single
  top-level key that contains a dot). Parsed, this gives top-level keys `walls` (holding only `main` and
  `other`) and 19 separate `walls.vsdd-factory:*` keys. `[callers]` and `[reveals]` moved to
  `tables/callers.toml`.
- Proposed resolution: use the original format, with `[callers]` and `[reveals]` in the same table.

### F15. Two retirements are wrong (major, wrongly retired)

- `BLUEPRINT.md#4.4` is retired as history (`retired.toml` line 6), yet `tables/methods.toml` cites it in
  its header and carries its whole table as `[[replaced-name]]` (from line 349). That table is a mapping
  from the pipeline design's provisional names, and `conflicts/agents.md` A-6 relies on it.
- `state.md#2.17` is retired as history (line 78), but it is the only source of the named-spike branch rule
  (F10) and of the reason for a project ledger outside spike worktrees.
- Proposed resolution: for 4.4, either drop `[[replaced-name]]` and its citation, or cite 4.4 and stop
  retiring it. For 2.17, copy the two design sentences to their owners, then retire the rest.

### F16. An open offer was retired, but `platform.md` still points to it (major, wrongly retired)

- Original: `BLUEPRINT.md#11.2` item 1 offers lock liveness by process (P2LEAN-17): "Until you say, the
  heartbeat is the design."
- New: the section is retired, but `platform.md` line 266 still says "the offered cut P2LEAN-17", which no
  spec file defines.
- Proposed resolution: record the open offer once, in `doc:decisions` or `topic:run-lock`, or remove the
  reference.

### F17. The `spike open --step` syntax has two forms (major, defined twice)

- Originals: `pipeline.md#2.8` line 715 gives `--step <workflow>/<subject>/<name>`. `state.md#2.16` line
  1410 gives `--step <workflow>/<subject>/<name>.<attempt>`.
- New: `topics/spikes.md` line 50 carries the first form, `commands/spike-open.md` line 15 the second.
  Neither file records the disagreement.
- Proposed resolution: define the syntax only in `command:spike-open`, record the disagreement, and link
  to it from `topic:spikes`.

### F18. A wrapped `--` was joined into a flag (major, altered)

- Original: `pipeline.md#2.3` lines 241-242: `/vsdd-factory:spike open uv-startup --` / `measure cold and
  warm start`, where `--` is the note separator of the `spike open <name> -- <mission>` grammar.
- New: `topics/workflow-arguments.md` line 22 has `uv-startup --measure cold and warm start`. The same
  file says "The harness runs every workflow with its documented argument text", so the error would end up
  in a conformance fixture.
- Proposed resolution: restore `-- measure`.

### F19. The limits table gained a row no source has and lost two rows (major, altered and lost)

- Original: `pipeline.md#2.5` lines 420-435.
- New: `tables/limits.toml` drops "Review passes: plan, ingest | 2 | Script | Open findings go to the packet
  or report" and "Token budget, if the person set one | none | Platform `budget` | `partial`". It adds a
  `ci_minutes` row whose `enforced_by` and `at_the_limit` appear in no source. Its `at_the_limit` fields
  send the reader to "Section 2.7 of the pipeline design", a retired document.
- Proposed resolution: copy the table row for row and link to `topic:convergence` instead.

### F20. The script harness cases were dropped as history (major, lost)

- Original: `pipeline.md#2.11` lines 1566-1579 list six story cases, three wave cases and twenty checks of
  corrected behaviour, then say "Package B5 turns these and the original nine cases into harness tests".
- New: no file lists them. `conflicts/workflows.md` calls the paragraph a "simulation account".
- Proposed resolution: copy the case list into `packaging:tests-workflows` as the B5 test list.

### F21. The launcher skeleton was replaced by a link to the old document (major, lost)

- Original: `hooks.md#2.2` lines 219-262, the `launch.sh` skeleton. It has the exact uv-missing message
  ("brew install uv (or: curl -LsSf https://astral.sh/uv/install.sh | sh)"), the `[uv_missing]` and
  `[launch_rc_N]` codes, the migration line and disabled briefing in `brief` mode, and the exit-code mapping.
- New: `topics/hook-launcher.md` line 49 says "The full launcher script is copied in hooks.md section 2.2".
  Some of its facts survive in `tables/environment.toml` and `packaging/cli-entry.md`. The message text and
  the per-mode branches do not.
- Proposed resolution: copy the skeleton as written into `topic:hook-launcher`.

### F22. The same facts are defined in several files (major, defined twice)

These were found by the name checks (see the table below) and by searching for repeated sentences:

- The hub model is written out in both `topics/multi-repo.md` and `workflows/multi.md` (7 long sentences in
  common).
- The green gate pass condition, the red gate pass condition and the behaviour of `vsdd story deliver` are
  in `workflows/story.md` and again in `commands/gate-green.md`, `commands/gate-red.md` and
  `commands/story-deliver.md`.
- Decision choices are in `tables/decision-kinds.toml`, `topics/decisions.md` lines 25-31 and
  `topics/human-stops.md` lines 60-80.
- The tier table is in `conventions.md` lines 111-116, `topics/model-tiers.md` lines 14-19 and
  `tables/roster.toml` `[tiers]`.
- Paragraphs of the auto-allow rule appear in both `rules/auto-allow.md` and `topics/tool-call-approval.md`.
- The `[contract]` row is in both `tables/artifacts.toml` and `artifacts/contract.md`. The `[story]` and
  `[_unknown]` rows are in both `tables/artifacts.toml` and `rules/artifacts.md`.
- The caller-identity paragraph is in `topics/run-lock.md` and `topics/caller-identity.md`.
- The holdout thresholds are in `topics/convergence.md` and `config/holdout.md`.
- Spike limits are in `topics/spikes.md`, `config/spikes.md` and `tables/limits.toml`.
- The rule that an ambiguous decision id is refused is in `conventions.md`, `topics/decisions.md`,
  `commands/decide.md`, `skills/decide.md` and `topics/hotfix-cycle.md`.
- Proposed resolution: keep each fact in its owner file and link to it everywhere else.

### F23. History was copied into design files (major, history leaked)

- Examples: `rules/compaction.md` line 12 ("named `compaction-snapshot` in the first pass");
  `rules/step-results.md` lines 28 and 68; `topics/hook-wiring.md` lines 12-13 and 57;
  `rules/write-scopes.md` line 61 ("Case table additions from the second pass"); `topics/verdicts.md` ("the
  first design pass wrote `review.<sha12>.json`"); `topics/artifact-rules.md` and `tables/artifacts.toml`
  (`code_values`, "the second pass removed"); `tables/callers.toml` ("Three lists that the first pass kept
  as data"); `topics/tool-call-approval.md` ("Two second-pass reviewers showed"); and the "Replaces the old
  ..." lines in `rules/merge-guard.md`, `rules/lifecycle-log.md`, `rules/purity-advice.md` and
  `agents/clerk.md`.
- Proposed resolution: strike these passages and record their section ids in `retired.toml` as
  passage-level history.

### F24. List fields in the schemas are written as objects (major, altered)

- Original: `pipeline.md#2.10.8` gives `claim_checks`, `coverage`, `resolutions`, `rulings`, `scores`,
  `symptoms`, `modules`, `epics` and `preconditions` as lists of records.
- New: `review-data`, `fix-data`, `ruling-data`, `holdout-data`, `module-map-data`, `outline-data`,
  `score-data` and `survey` in `schemas/` give these fields `properties` with no `"type": "array"` and no
  `items`. Every schema passes meta-validation, but an array value bypasses every property check, so the
  shapes are not described. `config.schema.json` types every section as a bare `object` with no
  `additionalProperties: false`, although its own description says "unknown keys ... are errors".
- Proposed resolution: wrap each list field as an array of the given item properties, and no more. This
  is a translation of the design's own words, not a redesign.

### F25. `artifacts.toml` holds 2 of the 31 kinds (major, table incomplete)

- Original: `state.md#2.4` says the content of `artifacts.toml` is the tables of `state.md#2.5`, one row per
  kind.
- New: `tables/artifacts.toml` holds only `[contract]`, `[story]` and `[_unknown]`. The other kinds' rows
  are prose in `artifacts/*.md`, while the registry summary promises "one row per artifact kind". The
  wall-table translations show how rows written by hand at build time drift from their source (F01).
- Proposed resolution: put every row in the table, from `state.md#2.5`, and have the artifact files link
  to it.

### F26. The roster's `[workflows]` table lists 2 of the 12 workflows (major, table incomplete)

- `tables/roster.toml` copies `agents-skills.md#2.12` line 901, "and so on for feature, verify, maintain
  and multi". L-ROSTER needs every workflow listed. The source itself is abbreviated, so this must be
  recorded rather than silently copied.
- Proposed resolution: record it in `conflicts/agents.md`, so the fixing pass fills the table from
  `BLUEPRINT.md#4.5`.

### F27. The baseline record does not use the blueprint's wording (minor, altered)

- `BLUEPRINT.md#4.13` lines 743-751 define the first release as having "no release, ingest or import
  baseline", and describe the flaky test as one that "fails and then passes". `records/baseline.md` uses
  the `state.md#2.8` wording instead. `conflicts/records.md` R4 records only the field list.
- Proposed resolution: record both differences.

### F28. `topics/spikes.md` uses `pipeline.md#2.8` where the blueprint's section 6 differs (minor, altered)

- It drops "dispatch agents" from the experiment step, and `vsdd status --fix` as a retention enforcer.
  Only C3 to C5 in `conflicts/workflows.md` are recorded.
- Proposed resolution: follow `BLUEPRINT.md#6` and record the rest.

### F29. Command files add commentary between quotations (minor, altered)

- `commands/list.md`: "a kind is refused, not silently shortened". `commands/gate-spec.md`: "(The blueprint
  row says "the planners" generically; the state design names the owning planner.)".
- Proposed resolution: move the commentary to `conflicts/commands.md`.

### F30. The event type lists differ (minor, defined twice)

- `schemas/event.schema.json` enumerates `runner.call`, which `tables/event-types.toml` lacks.
- Proposed resolution: one list, with the other linking to it.

### F31. Retirement reasons are wrong (minor)

- Every entry in `retired.toml` says "history", and none says "duplicate". The open-question sections
  (`pipeline.md#6`, `state.md#6`, `agents-skills.md#6`, `hooks.md#6`, `packaging.md#6`), `BLUEPRINT.md#11.3`
  (whose comment says "carried by ...") and `packaging.md#2.12` (superseded by `BLUEPRINT.md#9`) are
  duplicates under the README's definition and should name the section that holds the text.

### F32. A conflicts note is factually wrong (minor)

- `conflicts/commands.md` item 8 says "The two governed merge commands are not enumerated anywhere", but
  `hooks.md#2.7` line 802 names them: `GOVERNED`, `story deliver` and `release`.

### F33. Small sentences lost (minor, lost)

- `pipeline.md#2.9.13` lines 1272-1274: "Outside the workflows, the `brief` skill uses `brief` ... the
  `author` skill uses `conventions`" and "`stubs` is merged into `failing-tests`".
- `hooks.md#2.3` lines 371-377: the expected-time column (8 ms, 190 ms on the first call, 60-90 ms, 80 ms
  plus the flush, 55 ms).
- `hooks.md#2.4`: the frontmatter reader of about 120 lines.
- `state.md#5` P-7: "a packet of 12 KB". `records/decision.md` keeps the cap, but the reason is gone.

### F34. Globs in `path-classes.toml` contain prose (minor, altered)

- `research` has `".factory/research/** except inputs/"` and `local` has
  `".factory/local/** apart from runs and evidence"`. A program cannot match these.

### F35. New files still point into the original documents (minor)

- Examples: `tables/limits.toml` ("Section 2.7 of the pipeline design"), `rules/spike-bounds.md`
  (`pipeline.md` section 2.8), `topics/artifact-rules.md` (`hooks.md` section 2.11.3),
  `topics/run-lock.md` ("the answer to `pipeline.md` S8 and `packaging.md` S4"),
  `topics/hook-launcher.md` (F21).
- Proposed resolution: replace each reference with a `[[kind:id]]` link.

## Retired entries checked (20 of 41; every entry is "history")

| Section | Holds | Verdict |
|---|---|---|
| `BLUEPRINT.md#4.4` | Provisional names mapped to the roster | Wrong: carried and cited by `tables/methods.toml` (F15) |
| `BLUEPRINT.md#8` | First-pass findings | Right |
| `BLUEPRINT.md#11.1` | Second-pass finding table | Right |
| `BLUEPRINT.md#11.2` | Cuts offered and applied | Wrong while `platform.md` cites P2LEAN-17 (F16) |
| `BLUEPRINT.md#11.3` | Three consequences | Right in substance; its reason should be duplicate (F31) |
| `BLUEPRINT.md#11.4` | Close-out rounds | Right |
| `pipeline.md#2.12` | Old flow against 1.0 | Right |
| `pipeline.md#3.4` | Old step kinds | Right |
| `pipeline.md#3.5` | The 37 old approval steps | Right |
| `pipeline.md#6` | Open questions | Answered in `BLUEPRINT.md#2`; reason should be duplicate |
| `state.md#2.17` | Departures from STATE.md | Wrong: sole source of a design rule (F10, F15) |
| `state.md#3` | Old-system mapping | Right |
| `agents-skills.md#3.2` | Old skills mapped | Right |
| `agents-skills.md#6` | Open questions | Answered by D5, D6 and D13; reason should be duplicate |
| `hooks.md#2.15` | Old defects and preventions | Right; each prevention is carried, for example in `topic:rule-testing` |
| `hooks.md#2.17` | Departures from STATE.md | Right; "no PostCompact handler" is in `rules/compaction.md` |
| `hooks.md#3.1` | The 79 old registry entries | Right |
| `packaging.md#2.12` | The old build plan | Right; reason should be duplicate of `BLUEPRINT.md#9` |
| `packaging.md#6` | Open questions | Answered by D25 to D30 |
| `packaging.md#ci-and-release` | Old CI mapping | Right |

## Thirty names checked for a single definition

| Name | Defined in | Result |
|---|---|---|
| `[commands]` and the other config keys | `tables/config-keys.toml`, `config/*.md`, `schemas/config.schema.json` | Twice, disagreeing (F11) |
| `branches.protected` | `config/branches.md`, `tables/config-keys.toml` | Twice, invented meaning (F07) |
| `state.push` | `config/state.md`, `topics/state-commits.md` | Twice, disagreeing (F07) |
| `hooks.rules.auto-allow.scope` | `config/hooks.md`, `tables/config-keys.toml` | Twice, disagreeing (F11) |
| Wall rows | `tables/walls.toml`, `agents/holdout-evaluator.md`, `agents/security-reviewer.md` | Twice, disagreeing (F02, F13) |
| `analysis` grant | `walls.toml`, `path-classes.toml`, `agents/security-reviewer.md` | Three times (F13) |
| `vsdd spike open` | `commands/spike-open.md`, `topics/spikes.md` | Syntax twice (F17) |
| Spike limits | `topics/spikes.md`, `config/spikes.md`, `tables/limits.toml` | Three times (F22) |
| Green gate | `workflows/story.md`, `commands/gate-green.md` | Twice (F22) |
| Red gate | `workflows/story.md`, `commands/gate-red.md` | Twice (F22) |
| `vsdd story deliver` | `workflows/story.md`, `commands/story-deliver.md` | Twice (F22) |
| Hub model | `topics/multi-repo.md`, `workflows/multi.md` | Twice (F22) |
| Decision choices | `tables/decision-kinds.toml`, `topics/decisions.md`, `topics/human-stops.md` | Three times (F22) |
| Tiers | `conventions.md`, `topics/model-tiers.md`, `tables/roster.toml` | Three times (F22) |
| `auto-allow` scopes | `rules/auto-allow.md`, `topics/tool-call-approval.md` | Paragraphs repeated (F22) |
| `[contract]` registry row | `tables/artifacts.toml`, `artifacts/contract.md` | Twice (F22) |
| `[story]` and `[_unknown]` rows | `tables/artifacts.toml`, `rules/artifacts.md` | Twice (F22) |
| Caller marker | `topics/caller-identity.md`, `topics/run-lock.md`, `records/caller-marker.md` | Paragraph repeated (F22) |
| Holdout thresholds | `topics/convergence.md`, `config/holdout.md` | Twice (F22) |
| Ambiguous decision id | five files | Five times (F22) |
| Event types | `tables/event-types.toml`, `schemas/event.schema.json` | Twice, disagreeing (F30) |
| Limits | `tables/limits.toml`, `config/limits.md`, `tables/config-keys.toml` | Three times (F11, F19) |
| Route table | `tables/routes.toml` | Once; linked from `skills/drive.md` |
| Preconditions | `tables/preconditions.toml` | Once; workflow files repeat their own line (acceptable) |
| Story kinds | `tables/story-kinds.toml` | Once |
| Severity | `tables/severity.toml` | Once; `topics/convergence.md` links |
| `HUMAN_ONLY` and `GOVERNED` | `tables/callers.toml` | Once |
| Rule list | `tables/rules.toml` and the rule files | Once each |
| Lint checks | `tables/lint-checks.toml` | Once; `packaging/tests-lint.md` links |
| Identifiers | `tables/identifiers.toml` | Once; `conventions.md` summarises and links |

## Schemas and tables

- All 47 JSON files parse, have `$id` equal to their file name, and pass `jsonschema`'s Draft 2020-12
  `check_schema`. The seven pipeline schemas (`step-result`, `finding`, `verdict`, `spike-request`,
  `spike-finding`, `gate-result`, `workflow-result`) equal `pipeline.md#2.10.1` to `#2.10.7` exactly.
  Thirty-six schemas have no required list and fifteen carry empty `{}` property schemas. Both are recorded
  in `conflicts/records.md` part 2 as gaps in the design. List fields are written as objects (F24).
- All 21 TOML tables parse. Five do not hold what their names say: `walls.toml` (F01, F02, F14),
  `artifacts.toml` (F25), `roster.toml` (F06, F26), `limits.toml` (F19) and `path-classes.toml` (F34).

## Sections compared

Compared fragment by fragment, with every unmatched fragment read in context (about 35 of them also read
side by side in full):

- `BLUEPRINT.md`: `#1`, `#2`, `#2.1`, `#2.2`, `#2.4`, `#3.1`, `#3.4`, `#4.1`, `#4.3`, `#4.4`, `#4.5`,
  `#4.6`, `#4.7`, `#4.8`, `#4.9`, `#4.10`, `#4.11`, `#4.12`, `#4.13`, `#4.14`, `#5.1`, `#5.2`, `#5.4`, `#6`,
  `#7`, `#9.1`, `#10`.
- `pipeline.md`: `#2.3`, `#2.4`, `#2.5`, `#2.6`, `#2.7`, `#2.8`, `#2.9.1`, `#2.9.2`, `#2.9.3`, `#2.9.4`,
  `#2.9.6`, `#2.9.8`, `#2.9.10`, `#2.9.11`, `#2.9.13`, `#2.10.1` to `#2.10.8`, `#2.11`, `#2.11.1`, `#2.11.3`.
- `state.md`: `#decision-keep-the-separate-branch-and-take-every`, `#mounting`, `#layout`,
  `#who-commits-state-and-when`, `#2.4`, `#2.7`, `#2.8`, `#2.9`, `#2.10`, `#2.11`, `#2.12`, `#2.13`,
  `#2.15`, `#2.16`, `#4`, `#5`.
- `agents-skills.md`: `#2.3`, `#2.4`, `#2.5`, `#2.7`, `#2.8`, `#2.9`, `#2.10`, `#2.11`, `#2.12`.
- `hooks.md`: `#2.2`, `#2.3`, `#2.5`, `#2.6`, `#2.7`, `#2.8`, `#2.9`, `#2.10`, `#2.11.1` to `#2.11.8`,
  `#2.11.10`, `#2.12`, `#2.13`, `#2.14`, `#4.2`, `#5`.
- `packaging.md`: `#2.4`, `#2.5`, `#layer-3-lint-over-every-shipped-file-pytest-test`,
  `#layer-4-end-to-end-headless`, `#2.7`, `#2.8`, `#after-cutover`, `#2.13`, `#the-eight-projects`.

That is about 100 sections. About 80 carry every normative sentence, sometimes reworded. The 20 or so that
do not are named in the findings above.
