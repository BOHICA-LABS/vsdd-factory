---
name: wrap
description: >
  Pause the VSDD factory and make pipeline state durable so the session can be
  cleared and resumed safely. Use when the user says "wrap", "wrap up", "wrap
  the session", or wants to pause the factory / end the session mid-pipeline
  without losing progress. Not for wrapping code or text (use standard
  bash/editor tools for that). Distinct from /compact-state (which slims
  STATE.md content, not the session) and /rehydrate-wave (which resumes after
  a cleared session).
allowed-tools: Bash, Read, Skill, Agent
---

# /vsdd-factory:wrap — Pause Factory & Checkpoint for Session Clear

Bring the running VSDD factory to a safe stop: halt new work, persist everything
in-flight to disk, mark the pipeline PAUSED, write a Session Resume Checkpoint,
release the factory lock, and print resume instructions. After this completes,
the session can be `/clear`ed (or closed) with zero loss — a fresh session
resumes from `.factory/STATE.md` alone.

**Invariant (INV-1):** All STATE.md writes go through the `vsdd-factory:state-manager`
agent (BC-6.23.001 Invariant 5 / BC-6.28.001 INV-1). This skill NEVER edits
STATE.md directly. If state-manager is unavailable at Step 4, the wrap fails with a
clear error; it does NOT fall back to direct STATE.md editing.

**Invariant (INV-2):** The `## Factory Wrapped` report and the `Safe to /clear`
declaration are NOT emitted until all Step 6 durability checks pass completely.
Reporting success ahead of verified durability is a behavioral violation identical
in severity to data loss.

**Invariant (INV-3):** No new pipeline sub-agents (orchestrator, implementer,
test-writer, holdout-evaluator, etc.) are spawned after Step 1. The state-manager
dispatch at Step 4 is the only agent delegation allowed post-halt; it is not a
"pipeline sub-agent" — it is the checkpoint-write mechanism. The factory-unlock
invocation at Step 5 is a skill call, not an agent dispatch, and is permitted.
**Exemption:** The Step 2 health-remediation skills `/vsdd-factory:compact-state`
and `/vsdd-factory:recover-state` are permitted to run as part of Step 2 triage —
they are not pipeline sub-agents barred by INV-3.

## Announce at Start

Before any other action, say verbatim:

> I'm using the wrap skill to pause the factory and make pipeline state durable for a session clear.

## Step 1 — Halt new work

- Do NOT spawn any new pipeline sub-agents from this point (INV-3).
- If sub-agents are currently running, let them finish their current atomic
  step (e.g., a micro-commit or a single test run) but record any that must be
  abandoned mid-step in the checkpoint notes at Step 4 (see PC-8 field (c)).

## Step 2 — Verify factory health

Run `/vsdd-factory:check-state-health`.

- **HEALTHY / WARNINGS:** proceed to Step 3.
- **NEEDS-COMPACT:** run `/vsdd-factory:compact-state` first so the checkpoint
  lands in a slim STATE.md. Verify all postconditions PC-1 through PC-15 against
  the compacted file.
- **STATE.md missing or corrupted:** run `/vsdd-factory:recover-state`, obtain
  human approval of the reconstruction, then continue. Step 7 is NOT emitted
  until PC-1 through PC-15 are satisfied on the reconstructed file.

## Step 3 — Persist uncommitted work

Nothing in-flight may live only in this conversation or in a dirty working tree.

1. **Product repo / story worktrees:** run `git status` in the main repo and any
   active `.worktrees/STORY-*` worktrees. For each with uncommitted changes,
   exactly one of the following two conditions MUST hold:
   - **(a) Committable WIP:** commit to the story's feature branch (NEVER to
     `main`, `develop`, or the default branch) with a message prefixed
     `wip(<STORY-ID>): session wrap checkpoint — <what is half-done>`, then
     push to remote if a remote is configured. Note: the `wip(<STORY-ID>):` prefix
     is the intentional checkpoint-commit convention (not a mistake).
   - **(b) Uncommittable state** (mid-red-gate, merge conflict, build failure):
     do NOT force a broken commit. Document the exact state explicitly in the
     Session Resume Checkpoint at Step 4 (PC-8 field (c)) instead. Step 7
     shows the worktree name and its un-committed status.
2. **Factory worktree:** run `git status` in `.factory/`. Any uncommitted
   artifact files (specs, stories, cycle files) are committed by state-manager
   at Step 4 along with STATE.md.
3. **Push story branches** to remote if one is configured (`git push` per
   branch). Three outcomes are possible:
   - **(a) Push succeeds:** note `pushed: yes` in the Step 7 report.
   - **(b) No remote configured:** note `pushed: no (no remote configured)` in
     the Step 7 report.
   - **(c) Push rejected** (network error or remote reject against an existing
     remote): note `pushed: no (push rejected — <reason>)` in the Step 7
     report. This is distinct from (b): a remote exists but the push failed.
     See Step 7 for the off-machine durability warning and acknowledgement gate.

## Step 4 — Pause + checkpoint via state-manager

**Before delegating:** Read `.factory/STATE.md` (use the Read tool — this is a
read-only operation) and record the current `version:` frontmatter value as
`pre_pause_version`. This value is needed for the PC-6 verification at Step 6.

Delegate to the `vsdd-factory:state-manager` agent (via the `Agent` tool) with
the following instruction set. The wrap skill makes ZERO direct Write or Edit
tool calls on STATE.md at this step or any other step (BC-6.28.001 INV-1;
PC-10). Wait for state-manager to report completion before proceeding to Step 5.

State-manager MUST perform ALL of the following in a single burst
(TD-VSDD-053 single-commit-per-burst):

1. Set `pipeline:` frontmatter to `PAUSED` (PC-1).
2. Refresh `timestamp:` to the pause instant ISO-8601 string (PC-2).
3. Update `phase:` to begin with `PAUSED <YYYY-MM-DD>.` and include the
   pipeline position string (PC-3).
4. Update `current_step:` per the verbatim-strict chain
   (D-441/D-442/D-443/D-444/D-449): `SESSION-WRAP-PAUSE-<YYYY-MM-DD>` step
   description; no meta-commentary, no clause-reordering, no
   justification-suffix injection (PC-4).
5. Update `last_amended:` with today's date, version bump token, `state-manager`
   identity, and a single-line pause description (PC-5).
6. Bump `version:` by exactly one point from the current value; no double-bump
   (PC-6; TD-VSDD-053).
7. Update the banner `wc-l` claim and dual-margin to match the actual
   `wc -l .factory/STATE.md` after the pause commit (PC-7; BC-5.39.005).
8. Archive the prior `## Session Resume Checkpoint` section to
   `cycles/<current_cycle>/session-checkpoints.md` (using the template at
   `${CLAUDE_PLUGIN_ROOT}/templates/session-checkpoints-template.md`) before
   writing the new one (content-routing discipline; PC-8 preamble).
9. Write exactly ONE new `## Session Resume Checkpoint` with all six minimum
   fields (PC-8):
   - (a) date and current pipeline position (phase, step, story/wave, what is next)
   - (b) convergence counter if in an adversarial/convergence loop (streak, pass count)
   - (c) in-flight work (stories mid-TDD, PRs awaiting review/CI, sub-agent steps
         abandoned at Step 1 above)
   - (d) pending human decisions and unresolved blockers raised this session
   - (e) WIP branch list with SHAs (or "none")
   - (f) the exact resume command
10. Append a Phase Progress / Current Phase Steps table row named
    `SESSION-WRAP-PAUSE-<YYYY-MM-DD>` with status `COMPLETE`, agent
    `state-manager`, and a one-line output description (PC-9).
11. Commit with subject prefix `factory(pause):` including the phase and
    one-line position string (single commit; TD-VSDD-053; PC-11).
12. Push to remote (`git push origin factory-artifacts`) if a remote is
    configured (PC-12). Three outcomes:
    - Push succeeds: report `pushed: yes`.
    - Push rejected (network error / remote reject): report
      `pushed: no (push rejected — <reason>)`.
    - No remote configured: report `pushed: no (no remote configured)`.
    In all cases the working-tree clean assertion (PC-12) still applies locally.

## Step 5 — Release the factory lock

If this session holds the cross-session factory lock, run
`/vsdd-factory:factory-unlock` so the resuming session (or another machine)
can acquire it (BC-6.23.001 PC4 `factory.lock.released` event; PC-14).

Skip silently and note `Lock: not held` in the Step 7 report when this session
never held the lock — i.e., when the `factory_lock:` key is absent, the lock is
expired (`now >= expires_at`), or the lock is held by a foreign session (EC-001).
A self-held, unexpired lock that remains present after Step 5 completes is a wrap
failure (PC-14) — re-run `/vsdd-factory:factory-unlock`, which will succeed
because this session is the holder. A foreign or expired lock is NOT a wrap
failure; PC-14 verifies the distinction.

## Step 6 — Verify durability

**Read-only verification only.** All checks in this step use only the Read tool,
`grep`, and `bash` (read-only commands such as `wc -l`, `git log`, `git status`).
Do NOT edit STATE.md or any other file at this step. On any failed check:
re-delegate the specific fix to `vsdd-factory:state-manager` (for STATE.md
failures), or re-run `/vsdd-factory:factory-unlock` (for PC-14 lock failures),
then re-run Step 6 from the beginning. Do NOT proceed to Step 7 while any check
is failing (INV-2; EC-007).

### PC-1 — `pipeline:` equals `PAUSED`

```bash
grep '^pipeline:' .factory/STATE.md
```

Expected: `pipeline: PAUSED`

### PC-2 — `timestamp:` refreshed to today's date

```bash
grep '^timestamp:' .factory/STATE.md
```

Expected: an ISO-8601 string containing today's date (YYYY-MM-DD).

### PC-3 — `phase:` begins with `PAUSED <YYYY-MM-DD>`

```bash
grep '^phase:' .factory/STATE.md
```

Expected: value begins with `PAUSED <YYYY-MM-DD>.` (today's date).

### PC-4 — `current_step:` contains `SESSION-WRAP-PAUSE-<YYYY-MM-DD>`

```bash
grep '^current_step:' .factory/STATE.md
```

Expected: value contains `SESSION-WRAP-PAUSE-<YYYY-MM-DD>`.

### PC-5 — `last_amended:` updated with today's date and `state-manager`

```bash
grep '^last_amended:' .factory/STATE.md
```

Expected: value begins with today's date (YYYY-MM-DD) and includes `state-manager`.

### PC-6 — `version:` bumped by exactly one from pre-pause value

```bash
grep '^version:' .factory/STATE.md
```

Expected: the numeric value equals `pre_pause_version + 1` (captured at the start
of Step 4). Confirm no double-bump (TD-VSDD-053).

### PC-7 — banner `wc-l` claim matches actual line count (HIGH — EC-007)

```bash
wc -l .factory/STATE.md
grep -E 'wc.l|margin from actual' .factory/STATE.md | head -5
```

Expected: the banner comment's claimed line count matches the actual `wc -l`
output. The dual-margin `500 - <actual> = <margin>` must be arithmetically
correct. A mismatch is HIGH severity (BC-5.39.005 banner seal discipline).

### PC-8 — exactly one `## Session Resume Checkpoint`, dated today

```bash
grep -c '## Session Resume Checkpoint' .factory/STATE.md
grep -A 2 '## Session Resume Checkpoint' .factory/STATE.md
```

Expected: count is exactly `1`; the checkpoint content includes today's date
(YYYY-MM-DD).

### PC-9 — Phase Progress row `SESSION-WRAP-PAUSE-<YYYY-MM-DD>` exists

```bash
grep 'SESSION-WRAP-PAUSE' .factory/STATE.md
```

Expected: at least one match; the row shows status `COMPLETE` and agent
`state-manager`.

### PC-10 — STATE.md mutations performed exclusively by state-manager

Satisfied structurally: this skill has made zero direct Write/Edit calls on
STATE.md at any step. No additional grep check required.

### PC-11 — exactly one `factory(pause):` commit; no `backfill`/`Stage N` chain

```bash
git -C .factory log --oneline -3
```

Expected: the most recent commit subject begins with `factory(pause):`. Neither
HEAD nor HEAD^ subject contains `backfill`, `Stage 1`, or `Stage 2`
(TD-VSDD-053).

### PC-12 — factory-artifacts working tree clean

```bash
git -C .factory status --porcelain
```

Expected: empty output (no staged, unstaged, or untracked files).

### PC-13 — product repo and story worktrees committed or documented

```bash
git status --porcelain
# For each active .worktrees/STORY-* directory:
# git -C .worktrees/<STORY-ID> status --porcelain
```

Expected for each: empty output (all changes committed) OR the exact
uncommittable state is explicitly documented in the Session Resume Checkpoint
PC-8 field (c).

### PC-14 — factory lock is FREE (three-state check per BC-6.23.001 PC7)

```bash
# Check factory_lock key presence and sub-fields (empty = key absent):
grep '^factory_lock:' .factory/STATE.md || echo "(factory_lock key absent)"
grep -A 5 '^factory_lock:' .factory/STATE.md | grep -E '^\s*(holder|expires_at):'
# This session's identity:
git config user.email
# Current UTC time (ISO-8601, sorts lexicographically with expires_at):
date -u +%Y-%m-%dT%H:%M:%SZ
```

Interpret using BC-6.23.001 PC7 three-state semantics:

- **LOCK FREE — wrap PROCEEDS** when ANY of:
  - **(a) `factory_lock:` key absent** — factory-unlock removed it at Step 5, or
    no lock was ever acquired by this session.
  - **(b) Lock present but EXPIRED** — `expires_at` value is ≤ current UTC time
    (`now >= expires_at`; the exact boundary `now == expires_at` is treated as
    expired per BC-6.23.001 EC-002).
  - **(c) Lock present but FOREIGN** — `holder` sub-field differs from
    `git config user.email`. This session never held the lock; the correct action
    is to leave it alone — nudging the operator toward `factory-unlock --force`
    would STEAL another session's lock (BC-6.23.001 PC6 `factory.lock.stolen`
    audit event). Do NOT suggest force-unlock for a foreign lock.

- **WRAP FAILURE** only when ALL of: `factory_lock:` key present AND
  `holder == git config user.email` AND `now < expires_at` (self-held, unexpired
  lock still present after Step 5). Recovery: re-run `/vsdd-factory:factory-unlock`
  (succeeds because this session is the holder per BC-6.23.001 PC4), then re-run
  Step 6 from the beginning.

The `Lock:` line in Step 7 reflects the verified outcome, not merely Step 5 having
run:
- `released` — self-held lock was present before Step 5; factory-unlock removed it
  (PC-14 now sees case (a)).
- `not held` — lock was absent, expired, or foreign; Step 5 was silently skipped.

### PC-15 — resume guidance names `rehydrate-wave` before `next-step`

Verify structurally: the Step 7 report you are about to emit follows the template
order — `/vsdd-factory:rehydrate-wave` appears before `/vsdd-factory:next-step`.
A wrap report that lists only `next-step` without `rehydrate-wave` first is a
behavioral violation (BC-6.24.001).

---

Once all checks above pass, proceed to Step 7.

## Step 7 — Report

Only emit this report after Step 6 passes completely (INV-2).

**If anything was NOT pushed to remote** (Step 4 item 12 or Step 3 item 3
reports `pushed: no` for any artifact), you MUST perform the following before
emitting the full report. First determine which artifact(s) are local-only:

- **(i) factory-artifacts NOT pushed** (Step 4 item 12 reports `pushed: no`):
  the Session Resume Checkpoint is local-only — if this machine is lost before a
  push succeeds, the factory cannot resume from STATE.md alone.
- **(ii) WIP story branch NOT pushed** (Step 3 item 3 reports `pushed: no` for
  one or more branches): in-progress code is local-only — WIP commits exist only
  on this machine.
- **(iii) Both** are local-only: both risks apply.

1. Emit the warning line that names what is at risk:
   - factory-artifacts not pushed:
     `⚠ Off-machine durability NOT guaranteed — the factory checkpoint exists only on this machine.`
   - only WIP branch(es) not pushed:
     `⚠ Off-machine durability NOT guaranteed — WIP code on <branch> exists only on this machine; the factory checkpoint DID reach the remote.`
   - both not pushed:
     `⚠ Off-machine durability NOT guaranteed — the factory checkpoint AND WIP code on <branch(es)> exist only on this machine.`
2. Ask the operator explicitly:
   `If this machine is lost before a push succeeds, the local-only artifact(s)
   above will be unrecoverable. Type "acknowledge" to confirm local-only mode and
   proceed.`
3. Wait for the operator to type `acknowledge` (or equivalent explicit
   confirmation).
4. Only after acknowledgement, emit the full "Factory Wrapped" report below,
   including `Safe to /clear or close this session.`

Do NOT emit `Safe to /clear or close this session.` on an un-pushed checkpoint
without explicit operator acknowledgement (BC-6.28.001 EC-006 / INV-2).

```
## Factory Wrapped

Pipeline: PAUSED at <phase> / <current_step>
Checkpoint: .factory/STATE.md (committed <sha>, pushed: yes)
WIP commits: <branch>@<sha>, ... (or: none)
Lock: released (or: not held)

Safe to /clear or close this session.

To resume: open a new session in this project and run
  /vsdd-factory:rehydrate-wave
then
  /vsdd-factory:next-step
Both steps are required. rehydrate-wave injects the correct wave scope;
next-step routes work from the checkpoint.
```

Report notes:
- `<phase>` and `<current_step>` are quoted verbatim from the updated STATE.md
  frontmatter fields (PC-16a).
- `committed <sha>` is the factory-artifacts commit SHA from Step 4 (PC-16b).
- `pushed: yes` or `pushed: no (<reason>)` per actual push outcome (PC-16b).
  When the factory-artifacts checkpoint reports `pushed: no`, OR when any WIP
  story branch push reports `pushed: no`, the `⚠ Off-machine durability NOT
  guaranteed` warning MUST appear (naming the specific local-only artifact per
  case (i)/(ii)/(iii) above) and operator acknowledgement is required before
  `Safe to /clear`.
- `WIP commits:` lists `<branch>@<sha>` for every branch committed at Step 3,
  or `none` if no WIP was in-flight (PC-16c).
- `Lock:` is `released` if factory-unlock ran at Step 5 (self-held lock was
  present and released), or `not held` if Step 5 was skipped silently because
  the lock was absent, expired, or held by a foreign session (PC-16d). This
  value is VERIFIED by the PC-14 three-state check in Step 6 — not merely
  assumed from Step 5 having run.
- `Safe to /clear or close this session.` is the exact wording (PC-16e) — this
  is the human-facing signal that all postconditions have been verified.

## Out of scope

- Does NOT complete or merge in-flight stories — it checkpoints them.
- Does NOT modify spec or story content.
- Does NOT run compaction unless STATE.md health requires it (Step 2 routing).
- Does NOT wrap code or text (use standard bash/editor tools for that).
