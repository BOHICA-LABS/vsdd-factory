//! S-25.08 F-006 (ADR-052 v1.21 "EC-031 discharge"): in-crate tests for the
//! crate-private `run_bc_index_migration_with_ttl` seam. They live INSIDE the
//! crate (a `#[cfg(test)]` module of `shard_manager`) precisely so the seam stays
//! `pub(crate)`: an operator-injectable TTL (public API / env / argv) would defeat
//! the production floor. BC-1.18.013 EC-031; BC-1.18.011 EC-009(d).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use super::{BcIndexMigrationError, drain_bc_index_writers, run_bc_index_migration_with_ttl};

fn snapshot(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            let rel = p.strip_prefix(base).unwrap().to_string_lossy().to_string();
            if p.is_dir() {
                out.insert(format!("{rel}/"), Vec::new());
                walk(base, &p, out);
            } else {
                out.insert(rel, std::fs::read(&p).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

/// A project root with `.factory/migration-state/{gate-state.json,txn}` but NO
/// `exclusive.lock`.
fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let ms = dir.path().join(".factory/migration-state");
    std::fs::create_dir_all(&ms).unwrap();
    std::fs::write(ms.join("gate-state.json"), "\"OPEN\"").unwrap();
    dir
}

#[test]
fn test_BC_1_18_013_EC031_below_floor_ttl_rejected_before_any_mutation() {
    for secs in [120u64, 1799] {
        let dir = fixture();
        let before = snapshot(dir.path());
        let r = run_bc_index_migration_with_ttl(dir.path(), Duration::from_secs(secs));
        match r {
            Err(BcIndexMigrationError::ReservationTtlBelowFloor {
                configured_secs,
                floor_secs,
            }) => {
                assert_eq!(configured_secs, secs);
                assert_eq!(floor_secs, 1800);
            }
            other => panic!("TTL {secs}s must be ReservationTtlBelowFloor, got {other:?}"),
        }
        assert_eq!(
            snapshot(dir.path()),
            before,
            "TTL {secs}s: the tree must be byte-identical (no gate write, no txn, no GC)"
        );
        assert!(
            !dir.path()
                .join(".factory/migration-state/exclusive.lock")
                .exists(),
            "TTL {secs}s: no exclusive.lock may be created"
        );
        assert!(
            !dir.path()
                .join(".factory/migration-state/reservations")
                .exists(),
            "TTL {secs}s: no reservations/ directory may be created (validation is the FIRST statement)"
        );
    }
}

#[test]
fn test_BC_1_18_013_EC031_at_or_above_floor_ttl_proceeds() {
    for secs in [1800u64, 3600] {
        let dir = fixture();
        let r = run_bc_index_migration_with_ttl(dir.path(), Duration::from_secs(secs));
        assert!(
            !matches!(
                r,
                Err(BcIndexMigrationError::ReservationTtlBelowFloor { .. })
            ),
            "TTL {secs}s meets the floor and must not be rejected as below-floor, got {r:?}"
        );
    }
}

#[test]
fn test_BC_1_18_013_EC031_drain_test_seam_is_not_bound_by_the_floor() {
    let dir = tempfile::tempdir().unwrap();
    drain_bc_index_writers(
        &dir.path().join("reservations"),
        Duration::from_millis(50),
        Duration::from_secs(1),
    )
    .expect("the injectable drain seam accepts a sub-floor TTL");
}

/// BC-1.18.013 v1.11 EC-035 vector (B), crate-private half (S-25.09 T-14): the
/// in-crate TTL seam is a path-only entry like `run_bc_index_migration`, so a
/// project root without `.factory` yields `FactoryRootNotFound { root_source: None }`
/// with the exact suffix-less Display, and nothing is created or mutated. The
/// production default TTL passes the floor, so the failure is the missing root.
#[test]
fn test_BC_1_18_013_EC035_with_ttl_seam_root_source_none_suffixless_display() {
    use super::DEFAULT_MAX_RESERVATION_TTL;
    for variant_file in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        if variant_file {
            std::fs::write(dir.path().join(".factory"), b"i am a file").unwrap();
        }
        let before = snapshot(dir.path());
        match run_bc_index_migration_with_ttl(dir.path(), DEFAULT_MAX_RESERVATION_TTL) {
            Err(
                e @ BcIndexMigrationError::FactoryRootNotFound {
                    project_root: _,
                    root_source: _,
                },
            ) => {
                let BcIndexMigrationError::FactoryRootNotFound {
                    project_root,
                    root_source,
                } = &e
                else {
                    unreachable!()
                };
                assert_eq!(project_root, dir.path());
                assert_eq!(*root_source, None);
                assert_eq!(
                    e.to_string(),
                    format!(
                        "FACTORY_ROOT_NOT_FOUND: no .factory directory under project root {}",
                        dir.path().display()
                    )
                );
            }
            other => panic!("expected FactoryRootNotFound (file={variant_file}), got {other:?}"),
        }
        assert_eq!(snapshot(dir.path()), before, "tree byte-identical");
    }
}
