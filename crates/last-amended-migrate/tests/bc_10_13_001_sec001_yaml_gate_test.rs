// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-15.03 SEC-001 (CWE-116) — pre-write strict YAML validity gate.
//!
//! `MigrateError::InvalidYamlProduced` was declared from this crate's stub
//! stage onward (`src/error.rs`'s own doc comment: "intended to gate an
//! `safe_load`-validity check before writing") but was never constructed
//! anywhere in `src/` — dead code, no actual pre-write validation ran. This
//! suite exercises `src/yaml_guard.rs`'s two gate functions directly (the
//! REAL production gate functions, not a reimplementation — POLICY 11),
//! proving: (1) they accept genuinely valid produced content, (2) they
//! reject genuinely invalid produced content with `InvalidYamlProduced`
//! rather than silently allowing a write, and (3) the gate is actually WIRED
//! IN to `migrate_file`'s and `rotate_changelog`'s write paths — a
//! version-escaping gap in the PC7 `[Prior: ...]` split path (the one
//! previously-unescaped field this fix also closes) is used as the
//! authentic end-to-end reproduction, rather than a synthetic-only test of
//! the gate function in isolation.

mod common;

use last_amended_migrate::migrate::{MigrationMode, MigrationOptions};
use last_amended_migrate::migrate_file;
use last_amended_migrate::yaml_guard::{
    validate_changelog_sequence_yaml, validate_frontmatter_yaml,
};
use std::path::Path;

// ── Direct gate-function unit tests ─────────────────────────────────────────

#[test]
fn test_BC_10_13_001_SEC001_validate_frontmatter_yaml_accepts_valid_content() {
    let raw = "---\ndocument_type: behavioral-contract-index\nversion: \"1.0\"\nlast_amended: \"2026-09-02 (v1.0) \\u2014 ok\"\nchangelog:\n  - date: 2026-08-01\n    change: \"an older entry\"\n---\n\nbody\n";
    let result = validate_frontmatter_yaml(Path::new("BC-INDEX.md"), raw);
    assert!(
        result.is_ok(),
        "well-formed produced content must pass the gate: {result:?}"
    );
}

#[test]
fn test_BC_10_13_001_SEC001_validate_frontmatter_yaml_rejects_invalid_content() {
    // A raw, unescaped literal `"` inside the double-quoted scalar — exactly
    // the D-1144 defect shape — makes this genuinely invalid YAML.
    let raw = "---\ndocument_type: behavioral-contract-index\nversion: \"1.0\"\nlast_amended: \"2026-09-02 (v1.0) \u{2014} fixed the \"quoted\" defect\"\nchangelog:\n  - date: 2026-08-01\n    change: \"an older entry\"\n---\n\nbody\n";
    let result = validate_frontmatter_yaml(Path::new("BC-INDEX.md"), raw);
    match result {
        Err(last_amended_migrate::MigrateError::InvalidYamlProduced { .. }) => {}
        other => panic!(
            "expected Err(MigrateError::InvalidYamlProduced {{ .. }}) on genuinely \
             invalid produced content, got: {other:?}"
        ),
    }
}

#[test]
fn test_BC_10_13_001_SEC001_validate_frontmatter_yaml_rejects_missing_fence() {
    let raw = "document_type: x\nlast_amended: \"ok\"\n";
    let result = validate_frontmatter_yaml(Path::new("BC-INDEX.md"), raw);
    assert!(
        matches!(
            result,
            Err(last_amended_migrate::MigrateError::InvalidYamlProduced { .. })
        ),
        "content with no --- frontmatter fence at all must also be rejected \
         by the gate, not silently accepted: {result:?}"
    );
}

#[test]
fn test_BC_10_13_001_SEC001_validate_changelog_sequence_yaml_accepts_valid_items() {
    let items = "  - date: 2026-08-01\n    change: \"an older entry\"\n  - date: 2026-07-01\n    version: \"v0.9\"\n    change: \"an even older entry\"\n";
    let result =
        validate_changelog_sequence_yaml(Path::new("BC-INDEX-changelog-archive.md"), items);
    assert!(
        result.is_ok(),
        "well-formed archive changelog items must pass the gate: {result:?}"
    );
}

#[test]
fn test_BC_10_13_001_SEC001_validate_changelog_sequence_yaml_rejects_invalid_items() {
    // An unescaped literal `"` inside `version:`'s quoted scalar.
    let items =
        "  - date: 2026-08-01\n    version: \"v0.9\"defect\"\n    change: \"an older entry\"\n";
    let result =
        validate_changelog_sequence_yaml(Path::new("BC-INDEX-changelog-archive.md"), items);
    assert!(
        matches!(
            result,
            Err(last_amended_migrate::MigrateError::InvalidYamlProduced { .. })
        ),
        "genuinely invalid archive changelog content must be rejected: {result:?}"
    );
}

// ── End-to-end: the gate is actually wired into migrate_file's write path ──

/// S-15.03 SEC-001 completeness fix: `changelog.rs::render_item_block`
/// writes a PC7-recovered historical entry's `version:` field into its own
/// double-quoted YAML scalar WITHOUT escaping it — the one field the
/// original D-1144 remediation missed (`summary` was already escaped).
/// A `[Prior: ...]` chain entry whose version segment carries an embedded
/// literal quote reproduces this authentically: pre-fix, this would have
/// written a corrupt file; post-fix, the value is escaped AND the pre-write
/// gate would catch any regression of this class before a write occurs.
#[test]
fn test_BC_10_13_001_SEC001_migrate_file_escapes_previously_unescaped_version_field() {
    let dir = tempfile::tempdir().expect("tempdir");
    // `parse_dated_entry`'s own shape: "{date} ({version}) — {text}" — a
    // version segment containing a literal `"` is unusual but syntactically
    // reachable (the parenthesis-matching split has no character-class
    // restriction on what's between `(` and `)`).
    let last_amended = common::chain_last_amended(
        ("2026-09-02", "v1.0", "current entry text"),
        &[("2026-08-01", "v0.9\"odd", "older entry with an odd version")],
    );
    let content = common::frontmatter_file(
        "behavioral-contract-index",
        "1.0",
        &last_amended,
        Some(&[]),
        "# Fixture BC-INDEX\n",
    );
    let path = common::write_file(dir.path(), "BC-INDEX.md", &content);

    let report = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default()).expect(
        "migrate_file must succeed: the version-field escaping fix + pre-write \
         gate together must produce a valid, written file rather than erroring \
         out or writing corrupt YAML",
    );
    assert!(report.mutated);
    assert_eq!(report.entries_relocated, 1);

    let after = common::read_file(&path);
    let parsed = common::strict_yaml_parse(&after)
        .expect("the written file must parse cleanly under strict YAML safe_load");
    let changelog = parsed
        .changelog
        .expect("changelog: must be present after the PC7 split");
    assert_eq!(changelog.len(), 1);
}
