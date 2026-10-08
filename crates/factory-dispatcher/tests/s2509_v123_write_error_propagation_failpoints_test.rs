// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::type_complexity
)]
//! S-25.09 -- ADR-052 v1.23 item 11(d) (third binary-leg extension): txn-record write errors
//! on the COMPLETED and ABORTED paths PROPAGATE; the build must not create the blocked state
//! and report success, and must not open the gate over a live txn.
//!
//! `--features factory-dispatcher/failpoints` only. The whole file is empty without the
//! feature, exactly like `bc_1_18_011_b2_migration_obl1_failpoint_smoke_test.rs` and
//! `bc_1_18_011_b2_migration_crash_injection_test.rs`, and for the same reason: the injection
//! seam is the `Fs` trait's `migration_fs::*` failpoints. `run_bc_index_migration_core`
//! constructs `StdFs` itself (no `Fs` parameter is reachable from a test) and
//! `finish_committing_migration` / `discard_incomplete_staging` / the step-3b `abort_staging`
//! closure are crate-private, so the only real way to fail "the txn-record write" of those
//! functions from a test is to fail `Fs::write_temp` while the REAL `run_bc_index_migration`
//! drives them. `write_completed_record`, `write_admission_gate_state` and `append` use other
//! primitives, so failing `write_temp` fails ONLY txn-record (and staging) writes.
//!
//! # Method: calibrate, then fail the LAST `write_temp`
//!
//! In every scenario the txn-record write under test is the LAST `Fs::write_temp` of the run
//! (what follows is the gate write, which does not go through `Fs`). Each scenario therefore
//! runs twice on identically prepared state:
//!  1. CALIBRATION: a counting callback on `migration_fs::write_temp` learns N, the number of
//!     `write_temp` calls in the run; the (successful) run also supplies CONTROL assertions
//!     (the original error is returned when both writes succeed; txn ABORTED / COMPLETED; gate
//!     OPEN) that hold before and after the code change;
//!  2. FAILURE RUN: `"<N-1>*off->return(storage_full)"` fails exactly the last `write_temp`.
//! This is robust to however many writes precede the one under test.
//!
//! `fail::cfg` is PROCESS-GLOBAL, so every test takes `LOCK`; the failpoints are reset on drop.
//!
//! | Owed item | Scenario |
//! |-----------|----------|
//! | (5) COMPLETED txn write in `finish_committing_migration` | `..._finish_committing_...` |
//! | (6) ABORTED write in the step-3b `abort_staging` closure | `..._abort_staging_closure_...` (fingerprint mismatch; intent-log append failure) |
//! | (6) `discard_incomplete_staging` caller: `resume_from_staging` failure | `..._discard_caller_resume_from_staging_failure_...` |
//! | (6) `discard_incomplete_staging` caller: recompute failure | `..._discard_caller_recompute_failure_...` |
//! | (6) `discard_incomplete_staging` caller: fingerprint mismatch in the resume arm | `..._discard_caller_fingerprint_mismatch_...` |
//! | (6) `discard_incomplete_staging` caller: canonical-source I/O failure in the resume arm | `..._discard_caller_canonical_io_failure_...` |
#![cfg(feature = "failpoints")]

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use factory_dispatcher::shard_manager::migration_fs::StdFs;
use factory_dispatcher::shard_manager::{
    BcIndexMigrationError, BcIndexMigrationOutcome, BcIndexMigrationTxnRecord,
    run_bc_index_migration, write_txn_record,
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

const WRITE_TEMP: &str = "migration_fs::write_temp";
const APPEND: &str = "migration_fs::append_durable";
const POINTER_SWAP: &str = "migration_fs::pointer_swap";

static LOCK: Mutex<()> = Mutex::new(());

/// Serialises the tests (the failpoint registry is process-global) and resets every
/// failpoint this file uses on drop, even on panic.
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
    for name in [WRITE_TEMP, APPEND, POINTER_SWAP] {
        fail::remove(name);
    }
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn ms(root: &Path) -> PathBuf {
    root.join(".factory/migration-state")
}

fn canonical(root: &Path) -> PathBuf {
    root.join(".factory/specs/behavioral-contracts/BC-INDEX.md")
}

fn fresh_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".factory")).unwrap();
    std::fs::write(dir.path().join(".factory/shard-config.toml"), SHARD_CONFIG).unwrap();
    std::fs::create_dir_all(canonical(dir.path()).parent().unwrap()).unwrap();
    std::fs::write(canonical(dir.path()), ORIGINAL_CONTENT).unwrap();
    dir
}

fn write_gate(root: &Path, state: &str) {
    std::fs::create_dir_all(ms(root)).unwrap();
    std::fs::write(ms(root).join("gate-state.json"), format!("\"{state}\"")).unwrap();
}

fn gate_of(root: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(ms(root).join("gate-state.json")).unwrap()).unwrap()
}

/// The one live-or-terminal `txn-*.json` record, parsed raw.
fn txn_of(root: &Path) -> Value {
    let mut found = Vec::new();
    for e in std::fs::read_dir(ms(root)).unwrap().flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with("txn-") && name.ends_with(".json") {
            found.push(e.path());
        }
    }
    assert_eq!(found.len(), 1, "expected exactly one txn record: {found:?}");
    serde_json::from_slice(&std::fs::read(&found[0]).unwrap()).unwrap()
}

/// Seed a known-state COMMITTING record with a string `generation_id`, its gen dir and
/// no pending moves (=> `ForwardRecovery` straight into `finish_committing_migration`),
/// gate LOCKED. The record is built from JSON that carries `schema_version: 1` and written
/// through the production `write_txn_record`, so it is valid for the build both BEFORE the
/// `schema_version` field lands (the field is then ignored and not written) and AFTER (it is
/// modelled and written): the scenario fails for the write-propagation reason, not for a
/// fixture reason.
fn committing_project() -> tempfile::TempDir {
    let dir = fresh_project();
    std::fs::create_dir_all(ms(dir.path()).join("gen-gen-1")).unwrap();
    let rec: BcIndexMigrationTxnRecord = serde_json::from_value(json!({
        "schema_version": 1,
        "txn_id": "txn-act-1",
        "activation_id": "act-1",
        "fencing_generation": 1,
        "state": "COMMITTING",
        "generation_id": "gen-1",
        "source_sha256": null,
        "source_body_row_sha256": null,
        // ADR-052 v1.24 ruling (i): written with generation_id, equal to the generation path.
        "intent_log_path": ".factory/migration-state/intent-gen-1.log",
        "pending_canonical_moves": [],
        "created_at": "2026-10-07T00:00:00Z",
        "updated_at": "2026-10-07T00:00:00Z",
    }))
    .unwrap();
    write_txn_record(&StdFs, &ms(dir.path()), &rec).unwrap();
    write_gate(dir.path(), "LOCKED");
    dir
}

/// A real STAGING state with a COMPLETE staged generation, the intent log and persisted
/// pending moves: drive a fresh run until the pointer swap (the commit point) fails with a
/// non-retryable error, which by design leaves the txn at STAGING and the gate LOCKED.
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
    assert_eq!(gate_of(dir.path()), json!("LOCKED"), "setup");
    dir
}

fn gen_dir_of(root: &Path) -> PathBuf {
    let gid = txn_of(root)["generation_id"].as_str().unwrap().to_string();
    ms(root).join(format!("gen-{gid}"))
}

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

type Run = Result<BcIndexMigrationOutcome, BcIndexMigrationError>;

/// Run `act` on `dir` while counting `write_temp` calls; return the result and the count.
fn counting_run(dir: &Path, act: &dyn Fn(&Path) -> Run) -> (Run, usize) {
    let n = std::sync::Arc::new(AtomicUsize::new(0));
    let n2 = n.clone();
    fail::cfg_callback(WRITE_TEMP, move || {
        n2.fetch_add(1, Ordering::SeqCst);
    })
    .unwrap();
    let r = act(dir);
    fail::remove(WRITE_TEMP);
    (r, n.load(Ordering::SeqCst))
}

/// Run `act` on `dir` with the LAST of `n` `write_temp` calls failing with `StorageFull`.
fn failing_last_write_run(dir: &Path, n: usize, act: &dyn Fn(&Path) -> Run) -> Run {
    assert!(n >= 1, "calibration saw no write_temp call");
    let spec = if n == 1 {
        "return(storage_full)".to_string()
    } else {
        format!("{}*off->return(storage_full)", n - 1)
    };
    fail::cfg(WRITE_TEMP, spec.as_str()).unwrap();
    let r = act(dir);
    fail::remove(WRITE_TEMP);
    r
}

fn plain_run(dir: &Path) -> Run {
    run_bc_index_migration(dir)
}

/// First `UPPER_SNAKE` taxonomy token (`..._ABORT`, `..._FAILURE`, ...) in a rendered error.
fn code_token(text: &str) -> Option<String> {
    text.split(|c: char| !(c.is_ascii_uppercase() || c == '_'))
        .find(|w| {
            w.len() >= 8 && w.contains('_') && (w.ends_with("ABORT") || w.ends_with("FAILURE"))
        })
        .map(str::to_string)
}

/// The failure-run assertions common to every ABORTED-write scenario (ADR-052 v1.23 item
/// 11(d) "Rule for every abort-path cleanup"): the first failure is returned as that write's
/// own `Io` (exit 2) -- NOT the original error, NOT success -- naming the original code token
/// when the original has one; the txn record is NOT ABORTED (its write failed); the gate was
/// NOT written OPEN (`gate=OPEN => no live txn` must stay true).
fn assert_aborted_write_failure(
    label: &str,
    root: &Path,
    r: &Run,
    original_token: Option<&str>,
    failures: &mut Vec<String>,
) {
    match r {
        Err(e @ BcIndexMigrationError::Io { path, .. })
            if path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("txn-")) =>
        {
            if e.process_exit_code() != 2 {
                failures.push(format!("[{label}] the Io must be exit 2"));
            }
            if let Some(tok) = original_token
                && !e.to_string().contains(tok)
            {
                failures.push(format!(
                    "[{label}] the Io must carry the ORIGINAL failure's code token `{tok}` so the \
                     root cause stays visible; got {e}"
                ));
            }
        }
        other => failures.push(format!(
            "[{label}] a failed ABORTED txn write must be returned as that write's own `Io` \
             (naming the txn record), not the original error and not success; got {other:?}"
        )),
    }
    let txn = txn_of(root);
    if txn["state"] == json!("ABORTED") {
        failures.push(format!(
            "[{label}] the txn record write failed, so it cannot be ABORTED on disk: {txn}"
        ));
    }
    if gate_of(root) == json!("OPEN") {
        failures.push(format!(
            "[{label}] the gate must NOT be written OPEN while the txn is still live (txn \
             state {})",
            txn["state"]
        ));
    }
}

/// Calibrate on one prepared project, fail the last `write_temp` on a second identical one.
/// `control` is checked on the calibration project; `check_failure` on the failure project.
fn abort_scenario(
    name: &str,
    prepare: &dyn Fn() -> tempfile::TempDir,
    act: &dyn Fn(&Path) -> Run,
    gate_open_after_clean_abort: bool,
    expect_token: bool,
    mutate_before_act: &dyn Fn(&Path),
) {
    let _serial = Serial::take();
    let mut failures = Vec::new();

    // 1. calibration (also the CONTROL: both writes succeed => the ORIGINAL error returned)
    let cal = prepare();
    mutate_before_act(cal.path());
    let (cal_result, n) = counting_run(cal.path(), act);
    let cal_text = match &cal_result {
        Err(e) => e.to_string(),
        Ok(o) => {
            failures.push(format!(
                "[{name} / control] the run must fail with the ORIGINAL error; got Ok({o:?})"
            ));
            String::new()
        }
    };
    if matches!(&cal_result, Err(BcIndexMigrationError::Io { path, .. })
        if path.file_name().is_some_and(|f| f.to_string_lossy().starts_with("txn-")))
    {
        failures.push(format!(
            "[{name} / control] with both writes succeeding the ORIGINAL error alone must be \
             returned (not a txn-write Io); got {cal_text}"
        ));
    }
    let cal_txn = txn_of(cal.path());
    if cal_txn["state"] != json!("ABORTED") {
        failures.push(format!(
            "[{name} / control] the txn must be ABORTED when the write succeeds: {cal_txn}"
        ));
    }
    if gate_open_after_clean_abort && gate_of(cal.path()) != json!("OPEN") {
        failures.push(format!(
            "[{name} / control] the gate must be OPEN after a clean abort"
        ));
    }
    let token = if expect_token {
        let t = code_token(&cal_text);
        if t.is_none() {
            failures.push(format!(
                "[{name} / control] the original error carries no taxonomy token to be named: \
                 {cal_text}"
            ));
        }
        t
    } else {
        None
    };

    // 2. failure run
    let target = prepare();
    mutate_before_act(target.path());
    let r = failing_last_write_run(target.path(), n, act);
    assert_aborted_write_failure(name, target.path(), &r, token.as_deref(), &mut failures);

    assert!(
        failures.is_empty(),
        "{name}: {} check(s) failed:\n  - {}",
        failures.len(),
        failures.join("\n  - ")
    );
}

fn nothing(_: &Path) {}

// ===========================================================================
// (5) finish_committing_migration -- the COMPLETED txn write
// ===========================================================================

/// ADR-052 v1.23 item 11(d) / BC-1.18.011 new postcondition: `finish_committing_migration`
/// writes `completed.json` (the commit point) and then rewrites the txn COMPLETED. That
/// rewrite is no longer best-effort: a failed write (ENOSPC / EIO / EACCES on the rename) is
/// `Io`, exit 2, the function does NOT return `Completed` (exit 0) while the own txn is still
/// live beside `completed.json`, and the gate is NOT written OPEN (`gate=OPEN => no live txn`).
/// The best-effort `let _ = write_txn_record(..)` is a TD-VSDD-059 paper-fix: it yields a run
/// that reports success and leaves exactly the state the interim short-circuit then blocks
/// forever. Driven through the real `ForwardRecovery` arm (COMMITTING, string `generation_id`,
/// no pending moves); the COMPLETED write is the only `write_temp` of the run.
///
/// CONTROL (green before and after): with the write succeeding the run is `Completed`, the
/// txn COMPLETED, `completed.json` present, the gate OPEN.
#[test]
fn test_BC_1_18_011_item11d_finish_committing_completed_txn_write_failure_is_io_exit_2_gate_not_opened_no_completed_outcome()
 {
    let _serial = Serial::take();
    let mut failures: Vec<String> = Vec::new();

    // calibration + control
    let cal = committing_project();
    let (cal_result, n) = counting_run(cal.path(), &plain_run);
    if !matches!(cal_result, Ok(BcIndexMigrationOutcome::Completed { .. })) {
        failures.push(format!("[control] expected Completed; got {cal_result:?}"));
    }
    if txn_of(cal.path())["state"] != json!("COMPLETED")
        || !ms(cal.path()).join("completed.json").exists()
        || gate_of(cal.path()) != json!("OPEN")
    {
        failures.push("[control] txn COMPLETED + completed.json + gate OPEN expected".into());
    }
    if n != 1 {
        failures.push(format!(
            "[control] the COMPLETED write must be the run's only write_temp; saw {n}"
        ));
    }

    // failure run
    let target = committing_project();
    let r = failing_last_write_run(target.path(), n.max(1), &plain_run);
    match &r {
        Err(e @ BcIndexMigrationError::Io { path, .. })
            if path
                .file_name()
                .is_some_and(|f| f.to_string_lossy().starts_with("txn-")) =>
        {
            if e.process_exit_code() != 2 {
                failures.push("the Io must be exit 2".into());
            }
        }
        other => failures.push(format!(
            "a failed COMPLETED txn write must be returned as that write's own Io (exit 2), \
             never swallowed into a success outcome; got {other:?}"
        )),
    }
    if let Ok(BcIndexMigrationOutcome::Completed { .. }) = &r {
        failures.push("the function returned `Completed` (exit 0) beside a live txn".into());
    }
    let txn = txn_of(target.path());
    if txn["state"] != json!("COMMITTING") {
        failures.push(format!(
            "the txn write failed, so the record must still be COMMITTING on disk: {txn}"
        ));
    }
    if gate_of(target.path()) == json!("OPEN") {
        failures.push(
            "the gate must NOT be written OPEN while the own txn is still live (gate=OPEN => no \
             live txn)"
                .into(),
        );
    }
    assert!(
        failures.is_empty(),
        "{} check(s) failed:\n  - {}",
        failures.len(),
        failures.join("\n  - ")
    );
}

// ===========================================================================
// (6) the step-3b abort_staging closure (fresh run)
// ===========================================================================

/// Mutate the canonical source ONCE from inside an `append` (intent-log) failpoint callback:
/// the append happens AFTER the quiescence snapshot and BEFORE the Postcondition 3a
/// fingerprint recheck, so the recheck sees a changed source.
fn drift_source_during_intent_append(root: &Path) {
    let source = canonical(root);
    let done = std::sync::Arc::new(AtomicBool::new(false));
    fail::cfg_callback(APPEND, move || {
        if !done.swap(true, Ordering::SeqCst) {
            let mut text = std::fs::read_to_string(&source).unwrap();
            text.push_str("\n<!-- drift after quiescence -->\n");
            std::fs::write(&source, text).unwrap();
        }
    })
    .unwrap();
}

/// ADR-052 v1.23 item 11(d) sibling sweep (1): the step-3b `abort_staging` closure ignores the
/// ABORTED txn write (`let _ = write_txn_record(..)`) and then writes the gate OPEN
/// regardless, which can leave a live STAGING txn beside an OPEN gate. Rule: write the txn
/// ABORTED and CHECK it; only on success write the gate OPEN; a failed write is returned as
/// that write's `Io` (exit 2) naming the ORIGINAL failure's code token; the original error
/// alone is returned only when both writes succeed.
///
/// Original failure here: Postcondition 3a `FingerprintMismatchAbort` (the source drifted
/// between the quiescence snapshot and the pre-commit recheck), which carries a taxonomy
/// token.
#[test]
fn test_BC_1_18_011_item11d_abort_staging_closure_aborted_txn_write_failure_fingerprint_mismatch_is_io_exit_2_gate_not_opened()
 {
    let act = |root: &Path| -> Run {
        drift_source_during_intent_append(root);
        let r = run_bc_index_migration(root);
        fail::remove(APPEND);
        r
    };
    abort_scenario(
        "abort_staging closure / FingerprintMismatchAbort",
        &fresh_project,
        &act,
        true,
        true,
        &nothing,
    );
}

/// ADR-052 v1.23 item 11(d) sibling sweep (1), second `abort_staging` caller: the intent-log
/// append fails (`migration_fs::append_durable` returns `WriteZero`) -- the original failure is an
/// `Io` with no taxonomy token -- and the ABORTED txn write then fails too. Same rule: that
/// write's own `Io` (exit 2) is returned, the gate is not written OPEN, the txn stays live.
#[test]
fn test_BC_1_18_011_item11d_abort_staging_closure_aborted_txn_write_failure_intent_append_failure_is_io_exit_2_gate_not_opened()
 {
    let act = |root: &Path| -> Run {
        fail::cfg(APPEND, "return(write_zero)").unwrap();
        let r = run_bc_index_migration(root);
        fail::remove(APPEND);
        r
    };
    abort_scenario(
        "abort_staging closure / intent append failure",
        &fresh_project,
        &act,
        true,
        false,
        &nothing,
    );
}

// ===========================================================================
// (6) the discard_incomplete_staging callers in the ResumeFromStaging arm
// ===========================================================================

/// ADR-052 v1.23 item 11(d) sibling sweep (2), caller 1: `resume_from_staging` fails (the
/// staged shard set no longer matches the txn's `source_body_row_sha256`: a crash-truncated
/// staging pass, EC-002/EC-003) and the arm calls `let _ = discard_incomplete_staging(..)`,
/// returning the ORIGINAL error whether or not the ABORTED write landed. With the ABORTED
/// write failing, the result must be that write's `Io` (exit 2) naming the original code
/// token, the txn must still be live (STAGING), and the gate must not be OPEN.
#[test]
fn test_BC_1_18_011_item11d_discard_caller_resume_from_staging_failure_aborted_txn_write_failure_is_io_exit_2_gate_not_opened()
 {
    let remove_staged_shards = |root: &Path| {
        let shards = gen_dir_of(root).join("shards");
        for e in std::fs::read_dir(&shards).unwrap().flatten() {
            if e.path().is_file() {
                std::fs::remove_file(e.path()).unwrap();
            }
        }
    };
    abort_scenario(
        "discard caller 1 / resume_from_staging failure",
        &staged_project,
        &plain_run,
        false,
        true,
        &remove_staged_shards,
    );
}

/// ADR-052 v1.23 item 11(d) sibling sweep (2), caller 2: `resume_from_staging`'s census
/// passes but `recompute_pending_canonical_moves_from_staged_generation` rejects the
/// generation (its staged lean `BC-INDEX.md` body is missing) and the arm swallows the
/// ABORTED write. Same rule as above.
#[test]
fn test_BC_1_18_011_item11d_discard_caller_recompute_failure_aborted_txn_write_failure_is_io_exit_2_gate_not_opened()
 {
    let remove_staged_body = |root: &Path| {
        std::fs::remove_file(gen_dir_of(root).join("BC-INDEX.md")).unwrap();
    };
    abort_scenario(
        "discard caller 2 / recompute failure",
        &staged_project,
        &plain_run,
        false,
        true,
        &remove_staged_body,
    );
}

/// ADR-052 v1.23 item 11(d) sibling sweep (2), caller 3: the resume arm's pre-swap
/// Postcondition 3a recheck finds the canonical source changed since the quiescence snapshot
/// (`FingerprintMismatchAbort`); the arm swallows the ABORTED write and then writes the gate
/// OPEN unconditionally. With the ABORTED write failing: that write's `Io` (exit 2) naming
/// `FINGERPRINT_MISMATCH_ABORT`, the txn still STAGING, the gate NOT written OPEN.
#[test]
fn test_BC_1_18_011_item11d_discard_caller_fingerprint_mismatch_aborted_txn_write_failure_is_io_exit_2_gate_not_opened()
 {
    let drift_source = |root: &Path| {
        let mut text = std::fs::read_to_string(canonical(root)).unwrap();
        text.push_str("\n<!-- drift after quiescence -->\n");
        std::fs::write(canonical(root), text).unwrap();
    };
    abort_scenario(
        "discard caller 3 / fingerprint mismatch",
        &staged_project,
        &plain_run,
        true,
        true,
        &drift_source,
    );
}

/// ADR-052 v1.23 item 11(d) sibling sweep (2), caller 4: the resume arm's recheck cannot READ
/// the canonical source (`Io` whose path is the canonical `BC-INDEX.md`: SEC-004), routed
/// through `discard_incomplete_staging` and an unconditional gate-OPEN write. With the ABORTED
/// write failing: that write's `Io` (exit 2), the txn still STAGING, the gate NOT written
/// OPEN. (The original is an `Io`, which has no taxonomy token to name.)
#[test]
fn test_BC_1_18_011_item11d_discard_caller_canonical_io_failure_aborted_txn_write_failure_is_io_exit_2_gate_not_opened()
 {
    let remove_source = |root: &Path| {
        std::fs::remove_file(canonical(root)).unwrap();
    };
    abort_scenario(
        "discard caller 4 / canonical-source I/O failure",
        &staged_project,
        &plain_run,
        true,
        false,
        &remove_source,
    );
}
