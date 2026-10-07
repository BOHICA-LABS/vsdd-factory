// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! BC-4.18.001 PC1 (`last_amended` byte length is bounded regardless of
//! cumulative burst count) — the testable crate-level PROXY for the
//! fuel-budget-relief regression.
//!
//! # Scope note — what this file does and does NOT cover
//!
//! BC-4.18.001's PC2/PC3 (a synthetic-burst regression run completing
//! within the `legacy-bash-adapter`-hosted WASM validators' fuel budget,
//! and the differential pre-fix/post-fix fuel-exhaustion comparison) require
//! invoking the REAL `legacy-bash-adapter` dispatcher/WASM-plugin
//! infrastructure (`crates/hook-plugins/legacy-bash-adapter/`,
//! `crates/factory-dispatcher/src/invoke.rs`'s `DEFAULT_FUEL_CAP`/fuel
//! accounting) against `plugins/vsdd-factory/hooks-registry.toml`-registered
//! validators. `last-amended-migrate` has NO dependency on that
//! infrastructure (its own `Cargo.toml` doc comment: "NOT a WASM hook
//! plugin ... has no dependency on `vsdd-hook-sdk`"), and BC-4.18.001's own
//! VP candidate row for PC2 explicitly names that surface: "integration:
//! bats/Rust-workspace test invoking the real `legacy-bash-adapter`-hosted
//! validator(s)". That test belongs in
//! `crates/hook-plugins/legacy-bash-adapter/` or a bats suite under
//! `plugins/vsdd-factory/tests/`, dispatched by fuel-accounting
//! infrastructure this crate cannot and should not reach into — writing it
//! here would mean reimplementing (or worse, mocking) the fuel-accounting
//! logic the real regression test must exercise for real, which is exactly
//! the "vacuously true" Red Gate anti-pattern this agent's operating rules
//! forbid. See this story's final report for the explicit routing note.
//!
//! What IS in this crate's scope, and what this file covers: PC1's own
//! structural claim — "the `last_amended` field's own byte length never
//! exceeds a fixed per-entry ceiling — it is always exactly one dated
//! entry, independent of N" — verified against this tool's REAL
//! `migrate_all`/`migrate_file` output on realistic ADR-049-shaped
//! fixtures, including the 3 escape-defect files PC3 rewrites in place.
//!
//! # S-15.03 B3 — what the ORIGINAL two tests below do and do NOT prove
//!
//! `test_BC_4_18_001_PC1_last_amended_byte_length_bounded_across_all_five_files`
//! and `test_BC_4_18_001_EC003_quote_defect_entry_still_bounded_after_escape_fix`
//! start every fixture from an ALREADY-SHORT `last_amended` (a single
//! current-entry-only value, with or without a D-1144 quote defect — never
//! an inline `[Prior: ...]` chain). An assertion that the OUTPUT stays
//! under 2,000 bytes when the INPUT was already under 2,000 bytes is true
//! even for a no-op `migrate_all` — it is a valid EC-001/EC-003 no-op/
//! escape-fix control, but it is NOT, on its own, a fuel-RELIEF regression
//! guard (relief is a property of shrinking a LARGE input, which these two
//! fixtures never construct). They are retained below for their genuine
//! EC-001/EC-003 coverage; the actual fuel-relief regression guard is
//! `test_BC_4_18_001_B3_realistic_multi_entry_chain_relief_is_per_line_bounded`,
//! which starts from a REALISTIC ~300K-char chain of 100 modest-sized
//! entries (real D-1149 incident scale — NOT `bc_10_13_001_pc2_eligibility_
//! test.rs`'s `EC009_mega_line` fixture, which is a single monolithic
//! ~350K-char entry proving a DIFFERENT, still-valid claim — PC7's
//! bounded-scan linearity, Invariant 3 — since splitting one atomic
//! un-splittable entry cannot itself demonstrate per-line relief) and
//! asserts both halves of the real claim: (i) `last_amended` shrinks, AND
//! (ii) no single physical line in the rewritten file is anywhere near
//! mega-line scale (the assertion that would actually catch a "moved the
//! problem, didn't fix it" regression, e.g. concatenating all N entries
//! onto one `changelog:` item instead of relocating each into its own).

mod common;

use last_amended_migrate::frontmatter::parse_frontmatter;
use last_amended_migrate::migrate::{
    MigrationMode, MigrationOptions, TARGET_FILES, migrate_all, migrate_file,
};

/// A generous ceiling for a single dated `last_amended` entry — real corpus
/// entries in `.factory/` run from a few dozen to a few hundred bytes; 2,000
/// bytes is comfortably above any legitimate single entry and orders of
/// magnitude below the pre-fix 323,499-char mega-line this BC exists to
/// eliminate.
const LAST_AMENDED_BYTE_CEILING: usize = 2_000;

/// Build the full 5-file `.factory`-rooted fixture tree matching
/// `TARGET_FILES`'s real relative paths, mixing all 3 PC1/PC3 shapes this
/// story's fixtures need to exercise:
///
/// - `STORY-INDEX.md` — clean entry, NO `changelog:` yet (PC1/EC-002).
/// - `BC-INDEX.md` / `ARCH-INDEX.md` / `STATE.md` — D-1144 escape-defective
///   entry (PC3's 3-file target set).
/// - `VP-INDEX.md` — already fully compliant (EC-001 no-op control).
fn build_five_target_fixtures(tmp: &std::path::Path) -> std::path::PathBuf {
    let factory_root = tmp.join(".factory");

    let story_index_last_amended =
        common::clean_current_entry("2026-09-02", "v4.430", "clean bootstrap entry");
    common::write_file(
        &factory_root,
        "stories/STORY-INDEX.md",
        &common::frontmatter_file(
            "story-index",
            "4.430",
            &story_index_last_amended,
            None,
            "# Fixture STORY-INDEX\n",
        ),
    );

    for (rel, doc_type, version) in [
        (
            "specs/behavioral-contracts/BC-INDEX.md",
            "behavioral-contract-index",
            "5.41",
        ),
        (
            "specs/architecture/ARCH-INDEX.md",
            "architecture-index",
            "4.11",
        ),
    ] {
        let last_amended = common::quote_defect_current_entry("2026-09-02", version);
        common::write_file(
            &factory_root,
            rel,
            &common::frontmatter_file(
                doc_type,
                version,
                &last_amended,
                Some(&[common::changelog_item_block("2026-08-01", "an older entry")]),
                "# Fixture\n",
            ),
        );
    }

    let vp_index_last_amended =
        common::clean_current_entry("2026-09-02", "v2.99", "already compliant");
    common::write_file(
        &factory_root,
        "specs/verification-properties/VP-INDEX.md",
        &common::frontmatter_file(
            "verification-property-index",
            "2.99",
            &vp_index_last_amended,
            Some(&[common::changelog_item_block("2026-08-01", "an older entry")]),
            "# Fixture VP-INDEX\n",
        ),
    );

    let state_last_amended = common::quote_defect_current_entry("2026-09-02", "v9.65");
    common::write_file(
        &factory_root,
        "STATE.md",
        &common::frontmatter_file(
            "pipeline-state",
            "9.65",
            &state_last_amended,
            None,
            "# Fixture STATE\n",
        ),
    );

    factory_root
}

/// BC-4.18.001 PC1: after `migrate_all` runs across all 5 fixtures
/// (bootstrapping `changelog:` on one, escape-fixing 3, no-op on the 5th),
/// every file's `last_amended` byte length stays under the fixed ceiling —
/// the structural proxy for "cannot re-form a mega-line".
#[test]
fn test_BC_4_18_001_PC1_last_amended_byte_length_bounded_across_all_five_files() {
    let dir = tempfile::tempdir().expect("tempdir");
    let factory_root = build_five_target_fixtures(dir.path());

    let report = migrate_all(
        &factory_root,
        MigrationMode::Apply,
        MigrationOptions::default(),
    )
    .expect("migrate_all must succeed");
    assert_eq!(report.files.len(), TARGET_FILES.len());

    for rel in TARGET_FILES {
        let path = factory_root.join(rel);
        let doc = parse_frontmatter(&path)
            .unwrap_or_else(|e| panic!("parse_frontmatter on {path:?} failed: {e}"));
        let last_amended = doc
            .last_amended_raw
            .unwrap_or_else(|| panic!("{path:?} must have a last_amended value after migration"));
        assert!(
            last_amended.len() < LAST_AMENDED_BYTE_CEILING,
            "BC-4.18.001 PC1: {path:?}'s last_amended is {} bytes — must stay \
             far below the {LAST_AMENDED_BYTE_CEILING}-byte ceiling (proxy \
             for 'cannot re-form the pre-fix 323,499-char mega-line')",
            last_amended.len()
        );
    }
}

/// BC-4.18.001 EC-003: a synthetic burst whose entry includes an embedded
/// double-quote (D-1144 class) still stays within the same bound — the
/// fuel-relief property is orthogonal to the YAML-escape correctness
/// property (already covered by BC-10.13.001 PC3's own test file); this
/// test only confirms escaping doesn't materially change payload size.
#[test]
fn test_BC_4_18_001_EC003_quote_defect_entry_still_bounded_after_escape_fix() {
    let dir = tempfile::tempdir().expect("tempdir");
    let factory_root = build_five_target_fixtures(dir.path());
    migrate_all(
        &factory_root,
        MigrationMode::Apply,
        MigrationOptions::default(),
    )
    .expect("migrate_all must succeed");

    let bc_index_path = factory_root.join("specs/behavioral-contracts/BC-INDEX.md");
    let doc = parse_frontmatter(&bc_index_path).expect("parse post-migration BC-INDEX.md");
    let last_amended = doc
        .last_amended_raw
        .expect("BC-INDEX.md must have last_amended after migration");

    assert!(
        last_amended.len() < LAST_AMENDED_BYTE_CEILING,
        "escaping 2 quotes must not materially change payload size: {} bytes",
        last_amended.len()
    );
    assert!(
        last_amended.contains("quoted term"),
        "escape fix must preserve the entry's substantive text: {last_amended:?}"
    );
}

/// Precondition 1 / EC-006 at the `migrate_all` aggregation level: exactly
/// 4 of the 5 files may ever carry `changelog:` — `STATE.md` never does,
/// even after a full migration run across all 5.
#[test]
fn test_BC_10_13_001_EC006_migrate_all_state_md_never_gains_changelog() {
    let dir = tempfile::tempdir().expect("tempdir");
    let factory_root = build_five_target_fixtures(dir.path());
    migrate_all(
        &factory_root,
        MigrationMode::Apply,
        MigrationOptions::default(),
    )
    .expect("migrate_all must succeed");

    let state_content = common::read_file(&factory_root.join("STATE.md"));
    assert!(
        !state_content.contains("changelog:"),
        "STATE.md must NEVER gain a changelog: field, even via migrate_all \
         (ADR-049 Decision 4 / EC-006): {state_content:?}"
    );
}

/// S-15.03 B3 — the ACTUAL fuel-relief regression guard.
///
/// Starts from a REALISTIC `last_amended` chain of 100 modest-sized
/// historical entries (~3,000 chars of prose each, summing to ~300K total
/// chain chars — real D-1149 incident aggregate scale: dozens-to-hundreds
/// of entries accumulated over many bursts, NOT one giant blob) and proves
/// BOTH halves of BC-4.18.001 PC1's fuel-relief claim after the PC7 split:
///
/// 1. The resulting `last_amended` is short (< 2,000 bytes).
/// 2. No single physical line in the rewritten file is anywhere near
///    mega-line scale — each of the 100 recovered entries lands on its own
///    bounded `changelog:` item line, rather than all 100 being
///    concatenated onto one line (which would "move the problem" into
///    `changelog:` rather than actually relieving it, and would NOT be
///    caught by asserting `last_amended`'s length alone).
#[test]
fn test_BC_4_18_001_B3_realistic_multi_entry_chain_relief_is_per_line_bounded() {
    let dir = tempfile::tempdir().expect("tempdir");
    let factory_root = dir.path().join(".factory");

    let last_amended = common::realistic_multi_entry_prior_chain(100, 3_000);
    assert!(
        last_amended.len() > 300_000,
        "fixture sanity: must reach real D-1149 aggregate incident scale \
         (100 modest entries, not one monolithic blob): {} bytes",
        last_amended.len()
    );

    let content = common::frontmatter_file(
        "story-index",
        "4.430",
        &last_amended,
        None,
        "# Fixture STORY-INDEX\n",
    );
    let path = common::write_file(&factory_root, "stories/STORY-INDEX.md", &content);

    let report = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default())
        .expect("migrate_file must split a realistic multi-entry chain");
    assert!(report.mutated, "a PC7 split is a mutation");
    assert_eq!(
        report.entries_relocated, 100,
        "all 100 chained entries must be recovered"
    );

    let doc = parse_frontmatter(&path).expect("parse post-split STORY-INDEX.md");
    let new_last_amended = doc
        .last_amended_raw
        .expect("STORY-INDEX.md must have last_amended after the split");
    assert!(
        new_last_amended.len() < LAST_AMENDED_BYTE_CEILING,
        "BC-4.18.001 PC1 half (i): last_amended must be short after a \
         realistic multi-entry split — {} bytes",
        new_last_amended.len()
    );

    let after = common::read_file(&path);

    // A generous per-line ceiling: each entry's rendered prose is ~3,000
    // chars; its own `    change: "..."` line (plus quoting/date/version
    // overhead) stays comfortably under 4,000 chars. A line anywhere near
    // the ~300K aggregate scale here would mean the split concatenated
    // entries onto a single line instead of relocating each into its own
    // changelog: item — exactly the "moved the problem, didn't fix it"
    // regression this test exists to catch, which the byte-length-of-
    // last_amended assertion alone cannot detect.
    const PER_LINE_CEILING: usize = 4_000;
    let longest_line = after.lines().map(str::len).max().unwrap_or(0);
    assert!(
        longest_line < PER_LINE_CEILING,
        "BC-4.18.001 PC1 half (ii): the longest single physical line in \
         the rewritten file is {longest_line} bytes — must stay far below \
         the {PER_LINE_CEILING}-byte per-entry ceiling; a line anywhere \
         near the original ~300K aggregate scale would mean entries were \
         concatenated rather than split into separate changelog: items"
    );

    assert_eq!(
        after.matches("  - date:").count(),
        100,
        "all 100 entries must be relocated as 100 SEPARATE changelog: \
         items, not concatenated into fewer/one item(s)"
    );

    let parsed = common::strict_yaml_parse(&after)
        .expect("post-split frontmatter must parse under strict YAML safe_load");
    assert_eq!(
        parsed.changelog.expect("changelog: must be present").len(),
        100,
        "strict-YAML view must also confirm exactly 100 separate items"
    );
}
