---
document_type: adversary-pass-report
level: ops
title: "S-25.08 LOCAL Adversary Cascade — Pass 3 (verbatim)"
producer: adversary
persisted_by: state-manager
timestamp: 2026-10-08T00:00:00Z
phase: per-story-step-4.5-local-adversary-cascade
story: S-25.08
pass: 3
reviewed_head: "feature/S-25.08 @ 5091f88f"
verdict: NOT CLEAN
finding_count: { total_numbered: 9, medium: 4, low: 5, deferred: 1 }
streak_3_clean: "0/3"
novelty: "MEDIUM"
source_report_saved: true
supersedes: "the RECONSTRUCTED record persisted in the D-1254 burst (source report was not saved at that time)"
traces_to: S-25.08
---

> **Provenance.** Relayed verbatim by the orchestrator from the adversary hand-back in session 2026-10-07/08; reviewed HEAD feature/S-25.08 @ 5091f88f. This file SUPERSEDES the reconstructed pass-3 record persisted in the D-1254 burst. The review text below is authoritative; story tables are index/status copies.

# S-25.08 LOCAL Adversary — Pass 3

Verdict: NOT CLEAN. 9 findings: 4 MEDIUM, 5 LOW, plus 1 deferred. Streak stays 0/3.

Identity preflight passed (worktree basename S-25.08; .git/refs/heads/feature/S-25.08 = 5091f88f; HEAD reflog ends with reset to 5091f88f; grep for migration_writer_admission|AdmissionStateIntegrity|NativeGate|resolve_session_project_root under crates/ returns 0 hits; no s2508_v121_units_test.rs; specs read from canonical .factory/).

Gate evidence: not re-run (read-only); every story Test: name for AC-001..AC-020 and AC-027 exists in tests/s2508_*.rs and bc_1_18_011_b2_migration_test.rs except the S-25.09 names in F-S2508-L3-005.

F-S2508-L3-001 MEDIUM — admission requires the full B2 txn schema for every txn-*.json, so foreign and non-live records from another build fail as state_integrity. Where: src/shard_manager/admission.rs::read_txn_files runs serde_json::from_value::<BcIndexMigrationTxnRecord>(raw) on every txn-*.json before the migration_id check; a failure becomes BinaryIntegrityFailure → state_integrity; BcIndexMigrationTxnRecord requires txn_id, activation_id, fencing_generation: u64, pending_canonical_moves: Vec<_>, created_at, updated_at. Spec: ADR-052 §Error Code Semantics total cause rule and AC-018(iii) (a txn record must be an object with a known state and, if PRESENT, a string migration_id); ADR-052 §7e "Definition of foreign" (a record this build cannot interpret is not this build's to discard; plain E-MAINTENANCE-001). Failure scenarios: (1) a live {"state":"STAGING","migration_id":"future-migration",…} record without pending_canonical_moves gives E-MAINTENANCE-002 (state_integrity) instead of plain E-MAINTENANCE-001, violating EC-029; (2) a COMPLETED/ABORTED record of that shape fails every protected write permanently. Test gap: EC-029 fixtures always write the full B2 schema. Routing: implementer (Tier-0 projection) + test-writer (minimal-shape foreign vectors).

F-S2508-L3-002 MEDIUM — a PostToolUseFailure whose tool_name is not a string never releases the reservation. src/payload.rs::HookPayload has #[serde(default)] pub tool_name: String; null/number/object fails HookPayload::from_bytes; main.rs::run returns Err, mapped to exit 0, release never reached. Spec: BC-1.18.013 EC-021(c), ADR-052 §5a F-001. Effect: reservation leaks to the 3,600 s TTL; a coordinator drain ends in DRAIN_TIMEOUT_ABORT. Test gap: the EC021 test uses the string "SomeOtherTool". Confidence HIGH on behavior, MEDIUM on spec reading. Routing: implementer + test-writer.

F-S2508-L3-003 MEDIUM — no test pins "never clamped" for a pre-epoch or out-of-range created_at. test_BC_1_18_011_EC025_drain_gc_reservation_timestamp_vectors vectors (c) 1969-12-31T23:59:59Z and (f) out-of-u64 use an OLD mtime, so a clamp-to-Some(0) implementation also passes. Missing vector: a pre-epoch/out-of-range created_at with a FRESH mtime must be retained (DrainTimeoutAbort, file present). Routing: test-writer.

F-S2508-L3-004 MEDIUM — the story text does not stand alone at the cut: (a) AC-020, the Architecture Mapping and Files-to-MODIFY rows say record_present uses Path::try_exists, but the code reads with std::fs::read; (b) unticked tasks whose deliverables are on the branch: T-15, T-18, T-19, T-11(a)-(c), T-8, T-9, T-10, T-26; (c) the pass-1 table still says Code status OPEN and "Kani row labels PENDING"; (d) the AC-012 Test: line names test_BC_1_18_011_PC6_RULING1_gate_precedence_* as removed, but two still exist; the removed ones were ..._resolve_shard_gate_precedence_*. Routing: story-writer.

F-S2508-L3-005 LOW — Test: lines name tests that exist only in S-25.09 (AC-016: ..._EC025_timestamp_fallback_tokens_are_coordinator_stderr_not_events_blackbox; AC-017: ..._EC031_below_floor_ttl_rejected_before_any_mutation; AC-018 cites "S-25.09 T-14 row (10)"). Routing: story-writer.

F-S2508-L3-006 LOW — the two remaining PC6_RULING1_gate_precedence_* tests prove an ordering their own match enforces (false-assurance pattern ADR-052 §Downstream item 22 deleted). Routing: test-writer.

F-S2508-L3-007 LOW — the terminal-record io leg has no root-safe vector; EC032 rows use only chmod 000 with needs_non_root. Routing: test-writer (EISDIR rows for the terminal record and a txn-*.json).

F-S2508-L3-008 LOW — the CHANGELOG at the cut describes diagnostics an operator cannot see (no tracing subscriber in the dispatcher). Routing: implementer.

F-S2508-L3-009 LOW (pending intent verification) — any .factory stat error is treated as "no .factory" (admission.rs::resolve_factory_root uses metadata(..).is_ok_and(is_dir)). Routing: architect/product-owner to adjudicate; implementer to fix.

Deferred D-L3-1 (integration, wave-gate): read_txn_files deserializes every txn record into the B2 struct; if S-25.06's mechanism-A txn shape differs, every protected write fails state_integrity once one exists.

Checked clean: reserve-then-verify W1 before W2; release when blocked or errored; release on a later exit 2; admission and release once, before CLAUDE_PLUGIN_ROOT tiering and Registry::load; is_tool_completion_event and tool_use_id grammar; component-wise case-insensitive classification; resolve_target_path; foreign = migration_id ∉ K; raw-bytes reads; the mismatch suffix only on Branch C; Branches A/B/C via plan_stale_gate_reconciliation; TTL 3,600/1,800 and compile-time guard; reservation_is_stale; drain fails closed; registry_error_exit_code explicit; Kani 10 harnesses pinned; no unwrap/expect/println! in admission.rs.

Novelty: MEDIUM.
