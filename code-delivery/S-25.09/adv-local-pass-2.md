---
document_type: adversary-pass-report
level: ops
title: "S-25.09 LOCAL Adversary Cascade — Pass 2 (verbatim)"
producer: adversary
persisted_by: state-manager
timestamp: 2026-10-08T00:00:00Z
phase: per-story-step-4.5-local-adversary-cascade
story: S-25.09
pass: 2
reviewed_head: "feature/S-25.09 @ f5e603b3"
verdict: NOT CLEAN
finding_count: { total_numbered: 9, high: 0, medium: 5, low: 4, deferred: 0 }
streak_3_clean: "0/3"
novelty: "MEDIUM"
traces_to: S-25.09
---

> **Provenance.** Relayed verbatim by the orchestrator from the adversary hand-back, session 2026-10-07/08; reviewed HEAD feature/S-25.09 @ f5e603b3.

# S-25.09 LOCAL Adversary — Pass 2

Identity preflight passed: (.worktrees/S-25.09, f5e603b3, S-25.09, /Users/zious/Documents/GITHUB/vsdd-factory). Specs and ADR read from canonical .factory/. No shell; delta scoped by ACs and by comparing against .worktrees/S-25.08/CHANGELOG.md.

Verdict: NOT CLEAN. 0 HIGH, 5 MEDIUM, 4 LOW. Streak stays 0/3.

Pass-1 fix verification — fixed and load-bearing: F-001/F-002 (runbook §2 bare txn_id, fails on a txn- prefix; §3a recorded path only, stops on mismatch); F-003 (the drain-timeout reopen via abort_cleanup_outcome); F-004 (read_completed_marker lstat-first, four verdicts; read_completed_under_lock after the flock and before Tier 0; recover() gets completed_under_lock.as_ref(); detect_migration_read_state uses the same helper); F-005 (both release-on-block paths return reservation_release_failed); F-007 (fencing 3, every INTENT/DONE; absolute/./trailing-space variants); F-009 (source gate with controls); F-011 (recorded_intent_log_path(&factory_root, ..) in resume, fresh run and finish_committing_migration); F-012 (the non-Io arm names the original token); F-013 (no stale doc comments); F-014 (terminal records via Tier 0 projection); release-leg Event 13 on a migration-state stat error with an ENOENT control. Partly fixed: F-006 (L2-002/003/006), F-008 (L2-005), F-010/11(g) (L2-001/008). Known residual confirmed: runbook line 363 still says "at 3c62c64e"; the header cites 0b8f08a7, not f5e603b3.

F-S2509-L2-001 — MEDIUM (confidence HIGH) — BcIndexMigrationError::Io Display `#[error("BC-INDEX migration: I/O error at {path}: {source}")]`, printed by run_migrate_bc_index_cli via eprintln!, applies no sanitize_diagnostic. Breaks ADR-052 item 11(g)(2) and AC-022(d)(1); the CHANGELOG line 50 claims the same. Scenario 1: a directory named `txn-<ESC>[2J….json` in migration-state → read_txn_files EISDIR → Io { path } → the raw escape reaches the terminal (a malformed file of the same name would be sanitized via AdmissionStateIntegrity). Scenario 2: the V4 completed.json verdict wraps a serde message in InvalidData that includes on-disk content, uncapped, so a multi-MB value produces a multi-MB stderr line. CleanupWriteFailure.while_handling (original.to_string()) also re-embeds unsanitized text. Routing: implementer + test-writer.

F-S2509-L2-002 — MEDIUM (confidence HIGH) — shard_manager.rs close_sub_shard_chunk OVERSIZED_ROW_SUBSHARD is emitted at staging time from the fresh run's chunking loop; its fixed clause says "…and the migration completed", but later steps can fail with exit 2 (verify_content_preservation, verify_independent_census → abort_staging; the intent append; FingerprintMismatchAbort; a pointer_swap Io leaving STAGING; a canonical-move halt). A resumed STAGING txn does not re-chunk, so the completing run prints nothing. Routing: architect, implementer (return as data, print only on Completed), test-writer.

F-S2509-L2-003 — MEDIUM (confidence HIGH) — pub fn chunk_subsystem_rows_into_sub_shards → close_sub_shard_chunk → emit_coordinator_advisory (eprintln!). ADR-052 item 11(g)(3)(i) says the helpers stay module-private; the first is pub; its doc says "pure — no I/O" (false); it names "the future steady-state full-rebuild path" as a caller (a hook process would write a migrate-bc-index-prefixed line to hook stderr); AC-021's recorded grep claimed module-private. Routing: implementer (same fix as L2-002); architect to reconcile rule (i).

F-S2509-L2-004 — MEDIUM (confidence HIGH) — docs/guide/migration-interim-block-recovery.md step 3b awk `/^INTENT_LOG_RECORD_V1$/ { inb=1; done=0; …; next }` fires even when inb is already 1, so a torn block followed by a new header is dropped with no X| line and the mid-stream check never sees it. ADR-052 item 11(e) step 3 requires STOP and escalate on corruption mid-stream; the runbook "differences" item 3 claims it follows the spec. Scenario: a torn INTENT followed by valid records passes "LOG RECORDS OK". Routing: technical-writer (emit X| when a header arrives while inb is set; add the negative fixture).

F-S2509-L2-005 — MEDIUM (confidence HIGH) — F-008 only partly fixed. CHANGELOG line 37 still cites "BC-1.18.011 v1.20 … EC-043..EC-047; BC-1.18.013 v1.12; ADR-052 v1.23". S-25.09 deliverables are added inside the S-25.08 entry: line 28 "One anchoring rule", line 29 "Diagnostics are events", line 30 "Error classification". Self-contradiction: line 31 "no operator-visible channel … until S-25.09" vs line 32 "reported the same way (one advisory, never a verdict)" vs line 51 under S-25.09. Routing: implementer or devops (T-45).

F-S2509-L2-006 — LOW (pending intent verification) — run_bc_index_migration_core abort_staging closure `let _ = fs.remove(gen_dir);` swallows the inert generation-directory removal failure that discard_incomplete_staging reports as STAGING_DIR_REMOVE_FAILED. AC-022(a) says no `let _ =`; ADR item 11(d) says removing this best-effort call is not required. Routing: product-owner/architect, then implementer.

F-S2509-L2-007 — LOW — pub read_active_txn_record no-live-record branch returns planner_view(terminal, ..), fabricating values (schema_version 1, "" for missing txn_id/activation_id/created_at, fencing 0, empty plan, None hashes even when present), and `files.iter().find(|f| !f.is_live())` has no migration_id filter, so a backfill-append-logs terminal record comes back typed as B2. No production caller, but ADR item 11(g)(3) treats it as an admission entry point. Routing: implementer.

F-S2509-L2-008 — LOW — executor.rs admission_error `other => sanitize_diagnostic(&other.to_string(), 256)` caps the whole Display at 256, violating the item 11(g)(1) 64-per-substring InternalLog rule for any other variant reaching admission. Routing: implementer.

F-S2509-L2-009 — LOW (pending intent verification) — detect_migration_read_state reads CURRENT.json with read_to_string, treating NotFound as NotStarted; a dangling CURRENT.json during COMMITTING reads as "legacy is current" — the same fail-open class 11(c) ruling 5 fixed for completed.json, not swept (TD-VSDD-060). Routing: architect, then implementer.

[process-gap] Two story-level closure claims recorded without a code check: AC-022(d) "Io variants … Display renders sanitize_diagnostic(subject, 256)" (L2-001) and the AC-021 recorded grep (L2-003). No step requires that verification.

Checked clean: coordinator-only emission (apart from L2-003); GATE_OPEN_RESET_FAILED re-reads the gate; STAGING_DIR_REMOVE_FAILED only after the ABORTED write; FOREIGN_MIGRATION_REFUSED at 256; AdmissionStateIntegrity raw slots rendered per surface; the runbook checksum formula matches intent_log_checksum_input incl. MISSING; F-012/F-007/F-004 tests non-vacuous with controls. Deferred: none. Novelty: MEDIUM.
