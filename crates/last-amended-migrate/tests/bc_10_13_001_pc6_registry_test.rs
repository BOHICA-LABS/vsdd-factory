// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! BC-10.13.001 PC6 (frozen-sidecar non-mutation) + §Architecture Anchors /
//! S-15.03 AC-006 (registering the tool's own output paths and the 5
//! pre-existing D-1149 `*-amendment-history.md` sidecar paths in
//! `plugins/vsdd-factory/config/artifact-path-registry.yaml`).
//!
//! `register_artifact_paths` is read-only with respect to the 5 sidecar
//! FILES themselves (PC6) — it only appends entries to the REGISTRY config.
//! This suite exercises exactly that: the registry file, not any sidecar.

mod common;

use last_amended_migrate::registry::register_artifact_paths;

const REGISTRY_HEADER: &str = "version: 1\nartifacts:\n";

/// S-15.03 AC-006: after registration, every one of the 5 D-1149 sidecar
/// basenames (derived from the crate's own real `TARGET_FILES` constant,
/// not hardcoded here) appears somewhere in the registry file.
#[test]
fn test_BC_10_13_001_PC6_register_artifact_paths_adds_all_five_sidecars() {
    let dir = tempfile::tempdir().expect("tempdir");
    let registry_path =
        common::write_file(dir.path(), "artifact-path-registry.yaml", REGISTRY_HEADER);
    let before = common::read_file(&registry_path);
    for basename in common::expected_sidecar_basenames() {
        assert!(
            !before.contains(&basename),
            "fixture sanity: {basename} must not be pre-registered"
        );
    }

    register_artifact_paths(&registry_path).expect("register_artifact_paths must succeed");

    let after = common::read_file(&registry_path);
    for basename in common::expected_sidecar_basenames() {
        assert!(
            after.contains(&basename),
            "expected the registry to reference the D-1149 sidecar \
             {basename:?} after registration, got: {after:?}"
        );
    }
}

/// PC6: the registry gains entries, but the schema's existing structure
/// (the `version: 1` / `artifacts:` header this fixture starts with) is
/// preserved — this is an APPEND, not a destructive rewrite.
#[test]
fn test_BC_10_13_001_PC6_register_artifact_paths_preserves_existing_header() {
    let dir = tempfile::tempdir().expect("tempdir");
    let existing_entry = "  - artifact_type: behavioral-contract-index\n    canonical_path_pattern: \".factory/specs/behavioral-contracts/BC-INDEX.md\"\n    enforcement_level: block\n";
    let seed = format!("{REGISTRY_HEADER}{existing_entry}");
    let registry_path = common::write_file(dir.path(), "artifact-path-registry.yaml", &seed);

    register_artifact_paths(&registry_path).expect("register_artifact_paths must succeed");

    let after = common::read_file(&registry_path);
    assert!(
        after.contains("behavioral-contract-index"),
        "pre-existing unrelated registry entries must be preserved: {after:?}"
    );
    assert!(after.contains("version: 1"));
}

/// Idempotency: re-running against an already-registered set of paths must
/// not add duplicate entries — running twice produces byte-identical
/// output after the first call.
#[test]
fn test_BC_10_13_001_PC6_register_artifact_paths_is_idempotent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let registry_path =
        common::write_file(dir.path(), "artifact-path-registry.yaml", REGISTRY_HEADER);

    register_artifact_paths(&registry_path).expect("first registration");
    let after_first = common::read_file(&registry_path);

    register_artifact_paths(&registry_path).expect("second registration");
    let after_second = common::read_file(&registry_path);

    assert_eq!(
        after_first, after_second,
        "re-running registration must not add duplicate entries"
    );

    for basename in common::expected_sidecar_basenames() {
        assert_eq!(
            after_second.matches(&basename).count(),
            1,
            "{basename} must appear exactly once, not duplicated across 2 runs"
        );
    }
}
