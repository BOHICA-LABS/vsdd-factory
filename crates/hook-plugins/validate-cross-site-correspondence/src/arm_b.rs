//! arm_b.rs — Class B: STORY-INDEX.md three-way input-hash correspondence.
//!
//! Effectful-shell module (ADR-035 §Decision 1): reads STORY-INDEX.md via
//! `host::read_file` when processing story file writes (Arm B1).
//!
//! # Two sub-arms
//!
//! ## B1 — story file write trigger
//! Fires when a story file is written. Extracts B1 (story frontmatter `input-hash:`),
//! reads STORY-INDEX.md to extract B2 (catalog row hash) and B3 (blockquote hash).
//! Blocks if any pair disagrees. Advisory if B2/B3 are absent (new story, not yet
//! registered in STORY-INDEX.md).
//!
//! ## B2 — STORY-INDEX.md write trigger
//! Fires when STORY-INDEX.md is written. Parses all story IDs and hash tokens from
//! the catalog rows and aggregation blockquote within the file itself. Blocks if any
//! catalog hash disagrees with the corresponding blockquote hash (cascade).
//!
//! # Block message invariant (BC-5.39.010 invariant 11)
//! The B1 block message MUST include a provenance note distinguishing:
//! - `stale` — B2/B3 likely just need a `--update` sweep
//! - `fabricated` — B1 disagrees with B3 but agrees with B2, suggesting B1 was
//!   manually edited without updating the story file
//!
//! # BC trace
//! BC-5.39.010 Class B preconditions, postconditions, and invariant 11 (current
//! version per BC-INDEX) — Arm B1 three-way B1/B2/B3 input-hash comparison with
//! PC13a/PC13b directional carve-out and the PC40 volatile-input carve-out, Arm B2
//! internal catalog-vs-blockquote cascade (postcondition 15, invariant 11 provenance
//! taxonomy), and the v1.22 Arm B1 secondary-index UTF-8 decode-failure path
//! (precondition 15b / postcondition 26; Arm B2 explicitly excluded — already
//! primary-target-governed by precondition 15a / postcondition 25).

use crate::{Advisory, Violation};
use vsdd_hook_sdk::host::HostError;

/// Read cap for STORY-INDEX.md secondary read (B1 story write trigger).
/// BC-5.39.010 AC-019: `max_bytes = 1048576`, `timeout_ms = 3000`.
pub const STORY_INDEX_B1_MAX_BYTES: u32 = 1_048_576;
pub const STORY_INDEX_B1_TIMEOUT_MS: u32 = 3_000;

/// Read cap for STORY-INDEX.md when it is the primary target (B2 write trigger).
/// BC-5.39.010 AC-019: `max_bytes = 2097152`, `timeout_ms = 5000`.
pub const STORY_INDEX_B2_MAX_BYTES: u32 = 2_097_152;
pub const STORY_INDEX_B2_TIMEOUT_MS: u32 = 5_000;

/// Extract the `input-hash:` value from a story file's frontmatter.
///
/// Returns `None` if the field is absent (Arm B1 skips entirely per
/// BC-5.39.010 precondition 18 second clause).
///
/// Pure: no I/O.
///
/// # BC trace
/// BC-5.39.010 precondition 18 (input-hash: field presence check).
pub fn parse_story_input_hash(story_content: &str) -> Option<String> {
    crate::frontmatter::extract_frontmatter_field(story_content, "input-hash")
}

/// Extract the input-hash from the STORY-INDEX.md catalog row for `story_id`.
///
/// Scans the catalog table in STORY-INDEX.md for a row containing `story_id`
/// and extracts the `input-hash <hash>` token from that row.
/// Returns `None` if no row is found for this story ID (new story not yet indexed).
///
/// Pure: operates on already-decoded content. Takes `&str` (not `&[u8]`) — callers
/// decode STORY-INDEX.md's bytes ONCE at the orchestration entry point
/// (`run_arm_b1_with_index_result`) and thread the resulting `&str` into both this
/// function and `parse_story_index_blockquote_hash`, rather than each leaf parser
/// independently re-decoding the same bytes (ADV-RECON12-002: avoids 3x O(n) UTF-8
/// decode of a ≤1 MiB artifact on the Arm B1 path).
///
/// # BC trace
/// BC-5.39.010 precondition 19 (B2 catalog row extraction).
pub fn parse_story_index_catalog_hash(index_content: &str, story_id: &str) -> Option<String> {
    // F-S2107-P1B-008: naive `contains(story_id)` matches rows WHERE story_id appears
    // in depends_on/blocks columns (e.g. S-18.00's row contains "[S-18.01]"), returning
    // the wrong row's hash. PC16: catalog lookup must anchor on the FIRST pipe-delimited
    // cell — the row whose first cell is exactly the story_id.
    for line in index_content.lines() {
        // Must be a table row (starts with |)
        if !line.starts_with('|') {
            continue;
        }
        // First cell must be exactly the story_id (trim surrounding whitespace)
        let mut cells = line.split('|');
        cells.next(); // skip leading empty segment before first '|'
        let first_cell = cells.next().map(|c| c.trim()).unwrap_or("");
        if first_cell != story_id {
            continue;
        }
        // Extract `input-hash <hash>` from this row
        if let Some(hash) = extract_input_hash_token(line) {
            return Some(hash);
        }
    }
    None
}

/// Extract the input-hash from the STORY-INDEX.md aggregation blockquote for `story_id`.
///
/// Scans the aggregation blockquote (`> S-21.07=47a65c9`) in STORY-INDEX.md for
/// an entry matching `story_id`. Returns `None` if not found.
///
/// Pure: operates on already-decoded content. Takes `&str` (not `&[u8]`) — see
/// `parse_story_index_catalog_hash`'s doc for why (ADV-RECON12-002 decode-once refactor).
///
/// # BC trace
/// BC-5.39.010 precondition 20 (B3 blockquote extraction).
pub fn parse_story_index_blockquote_hash(index_content: &str, story_id: &str) -> Option<String> {
    // F-S2107-P1B-003: production STORY-INDEX blockquote is ONE prose line with all hashes
    // embedded as `S-XX.YY=HHHHHHH` tokens (not one line per story). PC21 specifies a
    // WITHIN-line search for `\b<id>=([0-9a-f]{7,40})\b` on `^> ` lines.
    let needle = format!("{}=", story_id);

    for line in index_content.lines() {
        if !line.starts_with("> ") {
            continue;
        }
        // Search for `story_id=HHHHHHH` anywhere within the line, with word boundary before.
        let mut search_start = 0;
        while search_start < line.len() {
            let search_in = &line[search_start..];
            if let Some(rel_pos) = search_in.find(&needle) {
                let abs_pos = search_start + rel_pos;
                // Word boundary before story_id: preceding char must not be
                // a word char (F-S2107-RECON-004/005: shares `crate::is_word_char`
                // with the trailing boundary check — same `\b` definition).
                let wb_ok = abs_pos == 0 || {
                    let prev = line[..abs_pos].chars().last().unwrap_or('\0');
                    !crate::is_word_char(prev)
                };
                if wb_ok {
                    let hash_start = abs_pos + needle.len();
                    if hash_start <= line.len() {
                        // Extract hex-only token bounded to 7..=40 chars (PC21).
                        // PC21 specifies [0-9a-f]{7,40} — lowercase only. This is
                        // deliberate and spec-mandated (BC-5.39.010 PC21); do not
                        // widen to is_ascii_hexdigit().
                        let hash: String = line[hash_start..]
                            .chars()
                            .take_while(|c| matches!(c, '0'..='9' | 'a'..='f'))
                            .collect();
                        // O-1 / PC21 trailing `\b`: the character immediately
                        // after the hex run must be absent (EOL) or a
                        // non-word character. All hex chars are ASCII, so
                        // `hash_start + hash.len()` is a valid byte boundary.
                        if hash.len() >= 7
                            && hash.len() <= 40
                            && trailing_word_boundary_ok(line, hash_start + hash.len())
                        {
                            return Some(hash);
                        }
                    }
                }
                search_start = abs_pos + 1;
            } else {
                break;
            }
        }
    }
    None
}

/// Arm B1 check with the STORY-INDEX.md read result as a seam.
///
/// Pure-seam entry point for unit testing.
///
/// Returns `(violations, advisories)`:
/// - Block: B2 ≠ B1 (catalog disagrees with story frontmatter).
/// - Block: B3 ≠ B1 (blockquote disagrees with story frontmatter).
/// - Block: CapabilityDenied on STORY-INDEX.md read (invariant 5 — sandbox
///   misconfiguration on secondary target is blocking per BC-5.39.010 PC26).
/// - Advisory: NotFound on STORY-INDEX.md (bootstrap ordering).
/// - Advisory: B2 or B3 absent (new story not yet in STORY-INDEX.md).
///
/// # BC trace
/// BC-5.39.010 Class B Arm B1 postconditions and preconditions (current version per
/// BC-INDEX) — three-way B1/B2/B3 comparison with PC13a/PC13b directional carve-out
/// (postconditions 12/13), precondition 26 (STORY-INDEX.md secondary-read
/// CapabilityDenied/NotFound disposition, invariant 11), and the v1.22
/// secondary-index UTF-8 decode-failure path (precondition 15b / postcondition 26 —
/// a distinct clause from precondition 26 above; v1.22 / ADV-RECON11-001).
pub fn run_arm_b1_with_index_result(
    story_id: &str,
    story_hash: &str,
    index_read_result: Result<Vec<u8>, HostError>,
) -> (Vec<Violation>, Vec<Advisory>) {
    let mut violations = Vec::new();
    let mut advisories = Vec::new();

    match index_read_result {
        Err(HostError::CapabilityDenied) => {
            // BC-5.39.010 PC26: CapabilityDenied on secondary target → block
            // Invariant 5: sandbox misconfiguration is never legitimate
            violations.push(Violation {
                description: format!(
                    "validate-cross-site-correspondence [Class B] POLICY 18: \
                    STORY-INDEX.md read denied (CapabilityDenied) for story {story_id} \
                    — sandbox misconfiguration must be investigated \
                    (stale registry entry or capability not granted)"
                ),
            });
        }
        Err(HostError::NotFound) => {
            // STORY-INDEX.md not found — bootstrap ordering, advisory only
            advisories.push(Advisory {
                message: format!(
                    "validate-cross-site-correspondence [Class B] advisory: \
                    STORY-INDEX.md not found during B1 check for {story_id} \
                    — bootstrap ordering: index may not exist yet"
                ),
            });
        }
        Err(other) => {
            // Other host errors → block (fail-closed)
            violations.push(Violation {
                description: format!(
                    "validate-cross-site-correspondence [Class B] POLICY 18: \
                    STORY-INDEX.md read error ({other:?}) for story {story_id} \
                    — cannot verify hash correspondence (stale read or host fault)"
                ),
            });
        }
        Ok(ref bytes) => {
            // BC-5.39.010 precondition 15b / postcondition 26 (v1.22 / ADV-RECON11-001):
            // STORY-INDEX.md is a SECONDARY read target at Arm B1 (not Arm B2, which is
            // already governed by precondition 15a / postcondition 25's primary-target
            // BLOCK). A decode failure here is genuinely INDETERMINATE, not confirmed-
            // absent — it MUST NOT be allowed to silently degrade into
            // `parse_story_index_catalog_hash`/`parse_story_index_blockquote_hash`'s
            // `.ok()?` -> `None` fallback, which is indistinguishable from a genuinely
            // new, not-yet-registered story in a DECODABLE index file and would
            // otherwise fall through into the generic `(None, None)` "not yet
            // registered" fail-open advisory below — silently disabling three-way hash
            // checking with no disclosure that the actual root cause is an undecodable
            // STORY-INDEX.md. Checked once here, at the orchestration entry point,
            // before either leaf parser runs.
            //
            // ADV-RECON12-002: decode once and thread the resulting `&str` into both
            // leaf parsers below, rather than each independently re-decoding `bytes`
            // via its own `.ok()?` — avoids 3x O(n) UTF-8 decode of a ≤1 MiB artifact.
            let content = match std::str::from_utf8(bytes) {
                Ok(content) => content,
                Err(_) => {
                    advisories.push(Advisory {
                        message: format!(
                            "validate-cross-site-correspondence: STORY-INDEX.md failed UTF-8 \
                            decode — row/hash state for '{story_id}' is INDETERMINATE, not \
                            confirmed-absent. Fix: verify the index file's encoding and \
                            re-save as UTF-8."
                        ),
                    });
                    return (violations, advisories);
                }
            };

            let catalog_hash = parse_story_index_catalog_hash(content, story_id);
            let blockquote_hash = parse_story_index_blockquote_hash(content, story_id);

            match (catalog_hash, blockquote_hash) {
                (None, None) => {
                    // Story not yet registered in STORY-INDEX.md — advisory only (new story)
                    advisories.push(Advisory {
                        message: format!(
                            "validate-cross-site-correspondence [Class B] advisory: \
                            story {story_id} has input-hash in frontmatter but is not yet \
                            registered in STORY-INDEX.md — run compute-input-hash --update \
                            after indexing"
                        ),
                    });
                }
                (Some(b2), Some(b3)) => {
                    // BC-5.39.010 §PC13: directional two sub-cases.
                    // PC13a: B2==B3 AND B1!=B2 — STORY-INDEX internally consistent;
                    //   story frontmatter stale (burst-ordering artefact). Advisory + Continue.
                    // PC13b: B2!=B3 — STORY-INDEX internally inconsistent; anomalous block.
                    // When B1==B2==B3: all three agree — no action.
                    if b2 == story_hash && b3 == story_hash {
                        // All three sites agree — no action.
                    } else if b2 == b3 {
                        // PC13a: STORY-INDEX internally consistent (B2==B3), story frontmatter
                        // differs. Burst-ordering artefact: story was just written; state-manager
                        // STORY-INDEX update is pending (POLICY 3). Advisory + Continue.
                        advisories.push(Advisory {
                            message: format!(
                                "validate-cross-site-correspondence [Class B] advisory: \
                                Story {story_id} input-hash mismatch — \
                                frontmatter={story_hash}; \
                                STORY-INDEX-catalog={b2}; \
                                STORY-INDEX-blockquote={b3}. \
                                STORY-INDEX sites agree with each other; story frontmatter \
                                differs. State-manager STORY-INDEX update pending; \
                                Class B BLOCK suspended."
                            ),
                        });
                    } else {
                        // PC13b: STORY-INDEX internally inconsistent (B2!=B3) — anomalous.
                        // No burst-ordering argument explains internal STORY-INDEX
                        // inconsistency: catalog row and blockquote are written in the
                        // same state-manager commit. Block with three-provenance message.
                        violations.push(Violation {
                            description: format!(
                                "validate-cross-site-correspondence [Class B]: \
                                Story {story_id} input-hash three-way mismatch: \
                                frontmatter={story_hash} STORY-INDEX-catalog={b2} \
                                STORY-INDEX-blockquote={b3}. \
                                STORY-INDEX catalog and blockquote disagree — \
                                this is anomalous and has no burst-ordering explanation. \
                                Update per POLICY 18 (D-923). \
                                This hook detects inconsistency only — operator MUST \
                                determine which of the following applies before \
                                remediating: \
                                (a) STALE: previously valid hash; inputs changed after \
                                authoring; remedy: rerun `compute-input-hash --update` \
                                on the story. \
                                (b) FABRICATED: hash was never output of \
                                `compute-input-hash --update` at any revision \
                                (POLICY 18 violation); remedy: acknowledge \
                                PROVENANCE-BREAK in burst-log before recomputing. \
                                (c) ALGORITHM-DIVERGENT: hash produced by prior binary \
                                version per ADR-036 §Decision 4; NOT fabricated; \
                                remedy: recompute with current authoritative binary, \
                                no PROVENANCE-BREAK annotation required."
                            ),
                        });
                    }
                }
                (Some(b2), None) => {
                    // Only catalog row present — no blockquote entry yet.
                    // ADR-038 §Decision 3: half-present is ALWAYS advisory + Continue per PC12
                    // ("B2 or B3 absent → advisory + Continue"). The hash comparison is
                    // irrelevant: the absent-site state is indistinguishable from mid-burst
                    // write ordering at trigger time (state-manager writes catalog and blockquote
                    // in the same STORY-INDEX commit). Blocking here creates a live self-lock.
                    if b2 != story_hash {
                        advisories.push(Advisory {
                            message: format!(
                                "validate-cross-site-correspondence [Class B] POLICY 18: \
                                input-hash mismatch for story {story_id} \
                                — story={story_hash} catalog={b2} blockquote=absent \
                                — stale — update both STORY-INDEX.md catalog and blockquote"
                            ),
                        });
                    } else {
                        advisories.push(Advisory {
                            message: format!(
                                "validate-cross-site-correspondence [Class B] advisory: \
                                story {story_id} catalog row present but blockquote entry absent \
                                in STORY-INDEX.md — run compute-input-hash --update"
                            ),
                        });
                    }
                }
                (None, Some(b3)) => {
                    // Only blockquote entry present — no catalog row yet.
                    // ADR-038 §Decision 3: half-present is ALWAYS advisory + Continue per PC12
                    // ("B2 or B3 absent → advisory + Continue"). The hash comparison is
                    // irrelevant: the absent-site state is indistinguishable from mid-burst
                    // write ordering at trigger time (state-manager writes catalog and blockquote
                    // in the same STORY-INDEX commit). Blocking here creates a live self-lock.
                    if b3 != story_hash {
                        advisories.push(Advisory {
                            message: format!(
                                "validate-cross-site-correspondence [Class B] POLICY 18: \
                                input-hash mismatch for story {story_id} \
                                — story={story_hash} catalog=absent blockquote={b3} \
                                — stale — update both STORY-INDEX.md catalog and blockquote"
                            ),
                        });
                    } else {
                        advisories.push(Advisory {
                            message: format!(
                                "validate-cross-site-correspondence [Class B] advisory: \
                                story {story_id} blockquote entry present but catalog row absent \
                                in STORY-INDEX.md — run compute-input-hash --update"
                            ),
                        });
                    }
                }
            }
        }
    }

    (violations, advisories)
}

/// Volatile filename set for the `.factory/cycles/**` family of patterns.
///
/// ADR-037 §Decision 2 defines **six canonical patterns**; patterns 2–5
/// are the `.factory/cycles/**/<named-file>` family, covering four specific
/// filenames. Those four expand to four entries in this constant, giving
/// **eight concrete path forms** in the implementation (patterns 1, 2–5
/// expanded ×4, 6, 7, 8). Each is checked in order by `is_volatile_path`.
/// Paths NOT in this table are non-volatile and MUST NOT match.
///
/// | # | Pattern | Rationale |
/// |---|---------|-----------|
/// | 1 | `.factory/STATE.md` | pipeline state (direct child) |
/// | 2 | `.factory/cycles/**/STATE.md` | per-cycle state |
/// | 3 | `.factory/cycles/**/decision-log.md` | append-only cycle log |
/// | 4 | `.factory/cycles/**/lessons.md` | append-only cycle log |
/// | 5 | `.factory/cycles/**/burst-log.md` | append-only cycle log |
/// | 6 | `.factory/specs/architecture/ARCH-INDEX.md` | growing architecture catalog |
/// | 7 | `.factory/specs/behavioral-contracts/BC-INDEX.md` | growing BC catalog |
/// | 8 | `.factory/stories/STORY-INDEX.md` | growing story catalog |
///
/// Note: VP-INDEX.md is intentionally absent (not in ADR-037 §Decision 2).
/// Note: `.factory/cycles/**` any-file widening is intentionally absent (immutable
/// historical artifacts like adv-cycle-pass-N.md are not volatile).
const VOLATILE_PATTERNS_CYCLES_NAMED: [&str; 4] =
    ["STATE.md", "decision-log.md", "lessons.md", "burst-log.md"];

/// Returns `true` if `path` is a volatile factory path whose content changes
/// frequently enough that a stable input-hash cannot be maintained.
///
/// Implements ADR-037 §Decision 2 exactly — six canonical patterns per the
/// spec, expanded to eight concrete path forms in implementation (see
/// `VOLATILE_PATTERNS_CYCLES_NAMED` doc table above).
///
/// Pure: no I/O.
///
/// # BC trace
/// BC-5.39.010 §PC40: volatile-input precondition.
/// ADR-037 §Decision 2: canonical volatile path list.
/// F-S2107-P4-020: `starts_with` narrowing fixed to `contains` per spec predicate.
pub fn is_volatile_path(path: &str) -> bool {
    use std::path::Path;

    // Pattern 1: `.factory/STATE.md` — pipeline state, direct child only.
    if path == ".factory/STATE.md" {
        return true;
    }

    // Patterns 2–5: files under `.factory/cycles/**/`.
    // Only four named files are volatile; any other cycles/ file is an immutable
    // historical artifact and must NOT match.
    // BC-5.39.010 PC40: spec says "path **contains** `.factory/cycles/`" not starts_with —
    // repo-root-relative paths are the common case but `contains` is the conforming predicate.
    if path.contains(".factory/cycles/") {
        let filename = Path::new(path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("");
        return VOLATILE_PATTERNS_CYCLES_NAMED.contains(&filename);
    }

    // Patterns 6–8: path-equals for the three index files.
    // Filename-only matching is intentionally avoided: BC-INDEX.md under cycles/
    // must NOT match (only the canonical spec path is volatile).
    matches!(
        path,
        ".factory/specs/architecture/ARCH-INDEX.md"
            | ".factory/specs/behavioral-contracts/BC-INDEX.md"
            | ".factory/stories/STORY-INDEX.md"
    )
}

/// Extract the `inputs:` YAML sequence from a story file's frontmatter.
///
/// Returns the list of input paths declared in the frontmatter `inputs:` field.
/// Returns an empty `Vec` if the field is absent or the frontmatter is malformed.
///
/// Delegates to `crate::frontmatter::extract_frontmatter_sequence` which handles
/// both inline (`inputs: [a, b]`) and block (`inputs:\n  - a\n  - b`) YAML forms.
///
/// Pure: no I/O.
///
/// # BC trace
/// BC-5.39.010 §PC40: volatile-input precondition.
pub fn parse_story_volatile_inputs(content: &str) -> Vec<String> {
    crate::frontmatter::extract_frontmatter_sequence(content, "inputs")
}

/// Arm B1 effectful entry point.
///
/// Reads STORY-INDEX.md via `host::read_file` (`max_bytes = 1048576`,
/// `timeout_ms = 3000`), then delegates to `run_arm_b1_with_index_result`.
///
/// BC-5.39.010 §PC40: if the story's `inputs:` list contains any volatile
/// paths (STATE.md, INDEX files, cycles/ artifacts), emits advisory + Continue
/// without performing the three-way hash comparison.
///
/// Called from `on_post_tool_use` when a story file write is detected.
///
/// # BC trace
/// BC-5.39.010 §PC40 volatile-input precondition (skip three-way comparison when
/// declared inputs are volatile) and precondition 26 (STORY-INDEX.md secondary-read
/// CapabilityDenied/NotFound disposition, delegated to `run_arm_b1_with_index_result`).
pub fn run_arm_b1(story_id: &str, story_content: &str) -> (Vec<Violation>, Vec<Advisory>) {
    let story_hash = match parse_story_input_hash(story_content) {
        Some(h) => h,
        None => return (vec![], vec![]), // No input-hash: skip (PC18)
    };

    // PC40: if any declared input is volatile, skip the three-way comparison.
    // Volatile paths (STATE.md, INDEX files, cycles/**) change too frequently to
    // maintain stable hashes; emitting a block would be a false positive.
    let inputs = parse_story_volatile_inputs(story_content);
    let volatile_found: Vec<&str> = inputs
        .iter()
        .filter(|p| is_volatile_path(p))
        .map(|p| p.as_str())
        .collect();
    if !volatile_found.is_empty() {
        // ADR-037 §Decision 4 prescribed advisory text — transcribed verbatim.
        let advisory = Advisory {
            message: format!(
                "validate-cross-site-correspondence [Class B] advisory: \
                Story {story_id} has volatile inputs per ADR-037 §Decision 2 — \
                three-way equality is unsatisfiable until story-writer removes volatile \
                inputs and state-manager recomputes the hash; Class B BLOCK suspended. \
                Volatile path(s): {volatile_found:?}"
            ),
        };
        return (vec![], vec![advisory]);
    }

    let index_result = vsdd_hook_sdk::host::read_file(
        ".factory/stories/STORY-INDEX.md",
        STORY_INDEX_B1_MAX_BYTES,
        STORY_INDEX_B1_TIMEOUT_MS,
    );
    run_arm_b1_with_index_result(story_id, &story_hash, index_result)
}

/// Arm B2: check all story hashes within STORY-INDEX.md itself for internal
/// catalog vs blockquote consistency.
///
/// STORY-INDEX.md IS the primary target for Arm B2: any `HostError` on its read
/// blocks (BC-5.39.010 precondition 22 note). The content has already been read
/// by `on_post_tool_use` before dispatch.
///
/// Scans all story IDs in the catalog and compares each with the corresponding
/// blockquote entry. Reports ALL mismatches in ONE combined block (cascade —
/// postcondition 15).
///
/// Pure: operates on already-read content string.
///
/// # BC trace
/// BC-5.39.010 Class B Arm B2 preconditions and postconditions (current version per
/// BC-INDEX) — precondition 22 (STORY-INDEX.md IS the primary target; any HostError
/// blocks), postcondition 15 (cascade — all mismatches reported in one combined
/// block), invariant 11 (provenance taxonomy). Not in scope for the v1.22
/// secondary-index decode-failure amendment (precondition 15b / postcondition 26
/// explicitly excludes Arm B2 — already primary-target-governed by precondition 15a
/// / postcondition 25).
pub fn run_arm_b2(story_index_content: &str) -> Vec<Violation> {
    let mut violations = Vec::new();

    // Build catalog map: Vec<(story_id, catalog_hash)>
    let mut catalog: Vec<(String, String)> = Vec::new();
    // Build blockquote map: Vec<(story_id, blockquote_hash)>
    let mut blockquote: Vec<(String, String)> = Vec::new();

    for line in story_index_content.lines() {
        if line.starts_with('|') {
            // Catalog table row — extract story_id (first cell) and input-hash token
            if let (Some(story_id), Some(hash)) = (
                extract_story_id_from_table_row(line),
                extract_input_hash_token(line),
            ) {
                catalog.push((story_id, hash));
            }
        } else if line.starts_with("> ") {
            // F-S2107-P1B-004: production STORY-INDEX blockquote is ONE prose line with
            // multiple `S-XX.YY=HHHHHHH` tokens. The current single-`find('=')` approach
            // grabs the whole prose prefix as story_id. Fix: extract ALL id=hash pairs.
            for (story_id, hash) in extract_blockquote_pairs(line) {
                blockquote.push((story_id, hash));
            }
        }
    }

    // Compare catalog→blockquote direction (BC-5.39.010 PC22 note: "scans all story IDs
    // in the catalog"). This direction correctly ignores blockquote entries for stories
    // not in the catalog (they belong to other wave aggregations).
    for (cat_story_id, cat_hash) in &catalog {
        let bq_hash = blockquote
            .iter()
            .find(|(id, _)| id == cat_story_id)
            .map(|(_, h)| h.as_str());

        if let Some(bq_h) = bq_hash
            && bq_h != cat_hash
        {
            violations.push(Violation {
                description: format!(
                    "validate-cross-site-correspondence [Class B] POLICY 18: \
                    STORY-INDEX.md internal parity violation for story {cat_story_id} \
                    — catalog={cat_hash} blockquote={bq_h} \
                    — run `compute-input-hash --update`"
                ),
            });
            // If they match: no violation (correct)
        }
        // If bq_hash is None (story not in blockquote): no violation — blockquote may
        // aggregate multiple waves; absence is not an error for Arm B2.
    }

    violations
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Extract `input-hash <hex>` token from a table row or any content line.
///
/// Returns the hex hash string, or `None` if not present or not valid hex.
///
/// F-S2107-P1B-009: PC20 requires `\binput-hash\s+([0-9a-f]{7,40})\b`.
/// - `is_ascii_alphanumeric()` accepted non-hex tokens like "bonus" (has 'o', 'n', 'u', 's').
/// - Single space needle missed multi-space forms (`\s+`).
/// - No retry past a non-conforming first match (live STORY-INDEX has "convention",
///   "mismatch", "bonus", "updated" at 6+ sites).
///
/// Fix: validate hex-only, bound to {7,40}, allow `\s+`, retry past bad matches.
fn extract_input_hash_token(line: &str) -> Option<String> {
    let keyword = "input-hash";
    let mut search_start = 0;

    while search_start < line.len() {
        let search_in = &line[search_start..];
        let Some(rel_pos) = search_in.find(keyword) else {
            break;
        };
        let pos = search_start + rel_pos;

        // Word boundary before "input-hash" (F-S2107-RECON-004/005: shares
        // `crate::is_word_char` with the trailing boundary check — same `\b`
        // definition).
        let wb_before_ok = pos == 0 || {
            let prev = line[..pos].chars().last().unwrap_or('\0');
            !crate::is_word_char(prev)
        };
        let after_keyword = pos + keyword.len();
        if !wb_before_ok || after_keyword >= line.len() {
            search_start = pos + 1;
            continue;
        }

        // Skip \s+ (at least one whitespace required — PC20)
        let mut hash_start = after_keyword;
        while hash_start < line.len()
            && (line.as_bytes()[hash_start] == b' ' || line.as_bytes()[hash_start] == b'\t')
        {
            hash_start += 1;
        }
        if hash_start == after_keyword {
            // No whitespace found — not a valid "input-hash \s+" pattern
            search_start = pos + 1;
            continue;
        }

        // Extract hex-only token bounded to {7,40} chars
        let hash: String = line[hash_start..]
            .chars()
            .take_while(|c| matches!(c, '0'..='9' | 'a'..='f'))
            .collect();

        // O-1 / PC20 trailing `\b`: the character immediately after the hex
        // run must be absent (EOL) or a non-word character. All hex chars
        // are ASCII, so `hash_start + hash.len()` is a valid byte boundary.
        if hash.len() >= 7
            && hash.len() <= 40
            && trailing_word_boundary_ok(line, hash_start + hash.len())
        {
            return Some(hash);
        }
        // Non-conforming token (length out of {7,40} range, OR the PC20
        // trailing \b fails): retry past this occurrence.
        // F-S2107-RECON-001: `hash_start + 1` is NOT safe — `hash` may be empty
        // (zero hex chars matched) when the char at `hash_start` is a multibyte
        // UTF-8 char, in which case `hash_start + 1` lands mid-char and the next
        // iteration's `&line[search_start..]` slice panics (BC-5.39.010
        // invariant 9). Step forward by the actual byte length of the char at
        // `hash_start` instead — always lands on a valid boundary.
        let step = line[hash_start..].chars().next().map_or(1, char::len_utf8);
        search_start = hash_start + step;
    }
    None
}

/// PC20/PC21 trailing `\b` (word-boundary) check.
///
/// Returns `true` if the byte position `pos` in `s` is at or past the end of
/// `s` (end-of-line — always a boundary), or the character starting at `pos`
/// is NOT a word character (see `crate::is_word_char`).
///
/// Shared by all three PC20/PC21 hex-run extractors (`extract_input_hash_token`,
/// `extract_blockquote_pairs`, `parse_story_index_blockquote_hash`) to enforce
/// the trailing `\b` in BC-5.39.010 PC20 (`\binput-hash\s+([0-9a-f]{7,40})\b`)
/// and PC21 (`\b<id>=([0-9a-f]{7,40})\b`). A hex-run candidate immediately
/// followed by another word character (e.g. uppercase `D` in `47a65c9D`, or
/// `g` in `47a65c9abcg`) fails `\b` — no regex match exists at that position
/// at all, since every shorter backtrack length within the same contiguous
/// run is still followed by a word character — so the candidate MUST be
/// rejected, not truncated to the maximal hex-only prefix (O-1).
///
/// `pos` MUST be a valid UTF-8 char boundary in `s`; all three call sites
/// derive it as `hash_start + hash.len()` where `hash` was built entirely
/// from single-byte ASCII hex digits, so this always holds.
///
/// `is_word_char` was promoted from `arm_b`-private (F-S2107-RECON-004) to
/// crate-shared (`crate::is_word_char`, F-S2107-RECON-005) so Arm A1 and
/// Arm A2's `\b` boundary checks share the same predicate — see
/// `crate::is_word_char` doc for the full sibling-file sweep rationale.
fn trailing_word_boundary_ok(s: &str, pos: usize) -> bool {
    pos >= s.len() || {
        let next = s[pos..].chars().next().unwrap_or('\0');
        !crate::is_word_char(next)
    }
}

/// Extract the story ID from a STORY-INDEX.md table row.
///
/// Table row format: `| S-21.07 | title | ... |`
/// The story ID is in the first pipe-delimited cell.
///
/// Returns `None` for non-canonical IDs like `S-README` that do not match
/// `S-[0-9]+\.[0-9]+` (PC9/PC16). Uses `parse_story_id_len` — the same
/// canonical predicate as `extract_blockquote_pairs` — so both extraction
/// sites enforce the same story-ID pattern.
///
/// # BC trace
/// BC-5.39.010 PC9/PC16: story ID canonical pattern `S-[0-9]+\.[0-9]+`.
/// F-S2107-P3-015: TD-VSDD-060 sibling-site sweep.
fn extract_story_id_from_table_row(line: &str) -> Option<String> {
    let mut cells = line.split('|');
    cells.next(); // skip leading empty (before first |)
    let first_cell = cells.next()?;
    let trimmed = first_cell.trim();
    // parse_story_id_len: length of S-[0-9]+\.[0-9]+ prefix, or 0 if no match.
    // Require the entire trimmed cell to be a canonical story ID (no trailing garbage).
    let len = parse_story_id_len(trimmed);
    if len > 0 && len == trimmed.len() {
        Some(trimmed.to_string())
    } else {
        None
    }
}

/// Extract all `S-XX.YY=HHHHHHH` pairs from a blockquote line.
///
/// F-S2107-P1B-004: production STORY-INDEX uses ONE prose `> ` line containing all
/// story hashes as semicolon-separated `S-XX.YY=HHHHHHH` tokens. This helper extracts
/// ALL such pairs (not just the first) using a word-boundary-aware scan. Returns only
/// pairs with valid hex hashes bounded to {7,40} chars.
fn extract_blockquote_pairs(line: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    // Skip the "> " prefix
    let rest = if let Some(r) = line.strip_prefix("> ") {
        r
    } else {
        return pairs;
    };

    let mut search_pos = 0;
    while search_pos < rest.len() {
        // Find next "S-" at a word boundary
        let Some(rel) = rest[search_pos..].find("S-") else {
            break;
        };
        let abs = search_pos + rel;

        // Word boundary: char before "S" must not be a word char
        // (F-S2107-RECON-004/005: shares `crate::is_word_char` with the
        // trailing boundary check — same `\b` definition).
        let wb_ok = abs == 0 || {
            let prev = rest[..abs].chars().last().unwrap_or('\0');
            !crate::is_word_char(prev)
        };
        if !wb_ok {
            search_pos = abs + 1;
            continue;
        }

        // Parse S-[0-9]+\.[0-9]+ id
        let candidate = &rest[abs..];
        let id_len = parse_story_id_len(candidate);
        if id_len == 0 {
            search_pos = abs + 1;
            continue;
        }

        // Must be followed immediately by '='
        if id_len >= candidate.len() || candidate.as_bytes()[id_len] != b'=' {
            search_pos = abs + id_len;
            continue;
        }

        let story_id = &candidate[..id_len];
        let hash_start = id_len + 1;
        // PC20/PC21 specify [0-9a-f]{7,40} — lowercase only. This is deliberate and
        // spec-mandated (BC-5.39.010 PC20/PC21); do not widen to is_ascii_hexdigit().
        let hash: String = candidate[hash_start..]
            .chars()
            .take_while(|c| matches!(c, '0'..='9' | 'a'..='f'))
            .collect();

        // O-1 / PC21 trailing `\b`: the character immediately after the hex
        // run must be absent (EOL) or a non-word character. All hex chars
        // are ASCII, so `hash_start + hash.len()` is a valid byte boundary.
        if hash.len() >= 7
            && hash.len() <= 40
            && trailing_word_boundary_ok(candidate, hash_start + hash.len())
        {
            pairs.push((story_id.to_string(), hash.clone()));
        }
        // F-S2107-RECON-001: `hash.len().max(1)` is NOT safe as a fallback step
        // when `hash` is empty (zero hex chars matched) — the char immediately
        // after `=` may be a multibyte UTF-8 char, in which case advancing by a
        // literal `1` byte lands mid-char and the next iteration's
        // `&rest[search_pos..].find("S-")` slice panics (BC-5.39.010
        // invariant 9). When `hash` is non-empty every matched char is ASCII
        // hex (1 byte each), so `hash.len()` is already a safe byte-length
        // step; only the empty case needs the char-boundary-aware fallback —
        // step by the actual byte length of the char right after `=`.
        let step = if hash.is_empty() {
            candidate[hash_start..]
                .chars()
                .next()
                .map_or(1, char::len_utf8)
        } else {
            hash.len()
        };
        search_pos = abs + hash_start + step;
    }
    pairs
}

/// Parse the byte-length of a `S-[0-9]+\.[0-9]+` story ID prefix at the start of `s`.
/// Returns the length of the match, or 0 if no valid story ID starts at position 0.
fn parse_story_id_len(s: &str) -> usize {
    let bytes = s.as_bytes();
    if bytes.len() < 4 || bytes[0] != b'S' || bytes[1] != b'-' {
        return 0;
    }
    let mut i = 2;
    // First digit group
    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
        return 0;
    }
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    // Dot separator
    if i >= bytes.len() || bytes[i] != b'.' {
        return 0;
    }
    i += 1;
    // Second digit group
    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
        return 0;
    }
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    i
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // parse_story_input_hash — BC-5.39.010 precondition 18
    // -----------------------------------------------------------------------

    #[test]
    fn test_BC_5_39_010_arm_b_story_hash_extracted_from_frontmatter() {
        let content = "---\ninput-hash: \"47a65c9\"\n---\nbody\n";
        let result = parse_story_input_hash(content);
        assert_eq!(result, Some("47a65c9".to_string()));
    }

    #[test]
    fn test_BC_5_39_010_arm_b_no_input_hash_returns_none() {
        // BC-5.39.010 EC-006 / AC-010: no input-hash field → Arm B1 skips entirely
        let content = "---\nstory_id: S-21.07\n---\nbody\n";
        let result = parse_story_input_hash(content);
        assert!(result.is_none(), "absent input-hash must return None");
    }

    // -----------------------------------------------------------------------
    // run_arm_b1_with_index_result — BC-5.39.010 postconditions 12-13
    // -----------------------------------------------------------------------

    /// AC-009 MUTANT: STORY-INDEX internally inconsistent blocks (BC-5.39.010 §PC13b).
    ///
    /// BC-5.39.010 §PC13 directional carve-out:
    /// - PC13a (B2==B3, B1!=B2): advisory (burst-ordering artefact; STORY-INDEX consistent).
    /// - PC13b (B2!=B3): block (anomalous — STORY-INDEX internal inconsistency has no
    ///   burst-ordering explanation; catalog row and blockquote written in the same commit).
    ///
    /// This test covers PC13b: B2!=B3 regardless of B1 → block.
    /// For PC13a (B2==B3, B1!=B2 → advisory) see T-P6C bats integration test.
    #[test]
    fn test_BC_5_39_010_arm_b1_hash_mismatch_blocks() {
        // PC13b: B2="4be9d21" (catalog) != B3="aabbcc0" (blockquote) → anomalous block.
        // B1 value is irrelevant for PC13b — B2!=B3 always blocks regardless of B1.
        let index_content = b"| S-21.07 | ... | input-hash 4be9d21 | ...\n> S-21.07=aabbcc0\n";
        let (violations, _) =
            run_arm_b1_with_index_result("S-21.07", "47a65c9", Ok(index_content.to_vec()));
        assert!(
            !violations.is_empty(),
            "PC13b (B2!=B3): STORY-INDEX internal inconsistency must produce a blocking violation"
        );
        let msg = &violations[0].description;
        assert!(msg.contains("[Class B]"), "violation must cite [Class B]");
        assert!(msg.contains("POLICY 18"), "violation must cite POLICY 18");
        // PC13b three-provenance note (invariant 11, BC-5.39.010)
        assert!(
            msg.contains("STALE") || msg.contains("FABRICATED"),
            "violation must include provenance categories (STALE/FABRICATED/ALGORITHM-DIVERGENT)"
        );
        assert!(
            msg.contains("catalog and blockquote disagree"),
            "PC13b violation must describe the internal STORY-INDEX inconsistency"
        );
    }

    /// AC-009 CONTROL: three-way match passes.
    #[test]
    fn test_BC_5_39_010_arm_b1_hash_match_passes() {
        let index_content = b"| S-21.07 | ... | input-hash 47a65c9 | ...\n> S-21.07=47a65c9\n";
        let (violations, _) =
            run_arm_b1_with_index_result("S-21.07", "47a65c9", Ok(index_content.to_vec()));
        assert!(violations.is_empty(), "three-way hash match must not block");
    }

    /// AC-010: absent secondary sites are advisory-only (postcondition 12).
    #[test]
    fn test_BC_5_39_010_arm_b1_absent_index_sites_advisory() {
        // STORY-INDEX.md exists but has no entry for this story yet
        let index_content = b"| S-21.06 | ... | input-hash aabbcc | ...\n> S-21.06=aabbcc\n";
        let (violations, advisories) =
            run_arm_b1_with_index_result("S-21.07", "47a65c9", Ok(index_content.to_vec()));
        assert!(
            violations.is_empty(),
            "absent secondary sites must not block"
        );
        assert!(
            !advisories.is_empty(),
            "absent secondary sites must emit advisory"
        );
    }

    /// AC-010: no input-hash field → Arm B1 skips entirely.
    #[test]
    fn test_BC_5_39_010_arm_b1_no_input_hash_skips() {
        // This is tested through run_arm_b1 (effectful) — the check is in run_arm_b1
        // before calling run_arm_b1_with_index_result. parse_story_input_hash returns
        // None → skips. Test via parse_story_input_hash which is pure.
        let content = "---\nstory_id: S-21.07\n---\nbody\n";
        let hash = parse_story_input_hash(content);
        assert!(
            hash.is_none(),
            "no input-hash field must return None (B1 skips)"
        );
    }

    /// EC-009 / BC-5.39.010 precondition 26 second clause: STORY-INDEX.md returns
    /// CapabilityDenied on story file write → BLOCK (sandbox misconfiguration is
    /// never legitimate on any secondary target per invariant 5).
    ///
    /// Note: the stub doc-comment for run_arm_b1_with_index_result labels
    /// CapabilityDenied as "Advisory" — that contradicts BC-5.39.010 PC26
    /// ("CapabilityDenied → block"). This test enforces the BC, not the stub comment.
    #[test]
    fn test_BC_5_39_010_arm_b1_story_index_capability_denied_blocks() {
        let (violations, _) =
            run_arm_b1_with_index_result("S-21.07", "47a65c9", Err(HostError::CapabilityDenied));
        assert!(
            !violations.is_empty(),
            "CapabilityDenied on STORY-INDEX.md must produce a blocking violation \
            (BC-5.39.010 PC26; invariant 5 — sandbox misconfiguration on secondary target is blocking)"
        );
    }

    /// Vacuity guard — Arm B2 with no blockquote entries must not pass vacuously.
    /// BC-5.39.010 invariant 8 (B2 cascade). An Arm B2 invocation with no
    /// blockquote entries must invoke the comparison logic (not short-circuit to
    /// "nothing to check → clean pass"). run_arm_b2 is fully implemented; this test
    /// is a green regression guard confirming it still executes the comparison
    /// logic (rather than short-circuiting) and correctly reports zero violations
    /// when there is nothing in the blockquote to compare.
    #[test]
    fn test_BC_5_39_010_arm_b2_no_blockquote_entries_not_vacuous() {
        // STORY-INDEX.md with catalog rows but NO blockquote (> lines)
        // Expected: 0 violations (nothing in blockquote to compare).
        // The key is that run_arm_b2 EXECUTES (not skips) the comparison logic.
        let content = "| S-21.07 | title | input-hash 47a65c9 | W4 | P1 |\n\
            # No blockquote section here\n";
        let violations = run_arm_b2(content);
        // run_arm_b2 is fully implemented: 0 violations expected (no blockquote
        // entries to compare against catalog).
        assert!(
            violations.is_empty(),
            "absent blockquote entries must not produce violations (nothing to compare)"
        );
    }

    // -----------------------------------------------------------------------
    // run_arm_b2 — BC-5.39.010 postcondition 15 (cascade)
    // -----------------------------------------------------------------------

    /// AC-011 MUTANT: catalog ≠ blockquote → block (cascade).
    #[test]
    fn test_BC_5_39_010_arm_b2_catalog_blockquote_mismatch_blocks() {
        let content = "| S-21.07 | ... | input-hash 47a65c9 | ...\n\
            > S-21.07=4be9d21\n";
        let violations = run_arm_b2(content);
        assert!(
            !violations.is_empty(),
            "catalog/blockquote mismatch must produce violations"
        );
        assert!(
            violations[0].description.contains("[Class B]"),
            "violation must cite [Class B]"
        );
    }

    /// AC-011 CONTROL: catalog and blockquote agree → passes.
    #[test]
    fn test_BC_5_39_010_arm_b2_catalog_blockquote_agree_passes() {
        let content = "| S-21.07 | ... | input-hash 47a65c9 | ...\n\
            > S-21.07=47a65c9\n";
        let violations = run_arm_b2(content);
        assert!(
            violations.is_empty(),
            "matching catalog/blockquote must not block"
        );
    }

    // -----------------------------------------------------------------------
    // F-S2107-P1B-003: parse_story_index_blockquote_hash uses starts_with
    // "> S-21.07=" which only matches synthetic per-story blockquote lines.
    // Production STORY-INDEX.md uses ONE prose line:
    //   > **E-21 delivery:** ... S-21.07=47a65c9. All 7 distinct.
    // This shape never starts with "> S-21.07=" → B3 is always None → advisory
    // (never blocking) even when blockquote hash mismatches → gate is inert.
    // BC-5.39.010 §B3 invariant: B3 must be extracted from the prose line.
    // -----------------------------------------------------------------------

    /// F-S2107-P1B-003: production blockquote B3 must be extractable from prose line.
    ///
    /// RED GATE: `starts_with("> S-21.07=")` never matches production prose.
    /// parse_story_index_blockquote_hash returns None → assert_eq fails → RED gate.
    /// After fix (scan embedded `S-21.07=HHHHHHH` token in prose line):
    /// returns Some("47a65c9") → PASSES.
    #[test]
    fn test_BC_5_39_010_arm_b1_production_blockquote_b3_extracted() {
        // Production STORY-INDEX.md blockquote: ONE prose line with all story hashes
        // embedded as `S-XX.YY=HHHHHHH` tokens separated by `;`.
        let content = concat!(
            "| S-21.07 | ... | input-hash 47a65c9 |\n",
            "> **E-21 delivery:** Completed E-21 wave-4 stories.",
            " Input-hashes: S-21.01=32aaccc; S-21.02=11bbddd; S-21.07=47a65c9.",
            " All 7 distinct.\n",
        );
        let b3 = parse_story_index_blockquote_hash(content, "S-21.07");
        assert_eq!(
            b3,
            Some("47a65c9".to_string()),
            "production blockquote prose line must yield B3 = '47a65c9'. \
            Red Gate: starts_with('> S-21.07=') never matches production shape → None (F-S2107-P1B-003)"
        );
    }

    // -----------------------------------------------------------------------
    // F-S2107-P1B-004: run_arm_b2 parses the blockquote by splitting each "> "
    // line on the first '='. Production prose line starts with "> **E-21 delivery:**"
    // which has an '=' in "delivery" after the '**'. No — actually the first '='
    // in the line is in the embedded `S-XX.YY=HHHHHHH` token. But `rest.find('=')`
    // from position 0 finds the FIRST '=' in the entire line-after-"> ". The story_id
    // parser takes everything before the first '=' as the story_id.
    //
    // For line "> **E-21 delivery:** ... S-21.01=32aaccc; ...",
    // after stripping "> ", rest = "**E-21 delivery:** ... S-21.01=32aaccc; ..."
    // `rest.find('=')` → position of '=' in "S-21.01=32aaccc" → rest[..pos] = garbage
    // → `garbage.starts_with("S-")` → false → orphaned entry → B2 cascade violation.
    //
    // Even if catalog rows agree, run_arm_b2 generates spurious violations for
    // every production blockquote "> " line because story_id extraction fails.
    // -----------------------------------------------------------------------

    /// F-S2107-P1B-004: production blockquote shape must not generate spurious B2 violations.
    ///
    /// RED GATE: run_arm_b2 on production-shaped STORY-INDEX with matching hashes
    /// generates "orphaned blockquote entry" violations. violations NOT empty.
    /// assert!(violations.is_empty()) FAILS → RED gate.
    /// After fix (parse embedded tokens from prose line): violations empty → PASSES.
    #[test]
    fn test_BC_5_39_010_arm_b2_production_blockquote_shape_no_spurious_violations() {
        // Production-shaped STORY-INDEX: catalog row + production prose blockquote.
        // Both agree on S-21.07=47a65c9. Expected: zero violations.
        let content = concat!(
            "| S-21.07 | validate-cross-site-correspondence | E-21 | 11 | P1",
            " | [] | [] | draft | [BC-5.39.010 v1.14] input-hash 47a65c9 |\n",
            "> **E-21 delivery:** Completed E-21 wave-4 stories.",
            " Input-hashes: S-21.01=32aaccc; S-21.07=47a65c9. All 7 distinct.\n",
        );
        let violations = run_arm_b2(content);
        assert!(
            violations.is_empty(),
            "production-shaped STORY-INDEX with catalog=blockquote=47a65c9 must not block. \
            Red Gate: run_arm_b2 generates spurious 'orphaned blockquote entry' from \
            production prose line (F-S2107-P1B-004)"
        );
    }

    // -----------------------------------------------------------------------
    // F-S2107-P1B-009: extract_input_hash_token accepts any 7+ char alphanumeric
    // token after "input-hash ". Non-hex tokens like "bonus" are accepted.
    // BC-5.39.010 precondition 17: input-hash value must be hex (0-9a-f only).
    // -----------------------------------------------------------------------

    /// F-S2107-P1B-009: non-hex catalog token must not be accepted as input-hash.
    ///
    /// RED GATE: current code returns Some("bonus") — no hex validation.
    /// assert!(catalog_hash.is_none()) FAILS → RED gate.
    /// After fix (validate hex charset): returns None → PASSES.
    #[test]
    fn test_BC_5_39_010_arm_b_non_hex_catalog_token_not_accepted() {
        // "bonus" contains letters outside 0-9a-f ('o', 'n', 'u', 's') → non-hex
        let content = "| S-21.07 | ... | input-hash bonus | ...\n";
        let catalog_hash = parse_story_index_catalog_hash(content, "S-21.07");
        assert!(
            catalog_hash.is_none(),
            "non-hex token 'bonus' must not be accepted as input-hash value. \
            BC-5.39.010 PC17: input-hash must be hex (F-S2107-P1B-009). \
            Red Gate: current code returns Some(\"bonus\") → assertion fails"
        );
    }

    // -----------------------------------------------------------------------
    // F-S2107-P1B-008: parse_story_index_catalog_hash uses naive contains(story_id).
    // When STORY-INDEX.md has a row for S-18.00 that includes "S-18.01" in its
    // blocks or depends_on column, `contains("S-18.01")` matches the S-18.00 row
    // FIRST (before the actual S-18.01 row), returning the wrong hash.
    // BC-5.39.010 §PC16: catalog lookup must match the CANONICAL S-NNN.NNN row
    // (i.e., the row whose FIRST cell is the story_id, not any row mentioning it).
    // -----------------------------------------------------------------------

    /// F-S2107-P1B-008: cross-story catalog lookup must not match wrong row.
    ///
    /// When S-18.00 row contains "S-18.01" in a later column, naive contains("S-18.01")
    /// matches the S-18.00 row first, returning S-18.00's hash instead of S-18.01's.
    ///
    /// RED GATE: parse_story_index_catalog_hash("S-18.01") returns "e5bc551" (S-18.00 hash).
    /// assert_eq!(catalog_hash, Some("1b4ea21")) FAILS → RED gate.
    /// After fix (match only rows where first cell == story_id): returns "1b4ea21" → PASSES.
    #[test]
    fn test_BC_5_39_010_arm_b1_cross_story_catalog_correct_row_matched() {
        // S-18.00 row mentions S-18.01 in a blocks/depends column — comes BEFORE S-18.01 row.
        // Naive contains("S-18.01") hits S-18.00 first → returns S-18.00 hash "e5bc551".
        let index = concat!(
            "| S-18.00 | parent epic | E-18 | ... | [S-18.01] | input-hash e5bc551 |\n",
            "| S-18.01 | child story | E-18 | ... | []        | input-hash 1b4ea21 |\n",
            "> **E-18 delivery:** S-18.00=e5bc551; S-18.01=1b4ea21.\n",
        );
        let catalog_hash = parse_story_index_catalog_hash(index, "S-18.01");
        assert_eq!(
            catalog_hash,
            Some("1b4ea21".to_string()),
            "S-18.01 catalog lookup must return '1b4ea21' (own row), not 'e5bc551' (S-18.00 row). \
            Red Gate: naive contains('S-18.01') matches S-18.00 row first → wrong hash (F-S2107-P1B-008)"
        );
    }

    // -----------------------------------------------------------------------
    // PC40 (BC-5.39.010 v1.19 amendment): volatile-input precondition.
    //
    // When a story has `input-hash:` AND `inputs:` containing volatile paths
    // (STATE.md, INDEX files, cycles/ artifacts), Arm B1 must emit advisory +
    // Continue rather than proceeding with the three-way comparison.
    //
    // Volatile paths (path-equals, per shipped `is_volatile_path`): .factory/STATE.md;
    // .factory/cycles/**/{STATE,decision-log,lessons,burst-log}.md (filename match under
    // cycles/); .factory/specs/architecture/ARCH-INDEX.md;
    // .factory/specs/behavioral-contracts/BC-INDEX.md; .factory/stories/STORY-INDEX.md.
    // VP-INDEX.md is NOT volatile (absent from ADR-037 §Decision 2; see
    // test_BC_5_39_010_pc40_vp_index_not_volatile below).
    //
    // IMPLEMENTATION REQUIREMENT:
    //   1. Add `pub fn is_volatile_path(path: &str) -> bool` to arm_b.rs.
    //   2. Add `pub fn parse_story_volatile_inputs(content: &str) -> Vec<String>`
    //      that extracts the `inputs:` frontmatter YAML list.
    //   3. Modify `run_arm_b1` to check for volatile inputs BEFORE calling
    //      `run_arm_b1_with_index_result`; if any volatile path found, emit
    //      advisory + return Continue (skip the three-way comparison).
    //
    // RED GATE strategy: test calls panic!() stub — always fails until PC40 is
    // implemented and the test is updated to call the real functions.
    // -----------------------------------------------------------------------

    /// PC40: is_volatile_path + parse_story_volatile_inputs smoke test.
    ///
    /// Verifies the two PC40 pure functions and indirectly validates that
    /// run_arm_b1 skips the three-way comparison when volatile inputs are present.
    ///
    /// BC-5.39.010 §PC40: volatile inputs → advisory + Continue (skip comparison).
    #[test]
    fn test_BC_5_39_010_arm_b1_pc40_volatile_input_detection_required() {
        assert!(
            is_volatile_path(".factory/STATE.md"),
            ".factory/STATE.md must be volatile (PC40)"
        );
        assert!(
            !is_volatile_path(".factory/stories/S-21.07-test.md"),
            "story file must NOT be volatile"
        );
        let story = "---\ninputs: [\".factory/STATE.md\"]\n---\n";
        let inputs = parse_story_volatile_inputs(story);
        assert_eq!(inputs, vec![".factory/STATE.md".to_string()]);
    }

    // -----------------------------------------------------------------------
    // F-S2107-P3-002 (BLOCKER): is_volatile_path three-way drift
    //
    // BC-5.39.010 v1.19 (ADR-037 §Decision 2) specifies six canonical volatile-input
    // patterns, expanded to eight concrete path forms in the implementation (pattern 3,
    // `{decision-log,lessons,burst-log}`, yields three concrete forms):
    //   1. `.factory/STATE.md` (direct child, parent==".factory")
    //   2. `.factory/cycles/**/STATE.md`
    //   3. `.factory/cycles/**/decision-log.md`
    //   4. `.factory/cycles/**/lessons.md`
    //   5. `.factory/cycles/**/burst-log.md`
    //   6. `.factory/specs/architecture/ARCH-INDEX.md`
    //   7. `.factory/specs/behavioral-contracts/BC-INDEX.md`  (path-equals)
    //   8. `.factory/stories/STORY-INDEX.md`                  (path-equals)
    //
    // Pre-fix impl (now closed) had FOUR drifts from this spec:
    //   (a) ARCH-INDEX.md ABSENT — caused live self-block on S-21.07 writes
    //   (b) `.factory/cycles/**` ANY-FILE — too broad; adv-cycle-pass-N.md was volatile
    //   (c) VP-INDEX.md PRESENT — not in PC40; used to always return true for VP-INDEX.md
    //       at any depth under .factory/
    //   (d) Index files matched by FILENAME ONLY (no path-depth check) — e.g.,
    //       .factory/cycles/v1.0/BC-INDEX.md used to return true
    //
    // Per-row documentary tests (patterns 1-5) confirm currently-correct behavior.
    // The RED GATE tests below covered these drifts; all four are now closed and green.
    // -----------------------------------------------------------------------

    // -- Per-row documentary tests (GREEN) -----------------------------------

    /// PC40 pattern 1: `.factory/STATE.md` direct child is volatile.
    #[test]
    fn test_BC_5_39_010_pc40_per_row_1_factory_state_md_volatile() {
        assert!(
            is_volatile_path(".factory/STATE.md"),
            "PC40 pattern 1: '.factory/STATE.md' must be volatile"
        );
    }

    /// PC40 pattern 2: `STATE.md` under cycles/ is volatile.
    #[test]
    fn test_BC_5_39_010_pc40_per_row_2_cycles_state_md_volatile() {
        assert!(
            is_volatile_path(".factory/cycles/v1.0-feature-engine-discipline-pass-1/STATE.md"),
            "PC40 pattern 2: STATE.md under cycles/ must be volatile"
        );
    }

    /// PC40 patterns 3-5: decision-log.md, lessons.md, burst-log.md under cycles/ volatile.
    #[test]
    fn test_BC_5_39_010_pc40_per_row_3_cycles_named_files_volatile() {
        assert!(
            is_volatile_path(".factory/cycles/v1.0/decision-log.md"),
            "PC40 pattern 3: decision-log.md under cycles/ must be volatile"
        );
        assert!(
            is_volatile_path(".factory/cycles/v1.0/lessons.md"),
            "PC40 pattern 4: lessons.md under cycles/ must be volatile"
        );
        assert!(
            is_volatile_path(".factory/cycles/v1.0/burst-log.md"),
            "PC40 pattern 5: burst-log.md under cycles/ must be volatile"
        );
    }

    /// PC40 pattern 7 (path-equals): `.factory/specs/behavioral-contracts/BC-INDEX.md`.
    #[test]
    fn test_BC_5_39_010_pc40_per_row_7_bc_index_volatile() {
        assert!(
            is_volatile_path(".factory/specs/behavioral-contracts/BC-INDEX.md"),
            "PC40 pattern 7: '.factory/specs/behavioral-contracts/BC-INDEX.md' must be volatile"
        );
    }

    /// PC40 pattern 8 (path-equals): `.factory/stories/STORY-INDEX.md`.
    #[test]
    fn test_BC_5_39_010_pc40_per_row_8_story_index_volatile() {
        assert!(
            is_volatile_path(".factory/stories/STORY-INDEX.md"),
            "PC40 pattern 8: '.factory/stories/STORY-INDEX.md' must be volatile"
        );
    }

    // -- RED GATE tests for F-S2107-P3-002 drifts ---------------------------

    /// F-S2107-P3-002(a) RED GATE: ARCH-INDEX.md must be volatile (PC40 pattern 6).
    ///
    /// ARCH-INDEX.md is absent from the current implementation. Every write to S-21.07
    /// itself (which lists ARCH-INDEX.md in its inputs:) self-blocks. 66 stories under
    /// .factory/stories/ reference ARCH-INDEX.md.
    ///
    /// RED GATE: current impl has no ARCH-INDEX.md match → returns false → FAILS.
    #[test]
    fn test_BC_5_39_010_pc40_arch_index_md_is_volatile() {
        assert!(
            is_volatile_path(".factory/specs/architecture/ARCH-INDEX.md"),
            "PC40 pattern 6: '.factory/specs/architecture/ARCH-INDEX.md' must be volatile. \
            F-S2107-P3-002(a): ARCH-INDEX.md absent from impl — S-21.07 self-blocks on every \
            write. 66 stories list ARCH-INDEX.md in inputs:. RED GATE."
        );
    }

    /// F-S2107-P3-002(b) RED GATE: arbitrary file under cycles/ must NOT be volatile.
    ///
    /// Blanket `.factory/cycles/**` (any file under cycles) permanently suppresses
    /// Class B for all ~20 stories that list cycles/ artifacts in their inputs:, even
    /// files like `adv-cycle-pass-1.md` that have no volatility rationale.
    ///
    /// PC40 guarantees no permanent weakening: only STATE.md + {decision-log,lessons,burst-log}.md
    /// under cycles/ are volatile. Other cycles/ files must return false.
    ///
    /// RED GATE: current blanket cycles component check → true → FAILS.
    #[test]
    fn test_BC_5_39_010_pc40_adv_cycle_pass_not_volatile() {
        assert!(
            !is_volatile_path(
                ".factory/cycles/v1.0-feature-engine-discipline-pass-1/adv-cycle-pass-1.md"
            ),
            "adv-cycle-pass-1.md is an immutable historical artifact — must NOT be volatile. \
            PC40 guarantees no permanent weakening of Class B gate. \
            F-S2107-P3-002(b): blanket cycles component check permanently suppresses BLOCKING \
            gate for ~20 stories. After fix: only 5 named patterns are volatile under cycles/. \
            RED GATE: current impl returns true for any path with a 'cycles' component."
        );
    }

    /// F-S2107-P3-002(c) RED GATE: VP-INDEX.md is NOT in PC40 and must NOT be volatile.
    ///
    /// VP-INDEX.md appears in the current impl's filename matches but is absent from
    /// ADR-037 §Decision 2 and BC-5.39.010 §PC40. It must NOT be volatile.
    ///
    /// RED GATE: current `matches!(filename, "BC-INDEX.md" | "VP-INDEX.md" | ...)` → true.
    #[test]
    fn test_BC_5_39_010_pc40_vp_index_not_volatile() {
        assert!(
            !is_volatile_path(".factory/specs/verification-properties/VP-INDEX.md"),
            "VP-INDEX.md is NOT in PC40 (ADR-037 §Decision 2) and must NOT be volatile. \
            F-S2107-P3-002(c): current impl has 'VP-INDEX.md' in filename matches → true. \
            After fix: VP-INDEX.md removed from volatile list. RED GATE."
        );
    }

    /// F-S2107-P3-002(c) RED GATE: BC-INDEX.md at wrong path must NOT be volatile.
    ///
    /// PC40 specifies path-equals semantics for index files. Only the canonical path
    /// `.factory/specs/behavioral-contracts/BC-INDEX.md` is volatile. BC-INDEX.md
    /// at a cycles/ path (or any other non-canonical path) must NOT match.
    ///
    /// RED GATE: current filename-only check `matches!(filename, "BC-INDEX.md" | ...)` plus
    /// the blanket cycles component check → `.factory/cycles/v1.0/BC-INDEX.md` returns true.
    #[test]
    fn test_BC_5_39_010_pc40_bc_index_wrong_path_not_volatile() {
        assert!(
            !is_volatile_path(".factory/cycles/v1.0/BC-INDEX.md"),
            "BC-INDEX.md at a cycles/ path must NOT be volatile. \
            PC40 specifies path-equals for '.factory/specs/behavioral-contracts/BC-INDEX.md'. \
            F-S2107-P3-002(c): current filename-only check admits BC-INDEX.md at any depth. \
            RED GATE: cycles component check also fires → returns true."
        );
    }

    // -----------------------------------------------------------------------
    // F-S2107-P3-012 (MEDIUM): volatile advisory message must use ADR-037 prescribed text
    //
    // Current advisory text:
    //   "has volatile inputs {paths:?} — skipping three-way input-hash comparison
    //   per BC-5.39.010 v1.19 PC40 ... Update input-hash manually when non-volatile
    //   inputs change."
    //
    // Prescribed text (ADR-037 §Decision 4 / BC-5.39.010 §PC40 note):
    //   "Story <id> has volatile inputs per ADR-037 §Decision 2 — three-way equality
    //   is unsatisfiable until story-writer removes volatile inputs and state-manager
    //   recomputes the hash; Class B BLOCK suspended. Volatile path(s): <list>"
    //
    // Differences: missing "ADR-037 §Decision 2" cite; missing "Class B BLOCK suspended";
    // wrong remediation instruction ("Update input-hash manually" vs. the spec's).
    // -----------------------------------------------------------------------

    /// F-S2107-P3-012 RED GATE: volatile advisory must cite ADR-037 §Decision 2
    /// and say "Class B BLOCK suspended".
    ///
    /// `run_arm_b1` returns early (before host call) when volatile inputs are detected.
    /// Safe to call in unit tests without a host — volatile check returns at line 379.
    ///
    /// RED GATE: current message lacks "ADR-037 §Decision 2" → assertion FAILS.
    #[test]
    fn test_BC_5_39_010_arm_b1_volatile_advisory_prescribed_text() {
        // Story with input-hash + volatile STATE.md in inputs → early return (no host call)
        let story_content = concat!(
            "---\n",
            "input-hash: \"abc1234\"\n",
            "inputs:\n",
            "  - .factory/STATE.md\n",
            "---\n",
            "body\n",
        );
        let (violations, advisories) = run_arm_b1("S-21.07", story_content);
        assert!(violations.is_empty(), "volatile inputs must not block");
        assert!(
            !advisories.is_empty(),
            "volatile inputs must produce an advisory"
        );
        let msg = &advisories[0].message;
        assert!(
            msg.contains("ADR-037 §Decision 2"),
            "advisory must cite 'ADR-037 §Decision 2' per prescribed text. \
            F-S2107-P3-012: current message uses 'BC-5.39.010 v1.19 PC40' instead. \
            RED GATE. Advisory: {msg}"
        );
        assert!(
            msg.contains("Class B BLOCK suspended"),
            "advisory must say 'Class B BLOCK suspended' per prescribed text. \
            F-S2107-P3-012: current message omits this phrase entirely. \
            RED GATE. Advisory: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // F-S2107-P3-015 (MEDIUM): extract_story_id_from_table_row too broad
    //
    // Pre-fix bug (now closed): `id.starts_with("S-")` used to admit non-canonical
    // IDs like "S-README". TD-VSDD-060 sibling sweep: dispatch.rs F-P2-011 fixed
    // is_canonical_story_basename; that fix has since been swept to arm_b.rs
    // extract_story_id_from_table_row as well (see `parse_story_id_len` below).
    //
    // PC9/PC16: story ID must match S-[0-9]+\.[0-9]+ (e.g., "S-21.07", "S-1.1").
    // -----------------------------------------------------------------------

    /// F-S2107-P3-015 RED GATE: non-canonical story IDs like "S-README" must return None.
    ///
    /// Pre-fix Red Gate (now closed): `extract_story_id_from_table_row("| S-README | ... |")`
    /// used to return Some("S-README") via `starts_with("S-")`. After fix (canonical
    /// predicate): None.
    ///
    /// RED GATE: current impl returns Some("S-README") → assert!(result.is_none()) FAILS.
    #[test]
    fn test_BC_5_39_010_arm_b2_non_canonical_story_id_rejected() {
        let result = extract_story_id_from_table_row(
            "| S-README | Some README title | input-hash abc1234 |",
        );
        assert!(
            result.is_none(),
            "extract_story_id_from_table_row must reject non-canonical IDs like 'S-README'. \
            PC9/PC16: story ID must match S-[0-9]+\\.[0-9]+. \
            TD-VSDD-060 sibling-site sweep: dispatch.rs F-P2-011 fix not swept to arm_b.rs. \
            F-S2107-P3-015: current `starts_with('S-')` admits 'S-README'. RED GATE. \
            Got: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // ADR-038 §Decision 3 RED GATE — half-present case in run_arm_b1_with_index_result
    //
    // Current behavior (pre-fix): when exactly one of {B2, B3} is present and disagrees
    // with B1, run_arm_b1_with_index_result produces a VIOLATION (block).
    //
    //   (Some(b2), None) where b2 != story_hash → violations.push(...)  ← BLOCKS
    //   (None, Some(b3)) where b3 != story_hash → violations.push(...)  ← BLOCKS
    //
    // Required behavior (ADR-038 §Decision 3): advisory + Continue, NOT block.
    //   PC12 literal: "B2 or B3 absent → advisory + Continue" — unconditional
    //   inclusive-or. Half-present satisfies this predicate because one site is absent.
    //
    // Live corpus counterexamples: S-18.11 (catalog=c45c0fc, no blockquote) and
    // S-18.12 (catalog=345086c, no blockquote) currently block Arm B1 gate on
    // STORY-INDEX.md writes — the correct behavior is advisory + Continue.
    //
    // RED GATE status:
    //   - Both tests below are RED until the implementer changes the (Some(b2), None)
    //     and (None, Some(b3)) arms from `violations.push` to `advisories.push`.
    //   - The match-case controls (B1==B2, no blockquote) are GREEN immediately.
    // -----------------------------------------------------------------------

    /// ADR-038 §Decision 3 RED GATE: half-present (catalog present, mismatch) →
    /// advisory + Continue, NOT block.
    ///
    /// (Some(b2), None) where b2 != story_hash: current code pushes to violations (blocks).
    /// Required: push to advisories (advisory + Continue per PC12).
    ///
    /// S-18.11 shape: STORY-INDEX has catalog row with hash c45c0fc; no blockquote entry.
    /// Arm B1 triggered on a story-file write where B1 = "aabbccd" (differs from B2).
    ///
    /// TEETH: current code returns violations=[...] for this input → assert!(violations.is_empty())
    /// FAILS. Proves the test is genuinely RED, not vacuous.
    /// Control companion below proves the match case (B1==B2) is advisory (not blocking).
    ///
    /// RED GATE: current `if b2 != story_hash { violations.push(...) }` in (Some(b2), None)
    /// arm → violations not empty → FAILS.
    /// GREEN after implementer changes arm to `advisories.push(...)`.
    #[test]
    fn test_BC_5_39_010_arm_b1_half_present_catalog_mismatch_is_advisory_not_block() {
        let index_content = b"| S-18.11 | some story | input-hash c45c0fc |\n\
            # No blockquote entry for S-18.11\n";
        let (violations, advisories) = run_arm_b1_with_index_result(
            "S-18.11",
            "aabbccd", // B1 differs from B2 (c45c0fc)
            Ok(index_content.to_vec()),
        );
        assert!(
            violations.is_empty(),
            "ADR-038 §Decision 3: half-present (catalog=c45c0fc, blockquote=absent, B1=aabbccd) \
            must be advisory + Continue per PC12 ('B2 or B3 absent → advisory + Continue'). \
            RED GATE: current (Some(b2), None) arm pushes to violations when b2 != story_hash. \
            GREEN after implementer changes the arm to push to advisories instead. \
            Violations: {violations:?}"
        );
        assert!(
            !advisories.is_empty(),
            "ADR-038 §Decision 3: half-present case must emit at least one advisory. \
            Advisories: {advisories:?}"
        );
    }

    /// CONTROL for half-present catalog mismatch RED GATE: when B1 == B2 and blockquote
    /// absent, the result must also be advisory (not a violation).
    ///
    /// This is the currently-GREEN adjacent case: (Some(b2), None) where b2 == story_hash
    /// already emits an advisory in the current code. Proves the RED GATE above is
    /// distinguishable from this passing case — if both passed, the RED GATE would be suspect.
    ///
    /// GREEN immediately (current code handles this correctly).
    #[test]
    fn test_BC_5_39_010_arm_b1_half_present_catalog_match_is_advisory() {
        let index_content = b"| S-18.11 | some story | input-hash c45c0fc |\n\
            # No blockquote entry for S-18.11\n";
        let (violations, advisories) = run_arm_b1_with_index_result(
            "S-18.11",
            "c45c0fc", // B1 == B2: same hash
            Ok(index_content.to_vec()),
        );
        assert!(
            violations.is_empty(),
            "Half-present (B1==B2, blockquote absent) must not block. \
            Control for ADR-038 §Decision 3 RED GATE. Violations: {violations:?}"
        );
        assert!(
            !advisories.is_empty(),
            "Half-present (B1==B2, blockquote absent) must emit advisory. \
            Advisories: {advisories:?}"
        );
    }

    /// ADR-038 §Decision 3 RED GATE: half-present (blockquote present, mismatch) →
    /// advisory + Continue, NOT block.
    ///
    /// (None, Some(b3)) where b3 != story_hash: current code pushes to violations (blocks).
    /// Required: push to advisories (advisory + Continue per PC12).
    ///
    /// S-18.12 shape: STORY-INDEX has no catalog row; blockquote entry has hash 345086c.
    /// Arm B1 triggered on a story-file write where B1 = "aabbccd" (differs from B3).
    ///
    /// TEETH: current code returns violations=[...] for this input → FAILS.
    /// GREEN after implementer changes (None, Some(b3)) arm to advisory + Continue.
    #[test]
    fn test_BC_5_39_010_arm_b1_half_present_blockquote_mismatch_is_advisory_not_block() {
        // Only blockquote present; no catalog row for S-18.12.
        let index_content = b"| S-18.11 | other story | input-hash c45c0fc |\n\
            > S-18.12=345086c\n";
        let (violations, advisories) = run_arm_b1_with_index_result(
            "S-18.12",
            "aabbccd", // B1 differs from B3 (345086c)
            Ok(index_content.to_vec()),
        );
        assert!(
            violations.is_empty(),
            "ADR-038 §Decision 3: half-present (catalog=absent, blockquote=345086c, B1=aabbccd) \
            must be advisory + Continue per PC12. \
            RED GATE: current (None, Some(b3)) arm pushes to violations when b3 != story_hash. \
            GREEN after implementer changes the arm to push to advisories instead. \
            Violations: {violations:?}"
        );
        assert!(
            !advisories.is_empty(),
            "ADR-038 §Decision 3: half-present case must emit at least one advisory. \
            Advisories: {advisories:?}"
        );
    }

    // -----------------------------------------------------------------------
    // BC-5.39.010 v1.22 / ADV-RECON11-001: precondition 15b / postcondition 26
    // — Secondary Index-File UTF-8 Decode Failure (Arm B1 STORY-INDEX.md
    // secondary read ONLY — NOT Arm B2, where STORY-INDEX.md is the primary
    // target and is already governed by precondition 15a / postcondition 25's
    // fail-closed BLOCK). ADVISORY (Continue), not block.
    //
    // RED GATE: `parse_story_index_catalog_hash` and
    // `parse_story_index_blockquote_hash` both do
    // `std::str::from_utf8(index_content).ok()?` — non-UTF-8 bytes silently
    // produce `None` for both B2 and B3, indistinguishable from a genuinely
    // new, not-yet-registered story in a DECODABLE index file.
    // `run_arm_b1_with_index_result`'s `(None, None)` arm then emits the
    // GENERIC postcondition-12 "not yet registered" fail-open advisory —
    // silently disabling three-way hash checking with no disclosure that the
    // actual root cause is an undecodable STORY-INDEX.md, not legitimate
    // bootstrap absence. Postcondition 26 instead requires a DISTINCT advisory
    // naming STORY-INDEX.md and stating the hash state is INDETERMINATE, not
    // confirmed-absent.
    // -----------------------------------------------------------------------

    /// BC-5.39.010 precondition 15b / postcondition 26 (v1.22 / ADV-RECON11-001):
    /// non-UTF-8 STORY-INDEX.md secondary read at Arm B1 must emit the distinct
    /// INDETERMINATE advisory + Continue, NOT the generic PC12 "not yet
    /// registered" fail-open advisory that conflates decode failure with
    /// legitimate bootstrap absence.
    #[test]
    fn test_BC_5_39_010_arm_b1_non_utf8_story_index_indeterminate_advisory() {
        // STORY-INDEX.md "read" succeeds as bytes (Ok(...)) but the bytes are not
        // valid UTF-8.
        let non_utf8_bytes: Vec<u8> = vec![0xFF, 0xFE, 0xFD, 0x80, 0x81, b'|', b'S', b'-'];
        let (violations, advisories) =
            run_arm_b1_with_index_result("S-21.07", "47a65c9", Ok(non_utf8_bytes));

        assert!(
            violations.is_empty(),
            "postcondition 26 (v1.22): non-UTF-8 STORY-INDEX.md secondary read at Arm \
            B1 MUST NOT block. Actual violations: {violations:?}"
        );
        assert!(
            !advisories.is_empty(),
            "postcondition 26 (v1.22): non-UTF-8 STORY-INDEX.md secondary read MUST \
            emit an advisory. Actual advisories: {advisories:?}"
        );

        // BC-5.39.010 §PC4a (NORMATIVE): test-writer MUST assert the COMPLETE
        // formatted string by equality check; `.contains()`-only on substrings is
        // NON-CONFORMING. Build the COMPLETE prescribed postcondition 26 message
        // verbatim from the BC body (§Postconditions, postcondition 26) with
        // `<index-file>` = "STORY-INDEX.md" and `<id>` = "S-21.07" substituted,
        // then assert full-string equality — this also asserts the second
        // sentence ("Fix: verify the index file's encoding and re-save as
        // UTF-8."), which a `.contains()`-only check on the first sentence would
        // leave a mutation free to delete or corrupt undetected.
        let expected_decoded = "validate-cross-site-correspondence: STORY-INDEX.md failed \
            UTF-8 decode — row/hash state for 'S-21.07' is INDETERMINATE, not confirmed-absent. \
            Fix: verify the index file's encoding and re-save as UTF-8."
            .to_string();
        assert_eq!(
            advisories.len(),
            1,
            "postcondition 26 (v1.22): exactly one distinct INDETERMINATE advisory \
            expected for the non-UTF-8 STORY-INDEX.md secondary read. Actual advisories: \
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
            !advisories[0].message.contains("not yet registered"),
            "postcondition 26 (v1.22): the non-UTF-8-decode advisory MUST be distinct \
            from the generic PC12 'not yet registered' fail-open advisory — decode \
            failure (indeterminate hash state) and legitimate bootstrap absence in a \
            decodable index file MUST NOT be conflated (precondition 15b / invariant 5 \
            extension). RED GATE: current code's (None, None) arm from `.ok()?` on \
            both parse_story_index_catalog_hash and parse_story_index_blockquote_hash \
            emits exactly this generic message. Actual advisories: {advisories:?}"
        );
    }

    // -----------------------------------------------------------------------
    // F-S2107-RECON-001 (MEDIUM): byte-index slicing on retry offsets computed
    // by `+1` on a byte index can land INSIDE a multibyte UTF-8 char, causing a
    // slice panic ("byte index N is not a char boundary") on the NEXT loop
    // iteration's `&line[search_start..]` / `&rest[search_pos..]` re-slice.
    //
    // Under the registry `on_error="continue"`, that WASM trap is silently
    // swallowed — the gate is silently disabled for that write, which is the
    // exact silent-guard-failure class BC-5.39.010 invariant 9 exists to
    // prevent ("byte-index slicing on extracted strings MUST use
    // `is_char_boundary()` checks where multi-byte UTF-8 is possible").
    //
    // Two sibling sites (TD-VSDD-060):
    //   1. extract_input_hash_token: `search_start = hash_start + 1;` retry.
    //   2. extract_blockquote_pairs: `search_pos = abs + hash_start +
    //      hash.len().max(1);` retry (the `.max(1)` fallback is the same
    //      naive `+1`-on-a-byte-index defect when `hash` is empty).
    //
    // Trigger: `input-hash`/`S-XX.YY=` followed by whitespace/`=` then a
    // multibyte UTF-8 char that is not a hex digit — the hex `take_while`
    // matches zero chars, so control reaches the `+1` retry landing mid-char.
    // -----------------------------------------------------------------------

    /// F-S2107-RECON-001 site 1: `extract_input_hash_token` must not panic when
    /// the byte immediately after `input-hash \s+` is a non-hex multibyte
    /// UTF-8 character (e.g. an em-dash). Pre-fix: `search_start = hash_start +
    /// 1` lands mid-char inside the 3-byte em-dash, and the next loop
    /// iteration's `&line[search_start..]` panics with "byte index N is not a
    /// char boundary". Post-fix: the retry step advances by the em-dash's
    /// actual UTF-8 byte length, no valid hex token is found, and the function
    /// returns `None` instead of panicking.
    #[test]
    fn test_BC_5_39_010_arm_b_extract_input_hash_token_multibyte_non_hex_no_panic() {
        // "input-hash " followed immediately by a 3-byte em-dash (non-hex) then
        // more non-hex ASCII — no valid [0-9a-f]{7,40} token exists on this line.
        let line = "| S-21.07 | ... | input-hash \u{2014}notahexvalue | ...";
        let result = extract_input_hash_token(line);
        assert_eq!(
            result, None,
            "no valid hex input-hash token exists on this line \
            (BC-5.39.010 F-S2107-RECON-001); got {result:?}"
        );
    }

    /// F-S2107-RECON-001 site 2: `extract_blockquote_pairs` must not panic when
    /// the byte immediately after `S-XX.YY=` is a non-hex multibyte UTF-8
    /// character, AND must correctly resume scanning past it to find a later
    /// well-formed pair on the same line. Pre-fix: `search_pos = abs +
    /// hash_start + hash.len().max(1)` lands mid-char inside the em-dash, and
    /// the next loop iteration's `&rest[search_pos..].find("S-")` panics.
    /// Post-fix: the retry step advances by the em-dash's actual UTF-8 byte
    /// length; the malformed `S-21.07=` entry yields no pair, but the
    /// well-formed `S-21.08=1234567` entry later on the same line is still
    /// found (proves the fix recovers correctly, not merely avoids panic).
    #[test]
    fn test_BC_5_39_010_arm_b_extract_blockquote_pairs_multibyte_non_hex_no_panic() {
        let line = "> S-21.07=\u{2014}notahex; S-21.08=1234567\n";
        let pairs = extract_blockquote_pairs(line);
        assert!(
            !pairs.iter().any(|(id, _)| id == "S-21.07"),
            "S-21.07 has no valid hex hash (multibyte non-hex char after '=') and \
            must not appear in the extracted pairs; got {pairs:?}"
        );
        assert!(
            pairs
                .iter()
                .any(|(id, hash)| id == "S-21.08" && hash == "1234567"),
            "S-21.08's well-formed pair later on the same line must still be found \
            after recovering from the malformed S-21.07 entry (BC-5.39.010 \
            F-S2107-RECON-001); got {pairs:?}"
        );
    }

    // -----------------------------------------------------------------------
    // O-1 (adversary observation): PC20/PC21 trailing `\b` (word-boundary)
    // enforcement. BC-5.39.010 PC20 specifies
    // `\binput-hash\s+([0-9a-f]{7,40})\b` and PC21 specifies
    // `\b<id>=([0-9a-f]{7,40})\b` — both require the character immediately
    // AFTER the matched hex run to be either end-of-line or a non-word
    // character. Pre-fix: the hex-run extractors accepted a 7-40-char
    // candidate purely on its length, without checking what followed it — a
    // hash immediately followed by another word character (uppercase hex
    // digit, `g`-`z`, digit, or `_`) fails the spec's trailing `\b` and has
    // NO regex match at that position at all (within a contiguous run, every
    // shorter backtrack length is still followed by a word char), yet the
    // implementation returned a TRUNCATED hash instead of rejecting.
    // -----------------------------------------------------------------------

    /// O-1 RED GATE site 1: `extract_input_hash_token` (PC20) must reject a
    /// hex run immediately followed by a word character. Pre-fix: returns
    /// `Some("47a65c9")` (truncated — drops the trailing `D`). Post-fix:
    /// `None` (no later valid occurrence on this line).
    #[test]
    fn test_BC_5_39_010_arm_b_input_hash_token_trailing_word_char_rejected() {
        let line = "| S-21.07 | ... | input-hash 47a65c9D | ...";
        let result = extract_input_hash_token(line);
        assert_eq!(
            result, None,
            "PC20 trailing \\b fails ('D' immediately follows the hex run) — \
            no valid match exists on this line; got truncated {result:?} (O-1)"
        );
    }

    /// O-1 RED GATE site 1b: hex run of 10 chars immediately followed by
    /// non-hex-but-alphanumeric 'g' — every backtrack length (7..=10) is
    /// still followed by a word char, so PC20 has no match anywhere in the
    /// run. Pre-fix: returns `Some("47a65c9abc")` (truncated at the 10th hex
    /// char, dropping 'g'). Post-fix: `None`.
    #[test]
    fn test_BC_5_39_010_arm_b_input_hash_token_trailing_alnum_non_hex_rejected() {
        let line = "| S-21.07 | ... | input-hash 47a65c9abcg | ...";
        let result = extract_input_hash_token(line);
        assert_eq!(
            result, None,
            "PC20 trailing \\b fails ('g' immediately follows the hex run, \
            still a word char even though not hex) — no valid match exists; \
            got truncated {result:?} (O-1)"
        );
    }

    /// O-1 continuation guard: a trailing-boundary failure on the FIRST
    /// occurrence must not abort the whole scan — a later, well-formed
    /// occurrence on the same line must still be found.
    #[test]
    fn test_BC_5_39_010_arm_b_input_hash_token_trailing_failure_then_later_valid_match() {
        let line = "input-hash 47a65c9D ... input-hash 8899aab ";
        let result = extract_input_hash_token(line);
        assert_eq!(
            result,
            Some("8899aab".to_string()),
            "first occurrence fails PC20 trailing \\b; scan must continue and \
            find the second, well-formed occurrence (O-1)"
        );
    }

    /// O-1 positive control: a hash followed by whitespace still extracts
    /// correctly (trailing \b succeeds — whitespace is not a word char).
    #[test]
    fn test_BC_5_39_010_arm_b_input_hash_token_trailing_whitespace_control() {
        let line = "input-hash 47a65c9 ";
        let result = extract_input_hash_token(line);
        assert_eq!(
            result,
            Some("47a65c9".to_string()),
            "hash followed by whitespace must still extract (positive control, O-1)"
        );
    }

    /// O-1 RED GATE site 2: `extract_blockquote_pairs` (PC21) must reject a
    /// hex run immediately followed by `_` (a word character in \b
    /// semantics). Pre-fix: returns `[("S-21.07", "47a65c9")]` (truncated,
    /// dropping the trailing `_`). Post-fix: no pair for S-21.07.
    #[test]
    fn test_BC_5_39_010_arm_b_blockquote_pairs_trailing_underscore_rejected() {
        let line = "> S-21.07=47a65c9_\n";
        let pairs = extract_blockquote_pairs(line);
        assert!(
            !pairs.iter().any(|(id, _)| id == "S-21.07"),
            "PC21 trailing \\b fails ('_' immediately follows the hex run, a \
            word char) — S-21.07 must not appear in extracted pairs; \
            got {pairs:?} (O-1)"
        );
    }

    /// O-1 continuation guard: a trailing-boundary failure on the first
    /// pair must not abort the scan — a later, well-formed pair on the same
    /// line must still be found.
    #[test]
    fn test_BC_5_39_010_arm_b_blockquote_pairs_trailing_failure_then_later_valid_pair() {
        let line = "> S-21.07=47a65c9D; S-21.08=8899aab\n";
        let pairs = extract_blockquote_pairs(line);
        assert!(
            !pairs.iter().any(|(id, _)| id == "S-21.07"),
            "S-21.07's hash fails PC21 trailing \\b; must not appear; got {pairs:?} (O-1)"
        );
        assert!(
            pairs
                .iter()
                .any(|(id, hash)| id == "S-21.08" && hash == "8899aab"),
            "S-21.08's well-formed pair later on the same line must still be \
            found (O-1); got {pairs:?}"
        );
    }

    /// O-1 positive control: a pair followed by `;` still extracts correctly.
    #[test]
    fn test_BC_5_39_010_arm_b_blockquote_pairs_trailing_semicolon_control() {
        let line = "> S-21.07=47a65c9;\n";
        let pairs = extract_blockquote_pairs(line);
        assert_eq!(
            pairs,
            vec![("S-21.07".to_string(), "47a65c9".to_string())],
            "hash followed by ';' must still extract (positive control, O-1)"
        );
    }

    /// O-1 RED GATE site 3: `parse_story_index_blockquote_hash` (PC21) must
    /// reject a hex run immediately followed by an uppercase letter ('F' is
    /// alphanumeric — a word char — even though it is outside the
    /// deliberately lowercase-only [0-9a-f] charset). Pre-fix: returns
    /// `Some("deadbee5")` (truncated). Post-fix: `None`.
    #[test]
    fn test_BC_5_39_010_arm_b_blockquote_hash_trailing_uppercase_rejected() {
        let content = "> S-21.08=deadbee5F\n";
        let result = parse_story_index_blockquote_hash(content, "S-21.08");
        assert_eq!(
            result, None,
            "PC21 trailing \\b fails ('F' immediately follows the hex run) — \
            no valid match exists; got truncated {result:?} (O-1)"
        );
    }

    /// O-1 positive control: `parse_story_index_blockquote_hash` still
    /// extracts correctly when the hash is followed by whitespace/EOL.
    #[test]
    fn test_BC_5_39_010_arm_b_blockquote_hash_trailing_eol_control() {
        let content = "> S-21.07=47a65c9\n";
        let result = parse_story_index_blockquote_hash(content, "S-21.07");
        assert_eq!(
            result,
            Some("47a65c9".to_string()),
            "hash followed by EOL must still extract (positive control, O-1)"
        );
    }

    // -----------------------------------------------------------------------
    // F-S2107-RECON-004 (adversary observation): the LEADING `\b`
    // (word-boundary) checks in the three PC20/PC21 hex-run extractors used
    // `!prev.is_ascii_alphanumeric()` only — omitting `_` — while the O-1
    // trailing-boundary helper (`trailing_word_boundary_ok`) correctly
    // treats `_` as a word char. Two different `\b` word-char definitions in
    // the same functions. A preceding `_` is a word char under regex `\b`
    // semantics (`[A-Za-z0-9_]`), so `\binput-hash` / `\b<id>=` must NOT
    // match immediately after a `_`. Pre-fix: the leading checks wrongly
    // treated `_` as a boundary and accepted the match. Post-fix: both
    // leading and trailing checks share one `is_word_char` predicate.
    // -----------------------------------------------------------------------

    /// F-S2107-RECON-004 RED GATE site 1: `extract_input_hash_token` (PC20)
    /// must reject a keyword occurrence immediately preceded by `_` (a word
    /// char under `\b`). Pre-fix: returns `Some("47a65c9")` (wrongly treats
    /// `_` as a boundary). Post-fix: `None` (no later valid occurrence on
    /// this line).
    #[test]
    fn test_BC_5_39_010_arm_b_input_hash_token_leading_underscore_rejected() {
        let line = "x_input-hash 47a65c9";
        let result = extract_input_hash_token(line);
        assert_eq!(
            result, None,
            "PC20 leading \\b fails ('_' immediately precedes 'input-hash', a \
            word char) — no valid match exists on this line; got {result:?} \
            (F-S2107-RECON-004)"
        );
    }

    /// F-S2107-RECON-004 continuation guard: a leading-boundary failure on
    /// the first occurrence must not abort the whole scan — a later,
    /// well-formed occurrence on the same line must still be found.
    #[test]
    fn test_BC_5_39_010_arm_b_input_hash_token_leading_underscore_then_later_valid_match() {
        let line = "_input-hash bad, input-hash 47a65c9";
        let result = extract_input_hash_token(line);
        assert_eq!(
            result,
            Some("47a65c9".to_string()),
            "first occurrence fails PC20 leading \\b ('_' precedes it); scan \
            must continue and find the later, well-formed occurrence \
            (F-S2107-RECON-004)"
        );
    }

    /// F-S2107-RECON-004 positive controls: `extract_input_hash_token` must
    /// still extract when the keyword is preceded by space, `|`, `;`, or is
    /// at the start of the line (none of these are word chars).
    #[test]
    fn test_BC_5_39_010_arm_b_input_hash_token_leading_boundary_positive_controls() {
        assert_eq!(
            extract_input_hash_token("foo input-hash 47a65c9"),
            Some("47a65c9".to_string()),
            "leading boundary preceded by space must still extract \
            (positive control, F-S2107-RECON-004)"
        );
        assert_eq!(
            extract_input_hash_token("|input-hash 47a65c9"),
            Some("47a65c9".to_string()),
            "leading boundary preceded by '|' must still extract \
            (positive control, F-S2107-RECON-004)"
        );
        assert_eq!(
            extract_input_hash_token(";input-hash 47a65c9"),
            Some("47a65c9".to_string()),
            "leading boundary preceded by ';' must still extract \
            (positive control, F-S2107-RECON-004)"
        );
        assert_eq!(
            extract_input_hash_token("input-hash 47a65c9"),
            Some("47a65c9".to_string()),
            "leading boundary at start-of-line must still extract \
            (positive control, F-S2107-RECON-004)"
        );
    }

    /// F-S2107-RECON-004 RED GATE site 2: `parse_story_index_blockquote_hash`
    /// (PC21) must reject a story-ID occurrence immediately preceded by `_`.
    /// Pre-fix: returns `Some("deadbee5")`. Post-fix: `None`.
    #[test]
    fn test_BC_5_39_010_arm_b_blockquote_hash_leading_underscore_rejected() {
        let content = "> x_S-21.07=deadbee5\n";
        let result = parse_story_index_blockquote_hash(content, "S-21.07");
        assert_eq!(
            result, None,
            "PC21 leading \\b fails ('_' immediately precedes 'S-21.07=', a \
            word char) — no valid match exists; got {result:?} \
            (F-S2107-RECON-004)"
        );
    }

    /// F-S2107-RECON-004 continuation guard: a leading-boundary failure on
    /// the first occurrence must not abort the scan — a later, well-formed
    /// occurrence must still be found.
    #[test]
    fn test_BC_5_39_010_arm_b_blockquote_hash_leading_underscore_then_later_valid_match() {
        let content = "> _S-21.07=deadbee5 S-21.07=8899aab\n";
        let result = parse_story_index_blockquote_hash(content, "S-21.07");
        assert_eq!(
            result,
            Some("8899aab".to_string()),
            "first occurrence fails PC21 leading \\b ('_' precedes it); scan \
            must continue and find the later, well-formed occurrence \
            (F-S2107-RECON-004)"
        );
    }

    /// F-S2107-RECON-004 positive controls: `parse_story_index_blockquote_hash`
    /// must still extract when the story ID is preceded by space or `;`
    /// (start-of-line is not reachable here — every qualifying line starts
    /// with the literal `"> "` prefix).
    #[test]
    fn test_BC_5_39_010_arm_b_blockquote_hash_leading_boundary_positive_controls() {
        assert_eq!(
            parse_story_index_blockquote_hash("> S-21.07=47a65c9\n", "S-21.07"),
            Some("47a65c9".to_string()),
            "leading boundary preceded by space (after '> ') must still \
            extract (positive control, F-S2107-RECON-004)"
        );
        assert_eq!(
            parse_story_index_blockquote_hash("> ;S-21.07=47a65c9\n", "S-21.07"),
            Some("47a65c9".to_string()),
            "leading boundary preceded by ';' must still extract \
            (positive control, F-S2107-RECON-004)"
        );
    }

    /// F-S2107-RECON-004 RED GATE site 3: `extract_blockquote_pairs` (PC21)
    /// must reject an `S-` occurrence immediately preceded by `_`. Pre-fix:
    /// returns `[("S-21.07", "47a65c9")]`. Post-fix: no pair for S-21.07.
    #[test]
    fn test_BC_5_39_010_arm_b_blockquote_pairs_leading_underscore_rejected() {
        let line = "> _S-21.07=47a65c9\n";
        let pairs = extract_blockquote_pairs(line);
        assert!(
            !pairs.iter().any(|(id, _)| id == "S-21.07"),
            "PC21 leading \\b fails ('_' immediately precedes 'S-21.07', a \
            word char) — S-21.07 must not appear in extracted pairs; \
            got {pairs:?} (F-S2107-RECON-004)"
        );
    }

    /// F-S2107-RECON-004 continuation guard: a leading-boundary failure on
    /// the first pair must not abort the scan — a later, well-formed pair on
    /// the same line must still be found.
    #[test]
    fn test_BC_5_39_010_arm_b_blockquote_pairs_leading_underscore_then_later_valid_pair() {
        let line = "> _S-21.07=47a65c9; S-21.08=8899aab\n";
        let pairs = extract_blockquote_pairs(line);
        assert!(
            !pairs.iter().any(|(id, _)| id == "S-21.07"),
            "S-21.07's leading boundary fails PC21 ('_' precedes it); must \
            not appear; got {pairs:?} (F-S2107-RECON-004)"
        );
        assert!(
            pairs
                .iter()
                .any(|(id, hash)| id == "S-21.08" && hash == "8899aab"),
            "S-21.08's well-formed pair later on the same line must still be \
            found (F-S2107-RECON-004); got {pairs:?}"
        );
    }

    /// F-S2107-RECON-004 positive controls: `extract_blockquote_pairs` must
    /// still extract when the story ID is preceded by space (start of
    /// `rest` after the `"> "` prefix is stripped) or `;` (the production
    /// semicolon separator between pairs).
    #[test]
    fn test_BC_5_39_010_arm_b_blockquote_pairs_leading_boundary_positive_controls() {
        assert_eq!(
            extract_blockquote_pairs("> S-21.07=47a65c9\n"),
            vec![("S-21.07".to_string(), "47a65c9".to_string())],
            "leading boundary at start-of-rest (immediately after '> ') must \
            still extract (positive control, F-S2107-RECON-004)"
        );
        let pairs = extract_blockquote_pairs("> S-21.07=47a65c9;S-21.08=8899aab\n");
        assert!(
            pairs.contains(&("S-21.07".to_string(), "47a65c9".to_string())),
            "leading boundary preceded by space must still extract \
            (positive control, F-S2107-RECON-004); got {pairs:?}"
        );
        assert!(
            pairs.contains(&("S-21.08".to_string(), "8899aab".to_string())),
            "leading boundary preceded by ';' must still extract \
            (positive control, F-S2107-RECON-004); got {pairs:?}"
        );
    }
}
