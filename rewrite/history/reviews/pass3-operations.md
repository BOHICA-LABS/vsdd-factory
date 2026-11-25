# Review pass 3: operations (how the design behaves in use)

Date: 2026-11-24. Branch `rewrite/v1`. Reviewer lens: operations. I read `rewrite/STATE.md`, the blueprint and the
five detail designs, and walked each scenario in the brief against them, using the script skeletons in
`pipeline.md` 2.11 as the executable statement of behaviour. R1 to R5 of `pass2-closeout-3.md` are not repeated.
Line numbers are left out where a section name is enough.

## Verdict

The happy path holds together, and so do most single failures inside one run. A dead agent, an empty result, a
compaction of the main session or of a step, a missing `uv`, a spike storm and a review that never converges
all end in a bounded number of agent starts and a named state. The agent-start budget (about 4 to 10 per story,
70 to 90 per realistic six-story wave) matches the code. The design is weakest where one run stops and the next
one starts. The run lock cannot be released in the session that orphaned it. A resumed parent cannot hand a
child a story that an earlier run admitted. A crashed planner run redoes everything. `direct` after a cap runs no
round at all. A hand-edited approved specification leaves no decision and no command. A second existing clone
never fetches state. Two enforcement holes sit on the thin-slice path:
- Delivery never checks for red evidence, so a fabricated red gate merges code that was never test-first.
- In local delivery mode, an agent can move the integration branch with git ref commands that `merge-guard`
  does not match. Local mode also has no defined way to merge without touching the person's checkout.

The tool cannot tell which method a stored review pass belonged to, so the agent-type check on review passes
cannot be built as written. The baseline comparison that brownfield depends on can be built only for JUnit
tests. It cannot be built for lint, format or scanner findings, or for a suite that writes no JUnit report at
ingest. I found no blocker. There are 16 major and 9 minor findings, 9 of them on the slice path.

---

## P3OPS-01. A run lock orphaned in a live session is never released, and nothing names the command that releases it

- Severity: major
- On the slice path: yes (B4a prelude `relay()`, S2b lock, S3 survey, H1 liveness)
- Locations:
  - `pipeline.md` 2.11.1, `relay()`: one retry when the clerk returns nothing.
  - `pipeline.md` 5.1 need S2: the survey is "Idempotent".
  - `pipeline.md` 2.11.2 `story.js`: `where` is built from `u.bcs` before `try`.
  - `pipeline.md` 2.4, route row "A run lock is held by a live run → human: wait, or inspect the run".
  - `state.md` 2.11 (first bullet: "A second run on the same scope gets `ok: false`"; the liveness and hard-crash
    bullets).
  - `state.md` 2.10, `status --fix` ("releases a lock whose holder is dead").
  - `state.md` 2.16, `vsdd lock release [--force]`.
  - `hooks.md` 2.11.10, `lifecycle-log`.
  - Blueprint 4.9 (rows for `status --fix`, `lock release`, `lock release --force`), 4.13 Liveness, and section 7
    ("A session crashes").
- Problem. Liveness is keyed on the session: any guarded call from the holder's session refreshes `seen`. Three
  ordinary events leave a lock held by a run that is no longer running, inside a session that is still alive:
  - The clerk runs `vsdd survey` (which takes the lock), then dies or returns nothing. `relay()` retries, and the
    second survey is refused because the scope is held by the first run.
  - A person interrupts a running workflow, so the script's `finally` never runs.
  - A statement between the survey and `try` throws.

  The person's own tool calls then keep `seen` fresh, so the lock never goes stale. `status --fix` releases only a
  lock "whose holder is dead", and no rule makes a live session's holder dead. Plain `vsdd lock release` has no
  stated semantics beside `--force`. After a hard crash, the briefing names `/vsdd-factory:status --fix`, which
  cannot tell a crashed holder from a live one for the first 20 minutes. `pipeline.md` S2 calls the survey
  idempotent; `state.md` 2.11 says a second survey on the scope is refused. The two disagree.
- What breaks. After an interrupt or a clerk hiccup, re-running the same command returns "lock held". The route
  says "wait". The driver stops after two runs with no progress. The person has no working command except ending
  the session, or knowing to run `vsdd lock release` with unstated effect. This is the first resume most people
  will try.
- Proposed resolution:
  1. Survey re-entry. When the scope is held by a run of the caller's own session, for the same workflow and
     subject, and that run has stored no step record, the survey takes that run over and returns it. That makes
     the `relay()` retry safe. State it in `state.md` 2.11 and 2.16 and in `pipeline.md` S2.
  2. Liveness. Refresh `seen` only on calls that belong to the run: a step's `StructuredOutput` carrying that
     `step.run`, a `vsdd` call carrying `--run <id>`, or a call from a worktree the run owns. Do not refresh on the
     main session's own calls (`state.md` 2.11, `hooks.md` 2.11.10).
  3. Define the commands:
     - `vsdd lock release` (main session) releases a lock held by the caller's own session, or a stale one.
     - `--force` (person only) releases any lock.
     - "Holder is dead" means stale or session ended.
     Write this in `state.md` 2.10, 2.11 and 2.16, and in blueprint 4.9.
  4. Name the next command:
     - When the holder is the current session, the route and the briefing give
       `vsdd lock release` ("if that workflow is no longer running here").
     - Otherwise they give the last-seen time and `vsdd lock release --force` in a terminal.
  5. Fixture: interrupt a workflow after its survey, re-run the same command in the same session, and the run
     continues.

## P3OPS-02. Delivery never requires red evidence, so a fabricated red gate merges code that was never test-first

- Severity: major
- On the slice path: yes (G2 gates, G4a delivery, H5 `tests-first`, H7 `step-results`)
- Locations:
  - `pipeline.md` 2.9.1: the tests row, the deliver row ("refuses to merge without green evidence on the head, a
    converged verdict ..., the demo gate and green CI"), and the green-gate detail ("compared against the latest
    red or re-freeze record").
  - `state.md` 2.16: the `vsdd gate green` and `vsdd story deliver` rows.
  - `hooks.md` 2.11.4: the `merge-guard` list of what makes a merge legitimate; `tests-first` is wired to Edit,
    Write, MultiEdit and NotebookEdit only (2.10 row 6).
  - `hooks.md` 2.11.7: `step-results` checks findings only, not gate entries.
  - Blueprint 3.2 ("A fabricated step result can waste a pass. It cannot merge code.") and 4.13.
- Problem. The test-writer may write `source` before red evidence exists. Nothing checks the gate entries of a
  step result against evidence records, and the script trusts `g(r,"red").passed && sha == head`. A test-writer
  can therefore commit tests together with the implementation and return a hand-made passing red result. Then:
  - The implementer has nothing to write, so `tests-first` never fires.
  - `gate green` passes, because there is no red record to compare frozen tests with.
  - `story deliver` merges, because it checks green, verdict, demo and CI, and never red.

  An implementer could also write source before red through Bash, because `tests-first` does not inspect shell
  writes.
- What breaks. The tests-first gate, the point of the method, can be bypassed by one lie, and nobody notices.
  The blueprint's own guarantee in 3.2 is false.
- Proposed resolution:
  1. `gate green` and `story deliver` require a passing red or re-freeze record on the story branch for every
     kind that `kind_has_red_gate`; otherwise green fails and delivery returns `refused` (`pipeline.md` 2.9.1,
     `state.md` 2.16, `hooks.md` 2.11.4, blueprint 4.13).
  2. `step-results` checks each `gates[]` entry against the evidence record named in its `evidence` field: the
     record exists in the code-only class, with the same gate, subject, sha and `passed`. A mismatch is blocked
     with "this gate result does not match its evidence record" (`hooks.md` 2.11.7).
  3. `tests-first` also inspects Bash writes into `source` (redirect, `tee`, `sed -i`, `cp`, `mv`, `git apply`,
     `git checkout -- <path>`) for a plain-`source` caller before red (`hooks.md` 2.11.4).
  4. Cases: a fabricated red result is refused at its `StructuredOutput`, and a story with no red record cannot
     merge.

## P3OPS-03. Local delivery mode has no defined way to merge without touching the person's checkout

- Severity: major
- On the slice path: yes (G4a, local mode)
- Locations:
  - `pipeline.md` 2.9.1 deliver detail ("With `delivery.mode = "local"` (no remote) it merges locally").
  - `pipeline.md` 3.6 ("Nothing in 1.0 pulls or merges in the person's checkout").
  - `state.md` 2.3 (the person's checkout is "never switched, pulled or committed to") and 2.12 `delivery.mode`.
  - `hooks.md` 2.11.4 (local mode is "where the integration branch is the product branch").
  - Blueprint 4.12 `delivery.mode`.
- Problem. With no remote, the integration branch is usually the branch checked out in the person's main
  checkout. Git refuses to check that branch out in a tool-owned worktree. Its ref can only be moved with
  `git update-ref`, which leaves the person's index and working tree showing the story reversed; a later commit
  there undoes the story. The alternative is merging in the person's checkout, which the design forbids and which
  clobbers or fails on uncommitted work. The design names neither path.
- What breaks. The slice delivers in local mode on a sample checkout on its integration branch, so the first
  delivery hits this. A builder must improvise a merge that either breaks the stated invariant or corrupts the
  person's checkout state.
- Proposed resolution. Define the local merge in `pipeline.md` 2.9.1 and `state.md` 2.16:
  - When the integration branch is checked out in a worktree with no tracked changes, merge there with the
    configured strategy, which keeps that worktree consistent.
  - When that worktree is dirty, return the state `checkout_dirty`. The checkpoint opens `blocked` with the
    reason `environment`, naming the checkout and "commit or stash, then `retry`".
  - When the branch is checked out nowhere, merge in a tool-owned detached worktree and move the ref.
  - Amend the invariant in `state.md` 2.3 and `pipeline.md` 3.6: "touched only by a clean merge in local mode".
  - `vsdd init` and `vsdd doctor` report when the integration branch is checked out at the project root in local
    mode.
  - Add slice tests for the clean and dirty cases.

## P3OPS-04. Agents can move the integration or release branch with git ref commands that `merge-guard` does not match, and the command-family table is wrong both ways

- Severity: major
- On the slice path: yes (H5 `merge-guard`; the test-writer and implementer rows)
- Locations:
  - `hooks.md` 2.11.4 `merge-guard`: it matches `gh pr merge`, merge, rebase or cherry-pick on a checked-out
    branch, commit or reset on a checked-out branch, and push.
  - `hooks.md` 2.11.1, the command-family table: `git-read` is "every other `git` and `gh` command".
  - `hooks.md` 2.7, the example rows: the implementer and test-writer have `commands = ["git-commit"]`; the
    verifier and devops have no `commands`, which means `*`.
  - `agents-skills.md` 2.9 command scopes: "Inside the pipeline no agent runs" `git push`, `gh pr create` or
    worktree add and remove.
  - `hooks.md` 2.11.10 `auto-allow` `core`: approves "`git add`, `commit`, `status`, `diff` and `log`".
- Problem. Commands that move a branch ref without having it checked out are not covered: `git update-ref
  refs/heads/<integration>`, `git branch -f|-M|-C <integration>`, `git switch -C` and `git checkout -B` naming it,
  and `git fetch . story/S-1:<integration>`. They fall in `git-read`. The tool never checks that the integration
  head is one it produced. The family table is also inconsistent the other way:
  - If `git-read` must be granted, the test-writer and implementer may not run `git status`, `git diff` or
    `git log`.
  - If `git-read` is implied, ref writes pass.
  - The verifier and devops rows default to every family, including `git-push`, `gh-pr` and `git-worktree`,
    which `agents-skills.md` forbids.
- What breaks. In local mode an agent can put unreviewed code on the integration branch in one command, and
  the next wave gate tests it as if it were delivered. On the slice, builders are either blocked from reading git
  state or free to write refs.
- Proposed resolution:
  1. Narrow `git-read` to read-only subcommands (`status`, `diff`, `log`, `show`, `rev-parse`, `branch --list`,
     `ls-files` and the like).
  2. Add a family `git-ref` (`update-ref`, `branch -f|-D|-M|-C`, `tag -f`, `switch -C`, `checkout -B`, a fetch or
     push whose refspec writes a local branch). No agent row holds it, and `merge-guard` blocks it for every caller
     when it names the integration or release branch.
  3. Every agent row lists `commands` explicitly, and an absent key means none, as `dispatch` already does
     (`hooks.md` 2.7). Builders get `git-commit` and `git-read`.
  4. `story deliver` and `gate wave` check that the integration head is the last head the tool recorded, or
     descends from it only through the tool's own merges. Otherwise delivery refuses with `integration_moved`
     (`state.md` 2.16).

## P3OPS-05. The tool cannot tell which method a stored review pass belonged to, so "counted only when the agent type owns the pass's method" cannot be built

- Severity: major
- On the slice path: yes (G3 verdict, H7 `step-results`)
- Locations:
  - `hooks.md` 2.11.7 item 2 ("the reviewer that owns the pass's method"; "no table of step names").
  - `pipeline.md` 2.10.1, step identity (`run`, `workflow`, `name`, `subject`, `attempt`).
  - `pipeline.md` 2.10.3 and 2.9.1 (the "Checked again by" column of the review row).
  - `state.md` 2.8, step record.
  - Blueprint 4.10 Verdict row, 5.4, and P2LEAN-10 in 11.1.
- Problem. The method appears only in the prompt. A step record carries the identity, the agent type and the
  head. Step names are built at run time and follow no single rule: `review-<agent>-p<k>` in `story`;
  `integration-review-r<k>`, `security-r<k>` and `holdout-r<k>-b<i>` in `wave`; `system-review-<unit>-r<k>` in
  `verify`. The tool has nothing that maps a stored record to a method. It must either rebuild the step-name table
  the second pass removed, or skip the check.
- What breaks. G3 either rejects real passes, so no verdict converges and every delivery is refused, or does not
  check, so a builder can store a clean "review". The slice's first review is where this is decided.
- Proposed resolution:
  - Add `method` to the step identity. `brief()` puts it in the identity the agent echoes, and `step-results`
    stores it (`pipeline.md` 2.10.1, 2.11.1; `hooks.md` 2.11.7; `state.md` 2.8).
  - The tool counts a record as a pass at a perimeter only when its method is one of that perimeter's review
    methods and the stored agent type owns that method in the roster, which lint already checks. A builder that
    echoes a reviewer's method still fails on its stored agent type.
  - Add a conformance fixture for that case.

## P3OPS-06. A crashing rule is not surfaced with its remedy, and an import error in a rule can never be quarantined

- Severity: major (decision D12's purpose)
- On the slice path: yes (H1 runner core)
- Locations:
  - `hooks.md` 2.5, main loop: `registry.select()` imports rule modules outside the per-rule `try`.
  - `hooks.md` 2.9, rows "A gate rule raises an exception" and "Runner crashes before rules run".
  - `hooks.md` 2.6, quarantine bullet ("only when the event log shows repeated `rule.error` events").
  - `hooks.md` 2.11.8, the briefing's runner lines, which have no rule-error line.
  - `state.md` 2.9, the State problems line.
  - `pipeline.md` 2.6, the `blocked` reason `stuck`.
  - Blueprint D12.
- Problem. A deterministic crash in, for example, `step-results` or `gate-guard` blocks every step's result or
  every clerk command. The blocks reach step agents, not the person. The workflow fails, and two runs later the
  person gets `blocked` with the reason `stuck` and the choices `retry`, `direct` and `descope`, none of which
  helps. Nothing tells them to restart with `VSDD_FACTORY_HOOKS_QUARANTINE=<rule>`. A syntax or import error in a
  rule module is a runner crash: it is logged as `runner.launch_failed`, never as `rule.error`, so the quarantine
  condition can never be met. The only escape then is `VSDD_FACTORY_HOOKS=off`, which turns every gate off.
- What breaks. D12 adopted the quarantine switch so that a bug in one rule does not lock a project, but the
  person is never led to it, and for module-level bugs it cannot work.
- Proposed resolution:
  - Import each rule module inside the per-rule `try`, so an import error is a `rule.error` of that rule
    (`hooks.md` 2.5).
  - The `rule_error` block's fix line names the restart command with the quarantine variable (`hooks.md` 2.9).
  - The briefing and `vsdd status` add a line for any rule with repeated `rule.error` in the last day, naming it
    and the restart line (`hooks.md` 2.11.8, `state.md` 2.9).
  - The checkpoint copies the last block codes logged for a unit into its `blocked` packet (`pipeline.md` 2.6).

## P3OPS-07. A resumed parent workflow cannot hand a child a story that an earlier run admitted

- Severity: major
- On the slice path: no (touches S3 and G2, but the slice runs `story` standalone)
- Locations:
  - `pipeline.md` 2.9.1, "Where a child's unit comes from" (every gate checks "that the story is recorded in the
    run they are given").
  - `pipeline.md` 2.9.4, the survey, story-draft and deliver rows (the units come only from the story-draft
    step's `data.units`).
  - `state.md` 2.16, `vsdd gate plan` ("Run again for a story it has admitted, it changes nothing").
  - `state.md` 2.8, run record ("the stories admitted to it").
  - Blueprint 3.3 steps 3 and 4, 3.4 table row 4, D15.
- Problem. A fast-route story is admitted by run R1 and recorded in R1's run record. R1 can end with the story
  `awaiting_merge`, or crash in the middle of the child. Under D15 the first CI story of a no-CI project and any
  dependency change always wait. The route then sends `feature` again, and the new run R2 has two bad paths:
  - Story-draft is done, so the script skips it and has no units to pass to `runStories()`. The `feature` survey
    returns none.
  - If story-draft runs again, `gate plan --story` "changes nothing", so the story is not recorded in R2, and
    every gate of the child refuses it as a garbled unit.

  Fix stories admitted by `vsdd story new-fix` in a crashed `wave` or `verify` run are in the same position.
- What breaks. Every fast-route story that waits for a person, and every fast route or fix round interrupted
  mid-child, never completes through the route. The driver stops with no progress, and only a person who knows to
  run `/vsdd-factory:story <id>` by hand gets past it. This is the blueprint's own example path (3.3) and E4 case
  2.
- Proposed resolution:
  - The `feature`, `wave`, `verify` and `maintain` surveys return, as units, every story admitted to the open
    cycle that is neither delivered nor descoped, and record each in the new run (`state.md` 2.16, `pipeline.md`
    2.9.4).
  - Re-running `gate plan --story` or `story new-fix` for an admitted story records it in the calling run.
  - The `feature` script passes the survey's admitted units to `runStories()` when story-draft is done.
  - Scenario tests: a fast-route story waits on `merge`, the decision is approved in a new session and `feature`
    delivers it; and a kill in the middle of the child.

## P3OPS-08. A crash in the middle of a planner workflow redoes it, and a resumed standard-route `feature` holds no amendment

- Severity: major
- On the slice path: no
- Locations:
  - `pipeline.md` 2.5 and `state.md` 2.8, the done test ("inside a run that has not reached its checkpoint, a
    step record with status `done` exists").
  - `state.md` 2.8 ("a fresh clone ... loses only the steps of a run that never reached its checkpoint").
  - `pipeline.md` 2.9.4, the delta row: only `vsdd feature route` marks the amendment.
  - Blueprint 4.14 "When a stage is done", and section 7 ("Its survey skips what is done").
  - Also noted, uncounted, in `pass2-closeout-3.md` under "Seen while checking".
- Problem. The done test names "a run that has not reached its checkpoint" without saying it includes an earlier
  run that died. The natural implementation reads the current run's records, so a resumed `spec`, `ingest` or
  `plan` redoes every Fable or Opus step of the crashed run, and the stated resume behaviour does not hold for
  those workflows. If earlier records do count, a resumed standard-route `feature` skips its done delta step.
  Then nothing marks the run as holding the amendment, and every `evolve` write is refused by the specification
  lock.
- What breaks. A crash in a multi-hour planner run costs the whole run. A crash after the delta step of a
  standard feature stalls the cycle on lock refusals.
- Proposed resolution:
  - State that done step records from any earlier run of the same workflow and unit that ended without a
    checkpoint count, once they validate against their schema (`pipeline.md` 2.5, `state.md` 2.8, blueprint
    4.14).
  - The `feature` survey marks the run as holding the amendment when the cycle's route is `standard` and the cycle
    has no approved `spec` decision of its own (the same predicate as R2).
  - Fixtures: kill a `spec` run after two stages and re-run it, with no agent for those stages; kill a
    standard-route `feature` after the delta step and re-run it, and `evolve` writes are allowed.

## P3OPS-09. After a cap, `direct` runs no round, a waiver depends on unstated tool behaviour, and a person's guidance never reaches an agent

- Severity: major (a regression of OPS-23)
- On the slice path: no (W2 code, off the happy path)
- Locations:
  - `pipeline.md` 2.11.1 `converge()` (`while (used + pass < c.cap + extra)`).
  - `pipeline.md` 2.11.2 `story.js`: `review(u.base, RUN.limits.passes, u.acs, u.rounds_used)`; `u.guidance` is
    never used.
  - `pipeline.md` 2.11.3 `wave.js` (`used: s.rounds_used`).
  - `pipeline.md` 2.6 (`direct`: "exactly one more round with the guidance"; `revise` and `hold`: "runs again ...
    with the guidance").
  - `state.md` 2.8, the choice table.
  - Blueprint 4.11 ("whose fix loop then works on the guidance") and 5.1 ("`direct` grants one more round").
- Problem. A unit blocked at its cap has `rounds_used` of at least `cap + 1`. On the next run `converge()` returns
  `blocked`/`cap` before any pass, with an empty `open` list, so `direct` does nothing and the checkpoint opens the
  same `finding` again. A waiver converges only if the tool's verdict applies settled waivers to the stored last
  pass and reports the position `reviewed`, which no document says. The survey returns `guidance` in every unit,
  but no prompt in the skeletons carries it. The guidance from `revise`, `hold`, `direct` and `blocked ... direct`
  never reaches an agent.
- What breaks. A person's only non-destructive answers to a disputed finding at the cap do nothing, and the unit
  loops back to the same decision.
- Proposed resolution:
  - The survey returns, per unit, `direct` (the guidance and the open keys) when the latest `finding` decision was
    settled `direct`. `converge()` then starts with one fix carrying the guidance, followed by one pass, with no
    arbitration.
  - The tool's verdict applies waivers from settled `finding` decisions, so a waived unit's position is
    `reviewed`.
  - `brief()` adds "Guidance from a person: ..." whenever the unit or survey carries guidance, with a universal
    harness test.
  - Conformance fixtures for `direct` and `waive` after a cap (`pipeline.md` 2.11, `state.md` 2.8 and 2.16,
    `packaging.md` 2.6).

## P3OPS-10. A person who edits an approved specification file by hand gets no decision, no briefing line and no command

- Severity: major
- On the slice path: no
- Locations:
  - `pipeline.md` 2.6 ("the decision reads as `stale` and the stop returns").
  - `state.md` 2.8 ("a decision whose digest no longer matches reads as `stale` and cannot be settled").
  - `pipeline.md` 2.5, the meaning of "The specification is approved".
  - `pipeline.md` 2.4, the route table, which has no row for a stale approval.
  - `state.md` 2.9, the State problems line.
  - `hooks.md` 2.11.5, step 3.
  - Blueprint D19 and 4.11.
- Problem. Settling refuses a stale decision, and nothing opens a new one. In a cycle opened by a change request
  the `spec` row cannot match, because it needs an approved brief. `plan`, `wave` and `story` are refused, because
  the specification is not approved. Every "could run" test is false, so the route ends at "only open decisions
  remain", with none listed. The briefing does not mention staleness. In a fast-route cycle `verify` does not need
  the approval, so the edit reaches a release unreviewed.
- What breaks. "The stop returns" has no mechanism. The person is left with no command, or an unapproved
  specification change ships, against D19.
- Proposed resolution:
  - Add a route row: "the latest `spec` decision is stale → human: that decision".
  - The next survey or checkpoint that finds the stale digest opens a new `spec` decision. Its packet lists the
    files changed since the approval, against the approval's state commit.
  - The briefing lists it under State problems, with the alternative "revert the files".
  - In-flight stories wait until it is settled (`pipeline.md` 2.4 and 2.6, `state.md` 2.8 and 2.9).

## P3OPS-11. Inherited lint, format and scanner findings have no machine-readable form or identity, so the baseline comparison cannot be built

- Severity: major (the brownfield requirement)
- On the slice path: no
- Locations:
  - Blueprint 3.4 ("a lint or scanner finding that already existed ... do not fail a gate") and 4.13 (the
    baseline).
  - `pipeline.md` 2.9.1 (the green gate on a project with a baseline) and 2.9.8 ("What the baseline is for").
  - `state.md` 2.8 (the baseline record holds "the lint and format findings") and 2.12 (`lint`, `format_check`,
    `scan` and `audit` are plain shell strings; only tests have a `test_report`).
  - `packaging.md` 2.6: the E4 row ("The inherited lint failure does not fail any gate, and a new one does") and
    the baseline fixture, which covers tests only.
- Problem. The tool receives an exit code and text from a linter, a formatter or a scanner. Nothing defines a
  report format, or what makes a finding "the same" after the code above it moves. Without both, "only what is
  new blocks" cannot be computed. Lint fails at the baseline and on every story.
- What breaks. On any ingested project with lint debt, every green gate fails and every story ends `stuck`. E4
  run 1 is designed to fail on exactly this.
- Proposed resolution:
  - Add `commands.lint_report`, `format_report` and `scan_report` (`sarif:<path>` or `json:<path>`, detected where
    the tool can produce one) to `state.md` 2.12.
  - Define a finding's identity as tool, rule, path and a hash of the flagged line's normalised text.
  - Without a report, compare by exit status only: the check fails a gate only if it passed at the baseline. The
    briefing announces that the check cannot fail while the baseline fails it, and a readiness chore to configure
    the report is scheduled.
  - Add a G2 fixture with an inherited finding whose line moves.

## P3OPS-12. An ingested codebase whose suite fails and writes no JUnit report gets a baseline that cannot protect it

- Severity: major
- On the slice path: no
- Locations:
  - `pipeline.md` 2.9.8, the coverage-and-baseline row: the baseline is measured before `synthesize` writes
    `toolchain.toml`.
  - `pipeline.md` 2.9.4, delta row: readiness asks for a harness "when the baseline has no suite".
  - `state.md` 2.12, "Where the commands cannot work yet": a harness chore "for a codebase with no suite or no
    JUnit report".
  - `state.md` 2.8, baseline record: only "for an import" does "the first survey that needs the measurements"
    take them.
  - Blueprint 3.4 row 4 and 4.13.
- Problem. Three separate defects:
  1. Most projects write no JUnit report until configured. Such a baseline records no identities for the tests
     that fail today. The harness chore's own green gate, and the first bug fix after it, then see every inherited
     failure as new.
  2. `pipeline.md` and `state.md` disagree on whether a suite without a report gets the harness chore. Under the
     `pipeline.md` wording, a bug-fix route skips it and the red gate opens `blocked` with the reason
     `environment`.
  3. An inherited failing test that a story makes pass stays "inherited". A later story can break it again
     without failing any gate.
- What breaks. On this case, a bug fix on an ingested project with a failing suite cannot pass green. Once a fix
  is delivered, its regression can return unnoticed.
- Proposed resolution:
  1. The baseline records whether per-test identities were measured. When they were not, delivery of the
     test-harness chore re-measures them on the new integration head and writes them into the baseline record, as
     an import's first survey does. That chore's own green gate requires only that the report is produced and its
     own tests pass.
  2. One readiness condition everywhere: "no suite or no JUnit report".
  3. A delivered story's closure record lists the inherited tests it made pass, and gates stop treating those as
     inherited (`pipeline.md` 2.9.4 and 2.9.8, `state.md` 2.8 and 2.12, blueprint 4.13).

## P3OPS-13. A second existing clone never fetches the state branch, and two machines can build the same wave at once

- Severity: major
- On the slice path: no
- Locations:
  - `state.md` 2.2 rule 6 ("Tracked state is enough to resume on another machine").
  - `state.md` 2.3, "Push" (a fetch happens only when a push is rejected).
  - `state.md` 2.11: "It is local, so it does not stop two machines", and the adoption bullet.
  - `state.md` 2.9, State problems ("the last push failed").
  - Blueprint 4.13, section 7 ("A fresh clone"), and OPS-07 in section 8 ("the cross-machine lease is not built").
- Problem. Nothing fetches the state branch before the briefing or the survey. A person who moves from laptop to
  desktop is routed on stale local state, and the desktop redoes work that the laptop already checkpointed. When
  both machines run at once, both build the same stories, and the adoption rule lets one take over the other's
  open pull request in the middle of delivery. Both reach the checkpoint push with conflicting closure records.
  "A real conflict ... is reported to a person" with no command to resolve it.
- What breaks. Duplicate agent spend, racing deliveries, and a diverged state branch with no named repair.
- Proposed resolution:
  - When a remote exists and `state.push` is true, the survey fetches the state branch and fast-forwards, or
    rebases local commits as the checkpoint does, before it evaluates preconditions. The briefing says "state is
    behind the remote" from the last fetch.
  - The survey writes and pushes a lease (`cycles/<cycle>/lease.json`: scope, run, machine, time) before it starts
    agents. Another machine's survey refuses while the lease is fresh. Each checkpoint and delivery push refreshes
    it, the checkpoint clears it, and it goes stale after a set time.
  - A push conflict opens `blocked` with the reason `environment`, naming the records, and `status --fix` retries
    the rebase (`state.md` 2.3 and 2.11, blueprint 4.13).

## P3OPS-14. `open-spike` has no effect: the unit runs again, asks again and is blocked again

- Severity: major
- On the slice path: no
- Locations:
  - `pipeline.md` 2.6, the `blocked` row ("For `spike_limit` also `open-spike name=<name>`").
  - `pipeline.md` 2.8 ("the person can open a named spike").
  - `state.md` 2.8, the choice table (`open-spike` is an `approved` choice) and "Settling changes no artifact".
  - Blueprint 4.11 and section 6.
- Problem. Settling writes only the decision record. It opens no named spike, carries no mission, and leaves the
  unit runnable. On the next run the step asks for its spikes again, uses two more and becomes the same `blocked`
  decision. When a person later runs `/vsdd-factory:spike close <name>`, nothing feeds that finding back to the
  step.
- What breaks. A step that keeps asking for spikes loops between runs at about four to six agent starts a time,
  and the person's escalation choice is not wired to anything.
- Proposed resolution:
  - Settling `open-spike name=<n>` also writes the named-spike request, with the questions from the packet, as
    `spec-gap amend` writes its request.
  - The unit stays blocked on that spike, and the route gives `/vsdd-factory:spike open <n> -- <question>`.
  - `vsdd spike close <n>` releases the unit, and the asking step receives the finding in its spike list on the
    next run (`pipeline.md` 2.6 and 2.8, `state.md` 2.8 and 2.16).

## P3OPS-15. The `core` scope of `auto-allow` approves a configured command followed by anything

- Severity: major (D4 security)
- On the slice path: no
- Locations:
  - `hooks.md` 2.11.10, the `auto-allow` table row `core` ("matched as a prefix: the configured string followed by
    further arguments ..., with no pipe to a shell and no redirect outside the worktree").
  - `hooks.md` 2.11.1 (`shell.py` splits segments; it says nothing about `$(...)`, backticks, `eval` or
    `sh -c`).
  - Blueprint 5.4, the second-pass `auto-allow` bullet.
- Problem. The built-in part requires "no pipe, redirect or second segment". The `core` prefix match does not. A
  builder that has been prompt-injected by a dependency's files can run a configured test command, then `;`, `&&`,
  a command substitution, `eval` or `sh -c`, with an opaque payload. The heuristic checks may not recognise the
  payload, and `auto-allow` approves it with no prompt in the default permission mode, which is the mode D4 exists
  for.
- What breaks. The last line of defence, the permission prompt, is removed for exactly the commands the
  heuristics cannot read.
- Proposed resolution:
  - A configured command is approved only when the whole call is one segment: the configured string followed by
    plain argument tokens, with no `;`, `&&`, `||`, pipe, newline, `$(`, backtick, `<(`, `>(`, `eval`, `sh -c`,
    `bash -c`, `xargs` or redirect.
  - `shell.py` marks a command with substitutions as opaque, and `auto-allow` never permits an opaque command.
  - Add cases to `tests/hooks/cases/auto-allow.toml`.

## P3OPS-16. The `secrets` rule misses common credential files and one token-printing command, and builders may read outside the project

- Severity: major
- On the slice path: no
- Locations:
  - `hooks.md` 2.11.1, the `secrets` row: home credential files are `~/.aws`, `~/.ssh`, `~/.netrc`,
    `~/.docker/config.json`, `~/.config/gh`, `~/.npmrc`, Claude's files, `*.pem` and `*.key`.
  - `hooks.md` amendment 16.
  - `hooks.md` 2.7 (builder rows do not deny `outside`).
  - Blueprint 5.4, the `secrets` bullet.
- Problem. Several files are readable by every builder, which keeps `outside` readable for its toolchain:
  - `~/.git-credentials` and `~/.config/git/credentials`
  - `~/.cargo/credentials.toml`
  - `~/.pypirc` and `~/.gem/credentials`
  - `~/.kube/config`
  - `~/.config/gcloud/**` and `~/.azure/**`
  - `~/.terraform.d/credentials.tfrc.json` and `~/.gnupg/**`

  `gh auth status --show-token` (`-t`) prints the token, as `gh auth token` does.
- What breaks. A credential can be read by an agent that also runs network commands, and only the "common token
  format" check stands in the way.
- Proposed resolution:
  - Add these paths to the default credential list. Keep the list as data (the default `secret_globs`) with a case
    per entry.
  - Block `gh auth status -t` and `--show-token`, and `git credential-*` helpers, beside `git credential fill`
    (`hooks.md` 2.11.1).

## P3OPS-17. The launcher cannot tell "no Python 3.11" from "uv cannot run", so the person gets a generic message with no fix

- Severity: minor
- On the slice path: yes (B2 launcher)
- Locations:
  - `hooks.md` 2.2, the launcher skeleton (`*) fail "launch_rc_$rc" "the hook runner could not run (exit $rc)."`).
  - `hooks.md` 2.9, the rows "No Python 3.11 or newer (uv exits 2)" and "uv cannot run".
  - Blueprint 4.1 ("Python and uv") and section 7 (last row).
- Problem. Three causes give uv exit 2, each with a different fix: no interpreter at the floor, a cache that
  cannot be written, and an unknown flag on an old uv. The mapping shown sends all three to one generic line, but
  the table promises `uv python install 3.12` for the first.
- What breaks. On a machine below the Python floor the person is told only "could not run (exit 2)".
- Proposed resolution:
  - On rc 2 only, the launcher runs `"$uv" python find --no-python-downloads '>=3.11'` once and picks the message,
    and also runs `uv --version` against the tested floor.
  - Add a failure test for each case (`hooks.md` 2.2 and 2.14).

## P3OPS-18. A hook timeout is blocked only if `onFailure` covers timeouts, which is not probed, and the scenario-text wall runs on a fail-open event with a one-second deadline

- Severity: minor
- On the slice path: yes (H1)
- Locations:
  - `hooks.md` 2.3 (notes on `onFailure`; the `SubagentStop` row in `notify` mode; the time budget "notify
    elsewhere ... 1 s").
  - `hooks.md` 2.9, the row "Claude Code's own timeout passes".
  - `hooks.md` 2.16, the `onFailure` row ("not relied on for either").
  - `hooks.md` 2.11.2, the final-message check.
  - Blueprint section 10, item 13, which tests only a missing file and the prompt-expansion event.
- Problem. The runner's own deadline covers Python, not the time before it starts: uv start-up and cache-lock
  contention under parallel agents. For a hook that exceeds its timeout, `onFailure` is the only defence, and no
  probe tests it. The final-message check that keeps scenario text from the main session runs on `SubagentStop`.
  That event is in `notify` mode, so a deadline or launch failure there exits 0 and the text passes, logged only.
- What breaks. Under load, a gate can be skipped silently, and the holdout wall to the main session opens without
  a notice.
- Proposed resolution:
  - Add "a gate hook that exceeds its timeout" to item 13. If such a hook does not block, the launcher runs the
    runner under a shell watchdog that ends it at 8 s and exits 2.
  - Give the final-message check the gate deadline (4 s), and announce any `rule.timeout` on it in the next
    briefing.

## P3OPS-19. The driver instruction is shown only while a run is live, so a compaction between two workflows loses the driver

- Severity: minor
- On the slice path: no
- Locations:
  - `state.md` 2.9 ("When `local/driver.json` names this session, the 'Running' line is followed by the
    instruction"; the Running line is shown only when "the run lock is held by a live run").
  - Blueprint section 7 (the main-session compaction row: "If the driver was active or paused in this session,
    the briefing tells the main session to continue it").
  - `pipeline.md` 2.4, the driver paragraph.
- Problem. Automatic compaction tends to fire when a workflow result has just arrived, after its checkpoint
  released the lock. The briefing then has no Running line, and so no driver line.
- What breaks. The compacted main session no longer knows it is driving, and the run stops until the person
  types `/vsdd-factory:drive` again.
- Proposed resolution. Print the driver line whenever `driver.json` names this session, with or without a live
  run: "The driver is active: start `next` with the Workflow tool" (`state.md` 2.9).

## P3OPS-20. The liveness fallback for probe 28 covers only runs with worktrees, and `SessionEnd` on `/clear` may release a live run's lock

- Severity: minor
- On the slice path: no
- Locations:
  - `state.md` 2.11, the liveness bullet.
  - `hooks.md` 2.11.10 (`lifecycle-log`) and 2.3 (the `SessionEnd` row).
  - Blueprint section 10, item 28, and section 7 ("Clearing it is always safe", from `state.md` 2.10).
- Problem. If calls inside steps carry their own session ids, `spec`, `plan`, `ingest` and the `feature` delta
  write no worktree and refresh nothing, and the lock goes stale 20 minutes into a long planner run. `SessionEnd`
  also fires on `/clear`, and the rule releases the lock whatever the reason. If a workflow survives `/clear`, its
  lock is released under it.
- What breaks. A second session can take over a live run.
- Proposed resolution:
  - Also refresh on any `StructuredOutput` whose `step.run` is the locked run and on any `vsdd` call carrying
    `--run <id>`, which is the same change as in P3OPS-01.
  - Probe whether a workflow survives `/clear`. If it does, keep the lock on `SessionEnd` with reason `clear`.

## P3OPS-21. `step-results` may store a result the platform then rejects, and its reason for not validating is out of date

- Severity: minor
- On the slice path: yes (H7)
- Locations:
  - `hooks.md` 2.11.7, last paragraph ("the workflow runtime already does that, and the standard library has no
    schema validator").
  - Blueprint 4.1 (`schema.py`, "a small JSON Schema checker") and 4.10.
  - `state.md` 2.8, the done test.
- Problem. Nothing shows whether the platform validates a `StructuredOutput` call before or after `PreToolUse`.
  If after, a `done` record is stored for a result the script never receives. With P3OPS-08's resume rule, a
  later survey could count that stage as done. The shared core ships a schema checker, so the stated reason not
  to validate no longer holds.
- What breaks. A stage can be counted done on a result that failed its schema.
- Proposed resolution:
  - `step-results` validates the result against the step-result schema, and the step's data schema when the
    identity names it (see P3OPS-05), with `schema.py`, and blocks an invalid result once.
  - Record in the capture set whether the platform's validation runs first.

## P3OPS-22. At the wave and release perimeters no finding can be disputed, so D18's arbiter dismissal never applies there

- Severity: minor
- On the slice path: no
- Locations:
  - `pipeline.md` 2.11.3 `wave.js` `fix()` (every resolution is `'fixed'`).
  - `pipeline.md` 2.7, loop step 5.
  - Blueprint D18 and 5.1.
- Problem. Fix stories always report `fixed`. An implementer who believes a wave finding is wrong has no way to
  dispute it at the wave, so the arbiter can never dismiss it.
- What breaks. Every finding upheld at the wave or release cap goes to a person, against the low-touch intent of
  D18.
- Proposed resolution:
  - A fix story's result carries the keys its fixer disputed (from its fix data), and the wave's `fix()` maps
    those keys to `disputed`.
  - Add a harness scenario.

## P3OPS-23. A clone whose 1.0 state is not mounted is completely silent, so its gates are off with no notice

- Severity: minor
- On the slice path: no
- Locations:
  - `state.md` 2.3, the conditions table (`unmounted`: "Hooks are silent there").
  - `hooks.md` 2.8.
  - Blueprint D24 and section 7 ("A fresh clone").
- Problem. A coworker's clone, or a second machine before `setup`, runs every model action ungated, and nothing
  says the project is a factory project. D24 asks about projects that are not initialised, and makes a one-line
  exception for pre-1.0 state. A project with a 1.0 state branch is initialised elsewhere.
- What breaks. A person may believe the gates are on.
- Proposed resolution. On `SessionStart` only, when `.git/refs/remotes/*/factory-artifacts` exists, or a
  `packed-refs` line names it (shell built-ins, no process), print one line: "factory state is not mounted here;
  gates are off; run `/vsdd-factory:setup`". This extends D24's pre-1.0 exception without reopening D24.

## P3OPS-24. A change request typed while another run holds the lock is refused instead of queued

- Severity: minor
- On the slice path: no
- Locations:
  - `state.md` 2.16, `vsdd survey` (the lock is taken before "it writes the request file ... or leaves the
    request queued").
  - `pipeline.md` 2.9.4, the survey row.
  - Blueprint 3.4 ("A request typed while a cycle is open is queued").
- Problem. In a second session during a wave, `/vsdd-factory:feature "<request>"` reaches the lock before it
  writes the request, and is refused.
- What breaks. The request is lost, and the person must remember to type it again.
- Proposed resolution. When another cycle is active and the request is not critical, the `feature` survey writes
  `requests/CR-NNN.md` and returns `queued` before it tries the lock.

## P3OPS-25. Recursive deletion and forced removal of another worktree are allowed

- Severity: minor
- On the slice path: no
- Locations:
  - `hooks.md` 2.11.1, `bash-destructive` ("except build output and worktree directories"; `git worktree remove
    --force` is blocked only "outside the worktree roots").
  - `hooks.md` 2.7 (devops and verifier hold every command family; see P3OPS-04).
- Problem. Parallel story worktrees sit side by side under `.worktrees/`, and the two exceptions cover them. A
  confused builder that cleans up `../` with `rm -rf`, or runs `git worktree remove --force`, destroys a sibling
  story's or a named spike's uncommitted work.
- What breaks. Work in flight is lost, and nothing records it.
- Proposed resolution. Allow recursive deletion only of build output inside the caller's own worktree. Worktree
  directories are removed only by the tool. Add a case for a sibling worktree.

---

## Checked and found sound

1. `uv` missing: the launcher blocks guarded calls with a fix, `VSDD_FACTORY_HOOKS=off` is read before `uv` is
   looked up, and the down-briefing carries `systemMessage`.
2. Missing plugin files: the inline guard blocks in factory projects and stays inert elsewhere.
3. A `UV_PYTHON` override is beaten by the explicit `--python` flag.
4. The main session compacts while a workflow runs: the workflow continues, `PreCompact` flushes, and the briefing
   is computed from the lock and the run record. The driver-line gap is P3OPS-19.
5. A step agent compacts: the identity and the closing command are repeated at the end of the prompt, the
   summariser gets step-specific instructions, and item 12 is listed as unproven with a fallback.
6. An agent returns nothing: `step()` retries and returns `failed`, `try`/`finally` reaches the checkpoint, and
   universal test 7 holds it.
7. Ungrounded findings are blocked once, then stored as advisory, so one bad citation cannot burn the platform's
   retries.
8. Repeated spike requests are bounded per step, per run (children share the count), per day, by the time box and
   by no nesting. Universal test 5 proves termination. The `open-spike` gap is P3OPS-14.
9. Review loops terminate at cap plus one. Arbitration dismisses only disputed findings that are not critical and
   not security, and a blocked reviewer opens a decision at once.
10. Detached jobs are keyed by command, subject and commit, and re-attach after a crash with no duplicates.
11. Two sessions on one machine: the lock is taken atomically under `flock`, a caller marker per call records the
    right session, flushes are serialised, and record writes are atomic.
12. Decisions only a person can make: `UserPromptExpansion` is proven, agents are refused `decide`, and settling
    fits the deadline.
13. The holdout wall to the main session uses walled keys and checks the final message. The timeout case is
    P3OPS-18.
14. Destructive commands are parsed per segment. Force-pushing the integration branch, `--no-verify`,
    `reset --hard` and `clean -f` are blocked. The gaps are P3OPS-04 and P3OPS-25.
15. Configuration cannot be written by a tool call, and changes to any gate-bearing section, including
    `[[component]]`, are announced.
16. The survey refuses to start without its own caller marker, so no workflow runs with hooks off.
17. Agent starts:
    - A clean child story costs 4 (tests, implement, one review, deliver), 6 with one fix, and 9 to 10 with a
      second lens, a fix round and evidence. Standalone adds 2 clerks.
    - A six-story wave costs about 30 clean and 70 to 90 realistic.
    These match blueprint 10 item 22 and the E2 budget of 12.
18. Flaky inherited tests are re-run once and recorded in closure records.
19. Missing commands are a state: workflows refuse and the route names the fix.
20. A session that ends normally releases its lock at `SessionEnd`.
21. A fresh clone resumes from closure records and adopts the tool's own pushed pull requests. The existing-clone
    gap is P3OPS-13.
22. Story-kind exemptions from tests-first come from one table.

Items examined: 47. Found sound: 22.
