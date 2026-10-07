// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::collapsible_if
)]
// s21_12_version_gate.rs — RED Gate integration tests for S-21.12.
//
// Asserts that the resolved versions of `wasmtime-wasi` and `crossbeam-epoch`
// in Cargo.lock satisfy the security-patch floor required by S-21.12.
//
// These tests FAIL at the pre-story state (wasmtime-wasi 44.0.3,
// crossbeam-epoch 0.9.18) and PASS only after the story is implemented.
//
// | Test name                                                      | AC    |
// |----------------------------------------------------------------|-------|
// | test_wasmtime_version_satisfies_rustsec_2026_0188_patched_range| AC-008|
// | test_crossbeam_epoch_satisfies_rustsec_2026_0204_patched_range | AC-009|
// | test_wasmtime_lockstep_satisfies_rustsec_2026_0316_set_floor   | ADR-035 v1.2 D6 |
//
// Mechanism (per AC-008 §Confirmed mechanism):
//   Cargo.lock is a TOML file with [[package]] entries. At wasmtime 44.0.3
//   (current), the relevant entry reads:
//     name = "wasmtime-wasi"
//     version = "44.0.3"
//   The test locates this entry by line-scanning (no external crate needed),
//   extracts the version string, and performs a semver comparison.
//
//   The wasmtime crate does NOT export a public version constant;
//   env!("CARGO_PKG_VERSION") written in our code expands to our own crate's
//   version, not wasmtime's.

use std::path::Path;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Scan Cargo.lock (TOML [[package]] format) and return the `version` field
/// for the first entry whose `name` matches `pkg_name`. Returns `None` if the
/// package is not found.
fn parse_cargo_lock_version(lock_content: &str, pkg_name: &str) -> Option<String> {
    let mut current_name: Option<&str> = None;

    for line in lock_content.lines() {
        let line = line.trim();

        if line == "[[package]]" {
            // Start of a new package block; reset the name tracker.
            current_name = None;
            continue;
        }

        if let Some(rest) = line.strip_prefix("name = \"") {
            if let Some(name) = rest.strip_suffix('"') {
                current_name = Some(name);
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("version = \"") {
            if let Some(ver) = rest.strip_suffix('"') {
                if current_name == Some(pkg_name) {
                    return Some(ver.to_string());
                }
            }
        }
    }

    None
}

/// Return true iff `version` (a "major.minor.patch" string) is >= the tuple
/// `(min_major, min_minor, min_patch)`.
fn semver_ge(version: &str, min: (u64, u64, u64)) -> bool {
    let parts: Vec<u64> = version
        .splitn(4, '.')
        .take(3)
        .map(|s| s.parse().unwrap_or(0))
        .collect();
    let v = (
        parts.first().copied().unwrap_or(0),
        parts.get(1).copied().unwrap_or(0),
        parts.get(2).copied().unwrap_or(0),
    );
    v >= min
}

/// Return true iff `version` is strictly below the tuple `(major, minor, patch)`.
fn semver_lt(version: &str, max_exclusive: (u64, u64, u64)) -> bool {
    !semver_ge(version, max_exclusive)
}

/// ADR-035 v1.2 §Decision 6 floor: wasmtime and wasmtime-wasi MUST be in
/// lockstep (identical version), >= 48.0.4 and < 49.0.0. Pure function over
/// Cargo.lock content so the guard itself can be exercised against fixtures.
fn check_wasmtime_lockstep_floor(lock_content: &str) -> Result<String, String> {
    let wasmtime = parse_cargo_lock_version(lock_content, "wasmtime")
        .ok_or_else(|| "wasmtime package entry not found in Cargo.lock".to_string())?;
    let wasi = parse_cargo_lock_version(lock_content, "wasmtime-wasi")
        .ok_or_else(|| "wasmtime-wasi package entry not found in Cargo.lock".to_string())?;
    for (name, v) in [("wasmtime", &wasmtime), ("wasmtime-wasi", &wasi)] {
        if !semver_ge(v, (48, 0, 4)) || !semver_lt(v, (49, 0, 0)) {
            return Err(format!(
                "{name} resolved to {v}, expected >= 48.0.4 and < 49.0.0"
            ));
        }
    }
    if wasmtime != wasi {
        return Err(format!(
            "wasmtime ({wasmtime}) and wasmtime-wasi ({wasi}) are not in lockstep"
        ));
    }
    Ok(wasmtime)
}

/// Resolve the absolute path to the workspace-root Cargo.lock from the
/// factory-dispatcher crate's CARGO_MANIFEST_DIR (two directories up).
fn workspace_cargo_lock() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock")
}

// ---------------------------------------------------------------------------
// AC-008: wasmtime-wasi resolved version >= 46.0.2
// SEC-001 sequencing gate: RUSTSEC-2026-0188 + RUSTSEC-2026-0222 patched range
// ---------------------------------------------------------------------------

/// RED-before state: wasmtime-wasi in Cargo.lock is 44.0.3 — test FAILS.
/// GREEN-after state: wasmtime-wasi in Cargo.lock is >= 46.0.2 — test PASSES.
/// Future-downgrade guard: if wasmtime-wasi is ever inadvertently downgraded
/// below 46.0.2, this test re-fails, blocking SEC-001 dispatch.
#[test]
fn test_wasmtime_version_satisfies_rustsec_2026_0188_patched_range() {
    let lock_path = workspace_cargo_lock();
    assert!(
        lock_path.exists(),
        "Cargo.lock not found at {lock_path:?} — workspace root resolution failed"
    );

    let content = std::fs::read_to_string(&lock_path)
        .unwrap_or_else(|e| panic!("Failed to read {lock_path:?}: {e}"));

    let version = parse_cargo_lock_version(&content, "wasmtime-wasi").unwrap_or_else(|| {
        panic!("wasmtime-wasi package entry not found in Cargo.lock at {lock_path:?}")
    });

    assert!(
        semver_ge(&version, (46, 0, 2)),
        "SEC-001 sequencing gate FAILED: wasmtime-wasi resolved to {version}, \
         expected >= 46.0.2 (RUSTSEC-2026-0188 / CVE-2026-58494 FilePerms bypass \
         and RUSTSEC-2026-0222 type-index confusion are NOT patched on 44.x/45.x; \
         patched range for both starts at wasmtime >= 46.0.2)"
    );
}

// ---------------------------------------------------------------------------
// ADR-035 v1.2 §Decision 6: wasmtime + wasmtime-wasi lockstep, >= 48.0.4, < 49
// ---------------------------------------------------------------------------

/// Version-floor regression guard for RUSTSEC-2026-0316, -0314, -0321, -0322,
/// -0323, -0324 and -0327 (all patched in wasmtime >= 48.0.4), per ADR-035
/// v1.2 §Decision 6. Both `wasmtime` and `wasmtime-wasi` in Cargo.lock MUST be
/// the same version, >= 48.0.4 and < 49.0.0. The floor implies the earlier
/// RUSTSEC-2026-0188/0222 range (>= 46.0.2), which stays asserted separately
/// by `test_wasmtime_version_satisfies_rustsec_2026_0188_patched_range`.
#[test]
fn test_wasmtime_lockstep_satisfies_rustsec_2026_0316_set_floor() {
    let lock_path = workspace_cargo_lock();
    let content = std::fs::read_to_string(&lock_path)
        .unwrap_or_else(|e| panic!("Failed to read {lock_path:?}: {e}"));
    if let Err(msg) = check_wasmtime_lockstep_floor(&content) {
        panic!(
            "ADR-035 v1.2 D6 floor gate FAILED: {msg} \
             (RUSTSEC-2026-0316/0314/0321/0322/0323/0324/0327 are unpatched below 48.0.4)"
        );
    }
}

/// Self-test of the guard: vulnerable or non-lockstep lock fixtures MUST be
/// rejected; a valid 48.x lockstep fixture MUST be accepted.
#[test]
fn test_wasmtime_lockstep_guard_rejects_vulnerable_fixtures() {
    let lock = |wt: &str, wasi: &str| {
        format!(
            "[[package]]\nname = \"wasmtime\"\nversion = \"{wt}\"\n\n\
             [[package]]\nname = \"wasmtime-wasi\"\nversion = \"{wasi}\"\n"
        )
    };
    assert!(check_wasmtime_lockstep_floor(&lock("48.0.3", "48.0.3")).is_err());
    assert!(check_wasmtime_lockstep_floor(&lock("46.0.3", "46.0.3")).is_err());
    assert!(check_wasmtime_lockstep_floor(&lock("49.0.0", "49.0.0")).is_err());
    assert!(check_wasmtime_lockstep_floor(&lock("48.0.5", "48.0.4")).is_err());
    assert!(check_wasmtime_lockstep_floor(&lock("48.0.5", "46.0.3")).is_err());
    assert_eq!(
        check_wasmtime_lockstep_floor(&lock("48.0.4", "48.0.4")),
        Ok("48.0.4".to_string())
    );
    assert_eq!(
        check_wasmtime_lockstep_floor(&lock("48.1.0", "48.1.0")),
        Ok("48.1.0".to_string())
    );
}

// ---------------------------------------------------------------------------
// AC-009: crossbeam-epoch resolved version >= 0.9.20
// RUSTSEC-2026-0204 patched range
// ---------------------------------------------------------------------------

/// RED-before state: crossbeam-epoch in Cargo.lock is 0.9.18 — test FAILS.
/// GREEN-after state: crossbeam-epoch in Cargo.lock is >= 0.9.20 — test PASSES.
#[test]
fn test_crossbeam_epoch_satisfies_rustsec_2026_0204_patched_range() {
    let lock_path = workspace_cargo_lock();
    assert!(
        lock_path.exists(),
        "Cargo.lock not found at {lock_path:?} — workspace root resolution failed"
    );

    let content = std::fs::read_to_string(&lock_path)
        .unwrap_or_else(|e| panic!("Failed to read {lock_path:?}: {e}"));

    let version = parse_cargo_lock_version(&content, "crossbeam-epoch").unwrap_or_else(|| {
        panic!("crossbeam-epoch package entry not found in Cargo.lock at {lock_path:?}")
    });

    assert!(
        semver_ge(&version, (0, 9, 20)),
        "RUSTSEC-2026-0204 gate FAILED: crossbeam-epoch resolved to {version}, \
         expected >= 0.9.20 (crossbeam-epoch pointer dereference advisory; \
         patched in crossbeam-epoch >= 0.9.20)"
    );
}

// ---------------------------------------------------------------------------
// Unit tests for the in-file helpers (sanity; do not test production code)
// ---------------------------------------------------------------------------

mod helper_tests {
    use super::*;

    #[test]
    fn semver_ge_basic() {
        assert!(semver_ge("46.0.2", (46, 0, 2)));
        assert!(semver_ge("46.0.3", (46, 0, 2)));
        assert!(semver_ge("47.0.0", (46, 0, 2)));
        assert!(!semver_ge("44.0.3", (46, 0, 2)));
        assert!(!semver_ge("45.0.3", (46, 0, 2)));
        assert!(!semver_ge("46.0.1", (46, 0, 2)));
    }

    #[test]
    fn semver_ge_crossbeam() {
        assert!(semver_ge("0.9.20", (0, 9, 20)));
        assert!(semver_ge("0.9.21", (0, 9, 20)));
        assert!(semver_ge("1.0.0", (0, 9, 20)));
        assert!(!semver_ge("0.9.18", (0, 9, 20)));
        assert!(!semver_ge("0.9.19", (0, 9, 20)));
    }

    #[test]
    fn parse_cargo_lock_version_finds_entry() {
        let lock_fragment = r#"
[[package]]
name = "some-other-crate"
version = "1.2.3"

[[package]]
name = "wasmtime-wasi"
version = "44.0.3"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "abc123"
"#;
        assert_eq!(
            parse_cargo_lock_version(lock_fragment, "wasmtime-wasi"),
            Some("44.0.3".to_string())
        );
        assert_eq!(
            parse_cargo_lock_version(lock_fragment, "some-other-crate"),
            Some("1.2.3".to_string())
        );
        assert_eq!(parse_cargo_lock_version(lock_fragment, "nonexistent"), None);
    }
}
