# Step 3 plan: record stores, lock and caller identity, decisions (S2a, S2b, S4)

Paths are relative to `next/`. `P` is `plugins/vsdd-factory`, `S` is `P/py/vsdd/state`. Spec files are under
`rewrite/spec/`.

## 1. Scope

This unit builds the storage layer of `S/`: the run, step, evidence, cycle, closure and decision stores with
their seven schemas; the digest, which "the specification is approved" needs; the run lock with re-entry,
liveness and release; reading the caller marker; flush; and of decisions the grammar, the kind table,
qualified ids, opening and settling, with `vsdd lock release` and `vsdd decide`. It decides no route,
position or verdict.

Left for when the build widens:

- The note store (it must read S1's `{"notes": []}` as no notes), `vsdd note`, the driver flag and
  `vsdd driver`, the change-request file; the state push, the fetch before a survey, a machine lease,
  `status --fix`; the S4 brief's "approve, survey, plan" fixture, which comes with the plan survey.
- `spec_locked`, `artifact_locked`, `guidance_locked` (first caller: H4); walled findings and holdout
  evaluation files in `store_step`; hub closure names, scopes and worktrees; a child run under its
  parent's lock; the hotfix scope; the `release` digest; the request files of `spec-gap amend` and
  `blocked open-spike`.
- The `review` verdict computed from stored passes (G3), the other gates' lookups, the red lookup after
  a rebase (G4a); the marker fallback by argument hash and lock liveness by process (items 27, 29).

## 2. Files

Owned:

| File | Holds |
|---|---|
| `S/{store,runs,steps,gate_evidence,cycles,digests,decisions,callers,locks,commits}.py` | One module per heading of section 3 |
| `S/cmd_decide.py`, `S/cmd_lock.py` | The two commands |
| `P/schemas/{run-record,step-record,evidence-record,closure-record,cycle-record,decision-record,lock}.schema.json`, `tests/unit/fixtures/schemas/<those seven>/` | The spec's `schemas/` files, changed as D3 says (C1's by the registry; delivered here), with fixtures in the existing form |
| `tests/unit/state_fixtures.py` | `make_project(tmp_path)`: S1's `tests/unit/_repo.make_project` (the real `init`), then one open cycle |
| `tests/unit/test_state_<module>.py` | Section 7, one file per module |

Shared (never `S/__init__.py` or `P/py/cli.py`: both are S1's):

| File | This unit changes | Avoiding collisions |
|---|---|---|
| `tools/check.py`, `tools/build.py` | The seven schema names in `required` of `schemas_are_the_single_source` and in `NON_PRELUDE_SCHEMAS`, in the same step as the seven files | After `hook-runner` is built: its `caller-marker.schema.json` name is there and stays |

No result schema changes: layer 2a already gave `story-unit.schema.json` the fields the story script reads.

## 3. Interfaces this unit provides

Common to every function:

- `project`: a `vsdd.project.Project`, any object with `.root`, or the project root as `str` or `Path`.
- Times: ISO 8601 UTC, whole seconds, ending `Z`; `now: str | None = None` wherever one is stamped.
- Errors: `StoreError(Exception)` with `.code`; subclasses `StoreRefused`, `StoreBusy` (a file lock not had in
  time), `StoreCorrupt` (a file that does not parse or fails its schema; names the path) and
  `DecisionRefused(StoreRefused)` with `.choices`. The codes are the quoted words below. A refusal raises;
  `None` and `[]` mean only "nothing there".
- A path segment matches `^[A-Za-z0-9][A-Za-z0-9._-]*$` without `..`, or is `_standing`; else `unsafe_id`.
  Paths and caps are those of `records/*.md`.
- `__all__` (state-core.md 3.2) holds state-API names only: `store_step` in `steps`, `evidence` in
  `gate_evidence`, `digest` in `digests`, `settle_decision` in `decisions`, `caller` in `callers`,
  `lock_seen` and `lock_release` in `locks`, `flush` in `commits`; `[]` in `store`, `runs`, `cycles`. Any
  other name is imported from its module. `locks`, `steps` and `callers` are on the hook path: they
  import git, configuration and registry code inside functions only.

### `store`

- `root_of(project) -> Path`; `now() -> str`; `read_record(path, schema=None) -> dict | None` (`None` when
  absent; `StoreCorrupt`).
- `write_record(path, obj, *, cap, schema=None)`: validates against `P/schemas/<schema>.schema.json` when
  named, writes under `.factory/local/tmp/`, then `os.replace`. `schema_invalid`, `over_cap`; the old file stays.
- `locked(project, name="flush.lock", wait=3.0)`: a context manager on `.factory/local/<name>`, re-entrant in
  one process; `StoreBusy`. `git(cwd, *args, timeout=10) -> str`: S1's `vsdd.state.gitrun.git`, with its
  `GitError` raised as `git_failed`.

### `runs` (`records/run.md`)

- `run_open(project, *, workflow, cycle, session, subject="", repository="", parent="", request="",
  amendment=False, stories=(), now=None) -> dict`: a new record, id `r-<YYYYMMDD>-<HHMM>-<2 of [0-9a-z]>`.
  Only a workflow that takes no lock calls it directly.
- `run_read(project, run_id) -> dict | None`; `run_live(project, run_id) -> bool` (it exists and `ended` is "").
- `run_require(project, run_id, *, story=None) -> dict`, or `run_unknown`, `run_ended`, `story_not_in_run`;
  for every command given `--run`.
- `run_admit(project, run_id, story) -> dict` adds the story once; `run_set(project, run_id, *, agents=None,
  amendment=None, request=None) -> dict`. Both refuse `run_unknown`, `run_ended`.
- `run_end(project, run_id, *, now=None) -> dict | None`: sets `ended` once; `None` for an unknown run.
- `runs_list(project, *, live=None, workflow=None, subject=None) -> list[dict]`, by `start`.

### `steps` (`records/step.md`)

- `store_step(project, result, agent_type, head, *, agent_id="") -> dict`: the signature of
  `topics/state-api.md` plus `agent_id`. Stores `result` with `agent_type`, `worktree_head = head` and `agent_id`
  at `local/runs/<run>/steps/<workflow>/<subject or _>/<name>.<attempt>.json`. Refuses `bad_identity`,
  `run_not_live`, `schema_invalid`, `over_cap`, `step_exists` (D4), `walled_not_built` (a finding under
  `.factory/holdout/`).
- `step_read(project, run_id, key) -> dict | None`; `key` is `<workflow>/<subject>/<name>.<attempt>`, as the
  prelude gives `spike open --step`. `run_has_steps(project, run_id) -> bool`.
- `steps_list(project, run_id=None, *, workflow=None, subject=None) -> list[dict]`: one run's records or every
  run's, by run start, then path; a corrupt file raises.

### `gate_evidence` (`records/evidence.md`, `topics/evidence-lookup.md`)

- `evidence_write(project, record, *, log=None, report=None) -> dict`: stamps `schema` and `at`, validates, and
  writes `<gate>.<sha12>.json`, replacing that gate and commit's record, with the last 256 KB of the text `log`
  as `.log` and a copy of the file `report` as `.xml`. A passing `red` record needs `frozen`, `tests`, `criteria`;
  a `refreeze` record must list every file the latest earlier `red` record froze (`refreeze_incomplete`).
- `evidence_read(project, subject, gate, sha) -> dict | None`: that commit's record, for the `step-results` rule.
- `evidence(project, gate, subject) -> dict | None`. `red`: the latest record by `at` with `passed` or `refreeze`
  true whose commit is the head of `.worktrees/<subject>/` or its ancestor and descends from the closure's
  `delivery.base`. `green`, `demo`: the passing record for the head, else one whose non-empty `patch_id` equals
  `patch_id(tree, base, head)`. `review`: the closure's `verdict` if its `outcome` is `converged` and it
  covers the head in that same way, by `head` or `patch_id`. `None` when nothing covers the head, or
  without a worktree or a base. Any other gate: `gate_lookup_not_built`.
- `patch_id(tree, base, head) -> str`: `git diff base head | git patch-id --stable`; "" if there is no diff.
- `register_gate(name, fn)`; `registered_gate(name) -> Callable | None`; `fn(project, subject, run) -> dict`, a
  gate result. The first lookup imports each `vsdd.gates.gate_*` module by file name (G2 names its gate
  modules `gate_<name>.py`; each registers at import); only a missing `vsdd.gates` is swallowed.
- `evidence_dir(project, subject) -> Path` (`_base` for base reports); `evidence_drop(project, subject)`.

### `cycles` (`records/cycle.md`, `records/closure.md`)

- `cycle_read(project, cycle) -> dict | None`; `cycle_write(project, record) -> dict`;
  `cycles_open(project) -> list[dict]` (no `closed`, not `_standing`, by `opened`).
- `closure_read(project, cycle, workflow, subject="") -> dict | None`; `closures_list(project, cycle)`.
- `closure_update(project, cycle, workflow, subject, mutate) -> dict`: the one write path. Under the file lock it
  loads the record or starts `{schema: 1, workflow, subject, cycle, state: "open", stages: {}}`, calls
  `mutate(record)`, validates and writes. Admission sets `delivery.base` through it.
- `closure_find(project, workflow, subject) -> dict | None` over open cycles and `_standing`; `ambiguous_unit`.

### `digests` (`topics/digests.md`)

- `digest(paths, *, root) -> str`: root-relative POSIX paths; the bytes are in D6; `digest_missing_file`.
- `spec_file_set(project) -> list[str]`: the sorted paths of the set that file defines;
  `spec_digest(project) -> tuple[str, list[str]]`: its digest and the paths, as a `spec` decision stores them.

### `decisions` (`topics/decisions.md`, `records/decision.md`, `rules/decisions.md`)

- `parse_decide(tokens: list[str]) -> tuple[str, str, list[tuple[str, str]], str]`: id, choice, pairs as typed,
  note (the tokens after a lone `--`, joined by one space). Keys match `[a-z][a-z0-9_-]*`. `bad_grammar`.
- `resolve_decision(project, id) -> tuple[str, str]` (cycle, bare id): `id@cycle` as written; a bare id is sought
  in open cycles and `_standing` (D9). `unknown`; `ambiguous`, with the qualified ids in `.choices`.
- `decision_read(project, id) -> dict`; `decisions_list(project, *, cycle=None, kind=None) ->
  list[tuple[str, dict]]`; `latest_decision(project, kind) -> tuple[str, dict] | None` (newest `opened`).
- `decision_status(project, id) -> str`: the stored status, or `stale` when that is `open` or `approved` and the
  kind's digest no longer matches.
- `spec_approved(project) -> tuple[bool, str]`: `(True, "")` when the latest `spec` decision reads
  `approved`, else `(False, detail)` with `no spec decision`, `stale` or the status. The one meaning of
  "the specification is approved"; survey's precondition calls it.
- `decision_open(project, *, cycle, kind, packet, subject="", digest="", files=(), reason="",
  dependencies=(), now=None) -> dict`: writes the `open` record and the packet, replacing that id's record.
  `bad_kind`, `bad_reason`, `over_cap`.
- `settle_decision(project, id, choice, values, note, session_id, *, settled_by=None, now=None) -> dict`:
  `values` is the pair list or a mapping. Writes the decision record and nothing else. `settled_by` defaults to
  `session` with a session id, else `terminal`. Refuses in this order: `unknown`, `ambiguous`, `not_open`,
  `stale`, `bad_choice` (listing the kind's choices), `bad_key`, `note_required`, `not_built`.
- `KINDS`: `tables/decision-kinds.toml` as data. Per choice: the status of the list the table puts it in; the
  keys its line shows and no other (bracketed ones optional, `key` and `item` repeatable, `ideas select` at
  least one list, `topology` one of `single`, `split`); a note required where the line shows `-- <text>`; for
  `open-spike` and `skip` the `reason` the decision must carry.

### `callers` (`topics/caller-identity.md`, `records/caller-marker.md`)

- `caller(key, *, project) -> dict | None`, both required: a command calls
  `caller(inv.caller_key, project=inv.root)`, and nothing reads `os.environ`. Reads
  `.factory/local/hooks/calls/<key>.json`, deletes it, caches the answer per `(root, key)` and prunes markers
  whose file time is over ten minutes old (the hook only writes them). Returns
  `{session_id, agent_id, agent_type, cwd}`, a missing one as ""; an empty `agent_id` is the main session.
  `None` for no key, a key outside `[A-Za-z0-9_-]{8,64}` or no file; `StoreCorrupt` without a `session_id`.
  The person-only check is S1's `Invocation.require_person`; there is no second one.

### `locks` (`records/lock.md`, `topics/run-lock.md`)

- `lock_scope(project, repository="") -> str`: `<repository | project.name | root name>:<branches.integration>`.
- `run_begin(project, *, scope, workflow, subject, session, cycle, now=None, **run_fields) -> dict`: the
  survey's one call, under `lock.flock`; `run_fields` are `run_open`'s other keywords. Returns
  `{ok: True, run: <record>, entered: "new" | "reentered" | "took_over", ended: <run id or "">}`, or
  `{ok: False, holder: <entry>, stale_at: <time>}` with nothing written. A free scope gives a new run. A stale
  holder's run is ended, then new. A holder with this session, workflow and subject gives the same run if it
  is live with no step record; else it is ended, then new. Any other holder refuses.
- `lock_seen(project, *, session_id, agent_id="", run="", story="", now=None) -> bool`: refreshes `seen` of the
  entry the call belongs to: `run` is its run, `story` is in its run's `stories`, or `session_id` is its session.
  An empty `agent_id` (the main session) gives false. True when the call belongs to a lock. At most one write
  a minute per entry. It raises nothing, gives false if the file lock is not had in 1 s, and with no lock
  file or nothing to refresh reads once and takes no file lock.
- `lock_release(project, *, run=None, session=None, stale=False, force=False, wait=3.0, now=None) ->
  list[dict]`: removes each entry matching any selector, ends its run, returns the entries. `StoreBusy` after
  `wait`, which bounds the whole call; `ValueError` with no selector.
- `lock_entries(project) -> list[dict]`; `lock_is_stale(project, entry, now=None) -> bool`.

### `commits` (`topics/state-commits.md`)

- `flush(root, reason, *, run="") -> dict`: `ok`, `state` (`committed`, `clean`, `partial`, `busy`, `failed`),
  `commit`, `files`, `rejected` (`[{path, problems}]`), `error`. `ok` is false for `partial`, `failed` and a
  non-empty `rejected`. It does that file's five steps and raises for none: step 2 is S1's
  `mount.mounted(root)`; step 3, for artifacts, is S1's
  `artifacts.check_paths(root, rels, config=cfg, references=False)`, each file with a `Problem` of severity
  `error` going into `rejected` with its messages.

### Commands (`commands/lock-release.md`, `commands/decide.md`)

Each is a `cmd_` module holding `COMMANDS = (Command(...),)` (state-core.md 3.1): `run(inv, args)` returns
a `Reply` (exit 0 or 1); exit 2 is `raise Refuse`.

- `S/cmd_lock.py`, words `("lock", "release")`, needs `initialised`: `vsdd lock release [--force]`. Fields
  `released`, `holder` (entry or null), `error`. Releases stale entries and those of the caller's session;
  exits 1 with `holder` if another session's live entry remains. `--force`: `inv.require_person(...)`,
  then every entry.
- `S/cmd_decide.py`, words `("decide",)`, `raw=True`, needs `mounted`:
  `vsdd decide <id> <choice> [key=value ...] [-- note]`, parsed from `args.tokens`. Fields `decision`
  (record or null), `error`, `message`, `choices`. `inv.require_person(...)` first. Exit 0 settled; 1
  refused by `settle_decision`; 2 for `bad_grammar` and for a caller that is not a person. It logs
  `decision.recorded` or `decision.refused` as session `terminal`; the stores log nothing.

## 4. Interfaces this unit needs

Built: `vsdd.schema`, `vsdd.config`, `vsdd.project`, `vsdd.events`, `vsdd.policy.load("classes")` (tables
`spec-product`, `spec-arch`, `spec-ux`, `brief`, `guidance`), and layer 2a's final `tools/check.py`,
`tools/build.py`, `P/schemas/` and lint.

From S1 (state-core.md, built first): `Command`, `Invocation`, `Reply`, `Refuse`; the `__all__` facade;
`vsdd.state.detect`; `mount.mounted`; `artifacts.check_paths`; `gitrun.git`; `vsdd.globs.glob_regex` (for
the class globs); `tests/unit/_repo.make_project`. A missing one is an import error at build time, never
a quiet `failed`.

From the hook runner: the marker, written before a `vsdd` Bash call runs, and the command prefixed
`VSDD_CALLER=<key> `. Its second part, built after this unit, imports `vsdd.state.locks` and calls
`lock_seen(project, session_id=, agent_id=, run=, story=)` on every guarded call and
`lock_release(project, session=<session_id>, wait=0.5)` on `SessionEnd` (hook-runner-b.md).

## 5. Design decisions

- D1. Two file locks: `flush.lock` for tracked records and every read-modify-write; `lock.flock` for `lock.json`,
  taken first when both are held. Step and evidence records are written once, by rename, so a step never
  waits on a flush.
- D2. Records under `local/` are written compact, tracked ones with `indent=2` and key order kept (the order of
  `stages` is data); caps count bytes written. Temporary files live in `local/tmp/`, where flush never looks.
  An empty subject is the directory `_`.
- D3. Schemas keep to the shipped checker's keywords: `step-record` is a `$ref` to `step-result` beside its own
  `type` and `properties`, with `agent_id` added; `x-notes` go into `description`; `closure-record.baseline`
  is a plain object until G7; `lock` requires every field.
- D4. A second write to a step key is taken only from the same `agent_id` or with identical content: a flat
  refusal could wedge an agent correcting its result.
- D5. A run is live when `ended` is empty. A time limit comes with the first workflow that takes no lock.
- D6. The digest is the SHA-256 of the lines `<path>\t<sha256 of the file's bytes>\n`, sorted by path, cut to 16
  hex characters. `root` is required: a tracked digest must not depend on where the clone sits.
- D7. `run_begin` joins the run record and the lock, and `lock_seen` decides belonging from the facts it
  is given: re-entry and the rule of `topics/run-lock.md` each exist once, here.
- D8. `caller` reads by key only (items 27, 28 are proven), so the lock's `session` is the marker's.
- D9. A repeated key is stored as its values joined by one space. A bare id means the one record of that id
  whose status is `open`, else the one record; more than one at either level is ambiguous, so a settled `spec`
  in `_standing` never forces `spec@<cycle>` on a person.
- D10. `KINDS` is a constant in `decisions.py`. What the slice leaves out refuses (`not_built` for `amend`
  and `open-spike`; `gate_lookup_not_built` for a gate other than `red`, `green`, `demo`, `review`).
- D11. `flush` waits 5 s for `flush.lock`. It checks records itself (schema and cap; packets by cap;
  `factory.toml` by the loader) and artifacts with `check_paths`. It stages deletions and commits with
  `core.hooksPath=/dev/null`, `commit.gpgsign=false` and, where git has no identity, `vsdd-factory`.
  `Run:` is the argument, else the one live lock's run, else `-`; `Code-Head:` is the integration head, else `-`.

## 6. Findings

- S2a. P3OPS-07, P3REQ-07: `run_begin(stories=…)` and `run_admit` record a story in the calling run;
  `run_require` is the one membership check. `test_runs`.
- P3SEAM-13: D4. `test_step_collision`.
- P3SEAM-15: `evidence_write` refuses an incomplete re-freeze. `test_refreeze_incomplete`.
- VSA-16: `closure_update` is the write path for every writer, the survey included. `test_closures`.
- Not at slice depth: P3OPS-12 (G7, G2); P3REQ-08 (S3, through `amendment=True`); VSA-01 (G4a); CL-R1 (G5);
  P3COV-11 (spec text only).
- S2b. P3SEAM-07: left-for-builder (H4, with `spec_locked`). Only the `run_live` half is built (D5,
  `test_runs`).
- P3BLD-14, P3OPS-20, P3SEAM-24: left-for-builder: `hook-runner-b` makes the calls and closes
  them. Built and tested here: `lock_release`, and `lock_seen` matching `run`, `story` and the session
  (`test_lock_seen`, `test_lock_release`).
- AU-S2-06: one module each for the lock and the marker reader. `test_reentry`, `test_caller`.
- Not at slice depth: P3OPS-13 (S3 for the fetch, S2b for push and lease); P3OPS-19 (S2b, H9); AU-P2-11 and
  AU-S2-15 (spec ledger; D7).
- S4. AU-P2-03, AU-S2-04: `KINDS` and `resolve_decision` are the one copy. `test_kinds_table`,
  `test_settle`.
- Not at slice depth: P3OPS-10 (S3: the route row and the new decision; here `decision_status` reads `stale`
  and settling refuses it, `test_spec_approved`); P3OPS-14 (S6; until then `open-spike` refuses).

## 7. Tests

- `test_store_refusals`: over cap, invalid and unsafe id raise, old bytes stay; a truncated file raises
  `StoreCorrupt` with its path. `test_locked`: a second process gets `StoreBusy`.
- `test_package_names`: each state-API name resolves through `vsdd.state` to its module.
- Seven schema fixture directories: valid accepted, invalid rejected; `tools/build.py --check` is clean.
- `test_runs`: a story recorded once; ended, unknown and foreign story refuse; the first end time stays.
- `test_store_step`: path and added fields; ended run, over cap, bad identity and holdout path refuse.
  `test_step_collision`: another agent refused; the same agent replaces. `test_steps_list`: run order;
  `step_read` by the prelude's key; a corrupt file raises.
- `test_evidence_write`: a rewrite replaces; the log is cut. `test_refreeze_incomplete`.
- `test_evidence_lookup`: red after later commits, not off-branch or failing; green by head, then patch
  id; review from a converged closure verdict at the head, `None` after a new commit; an unbuilt gate
  raises; a function registered by a `gate_x.py` module comes back.
- `test_closures`: creates, keeps stage order, sets `delivery.base`; over cap and two cycles refuse.
- `test_digest`: pinned value; order-free; content-bound; missing file refuses; brief and guidance left out.
- `test_grammar`: the seven lines of `rules/decisions.md`, a qualified id; a positional value refuses.
- `test_kinds_table`: nine kinds; each choice's status; an over-cap packet writes neither file.
- `test_resolve_prefers_open`: one open among two is found; two open are ambiguous.
- `test_settle`: one file differs and `store.git` is never called; every refusal code, in order.
- `test_spec_approved`: `no spec decision`; `stale` after a contract edit, not a brief or guidance edit.
- `test_caller`: read once, deleted, cached per root and key; old ones pruned; no key or file is `None`.
- `test_run_begin`: a second session gets the holder and no run; a stale run is ended; a race has one
  winner. `test_reentry`: the same run before the first step; after it the old run is ended.
- `test_lock_seen`: run, story, subagent refresh; main session and stranger do not; one write a minute.
  `test_lock_release`: each selector releases and ends runs.
- `test_flush`: one commit, both trailers, `local/` out, product index unchanged, then `clean`; an invalid
  story file stays out, named with its problems; a broken mount or held lock changes nothing.
- `test_cmd_lock_release`: own and stale go; another's live entry exits 1; `--force` with no terminal
  exits 2. `test_cmd_decide`: a terminal settles; `VSDD_CALLER` or no terminal exits 2; bad grammar exits
  2; a refused choice exits 1; `--json` after `--` is note text.

## 8. Order of work

Before the builder starts (`ORDER.md`): `state-core` and `hook-runner` have reported.

1. The seven schemas, their fixtures and the two set edits, as one step; `tools/check.py` passes.
2. `store.py`, `runs.py`, `steps.py`, `cycles.py`.
3. `digests.py`, `decisions.py`, `gate_evidence.py`.
4. `callers.py`, `locks.py`.
5. `commits.py`.
6. `cmd_lock.py`, `cmd_decide.py`.
7. Every check passes.

`survey`, `hook-runner-b` and `jobs-b` wait for this unit.
