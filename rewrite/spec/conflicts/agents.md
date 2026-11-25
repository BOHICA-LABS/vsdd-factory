# Conflicts and gaps: agents group

Found while writing the agents, skills, tables and topics of the agents group. Nothing here is resolved beyond
the rule already applied: section 4 of the blueprint, then the rest of the blueprint, then the detail document
that owns the subject. Each entry gives the section ids, both texts, and which text the new files use. The
repair pass's own record is `conflicts/people-repair.md`.

## Disagreements between documents

### A-1. What `security-reviewer` may write outside `reviews`

- `BLUEPRINT.md#4.3` (row 16 of the roster): "`reviews` and `analysis`, create only".
- `agents-skills.md#2.9` (write scopes): "`adversary`, `code-reviewer`, `security-reviewer`, `ui-reviewer` |
  `reviews`, creating new files only, in an on-demand review. `security-reviewer` may also create
  `analysis/<scope>/security.md` during an ingest". `agents-skills.md#2.6` (Writes column) agrees with the
  narrower form: "Nothing in the pipeline, except `analysis/<scope>/security.md` once, during an ingest. On
  demand only: one review report."
- Used (changed in the repair pass, per the audit's F13): the blueprint's class grant, because section 4 of
  the blueprint outranks the detail document. The one statement is the `write` key of the
  `vsdd-factory:security-reviewer` row in `tables/walls.toml` (`reviews:create`, `analysis:create`);
  `agents/security-reviewer.md` links to that row instead of restating it.

### A-2. Whether the frozen-test caveat is part of `verifier`'s write scope

- `BLUEPRINT.md#4.3` (row 10): "`harness`, `tests`".
- `agents-skills.md#2.9` (write scopes): "`harness` and `tests`, but not the tests frozen at a story's red
  gate".
- Used: `agents-skills.md#2.9`. The blueprint states the classes but drops the frozen-test exclusion; the
  design's `walls.toml` rows carry it as `:frozen`, and the tests-before-code rule turns on the split
  (need N-HOOK-4). The agent file `agents/verifier.md` carries the caveat.

## Gaps: names used but never defined, fields with no statement

### A-3. The `holdout-evaluator` row breaks the tool-set notation

- `agents-skills.md#2.5` (tools table, row `holdout-evaluator`): "`Read`, SHELL, SPAWN, RESULT, BROWSER".
- Every other row names sets in capitals only; this one spells a literal tool name inside the set list. It is
  the only place `Read` appears spelled out, and it is doing work there: the point of the row is that the
  evaluator keeps `Read` but loses `Grep` and `Glob`, so it cannot trawl the source tree. Recorded because a
  lint or generator keyed on set names will trip over the row. Carried verbatim into
  `agents/holdout-evaluator.md` and `topics/tool-sets.md`.

### A-4. Two "Is given" cells never name the method

- `agents-skills.md#2.6` (Is given column): every row begins "Method name;" except `demo-recorder` ("Story
  path; worktree path; the criteria to show; how to start the product (`vsdd env start`)") and
  `holdout-evaluator` ("The scenario identifiers to run; how to start the product (`vsdd env start`). No
  source, test or specification path.").
- Yet the same design requires one method per step (`agents-skills.md#2.8`: "One step, one method"), and both
  agents have exactly one: `demo` and `holdout-eval`. The clerk's cell ("One `vsdd` command") is consistent,
  since it has no method. Carried as written (conversion, not redesign); the fix belongs to a review pass.

### A-5. `ux-designer` has the VSDD set but no closing command

- `agents-skills.md#2.5`: the `ux-designer` tool row is "READ, EDIT, VSDD, SPAWN, RESULT, WEB, STORYBOOK"
  with the lack "No general shell." — no `vsdd` command is named for it.
- `BLUEPRINT.md#4.3` (rules across the roster): "The four planners use it to finish a step with the check for
  their own work (`vsdd gate spec --only <part>`, `vsdd gate plan --story`, `vsdd feature route`)" — three
  commands for four planners; the other three rows each name their command (`agents-skills.md#2.5`).
- Used: as written. The command the ux-designer may run is nowhere stated; the `walls` `callers` table will
  have to pick, or the row stays empty.

### A-6. Check names `formal` and `perf` versus the method name `performance`

- `BLUEPRINT.md#4.4`: "`formal-verifier`, `performance-engineer`, `dtu-validator` | `verifier`, inside harness
  stories; the measuring itself is the `formal`, `perf` and clone-fidelity checks of `vsdd gate release` and
  `vsdd gate wave`, run by a clerk".
- The methods table (`agents-skills.md#2.8`) and the step table (`pipeline.md#2.9.13`) name the method
  `performance`; no method or gate check named `formal` or `perf` is defined anywhere in the six documents.
  Carried verbatim into `tables/methods.toml` (`replaced-name` entry). The gate check names need defining or
  renaming in the review pass.

### A-7. Section 2.6 names package `A6`; the registry splits it

- `agents-skills.md#2.6` (preamble): "Package A6 builds the reviewer files from this table."
- The registry (and the blueprint's build plan) has no `A6`: the reviewer files are built by `A6a`
  (`adversary`, `code-reviewer`, `security-reviewer`) and `A6b` (`holdout-evaluator`, `ui-reviewer`,
  `scenario-reviewer`).
- Used: the registry's split; the agent files' frontmatter carry `A6a`/`A6b`.
