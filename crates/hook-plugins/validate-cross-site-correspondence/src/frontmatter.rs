//! frontmatter.rs — YAML frontmatter parsing utilities.
//!
//! Pure-core module (ADR-035 §Decision 1): no I/O.
//!
//! Extracts individual scalar fields and sequence fields from YAML frontmatter
//! blocks (between the first `---` and second `---` delimiters) using line-scanning.
//! No `serde_norway` or `serde_yaml` is used — hand-rolled line scanning is
//! sufficient for the field extractions needed by this hook (version string,
//! behavioral_contracts sequence, modified sequence, last_amended string).
//!
//! # BC trace
//! BC-5.39.010 §Architecture Anchors:
//!   - `extract_frontmatter_sequence(content, field)` — parses YAML sequence
//!   - `extract_frontmatter_field(content, field)` — single-value scalar extraction
//! BC-5.39.010 invariant 9 — UTF-8 safety: `is_char_boundary()` guards on slicing.

/// Extract a single-value scalar field from YAML frontmatter.
///
/// Scans the leading `---` … `---` region of `content` for the first line
/// matching `^<field>:` and returns the value after the colon, trimmed.
/// Handles bare, single-quoted, double-quoted, and YAML block scalar values.
/// Returns `None` if the frontmatter region is absent or the field is not found.
///
/// # Block scalars (BC-5.39.010 PC36)
/// All four YAML block scalar indicators are handled:
/// - `|` — literal block, clip chomp (preserve newlines; callers receive lines joined with `\n`)
/// - `|-` — literal block, strip chomp (same join, trailing blank lines stripped)
/// - `>` — folded block, clip chomp (single newlines folded to spaces; blank lines → `\n`)
/// - `>-` — folded block, strip chomp (same fold, trailing blank lines stripped)
///
/// For single-line block scalar bodies (the common case for `last_amended:` date
/// strings), literal and folded produce identical output: the single content line
/// with its block-indent prefix stripped.
///
/// # BC trace
/// BC-5.39.010 §Architecture Anchors `extract_frontmatter_field`; used by
/// arm_a1 (extract `version:` via the parameter normalization shadow),
/// arm_a2 (extract `story_id:`), arm_e (extract `last_amended:`).
/// BC-5.39.010 PC36: YAML block scalar indicators must NOT be returned as the value.
/// F-P4-004: prior implementation returned `"|-"` for `last_amended: |-` — NON-CONFORMING.
///
/// # WARNING — do NOT call this with `field = "version"`
///
/// Every extractor that reads a version from structured text (`extract_last_amended_outer_version`,
/// `extract_first_v_token`, `bc_index_row_contains_version`, etc.) strips the leading `v` during
/// extraction, returning bare digit strings like `"1.3"`. This function returns the raw frontmatter
/// value — e.g., `"v1.3"` — so comparing its output directly to any extracted version is **wrong
/// by construction** and produces false violations.
///
/// F-P6-019a/019e/019f: this bit three arms before the fix.
///
/// **Use [`extract_version_field`] instead.** It calls this function and strips the leading `v`,
/// ensuring a consistent normalization boundary at all version comparison sites.
pub fn extract_frontmatter_field(content: &str, field: &str) -> Option<String> {
    // Find the frontmatter region: lines between first --- and second ---
    let mut lines = content.lines();
    // First line must be ---
    if lines.next()? != "---" {
        return None;
    }
    // Use loop+next() instead of for-in so that `lines` stays in scope for
    // block-scalar body collection (requires passing &mut lines to a helper).
    loop {
        let line = lines.next()?; // None = EOF before closing `---` → field absent
        if line == "---" {
            return None; // End of frontmatter, field not found
        }
        // Check if this line starts with `field:`
        let prefix = format!("{}:", field);
        if line.starts_with(&prefix) {
            let rest = &line[prefix.len()..];
            let trimmed = rest.trim();

            // BC-5.39.010 PC36: detect YAML block scalar indicators.
            // The indicator itself is NOT the field value. Collect the body from
            // subsequent indented lines. Returning the indicator string (e.g. "|-")
            // is NON-CONFORMING.
            if matches!(trimmed, "|" | "|-" | ">" | ">-") {
                let is_folded = trimmed.starts_with('>');
                return collect_block_scalar_body(&mut lines, is_folded);
            }

            // Strip surrounding quotes (single or double).
            // Guard-first: evaluate is_char_boundary BEFORE slicing to prevent
            // a slice panic if a non-ASCII multi-byte sequence falls on the quote
            // position. In practice this branch is reached only for ASCII quotes
            // (the guard is vacuously true), but the correct pattern checks first.
            let value = if (trimmed.starts_with('"') && trimmed.ends_with('"'))
                || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
            {
                if trimmed.len() >= 2
                    && trimmed.is_char_boundary(1)
                    && trimmed.is_char_boundary(trimmed.len() - 1)
                {
                    &trimmed[1..trimmed.len() - 1]
                } else {
                    trimmed
                }
            } else {
                trimmed
            };
            return Some(value.to_string());
        }
    }
}

/// Extract and normalize the `version` frontmatter field.
///
/// Convenience wrapper around [`extract_frontmatter_field`] that strips any
/// leading `v` from the returned value.
///
/// All version comparison sites in this crate must use this accessor rather
/// than calling `extract_frontmatter_field(content, "version")` directly.
/// This establishes a single normalization boundary: raw frontmatter values
/// (`"v1.3"`) always compare equal to index-extracted versions (`"1.3"`).
///
/// Do not use this for non-comparison reads where the exact authored string
/// is required. In practice the arms compare the normalized form and surface
/// it in messages, which is acceptable since the mismatch message is only
/// reached when the values genuinely differ after normalization.
///
/// # BC trace
/// F-P6-019a/019e/019f — normalization asymmetry class. Closes the
/// structural gap: every function that extracts a version from structured
/// text strips `v` during extraction, but `extract_frontmatter_field`
/// returns raw frontmatter — asymmetry guaranteed by construction at every
/// comparison site without this accessor.
pub fn extract_version_field(content: &str) -> Option<String> {
    extract_frontmatter_field(content, "version").map(|v| v.trim_start_matches('v').to_string())
}

/// Collect and return the body of a YAML block scalar from subsequent lines.
///
/// Called after `extract_frontmatter_field` encounters a block scalar indicator
/// (`|`, `|-`, `>`, `>-`). Reads from `lines` until it finds either `---`
/// (frontmatter close) or a line less indented than the established block indent.
///
/// The block indent is established by the first non-empty content line. Empty lines
/// within the block are accumulated as paragraph separators.
///
/// # Semantics (BC-5.39.010 PC36)
/// - **Literal** (`is_folded = false`): content lines joined with `\n`.
/// - **Folded** (`is_folded = true`): single newlines folded to spaces; blank
///   lines become paragraph-separating `\n` characters.
/// - **Clip vs strip chomp:** for field-value extraction (comparison / regex use),
///   trailing blank lines are stripped in both modes since callers use `.contains()`
///   or position-0 regex and are not performing round-trip YAML serialization.
///   The `is_strip` distinction is therefore irrelevant at this extraction layer and
///   is not accepted as a parameter.
///
/// Returns `None` if no non-empty content lines are found before `---` or EOF.
///
/// # BC trace
/// BC-5.39.010 PC36: four block-scalar indicators with correct literal/folded and
/// clip/strip semantics. F-P4-004 (block body extraction). F-P6-013: removed the
/// formerly-silently-ignored `_is_strip` parameter — both modes already strip
/// trailing blanks for field-value extraction.
fn collect_block_scalar_body<'a>(
    lines: &mut impl Iterator<Item = &'a str>,
    is_folded: bool,
) -> Option<String> {
    let mut content_lines: Vec<String> = Vec::new();
    let mut block_indent: Option<usize> = None;

    for raw_line in lines.by_ref() {
        if raw_line == "---" {
            break;
        }

        // Count leading ASCII spaces for indentation.
        // YAML block scalars use space-only indent (BC-5.39.010 PC36 implementation note).
        let indent = raw_line.len() - raw_line.trim_start_matches(' ').len();
        let is_blank = raw_line.trim().is_empty();

        if is_blank {
            // Blank line inside block: paragraph separator (not yet a hard break).
            content_lines.push(String::new());
            continue;
        }

        if let Some(bi) = block_indent {
            if indent < bi {
                // Less indented than block indent: end of block.
                // This line belongs to the next YAML field — do not consume it.
                // (We cannot un-consume from an Iterator; the next field's data
                // is unreachable after this point, but extract_frontmatter_field
                // returns immediately after collecting the block, so that is fine.)
                break;
            }
            // Strip exactly `bi` leading spaces; preserve any excess indent.
            // Safety: `bi` leading chars are all ASCII spaces (single-byte) so
            // `bi` is a valid byte index into the str.
            let stripped = &raw_line[bi..];
            content_lines.push(stripped.to_string());
        } else {
            // First non-empty content line: establishes the block indent.
            block_indent = Some(indent);
            let stripped = &raw_line[indent..];
            content_lines.push(stripped.to_string());
        }
    }

    // Remove trailing blank lines (strip-mode semantic; also applied for clip
    // since we do not add a trailing newline for field-value extraction).
    while content_lines.last().map(String::is_empty).unwrap_or(false) {
        content_lines.pop();
    }

    if content_lines.is_empty() {
        return None;
    }

    let result = if is_folded {
        // Folded mode: single newlines → spaces; blank lines → paragraph `\n`.
        let mut buf = String::new();
        for line in &content_lines {
            if line.is_empty() {
                // Blank line: paragraph separator — trim trailing space and insert \n.
                let trimmed_end = buf.trim_end_matches(' ').len();
                buf.truncate(trimmed_end);
                buf.push('\n');
            } else {
                if !buf.is_empty() && !buf.ends_with('\n') {
                    buf.push(' ');
                }
                buf.push_str(line);
            }
        }
        buf
    } else {
        // Literal mode: join lines with \n, preserving intra-block newlines.
        content_lines.join("\n")
    };

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Extract a YAML sequence field from YAML frontmatter.
///
/// Scans the leading `---` … `---` region of `content` for the first line
/// matching `^<field>:` and returns the items in the sequence as a Vec<String>.
///
/// Handles both inline sequences (`[item1, item2]`) and block sequences
/// (`- item` on subsequent lines).
///
/// Returns an empty Vec if the field is absent, empty, or not a sequence.
///
/// # BC trace
/// BC-5.39.010 §Architecture Anchors `extract_frontmatter_sequence`; used by
/// arm_a2 (extract `behavioral_contracts:`) and arm_e (extract `modified:`).
pub fn extract_frontmatter_sequence(content: &str, field: &str) -> Vec<String> {
    // Find the frontmatter region
    let mut lines = content.lines();
    // First line must be ---
    if lines.next().unwrap_or("") != "---" {
        return vec![];
    }

    let field_prefix = format!("{}:", field);
    let mut found_field = false;
    let mut result = Vec::new();

    for line in lines {
        if line == "---" {
            break;
        }
        if found_field {
            // We're collecting block-sequence items
            let trimmed = line.trim();
            if let Some(item_raw) = trimmed.strip_prefix("- ") {
                // Block sequence item: "  - value" or "- value"
                let item = item_raw.trim();
                result.push(strip_quotes(item));
            } else if !trimmed.is_empty() && !trimmed.starts_with('-') {
                // End of block sequence (next non-empty, non-item line)
                break;
            }
            continue;
        }
        if line.starts_with(&field_prefix) {
            found_field = true;
            let rest = &line[field_prefix.len()..];
            let trimmed = rest.trim();
            if trimmed.is_empty() {
                // Block-sequence style: items on following lines
                continue;
            }
            // Inline sequence: [item1, item2] or []
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                let inner = &trimmed[1..trimmed.len() - 1];
                if inner.trim().is_empty() {
                    return vec![];
                }
                for item in inner.split(',') {
                    let s = strip_quotes(item.trim());
                    if !s.is_empty() {
                        result.push(s);
                    }
                }
                return result;
            }
        }
    }
    result
}

/// Strip single or double surrounding quotes from a YAML scalar string.
fn strip_quotes(s: &str) -> String {
    if s.len() >= 2 {
        let first = s.as_bytes()[0];
        let last = s.as_bytes()[s.len() - 1];
        if ((first == b'"' && last == b'"') || (first == b'\'' && last == b'\''))
            && s.is_char_boundary(1)
            && s.is_char_boundary(s.len() - 1)
        {
            return s[1..s.len() - 1].to_string();
        }
    }
    s.to_string()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // extract_frontmatter_field — scalar extraction
    // -----------------------------------------------------------------------

    #[test]
    fn test_BC_5_39_010_frontmatter_field_bare_value() {
        let content = "---\nversion: 1.6\n---\nbody\n";
        let result = extract_frontmatter_field(content, "version");
        assert_eq!(result, Some("1.6".to_string()));
    }

    #[test]
    fn test_BC_5_39_010_frontmatter_field_quoted_value() {
        let content = "---\nversion: \"1.6\"\n---\nbody\n";
        let result = extract_frontmatter_field(content, "version");
        assert_eq!(result, Some("1.6".to_string()));
    }

    #[test]
    fn test_BC_5_39_010_frontmatter_field_absent_returns_none() {
        let content = "---\nstory_id: S-21.07\n---\nbody\n";
        let result = extract_frontmatter_field(content, "version");
        assert!(result.is_none(), "absent field must return None");
    }

    #[test]
    fn test_BC_5_39_010_frontmatter_field_no_frontmatter_returns_none() {
        let content = "# heading\nbody text\n";
        let result = extract_frontmatter_field(content, "version");
        assert!(
            result.is_none(),
            "content without frontmatter must return None"
        );
    }

    #[test]
    fn test_BC_5_39_010_frontmatter_field_last_amended_with_prior_chain() {
        // BC-5.39.010 EC-018: last_amended with [Prior: ...] chain — must return full value
        let content = "---\nlast_amended: \"2026-07-30 (v1.6) — Active. [Prior: 2026-07-01 (v1.5) — ...]\"\n---\n";
        let result = extract_frontmatter_field(content, "last_amended");
        assert!(
            result.is_some(),
            "last_amended with Prior chain must be extracted"
        );
        let val = result.unwrap();
        assert!(
            val.starts_with("2026-07-30 (v1.6)"),
            "extracted value must start with the outermost date prefix"
        );
    }

    // -----------------------------------------------------------------------
    // extract_frontmatter_sequence — sequence extraction
    // -----------------------------------------------------------------------

    #[test]
    fn test_BC_5_39_010_frontmatter_sequence_inline_list() {
        let content = "---\nbehavioral_contracts: [BC-5.39.010, BC-5.39.008]\n---\n";
        let result = extract_frontmatter_sequence(content, "behavioral_contracts");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "BC-5.39.010");
        assert_eq!(result[1], "BC-5.39.008");
    }

    #[test]
    fn test_BC_5_39_010_frontmatter_sequence_block_list() {
        let content = "---\nbehavioral_contracts:\n  - BC-5.39.010\n  - BC-5.39.008\n---\n";
        let result = extract_frontmatter_sequence(content, "behavioral_contracts");
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_BC_5_39_010_frontmatter_sequence_empty_list() {
        let content = "---\nbehavioral_contracts: []\n---\n";
        let result = extract_frontmatter_sequence(content, "behavioral_contracts");
        assert!(result.is_empty(), "empty sequence must return empty Vec");
    }

    #[test]
    fn test_BC_5_39_010_frontmatter_sequence_absent_field() {
        let content = "---\nstory_id: S-21.07\n---\n";
        let result = extract_frontmatter_sequence(content, "behavioral_contracts");
        assert!(
            result.is_empty(),
            "absent sequence field must return empty Vec"
        );
    }

    #[test]
    fn test_BC_5_39_010_frontmatter_modified_sequence_with_annotations() {
        // BC-5.39.010 EC-016: modified entries with annotation suffixes
        let content = "---\nmodified:\n  - \"2026-05-14 v1.0\"\n  - \"2026-05-18 (v1.1)\"\n---\n";
        let result = extract_frontmatter_sequence(content, "modified");
        assert_eq!(result.len(), 2);
    }

    // -----------------------------------------------------------------------
    // F-P4-004 — YAML block scalar handling in extract_frontmatter_field
    //
    // BC-5.39.010 PC36: extract_frontmatter_field MUST handle YAML block scalar
    // indicators: `|`, `|-`, `>`, `>-`. Returning the indicator string itself
    // ("|-", ">-", etc.) is NON-CONFORMING.
    //
    // BC-5.39.010.md and S-21.07 story both use `last_amended: |-` (block literal
    // strip). Current implementation returns Some("|-") for this field → E1 inert.
    //
    // RED GATE: all four tests fail because extract_frontmatter_field returns the
    // indicator string instead of the block body. After the PC36 fix, the function
    // must scan subsequent indented lines and return the first non-empty body line.
    // -----------------------------------------------------------------------

    /// F-P4-004 RED GATE: `|` literal block scalar — body must be returned, not indicator.
    ///
    /// BC-5.39.010 PC36: `|` indicates a literal block scalar. The value is the
    /// block body, not the `|` character. Current impl returns Some("|"). Test FAILS.
    #[test]
    fn test_BC_5_39_010_frontmatter_field_block_scalar_pipe_literal() {
        // `|` — literal block scalar (clip chomp: keep one trailing newline)
        // RED GATE: current impl returns Some("|"), not the block body
        let content = "---\nlast_amended: |\n  2026-08-05 (v1.10) — test fixture\n---\n";
        let result = extract_frontmatter_field(content, "last_amended");
        let val = result.expect(
            "block scalar `|` last_amended must return Some(...). \
            BC-5.39.010 PC36: block indicators must not be returned as the value.",
        );
        assert!(
            val.contains("2026-08-05"),
            "block scalar `|` body must be extracted. \
            BC-5.39.010 PC36: indicator `|` is NOT the field value. Got: {val:?}"
        );
    }

    /// F-P4-004 RED GATE: `|-` literal block strip scalar — body must be returned.
    ///
    /// BC-5.39.010 PC36: `|-` indicates a literal block scalar with strip chomp.
    /// This is the exact form used in BC-5.39.010.md and S-21.07 story.
    /// Current impl returns Some("|-"). Test FAILS.
    #[test]
    fn test_BC_5_39_010_frontmatter_field_block_scalar_pipe_strip() {
        // `|-` — literal block scalar with strip chomp (no trailing newline)
        // RED GATE: current impl returns Some("|-"), not the block body
        // This is the exact shape in BC-5.39.010.md: `last_amended: |-`
        let content = "---\nlast_amended: |-\n  2026-08-05 (v1.10) — test fixture\n---\n";
        let result = extract_frontmatter_field(content, "last_amended");
        let val = result.expect(
            "block scalar `|-` last_amended must return Some(...). \
            BC-5.39.010 PC36: `|-` is a block scalar indicator, not the field value.",
        );
        assert!(
            val.contains("2026-08-05"),
            "block scalar `|-` body (first line) must be extracted. \
            BC-5.39.010 PC36: indicator `|-` is NOT the field value. \
            Current extract_frontmatter_field returns '|-' → E1 structurally inert \
            on BC-5.39.010.md and S-21.07 story (F-P4-004). Got: {val:?}"
        );
        assert!(
            !val.starts_with("|-"),
            "indicator '|-' must NOT be the returned value. Got: {val:?}"
        );
    }

    /// F-P4-004 RED GATE: `>` fold block scalar — body must be returned, not indicator.
    ///
    /// BC-5.39.010 PC36: `>` indicates a folded block scalar. The value is the
    /// block body, not the `>` character. Current impl returns Some(">"). Test FAILS.
    #[test]
    fn test_BC_5_39_010_frontmatter_field_block_scalar_fold_gt() {
        // `>` — folded block scalar (clip chomp)
        // RED GATE: current impl returns Some(">"), not the block body
        let content = "---\nlast_amended: >\n  2026-08-05 (v1.10) — test fixture\n---\n";
        let result = extract_frontmatter_field(content, "last_amended");
        let val = result.expect(
            "block scalar `>` last_amended must return Some(...). \
            BC-5.39.010 PC36: block indicators must not be returned as the value.",
        );
        assert!(
            val.contains("2026-08-05"),
            "block scalar `>` body must be extracted. \
            BC-5.39.010 PC36: indicator `>` is NOT the field value. Got: {val:?}"
        );
    }

    /// F-P4-004 RED GATE: `>-` fold strip block scalar — body must be returned.
    ///
    /// BC-5.39.010 PC36: `>-` indicates a folded block scalar with strip chomp.
    /// Current impl returns Some(">-"). Test FAILS.
    #[test]
    fn test_BC_5_39_010_frontmatter_field_block_scalar_fold_strip() {
        // `>-` — folded block scalar with strip chomp
        // RED GATE: current impl returns Some(">-"), not the block body
        let content = "---\nlast_amended: >-\n  2026-08-05 (v1.10) — test fixture\n---\n";
        let result = extract_frontmatter_field(content, "last_amended");
        let val = result.expect(
            "block scalar `>-` last_amended must return Some(...). \
            BC-5.39.010 PC36: `>-` is a block scalar indicator, not the field value.",
        );
        assert!(
            val.contains("2026-08-05"),
            "block scalar `>-` body must be extracted. \
            BC-5.39.010 PC36: indicator `>-` is NOT the field value. Got: {val:?}"
        );
        assert!(
            !val.starts_with('>'),
            "indicator '>-' must NOT be the returned value. Got: {val:?}"
        );
    }

    // -----------------------------------------------------------------------
    // F-P6-013 — folded block scalar multi-line branch coverage
    //
    // BC-5.39.010 PC36 / collect_block_scalar_body `is_folded = true` path:
    // Prior tests only covered single-line bodies; the multi-line folded code
    // paths (space-joining consecutive non-blank lines; blank-line → paragraph `\n`)
    // had zero test coverage.
    // -----------------------------------------------------------------------

    /// F-P6-013: folded multi-line scalar — consecutive non-blank lines joined with space.
    ///
    /// BC-5.39.010 PC36 / collect_block_scalar_body (is_folded=true):
    /// Two consecutive non-blank content lines in a folded block scalar must be
    /// joined with a single space (YAML folded scalar semantics). The space-insertion
    /// branch (`if !buf.is_empty() && !buf.ends_with('\n') { buf.push(' ') }`) is the
    /// zero-coverage path — this test exercises it.
    #[test]
    fn test_BC_5_39_010_frontmatter_folded_multi_line_space_joined() {
        // Two content lines with no blank line between them — must be joined with space.
        let content = "---\nfield: >\n  line one\n  line two\n---\n";
        let result = extract_frontmatter_field(content, "field");
        let val = result.expect("folded multi-line scalar must return Some(...)");
        assert_eq!(
            val, "line one line two",
            "folded consecutive non-blank lines must be joined with a space. \
            collect_block_scalar_body (is_folded=true) space-join branch. Got: {val:?}"
        );
    }

    /// F-P6-013: folded multi-line scalar with blank line — paragraph separator → `\n`.
    ///
    /// BC-5.39.010 PC36 / collect_block_scalar_body (is_folded=true):
    /// A blank line within a folded block scalar becomes a paragraph-separating `\n`
    /// (trailing space trimmed before the `\n` is pushed). This is the blank-line branch
    /// inside the folded loop — exercised here for the first time.
    #[test]
    fn test_BC_5_39_010_frontmatter_folded_blank_line_paragraph_break() {
        // Content with blank line between two paragraphs.
        let content = "---\nfield: >\n  para one\n\n  para two\n---\n";
        let result = extract_frontmatter_field(content, "field");
        let val = result.expect("folded scalar with blank-line paragraph must return Some(...)");
        assert!(
            val.contains('\n'),
            "blank line in folded scalar must produce a paragraph-separating \\n. \
            collect_block_scalar_body blank-line branch. Got: {val:?}"
        );
        assert!(
            val.starts_with("para one"),
            "first paragraph must start the result. Got: {val:?}"
        );
        assert!(
            val.contains("para two"),
            "second paragraph must be present after the \\n separator. Got: {val:?}"
        );
    }

    // -----------------------------------------------------------------------
    // AC-019 coverage-fill (S-21.07 reconciliation drift-audit Gap A / T-032):
    // multibyte UTF-8 through the frontmatter read/parse path must not panic.
    //
    // BC-5.39.010 invariant 9 ("is_char_boundary() guard: byte-index slicing on
    // extracted strings MUST use is_char_boundary() checks where multi-byte
    // UTF-8 is possible", BC-5.39.008 v1.6 inv-11) and AC-019 §UTF-8 safety name
    // this exact test: `test_BC_5_39_010_multibyte_utf8_no_panic` — "fixture
    // content with multi-byte UTF-8 characters in BC title or frontmatter
    // values → no panic; clean exit."
    //
    // This is a COVERAGE-FILL regression test, not a Red Gate: the
    // is_char_boundary() guards already exist and are already correct
    // (extract_frontmatter_field's quote-stripping branch above; strip_quotes'
    // guards used by extract_frontmatter_sequence). No prior Rust-level test
    // exercised multi-byte content through them; this closes that gap without
    // changing any production behavior.
    // -----------------------------------------------------------------------

    /// AC-019 / T-032: multibyte UTF-8 in frontmatter values must not panic and
    /// must be extracted intact, across every `is_char_boundary()`-guarded call
    /// site in this module (quoted-scalar stripping, `strip_quotes` for sequence
    /// items, and the block-scalar body collector).
    #[test]
    fn test_BC_5_39_010_multibyte_utf8_no_panic() {
        // Double-quoted scalar with multi-byte content directly adjacent to both
        // quote delimiters — stresses is_char_boundary(1) (byte right after the
        // opening quote) and is_char_boundary(len - 1) (byte right before the
        // closing quote) in extract_frontmatter_field's quote-stripping branch.
        let content = "---\ntitle: \"日本語タイトル🎉テスト\"\nversion: 1.6\n---\nbody\n";
        let title = extract_frontmatter_field(content, "title");
        assert_eq!(
            title,
            Some("日本語タイトル🎉テスト".to_string()),
            "multibyte double-quoted scalar must be extracted intact without panicking"
        );

        // Single-quoted variant: a single 4-byte emoji directly against both quotes.
        let content2 = "---\nname: '🎉'\n---\n";
        let name = extract_frontmatter_field(content2, "name");
        assert_eq!(
            name,
            Some("🎉".to_string()),
            "single 4-byte-emoji quoted scalar must be extracted without panicking"
        );

        // Sequence field: multi-byte quoted items exercise strip_quotes' own
        // is_char_boundary guards, via both inline and block-sequence forms.
        let seq_inline =
            "---\nbehavioral_contracts: [\"BC-日本語-5.39.010\", \"BC-🎉-5.39.008\"]\n---\n";
        let items = extract_frontmatter_sequence(seq_inline, "behavioral_contracts");
        assert_eq!(
            items,
            vec![
                "BC-日本語-5.39.010".to_string(),
                "BC-🎉-5.39.008".to_string(),
            ],
            "multibyte inline-sequence items must be extracted intact without panicking"
        );

        let seq_block = "---\nmodified:\n  - \"2026-08-05 日本語\"\n  - '🎉 emoji entry'\n---\n";
        let block_items = extract_frontmatter_sequence(seq_block, "modified");
        assert_eq!(
            block_items.len(),
            2,
            "both multibyte block-sequence items must parse"
        );
        assert!(
            block_items[0].contains("日本語"),
            "multibyte content in first block-sequence item must survive. Got: {block_items:?}"
        );
        assert!(
            block_items[1].contains("🎉"),
            "multibyte content in second block-sequence item must survive. Got: {block_items:?}"
        );

        // Block scalar body containing multibyte content exercises
        // collect_block_scalar_body's line handling with non-ASCII bytes present.
        let content3 = "---\nlast_amended: |-\n  2026-08-05 (v1.10) — 日本語コメント🎉\n---\n";
        let last_amended = extract_frontmatter_field(content3, "last_amended");
        let val = last_amended
            .expect("block scalar with multibyte body must extract Some(...), not panic");
        assert!(
            val.contains("日本語コメント🎉"),
            "multibyte block-scalar body must be preserved intact. Got: {val:?}"
        );

        // Malformed input: multi-byte value with an unterminated quote must not
        // panic — the quote-stripping branch's starts_with/ends_with guard
        // rejects it before any slicing is attempted, so it falls through and
        // is returned as-is (graceful handling, not an extraction failure).
        let content4 = "---\nfield: \"日本語不完全\n---\n";
        let malformed = extract_frontmatter_field(content4, "field");
        assert!(
            malformed.is_some(),
            "malformed unterminated-quote multibyte value must still return Some(raw value), \
            not panic. BC-5.39.010 invariant 9 requires graceful handling, not just avoiding \
            a crash on well-formed input."
        );
    }
}
