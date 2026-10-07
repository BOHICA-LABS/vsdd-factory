// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::type_complexity
)]
//! S-25.08 RED-GATE unit tests for ADR-052 v1.21 (D-2 anchoring rule, F-012
//! admission state-integrity variant, source-level sibling-sweep gate).
//!
//! # Stub surface added by the test-writer (BC-5.38.001)
//! * `resolve_session_project_root(Option<&OsStr>, &Path) -> SessionProjectRoot`
//!   (`shard_manager::admission`, `todo!()`);
//! * `BcIndexMigrationError::{AdmissionStateIntegrity, FactoryRootNotFound}`,
//!   `AdmissionStateIntegrityKind`, `AdmissionFailureCause` (declarative: variants,
//!   Display attributes, `token()` tables). `admission_failure_cause()` now RETURNS
//!   the closed enum but its body still ends in `_ =>` and the admission raise
//!   sites still raise `BinaryIntegrityFailure` (the implementer converts them);
//! * crate-private `run_bc_index_migration_with_ttl` (`todo!()`; tested by the
//!   in-crate module `shard_manager/ttl_seam_tests.rs`).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use factory_dispatcher::shard_manager::{
    AdmissionFailureCause, AdmissionOutcome, AdmissionStateIntegrityKind, BcIndexMigrationError,
    ProtectedPathFamily, admit_protected_write, resolve_session_project_root,
};

// ---------------------------------------------------------------------------
// D-2 -- resolve_session_project_root (BC-1.18.013 EC-034)
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_013_EC034_resolve_session_project_root_table() {
    use factory_dispatcher::shard_manager::{ProjectRootSource, SessionProjectRoot};
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().canonicalize().unwrap();
    let proc_cwd = root.join("proc/sub");
    std::fs::create_dir_all(&proc_cwd).unwrap();
    // (f) an ancestor of process_cwd has a `.factory`; process_cwd itself does not
    std::fs::create_dir_all(root.join("proc/.factory")).unwrap();
    let existing = root.join("existing");
    std::fs::create_dir_all(&existing).unwrap();
    let want = |path: PathBuf, source: ProjectRootSource| SessionProjectRoot { path, source };

    // (a) absolute existing dir => canonicalized path, source ClaudeProjectDir
    assert_eq!(
        resolve_session_project_root(Some(existing.as_os_str()), &proc_cwd),
        want(
            existing.canonicalize().unwrap(),
            ProjectRootSource::ClaudeProjectDir
        ),
        "(a) existing absolute dir => canonical path / ClaudeProjectDir"
    );
    // (b) empty == absent; (c) None => process_cwd exactly, source ProcessCwd
    assert_eq!(
        resolve_session_project_root(Some(OsStr::new("")), &proc_cwd),
        want(proc_cwd.clone(), ProjectRootSource::ProcessCwd),
        "(b)"
    );
    assert_eq!(
        resolve_session_project_root(None, &proc_cwd),
        want(proc_cwd.clone(), ProjectRootSource::ProcessCwd),
        "(c)"
    );
    // (d) absolute but nonexistent => the AS-GIVEN path (source ClaudeProjectDir),
    // NEVER process_cwd
    let ghost = root.join("does/not/exist");
    assert_eq!(
        resolve_session_project_root(Some(ghost.as_os_str()), &proc_cwd),
        want(ghost.clone(), ProjectRootSource::ClaudeProjectDir),
        "(d) nonexistent absolute path => as-given (never cwd) / ClaudeProjectDir"
    );
    // (e) symlink => symlink-resolved
    #[cfg(unix)]
    {
        let link = root.join("link");
        std::os::unix::fs::symlink(&existing, &link).unwrap();
        assert_eq!(
            resolve_session_project_root(Some(link.as_os_str()), &proc_cwd),
            want(
                existing.canonicalize().unwrap(),
                ProjectRootSource::ClaudeProjectDir
            ),
            "(e) symlink => resolved / ClaudeProjectDir"
        );
    }
    // (f) NO ancestor walk: with CLAUDE_PROJECT_DIR absent the answer is process_cwd
    // exactly, even though `proc/.factory` exists one level up.
    assert_eq!(
        resolve_session_project_root(None, &proc_cwd),
        want(proc_cwd, ProjectRootSource::ProcessCwd),
        "(f) no ancestor walk / git rev-parse"
    );
}

// ---------------------------------------------------------------------------
// reconciliation -> branch derivation (BC-1.18.013 v1.10 EC-037 table; ADR-052
// v1.21 item 33(b)) -- the pure builder
// ---------------------------------------------------------------------------

/// `derive_block_branch` is a `todo!()` stub added by the test-writer (the merged
/// `verify_admission` derives the branch inline); the implementer must route the
/// inline derivation through it. Rows are the PO's 9 vectors (token, live txn
/// remains?) -> branch.
#[test]
fn test_BC_1_18_013_EC037_reconciliation_to_branch_derivation_table() {
    use factory_dispatcher::shard_manager::{
        BlockBranch, StaleGateReconciliation as R, derive_block_branch,
    };
    let rows: [(R, bool, BlockBranch, &str); 9] = [
        (
            R::LiveCoordinator,
            true,
            BlockBranch::LiveCoordinator,
            "live_coordinator",
        ),
        (
            R::NothingToReconcile,
            true,
            BlockBranch::LiveTxn,
            "nothing_to_reconcile",
        ),
        (
            R::NothingToReconcile,
            false,
            BlockBranch::GateOnly,
            "nothing_to_reconcile",
        ),
        (
            R::GateReopened,
            false,
            BlockBranch::GateOnly,
            "gate_reopened",
        ),
        (R::GateReopened, true, BlockBranch::LiveTxn, "gate_reopened"),
        (
            R::NullGenerationTxnAborted,
            false,
            BlockBranch::GateOnly,
            "null_generation_txn_aborted",
        ),
        (
            R::NullGenerationTxnAborted,
            true,
            BlockBranch::LiveTxn,
            "null_generation_txn_aborted",
        ),
        (
            R::ForeignMigrationRefused,
            true,
            BlockBranch::ForeignMigration,
            "foreign_migration_refused",
        ),
        (
            R::CompletionRecordMismatch,
            true,
            BlockBranch::CompletionRecordMismatch,
            "completion_record_mismatch",
        ),
    ];
    let mut failures = Vec::new();
    for (recon, live, want_branch, want_token) in rows {
        if recon.token() != want_token {
            failures.push(format!(
                "{recon:?}: wire token {:?} != {want_token}",
                recon.token()
            ));
        }
        let got = std::panic::catch_unwind(|| derive_block_branch(recon, live));
        match got {
            Ok(b) if b == want_branch => {}
            Ok(b) => failures.push(format!(
                "({recon:?}, live={live}) => {b:?}, expected {want_branch:?}"
            )),
            Err(_) => failures.push(format!(
                "({recon:?}, live={live}) => derive_block_branch panicked (unimplemented)"
            )),
        }
    }
    assert!(
        failures.is_empty(),
        "{} failure(s):\n  - {}",
        failures.len(),
        failures.join("\n  - ")
    );
}

// ---------------------------------------------------------------------------
// D-2 -- sibling-sweep source gate (ADR-052 v1.21 "Single anchoring rule" (e))
// ---------------------------------------------------------------------------

/// SOURCE-LEVEL GATE (documented grep-by-`include_str!`): after the D-2 re-anchoring,
/// `shard_manager.rs` must derive every `.factory/...` path from the resolved
/// `FactoryRoot` -- the resolver (`resolve_factory_root`) lives in
/// `shard_manager/admission.rs`. This test fails while any non-comment line of
/// `shard_manager.rs` still contains `join(".factory`, mirroring
/// `grep -n 'join(".factory' crates/factory-dispatcher/src/shard_manager.rs`.
#[test]
fn test_BC_1_18_013_EC033_no_dot_factory_join_literal_outside_the_resolver_source_gate() {
    let src = include_str!("../src/shard_manager.rs");
    let offenders: Vec<(usize, &str)> = src
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim_start().starts_with("//"))
        .filter(|(_, l)| l.contains("join(\".factory"))
        .map(|(i, l)| (i + 1, l.trim()))
        .collect();
    assert!(
        offenders.is_empty(),
        "shard_manager.rs must contain no `join(\".factory` literal outside the resolver \
         (Invariant 8 sibling-sweep gate); {} offending line(s):\n{}",
        offenders.len(),
        offenders
            .iter()
            .map(|(n, l)| format!("  {n}: {l}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

// ---------------------------------------------------------------------------
// F-012 -- admission state-integrity variant
// ---------------------------------------------------------------------------

const ALL_KINDS: [AdmissionStateIntegrityKind; 5] = [
    AdmissionStateIntegrityKind::GateRecordMalformed,
    AdmissionStateIntegrityKind::TxnRecordMalformed,
    AdmissionStateIntegrityKind::TxnMigrationIdNotString,
    AdmissionStateIntegrityKind::MultipleLiveTxns,
    AdmissionStateIntegrityKind::ReservationSerialization,
];

/// Exhaustiveness pin (compile-time): matches EVERY `AdmissionStateIntegrityKind`
/// variant explicitly with no wildcard, so adding a kind breaks this file's build
/// until its token is decided here.
fn expected_kind_token(k: AdmissionStateIntegrityKind) -> &'static str {
    match k {
        AdmissionStateIntegrityKind::GateRecordMalformed => "gate_record_malformed",
        AdmissionStateIntegrityKind::TxnRecordMalformed => "txn_record_malformed",
        AdmissionStateIntegrityKind::TxnMigrationIdNotString => "txn_migration_id_not_string",
        AdmissionStateIntegrityKind::MultipleLiveTxns => "multiple_live_txns",
        AdmissionStateIntegrityKind::ReservationSerialization => "reservation_serialization",
    }
}

/// Exhaustiveness pin (compile-time) for the `E-MAINTENANCE-002` cause classification
/// over EVERY `BcIndexMigrationError` variant, no wildcard: adding a variant breaks
/// this build until it is classified here AND in `admission_failure_cause` (which must
/// likewise be an exhaustive match). Coordinator-only variants classify as
/// `StateIntegrity` (fail-closed).
fn expected_cause(e: &BcIndexMigrationError) -> AdmissionFailureCause {
    match e {
        BcIndexMigrationError::InvalidToolUseId { .. } => AdmissionFailureCause::InvalidToolUseId,
        BcIndexMigrationError::Io { .. } => AdmissionFailureCause::Io,
        BcIndexMigrationError::AdmissionStateIntegrity { .. } => {
            AdmissionFailureCause::StateIntegrity
        }
        BcIndexMigrationError::BinaryIntegrityFailure { .. }
        | BcIndexMigrationError::RecoveryRequiresReauthorization
        | BcIndexMigrationError::ExpiryAbort
        | BcIndexMigrationError::FingerprintMismatchAbort
        | BcIndexMigrationError::ReservationTtlBelowFloor { .. }
        | BcIndexMigrationError::DrainTimeoutAbort
        | BcIndexMigrationError::FactoryRootNotFound { .. }
        | BcIndexMigrationError::ArchIndexParityAbort { .. }
        | BcIndexMigrationError::CompletionManifestRejection { .. }
        | BcIndexMigrationError::CensusMismatchAbort { .. }
        | BcIndexMigrationError::ContentPreservationAbort { .. }
        | BcIndexMigrationError::WriterAdmissionRefused { .. }
        | BcIndexMigrationError::ShardCapConfigUnavailable { .. } => {
            AdmissionFailureCause::StateIntegrity
        }
    }
}

/// ADR-052 v1.21 F-012: Display, exit code, cause classification of the new variant.
#[test]
fn test_BC_1_18_013_PC6c_admission_state_integrity_display_exit_code_and_cause() {
    for kind in ALL_KINDS {
        assert_eq!(kind.token(), expected_kind_token(kind));
        let e = BcIndexMigrationError::AdmissionStateIntegrity {
            kind,
            detail: "gate-state.json: bad".to_string(),
        };
        assert_eq!(
            e.to_string(),
            format!(
                "migration admission: state integrity failure ({}): gate-state.json: bad",
                expected_kind_token(kind)
            )
        );
        assert!(
            !e.to_string().contains("BINARY_INTEGRITY_FAILURE"),
            "Display must never contain BINARY_INTEGRITY_FAILURE"
        );
        assert_eq!(e.process_exit_code(), 2);
        assert_eq!(e.admission_failure_cause().token(), "state_integrity");
        assert_eq!(e.admission_failure_cause(), expected_cause(&e));
    }
    // the other two classes
    let io = BcIndexMigrationError::Io {
        path: PathBuf::from("/x"),
        source: std::io::Error::other("e"),
    };
    assert_eq!(io.admission_failure_cause().token(), "io");
    let inv = BcIndexMigrationError::InvalidToolUseId { len: 3 };
    assert_eq!(inv.admission_failure_cause().token(), "invalid_tool_use_id");
}

fn ms_with_gate(gate_bytes: &[u8]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let ms = dir.path().join("migration-state");
    std::fs::create_dir_all(ms.join("reservations")).unwrap();
    std::fs::write(ms.join("exclusive.lock"), b"").unwrap();
    std::fs::write(ms.join("gate-state.json"), gate_bytes).unwrap();
    (dir, ms)
}

fn txn_json(state: &str, mig: serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "txn_id": "txn-x", "activation_id": "act-x", "fencing_generation": 1,
        "state": state, "generation_id": "gen-1", "source_sha256": null,
        "source_body_row_sha256": null, "intent_log_path": null,
        "pending_canonical_moves": [], "created_at": "2026-10-06T00:00:00Z",
        "updated_at": "2026-10-06T00:00:00Z", "migration_id": mig,
    }))
    .unwrap()
}

/// ADR-052 v1.21 F-012: the admission-side raise sites raise
/// `AdmissionStateIntegrity { kind }`, NOT `BinaryIntegrityFailure`. (The fifth kind,
/// `ReservationSerialization`, cannot be provoked from outside the crate and is
/// covered by the Display/token test above.)
#[test]
fn test_BC_1_18_013_PC6c_admission_raise_sites_use_state_integrity_variant_with_kind() {
    let mut failures = Vec::new();
    let cases: Vec<(
        &str,
        Box<dyn Fn() -> (tempfile::TempDir, PathBuf)>,
        AdmissionStateIntegrityKind,
    )> = vec![
        (
            "gate record malformed",
            Box::new(|| ms_with_gate(b"\"BOGUS\"")),
            AdmissionStateIntegrityKind::GateRecordMalformed,
        ),
        (
            "txn record malformed",
            Box::new(|| {
                let (d, ms) = ms_with_gate(b"\"OPEN\"");
                std::fs::write(ms.join("txn-a.json"), b"{not json").unwrap();
                (d, ms)
            }),
            AdmissionStateIntegrityKind::TxnRecordMalformed,
        ),
        (
            "txn migration_id not a string",
            Box::new(|| {
                let (d, ms) = ms_with_gate(b"\"OPEN\"");
                std::fs::write(
                    ms.join("txn-a.json"),
                    txn_json("STAGING", serde_json::json!(42)),
                )
                .unwrap();
                (d, ms)
            }),
            AdmissionStateIntegrityKind::TxnMigrationIdNotString,
        ),
        (
            "multiple live txns",
            Box::new(|| {
                let (d, ms) = ms_with_gate(b"\"OPEN\"");
                std::fs::write(
                    ms.join("txn-a.json"),
                    txn_json("STAGING", serde_json::json!("migrate-bc-index")),
                )
                .unwrap();
                std::fs::write(
                    ms.join("txn-b.json"),
                    txn_json("COMMITTING", serde_json::json!("migrate-bc-index")),
                )
                .unwrap();
                (d, ms)
            }),
            AdmissionStateIntegrityKind::MultipleLiveTxns,
        ),
    ];
    for (label, mk, kind) in cases {
        let (_dir, ms) = mk();
        match admit_protected_write(&ms, Some("T1"), ProtectedPathFamily::Cycles) {
            Err(BcIndexMigrationError::AdmissionStateIntegrity { kind: got, detail }) => {
                if got != kind {
                    failures.push(format!("[{label}] kind {got:?} != {kind:?}"));
                }
                if detail.is_empty() {
                    failures.push(format!("[{label}] detail must be non-empty"));
                }
            }
            Err(other) => failures.push(format!(
                "[{label}] must raise AdmissionStateIntegrity {{ kind: {kind:?} }}, got `{other}` ({other:?})"
            )),
            Ok(AdmissionOutcome::Admitted { .. }) | Ok(AdmissionOutcome::Blocked { .. }) => {
                failures.push(format!("[{label}] must fail closed with an error"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} failure(s):\n  - {}",
        failures.len(),
        failures.join("\n  - ")
    );
}

#[allow(dead_code)]
fn _path_marker(_: &Path) {}
