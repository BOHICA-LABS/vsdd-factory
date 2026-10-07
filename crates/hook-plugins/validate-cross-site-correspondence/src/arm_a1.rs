//! arm_a1.rs — Class A Arm1: BC frontmatter version vs BC-INDEX.md row version.
//!
//! Effectful-shell module (ADR-035 §Decision 1): reads BC-INDEX.md via
//! `host::read_file` as a secondary read.
//!
//! # Behavior
//! Fires when a BC file is written. Reads BC-INDEX.md and compares the version
//! cell in the table row for this BC's ID against the BC frontmatter `version:`
//! field. Blocks if the INDEX row is stale (present but different), advisory-only
//! if the BC is new (v1.0 and not yet in the INDEX).
//!
//! # Fail-closed (BC-5.39.008 v1.6 / BC-5.39.010 invariant 4/5)
//! - `CapabilityDenied` on BC-INDEX.md → Block (sandbox misconfiguration).
//! - `NotFound` on BC-INDEX.md → Advisory + Continue (bootstrap ordering).
//! - Primary target (BC file) read failure → Block always (invariant 4).
//!   Primary target failure is handled in `lib.rs` before dispatching here.
//!
//! # BC trace
//! BC-5.39.010 Class A Arm1 preconditions, postconditions, and invariants (current
//! version per BC-INDEX) — including precondition 11 (`derive_bc_path`), §PC5 row
//! classification, the PC2a/PC2b directional version carve-out, preconditions 7/8
//! read-failure semantics, invariants 4/5, and the v1.22 secondary-index UTF-8
//! decode-failure path (precondition 15b / postcondition 26, `IndexUnreadable`;
//! ADV-RECON11-001).

use crate::{Advisory, Violation};
use vsdd_hook_sdk::host::HostError;

/// Read cap for BC-INDEX.md secondary read.
/// BC-5.39.010 AC-019 precondition: `max_bytes = 1048576`, `timeout_ms = 3000`.
pub const BC_INDEX_MAX_BYTES: u32 = 1_048_576;
pub const BC_INDEX_TIMEOUT_MS: u32 = 3_000;

/// Read cap for BC files (primary target and secondary BC reads in Arm A2).
/// BC-5.39.010 AC-019 precondition: `max_bytes = 524288`, `timeout_ms = 3000`.
pub const BC_MAX_BYTES: u32 = 524_288;
pub const BC_TIMEOUT_MS: u32 = 3_000;

/// Derive a deterministic BC file path from a BC ID string.
///
/// Algorithm (BC-5.39.010 precondition 11 `derive_bc_path`):
///   `BC-<section>.<subsection>.<seq>` → `.factory/specs/behavioral-contracts/ss-<section_zero_padded>/<BC-ID>.md`
///
/// Example: `BC-5.39.010` → `.factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md`
///
/// No `read_dir`, no `glob`, no filesystem enumeration — deterministic derivation only.
///
/// # BC trace
/// BC-5.39.010 precondition 11; Architecture Compliance Rule (derive_bc_path).
pub fn derive_bc_path(bc_id: &str) -> String {
    // Algorithm: BC-<section>.<subsection>.<seq>
    // → .factory/specs/behavioral-contracts/ss-<section_zero_padded>/<BC-ID>.md
    // Example: BC-5.39.010 → .factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md
    //          BC-6.26.001 → .factory/specs/behavioral-contracts/ss-06/BC-6.26.001.md

    // Parse the section number from "BC-<section>.<rest>"
    let section_str = bc_id
        .strip_prefix("BC-")
        .and_then(|s| s.split('.').next())
        .unwrap_or("00");

    // Zero-pad section number to 2 digits
    let section_num: u32 = section_str.parse().unwrap_or(0);
    let section_padded = format!("{section_num:02}");

    format!(".factory/specs/behavioral-contracts/ss-{section_padded}/{bc_id}.md")
}

/// Five-variant result for BC-INDEX.md row classification: four post-decode
/// row-classification states defined by §PC5 (`RowAbsent`, `RowPresentNoVersion`,
/// `Version`, `RowMalformed` — see below), plus one pre-decode indeterminate state,
/// `IndexUnreadable` (§PC15b / postcondition 26, v1.22), returned when BC-INDEX.md's
/// bytes fail UTF-8 decoding before the §PC5 row scan can even run. `IndexUnreadable`
/// is not a fifth row-classification outcome of the §PC5 scan itself — it is a
/// distinct, earlier-stage decode-failure signal (see its own variant doc below).
///
/// BC-5.39.010 §PC5 (column-count-anchored, four-state post-decode scan, full-file scan):
/// - `RowAbsent`: NO candidate line found at all for this BC ID. A candidate line must
///   satisfy the normative recognition predicate conditions (1)+(2): (1) starts with `|`;
///   (2) first non-empty field is `[bc_id]` link form or `bc_id` plain form. If no line
///   satisfies (1)+(2), the result is `RowAbsent`. `RowAbsent` means EXCLUSIVELY "no
///   candidate line found" — NOT "found but wrong shape."
/// - `RowPresentNoVersion`: candidate found AND (a) non-empty field count is exactly 5
///   (the canonical 5-column shape `| BC ID | Title | Status | Capability | Stories |`,
///   present on ~1,943 of 1,983 rows), OR (b) non-empty field count is ≥6 AND the 6th
///   non-empty field contains no `\bv([0-9]+\.[0-9]+)\b` token — the version-chain cell
///   exists structurally but carries no parseable version (F-P6-018 normative addition).
///   Both forms produce the same Continue outcome. An empty 6th column `| |` is filtered
///   by the non-empty predicate and counts as exactly 5 fields (field 6 unread).
///   Silent-continue in all `RowPresentNoVersion` cases.
/// - `Version(v)`: candidate found AND ≥6 non-empty fields after escape-aware split;
///   `fields[5..]` are rejoined with `\|` and re-split on the escape sentinel to recover
///   the version-chain cell, then the FIRST `\bv([0-9]+\.[0-9]+)\b` token of the LAST
///   non-empty chain entry is extracted (first-token-of-last-chain-entry; ADR-038
///   §Decision 1 — rightmost-of-field[5] is NON-CONFORMING).
/// - `RowMalformed(n)`: a candidate line WAS found (conditions (1)+(2) hold) but after
///   escape-aware split has fewer than 5 non-empty fields — NOT a valid body-table row
///   (likely a Changelog entry, subsystem-section row, or notes table carrying the BC ID
///   link). Disposition: advisory + Continue (postcondition 4a). NEVER reaches the blocking
///   path. Distinct from `RowAbsent`: a candidate line WAS found; the found-but-malformed
///   case MUST NOT be collapsed into `RowAbsent` (which would trigger false BLOCKs).
///   `n` is the observed non-empty field count (included in the advisory message).
///   Corpus count (2026-08-04): 0 RowMalformed lines in real BC-INDEX — all 1,983
///   BC-ID-candidate lines have ≥5 fields. This state is forward-looking protection.
///
/// # BC trace
/// BC-5.39.010 §PC5: four-state post-decode classification with full-file scan
/// (F-S2107-P4-005). Overall enum is five-variant as of v1.22 — see the
/// `IndexUnreadable` variant doc above for the pre-decode fifth state.
/// F-P6-018: ≥6-field/no-v-token state classified as `RowPresentNoVersion` (normative).
/// F-S2107-P3-001 BLOCKER: two-state `Option<String>` conflated RowAbsent with
/// RowPresentNoVersion — every 5-column row triggered a spurious block for v>1.0 BCs.
/// v1.9 resolved Conflict 2: found-but-<5-fields → RowMalformed (advisory); only
/// "no candidate line at all" → RowAbsent (potential block for v>1.0 BCs).
/// v1.10 resolved first-match-wins (F-S2107-P4-005): full-file scan preferred; RowMalformed
/// only when ALL locator-matched lines fail condition (3).
/// v1.22 (ADV-RECON11-001) added the pre-decode `IndexUnreadable` state, described above.
#[derive(Debug, PartialEq)]
pub enum BcIndexVersionState {
    /// No candidate line found at all for this BC ID in BC-INDEX.md.
    /// Exclusively means "no line matching the locator pattern" — not "found but wrong shape."
    RowAbsent,
    /// Candidate found; exactly 5 non-empty fields (canonical shape) — no version-chain cell.
    RowPresentNoVersion,
    /// Candidate found; ≥6 non-empty fields — version extracted from the last chain entry
    /// of the version-chain cell (first-token-of-last-chain-entry; ADR-038 §Decision 1).
    Version(String),
    /// Candidate found but <5 non-empty fields after escape-aware split — not a body-table row.
    /// Advisory + Continue (postcondition 4a). NEVER blocks. `usize` = observed field count.
    RowMalformed(usize),
    /// BC-INDEX.md was read successfully as bytes but the bytes failed UTF-8 decoding —
    /// the row-location scan cannot run against undecodable bytes, so the row state for
    /// this BC ID is genuinely INDETERMINATE (not confirmed-absent). Distinct from
    /// `RowAbsent`: `RowAbsent` means the file WAS decodable and scanned but no candidate
    /// line was found; `IndexUnreadable` means the file could not be scanned at all.
    /// MUST NOT be collapsed into `RowAbsent` — for a BC with frontmatter `version:` >
    /// "1.0", that would trigger postcondition 4's BLOCK with a MISLEADING "dropped
    /// registration" message when the true root cause is index-file corruption.
    /// Disposition: distinct advisory naming BC-INDEX.md + Continue (postcondition 26).
    /// NEVER blocks.
    ///
    /// # BC trace
    /// BC-5.39.010 precondition 15b / postcondition 26 (v1.22 / ADV-RECON11-001).
    IndexUnreadable,
}

/// Extract the BC-INDEX.md row state for `bc_id`. As of v1.22, this is a two-stage
/// classification: a v1.22 pre-decode check (UTF-8 decode failure → `IndexUnreadable`,
/// see below) followed by the v1.13 four-state post-decode §PC5 scan (`RowAbsent`,
/// `RowPresentNoVersion`, `Version`, `RowMalformed`) described next.
///
/// **Algorithm (BC-5.39.010 §PC5 — column-count-anchored, full-file scan):**
///
/// Scans ALL lines in `index_content`. For each line:
/// 1. **Condition (1):** line starts with `|` — skips YAML frontmatter, prose, blank lines.
/// 2. **Condition (2):** first non-empty pipe-cell matches the normative locator pattern —
///    link form `[bc_id](...)` or plain form `bc_id` exactly (see `first_cell_matches_bc_id`).
///    This is the recognition predicate. If neither form matches, skip the line.
/// 3. **Condition (3):** If (1)+(2) both hold, this is a CANDIDATE line. Apply
///    escape-aware split (replace `\|` → `\x00`, split on `|`, count non-empty trimmed fields):
///    - Exactly 5 fields → return `RowPresentNoVersion` immediately
///    - ≥6 fields → join `fields[5..]` with `\|`, split on `\x00`, take last non-empty
///      entry, extract first `\bv([0-9]+\.[0-9]+)\b` token (F-P6-019b/019c; rightmost-of-field[5] NON-CONFORMING per ADR-038 §Decision 1):
///      - Token found → return `Version(v)`
///      - No token found → return `RowPresentNoVersion` (F-P6-018 normative: ≥6-field/no-v-token)
///    - <5 fields → record as malformed candidate; **continue scanning for a valid line**
///
/// **F-P4-005 full-file selection order (BC-5.39.010 v1.19):**
/// Return the FIRST (1)+(2)+(3)-satisfying line (≥5 fields). Return `RowMalformed(n)` ONLY
/// when ALL locator-matched lines fail condition (3). First-match-wins on malformed lines
/// is NON-CONFORMING — a malformed line earlier in the file MUST NOT shadow a valid row later.
///
/// After full scan:
/// - If a malformed candidate was found (but no valid one): `RowMalformed(n)` (first malformed count)
/// - If no candidate at all: `RowAbsent`
///
/// Note: `RowAbsent` exclusively means "no candidate line found at all" — it does NOT cover
/// found-but-malformed cases (those are `RowMalformed`). This distinction is critical:
/// collapsing `RowMalformed` into `RowAbsent` would trigger false BLOCKs via postcondition 4.
///
/// Pure: operates on already-read bytes.
///
/// # BC trace
/// BC-5.39.010 §PC5: full-file-scan selection order (F-S2107-P4-005).
/// F-P6-018: ≥6-field/no-v-token → `RowPresentNoVersion` (normative).
/// F-S2107-P2-002: first-cell anchor (cross-reference rows must not match).
/// F-P6-019b/019c: first-token-of-last-entry extraction (replaces F-S2107-P1B-006 last-wins).
/// F-P6-019d: join fields[5..] to recover cell content fragmented by bare `|` characters.
/// F-S2107-P1B-007: starts_with('|') to skip YAML frontmatter lines.
/// F-S2107-P3-001 Conflict 2 resolution: <5 fields → RowMalformed (advisory), not RowAbsent.
pub(crate) fn extract_bc_index_version_state(
    bc_id: &str,
    index_content: &[u8],
) -> BcIndexVersionState {
    // BC-5.39.010 precondition 15b / postcondition 26 (v1.22 / ADV-RECON11-001):
    // BC-INDEX.md is a SECONDARY read target. A decode failure here is genuinely
    // INDETERMINATE, not a confirmed-absent row — it MUST NOT silently degrade to
    // "" (zero candidate lines → RowAbsent), which would misclassify index-file
    // corruption as a dropped registration. Signal the distinct disposition instead.
    let content = match std::str::from_utf8(index_content) {
        Ok(c) => c,
        Err(_) => return BcIndexVersionState::IndexUnreadable,
    };

    // Track the first malformed candidate's field count. Used as the RowMalformed(n) value
    // only when the full-file scan finds NO valid (≥5-field) candidate line.
    let mut first_malformed_count: Option<usize> = None;

    for line in content.lines() {
        // Condition (1): skip non-pipe-table lines (YAML frontmatter, prose, blank lines).
        // F-S2107-P1B-007: starts_with('|') is necessary; YAML frontmatter can contain
        // lines referencing BC IDs that are not body-table rows.
        if !line.starts_with('|') {
            continue;
        }
        // Escape-aware split: replace literal `\|` with null-byte before splitting.
        // Applied here (before condition (2)) so that both the first-cell extraction
        // (condition 2) and the field count (condition 3) use the same escaped view.
        // F-P4-017: first-cell extraction must use the escape-aware split to conform
        // to PC5's "first non-empty pipe-cell" normative predicate — using splitn(3,'|')
        // on the raw line was non-conforming when `\|` appears before the BC ID cell.
        let escaped = line.replace("\\|", "\x00");
        let non_empty_fields: Vec<&str> = escaped
            .split('|')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();

        // Condition (2): normative recognition predicate — first non-empty pipe-cell must
        // match the BC ID in either link form `[bc_id](...)` or plain form `bc_id`.
        // BC-5.39.010 §PC5: "first non-empty field" (escape-aware).
        // F-P2-002: anchor on the first pipe-cell only; cross-reference rows that cite
        // bc_id in Title or Depends columns must not be matched.
        //
        // Routed through `escape_aware_first_field` (ADV-RECON3-007) rather than
        // `non_empty_fields.first()` so this candidacy check and Arm2's
        // `row_first_field_matches_bc_id` (§PC13 Phase 1) share the identical
        // escape-aware first-cell computation, not two independently-maintained
        // copies (TD-VSDD-060). `non_empty_fields` below is still used for condition
        // (3)'s field count and the version-cell join — only the first-cell value is
        // re-derived here.
        let first_cell = escape_aware_first_field(line);
        if !first_cell_matches_bc_id(&first_cell, bc_id) {
            continue;
        }

        // Conditions (1)+(2) satisfied — this is a candidate line.
        // Condition (3): non-empty field count from the escape-aware split above.

        match non_empty_fields.len() {
            5 => return BcIndexVersionState::RowPresentNoVersion,
            n if n >= 6 => {
                // Reconstruct the full version-chain cell by joining all non-empty fields
                // from index 5 onwards with `|`.
                //
                // F-P6-019d: bare (unescaped) `|` characters inside the version-chain cell
                // create phantom field boundaries when splitting on `|`, scattering later
                // version tokens (e.g., v1.17, v1.18) into fields 7+. Joining
                // non_empty_fields[5..] with `|` recovers the complete cell content.
                // For rows without bare pipes (the common case), this joins a single field —
                // equivalent to using non_empty_fields[5] alone.
                let version_cell = non_empty_fields[5..].join("|");
                return match extract_first_v_token_of_last_entry(&version_cell) {
                    Some(v) => BcIndexVersionState::Version(v),
                    // F-P6-018 (BC-5.39.010 §PC5 normative): ≥6 fields AND no
                    // \bv([0-9]+\.[0-9]+)\b in field 6 → RowPresentNoVersion.
                    // Same Continue outcome as the 5-field case; no version to compare.
                    None => BcIndexVersionState::RowPresentNoVersion,
                };
            }
            // <5 fields: candidate found but not a valid body-table row.
            // BC-5.39.010 §PC5 (F-P4-005): do NOT return here — keep scanning for
            // a valid (≥5-field) line. RowMalformed is returned only when the entire file
            // has been scanned and NO valid candidate was found.
            // MUST NOT be collapsed into RowAbsent — that would trigger false BLOCKs.
            n => {
                if first_malformed_count.is_none() {
                    first_malformed_count = Some(n);
                }
            }
        }
    }

    // Full scan complete. No valid (≥5-field) candidate found.
    match first_malformed_count {
        Some(n) => BcIndexVersionState::RowMalformed(n),
        None => BcIndexVersionState::RowAbsent,
    }
}

/// Returns `true` when the BC-INDEX terminal chain-entry version equals the normalized
/// BC frontmatter version; `false` on any mismatch.
///
/// This is the terminal-value predicate that replaced the deleted
/// `bc_index_row_contains_version` whole-row helper (F-S2107-P8-006). `index_ver` is
/// the value already extracted by `extract_bc_index_version_state`; this function
/// never inspects the raw index row, so whole-row-search semantics cannot be silently
/// reintroduced inside it. The corpus gate
/// (`test_BC_corpus_version_sync_all_indexed_bcs_match_frontmatter`) calls this
/// function; it is pinned by `test_bypass_1_*`, `test_bypass_2_*`, `test_bypass_3_*`
/// in `lib.rs`.
///
/// # TEST-ONLY — STRICT EQUALITY, deliberately stricter than production (ADV-RECON11-002)
///
/// This function is used ONLY by `#[cfg(test)]` code in `lib.rs` (the corpus-sync
/// gate and the three bypass-mutant tests) — never by the live PostToolUse dispatch
/// path. It is `pub` (not `pub(crate)`) solely so the cross-module test call sites in
/// `lib.rs` can name it; that visibility is NOT an invitation to wire it into
/// production.
///
/// Its strict-equality semantics (`index_ver == frontmatter_version`, no directional
/// carve-out) are CORRECT for its actual purpose — the corpus-sync gate asserts a
/// post-burst invariant ("every indexed BC's BC-INDEX entry exactly matches its
/// frontmatter") where any drift, in either direction, is a sync failure that must be
/// caught by CI.
///
/// This is intentionally DIFFERENT from — and stricter than — the runtime comparison
/// in [`run_arm_a1_with_index_result`], which implements the write-time PC2 directional
/// carve-out: `bc_version > index_version` (primary newer than index) is a legitimate,
/// non-blocking burst-ordering artifact (PC2a, advisory only), while
/// `index_version >= bc_version` is anomalous and blocks (PC2b). Post-burst corpus
/// state has no such in-flight window, so the corpus gate is right to demand exact
/// parity rather than tolerate the same directional slack.
///
/// **Do not** call this function from `run_arm_a1_with_index_result` or any other
/// production comparison path — doing so would re-introduce false BLOCKs for BCs
/// whose BC-INDEX entry legitimately lags behind a just-bumped frontmatter version
/// during an in-progress burst. The production directional comparison must stay
/// inline in `run_arm_a1_with_index_result` (see its `index_version == bc_version` /
/// PC2a / PC2b branches below).
///
/// # Arguments
/// * `index_ver` — last-chain-entry version extracted by `extract_bc_index_version_state`,
///   already stripped of the `v` prefix (e.g., `"1.19"`).
/// * `frontmatter_version` — BC frontmatter `version:` field value; may carry a leading
///   `v` (F-P6-019a / BC-5.24.006 pattern), stripped before comparison.
///
/// Three bypass vectors that whole-row search admitted are permanently closed:
///   - **index-newer-than-primary** (`v1.18 \| v1.19`, frontmatter `1.18`):
///     `"1.19" ≠ "1.18"` → `false` (mismatch detected).
///   - **annotation-rollback** (`v1.24 (promoted v1.23 …)`, frontmatter `1.23`):
///     `"1.24" ≠ "1.23"` → `false` (mismatch detected).
///   - **chain-rollback** (`v1.10 \| v1.11 \| v1.12 \| v1.13`, frontmatter `1.10`):
///     `"1.13" ≠ "1.10"` → `false` (mismatch detected).
///
/// # BC trace
/// F-S2107-P8-006; F-S2107-P9-001; POLICY 11 `no_test_tautologies`;
/// POLICY 15 per-guard mutant mandate.
pub fn index_ver_matches_frontmatter(index_ver: &str, frontmatter_version: &str) -> bool {
    let normalized_fv = frontmatter_version.trim_start_matches('v');
    normalized_fv == index_ver
}

/// Extracts the first non-empty pipe-delimited field of `line`, using the same
/// escape-aware split as `extract_bc_index_version_state`'s condition (2)/(3)
/// candidacy check: `\|` is substituted with the `\x00` sentinel BEFORE splitting
/// on `|`, so an escaped pipe inside an earlier cell cannot create a phantom field
/// boundary that shifts which cell counts as "first."
///
/// Shared by `extract_bc_index_version_state` (BC-INDEX row first-cell extraction,
/// PC5 condition (2)) and `arm_a2::row_first_field_matches_bc_id` (story-table row
/// first-cell extraction, PC13 Phase 1 eligibility gate, ADV-RECON-003) so
/// the two "first cell" computations can never silently diverge (TD-VSDD-060
/// sibling-site sweep; ADV-RECON3-007 — the prior Arm2 copy used a naive
/// `line.split('|')` with no escape substitution, which is byte-identical to this
/// escape-aware split for every corpus row today — no row has an escaped pipe
/// before its first cell — but was not provably the SAME computation).
///
/// # BC trace
/// BC-5.39.010 PC5 condition (2) (escape-aware first-cell, F-P4-017); PC13
/// Phase 1 (ADV-RECON-003 / ADV-RECON2-001 / ADV-RECON3-007).
pub(crate) fn escape_aware_first_field(line: &str) -> String {
    let escaped = line.replace("\\|", "\x00");
    escaped
        .split('|')
        .map(str::trim)
        .find(|field| !field.is_empty())
        .unwrap_or("")
        .to_string()
}

/// Returns `true` if the trimmed first pipe-cell content of a BC-INDEX body-table row
/// matches the given BC ID under the normative recognition predicate.
///
/// **Normative recognition predicate condition (2) per BC-5.39.010 §PC5:**
/// - **Link form:** first cell starts with `[bc_id]` followed by `(` (markdown link:
///   `[BC-5.39.010](ss-05/BC-5.39.010.md)`)
/// - **Plain form:** first cell equals `bc_id` exactly (e.g., `BC-5.39.010`)
///
/// More precise than `contains` — requires the BC ID to BE the cell content, not merely
/// a substring. Prevents cross-reference rows (where bc_id appears in Title/Depends cells)
/// from being classified as the BC's own registration row.
///
/// `pub(crate)`: reused by `arm_a2::row_first_field_matches_bc_id` (BC-5.39.010
/// PC13 Phase 1, ADV-RECON-003 / ADV-RECON2-001) so that Part A Arm1's row-eligibility
/// anchor and Part A Arm2's Phase 1 row-eligibility anchor are the SAME function, not
/// two independently-maintained copies that can drift apart (TD-VSDD-060 sibling-site
/// sweep — the two copies previously diverged: Arm1 used this strong equals/link-form
/// predicate while Arm2 used a weaker `contains`-at-boundary predicate).
///
/// # BC trace
/// BC-5.39.010 §PC5: normative recognition predicate condition (2).
/// BC-5.39.010 §PC13 Phase 1 (ADV-RECON-003 / ADV-RECON2-001): reused verbatim
/// by Arm2 as "the same locator-predicate test already normative for Part A Arm1".
/// F-P2-002: first-cell anchor.
pub(crate) fn first_cell_matches_bc_id(first_cell: &str, bc_id: &str) -> bool {
    // Plain form: cell IS the bc_id (e.g., "BC-5.39.010")
    if first_cell == bc_id {
        return true;
    }
    // Link form: cell starts with "[bc_id]" (e.g., "[BC-5.39.010](ss-05/BC-5.39.010.md)")
    // Strip the leading `[`, then check that bc_id is followed immediately by `]`.
    if let Some(rest) = first_cell.strip_prefix('[')
        && let Some(after_id) = rest.strip_prefix(bc_id)
    {
        return after_id.starts_with(']');
    }
    false
}

/// Find the FIRST `\bv([0-9]+\.[0-9]+)\b` token in `text`.
///
/// Scans left-to-right and returns the first v-prefixed version token encountered.
/// Used to extract the authoritative version from a single chain entry, where the
/// first v-token is the current version and subsequent tokens are annotation prose
/// (e.g., back-references in `(promoted v1.23 D-839)` clauses or `[prior: v1.4]`).
///
/// Leading/trailing `\b` boundary checks route through the crate-shared
/// `crate::is_word_char` predicate (F-S2107-RECON-005 — sibling-file sweep of
/// F-S2107-RECON-004, which had unified only Arm B's `\b` checks). `_` is a
/// word character under regex `\b` semantics, so a `v`-token immediately
/// preceded or followed by `_` must NOT match.
///
/// # BC trace
/// F-P6-019b/019c: first-token-of-entry semantics (replaces last-wins extraction).
fn extract_first_v_token(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'v' {
            let prev_ok = i == 0 || !crate::is_word_char(bytes[i - 1] as char);
            if prev_ok && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() {
                let start = i + 1;
                let mut end = start;
                while end < bytes.len() && bytes[end].is_ascii_digit() {
                    end += 1;
                }
                if end < bytes.len() && bytes[end] == b'.' {
                    end += 1;
                    if end < bytes.len() && bytes[end].is_ascii_digit() {
                        while end < bytes.len() && bytes[end].is_ascii_digit() {
                            end += 1;
                        }
                        let next_ok =
                            end >= bytes.len() || !crate::is_word_char(bytes[end] as char);
                        if next_ok {
                            return Some(text[start..end].to_string());
                        }
                    }
                }
            }
        }
        i += 1;
    }
    None
}

/// Extract the current version from a version-chain cell.
///
/// Algorithm (F-P6-019b/019c — first-token-of-last-chain-entry):
/// 1. Split on `\x00` (the escape sentinel substituted from `\|` before this call).
///    Chain entries are ordered oldest-to-newest; `\x00` is the entry separator.
/// 2. Take the LAST non-empty segment — the most-recent chain entry.
/// 3. Extract the FIRST `\bv([0-9]+\.[0-9]+)\b` token from that segment.
///    The first v-token in an entry is the authoritative current version; subsequent
///    v-tokens are annotation prose (e.g., `(promoted v1.23 D-839)`, `[prior: v1.4]`).
///
/// For cells with no `\x00` separators (single-entry or annotation-only cells),
/// this is equivalent to extracting the first v-token from the entire cell.
///
/// Returns `None` if the cell is empty or if no v-token exists in the last non-empty
/// chain entry.
///
/// # BC trace
/// F-P6-019b: parenthetical backward reference `(promoted v1.23)` must not shadow
///   the current version `v1.24` that precedes it in the same chain entry.
/// F-P6-019c: `[prior: v1.4]` annotation must not shadow the current version `v1.5`.
/// F-P6-019d (via caller): version_cell reconstruction joins fields 5+ so that
///   bare-pipe fragments in entry annotations are reassembled before this call.
fn extract_first_v_token_of_last_entry(cell: &str) -> Option<String> {
    // Split on \x00 (escape sentinel) to isolate chain entries.
    // For cells without any \x00 (no \| escape-chains), produces a single entry.
    let last_entry = cell.split('\x00').rfind(|s| !s.trim().is_empty())?;
    extract_first_v_token(last_entry)
}

/// Class A Arm1 check with the BC-INDEX.md read result provided as a seam.
///
/// This is the pure-seam entry point for unit testing: the caller provides the
/// host read result rather than performing a live `host::read_file` call. The
/// effectful wrapper `run_arm_a1` (used in `on_post_tool_use`) calls this after
/// performing the real read.
///
/// Returns `(violations, advisories)`:
/// - Block violation: INDEX row present with stale version.
/// - Block violation: BC version > 1.0 and no INDEX row (previous registration
///   existed but was dropped — postcondition 4).
/// - Advisory: BC version == "1.0" and no INDEX row (new BC not yet registered —
///   postcondition 3).
/// - Block violation: `CapabilityDenied` on BC-INDEX.md (invariant 5).
/// - Advisory: `NotFound` on BC-INDEX.md (precondition 8, invariant 5 second clause).
///
/// # BC trace
/// BC-5.39.010 Class A Arm1 postconditions and preconditions (current version per
/// BC-INDEX) — dispatches on all five `BcIndexVersionState` variants: `RowAbsent`
/// (postconditions 3/4), `RowPresentNoVersion` (silent continue), `Version`
/// directional carve-out (PC2a/PC2b, postconditions 1/2), `RowMalformed`
/// (postcondition 4a), and `IndexUnreadable` (precondition 15b / postcondition 26,
/// v1.22 / ADV-RECON11-001). BC-INDEX.md secondary-read HostError handling:
/// precondition 8, invariants 4/5.
pub fn run_arm_a1_with_index_result(
    bc_id: &str,
    bc_version: &str,
    _bc_file_path: &str,
    index_read_result: Result<Vec<u8>, HostError>,
) -> (Vec<Violation>, Vec<Advisory>) {
    match index_read_result {
        Err(HostError::CapabilityDenied) => {
            // BC-INDEX.md CapabilityDenied → block (invariant 5)
            let violation = Violation {
                description: format!(
                    "validate-cross-site-correspondence [Class A Arm1]: \
                    CapabilityDenied reading BC-INDEX.md for '{bc_id}' — \
                    sandbox misconfiguration. Verify read_file path_allow includes \
                    '.factory/specs/behavioral-contracts/'. POLICY 14 leg 5."
                ),
            };
            (vec![violation], vec![])
        }
        Err(HostError::NotFound) => {
            // BC-INDEX.md NotFound → advisory + Continue (bootstrap ordering)
            let advisory = Advisory {
                message: format!(
                    "validate-cross-site-correspondence [Class A Arm1] advisory: \
                    BC-INDEX.md not found when validating '{bc_id}' — \
                    bootstrap ordering: INDEX may not exist yet."
                ),
            };
            (vec![], vec![advisory])
        }
        Err(other) => {
            // Other host errors → block (fail-closed)
            let violation = Violation {
                description: format!(
                    "validate-cross-site-correspondence [Class A Arm1]: \
                    host error reading BC-INDEX.md for '{bc_id}': {other:?}. \
                    POLICY 14 leg 5."
                ),
            };
            (vec![violation], vec![])
        }
        Ok(index_bytes) => {
            // F-P6-019a: normalize bc_version — strip leading `v` if present.
            // BC frontmatter may carry `version: "v1.3"` (BC-5.24.006 pattern).
            // extract_bc_index_version_state returns version tokens without the v prefix.
            // Shadow the parameter so all downstream comparisons use the same format.
            let bc_version = bc_version.trim_start_matches('v');
            // BC-5.39.010: dispatches on all five BcIndexVersionState variants — the
            // §PC5 four-state post-decode classification (full-file scan) plus the
            // v1.22 pre-decode IndexUnreadable state (arm below).
            match extract_bc_index_version_state(bc_id, &index_bytes) {
                BcIndexVersionState::RowAbsent => {
                    // No candidate line found at all — BC not in INDEX.
                    // Postconditions 3 (v1.0 → advisory) and 4 (v>1.0 → block).
                    let is_v1_0 = bc_version == "1.0";
                    if is_v1_0 {
                        let advisory = Advisory {
                            message: format!(
                                "validate-cross-site-correspondence [Class A Arm1] advisory: \
                                '{bc_id}' (v{bc_version}) not yet registered in BC-INDEX.md — \
                                new BC, bootstrap ordering."
                            ),
                        };
                        (vec![], vec![advisory])
                    } else {
                        let violation = Violation {
                            description: format!(
                                "validate-cross-site-correspondence [Class A Arm1]: \
                                '{bc_id}' (v{bc_version}) has no row in BC-INDEX.md — \
                                previous registration appears to have been dropped. \
                                Add or restore the INDEX row. POLICY 14 leg 5."
                            ),
                        };
                        (vec![violation], vec![])
                    }
                }
                BcIndexVersionState::RowPresentNoVersion => {
                    // Candidate found; canonical 5-column shape — no version-chain cell.
                    // PC5 postcondition 4: RowPresentNoVersion → silent-continue.
                    // ~1,943 of 1,983 BC-INDEX rows have this shape; none are an error.
                    (vec![], vec![])
                }
                BcIndexVersionState::Version(index_version) => {
                    // Candidate found; explicit version-chain cell — compare versions.
                    // BC-5.39.010 §PC2: directional carve-out.
                    // Strip 'v' prefix (already stripped by extract_last_v_token); parse
                    // as major.minor integers for directional comparison.
                    // PC2a: bc_version > index_version → burst-ordering artefact (primary
                    //        written before index); advisory + Continue.
                    // PC2b: index_version >= bc_version (and not equal) OR parse failure
                    //        → anomalous block (index cannot legitimately advance ahead of BC).
                    if index_version == bc_version {
                        (vec![], vec![])
                    } else {
                        let parse_version = |v: &str| -> Option<(u32, u32)> {
                            let mut it = v.splitn(2, '.');
                            let maj = it.next()?.parse::<u32>().ok()?;
                            let min = it.next()?.parse::<u32>().ok()?;
                            Some((maj, min))
                        };
                        let primary_newer =
                            match (parse_version(bc_version), parse_version(&index_version)) {
                                (Some(fm), Some(idx)) => fm > idx,
                                // Parse failure → treat as PC2b (anomalous, block).
                                _ => false,
                            };
                        if primary_newer {
                            // PC2a: primary newer than index — burst-ordering artefact.
                            // Advisory + Continue (Class A BLOCK suspended).
                            let advisory = Advisory {
                                message: format!(
                                    "validate-cross-site-correspondence [Class A Arm1] advisory: \
                                    BC-INDEX.md body-table row for {bc_id} cites v{index_version} \
                                    but frontmatter version: is \"{bc_version}\" — primary newer \
                                    than index; state-manager index update pending; \
                                    Class A BLOCK suspended."
                                ),
                            };
                            (vec![], vec![advisory])
                        } else {
                            // PC2b: index newer than primary OR parse failure — anomalous, block.
                            let violation = Violation {
                                description: format!(
                                    "validate-cross-site-correspondence [Class A Arm1]: \
                                    BC-INDEX.md body-table row for {bc_id} cites v{index_version} \
                                    but frontmatter version: is \"{bc_version}\" — index is newer \
                                    than primary. This is anomalous: the index cannot legitimately \
                                    advance ahead of the BC it cites. Verify no index row was \
                                    updated out-of-burst or under the wrong BC path. \
                                    Update per POLICY 14 leg 5."
                                ),
                            };
                            (vec![violation], vec![])
                        }
                    }
                }
                BcIndexVersionState::RowMalformed(field_count) => {
                    // Candidate line found but <5 non-empty fields after escape-aware split —
                    // NOT a valid BC-INDEX body-table row (likely a Changelog entry,
                    // subsystem-section row, or notes table carrying the BC ID link).
                    // PC5 postcondition 4a: advisory + Continue. NEVER blocks.
                    // MUST NOT reach the RowAbsent blocking path (postcondition 4) —
                    // a found-but-malformed line is not a dropped registration.
                    // BC-5.39.010 §PC4a (NORMATIVE — verbatim message required):
                    let advisory = Advisory {
                        message: format!(
                            "validate-cross-site-correspondence [Class A Arm1]: \
                            BC-INDEX.md contains a malformed candidate line for {bc_id} \
                            ({field_count} fields found; expected ≥5 for a valid body-table \
                            row). This line is structurally not a BC-INDEX body-table row \
                            (likely a Changelog entry or notes table). Registration status \
                            cannot be determined from this line. \
                            Verify BC-INDEX body-table registration manually."
                        ),
                    };
                    (vec![], vec![advisory])
                }
                BcIndexVersionState::IndexUnreadable => {
                    // BC-5.39.010 precondition 15b / postcondition 26 (v1.22 /
                    // ADV-RECON11-001): BC-INDEX.md succeeded as bytes but failed UTF-8
                    // decoding. Row state is genuinely INDETERMINATE — MUST NOT fall
                    // through to RowAbsent's postcondition-4 BLOCK path (which would
                    // misreport index-file corruption as a "dropped registration").
                    // Deliberately lower severity than precondition 15a / postcondition
                    // 25's primary-target BLOCK: advisory + Continue only.
                    let advisory = Advisory {
                        message: format!(
                            "validate-cross-site-correspondence: BC-INDEX.md failed UTF-8 \
                            decode — row/hash state for '{bc_id}' is INDETERMINATE, not \
                            confirmed-absent. Fix: verify the index file's encoding and \
                            re-save as UTF-8."
                        ),
                    };
                    (vec![], vec![advisory])
                }
            }
        }
    }
}

/// Class A Arm1 effectful entry point.
///
/// Performs the live `host::read_file` for BC-INDEX.md (with `max_bytes = 1048576`,
/// `timeout_ms = 3000`), then delegates to `run_arm_a1_with_index_result`.
///
/// Called from `on_post_tool_use` after the primary BC file content has been read.
///
/// # Implementation guide
/// ```text
/// let index_result = host::read_file(".factory/specs/behavioral-contracts/BC-INDEX.md",
///     BC_INDEX_MAX_BYTES, BC_INDEX_TIMEOUT_MS);
/// run_arm_a1_with_index_result(bc_id, bc_version, bc_file_path, index_result)
/// ```
///
/// # BC trace
/// BC-5.39.010 precondition 7 (primary BC file read failure — handled upstream in
/// `lib.rs` before this fn is called) and precondition 8 (BC-INDEX.md secondary
/// read: NotFound advisory / CapabilityDenied block); invariants 4/5 (fail-closed
/// on CapabilityDenied, fail-open on secondary NotFound).
pub fn run_arm_a1(
    bc_id: &str,
    bc_version: &str,
    bc_file_path: &str,
) -> (Vec<Violation>, Vec<Advisory>) {
    let index_result = vsdd_hook_sdk::host::read_file(
        ".factory/specs/behavioral-contracts/BC-INDEX.md",
        BC_INDEX_MAX_BYTES,
        BC_INDEX_TIMEOUT_MS,
    );
    run_arm_a1_with_index_result(bc_id, bc_version, bc_file_path, index_result)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // derive_bc_path — BC-5.39.010 precondition 11
    // -----------------------------------------------------------------------

    #[test]
    fn test_BC_5_39_010_arm_a1_bc_path_derivation_ss05() {
        // BC-5.39.010 PC11: BC-5.39.010 → ss-05
        let path = derive_bc_path("BC-5.39.010");
        assert_eq!(
            path,
            ".factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md"
        );
    }

    #[test]
    fn test_BC_5_39_010_arm_a1_bc_path_derivation_ss06() {
        // BC path derivation for single-digit section zero-padded
        let path = derive_bc_path("BC-6.26.001");
        assert_eq!(
            path,
            ".factory/specs/behavioral-contracts/ss-06/BC-6.26.001.md"
        );
    }

    // -----------------------------------------------------------------------
    // run_arm_a1_with_index_result — BC-5.39.010 postconditions 1-6
    // -----------------------------------------------------------------------

    /// AC-001 MUTANT: index-ahead-of-primary blocks (BC-5.39.010 §PC2b postcondition 2).
    ///
    /// BC-5.39.010 §PC2 directional carve-out:
    /// - PC2a (primary newer than index): advisory + Continue (burst-ordering artefact).
    /// - PC2b (index newer than primary): block (anomalous — index cannot legitimately
    ///   advance ahead of the BC it cites).
    ///
    /// This test covers PC2b: bc_version < index_version → block.
    /// For PC2a (bc_version > index_version → advisory) see T-P6A bats integration test.
    #[test]
    fn test_BC_5_39_010_arm_a1_stale_index_blocks() {
        // PC2b: INDEX row cites v1.10, but BC frontmatter is only at v1.9.
        // index (1.10) > primary (1.9) → anomalous block.
        let index_content = b"| BC-5.39.010 | some title | draft | CAP-032 | S-21.07 | v1.10 |\n";
        let (violations, _) = run_arm_a1_with_index_result(
            "BC-5.39.010",
            "1.9",
            ".factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md",
            Ok(index_content.to_vec()),
        );
        assert!(
            !violations.is_empty(),
            "index-ahead-of-primary (PC2b) must produce a blocking violation"
        );
        let msg = &violations[0].description;
        assert!(
            msg.contains("[Class A Arm1]"),
            "violation must cite [Class A Arm1]"
        );
        assert!(
            msg.contains("v1.10"),
            "violation must cite the index version v1.10"
        );
        assert!(
            msg.contains("1.9"),
            "violation must cite the primary version 1.9"
        );
        assert!(
            msg.contains("index is newer than primary"),
            "violation must describe the PC2b anomalous direction"
        );
        assert!(
            msg.contains("POLICY 14 leg 5"),
            "violation must cite POLICY 14 leg 5"
        );
    }

    /// AC-001 CONTROL: current INDEX row passes (BC-5.39.010 postcondition 1).
    #[test]
    fn test_BC_5_39_010_arm_a1_current_index_passes() {
        let index_content = b"| BC-5.39.010 | some title | draft | CAP-032 | S-21.07 | v1.6 |\n";
        let (violations, _) = run_arm_a1_with_index_result(
            "BC-5.39.010",
            "1.6",
            ".factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md",
            Ok(index_content.to_vec()),
        );
        assert!(
            violations.is_empty(),
            "current INDEX row must produce no violations"
        );
    }

    /// AC-002: v1.0 BC not in INDEX → advisory-only (BC-5.39.010 postcondition 3).
    #[test]
    fn test_BC_5_39_010_arm_a1_new_v1_0_not_in_index_advisory() {
        // BC-INDEX has no row for BC-9.99.001 (new BC)
        let index_content = b"| BC-5.39.010 | existing | v1.6 | 2026-07-01 | active |\n";
        let (violations, advisories) = run_arm_a1_with_index_result(
            "BC-9.99.001",
            "1.0",
            ".factory/specs/behavioral-contracts/ss-09/BC-9.99.001.md",
            Ok(index_content.to_vec()),
        );
        assert!(violations.is_empty(), "v1.0 not-in-INDEX must not block");
        assert!(
            !advisories.is_empty(),
            "v1.0 not-in-INDEX must emit an advisory"
        );
    }

    /// AC-002: v1.1 BC not in INDEX → block (BC-5.39.010 postcondition 4).
    #[test]
    fn test_BC_5_39_010_arm_a1_v1_1_not_in_index_blocks() {
        // Version > 1.0 means previous registration existed but was dropped
        let index_content = b"| BC-5.39.010 | existing | v1.6 | 2026-07-01 | active |\n";
        let (violations, _) = run_arm_a1_with_index_result(
            "BC-9.99.001",
            "1.1",
            ".factory/specs/behavioral-contracts/ss-09/BC-9.99.001.md",
            Ok(index_content.to_vec()),
        );
        assert!(
            !violations.is_empty(),
            "v1.1 not-in-INDEX must produce a blocking violation"
        );
    }

    /// AC-003: BC-INDEX.md CapabilityDenied → block (BC-5.39.010 invariant 5).
    #[test]
    fn test_BC_5_39_010_arm_a1_bc_index_capability_denied_blocks() {
        let (violations, _) = run_arm_a1_with_index_result(
            "BC-5.39.010",
            "1.6",
            ".factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md",
            Err(HostError::CapabilityDenied),
        );
        assert!(
            !violations.is_empty(),
            "CapabilityDenied on BC-INDEX.md must produce a blocking violation"
        );
    }

    /// AC-003: BC-INDEX.md NotFound → advisory + Continue (BC-5.39.010 precondition 8).
    #[test]
    fn test_BC_5_39_010_arm_a1_bc_index_not_found_advisory() {
        let (violations, advisories) = run_arm_a1_with_index_result(
            "BC-5.39.010",
            "1.6",
            ".factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md",
            Err(HostError::NotFound),
        );
        assert!(
            violations.is_empty(),
            "NotFound on BC-INDEX.md must not block"
        );
        assert!(
            !advisories.is_empty(),
            "NotFound on BC-INDEX.md must emit an advisory"
        );
    }

    // -----------------------------------------------------------------------
    // F-S2107-P1B-006: escaped-pipe version chain — extract_bc_index_version_state
    // splits on '|' (which also splits at `\|` sequences in the raw bytes),
    // then calls extract_version_token on each cell. The first cell has "v1.3"
    // and is returned immediately without scanning later cells for a higher version.
    // Production BC-INDEX rows use `v1.3 \| v1.4 \| ... \| v1.12`; current version
    // is always the LAST token. Current code returns "1.3" → "1.3" ≠ "1.12" → BLOCK.
    //
    // F-S2107-P1B-007: frontmatter changelog pipe false-match — the YAML frontmatter
    // of BC-INDEX.md contains changelog entries that reference BC IDs with `|` chars
    // in the version column. `extract_bc_index_version_state` scans ALL lines and matches
    // any line containing BOTH '|' AND the bc_id. A frontmatter line like:
    //   `    change: "v4.43: BC-5.39.010 v1.5|v1.6."` → matches before the body row.
    // Result: returns "4.43" (from `v4.43`) instead of "1.6" → BLOCK.
    // -----------------------------------------------------------------------

    /// T-039 (Rust unit test): escaped-pipe chain must use LAST token (F-S2107-P1B-006).
    ///
    /// BC-5.39.010 §PC5 (F-P6-019b/019c): when the version-chain cell has an
    /// escaped-pipe-delimited chain, the first token of the last chain entry is authoritative.
    ///
    /// RED GATE: current code returns "1.3" (first token from split('|') on
    /// `v1.3 \| v1.4 \| v1.5 \| v1.6`). "1.3" ≠ "1.6" → violation → NOT empty.
    /// assert!(violations.is_empty()) FAILS → RED gate.
    /// After fix (scan all tokens, return last version found): "1.6" → empty → PASSES.
    #[test]
    fn test_BC_5_39_010_arm_a1_escaped_pipe_chain_last_token_wins() {
        // Production-shaped INDEX row: `v1.3 \| v1.4 \| v1.5 \| v1.6` (4-version chain)
        // split('|') yields cells: `""`, `" [BC-5.39.010]..."`, ..., `" v1.3 \"`, ...
        // extract_version_token on first cell "v1.3 \" → "1.3" (first match)
        // extract_version_token on last cell " v1.6 " → "1.6"
        // After fix: LAST version token is authoritative → returns "1.6"
        let index = concat!(
            "| [BC-5.39.010](ss-05/BC-5.39.010.md) | title | draft | CAP-032 | S-21.07",
            " | v1.3 \\| v1.4 \\| v1.5 \\| v1.6 |\n",
        );
        let (violations, _) = run_arm_a1_with_index_result(
            "BC-5.39.010",
            "1.6",
            ".factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md",
            Ok(index.as_bytes().to_vec()),
        );
        assert!(
            violations.is_empty(),
            "escaped-pipe version chain `v1.3 \\| v1.4 \\| v1.5 \\| v1.6`: current version \
            '1.6' is the LAST token → must not block. \
            Red Gate: current code extracts first token '1.3' → '1.3' ≠ '1.6' → violation (F-S2107-P1B-006)"
        );
    }

    /// T-039b (Rust unit test): frontmatter changelog line must not be matched as BC body row
    /// (F-S2107-P1B-007).
    ///
    /// BC-5.39.010 §PC5 condition (1) (F-S2107-P1B-007): extract_bc_index_version_state
    /// must scan only lines starting with '|' — skipping YAML frontmatter, prose, and blank lines.
    ///
    /// RED GATE: current code scans ALL lines. Frontmatter changelog entry
    /// `    change: "v4.43: BC-5.39.010 v1.5|v1.6."` contains both `|` and "BC-5.39.010"
    /// → matched first. extract_version_token on `    change: "v4.43: BC-5.39.010 v1.19`
    /// returns "4.43". "4.43" ≠ "1.6" → violation → violations NOT empty.
    /// assert!(violations.is_empty()) FAILS → RED gate.
    /// After fix (skip lines before body): canonical 6-field body row with `v1.6` in
    /// the version column → Version("1.6") → "1.6" matches bc_version "1.6" → empty → PASSES.
    #[test]
    fn test_BC_5_39_010_arm_a1_frontmatter_changelog_pipe_not_matched_as_table_row() {
        // YAML frontmatter has changelog line with both `|` and BC-5.39.010 → false match
        // canonical 6-field body row with v1.6 in the version column → should pass
        let index = concat!(
            "---\n",
            "document_type: bc-index\n",
            "changelog:\n",
            "  - date: 2026-07-31\n",
            "    change: \"v4.43: BC-5.39.010 v1.5|v1.6.\"\n",
            "---\n\n",
            "| BC ID | Title | Status | Capabilities | Stories | Version History |\n",
            "|-------|-------|--------|-------------|---------|------------------|\n",
            "| [BC-5.39.010](ss-05/BC-5.39.010.md) | title | draft | CAP-032 | S-21.07 | v1.6 |\n",
        );
        let (violations, _) = run_arm_a1_with_index_result(
            "BC-5.39.010",
            "1.6",
            ".factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md",
            Ok(index.as_bytes().to_vec()),
        );
        assert!(
            violations.is_empty(),
            "frontmatter changelog line `change: \"v4.43: BC-5.39.010 v1.5|v1.6.\"` must NOT \
            be matched as BC body table row. Red Gate: current code returns '4.43' from \
            frontmatter → '4.43' ≠ '1.6' → violation (F-S2107-P1B-007)"
        );
    }

    // -----------------------------------------------------------------------
    // F-P2-002 (BLOCKER): extract_bc_index_version_state — unanchored first-cell lookup.
    //
    // Bug: `line.contains(bc_id)` matches ANY row that mentions bc_id, not just the
    // row whose FIRST cell IS bc_id. Combined with LAST-wins semantics, a later row
    // that merely cites bc_id in a non-first cell (e.g., a cross-reference in the
    // Title or Depends column) overwrites the correct row's version.
    //
    // ORCH-VERIFIED: BC-INDEX row for BC-1.17.001 (own row) has v1.7; row for BC-2.07.001
    // mentions "BC-1.17.001" in its Title column and has v1.6. LAST-wins + contains picks
    // the later row → returns "1.6" → spurious violation on every BC-1.17.001 write.
    //
    // Fix: anchor on first pipe-cell — only rows whose first cell trims to bc_id match.
    // -----------------------------------------------------------------------

    /// F-P2-002 (BLOCKER): own-row version wins over cross-reference in later row.
    ///
    /// RED GATE: `line.contains("BC-1.17.001")` matches BOTH rows.
    /// LAST-wins picks BC-2.07.001 row's last version token "1.6" → returns Version("1.6").
    /// `assert_eq!(result, BcIndexVersionState::Version("1.7"))` FAILS.
    /// After fix (first-cell anchor): only BC-1.17.001's own row matches → Version("1.7") → PASSES.
    #[test]
    fn test_BC_5_39_010_arm_a1_cross_reference_in_later_row_own_row_version_wins() {
        // BC-INDEX with two rows:
        //   row 1: BC-1.17.001 own row — v1.7 in version column (first cell IS bc_id)
        //   row 2: BC-2.07.001 row — mentions "BC-1.17.001" in Title cell (non-first) with v1.6
        // Expected: extract_bc_index_version_state("BC-1.17.001", ...) → Version("1.7")
        let index = concat!(
            "---\ndocument_type: bc-index\n---\n\n",
            "| BC-1.17.001 | Title A: session-replay gate | draft | CAP-017 | S-14.03 | v1.7 |\n",
            "| BC-2.07.001 | Title B: depends on BC-1.17.001 parity | draft | CAP-018 | S-14.04 | v1.6 |\n",
        );
        let result = extract_bc_index_version_state("BC-1.17.001", index.as_bytes());
        assert_eq!(
            result,
            BcIndexVersionState::Version("1.7".to_string()),
            "extract_bc_index_version_state must anchor on first cell only. \
            BC-1.17.001 own row is v1.7; later BC-2.07.001 row mentions BC-1.17.001 \
            in a non-first cell with v1.6. LAST-wins + unanchored contains returns \
            Version(\"1.6\") (WRONG). F-P2-002 RED GATE. Current: {:?}",
            result
        );
    }

    /// AC-019: Read cap constant values match the spec (BC-5.39.010 §AC-019).
    ///
    /// This test verifies the constant VALUES are correct per spec. It does NOT verify
    /// that these constants are actually passed to `host::read_file` (cap-PASSING).
    ///
    /// Cap-PASSING cannot be tested from the pure seam (`run_arm_a1_with_index_result`)
    /// because the host call lives inside `run_arm_a1` (effectful). The implementer MUST
    /// add a bats integration test for cap-passing once the stub is replaced:
    ///   - Fixture: BC-INDEX.md with a small `max_bytes` override in the test registry
    ///   - Verify: the arm handles a truncated or denied read without out-of-bounds panic
    ///
    /// This test PASSES in Red Gate (constants are already correctly set per spec).
    #[test]
    fn test_BC_5_39_010_AC019_bc_index_read_cap_constant_values_match_spec() {
        assert_eq!(
            BC_INDEX_MAX_BYTES, 1_048_576,
            "[AC-019] BC-INDEX max_bytes must be 1 MiB (1_048_576) per BC-5.39.010"
        );
        assert_eq!(
            BC_INDEX_TIMEOUT_MS, 3_000,
            "[AC-019] BC-INDEX timeout_ms must be 3000 per BC-5.39.010"
        );
        assert_eq!(
            BC_MAX_BYTES, 524_288,
            "[AC-019] BC file max_bytes must be 512 KiB (524_288) per BC-5.39.010"
        );
        assert_eq!(
            BC_TIMEOUT_MS, 3_000,
            "[AC-019] BC timeout_ms must be 3000 per BC-5.39.010"
        );
    }

    // -----------------------------------------------------------------------
    // F-S2107-P3-001 (BLOCKER): three-state extractor — RowPresentNoVersion
    //
    // BC-5.39.010 §PC5: extract_bc_index_version_state must distinguish
    // four post-decode states (RowAbsent, RowPresentNoVersion, Version, RowMalformed) —
    // a fifth, pre-decode state (IndexUnreadable) was added in v1.22; see the enum doc:
    //   RowAbsent         — no line for this BC ID in the index
    //   RowPresentNoVersion — row exists but has no version-chain cell (5-column shape)
    //   Version(v)        — row exists and carries a version token
    //
    // The canonical BC-INDEX shape is 5 columns:
    //   | BC ID | Title | Status | Capability | Stories |
    // Version-chain cell (6th column) appears on only ~40 of 1983 rows.
    // The REMAINING ~1943 rows are RowPresentNoVersion — NOT an error.
    //
    // Two-state None conflation: current code returns None for BOTH RowAbsent
    // AND RowPresentNoVersion. In the None branch, version > "1.0" → BLOCK.
    // This silently blocks every write to ~1943 BCs (e.g., BC-1.01.001).
    //
    // Post-fix: RowPresentNoVersion → silent-continue (no violations, no advisory).
    // -----------------------------------------------------------------------

    /// F-S2107-P3-001 RED GATE: 5-column row (RowPresentNoVersion) with version > "1.0"
    /// must NOT block.
    ///
    /// BC-5.39.010 §PC5: COLUMN-COUNT-ANCHORED classification. After escape-aware
    /// split, count non-empty fields: exactly 5 → RowPresentNoVersion unconditionally —
    /// no token search performed. Current token-search implementation returns None
    /// (no v-prefixed token) which maps to the old RowAbsent → block path.
    /// After fix: RowPresentNoVersion → silent-continue → no violations.
    ///
    /// RED GATE: current None path → is_v1_0=false → block → violations NOT empty.
    #[test]
    fn test_BC_5_39_010_arm_a1_row_present_no_version_cell_not_blocked() {
        // 5-column canonical BC-INDEX shape: no version-chain cell.
        // BC-5.39.010 §PC5: column count alone determines state — no token search
        // is performed on any field, including story IDs in the Stories column.
        let index_content =
            b"| [BC-9.99.001](ss-09/BC-9.99.001.md) | Some title | draft | CAP-TBD | S-99.01 |\n";
        let (violations, advisories) = run_arm_a1_with_index_result(
            "BC-9.99.001",
            "1.2",
            ".factory/specs/behavioral-contracts/ss-09/BC-9.99.001.md",
            Ok(index_content.to_vec()),
        );
        assert!(
            violations.is_empty(),
            "5-column row (no version-chain cell) with bc_version='1.2' must NOT block. \
            BC-5.39.010 §PC5: column-count-anchored — exactly 5 fields → RowPresentNoVersion \
            unconditionally; no token search on any field. \
            Violations: {:?}",
            violations
        );
        assert!(
            advisories.is_empty(),
            "RowPresentNoVersion must be fully silent — no advisory either. \
            BC-5.39.010 §PC5. Advisories: {:?}",
            advisories
        );
    }

    /// F-P6-018: ≥6-field row with no v-token in field 6 → RowPresentNoVersion (normative).
    ///
    /// BC-5.39.010 §PC5 (F-P6-018 normative addition): a row with ≥6 non-empty fields
    /// where the 6th field contains no `\bv([0-9]+\.[0-9]+)\b` token is classified as
    /// `RowPresentNoVersion`, producing the same `Continue` outcome as the 5-field case.
    ///
    /// Rationale: the version-chain cell exists structurally but carries no parseable version.
    /// No direction comparison is possible → same silent-continue as absent version cell.
    /// Both escape paths (≥6 fields / no v-token, and 5 non-empty fields from a blank cell)
    /// produce `Continue` silently (F-P6-018 confirms both are `RowPresentNoVersion`).
    #[test]
    fn test_BC_5_39_010_arm_a1_six_field_no_v_token_row_present_no_version() {
        // ≥6 fields, but 6th field is "N/A" — no \bv[0-9]+\.[0-9]+\b token.
        // F-P6-018: must return RowPresentNoVersion (not RowAbsent or a block).
        let index_content = b"| BC-9.99.001 | Some title | draft | CAP-TBD | S-99.01 | N/A |\n";
        let (violations, advisories) = run_arm_a1_with_index_result(
            "BC-9.99.001",
            "1.2",
            ".factory/specs/behavioral-contracts/ss-09/BC-9.99.001.md",
            Ok(index_content.to_vec()),
        );
        assert!(
            violations.is_empty(),
            "6-field row with no v-token in field 6 must NOT block. \
            F-P6-018 (BC-5.39.010 §PC5 normative): ≥6 fields AND no v-token → \
            RowPresentNoVersion → silent-continue. Violations: {:?}",
            violations
        );
        assert!(
            advisories.is_empty(),
            "6-field row with no v-token in field 6 must emit NO advisory. \
            F-P6-018 (BC-5.39.010 §PC5): RowPresentNoVersion is fully silent. \
            Advisories: {:?}",
            advisories
        );
    }

    /// F-S2107-P3-001 RED GATE: exact BC-1.01.001 INDEX row shape must not block.
    ///
    /// Live corpus evidence (adversary pass-3 verified):
    ///   BC-INDEX row: `| [BC-1.01.001](ss-01/BC-1.01.001.md) | Registry rejects unknown
    ///   schema version | draft | CAP-TBD | S-15.01 |`
    ///   BC-1.01.001.md version: "1.2"
    ///
    /// BC-5.39.010 §PC5: column-count-anchored — 5 non-empty fields after escape-aware
    /// split → RowPresentNoVersion unconditionally. No token search on any field, including
    /// the Stories column which contains "S-15.01". Current implementation performs token
    /// search and finds no v-prefixed token → None → RowAbsent path → block.
    ///
    /// RED GATE: violations not empty. After fix: RowPresentNoVersion → silent-continue.
    #[test]
    fn test_BC_5_39_010_arm_a1_bc_1_01_001_exact_row_shape_not_blocked() {
        // Exact row shape from live BC-INDEX.md (adversary pass-3 corpus verification).
        // BC-5.39.010 §PC5: 5 fields → RowPresentNoVersion, no token search performed.
        let index_content = b"| [BC-1.01.001](ss-01/BC-1.01.001.md) | Registry rejects \
            unknown schema version | draft | CAP-TBD | S-15.01 |\n";
        let (violations, _) = run_arm_a1_with_index_result(
            "BC-1.01.001",
            "1.2",
            ".factory/specs/behavioral-contracts/ss-01/BC-1.01.001.md",
            Ok(index_content.to_vec()),
        );
        assert!(
            violations.is_empty(),
            "BC-1.01.001 canonical 5-column INDEX row with bc_version='1.2' must NOT block. \
            BC-5.39.010 §PC5: 5 fields escape-aware → RowPresentNoVersion unconditionally. \
            F-S2107-P3-001 BLOCKER. Violations: {:?}",
            violations
        );
    }

    /// F-S2107-P3-001 RED GATE (product-owner regression guard): row with S-15.01 in the
    /// Stories column MUST yield RowPresentNoVersion, NOT Version("15.01").
    ///
    /// BC-5.39.010 §PC5 (corpus stat): 194 of 1,943 five-field rows carry story IDs
    /// whose decimal fragments (e.g., "15.01") resemble version tokens. This is the single
    /// most important test in this burst — it names the exact defect the v1.8 contract was
    /// designed to eliminate. Any extractor that converts "S-15.01" → Version("15.01")
    /// is non-conforming with BC-5.39.010 §PC5.
    ///
    /// RED GATE: current None path → RowAbsent → block → violations NOT empty.
    #[test]
    fn test_BC_5_39_010_arm_a1_stories_column_s15_01_yields_row_present_no_version() {
        // Same corpus row as bc_1_01_001_exact_row_shape_not_blocked, explicitly named for
        // the S-15.01 regression guard as required by product-owner (BC-5.39.010 §PC5).
        let index_content = b"| [BC-1.01.001](ss-01/BC-1.01.001.md) | Registry rejects \
            unknown schema version | draft | CAP-TBD | S-15.01 |\n";
        let (violations, _) = run_arm_a1_with_index_result(
            "BC-1.01.001",
            "1.2",
            ".factory/specs/behavioral-contracts/ss-01/BC-1.01.001.md",
            Ok(index_content.to_vec()),
        );
        assert!(
            violations.is_empty(),
            "S-15.01 in the Stories column must yield RowPresentNoVersion, NOT Version('15.01'). \
            BC-5.39.010 §PC5 regression guard: column-count 5 → RowPresentNoVersion, \
            no token search on any field. This is the exact false-positive the v1.8 \
            column-count-anchored contract was designed to close. Violations: {:?}",
            violations
        );
    }

    /// BC-5.39.010 §PC5 escape-aware split RED GATE: 5-field row where the Stories cell
    /// contains a literal `\|` (escaped pipe) must NOT be inflated to 6+ fields.
    ///
    /// Row shape: `| [BC-9.99.002](...) | Title | active | CAP-TBD | S-1.03 \| S-2.06 |`
    ///
    /// Naive `|` split: 6 segments (the `\|` in the Stories cell creates a phantom boundary).
    /// Escape-aware split (substitute `\|` → placeholder, split on `|`, restore):
    ///   field 1: [BC-9.99.002](...)  field 2: Title  field 3: active
    ///   field 4: CAP-TBD             field 5: S-1.03 \| S-2.06
    ///   → 5 non-empty fields → RowPresentNoVersion.
    ///
    /// Current implementation (naive split + token search) returns None on this row (no
    /// v-prefixed token) → RowAbsent path → block. After fix: escape-aware 5 fields →
    /// RowPresentNoVersion → silent-continue.
    ///
    /// RED GATE: violations NOT empty.
    #[test]
    fn test_BC_5_39_010_arm_a1_escape_aware_5field_stories_pipe_not_a_version_cell() {
        // Stories cell contains `\|` which naive splitting inflates to phantom 6th field.
        // BC-5.39.010 §PC5: escape-aware split must count this as 5 fields → RowPresentNoVersion.
        let index_content =
            b"| [BC-9.99.002](ss-09/BC-9.99.002.md) | Two linked stories | active | \
            CAP-TBD | S-1.03 \\| S-2.06 |\n";
        let (violations, _) = run_arm_a1_with_index_result(
            "BC-9.99.002",
            "1.2",
            ".factory/specs/behavioral-contracts/ss-09/BC-9.99.002.md",
            Ok(index_content.to_vec()),
        );
        assert!(
            violations.is_empty(),
            "5-field row with \\| in Stories cell must NOT block. \
            BC-5.39.010 §PC5: escape-aware split → 5 non-empty fields → RowPresentNoVersion. \
            Naive split inflates to 6 fields, converting a RowPresentNoVersion row into a \
            spurious Version check. Violations: {:?}",
            violations
        );
    }

    /// BC-5.39.010 §PC5 escape-aware split GREEN regression guard: 6-field row where
    /// the Version cell contains a `\|`-separated version chain must still yield the correct
    /// version.
    ///
    /// Row shape:
    ///   `| [BC-9.99.003](...) | Title | active | CAP-TBD | S-99.01 | v1.5 \| v1.6 \| v1.7 |`
    ///
    /// Naive split: 9 segments. Escape-aware split:
    ///   field 1: [BC-9.99.003](...)  field 2: Title  field 3: active
    ///   field 4: CAP-TBD             field 5: S-99.01
    ///   field 6: v1.5 \| v1.6 \| v1.7  → ≥6 fields → extract rightmost vN.N from field 6
    ///
    /// Current token-search over naive-split segments accidentally produces the correct answer
    /// ("v1.7" is found in the rightmost phantom segment). This GREEN test is the regression
    /// guard ensuring the escape-aware implementation continues to extract "1.7" correctly.
    #[test]
    fn test_BC_5_39_010_arm_a1_escape_aware_6field_version_chain_with_pipe_regression() {
        // 6-field row: version chain uses `\|` as delimiter within the version cell.
        // The current code passes; this is a GREEN regression guard for the v1.8 fix.
        let index_content =
            b"| [BC-9.99.003](ss-09/BC-9.99.003.md) | Version chain test | active | \
            CAP-TBD | S-99.01 | v1.5 \\| v1.6 \\| v1.7 |\n";
        let (violations, _) = run_arm_a1_with_index_result(
            "BC-9.99.003",
            "1.7",
            ".factory/specs/behavioral-contracts/ss-09/BC-9.99.003.md",
            Ok(index_content.to_vec()),
        );
        assert!(
            violations.is_empty(),
            "6-field version chain row with \\| separators in version cell must extract 'v1.7'. \
            BC-5.39.010 §PC5: escape-aware split → 6 fields → Version from field 6 \
            (rightmost vN.N). This is a regression guard — the fix must NOT break this case. \
            Violations: {:?}",
            violations
        );
    }

    // -----------------------------------------------------------------------
    // F-P4-003 / F-P4-025 — RowMalformed advisory coverage
    //
    // BC-5.39.010 postcondition 4a (NORMATIVE VERBATIM):
    //   "validate-cross-site-correspondence [Class A Arm1]: BC-INDEX.md contains a
    //   malformed candidate line for <id> (<N> fields found; expected ≥5 for a valid
    //   body-table row). This line is structurally not a BC-INDEX body-table row (likely
    //   a Changelog entry or notes table). Registration status cannot be determined from
    //   this line. Verify BC-INDEX body-table registration manually."
    //
    // Fixture: locator-matched line with 2 non-empty fields (real corpus shape: a
    // changelog/notes table row carrying the BC ID link in cell 1, a note in cell 2).
    //
    // RED GATE: current advisory message omits BOTH operator-actionable clauses.
    // The shipped message (arm_a1.rs:406-415) substitutes non-normative prose:
    //   "Not blocking — this is not a dropped registration. Manual verification
    //   recommended. The genuine dropped-registration case (no candidate line at all)
    //   is RowAbsent (postcondition 4). BC-5.39.010 v1.19 PC5 postcondition 4a."
    // Neither "Registration status cannot be determined from this line" nor
    // "Verify BC-INDEX body-table registration manually" appears.
    // -----------------------------------------------------------------------

    /// F-P4-003: RowMalformed MUST NOT block (advisory-only, postcondition 4a).
    ///
    /// Fixture: 2-field locator-matched line (notes table row shape).
    /// BC-5.39.010 PC5 postcondition 4a: RowMalformed → advisory + Continue.
    /// NEVER blocks — a found-but-malformed line is not a dropped registration.
    ///
    /// RED GATE: if this test fails, advisory-only path is broken (test should pass
    /// once advisory message is otherwise corrected per postcondition 4a).
    /// Note: this specific assertion is GREEN (no block) — the RED GATE assertions
    /// are the verbatim-clause tests below.
    #[test]
    fn test_BC_5_39_010_arm_a1_row_malformed_no_block() {
        // 2-field locator-matched line: carries BC link in cell 1, a note in cell 2.
        // Real corpus shape: a notes/changelog table row that happens to carry the BC ID.
        // BC-5.39.010 §PC5: 2 non-empty fields < 5 → RowMalformed(2) → advisory only.
        let index_content = concat!(
            "| BC ID | Title | Status | Capabilities | Stories | Version History |\n",
            "|-------|-------|--------|--------------|---------|----------------|\n",
            "| [BC-5.39.010](ss-05/BC-5.39.010.md) | see D-954 |\n",
        );
        let (violations, advisories) = run_arm_a1_with_index_result(
            "BC-5.39.010",
            "1.6",
            "ss-05/BC-5.39.010.md",
            Ok(index_content.as_bytes().to_vec()),
        );
        assert!(
            violations.is_empty(),
            "RowMalformed MUST NOT block (BC-5.39.010 postcondition 4a: \
            advisory + Continue only). Violations: {:?}",
            violations
        );
        assert!(
            !advisories.is_empty(),
            "RowMalformed MUST emit an advisory (BC-5.39.010 postcondition 4a). \
            Advisories: {:?}",
            advisories
        );
    }

    /// F-P4-003 / F-P4-025 RED GATE: advisory MUST contain verbatim clause 1.
    ///
    /// BC-5.39.010 postcondition 4a NORMATIVE: advisory MUST contain
    /// "Registration status cannot be determined from this line".
    ///
    /// RED GATE: shipped message omits this clause entirely. Test FAILS until
    /// the implementer updates the advisory message to match postcondition 4a verbatim.
    #[test]
    fn test_BC_5_39_010_arm_a1_row_malformed_advisory_clause_registration_status() {
        let index_content = concat!(
            "| BC ID | Title | Status | Capabilities | Stories | Version History |\n",
            "|-------|-------|--------|--------------|---------|----------------|\n",
            "| [BC-5.39.010](ss-05/BC-5.39.010.md) | see D-954 |\n",
        );
        let (violations, advisories) = run_arm_a1_with_index_result(
            "BC-5.39.010",
            "1.6",
            "ss-05/BC-5.39.010.md",
            Ok(index_content.as_bytes().to_vec()),
        );
        assert!(violations.is_empty(), "RowMalformed MUST NOT block");
        assert!(!advisories.is_empty(), "RowMalformed MUST emit advisory");
        let msg = &advisories[0].message;
        assert!(
            msg.contains("Registration status cannot be determined from this line"),
            "advisory MUST contain verbatim postcondition-4a clause: \
            'Registration status cannot be determined from this line'. \
            BC-5.39.010 PC5 postcondition 4a (F-P4-003 / F-P4-025). \
            Got: {msg:?}"
        );
    }

    /// F-P4-003 / F-P4-025 RED GATE: advisory MUST contain verbatim clause 2.
    ///
    /// BC-5.39.010 postcondition 4a NORMATIVE: advisory MUST contain
    /// "Verify BC-INDEX body-table registration manually".
    ///
    /// RED GATE: shipped message omits this operator-actionable instruction.
    /// Test FAILS until the implementer updates the advisory message per postcondition 4a.
    #[test]
    fn test_BC_5_39_010_arm_a1_row_malformed_advisory_clause_verify_bc_index() {
        let index_content = concat!(
            "| BC ID | Title | Status | Capabilities | Stories | Version History |\n",
            "|-------|-------|--------|--------------|---------|----------------|\n",
            "| [BC-5.39.010](ss-05/BC-5.39.010.md) | see D-954 |\n",
        );
        let (violations, advisories) = run_arm_a1_with_index_result(
            "BC-5.39.010",
            "1.6",
            "ss-05/BC-5.39.010.md",
            Ok(index_content.as_bytes().to_vec()),
        );
        assert!(violations.is_empty(), "RowMalformed MUST NOT block");
        assert!(!advisories.is_empty(), "RowMalformed MUST emit advisory");
        let msg = &advisories[0].message;
        assert!(
            msg.contains("Verify BC-INDEX body-table registration manually"),
            "advisory MUST contain verbatim postcondition-4a clause: \
            'Verify BC-INDEX body-table registration manually'. \
            BC-5.39.010 PC5 postcondition 4a (F-P4-003 / F-P4-025). \
            Got: {msg:?}"
        );
    }

    /// F-P4-003: RowMalformed advisory must cite the field count.
    ///
    /// Fixture: 2-field line → RowMalformed(2). Advisory must mention "2" to tell
    /// the operator how many fields were found.
    /// BC-5.39.010 postcondition 4a: "(<N> fields found; expected ≥5 …)".
    ///
    /// Note: this assertion is ADDITIONALLY a RED GATE because the current message
    /// says "non-empty fields found" (different from spec's "fields found") and
    /// the verbatim-clause assertions above will fail first.
    #[test]
    fn test_BC_5_39_010_arm_a1_row_malformed_advisory_cites_field_count() {
        let index_content = concat!(
            "| BC ID | Title | Status | Capabilities | Stories | Version History |\n",
            "|-------|-------|--------|--------------|---------|----------------|\n",
            "| [BC-5.39.010](ss-05/BC-5.39.010.md) | see D-954 |\n",
        );
        let (violations, advisories) = run_arm_a1_with_index_result(
            "BC-5.39.010",
            "1.6",
            "ss-05/BC-5.39.010.md",
            Ok(index_content.as_bytes().to_vec()),
        );
        assert!(violations.is_empty(), "RowMalformed MUST NOT block");
        assert!(!advisories.is_empty(), "RowMalformed MUST emit advisory");
        let msg = &advisories[0].message;
        assert!(
            msg.contains('2'),
            "advisory MUST cite the field count (2 for this fixture). \
            BC-5.39.010 postcondition 4a: '(<N> fields found; …)'. \
            Got: {msg:?}"
        );
    }

    // -----------------------------------------------------------------------
    // F-P6-019a — v-prefix normalization (BC-5.24.006)
    //
    // Root cause: `run_arm_a1_with_index_result` compares `index_version == bc_version`
    // without stripping the leading `v` from bc_version. When the BC frontmatter carries
    // `version: "v1.3"` (leading v), the extracted index_version is "1.3" but bc_version
    // is "v1.3". The strings differ; `parse_version("v1.3")` then fails because the `v`
    // prefix causes `"v1".parse::<u32>()` to return Err. Parse failure is treated as PC2b
    // → false block.
    //
    // Three root causes in these four bugs:
    //   019a → normalization (strip v-prefix from bc_version before comparison)
    //   019b + 019c → last-wins vs first-cell anchoring (same root cause — last v-token
    //                 in the entire field 6 text, not the first token of the last chain entry)
    //   019d → escape-unaware bare-pipe field fragmentation (bare | inside an annotation
    //           creates phantom field boundaries, displacing the version field)
    // -----------------------------------------------------------------------

    /// F-P6-019a RED GATE: bc_version with leading `v` prefix must not trigger PC2b block.
    ///
    /// Real BC: BC-5.24.006 (frontmatter `version: "v1.3"`; INDEX row `v1.3 (…)`).
    ///
    /// Bug: `run_arm_a1_with_index_result` receives `bc_version = "v1.3"` from the
    /// frontmatter parser. `extract_bc_index_version_state` returns `Version("1.3")`.
    /// String comparison `"1.3" != "v1.3"` enters the comparison branch.
    /// `parse_version("v1.3")` → `"v1".parse::<u32>()` fails → `None`.
    /// `(None, Some((1,3)))` match arm → `primary_newer = false` → PC2b BLOCK.
    ///
    /// Fix: strip leading `v` from `bc_version` before the equality check and
    /// before `parse_version` (e.g., `bc_version.trim_start_matches('v')`).
    ///
    /// RED GATE: current implementation produces a violation for bc_version = "v1.3"
    /// with INDEX row "v1.3" → assert!(violations.is_empty()) FAILS.
    #[test]
    fn test_F_P6_019a_v_prefix_in_bc_version_must_not_block() {
        // Synthetic row mirroring BC-5.24.006's INDEX shape.
        // bc_version has the v prefix as it appears in the frontmatter.
        let index_content = b"| [BC-5.24.006](ss-05/BC-5.24.006.md) | phase-4:scenario-rotation | \
            draft | CAP-074 | TBD | v1.3 (v1.3 D-875: correction note; input-hash abc1234) |\n";
        let (violations, advisories) = run_arm_a1_with_index_result(
            "BC-5.24.006",
            "v1.3", // ← leading v as it appears in frontmatter `version: "v1.3"`
            ".factory/specs/behavioral-contracts/ss-05/BC-5.24.006.md",
            Ok(index_content.to_vec()),
        );
        assert!(
            violations.is_empty(),
            "F-P6-019a: bc_version 'v1.3' with INDEX row 'v1.3' must produce NO violation. \
            Root cause: v-prefix on bc_version causes parse_version() to fail → PC2b BLOCK. \
            Fix: strip leading v from bc_version before comparison. \
            Violations: {:?}",
            violations
        );
        assert!(
            advisories.is_empty(),
            "F-P6-019a: bc_version 'v1.3' with INDEX row 'v1.3' must produce NO advisory. \
            Advisories: {:?}",
            advisories
        );
    }

    // -----------------------------------------------------------------------
    // F-P6-019b — backward reference in parenthetical note (BC-3.08.001)
    //
    // Root cause: last-wins extraction scans the entire field-6 text left-to-right and
    // keeps overwriting `last_match`. In BC-3.08.001's real INDEX row, the last chain
    // entry ends with `(promoted v1.23 D-839 S-19.05)` — a backward reference to an
    // older version in the annotation prose. The actual current version (`v1.24`) appears
    // BEFORE the annotation in the same entry, but last-wins overwrites it with `v1.23`.
    //
    // Same root cause as 019c: last-wins vs first-token-of-last-chain-entry.
    // -----------------------------------------------------------------------

    /// F-P6-019b RED GATE: parenthetical backward reference must not shadow the current version.
    ///
    /// Real BC: BC-3.08.001 (frontmatter `version: "1.24"`; INDEX row ends with
    ///   `\| (2026-07-16 D-848: ... v1.24 already active (promoted v1.23 D-839) ...)`).
    ///
    /// Bug: `extract_last_v_token` on field 6 finds:
    ///   `v1.15`, `v1.16`, ..., `v1.24`, `v1.24` (again in annotation) → `v1.23` (from
    ///   "promoted v1.23 D-839") → LAST wins → returns "1.23".
    /// `index_version = "1.23"`, `bc_version = "1.24"`.
    /// `bc (1,24) > index (1,23)` → `primary_newer = true` → false PC2a advisory.
    ///
    /// Fix: use first-token-of-last-chain-entry (split field 6 on `\x00`, take the last
    /// non-empty segment, extract the FIRST v-token from it — the version itself, before
    /// annotation prose).
    ///
    /// RED GATE: `extract_bc_index_version_state` returns `Version("1.23")` instead of
    /// `Version("1.24")` → assert_eq!(result, BcIndexVersionState::Version("1.24")) FAILS.
    #[test]
    fn test_F_P6_019b_parenthetical_backward_reference_returns_current_version() {
        // Synthetic row mimicking the BC-3.08.001 last chain entry structure:
        // current version v1.24 appears first in the entry, then backward-reference to v1.23.
        let index = concat!(
            "| [BC-3.08.001](ss-03/BC-3.08.001.md) | dispatcher async-semantics | ",
            "active | CAP-003 | S-15.01, S-19.05 | v1.22 \\| v1.23 \\| v1.24 (v1.24 2026-07-15: ",
            "event-6-timestamp-field-parity; input-hash 6549a11 unchanged) \\| (2026-07-16 ",
            "D-848: POL-14 PASS-ALREADY-ACTIVE; BC-3.08.001 v1.24 already active (promoted ",
            "v1.23 D-839 S-19.05); no promotion required; input-hash 6549a11 UNCHANGED) |\n",
        );
        let result = extract_bc_index_version_state("BC-3.08.001", index.as_bytes());
        assert_eq!(
            result,
            BcIndexVersionState::Version("1.24".to_string()),
            "F-P6-019b: parenthetical backward reference 'promoted v1.23' must NOT shadow \
            the current version v1.24. extract_bc_index_version_state must return \
            Version(\"1.24\"), not Version(\"1.23\"). \
            Root cause: last-wins picks the last v-token in field 6 ('v1.23' from \
            '(promoted v1.23 D-839)'). Fix: first-token-of-last-chain-entry. \
            Got: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // F-P6-019c — [prior: vN.M] annotation (BC-7.03.079)
    //
    // Root cause: same as 019b — last-wins on field 6. BC-7.03.079's INDEX row uses a
    // `[prior: v1.4 (...)]` suffix (no `\|` separator — a different annotation style).
    // The current version `v1.5` appears first; `[prior: v1.4 ...]` appears last.
    // Last-wins returns "1.4" instead of "1.5".
    //
    // Note: BC-7.03.079 also has a v-prefix in its frontmatter (`version: "v1.5"`),
    // so 019a AND 019c fire together. The 019c test below uses bc_version "1.5" (bare)
    // to isolate the extraction bug from the v-prefix bug.
    // -----------------------------------------------------------------------

    /// F-P6-019c RED GATE: `[prior: vN.M]` annotation must not shadow the current version.
    ///
    /// Real BC: BC-7.03.079 (frontmatter `version: "v1.5"`; INDEX row field 6:
    ///   `v1.5 (D-838: POL-14 auto-promotion) [prior: v1.4 (D-837: invariant-1)]`).
    ///
    /// Bug: `extract_last_v_token` on field 6 finds `v1.5`, then `v1.5` again, then
    /// `v1.4` from `[prior: v1.4 (...]`. Last-wins returns "1.4".
    /// `index_version = "1.4"`, `bc_version = "1.5"` (v-prefix stripped for isolation).
    /// `bc (1,5) > index (1,4)` → `primary_newer = true` → false PC2a advisory.
    ///
    /// Fix: same as 019b — first-token-of-last-chain-entry (here: field 6 has no `\x00`
    /// separators; the last chain entry IS the whole field; first v-token = "1.5").
    ///
    /// RED GATE: `extract_bc_index_version_state` returns `Version("1.4")` instead of
    /// `Version("1.5")` → assert_eq!(result, BcIndexVersionState::Version("1.5")) FAILS.
    #[test]
    fn test_F_P6_019c_prior_annotation_returns_current_version() {
        // Synthetic row mirroring BC-7.03.079's INDEX shape.
        // The [prior: v1.4 (...)] suffix is a no-\| annotation style.
        let index = concat!(
            "| [BC-7.03.079](ss-07/BC-7.03.079.md) | track-agent-start: identity & ",
            "registry binding | active | TBD | S-8.08 | v1.5 (v1.5 2026-07-13 D-838: ",
            "POL-14 auto-promotion draft→active; input-hash 118ab49 UNCHANGED) ",
            "[prior: v1.4 (2026-07-13 D-837: Invariant 1 tuple-scoping per architect ",
            "ruling; anchored ^Agent$ cites; input-hash 118ab49)] |\n",
        );
        let result = extract_bc_index_version_state("BC-7.03.079", index.as_bytes());
        assert_eq!(
            result,
            BcIndexVersionState::Version("1.5".to_string()),
            "F-P6-019c: '[prior: v1.4]' annotation must NOT shadow the current version v1.5. \
            extract_bc_index_version_state must return Version(\"1.5\"), not Version(\"1.4\"). \
            Root cause: last-wins picks 'v1.4' from '[prior: v1.4 (...)]'. \
            Fix: first-token-of-last-chain-entry (same root cause as F-P6-019b). \
            Got: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // F-P6-019d — unescaped `|` in version annotation displaces field 6 (BC-4.13.001)
    //
    // Root cause: the escape-aware split converts `\|` → `\x00` but leaves bare `|`
    // characters untouched. BC-4.13.001's v1.16 changelog annotation contains
    // `^(Edit|Write|MultiEdit|Agent)$` — four bare `|` chars inside the version history
    // field. These create phantom field boundaries when the row is split on `|`, shifting
    // the actual version entries (v1.17, v1.18) into fields 7–9, beyond the field-6 window
    // that `extract_bc_index_version_state` reads.
    //
    // Result: field 6 contains only the beginning of the chain up to the first bare `|` in
    // the v1.16 annotation. `extract_last_v_token` on that truncated field 6 returns "1.16"
    // (the last version token before the phantom boundary). bc_version = "1.18".
    // "1.16" ≠ "1.18"; bc (1,18) > index (1,16) → primary_newer = true → false PC2a advisory.
    //
    // Fix: escape ALL bare `|` within cell content before splitting (full escape-aware
    // parsing that handles unescaped pipes in markdown table cells — possibly switching
    // to a raw substring search like bc_index_row_contains_version).
    // -----------------------------------------------------------------------

    /// ADR-038 §Decision 4 pinning test — DEFENSIVE `fields[5..].join("|")` reassembly arm (n>6).
    ///
    /// Motivation: The PO-adjudicated `fields[5..].join("|")` join (F-P6-019d) is retained
    /// for future bare-pipe annotation rows. As of S-21.07 pass-8, 0 live corpus rows produce
    /// n>6 (histogram: {5: 1945, 6: 40, >6: 0}). This pinning test prevents silent regression
    /// if the defensive arm is ever removed or narrowed.
    ///
    /// Teeth: without the join, `non_empty_fields[5]` alone ends at the first bare `|` inside
    /// the annotation `(BAD|v1.13)`, leaving only `"v1.10 \x00 (BAD"` — the last `\x00`-segment
    /// `" (BAD"` contains no v-token, so the function would return `RowPresentNoVersion`.
    /// With the join, `fields[5..].join("|")` = `"v1.10 \x00 (BAD|v1.13)"` — the last
    /// `\x00`-segment is `" (BAD|v1.13)"` and `extract_first_v_token` returns `"1.13"`.
    ///
    /// Fixture: a synthetic BC-INDEX row where the version chain is `v1.10 \| (BAD|v1.13)`.
    /// The escaped `\|` is the chain separator (→ `\x00`). The bare `|` inside `(BAD|v1.13)`
    /// is the annotation text that fragments the field. n=7 after escape-aware split.
    #[test]
    fn test_BC_5_39_010_arm_a1_defensive_reassembly_n_gt_6_extracts_correct_version() {
        // Row: version chain "v1.10 \| (BAD|v1.13)" where \\| = escaped chain separator.
        // The bare | after (BAD displaces v1.13 into fields[6].
        // fields[5..].join("|") reassembles the full cell; extract_first_v_token_of_last_entry
        // finds v1.13 in the last \x00-segment " (BAD|v1.13)".
        let index = concat!(
            "| [BC-PINTEST.001](ss-00/BC-PINTEST.001.md)",
            " | Title | desc | active | S-21.07",
            " | v1.10 \\| (BAD|v1.13) |\n",
        );
        let result = extract_bc_index_version_state("BC-PINTEST.001", index.as_bytes());
        assert_eq!(
            result,
            BcIndexVersionState::Version("1.13".to_string()),
            "ADR-038 §Decision 4: DEFENSIVE reassembly arm (fields[5..].join(\"|\")) MUST \
            recover v-token displaced by bare | into fields[6+]. Without the join, fields[5] \
            ends at '(BAD' and the last \\x00-segment contains no v-token → RowPresentNoVersion. \
            With the join the reconstructed cell yields Version(\"1.13\"). Got: {:?}",
            result
        );
    }

    /// F-P6-019d RED GATE: unescaped `|` in version annotation must not displace field 6.
    ///
    /// Real BC: BC-4.13.001 (frontmatter `version: "1.18"`; INDEX row v1.16 annotation
    ///   contains `^(Edit|Write|MultiEdit|Agent)$` with four unescaped `|` chars).
    ///
    /// Bug: escape-aware split converts `\|` → `\x00` but not bare `|`. The four bare `|`
    /// in `^(Edit|Write|MultiEdit|Agent)$` fragment field 6 at the first bare `|`, so field 6
    /// ends at `^(Edit`. `extract_last_v_token` on the truncated field 6 returns "1.16".
    /// `index_version = "1.16"`, `bc_version = "1.18"`. `bc (1,18) > index (1,16)` →
    /// false PC2a advisory.
    ///
    /// RED GATE: `extract_bc_index_version_state` returns `Version("1.16")` instead of
    /// `Version("1.18")` → the assert_eq!(result, BcIndexVersionState::Version("1.18")) FAILS.
    #[test]
    fn test_F_P6_019d_unescaped_pipe_in_annotation_must_not_displace_version_field() {
        // Synthetic row mirroring BC-4.13.001's structure:
        // v1.16 annotation contains ^(Edit|Write|MultiEdit|Agent)$ — 3 bare `|` inside.
        // v1.17 and v1.18 appear after the annotation in the same version-history cell.
        // After escape-aware split on bare `|`, field 6 is truncated at the first unescaped
        // `|` inside the annotation, cutting off v1.17 and v1.18 from field 6.
        let index = concat!(
            "| [BC-4.13.001](ss-04/BC-4.13.001.md) | verify-factory-lock guard | ",
            "active | CAP-031 | S-19.02, S-19.07 | v1.15 \\| v1.16 (v1.16 D-837: Invariant 5 ",
            "TOML snippet anchored — primary entry ^(Edit|Write|MultiEdit|Agent)$; ",
            "Bash arm ^Bash$; input-hash 14c1190) \\| v1.17 (v1.17 D-853: Phase-B-active; ",
            "input-hash ddbfdc2) \\| v1.18 (v1.18 D-853: W3G-001+W3G-002 closure; ",
            "EC-018/EC-019 added; input-hash c3ce066) |\n",
        );
        let result = extract_bc_index_version_state("BC-4.13.001", index.as_bytes());
        assert_eq!(
            result,
            BcIndexVersionState::Version("1.18".to_string()),
            "F-P6-019d: unescaped `|` in v1.16 annotation must NOT displace the version field. \
            extract_bc_index_version_state must return Version(\"1.18\"), not Version(\"1.16\"). \
            Root cause: escape-aware split handles \\| but not bare | — unescaped `|` chars in \
            '^(Edit|Write|MultiEdit|Agent)$' create phantom field boundaries, truncating field 6 \
            before v1.17 and v1.18 are reached. Fix: escape-unaware field splitting cannot handle \
            this — switch to bc_index_row_contains_version raw-substring strategy, or pre-escape \
            bare `|` inside markdown table cells before splitting. \
            Got: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // BC-5.39.010 v1.22 / ADV-RECON11-001: precondition 15b / postcondition 26
    // — Secondary Index-File UTF-8 Decode Failure (Arm A1 BC-INDEX.md secondary
    // read). DISTINCT from precondition 15a / postcondition 25 (primary-target
    // decode failure — BLOCK); this clause governs the secondary BC-INDEX.md
    // read only, and is ADVISORY (Continue), not block.
    //
    // RED GATE: `extract_bc_index_version_state` currently does
    // `std::str::from_utf8(index_content).unwrap_or("")` — non-UTF-8 bytes
    // silently decode to "" (empty string), yielding zero candidate lines →
    // `BcIndexVersionState::RowAbsent`. For a BC with frontmatter `version:` >
    // "1.0", `RowAbsent` routes to postcondition 4's BLOCK path ("previous
    // registration appears to have been dropped") — a MISLEADING message,
    // since the true root cause is index-file corruption (undecodable bytes),
    // not a dropped registration. Postcondition 26 instead requires a DISTINCT
    // advisory naming BC-INDEX.md and stating the row state is INDETERMINATE,
    // not confirmed-absent, with `HookResult::Continue` — no block.
    // -----------------------------------------------------------------------

    /// BC-5.39.010 precondition 15b / postcondition 26 (v1.22 / ADV-RECON11-001):
    /// non-UTF-8 BC-INDEX.md secondary read must emit the distinct INDETERMINATE
    /// advisory + Continue, NOT the misleading `RowAbsent` → postcondition 4 BLOCK.
    #[test]
    fn test_BC_5_39_010_arm_a1_non_utf8_bc_index_indeterminate_advisory_not_block() {
        // BC-INDEX.md "read" succeeds as bytes (Ok(...)) but the bytes are not
        // valid UTF-8 (lone continuation/invalid leading bytes 0xFF/0xFE mixed
        // with a plausible-looking pipe-table fragment so a naive lossy decode
        // couldn't accidentally "fix" it into a matching row either).
        let non_utf8_bytes: Vec<u8> = vec![0xFF, 0xFE, 0xFD, 0x80, 0x81, b'|', b'B', b'C'];
        let (violations, advisories) = run_arm_a1_with_index_result(
            "BC-5.39.010",
            "1.6",
            ".factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md",
            Ok(non_utf8_bytes),
        );

        assert!(
            violations.is_empty(),
            "postcondition 26 (v1.22): non-UTF-8 BC-INDEX.md secondary read MUST NOT \
            block — MUST NOT fall through to RowAbsent's postcondition-4 BLOCK path \
            ('previous registration appears to have been dropped'). \
            RED GATE: current code decodes non-UTF-8 bytes via unwrap_or(\"\") → zero \
            candidate lines → RowAbsent → BLOCK for BC version \"1.6\" (> \"1.0\"). \
            Actual violations: {violations:?}"
        );
        assert!(
            !advisories.is_empty(),
            "postcondition 26 (v1.22): non-UTF-8 BC-INDEX.md secondary read MUST emit \
            a distinct INDETERMINATE advisory + Continue. Actual advisories: {advisories:?}"
        );

        // BC-5.39.010 §PC4a (NORMATIVE): test-writer MUST assert the COMPLETE
        // formatted string by equality check; `.contains()`-only on substrings is
        // NON-CONFORMING. Build the COMPLETE prescribed postcondition 26 message
        // verbatim from the BC body (§Postconditions, postcondition 26) with
        // `<index-file>` = "BC-INDEX.md" and `<id>` = "BC-5.39.010" substituted,
        // then assert full-string equality — this also asserts the second
        // sentence ("Fix: verify the index file's encoding and re-save as
        // UTF-8."), which a `.contains()`-only check on the first sentence would
        // leave a mutation free to delete or corrupt undetected.
        let expected_decoded = "validate-cross-site-correspondence: BC-INDEX.md failed UTF-8 \
            decode — row/hash state for 'BC-5.39.010' is INDETERMINATE, not confirmed-absent. \
            Fix: verify the index file's encoding and re-save as UTF-8."
            .to_string();
        assert_eq!(
            advisories.len(),
            1,
            "postcondition 26 (v1.22): exactly one distinct INDETERMINATE advisory \
            expected for the non-UTF-8 BC-INDEX.md secondary read. Actual advisories: \
            {advisories:?}"
        );
        assert_eq!(
            advisories[0].message, expected_decoded,
            "postcondition 26 (v1.22) prescribed verbatim message does not match by \
            full-string equality (BC-5.39.010 §PC4a: test-writer MUST assert the \
            COMPLETE formatted string by equality check, not `.contains()`-only). \
            Expected: {expected_decoded:?} Actual advisories: {advisories:?}"
        );
        assert!(
            !advisories[0]
                .message
                .contains("previous registration appears to have been dropped"),
            "postcondition 26 (v1.22): the non-UTF-8-decode advisory MUST NOT be the \
            misleading RowAbsent-derived 'dropped registration' block message — those \
            two dispositions (INDETERMINATE decode failure vs. genuinely-absent row in \
            a decodable index file) MUST NOT be conflated (precondition 15b). \
            Actual advisories: {advisories:?}"
        );
    }

    // -----------------------------------------------------------------------
    // F-S2107-RECON-005 (adversary observation): the leading/trailing `\b`
    // (word-boundary) checks in `extract_first_v_token` used
    // `is_ascii_alphanumeric()` only — omitting `_` — while `arm_b::is_word_char`
    // (F-S2107-RECON-004) correctly treats `_` as a word char. Two divergent
    // `\b` word-char definitions in the same crate. A preceding/following `_`
    // is a word char under regex `\b` semantics (`[A-Za-z0-9_]`), so
    // `\bv([0-9]+\.[0-9]+)\b` must NOT match immediately after/before `_`.
    // Pre-fix: the leading/trailing checks wrongly treated `_` as a boundary
    // and accepted the match. Post-fix: both checks route through the single
    // crate-shared `is_word_char` predicate (promoted from `arm_b`-private).
    // -----------------------------------------------------------------------

    /// F-S2107-RECON-005 RED GATE (leading): `extract_first_v_token` must reject a
    /// `v`-token immediately preceded by `_` (a word char under `\b`).
    ///
    /// Pre-fix: `!bytes[i-1].is_ascii_alphanumeric()` on `_` (0x5F) is `true` (wrongly
    /// treats `_` as a boundary) → matches → returns `Some("1.4")`.
    /// Post-fix: `_` is a word char via `is_word_char` → `prev_ok` is `false` → no
    /// other `v`-token in the fixture → returns `None`.
    #[test]
    fn test_BC_5_39_010_arm_a1_first_v_token_leading_underscore_rejected() {
        let result = extract_first_v_token("foo_v1.4");
        assert_eq!(
            result, None,
            "'foo_v1.4': the 'v' is immediately preceded by '_' — a word char under \
            regex \\b semantics — so \\bv([0-9]+\\.[0-9]+)\\b must NOT match at that \
            position, and there is no other v-token in the fixture. \
            F-S2107-RECON-005 RED GATE: pre-fix code treats '_' as a boundary and \
            wrongly returns Some(\"1.4\")."
        );
    }

    /// F-S2107-RECON-005 RED GATE (trailing): `extract_first_v_token` must reject a
    /// `v`-token immediately followed by `_` (a word char under `\b`).
    ///
    /// Pre-fix: `!bytes[end].is_ascii_alphanumeric()` on `_` (0x5F) is `true` (wrongly
    /// treats `_` as a boundary) → matches → returns `Some("1.4")`.
    /// Post-fix: `_` is a word char via `is_word_char` → `next_ok` is `false` → no
    /// other qualifying v-token in the fixture → returns `None`.
    #[test]
    fn test_BC_5_39_010_arm_a1_first_v_token_trailing_underscore_rejected() {
        let result = extract_first_v_token("v1.4_5");
        assert_eq!(
            result, None,
            "'v1.4_5': the digit run '1.4' is immediately followed by '_' — a word char \
            under regex \\b semantics — so \\bv([0-9]+\\.[0-9]+)\\b must NOT match, and \
            there is no other v-token in the fixture. \
            F-S2107-RECON-005 RED GATE: pre-fix code treats '_' as a boundary and \
            wrongly returns Some(\"1.4\")."
        );
    }

    /// F-S2107-RECON-005 positive control: leading space still qualifies as a boundary.
    #[test]
    fn test_BC_5_39_010_arm_a1_first_v_token_leading_space_control() {
        let result = extract_first_v_token("foo v1.4");
        assert_eq!(
            result,
            Some("1.4".to_string()),
            "'foo v1.4': ' ' before 'v' is not a word char — \\b holds — must still match"
        );
    }

    /// F-S2107-RECON-005 positive control: leading pipe/period/start-of-string and
    /// trailing space/EOL all still qualify as boundaries (unaffected by the fix).
    #[test]
    fn test_BC_5_39_010_arm_a1_first_v_token_non_underscore_boundaries_control() {
        let cases: &[(&str, &str)] = &[
            ("v1.4", "1.4"),          // start-of-string leading boundary
            ("|v1.4", "1.4"),         // pipe leading boundary
            (".v1.4", "1.4"),         // period leading boundary
            ("v1.4 trailing", "1.4"), // trailing space boundary
            ("v1.4.", "1.4"),         // trailing period boundary (non-word char)
            ("v1.4", "1.4"),          // trailing EOL boundary
        ];
        for (input, expected) in cases {
            let result = extract_first_v_token(input);
            assert_eq!(
                result,
                Some((*expected).to_string()),
                "non-underscore boundary case {input:?} must still match — regression \
                control for F-S2107-RECON-005 fix"
            );
        }
    }
}
