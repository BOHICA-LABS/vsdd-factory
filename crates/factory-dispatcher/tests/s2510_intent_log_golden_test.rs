// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-25.10 RED tests -- the ADR-054 Decision 1.4 GOLDEN RECORD and the Decision 1.5
//! operator `sed` + `shasum` recipe (red test T10).
//!
//! Authority: ADR-054 v1.0 Decision 1.4 (byte-exact checksum, golden vector), 1.5 (operator
//! recipe); BC-1.18.011 v1.21 Postcondition 15(d) and its canonical test-vector row; story
//! S-25.10 AC-002, AC-003.
//!
//! | Story AC / red test | Test |
//! |---|---|
//! | AC-002 / T10 | `..._15d_golden_intent_and_done_records_are_byte_identical_to_the_adr_vector` |
//! | AC-002 / T10 | `..._15d_checksum_hex_of_the_first_nine_golden_lines_equals_the_adr_vector` |
//! | AC-002 | `..._15d_the_vector_embedded_in_this_suite_equals_adr_054_when_the_adr_is_reachable` |
//! | AC-003 / T10 | `..._15d_operator_sed_shasum_recipe_reproduces_every_checksum_of_a_four_record_log` |
//!
//! The golden bytes are embedded VERBATIM from ADR-054 Decision 1.4 (`s2510_support`), and
//! their checksums were independently recomputed with the very recipe of Decision 1.5 before
//! this suite was written (INTENT `2767fd1d...e02b`, DONE `3382c172...c5c1`, both equal to
//! the ADR). The ADR file lives on the `factory-artifacts` branch and is absent from a bare
//! checkout of this branch, so the third test compares the embedded vector with the ADR text
//! WHEN it is reachable from a parent directory (the developer worktree layout) and reports
//! that it was skipped otherwise; it never weakens the two byte-exact tests, which need no ADR.
//!
//! The module under test does not exist yet: this file fails to COMPILE (the accepted Red).

#[path = "s2510_support/api.rs"]
mod api;
#[path = "s2510_support/mod.rs"]
mod support;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use api::{IntentLogWriter, checksum_hex, encode_record, to_intent};
use factory_dispatcher::shard_manager::BcIndexMigrationError;
use factory_dispatcher::shard_manager::migration_fs::StdFs;
use support::{
    GOLDEN_DONE, GOLDEN_DONE_CHECKSUM, GOLDEN_INTENT, GOLDEN_INTENT_CHECKSUM, GOLDEN_TXN_ID, Rec,
    assert_no_failures, finish, sha256_hex,
};

/// The first nine LF-terminated lines of an 11-line record.
fn first_nine_lines(record: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for l in record.split_inclusive('\n').take(9) {
        out.extend_from_slice(l.as_bytes());
    }
    out
}

// ===========================================================================
// AC-002 / T10
// ===========================================================================

/// ADR-054 1.4: `encode_record` of the golden INTENT and DONE inputs is BYTE-IDENTICAL to
/// the ADR vector (11 LF-terminated lines, fixed key order), and the `record_checksum`
/// values equal the vector's.
#[test]
fn test_BC_1_18_011_15d_golden_intent_and_done_records_are_byte_identical_to_the_adr_vector() {
    let mut failures = Vec::new();
    for (label, rec, want, want_sum) in [
        (
            "INTENT",
            Rec::golden_intent(),
            GOLDEN_INTENT,
            GOLDEN_INTENT_CHECKSUM,
        ),
        (
            "DONE",
            Rec::golden_done(),
            GOLDEN_DONE,
            GOLDEN_DONE_CHECKSUM,
        ),
    ] {
        match encode_record(&to_intent(&rec)) {
            Ok(bytes) => {
                if bytes != want.as_bytes() {
                    failures.push(format!(
                        "[{label}] encode_record is not byte-identical to the ADR-054 1.4 vector:\n--- got ---\n{}\n--- want ---\n{want}",
                        String::from_utf8_lossy(&bytes)
                    ));
                }
                let text = String::from_utf8_lossy(&bytes).to_string();
                if text.lines().count() != 11 {
                    failures.push(format!(
                        "[{label}] must be exactly 11 lines, got {}",
                        text.lines().count()
                    ));
                }
                if !text.contains(&format!("\nrecord_checksum={want_sum}\n")) {
                    failures.push(format!(
                        "[{label}] record_checksum must equal the vector {want_sum}"
                    ));
                }
                if text.contains('\r') || !text.ends_with("END_INTENT_LOG_RECORD\n") {
                    failures.push(format!(
                        "[{label}] LF-only lines ending in END_INTENT_LOG_RECORD\\n"
                    ));
                }
            }
            Err(e) => failures.push(format!(
                "[{label}] the golden record is valid and must encode: {e}"
            )),
        }
    }
    assert_no_failures("golden records", failures);
}

/// ADR-054 1.4: the checksum is the lowercase-hex SHA-256 of the exact first nine lines.
/// `checksum_hex` over those bytes equals the vector, and so does an independent SHA-256
/// (guards the vector itself against a transcription error).
#[test]
fn test_BC_1_18_011_15d_checksum_hex_of_the_first_nine_golden_lines_equals_the_adr_vector() {
    let mut failures = Vec::new();
    for (label, record, want) in [
        ("INTENT", GOLDEN_INTENT, GOLDEN_INTENT_CHECKSUM),
        ("DONE", GOLDEN_DONE, GOLDEN_DONE_CHECKSUM),
    ] {
        let nine = first_nine_lines(record);
        if sha256_hex(&nine) != want {
            failures.push(format!(
                "[{label}] independent SHA-256 of the nine lines != vector"
            ));
        }
        let got = checksum_hex(&nine);
        if got != want {
            failures.push(format!("[{label}] checksum_hex -> {got}, vector {want}"));
        }
        if got.len() != 64 || got.chars().any(|c| c.is_ascii_uppercase()) {
            failures.push(format!(
                "[{label}] checksum must be 64 lowercase hex digits: {got}"
            ));
        }
    }
    // The checksum EXCLUDES the record_checksum and END lines: changing either must not
    // change `checksum_hex` of the nine lines, and the whole 11-line text hashes differently.
    if checksum_hex(GOLDEN_INTENT.as_bytes()) == GOLDEN_INTENT_CHECKSUM {
        failures
            .push("the checksum must cover ONLY the first nine lines, not the whole record".into());
    }
    assert_no_failures("golden checksums", failures);
}

/// When the ADR file is reachable, the vector embedded in this suite must equal the ADR's
/// text line for line and the ADR must state both checksums (a transcription guard).
#[test]
fn test_BC_1_18_011_15d_the_vector_embedded_in_this_suite_equals_adr_054_when_the_adr_is_reachable()
{
    const ADR: &str = ".factory/specs/architecture/decisions/ADR-054-governed-migration-intent-log-format-fixed-move-plan-and-crash-recovery.md";
    let mut dir: Option<PathBuf> = Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    let mut found: Option<PathBuf> = None;
    while let Some(d) = dir {
        let candidate = d.join(ADR);
        if candidate.is_file() {
            found = Some(candidate);
            break;
        }
        dir = d.parent().map(Path::to_path_buf);
    }
    let Some(adr_path) = found else {
        eprintln!(
            "SKIPPED (ADR-054 not reachable from this checkout): the byte-exact tests above are the gate"
        );
        return;
    };
    let adr = std::fs::read_to_string(&adr_path).unwrap();
    let mut failures = Vec::new();
    for line in GOLDEN_INTENT.lines() {
        if !adr.contains(&format!("\n{line}\n")) {
            failures.push(format!(
                "golden INTENT line not found verbatim in ADR-054: {line:?}"
            ));
        }
    }
    for needle in [GOLDEN_INTENT_CHECKSUM, GOLDEN_DONE_CHECKSUM, GOLDEN_TXN_ID] {
        if !adr.contains(needle) {
            failures.push(format!("ADR-054 does not contain {needle}"));
        }
    }
    assert_no_failures("golden vector vs ADR-054", failures);
}

// ===========================================================================
// AC-003 / T10 -- operator recipe
// ===========================================================================

fn shell(script: &str, args: &[&str]) -> String {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(script).arg("sh").args(args);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let out = finish(cmd.spawn().expect("spawn sh"), 30);
    assert!(
        out.status.success(),
        "shell script failed: {script}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// ADR-054 1.5: a four-record log written by the module's writer; for EVERY record
/// (`S` = 1, 12, 23, 34: records occupy lines `S..S+10`) the operator recipe
/// `sed -n "S,S+8p" <log> | shasum -a 256` reproduces the stored checksum, line `S+9`.
/// A clean log has `wc -l` a multiple of 11 and ends with `END_INTENT_LOG_RECORD`.
#[test]
fn test_BC_1_18_011_15d_operator_sed_shasum_recipe_reproduces_every_checksum_of_a_four_record_log()
{
    let i0 = Rec::golden_intent();
    let i1 = Rec {
        target: "/proj/.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-01.md".into(),
        staging: "/proj/.factory/migration-state/gen-7c1d2e3f-4a5b-4c6d-8e7f-9a0b1c2d3e4f/shards/BC-INDEX-SS-01.md".into(),
        pre: "MISSING".into(),
        post: sha256_hex(b"alpha-shard"),
        ..Rec::golden_intent()
    };
    let d0 = Rec::golden_done();
    let d1 = i1.done_of(2, "2026-10-08T12:00:08Z");
    let recs = [i0, i1, d0, d1];

    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("intent-gen-7c1d2e3f.log");
    let outcome = (|| -> Result<(), BcIndexMigrationError> {
        let mut w = IntentLogWriter::open(&StdFs, &log, GOLDEN_TXN_ID)?;
        // Two batches, as the coordinator writes them: the INTENT batch, then each DONE.
        w.append_batch(&[to_intent(&recs[0]), to_intent(&recs[1])])?;
        w.append_batch(&[to_intent(&recs[2])])?;
        w.append_batch(&[to_intent(&recs[3])])
    })();
    outcome.expect("writing the four-record log must succeed");

    let logp = log.to_string_lossy().to_string();
    let mut failures = Vec::new();

    let lines = shell("wc -l < \"$1\"", &[&logp]);
    if lines.parse::<usize>().ok() != Some(44) {
        failures.push(format!(
            "`wc -l` must be 44 (four 11-line records), got {lines:?}"
        ));
    }
    let last = shell("tail -n 1 \"$1\"", &[&logp]);
    if last != "END_INTENT_LOG_RECORD" {
        failures.push(format!(
            "the last line must be END_INTENT_LOG_RECORD, got {last:?}"
        ));
    }

    // `shasum` ships with macOS and perl-based Linux images; fall back to `sha256sum`.
    let hasher = if shell("command -v shasum || true", &[]).is_empty() {
        "sha256sum"
    } else {
        "shasum -a 256"
    };
    for (k, s) in [1usize, 12, 23, 34].into_iter().enumerate() {
        let recomputed = shell(
            &format!("sed -n \"${{2}},$((${{2}}+8))p\" \"$1\" | {hasher} | cut -d' ' -f1"),
            &[&logp, &s.to_string()],
        );
        let stored = shell(
            "sed -n \"$(($2+9))p\" \"$1\" | cut -d= -f2",
            &[&logp, &s.to_string()],
        );
        if recomputed != stored {
            failures.push(format!(
                "record {k} (S={s}): `sed | {hasher}` gave {recomputed}, the stored line S+9 is {stored}"
            ));
        }
        if k == 0 && stored != GOLDEN_INTENT_CHECKSUM {
            failures.push(format!(
                "record 0 is the golden INTENT: stored {stored} != {GOLDEN_INTENT_CHECKSUM}"
            ));
        }
        if k == 2 && stored != GOLDEN_DONE_CHECKSUM {
            failures.push(format!(
                "record 2 is the golden DONE: stored {stored} != {GOLDEN_DONE_CHECKSUM}"
            ));
        }
    }
    assert_no_failures("operator sed/shasum recipe", failures);
}
