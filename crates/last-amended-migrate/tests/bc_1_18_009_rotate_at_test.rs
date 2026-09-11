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

use last_amended_migrate::{MigrationMode, rotate_changelog, rotate_changelog_at};
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

// ---------------------------------------------------------------------------
// Invariant 6 / Obs B (v1.6 hardening) — crash-recovery no-duplicate:
// archive-write-success + source-write-failure → retry → archive ONCE only
// ---------------------------------------------------------------------------

/// Invariant 6, Obs B (v1.6 hardening): `rotate_changelog_at`'s
/// crash-idempotency at the archive boundary.
///
/// Crash-recovery scenario (BC-1.18.009 v1.6 Invariant 6):
///   1. First rotation attempt: archive write succeeds (overflow items appended
///      to archive file), but source rewrite fails AFTER the archive write
///      (`write_atomic`'s rename failure leaves source intact at N=50 items).
///   2. Next dispatch re-fires the item-count trigger (source still at N=50).
///   3. `rotate_changelog_at` self-heals: detects that the overflow `move_items`
///      are ALREADY present at the archive's tail via a byte-level tail-match
///      check (normalized for boundary `\n` on both sides — tail-anchored, never
///      `String::contains` to prevent false positives from recurrent content),
///      skips the archive write, proceeds only with the source rewrite.
///   4. Returns `RotationReport { mutated: true, items_moved: 25 }` (source WAS
///      rewritten even though the archive write was skipped).
///   5. Archive contains the overflow items EXACTLY ONCE — no duplicates.
///
/// # Implementer seam required
///
/// The tail-match self-healing logic described in Invariant 6 is NOT yet
/// implemented — it is part of the `rotate_changelog_at` `todo!()` stub. The
/// implementer MUST add this logic as part of `rotate_changelog_at`'s
/// implementation. The test is written against the INTENDED API; it will pass
/// only when the implementation includes the tail-match deduplication check.
///
/// A naive implementation that simply appends to the archive without checking
/// for existing overflow content would cause this test to FAIL the count
/// assertion (`archive_items_after == 50`, not 25 — the overflow items would
/// appear twice), which is exactly the "duplicate" defect Invariant 6 prevents.
///
/// # Setup methodology
///
/// The test uses the real (non-stub) `rotate_changelog` function to simulate
/// the "first rotation's archive write": calls `rotate_changelog` into a
/// temporary cycle directory to get the exact archive content that would have
/// been written, then copies that content to the REAL archive path, then
/// resets the source back to N=50 items (simulating the source-write failure).
/// This gives the test a precise, fixture-independent representation of the
/// archive content that `rotate_changelog_at` would need to match.
///
/// RED NOW: `rotate_changelog_at` panics at `todo!()` before any self-healing
/// logic is reached. The setup using `rotate_changelog` (real implementation)
/// succeeds.
#[test]
fn test_BC_1_18_009_INV6_crash_recovery_no_duplicate_in_archive_after_retry() {
    let dir = tempfile::tempdir().unwrap();
    let source_path = dir.path().join("BC-INDEX.md");
    let archive_path = dir.path().join("BC-INDEX-changelog-archive.md");

    // --- SETUP: simulate "first rotation archive-write-success + source-write-failure" ---

    // Write a 50-item fixture.
    let fixture_50 = bc_index_fixture(50);
    common::write_file(dir.path(), "BC-INDEX.md", &fixture_50);

    // Use the REAL rotate_changelog (non-stub) to get the exact archive content
    // that would have been written by a first rotation. This produces the same
    // `move_items` concatenation that rotate_changelog_at's implementation must
    // match for the tail-check to work correctly.
    //
    // The cycle-derived path goes into a temp ".factory/cycles/" subdirectory —
    // we copy the content to the real archive path and then discard the temp path.
    let sim_cycle = "crash-sim-setup";
    let first_rotation_result = rotate_changelog(&source_path, sim_cycle, 25, MigrationMode::Apply)
        .expect(
            "Obs B setup: first rotate_changelog must succeed (it uses the real implementation)",
        );

    assert_eq!(
        first_rotation_result.items_moved, 25,
        "Obs B setup: first rotation must move exactly 25 items (50 - 25 keep_recent)"
    );
    assert!(
        first_rotation_result.mutated,
        "Obs B setup: first rotation must mutate the source (used for setup only)"
    );

    // Read the archive content that the first rotation produced.
    let archive_content_after_first_rotation =
        std::fs::read_to_string(&first_rotation_result.archive_path)
            .expect("Obs B setup: read first-rotation archive content");

    // Copy that content to the REAL (test) archive path — this simulates the
    // crash point where the archive write succeeded but the source rewrite failed.
    std::fs::write(&archive_path, &archive_content_after_first_rotation)
        .expect("Obs B setup: write pre-seeded archive (simulates completed archive write)");

    // Reset the source back to N=50 items — this simulates the source-write
    // failure (source is unchanged at its pre-rotation state).
    let fixture_50_retry = bc_index_fixture(50);
    common::write_file(dir.path(), "BC-INDEX.md", &fixture_50_retry);

    // --- PRECONDITION CHECKS ---
    assert_eq!(
        count_items_in_file(&source_path),
        50,
        "Obs B precondition: source must be unchanged at 50 items (simulating source-write \
         failure during first rotation attempt)"
    );
    assert_eq!(
        count_items_in_file(&archive_path),
        25,
        "Obs B precondition: archive must contain the 25 overflow items from the (simulated) \
         first rotation's archive write"
    );

    // --- ACT: the retry dispatch (rotate_changelog_at must self-heal) ---
    let result = rotate_changelog_at(
        &source_path,
        &archive_path,
        25,
        last_amended_migrate::MigrationMode::Apply,
    )
    .expect(
        "Obs B/Inv-6: rotate_changelog_at must succeed on the crash-recovery retry — \
         it detects existing overflow content at archive tail (tail-match), skips the \
         archive write, and completes the source rewrite",
    );

    // --- ASSERTIONS ---

    // Inv-6 / Obs B: `mutated` must be true — the source WAS rewritten even
    // though the archive write was skipped (source mutation is what triggers
    // the `mutated=true` flag, not archive I/O).
    assert!(
        result.mutated,
        "Obs B/Inv-6: RotationReport.mutated must be true after a crash-recovery retry — \
         the source was rewritten (the source IS mutated by the rotation; the archive \
         write was only SKIPPED, not the source rewrite)"
    );

    // Inv-6 / Obs B: items_moved reflects the semantic count (25 items
    // logically moved), regardless of whether the archive write was physical.
    assert_eq!(
        result.items_moved, 25,
        "Obs B/Inv-6: items_moved must still be 25 (the semantic count of logically-moved \
         items), even when the archive write was skipped by the tail-match dedup check"
    );

    // VP-125 / Inv-6 NO-DUPLICATE: the archive must contain the overflow items
    // EXACTLY ONCE — not twice. A naive (non-self-healing) implementation would
    // append again and produce 50 items; this assertion would catch that defect.
    let archive_items_after = count_items_in_file(&archive_path);
    assert_eq!(
        archive_items_after, 25,
        "Obs B/Inv-6: archive must contain exactly 25 items after the crash-recovery retry — \
         NEVER 50 (duplicate append). The tail-match self-healing must detect the existing \
         overflow content and SKIP the archive write. Found {archive_items_after}. \
         NOTE: a count of 50 here means the implementation appended without checking — \
         the implementer MUST add the byte-level tail-match deduplication logic described \
         in BC-1.18.009 v1.6 Invariant 6 (tail-anchored normalized comparison, never \
         String::contains)"
    );

    // VP-125 bounded-live-sequence: source must have been trimmed to keep_recent=25.
    let live_count = count_items_in_file(&source_path);
    assert_eq!(
        live_count, 25,
        "Obs B/Inv-6: source must be trimmed to keep_recent=25 after the crash-recovery \
         retry. Found {live_count}"
    );

    // The archive must not be empty (the pre-seeded content was preserved).
    assert!(
        archive_path.exists(),
        "Obs B/Inv-6: archive file must still exist after the crash-recovery retry"
    );
}
