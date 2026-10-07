# Demo Evidence Report — S-17.06

**Story:** S-17.06 — factory-lock shared functions (renew_lock_if_holder, IdentityResolution, SkipReason, classify_identity_resolution, trim_git_email promotion + doc-comment corrections)
**BC:** BC-4.17.001 v1.27
**Product type:** Library (Rust) — VHS terminal recordings of `cargo test` runs
**Recording method:** VHS (real terminal capture); `Sleep`-based timing (VHS `Wait+Line` unavailable in this environment due to zsh prompt mismatch)
**POLICY 10 scoping:** All artifacts under `docs/demo-evidence/S-17.06/` — no flat files at `docs/demo-evidence/`

---

## Coverage Matrix

| AC | Title | Evidence artifact | Tests demonstrated | Result |
|----|-------|-------------------|--------------------|--------|
| AC-001 | `renew_lock_if_holder` 6-case decision tree | `AC-001-renew-lock-6-case-decision-tree.gif` / `.webm` | `test_renew_lock_if_holder_absent_block_no_op`, `test_renew_lock_if_holder_malformed_no_resolve`, `test_renew_lock_if_holder_already_expired_no_resolve`, `test_renew_lock_if_holder_not_holder_no_renewal`, `test_renew_lock_if_holder_identity_resolution_failed_no_renewal`, `test_renew_lock_if_holder_identity_match_renewed` (+ 2 mutation-kill tests) | 8/8 passed |
| AC-002 | Lazy `resolve_identity` — called at most once, never on cases 0/1/2 | `AC-002-lazy-resolve-identity-at-most-once.gif` / `.webm` | `test_resolve_identity_called_at_most_once` | 1/1 passed |
| AC-003 | `SkipReason::IdentityResolutionFailed` carries 4 fields from parsed LockState | `AC-003-skip-reason-four-fields.gif` / `.webm` | `test_skip_reason_identity_resolution_failed_carries_four_fields` | 1/1 passed |
| AC-004 | `classify_identity_resolution` 4-shape rule | `AC-004-classify-identity-4-shapes.gif` / `.webm` | `test_classify_identity_resolution_exec_error_maps_failed`, `test_classify_identity_resolution_nonzero_exit_maps_failed`, `test_classify_identity_resolution_empty_stdout_maps_failed`, `test_classify_identity_resolution_nonempty_stdout_maps_resolved` | 4/4 passed |
| AC-005 | `trim_git_email` single canonical home + verify-factory-lock delegation | `AC-005-trim-git-email-canonical-home.gif` / `.webm` | `grep -rn '^pub fn trim_git_email' crates/` (1 hit only), `test_trim_git_email_canonical_in_factory_lock`, `test_verify_factory_lock_delegates_trim_git_email` | 2/2 passed; grep = 1 definition |
| AC-006 | Corrected doc-comments (3 loci, post-F-P56-001 semantics) | `AC-006-doc-comment-corrections.gif` / `.webm` | `grep -n 'F-P56-001'` shows all 3 loci in `crates/factory-lock/src/lib.rs`; `grep -A1 'Block absent or fully-null'` shows corrected inline comment | No runtime test (doc-only); grep evidence shown |

---

## Artifact Index

| File | Type | AC | Description |
|------|------|----|-------------|
| `AC-001-renew-lock-6-case-decision-tree.tape` | VHS source | AC-001 | 6-case decision tree tape script |
| `AC-001-renew-lock-6-case-decision-tree.gif` | Recording | AC-001 | `cargo test -p factory-lock test_renew_lock_if_holder` — 8 tests pass |
| `AC-001-renew-lock-6-case-decision-tree.webm` | Recording | AC-001 | Same, webm format |
| `AC-002-lazy-resolve-identity-at-most-once.tape` | VHS source | AC-002 | Lazy identity tape script |
| `AC-002-lazy-resolve-identity-at-most-once.gif` | Recording | AC-002 | `test_resolve_identity_called_at_most_once` passes |
| `AC-002-lazy-resolve-identity-at-most-once.webm` | Recording | AC-002 | Same, webm format |
| `AC-003-skip-reason-four-fields.tape` | VHS source | AC-003 | SkipReason struct-fields tape script |
| `AC-003-skip-reason-four-fields.gif` | Recording | AC-003 | `test_skip_reason_identity_resolution_failed_carries_four_fields` passes |
| `AC-003-skip-reason-four-fields.webm` | Recording | AC-003 | Same, webm format |
| `AC-004-classify-identity-4-shapes.tape` | VHS source | AC-004 | 4-shape classifier tape script |
| `AC-004-classify-identity-4-shapes.gif` | Recording | AC-004 | 4 `test_classify_identity_resolution_*` tests pass |
| `AC-004-classify-identity-4-shapes.webm` | Recording | AC-004 | Same, webm format |
| `AC-005-trim-git-email-canonical-home.tape` | VHS source | AC-005 | Canonical home + delegation tape script |
| `AC-005-trim-git-email-canonical-home.gif` | Recording | AC-005 | grep shows 1 definition; 2 tests pass |
| `AC-005-trim-git-email-canonical-home.webm` | Recording | AC-005 | Same, webm format |
| `AC-006-doc-comment-corrections.tape` | VHS source | AC-006 | Doc-comment evidence tape script |
| `AC-006-doc-comment-corrections.gif` | Recording | AC-006 | grep shows F-P56-001 in all 3 loci + corrected inline comment |
| `AC-006-doc-comment-corrections.webm` | Recording | AC-006 | Same, webm format |

---

## Key Commands (captured output)

### AC-001 — 6-case decision tree
```
cargo test -p factory-lock test_renew_lock_if_holder 2>&1 | grep -E 'test |test result'
test tests::test_renew_lock_if_holder_absent_block_no_op ... ok
test tests::test_renew_lock_if_holder_malformed_expires_at_returns_err ... ok
test tests::test_renew_lock_if_holder_now_equals_expires_at_is_expired ... ok
test tests::test_renew_lock_if_holder_malformed_no_resolve ... ok
test tests::test_renew_lock_if_holder_not_holder_no_renewal ... ok
test tests::test_renew_lock_if_holder_identity_resolution_failed_no_renewal ... ok
test tests::test_renew_lock_if_holder_already_expired_no_resolve ... ok
test tests::test_renew_lock_if_holder_identity_match_renewed ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.00s
```

### AC-005 — Single canonical home (grep)
```
grep -rn '^pub fn trim_git_email' crates/
crates/factory-lock/src/lib.rs:545:pub fn trim_git_email(raw: &str) -> String {
```
One result only. `verify-factory-lock` contains no `pub fn trim_git_email` definition — only a delegation test that asserts no local body exists.

---

## AC-006 Static Evidence

AC-006 has no Red Gate test (doc-comment-only AC). Evidence is grep-based:

**Locus 1 (renew_lock_with_now algorithm doc):**
Lines 123–126 of `crates/factory-lock/src/lib.rs`:
```
///    - `Ok(None)` — block absent or fully-null → `Ok(RenewOutcome::NoOp)`.
///      NOTE: per F-P56-001, `Ok(None)` is returned ONLY when the block is absent
///      or fully-null (null-value holder); an empty/absent holder with sibling fields
///      present returns `Err(MalformedLockBlock)` instead.
```

**Locus 2 (Ok(None) arm inline comment in renew_lock_with_now body):**
Lines 171–172:
```rust
        Ok(None) => {
            // Block absent or fully-null → NoOp (F-P56-001).
            // Empty/absent holder with siblings present routes to Err(Malformed) instead.
```

**Locus 3 (parse_lock doc):**
Lines 332–336:
```
/// - `Ok(None)` — block absent or fully-null (per F-P56-001: `Ok(None)` is returned
///   ONLY for absent-or-fully-null block; empty/absent holder with sibling fields
///   present returns `Err(Malformed)` instead).
```

All three loci correctly reflect post-F-P56-001 semantics.
