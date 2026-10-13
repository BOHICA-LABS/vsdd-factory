---
document_type: adversary-pass-report
level: ops
title: "S-25.08 LOCAL Adversary Cascade — Pass 13 (verbatim)"
producer: adversary
persisted_by: state-manager
timestamp: 2026-10-13T00:00:00Z
phase: per-story-step-4.5-local-adversary-cascade
story: S-25.08
pass: 13
reviewed_head: "feature/S-25.08 @ 9e936d4c"
verdict: NOT CLEAN
finding_count: { total_numbered: 1, critical: 0, high: 0, medium: 0, low: 1, deferred: 0 }
streak_3_clean: "0/3"
novelty: "LOW"
traces_to: S-25.08
---

> **Provenance.** Relayed verbatim by the orchestrator from the adversary hand-back, session 2026-10-13; reviewed HEAD feature/S-25.08 @ 9e936d4c.

# S-25.08 local adversarial review, pass 13

## Identity and inputs
- **Identity tuple:**
  - worktree `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08`, branch `feature/S-25.08`
  - reviewed HEAD `9e936d4c`, story S-25.08
  - canonical root `/Users/zious/Documents/GITHUB/vsdd-factory`
- **Preflight passed.** The tuple was present, and the worktree basename `S-25.08` matches the story ID. I read code only from the worktree and specs only from canonical `.factory/`.
- **Spec versions:**
  - Story v1.13 (frontmatter `version: "1.13"`, `input-hash: "9440335"`).
  - BC-1.18.013 v1.16 and BC-1.18.011 v1.24, judged only on the clauses the story assigns to S-25.08.
  - ADR-052 v1.29: §5a "Release-leg evaluation order" (lines 1002–1013) and the "Factory-root lookup mapping" release row (line 999).
- **Prior review read:** only `.factory/code-delivery/S-25.08/adv-local-pass-12.md`.
- **Limits:** I cannot run git, cargo, Kani or bats. For tests, Kani, bats, deny, fmt/clippy, the pin sweeps and the 8/8 pre-split range check, I relied on the orchestrator's literal-shell gate evidence.

## Verdict: NOT CLEAN
- **Findings:** 0 CRITICAL, 0 HIGH, 0 MEDIUM, 1 LOW.
- **Novelty: LOW.** The finding is a sibling site of the stale-status class (L5-004, L6-002, L8-004, L10-002, L12-001). It sits in parts of the story that the v1.13 structural fix did not cover, and it affects the story file only. I found no defect in the code, CI, error paths or test strength.
- **Streak after this pass:** 0/3.

## Pass-12 fix verification
**F-S2508-L12-001 is CLOSED.**
- **Convergence paragraph (story line 1662):**
  - It no longer states persistence counts, which review files exist, or the status of any persist task.
  - It keeps the per-pass verdicts, severities, streak, what each fix closed, and "Pass 13 is next, on `9e936d4c`".
  - It names T-42, T-47, T-55..T-63 and `.factory/code-delivery/S-25.08/` as the only record.
- **T-63 (line 1352)** is ticked `[x]`. It covers the pass-12 persist by the D-1261 burst.
- **The older per-pass sections** now point to the task record:
  - Pass 3 (line 1595) and pass 4 (line 1629) say "its persistence is recorded by task T-42 and `.factory/code-delivery/S-25.08/`".
  - Pass 5 (line 1652) says the same with T-47.
- **The code-delivery folder** (canonical glob) holds `adv-local-pass-1.md` .. `adv-local-pass-12.md`, 12 files.
- **Task order:** T-60 (line 1349) now comes before T-61 (line 1350).
- **The eight-SHA correction is accurate.** A grep of the story for the eight pre-rebase SHAs finds:
  - in the body, only `37df1ebd` (T-31, line 1319) and `da52d067`, `ef9a2ec8`, `6c0fb5a4`, `d88e2f55` (T-34, line 1322), which is five;
  - `2f699e60`, `98ef531d` and `efd482d2` only at line 7 (the `last_amended` history) and line 1351 (T-62).
  - This matches the v1.13 correction and T-62's corrected sentence.

## Findings

### F-S2508-L13-001 — LOW (HIGH confidence) — unticked gate task T-6 says T-9/T-10 are "not done" although both are ticked; pass-2 table rows call T-27 "open" and AC-027 "verify" although both are done
- **Anchors in the story `S-25.08-shared-admission-core-b2-conformance-vp147-rebaseline.md`:**
  - **Line 1180, T-6 (`[ ]`, the live pre-merge re-verification gate).** Its reason for staying unticked includes "and the v1.20 implementation T-9/T-10 is not done". In fact T-9 (line 1215) and T-10 (line 1222) are both `[x]`, "TICKED in v1.6 round 2; DONE".
  - **Sibling, line 1560, pass-2 row F-S2508-L2-004.** The Status cell ends "T-27 open". T-27 (line 1307) is `[x]`: sub-item (a) is DONE, and sub-item (b) is carried by T-14.
  - **Sibling, line 1565, pass-2 row F-S2508-L2-009.** The Status cell is "code `0cc54c94`; verify". T-26 (line 1304) is `[x]`, "DONE, verified v1.6", with 5/5 passed.
  - **Minor sibling, line 1265, T-14.** It says "the pass-1 streak is 0/3". The streak is still 0/3 at pass 13, so the label "pass-1" is stale but the value is correct.
- **Failure scenario:**
  - T-6 is a merge-blocking gate. A formal-verifier or pr-manager deciding whether T-6 can be ticked will read its own text. It says the implementation tasks T-9/T-10 are still owed, which would hold back the gate or trigger a needless re-check of delivered work.
  - The story contradicts its own task list. This is the same contradiction the D-1257 rule forbids ("story status must match delivered commits before each adversary pass"), and the same class graded LOW in L5-004, L6-002, L10-002 and L12-001.
  - The v1.13 structural fix removed status prose from the convergence paragraph only. Status prose in other places, such as unticked-task parentheticals and per-pass table Status cells, was not swept (TD-VSDD-060 sibling sweep).
- **Routing:** story-writer.
  - In T-6, drop the "T-9/T-10 is not done" clause. Keep the reasons that are still true: CI is pending, the branch is unmerged, and VP-147 T-11(d)/T-12/T-14 are open.
  - In the L2-004 row, change "T-27 open" to "T-27 done; (b) carried by T-14".
  - In the L2-009 row, change "verify" to "verified, T-26".
  - Optionally, change T-14's "pass-1 streak" to "the streak".

## [process-gap]
- **The stale-status fix targets one paragraph, not the class.**
  - This is the fourth occurrence of the class (L8-004, L10-002, L12-001, L13-001).
  - v1.13 fixed it for the convergence paragraph. Unticked tasks and Status cells still carry free-text status claims about *other* tasks, and those are not re-checked when the referenced task is ticked.
  - **Proposed mechanical check**, run in the same commit as any task tick, with literal shell and captured output:
    - For each ticked T-NN, grep the story for `T-NN (is )?(open|not done|OPEN|owed|pending)` and for Status cells that cite T-NN with "open" or "verify". Zero hits are required.
    - Alternatively, prohibit cross-task status prose in unticked-task parentheticals altogether.
  - **Routing:** orchestrator, then state-manager (codification).

## Fresh sweep: checked, no finding
- **Release leg** (`executor::bc_index_migration_reservation_release`, worktree `crates/factory-dispatcher/src/executor.rs` around 616–685) matches ADR-052 §5a "Release-leg evaluation order" steps 1–5:
  1. `is_tool_completion_event`.
  2. String `tool_use_id` plus `is_valid_tool_use_id`, then a silent return before any filesystem access.
  3. `resolve_factory_root`: `Ok(None)` is silent; `Err` gives one warn and returns.
  4. `probe_release_migration_state_dir` (around 697–705): only `NotFound` gives `Ok(false)`; any other error gives one warn and returns.
  5. `release_bc_index_writer_reservation`: a failure warns, and the dispatch is not affected.
  - The ADR's `migration.admission_advisory` channel is S-25.09's. At S-25.08 it is `tracing::warn!`, which is in scope as documented.
- **Admission** (`bc_index_migration_admission`, around 468–568):
  - Order: event and tool guards, then the `file_path` guard, then `resolve_factory_root` (an `Err` gives `admission_error`, i.e. `E-MAINTENANCE-002`), then the `CLAUDE_PROJECT_DIR` lexical alias, then `resolve_target_path` and `classify_target`, then `classify_tool_use_id`, then `admit_protected_write`.
  - This is the deliberate L4-002 admission order (stat before id) that the ADR keeps (ADR line 1009–1010).
  - `admission_error` puts only the `<cause>` token in the verdict. The detail goes through `sanitize_diagnostic(…, 256)`.
- **`main.rs` wiring** (around 136–196):
  - Admission and release run right after `HookPayload::from_reader` and `resolve_project_cwd`, before `CLAUDE_PLUGIN_ROOT` tiering, `resolve_registry_path` and `Registry::load`.
  - A verdict returns through `exit_code_for_native_gate_verdict` before the registry loads.
  - Release-on-block is driven by `code == 2` through `release_reservation_file`.
  - Registry exit codes go through `registry_error_exit_code` (line 272).
- **Version-pin policy and CI gates:** I relied on the orchestrator's sweeps at `9e936d4c`. The only accepted exception is the "pre-v1.11 code" provenance line. I did not count test-function identifiers, per [D-1257-POLICY19-SCOPE].
- **Out of scope by instruction, not assessed:** S-25.09, S-25.13, S-25.14 and S-25.15 items.

## Observations (non-blocking)
- **T-11(d) and T-12 stay open although the orchestrator's gate reports Kani 10/10 at `9e936d4c`.** This is consistent: the tasks require formal-verifier-recorded evidence, M1–M5 mutant re-runs and a claim boundary in the PR, which an orchestrator gate run does not supply. It is not a defect.

## Deferred (carried, unchanged)
- D-L4-1 → phase-5 (architectural).
- D-L3-1 → wave gate (integration).
- No new deferrals.

## Relevant paths
- `/Users/zious/Documents/GITHUB/vsdd-factory/.factory/stories/S-25.08-shared-admission-core-b2-conformance-vp147-rebaseline.md`: T-6 (1180), T-14 (1263–1266), T-26 (1304), T-27 (1307), L2-004 / L2-009 rows (1560, 1565), T-62 / T-63 (1351–1352), convergence paragraph (1662).
- `/Users/zious/Documents/GITHUB/vsdd-factory/.factory/code-delivery/S-25.08/`: 12 pass files.
- `/Users/zious/Documents/GITHUB/vsdd-factory/.factory/specs/architecture/decisions/ADR-052-native-migration-cli-bash-tool-allowlist-sanctioned-execution-path.md`: lines 999–1013.
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/src/executor.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/src/main.rs`
