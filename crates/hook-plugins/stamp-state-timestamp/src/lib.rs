//! stamp-state-timestamp — PostToolUse WASM hook plugin (S-17.05 / ADR-046).
//!
//! Fires on every `PostToolUse` event for `Edit`, `Write`, or `MultiEdit` tools
//! that land a write to `.factory/STATE.md`.
//!
//! On each invocation the hook:
//!   1. Reads the on-disk `.factory/STATE.md` via `host::read_file` (full content,
//!      post-write — the tool's write has already landed by the time PostToolUse fires).
//!   2. Validates frontmatter: UTF-8, opening `---\n` / `---\r\n` fence
//!      (`content.starts_with`), and closing `---` delimiter (via
//!      `factory_lock_parse::extract_frontmatter`). On any structural failure
//!      (non-UTF-8, opening fence absent, closing delimiter absent): fail-open
//!      (no write, `StampingSkipped` advisory) per PC3a.
//!      When frontmatter is valid but lacks a `timestamp:` anchor line: PC1 is
//!      suppressed (GAP-1 advisory emitted) while PC2 still runs (AC-016 / EC-013).
//!   3. PC1 (unconditional timestamp re-stamp): replaces the `timestamp:` line with
//!      `timestamp: <now_fn() formatted as YYYY-MM-DDTHH:MM:SSZ>`, regardless of what
//!      the agent's own write proposed.
//!   4. PC2 (identity-gated expires_at renewal): calls `callbacks.exec_subprocess`
//!      to get git email, classifies via `factory_lock::classify_identity_resolution`,
//!      calls `factory_lock::renew_lock_if_holder` to conditionally rewrite
//!      `factory_lock.expires_at = now + TTL_SECONDS` only when the resolved identity
//!      byte-equals the recorded `holder`. Foreign or expired holders are never renewed.
//!   5. Writes the reconstructed full content (frontmatter substitution + unchanged body)
//!      back via `host::write_file`. On write error: swallow (fail-open per PC3).
//!   6. Never touches acquire/release/CAS-push (PC5 — `event = "PostToolUse"`,
//!      `tool = "^(Edit|Write|MultiEdit)$"` in the registry ensures structural exclusion).
//!
//! # Behavioral Contracts
//!
//! - BC-4.17.001 — stamp-state-timestamp PostToolUse hook — unconditional
//!   `timestamp:` re-stamp (PC1), identity-gated `expires_at` renewal (PC2),
//!   fail-open (PC3), idempotent frontmatter-only rewrite (PC4), no lock-lifecycle
//!   involvement (PC5).
//! - BC-5.40.001 PC4: mid-burst TTL keep-alive — actor reassigned to this hook.
//!
//! # Architecture compliance
//!
//! - HOST_ABI_VERSION = 1 (ADR-025 Decision 1; BC-4.17.001 architecture compliance).
//! - No `serde_yaml` / `serde_norway` (Architecture Compliance Rule 6; manual
//!   line-by-line frontmatter scan via `factory_lock_parse::extract_frontmatter`).
//! - No `regex` crate (Architecture Compliance Rule 6).
//! - No independent TTL literal (Architecture Compliance Rule 7; imports
//!   `factory_lock_parse::TTL_SECONDS` — never redeclares 2700 as its own const/literal).
//! - Frontmatter-only rewrite (Architecture Compliance Rule 8; body bytes never read
//!   for decision purposes or modified).
//! - Identity comparison via `factory_lock::renew_lock_if_holder` →
//!   `factory_lock::classify_identity_resolution` (Architecture Compliance Rule 9;
//!   no direct `trim_git_email` call; flows through renew_lock_if_holder transparently).
//! - `async = false` required in registry entry (ADR-019; ADR-046; BC-4.17.001).
//! - `on_error = "continue"` (BC-4.17.001 PC3/Invariant 4; fail-open; no block_intent).
//! - Pure `fn guard_logic(...)` takes all host I/O as injectable closures;
//!   unit tests exercise every branch without a WASM runtime.

// Allow `#[cfg(kani)]` without triggering unexpected_cfgs warning.
#![cfg_attr(not(kani), allow(unexpected_cfgs))]

use chrono::{DateTime, Utc};
use factory_lock_parse as flp;
use vsdd_hook_sdk::{HookPayload, HookResult};

// ---------------------------------------------------------------------------
// ABI version constant (BC-4.17.001 architecture compliance)
// ---------------------------------------------------------------------------

/// HOST_ABI_VERSION declares the ABI contract version this plugin was built
/// against. The dispatcher reads this before any host call. Must remain 1.
pub const HOST_ABI_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Injectable callbacks surface (testable without WASM runtime)
// ---------------------------------------------------------------------------

/// All side-effecting host calls injected into `guard_logic` for testability.
/// In production (`on_post_tool_use`), these are wired to real vsdd_hook_sdk host fns.
///
/// Mirrors `verify-factory-lock`'s `GuardCallbacks` pattern, extended with
/// `write_file` (PostToolUse write-back), `now_fn` (injectable clock for
/// deterministic testing of AC-003/AC-010 timestamp-equality assertions),
/// `log_warn` and `emit_event` (PC3b observability for Case-4 identity-resolution
/// failure and Case-1 malformed-block advisory; BC-4.17.001 PC3b).
///
/// `log_warn` and `emit_event` use `Fn` (not `FnOnce`) because guard_logic may call
/// each more than once: `log_warn` is called for both GAP-1 `TimestampAnchorMissing`
/// and PC2 Case-1 malformed advisory; `emit_event` is called for both GAP-4
/// `state_md_approaching_cap` and PC2 Case-4 `renewal_indeterminate`. S-17.07
/// precompact-flush uses the same `Fn` bound for consistency (adjudicated by
/// orchestrator per story spec).
pub struct StampCallbacks<R, W, E, NF, LW, EV>
where
    R: FnOnce(&str, u32, u32) -> Result<Vec<u8>, String>,
    W: FnOnce(&str, &[u8]) -> Result<(), String>,
    E: FnOnce(&[&str]) -> Result<(i32, String), String>,
    NF: FnOnce() -> DateTime<Utc>,
    LW: Fn(&str),
    EV: Fn(&str, &[(&str, &str)]),
{
    /// Read the full content of `.factory/STATE.md` via `host::read_file`.
    /// `(path, max_bytes, timeout_ms)` → `Ok(bytes)` or `Err(host_error_description)`.
    /// Production caller passes `factory_lock_parse::STATE_MD_MAX_BYTES` as `max_bytes`
    /// (BC-5.40.001 / BC-4.13.001 cap-parity requirement; AC-008).
    pub read_file: R,
    /// Write the rewritten content back to `.factory/STATE.md` via `host::write_file`.
    /// `(path, content_bytes)` → `Ok(())` or `Err(host_error_description)`.
    pub write_file: W,
    /// Execute a subprocess with the given argv slice via `host::exec_subprocess`.
    /// Returns `Ok((exit_code, stdout))` or `Err(host_error_description)`.
    /// Used for `git config user.email` identity resolution (PC2 gate; AC-007).
    pub exec_subprocess: E,
    /// Injectable clock returning the current UTC instant.
    /// Production: `Utc::now`. Tests: inject a fixed timestamp for deterministic
    /// AC-003/AC-010 `expires_at = now + TTL_SECONDS` assertions.
    pub now_fn: NF,
    /// Emit an advisory warn-level log line via `host::log_warn` (best-effort; PC3b).
    /// Called on Case-4 (IdentityResolutionFailed), Case-1 (Malformed), GAP-1
    /// (TimestampAnchorMissing), and GAP-7 (OutputTooLarge) paths.
    /// Production: `|msg| host::log_warn(msg)`. Tests: no-op or capturing closure.
    /// `Fn` (not `FnOnce`): may be called more than once per invocation.
    pub log_warn: LW,
    /// Emit a structured observability event via `host::emit_event` (best-effort; PC3b).
    /// Called on Case-4 (IdentityResolutionFailed) and GAP-4 (state_md_approaching_cap).
    /// Production: `|et, fields| host::emit_event(et, fields)`. Tests: no-op or capturing.
    /// `Fn` (not `FnOnce`): may be called more than once per invocation.
    pub emit_event: EV,
}

// ---------------------------------------------------------------------------
// Core guard logic (injectable callbacks — testable without WASM runtime)
// ---------------------------------------------------------------------------

/// Core stamp-state-timestamp guard logic.
///
/// All host I/O is injected via `callbacks` so unit tests can exercise every
/// branch without a WASM runtime.
///
/// Decision tree (per BC-4.17.001):
///   1. Call `read_file(".factory/STATE.md", STATE_MD_MAX_BYTES, timeout_ms)`.
///      On error: return `HookResult::Continue` (PC3 fail-open; no write).
///   2. Validate UTF-8, opening `---\n`, closing `---` delimiter, `timestamp:`
///      anchor line present. On any failure: return Continue (PC3).
///   3. PC1: replace `timestamp:` line → `timestamp: <now formatted YYYY-MM-DDTHH:MM:SSZ>`.
///   4. PC2: build a lazy `resolve_identity` closure wrapping `exec_subprocess` +
///      `factory_lock::classify_identity_resolution`, then call
///      `factory_lock::renew_lock_if_holder(&content_after_pc1, resolve_identity, || now)`.
///      The canonical 6-case decision tree (ADR-046 Decision 1(b)) handles:
///      Case 0 (absent/null → NoOp), Case 1 (malformed → Err → fail-open write PC1 only),
///      Case 2 (already expired → NoOp; ADR-046 anti-resurrection; SAFETY-CRITICAL),
///      Case 3 (not holder → NoOp), Case 4 (resolution failed → NoOp),
///      Case 5 (identity matches + not expired → Renewed).
///      An expired self-held lock is NEVER renewed even when identity matches.
///   5. Call `write_file(".factory/STATE.md", reconstructed_full_content)`.
///      On write error: swallow (PC3 fail-open; agent's write is not reverted).
///   6. Return `HookResult::Continue` (Invariant 4: no `block_intent` capability).
///
/// PC3b observability (BC-4.17.001 PC3b / F-S1705-P3-001):
///   On Case-4 (`Ok((NoOp, Some(SkipReason::IdentityResolutionFailed { ... })))`):
///   - `emit_event("factory.lock.renewal_indeterminate", &[("plugin", ...), ("holder", ...),
///     ("locked_at", ...), ("expires_at", ...), ("resolution_error", ...)])`, AND
///   - `log_warn(<human-readable message naming holder + reason>)`.
///   On Case-1 (`Err(LockError::Malformed(_))`): `log_warn` advisory only; no emit.
///   `NotHolder`, `AlreadyExpired`, `(NoOp, None)`: no diagnostic (PC3b non-goal).
///   The emit/log calls are best-effort; fail-open PC3 maintained regardless.
///
/// # BC traces
/// - BC-4.17.001 PC1: unconditional `timestamp:` re-stamp (AC-001, AC-002)
/// - BC-4.17.001 PC2: identity-gated `factory_lock.expires_at` renewal (AC-003..AC-007)
/// - BC-4.17.001 PC3: fail-open on any read/parse/UTF-8/write error (AC-008, AC-009)
/// - BC-4.17.001 PC3b: renewal_indeterminate emit+log on Case-4; advisory log on Case-1
/// - BC-4.17.001 PC4: idempotent, frontmatter-only rewrite (AC-010)
/// - BC-4.17.001 PC5: no acquire/release/CAS involvement (AC-011)
/// - BC-5.40.001 PC4: mid-burst TTL keep-alive (AC-014)
pub fn guard_logic<R, W, E, NF, LW, EV>(
    payload: HookPayload,
    callbacks: StampCallbacks<R, W, E, NF, LW, EV>,
) -> HookResult
where
    R: FnOnce(&str, u32, u32) -> Result<Vec<u8>, String>,
    W: FnOnce(&str, &[u8]) -> Result<(), String>,
    E: FnOnce(&[&str]) -> Result<(i32, String), String>,
    NF: FnOnce() -> DateTime<Utc>,
    LW: Fn(&str),
    EV: Fn(&str, &[(&str, &str)]),
{
    use factory_lock::{
        LockError, RenewOutcome, SkipReason, classify_identity_resolution, renew_lock_if_holder,
    };

    // Destructure callbacks — allows `Fn` types to be called multiple times by borrow.
    let StampCallbacks {
        read_file,
        write_file,
        exec_subprocess,
        now_fn,
        log_warn,
        emit_event,
    } = callbacks;

    // GAP 3 (Precondition 1 / F-013 tool-write-success):
    // Inspect payload.tool_response BEFORE reading the file.
    // Skip if tool_response is absent/null (None) or is an object with a non-null
    // top-level "error" key — both indicate the prior tool write did not succeed.
    // Accepted residual: a tool response present but with NO "error" key is treated
    // as success (harmless spurious re-stamp on ambiguous responses; BC-4.17.001 PC3).
    {
        let ok = match &payload.tool_response {
            None => false,
            Some(v) => v.get("error").is_none_or(|e| e.is_null()),
        };
        if !ok {
            return HookResult::Continue;
        }
    }

    // Step 1 (PC3): read STATE.md — fail-open on host error.
    let raw_bytes = match read_file(".factory/STATE.md", flp::STATE_MD_MAX_BYTES, 5000) {
        Ok(bytes) => bytes,
        Err(e) => {
            // GAP 7 (EC-015 / EC-005): emit StampingSkipped advisory on all read-error paths.
            // OutputTooLarge keeps its distinct cause string; all other read errors use
            // "read error: {e}" (EC-005 structural fail-open observability requirement).
            if e.contains("OutputTooLarge") {
                log_warn(&format!(
                    "stamp-state-timestamp: StateReadError/StampingSkipped: \
                     OutputTooLarge — {e}",
                ));
            } else {
                log_warn(&format!(
                    "stamp-state-timestamp: StampingSkipped: read error: {e}",
                ));
            }
            return HookResult::Continue;
        }
    };

    // GAP 4 (Invariant 8 soft-warn / AC-018):
    // STATE.md is approaching the read cap (262144 bytes). Emit an advisory event
    // so operators can compact the file before reads start failing (OutputTooLarge).
    // Does NOT suppress PC1/PC2 (observability only).
    let raw_bytes_len = raw_bytes.len();
    if raw_bytes_len > 200_000 && raw_bytes_len <= 262_144 {
        let bytes_read_str = raw_bytes_len.to_string();
        emit_event(
            "state_md_approaching_cap",
            &[("bytes_read", &bytes_read_str), ("cap_bytes", "262144")],
        );
    }

    // GAP 2 (Architecture Rule 6 / EC-017 CRLF):
    // Delegate fence detection to factory_lock_parse::extract_frontmatter instead of
    // a hand-rolled `starts_with("---\n")` / `find("\n---\n")` check.
    // extract_frontmatter handles LF inline (`\n---\n`), CRLF inline (`\r\n---\r\n`),
    // LF-EOF (`\n---`), and CRLF-EOF (`\r\n---`) delimiters (EC-017 Windows autocrlf).
    // When no delimiter is found it returns the full input — structural fail-open (PC3a).
    let fm_bytes_len = {
        let fm = flp::extract_frontmatter(&raw_bytes);
        let len = fm.len();
        if len == raw_bytes_len {
            // Fence not located: structural fail-open (PC3a — both PC1 and PC2 suppressed).
            // EC-005: emit StampingSkipped advisory so the path is observable.
            log_warn(
                "stamp-state-timestamp: StampingSkipped: \
                 missing/invalid frontmatter delimiters",
            );
            return HookResult::Continue;
        }
        len
    };
    // raw_bytes is still owned here; fm borrow has ended (block scope above).

    // Step 2 (PC3): validate UTF-8 — fail-open on non-UTF-8 content.
    // raw_bytes is consumed here; fm_bytes_len retains the byte boundary.
    let content = match String::from_utf8(raw_bytes) {
        Ok(s) => s,
        Err(_) => {
            // EC-005: emit StampingSkipped advisory on non-UTF-8 structural fail-open.
            log_warn("stamp-state-timestamp: StampingSkipped: STATE.md not valid UTF-8");
            return HookResult::Continue;
        }
    };

    // F-S1705-P6-001 (AC-008): opening-fence validation.
    // PC3a structural fail-open: content MUST begin with "---\n" (LF) or "---\r\n" (CRLF).
    // If the opening fence is absent, emit a distinct StampingSkipped advisory and return
    // Continue. This complements the closing-fence check above (which fires when
    // extract_frontmatter returns the full input unchanged, i.e., no closing delimiter).
    // Example: "garbage\n---\n" has a closing-delimiter pattern but no opening fence —
    // the closing-fence check passes but this opening-fence check catches it.
    if !content.starts_with("---\n") && !content.starts_with("---\r\n") {
        log_warn(
            "stamp-state-timestamp: StampingSkipped: \
             missing/invalid frontmatter delimiters (opening fence absent)",
        );
        return HookResult::Continue;
    }

    // EC-017: CRLF-aware opening-fence offset.
    // `---\r\n` = 5 bytes; `---\n` = 4 bytes.
    // All fm_body slices use fence_len instead of the hardcoded 4 so that CRLF-checked-out
    // STATE.md files are handled correctly. LF (the repo default) is unchanged.
    let fence_len: usize = if content.starts_with("---\r\n") { 5 } else { 4 };

    // fm_body: frontmatter text between opening fence and the closing delimiter,
    // bounded by fm_bytes_len from extract_frontmatter (CRLF-safe byte boundary).
    // fence_len (4 for LF, 5 for CRLF) skips the opening `---\n` or `---\r\n`.
    //
    // F-S1705-P6-001 invariant guard: empty-frontmatter case.
    // extract_frontmatter("---\n---\n") returns 3 bytes (fm_bytes_len=3 < fence_len=4):
    // the `\n` ending the opening fence IS the leading `\n` of the `\n---\n` closing
    // pattern, so there is zero content between the fences. Using fence_len.min(fm_bytes_len)
    // as the slice start yields an empty &str without panicking on start > end.
    //
    // Locate the "timestamp:" key line within fm_body.
    // Scoped block ensures the fm_body borrow ends before content is conditionally moved.
    let ts_pos_opt: Option<usize> = {
        let fm_start = fence_len.min(fm_bytes_len);
        let fm_body = &content[fm_start..fm_bytes_len];
        if fm_body.starts_with("timestamp:") {
            Some(0)
        } else {
            fm_body.find("\ntimestamp:").map(|p| p + 1)
        }
    };
    // fm_body borrow ends here.

    // Step 4 (PC1): build content_after_pc1.
    // now_fn is called exactly once here (FnOnce bound).
    let now = now_fn();
    let now_str = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let new_ts_line = format!("timestamp: {now_str}");

    // GAP 1 (EC-013 / PC3a scoped exception):
    // When frontmatter is VALID but has NO `timestamp:` anchor line:
    //   - PC1 is a no-op (content_after_pc1 = content unchanged; no timestamp rewrite).
    //   - Emit an advisory log_warn (best-effort).
    //   - PC2 still evaluated via renew_lock_if_holder (lock renewal may still occur).
    //   - On PC2 Renewed  → write Renewed content    (Invariant-9 row 3).
    //   - On PC2 NoOp     → write nothing            (Invariant-9 row 4).
    // Structural failures (no fence / non-UTF-8 / read error) STILL take the PC3
    // fail-open path (return Continue above) — those paths are unchanged.
    let (content_after_pc1, has_timestamp) = match ts_pos_opt {
        Some(ts_pos) => {
            // Nominal PC1 path: replace the timestamp: line.
            // Returns (ts_line_end_in_fm, crlf_line) where:
            //   ts_line_end_in_fm: position of '\n' at end of the old timestamp line (in fm_body coords)
            //   crlf_line: true if the old timestamp line ended with '\r\n' (EC-017)
            let (ts_line_end_in_fm, crlf_line) = {
                let fm_body = &content[fence_len..fm_bytes_len];

                // GAP 6 (EC-014 duplicate timestamp: advisory):
                // After locating the first timestamp: line, scan for a second one.
                // If found, log an advisory (only the first is rewritten; no data loss).
                let after_first_ts = &fm_body[ts_pos + "timestamp:".len()..];
                if after_first_ts.contains("\ntimestamp:") {
                    log_warn(
                        "stamp-state-timestamp: DuplicateTimestampKey: multiple \
                         `timestamp:` lines found in frontmatter (only the first \
                         will be rewritten)",
                    );
                }

                let ts_line_end = match fm_body[ts_pos..].find('\n') {
                    Some(rel) => ts_pos + rel,
                    None => fm_body.len(), // timestamp: is the last line (no trailing '\n')
                };
                // EC-017: detect CRLF line ending on the timestamp line.
                // ts_line_end points at '\n'; if the preceding byte is '\r', preserve it.
                let crlf =
                    ts_line_end > 0 && fm_body.as_bytes().get(ts_line_end - 1) == Some(&b'\r');
                (ts_line_end, crlf)
            };
            // fm_body borrow ends here.

            // content[..fence_len+ts_pos]         → opening fence + fm prefix before old timestamp: line
            // new_ts_line (+ optional '\r')        → "timestamp: <now>" with original line terminator
            // content[fence_len+ts_line_end_in_fm..] → '\n' (or '\r\n' tail) after old line + body
            let ts_line_with_term = format!("{}{}", new_ts_line, if crlf_line { "\r" } else { "" });
            let new_content = format!(
                "{}{}{}",
                &content[..fence_len + ts_pos],
                ts_line_with_term,
                &content[fence_len + ts_line_end_in_fm..],
            );
            (new_content, true)
        }
        None => {
            // GAP 1 (EC-013): valid frontmatter, no timestamp: anchor → PC1 no-op.
            log_warn(
                "stamp-state-timestamp: TimestampAnchorMissing: no `timestamp:` line found \
                 in frontmatter — PC1 skipped; lock renewal (PC2) will still run",
            );
            (content, false) // content moved here — no clone needed
        }
    };

    // Step 5 (PC2 + PC3b): identity-gated expires_at renewal via canonical renew_lock_if_holder.
    // The 6-case decision tree (ADR-046 Decision 1(b)) is fully encapsulated inside
    // renew_lock_if_holder — including the expiry gate (Case 2):
    //   Case 0 (absent/null) → NoOp, None
    //   Case 1 (malformed)   → Err(Malformed) → advisory log_warn; fail-open: write PC1 only
    //   Case 2 (already expired) → NoOp, Some(AlreadyExpired); ADR-046 anti-resurrection
    //   Case 3 (not holder)  → NoOp, Some(NotHolder)
    //   Case 4 (identity resolution failed) → NoOp, Some(IdentityResolutionFailed{..})
    //     → PC3b: emit_event("factory.lock.renewal_indeterminate") + log_warn (best-effort)
    //   Case 5 (identity matches, not expired) → Renewed
    let resolve_identity =
        move || classify_identity_resolution(exec_subprocess(&["git", "config", "user.email"]));
    let pc2_result = renew_lock_if_holder(&content_after_pc1, resolve_identity, move || now);

    // Determine what to write (Invariant-9 write rules):
    //   has_timestamp=true  + any PC2 outcome → write content_after_pc2
    //                         (PC1 already changed content; always write)
    //   has_timestamp=false + Renewed         → write Renewed content (PC2-only change)
    //   has_timestamp=false + NoOp/Err        → write nothing (row 4; no net change)
    let content_to_write: Option<String> = match pc2_result {
        Ok((RenewOutcome::Renewed(new_content), _)) => Some(new_content),
        // Case 4 — PC3b: identity-resolution failure → emit renewal_indeterminate event
        // and advisory log_warn. Best-effort observability; fail-open PC3 maintained.
        Ok((
            RenewOutcome::NoOp,
            Some(SkipReason::IdentityResolutionFailed {
                reason,
                holder,
                locked_at,
                expires_at,
            }),
        )) => {
            emit_event(
                "factory.lock.renewal_indeterminate",
                &[
                    ("plugin", "stamp-state-timestamp"),
                    ("holder", &holder),
                    ("locked_at", &locked_at),
                    ("expires_at", &expires_at),
                    ("resolution_error", &reason),
                ],
            );
            log_warn(&format!(
                "stamp-state-timestamp: renewal_indeterminate — holder: {holder}, \
                 reason: {reason}",
            ));
            if has_timestamp {
                Some(content_after_pc1)
            } else {
                None
            }
        }
        // Cases 0, 2, 3 — no diagnostic (PC3b non-goal).
        Ok((RenewOutcome::NoOp, Some(SkipReason::NotHolder)))
        | Ok((RenewOutcome::NoOp, Some(SkipReason::AlreadyExpired)))
        | Ok((RenewOutcome::NoOp, None)) => {
            if has_timestamp {
                Some(content_after_pc1)
            } else {
                None
            }
        }
        // Case 1 — malformed block: advisory log_warn only; no emit.
        Err(LockError::Malformed(msg)) => {
            log_warn(&format!(
                "stamp-state-timestamp: malformed factory_lock block (advisory): {msg}",
            ));
            if has_timestamp {
                Some(content_after_pc1)
            } else {
                None
            }
        }
    };

    // Step 6 (PC3): write back — fail-open on write error (agent's write not reverted).
    if let Some(content_final) = content_to_write {
        let _ = write_file(".factory/STATE.md", content_final.as_bytes());
    }

    HookResult::Continue
}

// ---------------------------------------------------------------------------
// Top-level entry point (wired to real host fns — WIRING-EXEMPT per BC-5.38.003)
// ---------------------------------------------------------------------------

/// Called from the WASI entry point in `main.rs`.
///
/// Wires the real vsdd_hook_sdk host functions to the injectable-callback
/// surface of `guard_logic`. WIRING-EXEMPT (BC-5.38.003): this function is
/// purely delegating to `guard_logic` via single-call wiring to each host
/// capability; no domain logic resides here.
///
/// host::exec_subprocess signature: `(cmd, args, stdin, timeout_ms, max_output_bytes)`.
/// We call `git config user.email` with no stdin. Max output is 512 bytes
/// (email addresses are short; prevents wasting WASM fuel on large output).
///
/// host::read_file uses `STATE_MD_MAX_BYTES` as the byte cap
/// (BC-5.40.001 / BC-4.13.001 cap-parity requirement).
#[allow(dead_code)]
pub fn on_post_tool_use(payload: HookPayload) -> HookResult {
    use vsdd_hook_sdk::host;

    guard_logic(
        payload,
        StampCallbacks {
            read_file: |path, max_bytes, timeout_ms| match host::read_file(
                path, max_bytes, timeout_ms,
            ) {
                Ok(bytes) => Ok(bytes),
                Err(e) => Err(format!("{:?}", e)),
            },
            write_file: |path, content| match host::write_file(
                path,
                content,
                flp::STATE_MD_MAX_BYTES,
                5000,
            ) {
                Ok(()) => Ok(()),
                Err(e) => Err(format!("{:?}", e)),
            },
            exec_subprocess: |argv| match argv.split_first() {
                Some((cmd, args)) => match host::exec_subprocess(cmd, args, &[], 5000, 512) {
                    Ok(result) => {
                        let stdout = String::from_utf8_lossy(&result.stdout).into_owned();
                        Ok((result.exit_code, stdout))
                    }
                    Err(e) => Err(format!("{:?}", e)),
                },
                None => Err("exec_subprocess: empty argv".to_string()),
            },
            now_fn: Utc::now,
            // PC3b: wire best-effort observability callbacks to host functions.
            // These are fire-and-forget; host::log_warn and host::emit_event always succeed.
            log_warn: |msg| host::log_warn(msg),
            emit_event: |event_type, fields| host::emit_event(event_type, fields),
        },
    )
}

// ---------------------------------------------------------------------------
// S-17.05 stamp-state-timestamp test suite (BC-5.38.001 strict tdd_mode)
//
// 32 unit tests: 30 exercise guard_logic() with injected callbacks, covering
// BC-4.17.001 PC1–PC5 + AC-001..AC-018 + EC-013..EC-017;
// 2 assert on canonical constants (do NOT call guard_logic):
//   - test_ttl_seconds_constant_equals_2700
//   - test_ttl_seconds_is_imported_not_redeclared
//
// Test naming follows the S-17.05 Red Gate Test Table (authoritative).
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::missing_panics_doc,
        clippy::type_complexity
    )]

    use super::*;
    use chrono::{DateTime, Duration, Utc};
    use factory_lock_parse as flp;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    // -----------------------------------------------------------------------
    // HookPayload fixture builder
    // -----------------------------------------------------------------------

    /// Build a minimal PostToolUse HookPayload for the given tool name targeting STATE.md.
    fn payload_for_post_tool_use(tool_name: &str) -> HookPayload {
        serde_json::from_value(json!({
            "event_name": "PostToolUse",
            "tool_name": tool_name,
            "session_id": "test-session",
            "dispatcher_trace_id": "test-trace",
            "tool_input": { "file_path": ".factory/STATE.md" },
            // Minimal successful tool_response: present (non-null), no "error" key.
            // Satisfies GAP-3 Precondition-1 check (F-013 tool-write-success gate).
            // test-writer will add dedicated fixtures for the failure-response paths.
            "tool_response": {}
        }))
        .expect("fixture HookPayload must deserialize")
    }

    // -----------------------------------------------------------------------
    // STATE.md content fixture builders
    // -----------------------------------------------------------------------

    /// STATE.md with a stale timestamp and NO factory_lock block.
    fn state_no_lock_old_ts() -> String {
        concat!(
            "---\n",
            "document_type: state\n",
            "version: \"0.0.1-test\"\n",
            "timestamp: 2020-01-01T00:00:00Z\n",
            "phase: test\n",
            "---\n",
            "\n# STATE\n",
            "Body content here.\n",
        )
        .to_string()
    }

    /// STATE.md with stale timestamp, factory_lock.holder = "holder@example.com" (foreign).
    /// Caller identity is "caller@example.com" in these tests — deliberately mismatches.
    fn state_with_foreign_lock(expires_at: &str) -> String {
        format!(
            concat!(
                "---\n",
                "document_type: state\n",
                "version: \"0.0.1-test\"\n",
                "timestamp: 2020-01-01T00:00:00Z\n",
                "phase: test\n",
                "factory_lock:\n",
                "  holder: \"holder@example.com\"\n",
                "  locked_at: \"2026-01-01T10:00:00Z\"\n",
                "  expires_at: \"{expires}\"\n",
                "---\n",
                "\n# STATE\n",
                "Body content here.\n",
            ),
            expires = expires_at,
        )
    }

    /// STATE.md with stale timestamp, factory_lock.holder = "caller@example.com" (self-held).
    /// Identity match: caller == holder → PC2 gate opens → renewal occurs.
    fn state_with_self_lock() -> String {
        concat!(
            "---\n",
            "document_type: state\n",
            "version: \"0.0.1-test\"\n",
            "timestamp: 2020-01-01T00:00:00Z\n",
            "phase: test\n",
            "factory_lock:\n",
            "  holder: \"caller@example.com\"\n",
            "  locked_at: \"2026-01-01T10:00:00Z\"\n",
            "  expires_at: \"2099-01-01T00:00:00Z\"\n",
            "---\n",
            "\n# STATE\n",
            "Body content here.\n",
        )
        .to_string()
    }

    /// STATE.md with factory_lock.holder = "" (empty string).
    /// parse_factory_lock returns Err(MalformedLockBlock) for empty holder.
    /// PC2 gate must skip renewal; PC1 timestamp stamp must still proceed.
    fn state_with_empty_holder() -> String {
        concat!(
            "---\n",
            "document_type: state\n",
            "version: \"0.0.1-test\"\n",
            "timestamp: 2020-01-01T00:00:00Z\n",
            "phase: test\n",
            "factory_lock:\n",
            "  holder: \"\"\n",
            "  locked_at: \"2026-01-01T10:00:00Z\"\n",
            "  expires_at: \"2099-01-01T00:00:00Z\"\n",
            "---\n",
            "\n# STATE\n",
        )
        .to_string()
    }

    /// Malformed STATE.md: valid opening `---` but no closing `---` delimiter.
    fn state_malformed_no_closing_delimiter() -> String {
        concat!(
            "---\n",
            "document_type: state\n",
            "timestamp: 2020-01-01T00:00:00Z\n",
            "phase: test\n",
        )
        .to_string()
        // No closing `---` → extract_frontmatter / parse_factory_lock sees malformed structure
    }

    /// Valid STATE.md frontmatter but NO `timestamp:` anchor line.
    fn state_no_timestamp_line() -> String {
        concat!(
            "---\n",
            "document_type: state\n",
            "version: \"0.0.1-test\"\n",
            "phase: test\n",
            "---\n",
            "\n# STATE\n",
        )
        .to_string()
    }

    /// Fixed UTC instant for deterministic test assertions.
    /// 2026-08-27T12:00:00Z → now + 2700s = 2026-08-27T12:45:00Z.
    fn fixed_now() -> DateTime<Utc> {
        "2026-08-27T12:00:00Z"
            .parse::<DateTime<Utc>>()
            .expect("fixed_now must parse as DateTime<Utc>")
    }

    // -----------------------------------------------------------------------
    // Additional fixture helpers (8 consolidated-conformance tests)
    // -----------------------------------------------------------------------

    /// Build a valid STATE.md byte-vec of exactly `total_len` bytes.
    /// Frontmatter is minimal (timestamp line + LF fences); body is padded with 'A'.
    /// Used for soft-warn boundary tests (AC-018 / Invariant 8 / GAP-4).
    fn state_padded_to(total_len: usize) -> Vec<u8> {
        let header: &[u8] = b"---\ntimestamp: 2020-01-01T00:00:00Z\n---\n";
        assert!(
            total_len >= header.len(),
            "total_len {total_len} must be >= header len {}",
            header.len()
        );
        let mut content = header.to_vec();
        content.resize(total_len, b'A');
        content
    }

    /// Valid frontmatter with NO `timestamp:` line but WITH a self-held, far-future lock.
    /// Used by EC-013 Invariant-9 row 3 test (PC2-only write when no timestamp anchor).
    fn state_no_timestamp_self_lock_far_future() -> String {
        concat!(
            "---\n",
            "document_type: state\n",
            "version: \"0.0.1-test\"\n",
            "phase: test\n",
            "factory_lock:\n",
            "  holder: \"caller@example.com\"\n",
            "  locked_at: \"2026-01-01T10:00:00Z\"\n",
            "  expires_at: \"2099-01-01T00:00:00Z\"\n",
            "---\n",
            "\n# STATE\n",
        )
        .to_string()
    }

    /// Frontmatter with two `timestamp:` lines (EC-014 duplicate-timestamp advisory).
    fn state_duplicate_timestamp() -> String {
        concat!(
            "---\n",
            "document_type: state\n",
            "timestamp: 2020-01-01T00:00:00Z\n",
            "timestamp: 2021-06-15T00:00:00Z\n",
            "phase: test\n",
            "---\n",
            "\n# STATE\n",
        )
        .to_string()
    }

    /// STATE.md with CRLF (`\r\n`) line endings including the fence lines (EC-017).
    fn state_crlf_valid() -> Vec<u8> {
        b"---\r\ndocument_type: state\r\ntimestamp: 2020-01-01T00:00:00Z\r\nphase: test\r\n---\r\n\r\n# STATE\r\n".to_vec()
    }

    // -----------------------------------------------------------------------
    // AC-001: PC1 unconditional timestamp re-stamp (no lock present)
    // -----------------------------------------------------------------------

    /// test_timestamp_always_restamped_no_lock_present (AC-001)
    ///
    /// BC-4.17.001 PC1: timestamp: re-stamped regardless of lock presence.
    /// Fixture: STATE.md with stale timestamp, no factory_lock block.
    /// Expected: write_file called; written content has timestamp = fixed_now;
    ///           no factory_lock block created.
    ///
    /// RED GATE: guard_logic is todo!() → panics before write_file can be called.
    #[test]
    fn test_timestamp_always_restamped_no_lock_present() {
        let content = state_no_lock_old_ts();
        let written = Arc::new(Mutex::new(None::<Vec<u8>>));
        let w = written.clone();

        let _result = guard_logic(
            payload_for_post_tool_use("Edit"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content.into_bytes()),
                write_file: move |_path, bytes| {
                    *w.lock().unwrap() = Some(bytes.to_vec());
                    Ok(())
                },
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: |_msg| {},
                emit_event: |_event_type, _fields| {},
            },
        );

        let written_bytes = written
            .lock()
            .unwrap()
            .clone()
            .expect("AC-001: write_file must be called — PC1 always stamps");
        let written_str = String::from_utf8_lossy(&written_bytes);
        assert!(
            written_str.contains("timestamp: 2026-08-27T12:00:00Z"),
            "AC-001 PC1: timestamp must be re-stamped to now (2026-08-27T12:00:00Z). \
             Got:\n{written_str:?}"
        );
        assert!(
            !written_str.contains("timestamp: 2020-01-01T00:00:00Z"),
            "AC-001 PC1: original stale timestamp must NOT remain"
        );
    }

    // -----------------------------------------------------------------------
    // AC-002: PC1 + Invariant 1 — timestamp re-stamped even with identity mismatch
    // -----------------------------------------------------------------------

    /// test_timestamp_restamped_when_lock_held_regardless_of_identity_match (AC-002)
    ///
    /// BC-4.17.001 PC1 / Invariant 1: timestamp re-stamp has no identity gate.
    /// Fixture: STATE.md with foreign holder (identity MISMATCH with caller).
    /// Expected: timestamp changed; expires_at byte-identical (mismatch → no renewal).
    ///
    /// RED GATE: guard_logic is todo!() → panics.
    #[test]
    fn test_timestamp_restamped_when_lock_held_regardless_of_identity_match() {
        let original_expires = "2099-01-01T00:00:00Z";
        let content = state_with_foreign_lock(original_expires);
        let written = Arc::new(Mutex::new(None::<Vec<u8>>));
        let w = written.clone();

        let _result = guard_logic(
            payload_for_post_tool_use("Write"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content.into_bytes()),
                write_file: move |_path, bytes| {
                    *w.lock().unwrap() = Some(bytes.to_vec());
                    Ok(())
                },
                // Identity MISMATCH: "caller@example.com" != "holder@example.com"
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: |_msg| {},
                emit_event: |_event_type, _fields| {},
            },
        );

        let written_bytes = written
            .lock()
            .unwrap()
            .clone()
            .expect("write_file must be called");
        let written_str = String::from_utf8_lossy(&written_bytes);
        assert!(
            written_str.contains("timestamp: 2026-08-27T12:00:00Z"),
            "AC-002 Invariant 1: timestamp MUST be re-stamped even when identity mismatches. \
             Got:\n{written_str:?}"
        );
        assert!(
            written_str.contains(original_expires),
            "AC-002: expires_at must remain byte-identical on identity mismatch. \
             Got:\n{written_str:?}"
        );
    }

    // -----------------------------------------------------------------------
    // AC-003: PC2 identity-gate row 1 — identity match renews expires_at
    // -----------------------------------------------------------------------

    /// test_identity_match_renews_expires_at (AC-003)
    ///
    /// BC-4.17.001 PC2: identity match → expires_at = now + TTL_SECONDS;
    /// holder + locked_at must remain byte-identical.
    ///
    /// RED GATE: guard_logic is todo!() → panics.
    #[test]
    fn test_identity_match_renews_expires_at() {
        let content = state_with_self_lock();
        let written = Arc::new(Mutex::new(None::<Vec<u8>>));
        let w = written.clone();
        // Expected expires_at = fixed_now() + 2700s = 2026-08-27T12:45:00Z
        let expected_expires = (fixed_now() + Duration::seconds(2700))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();

        let _result = guard_logic(
            payload_for_post_tool_use("MultiEdit"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content.into_bytes()),
                write_file: move |_path, bytes| {
                    *w.lock().unwrap() = Some(bytes.to_vec());
                    Ok(())
                },
                // Identity MATCH: "caller@example.com" == holder "caller@example.com"
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: |_msg| {},
                emit_event: |_event_type, _fields| {},
            },
        );

        let written_bytes = written
            .lock()
            .unwrap()
            .clone()
            .expect("write_file must be called");
        let written_str = String::from_utf8_lossy(&written_bytes);
        assert!(
            written_str.contains(&format!("expires_at: {expected_expires}")),
            "AC-003 PC2: expires_at must equal now + TTL_SECONDS ({expected_expires}). \
             Got:\n{written_str:?}"
        );
        // PC4: holder and locked_at must remain byte-identical
        assert!(
            written_str.contains("holder: \"caller@example.com\""),
            "AC-003 PC4: holder must remain byte-identical after renewal"
        );
        assert!(
            written_str.contains("locked_at: \"2026-01-01T10:00:00Z\""),
            "AC-003 PC4: locked_at must remain byte-identical after renewal"
        );
        // Stale expires_at must be gone
        assert!(
            !written_str.contains("2099-01-01T00:00:00Z"),
            "AC-003: stale fixture expires_at must be replaced by the renewed value"
        );
    }

    // -----------------------------------------------------------------------
    // AC-004: PC2 identity-gate row 2 — no lock block → no renewal attempted
    // -----------------------------------------------------------------------

    /// test_no_lock_block_skips_renewal_entirely (AC-004)
    ///
    /// BC-4.17.001 PC2 row 2: no factory_lock block → hook MUST NOT create one.
    ///
    /// RED GATE: guard_logic is todo!() → panics.
    #[test]
    fn test_no_lock_block_skips_renewal_entirely() {
        let content = state_no_lock_old_ts();
        let written = Arc::new(Mutex::new(None::<Vec<u8>>));
        let w = written.clone();

        let _result = guard_logic(
            payload_for_post_tool_use("Edit"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content.into_bytes()),
                write_file: move |_path, bytes| {
                    *w.lock().unwrap() = Some(bytes.to_vec());
                    Ok(())
                },
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: |_msg| {},
                emit_event: |_event_type, _fields| {},
            },
        );

        let written_bytes = written
            .lock()
            .unwrap()
            .clone()
            .expect("write_file must be called");
        let written_str = String::from_utf8_lossy(&written_bytes);
        assert!(
            !written_str.contains("factory_lock:"),
            "AC-004 PC2: hook must NOT create a factory_lock block when none was present. \
             Got:\n{written_str:?}"
        );
    }

    // -----------------------------------------------------------------------
    // AC-005: PC2 identity-gate row 3 — holder present but empty → no renewal
    // -----------------------------------------------------------------------

    /// test_empty_holder_skips_renewal (AC-005)
    ///
    /// BC-4.17.001 PC2 row 3: empty holder → no renewal; timestamp still re-stamped.
    /// parse_factory_lock returns Err(MalformedLockBlock) for empty holder;
    /// hook must treat this as "no renewal" (safe direction) while PC1 proceeds.
    ///
    /// RED GATE: guard_logic is todo!() → panics.
    #[test]
    fn test_empty_holder_skips_renewal() {
        let content = state_with_empty_holder();
        let written = Arc::new(Mutex::new(None::<Vec<u8>>));
        let w = written.clone();

        let _result = guard_logic(
            payload_for_post_tool_use("Write"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content.into_bytes()),
                write_file: move |_path, bytes| {
                    *w.lock().unwrap() = Some(bytes.to_vec());
                    Ok(())
                },
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: |_msg| {},
                emit_event: |_event_type, _fields| {},
            },
        );

        let written_bytes = written.lock().unwrap().clone().expect(
            "write_file must be called — timestamp re-stamp proceeds even with empty holder",
        );
        let written_str = String::from_utf8_lossy(&written_bytes);
        // expires_at must remain unchanged (empty holder → no renewal)
        assert!(
            written_str.contains("2099-01-01T00:00:00Z"),
            "AC-005 PC2: expires_at must remain byte-identical when holder is empty. \
             Got:\n{written_str:?}"
        );
        // PC1 still proceeds: timestamp re-stamped
        assert!(
            written_str.contains("timestamp: 2026-08-27T12:00:00Z"),
            "AC-005: timestamp must still be re-stamped even when holder is empty (PC1 has no identity gate). \
             Got:\n{written_str:?}"
        );
    }

    // -----------------------------------------------------------------------
    // AC-006: PC2 + Invariant 2 — identity mismatch, SAFETY-CRITICAL
    // -----------------------------------------------------------------------

    /// test_identity_mismatch_never_renews_expires_at (AC-006)
    ///
    /// BC-4.17.001 PC2 + Invariant 2 (SAFETY-CRITICAL):
    /// identity mismatch → expires_at NEVER renewed; foreign lock MUST NOT be resurrected.
    ///
    /// RED GATE: guard_logic is todo!() → panics.
    #[test]
    fn test_identity_mismatch_never_renews_expires_at() {
        let original_expires = "2099-06-01T00:00:00Z";
        let content = state_with_foreign_lock(original_expires);
        let written = Arc::new(Mutex::new(None::<Vec<u8>>));
        let w = written.clone();

        let _result = guard_logic(
            payload_for_post_tool_use("Edit"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content.into_bytes()),
                write_file: move |_path, bytes| {
                    *w.lock().unwrap() = Some(bytes.to_vec());
                    Ok(())
                },
                // "caller@example.com" != "holder@example.com" → MISMATCH
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: |_msg| {},
                emit_event: |_event_type, _fields| {},
            },
        );

        let written_bytes = written
            .lock()
            .unwrap()
            .clone()
            .expect("write_file must be called");
        let written_str = String::from_utf8_lossy(&written_bytes);
        assert!(
            written_str.contains(original_expires),
            "AC-006 (SAFETY-CRITICAL) Invariant 2: expires_at must be byte-identical — \
             foreign holder must NEVER be renewed. Got:\n{written_str:?}"
        );
        let renewed = (fixed_now() + Duration::seconds(2700))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        assert!(
            !written_str.contains(&format!("expires_at: {renewed}")),
            "AC-006 (SAFETY-CRITICAL): expires_at must NOT be advanced to now+2700s on identity mismatch. \
             Got:\n{written_str:?}"
        );
    }

    /// test_lock_expired_admitted_non_holder_writer_never_renews (AC-006)
    ///
    /// BC-4.17.001 PC2 / Invariant 2 — BC-4.13.001-PC2-admission scenario (SAFETY-CRITICAL):
    /// a non-holder writer was admitted through an expired lock window (LockExpired path).
    /// The hook MUST NOT resurrect the dead lock's expires_at.
    ///
    /// RED GATE: guard_logic is todo!() → panics.
    #[test]
    fn test_lock_expired_admitted_non_holder_writer_never_renews() {
        // Expired lock: expires_at in the past → BC-4.13.001 PC2 LockExpired admission
        let expired_expires = "2020-01-01T00:45:00Z";
        let content = format!(
            concat!(
                "---\n",
                "document_type: state\n",
                "version: \"0.0.1-test\"\n",
                "timestamp: 2020-01-01T00:00:00Z\n",
                "phase: test\n",
                "factory_lock:\n",
                "  holder: \"holder@example.com\"\n",
                "  locked_at: \"2020-01-01T00:00:00Z\"\n",
                "  expires_at: \"{expires}\"\n",
                "---\n",
                "\n# STATE (expired-lock, non-holder writer admitted via LockExpired)\n",
            ),
            expires = expired_expires,
        );
        let written = Arc::new(Mutex::new(None::<Vec<u8>>));
        let w = written.clone();

        let _result = guard_logic(
            payload_for_post_tool_use("Edit"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content.into_bytes()),
                write_file: move |_path, bytes| {
                    *w.lock().unwrap() = Some(bytes.to_vec());
                    Ok(())
                },
                // Non-holder writer admitted via expired lock (different identity)
                exec_subprocess: |_argv| Ok((0, "newwriter@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: |_msg| {},
                emit_event: |_event_type, _fields| {},
            },
        );

        let written_bytes = written
            .lock()
            .unwrap()
            .clone()
            .expect("write_file must be called");
        let written_str = String::from_utf8_lossy(&written_bytes);
        assert!(
            written_str.contains(expired_expires),
            "AC-006 (BC-4.13.001-PC2-admission, SAFETY-CRITICAL): \
             expired lock's expires_at must remain unchanged — NEVER resurrected. \
             Got:\n{written_str:?}"
        );
    }

    // -----------------------------------------------------------------------
    // PC2 Case-2 safety: expired-BUT-SELF-HELD → NoOp (ADR-046 anti-resurrection)
    // -----------------------------------------------------------------------

    /// test_expired_self_held_lock_never_renewed
    ///
    /// BC-5.40.001 PC4 condition (c) / ADR-046 anti-resurrection (SAFETY-CRITICAL):
    /// an ALREADY-EXPIRED lock whose `holder` byte-equals the writer's resolved identity
    /// MUST NOT be renewed. The stamp hook must leave `expires_at` byte-identical to the
    /// expired value even though identity matches.
    ///
    /// This test is additive — it covers the SELF-held expired case.
    /// `test_lock_expired_admitted_non_holder_writer_never_renews` covers only the
    /// NON-holder expired case (different identity). Both expired cases must be pinned.
    ///
    /// RED GATE: current guard_logic calls renew_lock_with_now purely on identity match
    /// (no expiry gate) → for an expired self-held lock, new_expires_at = now + TTL_SECONDS
    /// != expired_expires → Renewed(new_content) → expires_at changes → this assertion FAILS.
    /// After implementer rework (add expiry gate in guard_logic for Case-2), this test PASSES.
    #[test]
    fn test_expired_self_held_lock_never_renewed() {
        // Expired lock: holder == caller identity, expires_at well in the past.
        // fixed_now() = 2026-08-27T12:00:00Z; expired_expires (2020) < now → expired.
        let expired_expires = "2020-01-01T00:45:00Z";
        let content = format!(
            concat!(
                "---\n",
                "document_type: state\n",
                "version: \"0.0.1-test\"\n",
                "timestamp: 2020-01-01T00:00:00Z\n",
                "phase: test\n",
                "factory_lock:\n",
                "  holder: \"caller@example.com\"\n",
                "  locked_at: \"2020-01-01T00:00:00Z\"\n",
                "  expires_at: \"{expires}\"\n",
                "---\n",
                "\n# STATE (expired self-held lock)\n",
            ),
            expires = expired_expires,
        );
        let written = Arc::new(Mutex::new(None::<Vec<u8>>));
        let w = written.clone();
        // F-P9-003: track whether exec_subprocess is invoked.
        // Case-2 (already expired) must short-circuit before identity resolution,
        // so exec_subprocess must NOT be called even though identity would match.
        let exec_called = Arc::new(Mutex::new(false));

        let _result = guard_logic(
            payload_for_post_tool_use("Write"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content.into_bytes()),
                write_file: move |_path, bytes| {
                    *w.lock().unwrap() = Some(bytes.to_vec());
                    Ok(())
                },
                // Identity MATCHES the lock holder — but the lock is already expired.
                // exec_called is set to true if the closure is invoked; for Case-2 it must
                // remain false (expiry gate fires before identity resolution per ADR-046).
                exec_subprocess: {
                    let ec = exec_called.clone();
                    move |_argv| {
                        *ec.lock().unwrap() = true;
                        Ok((0, "caller@example.com\n".to_string()))
                    }
                },
                now_fn: fixed_now,
                log_warn: |_msg| {},
                emit_event: |_event_type, _fields| {},
            },
        );

        // F-P9-003 Red Gate contract: exec_subprocess must NOT be called for an
        // already-expired lock (PC2 Case-2 resolves before identity resolution).
        assert!(
            !*exec_called.lock().unwrap(),
            "PC2 Case-2 (F-P9-003, ADR-046 anti-resurrection): exec_subprocess must NOT be \
             called — the expiry gate must short-circuit before identity resolution. \
             Got: exec_subprocess was invoked."
        );

        let written_bytes = written
            .lock()
            .unwrap()
            .clone()
            .expect("write_file must be called — PC1 always stamps timestamp");
        let written_str = String::from_utf8_lossy(&written_bytes);

        // PC1: timestamp MUST be re-stamped (unconditional — no identity or expiry gate).
        assert!(
            written_str.contains("timestamp: 2026-08-27T12:00:00Z"),
            "PC1: timestamp must be re-stamped even for expired self-held lock. \
             Got:\n{written_str:?}"
        );

        // PC2 Case-2 / ADR-046 anti-resurrection (SAFETY-CRITICAL):
        // expires_at MUST remain byte-identical to the expired value.
        assert!(
            written_str.contains(expired_expires),
            "PC2 Case-2 (BC-5.40.001 PC4 condition (c), ADR-046 anti-resurrection, \
             SAFETY-CRITICAL): expired self-held lock's expires_at must remain \
             byte-identical — NEVER resurrected. Got:\n{written_str:?}"
        );
        // Belt-and-suspenders: confirm the renewed value is NOT present.
        let renewed = (fixed_now() + Duration::seconds(i64::from(flp::TTL_SECONDS)))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        assert!(
            !written_str.contains(&format!("expires_at: {renewed}")),
            "PC2 Case-2 (ADR-046): expires_at must NOT be advanced to now+TTL_SECONDS \
             for an expired self-held lock. Got:\n{written_str:?}"
        );
    }

    // -----------------------------------------------------------------------
    // AC-007: PC2 + PC3 — identity-resolution failure → skip renewal, still stamp
    // -----------------------------------------------------------------------

    /// test_identity_resolution_failure_skips_renewal_but_timestamp_still_restamped (AC-007)
    ///
    /// BC-4.17.001 PC2 + PC3: exec_subprocess Err → expires_at unchanged;
    /// timestamp: STILL re-stamped (PC1 has no identity gate).
    ///
    /// RED GATE: guard_logic is todo!() → panics.
    #[test]
    fn test_identity_resolution_failure_skips_renewal_but_timestamp_still_restamped() {
        let original_expires = "2099-01-01T00:00:00Z";
        let content = format!(
            concat!(
                "---\n",
                "document_type: state\n",
                "version: \"0.0.1-test\"\n",
                "timestamp: 2020-01-01T00:00:00Z\n",
                "phase: test\n",
                "factory_lock:\n",
                "  holder: \"holder@example.com\"\n",
                "  locked_at: \"2026-01-01T10:00:00Z\"\n",
                "  expires_at: \"{expires}\"\n",
                "---\n",
                "\n# STATE\n",
            ),
            expires = original_expires,
        );
        let written = Arc::new(Mutex::new(None::<Vec<u8>>));
        let w = written.clone();

        let _result = guard_logic(
            payload_for_post_tool_use("Write"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content.into_bytes()),
                write_file: move |_path, bytes| {
                    *w.lock().unwrap() = Some(bytes.to_vec());
                    Ok(())
                },
                // Identity resolution FAILURE
                exec_subprocess: |_argv| Err("git config user.email failed".to_string()),
                now_fn: fixed_now,
                log_warn: |_msg| {},
                emit_event: |_event_type, _fields| {},
            },
        );

        let written_bytes = written.lock().unwrap().clone().expect(
            "write_file must be called — timestamp re-stamp proceeds even on identity-resolution failure",
        );
        let written_str = String::from_utf8_lossy(&written_bytes);
        // PC2 gate failed → expires_at unchanged
        assert!(
            written_str.contains(original_expires),
            "AC-007 PC2+PC3: expires_at must be unchanged when identity resolution fails. \
             Got:\n{written_str:?}"
        );
        // PC1 unaffected → timestamp still re-stamped
        assert!(
            written_str.contains("timestamp: 2026-08-27T12:00:00Z"),
            "AC-007: timestamp must still be re-stamped even on identity-resolution failure \
             (PC1 has no identity gate). Got:\n{written_str:?}"
        );
    }

    // -----------------------------------------------------------------------
    // AC-008: PC3 fail-open — various structural error conditions → zero writes
    // -----------------------------------------------------------------------

    /// test_malformed_frontmatter_writes_nothing (AC-008)
    ///
    /// BC-4.17.001 PC3: malformed frontmatter (no closing --- delimiter) →
    /// hook writes NOTHING (fail-open; agent's write is not touched).
    ///
    /// RED GATE: guard_logic is todo!() → panics before any write_file invocation.
    #[test]
    fn test_malformed_frontmatter_writes_nothing() {
        let content = state_malformed_no_closing_delimiter();
        let write_called = Arc::new(Mutex::new(false));
        let wc = write_called.clone();
        let warns: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let warns_clone = warns.clone();

        let _result = guard_logic(
            payload_for_post_tool_use("Edit"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content.into_bytes()),
                write_file: move |_path, _bytes| {
                    *wc.lock().unwrap() = true;
                    Ok(())
                },
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: move |msg| {
                    warns_clone.lock().unwrap().push(msg.to_string());
                },
                emit_event: |_event_type, _fields| {},
            },
        );

        assert!(
            !*write_called.lock().unwrap(),
            "AC-008 PC3: write_file must NOT be called when frontmatter is malformed (fail-open)"
        );
        // F-P5-001: StampingSkipped advisory must fire on the fence-not-found path.
        // Mutation-kill: removing the log_warn call from the fence-not-found branch in
        // guard_logic makes `warns` remain empty → `any(contains("StampingSkipped"))` fails.
        let captured = warns.lock().unwrap();
        assert!(
            captured.iter().any(|m| m.contains("StampingSkipped")),
            "F-P5-001: log_warn must fire with 'StampingSkipped' on fence-not-found \
             (missing/invalid frontmatter delimiters) path. Got: {captured:?}"
        );
    }

    /// test_read_file_error_writes_nothing (AC-008)
    ///
    /// BC-4.17.001 PC3: read_file returns Err → hook writes NOTHING.
    ///
    /// RED GATE: guard_logic is todo!() → panics.
    #[test]
    fn test_read_file_error_writes_nothing() {
        let write_called = Arc::new(Mutex::new(false));
        let wc = write_called.clone();
        let warns: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let warns_clone = warns.clone();

        let _result = guard_logic(
            payload_for_post_tool_use("Edit"),
            StampCallbacks {
                read_file: |_path, _max, _timeout| {
                    Err("HostError: read_file capability denied".to_string())
                },
                write_file: move |_path, _bytes| {
                    *wc.lock().unwrap() = true;
                    Ok(())
                },
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: move |msg| {
                    warns_clone.lock().unwrap().push(msg.to_string());
                },
                emit_event: |_event_type, _fields| {},
            },
        );

        assert!(
            !*write_called.lock().unwrap(),
            "AC-008 PC3: write_file must NOT be called when read_file fails (fail-open)"
        );
        // F-P5-001: StampingSkipped advisory must fire on the generic read-error path.
        // Mutation-kill: removing the log_warn call from the read-error branch in
        // guard_logic makes `warns` remain empty → `any(contains("StampingSkipped"))` fails.
        let captured = warns.lock().unwrap();
        assert!(
            captured.iter().any(|m| m.contains("StampingSkipped")),
            "F-P5-001: log_warn must fire with 'StampingSkipped' on generic read-error path. \
             Got: {captured:?}"
        );
    }

    /// test_non_utf8_content_writes_nothing (AC-008)
    ///
    /// BC-4.17.001 PC3: non-UTF-8 bytes in file content → hook writes NOTHING.
    ///
    /// RED GATE: guard_logic is todo!() → panics.
    #[test]
    fn test_non_utf8_content_writes_nothing() {
        // Valid ASCII frontmatter followed by invalid UTF-8 bytes in the body region.
        let invalid_utf8: Vec<u8> =
            b"---\ntimestamp: 2020-01-01T00:00:00Z\nphase: test\n---\n\xFF\xFE".to_vec();
        let write_called = Arc::new(Mutex::new(false));
        let wc = write_called.clone();
        let warns: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let warns_clone = warns.clone();

        let _result = guard_logic(
            payload_for_post_tool_use("Write"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(invalid_utf8),
                write_file: move |_path, _bytes| {
                    *wc.lock().unwrap() = true;
                    Ok(())
                },
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: move |msg| {
                    warns_clone.lock().unwrap().push(msg.to_string());
                },
                emit_event: |_event_type, _fields| {},
            },
        );

        assert!(
            !*write_called.lock().unwrap(),
            "AC-008 PC3: write_file must NOT be called when content is non-UTF-8 (fail-open)"
        );
        // F-P5-001: StampingSkipped advisory must fire on the non-UTF-8 path.
        // Mutation-kill: removing the log_warn call from the from_utf8 Err branch in
        // guard_logic makes `warns` remain empty → `any(contains("StampingSkipped"))` fails.
        let captured = warns.lock().unwrap();
        assert!(
            captured.iter().any(|m| m.contains("StampingSkipped")),
            "F-P5-001: log_warn must fire with 'StampingSkipped' on non-UTF-8 content path. \
             Got: {captured:?}"
        );
    }

    // -----------------------------------------------------------------------
    // EC-013 Invariant-9 row 4: no timestamp + no lock → write nothing
    // Replaces the RETIRED `test_missing_timestamp_anchor_writes_nothing`.
    // That test conflated structural-fail (PC3a) with the EC-013 scoped exception (GAP-1).
    // The row 3 counter-case (no-ts + matching lock → write) is covered by
    // `test_no_timestamp_anchor_with_matching_lock_renews_expires_at` (new, below).
    // -----------------------------------------------------------------------

    /// test_no_timestamp_anchor_no_lock_writes_nothing
    /// (EC-013 / GAP-1 / Invariant-9 row 4)
    ///
    /// BC-4.17.001 GAP-1 EC-013 row 4: valid frontmatter, NO `timestamp:` line,
    /// NO `factory_lock` block → PC1 is a no-op, PC2 is NoOp → write_file NOT called.
    ///
    /// MUTATION-KILLING: if the GAP-1 path writes unconditionally even without a PC2
    /// Renewed outcome, write_file would be called → assertion fails.
    #[test]
    fn test_no_timestamp_anchor_no_lock_writes_nothing() {
        let content = state_no_timestamp_line();
        let write_called = Arc::new(Mutex::new(false));
        let wc = write_called.clone();

        let _result = guard_logic(
            payload_for_post_tool_use("Edit"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content.into_bytes()),
                write_file: move |_path, _bytes| {
                    *wc.lock().unwrap() = true;
                    Ok(())
                },
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: |_msg| {},
                emit_event: |_event_type, _fields| {},
            },
        );

        assert!(
            !*write_called.lock().unwrap(),
            "EC-013 Invariant-9 row 4: write_file must NOT be called when no timestamp: \
             anchor and no lock block — PC1=no-op, PC2=NoOp → write suppressed."
        );
    }

    // -----------------------------------------------------------------------
    // AC-009: PC3 — no accumulated failure state across invocations
    // -----------------------------------------------------------------------

    /// test_failure_then_success_is_independent_per_invocation (AC-009)
    ///
    /// BC-4.17.001 PC3: hook is stateless per-invocation.
    /// After a fail-open invocation (read error), the next well-formed invocation
    /// MUST proceed normally — hook MUST NOT carry poisoned state.
    ///
    /// RED GATE: guard_logic is todo!() → panics on the first call.
    #[test]
    fn test_failure_then_success_is_independent_per_invocation() {
        // First invocation: read_file fails → fail-open, no write
        let _first = guard_logic(
            payload_for_post_tool_use("Edit"),
            StampCallbacks {
                read_file: |_path, _max, _timeout| Err("HostError: transient failure".to_string()),
                write_file: |_path, _bytes| Ok(()),
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: |_msg| {},
                emit_event: |_event_type, _fields| {},
            },
        );

        // Second invocation: valid content → write MUST proceed normally
        let content = state_no_lock_old_ts();
        let write_called = Arc::new(Mutex::new(false));
        let wc = write_called.clone();
        let _second = guard_logic(
            payload_for_post_tool_use("Edit"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content.into_bytes()),
                write_file: move |_path, _bytes| {
                    *wc.lock().unwrap() = true;
                    Ok(())
                },
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: |_msg| {},
                emit_event: |_event_type, _fields| {},
            },
        );
        assert!(
            *write_called.lock().unwrap(),
            "AC-009 PC3: second invocation must write normally — no carry-over from prior failure"
        );
    }

    // -----------------------------------------------------------------------
    // AC-010: PC4 idempotent frontmatter-only rewrite
    // -----------------------------------------------------------------------

    /// test_rewrite_touches_only_two_frontmatter_lines (AC-010)
    ///
    /// BC-4.17.001 PC4: at most 2 lines changed (timestamp: + expires_at:).
    /// Fixture: STATE.md with multiple frontmatter lines, identity match → both lines change.
    /// Expected: exactly 2 lines differ between original and written content.
    ///
    /// RED GATE: guard_logic is todo!() → panics.
    #[test]
    fn test_rewrite_touches_only_two_frontmatter_lines() {
        let content = state_with_self_lock();
        let original_bytes = content.as_bytes().to_vec();
        let written = Arc::new(Mutex::new(None::<Vec<u8>>));
        let w = written.clone();

        let _result = guard_logic(
            payload_for_post_tool_use("MultiEdit"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content.into_bytes()),
                write_file: move |_path, bytes| {
                    *w.lock().unwrap() = Some(bytes.to_vec());
                    Ok(())
                },
                // Identity match → both timestamp + expires_at change
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: |_msg| {},
                emit_event: |_event_type, _fields| {},
            },
        );

        let written_bytes = written
            .lock()
            .unwrap()
            .clone()
            .expect("write_file must be called");
        let orig_str = std::str::from_utf8(&original_bytes).expect("original must be valid UTF-8");
        let written_str = std::str::from_utf8(&written_bytes).expect("written must be valid UTF-8");
        let orig_lines: Vec<&str> = orig_str.lines().collect();
        let written_lines: Vec<&str> = written_str.lines().collect();

        assert_eq!(
            orig_lines.len(),
            written_lines.len(),
            "AC-010 PC4: rewrite must not add or remove lines — only in-place replacement"
        );

        let changed: Vec<usize> = orig_lines
            .iter()
            .zip(written_lines.iter())
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, _)| i)
            .collect();

        assert!(
            changed.len() <= 2,
            "AC-010 PC4: at most 2 lines must change (timestamp: + expires_at:). \
             Got {} changes at line indices {:?}",
            changed.len(),
            changed
        );
        assert!(
            !changed.is_empty(),
            "AC-010 PC4: at least 1 line must change (timestamp: always re-stamped)"
        );
    }

    /// test_body_content_after_closing_delimiter_never_modified (AC-010)
    ///
    /// BC-4.17.001 PC4 + Invariant 5: bytes after the closing `---` delimiter
    /// must be byte-identical (body never read for decision purposes or modified).
    ///
    /// RED GATE: guard_logic is todo!() → panics.
    #[test]
    fn test_body_content_after_closing_delimiter_never_modified() {
        let content = state_with_self_lock();
        // Extract expected body: everything after the closing `---\n`
        let body_start = content
            .find("\n---\n")
            .map(|pos| pos + "\n---\n".len())
            .unwrap_or(content.len());
        let expected_body = content[body_start..].to_string();

        let written = Arc::new(Mutex::new(None::<Vec<u8>>));
        let w = written.clone();

        let _result = guard_logic(
            payload_for_post_tool_use("Edit"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content.into_bytes()),
                write_file: move |_path, bytes| {
                    *w.lock().unwrap() = Some(bytes.to_vec());
                    Ok(())
                },
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: |_msg| {},
                emit_event: |_event_type, _fields| {},
            },
        );

        let written_bytes = written
            .lock()
            .unwrap()
            .clone()
            .expect("write_file must be called");
        let written_str = String::from_utf8_lossy(&written_bytes);
        let written_body_start = written_str
            .find("\n---\n")
            .map(|pos| pos + "\n---\n".len())
            .unwrap_or(written_str.len());
        let written_body = &written_str[written_body_start..];

        assert_eq!(
            written_body, expected_body,
            "AC-010 Invariant 5: body content after closing --- must be byte-identical. \
             Got:\n{written_body:?}"
        );
    }

    // -----------------------------------------------------------------------
    // AC-012: TTL_SECONDS canonical constant — import check
    // -----------------------------------------------------------------------

    /// test_ttl_seconds_constant_equals_2700 (AC-012, stamp-state-timestamp crate)
    ///
    /// BC-4.17.001 Precondition 3 / Invariant 3 / ADR-046 F-006:
    /// factory_lock_parse::TTL_SECONDS must equal 2700 (canonical factory_lock TTL).
    /// This crate imports it; the value must be correct before guard_logic can be used.
    ///
    /// RED GATE: factory_lock_parse::TTL_SECONDS = 0 (stub) → assert_eq fails.
    #[test]
    fn test_ttl_seconds_constant_equals_2700() {
        assert_eq!(
            flp::TTL_SECONDS,
            2700u32,
            "AC-012 BC-4.17.001 Precondition 3 / Invariant 3: factory_lock_parse::TTL_SECONDS \
             must equal 2700. Stub has 0 — S-17.05 T-2 must set to 2700."
        );
    }

    /// test_ttl_seconds_is_imported_not_redeclared (AC-012, source-scan)
    ///
    /// Architecture Compliance Rule 7: stamp-state-timestamp/src/lib.rs MUST NOT contain
    /// a second `2700`-valued literal or const in production code — TTL_SECONDS is imported
    /// from factory-lock-parse, never redeclared here.
    ///
    /// Source-scan: reads stamp-state-timestamp/src/lib.rs, strips the test module and
    /// comment-only lines, then asserts no occurrence of `= 2700` (which would indicate
    /// a redeclaration of the constant or a bare literal assignment).
    ///
    /// NOTE: This test PASSES in the stub phase (no `2700` literal in the stub).
    /// It is a stay-green correctness guard for the implementation phase — fails if
    /// the implementer accidentally redeclares the literal.
    #[test]
    fn test_ttl_seconds_is_imported_not_redeclared() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let lib_path = std::path::Path::new(manifest_dir).join("src/lib.rs");
        let source = std::fs::read_to_string(&lib_path)
            .unwrap_or_else(|e| panic!("failed to read {:?}: {}", lib_path, e));
        // Normalize CRLF → LF so the test-module boundary search is platform-agnostic.
        // Without this, `source.find("#[cfg(test)]\nmod tests")` returns None on
        // Windows CRLF checkouts, leaving the test module included in the scan.
        let source = source.replace("\r\n", "\n");

        // Restrict scan to production code only: strip the test module.
        let non_test_source = if let Some(test_mod_pos) = source.find("#[cfg(test)]\nmod tests") {
            &source[..test_mod_pos]
        } else {
            source.as_str()
        };

        // A genuine redeclaration of TTL_SECONDS looks like:
        //   `const TTL_SECONDS: u32 = 2700;`  (or `static`, or with a visibility modifier)
        // Matching bare `= 2700` alone is too broad: it self-matches this test's own
        // filter-predicate string literal and its error-message string literal on any
        // platform where the test module is not stripped.  Requiring `const TTL_SECONDS`
        // or `static TTL_SECONDS` AND `= 2700` on the same line is declaration-shaped:
        // those two substrings never appear together on any single line of this test code,
        // so no self-match is possible on any platform regardless of line-ending style.
        let violations: Vec<&str> = non_test_source
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .filter(|line| {
                let is_decl =
                    line.contains("const TTL_SECONDS") || line.contains("static TTL_SECONDS");
                let has_val = line.contains("= 2700");
                is_decl && has_val
            })
            .collect();

        assert!(
            violations.is_empty(),
            "AC-012 Architecture Compliance Rule 7: stamp-state-timestamp/src/lib.rs \
             must NOT redeclare TTL_SECONDS as a literal `= 2700`. \
             Import factory_lock_parse::TTL_SECONDS instead. \
             Found in {:?}:\n{:?}",
            lib_path,
            violations
        );
    }

    // -----------------------------------------------------------------------
    // AC-015: BC-4.17.001 PC3b renewal-indeterminate diagnostic
    // -----------------------------------------------------------------------

    /// test_renewal_indeterminate_emits_event_and_log_warn_on_identity_resolution_failure
    /// (AC-015 / BC-4.17.001 PC3b)
    ///
    /// PC3b positive path: Case-4 (SkipReason::IdentityResolutionFailed) MUST:
    ///   (a) call emit_event exactly once with type "factory.lock.renewal_indeterminate"
    ///       and all 5 required fields: plugin, holder, locked_at, expires_at,
    ///       resolution_error — with lock-sourced field values matching the fixture;
    ///   (b) call log_warn at least once (advisory);
    ///   (c) still re-stamp timestamp: (PC1 unconditional);
    ///   (d) leave expires_at byte-identical (NoOp on PC2).
    ///
    /// Fixture: NOT-expired lock (far-future expires_at, holder present) +
    ///          exec_subprocess returns Err → classify_identity_resolution →
    ///          IdentityResolution::Failed → SkipReason::IdentityResolutionFailed.
    ///
    /// MUTATION-KILLING: delete emit_event call → events.len() == 0 → assertion (a) fails.
    #[test]
    fn test_renewal_indeterminate_emits_event_and_log_warn_on_identity_resolution_failure() {
        // Fixture: valid NOT-expired lock; exec fails → Case-4.
        let lock_holder = "holder@example.com";
        let lock_locked_at = "2026-01-01T10:00:00Z";
        let lock_expires_at = "2099-01-01T00:00:00Z"; // far future, NOT expired
        let content = format!(
            concat!(
                "---\n",
                "document_type: state\n",
                "version: \"0.0.1-test\"\n",
                "timestamp: 2020-01-01T00:00:00Z\n",
                "phase: test\n",
                "factory_lock:\n",
                "  holder: \"{holder}\"\n",
                "  locked_at: \"{locked_at}\"\n",
                "  expires_at: \"{expires_at}\"\n",
                "---\n",
                "\n# STATE\n",
                "Body content here.\n",
            ),
            holder = lock_holder,
            locked_at = lock_locked_at,
            expires_at = lock_expires_at,
        );

        let event_calls: Arc<Mutex<Vec<(String, Vec<(String, String)>)>>> =
            Arc::new(Mutex::new(vec![]));
        let ec = event_calls.clone();
        let warn_calls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        let wc = warn_calls.clone();
        let written: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
        let wr = written.clone();

        let _result = guard_logic(
            payload_for_post_tool_use("Edit"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content.into_bytes()),
                write_file: move |_path, bytes| {
                    *wr.lock().unwrap() = Some(bytes.to_vec());
                    Ok(())
                },
                // exec_subprocess FAILS → IdentityResolution::Failed → Case-4.
                exec_subprocess: |_argv| Err("git config user.email: exit code 128".to_string()),
                now_fn: fixed_now,
                log_warn: move |msg: &str| {
                    wc.lock().unwrap().push(msg.to_string());
                },
                emit_event: move |event_type: &str, fields: &[(&str, &str)]| {
                    let owned: Vec<(String, String)> = fields
                        .iter()
                        .map(|&(k, v)| (k.to_string(), v.to_string()))
                        .collect();
                    ec.lock().unwrap().push((event_type.to_string(), owned));
                },
            },
        );

        // (a) emit_event called exactly once with the correct event type and all 5 fields.
        {
            let events = event_calls.lock().unwrap();
            assert_eq!(
                events.len(),
                1,
                "AC-015 PC3b: emit_event must be called exactly once on Case-4. Got {} calls.",
                events.len()
            );
            let (event_type, fields) = &events[0];
            assert_eq!(
                event_type, "factory.lock.renewal_indeterminate",
                "AC-015 PC3b: event type must be \"factory.lock.renewal_indeterminate\". \
                 Got: {event_type:?}"
            );
            // All 5 required field keys must be present.
            let required_keys = [
                "plugin",
                "holder",
                "locked_at",
                "expires_at",
                "resolution_error",
            ];
            for key in &required_keys {
                assert!(
                    fields.iter().any(|(k, _)| k == key),
                    "AC-015 PC3b: emit_event fields must contain key {:?}. Got: {:?}",
                    key,
                    fields
                );
            }
            // Field values for lock-sourced fields must match the fixture exactly.
            let field_map: std::collections::HashMap<&str, &str> = fields
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            assert_eq!(
                field_map.get("plugin").copied(),
                Some("stamp-state-timestamp"),
                "AC-015: 'plugin' field must be \"stamp-state-timestamp\""
            );
            assert_eq!(
                field_map.get("holder").copied(),
                Some(lock_holder),
                "AC-015: 'holder' field must match lock's holder"
            );
            assert_eq!(
                field_map.get("locked_at").copied(),
                Some(lock_locked_at),
                "AC-015: 'locked_at' field must match lock's locked_at"
            );
            assert_eq!(
                field_map.get("expires_at").copied(),
                Some(lock_expires_at),
                "AC-015: 'expires_at' field must match lock's expires_at"
            );
            assert!(
                !field_map.get("resolution_error").unwrap_or(&"").is_empty(),
                "AC-015: 'resolution_error' field must be non-empty"
            );
        } // events guard dropped

        // (b) log_warn called at least once.
        {
            let warns = warn_calls.lock().unwrap();
            assert!(
                !warns.is_empty(),
                "AC-015 PC3b: log_warn must be called at least once on Case-4. Got 0 calls."
            );
        }

        // (c) PC1: timestamp still re-stamped unconditionally.
        // (d) expires_at unchanged (NoOp on PC2 — resolution failed, renewal skipped).
        let written_bytes = written
            .lock()
            .unwrap()
            .clone()
            .expect("write_file must be called — PC1 still stamps on Case-4");
        let written_str = String::from_utf8_lossy(&written_bytes);
        assert!(
            written_str.contains("timestamp: 2026-08-27T12:00:00Z"),
            "AC-015 PC1: timestamp must still be re-stamped on Case-4. Got:\n{written_str:?}"
        );
        assert!(
            written_str.contains(lock_expires_at),
            "AC-015 PC3b: expires_at must remain byte-identical on Case-4 (NoOp). \
             Got:\n{written_str:?}"
        );
    }

    /// test_no_renewal_indeterminate_event_for_not_holder_or_already_expired_or_absent
    /// (AC-015 / BC-4.17.001 PC3b non-goal)
    ///
    /// PC3b NON-GOAL: "factory.lock.renewal_indeterminate" MUST NOT be emitted for
    /// Cases 0, 2, or 3. Three sub-fixtures, one per disallowed case:
    ///   Sub-case 1 — NotHolder (Case-3): valid non-expired lock, exec returns DIFFERENT email.
    ///   Sub-case 2 — AlreadyExpired (Case-2): self-held lock with past expires_at.
    ///   Sub-case 3 — Absent (Case-0): no factory_lock block.
    /// For each: emit_event must not be called; PC1 timestamp still re-stamped.
    ///
    /// MUTATION-KILLING: if the implementation also emits the event for Case-3 or Case-2
    /// or Case-0 (e.g., unconditional emit_event), event_calls becomes non-empty → fails.
    #[test]
    fn test_no_renewal_indeterminate_event_for_not_holder_or_already_expired_or_absent() {
        // Sub-case 1: NotHolder — valid NOT-expired lock, exec returns DIFFERENT email.
        {
            let content = state_with_foreign_lock("2099-01-01T00:00:00Z"); // not expired
            let event_calls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
            let ec = event_calls.clone();
            let written: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
            let wr = written.clone();

            let _result = guard_logic(
                payload_for_post_tool_use("Edit"),
                StampCallbacks {
                    read_file: move |_path, _max, _timeout| Ok(content.into_bytes()),
                    write_file: move |_path, bytes| {
                        *wr.lock().unwrap() = Some(bytes.to_vec());
                        Ok(())
                    },
                    // "caller@example.com" != "holder@example.com" → Case-3 (NotHolder).
                    exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                    now_fn: fixed_now,
                    log_warn: |_msg| {},
                    emit_event: move |event_type: &str, _fields: &[(&str, &str)]| {
                        ec.lock().unwrap().push(event_type.to_string());
                    },
                },
            );

            assert!(
                event_calls.lock().unwrap().is_empty(),
                "AC-015 PC3b: emit_event must NOT be called for Case-3 (NotHolder). \
                 Got: {:?}",
                *event_calls.lock().unwrap()
            );
            let written_bytes = written
                .lock()
                .unwrap()
                .clone()
                .expect("write_file must be called — PC1 stamps for NotHolder");
            let written_str = String::from_utf8_lossy(&written_bytes);
            assert!(
                written_str.contains("timestamp: 2026-08-27T12:00:00Z"),
                "NotHolder sub-case: PC1 must still stamp timestamp. Got:\n{written_str:?}"
            );
        }

        // Sub-case 2: AlreadyExpired — self-held lock with PAST expires_at.
        // fixed_now() = 2026-08-27T12:00:00Z > 2020-01-01T00:45:00Z → AlreadyExpired.
        {
            let expired_content = concat!(
                "---\n",
                "document_type: state\n",
                "version: \"0.0.1-test\"\n",
                "timestamp: 2020-01-01T00:00:00Z\n",
                "phase: test\n",
                "factory_lock:\n",
                "  holder: \"caller@example.com\"\n",
                "  locked_at: \"2020-01-01T00:00:00Z\"\n",
                "  expires_at: \"2020-01-01T00:45:00Z\"\n", // past → AlreadyExpired
                "---\n",
                "\n# STATE\n",
            )
            .to_string();
            let event_calls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
            let ec = event_calls.clone();
            let written: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
            let wr = written.clone();

            let _result = guard_logic(
                payload_for_post_tool_use("Write"),
                StampCallbacks {
                    read_file: move |_path, _max, _timeout| Ok(expired_content.into_bytes()),
                    write_file: move |_path, bytes| {
                        *wr.lock().unwrap() = Some(bytes.to_vec());
                        Ok(())
                    },
                    // Identity matches but resolve_identity is NOT called for AlreadyExpired
                    // (Case-2 short-circuits before calling it — AC-002 lazy-call invariant).
                    exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                    now_fn: fixed_now,
                    log_warn: |_msg| {},
                    emit_event: move |event_type: &str, _fields: &[(&str, &str)]| {
                        ec.lock().unwrap().push(event_type.to_string());
                    },
                },
            );

            assert!(
                event_calls.lock().unwrap().is_empty(),
                "AC-015 PC3b: emit_event must NOT be called for Case-2 (AlreadyExpired). \
                 Got: {:?}",
                *event_calls.lock().unwrap()
            );
            let written_bytes = written
                .lock()
                .unwrap()
                .clone()
                .expect("write_file must be called — PC1 stamps for AlreadyExpired");
            let written_str = String::from_utf8_lossy(&written_bytes);
            assert!(
                written_str.contains("timestamp: 2026-08-27T12:00:00Z"),
                "AlreadyExpired sub-case: PC1 must still stamp timestamp. Got:\n{written_str:?}"
            );
        }

        // Sub-case 3: Absent — no factory_lock block → Case-0.
        {
            let content = state_no_lock_old_ts();
            let event_calls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
            let ec = event_calls.clone();
            let written: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
            let wr = written.clone();

            let _result = guard_logic(
                payload_for_post_tool_use("Edit"),
                StampCallbacks {
                    read_file: move |_path, _max, _timeout| Ok(content.into_bytes()),
                    write_file: move |_path, bytes| {
                        *wr.lock().unwrap() = Some(bytes.to_vec());
                        Ok(())
                    },
                    exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                    now_fn: fixed_now,
                    log_warn: |_msg| {},
                    emit_event: move |event_type: &str, _fields: &[(&str, &str)]| {
                        ec.lock().unwrap().push(event_type.to_string());
                    },
                },
            );

            assert!(
                event_calls.lock().unwrap().is_empty(),
                "AC-015 PC3b: emit_event must NOT be called for Case-0 (absent lock). \
                 Got: {:?}",
                *event_calls.lock().unwrap()
            );
            let written_bytes = written
                .lock()
                .unwrap()
                .clone()
                .expect("write_file must be called — PC1 stamps for absent lock");
            let written_str = String::from_utf8_lossy(&written_bytes);
            assert!(
                written_str.contains("timestamp: 2026-08-27T12:00:00Z"),
                "Absent sub-case: PC1 must still stamp timestamp. Got:\n{written_str:?}"
            );
        }
    }

    /// test_malformed_lock_block_case1_emits_advisory_log_warn_not_event
    /// (AC-015 / BC-4.17.001 PC2 case-1 / PC3b)
    ///
    /// Case-1 (malformed block → Err(LockError::Malformed)):
    ///   - log_warn MUST be called (advisory; PC3b case-1 arm);
    ///   - emit_event must NOT be called (renewal_indeterminate is Case-4 only).
    ///
    /// Fixture: factory_lock block with empty holder → parse_factory_lock returns
    ///          Err(MalformedLockBlock) → guard_logic Case-1 → log_warn only.
    ///
    /// MUTATION-KILLING:
    ///   Drop case-1 log_warn call → warn_calls remains empty → first assert fails.
    ///   Add emit_event to case-1 arm → event_calls becomes non-empty → second assert fails.
    #[test]
    fn test_malformed_lock_block_case1_emits_advisory_log_warn_not_event() {
        // Empty holder triggers LockParseError::MalformedLockBlock → Case-1 in guard_logic.
        let content = state_with_empty_holder();

        let warn_calls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        let wc = warn_calls.clone();
        let event_calls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        let ec = event_calls.clone();

        let _result = guard_logic(
            payload_for_post_tool_use("Edit"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content.into_bytes()),
                write_file: |_path, _bytes| Ok(()),
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: move |msg: &str| {
                    wc.lock().unwrap().push(msg.to_string());
                },
                emit_event: move |event_type: &str, _fields: &[(&str, &str)]| {
                    ec.lock().unwrap().push(event_type.to_string());
                },
            },
        );

        // log_warn MUST be called (advisory) for the malformed block.
        assert!(
            !warn_calls.lock().unwrap().is_empty(),
            "AC-015 PC3b Case-1: log_warn must be called at least once for malformed block. \
             Got 0 calls."
        );

        // emit_event must NOT be called — renewal_indeterminate is Case-4 only.
        assert!(
            event_calls.lock().unwrap().is_empty(),
            "AC-015 PC3b Case-1: emit_event must NOT be called for malformed block. \
             renewal_indeterminate is Case-4 only. Got: {:?}",
            *event_calls.lock().unwrap()
        );
    }

    // -----------------------------------------------------------------------
    // AC-016 / EC-013 / Invariant-9 row 3: no timestamp + matching lock → renews
    // -----------------------------------------------------------------------

    /// test_no_timestamp_anchor_with_matching_lock_renews_expires_at
    /// (AC-016 / EC-013 / Invariant-9 row 3)
    ///
    /// BC-4.17.001 GAP-1 EC-013 row 3: valid frontmatter, NO `timestamp:` line, plus a
    /// factory_lock block whose holder == mock identity (far-future expires_at, not expired).
    ///
    /// Expected:
    ///   - write_file IS called (PC2 Renewed → write even though PC1 is no-op).
    ///   - Written content has renewed expires_at = fixed_now() + TTL_SECONDS.
    ///   - NO `timestamp:` line is inserted (PC1 never invents a key).
    ///
    /// MUTATION-KILLING: if the pre-fix early-return-on-no-timestamp is still in place
    /// (old code returned Continue before reaching PC2), write_file is NOT called →
    /// the expect() panics → test fails.
    #[test]
    fn test_no_timestamp_anchor_with_matching_lock_renews_expires_at() {
        let content = state_no_timestamp_self_lock_far_future();
        let written = Arc::new(Mutex::new(None::<Vec<u8>>));
        let w = written.clone();
        let expected_expires = (fixed_now() + Duration::seconds(i64::from(flp::TTL_SECONDS)))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();

        let _result = guard_logic(
            payload_for_post_tool_use("Edit"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content.into_bytes()),
                write_file: move |_path, bytes| {
                    *w.lock().unwrap() = Some(bytes.to_vec());
                    Ok(())
                },
                // Identity MATCH: caller@example.com == holder caller@example.com
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: |_msg| {},
                emit_event: |_event_type, _fields| {},
            },
        );

        let written_bytes = written.lock().unwrap().clone().expect(
            "AC-016 EC-013 row 3: write_file MUST be called when no-timestamp + \
                 identity-match lock → PC2 Renewed requires write even though PC1 is no-op",
        );
        let written_str = String::from_utf8_lossy(&written_bytes);

        // PC2: expires_at renewed to now + TTL_SECONDS.
        assert!(
            written_str.contains(&format!("expires_at: {expected_expires}")),
            "AC-016 EC-013 row 3: expires_at must be renewed to now + TTL_SECONDS \
             ({expected_expires}). Got:\n{written_str:?}"
        );
        // PC1 no-op: NO timestamp: line must be inserted.
        assert!(
            !written_str.contains("timestamp:"),
            "AC-016 EC-013 row 3: PC1 no-op must NOT insert a timestamp: line. \
             Got:\n{written_str:?}"
        );
    }

    // -----------------------------------------------------------------------
    // AC-017 / Precondition 1 / F-013: tool_response error gate
    // -----------------------------------------------------------------------

    /// test_failed_tool_write_skips_both_arms
    /// (AC-017 / Precondition 1 / F-013)
    ///
    /// BC-4.17.001 Precondition 1 (GAP-3 / F-013 tool-write-success gate):
    /// sub-case A — tool_response with non-null "error" key → read_file NOT called.
    /// sub-case B — tool_response absent (None) → read_file NOT called.
    ///
    /// MUTATION-KILLING: remove the GAP-3 gate entirely → read_file IS called for
    /// both sub-cases → Arc<Mutex<bool>> flips true → assertions fail.
    #[test]
    fn test_failed_tool_write_skips_both_arms() {
        // Sub-case A: non-null "error" key in tool_response.
        {
            let payload_with_error: HookPayload = serde_json::from_value(json!({
                "event_name": "PostToolUse",
                "tool_name": "Write",
                "session_id": "test-session",
                "dispatcher_trace_id": "test-trace",
                "tool_input": { "file_path": ".factory/STATE.md" },
                "tool_response": { "error": "Disk full — write not committed" }
            }))
            .expect("fixture must deserialize");

            let read_called = Arc::new(Mutex::new(false));
            let rc = read_called.clone();

            let _result = guard_logic(
                payload_with_error,
                StampCallbacks {
                    read_file: move |_path, _max, _timeout| {
                        *rc.lock().unwrap() = true;
                        Ok(vec![])
                    },
                    write_file: |_path, _bytes| Ok(()),
                    exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                    now_fn: fixed_now,
                    log_warn: |_msg| {},
                    emit_event: |_event_type, _fields| {},
                },
            );

            assert!(
                !*read_called.lock().unwrap(),
                "AC-017 Precondition 1 sub-case A: read_file must NOT be called when \
                 tool_response contains non-null 'error' key (F-013 gate)"
            );
        }

        // Sub-case B: tool_response absent → None in HookPayload.
        {
            let payload_no_response: HookPayload = serde_json::from_value(json!({
                "event_name": "PostToolUse",
                "tool_name": "Write",
                "session_id": "test-session",
                "dispatcher_trace_id": "test-trace",
                "tool_input": { "file_path": ".factory/STATE.md" }
            }))
            .expect("fixture must deserialize");

            let read_called = Arc::new(Mutex::new(false));
            let rc = read_called.clone();

            let _result = guard_logic(
                payload_no_response,
                StampCallbacks {
                    read_file: move |_path, _max, _timeout| {
                        *rc.lock().unwrap() = true;
                        Ok(vec![])
                    },
                    write_file: |_path, _bytes| Ok(()),
                    exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                    now_fn: fixed_now,
                    log_warn: |_msg| {},
                    emit_event: |_event_type, _fields| {},
                },
            );

            assert!(
                !*read_called.lock().unwrap(),
                "AC-017 Precondition 1 sub-case B: read_file must NOT be called when \
                 tool_response is absent/None (F-013 gate)"
            );
        }
    }

    // -----------------------------------------------------------------------
    // AC-018 / Invariant 8 / GAP-4: soft-warn when approaching read cap
    // -----------------------------------------------------------------------

    /// test_approaching_cap_emits_soft_warn_event (AC-018 / Invariant 8)
    ///
    /// BC-4.17.001 GAP-4 Invariant 8: content length 200001 bytes (in the range
    /// (200000, 262144]) must trigger emit_event("state_md_approaching_cap", …) with
    /// fields bytes_read="200001" and cap_bytes="262144". PC1 must still proceed.
    ///
    /// MUTATION-KILLING: remove the emit_event call or raise the lower bound above 200001 →
    /// event_calls empty → soft_warn.is_some() assertion fails.
    #[test]
    fn test_approaching_cap_emits_soft_warn_event() {
        let content = state_padded_to(200_001);
        let event_calls: Arc<Mutex<Vec<(String, Vec<(String, String)>)>>> =
            Arc::new(Mutex::new(vec![]));
        let ec = event_calls.clone();
        let written = Arc::new(Mutex::new(None::<Vec<u8>>));
        let wr = written.clone();

        let _result = guard_logic(
            payload_for_post_tool_use("Write"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content),
                write_file: move |_path, bytes| {
                    *wr.lock().unwrap() = Some(bytes.to_vec());
                    Ok(())
                },
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: |_msg| {},
                emit_event: move |event_type: &str, fields: &[(&str, &str)]| {
                    let owned: Vec<(String, String)> = fields
                        .iter()
                        .map(|&(k, v)| (k.to_string(), v.to_string()))
                        .collect();
                    ec.lock().unwrap().push((event_type.to_string(), owned));
                },
            },
        );

        let events = event_calls.lock().unwrap();
        let soft_warn = events
            .iter()
            .find(|(et, _)| et == "state_md_approaching_cap");
        assert!(
            soft_warn.is_some(),
            "AC-018 Invariant 8: emit_event('state_md_approaching_cap') must be called \
             when content is 200001 bytes (> 200000 && <= 262144). Got events: {:?}",
            events.iter().map(|(et, _)| et).collect::<Vec<_>>()
        );
        let (_, fields) = soft_warn.unwrap();
        let field_map: std::collections::HashMap<&str, &str> = fields
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        assert_eq!(
            field_map.get("bytes_read").copied(),
            Some("200001"),
            "AC-018: bytes_read field must be \"200001\""
        );
        assert_eq!(
            field_map.get("cap_bytes").copied(),
            Some("262144"),
            "AC-018: cap_bytes field must be \"262144\""
        );
        drop(events);

        // PC1 must still proceed (soft-warn is observability only, not a suppression gate).
        assert!(
            written.lock().unwrap().is_some(),
            "AC-018: write_file must still be called after soft-warn (PC1 not suppressed)"
        );
    }

    /// test_at_cap_boundary_emits_soft_warn (AC-018)
    ///
    /// BC-4.17.001 Invariant 8: the upper bound is INCLUSIVE — content exactly 262144 bytes
    /// must still trigger the soft-warn event.
    ///
    /// MUTATION-KILLING: change `<= 262_144` to `< 262_144` in the guard →
    /// 262144 bytes no longer triggers → event_calls empty → assertion fails.
    #[test]
    fn test_at_cap_boundary_emits_soft_warn() {
        let content = state_padded_to(262_144);
        let event_calls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        let ec = event_calls.clone();

        let _result = guard_logic(
            payload_for_post_tool_use("Write"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content),
                write_file: |_path, _bytes| Ok(()),
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: |_msg| {},
                emit_event: move |event_type: &str, _fields: &[(&str, &str)]| {
                    ec.lock().unwrap().push(event_type.to_string());
                },
            },
        );

        assert!(
            event_calls
                .lock()
                .unwrap()
                .iter()
                .any(|et| et == "state_md_approaching_cap"),
            "AC-018: soft-warn must fire at exactly 262144 bytes (inclusive upper bound). \
             Got events: {:?}",
            *event_calls.lock().unwrap()
        );
    }

    /// test_below_threshold_no_soft_warn (AC-018 negative)
    ///
    /// BC-4.17.001 Invariant 8: the lower bound is EXCLUSIVE — content exactly 200000 bytes
    /// must NOT trigger the soft-warn event.
    ///
    /// MUTATION-KILLING: change `> 200_000` to `>= 200_000` in the guard →
    /// 200000 bytes now triggers → event_calls non-empty → assertion fails.
    #[test]
    fn test_below_threshold_no_soft_warn() {
        let content = state_padded_to(200_000);
        let event_calls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        let ec = event_calls.clone();

        let _result = guard_logic(
            payload_for_post_tool_use("Write"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content),
                write_file: |_path, _bytes| Ok(()),
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: |_msg| {},
                emit_event: move |event_type: &str, _fields: &[(&str, &str)]| {
                    ec.lock().unwrap().push(event_type.to_string());
                },
            },
        );

        assert!(
            !event_calls
                .lock()
                .unwrap()
                .iter()
                .any(|et| et == "state_md_approaching_cap"),
            "AC-018 negative: soft-warn must NOT fire at exactly 200000 bytes \
             (lower bound is exclusive: > 200000). Got events: {:?}",
            *event_calls.lock().unwrap()
        );
    }

    // -----------------------------------------------------------------------
    // AC-019 / EC-014 / GAP-6: duplicate timestamp: advisory
    // -----------------------------------------------------------------------

    /// test_duplicate_timestamp_rewrites_first_emits_advisory
    /// (AC-019 / EC-014 / GAP-6)
    ///
    /// BC-4.17.001 GAP-6 EC-014: frontmatter with two `timestamp:` lines →
    ///   - log_warn called with message containing "DuplicateTimestampKey".
    ///   - First timestamp: line is rewritten to fixed_now.
    ///   - Second timestamp: line is left byte-identical (untouched).
    ///
    /// MUTATION-KILLING:
    ///   Remove the log_warn("DuplicateTimestampKey…") call → warns empty → first assert fails.
    ///   Rewrite both timestamps → second original timestamp absent → third assert fails.
    #[test]
    fn test_duplicate_timestamp_rewrites_first_emits_advisory() {
        let content = state_duplicate_timestamp();
        let written = Arc::new(Mutex::new(None::<Vec<u8>>));
        let w = written.clone();
        let warn_calls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        let wc = warn_calls.clone();

        let _result = guard_logic(
            payload_for_post_tool_use("Edit"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content.into_bytes()),
                write_file: move |_path, bytes| {
                    *w.lock().unwrap() = Some(bytes.to_vec());
                    Ok(())
                },
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: move |msg: &str| {
                    wc.lock().unwrap().push(msg.to_string());
                },
                emit_event: |_event_type, _fields| {},
            },
        );

        // log_warn must contain "DuplicateTimestampKey".
        {
            let warns = warn_calls.lock().unwrap();
            assert!(
                warns.iter().any(|w| w.contains("DuplicateTimestampKey")),
                "AC-019 EC-014: log_warn must be called with a message containing \
                 'DuplicateTimestampKey'. Got warns: {:?}",
                &*warns
            );
        }

        let written_bytes = written
            .lock()
            .unwrap()
            .clone()
            .expect("write_file must be called — PC1 restamps the first timestamp");
        let written_str = String::from_utf8_lossy(&written_bytes);

        // First timestamp: replaced with fixed_now.
        assert!(
            written_str.contains("timestamp: 2026-08-27T12:00:00Z"),
            "AC-019: first timestamp: must be replaced with fixed_now. \
             Got:\n{written_str:?}"
        );
        // Original first timestamp must be gone.
        assert!(
            !written_str.contains("timestamp: 2020-01-01T00:00:00Z"),
            "AC-019: original first timestamp must be replaced. \
             Got:\n{written_str:?}"
        );
        // Second timestamp: must remain byte-identical.
        assert!(
            written_str.contains("timestamp: 2021-06-15T00:00:00Z"),
            "AC-019 EC-014: second timestamp: line must remain byte-identical \
             (only first is rewritten). Got:\n{written_str:?}"
        );
    }

    // -----------------------------------------------------------------------
    // AC-008 / EC-015 / GAP-7: OutputTooLarge advisory
    // -----------------------------------------------------------------------

    /// test_output_too_large_writes_nothing_with_advisory
    /// (AC-008 / EC-015 / GAP-7)
    ///
    /// BC-4.17.001 GAP-7 EC-015: when read_file returns Err containing "OutputTooLarge",
    /// log_warn must be called with an advisory message and write_file must NOT be called
    /// (PC3 fail-open). This is a structured advisory — not a silent skip.
    ///
    /// MUTATION-KILLING:
    ///   Remove the `if e.contains("OutputTooLarge") { log_warn(…) }` block →
    ///   warns empty → first assert fails.
    ///   Remove the fail-open return after log_warn → write_file called → second assert fails.
    #[test]
    fn test_output_too_large_writes_nothing_with_advisory() {
        let warn_calls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        let wc = warn_calls.clone();
        let write_called = Arc::new(Mutex::new(false));
        let wf = write_called.clone();

        let _result = guard_logic(
            payload_for_post_tool_use("Edit"),
            StampCallbacks {
                read_file: |_path, _max, _timeout| {
                    Err("OutputTooLarge: read_file cap exceeded (262144 bytes)".to_string())
                },
                write_file: move |_path, _bytes| {
                    *wf.lock().unwrap() = true;
                    Ok(())
                },
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: move |msg: &str| {
                    wc.lock().unwrap().push(msg.to_string());
                },
                emit_event: |_event_type, _fields| {},
            },
        );

        // log_warn must be called (advisory observability).
        assert!(
            !warn_calls.lock().unwrap().is_empty(),
            "AC-008 EC-015 GAP-7: log_warn must be called when read_file returns OutputTooLarge. \
             Got 0 warn calls."
        );
        // write_file must NOT be called (PC3 fail-open).
        assert!(
            !*write_called.lock().unwrap(),
            "AC-008 EC-015 GAP-7: write_file must NOT be called when read_file returns \
             OutputTooLarge (PC3 fail-open)."
        );
    }

    // -----------------------------------------------------------------------
    // AC-010 / GAP-2 / EC-017: CRLF frontmatter delimiters handled correctly
    // -----------------------------------------------------------------------

    /// test_crlf_frontmatter_delimiters_handled_correctly
    /// (AC-010 / GAP-2 / EC-017 / O-1)
    ///
    /// BC-4.17.001 GAP-2 EC-017: STATE.md files with CRLF (`\r\n`) line endings
    /// (Windows autocrlf = true) must have their frontmatter fences detected correctly,
    /// the timestamp line re-stamped to exactly `fixed_now()`, the CRLF line terminator
    /// preserved on the rewritten timestamp line, and the body after the closing fence
    /// preserved byte-for-byte.
    ///
    /// MUTATION-KILL 1 (fence detection): revert extract_frontmatter usage to a hand-rolled
    /// `starts_with("---\n")` check → CRLF content (`---\r\n`) doesn't match → fence not
    /// found → guard returns Continue before write → `written.is_none()` → first assertion fails.
    ///
    /// MUTATION-KILL 2 (PC1 restamp value): replace the `timestamp:` line with a
    /// different or empty string → `contains("timestamp: 2026-08-27T12:00:00Z")` fails.
    ///
    /// MUTATION-KILL 3 (CRLF terminator): drop the `crlf_line` `\r` in ts_line_with_term
    /// (hardcode `""` instead of `"\r"`) → timestamp line ends with bare `\n`, not `\r\n`
    /// → `contains("timestamp: 2026-08-27T12:00:00Z\r\n")` fails.
    ///
    /// MUTATION-KILL 4 (body preservation): truncate or mutate the body suffix in the
    /// write_file output → `ends_with("---\r\n\r\n# STATE\r\n")` fails.
    #[test]
    fn test_crlf_frontmatter_delimiters_handled_correctly() {
        let content = state_crlf_valid();
        let written = Arc::new(Mutex::new(None::<Vec<u8>>));
        let w = written.clone();

        let _result = guard_logic(
            payload_for_post_tool_use("Edit"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content),
                write_file: move |_path, bytes| {
                    *w.lock().unwrap() = Some(bytes.to_vec());
                    Ok(())
                },
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: |_msg| {},
                emit_event: |_event_type, _fields| {},
            },
        );

        let written_bytes = written.lock().unwrap().clone().expect(
            "O-1 / AC-010 GAP-2 EC-017: write_file must be called for CRLF-fenced STATE.md — \
             extract_frontmatter must detect \\r\\n delimiters correctly",
        );
        let written_str = String::from_utf8(written_bytes.clone())
            .expect("O-1: rewritten content must be valid UTF-8");

        // O-1 assertion 1: PC1 actually restamped (value = fixed_now, not the old stale value).
        // Mutation-kill 2: if the timestamp line is not rewritten, this fails.
        assert!(
            written_str.contains("timestamp: 2026-08-27T12:00:00Z"),
            "O-1 PC1: timestamp must be re-stamped to fixed_now() = 2026-08-27T12:00:00Z. \
             Got:\n{written_str:?}"
        );
        assert!(
            !written_str.contains("timestamp: 2020-01-01T00:00:00Z"),
            "O-1 PC1: original stale timestamp must NOT remain in CRLF content. \
             Got:\n{written_str:?}"
        );

        // O-1 assertion 2: CRLF line terminator preserved on the rewritten timestamp line.
        // Pins the O-2 fix (crlf_line detection → ts_line_with_term includes `\r`).
        // Mutation-kill 3: dropping the `\r` from crlf_line branch → bare `\n` terminator
        // → this assertion fails.
        assert!(
            written_str.contains("timestamp: 2026-08-27T12:00:00Z\r\n"),
            "O-1 EC-017: rewritten timestamp line must end with CRLF (\\r\\n), not bare \\n. \
             Got (hex of first 120 bytes): {:02X?}",
            &written_bytes[..written_bytes.len().min(120)]
        );

        // O-1 assertion 3: body after closing fence is byte-preserved.
        // state_crlf_valid body = `\r\n# STATE\r\n` (CRLF throughout).
        // Mutation-kill 4: any truncation or modification of the body suffix fails this.
        assert!(
            written_bytes.ends_with(b"---\r\n\r\n# STATE\r\n"),
            "O-1 EC-017: closing fence + body must be byte-preserved in CRLF output. \
             Got tail (last 30 bytes): {:02X?}",
            &written_bytes[written_bytes.len().saturating_sub(30)..]
        );
    }

    // -----------------------------------------------------------------------
    // F-S1705-P6-001 / AC-016: empty valid frontmatter — no panic, no write
    // -----------------------------------------------------------------------

    /// test_empty_valid_frontmatter_no_panic_no_write (F-S1705-P6-001 / AC-016)
    ///
    /// Degenerate case: input STATE.md = exactly `"---\n---\n"` (valid opening
    /// + closing fence, empty body, no `timestamp:` anchor, no `factory_lock`).
    ///
    /// Expected:
    ///   - `guard_logic` returns `HookResult::Continue` WITHOUT panicking (PC3
    ///     fail-open; the `fm_start = fence_len.min(fm_bytes_len)` clamp prevents
    ///     a start>end slice panic when `fm_bytes_len=3 < fence_len=4`).
    ///   - `write_file` NOT called (PC2 NoOp — no lock; PC1 suppressed — no
    ///     timestamp anchor; `has_timestamp=false` + `Ok((NoOp, None))` →
    ///     `content_to_write = None`; Invariant-9 row 4).
    ///
    /// MUTATION-KILL: reverting Guard 2 (the `fence_len.min(fm_bytes_len)` clamp)
    /// back to the unguarded `content[fence_len..fm_bytes_len]` = `content[4..3]`
    /// causes a start-greater-than-end slice panic that aborts the test thread.
    /// The test MUST reach the `write_file` assertion without panicking — a panic
    /// is detected as a test failure, proving the clamp is load-bearing.
    #[test]
    fn test_empty_valid_frontmatter_no_panic_no_write() {
        // "---\n---\n": valid opening fence, valid closing fence, zero content between.
        // extract_frontmatter returns 3 bytes (fm_bytes_len=3 < fence_len=4):
        // the `\n` at byte-position 3 is shared between the opening fence end and
        // the `\n---\n` closing delimiter pattern, leaving zero fm_body bytes.
        let content = "---\n---\n".to_string();
        let written = Arc::new(Mutex::new(None::<Vec<u8>>));
        let w = written.clone();

        // Must NOT panic — if the `fence_len.min(fm_bytes_len)` clamp is absent,
        // guard_logic panics with a start>end slice index here.
        let result = guard_logic(
            payload_for_post_tool_use("Edit"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content.into_bytes()),
                write_file: move |_path, bytes| {
                    *w.lock().unwrap() = Some(bytes.to_vec());
                    Ok(())
                },
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                // log_warn will fire for GAP-1 TimestampAnchorMissing; silently discard.
                log_warn: |_msg| {},
                emit_event: |_event_type, _fields| {},
            },
        );

        // Invariant 4 / PC5: hook always returns Continue (no block_intent path).
        assert!(
            matches!(result, HookResult::Continue),
            "F-S1705-P6-001 AC-016: guard_logic must return HookResult::Continue on \
             empty-frontmatter input"
        );

        // Invariant-9 row 4: has_timestamp=false + PC2=Ok((NoOp, None)) → no write.
        assert!(
            written.lock().unwrap().is_none(),
            "F-S1705-P6-001 AC-016 (Invariant-9 row 4): write_file must NOT be called — \
             has_timestamp=false and PC2=NoOp(None) → content_to_write=None"
        );
    }

    // -----------------------------------------------------------------------
    // F-S1705-P6-001 / AC-008: opening fence absent — fail-open with advisory
    // -----------------------------------------------------------------------

    /// test_opening_fence_absent_fails_open_with_advisory (F-S1705-P6-001 / AC-008)
    ///
    /// Input: content with NO opening `---\n` fence but with an inline `\n---\n`
    /// pattern that satisfies the closing-delimiter check (e.g., `"garbage
    /// line\n---\nmore\n"`).
    ///
    /// `extract_frontmatter` finds the `\n---\n` and returns a shorter slice
    /// (fm_bytes_len < raw_bytes_len), so the *closing*-fence structural check
    /// passes (fence IS located). The subsequent opening-fence guard (Guard 1,
    /// F-S1705-P6-001) catches that content does not start with `---\n` or
    /// `---\r\n` and emits the distinct `StampingSkipped (opening fence absent)`
    /// advisory, then returns Continue.
    ///
    /// Expected:
    ///   - `write_file` NOT called (PC3a fail-open).
    ///   - `log_warn` called with a message containing BOTH `"StampingSkipped"`
    ///     AND `"opening fence absent"` — distinct from the closing-delimiter
    ///     advisory (`"missing/invalid frontmatter delimiters"` without the
    ///     `"opening fence absent"` suffix).
    ///
    /// MUTATION-KILL: removing Guard 1 (the `starts_with` check + advisory
    /// return) causes execution to continue into the fm_body slice computation.
    /// Without a `timestamp:` line in the mis-parsed fm_body, `log_warn` is
    /// called with a `TimestampAnchorMissing` message — which contains neither
    /// `"StampingSkipped"` nor `"opening fence absent"`. The advisory assertion
    /// therefore fails, detecting the missing guard.
    #[test]
    fn test_opening_fence_absent_fails_open_with_advisory() {
        // Content starts with "garbage line" — no opening "---\n" or "---\r\n" fence.
        // The inline "\n---\n" at position 12 satisfies extract_frontmatter's
        // closing-delimiter search (fm_bytes_len=12 < raw_bytes_len=22), so the
        // closing-fence structural check passes; only Guard 1 catches the missing
        // opening fence.
        let content = "garbage line\n---\nmore\n".to_string();
        let written = Arc::new(Mutex::new(None::<Vec<u8>>));
        let w = written.clone();
        let warnings: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let warn_capture = warnings.clone();

        let result = guard_logic(
            payload_for_post_tool_use("Edit"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content.into_bytes()),
                write_file: move |_path, bytes| {
                    *w.lock().unwrap() = Some(bytes.to_vec());
                    Ok(())
                },
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: move |msg| {
                    warn_capture.lock().unwrap().push(msg.to_string());
                },
                emit_event: |_event_type, _fields| {},
            },
        );

        // Invariant 4 / PC5: hook always returns Continue.
        assert!(
            matches!(result, HookResult::Continue),
            "F-S1705-P6-001 AC-008: guard_logic must return HookResult::Continue when \
             opening fence is absent"
        );

        // PC3a fail-open: no write when structural validation fails.
        assert!(
            written.lock().unwrap().is_none(),
            "F-S1705-P6-001 AC-008: write_file must NOT be called when opening fence is absent"
        );

        // PC3b / EC-005: the distinct opening-fence StampingSkipped advisory must be
        // emitted. This is the mutation-kill assertion: Guard 1 removed → log_warn
        // fires for TimestampAnchorMissing (no "StampingSkipped" + "opening fence
        // absent") → this assertion fails, detecting the missing guard.
        let all_warnings = warnings.lock().unwrap().join("\n");
        assert!(
            all_warnings.contains("StampingSkipped")
                && all_warnings.contains("opening fence absent"),
            "F-S1705-P6-001 AC-008: log_warn must be called with a message containing \
             both 'StampingSkipped' and 'opening fence absent'. \
             Got warnings:\n{all_warnings:?}"
        );
    }

    // -----------------------------------------------------------------------
    // F-S1705-P7-001: CRLF self-lock renewal end-to-end through guard_logic
    // (EC-017 / BC-4.17.001 PC4/AC-010 Invariant 5)
    // -----------------------------------------------------------------------

    /// test_crlf_self_lock_renewal_preserves_crlf_body
    ///
    /// F-S1705-P7-001 / EC-017 / BC-4.17.001 PC4/AC-010 Invariant 5:
    /// When `guard_logic` processes a fully-CRLF STATE.md with a self-held
    /// (identity-matching), far-future (unexpired) `factory_lock`, both PC1
    /// (timestamp restamp) and PC2 (expires_at renewal) fire. The written output MUST:
    ///   (a) have `timestamp:` restamped to `fixed_now()` (2026-08-27T12:00:00Z);
    ///   (b) have `expires_at:` renewed to `fixed_now() + TTL_SECONDS`
    ///       (2026-08-27T12:45:00Z);
    ///   (c) have the body after the closing fence byte-preserved with `\r\n`
    ///       (not converted to bare `\n`);
    ///   (d) have the rewritten `timestamp:` line end with `\r\n` (not bare `\n`);
    ///   (e) have the rewritten `expires_at:` line end with `\r\n` (not bare `\n`);
    ///   (f) have at most two frontmatter lines changed (timestamp + expires_at).
    ///
    /// MUTATION-KILL A (whole-file CRLF normalization before renew_lock_if_holder):
    /// inserting `content_after_pc1.replace("\r\n", "\n")` before the PC2 call →
    /// body bytes lose their `\r` → assertion (c) fails.
    ///
    /// MUTATION-KILL B (timestamp terminator in stamp logic):
    /// dropping the `\r` from the `crlf_line` branch in the PC1 rewrite →
    /// the rewritten `timestamp:` line ends with bare `\n` → assertion (d) fails.
    ///
    /// MUTATION-KILL C (expires_at terminator in rewrite_expires_at):
    /// reverting `rewrite_expires_at` to emit bare `\n` instead of the detected
    /// `\r\n` → assertion (e) fails.
    #[test]
    fn test_crlf_self_lock_renewal_preserves_crlf_body() {
        // CRLF STATE.md with self-held lock (holder == "caller@example.com"),
        // far-future expires_at (2099 → not expired at fixed_now 2026), and a CRLF body.
        // Single-line string so two-space YAML sub-field indentation is exact.
        let content: Vec<u8> = b"---\r\ndocument_type: state\r\ntimestamp: 2020-01-01T00:00:00Z\r\nphase: test\r\nfactory_lock:\r\n  holder: caller@example.com\r\n  locked_at: 2026-01-01T10:00:00Z\r\n  expires_at: 2099-01-01T00:00:00Z\r\n---\r\n\r\n# STATE BODY\r\nExtra body line.\r\n".to_vec();

        let written = Arc::new(Mutex::new(None::<Vec<u8>>));
        let w = written.clone();

        // expected_expires = fixed_now() + TTL_SECONDS = 2026-08-27T12:00:00Z + 2700s
        //                  = 2026-08-27T12:45:00Z
        let expected_expires = (fixed_now() + Duration::seconds(i64::from(flp::TTL_SECONDS)))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        assert_eq!(
            expected_expires, "2026-08-27T12:45:00Z",
            "test fixture sanity: expected_expires must equal fixed_now + TTL_SECONDS"
        );

        let _result = guard_logic(
            payload_for_post_tool_use("Edit"),
            StampCallbacks {
                read_file: move |_path, _max, _timeout| Ok(content),
                write_file: move |_path, bytes| {
                    *w.lock().unwrap() = Some(bytes.to_vec());
                    Ok(())
                },
                // Identity MATCHES lock holder: "caller@example.com" == "caller@example.com".
                exec_subprocess: |_argv| Ok((0, "caller@example.com\n".to_string())),
                now_fn: fixed_now,
                log_warn: |_msg| {},
                emit_event: |_event_type, _fields| {},
            },
        );

        let written_bytes = written.lock().unwrap().clone().expect(
            "F-S1705-P7-001: write_file must be called — PC1 restamps + PC2 renews on self-lock",
        );
        let written_str = String::from_utf8(written_bytes.clone())
            .expect("F-S1705-P7-001: written content must be valid UTF-8");

        // (a) PC1: timestamp restamped to fixed_now().
        assert!(
            written_str.contains("timestamp: 2026-08-27T12:00:00Z"),
            "F-S1705-P7-001 (a) PC1: timestamp must be restamped to fixed_now(). \
             Got:\n{written_str:?}"
        );
        assert!(
            !written_str.contains("timestamp: 2020-01-01T00:00:00Z"),
            "F-S1705-P7-001 (a): stale timestamp must be gone. Got:\n{written_str:?}"
        );

        // (b) PC2: expires_at renewed to fixed_now() + TTL_SECONDS.
        assert!(
            written_str.contains(&format!("  expires_at: {expected_expires}")),
            "F-S1705-P7-001 (b) PC2: expires_at must be renewed to {expected_expires}. \
             Got:\n{written_str:?}"
        );
        assert!(
            !written_str.contains("2099-01-01T00:00:00Z"),
            "F-S1705-P7-001 (b): stale fixture expires_at (2099) must be replaced. \
             Got:\n{written_str:?}"
        );

        // (c) Body after closing fence is byte-preserved with CRLF.
        // MUTATION-KILL A: whole-file \r\n→\n normalization before PC2 →
        // body lines become bare-LF → written_bytes no longer ends with the
        // original CRLF body bytes → this fails.
        assert!(
            written_bytes.ends_with(b"---\r\n\r\n# STATE BODY\r\nExtra body line.\r\n"),
            "F-S1705-P7-001 (c): closing fence + body must be byte-preserved with CRLF. \
             Got tail (last 40 bytes): {:02X?}",
            &written_bytes[written_bytes.len().saturating_sub(40)..]
        );

        // (d) Rewritten timestamp: line ends with \r\n.
        // MUTATION-KILL B: dropping the \r from the crlf_line branch in PC1 rewrite →
        // the timestamp line ends with bare \n → this fails.
        assert!(
            written_str.contains("timestamp: 2026-08-27T12:00:00Z\r\n"),
            "F-S1705-P7-001 (d): rewritten timestamp: line must end with \\r\\n (CRLF). \
             Got:\n{written_str:?}"
        );

        // (e) Rewritten expires_at: line ends with \r\n.
        // MUTATION-KILL C: rewrite_expires_at emits bare \n instead of detected \r\n →
        // the expires_at line ends with bare \n → this fails.
        assert!(
            written_str.contains(&format!("  expires_at: {expected_expires}\r\n")),
            "F-S1705-P7-001 (e): rewritten expires_at: line must end with \\r\\n (CRLF). \
             Got:\n{written_str:?}"
        );

        // (f) At most 2 frontmatter lines changed (timestamp + expires_at).
        let orig = "---\r\ndocument_type: state\r\ntimestamp: 2020-01-01T00:00:00Z\r\nphase: test\r\nfactory_lock:\r\n  holder: caller@example.com\r\n  locked_at: 2026-01-01T10:00:00Z\r\n  expires_at: 2099-01-01T00:00:00Z\r\n---\r\n\r\n# STATE BODY\r\nExtra body line.\r\n";
        let orig_segments: Vec<&str> = orig.split_inclusive('\n').collect();
        let new_segments: Vec<&str> = written_str.split_inclusive('\n').collect();
        let changed: usize = orig_segments
            .iter()
            .zip(new_segments.iter())
            .filter(|(a, b)| a != b)
            .count();
        assert!(
            changed <= 2,
            "F-S1705-P7-001 (f): at most 2 lines may change (timestamp + expires_at), \
             but {changed} changed. Got:\n{written_str:?}"
        );
        assert!(
            changed >= 1,
            "F-S1705-P7-001 (f): at least 1 line must change (timestamp restamp). \
             Got:\n{written_str:?}"
        );
    }
}
