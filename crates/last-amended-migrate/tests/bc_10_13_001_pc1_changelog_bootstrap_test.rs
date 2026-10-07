// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! BC-10.13.001 PC1 (`changelog:` coverage completion across the 5 files) +
//! EC-002 (`STORY-INDEX.md` first-migration bootstrap) + EC-006 (`STATE.md`
//! never gains a `changelog:` field).
//!
//! Two layers of coverage:
//!
//! 1. **Direct unit tests of `ensure_changelog_field`** — constructed via a
//!    hand-built `FrontmatterDoc` (every field is `pub`), decoupling these
//!    tests from `parse_frontmatter`'s own (separately Red-Gated) I/O and
//!    parsing correctness. This is the PC1-implementing function per
//!    `src/lib.rs`'s own module doc table, so exercising it directly with a
//!    real `&mut FrontmatterDoc` is a genuine POLICY-11-compliant test of
//!    real production logic, not a reimplementation.
//! 2. **End-to-end `migrate_file` integration tests** against real fixture
//!    files, proving the orchestration actually invokes step 1's function
//!    with the correct `is_state_file` derivation and persists the result to
//!    disk.

mod common;

use std::path::PathBuf;

use last_amended_migrate::changelog::{ChangelogMutation, ensure_changelog_field};
use last_amended_migrate::frontmatter::FrontmatterDoc;
use last_amended_migrate::migrate::{MigrationMode, MigrationOptions};
use last_amended_migrate::migrate_file;

fn doc_without_changelog(raw: &str) -> FrontmatterDoc {
    FrontmatterDoc {
        path: PathBuf::from("/fixture/STORY-INDEX.md"),
        raw: raw.to_string(),
        last_amended_raw: Some("2026-09-02 (v1.0) — some entry".to_string()),
        changelog_present: false,
        changelog_items_raw: Vec::new(),
    }
}

fn doc_with_changelog(raw: &str, items: Vec<String>) -> FrontmatterDoc {
    FrontmatterDoc {
        path: PathBuf::from("/fixture/BC-INDEX.md"),
        raw: raw.to_string(),
        last_amended_raw: Some("2026-09-02 (v1.0) — some entry".to_string()),
        changelog_present: true,
        changelog_items_raw: items,
    }
}

// ── Direct `ensure_changelog_field` unit tests ──────────────────────────────

/// PC1 / EC-002: `changelog:` absent, not `STATE.md` → `Added`, the field
/// now exists in both the bookkeeping flag and the byte-patched `raw` text.
#[test]
fn test_BC_10_13_001_PC1_ensure_changelog_field_adds_when_absent() {
    let raw = "---\nlast_amended: \"2026-09-02 (v1.0) — some entry\"\n---\n\n# Body\n";
    let mut doc = doc_without_changelog(raw);
    assert!(
        !doc.raw.contains("changelog:"),
        "fixture sanity: no changelog: key yet"
    );

    let outcome = ensure_changelog_field(&mut doc, false);

    assert_eq!(outcome, ChangelogMutation::Added);
    assert!(
        doc.changelog_present,
        "changelog_present bookkeeping flag must flip to true"
    );
    assert!(
        doc.raw.contains("changelog:"),
        "the byte-patched raw text must now carry a changelog: key: {:?}",
        doc.raw
    );
}

/// PC1: `changelog:` already present → verified no-op (`AlreadyPresent`),
/// and — critically — `raw` is BYTE-IDENTICAL afterward (no duplicate
/// `changelog:` key added).
#[test]
fn test_BC_10_13_001_PC1_ensure_changelog_field_already_present_is_noop() {
    let raw = "---\nlast_amended: \"2026-09-02 (v1.0) — some entry\"\nchangelog:\n  - date: 2026-08-01\n    change: \"an older entry\"\n---\n\n# Body\n";
    let existing_items = vec![common::changelog_item_block("2026-08-01", "an older entry")];
    let mut doc = doc_with_changelog(raw, existing_items);
    let before = doc.raw.clone();

    let outcome = ensure_changelog_field(&mut doc, false);

    assert_eq!(outcome, ChangelogMutation::AlreadyPresent);
    assert_eq!(
        doc.raw, before,
        "PC4 idempotency: an already-present changelog: must not be re-added \
         or duplicated — raw text must be byte-for-byte unchanged"
    );
    assert_eq!(
        doc.raw.matches("changelog:").count(),
        1,
        "must never end up with 2 changelog: keys"
    );
}

/// PC1 / EC-006: `is_state_file=true` → `SkippedStateFile`, and `changelog:`
/// is NEVER added even though it was absent — `STATE.md` relies on its
/// body-level `## Decisions Log`/`## Phase Progress` instead (ADR-049
/// Decision 4).
#[test]
fn test_BC_10_13_001_EC006_ensure_changelog_field_skips_state_file() {
    let raw = "---\nlast_amended: \"2026-09-02 (v9.65) — some entry\"\n---\n\n# STATE\n";
    let mut doc = doc_without_changelog(raw);
    doc.path = PathBuf::from("/fixture/STATE.md");

    let outcome = ensure_changelog_field(&mut doc, true);

    assert_eq!(outcome, ChangelogMutation::SkippedStateFile);
    assert!(
        !doc.changelog_present,
        "STATE.md must never gain changelog_present=true"
    );
    assert!(
        !doc.raw.contains("changelog:"),
        "STATE.md's raw text must never gain a changelog: key: {:?}",
        doc.raw
    );
}

// ── `migrate_file` end-to-end integration ───────────────────────────────────

/// BC-10.13.001 EC-002: `STORY-INDEX.md`-shaped fixture with NO `changelog:`
/// field, current-entry-only `last_amended`, no D-1144 defect. After
/// migration: `changelog:` sequence added; `last_amended` UNCHANGED; report
/// says 1 file mutated, 0 escape fixes — the BC's own happy-path canonical
/// test vector.
#[test]
fn test_BC_10_13_001_EC002_migrate_file_bootstraps_story_index_changelog() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended = common::clean_current_entry("2026-09-02", "v4.430", "some entry text");
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

    assert!(report.mutated, "adding changelog: is a mutation");
    assert!(!report.escape_fixed, "no D-1144 defect in this fixture");

    let after = common::read_file(&path);
    assert!(
        after.contains("changelog:"),
        "changelog: sequence must be added: {after:?}"
    );
    assert!(
        after.contains(&last_amended),
        "last_amended must be UNCHANGED per the canonical happy-path vector: {after:?}"
    );
}

/// BC-10.13.001 PC1: `BC-INDEX.md`-shaped fixture that ALREADY carries
/// `changelog:` — migration is a verified no-op on that field (confirms
/// presence/validity rather than re-adding it). Combined here with a clean
/// (non-D-1144-defective) `last_amended` so the ENTIRE file is a no-op,
/// isolating this test to the PC1 dimension only.
#[test]
fn test_BC_10_13_001_PC1_migrate_file_reuses_existing_changelog_no_duplicate() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended = common::clean_current_entry("2026-09-02", "v5.41", "some entry text");
    let existing_item = common::changelog_item_block("2026-08-01", "an older entry");
    let content = common::frontmatter_file(
        "verification-property-index",
        "2.99",
        &last_amended,
        Some(&[existing_item]),
        "# Fixture VP-INDEX\n",
    );
    let path = common::write_file(dir.path(), "VP-INDEX.md", &content);

    let report = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default())
        .expect("migrate_file must succeed");

    assert!(
        !report.mutated,
        "fully compliant file (EC-001) must be a verified-clean no-op"
    );

    let after = common::read_file(&path);
    assert_eq!(
        after.matches("changelog:").count(),
        1,
        "must never duplicate the changelog: key: {after:?}"
    );
    assert_eq!(
        after.matches("an older entry").count(),
        1,
        "must never duplicate an existing changelog item: {after:?}"
    );
}
