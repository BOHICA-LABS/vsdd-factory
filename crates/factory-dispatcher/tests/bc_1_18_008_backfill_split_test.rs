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
//! `mechanism_a_verify_backfill_per_shard_cap_preserved` (BC-1.18.008 v1.4's
//! Postcondition 6(c)/Invariant 4 per-shard-cap hard gate — this file's tests
//! pin its `true` outcome for a partition that respects `shard_cap_bytes` and
//! its `false` outcome for one that violates it, at both the unit level and
//! end-to-end through `run_mechanism_a_backfill_split`),
//! `mechanism_a_verify_backfill_record_counts_preserved`,
//! `mechanism_a_backfill_already_migrated`, `run_mechanism_a_backfill_split`)
//! is a real, fully implemented body (no `todo!()` stubs) once GREEN. This file's tests
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
    ShardIndex, ShardShape, mechanism_a_backfill_already_migrated,
    mechanism_a_partition_for_backfill, mechanism_a_record_boundary_offsets,
    mechanism_a_verify_backfill_content_preserved,
    mechanism_a_verify_backfill_per_shard_cap_preserved,
    mechanism_a_verify_backfill_record_counts_preserved, mechanism_a_write_and_verify_sealed_shard,
    run_mechanism_a_backfill_split,
};
use std::path::Path;

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
    // row, never themselves a primary shard-boundary. This test pins that
    // exclusion (the `| D-` marker must never match a `### ` or `## Appendix`
    // line) as permanent regression coverage for the marker table's
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
// Statement 1) -- RETIRED, this burst (BC-1.18.008 v1.6 F-C3-P6-001).
//
// The test formerly here hand-constructed a shard-index fixture (via a
// literal `ShardIndex { .. }` with NO `[backfill_manifest]` table) to
// simulate the crash window between the index-publish and canonical-
// truncate writes, then asserted that re-running the backfill self-heals by
// completing the truncation. That self-heal disposition is still correct
// behavior -- but the MECHANISM it exercised is not: pre-v1.6,
// `heal_or_confirm_already_migrated` decided SAFE-vs-DANGEROUS via a
// structural byte-prefix comparison against the already-sealed shards'
// concatenation. BC-1.18.008 v1.6's Recovery-Confirmation Rule (Postcondition
// 5) RETIRES that byte-prefix heuristic entirely -- Invariant 3 now
// explicitly requires the determination to rest SOLELY on an exact
// whole-file `(length, SHA-256)` comparison against the Backfill Recovery
// Manifest's `original_bytes`/`original_sha256`/`final_bytes`/`final_sha256`
// fields, "NEVER a structural byte-prefix comparison." The shipped v1.6
// implementation (`heal_or_confirm_already_migrated`) now requires that
// manifest to exist at all: a shard-index with no `[backfill_manifest]`
// table (exactly what this test's hand-built fixture produced) makes
// `read_backfill_manifest` return `None`, which the function maps to
// `Err(MechanismABackfillError::MissingBackfillManifest)` (the `E-SHD-011`
// fail-loud path) rather than proceeding to heal -- so this test's own
// `assert!(result.is_ok(), ...)` now asserts behavior BC-1.18.008 v1.6
// explicitly forbids for a no-manifest fixture, not a residual gap.
//
// Retiring rather than patching: adding a `[backfill_manifest]` table to
// this test's hand-built fixture would only re-derive
// `test_BC_1_18_008_FC3P6001_run_backfill_split_second_invocation_heals_genuine_crash_window`
// below (same crash window, same DANGEROUS disposition, same self-heal
// assertion) by manual construction instead of the real API -- strictly
// worse coverage, since a hand-built manifest can silently drift out of
// sync with whatever `run_mechanism_a_backfill_split` actually writes,
// whereas the FC3P6001 test below produces its manifest via a REAL first
// invocation of the production split function itself, then genuinely
// restores the canonical file to simulate the crash. The identical
// crash-window property (self-heal completes the interrupted
// canonical-truncate, whole-corpus reconstruction holds, no duplicate
// shard-index entries) is verified there, end-to-end, through the real,
// v1.6-compliant manifest-based path. The crash-window property is not
// uncovered -- it has a stronger, v1.6-compliant pin than this retired test
// ever provided.
// ---------------------------------------------------------------------------

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

/// One `total_len`-byte, ORACLE-DETECTABLE synthetic `decision-log.md`
/// record: a real `"| D-NNN | ... |"` table row that matches
/// `mechanism_a_record_boundary_offsets`'s own `^\| D-[0-9]+ \|` marker
/// regex exactly (via `is_decision_log_row_marker`), padded with a
/// repeated `'A'` fill byte to hit `total_len` precisely. `idx` (1-based,
/// zero-padded to 3 digits) is folded into the row's own `D-NNN` id so no
/// two rows in the same fixture collide.
///
/// P3-002 (MEDIUM, S-25.02 F4 cluster-3 adversarial pass-3): this REPLACES
/// the prior `sized_record`/`concat_sized_records` fill-byte fixture
/// (below, superseded) that a fresh-context adversarial pass-3 review found
/// rode the "oracle finds ZERO markers -> trust the caller's offsets at
/// face value" fallback in `run_mechanism_a_backfill_split` rather than
/// exercising the REAL `mechanism_a_record_boundary_offsets` oracle
/// cross-check (F-001/P2-001's own load-bearing gate) at all -- every
/// F-004 test below now drives genuinely oracle-matched content instead.
///
/// Hand-verified byte-length arithmetic (not derived from calling this
/// function and pinning whatever it returns, POLICY 11): the fixed
/// (non-padding) overhead is exactly 13 bytes --
/// `"| D-" + "NNN" (3 digits) + " | "` = `4 + 3 + 3 = 10` bytes of prefix,
/// plus `" |\n"` = `3` bytes of suffix, `10 + 3 = 13`. `pad_len =
/// total_len - 13`.
fn decision_log_row_of_len(idx: usize, total_len: usize) -> Vec<u8> {
    let prefix = format!("| D-{idx:03} | ");
    let suffix = " |\n";
    let pad_len = total_len
        .checked_sub(prefix.len() + suffix.len())
        .expect("total_len must be large enough to hold the D-NNN row's own prefix+suffix");
    let mut row = prefix.into_bytes();
    row.extend_from_slice(&b"A".repeat(pad_len));
    row.extend_from_slice(suffix.as_bytes());
    row
}

/// `count` consecutive `row_len`-byte oracle-detectable
/// [`decision_log_row_of_len`] records (1-based `D-NNN` ids), concatenated
/// in order -- record `i` (0-based) therefore starts at byte offset
/// `i * row_len`, exactly like the superseded fill-byte
/// `concat_sized_records` fixture's own offset arithmetic.
fn concat_decision_log_rows(count: usize, row_len: usize) -> Vec<u8> {
    (0..count)
        .flat_map(|i| decision_log_row_of_len(i + 1, row_len))
        .collect()
}

/// One `total_len`-byte, ORACLE-DETECTABLE synthetic `lessons.md` record: a
/// real `"## L-EDP1-NNN ..."` ID-tagged h2 heading that matches
/// `mechanism_a_record_boundary_offsets`'s own primary `lessons.md` marker
/// exactly (via `is_lesson_h2_record_heading`/`is_id_tagged_lesson_heading`),
/// padded with a repeated `'A'` fill byte to hit `total_len` precisely.
///
/// Hand-verified byte-length arithmetic: the fixed (non-padding) overhead
/// is exactly 15 bytes -- `"## " + "L-EDP1-" + "NNN" (3 digits) + " "` =
/// `3 + 7 + 3 + 1 = 14` bytes of prefix, plus the trailing `'\n'` = `1`
/// byte, `14 + 1 = 15`. `pad_len = total_len - 15`.
fn lessons_h2_row_of_len(idx: usize, total_len: usize) -> Vec<u8> {
    let prefix = format!("## L-EDP1-{idx:03} ");
    let pad_len = total_len
        .checked_sub(prefix.len() + 1)
        .expect("total_len must be large enough to hold the L-EDP1-NNN heading's own prefix");
    let mut row = prefix.into_bytes();
    row.extend_from_slice(&b"A".repeat(pad_len));
    row.push(b'\n');
    row
}

/// `count` consecutive `row_len`-byte oracle-detectable
/// [`lessons_h2_row_of_len`] records (1-based `L-EDP1-NNN` ids),
/// concatenated in order -- record `i` (0-based) starts at byte offset
/// `i * row_len`.
fn concat_lessons_rows(count: usize, row_len: usize) -> Vec<u8> {
    (0..count)
        .flat_map(|i| lessons_h2_row_of_len(i + 1, row_len))
        .collect()
}

/// The trivially-known record-boundary offsets (`i * size`) for
/// [`concat_decision_log_rows`]/[`concat_lessons_rows`]'s own
/// fixed-record-length output -- independent of (and never exercising)
/// this module's separately-tested `mechanism_a_record_boundary_offsets`
/// boundary-detection logic itself, exactly like this file's existing
/// `concat_records`/fixed-boundary fixture convention above. (The two
/// builders above are independently ALSO oracle-detectable -- every F-004
/// test below cross-checks this hand-computed offsets list against the
/// real oracle's own output before using it, so this function's role is
/// purely "the hand-derived expectation", never the sole source of truth.)
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
    //
    // P3-002 (MEDIUM, adversarial pass-3): REBUILT from the prior fill-byte
    // `sized_record`/`concat_sized_records` fixture (which used a
    // `decision-log` stem but content the real oracle never matches, so the
    // test rode `run_mechanism_a_backfill_split`'s "oracle finds ZERO
    // markers -> trust the caller's offsets" fallback rather than the real
    // `mechanism_a_record_boundary_offsets` cross-check) to use
    // `decision_log_row_of_len` -- genuine, oracle-detectable `| D-NNN | |`
    // rows padded to the SAME 24,577-byte `RECORD_SIZE` this vector's own
    // hand arithmetic above already establishes, so the packed-shard-count
    // arithmetic is unchanged; only the fixture's oracle-detectability
    // changed.
    const RECORD_SIZE: usize = 24_577;
    const RECORD_COUNT: usize = 19;
    const CAP: u64 = 49_152;

    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    let original_content = concat_decision_log_rows(RECORD_COUNT, RECORD_SIZE);
    assert_eq!(
        original_content.len(),
        466_963,
        "arithmetic precondition: 19 * 24,577 must equal 466,963"
    );
    std::fs::write(&canonical_path, &original_content).unwrap();

    let boundaries = boundary_offsets_for(RECORD_COUNT, RECORD_SIZE);
    let oracle_offsets = mechanism_a_record_boundary_offsets("decision-log", &original_content);
    assert_eq!(
        oracle_offsets, boundaries,
        "P3-002: this fixture's hand-derived offsets must match the REAL oracle's own detected \
         `| D-NNN |` row boundaries exactly -- proving this test now exercises the genuine \
         `mechanism_a_record_boundary_offsets` cross-check (F-001/P2-001's own load-bearing \
         gate) rather than the empty-oracle trust-the-caller fallback the superseded fill-byte \
         fixture rode. Got: {oracle_offsets:?}"
    );
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
    //
    // P3-002 (MEDIUM, adversarial pass-3): REBUILT from the prior fill-byte
    // `sized_record`/`concat_sized_records` fixture to use
    // `lessons_h2_row_of_len` -- genuine, oracle-detectable
    // `## L-EDP1-NNN ...` h2 headings padded to the SAME 24,577-byte
    // `RECORD_SIZE` this vector's own hand arithmetic above already
    // establishes, so the packed-shard-count arithmetic is unchanged; only
    // the fixture's oracle-detectability changed (same rationale as the
    // sibling decision-log F-004 test above).
    const RECORD_SIZE: usize = 24_577;
    const RECORD_COUNT: usize = 5;
    const CAP: u64 = 49_152;

    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("lessons.md");
    let original_content = concat_lessons_rows(RECORD_COUNT, RECORD_SIZE);
    assert_eq!(
        original_content.len(),
        122_885,
        "arithmetic precondition: 5 * 24,577 must equal 122,885"
    );
    std::fs::write(&canonical_path, &original_content).unwrap();

    let boundaries = boundary_offsets_for(RECORD_COUNT, RECORD_SIZE);
    let oracle_offsets = mechanism_a_record_boundary_offsets("lessons", &original_content);
    assert_eq!(
        oracle_offsets, boundaries,
        "P3-002: this fixture's hand-derived offsets must match the REAL oracle's own detected \
         `## L-EDP1-NNN` heading boundaries exactly -- proving this test now exercises the \
         genuine `mechanism_a_record_boundary_offsets` cross-check rather than the empty-oracle \
         trust-the-caller fallback. Got: {oracle_offsets:?}"
    );
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
    //
    // P3-002 (MEDIUM, adversarial pass-3): REBUILT from the prior fill-byte
    // `sized_record`/`concat_sized_records` fixture to use
    // `decision_log_row_of_len` -- genuine, oracle-detectable `| D-NNN | |`
    // rows, exactly like the sibling F-004 decision-log test above (same
    // 466,963-byte / 19-record / 24,577-byte-per-row arithmetic, unchanged).
    const RECORD_SIZE: usize = 24_577;
    const RECORD_COUNT: usize = 19;
    const CAP: u64 = 49_152;

    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    let original_content = concat_decision_log_rows(RECORD_COUNT, RECORD_SIZE);
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
    let oracle_offsets = mechanism_a_record_boundary_offsets("decision-log", &original_content);
    assert_eq!(
        oracle_offsets, boundaries,
        "P3-002: this fixture's hand-derived offsets must match the REAL oracle's own detected \
         boundaries exactly. Got: {oracle_offsets:?}"
    );
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

// ---------------------------------------------------------------------------
// P3-001 (S-25.02 F4 cluster-3 adversarial pass-3, HIGH; BC-1.18.008 v1.4's
// new Leading-Preamble Handling Rule, EC-007/EC-008): the preamble/first-
// record cap-overflow case. Prior to this amendment, `mechanism_a_partition_for_backfill`
// (post-BLOCKER-1/P2-003) unconditionally folds every leading-preamble byte
// into whichever partition ends up holding the first record, with no
// overflow check -- when `preamble_bytes + first_record_bytes >
// shard_cap_bytes`, that fold pushes the FIRST sealed shard over cap without
// any sanctioning `oversized_record`/`is_preamble_shard` flag, re-creating
// exactly the unsanctioned Postcondition 2 violation Layer 2 exists to
// eliminate. v1.4's Leading-Preamble Handling Rule resolves this: the
// preamble is now a single atomic, indivisible packing unit resolved ONCE
// before record-based packing begins, sealing as its own zero-record shard
// in the overflow case (EC-007) -- or, in the degenerate case where the
// preamble ALONE exceeds cap (EC-008), sealing as its own oversized shard
// via the SAME EC-002 oversized-atomic-unit exception.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_008_P3001_EC007_partition_for_backfill_preamble_overflow_seals_as_own_zero_record_partition()
 {
    // The exact case the fresh-context adversarial pass-3 review flagged:
    // a 50-byte preamble + five 30-byte records, cap 70.
    // preamble_bytes (50) + first_record_bytes (30) = 80 > 70 -> OVERFLOW
    // case (Postcondition 2's Leading-Preamble Handling Rule): the preamble
    // must seal as its OWN zero-record partition BEFORE record-based
    // packing begins; record packing then starts fresh over the five
    // 30-byte records against the SAME 70-byte cap this file's existing
    // `test_BC_1_18_008_PC2_partition_for_backfill_never_splits_mid_record_and_stays_under_cap`
    // test already hand-verifies produces 3 partitions (60/60/30 bytes) --
    // so this fixture's total expected partition count is 1 (preamble) + 3
    // (records) = 4.
    let preamble = b"p".repeat(50);
    let records = concat_records(&[1, 2, 3, 4, 5]); // five 30-byte records, 150 bytes
    let mut content = preamble.clone();
    content.extend_from_slice(&records);
    let boundaries: Vec<usize> = [0usize, 30, 60, 90, 120]
        .iter()
        .map(|o| o + preamble.len())
        .collect();
    assert!(
        (preamble.len() + 30) > 70,
        "test fixture precondition: preamble_bytes + first_record_bytes must exceed \
         shard_cap_bytes (70) to exercise the OVERFLOW case, not the normal case"
    );

    let cap = 70u64;
    let partitions = mechanism_a_partition_for_backfill(&content, &boundaries, cap);

    assert_eq!(
        partitions.len(),
        4,
        "EC-007: 1 preamble-only partition + 3 record partitions (60/60/30 bytes, per this \
         file's own existing PC2 five-30-byte-records-vs-70-byte-cap vector) expected. Got {} \
         partitions: {partitions:?}",
        partitions.len()
    );

    // Every partition (this fixture has no individually-oversized record or
    // preamble) must respect the cap.
    for (i, p) in partitions.iter().enumerate() {
        assert!(
            p.bytes.len() as u64 <= cap,
            "EC-007/Postcondition 2: partition {i} ({} bytes) must not exceed shard_cap_bytes \
             ({cap}) -- the {}-byte preamble plus the first 30-byte record together exceed cap, \
             so they must NOT be folded into the same partition. Partition {i}: {p:?}",
            p.bytes.len(),
            preamble.len()
        );
    }

    // The FIRST partition must be the preamble alone: zero records, exactly
    // the preamble's own bytes, and NOT flagged oversized (the preamble
    // alone, 50 bytes, is well under the 70-byte cap -- only
    // preamble+first-record together exceed it, which is EC-007's overflow
    // case, distinct from EC-008's degenerate case below).
    assert_eq!(
        partitions[0].bytes, preamble,
        "EC-007/Leading-Preamble Handling Rule (overflow case): the FIRST partition must contain \
         ONLY the preamble bytes, sealed before record-based packing begins"
    );
    assert_eq!(
        partitions[0].record_count, 0,
        "EC-007: the preamble-only partition holds zero domain records"
    );
    assert!(
        !partitions[0].oversized_record,
        "EC-007 (not EC-008): the preamble ALONE ({} bytes) does not exceed shard_cap_bytes \
         ({cap}) on its own -- only preamble+first-record together do -- so this must NOT be \
         flagged oversized_record (that flag is reserved for EC-008's degenerate case)",
        preamble.len()
    );

    // Postcondition 6(a)/6(b): every byte and every record must still
    // round-trip exactly, preamble included.
    let mut reconstructed = Vec::new();
    let mut total_records = 0usize;
    for p in &partitions {
        reconstructed.extend_from_slice(&p.bytes);
        total_records += p.record_count;
    }
    assert_eq!(
        reconstructed, content,
        "Postcondition 6(a): all partitions concatenated (preamble partition included) must \
         reproduce the original content byte-for-byte"
    );
    assert_eq!(
        total_records, 5,
        "Postcondition 6(b): total record_count across all partitions (including the \
         zero-record preamble partition) must equal the 5 real records -- never zero, never \
         two, for any record"
    );
}

#[test]
fn test_BC_1_18_008_P3001_EC007_run_backfill_split_preamble_overflow_seals_own_shard_flagged_is_preamble_shard_end_to_end()
 {
    // End-to-end counterpart, driven through the real, on-disk
    // `run_mechanism_a_backfill_split` entry point: the FIRST sealed shard
    // (seq=1) must be the preamble-only shard, its own file bytes `<= cap`,
    // and its PUBLISHED shard-index entry must carry
    // `is_preamble_shard: true` and `records: 0` (Postcondition 3's
    // extended preamble-shard index-entry shape) -- read back as a generic
    // `toml::Value` (same technique this file's existing P2-002 test uses)
    // so this test does not itself assume these NEW fields already exist on
    // the strongly-typed `ShardIndexEntry` struct. EVERY sealed shard this
    // call produces must respect the cap (Postcondition 6(c)/Invariant 4).
    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");

    let preamble = b"p".repeat(50);
    let records = concat_records(&[1, 2, 3, 4, 5]);
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
        .expect("EC-007: a well-formed backfill with a preamble-overflow condition must succeed");

    let MechanismABackfillOutcome::Migrated { sealed_count, .. } = outcome else {
        panic!("EC-007: expected a Migrated outcome for this fixture, got {outcome:?}");
    };

    // Postcondition 6(c)/Invariant 4: EVERY sealed shard's actual on-disk
    // bytes must respect cap.
    for seq in 1..=sealed_count {
        let sealed_bytes = std::fs::read(dir.path().join(format!("decision-log.{seq:04}.md")))
            .unwrap_or_else(|e| panic!("EC-007: sealed shard seq={seq} must exist: {e}"));
        assert!(
            sealed_bytes.len() as u64 <= cap,
            "PC6(c)/Invariant 4: sealed shard seq={seq} ({} bytes) exceeds shard_cap_bytes \
             ({cap}) without a sanctioned exception",
            sealed_bytes.len()
        );
    }

    // The FIRST sealed shard (seq=1) is the preamble-only shard.
    let seq1_bytes = std::fs::read(dir.path().join("decision-log.0001.md")).unwrap();
    assert_eq!(
        seq1_bytes, preamble,
        "EC-007: sealed shard seq=1 must contain ONLY the preamble bytes"
    );

    let index_toml =
        std::fs::read_to_string(dir.path().join("decision-log.shard-index.toml")).unwrap();
    let index_value: toml::Value =
        toml::from_str(&index_toml).expect("the shard-index TOML must parse as generic TOML");
    let shards = index_value
        .get("shard")
        .and_then(|v| v.as_array())
        .expect("the shard-index must publish a [[shard]] array");
    let seq1_entry = shards
        .iter()
        .find(|s| s.get("seq").and_then(toml::Value::as_integer) == Some(1))
        .expect("seq=1 entry must exist in the published shard-index");

    assert_eq!(
        seq1_entry
            .get("is_preamble_shard")
            .and_then(toml::Value::as_bool),
        Some(true),
        "PC2/PC3 Leading-Preamble Handling Rule: the preamble-only shard's PUBLISHED index entry \
         must carry `is_preamble_shard: true`, distinguishing it from an ordinary record-bearing \
         shard for downstream readers without having to re-derive record count from file \
         content. Got shard entry: {seq1_entry:?}"
    );
    assert_eq!(
        seq1_entry.get("records").and_then(toml::Value::as_integer),
        Some(0),
        "PC3: the preamble shard's PUBLISHED index entry must carry `records: 0`. Got shard \
         entry: {seq1_entry:?}"
    );
}

#[test]
fn test_BC_1_18_008_P3001_EC008_partition_for_backfill_preamble_alone_exceeds_cap_sealed_whole_oversized()
 {
    // Leading-Preamble Handling Rule, DEGENERATE case (EC-008): the
    // preamble ALONE exceeds shard_cap_bytes (not reachable for any of the
    // four mandatory artifacts at their current measured preamble sizes,
    // per BC-1.18.008 v1.4's own EC-008 text, but specified for
    // completeness). Per Postcondition 2, the preamble seals as its own
    // oversized shard using the SAME EC-002 oversized-atomic-unit
    // exception (`oversized_record: true`) -- NOT a fail-loud abort;
    // content atomicity for an indivisible structural unit takes
    // precedence over the cap, identically to EC-002's own rationale for
    // an oversized record.
    let preamble = b"p".repeat(100); // exceeds the 70-byte cap alone
    let records = concat_records(&[1, 2]); // two 30-byte records, 60 bytes total (fits under cap)
    let mut content = preamble.clone();
    content.extend_from_slice(&records);
    let boundaries: Vec<usize> = [0usize, 30].iter().map(|o| o + preamble.len()).collect();

    let cap = 70u64;
    assert!(
        preamble.len() as u64 > cap,
        "test fixture precondition: the preamble ALONE must exceed shard_cap_bytes (70) to \
         exercise the DEGENERATE case"
    );

    let partitions = mechanism_a_partition_for_backfill(&content, &boundaries, cap);

    assert_eq!(
        partitions[0].bytes, preamble,
        "EC-008: the first partition must contain ONLY the oversized preamble"
    );
    assert_eq!(
        partitions[0].record_count, 0,
        "EC-008: the preamble holds zero domain records"
    );
    assert!(
        partitions[0].oversized_record,
        "EC-008/Leading-Preamble Handling Rule (degenerate case): a preamble that ALONE exceeds \
         shard_cap_bytes ({cap}) must be flagged `oversized_record: true` (the SAME EC-002 \
         oversized-atomic-unit exception, broadened here to cover the preamble too) -- NOT a \
         fail-loud abort"
    );

    // Content-preservation across all partitions (preamble + the two
    // 30-byte records that fit together under cap, forming the fresh
    // current partition).
    let mut reconstructed = Vec::new();
    let mut total_records = 0usize;
    for p in &partitions {
        reconstructed.extend_from_slice(&p.bytes);
        total_records += p.record_count;
    }
    assert_eq!(reconstructed, content);
    assert_eq!(total_records, 2);
}

#[test]
fn test_BC_1_18_008_P3001_EC008_run_backfill_split_preamble_alone_exceeds_cap_flagged_oversized_and_is_preamble_shard_end_to_end()
 {
    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");

    let preamble = b"p".repeat(100);
    let records = concat_records(&[1, 2]);
    let mut original_content = preamble.clone();
    original_content.extend_from_slice(&records);
    std::fs::write(&canonical_path, &original_content).unwrap();

    let boundaries: Vec<usize> = [0usize, 30].iter().map(|o| o + preamble.len()).collect();
    let cap = 70u64;
    let entry = flat_entry("decision-log", cap);

    let outcome = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 10).expect(
        "EC-008: a backfill with an oversized preamble must SUCCEED (not abort) -- content \
             atomicity for an indivisible unit beats the cap, identically to EC-002",
    );

    assert_eq!(
        outcome,
        MechanismABackfillOutcome::Migrated {
            sealed_count: 1,
            archived_count: 0
        },
        "EC-008: only the oversized preamble seals (seq=1); the two 30-byte records together \
         fit under cap and become the fresh current file"
    );

    let seq1_bytes = std::fs::read(dir.path().join("decision-log.0001.md")).unwrap();
    assert_eq!(
        seq1_bytes, preamble,
        "EC-008: sealed shard seq=1 must contain the FULL oversized preamble, un-truncated"
    );

    let index_toml =
        std::fs::read_to_string(dir.path().join("decision-log.shard-index.toml")).unwrap();
    let index_value: toml::Value = toml::from_str(&index_toml).unwrap();
    let shards = index_value
        .get("shard")
        .and_then(|v| v.as_array())
        .expect("the shard-index must publish a [[shard]] array");
    let seq1_entry = shards
        .iter()
        .find(|s| s.get("seq").and_then(toml::Value::as_integer) == Some(1))
        .expect("seq=1 entry must exist");

    assert_eq!(
        seq1_entry
            .get("oversized_record")
            .and_then(toml::Value::as_bool),
        Some(true),
        "EC-008: the published shard-index entry must carry `oversized_record: true` (the SAME \
         EC-002 flag, broadened to the oversized-preamble case). Got: {seq1_entry:?}"
    );
    assert_eq!(
        seq1_entry
            .get("is_preamble_shard")
            .and_then(toml::Value::as_bool),
        Some(true),
        "EC-008: the published shard-index entry must ALSO carry `is_preamble_shard: true`. \
         Got: {seq1_entry:?}"
    );
    assert_eq!(
        seq1_entry.get("records").and_then(toml::Value::as_integer),
        Some(0),
        "EC-008: the published shard-index entry must carry `records: 0`. Got: {seq1_entry:?}"
    );
}

// ---------------------------------------------------------------------------
// P3-001 (continued) -- new Postcondition 6(c)/Invariant 4 hard gate:
// `mechanism_a_verify_backfill_per_shard_cap_preserved` (this file's own
// test authorship defines this function's expected name/signature,
// mirroring the existing `mechanism_a_verify_backfill_content_preserved`
// (Postcondition 6(a)) / `mechanism_a_verify_backfill_record_counts_preserved`
// (Postcondition 6(b)) precedent exactly -- Postcondition 6(c) is the THIRD
// sub-clause of the SAME mandatory verification gate, so a third dedicated
// predicate, tested at the identical unit-test grain as its two siblings
// via this file's existing `partition()` helper, is the natural
// implementation shape). `run_mechanism_a_backfill_split` is expected to
// wire this in as a hard, fail-loud gate BEFORE any durable write occurs --
// exactly like its two siblings -- per Invariant 4's own text: "enforced as
// a fail-loud verification gate, never merely implied by the packer's own
// behavior."
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_008_PC6c_INV4_verify_per_shard_cap_preserved_true_when_every_shard_within_cap() {
    let partitions = vec![
        partition(&[b'a'; 50], 2, false),
        partition(&[b'b'; 70], 2, false),
    ];

    assert!(
        mechanism_a_verify_backfill_per_shard_cap_preserved(&partitions, 70),
        "PC6(c)/Invariant 4: every partition at or under shard_cap_bytes (70), none flagged \
         oversized_record, must verify as preserved"
    );
}

#[test]
fn test_BC_1_18_008_PC6c_INV4_verify_per_shard_cap_preserved_false_when_unflagged_shard_exceeds_cap()
 {
    let partitions = vec![
        partition(&[b'a'; 50], 2, false),
        partition(&[b'b'; 90], 2, false), // exceeds cap (70), NOT flagged oversized_record
    ];

    assert!(
        !mechanism_a_verify_backfill_per_shard_cap_preserved(&partitions, 70),
        "PC6(c)/Invariant 4: an unflagged partition exceeding shard_cap_bytes is EXACTLY the \
         unsanctioned Postcondition 2 violation Layer 2 exists to eliminate -- this hard gate \
         must detect it as NOT preserved, mirroring PC6(a)/PC6(b)'s own hard-gate shape, so that \
         `run_mechanism_a_backfill_split` aborts with `ContentPreservationFailed` (E-SHD-003) \
         rather than durably writing an unsanctioned over-cap shard"
    );
}

#[test]
fn test_BC_1_18_008_PC6c_INV4_verify_per_shard_cap_preserved_true_when_oversized_record_flagged_exception()
 {
    let partitions = vec![
        partition(&[b'a'; 200], 1, true), // exceeds cap but sanctioned via EC-002/EC-008
        partition(&[b'b'; 30], 1, false),
    ];

    assert!(
        mechanism_a_verify_backfill_per_shard_cap_preserved(&partitions, 70),
        "PC6(c)/Invariant 4: a partition exceeding shard_cap_bytes that IS flagged \
         `oversized_record: true` (EC-002's single-oversized-record exception, or EC-008's \
         degenerate oversized-preamble exception -- this module represents BOTH identically via \
         `oversized_record: true` on the returned partition) is the SANCTIONED exception -- must \
         verify as preserved, never abort"
    );
}

// ---------------------------------------------------------------------------
// P3-002 (S-25.02 F4 cluster-3 adversarial pass-3, MEDIUM): the "oracle
// recognizes NO independently-detectable boundary at all" fallback in
// `run_mechanism_a_backfill_split` -- "the caller-supplied offsets are
// trusted at face value" -- must fail loud, not silently proceed, for an
// artifact_stem this module has literally NO marker rule for at all (falls
// through `mechanism_a_record_boundary_offsets`'s own `_ => Vec::new()`
// arm). BC-1.18.008 v1.4's own Changelog (F-C3-P3-002) confirms
// Postcondition 2's Normalization rule already states this MUST reject the
// backfill run "rather than silently mis-partition" for content this
// module cannot classify -- this test pins that already-mandated behavior
// as executable coverage.
//
// Scope note: this test deliberately targets a stem OUTSIDE the four
// mandatory artifacts (`decision-log`/`burst-log`/`lessons`/
// `session-checkpoints`), never one of those four with merely
// non-marker-matching fixture content (e.g. this file's own
// `concat_records`-based fixtures for `decision-log`) -- the latter shape
// is unaffected by this fix and remains covered, unmodified, by this
// file's many pre-existing `decision-log`/`burst-log` fixtures elsewhere.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_008_P3002_run_backfill_split_aborts_when_artifact_stem_unrecognized_with_caller_offsets()
 {
    let content =
        b"arbitrary content for an artifact_stem this module has no marker rule for at all"
            .to_vec();

    let oracle_offsets =
        mechanism_a_record_boundary_offsets("some-unrecognized-artifact", &content);
    assert!(
        oracle_offsets.is_empty(),
        "test fixture precondition: this module has no marker rule for \
         'some-unrecognized-artifact' -- the oracle must return an empty boundary set. Got: \
         {oracle_offsets:?}"
    );

    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("some-unrecognized-artifact.md");
    std::fs::write(&canonical_path, &content).unwrap();

    let entry = flat_entry("some-unrecognized-artifact", 20);
    let caller_offsets = [0usize, 40]; // well-formed, but utterly unverifiable by the oracle

    let outcome = run_mechanism_a_backfill_split(&entry, &canonical_path, &caller_offsets, 10);

    assert!(
        matches!(
            outcome,
            Err(MechanismABackfillError::ContentPreservationFailed { .. })
        ),
        "P3-002/PC2 Normalization rule: an artifact_stem this module has NO marker rule for at \
         all, combined with a caller-supplied `record_boundary_offsets` argument, must ABORT -- \
         the oracle has nothing to corroborate the caller's claim against, so trusting it at \
         face value is exactly the silently-mis-partition outcome the Normalization rule \
         forbids. Got: {outcome:?}"
    );

    let post_content = std::fs::read(&canonical_path).unwrap();
    assert_eq!(
        post_content, content,
        "P3-002/Postcondition 6: on abort, the original monolithic file MUST be left completely \
         untouched (fail-loud, never partial-and-silent)"
    );
    assert!(
        !dir.path()
            .join("some-unrecognized-artifact.0001.md")
            .exists(),
        "P3-002/Postcondition 5: on abort, no sealed shard file may have been durably written"
    );
}

// ---------------------------------------------------------------------------
// P3-003 (S-25.02 F4 cluster-3 adversarial pass-3, MINOR): predicate
// over-match decoys. `is_lesson_h2_record_heading`'s `"LESSON (D-"` /
// `"RECURRENCE NOTE (D-"` branches and `is_pass_fix_burst_heading`'s
// `" Fix Burst"` suffix check both currently use a bare
// `starts_with`/prefix match rather than the Record-Boundary Marker
// Table's own full regex (`^## LESSON \(D-[0-9]+\)` /
// `^### Pass-[0-9]+ Fix Burst\b`) -- neither requires digits to actually
// follow `D-`, nor a word boundary immediately after `Burst`, so a heading
// that merely SHARES the marker's own leading substring (without matching
// its full documented shape) is misdetected as a genuine record boundary.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_008_P3003_record_boundary_offsets_lessons_rejects_lesson_marker_without_digits() {
    // Marker table regex: `^## LESSON \(D-[0-9]+\)` -- REQUIRES one-or-more
    // ASCII digits immediately after `D-`. `## LESSON (D-foo)` (non-digit
    // suffix) and `## LESSON (D-)` (no suffix at all) both share the bare
    // `"LESSON (D-"` prefix the current implementation checks via
    // `starts_with`, but neither is a genuine `D-NNN`-tagged lesson record.
    let real_lesson = "## LESSON (D-1065) — a genuine, digit-tagged lesson record\n";
    let decoy_non_digit = "## LESSON (D-foo) — not a real D-NNN ID, must not be a boundary\n";
    let decoy_empty = "## LESSON (D-) — no digits at all, must not be a boundary\n";

    let content = format!(
        "# Lessons Learned\n\n\
         {decoy_non_digit}\
         Body text for the non-digit decoy.\n\n\
         {decoy_empty}\
         Body text for the empty-suffix decoy.\n\n\
         {real_lesson}\
         Body text for the genuine record.\n"
    );

    let offset_decoy1 = content.find(decoy_non_digit).unwrap();
    let offset_decoy2 = content.find(decoy_empty).unwrap();
    let offset_real = content.find(real_lesson).unwrap();

    let offsets = mechanism_a_record_boundary_offsets("lessons", content.as_bytes());

    assert_eq!(
        offsets,
        vec![offset_real],
        "P3-003/PC2 Record-Boundary Marker Table: only the digit-tagged `## LESSON (D-1065)` \
         heading is a genuine record boundary -- `## LESSON (D-foo)` and `## LESSON (D-)` \
         merely share the bare `\"LESSON (D-\"` prefix without matching the marker table's own \
         `^## LESSON \\(D-[0-9]+\\)` regex, and must NOT be treated as boundaries. Got: \
         {offsets:?}"
    );
    assert!(
        !offsets.contains(&offset_decoy1) && !offsets.contains(&offset_decoy2),
        "P3-003: neither non-digit-suffix decoy may appear in the boundary set. Got: {offsets:?}"
    );
}

#[test]
fn test_BC_1_18_008_P3003_record_boundary_offsets_burst_log_rejects_fix_burst_heading_without_word_boundary()
 {
    // Marker table regex: `^### Pass-[0-9]+ Fix Burst\b` -- the `\b` word
    // boundary REQUIRES the text immediately after "Burst" to be a
    // non-word character (whitespace, punctuation, end-of-line) -- never
    // another word character continuing the same word. `### Pass-39 Fix
    // Bursting — ...` shares the bare `" Fix Burst"` prefix the current
    // implementation checks via `starts_with`, but "Bursting" is a
    // DIFFERENT word than "Burst" -- not a genuine Pass-N-Fix-Burst record.
    let h2_a = "## F5 pass-38 fix burst\n";
    let decoy_no_word_boundary = "### Pass-39 Fix Bursting — not a real Pass-N Fix Burst record\n";
    let real_pass_fix_burst = "### Pass-40 Fix Burst — a genuine h3-exception record\n";
    let h2_b = "## Burst: F5 pass-41 fix burst\n";

    let content = format!(
        "# burst-log\n\n\
         {h2_a}\
         Body of pass-38.\n\n\
         {decoy_no_word_boundary}\
         Body text for the no-word-boundary decoy.\n\n\
         {real_pass_fix_burst}\
         Body of the genuine pass-40 h3-exception record.\n\n\
         {h2_b}\
         Body of pass-41.\n"
    );

    let offset_h2_a = content.find(h2_a).unwrap();
    let offset_decoy = content.find(decoy_no_word_boundary).unwrap();
    let offset_real = content.find(real_pass_fix_burst).unwrap();
    let offset_h2_b = content.find(h2_b).unwrap();

    let offsets = mechanism_a_record_boundary_offsets("burst-log", content.as_bytes());

    assert_eq!(
        offsets,
        vec![offset_h2_a, offset_real, offset_h2_b],
        "P3-003/PC2 Record-Boundary Marker Table: `### Pass-39 Fix Bursting` shares the bare \
         `\" Fix Burst\"` prefix without matching the marker table's own \
         `^### Pass-[0-9]+ Fix Burst\\b` regex (no word boundary after \"Burst\" -- \"Bursting\" \
         continues the same word) and must NOT be treated as a boundary; only the two genuine h2 \
         records and the genuine `### Pass-40 Fix Burst` h3-exception record are boundaries. \
         Got: {offsets:?}"
    );
    assert!(
        !offsets.contains(&offset_decoy),
        "P3-003: the no-word-boundary decoy must never appear in the boundary set. Got: \
         {offsets:?}"
    );
}

// ---------------------------------------------------------------------------
// F-C3-P5-001 (S-25.02 F4 cluster-3 adversarial pass-5, LOW): the empty
// caller-offsets twin of P3-002 above. `run_mechanism_a_backfill_split`'s
// `original_record_count` computation branches directly on
// `record_boundary_offsets.is_empty()`; the `is_empty()` arm falls back to
// `usize::from(!original_content.is_empty())` WITHOUT ever consulting the
// `mechanism_a_record_boundary_offsets` oracle -- unlike the non-empty-offsets
// branch immediately below it, which cross-checks the caller's list against
// the oracle's own output (F-001/P2-001/P3-002).
// `mechanism_a_partition_for_backfill` shares the identical blind spot: an
// empty `record_boundary_offsets` argument makes it treat the WHOLE content
// as a single record, regardless of how many genuine records the oracle can
// independently find. For a KNOWN stem (`decision-log`) whose real content
// contains multiple genuine, oracle-detectable `| D-NNN |` rows, this
// collapses `partitions.len()` to `1`, tripping the EC-016 "nothing to do"
// path and reporting `Ok(Migrated { sealed_count: 0 })` -- the mandated
// split silently does not happen, and the outcome is reported as success.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_008_P5001_run_backfill_split_aborts_when_caller_offsets_empty_but_oracle_finds_real_boundaries()
 {
    let row_len = 50;
    let record_count = 5;
    let content = concat_decision_log_rows(record_count, row_len);

    // Fixture precondition: the oracle genuinely finds all `record_count`
    // real `| D-NNN |` row boundaries in this content -- there is real
    // structure here for an empty caller-offsets argument to miss.
    let oracle_offsets = mechanism_a_record_boundary_offsets("decision-log", &content);
    assert_eq!(
        oracle_offsets,
        boundary_offsets_for(record_count, row_len),
        "test fixture precondition: the oracle must find all {record_count} genuine \
         `| D-NNN |` row boundaries in this fixture's content. Got: {oracle_offsets:?}"
    );

    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    std::fs::write(&canonical_path, &content).unwrap();

    // A small test cap that genuinely mandates a multi-shard split for this
    // content's real byte size (`ceil(250/100) = 3 > 1`) -- not a borderline
    // "nothing to split" fixture; the split is structurally required.
    let shard_cap_bytes = 100;
    let entry = flat_entry("decision-log", shard_cap_bytes);

    let outcome = run_mechanism_a_backfill_split(&entry, &canonical_path, &[], 10);

    assert!(
        matches!(
            outcome,
            Err(MechanismABackfillError::ContentPreservationFailed { .. })
        ),
        "F-C3-P5-001: an empty caller-supplied `record_boundary_offsets` combined with a KNOWN \
         stem whose real content the oracle can genuinely partition into {record_count} records \
         must ABORT fail-loud -- the oracle found real boundaries the empty caller-offsets \
         argument missed entirely, which is exactly the silently-mis-partition outcome \
         Postcondition 2's Normalization rule forbids (P3-002 already covers the 'no oracle \
         rule at all' case above; this is its twin for 'an oracle rule exists and finds real \
         structure, but the caller supplied nothing'). Got: {outcome:?}"
    );

    let post_content = std::fs::read(&canonical_path).unwrap();
    assert_eq!(
        post_content, content,
        "F-C3-P5-001/Postcondition 6: on abort, the original monolithic file MUST be left \
         completely untouched (fail-loud, never partial-and-silent)"
    );
    assert!(
        !dir.path().join("decision-log.0001.md").exists(),
        "F-C3-P5-001/Postcondition 5: on abort, no sealed shard file may have been durably \
         written"
    );
    assert!(
        !dir.path().join("decision-log.shard-index.toml").exists(),
        "F-C3-P5-001: on abort, no shard-index may have been durably written either"
    );
}

/// Companion to the abort test above: the genuinely-valid empty-content +
/// empty-offsets no-op path (EC-016's zero-record case) must keep succeeding
/// -- there is no real record structure here for an empty caller-offsets
/// argument to have missed, so this shape must never be swept up by
/// F-C3-P5-001's fix into an unwarranted abort.
#[test]
fn test_BC_1_18_008_P5001_run_backfill_split_empty_content_and_empty_offsets_still_succeeds() {
    let content: Vec<u8> = Vec::new();

    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    std::fs::write(&canonical_path, &content).unwrap();

    let entry = flat_entry("decision-log", 10_000);

    let outcome = run_mechanism_a_backfill_split(&entry, &canonical_path, &[], 10).expect(
        "F-C3-P5-001 companion: genuinely empty content with an empty \
         `record_boundary_offsets` argument has no real record structure for the oracle to \
         have missed, and must continue to succeed as a legitimate zero-record no-op",
    );

    assert_eq!(
        outcome,
        MechanismABackfillOutcome::Migrated {
            sealed_count: 0,
            archived_count: 0
        },
        "F-C3-P5-001 companion: an empty artifact with empty caller offsets seals zero shards"
    );

    let post_content = std::fs::read(&canonical_path).unwrap();
    assert_eq!(
        post_content, content,
        "F-C3-P5-001 companion: the canonical (empty) file must remain unchanged"
    );
}

// ---------------------------------------------------------------------------
// F-C3-P6-001 (BC-1.18.008 v1.6 Postcondition 5's Recovery-Confirmation
// Rule, Invariant 3, EC-009/EC-010): a fresh-context CROSS-VENDOR (OpenAI
// Codex) adversarial pass-6 review found `heal_or_confirm_already_migrated`
// classifies the SAFE/DANGEROUS crash-window disposition using ONLY a
// structural byte-prefix comparison (`canonical_bytes[..sealed_concat.len()]
// == sealed_concat`) against the artifact's own already-sealed shards. This
// heuristic false-positives whenever the artifact's real content
// legitimately repeats a sealed shard's exact bytes as a PREFIX of the
// SAFE-window final partition -- realistic for `session-checkpoints.md`,
// since the Record-Boundary Marker Table imposes no uniqueness requirement
// on checkpoint headings/bodies. BC-1.18.008 v1.6 replaces this heuristic
// with a Backfill Recovery Manifest (`[backfill_manifest]` in the
// shard-index: `original_bytes`/`original_sha256`/`final_bytes`/
// `final_sha256`, computed once at split time) and an exact whole-file
// `(length, SHA-256)` comparison against it: match `final_*` => SAFE
// no-op; match `original_*` => DANGEROUS, heal by writing the manifest's
// own recorded `final_bytes`; match neither => AMBIGUOUS, fail loud with
// the new `E-SHD-011` error code (added to `prd-supplements/
// error-taxonomy.md` v1.10 in the same burst) -- never silently overwrite.
//
// The three tests below drive the BC's own verified counterexample
// (`A = "## Checkpoint\nx\n"`, 16 bytes; `shard_cap_bytes = 16`) end-to-end
// through the real, public `run_mechanism_a_backfill_split` entry point --
// no internal struct is hand-constructed, since `ShardIndex` does not yet
// expose a typed `backfill_manifest` field; the manifest's own presence is
// pinned separately below at the raw-TOML level. This file's own doc
// comment (top of file) governs the discipline followed here: each test
// names the BC clause it pins and the real-world shape that motivated it,
// never a live claim about its own current pass/fail status.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_008_FC3P6001_EC009_run_backfill_split_second_invocation_preserves_repeated_prefix_content()
 {
    // BC-1.18.008 v1.6's own verified counterexample: `session-checkpoints`
    // content whose real body legitimately repeats a sealed shard's exact
    // bytes as the leading prefix of the SAFE-window final partition.
    let a = b"## Checkpoint\nx\n".to_vec(); // 16 bytes
    let mut original_content = a.clone();
    original_content.extend_from_slice(&a);
    original_content.extend_from_slice(b"more\n");
    assert_eq!(
        original_content.len(),
        37,
        "fixture precondition: A + A + \"more\\n\" must be 37 bytes, matching BC-1.18.008 v1.6's \
         own cited counterexample"
    );

    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("session-checkpoints.md");
    std::fs::write(&canonical_path, &original_content).unwrap();

    let boundaries = mechanism_a_record_boundary_offsets("session-checkpoints", &original_content);
    assert_eq!(
        boundaries,
        vec![0, 16],
        "fixture precondition: the two repeated `## Checkpoint` headings are the content's own \
         two real record boundaries"
    );

    let entry = flat_entry("session-checkpoints", 16);

    // First (uninterrupted) migration: seals the first `A` as shard 1;
    // the fresh current file legitimately holds `A + "more\n"` (21 bytes) --
    // this is a genuinely-correct SAFE-window state, not a crash.
    let first = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 10)
        .expect("the first, uninterrupted migration over this fixture must succeed");
    assert_eq!(
        first,
        MechanismABackfillOutcome::Migrated {
            sealed_count: 1,
            archived_count: 0
        },
        "precondition: the first run must seal exactly the first `A` and leave `A + \"more\\n\"` \
         as the fresh current file"
    );
    let sealed_shard =
        std::fs::read(dir.path().join("session-checkpoints.0001.md")).expect("shard must exist");
    assert_eq!(
        sealed_shard, a,
        "precondition: the sealed shard must hold exactly the first `A`"
    );
    let expected_final = &original_content[16..]; // A + "more\n", 21 bytes
    let after_first = std::fs::read(&canonical_path).unwrap();
    assert_eq!(
        after_first, expected_final,
        "precondition: after the first run the canonical file must hold exactly the legitimate \
         final partition"
    );

    // SECOND invocation, no intervening crash: the canonical file's own
    // leading 16 bytes happen to be byte-identical to the sealed shard's
    // content (the artifact's real content legitimately repeats it) --
    // exactly the false-positive shape F-C3-P6-001 names. The
    // Recovery-Confirmation Rule must classify this as SAFE (canonical
    // bytes exactly match the manifest's `final_bytes`/`final_sha256`) and
    // take NO action.
    let second = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 10).expect(
        "EC-009: a second invocation against a legitimately-migrated, repeated-prefix \
                 artifact must not error",
    );

    assert_eq!(
        second,
        MechanismABackfillOutcome::AlreadyMigrated,
        "EC-009/Postcondition 5 Recovery-Confirmation Rule: the canonical file's exact whole-file \
         (length, SHA-256) matches the manifest's `final_bytes`/`final_sha256` -- SAFE window, no \
         action. Got: {second:?}"
    );

    let after_second = std::fs::read(&canonical_path).unwrap();
    assert_eq!(
        after_second,
        expected_final,
        "EC-009 (the data-loss regression F-C3-P6-001 corrects): the canonical file must remain \
         byte-for-byte `A + \"more\\n\"` (21 bytes) after the second invocation -- a structural \
         byte-prefix heuristic instead misclassifies this as the DANGEROUS window (the canonical \
         file's own leading 16 bytes coincidentally equal the sealed shard's content) and \
         overwrites the canonical file with just `canonical_bytes[16..]` (\"more\\n\", 5 bytes), \
         permanently destroying the second `A` record's heading and body. Got {} bytes instead of \
         the expected 21.",
        after_second.len()
    );
}

#[test]
fn test_BC_1_18_008_FC3P6001_run_backfill_split_second_invocation_heals_genuine_crash_window() {
    // Companion to the EC-009 test above: a GENUINE crash window (the
    // canonical file was never truncated after the first run's index
    // publish) must still be healed correctly by the manifest-based rule --
    // this pins the DANGEROUS disposition's own positive behavior, not just
    // its negative (EC-009) false-positive guard.
    let a = b"## Checkpoint\nx\n".to_vec(); // 16 bytes
    let mut original_content = a.clone();
    original_content.extend_from_slice(&a);
    original_content.extend_from_slice(b"more\n");

    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("session-checkpoints.md");
    std::fs::write(&canonical_path, &original_content).unwrap();

    let boundaries = mechanism_a_record_boundary_offsets("session-checkpoints", &original_content);
    let entry = flat_entry("session-checkpoints", 16);

    let first = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 10)
        .expect("the first, uninterrupted migration over this fixture must succeed");
    assert_eq!(
        first,
        MechanismABackfillOutcome::Migrated {
            sealed_count: 1,
            archived_count: 0
        }
    );

    // Simulate the genuine crash window named by Postcondition 5's "Two-phase
    // publish and the index-publish/canonical-truncate crash window"
    // subsection: the shard-index (and its Backfill Recovery Manifest) are
    // already durably published from the real first run above, but the
    // canonical file is restored to hold the FULL pre-split content, as if
    // the canonical-truncate write never completed.
    std::fs::write(&canonical_path, &original_content).unwrap();

    let expected_final = original_content[16..].to_vec(); // A + "more\n", 21 bytes

    let second = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 10).expect(
        "a re-invocation against the confirmed DANGEROUS crash window must self-heal, \
                 not error",
    );

    assert_eq!(
        second,
        MechanismABackfillOutcome::Healed { sealed_count: 1 },
        "Postcondition 5 Recovery-Confirmation Rule: canonical bytes exactly match the manifest's \
         `original_bytes`/`original_sha256` -- DANGEROUS window, unambiguously confirmed. Got: \
         {second:?}"
    );

    let healed = std::fs::read(&canonical_path).unwrap();
    assert_eq!(
        healed, expected_final,
        "Postcondition 5: recovery must complete the interrupted canonical-truncate write by \
         atomically writing the manifest's OWN recorded `final_bytes` content -- which for this \
         single-sealed-shard fixture is exactly the same 21 bytes independently established at \
         split time as the last partition, `A + \"more\\n\"`"
    );

    let index_toml =
        std::fs::read_to_string(dir.path().join("session-checkpoints.shard-index.toml")).unwrap();
    let index: ShardIndex = toml::from_str(&index_toml).unwrap();
    assert_eq!(
        index.shards.len(),
        1,
        "Invariant 3 (Idempotency): self-healing the interrupted truncation must never re-seal \
         already-sealed content into new, redundant shards"
    );
}

#[test]
fn test_BC_1_18_008_FC3P6001_EC010_run_backfill_split_ambiguous_on_disk_state_fails_loud_e_shd_011()
{
    // EC-010: at recovery-confirmation time, the canonical file's
    // `(length, SHA-256)` matches NEITHER the manifest's `original_*` nor
    // `final_*` pair (e.g. an operator manually edited the canonical file
    // between a crash and the recovery attempt, or the file is corrupted).
    // This MUST fail loud with `E-SHD-011` and MUST NOT write anything to
    // the canonical file -- never silently default to either the SAFE or
    // DANGEROUS disposition on an ambiguous match.
    let a = b"## Checkpoint\nx\n".to_vec(); // 16 bytes
    let mut original_content = a.clone();
    original_content.extend_from_slice(&a);
    original_content.extend_from_slice(b"more\n");

    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("session-checkpoints.md");
    std::fs::write(&canonical_path, &original_content).unwrap();

    let boundaries = mechanism_a_record_boundary_offsets("session-checkpoints", &original_content);
    let entry = flat_entry("session-checkpoints", 16);

    let first = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 10)
        .expect("the first, uninterrupted migration over this fixture must succeed");
    assert_eq!(
        first,
        MechanismABackfillOutcome::Migrated {
            sealed_count: 1,
            archived_count: 0
        }
    );

    // Tamper with the canonical file so its bytes match NEITHER the
    // manifest's `original_*` (37 bytes, A+A+"more\n") NOR `final_*` (21
    // bytes, A+"more\n") pair -- e.g. an operator manually edited it between
    // the crash and the recovery attempt. Deliberately does NOT start with
    // `A` either, so it is unambiguous under BOTH the old heuristic and the
    // new manifest-based rule that this state is neither SAFE nor DANGEROUS.
    let tampered: Vec<u8> =
        b"ZZZ operator-edited content unrelated to either recorded manifest state\n".to_vec();
    std::fs::write(&canonical_path, &tampered).unwrap();

    let result = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 10);

    let err = result.expect_err(
        "EC-010/Postcondition 5 Recovery-Confirmation Rule: an on-disk canonical state matching \
         NEITHER the manifest's original nor final (length, SHA-256) pair MUST fail loud, never \
         silently resolve to SAFE or DANGEROUS",
    );
    let err_message = err.to_string();
    assert!(
        err_message.contains("E-SHD-011"),
        "EC-010: the AMBIGUOUS disposition must surface the NEW `E-SHD-011` error code (per \
         `prd-supplements/error-taxonomy.md` v1.10) -- got a different error: {err_message}"
    );

    let post = std::fs::read(&canonical_path).unwrap();
    assert_eq!(
        post, tampered,
        "EC-010: on the AMBIGUOUS disposition, the canonical file MUST NOT be written to under \
         any circumstance -- it must remain exactly as found, pending operator investigation"
    );
    assert!(
        !dir.path().join("session-checkpoints.0002.md").exists(),
        "EC-010: an AMBIGUOUS recovery attempt must never seal an additional shard either"
    );
}

#[test]
fn test_BC_1_18_008_FC3P6001_PC3_run_backfill_split_publishes_backfill_recovery_manifest_fields() {
    // Postcondition 3's Backfill Recovery Manifest: the published
    // `<artifact-stem>.shard-index.toml` must carry a `[backfill_manifest]`
    // table -- written in the SAME atomic index-publish write as the
    // `[[shard]]` entries -- with `original_bytes`/`original_sha256` (the
    // pre-split monolithic file's own exact length + SHA-256) and
    // `final_bytes`/`final_sha256` (the intended final partition's own exact
    // length + SHA-256), both computed once at split time from the same
    // in-memory `original_content` buffer that drives Postcondition 2's
    // partitioning. Checked here at the raw-TOML-text level, since this file
    // only imports the pre-existing (not-yet-extended) `ShardIndex` schema
    // for its typed round-trips elsewhere.
    let a = b"## Checkpoint\nx\n".to_vec(); // 16 bytes
    let mut original_content = a.clone();
    original_content.extend_from_slice(&a);
    original_content.extend_from_slice(b"more\n");
    assert_eq!(original_content.len(), 37);

    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("session-checkpoints.md");
    std::fs::write(&canonical_path, &original_content).unwrap();

    let boundaries = mechanism_a_record_boundary_offsets("session-checkpoints", &original_content);
    assert_eq!(boundaries, vec![0, 16]);

    let entry = flat_entry("session-checkpoints", 16);
    let outcome = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 10)
        .expect("the first migration over this fixture must succeed");
    assert_eq!(
        outcome,
        MechanismABackfillOutcome::Migrated {
            sealed_count: 1,
            archived_count: 0
        }
    );

    let index_toml =
        std::fs::read_to_string(dir.path().join("session-checkpoints.shard-index.toml"))
            .expect("the shard-index must be published");

    assert!(
        index_toml.contains("[backfill_manifest]"),
        "Postcondition 3: the published shard-index MUST carry a `[backfill_manifest]` table, \
         populated in the SAME atomic write as the `[[shard]]` entries. Got:\n{index_toml}"
    );
    assert!(
        index_toml.contains("original_bytes = 37"),
        "Postcondition 3: `original_bytes` must record the pre-split monolithic file's exact \
         byte length (37 = len(A + A + \"more\\n\")). Got:\n{index_toml}"
    );
    assert!(
        index_toml.contains("final_bytes = 21"),
        "Postcondition 3: `final_bytes` must record the intended final (last) partition's exact \
         byte length (21 = len(A + \"more\\n\")). Got:\n{index_toml}"
    );
    assert!(
        index_toml.contains("original_sha256"),
        "Postcondition 3: `original_sha256` (the pre-split monolithic file's own content hash) \
         must be present. Got:\n{index_toml}"
    );
    assert!(
        index_toml.contains("final_sha256"),
        "Postcondition 3: `final_sha256` (the intended final partition's own content hash) must \
         be present. Got:\n{index_toml}"
    );
}

// ---------------------------------------------------------------------------
// F-C3-P6-002 (BC-1.18.008 v1.6 Postcondition 6(c)/Invariant 4's disk
// read-back ruling): a fresh-context CROSS-VENDOR (OpenAI Codex) adversarial
// pass-6 review asked whether "the actual bytes written to disk" (PC6(c),
// Invariant 4) means a POST-HOC read-back of each sealed shard file from
// disk after its write completes, or is satisfied by checking only the
// in-memory partition buffer's length before the write is issued. The BC
// RULED (a): a fresh, post-hoc disk read-back is REQUIRED.
//
// PREVIOUSLY (see this file's own git history at this comment block): no
// seam existed to drive a genuine fault-injection test for this ruling --
// `run_mechanism_a_backfill_split` was one synchronous call with no
// injectable I/O layer, callback, or lower-level "write one shard, then
// verify" function a test could drive independently and corrupt in between.
// The implementer's pass-6 fix extracted `mechanism_a_write_and_verify_sealed_shard`
// (`sealed_path: &Path, bytes: &[u8], artifact_stem: &str) ->
// Result<(), MechanismABackfillError>`) as exactly that seam: the
// write-then-read-back-then-verify unit in isolation, addressable directly.
//
// The two tests below drive it via a REAL, non-simulated disk race against
// `write_atomic`'s own sibling temp file (`last-amended-migrate/src/
// atomic_write.rs`'s documented `.{basename}.tmp-{pid}` naming convention,
// `pid` deterministically known via `std::process::id()`): a background
// thread busy-polls for that temp file's appearance and overwrites its
// on-disk content with corrupt bytes for as long as it exists before
// `write_atomic`'s own atomic `rename` consumes it. Because
// `write_and_sync_temp` (`File::create` + `write_all` + `sync_all`) is real
// disk I/O -- the `sync_all` fsync durability step in particular
// (S-15.03 N2) -- against a tight, syscall-driven polling loop with no
// deliberate sleep, this reliably lands the corrupting write on the temp
// file's bytes BEFORE the rename; `rename` is then atomic, so whatever
// bytes sit on the temp file at that moment are EXACTLY what ends up at
// `sealed_path`, independent of what the in-memory `bytes` argument
// intended -- a real, filesystem-level content divergence, not a simulated
// one. Empirically 100%-reliable across 200 local trials against a real
// temp-dir-backed filesystem (not a best-effort/flaky sleep-based guess);
// each test additionally retries the race itself (bounded, fresh fixture
// per attempt) as a portability safety margin across the release matrix's
// different filesystems/CI runners, so a transient scheduling miss on any
// single attempt cannot itself flake the test.
// ---------------------------------------------------------------------------

/// Race harness shared by both `mechanism_a_write_and_verify_sealed_shard`
/// tests below. See the comment block above for the full mechanism this
/// exploits (`write_atomic`'s deterministic sibling temp-file naming +
/// its real `fsync` durability window).
///
/// Returns the `(stop flag, thread handle)` pair the caller must signal
/// (`stop.store(true, Ordering::Relaxed)`) and join immediately after
/// invoking the function under test, so the corrupting thread never
/// outlives the single call it targets.
fn spawn_temp_file_corruptor(
    sealed_path: &Path,
    corrupt_bytes: Vec<u8>,
) -> (
    std::sync::Arc<std::sync::atomic::AtomicBool>,
    std::thread::JoinHandle<()>,
) {
    let parent = sealed_path
        .parent()
        .expect("sealed_path must have a parent dir")
        .to_path_buf();
    let basename = sealed_path
        .file_name()
        .expect("sealed_path must have a filename")
        .to_string_lossy()
        .into_owned();
    let tmp_path = parent.join(format!(".{basename}.tmp-{}", std::process::id()));

    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_for_thread = std::sync::Arc::clone(&stop);
    let handle = std::thread::spawn(move || {
        while !stop_for_thread.load(std::sync::atomic::Ordering::Relaxed) {
            if tmp_path.exists() {
                let _ = std::fs::write(&tmp_path, &corrupt_bytes);
            }
        }
    });
    (stop, handle)
}

#[test]
fn test_BC_1_18_008_FC3P6002_mechanism_a_write_and_verify_sealed_shard_round_trips_uncorrupted_write()
 {
    // Positive companion to the fault-injection test below: an ordinary,
    // uninterrupted write+read-back must succeed and report `Ok(())`, with
    // the sealed shard file on disk holding exactly the intended bytes.
    let dir = tempfile::tempdir().unwrap();
    let sealed_path = dir.path().join("decision-log.0001.md");
    let intended = b"legitimate sealed shard content, written and read back intact".to_vec();

    let result = mechanism_a_write_and_verify_sealed_shard(&sealed_path, &intended, "decision-log");

    assert!(
        result.is_ok(),
        "an uncorrupted write must round-trip successfully through the post-hoc disk read-back \
         gate. Got: {:?}",
        result.err()
    );
    assert_eq!(
        std::fs::read(&sealed_path).unwrap(),
        intended,
        "the sealed shard file on disk must hold exactly the intended bytes"
    );
}

#[test]
fn test_BC_1_18_008_FC3P6002_mechanism_a_write_and_verify_sealed_shard_aborts_fail_loud_on_disk_corruption_race()
 {
    // F-C3-P6-002 (Postcondition 6(c)/Invariant 4's disk read-back ruling):
    // the load-bearing fault-injection test the comment block above
    // previously deferred for lack of a seam. This races a corrupting write
    // against the sealed-shard temp file `write_atomic` itself creates
    // (`spawn_temp_file_corruptor`), landing corrupted bytes on disk BEFORE
    // the atomic `rename` step, so `sealed_path` legitimately (from the
    // filesystem's own point of view) ends up holding bytes that differ
    // from what `mechanism_a_write_and_verify_sealed_shard`'s own in-memory
    // `bytes` argument intended.
    //
    // The function's post-hoc disk read-back is the ONLY thing that can
    // catch this: an in-memory-only length check (the shipped,
    // pre-pass-6-fix behavior PC6(c)/Invariant 4's ruling corrects) would
    // never re-read `sealed_path` from disk at all and would report
    // `Ok(())` here regardless of the corruption -- so this test would FAIL
    // if the disk read-back step were ever removed or downgraded back to an
    // in-memory-only check. It genuinely distinguishes "read-back verified"
    // from "no read-back," not merely a renamed/asserted-only paper-fix.
    let mut caught = None;
    for _attempt in 0..20 {
        let dir = tempfile::tempdir().unwrap();
        let sealed_path = dir.path().join("decision-log.0001.md");
        let intended =
            b"legitimate sealed shard content, exactly as intended -- 55 bytes!!".to_vec();
        let corrupt = b"CORRUPTED-ON-DISK-BEFORE-RENAME-DIFFERENT-CONTENT".to_vec();

        let (stop, handle) = spawn_temp_file_corruptor(&sealed_path, corrupt);

        let result =
            mechanism_a_write_and_verify_sealed_shard(&sealed_path, &intended, "decision-log");

        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        handle.join().expect("corrupting thread must not panic");

        if matches!(
            result,
            Err(MechanismABackfillError::ContentPreservationFailed { .. })
        ) {
            caught = Some((result, sealed_path, intended, dir));
            break;
        }
        // This attempt's corrupting write didn't land before the rename
        // (a transient scheduling miss, not a code defect) -- retry with a
        // fresh fixture. `dir` (and its temp files) drop here.
    }

    let (result, sealed_path, intended, _dir) = caught.expect(
        "PC6(c)/Invariant 4 (F-C3-P6-002): the disk-corruption race never landed a mismatch in \
         20 attempts. Empirically this race is reliable (200/200 in local validation), so a \
         run of 20 straight misses most likely means the read-back gate itself is missing or \
         broken (every attempt silently reported Ok), not scheduling bad luck -- re-run with \
         `--nocapture` and inspect `mechanism_a_write_and_verify_sealed_shard`'s own read-back \
         step if this reproduces.",
    );

    assert!(
        matches!(
            result,
            Err(MechanismABackfillError::ContentPreservationFailed { .. })
        ),
        "PC6(c)/Invariant 4 (F-C3-P6-002): a sealed shard whose on-disk bytes were corrupted \
         between `write_atomic`'s own rename and this function's post-hoc read-back must abort \
         fail-loud with `ContentPreservationFailed`, never silently report success. Got: \
         {result:?}"
    );
    if let Err(MechanismABackfillError::ContentPreservationFailed {
        artifact_stem,
        detail,
    }) = &result
    {
        assert_eq!(artifact_stem, "decision-log");
        assert!(
            detail.contains("read-back") || detail.contains("read back"),
            "the fail-loud detail message should name the read-back mismatch as the cause: \
             {detail}"
        );
    }

    // Sanity: the race must have genuinely landed corrupted bytes on disk
    // (not raced without effect) -- confirms the mismatch this test caught
    // was a real filesystem-level divergence, not an artifact of the
    // assertion above alone.
    let on_disk = std::fs::read(&sealed_path).unwrap();
    assert_ne!(
        on_disk, intended,
        "sanity: the corrupting race must have genuinely landed corrupted bytes on disk"
    );
}
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// F-C3-P6-003 (BC-1.18.008 v1.6 Record-Boundary Marker Table, `lessons.md`
// row): the marker table's `lessons.md` id-tag shape is
// `^L-<tag>-[0-9]+\b` for BOTH the h3 confirmed-exception form
// (`is_lesson_record_heading`) and the h2 primary form
// (`is_lesson_h2_record_heading`) -- both share the SAME
// `is_id_tagged_lesson_heading` predicate. That predicate only checks that
// the byte immediately following the tag's trailing `-` is an ASCII digit;
// it never checks what follows the digit RUN, so a heading whose numeric id
// is immediately followed by another word character (no `\b` word
// boundary) is misdetected as a genuine tagged record -- `### L-EDP1-050details`
// and `### L-EDP1-050_extra` merely SHARE the `L-EDP1-050` id prefix with a
// genuine record; they are prose in that record's own body, not new
// records.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_008_FC3P6003_record_boundary_offsets_lessons_h3_id_tag_requires_word_boundary_after_digits()
 {
    let genuine_bare = "### L-EDP1-050\n";
    let genuine_titled = "### L-EDP1-050 title\n";
    let decoy_suffix_word = "### L-EDP1-050details\n";
    let decoy_underscore = "### L-EDP1-050_extra\n";
    let decoy_trailing_char = "### L-EDP1-050x\n";

    let content = format!(
        "# Lessons Learned\n\n\
         {genuine_bare}\
         Body one.\n\n\
         {decoy_suffix_word}\
         Nested prose that merely shares the L-EDP1-050 id prefix -- not a new record.\n\n\
         {decoy_underscore}\
         Same -- an underscore is a word character too, so no boundary exists here either.\n\n\
         {decoy_trailing_char}\
         Same again -- a trailing alnum char with no separator is still no boundary.\n\n\
         {genuine_titled}\
         Body two.\n"
    );

    let offset_bare = content.find(genuine_bare).unwrap();
    let offset_titled = content.find(genuine_titled).unwrap();
    let offset_decoy_word = content.find(decoy_suffix_word).unwrap();
    let offset_decoy_underscore = content.find(decoy_underscore).unwrap();
    let offset_decoy_trailing = content.find(decoy_trailing_char).unwrap();

    let offsets = mechanism_a_record_boundary_offsets("lessons", content.as_bytes());

    assert!(
        !offsets.contains(&offset_decoy_word)
            && !offsets.contains(&offset_decoy_underscore)
            && !offsets.contains(&offset_decoy_trailing),
        "F-C3-P6-003/PC2 Record-Boundary Marker Table (`^### L-<tag>-[0-9]+\\b`): a heading whose \
         numeric id run is immediately followed by another word character (no word boundary) \
         must NOT be treated as a record boundary -- `### L-EDP1-050details`, \
         `### L-EDP1-050_extra`, and `### L-EDP1-050x` each merely SHARE the `L-EDP1-050` id \
         prefix with a genuine record, they are not one. Got: {offsets:?} (decoys at \
         {offset_decoy_word}, {offset_decoy_underscore}, {offset_decoy_trailing})"
    );
    assert_eq!(
        offsets,
        vec![offset_bare, offset_titled],
        "F-C3-P6-003: only the two genuine `### L-EDP1-050` headings (bare, and followed by a \
         space) are real record boundaries. Got: {offsets:?}"
    );
}

#[test]
fn test_BC_1_18_008_FC3P6003_record_boundary_offsets_lessons_h2_id_tag_requires_word_boundary_after_digits()
 {
    // Same F-C3-P6-003 defect, exercised through the h2 primary-form caller
    // (`is_lesson_h2_record_heading`) -- both h2 and h3 lessons.md detection
    // share the SAME `is_id_tagged_lesson_heading` predicate, so the missing
    // word-boundary check affects both marker forms identically.
    let genuine_bare = "## L-EDP1-052\n";
    let genuine_titled = "## L-EDP1-053 title\n";
    let decoy_suffix_word = "## L-EDP1-052details\n";
    let decoy_underscore = "## L-EDP1-052_extra\n";
    let decoy_trailing_char = "## L-EDP1-052x\n";

    let content = format!(
        "# Lessons Learned\n\n\
         {genuine_bare}\
         Body one.\n\n\
         {decoy_suffix_word}\
         Nested prose sharing the id prefix -- not a new record.\n\n\
         {decoy_underscore}\
         Same defect via an underscore suffix.\n\n\
         {decoy_trailing_char}\
         Same defect via a trailing alnum char.\n\n\
         {genuine_titled}\
         Body two.\n"
    );

    let offset_bare = content.find(genuine_bare).unwrap();
    let offset_titled = content.find(genuine_titled).unwrap();
    let offset_decoy_word = content.find(decoy_suffix_word).unwrap();
    let offset_decoy_underscore = content.find(decoy_underscore).unwrap();
    let offset_decoy_trailing = content.find(decoy_trailing_char).unwrap();

    let offsets = mechanism_a_record_boundary_offsets("lessons", content.as_bytes());

    assert!(
        !offsets.contains(&offset_decoy_word)
            && !offsets.contains(&offset_decoy_underscore)
            && !offsets.contains(&offset_decoy_trailing),
        "F-C3-P6-003 (h2 form): a heading whose numeric id run is immediately followed by \
         another word character must NOT be a record boundary. Got: {offsets:?}"
    );
    assert_eq!(
        offsets,
        vec![offset_bare, offset_titled],
        "F-C3-P6-003 (h2 form): only the two genuine `## L-EDP1-NNN` headings are real record \
         boundaries. Got: {offsets:?}"
    );
}

// ---------------------------------------------------------------------------
// F-C3-P7-002 (BC-1.18.008 v1.7 Record-Boundary Marker Table, `decision-log.md`
// row; EC-012): a LOCAL adversarial pass-7 review found the decision-log
// primary-key regex was still the bare-only `^\| D-[0-9]+ \|` form
// (`is_decision_log_row_marker`) even though real `decision-log.md` content
// in both cycles carries two confirmed sub-clause-suffix row shapes:
// parenthetical suffixes (`| D-440(a) |`, the combined `| D-446(a/b/c/d/e) |`
// form) and the hyphenated non-parenthetical suffix (`| D-355-AMEND |`). The
// bare-only regex silently fails to match any of them, under-segmenting the
// artifact by absorbing each sub-clause row into the PRECEDING record --
// this test pins the corrected regex,
// `^\| D-[0-9]+(\([a-z0-9/]+\)|-[A-Za-z]+)? \|`, treating all three forms
// as boundaries identical in kind to a bare `| D-NNN |` row.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_008_FC3P7002_EC012_record_boundary_offsets_decision_log_matches_subclause_and_amend_rows()
 {
    let row_bare = "| D-100 | a bare decision row | author |\n";
    let row_paren_single = "| D-440(a) | single-letter sub-clause suffix | author |\n";
    let row_paren_combined = "| D-446(a/b/c/d/e) | combined sub-clause suffix | author |\n";
    let row_hyphen_amend = "| D-355-AMEND | hyphenated amend suffix | author |\n";
    let non_row_no_digits = "| D-something, not a row, continues a multi-line cell...\n";
    let appendix_heading = "## Appendix: Sub-clause Expansion\n";
    let appendix_subclause = "### D-440 (F5 pass-60 codification block)\n";

    let content = format!(
        "# decision-log\n\n\
         ## Decisions Log\n\n\
         {row_bare}\
         {row_paren_single}\
         {row_paren_combined}\
         {row_hyphen_amend}\
         {non_row_no_digits}\n\
         {appendix_heading}\n\
         {appendix_subclause}\
         Sub-clause expansion detail (elided).\n"
    );

    let offset_bare = content.find(row_bare).unwrap();
    let offset_paren_single = content.find(row_paren_single).unwrap();
    let offset_paren_combined = content.find(row_paren_combined).unwrap();
    let offset_hyphen_amend = content.find(row_hyphen_amend).unwrap();
    let offset_non_row = content.find(non_row_no_digits).unwrap();
    let offset_appendix = content.find(appendix_heading).unwrap();
    let offset_subclause = content.find(appendix_subclause).unwrap();

    let offsets = mechanism_a_record_boundary_offsets("decision-log", content.as_bytes());

    assert_eq!(
        offsets,
        vec![
            offset_bare,
            offset_paren_single,
            offset_paren_combined,
            offset_hyphen_amend
        ],
        "EC-012/F-C3-P7-002's corrected primary-key regex \
         `^\\| D-[0-9]+(\\([a-z0-9/]+\\)|-[A-Za-z]+)? \\|` MUST detect the bare `| D-NNN |` form \
         AND both confirmed sub-clause-suffix forms -- the parenthetical single-letter suffix \
         (`| D-440(a) |`), the combined multi-letter parenthetical suffix \
         (`| D-446(a/b/c/d/e) |`), and the hyphenated non-parenthetical suffix \
         (`| D-355-AMEND |`) -- as boundaries identical in kind to a bare row. The PRIOR \
         bare-only regex `^\\| D-[0-9]+ \\|` silently failed to match any of the three suffix \
         forms, under-segmenting the artifact by absorbing each sub-clause row into the preceding \
         record (the defect F-C3-P7-002 corrects). Got: {offsets:?}"
    );
    assert!(
        !offsets.contains(&offset_non_row)
            && !offsets.contains(&offset_appendix)
            && !offsets.contains(&offset_subclause),
        "the corrected regex must still EXCLUDE a non-row wrapped-prose continuation line (no \
         digits immediately after the `| D-` prefix), the `## Appendix: Sub-clause Expansion` \
         section label, and its nested `### D-NNN (...)` Appendix h3 blocks -- none of these are \
         PRIMARY partition boundaries for decision-log.md. Got: {offsets:?}"
    );
}

// ---------------------------------------------------------------------------
// F-C3-P7-001 (BC-1.18.008 v1.7 Postcondition 5's Manifest-Authoritative
// Slice-and-Verify Rule; EC-011; Invariant 3): a LOCAL adversarial pass-7
// review found `heal_or_confirm_already_migrated`'s DANGEROUS-window heal
// derives its slice offset by SUMMING the shard-index's own per-shard
// `bytes_at_seal` fields (`index.shards.iter().map(|s| s.bytes_at_seal).sum()`)
// and writes the resulting slice via `write_atomic_bytes` UNCONDITIONALLY --
// with no verification against the Backfill Recovery Manifest's own
// `final_bytes`/`final_sha256` fields at all, and no post-hoc disk read-back
// of its own write. This is precisely the defect the BC's "Rationale" prose
// names: a corrupted or stale `bytes_at_seal` value on any sealed shard (or
// a corrupted Manifest field, or a corrupted on-disk write) silently mis-heals
// the canonical file with NO check catching it.
//
// The corrected behavior (Postcondition 5's Manifest-Authoritative
// Slice-and-Verify Rule):
//   1. Derive `offset = original_bytes - final_bytes` from the Manifest
//      itself, NEVER from summing shard-index `bytes_at_seal` fields.
//   2. Verify `sliced.len() == final_bytes AND sha256(sliced) == final_sha256`
//      before writing anything.
//   3. On success, write via `write_atomic`, then perform a FRESH disk
//      read-back confirming `(length, SHA-256) == (final_bytes, final_sha256)`
//      -- the SAME post-hoc disk-read-back discipline
//      `mechanism_a_write_and_verify_sealed_shard` (F-C3-P6-002) already
//      gives sealed shard writes. On ANY mismatch at either step 2 or step
//      3, fail loud with `E-SHD-012` and write nothing (step 2) or surface
//      the corruption immediately (step 3, since the destructive write
//      already landed by then).
// ---------------------------------------------------------------------------

/// Corrupts the on-disk `[backfill_manifest]` table's `final_sha256` value at
/// `index_path` to a DIFFERENT (still syntactically valid) hex-looking
/// string, leaving every other manifest field (`original_bytes`,
/// `original_sha256`, `final_bytes`) untouched and correct -- targets
/// exactly the failure mode EC-011's own text names: "the Manifest's own
/// `final_bytes`/`final_sha256` fields are themselves in an inconsistent
/// state." Reversing the real digest (rather than fabricating an unrelated
/// one) guarantees a value that is still a same-length string without this
/// test needing to compute or know the real SHA-256 digest itself.
fn corrupt_manifest_final_sha256(index_path: &Path) {
    let text = std::fs::read_to_string(index_path)
        .expect("the shard-index file must exist after a successful first migration");
    let key = "final_sha256 = \"";
    let key_start = text
        .find(key)
        .expect("the published shard-index must carry a `final_sha256` field to corrupt");
    let value_start = key_start + key.len();
    let value_end = text[value_start..]
        .find('"')
        .map(|i| value_start + i)
        .expect("`final_sha256`'s value must be a quoted TOML string");
    let original_value = text[value_start..value_end].to_string();
    let corrupted_value: String = original_value.chars().rev().collect();
    assert_ne!(
        corrupted_value, original_value,
        "sanity: reversing a real SHA-256 hex digest must differ from the original (a palindrome \
         digest is astronomically unlikely for real content)"
    );
    let mut corrupted_text = text;
    corrupted_text.replace_range(value_start..value_end, &corrupted_value);
    std::fs::write(index_path, corrupted_text)
        .expect("writing the corrupted shard-index back to disk must succeed");
}

#[test]
fn test_BC_1_18_008_FC3P7001_EC011_run_backfill_split_heal_aborts_e_shd_012_when_manifest_slice_verification_fails()
 {
    // The load-bearing EC-011 test: forces a DANGEROUS-window recovery where
    // the Manifest-derived candidate slice does NOT verify against the
    // Manifest's own (here, deliberately corrupted) `final_sha256` field --
    // Postcondition 5 step 2's pre-write gate MUST catch this and abort
    // loud with `E-SHD-012`, writing nothing. Distinct from EC-010's
    // `E-SHD-011`: the TOP-LEVEL `(length, hash)` check here still
    // unambiguously confirms DANGEROUS (canonical bytes exactly match the
    // manifest's UNCORRUPTED `original_bytes`/`original_sha256` pair) -- the
    // failure is ONE LEVEL DEEPER, in the slice-verification step itself.
    let a = b"## Checkpoint\nx\n".to_vec(); // 16 bytes
    let mut original_content = a.clone();
    original_content.extend_from_slice(&a);
    original_content.extend_from_slice(b"more\n"); // 37 bytes total

    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("session-checkpoints.md");
    std::fs::write(&canonical_path, &original_content).unwrap();

    let boundaries = mechanism_a_record_boundary_offsets("session-checkpoints", &original_content);
    let entry = flat_entry("session-checkpoints", 16);

    let first = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 10)
        .expect("the first, uninterrupted migration over this fixture must succeed");
    assert_eq!(
        first,
        MechanismABackfillOutcome::Migrated {
            sealed_count: 1,
            archived_count: 0
        }
    );

    let index_path = dir.path().join("session-checkpoints.shard-index.toml");
    corrupt_manifest_final_sha256(&index_path);

    // Simulate the confirmed DANGEROUS window: canonical bytes still exactly
    // match the manifest's UNCORRUPTED `original_bytes`/`original_sha256`
    // pair, as if the prior run's canonical-truncate write never ran.
    std::fs::write(&canonical_path, &original_content).unwrap();

    let result = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 10);

    let err = result.expect_err(
        "EC-011/Postcondition 5's Manifest-Authoritative Slice-and-Verify Rule: when the derived \
         slice's SHA-256 does not match the Manifest's own (corrupted) `final_sha256`, the heal \
         MUST fail loud with `E-SHD-012` and MUST NOT write the unverified slice -- silently \
         writing a slice that was never confirmed against the Manifest is exactly the class of \
         defect this amendment closes (the shipped `canonical_bytes[sealed_len..]` write with no \
         verification at all).",
    );
    let err_message = err.to_string();
    assert!(
        err_message.contains("E-SHD-012"),
        "EC-011: the slice-verification-failure disposition must surface the NEW `E-SHD-012` \
         error code (distinct from EC-010's top-level `E-SHD-011`) -- got: {err_message}"
    );

    let post = std::fs::read(&canonical_path).unwrap();
    assert_eq!(
        post, original_content,
        "Postcondition 5 step 3: on ANY slice-verification mismatch the heal MUST NOT write \
         anything to the canonical file -- it must remain exactly as found (the confirmed \
         DANGEROUS-window original content), pending operator investigation."
    );
}

#[test]
fn test_BC_1_18_008_FC3P7001_run_backfill_split_heal_offset_derived_from_manifest_never_shard_index_bytes_at_seal()
 {
    // Positive companion to the EC-011 abort test above, and the direct
    // regression pin for the BC's own "Rationale" paragraph: a genuine
    // DANGEROUS window whose Manifest-derived slice DOES verify must still
    // heal correctly to the manifest-verified final content -- even when
    // the shard-index's own `bytes_at_seal` field (the PRIOR implementation's
    // sole offset source, now retired) is corrupted to a wrong value. This
    // proves the offset is Manifest-derived (`original_bytes - final_bytes`),
    // never re-derived by summing shard-index `bytes_at_seal` entries.
    let a = b"## Checkpoint\nx\n".to_vec(); // 16 bytes
    let mut original_content = a.clone();
    original_content.extend_from_slice(&a);
    original_content.extend_from_slice(b"more\n"); // 37 bytes total

    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("session-checkpoints.md");
    std::fs::write(&canonical_path, &original_content).unwrap();

    let boundaries = mechanism_a_record_boundary_offsets("session-checkpoints", &original_content);
    let entry = flat_entry("session-checkpoints", 16);

    let first = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 10)
        .expect("the first, uninterrupted migration over this fixture must succeed");
    assert_eq!(
        first,
        MechanismABackfillOutcome::Migrated {
            sealed_count: 1,
            archived_count: 0
        }
    );

    // Corrupt the published shard-index's `bytes_at_seal` field for the one
    // sealed shard (16 -> 5) -- a LEGACY offset-derivation (summing
    // `bytes_at_seal` across shards, the exact defect this amendment
    // retires) would compute a WRONG split point, while the Backfill
    // Recovery Manifest's own `original_bytes`/`final_bytes`/`*_sha256`
    // fields remain untouched and correct.
    let index_path = dir.path().join("session-checkpoints.shard-index.toml");
    let index_toml = std::fs::read_to_string(&index_path).unwrap();
    assert!(
        index_toml.contains("bytes_at_seal = 16"),
        "fixture assumption: the one sealed shard's `bytes_at_seal` must be 16 pre-corruption. \
         Got:\n{index_toml}"
    );
    let corrupted_index_toml = index_toml.replacen("bytes_at_seal = 16", "bytes_at_seal = 5", 1);
    std::fs::write(&index_path, &corrupted_index_toml).unwrap();

    // Simulate the confirmed DANGEROUS window.
    std::fs::write(&canonical_path, &original_content).unwrap();

    let expected_final = original_content[16..].to_vec(); // "A" + "more\n", 21 bytes -- the REAL final partition

    let second = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 10).expect(
        "a re-invocation against the confirmed DANGEROUS window must self-heal correctly even \
         with a corrupted shard-index `bytes_at_seal` field -- the Manifest, not the shard index, \
         is the sole authoritative offset source (Invariant 3)",
    );
    assert_eq!(
        second,
        MechanismABackfillOutcome::Healed { sealed_count: 1 }
    );

    let healed = std::fs::read(&canonical_path).unwrap();
    assert_eq!(
        healed,
        expected_final,
        "Postcondition 5's Manifest-Authoritative Slice-and-Verify Rule (F-C3-P7-001): the heal's \
         slice offset MUST be derived from the Backfill Recovery Manifest's own \
         `original_bytes - final_bytes` arithmetic, NEVER by summing the shard-index's \
         `bytes_at_seal` fields -- a corrupted `bytes_at_seal` (5 instead of the real 16) must NOT \
         perturb the healed result at all. Got {} bytes: {:?}",
        healed.len(),
        healed
    );
}

#[test]
fn test_BC_1_18_008_FC3P7001_run_backfill_split_heal_write_receives_disk_read_back_verification() {
    // v1.7's "Extension to the DANGEROUS-window heal write" (Postcondition
    // 6, this amendment F-C3-P7-001): the heal's own destructive write to
    // the canonical file must receive the SAME post-hoc disk read-back
    // discipline `mechanism_a_write_and_verify_sealed_shard` already gives
    // sealed shard writes (F-C3-P6-002) -- an in-memory-only confidence that
    // `write_atomic` succeeded does NOT satisfy this; it cannot detect a
    // write that silently truncated, partially flushed, or otherwise landed
    // corrupted bytes on disk, at the exact moment the pre-heal content
    // becomes irretrievably gone.
    //
    // Fault-injection mechanism: this reuses `spawn_temp_file_corruptor` --
    // the SAME real, non-simulated disk race the F-C3-P6-002 sealed-shard
    // tests above establish against `write_atomic`'s own deterministic
    // sibling temp-file naming convention -- this time targeting
    // `canonical_path` itself, since `heal_or_confirm_already_migrated`'s
    // own write is a `write_atomic_bytes(canonical_path, ...)` call using
    // the EXACT SAME underlying `write_atomic` primitive as sealed shard
    // writes. This IS reachable at the public `run_mechanism_a_backfill_split`
    // entry point with no source change: re-invoking it against a confirmed
    // DANGEROUS window exercises the heal path (and therefore the heal's own
    // write) automatically.
    let a = b"## Checkpoint\nx\n".to_vec(); // 16 bytes
    let mut original_content = a.clone();
    original_content.extend_from_slice(&a);
    original_content.extend_from_slice(b"more\n"); // 37 bytes total

    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("session-checkpoints.md");
    std::fs::write(&canonical_path, &original_content).unwrap();

    let boundaries = mechanism_a_record_boundary_offsets("session-checkpoints", &original_content);
    let entry = flat_entry("session-checkpoints", 16);

    let first = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 10)
        .expect("the first, uninterrupted migration over this fixture must succeed");
    assert_eq!(
        first,
        MechanismABackfillOutcome::Migrated {
            sealed_count: 1,
            archived_count: 0
        }
    );

    let expected_final = original_content[16..].to_vec(); // "A" + "more\n", 21 bytes
    let corrupt = b"CORRUPTED-DURING-HEAL-WRITE-BEFORE-RENAME-DIFFERENT-LENGTH".to_vec();

    let mut caught = None;
    for _attempt in 0..20 {
        // Re-arm the confirmed DANGEROUS window fresh for every attempt --
        // a prior failed attempt may have left corrupted bytes on disk (the
        // heal's own destructive write already happened by the time its
        // read-back would catch it), so it must never be reused as the next
        // attempt's starting state.
        std::fs::write(&canonical_path, &original_content).unwrap();

        let (stop, handle) = spawn_temp_file_corruptor(&canonical_path, corrupt.clone());

        let result = run_mechanism_a_backfill_split(&entry, &canonical_path, &boundaries, 10);

        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        handle.join().expect("corrupting thread must not panic");

        let on_disk = std::fs::read(&canonical_path).unwrap();
        if on_disk != expected_final {
            // The race landed corrupted bytes on disk before the rename --
            // a real, filesystem-level divergence, independent of whether
            // this (possibly still-defective) implementation caught it.
            caught = Some((result, on_disk));
            break;
        }
        // This attempt's corrupting write didn't land before the rename (a
        // transient scheduling miss, not a code defect) -- retry with a
        // fresh attempt.
    }

    let (result, on_disk) = caught.expect(
        "F-C3-P7-001: the heal-write disk-corruption race never landed a mismatch in 20 attempts \
         -- re-run with --nocapture and inspect the heal's own write step if this reproduces (the \
         sealed-shard F-C3-P6-002 race above is empirically 100% reliable across 200 local trials \
         against the same `write_atomic` primitive).",
    );

    assert!(
        result.is_err(),
        "Postcondition 6(c)/Invariant 4's F-C3-P7-001 extension: the heal's own destructive write \
         to the canonical file MUST receive a FRESH post-hoc disk read-back confirming \
         `(length, SHA-256) == (final_bytes, final_sha256)` before reporting the heal complete -- \
         an in-memory-only confidence in `write_atomic`'s own return value (the current gap this \
         test pins) cannot detect a write that landed corrupted bytes on disk. Got Ok(_) over \
         on-disk bytes that do NOT match the manifest-verified final content ({} bytes, expected \
         {}): {:?}",
        on_disk.len(),
        expected_final.len(),
        on_disk
    );
    let err_message = result.unwrap_err().to_string();
    assert!(
        err_message.contains("E-SHD-012"),
        "the heal-write read-back mismatch must surface the E-SHD-012 error code (the SAME code \
         EC-011's pre-write slice-verification failure uses, since both are hard gates of the \
         Manifest-Authoritative Slice-and-Verify Rule / its Postcondition 6(c) extension): \
         {err_message}"
    );
    assert_ne!(
        on_disk, expected_final,
        "sanity: the corrupting race must have genuinely landed corrupted bytes on disk, not the \
         correct manifest-verified final content"
    );
}
