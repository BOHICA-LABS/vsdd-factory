// Test-support files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
// This module is `mod common;`-included by many separate `tests/bc_*.rs`
// integration-test binaries (Cargo compiles each `tests/*.rs` file as its
// own crate); no single one of them uses every helper here, so per-binary
// dead-code warnings are expected and not a real defect — CI runs
// `cargo clippy --workspace --all-targets -- -D warnings`, which would
// otherwise fail the build on this intentional, shared-fixture-module
// sharing pattern (mirrors how other workspace test-support modules handle
// the same "not every consumer uses every helper" shape).
#![allow(dead_code)]
//! Shared fixture-construction helpers for `last-amended-migrate`'s Red Gate
//! test suite (BC-10.13.001, BC-5.45.001 PC2, BC-4.18.001).
//!
//! `tests/common/mod.rs` is Cargo's special-cased "not a test binary" path
//! (mirrors `crates/policy15-attestation-gate`'s pattern of a private
//! `Repo` helper, but shared across multiple `tests/bc_*.rs` files here via
//! `mod common;` rather than duplicated, since none of these fixtures need
//! to differ per-file).
//!
//! # BC-5.38.001 / POLICY 11 discipline
//!
//! This module builds INPUT fixtures and reads OUTPUT files back from disk;
//! it never reimplements `last-amended-migrate`'s own parsing, eligibility,
//! escaping, or write logic. Every test that needs to inspect a mutated
//! file's structure calls the crate's own real `parse_frontmatter` (or
//! constructs a `FrontmatterDoc` directly via its public fields, which is
//! itself a POLICY-11-compliant use of the real public type) rather than a
//! hand-rolled parser here.

use std::fs;
use std::path::{Path, PathBuf};

use last_amended_migrate::migrate::TARGET_FILES;

/// Write `content` to `dir`/`rel_path`, creating parent directories as
/// needed. Returns the absolute path written.
pub fn write_file(dir: &Path, rel_path: &str, content: &str) -> PathBuf {
    let full = dir.join(rel_path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("create_dir_all fixture parent");
    }
    fs::write(&full, content).expect("write fixture file");
    full
}

pub fn read_file(path: &Path) -> String {
    fs::read_to_string(path).expect("read fixture file back")
}

/// One `changelog:` sequence item block, newest-first convention (matching
/// every real ADR-049-governed file's corpus shape — see
/// `.factory/specs/behavioral-contracts/BC-INDEX.md`'s own `changelog:`
/// sequence for the shape this mirrors).
pub fn changelog_item_block(date: &str, change_text: &str) -> String {
    format!("  - date: {date}\n    change: \"{change_text}\"\n")
}

/// Build one of the 5 ADR-049-governed files' frontmatter shape.
///
/// `last_amended_inner` is written VERBATIM between the outer double quotes
/// — callers control escaping precisely (including deliberately-broken
/// D-1144-defect fixtures), matching `FrontmatterDoc::last_amended_raw`'s own
/// documented "unescaped/unquoted" (i.e. literal-source-text-between-the-
/// quotes) semantics.
///
/// `changelog_items` is `None` for a file that has no `changelog:` key at
/// all (the `STORY-INDEX.md`-before-migration / `STATE.md` shape); `Some(&[])`
/// for a present-but-empty sequence; `Some(items)` for a populated one,
/// newest-first.
pub fn frontmatter_file(
    document_type: &str,
    version: &str,
    last_amended_inner: &str,
    changelog_items: Option<&[String]>,
    body: &str,
) -> String {
    let mut s = String::new();
    s.push_str("---\n");
    s.push_str(&format!("document_type: {document_type}\n"));
    s.push_str(&format!("version: \"{version}\"\n"));
    s.push_str(&format!("last_amended: \"{last_amended_inner}\"\n"));
    if let Some(items) = changelog_items {
        s.push_str("changelog:\n");
        for item in items {
            s.push_str(item);
        }
    }
    s.push_str("---\n\n");
    s.push_str(body);
    s
}

/// A `last_amended` current-entry-only value with NO D-1144 escape defect —
/// the fully-compliant post-ADR-049 shape (BC-10.13.001 EC-001 verified-
/// clean-no-op case).
pub fn clean_current_entry(date: &str, version: &str, summary: &str) -> String {
    format!("{date} ({version}) — {summary}")
}

/// A `last_amended` current-entry-only value carrying the D-1144 unescaped-
/// literal-double-quote defect (BC-10.13.001 PC3's 3-file target class:
/// `BC-INDEX.md`/`ARCH-INDEX.md`/`STATE.md`). Contains 2 unescaped `"`.
pub fn quote_defect_current_entry(date: &str, version: &str) -> String {
    format!("{date} ({version}) — fixed the \"quoted term\" defect")
}

/// A `last_amended` value carrying a nested `[Prior: <date> (vX.Y) — ...]`
/// bracket chain (2 historical entries) — the pre-D-1149 / unbounded-growth
/// shape that BC-10.13.001 v1.1 Precondition 2(b) classifies as ELIGIBLE for
/// the PC7 full-recovery split (superseding the v1.0 "out of scope, NOT
/// eligible" framing this helper predates — see `chain_last_amended` for the
/// general N-entry builder new v1.1 fixtures use).
pub fn prior_chain_last_amended(date: &str, version: &str) -> String {
    chain_last_amended(
        (date, version, "current entry text"),
        &[
            ("2026-08-01", "v0.9", "older entry text"),
            ("2026-07-01", "v0.8", "oldest entry text"),
        ],
    )
}

/// Build a `last_amended` inline `[Prior: ...]` bracket chain with an
/// arbitrary number of nested historical entries (BC-10.13.001 v1.1 PC7 step
/// 3). `current` is `(date, version, text)` for the entry that stays in
/// `last_amended` after the split. `priors` lists each historical entry as
/// `(date, version, text)`, ordered newest-of-the-priors-first: `priors[0]`
/// is nested immediately inside the CURRENT entry's own `[Prior: ...]`
/// bracket (PC7 step 5 — it becomes the first/topmost new `changelog:`
/// item), `priors[last]` is the oldest / innermost-nested entry. Returns the
/// raw (unescaped-by-this-helper) `last_amended` value — callers control
/// escaping of embedded text precisely, matching `frontmatter_file`'s own
/// verbatim-between-quotes convention.
pub fn chain_last_amended(current: (&str, &str, &str), priors: &[(&str, &str, &str)]) -> String {
    let (date, version, text) = current;
    let mut s = format!("{date} ({version}) — {text}");
    for (prior_date, prior_version, prior_text) in priors {
        s.push_str(&format!(
            " [Prior: {prior_date} ({prior_version}) — {prior_text}"
        ));
    }
    s.push_str(&"]".repeat(priors.len()));
    s
}

/// Synthetic MEGA-LINE `last_amended` value reproducing the D-1149
/// 323,499-char `STORY-INDEX.md` calibration ceiling: a single `[Prior:
/// ...]` bracket whose embedded text is padded well past that ceiling
/// (350,000 `x` filler characters — no quotes/colons, so the fixture's
/// *shape* is unambiguous regardless of how the eventual implementation
/// tokenizes it). Proves Invariant 3 (bounded-resource safety on
/// arbitrarily long input) at the EC-009 calibration scale — this is "the
/// whole reason the tool exists": Edit/Write-tool-mediated manual editing
/// cannot safely handle content at this scale (BC-10.13.001 v1.1 Invariant 3
/// doc note), which is why the v1.1 amendment makes the tool actively SPLIT
/// this shape rather than merely refusing/tolerating it.
///
/// # Scope note (S-15.03 B3)
///
/// This fixture is a SINGLE monolithic un-splittable entry — it proves the
/// PC7 bounded/streaming SCAN stays linear-time even at this scale
/// (Invariant 3), which is a genuinely distinct claim from BC-4.18.001's
/// fuel-RELIEF property. Splitting one atomic entry necessarily still
/// leaves a ~350K-char value living somewhere (now inside a single
/// `changelog:` item's `date`/`summary` field instead of `last_amended`) —
/// that is an inherent limit of splitting an un-splittable entry, not a
/// fuel-relief regression. Use `realistic_multi_entry_prior_chain` (below)
/// to exercise the actual fuel-relief claim: a REALISTIC chain of many
/// modest-sized entries, at real D-1149 aggregate scale, where relief means
/// every individual rendered line stays small even though the file's total
/// `changelog:` content is large.
pub fn mega_line_prior_chain(filler_len: usize) -> String {
    let filler = "x".repeat(filler_len);
    format!("2026-09-02 (v1.0) — current entry text [Prior: {filler} (v0.9) — old]")
}

/// A REALISTIC `last_amended` inline `[Prior: ...]` chain built from `n`
/// modest-sized historical entries (each a few hundred to a few thousand
/// characters, matching real ADR-049-governed burst-summary prose — see
/// `.factory/STATE.md`'s own historical `last_amended` chain for the shape
/// this mirrors), summing to real D-1149 incident scale when `n` and
/// `entry_text_chars` are chosen accordingly (e.g. `n=100`,
/// `entry_text_chars=3_000` sums to ~300K total chain chars across ~100
/// entries — the actual shape of the incident this tool exists to recover
/// from, as opposed to `mega_line_prior_chain`'s single atomic monolithic
/// entry).
///
/// This is the fixture BC-4.18.001's fuel-relief property must be proven
/// against: PC7 relocates each of the `n` entries into its own
/// `changelog:` item, so the fuel-relief claim is "every individual
/// rendered line stays close to `entry_text_chars` in size" — NOT "the
/// file's total size shrinks" (`changelog:`'s aggregate size is expected to
/// stay large; only `last_amended` and each individual line within it must
/// shrink).
pub fn realistic_multi_entry_prior_chain(n: usize, entry_text_chars: usize) -> String {
    let mut s = "2026-09-02 (v9.65) — current burst summary text".to_string();
    for i in 0..n {
        let filler = "prose ".repeat(entry_text_chars / 6 + 1);
        let filler: String = filler.chars().take(entry_text_chars).collect();
        s.push_str(&format!(
            " [Prior: 2026-{:02}-{:02} (v9.{}) — {filler}",
            (i % 12) + 1,
            (i % 28) + 1,
            n - i,
        ));
    }
    s.push_str(&"]".repeat(n));
    s
}

/// The 5 D-1149 sidecar basenames this tool must register (BC-10.13.001
/// PC6 / §Architecture Anchors), derived from the crate's own real
/// `TARGET_FILES` constant rather than hardcoded — stays in lockstep with
/// the crate's own source of truth if `TARGET_FILES` ever changes.
pub fn expected_sidecar_basenames() -> Vec<String> {
    TARGET_FILES
        .iter()
        .map(|rel| {
            let stem = Path::new(rel)
                .file_stem()
                .and_then(|s| s.to_str())
                .expect("TARGET_FILES entry has a file stem");
            format!("{stem}-amendment-history.md")
        })
        .collect()
}

/// Minimal strict-YAML frontmatter shape for the `serde_norway` round-trip
/// check (BC-10.13.001 PC3 / Invariant 4; BC-5.45.001 Invariant 3). Extra
/// frontmatter fields are ignored by default serde struct deserialization
/// (no `deny_unknown_fields`), so this only needs the 2 fields this tool
/// operates on.
#[derive(serde::Deserialize)]
pub struct MinimalFrontmatter {
    pub last_amended: String,
    #[serde(default)]
    pub changelog: Option<Vec<serde_norway::Value>>,
}

/// Extract just the `---`-fenced frontmatter block (without the fences) so
/// `serde_norway::from_str` parses only the YAML region, not the markdown
/// body below it.
pub fn frontmatter_block(file_content: &str) -> &str {
    let after_open = file_content
        .strip_prefix("---\n")
        .expect("fixture must start with a frontmatter fence");
    let end = after_open
        .find("\n---")
        .expect("fixture must have a closing frontmatter fence");
    &after_open[..end]
}

/// Parse a file's frontmatter block under strict YAML `safe_load` semantics
/// (`serde_norway::from_str`), independent of this crate's own hand-rolled
/// production parser — a genuine black-box verification technique, not a
/// reimplementation of `parse_frontmatter`'s logic (POLICY 11).
pub fn strict_yaml_parse(file_content: &str) -> Result<MinimalFrontmatter, serde_norway::Error> {
    serde_norway::from_str(frontmatter_block(file_content))
}
