//! arm_e.rs — Class E: frontmatter version/last_amended parity and modified[] monotonicity.
//!
//! Pure-core module (ADR-035 §Decision 1): operates on already-read content.
//!
//! # Two sub-arms
//!
//! ## E1 — version vs last_amended outer prefix
//! Fires on BC, VP, story, and epic file writes. Compares the `version:` field
//! against the outermost version prefix in `last_amended:`.
//! Extraction uses regex `^\d{4}-\d{2}-\d{2}\s+\(v([0-9]+(?:\.[0-9]+)*)\)` anchored
//! at CHARACTER POSITION 0 of the field value (BC-5.39.010 precondition 37).
//! Prior-chain versions inside `[Prior: ... (v1.5) ...]` are excluded STRUCTURALLY
//! by the positional anchor (not by `[Prior:` pattern match).
//!
//! ## E2 — modified[] date sequence monotonicity
//! Fires on BC, VP, story, and epic file writes. Extracts the `modified:` sequence,
//! strips annotation suffixes (e.g., `(v1.1)` → strip to `YYYY-MM-DD`), and verifies
//! dates are non-decreasing (monotonic). Equal dates (same-day entries) are permitted;
//! only a genuine decrease triggers a violation.
//!
//! # Block semantics
//! Both E1 and E2 violations are blocking. A single file write may trigger both.
//! All violations are combined into ONE `block_with_fix` message (postcondition 23).
//!
//! # Advisory semantics
//! If `last_amended:` does not match the regex (unparseable format), emit advisory
//! and return Continue — do NOT block on unparseable format (precondition 37 last sentence).
//!
//! # BC trace
//! BC-5.39.010 Class E preconditions, postconditions, and invariant 9 (UTF-8 safety;
//! current version per BC-INDEX) — E1 frontmatter `version:` vs `last_amended:`
//! text-prefix parity and E2 `modified:` sequence date monotonicity. Not in scope for
//! the v1.22 secondary-index-file amendment (precondition 15b / postcondition 26
//! governs Arm A1's BC-INDEX.md and Arm B1's STORY-INDEX.md secondary reads only).

use crate::{Advisory, Violation};

/// Extract the outermost version prefix from a `last_amended:` field value.
///
/// Applies regex `^\d{4}-\d{2}-\d{2}\s+\(v([0-9]+(?:\.[0-9]+)*)\)` anchored at
/// CHARACTER POSITION 0 of `last_amended` (the field value, not the full frontmatter line).
///
/// Returns `Some(version_string)` if the regex matches (e.g., `Some("1.6")`).
/// Returns `None` if the regex does not match (unparseable format).
///
/// The positional anchor at character position 0 ensures nested `(v1.5)` tokens
/// inside a `[Prior: ...]` chain are NEVER matched — they are not at position 0.
/// This is a structural exclusion, NOT a pattern-match exclusion on `[Prior:`.
///
/// Hand-rolled: no `regex` crate. Uses manual byte-walk for date and version parsing
/// within the WASM fuel budget.
///
/// # BC trace
/// BC-5.39.010 precondition 37 (positional regex anchor); EC-018 (Prior-chain exclusion).
pub fn extract_last_amended_outer_version(last_amended: &str) -> Option<String> {
    // Hand-rolled parse of: ^\d{4}-\d{2}-\d{2}\s+\(v([0-9]+(?:\.[0-9]+)*)\)
    // anchored at CHARACTER POSITION 0 of last_amended.
    let bytes = last_amended.as_bytes();
    let len = bytes.len();

    // F-S2107-P1C-014: minimum valid pattern is `YYYY-MM-DD (v1)` = 15 bytes
    // (single-digit outer version with no sub-version suffix). The old guard
    // `len < 17` rejected valid 15-byte strings. New threshold: 14 (strictly
    // less than 15 = minimum valid). All subsequent bounds checks are safe for
    // len ≥ 14 because they iterate with `pos < len` guards throughout.
    if len < 14 {
        return None;
    }

    // Match \d{4}-\d{2}-\d{2}
    let date_ok = bytes[0..4].iter().all(|b| b.is_ascii_digit())
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(|b| b.is_ascii_digit())
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(|b| b.is_ascii_digit());

    if !date_ok {
        return None;
    }

    // Skip whitespace after date (\s+)
    let mut pos = 10;
    while pos < len && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
        pos += 1;
    }
    if pos >= len || bytes[pos] != b'(' {
        return None;
    }
    pos += 1; // skip '('

    // Match 'v'
    if pos >= len || bytes[pos] != b'v' {
        return None;
    }
    pos += 1; // skip 'v'

    // Match [0-9]+(\.[0-9]+)*
    let version_start = pos;
    if pos >= len || !bytes[pos].is_ascii_digit() {
        return None;
    }
    while pos < len && bytes[pos].is_ascii_digit() {
        pos += 1;
    }
    while pos < len && bytes[pos] == b'.' {
        pos += 1;
        if pos >= len || !bytes[pos].is_ascii_digit() {
            return None;
        }
        while pos < len && bytes[pos].is_ascii_digit() {
            pos += 1;
        }
    }
    let version_end = pos;

    // Match ')'
    if pos >= len || bytes[pos] != b')' {
        return None;
    }

    // version_start..version_end is guaranteed ASCII (digits and dots)
    Some(last_amended[version_start..version_end].to_string())
}

/// Strip annotation suffixes from a `modified:` sequence entry.
///
/// Entries may have suffixes like `(v1.1)`, `v1.3`, `: description text`, etc.
/// Strip everything after the first whitespace to extract just the `YYYY-MM-DD` date.
///
/// Returns the stripped date string. All characters are ASCII so byte slicing is safe.
///
/// # BC trace
/// BC-5.39.010 precondition 38 (suffix strip for E2 monotonicity check).
pub fn strip_date_annotation(entry: &str) -> String {
    // Strip everything after the first whitespace to extract just YYYY-MM-DD.
    // All chars before first whitespace are ASCII so byte slicing is safe.
    match entry.find([' ', '\t']) {
        Some(idx) => entry[..idx].to_string(),
        None => entry.to_string(),
    }
}

/// Class E1 check: version vs last_amended outer version parity.
///
/// Returns a `Vec<Violation>` (0 or 1 entries):
/// - Empty: `version:` and `last_amended:` outer prefix agree.
/// - One violation: mismatch between `version:` and `last_amended:` outer prefix.
///   Message must contain `[Class E1]`, both version values, and cite
///   `POLICY 14 leg 4 / POLICY 17`.
///
/// Returns a separate `Vec<Advisory>` for the unparseable-format case:
/// - One advisory: `last_amended:` does not match the regex (precondition 37 last sentence).
///
/// # BC trace
/// BC-5.39.010 Class E1: frontmatter `version:` vs `last_amended:` outer-prefix
/// parity check (precondition 37 positional regex anchor), invariant 9
/// (`is_char_boundary()` UTF-8 slicing safety).
pub fn run_arm_e1(content: &str) -> (Vec<Violation>, Vec<Advisory>) {
    use crate::frontmatter::extract_frontmatter_field;
    // F-P6-019f: extract_version_field normalizes at the parse boundary.
    // `extract_last_amended_outer_version` skips 'v' during extraction and
    // always returns a bare digit string (e.g., "1.3"). Without this accessor,
    // raw frontmatter `version: "v1.3"` would compare as "v1.3" != "1.3" →
    // false [Class E1] violation even when both values agree.
    // See frontmatter::extract_version_field for the class-level rationale.
    use crate::frontmatter::extract_version_field;

    let version = match extract_version_field(content) {
        Some(v) => v,
        None => return (vec![], vec![]), // No version field → no check
    };

    let last_amended_raw = match extract_frontmatter_field(content, "last_amended") {
        Some(la) => la,
        None => {
            // F-S2107-P1C-015: when version: is present, last_amended: should also be
            // present. Emit an advisory (not a block) so the author can add the field.
            let advisory = Advisory {
                message: format!(
                    "validate-cross-site-correspondence [Class E1] advisory: \
                    version '{version}' is set but last_amended field is absent. \
                    Both fields must be present per BC-5.39.010 §E1."
                ),
            };
            return (vec![], vec![advisory]);
        }
    };

    match extract_last_amended_outer_version(&last_amended_raw) {
        None => {
            // Unparseable format → advisory, not block (PC37 last sentence)
            let advisory = Advisory {
                message: format!(
                    "validate-cross-site-correspondence [Class E1] advisory: \
                    unparseable last_amended format — could not extract outer version prefix from '{last_amended_raw}'. \
                    Expected format: 'YYYY-MM-DD (vN.N) — description'."
                ),
            };
            (vec![], vec![advisory])
        }
        Some(amended_version) => {
            if amended_version == version {
                (vec![], vec![])
            } else {
                let violation = Violation {
                    description: format!(
                        "validate-cross-site-correspondence [Class E1]: version '{version}' \
                        does not match last_amended outer version prefix '(v{amended_version})'. \
                        Update last_amended to reflect the current version. \
                        POLICY 14 leg 4 / POLICY 17."
                    ),
                };
                (vec![violation], vec![])
            }
        }
    }
}

/// Class E2 check: modified[] sequence monotonicity.
///
/// Returns a `Vec<Violation>` (0 or more entries):
/// - Empty: dates are non-decreasing (monotonic) after suffix strip. Equal dates are
///   permitted — same-day entries (multiple bursts in one day) are the normal cadence.
/// - One violation: first genuine decrease found (block on first violation).
///   Message must contain `[Class E2]` and identify the non-monotonic pair.
///
/// The relation checked is `∀i: date[i] ≤ date[i+1]`. Only a genuine decrease
/// (`date[i+1] < date[i]`) is a violation; equality is allowed.
///
/// # BC trace
/// BC-5.39.010 precondition 38 (suffix strip) and postcondition 22 (block on first
/// non-monotonic `modified:` date pair — postcondition 23 no-block-on-equality
/// clause).
pub fn run_arm_e2(content: &str) -> Vec<Violation> {
    use crate::frontmatter::extract_frontmatter_sequence;

    let modified = extract_frontmatter_sequence(content, "modified");
    if modified.len() < 2 {
        return vec![];
    }

    let dates: Vec<String> = modified.iter().map(|e| strip_date_annotation(e)).collect();

    // Verify non-decreasing (monotonic): each date must be >= previous.
    // Equal dates are permitted (same-day entries are normal).
    // Only a genuine decrease (curr < prev) is a violation.
    for window in dates.windows(2) {
        let prev = &window[0];
        let curr = &window[1];
        // Lexicographic comparison works for YYYY-MM-DD dates.
        if curr < prev {
            return vec![Violation {
                description: format!(
                    "validate-cross-site-correspondence [Class E2]: modified[] sequence is \
                    non-monotonic — '{curr}' is earlier than preceding '{prev}'. \
                    Dates must be in non-decreasing order (equal dates are permitted). \
                    POLICY 14 leg 3."
                ),
            }];
        }
    }
    vec![]
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // extract_last_amended_outer_version — BC-5.39.010 precondition 37
    // -----------------------------------------------------------------------

    #[test]
    fn test_BC_5_39_010_class_e1_last_amended_version_extracted() {
        let last_amended = "2026-07-29 (v1.6) — some description text";
        let result = extract_last_amended_outer_version(last_amended);
        assert_eq!(result, Some("1.6".to_string()));
    }

    #[test]
    fn test_BC_5_39_010_class_e1_unparseable_last_amended_returns_none() {
        // AC-015: unparseable format → None (advisory path in run_arm_e1)
        let last_amended = "some-nonstandard-format-here";
        let result = extract_last_amended_outer_version(last_amended);
        assert!(
            result.is_none(),
            "unparseable last_amended must return None"
        );
    }

    /// AC-017: Prior-chain version excluded by positional anchor (EC-018).
    #[test]
    fn test_BC_5_39_010_class_e1_prior_chain_version_excluded() {
        // The positional anchor matches only the outermost (v1.6), not the Prior (v1.5)
        let last_amended = "2026-07-30 (v1.6) — Active text. [Prior: 2026-07-01 (v1.5) — old text]";
        let result = extract_last_amended_outer_version(last_amended);
        assert_eq!(
            result,
            Some("1.6".to_string()),
            "positional anchor must extract only outermost version (v1.6)"
        );
    }

    // -----------------------------------------------------------------------
    // strip_date_annotation — BC-5.39.010 precondition 38
    // -----------------------------------------------------------------------

    #[test]
    fn test_BC_5_39_010_class_e2_strip_plain_date() {
        let result = strip_date_annotation("2026-05-14");
        assert_eq!(result, "2026-05-14");
    }

    #[test]
    fn test_BC_5_39_010_class_e2_strip_version_annotation() {
        let result = strip_date_annotation("2026-05-18 (v1.1)");
        assert_eq!(result, "2026-05-18");
    }

    #[test]
    fn test_BC_5_39_010_class_e2_strip_descriptive_suffix() {
        let result = strip_date_annotation("2026-05-20 v1.3: Added Class E2");
        assert_eq!(result, "2026-05-20");
    }

    // -----------------------------------------------------------------------
    // run_arm_e1 — BC-5.39.010 postconditions 19-21
    // -----------------------------------------------------------------------

    /// AC-015 MUTANT: version 1.33 vs last_amended (v1.31) → block [Class E1].
    #[test]
    fn test_BC_5_39_010_class_e1_version_mismatch_blocks() {
        let content = "---\nversion: \"1.33\"\n\
            last_amended: \"2026-07-29 (v1.31) — some text\"\n---\nbody\n";
        let (violations, _) = run_arm_e1(content);
        assert!(
            !violations.is_empty(),
            "version mismatch must produce a blocking violation"
        );
        let msg = &violations[0].description;
        assert!(msg.contains("[Class E1]"), "violation must cite [Class E1]");
        assert!(
            msg.contains("POLICY 14 leg 4") || msg.contains("POLICY 17"),
            "violation must cite POLICY 14 leg 4 / POLICY 17"
        );
    }

    /// AC-015 CONTROL: version 1.6 matches last_amended (v1.6) → passes.
    #[test]
    fn test_BC_5_39_010_class_e1_version_match_passes() {
        let content =
            "---\nversion: \"1.6\"\nlast_amended: \"2026-07-29 (v1.6) — text\"\n---\nbody\n";
        let (violations, _) = run_arm_e1(content);
        assert!(violations.is_empty(), "matching version must not block");
    }

    /// AC-015: unparseable last_amended → advisory, not block.
    #[test]
    fn test_BC_5_39_010_class_e1_unparseable_last_amended_advisory() {
        let content = "---\nversion: \"1.6\"\nlast_amended: \"nonstandard\"\n---\nbody\n";
        let (violations, advisories) = run_arm_e1(content);
        assert!(
            violations.is_empty(),
            "unparseable last_amended must not block"
        );
        assert!(
            !advisories.is_empty(),
            "unparseable last_amended must emit an advisory"
        );
    }

    /// AC-017: Prior-chain version in last_amended is excluded by positional anchor.
    #[test]
    fn test_BC_5_39_010_class_e1_prior_chain_excluded() {
        // version "1.6"; last_amended outermost prefix "(v1.6)"; prior chain has "(v1.5)"
        let content = "---\nversion: \"1.6\"\n\
            last_amended: \"2026-07-30 (v1.6) — Active. [Prior: 2026-07-01 (v1.5) — old.]\"\n\
            ---\nbody\n";
        let (violations, _) = run_arm_e1(content);
        assert!(
            violations.is_empty(),
            "Prior-chain (v1.5) must not trigger E1 mismatch"
        );
    }

    /// Over-broad exclusion mutant for E1 (BC-5.39.010 EC-018 / precondition 37):
    /// The OUTERMOST version in last_amended is WRONG AND a [Prior:] chain is present.
    ///
    /// A bad implementation that says "skip check if [Prior: is present" would miss
    /// this genuine mismatch. The positional anchor ONLY excludes nested Prior-chain
    /// versions, not the outermost position-0 version.
    ///
    /// BC-5.39.010 precondition 37: extraction is by CHARACTER POSITION 0, not by
    /// [Prior: pattern matching. F-S2104-P29-H02 failed exactly this shape.
    #[test]
    fn test_BC_5_39_010_class_e1_outermost_wrong_prior_chain_present_still_blocks() {
        // version "1.6" but outermost last_amended says "(v1.5)" AND has [Prior:] chain
        // The outermost mismatch MUST block despite the Prior chain being present
        let content = "---\nversion: \"1.6\"\n\
            last_amended: \"2026-07-30 (v1.5) — Active. [Prior: 2026-07-01 (v1.4) — old.]\"\n\
            ---\nbody\n";
        let (violations, _) = run_arm_e1(content);
        assert!(
            !violations.is_empty(),
            "outermost version mismatch (1.5 vs 1.6) must block even when [Prior:] chain \
            is present — exclusion is positional (char pos 0), not pattern-based"
        );
        let msg = &violations[0].description;
        assert!(msg.contains("[Class E1]"), "violation must cite [Class E1]");
    }

    // -----------------------------------------------------------------------
    // run_arm_e2 — BC-5.39.010 postcondition 22
    // -----------------------------------------------------------------------

    /// AC-016 MUTANT: non-ascending modified[] → block [Class E2].
    #[test]
    fn test_BC_5_39_010_class_e2_non_monotonic_blocks() {
        // ["2026-05-14", "2026-05-18 (v1.1)", "2026-05-15"] — 2026-05-15 < 2026-05-18
        let content = "---\nmodified:\n  - \"2026-05-14\"\n  - \"2026-05-18 (v1.1)\"\n\
            - \"2026-05-15\"\n---\nbody\n";
        let violations = run_arm_e2(content);
        assert!(
            !violations.is_empty(),
            "non-monotonic modified[] must block"
        );
        assert!(
            violations[0].description.contains("[Class E2]"),
            "violation must cite [Class E2]"
        );
    }

    /// AC-016 CONTROL: strictly ascending modified[] → passes.
    #[test]
    fn test_BC_5_39_010_class_e2_ascending_passes() {
        let content = "---\nmodified:\n  - \"2026-05-14\"\n  - \"2026-05-18\"\n\
            - \"2026-05-20 (v1.3)\"\n---\nbody\n";
        let violations = run_arm_e2(content);
        assert!(
            violations.is_empty(),
            "strictly ascending modified[] must not block"
        );
    }

    /// E2 boundary CONTROL: equal dates (same-day entries) must be permitted.
    ///
    /// BC-5.39.010 specifies monotonicity (`∀i: date[i] ≤ date[i+1]`) — equality is
    /// allowed. Multiple bursts per day is the normal cadence; blocking on equal dates
    /// would fire on nearly every multi-burst day and render the gate useless.
    ///
    /// Without this test, `curr <= prev` (strict) silently re-passes all existing tests
    /// yet breaks on real-world same-day entries.
    #[test]
    fn test_BC_5_39_010_class_e2_equal_dates_permitted() {
        // ["2026-05-14", "2026-05-14"] — same day, two bursts — monotonic by ≤
        let content = "---\nmodified:\n  - \"2026-05-14\"\n  - \"2026-05-14\"\n---\nbody\n";
        let violations = run_arm_e2(content);
        assert!(
            violations.is_empty(),
            "equal dates (same-day entries) must not trigger E2 — \
            monotonicity allows equality (∀i: date[i] ≤ date[i+1])"
        );
    }

    /// E2 boundary MUTANT: genuine decrease must still block.
    ///
    /// Confirms that changing `<=` to `<` did not accidentally allow genuine decreases.
    #[test]
    fn test_BC_5_39_010_class_e2_genuine_decrease_blocks() {
        // ["2026-05-18", "2026-05-14"] — date goes backward — non-monotonic → block
        let content = "---\nmodified:\n  - \"2026-05-18\"\n  - \"2026-05-14\"\n---\nbody\n";
        let violations = run_arm_e2(content);
        assert!(
            !violations.is_empty(),
            "genuine decrease (2026-05-14 after 2026-05-18) must block"
        );
        assert!(
            violations[0].description.contains("[Class E2]"),
            "violation must cite [Class E2]"
        );
    }

    // -----------------------------------------------------------------------
    // F-S2107-P1C-014: 15-byte last_amended string rejected by length guard.
    // BC-5.39.010 v1.19 §E1: "2026-07-30 (v2)" is a valid last_amended format
    // (single-digit outer version, no sub-version suffix). The string is 15 bytes.
    // Pre-fix code (now closed): `if len < 17 { return None }` — 15 < 17 → returned None.
    // Returning None used to make the advisory "unparseable format" fire → exit 2.
    // Expected (and now implemented): extract Some("2"), match BC version "2", exit 0, no advisory.
    // -----------------------------------------------------------------------

    /// T-045 (Rust unit test): 15-byte last_amended must parse to Some("2") (F-S2107-P1C-014).
    ///
    /// RED GATE: `if len < 17 { return None }` → 15 < 17 → returns None.
    /// assert_eq!(result, Some("2".to_string())) FAILS → RED gate.
    /// After fix (lower threshold to 14): returns Some("2") → PASSES.
    #[test]
    fn test_BC_5_39_010_class_e1_15_byte_last_amended_accepted() {
        // "2026-07-30 (v2)" is exactly 15 bytes: 10 + 1 + 1 + 1 + 1 + 1 = 15
        let s = "2026-07-30 (v2)";
        assert_eq!(s.len(), 15, "precondition: string must be 15 bytes");

        let result = extract_last_amended_outer_version(s);
        assert_eq!(
            result,
            Some("2".to_string()),
            "15-byte last_amended '2026-07-30 (v2)' must parse to outer version '2'. \
            BC-5.39.010 v1.19 §E1 single-digit outer versions are valid (F-S2107-P1C-014). \
            Red Gate: current `if len < 17 {{return None}}` rejects 15-byte strings → None"
        );
    }

    // -----------------------------------------------------------------------
    // F-S2107-P1C-015: run_arm_e1 returns (vec![], vec![]) when version present
    // but last_amended field absent — silently passes instead of emitting advisory.
    // BC-5.39.010 v1.19 §E1: when version: is present, last_amended: must also be
    // present. If absent, an advisory MUST be emitted (not a silent pass).
    // -----------------------------------------------------------------------

    /// F-S2107-P1C-015: absent last_amended when version present must emit advisory.
    ///
    /// RED GATE: current code returns `(vec![], vec![])` on None from
    /// `extract_frontmatter_field(content, "last_amended")` — no advisory emitted.
    /// `assert!(!advisories.is_empty())` FAILS → RED gate.
    /// After fix (return advisory when last_amended missing but version present):
    /// advisory vector non-empty → PASSES.
    #[test]
    fn test_BC_5_39_010_class_e1_absent_last_amended_emits_advisory() {
        // BC file with version but NO last_amended field
        let content = "---\nversion: \"1.6\"\nstatus: draft\n---\n\nbody content\n";
        let (violations, advisories) = run_arm_e1(content);
        assert!(
            violations.is_empty(),
            "absent last_amended must not BLOCK — only advisory (F-S2107-P1C-015)"
        );
        assert!(
            !advisories.is_empty(),
            "absent last_amended when version present must emit advisory. \
            BC-5.39.010 v1.19 §E1: both fields must be present (F-S2107-P1C-015). \
            Red Gate: current code returns (vec![], vec![]) silently → advisory IS empty → FAILS"
        );
    }

    /// Over-broad exclusion mutant for E2: an entry with a complex annotation
    /// (including [Prior:] or similar frozen-provenance text) must still have its
    /// DATE checked for monotonicity after suffix strip.
    ///
    /// BC-5.39.010 precondition 38: strip suffix, compare dates. A bad implementation
    /// that skips entries containing "Prior" would miss the date violation below.
    #[test]
    fn test_BC_5_39_010_class_e2_entry_with_complex_annotation_still_checked() {
        // "2026-07-18 (v1.1): Active [Prior: ...]" → strip → "2026-07-18"
        // Followed by "2026-07-15" → 2026-07-15 < 2026-07-18 → non-monotonic → BLOCK
        // A bad implementation that skips "Prior" entries would pass this incorrectly
        let content = "---\nmodified:\n  - \"2026-07-14\"\n  \
            - \"2026-07-18 (v1.1): Active [Prior: 2026-07-01 (v1.0) — first]\"\n  \
            - \"2026-07-15\"\n---\nbody\n";
        let violations = run_arm_e2(content);
        // run_arm_e2 is fully implemented: strips suffixes →
        // ["2026-07-14", "2026-07-18", "2026-07-15"]
        // 2026-07-15 < 2026-07-18 → non-monotonic → block
        assert!(
            !violations.is_empty(),
            "modified[] entries with complex annotations must still have dates checked \
            for monotonicity after suffix strip (BC-5.39.010 PC38)"
        );
    }

    // -----------------------------------------------------------------------
    // F-P6-019f — arm_e1 v-prefix normalization asymmetry
    //
    // Root cause (same class as F-P6-019a / F-P6-019e):
    //   `extract_last_amended_outer_version` skips the `v` byte at parse time
    //   (arm_e.rs: `pos += 1; // skip 'v'`; `version_start = pos`) and returns
    //   the bare digit substring — always v-stripped (e.g., "1.3").
    //
    //   `version` comes from `extract_frontmatter_field(content, "version")` —
    //   raw frontmatter, may have a leading `v` (e.g., "v1.3").
    //
    //   Comparison at `if amended_version == version`:
    //     "1.3" == "v1.3" → false → spurious [Class E1] violation.
    //
    // Fix (implementer): normalize `version` with `strip_prefix('v')` before
    // the comparison at arm_e.rs run_arm_e1, so "1.3" == "1.3".
    //
    // The control (019f-b) verifies that a genuinely stale last_amended still
    // blocks after the fix — preventing over-normalisation from silencing real
    // E1 violations.
    // -----------------------------------------------------------------------

    /// F-P6-019f RED GATE: BC with `version: "v1.3"` and matching `last_amended: "... (v1.3) ..."`
    /// currently fires a false [Class E1] violation; must produce NO violation after fix.
    ///
    /// `extract_last_amended_outer_version("2026-07-22 (v1.3) — desc")` → `"1.3"` (v-stripped).
    /// `extract_frontmatter_field(content, "version")` → `"v1.3"` (raw).
    /// Comparison: `"1.3" == "v1.3"` → false → violation. WRONG.
    ///
    /// RED GATE: `violations.is_empty()` FAILS (false violation produced now).
    /// After fix (normalize `version` to strip leading `v` before comparison): no violation.
    #[test]
    fn test_F_P6_019f_v_prefix_asymmetry_must_not_block() {
        // Synthetic fixture: BC frontmatter uses `version: "v1.3"` (v-prefixed).
        // The last_amended outer version is "(v1.3)" — a matching, current value.
        // extract_last_amended_outer_version skips 'v', returns "1.3".
        // extract_frontmatter_field returns "v1.3".
        // "1.3" == "v1.3" → false → false violation produced.
        let content = "---\nversion: \"v1.3\"\n\
            last_amended: \"2026-07-22 (v1.3) — description text\"\n\
            ---\nbody\n";
        let (violations, _advisories) = run_arm_e1(content);
        assert!(
            violations.is_empty(),
            "F-P6-019f: BC with version: \"v1.3\" and last_amended outer version (v1.3) \
            are in agreement — must produce NO violation. \
            `extract_last_amended_outer_version` returns \"1.3\" (v-stripped); \
            `extract_frontmatter_field` returns \"v1.3\" (raw). \
            Comparison \"1.3\" == \"v1.3\" is false → false [Class E1] block. \
            Fix: normalize version with strip_prefix('v') before comparison. \
            Violations: {:?}",
            violations
        );
    }

    /// F-P6-019f CONTROL: genuinely stale last_amended must still block after the fix.
    ///
    /// `version: "v1.3"` but `last_amended` outer prefix is `(v1.2)` — a real E1 mismatch.
    /// After fix: `version` normalised to `"1.3"`, `amended_version` is `"1.2"`. `"1.2" != "1.3"` → violation.
    ///
    /// This test is GREEN now and must remain GREEN after the fix — over-normalisation
    /// must not silence genuine last_amended staleness violations.
    #[test]
    fn test_F_P6_019f_genuinely_stale_last_amended_still_blocks() {
        let content = "---\nversion: \"v1.3\"\n\
            last_amended: \"2026-07-20 (v1.2) — previous description\"\n\
            ---\nbody\n";
        let (violations, _advisories) = run_arm_e1(content);
        assert!(
            !violations.is_empty(),
            "F-P6-019f CONTROL: last_amended outer version '1.2' is genuinely stale \
            against version: \"v1.3\" (normalised to \"1.3\"). Must still produce a \
            blocking [Class E1] violation after the v-prefix normalization fix. \
            Violations: {:?}",
            violations
        );
    }
}
