// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-15.03 pr-reviewer B1 (blocking) — PC7 split can emit invalid YAML from
//! colon-leading date tokens.
//!
//! `parse_dated_entry` (`src/migrate.rs`) takes the first whitespace-
//! delimited token of a chained `[Prior: ...]` entry as its `date` field,
//! with NO validation that the token is actually date-shaped
//! (`"{date} ({version}) — {text}"` is the convention, not an enforced
//! grammar). This crate's real corpus (`.factory/STATE.md`'s own
//! `last_amended` chain, pre-D-1149) contains prior entries that begin with
//! a decision-reference prefix like `D-1149:` rather than an ISO date — a
//! genuinely common, non-exotic shape this tool MUST recover correctly, not
//! merely refuse without corrupting.
//!
//! Before the fix, `changelog.rs::render_item_block` wrote `date:` as a bare,
//! UNQUOTED plain YAML scalar (`date: {date}`) — a colon-terminated token
//! such as `D-1149:` produced `date: D-1149:`, which is invalid YAML (an
//! unescaped colon inside a plain scalar reads as a nested mapping
//! indicator). The fix quotes+escapes `date` exactly like `version`/
//! `summary` are already quoted+escaped, so the PC7 split RECOVERS this
//! real-world shape instead of either corrupting the file or refusing to
//! migrate it (BC-10.13.001's whole purpose per EC-003).

mod common;

use last_amended_migrate::migrate::{MigrationMode, MigrationOptions};
use last_amended_migrate::migrate_file;

/// End-to-end: a chained entry beginning with a colon-containing,
/// non-ISO-date-shaped prefix (`D-1149:`, no parenthesized version segment)
/// is preserved correctly by the PC7 split, and the resulting file parses
/// under strict YAML `safe_load` — the direct regression guard for the
/// pr-reviewer's reproduction.
#[test]
fn test_BC_10_13_001_B1_migrate_file_preserves_colon_prefixed_non_date_chain_entry() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Deliberately NOT built via `common::chain_last_amended` — that helper
    // always emits the structured `"{date} ({version}) — {text}"` shape for
    // every entry, including priors. This fixture reproduces the REAL,
    // common legacy shape: a prior entry whose leading token is a
    // decision-reference prefix, not a date, and which carries no
    // parenthesized version segment at all.
    let last_amended = "2026-09-02 (v9.65) — current entry text [Prior: D-1149: emergency \
         incident remediation completed without a version tag]"
        .to_string();
    let content = common::frontmatter_file(
        "pipeline-state",
        "9.65",
        &last_amended,
        None,
        "# Fixture STORY-INDEX\n",
    );
    let path = common::write_file(dir.path(), "STORY-INDEX.md", &content);

    let report = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default()).expect(
        "migrate_file must SUCCEED and recover the colon-prefixed prior entry, not error out \
         or write corrupt YAML — refusing real-world D-1149:-shaped chains defeats \
         BC-10.13.001's whole purpose",
    );
    assert!(report.mutated);
    assert_eq!(
        report.entries_relocated, 1,
        "the colon-prefixed prior entry must be recovered, not dropped or refused"
    );

    let after = common::read_file(&path);
    assert!(
        !after.contains(" [Prior:"),
        "last_amended must be current-entry-only after the split: {after:?}"
    );
    assert!(
        after.contains("emergency"),
        "the recovered entry's substantive text must be preserved verbatim: {after:?}"
    );

    // The direct regression guard: the written file parses cleanly under
    // strict YAML `safe_load` (an independent, general-purpose parser) —
    // pre-fix, this exact fixture shape produced `date: D-1149:` which
    // `serde_norway` rejects with a scanner error ("mapping values are not
    // allowed here").
    let parsed = common::strict_yaml_parse(&after)
        .expect("post-split frontmatter must parse under strict YAML safe_load");
    let changelog = parsed
        .changelog
        .expect("changelog: must be present after the PC7 split");
    assert_eq!(changelog.len(), 1);
}

/// Direct unit-level guard on the rendering primitive itself: a `date`
/// field containing a colon (or any other YAML-significant character) is
/// quoted, exactly like `version`/`summary`, so `prepend_changelog_item`'s
/// own output is valid YAML regardless of what `parse_dated_entry` extracts.
#[test]
fn test_BC_10_13_001_B1_prepend_changelog_item_quotes_colon_containing_date() {
    use last_amended_migrate::changelog::{ChangelogItem, prepend_changelog_item};
    use last_amended_migrate::frontmatter::FrontmatterDoc;
    use std::path::PathBuf;

    let raw = "---\nlast_amended: \"2026-09-02 (v1.0) \u{2014} current entry\"\nchangelog:\n---\n\n# Body\n".to_string();
    let mut doc = FrontmatterDoc {
        path: PathBuf::from("/fixture/STORY-INDEX.md"),
        raw,
        last_amended_raw: Some("2026-09-02 (v1.0) \u{2014} current entry".to_string()),
        changelog_present: true,
        changelog_items_raw: Vec::new(),
    };

    prepend_changelog_item(
        &mut doc,
        ChangelogItem {
            date: "D-1149:".to_string(),
            version: None,
            summary: "emergency incident remediation".to_string(),
        },
    );

    assert!(
        doc.raw.contains("date: \"D-1149:\""),
        "a colon-containing date must be rendered as a quoted YAML scalar, \
         not a bare unquoted one: {:?}",
        doc.raw
    );

    let parsed = common::strict_yaml_parse(&doc.raw)
        .expect("output containing a colon-prefixed date must still parse under strict YAML");
    let changelog = parsed.changelog.expect("changelog: must be present");
    assert_eq!(changelog.len(), 1);
}
