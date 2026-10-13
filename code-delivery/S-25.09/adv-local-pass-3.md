---
document_type: adversary-pass-report
level: ops
title: "S-25.09 LOCAL Adversary Cascade — Pass 3 (verbatim)"
producer: adversary
persisted_by: state-manager
timestamp: 2026-10-13T00:00:00Z
phase: per-story-step-4.5-local-adversary-cascade
story: S-25.09
pass: 3
reviewed_head: "feature/S-25.09 @ df03ecbe"
verdict: NOT CLEAN
finding_count: { total_numbered: 5, critical: 0, high: 0, medium: 2, low: 3, deferred: 0 }
streak_3_clean: "0/3"
novelty: "MEDIUM"
traces_to: S-25.09
---

> **Provenance.** Relayed verbatim by the orchestrator from the adversary hand-back, session 2026-10-13; reviewed HEAD feature/S-25.09 @ df03ecbe.

# S-25.09 LOCAL Adversary — Pass 3

**Identity preflight: PASSED.**
- Worktree: `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.09`. Its basename `S-25.09` matches the story id.
- Reviewed HEAD: `df03ecbe`.
- Canonical root: `/Users/zious/Documents/GITHUB/vsdd-factory`. All specs were read from canonical `.factory/`; all code was read from the worktree.

**Spec versions** (frontmatter checked):
- BC-1.18.010 v1.12
- BC-1.18.011 v1.23
- BC-1.18.013 v1.15
- ADR-052 v1.27
- Story S-25.09 v1.5
- Prior pass read: `code-delivery/S-25.09/adv-local-pass-2.md` only.

**Verdict: NOT CLEAN.** 0 HIGH, 2 MEDIUM, 3 LOW. Novelty is MEDIUM: both MEDIUM findings are sibling gaps that the pass-2 fixes did not sweep (TD-VSDD-060), not rewordings of pass-2 findings. **Streak after this pass: 0/3.**

## Pass-2 fix verification

Each line names the mutant that the cited test kills.

- **L2-001 — CLOSED for the named variants, load-bearing.**
  - `BcIndexMigrationError::Io` now renders two separate 256-capped substrings: `sanitize_diagnostic(path)` and `io_source_display(source)`.
  - `io_source_display` special-cases a `CleanupWriteFailure` source by downcast, so the "while handling" tail keeps its own cap.
  - `BcIndexAddressingError::{Io, Toml, MalformedBcId}` use `addressing_display_slot`. `Toml` carries `toml_failure_message`, which is category plus line/column only.
  - Mutants: reverting to a raw `{path}: {source}` is killed by `..._L2_001_io_display_escapes_control_chars_in_path_single_line_blackbox`. A single joined cap is killed by `..._cleanup_write_failure_display_caps_each_substring_and_keeps_the_tail`.
  - The "every variant" sweep is still incomplete: see F-S2509-L3-001.
- **L2-002 — CLOSED, load-bearing.**
  - Advisories are collected as data in `run_bc_index_migration_core` (`oversized_advisories.extend(oversized_row_advisories(..))`).
  - `emit_oversized_row_advisories` runs only after `finish_committing_migration(..)?` returns. That function's only `Ok` is `Completed`.
  - The resume arms never collect advisories.
  - Mutant: moving emission back into the chunker, or emitting before `finish_committing_migration`, is killed by `..._L2_002_no_oversized_advisory_when_verification_fails_blackbox` and `..._when_a_later_step_fails_blackbox`. Both run on an oversized fixture, with a positive control that expects exactly 1 line.
- **L2-003 — CLOSED.**
  - `close_sub_shard_chunk` is private and pure.
  - `emit_coordinator_advisory`, `emit_oversized_row_advisories` and `emit_staging_dir_remove_failed` are all `fn`, not `pub`.
  - Mutant: making a helper `pub`, or calling it from the chunker, is killed by the purity source gates (positive and negative controls work). The child-process stderr-empty test adds behavioral coverage.
- **L2-004 — CLOSED.**
  - The runbook step-3b awk header rule now prints an `X|` line when a header arrives while `inb` is set.
  - I traced the fixture by hand: the record order is V, X, V, V, so the check prints `STOP`.
  - The awk agrees with `append_intent_log_record` / `parse_intent_log_block` on the key set, the `MISSING` sentinel, the 8-value checksum order, split on the first `=`, last-duplicate-wins, and ignoring lines without `=`.
  - The stale SHA self-cites are gone (no 8-hex tokens remain on the page).
- **L2-005 — CLOSED.**
  - The CHANGELOG now has a separate S-25.09 entry. Its header has no version pins and every EC/AC is owner-qualified.
  - The S-25.08 entry only contains forward references ("arrives with S-25.09").
  - Residual inaccuracies are recorded under L3-001 and L3-003.
- **L2-006 — CLOSED, load-bearing.**
  - `abort_staging_generation` is shared by `discard_incomplete_staging` and the fresh-run `abort_staging` closure. Order: capture the removal result, then the checked ABORTED write, then the advisory, then the caller's checked gate-OPEN write.
  - Mutant: restoring `let _ = fs.remove(gen_dir)` is killed by the AC-024 source gate and by `..._fresh_run_abort_dir_remove_failure_is_reported_blackbox`.
- **L2-007 — CLOSED.**
  - The `migration_id == MIGRATION_ID_B2` filter is in place, and live records are chosen with `select_live_txn_among`.
  - Terminal records go through `decode_terminal_txn_record`, which strict-decodes and returns `None` on failure. There is no `planner_view` fabrication left.
  - Mutant: reinstating `planner_view(terminal)` is killed by the `ec085_*` pins and by `..._does_not_fabricate_absent_terminal_fields`.
- **L2-008 — CLOSED as specified by the story's T-56.**
  - The generic arm now uses `EVENT_DETAIL_SUBSTRING_MAX_CHARS`, enforced by a source gate with controls.
  - Residual: see F-S2509-L3-003.
- **L2-009 — CLOSED, load-bearing.**
  - `read_current_pointer_marker` is lstat-first, builds category-form `InvalidData`, checks `status == "committing"` and requires a single-component `generation_id`. Both `detect_migration_read_state` and `read_current_generation_pointer_if_present` use it.
  - In `ResumeFromStaging`, the probe runs before `resume_from_staging`, the discard and any write.
  - Mutant: reverting to read-with-`NotFound`-as-absent is killed by `..._coordinator_probe_io_verdicts_exit_2_nothing_mutated_blackbox` (a byte-identical snapshot over every EC-078..EC-082 vector).

**Rewritten older tests did not lose coverage:**
- `ec11326b`: `snapshot_readable` unlocks a mode-000 file under the same conditions before and after the run, and records the mode bits, so a chmod by the run would still be caught.
- `d2a106bc`: the F004 fixtures now carry `txn_id`. This is required by the v1.12 pointer schema; the dangling-symlink and absent controls are kept.
- `84ee8d26`: the test previously asserted a warning; it now asserts a stronger contract. Its subscriber's `enabled` returns `true`, so "no event" is not a tautology, and it pins the `oversized_row_advisories` data fields.

**Failpoint seams:**
- `migration_core::verify_content_preservation` and `migration_core::verify_independent_census` are `#[cfg(feature = "failpoints")]` shadowing `let` bindings.
- The `fail` dependency is `optional = true`, and the `failpoints` feature is `["dep:fail", "fail/failpoints"]`. A build without the feature contains neither the binding nor the crate, so the seams cannot affect release builds.
- With the feature on but the failpoint not configured, `fail::eval` returns `None` and the real check result is used.

## Findings

### F-S2509-L3-001 — MEDIUM (confidence HIGH): the "every variant" Display sanitization sweep is incomplete, and a TOML parser message still reaches coordinator stderr

**Anchors:** `resolve_bc_index_shard_cap_bytes`, `BcIndexMigrationError::ShardCapConfigUnavailable`, `ShardConfigError::Toml`, the `RecoveryDecision::Quarantine` mapping in `run_bc_index_migration_core`, `recompute_pending_canonical_moves_from_staged_generation`, `run_migrate_bc_index_cli`.

**Rules violated:**
- BC-1.18.013 Postcondition 10 sanitization paragraph: "the Display of every `BcIndexMigrationError` variant that quotes data" gets the 256 per-substring rule, and "Serde and TOML messages are never carried".
- ADR-052 §Decision 11(g)(2): "operative for EVERY variant".
- D-1257: human-confirmed.
- AC-026 also required "the grep of every … `#[error("...{source}")]` that embeds a parser message".

**Verified sites:**
- (a) `ShardConfigError::Toml` is `#[error("shard config parse failed: {0}")]` over a `toml::de::Error`. This is toml 0.8, whose Display is several lines and includes the offending source line. The module's own `toml_failure_message` doc says this text "can echo on-disk content".
  - `resolve_bc_index_shard_cap_bytes` builds `ShardCapConfigUnavailable { detail: format!("failed to load [[shard]] config at {}: {source}", shard_config_path.display()) }`.
  - That variant's Display is `{detail}`, raw.
- (b) The `Quarantine` arm maps `other => BinaryIntegrityFailure { message: format!("recover(): quarantined -- {other:?}") }`. Its `QuarantineReason::GenerationIdWithoutGenDir { activation_id, generation_id }` comes from `planner_view` over a live record that has not been strict-decoded.
  - It is reachable whenever a live STAGING or COMMITTING record has a string `generation_id` and no `gen-` directory.
  - Debug escapes control characters but applies no cap, so the line is unbounded.
- (c) `recompute_pending_canonical_moves_from_staged_generation` interpolates `gen-{generation_id}` raw into a `BinaryIntegrityFailure` message.

**Failure scenarios:**
- `.factory/shard-config.toml` contains `shard_cap_bytes = "AKIA…secret"`. A fresh `migrate-bc-index` run reaches the shard-cap resolution (after the drain and the STAGING txn write). `eprintln!("migrate-bc-index: {e}")` prints a multi-line message that contains the secret line. This breaks both "one single line" and "never carried".
- A live txn record with a multi-megabyte `activation_id` and no gen directory produces a stderr line of several megabytes. This is the uncapped class pass-2 L2-001 scenario 2 raised.
- The CHANGELOG S-25.09 bullet "A parser message from serde or TOML is never carried" overclaims.

**Fix:**
- Build `ShardCapConfigUnavailable` (or a `ShardConfigError::Toml` category form) from `toml_failure_message`, keeping raw slots and sanitizing them at render time at 256 each.
- Render the `Quarantine` and recompute messages with per-slot `sanitize_diagnostic(_, 256)`.
- Add a hostile-TOML black-box vector and an oversized-`activation_id` quarantine vector.
- Re-record the grep of `#[error(...{0}|{source}|{detail}|{message}|{reason})]` over the variants reachable from `run_migrate_bc_index_cli`.

**Routing:** implementer and test-writer.

### F-S2509-L3-002 — MEDIUM (confidence HIGH on the code path, MEDIUM on spec intent): the txn record's `generation_id` is never checked to be a single path component

**Anchors:** `resolve_live_generation_id`, `check_generation_intent_log_pair`, the `gen_dir_exists` probe, `discard_incomplete_staging` → `abort_staging_generation` → `try_remove_staging_dir` → `StdFs::remove` (`remove_dir_all`), `resume_from_staging`, `intent_log_path_for_generation`.

**Gap:**
- The pass-2 fix (EC-082, `read_current_pointer_marker`) rejects an empty, `.`, `..`, `/`, `\` or NUL `generation_id` in CURRENT.json, "never a path escaping `migration-state/`".
- The txn record's `generation_id` gets no such check. `resolve_live_generation_id` accepts any string.
- The pair check only requires `intent_log_path` to equal `intent_log_path_for_generation(gid)`, so a crafted record can satisfy both fields at once.
- The coordinator then uses `migration_state_dir.join(format!("gen-{gid}"))` for the existence probe, the resume reads and the discard.

**Failure scenario:**
- A live STAGING record has `generation_id: "x/../../specs"` and a matching `intent_log_path`, and a `gen-x` directory exists.
- `gen_dir_exists` resolves to `.factory/specs` and returns true.
- `resume_from_staging` fails its census over that tree. `discard_incomplete_staging` then calls `remove_dir_all(".factory/migration-state/gen-x/../../specs")` and recursively deletes `.factory/specs`.
- `intent_log_path_for_generation` likewise appends outside `migration-state/`.
- Writes to `migration-state` are guard-blocked for agents, so the realistic triggers are corruption or operator error. Still, this is the exact escape class that EC-082 closed for the sibling file.

**Fix:** extend the EC-082 single-component rule to the txn `generation_id`. It should run before the tri-state hands a string to the planner and before any join, and fail with `txn_record_malformed`, nothing mutated. Then add vectors for `..`, `a/b`, `a\b`, NUL and empty, each asserting a byte-identical tree.

**Routing:**
- architect: ADR-052 item 5 / 11(c) mirror.
- product-owner: BC-1.18.011 Precondition 6(f) tri-state plus a new EC.
- implementer and test-writer.

### F-S2509-L3-003 — LOW (confidence MEDIUM; pending intent verification): the generic `admission_error` arm truncates fixed text at 64, and the CHANGELOG says it caps per substring

**Anchors:** `executor.rs` `admission_error` `other =>` arm; CHANGELOG S-25.09 "Per-surface caps" bullet.

**What happens:**
- `sanitize_diagnostic(&other.to_string(), 64)` caps the whole Display.
- ADR-052 §Decision 11(g) says data-derived substrings are capped at 64 and the fixed `message` is never re-truncated, with "no re-truncation of an already-escaped string".
- For a variant with no data slots, the event detail therefore loses its code token. The pass-2 red test file names `ReservationTtlBelowFloor` (via the TTL seam) as one such variant: it would render as "BC-INDEX migration: configured reservation TTL 10s is below th…".
- The CHANGELOG states "each data-derived substring is escaped and capped at 64 characters, including the generic `other` arm", which this code does not do. No production variant with data reaches this arm today, so the impact is limited to the event detail and the CHANGELOG's accuracy.

**Routing:**
- product-owner or architect to confirm the intended rule.
- Then the implementer, either to render per-slot, or to correct the CHANGELOG wording.

### F-S2509-L3-004 — LOW (pending intent verification): the oversized-row emitter dropped the "retained `tracing::warn!`"

**Anchors:** `emit_oversized_row_advisories`; sibling `try_remove_staging_dir`.

**Gap:**
- BC-1.18.013 v1.15 Postcondition 10: "The retained `tracing::warn!` is emitted by the same module-private emitting helper at the same moment, not from the chunker."
- `emit_oversized_row_advisories` emits only stderr.
- The sibling `try_remove_staging_dir` keeps its `tracing::warn!`.
- There is no observable effect today because no subscriber is installed. Once S-26.06 installs one, the oversized fact will be missing from the structured log while every other coordinator advisory is present.

**Routing:** implementer (add the warn inside the private emitter), or product-owner (if the BC sentence is meant to be permissive).

### F-S2509-L3-005 — LOW: stale doc comments on functions this story changed contradict the new emission and selection contracts

**Anchors:** `discard_incomplete_staging` doc; `read_active_txn_record` doc.

**Defects:**
- `discard_incomplete_staging`'s doc says it is "Used by both … `run_bc_index_migration`'s own STAGING-resume branch and [`reconcile_stale_admission_gate`]'s lighter-touch probe".
  - Grep shows only coordinator callers.
  - Since S-25.09 this function prints a stderr advisory through `abort_staging_generation`. A doc claiming an admission-path caller describes a violation of BC-1.18.013's coordinator-only emission rule, which would mislead a maintainer.
- `read_active_txn_record`'s doc still says it "selects deliberately among every `txn-*.json` file present" and frames itself as the admission gate's observer. AC-027 made it a B2-only typed observation that admission does not use.

**Routing:** implementer (doc correction; same class as L1-013).

## [process-gap]

- **[process-gap]** For the second consecutive pass, a sanitization closure (L2-001, and now L3-001) was checked against the variants an AC enumerated rather than the BC's "every variant" scope sentence.
  - AC-026 asked for the grep of every `#[error("...{source}")]` that embeds a parser message. `ShardConfigError::Toml` `{0}`, reached through `ShardCapConfigUnavailable { detail }`, was missed anyway.
  - No step turns a BC "every X" scope sentence into a mechanical enumeration, such as a test iterating all `BcIndexMigrationError` constructors reachable from `run_migrate_bc_index_cli`.
- **[process-gap]** A sibling-component rule (EC-082) was added for one file without sweeping the other on-disk sources of the same identifier (L3-002). The AC-023 sibling-sweep instruction covered absence semantics only, not value validation.

## Checked clean (evidence)

- **Coordinator-only emission:** every `emit_coordinator_advisory` call is reached only from coordinator call trees (`discard_incomplete_staging` callers are coordinator-only; the chunker is pure).
- **`OVERSIZED_ROW_SUBSHARD` ordering:** printed after `finish_committing_migration`, so it follows `GATE_OPEN_RESET_FAILED` / `TERMINAL_TXN_ARCHIVE_FAILED`.
- **EC-060b order and no-advisory-on-failed-ABORTED:** structurally enforced in `abort_staging_generation`.
- **`CleanupWriteFailure`:** two independent caps; `io_source_display` downcast verified.
- **CURRENT.json and completed.json reads:** no remaining `exists` / `read_to_string` with `NotFound` treated as absent (grep of both file names).
- **`InvalidData` sites in the coordinator and reader:** all use the category form via `parse_failure_message` or fixed text. The remaining `e.to_string()` site is S-25.06's mechanism-A code.
- **Runbook:** the gate file name `gate-state.json` matches `write_admission_gate_state`. Step-3b parity with the Rust reader was confirmed except where the ADR intentionally differs: the runbook STOPs on a mid-stream tear, while `read_intent_log` skips it.
- **CHANGELOG:** no version pins; every EC/AC is owner-qualified; S-25.09 content only in the S-25.09 entry.
- **Silent failures:** no new `let _ =`, `.ok()` or `unwrap_or` in S-25.09's pass-2 code (grep).

## Deferred

None.

## Files

- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.09/crates/factory-dispatcher/src/shard_manager.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.09/crates/factory-dispatcher/src/executor.rs`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.09/CHANGELOG.md`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.09/docs/guide/migration-interim-block-recovery.md`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.factory/specs/behavioral-contracts/ss-01/BC-1.18.013.md`
- `/Users/zious/Documents/GITHUB/vsdd-factory/.factory/specs/behavioral-contracts/ss-01/BC-1.18.010.md`
