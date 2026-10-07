# Demo Evidence Report — S-17.07

**Story:** S-17.07 — precompact-flush Step-4 identity-gate amendment (ADR-046 Decision 3)
**Branch:** feature/S-17.07
**BC gate:** BC-7.07.001 v1.40
**Demo strategy:** VHS terminal recordings of `cargo test -p precompact-flush` per-AC test filter
**Product type:** Non-UI Rust WASM hook — VHS used (no browser/GUI)
**POLICY 10:** All files are under `docs/demo-evidence/S-17.07/` (story-scoped subfolder)

---

## Coverage Map

| Recording | AC | BC-7.07.001 clause | Outcome branch | GIF | WebM | Tape |
|-----------|----|--------------------|----------------|-----|------|------|
| AC-001-already-expired-no-exec-subprocess | AC-001 | PC3 — already-expired arm | `SkipReason::AlreadyExpired`; `resolve_identity` NOT called (counter=0); original `expires_at`; flush proceeds | [gif](AC-001-already-expired-no-exec-subprocess.gif) | [webm](AC-001-already-expired-no-exec-subprocess.webm) | [tape](AC-001-already-expired-no-exec-subprocess.tape) |
| AC-002-identity-match-renews-content | AC-002 | PC3 — identity-match arm | `RenewOutcome::Renewed(new_content)`; flushed content uses `new_content` with `expires_at` advanced by `TTL_SECONDS` | [gif](AC-002-identity-match-renews-content.gif) | [webm](AC-002-identity-match-renews-content.webm) | [tape](AC-002-identity-match-renews-content.tape) |
| AC-003-not-holder-no-renewal | AC-003 | Invariant 3 — not-holder arm | `SkipReason::NotHolder`; flush proceeds; `expires_at` byte-identical | [gif](AC-003-not-holder-no-renewal.gif) | [webm](AC-003-not-holder-no-renewal.webm) | [tape](AC-003-not-holder-no-renewal.tape) |
| AC-004-resolution-failed-emits-event-and-logs | AC-004 | Invariant 3b — IdentityResolutionFailed arm | `SkipReason::IdentityResolutionFailed`; `factory.lock.renewal_indeterminate` event emitted with 5 payload fields; `host::log_warn` called; flush proceeds | [gif](AC-004-resolution-failed-emits-event-and-logs.gif) | [webm](AC-004-resolution-failed-emits-event-and-logs.webm) | [tape](AC-004-resolution-failed-emits-event-and-logs.tape) |
| AC-005-malformed-lock-emits-log-warn-no-exec | AC-005 | PC3 case 1 / EC-004 / Invariant 3 step 3 (primary); PC3 0th case / EC-009 (secondary) | Primary: `Err(LockError::Malformed(msg))` → `host::log_warn` MANDATORY (asserted called once), `resolve_identity` NOT invoked (counter=0), `expires_at` NOT written, flush proceeds. Secondary: `Ok((NoOp, None))` absent lock → `resolve_identity` NOT invoked, no event, flush proceeds | [gif](AC-005-malformed-lock-emits-log-warn-no-exec.gif) | [webm](AC-005-malformed-lock-emits-log-warn-no-exec.webm) | [tape](AC-005-malformed-lock-emits-log-warn-no-exec.tape) |

---

## Test Suite Summary

All 5 Red Gate tests pass under `cargo test -p precompact-flush step4_tests`:

| Test name | AC | Result |
|-----------|----|--------|
| `test_precompact_flush_step4_already_expired_no_exec_subprocess` | AC-001 | PASS |
| `test_precompact_flush_step4_identity_match_renews_content` | AC-002 | PASS |
| `test_precompact_flush_step4_not_holder_no_renewal` | AC-003 | PASS |
| `test_precompact_flush_step4_resolution_failed_emits_event_and_logs` | AC-004 | PASS |
| `test_precompact_flush_step4_malformed_lock_emits_log_warn_no_exec` | AC-005 | PASS |

---

## AC-001 — AlreadyExpired: no exec subprocess

**BC-7.07.001 clause:** PC3 — already-expired arm
**Outcome branch:** When `factory_lock.expires_at < now`, `renew_lock_if_holder` returns `SkipReason::AlreadyExpired`. The `resolve_identity` closure is NOT called (call counter asserted == 0, verified no `exec_subprocess` is issued). Flushed content retains the original `expires_at` byte-for-byte. Flush proceeds unblocked.

Recording: shows `cargo test -p precompact-flush step4_tests::test_precompact_flush_step4_already_expired_no_exec_subprocess -- --nocapture` passing (`ok`).

---

## AC-002 — Identity match: renews content

**BC-7.07.001 clause:** PC3 — identity-match arm
**Outcome branch:** When `resolve_identity` returns `IdentityResolution::Resolved(holder)` matching the lock's holder field, `renew_lock_if_holder` returns `RenewOutcome::Renewed(new_content)`. The flush MUST use `new_content` (with `expires_at` advanced by `TTL_SECONDS`). Test asserts flushed bytes differ from original and new `expires_at` is strictly greater than original.

Recording: shows `cargo test -p precompact-flush step4_tests::test_precompact_flush_step4_identity_match_renews_content -- --nocapture` passing (`ok`).

---

## AC-003 — NotHolder: flush proceeds, expires_at byte-identical

**BC-7.07.001 clause:** Invariant 3 — not-holder arm
**Outcome branch:** When `resolve_identity` returns `Resolved("other@example.com")` but the lock's `holder` is `"me@example.com"`, `renew_lock_if_holder` returns `SkipReason::NotHolder`. Flush proceeds with un-renewed content. `expires_at` is byte-identical to its pre-renewal value. No abort, no block.

Recording: shows `cargo test -p precompact-flush step4_tests::test_precompact_flush_step4_not_holder_no_renewal -- --nocapture` passing (`ok`).

---

## AC-004 — IdentityResolutionFailed: emit event + log_warn, flush proceeds

**BC-7.07.001 clause:** Invariant 3b — IdentityResolutionFailed arm
**Outcome branch:** When `resolve_identity` returns `IdentityResolution::Failed("test error")`, `renew_lock_if_holder` returns `SkipReason::IdentityResolutionFailed { reason, holder, locked_at, expires_at }`. The plugin:
1. Emits `factory.lock.renewal_indeterminate` via `host::emit_event` — asserted called once
2. Asserts all 5 payload fields present: `plugin`, `holder`, `locked_at`, `expires_at`, `resolution_error`
3. Emits `host::log_warn` — asserted called once
4. Flush proceeds unblocked with un-renewed content

Recording: shows `cargo test -p precompact-flush step4_tests::test_precompact_flush_step4_resolution_failed_emits_event_and_logs -- --nocapture` passing (`ok`).

---

## AC-005 — Malformed: mandatory log_warn (primary) + absent-lock 0th case (secondary)

**BC-7.07.001 clauses:** PC3 case 1 / EC-004 / Invariant 3 step 3 (primary); PC3 0th case / EC-009 (secondary)
**Outcome branches:**

**Primary (Malformed arm):** `Err(LockError::Malformed(msg))` — `host::log_warn` is **MANDATORY** (SHALL per BC-7.07.001). Test asserts:
- `host::log_warn` called exactly once (not zero — emission is non-optional)
- `resolve_identity` closure NOT invoked (call counter == 0)
- `host::write_file` NOT called for `expires_at`
- Flush proceeds unblocked (return value indicates success)

**Secondary (0th case / absent lock):** `Ok((RenewOutcome::NoOp, None))` when `factory_lock:` is absent from STATE.md. Test asserts:
- `resolve_identity` NOT invoked
- No event emitted via `host::emit_event`
- Flush proceeds unblocked

Recording: shows `cargo test -p precompact-flush step4_tests::test_precompact_flush_step4_malformed_lock_emits_log_warn_no_exec -- --nocapture` passing (`ok`).

---

## File Index

```
docs/demo-evidence/S-17.07/
├── evidence-report.md                                  (this file)
├── AC-001-already-expired-no-exec-subprocess.tape
├── AC-001-already-expired-no-exec-subprocess.gif
├── AC-001-already-expired-no-exec-subprocess.webm
├── AC-002-identity-match-renews-content.tape
├── AC-002-identity-match-renews-content.gif
├── AC-002-identity-match-renews-content.webm
├── AC-003-not-holder-no-renewal.tape
├── AC-003-not-holder-no-renewal.gif
├── AC-003-not-holder-no-renewal.webm
├── AC-004-resolution-failed-emits-event-and-logs.tape
├── AC-004-resolution-failed-emits-event-and-logs.gif
├── AC-004-resolution-failed-emits-event-and-logs.webm
├── AC-005-malformed-lock-emits-log-warn-no-exec.tape
├── AC-005-malformed-lock-emits-log-warn-no-exec.gif
└── AC-005-malformed-lock-emits-log-warn-no-exec.webm
```

Total: 16 files (1 report + 5 tapes + 5 gifs + 5 webms)
POLICY 10 compliance: all files under `docs/demo-evidence/S-17.07/` — no flat files at `docs/demo-evidence/*.md`
