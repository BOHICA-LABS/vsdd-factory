# Review: how the 1.0 design behaves in use

Reviewer lens: operations. Read: `rewrite/STATE.md`, the five designs (`pipeline.md`, `state.md`, `agents-skills.md`,
`hooks.md`, `packaging.md`), the platform facts and the spike plugin. Written 2026-11-23.

## Verdict

The designs get the big operational choices right. The main session stays thin, every workflow works out progress
from disk, gates are measured by code and checked again at the point of no return, and a missing `uv` or a crashing
runner blocks loudly instead of switching the gates off. Walked through in use, though, four things stop the pipeline
outright. Gate and delivery commands run inside one Bash call, which the platform kills after two minutes by default
and ten at most. Approving the brief or the specification rewrites the approved files, so the approval goes stale as
soon as it is recorded. The `decide` command and the driver, which every human stop and the low-touch promise depend
on, are missing from the skill roster. Write-time reference checks deadlock the spec and plan stages. Beyond those:
resume after a crash and two-session locking rely on facts no design supplies (the session id inside a `vsdd` call,
liveness during long steps, verdicts that outlive a run). The wave gate can send the driver round an unbounded loop.
Several paths let a gate go quiet: a configuration file the model can write, a settings `env` switch, a canary no
workflow runs, and web and MCP tools the runner never sees. The secrets rule still covers only `.env` files. A typical
story starts 8 to 12 agents and a six-story wave 70 to 90, about double the pipeline's estimate, and roughly a third of
those starts are clerk relays. None of this needs a different architecture. Each finding below has a local fix.

Severity counts: 4 blockers, 19 majors, 8 minors.

## Findings

### OPS-01 — Gate, wave, release and delivery commands run longer than the Bash tool allows

- **Severity:** blocker
- **Documents:** `pipeline.md` 2.9.1 (red and green gates, `factory story deliver`), 2.9.2 (`factory gate wave` with
  mutation testing), 2.9.6 (`factory gate release`), 2.11.1 (`relay()`), 2.11.2 (the delivery loop); `state.md` 2.12
  (`[commands]`, `delivery.ci_wait_minutes = 20`, `limits.ci_waits = 6`); `packaging.md` 2.10.
- **Problem:** Each gate runs as one synchronous `vsdd gate …` command inside one Bash tool call. The Bash tool defaults
  to a 2-minute timeout and caps a foreground command at 10 minutes. The green gate runs build, full suite, lint and
  format in a fresh worktree, which means a cold build. The wave and release gates add mutation testing and scans.
  `story deliver` rebases, reruns the green gate, pushes, and polls CI for up to `ci_wait_minutes = 20`. The clerk is
  told to "run exactly this command once" and sets no timeout. The CI wait loop in `story.js` calls the clerk again as
  soon as it gets `ci_pending`, because scripts cannot sleep. If `deliver` returns early, the six waits are therefore
  six clerk starts within a minute or two.
- **What breaks:** In any project whose cold build and suite take more than two minutes, the command is killed. That
  covers every Rust workspace the user runs; the old repository's CI took up to 50 minutes. When the command is killed
  it prints no JSON, so no gate result comes back. `gated()` retries once, the step ends `blocked` with kind `gate`,
  and after two runs every story opens `stuck`. Mutation testing never fits inside the limit. Delivery either dies
  mid-poll or spends six clerk starts and stops at `ci_pending`. The platform may also restart an agent it judges
  stalled, which reruns the gate.
- **Resolution:** Run every long command as a detached job with a bounded wait.
  - `vsdd gate <name> --start` forks a detached runner that writes the evidence record and `local/jobs/<id>.json`.
  - `vsdd job wait <id> --max 540` blocks for at most nine minutes and returns `pending` or the result.
  - Steps loop on `wait` within one agent turn, with the Bash timeout set to 600000, up to a configured total.
  - One clerk agent covers the whole CI wait (`vsdd story deliver --wait`), not one agent per poll.
  - Set `stallMs` on gate-running steps above the wait length.
  - Record the Bash ceiling in the blueprint as a platform fact, and add a conformance probe that runs an 11-minute
    gate.

### OPS-02 — Approving the brief or the specification makes the approval stale

- **Severity:** blocker
- **Documents:** `pipeline.md` 2.6 ("If the content changes afterwards the decision reads as stale and the stop
  returns"), 2.9.5 (plan precondition "decision `spec` approved and not stale"); `state.md` 2.4 (`code_values`), 2.6
  (the digest), 2.8 ("When decision `spec` is approved, `settle_decision` sets `status: approved` on every draft
  specification artifact in the digest"); `hooks.md` 2.11.5.
- **Problem:** The digest is SHA-256 over each file's path and content hash. Settling the decision rewrites the
  `status` line of every file in the digest: contracts to `approved`, decision records to `accepted`, verification
  properties to `active`, and the brief. The survey then recomputes the digest, and it no longer matches. Settling also
  runs inside a hook with a 4-second deadline and rewrites every specification file (`prism` has 278 contracts). If it
  stops halfway, some files are already approved and frozen while the decision is not recorded.
- **What breaks:** The next survey reads the approved decision as stale. Plan's precondition fails and the route sends
  the person back to the spec stop. Approving again changes nothing, so the loop repeats. With the brief, `spec` never
  starts. After a partial settle, the retyped decision sees a changed digest and is refused.
- **Resolution:**
  - Compute decision digests over content with the frontmatter `status` value normalised out. Use one function in
    checkpoint, survey and settle.
  - Make settling write the decision record first, then promote statuses idempotently. Let the next `vsdd survey`
    finish any promotion left undone for an approved decision, so the hook's work stays small.
  - Add a fixture test: approve spec, run survey, and check that plan's precondition is met.

### OPS-03 — There is no `decide` command and no driver, and the terminal fallback cannot be reached

- **Severity:** blocker
- **Documents:** `pipeline.md` 2.4 (the driver, need A6), 2.6, section 4 I12; `agents-skills.md` 2.12 (twelve skills,
  `model_visible_skills = 2`), section 4 P-8 and P-9; `hooks.md` 2.11.5 (`UserPromptExpansion` needs a registered
  command), section 5 A4; `packaging.md` 2.10 (`bin/vsdd` is on the Bash tool's `PATH`), 2.6 L-ROSTER; `state.md`
  2.3 (`local/driver.json`), 2.9 ("The driver is active: continue it"), 2.16.
- **Problem:**
  - The roster has twelve skills, and lint L-ROSTER requires the files on disk to match the roster exactly. Neither
    `decide` nor the driver is on it.
  - `UserPromptExpansion` fires only for a registered command, so typing `/vsdd-factory:decide` records nothing.
  - The fallback "a person can run `vsdd decide` in their own terminal" does not work. `bin/` is added to the Bash
    tool's `PATH` inside Claude Code, not to the person's shell.
  - After compaction the briefing tells the model to continue the driver. A driver only a person can invoke cannot be
    restarted by the model; a driver the model can invoke breaks the budget of two model-visible skills.
  - `local/driver.json` is code-only, and no `vsdd` subcommand writes it.
- **What breaks:** The first human stop, `brief`, cannot be settled inside a session, so the pipeline halts there.
  Without the driver, every workflow boundary needs a typed command: about fifteen for a ten-wave cycle, not three.
  This breaks user decision 7 (low touch).
- **Resolution:**
  - Add `decide` as a person-only skill (`disable-model-invocation: true`). Its body only acknowledges, because the
    hook does the work.
  - Add `run` as the driver and make it model-invocable so it survives compaction. Either raise the budget to three,
    or fold the driver into `status`.
  - Add `vsdd driver start|stop`, and have the skill call it.
  - Add both names to P-9 and the reserved-name lint.
  - Make the terminal path real: `setup` offers, with the person's confirmation, to link `vsdd` into `~/.local/bin`,
    and the docs give that command.

### OPS-04 — Write-time reference checks deadlock the spec and plan stages

- **Severity:** blocker
- **Documents:** `state.md` 2.4 (contract registry: `subsystem = "ref:subsystem"`), 2.5 (the architect writes the
  subsystem files; stories carry `depends_on`); `hooks.md` 2.11.3 (`dangling_reference` at `PreToolUse`);
  `pipeline.md` 2.9.3 (stage order domain, contracts, architecture), 2.9.5 (one story-writer per epic, in parallel).
- **Problem:** The `artifacts` rule refuses a write whose reference names an artifact that does not exist yet. Every
  contract must name a subsystem, but the architect writes the subsystem files in the stage after contracts. In
  `plan`, story writers run in parallel by epic, and a story's `depends_on` can name a story in another epic that has
  not been written yet.
- **What breaks:** Every contract write in the contracts stage is blocked, the step returns `blocked`, and `spec`
  cannot finish. Plan's write stage blocks or passes depending on which writer happens to finish first.
- **Resolution:**
  - Check that references exist at gate time (`vsdd gate spec`, `vsdd gate plan`) and at approval. Keep only syntactic
    checks at write time while the artifact is a draft.
  - Or reorder: a short architect "subsystem map" stage before contracts, and a clerk that stamps skeleton files for
    every story in the outline (`vsdd plan stub --run`) before the parallel writers start.
  - Add a scenario test that runs the spec stages in order against the real `artifacts` rule.

### OPS-05 — A wave gate that never converges sends the driver round forever

- **Severity:** major
- **Documents:** `pipeline.md` 2.4 (route table order; the driver stops only on a human `next`, on done, or after two
  runs with `progress: false`), 2.7, 2.9.2, 2.11.3; `state.md` 2.5 (the wave record holds gate rounds).
- **Problem:** At the wave cap the run ends `needs_human` and the checkpoint opens a `finding` decision. The wave is
  still open, though, and the route row "a wave is open → `wave`" comes before "only open decisions remain → human".
  Every run's `converge()` starts again from pass 0, because the survey does not report how many rounds were already
  used. The survey also does not refuse a wave whose gate is waiting on a decision. Each rerun makes and delivers new
  fix stories, so `progress` stays true.
- **What breaks:** The driver reruns `wave` with no end. Each lap costs three or four gate rounds at the reviewer tier,
  plus fix stories at 8 to 12 agents each, until a person notices. The cap and the arbitration step are bypassed across
  runs.
- **Resolution:** Store gate rounds in the wave's closure record. The survey returns `rounds_used`, and `converge()`
  counts the cap across runs. A unit with an open decision is not runnable: the survey marks its stage `waiting`, and
  the route places "all runnable work waits on decisions → human" above "a wave is open".

### OPS-06 — A cold start after a mid-wave crash repeats review and hides partial work

- **Severity:** major
- **Documents:** `pipeline.md` 2.5 (the done test; the earlier-work line appears only when attempt > 1), 2.9.1, 2.11.2
  (`end()` writes a checkpoint only for a standalone run); `state.md` 2.8 (step records count only "within a run that
  has not yet reached its checkpoint"), 2.10 (resume cases); `packaging.md` 2.6 (the story tier kills only at a step
  boundary).
- **Problem:**
  - A story that runs as a child never writes a checkpoint. Its converged verdict exists only as step records under the
    dead run's id. The new run has a new id, and no design says the survey or `vsdd review verdict` reads another run's
    records.
  - Worktrees can hold uncommitted partial work. The attempt counter restarts at 1, so the next agent is never told that
    earlier work may be on disk.
  - Nothing says `story deliver` takes over a PR that is already open.
  - "A story branch that already exists on the remote is reported as owned elsewhere." That includes this machine's own
    branch, pushed just before the crash.
- **What breaks:** Every story that had been reviewed but not delivered is reviewed again from pass 1. By the pipeline's
  own premise, fresh reviewers usually find something new, so more fix passes follow. Agents start over partial work
  they cannot see. A branch this machine pushed can be refused as someone else's. The kill-at-a-step-boundary end-to-end
  test catches none of this.
- **Resolution:**
  - When a pass converges, the next `vsdd` call (`gate demo`, `story deliver`, or the survey) records a verdict
    evidence record (`local/evidence/<story>/review.<sha12>.json`), computed from step records of any run for that unit.
  - The survey returns `dirty` and `open_pr` for each unit. The wrapper adds the earlier-work line whenever the
    worktree is dirty.
  - `story deliver` adopts an existing PR.
  - "Owned elsewhere" means the remote head is not the local branch's head or an ancestor of it, and no local run
    record names the branch.
  - Add end-to-end kills mid-step and mid-CI-wait.

### OPS-07 — The run lock cannot identify its session, goes stale during long steps, and has an unclear scope

- **Severity:** major
- **Documents:** `state.md` 2.11, section 5 H-1 and H-2, section 7 risk 5; `pipeline.md` 2.5 (concurrency), S8;
  `hooks.md` 2.10 (`lifecycle-log`), 4.2. `hooks.md` was written before `state.md` and answers neither H-1 nor H-2.
- **Problem:**
  - `vsdd survey` runs inside a clerk's Bash call, and no design says how a `vsdd` process learns the session id. The
    lock's `session` field, release on `SessionEnd`, and the liveness refresh "for the holder's session" therefore have
    nothing to match on.
  - `seen` is refreshed only on agent start and stop, and on `vsdd` calls that name the run. A single implementer step at
    extra-high effort, or a long gate, can run past 20 minutes with neither.
  - The scope is unclear. The example lock for a wave is scoped `acme-api:develop`. Does a standalone `story` run on a
    story in that wave take the same scope?
  - Taking the lock is a read, check and rename, not an atomic test-and-set.
- **What breaks:** After 20 quiet minutes a second session takes over a live wave, and two implementers end up writing
  in the same `.worktrees/<story>/`. After a normal session end, the next session waits 20 minutes. Two surveys racing
  can both take the lock.
- **Resolution:**
  - The hook runner owns liveness. On guarded calls from a session that holds a lock, it touches `seen` at most once a
    minute, and it releases the lock on `SessionEnd`.
  - The hook supplies the session to every `vsdd` call, either by appending `--session <id>` through `updatedInput`
    (verify with a probe) or by writing it to `local/hooks/`.
  - Scopes: any run that can merge into the integration branch takes `<repo>:<integration>`, plus one lock per story it
    hands out.
  - Take the lock with `fcntl.flock` held around the read, check and write.
  - Optionally, when `state.push = true`, add a cross-machine lease on a ref (`refs/vsdd/lock/<scope>`).

### OPS-08 — A step agent that compacts loses its step identity and its closing command

- **Severity:** major
- **Documents:** `hooks.md` 2.11.9 (instructions for the summariser), 2.3; `pipeline.md` 2.11.1 (`brief()` and the
  identity check in `step()`); `state.md` 2.9 ("A subagent never receives it").
- **Problem:** All five designs treat compaction as a main-session event. A builder or reviewer working at extra-high
  effort on a large story can compact its own context. `compaction-snapshot` gives the same instruction on every
  `PreCompact`: keep the current phase and the next command, because "a fresh briefing will follow". That is false for
  a subagent. Nothing keeps the exact `step` JSON the agent must echo, the worktree path, or
  `Finish by running: vsdd gate green …`. Nobody has verified whether `PreCompact` fires for workflow steps at all.
- **What breaks:** After compaction the agent echoes a wrong or missing identity, so `step()` rejects the result as
  unusable. A fresh agent then restarts the step without the earlier context, and after two such failures the unit
  fails. Or the agent forgets its closing gate, and `gated()` retries.
- **Resolution:**
  - `compaction-snapshot` branches on `agent_type`. For a subagent it tells the summariser to keep the identity block,
    worktree, story id, closing command and finding list word for word, and it drops the "fresh briefing" line.
  - `brief()` repeats the identity and the closing command at the end of the prompt.
  - Allow `autoCompactWindow` in the agent lint.
  - Add a probe that records `PreCompact` inside a workflow step.

### OPS-09 — Failure paths in the scripts throw or leave the lock held

- **Severity:** major
- **Documents:** `pipeline.md` 2.10.1 (`data` is not required), 2.11.1 (`relay()` has no retry), 2.11.2
  (`r.data.reviewed_sha`, `r.data.coverage.some`), 2.11.3 (the `catch` returns `deferred: []`), 2.5; `state.md`
  section 5 P-4; `agents-skills.md` section 7 risk 8.
- **Problem:**
  1. `data` is optional in the schema but always dereferenced. A reviewer that returns `done` without `data` or
     `coverage` raises a TypeError.
  2. No `try`/`finally` guarantees the checkpoint that P-4 requires on failure paths.
  3. One empty clerk reply at survey or close fails the run, because `relay()` never retries.
  4. A child that throws returns `deferred: []`, which drops the findings it was passing up to the wave review.
  5. A reviewer that returns `blocked`, for example a safety refusal during a security review, is flattened to
     `no_result`. The unit is marked `failed`, and a person sees it only as `stuck`, two whole runs later.
- **What breaks:** A standalone run that throws never writes its checkpoint. The lock stays held for 20 minutes, no
  decision opens, and `status.json` goes stale. Reviewer refusals waste two runs before anyone is told.
- **Resolution:**
  - Mark `data`, and every field the script reads, as required.
  - Wrap each workflow body in `try`/`finally` that always runs the checkpoint with `--outcome failed`.
  - Give `relay()` one retry.
  - Keep deferred findings in step records so the parent's checkpoint can read them.
  - Open a `review-blocked` decision the first time a reviewer returns `blocked`.
  - Universal test: every agent returns `{status: "done"}` with no `data`, and the run must still reach its
    checkpoint.

### OPS-10 — Grounding blocks the whole review and can use up the platform's retries

- **Severity:** major
- **Documents:** `hooks.md` 2.11.7, 2.16, section 7 risk 10; `pipeline.md` H2, R1.
- **Problem:** One ungrounded finding blocks the entire `StructuredOutput` call. The platform allows five
  structured-output retries, and model-written quotes and line numbers drift. Paths are "resolved against the step's
  worktree", but no mechanism says how the hook knows that worktree. A step without isolation runs with the project
  root as its working directory, while story paths are relative to `.worktrees/<story>/`. Those paths therefore resolve
  against the main checkout, which does not contain the story's changes.
- **What breaks:** Grounding reports valid findings as not found, and whole reviewer passes are lost. `agent()` returns
  `null`, a fresh reviewer runs and finds different things, and the story ends up `failed`. The reviewer-tier spend on
  the lost passes is wasted.
- **Resolution:** Block at most once per step. On the second attempt, store the result with the ungrounded findings
  moved to advisory, which `converge()` already treats as non-blocking. Resolve paths through `step.subject` to the
  unit's worktree. Allow ±3 lines of drift. Apply the same rule in `vsdd review verdict`.

### OPS-11 — Step prompts leave out inputs that agents are told to block on

- **Severity:** major
- **Documents:** `pipeline.md` 2.5 ("What every step is given"), 2.11.2; `agents-skills.md` 2.6 and 2.7 (Inputs: "If a
  required input is missing, return blocked"; the adversary needs a method, a report path and evidence paths), N-PIPE-3;
  `state.md` 2.5 (`vsdd review path`).
- **Problem:** The pipeline's list of what every step receives has no method name, no report path and no diff-patch
  path. Every agent file says "Your task names a method" and tells the agent to return `blocked` when an input is
  missing. The adversary has no shell, so it cannot run `vsdd review path` to find out where its report goes. The two
  designs also disagree on what a review returns: the agents' role fields differ from the pipeline's `S_REVIEW`.
- **What breaks:** As written, every pipeline step returns `blocked`, so stories become blocked with kind `other` and
  then `stuck`. At best agents invent paths, and the create-only scope refuses their writes.
- **Resolution:** `brief()` passes `method` and `paths`. The method comes from a table in the prelude. The script
  computes the report path from the survey's `cycle` with a fixed pattern. The patch path comes from the green gate's
  evidence and is returned by the survey. Pin the `S_REVIEW` field names in the agents' Result sections. See also
  OPS-28: pipeline reviewers should not write prose reports at all.

### OPS-12 — No design says how workflow agents get permission for their tool calls

- **Severity:** major
- **Documents:** `hooks.md` 2.11.10 (`auto-allow`: a narrow built-in part, with the project allow-list off), section
  6 Q2; `agents-skills.md` N-HOOK-6; `packaging.md` 2.6 layer 4 (`--allowedTools`), P6; platform facts (plugin agents
  ignore `permissionMode`; the only thing that pauses a run is a permission prompt).
- **Problem:** Workflow steps run under the session's permission mode. In the default mode, every build, test,
  `git commit` and edit a builder makes raises a permission prompt. The built-in `auto-allow` covers only a bare
  `vsdd` call made of a single segment. The clerk prompt ("run this command in <dir>") invites `cd <dir> && vsdd …`,
  which is two segments: it raises a prompt and also breaks the clerk's `tool_only` rule.
- **What breaks:** A six-story wave raises dozens to hundreds of prompts across 16 parallel agents, and headless runs
  deny them. The person either sits at the keyboard, which breaks decision 7, or switches to bypass mode, which no
  design states or plans for.
- **Resolution:** Choose the mode in the blueprint. Hooks are the enforcement layer, so `setup` writes a project
  allow-list for the person to approve once: the configured `[commands]`, the git and gh families, `vsdd *`, and edits
  under `.worktrees/**` and `.factory/**`. Give `vsdd` a `-C <dir>` option so calls stay one segment. Add a smoke
  probe: an interactive wave under that allow-list raises no prompts.

### OPS-13 — Parallel stories thrash at delivery and compete for the machine

- **Severity:** major
- **Documents:** `pipeline.md` 2.9.1 (delivery rebases, reruns green if the base moved, and re-reviews if the patch
  changed), 2.9.2, R8; `packaging.md` P1; `state.md` 2.12 (one `commands.run` string).
- **Problem:** Up to 16 stories reach delivery close together, and each merge moves the base for all the others. The
  k-th story to merge can rerun build, full suite and CI up to k times. "The rebase changed the story's own patch" is
  not defined. Checked by commit id, every rebase counts as a change and triggers re-review, and
  `--match-head-commit` ties the merge to the reviewed commit id, which a rebase changes. Meanwhile, 16 worktrees run
  cold builds at once, each with its own Rust target directory. Holdout, demo and end-to-end agents all start the
  product from one `run` command on one port.
- **What breaks:** The wave spends most of its time and CI on re-verification. Deliveries run out of CI waits and
  block. Re-reviews spend reviewer-tier passes. Disks fill, and product starts collide on the port.
- **Resolution:**
  - Add a merge queue in the tool, and tie verdicts to `git patch-id` rather than the commit id. A story whose patch is
    unchanged after a clean rebase keeps its verdict and gets a build-only check. The full suite runs once per batch on
    the integration head.
  - Add a gate semaphore (`limits.parallel_gates`).
  - Add `[commands.env]` for shared build caches.
  - Give each agent that starts the product its own port (`VSDD_PORT`), and stop the product when the step ends.

### OPS-14 — Gates can be switched off without anyone being told

- **Severity:** major
- **Documents:** `state.md` 2.4 (class `config`: writable by the main session; `gate-guard` protects only `[hooks]`),
  2.12; `hooks.md` 2.6 (only changes to `[hooks]` are announced), 2.7 (`[control]` puts `factory.toml` among the
  code-only files, which contradicts `state.md`), 2.9 item 7, 2.11.10, P6; `pipeline.md` 2.5 (the survey runs no
  canary).
- **Problem:**
  - Under `state.md`'s class table, the model can edit `factory.toml`. It can set `commands.test = "true"`, set
    `delivery.merge = "auto"`, empty `protected_paths`, set the holdout and convergence thresholds to 0 or 1, or narrow
    `paths.source`. Only changes to `[hooks]` are fingerprinted and announced.
  - Any tool call can write `env: {VSDD_FACTORY_HOOKS: "off"}` into `.claude/settings.local.json`. `gate-guard` checks
    only `disableAllHooks` and disabling the plugin. `claude plugin disable` run through Bash is not blocked.
  - The canary that detects hooks lost at the platform level is not run by any workflow.
- **What breaks:** The green gate passes on `true`. The next session starts with hooks off, and the only notice is a
  line in the briefing, which goes to the model.
- **Resolution:**
  - Make `factory.toml` code-only for every tool call. `vsdd init` writes it after the person confirms, and people edit
    it in an editor.
  - Fingerprint every section that bears on a gate, and announce changes through `systemMessage`, which the person
    sees.
  - `gate-guard` also blocks settings writes that set `VSDD_*` env keys or change `enabledPlugins`, and the commands
    `claude plugin disable|uninstall` and `claude config`.
  - Move the liveness check into `vsdd survey`. Every `PreToolUse` writes a heartbeat, and the survey refuses to run if
    the clerk's own call left none.

### OPS-15 — `vsdd` subcommands are open to every agent without a prompt, and step records and decisions can be forged

- **Severity:** major
- **Documents:** `hooks.md` 2.7 (`human_only = ["decide"]`), 2.11.1 (a call of the tool "is in no family"), 2.11.7,
  2.11.10; `state.md` 2.16; `packaging.md` 2.10 (`vsdd events emit`); `pipeline.md` 2.2 ("A fabricated result can
  waste a pass. It cannot merge code."), 2.6, R5.
- **Problem:**
  - Any agent with a shell can run these with no prompt: `vsdd lock release --force`, `init --fresh`, `cycle close`,
    `status --fix`, `spike close`, `story deliver`, and `events emit`, which can append any event, including
    `decision.recorded`.
  - `step-results` stores any `StructuredOutput` that carries `step.run`, whoever sends it and whatever step name it
    claims. It stores the record before the platform validates the schema. A builder can therefore emit
    `review-adversary-p3` with no findings, and `vsdd review verdict` counts it.
  - A `python3 -c` that imports `settle_decision` from the plugin directory writes a valid decision.
- **What breaks:** A forged clean review can let `story deliver` see a converged verdict, which contradicts 2.2. A
  model can settle a decision. Destructive state operations need no prompt.
- **Resolution:**
  - Give each agent role in `walls.toml` its own list of allowed `vsdd` subcommands.
  - Add `init --fresh`, `import`, `lock release --force` and `cycle close` to the human-only list.
  - Limit `auto-allow` to read-only subcommands.
  - Accept a step record only for a live run with the same workflow and subject, and only from an agent type the role
    table allows for that step name. Store records at `PostToolUse`.
  - Reserve the event types the tool itself writes.
  - Block Bash commands that name `py/vsdd`, `cli.py`, `hook.py` or `launch.sh` other than through `bin/vsdd`.

### OPS-16 — The secrets rule covers `.env` files only, and the runner never sees web or MCP tools

- **Severity:** major
- **Documents:** `hooks.md` 2.11.1 (`secrets`), 2.3 (the matcher leaves out `WebFetch`, `WebSearch` and MCP tools);
  `agents-skills.md` 2.5 (web tools on `researcher`, `spike-runner`, `product-owner`, `architect`,
  `security-reviewer`), section 7 risk 7; `packaging.md` 2.1 (every project has a `.mcp.json`; the repository root's
  holds live keys).
- **Problem:**
  - Files not covered: `.mcp.json`, `~/.aws/credentials`, `~/.ssh/*`, `~/.netrc`, `~/.docker/config.json`,
    `~/.config/gh/hosts.yml`, `~/.npmrc`, `*.pem`, `*.key`, `~/.claude/.credentials.json`.
  - Commands not covered: `gh auth token`, `git credential fill`, a bare `env`, `printenv` or `export -p`, and
    `/proc/*/environ`.
  - No read wall covers paths outside the project.
  - Ways out of the machine: `WebFetch` and MCP tools never reach the runner, and `curl -d` is allowed.
  - `researcher` and `spike-runner` read untrusted pages.
- **What breaks:** A page with injected instructions has the researcher read `.mcp.json` and send its contents out in a
  `WebFetch` URL. Nothing blocks the call and nothing logs it.
- **Resolution:**
  - Add the files above to the default secret globs, and block the listed credential commands and environment dumps.
  - Add an `outside-project` read class, denied to every agent role. It does not apply to the main session, and the
    plugin root and `tmp` stay readable.
  - Route `WebFetch` and the plugin's MCP tools through `PreToolUse` for the roles that have them. Block URLs that carry
    long high-entropy tokens, and log every call.
  - Block `curl` and `wget` upload flags for agent roles.

### OPS-17 — Browser tools read walled files through `file:` URLs (N-HOOK-9 is not met)

- **Severity:** major
- **Documents:** `agents-skills.md` 2.5 (browser tools on `demo-recorder`, `holdout-evaluator` and `ui-reviewer`),
  N-HOOK-9; `hooks.md` 2.3, 4.2 (which answers N-HOOK-1 to 8 but not 9), A2.
- **Problem:** `browser_navigate` to `file:///…/.factory/holdout/HS-001.md` from `demo-recorder`, or to
  `file:///…/src/…` from `holdout-evaluator`, puts a walled file into the agent's context. The runner never sees MCP
  tools. `hooks.md` A2 asks for a lint that walled agents hold only tools the runner can inspect. The tool lists in
  `agents-skills.md` fail that lint, or the lint is written to accept browser tools and the bypass stays silent.
- **What breaks:** The holdout wall leaks in both directions.
- **Resolution:** Add the Playwright tool names, in both spellings, to the matcher. Have `read-walls` treat a `file:`
  URL as a read of the resolved path, and block `browser_file_upload` from walled paths. Start the bundled server with
  `file://` blocked. Make A2 concrete for MCP tools.

### OPS-18 — The red gate cannot be computed from one test command, and tests written inside source files break the split

- **Severity:** major
- **Documents:** `pipeline.md` 2.9.1 (the red gate's definition: tests fail on assertions, not on build errors, and at
  least half are red; frozen test hashes), 2.2; `state.md` 2.12 (`commands.test` is one shell string; `paths.tests`
  defaults); `hooks.md` 2.11.4.
- **Problem:** "New tests", "fails on an assertion, not a build error" and "half are red" all need results per test.
  `commands.test` gives an exit code and free text in each toolchain's own format. Separately, idiomatic Rust unit
  tests live inside source files (`#[cfg(test)] mod tests` in `src/lib.rs`). Such a file counts as `source`. If it is
  frozen, the implementer cannot implement in it. If it is not frozen, the implementer can weaken those tests.
- **What breaks:** A red gate built on exit codes passes a compile error. In the user's Rust projects, stories either
  deadlock or lose the frozen-test protection without anyone noticing.
- **Resolution:**
  - Add `[commands] test_report = "junit:<path>"`, which `vsdd init` detects: nextest, `pytest --junitxml`,
    `jest-junit`, `go-junit-report`.
  - Red gate: the build passes, the new tests are the test ids missing from the base commit's report, and the gate
    counts their failures. Freeze tests by test id.
  - Default Rust test globs `**/tests/**` and `**/tests.rs`. Put unit tests in a `tests.rs` submodule. The gate refuses
    to count tests defined in `source` files.

### OPS-19 — Every agent loads the old project `CLAUDE.md`

- **Severity:** major
- **Documents:** `agents-skills.md` 2.7 (`omitClaudeMd` only on `holdout-evaluator` and `researcher`); `packaging.md`
  2.1 (`prism` and `pregolya` each have about 600 lines of old process rules), 2.9, section 7 risk 7; `state.md` 2.5
  (`conventions.md`, given to builders).
- **Problem:** Sixteen of the eighteen agent types load the project's `CLAUDE.md` every time they start. In the two
  largest projects it tells them to route work to state-manager, never edit `STATE.md`, and run 3-clean convergence and
  attestation gates. It also adds roughly 8,000 to 12,000 tokens to every start.
- **What breaks:** Agents act on rules that no longer exist, and at 70 to 90 starts a wave the file alone adds close to
  a million input tokens per wave. The cutover only reports the problem.
- **Resolution:** Set `omitClaudeMd: true` on every agent. Project conventions reach agents through
  `conventions.md`, and toolchain rules (for example the user's `fnm` Node policy) through `[commands]`. `setup` moves
  the build and convention content into `conventions.md` after the person confirms, and offers to archive the old
  process sections. Verify V7.

### OPS-20 — Unattended maintenance can merge dependency changes without a person

- **Severity:** major (a needed human stop is missing)
- **Documents:** `pipeline.md` 2.9.10 (the dependency sweep produces fix stories that go through `story`), 2.6, section
  6 Q1 (recommends taking lock files and manifests off the protected list); `state.md` 2.12; `packaging.md` P6
  (headless schedule).
- **Problem:** With the recommended protected list, a scheduled headless `maintain` run writes dependency-upgrade
  stories and merges them on AI review alone.
- **What breaks:** New third-party code reaches the integration branch with no person involved. That is a supply-chain
  change.
- **Resolution:** If a diff adds a dependency or upgrades one beyond the range the approved architecture lists, a
  `merge` decision is needed. The tool detects this from the manifests. In `maintain`, every such change waits in one
  batched decision per run.

### OPS-21 — States that wait on a person do not become decisions, and greenfield hits one at plan

- **Severity:** major
- **Documents:** `pipeline.md` 2.6 (how blocked steps map to decisions), 2.9.1 (`awaiting_merge`), 2.9.2 (`--settle`),
  2.9.5 (readiness: the devops agent writes CI files), 2.11.1 (clerk results carry no step identity, so H1 stores
  nothing); `state.md` 2.7 and 2.9.
- **Problem:**
  - `awaiting_merge` returns `needs_human`, but no rule maps it to a `merge` decision. Clerk results are not stored as
    step records, so the checkpoint cannot see the state unless `deliver` records it.
  - In plan's readiness stage, the devops agent writes CI files with no story and no worktree. It either writes into
    the person's checkout or is stopped by `story_only`. `.github/**` is a protected path, so a person must merge it.
  - In the last wave, `--settle` has nowhere to move blocked stories, so the run returns `partial` with
    `next: wave`.
- **What breaks:** Nobody is told that a PR is waiting. The briefing shows no "Waiting on you" line. The driver stops
  without saying why, and greenfield stalls at plan.
- **Resolution:** `deliver` records the delivery state in the closure record, and the checkpoint opens a `merge`
  decision with the PR link. Readiness work becomes a chore story in `E-0`. When everything left waits on decisions,
  the run returns `needs_human` and the route says which decisions. `--settle` may create a trailing wave.

### OPS-22 — When `uv` or Python is missing, the printed fix cannot run, and the Bash sandbox is not considered

- **Severity:** major
- **Documents:** `hooks.md` 2.2 (`--offline --no-python-downloads`; the message "Run: vsdd doctor"), 2.9;
  `packaging.md` 2.4 (hosts: `uv` may download an interpreter), I3, 2.10, section 7 risk 10; `rewrite/STATE.md` (the
  TLS failure in a sandboxed shell).
- **Problem:**
  - With `uv` missing, every guarded tool is blocked. The message points to `vsdd doctor`, which needs `uv` and is a
    Bash call the gate blocks.
  - On a Mac with only the system Python 3.9, the hook path refuses to download a newer Python, but the CLI path may
    download one: two different policies.
  - No design specifies or tests what `vsdd` needs when it runs inside Claude Code's Bash sandbox: write access to the
    `uv` cache and to `~/.cache/vsdd-factory`, and network for `git` and `gh`.
- **What breaks:** On first run the model can neither read files nor help, and the person is given a command that
  cannot work. In a sandboxed session the hooks work, because they run outside the sandbox, but every clerk's `vsdd`
  call fails, so every workflow fails at its survey.
- **Resolution:** The block message gives commands the person runs in their own terminal (install `uv`;
  `uv python install 3.12`) and names the relaunch switch. `vsdd init` installs a managed Python once. Use one download
  policy for both entries. Specify what the sandbox must allow, have `vsdd doctor` run inside a Bash tool call, and
  include `systemMessage` in the briefing shown when gates are down.

### OPS-23 — Some reviews can never converge, and the arbiter can dismiss findings nobody disputed

- **Severity:** major
- **Documents:** `pipeline.md` 2.7 (a clean pass requires full coverage; "A `not_defect` ruling … needs the builder's
  dispute first"), 2.11.1 (`converge()`), 2.6 (`waive <keys>`), 2.11.2 (`must = u.acs`).
- **Problem:**
  - Reviewers may mark an acceptance criterion `skipped` when a diff cannot show it: a performance criterion, a
    user-visible criterion, or one an earlier story already met. With no open findings, `converge()` never calls the
    fixer and stops before arbitration. Passes 2 and 3 are fresh reviews hoping for a different answer, and the result
    is `blocked`/`cap` with no finding key a person could waive.
  - The code lets the arbiter dismiss any open high finding, including one raised on the cap pass that no builder ever
    saw. That contradicts the prose rule quoted above.
  - The `direct` choice promises "one more round", but the next run gets a full cap.
- **What breaks:** Reviewer-tier passes are wasted, and the person gets a decision whose only real choice is to
  descope. A single agent can dismiss a high finding, and the tool will copy the same behaviour through the P4
  fixtures.
- **Resolution:**
  - Turn coverage gaps into findings of category `coverage`, routed to the test-writer. Or allow a `not_checkable`
    state, with a reason, when another evidence route exists, and offer `waive-coverage <item>`.
  - Let `not_defect` dismiss only findings the builder disputed.
  - `direct` sets the cap to 1 for the next run.

### OPS-24 — A bug in a locked rule can only be escaped by turning every rule off

- **Severity:** minor
- **Documents:** `hooks.md` 2.6, 2.9, section 6 Q3, section 7 risk 3.
- **Problem:** The locked rules cannot be relaxed: `read-walls`, `write-scopes`, `gate-guard`, `decisions`,
  `step-results` and `session-briefing`. If one crashes, every call it covers is blocked. For `step-results`, that is
  every workflow step.
- **What breaks:** The only way out, `VSDD_FACTORY_HOOKS=off`, also switches off the secrets, destructive-command and
  git guards for that session.
- **Resolution:** Add a person-only `VSDD_FACTORY_HOOKS_QUARANTINE=<rule>`. It skips that one rule only after it has
  failed with repeated `rule_error`, and it is announced in every briefing.

### OPS-25 — The pipeline's agent-count estimate is about half the realistic figure, and a third of starts are relays

- **Severity:** minor
- **Documents:** `pipeline.md` 2.11.3 (counts from the simulation), R12, 2.1 (the clerk tier); `agents-skills.md` 2.3,
  2.12, N-PIPE-2 (mechanical steps go to `devops` at extra-high effort, and there is no clerk), V4; `packaging.md` 2.6
  (the story-tier budget is 1.5 times the pipeline's figure), P7.
- **Problem:** The pipeline says a typical story starts 4 to 6 agents. Under its own premise that a fresh reviewer
  nearly always finds something, the count is higher:

  | Unit | Starts |
  |---|---|
  | Story, run as a child: tests 1, implement 1, review passes 2, fix 1, deliver plus two CI polls 3 | 8 |
  | Same story in a critical or high module (security lens on each pass) | 10 |
  | Add when fixes touch both tests and code, when there are user-visible criteria, or when run standalone | +1, +1, +2 |
  | Wave of six stories, one integration finding: survey 1, stories 48–72, two gate rounds 10, new-fix clerk plus one fix story 9, close 1 | 69–93 |

  Clerk starts are about 26 of those per wave, roughly 30%. At the measured average of about $0.15 a start, the fixed
  cost is $10–14 per wave before any work is done, and more on the reviewer and planner tiers. A 60-story greenfield
  run comes to roughly 800 to 1,000 starts. A verify round with 100 scenarios at 80% rotation adds 16 evaluator starts.
  The two designs also disagree on tiers. The pipeline's clerk runs Sonnet at low effort, while `agents-skills.md` has
  no clerk and routes mechanical steps to `devops` at extra-high effort. The pipeline runs experiment spikes on the
  builder tier, while `agents-skills.md` puts `spike-runner` on the planner tier. Probe V4 is unverified: the user has
  many other plugins enabled (this reviewer's own context lists well over a hundred skills from them), so whether agents
  without the Skill tool still receive those listings decides much of the fixed cost.
- **What breaks:** The end-to-end budget assertion (P7) is set from an estimate that is too low, and the spend at the
  user's chosen tiers is underplanned.
- **Resolution:**
  - Have one clerk serve the whole CI wait (OPS-01).
  - Merge the `integration-r<k>` and `new-fix-r<k>` clerks.
  - Use holdout batches of 10 to 15 scenarios.
  - Apply OPS-19.
  - Settle the clerk and spike-runner tiers.
  - Measure V4 in the user's real environment. If listings do reach those agents, document running pipeline sessions
    with only this plugin enabled.
  - Set the budget from measured counts.

### OPS-26 — `STORY_WORST = 30` is not a real bound, and each child is given 900 agents

- **Severity:** minor
- **Documents:** `pipeline.md` 2.11.3 (`STORY_WORST`, `room`, `limits: RUN.limits`), 2.5.
- **Problem:** Counted from the loop bounds, one story can start well over 100 agents: attempts × spikes × four passes
  × two lenses, plus three delivery rounds that each run a repair and a re-review. Children receive `agents: 900`, so
  their own counter never stops them.
- **What breaks:** The `room` calculation does not keep a wave under the platform's limit of 1,000 agents per run.
  Runaway children hit that hard stop mid-wave, and every unit returns `null`.
- **Resolution:** Pass each child a share of the budget (`limits.agents = STORY_WORST`), have `step()` return
  `agent_budget` when it is reached, and derive `STORY_WORST` from the loop bounds in a unit test.

### OPS-27 — Spike accounting has holes

- **Severity:** minor
- **Documents:** `pipeline.md` 2.8 (limits; "the same question"), 2.11.1 (each `gated()` attempt starts a fresh
  `step()` with `spikes = []`; research spikes run without isolation); `agents-skills.md` 2.11 (isolation required;
  `spike-runner` on the planner tier); `hooks.md` 2.11.6 (ledger entries matched by worktree); `state.md` 2.12
  (`max_per_session` and `max_per_requester`, which `hooks.md` never defines).
- **Problem:**
  - A finding from the first attempt is not passed to the second, so the retry asks the same question again.
  - Nothing defines when two questions are "the same", and plain text equality misses rewordings.
  - Limits are checked inside the runner, so a refused spike still costs an agent start. Children's counters start at
    0.
  - Research spikes have no worktree of their own, so they collide on the ledger key.
- **What breaks:** Spikes repeat, and refused spikes are paid for.
- **Resolution:** Carry findings across attempts. The survey returns the run's `spikes_used`, and the wrapper refuses
  before it starts an agent. Deduplicate by question hash plus unit, and have `spike open` return similar earlier
  findings. Isolate every spike. Implement the two limit keys or drop them.

### OPS-28 — Some bookkeeping is creeping back

- **Severity:** minor
- **Documents:** `state.md` 2.5 (review report files, `vsdd review path`, pruning when a cycle closes; lessons capped at
  30 adopted, with `vsdd validate` failing at 31; deferred items capped at 40), 2.3 (what triggers a flush);
  `agents-skills.md` 2.6.
- **Problem:**
  - Pipeline reviewers write a prose report besides the structured result the pipeline actually uses. Nothing in the
    pipeline reads the report, yet it needs path allocation, a size cap and pruning, and costs reviewer-tier output
    tokens on every pass.
  - The caps on lessons and deferred items fail `vsdd validate`, and no stage folds lessons into conventions.
  - Flushing on every `vsdd` call, every `SubagentStop` and every `SessionEnd` makes hundreds of state-branch commits
    per wave.
- **What breaks:** If `validate` runs inside a gate, the 31st lesson blocks a release: governance files blocking
  delivery, the old failure mode. The commit ratio of the old system comes back, though at no cost in agent effort.
- **Resolution:** Pipeline reviews write no prose; `vsdd review show` renders the findings for people, and only the
  `review` skill writes a report. Register limits are reported as status problems, never as gate failures, and the
  retrospective step refuses to add lessons past the cap. Flush only at checkpoint, decide, deliver and pre-compaction.

### OPS-29 — `status.json` conflicts on every push from two machines

- **Severity:** minor
- **Documents:** `state.md` 2.3 (push: rebase and retry, and a conflict stops the push), 2.7 (rewritten at every
  checkpoint; can be rebuilt), 2.11.
- **Problem:** Both machines rewrite `status.json` at every checkpoint, so every concurrent push conflicts on it. The
  conflict is reported to a person even though the file can be rebuilt.
- **What breaks:** Using two machines always ends with a person resolving a file the tool could rebuild itself.
- **Resolution:** Move `notes` into a tracked file of its own. Then either keep `status.json` local, or regenerate it
  during the rebase instead of reporting a conflict.

### OPS-30 — Direct commits on the integration branch get past the merge guard

- **Severity:** minor
- **Documents:** `hooks.md` 2.11.4 (`merge-guard` covers merge, rebase, cherry-pick and push), 2.7 (the main session's
  write scope includes `project`); `pipeline.md` 2.9.1 (`delivery.mode = local`).
- **Problem:** A `git commit` while the integration branch is checked out in the main checkout is not blocked. In local
  delivery mode, that branch is the product branch.
- **What breaks:** Product changes land without the tests-first rule, review or evidence.
- **Resolution:** Block `git commit`, `--amend` and any `reset` that moves the branch while the integration or release
  branch is checked out. For the main session, the block message names `/vsdd-factory:feature`. A person can relax the
  rule.

### OPS-31 — The default commit attribution collides with `bash-git`

- **Severity:** minor
- **Documents:** `hooks.md` 2.11.1 (`bash-git` blocks attribution); `agents-skills.md` 2.7.
- **Problem:** Claude Code tells the model to add attribution trailers whenever its settings supply them, so each
  builder's first commit in each step is blocked and has to be retried.
- **What breaks:** One wasted turn per committing step across the whole cycle.
- **Resolution:** `setup` writes the project setting that turns off commit and pull-request attribution, and
  `vsdd doctor` checks that it is set.

## Areas checked and found sound

- One-level nesting, with spikes as steps inside children, matches what the platform spike proved.
- The main session stays thin: results are under 2 KB, every workflow surveys from disk, and the briefing on startup,
  resume and compaction is proven.
- The launcher turns every unexpected exit on a gate event into exit 2, with `onFailure: "block"` as a backstop. A
  missing `uv` cannot silently switch gates off (the message problem is OPS-22).
- `hooks.json` is generated from the rule table, timeouts are in seconds, and tests run in both directions.
- The plugin is inert in projects that are not initialised and in pre-1.0 projects, which makes the shared rc.25 cache
  risk safe.
- Validation runs before the write on the content the file would have. Size caps are checked before the write. No
  index, count or version is stored.
- Evidence is written by code from real exit codes and checked again at merge (apart from OPS-15 and OPS-30).
- Capturing decisions on `UserPromptExpansion` rather than `UserPromptSubmit` is right, and so is binding approvals to a
  digest once OPS-02 is fixed.
- Each of the three routine stops (brief, spec, release) is a real decision. Leaving plan approval off by default and
  making ideas and merge conditional are right. Queued exceptional decisions let other units continue.
- The story-level cap, arbitration and `finding` decision end the loop. Stories waiting on a decision are left out of
  the next wave run.
- Releasing by immutable tag, with the marketplace pull request as the single switch and `install-check` behind it, is
  sound.
- The hook latency budget (8 ms when inert, about 60 ms per gate call) is acceptable with 16 agents running at once.
