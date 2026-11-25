# Disagreements and gaps met while repairing the records group

Written by the records-group repair pass. It covers the things under `group = "records"` in `registry.toml`
(the 7 topics, 15 records, 46 schemas, 22 config sections and 4 tables) plus the shared facts the audit
settled there. Nothing below is resolved; each entry gives both texts. Part 1 of
`conflicts/records.md` (R1 to R6) and its Parts 2 to 4 stand unchanged; this file adds what that file did
not record.

## Part 1: disagreements between the originals

### RR1. The default of `branches.protected`

- `state.md#2.12` (annotated example, line 1055): `protected = ["main"]`.
- No original states `[]`. The converted files gave `"[]"` with the meaning "Branches a story branch is
  never merged to directly", which no source states.
- Used: `["main"]`, in `tables/config-keys.toml`. The invented meaning is removed.

### RR2. The defaults of `delivery.mode` and `delivery.ci`

- `state.md#2.12` (annotated example, lines 1090-1091): `mode = "pr"`, `ci = "required"`, and the example
  "is the whole key list: every section and every key the loader accepts appears in it, with its default".
- `BLUEPRINT.md#4.12` (table): `delivery.mode`, `delivery.ci` — "detected": "`pr` or `local` (no remote);
  `required` or `none` (the repository has no CI)".
- Used: both as written. The keys table gives "detected" (the blueprint row), and the annotated example with
  the `pr`/`required` values is copied into `topics/project-configuration.md`. `pipeline.md#2.10.8` and
  `state.md#2.12` agree that 2.12 is the one key list, so the example stands; but the blueprint's "detected"
  is why the keys table cannot name a literal default for these two keys.

### RR3. The default of `hooks.rules.auto-allow.scope`

- `hooks.md#2.6` (annotated `[hooks]` example, line 506): `scope = "core"`.
- `state.md#2.12` (annotated example, line 1190): `scope = "off"` — "decision D4: off | core | extended.
  vsdd init writes the answer setup collected".
- `BLUEPRINT.md#4.12` (table): "set at setup".
- Used: the keys table states no literal default; it says "written once by vsdd init, from the answer setup
  collected" (the blueprint's meaning, which both examples agree with in substance: each example shows one
  project's written answer, not the default).

### RR4. The default of `hooks.rules.bash-destructive.allow_pr_close`

- `hooks.md#2.6` (annotated example, line 522): `allow_pr_close = true`.
- `BLUEPRINT.md#4.12` does not list the key; no original states a default for it.
- Used: the keys row carries no default value (blank) and the gap is recorded here. The `config/hooks.md`
  file before repair stated `false`, which no source states. Second audit T2-19 noted the asymmetry that
  `bash-git.attribution` carried the example's `"block"` while this row was blank; both rows now carry a
  blank default with the example value recorded in the meaning.

### RR5. The strength key's name

- `hooks.md#2.6`: `[hooks.rules.<rule>] strength = "block (default) | advise | off"` — a per-rule key, and
  the locked-rules list in the same section speaks of "strength" for every rule.
- The converted `config/hooks.md` used `hooks.rules.tests-first.strength` with default "block", i.e. it
  copied the example's value for the one rule shown.
- Used: `hooks.rules.<rule>.strength`, default `block`, in the keys table; `config/hooks.md` keeps the prose
  and links.

### RR6. `limits` rows: the two at-the-limit pointers

- `pipeline.md#2.5` (table, rows for review passes and rounds, and the spike rows): "At the limit" reads
  "Section 2.7" and "Section 2.8" — pointers into the pipeline design itself.
- `BLUEPRINT.md#5.1` states the convergence behaviour in prose; the spike behaviour is in `BLUEPRINT.md#6`
  and `pipeline.md#2.8`.
- Used: `tables/limits.toml` points at `[[topic:convergence]]` and `[[topic:spikes]]` instead of naming a
  section of a retired document. The rows themselves are copied from `pipeline.md#2.5`.

### RR7. The `ci_minutes` prelude name

- `BLUEPRINT.md#4.12` maps the prelude name `ci_minutes` to `limits.gate_minutes`, `delivery.ci_wait_minutes`
  ("60, 30") in one row.
- `pipeline.md#2.5` carries `delivery.ci_wait_minutes` inside the delivery-rounds row ("each CI wait is one
  job of at most `delivery.ci_wait_minutes` (30)") and has no separate CI row.
- Used, as amended by the second audit (T2-04): `tables/limits.toml` keeps the pipeline's delivery-rounds
  row as written and additionally carries one entry per prelude name, so `ci_minutes` is its own row with
  the mapping's key (`delivery.ci_wait_minutes`) and default (30) and no `enforced_by` or `at_the_limit`
  cell, which no source gives for it separately. An earlier version of this record said the separate row
  was not carried; the table now carries it.

### RR8. `state.push` default and condition

- `state.md#2.12` (annotated example, line 1158): `push = true`.
- `state.md#who-commits-state-and-when` (line 367): "`vsdd checkpoint` pushes the state branch when
  `state.push = true` (the default when the repository has a remote)".
- Used: both — default "true (the default when the repository has a remote)" in the keys table, the
  conditional prose in `config/state.md` and `topics/project-configuration.md`.

### RR9. Types the originals give as prose only

- `state.md#2.12`'s example gives values, not TOML types; `BLUEPRINT.md#4.12` gives no types. The `type`
  column values ("string (shell command)", "array of globs", "integer", ...) are the schema's and the keys
  table's transliteration of the values in the example; where the example shows a string, a list or an
  integer the type says so. This is stated here because the type column is not quoted text.

## Part 2: gaps (values the originals do not give)

- `hooks.rules.bash-destructive.allow_pr_close`: no default in any original (RR4).
- `notify.command`: the example gives `""`; no original states what a non-empty command receives or when it
  is run beyond "the tool notifies when a decision opens" (`pipeline.md#2.6` old-system table).
- `release.version_files`, `release.changelog`, `release.tag_format`, `release.publish`,
  `release.github_release`: the example shows values (`[]`, `CHANGELOG.md`, `v{version}`, `[]`, `true`) and
  the section comment says "detected by vsdd init". The keys table records the example's values; it cannot
  distinguish a default from one example's detected result, so both readings are recorded here.
- `project.name`, `project.languages`, `branches.integration`, `branches.release`: the example shows
  `"acme-api"`, `["rust"]`, `"develop"`, `"main"`; every original says only that `vsdd init` detects them.
  The keys table records "detected" for these four.
- `schemas/driver-flag.schema.json`: whether `vsdd driver off` deletes the file or writes an `off` state is
  not stated anywhere (already noted in `conflicts/records.md` R2; unresolved).
- `[[component]].commands` and the per-`[[repo]]` overrides: the example shows only the example keys
  (`test`, `lint`); the full key list of an override table is "the same keys as the top level" (2.12 line
  1184) for `[[repo]]`, but for `[[component]]` no original says more than "its commands override
  `[commands]`".
- `caps.<kind>`: which kinds are valid caps is never enumerated; `state.md#2.4` says "a project may raise a
  cap in `[caps]`" over the size caps, and the example shows `story`.
- The survey's `config` field "including `review_rules` and any `tier_override`" (pipeline.md#2.10.8): the
  two sub-keys are defined nowhere in the sections this group cites (already in `conflicts/records.md`
  part 2; still unresolved).
- `hooks.rules.<rule>.strength` for rules other than `tests-first`: only the example row exists; the locked
  rules list is the only other statement.

### RR10. Which run saves a flaky test, and what "first release" requires

- `state.md#2.8`: "An inherited test that passed at the baseline and fails now is run once more before it
  counts as new; one that passes in either run does not fail the gate" — a pass in either run saves it.
- `BLUEPRINT.md#4.13`: "An inherited test that fails and then passes when the gate runs the suite once more
  does not fail the gate" — only the fail-at-baseline-then-pass-at-the-gate case is described.
- Used: `BLUEPRINT.md#4.13` (section 4 of the blueprint wins, per this record's precedence rule) in
  `records/baseline.md`; both texts are
  quoted here. Related wording: `state.md#2.8` "A cycle whose baseline is empty is a first release" against
  `BLUEPRINT.md#4.13` "A cycle with no release, ingest or import baseline is a first release"; same in
  substance, and `records/baseline.md` keeps the blueprint's wording.

### RR11. Where the refusal of an ambiguous decision id is defined (audit F22)

- Five files stated the rule: `conventions.md`, `topics/decisions.md`, `commands/decide.md`,
  `skills/decide.md` and `topics/hotfix-cycle.md`. Only `topics/decisions.md` is in this group.
- Used: the records group keeps the definition in `topics/decisions.md`, as `state.md#2.8` states it ("a
  bare id that is ambiguous is refused with the choices"). The other four files belong to other groups.

### RR12. The event-type list (audit F30) is outside this group

- `schema.event` and `table.event-types` are in the group `rules`; the disagreement between them is that
  group's to settle. The records group takes no action.

### RR10. The default of `delivery.protected_paths`

Second audit T2-15. The two surviving texts:

- `BLUEPRINT.md#4.12` (line 721) and `pipeline.md#2.6` (line 479): "`.github/**`, `.env*`, `CODEOWNERS`,
  and the files `[release]` names".
- `state.md#2.12` (annotated example, line 1095): the literal `CHANGELOG.md` in the list, with the comment
  "vsdd init adds the files `[release]` names: version files, the change log, the release workflow".

Used: `tables/config-keys.toml` follows the blueprint (section 4 is the authority), matching the
`delivery.mode` and `delivery.ci` choice recorded in RR2. state.md's example stands as the annotated
example in [[topic:project-configuration]].
