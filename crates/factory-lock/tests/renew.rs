// Test code: expect(), unwrap(), and panic!() are acceptable per AC-010 (non-test code only).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Red Gate tests for `factory-lock::renew_lock` (S-18.04a T-1).
//!
//! All tests in this file correspond to rows in the Red Gate Test Table in
//! `S-18.04a-precompact-flush-sh-core.md`. All must FAIL against stubs
//! (todo!() bodies); they will pass only after the implementer completes T-4.
//!
//! # BC / ADR traces
//!
//! - BC-7.07.001 PC3 + ADR-028 §Decision 2 (native lock renewal)
//! - ADR-028 §Decision 9 F-NW2-005/006 (path-based form struck; presence pre-check)
//! - ADR-028 §Decision 9 F-NW2-004 (Err(Malformed) not Ok(None) when key present+malformed)
//! - ADR-028 §Decision 9 F-NW-005 (no-frontmatter → NoOp, no advisory)
//! - ADR-028 §Decision 9 F-NW-008 (expires_at YYYY-MM-DDTHH:MM:SSZ exact format)
//! - ADR-028 §Decision 14 F-R3-002 (malformed-fence + lock key in body → Err(Malformed))
//! - ADR-028 §Decision 16 F-R3-005 (byte-identical expires_at → NoOp)
//! - BC-5.40.001 §Invariant 2 (expires_at Z-suffix second-precision format)

use factory_lock::{LockError, RenewOutcome, renew_lock, renew_lock_with_now};

// ---------------------------------------------------------------------------
// Helpers — shared STATE.md frontmatter fixtures
// ---------------------------------------------------------------------------

const LOCK_HELD_TEMPLATE: &str = r#"---
current_cycle: test-cycle
current_step: stub-step
factory_lock:
  holder: agent@example.com
  locked_at: 2026-06-01T10:00:00Z
  expires_at: {expires_at}
---

# STATE.md body content
"#;

const LOCK_ABSENT_TEMPLATE: &str = r#"---
current_cycle: test-cycle
current_step: stub-step
---

# STATE.md body content — no factory_lock key
"#;

const NO_FRONTMATTER: &str = r#"# STATE.md with no YAML frontmatter at all

Just prose, no --- fences.
"#;

const MALFORMED_LOCK_HELD: &str = r#"---
current_cycle: test-cycle
current_step: stub-step
factory_lock:
  holder: agent@example.com
  locked_at: 2026-06-01T10:00:00Z
---

# STATE.md body content — factory_lock: present, expires_at missing → Malformed
"#;

/// Malformed fence (opening `---` but no closing `---`) WITH `factory_lock:` key
/// in the body region. Per ADR-028 §Decision 14 F-R3-002, awk open-region semantics
/// mean the key is considered "found" inside the fence → `Err(Malformed)`.
const MALFORMED_FENCE_WITH_LOCK_KEY: &str = r#"---
current_cycle: test-cycle
factory_lock:
  holder: agent@example.com
  locked_at: 2026-06-01T10:00:00Z
  expires_at: 2026-06-01T10:45:00Z
# No closing --- fence — body continues
More content here
"#;

/// Malformed fence WITHOUT `factory_lock:` key — per ADR-028 §Decision 9 F-NW2-006,
/// the presence pre-check short-circuits before checking fence shape → `Ok(NoOp)`.
const MALFORMED_FENCE_NO_LOCK_KEY: &str = r#"---
current_cycle: test-cycle
current_step: stub-step
# No closing --- fence, no factory_lock: key
More content here
"#;

// ---------------------------------------------------------------------------
// Red Gate: AC-018 — renew_lock updates expires_at only (holder + locked_at preserved)
// ---------------------------------------------------------------------------

/// Red Gate: test_renew_lock_updates_expires_at_only
///
/// Traces to: AC-018 / ADR-028 §Decision 2 / BC-5.40.001 INV2 (TTL=2700s; holder+locked_at preserved)
///
/// Given a STATE.md with a held lock (expires_at in the past), `renew_lock()` MUST
/// return `Ok(RenewOutcome::Renewed(new_content))` where:
/// - The `expires_at` field is updated to `now + 2700s` in YYYY-MM-DDTHH:MM:SSZ format.
/// - The `holder` and `locked_at` fields are unchanged.
/// - The rest of the STATE.md content is byte-for-byte preserved.
#[test]
fn test_renew_lock_updates_expires_at_only() {
    let old_expires = "2020-01-01T00:00:00Z"; // well in the past
    let content = LOCK_HELD_TEMPLATE.replace("{expires_at}", old_expires);

    let result = renew_lock(&content);

    let RenewOutcome::Renewed(new_content) = result.expect("expected Renewed") else {
        panic!("expected Renewed but got NoOp");
    };

    // holder and locked_at must be preserved
    assert!(
        new_content.contains("  holder: agent@example.com"),
        "holder must be preserved"
    );
    assert!(
        new_content.contains("  locked_at: 2026-06-01T10:00:00Z"),
        "locked_at must be preserved"
    );

    // new expires_at must differ from old (it should be ~now + 2700s)
    assert!(
        !new_content.contains(&format!("  expires_at: {old_expires}")),
        "expires_at must be updated, not old value"
    );

    // expires_at must be in YYYY-MM-DDTHH:MM:SSZ format (no sub-seconds, no +00:00)
    let expires_line = new_content
        .lines()
        .find(|l| l.trim_start().starts_with("expires_at:"))
        .expect("expires_at line must be present");
    let expires_val = expires_line
        .split(':')
        .skip(1)
        .collect::<Vec<_>>()
        .join(":")
        .trim()
        .to_string();
    assert!(
        expires_val.ends_with('Z'),
        "expires_at must end with uppercase Z, got: {expires_val}"
    );
    assert!(
        !expires_val.contains('+'),
        "expires_at must not contain +00:00 (rfc3339 form forbidden), got: {expires_val}"
    );
    assert!(
        !expires_val.contains('.'),
        "expires_at must not contain sub-seconds, got: {expires_val}"
    );
}

// ---------------------------------------------------------------------------
// Red Gate: AC-018 — renew_lock returns NoOp when factory_lock key absent
// ---------------------------------------------------------------------------

/// Red Gate: test_renew_lock_noop_when_absent
///
/// Traces to: BC-7.07.001 PC3 + ADR-028 §Decision 2 (no-op when factory_lock: absent)
///
/// When STATE.md has no `factory_lock:` key in the frontmatter, `renew_lock()` MUST
/// return `Ok(RenewOutcome::NoOp)`. No write_file call is made.
#[test]
fn test_renew_lock_noop_when_absent() {
    let result = renew_lock(LOCK_ABSENT_TEMPLATE).expect("must not error on absent lock");
    assert!(
        matches!(result, RenewOutcome::NoOp),
        "expected NoOp when factory_lock: is absent"
    );
}

// ---------------------------------------------------------------------------
// Red Gate: AC-018 — renew_lock returns Err(Malformed) when key present + block malformed
// ---------------------------------------------------------------------------

/// Red Gate: test_renew_lock_malformed_returns_err_not_ok
///
/// Traces to: ADR-028 §Decision 2 F-NW-004 (library returns Err on malformed block,
/// not Ok(None); caller downgrades Err to advisory)
///
/// When STATE.md has `factory_lock:` key but the block is malformed (missing
/// `expires_at`), `renew_lock()` MUST return `Err(LockError::Malformed)`, NOT
/// `Ok(None)` or `Ok(RenewOutcome::NoOp)`.
#[test]
fn test_renew_lock_malformed_returns_err_not_ok() {
    let result = renew_lock(MALFORMED_LOCK_HELD);
    assert!(
        matches!(result, Err(LockError::Malformed(_))),
        "expected Err(Malformed) when factory_lock: key present but block malformed, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Red Gate: AC-018 — no-frontmatter STATE.md returns NoOp
// ---------------------------------------------------------------------------

/// Red Gate: test_renew_no_frontmatter_skips_noop
///
/// Traces to: ADR-028 §Decision 2 F-NW-005 (no-frontmatter STATE.md → Ok(NoOp) skip,
/// no advisory emitted)
///
/// When STATE.md has no YAML frontmatter fences at all, `renew_lock()` MUST return
/// `Ok(RenewOutcome::NoOp)` without emitting any error or advisory.
#[test]
fn test_renew_no_frontmatter_skips_noop() {
    let result = renew_lock(NO_FRONTMATTER).expect("must not error on no-frontmatter input");
    assert!(
        matches!(result, RenewOutcome::NoOp),
        "expected NoOp when STATE.md has no frontmatter"
    );
}

// ---------------------------------------------------------------------------
// Red Gate: AC-018 — expires_at format must be Z-suffix second-precision
// ---------------------------------------------------------------------------

/// Red Gate: test_expires_at_format_is_z_suffix_second_precision
///
/// Traces to: ADR-028 §Decision 2 F-NW-008 (expires_at MUST be YYYY-MM-DDTHH:MM:SSZ;
/// NOT chrono to_rfc3339() +00:00 or sub-second form) / BC-5.40.001 §Invariant 2
///
/// Verifies that the renewed `expires_at` uses exactly the format produced by
/// `chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ")` — no `+00:00`, no fractional
/// seconds, uppercase `Z` suffix.
#[test]
fn test_expires_at_format_is_z_suffix_second_precision() {
    let old_expires = "2020-01-01T00:00:00Z";
    let content = LOCK_HELD_TEMPLATE.replace("{expires_at}", old_expires);

    let RenewOutcome::Renewed(new_content) = renew_lock(&content).expect("expected Renewed") else {
        panic!("expected Renewed");
    };

    let expires_line = new_content
        .lines()
        .find(|l| l.trim_start().starts_with("expires_at:"))
        .expect("expires_at line must exist");
    let raw_val = expires_line
        .trim_start()
        .strip_prefix("expires_at:")
        .expect("must have expires_at:")
        .trim();

    // Must match YYYY-MM-DDTHH:MM:SSZ exactly (20 chars)
    assert_eq!(
        raw_val.len(),
        20,
        "expires_at must be 20 chars (YYYY-MM-DDTHH:MM:SSZ), got: {raw_val}"
    );
    assert!(
        raw_val.ends_with('Z'),
        "must end with uppercase Z, got: {raw_val}"
    );
    assert!(
        !raw_val.contains('+'),
        "must not contain +00:00, got: {raw_val}"
    );
    assert!(
        !raw_val.contains('.'),
        "must not contain sub-seconds, got: {raw_val}"
    );
}

// ---------------------------------------------------------------------------
// Red Gate: AC-018 — malformed fence WITH lock key → Err(Malformed) (bash parity)
// ---------------------------------------------------------------------------

/// Red Gate: test_malformed_fence_lock_key_in_body_matches_bash
///
/// Traces to: ADR-028 §Decision 14 F-R3-002 (opening fence, no closing fence,
/// factory_lock: key present in body region → has_factory_lock_key() returns true
/// → Err(Malformed) → caller emits advisory; bash parity — awk open-region semantics)
///
/// When STATE.md has an opening `---` fence but NO closing `---` fence AND
/// `factory_lock:` key appears in the body region, `renew_lock()` MUST return
/// `Err(LockError::Malformed)` (not `Ok(NoOp)`) — bash parity for awk open-region.
#[test]
fn test_malformed_fence_lock_key_in_body_matches_bash() {
    let result = renew_lock(MALFORMED_FENCE_WITH_LOCK_KEY);
    assert!(
        matches!(result, Err(LockError::Malformed(_))),
        "expected Err(Malformed) for malformed-fence + lock key in body, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Red Gate: AC-018 — malformed fence WITHOUT lock key → NoOp (bash parity)
// ---------------------------------------------------------------------------

/// Red Gate: test_renew_lock_malformed_fence_no_lock_key_returns_noop
///
/// Traces to: ADR-028 §Decision 9 F-NW2-006 (malformed fence + no factory_lock: key
/// → Ok(RenewOutcome::NoOp); bash parity; NO Err(Malformed))
///
/// When STATE.md has a malformed fence (no closing `---`) but NO `factory_lock:` key,
/// `renew_lock()` MUST return `Ok(RenewOutcome::NoOp)` — the presence pre-check
/// short-circuits before checking fence shape (bash parity).
#[test]
fn test_renew_lock_malformed_fence_no_lock_key_returns_noop() {
    let result = renew_lock(MALFORMED_FENCE_NO_LOCK_KEY).expect("must not error when no lock key");
    assert!(
        matches!(result, RenewOutcome::NoOp),
        "expected NoOp for malformed-fence without lock key, got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Red Gate: AC-018 — byte-identical expires_at returns NoOp (spurious renewal guard)
// ---------------------------------------------------------------------------

/// Red Gate: test_renew_noop_on_byte_identical_expires_at
///
/// Traces to: ADR-028 §Decision 16 F-R3-005 (recomputed expires_at byte-identical
/// to existing expires_at → Ok(RenewOutcome::NoOp); no write_file call;
/// spurious renewal suppressed)
///
/// Uses `renew_lock_with_now` to inject a fixed "now" clock so the test is
/// deterministic regardless of wall-clock time.
///
/// The fixture sets `expires_at = now + 2700s` (computed from the injected now).
/// `renew_lock_with_now` must detect the byte-identical recomputation and return
/// `Ok(RenewOutcome::NoOp)` — the spurious renewal guard (F-R3-005).
///
/// Stub will return todo!() — Red Gate fails (panics).
#[test]
fn test_renew_noop_on_byte_identical_expires_at() {
    use chrono::{Duration, TimeZone, Utc};

    // Fixed "now" timestamp: 2026-06-22T12:00:00Z
    let fixed_now = Utc.with_ymd_and_hms(2026, 6, 22, 12, 0, 0).unwrap();
    // expires_at = now + 2700s = 2026-06-22T12:45:00Z (exactly matching what renew would compute)
    let expected_expires_at = (fixed_now + Duration::seconds(2700))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    assert_eq!(
        expected_expires_at, "2026-06-22T12:45:00Z",
        "test fixture sanity: expected_expires_at must be 2700s after fixed_now"
    );

    // Build STATE.md with expires_at = exact byte-identical value
    let content = LOCK_HELD_TEMPLATE.replace("{expires_at}", &expected_expires_at);

    // Call renew_lock_with_now with injected clock returning fixed_now
    let result = renew_lock_with_now(&content, move || fixed_now);

    // F-R3-005: byte-identical expires_at must produce NoOp (no spurious renewal)
    assert!(
        matches!(result, Ok(RenewOutcome::NoOp)),
        "F-R3-005: byte-identical expires_at must return Ok(NoOp); \
        recomputed expires_at == existing expires_at → spurious renewal suppressed; \
        got: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// F-S1705-P7-001: CRLF-preserving renew_lock_with_now
// (EC-017 / BC-4.17.001 PC4/AC-010 Invariant 5)
// ---------------------------------------------------------------------------

/// test_renew_lock_with_now_crlf_preserves_all_terminators
///
/// Traces to: F-S1705-P7-001 / EC-017 / BC-4.17.001 PC4/AC-010 Invariant 5.
///
/// `renew_lock_with_now` on a fully-CRLF STATE.md MUST:
///   (a) renew the `expires_at` VALUE to `now + 2700s` (returns `Renewed`);
///   (b) preserve `\r\n` on the rewritten `expires_at` line;
///   (c) leave every other line's `\r\n` terminator byte-for-byte unchanged
///       (no bare `\n` introduced anywhere);
///   (d) change exactly one line — the `expires_at` line — and nothing else.
///
/// MUTATION-KILL A (terminator preservation): revert `rewrite_expires_at` to emit
/// bare `\n` instead of the detected `\r\n` → assertion (b) fails immediately.
///
/// MUTATION-KILL B (whole-file normalization): insert a `replace("\r\n", "\n")` pass
/// before the line-by-line rewrite → body lines lose `\r` → assertion (c) fails.
///
/// The companion LF baseline is covered by `test_renew_lock_updates_expires_at_only`
/// (above), which uses an LF-only `LOCK_HELD_TEMPLATE` and asserts the rewritten
/// content retains LF terminators — no new LF test needed here.
#[test]
fn test_renew_lock_with_now_crlf_preserves_all_terminators() {
    use chrono::{Duration, TimeZone, Utc};

    let old_expires = "2020-01-01T00:00:00Z";
    // TTL = 2700 s (factory_lock_parse::TTL_SECONDS canonical value; literal used in
    // integration tests by convention — see test_renew_noop_on_byte_identical_expires_at).
    let ttl_seconds: i64 = 2700;

    let fixed_now = Utc.with_ymd_and_hms(2026, 8, 27, 12, 0, 0).unwrap();
    let expected_expires = (fixed_now + Duration::seconds(ttl_seconds))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    // Sanity: fixed_now + 2700 s = 2026-08-27T12:45:00Z
    assert_eq!(
        expected_expires, "2026-08-27T12:45:00Z",
        "test fixture sanity check failed"
    );

    // CRLF STATE.md — every line ends with \r\n, including fences and body.
    // Written as a single format-string (no backslash continuation) so the
    // two-space indent on the factory_lock sub-fields is preserved exactly.
    let crlf_fixture = format!(
        "---\r\ncurrent_cycle: test-cycle\r\ncurrent_step: stub-step\r\nfactory_lock:\r\n  holder: agent@example.com\r\n  locked_at: 2026-06-01T10:00:00Z\r\n  expires_at: {old_expires}\r\n---\r\n\r\n# STATE.md body content\r\n"
    );

    // Fixture sanity: every LF must be preceded by \r.
    for (i, segment) in crlf_fixture.split_inclusive('\n').enumerate() {
        if !segment.ends_with('\n') {
            continue; // last chunk with no trailing LF is OK
        }
        assert!(
            segment.ends_with("\r\n"),
            "crlf_fixture line {} must end with \\r\\n; got: {:?}",
            i,
            segment
        );
    }

    let result = renew_lock_with_now(&crlf_fixture, move || fixed_now);

    let new_content = match result {
        Ok(RenewOutcome::Renewed(c)) => c,
        other => panic!(
            "F-S1705-P7-001: CRLF fixture with stale expires_at must return Renewed, got: {:?}",
            other
        ),
    };

    // (a) The expires_at VALUE was renewed to now + 2700 s.
    assert!(
        new_content.contains(&format!("  expires_at: {expected_expires}")),
        "F-S1705-P7-001 (a): expires_at must be renewed to {expected_expires}. \
         Got:\n{new_content:?}"
    );
    assert!(
        !new_content.contains(&format!("  expires_at: {old_expires}")),
        "F-S1705-P7-001 (a): stale expires_at ({old_expires}) must be gone. \
         Got:\n{new_content:?}"
    );

    // (b) The rewritten expires_at line ends with \r\n (CRLF terminator preserved).
    // MUTATION-KILL A: reverting the terminator detection so \n is emitted instead
    // of \r\n → this assertion fails.
    let expires_line = new_content
        .split_inclusive('\n')
        .find(|seg| {
            let trimmed = seg.trim_end_matches('\n').trim_end_matches('\r');
            trimmed.starts_with("  expires_at:")
        })
        .expect("F-S1705-P7-001: expires_at line must exist in renewed content");
    assert!(
        expires_line.ends_with("\r\n"),
        "F-S1705-P7-001 (b): rewritten expires_at line must end with \\r\\n (CRLF), \
         not bare \\n. Got segment: {:?}",
        expires_line
    );

    // (c) No bare \n introduced — every LF still preceded by \r.
    // MUTATION-KILL B: whole-file \r\n→\n normalization before rewrite → body lines
    // become bare-LF → this assertion fails on the first body segment.
    for (i, segment) in new_content.split_inclusive('\n').enumerate() {
        if !segment.ends_with('\n') {
            continue;
        }
        assert!(
            segment.ends_with("\r\n"),
            "F-S1705-P7-001 (c): line {} must still end with \\r\\n after CRLF renewal; \
             got bare \\n: {:?}",
            i,
            segment
        );
    }

    // (d) Exactly one line differs between input and output — the expires_at line.
    let in_segments: Vec<&str> = crlf_fixture.split_inclusive('\n').collect();
    let out_segments: Vec<&str> = new_content.split_inclusive('\n').collect();
    assert_eq!(
        in_segments.len(),
        out_segments.len(),
        "F-S1705-P7-001 (d): CRLF renewal must not change the segment count (line count)"
    );
    let differing: Vec<usize> = in_segments
        .iter()
        .zip(out_segments.iter())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        differing.len(),
        1,
        "F-S1705-P7-001 (d): exactly 1 line must change (the expires_at line); \
         {} lines differed at indices {:?}",
        differing.len(),
        differing
    );
    let changed_segment = out_segments[differing[0]];
    let changed_trimmed = changed_segment
        .trim_end_matches('\n')
        .trim_end_matches('\r');
    assert!(
        changed_trimmed.starts_with("  expires_at:"),
        "F-S1705-P7-001 (d): the one changed line must be the expires_at line, \
         got: {:?}",
        changed_segment
    );
}
