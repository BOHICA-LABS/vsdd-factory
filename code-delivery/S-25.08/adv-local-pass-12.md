---
document_type: adversary-pass-report
level: ops
title: "S-25.08 LOCAL Adversary Cascade — Pass 12 (verbatim)"
producer: adversary
persisted_by: state-manager
timestamp: 2026-10-13T00:00:00Z
phase: per-story-step-4.5-local-adversary-cascade
story: S-25.08
pass: 12
reviewed_head: "feature/S-25.08 @ 9e936d4c"
verdict: NOT CLEAN
finding_count: { total_numbered: 1, critical: 0, high: 0, medium: 0, low: 1, deferred: 0 }
streak_3_clean: "0/3"
novelty: "LOW"
traces_to: S-25.08
---

> **Provenance.** Relayed verbatim by the orchestrator from the adversary hand-back, session 2026-10-13; reviewed HEAD feature/S-25.08 @ 9e936d4c.

# S-25.08 local adversarial review, pass 12

## Identity and inputs
- **Identity tuple:**
  - worktree `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08`, branch `feature/S-25.08`
  - reviewed HEAD `9e936d4c`, story S-25.08
  - canonical root `/Users/zious/Documents/GITHUB/vsdd-factory`
- **Preflight passed:**
  - The tuple was present, and the worktree basename `S-25.08` matches the story ID.
  - `.git/worktrees/S-25.08/HEAD` is `ref: refs/heads/feature/S-25.08`.
  - The reflog `.git/worktrees/S-25.08/logs/HEAD` ends at line 121 (`1bdd86fc → 9e936d4c`), so there has been no code change since pass 9's fix.
  - I read code only from the worktree and specs only from canonical `.factory/`.
- **Spec versions:**
  - Story v1.12.
  - BC-1.18.013 v1.16 and BC-1.18.011 v1.24, judged only on the clauses the story assigns to S-25.08.
  - ADR-052 v1.29: I read §5a "Factory-root lookup mapping" (around lines 974–1000) and §5a "Release-leg evaluation order" (around lines 1002–1013).
- **Prior review read:** only `.factory/code-delivery/S-25.08/adv-local-pass-11.md`.
- **Limits:** I cannot run git or cargo. I checked SHA labels against both worktree reflogs. For tests, Kani, bats, deny, fmt/clippy and the pin sweeps I relied on the orchestrator's literal-shell gate evidence.

## Verdict: NOT CLEAN
- **Findings:** 0 CRITICAL, 0 HIGH, 0 MEDIUM, 1 LOW.
- **Novelty: LOW.** The one finding is a stale status statement that contradicts the story's own ticked task. It is the same class as L10-002, and only the story file is affected. I found no defect in the code, CI, CHANGELOG, error paths or test strength.
- **Streak after this pass:** 0/3.

## Pass-11 fix verification
- **F-S2508-L11-001: CLOSED.** I checked each label against `.git/worktrees/S-25.09/logs/HEAD`:
  - **`37df1ebd`** is a fresh S-25.09 commit (line 57). It was re-picked as `a33c5b41` (line 191, same subject). T-31 (line 1319) now reads "pre-rebase `37df1ebd`; on `feature/S-25.09` as `a33c5b41`".
  - **The `1d9f69be` chain:** `ef9a2ec8` (line 95) → `da52d067` (line 149) → `1d9f69be` (line 201).
  - **The `5fe27f59` chain:** `d88e2f55` (line 96) → `6c0fb5a4` (line 150) → `5fe27f59` (line 202).
  - T-34 (line 1322) labels all four intermediate SHAs pre-rebase.
  - All six of these chain SHAs have identical subjects at every step.
  - **The other three of the eight:**
    - `efd482d2` (line 97) → `2f699e60` (line 151) → `fa96531b` (line 203).
    - `98ef531d` (line 173) → `0b8f08a7` (line 224).
    - The v1.12 mapping is correct.
  - **The S-25.09 tip SHAs are current:** no rebase or reset follows the last `rebase (finish)` at line 225 (lines 226–250 are plain commits), so these SHAs are still on `feature/S-25.09`.
  - **Every remaining "pre-split" label is in the v1.5 split list `5091f88f..39e89c59`:** `e7a490cd`, `39e89c59`, `07fccafd`, `0e90e101`, `67fc6437`, `41acb16a`, `f8726c34`, `dbca5754`. That is 8 of 8, matching the orchestrator's gate.
- **Pass-11 cosmetic observation: FIXED.**
  - The convergence paragraph (line 1661) has balanced emphasis: `**Convergence status:**` and `**Pass 12 is next, …**`.
  - The stale heading "Convergence status (v1.8)" is gone.

## Findings

### F-S2508-L12-001 — LOW (HIGH confidence) — the convergence paragraph says T-62 is OPEN and the pass-11 file is unpersisted, but T-62 is ticked DONE and the file exists
- **Anchor:** story `S-25.08-shared-admission-core-b2-conformance-vp147-rebaseline.md`.
  - Line 1661 (the convergence paragraph) says:
    - "its review file is T-62 (OPEN, state-manager)";
    - "Ten review files `adv-local-pass-1.md` .. `adv-local-pass-10.md` are persisted";
    - "`adv-local-pass-11.md` is not persisted until T-62".
  - Line 1350 says "- [x] T-62 … DONE by the D-1260 state-manager burst: `code-delivery/S-25.08/adv-local-pass-11.md` persisted verbatim".
- **Ground truth:** a glob of canonical `.factory/code-delivery/S-25.08/*` returns 11 files, `adv-local-pass-1.md` .. `adv-local-pass-11.md`. So the paragraph's count and T-62 status are both stale at factory-artifacts `f2b6739c`.
- **Failure scenario:**
  - A reader resuming from the story's convergence summary would conclude that the pass-11 record is still owed and the D-1257 rule (the story must be reconciled before each pass) is unmet. They would then re-dispatch a persist, or hold pass 12 back.
  - The story contradicts itself on its own bookkeeping. F-S2508-L10-002 was the same defect class, and it was LOW.
  - The likely cause is that T-62 was ticked when the review file was persisted, and the paragraph written in the same v1.12 edit was not swept.
- **Routing:** story-writer, or the state-manager as part of the T-62 tick.
  - Update line 1661: eleven review files persisted (pass 11 by the D-1260 burst), T-62 DONE.
  - Add a T-63 entry for persisting pass 12.

## [process-gap]
- **Ticking a bookkeeping task does not sweep the convergence paragraph.**
  - This is the third occurrence of the stale-status class: L8-004, L10-002 and now L12-001.
  - The cause is that whoever ticks a "persist pass-N" task (T-55..T-57, T-59, T-61, T-62) does not update the convergence paragraph, which states persistence counts and task status in prose.
  - **Proposed:** in the same commit as the tick, require a literal-shell check that the persisted count (`ls code-delivery/<story>/adv-local-pass-*.md | wc -l`) equals the count stated in the convergence paragraph, and that no `T-NN (OPEN` string refers to a ticked task. Capture the output.
  - **Routing:** orchestrator, then state-manager (codification).

## Fresh sweep: checked, no finding
- **Release leg** (`executor::bc_index_migration_reservation_release`, `executor.rs` around 616–685) matches ADR-052 §5a "Release-leg evaluation order" steps 1–5:
  1. `is_tool_completion_event`.
  2. String id plus `is_valid_tool_use_id`, silent, with no filesystem access.
  3. `resolve_factory_root`: `Ok(None)` is silent; `Err` gives one warn and returns.
  4. `probe_release_migration_state_dir`: only `NotFound` gives `Ok(false)`; any other error gives one warn and returns.
  5. `release_bc_index_writer_reservation`: NotFound is a no-op, any other error warns. It keeps its own grammar guard as defense in depth, as the ADR requires.
- **Admission** (`bc_index_migration_admission`, around 468–568):
  - Order: input guards, then `resolve_factory_root` (an `Err` gives `admission_error`, i.e. `E-MAINTENANCE-002 (io)`), then the `CLAUDE_PROJECT_DIR` lexical alias, then `resolve_target_path` and `classify_target`, then `classify_tool_use_id`, then `admit_protected_write`. This is the deliberate L4-002 admission order the ADR keeps.
  - `admission_error` puts only the `<cause>` token in the verdict. The detail goes through `sanitize_diagnostic` with a 256-character cap.
- **`resolve_factory_root`** (`admission.rs` around 408–438) makes one `metadata` call:
  - a non-directory gives `Ok(None)`;
  - `NotFound` and `NotADirectory` are the closed Absent set;
  - any other error gives `Err(Io { path: <root>/.factory })`.
  - This matches the ADR's Factory-root lookup mapping.
- **`main.rs` wiring** (around 136–196):
  - Admission and release run right after the payload parse and `resolve_project_cwd`, before `CLAUDE_PLUGIN_ROOT` tiering, `resolve_registry_path` and `Registry::load`.
  - A verdict returns before the registry loads.
  - Release-on-block is driven by `code == 2` through `release_reservation_file`, which is idempotent on NotFound and warns otherwise.
  - The `run` → `Err` → exit 0 path, which includes a failure in `resolve_registry_path()?`, keeps the reservation for PostToolUse. That is correct because the tool is admitted.
  - Registry exit codes go through `registry_error_exit_code`.
- **Version-pin policy:** the orchestrator's sweeps at `9e936d4c` cover the diff. The `admission.rs` "pre-v1.11 code" provenance line is the accepted exception. I did not count test-function identifiers, per [D-1257-POLICY19-SCOPE].
- **Out of scope by instruction, not assessed:** S-25.09 and S-25.13 items (path-component predicate, per-slot rendering, manifest `activation_id`, `ACTIVATION_MANIFEST_INVALID`, `ShardConfigError` sanitization).

## Observations (non-blocking)
- **The v1.12 entry and T-62 slightly overstate the scope of the relabel.** The v1.12 `last_amended` entry and T-62 say all eight SHAs were labelled "pre-split" in the body and are now relabelled there. In fact `2f699e60`, `98ef531d` and `efd482d2` were never in the body: they appear only in the v1.7 history text and now in T-62 itself. Every label as stated is true, so there is no provenance error.
- **Task order:** T-62 (line 1350) is listed before T-60 (line 1351). This is cosmetic.

## Deferred (carried, unchanged)
- D-L4-1 → phase-5 (architectural).
- D-L3-1 → wave gate.
- No new deferrals.

## Relevant paths
- `/Users/zious/Documents/GITHUB/vsdd-factory/.factory/stories/S-25.08-shared-admission-core-b2-conformance-vp147-rebaseline.md`: T-31 (1319), T-34 (1322), T-62 (1350), the convergence paragraph (1661), frontmatter `last_amended` v1.12.
- `/Users/zious/Documents/GITHUB/vsdd-factory/.factory/code-delivery/S-25.08/`: 11 pass files.
- `/Users/zious/Documents/GITHUB/vsdd-factory/.git/worktrees/S-25.09/logs/HEAD`: lines 57, 95–97, 149–151, 173, 191, 201–204, 224–225.
- `/Users/zious/Documents/GITHUB/vsdd-factory/.git/worktrees/S-25.08/logs/HEAD`: line 121.
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/src/executor.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/src/shard_manager/admission.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.08/crates/factory-dispatcher/src/main.rs`
