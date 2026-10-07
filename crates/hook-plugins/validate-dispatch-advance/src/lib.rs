//! validate-dispatch-advance — PostToolUse WASM hook plugin.
//!
//! Blocks any Edit/Write to `STATE.md` whose `current_step:` frontmatter field
//! contains structural violations, and any Edit/Write to `INDEX.md` containing
//! adversary-pass rows with the wrong column count.
//!
//! # Validation gates (STATE.md arm)
//!
//! 1. **Forbidden meta-commentary** (D-440(a)+D-441(a)+D-442(a)): patterns
//!    `META-LEVEL-\d+ WATCH`, `self-app TEST`, `expected verdict` must not
//!    appear in the `current_step:` frontmatter value.
//!
//! 2. **4-index version citations** (D-439(b)): all four literal substrings
//!    `BC-INDEX v`, `VP-INDEX v`, `STORY-INDEX v`, `ARCH-INDEX v` must be
//!    present in `current_step:`.
//!
//! 3. **Trajectory-tail LENGTH=4** (D-451(c)): the global regex `→(\d+)` must
//!    match exactly 4 times in the `current_step:` value.
//!
//! 4. **D-chain cite currency** (D-443(a)): `current_step:` must contain at
//!    least one `D-\d+` reference. The maximum D-NNN integer found in
//!    `current_step:` must be >= the maximum D-NNN integer visible anywhere
//!    in STATE.md. The literal `D-382..D-` range prefix is NOT required;
//!    prose forms like `D-chain cite D-476` are accepted (BC-5.39.006 v1.1).
//!
//! # Validation gate (INDEX.md arm)
//!
//! 5. **5-column adversary-pass rows** (D-441(b)/D-442(b)): within the
//!    `## Adversarial Reviews` h2 section only, rows must have 6 pipe characters
//!    (1 leading + 4 internal + 1 trailing = 5 columns). Rows under other h2
//!    headings are not validated. Separator rows excluded. Historical 4-column
//!    sections (5-pipe header) are grandfathered and skipped entirely.
//!
//! # Behavioral Contracts
//!
//! - BC-5.39.006: blocks dispatch-advance structural violations.
//!
//! # D-NNN closures
//!
//! - D-440(a): forbidden meta-commentary prohibition.
//! - D-441(a): verbatim-strict `current_step:` — no meta-commentary injection.
//! - D-442(a): prescribed clause order — forbidden-pattern gate prevents markers.
//! - D-443(a): D-chain cite currency gate.
//! - D-439(b): all 4 index version cite patterns required.
//! - D-441(b)/D-442(b): 5-column INDEX.md adversary-pass row schema (6 pipes) within
//!   `## Adversarial Reviews` h2 section; historical 4-col rows grandfathered.
//! - D-451(c): trajectory-tail LENGTH=4.
//!
//! # Architecture compliance
//!
//! - HOST_ABI_VERSION = 1 (no new host functions introduced).
//! - Fail-open on every `host::read_file` error (BC-5.39.006 invariant 9).
//! - No `println!` — use `host::log_*` for all diagnostic output.
//! - No `unwrap()` or `expect()` in production paths.
//! - No `regex` crate: hand-rolled pattern scanning to stay within WASM fuel budget.
//! - File-path enforcement via in-plugin guard using `Path::file_name` (Q5/Q6 canonical
//!   pattern; NOT `ends_with("STATE.md")` or `ends_with("INDEX.md")` — false-positive
//!   on paths like `xSTATE.md` or `xINDEX.md`).
//! - `tool = "Edit|Write"` is the canonical Q5/Q6 form for this hook's registry entry.
//! - All byte-index slice expressions with possible multi-byte UTF-8 input MUST use
//!   `is_char_boundary()` guards (BC-5.39.006 invariant 10; S-15.11 F-P4-001 lesson).

use vsdd_hook_sdk::{HookPayload, HookResult};

/// HOST_ABI_VERSION declares the ABI contract version this plugin was built
/// against. Must remain 1.
pub const HOST_ABI_VERSION: u32 = 1;

/// Maximum bytes to read from STATE.md or INDEX.md via `host::read_file`.
///
/// Set to 512 KiB (524288 bytes) — parity with BC-5.39.005/BC-5.39.006
/// cap and the validate-state-structure sibling crate (F-P5-002 fix).
/// Real STATE.md as of the F-P5-002 fix burst is 95185 bytes (95 KiB);
/// INDEX.md is typically smaller. The 4x growth runway accommodates future
/// expansion without requiring a further cap raise.
///
/// BC-5.39.006 precondition 4.
pub const MAX_BYTES: u32 = 524_288;

// ---------------------------------------------------------------------------
// Violation type
// ---------------------------------------------------------------------------

/// A structural violation found in the write target content.
///
/// Carries both a human-readable `description` (used verbatim in the block
/// message) and the `cited_raw` body-literal form of the offending text.
/// Structural plumbing per TD-VSDD-059 paper-fix avoidance: block messages
/// quote the exact string the author wrote, not a paraphrase.
#[derive(Debug, Clone)]
pub struct Violation {
    /// Human-readable description of the violation, used in the block message.
    pub description: String,
    /// The raw source text that triggered the violation.
    /// Populated via `.trim_end().to_string()` at every violation site.
    /// Required per TD-VSDD-059 paper-fix avoidance (structural plumbing
    /// from day 1 per S-15.07+S-15.11 cascade lessons).
    pub cited_raw: String,
}

// ---------------------------------------------------------------------------
// File-path guards
// ---------------------------------------------------------------------------

/// Returns `true` if `file_path` names a file whose `file_name` component is
/// exactly `STATE.md`.
///
/// Uses path-component-strict matching (`Path::file_name`) rather than
/// `ends_with`, preventing false-positive fires on paths like
/// `/some/dir/xSTATE.md`.
///
/// Returns `false` if the path has no file-name component (e.g. `/`).
///
/// # BC trace
/// BC-5.39.006 precondition 1 — hook STATE.md arm activates only on exact-name match.
/// BC-5.39.006 invariant 3 — path-component-strict matching is mandatory.
/// AC-16 / AC-17 — `xSTATE.md` and `xINDEX.md` must not trigger validation.
pub fn is_state_md_target(file_path: &str) -> bool {
    std::path::Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        == Some("STATE.md")
}

/// Returns `true` if `file_path` names a file whose `file_name` component is
/// exactly `INDEX.md`.
///
/// Uses path-component-strict matching (`Path::file_name`) rather than
/// `ends_with`, preventing false-positive fires on paths like
/// `/some/dir/xINDEX.md`.
///
/// Returns `false` if the path has no file-name component (e.g. `/`).
///
/// # BC trace
/// BC-5.39.006 precondition 1 — hook INDEX.md arm activates only on exact-name match.
/// BC-5.39.006 invariant 3 — path-component-strict matching is mandatory.
pub fn is_index_md_target(file_path: &str) -> bool {
    std::path::Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        == Some("INDEX.md")
}

// ---------------------------------------------------------------------------
// current_step: extraction
// ---------------------------------------------------------------------------

/// Extract the `current_step:` frontmatter value from YAML frontmatter.
///
/// YAML frontmatter is bounded by `---` delimiters. Scans for the line
/// starting with `current_step:` within the first frontmatter block and
/// returns the rest of that line as the value (trimming leading whitespace).
///
/// Byte-index slice expressions guard `is_char_boundary()` before slicing
/// to avoid panics on multi-byte UTF-8 content (em-dash, en-dash, NBSP in
/// STATE.md narrative text). BC-5.39.006 invariant 10; S-15.11 F-P4-001.
///
/// Returns `None` if the frontmatter block is absent, the delimiter is not
/// found, or `current_step:` is not present in frontmatter.
///
/// # BC trace
/// BC-5.39.006 invariant 4 — validation is anchored to `current_step:` only.
fn extract_current_step(content: &str) -> Option<&str> {
    // Find the start of YAML frontmatter: content must begin with `---`.
    // Skip leading whitespace/newlines to be tolerant of BOM-free UTF-8.
    let content = content.trim_start_matches('\u{feff}'); // strip BOM if present
    let after_open = content.strip_prefix("---")?;
    // Skip the newline after the opening delimiter.
    let body_start = after_open
        .strip_prefix('\n')
        .or_else(|| after_open.strip_prefix("\r\n"))
        .unwrap_or(after_open);

    // Find the closing `---` delimiter.
    // Use line-by-line search so we don't match `---` inside values.
    let mut fm_end = None;
    let mut offset = 0usize;
    for line in body_start.lines() {
        if line == "---" {
            fm_end = Some(offset);
            break;
        }
        // Advance offset by line length + newline.
        // We use safe `get` to avoid non-boundary slicing (invariant 10).
        offset += line.len() + 1; // +1 for '\n'; \r\n handled by lines()
    }
    let fm_body = match fm_end {
        Some(end) => {
            // Guard: ensure `end` is on a char boundary before slicing.
            if body_start.is_char_boundary(end) {
                &body_start[..end]
            } else {
                // Fallback: scan from start of body_start to the unsafe index
                // using the safe .get() accessor. If boundary is invalid, bail.
                body_start.get(..end)?
            }
        }
        None => body_start, // No closing delimiter; scan full remainder.
    };

    // Scan frontmatter body for the `current_step:` key.
    for line in fm_body.lines() {
        let trimmed = line.trim_start();
        let prefix = "current_step:";
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            // Value is the rest of the line, trimmed of leading whitespace and quotes.
            let value = rest.trim();
            // Strip optional surrounding single or double quotes.
            let value = value
                .strip_prefix('\'')
                .and_then(|v| v.strip_suffix('\''))
                .or_else(|| value.strip_prefix('"').and_then(|v| v.strip_suffix('"')))
                .unwrap_or(value);
            return Some(value);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// STATE.md validation checks
// ---------------------------------------------------------------------------

/// Check for forbidden meta-commentary patterns in `current_step:` value.
///
/// Scans for all occurrences of:
/// - `META-LEVEL-\d+ WATCH` (hand-rolled: `META-LEVEL-` prefix + digit run + ` WATCH`)
/// - `self-app TEST` (literal substring)
/// - `expected verdict` (literal substring)
///
/// Returns `Vec<Violation>` — all matched patterns are enumerated. Each
/// violation has `cited_raw` set to the matched substring and description
/// citing D-440(a)+D-441(a)+D-442(a).
///
/// Changed from `Option<Violation>` to `Vec<Violation>` to support multi-pattern
/// enumeration per BC-5.39.006 PC-6 (F-P1-012 fix).
///
/// # BC trace
/// BC-5.39.006 v1.1 postcondition 2/6; D-440(a)+D-441(a)+D-442(a); AC-1/AC-2/AC-3.
fn check_forbidden_meta_commentary(current_step_value: &str) -> Vec<Violation> {
    let mut violations: Vec<Violation> = Vec::new();

    // Hand-rolled scan for `META-LEVEL-\d+ WATCH` (BC-5.39.006 invariant 4).
    // The pattern is: literal "META-LEVEL-" followed by one or more digits,
    // followed by " WATCH". Scan globally (there may be multiple such occurrences).
    let meta_prefix = "META-LEVEL-";
    let meta_suffix = " WATCH";
    let mut search = current_step_value;
    while let Some(pos) = search.find(meta_prefix) {
        let after_prefix = &search[pos + meta_prefix.len()..];
        let digit_end = after_prefix
            .char_indices()
            .take_while(|(_, c)| c.is_ascii_digit())
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);
        if digit_end > 0 {
            let after_digits = &after_prefix[digit_end..];
            if after_digits.starts_with(meta_suffix) {
                let matched_end = pos + meta_prefix.len() + digit_end + meta_suffix.len();
                let cited_raw = search
                    .get(pos..matched_end)
                    .unwrap_or(meta_prefix)
                    .to_string();
                violations.push(Violation {
                    description: format!(
                        "forbidden meta-commentary pattern found in current_step: \
                         matched pattern `META-LEVEL-N WATCH`; \
                         cited: \"{cited_raw}\"; \
                         D-440(a)+D-441(a)+D-442(a)"
                    ),
                    cited_raw,
                });
                // Advance past this match to find further occurrences.
                let advance = pos + meta_prefix.len() + digit_end + meta_suffix.len();
                if advance >= search.len() {
                    break;
                }
                search = &search[advance..];
                continue;
            }
        }
        // Advance past this `META-LEVEL-` occurrence (no match) to avoid infinite loop.
        let advance = pos + meta_prefix.len();
        if advance >= search.len() {
            break;
        }
        search = &search[advance..];
    }

    // Literal substring: `self-app TEST`.
    if current_step_value.contains("self-app TEST") {
        violations.push(Violation {
            description: "forbidden meta-commentary pattern found in current_step: \
                          matched pattern `self-app TEST`; \
                          D-440(a)+D-441(a)+D-442(a)"
                .to_string(),
            cited_raw: "self-app TEST".to_string(),
        });
    }

    // Literal substring: `expected verdict`.
    if current_step_value.contains("expected verdict") {
        violations.push(Violation {
            description: "forbidden meta-commentary pattern found in current_step: \
                          matched pattern `expected verdict`; \
                          D-440(a)+D-441(a)+D-442(a)"
                .to_string(),
            cited_raw: "expected verdict".to_string(),
        });
    }

    violations
}

/// Check that all 4 required index version citation patterns are present in
/// `current_step:` value.
///
/// Required literal substrings (each followed by a version token):
/// - `BC-INDEX v`
/// - `VP-INDEX v`
/// - `STORY-INDEX v`
/// - `ARCH-INDEX v`
///
/// If any are absent: returns `Some(Violation)` naming each missing cite,
/// citing D-439(b).
///
/// # BC trace
/// BC-5.39.006 postcondition 3; D-439(b); AC-4; invariant 5.
fn check_index_version_cites(current_step_value: &str) -> Option<Violation> {
    const REQUIRED: &[&str] = &["BC-INDEX v", "VP-INDEX v", "STORY-INDEX v", "ARCH-INDEX v"];
    let missing: Vec<&str> = REQUIRED
        .iter()
        .copied()
        .filter(|pat| !current_step_value.contains(pat))
        .collect();

    if missing.is_empty() {
        return None;
    }

    let missing_list = missing.join(", ");
    Some(Violation {
        description: format!(
            "missing required index version citation(s) in current_step: {missing_list}; \
             all 4 patterns (BC-INDEX v, VP-INDEX v, STORY-INDEX v, ARCH-INDEX v) \
             must be present per D-439(b)"
        ),
        cited_raw: missing_list,
    })
}

/// Check that trajectory-tail in `current_step:` contains exactly 4 `→(\d+)` matches.
///
/// # BC-5.39.006 v1.2 PC 6 + invariant 6 — prefix-mandatory semantics
///
/// The literal prefix `trajectory-tail ` (with trailing space) MUST be present
/// in `current_step_value`. Absence is itself a HARD violation — the LENGTH
/// count is skipped entirely when the prefix is absent (prefix-absent block fires
/// first; no fallback to global count).
///
/// When the prefix IS present, counting is scoped to the substring that starts
/// at the prefix and ends at the first `;` (segment separator) or end-of-string —
/// isolating the canonical `→N→N→N→N` run from incidental `→NNN` patterns
/// elsewhere in the value (e.g., `TD-VSDD-064/065→095/096` TD-renumber
/// references). This prevents false-positive fires on valid STATE.md content
/// that uses `→NNN` TD-reference notation (F-P2-006 production false-positive fix).
///
/// # UTF-8 / slice-safety note
/// `str::find` returns a byte index. `prefix.len()` is the UTF-8 byte length of
/// the ASCII string `"trajectory-tail "` (all single-byte chars), so
/// `idx + prefix.len()` is always a valid char boundary by construction — no
/// additional `.get(..)` guard is required.
///
/// # BC trace
/// BC-5.39.006 v1.2 postcondition 6; D-451(c)/F-P3-006/EC-023; AC-22; invariant 6.
fn check_trajectory_tail_length(current_step_value: &str) -> Option<Violation> {
    const PREFIX: &str = "trajectory-tail ";

    // BC-5.39.006 v1.2 PC 6 + invariant 6: prefix MANDATORY.
    // Absence is a HARD violation; LENGTH count does NOT run when prefix absent.
    let idx = match current_step_value.find(PREFIX) {
        Some(i) => i,
        None => {
            return Some(Violation {
                description: "current_step missing 'trajectory-tail ' canonical marker; \
                              D-451(c)/F-P3-006/EC-023"
                    .to_string(),
                cited_raw: current_step_value.chars().take(120).collect::<String>(),
            });
        }
    };

    // Slice safety: idx is from find(), PREFIX.len() is pure ASCII — idx + PREFIX.len()
    // is a valid char boundary by construction.
    let scoped_from_prefix = &current_step_value[idx + PREFIX.len()..];
    // Extent ends at the first `;` (segment separator) or end-of-string.
    let scan_target = match scoped_from_prefix.find(';') {
        Some(semi_pos) => &scoped_from_prefix[..semi_pos],
        None => scoped_from_prefix,
    };

    let count = count_arrow_digit_matches(scan_target);
    if count == 4 {
        return None;
    }
    Some(Violation {
        description: format!(
            "trajectory-tail in current_step: has {count} `→N` component(s); \
             required LENGTH=4 per D-451(c); \
             canonical form is `→N→N→N→N`"
        ),
        cited_raw: format!("found {count} arrow-digit matches"),
    })
}

/// Count `→N` (arrow followed by one or more ASCII digits) occurrences in `s`.
///
/// Uses `char_indices()` for safe UTF-8 iteration. Arrow U+2192 = 3 bytes.
/// ASCII digit bytes are single-byte and cannot be UTF-8 continuation bytes —
/// safe to check via `bytes[j].is_ascii_digit()` in the inner walk.
///
/// This is the same algorithm used in the validate-state-structure sibling
/// (S-15.09) for trajectory-tail counting; kept consistent for cohesion.
fn count_arrow_digit_matches(s: &str) -> usize {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let arrow_byte_len = '\u{2192}'.len_utf8(); // 3 bytes

    let mut count = 0usize;
    let mut skip_until_byte: usize = 0;

    for (byte_pos, ch) in s.char_indices() {
        if byte_pos < skip_until_byte {
            continue;
        }
        if ch == '\u{2192}' {
            let after = byte_pos + arrow_byte_len;
            if after < len && bytes[after].is_ascii_digit() {
                count += 1;
                // Skip past all trailing digits to avoid double-counting.
                let mut j = after;
                while j < len && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                skip_until_byte = j;
            }
        }
    }
    count
}

/// Check that the D-chain cite in `current_step:` is current.
///
/// BC-5.39.006 v1.1 invariant 7 — D-(\\d+) max-extraction semantics:
///
/// 1. Extract ALL `D-(\d+)` integers from `current_step_value` globally;
///    take the maximum integer found (call it `max_cited`).
/// 2. Extract ALL `D-(\d+)` integers from the full STATE.md `content`;
///    take the maximum integer found (call it `max_in_file`).
/// 3. If `max_cited` is None (no D-NNN reference in current_step): violation.
/// 4. If `max_cited < max_in_file`: stale cite violation.
/// 5. If `max_cited >= max_in_file`: current (or fail-open; no violation).
///
/// The literal prefix `D-382..` is NOT required — production `current_step:`
/// uses prose forms like `D-chain cite D-476 latest brownfield`.
///
/// Fail-open design: when the body has no D-NNN at a higher integer than
/// current_step cites, returns None to avoid blocking in-progress writes.
///
/// # BC trace
/// BC-5.39.006 v1.1 postcondition 5; D-443(a); EC-008/EC-009/EC-010; invariant 7.
fn check_d_chain_currency(content: &str, current_step_value: &str) -> Option<Violation> {
    // Step 1: extract max D-NNN from current_step_value.
    let max_cited = scan_max_d_nnn(current_step_value);

    if max_cited == 0 {
        // No D-NNN reference at all in current_step — violation per invariant 7.
        return Some(Violation {
            description: "D-chain cite absent from current_step: no `D-\\d+` reference found; \
                 current_step: must contain a D-NNN cite per D-443(a); \
                 production form: `D-chain cite D-NNN latest brownfield`"
                .to_string(),
            cited_raw: String::new(),
        });
    }

    // Step 2: extract max D-NNN from full STATE.md content.
    let max_in_file = scan_max_decision_log_id(content);

    if max_cited < max_in_file {
        Some(Violation {
            description: format!(
                "D-chain cite in current_step: is stale: max_cited D-{max_cited} \
                 but STATE.md body shows D-{max_in_file} as latest; \
                 update D-chain cite to include D-{max_in_file} per D-443(a)"
            ),
            cited_raw: format!("D-{max_cited}"),
        })
    } else {
        // Current or fail-open per BC-5.39.006 v1.1 invariant 7.
        None
    }
}

/// Scan `s` for all `D-NNN` patterns (where NNN is one or more ASCII digits)
/// and return the maximum integer found. Returns 0 if no match.
///
/// Uses the same hand-rolled scanning strategy as the sibling functions to
/// avoid the regex crate (WASM fuel budget constraint).
///
/// The scan does NOT skip any D-NNN patterns based on self-reference — the
/// caller (check_d_chain_currency) feeds current_step_value and full content
/// separately, so self-reference exclusion is unnecessary: taking the max of
/// current_step's own D-NNNs as `max_cited` is correct by construction.
///
/// # Word-boundary rule (F-P1-006-B / RC25-RELEASED-2026 false-positive class)
/// A `D-` match only counts as a decision reference when the `D` sits at a
/// word boundary — i.e. `pos == 0` or the byte immediately before `pos` is
/// NOT ASCII-alphanumeric. This prevents two confirmed false-positive
/// classes from being miscounted as `D-NNN` decision cites:
///
/// 1. **Date-embedded digits** — banner text like
///    `RC25-RELEASED-2026-09-04` contains the substring `D-2026` (the
///    trailing "D" of "RELEASED" immediately followed by "-2026"); without
///    the boundary check this is misread as decision D-2026, inflating
///    `max_in_file` far past the real max and flagging a legitimate
///    `current_step:` cite as stale.
/// 2. **Word-internal identifiers** — `TD-VSDD-053` contains the substring
///    `D-053` (the "D" of "TD" is itself preceded by the alphanumeric "T");
///    without the boundary check this is misread as decision D-053.
///
/// A non-boundary match is skipped (not counted) but the search cursor still
/// advances past it, so scanning continues safely without an infinite loop.
///
/// # Slice safety (BC-5.39.006 invariant 10)
/// Advances by `pos + 2 + digit_len` bytes, which always lands on an ASCII
/// boundary (digits are single-byte; "D-" is ASCII). Guards `>= len` before
/// slicing to prevent out-of-bounds access.
///
/// # BC trace
/// BC-5.39.006 v1.1 invariant 7 — D-(\d+) max-extraction.
fn scan_max_d_nnn(s: &str) -> u64 {
    let mut max = 0u64;
    let mut search = s;
    let mut absolute_offset = 0usize;
    let bytes = s.as_bytes();

    while let Some(pos) = search.find("D-") {
        let after = &search[pos + 2..];
        // Extract digits after `D-`.
        let digit_str: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();

        // Word-boundary check: the byte immediately before this "D" (in the
        // original string) must be absent (start of string) or non-
        // alphanumeric for this match to count as a real D-NNN reference.
        let d_absolute_pos = absolute_offset + pos;
        let is_boundary = d_absolute_pos == 0 || !bytes[d_absolute_pos - 1].is_ascii_alphanumeric();

        if is_boundary && let Ok(n) = digit_str.parse::<u64>() {
            max = max.max(n);
        }

        // Advance past pos + "D-" + digit_len.
        // `pos + 2 + digit_len` lands on an ASCII digit boundary or the byte
        // after the last digit — always a valid UTF-8 char boundary.
        let advance = pos + 2 + digit_str.len();
        if advance >= search.len() {
            break;
        }
        absolute_offset += advance;
        search = &search[advance..];
    }
    max
}

/// Scan `content` for the maximum `D-\d+` integer that appears as a
/// **whole-cell, first-pipe-column** match on a table row under the exact
/// `## Decisions Log` h2 heading. Returns 0 if the section is absent or
/// contains no matching rows (fail-open, matching invariant 7's existing
/// fail-open design philosophy).
///
/// BC-5.39.006 v1.9 invariant 7, Side B (`max_in_file`) — replaces the old
/// whole-body `scan_max_d_nnn(content)` call for `max_in_file` only.
/// `scan_max_d_nnn` (Side A) remains unchanged and is still used for
/// `max_cited` extraction from `current_step:`.
///
/// # Structural scan algorithm (per BC-5.39.006 v1.9 invariant 7 Side B)
///
/// 1. Scan lines for h2 headings (`^## `, after trimming). Enter the
///    `## Decisions Log` section on the exact heading text; exit back to
///    "outside" on ANY subsequent h2 heading (or end-of-file).
/// 2. Inside the section, a line is a candidate row only if its trimmed
///    content both starts and ends with `|`.
/// 3. **(v1.9, EC-026)** A row is classified as a GFM separator row — and
///    skipped — iff, after splitting the row on `|`, EVERY resulting cell's
///    trimmed content is composed solely of `-`, `:`, and whitespace
///    characters, AND the row contains at least one `-` (whole-row
///    composition test). A row is NOT classified as a separator merely
///    because some cell's content *contains* the substring `---` — a
///    genuine data row whose non-ID cell contains `---` (an em-dash
///    rendering, a `D-441---D-449` range citation, etc.) is retained.
/// 4. For each remaining candidate row, split on `|` and take the first
///    non-empty-position pipe-delimited cell (i.e., the first cell after
///    the row's leading `|`), trim it. **(v1.9, EC-025)** Before the
///    whole-cell match, strip SYMMETRIC leading/trailing runs of `*`, `_`,
///    and backtick `` ` `` from the trimmed cell (so `**D-1164**`,
///    `` `D-1164` ``, and `_D-1164_` all normalize to `D-1164`); interior
///    or malformed decoration (e.g. `D-**1164**`) is NOT repaired. Test the
///    normalized cell for a **whole-string** match to `D-\d+` (the ENTIRE
///    normalized cell — not a substring match). A matching cell's integer
///    is added to the candidate set.
/// 5. `max_in_file` is the maximum of the candidate set, or 0 if empty.
///
/// This is immune to the narrative-literal false-positive class **by
/// construction**: a quoted (`"D-2026"`), backtick-quoted, or bare-prose
/// (`decision D-2026`) mention of a `D-NNN` token can occur anywhere in
/// free text outside `## Decisions Log`, or even inside a non-first cell
/// of a `## Decisions Log` row (e.g., in the Summary column's prose), but
/// it can never satisfy "is the entire first pipe-cell of a `## Decisions
/// Log` row" — that column is populated exclusively by state-manager at
/// decision-codification time (D-448(b)).
///
/// # BC trace
/// BC-5.39.006 v1.9 invariant 7 (Side B); EC-024; EC-025; EC-026; D-443(a).
fn scan_max_decision_log_id(content: &str) -> u64 {
    /// State machine for h2-section tracking (mirrors `validate_index_md`'s
    /// `## Adversarial Reviews` scan technique, narrowed to `## Decisions Log`).
    #[derive(PartialEq)]
    enum State {
        /// Not inside `## Decisions Log` section.
        Outside,
        /// Inside `## Decisions Log`.
        Inside,
    }

    let mut max = 0u64;
    let mut state = State::Outside;

    for line in content.lines() {
        let trimmed = line.trim();

        // Detect h2 headings — update section state.
        if trimmed.starts_with("## ") {
            state = if trimmed == "## Decisions Log" {
                State::Inside
            } else {
                State::Outside
            };
            continue;
        }

        if state == State::Outside {
            continue;
        }

        // Must be a table row: trimmed starts and ends with `|`.
        if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
            continue;
        }

        // GFM separator rows are skipped based on whole-row composition
        // (v1.9, EC-026): a row is a separator iff, after splitting on `|`,
        // EVERY resulting cell's trimmed content is composed solely of `-`,
        // `:`, and whitespace characters (an empty cell — e.g. the
        // leading/trailing boundary cells produced by splitting a row that
        // starts and ends with `|` — vacuously satisfies this), AND the row
        // contains at least one `-` character (the "genuine separator, not
        // an all-blank row" guard). A row where any cell contains other
        // characters (letters, digits, prose) is a DATA row and is retained
        // even if some cell contains the substring `---`.
        let is_separator_row = trimmed.contains('-')
            && trimmed.split('|').all(|raw_cell| {
                let cell = raw_cell.trim();
                cell.bytes()
                    .all(|b| matches!(b, b'-' | b':') || b.is_ascii_whitespace())
            });
        if is_separator_row {
            continue;
        }

        // Take the first pipe-delimited cell (the content between the
        // leading `|` and the next `|`), trim it, normalize surrounding
        // markdown emphasis decoration, and test for a whole-string
        // `D-\d+` match.
        let after_leading_pipe = &trimmed[1..];
        let Some(first_cell) = after_leading_pipe.split('|').next() else {
            continue;
        };
        let cell = first_cell.trim();

        // ID-cell emphasis normalization (v1.9, EC-025): strip SYMMETRIC
        // leading/trailing runs of `*`, `_`, and backtick `` ` `` from the
        // trimmed cell before the whole-cell match, so `**D-1164**`,
        // `` `D-1164` ``, and `_D-1164_` all normalize to `D-1164`.
        // Interior/malformed decoration (e.g. `D-**1164**`) is NOT
        // repaired — only symmetric outer runs are stripped.
        let is_emphasis = |b: u8| matches!(b, b'*' | b'_' | b'`');
        let leading_trim = cell.bytes().take_while(|&b| is_emphasis(b)).count();
        let trailing_trim = cell.bytes().rev().take_while(|&b| is_emphasis(b)).count();
        let cell = if leading_trim + trailing_trim < cell.len() {
            &cell[leading_trim..cell.len() - trailing_trim]
        } else {
            ""
        };

        let Some(digits) = cell.strip_prefix("D-") else {
            continue;
        };
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        if let Ok(n) = digits.parse::<u64>() {
            max = max.max(n);
        }
    }

    max
}

/// Extract the `current_cycle:` frontmatter value from YAML frontmatter.
///
/// Uses the same scanning strategy as `extract_current_step`: anchors to the
/// opening `---` delimiter and scans for `current_cycle:` within the first
/// frontmatter block, returning the trimmed value (with optional surrounding
/// quotes stripped).
///
/// Returns `None` if the frontmatter block is absent or `current_cycle:` is
/// not present. Fail-open: callers treat `None` as "unknown cycle" and apply
/// the conservative (F5-applicable) check set.
///
/// # BC trace
/// D-439(b)/D-451(c) F5-scope gate — 4-index-citation + trajectory-tail
/// requirements apply ONLY to the F5 engine-discipline cycle
/// (`v1.0-feature-engine-discipline-pass-1`). This helper enables the gate
/// check in `validate_state_md`.
fn extract_current_cycle(content: &str) -> Option<&str> {
    // Strip BOM if present.
    let content = content.trim_start_matches('\u{feff}');
    let after_open = content.strip_prefix("---")?;
    let body_start = after_open
        .strip_prefix('\n')
        .or_else(|| after_open.strip_prefix("\r\n"))
        .unwrap_or(after_open);

    // Find the closing `---` delimiter.
    let mut fm_end = None;
    let mut offset = 0usize;
    for line in body_start.lines() {
        if line == "---" {
            fm_end = Some(offset);
            break;
        }
        offset += line.len() + 1;
    }
    let fm_body = match fm_end {
        Some(end) => {
            if body_start.is_char_boundary(end) {
                &body_start[..end]
            } else {
                body_start.get(..end)?
            }
        }
        None => body_start,
    };

    // Scan frontmatter body for the `current_cycle:` key.
    for line in fm_body.lines() {
        let trimmed = line.trim_start();
        let prefix = "current_cycle:";
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let value = rest.trim();
            let value = value
                .strip_prefix('\'')
                .and_then(|v| v.strip_suffix('\''))
                .or_else(|| value.strip_prefix('"').and_then(|v| v.strip_suffix('"')))
                .unwrap_or(value);
            return Some(value);
        }
    }
    None
}

/// The sole F5 engine-discipline cycle identifier for which 4-index-citation
/// and trajectory-tail checks apply (D-439(b) + D-451(c) F5-scope).
const F5_CYCLE_ID: &str = "v1.0-feature-engine-discipline-pass-1";

/// Returns `true` when `current_cycle` is the F5 engine-discipline cycle —
/// the only cycle for which D-439(b) 4-index-citation + D-451(c)
/// trajectory-tail checks are required.
///
/// All other cycles (e.g. `v1.0-brownfield-backfill`) use bare dispatch-step
/// labels for `current_step:` and MUST NOT be subjected to these checks.
fn is_f5_cycle(current_cycle: Option<&str>) -> bool {
    // Conservative: if cycle is unknown (None), apply F5 checks to avoid
    // silently disabling them on a genuinely-F5 state file that is missing
    // the current_cycle: field.
    current_cycle.is_none_or(|c| c == F5_CYCLE_ID)
}

/// Orchestrate all STATE.md validation checks.
///
/// Extracts `current_step:` value, runs all 4 checks, accumulates non-None
/// results into a `Vec<Violation>`. Returns an empty Vec for a clean write.
///
/// If `current_step:` cannot be extracted (frontmatter absent, truncated, or
/// malformed), returns an empty Vec — fail-open per BC-5.39.006 invariant 7/9.
/// The caller (`on_post_tool_use`) logs a warning via `host::log_warn` so the
/// skip is observable in dispatcher telemetry. This is consistent with the
/// read-error path which also produces Continue + log_warn.
///
/// # Cycle-identity gate (D-439(b)/D-451(c) F5-scope)
///
/// `check_index_version_cites` and `check_trajectory_tail_length` are gated
/// on the F5 engine-discipline cycle (`v1.0-feature-engine-discipline-pass-1`).
/// Brownfield and other cycles use bare dispatch-step labels for `current_step:`
/// (e.g. `D-689-S18.14-3CLEAN-CONVERGED-PROMOTION-2026-06-22`) which do not
/// contain 4-index cites or trajectory-tail markers — running these checks
/// unconditionally produces false-positive blocks on every correct brownfield
/// STATE.md write. The gate reads `current_cycle:` from frontmatter; if absent,
/// the conservative path (apply F5 checks) is taken to prevent silently
/// disabling checks on a genuine F5 state file.
///
/// `check_forbidden_meta_commentary` and `check_d_chain_currency` run
/// unconditionally for all cycles.
///
/// # BC trace
/// BC-5.39.006 v1.1 postcondition 1/6/7; invariant 7/9; F-P1-004;
/// D-439(b)/D-451(c) F5-scope gate.
pub fn validate_state_md(content: &str) -> Vec<Violation> {
    let current_step = match extract_current_step(content) {
        Some(v) => v,
        None => {
            // current_step: absent or frontmatter malformed — fail-open per invariant 9.
            // Caller (on_post_tool_use) emits log_warn for observability.
            return vec![];
        }
    };

    // Determine whether F5-only checks apply (D-439(b)/D-451(c) F5-scope gate).
    let current_cycle = extract_current_cycle(content);
    let apply_f5_checks = is_f5_cycle(current_cycle);

    let mut violations: Vec<Violation> = Vec::new();

    // check_forbidden_meta_commentary runs unconditionally for all cycles.
    violations.extend(check_forbidden_meta_commentary(current_step));

    // 4-index-citation and trajectory-tail checks are F5-cycle-specific.
    // D-439(b) + D-451(c): required ONLY when current_cycle is the F5
    // engine-discipline cycle. Brownfield current_step labels omit these
    // markers by design.
    if apply_f5_checks {
        if let Some(v) = check_index_version_cites(current_step) {
            violations.push(v);
        }
        if let Some(v) = check_trajectory_tail_length(current_step) {
            violations.push(v);
        }
    }

    // D-chain currency check runs unconditionally for all cycles.
    if let Some(v) = check_d_chain_currency(content, current_step) {
        violations.push(v);
    }

    violations
}

// ---------------------------------------------------------------------------
// INDEX.md validation
// ---------------------------------------------------------------------------

/// Validate adversary-pass table rows in INDEX.md content.
///
/// BC-5.39.006 v1.1 invariant 8 — h2-section state machine:
///
/// 1. Scan lines for h2 headings (`^## `). Enter `InAdversaryReviews` state
///    when the exact heading `## Adversarial Reviews` is seen. Exit back to
///    `Outside` when ANY next h2 heading appears.
/// 2. Inside `InAdversaryReviews`, find the first non-separator `|`-delimited
///    row — this is the header row. Count its pipes:
///    - pipe_count == 5 (4-column schema): GRANDFATHERED. Skip all remaining
///      rows in this section — no column validation.
///    - pipe_count == 6 (5-column schema): ENFORCE. All subsequent non-separator
///      `|`-rows in this section must have exactly 6 pipes (5 columns).
///    - any other count: treat as 5-col schema enforcement (default to strict).
/// 3. Separator rows (trimmed contains `|---` or `| ---` or `|:---`) are
///    ALWAYS skipped, including when determining header schema.
/// 4. Rows OUTSIDE `## Adversarial Reviews` are NEVER validated.
///
/// Returns all accumulated violations (may be empty for a clean write).
///
/// # Pipe arithmetic (BC-5.39.006 v1.1 invariant 8 corrected)
///
/// For N columns: 1 leading + (N-1) internal + 1 trailing = N+1 pipes.
/// - 5 columns: 6 pipes. 4 columns: 5 pipes. 6 columns: 7 pipes.
/// - The canonical 5-column adversary-pass schema = 6 pipes per D-442(b).
///
/// # BC trace
/// BC-5.39.006 v1.1 postcondition 8/9; D-441(b)/D-442(b); EC-013..EC-016; invariant 8.
pub fn validate_index_md(content: &str) -> Vec<Violation> {
    /// State machine for h2-section tracking.
    #[derive(PartialEq)]
    enum State {
        /// Not inside `## Adversarial Reviews` section.
        Outside,
        /// Inside `## Adversarial Reviews`; header schema not yet detected.
        InSectionNoHeader,
        /// Inside section; header is 4-column (grandfathered; skip validation).
        InSectionGrandfathered,
        /// Inside section; header is 5-column; enforce 6-pipe compliance.
        InSectionEnforce,
    }

    let mut violations: Vec<Violation> = Vec::new();
    let mut state = State::Outside;

    for line in content.lines() {
        let trimmed = line.trim();

        // Detect h2 headings — update section state.
        if trimmed.starts_with("## ") {
            if trimmed == "## Adversarial Reviews" {
                state = State::InSectionNoHeader;
            } else {
                // Any other h2 ends the current section.
                state = State::Outside;
            }
            continue;
        }

        // Only process rows while inside the ## Adversarial Reviews section.
        if state == State::Outside {
            continue;
        }

        // Must be a table row: trimmed starts and ends with `|`.
        if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
            continue;
        }

        // Separator rows are always skipped.
        if trimmed.contains("|---") || trimmed.contains("| ---") || trimmed.contains("|:---") {
            continue;
        }

        let pipe_count = trimmed.chars().filter(|&c| c == '|').count();

        match state {
            State::InSectionNoHeader => {
                // First non-separator row: determine schema from header pipe count.
                if pipe_count == 5 {
                    // 4-column header (5 pipes) — grandfathered; skip validation.
                    state = State::InSectionGrandfathered;
                } else {
                    // 5-column (6 pipes) or other — enforce 6-pipe compliance.
                    state = State::InSectionEnforce;
                    // Validate this header row too if not 6 pipes.
                    // (Header should be 6 pipes for 5-col schema; if it's different
                    // that is unusual but let enforcement below handle it.)
                    if pipe_count != 6 {
                        let actual_cols = pipe_count.saturating_sub(1);
                        let row_preview = safe_truncate(trimmed, 120);
                        violations.push(Violation {
                            description: format!(
                                "INDEX.md ## Adversarial Reviews header row has \
                                 {actual_cols} column(s); required 5 columns per \
                                 D-441(b)/D-442(b); row: `{row_preview}`"
                            ),
                            cited_raw: row_preview,
                        });
                    }
                }
            }
            State::InSectionEnforce => {
                // Enforce 6-pipe (5-column) compliance on all data rows.
                if pipe_count != 6 {
                    let actual_cols = pipe_count.saturating_sub(1);
                    let row_preview = safe_truncate(trimmed, 120);
                    violations.push(Violation {
                        description: format!(
                            "INDEX.md adversary-pass row has {actual_cols} column(s); \
                             required 5 columns per D-441(b)/D-442(b); \
                             row: `{row_preview}`"
                        ),
                        cited_raw: row_preview,
                    });
                }
            }
            State::InSectionGrandfathered => {
                // Grandfathered 4-col section — skip column validation entirely.
            }
            State::Outside => {
                // Unreachable here (handled above), but exhaustive match required.
            }
        }
    }

    violations
}

/// Truncate `s` to at most `max_chars` characters, preserving UTF-8 char boundaries.
///
/// Uses `char_indices()` to find the correct byte boundary (BC-5.39.006 invariant 10).
fn safe_truncate(s: &str, max_chars: usize) -> String {
    let mut byte_end = s.len();
    for (char_count, (byte_pos, _ch)) in s.char_indices().enumerate() {
        if char_count == max_chars {
            byte_end = byte_pos;
            break;
        }
    }
    s[..byte_end].to_string()
}

// ---------------------------------------------------------------------------
// PreCompact flush exemption (BC-5.41.003 + S-18.04b)
// ---------------------------------------------------------------------------

/// The exact commit subject prefix produced by the `precompact-flush` PreCompact WASM plugin (`precompact-flush.wasm`, S-18.04a).
///
/// Matches the value of `COMMIT_PREFIX` in `crates/hook-plugins/precompact-flush/src/lib.rs`.
/// Case-sensitive; no substring match; no regex. BC-5.41.003 invariant 3.
pub const PRECOMPACT_FLUSH_PREFIX: &str = "PreCompact flush ";

/// Determine whether a commit is exempt from the MULTI_COMMIT_CHAIN_NOT_ALLOWED
/// detector under the PreCompact flush exemption (BC-5.41.003 PC1 three-case logic).
///
/// Three-case logic (BC-5.41.003 PC1):
///
/// - Case (a): log exists, last line FIELD-4 == `commit`, SHA matches FIELD-2 → exempt.
/// - Case (b): log exists but last line FIELD-4 absent/empty/non-`commit` → treat
///   as stale/corrupted; fall through to case (c).
/// - Case (c): log absent or last line empty → prefix-match alone is sufficient.
///
/// SHA-mismatch with valid FIELD-4=`commit` is NOT exempt (BC-5.41.003 INV1).
///
/// # Parameters
/// - `commit_subject`: the raw commit subject (first line of `git log --format=%s`).
/// - `commit_sha`: the SHA of the commit being evaluated.
/// - `flush_log_content`: `Some(last_line_of_log)` when the log file exists and has
///   at least one line; `None` when the log is absent or empty.
///
/// # Returns
/// `true` if the commit is exempt from chain detection; `false` otherwise.
///
/// # BC trace
/// BC-5.41.003 PC1 cases (a)/(b)/(c); INV1; AC-001/AC-002/AC-003/AC-004; AC-005; AC-008.
///
/// # Self-check (BC-5.38.005 invariant 1)
/// "If I include this real implementation, will the test for this function pass trivially
/// without any implementer work?" — YES: the 3-case logic is non-trivial (branching,
/// field parsing, SHA comparison). Body is `todo!()` per BC-5.38.001.
pub fn is_precompact_flush_exempt(
    commit_subject: &str,
    commit_sha: &str,
    flush_log_content: Option<&str>,
) -> bool {
    // Step 0: prefix match (case-sensitive, exact). BC-5.41.003 INV3.
    if !commit_subject.starts_with(PRECOMPACT_FLUSH_PREFIX) {
        return false;
    }

    // Step 1: examine log content.
    match flush_log_content {
        None => {
            // Case (c): log absent → prefix-match alone is sufficient. AC-002.
            true
        }
        Some(last_line) => {
            // Parse the 4-field log line: <ISO-timestamp> <SHA> <cycle>/<step> <type>
            // Fields are space-separated; we need FIELD-2 (SHA) and FIELD-4 (type token).
            let mut fields = last_line.split_whitespace();
            let _field1 = fields.next(); // ISO-timestamp
            let field2_sha = fields.next(); // SHA
            let _field3 = fields.next(); // cycle/step
            let field4_type = fields.next(); // type token ("commit" or absent/corrupt)

            match field4_type {
                Some("commit") => {
                    // Case (a): log valid, FIELD-4=commit. SHA must match. AC-001/AC-004.
                    match field2_sha {
                        Some(log_sha) => log_sha == commit_sha,
                        None => {
                            // FIELD-4=commit but FIELD-2 absent — treat as corrupted.
                            // Case (b) → case (c): prefix-match-only. AC-003.
                            true
                        }
                    }
                }
                _ => {
                    // Case (b): FIELD-4 absent, empty, or non-"commit" → treat as stale.
                    // Fall through to case (c): prefix-match-only exemption. AC-003.
                    true
                }
            }
        }
    }
}

/// Check whether the HEAD and HEAD^ commit subjects form a `MULTI_COMMIT_CHAIN_NOT_ALLOWED`
/// pattern (TD-VSDD-053), with the PreCompact flush exemption applied (BC-5.41.003).
///
/// A chain violation is triggered when BOTH of the following hold:
/// 1. `head_subject` contains a sentinel word (`backfill`, `Stage 1`, `Stage 2`).
/// 2. `head_parent_subject` contains a sentinel word.
///
/// The exemption: if either commit is exempt under `is_precompact_flush_exempt`, the
/// chain comparison is skipped (no violation). The exemption is checked symmetrically for
/// both HEAD and HEAD^ (BC-5.41.003 INV1 — both hooks must implement identically).
///
/// # Parameters
/// - `head_subject`: commit subject of HEAD.
/// - `head_sha`: SHA of HEAD.
/// - `head_parent_subject`: commit subject of HEAD^.
/// - `head_parent_sha`: SHA of HEAD^.
/// - `flush_log_content`: last line of `.factory/hooks/precompact-flush-log`, or `None`.
///
/// # Returns
/// `Some(Violation)` with `MULTI_COMMIT_CHAIN_NOT_ALLOWED` message if a violation is
/// detected; `None` if no violation (or exemption applies).
///
/// # BC trace
/// BC-5.41.003; BC-5.39.006 INV4; TD-VSDD-053.
///
/// # Self-check (BC-5.38.005 invariant 1)
/// "If I include this real implementation, will the test for this function pass trivially
/// without any implementer work?" — YES: non-trivial branching + sentinel scanning +
/// exemption delegation. Body is `todo!()` per BC-5.38.001.
pub fn check_multi_commit_chain(
    head_subject: &str,
    head_sha: &str,
    head_parent_subject: &str,
    head_parent_sha: &str,
    flush_log_content: Option<&str>,
) -> Option<Violation> {
    // If either commit is exempt under the PreCompact flush exemption,
    // skip the chain comparison entirely. BC-5.41.003 PC1.
    if is_precompact_flush_exempt(head_subject, head_sha, flush_log_content) {
        return None;
    }
    if is_precompact_flush_exempt(head_parent_subject, head_parent_sha, flush_log_content) {
        return None;
    }

    // TD-VSDD-053 chain detector: MULTI_COMMIT_CHAIN_NOT_ALLOWED fires when both
    // HEAD and HEAD^ contain a sentinel word.
    let head_has_sentinel = contains_sentinel(head_subject);
    let parent_has_sentinel = contains_sentinel(head_parent_subject);

    if head_has_sentinel && parent_has_sentinel {
        Some(Violation {
            description: format!(
                "MULTI_COMMIT_CHAIN_NOT_ALLOWED — HEAD and HEAD^ both contain chain-sentinel words; \
                 HEAD: {:?}; HEAD^: {:?}; TD-VSDD-053",
                head_subject, head_parent_subject
            ),
            cited_raw: format!("HEAD={head_subject:?}; HEAD^={head_parent_subject:?}"),
        })
    } else {
        None
    }
}

/// Returns `true` if `subject` contains any TD-VSDD-053 sentinel word.
///
/// Sentinels: "backfill", "Stage 1", "Stage 2" (case-insensitive for "backfill";
/// case-sensitive for "Stage N" per existing convention).
///
/// # BC trace
/// TD-VSDD-053 (single-commit-per-burst chain detector).
fn contains_sentinel(subject: &str) -> bool {
    let lower = subject.to_lowercase();
    lower.contains("backfill") || subject.contains("Stage 1") || subject.contains("Stage 2")
}

// ---------------------------------------------------------------------------
// Block message formatting
// ---------------------------------------------------------------------------

/// Format a list of violations into a `HookResult::block_with_fix`.
///
/// Each violation is formatted as:
/// ```text
///   - {description}
///       cited: "{cited_raw}"
/// ```
/// The `cited_raw` field is included when non-empty, making the block message
/// actionable by quoting the exact offending text the author wrote.
///
/// # TD-VSDD-059 compliance
/// `cited_raw` is wired through `emit_block` rather than being a dead field.
fn emit_block(hook_name: &str, violations: &[Violation]) -> HookResult {
    let lines: Vec<String> = violations
        .iter()
        .map(|v| {
            if v.cited_raw.is_empty() {
                format!("  - {}", v.description)
            } else {
                format!("  - {}\n      cited: \"{}\"", v.description, v.cited_raw)
            }
        })
        .collect();
    let reason = format!(
        "{hook_name}: {} violation(s):\n{}",
        violations.len(),
        lines.join("\n")
    );
    HookResult::block_with_fix(
        hook_name,
        reason,
        "Fix the violations listed above before re-writing this file",
        "DISPATCH_ADVANCE_VIOLATION",
    )
}

// ---------------------------------------------------------------------------
// Hook entry point
// ---------------------------------------------------------------------------

/// PostToolUse hook entry point.
///
/// Called by the SDK trampoline (`__internal::run`) for PostToolUse events.
///
/// **Dispatch path A — PostToolUse Bash (ADR-029 §Decision 1)**:
/// When `tool_name == "Bash"`, the chain-detection gate fires via `git_context`
/// in `payload.extra` (injected by the dispatcher host layer). This is exec-free:
/// no `host::exec_subprocess` is called for commit-context acquisition (ADR-029
/// §Decision 3; BC-5.41.003 PC1 addendum). Fail-open on absent/empty git_context.
///
/// **Dispatch path B — PostToolUse Edit|Write (STATE.md + INDEX.md validation)**:
/// Dispatches to the STATE.md arm or INDEX.md arm based on path-component-strict
/// guards. Runs `validate_state_md` and `validate_index_md` respectively.
///
/// # Control flow
///
/// Bash path: detect tool_name=="Bash" → read git_context from payload.extra →
/// run chain detection → return Continue or Block.
///
/// Edit|Write path:
/// 1. Extract `file_path` from `payload.tool_input`. If absent: Continue
///    (graceful degrade; log_warn).
/// 2. If `is_state_md_target(file_path)`: read via `host::read_file`. On error:
///    Continue + log_warn (fail-open per invariant 9). On success: run
///    `validate_state_md`. If violations non-empty: emit single BlockWithFix
///    enumerating all. Else: Continue.
/// 3. Else if `is_index_md_target(file_path)`: read via `host::read_file`. On
///    error: Continue + log_warn. On success: run `validate_index_md`. If
///    violations non-empty: emit single BlockWithFix enumerating all. Else:
///    Continue.
/// 4. Else: Continue (file not in scope of this hook).
///
/// # BC trace
/// BC-5.39.006 postconditions 1–10; invariants 1–10.
/// BC-5.41.003 PC1 addendum (ADR-029 wiring); BC-1.16.001 PC1+INV3.
/// ADR-029 §Decision 1+3+5.
pub fn on_post_tool_use(payload: HookPayload) -> HookResult {
    use vsdd_hook_sdk::host;

    const HOOK_NAME: &str = "validate-dispatch-advance";

    // ADR-029 §Decision 1: chain detection fires on PostToolUse Bash git-commit events.
    // STATE.md/INDEX.md validation fires on PostToolUse Edit|Write events.
    if payload.tool_name == "Bash" {
        // Dispatch path A: Bash git-commit → chain detection via git_context.
        return check_chain_from_git_context(&payload, HOOK_NAME);
    }

    // Dispatch path B: Edit|Write → STATE.md / INDEX.md validation.

    // Step 1: Extract file_path from tool_input.
    let file_path = match payload.tool_input.get("file_path").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => {
            host::log_warn(
                "[validate-dispatch-advance] file_path absent from tool_input — graceful degrade",
            );
            return HookResult::Continue;
        }
    };

    // Collect violations from all active gates.
    let mut all_violations: Vec<Violation> = Vec::new();

    if is_state_md_target(&file_path) {
        // STATE.md arm.
        let content = match host::read_file(&file_path, MAX_BYTES, 2000) {
            Ok(bytes) => {
                if bytes.len() as u32 == MAX_BYTES {
                    host::log_warn(&format!(
                        "[{HOOK_NAME}] read_file returned exactly {MAX_BYTES} bytes \
                         for {file_path} — possible truncation; consider raising MAX_BYTES"
                    ));
                }
                match String::from_utf8(bytes) {
                    Ok(s) => s,
                    Err(e) => {
                        host::log_warn(&format!(
                            "[{HOOK_NAME}] UTF-8 decode failure reading {file_path}: {e}"
                        ));
                        return HookResult::Continue;
                    }
                }
            }
            Err(e) => {
                host::log_warn(&format!(
                    "[{HOOK_NAME}] read_file failed for {file_path}: {e:?}"
                ));
                return HookResult::Continue;
            }
        };

        // Fail-open: if current_step: cannot be extracted, skip validation and warn.
        // Consistent with read-error path (invariant 9); aligns with F-P1-004 fix.
        if extract_current_step(&content).is_none() {
            host::log_warn(&format!(
                "[{HOOK_NAME}] current_step: absent or frontmatter malformed in \
                 {file_path} — skipping STATE.md validation (fail-open per invariant 9)"
            ));
            return HookResult::Continue;
        }

        all_violations.extend(validate_state_md(&content));
    } else if is_index_md_target(&file_path) {
        // INDEX.md arm.
        let content = match host::read_file(&file_path, MAX_BYTES, 2000) {
            Ok(bytes) => {
                if bytes.len() as u32 == MAX_BYTES {
                    host::log_warn(&format!(
                        "[{HOOK_NAME}] read_file returned exactly {MAX_BYTES} bytes \
                         for {file_path} — possible truncation; consider raising MAX_BYTES"
                    ));
                }
                match String::from_utf8(bytes) {
                    Ok(s) => s,
                    Err(e) => {
                        host::log_warn(&format!(
                            "[{HOOK_NAME}] UTF-8 decode failure reading {file_path}: {e}"
                        ));
                        return HookResult::Continue;
                    }
                }
            }
            Err(e) => {
                host::log_warn(&format!(
                    "[{HOOK_NAME}] read_file failed for {file_path}: {e:?}"
                ));
                return HookResult::Continue;
            }
        };

        all_violations.extend(validate_index_md(&content));
    }
    // else: not a STATE.md or INDEX.md target — continue without file-content validation.

    if all_violations.is_empty() {
        HookResult::Continue
    } else {
        emit_block(HOOK_NAME, &all_violations)
    }
}

/// Dispatch path A — PostToolUse Bash (ADR-029 §Decision 1).
///
/// Reads `git_context` from `payload.extra` (injected by the dispatcher host layer)
/// and runs the MULTI_COMMIT_CHAIN_NOT_ALLOWED detector. Exec-free: no
/// `host::exec_subprocess` is called for commit-context acquisition.
///
/// Fail-open semantics (BC-1.16.001 INV3; BC-5.41.003 Invariant 5):
/// - `git_context` absent from `payload.extra` → Continue.
/// - All four fields empty → Continue.
/// - `head_parent_subject` empty → Continue (initial commit; no chain possible).
///
/// # BC trace
/// BC-5.41.003 PC1 addendum; BC-1.16.001 PC1+INV3; ADR-029 §Decision 1+3+5.
fn check_chain_from_git_context(payload: &HookPayload, hook_name: &str) -> HookResult {
    use vsdd_hook_sdk::host;

    // CR-002: Short-circuit for non-git-commit Bash events per ADR-029 §Decision 1.
    // The dispatcher only injects git_context on qualifying git-commit events, but adding
    // an explicit WASM-level filter avoids unnecessary processing of all other Bash events.
    let command = payload
        .tool_input
        .get("command")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !command.contains("git") || !command.contains("commit") {
        return HookResult::Continue;
    }

    // Step 1: Extract git_context from payload.extra. Fail-open if absent.
    let git_context = match payload.extra.get("git_context") {
        Some(v) => v,
        None => return HookResult::Continue,
    };

    // Step 2: Extract the 4 required fields.
    let head_subject = git_context
        .get("head_subject")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let head_sha = git_context
        .get("head_sha")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let head_parent_subject = git_context
        .get("head_parent_subject")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let head_parent_sha = git_context
        .get("head_parent_sha")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();

    // Step 3: Fail-open if all fields empty (dispatcher fail-open path).
    // SEC-004: emit telemetry warn so silent bypasses are observable.
    if head_subject.is_empty() && head_parent_subject.is_empty() {
        host::log_warn(
            "[validate-dispatch-advance] git_context all-empty — chain check skipped (fail-open per BC-1.16.001 INV3)",
        );
        return HookResult::Continue;
    }

    // Step 4: Fail-open if head_parent_subject is empty (initial commit; no chain).
    if head_parent_subject.is_empty() {
        return HookResult::Continue;
    }

    // Step 5: Read precompact-flush-log for SHA corroboration (host::read_file; NOT exec).
    let flush_log_last_line: Option<String> = {
        let cwd = host::cwd();
        if !cwd.is_empty() {
            let flush_log_path = format!("{cwd}/.factory/hooks/precompact-flush-log");
            match host::read_file(&flush_log_path, 4096, 500) {
                Ok(bytes) => String::from_utf8(bytes).ok().and_then(|s| {
                    s.lines()
                        .rev()
                        .find(|l| !l.trim().is_empty())
                        .map(|l| l.to_string())
                }),
                Err(_) => None,
            }
        } else {
            None
        }
    };

    // Step 6: Run check_multi_commit_chain (pure logic; no I/O).
    match check_multi_commit_chain(
        &head_subject,
        &head_sha,
        &head_parent_subject,
        &head_parent_sha,
        flush_log_last_line.as_deref(),
    ) {
        Some(violation) => emit_block(hook_name, &[violation]),
        None => HookResult::Continue,
    }
}

// ---------------------------------------------------------------------------
// Unit tests (all red until implementer fills in real logic)
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    // -- Path-guard tests (GREEN-BY-DESIGN: pure Path::file_name delegation) --

    #[test]
    fn test_is_state_md_target_positive() {
        assert!(is_state_md_target(".factory/STATE.md"));
    }

    #[test]
    fn test_xstate_md_is_not_target() {
        assert!(!is_state_md_target("/some/dir/xSTATE.md"));
    }

    #[test]
    fn test_is_index_md_target_positive() {
        assert!(is_index_md_target(
            ".factory/cycles/v1.0-feature-engine-discipline-pass-1/INDEX.md"
        ));
    }

    #[test]
    fn test_xindex_md_is_not_target() {
        assert!(!is_index_md_target("/some/dir/xINDEX.md"));
    }

    // -- scan_max_d_nnn slice-guard safety (F-P1-006; TD-VSDD-060 sibling-site sweep) --

    #[test]
    fn test_scan_max_d_nnn_d_at_end_of_string() {
        // "D-" at the very end of string with no digits — must not panic.
        // Tests the `advance >= search.len()` break guard in scan_max_d_nnn.
        let result = std::panic::catch_unwind(|| {
            // Call check_d_chain_currency which calls scan_max_d_nnn internally.
            let current_step =
                "BC-INDEX v1.14 VP-INDEX v1.8 STORY-INDEX v1.12 ARCH-INDEX v1.9 D-477";
            let content = "---\ncurrent_step: 'x'\n---\nsuffix D-";
            check_d_chain_currency(content, current_step)
        });
        assert!(
            result.is_ok(),
            "scan_max_d_nnn must not panic on 'D-' at end of string"
        );
    }

    #[test]
    fn test_scan_max_d_nnn_multiple_references() {
        // Multiple D-NNN in current_step; max is correctly extracted.
        let current_step = "BC-INDEX v1.14 VP-INDEX v1.8 STORY-INDEX v1.12 ARCH-INDEX v1.9 \
             D-382 D-440 D-477 →9→9→9→9";
        // Side A: scan_max_d_nnn (this test's namesake) must extract 477 as
        // the max of the three embedded references D-382, D-440, D-477.
        assert_eq!(
            scan_max_d_nnn(current_step),
            477,
            "scan_max_d_nnn must extract 477 as the max of D-382, D-440, D-477"
        );
        // Side B integration check: wrap the D-477 row in a real '## Decisions
        // Log' section so max_in_file is genuinely computed as 477 from the
        // structural scan (not the section-absent fail-open 0) — the
        // "should not violate" assertion below then exercises the real
        // max_cited >= max_in_file equality per BC-5.39.006 v1.8 invariant 7
        // Side B, rather than passing vacuously via fail-open.
        let content = "---\ncurrent_step: 'x'\n---\n\
             ## Decisions Log\n\
             \n\
             | ID | Decision | Summary | Phase | Date |\n\
             |----|----------|---------|-------|------|\n\
             | D-477 | some-decision | summary text | phase | 2026-09-05 |\n";
        assert_eq!(
            scan_max_decision_log_id(content),
            477,
            "structural scan of '## Decisions Log' must resolve max_in_file to 477"
        );
        let v = check_d_chain_currency(content, current_step);
        assert!(
            v.is_none(),
            "max_cited=477 >= max_in_file=477 — should not violate"
        );
    }

    // -- scan_max_d_nnn word-boundary false-positive fixes (RC25-RELEASED-2026 /
    // TD-VSDD-053 classes) --

    #[test]
    fn test_scan_max_d_nnn_ignores_d_yyyy_embedded_in_dates() {
        // "RC25-RELEASED-2026-09-04" contains the substring "D-2026" (the
        // trailing "D" of "RELEASED" immediately followed by "-2026") — this
        // is NOT a decision-log reference and must not be counted. A real
        // standalone reference `D-1162` elsewhere in the same string must
        // still be picked up as the max.
        let s = "banner RC25-RELEASED-2026-09-04 tail ... D-1162 ...";
        assert_eq!(
            scan_max_d_nnn(s),
            1162,
            "word-internal 'D-2026' (from RELEASED-2026) must not be counted; \
             the real standalone D-1162 must be the max"
        );
    }

    #[test]
    fn test_scan_max_d_nnn_ignores_word_internal_td_vsdd_053() {
        // "TD-VSDD-053" contains the substring "D-053" (the "D" of "TD" is
        // itself preceded by "T", an alphanumeric byte) — word-internal, so
        // it must not be counted. With no other D-NNN reference present, the
        // result must be 0.
        assert_eq!(
            scan_max_d_nnn("TD-VSDD-053"),
            0,
            "word-internal 'D-053' inside 'TD-VSDD-053' must not be counted"
        );
    }

    #[test]
    fn test_scan_max_d_nnn_boundary_preceded_refs_still_count() {
        // Legitimate standalone references — "D" preceded by whitespace or
        // punctuation (word boundary) — must still be extracted and maxed.
        assert_eq!(
            scan_max_d_nnn("cite D-1162 and (D-1200)"),
            1200,
            "boundary-preceded D-NNN refs must still be counted and maxed"
        );
    }

    #[test]
    fn test_check_d_chain_currency_still_flags_genuinely_stale_cite() {
        // Positive control: a genuinely stale cite (current_step citing
        // D-1000 while the body legitimately contains a higher D-1162 as a
        // first-column whole-cell row under `## Decisions Log`) must still
        // produce a violation. This confirms the word-boundary fix does not
        // disable the real staleness check, under the BC-5.39.006 v1.8
        // invariant 7 Side B structural-scan contract for `max_in_file`.
        let current_step = "D-chain cite D-1000 latest brownfield";
        let content = "---\ncurrent_step: 'x'\n---\n\
             ## Decisions Log\n\
             \n\
             | ID | Decision | Summary | Phase | Date |\n\
             |----|----------|---------|-------|------|\n\
             | D-1162 | some-decision | latest | phase | 2026-09-05 |\n";
        let v = check_d_chain_currency(content, current_step);
        assert!(
            v.is_some(),
            "max_cited=1000 < max_in_file=1162 (legitimate Decisions Log \
             row) — must still violate"
        );
    }

    // -- scan_max_decision_log_id / D-chain narrative-literal hardening
    // (BC-5.39.006 v1.8 invariant 7 Side B; EC-024; D-443(a); architect design
    // `.factory/cycles/v1.0-brownfield-backfill/
    //  scan-max-d-nnn-narrative-literal-hardening-analysis.md`) --

    /// Builds a synthetic STATE.md-shaped string: a `## Decisions Log`
    /// section (header + separator + a single data row whose ID column is
    /// `decisions_log_max_id`) followed by a `## Skip Log` section (models
    /// "next h2 heading ends the section") containing narrative prose that
    /// reproduces the exact task-verified word-boundary-passing `D-2026`
    /// tokens: 3 quoted-literal occurrences and 1 bare-prose occurrence, none
    /// of which occupy the whole first pipe-cell of a Decisions Log row.
    fn narrative_literal_fixture(decisions_log_max_id: u64) -> String {
        format!(
            "---\n\
             current_step: 'placeholder'\n\
             ---\n\
             ## Decisions Log\n\
             \n\
             | ID | Decision | Summary | Phase | Date |\n\
             |----|----------|---------|-------|------|\n\
             | D-{decisions_log_max_id} | some-decision | summary text | phase | 2026-09-05 |\n\
             \n\
             ## Skip Log\n\
             \n\
             1. word-boundary fix to `scan_max_d_nnn` (was misreading \"D-2026\" \
             inside \"RC25-RELEASED-2026-09-04\" as a stale decision cite).\n\
             2. word-boundary fix (was misreading \"D-2026\" inside \
             \"RC25-RELEASED-2026-09-04\" as a stale decision cite, falsely \
             flagging `current_step`'s cite).\n\
             3. it was matching the \"D\" inside \"RC25-RELEASED-2026-09-04\" as \
             decision D-2026 and falsely flagging `current_step`'s cite as stale.\n\
             4. the \"D-2026\" false-positive (this session hit it live) is fixed \
             on `develop`.\n"
        )
    }

    #[test]
    fn test_scan_max_decision_log_id_ignores_narrative_and_quoted_mentions() {
        // The `## Decisions Log` section's max ID-column row is D-1163; the
        // `## Skip Log` narrative outside that section contains 3 quoted
        // `"D-2026"` tokens + 1 bare-prose `decision D-2026` token — all
        // word-boundary-passing per scan_max_d_nnn's existing rule, but NONE
        // occupy the whole first pipe-cell of a `## Decisions Log` row.
        let content = narrative_literal_fixture(1163);
        assert_eq!(
            scan_max_decision_log_id(&content),
            1163,
            "structural scan must return the Decisions Log ID-column max \
             (1163), not the narrative-literal D-2026 mentions outside the \
             section"
        );
    }

    #[test]
    fn test_scan_max_decision_log_id_fail_open_on_missing_section() {
        // No `## Decisions Log` heading anywhere in content — fail-open per
        // invariant 7's existing fail-open design philosophy.
        let content = "---\ncurrent_step: 'x'\n---\n\
             # No Decisions Log section here\n\
             some narrative mentioning D-9999 and \"D-2026\" in prose.\n";
        assert_eq!(
            scan_max_decision_log_id(content),
            0,
            "missing '## Decisions Log' section must fail-open to 0"
        );
    }

    #[test]
    fn test_check_d_chain_currency_no_false_positive_on_narrative_literal() {
        // Full reproduction of the live-bug shape (D-2026 self-referential
        // false positive, PR #813/5e009dc0-adjacent, narrative-literal class):
        // current_step: cites D-1163; the '## Decisions Log' section's max
        // ID-column row is D-1163; narrative elsewhere in the body contains
        // the 4 word-boundary-passing D-2026 mentions (3 quoted + 1 prose).
        // Once scan_max_decision_log_id is implemented AND wired into
        // check_d_chain_currency's max_in_file computation, this must return
        // None (max_cited=1163 >= max_in_file=1163).
        let current_step = "D-chain cite D-1163 latest brownfield";
        let content = narrative_literal_fixture(1163);
        let v = check_d_chain_currency(&content, current_step);
        assert!(
            v.is_none(),
            "max_cited=1163 >= max_in_file=1163 (Side B structural scan must \
             ignore the narrative D-2026 mentions) — must NOT violate; got: \
             {v:?}"
        );
    }

    #[test]
    fn test_check_d_chain_currency_still_flags_stale_via_decisions_log() {
        // Positive control: current_step cites D-1163, but the '## Decisions
        // Log' section's max ID-column row is D-1164 (genuinely newer than
        // current_step's cite) — the same narrative D-2026 noise is present
        // as in the false-positive-reproduction test above, to confirm the
        // structural narrowing to the Decisions Log ID column does not
        // accidentally suppress a real staleness violation just because
        // narrative noise is also present in the body. Once implemented, the
        // violation must cite max_in_file=1164 (from the Decisions Log row),
        // NOT 2026 (which the old whole-body scan_max_d_nnn(content) would
        // have picked up from the narrative).
        let current_step = "D-chain cite D-1163 latest brownfield";
        let content = narrative_literal_fixture(1164);
        let v = check_d_chain_currency(&content, current_step);
        assert!(
            v.is_some(),
            "max_cited=1163 < max_in_file=1164 (genuine Decisions Log \
             staleness) — must still violate even with narrative D-2026 \
             noise present"
        );
        let violation = v.expect("checked is_some above");
        assert!(
            violation.description.contains("D-1164"),
            "violation must cite the genuine Decisions Log max (D-1164), not \
             the narrative-inflated D-2026; got: {}",
            violation.description
        );
        assert!(
            !violation.description.contains("D-2026"),
            "violation must NOT cite the narrative-literal D-2026 token as \
             max_in_file; got: {}",
            violation.description
        );
    }

    // -- v1.9 hardening: EC-025 (ID-cell emphasis normalization) + EC-026
    // (separator-row whole-row-composition detection) --
    // BC-5.39.006 v1.9 invariant 7 Side B; closes adversary pass-1 findings
    // LOW-2 (EC-025) and LOW-1 (EC-026) against
    // `.factory/cycles/v1.0-brownfield-backfill/
    //  scan-max-d-nnn-narrative-literal-hardening-analysis.md`.
    //
    // RED GATE: as of v1.8 production code, `scan_max_decision_log_id`
    // rejects any ID cell that is not a bare whole-cell `D-\d+` match (no
    // emphasis-stripping) and skips any row whose trimmed content merely
    // *contains* the substring `---` (not a whole-row-composition test).
    // Both defects are latent false-NEGATIVE (stale-cite-escape) vectors:
    // a genuinely-newer Decisions Log row can be silently dropped from the
    // `max_in_file` candidate set, letting a stale `current_step:` D-chain
    // cite pass undetected.

    #[test]
    fn test_scan_max_decision_log_id_normalizes_emphasis_decorated_id() {
        // Bold-decorated ID cell (`**D-1164**`) plus lower plain rows below
        // it. EC-025: pre-v1.9 whole-cell match against the raw, undecorated
        // cell rejects "**D-1164**" outright (it does not start with the
        // literal prefix "D-"), so the row's ID is silently excluded from
        // the candidate set and max_in_file falls back to the lower plain
        // rows (1160), NOT 1164.
        let content_bold = "---\ncurrent_step: 'x'\n---\n\
             ## Decisions Log\n\
             \n\
             | ID | Decision | Summary | Phase | Date |\n\
             |----|----------|---------|-------|------|\n\
             | **D-1164** | bold-decorated | latest | phase | 2026-09-05 |\n\
             | D-1160 | plain-lower | earlier | phase | 2026-09-04 |\n\
             | D-1000 | plain-lower-2 | earlier | phase | 2026-09-03 |\n";
        assert_eq!(
            scan_max_decision_log_id(content_bold),
            1164,
            "bold-decorated ID cell '**D-1164**' must normalize to 'D-1164' \
             and be counted toward max_in_file (EC-025); a lower plain-row \
             result (e.g. 1160) proves the decorated row was silently \
             dropped"
        );

        // Backtick-decorated ID cell variant.
        let content_backtick = "---\ncurrent_step: 'x'\n---\n\
             ## Decisions Log\n\
             \n\
             | ID | Decision | Summary | Phase | Date |\n\
             |----|----------|---------|-------|------|\n\
             | `D-1164` | backtick-decorated | latest | phase | 2026-09-05 |\n\
             | D-1160 | plain-lower | earlier | phase | 2026-09-04 |\n";
        assert_eq!(
            scan_max_decision_log_id(content_backtick),
            1164,
            "backtick-decorated ID cell '`D-1164`' must normalize to \
             'D-1164' and be counted toward max_in_file (EC-025)"
        );

        // Underscore-italic-decorated ID cell variant.
        let content_underscore = "---\ncurrent_step: 'x'\n---\n\
             ## Decisions Log\n\
             \n\
             | ID | Decision | Summary | Phase | Date |\n\
             |----|----------|---------|-------|------|\n\
             | _D-1164_ | underscore-decorated | latest | phase | 2026-09-05 |\n\
             | D-1160 | plain-lower | earlier | phase | 2026-09-04 |\n";
        assert_eq!(
            scan_max_decision_log_id(content_underscore),
            1164,
            "underscore-decorated ID cell '_D-1164_' must normalize to \
             'D-1164' and be counted toward max_in_file (EC-025)"
        );
    }

    #[test]
    fn test_scan_max_decision_log_id_retains_data_row_with_triple_dash() {
        // Genuine data row whose non-ID cell contains the substring "---"
        // (e.g. an em-dash rendering or a "Closes D-441---D-449 range"
        // style citation) must be RETAINED and counted — it is NOT a GFM
        // separator row. The header's own separator row
        // (`|----|----------|---------|-------|------|`) is a REAL
        // separator row that must still be skipped correctly alongside it.
        //
        // EC-026: pre-v1.9 code skips ANY row whose trimmed content
        // contains "---" — including this genuine data row — so
        // max_in_file falls back to the lower row (1100), NOT 1164.
        let content = "---\ncurrent_step: 'x'\n---\n\
             ## Decisions Log\n\
             \n\
             | ID | Decision | Summary | Phase | Date |\n\
             |----|----------|---------|-------|------|\n\
             | D-1164 | some-decision | summary with --- inside | phase | 2026-09-05 |\n\
             | D-1100 | earlier-decision | earlier summary | phase | 2026-09-01 |\n";
        assert_eq!(
            scan_max_decision_log_id(content),
            1164,
            "a genuine Decisions Log data row whose Summary cell contains \
             '---' must be retained and its ID counted (EC-026); the real \
             GFM header-separator row must still be skipped; a result of \
             1100 would prove the data row was wrongly dropped as a false \
             separator"
        );
    }

    #[test]
    fn test_check_d_chain_currency_flags_stale_when_latest_is_emphasis_decorated() {
        // current_step cites D-1163; the Decisions Log's genuinely-newer max
        // row is emphasis-decorated ("**D-1164**"). A stale-cite violation
        // must fire citing max_in_file=1164 — the decoration must not mask
        // staleness detection (EC-025 wired through check_d_chain_currency).
        let current_step = "D-chain cite D-1163 latest brownfield";
        let content = "---\ncurrent_step: 'x'\n---\n\
             ## Decisions Log\n\
             \n\
             | ID | Decision | Summary | Phase | Date |\n\
             |----|----------|---------|-------|------|\n\
             | **D-1164** | bold-decorated | latest | phase | 2026-09-05 |\n";
        let v = check_d_chain_currency(content, current_step);
        assert!(
            v.is_some(),
            "max_cited=1163 < max_in_file=1164 (emphasis-decorated row) — \
             must still violate; decoration must not mask a stale cite; \
             got: {v:?}"
        );
        let violation = v.expect("checked is_some above");
        assert!(
            violation.description.contains("D-1164"),
            "violation must cite the genuine (decoration-normalized) \
             Decisions Log max (D-1164); got: {}",
            violation.description
        );
    }

    #[test]
    fn test_check_d_chain_currency_flags_stale_when_latest_row_has_triple_dash_cell() {
        // current_step cites D-1163; the Decisions Log's genuinely-newer max
        // row (D-1164) has a non-ID cell containing "---". A stale-cite
        // violation must fire citing max_in_file=1164 — the "---" substring
        // in a non-ID cell must not cause the row to be misclassified as a
        // separator and dropped (EC-026 wired through check_d_chain_currency).
        let current_step = "D-chain cite D-1163 latest brownfield";
        let content = "---\ncurrent_step: 'x'\n---\n\
             ## Decisions Log\n\
             \n\
             | ID | Decision | Summary | Phase | Date |\n\
             |----|----------|---------|-------|------|\n\
             | D-1164 | x --- y | detail | phase | 2026-09-05 |\n";
        let v = check_d_chain_currency(content, current_step);
        assert!(
            v.is_some(),
            "max_cited=1163 < max_in_file=1164 (data row with '---' in a \
             non-ID cell) — must still violate; the row must not be \
             mistaken for a separator; got: {v:?}"
        );
        let violation = v.expect("checked is_some above");
        assert!(
            violation.description.contains("D-1164"),
            "violation must cite the genuine (correctly-retained) \
             Decisions Log max (D-1164); got: {}",
            violation.description
        );
    }

    #[test]
    fn test_scan_max_decision_log_id_still_skips_genuine_gfm_separator_row() {
        // Negative control: a genuine GFM separator row — every cell's
        // trimmed content composed SOLELY of '-', ':', and whitespace (the
        // canonical colon-alignment separator form `:---`, `:---:`, `---:`)
        // — must still be skipped and must contribute nothing to the
        // candidate set. This confirms the v1.9 whole-row-composition
        // tightening does not regress genuine separator-row detection; only
        // the false-classification of data rows (EC-026) is fixed.
        let content = "---\ncurrent_step: 'x'\n---\n\
             ## Decisions Log\n\
             \n\
             | ID | Decision | Summary | Phase | Date |\n\
             | :--- | :---: | ---: | --- | --- |\n\
             | D-1163 | some-decision | summary | phase | 2026-09-05 |\n";
        assert_eq!(
            scan_max_decision_log_id(content),
            1163,
            "a genuine colon-alignment GFM separator row must still be \
             skipped and must not affect max_in_file; only the real data \
             row's D-1163 should be counted"
        );
    }

    #[test]
    fn test_check_d_chain_currency_verbatim_state_md_d2026_fixture() {
        // Load-bearing fixture: the actual offending sentences copied
        // VERBATIM (exact contiguous substrings, not paraphrased) from the
        // live `.factory/STATE.md` as it stood on 2026-09-05 (per
        // scan-max-d-nnn-narrative-literal-hardening-analysis.md §1.2/§8.6),
        // reproducing the exact production defect this BC-5.39.006 v1.8
        // amendment closes, independent of the live production-file
        // integration test (which will stop exercising this exact text once
        // STATE.md is next compacted). Exactly 4 word-boundary-passing
        // `D-2026` tokens are reproduced below — 3 quoted-literal (from
        // STATE.md's Active Branches row, Concurrent Cycles narrative, and
        // Drift Items row) and 1 bare-prose (from STATE.md's own Decisions
        // Log row Summary-cell narrative, itself a non-first-column
        // occurrence) — matching the exact 3-quoted+1-bare split confirmed
        // by the architect's literal-shell reproduction in the analysis doc.
        let verbatim_active_branches_prose = "\
            maintenance/fix-orphan-wasm-bundle | MERGED+DELETED | PR #813 squash-merged \
            `5e009dc0` 2026-09-04 into develop (branch base `5824979d`); branch deleted. \
            Prerequisite CI-hardening for S-25.04: removed 2 orphaned tracked wasms \
            (`last-amended-migrate`, `verify-state-timestamp-refresh`) fixing \
            `bundle_orphan_check` T-009; word-boundary fix to `scan_max_d_nnn` in \
            `validate-dispatch-advance` (was misreading \"D-2026\" inside \
            \"RC25-RELEASED-2026-09-04\" as a stale decision cite). Maintenance PR — \
            does NOT increment merged_count.";
        let verbatim_concurrent_cycles_prose = "\
            (ii) word-boundary fix to `scan_max_d_nnn` in `validate-dispatch-advance` \
            (was misreading \"D-2026\" inside \"RC25-RELEASED-2026-09-04\" as a stale \
            decision cite, falsely flagging `current_step`'s D-1162 cite).";
        let verbatim_decisions_log_row_prose = "\
            (b) Prerequisite maintenance PR #813 (`maintenance/fix-orphan-wasm-bundle`) \
            SQUASH-MERGED as `5e009dc0` immediately prior: (i) removed 2 orphaned tracked \
            wasms fixing `bundle_orphan_check` T-009; (ii) word-boundary fix to \
            `scan_max_d_nnn` in `validate-dispatch-advance` — it was matching the \"D\" \
            inside \"RC25-RELEASED-2026-09-04\" as decision D-2026 and falsely flagging \
            `current_step`'s D-1162 cite as stale, failing \
            `validate_production_state_md_no_false_positive` across `cargo-host` + \
            `build-dispatcher` CI (3 new Red Gate tests added; also stops mis-reading \
            TD-VSDD-053 as D-053).";
        let verbatim_drift_item_prose = "\
            4. **Operator-level `scan_max_d_nnn` fix** — the \"D-2026\" false-positive \
            (this session hit it live, at Edit-time, on this very STATE.md pause burst — \
            PostToolUse advisory, writes landed) is fixed on `develop` (`5e009dc0`, \
            D-1163) but NOT yet at operator/rc.25 level; it takes effect only at the \
            next release (rc.26). Hook noise persists locally until then.";

        let content = format!(
            "---\n\
             current_step: 'D-chain cite D-1163 latest brownfield'\n\
             ---\n\
             ## Active Branches\n\
             \n\
             {verbatim_active_branches_prose}\n\
             \n\
             ## Concurrent Cycles\n\
             \n\
             {verbatim_concurrent_cycles_prose}\n\
             \n\
             ## Decisions Log\n\
             \n\
             | ID | Decision | Summary | Phase | Date |\n\
             |----|----------|---------|-------|------|\n\
             | D-1163 | D-1163-VERBATIM-FIXTURE | {verbatim_decisions_log_row_prose} | \
             brownfield | 2026-09-04 |\n\
             \n\
             ## Drift Items / Tech Debt\n\
             \n\
             {verbatim_drift_item_prose}\n"
        );

        assert_eq!(
            scan_max_decision_log_id(&content),
            1163,
            "verbatim STATE.md fixture: structural scan must return the \
             Decisions Log ID-column max (1163), ignoring the verbatim \
             quoted/bare 'D-2026' narrative-literal tokens copied from the \
             live production defect"
        );

        let current_step = "D-chain cite D-1163 latest brownfield";
        let v = check_d_chain_currency(&content, current_step);
        assert!(
            v.is_none(),
            "verbatim STATE.md fixture: max_cited=1163 >= max_in_file=1163 \
             — must NOT false-positive-block on the verbatim quoted/bare \
             'D-2026' narrative mentions; got: {v:?}"
        );
    }

    // -- extract_current_step fail-open (F-P1-004 / BC-5.39.006 v1.1 invariant 7/9) --

    #[test]
    fn test_validate_state_md_fail_open_on_missing_frontmatter() {
        // Content with no frontmatter — extract_current_step returns None.
        // validate_state_md must return vec![] (no violations) — fail-open per invariant 9.
        let content = "# No frontmatter\n\nsome body text D-477\n";
        let violations = validate_state_md(content);
        assert!(
            violations.is_empty(),
            "missing frontmatter must be fail-open (no violations); \
             got: {violations:?}"
        );
    }

    #[test]
    fn test_validate_state_md_fail_open_on_unclosed_frontmatter() {
        // Frontmatter opened but never closed — extract_current_step returns None
        // (no closing `---` found, and current_step: is absent from the partial block).
        let content = "---\nphase: test\n# body without closing ---\n";
        let violations = validate_state_md(content);
        assert!(
            violations.is_empty(),
            "truncated/malformed frontmatter must be fail-open (no violations); \
             got: {violations:?}"
        );
    }

    // -- Forbidden meta-commentary tests (F-P1-012: Vec<Violation> return) --

    #[test]
    fn test_forbidden_meta_commentary_watch() {
        let vs = check_forbidden_meta_commentary(
            "BC-INDEX v1.14, VP-INDEX v1.8, STORY-INDEX v1.12, ARCH-INDEX v1.9 \
             META-LEVEL-5 WATCH: self-application →9→9→9→9 D-382..D-477",
        );
        assert!(!vs.is_empty(), "should detect META-LEVEL-5 WATCH pattern");
        assert!(
            vs[0].description.contains("D-440(a)"),
            "block message must cite D-440(a)"
        );
    }

    #[test]
    fn test_forbidden_meta_commentary_self_app() {
        let vs = check_forbidden_meta_commentary(
            "BC-INDEX v1.14 VP-INDEX v1.8 STORY-INDEX v1.12 ARCH-INDEX v1.9 \
             self-app TEST example →9→9→9→9 D-382..D-477",
        );
        assert!(!vs.is_empty(), "should detect self-app TEST pattern");
    }

    #[test]
    fn test_forbidden_meta_commentary_expected_verdict() {
        let vs = check_forbidden_meta_commentary(
            "BC-INDEX v1.14 VP-INDEX v1.8 STORY-INDEX v1.12 ARCH-INDEX v1.9 \
             expected verdict: PASS →9→9→9→9 D-382..D-477",
        );
        assert!(!vs.is_empty(), "should detect expected verdict pattern");
    }

    #[test]
    fn test_forbidden_meta_commentary_multiple_patterns() {
        // BC-5.39.006 PC-6: multi-violation enumeration per check.
        // current_step with both META-LEVEL WATCH and self-app TEST should produce 2 violations.
        let vs = check_forbidden_meta_commentary(
            "BC-INDEX v1.14 META-LEVEL-5 WATCH self-app TEST →9→9→9→9 D-477",
        );
        assert!(
            vs.len() >= 2,
            "both META-LEVEL-5 WATCH and self-app TEST must produce separate violations; \
             got {} violations",
            vs.len()
        );
    }

    // -- Index version cite tests --

    #[test]
    fn test_index_version_cites_all_present() {
        let v = check_index_version_cites(
            "BC-INDEX v1.14, VP-INDEX v1.8, STORY-INDEX v1.12, ARCH-INDEX v1.9 \
             →9→9→9→9 D-382..D-477",
        );
        assert!(v.is_none(), "all 4 cites present — should not violate");
    }

    #[test]
    fn test_index_version_cites_missing_arch() {
        let v = check_index_version_cites(
            "BC-INDEX v1.14, VP-INDEX v1.8, STORY-INDEX v1.12 →9→9→9→9 D-382..D-477",
        );
        assert!(v.is_some(), "missing ARCH-INDEX v cite — should violate");
        let v = v.unwrap();
        assert!(
            v.description.contains("ARCH-INDEX v"),
            "block message must name missing ARCH-INDEX v"
        );
        assert!(
            v.description.contains("D-439(b)"),
            "block message must cite D-439(b)"
        );
    }

    // -- Trajectory-tail length tests (BC-5.39.006 v1.2 PC 6 + invariant 6) --
    //
    // All LENGTH tests must include the `trajectory-tail ` prefix; prefix absence
    // is now a separate hard violation (F-P3-006/EC-023). Tests that exercise the
    // LENGTH path must supply a well-formed prefix so the scoped count runs.

    #[test]
    fn test_trajectory_tail_length_3() {
        let v = check_trajectory_tail_length(
            "BC-INDEX v1.14, VP-INDEX v1.8, STORY-INDEX v1.12, ARCH-INDEX v1.9 \
             trajectory-tail →9→9→9 D-382..D-477",
        );
        assert!(v.is_some(), "tail has 3 components — should violate");
        let v = v.unwrap();
        assert!(
            v.description.contains('3'),
            "block message must name actual count 3"
        );
        assert!(
            v.description.contains("D-451(c)"),
            "block message must cite D-451(c)"
        );
    }

    #[test]
    fn test_trajectory_tail_length_5() {
        let v = check_trajectory_tail_length(
            "BC-INDEX v1.14, VP-INDEX v1.8, STORY-INDEX v1.12, ARCH-INDEX v1.9 \
             trajectory-tail →9→9→9→9→9 D-382..D-477",
        );
        assert!(v.is_some(), "tail has 5 components — should violate");
        let v = v.unwrap();
        assert!(
            v.description.contains('5'),
            "block message must name actual count 5"
        );
    }

    #[test]
    fn test_trajectory_tail_length_4() {
        let v = check_trajectory_tail_length(
            "BC-INDEX v1.14, VP-INDEX v1.8, STORY-INDEX v1.12, ARCH-INDEX v1.9 \
             trajectory-tail →9→9→9→9 D-382..D-477",
        );
        assert!(
            v.is_none(),
            "tail has exactly 4 components — should not violate"
        );
    }

    // -- Trajectory-tail prefix-absent tests (BC-5.39.006 v1.2 PC 6 / F-P3-006 / EC-023) --

    #[test]
    fn test_trajectory_tail_prefix_absent_is_hard_violation() {
        // BC v1.2 PC 6: absence of `trajectory-tail ` prefix is itself a violation.
        // LENGTH count must NOT run; the prefix-absent block fires instead.
        let v = check_trajectory_tail_length(
            "BC-INDEX v1.14, VP-INDEX v1.8, STORY-INDEX v1.12, ARCH-INDEX v1.9 \
             →9→9→9→9 D-382..D-477",
        );
        assert!(
            v.is_some(),
            "prefix absent — should violate (EC-023 / F-P3-006)"
        );
        let v = v.unwrap();
        assert!(
            v.description.contains("trajectory-tail "),
            "block message must name the missing canonical marker"
        );
        assert!(
            v.description.contains("F-P3-006"),
            "block message must cite F-P3-006"
        );
        assert!(
            v.description.contains("EC-023"),
            "block message must cite EC-023"
        );
    }

    #[test]
    fn test_trajectory_tail_prefix_absent_length_3_still_prefix_violation() {
        // Even if LENGTH would also be wrong (3 arrows), the prefix-absent
        // violation fires first — LENGTH count is skipped entirely.
        let v = check_trajectory_tail_length(
            "BC-INDEX v1.14, VP-INDEX v1.8, STORY-INDEX v1.12, ARCH-INDEX v1.9 \
             →9→9→9 D-382..D-477",
        );
        assert!(
            v.is_some(),
            "prefix absent — hard violation regardless of arrow count"
        );
        let v = v.unwrap();
        // The description should name the missing marker, not the arrow count.
        assert!(
            v.description.contains("trajectory-tail "),
            "description must call out missing canonical marker, not arrow count"
        );
    }

    #[test]
    fn test_trajectory_tail_prefix_present_length_3_fires_length_violation() {
        // Prefix present but LENGTH=3 → LENGTH violation (not prefix-absent).
        let v = check_trajectory_tail_length("trajectory-tail →9→9→9 D-382..D-477");
        assert!(
            v.is_some(),
            "prefix present, LENGTH=3 — LENGTH violation expected"
        );
        let v = v.unwrap();
        assert!(
            v.description.contains('3'),
            "block message must name actual count 3"
        );
        assert!(
            !v.description.contains("canonical marker"),
            "should be LENGTH violation, not prefix-absent violation"
        );
    }

    // -- D-chain currency tests (BC-5.39.006 v1.1 invariant 7: D-(\d+) max-extraction) --

    #[test]
    fn test_d_chain_absent() {
        // No D-NNN at all in current_step — should violate.
        let content = "---\ncurrent_step: 'BC-INDEX v1.14 →9→9→9→9'\n---\n| D-477 |";
        let v = check_d_chain_currency(
            content,
            "BC-INDEX v1.14, VP-INDEX v1.8, STORY-INDEX v1.12, ARCH-INDEX v1.9 →9→9→9→9",
        );
        assert!(v.is_some(), "absent D-chain cite — should violate");
        let v = v.unwrap();
        assert!(
            v.description.contains("D-443(a)"),
            "block message must cite D-443(a)"
        );
    }

    #[test]
    fn test_d_chain_stale_range_form() {
        // Prose range form D-382..D-476; body's `## Decisions Log` section
        // shows D-477 as a first-column whole-cell row → stale, per
        // BC-5.39.006 v1.8 invariant 7 Side B structural-scan contract.
        let content = "---\ncurrent_step: 'BC-INDEX v1.14 →9→9→9→9 D-382..D-476'\n---\n\
             ## Decisions Log\n\
             \n\
             | ID | Decision | Summary | Phase | Date |\n\
             |----|----------|---------|-------|------|\n\
             | D-477 | some-decision | summary text | phase | 2026-09-05 |\n";
        let current_step = "BC-INDEX v1.14, VP-INDEX v1.8, STORY-INDEX v1.12, ARCH-INDEX v1.9 \
             →9→9→9→9 D-382..D-476";
        let v = check_d_chain_currency(content, current_step);
        assert!(
            v.is_some(),
            "stale D-chain cite (max_cited=476 < max_in_file=477) — should violate"
        );
        let v = v.unwrap();
        assert!(
            v.description.contains("476"),
            "block message must name stale max_cited 476"
        );
    }

    #[test]
    fn test_d_chain_stale_prose_form() {
        // Production prose form "D-chain cite D-476"; body's `## Decisions
        // Log` section shows D-477 as a first-column whole-cell row →
        // stale, per BC-5.39.006 v1.8 invariant 7 Side B structural-scan
        // contract.
        let content = "---\ncurrent_step: 'BC-INDEX v1.14 →9→9→9→9 D-chain cite D-476'\n---\n\
             ## Decisions Log\n\
             \n\
             | ID | Decision | Summary | Phase | Date |\n\
             |----|----------|---------|-------|------|\n\
             | D-477 | some-decision | summary text | phase | 2026-09-05 |\n";
        let current_step = "BC-INDEX v1.14, VP-INDEX v1.8, STORY-INDEX v1.12, ARCH-INDEX v1.9 \
             →9→9→9→9 D-chain cite D-476";
        let v = check_d_chain_currency(content, current_step);
        assert!(
            v.is_some(),
            "stale prose D-chain cite (max_cited=476 < max_in_file=477) — should violate"
        );
        let v = v.unwrap();
        assert!(
            v.description.contains("476"),
            "block message must name stale max_cited 476"
        );
    }

    #[test]
    fn test_d_chain_current_range_form() {
        // current_step cites D-382..D-477 and body max is also D-477 — current,
        // per BC-5.39.006 v1.8 invariant 7 Side B structural-scan contract.
        // The D-477 row is wrapped in a real '## Decisions Log' section (mirrors
        // test_d_chain_stale_range_form) so max_in_file is genuinely computed
        // from the section, not the section-absent fail-open 0.
        let content = "---\ncurrent_step: 'BC-INDEX v1.14 →9→9→9→9 D-382..D-477'\n---\n\
             ## Decisions Log\n\
             \n\
             | ID | Decision | Summary | Phase | Date |\n\
             |----|----------|---------|-------|------|\n\
             | D-477 | some-decision | summary text | phase | 2026-09-05 |\n";
        assert_eq!(
            scan_max_decision_log_id(content),
            477,
            "structural scan of '## Decisions Log' must resolve max_in_file to 477"
        );
        let current_step = "BC-INDEX v1.14, VP-INDEX v1.8, STORY-INDEX v1.12, ARCH-INDEX v1.9 \
             →9→9→9→9 D-382..D-477";
        let v = check_d_chain_currency(content, current_step);
        assert!(v.is_none(), "D-chain cite is current — should not violate");
    }

    #[test]
    fn test_d_chain_current_prose_form() {
        // Production prose form "D-chain cite D-476"; body max is also 476 — current.
        // This matches the real production STATE.md current_step format. The
        // D-476 row is wrapped in a real '## Decisions Log' section (mirrors
        // test_d_chain_stale_prose_form) so max_in_file is genuinely computed
        // from the section, not the section-absent fail-open 0.
        let content = "---\ncurrent_step: 'BC-INDEX v1.14 →9→9→9→9 D-chain cite D-476'\n---\n\
             ## Decisions Log\n\
             \n\
             | ID | Decision | Summary | Phase | Date |\n\
             |----|----------|---------|-------|------|\n\
             | D-476 | some-decision | summary text | phase | 2026-09-05 |\n";
        assert_eq!(
            scan_max_decision_log_id(content),
            476,
            "structural scan of '## Decisions Log' must resolve max_in_file to 476"
        );
        let current_step = "BC-INDEX v1.14, VP-INDEX v1.8, STORY-INDEX v1.12, ARCH-INDEX v1.9 \
             →9→9→9→9 D-chain cite D-476 latest brownfield";
        let v = check_d_chain_currency(content, current_step);
        assert!(
            v.is_none(),
            "prose D-chain cite matches body max — should not violate"
        );
    }

    // -- INDEX.md column tests (BC-5.39.006 v1.1: h2-section state machine + header-schema) --

    // Helper: wrap rows in a canonical 5-col-header ## Adversarial Reviews section.
    fn in_5col_section(rows: &str) -> String {
        format!(
            "## Adversarial Reviews\n\n\
             | Pass | Date | Findings Count | Verdict | File |\n\
             |------|------|---------------|---------|------|\n\
             {rows}\n\
             \n## Next Section\n"
        )
    }

    // Helper: wrap rows in a grandfathered 4-col-header ## Adversarial Reviews section.
    fn in_4col_section(rows: &str) -> String {
        format!(
            "## Adversarial Reviews\n\n\
             | Pass | Date | Findings | Status |\n\
             |------|------|----------|--------|\n\
             {rows}\n\
             \n## Next Section\n"
        )
    }

    #[test]
    fn test_index_md_4_col_row_in_5col_section() {
        // EC-013: 4-col row (5 pipes) in 5-col-header section → violation.
        let content = in_5col_section("| col1 | col2 | col3 | col4 |");
        let violations = validate_index_md(&content);
        assert!(
            !violations.is_empty(),
            "4-column row in 5-col section should violate"
        );
        assert!(
            violations[0].description.contains("D-441(b)"),
            "violation must cite D-441(b)"
        );
        assert!(
            violations[0].description.contains('4'),
            "violation must name actual count 4"
        );
    }

    #[test]
    fn test_index_md_6_col_row_in_5col_section() {
        // EC-014: 6-col row (7 pipes) in 5-col-header section → violation.
        let content = in_5col_section("| col1 | col2 | col3 | col4 | col5 | col6 |");
        let violations = validate_index_md(&content);
        assert!(
            !violations.is_empty(),
            "6-column row in 5-col section should violate"
        );
        assert!(
            violations[0].description.contains('6'),
            "violation must name actual count 6"
        );
    }

    #[test]
    fn test_index_md_5_col_row_in_5col_section() {
        // EC-015: 5-col row (6 pipes) in 5-col-header section → pass.
        let content = in_5col_section("| pass1 | 2026-05-17 | 3 | MEDIUM | adv-1.md |");
        let violations = validate_index_md(&content);
        assert!(
            violations.is_empty(),
            "5-column row in 5-col section — should not violate"
        );
    }

    #[test]
    fn test_index_md_4col_section_grandfathered() {
        // 4-col-header section (5 pipes in header) → skip column validation.
        let content = in_4col_section("| 1 | 2026-04-25 | 17 (1C) | substantive |");
        let violations = validate_index_md(&content);
        assert!(
            violations.is_empty(),
            "4-col-header section is grandfathered — no violations expected"
        );
    }

    #[test]
    fn test_index_md_rows_outside_section_skipped() {
        // Rows outside ## Adversarial Reviews h2 must NOT be validated.
        // 3-col row outside section: should not trigger violation.
        let content = "## Story Listing\n\n| S-01 | desc | status |\n|------|------|--------|\n\
             | S-1.01 | foo | done |\n\n## Adversarial Reviews\n\n\
             | Pass | Date | Findings Count | Verdict | File |\n\
             |------|------|---------------|---------|------|\n\
             | 1 | 2026-05-17 | 3 | MEDIUM | adv-1.md |\n\n## More\n";
        let violations = validate_index_md(content);
        assert!(
            violations.is_empty(),
            "out-of-section rows must not be validated; 5-col row inside section passes"
        );
    }

    #[test]
    fn test_index_md_header_row_skipped() {
        // Separator row (|---|---|) within 5-col section must not be flagged.
        let content = in_5col_section(""); // only separator rows in the helpers
        let violations = validate_index_md(&content);
        assert!(
            violations.is_empty(),
            "separator rows must be skipped by validate_index_md"
        );
    }

    #[test]
    fn test_index_md_production_brownfield() {
        // Simulate brownfield INDEX.md — 4-col-header section → grandfathered.
        let content = "\
## Adversarial Reviews\n\n\
| Pass | Date | Findings | Status |\n\
|------|------|----------|--------|\n\
| 1 | 2026-04-25 | 17 (1 CRIT + 7 HIGH) | substantive — fixes applied |\n\
| 2 | 2026-04-25 | 11 (1 CRIT + 4 HIGH) | substantive — fixes applied |\n\n\
## S-6.01 Sub-cycle Adversarial Reviews\n";
        let violations = validate_index_md(content);
        assert!(
            violations.is_empty(),
            "brownfield 4-col rows must be grandfathered; no violations"
        );
    }

    #[test]
    fn test_index_md_production_edp1() {
        // Simulate EDP1 INDEX.md — 5-col-header section; all rows 5-col → pass.
        let content = "\
## Adversarial Reviews\n\n\
> Format note.\n\n\
| Pass | Date | Findings Count | Verdict | File |\n\
|------|------|---------------|---------|------|\n\
| 1 | 2026-05-07 | 29 (4C+14H+6M+5L) | CRITICAL | adv-cycle-pass-1.md |\n\
| 2 | 2026-05-07 | 15 (2C+6H+4M+3L) | CRITICAL | adv-cycle-pass-2.md |\n\n\
## Convergence Status\n";
        let violations = validate_index_md(content);
        assert!(
            violations.is_empty(),
            "EDP1 5-col rows must all pass; no violations"
        );
    }

    // -- UTF-8 / em-dash safety (AC-20) --

    /// AC-20: `current_step:` value containing em-dash (U+2014) or other
    /// multi-byte UTF-8 characters must not cause a panic. This test invokes
    /// `extract_current_step` on content with `—` adjacent to a digit and
    /// asserts the call completes without panicking.
    ///
    /// "If I include this real implementation, will the test for this function
    /// pass trivially without any implementer work?" — No. The extraction
    /// logic must guard is_char_boundary() correctly for this test to pass.
    #[test]
    fn validate_current_step_with_em_dash() {
        let content = "---\ncurrent_step: 'pass 74—D-382..D-477 →9→9→9→9'\n---\nbody\n";
        // Must not panic; correctness of returned value tested by implementer.
        let _ = extract_current_step(content);
    }

    // -- Cycle-identity gate (D-439(b)/D-451(c) F5-scope) --

    /// Brownfield cycle must NOT trigger 4-index-cite or trajectory-tail violations.
    ///
    /// Synthetic STATE.md with `current_cycle: v1.0-brownfield-backfill` and a
    /// bare dispatch-step `current_step:` label (D-689 form) plus a valid D-689
    /// body reference.  validate_state_md must return ZERO violations — the
    /// 4-index-cite and trajectory-tail checks are F5-only per D-439(b)/D-451(c).
    ///
    /// This is the primary RED→GREEN guard for the D-439(b)/D-451(c) F5-scope gate
    /// fix. Before the fix it fails with two violations (missing index cites +
    /// missing trajectory-tail prefix); after the fix it passes.
    #[test]
    fn test_brownfield_current_step_no_false_positive() {
        let content = concat!(
            "---\n",
            "current_cycle: v1.0-brownfield-backfill\n",
            "current_step: \"D-689-S18.14-3CLEAN-CONVERGED-PROMOTION-2026-06-22\"\n",
            "---\n",
            "| D-689 | some decision row |\n",
        );
        let violations = validate_state_md(content);
        assert!(
            violations.is_empty(),
            "brownfield current_step must produce ZERO violations (D-439(b)/D-451(c) \
             F5-only gate); violations found: {violations:?}"
        );
    }

    /// F5 cycle still enforces 4-index-cite AND trajectory-tail.
    ///
    /// Synthetic STATE.md with `current_cycle: v1.0-feature-engine-discipline-pass-1`
    /// and a bare dispatch-step label (missing 4-index cites + trajectory-tail)
    /// must produce violations — confirming the gate does NOT disable F5 checks.
    #[test]
    fn test_f5_cycle_still_enforces_index_and_trajectory_checks() {
        let content = concat!(
            "---\n",
            "current_cycle: v1.0-feature-engine-discipline-pass-1\n",
            "current_step: \"D-689-some-f5-step\"\n",
            "---\n",
            "| D-689 | some row |\n",
        );
        let violations = validate_state_md(content);
        let descs: Vec<&str> = violations.iter().map(|v| v.description.as_str()).collect();
        // Check 1: 4-index version citations (D-439(b)) must fire.
        assert!(
            descs.iter().any(|d| d.contains("BC-INDEX v")),
            "F5 cycle: missing 4-index cites (D-439(b)) must produce a violation; got: {descs:?}"
        );
        // Check 2: trajectory-tail LENGTH=4 (D-451(c)) must fire independently.
        assert!(
            descs.iter().any(|d| d.contains("trajectory-tail")),
            "F5 cycle: missing trajectory-tail (D-451(c)) must produce a violation; got: {descs:?}"
        );
    }

    /// extract_current_cycle returns the bare cycle value from frontmatter.
    #[test]
    fn test_extract_current_cycle_brownfield() {
        let content = "---\ncurrent_cycle: v1.0-brownfield-backfill\ncurrent_step: 'x'\n---\n";
        assert_eq!(
            extract_current_cycle(content),
            Some("v1.0-brownfield-backfill")
        );
    }

    /// extract_current_cycle returns the F5 cycle id from frontmatter.
    #[test]
    fn test_extract_current_cycle_f5() {
        let content =
            "---\ncurrent_cycle: v1.0-feature-engine-discipline-pass-1\ncurrent_step: 'x'\n---\n";
        assert_eq!(
            extract_current_cycle(content),
            Some("v1.0-feature-engine-discipline-pass-1")
        );
    }

    /// extract_current_cycle returns None when field is absent.
    #[test]
    fn test_extract_current_cycle_absent() {
        let content = "---\ncurrent_step: 'x'\n---\n";
        assert_eq!(extract_current_cycle(content), None);
    }

    /// is_f5_cycle is conservative: None (unknown cycle) → true (apply F5 checks).
    #[test]
    fn test_is_f5_cycle_none_is_conservative() {
        assert!(
            is_f5_cycle(None),
            "unknown cycle must default to applying F5 checks (conservative)"
        );
    }

    // -- Production false-positive regression (AC-21 preemptive; S-15.09 lesson) --

    /// Resolve a `.factory/`-relative path to an absolute path, handling both
    /// normal and worktree layouts.
    ///
    /// Resolution order:
    /// 1. `CARGO_MANIFEST_DIR/../../../<relative>` (standard workspace layout)
    /// 2. Main repo root via `git rev-parse --git-common-dir`, then `<root>/<relative>`
    ///    (worktree layout where `CARGO_MANIFEST_DIR` is inside a linked worktree)
    ///
    /// Returns the first path that exists. If neither exists, returns the
    /// manifest-dir-relative path so the caller can emit a meaningful error.
    fn resolve_factory_path(relative: &str) -> std::path::PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let direct = std::path::PathBuf::from(manifest_dir)
            .join("../../../")
            .join(relative);
        if direct.exists() {
            return direct;
        }
        // Worktree mode: git --git-common-dir points to main repo's .git.
        // Its parent is the main repo root.
        let common_dir = std::process::Command::new("git")
            .args(["-C", manifest_dir, "rev-parse", "--git-common-dir"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());
        if let Some(cd) = common_dir {
            let git_common = std::path::PathBuf::from(&cd);
            // --git-common-dir returns either ".git" (main repo) or an absolute path
            // (/path/to/main/.git) for worktrees. Parent is the repo root.
            let root = if git_common.is_absolute() {
                git_common.parent().unwrap_or(&git_common).to_path_buf()
            } else {
                // Relative: anchor to manifest_dir and resolve
                std::path::PathBuf::from(manifest_dir)
                    .join(&git_common)
                    .parent()
                    .unwrap_or(std::path::Path::new(manifest_dir))
                    .to_path_buf()
            };
            let via_common = root.join(relative);
            if via_common.exists() {
                return via_common;
            }
        }
        // Return direct path even if missing — caller will emit a meaningful error.
        direct
    }

    /// Load-bearing preemptive test: reads the real `.factory/STATE.md` from
    /// disk and asserts `validate_state_md` emits no violations. This catches
    /// false-positive blocks that would fire against production content before
    /// any bats test runs. Added preemptively per S-15.09 AC-13 cascade lesson.
    ///
    /// Path resolved via `resolve_factory_path` which handles both normal and
    /// worktree layouts (F-P2-006 fix: no longer silently skips in worktree mode).
    ///
    /// Fails loud if STATE.md cannot be found in any search path. The only
    /// exception is an explicit opt-out via env var
    /// `VSDD_SKIP_PRODUCTION_STATE_MD_TEST=1`, which is documented and
    /// intentional (e.g., ephemeral CI runners without a factory worktree
    /// mounted as a subtree).
    #[test]
    fn validate_production_state_md_no_false_positive() {
        if std::env::var("VSDD_SKIP_PRODUCTION_STATE_MD_TEST").as_deref() == Ok("1") {
            eprintln!(
                "validate_production_state_md_no_false_positive: \
                 VSDD_SKIP_PRODUCTION_STATE_MD_TEST=1 — skipping (explicit opt-out)"
            );
            return;
        }
        let state_md_path = resolve_factory_path(".factory/STATE.md");
        let content = match std::fs::read_to_string(&state_md_path) {
            Ok(c) => c,
            Err(e) => {
                panic!(
                    "validate_production_state_md_no_false_positive: STATE.md \
                     not found at {} (also tried git --git-common-dir resolution): {}; \
                     set VSDD_SKIP_PRODUCTION_STATE_MD_TEST=1 to opt out explicitly \
                     (F-P2-006)",
                    state_md_path.display(),
                    e
                );
            }
        };
        let violations = validate_state_md(&content);
        assert!(
            violations.is_empty(),
            "validate_state_md must not produce false-positive violations \
             against production STATE.md; violations found: {violations:?}"
        );
    }

    /// Validate production EDP1 INDEX.md against validate_index_md — no false positives.
    /// Preemptive test matching the same pattern as validate_production_state_md.
    /// Uses `resolve_factory_path` for worktree-resilient path resolution (F-P2-006).
    #[test]
    fn validate_production_edp1_index_md_no_false_positive() {
        if std::env::var("VSDD_SKIP_PRODUCTION_STATE_MD_TEST").as_deref() == Ok("1") {
            eprintln!(
                "validate_production_edp1_index_md_no_false_positive: \
                 VSDD_SKIP_PRODUCTION_STATE_MD_TEST=1 — skipping (explicit opt-out)"
            );
            return;
        }
        let index_md_path =
            resolve_factory_path(".factory/cycles/v1.0-feature-engine-discipline-pass-1/INDEX.md");
        let content = match std::fs::read_to_string(&index_md_path) {
            Ok(c) => c,
            Err(e) => {
                panic!(
                    "validate_production_edp1_index_md_no_false_positive: INDEX.md \
                     not found at {} (also tried git --git-common-dir resolution): {}; \
                     set VSDD_SKIP_PRODUCTION_STATE_MD_TEST=1 to opt out explicitly \
                     (F-P2-006)",
                    index_md_path.display(),
                    e
                );
            }
        };
        let violations = validate_index_md(&content);
        assert!(
            violations.is_empty(),
            "validate_index_md must not produce false-positive violations \
             against production EDP1 INDEX.md; violations found: {violations:?}"
        );
    }
}
