//! factory-lock-parse — shared library crate (D15, S-17.04).
//!
//! Exports the parse primitives extracted from `verify-factory-lock::lib`:
//!   - `LockState` — parsed `factory_lock` block (holder, locked_at, expires_at)
//!   - `LockParseError` — error variants from lock-block parsing
//!   - `parse_factory_lock(content: &str) -> Result<Option<LockState>, LockParseError>`
//!   - `extract_yaml_string_value(line: &str, key: &str) -> Option<String>`
//!   - `parse_iso8601(s: &str) -> Result<chrono::DateTime<chrono::Utc>, LockParseError>`
//!
//! Both `verify-factory-lock` and `verify-state-timestamp-refresh` depend on this
//! crate instead of each maintaining independent frontmatter scanners.
//!
//! # Architecture compliance
//!
//! - No `serde_yaml` / `serde_norway` — manual line-by-line scan (Architecture
//!   Compliance Rule 4 / ADR-025 Decision 12 §12.4).
//! - No `regex` crate — manual tokenisation only.
//! - `chrono` (workspace dep) is the only non-std dependency.
//! - Pure library: no I/O, no host calls. All functions are deterministic given input.
//!
//! # Implementation status (S-17.04 T-2 — COMPLETE)
//!
//! All public functions are fully implemented and their unit tests pass green.
//! This crate is the canonical source for `parse_factory_lock`, `LockState`,
//! `extract_yaml_string_value`, and `parse_iso8601` (D15 / S-17.04).

// Allow the BC-based test naming convention (non_snake_case is workspace-allowed).
#![cfg_attr(not(kani), allow(unexpected_cfgs))]

// ---------------------------------------------------------------------------
// Canonical shared constants (S-17.05 T-2 / ADR-046 F-006 / BC-4.17.001 Precondition 3)
// ---------------------------------------------------------------------------

/// Canonical TTL in seconds for `factory_lock.expires_at` renewal.
///
/// Cross-references `plugins/vsdd-factory/bin/factory-lock-write.sh` `TTL_SECONDS=2700`.
/// Both must remain in sync (BC-4.17.001 Invariant 3 / ADR-046 F-006 single-canonical-TTL-source).
/// Imported by `stamp-state-timestamp` for PC2 renewal (`expires_at = now + TTL_SECONDS`);
/// also used to migrate `crates/factory-lock/renew_lock_with_now`'s bare `2700` literal (T-2).
///
/// BC-5.38.001: `test_ttl_seconds_constant_equals_2700` asserts `TTL_SECONDS == 2700`
/// (S-17.05 T-2 / ADR-046 F-006).
pub const TTL_SECONDS: u32 = 2700;

/// Canonical STATE.md byte read-cap shared across the hook family.
///
/// Single canonical home per ADR-046 Decision 5 / F-P5-001.
/// Imported by `stamp-state-timestamp` as the `max_bytes` argument to
/// `host::read_file(".factory/STATE.md", STATE_MD_MAX_BYTES, timeout_ms)`.
/// Parity with BC-5.40.001 / BC-4.13.001 cap-parity requirement (formerly a
/// per-crate local constant `STATE_MD_MAX_BYTES = 262144` in `verify-state-timestamp-refresh`).
///
/// BC-5.38.001: `test_state_md_max_bytes_constant_equals_262144` asserts
/// `STATE_MD_MAX_BYTES == 262144` (ADR-046 Decision 5 / F-P5-001).
pub const STATE_MD_MAX_BYTES: u32 = 262_144;

// ---------------------------------------------------------------------------
// Error variants
// ---------------------------------------------------------------------------

/// Error variants from lock-block parsing.
///
/// Mirrors `LockCheckError` in `verify-factory-lock` but scoped to parse-only
/// concerns — no `StateReadError` or `IdentityResolutionFailed` (those are
/// plugin-level, not parse-level).
#[derive(Debug, Clone, PartialEq)]
pub enum LockParseError {
    /// The `factory_lock` block is present but malformed (missing field, empty
    /// field, unexpected inline value, or absent closing `---` delimiter).
    MalformedLockBlock(String),
}

// ---------------------------------------------------------------------------
// Parsed lock state
// ---------------------------------------------------------------------------

/// A successfully-parsed `factory_lock` block from STATE.md frontmatter.
///
/// All three sub-fields (`holder`, `locked_at`, `expires_at`) are required;
/// absence of any field routes to `MalformedLockBlock`.
#[derive(Debug, Clone, PartialEq)]
pub struct LockState {
    /// Email of the current lock holder.
    pub holder: String,
    /// ISO-8601 timestamp when the lock was acquired.
    pub locked_at: String,
    /// ISO-8601 datetime when the lock auto-expires.
    pub expires_at: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Extract the YAML frontmatter prefix from raw STATE.md bytes.
///
/// Scans `bytes` for the closing frontmatter delimiter in LF or CRLF form
/// and returns the prefix `bytes[0..delimiter_start_offset]`:
///
/// - **LF inline** `\n---\n`: `delimiter_start_offset` = byte index of the
///   leading `\n`. Returned slice = `bytes[0..pos]`.
/// - **CRLF inline** `\r\n---\r\n`: `delimiter_start_offset` = byte index of
///   the leading `\r`. Returned slice = `bytes[0..pos]`.
/// - **CRLF EOF** `\r\n---` at EOF (no trailing newline): `delimiter_start_offset`
///   = `len - 5`. Must be checked before LF-EOF because `\r\n---` also ends
///   with `\n---`.
/// - **LF EOF** `\n---` at EOF (no trailing newline): `delimiter_start_offset`
///   = `len - 4`.
/// - **None found**: returns the full input unchanged.
///
/// The opening delimiter (`---\n` for LF files; `---\r\n` for CRLF files) IS
/// included in the returned slice (slice starts at byte 0). The closing
/// delimiter bytes are NOT included.
///
/// This function is pure: deterministic, no I/O, side-effect free.
///
/// # BC Traces
/// - BC-4.13.001 v1.15 Phase-A Invariant 9 (frontmatter-only-parsing mandate;
///   CRLF delimiter forms `\r\n---\r\n` / `\r\n---`-EOF added in v1.15 / EC-017)
/// - AC-005, VP-096 (extract_frontmatter byte-exact boundary purity)
/// - F-P29-001 (architect ruling: this function lives in factory-lock-parse)
pub fn extract_frontmatter(bytes: &[u8]) -> &[u8] {
    // 1. LF inline `\n---\n` — most common case (pure-LF STATE.md).
    //    delimiter_start_offset = byte index of the leading `\n`.
    let lf_inline = b"\n---\n";
    if let Some(pos) = bytes.windows(lf_inline.len()).position(|w| w == lf_inline) {
        return &bytes[..pos];
    }
    // 2. CRLF inline `\r\n---\r\n` — Windows autocrlf checkout (EC-017).
    //    delimiter_start_offset = byte index of the leading `\r`.
    let crlf_inline = b"\r\n---\r\n";
    if let Some(pos) = bytes
        .windows(crlf_inline.len())
        .position(|w| w == crlf_inline)
    {
        return &bytes[..pos];
    }
    // 3. CRLF EOF `\r\n---` — CRLF file with no trailing newline after delimiter.
    //    Checked BEFORE LF-EOF because `\r\n---` also ends with `\n---` and
    //    the LF-EOF check would strip only 4 bytes instead of 5, leaving a
    //    stray `\r` in the returned slice.
    let crlf_eof = b"\r\n---";
    if bytes.ends_with(crlf_eof) {
        return &bytes[..bytes.len() - crlf_eof.len()];
    }
    // 4. LF EOF `\n---` — LF file with no trailing newline after delimiter.
    let lf_eof = b"\n---";
    if bytes.ends_with(lf_eof) {
        return &bytes[..bytes.len() - lf_eof.len()];
    }
    // No closing delimiter found: return the full input.
    bytes
}

/// Scan the YAML frontmatter of STATE.md content for the `factory_lock:` block.
///
/// Reads only the region between the first and second `---\n` delimiters.
/// Uses a line-by-line scan (no YAML parser; no `regex` crate).
/// Sub-fields are indented with exactly 2 spaces under `factory_lock:`.
///
/// Returns:
/// - `Ok(None)` if the `factory_lock` key is absent (unlocked path).
/// - `Ok(Some(LockState))` if all three sub-fields are present and non-empty.
/// - `Err(MalformedLockBlock)` if the block is present but malformed.
pub fn parse_factory_lock(content: &str) -> Result<Option<LockState>, LockParseError> {
    // Normalise Windows-style CRLF line endings to LF before scanning.
    let normalised;
    let content = if content.contains('\r') {
        normalised = content.replace("\r\n", "\n");
        normalised.as_str()
    } else {
        content
    };

    // Extract frontmatter region: between first and second `---\n`.
    let frontmatter = if let Some(after_open) = content.strip_prefix("---\n") {
        if let Some(close_pos) = after_open.find("\n---\n").or_else(|| {
            if after_open.ends_with("\n---") {
                Some(after_open.len() - 4)
            } else {
                None
            }
        }) {
            &after_open[..close_pos]
        } else {
            // No closing `---` delimiter — malformed frontmatter.
            return Err(LockParseError::MalformedLockBlock(
                "missing closing --- delimiter".to_string(),
            ));
        }
    } else {
        // No opening `---\n` — no frontmatter at all, treat as unlocked.
        return Ok(None);
    };

    // Scan lines for `factory_lock:` key.
    let mut in_factory_lock = false;
    let mut holder: Option<String> = None;
    let mut locked_at: Option<String> = None;
    let mut expires_at: Option<String> = None;

    for line in frontmatter.lines() {
        if line == "factory_lock:" || line.starts_with("factory_lock:") {
            let after_colon = line["factory_lock:".len()..].trim();
            if after_colon.is_empty() || after_colon == "~" || after_colon == "null" {
                in_factory_lock = true;
            } else {
                return Err(LockParseError::MalformedLockBlock(
                    "factory_lock key has unexpected inline value".to_string(),
                ));
            }
            continue;
        }

        if in_factory_lock {
            // Sub-fields must be indented with exactly 2 spaces.
            if line.starts_with("  ") && !line.starts_with("   ") {
                let field_line = &line[2..];
                if let Some(value) = extract_yaml_string_value(field_line, "holder") {
                    holder = Some(value);
                } else if let Some(value) = extract_yaml_string_value(field_line, "locked_at") {
                    locked_at = Some(value);
                } else if let Some(value) = extract_yaml_string_value(field_line, "expires_at") {
                    expires_at = Some(value);
                }
            } else if !line.is_empty() {
                // Non-indented, non-empty line after factory_lock: — exited the block.
                in_factory_lock = false;
            }
        }
    }

    // If factory_lock was not found, return Ok(None) (unlocked).
    if !in_factory_lock && holder.is_none() && locked_at.is_none() && expires_at.is_none() {
        return Ok(None);
    }

    // factory_lock block present but null/empty — treat as unlocked.
    if in_factory_lock && holder.is_none() && locked_at.is_none() && expires_at.is_none() {
        return Ok(None);
    }

    // factory_lock was found — validate all three required sub-fields.
    let holder_val = match holder {
        Some(h) if !h.is_empty() => h,
        Some(_) => {
            return Err(LockParseError::MalformedLockBlock(
                "factory_lock.holder is empty string".to_string(),
            ));
        }
        None => {
            return Err(LockParseError::MalformedLockBlock(
                "factory_lock.holder field is absent".to_string(),
            ));
        }
    };

    let locked_at_val = match locked_at {
        Some(v) if !v.is_empty() => v,
        Some(_) => {
            return Err(LockParseError::MalformedLockBlock(
                "factory_lock.locked_at is empty string".to_string(),
            ));
        }
        None => {
            return Err(LockParseError::MalformedLockBlock(
                "factory_lock.locked_at field is absent".to_string(),
            ));
        }
    };

    let expires_at_val = match expires_at {
        Some(v) if !v.is_empty() => v,
        Some(_) => {
            return Err(LockParseError::MalformedLockBlock(
                "factory_lock.expires_at is empty string".to_string(),
            ));
        }
        None => {
            return Err(LockParseError::MalformedLockBlock(
                "factory_lock.expires_at field is absent".to_string(),
            ));
        }
    };

    Ok(Some(LockState {
        holder: holder_val,
        locked_at: locked_at_val,
        expires_at: expires_at_val,
    }))
}

/// Extract the string value from a YAML key-value line like `key: "value"` or `key: value`.
///
/// Returns `Some(value)` if the line starts with `{key}: `, otherwise `None`.
/// Strips surrounding double-quotes from quoted values.
/// Returns `Some("")` for empty quoted values `""`.
pub fn extract_yaml_string_value(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{}: ", key);
    let bare_prefix = format!("{}:", key);

    let raw_value = if let Some(rest) = line.strip_prefix(&prefix) {
        rest
    } else if line == bare_prefix {
        // `key:` with no value — treat as empty.
        ""
    } else {
        return None;
    };

    // Strip surrounding double-quotes if present.
    let value = if raw_value.starts_with('"') && raw_value.ends_with('"') && raw_value.len() >= 2 {
        &raw_value[1..raw_value.len() - 1]
    } else {
        raw_value
    };

    Some(value.to_string())
}

/// Parse an ISO-8601 datetime string into a `chrono::DateTime<chrono::Utc>`.
///
/// Returns `Ok(dt)` on success, `Err(MalformedLockBlock)` if unparseable.
pub fn parse_iso8601(s: &str) -> Result<chrono::DateTime<chrono::Utc>, LockParseError> {
    s.parse::<chrono::DateTime<chrono::Utc>>().map_err(|e| {
        LockParseError::MalformedLockBlock(format!("invalid ISO-8601 datetime '{}': {}", s, e))
    })
}

// ---------------------------------------------------------------------------
// Unit tests — factory-lock-parse crate (D15 / S-17.04)
//
// These tests exercise the public API of this crate directly.
// All tests pass green against the implemented functions (T-2 complete).
//
// Test naming: test_BC_5_40_001_xxx() — traces to BC-5.40.001 PC4 (shared
// parse logic is a prerequisite for the timestamp-refresh guard).
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

    use super::*;

    // -----------------------------------------------------------------------
    // Fixtures (inline — no I/O needed for pure parse functions)
    // -----------------------------------------------------------------------

    /// Minimal STATE.md with NO factory_lock block (unlocked baseline).
    fn state_no_lock() -> &'static str {
        "---\ndocument_type: state\nversion: \"0.0.1-test\"\nphase: test\n---\n\n# STATE\n"
    }

    /// STATE.md with a valid factory_lock block (all three sub-fields present).
    fn state_with_valid_lock() -> &'static str {
        concat!(
            "---\n",
            "document_type: state\n",
            "version: \"0.0.1-test\"\n",
            "phase: test\n",
            "factory_lock:\n",
            "  holder: \"holder@example.com\"\n",
            "  locked_at: \"2026-06-10T14:00:00Z\"\n",
            "  expires_at: \"2099-01-01T00:00:00Z\"\n",
            "---\n\n# STATE\n",
        )
    }

    /// STATE.md with a factory_lock block where holder is empty string (malformed).
    fn state_with_empty_holder() -> &'static str {
        concat!(
            "---\n",
            "document_type: state\n",
            "version: \"0.0.1-test\"\n",
            "phase: test\n",
            "factory_lock:\n",
            "  holder: \"\"\n",
            "  locked_at: \"2026-06-10T14:00:00Z\"\n",
            "  expires_at: \"2099-01-01T00:00:00Z\"\n",
            "---\n\n# STATE\n",
        )
    }

    // -----------------------------------------------------------------------
    // S-19.02 extract_frontmatter tests (T-004, T-005, T-007, T-008)
    //
    // RED GATE: extract_frontmatter is currently a todo!() stub; all four
    // tests will fail until the implementation is complete (Task 10).
    // -----------------------------------------------------------------------

    /// T-004 (AC-003): extract_frontmatter with a `\n---\n` delimiter present.
    ///
    /// Fixture: `---\nfactory_lock: null\n---\nbody content`
    /// Expected: extracted slice contains only the frontmatter region (bytes
    /// 0..delimiter_start_offset), ending before the closing `\n---\n`.
    ///
    /// RED: extract_frontmatter is a todo!() stub.
    #[test]
    fn test_S1902_extract_frontmatter_delimiter_present_excludes_body() {
        let input = b"---\nfactory_lock: null\n---\nbody content here";
        let extracted = extract_frontmatter(input);
        // Body content must not appear in the extracted slice.
        let extracted_str = std::str::from_utf8(extracted).expect("extracted slice must be UTF-8");
        assert!(
            !extracted_str.contains("body content here"),
            "extracted slice must NOT contain body bytes past the delimiter. Got: {extracted_str:?}"
        );
        // Extracted slice must start with the opening `---\n`.
        assert!(
            extracted_str.starts_with("---\n"),
            "extracted slice must start with opening `---\\n` (byte 0). Got: {extracted_str:?}"
        );
    }

    /// T-005 (AC-003): extract_frontmatter with no `\n---\n` delimiter → full slice returned.
    ///
    /// Fixture: bytes with no closing `---` delimiter.
    /// Expected: returned slice byte-equals the full input.
    ///
    /// RED: extract_frontmatter is a todo!() stub.
    #[test]
    fn test_S1902_extract_frontmatter_no_delimiter_returns_full_slice() {
        let input = b"---\nfactory_lock: null\nphase: test\n";
        let extracted = extract_frontmatter(input);
        assert_eq!(
            extracted, input,
            "When no closing delimiter present, extract_frontmatter must return the full input slice"
        );
    }

    /// T-007 (AC-005): extract_frontmatter byte-exact boundary — body bytes absent from extracted slice.
    ///
    /// Unit test A (boundary): fixture = `---\nfactory_lock: null\n---\nbody content here`
    ///   - Extracted slice must start at byte 0 (includes opening `---\n`).
    ///   - extracted_len must equal delimiter_start_offset (index of `\n` beginning `\n---\n`).
    ///   - `body content here` must NOT appear in the extracted slice.
    ///
    /// RED: extract_frontmatter is a todo!() stub.
    #[test]
    fn test_S1902_extract_frontmatter_byte_exact_boundary_body_absent() {
        // Fixture: known structure; delimiter_start_offset = byte index of the `\n` in `\n---\n`
        // "---\nfactory_lock: null" = 22 bytes; then `\n---\nbody content here`
        // So delimiter_start_offset = 22, extracted_len must == 22.
        let input = b"---\nfactory_lock: null\n---\nbody content here";
        let extracted = extract_frontmatter(input);

        // delimiter_start_offset = index of the '\n' that starts '\n---\n'
        // input[22] == b'\n', input[23..27] == b"---\n"
        let delimiter_start_offset = 22usize;

        assert_eq!(
            extracted.len(),
            delimiter_start_offset,
            "extracted_len must equal delimiter_start_offset ({}). Got: {}",
            delimiter_start_offset,
            extracted.len()
        );
        assert_eq!(
            extracted,
            &input[..delimiter_start_offset],
            "extracted slice must byte-equal input[0..delimiter_start_offset]"
        );
        // Opening `---\n` is included (starts at byte 0).
        assert!(
            extracted.starts_with(b"---\n"),
            "extracted slice must start with opening `---\\n`"
        );
        // No body bytes leak through.
        assert!(
            !extracted.windows(4).any(|w| w == b"body"),
            "body bytes must NOT appear in extracted slice"
        );
    }

    /// T-008 (AC-005): extract_frontmatter EOF delimiter — `\n---` at EOF, no trailing body.
    ///
    /// Unit test B (EOF delimiter): fixture = `---\nfactory_lock: null\n---`
    /// Expected: extracted slice contains only frontmatter content (no trailing `\n---`).
    ///
    /// RED: extract_frontmatter is a todo!() stub.
    #[test]
    fn test_S1902_extract_frontmatter_eof_delimiter_no_trailing_body() {
        let input = b"---\nfactory_lock: null\n---";
        let extracted = extract_frontmatter(input);
        let extracted_str = std::str::from_utf8(extracted).expect("extracted slice must be UTF-8");
        // The `\n---` EOF delimiter itself must not appear in the extracted content.
        // Extracted slice should be the frontmatter up to (but not including) `\n---`.
        assert!(
            !extracted_str.ends_with("\n---"),
            "extracted slice must NOT end with the `\\n---` EOF delimiter. Got: {extracted_str:?}"
        );
        assert!(
            extracted_str.starts_with("---\n"),
            "extracted slice must start with opening `---\\n`. Got: {extracted_str:?}"
        );
    }

    // -----------------------------------------------------------------------
    // F-S1902-P1-001: CRLF extract_frontmatter tests (pass-1 adversary finding)
    //
    // BC-4.13.001 v1.14→v1.15 amendment (human approved): extract_frontmatter
    // MUST recognize CRLF delimiter forms (`\r\n---\r\n` and `\r\n---` at EOF)
    // in addition to the LF forms already implemented.
    //
    // These tests are RED because the current implementation only searches for
    // `b"\n---\n"` and `b"\n---"` (LF-only). CRLF inputs return the full slice.
    // -----------------------------------------------------------------------

    /// F-S1902-P1-001 / T-011-A: extract_frontmatter CRLF inline delimiter — body excluded.
    ///
    /// Fixture: `---\r\nfactory_lock: null\r\n---\r\nbody content`
    /// Expected: extracted slice starts with `---\r\n` and does NOT contain `body content`.
    ///
    /// RED: current implementation searches only for `\n---\n`; the CRLF form
    /// `\r\n---\r\n` is not recognized → full input returned including body bytes.
    #[test]
    fn test_S1902_crlf_extract_frontmatter_excludes_body() {
        let input = b"---\r\nfactory_lock: null\r\n---\r\nbody content";
        let extracted = extract_frontmatter(input);
        let extracted_str = std::str::from_utf8(extracted).expect("extracted slice must be UTF-8");
        // Body bytes must not appear in the extracted frontmatter slice.
        assert!(
            !extracted_str.contains("body content"),
            "CRLF: extracted slice must NOT contain body bytes past the \\r\\n---\\r\\n delimiter. \
             Got: {extracted_str:?}. \
             Fix: update extract_frontmatter to recognize `\\r\\n---\\r\\n` (BC-4.13.001 v1.15)."
        );
        // Slice must start with opening `---\r\n`.
        assert!(
            extracted_str.starts_with("---\r\n"),
            "CRLF: extracted slice must start with opening `---\\r\\n` (byte 0). Got: {extracted_str:?}"
        );
    }

    /// F-S1902-P1-001 / T-011-B: extract_frontmatter CRLF byte-exact boundary.
    ///
    /// Fixture: `---\r\nfactory_lock: null\r\n---\r\nbody`
    ///   - `---\r\nfactory_lock: null` = 23 bytes (---=3 + \r\n=2 + factory_lock: null=18);
    ///     then `\r\n---\r\n` begins at byte 23.
    ///   - delimiter_start_offset (index of `\r` in `\r\n---\r\n`) = 23.
    ///   - Extracted slice must byte-equal input[0..23].
    ///
    /// RED: current implementation does not recognize `\r\n---\r\n` → returns full input.
    #[test]
    fn test_S1902_crlf_extract_frontmatter_byte_exact_boundary() {
        let input = b"---\r\nfactory_lock: null\r\n---\r\nbody";
        let extracted = extract_frontmatter(input);
        // delimiter_start_offset: index of the `\r` that starts `\r\n---\r\n`.
        // input[0..23] = b"---\r\nfactory_lock: null" (23 bytes: ---=3 + \r\n=2 + factory_lock: null=18)
        let delimiter_start_offset = 23usize;
        assert_eq!(
            extracted.len(),
            delimiter_start_offset,
            "CRLF: extracted_len ({}) must equal delimiter_start_offset ({}). \
             Full input returned instead of frontmatter-only slice.",
            extracted.len(),
            delimiter_start_offset
        );
        assert_eq!(
            extracted,
            &input[..delimiter_start_offset],
            "CRLF: extracted slice must byte-equal input[0..delimiter_start_offset ({})]",
            delimiter_start_offset
        );
    }

    // -----------------------------------------------------------------------
    // parse_factory_lock tests
    // -----------------------------------------------------------------------

    /// parse_factory_lock: valid block → Ok(Some(LockState)) with correct fields.
    #[test]
    fn test_BC_5_40_001_parse_factory_lock_returns_some_on_valid_block() {
        let result = parse_factory_lock(state_with_valid_lock());
        let lock = result
            .expect("parse must succeed on valid content")
            .expect("lock block must be present");
        assert_eq!(lock.holder, "holder@example.com");
        assert_eq!(lock.locked_at, "2026-06-10T14:00:00Z");
        assert_eq!(lock.expires_at, "2099-01-01T00:00:00Z");
    }

    /// parse_factory_lock: no factory_lock key → Ok(None).
    #[test]
    fn test_BC_5_40_001_parse_factory_lock_returns_none_when_absent() {
        let result = parse_factory_lock(state_no_lock())
            .expect("parse must not error on valid unlocked content");
        assert!(
            result.is_none(),
            "Absent factory_lock block must return Ok(None)"
        );
    }

    /// parse_factory_lock: malformed block (empty holder) → Err(MalformedLockBlock).
    #[test]
    fn test_BC_5_40_001_parse_factory_lock_errors_on_empty_holder() {
        let result = parse_factory_lock(state_with_empty_holder());
        match result {
            Err(LockParseError::MalformedLockBlock(_)) => {
                // Correct.
            }
            Ok(Some(lock)) => panic!(
                "Expected MalformedLockBlock for empty holder, got Ok(Some) with holder: '{}'",
                lock.holder
            ),
            Ok(None) => panic!("Expected MalformedLockBlock for empty holder, got Ok(None)"),
        }
    }

    /// parse_factory_lock: CRLF line endings → same result as LF-only.
    #[test]
    fn test_BC_5_40_001_parse_factory_lock_handles_crlf_line_endings() {
        // CRLF version of state_with_valid_lock.
        let crlf = concat!(
            "---\r\n",
            "document_type: state\r\n",
            "version: \"0.0.1-test\"\r\n",
            "phase: test\r\n",
            "factory_lock:\r\n",
            "  holder: \"holder@example.com\"\r\n",
            "  locked_at: \"2026-06-10T14:00:00Z\"\r\n",
            "  expires_at: \"2099-01-01T00:00:00Z\"\r\n",
            "---\r\n\r\n# STATE\r\n",
        );
        let result = parse_factory_lock(crlf);
        let lock = result
            .expect("CRLF content must parse successfully")
            .expect("lock block must be found in CRLF content");
        assert_eq!(lock.holder, "holder@example.com");
        assert_eq!(lock.expires_at, "2099-01-01T00:00:00Z");
    }

    // -----------------------------------------------------------------------
    // extract_yaml_string_value tests
    // -----------------------------------------------------------------------

    /// extract_yaml_string_value: bare unquoted value → Some("value").
    #[test]
    fn test_BC_5_40_001_extract_yaml_string_value_bare_unquoted() {
        let result = extract_yaml_string_value("holder: user@example.com", "holder");
        assert_eq!(
            result,
            Some("user@example.com".to_string()),
            "Bare unquoted value must be returned as-is"
        );
    }

    /// extract_yaml_string_value: double-quoted value → Some("value") without quotes.
    #[test]
    fn test_BC_5_40_001_extract_yaml_string_value_double_quoted() {
        let result =
            extract_yaml_string_value("expires_at: \"2099-01-01T00:00:00Z\"", "expires_at");
        assert_eq!(
            result,
            Some("2099-01-01T00:00:00Z".to_string()),
            "Double-quoted value must strip surrounding quotes"
        );
    }

    /// extract_yaml_string_value: wrong key → None.
    #[test]
    fn test_BC_5_40_001_extract_yaml_string_value_wrong_key_returns_none() {
        let result = extract_yaml_string_value("holder: user@example.com", "expires_at");
        assert!(result.is_none(), "Non-matching key must return None");
    }

    // -----------------------------------------------------------------------
    // parse_iso8601 tests
    // -----------------------------------------------------------------------

    /// parse_iso8601: valid UTC ISO-8601 string → Ok(DateTime).
    #[test]
    fn test_BC_5_40_001_parse_iso8601_valid_utc_string_succeeds() {
        let result = parse_iso8601("2026-06-10T14:00:00Z");
        assert!(
            result.is_ok(),
            "Valid ISO-8601 string must parse successfully"
        );
    }

    /// parse_iso8601: invalid string → Err(MalformedLockBlock).
    #[test]
    fn test_BC_5_40_001_parse_iso8601_invalid_string_errors() {
        let result = parse_iso8601("not-a-timestamp");
        match result {
            Err(LockParseError::MalformedLockBlock(_)) => {
                // Correct.
            }
            Ok(_) => panic!("Invalid timestamp must NOT parse successfully"),
        }
    }

    // -----------------------------------------------------------------------
    // S-17.05 T-2 Red Gate tests — TTL_SECONDS + STATE_MD_MAX_BYTES constants
    // (BC-4.17.001 Precondition 3 / Invariant 3 / ADR-046 F-006 / F-P5-001)
    //
    // Both tests MUST FAIL against the stub values (0) until S-17.05 T-2 is
    // implemented and sets them to the correct canonical values (2700 / 262144).
    // -----------------------------------------------------------------------

    /// S-17.05 T-2 / AC-012 / BC-4.17.001 Precondition 3 / Invariant 3:
    /// TTL_SECONDS must equal 2700 (canonical factory_lock TTL in seconds).
    ///
    /// Cross-references `plugins/vsdd-factory/bin/factory-lock-write.sh` TTL_SECONDS=2700
    /// (ADR-046 F-006 single-canonical-TTL-source). stamp-state-timestamp imports this
    /// constant for PC2 renewal (expires_at = now + TTL_SECONDS).
    ///
    /// RED GATE: stub sets TTL_SECONDS = 0 → assert_eq fails until implementer sets 2700.
    #[test]
    fn test_ttl_seconds_constant_equals_2700() {
        assert_eq!(
            TTL_SECONDS, 2700u32,
            "TTL_SECONDS must equal 2700 (canonical factory_lock TTL; \
             AC-012 / BC-4.17.001 Precondition 3 / Invariant 3 / ADR-046 F-006). \
             Stub has 0 — S-17.05 T-2 must set to 2700."
        );
    }

    /// S-17.05 T-2 / AC-012 / ADR-046 Decision 5 / F-P5-001:
    /// STATE_MD_MAX_BYTES must equal 262144 (canonical STATE.md byte read-cap).
    ///
    /// Single canonical home per ADR-046 Decision 5 — imported by stamp-state-timestamp
    /// as the max_bytes argument to host::read_file (BC-5.40.001 / BC-4.13.001 cap-parity).
    ///
    /// RED GATE: stub sets STATE_MD_MAX_BYTES = 0 → assert_eq fails until implementer sets 262144.
    #[test]
    fn test_state_md_max_bytes_constant_equals_262144() {
        assert_eq!(
            STATE_MD_MAX_BYTES, 262_144u32,
            "STATE_MD_MAX_BYTES must equal 262144 (canonical STATE.md read-cap; \
             AC-012 / ADR-046 Decision 5 / F-P5-001). \
             Stub has 0 — S-17.05 T-2 must set to 262144."
        );
    }
}
