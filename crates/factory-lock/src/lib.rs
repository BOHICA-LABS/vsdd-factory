//! factory-lock — pure library crate for factory_lock CRUD on STATE.md frontmatter.
//!
//! Exports:
//!   - [`RenewOutcome`] — result of `renew_lock()`: NoOp (absent or no-op) or
//!     Renewed(String) (new full STATE.md content with updated expires_at).
//!   - [`LockError`] — error variants for lock operations.
//!   - [`FactoryLock`] — parsed representation of a held factory lock.
//!   - [`renew_lock(state_md_content: &str) -> Result<RenewOutcome, LockError>`]
//!     — pure content-in/content-out renewal; no std::fs; WASM-hermetic.
//!   - [`renew_lock_if_holder`] — identity-gated renewal (6-case decision tree,
//!     BC-4.17.001 PC2 / ADR-046 Decision 1(b); S-17.06).
//!   - [`IdentityResolution`] — result of resolving caller git identity for
//!     lock renewal (Resolved(email) or Failed(reason); S-17.06).
//!   - [`SkipReason`] — reason a `renew_lock_if_holder` call was skipped
//!     (NotHolder, AlreadyExpired, or IdentityResolutionFailed; S-17.06).
//!   - [`classify_identity_resolution`] — pure 4-shape classifier for
//!     `git config user.email` subprocess results (S-17.06).
//!   - [`trim_git_email`] — canonical home for trimming trailing whitespace from
//!     git subprocess stdout (F-P7-001 single-canonical-home; S-17.06).
//!   - [`acquire_lock`] — stub (S-18.04b scope).
//!   - [`clear_lock`] — stub (S-18.04b scope).
//!
//! # Architecture compliance (ADR-028)
//!
//! - Pure library: no `std::fs`, no host I/O, no `exec_subprocess`. WASM-hermetic.
//! - Dependency: factory-lock → factory-lock-parse; never the reverse (no cycles).
//! - No `[[bin]]` target in this crate (forbidden per story Forbidden Dependencies).
//! - `renew_lock()` performs a `factory_lock:` key presence pre-check BEFORE calling
//!   `parse_factory_lock()` — bash parity (F-NW2-006 / ADR-028 §Decision 9).
//! - Path-based `renew_lock(&Path)` form is STRUCK (F-NW2-005); only the pure
//!   content-in/content-out form `renew_lock(state_md_content: &str)` is acceptable.
//! - `expires_at` MUST be formatted as `YYYY-MM-DDTHH:MM:SSZ` (UTC, second precision,
//!   uppercase Z suffix) using `chrono::format("%Y-%m-%dT%H:%M:%SZ")` — NOT
//!   `to_rfc3339()` (AC-018 F-NW-008 / BC-5.40.001 §Invariant 2).
//! - CRLF-preserving output: `parse_factory_lock()` normalizes CRLF internally for its
//!   own parsing, but the input content is passed untouched to `rewrite_expires_at`,
//!   which preserves every line's original terminator (`\r\n` or `\n`) byte-for-byte.
//!   Only the `expires_at` line value is rewritten; its original terminator is mirrored
//!   (EC-017 CRLF input class / BC-4.17.001 PC4/AC-010 Invariant 5).
//! - `Err(LockError::Malformed)` ONLY when `factory_lock:` key IS present AND block
//!   is malformed. Malformed fence WITHOUT the lock key → `Ok(RenewOutcome::NoOp)`.

// Allow the BC-based test naming convention workspace-wide.
#![cfg_attr(not(kani), allow(unexpected_cfgs))]

use chrono::{DateTime, Duration, Utc};
use factory_lock_parse as flp;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Outcome of [`renew_lock`].
///
/// # Variants
///
/// - `NoOp` — `factory_lock:` key is absent (or absent regardless of fence shape),
///   OR the recomputed `expires_at` is byte-identical to the existing value.
///   STATE.md is unchanged; the caller MUST NOT call `host::write_file`.
///   Matches bash `factory-lock-write.sh renew` parity: silently exits 0 when
///   `factory_lock:` key is absent (F-NW2-006 / ADR-028 §Decision 9).
///
/// - `Renewed(String)` — lock was held and `expires_at` was updated to
///   `now + 2700s`. The `String` contains the new full STATE.md content.
///   The caller MUST call `host::write_file(".factory/STATE.md", content)`.
#[derive(Debug, PartialEq)]
pub enum RenewOutcome {
    /// Lock was absent (or key absent regardless of fence shape) — STATE.md unchanged.
    NoOp,
    /// Lock was held and expires_at was updated. Contains the new full STATE.md content.
    Renewed(String),
}

/// Error variants for lock operations.
///
/// # `Malformed`
///
/// `factory_lock:` key IS present in the frontmatter AND the block is malformed
/// (missing sub-field, empty sub-field, or missing closing `---` fence after the
/// lock key is found). Callers (the precompact-flush plugin) MUST downgrade this
/// to an advisory warning and proceed with the flush commit — do NOT exit 2
/// (EC-012 / BC-7.07.001 PC3 / ADR-028 §Decision 9).
#[derive(Debug, Clone, PartialEq)]
pub enum LockError {
    /// `factory_lock:` key IS present but the block is malformed.
    Malformed(String),
}

impl std::fmt::Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockError::Malformed(msg) => write!(f, "factory_lock block is malformed: {msg}"),
        }
    }
}

impl std::error::Error for LockError {}

/// Parsed representation of a held factory lock.
///
/// All three sub-fields are required when the lock block is present.
#[derive(Debug, Clone, PartialEq)]
pub struct FactoryLock {
    /// Email of the current lock holder.
    pub holder: String,
    /// ISO-8601 timestamp when the lock was acquired.
    pub locked_at: String,
    /// ISO-8601 datetime when the lock auto-expires (YYYY-MM-DDTHH:MM:SSZ format).
    pub expires_at: String,
}

// ---------------------------------------------------------------------------
// Public functions
// ---------------------------------------------------------------------------

/// Renew `factory_lock.expires_at` in STATE.md frontmatter.
///
/// Pure content-in/content-out; no `std::fs`; WASM-hermetic.
///
/// # Algorithm (ADR-028 §Decision 9 / F-NW2-005/006)
///
/// 1. Presence pre-check: if `factory_lock:` key is NOT in the frontmatter
///    (regardless of fence shape), return `Ok(RenewOutcome::NoOp)` immediately.
///    This matches bash parity — bash checks `factory_lock:` presence first.
/// 2. Call `factory_lock_parse::parse_factory_lock(content)`.
///    - `Ok(None)` — block absent or fully-null → `Ok(RenewOutcome::NoOp)`.
///      NOTE: per F-P56-001, `Ok(None)` is returned ONLY when the block is absent
///      or fully-null (null-value holder); an empty/absent holder with sibling fields
///      present returns `Err(MalformedLockBlock)` instead.
///    - `Err(MalformedLockBlock)` (key present but malformed) → `Err(LockError::Malformed)`.
///    - `Ok(Some(lock_state))` (key present, holder non-empty) → proceed to step 3.
/// 3. Compute `new_expires_at = Utc::now() + 2700s`, formatted as
///    `YYYY-MM-DDTHH:MM:SSZ` via `chrono::format("%Y-%m-%dT%H:%M:%SZ")` (NOT
///    `to_rfc3339()` — which emits `+00:00` / sub-seconds and violates AC-018 F-NW-008).
/// 4. If `new_expires_at == existing expires_at` (byte-identical recomputation,
///    F-R3-005 / ADR-028 §Decision 16): return `Ok(RenewOutcome::NoOp)` to suppress
///    spurious renewal.
/// 5. Rewrite the `expires_at:` line in the frontmatter block; preserve `holder` and
///    `locked_at` unchanged. Return `Ok(RenewOutcome::Renewed(new_content))`.
///
/// CRLF handling: `parse_factory_lock()` normalizes CRLF internally for its own parsing
/// (so parsed values — holder, locked_at, expires_at — are always clean). The input
/// `state_md_content` is passed untouched to `rewrite_expires_at`, which preserves every
/// line's original terminator byte-for-byte. Only the `expires_at` line value is replaced;
/// its original `\r\n` or `\n` terminator is mirrored in the output
/// (EC-017 CRLF input class / BC-4.17.001 PC4/AC-010 Invariant 5).
///
/// # Errors
///
/// Returns `Err(LockError::Malformed)` ONLY when `factory_lock:` key IS present AND
/// the block is malformed. The caller must downgrade to an advisory warning and
/// proceed with flush (EC-012 / ADR-028 §Decision 9).
pub fn renew_lock(state_md_content: &str) -> Result<RenewOutcome, LockError> {
    renew_lock_with_now(state_md_content, Utc::now)
}

/// Injectable clock variant of `renew_lock` for testability.
///
/// Identical semantics to `renew_lock`, but accepts a `now_fn` closure that
/// returns the current UTC time as a `chrono::DateTime<chrono::Utc>`.
/// This enables deterministic testing of the byte-identical expires_at guard
/// (ADR-028 §Decision 16 F-R3-005) without relying on wall-clock timing.
///
/// `renew_lock` delegates to this with `|| Utc::now()`.
///
/// `now_fn` is called exactly once per invocation; the `FnOnce` bound encodes
/// this invariant at the type level.
pub fn renew_lock_with_now<F>(state_md_content: &str, now_fn: F) -> Result<RenewOutcome, LockError>
where
    F: FnOnce() -> DateTime<Utc>,
{
    // Step 1: presence pre-check — bash parity (F-NW2-006 / ADR-028 §Decision 9).
    // If factory_lock: key is NOT present (regardless of fence shape), return NoOp.
    if !has_factory_lock_key(state_md_content) {
        return Ok(RenewOutcome::NoOp);
    }

    // Step 2: parse the factory_lock block.
    let lock_state = match flp::parse_factory_lock(state_md_content) {
        Ok(Some(ls)) => ls,
        Ok(None) => {
            // Block absent or fully-null → NoOp (F-P56-001).
            // Empty/absent holder with siblings present routes to Err(Malformed) instead.
            return Ok(RenewOutcome::NoOp);
        }
        Err(flp::LockParseError::MalformedLockBlock(msg)) => {
            return Err(LockError::Malformed(msg));
        }
    };

    // Step 3: compute new expires_at = now + TTL_SECONDS, formatted as YYYY-MM-DDTHH:MM:SSZ.
    // MUST use format("%Y-%m-%dT%H:%M:%SZ") — NOT to_rfc3339() (AC-018 F-NW-008).
    // TTL_SECONDS canonical source: factory-lock-parse::TTL_SECONDS (AC-012/ADR-046 F-006).
    let new_expires_at = (now_fn() + Duration::seconds(i64::from(flp::TTL_SECONDS)))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    // Step 4: spurious renewal guard — if byte-identical, return NoOp (F-R3-005).
    if new_expires_at == lock_state.expires_at {
        return Ok(RenewOutcome::NoOp);
    }

    // Step 5: rewrite expires_at in the frontmatter, preserving holder and locked_at.
    // Pass state_md_content directly — rewrite_expires_at preserves all line terminators
    // byte-for-byte (EC-017 CRLF input class; BC-4.17.001 PC4/AC-010 Invariant 5).
    // parse_factory_lock() normalizes CRLF internally for parsing; the content string
    // itself is untouched, so we pass it straight through for surgical rewriting.
    let new_content = rewrite_expires_at(state_md_content, &new_expires_at);

    Ok(RenewOutcome::Renewed(new_content))
}

/// Check whether the `factory_lock:` key appears in the frontmatter.
///
/// Presence pre-check used by `renew_lock()` BEFORE calling `parse_factory_lock()`.
/// Matches bash parity: `factory_lock:` absent → silent NoOp; malformed fence WITHOUT
/// the lock key → NoOp (not Malformed). Only a present key with a malformed block
/// yields `Err(Malformed)` (F-NW2-006 / ADR-028 §Decision 9).
///
/// Scans lines in the "open frontmatter region" (after the opening `---` line,
/// until the closing `---` line or end of content). Per ADR-028 §Decision 14 F-R3-002,
/// awk open-region semantics apply: the scan continues even if there is no closing `---`,
/// so `factory_lock:` in the "body region" (after an unclosed `---`) is still found.
pub fn has_factory_lock_key(state_md_content: &str) -> bool {
    // Normalize CRLF.
    let normalized;
    let content = if state_md_content.contains('\r') {
        normalized = state_md_content.replace("\r\n", "\n");
        normalized.as_str()
    } else {
        state_md_content
    };

    // Must start with `---\n` to have a frontmatter region.
    let after_open = match content.strip_prefix("---\n") {
        Some(rest) => rest,
        None => return false,
    };

    // Scan all lines in the open region (awk open-region semantics: no closing `---` required).
    // This matches bash parity: the region ends at the closing `---` fence if present,
    // but the scan does not require the fence to close (F-R3-002 / ADR-028 §Decision 14).
    for line in after_open.lines() {
        if line == "---" {
            // Closing fence found — stop scanning.
            break;
        }
        // Check if this line starts with `factory_lock:` (with optional trailing content).
        // `starts_with("factory_lock:")` subsumes the `== "factory_lock:"` case.
        if line.starts_with("factory_lock:") {
            return true;
        }
    }
    false
}

/// Rewrite the `expires_at:` sub-field inside the `factory_lock:` block.
///
/// Finds the first occurrence of `  expires_at:` (2-space indent) and replaces
/// its value. Content before and after is preserved byte-for-byte, including each
/// line's original terminator (`\r\n` or `\n`). The rewritten `expires_at` line
/// mirrors its own original terminator (EC-017 CRLF / BC-4.17.001 PC4/AC-010 Invariant 5).
///
/// Internal helper for `renew_lock_with_now`.
fn rewrite_expires_at(content: &str, new_expires_at: &str) -> String {
    let mut result = String::with_capacity(content.len() + 32);
    let mut in_factory_lock = false;
    let mut expires_at_rewritten = false;

    for line in content.split_inclusive('\n') {
        let trimmed = line.trim_end_matches('\n').trim_end_matches('\r');

        // `starts_with("factory_lock:")` subsumes the `== "factory_lock:"` case.
        if trimmed.starts_with("factory_lock:") {
            in_factory_lock = true;
            result.push_str(line);
            continue;
        }

        if in_factory_lock && !expires_at_rewritten {
            // Look for `  expires_at:` with exactly 2-space indent.
            if trimmed.starts_with("  expires_at:") {
                // Replace value while preserving the original line terminator byte-for-byte.
                // MUST check \r\n before \n — every CRLF line also ends with \n (EC-017).
                // This satisfies BC-4.17.001 PC4/AC-010 Invariant 5 for CRLF input.
                let terminator = if line.ends_with("\r\n") {
                    "\r\n"
                } else if line.ends_with('\n') {
                    "\n"
                } else {
                    ""
                };
                result.push_str(&format!("  expires_at: {new_expires_at}"));
                result.push_str(terminator);
                expires_at_rewritten = true;
                continue;
            }
            // Exit factory_lock block on non-indented, non-empty line.
            if !trimmed.is_empty() && !trimmed.starts_with(' ') {
                in_factory_lock = false;
            }
        }

        result.push_str(line);
    }

    // CR-005: if expires_at_rewritten is still false after the full scan, the parser
    // diverged (factory_lock block was detected but expires_at line was not found).
    // Surface in test builds to catch parser regressions early; no release-path cost.
    debug_assert!(
        expires_at_rewritten,
        "rewrite_expires_at: factory_lock block found but expires_at line was not rewritten \
        — parser divergence; check that the block uses 2-space indent for expires_at"
    );

    result
}

/// Acquire the factory lock in STATE.md frontmatter content.
///
/// Pure content-in/content-out; no `std::fs`.
/// Intended for S-18.04b and related lock-management stories.
pub fn acquire_lock(
    _state_md_content: &str,
    _holder: &str,
    _locked_at: &str,
    _expires_at: &str,
) -> Result<String, LockError> {
    todo!("acquire_lock: scoped to S-18.04b")
}

/// Clear the factory lock in STATE.md frontmatter content.
///
/// Pure content-in/content-out; no `std::fs`.
/// Removes the `factory_lock:` block (or writes it fully null) so subsequent
/// `renew_lock()` calls return `Ok(RenewOutcome::NoOp)`.
pub fn clear_lock(_state_md_content: &str) -> Result<String, LockError> {
    todo!("clear_lock: scoped to S-18.04b")
}

// ---------------------------------------------------------------------------
// Internal helper — re-exported for use by has_factory_lock_key tests
// ---------------------------------------------------------------------------

/// Parse the `factory_lock:` block from STATE.md content, delegating to
/// `factory_lock_parse::parse_factory_lock`.
///
/// Internal helper surfaced for testability. Use `renew_lock` for the public API.
///
/// # Returns
///
/// - `Ok(Some(FactoryLock))` — key present, holder non-empty, all fields valid.
/// - `Ok(None)` — block absent or fully-null (per F-P56-001: `Ok(None)` is returned
///   ONLY for absent-or-fully-null block; empty/absent holder with sibling fields
///   present returns `Err(Malformed)` instead).
/// - `Err(LockError::Malformed)` — key present but block is malformed.
pub fn parse_lock(state_md_content: &str) -> Result<Option<FactoryLock>, LockError> {
    match flp::parse_factory_lock(state_md_content) {
        Ok(Some(ls)) => Ok(Some(FactoryLock {
            holder: ls.holder,
            locked_at: ls.locked_at,
            expires_at: ls.expires_at,
        })),
        Ok(None) => Ok(None),
        Err(flp::LockParseError::MalformedLockBlock(msg)) => Err(LockError::Malformed(msg)),
    }
}

// ---------------------------------------------------------------------------
// S-17.06: IdentityResolution, SkipReason, renew_lock_if_holder,
//          classify_identity_resolution, trim_git_email
// (BC-4.17.001 PC2 — ADR-046 Decision 1(b)/2)
// ---------------------------------------------------------------------------

/// The result of resolving the caller's git identity for lock renewal.
///
/// Produced by [`classify_identity_resolution`] from the output of a
/// `git config user.email` subprocess invocation.
///
/// - `Resolved(email)` — subprocess returned exit 0 and a non-empty email after
///   trimming. Contains the trimmed git email string.
/// - `Failed(reason)` — identity could not be resolved (exec error, non-zero exit,
///   or empty stdout after trimming via [`trim_git_email`]).
///
/// ADR-046 Decision 2 / BC-4.17.001 Precondition 2.
#[derive(Debug, Clone, PartialEq)]
pub enum IdentityResolution {
    /// Identity resolved successfully; contains the trimmed git email.
    Resolved(String),
    /// Identity resolution failed; contains a human-readable reason string.
    Failed(String),
}

/// The reason a [`renew_lock_if_holder`] call was skipped without renewing.
///
/// Returned alongside [`RenewOutcome::NoOp`] when the skip has a diagnosable
/// cause (Cases 2, 3, 4 of the 6-case decision tree). Case 0 returns
/// `Ok((RenewOutcome::NoOp, None))`. Case 1 returns `Err(LockError::Malformed)`
/// — there is no `Option` at all for case 1 (it is not an `Ok` return).
///
/// BC-4.17.001 PC2 / ADR-046 Decision 1(b).
#[derive(Debug, Clone, PartialEq)]
pub enum SkipReason {
    /// Case 3: Lock is valid and unexpired, but held by a different identity.
    NotHolder,
    /// Case 2: Lock exists but `now >= expires_at`; renewal skipped (lock expired).
    AlreadyExpired,
    /// Case 4: Identity resolution failed; cannot determine whether caller is the holder.
    ///
    /// All four fields are populated from the parsed `LockState` (AC-003 / F-P21-001).
    /// No field may be absent — they are required by the 4-field struct invariant.
    IdentityResolutionFailed {
        /// Human-readable reason the identity resolution failed.
        reason: String,
        /// Email of the current lock holder (from parsed `LockState`).
        holder: String,
        /// ISO-8601 timestamp when the lock was acquired (from parsed `LockState`).
        locked_at: String,
        /// ISO-8601 expiry timestamp of the lock (from parsed `LockState`).
        expires_at: String,
    },
}

/// Classify the result of a `git config user.email` subprocess invocation into
/// an [`IdentityResolution`].
///
/// Implements the 4-shape mapping defined in ADR-046 Decision 2 / BC-4.17.001
/// Precondition 2 (F-006):
///
/// 1. `Err(_)` → `IdentityResolution::Failed("exec error: <description>")`
/// 2. `Ok((exit_code, _))` where `exit_code != 0` → `IdentityResolution::Failed("exit N: git config user.email failed")`
/// 3. `Ok((0, stdout))` where `trim_git_email(&stdout).is_empty()` →
///    `IdentityResolution::Failed("empty identity")`
/// 4. `Ok((0, stdout))` where `!trim_git_email(&stdout).is_empty()` →
///    `IdentityResolution::Resolved(trim_git_email(&stdout))`
///
/// # Parameters
///
/// - `exec_result` — `Ok((exit_code, stdout))` on subprocess success,
///   `Err(host_error_description)` on exec/host failure.
///
/// Pure function; no I/O. WASM-hermetic.
pub fn classify_identity_resolution(
    exec_result: Result<(i32, String), String>,
) -> IdentityResolution {
    match exec_result {
        Err(e) => IdentityResolution::Failed(format!("exec error: {}", e)),
        Ok((exit, _)) if exit != 0 => {
            IdentityResolution::Failed(format!("exit {}: git config user.email failed", exit))
        }
        Ok((_, stdout)) => {
            let email = trim_git_email(&stdout);
            if email.is_empty() {
                IdentityResolution::Failed("empty identity".to_string())
            } else {
                IdentityResolution::Resolved(email)
            }
        }
    }
}

/// Renew the factory lock in STATE.md if and only if the caller is the current
/// lock holder (identity-gated renewal).
///
/// Implements the 6-case decision tree from BC-4.17.001 PC2 / ADR-046 Decision 1(b):
///
/// - **Case 0 (absent/null block):** `parse_factory_lock` returns `Ok(None)` →
///   `Ok((RenewOutcome::NoOp, None))`. `resolve_identity` is NOT called.
/// - **Case 1 (malformed block):** `parse_factory_lock` returns `Err(Malformed)` →
///   `Err(LockError::Malformed)`. `resolve_identity` is NOT called.
/// - **Case 2 (already expired):** lock present, `now >= expires_at` →
///   `Ok((RenewOutcome::NoOp, Some(SkipReason::AlreadyExpired)))`.
///   `resolve_identity` is NOT called.
/// - **Case 3 (not holder):** identity resolved, email != holder (after
///   [`trim_git_email`]) → `Ok((RenewOutcome::NoOp, Some(SkipReason::NotHolder)))`.
/// - **Case 4 (identity resolution failed):** `resolve_identity` returns
///   `IdentityResolution::Failed(reason)` →
///   `Ok((RenewOutcome::NoOp, Some(SkipReason::IdentityResolutionFailed { reason, holder, locked_at, expires_at })))`.
///   All four struct fields are populated from the parsed `LockState`.
/// - **Case 5 (success):** identity matches, `now < expires_at` →
///   `Ok((RenewOutcome::Renewed(new_content), None))` with
///   `expires_at = now + TTL_SECONDS`, formatted as `YYYY-MM-DDTHH:MM:SSZ`.
///
/// `resolve_identity` is called AT MOST ONCE per invocation and MUST NOT be
/// called for Cases 0, 1, or 2 (AC-002 lazy-call constraint).
///
/// # Parameters
///
/// - `content` — full STATE.md content (pure content-in/content-out; no I/O).
/// - `resolve_identity` — lazy `FnOnce` closure that resolves the caller's git
///   identity. Called only when the lock is present, valid, and not expired.
/// - `now_fn` — injectable clock closure (`FnOnce`); use `|| Utc::now()` in production.
///   Enables deterministic testing without wall-clock timing. Called exactly once
///   per invocation (SEC-004 / B-3 compile-time safety gate).
///
/// Pure-core function; `resolve_identity` and `now_fn` are the only effectful
/// surfaces (injected by caller). WASM-hermetic; no direct host I/O.
pub fn renew_lock_if_holder<F, I>(
    content: &str,
    resolve_identity: I,
    now_fn: F,
) -> Result<(RenewOutcome, Option<SkipReason>), LockError>
where
    F: FnOnce() -> DateTime<Utc>,
    I: FnOnce() -> IdentityResolution,
{
    // B-1 pre-check: if factory_lock: key absent, return NoOp immediately.
    // Matches bash parity (F-NW2-006 / ADR-028 §Decision 9) — same guard used by
    // renew_lock_with_now. Without this, parse_factory_lock returns Err(Malformed)
    // for any STATE.md with no closing --- delimiter even when factory_lock: is absent.
    // resolve_identity is NOT called (AC-002 lazy-call invariant).
    if !has_factory_lock_key(content) {
        return Ok((RenewOutcome::NoOp, None));
    }

    // Cases 0 and 1: parse the factory_lock block.
    // resolve_identity is NOT called for these two cases (AC-002 lazy-call invariant).
    let lock_state = match flp::parse_factory_lock(content) {
        Ok(None) => {
            // Case 0: block absent/fully-null → NoOp, no skip reason.
            return Ok((RenewOutcome::NoOp, None));
        }
        Err(flp::LockParseError::MalformedLockBlock(msg)) => {
            // Case 1: key present but block malformed → propagate error.
            return Err(LockError::Malformed(msg));
        }
        Ok(Some(ls)) => ls,
    };

    // Case 2: check expiry before calling resolve_identity (AC-002 lazy-call invariant).
    // parse_iso8601 on expires_at — failure means the block is malformed (case 1 extension).
    let expires_at_dt = flp::parse_iso8601(&lock_state.expires_at)
        .map_err(|flp::LockParseError::MalformedLockBlock(msg)| LockError::Malformed(msg))?;

    let now = now_fn();
    if now >= expires_at_dt {
        // Case 2: already expired — resolve_identity is NOT called.
        return Ok((RenewOutcome::NoOp, Some(SkipReason::AlreadyExpired)));
    }

    // Identity step: call resolve_identity EXACTLY ONCE (AC-002 at-most-once).
    // Reached only when lock is present, valid, and not expired.
    match resolve_identity() {
        IdentityResolution::Failed(reason) => {
            // Case 4: identity resolution failed — populate all four struct fields from LockState.
            Ok((
                RenewOutcome::NoOp,
                Some(SkipReason::IdentityResolutionFailed {
                    reason,
                    holder: lock_state.holder,
                    locked_at: lock_state.locked_at,
                    expires_at: lock_state.expires_at,
                }),
            ))
        }
        IdentityResolution::Resolved(email) => {
            if email != trim_git_email(&lock_state.holder) {
                // Case 3: different identity — not the holder (comparison after trim_git_email
                // on both sides per AC-001; email already trimmed by classify_identity_resolution,
                // holder trimmed here to handle raw YAML-parsed whitespace — B-2).
                Ok((RenewOutcome::NoOp, Some(SkipReason::NotHolder)))
            } else {
                // Case 5: identity matches — delegate to renew_lock_with_now.
                // SEC-004 (CWE-362): pass `|| now` (capturing the already-evaluated
                // `now`) instead of re-passing `now_fn`. This ensures `now_fn` is
                // called exactly once per invocation and the same instant is used for
                // both the expiry check (above) and the renewal timestamp.
                let outcome = renew_lock_with_now(content, || now)?;
                Ok((outcome, None))
            }
        }
    }
}

/// Trim trailing whitespace (including `\n`) from a git subprocess stdout line.
///
/// Canonical home for this function per F-P7-001 single-canonical-home principle
/// (AC-005 / ADR-046 Decision 2). After AC-005 implementation, any crate that
/// previously contained a local copy of this function (e.g.,
/// `crates/hook-plugins/verify-factory-lock/src/lib.rs`) MUST delegate to this
/// function. No re-implementation permitted in any crate.
pub fn trim_git_email(raw: &str) -> String {
    raw.trim_end().to_string()
}

// ---------------------------------------------------------------------------
// S-17.06 Red Gate tests (BC-4.17.001 / BC-5.38.001 strict tdd_mode)
//
// All 13 tests MUST fail before implementation exists (Red Gate armed).
// Each test calls a real production function (POLICY 11: no vacuous assertions).
// Red Gate failure mode: todo!() panic for fns 1-13; source-scan assertion
// failure for test 14 (in verify-factory-lock crate).
//
// Test naming follows the Red Gate Test Table in S-17.06 spec (authoritative).
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

    use super::*;
    use chrono::{DateTime, Utc};

    // -----------------------------------------------------------------------
    // Fixture builders (pure STATE.md content strings)
    // -----------------------------------------------------------------------

    /// STATE.md with NO factory_lock block — case 0 input (absent/fully-null block).
    fn fixture_no_lock() -> &'static str {
        concat!(
            "---\n",
            "document_type: state\n",
            "version: \"test\"\n",
            "phase: test\n",
            "---\n\n# STATE\n",
        )
    }

    /// STATE.md with malformed factory_lock block (holder = "" → MalformedLockBlock).
    /// Case 1 input.
    fn fixture_malformed_lock() -> &'static str {
        concat!(
            "---\n",
            "document_type: state\n",
            "version: \"test\"\n",
            "phase: test\n",
            "factory_lock:\n",
            "  holder: \"\"\n",
            "  locked_at: \"2026-01-01T10:00:00Z\"\n",
            "  expires_at: \"2026-01-01T10:45:00Z\"\n",
            "---\n\n# STATE\n",
        )
    }

    /// STATE.md with a valid factory_lock block but expires_at is in the past (2020).
    /// Case 2 input — now_past() (2026) >= expires_at (2020) → AlreadyExpired.
    fn fixture_expired_lock() -> &'static str {
        concat!(
            "---\n",
            "document_type: state\n",
            "version: \"test\"\n",
            "phase: test\n",
            "factory_lock:\n",
            "  holder: \"holder@example.com\"\n",
            "  locked_at: \"2020-01-01T10:00:00Z\"\n",
            "  expires_at: \"2020-01-01T10:45:00Z\"\n",
            "---\n\n# STATE\n",
        )
    }

    /// STATE.md with a valid unexpired factory_lock block (expires_at 2099).
    /// Cases 3/4/5 input — now_past() (2026) < expires_at (2099) → identity step reached.
    /// holder = "holder@example.com"; locked_at = "2026-01-01T10:00:00Z".
    fn fixture_valid_unexpired_lock() -> &'static str {
        concat!(
            "---\n",
            "document_type: state\n",
            "version: \"test\"\n",
            "phase: test\n",
            "factory_lock:\n",
            "  holder: \"holder@example.com\"\n",
            "  locked_at: \"2026-01-01T10:00:00Z\"\n",
            "  expires_at: \"2099-01-01T10:45:00Z\"\n",
            "---\n\n# STATE\n",
        )
    }

    /// STATE.md with a factory_lock block whose expires_at is present-but-unparseable.
    /// holder is non-empty so parse_factory_lock returns Ok(Some(ls)), but expires_at
    /// = "not-a-timestamp" makes parse_iso8601 fail → Err(LockError::Malformed(_)).
    /// F-P1-002 / EC-007 input.
    fn fixture_unparseable_expires_at_lock() -> &'static str {
        concat!(
            "---\n",
            "document_type: state\n",
            "version: \"test\"\n",
            "phase: test\n",
            "factory_lock:\n",
            "  holder: \"holder@example.com\"\n",
            "  locked_at: \"2026-01-01T10:00:00Z\"\n",
            "  expires_at: \"not-a-timestamp\"\n",
            "---\n\n# STATE\n",
        )
    }

    /// Injectable clock returning 2026-08-27 12:00:00Z.
    /// After expired fixture (2020) but before valid fixture (2099): discriminates cases 2 vs 3/4/5.
    fn now_past() -> DateTime<Utc> {
        "2026-08-27T12:00:00Z"
            .parse::<DateTime<Utc>>()
            .expect("fixture timestamp must parse as DateTime<Utc>")
    }

    // -----------------------------------------------------------------------
    // AC-001 tests — renew_lock_if_holder 6-case decision tree (BC-4.17.001 PC2)
    // -----------------------------------------------------------------------

    /// Case 0: parse_factory_lock returns Ok(None) → Ok((NoOp, None)).
    /// resolve_identity MUST NOT be called (AC-002 lazy-call invariant).
    /// Red Gate: todo!() panics before any return.
    #[test]
    fn test_renew_lock_if_holder_absent_block_no_op() {
        let result = renew_lock_if_holder(
            fixture_no_lock(),
            || panic!("resolve_identity must NOT be called for absent block (AC-002, case 0)"),
            now_past,
        );
        let (outcome, skip) = result.expect("case 0 (absent block) must return Ok");
        match outcome {
            RenewOutcome::NoOp => {}
            RenewOutcome::Renewed(_) => panic!("case 0 must return NoOp, not Renewed"),
        }
        assert!(
            skip.is_none(),
            "case 0 must return None skip reason, got: {:?}",
            skip
        );
    }

    /// Case 1: parse_factory_lock returns Err(MalformedLockBlock) → Err(LockError::Malformed).
    /// resolve_identity MUST NOT be called (AC-002 lazy-call invariant).
    /// Red Gate: todo!() panics before any return.
    #[test]
    fn test_renew_lock_if_holder_malformed_no_resolve() {
        let result = renew_lock_if_holder(
            fixture_malformed_lock(),
            || panic!("resolve_identity must NOT be called for malformed block (AC-002, case 1)"),
            now_past,
        );
        match result {
            Err(LockError::Malformed(_)) => {}
            Ok((_, _)) => panic!("case 1 (malformed block) must return Err(Malformed), not Ok"),
        }
    }

    /// Case 2: lock present but now >= expires_at → Ok((NoOp, Some(AlreadyExpired))).
    /// resolve_identity MUST NOT be called (AC-002 lazy-call invariant).
    /// Red Gate: todo!() panics before any return.
    #[test]
    fn test_renew_lock_if_holder_already_expired_no_resolve() {
        let result = renew_lock_if_holder(
            fixture_expired_lock(),
            || panic!("resolve_identity must NOT be called for expired lock (AC-002, case 2)"),
            now_past,
        );
        let (outcome, skip) = result.expect("case 2 (already expired) must return Ok");
        match outcome {
            RenewOutcome::NoOp => {}
            RenewOutcome::Renewed(_) => panic!("case 2 must return NoOp, not Renewed"),
        }
        match skip.expect("case 2 must carry a skip reason") {
            SkipReason::AlreadyExpired => {}
            other => panic!("case 2 must return AlreadyExpired, got: {:?}", other),
        }
    }

    /// Case 3: identity resolved, email != holder (after trim_git_email) → Ok((NoOp, Some(NotHolder))).
    /// Red Gate: todo!() panics before any return.
    #[test]
    fn test_renew_lock_if_holder_not_holder_no_renewal() {
        let result = renew_lock_if_holder(
            fixture_valid_unexpired_lock(),
            || IdentityResolution::Resolved("other@example.com".to_string()),
            now_past,
        );
        let (outcome, skip) = result.expect("case 3 (not holder) must return Ok");
        match outcome {
            RenewOutcome::NoOp => {}
            RenewOutcome::Renewed(_) => panic!("case 3 (not holder) must return NoOp, not Renewed"),
        }
        match skip.expect("case 3 must carry a skip reason") {
            SkipReason::NotHolder => {}
            other => panic!("case 3 must return NotHolder, got: {:?}", other),
        }
    }

    /// Case 4: identity resolution returns Failed(reason) →
    /// Ok((NoOp, Some(IdentityResolutionFailed { reason, holder, locked_at, expires_at }))).
    /// All four SkipReason fields must be populated from the parsed LockState (AC-003).
    /// Red Gate: todo!() panics before any return.
    #[test]
    fn test_renew_lock_if_holder_identity_resolution_failed_no_renewal() {
        let result = renew_lock_if_holder(
            fixture_valid_unexpired_lock(),
            || IdentityResolution::Failed("git config failed".to_string()),
            now_past,
        );
        let (outcome, skip) = result.expect("case 4 (resolution failed) must return Ok");
        match outcome {
            RenewOutcome::NoOp => {}
            RenewOutcome::Renewed(_) => {
                panic!("case 4 (resolution failed) must return NoOp, not Renewed")
            }
        }
        match skip.expect("case 4 must carry a skip reason") {
            SkipReason::IdentityResolutionFailed {
                reason,
                holder,
                locked_at,
                expires_at,
            } => {
                assert_eq!(
                    reason, "git config failed",
                    "reason must match Failed variant payload"
                );
                assert_eq!(
                    holder, "holder@example.com",
                    "holder must come from parsed LockState"
                );
                assert_eq!(
                    locked_at, "2026-01-01T10:00:00Z",
                    "locked_at must come from parsed LockState"
                );
                assert_eq!(
                    expires_at, "2099-01-01T10:45:00Z",
                    "expires_at must come from parsed LockState"
                );
            }
            other => panic!(
                "case 4 must return IdentityResolutionFailed, got: {:?}",
                other
            ),
        }
    }

    /// Case 5: identity matches (email == holder after trim_git_email), now < expires_at →
    /// Ok((Renewed(new_content), None)) with expires_at advanced by TTL_SECONDS.
    /// Red Gate: todo!() panics before any return.
    #[test]
    fn test_renew_lock_if_holder_identity_match_renewed() {
        let result = renew_lock_if_holder(
            fixture_valid_unexpired_lock(),
            || IdentityResolution::Resolved("holder@example.com".to_string()),
            now_past,
        );
        let (outcome, skip) = result.expect("case 5 (identity match) must return Ok");
        match outcome {
            RenewOutcome::Renewed(new_content) => {
                assert!(
                    new_content.contains("expires_at:"),
                    "Renewed content must contain an updated expires_at line"
                );
                assert!(
                    new_content.contains("holder@example.com"),
                    "Renewed content must preserve the holder field"
                );
                // Mutation-killing: assert expires_at actually ADVANCED.
                // The fixture's stale timestamp (2099-01-01T10:45:00Z) must be gone …
                assert!(
                    !new_content.contains("2099-01-01T10:45:00Z"),
                    "Renewed content must NOT still contain the stale fixture expires_at \
                     (2099-01-01T10:45:00Z) — rewrite_expires_at may have been suppressed"
                );
                // … and the expected value from now_past() + 2700s must be present.
                // now_past() = 2026-08-27T12:00:00Z; + 2700 s = 2026-08-27T12:45:00Z.
                assert!(
                    new_content.contains("2026-08-27T12:45:00Z"),
                    "Renewed content must contain the expected advanced expires_at \
                     (now_past() + 2700s = 2026-08-27T12:45:00Z)"
                );
            }
            RenewOutcome::NoOp => panic!("case 5 (identity match) must return Renewed, not NoOp"),
        }
        assert!(
            skip.is_none(),
            "case 5 (success) must return None skip reason, got: {:?}",
            skip
        );
    }

    // -----------------------------------------------------------------------
    // AC-002 test — lazy identity evaluation: resolve_identity called at most once
    // -----------------------------------------------------------------------

    /// resolve_identity must be called AT MOST ONCE per invocation and MUST NOT
    /// be called for cases 0, 1, or 2. For case 5 (success) it is called exactly once.
    ///
    /// Red Gate: todo!() panics on the first call to renew_lock_if_holder; the
    /// counter assertions are unreachable. The test fails due to panic (correct Red Gate).
    #[test]
    fn test_resolve_identity_called_at_most_once() {
        use std::sync::{Arc, Mutex};

        // Part A: case 0 (absent block) — resolve_identity must NOT be called.
        let count_0 = Arc::new(Mutex::new(0u32));
        let c0 = count_0.clone();
        let result_0 = renew_lock_if_holder(
            fixture_no_lock(),
            move || {
                *c0.lock().unwrap() += 1;
                IdentityResolution::Resolved("test@example.com".to_string())
            },
            now_past,
        );
        assert!(result_0.is_ok(), "case 0 must return Ok");
        assert_eq!(
            *count_0.lock().unwrap(),
            0,
            "resolve_identity must NOT be called for absent block (AC-002 case 0)"
        );

        // Part B: case 5 (identity match) — resolve_identity must be called exactly once.
        let count_5 = Arc::new(Mutex::new(0u32));
        let c5 = count_5.clone();
        let result_5 = renew_lock_if_holder(
            fixture_valid_unexpired_lock(),
            move || {
                *c5.lock().unwrap() += 1;
                IdentityResolution::Resolved("holder@example.com".to_string())
            },
            now_past,
        );
        assert!(result_5.is_ok(), "case 5 must return Ok");
        assert_eq!(
            *count_5.lock().unwrap(),
            1,
            "resolve_identity must be called exactly once for case 5 (AC-002 at-most-once)"
        );
    }

    // -----------------------------------------------------------------------
    // AC-003 test — SkipReason::IdentityResolutionFailed carries all four fields
    // -----------------------------------------------------------------------

    /// SkipReason::IdentityResolutionFailed must be a struct variant with exactly
    /// four fields (reason, holder, locked_at, expires_at), all populated from the
    /// parsed LockState — NOT from caller-supplied data (F-P21-001 / AC-003).
    ///
    /// Red Gate: todo!() panics before case 4 return.
    #[test]
    fn test_skip_reason_identity_resolution_failed_carries_four_fields() {
        let result = renew_lock_if_holder(
            fixture_valid_unexpired_lock(),
            || IdentityResolution::Failed("identity error reason".to_string()),
            now_past,
        );
        let (_, skip) = result.expect("case 4 must return Ok");
        match skip.expect("case 4 must carry a skip reason") {
            SkipReason::IdentityResolutionFailed {
                reason,
                holder,
                locked_at,
                expires_at,
            } => {
                // All four fields non-empty and sourced from the parsed LockState fixture.
                assert!(!reason.is_empty(), "reason must be non-empty (AC-003)");
                assert!(
                    !holder.is_empty(),
                    "holder must be populated from LockState (AC-003, F-P21-001)"
                );
                assert!(
                    !locked_at.is_empty(),
                    "locked_at must be populated from LockState (AC-003, F-P21-001)"
                );
                assert!(
                    !expires_at.is_empty(),
                    "expires_at must be populated from LockState (AC-003, F-P21-001)"
                );
                // Exact field values match the parsed fixture — NOT caller-supplied data.
                assert_eq!(holder, "holder@example.com");
                assert_eq!(locked_at, "2026-01-01T10:00:00Z");
                assert_eq!(expires_at, "2099-01-01T10:45:00Z");
            }
            other => panic!(
                "Expected SkipReason::IdentityResolutionFailed, got: {:?}",
                other
            ),
        }
    }

    // -----------------------------------------------------------------------
    // AC-004 tests — classify_identity_resolution 4-shape rule (BC-4.17.001 PC2/Precondition 2)
    // -----------------------------------------------------------------------

    /// Shape 1: Err(_) → IdentityResolution::Failed("exec error: ...")
    /// Red Gate: todo!() panics.
    #[test]
    fn test_classify_identity_resolution_exec_error_maps_failed() {
        let result = classify_identity_resolution(Err("process spawn failed".to_string()));
        match result {
            IdentityResolution::Failed(reason) => {
                assert!(
                    reason.starts_with("exec error"),
                    "exec Err shape must produce Failed starting with 'exec error', got: {:?}",
                    reason
                );
            }
            IdentityResolution::Resolved(v) => {
                panic!("exec Err must produce Failed, not Resolved({:?})", v)
            }
        }
    }

    /// Shape 2: Ok((exit_code != 0, _)) → IdentityResolution::Failed("exit N: ...")
    /// Red Gate: todo!() panics.
    #[test]
    fn test_classify_identity_resolution_nonzero_exit_maps_failed() {
        let result = classify_identity_resolution(Ok((1, "error output".to_string())));
        match result {
            IdentityResolution::Failed(reason) => {
                assert!(
                    reason.starts_with("exit 1"),
                    "nonzero exit shape must produce Failed starting with 'exit 1', got: {:?}",
                    reason
                );
            }
            IdentityResolution::Resolved(v) => {
                panic!("nonzero exit must produce Failed, not Resolved({:?})", v)
            }
        }
    }

    /// Shape 3: Ok((0, stdout)) where trim_git_email(stdout).is_empty() →
    /// IdentityResolution::Failed("empty identity")
    /// Red Gate: todo!() panics.
    #[test]
    fn test_classify_identity_resolution_empty_stdout_maps_failed() {
        let result = classify_identity_resolution(Ok((0, "\n".to_string())));
        match result {
            IdentityResolution::Failed(reason) => {
                assert_eq!(
                    reason, "empty identity",
                    "empty stdout shape must produce Failed(\"empty identity\"), got: {:?}",
                    reason
                );
            }
            IdentityResolution::Resolved(v) => {
                panic!("empty stdout must produce Failed, not Resolved({:?})", v)
            }
        }
    }

    /// Shape 4: Ok((0, non-empty stdout)) →
    /// IdentityResolution::Resolved(trim_git_email(stdout))
    /// Red Gate: todo!() panics.
    #[test]
    fn test_classify_identity_resolution_nonempty_stdout_maps_resolved() {
        let result = classify_identity_resolution(Ok((0, "user@example.com\n".to_string())));
        match result {
            IdentityResolution::Resolved(email) => {
                assert_eq!(
                    email, "user@example.com",
                    "valid stdout shape must produce Resolved with trimmed email, got: {:?}",
                    email
                );
            }
            IdentityResolution::Failed(reason) => {
                panic!(
                    "valid stdout must produce Resolved, not Failed({:?})",
                    reason
                )
            }
        }
    }

    // -----------------------------------------------------------------------
    // AC-005 test — trim_git_email canonical home in factory_lock crate
    // -----------------------------------------------------------------------

    /// factory_lock::trim_git_email must strip trailing whitespace/newlines.
    /// Canonical home per F-P7-001 single-canonical-home principle (AC-005 / ADR-046 Decision 2).
    ///
    /// Red Gate: todo!() panics.
    #[test]
    fn test_trim_git_email_canonical_in_factory_lock() {
        // Trailing newline stripped.
        let r1 = trim_git_email("user@example.com\n");
        assert_eq!(
            r1, "user@example.com",
            "trim_git_email must strip trailing newline, got: {:?}",
            r1
        );

        // Trailing whitespace + newline stripped.
        let r2 = trim_git_email("user@example.com  \n");
        assert_eq!(
            r2, "user@example.com",
            "trim_git_email must strip trailing whitespace+newline, got: {:?}",
            r2
        );

        // Email without trailing whitespace preserved as-is.
        let r3 = trim_git_email("user@example.com");
        assert_eq!(
            r3, "user@example.com",
            "trim_git_email must preserve email with no trailing whitespace, got: {:?}",
            r3
        );

        // Bare newline → empty string (EC-008: stdout = "\n" → Failed("empty identity")).
        let r4 = trim_git_email("\n");
        assert_eq!(
            r4, "",
            "trim_git_email(\"\\n\") must return empty string (EC-008), got: {:?}",
            r4
        );
    }

    // -----------------------------------------------------------------------
    // F-P1-002 — mutation-killing coverage: case-1 extension, unparseable expires_at (EC-007)
    // -----------------------------------------------------------------------

    /// F-P1-002 (EC-007): expires_at present-but-unparseable routes to Err(Malformed).
    ///
    /// fixture_unparseable_expires_at_lock() carries holder="holder@example.com"
    /// (non-empty) and expires_at="not-a-timestamp" (non-empty). parse_factory_lock
    /// returns Ok(Some(ls)) because the holder field is valid; the block is structurally
    /// well-formed. parse_iso8601("not-a-timestamp") then fails in renew_lock_if_holder,
    /// and map_err(...)? propagates Err(LockError::Malformed(_)) before the identity step.
    ///
    /// resolve_identity MUST NOT be called (AC-002 lazy-call: case-1 extension
    /// short-circuits before the identity step). The panic closure enforces this.
    ///
    /// Mutant killed: dropping the `?` or the map_err on the parse_iso8601 call would
    /// allow execution to fall through past the error and either panic on an unwrap or
    /// proceed to the identity step (hitting the panic closure). Either way the test fails
    /// on the mutant.
    #[test]
    fn test_renew_lock_if_holder_malformed_expires_at_returns_err() {
        let result = renew_lock_if_holder(
            fixture_unparseable_expires_at_lock(),
            || {
                panic!(
                    "resolve_identity must NOT be called when expires_at is unparseable \
                 (F-P1-002, AC-002 case-1 extension)"
                )
            },
            now_past,
        );
        match result {
            Err(LockError::Malformed(_)) => {}
            Ok((_, _)) => panic!(
                "F-P1-002: unparseable expires_at must return Err(LockError::Malformed), not Ok"
            ),
        }
    }

    // -----------------------------------------------------------------------
    // F-P1-003 — mutation-killing coverage: case-2 boundary now == expires_at (BC-4.17.001 PC2)
    // -----------------------------------------------------------------------

    /// F-P1-003: boundary now == expires_at is treated as AlreadyExpired (load-bearing `>=`).
    ///
    /// The fixture carries expires_at="2026-06-15T08:30:00Z"; now_fn returns exactly
    /// that same instant. The condition `now >= expires_at_dt` is true at the boundary
    /// point → Ok((NoOp, Some(AlreadyExpired))). resolve_identity MUST NOT be called
    /// (AC-002 lazy-call: case 2 short-circuits before the identity step).
    ///
    /// Mutant killed: flipping `>=` → `>` makes `now == expires_at_dt` evaluate to
    /// false. Execution would then fall through to the identity step and hit the panic
    /// closure, failing the test on the mutant.
    #[test]
    fn test_renew_lock_if_holder_now_equals_expires_at_is_expired() {
        let content = concat!(
            "---\n",
            "document_type: state\n",
            "version: \"test\"\n",
            "phase: test\n",
            "factory_lock:\n",
            "  holder: \"holder@example.com\"\n",
            "  locked_at: \"2026-01-01T10:00:00Z\"\n",
            "  expires_at: \"2026-06-15T08:30:00Z\"\n",
            "---\n\n# STATE\n",
        );
        // now_fn returns EXACTLY expires_at — the boundary instant.
        let now_exact = || {
            "2026-06-15T08:30:00Z"
                .parse::<DateTime<Utc>>()
                .expect("boundary timestamp must parse as DateTime<Utc>")
        };
        let result = renew_lock_if_holder(
            content,
            || {
                panic!(
                    "resolve_identity must NOT be called when now == expires_at \
                 (F-P1-003, AC-002 case 2 boundary)"
                )
            },
            now_exact,
        );
        let (outcome, skip) =
            result.expect("F-P1-003: now==expires_at must return Ok (AlreadyExpired)");
        match outcome {
            RenewOutcome::NoOp => {}
            RenewOutcome::Renewed(_) => {
                panic!("F-P1-003: now==expires_at must return NoOp, not Renewed")
            }
        }
        match skip.expect("F-P1-003: now==expires_at must carry a skip reason") {
            SkipReason::AlreadyExpired => {}
            other => panic!(
                "F-P1-003: now==expires_at must return AlreadyExpired, got: {:?}",
                other
            ),
        }
    }

    // -----------------------------------------------------------------------
    // B-1 test — has_factory_lock_key pre-check in renew_lock_if_holder
    // -----------------------------------------------------------------------

    /// B-1: plain STATE.md body with no factory_lock section →
    /// Ok((RenewOutcome::NoOp, None)). resolve_identity must NOT be called.
    ///
    /// Without the B-1 pre-check, parse_factory_lock would return Err(Malformed)
    /// for a STATE.md with no closing --- delimiter, even when factory_lock: is absent.
    #[test]
    fn test_renew_lock_if_holder_no_factory_lock_key_returns_noop() {
        // fixture_no_lock() is a plain STATE.md with no factory_lock section.
        let result = renew_lock_if_holder(
            fixture_no_lock(),
            || {
                panic!(
                    "resolve_identity must NOT be called when factory_lock: key is absent (B-1 pre-check)"
                )
            },
            now_past,
        );
        let (outcome, skip) = result.expect("no factory_lock key must return Ok");
        match outcome {
            RenewOutcome::NoOp => {}
            RenewOutcome::Renewed(_) => {
                panic!("no factory_lock key must return NoOp, not Renewed (B-1)")
            }
        }
        assert!(
            skip.is_none(),
            "no factory_lock key must return None skip reason, got: {:?}",
            skip
        );
    }

    // -----------------------------------------------------------------------
    // B-3 test — now_fn called exactly once in case 5 (SEC-004 / FnOnce gate)
    // -----------------------------------------------------------------------

    /// B-3: now_fn must be called exactly once in the case 5 (success) path.
    /// The FnOnce bound on now_fn encodes this invariant at the type level —
    /// re-passing now_fn directly to renew_lock_with_now becomes a compile error.
    #[test]
    fn test_renew_lock_if_holder_now_fn_called_exactly_once() {
        use std::sync::{Arc, Mutex};

        let count = Arc::new(Mutex::new(0u32));
        let c = count.clone();
        let result = renew_lock_if_holder(
            fixture_valid_unexpired_lock(),
            || IdentityResolution::Resolved("holder@example.com".to_string()),
            move || {
                *c.lock().unwrap() += 1;
                now_past()
            },
        );
        assert!(result.is_ok(), "case 5 must return Ok");
        assert_eq!(
            *count.lock().unwrap(),
            1,
            "now_fn must be called exactly once for case 5 (SEC-004 / B-3)"
        );
        match result.unwrap().0 {
            RenewOutcome::Renewed(_) => {}
            RenewOutcome::NoOp => panic!("case 5 must return Renewed, not NoOp (B-3)"),
        }
    }

    // -----------------------------------------------------------------------
    // BLOCKING-2 discriminating test — unclosed fence without factory_lock key
    // -----------------------------------------------------------------------

    /// BLOCKING-2: STATE.md with an unclosed frontmatter fence and NO factory_lock: key →
    /// Ok((RenewOutcome::NoOp, None)).
    ///
    /// Without the has_factory_lock_key guard, parse_factory_lock would return
    /// Err(Malformed("missing closing --- delimiter")), making this Err(LockError::Malformed).
    /// With the guard, the function short-circuits to Ok((NoOp, None)).
    ///
    /// This is the discriminating input that distinguishes the B-1 pre-check from
    /// the weaker fixture_no_lock() (which already returns Ok(None) via Case 0 even
    /// without the pre-check, because parse_factory_lock returns Ok(None) for a
    /// well-formed frontmatter with no lock key).
    #[test]
    fn test_renew_lock_if_holder_unclosed_fence_no_lock_key_returns_noop() {
        // Content has an unclosed frontmatter fence but no factory_lock: key.
        // Without the has_factory_lock_key guard, parse_factory_lock would return
        // Err(Malformed("missing closing --- delimiter")), making this Err(LockError::Malformed).
        // With the guard, the function short-circuits to Ok((NoOp, None)).
        let content =
            "---\ndocument_type: state\nphase: test\n\n# STATE body without closing fence";
        let mut called = 0u32;
        let result = renew_lock_if_holder(
            content,
            || {
                called += 1;
                IdentityResolution::Resolved("anyone@example.com".to_string())
            },
            chrono::Utc::now,
        );
        assert_eq!(result, Ok((RenewOutcome::NoOp, None)));
        assert_eq!(
            called, 0,
            "resolve_identity must not be called when factory_lock key is absent"
        );
    }

    // -----------------------------------------------------------------------
    // BLOCKING-3 discriminating test — holder with trailing whitespace still matches
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // S-17.05 Red Gate test — renew_lock_with_now literal migration (T-2 sub-task, D-1126)
    // -----------------------------------------------------------------------

    /// S-17.05 T-2 sub-task (D-1126 boundary correction) / BC-4.17.001 Invariant 3 / ADR-046 F-006:
    /// renew_lock_with_now must use factory_lock_parse::TTL_SECONDS, NOT the bare literal 2700.
    ///
    /// Source-scan: grep crates/factory-lock/src/lib.rs for `Duration::seconds(2700)` inside
    /// the renew_lock_with_now function body. Asserts no match — i.e., the migration from
    /// `Duration::seconds(2700)` to `Duration::seconds(i64::from(factory_lock_parse::TTL_SECONDS))`
    /// has been applied.
    ///
    /// RED GATE: stub still contains `Duration::seconds(2700)` at the bare-literal location
    /// → assertion fails until implementer migrates in S-17.05 T-2.
    /// GREEN after: `Duration::seconds(i64::from(factory_lock_parse::TTL_SECONDS))` replaces it.
    #[test]
    fn test_renew_lock_with_now_uses_ttl_seconds_not_bare_literal() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let lib_path = std::path::Path::new(manifest_dir).join("src/lib.rs");
        let source = std::fs::read_to_string(&lib_path)
            .unwrap_or_else(|e| panic!("failed to read {:?}: {}", lib_path, e));

        // Locate the renew_lock_with_now function body.
        let fn_marker = "fn renew_lock_with_now<F>";
        let fn_start = source
            .find(fn_marker)
            .unwrap_or_else(|| panic!("renew_lock_with_now not found in {:?}", lib_path));
        // Bound the search to just the renew_lock_with_now function body — NOT to EOF.
        // Without this bound, the slice captures the test module's own source, which
        // contains the very string being searched for in the contains() call below,
        // causing a false-positive self-match regardless of production code state.
        // Find the next `\npub fn ` boundary after the function signature start.
        let after_sig = fn_start + fn_marker.len();
        let fn_end = source[after_sig..]
            .find("\npub fn ")
            .or_else(|| source[after_sig..].find("\nfn "))
            .map(|rel| after_sig + rel)
            .unwrap_or(source.len());
        let fn_body = &source[fn_start..fn_end];

        // Assert that the bare literal `Duration::seconds(` followed by `2700)` is absent.
        // After S-17.05 T-2 migration: Duration::seconds(i64::from(factory_lock_parse::TTL_SECONDS)).
        // The searched literal is assembled here to prevent this test's own source
        // from appearing in fn_body if the boundary calculation is ever widened.
        let bare_literal = format!("Duration::seconds({})", 2700u32);
        assert!(
            !fn_body.contains(&bare_literal),
            "renew_lock_with_now in {:?} must NOT use the bare-literal TTL form. \
             Migrate to Duration::seconds(i64::from(factory_lock_parse::TTL_SECONDS)) per \
             S-17.05 T-2 (D-1126 / BC-4.17.001 Invariant 3 / ADR-046 F-006). \
             Found bare literal — Red Gate armed.",
            lib_path
        );
    }

    // -----------------------------------------------------------------------
    // S-17.06 CR-003 coverage fold-in: Case 5 spurious-renewal guard
    // (already-merged S-17.06 code; this test PASSES immediately)
    // -----------------------------------------------------------------------

    /// S-17.06 CR-003 (coverage fold-in, independent review):
    /// renew_lock_if_holder Case 5 returns Ok((NoOp, None)) when expires_at is exactly
    /// now + TTL_SECONDS at second precision (spurious-renewal guard, F-R3-005).
    ///
    /// Fixture: factory_lock with holder = "holder@example.com" (matches caller),
    /// expires_at = base + 2700s (i.e., the precise value renew_lock_with_now would compute
    /// from the same `base` now_fn). The lock is NOT expired (base < base + 2700s).
    ///
    /// Expected: Ok((RenewOutcome::NoOp, None)) — the spurious-renewal guard in
    /// renew_lock_with_now suppresses the write (new_expires_at == existing expires_at).
    ///
    /// IMPORTANT: This is NOT a Case-0 absent-block misclassification. The lock block IS
    /// present and valid, the identity DOES match, and now < expires_at. The NoOp arises
    /// solely from the same-second recomputation guard (F-R3-005 / ADR-028 §Decision 16).
    ///
    /// PASSES immediately: S-17.06 code is live on develop (3200149d).
    #[test]
    fn test_renew_lock_if_holder_case5_spurious_renewal_guard_returns_noop() {
        use chrono::Duration;

        // Fixed base instant; expires_at is exactly base + 2700s formatted as YYYY-MM-DDTHH:MM:SSZ.
        let base: DateTime<Utc> = "2026-08-28T10:00:00Z"
            .parse::<DateTime<Utc>>()
            .expect("base timestamp must parse");
        let expires_str = (base + Duration::seconds(2700))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        // expires_str = "2026-08-28T10:45:00Z"

        let content = format!(
            concat!(
                "---\n",
                "document_type: state\n",
                "version: \"test\"\n",
                "phase: test\n",
                "factory_lock:\n",
                "  holder: \"holder@example.com\"\n",
                "  locked_at: \"2026-08-28T10:00:00Z\"\n",
                "  expires_at: \"{expires}\"\n",
                "---\n\n# STATE\n",
            ),
            expires = expires_str,
        );

        // now_fn returns `base` — same instant used to build expires_str above.
        // now (base) < expires_at (base + 2700s) → Case 5 identity step is reached.
        // Inside renew_lock_with_now: new_expires_at = base + 2700s = expires_str → guard fires → NoOp.
        let result = renew_lock_if_holder(
            &content,
            || IdentityResolution::Resolved("holder@example.com".to_string()),
            move || base,
        );

        let (outcome, skip) = result.expect("Case 5 spurious-renewal must return Ok, not Err");
        assert!(
            matches!(outcome, RenewOutcome::NoOp),
            "CR-003: spurious-renewal guard must return NoOp when expires_at == now + TTL_SECONDS \
             (F-R3-005 / ADR-028 §Decision 16). \
             This is NOT a Case-0 absent-block — the lock IS present and identity DOES match. \
             Got: {outcome:?}"
        );
        assert!(
            skip.is_none(),
            "CR-003: Case 5 spurious-renewal path must return None skip reason. Got: {skip:?}"
        );
    }

    /// BLOCKING-3: holder field in the YAML has trailing whitespace; resolve_identity
    /// returns the trimmed email (no trailing whitespace) → Case 5 fires: Renewed.
    ///
    /// Without trim_git_email(&state.holder), the comparison fails (Case 3: NotHolder).
    /// With the fix, both sides are trimmed and Case 5 fires: Renewed.
    ///
    /// This is the discriminating test for the B-2 fix in renew_lock_if_holder:
    /// all existing fixtures have clean holder values so reverting the trim leaves
    /// all prior tests green. This test fails on the revert.
    #[test]
    fn test_renew_lock_if_holder_holder_with_trailing_whitespace_still_matches() {
        // The holder field in the YAML has trailing whitespace.
        // resolve_identity returns the trimmed email (no trailing whitespace).
        // Without trim_git_email(&state.holder), the comparison fails (Case 3: NotHolder).
        // With the fix, both sides are trimmed and Case 5 fires: Renewed.
        //
        // Build fixture content with holder that has trailing whitespace.
        // Use a fixed far-future expires_at (2099) so Case 2 doesn't fire AND the
        // spurious-renewal guard (new_expires_at == existing expires_at) cannot fire —
        // now_fn() + 2700s is always ~2026+45min, never 2099.
        let locked_at = "2026-08-28T00:00:00Z";
        let expires_str = "2099-01-01T10:45:00Z";
        let content = format!(
            "---\ndocument_type: state\nfactory_lock:\n  holder: holder@example.com   \n  locked_at: {locked_at}\n  expires_at: {expires_str}\n---\n"
        );
        let result = renew_lock_if_holder(
            &content,
            || IdentityResolution::Resolved("holder@example.com".to_string()),
            chrono::Utc::now,
        );
        match result {
            Ok((RenewOutcome::Renewed(_), None)) => {} // expected
            other => panic!("Expected Renewed but got {:?}", other),
        }
    }
}
