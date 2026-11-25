# Tables and schemas repair record

Written by the fixing pass for the second audit's machine files (26 findings T2-01 to T2-26, plus the
parts of the prose and structure audits that concern a file under `tables/` or `schemas/`). The rule
applied throughout: the six originals are right by definition; every row, key, default, enumeration,
type and required list is traceable to original text; where the originals disagree, section 4 of
BLUEPRINT.md wins, then the rest of the blueprint, then the detail document that owns the subject.
Disagreements are recorded here. Nothing under `next/` was changed.

## Where the originals disagree, and what the tables now say

- **T2-15 / RR10, `delivery.protected_paths`.** `BLUEPRINT.md#4.12` and `pipeline.md#2.6` end the
  list with "and the files `[release]` names"; `state.md#2.12` line 1095 adds the literal
  `CHANGELOG.md`. Used: the blueprint's default (RR10, added to `records-repair.md`).
- **T2-23, the claim's fields.** `pipeline.md#2.10.8` line 1524: a holdout claim has "`key`,
  `resolution` and `commit` only"; `BLUEPRINT.md#4.10` line 615: "carries the key alone". The schema
  (`schemas/claim.schema.json`) follows the blueprint: every field is declared, none is required,
  and the description carries both texts with the blueprint's precedence.
- **T2-24, `spec-gap.opened_when`.** `BLUEPRINT.md#4.11` line 651: "A reviewer showed the approved
  specification is silent or wrong"; `pipeline.md#2.6` line 490 adds "on a point the story depends
  on". Used: the blueprint's shorter text (section 4 first). The `blocked` reasons quote
  `BLUEPRINT.md#4.11`; the `merge`/`reviewer` rewordings of `state.md#2.8` and `pipeline.md#2.6`
  stay in those documents and in `topics/human-stops.md`.
- **T2-09, the kind vocabulary.** `hooks.md#2.7` uses the list kind `scenario` ("list scenario") and
  generates `[reveals]` from the registry, while `state.md#2.5` names the kind's file
  `holdout/HS-<NNN>.md` and the registry row is spelled `holdout-scenario`; `conventions` is the
  `review-rules`/`project-conventions` file of `state.md#2.5`. The table keeps the originals' two
  spellings as they stand (`holdout-scenario`, `project-conventions` in `tables/artifacts.toml`;
  `"list scenario"` in `tables/walls.toml`) and this record notes the mismatch for the registry
  owner: one vocabulary must be chosen when the kinds are named for the build, and
  `tables/walls.toml`'s `[reveals]` comment already ties the list kinds to the registry's classes.
- **T2-17, key names the originals never spell.** `conflicts/records.md` part 2 records the gaps.
  Applied: `schemas/evidence-record.schema.json` names no property (the git patch-id, the frozen
  test identities and `flaky` are described, not keyed); `schemas/caller-marker.schema.json` keeps
  only the four spelled keys and describes "the time" without naming it;
  `schemas/notes.schema.json` imposes no top-level wrapping key. The previously invented `patch_id`,
  `time` and `notes` wrapper are gone.
- **S2-03, the configuration defaults.** `hooks.rules.auto-allow.scope` now carries the blueprint's
  wording "set at setup" (P2-20). `delivery.mode` and `delivery.ci` stay "detected" per RR2, which
  records both texts; the example in `topics/project-configuration.md` is the other copy. The two
  lists no longer disagree: the table states key, type, default, owner and meaning, and
  `schemas/config.schema.json` states only structure and types, linking the table for everything
  else (T2-13, T2-14).

## Built files under `next/` (recorded, not changed)

The task says the built roster and schemas were made from the same originals; where the spec table
or schema and its built counterpart differ, the difference is recorded here and the built file
stands.

- `next/tests/lint/roster.toml` against `tables/roster.toml`: B6 brought the built file into line with
  `agents-skills.md#2.12` for the cells this repair recorded (`status = "both"`;
  `listed_description_chars = 4400`; `agent_file_lines = 70`, whole file, frontmatter included, body at
  most 60 lines), and the spec table is now a pointer to the shipped file, so no difference remains
  (T2-01/P2-01). Its `[platform]`, `[tiers]`, `[agents]`, `[overrides]`, `[budgets]` and `[workflows]`
  agree.
- `next/plugins/vsdd-factory/schemas/*.schema.json` against `rewrite/spec/schemas/`: the built set
  covers 15 schemas (the pipeline design's 2.10 series). Differences in the overlapping ones:
  - `claim.schema.json`: built makes `key`, `resolution` and `commit` required and describes
    `path`/`line_start`/`line_end` as carried by a non-holdout claim; the spec schema leaves the
    required list open, because no original states one for the general claim (the built file
    encodes the blueprint's "structured only" sentence as a decision).
  - `story-draft-data.schema.json`: built types `units` as an array of plain objects and marks it
    required; the spec marks it required with `$ref: story-unit.schema.json` items — same
    requirement, stronger items in the spec.
  - `intake-data.schema.json`: built requires `covers` and types it an array of strings; the spec
    types the array and leaves required open (no original requires it).
  - These three differences are design tightenings the builders chose; the spec files follow the
    originals' letter, per this task's rule. Nothing in `next/` was edited.

## Catch-up: the delivered schemas are the single source (slots SL1-03, SL1-05, SL1-06)

The catch-up unit built C1's set out and closed the three left-for-builder findings. From here:

- Twenty-one schemas are delivered built under `next/plugins/vsdd-factory/schemas/`; the
  corresponding files here carry a pointer at the built file and nothing else. The build generates
  every `S_*` literal in `workflow-lib/prelude.js` from them (`tools/build.py`, `--check` fails on
  drift), so the tightenings recorded above now live in the built files alone.
- `gate-output.schema.json` was renamed `pending-output.schema.json` to match the registry's id;
  and the anyOf union (a gate result, or the pending output) was dropped: `pending-output` is the
  pending branch alone, the ordinary result has its own schema, and no script read the union
  (SL1-46). A pending object is therefore not a valid gate result, which a fixture pins.
- `wave-gate-data.schema.json` is delivered as one flat object (the gate-result fields plus the
  wave's own, `holdout` included) instead of the `allOf` composition, because the shipped checker's
  keyword subset has no `allOf` and the platform promises only a root object with `properties`.
- `delivery-data.schema.json`'s `ok` now means "the command ran and reached a state" — false only
  for `refused`, `checkout_dirty` and a command failure (VSA-02; the per-state list is in
  `commands/story-deliver.md`).
- `story-unit.schema.json` requires every field, as the relay rule demands (VSA-14).

## One fact, one place

- The 31 `artifacts/<kind>.md` files now link their `[[table:artifacts]]` row instead of restating
  path, frontmatter, writer, cap and lifecycle (T2-08, S2-05); `rules/artifacts.md` shows no rows.
- The configuration key list is `tables/config-keys.toml` alone; `config/holdout.md`,
  `config/commands.md`, `config/state.md`, `config/evidence.md` and `config/maintain.md` link it
  (T2-13, S2-03), as `topics/convergence.md` and `topics/spikes.md` now link the threshold and
  spike rows (S2-09).
- The event log line's single source is the shipped file
  `next/plugins/vsdd-factory/schemas/event.schema.json`, which the Python writer loads and validates
  every event against (`py/vsdd/events.py`); the schema file here is a pointer at it, and the
  documented event-type list is `tables/event-types.toml` (S2-09, L2-catchup-1). The documented list
  is prose-side only: the shipped schema's `type` rule is the dotted-type rule the writer enforces.
- Conversion commentary left the table headers it sat in (`tables/walls.toml`, `tables/story-kinds.toml`,
  `tables/command-families.toml`) and the repair derivations are cited in this file and
  `conflicts/rules.md` instead (S2-12).
- Pointers into the originals were replaced with `[[kind:id]]` links or struck in
  `tables/limits.toml`, `tables/lint-checks.toml`, `tables/methods.toml` and `tables/artifacts.toml`
  (P2-12, S2-11, T2-26).
