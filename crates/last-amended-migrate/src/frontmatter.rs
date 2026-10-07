//! Frontmatter parsing primitive for `last-amended-migrate`.
//!
//! Isolates exactly the two YAML frontmatter fields BC-10.13.001 operates on
//! — `last_amended` (a single scalar) and `changelog:` (a top-level sequence,
//! when present) — without depending on a general-purpose YAML parser.
//! Invariant 3 requires this parse to stay linear-time/bounded-memory even
//! against the D-1149 calibration ceiling (a 323,499-char single scalar
//! line), which rules out any parse strategy with quadratic backtracking on
//! long lines (the same constraint `factory-lock-parse` documents for its own
//! hand-rolled scan, "serde_yaml/serde_norway FORBIDDEN").
//!
//! # Implementation decision — hand-rolled bounded scan, not a YAML library
//!
//! Every function in this module operates via a small, constant number of
//! single forward/backward linear scans (`str::find`/`str::rfind`/
//! `str::split_inclusive`, each called once or a bounded number of times per
//! field) rather than a general-purpose YAML parser. This is a deliberate
//! choice (S-15.03 implementer decision, ambiguity #1): loading the whole
//! document into `serde_norway`/`serde_yaml` would require that library to
//! tokenize the single 323K-350K-char `last_amended` scalar line as part of a
//! general-purpose grammar, which is not a bounded-cost operation this crate
//! controls or can prove linear for. The hand-rolled scan here mirrors
//! `factory-lock-parse`'s own documented approach for the same reason.
//!
//! `serde_norway` IS a real `[dependencies]` entry as of S-15.03 SEC-001
//! (this stale comment previously said "only in dev-dependencies" — no
//! longer accurate): `src/yaml_guard.rs` now uses it in PRODUCTION code as
//! an independent, bounded-size, OUTPUT-side pre-write validity gate (see
//! that module's own doc comment) — but never for THIS module's hot-path
//! read, which stays the hand-rolled bounded scan described above for the
//! same D-1149-calibration-scale reason. It is separately used in this
//! crate's test suite (`tests/common::strict_yaml_parse`) for independent
//! black-box output verification, unrelated to the production gate.

use crate::error::MigrateError;
use std::path::{Path, PathBuf};

/// Parsed frontmatter of one of the 5 BC-10.13.001 target files, isolating
/// `last_amended` and `changelog:` for the migration/rotation subcommands.
///
/// `changelog_items_raw` holds each sequence item's exact raw YAML block text
/// verbatim (not a re-serializable struct) so PC5 rotation can move items
/// without any parse/reserialize round-trip risking a content change — see
/// `crate::changelog::ChangelogItem` for the distinct struct this tool uses
/// when *constructing* a brand-new item to prepend.
#[derive(Debug, Clone)]
pub struct FrontmatterDoc {
    /// Absolute path of the file this frontmatter was parsed from.
    pub path: PathBuf,
    /// The full raw file content, unmodified. Writers reconstruct the file by
    /// patching precise byte ranges of this string rather than
    /// re-serializing the whole document, preserving untouched content
    /// byte-for-byte — the same raw-byte-preservation discipline
    /// `compute-input-hash` documents for its own file handling.
    pub raw: String,
    /// Raw value of `last_amended:` (unescaped/unquoted), or `None` if the
    /// field is absent from frontmatter entirely. Absence is not expected on
    /// any of the 5 target files but is represented rather than assumed.
    pub last_amended_raw: Option<String>,
    /// Whether a top-level `changelog:` key exists in frontmatter at all —
    /// distinct from an empty sequence. PC1 cares about presence.
    pub changelog_present: bool,
    /// Each `changelog:` sequence item's raw YAML block text, in file order
    /// (newest-first, matching the prepend convention), verbatim.
    pub changelog_items_raw: Vec<String>,
}

/// Locate the `---`-delimited frontmatter region within `raw`, returning the
/// byte range `(start, end)` of the content BETWEEN the fences (excluding
/// both fence lines and the newline immediately preceding the closing
/// fence). A single forward `find` call — O(n), never looped/backtracking.
pub(crate) fn frontmatter_bounds(raw: &str) -> Result<(usize, usize), &'static str> {
    let start = 4; // len("---\n")
    if !raw.starts_with("---\n") {
        return Err("missing opening --- frontmatter fence");
    }
    match raw[start..].find("\n---") {
        Some(rel) => Ok((start, start + rel)),
        None => Err("missing closing --- frontmatter fence"),
    }
}

/// Find the absolute byte offset of the start of the (unique, top-level —
/// zero-indent) line beginning with `key` inside `raw[region.0..region.1]`.
/// A single linear pass over the region's lines.
fn find_key_line_start(raw: &str, region: (usize, usize), key: &str) -> Option<usize> {
    let (start, end) = region;
    let mut offset = start;
    for line in raw[start..end].split_inclusive('\n') {
        if line.starts_with(key) {
            return Some(offset);
        }
        offset += line.len();
    }
    None
}

/// Extract `last_amended:`'s raw scalar text (the literal source text
/// between the outer double quotes) from within the frontmatter block.
/// Tolerates an already-defective (D-1144 unescaped-quote) value, since the
/// convention is "outer quotes are the first/last `"` on the physical line",
/// not "the first matched quote pair" — a `.find` + a single `.rfind` bounded
/// to that one line, never a repeated re-scan from index 0.
fn extract_last_amended(raw: &str, region: (usize, usize)) -> Option<String> {
    let key = "last_amended:";
    let line_start = find_key_line_start(raw, region, key)?;
    let (_, fm_end) = region;
    let after_key_start = line_start + key.len();
    let line_end = raw[after_key_start..fm_end]
        .find('\n')
        .map_or(fm_end, |p| after_key_start + p);
    let first_q_rel = raw[after_key_start..line_end].find('"')?;
    let value_start = after_key_start + first_q_rel + 1;
    let last_q_rel = raw[value_start..line_end].rfind('"')?;
    Some(raw[value_start..value_start + last_q_rel].to_string())
}

/// Split changelog sequence text into its individual raw item blocks. Each
/// item starts at a line beginning with the two-space list-item marker
/// (`"  - "`); every subsequent line up to (but excluding) the next such
/// marker line belongs to the same item. Single linear pass.
fn split_into_items(sequence_text: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut current = String::new();
    for line in sequence_text.split_inclusive('\n') {
        if line.starts_with("  - ") && !current.is_empty() {
            items.push(std::mem::take(&mut current));
        }
        current.push_str(line);
    }
    if !current.is_empty() {
        items.push(current);
    }
    items
}

/// Locate the byte range of the `changelog:` sequence's ITEM TEXT (the
/// region immediately after the `changelog:\n` key line, up to the first
/// subsequent zero-indent line or the end of the frontmatter region) —
/// shared by extraction (read) and rotation (rewrite).
pub(crate) fn changelog_sequence_bounds(raw: &str) -> Option<(usize, usize)> {
    let (fm_start, fm_end) = frontmatter_bounds(raw).ok()?;
    let mut offset = fm_start;
    let mut seq_start = None;
    for line in raw[fm_start..fm_end].split_inclusive('\n') {
        if line == "changelog:\n" || line == "changelog:" {
            seq_start = Some(offset + line.len());
            break;
        }
        offset += line.len();
    }
    let seq_start = seq_start?;
    let mut seq_end = fm_end;
    let mut off2 = seq_start;
    for line in raw[seq_start..fm_end].split_inclusive('\n') {
        let content = line.trim_end_matches('\n');
        if !content.is_empty() && !content.starts_with(' ') {
            seq_end = off2;
            break;
        }
        off2 += line.len();
    }
    Some((seq_start, seq_end))
}

/// Extract `(changelog_present, changelog_items_raw)`.
fn extract_changelog(raw: &str, region: (usize, usize)) -> (bool, Vec<String>) {
    let (fm_start, fm_end) = region;
    let has_key = find_key_line_start(raw, region, "changelog:").is_some();
    if !has_key {
        return (false, Vec::new());
    }
    match changelog_sequence_bounds(raw) {
        Some((seq_start, seq_end)) if seq_start >= fm_start && seq_end <= fm_end => {
            (true, split_into_items(&raw[seq_start..seq_end]))
        }
        _ => (true, Vec::new()),
    }
}

/// Parse `path`'s YAML frontmatter, isolating `last_amended` and
/// `changelog:`.
///
/// This is the shared primitive every PC-satisfying function in this crate is
/// built on — parsing on its own is not itself a BC-10.13.001 postcondition,
/// but Invariant 3 (bounded-resource safety on arbitrarily long input) binds
/// this function directly.
///
/// Every step here is a small, constant number of single linear (`O(n)`)
/// passes over the file content — never a repeated substring re-scan from
/// index 0, never a backtracking regex — so this stays bounded even against
/// the D-1149 323,499-char (and up to ~350K-char) calibration ceiling.
pub fn parse_frontmatter(path: &Path) -> Result<FrontmatterDoc, MigrateError> {
    let raw = std::fs::read_to_string(path).map_err(|source| MigrateError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    let region = frontmatter_bounds(&raw).map_err(|reason| MigrateError::FrontmatterParse {
        path: path.to_path_buf(),
        reason: reason.to_string(),
    })?;

    let last_amended_raw = extract_last_amended(&raw, region);
    let (changelog_present, changelog_items_raw) = extract_changelog(&raw, region);

    Ok(FrontmatterDoc {
        path: path.to_path_buf(),
        raw,
        last_amended_raw,
        changelog_present,
        changelog_items_raw,
    })
}

/// Overwrite `last_amended:`'s raw scalar value (the text strictly between
/// the outer quotes on its line) with `new_value`, leaving every other byte
/// of `doc.raw` untouched. Updates `doc.last_amended_raw` to match.
pub(crate) fn set_last_amended(
    doc: &mut FrontmatterDoc,
    new_value: &str,
) -> Result<(), MigrateError> {
    let region = frontmatter_bounds(&doc.raw).map_err(|reason| MigrateError::FrontmatterParse {
        path: doc.path.clone(),
        reason: reason.to_string(),
    })?;
    let key = "last_amended:";
    let line_start = find_key_line_start(&doc.raw, region, key).ok_or_else(|| {
        MigrateError::FrontmatterParse {
            path: doc.path.clone(),
            reason: "last_amended key not found while writing new value".to_string(),
        }
    })?;
    let (_, fm_end) = region;
    let after_key_start = line_start + key.len();
    let line_end = doc.raw[after_key_start..fm_end]
        .find('\n')
        .map_or(fm_end, |p| after_key_start + p);
    let first_q_rel = doc.raw[after_key_start..line_end]
        .find('"')
        .ok_or_else(|| MigrateError::FrontmatterParse {
            path: doc.path.clone(),
            reason: "last_amended value has no opening quote".to_string(),
        })?;
    let value_start = after_key_start + first_q_rel + 1;
    let last_q_rel = doc.raw[value_start..line_end].rfind('"').ok_or_else(|| {
        MigrateError::FrontmatterParse {
            path: doc.path.clone(),
            reason: "last_amended value has no closing quote".to_string(),
        }
    })?;
    let value_end = value_start + last_q_rel;
    doc.raw.replace_range(value_start..value_end, new_value);
    doc.last_amended_raw = Some(new_value.to_string());
    Ok(())
}
