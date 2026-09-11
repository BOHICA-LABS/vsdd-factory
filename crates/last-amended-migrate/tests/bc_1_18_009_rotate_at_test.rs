// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! BC-1.18.009 library-level coverage for `rotate_changelog_at` — the new
//! explicit-archive-path overload of `rotate_changelog` added to
//! `last-amended-migrate` to support the `BC-INDEX.md` gate in cluster-4.
//!
//! These tests call `rotate_changelog_at` directly (not through the gate) to
//! verify the thin-wrapper contract in isolation from the dispatcher layer.
//!
//! # BC-5.38.001 Red Gate discipline — RED
//!
//! `rotate_changelog_at` is a `todo!()` stub. Both tests below will panic at
//! the `todo!()` call site before any assertion is reached. This is the
//! expected Red Gate state.
//!
//! # What these tests verify
//!
//! * `test_BC_1_18_009_rotate_changelog_at_uses_explicit_archive_path`:
//!   confirms that `rotate_changelog_at(path, archive_path, ...)` appends to
//!   the CALLER-SUPPLIED `archive_path` — NOT to a cycle-derived path. The
//!   distinction matters because `BC-INDEX.md` is a catalog artifact that
//!   lives outside any cycle directory, so there is no `cycle_name` to derive
//!   from. The caller (the B1 gate in `shard_manager.rs`) pre-computes
//!   `target.parent() / "BC-INDEX-changelog-archive.md"` and hands it to
//!   `rotate_changelog_at` verbatim.
//!
//! * `test_BC_1_18_009_AC016_VP125_single_evergreen_archive_accumulates`:
//!   confirms BC-1.18.009 Postcondition 5 (single evergreen archive) at the
//!   library primitive level — two successive `rotate_changelog_at` calls on
//!   the same `path` with the same `archive_path` produce a cumulative archive
//!   whose first rotation's content is preserved verbatim at the front,
//!   followed by the second rotation's appended items.

use last_amended_migrate::{MigrationMode, rotate_changelog_at};
use std::path::Path;

mod common;

/// Build a well-formed `changelog:` frontmatter file usable with
/// `parse_frontmatter`. Items are in newest-first order (matching
/// `rotate_changelog`'s contract: `keep_recent` retains the FIRST `n` items
/// in the array, which are the newest).
///
/// Each item uses the two-key YAML mapping:
/// ```yaml
///   - date: 2026-MM-DD
///     change: "item-N"
/// ```
fn bc_index_fixture(n_items: usize) -> String {
    let items: Vec<String> = (0..n_items)
        .map(|i| {
            let item_num = n_items - i; // newest-first: highest number first
            let day = (item_num % 28) + 1;
            let month = (item_num % 12) + 1;
            common::changelog_item_block(
                &format!("2026-{month:02}-{day:02}"),
                &format!("item-{item_num}"),
            )
        })
        .collect();
    common::frontmatter_file(
        "behavioral-contract-index",
        "1.0",
        "2026-09-01 (v1.0) — test fixture",
        Some(&items),
        "# BC-INDEX Test Fixture\n",
    )
}

/// Count the `changelog:` sequence items in a file by counting `  - ` lines.
///
/// Two-branch dispatch:
/// * If the file starts with `---\n` (a frontmatter-fenced source file such as
///   `BC-INDEX.md`): count `  - ` lines **inside** the frontmatter block only.
/// * Otherwise (a fence-less archive file such as
///   `BC-INDEX-changelog-archive.md`): count `  - ` lines across the **whole**
///   file. Archive files contain only raw YAML sequence items appended by
///   `rotate_changelog_at` (no frontmatter, no false-positive `  - ` lines),
///   so counting globally gives a correct item total.
fn count_items_in_file(path: &Path) -> usize {
    let content = std::fs::read_to_string(path).expect("count_items_in_file: read file");
    if let Some(after_open) = content.strip_prefix("---\n") {
        // Frontmatter-fenced source file: count `  - ` lines inside the
        // frontmatter block only.
        let end = after_open
            .find("\n---")
            .expect("frontmatter-fenced file must have closing frontmatter fence");
        after_open[..end]
            .lines()
            .filter(|l| l.starts_with("  - "))
            .count()
    } else {
        // Fence-less archive file: count all `  - ` lines across the whole file.
        content.lines().filter(|l| l.starts_with("  - ")).count()
    }
}

// ---------------------------------------------------------------------------
// CTV #1 (library) — explicit archive path, not cycle-derived
// ---------------------------------------------------------------------------

/// BC-1.18.009: `rotate_changelog_at` uses the EXPLICIT `archive_path`
/// argument — never a path derived from `path.parent() / cycle_name / ...`
/// (the `rotate_changelog` shape). The test supplies an archive path that does
/// NOT follow any cycle-directory naming convention (plain `"my-archive.md"`)
/// and verifies the rotation result's `archive_path` field is the provided
/// path, not a synthesized one.
///
/// This guards against an accidental reimplementation that ignores the
/// `archive_path` parameter and falls through to `resolve_archive_path`
/// instead (BC-1.18.009 Architecture Anchor: "the dispatcher's B1 handler
/// pre-computes the FIXED, non-cycle, BC-INDEX-sibling path").
///
/// RED NOW: `todo!()` stub panics.
#[test]
fn test_BC_1_18_009_rotate_changelog_at_uses_explicit_archive_path_not_cycle_derived() {
    let dir = tempfile::tempdir().unwrap();
    let source_path = dir.path().join("BC-INDEX.md");
    let explicit_archive = dir.path().join("my-custom-archive.md");

    // Write a 30-item fixture: rotate with keep_recent=20 → should move 10
    // oldest items to `explicit_archive`.
    let fixture = bc_index_fixture(30);
    common::write_file(dir.path(), "BC-INDEX.md", &fixture);

    let result = rotate_changelog_at(
        &source_path,
        &explicit_archive,
        20,
        MigrationMode::Apply,
    )
    .expect("rotate_changelog_at must succeed for a well-formed 30-item fixture with keep_recent=20");

    // Primary assertion: the returned `archive_path` is the EXPLICIT caller
    // path, not a cycle-directory-derived path.
    assert_eq!(
        result.archive_path,
        explicit_archive,
        "BC-1.18.009: rotate_changelog_at must use the EXPLICIT archive_path supplied by \
         the caller, NOT any path derived from a cycle_name or the source file's directory \
         structure. Expected: {}, got: {}",
        explicit_archive.display(),
        result.archive_path.display()
    );

    // Archive file must exist at the explicit path.
    assert!(
        explicit_archive.exists(),
        "BC-1.18.009: the archive file must exist at the explicitly-supplied path after \
         rotation: {}",
        explicit_archive.display()
    );

    // NO cycle-derived archive path must have been created.
    // (A misbehaving implementation might call `resolve_archive_path` and
    // create `.factory/cycles/<cycle>/BC-INDEX-changelog-archive.md`.)
    let cycle_derived = dir.path().join(".factory");
    assert!(
        !cycle_derived.exists(),
        "BC-1.18.009: rotate_changelog_at must NOT create a cycle-derived archive path \
         (.factory/) — the caller supplies the archive path explicitly. Found: {}",
        cycle_derived.display()
    );

    // Verify rotation correctness: 10 items moved, 20 retained.
    assert_eq!(
        result.items_moved, 10,
        "BC-1.18.009: rotate_changelog_at must move exactly 10 items \
         (30 total - 20 keep_recent) to the archive"
    );
    assert!(
        result.mutated,
        "BC-1.18.009: result.mutated must be true when items were actually moved"
    );

    let live_count = count_items_in_file(&source_path);
    assert_eq!(
        live_count, 20,
        "BC-1.18.009: live sequence must contain exactly keep_recent=20 items after rotation. \
         Found {live_count}"
    );

    let archive_count = count_items_in_file(&explicit_archive);
    assert_eq!(
        archive_count, 10,
        "BC-1.18.009: archive must contain exactly 10 moved items. Found {archive_count}"
    );
}

// ---------------------------------------------------------------------------
// AC-016 / VP-125 — Single evergreen archive accumulates (library level)
// ---------------------------------------------------------------------------

/// AC-016, VP-125 (no-history-loss facet): two successive `rotate_changelog_at`
/// calls with the same `archive_path` produce a cumulative archive — the second
/// rotation's items are APPENDED to the first rotation's items, never
/// overwriting or truncating them (BC-1.18.009 Postcondition 5).
///
/// This is the library-level complement to the integration test
/// `test_BC_1_18_009_AC016_VP125_single_evergreen_archive_no_history_loss` in
/// `bc_1_18_009_b1_rotate_test.rs`. Both must pass for VP-125 to be fully
/// covered.
///
/// RED NOW: first `rotate_changelog_at` call panics at `todo!()`.
#[test]
fn test_BC_1_18_009_AC016_VP125_single_evergreen_archive_accumulates_across_rotations() {
    let dir = tempfile::tempdir().unwrap();
    let source_path = dir.path().join("BC-INDEX.md");
    let archive_path = dir.path().join("BC-INDEX-changelog-archive.md");

    // First rotation: 50 items → keep 25, archive 25.
    let fixture_1 = bc_index_fixture(50);
    common::write_file(dir.path(), "BC-INDEX.md", &fixture_1);

    let result_1 = rotate_changelog_at(&source_path, &archive_path, 25, MigrationMode::Apply)
        .expect("rotate_changelog_at must succeed for a 50-item fixture with keep_recent=25");

    assert_eq!(
        result_1.items_moved, 25,
        "VP-125 first rotation: must move exactly 25 items (50 - 25 keep_recent)"
    );

    let archive_content_1 =
        std::fs::read_to_string(&archive_path).expect("VP-125: read archive after first rotation");
    let archive_items_1 = count_items_in_file(&archive_path);
    assert_eq!(
        archive_items_1, 25,
        "VP-125 first rotation: archive must contain exactly 25 items. Found {archive_items_1}"
    );

    // Second rotation: reset the live source to 50 items (simulates
    // low_water_mark+25 agent prepends arriving between rotations).
    let fixture_2 = bc_index_fixture(50);
    common::write_file(dir.path(), "BC-INDEX.md", &fixture_2);

    let result_2 = rotate_changelog_at(&source_path, &archive_path, 25, MigrationMode::Apply)
        .expect(
            "rotate_changelog_at must succeed for the second 50-item fixture with keep_recent=25",
        );

    assert_eq!(
        result_2.items_moved, 25,
        "VP-125 second rotation: must also move exactly 25 items"
    );

    // VP-125 core assertion: the archive's first-rotation content must be
    // present at the START of the archive after the second rotation —
    // APPEND-only, never overwrite.
    let archive_content_2 =
        std::fs::read_to_string(&archive_path).expect("VP-125: read archive after second rotation");

    assert!(
        archive_content_2.starts_with(&archive_content_1),
        "AC-016/VP-125/PC5: the second rotation's archive write must APPEND to the existing \
         archive — the first rotation's content must be verbatim at the front of the archive \
         (no overwrite, no truncation, no deduplication). \
         archive-after-rotation-1 bytes={}, archive-after-rotation-2 bytes={}. \
         First-rotation content was NOT found at start of second-rotation archive.",
        archive_content_1.len(),
        archive_content_2.len()
    );

    let archive_items_2 = count_items_in_file(&archive_path);
    assert_eq!(
        archive_items_2, 50,
        "AC-016/VP-125: archive must contain exactly 50 items (25 from each of two rotations) \
         — no history lost, no items deduplicated. Found {archive_items_2}"
    );

    // Additional invariant: the live source must still have 25 items after the
    // second rotation (VP-125 bounded-live-sequence facet).
    let live_count = count_items_in_file(&source_path);
    assert_eq!(
        live_count, 25,
        "VP-125: live sequence must be trimmed to keep_recent=25 by the second rotation. \
         Found {live_count}"
    );
}
