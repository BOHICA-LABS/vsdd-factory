// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! BC-1.18.010 (S-25.02 cluster-5 "shard-b2", T-10, AC-017) RED-Gate
//! coverage for the B2 mechanism's END-STATE addressing surface in
//! `shard_manager.rs`.
//!
//! # BC-5.38.001 Red Gate discipline — RED (all functions under test are
//! `todo!()` as of stub-architect's cluster-5 commit `adbc795a`)
//!
//! Every function this file exercises (`parse_bc_id`, `first_level_shard_path`,
//! `resolve_bc_shard_path`, `check_arch_index_parity`,
//! `load_subsystem_prefix_snapshot`, `load_shard_manifest`,
//! `load_sub_shard_manifest`, `detect_migration_read_state`,
//! `open_bc_index_path_during_migration`, and
//! `From<BcIndexAddressingError> for HookResult`) is `todo!()` — every test
//! below panics today. Each test asserts the REAL post-implementation
//! expected outcome (never `#[should_panic]`), mirroring the methodology
//! `bc_1_18_005_shard_cap_trigger_test.rs`'s own header comment establishes:
//! the SAME assertion shape is correct both during Red Gate (where it fails
//! against the stub) and once `shard_manager.rs`'s BC-1.18.010 section is
//! implemented (where it should pass unchanged).
//!
//! VP classification (BC-1.18.010 §Verification Properties): VP-127 is a
//! unit-test property (zero-lookup first-level addressing); VP-128 is
//! twofold — an integration-test property (single-authoritative-row /
//! manifest-keyed second-level addressing) and a second integration-test
//! property (mapping-source-of-truth / three-way ARCH-INDEX parity). Groups
//! below are labeled accordingly.
//!
//! # Ambiguity flagged for implementer/architect (not guessed at)
//!
//! `resolve_bc_shard_path(_bc_id, _prefixes, _shards_dir)`'s own `todo!()`
//! body does not pin exactly how `_shards_dir` composes with the top-level
//! shard-manifest path and a sub-manifest's own `sub_manifest` field to
//! produce the paths passed to `load_shard_manifest`/`load_sub_shard_manifest`.
//! This file's EC-001/EC-002/EC-004 tests below assume the canonical layout
//! BC-1.18.010 Postcondition 3's OWN schema comment names literally
//! (`.factory/specs/behavioral-contracts/shards/BC-INDEX.shard-manifest.toml`)
//! — i.e. `_shards_dir` is the `.factory/specs/behavioral-contracts/` root
//! and manifest/sub-manifest paths are resolved by joining `_shards_dir`
//! with the manifest entry's own `path`/`sub_manifest` field verbatim. This
//! is the single most spec-grounded reading, not an arbitrary choice, but it
//! is not literally pinned by the stub signature — confirm against the real
//! implementation once T-10 lands.

use std::io::Read as _;
use std::path::{Path, PathBuf};

use factory_dispatcher::shard_manager::{
    BcId, BcIndexAddressingError, BcIndexMigrationReadState, SubsystemPrefixEntry,
    SubsystemPrefixSnapshot, check_arch_index_parity, detect_migration_read_state,
    first_level_shard_path, load_shard_manifest, load_sub_shard_manifest,
    load_subsystem_prefix_snapshot, open_bc_index_path_during_migration, parse_bc_id,
    resolve_bc_shard_path,
};
use vsdd_hook_sdk::HookResult;

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

fn bc_id(subsystem_major: u32, capability_minor: u32, sequence: u32) -> BcId {
    BcId {
        subsystem_major,
        capability_minor,
        sequence,
    }
}

fn snapshot_with_sha(entries: &[(u32, &str)], arch_index_sha: &str) -> SubsystemPrefixSnapshot {
    SubsystemPrefixSnapshot {
        schema_version: 1,
        arch_index_sha: arch_index_sha.to_string(),
        prefix: entries
            .iter()
            .map(|(major, ss_id)| SubsystemPrefixEntry {
                bc_prefix_major: *major,
                ss_id: (*ss_id).to_string(),
            })
            .collect(),
    }
}

fn snapshot(entries: &[(u32, &str)]) -> SubsystemPrefixSnapshot {
    snapshot_with_sha(entries, "sha-default-fixture")
}

/// The canonical layout root BC-1.18.010 Postcondition 3's own schema
/// comment names: `.factory/specs/behavioral-contracts/`.
fn canonical_layout_dir(dir: &Path) -> PathBuf {
    dir.join(".factory/specs/behavioral-contracts")
}

const TOP_MANIFEST_TOML: &str = r#"schema_version = 1

[[subsystem_shard]]
ss_id = "SS-01"
bc_prefix = "BC-1"
path = "shards/BC-INDEX-SS-01.md"
sub_sharded = false

[[subsystem_shard]]
ss_id = "SS-05"
bc_prefix = "BC-5"
path = "shards/BC-INDEX-SS-05.md"
sub_sharded = true
sub_manifest = "shards/BC-INDEX-SS-05.manifest.toml"

[[subsystem_shard]]
ss_id = "SS-07"
bc_prefix = "BC-7"
path = "shards/BC-INDEX-SS-07.md"
sub_sharded = false
"#;

const SS05_SUB_MANIFEST_TOML: &str = r#"schema_version = 1
ss_id = "SS-05"

[[sub_shard]]
sub_shard_id = ".a"
path = "shards/BC-INDEX-SS-05.a.md"
range_start = "BC-5.01.001"
range_end = "BC-5.30.999"

[[sub_shard]]
sub_shard_id = ".b"
path = "shards/BC-INDEX-SS-05.b.md"
range_start = "BC-5.31.001"
range_end = "BC-5.60.999"
"#;

/// Writes the top-level shard manifest AND SS-05's sub-manifest under
/// `root/shards/` — deliberately does NOT write an SS-07 sub-manifest,
/// since SS-07 is not sub-sharded (proving a correct implementation never
/// needs to read one for an SS-07 lookup).
fn write_full_manifest_fixture(root: &Path) {
    let shards_dir = root.join("shards");
    std::fs::create_dir_all(&shards_dir).expect("create shards dir");
    std::fs::write(
        shards_dir.join("BC-INDEX.shard-manifest.toml"),
        TOP_MANIFEST_TOML,
    )
    .expect("write top-level manifest");
    std::fs::write(
        shards_dir.join("BC-INDEX-SS-05.manifest.toml"),
        SS05_SUB_MANIFEST_TOML,
    )
    .expect("write SS-05 sub-manifest");
}

// ---------------------------------------------------------------------------
// parse_bc_id (shared BC-1.18.010/BC-1.18.011 grammar; VP-127 unit-scoped)
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_010_PC2_parse_bc_id_accepts_well_formed_identifier() {
    let id = parse_bc_id("BC-5.39.006").expect("a well-formed BC-X.YY.NNN identifier must parse");
    assert_eq!(id, bc_id(5, 39, 6));
}

#[test]
fn test_BC_1_18_010_PC2_parse_bc_id_rejects_malformed_identifiers() {
    for candidate in [
        "BC-5.39",         // missing NNN component
        "5.39.006",        // missing BC- prefix
        "BC-5-39-006",     // wrong separators (no dots)
        "BC-x.39.006",     // non-numeric subsystem_major
        "BC-5.39.abc",     // non-numeric sequence
        "",                // empty
        "BC-5.39.006.007", // too many components
    ] {
        let result = parse_bc_id(candidate);
        assert!(
            matches!(result, Err(BcIndexAddressingError::MalformedBcId { .. })),
            "parse_bc_id({candidate:?}) must reject with MalformedBcId, got {result:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// first_level_shard_path — VP-127 zero-lookup invariant (unit-scoped).
//
// The function's own signature takes NO manifest/Path argument at all —
// only the already-loaded `SubsystemPrefixSnapshot` — so the "zero
// shard-manifest reads" property is enforced structurally at the type
// level for any implementation that satisfies this signature; a real mock
// filesystem read-call counter is therefore unnecessary for THIS function
// specifically (contrast `resolve_bc_shard_path` below, which genuinely
// does I/O). These tests assert the correct output value the zero-lookup
// path must produce.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_010_VP127_EC002_first_level_shard_path_computes_ss01_from_prefix_snapshot_alone() {
    // Canonical Test Vector: Lookup BC-1.18.005 -> shards/BC-INDEX-SS-01.md,
    // computed with zero manifest reads.
    let prefixes = snapshot(&[(1, "SS-01"), (5, "SS-05"), (7, "SS-07")]);
    let path = first_level_shard_path(&bc_id(1, 18, 5), &prefixes)
        .expect("BC-1.* must resolve via the already-loaded prefixes snapshot alone");
    assert_eq!(path, PathBuf::from("shards/BC-INDEX-SS-01.md"));
}

#[test]
fn test_BC_1_18_010_INV1_first_level_shard_path_returns_unmapped_prefix_error_for_unknown_subsystem()
 {
    let prefixes = snapshot(&[(1, "SS-01")]);
    let result = first_level_shard_path(&bc_id(99, 1, 1), &prefixes);
    assert!(
        matches!(
            result,
            Err(BcIndexAddressingError::UnmappedBcPrefix {
                subsystem_major: 99
            })
        ),
        "an unmapped BC-S prefix must fail with UnmappedBcPrefix, never silently resolve or \
         panic, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// resolve_bc_shard_path — Postconditions 2/4, EC-001/EC-002/EC-020
// (VP-128 integration-scoped: genuinely reads the top-level manifest and,
// for a sub-sharded subsystem, the sub-manifest too).
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_010_EC001_EC020_resolve_bc_shard_path_ss05_subsharded_resolves_correct_sub_shard() {
    let dir = tempfile::tempdir().unwrap();
    let root = canonical_layout_dir(dir.path());
    write_full_manifest_fixture(&root);
    let prefixes = snapshot(&[(1, "SS-01"), (5, "SS-05"), (7, "SS-07")]);

    // Canonical Test Vector: lookup BC-5.39.006 (SS-05, sub-sharded) -> top
    // manifest discovers sub_sharded=true -> sub-manifest lookup resolves
    // the correct sub-shard (range BC-5.31.001..BC-5.60.999 covers
    // BC-5.39.006, landing in `.b`, not `.a`).
    let path = resolve_bc_shard_path(&bc_id(5, 39, 6), &prefixes, &root)
        .expect("a sub-sharded lookup within a covered range must resolve");
    assert_eq!(path, PathBuf::from("shards/BC-INDEX-SS-05.b.md"));
}

#[test]
fn test_BC_1_18_010_EC002_resolve_bc_shard_path_ss01_not_subsharded_delegates_to_first_level() {
    let dir = tempfile::tempdir().unwrap();
    let root = canonical_layout_dir(dir.path());
    write_full_manifest_fixture(&root);
    let prefixes = snapshot(&[(1, "SS-01"), (5, "SS-05"), (7, "SS-07")]);

    // EC-002: SS-01 is not sub-sharded — zero manifest reads via the
    // first-level path alone, unlike SS-05's two-read case above.
    let path = resolve_bc_shard_path(&bc_id(1, 18, 5), &prefixes, &root)
        .expect("a non-sub-sharded subsystem must resolve via the first-level path alone");
    assert_eq!(path, PathBuf::from("shards/BC-INDEX-SS-01.md"));
}

#[test]
fn test_BC_1_18_010_PC4_resolve_bc_shard_path_returns_sub_shard_range_not_found_when_uncovered() {
    let dir = tempfile::tempdir().unwrap();
    let root = canonical_layout_dir(dir.path());
    write_full_manifest_fixture(&root);
    let prefixes = snapshot(&[(1, "SS-01"), (5, "SS-05"), (7, "SS-07")]);

    // BC-5.99.001 falls outside BOTH the `.a` (..30.999) and `.b` (..60.999)
    // ranges the sub-manifest declares.
    let result = resolve_bc_shard_path(&bc_id(5, 99, 1), &prefixes, &root);
    assert!(
        matches!(
            result,
            Err(BcIndexAddressingError::SubShardRangeNotFound { .. })
        ),
        "a BC ID with no covering sub-shard range must fail with SubShardRangeNotFound, got \
         {result:?}"
    );
}

#[test]
fn test_BC_1_18_010_EC004_resolve_bc_shard_path_ss07_whole_corpus_style_lookup_needs_no_sub_manifest()
 {
    let dir = tempfile::tempdir().unwrap();
    let root = canonical_layout_dir(dir.path());
    // Deliberately write ONLY the top-level manifest fixture — no SS-07
    // sub-manifest file exists anywhere on disk. If a correct
    // implementation mistakenly tried to read a sub-manifest for a
    // non-sub-sharded subsystem, this would surface as an I/O error rather
    // than a resolved path.
    write_full_manifest_fixture(&root);
    let prefixes = snapshot(&[(1, "SS-01"), (5, "SS-05"), (7, "SS-07")]);

    let path = resolve_bc_shard_path(&bc_id(7, 1, 1), &prefixes, &root).expect(
        "SS-07 (not sub-sharded) must resolve via the top-level manifest alone, with no \
         sub-manifest file present on disk for SS-07 at all",
    );
    assert_eq!(path, PathBuf::from("shards/BC-INDEX-SS-07.md"));
}

// ---------------------------------------------------------------------------
// check_arch_index_parity — Invariant 2's three-way ARCH-INDEX parity
// check, fail-CLOSED semantics (VP-128 integration-scoped).
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_010_INV2_check_arch_index_parity_passes_when_all_three_shas_match() {
    let config = snapshot_with_sha(&[(1, "SS-01")], "sha-live-1234");
    let result = check_arch_index_parity(&config, "sha-live-1234", "sha-live-1234");
    assert!(
        result.is_ok(),
        "identical config/manifest/live SHAs must pass the three-way parity check: {result:?}"
    );
}

#[test]
fn test_BC_1_18_010_INV2_check_arch_index_parity_fails_closed_when_manifest_sha_diverges() {
    let config = snapshot_with_sha(&[(1, "SS-01")], "sha-config-A");
    let result = check_arch_index_parity(&config, "sha-manifest-B", "sha-config-A");
    assert!(
        matches!(
            result,
            Err(BcIndexAddressingError::ArchIndexParityMismatch { .. })
        ),
        "a config/manifest SHA divergence must fail CLOSED (ARCH_INDEX_PARITY_ABORT), got \
         {result:?}"
    );
}

#[test]
fn test_BC_1_18_010_INV2_check_arch_index_parity_fails_closed_for_stale_binary_against_current_manifest()
 {
    // config at revision A, manifest approved at revision A (agrees with
    // config), but the LIVE ARCH-INDEX has since moved to revision B — the
    // stale-binary case Invariant 2 explicitly names: a pairwise match
    // between config and manifest is NOT sufficient when the third value
    // (live) diverges.
    let config = snapshot_with_sha(&[(1, "SS-01")], "sha-revision-A");
    let result = check_arch_index_parity(&config, "sha-revision-A", "sha-revision-B");
    assert!(
        matches!(
            result,
            Err(BcIndexAddressingError::ArchIndexParityMismatch { .. })
        ),
        "a stale binary (config at revision A) invoked against a CURRENT live ARCH-INDEX \
         (revision B) must fail CLOSED even though config.arch_index_sha == \
         manifest.approved_arch_index_sha, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// load_subsystem_prefix_snapshot / load_shard_manifest / load_sub_shard_manifest
// — Postcondition 3/4 TOML parse paths.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_010_INV2_load_subsystem_prefix_snapshot_parses_valid_toml() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("subsystem-prefixes.toml");
    std::fs::write(
        &path,
        "schema_version = 1\narch_index_sha = \"abc123\"\n\n[[prefix]]\nbc_prefix_major = 1\n\
         ss_id = \"SS-01\"\n\n[[prefix]]\nbc_prefix_major = 5\nss_id = \"SS-05\"\n",
    )
    .unwrap();
    let loaded =
        load_subsystem_prefix_snapshot(&path).expect("a valid config snapshot TOML must parse");
    assert_eq!(loaded.arch_index_sha, "abc123");
    assert_eq!(loaded.prefix.len(), 2);
    assert_eq!(loaded.prefix[1].ss_id, "SS-05");
}

#[test]
fn test_BC_1_18_010_PC3_load_shard_manifest_parses_valid_toml() {
    let dir = tempfile::tempdir().unwrap();
    let shards_dir = dir.path().join("shards");
    std::fs::create_dir_all(&shards_dir).unwrap();
    std::fs::write(
        shards_dir.join("BC-INDEX.shard-manifest.toml"),
        TOP_MANIFEST_TOML,
    )
    .unwrap();
    let loaded = load_shard_manifest(&shards_dir.join("BC-INDEX.shard-manifest.toml"))
        .expect("a valid top-level manifest TOML must parse");
    assert_eq!(loaded.subsystem_shard.len(), 3);
    assert!(
        loaded
            .subsystem_shard
            .iter()
            .any(|e| e.ss_id == "SS-05" && e.sub_sharded)
    );
}

#[test]
fn test_BC_1_18_010_PC4_load_sub_shard_manifest_parses_valid_toml() {
    let dir = tempfile::tempdir().unwrap();
    let shards_dir = dir.path().join("shards");
    std::fs::create_dir_all(&shards_dir).unwrap();
    std::fs::write(
        shards_dir.join("BC-INDEX-SS-05.manifest.toml"),
        SS05_SUB_MANIFEST_TOML,
    )
    .unwrap();
    let loaded = load_sub_shard_manifest(&shards_dir.join("BC-INDEX-SS-05.manifest.toml"))
        .expect("a valid sub-shard manifest TOML must parse");
    assert_eq!(loaded.ss_id, "SS-05");
    assert_eq!(loaded.sub_shard.len(), 2);
    assert_eq!(loaded.sub_shard[1].sub_shard_id, ".b");
}

// ---------------------------------------------------------------------------
// §Reader Integration — OPEN-based-with-ENOENT-fallback migration-window
// read protocol (VP-128 integration-scoped).
// ---------------------------------------------------------------------------

fn write_completed_json(migration_state_dir: &Path) {
    std::fs::create_dir_all(migration_state_dir).unwrap();
    std::fs::write(
        migration_state_dir.join("completed.json"),
        r#"{"generation_id":"gen-abc","txn_id":"txn-abc","completed_at":"2026-09-13T00:00:00Z","canonical_paths_count":11}"#,
    )
    .unwrap();
}

fn write_current_committing_json(migration_state_dir: &Path, generation_id: &str) {
    std::fs::create_dir_all(migration_state_dir).unwrap();
    std::fs::write(
        migration_state_dir.join("CURRENT.json"),
        format!(
            r#"{{"generation_id":"{generation_id}","status":"committing","txn_id":"txn-abc"}}"#
        ),
    )
    .unwrap();
}

#[test]
fn test_BC_1_18_010_READERINT_detect_migration_read_state_completed_when_marker_present() {
    let dir = tempfile::tempdir().unwrap();
    let migration_state_dir = dir.path().join(".factory/migration-state");
    write_completed_json(&migration_state_dir);
    let state = detect_migration_read_state(&migration_state_dir)
        .expect("a well-formed completed.json must parse");
    assert_eq!(state, BcIndexMigrationReadState::Completed);
}

#[test]
fn test_BC_1_18_010_READERINT_detect_migration_read_state_committing_when_current_json_present() {
    let dir = tempfile::tempdir().unwrap();
    let migration_state_dir = dir.path().join(".factory/migration-state");
    write_current_committing_json(&migration_state_dir, "gen-xyz");
    let state = detect_migration_read_state(&migration_state_dir)
        .expect("a well-formed CURRENT.json with status=committing must parse");
    assert_eq!(
        state,
        BcIndexMigrationReadState::Committing {
            generation_id: "gen-xyz".to_string()
        }
    );
}

#[test]
fn test_BC_1_18_010_READERINT_detect_migration_read_state_not_started_when_neither_marker_present()
{
    let dir = tempfile::tempdir().unwrap();
    let migration_state_dir = dir.path().join(".factory/migration-state");
    std::fs::create_dir_all(&migration_state_dir).unwrap();
    let state = detect_migration_read_state(&migration_state_dir)
        .expect("an empty migration-state dir must resolve NotStarted, never an error");
    assert_eq!(state, BcIndexMigrationReadState::NotStarted);
}

#[test]
fn test_BC_1_18_010_READERINT_detect_migration_read_state_completed_takes_precedence_over_current()
{
    // Step 1 of the protocol checks completed.json FIRST; if both markers
    // somehow coexist, Completed must win.
    let dir = tempfile::tempdir().unwrap();
    let migration_state_dir = dir.path().join(".factory/migration-state");
    write_completed_json(&migration_state_dir);
    write_current_committing_json(&migration_state_dir, "gen-xyz");
    let state = detect_migration_read_state(&migration_state_dir).expect("must parse");
    assert_eq!(state, BcIndexMigrationReadState::Completed);
}

#[test]
fn test_BC_1_18_010_READERINT_open_bc_index_path_during_migration_opens_gen_path_when_present() {
    let dir = tempfile::tempdir().unwrap();
    let migration_state_dir = dir.path().join(".factory/migration-state");
    let canonical_root = dir.path().join("canonical");
    std::fs::create_dir_all(migration_state_dir.join("gen-abc")).unwrap();
    std::fs::create_dir_all(&canonical_root).unwrap();
    std::fs::write(
        migration_state_dir.join("gen-abc/BC-INDEX.md"),
        "NEW CONTENT",
    )
    .unwrap();
    std::fs::write(canonical_root.join("BC-INDEX.md"), "STALE CONTENT").unwrap();

    let mut file = open_bc_index_path_during_migration(
        "gen-abc",
        Path::new("BC-INDEX.md"),
        &migration_state_dir,
        &canonical_root,
    )
    .expect("a file present at gen-<id>/ must open successfully");
    let mut buf = String::new();
    file.read_to_string(&mut buf).unwrap();
    assert_eq!(
        buf, "NEW CONTENT",
        "when the gen-path file exists (not yet moved), it must be opened in preference to the \
         canonical path, which holds STALE old content for in-place-overwrite targets"
    );
}

#[test]
fn test_BC_1_18_010_READERINT_open_bc_index_path_during_migration_falls_back_to_canonical_on_enoent()
 {
    let dir = tempfile::tempdir().unwrap();
    let migration_state_dir = dir.path().join(".factory/migration-state");
    let canonical_root = dir.path().join("canonical");
    std::fs::create_dir_all(migration_state_dir.join("gen-abc")).unwrap();
    std::fs::create_dir_all(&canonical_root).unwrap();
    // gen-abc/shards-BC-INDEX-SS-01.md is ABSENT — already renamed to
    // canonical; ENOENT on the gen-path signals the rename already happened.
    std::fs::write(
        canonical_root.join("shards-BC-INDEX-SS-01.md"),
        "ALREADY MOVED",
    )
    .unwrap();

    let mut file = open_bc_index_path_during_migration(
        "gen-abc",
        Path::new("shards-BC-INDEX-SS-01.md"),
        &migration_state_dir,
        &canonical_root,
    )
    .expect("ENOENT on the gen-path must fall back to the canonical path, not error");
    let mut buf = String::new();
    file.read_to_string(&mut buf).unwrap();
    assert_eq!(buf, "ALREADY MOVED");
}

// ---------------------------------------------------------------------------
// From<BcIndexAddressingError> for HookResult — variant mapping.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_010_from_bc_index_addressing_error_malformed_bc_id_maps_to_hookresult_error() {
    let err = BcIndexAddressingError::MalformedBcId {
        candidate: "not-a-bc-id".to_string(),
    };
    let result: HookResult = err.into();
    assert!(
        matches!(result, HookResult::Error { .. }),
        "an addressing-layer defect (malformed BC ID) must surface as HookResult::Error, got \
         {result:?}"
    );
}

#[test]
fn test_BC_1_18_010_from_bc_index_addressing_error_arch_index_parity_mismatch_maps_to_hookresult_error()
 {
    let err = BcIndexAddressingError::ArchIndexParityMismatch {
        config_sha: "a".to_string(),
        manifest_sha: "b".to_string(),
        live_sha: "c".to_string(),
    };
    let result: HookResult = err.into();
    assert!(
        matches!(result, HookResult::Error { .. }),
        "ARCH_INDEX_PARITY_ABORT surfaced through this addressing-layer conversion must be \
         HookResult::Error (Block is reserved for BC-1.18.011's writer-admission gate, a \
         different call site), got {result:?}"
    );
}
