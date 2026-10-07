// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! BC-10.13.001 PC4 (migration idempotency) + Invariant 2 (idempotent on
//! every subcommand) + EC-001 (already-fully-compliant file is a
//! verified-clean no-op).
//!
//! Complements the idempotency assertions embedded in the PC3 escape-test
//! file (which covers the escape-specific idempotency angle) with the
//! general contract across a variety of starting shapes: a file that was
//! ALREADY fully compliant before the first call ever runs, the
//! "second run after a real mutation" angle from a `changelog:`-bootstrap
//! starting point rather than an escape-fix starting point, and — new in
//! v1.1 — the PC7 step 8 / Invariant 2 guarantee that a full-recovery split
//! is a one-time event: re-running migration against the now-slim,
//! post-split file is governed by this same PC4 no-op contract, not by PC7
//! again (it does not re-detect a chain where none remains, and never
//! re-splits).

mod common;

use last_amended_migrate::eligibility::Eligibility;
use last_amended_migrate::migrate::{MigrationMode, MigrationOptions};
use last_amended_migrate::migrate_file;

/// PC4 / EC-001: a fixture that is ALREADY fully compliant (has
/// `changelog:`, clean current-entry-only `last_amended`, no D-1144 defect)
/// produces a verified-clean report — zero mutations — on the FIRST call,
/// and the file content is byte-for-byte unchanged.
#[test]
fn test_BC_10_13_001_EC001_already_compliant_file_is_noop_on_first_run() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended = common::clean_current_entry("2026-09-02", "v4.11", "already compliant");
    let content = common::frontmatter_file(
        "architecture-index",
        "4.11",
        &last_amended,
        Some(&[common::changelog_item_block("2026-08-01", "an older entry")]),
        "# Fixture ARCH-INDEX\n",
    );
    let path = common::write_file(dir.path(), "ARCH-INDEX.md", &content);
    let before = common::read_file(&path);

    let report = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default())
        .expect("migrate_file must succeed");

    assert!(!report.mutated, "EC-001: already-compliant file is a no-op");
    assert!(!report.escape_fixed);
    let after = common::read_file(&path);
    assert_eq!(before, after, "content must be byte-for-byte unchanged");
}

/// PC4: running migration a SECOND time (after a REAL first-run mutation
/// that added `changelog:`) is a verified-clean no-op — the tool "detects
/// the already-compliant shape and takes no destructive or duplicating
/// action (it never double-adds a `changelog:` field ...)".
#[test]
fn test_BC_10_13_001_PC4_second_run_after_changelog_bootstrap_is_noop() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended = common::clean_current_entry("2026-09-02", "v4.430", "bootstrap fixture");
    let content = common::frontmatter_file(
        "story-index",
        "4.430",
        &last_amended,
        None,
        "# Fixture STORY-INDEX\n",
    );
    let path = common::write_file(dir.path(), "STORY-INDEX.md", &content);

    let first = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default())
        .expect("first migrate_file call");
    assert!(first.mutated, "first run adds changelog: — a real mutation");
    let after_first = common::read_file(&path);

    let second = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default())
        .expect("second migrate_file call");
    assert!(
        !second.mutated,
        "PC4: second run must be a verified-clean no-op (0 mutations)"
    );

    let after_second = common::read_file(&path);
    assert_eq!(
        after_first, after_second,
        "PC4: second run must produce byte-identical output to the first \
         run's result — no double-add, no re-wrap"
    );
}

/// Invariant 2 extends the idempotency guarantee to `Check` mode too:
/// running `Check` against an already-migrated file must also report zero
/// mutations (and, being `Check` mode, must never write regardless).
#[test]
fn test_BC_10_13_001_invariant2_check_mode_after_apply_reports_zero_mutations() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended = common::clean_current_entry("2026-09-02", "v4.430", "bootstrap fixture");
    let content = common::frontmatter_file(
        "story-index",
        "4.430",
        &last_amended,
        None,
        "# Fixture STORY-INDEX\n",
    );
    let path = common::write_file(dir.path(), "STORY-INDEX.md", &content);

    let apply =
        migrate_file(&path, MigrationMode::Apply, MigrationOptions::default()).expect("apply call");
    assert!(apply.mutated);
    let after_apply = common::read_file(&path);

    let check =
        migrate_file(&path, MigrationMode::Check, MigrationOptions::default()).expect("check call");
    assert!(
        !check.mutated,
        "Check mode after a real Apply must report zero mutations (already compliant)"
    );

    let after_check = common::read_file(&path);
    assert_eq!(
        after_apply, after_check,
        "Check mode must NEVER write, by definition"
    );
}

/// BC-10.13.001 v1.1 6th canonical test vector (re-run idempotency after
/// split): running migration a SECOND time on a file that was just resolved
/// by a PC7 full-recovery split is a verified-clean no-op — PC7 step 8 /
/// Invariant 2's "fires at most once per pre-existing chain, never re-
/// detects a chain where none remains, and never re-splits" guarantee. The
/// second run's output must be byte-identical to the first run's result.
#[test]
fn test_BC_10_13_001_PC7_rerun_after_split_is_idempotent_noop() {
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

    let first = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default())
        .expect("first migrate_file call (split)");
    assert!(
        first.mutated,
        "the first run performs the PC7 split — a real mutation"
    );
    assert_eq!(first.eligibility, Eligibility::PriorChainSplit);
    assert_eq!(first.entries_relocated, 2);
    let after_first = common::read_file(&path);
    assert!(
        !after_first.contains(" [Prior:"),
        "fixture sanity: split resolved the chain"
    );

    let second = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default())
        .expect("second migrate_file call (post-split)");
    assert!(
        !second.mutated,
        "PC7 step 8 / PC4: re-running migration against the now-slim, \
         post-split file must be a verified-clean no-op"
    );
    assert_eq!(
        second.eligibility,
        Eligibility::CurrentEntryOnly,
        "the post-split file must reclassify as CurrentEntryOnly — there is \
         no chain left to re-detect"
    );
    assert_eq!(
        second.entries_relocated, 0,
        "the second run must not report any entries recovered — it does not \
         re-split"
    );

    let after_second = common::read_file(&path);
    assert_eq!(
        after_first, after_second,
        "PC7/PC4: the second run must produce byte-identical output to the \
         first run's split result — no re-wrap, no re-split"
    );
}
