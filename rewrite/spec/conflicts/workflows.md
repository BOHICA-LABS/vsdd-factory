# Conflicts and gaps found while extracting the workflows group

Written by the workflows extraction step (the 12 workflow files, the 25 workflow-group topics, and the
tables routes, preconditions, story-kinds, step-methods). Nothing here is resolved beyond the choice
stated per entry. Each entry gives both section ids, both texts, and which text the new files use.
The rule applied: section 4 of the blueprint, then the rest of the blueprint, then the detail document
that owns the subject.

## C1. Which perimeters there are

- `BLUEPRINT.md#3.1`: "A **perimeter** is the scope of a review: story, wave or release." (`BLUEPRINT.md#1`
  agrees: "three widening perimeters (story, wave, release)".)
- `pipeline.md#2.1`: "Perimeter | The scope at which review happens: `spec`, `plan`, `story`, `wave`,
  `release`." (`pipeline.md#2.10.3`, the verdict schema, agrees: `"enum": ["spec", "plan", "story",
  "wave", "release"]`. `pipeline.md#2.11.1`, `REACH`, likewise has five keys.)
- Used: the five-perimeter text where the code and the convergence caps are concerned
  (`topics/prelude-converge.md`, `topics/convergence.md` — caps at `spec` and `plan` exist), and the
  three-perimeter phrasing where the design summarises itself. This is the same disagreement recorded
  as C1 in `conflicts/index.md`.

## C2. Do `docs`, `chore` and `harness` stories start at the red gate?

- `pipeline.md#2.9.1`: "`docs`: starts at `red`, the implement step uses technical-writer and the
  green gate runs the documentation checks. `chore` (scaffold, CI, toolchain configuration): starts
  at `red` and the implement step uses devops. `harness` (proofs, fuzz targets, benchmarks): starts
  at `red` and the implement step uses verifier."
- `BLUEPRINT.md#4.6`: for `docs`, `chore` and `harness` the Tests stage is "none" and the Red gate
  column reads "no" (for `docs`: "no; the green gate runs the documentation check (`commands.docs_check`)").
- `pipeline.md#2.9.13`: "No tests stage for `facade`, `docs`, `chore`, `harness`." — agrees with the
  blueprint, not with `2.9.1`.
- Used: `BLUEPRINT.md#4.6` (authority section 4, backed by `pipeline.md#2.9.13`): `docs`, `chore` and
  `harness` have no tests stage and no red gate. Carried in `tables/story-kinds.toml`,
  `workflows/story.md`.

## C3. Cap on open named spikes

- `BLUEPRINT.md#6`: "Spikes per project per day; named spikes open | 20; 3 | `vsdd spike open`."
- `BLUEPRINT.md#4.12`: "Dropped: `limits.ci_waits`, `spikes.max_per_session`,
  `spikes.max_per_requester`. `spikes.max_open` (3) and `spikes.per_day` (20) are enforced by
  `vsdd spike open`."
- `pipeline.md#2.8`: the limits table has "Spikes per project per day | 20" and no named-spikes-open
  row, though the named-spikes paragraph says attaching "does not count against `spikes.max_open`".
- Used: `BLUEPRINT.md#6` with `BLUEPRINT.md#4.12` — the open-named-spikes cap is 3. Carried in
  `topics/named-spikes.md`.

## C4. What enforces the per-run spike limit

- `pipeline.md#2.8`: "Spikes per run | 6 | The wrapper (per script) and `vsdd spike open` (per run
  id, so parent and children share it). A parent passes each child its share of what remains (at
  least 1) and the count used, so the child's wrapper refuses before it starts an agent."
- `BLUEPRINT.md#6`: "Spikes per step, per run | 2, 6 | The wrapper, which refuses before it starts an
  agent; the survey reports how many the run has used."
- Used: `pipeline.md#2.8`, which matches `BLUEPRINT.md#4.12` ("`vsdd spike open` remains the
  backstop"): wrapper first, tool as backstop. Carried in `topics/spikes.md`.

## C5. How many spike entrances are enumerated

- `pipeline.md#2.8`: "The three entrances." — table rows: a workflow step; an agent dispatched in a
  normal session; a person, or a skill on a person's behalf. The prose also covers the main session's
  direct dispatch.
- `BLUEPRINT.md#6`: a six-row table (person; main session; agent dispatched in a session; skill;
  agent running as a workflow step; a workflow calling `runSpike()`).
- Used: both, without forcing agreement — `topics/spikes.md` carries `pipeline.md#2.8`'s asker-
  provenance table and `BLUEPRINT.md#6`'s fuller caller table. Not a contradiction; an enumeration
  difference worth a fix pass.

## C6. The `feature` argument list in the stage specification is incomplete

- `pipeline.md#2.3`: arguments "`request` (required on first run, unless `amend`), `intent`,
  `severity`, `guidance`, `amend` (run the amendment the open cycle is waiting for), `cycle`".
- `pipeline.md#2.9.4`: "Arguments: `request` (required on the first run), `intent` (`feature`,
  `enhancement`, `bug-fix`, `deprecation`, `port`), `severity`, `guidance`, `repo`." — no `amend`,
  no `cycle`, and it drops the "unless `amend`" qualifier on `request`.
- Used: `pipeline.md#2.3` for the argument list (`workflows/feature.md`); the `amend` behaviour is
  described in `pipeline.md#2.6` and used by the survey paragraph of `2.9.4` itself.

## C7. The story.js KIND map is narrower than the method table

- `pipeline.md#2.11.2` (KIND in the skeleton): `chore: { impl: ['devops', 'repo-setup'] }` and
  `harness: { impl: ['verifier', 'proofs'] }`.
- `pipeline.md#2.9.13` and `BLUEPRINT.md#4.6`: `chore` implements with `repo-setup`, `ci` or
  `toolchain`; `harness` with `proofs`, `fuzzing`, `mutation`, `performance` or `clone-fidelity`.
- Used: the method table is the definition ([[table:step-methods]], [[table:story-kinds]]); the
  skeleton is copied verbatim and not corrected, and its own comment says "a story's own `method` key
  wins". Fix pass may want to reconcile the illustration with the table.

## C8. The wave "Ends" line omits `failed`

- `pipeline.md#2.9.2`: "Ends: `done` (next is the next wave or `verify`); `partial`; or
  `needs_human` with decisions queued."
- `pipeline.md#2.11.3` (`close()` in the skeleton): can return `status: 'failed'` (surveys that fail,
  exceptions), and the story workflow's Ends line in `2.9.1` does include "`failed`, which the next
  run retries".
- Used: the workflow file `workflows/wave.md` carries the `2.9.2` Ends line unchanged; the fix pass
  may want to add `failed`.

## C9. Where the driver prose belongs

- `pipeline.md#2.4` ("The driver."): a full description of the `drive` skill's loop, including
  `vsdd driver on|pause|off`, resume after compaction, and the two-stall rule.
- Used: not copied into the route table (`tables/routes.toml` carries only the ordered rows and the
  fixture notes); the driver's home is [[skill:drive]] in the agents-skills group. Flagged so the
  text is not lost to the route-table file.

## C10. Duplicated severity text

- `pipeline.md#2.7` and `BLUEPRINT.md#4.10` carry the same four-row severity table; both say the
  single source is `workflow-lib/severity.md`, stamped into every reviewer file.
- Used: the table is carried once, in [[table:severity]] (not a workflows-group file); the pipeline
  topics link it. Not a disagreement — recorded so the second copy is not created again.

## History in the workflow sources, deliberately not copied

- `pipeline.md#2.3`, final paragraph ("Why these twelve and not one per old file."): a statement of
  what changed from the earlier flow, including the reading of STATE.md; the design fact that the old
  greenfield/brownfield/feature/planning files become routes is carried by `topics/brownfield.md` and
  `tables/routes.toml`.
- `pipeline.md#2.4`, first table ("Old mode or path | 1.0 route"): a mapping from the old system. The
  1.0 route table itself (project state → next) is carried in `tables/routes.toml`; the old-mode
  routes survive as the rows of `topics/brownfield.md` and the route table.
- `pipeline.md#2.6`, closing line ("What happened to the old 37 approval steps is in section 3.5."):
  a pointer to a history section.
- `pipeline.md#2.7`: the opening paragraph ("Why the old rule could not end.") and the closing table
  ("What is dropped from the old convergence document, and what replaces it"): old-system statements.
  The 1.0 rule itself is carried in `topics/convergence.md`.
- `pipeline.md#2.8`, paragraph "This improves on the reading in STATE.md in three ways.": an
  amendment note against an earlier document.
- `pipeline.md#2.11`, paragraphs two and three: the correction list for the skeletons and the
  simulation account ("Before the corrections the three blocks were parsed with Node..."), including
  the note that the corrected behaviours are conformance fixtures. These are amendment/verification
  history; the platform-limits paragraph, the skeletons and the trailing scale figures are carried
  (`topics/prelude.md`, `topics/story-script.md`, `topics/wave-script.md`).
- `BLUEPRINT.md#5.1`, the two lists "Amendments made here" and "Amendments made by the second pass":
  amendment lists. Every design fact they state is carried in the owning pipeline sections and in
  this group's files; the "Final design." opening paragraph is reflected in `topics/workflow-shape.md`.

## Gaps noticed while copying (no conflict, recorded for the fix pass)

- `pipeline.md#2.9.2` says the wave gate's holdout steps run "one per batch of up to `holdout.batch`
  (12) scenarios"; the number 12 also appears in the configuration key table (`BLUEPRINT.md#4.12`),
  but the wave stage table hard-codes it in prose — one place should win.
- `pipeline.md#2.9.1` red-gate bullet cites "the class precedence of blueprint 4.7" for test-file
  classification; the path-class table is `tables/path-classes.toml` in this layout. The workflow
  file says "the class precedence of the path classes" and links nothing, because the pathway is the
  path-classes table's to define.
- `pipeline.md#2.9.13` says "Methods this table needs that the first pass did not have:
  `equivalence-tests` and `test-fixes` (test-writer), `recover-contracts` (product-owner),
  `verification-assessment` (architect), `docs-drift` (technical-writer) and `pattern-sweep`
  (code-reviewer)." — this is change-note phrasing; the method-to-agent mapping itself is carried in
  `tables/step-methods.toml`, and the first-pass comparison was not copied.
- `pipeline.md#2.10.7` (the workflow result) is referenced by the design ("Each returns a workflow
  result (2.10.7)") but this extractor did not carry the schema; it belongs to
  [[schema:workflow-result]], linked throughout.
- `pipeline.md#2.4` (driver paragraph) states the driver lives in the main session "because a
  workflow may nest only one level"; the nesting rule is restated in `workflows/story.md` from
  `pipeline.md#2.3`. Both texts are carried; they agree.
