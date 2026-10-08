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
//! recovery and finalize legs" items 1-5 and 7-11; BC-1.18.011 v1.20 Precondition
//! 6(e), 6(f)(i)-(v), Postcondition 9(e), EC-043..EC-050 and the v1.20 test-vector
//! rows; BC-1.18.013 v1.12
//! EC-049..EC-052 (see "Not applicable" below); error-taxonomy v1.40
//! `MIGRATION_STATE_INTEGRITY_FAILURE` and `EXPIRY_ABORT`.
//!
//! | BC EC | Rule | Test |
//! |-------|------|------|
//! | EC-047 | item 8: EXACT null-generation `EXPIRY_ABORT` line; Display of both arms | `..._EC047_null_generation_discard_...`, `..._EC047_expiry_abort_display_...` |
//! | EC-047 | item 8: gate / txn write failure is `Io` exit 2, never `EXPIRY_ABORT` | `..._EC047_gate_write_failure_...`, `..._EC047_txn_write_failure_...` |
//! | EC-046(e)(f) | item 9: EXACT `FOREIGN_MIGRATION_REFUSED` line, id truncation / escaping | `..._EC046_live_foreign_record_refused_...`, `..._EC046_foreign_id_rendering_...` |
//! | EC-046 | item 9: precedence flock -> Tier 0 -> one-live -> foreign | `..._EC046_precedence_...` |
//! | EC-048 | item 9: `MIGRATION_LOCK_CONTENTION` exit 1, flock BEFORE loader | `..._EC048_...` |
//! | EC-045 | item 10: wrong-typed strict-presence values (full 11-key set) | `..._EC045_wrong_typed_values_...` |
//! | EC-049 | item 10: unknown top-level key; `migration_id` preserved verbatim | `..._EC049_...` |
//! | EC-050 | item 11: INTERIM `completed.json` short-circuit | `..._EC050_...` |
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
//! `migrate-bc-index` binary has no verifier in this story: the verify-then-finalize
//! is S-25.06 AC-031 (ADR-052 v1.23 item 11). Until it lands the `completed.json`
//! short-circuit is narrowed to the INTERIM fail-closed subset pinned by the EC-050
//! tests below (BC-1.18.011 Postcondition 9(e)); the S-25.06 verifier tests cover
//! the rest.
//!
//! Unreachable today (reported, not tested through the binary): the
//! `ManifestExpiredOrAbsent` `EXPIRY_ABORT` arm. `ManifestStatus` is fixed at
//! `StillValid` (no armed-manifest reader exists), so `recover()` never returns
//! `CleanAbortExpiredStaging` from the binary; its exact `Display` is pinned at unit
//! level by `..._EC047_expiry_abort_display_...`.
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

/// ADR-052 v1.23 item 8 / BC-1.18.011 Precondition 6(f)(iv): the EXACT stderr line of
/// the null-generation `EXPIRY_ABORT` arm (`migrate-bc-index: ` + the `NullGeneration`
/// text, no trailing text other than the newline).
const EXPIRY_NULL_GENERATION_LINE: &str = "migrate-bc-index: BC-INDEX migration: pre-generation STAGING record discarded (EXPIRY_ABORT, exit 1); the prior run crashed before any generation was created (generation_id null), nothing was staged, the txn record is ABORTED and the writer gate is OPEN; no canonical path changed; re-activation required";

/// The `ManifestExpiredOrAbsent` text (variant `Display`, i.e. WITHOUT the
/// `migrate-bc-index: ` subcommand prefix).
const EXPIRY_MANIFEST_DISPLAY: &str = "BC-INDEX migration: activation manifest expired or absent at STAGING resume (EXPIRY_ABORT, exit 1); the staged generation was discarded, the txn record is ABORTED and the writer gate is OPEN; no canonical path changed; re-activation required";

/// ADR-052 v1.23 item 9: the EXACT `MIGRATION_LOCK_CONTENTION` line (exit 1).
const LOCK_CONTENTION_LINE: &str = "migrate-bc-index: BC-INDEX migration: another migration coordinator holds the exclusive migration lock (MIGRATION_LOCK_CONTENTION, exit 1); nothing was changed; retry after it exits";

/// ADR-052 v1.23 item 11 / BC-1.18.011 Postcondition 9(e): the EXACT interim line.
const INTERIM_MISMATCH_LINE: &str = "migrate-bc-index: BC-INDEX migration: the terminal record completed.json cannot be proven to describe the live txn (COMPLETION_RECORD_MISMATCH_ABORT, exit 2); no verification was performed in this build; txn and gate unchanged; operator investigation required";

/// ADR-052 v1.23 item 9 / BC-1.18.011 Precondition 6(e): the EXACT
/// `FOREIGN_MIGRATION_REFUSED` line for an id already RENDERED (truncated to 64
/// characters, control characters escaped).
fn foreign_line(rendered_id: &str) -> String {
    format!(
        "migrate-bc-index: BC-INDEX migration: refused: a live migration transaction owned by \
         migration_id \"{rendered_id}\" is in progress (FOREIGN_MIGRATION_REFUSED, exit 2); this \
         subcommand never recovers, finalizes or aborts another migration's record; nothing was \
         changed"
    )
}

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
    fn gate(&self, state: &str) {
        std::fs::write(
            self.ms().join("gate-state.json"),
            serde_json::to_vec(&json!(state)).unwrap(),
        )
        .unwrap();
    }
    /// A valid terminal record `completed.json` (BC-1.18.011 Precondition 6(e)).
    fn completed_json(&self) {
        std::fs::write(
            self.ms().join("completed.json"),
            serde_json::to_vec_pretty(&json!({
                "generation_id": "gen-1",
                "txn_id": "txn-act-1",
                "completed_at": "2026-10-07T00:00:00Z",
                "canonical_paths_count": 1,
            }))
            .unwrap(),
        )
        .unwrap();
    }
    /// Hold `flock(exclusive.lock, LOCK_EX|LOCK_NB)` in THIS process (a separate open
    /// file description, so the spawned coordinator sees EWOULDBLOCK). Released when
    /// the returned file is dropped.
    fn hold_lock(&self) -> std::fs::File {
        let f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(self.ms().join("exclusive.lock"))
            .expect("open exclusive.lock");
        f.try_lock().expect("the test must acquire the free flock");
        f
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

/// Exit code, empty stdout and stderr EXACTLY `<line>\n` (one line, nothing else).
fn check_exact_line(label: &str, out: &Output, code: i32, line: &str, failures: &mut Vec<String>) {
    let err = stderr_of(out);
    if out.status.code() != Some(code) || !out.stdout.is_empty() || err != format!("{line}\n") {
        failures.push(format!(
            "[{label}] expected exit {code}, empty stdout and stderr EXACTLY\n    {line:?}\n  got \
             exit {:?}, stdout {} bytes, stderr\n    {err:?}",
            out.status.code(),
            out.stdout.len()
        ));
    }
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
        // The live known txn must still be recovered: the null-generation discard,
        // exit 1 with EXACTLY the item 8 line -- a terminal record is never rejected
        // for a missing Tier 1 field.
        check_exact_line(label, &out, 1, EXPIRY_NULL_GENERATION_LINE, &mut failures);
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

/// BC-1.18.011 v1.20 EC-046 (e)(f) + Precondition 6(e)/6(f)(ii) + ADR-052 v1.23 item 9:
/// a LIVE record of another known migration (`backfill-append-logs`) or with
/// `migration_id` not in K (`future-migration`) is refused with exit 2
/// `FOREIGN_MIGRATION_REFUSED` and EXACTLY the Precondition 6(e) stderr line (the
/// former "`LockContention`-class" label is retired), `recover()` not run, NO Tier 1
/// field read, nothing mutated -- in particular a foreign STAGING record with
/// `generation_id: null` is NOT discarded, and a minimal foreign record (no Tier 1
/// field at all) is NOT a state-integrity failure. Never `BINARY_INTEGRITY_FAILURE`,
/// never `EXPIRY_ABORT`.
#[test]
fn test_BC_1_18_011_EC046_live_foreign_record_refused_exit_2_no_tier1_read_nothing_mutated_blackbox()
 {
    let mut failures = Vec::new();
    let variants: Vec<(&str, Value, &str)> = vec![
        (
            "(e) backfill-append-logs STAGING, generation_id null",
            with(
                full("STAGING", Value::Null),
                "migration_id",
                json!("backfill-append-logs"),
            ),
            "backfill-append-logs",
        ),
        (
            "(e) backfill-append-logs COMMITTING, generation_id absent",
            with(
                without(full("COMMITTING", Value::Null), "generation_id"),
                "migration_id",
                json!("backfill-append-logs"),
            ),
            "backfill-append-logs",
        ),
        (
            "(f) future-migration STAGING, generation_id null",
            with(
                full("STAGING", Value::Null),
                "migration_id",
                json!("future-migration"),
            ),
            "future-migration",
        ),
        (
            "(f) minimal future-migration STAGING",
            json!({"state": "STAGING", "migration_id": "future-migration"}),
            "future-migration",
        ),
        (
            "(f) minimal future-migration COMMITTING",
            json!({"state": "COMMITTING", "migration_id": "future-migration"}),
            "future-migration",
        ),
        (
            "(e) minimal backfill-append-logs STAGING",
            json!({"state": "STAGING", "migration_id": "backfill-append-logs"}),
            "backfill-append-logs",
        ),
        (
            "(e) minimal backfill-append-logs COMMITTING",
            json!({"state": "COMMITTING", "migration_id": "backfill-append-logs"}),
            "backfill-append-logs",
        ),
    ];
    for (label, rec, rendered_id) in variants {
        let fx = Fx::new();
        fx.txn("txn-act-1.json", &rec);
        let before = snapshot(fx.root());
        let out = fx.run();
        let after = snapshot(fx.root());
        check_exact_line(label, &out, 2, &foreign_line(rendered_id), &mut failures);
        diff_trees(label, &before, &after, &[], &mut failures);
    }
    assert_no_failures(
        "test_BC_1_18_011_EC046_live_foreign_record_refused_exit_2_no_tier1_read_nothing_mutated_blackbox",
        failures,
    );
}

/// BC-1.18.011 v1.20 Precondition 6(e): `<id>` is the record's `migration_id` rendered
/// by the SAME function as the v1.21 admission diagnostic (`sanitize_diagnostic_id`:
/// control characters ESCAPED with `char::escape_default`, the ESCAPED form capped so
/// the result including a trailing `…` marker is at most 64 characters), "so a hostile
/// id cannot forge terminal output". A 100-character id, an id with control characters
/// (newline, ESC, tab) and an id whose ESCAPED form crosses the cap.
#[test]
fn test_BC_1_18_011_EC046_foreign_id_rendering_truncated_to_64_and_control_chars_escaped_blackbox()
{
    let mut failures = Vec::new();
    let long_id = "x".repeat(100);
    let long_rendered = format!("{}…", "x".repeat(63));
    let exact_64 = "y".repeat(64);
    let ctrl_id = "evil\n\u{1b}[31mFAKE\tline".to_string();
    let ctrl_rendered = "evil\\n\\u{1b}[31mFAKE\\tline".to_string();
    // 70 newlines escape to 140 characters; the cap applies to the ESCAPED form:
    // 31 `\n` pairs (62 chars) + the lone `\` (63rd) + `…`.
    let nl_id = "\n".repeat(70);
    let nl_rendered = format!("{}\\…", "\\n".repeat(31));
    let variants: Vec<(&str, String, String)> = vec![
        ("100-char id", long_id, long_rendered),
        (
            "exactly 64 chars is NOT truncated",
            exact_64.clone(),
            exact_64,
        ),
        ("control characters escaped", ctrl_id, ctrl_rendered),
        ("escaped form crosses the cap", nl_id, nl_rendered),
    ];
    for (label, id, rendered) in variants {
        for rec in [
            json!({"state": "STAGING", "migration_id": id}),
            with(
                full("COMMITTING", json!("gen-1")),
                "migration_id",
                json!(id),
            ),
        ] {
            let label = format!("{label} / {}", rec["state"].as_str().unwrap_or("?"));
            let fx = Fx::new();
            fx.txn("txn-act-1.json", &rec);
            let before = snapshot(fx.root());
            let out = fx.run();
            let after = snapshot(fx.root());
            check_exact_line(&label, &out, 2, &foreign_line(&rendered), &mut failures);
            if stderr_of(&out).lines().count() != 1 {
                failures.push(format!(
                    "[{label}] a hostile id must not forge extra stderr lines"
                ));
            }
            diff_trees(&label, &before, &after, &[], &mut failures);
        }
    }
    assert_no_failures(
        "test_BC_1_18_011_EC046_foreign_id_rendering_truncated_to_64_and_control_chars_escaped_blackbox",
        failures,
    );
}

/// BC-1.18.011 v1.20 Precondition 6(f)(ii) precedence (ADR-052 v1.23 item 9): after
/// the flock, (2) a Tier 0 failure of ANY record wins over a foreign refusal; (3) two
/// live records are `multiple_live_txns` integrity, NOT the foreign refusal; a PRESENT
/// non-string `migration_id` is `txn_migration_id_not_string`, not foreign. Nothing
/// mutated in all.
#[test]
fn test_BC_1_18_011_EC046_precedence_tier0_failure_and_one_live_check_win_over_foreign_refusal_blackbox()
 {
    let mut failures = Vec::new();
    let pre = |token: &str| {
        format!("migrate-bc-index: migration admission: state integrity failure ({token}): ")
    };
    let foreign = || {
        with(
            full("STAGING", Value::Null),
            "migration_id",
            json!("backfill-append-logs"),
        )
    };
    // (label, files (name, bytes), expected token)
    let cases: Vec<(&str, Vec<(&str, Vec<u8>)>, &str)> = vec![
        (
            "non-UTF-8 record sorts BEFORE a foreign live record",
            vec![
                ("txn-a-bad.json", vec![0xFF, 0xFE]),
                (
                    "txn-b-foreign.json",
                    serde_json::to_vec(&foreign()).unwrap(),
                ),
            ],
            "txn_record_malformed",
        ),
        (
            "non-UTF-8 record sorts AFTER a foreign live record",
            vec![
                (
                    "txn-a-foreign.json",
                    serde_json::to_vec(&foreign()).unwrap(),
                ),
                ("txn-b-bad.json", vec![0xFF, 0xFE]),
            ],
            "txn_record_malformed",
        ),
        (
            "unknown state beside a foreign live record",
            vec![
                (
                    "txn-a-weird.json",
                    serde_json::to_vec(&json!({"state": "WEIRD"})).unwrap(),
                ),
                (
                    "txn-b-foreign.json",
                    serde_json::to_vec(&foreign()).unwrap(),
                ),
            ],
            "txn_record_malformed",
        ),
        (
            "foreign live + own live => multiple_live_txns",
            vec![
                (
                    "txn-a-foreign.json",
                    serde_json::to_vec(&foreign()).unwrap(),
                ),
                (
                    "txn-b-own.json",
                    serde_json::to_vec(&full("STAGING", json!("gen-1"))).unwrap(),
                ),
            ],
            "multiple_live_txns",
        ),
        (
            "two foreign live records => multiple_live_txns",
            vec![
                (
                    "txn-a-foreign.json",
                    serde_json::to_vec(&foreign()).unwrap(),
                ),
                (
                    "txn-b-foreign.json",
                    serde_json::to_vec(&json!({
                        "state": "COMMITTING", "migration_id": "future-migration"
                    }))
                    .unwrap(),
                ),
            ],
            "multiple_live_txns",
        ),
        (
            "non-string migration_id",
            vec![(
                "txn-a-numid.json",
                serde_json::to_vec(&json!({"state": "STAGING", "migration_id": 7})).unwrap(),
            )],
            "txn_migration_id_not_string",
        ),
    ];
    for (label, files, token) in cases {
        let fx = Fx::new();
        for (name, bytes) in &files {
            fx.txn_bytes(name, bytes);
        }
        let before = snapshot(fx.root());
        let out = fx.run();
        let after = snapshot(fx.root());
        let err = stderr_of(&out);
        let want = pre(token);
        if out.status.code() != Some(2)
            || !out.stdout.is_empty()
            || err.lines().count() != 1
            || !err.starts_with(&want)
            || err.contains("FOREIGN_MIGRATION_REFUSED")
            || err.contains("BINARY_INTEGRITY_FAILURE")
            || err.contains("EXPIRY_ABORT")
        {
            failures.push(format!(
                "[{label}] expected exit 2 and ONE line starting `{want}` (never \
                 FOREIGN_MIGRATION_REFUSED); got exit {:?}, stderr {err:?}",
                out.status.code()
            ));
        }
        diff_trees(label, &before, &after, &[], &mut failures);
    }
    assert_no_failures(
        "test_BC_1_18_011_EC046_precedence_tier0_failure_and_one_live_check_win_over_foreign_refusal_blackbox",
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
/// Both the full record (`source_sha256: null`) and the minimal record. The stderr
/// is EXACTLY the ADR-052 v1.23 item 8 `NullGeneration` line (BC-1.18.011
/// Precondition 6(f)(iv)), not merely "contains EXPIRY_ABORT".
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
        // ADR-052 v1.23 item 8: exit 1 and EXACTLY the NullGeneration line.
        check_exact_line(label, &out, 1, EXPIRY_NULL_GENERATION_LINE, &mut failures);
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

// ===========================================================================
// Item 8 -- EXPIRY_ABORT: exact Display of both arms; write failure is Io exit 2
// ===========================================================================

/// ADR-052 v1.23 item 8 (`ExpiryAbort { arm: ExpiryAbortArm }`): the variant `Display`
/// of BOTH arms is EXACTLY the normative text (the binary prefixes
/// `migrate-bc-index: `). The `ManifestExpiredOrAbsent` arm is unreachable through
/// the binary today (`ManifestStatus` is fixed at `StillValid`), so its line is
/// pinned here at unit level; the `NullGeneration` arm is also pinned through the
/// real binary. Rules (a)/(c): each carries `EXPIRY_ABORT` and `exit 1` and none the
/// digest code, the integrity-failure text, the internal decision identifier or the
/// retired "so the gate self-heals" wording.
#[test]
fn test_BC_1_18_011_EC047_expiry_abort_display_exact_text_for_both_arms_unit() {
    use factory_dispatcher::shard_manager::{BcIndexMigrationError, ExpiryAbortArm};
    let manifest = BcIndexMigrationError::ExpiryAbort {
        arm: ExpiryAbortArm::ManifestExpiredOrAbsent,
    }
    .to_string();
    let null_gen = BcIndexMigrationError::ExpiryAbort {
        arm: ExpiryAbortArm::NullGeneration,
    }
    .to_string();
    assert_eq!(manifest, EXPIRY_MANIFEST_DISPLAY);
    assert_eq!(
        format!("migrate-bc-index: {null_gen}"),
        EXPIRY_NULL_GENERATION_LINE
    );
    for text in [&manifest, &null_gen] {
        for needle in ["EXPIRY_ABORT", "exit 1"] {
            assert!(text.contains(needle), "{needle:?} missing from {text:?}");
        }
        for banned in [
            "BINARY_INTEGRITY_FAILURE",
            "state integrity failure",
            "CleanAbortExpiredStaging",
            "self-heals",
        ] {
            assert!(!text.contains(banned), "{banned:?} present in {text:?}");
        }
    }
}

/// ADR-052 v1.23 item 8(b) / BC-1.18.011 Precondition 6(f)(iv)(b) + test-vector
/// "EC-047 exact stderr and write-failure" (b): the null-generation discard rewrites
/// the txn ABORTED FIRST and the gate OPEN SECOND; when the gate write fails (here
/// `gate-state.json` is a DIRECTORY, so the rename onto it fails -- root-safe) the
/// result is that write's own `Io` error, exit 2, and stderr carries NO `EXPIRY_ABORT`
/// text (the printed past-tense claims would be false). The txn was already ABORTED
/// (txn before gate); the gate is untouched.
#[test]
fn test_BC_1_18_011_EC047_gate_write_failure_after_txn_aborted_is_io_exit_2_never_expiry_abort_blackbox()
 {
    let mut failures = Vec::new();
    for (label, rec) in [
        ("full record", full("STAGING", Value::Null)),
        (
            "minimal record",
            with(minimal("STAGING"), "generation_id", Value::Null),
        ),
    ] {
        let fx = Fx::new();
        fx.txn("txn-act-1.json", &rec);
        let gate_path = fx.ms().join("gate-state.json");
        std::fs::remove_file(&gate_path).unwrap();
        std::fs::create_dir(&gate_path).unwrap();
        let out = fx.run();
        let err = stderr_of(&out);
        let io_prefix = "migrate-bc-index: BC-INDEX migration: I/O error at ";
        if out.status.code() != Some(2)
            || !out.stdout.is_empty()
            || err.lines().count() != 1
            || !err.starts_with(io_prefix)
            || !err.contains("gate-state.json")
            || err.contains("EXPIRY_ABORT")
            || err.contains("BINARY_INTEGRITY_FAILURE")
        {
            failures.push(format!(
                "[{label}] expected exit 2 and ONE `{io_prefix}...gate-state.json...` line with no \
                 EXPIRY_ABORT text (a failed gate write is `Io`, never EXPIRY_ABORT); got exit \
                 {:?}, stderr {err:?}",
                out.status.code()
            ));
        }
        if !gate_path.is_dir() {
            failures.push(format!("[{label}] the unwritable gate must be untouched"));
        }
        let mut want = rec.clone();
        want["state"] = json!("ABORTED");
        want["abort_reason"] = json!("null_generation");
        let got = read_json(&fx.ms().join("txn-act-1.json"));
        if got != want {
            failures.push(format!(
                "[{label}] txn must already be ABORTED with abort_reason (txn is written BEFORE \
                 the gate); want {want}, got {got}"
            ));
        }
    }
    assert_no_failures(
        "test_BC_1_18_011_EC047_gate_write_failure_after_txn_aborted_is_io_exit_2_never_expiry_abort_blackbox",
        failures,
    );
}

/// Restores a directory's permissions on drop (so the tempdir can be cleaned up).
#[cfg(unix)]
struct RestorePerms(PathBuf);
#[cfg(unix)]
impl Drop for RestorePerms {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
    }
}

/// BC-1.18.011 test-vector "EC-047 ... write-failure" (c): the txn write fails (the
/// `migration-state/` directory is made read-only, so the atomic temp-file create
/// fails) => `Io` exit 2, NO `EXPIRY_ABORT` text, nothing mutated (txn still STAGING
/// and not ABORTED, gate unchanged). Skipped when the process can bypass directory
/// permissions (running as root), where the failure cannot be injected.
#[cfg(unix)]
#[test]
fn test_BC_1_18_011_EC047_txn_write_failure_is_io_exit_2_never_expiry_abort_nothing_mutated_blackbox()
 {
    use std::os::unix::fs::PermissionsExt as _;
    let fx = Fx::new();
    fx.txn("txn-act-1.json", &full("STAGING", Value::Null));
    let before = snapshot(fx.root());
    let _restore = RestorePerms(fx.ms());
    std::fs::set_permissions(fx.ms(), std::fs::Permissions::from_mode(0o555)).unwrap();
    if std::fs::write(fx.ms().join("probe"), b"x").is_ok() {
        eprintln!("skipped: permissions are not enforced for this user (root?)");
        return;
    }
    let out = fx.run();
    let err = stderr_of(&out);
    let after = snapshot(fx.root());
    let mut failures = Vec::new();
    if out.status.code() != Some(2)
        || !out.stdout.is_empty()
        || err.lines().count() != 1
        || !err.starts_with("migrate-bc-index: BC-INDEX migration: I/O error at ")
        || !err.contains("txn-act-1.json")
        || err.contains("gate-state.json")
        || err.contains("EXPIRY_ABORT")
        || err.contains("BINARY_INTEGRITY_FAILURE")
    {
        failures.push(format!(
            "expected exit 2, ONE `I/O error at ...txn-act-1.json...` line (the TXN write's own \
             error -- not the gate's, which a swallowed txn failure would reach next), no \
             EXPIRY_ABORT text; got exit {:?}, stderr {err:?}",
            out.status.code()
        ));
    }
    diff_trees("txn write failure", &before, &after, &[], &mut failures);
    assert_no_failures(
        "test_BC_1_18_011_EC047_txn_write_failure_is_io_exit_2_never_expiry_abort_nothing_mutated_blackbox",
        failures,
    );
}

// ===========================================================================
// Item 9 -- MIGRATION_LOCK_CONTENTION (exit 1), flock BEFORE the loader
// ===========================================================================

/// BC-1.18.011 v1.20 EC-048 + Precondition 6(e) flock clause + ADR-052 v1.23 item 9:
/// while ANOTHER coordinator holds `flock(exclusive.lock, LOCK_EX|LOCK_NB)`,
/// `migrate-bc-index` exits **1** with EXACTLY the `MIGRATION_LOCK_CONTENTION` line
/// (never the digest-coded `BINARY_INTEGRITY_FAILURE` text, never exit 2). The flock is
/// checked BEFORE the Tier 0 loader: a MALFORMED txn (non-UTF-8 bytes; unknown state)
/// or a live FOREIGN record beside the held lock reports CONTENTION, not an integrity
/// failure and not the foreign refusal. Variants: no txn; live known txn; live
/// `future-migration`; live `backfill-append-logs`; malformed. Nothing mutated.
#[test]
fn test_BC_1_18_011_EC048_flock_held_by_another_coordinator_is_lock_contention_exit_1_checked_before_loader_blackbox()
 {
    let mut failures = Vec::new();
    let cases: Vec<(&str, Option<Vec<u8>>)> = vec![
        ("(i) no txn record", None),
        (
            "(ii) live known STAGING",
            Some(serde_json::to_vec(&full("STAGING", json!("gen-1"))).unwrap()),
        ),
        (
            "(ii) live known STAGING, generation_id null (must NOT be discarded)",
            Some(serde_json::to_vec(&full("STAGING", Value::Null)).unwrap()),
        ),
        (
            "(iii) live future-migration",
            Some(
                serde_json::to_vec(
                    &json!({"state": "STAGING", "migration_id": "future-migration"}),
                )
                .unwrap(),
            ),
        ),
        (
            "live backfill-append-logs COMMITTING",
            Some(
                serde_json::to_vec(
                    &json!({"state": "COMMITTING", "migration_id": "backfill-append-logs"}),
                )
                .unwrap(),
            ),
        ),
        ("malformed: non-UTF-8 bytes", Some(vec![0xFF, 0xFE])),
        (
            "malformed: unknown state",
            Some(serde_json::to_vec(&json!({"state": "WEIRD"})).unwrap()),
        ),
        (
            "malformed: present non-string migration_id",
            Some(serde_json::to_vec(&json!({"state": "STAGING", "migration_id": 7})).unwrap()),
        ),
    ];
    for (label, bytes) in cases {
        let fx = Fx::new();
        if let Some(b) = &bytes {
            fx.txn_bytes("txn-act-1.json", b);
        }
        let held = fx.hold_lock();
        let before = snapshot(fx.root());
        let out = fx.run();
        let after = snapshot(fx.root());
        drop(held);
        check_exact_line(label, &out, 1, LOCK_CONTENTION_LINE, &mut failures);
        diff_trees(label, &before, &after, &[], &mut failures);
    }
    assert_no_failures(
        "test_BC_1_18_011_EC048_flock_held_by_another_coordinator_is_lock_contention_exit_1_checked_before_loader_blackbox",
        failures,
    );
}

// ===========================================================================
// Item 10 -- strict-presence scope (full 11-key set) and unknown-key rejection
// ===========================================================================

/// BC-1.18.011 v1.20 Precondition 6(f)(iii) + EC-045 wrong-typed vectors (ADR-052
/// v1.23 item 10): at the rewriting arms (`ResumeFromStaging` from STAGING with a
/// string `generation_id` and its gen dir; `ForwardRecovery` from COMMITTING) a
/// record with `fencing_generation` as `"3"`, `3.5`, `-1`, `null` or `2^64`;
/// `pending_canonical_moves` as `null`, an object, a string element, or an element
/// with a missing / ill-typed key; `source_sha256` / `source_body_row_sha256` /
/// `intent_log_path` as a number, array, object or bool; `txn_id` / `activation_id` /
/// `created_at` / `updated_at` as a number => exit 2 `txn_record_malformed` BEFORE the
/// first mutation, tree byte-identical. (A string and JSON `null` are VALID for the
/// three `Option` keys and are deliberately not listed here.)
#[test]
fn test_BC_1_18_011_EC045_wrong_typed_values_across_the_full_key_set_rejected_before_first_mutation_blackbox()
 {
    let mut failures = Vec::new();
    for state in ["STAGING", "COMMITTING"] {
        let base = full(state, json!("gen-1"));
        let mut variants: Vec<(String, Value)> = Vec::new();
        for (what, v) in [
            ("string \"3\"", json!("3")),
            ("float 3.5", json!(3.5)),
            ("negative -1", json!(-1)),
            ("null", Value::Null),
            ("bool", json!(true)),
        ] {
            variants.push((
                format!("{state}: fencing_generation {what}"),
                with(base.clone(), "fencing_generation", v),
            ));
        }
        for (what, v) in [
            ("null", Value::Null),
            ("object", json!({})),
            ("string", json!("moves")),
            ("string element", json!(["not-an-object"])),
            (
                "element missing canonical_path",
                json!([{"staging_path": "s"}]),
            ),
            (
                "element missing staging_path",
                json!([{"canonical_path": "c"}]),
            ),
            (
                "element with a number key",
                json!([{"staging_path": 1, "canonical_path": "c"}]),
            ),
            (
                "second element ill-typed",
                json!([
                    {"staging_path": "s", "canonical_path": "c"},
                    {"staging_path": "s", "canonical_path": null}
                ]),
            ),
        ] {
            variants.push((
                format!("{state}: pending_canonical_moves {what}"),
                with(base.clone(), "pending_canonical_moves", v),
            ));
        }
        for k in ["source_sha256", "source_body_row_sha256", "intent_log_path"] {
            for (what, v) in [
                ("number", json!(7)),
                ("array", json!(["x"])),
                ("object", json!({})),
                ("bool", json!(false)),
            ] {
                variants.push((format!("{state}: {k} {what}"), with(base.clone(), k, v)));
            }
        }
        for k in ["txn_id", "activation_id", "created_at", "updated_at"] {
            variants.push((
                format!("{state}: {k} number"),
                with(base.clone(), k, json!(5)),
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
        // 2^64: does not fit u64 (raw text, serde_json::Value cannot carry it).
        let label = format!("{state}: fencing_generation 18446744073709551616 (2^64)");
        let fx = Fx::new();
        let text = serde_json::to_string_pretty(&base).unwrap().replace(
            "\"fencing_generation\": 1,",
            "\"fencing_generation\": 18446744073709551616,",
        );
        assert!(
            text.contains("18446744073709551616"),
            "fixture replace failed"
        );
        fx.txn_bytes("txn-act-1.json", text.as_bytes());
        fx.gen_dir("gen-1");
        let before = snapshot(fx.root());
        let out = fx.run();
        let after = snapshot(fx.root());
        check_integrity_refusal(&label, &out, &before, &after, &mut failures);
    }
    assert_no_failures(
        "test_BC_1_18_011_EC045_wrong_typed_values_across_the_full_key_set_rejected_before_first_mutation_blackbox",
        failures,
    );
}

/// BC-1.18.011 v1.20 EC-049 + Precondition 6(f)(iii) unknown-top-level-key rule
/// (ADR-052 v1.23 item 10): a live known STAGING (string `generation_id`, gen dir =>
/// `ResumeFromStaging`) or COMMITTING (=> `ForwardRecovery`) record carrying an extra
/// top-level key (`schema_v2_field`, with a number / string / null / object / array
/// value) alongside every one of the 11 required keys => exit 2 `txn_record_malformed`
/// raised at the arm entry BEFORE the first mutation (a typed rewrite cannot preserve a
/// field it does not model). Also with the `migration_id` discriminator present.
#[test]
fn test_BC_1_18_011_EC049_unknown_top_level_key_at_rewriting_arm_exit_2_integrity_nothing_mutated_blackbox()
 {
    let mut failures = Vec::new();
    for state in ["STAGING", "COMMITTING"] {
        for (what, v) in [
            ("number", json!(1)),
            ("string", json!("x")),
            ("null", Value::Null),
            ("object", json!({"a": [1, 2]})),
            ("array", json!([1])),
        ] {
            for without_migration_id in [false, true] {
                let mut rec = with(full(state, json!("gen-1")), "schema_v2_field", v.clone());
                if without_migration_id {
                    rec = without(rec, "migration_id");
                }
                let label = format!(
                    "{state}: unknown key ({what}){}",
                    if without_migration_id {
                        ", migration_id absent"
                    } else {
                        ""
                    }
                );
                let fx = Fx::new();
                fx.txn("txn-act-1.json", &rec);
                fx.gen_dir("gen-1");
                let before = snapshot(fx.root());
                let out = fx.run();
                let after = snapshot(fx.root());
                check_integrity_refusal(&label, &out, &before, &after, &mut failures);
            }
        }
    }
    assert_no_failures(
        "test_BC_1_18_011_EC049_unknown_top_level_key_at_rewriting_arm_exit_2_integrity_nothing_mutated_blackbox",
        failures,
    );
}

/// BC-1.18.011 v1.20 EC-049 control B: a STAGING record with `generation_id: null` AND
/// an unknown key takes the null-generation discard (the `generation_id` tri-state is
/// resolved FIRST and decodes no other key): exit 1 with EXACTLY the `NullGeneration`
/// line, the raw-object rewrite preserves the unknown key AND the present
/// `migration_id` VERBATIM (state ABORTED + `abort_reason`), gate OPEN.
#[test]
fn test_BC_1_18_011_EC049_control_b_null_generation_with_unknown_key_discards_preserving_unknown_key_and_migration_id_blackbox()
 {
    let mut failures = Vec::new();
    let rec = with(
        full("STAGING", Value::Null),
        "schema_v2_field",
        json!({"nested": [1, "two"]}),
    );
    let fx = Fx::new();
    fx.txn("txn-act-1.json", &rec);
    let out = fx.run();
    check_exact_line(
        "control B",
        &out,
        1,
        EXPIRY_NULL_GENERATION_LINE,
        &mut failures,
    );
    let mut want = rec.clone();
    want["state"] = json!("ABORTED");
    want["abort_reason"] = json!("null_generation");
    let got = read_json(&fx.ms().join("txn-act-1.json"));
    if got != want {
        failures.push(format!(
            "control B: the raw-object discard must preserve the unknown key and migration_id \
             verbatim; want {want}, got {got}"
        ));
    }
    if got["migration_id"] != json!(MIGRATION_ID)
        || got["schema_v2_field"] != rec["schema_v2_field"]
    {
        failures.push("control B: migration_id / schema_v2_field not preserved".to_string());
    }
    if read_json(&fx.ms().join("gate-state.json")) != json!("OPEN") {
        failures.push("control B: gate must be OPEN".to_string());
    }
    assert_no_failures(
        "test_BC_1_18_011_EC049_control_b_null_generation_with_unknown_key_discards_preserving_unknown_key_and_migration_id_blackbox",
        failures,
    );
}

/// BC-1.18.011 v1.20 EC-049 control A / Precondition 6(f)(iii) last bullet: a PRESENT
/// `migration_id: "migrate-bc-index"` is "preserved VERBATIM on rewrite" -- including
/// the TYPED rewrites (`write_txn_record`), which today serialize only the modelled
/// `BcIndexMigrationTxnRecord` fields and so DROP the key. Two typed-rewrite paths are
/// reachable with a fully-written (valid) fixture:
/// (1) STAGING, `generation_id: "gen-1"` and a gen dir that is not a valid staged
///     generation => `ResumeFromStaging`, whose resume failure routes through
///     `discard_incomplete_staging` -- a typed rewrite of the record to ABORTED;
/// (2) COMMITTING, `generation_id: "gen-1"`, gen dir present, no pending moves =>
///     `ForwardRecovery` => `finish_committing_migration` -- a typed rewrite to COMPLETED.
/// In both the rewritten record must still carry `"migration_id":"migrate-bc-index"`,
/// and it must really have been rewritten (state changed), so the assertion cannot pass
/// vacuously on an unmodified file.
#[test]
fn test_BC_1_18_011_EC049_control_a_present_migration_id_preserved_verbatim_by_the_typed_rewrite_blackbox()
 {
    let mut failures = Vec::new();
    for (label, state, want_state) in [
        ("ResumeFromStaging -> ABORTED", "STAGING", "ABORTED"),
        ("ForwardRecovery -> COMPLETED", "COMMITTING", "COMPLETED"),
    ] {
        let fx = Fx::new();
        fx.txn("txn-act-1.json", &full(state, json!("gen-1")));
        fx.gen_dir("gen-1");
        let _ = fx.run();
        let got = read_json(&fx.ms().join("txn-act-1.json"));
        if got["state"] != json!(want_state) {
            failures.push(format!(
                "[{label}] fixture precondition: the arm must rewrite the record to {want_state} \
                 (typed rewrite); got {got}"
            ));
        }
        if got["migration_id"] != json!(MIGRATION_ID) {
            failures.push(format!(
                "[{label}] a PRESENT migration_id must be preserved VERBATIM on the typed \
                 rewrite; got {got}"
            ));
        }
    }
    assert_no_failures(
        "test_BC_1_18_011_EC049_control_a_present_migration_id_preserved_verbatim_by_the_typed_rewrite_blackbox",
        failures,
    );
}

// ===========================================================================
// Item 11 / EC-050 -- the INTERIM completed.json short-circuit
// ===========================================================================

/// BC-1.18.011 v1.20 EC-050(b) + Postcondition 9(e)(a): beside a valid `completed.json`,
/// a Tier 0-malformed `txn-*.json` (non-UTF-8 bytes `0xFF 0xFE`; unknown `state`;
/// unparseable text; a non-object) is exit 2 `MIGRATION_STATE_INTEGRITY_FAILURE`
/// (`txn_record_malformed`): the Tier 0 `Err` is NOT swallowed (the former
/// `if let Ok(Some(..))` discarded it) and the gate is NOT opened. Also with a LIVE
/// foreign record beside it (a Tier 0 failure wins over the foreign refusal). Whole
/// tree byte-identical.
#[test]
fn test_BC_1_18_011_EC050_malformed_txn_beside_completed_json_is_integrity_failure_not_swallowed_gate_not_opened_blackbox()
 {
    let mut failures = Vec::new();
    let foreign = serde_json::to_vec(&json!({
        "state": "STAGING", "migration_id": "backfill-append-logs"
    }))
    .unwrap();
    let cases: Vec<(&str, Vec<(&str, Vec<u8>)>)> = vec![
        ("non-UTF-8", vec![("txn-a-bad.json", vec![0xFF, 0xFE])]),
        (
            "unknown state",
            vec![(
                "txn-a-bad.json",
                serde_json::to_vec(&json!({"state": "WEIRD"})).unwrap(),
            )],
        ),
        (
            "unparseable",
            vec![("txn-a-bad.json", b"{not json".to_vec())],
        ),
        ("non-object", vec![("txn-a-bad.json", b"[1,2]".to_vec())]),
        (
            "present non-string migration_id is TxnMigrationIdNotString",
            vec![(
                "txn-a-bad.json",
                serde_json::to_vec(&json!({"state": "COMPLETED", "migration_id": 7})).unwrap(),
            )],
        ),
        (
            "non-UTF-8 beside a live foreign record",
            vec![
                ("txn-a-bad.json", vec![0xFF, 0xFE]),
                ("txn-b-foreign.json", foreign.clone()),
            ],
        ),
        (
            "non-UTF-8 sorting AFTER a live foreign record",
            vec![
                ("txn-a-foreign.json", foreign),
                ("txn-b-bad.json", vec![0xFF, 0xFE]),
            ],
        ),
    ];
    for (label, files) in cases {
        let fx = Fx::new();
        fx.completed_json();
        for (name, bytes) in &files {
            fx.txn_bytes(name, bytes);
        }
        let before = snapshot(fx.root());
        let out = fx.run();
        let after = snapshot(fx.root());
        let err = stderr_of(&out);
        let ok_prefix = err.starts_with(
            "migrate-bc-index: migration admission: state integrity failure (txn_record_malformed): ",
        ) || err.starts_with(
            "migrate-bc-index: migration admission: state integrity failure \
             (txn_migration_id_not_string): ",
        );
        if out.status.code() != Some(2)
            || !out.stdout.is_empty()
            || err.lines().count() != 1
            || !ok_prefix
            || err.contains("BINARY_INTEGRITY_FAILURE")
            || err.contains("FOREIGN_MIGRATION_REFUSED")
        {
            failures.push(format!(
                "[{label}] expected exit 2 and ONE MIGRATION_STATE_INTEGRITY_FAILURE line (the \
                 Tier 0 error is not swallowed); got exit {:?}, stderr {err:?}",
                out.status.code()
            ));
        }
        diff_trees(label, &before, &after, &[], &mut failures);
        if read_json(&fx.ms().join("gate-state.json")) == json!("OPEN") {
            failures.push(format!("[{label}] the gate must NOT be flipped to OPEN"));
        }
    }
    assert_no_failures(
        "test_BC_1_18_011_EC050_malformed_txn_beside_completed_json_is_integrity_failure_not_swallowed_gate_not_opened_blackbox",
        failures,
    );
}

/// BC-1.18.011 v1.20 EC-050(a) + Postcondition 9(e)(b): a LIVE record beside a valid
/// `completed.json` is NEVER finalized and the gate is NEVER flipped. A foreign record
/// (`backfill-append-logs`, `future-migration`, STAGING and COMMITTING; minimal and full)
/// => exit 2 `FOREIGN_MIGRATION_REFUSED` with the exact Precondition 6(e) line; the
/// OWN migration's live STAGING or COMMITTING record (minimal and full; the gate
/// DRAINING / LOCKED) => exit 2 `COMPLETION_RECORD_MISMATCH_ABORT` with EXACTLY the
/// interim line. In every case the txn is not rewritten to COMPLETED, the gate is not
/// OPEN, and `migration-state/` is byte-identical.
#[test]
fn test_BC_1_18_011_EC050_live_record_beside_completed_json_is_never_finalized_foreign_refused_own_mismatch_abort_blackbox()
 {
    let mut failures = Vec::new();
    let mut cases: Vec<(String, Value, String, &str)> = Vec::new();
    for state in ["STAGING", "COMMITTING"] {
        for foreign_id in ["backfill-append-logs", "future-migration"] {
            cases.push((
                format!("foreign {foreign_id} {state} (minimal)"),
                json!({"state": state, "migration_id": foreign_id}),
                foreign_line(foreign_id),
                "foreign",
            ));
            cases.push((
                format!("foreign {foreign_id} {state} (full)"),
                with(
                    full(state, json!("gen-1")),
                    "migration_id",
                    json!(foreign_id),
                ),
                foreign_line(foreign_id),
                "foreign",
            ));
        }
        cases.push((
            format!("own {state} (full, string generation_id)"),
            full(state, json!("gen-1")),
            INTERIM_MISMATCH_LINE.to_string(),
            "own",
        ));
        cases.push((
            format!("own {state} (minimal)"),
            minimal(state),
            INTERIM_MISMATCH_LINE.to_string(),
            "own",
        ));
        cases.push((
            format!("own {state} (migration_id absent)"),
            without(full(state, json!("gen-1")), "migration_id"),
            INTERIM_MISMATCH_LINE.to_string(),
            "own",
        ));
    }
    cases.push((
        "own STAGING (generation_id null: NOT discarded here)".to_string(),
        full("STAGING", Value::Null),
        INTERIM_MISMATCH_LINE.to_string(),
        "own",
    ));
    for (label, rec, line, _kind) in cases {
        for gate in ["DRAINING", "LOCKED"] {
            let label = format!("{label}, gate {gate}");
            let fx = Fx::new();
            fx.gate(gate);
            fx.completed_json();
            fx.txn("txn-act-1.json", &rec);
            let before = snapshot(fx.root());
            let out = fx.run();
            let after = snapshot(fx.root());
            check_exact_line(&label, &out, 2, &line, &mut failures);
            diff_trees(&label, &before, &after, &[], &mut failures);
            let txn = read_json(&fx.ms().join("txn-act-1.json"));
            if txn["state"] != rec["state"] {
                failures.push(format!(
                    "[{label}] the live txn must never be finalized (state {} -> {})",
                    rec["state"], txn["state"]
                ));
            }
            if read_json(&fx.ms().join("gate-state.json")) != json!(gate) {
                failures.push(format!("[{label}] the gate must not be flipped"));
            }
        }
    }
    assert_no_failures(
        "test_BC_1_18_011_EC050_live_record_beside_completed_json_is_never_finalized_foreign_refused_own_mismatch_abort_blackbox",
        failures,
    );
}

/// BC-1.18.011 v1.20 EC-050(c1) + Postcondition 9(e)(c): with NO live record beside a
/// valid `completed.json` and the gate LOCKED or DRAINING, the gate is reconciled to
/// OPEN under the non-blocking flock (the flock is free), exit 0. Terminal
/// (COMPLETED / ABORTED, own and foreign) records beside it are never modified: with no
/// live record the short-circuit reconciles the GATE only (Precondition 6(f)(ii): a
/// terminal record is never modified by recovery).
#[test]
fn test_BC_1_18_011_EC050_no_live_record_gate_reconciled_to_open_under_flock_terminal_records_untouched_blackbox()
 {
    let mut failures = Vec::new();
    let terminals: Vec<(&str, Vec<(&str, Value)>)> = vec![
        ("no txn record", vec![]),
        (
            "own COMPLETED",
            vec![("txn-act-1.json", full("COMPLETED", json!("gen-1")))],
        ),
        (
            "own ABORTED",
            vec![("txn-act-1.json", full("ABORTED", json!("gen-1")))],
        ),
        (
            "minimal ABORTED + foreign COMPLETED",
            vec![
                ("txn-a-1.json", json!({"state": "ABORTED"})),
                (
                    "txn-a-2.json",
                    json!({"state": "COMPLETED", "migration_id": "future-migration"}),
                ),
            ],
        ),
    ];
    for gate in ["LOCKED", "DRAINING"] {
        for (label, recs) in &terminals {
            let label = format!("gate {gate}, {label}");
            let fx = Fx::new();
            fx.gate(gate);
            fx.completed_json();
            for (name, v) in recs {
                fx.txn(name, v);
            }
            let before = snapshot(fx.root());
            let out = fx.run();
            let after = snapshot(fx.root());
            if out.status.code() != Some(0) {
                failures.push(format!(
                    "[{label}] expected exit 0 (gate reconciled); got {:?}, stderr {:?}",
                    out.status.code(),
                    stderr_of(&out)
                ));
            }
            if read_json(&fx.ms().join("gate-state.json")) != json!("OPEN") {
                failures.push(format!("[{label}] the gate must be reconciled to OPEN"));
            }
            diff_trees(&label, &before, &after, &["gate-state.json"], &mut failures);
        }
    }
    assert_no_failures(
        "test_BC_1_18_011_EC050_no_live_record_gate_reconciled_to_open_under_flock_terminal_records_untouched_blackbox",
        failures,
    );
}

fn mtime_of(p: &Path) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

/// BC-1.18.011 v1.20 EC-050(c2) + Postcondition 9(e)(c): clean steady state = ZERO
/// filesystem writes. A valid `completed.json`, no live record and the gate already
/// OPEN (and, separately, `gate-state.json` ABSENT, which reads as OPEN) => exit 0, no
/// write at all: the tree is byte-identical, `gate-state.json` is not re-created, and
/// its mtime (pinned to 2001) is unchanged -- the former unconditional
/// `write_admission_gate_state(.., Open)` rewrote it every run.
#[test]
fn test_BC_1_18_011_EC050_gate_already_open_is_zero_filesystem_writes_blackbox() {
    let mut failures = Vec::new();
    for absent in [false, true] {
        let label = if absent {
            "gate-state.json ABSENT"
        } else {
            "gate already OPEN"
        };
        let fx = Fx::new();
        fx.completed_json();
        let gate_path = fx.ms().join("gate-state.json");
        if absent {
            std::fs::remove_file(&gate_path).unwrap();
        } else {
            fx.gate("OPEN");
            filetime::set_file_mtime(
                &gate_path,
                filetime::FileTime::from_unix_time(1_000_000_000, 0),
            )
            .unwrap();
        }
        let before = snapshot(fx.root());
        let mtime_before = mtime_of(&gate_path);
        let out = fx.run();
        let after = snapshot(fx.root());
        if out.status.code() != Some(0) {
            failures.push(format!(
                "[{label}] expected exit 0; got {:?}, stderr {:?}",
                out.status.code(),
                stderr_of(&out)
            ));
        }
        diff_trees(label, &before, &after, &[], &mut failures);
        if mtime_of(&gate_path) != mtime_before {
            failures.push(format!(
                "[{label}] gate-state.json must not be rewritten (mtime changed): clean steady \
                 state is ZERO writes"
            ));
        }
    }
    assert_no_failures(
        "test_BC_1_18_011_EC050_gate_already_open_is_zero_filesystem_writes_blackbox",
        failures,
    );
}

/// BC-1.18.011 v1.20 EC-050(c3) + Postcondition 9(e)(c) / ADR-052 v1.23 item 11(c): with
/// NO live record beside a valid `completed.json` and the flock HELD by another
/// coordinator (EWOULDBLOCK), the reconciliation is skipped: exit 0, NO gate write --
/// the gate of a live `backfill-append-logs` coordinator (DRAINING / LOCKED) is never
/// opened by this binary (the section 7e D4 hazard). Tree byte-identical.
#[test]
fn test_BC_1_18_011_EC050_flock_held_elsewhere_skips_gate_reconciliation_exit_0_no_write_blackbox()
{
    let mut failures = Vec::new();
    for gate in ["LOCKED", "DRAINING"] {
        let label = format!("gate {gate}, flock held");
        let fx = Fx::new();
        fx.gate(gate);
        fx.completed_json();
        let held = fx.hold_lock();
        let before = snapshot(fx.root());
        let out = fx.run();
        let after = snapshot(fx.root());
        drop(held);
        if out.status.code() != Some(0) {
            failures.push(format!(
                "[{label}] expected exit 0 (skip); got {:?}, stderr {:?}",
                out.status.code(),
                stderr_of(&out)
            ));
        }
        if read_json(&fx.ms().join("gate-state.json")) != json!(gate) {
            failures.push(format!(
                "[{label}] the gate of the live coordinator must NOT be opened by this binary"
            ));
        }
        diff_trees(&label, &before, &after, &[], &mut failures);
    }
    assert_no_failures(
        "test_BC_1_18_011_EC050_flock_held_elsewhere_skips_gate_reconciliation_exit_0_no_write_blackbox",
        failures,
    );
}
