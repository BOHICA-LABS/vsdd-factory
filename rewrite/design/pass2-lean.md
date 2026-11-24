# Second pass: what can be cut

Reviewer: lean (prefix P2LEAN). Written 2026-11-23 on branch `rewrite/v1`. Scope: `rewrite/BLUEPRINT.md` and the five
design documents as amended, read against `rewrite/STATE.md`. Decisions D1 to D30 are treated as fixed. A cut is
proposed only when it removes no capability the user asked to keep, weakens no enforcement the design depends on, and
contradicts no user decision.

## Verdict

The design can be built as written, and its skeleton is already lean: three layers with one job each, one state
directory written by code, three routine human stops, one review loop, 17 hook rules in place of 79. It is not yet lean
enough in its detail. About a fifth of its mechanism either stores a fact that is already stored somewhere else, or
spends a whole agent start on one command that the step before could run, or that the tool could run together with the
next command. The stored copies are: file statuses rewritten after a decision, a status record rebuilt from the
closures, a verdict record kept beside the closure's own verdict, an input digest for every stage kept beside the
amendment mechanism, a second copy of each change request, a compaction snapshot, and a lessons register that nothing
reads. This is the OPS-28 pattern moved into code. No agent keeps books any more, but code keeps several books about
the same facts, and every extra write point is a place where two of them can disagree. Seven cuts (P2LEAN-01 to 07) are
strongly recommended. Together they remove about six record types, about twelve `vsdd` subcommands, the stage registry
file and one hook check. They also save about 9 agent starts per greenfield cycle, 2 per fast-route feature and 4 per
maintenance run. None of them removes a capability, weakens an enforcement or touches a decision in section 2. They
make packages S2, S3, S4, C3, G1, G5, G6 and H4 smaller, so they should be applied before those packages start. The
eleven minor cuts are worth taking where convenient. P2LEAN-17 needs a probe first. P2LEAN-18 is not recommended; it is
recorded to show why the survey and checkpoint clerks stay. No cut removes a stage from the brownfield routes (`ingest`
then `feature`, and `feature` on its own after a greenfield build).

Findings are ranked by value.

---

## P2LEAN-01: Stop promoting file statuses; read approval, readiness and completion from the records that decide them

- **Severity:** major (recommended)
- **Locations:** `BLUEPRINT.md` 4.11, last paragraph (the digest leaves out the status line; the decision is written
  before the promotion); `state.md` 2.4 ("Who may write": the `code_values` and `frozen_when` rows; the `contract`
  registry example), 2.5 (the lifecycle column for contract, decision record, verification property and story), 2.8
  ("How a decision is protected": `settle_decision` promotes, and the next survey finishes an interrupted promotion),
  2.15 (the importer's status mapping); `hooks.md` 2.11.3 (the `code_only_value` check), amendment 7; `pipeline.md` 2.6
  ("An approval is bound to a digest… leaves out each file's `status` line").
- **What it is now:** when a person approves `spec`, `settle_decision` writes `status: approved` into every draft
  specification file. Closing the plan writes `ready` into every story. `story deliver` writes `done`. `gate formal`
  writes `proven`. The `spec` decision also writes `accepted` and `active`. The digest must skip the status line so that
  these writes do not make the approval stale. The survey must finish a promotion that was interrupted. A registry
  column and a hook check exist so that an agent cannot write those values itself.
- **What is lost by the cut:** a person who opens a contract in an editor no longer sees `approved` in its frontmatter,
  and runs `vsdd list contract` (already a derived view) instead. The `artifacts` rule reads one decision record, or
  one closure for a story, on each guarded write to decide whether the file is frozen. That is one more small file read.
- **If left alone:** every approval rewrites dozens to hundreds of tracked files after the fact. That is the "status
  promoted after merge" bookkeeping that the state design's rule 5 (2.2, "nothing stored is updated after the fact")
  forbids, now done by code. It caused the OPS-02 blocker (an approval made stale by its own promotion), and it still
  needs a special digest, a settle in two phases and a repair path. After a crash, the file status and the decision
  record can disagree until the next survey.
- **Proposed resolution:** read each of these facts from the record that decides it:
  - A specification file is locked while the latest `spec` decision is approved and no live run holds an amendment.
    `spec_locked` already has this shape.
  - The `spec` decision record lists the decision-record ids it accepted, so an accepted ADR stays frozen through later
    amendments.
  - A story is ready when the plan closure places it in a wave, and done when its delivery closure exists.
  - A property is proven when the `verify` closure lists it.

  Agents keep setting only the statuses they own (`draft`, `retired`, `dropped`, `stale`). Drop the `code_values`
  column, the `code_only_value` check, the status-line exclusion in `digest()`, the promotion and its repair, and the
  code writes of `ready`, `done`, `proven`, `accepted` and `active`. The importer writes one imported, approved `spec`
  decision instead of mapping old statuses. This saves one registry column, one hook check, six code write paths into
  tracked files, one digest special case, one recovery path, and the state commit that each promotion makes. D7 and D19
  are unaffected, because the decision record was already the authority.

## P2LEAN-02: Merge back-to-back clerk steps into one tool command

- **Severity:** major (recommended)
- **Locations:** `pipeline.md` 2.9.6 (verify checks: `baseline`, `formal`, `perf` and `ui` are each a clerk step),
  2.9.2 (wave gate row: `integration-r<k>` and `ui-verify-r<k>`), 2.9.10 (maintain sweeps: `gate audit`, `gate perf`,
  `gate spec --full`, `report debt`, `discover collect`), 2.9.5 (plan `schedule`, then `readiness` with
  `gate baseline`), 2.9.7 (release `prepare`, then `publish`); `state.md` 2.16 (the measuring commands table);
  `BLUEPRINT.md` 4.9 (gate names), D5 ("about a third of all agent starts"), 10 item 22.
- **What it is now:** each measured check is its own clerk start, even when several run on the same commit with no
  judgment between them.
- **What is lost by the cut:** the workflow's progress display no longer shows each check as its own step. A combined
  job ends when its slowest check ends, although the tool still runs the checks in parallel under `parallel_gates`. A
  failing check is reported alongside the others in one result.
- **If left alone:** clerk starts per occasion:

  | Occasion | Clerk starts |
  |---|---|
  | A `verify` round | up to 4 (baseline, formal, perf, ui) |
  | A UI wave round | 2 |
  | A maintenance run | 5 |
  | A plan | 2 |
  | A release | 2 |

  Each start pays the fixed per-agent context the spike measured (about $0.15) and 10 to 30 seconds of start-up, for
  steps that make no judgment.
- **Proposed resolution:**
  - `vsdd gate wave` runs the UI check itself when the product has a UI, so the `ui-verify` clerk goes.
  - In `verify`, one clerk runs the baseline. A second clerk runs every due measured check (`formal`, `perf`, `ui`) as
    one job, in parallel with the reviewers.
  - In `maintain`, one `vsdd gate sweep` job runs the audit, the benchmarks, the spec coherence check, the debt report
    and the collection. The two judgment sweeps that read its output (security triage and post-release signals) run
    after it.
  - `vsdd plan schedule` records the baseline in the same job.
  - One idempotent `vsdd release` command prepares and publishes.

  Gate names fall from 15 to 11: `ui`, `formal`, `perf`, `audit` and `baseline` become checks inside `wave`, `release`,
  `sweep` and `plan schedule`, selectable with `--only`. This saves up to 2 starts per verify round, 1 per UI wave
  round, 4 per maintenance run, 1 per plan and 1 per release. For a greenfield cycle of three UI waves and two verify
  rounds that is about 9 starts. The clerk tier of D5 is untouched; it serves the clerk steps that remain.

## P2LEAN-03: Let planner steps finish with their own gate, as builder steps already do

- **Severity:** major (recommended)
- **Locations:** `agents-skills.md` 2.5 (tool table: `product-owner`, `architect`, `ux-designer` and `story-writer`
  have no SHELL); `BLUEPRINT.md` 4.3 (the "Shell" column); `hooks.md` 2.7 (`tool_only` in the clerk row, `[callers]`);
  `pipeline.md` 2.9.4 (a `route` clerk after `delta`; `story-draft` followed by a `gate plan --story` clerk, with at
  most 2 fix rounds), 2.9.3 (`checks` stage: a clerk, then fixes by the owning planner, at most 2 rounds), 2.9.5
  (`schedule` fixes by the story-writer), 2.8 and 2.9.12 (a named spike runs a clerk for `spike open --name` before
  the spike-runner step).
- **What it is now:** builders end with `vsdd gate red`, `green` or `demo`, and `gated()` retries on a failure.
  Planners cannot do this because they have no shell. Each of their gates is therefore a separate clerk start, and each
  failed gate costs a fresh planner start to fix it.
- **What is lost by the cut:** planners lose the simplicity of having no shell at all. A planner gets `Bash` limited by
  `tool_only = true` to single `vsdd` calls in the groups its row grants. Planners that read outside documents
  (`intake`, ingest analysis) become slightly more exposed to prompt injection. The exposure is limited to read and
  measure commands, whose inputs come from `factory.toml`, which no tool call can write. `researcher` keeps no shell,
  because it reads web content.
- **If left alone:** a fast-route feature (the common case once greenfield is done) spends 4 of its 6 non-story starts
  on clerks: survey, `feature route`, `gate plan --story`, close. Every failed plan or spec check costs a new start of
  the most expensive tier instead of a second turn of the agent that already holds the context.
- **Proposed resolution:** give `product-owner`, `architect`, `ux-designer` and `story-writer` `Bash` with
  `tool_only = true` and `vsdd = ["read", "measure"]`. The architect is also granted `feature route`. Each step then
  closes its own gate, all through `gated()`:
  - The architect's delta step ends with `vsdd feature route --run <id>`. The tool derives the route from the delta
    record file the architect has just written.
  - The story-writer's draft ends with `vsdd gate plan --story <id>`.
  - Planner fix steps in `spec` end with `vsdd gate spec --only <their part>`.
  - The spike-runner opens a named spike itself, as it already opens a quick one.

  One clerk stays for the cross-set `gate spec` and one for `plan schedule`, because both follow parallel steps. This
  saves 2 starts per fast-route feature and 1 per named-spike `open`. It also removes the separate fix-round starts of
  the spec and plan checks (up to 2 rounds each).

## P2LEAN-04: Drop the two stored derived views: the status record and the verdict record

- **Severity:** major (recommended)
- **Locations:** `state.md` 2.7 (all of it: `status.json`, its 8 KB cap and the commands that write it), 2.3
  (`detect`: `ready` needs a parseable `status.json`, `partial` when it fails), 2.9 (the briefing reads
  `status.json`), 2.10 (`status --fix` rebuilds it, and `vsdd status` flushes), 2.8 (the verdict record row; the
  evidence lookup for `review`); `BLUEPRINT.md` 4.7 (`local/status.json`), 4.9 (`review verdict`), 4.13 (verdict
  record), 5.2; `hooks.md` 2.8 (the Partial condition).
- **What it is now:** `status.json` copies facts held in closure and decision records, plus the route result. Nine
  commands rewrite it: survey, checkpoint, story deliver, gate wave, decide, spike open, spike close, checkpoint brief
  and note. The verdict record stores the result of running `converge` over the stored step records, beside the
  closure's own verdict.
- **What is lost by the cut:** the briefing becomes a computation instead of one read. It reads `factory.toml`, the
  lock, the driver flag, the open cycle's `cycle.json`, its closures and open decisions, then evaluates the route. That
  is tens of small JSON reads instead of four: still a few milliseconds, and well inside the 8-second `brief` deadline.
  After a crash, a delivery recomputes the verdict from step records instead of reading one file.
- **If left alone:** nine commands must each keep a derived copy current. A missing or stale copy is a failure condition
  of its own (`partial`), with its own repair. Each verdict exists twice, and the copies can disagree. `vsdd status`,
  which only reads, commits state.
- **Proposed resolution:** delete `status.json` and the verdict record.
  - `vsdd.state.briefing()` and `vsdd status` compute from the records each time; they are already one function.
  - `evidence(project, "review", subject)` computes from step records, or reads the closure after the checkpoint.
  - `detect` stops looking for a status record: `ready` means configuration plus mount.
  - `vsdd status` writes nothing; only `--fix` writes.
  - `vsdd review verdict` stops being a command; the function stays.

  This saves two record types, nine write points, one failure condition and its repair, one subcommand, and a flush on
  every status call.

## P2LEAN-05: Make long commands re-attach instead of handing out job ids

- **Severity:** major (recommended)
- **Locations:** `BLUEPRINT.md` 4.9 ("Long commands"; the `job wait` row; `--start` on `gate`), 4.10 (gate and job
  output); `state.md` 2.16 (the measuring commands preface; the `job wait` and `job status` row; the "(job)" marks);
  `pipeline.md` amendment 4, 2.10.8 (the job output row), 2.11.1 (the `relay()` comment); `agents-skills.md` amendment
  1 (the clerk repeats the wait).
- **What it is now:** every gate that builds or tests, and `story deliver`, starts with `--start`, which returns a job
  id. The caller then repeats `vsdd job wait <id> --max 540`.
- **What is lost by the cut:** nothing functional. Jobs, their records and the semaphore stay inside the tool. A caller
  can no longer address a job by its id. It addresses the job by the command and subject it already knows.
- **If left alone:** every prompt that runs a gate (test-writer, implementer, demo-recorder, verifier, clerk) carries a
  protocol of two commands. The agent must carry a job id from one tool call to the next, and an LLM can garble that
  value. The tool keeps a `job` command group, a second output shape, and `--start` on about fourteen commands.
- **Proposed resolution:** make each long command idempotent and re-attaching. The agent runs
  `vsdd gate green --story S-1.02`. The tool finds or starts the job for (gate, subject, head commit), waits up to nine
  minutes, and returns either the result or `{state: "pending"}`. On `pending` the agent runs the same command again.
  `story deliver` keys its job by story. Drop `--start`, `job wait`, `job status` and the `{ok, job, state}` shape. This
  saves a command group of two, one flag on about fourteen commands, one output schema, and a protocol in every prompt
  that runs a gate.

## P2LEAN-06: Drop per-stage input digests, `built_against`, `vsdd stale` and the stage registry file

- **Severity:** major (recommended)
- **Locations:** `pipeline.md` 2.5 ("How a workflow knows what is already done": the four-part test with input
  digests), amendment 17, section 4 (I2); `state.md` 2.6 (the staleness row), 2.8 (the closure fields
  `stages.*.inputs` and `built_against`; the refined done test), 2.16 (`vsdd stale`), section 5 (P-1); `BLUEPRINT.md`
  4.1 (`pipeline/stages.toml`), 4.14 ("Stages and routes"), 9.1 (C3); `packaging.md` 2.10 (`vsdd stale`).
- **What it is now:** about eighty stage entries each declare input and output globs in `stages.toml`. Closures store
  an input digest per stage, and the survey recomputes the digests to mark stages `stale`. Story closures store a
  digest of each contract they were built against, for `vsdd stale`, a report that no gate reads. The same stage
  structure also lives in each workflow script.
- **What is lost by the cut:**
  - Downstream stages no longer re-run automatically when a person hand-edits a draft upstream artifact between runs.
    The person runs the workflow with `only` naming the stages instead, and the specification review sees the
    inconsistency either way.
  - The `vsdd stale` report goes. After approval, contracts change only through an amendment, and the amendment's delta
    analysis already names the affected contracts and stories.
- **If left alone:**
  - About eighty glob pairs are written by hand and must be kept in step with the scripts. That makes two sources of
    stage truth, and no test cross-checks them: a renamed stage silently re-runs its work.
  - Digests are computed at every survey.
  - The mechanism still cannot reopen a stage when nothing on disk has changed, which is exactly what
    `revise "<guidance>"` and `hold "<guidance>"` need.
- **Proposed resolution:**
  - A stage is done when its closure (or, inside a run, a done step record) says so, the artifacts listed in its result
    still validate, and its gate has evidence for the current commit.
  - Stages reopen explicitly. Settling `revise` or `hold`, or opening an amendment, marks stages `todo` in the unit's
    closure: from the first stage the guidance names, or from review onward by default.
  - Remove `stages.toml`. The script becomes the one source of stage structure, and the survey reports the closure
    state of each stage name it finds.
  - Remove the per-stage digests, `built_against` and `vsdd stale`. Keep the decision digest, which binds an approval.

  This saves one data file, about eighty glob pairs, a digest per stage per unit, one closure field, one subcommand, and
  package C3, which merges into S3.

## P2LEAN-07: Fold about twelve `vsdd` subcommands into others

- **Severity:** major (recommended)
- **Locations:** `BLUEPRINT.md` 4.9 (the command table and exit codes); `state.md` 2.16; `packaging.md` 2.10 (the
  operator command table); `hooks.md` 2.13 (`hooks canary`), 2.9 item 7, 2.7 (the `[callers]` groups).
- **What it is now:** about 60 subcommands (counting each two-word name once) and 15 gate names.
- **What is lost by the cut:** a few names a person might type (`lock status`, `flush`). Their output appears in
  `status` and their action in `status --fix` instead. Exit code 3 also goes.
- **If left alone:** every subcommand is a module, a parser entry, a `[callers]` grant, a target for the L-XREF check,
  a generated reference row and a test file. Several exist only to expose a library function or to wait for another
  command.
- **Proposed resolution:**

  | Subcommand | What happens to it |
  |---|---|
  | `job wait`, `job status` | Removed (P2LEAN-05). |
  | `review verdict` | Becomes a library function (P2LEAN-04). |
  | `stale` | Removed (P2LEAN-06). |
  | `release prepare`, `release publish` | Become one `release` command (P2LEAN-02). |
  | `review diff`, `review path` | Become `review prepare <lens> [base]`, which prints both paths. Only the `review` skill uses it. |
  | `defer resolve`, `lesson retire`, `research retire` | Become `retire <path>`, which deletes only in classes that allow deletion. |
  | `flush` | Removed. Checkpoint, decide, deliver and `status --fix` flush, and the hook calls the function. |
  | `lock status` | Removed. `status` shows the lock. |
  | `hooks canary` | Removed. `doctor` runs through the Bash tool, so the hook has just written a session marker for that very call. A fresh marker proves the hooks are active, as it already does for the survey. Exit code 3 and the canary branch of `gate-guard` go with it. |
  | `events emit` | Removed (P2LEAN-16). |

  This takes the count from about 60 to about 48, removes one exit code, and removes the matching `[callers]` entries,
  reference rows and tests.

## P2LEAN-08: One decision kind for a blocked unit instead of four

- **Severity:** minor (worth considering)
- **Locations:** `BLUEPRINT.md` 4.11 (the rows `spike-limit`, `environment`, `stuck`, `review-blocked`); `pipeline.md`
  2.6 (exceptional stops; "Which decision a blocked step becomes"); `state.md` 2.8 (the choice table); `hooks.md`
  2.11.5 (grammar parsing).
- **What is lost by the cut:** the decision id no longer names the reason (`blocked.S-7.04` instead of
  `review-blocked.S-7.04`). The packet and the briefing line carry the reason instead.
- **If left alone:** four kinds with nearly the same choices (retry or done, direct, descope, plus `open-spike` or
  `skip`). That means four rows in the choice table, four branches in the checkpoint and four cases in the grammar
  tests.
- **Proposed resolution:** one kind, `blocked`, with a `reason` (`spike_limit`, `environment`, `stuck`, `reviewer`) and
  the choices `retry`, `direct` and `descope`. `open-spike` is added when the reason is `spike_limit`, and `skip` when
  an environment check is optional. This takes decision kinds from 12 to 9 and removes their rows, branches and cases.

## P2LEAN-09: Drop the lessons register, which nothing reads

- **Severity:** minor (worth considering)
- **Locations:** `state.md` 2.5 (Registers, lesson row: "folded into `conventions.md` or `review-rules.md` by the
  architect"), 2.4 (the `lessons` class; deletion); `BLUEPRINT.md` 4.3 (`researcher` may write `lessons`), 4.7 (layout
  and class table), 4.9 (`lesson retire`); `agents-skills.md` 2.8 (`retrospective` method), 2.9 (class and write
  tables); `pipeline.md` 2.9.7 (the `retro` stage).
- **What it is now:** the retrospective writes lesson files, capped at 30 adopted. The architect is meant to fold
  adopted lessons into `conventions.md` or `review-rules.md` and retire them. No workflow stage does the folding, and no
  step prompt passes lessons to any agent. OPS-28 raised the missing stage; the blueprint added caps but no reader.
- **What is lost by the cut:** a capped list of one-sentence rules kept apart from the retrospective report. The
  capability itself (capture lessons, propose improvements) stays in the retrospective, which is already a research
  report of kind `retrospective`.
- **If left alone:** a register with a path class, a wall entry, a cap, a status problem, a deletion command and an
  output format. It is filled at every release and read by nobody: a smaller copy of the old 1.4 MB `lessons.md` that
  "changed nobody's behaviour".
- **Proposed resolution:** the retrospective writes its proposals into its own report. Adopting one is an edit of
  `conventions.md` or `review-rules.md` through `/vsdd-factory:author`, and builders and reviewers already receive both
  files. Remove `lessons/`, its class, its cap, `lesson retire` and the lesson format. This saves one register, one path
  class and one subcommand. If the user counts the register as a capability in its own right, keep it, but add the
  folding step to `release`; do not keep it as written.

## P2LEAN-10: Replace the generated step-name sender table with the recorded agent type

- **Severity:** minor (worth considering)
- **Locations:** `hooks.md` 2.11.7 item 2, amendment 12; `BLUEPRINT.md` 5.4 (the `step-results` bullet);
  `packaging.md` 2.5 (the generated items).
- **What it is now:** the hook stores a step record only when the caller's agent type is one "the step name allows",
  taken from a table generated from the workflows at build time.
- **What is lost by the cut:** a record from the wrong agent type is stored with its type rather than refused at the
  hook. The refusal moves to the one point where the record is used.
- **If left alone:** one more generated artifact. It has to pull step names out of twelve scripts, where many names are
  built at run time from template strings such as `` `review-${l}-p${pass}` ``, and it must be regenerated and checked
  whenever a step is renamed.
- **Proposed resolution:** the hook stores the payload's `agent_type` in every record. The tool counts a pass toward a
  verdict only when a reviewer-tier agent type wrote it. That catches the forged-review case where it matters: the
  verdict computed before a merge. This saves one generated table and its step in the build.

## P2LEAN-11: One copy of each change request, and one record per cycle

- **Severity:** minor (worth considering)
- **Locations:** `state.md` 2.3 (layout: `cycles/<cycle>/request.md`, `summary.json`), 2.5 (Registers: change request
  row), 2.8 (rows: cycle record, request, cycle summary); `BLUEPRINT.md` 4.7 (layout).
- **What it is now:** a queued request is `requests/CR-NNN.md`, and when its cycle opens it is copied to
  `cycles/<cycle>/request.md`. A cycle has `cycle.json` from when it opens and `summary.json` from when it closes.
- **What is lost by the cut:** nothing. The totals in `summary.json` can be derived from closures, and its one field
  that cannot (the state-branch commit before pruning) fits in `cycle.json`.
- **If left alone:** two copies of every request, against rule 2 of `state.md` 2.2 ("each fact is stored in one
  place"), and two records per cycle.
- **Proposed resolution:** every request, queued or not, is `requests/CR-NNN.md`, and `cycle.json` names it
  (`request = "CR-012"` or `"brief"`). The release checkpoint adds `closed`, `version` and `archive_commit` to
  `cycle.json`. This saves two record types.

## P2LEAN-12: Drop the compaction snapshot file

- **Severity:** minor (worth considering)
- **Locations:** `hooks.md` 2.11.9 item 2, 2.11.8 third bullet; `BLUEPRINT.md` 4.7 (`local/hooks/`), section 7 (the
  row "The main session compacts while a workflow runs").
- **What it is now:** on `PreCompact` the hook writes a snapshot: branches, uncommitted counts per worktree, open
  spikes and the last 15 events. The next briefing adds up to three lines from it.
- **What is lost by the cut:** those three lines. The lock and the run record already say which workflow is running,
  and the survey reports `dirty` worktrees when the workflow continues.
- **If left alone:** a record type and a writer that repeat the lock, the run record and the survey. The writer also
  runs a `git status` per worktree inside the 25-second compaction budget, which the flush needs as well.
- **Proposed resolution:** keep the flush and the summariser instruction, which are the parts that matter. Drop the
  snapshot and the briefing branch that reads it. This saves one record type.

## P2LEAN-13: Fold `gates.toml` into `walls.toml` and code, and make the route table code

- **Severity:** minor (worth considering)
- **Locations:** `hooks.md` 2.7 (`gates.toml`: `human_only`, `governed`, `tests_first.exempt_kinds`, `[callers]`,
  `[reveals]`); `BLUEPRINT.md` 4.1 (policy tables), 4.6 (story kinds: the "Red gate" column), 4.14; `pipeline.md` 2.4
  (`routes.toml`), section 4 (I3).
- **What it is now:** `gates.toml` holds five tables. `exempt_kinds` repeats the "Red gate" column of the story-kind
  table. The `[callers]` groups are granted from `walls.toml` rows anyway. `human_only` and `governed` are constants.
  `routes.toml` is a list of named conditions on project state, and only Python evaluates it.
- **What is lost by the cut:** the ability to edit these lists as data without touching code. Nobody does that: they
  ship in the plugin and change only with it.
- **If left alone:** two more data formats, each with a loader, a schema and policy tests. `exempt_kinds` and the
  story-kind table can drift apart.
- **Proposed resolution:** move `[callers]` and `[reveals]` into `walls.toml`. Make `human_only`, `governed` and the
  exempt kinds constants, with the exempt kinds derived from the story-kind definitions. Make the route table an
  ordered list of (condition, next) in Python, beside the condition functions it needs anyway. This saves two data
  files and their loaders.

## P2LEAN-14: Merge seven build packages

- **Severity:** minor (worth considering)
- **Locations:** `BLUEPRINT.md` 9.1 to 9.6.
- **What is lost by the cut:** some parallelism between merged pairs, most of which already depend on each other.
- **If left alone:** about 70 packages, each with its own builder and reviewer starts and its own handoff.
- **Proposed resolution:**

  | Merge | Why |
  |---|---|
  | T2 into T3 | T3 depends only on T2. |
  | D1 into D2 | Both are documentation, and D2 depends on D1. |
  | S5 into CO1 | Both exist for the eight old projects, and CO1 depends on S5. |
  | H8 into W1 | `spike-bounds` is one rule, tested by the spike workflow, which already depends on it. |
  | T4 into W12 | `repos scan` serves only multi-repository projects. |
  | W8 into W7 | `release` is five steps and closes the same cycle. |
  | C3 into S3 | With P2LEAN-06 and P2LEAN-13, all that is left of C3 is the route function, which the survey and checkpoint call. |

  This saves 7 packages (70 to 63) and their review starts. Nothing is dropped. The packages for rare cases (the
  importer, cutover, export and render, multi-repository) are protected by D3, D27, D14 and the decision to port every
  capability.

## P2LEAN-15: Drop the two `PostToolUse` recorders

- **Severity:** minor (worth considering)
- **Locations:** `hooks.md` 2.3 (the `PostToolUse` row and the note on its two handlers), 2.11.10 (`lifecycle-log`:
  `commit.made`, `pr.created`), 2.12 (event types).
- **What it is now:** a filtered `PostToolUse` handler starts the runner after every `git commit` and every `gh pr`
  command, to log the event for the dashboards.
- **What is lost by the cut:** one event per agent commit in the log. The dashboards can take commits from the gates,
  which record the head they measure, and pull requests from the tool's own `pr.created` and `pr.merged`. In 1.0 no
  pipeline agent opens a pull request: the tool does, and the command families keep `gh pr` away from agents. A
  hook-written `pr.created` therefore never fires inside the pipeline.
- **If left alone:** one more wired event, and two handlers whose `if` filter the design itself calls best effort.
  Each commit pays about 55 ms.
- **Proposed resolution:** remove the `PostToolUse` wiring. The tool emits `pr.created` and `pr.merged`, and the gates
  emit `commit.made` for the heads they measure. This saves one wired event and two handlers.

## P2LEAN-16: Drop `vsdd events emit` and its reserved event types

- **Severity:** minor (worth considering)
- **Locations:** `hooks.md` amendment 22, 2.12 ("the one writer, used by the runner and by `vsdd events emit`");
  `packaging.md` 2.10 (the `events emit` row and "Two honest limits"); `BLUEPRINT.md` 8 (OPS-15), 9.6 (T2: "emit with
  reserved types").
- **What it is now:** `events emit` lets any caller append an event. A reserved-type list therefore stops agents from
  forging the event types that the runner writes and that the quarantine switch reads.
- **What is lost by the cut:** a shell entry point for writing events. No 1.0 agent, skill or workflow uses it, because
  the runner and the tool write in-process.
- **If left alone:** a command and a deny-list exist only to protect the log from that same command. The event
  directory is class `local`, with `write = "code"`, so no other path for an agent to write to the log exists.
- **Proposed resolution:** drop `events emit` and the reserved-type list. With them gone, only the runner and the tool
  can write the log, and the quarantine condition of OPS-24 and D12 keeps its meaning unchanged. This saves one
  subcommand and one deny-list.

## P2LEAN-17: Take lock liveness from the holding process instead of a heartbeat (needs a probe)

- **Severity:** minor (worth considering after a probe)
- **Locations:** `BLUEPRINT.md` 4.13 (Liveness); `state.md` 2.11, amendment 12, section 5 (H-2); `hooks.md` 2.11.10
  (`lifecycle-log` refreshes `seen`).
- **What it is now:** on any guarded tool call from the holder's session, the hook writes `seen` into `lock.json`, at
  most once a minute. The lock goes stale after 20 minutes without such a write. `SessionEnd` releases it.
- **What is lost by the cut:** nothing if the probe passes. If it fails, keep the heartbeat.
- **If left alone:** a hook in gate mode writes to the lock on the hot path. After a crash the next run waits 20
  minutes, or a person runs `vsdd lock release`. `lock_stale_minutes` must also stay above the longest command that
  runs in the foreground.
- **Proposed resolution:** the session marker also records the process id of the Claude Code process that launched the
  hook. The survey copies that id into the lock, and the lock is stale as soon as that process no longer exists on this
  machine. Drop the `seen` refresh, `lock_seen`, `lock_stale_minutes` and the 20-minute wait. Keep the `SessionEnd`
  release and `lock release` as backstops. Probe first that the launcher can find a long-lived Claude Code ancestor on
  macOS and on Linux, including with the Bash sandbox on.

## P2LEAN-18: Run survey and checkpoint from a hook on the `Workflow` call (not recommended for 1.0)

- **Severity:** minor (not recommended)
- **Locations:** `pipeline.md` 2.5 (every workflow starts and ends with a clerk); `hooks.md` 2.3 ("Every other tool
  (`Workflow`, …) never reaches it"; "`Workflow` calls never start the runner").
- **What it is now:** every top-level run spends two clerk starts, on `survey` and `checkpoint`, which is about 16 per
  greenfield cycle. No step can absorb them: the script needs the survey before its first step, and the checkpoint must
  run on every path out of the script.
- **What is lost by the cut:**
  - The design's rule that hooks never decide when work starts.
  - The survey would have to make worktrees inside a 10-second hook budget.
  - It would depend on `updatedInput` being honoured for the `Workflow` tool, which is unverified.
  - A checkpoint on `PostToolUse` would not run for a child `story` started by a parent, so two paths would exist.
- **If left alone:** about 16 clerk starts per cycle, at clerk-tier cost.
- **Proposed resolution:** no cut for 1.0. This is recorded so that the remaining survey and checkpoint clerks are known
  to be deliberate. Revisit it only if a probe shows that `updatedInput` works on `Workflow` and that the survey fits
  the hook budget.

---

## Checked and found sound

Each item below was examined for a cut and kept. Each one either enforces something, is needed to resume, or is
protected by a user decision.

1. The three-layer split: scripts steer, the tool measures, hooks enforce. It is the smallest arrangement that keeps
   truth out of agent output.
2. One state directory on its own branch, committed and pushed by code (D1, D2).
3. Run records: they hold the amendment flag for `spec_locked`, the live-run check for `step-results`, and the parent
   run for spike counts.
4. Step records stored by the hook: needed for verdicts, for the decisions the checkpoint opens, and for resume inside
   a run.
5. Evidence records and the lookup per gate: this is what `tests-first` and `story deliver` enforce from.
6. Binding evidence and verdicts to the `git patch-id`: one git command, and it avoids a re-review after a clean
   rebase.
7. Decision records, packets and the approval digest (D7).
8. Closure records, without the per-stage input digests: they are the tracked state that resume on another machine
   needs.
9. The run lock itself. Only how its liveness is measured is questioned (P2LEAN-17).
10. Detached jobs and the `parallel_gates` semaphore: forced by the ten-minute cap on a Bash call and by parallel
    stories. Only their command surface changes (P2LEAN-05).
11. The session marker that the survey requires: it stops a workflow running with the hooks off.
12. Survey and checkpoint as clerk steps (see P2LEAN-18).
13. The `story deliver` clerk: the step before it is a reviewer, which has no shell.
14. The `new-fix` clerk in the wave and verify gate loops: fix stories must be delivered between two gate rounds.
15. The convergence rule written in both JavaScript and Python, with the conformance fixtures. The JavaScript drives
    the loop without a clerk per pass; the Python is the check before a merge. Dropping either would cost more than
    the fixtures do.
16. Stamping the prelude and generating the schema literals: scripts cannot import or read files.
17. Generating `hooks.json`: it prevents a rule that is registered but never wired.
18. The 17 hook rules: each one maps to an enforcement or to a ported capability.
19. `[reveals]` and `[callers]`: needed because clerks and builders have a shell.
20. The 20-agent roster: each agent differs from the others by tier, tools, wall or write scope.
21. Thirteen skills, of which one is model-visible.
22. About 70 method files: they cost nothing at run time, and the build may merge them.
23. The test layers (pytest, the harness with its universal tests, conformance fixtures, lint, end to end). The weekly
    conformance tier is B8 run on a schedule, not a duplicate of it.
24. Spike limits enforced both in the wrapper and in `vsdd spike open`. The wrapper avoids a wasted start; the tool
    covers agents that dispatch a spike directly.
25. `notes.json` and `driver.json`: tiny, and both are needed to resume after compaction.
26. Packages that exist for one-off or rare cases (the importer, cutover, export and render, multi-repository):
    protected by D3, D27, D14 and the decision to port every capability, so they are merged (P2LEAN-14), not dropped.

Counts: 44 items examined. 26 were found sound, and 18 are proposed as cuts above.
