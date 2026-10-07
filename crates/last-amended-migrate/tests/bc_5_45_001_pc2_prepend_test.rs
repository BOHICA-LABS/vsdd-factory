// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! BC-5.45.001 PC2 (exactly-one prepend of the displaced prior entry) as
//! exercised by `last-amended-migrate`'s own `prepend_changelog_item`
//! primitive (BC-10.13.001 Invariant 4 — "every write the tool performs
//! satisfies BC-5.45.001's write-path invariants"; this tool's writes must
//! themselves be a CONFORMING writer under that BC).
//!
//! # Why this is tested directly rather than via `migrate_file`
//!
//! `src/migrate.rs`'s own orchestration doc comment and BOTH of
//! BC-10.13.001's canonical test vectors show `migrate_file` NEVER calling
//! `prepend_changelog_item` in today's scope — its `changelog:`-touching
//! step is only `ensure_changelog_field` (PC1, add-if-absent); `last_amended`
//! is either left untouched or rewritten IN PLACE for the D-1144 escape fix
//! (PC3), never displaced into a new `changelog:` item during migration.
//! `prepend_changelog_item` is nonetheless real, exported, `todo!()`-stubbed
//! production API (`src/changelog.rs`) whose own doc comment explicitly ties
//! it to "the BC-5.45.001 PC2 discipline that this tool's own migration/
//! rotation output must satisfy per BC-10.13.001 Invariant 4" — i.e. it is
//! the primitive this tool (or a future write path built on this crate)
//! MUST use if it ever does need to displace a `last_amended` entry into
//! `changelog:`. Testing it directly, with a real `&mut FrontmatterDoc`
//! (every field `pub`), is the correct-altitude test: it pins the PC2
//! contract on the function that implements it without inventing a
//! `migrate_file` call path the BC does not specify.
//!
//! This is also the reference incident BC-INDEX.md v5.41's own changelog
//! entry documents (F-7 BLOCKER, 2026-09-02): "removed the premature v5.40
//! self-summary `changelog:` item that duplicated the still-live
//! `last_amended` entry (PC2 violation: two items prepended where exactly
//! one — the displaced prior entry — is permitted)". This test suite is
//! this discipline's regression guard at the primitive level.

mod common;

use std::path::PathBuf;

use last_amended_migrate::changelog::{ChangelogItem, prepend_changelog_item};
use last_amended_migrate::frontmatter::FrontmatterDoc;

fn doc_with_two_existing_items() -> (FrontmatterDoc, String, String) {
    let item_a = common::changelog_item_block("2026-08-01", "second-newest existing entry");
    let item_b = common::changelog_item_block("2026-07-01", "oldest existing entry");
    let raw = format!(
        "---\nlast_amended: \"2026-09-02 (v5.40) \u{2014} the entry about to be displaced\"\nchangelog:\n{item_a}{item_b}---\n\n# Body\n"
    );
    let doc = FrontmatterDoc {
        path: PathBuf::from("/fixture/BC-INDEX.md"),
        raw,
        last_amended_raw: Some(
            "2026-09-02 (v5.40) \u{2014} the entry about to be displaced".to_string(),
        ),
        changelog_present: true,
        changelog_items_raw: vec![item_a.clone(), item_b.clone()],
    };
    (doc, item_a, item_b)
}

/// BC-5.45.001 PC2 core assertion: prepending exactly one displaced entry
/// grows `changelog_items_raw` by EXACTLY 1 (count delta == 1) — the direct
/// regression guard for the F-7 defect class (two items prepended instead
/// of one).
#[test]
fn test_BC_5_45_001_PC2_prepend_grows_sequence_by_exactly_one() {
    let (mut doc, _item_a, _item_b) = doc_with_two_existing_items();
    let before_len = doc.changelog_items_raw.len();
    assert_eq!(before_len, 2, "fixture sanity");

    prepend_changelog_item(
        &mut doc,
        ChangelogItem {
            date: "2026-09-02".to_string(),
            version: Some("v5.40".to_string()),
            summary: "the entry about to be displaced".to_string(),
        },
    );

    assert_eq!(
        doc.changelog_items_raw.len(),
        before_len + 1,
        "PC2: exactly ONE new item — not zero, not two (the F-7 defect class)"
    );
}

/// BC-5.45.001 PC2: the new item is the DISPLACED PRIOR ENTRY, placed at
/// position 0 (newest-first) — not a premature self-summary of the write
/// itself (the exact F-7 defect: a summary item duplicating the still-live
/// `last_amended` entry rather than carrying the entry it displaced).
#[test]
fn test_BC_5_45_001_PC2_new_item_is_first_and_carries_displaced_entry_text() {
    let (mut doc, item_a, _item_b) = doc_with_two_existing_items();

    prepend_changelog_item(
        &mut doc,
        ChangelogItem {
            date: "2026-09-02".to_string(),
            version: Some("v5.40".to_string()),
            summary: "the entry about to be displaced".to_string(),
        },
    );

    let new_first = &doc.changelog_items_raw[0];
    assert!(
        new_first.contains("the entry about to be displaced"),
        "the new first item must carry the DISPLACED last_amended entry's \
         text, not some other summary: {new_first:?}"
    );
    assert_eq!(
        doc.changelog_items_raw[1], item_a,
        "position 1 must now hold the item that used to be at position 0"
    );
}

/// BC-5.45.001 PC2 / Invariant 4: every PRE-EXISTING `changelog:` item is
/// left byte-for-byte untouched — this is a prepend, never a rewrite-in-
/// place of the sequence or of any existing item.
#[test]
fn test_BC_5_45_001_PC2_existing_items_untouched_byte_for_byte() {
    let (mut doc, item_a, item_b) = doc_with_two_existing_items();

    prepend_changelog_item(
        &mut doc,
        ChangelogItem {
            date: "2026-09-02".to_string(),
            version: Some("v5.40".to_string()),
            summary: "the entry about to be displaced".to_string(),
        },
    );

    assert_eq!(doc.changelog_items_raw.len(), 3);
    assert_eq!(
        doc.changelog_items_raw[1], item_a,
        "existing item A must be byte-for-byte unchanged"
    );
    assert_eq!(
        doc.changelog_items_raw[2], item_b,
        "existing item B must be byte-for-byte unchanged"
    );
    assert!(
        doc.raw.contains(&item_a),
        "raw text must still contain existing item A verbatim: {:?}",
        doc.raw
    );
    assert!(
        doc.raw.contains(&item_b),
        "raw text must still contain existing item B verbatim: {:?}",
        doc.raw
    );
}
