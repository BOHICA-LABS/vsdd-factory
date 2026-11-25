# Conflicts and gaps found while writing the records group

Written by the records-group extractor. Covers the things under `group = "records"` in `registry.toml`:
the 7 topics, 15 records, 46 schemas, 22 config sections and 4 tables. Nothing here is resolved beyond the
rule already agreed (section 4 of the blueprint, then the rest of the blueprint, then the detail document
that owns the subject). Each entry gives both section ids, both texts, and what the new files use.

## Part 1: disagreements

### R1. The closure record's verdict: counts or lists?

- `state.md#2.8`, the closure-record example: `"verdict": { "perimeter": "story", "head": "5d6e7f8", "passes": 2,
  "rounds_used": 2, "outcome": "converged", "advisory": 1, "dismissed": 0, "waived": [] }` — `advisory` and
  `dismissed` are counts, `waived` is a list.
- `state.md#2.5` (Convergence record row): "The `verdict` in the unit's closure record: perimeter, commit,
  passes, outcome, and the counts of advisory, dismissed and waived findings" — counts for all three,
  including `waived`.
- `pipeline.md#2.10.3` (verdict.schema.json): `advisory`, `deferred` and `dismissed` are arrays of findings,
  and there is no `waived` field there.
- Used: the example as printed, in `schemas/closure-record.schema.json` and `records/closure.md`; the
  verdict's own schema (`schemas/verdict.schema.json`) is unchanged from the pipeline. Not resolved.

### R2. Driver flag: "absent" or "off"?

- `state.md#2.8`: "`local/driver.json`: `on`, `paused` at a decision id, or absent; and the session".
- `state.md#2.7` (status object): "`driver` | Whether the driver is `on`, `paused` (and at which decision)
  or `off`, and in which session" (example: `"driver": { "state": "on", "at": null }`).
- Used: both as written — the record schema (`schemas/driver-flag.schema.json`) keeps the record's
  description, the status schema (`schemas/status.schema.json`) the derived field. Whether `vsdd driver off`
  deletes the file or writes an `off` state is not stated anywhere.

### R3. Fourteen kinds or fifteen records?

- `BLUEPRINT.md#4.7`: "Machine records, written only by code. There are 14 kinds." (its table has 14 rows;
  one row is "Closure record, one per unit; a baseline is a closure record").
- `registry.toml` has 15 `record.*` entries: the baseline has a file of its own (`record.baseline`,
  `cycles/_standing/units/ingest.json` / `import.json`).
- Already noted in `conflicts/index.md` part 2. Used: "There are 14 kinds" in `topics/machine-records.md`,
  with the 15 files the registry asks for.

### R4. The baseline's contents: the blueprint's list is shorter

- `BLUEPRINT.md#4.13`: "a baseline record: the tests that failed then, lint, format and scanner findings,
  mutation scores per module, and the recovered contract set".
- `state.md#2.8`: "the integration head it was measured on; any earlier release tag; the suite result with
  the identities of the tests that failed; the lint and format findings; the mutation score per module; the
  scanner findings in existing dependencies; the recovered or imported contract ids; and whether the project
  has a suite, a JUnit report and CI".
- Used: `state.md#2.8` (the detail document that owns the subject), in `records/baseline.md` and
  `schemas/baseline.schema.json`. The blueprint omits the integration head, the release tag and the
  suite/JUnit/CI flags; nothing contradicts.

### R5. Holdout evaluation: whose step records feed it

- `state.md#2.5`: "`vsdd checkpoint`, from the evaluator's and the scenario-reviewer's step records".
- `state.md#2.4` (class `holdout-evaluations`): "`vsdd checkpoint`, from the evaluator's step records; and
  the full form of walled findings (a finding from the `scenario-reviewer`, or one located under
  `.factory/holdout/`)".
- Used: `state.md#2.5` (the fuller statement), in `records/holdout-evaluation.md`. The two agree in effect;
  the class-table phrasing folds the scenario-reviewer's contribution into "walled findings".

### R6. Pending output: the blueprint's shorthand

- `BLUEPRINT.md#4.9`: "the first call starts the job, and every call waits for it and returns either the
  result or `{state: "pending"}`".
- `pipeline.md#2.10.8` and `state.md#2.16`: "`ok: true`, `state: "pending"`, and `retry`, the same command to
  run again".
- Used: the fuller shape, in `schemas/pending-output.schema.json`.

## Part 2: gaps (fields given without types, or names never spelled out)

The general case: `pipeline.md#2.10.8` gives field lists without JSON types and without required lists for
the step-data and tool-output schemas, and `state.md#2.16` does the same for three tool outputs. The
`schemas/*.schema.json` files state every property name the design gives and only the enumerations and
bounds the design states (`fixed`/`disputed`; `resolved`/`not_resolved`/`withdrawn`;
`checked`/`not_checkable`/`skipped`; `defect`/`not_defect`/`spec_gap`; `full`/`delta`; `protected_path`/
`dependency`/`configured`; satisfaction 0 to 1; score 1 to 10), and required lists only where the design
states one (review-data "All four are required"; story-draft-data "Required"). Affected ids, for the fixing
pass:

- `schemas/delta-data.schema.json`, `schemas/route-data.schema.json`, `schemas/outline-data.schema.json`,
  `schemas/module-map-data.schema.json`, `schemas/score-data.schema.json`, `schemas/sweep-data.schema.json`,
  `schemas/survey.schema.json`, `schemas/story-unit.schema.json`, `schemas/wave-gate-data.schema.json`,
  `schemas/delivery-data.schema.json`, `schemas/checkpoint-result.schema.json` (pipeline.md#2.10.8);
  `schemas/release-result.schema.json`, `schemas/env-result.schema.json`, `schemas/repos-split-result.schema.json`
  (state.md#2.16: "ok, state, version, tag", "ok, url, port, pid", "ok, repos").

Specific, named keys the design never spells out (the new schemas carry the design's words in
`description`s; the key spellings below are the schema's, not the design's):

- Run record, `state.md#2.8`: "agent count, parent run, the stories admitted to it, the request or question
  it was given, whether it holds an amendment" — five fields with no key spellings
  (`schemas/run-record.schema.json`).
- Step record, `hooks.md#2.11.7`: "the head commit of the unit's worktree" — `agent_type` is spelled, the
  head key is not (`schemas/step-record.schema.json`).
- Closure record additions, `state.md#2.8`: "checks per round and the properties proven" in `verify.json`
  and the wave record's "stories and its gate rounds" have no key spellings
  (`schemas/closure-record.schema.json`).
- Evidence record, `state.md#2.8`: the record's own key list is never given; only the side files (".log, cut
  to 256 KB", "diff.<sha12>.patch"), the "git patch-id" and the red record's "frozen test identities"
  (`BLUEPRINT.md#4.13`) are stated (`schemas/evidence-record.schema.json`).
- Job record, `state.md#2.8`: "a job record with its port, process id and the caller's agent id" — no key
  names (`schemas/job-record.schema.json`).
- Driver flag, `state.md#2.8`: fields stated in prose only (`schemas/driver-flag.schema.json`).
- Caller marker, `hooks.md#2.11.10`: `session_id`, `agent_id`, `agent_type`, `cwd` are spelled; "and the
  time" is not (`schemas/caller-marker.schema.json` uses `time`).
- Spike admission, `hooks.md#2.11.6`: "the session and the dispatching caller's agent type (empty for the
  main session)" — no key names (`schemas/spike-admission.schema.json`).
- Configuration fingerprint, `hooks.md#2.6`: no field list at all, only the list of sections fingerprinted
  (`schemas/config-fingerprint.schema.json`).
- notes.json, `state.md#2.7`: no example of the file itself; the shape is inferred from the status object's
  notes entries (`{"at": ..., "text": ...}`, "5 entries of 200 characters; the oldest is dropped")
  (`schemas/notes.schema.json`).
- Decision record, `state.md#2.8`: fields are named; types and a required list are not
  (`schemas/decision-record.schema.json`).
- Cycle record, `state.md#2.8`: fields named; the close fields ("`closed`, `version`, `archive_commit`") have
  no types (`schemas/cycle-record.schema.json`).
- `factory.toml`: the design gives no JSON Schema; `schemas/config.schema.json` states the structure and
  quotes the key lists, but per-key types are derived from the annotated example in `state.md#2.12` and live
  authoritatively in `tables/config-keys.toml`. Note for the fixing pass: the registry carries both
  `schema.config` ("every section and key the loader accepts") and `table.config-keys` ("every section and
  key ... with its type, default and owner"); the two now divide the work as described, but the overlap is
  real and one of them could shrink to a pointer.
- The survey's `config` field "including `review_rules` and any `tier_override`" (pipeline.md#2.10.8) names
  two sub-keys that are defined nowhere in my sources; they appear to belong with the workflows group
  (`survey.schema.json` carries the names only).

## Part 3: history left behind (deliberately not copied)

Paragraph-level history inside sections the new files cite; candidates for `retired.toml` at
passage level, not whole sections:

- `BLUEPRINT.md#4.7`: "Changes from the state design, first pass: ...", "Changes in the second pass. ..."
  and "The first pass had 19 kinds. The five removed are ..." — amendment history.
- `BLUEPRINT.md#4.9`: "Removed in the first pass: ... Removed in the second pass: ..." — removed-command
  lists (owned by the commands group).
- `state.md#2.6`: the "Old system" column of the stored-and-derived table — a mapping from the old system.
  The digest definition at the foot of 2.6 is design and is copied (`topics/digests.md`).
- `state.md#2.5`: old-size remarks ("The old story files grew to 480 KB", "The old ADR-052 reached 685 KB",
  "The old repository carries 110 MB of committed demo media", "The old pass files ran to 198 KB").
- `state.md#2.8`: "The first pass wrote `review.<sha12>.json` beside the evidence ..." — kept in
  `topics/verdicts.md` as the design's stated reason there is no stored verdict record; flagging in case
  the fixing pass wants it struck as history.
- `pipeline.md#2.6`: "What happened to the old 37 approval steps is in section 3.5." — pointer.
- `pipeline.md#2.7`: the table "What is dropped from the old convergence document, and what replaces it" —
  a mapping from the old system; only the thresholds above it are copied (`config/holdout.md`,
  `config/mutation.md`).
- `pipeline.md#2.9.8`: "Measured file coverage replaces the old 'deepen until novelty is low'." — old-system
  comparison; the baseline behaviour itself is copied (`records/baseline.md`).
- `hooks.md#2.11.7`: "The two platform facts under this rule were proven by the probe of 2026-11-23 ..." and
  the fallback paragraph; `hooks.md#2.11.10`: "Two things are not yet proven (blueprint section 10, items 27
  and 28) ..." — probe history; the rule behaviour is copied (`records/step.md`, `records/caller-marker.md`).

No whole section in my sources is pure history; the index step's `conflicts/index.md` part 3 already lists
the amendment sections of the six documents.

## Part 4: not in the registry

- `.factory/local/index.json`. `state.md#layout` (the state layout tree): "├── media/, index.json" under
  `.factory/local/`. `BLUEPRINT.md#4.7`'s layout and local-store row list "runs/, evidence/, jobs/,
  lock.json, driver.json, hooks/, events/, media/" with no `index.json`, and no `record.*` entry covers it
  (also flagged in `conflicts/index.md` part 2). Purpose never stated in the sections I read.
- `local/lock.flock` and `flush.lock` (`state.md#2.11`, `state.md#layout`) are mechanisms, not records;
  they are described inside `records/lock.md` and need no file of their own.
