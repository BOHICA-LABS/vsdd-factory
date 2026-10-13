---
document_type: adversary-pass-report
level: ops
title: "S-25.08 LOCAL Adversary Cascade — Pass 14 (verbatim)"
producer: adversary
persisted_by: state-manager
timestamp: 2026-10-13T00:00:00Z
phase: per-story-step-4.5-local-adversary-cascade
story: S-25.08
pass: 14
reviewed_head: "feature/S-25.08 @ 9e936d4c"
verdict: NOT CLEAN
finding_count: { total_numbered: 1, critical: 0, high: 0, medium: 0, low: 1, deferred: 0 }
streak_3_clean: "0/3"
novelty: "LOW"
traces_to: S-25.08
---

> **Provenance.** Relayed verbatim by the orchestrator from the adversary hand-back, session 2026-10-13; reviewed HEAD feature/S-25.08 @ 9e936d4c.

# S-25.08 local adversarial review, pass 14

## Identity and inputs
- **Identity tuple:**
  - worktree `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08`, branch `feature/S-25.08`
  - reviewed HEAD `9e936d4c`, story S-25.08
  - canonical root `/Users/zious/Documents/GITHUB/vsdd-factory`
- **Preflight passed.** The tuple was present and the worktree basename `S-25.08` matches the story ID. I read code only from the worktree and specs only from canonical `.factory/`.
- **Spec versions:**
  - Story v1.14 (frontmatter `version: "1.14"`, `input-hash: "9440335"`).
  - BC-1.18.013 v1.16 and BC-1.18.011 v1.24, judged only on the clauses the story assigns to S-25.08.
  - ADR-052 v1.29: §5a "Factory-root lookup mapping" (lines 974–1000) and "Release-leg evaluation order" (lines 1002–1013).
- **Prior review read:** only `.factory/code-delivery/S-25.08/adv-local-pass-13.md`.
- **Limits:** I cannot run git, cargo, Kani or bats. For tests, Kani, bats, deny, fmt/clippy, the pin sweeps and the 8/8 pre-split check, I relied on the orchestrator's literal-shell gate evidence.

## Verdict: NOT CLEAN
- **Findings:** 0 CRITICAL, 0 HIGH, 0 MEDIUM, 1 LOW.
- **Novelty: LOW.** The finding is in the story file only. It is the same pointer-drift class as L12-001: the v1.14 edit added a persist task but did not update the convergence paragraph's list of persist tasks. I found no defect in the code, CI, error paths or test strength.
- **Streak after this pass:** 0/3.

## Pass-13 fix verification
**F-S2508-L13-001 is CLOSED.**
- **T-6 (line 1180)** no longer says T-9/T-10 are "not done". The tasks it still names (T-11, T-12, T-14) are all unticked (lines 1234, 1243, 1263).
- **L2-004 row (line 1561)** reads "T-27 done; (b) carried by T-14". This matches T-27 `[x]` at line 1307.
- **L2-009 row (line 1566)** reads "verified (T-26)". This matches T-26 `[x]` at line 1304.
- **L3-001 and L3-002 rows (lines 1606–1607)** read "FIXED … T-10 ticked" and "FIXED … T-9 ticked".
- **L3-008 row (line 1613)** points to T-31 with "see T-10; the CHANGELOG obligation … is T-31". It makes no OPEN claim.
- **The pass-3 convergence sentence (line 1621)** reads "Obligations at v1.6 (current status: see the tasks)".
- **T-14 (line 1265)** reads "the streak is 0/3", which is true.
- **T-64 (line 1353)** is ticked. `adv-local-pass-13.md` exists: the canonical glob of `.factory/code-delivery/S-25.08/` finds 13 files, passes 1–13.

**My own class-level sweep:**
- **Unticked tasks.** Only T-6, T-11, T-12, T-14 and the retired T-20..T-25 are unticked. Each status claim they make about another task names a task that really is still unticked.
- **Ticked tasks described as open.** I searched for "remain/still/stays open/tracked by/owed/pending/OPEN" within 60 characters of a T-NN reference. Outside the dated `last_amended` history, the only hits are:
  - line 1176, T-5: "remain in T-11" (T-11 is unticked);
  - line 1234: T-11's own text;
  - line 1307: "tracked by T-14" (T-14 is unticked);
  - line 1327: dated history;
  - line 1533: "10/10 re-run still owed, T-11(d)" (true).
- No ticked task is described as open.

## Findings

### F-S2508-L14-001 — LOW (HIGH confidence) — the convergence paragraph's exclusive persistence pointer stops at T-63 and leaves out the new T-64
- **Anchor:** the story `S-25.08-shared-admission-core-b2-conformance-vp147-rebaseline.md`, line 1663 (the "Convergence status" paragraph). It says: "Which review files exist and whether each was persisted is recorded ONLY by the tasks T-42, T-47 and T-55..T-63 and by the files in `.factory/code-delivery/S-25.08/`".
- **What is wrong:**
  - v1.14 added T-64 (line 1353), the persist record for `adv-local-pass-13.md`, but did not extend the range.
  - The paragraph claims its list is the *only* record, so leaving T-64 out makes the claim wrong.
  - The same paragraph already mentions pass 13 ("Pass 13 then ran … closed by the v1.14 story edit"), so the omission is a missed sibling update in the same edit (TD-VSDD-060).
- **Failure scenario:**
  - A state-manager or reviewer follows the pointer to check pass-13 persistence. Per the paragraph's own "ONLY" wording, they will not find the persist record.
  - Structurally, the v1.13 fix replaced restated status with a closed range that has to be bumped every pass. The range is now a recurrence generator of the L8-004 / L10-002 / L12-001 / L13-001 class: every new persist task makes it stale unless someone edits it by hand.
- **Routing:** story-writer.
  - Change the pointer to an open-ended form that needs no per-pass edit, such as "the state-manager persist tasks T-42, T-47 and T-55 onward". Alternatively, drop the task list and point only at `.factory/code-delivery/S-25.08/` and the Tasks section.
  - Also apply the minor T-6 tidy-up in Observations.

## [process-gap]
- **Closed task ranges in story prose go stale whenever a new task is appended.**
  - This is the fifth occurrence of the stale-pointer/status class on this story.
  - The current class-check, as described in the gate evidence, matches status words only. It does not detect a T-NN..T-MM range whose upper bound is lower than the highest task of the same kind.
  - **Proposed mechanical check**, run in the same commit as any new task: for each `T-NN..T-MM` range in body prose, confirm that MM is at least the highest task number of that kind, or ban closed ranges in convergence or pointer prose.
  - **Routing:** orchestrator, then state-manager (codification).

## Fresh sweep: checked, no finding
- **Release leg** (`executor::bc_index_migration_reservation_release`, worktree `crates/factory-dispatcher/src/executor.rs` lines 616–685) matches ADR-052 §5a "Release-leg evaluation order" steps 1–5:
  1. `is_tool_completion_event` (line 624).
  2. String `tool_use_id` plus `is_valid_tool_use_id`, with a silent return before any filesystem access (lines 630–637).
  3. `resolve_factory_root`: `Ok(None)` is silent; `Err` gives one warn and returns (lines 639–654).
  4. `probe_release_migration_state_dir` (lines 697–705): only `NotFound` gives `Ok(false)`; any other error gives one warn and returns.
  5. `release_bc_index_writer_reservation`: a failure warns and does not affect the dispatch.
  - The ADR's `migration.admission_advisory` channel belongs to S-25.09 and is out of scope here.
- **Admission** (`bc_index_migration_admission`, lines 468–568):
  - Order: event and tool guards, then the `file_path` input guard, then `resolve_factory_root` (an `Err` gives `admission_error`, i.e. `E-MAINTENANCE-002 (io)`), then the `CLAUDE_PROJECT_DIR` lexical alias, then `resolve_target_path` and `classify_target`, then `classify_tool_use_id`, then `admit_protected_write`.
  - This is the deliberate L4-002 order (stat before id) that ADR lines 1009–1010 keep.
  - `admission_error` (lines 574–590) puts only the `<cause>` token in the verdict. The detail goes through `sanitize_diagnostic(…, 256)`.
- **Mapping table, release row (ADR line 999):** "absent → silent no-op" and "unstatable → no verdict" are honored. The S-25.08 warn stands in for the S-25.09 advisory.
- **Version-pin policy and CI gates:** I relied on the orchestrator's sweeps at `9e936d4c` (the only exception is the accepted "pre-v1.11 code" line). I did not count test-function identifiers, per [D-1257-POLICY19-SCOPE].
- **Out of scope by instruction, not assessed:** S-25.09, S-25.13, S-25.14 and S-25.15 items.

## Observations (non-blocking)
- **T-6 wording is imprecise.** At line 1180, T-6 says "the VP-147 sub-items of T-11 … are open". T-11 (line 1234) records (a), (b) and (c) as DONE and only (d) as not evidenced. "T-11(d)" would be exact, and the story-writer can fold this into the L14-001 edit. It is not a separate finding: the claim that T-11 is unfinished is true, and T-11 itself is unambiguous.
- **T-11(d) and T-12 stay open although the orchestrator's gate reports Kani 10/10.** This is consistent: those tasks need formal-verifier-recorded evidence, the M1–M5 mutant re-runs and a claim boundary in the PR, which an orchestrator gate run does not supply.

## Deferred (carried, unchanged)
- D-L4-1 → phase-5 (architectural).
- D-L3-1 → wave-gate (integration).
- No new deferrals.

## Relevant paths
- `/Users/zious/Documents/GITHUB/vsdd-factory/.factory/stories/S-25.08-shared-admission-core-b2-conformance-vp147-rebaseline.md`: convergence paragraph (line 1663), T-6 (1180), T-11 (1234), T-64 (1353).
- `/Users/zious/Documents/GITHUB/vsdd-factory/.factory/code-delivery/S-25.08/`: 13 pass files.
- `/Users/zious/Documents/GITHUB/vsdd-factory/.factory/specs/architecture/decisions/ADR-052-native-migration-cli-bash-tool-allowlist-sanctioned-execution-path.md`: lines 974–1013.
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/src/executor.rs`
