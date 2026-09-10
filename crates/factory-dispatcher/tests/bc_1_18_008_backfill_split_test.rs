// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! BC-1.18.008 (S-25.02 F4 BC-cluster 3 "retention+backfill") coverage for
//! the mechanism-A one-time backfill-split functions in `shard_manager.rs`
//! (AC-013/AC-014).
//!
//! # Coverage map
//!
//! Every non-trivial function this file drives
//! (`mechanism_a_record_boundary_offsets`, `mechanism_a_partition_for_backfill`,
//! `mechanism_a_verify_backfill_content_preserved`,
//! `mechanism_a_verify_backfill_record_counts_preserved`,
//! `mechanism_a_backfill_already_migrated`, `run_mechanism_a_backfill_split`)
//! is a real, fully implemented body (no `todo!()` stubs). This file's tests
//! are organized by the BC-1.18.008 clause each one pins, with the
//! originating fresh-context adversarial finding (BLOCKER-1, HIGH-2, MED-3,
//! F-001, F-002, F-004, P2-001, P2-002, P2-003, ...) named in each test's own
//! doc comment as historical provenance, never as a claim about that test's
//! current pass/fail status — a test's own `cargo test` result is the only
//! authoritative source for that, and prose asserting "EXPECTED TO FAIL
//! until X" or "now PASSES" goes stale the moment the referenced fix lands.
//! Two goals every test in this file, and every future addition, must serve
//! without amendment across that transition:
//!
//! - **What BC clause does this test pin?** (a Postcondition, Invariant,
//!   Edge Case, or Canonical Test Vector row — cited by ID).
//! - **What real-world shape motivated it?** (the finding that surfaced the
//!   gap, described as historical narrative — "a fresh-context adversarial
//!   pass found ..." — not as a live status flag).
//!
//! # Content-preservation hard-gate coverage note (AC-014, Postcondition 6)
//!
//! `run_mechanism_a_backfill_split`'s own implementation computes its
//! partitions internally (via `mechanism_a_partition_for_backfill`, whose
//! output always structurally satisfies content-preservation by
//! construction when the `record_boundary_offsets` argument itself
//! genuinely matches the content's real record structure) — the two
//! `mechanism_a_verify_backfill_*` predicates that make up that hard gate
//! are covered directly at the unit level below (both their `true` and
//! `false` outcomes), which is the correct-grained place to test a boolean
//! predicate's own logic. The prior claim that "there is no external
//! caller-visible way to inject a genuine content/record-count MISMATCH …
//! from outside the module" is STALE and, per the F-001 finding above, was
//! itself the bug: a caller CAN supply a `record_boundary_offsets` argument
//! that is well-formed (strictly ascending, in-bounds) but does not match
//! the actual content's real record boundaries (e.g. missing a genuine
//! boundary) — see `test_BC_1_18_008_F001_*`'s own doc comment for the full
//! mechanism. `run_mechanism_a_backfill_split`'s remaining tests cover its
//! ORCHESTRATION of a successful (gate-passing) run, plus
//! EC-016/EC-017/idempotency/crash-atomicity.

use factory_dispatcher::shard_manager::{
    MechanismABackfillError, MechanismABackfillOutcome, MechanismABackfillPartition, ShardEntry,
    ShardIndex, ShardIndexEntry, ShardShape, mechanism_a_backfill_already_migrated,
    mechanism_a_partition_for_backfill, mechanism_a_record_boundary_offsets,
    mechanism_a_verify_backfill_content_preserved,
    mechanism_a_verify_backfill_record_counts_preserved, run_mechanism_a_backfill_split,
};

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

/// A well-formed `"flat"`-shaped `[[shard]]` config entry for `decision-log`,
/// with a caller-supplied `shard_cap_bytes` so each test can pick a cap that
/// deterministically produces the partition count it wants to exercise.
fn flat_entry(artifact_stem: &str, shard_cap_bytes: u64) -> ShardEntry {
    ShardEntry {
        artifact_stem: artifact_stem.to_string(),
        artifact_path: format!("{artifact_stem}.md"),
        practical_fuel_ceiling: 8_000_000,
        worst_case_fuel_per_byte: 106.36,
        max_single_record_bytes: 16_384,
        safety_margin: 8_192,
        shard_cap_bytes,
        shape: Some(ShardShape::Flat),
        n: None,
        low_water_mark: None,
    }
}

/// One fixed-length (30-byte), digit-tagged synthetic "record" — used to
/// build multi-record content whose record boundaries are trivially known
/// (`i * 30`) without depending on this module's own (separately tested)
/// `mechanism_a_record_boundary_offsets` boundary-detection logic.
fn synthetic_record(tag: u8) -> Vec<u8> {
    vec![b'0' + tag; 30]
}

fn concat_records(tags: &[u8]) -> Vec<u8> {
    tags.iter().flat_map(|&t| synthetic_record(t)).collect()
}

// ---------------------------------------------------------------------------
// AC-013 — `mechanism_a_record_boundary_offsets`
// (BC-1.18.008 Postcondition 2, Invariant 2)
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_008_PC2_record_boundary_offsets_decision_log_finds_row_starts() {
    let row1 = "| D-001 | decided one | author |\n";
    let row2 = "| D-002 | decided two | author |\n";
    let row3 = "| D-003 | decided three | author |\n";
    let content = format!("# decision-log\n\n## Decisions Log\n\n{row1}{row2}{row3}");

    let offset1 = content.find(row1).unwrap();
    let offset2 = content.find(row2).unwrap();
    let offset3 = content.find(row3).unwrap();

    let offsets = mechanism_a_record_boundary_offsets("decision-log", content.as_bytes());

    assert_eq!(
        offsets,
        vec![offset1, offset2, offset3],
        "PC2: decision-log.md's structural record boundaries are its own `## Decisions Log` \
         table row starts (D-NNN rows) — never an arbitrary byte offset"
    );
}

#[test]
fn test_BC_1_18_008_PC2_MT_record_boundary_offsets_burst_log_grounded_h2_h3_exception_and_nested_block()
 {
    // Re-grounded per BC-1.18.008 v1.2's Record-Boundary Marker Table
    // (2026-09-10 amendment) and its EC-006 canonical test vector. This
    // fixture REPLACES the prior fabricated `### Burst 1`/`### Burst 2`
    // fixture (which used an h3-only form that isn't any real burst-log.md
    // record shape and certified detection at the WRONG marker level — the
    // fresh-context adversarial finding this re-grounding fixes: an h2-only
    // OR h3-only detector either silently no-ops on real h2-keyed content
    // or mis-treats a nested `### Block N:` sub-heading as a false
    // boundary).
    //
    // Real, verbatim heading forms: the two h2 records + two confirmed
    // `### Pass-N Fix Burst` h3-exception records are lifted from
    // `.factory/cycles/v1.0-feature-engine-discipline-pass-1/burst-log.md`
    // (lines 1951, 2033, 2099, 2163); the nested `### Block N:`
    // sub-headings are lifted from `.factory/cycles/v1.0-brownfield-
    // backfill/burst-log.md`'s own 8-block burst structure (e.g. line
    // 151's `### Block 8: factory-artifacts commit`).
    let h2_pass38 = "## F5 pass-38 fix burst\n";
    let h3_pass39 = "### Pass-39 Fix Burst — F5 Engine Discipline (2026-05-12)\n";
    let block3_decoy = "### Block 3: Codifications\n";
    let h3_pass40 = "### Pass-40 Fix Burst — F5 Engine Discipline (2026-05-12)\n";
    let h2_pass41 = "## Burst: F5 pass-41 fix burst (2026-05-12)\n";
    let block8_decoy = "### Block 8: factory-artifacts commit\n";

    let content = format!(
        "# burst-log\n\n\
         {h2_pass38}\
         Body of the pass-38 fix burst (elided).\n\n\
         {h3_pass39}\
         Body of the pass-39 fix burst (elided).\n\n\
         {block3_decoy}\
         Decision D-NNN codified here — nested sub-structure, NOT its own record.\n\n\
         {h3_pass40}\
         Body of the pass-40 fix burst (elided).\n\n\
         {h2_pass41}\
         Body of the pass-41 fix burst (elided).\n\n\
         {block8_decoy}\
         Committed to factory-artifacts — nested sub-structure, NOT its own record.\n"
    );

    let offset_pass38 = content.find(h2_pass38).unwrap();
    let offset_pass39 = content.find(h3_pass39).unwrap();
    let offset_pass40 = content.find(h3_pass40).unwrap();
    let offset_pass41 = content.find(h2_pass41).unwrap();
    let offset_block3 = content.find(block3_decoy).unwrap();
    let offset_block8 = content.find(block8_decoy).unwrap();

    let offsets = mechanism_a_record_boundary_offsets("burst-log", content.as_bytes());

    assert_eq!(
        offsets,
        vec![offset_pass38, offset_pass39, offset_pass40, offset_pass41],
        "PC2 Record-Boundary Marker Table / EC-006 canonical test vector: burst-log.md's real \
         record boundaries are its `## ` (h2) records PLUS the two confirmed `### Pass-N Fix \
         Burst` h3-exception records — exactly 4 boundaries here (pass-38, pass-39, pass-40, \
         pass-41), never keyed on heading level alone. Got: {offsets:?}"
    );
    assert!(
        !offsets.contains(&offset_block3) && !offsets.contains(&offset_block8),
        "PC2/Invariant 2: nested `### Block N:` sub-headings must be excluded from the boundary \
         set even though they share the `### ` prefix with the genuine h3-exception records — \
         mistaking them for boundaries would split a record mid-record. Got: {offsets:?}"
    );
}

#[test]
fn test_BC_1_18_008_PC2_MT_record_boundary_offsets_lessons_grounded_h2_h3_heterogeneous_forms() {
    // Re-grounded per BC-1.18.008 v1.2's Record-Boundary Marker Table
    // (2026-09-10 amendment). The prior MED-3 lessons test (below) only
    // ever exercised the pre-052 h3-exception form; it never exercised the
    // h2 primary form that covers the MAJORITY of real lesson records in
    // both cycles, so it could not have caught an h3-only detector missing
    // that content entirely — exactly the false-green gap the fresh-context
    // adversarial pass found.
    //
    // Real, verbatim heading forms: the pre-`L-EDP1-052` h3-exception
    // records `### L-EDP1-050 ...` / `### L-EDP1-051 ...` (lines 24, 123 of
    // `.factory/cycles/v1.0-feature-engine-discipline-pass-1/lessons.md`),
    // the h2 form adopted starting at `## L-EDP1-052 ...` (line 210 of the
    // same file), and brownfield's own h2 forms `## LESSON (D-NNNN) ...` /
    // `## RECURRENCE NOTE (D-NNNN) ...` (lines 96, 16 of
    // `.factory/cycles/v1.0-brownfield-backfill/lessons.md`).
    let h3_l050 =
        "### L-EDP1-050 — 49th-layer L-EDP1-003 recurrence: nineteenth consecutive violation\n";
    let decoy_subheading = "### Recursion ply mapping (nested detail, NOT a new lesson record)\n";
    let h3_l051 =
        "### L-EDP1-051 — 50th-layer L-EDP1-003 recurrence: twentieth consecutive violation\n";
    let h2_l052 = "## L-EDP1-052 — F5 pass-60 51st-layer L-EDP1-003 recurrence\n";
    let h2_lesson_d1065 = "## LESSON (D-1065) — S-21.19 is the first of the 7 split stories\n";
    let h2_recurrence_d1063 =
        "## RECURRENCE NOTE (D-1063) — D-1044(g)/D-995 class recurs one layer further out\n";

    let content = format!(
        "# Lessons Learned — engine-discipline cycle\n\n\
         {h3_l050}\
         **Pattern:** body text describing the 49th-layer recurrence in detail.\n\n\
         {decoy_subheading}\
         - Level-1: rule applied to named findings only\n\n\
         {h3_l051}\
         **Pattern:** body text describing the 50th-layer recurrence in detail.\n\n\
         {h2_l052}\
         **Pattern:** the h2 form adopted starting at this record.\n\n\
         {h2_lesson_d1065}\
         Brownfield's own LESSON (D-NNNN) h2 record form.\n\n\
         {h2_recurrence_d1063}\
         Brownfield's own RECURRENCE NOTE (D-NNNN) h2 record form.\n"
    );

    let offset_l050 = content.find(h3_l050).unwrap();
    let offset_decoy = content.find(decoy_subheading).unwrap();
    let offset_l051 = content.find(h3_l051).unwrap();
    let offset_l052 = content.find(h2_l052).unwrap();
    let offset_lesson = content.find(h2_lesson_d1065).unwrap();
    let offset_recurrence = content.find(h2_recurrence_d1063).unwrap();

    let offsets = mechanism_a_record_boundary_offsets("lessons", content.as_bytes());

    assert!(
        !offsets.contains(&offset_decoy),
        "PC2/Invariant 2: a nested `### ` sub-heading inside a lesson's own body (no L-<tag>-NNN \
         tag) must never be treated as a boundary. Got: {offsets:?}"
    );
    assert_eq!(
        offsets,
        vec![
            offset_l050,
            offset_l051,
            offset_l052,
            offset_lesson,
            offset_recurrence
        ],
        "PC2 Record-Boundary Marker Table: lessons.md's real record boundaries are its h2 \
         `## L-<tag>-NNN` / `## LESSON (D-NNNN)` / `## RECURRENCE NOTE (D-NNNN)` forms PLUS the \
         two confirmed pre-052 `### L-EDP1-050`/`### L-EDP1-051` h3-exception records — never \
         keyed on heading level alone. Got: {offsets:?}"
    );
}

#[test]
fn test_BC_1_18_008_PC2_MT_record_boundary_offsets_decision_log_ignores_appendix_subclause_h3_blocks()
 {
    // Re-grounded per BC-1.18.008 v1.2's Record-Boundary Marker Table:
    // decision-log.md's PRIMARY partition key is the `| D-` table row start
    // (already confirmed correct in both cycles); its `## Appendix:
    // Sub-clause Expansion` section nests `### D-NNN (...)` blocks (real,
    // verbatim heading forms per `.factory/cycles/v1.0-feature-engine-
    // discipline-pass-1/decision-log.md` lines 158/170/182/194/206, e.g.
    // `### D-440 (F5 pass-60 codification block; META-LEVEL-15 CANDIDATE
    // CONFIRMED)`) — these are secondary atomic units tied to their D-NNN
    // row, never themselves a primary shard-boundary. This test is NOT
    // expected to fail against the current implementation (its `| D-`
    // marker already never matches a `### ` or `## Appendix` line); it
    // exists as permanent regression coverage for the marker table's
    // decision-log row.
    let row_439 = "| D-439 | decided something | author |\n";
    let row_440 = "| D-440 | decided something else | author |\n";
    let appendix_heading = "## Appendix: Sub-clause Expansion\n";
    let subclause_440 =
        "### D-440 (F5 pass-60 codification block; META-LEVEL-15 CANDIDATE CONFIRMED)\n";
    let subclause_441 =
        "### D-441 (F5 pass-61 codification block; META-LEVEL-16 CANDIDATE CONFIRMED)\n";

    let content = format!(
        "# decision-log\n\n\
         ## Decisions Log\n\n\
         {row_439}\
         {row_440}\n\
         {appendix_heading}\n\
         {subclause_440}\
         Sub-clause expansion detail for D-440 (elided).\n\n\
         {subclause_441}\
         Sub-clause expansion detail for D-441 (elided).\n"
    );

    let offset_439 = content.find(row_439).unwrap();
    let offset_440 = content.find(row_440).unwrap();
    let offset_appendix = content.find(appendix_heading).unwrap();
    let offset_sub440 = content.find(subclause_440).unwrap();
    let offset_sub441 = content.find(subclause_441).unwrap();

    let offsets = mechanism_a_record_boundary_offsets("decision-log", content.as_bytes());

    assert_eq!(
        offsets,
        vec![offset_439, offset_440],
        "PC2 Record-Boundary Marker Table: decision-log.md's PRIMARY partition key is the `| D- \
         ` table row start — the `## Appendix: Sub-clause Expansion` section heading and its \
         nested `### D-NNN (...)` sub-clause blocks are secondary atomic units, never a primary \
         shard-boundary. Got: {offsets:?}"
    );
    assert!(
        !offsets.contains(&offset_appendix)
            && !offsets.contains(&offset_sub440)
            && !offsets.contains(&offset_sub441),
        "Appendix section heading and D-NNN sub-clause blocks must never appear in the primary \
         boundary set. Got: {offsets:?}"
    );
}

#[test]
fn test_BC_1_18_008_INV2_record_boundary_offsets_are_line_anchored_never_a_bare_substring_match() {
    // Invariant 2: boundaries are native structural record starts, never an
    // arbitrary byte offset — a mid-line, non-line-anchored occurrence of
    // the row-marker substring (inside prose, not at the start of an actual
    // table row) must NOT be treated as a record boundary.
    let real_row = "| D-010 | a real decision row | author |\n";
    let prose_line = "Note: rows begin with the literal text \"| D-\" by convention.\n";
    let content = format!("# decision-log\n\n## Decisions Log\n\n{prose_line}{real_row}");

    let real_offset = content.find(real_row).unwrap();

    let offsets = mechanism_a_record_boundary_offsets("decision-log", content.as_bytes());

    assert_eq!(
        offsets,
        vec![real_offset],
        "Invariant 2: only the LINE-ANCHORED real table row is a record boundary — a mid-line \
         mention of the same marker substring inside prose text must never be mistaken for one \
         (never mid-record, never a naive substring scan). Got: {offsets:?}"
    );
}

// ---------------------------------------------------------------------------
// AC-013 — `mechanism_a_partition_for_backfill`
// (BC-1.18.008 Postcondition 2, EC-001/EC-016, EC-002/EC-017)
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_008_EC016_partition_for_backfill_undersized_content_yields_single_partition() {
    let content = concat_records(&[1]); // one 30-byte record
    let partitions = mechanism_a_partition_for_backfill(&content, &[0], 1_000);

    assert_eq!(
        partitions.len(),
        1,
        "EC-016: content under shard_cap_bytes must yield exactly ceil(bytes/cap) = 1 partition"
    );
    assert_eq!(partitions[0].bytes, content);
    assert_eq!(partitions[0].record_count, 1);
    assert!(!partitions[0].oversized_record);
}

#[test]
fn test_BC_1_18_008_PC2_partition_for_backfill_never_splits_mid_record_and_stays_under_cap() {
    // 5 records x 30 bytes = 150 bytes; cap = 70 -> ceil(150/70) = 3
    // partitions. Every returned partition must (a) end exactly on a record
    // boundary, (b) stay <= cap, and (c) concatenate back to the original
    // content exactly (Postcondition 6a's own guarantee, checked here at the
    // partitioning level directly).
    let content = concat_records(&[1, 2, 3, 4, 5]);
    let boundaries = [0, 30, 60, 90, 120];

    let partitions = mechanism_a_partition_for_backfill(&content, &boundaries, 70);

    assert_eq!(
        partitions.len(),
        3,
        "PC2: ceil(150/70) = 3 partitions expected. Got {} partitions: {partitions:?}",
        partitions.len()
    );

    let mut reconstructed = Vec::new();
    let mut total_records = 0usize;
    for (i, p) in partitions.iter().enumerate() {
        assert!(
            p.bytes.len() <= 70,
            "PC2: partition {i} exceeds shard_cap_bytes (70) at {} bytes — no partition may \
             exceed cap here since no record in this fixture is individually oversized",
            p.bytes.len()
        );
        assert!(
            p.bytes.len() % 30 == 0,
            "PC2/Invariant 2: partition {i}'s byte length ({}) must be a whole multiple of the \
             30-byte record size — a partition boundary may never land mid-record",
            p.bytes.len()
        );
        assert!(
            !p.oversized_record,
            "no record in this fixture is individually oversized"
        );
        reconstructed.extend_from_slice(&p.bytes);
        total_records += p.record_count;
    }
    assert_eq!(
        reconstructed, content,
        "Postcondition 6(a): concatenating all partitions in order must reproduce the original \
         content byte-for-byte"
    );
    assert_eq!(
        total_records, 5,
        "Postcondition 6(b): the sum of every partition's record_count must equal the original \
         5-record total — never zero, never two, for any record"
    );
}

#[test]
fn test_BC_1_18_008_EC002_EC017_partition_for_backfill_oversized_single_record_flagged_not_split() {
    // Record 1 is 100 bytes (exceeds the 50-byte cap on its own); record 2
    // is a normal 20-byte record that fits. EC-002/EC-017: the oversized
    // record is NOT split mid-record — it is allowed to exceed cap for that
    // ONE record only, flagged `oversized_record: true`.
    let record1 = vec![b'A'; 100];
    let record2 = vec![b'B'; 20];
    let content: Vec<u8> = record1.iter().chain(record2.iter()).copied().collect();
    let boundaries = [0, 100];

    let partitions = mechanism_a_partition_for_backfill(&content, &boundaries, 50);

    assert_eq!(
        partitions.len(),
        2,
        "EC-002/EC-017: the oversized record and the following normal record must land in \
         separate partitions (the oversized one is never merged with, nor split from, its \
         neighbor)"
    );
    assert_eq!(partitions[0].bytes, record1);
    assert!(
        partitions[0].oversized_record,
        "EC-002/EC-017: the 100-byte record against a 50-byte cap must be flagged \
         oversized_record: true"
    );
    assert_eq!(partitions[0].record_count, 1);

    assert_eq!(partitions[1].bytes, record2);
    assert!(
        !partitions[1].oversized_record,
        "the second, normally-sized record must NOT be flagged oversized_record"
    );
    assert_eq!(partitions[1].record_count, 1);
}

// ---------------------------------------------------------------------------
// AC-014 — `mechanism_a_verify_backfill_content_preserved`
// (BC-1.18.008 Postcondition 6(a))
// ---------------------------------------------------------------------------

fn partition(
    bytes: &[u8],
    record_count: usize,
    oversized_record: bool,
) -> MechanismABackfillPartition {
    MechanismABackfillPartition {
        bytes: bytes.to_vec(),
        record_count,
        oversized_record,
    }
}

#[test]
fn test_BC_1_18_008_PC6a_verify_content_preserved_true_when_concatenation_matches_original() {
    let original = b"hello world foo bar".to_vec();
    let partitions = vec![
        partition(b"hello world ", 1, false),
        partition(b"foo bar", 1, false),
    ];

    assert!(
        mechanism_a_verify_backfill_content_preserved(&original, &partitions),
        "PC6(a): a correct byte-for-byte concatenation must verify as preserved"
    );
}

#[test]
fn test_BC_1_18_008_PC6a_verify_content_preserved_false_when_byte_mismatch() {
    let original = b"hello world foo bar".to_vec();
    // Dropped a trailing byte relative to the original — a genuine mismatch.
    let partitions = vec![
        partition(b"hello world ", 1, false),
        partition(b"foo ba", 1, false),
    ];

    assert!(
        !mechanism_a_verify_backfill_content_preserved(&original, &partitions),
        "PC6(a): a hard content mismatch (dropped trailing byte) must be detected as NOT \
         preserved — this is the mandatory content-preservation gate (EC-004's trigger)"
    );
}

// ---------------------------------------------------------------------------
// AC-014 — `mechanism_a_verify_backfill_record_counts_preserved`
// (BC-1.18.008 Postcondition 6(b))
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_008_PC6b_verify_record_counts_preserved_true_when_sum_matches() {
    let partitions = vec![
        partition(b"aaa", 2, false),
        partition(b"bbb", 2, false),
        partition(b"c", 1, false),
    ];

    assert!(
        mechanism_a_verify_backfill_record_counts_preserved(5, &partitions),
        "PC6(b): partition record_counts summing to the original total must verify as preserved"
    );
}

#[test]
fn test_BC_1_18_008_PC6b_verify_record_counts_preserved_false_when_record_dropped() {
    let partitions = vec![partition(b"aaa", 2, false), partition(b"bbb", 2, false)];

    assert!(
        !mechanism_a_verify_backfill_record_counts_preserved(5, &partitions),
        "PC6(b): a sum of 4 against an original count of 5 means a record was DROPPED — this \
         must be detected as NOT preserved (never zero, per Postcondition 6(b))"
    );
}

#[test]
fn test_BC_1_18_008_PC6b_verify_record_counts_preserved_false_when_record_duplicated() {
    let partitions = vec![
        partition(b"aaa", 3, false),
        partition(b"bbb", 2, false),
        partition(b"c", 1, false),
    ];

    assert!(
        !mechanism_a_verify_backfill_record_counts_preserved(5, &partitions),
        "PC6(b): a sum of 6 against an original count of 5 means a record was DUPLICATED across \
         two shards — this must be detected as NOT preserved (never two, per Postcondition 6(b))"
    );
}

// ---------------------------------------------------------------------------
// AC-014 — `mechanism_a_backfill_already_migrated`
// (BC-1.18.008 Invariant 3)
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_008_INV3_backfill_already_migrated_false_when_no_shard_index_exists() {
    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    std::fs::write(&canonical_path, "monolithic content, never backfilled").unwrap();

    let result = mechanism_a_backfill_already_migrated(&canonical_path, "decision-log")
        .expect("checking migration state against a plain, un-migrated artifact must not error");

    assert!(
        !result,
        "Invariant 3: an artifact with no shard-index sibling has never been backfilled"
    );
}

#[test]
fn test_BC_1_18_008_INV3_backfill_already_migrated_true_when_shard_index_already_exists() {
    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    std::fs::write(&canonical_path, "post-backfill current content").unwrap();
    // A shard-index already exists (EC-016's own "shard-index IS still
    // created, even with zero [[shard]] entries" registered state counts as
    // migrated too).
    std::fs::write(
        dir.path().join("decision-log.shard-index.toml"),
        "schema_version = 1\n",
    )
    .unwrap();

    let result = mechanism_a_backfill_already_migrated(&canonical_path, "decision-log")
        .expect("checking migration state against an already-migrated artifact must not error");

    assert!(
        result,
        "Invariant 3: a pre-existing shard-index for this artifact means the backfill-split has \
         already run — the idempotency short-circuit must detect this"
    );
}

// ---------------------------------------------------------------------------
// AC-013/AC-014 — `run_mechanism_a_backfill_split` end-to-end
// (BC-1.18.008 Postconditions 1-6, Invariants 1-3, EC-003, EC-016, EC-017)
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_008_EC016_run_backfill_split_undersized_artifact_creates_zero_shard_index_no_sealing()
 {
    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    let original_content = concat_records(&[7, 8]); // 60 bytes, well under cap
    std::fs::write(&canonical_path, &original_content).unwrap();

    let entry = flat_entry("decision-log", 10_000);
    // A single record boundary at offset 0: the whole (undersized) content
    // is treated as one record for this fixture's purposes.
    let outcome = run_mechanism_a_backfill_split(&entry, &canonical_path, &[0], 10)
        .expect("EC-016: an undersized artifact's backfill must succeed, not error");

    assert_eq!(
        outcome,
        MechanismABackfillOutcome::Migrated {
            sealed_count: 0,
            archived_count: 0
        },
        "EC-016: ceil(bytes/cap) = 1 -> zero shards sealed, zero archived"
    );

    // "the file remains at the canonical name unchanged"
    let post_content = std::fs::read(&canonical_path).unwrap();
    assert_eq!(
        post_content, original_content,
        "EC-016: the canonical file's content must be COMPLETELY UNCHANGED when no split was \
         structurally necessary"
    );

    // "a shard-index IS still created (with zero [[shard]] entries)"
    let index_path = dir.path().join("decision-log.shard-index.toml");
    let index_toml = std::fs::read_to_string(&index_path)
        .expect("EC-016: a shard-index MUST still be created, even with zero sealed shards");
    let index: factory_dispatcher::shard_manager::ShardIndex = toml::from_str(&index_toml)
        .expect("the shard-index must be valid TOML matching the ShardIndex schema");
    assert_eq!(
        index.shards.len(),
        0,
        "EC-016: the shard-index must be registered with exactly zero [[shard]] entries"
    );
}

#[test]
fn test_BC_1_18_008_PC2_PC3_run_backfill_split_oversized_artifact_produces_sealed_shards_and_fresh_current()
 {
    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    // 5 records x 30 bytes = 150 bytes; cap = 70 -> ceil(150/70) = 3
    // partitions -> 2 sealed shards + 1 fresh current file.
    let original_content = concat_records(&[1, 2, 3, 4, 5]);
    std::fs::write(&canonical_path, &original_content).unwrap();

    let entry = flat_entry("decision-log", 70);
    let boundaries = [0, 30, 60, 90, 120];

    let outcome = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 10)
        .expect("PC2/PC3: a well-formed oversized-artifact backfill must succeed");

    let MechanismABackfillOutcome::Migrated {
        sealed_count,
        archived_count,
    } = outcome
    else {
        panic!("PC2/PC3: expected Migrated outcome, got {outcome:?}");
    };
    assert_eq!(
        sealed_count, 2,
        "PC2: 3 total partitions -> 2 sealed shards (the last partition becomes the fresh \
         current file, never sealed)"
    );
    assert_eq!(
        archived_count, 0,
        "PC4: retention_count=10 comfortably covers 2 sealed shards -> no archival composition \
         needed this run"
    );

    // Postcondition 6(a), checked end-to-end across the real files this
    // call produced: concatenating every sealed shard (in seq order) plus
    // the fresh current file must reproduce the ORIGINAL monolithic
    // content byte-for-byte.
    let mut reconstructed = Vec::new();
    for seq in 1..=sealed_count {
        let sealed_path = dir.path().join(format!("decision-log.{seq:04}.md"));
        let sealed_bytes = std::fs::read(&sealed_path)
            .unwrap_or_else(|e| panic!("PC2: sealed shard seq={seq} must exist on disk: {e}"));
        assert!(
            sealed_bytes.len() as u64 <= entry.shard_cap_bytes,
            "PC2: sealed shard seq={seq} ({} bytes) must not exceed shard_cap_bytes (70) — no \
             record in this fixture is individually oversized",
            sealed_bytes.len()
        );
        reconstructed.extend_from_slice(&sealed_bytes);
    }
    let current_bytes = std::fs::read(&canonical_path).unwrap();
    assert!(
        current_bytes.len() as u64 <= entry.shard_cap_bytes,
        "PC2: the fresh current file ({} bytes) must not exceed shard_cap_bytes",
        current_bytes.len()
    );
    reconstructed.extend_from_slice(&current_bytes);

    assert_eq!(
        reconstructed, original_content,
        "Postcondition 6(a): sealed shards (in seq order) + fresh current file must reproduce \
         the original monolithic content byte-for-byte"
    );
    assert_ne!(
        current_bytes, original_content,
        "Postcondition 2: the canonical file must hold ONLY the LAST partition after the split \
         — never the full original content"
    );

    // Postcondition 3: the full shard index is published for the complete
    // pre-existing history in this SAME operation.
    let index_path = dir.path().join("decision-log.shard-index.toml");
    let index_toml = std::fs::read_to_string(&index_path)
        .expect("PC3: the shard-index must be published in the same operation");
    let index: factory_dispatcher::shard_manager::ShardIndex =
        toml::from_str(&index_toml).expect("the shard-index must be valid TOML");
    assert_eq!(
        index.shards.len(),
        2,
        "PC3: one [[shard]] entry per sealed partition (not the fresh current)"
    );
    let mut seqs: Vec<u32> = index.shards.iter().map(|s| s.seq).collect();
    seqs.sort_unstable();
    assert_eq!(
        seqs,
        vec![1, 2],
        "PC2: sealed seq numbers start at 1, sequential"
    );
}

#[test]
fn test_BC_1_18_008_PC4_run_backfill_split_composes_with_retention_when_shard_count_exceeds_retention_count()
 {
    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    let original_content = concat_records(&[1, 2, 3, 4, 5]);
    std::fs::write(&canonical_path, &original_content).unwrap();

    let entry = flat_entry("decision-log", 70);
    let boundaries = [0, 30, 60, 90, 120];
    // retention_count = 1: the 2 sealed shards this split produces already
    // exceed it by 1 -> Postcondition 4's SAME-operation archival composition
    // must archive the oldest (seq=1) immediately, never deferred.
    let outcome = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 1)
        .expect("PC4: composing backfill-split with retention archival must succeed");

    assert_eq!(
        outcome,
        MechanismABackfillOutcome::Migrated {
            sealed_count: 2,
            archived_count: 1
        },
        "PC4: 2 sealed shards against retention_count=1 must archive exactly 1 (the oldest) in \
         this SAME operation"
    );

    // The oldest sealed shard (seq=1) must have been relocated to the
    // archive subdirectory, never left at the cycle root.
    assert!(
        !dir.path().join("decision-log.0001.md").exists(),
        "PC4: the archived (oldest) sealed shard must not remain at the cycle root"
    );
    assert!(
        dir.path()
            .join("archive")
            .join("decision-log")
            .join("decision-log.0001.md")
            .exists(),
        "PC4: the archived (oldest) sealed shard must exist under archive/<artifact-stem>/"
    );
    assert!(
        dir.path().join("decision-log.0002.md").exists(),
        "PC4: the newer sealed shard (seq=2) stays within the retention window at the cycle root"
    );

    let index_toml =
        std::fs::read_to_string(dir.path().join("decision-log.shard-index.toml")).unwrap();
    let index: factory_dispatcher::shard_manager::ShardIndex = toml::from_str(&index_toml).unwrap();
    let seq1 = index
        .shards
        .iter()
        .find(|s| s.seq == 1)
        .expect("seq=1 must remain enumerable");
    assert!(
        seq1.path.starts_with("archive/"),
        "PC4/Invariant 3: the archived entry's index record must reflect the new archived path"
    );
}

#[test]
fn test_BC_1_18_008_EC017_run_backfill_split_oversized_record_sealed_whole_never_split_mid_record()
{
    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    let record1 = vec![b'A'; 100]; // oversized (> cap)
    let record2 = vec![b'B'; 20]; // normal, becomes the fresh current
    let original_content: Vec<u8> = record1.iter().chain(record2.iter()).copied().collect();
    std::fs::write(&canonical_path, &original_content).unwrap();

    let entry = flat_entry("decision-log", 50);
    let outcome = run_mechanism_a_backfill_split(&entry, &canonical_path, &[0, 100], 10)
        .expect("EC-017: a backfill containing one oversized record must still succeed");

    assert_eq!(
        outcome,
        MechanismABackfillOutcome::Migrated {
            sealed_count: 1,
            archived_count: 0
        },
        "EC-017: the oversized record seals as its OWN shard (never split); the normal 20-byte \
         record becomes the fresh current file"
    );

    let sealed_bytes = std::fs::read(dir.path().join("decision-log.0001.md"))
        .expect("EC-017: the oversized record must be sealed as its own shard");
    assert_eq!(
        sealed_bytes, record1,
        "EC-017: the sealed shard for the oversized record must contain the FULL 100-byte \
         record, un-truncated, un-split — exceeding shard_cap_bytes (50) for this ONE record"
    );

    let current_bytes = std::fs::read(&canonical_path).unwrap();
    assert_eq!(
        current_bytes, record2,
        "EC-017: the fresh current file must hold exactly the trailing normal-sized record"
    );
}

#[test]
fn test_BC_1_18_008_INV3_run_backfill_split_idempotent_second_run_returns_already_migrated() {
    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    let original_content = concat_records(&[1, 2, 3, 4, 5]);
    std::fs::write(&canonical_path, &original_content).unwrap();

    let entry = flat_entry("decision-log", 70);
    let boundaries = [0, 30, 60, 90, 120];

    let first = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 10)
        .expect("the first backfill run must succeed");
    assert_eq!(
        first,
        MechanismABackfillOutcome::Migrated {
            sealed_count: 2,
            archived_count: 0
        },
        "precondition: the first run must actually migrate (see the PC2/PC3 happy-path test)"
    );

    // Invariant 3: re-running the backfill against the now-already-migrated
    // artifact (its canonical file has ALREADY been rewritten to the fresh
    // current partition by the first call) must detect the already-migrated
    // state and short-circuit — never double-split.
    let second = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 10)
        .expect("Invariant 3: re-running an already-migrated backfill must not error");

    assert_eq!(
        second,
        MechanismABackfillOutcome::AlreadyMigrated,
        "Invariant 3: a second invocation against an already-migrated artifact must return \
         AlreadyMigrated, never re-split"
    );

    // No third sealed shard must have been produced by the second call.
    assert!(
        !dir.path().join("decision-log.0003.md").exists(),
        "Invariant 3: the idempotent second run must never produce additional sealed shards"
    );
    let index_toml =
        std::fs::read_to_string(dir.path().join("decision-log.shard-index.toml")).unwrap();
    let index: factory_dispatcher::shard_manager::ShardIndex = toml::from_str(&index_toml).unwrap();
    assert_eq!(
        index.shards.len(),
        2,
        "Invariant 3: the shard-index's shard count must be UNCHANGED after the idempotent \
         second run (still exactly the 2 shards the first run produced)"
    );
}

#[test]
fn test_BC_1_18_008_EC003_run_backfill_split_restart_after_partial_prior_attempt_produces_correct_result()
 {
    // EC-003: "Backfill process crashes after writing shard files 1-3 of an
    // expected 19" / Postcondition 5: "the operation MUST be safely
    // re-runnable from scratch ... it does not corrupt the original
    // monolithic file until every resulting shard file AND the shard-index
    // have been written to a staging location and validated". Simulated
    // here as a stray, STALE partial artifact from an aborted prior attempt
    // (wrong content, no valid shard-index alongside it — the crash
    // happened before the operation ever reached a validated, complete
    // state) sitting at the destination seq path BEFORE a fresh, from-
    // scratch run against the STILL-ORIGINAL, untouched monolithic file.
    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    let original_content = concat_records(&[1, 2, 3, 4, 5]);
    std::fs::write(&canonical_path, &original_content).unwrap();

    // A stale, WRONG leftover at the seq=1 destination from a crashed prior
    // attempt — critically, with NO shard-index present, so
    // mechanism_a_backfill_already_migrated would correctly report `false`
    // (this crash happened before the operation ever durably completed).
    std::fs::write(
        dir.path().join("decision-log.0001.md"),
        b"stale garbage from a crash",
    )
    .unwrap();

    let entry = flat_entry("decision-log", 70);
    let boundaries = [0, 30, 60, 90, 120];

    let outcome = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 10).expect(
        "EC-003/Postcondition 5: a from-scratch restart over a stale, incomplete prior \
             attempt must succeed, not be blocked or corrupted by the stray leftover",
    );

    assert_eq!(
        outcome,
        MechanismABackfillOutcome::Migrated {
            sealed_count: 2,
            archived_count: 0
        },
        "EC-003: the restart must produce the SAME correct result an uninitialized run would \
         have produced"
    );

    let sealed_1 = std::fs::read(dir.path().join("decision-log.0001.md")).unwrap();
    assert_ne!(
        sealed_1, b"stale garbage from a crash",
        "EC-003/Postcondition 5: the restart must OVERWRITE the stale leftover with the \
         correct, freshly-computed sealed content — never leave the crash-orphaned garbage in \
         place"
    );

    // Full content-preservation across the restart's real output.
    let sealed_2 = std::fs::read(dir.path().join("decision-log.0002.md")).unwrap();
    let current = std::fs::read(&canonical_path).unwrap();
    let mut reconstructed = sealed_1;
    reconstructed.extend_from_slice(&sealed_2);
    reconstructed.extend_from_slice(&current);
    assert_eq!(
        reconstructed, original_content,
        "EC-003/Postcondition 6(a): the restart's real output must reproduce the original \
         monolithic content byte-for-byte, exactly as an uninitialized run would"
    );
}

// ---------------------------------------------------------------------------
// F4 BC-cluster-3 adversarial-review additions (adv-cluster3-p1, adv-cluster3-p2)
//
// The groups below encode findings from successive fresh-context
// adversarial passes over this cluster's implementation. Each test asserts
// the REAL, spec-mandated outcome (never a weakened/should-panic
// substitute) and pins that outcome as PERMANENT regression coverage — the
// originating finding ID (BLOCKER-1, HIGH-2, MED-3, F-001, F-002, F-004,
// P2-001, P2-002, P2-003, ...) named in each test's own doc comment is
// historical provenance for why the test exists, never a live claim about
// whether it currently passes or fails against `shard_manager.rs` (see the
// module-level doc comment at the top of this file for the same discipline
// applied there).
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// BLOCKER-1 (BC-1.18.008 Postcondition 2/Postcondition 6(a)): preamble byte
// loss. `mechanism_a_partition_for_backfill` seeds `partition_start` from
// `record_boundary_offsets[0]` — when the first detected record boundary is
// NOT at byte 0 (a real leading preamble, e.g. a `# decision-log` header
// before the first `## Decisions Log` table row), every byte in
// `content[0..record_boundary_offsets[0])` is silently DROPPED from every
// returned partition, which then fails Postcondition 6(a)'s mandatory
// byte-for-byte content-preservation gate.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_008_BLOCKER1_partition_for_backfill_preserves_preamble_bytes_before_first_boundary()
{
    // Realistic decision-log.md shape: a document header/preamble BEFORE
    // the first real `| D-` row -- offsets[0] is therefore > 0, a real
    // leading preamble the split must not drop.
    let header = "# decision-log\n\n## Decisions Log\n\n";
    let row1 = "| D-001 | decided one | author |\n";
    let row2 = "| D-002 | decided two | author |\n";
    let content = format!("{header}{row1}{row2}");
    let content_bytes = content.as_bytes();

    let offsets = mechanism_a_record_boundary_offsets("decision-log", content_bytes);
    assert!(
        offsets[0] > 0,
        "test fixture precondition: the first detected record boundary must be AFTER a real, \
         non-empty preamble (offsets[0] == 0 would not exercise BLOCKER-1). Got offsets: \
         {offsets:?}"
    );

    // A large cap -> structurally a single partition; the whole content
    // (preamble included) must round-trip regardless of partition count.
    let partitions = mechanism_a_partition_for_backfill(content_bytes, &offsets, 10_000);

    let mut reconstructed = Vec::new();
    for p in &partitions {
        reconstructed.extend_from_slice(&p.bytes);
    }
    assert_eq!(
        reconstructed,
        content_bytes,
        "BLOCKER-1 (PC6(a)/PC2): the concatenation of ALL returned partitions must reproduce \
         the ORIGINAL content byte-for-byte, including the {} leading preamble bytes before the \
         first record boundary at offset {} -- the current implementation seeds \
         `partition_start` from `record_boundary_offsets[0]` and silently DROPS \
         content[0..{}) from every partition. Reconstructed length: {}, original length: {}",
        offsets[0],
        offsets[0],
        offsets[0],
        reconstructed.len(),
        content_bytes.len()
    );
}

#[test]
fn test_BC_1_18_008_BLOCKER1_run_backfill_split_preserves_preamble_bytes_end_to_end() {
    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");

    // Same realistic preamble-before-first-row shape as the unit-level test
    // above, but driven all the way through the real, on-disk end-to-end
    // entry point.
    let preamble = b"# decision-log\n\n## Decisions Log\n\n".to_vec();
    let records = concat_records(&[1, 2, 3, 4, 5]); // 150 bytes, 5 records
    let mut original_content = preamble.clone();
    original_content.extend_from_slice(&records);
    std::fs::write(&canonical_path, &original_content).unwrap();

    let boundaries: Vec<usize> = [0usize, 30, 60, 90, 120]
        .iter()
        .map(|o| o + preamble.len())
        .collect();
    assert!(
        boundaries[0] > 0,
        "test fixture precondition: a real leading preamble before the first record boundary"
    );

    let entry = flat_entry("decision-log", 70);

    let outcome = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 10);

    let outcome = outcome.unwrap_or_else(|e| {
        panic!(
            "BLOCKER-1: a backfill over content with a leading preamble before its first record \
             boundary must SUCCEED (byte-for-byte content-preservation must hold over the WHOLE \
             original content, preamble included) -- got an error instead, which means the \
             preamble bytes were silently dropped from the computed partitions and the \
             Postcondition 6 hard gate (correctly) caught the resulting corruption: {e}"
        )
    });

    let MechanismABackfillOutcome::Migrated { sealed_count, .. } = outcome else {
        panic!(
            "BLOCKER-1: expected a Migrated outcome for this oversized-with-preamble fixture, \
             got {outcome:?}"
        );
    };

    // Postcondition 6(a), end-to-end: every sealed shard (seq order) plus
    // the fresh current file must reproduce the ORIGINAL monolithic
    // content byte-for-byte, preamble included.
    let mut reconstructed = Vec::new();
    for seq in 1..=sealed_count {
        let sealed_path = dir.path().join(format!("decision-log.{seq:04}.md"));
        let sealed_bytes = std::fs::read(&sealed_path)
            .unwrap_or_else(|e| panic!("BLOCKER-1: sealed shard seq={seq} must exist: {e}"));
        reconstructed.extend_from_slice(&sealed_bytes);
    }
    reconstructed.extend_from_slice(&std::fs::read(&canonical_path).unwrap());

    assert_eq!(
        reconstructed,
        original_content,
        "BLOCKER-1 (PC6(a)): sealed shards (in seq order) + fresh current file must reproduce \
         the ORIGINAL monolithic content byte-for-byte, including its {}-byte leading preamble",
        preamble.len()
    );
}

// ---------------------------------------------------------------------------
// HIGH-2 (BC-1.18.008 Postcondition 5, Invariant 3, VP-124 Property
// Statement 1): a crash between the shard-index publish and the canonical-
// truncate write is not self-healed on re-run. `run_mechanism_a_backfill_split`
// publishes the shard-index BEFORE truncating the canonical file (by design,
// per this module's own doc comment reasoning) -- but
// `mechanism_a_backfill_already_migrated` keys ONLY on the shard-index's
// existence. A re-run against exactly this crash window observes the index
// already exists, reports `AlreadyMigrated`, and returns immediately WITHOUT
// ever truncating the canonical file -- leaving the canonical file holding
// the FULL original content (records already duplicated into the sealed
// shards AND still present in the untouched canonical file), a silent,
// permanent whole-corpus double-count corruption. This is the DANGEROUS
// crash window VP-124 names distinctly from EC-003's SAFE no-index-yet
// window (already covered by the existing EC-003 test above).
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_008_HIGH2_run_backfill_split_self_heals_crash_between_index_publish_and_canonical_truncate()
 {
    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    // Same 5-record / cap=70 fixture as the PC2/PC3 happy-path test above:
    // partitions = [0..60) seq=1, [60..120) seq=2, [120..150) current.
    let original_content = concat_records(&[1, 2, 3, 4, 5]); // 150 bytes
    let sealed1 = &original_content[0..60];
    let sealed2 = &original_content[60..120];
    let expected_current = &original_content[120..150];

    // Simulate the crash window: the canonical file is STILL the full,
    // untruncated original (the crash happened AFTER the index publish
    // below, BEFORE the canonical-truncate write) ...
    std::fs::write(&canonical_path, &original_content).unwrap();

    // ... but BOTH sealed shard files ...
    std::fs::write(dir.path().join("decision-log.0001.md"), sealed1).unwrap();
    std::fs::write(dir.path().join("decision-log.0002.md"), sealed2).unwrap();

    // ... AND the shard-index (fully accounting for both seals) already
    // exist on disk, exactly as a completed split's index-publish step
    // would have left them.
    let index = ShardIndex {
        schema_version: 1,
        artifact_stem: "decision-log".to_string(),
        current_shard: "decision-log.md".to_string(),
        shard_cap_bytes: 70,
        max_single_record_bytes: 16_384,
        safety_margin_bytes: 8_192,
        practical_fuel_ceiling: 8_000_000,
        worst_case_fuel_per_byte: 106.36,
        retention_count: 10,
        shards: vec![
            ShardIndexEntry {
                seq: 1,
                path: "decision-log.0001.md".to_string(),
                sealed_at: "2026-01-01T00:00:00Z".to_string(),
                bytes_at_seal: sealed1.len() as u64,
                sealed_retroactively: false,
            },
            ShardIndexEntry {
                seq: 2,
                path: "decision-log.0002.md".to_string(),
                sealed_at: "2026-01-01T00:00:00Z".to_string(),
                bytes_at_seal: sealed2.len() as u64,
                sealed_retroactively: false,
            },
        ],
    };
    let index_toml = toml::to_string(&index).expect("ShardIndex must serialize to valid TOML");
    std::fs::write(dir.path().join("decision-log.shard-index.toml"), index_toml).unwrap();

    let entry = flat_entry("decision-log", 70);
    let boundaries = [0, 30, 60, 90, 120];

    let result = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 10);

    assert!(
        result.is_ok(),
        "HIGH-2/VP-124: re-running the backfill against the dangerous crash window (index \
         already published, canonical NOT yet truncated) must self-heal, not error. Got: {:?}",
        result.err()
    );

    // The self-heal MUST complete the truncation the crash interrupted:
    // the canonical file must end up holding ONLY the final partition, not
    // the full pre-crash original content.
    let post_canonical = std::fs::read(&canonical_path).unwrap();
    assert_eq!(
        post_canonical,
        expected_current,
        "HIGH-2/VP-124: after self-healing this crash window, the canonical file MUST be \
         truncated to exactly the final partition's content -- got {} bytes (still holding the \
         full {}-byte pre-crash original) instead of the expected {} bytes. Leaving the \
         canonical file un-truncated here means records 1-4 exist BOTH in the sealed shards AND \
         in the still-full canonical file -- a silent, permanent whole-corpus double-count \
         corruption that `mechanism_a_backfill_already_migrated`'s index-existence-only check \
         allowed to persist forever (it never re-inspects the canonical file's own content).",
        post_canonical.len(),
        original_content.len(),
        expected_current.len()
    );

    // Whole-corpus reconstruction: sealed shard 1 + sealed shard 2 + the
    // (now-healed) canonical file must reproduce the original content
    // EXACTLY ONCE -- no record dropped, no record duplicated.
    let mut reconstructed = Vec::new();
    reconstructed
        .extend_from_slice(&std::fs::read(dir.path().join("decision-log.0001.md")).unwrap());
    reconstructed
        .extend_from_slice(&std::fs::read(dir.path().join("decision-log.0002.md")).unwrap());
    reconstructed.extend_from_slice(&std::fs::read(&canonical_path).unwrap());
    assert_eq!(
        reconstructed, original_content,
        "HIGH-2/VP-124 Property Statement 1: sealed shard 1 + sealed shard 2 + the healed \
         canonical file must reconstruct the original content EXACTLY ONCE -- no duplicated, no \
         dropped records"
    );

    // The shard-index itself must remain exactly 2 entries -- the self-heal
    // must never re-seal already-sealed content into new, redundant shards.
    let index_toml_after =
        std::fs::read_to_string(dir.path().join("decision-log.shard-index.toml")).unwrap();
    let index_after: ShardIndex = toml::from_str(&index_toml_after).unwrap();
    assert_eq!(
        index_after.shards.len(),
        2,
        "HIGH-2/VP-124 (Idempotency, Invariant 3): the self-heal must never produce additional \
         or duplicate shard-index entries beyond the 2 that already correctly existed"
    );
}

// ---------------------------------------------------------------------------
// MED-3 (BC-1.18.008 Postcondition 2/Invariant 2): `lessons.md` and
// `session-checkpoints.md` boundary detection over REALISTIC content. The
// current implementation matches ANY line-anchored `"### "` (lessons) or
// `"## "` (session-checkpoints) occurrence as a record boundary, with no way
// to distinguish a genuine new-record heading from a NESTED, non-record
// sub-heading inside an existing record's own body -- both real formats
// contain exactly this shape (see e.g. `.factory/cycles/*/lessons.md`'s own
// `### L-EDP1-NNN` records containing nested `###`-free prose, and
// `.factory/cycles/*/session-checkpoints.md`'s own `## Session Resume
// Checkpoint (...)` records, which some entries nest `### State` / `###
// Resume Path A` / `### Outstanding follow-up tasks` sub-headings under).
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_008_MED3_record_boundary_offsets_lessons_ignores_nested_non_record_subheading() {
    // Realistic lessons.md shape (mirrors the real
    // `v1.0-feature-engine-discipline-pass-1/lessons.md` L-EDP1-NNN record
    // convention): each genuine lesson record starts with a line-anchored
    // `### L-EDP1-NNN -- ...` heading. A lesson's own BODY may contain a
    // nested sub-heading at the SAME `### ` markdown level (e.g. breaking
    // out a numbered recursion-ply mapping) that is NOT itself a new lesson
    // record -- it carries no `L-EDP1-NNN` tag and is clearly part of the
    // PRECEDING lesson's own content.
    let lesson1 = "### L-EDP1-050 -- 49th-layer recurrence: nineteenth consecutive violation\n";
    let decoy_subheading = "### Recursion ply mapping (nested detail, NOT a new lesson record)\n";
    let lesson2 = "### L-EDP1-051 -- 50th-layer recurrence: twentieth consecutive violation\n";
    let content = format!(
        "# Lessons Learned -- engine-discipline cycle\n\n\
         {lesson1}\
         **Pattern:** body text describing the 49th-layer recurrence in detail.\n\n\
         {decoy_subheading}\
         - Level-1: rule applied to named findings only\n\
         - Level-2: fix-extension applied to named forms only\n\n\
         {lesson2}\
         **Pattern:** body text describing the 50th-layer recurrence in detail.\n"
    );

    let offset1 = content.find(lesson1).unwrap();
    let decoy_offset = content.find(decoy_subheading).unwrap();
    let offset2 = content.find(lesson2).unwrap();

    let offsets = mechanism_a_record_boundary_offsets("lessons", content.as_bytes());

    assert!(
        !offsets.contains(&decoy_offset),
        "MED-3 (PC2/Invariant 2): a nested `### ` sub-heading inside a lesson's OWN body (no \
         `L-EDP1-NNN` tag -- not a new lesson record) must NOT be treated as a structural record \
         boundary, or the backfill-split would cut the L-EDP1-050 lesson's own body in half \
         across two shards. Got offsets: {offsets:?} (decoy at {decoy_offset})"
    );
    assert_eq!(
        offsets,
        vec![offset1, offset2],
        "MED-3: only the two genuine `### L-EDP1-NNN` lesson-record headings are real \
         structural boundaries. Got: {offsets:?}"
    );
}

// ---------------------------------------------------------------------------
// F-002 (BC-1.18.008 v1.3 fix-burst, HIGH): session-checkpoints.md bare
// `^## ` re-grounding. This test pins the Record-Boundary Marker Table's
// session-checkpoints.md row: EVERY bare `^## ` heading is a record
// boundary, with no content-based filter.
//
// Provenance: the original MED-3 finding had argued session-checkpoints.md
// needed the SAME content-based h2 filtering lessons.md uses
// (`is_checkpoint_record_heading`, gating on `starts_with("Archived") ||
// contains("Checkpoint")`). A follow-up fresh-context adversarial pass over
// BC-1.18.008 v1.3 found this filter itself defective and product-owner's
// DECISION (BC-1.18.008 v1.3 Changelog, finding F-002) was to revert to the
// ORIGINAL "any h2 = record boundary" rule: direct inspection of BOTH real
// `session-checkpoints.md` files (`v1.0-brownfield-backfill/`, 182 h2
// records; `v1.0-feature-engine-discipline-pass-1/`, 12 h2 records)
// confirmed every h2 heading in both files is a genuine checkpoint record
// with ZERO legitimate non-record h2 asides -- so the marker table's own
// session-checkpoints.md row ("any h2 = boundary, no confirmed exception
// forms") was ALREADY correct as written. The code-side
// `is_checkpoint_record_heading` heuristic was the defective party: being
// case-sensitive, it silently dropped real records whose heading text
// didn't literally contain the substrings "Archived" (title-case) or
// "Checkpoint" (title-case) -- e.g. the real, verbatim `## ARCHIVED
// CHECKPOINT: 2026-08-27 -- pass-60 CLEAN D-1117...` record (all-caps)
// matches NEITHER `starts_with("Archived")` NOR `contains("Checkpoint")`.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_008_F002_record_boundary_offsets_session_checkpoints_treats_every_bare_h2_as_boundary()
 {
    // Real, verbatim heading forms per BC-1.18.008 v1.3's own F-002
    // adjudication evidence: the all-caps `## ARCHIVED CHECKPOINT: ...` form
    // is lifted directly from the BC changelog's cited real record. The
    // `## SESSION WRAP -- ...` heading is a genuine checkpoint record that
    // contains NEITHER "Archived" NOR "Checkpoint" in any case -- proving
    // the current case-sensitive, content-based heuristic drops legitimate
    // records outright, not merely mis-cases the all-caps form.
    let h2_checkpoint1 =
        "## Session Resume Checkpoint (2026-08-20 -- PIPELINE ACTIVE; pass-12 dispatch NEXT)\n";
    let h3_nested = "### State\n";
    let h2_archived_allcaps =
        "## ARCHIVED CHECKPOINT: 2026-08-27 -- pass-60 CLEAN D-1117 confirmed\n";
    let h2_no_keyword = "## SESSION WRAP -- 2026-07-01 (pause burst, D-987 pre-clear)\n";

    let content = format!(
        "# Session Checkpoints -- v1.0-brownfield-backfill\n\n\
         {h2_checkpoint1}\
         Archived from STATE.md by the D-1052 pass-12 CLEAN burst.\n\n\
         {h3_nested}\
         Nested sub-heading inside checkpoint 1's own body -- NOT a boundary (one level deeper \
         than the `## ` record marker, per the marker table's own \"NOT a boundary\" column).\n\n\
         {h2_archived_allcaps}\
         Body text for the real, all-caps ARCHIVED CHECKPOINT record form.\n\n\
         {h2_no_keyword}\
         Body text for a genuine checkpoint record whose own heading contains neither \
         \"Archived\" nor \"Checkpoint\" in any case.\n"
    );

    let offset1 = content.find(h2_checkpoint1).unwrap();
    let offset_h3 = content.find(h3_nested).unwrap();
    let offset_archived = content.find(h2_archived_allcaps).unwrap();
    let offset_no_keyword = content.find(h2_no_keyword).unwrap();

    let offsets = mechanism_a_record_boundary_offsets("session-checkpoints", content.as_bytes());

    assert_eq!(
        offsets,
        vec![offset1, offset_archived, offset_no_keyword],
        "F-002/PC2 Record-Boundary Marker Table (session-checkpoints.md row, 'any h2, no \
         confirmed exception forms'): EVERY bare `^## ` heading is a record boundary -- \
         including the all-caps `## ARCHIVED CHECKPOINT: ...` form and a genuine record whose \
         heading contains neither \"Archived\" nor \"Checkpoint\" -- regardless of a \
         content-based filter. Got: {offsets:?}"
    );
    assert!(
        !offsets.contains(&offset_h3),
        "Invariant 2: a nested `### ` sub-heading (one level deeper than the `## ` record \
         marker) must never be treated as a boundary. Got: {offsets:?}"
    );
}

// ---------------------------------------------------------------------------
// F-001 (BC-1.18.008 v1.3 fix-burst, BLOCKER): the Postcondition 6
// content-preservation abort (`E-SHD-003`) must be reachable from outside
// the module for a caller-supplied `record_boundary_offsets` argument that
// is well-formed (strictly ascending, in-bounds -- MED-C's own check) but
// simply WRONG against the content's real structure -- missing a genuine
// boundary the content actually contains (stale against the just-read
// `original_content`). This test pins that abort: it feeds
// `run_mechanism_a_backfill_split` a `decision-log` fixture with 3 genuine
// `"| D-"` row boundaries but a caller-supplied `record_boundary_offsets`
// missing the middle one, and asserts the operation ABORTS with
// `E-SHD-003` (BC-1.18.008 Postcondition 6, EC-004/EC-006), leaving the
// original file completely untouched.
//
// Provenance: prior to this fix, `run_mechanism_a_backfill_split` derived
// BOTH sides of its Postcondition 6(b) record-count comparison from the
// SAME `record_boundary_offsets.len()` value -- `original_record_count` was
// set directly from it, and `mechanism_a_partition_for_backfill`'s own
// `record_count` sum, by construction, always totals exactly
// `record_boundary_offsets.len()` too (the partitioning loop iterates the
// offsets list exactly once per entry) -- so a well-formed-but-wrong
// offsets list tautologically reported "preserved," because both checks
// were computed FROM the same wrong offsets, never against the content's
// own independently-detectable true structure. The fix makes the gate
// independently recompute the content's own true record structure (via
// `mechanism_a_record_boundary_offsets`) and fold in any genuine boundary
// the caller's list is missing, rather than trusting the caller-supplied
// offsets count at face value.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_008_F001_run_backfill_split_aborts_when_supplied_offsets_miss_a_real_boundary() {
    // Three genuine, uniformly-sized `decision-log` rows -- real record
    // boundaries independently detectable via
    // `mechanism_a_record_boundary_offsets`, the SAME function
    // `run_mechanism_a_backfill_split` must eventually cross-check its
    // caller-supplied offsets against once F-001 is fixed.
    let row1 = "| D-001 | decided one | author |\n";
    let row2 = "| D-002 | decided two | author |\n";
    let row3 = "| D-003 | decided six | author |\n";
    let content = format!("{row1}{row2}{row3}");
    let original_content = content.into_bytes();

    let true_offsets = mechanism_a_record_boundary_offsets("decision-log", &original_content);
    assert_eq!(
        true_offsets.len(),
        3,
        "test fixture precondition: exactly 3 genuine `| D-` row boundaries must be \
         independently detectable in this content. Got: {true_offsets:?}"
    );

    // STALE/WRONG caller-supplied offsets: well-formed (strictly ascending,
    // in-bounds -- passes MED-C's `record_boundary_offsets_are_well_formed`
    // check) but SILENTLY MISSING the real middle boundary
    // (`true_offsets[1]`, row2's own start) -- exactly the "offsets that
    // miss a real boundary, or are stale vs the content" scenario F-001
    // names.
    let stale_offsets = vec![true_offsets[0], true_offsets[2]];
    assert!(
        stale_offsets.windows(2).all(|pair| pair[0] < pair[1])
            && stale_offsets
                .last()
                .is_some_and(|&last| last < original_content.len()),
        "test fixture precondition: the stale offsets must themselves be well-formed (ascending, \
         in-bounds) -- this test targets the INDEPENDENT-recomputation gap, not the already-fixed \
         MED-C well-formedness gap"
    );

    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    std::fs::write(&canonical_path, &original_content).unwrap();

    // A cap small enough that row1+row2 merged (as the stale offsets would
    // have it) exceeds cap on its own -- forcing an actual durable write
    // (oversized-record seal + canonical truncation) if the bug silently
    // "succeeds," which is exactly what must NOT happen once F-001 is
    // fixed.
    let entry = flat_entry("decision-log", 40);

    let outcome = run_mechanism_a_backfill_split(&entry, &canonical_path, &stale_offsets, 10);

    assert!(
        matches!(
            outcome,
            Err(MechanismABackfillError::ContentPreservationFailed { .. })
        ),
        "F-001/Postcondition 6/EC-004/EC-006: a `record_boundary_offsets` argument that silently \
         omits a real record boundary present in the actual on-disk content MUST abort with \
         E-SHD-003 (`ContentPreservationFailed`), never succeed. Got: {outcome:?}"
    );

    let post_content = std::fs::read(&canonical_path).unwrap();
    assert_eq!(
        post_content, original_content,
        "F-001/Postcondition 6: on abort, the original monolithic file MUST be left completely \
         untouched (fail-loud, never partial-and-silent) -- got a file that no longer matches the \
         pre-call original content, meaning the buggy path silently sealed/truncated despite the \
         missing real boundary"
    );
    assert!(
        !dir.path().join("decision-log.0001.md").exists(),
        "F-001/Postcondition 5: on abort, no sealed shard file may have been durably written"
    );
}

// ---------------------------------------------------------------------------
// F-004 (BC-1.18.008 v1.3 fix-burst, MEDIUM): Canonical Test Vectors
// correction. Three rows in BC-1.18.008's Canonical Test Vectors table were
// marked NEEDS-UPDATE: they previously asserted the EXACT `ceil(bytes/cap)`
// value (19 for `decision-log.md`, 5 for `lessons.md`, plus the dependent
// interrupted-restart row) as if `ceil()` were the exact resulting shard
// count. Postcondition 2 (v1.3) now documents `ceil()` as a LOWER BOUND
// only -- the exact count is whatever the greedy boundary-preserving packer
// actually produces for the real record-size distribution, which is
// `>= ceil()` and equal to it only when records happen to pack without
// slack.
//
// Per BC-1.18.008 Precondition 3/EC-005, the real F2-era byte counts
// (908,938 / 234,731) are explicitly illustrative-scale-only, NOT literal
// test inputs -- exact re-measurement at F4 execution time is mandated
// instead. Reproducing an exact 908,938-byte real `decision-log.md` inline
// in a unit test is neither possible (the real file's content is not a
// fixed artifact of this codebase) nor spec-required. The three tests below
// instead use independently hand-verified, byte-faithful SYNTHETIC fixtures
// at the SAME production `shard_cap_bytes` (49,152) that deliberately
// reproduce the ORIGINAL placeholder counts (19, 5) as the fixtures'
// genuinely-computed ACTUAL packed-shard counts -- while proving those
// counts are NOT equal to `ceil(fixture_bytes/49152)` for these fixtures,
// which is the exact defect the v1.3 amendment corrects. Every expected
// count below is derived by hand arithmetic in each test's own doc comment,
// never by calling the packer once and pinning whatever it returns
// (POLICY 11 no-tautology).
// ---------------------------------------------------------------------------

/// One `size`-byte synthetic "record" (a single repeated fill byte derived
/// from `idx`, purely for at-a-glance debuggability on assertion failure --
/// the byte VALUE carries no semantic meaning to the partitioner).
fn sized_record(idx: usize, size: usize) -> Vec<u8> {
    let fill = b'A' + ((idx % 26) as u8);
    vec![fill; size]
}

/// `count` consecutive `size`-byte synthetic records, concatenated in
/// order -- record `i` therefore starts at byte offset `i * size`.
fn concat_sized_records(count: usize, size: usize) -> Vec<u8> {
    (0..count).flat_map(|i| sized_record(i, size)).collect()
}

/// The trivially-known record-boundary offsets (`i * size`) for
/// [`concat_sized_records`]'s own output -- independent of (and never
/// exercising) this module's separately-tested
/// `mechanism_a_record_boundary_offsets` marker-detection logic, exactly
/// like this file's existing `concat_records`/fixed-boundary fixture
/// convention above.
fn boundary_offsets_for(count: usize, size: usize) -> Vec<usize> {
    (0..count).map(|i| i * size).collect()
}

#[test]
fn test_BC_1_18_008_F004_run_backfill_split_decision_log_canonical_vector_actual_exceeds_ceil_lower_bound()
 {
    // Hand arithmetic (shown, not derived from the implementation under
    // test):
    //   shard_cap_bytes = 49,152 (real production cap, BC-1.18.008
    //     Precondition 3's own cited value).
    //   RECORD_SIZE = cap/2 + 1 = 24,577 bytes -- chosen so that exactly
    //     ONE such record fits under cap (24,577 <= 49,152, 24,575 bytes of
    //     slack remain) but a SECOND record can never be added to the same
    //     shard: 24,577 * 2 = 49,154 > 49,152.
    //   RECORD_COUNT = 19 -- reproducing the SUPERSEDED vector's own
    //     placeholder number, but now as the genuinely-computed ACTUAL
    //     count rather than an asserted `ceil()` estimate.
    //   total bytes = 19 * 24,577 = 466,963.
    //   ceil(466,963 / 49,152) = ceil(9.5007...) = 10  <-- LOWER BOUND only
    //     (49,152 * 9 = 442,368 < 466,963 <= 49,152 * 10 = 491,520).
    //   greedy packing: since no two records can ever share a shard (as
    //     established above), the packer seals after every single record
    //     -- ACTUAL packed count = 19, one partition per record. 18 of the
    //     19 partitions are sealed (seq 1..=18); the 19th (last) becomes
    //     the fresh current file (Postcondition 2).
    //   19 > 10 confirms Postcondition 2's `actual_count >=
    //   ceil(bytes/cap)` inequality holds with STRICT inequality here --
    //   exactly the shape of PC2's own five-40-byte-records-vs-70-byte-cap
    //   worked example, at a decision-log-scaled magnitude.
    const RECORD_SIZE: usize = 24_577;
    const RECORD_COUNT: usize = 19;
    const CAP: u64 = 49_152;

    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    let original_content = concat_sized_records(RECORD_COUNT, RECORD_SIZE);
    assert_eq!(
        original_content.len(),
        466_963,
        "arithmetic precondition: 19 * 24,577 must equal 466,963"
    );
    std::fs::write(&canonical_path, &original_content).unwrap();

    let boundaries = boundary_offsets_for(RECORD_COUNT, RECORD_SIZE);
    let entry = flat_entry("decision-log", CAP);

    let outcome = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 100)
        .expect("F-004: a well-formed oversized-artifact backfill must succeed");

    assert_eq!(
        outcome,
        MechanismABackfillOutcome::Migrated {
            sealed_count: 18,
            archived_count: 0
        },
        "F-004 (Canonical Test Vectors, decision-log.md row, NEEDS-UPDATE): the ACTUAL \
         greedy-packed count for this hand-verified fixture is 19 total partitions (18 sealed + \
         1 fresh current) -- NOT ceil(466963/49152) = 10, which Postcondition 2 documents as a \
         lower bound only. See this test's own doc comment for the full hand arithmetic."
    );

    // Postcondition 6(a), end-to-end across the real files this call
    // produced: every sealed shard is exactly one whole record, and the
    // full reconstruction reproduces the original content byte-for-byte.
    let mut reconstructed = Vec::new();
    for seq in 1..=18u32 {
        let sealed_path = dir.path().join(format!("decision-log.{seq:04}.md"));
        let sealed_bytes = std::fs::read(&sealed_path)
            .unwrap_or_else(|e| panic!("F-004: sealed shard seq={seq} must exist on disk: {e}"));
        assert_eq!(
            sealed_bytes.len(),
            RECORD_SIZE,
            "F-004: sealed shard seq={seq} must hold exactly one {RECORD_SIZE}-byte record"
        );
        reconstructed.extend_from_slice(&sealed_bytes);
    }
    let current_bytes = std::fs::read(&canonical_path).unwrap();
    assert_eq!(current_bytes.len(), RECORD_SIZE);
    reconstructed.extend_from_slice(&current_bytes);
    assert_eq!(
        reconstructed, original_content,
        "Postcondition 6(a): 18 sealed shards + fresh current file must reproduce the original \
         content byte-for-byte"
    );
}

#[test]
fn test_BC_1_18_008_F004_run_backfill_split_lessons_canonical_vector_actual_exceeds_ceil_lower_bound()
 {
    // Hand arithmetic (shown, not derived from the implementation under
    // test):
    //   shard_cap_bytes = 49,152 (real production cap).
    //   RECORD_SIZE = cap/2 + 1 = 24,577 bytes (same forced
    //     one-record-per-shard construction as the decision-log vector
    //     above: 24,577 * 2 = 49,154 > 49,152, so no two records ever
    //     share a shard).
    //   RECORD_COUNT = 5 -- reproducing the SUPERSEDED lessons.md vector's
    //     own placeholder number, now as the genuinely-computed ACTUAL
    //     count.
    //   total bytes = 5 * 24,577 = 122,885.
    //   ceil(122,885 / 49,152) = ceil(2.500...) = 3  <-- LOWER BOUND only
    //     (49,152 * 2 = 98,304 < 122,885 <= 49,152 * 3 = 147,456).
    //   greedy packing: one record per shard (established above) -- ACTUAL
    //     packed count = 5. 4 of the 5 partitions are sealed (seq 1..=4);
    //     the 5th (last) becomes the fresh current file.
    //   5 > 3 confirms the `actual_count >= ceil(bytes/cap)` inequality
    //   with strict inequality for this fixture too.
    const RECORD_SIZE: usize = 24_577;
    const RECORD_COUNT: usize = 5;
    const CAP: u64 = 49_152;

    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("lessons.md");
    let original_content = concat_sized_records(RECORD_COUNT, RECORD_SIZE);
    assert_eq!(
        original_content.len(),
        122_885,
        "arithmetic precondition: 5 * 24,577 must equal 122,885"
    );
    std::fs::write(&canonical_path, &original_content).unwrap();

    let boundaries = boundary_offsets_for(RECORD_COUNT, RECORD_SIZE);
    let entry = flat_entry("lessons", CAP);

    let outcome = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 100)
        .expect("F-004: a well-formed oversized-artifact backfill must succeed");

    assert_eq!(
        outcome,
        MechanismABackfillOutcome::Migrated {
            sealed_count: 4,
            archived_count: 0
        },
        "F-004 (Canonical Test Vectors, lessons.md row, NEEDS-UPDATE): the ACTUAL greedy-packed \
         count for this hand-verified fixture is 5 total partitions (4 sealed + 1 fresh current) \
         -- NOT ceil(122885/49152) = 3, which Postcondition 2 documents as a lower bound only. \
         See this test's own doc comment for the full hand arithmetic."
    );

    let mut reconstructed = Vec::new();
    for seq in 1..=4u32 {
        let sealed_path = dir.path().join(format!("lessons.{seq:04}.md"));
        let sealed_bytes = std::fs::read(&sealed_path)
            .unwrap_or_else(|e| panic!("F-004: sealed shard seq={seq} must exist on disk: {e}"));
        assert_eq!(sealed_bytes.len(), RECORD_SIZE);
        reconstructed.extend_from_slice(&sealed_bytes);
    }
    let current_bytes = std::fs::read(&canonical_path).unwrap();
    assert_eq!(current_bytes.len(), RECORD_SIZE);
    reconstructed.extend_from_slice(&current_bytes);
    assert_eq!(
        reconstructed, original_content,
        "Postcondition 6(a): 4 sealed shards + fresh current file must reproduce the original \
         content byte-for-byte"
    );
}

#[test]
fn test_BC_1_18_008_F004_run_backfill_split_interrupted_restart_reproduces_same_actual_count() {
    // Canonical Test Vectors, "Backfill interrupted after 3/N shards
    // written" row (NEEDS-UPDATE, count only -- behavior unchanged): N is
    // the ACTUAL greedy-packed count for decision-log.md, which the
    // sibling F-004 decision-log test above hand-verifies as 19 (18 sealed
    // + 1 current), NOT the superseded ceil()-only placeholder. This test
    // reuses that SAME 19-record fixture and simulates a crash after 3 of
    // the expected 18 sealed shard files were durably written (a stale,
    // incomplete leftover at each of the first 3 destination `seq` paths,
    // with NO shard-index present -- the crash happened before the
    // operation ever durably completed, so
    // `mechanism_a_backfill_already_migrated` correctly reports `false`),
    // then asserts a from-scratch restart against the still-untouched
    // original monolithic file produces the IDENTICAL 19-total-partition
    // result an uninterrupted run would have (Postcondition 5's own
    // determinism guarantee: the greedy packer is a pure function of the
    // original content and `shard_cap_bytes`).
    const RECORD_SIZE: usize = 24_577;
    const RECORD_COUNT: usize = 19;
    const CAP: u64 = 49_152;

    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    let original_content = concat_sized_records(RECORD_COUNT, RECORD_SIZE);
    std::fs::write(&canonical_path, &original_content).unwrap();

    // Stale, WRONG leftovers at the first 3 destination seq paths from a
    // simulated crashed prior attempt -- no shard-index alongside them.
    for seq in 1..=3u32 {
        std::fs::write(
            dir.path().join(format!("decision-log.{seq:04}.md")),
            b"stale garbage from a crash",
        )
        .unwrap();
    }

    let boundaries = boundary_offsets_for(RECORD_COUNT, RECORD_SIZE);
    let entry = flat_entry("decision-log", CAP);

    let outcome = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 100).expect(
        "EC-003/Postcondition 5: a from-scratch restart over a stale, incomplete prior attempt \
         must succeed, not be blocked or corrupted by the stray leftovers",
    );

    assert_eq!(
        outcome,
        MechanismABackfillOutcome::Migrated {
            sealed_count: 18,
            archived_count: 0
        },
        "F-004 (interrupted-restart row, NEEDS-UPDATE — count only, behavior unchanged): the \
         restart must produce the SAME 19-total-partition (18 sealed + 1 current) result an \
         uninitialized run would have produced -- deterministic, since the greedy packer is a \
         pure function of the original content and shard_cap_bytes"
    );

    let mut reconstructed = Vec::new();
    for seq in 1..=18u32 {
        let sealed_bytes =
            std::fs::read(dir.path().join(format!("decision-log.{seq:04}.md"))).unwrap();
        assert_ne!(
            sealed_bytes, b"stale garbage from a crash",
            "EC-003/Postcondition 5: the restart must OVERWRITE every stale leftover with the \
             correct, freshly-computed sealed content -- seq={seq}"
        );
        reconstructed.extend_from_slice(&sealed_bytes);
    }
    reconstructed.extend_from_slice(&std::fs::read(&canonical_path).unwrap());
    assert_eq!(
        reconstructed, original_content,
        "EC-003/Postcondition 6(a): the restart's real output must reproduce the original \
         monolithic content byte-for-byte, exactly as an uninitialized run would"
    );
}

// ---------------------------------------------------------------------------
// P2-001 (S-25.02 F4 cluster-3 adversarial pass-2, HIGH): Postcondition
// 6(b)'s record-count gate must catch OVER-detection, not just
// UNDER-detection.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_008_P2001_run_backfill_split_aborts_when_supplied_offsets_add_a_spurious_mid_record_boundary()
 {
    // This test pins Postcondition 6(b)/Invariant 2/EC-006: a
    // caller-supplied `record_boundary_offsets` argument that is
    // well-formed (strictly ascending, in-bounds) but adds a SPURIOUS extra
    // offset landing MID-RECORD -- at a byte position the Record-Boundary
    // Marker Table explicitly documents as NOT a boundary (here, a nested
    // `### Block N:` sub-heading inside a real `burst-log.md` record) --
    // must ABORT with `E-SHD-003`, never silently succeed and physically
    // seal the real record across two separate shard files.
    //
    // Provenance: a fresh-context adversarial pass-2 review found F-001's
    // own fix (see `test_BC_1_18_008_F001_*` above) derives
    // `original_record_count` from `record_boundary_offsets ∪
    // true_boundary_offsets` (the union of the caller's list and this
    // module's own independently-detected true boundaries) -- which catches
    // UNDER-detection (a caller list missing a real boundary, F-001's own
    // scenario) but structurally cannot catch OVER-detection: whenever the
    // caller's list is a SUPERSET of the true boundaries (every real
    // boundary already present, plus one spurious extra), the union
    // contributes nothing new (`|caller ∪ oracle| == |caller|`), and
    // `mechanism_a_partition_for_backfill`'s own `record_count` sum, by
    // construction, always totals exactly `record_boundary_offsets.len()`
    // too -- so both sides of the Postcondition 6(b) comparison are, in
    // this shape, always equal, independent of whether the spurious offset
    // actually caused a real record to be split across two physical shard
    // files. This fixture chooses `shard_cap_bytes` small enough that the
    // spurious offset DOES cause exactly that: burst-log's real "Burst A"
    // record (the oracle-true span from `## Burst A` to `## Burst B`) ends
    // up sealed across two separate shard files, split at the spurious
    // `### Block 3:` byte offset -- a nested sub-heading the marker table's
    // own "NOT a boundary" column names for this artifact.
    let h2_a = "## Burst A\n";
    let filler_a1 = format!("{}\n", "x".repeat(20));
    let block3_decoy = "### Block 3: Codifications\n";
    let filler_a2 = format!("{}\n", "y".repeat(20));
    let h2_b = "## Burst B\n";
    let filler_b = format!("{}\n", "z".repeat(20));

    let content = format!("{h2_a}{filler_a1}{block3_decoy}{filler_a2}{h2_b}{filler_b}");

    let offset_h2_a = content.find(h2_a).unwrap();
    let offset_block3 = content.find(block3_decoy).unwrap();
    let offset_h2_b = content.find(h2_b).unwrap();

    let original_content = content.into_bytes();

    let true_offsets = mechanism_a_record_boundary_offsets("burst-log", &original_content);
    assert_eq!(
        true_offsets,
        vec![offset_h2_a, offset_h2_b],
        "test fixture precondition: the oracle must find exactly the 2 real h2 boundaries, \
         never the nested `### Block 3:` decoy. Got: {true_offsets:?}"
    );

    // Caller-supplied offsets: well-formed (ascending, in-bounds) but a
    // SUPERSET of the true boundaries -- includes the spurious block3_decoy
    // offset the marker table documents as NOT a boundary.
    let spurious_offsets = vec![offset_h2_a, offset_block3, offset_h2_b];
    assert!(
        spurious_offsets.windows(2).all(|pair| pair[0] < pair[1])
            && spurious_offsets
                .last()
                .is_some_and(|&last| last < original_content.len()),
        "test fixture precondition: the spurious offsets must themselves be well-formed \
         (ascending, in-bounds) -- this test targets the OVER-detection gap, not the \
         already-fixed well-formedness gap"
    );

    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("burst-log.md");
    std::fs::write(&canonical_path, &original_content).unwrap();

    // Cap chosen so the first chunk (`## Burst A` through the spurious
    // `### Block 3:` offset) exactly fills the cap, forcing a flush before
    // the rest of the real "Burst A" record is processed -- guaranteeing
    // the real record ends up sealed across (at least) two separate shard
    // files, split at the spurious offset.
    let cap = (offset_block3 - offset_h2_a) as u64;
    let entry = flat_entry("burst-log", cap);

    let outcome = run_mechanism_a_backfill_split(&entry, &canonical_path, &spurious_offsets, 10);

    assert!(
        matches!(
            outcome,
            Err(MechanismABackfillError::ContentPreservationFailed { .. })
        ),
        "P2-001/Postcondition 6(b)/Invariant 2/EC-006: a caller-supplied \
         `record_boundary_offsets` argument that is well-formed but adds a SPURIOUS mid-record \
         offset (landing on a real record's own nested, non-boundary sub-structure) MUST abort \
         with E-SHD-003 -- the real 'Burst A' record would otherwise be sealed across two \
         physical shard files. Got: {outcome:?}"
    );

    let post_content = std::fs::read(&canonical_path).unwrap();
    assert_eq!(
        post_content, original_content,
        "P2-001/Postcondition 6: on abort, the original monolithic file MUST be left completely \
         untouched (fail-loud, never partial-and-silent)"
    );
    assert!(
        !dir.path().join("burst-log.0001.md").exists(),
        "P2-001/Postcondition 5: on abort, no sealed shard file may have been durably written"
    );
}

// ---------------------------------------------------------------------------
// P2-002 (S-25.02 F4 cluster-3 adversarial pass-2, HIGH): the shard-index
// entry for an EC-002 oversized single record must itself carry the
// `oversized_record` flag.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_008_P2002_run_backfill_split_shard_index_entry_carries_oversized_record_flag() {
    // This test pins EC-002's Canonical Test Vector: "a single 60,000-byte
    // decision-log row (exceeds 49,152-byte cap alone) ... flagged
    // `oversized_record: true`" -- specifically, that the flag is visible
    // in the PUBLISHED `<artifact-stem>.shard-index.toml` shard entry
    // itself, not merely inferable from the sealed shard's own byte size
    // exceeding `shard_cap_bytes`.
    //
    // Provenance: `mechanism_a_partition_for_backfill` already computes
    // `oversized_record: true` on the internal `MechanismABackfillPartition`
    // for a single over-cap record (see the EC-002/EC-017 unit tests
    // above), but a fresh-context adversarial pass-2 review found
    // `run_mechanism_a_backfill_split` builds each `ShardIndexEntry`
    // WITHOUT carrying that flag through. This test reads the published
    // shard-index back as a generic `toml::Value` -- rather than the
    // strongly-typed `ShardIndex`/`ShardIndexEntry` structs this file's
    // other tests use -- specifically so it does not itself assume the
    // field already exists on those structs, and instead inspects the
    // actual published TOML content directly.
    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    let record1 = [b'A'; 100]; // oversized (> cap)
    let record2 = [b'B'; 20]; // normal, becomes the fresh current
    let original_content: Vec<u8> = record1.iter().chain(record2.iter()).copied().collect();
    std::fs::write(&canonical_path, &original_content).unwrap();

    let entry = flat_entry("decision-log", 50);
    let outcome = run_mechanism_a_backfill_split(&entry, &canonical_path, &[0, 100], 10)
        .expect("EC-002/EC-017: a backfill containing one oversized record must still succeed");
    assert_eq!(
        outcome,
        MechanismABackfillOutcome::Migrated {
            sealed_count: 1,
            archived_count: 0
        },
        "test fixture precondition: the oversized record must seal as its own single shard"
    );

    let index_toml =
        std::fs::read_to_string(dir.path().join("decision-log.shard-index.toml")).unwrap();
    let index_value: toml::Value =
        toml::from_str(&index_toml).expect("the shard-index TOML must parse as generic TOML");
    let shards = index_value
        .get("shard")
        .and_then(|v| v.as_array())
        .expect("the shard-index must publish a [[shard]] array");
    assert_eq!(
        shards.len(),
        1,
        "test fixture precondition: exactly one sealed [[shard]] entry for this fixture"
    );
    let sealed_entry = &shards[0];
    assert_eq!(
        sealed_entry.get("seq").and_then(toml::Value::as_integer),
        Some(1),
        "sanity: the sealed entry is seq=1"
    );

    let oversized_flag = sealed_entry
        .get("oversized_record")
        .and_then(toml::Value::as_bool);
    assert_eq!(
        oversized_flag,
        Some(true),
        "P2-002/EC-002 Canonical Test Vector: the published shard-index entry for a single \
         over-cap record must carry `oversized_record: true` -- got shard entry: \
         {sealed_entry:?}"
    );
}

// ---------------------------------------------------------------------------
// P2-003 (S-25.02 F4 cluster-3 adversarial pass-2, MEDIUM): the cap
// accounting that decides when to flush a partition must include any
// leading preamble bytes folded into that partition's own content -- not
// just the whole-record bytes accumulated after BLOCKER-1's fix.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_008_P2003_partition_for_backfill_first_partition_counts_preamble_bytes_toward_cap()
{
    // This test pins Postcondition 2's per-partition cap bound
    // (`<= shard_cap_bytes`, EC-002's oversized-single-record exception
    // aside): every returned partition's OWN byte length must respect
    // `shard_cap_bytes`, including the FIRST partition when a leading
    // preamble precedes the first record boundary.
    //
    // Provenance: BLOCKER-1's fix folds any leading preamble bytes
    // (`content[0..record_boundary_offsets[0])`) into whichever partition
    // ends up holding the first record, so no byte is ever dropped
    // (Postcondition 6(a)). A fresh-context adversarial pass-2 review found
    // that fix incomplete on the CAP side: `mechanism_a_partition_for_backfill`'s
    // own `partition_bytes` cap-accounting accumulator starts at 0 and is
    // only ever incremented by each whole RECORD's own length (`rec_len`)
    // -- it never accounts for the preamble bytes physically folded into
    // the partition's `bytes` field. A 33-byte preamble followed by five
    // 30-byte records against a 70-byte cap can therefore flush a FIRST
    // partition whose real on-disk byte length is 93 bytes (33-byte
    // preamble + two 30-byte records), silently exceeding the 70-byte cap,
    // while the internal accounting variable that decided when to flush
    // only ever saw 60 (two records' worth), never noticing the preamble.
    let preamble = b"x".repeat(33); // a real, non-record leading preamble
    let records = concat_records(&[1, 2, 3, 4, 5]); // five 30-byte records, 150 bytes
    let mut content = preamble.clone();
    content.extend_from_slice(&records);

    let boundaries: Vec<usize> = [0usize, 30, 60, 90, 120]
        .iter()
        .map(|o| o + preamble.len())
        .collect();
    assert!(
        boundaries[0] > 0,
        "test fixture precondition: a real leading preamble before the first record boundary"
    );

    let cap = 70u64;
    let partitions = mechanism_a_partition_for_backfill(&content, &boundaries, cap);

    for (i, p) in partitions.iter().enumerate() {
        assert!(
            p.oversized_record || p.bytes.len() as u64 <= cap,
            "P2-003 (Postcondition 2, preamble byte-counting): partition {i} is {} bytes, \
             exceeding shard_cap_bytes ({cap}) WITHOUT being flagged `oversized_record` -- no \
             record in this fixture is individually oversized, so the preamble bytes before the \
             first record boundary must count toward the FIRST partition's own cap accounting. \
             Partition {i}: {p:?}",
            p.bytes.len()
        );
    }
}

#[test]
fn test_BC_1_18_008_P2003_run_backfill_split_sealed_shards_never_exceed_cap_when_preamble_present()
{
    // End-to-end counterpart to the unit-level test above, driven through
    // the real, on-disk `run_mechanism_a_backfill_split` entry point: pins
    // Postcondition 2's per-shard cap bound against every REAL sealed
    // shard file (and the fresh current file) this call produces, for a
    // fixture with a real leading preamble and no individually-oversized
    // record.
    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");

    let preamble = b"x".repeat(33);
    let records = concat_records(&[1, 2, 3, 4, 5]); // 150 bytes, 5 records
    let mut original_content = preamble.clone();
    original_content.extend_from_slice(&records);
    std::fs::write(&canonical_path, &original_content).unwrap();

    let boundaries: Vec<usize> = [0usize, 30, 60, 90, 120]
        .iter()
        .map(|o| o + preamble.len())
        .collect();

    let cap = 70u64;
    let entry = flat_entry("decision-log", cap);

    let outcome = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 10)
        .expect("P2-003: a well-formed oversized-artifact-with-preamble backfill must succeed");

    let MechanismABackfillOutcome::Migrated { sealed_count, .. } = outcome else {
        panic!("P2-003: expected a Migrated outcome for this fixture, got {outcome:?}");
    };

    for seq in 1..=sealed_count {
        let sealed_path = dir.path().join(format!("decision-log.{seq:04}.md"));
        let sealed_bytes = std::fs::read(&sealed_path)
            .unwrap_or_else(|e| panic!("P2-003: sealed shard seq={seq} must exist: {e}"));
        assert!(
            sealed_bytes.len() as u64 <= cap,
            "P2-003/Postcondition 2: sealed shard seq={seq} is {} bytes, exceeding \
             shard_cap_bytes ({cap}) -- no record in this fixture is individually oversized, so \
             every sealed shard must stay within cap; the leading {}-byte preamble must count \
             toward the FIRST partition's own cap accounting.",
            sealed_bytes.len(),
            preamble.len()
        );
    }
    let current_bytes = std::fs::read(&canonical_path).unwrap();
    assert!(
        current_bytes.len() as u64 <= cap,
        "P2-003/Postcondition 2: the fresh current file ({} bytes) must not exceed \
         shard_cap_bytes ({cap})",
        current_bytes.len()
    );
}
