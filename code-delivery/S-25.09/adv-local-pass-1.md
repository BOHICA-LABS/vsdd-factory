---
document_type: adversary-pass-report
level: ops
title: "S-25.09 LOCAL Adversary Cascade — Pass 1 (verbatim)"
producer: adversary
persisted_by: state-manager
timestamp: 2026-10-08T00:00:00Z
phase: per-story-step-4.5-local-adversary-cascade
story: S-25.09
pass: 1
reviewed_head: "feature/S-25.09 @ 5632964d (pre-rebase SHA)"
verdict: NOT CLEAN
finding_count: { total_numbered: 14, high: 2, medium: 7, low: 5, deferred: 0 }
streak_3_clean: "0/3"
novelty: "HIGH"
traces_to: S-25.09
---

> **Provenance.** Relayed verbatim by the orchestrator from the adversary hand-back in session 2026-10-07/08; reviewed HEAD feature/S-25.09 @ 5632964d (pre-rebase SHA; the branch has since been rebased).

# S-25.09 LOCAL Adversary — Pass 1

Worktree identity preflight PASS (.worktrees/S-25.09, HEAD 5632964d; no Bash, so scoped by AC).

Verdict: NOT CLEAN. 2 HIGH, 7 MEDIUM, 5 LOW. Streak 0/3.

F-S2509-L1-001 HIGH — the runbook's identity check (docs/guide/migration-interim-block-recovery.md §2 and "Where the spec and the code differ" item 1) requires txn_id == "txn-" + activation_id; the code since a8ba160f writes the bare id; ADR-052 item 11(e) step 2 says a txn- prefix is a mismatch → stop. Healthy records fail; the operator is taught to accept the rejected form. Cause: the runbook was written at efd482d2, before a8ba160f. Routing: devops-engineer (T-36 deliverable); T-37 verifies. [process-gap] no task re-validates operator docs when the behavior they describe changes.

F-S2509-L1-002 HIGH — runbook step 3 substitutes the derived intent-log path when intent_log_path is null (LOG=${LOG:-$MS/intent-$GEN.log}), and `[ -f "$LOG" ] || { echo …; }` does not stop. ADR-052 item 11(e) step 3: null or absent fails; never substitute. The completed_json_interim_short_circuit never decodes the live record, so the runbook is the only gate and finalizes a record the code and spec call malformed. Routing: devops-engineer.

F-S2509-L1-003 MEDIUM — the item 11(d) sweep missed the drain-timeout gate write: `let _ = write_admission_gate_state(.., Open); return Err(e);`, and the DrainTimeoutAbort Display says "gate returned to OPEN". Routing: implementer + test-writer.

F-S2509-L1-004 MEDIUM — the under-lock completed.json read is `fs.exists(..)` (false on any stat error or dangling symlink), and `let completed_under_lock: Option<CompletedMigrationRecord> = None;` is passed to recover(); ADR item 11(c) requires the hard-coded None removed. Failure: a non-ENOENT stat error or dangling symlink → NoActiveTransaction → a fresh run on an already-migrated tree. Routing: implementer; tests for dangling and ELOOP.

F-S2509-L1-005 MEDIUM — release failures on the release-on-block paths are tracing-only (admission.rs release_reservation_file; callers remove_own_reservation and main.rs `if code == 2`); BC-1.18.013 Postcondition 10(c) and BC-3.08.001 Event 13 require reservation_release_failed. Routing: implementer + test-writer.

F-S2509-L1-006 MEDIUM — coordinator anomalies go only to tracing (finish_committing_migration best-effort gate-OPEN; archive_terminal_txn_record; execute_canonical_path_moves halt reasons); BC-1.18.013 Postcondition 10 coordinator clause and AC-002(f). Routing: implementer (or product-owner to exempt).

F-S2509-L1-007 MEDIUM — AC-019 tests are too weak: fencing_generation is never asserted (a mutant passing 0/1 survives), and the AC-019(c) absolute / ./ / trailing-space path variants are skipped. Routing: test-writer.

F-S2509-L1-008 MEDIUM — the CHANGELOG is inaccurate for S-25.09 (no intent_log_path/activation_id/runbook mention; the header cites only BC-1.18.011 v1.20 EC-043..EC-047; the D-2/D-1/F-012/F-013/F-014 bullets sit under the S-25.08 entry). Routing: implementer or devops (T-42).

F-S2509-L1-009 MEDIUM — AC-006 has no CI source gate (T-13 open). Routing: test-writer.

F-S2509-L1-010 LOW (pending intent verification) — the Event 12/13 detail cap is 256 per part (sanitize_diagnostic(.., 256)) while BC-3.08.001 says 64 for data-derived substrings. Routing: product-owner, then implementer.

F-S2509-L1-011 LOW — the intent-log path is resolved from project_root, not the resolved FactoryRoot (AC-001(b)); the source gate greps only join(".factory". Routing: implementer.

F-S2509-L1-012 LOW — abort_cleanup_outcome `Err(other) => other` drops the original error when the cleanup failure is not Io (ADR item 11(d)). Routing: implementer.

F-S2509-L1-013 LOW — doc comments contradict the code (AdmissionStateIntegrity "Red-Gate STUB"; AdmissionStateIntegrityKind "Red-Gate STUB"; admission_failure_cause "detail goes to tracing::warn!"; main.rs migrate-bc-index "todo!()"). Routing: implementer.

F-S2509-L1-014 LOW — the public read_active_txn_record strict-decodes terminal records (ADR item 3 / BC-1.18.011 6(f)(ii)); no production caller, but it is pub. Routing: implementer.

Checked clean: exhaustive process_exit_code/code_token/admission_failure_cause; the exit-1 class; lock-first contention on every path; foreign refusal before any Tier 1 read; one-live-txn; the version gate first (any magnitude via RawValue); the 12-key decode; nested strictness; the pair check before the GenerationIdWithoutGenDir quarantine; ExpiryAbort only after ABORTED then OPEN; the shared discard primitive; txn_id == activation_id across the txn, CURRENT.json, completed.json and the intent log; NativeGate; BranchCCheck; D-2 resolve_session_project_root; the TOCTOU failpoints tests are non-vacuous; no unwrap/expect/println! in S-25.09 production paths.

Deferred: none. Novelty: HIGH (first fresh-context pass).
