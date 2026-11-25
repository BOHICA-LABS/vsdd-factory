# Seam review of the five step 3 plans

Reviewed: `state-core.md` (S1), `records.md` (S2a, S2b, S4), `survey.md` (S3 first part), `hook-runner.md`
(H1, then B4b), `jobs.md` (G1). Checked against `rewrite/spec/` (the files each plan cites where a seam was in
doubt), the code under `next/` as it stands, `rewrite/STATE.md`, the platform facts in
`rewrite/spec/topics/platform-proven.md`, and the ledger lists in `rewrite/findings/by-package/`.

## Verdict

The plans cannot be built as written. Each plan holds together on its own. The three blockers are at the
seams S1 owns: how a command is registered, how a `vsdd.state.<name>` is published, and the registry check
that flush calls. The other four plans each guessed these seams differently from S1, and from each other.
Every blocker is fixed by making the other plans use the S1 interface. None needs a design change.

The major findings fall into two groups:

- Four seams where two plans disagree: what an exit 2 prints for a relayed command; who checks that the
  caller is a person; `spec_approved`, defined twice; and the `review` evidence lookup.
- Three gaps that no plan fills: lock liveness from the hook (the hook-side half of P3BLD-14),
  `vsdd.project.path_class`, and the story-unit fields that the story script reads.

Fix these in the plans before the builders start. Most of the fixes are a paragraph in records.md,
survey.md or jobs.md that adopts S1's `Command`, `Invocation`, `Reply` and `__all__` rules.

Counts: 3 blockers, 7 major, 22 minor.

## Blockers

### PL3-1 (blocker): S1 registers commands one way; records, survey and jobs each assume another

S1 (state-core.md 3.1) registers a command by file. Each command has a module `cmd_<family>[_<x>].py` in
`vsdd/state`, `gates`, `delivery` or `cli`, holding `COMMANDS = (Command(words, summary, run, configure,
needs),)`, where `run(Invocation, argparse.Namespace) -> Reply`. `Reply.code` may be only 0 or 1. Exit 2 is
`raise Refuse`. No shared file is edited. The other plans assume something else:

- records.md 3 (Commands): `run(argv, root, env) -> tuple[int, dict]`, and "two entries in S1's command
  table (or `P/py/cli.py`)". Its module names, `cmd_decide.py` and `cmd_lock.py`, happen to fit S1's
  pattern, but the entry point does not. `decide` also returns exit 2 itself, for `bad_grammar`.
- survey.md 2 and 3.2: modules `survey.py` and `checkpoint.py`, which S1's discovery never lists (they lack
  the `cmd_` prefix), each with `add_arguments(parser)` and `command(root, args, env) -> (dict, int)`. Both
  plans also expect "two rows" in a subcommand table in `cli.py`.
- jobs.md 2: "two rows in one block marked `# G1`" in `cli.py`. The fallback, "that hook in
  `gates/env.py`", is still a module that discovery does not see (`cmd_env.py` is needed). Its handlers
  return `(dict, int)`.

Built from their plans, none of the three registers with S1's dispatcher, and three builders would edit
`cli.py`, which S1 replaces. `decide` has a further problem: its grammar (`<id> <choice> [key=value ...]
[-- note]`) needs the raw tokens. Behind argparse, an `nargs=REMAINDER` positional swallows a trailing
`--json`.

Resolution:

- Records adds `state/cmd_decide.py` and `state/cmd_lock.py`. Survey adds `state/cmd_survey.py` and
  `state/cmd_checkpoint.py`. Jobs adds `gates/cmd_env.py`. Each module holds `COMMANDS` and thin `run`
  adapters over that plan's own library functions. Remove `cli.py` from every shared-file list except S1's.
- Each plan states its `needs` level: `decide` uses `mounted`, and the rest are covered in PL3-25.
- Exit 2 is `raise Refuse` (see PL3-4 for relayed outputs).
- S1 adds a raw-argument mode (`configure=None` gives the command `inv.argv` after its words, with `--json`
  and `--wait` taken off the end by the dispatcher), so `decide` parses its own grammar.

### PL3-2 (blocker): three different mechanisms for `vsdd/state/__init__.py`

The plans publish flat `vsdd.state.<name>` names in three ways:

- S1 (state-core.md 3.2): S1 alone writes `__init__.py`, whose `__getattr__` reads the literal `__all__` of
  each sibling source. A unit publishes a name by listing it in its own module's `__all__`.
- records.md 2: adds `_API_RECORDS = {name: submodule}` between `# >>> records` and `# <<< records` in
  `S/__init__.py`. If the file is absent, records creates it, with "one module `__getattr__` serving every
  `_API_*` dict". Records' order of work (section 8) lets steps 1 to 4 run before S1 is built.
- survey.md 2: "appends three re-exports" to `ST/__init__.py`.

The working tree is shared during the build (`rewrite/STATE.md`, "Builds share one working tree"), so
records and S1 can each create the file. If S1's file wins, records' dict is ignored: unless records' modules
also carry `__all__`, `vsdd.state.flush`, `.caller` and `.evidence`, which survey and jobs call, raise
AttributeError. Survey's appended imports break S1's rule that no import is eager.

Resolution: one mechanism, S1's. Records and survey edit nothing in `__init__.py`. Every module they add
carries a literal top-level `__all__` naming its public flat names. Records drops "adds it if absent", and
records may start only after S1's step 2 has landed. Survey lists `unit_tree`, `story_kind` and
`kind_has_red_gate` in the `__all__` of `worktrees.py` and `story_unit.py`. Both plans state the S1 rule that
no module is named after an exported name. Records' D14 already follows that rule.

### PL3-3 (blocker): flush calls an S1 registry check that does not exist

records.md 4 assumes "the registry check behind `vsdd validate`, as `check_path(root, relpath) ->
list[str]`", and adds that "`flush` reports `failed` if it is missing". S1 (state-core.md 3.5) provides
`vsdd.state.artifacts.check_paths(root, rels=None, *, config, references=True) -> list[Problem]`, and says
"Flush calls it with `references=False` on its changed files". The name, the signature (a required `config`)
and the return type (Problem objects, not strings) all differ.

Built from the plans, every flush returns `failed`. Survey flushes before it takes the lock (survey.md 3.1)
but does not say what it does when that flush fails. If it refuses, every survey fails. If it continues,
state is never committed, and the only trace is a later checkpoint's `ok: false`.

Resolution:

- Records calls `vsdd.state.artifacts.check_paths(root, rels, config=cfg, references=False)` and maps each
  `Problem` with severity `error` into `rejected[{path, problems}]`.
- Records uses S1's `mount.mounted(root)` for flush step 2, instead of a second mount test.
- Records drops the "if it is missing" branch: a missing check is an ImportError at build time, not a quiet
  `failed`.
- Survey states its rule for a failed opening flush: `busy` or `clean` continues; `failed` or `partial` is
  `ok: false`, exit 1, with the flush error, before any lock is taken.

## Major

### PL3-4 (major): exit 2 from a relayed command prints a partial object

`tools/check.py::relayed_outputs_require_every_property` and `topics/result-schemas.md` require a command
that a script relays to print every field of its output schema on every exit. The relayed commands are
`survey`, `checkpoint-result`, `delivery-data`, `story-unit`, `wave-gate-data` and `pending-output`. S1 prints
`{"ok": false, "error": ...}` and nothing else for every `Refuse`, for an invalid configuration and for a
crash. Its decision 3 says this is "the error object the prelude's relay() builds", but `relay()` builds
`{...emptyOf(schema), ok: false, error}` (`next/workflow-lib/prelude.js`, `relay`).

Survey.md 3.2 wants the full checkpoint object, with `status: failed`, on exit 2, and the same holds for an
unbuilt workflow or a crash in `survey`. If the clerk gets a partial object, its StructuredOutput fails the
`S_SURVEY` or `S_CLOSE` schema, and the clerk is pushed to invent the missing fields, which the relay prompt
forbids.

Resolution: `Command` gains `output: str | None` (a shipped schema name). When it is set, the dispatcher
prints `{**empty_of(schema), "ok": false, "error": message}` for a `Refuse`, a `ConfigError` or a crash: the
same empty-by-type rule as `emptyOf()`, built from the shipped schema. Survey and checkpoint set it. The rest
of each command's output stays its own; the frame owns only the empty fill. Add a test in `test_dispatch.py`.

### PL3-5 (major): the person-only check is defined twice, with different strength and exit codes

- S1: `Invocation.require_person(what)` raises `Refuse` (exit 2) when `VSDD_CALLER` is set.
- records.md: `callers.require_person(env, tty)` refuses `person_only` when `VSDD_CALLER` is set or standard
  input is not a terminal. In records' tests a refusal exits 1 (`test_cmd_decide`, `test_cmd_lock_release`).

In the slice, the hook marks only a single-segment `vsdd` call (hook-runner.md HD8), and `gate-guard` (H6)
is not built. An agent's `true && vsdd decide spec-1 approve` therefore carries no `VSDD_CALLER`. S1's check
lets that call settle a decision; only records' terminal test stops it. Which check a builder uses decides
whether an agent can settle a decision during the slice, and whether the refusal exits 1 or 2.

Resolution: keep one check, records' semantics, behind S1's method. `Invocation.require_person` calls
`vsdd.state.callers.require_person(inv.environ, stdin_isatty)` and raises `Refuse`. A refused person-only
command is a usage or environment error, so it exits 2, and records' tests change to exit 2. Records' D11
(why the terminal test is needed while H6 is absent) moves into S1's decision 4.

### PL3-6 (major): `spec_approved` is defined twice, with different return types

`records.md` defines `decisions.spec_approved(project) -> bool`. `survey.md` 3.5 defines
`preconditions.spec_approved(root) -> tuple[bool, str]` and calls it "the one meaning".
`tables/preconditions.toml` requires one implementation. If both modules list the name in `__all__`, S1's
facade raises AttributeError for `vsdd.state.spec_approved` (two owners). If one does not, readers bind to
different functions.

Resolution:

- Records owns the predicate, as `spec_approved(project) -> tuple[bool, str]`. The detail names `stale`,
  `no spec decision` or the status.
- Survey's `preconditions.spec_approved` becomes a call to it and is not exported.
- Records' `test_spec_approved` is the fixture for the predicate.

### PL3-7 (major): `evidence(project, "review", subject)` raises where survey expects an answer

records.md 3 and D12: any gate other than red, green and demo raises `gate_lookup_not_built`, including
`review`. survey.md D6 and section 4 call `evidence()` for `review` and expect the closure's verdict or
`None`. `topics/evidence-lookup.md` says the `review` lookup returns the computed verdict "or the one in the
closure record".

Built from the plans, `story_unit.position` raises as soon as a story is green. That position is computed in
the survey and in every checkpoint, so the survey and the checkpoint of any story past green crash (exit 2),
and the lock stays held.

Resolution: records' `evidence(project, "review", subject)` returns the closure record's `verdict` when
`verdict.head` (or its `patch_id`) covers the current head, and `None` otherwise. Only recomputing the verdict
from stored passes is left for G3. The other unbuilt gates still raise. Add a case to `test_evidence_lookup`.

### PL3-8 (major): nobody calls `lock_seen` or `lock_release` from the hook, and P3BLD-14 stays open

records.md 4: "H1 calls `lock_seen` on each guarded call ... and `lock_release(project, session=<id>,
wait=1.0)` on `SessionEnd`". Records closes P3BLD-14, P3OPS-20 and P3SEAM-24 on that basis. hook-runner.md
leaves "lock liveness and session-end pruning (H9)" out, and says "This unit calls nothing in `vsdd.state`".

P3BLD-14's proposed resolution puts exactly this in H1. Without it, every slice run's lock goes stale after
`state.lock_stale_minutes` (20), and one story through red, green, review and delivery takes longer than
that. A second session's survey then takes the lock over and ends the run, and from then on every gate of
the first run refuses with `run_not_live`. A session that ends also leaves its lock standing for 20 minutes,
so an immediate re-run of E2 from a new session is refused. P3OPS-20 also needs `PreToolUse` on
`StructuredOutput`, which H1 does not wire.

Resolution: H1 takes the two duties as core duties (HD2), like the marker.

- On every guarded call, it calls `vsdd.state.lock_seen` with the payload ids, the `--run` of a `vsdd`
  segment, `step.run` of a `StructuredOutput` call, and `ctx.story`.
- `SessionEnd` is wired in `notify` mode and calls `lock_release(project, session=<id>, wait=1.0)`. Reason
  `clear` stays open for the probe records lists.
- `StructuredOutput` joins the `PreToolUse` matcher.
- Add these to hook-runner.md's interfaces and tests. Until then, records must not mark P3BLD-14,
  P3OPS-20 or P3SEAM-24 fixed.

### PL3-9 (major): `vsdd.project.path_class` is needed and no plan builds it

hook-runner.md 4 needs `vsdd.project.path_class(path) -> str | None` from S1 for `classes_of` (its step 6).
state-core.md 1 leaves it out: "first user: the hook runner or G2". Each plan assigns it to the other, so H1's
step 6 is blocked, as H1 itself predicts. Step 4's `tests-first` (H5) and `step-results` (H7) need
`ctx.classes` to tell a test from source. They are built straight after step 3, on the story path.

Resolution: S1 builds `path_class` in step 3, about 30 lines over `classes.toml`'s precedence and
`Config.get("paths")`, matched with S1's `glob_regex`. It is S1's by `topics/state-api.md`. S1 then owns the
single `project.py` change that adds it, and H1 makes its `find_project_root` default-home change in the same
file. Order the two hunks: S1's lands first, then H1's.

### PL3-10 (major): the story unit lacks the fields that the story script reads

`topics/story-script.md` reads `K.tests_agent`, `K.tests_method`, `K.impl_agent` and `K.impl_method` from
the unit (lines 73 to 89), and says the survey resolves them. The shipped `story-unit.schema.json` has
`method` ("the story's method key; empty when it has none") and `tests_method`. It has no agents, no
`impl_method` and no `red_gate`, and the relay rule makes `required` equal `properties`.

survey.md D14 resolves only `method` and `tests_method`. If W2 is built from the story script, a missing
`K.tests_agent` silently skips the whole tests stage (`if (before('red') && K.tests_agent)`), and
`K.impl_agent` being undefined throws from `step()`. Survey validates its output against the schema, so it
cannot add the fields by itself.

Resolution: settle the unit's field names before survey builds. Either amend `story-unit.schema.json`, add
`tests_agent`, `impl_agent`, `impl_method` and `red_gate` as required fields, drop `method`, update the
fixtures and regenerate the prelude literal; or rewrite the story script to read `method` and
`tests_method` and to derive the agents from the kind. The first matches C2's `impl_method` key and the
script. Survey then fills all of them from the `[[story-kind]]` row (D14). Whoever owns the schema edit is
named in survey.md's shared files.

## Minor

### PL3-11 (minor): records claims P3SEAM-07, but only half of it is built

records.md D5 defines live as "`ended` empty" and leaves the time limit "with the first workflow that takes
no lock". That matches `topics/run-lock.md`. P3SEAM-07's proposed resolution also covers `spec_locked` (an
amendment run that crashed keeps the specification unlocked until it is ended). Records lists P3SEAM-07
under S2b as resolved by D5, but `spec_locked` is not built in step 3, so that half is unverified.

Resolution: records marks P3SEAM-07 `left-for-builder` (H4, with `spec_locked`), not fixed. Only the
`run_live` half has a test.

### PL3-12 (minor): records and the hook runner edit `tools/check.py` and `tools/build.py` in the same places

Records adds seven names, and H1 one, to `required` in `schemas_are_the_single_source` and to
`NON_PRELUDE_SCHEMAS`. H1 also changes the gate payload in `launcher_modes_on_a_live_project` and adds a
fifth step to `build()`. The check needs the set to match exactly, and the tree is shared. While one unit's
schema files exist without their names, `tools/check.py` fails for every builder, and a builder may "fix" it
by removing the other unit's file.

Resolution: serialise the two edits. H1's step 7 lands before records' step 1 starts, or records waits on it.
Each plan tells its builder that the other unit's schema names may already be present and must be kept.

### PL3-13 (minor): `caller()` is called without a project, and its cache is not keyed

records.md: `caller(key=None, *, project=None)`. Without `project` it finds the root from the working
directory, "which `-C` makes wrong", and it "caches the answer for the process". survey.md 4 calls
`vsdd.state.caller(key)` with no project, and jobs.md expects "the frame or G1's rows" to call it.

Survey's and jobs' in-process tests run many commands, with different keys, in one pytest process. A cache
that is not keyed by `(root, key)` would hand later tests the first marker. `key=None` also reads
`os.environ`, which bypasses the `environ` that S1's `main()` injects.

Resolution:

- Every command calls `caller(inv.caller_key, project=inv.root)`.
- Records keys the cache by `(root, key)`.
- Records drops the `os.environ` default, or keeps it only for the hook-free case.

### PL3-14 (minor): the lock scope and re-entry are written twice

records.md: `lock_scope(project, repository="")` is `<repository | project.name | root name>:<branches.integration>`,
and `run_begin` opens the run and decides re-entry under one file lock (D7). survey.md D11 writes the scope as
`<project.name>:<target branch>`, and D7 says the survey decides re-entry and then writes the run record.
`project.name` defaults to `""` in `config.SPECS`, so survey's form gives `":main"`.

Resolution: survey calls `lock_scope(root)` and `run_begin(...)`. It drops D11 and narrows D7 to "the survey
uses `run_begin`'s answer". Re-entry is tested once, in records' `test_reentry`. Survey's re-entry tests drive
the command.

### PL3-15 (minor): the checkpoint's order of writes runs into `run_ended`

survey.md 3.2: "write `ended` and the agent count, release the lock (these last two in `finally`)". records.md:
`lock_release(..., run=)` "ends each released run", and `run_set(agents=)` refuses `run_ended`. Writing
`ended` before the count, or releasing first, makes the count write raise inside the `finally`.

Resolution: the checkpoint calls `run_set(agents=n)`, then `lock_release(run=...)`, which ends the run. It
calls `run_end` itself only when no lock entry names the run.

### PL3-16 (minor): after delivery, a delivered story's positions cannot be recomputed

`vsdd story deliver` removes the story worktree at the merge (`topics/worktrees.md`). records.md's
`evidence()` returns `None` without a worktree, so the checkpoint that follows delivery computes the ladder
as `ready`. Survey D9 takes the stage entries from positions, so a delivered story's closure gets no `tests`,
`implement`, `review` or `evidence` entry.

Resolution:

- `position()` returns `delivered` when `delivery.merged_sha` is set, before it reads any evidence.
- The checkpoint records stage entries from the positions held before the deliver stage (or leaves them to
  the second part's step-record mapping), and says so in D9.

### PL3-17 (minor): helpers defined more than once

- `retry_command`: S1's `Invocation.retry_command()` and G1's `jobs.retry_command(argv)` → G1 uses the
  `Invocation`'s.
- The git subprocess helper: S1's `gitrun.git` (raises `GitError`) and records' `store.git` (raises
  `StoreError` with code `git_failed`). A caller that catches one misses the other → records wraps
  `gitrun.git`.
- Run membership: records' `run_require(project, run_id, story=)` is "the one membership check", and survey's
  `require_story` re-checks it → `require_story` calls `run_require` and adds only the worktree and descent
  checks.
- Worktree paths: G1's D17 hard-codes `.worktrees/<id>/` and `_integration`, and survey's `story_path` and
  `integration_path` are "one creator, so paths cannot disagree" → G1 uses survey's functions (they run no
  git), or `vsdd.project` constants.
- Glob matching: S1's `glob_regex` is "one glob semantics for the tool". H1's `classes_of` and records'
  digests (which accept "a literal path or `<directory>/**`") each match globs their own way → both use
  `glob_regex`.
- Pruning stale markers: H1 does it on write and records on read. That is harmless, but one rule (file time
  older than 10 minutes) is stated once, in records.

### PL3-18 (minor): four test-project builders that can disagree

S1's `_repo.py`, records' `state_fixtures.make_project` (which mounts `.factory` by hand), survey's
`story_ready.py` (which runs S1's init) and H1's `runner_helpers.make_project` each build a project.
Records' hand-made mount must match S1's init exactly: the branch `factory-artifacts`, `.gitignore` holding
`local/`, and the exclusion lines. Otherwise `detect()` reports `partial`, and records' flush tests pass on a
layout the real tool never makes.

Resolution: once S1 is built, records' `make_project` calls S1's init on a scratch repository. Until then it
uses S1's `STATE_BRANCH` and `ensure_excludes`. H1's helper can stay minimal, because hooks need only
`factory.toml`.

### PL3-19 (minor): survey's fixture project has no commands after init

S1's decision 9 leaves `[commands]` empty and lists them in `undetected`. Survey's `story_ready.py` builds its
project "with S1's init and the stores", and the precondition `commands_configured` then fails, so
`test_survey_story_ready` cannot pass as written.

Resolution: `story_ready.py` writes `commands.test` and `commands.test_report` (the sample's values) into
`factory.toml` after init, and commits them with a flush. Say so in survey.md section 7.

### PL3-20 (minor): the gate registry triggers registration by importing a package that imports nothing

records.md: "The first lookup imports `vsdd.gates`; only that package being absent is swallowed". jobs.md:
`gates/__init__.py` is "a docstring. Later gate packages add modules beside it, not text in it". Importing
the package registers no gate, so `registered_gate("red")` is always `None` and the step 4 re-measure never
runs. Separately, survey's checkpoint imports `vsdd.gates.jobs` for `stop_products`. The build plan's S3 rule
says "S3 imports no gate package". There is no cycle, because `jobs.py` imports no state code, but the text
disagrees.

Resolution: records' lookup imports every `vsdd.gates.gate_*` module it finds. This is a naming rule for G2:
modules named `gate_<name>.py` register at import. Note in survey.md that importing `vsdd.gates.jobs` is
allowed because it has no back edge.

### PL3-21 (minor): HD7 approves every agent's `vsdd` call, whatever the auto-allow scope

Hook-runner HD7 sends `allow` with the rewrite whenever `agent_id` is set, and until H6 nothing checks the
subcommand against the agent's row. S1's decision 10 makes `hooks.rules.auto-allow.scope = "off"` the
default. User decision D4 makes approving the plugin's own calls something offered at setup and scoped.
PR3-3's proposal (pair a rewrite with `allow` on unattended paths) is the reason, and the alternative,
`ask`, is proven not to run headless. The widening is still a change to a person's decision.

Resolution: record it as an open question for the person in hook-runner.md, next to its existing ones. The
`allow` is limited to `vsdd` calls in the slice and is narrowed by H6's rows.

### PL3-22 (minor): the ledger marks the checkpoint `--subject` findings fixed one way; survey fixes them another

survey.md D1 accepts an optional `--subject` that must equal the run's subject, and lists P3COV-10 and
P3SEAM-10 as resolved by D1. The ledger has both as fixed by the catch-up (`outcomes/catchup-fix.json`),
which dropped `--subject` from `toy-story.js`. That outcome says the story script "already did" without it,
but `topics/story-script.md` line 55 still passes `--subject ${sid}`. The toy also passes `--outcome done`
and `--outcome blocked`, while survey's `[--outcome failed]` takes only `failed`.

Resolution: keep D1. It accepts both callers. Drop survey.md's claim to these findings. The orchestrator
corrects the catch-up outcome, and W2 settles what the story script sends. Survey states what
`--outcome <other>` does: refuse it, or accept it and ignore it.

### PL3-23 (minor): the state-API functions take a root where the hook will pass a project

`topics/state-api.md` writes `unit_tree(project, ...)` and `story_kind(project, story)`, and H7 (step 4) will
pass `ctx.project`, a `vsdd.project.Project` subclass. Survey's functions take `root: Path`, and so do G1's
`stop_products` and `product_ports`, which H9 and H6 will call. Records takes "a `Project`, any object with
`.root`, or a root".

Resolution: survey and jobs adopt records' convention, using `vsdd.state.store.root_of(project)` or a local
equivalent for G1, which imports no state code.

### PL3-24 (minor): the note store and the S4 fixture fall between plans

S1 writes `notes.json` as `{"notes": []}` and asks S2a to accept it. records.md leaves notes out, so nothing
confirms that shape, and `schemas/notes.schema.json` names no wrapping key. The S4 brief delivers "the
approve, survey, plan fixture". Records does not build it, and survey says S4's fixture "should assert on"
`spec_approved`.

Resolution: record both as left for the build to widen, named in records.md section 1: the note store
accepts `{"notes": []}`, and the fixture comes with the plan survey. S1's form then stands as the design.

### PL3-25 (minor): `needs` levels for survey, checkpoint and lock release

S1's `mounted` turns a broken mount into exit 2 before the command runs. If `checkpoint` declares it, a mount
that broke mid-run strands the lock. If `survey` declares it, `project_ready` never reaches the precondition
list that D2 promises.

Resolution: `survey`, `checkpoint`, `lock release` and `env start/stop` declare `initialised`, and `decide`
declares `mounted`. Each command plan states its level.

### PL3-26 (minor): the S1 facade costs time on the hook path

S1's `state/__init__.py` imports `mount.py` (and through it `gitrun`) eagerly. Its `__getattr__` reads the
`__all__` of every sibling source with the AST parser on first use. records.md: "Nothing is imported
eagerly: the hook runner loads the package on every call". From step 4, H7 calls `vsdd.state.store_step` on
every `StructuredOutput`, under H1's 120 ms median launch test.

Resolution: S1 keeps `detect` lazy too, and caches the name table in the process, computed from a filename
scan with a fast text regex for `__all__ = [...]`. Hook code imports the submodule directly
(`vsdd.state.steps.store_step`). Add a latency assertion in `test_state_facade.py`.

### PL3-27 (minor): a crash before dispatch exits 1, which the launcher passes on silently

S1's "a crash never exits 1" covers exceptions in `run`. An import error in `vsdd/cli/dispatch.py`, or in
`vsdd/config.py` (which S1 is editing in the shared tree), raises in `cli.py`. Python exits 1, and
`launch.sh`'s `1) exit 1` passes that on with nothing added, as a negative answer.

Resolution: `cli.py` imports the dispatcher inside `main()` with a `BaseException` guard that prints the
error and exits 2. The `sys.path` line stays verbatim, and `test_sibling_import.py` only appends its own
import after the anchor.

### PL3-28 (minor): `env stop` without a marker stops every product

jobs.md D16: with no marker, `env stop` selects every product. hook-runner.md HD8 gives no marker to a
compound command, so an agent's `cd .worktrees/S-1.01 && vsdd env stop` stops other agents' products, and
nothing reports it. Separately, `stop_products` defaults to a 5 s grace, while H1 gives `SubagentStop`, where
H9 will call it, a 4 s deadline (HD4).

Resolution:

- With no marker, `env stop` stops all products only when standard input is a terminal (a person). From a
  tool call it refuses with exit 1, naming the single-segment form.
- H9 passes `grace` below its deadline. Note this in jobs.md 3.4.

### PL3-29 (minor): survey builds the parts the build plan assigns to S3's second part

`packaging/build-plan.md` puts "story positions ... and the checkpoint's mapping of results to closures" in
S3's second part, after G2 and G3. survey.md builds positions over `evidence()`, and a checkpoint that maps
positions to stage entries (D9). That is workable for the slice, but the code is rebuilt in step 4, and
PL3-7 and PL3-16 come from it.

Resolution: keep it, and say in survey.md section 1 that `position()` and the checkpoint's stage mapping are
provisional, to be replaced by the second part's step-record mapping. The step 4 planner then does not treat
them as settled.

### PL3-30 (minor): built beyond what the story path touches

S1 builds the `new` command with two skeletons and their L-TEMPLATE obligations, plus `list`, `path` as a
command, `config --json` and `init --auto-allow`. G1 builds `env start` and `env stop`. The sample sets
`commands.run = ""` and `ui = false`, so the slice's demo never starts a product. All of this is named in the
build plan's step 3 packages and none of it is harmful, but it lengthens the critical path (S1 then records
then survey).

Resolution: build S1 in the order of state-core.md section 8, with steps 1, 2 and 4 as the seam. Records may
start as soon as `check_paths` and the dispatcher exist. `new`, `list` and `path` (the command; `path_for`
is needed) follow off the critical path.

### PL3-31 (minor): the facade's two-owner error can be set off by an ordinary name

The facade raises for a name two modules export. Plans that write modules with `__all__` and do not know the
rule risk collisions. Likely examples: `check` (survey's `preconditions.check`), `build`
(`story_unit.build`), `head` (`worktrees.head`), and `now` and `git` (records' `store`). Nobody imports these
names flat, but S1's `test_no_owner_or_two_is_loud` and `test_shipped_commands_register_cleanly` run over the
real package in the shared tree.

Resolution: each plan's `__all__` lists only the state-API names (`topics/state-api.md`) and the names
another unit calls. Helpers like `check`, `build`, `head`, `now` and `git` stay out of `__all__` and are
imported from their submodule.

### PL3-32 (minor): the record schemas are attributed to C1, but records ships them

survey.md 4: "C1: the record schemas (run, closure, cycle, decision, lock, evidence)". records.md ships all
seven itself ("The registry gives them to C1, which has not shipped them"). Survey's dependency therefore
points at a package that is not in step 3.

Resolution: survey.md names records (S2a/S2b) as the source of the seven schemas. The orchestrator records
that records delivered C1's seven record schemas.

## Which units can be built at the same time

The tree is shared and the builds are not committed (`rewrite/STATE.md`), so the order below assumes the
plan fixes above.

Groups:

1. **Group 1, in parallel:**
   - S1, all of it, in its section 8 order. Steps 1, 2 and 4 are the seam: the dispatcher, the facade,
     `gitrun`, the mount, and `check_paths`. Add `path_class` (PL3-9).
   - H1, steps 1 to 5 and 7. Its tests of rule-table faults wait for S1's step 3 (`config.py`), and its
     `project.py` hunk goes after S1's `path_class` hunk.
   - G1, steps 1 to 5. It owns `vsdd/gates/` only.
2. **Group 2, after S1:**
   - Records (S2a, S2b, S4), in parallel with whatever is left of H1 and G1. Its step 1 (the schema names in
     `check.py` and `build.py`) starts only after H1's step 7 has landed (PL3-12).
   - H1's step 6 (`classes_of`) once `path_class` exists.
   - G1's step 6 (`gates/cmd_env.py`) once S1's dispatcher and records' `callers.py` exist.
3. **Group 3, after S1 and records:** survey (S3 first part). It needs the stores, `run_begin`, `flush`,
   `caller`, `evidence` with the review fix (PL3-7), and `spec_approved` (PL3-6). It needs G1's
   `stop_products` only as one call, which its tests replace.

Must be built in order:

- Layer 2a (catch-up, C2, B6) has reported before S1 starts, because S1 edits `config.py`, which C2 changed.
- S1 (at least the dispatcher, the facade, the mount and `check_paths`) comes before records. Records'
  commands register through S1's `Command`, its names publish through S1's `__all__` rule, and its flush
  calls `check_paths`.
- Records comes before survey: the survey is built on records' stores, lock, caller and flush.
- S1's dispatcher and records' `caller` come before G1's command module.
- H1 comes before B4b, inside the hook-runner unit: `hooks.json` is generated from H1's registry.
- H1's `check.py`/`build.py` edits and records' schema-set edits are made one after the other (PL3-12).
- S1's `path_class` lands before H1's `classes_of` (PL3-9), and before step 4's H5 and H7.

H1's 120 ms latency test is sensitive to load. Run it after the group's parallel builds have stopped, or let
it record its result without failing the build while other builders share the machine.
