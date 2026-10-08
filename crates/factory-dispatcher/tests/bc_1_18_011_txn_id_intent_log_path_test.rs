// Test files use .expect()/.unwrap()/.panic!() for failure reporting,
// matching bc_1_18_011_b2_migration_test.rs's own established convention.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Red tests for two spec-mandated coordinator code defects in `migrate-bc-index`
//! (S-25.09 operator-runbook findings at `c4e9f717`; ADR-052 v1.23 "fourth extension",
//! classified CODE DEFECT / CODE GAP -- the SPEC wins).
//!
//! # Spec anchors
//!
//! * **`txn_id` (ADR-052 Decision 7a; Decision 7c steps 6/8; "Branch C hash source"
//!   ruling (iii); BC-1.18.011 Branch C; error-taxonomy `COMPLETION_RECORD_MISMATCH_ABORT`
//!   row (b)).** Decision 7a: "`txn_id`: UUID (same as `activation_id`)". The step-6
//!   `CURRENT.json` payload and the step-8 `completed.json` payload carry
//!   `"txn_id": "<activation_id>"`. Ruling (iii): an implementation that writes any other
//!   `txn_id` (e.g. a `txn-` prefix) makes every genuine `completed.json` fail the Branch C
//!   `txn_id == activation_id` equality. The txn record's `txn_id`, the intent-log
//!   records' `txn_id`, `CURRENT.json.txn_id` and `completed.json.txn_id` therefore ALL equal
//!   the `activation_id`, with no prefix. (The txn record FILE is still named
//!   `txn-<activation_id>.json`, Decision 7a; that is a file name, not a `txn_id` value.)
//! * **`intent_log_path` (ADR-052 Decision 7c step 1; "Branch C hash source" ruling (i);
//!   v1.23 fourth extension, Code fix (2)).** The txn record is updated with `generation_id`
//!   AND `intent_log_path` in the SAME write; the value is the Decision 7b path
//!   `.factory/migration-state/intent-<generation_id>.log`, i.e. exactly the path the
//!   coordinator appends to; it is JSON `null` ONLY while `generation_id` is null; a
//!   STAGING-resume rewrite leaves the already-correct value unchanged, and a record with
//!   a string `generation_id` but a null (or non-matching) `intent_log_path` is
//!   `txn_record_malformed`, never repaired (ruling (i): "no derived fallback").
//!
//! # Deliberately NOT tested (human decision + research pending)
//!
//! * the intent-log TEXT ENCODING (Decision 7b literal block vs the shipped encoding): the
//!   intent-log `txn_id` assertions below are encoding-agnostic (every line mentioning
//!   `txn_id` must carry the activation id and no `txn-` prefix);
//! * the `pending_canonical_moves` shrink semantics.
//!
//! # Gating
//!
//! The fresh-run tests need no feature. The mid-run observation tests (txn record at a
//! crash point) are `--features factory-dispatcher/failpoints` only and reuse the
//! crash-injection child-process harness shape of
//! `bc_1_18_011_b2_migration_crash_injection_test.rs`.

use std::path::{Path, PathBuf};

use factory_dispatcher::shard_manager::{BcIndexMigrationOutcome, run_bc_index_migration};
use serde_json::Value;

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

const ORIGINAL_CONTENT: &str = "\
---
document_type: bc-index
version: \"1.0\"
total_bcs: 2
---

## Summary

| Subsystem | BC-S Prefix | Count | Directory |
|-----------|------------|-------|-----------|
| SS-01 Hook Dispatcher Core | BC-1 | 1 | ss-01/ |
| SS-02 Shard Manager | BC-2 | 1 | ss-02/ |

## Index by subsystem

### SS-01 — Hook Dispatcher Core (BC-1) — 1 BC

| BC ID | Title | Status | Capability | Stories |
|-------|-------|--------|-----------|---------|
| [BC-1.01.001](ss-01/BC-1.01.001.md) | Registry rejects unknown schema version | draft | CAP-TBD | S-15.01 |

### SS-02 — Shard Manager (BC-2) — 1 BC

| BC ID | Title | Status | Capability | Stories |
|-------|-------|--------|-----------|---------|
| [BC-2.01.001](ss-02/BC-2.01.001.md) | Shard manager splits oversized index | draft | CAP-TBD | S-25.02 |
";

fn migration_state_dir(cwd: &Path) -> PathBuf {
    cwd.join(".factory/migration-state")
}

fn setup_fixture(cwd: &Path) {
    let factory_dir = cwd.join(".factory");
    std::fs::create_dir_all(&factory_dir).unwrap();
    std::fs::write(factory_dir.join("shard-config.toml"), SHARD_CONFIG).unwrap();
    let canonical = cwd.join(".factory/specs/behavioral-contracts/BC-INDEX.md");
    std::fs::create_dir_all(canonical.parent().unwrap()).unwrap();
    std::fs::write(&canonical, ORIGINAL_CONTENT).unwrap();
}

/// In a failpoints build, serialize against the process-global fail registry.
#[cfg(feature = "failpoints")]
fn fail_point_scope() -> fail::FailScenario<'static> {
    fail::FailScenario::setup()
}

fn read_json(path: &Path) -> Value {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("reading {} failed: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{} is not JSON: {e}", path.display()))
}

fn txn_record_files(msd: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(msd)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy().into_owned();
            n.starts_with("txn-") && n.ends_with(".json")
        })
        .collect()
}

/// The single live-or-terminal txn record file `txn-<activation_id>.json` (raw JSON, so the
/// assertions do not depend on the typed decoder's field handling).
fn read_txn(msd: &Path) -> Value {
    let mut found = txn_record_files(msd);
    assert_eq!(
        found.len(),
        1,
        "expected exactly one txn record, got {found:?}"
    );
    read_json(&found.pop().unwrap())
}

fn str_field<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("field `{key}` must be a present JSON string in {v}"))
}

/// Resolve a recorded `intent_log_path` string to an on-disk path (it may be absolute or
/// project-root-relative).
fn resolve_recorded(cwd: &Path, recorded: &str) -> PathBuf {
    let p = PathBuf::from(recorded);
    if p.is_absolute() { p } else { cwd.join(p) }
}

/// Assert the txn record's `intent_log_path` is the persisted string path of
/// `intent-<generation_id>.log` -- the log the coordinator appends to -- and that the
/// `generation_id` is non-null in the SAME record.
fn assert_intent_log_path_persisted_with_generation(cwd: &Path, txn: &Value) {
    let gen_id = txn
        .get("generation_id")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("generation_id must be a non-null string in {txn}"));
    let recorded = txn
        .get("intent_log_path")
        .unwrap_or_else(|| panic!("intent_log_path key must be present in {txn}"))
        .as_str()
        .unwrap_or_else(|| {
            panic!(
                "ADR-052 7c step 1: intent_log_path must be persisted (a string) in the same \
                 write that assigns generation_id={gen_id}; got {:?}",
                txn.get("intent_log_path")
            )
        });
    let expected_name = format!("intent-{gen_id}.log");
    let resolved = resolve_recorded(cwd, recorded);
    assert_eq!(
        resolved.file_name().unwrap().to_string_lossy(),
        expected_name,
        "intent_log_path must name intent-<generation_id>.log, got {recorded:?}"
    );
    assert!(
        recorded.ends_with(&format!(".factory/migration-state/{expected_name}")),
        "intent_log_path must be the Decision 7b path .factory/migration-state/{expected_name}, \
         got {recorded:?}"
    );
    // It must be the very path the coordinator appends to: the same location as
    // `<migration-state>/intent-<generation_id>.log` (the log itself may not exist yet at
    // the instant generation_id is assigned, so compare the canonicalized parent + name).
    let on_disk = migration_state_dir(cwd).join(&expected_name);
    assert_eq!(
        std::fs::canonicalize(resolved.parent().unwrap()).unwrap(),
        std::fs::canonicalize(on_disk.parent().unwrap()).unwrap(),
        "intent_log_path must live in .factory/migration-state/, got {recorded:?}"
    );
}

/// The intent log the coordinator appended to exists at the recorded path (used at points
/// where INTENT records are already durable).
fn assert_recorded_intent_log_exists(cwd: &Path, txn: &Value) {
    let recorded = str_field(txn, "intent_log_path");
    let resolved = resolve_recorded(cwd, recorded);
    assert!(
        resolved.is_file(),
        "recorded intent_log_path {recorded:?} must resolve to the appended intent log"
    );
    let gen_id = str_field(txn, "generation_id");
    assert_eq!(
        std::fs::canonicalize(&resolved).unwrap(),
        std::fs::canonicalize(migration_state_dir(cwd).join(format!("intent-{gen_id}.log")))
            .unwrap()
    );
}

/// Encoding-agnostic intent-log `txn_id` check: the log must mention `txn_id` at least
/// once, every line mentioning it carries the bare activation id, and the prefixed form
/// `txn-<activation_id>` appears nowhere in the log.
fn assert_intent_log_txn_id_is_activation_id(cwd: &Path, gen_id: &str, activation_id: &str) {
    let path = migration_state_dir(cwd).join(format!("intent-{gen_id}.log"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading intent log {} failed: {e}", path.display()));
    let lines: Vec<&str> = text.lines().filter(|l| l.contains("txn_id")).collect();
    assert!(
        !lines.is_empty(),
        "intent log must record a txn_id; log was {text:?}"
    );
    for l in &lines {
        assert!(
            l.contains(activation_id),
            "intent-log txn_id line must carry activation_id {activation_id}: {l:?}"
        );
    }
    assert!(
        !text.contains(&format!("txn-{activation_id}")),
        "no intent-log record may carry the `txn-` prefixed id; log was {text:?}"
    );
}

fn assert_pointer_and_terminal_txn_ids(msd: &Path, activation_id: &str, need_completed: bool) {
    let current = msd.join("CURRENT.json");
    assert!(current.exists(), "CURRENT.json must exist at this point");
    let cur = read_json(&current);
    assert_eq!(
        str_field(&cur, "txn_id"),
        activation_id,
        "CURRENT.json.txn_id must equal the activation_id (ADR-052 7c step 6), no prefix"
    );
    if need_completed {
        let done = read_json(&msd.join("completed.json"));
        assert_eq!(
            str_field(&done, "txn_id"),
            activation_id,
            "completed.json.txn_id must equal the activation_id (ADR-052 7c step 8; Branch C \
             txn_id == activation_id), no prefix"
        );
    }
}

// ===========================================================================
// 1. txn_id == activation_id, through a real fresh migrate-bc-index run
// ===========================================================================

/// ADR-052 7a / 7c steps 6, 8 / Branch C hash-source ruling (iii): on a genuine fresh run
/// the txn record's `txn_id`, the intent-log `txn_id`, `CURRENT.json.txn_id` and
/// `completed.json.txn_id` ALL equal the `activation_id`, no `txn-` prefix.
#[test]
fn test_BC_1_18_011_txn_id_equals_activation_id_across_txn_intent_log_pointer_and_completed() {
    #[cfg(feature = "failpoints")]
    let _fp = fail_point_scope();
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());

    let outcome = run_bc_index_migration(dir.path());
    assert!(
        matches!(outcome, Ok(BcIndexMigrationOutcome::Completed { .. })),
        "fresh run must complete, got {outcome:?}"
    );

    let msd = migration_state_dir(dir.path());
    let txn = read_txn(&msd);
    let activation_id = str_field(&txn, "activation_id").to_string();
    assert!(!activation_id.is_empty());
    assert_eq!(
        str_field(&txn, "txn_id"),
        activation_id,
        "ADR-052 7a: txn record txn_id is the activation_id (UUID), not `txn-<activation_id>`"
    );
    assert!(
        !str_field(&txn, "txn_id").starts_with("txn-"),
        "txn_id must carry no `txn-` prefix"
    );

    assert_pointer_and_terminal_txn_ids(&msd, &activation_id, true);
    let gen_id = str_field(&txn, "generation_id").to_string();
    assert_intent_log_txn_id_is_activation_id(dir.path(), &gen_id, &activation_id);

    // The completed.json txn_id must also equal the txn record's own txn_id (the
    // Branch C verification reads both sides).
    let done = read_json(&msd.join("completed.json"));
    assert_eq!(str_field(&done, "txn_id"), str_field(&txn, "txn_id"));
}

/// Negative guard: the txn record FILE name keeps the `txn-<activation_id>.json` form
/// (Decision 7a); only the `txn_id` VALUE is unprefixed. Pins that the fix does not
/// over-correct by renaming the file.
#[test]
fn test_BC_1_18_011_txn_record_file_name_keeps_txn_prefix_while_txn_id_value_does_not() {
    #[cfg(feature = "failpoints")]
    let _fp = fail_point_scope();
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    run_bc_index_migration(dir.path()).expect("fresh run must complete");

    let msd = migration_state_dir(dir.path());
    let txn = read_txn(&msd);
    let activation_id = str_field(&txn, "activation_id");
    assert!(msd.join(format!("txn-{activation_id}.json")).exists());
    assert_eq!(str_field(&txn, "txn_id"), activation_id);
}

// ===========================================================================
// 2. intent_log_path persisted with generation_id (fresh run, final record)
// ===========================================================================

/// ADR-052 7c step 1 / ruling (i): after a genuine fresh run, the (COMPLETED) txn record
/// carries `generation_id` non-null AND `intent_log_path` = the string path of
/// `.factory/migration-state/intent-<generation_id>.log`.
#[test]
fn test_BC_1_18_011_intent_log_path_persisted_with_generation_id_after_fresh_run() {
    #[cfg(feature = "failpoints")]
    let _fp = fail_point_scope();
    let dir = tempfile::tempdir().unwrap();
    setup_fixture(dir.path());
    run_bc_index_migration(dir.path()).expect("fresh run must complete");

    let txn = read_txn(&migration_state_dir(dir.path()));
    assert_intent_log_path_persisted_with_generation(dir.path(), &txn);
    assert_recorded_intent_log_exists(dir.path(), &txn);
}

/// Every file under `dir` (relative path -> bytes), for byte-identity assertions.
#[cfg(feature = "failpoints")]
fn tree_bytes(dir: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, d: &Path, out: &mut std::collections::BTreeMap<String, Vec<u8>>) {
        for e in std::fs::read_dir(d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(base, &p, out);
            } else {
                out.insert(
                    p.strip_prefix(base).unwrap().display().to_string(),
                    std::fs::read(&p).unwrap(),
                );
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

#[cfg(feature = "failpoints")]
mod mid_run {
    use super::*;
    use factory_dispatcher::shard_manager::{AdmissionStateIntegrityKind, BcIndexMigrationError};
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    const ENV_BOUNDARY: &str = "VSDD_TXNID_CRASH_BOUNDARY";
    const ENV_OCCURRENCE: &str = "VSDD_TXNID_CRASH_OCCURRENCE";
    const ENV_CWD: &str = "VSDD_TXNID_CRASH_CWD";

    /// Child-process entrypoint: a no-op in an ordinary run; becomes the crash-injection
    /// child (abort on the Nth reach of the named failpoint, true no-unwind crash) only
    /// when spawned by [`crash_at`].
    #[test]
    fn test_TXNID_crash_child_entrypoint() {
        let Ok(boundary) = std::env::var(ENV_BOUNDARY) else {
            return;
        };
        let occurrence: usize = std::env::var(ENV_OCCURRENCE).unwrap().parse().unwrap();
        let cwd = PathBuf::from(std::env::var(ENV_CWD).unwrap());
        let counter = AtomicUsize::new(0);
        fail::cfg_callback(boundary.clone(), move || {
            if counter.fetch_add(1, Ordering::SeqCst) + 1 == occurrence {
                std::process::abort();
            }
        })
        .expect("configuring the crash-injection callback must succeed");
        let _ = run_bc_index_migration(&cwd);
        // Reaching here means the boundary/occurrence never fired: not an abort.
        std::process::exit(66);
    }

    /// Spawn the child (stdin closed, 30s timeout) and assert it died by SIGABRT.
    fn crash_at(boundary: &str, occurrence: usize, cwd: &Path) {
        let exe = std::env::current_exe().unwrap();
        let mut cmd = Command::new(exe);
        cmd.arg("--exact")
            .arg("mid_run::test_TXNID_crash_child_entrypoint")
            .arg("--test-threads=1")
            .arg("--nocapture")
            .envs([
                (ENV_BOUNDARY, boundary.to_string()),
                (ENV_OCCURRENCE, occurrence.to_string()),
                (ENV_CWD, cwd.display().to_string()),
                ("RUST_BACKTRACE", "0".to_string()),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn().expect("spawning the crash child must succeed");
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("crash child for {boundary}#{occurrence} timed out");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let out = child.wait_with_output().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            assert_eq!(
                out.status.signal(),
                Some(6),
                "child for {boundary}#{occurrence} must SIGABRT, got {:?} stderr={}",
                out.status,
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }

    fn crashed_fixture(boundary: &str, occurrence: usize) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        setup_fixture(dir.path());
        crash_at(boundary, occurrence, dir.path());
        dir
    }

    fn converge(cwd: &Path) {
        let mut last = None;
        for _ in 0..3 {
            let r = run_bc_index_migration(cwd);
            if matches!(r, Ok(BcIndexMigrationOutcome::Completed { .. })) {
                return;
            }
            last = Some(r);
        }
        panic!("recovery did not converge: {last:?}");
    }

    /// `intent_log_path` is null exactly when `generation_id` is null: observed at the
    /// crash point BEFORE the generation_id write (STAGING, generation_id null).
    #[test]
    fn test_BC_1_18_011_intent_log_path_null_exactly_while_generation_id_null() {
        let _fp = fail_point_scope();
        let dir = crashed_fixture("migration_fs::write_temp", 2);
        let txn = read_txn(&migration_state_dir(dir.path()));
        assert!(
            txn.get("generation_id").unwrap().is_null(),
            "premise: crash landed before the generation_id write; got {txn}"
        );
        assert!(
            txn.get("intent_log_path")
                .expect("key must be present")
                .is_null(),
            "intent_log_path is null while generation_id is null; got {txn}"
        );
    }

    /// The converse, at the very next write: STAGING WITH a generation_id already carries
    /// the non-null `intent_log_path` in the SAME record (never `generation_id` set while
    /// `intent_log_path` is still null).
    #[test]
    fn test_BC_1_18_011_intent_log_path_set_in_same_write_as_generation_id_staging() {
        let _fp = fail_point_scope();
        let dir = crashed_fixture("migration_fs::write_temp", 3);
        let txn = read_txn(&migration_state_dir(dir.path()));
        assert_eq!(str_field(&txn, "state"), "STAGING");
        assert_intent_log_path_persisted_with_generation(dir.path(), &txn);
    }

    /// Observed at STAGING after all intents are durable (pending moves not yet
    /// persisted): still persisted.
    #[test]
    fn test_BC_1_18_011_intent_log_path_present_at_staging_before_commit_point() {
        let _fp = fail_point_scope();
        let dir = crashed_fixture("migration_fs::write_temp", 7);
        let txn = read_txn(&migration_state_dir(dir.path()));
        assert_eq!(str_field(&txn, "state"), "STAGING");
        assert_intent_log_path_persisted_with_generation(dir.path(), &txn);
        assert_recorded_intent_log_exists(dir.path(), &txn);
    }

    /// COMMITTING point: crash after `completed.json` is durable but before the txn is
    /// rewritten COMPLETED, so the on-disk txn is COMMITTING. The record carries
    /// generation_id + intent_log_path, and every txn_id (txn record, intent log,
    /// CURRENT.json, completed.json) equals the activation_id.
    #[test]
    fn test_BC_1_18_011_committing_txn_has_intent_log_path_and_all_txn_ids_equal_activation_id() {
        let _fp = fail_point_scope();
        let dir = crashed_fixture("migration_fs::write_temp", 10);
        let msd = migration_state_dir(dir.path());
        let txn = read_txn(&msd);
        assert_eq!(
            str_field(&txn, "state"),
            "COMMITTING",
            "premise: crash between completed.json and the COMPLETED rewrite; got {txn}"
        );
        assert_intent_log_path_persisted_with_generation(dir.path(), &txn);
        assert_recorded_intent_log_exists(dir.path(), &txn);
        let activation_id = str_field(&txn, "activation_id").to_string();
        assert_eq!(str_field(&txn, "txn_id"), activation_id);
        assert_pointer_and_terminal_txn_ids(&msd, &activation_id, true);
        assert_intent_log_txn_id_is_activation_id(
            dir.path(),
            str_field(&txn, "generation_id"),
            &activation_id,
        );
    }

    /// CURRENT.json landed (occurrence 8 durable) with the txn label still STAGING: the
    /// pointer's txn_id is already the activation_id.
    #[test]
    fn test_BC_1_18_011_current_json_txn_id_equals_activation_id_at_commit_point() {
        let _fp = fail_point_scope();
        let dir = crashed_fixture("migration_fs::write_temp", 9);
        let msd = migration_state_dir(dir.path());
        let txn = read_txn(&msd);
        let activation_id = str_field(&txn, "activation_id").to_string();
        assert_eq!(str_field(&txn, "txn_id"), activation_id);
        assert_pointer_and_terminal_txn_ids(&msd, &activation_id, false);
    }

    /// STAGING-resume idempotence (1/2): a record this build wrote already carries
    /// `intent_log_path`; resuming leaves the SAME value in place (and it still names the
    /// appended log).
    #[test]
    fn test_BC_1_18_011_intent_log_path_unchanged_by_staging_resume() {
        let _fp = fail_point_scope();
        let dir = crashed_fixture("migration_fs::write_temp", 7);
        let msd = migration_state_dir(dir.path());
        let before = read_txn(&msd);
        let recorded_before = str_field(&before, "intent_log_path").to_string();
        converge(dir.path());
        let after = read_txn(&msd);
        assert_eq!(str_field(&after, "intent_log_path"), recorded_before);
        assert_intent_log_path_persisted_with_generation(dir.path(), &after);
        assert_recorded_intent_log_exists(dir.path(), &after);
    }

    /// STAGING-resume rejects a null `intent_log_path` beside a string `generation_id`
    /// (it does NOT repair it). ADR-052 v1.24 "Branch C hash source" ruling (i): "`intent_log_path`
    /// is written by the coordinator, not derived ... It is JSON `null` only while
    /// `generation_id` is null ... A record that is COMMITTING, or STAGING with a string
    /// `generation_id`, and carries a null `intent_log_path` therefore violates the spec'd
    /// write; ... `state_integrity` / `TxnRecordMalformed` ... MUST NOT fall back to deriving
    /// the path from `generation_id` (a derived fallback would hide the corruption forever)."
    /// The coordinator persists the pair in ONE `write_txn_record` (Decision 7c step 1), so
    /// this shape cannot be produced by a crash of this build and no activation of the
    /// pre-fix build ever ran; "set idempotently on the STAGING-resume path" (v1.23 fourth
    /// extension, Code fix (2)) means a resume rewriting the SAME already-correct value
    /// (see `..._unchanged_by_staging_resume`), not filling in a missing one.
    ///
    /// REPLACES the former `..._staging_resume_sets_intent_log_path_when_null`, which asserted
    /// that resume fills the null in (the behavior ruling (i) forbids). Seeded by hand at the
    /// crash point: the run must fail `MIGRATION_STATE_INTEGRITY_FAILURE` /
    /// `txn_record_malformed` and leave every byte under `.factory/` unchanged.
    #[test]
    fn test_BC_1_18_011_staging_resume_rejects_null_intent_log_path_with_generation_id_set() {
        let _fp = fail_point_scope();
        let dir = crashed_fixture("migration_fs::write_temp", 7);
        let msd = migration_state_dir(dir.path());
        let txn_file = txn_record_files(&msd).pop().unwrap();
        let mut v = read_json(&txn_file);
        assert!(v.get("generation_id").unwrap().is_string());
        v["intent_log_path"] = Value::Null;
        std::fs::write(&txn_file, serde_json::to_vec_pretty(&v).unwrap()).unwrap();

        let before = tree_bytes(&dir.path().join(".factory"));
        let result = run_bc_index_migration(dir.path());
        assert!(
            matches!(
                result,
                Err(BcIndexMigrationError::AdmissionStateIntegrity {
                    kind: AdmissionStateIntegrityKind::TxnRecordMalformed,
                    ..
                })
            ),
            "a STAGING record with a string generation_id and a null intent_log_path is \
             txn_record_malformed (ruling (i)), never repaired; got {result:?}"
        );
        assert_eq!(
            tree_bytes(&dir.path().join(".factory")),
            before,
            "nothing may be mutated (the null path must not be filled in)"
        );
    }
}
