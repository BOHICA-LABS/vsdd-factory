// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-15.03 pr-reviewer B2 / B2-R — PC7 full-recovery split on `STATE.md`.
//!
//! # Why this file exists
//!
//! No prior test in this crate exercised `migrate_file`'s PC7
//! (`Eligibility::PriorChainSplit`) path against a `STATE.md`-shaped
//! fixture carrying an inline `[Prior: ...]` chain. `tests/
//! bc_10_13_001_pc1_changelog_bootstrap_test.rs`'s EC-006 coverage only
//! exercises the `CurrentEntryOnly` path via `ensure_changelog_field`
//! directly — it never constructs a chained `last_amended` and never calls
//! `migrate_file`. This file closes that gap.
//!
//! # B2 -> B2-R: from "spec-conformant, no bug" to "refuse-by-default"
//!
//! Cycle-1's B2 fix (this file's original form) investigated a pr-reviewer
//! finding that `migrate_file`'s STATE.md branch silently discarded
//! `entries_recovered` chained-entry text and concluded this was
//! spec-conformant per BC-10.13.001 EC-006 ("superseded by its body-level
//! Decisions Log"), not a defect.
//!
//! Cycle-2's fresh-eyes pr-reviewer (B2-R) narrowed that call: the
//! `state-burst` skill and `state-manager` agent prompt carve STATE.md out
//! of their *write-path* discipline, but their *Recovery* sections did not
//! — both claimed unconditionally that `migrate --path <file>` "relocates
//! every chained historical entry into `changelog:`", which is false for
//! STATE.md (the entries are dropped, not relocated), and (b) the very
//! presence of a surviving inline chain is itself evidence the "write the
//! substantive content into the body Decisions Log first" discipline was
//! NOT followed for those entries, so "the content already lives
//! elsewhere" cannot be assumed.
//!
//! The fix implemented here (not just a docs patch) is BC-10.13.001 v1.2's
//! `MigrationOptions::discard_state_chain` opt-in gate: `migrate_file`
//! (and `migrate_all`) now REFUSE — via
//! `Err(MigrateError::StateChainDiscardNotAuthorized)`, no file mutated —
//! to perform a PC7 split on STATE.md unless the caller explicitly
//! authorizes the resulting data loss. Writing the recovered entries into
//! the registered `STATE-amendment-history.md` sidecar instead (the other
//! candidate design) was rejected: BC-10.13.001 PC6 unconditionally forbids
//! this tool from ever writing to any of the 5 frozen sidecars, "including
//! [via] the PC7 full-recovery split path" — implementing that would
//! require amending PC6 itself, out of scope for this fix.
//!
//! This test suite pins BOTH halves of the new behavior end-to-end via
//! `migrate_file`: the refuse-by-default path (no flag) and the
//! explicit-opt-in path (`discard_state_chain: true`), so any future
//! change to either shows up as a test failure requiring a deliberate,
//! reviewed decision — not a silent regression in either direction.

mod common;

use last_amended_migrate::MigrateError;
use last_amended_migrate::eligibility::Eligibility;
use last_amended_migrate::migrate::{MigrationMode, MigrationOptions};
use last_amended_migrate::migrate_file;

/// S-15.03 pr-reviewer B2-R: by DEFAULT (no opt-in), `migrate_file` REFUSES
/// a PC7 split on a `STATE.md` fixture carrying a 2-entry inline
/// `[Prior: ...]` chain — it returns
/// `Err(MigrateError::StateChainDiscardNotAuthorized)`, names the file and
/// the correct entry count in the error, and leaves the file byte-for-byte
/// untouched (no silent drop, no partial write).
#[test]
fn test_BC_10_13_001_EC006_B2R_migrate_file_refuses_state_md_split_without_opt_in() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended = common::chain_last_amended(
        ("2026-09-02", "v9.65", "current burst summary text"),
        &[
            ("2026-09-01", "v9.64", "previous burst summary text"),
            ("2026-08-31", "v9.63", "earlier burst summary text"),
        ],
    );
    let content = common::frontmatter_file(
        "pipeline-state",
        "9.65",
        &last_amended,
        None,
        "# Pipeline State: vsdd-factory\n\n## Decisions Log\n\n\
         (this fixture deliberately does NOT pre-populate the Decisions Log \
         with the chained entries' text, to prove the refusal does not \
         depend on that assumption)\n",
    );
    let path = common::write_file(dir.path(), "STATE.md", &content);
    let before = common::read_file(&path);

    let result = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default());

    match result {
        Err(MigrateError::StateChainDiscardNotAuthorized {
            path: err_path,
            entries,
        }) => {
            assert_eq!(err_path, path, "error must name the refused file");
            assert_eq!(
                entries, 2,
                "error must accurately report how many entries would be discarded"
            );
        }
        other => panic!(
            "expected Err(MigrateError::StateChainDiscardNotAuthorized {{ .. }}), got {other:?}"
        ),
    }

    let after = common::read_file(&path);
    assert_eq!(
        before, after,
        "refusal must leave STATE.md completely untouched — no partial write"
    );

    // `--check` mode must refuse identically — a check run against this
    // file IS the drift signal an operator needs to see, not a case to
    // silently report clean.
    let check_result = migrate_file(&path, MigrationMode::Check, MigrationOptions::default());
    assert!(
        matches!(
            check_result,
            Err(MigrateError::StateChainDiscardNotAuthorized { .. })
        ),
        "MigrationMode::Check must refuse identically to Apply, not report a \
         false-clean result: {check_result:?}"
    );
}

/// S-15.03 pr-reviewer B2-R: WITH explicit opt-in
/// (`MigrationOptions::discard_state_chain = true`), `migrate_file` SPLITS
/// a `STATE.md` chain exactly as B2's original investigation described —
/// `last_amended` becomes current-entry-only, `changelog:` is NEVER added,
/// and the file remains valid — but now reports the dropped count via
/// `entries_discarded` (truthful naming — B2-R's `entries_recovered`
/// rename), not the misleading `entries_recovered` name B2 used.
#[test]
fn test_BC_10_13_001_EC006_B2R_migrate_file_splits_state_md_chain_with_explicit_opt_in() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended = common::chain_last_amended(
        ("2026-09-02", "v9.65", "current burst summary text"),
        &[
            ("2026-09-01", "v9.64", "previous burst summary text"),
            ("2026-08-31", "v9.63", "earlier burst summary text"),
        ],
    );
    let content = common::frontmatter_file(
        "pipeline-state",
        "9.65",
        &last_amended,
        None,
        "# Pipeline State: vsdd-factory\n\n## Decisions Log\n\n\
         (the substantive history for every chained entry already lives \
         here, per the state-burst skill's Write-Path Discipline)\n",
    );
    let path = common::write_file(dir.path(), "STATE.md", &content);
    assert!(!content.contains("changelog:"), "fixture sanity");

    let options = MigrationOptions {
        discard_state_chain: true,
    };
    let report = migrate_file(&path, MigrationMode::Apply, options)
        .expect("migrate_file must SPLIT a STATE.md chain when explicitly authorized");

    assert!(report.mutated, "a PC7 split is a mutation");
    assert_eq!(report.eligibility, Eligibility::PriorChainSplit);
    assert_eq!(
        report.entries_discarded, 2,
        "both chained entries must be counted as DISCARDED (not relocated) \
         — this reports how many redundant chained summaries were dropped \
         from last_amended after explicit operator opt-in, per EC-006 (the \
         substantive content is understood to already live in STATE.md's \
         own body, not relocated by this tool)"
    );
    assert_eq!(
        report.entries_relocated, 0,
        "STATE.md must never report entries_relocated nonzero — it has no \
         changelog: destination to relocate into"
    );

    let after = common::read_file(&path);
    assert!(
        !after.contains(" [Prior:"),
        "last_amended must be current-entry-only after the split — no \
         inline chain marker may remain: {after:?}"
    );
    assert!(
        after.contains("current burst summary text"),
        "the current entry's text must be preserved verbatim: {after:?}"
    );
    assert!(
        !after.contains("changelog:"),
        "STATE.md must NEVER gain a changelog: field, even via a PC7 \
         split (ADR-049 Decision 4 / EC-006): {after:?}"
    );
    assert!(
        after.contains("## Decisions Log"),
        "STATE.md's body content must be left untouched by the split: {after:?}"
    );

    // BC-4.18.001 PC1 (fuel relief): the split's whole purpose is a bounded
    // last_amended — confirm it actually holds for STATE.md too, not just
    // the 4 changelog:-bearing files.
    let doc = last_amended_migrate::frontmatter::parse_frontmatter(&path)
        .expect("parse post-split STATE.md");
    let new_last_amended = doc
        .last_amended_raw
        .expect("STATE.md must have last_amended after the split");
    assert!(
        new_last_amended.len() < 2_000,
        "post-split last_amended must be short — {} bytes is not bounded \
         relief: {new_last_amended:?}",
        new_last_amended.len()
    );

    // Invariant 4 / PC3: the resulting file still parses cleanly under
    // strict YAML `safe_load`.
    let parsed = common::strict_yaml_parse(&after)
        .expect("post-split STATE.md frontmatter must parse under strict YAML safe_load");
    assert!(
        parsed.changelog.is_none(),
        "strict-YAML view must also confirm no changelog: key exists on STATE.md"
    );
}

/// BC-10.13.001 EC-006 / Invariant 3 at STATE.md's own D-1149 calibration
/// scale, WITH explicit opt-in: a mega-line STATE.md chain still splits
/// successfully (not merely refuses), staying consistent with the EC-009
/// mega-line vector already proven for the other 4 files.
#[test]
fn test_BC_10_13_001_EC006_B2R_migrate_file_splits_mega_line_state_md_chain_with_opt_in() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended = common::mega_line_prior_chain(350_000);
    let content = common::frontmatter_file(
        "pipeline-state",
        "9.65",
        &last_amended,
        None,
        "# Pipeline State: vsdd-factory\n\n## Decisions Log\n",
    );
    let path = common::write_file(dir.path(), "STATE.md", &content);
    let before = common::read_file(&path);
    assert!(before.len() > 323_499, "fixture sanity");

    let options = MigrationOptions {
        discard_state_chain: true,
    };
    let report = migrate_file(&path, MigrationMode::Apply, options)
        .expect("mega-line STATE.md chain must SPLIT successfully when authorized");

    assert!(report.mutated);
    assert_eq!(report.eligibility, Eligibility::PriorChainSplit);
    assert_eq!(report.entries_discarded, 1);
    assert_eq!(report.entries_relocated, 0);

    let after = common::read_file(&path);
    assert_ne!(before, after, "the mega-line STATE.md file must be mutated");
    assert!(!after.contains(" [Prior:"));
    assert!(
        !after.contains("changelog:"),
        "STATE.md must never gain changelog: even at mega-line scale: {after:?}"
    );
}

/// BC-10.13.001 EC-006 / Invariant 3 at STATE.md's own D-1149 calibration
/// scale, WITHOUT opt-in: refusal must also hold — and must be cheap — at
/// mega-line scale, not just for small chains (the gate check runs before
/// any escaping/writing work, so refusing is itself bounded-cost).
#[test]
fn test_BC_10_13_001_EC006_B2R_migrate_file_refuses_mega_line_state_md_chain_without_opt_in() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended = common::mega_line_prior_chain(350_000);
    let content = common::frontmatter_file(
        "pipeline-state",
        "9.65",
        &last_amended,
        None,
        "# Pipeline State: vsdd-factory\n\n## Decisions Log\n",
    );
    let path = common::write_file(dir.path(), "STATE.md", &content);
    let before = common::read_file(&path);

    let result = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default());
    match result {
        Err(MigrateError::StateChainDiscardNotAuthorized { entries, .. }) => {
            assert_eq!(entries, 1);
        }
        other => panic!(
            "expected Err(MigrateError::StateChainDiscardNotAuthorized {{ .. }}), got {other:?}"
        ),
    }
    let after = common::read_file(&path);
    assert_eq!(
        before, after,
        "refusal must leave the mega-line file untouched"
    );
}
