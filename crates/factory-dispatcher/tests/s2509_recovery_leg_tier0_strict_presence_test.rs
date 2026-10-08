// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::type_complexity
)]
//! S-25.09 -- `migrate-bc-index` coordinator RECOVERY LEG, real spawned binary.
//!
//! Authority: ADR-052 v1.23 section Error Code Semantics, "Migration binaries --
//! recovery and finalize legs" items 1-5 and 7; BC-1.18.011 v1.20 Precondition 6(f)
//! (i)-(v), EC-043..EC-047 and the two v1.20 test-vector rows; BC-1.18.013 v1.12
//! EC-049..EC-052 (see "Not applicable" below); error-taxonomy v1.40
//! `MIGRATION_STATE_INTEGRITY_FAILURE` and `EXPIRY_ABORT`.
//!
//! | BC EC | Rule | Test |
//! |-------|------|------|
//! | EC-043 | STAGING `generation_id` absent / non-string-non-null | `..._EC043_staging_generation_id_absent_or_ill_typed_...` |
//! | EC-044 | COMMITTING `generation_id` null / absent / number | `..._EC044_committing_generation_id_not_a_string_...` |
//! | EC-045 | strict-presence decode at arm entry (3 named keys, both arms) | `..._EC045_strict_presence_named_keys_...` |
//! | EC-045 | iii: EVERY key required, non-Option keys reject null | `..._EC045_every_other_key_required_...` |
//! | EC-046(a)(b)(c) | terminal / foreign-terminal records never rejected | `..._EC046_terminal_records_never_rejected_...` |
//! | EC-046(d) | non-UTF-8 record is `TxnRecordMalformed`, not `Io` | `..._EC046_non_utf8_txn_record_...` |
//! | EC-046(e)(f) | live record of another migration / `migration_id` not in K | `..._EC046_live_foreign_record_refused_...` |
//! | EC-047 / EC-043 control | null-generation discard, exit 1 `EXPIRY_ABORT` | `..._EC047_null_generation_discard_...` |
//! | EC-047 | the SAME on-disk result as admission Branch B | `..._EC047_binary_discard_matches_admission_branch_b_...` |
//! | EC-043 control | string `generation_id` is not integrity-failed, not null-discarded | `..._EC043_control_string_generation_id_...` |
//!
//! Not applicable to this story (BC-1.18.013 v1.12 EC-049..EC-052 as seen by
//! `migrate-bc-index`): they bind the VERIFIER / FINALIZE leg (`activation_id`,
//! `generation_id`, `intent_log_path` read at their Branch C check; the intent-log
//! DONE-record hash source; the intent-log read-call failure mapping). The
//! `migrate-bc-index` binary has no such verifier today: its `completed.json`
//! short-circuit converges the txn best-effort without comparing anything, and the
//! Branch C verification (the nine-token `check` domain, the DONE-record hash
//! source) is S-25.06's. They are exercised by S-25.06's `backfill-append-logs`
//! verifier tests, not here.
//!
//! Every spawned child has stdin set to null and a per-command timeout.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const MIGRATION_ID: &str = "migrate-bc-index";
const BC_PATH: &str = ".factory/specs/behavioral-contracts/ss-01/BC-1.01.001.md";

/// The one stderr line the coordinator prints for `AdmissionStateIntegrity`
/// (`migrate-bc-index: ` + error-taxonomy v1.40 `Message: migration admission:
/// state integrity failure (<kind-token>): <detail>`). `<detail>` is free text.
const INTEGRITY_PREFIX: &str =
    "migrate-bc-index: migration admission: state integrity failure (txn_record_malformed): ";

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

struct Fx {
    dir: tempfile::TempDir,
}

impl Fx {
    /// `.factory/migration-state/` with an empty `exclusive.lock` and gate
    /// DRAINING (a crashed coordinator's state).
    fn new() -> Self {
        let fx = Fx {
            dir: tempfile::tempdir().expect("project tempdir"),
        };
        std::fs::create_dir_all(fx.ms().join("reservations")).unwrap();
        std::fs::write(fx.ms().join("exclusive.lock"), b"").unwrap();
        std::fs::write(fx.ms().join("gate-state.json"), "\"DRAINING\"").unwrap();
        fx
    }
    fn root(&self) -> &Path {
        self.dir.path()
    }
    fn ms(&self) -> PathBuf {
        self.root().join(".factory/migration-state")
    }
    fn txn(&self, name: &str, v: &Value) {
        std::fs::write(self.ms().join(name), serde_json::to_vec_pretty(v).unwrap()).unwrap();
    }
    fn txn_bytes(&self, name: &str, bytes: &[u8]) {
        std::fs::write(self.ms().join(name), bytes).unwrap();
    }
    /// `gen-<id>/` with a payload file, so a discard of it is observable.
    fn gen_dir(&self, id: &str) {
        let d = self.ms().join(format!("gen-{id}"));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("staged.bin"), b"staged-bytes").unwrap();
    }
    fn run(&self) -> Output {
        let mut cmd = Command::new(binary_path());
        cmd.arg("migrate-bc-index")
            .env("CLAUDE_PROJECT_DIR", self.root())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        finish(cmd.spawn().expect("spawn factory-dispatcher"), 30)
    }
}

fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_factory-dispatcher"))
}

fn finish(mut child: Child, secs: u64) -> Output {
    let start = Instant::now();
    let timeout = Duration::from_secs(secs);
    loop {
        if child.try_wait().expect("try_wait").is_some() {
            return child.wait_with_output().expect("wait_with_output");
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let _ = child.wait();
            panic!("child did not exit within {timeout:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn stderr_of(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

/// Whole-tree snapshot: relative path -> `None` (directory) / `Some(bytes)`.
fn snapshot(root: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Option<Vec<u8>>>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            let rel = p.strip_prefix(base).unwrap().to_string_lossy().to_string();
            if p.is_dir() {
                out.insert(format!("{rel}/"), None);
                walk(base, &p, out);
            } else {
                out.insert(rel, Some(std::fs::read(&p).unwrap_or_default()));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn assert_no_failures(test: &str, failures: Vec<String>) {
    assert!(
        failures.is_empty(),
        "{test}: {} scenario(s) failed:\n  - {}",
        failures.len(),
        failures.join("\n  - ")
    );
}

/// A record exactly as this build writes it (every key present, `Option` fields
/// `null`), plus the `migration_id` discriminator.
fn full(state: &str, generation_id: Value) -> Value {
    json!({
        "txn_id": "txn-act-1",
        "migration_id": MIGRATION_ID,
        "activation_id": "act-1",
        "fencing_generation": 1,
        "state": state,
        "generation_id": generation_id,
        "source_sha256": null,
        "source_body_row_sha256": null,
        "intent_log_path": null,
        "pending_canonical_moves": [],
        "created_at": "2026-10-07T00:00:00Z",
        "updated_at": "2026-10-07T00:00:00Z",
    })
}

fn minimal(state: &str) -> Value {
    json!({ "state": state, "migration_id": MIGRATION_ID })
}

fn without(mut v: Value, key: &str) -> Value {
    v.as_object_mut().unwrap().remove(key);
    v
}

fn with(mut v: Value, key: &str, val: Value) -> Value {
    v.as_object_mut().unwrap().insert(key.to_string(), val);
    v
}

/// ADR-052 v1.23 item 1+2 / BC-1.18.011 Precondition 6(f)(i): exit 2, ONE stderr
/// line `migrate-bc-index: migration admission: state integrity failure
/// (txn_record_malformed): <detail>`, empty stdout, never the digest code, never
/// `EXPIRY_ABORT`, and the whole project tree byte-identical.
fn check_integrity_refusal(
    label: &str,
    out: &Output,
    before: &BTreeMap<String, Option<Vec<u8>>>,
    after: &BTreeMap<String, Option<Vec<u8>>>,
    failures: &mut Vec<String>,
) {
    let err = stderr_of(out);
    let lines: Vec<&str> = err.lines().collect();
    let line_ok = lines.len() == 1
        && lines[0].starts_with(INTEGRITY_PREFIX)
        && lines[0].len() > INTEGRITY_PREFIX.len()
        && !err.contains("BINARY_INTEGRITY_FAILURE")
        && !err.contains("EXPIRY_ABORT");
    if out.status.code() != Some(2) || !out.stdout.is_empty() || !line_ok {
        failures.push(format!(
            "[{label}] expected exit 2, empty stdout and exactly ONE stderr line starting \
             `{INTEGRITY_PREFIX}` (MIGRATION_STATE_INTEGRITY_FAILURE, txn_record_malformed; not \
             BINARY_INTEGRITY_FAILURE, not EXPIRY_ABORT); got exit {:?}, stdout {} bytes, stderr \
             {lines:?}",
            out.status.code(),
            out.stdout.len()
        ));
    }
    diff_trees(label, before, after, &[], failures);
}

fn diff_trees(
    label: &str,
    before: &BTreeMap<String, Option<Vec<u8>>>,
    after: &BTreeMap<String, Option<Vec<u8>>>,
    ignore: &[&str],
    failures: &mut Vec<String>,
) {
    let keep = |m: &BTreeMap<String, Option<Vec<u8>>>| -> BTreeMap<String, Option<Vec<u8>>> {
        m.iter()
            .filter(|(k, _)| !ignore.iter().any(|i| k.ends_with(i)))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    };
    let (b, a) = (keep(before), keep(after));
    if b != a {
        let changed: Vec<&String> = b
            .keys()
            .chain(a.keys())
            .filter(|k| b.get(*k) != a.get(*k))
            .collect();
        failures.push(format!(
            "[{label}] the project tree must be byte-identical (nothing mutated); changed: \
             {changed:?}"
        ));
    }
}

// ===========================================================================
// EC-043 -- STAGING generation_id tri-state (absent / ill-typed are NEVER null)
// ===========================================================================

/// BC-1.18.011 v1.20 EC-043 (+ test-vector rows (a)(b)): a live known STAGING
/// txn whose `generation_id` key is REMOVED (minimal record, and the full record
/// minus the key) or is a non-string non-null value (`7`, `["x"]`, `true`, `{}`)
/// => exit 2 `MIGRATION_STATE_INTEGRITY_FAILURE`; the txn bytes are UNCHANGED (not
/// rewritten to ABORTED, no `abort_reason`), the gate and the whole
/// `migration-state/` tree byte-identical. An absent key is NEVER read as null.
#[test]
fn test_BC_1_18_011_EC043_staging_generation_id_absent_or_ill_typed_exit_2_integrity_not_a_discard_blackbox()
 {
    let mut failures = Vec::new();
    let variants: Vec<(&str, Value)> = vec![
        ("minimal, key absent", minimal("STAGING")),
        (
            "full minus generation_id",
            without(full("STAGING", Value::Null), "generation_id"),
        ),
        ("full, generation_id 7", full("STAGING", json!(7))),
        ("full, generation_id [\"x\"]", full("STAGING", json!(["x"]))),
        ("full, generation_id true", full("STAGING", json!(true))),
        ("full, generation_id {}", full("STAGING", json!({}))),
        (
            "minimal, generation_id 7",
            with(minimal("STAGING"), "generation_id", json!(7)),
        ),
    ];
    for (label, rec) in variants {
        let fx = Fx::new();
        fx.txn("txn-act-1.json", &rec);
        let txn_before = std::fs::read(fx.ms().join("txn-act-1.json")).unwrap();
        let before = snapshot(fx.root());
        let out = fx.run();
        let after = snapshot(fx.root());
        check_integrity_refusal(label, &out, &before, &after, &mut failures);
        let txn_after = std::fs::read(fx.ms().join("txn-act-1.json")).unwrap_or_default();
        if txn_after != txn_before || String::from_utf8_lossy(&txn_after).contains("abort_reason") {
            failures.push(format!(
                "[{label}] the txn record must be byte-identical (NOT rewritten to ABORTED, no \
                 abort_reason): an absent / ill-typed generation_id is never the null-generation \
                 discard"
            ));
        }
    }
    assert_no_failures(
        "test_BC_1_18_011_EC043_staging_generation_id_absent_or_ill_typed_exit_2_integrity_not_a_discard_blackbox",
        failures,
    );
}

/// BC-1.18.011 v1.20 EC-043 control (test-vector (d)): `"generation_id": "gen-1"`
/// is the planner's (resume / clean-abort), NOT an integrity failure and NOT the
/// null-generation discard. The planner outcome is deliberately not pinned here
/// (it depends on shard content); what is pinned is that the string value is not
/// misclassified by the tri-state: no `state integrity failure` line, and the txn
/// never carries `abort_reason: "null_generation"`.
#[test]
fn test_BC_1_18_011_EC043_control_string_generation_id_is_not_integrity_failure_and_not_null_discard_blackbox()
 {
    let mut failures = Vec::new();
    for with_gen_dir in [false, true] {
        let label = if with_gen_dir {
            "string generation_id, gen dir present"
        } else {
            "string generation_id, gen dir absent"
        };
        let fx = Fx::new();
        fx.txn("txn-act-1.json", &full("STAGING", json!("gen-1")));
        if with_gen_dir {
            fx.gen_dir("gen-1");
        }
        let out = fx.run();
        let err = stderr_of(&out);
        if err.contains("state integrity failure") {
            failures.push(format!(
                "[{label}] a string generation_id must reach the planner, not the \
                 MIGRATION_STATE_INTEGRITY_FAILURE tri-state arm; stderr {err:?}"
            ));
        }
        let txn_after = std::fs::read_to_string(fx.ms().join("txn-act-1.json")).unwrap_or_default();
        if txn_after.contains("null_generation") {
            failures.push(format!(
                "[{label}] a string generation_id must never be discarded as null_generation"
            ));
        }
    }
    assert_no_failures(
        "test_BC_1_18_011_EC043_control_string_generation_id_is_not_integrity_failure_and_not_null_discard_blackbox",
        failures,
    );
}

// ===========================================================================
// EC-044 -- COMMITTING requires a string generation_id
// ===========================================================================

/// BC-1.18.011 v1.20 EC-044 (+ test-vector rows (e)(f)(g)): a live known
/// COMMITTING txn whose `generation_id` is JSON `null`, ABSENT or a number =>
/// exit 2 `MIGRATION_STATE_INTEGRITY_FAILURE` in all three; stderr lacks
/// `BINARY_INTEGRITY_FAILURE` (supersedes the former `Quarantine {
/// CommittingWithoutGenerationId }` mapping), is not `EXPIRY_ABORT`, and nothing
/// is mutated (a COMMITTING record is never discarded; no forward-recovery
/// renames, no txn rewrite, no gate write). Both the minimal and the full record.
#[test]
fn test_BC_1_18_011_EC044_committing_generation_id_not_a_string_exit_2_integrity_nothing_mutated_blackbox()
 {
    let mut failures = Vec::new();
    let variants: Vec<(&str, Value)> = vec![
        ("minimal, key absent", minimal("COMMITTING")),
        (
            "minimal, generation_id null",
            with(minimal("COMMITTING"), "generation_id", Value::Null),
        ),
        (
            "minimal, generation_id 7",
            with(minimal("COMMITTING"), "generation_id", json!(7)),
        ),
        ("full, generation_id null", full("COMMITTING", Value::Null)),
        (
            "full minus generation_id",
            without(full("COMMITTING", Value::Null), "generation_id"),
        ),
        ("full, generation_id 7", full("COMMITTING", json!(7))),
        (
            "full, generation_id [\"x\"]",
            full("COMMITTING", json!(["x"])),
        ),
    ];
    for (label, rec) in variants {
        let fx = Fx::new();
        fx.txn("txn-act-1.json", &rec);
        let before = snapshot(fx.root());
        let out = fx.run();
        let after = snapshot(fx.root());
        check_integrity_refusal(label, &out, &before, &after, &mut failures);
    }
    assert_no_failures(
        "test_BC_1_18_011_EC044_committing_generation_id_not_a_string_exit_2_integrity_nothing_mutated_blackbox",
        failures,
    );
}

// ===========================================================================
// EC-045 -- strict-presence decode at arm entry
// ===========================================================================

/// BC-1.18.011 v1.20 EC-045 (+ test-vector "EC-045/EC-046" row): a live known txn
/// reaching the `ResumeFromStaging` arm (STAGING, string `generation_id`, gen dir
/// present) or the `ForwardRecovery` arm (COMMITTING, string `generation_id`, gen
/// dir present) that is MISSING `activation_id`, `fencing_generation` or
/// `pending_canonical_moves` (one fixture per key), or whose `activation_id` is a
/// number => exit 2 `MIGRATION_STATE_INTEGRITY_FAILURE` raised BEFORE the first
/// mutation: txn, gate, `gen-<id>/`, staging, intent log and canonical files
/// byte-identical -- never "repaired" or re-serialized with a defaulted field.
#[test]
fn test_BC_1_18_011_EC045_strict_presence_named_keys_missing_exit_2_integrity_before_first_mutation_blackbox()
 {
    let mut failures = Vec::new();
    for state in ["STAGING", "COMMITTING"] {
        let base = full(state, json!("gen-1"));
        let mut variants: Vec<(String, Value)> = [
            "activation_id",
            "fencing_generation",
            "pending_canonical_moves",
        ]
        .iter()
        .map(|k| (format!("{state}: {k} removed"), without(base.clone(), k)))
        .collect();
        variants.push((
            format!("{state}: activation_id is a number"),
            with(base.clone(), "activation_id", json!(5)),
        ));
        for (label, rec) in variants {
            let fx = Fx::new();
            fx.txn("txn-act-1.json", &rec);
            fx.gen_dir("gen-1");
            let before = snapshot(fx.root());
            let out = fx.run();
            let after = snapshot(fx.root());
            check_integrity_refusal(&label, &out, &before, &after, &mut failures);
        }
    }
    assert_no_failures(
        "test_BC_1_18_011_EC045_strict_presence_named_keys_missing_exit_2_integrity_before_first_mutation_blackbox",
        failures,
    );
}

/// BC-1.18.011 v1.20 Precondition 6(f)(iii): "EVERY key required to be present
/// (an `Option` field must be present, JSON `null` allowed; `#[serde(default)]`
/// is forbidden on txn fields ...). Any absent, ill-typed or -- for a field the
/// arm requires to hold a value -- null field => (i)". The remaining keys of the
/// record (`txn_id`, `created_at`, `updated_at`, and the three `Option` keys
/// `source_sha256`, `source_body_row_sha256`, `intent_log_path`) removed, and the
/// non-`Option` keys set to `null`, on both arms => exit 2 integrity failure,
/// nothing mutated.
#[test]
fn test_BC_1_18_011_EC045_every_other_key_required_absent_option_key_and_null_non_option_key_rejected_blackbox()
 {
    let mut failures = Vec::new();
    for state in ["STAGING", "COMMITTING"] {
        let base = full(state, json!("gen-1"));
        let mut variants: Vec<(String, Value)> = Vec::new();
        for k in [
            "txn_id",
            "created_at",
            "updated_at",
            "source_sha256",
            "source_body_row_sha256",
            "intent_log_path",
        ] {
            variants.push((format!("{state}: {k} removed"), without(base.clone(), k)));
        }
        for k in [
            "activation_id",
            "fencing_generation",
            "pending_canonical_moves",
            "txn_id",
            "created_at",
            "updated_at",
        ] {
            variants.push((
                format!("{state}: non-Option {k} is null"),
                with(base.clone(), k, Value::Null),
            ));
        }
        for (label, rec) in variants {
            let fx = Fx::new();
            fx.txn("txn-act-1.json", &rec);
            fx.gen_dir("gen-1");
            let before = snapshot(fx.root());
            let out = fx.run();
            let after = snapshot(fx.root());
            check_integrity_refusal(&label, &out, &before, &after, &mut failures);
        }
    }
    assert_no_failures(
        "test_BC_1_18_011_EC045_every_other_key_required_absent_option_key_and_null_non_option_key_rejected_blackbox",
        failures,
    );
}

// ===========================================================================
// EC-046 -- loader precedence (Tier 0 raw reader)
// ===========================================================================

fn live_null_generation_fixture(fx: &Fx) {
    // Sorts AFTER every `txn-a-*.json` and BEFORE nothing the tests add; the live
    // known record the coordinator must still recover ("recovery proceeds") --
    // observable with minimal fixtures as the null-generation discard.
    fx.txn("txn-live.json", &full("STAGING", Value::Null));
}

/// BC-1.18.011 v1.20 EC-046 (a)(b)(c) + test-vector: beside a live known txn, a
/// COMPLETED record from a NEWER schema (extra unknown fields, `activation_id`
/// absent), an ABORTED record missing every Tier 1 field (`{"state":"ABORTED"}`),
/// and foreign terminal records (`migration_id: "future-migration"`; COMPLETED and
/// ABORTED) are NOT rejected for a missing Tier 1 field and never modified or
/// deleted; recovery of the live known txn proceeds. (Observed as the
/// null-generation discard of the live record: exit 1 `EXPIRY_ABORT`, live txn
/// ABORTED, every terminal record byte-identical.) The loader must not fully
/// deserialize at read time.
#[test]
fn test_BC_1_18_011_EC046_terminal_records_never_rejected_for_missing_tier1_and_never_modified_recovery_proceeds_blackbox()
 {
    let mut failures = Vec::new();
    let terminals: Vec<(&str, Vec<(&str, Value)>)> = vec![
        (
            "(a) COMPLETED from a newer schema",
            vec![(
                "txn-a-completed.json",
                json!({
                    "state": "COMPLETED",
                    "migration_id": MIGRATION_ID,
                    "schema_version": 9,
                    "txn_ulid": "01HZZZ",
                    "future_blob": {"x": [1, 2, 3]},
                }),
            )],
        ),
        (
            "(b) ABORTED missing every Tier 1 field",
            vec![("txn-a-aborted.json", json!({"state": "ABORTED"}))],
        ),
        (
            "(c) foreign COMPLETED",
            vec![(
                "txn-a-foreign-completed.json",
                json!({"state": "COMPLETED", "migration_id": "future-migration"}),
            )],
        ),
        (
            "(c) foreign ABORTED",
            vec![(
                "txn-a-foreign-aborted.json",
                json!({"state": "ABORTED", "migration_id": "future-migration"}),
            )],
        ),
        (
            "(a)+(b)+(c) together",
            vec![
                (
                    "txn-a-1-completed.json",
                    json!({"state": "COMPLETED", "extra": true}),
                ),
                ("txn-a-2-aborted.json", json!({"state": "ABORTED"})),
                (
                    "txn-a-3-foreign.json",
                    json!({"state": "ABORTED", "migration_id": "future-migration"}),
                ),
            ],
        ),
    ];
    for (label, recs) in terminals {
        let fx = Fx::new();
        for (name, v) in &recs {
            fx.txn(name, v);
        }
        live_null_generation_fixture(&fx);
        let before = snapshot(fx.root());
        let out = fx.run();
        let after = snapshot(fx.root());
        let err = stderr_of(&out);
        if out.status.code() != Some(1)
            || !err.contains("EXPIRY_ABORT")
            || err.contains("BINARY_INTEGRITY_FAILURE")
            || err.contains("state integrity failure")
        {
            failures.push(format!(
                "[{label}] the live known txn must still be recovered (null-generation discard, \
                 exit 1 EXPIRY_ABORT) -- a terminal record is never rejected for a missing \
                 Tier 1 field; got exit {:?}, stderr {err:?}",
                out.status.code()
            ));
        }
        for (name, _) in &recs {
            let rel = format!(".factory/migration-state/{name}");
            if before.get(&rel) != after.get(&rel) {
                failures.push(format!(
                    "[{label}] terminal record {name} must be neither modified nor deleted nor \
                     archived"
                ));
            }
        }
    }
    assert_no_failures(
        "test_BC_1_18_011_EC046_terminal_records_never_rejected_for_missing_tier1_and_never_modified_recovery_proceeds_blackbox",
        failures,
    );
}

/// BC-1.18.011 v1.20 EC-046 (d) (+ Precondition 6(f)(ii)): a txn record that is
/// not valid UTF-8 (bytes `0xFF 0xFE`) is `TxnRecordMalformed` -- exit 2
/// `MIGRATION_STATE_INTEGRITY_FAILURE`, NOT `Io` (the former `String::from_utf8`
/// -> `Io { InvalidData }` mapping) -- with nothing mutated. Alone, and beside a
/// valid live known txn (the first failure in ascending filename order wins, so
/// the live record is not recovered).
#[test]
fn test_BC_1_18_011_EC046_non_utf8_txn_record_is_txn_record_malformed_not_io_nothing_mutated_blackbox()
 {
    let mut failures = Vec::new();
    for (label, besides_valid_live) in [
        ("sole non-UTF-8 record", false),
        ("beside a valid live", true),
    ] {
        let fx = Fx::new();
        fx.txn_bytes("txn-a-bad.json", &[0xFF, 0xFE]);
        if besides_valid_live {
            live_null_generation_fixture(&fx);
        }
        let before = snapshot(fx.root());
        let out = fx.run();
        let after = snapshot(fx.root());
        check_integrity_refusal(label, &out, &before, &after, &mut failures);
    }
    assert_no_failures(
        "test_BC_1_18_011_EC046_non_utf8_txn_record_is_txn_record_malformed_not_io_nothing_mutated_blackbox",
        failures,
    );
}

/// BC-1.18.011 v1.20 EC-046 (e)(f) + Precondition 6(e)/6(f)(ii): a LIVE record of
/// another known migration (`backfill-append-logs`) or with `migration_id` not in
/// K (`future-migration`) is refused with the `LockContention`-class exit 2,
/// `recover()` not run, NO Tier 1 field read, nothing mutated -- in particular a
/// foreign STAGING record with `generation_id: null` is NOT discarded, and a
/// minimal foreign record (no Tier 1 field at all) is NOT a state-integrity
/// failure. (The `LockContention`-class stderr text is not pinned by the spec; the
/// pinned parts are the exit code, the zero mutation, and that the refusal is
/// neither the Tier 1 integrity failure nor `EXPIRY_ABORT`.)
#[test]
fn test_BC_1_18_011_EC046_live_foreign_record_refused_exit_2_no_tier1_read_nothing_mutated_blackbox()
 {
    let mut failures = Vec::new();
    let variants: Vec<(&str, Value)> = vec![
        (
            "(e) backfill-append-logs STAGING, generation_id null",
            with(
                full("STAGING", Value::Null),
                "migration_id",
                json!("backfill-append-logs"),
            ),
        ),
        (
            "(e) backfill-append-logs COMMITTING, generation_id absent",
            with(
                without(full("COMMITTING", Value::Null), "generation_id"),
                "migration_id",
                json!("backfill-append-logs"),
            ),
        ),
        (
            "(f) future-migration STAGING, generation_id null",
            with(
                full("STAGING", Value::Null),
                "migration_id",
                json!("future-migration"),
            ),
        ),
        (
            "(f) minimal future-migration STAGING",
            json!({"state": "STAGING", "migration_id": "future-migration"}),
        ),
        (
            "(f) minimal future-migration COMMITTING",
            json!({"state": "COMMITTING", "migration_id": "future-migration"}),
        ),
        (
            "(e) minimal backfill-append-logs STAGING",
            json!({"state": "STAGING", "migration_id": "backfill-append-logs"}),
        ),
    ];
    for (label, rec) in variants {
        let fx = Fx::new();
        fx.txn("txn-act-1.json", &rec);
        let before = snapshot(fx.root());
        let out = fx.run();
        let after = snapshot(fx.root());
        let err = stderr_of(&out);
        if out.status.code() != Some(2)
            || err.trim().is_empty()
            || err.contains("state integrity failure")
            || err.contains("EXPIRY_ABORT")
        {
            failures.push(format!(
                "[{label}] expected the LockContention-class refusal: exit 2, a stderr \
                 diagnostic, neither a state-integrity failure (no Tier 1 field read) nor \
                 EXPIRY_ABORT; got exit {:?}, stderr {err:?}",
                out.status.code()
            ));
        }
        diff_trees(label, &before, &after, &[], &mut failures);
    }
    assert_no_failures(
        "test_BC_1_18_011_EC046_live_foreign_record_refused_exit_2_no_tier1_read_nothing_mutated_blackbox",
        failures,
    );
}

// ===========================================================================
// EC-047 -- the null-generation discard: shared primitive, exit 1 EXPIRY_ABORT
// ===========================================================================

fn read_json(p: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

/// BC-1.18.011 v1.20 EC-047 / EC-043 control (c) / Precondition 6(f)(iv): a live
/// known STAGING txn with `"generation_id": null` (PRESENT) is discarded with the
/// EXISTING ADR-052 section 4e semantics -- txn rewritten IN PLACE to `ABORTED`
/// with `abort_reason: "null_generation"` (every other field preserved, retained at
/// its original path), gate -> OPEN -- and the binary exits **1** `EXPIRY_ABORT`
/// (the third `EXPIRY_ABORT` arm, a non-error sentinel), NOT exit 2
/// `BinaryIntegrityFailure` ("resumed STAGING txn record has no generation_id").
/// Both the full record (`source_sha256: null`) and the minimal record.
#[test]
fn test_BC_1_18_011_EC047_null_generation_discard_exit_1_expiry_abort_txn_aborted_with_reason_gate_open_blackbox()
 {
    let mut failures = Vec::new();
    let variants: Vec<(&str, Value)> = vec![
        ("full record", full("STAGING", Value::Null)),
        (
            "minimal record",
            with(minimal("STAGING"), "generation_id", Value::Null),
        ),
        (
            "full record, migration_id key absent (read as migrate-bc-index)",
            without(full("STAGING", Value::Null), "migration_id"),
        ),
    ];
    for (label, rec) in variants {
        let fx = Fx::new();
        fx.txn("txn-act-1.json", &rec);
        let before = snapshot(fx.root());
        let out = fx.run();
        let after = snapshot(fx.root());
        let err = stderr_of(&out);
        let lines: Vec<&str> = err.lines().collect();
        if out.status.code() != Some(1)
            || !out.stdout.is_empty()
            || lines.len() != 1
            || !lines[0].starts_with("migrate-bc-index: ")
            || !err.contains("EXPIRY_ABORT")
            || err.contains("BINARY_INTEGRITY_FAILURE")
            || err.contains("state integrity failure")
        {
            failures.push(format!(
                "[{label}] expected exit 1 and ONE stderr line `migrate-bc-index: ...EXPIRY_ABORT...` \
                 (ADR-052 4e third arm, not BinaryIntegrityFailure); got exit {:?}, stderr {lines:?}",
                out.status.code()
            ));
        }
        let mut want = rec.clone();
        want["state"] = json!("ABORTED");
        want["abort_reason"] = json!("null_generation");
        let got = read_json(&fx.ms().join("txn-act-1.json"));
        if got != want {
            failures.push(format!(
                "[{label}] the txn must be retained at its original path as the SAME record with \
                 state=ABORTED and abort_reason=\"null_generation\" (other fields preserved); \
                 want {want}, got {got}"
            ));
        }
        let gate = read_json(&fx.ms().join("gate-state.json"));
        if gate != json!("OPEN") {
            failures.push(format!(
                "[{label}] gate must be OPEN after the discard; got {gate}"
            ));
        }
        diff_trees(
            label,
            &before,
            &after,
            &["txn-act-1.json", "gate-state.json"],
            &mut failures,
        );
    }
    assert_no_failures(
        "test_BC_1_18_011_EC047_null_generation_discard_exit_1_expiry_abort_txn_aborted_with_reason_gate_open_blackbox",
        failures,
    );
}

fn envelope(event: &str, tool: &str, tool_use_id: &str, tool_input: Value) -> String {
    json!({
        "hook_event_name": event,
        "tool_name": tool,
        "session_id": "sess-s2509-rl",
        "tool_use_id": tool_use_id,
        "tool_input": tool_input,
    })
    .to_string()
}

/// BC-1.18.011 v1.20 EC-047 (shared primitive, `abort_null_generation_txn`): the
/// SAME pre-generation record (`state: STAGING`, `generation_id: null`,
/// `source_sha256: null`) is discarded once by admission Branch B (PreToolUse, no
/// live coordinator) and once by `migrate-bc-index` recovery; both produce the
/// SAME on-disk result -- the txn file byte-identical to the other surface's, and
/// the same gate file -- "the same record is discarded by both surfaces or by
/// neither". (Admission admits with `branch_b_txn_aborted`; the binary exits 1.)
#[test]
fn test_BC_1_18_011_EC047_binary_discard_matches_admission_branch_b_on_disk_result_blackbox() {
    let rec = full("STAGING", Value::Null);

    // Surface 1: admission Branch B (PreToolUse Edit on a BC path; lock free).
    let adm = Fx::new();
    adm.txn("txn-act-1.json", &rec);
    let plugin_root = tempfile::tempdir().unwrap();
    std::fs::write(
        plugin_root.path().join("hooks-registry.toml"),
        "schema_version = 2\n",
    )
    .unwrap();
    let log_dir = tempfile::tempdir().unwrap();
    let abs = adm.root().join(BC_PATH);
    std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
    let payload = envelope(
        "PreToolUse",
        "Edit",
        "toolu_S2509RL",
        json!({"file_path": abs.to_string_lossy(), "old_string": "a", "new_string": "b"}),
    );
    let mut cmd = Command::new(binary_path());
    cmd.env("CLAUDE_PLUGIN_ROOT", plugin_root.path())
        .env("CLAUDE_PROJECT_DIR", adm.root())
        .env("VSDD_LOG_DIR", log_dir.path())
        .env_remove("VSDD_TEST_ADMISSION_SEAM_DIR")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn factory-dispatcher");
    let mut stdin = child.stdin.take().expect("child stdin");
    stdin.write_all(payload.as_bytes()).expect("write payload");
    drop(stdin);
    let adm_out = finish(child, 30);
    assert_eq!(
        adm_out.status.code(),
        Some(0),
        "admission Branch B must admit after the discard; stderr {:?}",
        stderr_of(&adm_out)
    );

    // Surface 2: the binary.
    let bin = Fx::new();
    bin.txn("txn-act-1.json", &rec);
    let bin_out = bin.run();

    let mut failures = Vec::new();
    if bin_out.status.code() != Some(1) {
        failures.push(format!(
            "binary must exit 1 (EXPIRY_ABORT); got {:?}, stderr {:?}",
            bin_out.status.code(),
            stderr_of(&bin_out)
        ));
    }
    for f in ["txn-act-1.json", "gate-state.json"] {
        let a = std::fs::read(adm.ms().join(f)).unwrap();
        let b = std::fs::read(bin.ms().join(f)).unwrap_or_default();
        if a != b {
            failures.push(format!(
                "{f}: the two surfaces must produce the SAME bytes (one shared \
                 abort_null_generation_txn primitive); admission {:?} vs binary {:?}",
                String::from_utf8_lossy(&a),
                String::from_utf8_lossy(&b)
            ));
        }
    }
    assert_no_failures(
        "test_BC_1_18_011_EC047_binary_discard_matches_admission_branch_b_on_disk_result_blackbox",
        failures,
    );
}
