# Demo Evidence Report — S-17.05

**Story:** S-17.05 — `stamp-state-timestamp` PostToolUse WASM hook  
**BC gate:** BC-4.17.001 v1.28 + BC-5.40.001 v1.21  
**ADR:** ADR-046  
**Status:** LOCAL 3-CLEAN convergence certified; PR-ready  
**Recorded:** 2026-08-29  

---

## Overview

S-17.05 implements a PostToolUse WASM hook that fires on every `Edit`/`Write`/`MultiEdit` to
`.factory/STATE.md`. It unconditionally re-stamps the `timestamp:` field to the current wall-clock
UTC instant (PC1) and, only when this session is the recorded `factory_lock.holder`, renews
`factory_lock.expires_at` to `now + TTL_SECONDS` (2700s) (PC2). Errors fail open (PC3). Line
endings are preserved byte-for-byte (PC4/Invariant 5). The hook is a non-interactive WASM plugin;
all demos use the real dispatcher + WASM runtime (`factory-dispatcher` + `stamp-state-timestamp.wasm`).

All demo scripts are in `docs/demo-evidence/S-17.05/demo-runner.sh`. Run from repo root:
```
bash docs/demo-evidence/S-17.05/demo-runner.sh <scenario>
```

---

## Recording Coverage Map

| Recording | ACs Demonstrated | BC Clauses | Description |
|-----------|-----------------|------------|-------------|
| [AC-001-002-timestamp-restamp.gif](#ac-001002-timestamp-restamp) | AC-001, AC-002 | BC-4.17.001 PC1, Invariant 1 | Timestamp unconditionally re-stamped on every qualifying write |
| [AC-003-006-identity-gate.gif](#ac-003006-identity-gate) | AC-003, AC-006 | BC-4.17.001 PC2, Invariant 2 | Self-held lock renewed; foreign holder NOT renewed (anti-resurrection) |
| [AC-008-fail-open.gif](#ac-008-fail-open) | AC-008 | BC-4.17.001 PC3 | Malformed frontmatter: 0 bytes written, agent write preserved |
| [AC-010-crlf-preservation.gif](#ac-010-crlf-preservation) | AC-010 | BC-4.17.001 PC4, Invariant 5 | CRLF line endings preserved byte-for-byte through re-stamp |
| [AC-011-013-registry.gif](#ac-011013-registry) | AC-011, AC-013 | BC-4.17.001 PC5, Preconditions 1-2 | Tool matcher excludes Bash/Agent; atomicity: new entry present + old guard absent |
| [AC-014-bats-suite.gif](#ac-014-bats-suite) | AC-006, AC-011, AC-013, AC-014 | BC-4.17.001 PC2/PC5/Preconditions; BC-5.40.001 PC4 | Real WASM dispatcher: all 4 bats integration tests pass |

---

## Individual Recording Details

### AC-001/002: Timestamp Restamp

**File:** `AC-001-002-timestamp-restamp.gif` / `.webm`  
**Tape:** `AC-001-002-timestamp-restamp.tape`  
**Scenario:** `demo-runner.sh ac001-restamp`

Demonstrates that `timestamp:` is unconditionally re-stamped from the stale fixture value
(`2020-01-01T00:00:00Z`) to the current wall-clock UTC instant after a PostToolUse Write fires.
No `factory_lock` block is present in this fixture. Shows the dispatcher trace confirming the
hook ran (`plugins_run=1 block_intent=false exit_code=0`).

**ACs covered:**
- AC-001 (BC-4.17.001 PC1): timestamp re-stamped when no lock block present
- AC-002 (BC-4.17.001 PC1/Invariant 1): timestamp re-stamping has no identity gate

---

### AC-003/006: Identity Gate

**File:** `AC-003-006-identity-gate.gif` / `.webm`  
**Tape:** `AC-003-006-identity-gate.tape`  
**Scenarios:** `demo-runner.sh ac003-self-renewal` then `demo-runner.sh ac006-foreign-no-renewal`

Two scenarios in sequence demonstrating the load-bearing PC2 identity gate:

**Part 1 (AC-003):** STATE.md fixture has `holder = $(git config user.email)` (this session)
with far-future `expires_at: 2099-01-01T00:45:00Z`. After the hook fires, `expires_at` is
advanced to `now + 2700s` (approximately 45 minutes from recording time) — BC-5.40.001 PC4
mid-burst TTL keep-alive mechanically satisfied.

**Part 2 (AC-006 — SAFETY-CRITICAL):** STATE.md fixture has `holder = foreign-holder@example.com`
(NOT this session). After the hook fires, `expires_at` remains byte-identical to the fixture
value. The foreign lock is NEVER silently resurrected. `timestamp:` IS re-stamped (PC1
unconditional, AC-002 cross-verification).

**ACs covered:**
- AC-003 (BC-4.17.001 PC2 row 1): identity match → expires_at renewed
- AC-006 (BC-4.17.001 PC2/Invariant 2): identity MISMATCH → expires_at NOT renewed

---

### AC-008: Fail-Open

**File:** `AC-008-fail-open.gif` / `.webm`  
**Tape:** `AC-008-fail-open.tape`  
**Scenario:** `demo-runner.sh ac008-fail-open`

STATE.md fixture has malformed frontmatter (no closing `---` delimiter). The hook fires,
reads the file, detects the structural error, and writes NOTHING. Byte count before and after
the hook invocation is identical. The agent's write is preserved intact (fail-open per PC3).
Dispatcher still exits 0 (PostToolUse hooks cannot block).

**ACs covered:**
- AC-008 (BC-4.17.001 PC3): structural frontmatter error → 0 bytes written, agent write preserved

---

### AC-010: CRLF Preservation

**File:** `AC-010-crlf-preservation.gif` / `.webm`  
**Tape:** `AC-010-crlf-preservation.tape`  
**Scenario:** `demo-runner.sh ac010-crlf`

STATE.md fixture uses CRLF (`\r\n`) line endings throughout (Windows-authored content). The
xxd hex dump of the first 80 bytes before and after the hook confirms `0d 0a` (CRLF) sequences
are preserved in both the frontmatter `---` delimiter lines and content lines. `timestamp:`
is re-stamped (PC1 unconditional). CRLF count: 5 sequences verified present after re-stamp.

**ACs covered:**
- AC-010 (BC-4.17.001 PC4/Invariant 5): CRLF line endings preserved byte-for-byte

---

### AC-011/013: Registry Shape + Atomicity

**File:** `AC-011-013-registry.gif` / `.webm`  
**Tape:** `AC-011-013-registry.tape`  
**Scenario:** `demo-runner.sh registry`

Pure registry-grep test against the production `plugins/vsdd-factory/hooks-registry.toml`.
No WASM or dispatcher required for these assertions.

**AC-011:** Shows the `tool = "^(Edit|Write|MultiEdit)$"` line for the `stamp-state-timestamp`
entry. Confirms `Bash` and `Agent` are absent from the matcher (PC5: hook must never intercept
`factory-lock-write.sh` acquire/release or CAS-push operations).

**AC-013:** Shows `stamp-state-timestamp` occurrence count ≥ 1 AND `verify-state-timestamp-refresh`
occurrence count = 0 in the same registry state — proving ADR-046 Decision 3 atomicity: the
old guard was deregistered in the same commit that registered the new stamper.

**ACs covered:**
- AC-011 (BC-4.17.001 PC5): tool matcher = `^(Edit|Write|MultiEdit)$` (no Bash, no Agent)
- AC-013 (BC-4.17.001 Preconditions 1-2 / ADR-046 Decision 3): registry atomicity verified

---

### AC-014: Bats Integration Suite

**File:** `AC-014-bats-suite.gif` / `.webm`  
**Tape:** `AC-014-bats-suite.tape`  
**Command:** `bats plugins/vsdd-factory/tests/stamp-state-timestamp.bats`

Records the complete bats integration suite running against the real WASM dispatcher. All 4
tests pass:

1. `test_stamp_state_timestamp_foreign_holder_write_never_renews_e2e` (AC-006 e2e)
2. `test_stamp_state_timestamp_registry_tool_matcher_excludes_bash_and_agent` (AC-011)
3. `test_hooks_registry_stamper_present_and_old_guard_absent_atomically` (AC-013)
4. `test_stamp_state_timestamp_mid_burst_renewal_e2e` (AC-014)

This recording proves the deployed WASM binary is wired correctly through the dispatcher
(real trigger/registry-wiring path validated — see S-17.04 lesson L-1704 on why native-env
unit tests alone are insufficient).

**ACs covered:**
- AC-006 (e2e): PostToolUse Write, foreign holder → expires_at byte-identical
- AC-011: registry tool matcher shape
- AC-013: registry atomicity (stamper present, old guard absent)
- AC-014 (BC-5.40.001 PC4): PostToolUse Write, self-holder → expires_at renewed to now+2700s

---

## AC Coverage Matrix

| AC | BC Clause | Demo Coverage |
|----|-----------|---------------|
| AC-001 | BC-4.17.001 PC1 | AC-001-002-timestamp-restamp.gif |
| AC-002 | BC-4.17.001 PC1/Invariant 1 | AC-001-002-timestamp-restamp.gif |
| AC-003 | BC-4.17.001 PC2 row 1 | AC-003-006-identity-gate.gif |
| AC-004 | BC-4.17.001 PC2 row 2 | AC-001-002-timestamp-restamp.gif (no lock block fixture) |
| AC-005 | BC-4.17.001 PC2 row 3 | Rust unit test `test_empty_holder_skips_renewal` |
| AC-006 | BC-4.17.001 PC2/Invariant 2 | AC-003-006-identity-gate.gif + AC-014-bats-suite.gif (e2e) |
| AC-007 | BC-4.17.001 PC2/PC3 | Rust unit test `test_identity_resolution_failure_skips_renewal_but_timestamp_still_restamped` |
| AC-008 | BC-4.17.001 PC3 | AC-008-fail-open.gif |
| AC-009 | BC-4.17.001 PC3 | Rust unit test `test_failure_then_success_is_independent_per_invocation` |
| AC-010 | BC-4.17.001 PC4/Invariant 5 | AC-010-crlf-preservation.gif |
| AC-011 | BC-4.17.001 PC5 | AC-011-013-registry.gif + AC-014-bats-suite.gif |
| AC-012 | BC-4.17.001 Precondition 3 | Rust unit test `test_ttl_seconds_constant_equals_2700` |
| AC-013 | BC-4.17.001 Preconditions 1-2 / ADR-046 Decision 3 | AC-011-013-registry.gif + AC-014-bats-suite.gif |
| AC-014 | BC-5.40.001 PC4 | AC-014-bats-suite.gif (real WASM e2e) |
| AC-015 | BC-4.17.001 PC3b | Rust unit tests (`test_renewal_indeterminate_*`) |
| AC-016 | BC-4.17.001 PC3a scoped exception | Rust unit test `test_no_timestamp_anchor_with_matching_lock_renews_expires_at` |
| AC-017 | BC-4.17.001 Precondition 1 | Rust unit test `test_failed_tool_write_skips_both_arms` |
| AC-018 | BC-4.17.001 Invariant 8 | Rust unit tests (`test_approaching_cap_*`, `test_at_cap_boundary_*`, `test_below_threshold_*`) |
| AC-019 | BC-4.17.001 PC1/EC-014 | Rust unit test `test_duplicate_timestamp_rewrites_first_emits_advisory` |

**VHS recordings cover:** AC-001, AC-002, AC-003, AC-004 (implicitly), AC-006, AC-008, AC-010, AC-011, AC-013, AC-014 (10/19 ACs directly; 4 more via bats suite)  
**Remaining ACs** (AC-005, AC-007, AC-009, AC-012, AC-015, AC-016, AC-017, AC-018, AC-019): covered by the 31 Rust unit tests in `crates/hook-plugins/stamp-state-timestamp/src/lib.rs` and `crates/factory-lock-parse/src/lib.rs` (all passing via `cargo test --workspace --all-targets`).

---

## Artifacts Index

```
docs/demo-evidence/S-17.05/
  demo-runner.sh                     — Shell script driver for all demo scenarios
  evidence-report.md                 — This file
  AC-001-002-timestamp-restamp.tape  — VHS tape source
  AC-001-002-timestamp-restamp.gif   — Recording: AC-001, AC-002
  AC-001-002-timestamp-restamp.webm  — Recording: AC-001, AC-002
  AC-003-006-identity-gate.tape      — VHS tape source
  AC-003-006-identity-gate.gif       — Recording: AC-003, AC-006
  AC-003-006-identity-gate.webm      — Recording: AC-003, AC-006
  AC-008-fail-open.tape              — VHS tape source
  AC-008-fail-open.gif               — Recording: AC-008
  AC-008-fail-open.webm              — Recording: AC-008
  AC-010-crlf-preservation.tape      — VHS tape source
  AC-010-crlf-preservation.gif       — Recording: AC-010
  AC-010-crlf-preservation.webm      — Recording: AC-010
  AC-011-013-registry.tape           — VHS tape source
  AC-011-013-registry.gif            — Recording: AC-011, AC-013
  AC-011-013-registry.webm           — Recording: AC-011, AC-013
  AC-014-bats-suite.tape             — VHS tape source
  AC-014-bats-suite.gif              — Recording: AC-006/AC-011/AC-013/AC-014 (real WASM bats)
  AC-014-bats-suite.webm             — Recording: AC-006/AC-011/AC-013/AC-014 (real WASM bats)
```
