// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! BC-1.18.007 (S-25.02 F4 BC-cluster 3 "retention+backfill") coverage for
//! the retention/compaction functions in `shard_manager.rs`
//! (AC-010/AC-011/AC-012).
//!
//! # Current implementation status (MED-E header correction)
//!
//! Every non-trivial function this file drives
//! (`load_shard_index_for_retention_check`, `retention_overflow_count`,
//! `archive_overflow_shards`, `whole_corpus_shard_paths`) is IMPLEMENTED
//! (not `todo!()`) as of the implementer's cluster-3 burst. All tests in
//! this file PASS against the current implementation — the prior claim that
//! every function was `todo!()` and every test "presently FAILS" is stale
//! and no longer describes this file's actual state. Each test below still
//! asserts the REAL, spec-mandated expected outcome (never
//! `#[should_panic]`) — the same methodology `bc_1_18_005_shard_cap_trigger_test.rs`
//! and `bc_1_18_006_roll_test.rs` establish for this crate — and this file
//! now serves as permanent regression coverage for AC-010/AC-011/AC-012.
//!
//! Deliberately NOT covered here (GREEN-BY-DESIGN / WIRING-EXEMPT per
//! `shard_manager.rs`'s own per-function doc comments, mirroring cluster-1/2's
//! own precedent of excluding trivial helpers from dedicated coverage):
//! `default_retention_count`, `archived_shard_path`,
//! `archived_shard_index_path_string`, `shard_index_entry_is_archived`, and
//! `From<ShardRetentionError> for HookResult`. These are single-expression,
//! zero-branching helpers with no domain decision for a test to exercise
//! non-trivially; this file uses them internally as fixture-construction
//! helpers where convenient, but the enumerated Red-Gate-dispatch surface for
//! AC-010/AC-011/AC-012 is the four `todo!()` functions listed above.
//!
//! AC-012's POLICY-1 present-day enforcement mechanism (BC-1.18.007
//! Postcondition 6) is agent-instruction-level (`.factory/policies.yaml`
//! `verification_steps` + the `consistency-validator`/adversary-prompt agent
//! definitions) — that content lives on the `factory-artifacts` orphan
//! branch, not in this crate's own git history, so it is out of scope for a
//! `cargo test` file in this repository's `develop`-descended tree. This
//! file covers AC-012's CODE-level obligation only: `whole_corpus_shard_paths`
//! with `WholeCorpusGlobScope::ArchiveInclusive` (the mechanism the
//! agent-level instruction is required to invoke).

use factory_dispatcher::shard_manager::{
    ShardIndex, ShardIndexEntry, ShardRetentionError, WholeCorpusGlobScope,
    archive_overflow_shards, load_shard_index_for_retention_check, retention_overflow_count,
    whole_corpus_shard_paths,
};

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

/// A well-formed `ShardIndex` fixture — same calibration constants as
/// `bc_1_18_005_shard_cap_trigger_test.rs`'s / `bc_1_18_006_roll_test.rs`'s
/// own `FLAT_SHARD_CONFIG` (cap 49,152), so this cluster's fixtures stay
/// self-consistent with the already-validated formula ceiling.
fn sample_index(
    artifact_stem: &str,
    retention_count: u32,
    shards: Vec<ShardIndexEntry>,
) -> ShardIndex {
    ShardIndex {
        schema_version: 1,
        artifact_stem: artifact_stem.to_string(),
        current_shard: format!("{artifact_stem}.md"),
        shard_cap_bytes: 49_152,
        max_single_record_bytes: 16_384,
        safety_margin_bytes: 8_192,
        practical_fuel_ceiling: 8_000_000,
        worst_case_fuel_per_byte: 106.36,
        retention_count,
        shards,
    }
}

/// An ACTIVE (un-archived) `[[shard]]` entry — a bare sealed-shard filename
/// sibling to the canonical file, per `shard_index_entry_is_archived`'s own
/// "never starts with `archive/`" contract.
fn active_entry(seq: u32, artifact_stem: &str, bytes_at_seal: u64) -> ShardIndexEntry {
    ShardIndexEntry {
        seq,
        path: format!("{artifact_stem}.{seq:04}.md"),
        sealed_at: "2026-01-01T00:00:00Z".to_string(),
        bytes_at_seal,
        sealed_retroactively: false,
    }
}

/// An ALREADY-ARCHIVED `[[shard]]` entry — its `path` already rewritten to
/// `archived_shard_index_path_string`'s form (BC-1.18.007 Invariant 3's
/// path-mutation implementation choice, per `shard_manager.rs`'s own scope
/// note).
fn archived_entry(seq: u32, artifact_stem: &str, bytes_at_seal: u64) -> ShardIndexEntry {
    ShardIndexEntry {
        seq,
        path: format!("archive/{artifact_stem}/{artifact_stem}.{seq:04}.md"),
        sealed_at: "2026-01-01T00:00:00Z".to_string(),
        bytes_at_seal,
        sealed_retroactively: false,
    }
}

fn write_index_toml(index_path: &std::path::Path, index: &ShardIndex) {
    let text = toml::to_string(index).expect("ShardIndex must serialize to valid TOML");
    std::fs::write(index_path, text).expect("write shard-index.toml fixture");
}

fn index_path_for(canonical_path: &std::path::Path, artifact_stem: &str) -> std::path::PathBuf {
    canonical_path
        .parent()
        .expect("canonical_path must have a parent directory")
        .join(format!("{artifact_stem}.shard-index.toml"))
}

// ---------------------------------------------------------------------------
// AC-010/AC-011 — `load_shard_index_for_retention_check`
// (BC-1.18.007 Postcondition 1, EC-005/E-SHD-002)
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_007_PC1_load_shard_index_for_retention_check_returns_persisted_index() {
    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    std::fs::write(&canonical_path, "current shard content").unwrap();

    let fixture_index = sample_index(
        "decision-log",
        5,
        vec![
            active_entry(1, "decision-log", 40_000),
            active_entry(2, "decision-log", 41_000),
        ],
    );
    write_index_toml(
        &index_path_for(&canonical_path, "decision-log"),
        &fixture_index,
    );

    // AC-010/Postcondition 1: retention_count is READ from the shard-index,
    // never hardcoded — this test's fixture uses 5, not the default 10, to
    // prove the value round-trips rather than being silently replaced by a
    // hardcoded constant.
    let loaded = load_shard_index_for_retention_check(&canonical_path, "decision-log")
        .expect("BC-1.18.007 PC1: a well-formed, existing shard-index must load successfully");

    assert_eq!(
        loaded.retention_count, 5,
        "AC-010/PC1: retention_count must be read from the persisted index, not hardcoded"
    );
    assert_eq!(
        loaded.shards.len(),
        2,
        "PC1: every persisted [[shard]] entry must round-trip"
    );
    assert_eq!(loaded.shards[0].seq, 1);
    assert_eq!(loaded.shards[1].seq, 2);
    assert_eq!(loaded.artifact_stem, "decision-log");
}

#[test]
fn test_BC_1_18_007_EC005_load_shard_index_for_retention_check_missing_index_fails_loud_e_shd_002()
{
    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    std::fs::write(&canonical_path, "current shard content").unwrap();
    // Deliberately NOT writing a shard-index sibling file: a seal is known to
    // have just occurred (Precondition 1), so an absent index here is an
    // anomaly, never the legitimate "no roll has ever occurred yet" case
    // this function's own doc comment carves out.

    let result = load_shard_index_for_retention_check(&canonical_path, "decision-log");

    let err = result.expect_err(
        "EC-005: a missing shard-index at retention-check time MUST fail loud, never silently \
         proceed as if no archival were needed",
    );
    assert!(
        matches!(err, ShardRetentionError::IndexUnavailable { .. }),
        "EC-005: the missing-index failure must be IndexUnavailable, got {err:?}"
    );
    assert!(
        err.to_string().contains("E-SHD-002"),
        "EC-005: the error message must carry the E-SHD-002 error-taxonomy code. Got: {err}"
    );
}

#[test]
fn test_BC_1_18_007_EC005_load_shard_index_for_retention_check_corrupt_toml_fails_loud_e_shd_002() {
    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    std::fs::write(&canonical_path, "current shard content").unwrap();
    std::fs::write(
        index_path_for(&canonical_path, "decision-log"),
        "this is not valid TOML [[[ %%% ===",
    )
    .unwrap();

    let result = load_shard_index_for_retention_check(&canonical_path, "decision-log");

    let err = result.expect_err(
        "EC-005: a corrupt (malformed TOML) shard-index at retention-check time MUST fail loud",
    );
    assert!(
        matches!(err, ShardRetentionError::IndexUnavailable { .. }),
        "EC-005: a corrupt index's failure must also be IndexUnavailable, got {err:?}"
    );
    assert!(
        err.to_string().contains("E-SHD-002"),
        "EC-005: the corrupt-index error message must carry E-SHD-002. Got: {err}"
    );
}

// ---------------------------------------------------------------------------
// AC-010 — `retention_overflow_count`
// (BC-1.18.007 Postcondition 1, EC-001, EC-002, Invariant 2)
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_007_EC001_retention_overflow_count_zero_when_active_count_at_limit_exactly() {
    let shards: Vec<_> = (1..=10)
        .map(|seq| active_entry(seq, "decision-log", 1_000))
        .collect();
    let index = sample_index("decision-log", 10, shards);

    assert_eq!(
        retention_overflow_count(&index),
        0,
        "EC-001: exactly retention_count active shards must overflow by 0 — the boundary is \
         inclusive, no archival needed"
    );
}

#[test]
fn test_BC_1_18_007_AC010_retention_overflow_count_zero_when_active_count_below_limit() {
    let shards: Vec<_> = (1..=5)
        .map(|seq| active_entry(seq, "decision-log", 1_000))
        .collect();
    let index = sample_index("decision-log", 10, shards);

    assert_eq!(
        retention_overflow_count(&index),
        0,
        "AC-010: fewer active shards than retention_count must never trigger archival"
    );
}

#[test]
fn test_BC_1_18_007_AC010_retention_overflow_count_positive_when_active_exceeds_limit() {
    let shards: Vec<_> = (1..=11)
        .map(|seq| active_entry(seq, "decision-log", 1_000))
        .collect();
    let index = sample_index("decision-log", 10, shards);

    assert_eq!(
        retention_overflow_count(&index),
        1,
        "AC-010: 11 active shards against a retention_count of 10 must overflow by exactly 1"
    );
}

#[test]
fn test_BC_1_18_007_EC002_retention_overflow_count_multiple_when_retention_lowered_mid_flight() {
    // EC-002: retention_count manually lowered (10 -> 5) while 8 shards are
    // already active — the NEXT seal's retention check must archive enough
    // of the oldest to bring the active count back within the NEW limit,
    // potentially archiving more than one shard in a single invocation.
    let shards: Vec<_> = (1..=8)
        .map(|seq| active_entry(seq, "decision-log", 1_000))
        .collect();
    let index = sample_index("decision-log", 5, shards);

    assert_eq!(
        retention_overflow_count(&index),
        3,
        "EC-002: 8 active shards against a newly-lowered retention_count of 5 must overflow by \
         exactly 3 (bringing the active count back to 5) in a single invocation"
    );
}

#[test]
fn test_BC_1_18_007_INV2_retention_overflow_count_ignores_already_archived_entries() {
    // Invariant 3 / this BC's own path-mutation convention: an already-
    // archived entry stays enumerable in the index forever, but it must NOT
    // count toward the ACTIVE total a fresh retention check evaluates
    // against — otherwise repeated archival passes would double-count
    // history already relocated out of the active window.
    let mut shards: Vec<ShardIndexEntry> = (1..=3)
        .map(|seq| archived_entry(seq, "decision-log", 1_000))
        .collect();
    shards.extend((4..=12).map(|seq| active_entry(seq, "decision-log", 1_000)));
    // 9 ACTIVE entries (seq 4..=12) + 3 already-archived (seq 1..=3).
    let index = sample_index("decision-log", 10, shards);

    assert_eq!(
        retention_overflow_count(&index),
        0,
        "Invariant 3: retention_overflow_count must count ONLY active (non-archived) entries — \
         9 active against retention_count=10 must overflow by 0, regardless of the 3 \
         already-archived siblings also present in the index"
    );
}

// ---------------------------------------------------------------------------
// AC-010 — `archive_overflow_shards`
// (BC-1.18.007 Postcondition 2, Invariant 1, Invariant 2, EC-005)
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_007_AC010_PC2_archive_overflow_shards_moves_oldest_active_shard_to_archive_dir() {
    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    std::fs::write(&canonical_path, "current shard content").unwrap();

    // 11 active sealed shards on disk at the cycle root, retention_count=10
    // -> exactly 1 overflow (the oldest, seq=1).
    let mut shards = Vec::new();
    for seq in 1..=11u32 {
        let filename = format!("decision-log.{seq:04}.md");
        let content = format!("sealed-content-for-seq-{seq}");
        std::fs::write(dir.path().join(&filename), &content).unwrap();
        shards.push(active_entry(seq, "decision-log", content.len() as u64));
    }
    let mut index = sample_index("decision-log", 10, shards);

    let archived = archive_overflow_shards(&mut index, &canonical_path)
        .expect("PC2: a well-formed overflow archival move must succeed");

    assert_eq!(
        archived.len(),
        1,
        "PC2: exactly retention_overflow_count (1) shard must be archived in this invocation"
    );
    assert_eq!(
        archived[0].seq, 1,
        "PC2: the OLDEST active shard (lowest seq) must be archived first — oldest-first order"
    );

    // Invariant 1: move, never delete — the old sibling location no longer
    // holds the file, but the content survives at the new archive location,
    // byte-for-byte.
    let old_path = dir.path().join("decision-log.0001.md");
    assert!(
        !old_path.exists(),
        "PC2: the archived shard must no longer exist at its old cycle-root sibling location"
    );
    let new_path = dir
        .path()
        .join("archive")
        .join("decision-log")
        .join("decision-log.0001.md");
    let moved_content = std::fs::read_to_string(&new_path).expect(
        "Invariant 1: the archived shard must exist, byte-for-byte, at \
         archive/<artifact-stem>/<sealed-filename>",
    );
    assert_eq!(moved_content, "sealed-content-for-seq-1");

    // Invariant 3: the moved entry's OWN index record is rewritten in place
    // (within the SAME `index` value the caller subsequently persists) to
    // reflect the new archived location — never removed from the index.
    let rewritten = index
        .shards
        .iter()
        .find(|e| e.seq == 1)
        .expect("Invariant 3: the archived entry must remain enumerable in the shard-index");
    assert_eq!(
        rewritten.path, "archive/decision-log/decision-log.0001.md",
        "Invariant 3: the archived entry's path must be rewritten to the archive-relative form"
    );

    // The remaining 10 active shards (seq 2..=11) must be untouched.
    for seq in 2..=11u32 {
        assert!(
            dir.path()
                .join(format!("decision-log.{seq:04}.md"))
                .exists(),
            "PC2: shard seq={seq} is within the retention window and must remain at the cycle \
             root, untouched"
        );
    }
}

// F4 BC-cluster-3 adversarial-review finding MED-4 (EC-014, BC-1.18.007
// Postcondition 2/EC-002 — "potentially archiving more than one shard in a
// single invocation"): the existing PC2 test above covers only a
// SINGLE-shard overflow (11 active vs retention_count=10 -> exactly 1
// archived). This test exercises the genuinely MULTI-shard real-filesystem
// MOVE the same edge case names explicitly — retention_count lowered
// mid-flight while several shards are already active, in one invocation.
#[test]
fn test_BC_1_18_007_MED4_EC014_archive_overflow_shards_moves_multiple_oldest_active_shards_in_one_invocation()
 {
    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    std::fs::write(&canonical_path, "current shard content").unwrap();

    // 5 active sealed shards on disk at the cycle root, retention_count
    // lowered to 2 -> exactly 3 overflow (seq 1, 2, 3 — the three oldest).
    let mut shards = Vec::new();
    for seq in 1..=5u32 {
        let filename = format!("decision-log.{seq:04}.md");
        let content = format!("sealed-content-for-seq-{seq}");
        std::fs::write(dir.path().join(&filename), &content).unwrap();
        shards.push(active_entry(seq, "decision-log", content.len() as u64));
    }
    let mut index = sample_index("decision-log", 2, shards);

    let archived = archive_overflow_shards(&mut index, &canonical_path).expect(
        "MED-4/EC-014: a well-formed multi-shard overflow archival move must succeed in one \
         invocation",
    );

    assert_eq!(
        archived.len(),
        3,
        "MED-4/EC-014: exactly retention_overflow_count (3) shards must be archived in this \
         SINGLE invocation, not just the single oldest one. Got: {archived:?}"
    );
    let mut archived_seqs: Vec<u32> = archived.iter().map(|e| e.seq).collect();
    archived_seqs.sort_unstable();
    assert_eq!(
        archived_seqs,
        vec![1, 2, 3],
        "MED-4/EC-014: the THREE oldest active shards (lowest seq) must be archived — oldest-\
         first order across the whole overflow, not just the single oldest"
    );

    // Invariant 1: move (never delete) — EVERY one of the 3 overflow shards
    // must have been physically relocated off the cycle root, each with its
    // content preserved byte-for-byte at its new archive location.
    for seq in 1..=3u32 {
        let old_path = dir.path().join(format!("decision-log.{seq:04}.md"));
        assert!(
            !old_path.exists(),
            "MED-4/EC-014: archived shard seq={seq} must no longer exist at its old cycle-root \
             sibling location"
        );
        let new_path = dir
            .path()
            .join("archive")
            .join("decision-log")
            .join(format!("decision-log.{seq:04}.md"));
        let moved_content = std::fs::read_to_string(&new_path).unwrap_or_else(|e| {
            panic!(
                "MED-4/EC-014 Invariant 1: archived shard seq={seq} must exist, byte-for-byte, \
                 at archive/<artifact-stem>/<sealed-filename>: {e}"
            )
        });
        assert_eq!(moved_content, format!("sealed-content-for-seq-{seq}"));

        // Invariant 3: EACH moved entry's own index record is rewritten in
        // place (within the SAME `index` value) — never just the first one.
        let rewritten = index
            .shards
            .iter()
            .find(|e| e.seq == seq)
            .unwrap_or_else(|| {
                panic!("MED-4/EC-014 Invariant 3: archived seq={seq} must remain enumerable")
            });
        assert_eq!(
            rewritten.path,
            format!("archive/decision-log/decision-log.{seq:04}.md"),
            "MED-4/EC-014 Invariant 3: archived seq={seq}'s path must be rewritten to the \
             archive-relative form"
        );
    }

    // The 2 shards within the new retention window (seq 4, 5) must remain
    // untouched at the cycle root — NOTHING beyond the 3 overflow shards is
    // deleted, truncated, or moved.
    for seq in 4..=5u32 {
        let path = dir.path().join(format!("decision-log.{seq:04}.md"));
        assert!(
            path.exists(),
            "MED-4/EC-014: shard seq={seq} is within the (new, lowered) retention window and \
             must remain at the cycle root, untouched"
        );
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            content,
            format!("sealed-content-for-seq-{seq}"),
            "MED-4/EC-014 Invariant 1: the retained shard's own content must be completely \
             unmodified (move-only semantics never touch retained shards)"
        );
    }
}

#[test]
fn test_BC_1_18_007_EC005_archive_overflow_shards_missing_source_file_fails_loud() {
    let dir = tempfile::tempdir().unwrap();
    let canonical_path = dir.path().join("decision-log.md");
    std::fs::write(&canonical_path, "current shard content").unwrap();

    // The index records an overflow shard, but the actual sealed file is
    // NOT present on disk (a genuine anomaly this BC's contract still must
    // fail loud against, never silently skip).
    let overflow_entry = active_entry(1, "decision-log", 1_000);
    let mut index = sample_index("decision-log", 0, vec![overflow_entry.clone()]);

    let result = archive_overflow_shards(&mut index, &canonical_path);

    let err = result.expect_err(
        "EC-005/Postcondition 2: an archival move against a missing source file must fail loud, \
         never silently skip the archival",
    );
    assert!(
        matches!(err, ShardRetentionError::ArchivalMoveFailed { .. }),
        "the failure must be ArchivalMoveFailed, got {err:?}"
    );
    assert!(
        err.to_string().contains("E-SHD-002"),
        "the archival-move failure must carry the E-SHD-002 error-taxonomy code. Got: {err}"
    );
    // "No shard-index change is applied for a failed move": the entry must
    // be left exactly as it was before this call was attempted.
    assert_eq!(
        index.shards[0], overflow_entry,
        "a failed archival move must leave the in-memory index's entry completely unchanged"
    );
}

// ---------------------------------------------------------------------------
// AC-011/AC-012 — `whole_corpus_shard_paths`
// (BC-1.18.007 Postcondition 3, Postcondition 4, Postcondition 6, EC-003,
// EC-006, VP-122, VP-141)
// ---------------------------------------------------------------------------

/// Seeds a cycle-root fixture: a current file, 2 active sealed shards, and 1
/// archived sealed shard under `archive/<stem>/`.
fn seed_whole_corpus_fixture(cycle_root: &std::path::Path, stem: &str) {
    std::fs::write(cycle_root.join(format!("{stem}.md")), "current").unwrap();
    std::fs::write(cycle_root.join(format!("{stem}.0002.md")), "active-2").unwrap();
    std::fs::write(cycle_root.join(format!("{stem}.0003.md")), "active-3").unwrap();
    let archive_dir = cycle_root.join("archive").join(stem);
    std::fs::create_dir_all(&archive_dir).unwrap();
    std::fs::write(archive_dir.join(format!("{stem}.0001.md")), "archived-1").unwrap();
}

#[test]
fn test_BC_1_18_007_AC011_PC3_whole_corpus_shard_paths_default_excluded_returns_current_and_active_only()
 {
    let dir = tempfile::tempdir().unwrap();
    seed_whole_corpus_fixture(dir.path(), "decision-log");

    let paths = whole_corpus_shard_paths(
        dir.path(),
        "decision-log",
        WholeCorpusGlobScope::DefaultExcluded,
    )
    .expect("AC-011: the default whole-corpus glob must succeed against a well-formed fixture");

    assert_eq!(
        paths.len(),
        3,
        "AC-011/PC3: the default scope must match the current file plus the 2 ACTIVE sealed \
         shards only — never the archived one. Got: {paths:?}"
    );
    let names: std::collections::BTreeSet<String> = paths
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert!(names.contains("decision-log.md"));
    assert!(names.contains("decision-log.0002.md"));
    assert!(names.contains("decision-log.0003.md"));
    assert!(
        !names.contains("decision-log.0001.md"),
        "AC-011/EC-003: the archived shard (seq=1) must be silently excluded from the default \
         whole-corpus glob scope. Got: {names:?}"
    );
}

#[test]
fn test_BC_1_18_007_AC012_PC6_EC006_whole_corpus_shard_paths_archive_inclusive_returns_archived_too()
 {
    let dir = tempfile::tempdir().unwrap();
    seed_whole_corpus_fixture(dir.path(), "decision-log");

    let paths = whole_corpus_shard_paths(
        dir.path(),
        "decision-log",
        WholeCorpusGlobScope::ArchiveInclusive,
    )
    .expect("AC-012: the archive-inclusive whole-corpus glob must succeed");

    assert_eq!(
        paths.len(),
        4,
        "AC-012/PC6/EC-006: the archive-inclusive scope (POLICY-1's mandatory carve-out) must \
         match the current file, BOTH active shards, AND the archived shard. Got: {paths:?}"
    );
    let names: std::collections::BTreeSet<String> = paths
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    for expected in [
        "decision-log.md",
        "decision-log.0001.md",
        "decision-log.0002.md",
        "decision-log.0003.md",
    ] {
        assert!(
            names.contains(expected),
            "AC-012: archive-inclusive scope must include \"{expected}\". Got: {names:?}"
        );
    }
}

#[test]
fn test_BC_1_18_007_AC011_whole_corpus_shard_paths_default_excluded_with_no_archive_dir_present() {
    // No archive/ subdirectory has ever been created for this artifact
    // (retention_count has never been exceeded) — the default scope must
    // still succeed (never error merely because there is no archive to
    // exclude).
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("burst-log.md"), "current").unwrap();
    std::fs::write(dir.path().join("burst-log.0001.md"), "active-1").unwrap();

    let paths = whole_corpus_shard_paths(
        dir.path(),
        "burst-log",
        WholeCorpusGlobScope::DefaultExcluded,
    )
    .expect(
        "AC-011: an artifact with no archive/ directory at all must not error under the \
                 default scope",
    );

    assert_eq!(
        paths.len(),
        2,
        "AC-011: current file + the single active shard, with no archive present. Got: {paths:?}"
    );
}
