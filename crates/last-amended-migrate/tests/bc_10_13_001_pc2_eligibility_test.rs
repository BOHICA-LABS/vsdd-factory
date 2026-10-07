// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! BC-10.13.001 v1.1 PC2 (current-entry-only confirmation, or full-recovery
//! split into that shape) + PC7 (full-recovery split of an inline
//! `[Prior: ...]` chain) + EC-003/EC-009 (split eligibility, including the
//! mega-line calibration scale) + EC-008 (the narrowed `NotEligible`
//! outcome) + Invariant 3 (bounded-resource safety on arbitrarily long
//! input).
//!
//! # v1.1 amendment note
//!
//! This file previously (v1.0) tested a `[Prior: ...]` chain as
//! `NotEligiblePriorChain` / `Err(MigrateError::NotEligible)` — the tool
//! refused to touch such a file. BC-10.13.001's v1.1 human-directed
//! amendment reframes this: an inline chain is now ELIGIBLE and gets SPLIT
//! (PC7) in the same migration run, making the tool a full replacement for
//! the one-time D-1149 POL-3 exception (S-15.03 AC-010). The tests below
//! were rewritten accordingly; `NotEligible` is now reserved exclusively for
//! EC-008 (a `last_amended` field that cannot be located in frontmatter at
//! all — a malformed-frontmatter condition, never a chain-length or
//! chain-presence condition).

mod common;

use std::time::Instant;

use last_amended_migrate::eligibility::{Eligibility, check_eligibility};
use last_amended_migrate::migrate::{MigrationMode, MigrationOptions};
use last_amended_migrate::{MigrateError, migrate_file};

const QUOTE: char = '\u{22}';

// ── `check_eligibility` unit tests ──────────────────────────────────────────

/// Sanity/positive-classification control: a genuinely current-entry-only
/// value (no `[Prior: ` marker at all) classifies as `CurrentEntryOnly`.
#[test]
fn test_BC_10_13_001_PC2_current_entry_only_no_marker_classifies_eligible() {
    let raw = common::clean_current_entry("2026-09-02", "v1.0", "some entry text");
    assert_eq!(check_eligibility(&raw), Eligibility::CurrentEntryOnly);
}

/// BC-10.13.001 Precondition 2(a): a trailing `[Prior history →
/// <file>-amendment-history.md]` POINTER NOTE (distinct text — "history →",
/// not "Prior: ") must NOT be misclassified as a `[Prior: ...]` chain
/// marker — it is the non-growing D-1149 sidecar-pointer shape (PC6), not
/// the growing inline chain PC7 exists to split.
#[test]
fn test_BC_10_13_001_PC2_trailing_pointer_note_is_not_a_chain_marker() {
    let raw = "2026-09-02 (v1.0) — some entry text [Prior history → BC-INDEX-amendment-history.md]"
        .to_string();
    assert_eq!(
        check_eligibility(&raw),
        Eligibility::CurrentEntryOnly,
        "a `[Prior history \u{2192} ...]` pointer note is NOT a `[Prior: ...]` \
         chain marker and must classify as eligible (no split)"
    );
}

/// BC-10.13.001 v1.1 PC7 / EC-003: a genuine `[Prior: <date> (vX.Y) — ...]`
/// chain classifies as `PriorChainSplit` — ELIGIBLE, not refused.
#[test]
fn test_BC_10_13_001_PC7_prior_chain_classifies_eligible_for_split() {
    let raw = common::prior_chain_last_amended("2026-09-02", "v1.0");
    assert_eq!(check_eligibility(&raw), Eligibility::PriorChainSplit);
}

/// BC-10.13.001 Invariant 3 — bounded-resource safety on arbitrarily long
/// input, calibrated to (and exceeding) the D-1149 323,499-char ceiling.
/// `check_eligibility` must both (a) correctly classify the mega-line as
/// ELIGIBLE for split, and (b) do so in bounded time — a quadratic-
/// backtracking scan would make this test hang rather than fail, which is
/// exactly the defect class this invariant exists to rule out.
#[test]
fn test_BC_10_13_001_invariant3_mega_line_eligibility_scan_is_bounded() {
    let raw = common::mega_line_prior_chain(350_000);
    assert!(
        raw.len() > 323_499,
        "fixture sanity: must exceed the D-1149 calibration ceiling"
    );

    let start = Instant::now();
    let classification = check_eligibility(&raw);
    let elapsed = start.elapsed();

    assert_eq!(classification, Eligibility::PriorChainSplit);
    assert!(
        elapsed.as_secs() < 2,
        "check_eligibility on a {}-byte mega-line took {:?} — Invariant 3 \
         requires a bounded (non-quadratic-backtracking) scan; a hang or \
         multi-second runtime here is the exact D-1149 defect class this \
         tool exists to eliminate",
        raw.len(),
        elapsed
    );
}

// ── `migrate_file` PC7 full-recovery split, end-to-end ──────────────────────

/// BC-10.13.001 v1.1 3rd canonical test vector (small split, bootstrap
/// case): a `STORY-INDEX.md`-shaped fixture with NO pre-existing
/// `changelog:` field, whose `last_amended` is a current entry plus 2
/// inline `[Prior: ...]` bracket entries. After migration: `last_amended`
/// becomes current-entry-only (unchanged text); `changelog:` is CREATED and
/// seeded with the 2 split entries in newest-first order, verbatim; report:
/// 1 file mutated, 1 split (2 entries recovered).
#[test]
fn test_BC_10_13_001_PC7_migrate_file_splits_chain_and_bootstraps_changelog() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended = common::chain_last_amended(
        ("2026-09-02", "v4.430", "some entry text"),
        &[
            ("2026-08-15", "v4.20", "previous change entry"),
            ("2026-07-01", "v4.10", "earlier change entry"),
        ],
    );
    let content = common::frontmatter_file(
        "story-index",
        "4.430",
        &last_amended,
        None,
        "# Fixture STORY-INDEX\n",
    );
    let path = common::write_file(dir.path(), "STORY-INDEX.md", &content);
    assert!(!content.contains("changelog:"), "fixture sanity");

    let report = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default())
        .expect("migrate_file must succeed");

    assert!(report.mutated, "a PC7 split is a mutation");
    assert_eq!(
        report.eligibility,
        Eligibility::PriorChainSplit,
        "report must record that a split occurred"
    );
    assert_eq!(
        report.entries_relocated, 2,
        "both chained entries must be recovered"
    );

    let after = common::read_file(&path);
    assert!(
        !after.contains(" [Prior:"),
        "last_amended must be current-entry-only after the split — no \
         inline chain marker may remain: {after:?}"
    );
    assert!(
        after.contains("some entry text"),
        "the current entry's text must be preserved verbatim: {after:?}"
    );
    assert!(
        after.contains("changelog:"),
        "changelog: must be created (PC7 step 6 bootstrap): {after:?}"
    );
    assert!(after.contains("previous change entry"));
    assert!(after.contains("earlier change entry"));

    let newer_pos = after
        .find("previous change entry")
        .expect("newer split entry must be present");
    let older_pos = after
        .find("earlier change entry")
        .expect("older split entry must be present");
    assert!(
        newer_pos < older_pos,
        "PC7 step 5: split entries must be prepended newest-first — the \
         entry nested immediately inside the current entry's bracket \
         (\"previous change entry\") must appear before the more deeply \
         nested one (\"earlier change entry\"): {after:?}"
    );
}

/// BC-10.13.001 v1.1 4th canonical test vector (split + D-1144 escape
/// combo): a `BC-INDEX.md`-shaped fixture with M=1 pre-existing
/// `changelog:` item, whose `last_amended` is a clean current entry plus 1
/// inline `[Prior: ...]` entry containing an unescaped `"`. After migration:
/// `last_amended` becomes current-entry-only; `changelog:` gains 1 new item
/// at position 0 with the `"` escaped to `\"`; the pre-existing item is
/// unchanged, byte-for-byte, now after it; report: 1 file mutated, 1 split
/// (1 entry recovered), 1 escape fix.
#[test]
fn test_BC_10_13_001_PC7_migrate_file_splits_chain_with_d1144_escape_combo() {
    let dir = tempfile::tempdir().expect("tempdir");
    let quoted_prior_text = format!("fixed the {QUOTE}quoted term{QUOTE} defect");
    let last_amended = common::chain_last_amended(
        ("2026-09-02", "v5.42", "clean current text"),
        &[("2026-08-20", "v5.41", &quoted_prior_text)],
    );
    let existing_item = common::changelog_item_block("2026-08-01", "an older entry");
    let content = common::frontmatter_file(
        "behavioral-contract-index",
        "5.42",
        &last_amended,
        Some(&[existing_item]),
        "# Fixture BC-INDEX\n",
    );
    let path = common::write_file(dir.path(), "BC-INDEX.md", &content);

    // Fixture sanity: the embedded raw quote breaks strict YAML before the
    // fix is applied (same discipline as the PC3 escape-test file).
    assert!(
        common::strict_yaml_parse(&content).is_err(),
        "fixture sanity: an unescaped embedded quote inside the chained \
         entry must break strict YAML parsing before the split+escape fix"
    );

    let report = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default())
        .expect("migrate_file must succeed");

    assert!(report.mutated);
    assert_eq!(report.eligibility, Eligibility::PriorChainSplit);
    assert_eq!(report.entries_relocated, 1);
    assert!(
        report.escape_fixed,
        "the split-relocated entry's embedded quote must be escaped per PC3"
    );

    let after = common::read_file(&path);
    assert!(!after.contains(" [Prior:"));
    assert_eq!(
        after.matches("an older entry").count(),
        1,
        "the pre-existing changelog item must survive untouched: {after:?}"
    );

    let new_item_pos = after
        .find("quoted term")
        .expect("split-relocated entry must be present");
    let old_item_pos = after
        .find("an older entry")
        .expect("pre-existing item must be present");
    assert!(
        new_item_pos < old_item_pos,
        "the newly split entry must be prepended before the pre-existing \
         item (newest-first): {after:?}"
    );

    // Invariant 4 / PC3: the resulting file parses cleanly under strict
    // YAML, and the current entry's text is preserved.
    let parsed = common::strict_yaml_parse(&after)
        .expect("post-split frontmatter must parse under strict YAML safe_load");
    assert!(parsed.last_amended.contains("clean current text"));
}

/// BC-10.13.001 v1.1 5th canonical test vector (EC-009 mega-line
/// calibration): a fixture matching the D-1149 calibration scale
/// (~323K-350K-char inline `[Prior: ...]` chain) must SPLIT successfully —
/// `last_amended` becomes current-entry-only, `changelog:` gains 1 new item
/// — within bounded time via the PC7 linear scan (no non-linear blowup).
/// This is the calibration proof that the tool does what Edit/Write-tool-
/// mediated manual editing cannot perform safely at this scale.
#[test]
fn test_BC_10_13_001_EC009_mega_line_file_migrate_file_splits_within_bounded_time() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended = common::mega_line_prior_chain(350_000);
    let content = common::frontmatter_file(
        "story-index",
        "4.430",
        &last_amended,
        None,
        "# Fixture STORY-INDEX\n",
    );
    let path = common::write_file(dir.path(), "STORY-INDEX.md", &content);
    let before = common::read_file(&path);
    assert!(before.len() > 323_499, "fixture sanity");

    let start = Instant::now();
    let result = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default());
    let elapsed = start.elapsed();

    let report = result.expect("mega-line chain must SPLIT successfully, not error");
    assert!(
        report.mutated,
        "a PC7 split on the mega-line must mutate the file"
    );
    assert_eq!(report.eligibility, Eligibility::PriorChainSplit);
    assert_eq!(
        report.entries_relocated, 1,
        "the mega-line fixture carries exactly 1 historical entry"
    );
    assert!(
        elapsed.as_secs() < 5,
        "migrate_file's PC7 split on a mega-line fixture took {elapsed:?} — \
         must stay bounded (Invariant 3)"
    );

    let after = common::read_file(&path);
    assert_ne!(
        before, after,
        "the mega-line file MUST be mutated by the split (v1.1 supersedes \
         the v1.0 unmutated-refusal behavior)"
    );
    assert!(
        !after.contains(" [Prior:"),
        "last_amended must be current-entry-only after the mega-line split"
    );
}

// ── EC-008 — the narrowed `NotEligible` outcome ─────────────────────────────

/// BC-10.13.001 v1.1 EC-008: a `last_amended:` field that cannot be located
/// at all in frontmatter is the ONLY remaining `NotEligible` outcome — a
/// malformed-frontmatter condition, never a chain-length or chain-presence
/// condition. The file must be left completely unmutated.
#[test]
fn test_BC_10_13_001_EC008_migrate_file_missing_last_amended_field_is_not_eligible() {
    let dir = tempfile::tempdir().expect("tempdir");
    let content = "---\n\
document_type: behavioral-contract-index\n\
version: \"5.41\"\n\
changelog:\n  - date: 2026-08-01\n    change: \"an older entry\"\n\
---\n\n# Fixture BC-INDEX (no last_amended field at all)\n"
        .to_string();
    let path = common::write_file(dir.path(), "BC-INDEX.md", &content);
    let before = common::read_file(&path);

    let result = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default());

    match result {
        Err(MigrateError::NotEligible { path: err_path }) => {
            assert_eq!(err_path, path, "NotEligible must name the offending file");
        }
        other => {
            panic!("expected Err(MigrateError::NotEligible {{ .. }}) per EC-008, got: {other:?}")
        }
    }

    let after = common::read_file(&path);
    assert_eq!(
        before, after,
        "EC-008: the tool MUST NOT attempt any mutation on a file whose \
         last_amended field cannot be located — content must be byte-for-\
         byte unchanged"
    );
}

/// BC-10.13.001 v1.1 EC-008 (second sub-case): a corrupted frontmatter
/// delimiter (missing closing `---` fence) is a genuinely-unparseable-
/// frontmatter condition — reported via `MigrateError::FrontmatterParse`
/// (the parse-level failure `parse_frontmatter` itself surfaces), which is
/// EC-008's "raw value does not parse ... e.g. a corrupted frontmatter
/// delimiter" sub-case, distinct from the missing-field sub-case above.
#[test]
fn test_BC_10_13_001_EC008_migrate_file_corrupted_frontmatter_delimiter_is_not_eligible() {
    let dir = tempfile::tempdir().expect("tempdir");
    // No closing `---` fence at all — a corrupted/unparseable frontmatter
    // delimiter (EC-008's second sub-case).
    let content = "---\n\
document_type: story-index\n\
version: \"4.430\"\n\
last_amended: \"2026-09-02 (v4.430) — some entry text\"\n\
\n# Fixture STORY-INDEX (frontmatter never closes)\n"
        .to_string();
    let path = common::write_file(dir.path(), "STORY-INDEX.md", &content);
    let before = common::read_file(&path);

    let result = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default());

    assert!(
        matches!(result, Err(MigrateError::FrontmatterParse { .. })),
        "expected Err(MigrateError::FrontmatterParse {{ .. }}) on a corrupted \
         frontmatter delimiter (EC-008), got: {result:?}"
    );

    let after = common::read_file(&path);
    assert_eq!(
        before, after,
        "EC-008: no mutation attempted on genuinely-unparseable frontmatter"
    );
}

// ── Pointer note is never mistaken for a chain (no split attempted) ────────

/// BC-10.13.001 v1.1 Precondition 2 / PC7 step 1: a file whose
/// `last_amended` is ALREADY current-entry-only plus a trailing `[Prior
/// history → <file>-amendment-history.md]` pointer note must be a PC4
/// no-op — the pointer note is the non-growing D-1149 sidecar-pointer shape
/// (PC6), never mistaken for the growing inline `[Prior: ...]` chain marker
/// PC7 splits. `migrate_file` must NOT attempt any split on this fixture.
#[test]
fn test_BC_10_13_001_PC7_pointer_note_fixture_is_pc4_noop_not_split() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended = "2026-09-02 (v5.41) — some entry text \
        [Prior history \u{2192} BC-INDEX-amendment-history.md]";
    let content = common::frontmatter_file(
        "behavioral-contract-index",
        "5.41",
        last_amended,
        Some(&[common::changelog_item_block("2026-08-01", "an older entry")]),
        "# Fixture BC-INDEX\n",
    );
    let path = common::write_file(dir.path(), "BC-INDEX.md", &content);
    let before = common::read_file(&path);

    let report = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default())
        .expect("migrate_file must succeed");

    assert!(
        !report.mutated,
        "a pointer note is not a chain — this must be a PC4 no-op, never a \
         PC7 split"
    );
    assert_eq!(
        report.entries_relocated, 0,
        "no entries may be 'recovered' from a pointer note"
    );
    assert_eq!(report.eligibility, Eligibility::CurrentEntryOnly);

    let after = common::read_file(&path);
    assert_eq!(
        before, after,
        "content must be byte-for-byte unchanged on this no-op"
    );
}
