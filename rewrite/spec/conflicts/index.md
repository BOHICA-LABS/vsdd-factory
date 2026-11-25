# Conflicts and retire candidates found while building the index

Written by the index step of the restructure (the registry, `overview.md`, `decisions.md`, `conventions.md`,
`platform.md`, `topics/glossary.md`, `topics/decisions-built.md`). Nothing here is resolved. Each entry gives both
section ids, both texts, and which text the index files use. The rule applied: section 4 of the blueprint, then
the rest of the blueprint, then the detail document that owns the subject.

## Part 1: disagreements that touch the index files

### C1. Which perimeters there are

- `BLUEPRINT.md#3.1`: "A **perimeter** is the scope of a review: story, wave or release."
  (`BLUEPRINT.md#1` agrees: "three widening perimeters (story, wave, release)".)
- `pipeline.md#2.1`: "Perimeter | The scope at which review happens: `spec`, `plan`, `story`, `wave`, `release`."
  (`pipeline.md#2.10.3`, the verdict schema, agrees with this one: `"enum": ["spec", "plan", "story", "wave", "release"]`.)
- Used: `BLUEPRINT.md#3.1`, in `topics/glossary.md`.

### C2. What a unit can be

- `BLUEPRINT.md#3.1`: "A **unit** is the thing a workflow works on: a story, a wave (a group of stories built in
  parallel), the specification set, a release candidate."
- `state.md#2.1`: "Unit | The thing one workflow run works on: the specification set, the plan, a wave, a story,
  the release candidate. The term is the pipeline design's."
- `pipeline.md#2.1`: "Unit | The thing a workflow works on: a story, a wave, a specification set, a release
  candidate."
- Used: `BLUEPRINT.md#3.1`, in `topics/glossary.md`. The state design's list adds "the plan".

### C3. How many tiers

- `BLUEPRINT.md#4.2`: four rows: planner, builder, reviewer, clerk.
- `agents-skills.md#2.1`: "Tier | One of the three model settings in section 2.3."
  (`agents-skills.md#2.3` itself lists four.)
- Used: `BLUEPRINT.md#4.2`, in `conventions.md`. The glossary only links the term to the conventions.

### C4. Two meanings of "gate"

- `BLUEPRINT.md#3.1`: "A **gate** is a fact the tool measures, such as "the new tests fail" or "the suite passes
  on this commit"."
- `hooks.md#2.1`: "Gate | A rule that can block. An advisor can only add text. A recorder only writes records."
- Used: `BLUEPRINT.md#3.1` as the definition in `topics/glossary.md`. The hook design's meaning is quoted there
  as a second sense, because the hook rules use the word that way throughout. The two meanings are not merged.

### C5. Whether `.factory/factory.toml` is a working name

- `BLUEPRINT.md#4.1`: "Project configuration | `.factory/factory.toml`. Created by `vsdd init`, edited by a
  person."
- `hooks.md#2.1`: "Factory project | A project whose root holds the project configuration file (working name
  `.factory/factory.toml`)."
- Used: `BLUEPRINT.md#4.1`. In `topics/glossary.md` the hook design's sentence is kept and the words "working
  name" are dropped.

### C6. Where the event log is

- `BLUEPRINT.md#4.1`: "Event log | `.factory/local/events/<UTC date>.jsonl`. Only the hook runner and the tool
  write it."
- `hooks.md#2.12`: "Location: daily files, one JSON object per line, in the directory the project configuration
  names (working name `.factory/local/events/<UTC date>.jsonl`; `VSDD_EVENTS_DIR` overrides it for tests)."
- Used: `BLUEPRINT.md#4.1`, in `conventions.md`. Whether a configuration key names the directory is left to
  `topics/event-log.md`.

### C7. The modules of the Python package

- `BLUEPRINT.md#4.1`: "`py/vsdd/` with `project.py`, `config.py`, `frontmatter.py`, `events.py`, `schema.py` (a
  small JSON Schema checker), and the sub-packages `hooks/`, `rules/`, `policy/`, `state/`, `gates/`,
  `delivery/`, `cli/`."
- `hooks.md#2.4` (the layout tree): the shared core is `project.py`, `config.py`, `frontmatter.py`, `events.py`;
  the sub-packages shown are `hooks/`, `rules/`, `policy/`, and "`state/  cli/` (state and packaging designs)".
  It has no `schema.py`, `gates/` or `delivery/`.
- Used: `BLUEPRINT.md#4.1`, in `conventions.md`. (`packaging.md#after-cutover` agrees with the blueprint.)

### C8. What `CR-NNN` identifies (both texts are in section 4)

- `BLUEPRINT.md#4.8`: "`CR-NNN` for a queued change request (new)".
- `BLUEPRINT.md#4.7`: "requests/CR-<NNN>.md             every change request, queued or running (written by code)".
  (`state.md#2.5` agrees with 4.7: status is one of queued, active, done, dropped.)
- Used: `BLUEPRINT.md#4.8` as written, in `conventions.md`, because that file is taken from 4.8. Not resolved.

### C9. What package B8 tests (both texts are in the blueprint)

- `BLUEPRINT.md#10`: "Package B8 tests each item that is still open before the packages that depend on it, and
  records the result in `rewrite/STATE.md`."
- `BLUEPRINT.md#9.1`: "B8 | Platform probes | The items of section 10 that a toy probe can settle (those still
  open among 3, 5, 6, 9, 10, 12, 13, 16, 18 and 27 to 35) ... Items that need the product (7 at wave scale, and
  19 to 25) belong to R0, E1 to E4 and T5."
- Used: `BLUEPRINT.md#10` in `platform.md`, because that file is taken from section 10. The generated brief
  `packages/B8.md` carries the text of 9.1.

### C10. What is re-run when a model changes (both texts are in section 2)

- `BLUEPRINT.md#2.1`, D5: "Re-run the end-to-end proof after any model release."
- `BLUEPRINT.md#2.4`, D5: "The weekly conformance job reruns the story tier when a model alias moves."
  (`BLUEPRINT.md#4.2` agrees with 2.4: "runs the story tier again when one changes".)
- Used: each as written, 2.1 in `decisions.md` and 2.4 in `topics/decisions-built.md`.

### C11. Item 26 in the two tables of section 10

- `BLUEPRINT.md#10`, first table: "9, 10, 12, 13, 16, 18 to 35 | Not yet probed."
- `BLUEPRINT.md#10`, second table, item 26: "How to test: It cannot be observed", "If it fails: Not relied on:
  every step sets effort".
- Used: both as written. In `platform.md` item 26 therefore reads "Not yet probed" beside "It cannot be
  observed".

## Part 2: noticed in passing, in files other groups write

These are not in the index files. They are listed so the owning group does not have to find them again.

- The contents of `.factory/local/`. `BLUEPRINT.md#4.7`: "runs/, evidence/, jobs/, lock.json, driver.json,
  hooks/, events/, media/". `state.md#layout` also lists `lock.flock`, `index.json` and `flush.lock`.
  `BLUEPRINT.md#4.1` ("Local store") lists "runs, step records, evidence, jobs, the lock, the driver flag, hook
  records, the event log" and no media. Owner: `topics/state-layout.md`.
- The registry's columns. `state.md#2.1`: the registry gives each kind's "path pattern, identifier pattern,
  frontmatter keys, required headings, size cap and frozen statuses". `state.md#2.4` says the status column
  (`code_values`) is gone and shows `lock = "spec"` and `template`. Owner: `tables/artifacts.toml`.
- `dependencies.toml` is in the layout (`state.md#layout`) and in `BLUEPRINT.md#5.2` and `state.md#2.8`, and has
  no row in the artifact tables of `state.md#2.5`. It is registered as `artifact:dependencies`.
- The holdout evaluation is a machine record in `BLUEPRINT.md#4.7` and a row of the table "Reports written by
  agents during a cycle" in `state.md#2.5`. It is registered once, as `record:holdout-evaluation`.
- `BLUEPRINT.md#4.7` says "There are 14 kinds" of machine record and also "a baseline is a closure record". The
  registry has 15 records because the baseline has a file of its own (`record:baseline`).

## Part 3: sections to record as retired

No index file copies these. Reason `history` unless stated.

Blueprint:

- `BLUEPRINT.md#8`: the table of first-pass review findings.
- `BLUEPRINT.md#11`, `BLUEPRINT.md#11.1`, `BLUEPRINT.md#11.4`: the second pass, its finding table, the close-out.
- `BLUEPRINT.md#11.2`: cuts offered and applied. Item 1 is still an open offer to the person (lock liveness by
  process, P2LEAN-17: "Until you say, the heartbeat is the design"); `platform.md` item 29 refers to it. It is
  not design until it is taken, and it has no file in the new layout.
- `BLUEPRINT.md#11.3`: three consequences of the second pass, each stated again where it is built
  (`topic:tool-call-approval`, `workflow:ingest`, `topic:spikes`). Reason `duplicate`.
- Inside cited sections, left out as history: the last paragraph of `BLUEPRINT.md#1` (the two review passes),
  and the last two sentences of `BLUEPRINT.md#2` (what the second pass found).
- `BLUEPRINT.md#4.4` (the provisional names the pipeline design used) is a mapping from an earlier version. It
  is not retired here: it is listed under `table:methods` in the registry so the agents group decides.
- `BLUEPRINT.md#5.1` to `#5.5` are lists of amendments by form, and they are also the blueprint's statement of
  the final design, so the registry cites them as sources. The owning groups decide what in them is history.

Detail documents:

- `pipeline.md#amendments-from-the-blueprint`, `state.md#amendments-from-the-blueprint`,
  `agents-skills.md#amendments-from-the-blueprint`, `hooks.md#amendments-from-the-blueprint`,
  `packaging.md#amendments-from-the-blueprint`: amendment lists.
- `pipeline.md#2.12`: what changed from the old flow.
- `pipeline.md#3.1` to `pipeline.md#3.7`, `state.md#3`, `agents-skills.md#3.1` to `agents-skills.md#3.4`,
  `hooks.md#3.1`, `hooks.md#3.2`, `packaging.md#packaging-and-manifests`, `packaging.md#bin-21-tools`,
  `packaging.md#scripts-ci-benches`, `packaging.md#tests`, `packaging.md#ci-and-release`: mappings from the old
  system.
- `state.md#2.17`, `hooks.md#2.17`: where a design departs from the readings in `rewrite/STATE.md`.
- `hooks.md#2.15`: old defects and what prevents each one.
- `packaging.md#2.12`: the build plan, which says of itself "Superseded by section 9 of the blueprint". Reason
  `duplicate` of `BLUEPRINT.md#9`.
- `pipeline.md#6`, `state.md#6`, `agents-skills.md#6`, `hooks.md#6`, `packaging.md#6`: open questions for the
  user, carried into `BLUEPRINT.md#2` and answered there. Reason `duplicate` of `BLUEPRINT.md#2`.

Not retired, and registered so that a group reads them: the "Interfaces I provide" and "Interfaces I need"
sections of the five detail documents (`topic:seams-pipeline`, `topic:seams-state`, `topic:seams-agents`,
`topic:seams-hooks`, `topic:seams-packaging`) and their risk sections (`topic:risks-pipeline` and the rest).
