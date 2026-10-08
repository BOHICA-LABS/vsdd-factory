// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::type_complexity
)]
//! S-25.09 -- ADR-052 v1.23 "third binary-leg extension", the IN-PROCESS share.
//!
//! Authority: ADR-052 v1.23 section Error Code Semantics "Migration binaries -- recovery
//! and finalize legs" items 7(e) (exit-code classes, exhaustive `process_exit_code`),
//! 10(a)-(b) (nested strictness; `schema_version` required, written as 1), and the
//! Downstream "Red tests owed" list items (1), (8) (write round trip) and the CleanAbort
//! stand-in for (7)/(8).
//!
//! | Owed item | Test |
//! |-----------|------|
//! | (1) exit-code table, every `BcIndexMigrationError` variant | `..._exit_code_table_every_variant_has_its_ruled_exit_exhaustive_no_wildcard` |
//! | (1) the table lists exactly the variants the enum declares | `..._exit_code_table_lists_exactly_the_declared_variants` |
//! | (1) `process_exit_code` has no wildcard arm and names every variant | `..._process_exit_code_source_has_no_wildcard_arm_and_names_every_variant` |
//! | (7)/(8) third rewriting arm (`CleanAbortExpiredStaging`, unreachable through the binary) via the shared strict decoder `read_active_txn_record` | `..._shared_strict_decoder_via_read_active_txn_record_...` |
//! | (8) written records carry `schema_version: 1` (round trip through `write_txn_record`) | `..._write_txn_record_emits_schema_version_1_and_all_twelve_keys` |
//! | (8) record CREATION carries `schema_version: 1` (a real fresh run) | `..._fresh_run_creates_and_rewrites_the_txn_record_with_schema_version_1` |
//!
//! Why the exit-code table is built the way it is. `process_exit_code` lives in the
//! production crate, so a test cannot make rustc check ITS match for exhaustiveness. What a
//! test can do is (a) hold its OWN exhaustive, wildcard-free `match` over the enum (adding a
//! variant breaks THIS file's build until the new variant is given an explicit ruled exit
//! here), (b) cross-check that the table's instance list covers exactly the variants the
//! production enum declares (a source scan), and (c) scan the production `process_exit_code`
//! body for the wildcard arm ADR item 7(e) deletes. (a)+(b) are green on arrival (the wildcard
//! `_ => 2` happens to produce the ruled values today); they are proven non-vacuous by
//! mutants in the commit report. (c) is the red test.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use factory_dispatcher::shard_manager::migration_fs::StdFs;
use factory_dispatcher::shard_manager::{
    AdmissionStateIntegrityKind, BcIndexAddressingError, BcIndexMigrationError,
    BcIndexMigrationOutcome, BcIndexMigrationTxnRecord, ExpiryAbortArm,
    migration_process_exit_code, read_active_txn_record, run_bc_index_migration, write_txn_record,
};
use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// (1) The exit-code table
// ---------------------------------------------------------------------------

/// The ruled exit class of every variant (ADR-052 v1.23 item 7(e)): exit 1 = "no harm done,
/// safe to re-run; the stderr code token says what to do next" for exactly
/// {`ExpiryAbort` (both arms), `MigrationLockContention`}; exit 2 = fail closed, an operator
/// must act, for every other variant. EXHAUSTIVE and WILDCARD-FREE on purpose: a new
/// `BcIndexMigrationError` variant fails this file's build until it is assigned an exit class
/// here.
fn ruled(e: &BcIndexMigrationError) -> (&'static str, i32) {
    match e {
        BcIndexMigrationError::BinaryIntegrityFailure { .. } => ("BinaryIntegrityFailure", 2),
        BcIndexMigrationError::RecoveryRequiresReauthorization => {
            ("RecoveryRequiresReauthorization", 2)
        }
        BcIndexMigrationError::ExpiryAbort { .. } => ("ExpiryAbort", 1),
        BcIndexMigrationError::FingerprintMismatchAbort => ("FingerprintMismatchAbort", 2),
        BcIndexMigrationError::ReservationTtlBelowFloor { .. } => ("ReservationTtlBelowFloor", 2),
        BcIndexMigrationError::DrainTimeoutAbort => ("DrainTimeoutAbort", 2),
        BcIndexMigrationError::InvalidToolUseId { .. } => ("InvalidToolUseId", 2),
        BcIndexMigrationError::AdmissionStateIntegrity { .. } => ("AdmissionStateIntegrity", 2),
        BcIndexMigrationError::FactoryRootNotFound { .. } => ("FactoryRootNotFound", 2),
        BcIndexMigrationError::ArchIndexParityAbort { .. } => ("ArchIndexParityAbort", 2),
        BcIndexMigrationError::CompletionManifestRejection { .. } => {
            ("CompletionManifestRejection", 2)
        }
        BcIndexMigrationError::CensusMismatchAbort { .. } => ("CensusMismatchAbort", 2),
        BcIndexMigrationError::ContentPreservationAbort { .. } => ("ContentPreservationAbort", 2),
        BcIndexMigrationError::ForeignMigrationRefused { .. } => ("ForeignMigrationRefused", 2),
        BcIndexMigrationError::MigrationLockContention => ("MigrationLockContention", 1),
        BcIndexMigrationError::CompletionRecordMismatchInterim => {
            ("CompletionRecordMismatchInterim", 2)
        }
        BcIndexMigrationError::Io { .. } => ("Io", 2),
        BcIndexMigrationError::WriterAdmissionRefused { .. } => ("WriterAdmissionRefused", 2),
        BcIndexMigrationError::ShardCapConfigUnavailable { .. } => ("ShardCapConfigUnavailable", 2),
    }
}

/// One instance of every variant; the variants with a sub-classification are instantiated once
/// per sub-class (both `ExpiryAbortArm`s, every pre-v1.23 `AdmissionStateIntegrityKind`).
fn instances() -> Vec<(String, BcIndexMigrationError)> {
    let mut v: Vec<(String, BcIndexMigrationError)> = vec![
        (
            "BinaryIntegrityFailure".into(),
            BcIndexMigrationError::BinaryIntegrityFailure {
                message: "m".into(),
            },
        ),
        (
            "RecoveryRequiresReauthorization".into(),
            BcIndexMigrationError::RecoveryRequiresReauthorization,
        ),
        (
            "FingerprintMismatchAbort".into(),
            BcIndexMigrationError::FingerprintMismatchAbort,
        ),
        (
            "ReservationTtlBelowFloor".into(),
            BcIndexMigrationError::ReservationTtlBelowFloor {
                configured_secs: 1,
                floor_secs: 1800,
            },
        ),
        (
            "DrainTimeoutAbort".into(),
            BcIndexMigrationError::DrainTimeoutAbort,
        ),
        (
            "InvalidToolUseId".into(),
            BcIndexMigrationError::InvalidToolUseId { len: 0 },
        ),
        (
            "FactoryRootNotFound".into(),
            BcIndexMigrationError::FactoryRootNotFound {
                project_root: PathBuf::from("/nonexistent"),
                root_source: None,
            },
        ),
        (
            "ArchIndexParityAbort".into(),
            BcIndexMigrationError::ArchIndexParityAbort {
                source: BcIndexAddressingError::MalformedBcId {
                    candidate: "x".into(),
                },
            },
        ),
        (
            "CompletionManifestRejection".into(),
            BcIndexMigrationError::CompletionManifestRejection { reason: "r".into() },
        ),
        (
            "CensusMismatchAbort".into(),
            BcIndexMigrationError::CensusMismatchAbort {
                bc_id: "BC-1.01.001".into(),
                detail: "d".into(),
            },
        ),
        (
            "ContentPreservationAbort".into(),
            BcIndexMigrationError::ContentPreservationAbort { detail: "d".into() },
        ),
        (
            "ForeignMigrationRefused".into(),
            BcIndexMigrationError::ForeignMigrationRefused {
                live_migration_id: "other".into(),
            },
        ),
        (
            "MigrationLockContention".into(),
            BcIndexMigrationError::MigrationLockContention,
        ),
        (
            "CompletionRecordMismatchInterim".into(),
            BcIndexMigrationError::CompletionRecordMismatchInterim,
        ),
        (
            "Io".into(),
            BcIndexMigrationError::Io {
                path: PathBuf::from("/p"),
                source: std::io::Error::other("e"),
            },
        ),
        (
            "WriterAdmissionRefused".into(),
            BcIndexMigrationError::WriterAdmissionRefused { reason: "r".into() },
        ),
        (
            "ShardCapConfigUnavailable".into(),
            BcIndexMigrationError::ShardCapConfigUnavailable { detail: "d".into() },
        ),
    ];
    for arm in [
        ExpiryAbortArm::ManifestExpiredOrAbsent,
        ExpiryAbortArm::NullGeneration,
    ] {
        v.push((
            format!("ExpiryAbort/{arm:?}"),
            BcIndexMigrationError::ExpiryAbort { arm },
        ));
    }
    for kind in [
        AdmissionStateIntegrityKind::GateRecordMalformed,
        AdmissionStateIntegrityKind::TxnRecordMalformed,
        AdmissionStateIntegrityKind::TxnMigrationIdNotString,
        AdmissionStateIntegrityKind::MultipleLiveTxns,
        AdmissionStateIntegrityKind::ReservationSerialization,
    ] {
        v.push((
            format!("AdmissionStateIntegrity/{}", kind.token()),
            BcIndexMigrationError::AdmissionStateIntegrity {
                kind,
                subject: None,
                message: "d".into(),
                names: Vec::new(),
            },
        ));
    }
    v
}

/// ADR-052 v1.23 item 7(e): every `BcIndexMigrationError` variant maps to its ruled exit
/// status through the PRODUCTION `process_exit_code` -- exit 1 for exactly
/// {`ExpiryAbort { .. }` (both arms), `MigrationLockContention`}, exit 2 for every other
/// variant -- and `migration_process_exit_code` agrees (`Err(e)` => the variant's class,
/// `Ok(_)` => 0).
#[test]
fn test_BC_1_18_011_item7e_exit_code_table_every_variant_has_its_ruled_exit_exhaustive_no_wildcard()
{
    let mut failures = Vec::new();
    for (label, e) in instances() {
        let (_, want) = ruled(&e);
        let got = e.process_exit_code();
        if got != want {
            failures.push(format!(
                "[{label}] process_exit_code() == {got}, ruled exit is {want}"
            ));
        }
    }
    // The wrapper agrees (consume each instance once).
    for (label, e) in instances() {
        let (_, want) = ruled(&e);
        let got = migration_process_exit_code(&Err(e));
        if got != want {
            failures.push(format!(
                "[{label}] migration_process_exit_code(Err(..)) == {got}, ruled exit is {want}"
            ));
        }
    }
    for ok in [
        BcIndexMigrationOutcome::AlreadyMigrated,
        BcIndexMigrationOutcome::Completed {
            canonical_paths_count: 4,
        },
    ] {
        if migration_process_exit_code(&Ok(ok.clone())) != 0 {
            failures.push(format!("{ok:?} must map to exit 0"));
        }
    }
    assert!(
        failures.is_empty(),
        "exit-code table: {} mismatch(es):\n  - {}",
        failures.len(),
        failures.join("\n  - ")
    );
}

// ---------------------------------------------------------------------------
// Source scan helpers (comments and string contents blanked; attributes removed)
// ---------------------------------------------------------------------------

fn production_source() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shard_manager.rs");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Blank comments and the CONTENTS of string literals, keep everything else.
fn blank_comments_and_strings(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else if c == '"' {
            out.push('"');
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            out.push('"');
            i += 1;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// Remove every `#[ ... ]` attribute (bracket-matched) from already-blanked text.
fn strip_attributes(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '#' && chars.get(i + 1) == Some(&'[') {
            let mut depth = 0;
            while i < chars.len() {
                match chars[i] {
                    '[' => depth += 1,
                    ']' => {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// The text between the braces of the first `{ ... }` that follows `header` in the BLANKED
/// source (strings and comments already neutralised, so braces inside them cannot confuse
/// the matcher).
fn braced_block_after(blanked: &str, header: &str) -> String {
    let start = blanked
        .find(header)
        .unwrap_or_else(|| panic!("header {header:?} not found in the production source"));
    let rest = &blanked[start + header.len()..];
    let open = rest.find('{').expect("no `{` after header");
    let mut depth = 0;
    let mut end = None;
    for (i, c) in rest[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(open + i);
                    break;
                }
            }
            _ => {}
        }
    }
    rest[open + 1..end.expect("unbalanced braces")].to_string()
}

/// The variant names an enum body declares: split the (attribute-free) body on top-level
/// commas and take each segment's leading identifier.
fn declared_variants(enum_body: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut depth = 0i32;
    let mut seg = String::new();
    let flush = |seg: &mut String, names: &mut BTreeSet<String>| {
        let t = seg.trim();
        let ident: String = t
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !ident.is_empty() {
            names.insert(ident);
        }
        seg.clear();
    };
    for c in enum_body.chars() {
        match c {
            '{' | '(' | '[' => {
                depth += 1;
                seg.push(c);
            }
            '}' | ')' | ']' => {
                depth -= 1;
                seg.push(c);
            }
            ',' if depth == 0 => flush(&mut seg, &mut names),
            _ => seg.push(c),
        }
    }
    flush(&mut seg, &mut names);
    names
}

/// ADR-052 v1.23 item 7(e) cross-check: the table in this file lists EXACTLY the variants the
/// production enum declares (names compared, from a scan of `pub enum
/// BcIndexMigrationError`), so a variant cannot be added to the enum, given a row in neither
/// `ruled` nor `instances`, and slip through.
#[test]
fn test_BC_1_18_011_item7e_exit_code_table_lists_exactly_the_declared_variants() {
    let blanked = strip_attributes(&blank_comments_and_strings(&production_source()));
    let body = braced_block_after(&blanked, "pub enum BcIndexMigrationError");
    let declared = declared_variants(&body);
    let in_table: BTreeSet<String> = instances()
        .iter()
        .map(|(_, e)| ruled(e).0.to_string())
        .collect();
    assert!(
        declared.len() >= 19,
        "the enum scan found only {} variants ({declared:?}); the scanner is broken",
        declared.len()
    );
    assert_eq!(
        declared, in_table,
        "the exit-code table and the declared `BcIndexMigrationError` variants must be the same \
         set"
    );
}

/// ADR-052 v1.23 item 7(e) / Code changes (a): `process_exit_code` is an EXHAUSTIVE `match`
/// with NO wildcard arm -- "exit 1 is listed by name (`ExpiryAbort { .. } |
/// MigrationLockContention`), exit 2 is listed by name for every other variant -- the same
/// discipline as `admission_failure_cause`" -- so a future variant must be assigned an exit
/// class at compile time instead of silently inheriting 2. Pinned structurally (a runtime
/// test cannot see a production `match`'s exhaustiveness): the function body contains no
/// `_ =>` arm and names every declared variant as `BcIndexMigrationError::<Variant>`.
/// (`admission_failure_cause`, the cited precedent, is scanned the same way as a control that
/// the scanner accepts a genuinely exhaustive production match.)
#[test]
fn test_BC_1_18_011_item7e_process_exit_code_source_has_no_wildcard_arm_and_names_every_variant() {
    let blanked = strip_attributes(&blank_comments_and_strings(&production_source()));
    let enum_body = braced_block_after(&blanked, "pub enum BcIndexMigrationError");
    let declared = declared_variants(&enum_body);

    let scan = |fn_header: &str| -> Vec<String> {
        let body = braced_block_after(&blanked, fn_header);
        let mut problems = Vec::new();
        let squashed: String = body.split_whitespace().collect::<Vec<_>>().join(" ");
        if squashed.contains("_ =>") || squashed.contains("_ |") {
            problems.push(format!("`{fn_header}` has a wildcard `_` arm"));
        }
        for name in &declared {
            if !body.contains(&format!("BcIndexMigrationError::{name}")) {
                problems.push(format!(
                    "`{fn_header}` does not name `BcIndexMigrationError::{name}`"
                ));
            }
        }
        problems
    };

    // Control: the precedent the ADR cites is exhaustive and wildcard-free.
    let control = scan("pub fn admission_failure_cause(&self)");
    assert!(
        control.is_empty(),
        "scanner control (admission_failure_cause) must be clean: {control:?}"
    );

    let problems = scan("pub fn process_exit_code(&self)");
    assert!(
        problems.is_empty(),
        "process_exit_code must be an exhaustive match with no wildcard arm (ADR-052 v1.23 \
         item 7(e)):\n  - {}",
        problems.join("\n  - ")
    );
}

// ---------------------------------------------------------------------------
// The shared strict decoder (stand-in for the unreachable CleanAbortExpiredStaging arm)
// ---------------------------------------------------------------------------

fn live_record(state: &str) -> Value {
    json!({
        "schema_version": 1,
        "txn_id": "txn-act-1",
        "migration_id": "migrate-bc-index",
        "activation_id": "act-1",
        "fencing_generation": 1,
        "state": state,
        "generation_id": "gen-1",
        "source_sha256": null,
        "source_body_row_sha256": null,
        // ADR-052 v1.24 ruling (i): written with generation_id, equal to the generation path.
        "intent_log_path": ".factory/migration-state/intent-gen-1.log",
        "pending_canonical_moves": [],
        "created_at": "2026-10-07T00:00:00Z",
        "updated_at": "2026-10-07T00:00:00Z",
    })
}

fn decode(rec_bytes: &[u8]) -> Result<Option<BcIndexMigrationTxnRecord>, BcIndexMigrationError> {
    let dir = tempfile::tempdir().unwrap();
    let ms = dir.path().join(".factory/migration-state");
    std::fs::create_dir_all(&ms).unwrap();
    std::fs::write(ms.join("txn-act-1.json"), rec_bytes).unwrap();
    read_active_txn_record(&StdFs, &ms)
}

fn kind_and_detail(
    r: Result<Option<BcIndexMigrationTxnRecord>, BcIndexMigrationError>,
) -> Result<(String, String), String> {
    match r {
        Err(BcIndexMigrationError::AdmissionStateIntegrity {
            kind,
            subject,
            message,
            names,
        }) => {
            let detail = factory_dispatcher::shard_manager::admission_state_integrity_detail(
                subject.as_deref(),
                &message,
                &names,
                256,
            );
            Ok((kind.token().to_string(), detail))
        }
        other => Err(format!("{other:?}")),
    }
}

/// ADR-052 v1.23 items 10(a)-(b), shared strict decoder. The third rewriting arm,
/// `CleanAbortExpiredStaging`, cannot be reached through the real binary today
/// (`ManifestStatus` is fixed at `StillValid`), but it consumes the SAME
/// `decode_txn_record_strict` that the public `read_active_txn_record` applies to the one
/// live record. Through it, for both live states: `schema_version` 1 decodes;
/// an integer >= 2 is token `txn_record_newer_schema` (also when the record has other
/// defects: unknown key, missing key, nested defect); 0 / -1 / "1" / 1.5 / null / true /
/// [1] / {} are `txn_record_malformed` naming `schema_version` (not the unknown-key
/// text); ABSENT is `txn_record_malformed` naming `schema_version`; a nested unknown key,
/// non-object element or missing element key is `txn_record_malformed` naming
/// `pending_canonical_moves[<index>]`.
#[test]
fn test_BC_1_18_011_EC049_shared_strict_decoder_via_read_active_txn_record_version_gate_and_nested_strictness_inprocess()
 {
    let mut failures = Vec::new();
    for state in ["STAGING", "COMMITTING"] {
        let base = || live_record(state);
        let set = |mut v: Value, k: &str, x: Value| {
            v[k] = x;
            v
        };
        let drop_key = |mut v: Value, k: &str| {
            v.as_object_mut().unwrap().remove(k);
            v
        };
        // 1 decodes.
        match decode(&serde_json::to_vec(&base()).unwrap()) {
            Ok(Some(rec)) if rec.activation_id == "act-1" => {}
            other => failures.push(format!(
                "[{state}: schema_version 1] must decode; got {other:?}"
            )),
        }
        // newer schema (token, wins over other defects)
        let newer: Vec<(&str, Value)> = vec![
            ("2", set(base(), "schema_version", json!(2))),
            (
                "4294967296",
                set(base(), "schema_version", json!(4_294_967_296u64)),
            ),
            (
                "2 + unknown key",
                set(set(base(), "schema_version", json!(2)), "extra", json!(1)),
            ),
            (
                "2 + missing key",
                drop_key(set(base(), "schema_version", json!(2)), "txn_id"),
            ),
            (
                "2 + nested unknown key",
                set(
                    set(base(), "schema_version", json!(2)),
                    "pending_canonical_moves",
                    json!([{"staging_path": "s", "canonical_path": "c", "x": 1}]),
                ),
            ),
        ];
        for (what, rec) in newer {
            match kind_and_detail(decode(&serde_json::to_vec(&rec).unwrap())) {
                Ok((kind, _)) if kind == "txn_record_newer_schema" => {}
                other => failures.push(format!(
                    "[{state}: newer {what}] expected kind txn_record_newer_schema; got {other:?}"
                )),
            }
        }
        // malformed version values / absent
        let mut bad: Vec<(String, Value)> = [
            ("0", json!(0)),
            ("-1", json!(-1)),
            ("\"1\"", json!("1")),
            ("1.5", json!(1.5)),
            ("null", Value::Null),
            ("true", json!(true)),
            ("[1]", json!([1])),
            ("{}", json!({})),
        ]
        .into_iter()
        .map(|(w, v)| {
            (
                format!("schema_version {w}"),
                set(base(), "schema_version", v),
            )
        })
        .collect();
        bad.push((
            "schema_version ABSENT".into(),
            drop_key(base(), "schema_version"),
        ));
        for (what, rec) in bad {
            match kind_and_detail(decode(&serde_json::to_vec(&rec).unwrap())) {
                Ok((kind, detail))
                    if kind == "txn_record_malformed"
                        && detail.contains("schema_version")
                        && !detail.contains("unknown top-level key") => {}
                other => failures.push(format!(
                    "[{state}: {what}] expected txn_record_malformed naming schema_version (not \
                     the unknown-key text); got {other:?}"
                )),
            }
        }
        // 1 + unknown top-level key: malformed naming the unknown key, not the version.
        match kind_and_detail(decode(
            &serde_json::to_vec(&set(base(), "schema_v2_field", json!(1))).unwrap(),
        )) {
            Ok((kind, detail))
                if kind == "txn_record_malformed"
                    && detail.contains("schema_v2_field")
                    && !detail.contains("schema_version") => {}
            other => failures.push(format!(
                "[{state}: 1 + unknown key] expected malformed naming schema_v2_field only; got \
                 {other:?}"
            )),
        }
        // nested strictness
        let nested: Vec<(&str, Value, &str)> = vec![
            (
                "unknown key in element 0",
                json!([{"staging_path": "s", "canonical_path": "c", "extra": 1}]),
                "pending_canonical_moves[0]",
            ),
            (
                "unknown key in element 1",
                json!([
                    {"staging_path": "s", "canonical_path": "c"},
                    {"staging_path": "s", "canonical_path": "c", "extra": 1}
                ]),
                "pending_canonical_moves[1]",
            ),
            (
                "non-object element",
                json!(["not-an-object"]),
                "pending_canonical_moves[0]",
            ),
            (
                "missing staging_path",
                json!([{"canonical_path": "c"}]),
                "pending_canonical_moves[0]",
            ),
            (
                "missing canonical_path",
                json!([{"staging_path": "s"}]),
                "pending_canonical_moves[0]",
            ),
        ];
        for (what, moves, needle) in nested {
            let rec = set(base(), "pending_canonical_moves", moves);
            match kind_and_detail(decode(&serde_json::to_vec(&rec).unwrap())) {
                Ok((kind, detail)) if kind == "txn_record_malformed" && detail.contains(needle) => {
                }
                other => failures.push(format!(
                    "[{state}: nested {what}] expected txn_record_malformed naming {needle}; got \
                     {other:?}"
                )),
            }
        }
        // a valid nested element still decodes
        let ok = set(
            base(),
            "pending_canonical_moves",
            json!([{"staging_path": "s", "canonical_path": "c"}]),
        );
        match decode(&serde_json::to_vec(&ok).unwrap()) {
            Ok(Some(rec)) if rec.pending_canonical_moves.len() == 1 => {}
            other => failures.push(format!(
                "[{state}: valid nested element] must decode; got {other:?}"
            )),
        }
    }
    assert!(
        failures.is_empty(),
        "{} scenario(s) failed:\n  - {}",
        failures.len(),
        failures.join("\n  - ")
    );
}

// ---------------------------------------------------------------------------
// schema_version on WRITTEN records
// ---------------------------------------------------------------------------

/// ADR-052 v1.23 item 10(b): `BcIndexMigrationTxnRecord` gains a REQUIRED top-level
/// `schema_version: u32` = 1, "written by every `write_txn_record` call". The record is built
/// from JSON (which carries `schema_version: 1`; unknown keys are ignored by the current
/// struct, so this compiles and runs before and after the field lands) and written through the
/// public `write_txn_record`: the file on disk has EXACTLY the twelve keys, `schema_version`
/// the JSON integer 1, and a PRESENT `migration_id` already on disk is preserved verbatim.
#[test]
fn test_BC_1_18_011_EC049_write_txn_record_emits_schema_version_1_and_all_twelve_keys() {
    let rec: BcIndexMigrationTxnRecord = serde_json::from_value(live_record("STAGING")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let ms = dir.path().join(".factory/migration-state");
    std::fs::create_dir_all(&ms).unwrap();
    write_txn_record(&StdFs, &ms, &rec).expect("write_txn_record");
    let written: Value =
        serde_json::from_slice(&std::fs::read(ms.join("txn-act-1.json")).unwrap()).unwrap();
    let keys: BTreeSet<String> = written.as_object().unwrap().keys().cloned().collect();
    let want: BTreeSet<String> = [
        "schema_version",
        "txn_id",
        "activation_id",
        "fencing_generation",
        "state",
        "generation_id",
        "source_sha256",
        "source_body_row_sha256",
        "intent_log_path",
        "pending_canonical_moves",
        "created_at",
        "updated_at",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(
        keys, want,
        "a record this build writes carries exactly the twelve keys (no migration_id when none \
         was on disk)"
    );
    assert_eq!(
        written["schema_version"],
        json!(1),
        "schema_version must be the JSON integer 1"
    );

    // A second write over a file that already carries migration_id keeps it verbatim and still
    // carries schema_version 1.
    std::fs::write(
        ms.join("txn-act-1.json"),
        serde_json::to_vec(&live_record("STAGING")).unwrap(),
    )
    .unwrap();
    write_txn_record(&StdFs, &ms, &rec).expect("write_txn_record (rewrite)");
    let rewritten: Value =
        serde_json::from_slice(&std::fs::read(ms.join("txn-act-1.json")).unwrap()).unwrap();
    assert_eq!(rewritten["migration_id"], json!("migrate-bc-index"));
    assert_eq!(rewritten["schema_version"], json!(1));
}

const FRESH_RUN_BC_INDEX: &str = "\
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

const FRESH_RUN_SHARD_CONFIG: &str = "\
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

/// ADR-052 v1.23 item 10(b): the record is created with `schema_version: 1` ("written by
/// every `write_txn_record` call and by record creation"). A real fresh `migrate-bc-index`
/// run in-process creates the STAGING record, rewrites it through STAGING-with-generation and
/// COMMITTING, and finishes COMPLETED; the record left on disk is the 12-key shape with
/// `schema_version` = 1.
#[test]
fn test_BC_1_18_011_EC049_fresh_run_creates_and_rewrites_the_txn_record_with_schema_version_1() {
    let dir = tempfile::tempdir().unwrap();
    let factory = dir.path().join(".factory");
    std::fs::create_dir_all(&factory).unwrap();
    std::fs::write(factory.join("shard-config.toml"), FRESH_RUN_SHARD_CONFIG).unwrap();
    let canonical = dir
        .path()
        .join(".factory/specs/behavioral-contracts/BC-INDEX.md");
    std::fs::create_dir_all(canonical.parent().unwrap()).unwrap();
    std::fs::write(&canonical, FRESH_RUN_BC_INDEX).unwrap();
    let outcome = run_bc_index_migration(dir.path());
    assert!(
        matches!(outcome, Ok(BcIndexMigrationOutcome::Completed { .. })),
        "the fresh run must complete; got {outcome:?}"
    );
    let ms = factory.join("migration-state");
    let txns: Vec<PathBuf> = std::fs::read_dir(&ms)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().is_some_and(|n| {
                let n = n.to_string_lossy();
                n.starts_with("txn-") && n.ends_with(".json")
            })
        })
        .collect();
    assert_eq!(txns.len(), 1, "exactly one txn record: {txns:?}");
    let rec: Value = serde_json::from_slice(&std::fs::read(&txns[0]).unwrap()).unwrap();
    assert_eq!(rec["state"], json!("COMPLETED"));
    assert_eq!(
        rec["schema_version"],
        json!(1),
        "the record created and rewritten by a fresh run carries schema_version 1; got {rec}"
    );
}
