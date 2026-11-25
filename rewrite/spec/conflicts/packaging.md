# Conflicts and gaps found while writing the packaging group

Written by the packaging extractor. One entry per place where two documents disagreed about a thing in the
packaging group, and per gap noticed while copying. Each entry gives both section ids, both texts, and which text
the new files use. The rule applied: section 4 of the blueprint, then the rest of the blueprint, then the detail
document that owns the subject.

## Disagreements

### C1. The modules of the Python package

- `BLUEPRINT.md#4.1`: "`py/vsdd/` with `project.py`, `config.py`, `frontmatter.py`, `events.py`, `schema.py` (a
  small JSON Schema checker), and the sub-packages `hooks/`, `rules/`, `policy/`, `state/`, `gates/`, `delivery/`,
  `cli/`."
- `hooks.md#2.4` (the layout tree): the shared core shown is `project.py`, `config.py`, `frontmatter.py`,
  `events.py`; the sub-packages shown are `hooks/`, `rules/`, `policy/`, and "`state/  cli/` (state and packaging
  designs)". It has no `schema.py`, no `gates/` and no `delivery/`.
- `packaging.md#after-cutover` agrees with the blueprint (`schema.py` in the shared core; `gates/` and `delivery/`
  present).
- Used: `BLUEPRINT.md#4.1`, in `packaging/python-package.md`. (Same disagreement the index recorded as C7; it is
  restated here because the file that carries the fact is this group's.)

### C2. What package T5 delivers

- `BLUEPRINT.md#9.6`: "T5 | Export and render | `vsdd export pdf`, `vsdd render diagram`, `<plugin>/assets/`;
  begins by proving the engines (section 10, item 24)".
- `packaging.md#2.12` (the old build-plan table, kept "for the detail of the packaging packages"): "T5 |
  `vsdd status`, `lock`, `validate`, `stale` | Thin commands over state modules; built with S1 and S2 in the
  blueprint. The blueprint's T5 is export and render."
- Used: `BLUEPRINT.md#9.6`. The registry agrees: `command.export-pdf`, `command.render-diagram` and
  `packaging.plugin-assets` all name package T5. The thin commands belong to the state design's packages.

### C3. Which package `vsdd repos scan` belongs to

- `BLUEPRINT.md#9.5`: "W12 | `multi`, with `vsdd repos scan` (the former T4, which serves only multi-repository
  projects)".
- `packaging.md#2.12`: "T6 | `vsdd repos scan` | T4 in the blueprint." (listed as a packaging-area package).
- Used: `BLUEPRINT.md#9.5`. The registry agrees: `command.repos-scan` has package W12.

### C4. Which tier carries the eleven-minute gate

- `BLUEPRINT.md#9.6` (package E2, story tier): "One story delivered; kills at a step boundary, mid-step and
  mid-CI-wait; an eleven-minute gate; budgets from measured counts (section 10, items 6 and 22)". Agrees with
  `BLUEPRINT.md#10` item 6: "A Bash call inside a workflow step accepts a ten-minute timeout, and a detached job
  outlives the call that started it | An eleven-minute gate ... The full ten minutes is tested in E2."
- `packaging.md#amendments-from-the-blueprint`, item 10 (first pass): "The conformance tier adds an eleven-minute
  gate job and the probes of the blueprint's section 10."
- Used: `BLUEPRINT.md#9.6` (story tier, E2), in `packaging/e2e-story-tier.md`. The second-pass blueprint is later
  than the first-pass amendment, and section 9.6 is the blueprint's own statement.

### C5. Dependencies of package E1

- `BLUEPRINT.md#9.6`: "E1 | Smoke tier | ... | H1 to H9, S3, W1, A4, A6b, G2".
- `packaging.md#2.12`: "E1 | End-to-end harness and smoke tier | Sample project with its JUnit runner, fixtures,
  probe runner, the ten smoke probes of 2.6, the `e2e-smoke` job. | H1, S1, one workflow, B8".
- Used: `BLUEPRINT.md#9.6` for the dependency set. Both agree the smoke probes are E1's deliverable; the
  first-pass row predates the second-pass split of the H packages.

## Gaps

### G1. `packaging.tests-python` names no package in the registry

Every other registry entry in the packaging group carries a `package` field except `packaging.tests-python`,
`packaging.build-plan`, `packaging.packaging-decisions` and `packaging.packaging-assumptions`. For the latter
three that is right (they are statements, not deliverables of one package). `tests-python` is a deliverable: its
fixture table names build packages S3, S4, S5, S1, G2, G3 and G1, and the directory it ships tests into spans
them. Its registry entry has no package, so its file carries no `package` field. Someone must decide what owns it
(or it stays deliberately shared).

### G2. "Eleven-minute gate" is used but never defined

The name appears in `BLUEPRINT.md#9.6` (E2: "an eleven-minute gate"), in `BLUEPRINT.md#5.5` ("End-to-end tests add
kills in the middle of a step and of a CI wait, an eleven-minute gate, ..."), and in
`packaging.md#amendments-from-the-blueprint` item 10. Its only definition is indirect, in `BLUEPRINT.md#10`
item 6: the probe for "A Bash call inside a workflow step accepts a ten-minute timeout, and a detached job outlives
the call that started it" is "An eleven-minute gate", with the fallback "Shorter wait slices; the job design is
unchanged". No document says what runs inside the gate job (which gate command, on what subject) or why eleven
rather than the ten-plus-margin one expects. `packaging/e2e-story-tier.md` carries the definition by reference to
section 10, item 6.

### G3. Mapping sections of the packaging design missing from the retire list

`conflicts/index.md` retires `packaging.md#packaging-and-manifests`, `packaging.md#bin-21-tools`,
`packaging.md#scripts-ci-benches`, `packaging.md#tests` and `packaging.md#ci-and-release` as mappings from the old
system. Three more sections of the same mapping are not listed: `packaging.md#3` (the parent heading, "Mapping
from the old system", with its mark legend ported/merged/replaced/dropped), `packaging.md#tools-observability-15-files`
("All ported; see the table in 2.10. The sink crates were never switched on and are dropped."), and
`packaging.md#documentation` ("See the last column of the table in 2.11."). None is copied; all three should be
retired with the same reason.

### G4. The first-pass amendment numbering of generated artefacts is stale

`packaging.md#amendments-from-the-blueprint` item 1 says the build "generates four things"; item 17 (second pass)
amends that; the body of `packaging.md#2.5` says six, which is what `BLUEPRINT.md#5.5`'s two amendment bullets sum
to (prelude, schema literals, tier/role tables, `hooks.json` from the first pass; shared agent block, severity
table, `OVERRIDE` from the second). No new file follows item 1. Listed so nobody "fixes" the six-item list back to
four; the amendments section itself is history and is retired.

### G5. The "Replaces" columns and the old-tool mapping are not carried

The operator-tool table of `packaging.md#2.10`, the documentation table of `packaging.md#2.11`, the CI section
`packaging.md#2.7` ("What each old job becomes") and the whole of `packaging.md#2.12` carry columns or tables
naming the old tools and jobs each thing replaces. Those columns are mapping from the old system, not design; the
new command files omit them. They remain retrievable from the original sections, which stay cited in the
`sources` frontmatter of the files that hold the 1.0 side of each row.
