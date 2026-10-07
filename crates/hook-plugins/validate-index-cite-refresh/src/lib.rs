//! validate-index-cite-refresh — PostToolUse WASM hook plugin.
//!
//! Blocks any Edit/Write to `ARCH-INDEX.md` that leaves a stale version
//! citation for any of the four canonical factory indexes
//! (BC-INDEX, VP-INDEX, STORY-INDEX, ARCH-INDEX). Also performs a
//! cross-cell sibling sweep (D-429(b)): reads STATE.md and the
//! brownfield-cycle INDEX.md and verifies that any 4-index version strings
//! they contain agree with the live index frontmatter values.
//!
//! # Behavioral Contracts
//!
//! - BC-5.39.003: blocks stale 4-index version cites + cross-cell mismatch.
//!
//! # D-NNN closures
//!
//! - D-405(c): ARCH-INDEX body cites of BC/VP/STORY/ARCH-INDEX checked.
//! - D-429(b): STATE.md + INDEX.md cross-cell version agreement checked.
//!
//! # Architecture compliance
//!
//! - HOST_ABI_VERSION = 1 (no new host functions introduced).
//! - Fail-open on every `host::read_file` error (BC-5.39.003 invariant 5).
//! - No `println!` — use `host::log_*` for all diagnostic output.
//! - No `unwrap()` or `expect()` in production paths.
//! - No `regex` crate: hand-rolled pattern scanning to stay within WASM fuel budget.

use std::collections::HashMap;
use vsdd_hook_sdk::{HookPayload, HookResult};

/// HOST_ABI_VERSION declares the ABI contract version this plugin was built
/// against. Must remain 1.
pub const HOST_ABI_VERSION: u32 = 1;

/// Maximum bytes to read from any factory artifact via `host::read_file`.
///
/// Set to 512 KiB (524288 bytes) — consistent with the cap used by
/// `validate-state-structure` (F-P5-002) and `validate-dispatch-advance`.
/// Production STATE.md is 95 KiB; this cap provides 5x growth runway.
/// The old 65536-byte cap caused `host::read_file` to return
/// `Err(HostError::OutputTooLarge)` (-3), silently disabling the
/// D-429(b) cross-cell version sweep against the real production STATE.md.
///
/// F-PASS15-001/F-PASS15-004: raise from 65536 to 524288.
pub const MAX_BYTES: u32 = 524_288;

// ---------------------------------------------------------------------------
// Index name enum
// ---------------------------------------------------------------------------

/// One of the four canonical factory index files tracked by this hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IndexName {
    BcIndex,
    VpIndex,
    StoryIndex,
    ArchIndex,
}

impl IndexName {
    /// Map the canonical name string from the document body to the enum variant.
    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "BC-INDEX" => Some(IndexName::BcIndex),
            "VP-INDEX" => Some(IndexName::VpIndex),
            "STORY-INDEX" => Some(IndexName::StoryIndex),
            "ARCH-INDEX" => Some(IndexName::ArchIndex),
            _ => None,
        }
    }

    /// Return the human-readable index name string.
    pub fn as_str(&self) -> &'static str {
        match self {
            IndexName::BcIndex => "BC-INDEX",
            IndexName::VpIndex => "VP-INDEX",
            IndexName::StoryIndex => "STORY-INDEX",
            IndexName::ArchIndex => "ARCH-INDEX",
        }
    }

    /// Return the canonical path (relative to project root) for this index file.
    pub fn canonical_path(&self) -> &'static str {
        match self {
            IndexName::BcIndex => ".factory/specs/behavioral-contracts/BC-INDEX.md",
            IndexName::VpIndex => ".factory/specs/verification-properties/VP-INDEX.md",
            IndexName::StoryIndex => ".factory/stories/STORY-INDEX.md",
            IndexName::ArchIndex => ".factory/specs/architecture/ARCH-INDEX.md",
        }
    }
}

/// All the canonical index name prefixes in match-priority order.
/// Longer names must come first to avoid "BC-INDEX" matching "BC-INDEX" in "BC-INDEX".
/// (STORY-INDEX is longest, so it's first.)
const INDEX_PREFIXES: &[(&str, IndexName)] = &[
    ("STORY-INDEX", IndexName::StoryIndex),
    ("ARCH-INDEX", IndexName::ArchIndex),
    ("BC-INDEX", IndexName::BcIndex),
    ("VP-INDEX", IndexName::VpIndex),
];

// ---------------------------------------------------------------------------
// Version cite data types
// ---------------------------------------------------------------------------

/// A parsed version string extracted from document body text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionCite {
    pub index_name: IndexName,
    pub major: u32,
    pub minor: u32,
    /// 1-based line number within the source document where this cite appears.
    pub line: u32,
    /// The raw major.minor literal as it appeared in the source document body
    /// (e.g. "1.05" for "v1.05", "3.28" for "v3.28").  Preserved so the block
    /// message can reproduce the body-literal form byte-for-byte, satisfying
    /// BC-5.39.003 EC-005 Ctrl-F UX requirement.
    pub cite_raw: String,
}

/// A stale-cite violation found during validation.
#[derive(Debug, Clone)]
pub struct Violation {
    /// Source document where the stale cite was found (e.g. "ARCH-INDEX.md",
    /// "STATE.md", "INDEX.md").
    pub source: String,
    /// Location within the source document where the stale cite appears.
    /// Uses 1-based line number form (e.g. "line 47") so authors can jump
    /// directly to the stale cite in the edited file.
    pub location: String,
    /// Index file name whose version was cited stale.
    pub index_name: IndexName,
    /// The version that was cited in the source document.
    pub cited: (u32, u32),
    /// The live version from the index file frontmatter.
    pub live: (u32, u32),
    /// The raw body-literal version string as it appeared in the source
    /// document (e.g. "1.05" for `BC-INDEX v1.05`, "3.28" for
    /// `STORY-INDEX v3.28`).  Used in block message so the cited side
    /// is byte-identical to the body literal — satisfying BC-5.39.003
    /// EC-005 Ctrl-F UX requirement.  The live side is always
    /// integer-rendered from `live: (u32, u32)`.
    pub cited_raw: String,
}

// ---------------------------------------------------------------------------
// Hand-rolled pattern scanning (avoids regex crate WASM fuel cost)
// ---------------------------------------------------------------------------

/// Try to parse a decimal u32 from the start of `s`, returning the parsed
/// value and the number of characters consumed.  Returns `None` if there
/// are no leading digits.
fn parse_leading_digits(s: &str) -> Option<(u32, usize)> {
    let bytes = s.as_bytes();
    let mut end = 0;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end == 0 {
        return None;
    }
    let num_str = &s[..end];
    // Reject obviously-overflowing strings (u32 max is 4294967295 = 10 digits).
    if end > 10 {
        return None;
    }
    let val: u32 = num_str.parse().ok()?;
    Some((val, end))
}

/// Scan `content` for all occurrences of patterns like
/// `BC-INDEX v12.34`, `STORY-INDEX v1.2` etc.
///
/// Uses hand-rolled scanning: locate each `INDEX v` token, parse the
/// digits, skip malformed tokens.  No regex crate — keeps WASM fuel
/// consumption within the 20M default budget (ADR-042 §Decision 1).
///
/// Each returned `VersionCite` carries the 1-based line number where the
/// cite appears, so callers can populate `Violation::location`.
///
/// # BC trace
/// BC-5.39.003 invariant 3 — extraction restricted to 4 canonical names.
pub fn extract_index_cites(content: &str) -> Vec<VersionCite> {
    let mut cites = Vec::new();
    let mut pos = 0usize;
    // Track current 1-based line number: count newlines up to pos.
    let mut current_line: u32 = 1;

    while pos < content.len() {
        // Try each canonical index prefix at the current position.
        let mut found = false;
        for &(prefix, index_name) in INDEX_PREFIXES {
            if content[pos..].starts_with(prefix) {
                let after_prefix = pos + prefix.len();
                // Expect " v" immediately after the prefix.
                if content[after_prefix..].starts_with(" v") {
                    let after_v = after_prefix + 2; // skip " v"
                    // Parse major digits.
                    if let Some((major, major_len)) = parse_leading_digits(&content[after_v..]) {
                        let after_major = after_v + major_len;
                        // Expect "." after major.
                        if content[after_major..].starts_with('.') {
                            let after_dot = after_major + 1;
                            // Parse minor digits.
                            if let Some((minor, minor_len)) =
                                parse_leading_digits(&content[after_dot..])
                            {
                                // Capture raw cite slice (e.g. "1.05" from "v1.05") to
                                // preserve body-literal form byte-for-byte.
                                let major_raw = &content[after_v..after_major];
                                let minor_raw = &content[after_dot..after_dot + minor_len];
                                cites.push(VersionCite {
                                    index_name,
                                    major,
                                    minor,
                                    line: current_line,
                                    cite_raw: format!("{major_raw}.{minor_raw}"),
                                });
                                let end = after_dot + minor_len;
                                // Count newlines in the consumed span to keep
                                // current_line accurate.
                                for b in content[pos..end].bytes() {
                                    if b == b'\n' {
                                        current_line += 1;
                                    }
                                }
                                pos = end;
                                found = true;
                                break;
                            }
                        }
                    }
                }
                // Partial match (prefix found but no valid " vN.N") — skip prefix length
                // to avoid infinite loop, then continue scanning.
                if !found {
                    for b in content[pos..pos + prefix.len()].bytes() {
                        if b == b'\n' {
                            current_line += 1;
                        }
                    }
                    pos += prefix.len();
                    found = true;
                    break;
                }
            }
        }
        if !found {
            // Count newlines at the current character before advancing.
            if content.as_bytes()[pos] == b'\n' {
                current_line += 1;
            }
            // Advance to the next UTF-8 character boundary to avoid
            // panicking when content contains multi-byte characters (e.g.
            // em-dashes in headings like "ARCH-INDEX — Architecture Index").
            // `str::is_char_boundary` returns true at pos==0 and pos==len
            // and at every start byte of a UTF-8 sequence.
            pos += 1;
            while pos < content.len() && !content.is_char_boundary(pos) {
                pos += 1;
            }
        }
    }

    cites
}

/// Parse `version: "M.N"` from the frontmatter region of an index file.
///
/// Scans the first 2048 bytes for `version: "` followed by digits, `.`,
/// digits, `"`.  Returns `None` on parse failure or absent field.
///
/// Hand-rolled to avoid regex crate WASM fuel cost.
///
/// # BC trace
/// BC-5.39.003 postcondition 5 — version comparison is NUMERIC.
pub fn parse_frontmatter_version(content: &str) -> Option<(u32, u32)> {
    let region = if content.len() > 2048 {
        &content[..2048]
    } else {
        content
    };

    let token = "version: \"";
    let start = region.find(token)?;
    let after_token = start + token.len();
    let rest = &region[after_token..];

    let (major, major_len) = parse_leading_digits(rest)?;
    let after_major = &rest[major_len..];
    if !after_major.starts_with('.') {
        return None;
    }
    let (minor, _minor_len) = parse_leading_digits(&after_major[1..])?;
    Some((major, minor))
}

/// Return `true` if `cited` version is strictly older than `live` version
/// (numeric less-than comparison).
///
/// Version equality (`cited == live`) is NOT stale.
/// Version where cited > live is NOT stale (impossible under normal operation).
///
/// # BC trace
/// BC-5.39.003 postcondition 5 — numeric comparison.
pub fn is_stale(cited: (u32, u32), live: (u32, u32)) -> bool {
    cited.0 < live.0 || (cited.0 == live.0 && cited.1 < live.1)
}

// ---------------------------------------------------------------------------
// Effectful functions (call host::read_file)
// ---------------------------------------------------------------------------

/// Read the live version of an index file from the host filesystem.
///
/// Maps `IndexName` to its canonical path and calls `host::read_file`.
/// On error, logs a warning and returns `None` — fail-open per
/// BC-5.39.003 invariant 5.
///
/// # BC trace
/// BC-5.39.003 postcondition 4 — read failure → Continue + log_warn.
pub fn read_live_version(index_name: IndexName) -> Option<(u32, u32)> {
    let path = index_name.canonical_path();
    match vsdd_hook_sdk::host::read_file(path, MAX_BYTES, 2000) {
        Ok(bytes) => {
            let content = match String::from_utf8(bytes) {
                Ok(s) => s,
                Err(e) => {
                    vsdd_hook_sdk::host::log_warn(&format!(
                        "[validate-index-cite-refresh] UTF-8 decode failure reading {path}: {e}"
                    ));
                    return None;
                }
            };
            match parse_frontmatter_version(&content) {
                Some(v) => Some(v),
                None => {
                    vsdd_hook_sdk::host::log_warn(&format!(
                        "[validate-index-cite-refresh] could not parse version: from frontmatter of {path}"
                    ));
                    None
                }
            }
        }
        Err(e) => {
            vsdd_hook_sdk::host::log_warn(&format!(
                "[validate-index-cite-refresh] read_file failed for {path}: {e:?}"
            ));
            None
        }
    }
}

/// Build a map of live versions for all four canonical indexes.
///
/// For each index, calls `read_live_version`. Missing/unreadable indexes are
/// simply absent from the returned map (fail-open — callers skip them).
pub fn build_live_version_map() -> HashMap<IndexName, (u32, u32)> {
    let mut map = HashMap::new();
    for &name in &[
        IndexName::BcIndex,
        IndexName::VpIndex,
        IndexName::StoryIndex,
        IndexName::ArchIndex,
    ] {
        if let Some(v) = read_live_version(name) {
            map.insert(name, v);
        }
    }
    map
}

/// Perform the D-429(b) cross-cell sibling sweep.
///
/// Reads STATE.md and INDEX.md. Extracts all 4-index version strings from
/// each file. Compares each against the live versions in `live_versions`.
/// Returns a list of violations.
///
/// On read errors, logs a warning and skips that file (fail-open per
/// BC-5.39.003 invariant 5 + EC-009).
///
/// # BC trace
/// BC-5.39.003 postcondition 3 — cross-cell disagreement → BlockWithFix.
pub fn cross_cell_check(live_versions: &HashMap<IndexName, (u32, u32)>) -> Vec<Violation> {
    let mut violations = Vec::new();

    // STATE.md cross-cell check
    let state_path = ".factory/STATE.md";
    match vsdd_hook_sdk::host::read_file(state_path, MAX_BYTES, 2000) {
        Ok(bytes) => {
            if let Ok(content) = String::from_utf8(bytes) {
                for cite in extract_index_cites(&content) {
                    if let Some(&live) = live_versions.get(&cite.index_name) {
                        let cited = (cite.major, cite.minor);
                        if is_stale(cited, live) {
                            violations.push(Violation {
                                source: "STATE.md".to_string(),
                                location: format!("line {}", cite.line),
                                index_name: cite.index_name,
                                cited,
                                live,
                                cited_raw: cite.cite_raw.clone(),
                            });
                        }
                    }
                }
            } else {
                vsdd_hook_sdk::host::log_warn(
                    "[validate-index-cite-refresh] UTF-8 decode failure reading STATE.md",
                );
            }
        }
        Err(e) => {
            vsdd_hook_sdk::host::log_warn(&format!(
                "[validate-index-cite-refresh] read_file failed for {state_path}: {e:?}"
            ));
        }
    }

    // INDEX.md cross-cell check (D-429(b); path hardcoded for brownfield cycle)
    // NOTE: This path is hardcoded to the brownfield cycle currently in-flight.
    // When the cycle rotates, this path must be updated. Tracked in S-15.07 spec §Risk.
    let index_path = ".factory/cycles/v1.0-brownfield-backfill/INDEX.md";
    match vsdd_hook_sdk::host::read_file(index_path, MAX_BYTES, 2000) {
        Ok(bytes) => {
            if let Ok(content) = String::from_utf8(bytes) {
                for cite in extract_index_cites(&content) {
                    if let Some(&live) = live_versions.get(&cite.index_name) {
                        let cited = (cite.major, cite.minor);
                        if is_stale(cited, live) {
                            violations.push(Violation {
                                source: "INDEX.md".to_string(),
                                location: format!("line {}", cite.line),
                                index_name: cite.index_name,
                                cited,
                                live,
                                cited_raw: cite.cite_raw.clone(),
                            });
                        }
                    }
                }
            } else {
                vsdd_hook_sdk::host::log_warn(
                    "[validate-index-cite-refresh] UTF-8 decode failure reading INDEX.md",
                );
            }
        }
        Err(e) => {
            vsdd_hook_sdk::host::log_warn(&format!(
                "[validate-index-cite-refresh] read_file failed for {index_path}: {e:?}"
            ));
        }
    }

    violations
}

// ---------------------------------------------------------------------------
// Block message formatting
// ---------------------------------------------------------------------------

/// Format a single violation into a human-readable line.
///
/// Format matches spec §Block message format exactly:
///   [source] location cites INDEX vM.N but live version is vM.N. Update cite to vM.N.
/// `v.location` carries the 1-based line number (e.g. "line 47") so authors
/// can jump directly to the stale cite in the edited file.
///
/// The cited side uses `v.cited_raw` (body-literal form, e.g. "v1.05") so the
/// block message is byte-identical to what the author wrote.  The live side
/// is always integer-rendered from `v.live` (canonical form, e.g. "v2.24").
/// BC-5.39.003 EC-005 Ctrl-F UX requirement.
fn format_violation(v: &Violation) -> String {
    format!(
        "  [{}] {} cites {} v{} but live version is v{}.{}. Update cite to v{}.{}.",
        v.source,
        v.location,
        v.index_name.as_str(),
        v.cited_raw,
        v.live.0,
        v.live.1,
        v.live.0,
        v.live.1,
    )
}

// ---------------------------------------------------------------------------
// File-path guard
// ---------------------------------------------------------------------------

/// Returns `true` if `file_path` names a file whose `file_name` component is
/// exactly `ARCH-INDEX.md`.
///
/// Uses path-component-strict matching (`Path::file_name`) rather than
/// `ends_with`, preventing false-positive fires on paths whose suffix happens
/// to match but whose file name is not `ARCH-INDEX.md`.
///
/// Returns `false` if the path has no file-name component (e.g. `/`).
///
/// # BC trace
/// BC-5.39.003 invariant 1 — hook only activates on ARCH-INDEX.md writes.
pub fn is_arch_index_target(file_path: &str) -> bool {
    std::path::Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        == Some("ARCH-INDEX.md")
}

// ---------------------------------------------------------------------------
// Hook entry point (wired to real host fns in main.rs)
// ---------------------------------------------------------------------------

/// Core hook logic for validate-index-cite-refresh.
///
/// Called from the WASI entry point in `main.rs` via the SDK trampoline.
///
/// 1. Extracts `file_path` from `tool_input`; early-exit Continue for
///    non-ARCH-INDEX.md paths (belt-and-suspenders guard — the dispatcher
///    routes by event+tool, so any PostToolUse Edit|Write reaches this hook).
/// 2. Reads the written ARCH-INDEX.md content from the host filesystem.
/// 3. Extracts all 4-index version cites from ARCH-INDEX body.
/// 4. Reads live versions from the four canonical index frontmatters.
/// 5. Checks each ARCH-INDEX cite against the live version.
/// 6. Performs D-429(b) cross-cell sweep against STATE.md and INDEX.md.
/// 7. If any violations found, emits `HookResult::block_with_fix`.
///
/// # BC trace
/// BC-5.39.003 postconditions 1-4; invariants 1-5.
pub fn on_post_tool_use(payload: HookPayload) -> HookResult {
    use vsdd_hook_sdk::host;

    // Extract file_path from tool_input.
    let file_path = match payload.tool_input.get("file_path").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => {
            host::log_warn(
                "[validate-index-cite-refresh] file_path absent from tool_input — graceful degrade",
            );
            return HookResult::Continue;
        }
    };

    // Only act on writes to ARCH-INDEX.md.
    // The dispatcher routes by event+tool (PostToolUse + Edit|Write) but not by
    // file path — this guard applies the ARCH-INDEX.md file-path filter.
    // Uses path-component-strict matching to avoid false-positives from
    // suffix-only ends_with checks.
    if !is_arch_index_target(&file_path) {
        return HookResult::Continue;
    }

    // Read ARCH-INDEX.md content that was just written.
    let arch_content = match host::read_file(
        ".factory/specs/architecture/ARCH-INDEX.md",
        131072, // 128 KB cap — generous for ARCH-INDEX growth per §Risk
        2000,
    ) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(s) => s,
            Err(e) => {
                host::log_warn(&format!(
                    "[validate-index-cite-refresh] UTF-8 decode failure reading ARCH-INDEX.md: {e}"
                ));
                return HookResult::Continue;
            }
        },
        Err(e) => {
            host::log_warn(&format!(
                "[validate-index-cite-refresh] read_file failed for ARCH-INDEX.md: {e:?}"
            ));
            return HookResult::Continue;
        }
    };

    // Build live version map for all four indexes.
    let live_versions = build_live_version_map();

    // Extract all 4-index version cites from ARCH-INDEX body and check them.
    let arch_cites = extract_index_cites(&arch_content);
    let mut violations: Vec<Violation> = Vec::new();

    for cite in &arch_cites {
        match live_versions.get(&cite.index_name) {
            Some(&live) => {
                let cited = (cite.major, cite.minor);
                if is_stale(cited, live) {
                    violations.push(Violation {
                        source: "ARCH-INDEX.md".to_string(),
                        location: format!("line {}", cite.line),
                        index_name: cite.index_name,
                        cited,
                        live,
                        cited_raw: cite.cite_raw.clone(),
                    });
                }
            }
            None => {
                // Index file was unreadable — fail-open per BC-5.39.003 invariant 5.
                // read_live_version already logged a warning.
            }
        }
    }

    // D-429(b) cross-cell sweep.
    let cross_violations = cross_cell_check(&live_versions);
    violations.extend(cross_violations);

    if violations.is_empty() {
        HookResult::Continue
    } else {
        emit_block(&violations)
    }
}

/// Format all violations into a single `HookResult::block_with_fix`.
fn emit_block(violations: &[Violation]) -> HookResult {
    let lines: Vec<String> = violations.iter().map(format_violation).collect();
    let reason = format!(
        "validate-index-cite-refresh: {} stale cite(s) found:\n{}",
        violations.len(),
        lines.join("\n")
    );
    HookResult::block_with_fix(
        "validate-index-cite-refresh",
        reason,
        "Update the stale version cite(s) listed above to the current index versions",
        "STALE_INDEX_VERSION_CITE",
    )
}

// ---------------------------------------------------------------------------
// Unit tests — BC-5.39.003
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    // ── extract_index_cites ──────────────────────────────────────────────────

    #[test]
    fn test_BC_5_39_003_extract_all_four_indexes() {
        let content = "- BC-INDEX v2.24\n- VP-INDEX v1.97\n- STORY-INDEX v3.31\n- ARCH-INDEX v2.18";
        let cites = extract_index_cites(content);
        assert_eq!(cites.len(), 4);
        assert!(
            cites
                .iter()
                .any(|c| c.index_name == IndexName::BcIndex && c.major == 2 && c.minor == 24)
        );
        assert!(
            cites
                .iter()
                .any(|c| c.index_name == IndexName::VpIndex && c.major == 1 && c.minor == 97)
        );
        assert!(
            cites
                .iter()
                .any(|c| c.index_name == IndexName::StoryIndex && c.major == 3 && c.minor == 31)
        );
        assert!(
            cites
                .iter()
                .any(|c| c.index_name == IndexName::ArchIndex && c.major == 2 && c.minor == 18)
        );
    }

    #[test]
    fn test_BC_5_39_003_extract_empty_content_returns_empty() {
        let cites = extract_index_cites("");
        assert!(cites.is_empty());
    }

    #[test]
    fn test_BC_5_39_003_extract_no_cite_strings_returns_empty() {
        let content = "No version cites in this document body.";
        let cites = extract_index_cites(content);
        assert!(cites.is_empty());
    }

    #[test]
    fn test_BC_5_39_003_extract_ignores_malformed_version_letters() {
        // "BC-INDEX vX.Y" — no digits after "v", so no match.
        let content = "BC-INDEX vX.Y is not valid";
        let cites = extract_index_cites(content);
        assert!(cites.is_empty());
    }

    #[test]
    fn test_BC_5_39_003_extract_handles_multibyte_chars_no_panic() {
        // Content with em-dashes (U+2014, 3 bytes) mixed with a valid cite.
        // Regression guard for the pos+=1 panic when pos lands inside a
        // multi-byte sequence (e.g. "ARCH-INDEX — Architecture Index").
        let content = "# ARCH-INDEX \u{2014} Architecture Index\n\n- BC-INDEX v2.24\n";
        let cites = extract_index_cites(content);
        assert_eq!(cites.len(), 1);
        assert_eq!(cites[0].index_name, IndexName::BcIndex);
        assert_eq!(cites[0].major, 2);
        assert_eq!(cites[0].minor, 24);
    }

    #[test]
    fn test_BC_5_39_003_extract_story_index_not_confused_with_arch_index() {
        let content = "STORY-INDEX v3.31 and ARCH-INDEX v2.18";
        let cites = extract_index_cites(content);
        assert_eq!(cites.len(), 2);
        assert!(
            cites
                .iter()
                .any(|c| c.index_name == IndexName::StoryIndex && c.major == 3 && c.minor == 31)
        );
        assert!(
            cites
                .iter()
                .any(|c| c.index_name == IndexName::ArchIndex && c.major == 2 && c.minor == 18)
        );
    }

    #[test]
    fn test_BC_5_39_003_extract_stale_bc_index_cite() {
        let content = "BC-INDEX v1.05";
        let cites = extract_index_cites(content);
        assert_eq!(cites.len(), 1);
        assert_eq!(cites[0].index_name, IndexName::BcIndex);
        assert_eq!(cites[0].major, 1);
        assert_eq!(cites[0].minor, 5);
    }

    // ── parse_frontmatter_version ────────────────────────────────────────────

    #[test]
    fn test_BC_5_39_003_parse_version_standard_frontmatter() {
        let content = "---\ndocument_type: index\nversion: \"2.24\"\ntitle: \"BC-INDEX\"\n---\n";
        assert_eq!(parse_frontmatter_version(content), Some((2, 24)));
    }

    #[test]
    fn test_BC_5_39_003_parse_version_absent_returns_none() {
        let content = "---\ndocument_type: index\ntitle: \"BC-INDEX\"\n---\n";
        assert_eq!(parse_frontmatter_version(content), None);
    }

    #[test]
    fn test_BC_5_39_003_parse_version_numeric_minor_greater_than_9() {
        // v3.31 — minor is 31; numeric parse must not treat as "3" truncated.
        let content = "---\nversion: \"3.31\"\n---\n";
        assert_eq!(parse_frontmatter_version(content), Some((3, 31)));
    }

    // ── is_stale ─────────────────────────────────────────────────────────────

    #[test]
    fn test_BC_5_39_003_is_stale_older_major_is_stale() {
        assert!(is_stale((1, 5), (2, 0)));
    }

    #[test]
    fn test_BC_5_39_003_is_stale_same_major_older_minor_is_stale() {
        assert!(is_stale((1, 5), (1, 10)));
    }

    #[test]
    fn test_BC_5_39_003_is_stale_equal_versions_not_stale() {
        assert!(!is_stale((2, 24), (2, 24)));
    }

    #[test]
    fn test_BC_5_39_003_is_stale_newer_cited_not_stale() {
        // EC-003: cited > live is NOT stale.
        assert!(!is_stale((3, 0), (2, 24)));
    }

    #[test]
    fn test_BC_5_39_003_is_stale_major_boundary() {
        // cited.0 == live.0, cited.1 < live.1 → stale
        assert!(is_stale((2, 23), (2, 24)));
        // cited.0 == live.0, cited.1 == live.1 → not stale
        assert!(!is_stale((2, 24), (2, 24)));
        // cited.0 < live.0 → stale regardless of minor
        assert!(is_stale((1, 99), (2, 0)));
    }

    // ── emit_block ───────────────────────────────────────────────────────────

    #[test]
    fn test_BC_5_39_003_emit_block_names_index_and_versions() {
        let violations = vec![Violation {
            source: "ARCH-INDEX.md".to_string(),
            location: "line 47".to_string(),
            index_name: IndexName::BcIndex,
            cited: (1, 5),
            live: (2, 24),
            // Body-literal form: "1.05" — preserved via cited_raw so the block
            // message is byte-identical to what the author wrote.
            // BC-5.39.003 EC-005 Ctrl-F UX requirement.
            cited_raw: "1.05".to_string(),
        }];
        let result = emit_block(&violations);
        match &result {
            HookResult::Block { reason } => {
                assert!(reason.contains("BC-INDEX"), "reason must name BC-INDEX");
                // Body-literal form: fixture cites "BC-INDEX v1.05"; block message
                // must reproduce "v1.05" byte-for-byte (F-P2-001: cited_raw plumbing).
                assert!(
                    reason.contains("1.05"),
                    "reason must name cited version as body-literal v1.05 (not stripped v1.5)"
                );
                assert!(reason.contains("2.24"), "reason must name live version");
                assert!(
                    reason.contains("line 47"),
                    "reason must include cite location"
                );
            }
            _ => panic!("expected Block result, got {result:?}"),
        }
    }

    #[test]
    fn test_BC_5_39_003_emit_block_exit_code_2() {
        let violations = vec![Violation {
            source: "ARCH-INDEX.md".to_string(),
            location: "line 1".to_string(),
            index_name: IndexName::BcIndex,
            cited: (1, 5),
            live: (2, 24),
            cited_raw: "1.5".to_string(),
        }];
        let result = emit_block(&violations);
        assert_eq!(result.exit_code(), 2);
    }

    // ── is_arch_index_target ─────────────────────────────────────────────────

    #[test]
    fn test_BC_5_39_003_file_path_suffix_arch_index_md_does_not_match() {
        // O-P1-002 (sibling): suffix-only match is a false-positive — any path
        // whose file name component is not exactly ARCH-INDEX.md must be rejected.
        assert!(!is_arch_index_target("/some/dir/XARCH-INDEX.md"));
        assert!(!is_arch_index_target("not-ARCH-INDEX.md"));
    }

    #[test]
    fn test_BC_5_39_003_file_path_canonical_arch_index_md_matches() {
        // Canonical path at any directory depth must match.
        assert!(is_arch_index_target(
            ".factory/specs/architecture/ARCH-INDEX.md"
        ));
        assert!(is_arch_index_target("ARCH-INDEX.md"));
        assert!(is_arch_index_target("/absolute/path/to/ARCH-INDEX.md"));
    }

    // ── F-PASS15-001/F-PASS15-004: MAX_BYTES cap regression test ────────────

    /// F-PASS15-001/F-PASS15-004: verifies that MAX_BYTES is set to at least
    /// 524288 (512 KiB), consistent with the sibling cap used by
    /// validate-state-structure (F-P5-002) and validate-dispatch-advance.
    ///
    /// If someone lowers MAX_BYTES below 512 KiB, this compile-time assertion
    /// fails. This prevents silent regression where the D-429(b) cross-cell
    /// sweep becomes functionally dead against production-sized STATE.md files
    /// (95 KiB as of pass-15, growing with each adversarial pass).
    #[test]
    fn test_BC_5_39_003_max_bytes_cap_at_least_512_kib() {
        // Compile-time assertion: MAX_BYTES must be >= 524288 (512 KiB).
        // This is the load-bearing constant check that closes F-PASS15-001/F-PASS15-004.
        // If someone lowers the cap below 512 KiB, this line fails to compile.
        const _: () = assert!(
            MAX_BYTES >= 524_288,
            "MAX_BYTES must be >= 524288 (512 KiB) per F-PASS15-001/F-PASS15-004"
        );
    }
}
