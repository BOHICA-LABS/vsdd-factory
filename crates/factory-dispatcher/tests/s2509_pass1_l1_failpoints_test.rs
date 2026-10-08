// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::type_complexity
)]
//! S-25.09 LOCAL adversary pass-1 (cycle L1) failpoint-gated red tests: F-S2509-L1-007(a)
//! and F-S2509-L1-012. `--features factory-dispatcher/failpoints` only (the whole file is
//! empty without the feature), for the same reason as
//! `s2509_v123_write_error_propagation_failpoints_test.rs`: `run_bc_index_migration`
//! constructs `StdFs` itself, so the only real injection seam is the `migration_fs::*`
//! failpoints.
//!
//! | Finding | Test |
//! |---------|------|
//! | F-007(a) | `..._F007_resumed_txn_fencing_generation_is_carried_by_every_intent_log_record` |
//! | F-012 | `..._F012_non_io_cleanup_failure_still_names_the_original_code_token` |
//!
//! `fail::cfg` is PROCESS-GLOBAL, so every test takes `LOCK`; the failpoints are reset on drop.
#![cfg(feature = "failpoints")]

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use factory_dispatcher::shard_manager::{
    BcIndexMigrationError, BcIndexMigrationOutcome, run_bc_index_migration,
};
use serde_json::{Value, json};

const ORIGINAL_CONTENT: &str = "\
---
document_type: bc-index
version: \"1.0\"
total_bcs: 1
---

## Summary

| Subsystem | BC-S Prefix | Count | Directory |
|-----------|------------|-------|-----------|
| SS-01 Hook Dispatcher Core | BC-1 | 1 | ss-01/ |

## Index by subsystem

### SS-01 — Hook Dispatcher Core (BC-1) — 1 BC

| BC ID | Title | Status | Capability | Stories |
|-------|-------|--------|-----------|---------|
| [BC-1.01.001](ss-01/BC-1.01.001.md) | Registry rejects unknown schema version | draft | CAP-TBD | S-15.01 |
";

const SHARD_CONFIG: &str = "\
[[shard]]
artifact_stem = \"BC-INDEX\"
artifact_path = \".factory/specs/behavioral-contracts/BC-INDEX.md\"
practical_fuel_ceiling = 8000000
worst_case_fuel_per_byte = 106.36
max_single_record_bytes = 16384
safety_margin = 8192
shard_cap_bytes = 100000
shape = \"flat\"
";

const REMOVE: &str = "migration_fs::remove";
const POINTER_SWAP: &str = "migration_fs::pointer_swap";

static LOCK: Mutex<()> = Mutex::new(());

struct Serial {
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl Serial {
    fn take() -> Self {
        let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        reset();
        Serial { _guard: guard }
    }
}

impl Drop for Serial {
    fn drop(&mut self) {
        reset();
    }
}

fn reset() {
    for name in [REMOVE, POINTER_SWAP] {
        fail::remove(name);
    }
}

fn ms(root: &Path) -> PathBuf {
    root.join(".factory/migration-state")
}

fn fresh_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".factory")).unwrap();
    std::fs::write(dir.path().join(".factory/shard-config.toml"), SHARD_CONFIG).unwrap();
    let canonical = dir
        .path()
        .join(".factory/specs/behavioral-contracts/BC-INDEX.md");
    std::fs::create_dir_all(canonical.parent().unwrap()).unwrap();
    std::fs::write(canonical, ORIGINAL_CONTENT).unwrap();
    dir
}

fn txn_path(root: &Path) -> PathBuf {
    let mut found = Vec::new();
    for e in std::fs::read_dir(ms(root)).unwrap().flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with("txn-") && name.ends_with(".json") {
            found.push(e.path());
        }
    }
    assert_eq!(found.len(), 1, "expected exactly one txn record: {found:?}");
    found.pop().unwrap()
}

fn txn_of(root: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(txn_path(root)).unwrap()).unwrap()
}

/// A real STAGING state with a COMPLETE staged generation: drive a fresh run until the
/// pointer swap (the commit point) fails with a non-retryable error, which by design leaves
/// the txn at STAGING and the gate LOCKED.
fn staged_project() -> tempfile::TempDir {
    let dir = fresh_project();
    fail::cfg(POINTER_SWAP, "return(storage_full)").unwrap();
    let r = run_bc_index_migration(dir.path());
    fail::remove(POINTER_SWAP);
    assert!(
        matches!(r, Err(BcIndexMigrationError::Io { .. })),
        "setup: the injected pointer-swap failure must stop the fresh run at STAGING; got {r:?}"
    );
    let txn = txn_of(dir.path());
    assert_eq!(txn["state"], json!("STAGING"), "setup: {txn}");
    assert!(txn["generation_id"].is_string(), "setup: {txn}");
    dir
}

fn gen_id_of(root: &Path) -> String {
    txn_of(root)["generation_id"].as_str().unwrap().to_string()
}

/// The integer that follows the first `fencing_generation` key of `line`, encoding-agnostic
/// (`key=3`, `"key": 3`, `key: 3`).
fn fencing_value(line: &str) -> Option<String> {
    let rest = &line[line.find("fencing_generation")? + "fencing_generation".len()..];
    let digits: String = rest
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect();
    (!digits.is_empty()).then_some(digits)
}

/// F-007(a): `fencing_generation` is plumbed from the txn record into EVERY intent-log
/// record (ADR-052 Decision 7a: the txn's `fencing_generation`; ADR-052 v1.24 §7a/§7b/§7c
/// plumbing, story AC-019: "intent-log records carry the txn's `txn_id` and
/// `fencing_generation`"). Every fixture so far had `fencing_generation: 1`, which is also
/// the value a constant-1 implementation would write, so the plumbing was never asserted.
///
/// Fixture: a real STAGING txn that is RESUMED with `fencing_generation` 3 (the intent log
/// from the first attempt is removed so the resume re-appends its INTENT records; the
/// forward moves then append DONE records). Expected: the run completes and every record in
/// the intent log -- INTENT and DONE alike -- carries fencing_generation 3.
#[test]
fn test_BC_1_18_011_F007_resumed_txn_fencing_generation_is_carried_by_every_intent_log_record() {
    let _serial = Serial::take();
    let dir = staged_project();
    let gid = gen_id_of(dir.path());
    let intent = ms(dir.path()).join(format!("intent-{gid}.log"));
    let _ = std::fs::remove_file(&intent);
    let path = txn_path(dir.path());
    let mut txn = txn_of(dir.path());
    txn["fencing_generation"] = json!(3);
    std::fs::write(&path, serde_json::to_vec_pretty(&txn).unwrap()).unwrap();

    let r = run_bc_index_migration(dir.path());
    assert!(
        matches!(r, Ok(BcIndexMigrationOutcome::Completed { .. })),
        "setup: the resumed STAGING txn must complete; got {r:?}"
    );

    let text = std::fs::read_to_string(&intent).expect("the resumed run must write the intent log");
    let txn_id_lines = text.lines().filter(|l| l.contains("txn_id")).count();
    let fencing: Vec<(&str, Option<String>)> = text
        .lines()
        .filter(|l| l.contains("fencing_generation"))
        .map(|l| (l, fencing_value(l)))
        .collect();
    let types: Vec<&str> = text.lines().filter(|l| l.contains("record_type")).collect();
    assert!(
        types.iter().any(|l| l.contains("INTENT")) && types.iter().any(|l| l.contains("DONE")),
        "the log must hold INTENT and DONE records (setup); record_type lines {types:?}"
    );
    assert!(
        txn_id_lines >= 2 && fencing.len() == txn_id_lines,
        "every record must carry a fencing_generation ({} records, {} fencing lines): {text:?}",
        txn_id_lines,
        fencing.len()
    );
    let wrong: Vec<_> = fencing
        .iter()
        .filter(|(_, v)| v.as_deref() != Some("3"))
        .collect();
    assert!(
        wrong.is_empty(),
        "every intent-log record (INTENT and DONE) must carry the txn's fencing_generation 3; \
         offending lines {wrong:?}"
    );
}

/// F-012: `abort_cleanup_outcome`'s `Err(other) => other` arm drops the original error when
/// the cleanup failure is not an `Io`. ADR-052 v1.24 item 11(d), "Rule for every abort-path
/// cleanup": "the first failure is returned as that write's `Io` (exit 2) with the ORIGINAL
/// failure's code token named in its `detail`, so the root cause is still visible; the original
/// error alone is returned only when both writes succeed."
///
/// Fixture: a STAGING txn whose staged generation is destroyed (so `resume_from_staging`
/// fails with a coded error), then -- between the failure and the ABORTED write, at the
/// `Fs::remove` of the generation directory -- the txn file is overwritten with unparseable
/// JSON, so the ABORTED write fails with `txn_record_malformed` (`existing_migration_id`), a
/// NON-`Io` cleanup failure. CONTROL: the identical run without the corruption returns the
/// original coded error unchanged. The corrupted run must still name that original token.
#[test]
fn test_BC_1_18_011_F012_non_io_cleanup_failure_still_names_the_original_code_token() {
    let destroy_staged_shards = |root: &Path| {
        let gid = gen_id_of(root);
        let shards = ms(root).join(format!("gen-{gid}")).join("shards");
        for e in std::fs::read_dir(&shards).unwrap().flatten() {
            if e.path().is_file() {
                std::fs::remove_file(e.path()).unwrap();
            }
        }
    };

    // CONTROL: no corruption => the original error, unchanged.
    let original_token = {
        let _serial = Serial::take();
        let dir = staged_project();
        destroy_staged_shards(dir.path());
        let r = run_bc_index_migration(dir.path());
        let e = r.expect_err("control: the damaged staged generation must abort the resume");
        let tok = e
            .code_token()
            .expect("control: the original resume failure must carry a taxonomy code token")
            .to_string();
        assert_eq!(
            txn_of(dir.path())["state"],
            json!("ABORTED"),
            "control: the clean cleanup writes the txn ABORTED"
        );
        tok
    };

    // FAILURE RUN: the txn file is corrupted at the generation-dir removal.
    let _serial = Serial::take();
    let dir = staged_project();
    destroy_staged_shards(dir.path());
    let txn = txn_path(dir.path());
    fail::cfg_callback(REMOVE, move || {
        std::fs::write(&txn, b"{ this is not json").unwrap();
    })
    .unwrap();
    let r = run_bc_index_migration(dir.path());
    fail::remove(REMOVE);
    let e = r.expect_err("the failed ABORTED write must surface as an error");
    assert!(
        !matches!(e, BcIndexMigrationError::Io { .. }),
        "fixture check: this scenario must exercise the NON-Io cleanup arm (got an Io: {e})"
    );
    assert_eq!(
        e.process_exit_code(),
        2,
        "a failed cleanup write is exit 2: {e}"
    );
    assert!(
        e.to_string().contains(&original_token),
        "item 11(d): the cleanup failure must still name the ORIGINAL failure's code token \
         `{original_token}` so the root cause stays visible; got: {e}"
    );
}
