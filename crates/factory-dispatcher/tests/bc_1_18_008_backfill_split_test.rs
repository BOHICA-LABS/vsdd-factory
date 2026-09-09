// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! BC-1.18.008 (S-25.02 F4 BC-cluster 3 "retention+backfill") RED GATE
//! coverage for the mechanism-A one-time backfill-split functions in
//! `shard_manager.rs` (AC-013/AC-014).
//!
//! # BC-5.38.001 Red Gate discipline — RED (all tests FAIL against the stub)
//!
//! Every non-trivial function this file drives
//! (`mechanism_a_record_boundary_offsets`, `mechanism_a_partition_for_backfill`,
//! `mechanism_a_verify_backfill_content_preserved`,
//! `mechanism_a_verify_backfill_record_counts_preserved`,
//! `mechanism_a_backfill_already_migrated`, `run_mechanism_a_backfill_split`)
//! is `todo!()` as of the stub-architect's cluster-3 burst. Every test below
//! asserts the REAL, spec-mandated expected outcome (never `#[should_panic]`)
//! so each test currently fails via the `todo!()` panic (or, once
//! implementer lands a partial implementation, via a normal assertion
//! failure) until real logic replaces the stub.
//!
//! # Content-preservation hard-gate coverage note (AC-014, Postcondition 6)
//!
//! `run_mechanism_a_backfill_split`'s own future implementation computes its
//! partitions internally (via `mechanism_a_partition_for_backfill`, whose
//! output always structurally satisfies content-preservation by
//! construction) — there is no external caller-visible way to inject a
//! genuine content/record-count MISMATCH into that internal call and force
//! the Postcondition 6 hard gate to fire an abort from outside the module.
//! The two `mechanism_a_verify_backfill_*` predicates that make up that hard
//! gate are therefore covered directly at the unit level below (both their
//! `true` and `false` outcomes), which is the correct-grained place to
//! Red-Gate-test a boolean predicate's own logic; `run_mechanism_a_backfill_split`'s
//! own tests cover its ORCHESTRATION of a successful (gate-passing) run,
//! plus EC-016/EC-017/idempotency/crash-atomicity.

use factory_dispatcher::shard_manager::{
    MechanismABackfillOutcome, MechanismABackfillPartition, ShardEntry, ShardShape,
    mechanism_a_backfill_already_migrated, mechanism_a_partition_for_backfill,
    mechanism_a_record_boundary_offsets, mechanism_a_verify_backfill_content_preserved,
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
fn test_BC_1_18_008_PC2_record_boundary_offsets_burst_log_finds_heading_starts() {
    let heading1 = "### Burst 1 — first burst\n";
    let heading2 = "### Burst 2 — second burst\n";
    let content = format!("# burst-log\n\n{heading1}body one\n\n{heading2}body two\n");

    let offset1 = content.find(heading1).unwrap();
    let offset2 = content.find(heading2).unwrap();

    let offsets = mechanism_a_record_boundary_offsets("burst-log", content.as_bytes());

    assert_eq!(
        offsets,
        vec![offset1, offset2],
        "PC2: burst-log.md's structural record boundaries are its own `### <burst-heading>` \
         block starts"
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
