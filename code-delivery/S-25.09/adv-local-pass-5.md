---
document_type: adversary-pass-report
level: ops
title: "S-25.09 LOCAL Adversary Cascade — Pass 5 (verbatim)"
producer: adversary
persisted_by: state-manager
timestamp: 2026-10-14T00:00:00Z
phase: per-story-step-4.5-local-adversary-cascade
story: S-25.09
pass: 5
reviewed_head: "feature/S-25.09 @ 1edac860"
base: "a7fc5930"
verdict: NOT CLEAN
finding_count: { total_numbered: 6, critical: 0, high: 0, medium: 2, low: 4, deferred: 2 }
streak_3_clean: "0/3"
novelty: "MEDIUM"
traces_to: S-25.09
---

> **Provenance.** Relayed verbatim by the orchestrator from the adversary hand-back, session 2026-10-14; reviewed HEAD feature/S-25.09 @ 1edac860, base a7fc5930.

# S-25.09 LOCAL Adversary — Pass 5

## Identity
- **Worktree-identity preflight: PASSED.** The identity tuple was present. The worktree basename `S-25.09` matches the story id.
- Worktree: `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.09` (branch `feature/S-25.09`).
- Reviewed HEAD: `1edac860`. Base: `a7fc5930`.
- Canonical root: `/Users/zious/Documents/GITHUB/vsdd-factory`.
- I read code only from the worktree and specs only from the canonical `.factory/`.
- Specs read: story S-25.09 v1.10, ADR-052 (§11(c) 7(e), §11(f), §11(g)(2)(c)-(f) and (4)), BC-1.18.005 Postcondition 10, and BC-1.18.013 Postcondition 10 / EC-068.
- The only prior pass I read is `adv-local-pass-4.md`.
- I cannot run git, so "changed by S-25.09" is inferred from the story, the CHANGELOG and the pass-4 fix scope you listed.

## Verdict
- **NOT CLEAN:** 0 CRITICAL, 0 HIGH, 2 MEDIUM, 4 LOW.
- **Novelty: MEDIUM.**
  - L5-001 is a sibling of L4-002 that the fix missed, in the same `tracing` calls it edited.
  - L5-002 is the pass-4 [process-gap] coming back: the "provenance" gates are still keyed on names.
  - L5-004 is an unswept sibling of L3-003.
- **Streak after this pass: 0/3.**

## Pass-4 fix verification
- **L4-001: CLOSED, and the fix is load-bearing.**
  - `write_txn_record` is gone. Every rewrite calls `write_txn_record_at(fs, path, ..)` with `live.path` (the enumerated entry) or with the fresh-run path from `txn_path_for_activation` (UUID v4 through `SafeId::parse` and `txn_path_for`).
  - Mutant "rebuild `txn-<activation_id>.json` at any of the 3 callers" is killed by `test_BC_1_18_011_L4_001_rewrite_lands_at_the_enumerated_txn_path_when_activation_id_mismatches_the_filename`. Its liveness control is `..._control_matching_activation_id_is_rewritten_in_place`.
  - Mutant "join a hostile id" is killed by `..._hostile_activation_id_writes_nothing_outside_migration_state`.
  - The source gate is weaker than T-67 asks: see L5-002.
- **L4-002: PARTIAL.**
  - Fixed: the `invoke.rs` legs (`render_with_cap(64)` plus sanitized path and stem), the `validate_entry` EC-012 advisory and the recover-arm `activation_id` fields.
  - Mutant "revert `error = %e.render_with_cap(64)` to `%e`" is killed by the RULE 1 gate and the capture tests in `s2509_pass4_l4_002_tracing_field_sanitization_test.rs`.
  - Still open: sibling fields in the same `tracing` calls and in coordinator sites: see L5-001.
- **L4-003: CLOSED, and the fix is load-bearing.**
  - The `Display` of `BcIndexMigrationError`, `ShardConfigError`, `ShardRollError` and `BcIndexAddressingError` is a manual `f.write_str(&self.render_with_cap(ADMISSION_STDERR_SUBSTRING_MAX_CHARS))`.
  - `ArchIndexParityAbort` splices `source.render_parts()`. `ShardRollError`'s io source is `Data`.
  - Mutant "re-add an `#[error]` template or a non-delegating `Display`" is killed by `s2509_pass4_l4_003_display_single_source_test.rs`.
  - Mutant "`Verbatim` the nested parity SHAs" is killed by `s2509_pass5_render_source_text_bounds_test.rs`.
- **L4-004: CLOSED.** `s2509_pass3_l3_003_per_slot_rendering_api_test.rs` imports the production `FIXED_TEXT_MAX_CHARS` and asserts it equals 512. Mutant "set the constant to 1 or 100_000" is now killed.
- **L4-005: CLOSED in text.** The CHANGELOG "Per-surface caps" bullet now describes the exhaustive per-variant match. The T-71 CHANGELOG-scan control is not present (no test in `crates/` reads `CHANGELOG.md`); it depends on the T-66 scan, which is still OPEN. See L5-006.
- **L4-006 / T-66:** the T-66 text now covers `CHANGELOG.md`, and its scope is correct. `migration_writer_admission_precheck` remains in `executor.rs` and on CHANGELOG line 40, as expected while T-66 is OPEN.

## Findings

### F-S2509-L5-001 — MEDIUM (confidence HIGH on code; latent until a tracing subscriber is installed) — tracing fields with on-disk or config-derived values are still raw or 256-capped beside the fields the pass-4 fix sanitized

**Anchors** (all in `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.09/crates/factory-dispatcher/src/shard_manager.rs`):
- `self_heal_resume_from_truncate`, the 0-byte guard: `artifact_stem = %sanitize_diagnostic(..)` is followed in the same macro by `sealed_path = %sealed_path.display()`, which is raw.
- `self_heal_reconcile_missing_index_entries`, the 0-byte guard: the stem is sanitized, then `path = %file_name` is raw. `file_name` is a directory-enumerated entry that embeds the stem.
- The 0-byte reclaim warn in the seal-publish path: `sealed_path = %sealed_path.display()`, raw.
- `archive_terminal_txn_record`: `path = %path.display()`, raw. This is the enumerated `txn-*.json` file name.
- `try_remove_staging_dir`: `error = %source`. Here `source` is a `BcIndexMigrationError`, so the field gets `Display` at 256 per slot, not 64.
- The best-effort gate reset in `finish_committing_migration`: `error = %e`, also 256 per slot.

**Why it fails:**
- The sealed-shard name is `format!("{artifact_stem}.{seq:04}.md")`, so `sealed_path` and `file_name` carry the config `artifact_stem`.
- `validate_entry` rejects only `/`, `\`, `..` and NUL, so ESC and LF survive.
- The result: the hostile stem is escaped in the `artifact_stem` field and emitted raw in the next field of the same event.

**Rules violated:**
- Story AC-030 GATE KEY: "any `tracing` field ... fed by an on-disk or config-derived value".
- AC-031 GATE KEY: "EVERY tracing field and on-disk or config string on every surface".
- T-68: "every on-disk or config-derived string is rendered per slot at no more than 64 characters and sanitized".
- BC-1.18.005 Postcondition 10(a).

**False CHANGELOG claim:** the AC-031 bullet says "Every `tracing` field that carries a config-derived or on-disk string ... is escaped and capped at 64 characters per slot". That is not true.

**Why the gate missed it:** `s2509_pass4_l4_002_tracing_provenance_source_gate_test.rs` keys on 7 field NAMES (`RAW_FIELDS`: `artifact_stem`, `target_path`, `activation_id`, ...) and 5 producer-function names (`CONFIG_ERROR_PRODUCERS`). Fields named `sealed_path`, `path`, `gen_dir` or `error` (when it carries a `BcIndexMigrationError`) are invisible to it. This is the exact name-keyed reading that the v1.10 GATE KEY sentence forbids.

**Failure scenario:** a `[[shard]]` stem `"x\u{1b}[2J\nFORGED"` with a matching target, where a 0-byte file sits at the next sealed-seq path. The warn event carries a raw ESC and LF in `sealed_path` / `path`, and the CHANGELOG says this cannot happen.

**Fix:**
- Pass every listed field through `sanitize_diagnostic(_, 64)` or `render_with_cap(64)`.
- Rebuild the gate so it flags every `tracing` field whose value is not a literal, a numeric, a closed token or a sanitizer call (default deny with an allowlist), not a name list.
- Extend the capture tests to the `self_heal_*` and coordinator sites.

**Routing:** test-writer, then implementer; implementer or devops for the CHANGELOG bullet.

### F-S2509-L5-002 — MEDIUM (confidence HIGH on the gate code) — the AC-028 "provenance" gate is name-keyed and statement-local, and does not check write, rename or remove targets as T-67 requires

**Anchor:** `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.09/crates/factory-dispatcher/tests/s2509_pass3_l3_002_path_component_source_gate_test.rs`. The relevant parts are `RECORD_IDENTS`, `RECORD_RECEIVERS`, `mentions_record_value` and `provenance_scan`.

**How the gate works:**
- It flags only `.join(` operands and path-shaped `format!` bodies.
- It flags them only when they lexically contain one of 5 identifiers or one of 4 receiver names (`record.`, `_record.`, `txn.`, `rec.`), within the same statement.

**What T-67 requires and the gate does not do:**
- T-67 asks for a gate that "flags any write, rename or remove of a txn record whose target path is not derived from the enumerated entry".
- The gate never inspects the targets of `write_txn_record_at`, `rename` or `remove`.
- It does not see `PathBuf::from` or `Path::new` of a record value.
- It is defeated by one intermediate binding: `let n = t.activation_id.clone(); dir.join(format!("txn-{n}.json"))` passes.

**Concrete evidence it is blind:** `recorded_intent_log_path` (`shard_manager.rs`) builds `relative` from `txn.intent_log_path` and returns `factory_root.path().join(relative)`. That is a record-derived join, and the gate does not flag it.
- ADR-052 §11(c) 7(e)(6) says txn `intent_log_path` is "never opened or joined; compared for equality with the path DERIVED".
- The behaviour is safe today, because the strict pair check proves the recorded value equals `intent_log_path_for_generation`.
- But the code opens the recorded value, not the derived one. The simplest conformant change is to join `intent_log_path_for_generation(&gid)` after the equality check.
- The gate cannot tell either way.

**What still protects L4-001:** the three black-box tests, but only for the three callers they cover.

**Routing:** test-writer, to make the gate check write, rename and remove targets by provenance (a target must flow from `TxnFile.path` or from `txn_path_for`), with a laundering positive control. Implementer, to optionally join the derived intent-log path. Architect, if 7(e)(6) and ruling (i) are meant to allow opening the recorded value.

### F-S2509-L5-003 — LOW (confidence HIGH) — the `ShardConfigError::Io.source` allowlist proof does not cover the implicit `#[from]` door

**Anchors:**
- `shard_manager.rs`: `Io(#[from] io::Error)` is rendered `Verbatim(source.to_string())`.
- The allowlist proof is in `s2509_pass3_l3_every_variant_sanitization_gate_test.rs`.
- Its mechanical guard is `test_BC_1_18_005_P10_shard_config_io_has_no_other_constructor_in_src`, which only greps `ShardConfigError::Io(` and `ShardConfigError::from(`.

**Why the proof is incomplete:**
- `#[from]` means any `?` on an `io::Result` inside a function that returns `ShardConfigError` builds the variant implicitly, and the grep cannot see that.
- The claim is true today: only `ShardRegistry::load` does this, and `read_to_string` errors carry only OS text.
- Mutant that survives every test: `validate_entry` gains `let _ = load_shard_index(&p)?;`. That function wraps the toml parser message with `io::Error::other`, which echoes source lines. The message would flow `Verbatim`, unescaped and uncapped, into `block_reason`.

**Fix:** either drop `#[from]` and build `Io` in `load` with an explicit constructor (then the grep becomes total), or render the source as `Data` like every sibling `Io` and delete the allowlist entry.

**Routing:** implementer, with test-writer.

### F-S2509-L5-004 — LOW (confidence HIGH on code; no non-`Io` caller today) — a wildcard arm caps a whole `Display` at 64 in the Event 13 `detail`

**Anchor:** `reservation_release_failed_advisory` in `/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-25.09/crates/factory-dispatcher/src/shard_manager/admission.rs`:

```rust
other => sanitize_diagnostic(&other.to_string(), cap),
```

**What is wrong:**
- `cap` is 64. This truncates the composed `Display`, fixed text included, which is the L3-003 pattern that ADR-052 §11(g)(4) and 11(g)(1) forbid ("each data-derived substring capped at 64").
- The function takes any `&BcIndexMigrationError` and is S-25.09 code (AC-022(c)/(f2)).
- The sweep for "no variant caps a whole Display" covered only `admission_error`.

**Fix:** use `other.render_with_cap(cap)`, or make the match exhaustive. Add a vector with a long fixed-text variant.

**Routing:** test-writer, then implementer.

### F-S2509-L5-005 — LOW (confidence HIGH) — doc comments contradict the delivered rendering

Anchors in `shard_manager.rs`:
- **`ShardRollError::render_with_cap` doc:** says "fixed text and the OS error text are verbatim". The pass-4 fix made the source a `Data` slot (escaped and capped).
- **The `From<ShardConfigError> for HookResult` doc:** says "the error's own `Display` impl (via `thiserror`)". `Display` is now the manual `render_with_cap(256)` delegation. The `ShardRollError` sibling doc was updated; this one was not.

This is Standing Rule 3 §3 (a doc comment that claims something the code does not do).

**Routing:** implementer.

### F-S2509-L5-006 — LOW (confidence HIGH) — story status does not match the delivery under review

**Anchor:** story v1.10 still shows T-67, T-68, T-69, T-70 and T-71 as `[ ]` OPEN, while their deliverables are present at `1edac860`. T-64, also still open, requires that the story be "reconciled with any pass-4 fix commits BEFORE pass 5 is dispatched" (the codified D-1257 rule).

The story should also record that T-71's test half (the CHANGELOG wildcard-wording control) is still pending on T-66's CHANGELOG scan. No test in `crates/` scans `CHANGELOG.md` yet.

**Routing:** state-manager, then story-writer.

## Observations (not counted)
- **`FIXED_TEXT_MAX_CHARS` vs `ShardConfigError`: no spec-vs-code contradiction, but the ADR is ambiguous.**
  - By my count, `EmptyArtifactPath`'s fixed parts total about 536 characters.
  - ADR §11(g)(2)(f)(3) lists what `ShardConfigError` adopts: one source, `Display` = 256, an exhaustive match. It does not list the (4)(c) bound, and the constant's doc scopes it to `BcIndexMigrationError`.
  - Latent interaction: `ShardCapConfigUnavailable` renders `Verbatim(sanitize_diagnostic(detail, cap*8))`. At cap 64 that is a 512 limit, which would cut a 536-character fixed text.
  - This is reachable only at cap 64 via `admission_error`, and that path is coordinator-only and unreachable.
  - The architect should state whether (4)(c) covers `ShardConfigError` and the composed detail.
- **Stale terminal-record archive:** besides the raw `path` field (L5-001), the stderr advisory is sanitized at 256, which is fine.

## [process-gap]
- **[process-gap]** For the fourth consecutive pass, a gate described as "provenance-keyed" is keyed on identifier lists:
  - the tracing gate uses `RAW_FIELDS` and `CONFIG_ERROR_PRODUCERS`;
  - the path gate uses `RECORD_IDENTS` and `RECORD_RECEIVERS`.

  The T-72 "Gate key" sentence changed the AC wording but did not change the gate design. A gate that must cover "every surface" needs default-deny (an allowlist of safe expression shapes), not a deny-list of names. Codify this as the test-writer rule.

## Deferred
- **D-1 — cross-story, routes to wave-gate.**
  - In `execute_canonical_path_moves`, the `tracing` fields `canonical`, `staging` and `reason` log values decoded from the record (`pending_canonical_moves[]`) raw and uncapped.
  - In the COMMITTING `ForwardRecovery` arm, the function renames record-supplied `staging_path` to `canonical_path` with no anchoring check in this build.
  - ADR-052 §11(f)(4) assigns the halt sites to S-25.11 AC-008, and §11(c) 7(e)(8) assigns plan-path validation to ADR-054 Decision 1.2.
  - Verify at wave-gate that S-25.11 and the ADR-054 owner cover both the sanitization and the anchoring.
- **D-2 — cross-story, routes to wave-gate.**
  - `ShardRetentionError` and `MechanismABackfillError` (S-25.02; BC-1.18.007/008) keep `#[error]` templates that interpolate the config-derived `artifact_stem` and an `io::Error` source raw into `HookResult::Error`.
  - `load_shard_index` wraps the toml parser message with `io::Error::other`.
  - I found no production caller of `load_shard_index_for_retention_check` or `run_mechanism_a_backfill_split` in `src/`, so this is latent.
  - ADR §11(g)(2)(f) says the rule is "operative for every error type whose rendering reaches an operator, hook-result or log surface"; the owner should decide whether these types adopt `render_with_cap`.
