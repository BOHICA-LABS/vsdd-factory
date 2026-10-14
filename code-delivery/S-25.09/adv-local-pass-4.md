---
document_type: adversary-pass-report
level: ops
title: "S-25.09 LOCAL Adversary Cascade — Pass 4 (verbatim)"
producer: adversary
persisted_by: state-manager
timestamp: 2026-10-14T00:00:00Z
phase: per-story-step-4.5-local-adversary-cascade
story: S-25.09
pass: 4
reviewed_head: "feature/S-25.09 @ a3923ee0"
base: "a7fc5930"
verdict: NOT CLEAN
finding_count: { total_numbered: 6, critical: 0, high: 0, medium: 2, low: 4, deferred: 0 }
streak_3_clean: "0/3"
novelty: "MEDIUM"
traces_to: S-25.09
---

> **Provenance.** Relayed verbatim by the orchestrator from the adversary hand-back, session 2026-10-14; reviewed HEAD feature/S-25.09 @ a3923ee0, base a7fc5930.

# S-25.09 LOCAL Adversary — Pass 4

## Identity and spec versions

**Identity preflight: PASSED.**
- Worktree: `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.09`. The basename `S-25.09` matches the story id.
- Reviewed HEAD: `a3923ee0`. The base is the older S-25.08 tip `a7fc5930`.
- Canonical root: `/Users/zious/Documents/GITHUB/vsdd-factory`.
- All code was read from the worktree. All specs were read from the canonical `.factory/`.

**Specs used:**
- Story S-25.09 v1.9 (uncommitted).
- ADR-052 v1.31: §Decision 11(c) point 7(a)-(e) and §Decision 11(g)(2)(f) / (4).
- BC-1.18.005 v1.16 Postcondition 10.
- BC-1.18.013 / BC-1.18.011 / BC-1.18.010, at the versions cited by the story.

**Prior pass read:** `code-delivery/S-25.09/adv-local-pass-3.md` only.

**Limitation:** I cannot run git, so "S-25.09 changed this" claims are inferred from the story's commit attributions and from the AC and CHANGELOG claims that each finding contradicts.

## Verdict

**NOT CLEAN.**
- Severity counts: 0 CRITICAL, 0 HIGH, 2 MEDIUM, 4 LOW.
- Novelty: MEDIUM. Both MEDIUM findings are siblings the pass-3 fixes did not reach: another record-derived path join, and the log surface of `ShardConfigError`. Neither restates a pass-3 finding.
- Streak after this pass: 0/3.

## Pass-3 fix verification

Each line names the mutant that the cited test kills.

- **L3-001 — CLOSED on the Display, stderr and `block_reason` surfaces; load-bearing.**
  - `ShardConfigError::Toml` now stores only `{category, line, column}`, built by `from_toml_parse_failure`. There is no `From<toml::de::Error>`.
  - `ShardCapConfigUnavailable.detail` is built from `render_with_cap(256)`.
  - The Quarantine arm passes `activation_id` through `sanitize_diagnostic`.
  - Mutants: reinstating `Toml(#[from] toml::de::Error)` is killed by `test_BC_1_18_005_EC023g_shard_config_error_toml_stores_only_category_line_column_source_gate`. Reverting the detail to `{source}` is killed by `..._hostile_shard_config_toml_value_is_never_echoed_blackbox`. Reverting to `{other:?}` is killed by `..._quarantine_detail_is_bounded_and_escaped_blackbox`.
  - The log surface is still open: see F-S2509-L4-002.
- **L3-002 — CLOSED for `generation_id`; load-bearing.**
  - The `GenerationId` newtype is built only through `is_safe_path_component`, with `serde(try_from)`. `gen_dir_for` and `intent_log_path_for_generation` are the only `gen-` / `intent-` builders.
  - `resolve_live_generation_id`, `check_generation_intent_log_pair` and admission Branch B all parse before any join.
  - Mutant: letting `resolve_live_generation_id` accept any string is killed by `..._adversary_scenario_x_dotdot_dotdot_specs_never_deletes_factory_specs`.
  - A sibling identifier is still open: see F-S2509-L4-001.
- **L3-003 — CLOSED; load-bearing.**
  - `admission_error` is an exhaustive match with no wildcard. Every non-special variant goes through `e.render_with_cap(64)`.
  - Mutant: restoring `other => sanitize_diagnostic(&other.to_string(), 64)` is killed by `..._admission_error_has_no_wildcard_arm_and_renders_per_slot_source_gate` and by `..._reservation_ttl_below_floor_detail_is_the_full_fixed_text_at_cap_64`.
  - Residuals: F-S2509-L4-003, F-S2509-L4-004, F-S2509-L4-005.
- **L3-004 — CLOSED; load-bearing.**
  - `emit_oversized_row_advisories` emits `tracing::warn!(bc_id, sub_shard_id, ..)` beside the stderr line.
  - Mutant: deleting that warn is killed by `test_BC_1_18_013_L3_004_completed_fresh_run_emits_one_tracing_warn_per_oversized_sub_shard`. It has a capture-liveness control and a within-cap control.
- **L3-005 — CLOSED.**
  - The docs of `discard_incomplete_staging` and `read_active_txn_record` now match the coordinator-only, B2-only contracts.
  - This is a doc-only fix, so no mutant applies.

**T-66** is present and its code scope is correct. It has one documentation gap: see F-S2509-L4-006.

## Findings

### F-S2509-L4-001 — MEDIUM (confidence HIGH on the code path, MEDIUM on intent): a txn record's on-disk `activation_id` still builds a path

**Anchors:**
- `write_txn_record` in `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.09/crates/factory-dispatcher/src/shard_manager.rs`: `let path = _migration_state_dir.join(format!("txn-{}.json", _record.activation_id));`
- Its callers that rewrite a record decoded from disk by `decode_txn_record_strict`:
  - `abort_staging_generation`, which `discard_incomplete_staging` uses.
  - The `ResumeFromStaging` pending-moves persist and the COMMITTING advance.
  - `finish_committing_migration`.
- ADR-052 §Decision 11(c) point 7(e)(4) states that txn `activation_id` and `txn_id` are "never used to build a path; the txn file name is the directory-enumerated `txn-*.json` entry and every archive / rename derives from THAT path". The code contradicts this.
- No check anywhere ties `activation_id` to the file name. I grepped `activation_id` in `admission.rs` (0 hits), and there is no `txn_id == activation_id` check in `shard_manager.rs`.

**Why the AC-028 gate missed it:** the gate (`s2509_pass3_l3_002_path_component_source_gate_test.rs`, `mentions_generation`) only flags `.join(` operands that contain `generation_id` or `gid`. So this record-derived join is invisible to it, even though AC-028 says "no txn-record-derived path join outside the two helpers".

**Failure scenarios:**
- **(a) Benign mismatch.** `txn-<A>.json` is a live STAGING record whose `activation_id` is `<B>`, for example after a hand-restored backup.
  - `ResumeFromStaging` fails the census. `discard_incomplete_staging` removes `gen-*` and writes the record ABORTED into a new file, `txn-<B>.json`.
  - `txn-<A>.json` stays live STAGING with its generation directory gone. Every later run quarantines (`GenerationIdWithoutGenDir`, exit 2), and admission blocks every BC-INDEX write indefinitely.
  - `finish_committing_migration` has the same problem: COMPLETED lands in the wrong file, leaving a live COMMITTING record beside `completed.json`, which gives a permanent `COMPLETION_RECORD_MISMATCH_ABORT`.
- **(b) Hostile value.** An `activation_id` such as `x/../../../specs/foo`, with a `migration-state/txn-x/` directory present, makes `write_temp` and the rename land outside `migration-state/`. This is the same escape class L3-002 closed for `generation_id`.

**Fix:**
- Rewrites target the enumerated `TxnFile.path`, as `archive_terminal_txn_record` and `abort_null_generation_txn` already do. Alternatively, parse `activation_id` through `is_safe_path_component` and require that the file name equals `txn-<activation_id>.json` (`txn_record_malformed` otherwise, nothing mutated).
- Extend the AC-028 gate to flag every `.join(` operand that comes from a decoded record, not only operands named `generation_id`.
- Add a black-box vector for a mismatched `activation_id` and one for a hostile `activation_id`.

**Routing:**
- architect: correct ADR 11(c) 7(e)(4) or state the rule.
- product-owner: add the BC-1.18.011 EC.
- test-writer, then implementer.

### F-S2509-L4-002 — MEDIUM (confidence HIGH on the code, latent until a tracing subscriber is installed): `ShardConfigError` log surfaces are not sanitized at 64 per slot, and `artifact_stem` is logged raw

**Anchors:** `detect_replace_all_overcap_candidate` in `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.09/crates/factory-dispatcher/src/invoke.rs`. Three `tracing::warn!` sites render a `ShardConfigError`:
- `ShardRegistry::load` failure: `error = %e`, `shard_config_path = %..display()`.
- `find_matching_entry` failure: `error = %e`, `target_path = %..`.
- `validate_entry` failure: `artifact_stem = %entry.artifact_stem` (raw) plus `error = %e`.

The sibling EC-012 advisory inside `validate_entry` (`shard_manager.rs`) also logs `artifact_stem = %entry.artifact_stem` raw.

**Rules violated:**
- BC-1.18.005 v1.16 Postcondition 10: "The rule holds on EVERY surface … any log or event field", and "(a) every InternalLog or tracing event field renders each data slot independently capped at 64".
- ADR-052 §Decision 11(g)(2)(f)(3): "any InternalLog/tracing field (64 per data slot)".

`error = %e` renders `Display`, which is 256 per slot, and the raw `artifact_stem` field carries ESC/LF unescaped.

**CHANGELOG overclaim:** the S-25.09 bullet says "the dispatcher `shard-cap-gate` `block_reason`, the `migrate-bc-index` stderr line **and the logs** all render `ShardConfigError` through the same per-slot path, with the artifact stem and path escaped and capped". For the logs this is not true.

**Failure scenario:** a `[[shard]]` entry with no `shape` and the stem `"\u{1b}[2J\nFORGED"` hits the PostToolUse replace_all leg. The warn event carries the raw ESC and LF. This is invisible today because no production subscriber exists, but it becomes a log-injection channel as soon as one is installed. The black-box tests only cover `block_reason` and stderr.

**Fix:**
- Render `error` as `e.render_with_cap(64)`.
- Pass `artifact_stem`, `target_path` and `shard_config_path` through `sanitize_diagnostic(_, 64)` at every such site.
- Add a capture-subscriber test, in the style of the L3-004 test, that drives a hostile stem through the replace_all leg.

**Routing:** test-writer, then implementer.

### F-S2509-L4-003 — LOW (confidence HIGH): `Display` is not `render_with_cap(256)`; there are two parallel rendering sources

**Anchors:**
- The `#[error(..)]` templates on `BcIndexMigrationError` and `ShardConfigError` are separate from their hand-written `render_with_cap` part lists.
- `s2509_display_render_parity_guard_test.rs` admits in its module doc that these are "two parallel sources".

**Rules violated:**
- ADR-052 §Decision 11(g)(4)(b): "each variant declares its rendering ONCE … `Display` calls it with 256".
- BC-1.18.005 Postcondition 10(a): "`Display` and the event rendering are ONE `render_with_cap(cap)` source".
- Story AC-030 and T-60: "`Display` calls it with 256".

**False claims about the delivered code:**
- The CHANGELOG says "the event `detail` and the operator `Display` come from the same `render_with_cap` implementation".
- A test doc says "`Display` IS `render_with_cap(256)`".
- The parity guard's claim that "the one source requirement is only checkable by pinning them equal" is wrong: a manual `impl Display` that delegates makes the two sources one by construction.

**Exposure:** the guard compares only the sampled instances. For example, the `Io` + `CleanupWriteFailure` branch, a private type, is never constructed by the guard, so a divergence there would go unseen.

**Fix:** implement `Display` by delegating to `render_with_cap(256)`, with a manual `impl Display` and `impl Error::source`. Alternatively, the architect accepts "two sources plus a parity guard" and amends the ADR, and the CHANGELOG and test-doc wording are corrected.

**Routing:** implementer, or architect if the alternative is to be accepted.

### F-S2509-L4-004 — LOW (confidence HIGH): `pub const FIXED_TEXT_MAX_CHARS` is dead code, and the test's claim about it is false

**Anchors:**
- `shard_manager.rs` declares `pub const FIXED_TEXT_MAX_CHARS: usize = 512`. A grep shows it is referenced nowhere else in `src` or `tests`.
- `s2509_pass3_l3_003_per_slot_rendering_api_test.rs` redefines a local `const FIXED_TEXT_MAX_CHARS: usize = 512`. Its comment says "restated here rather than imported, so a silent change to the production constant fails this test", which is false because nothing compares the two.

**Mutant that survives:** setting the production constant to `1` or to `100_000`.

**Rule:** ADR 11(g)(4)(c) / T-60 call for "the named constant … with the per-variant bound asserted".

**Fix:** have the test import the production constant, assert that it equals 512, and assert the bound against it.

**Routing:** test-writer.

### F-S2509-L4-005 — LOW (confidence HIGH; partial fix of L3-003): a CHANGELOG bullet still describes the removed `other` arm

**Anchor:** the S-25.09 "Per-surface caps on data-derived substrings (S-25.09 AC-022(d)…)" bullet in `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.09/CHANGELOG.md`. It says "…capped at 64 characters, **including the generic `other` arm of `admission_error`**".

**Defect:** after `b7f43b9d`, `admission_error` has no `other` arm (exhaustive, no wildcard). Pass 3 named this exact bullet (L3-003). The `614be57e` CHANGELOG commit added a new bullet but did not correct this one.

**Fix:** reword the bullet to describe the exhaustive per-slot rendering, or fold it into the AC-030 bullet.

**Routing:** implementer or devops-engineer (CHANGELOG).

### F-S2509-L4-006 — LOW (confidence HIGH): T-66 leaves the CHANGELOG naming `migration_writer_admission_precheck`

**Anchors:**
- The story's T-66 lists "its definition in `executor.rs`, its imports, every call site, doc comment and Kani-comment mention".
- The S-25.09 CHANGELOG bullet "Error classification and naming (S-25.09 AC-004, AC-005, AC-006)" lists `migration_writer_admission_precheck` as a shipped migration-neutral entry point.
- T-66's extended AC-006 gate is scoped to `crates/`, so it cannot catch `CHANGELOG.md`.

**Failure scenario:** after the rebase drop, the release notes advertise an entry point that ADR-052 v1.31 forbids and that does not exist.

**Fix:** add the CHANGELOG bullet to T-66's drop list. Optionally, extend the gate to cover `CHANGELOG.md`.

**Routing:** story-writer (T-66 text), then implementer.

## Observations (non-blocking)

- At cap 64, `render_with_cap` does not cap the data slots of `ArchIndexParityAbort` (its nested Display is 256 per slot) or `ShardCapConfigUnavailable` (`cap * 8`). Both are classified as coordinator-only and unreachable from `admission_error`, so ADR 11(g)(4)(a), which covers variants "that can reach `admission_error`", is not violated. The boundedness test only exercises `Io`.
- The recover-arm tracing fields `activation_id = %activation_id` (the DiscardPreGeneration, CleanAbortExpiredStaging, ResumeFromStaging, ForwardRecovery and RequiresReauthorization arms) log on-disk text raw. These appear to predate S-25.09 and no subscriber is installed. I am noting them for the F-S2509-L4-001 sweep rather than raising them as a finding.

## [process-gap]

- **[process-gap]** For the third consecutive pass, a closure was checked against the identifier or variant the finding named, not the BC's scope sentence:
  - L2-001 was checked against the AC's variant list.
  - The L3-001 fix covered Display and stderr but not "any log or event field".
  - The L3-002 gate matches the token `generation_id`, not "record-derived".

  Source gates should key on provenance (any value decoded from an on-disk record or config) and on every surface named in the BC sentence, not on the identifier name in the finding.

## Deferred

None. Every finding is within S-25.09's own ACs: AC-028, AC-030, AC-031, AC-032 and T-66.
