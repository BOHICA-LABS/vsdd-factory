// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-25.10 RED tests -- the byte-level, line-anchored intent-log READER and the tail-repair
//! half of the WRITER (module level; the real-binary coordinator tests are in
//! `s2510_coordinator_blackbox_test.rs`).
//!
//! Authority: ADR-054 v1.0 Decision 1.2 (grammar, value rules), 1.7 (reader algorithm,
//! properties (i)-(vi)), 1.8 (log invariants L1-L4), 1.9 steps 2-5; BC-1.18.011 v1.21
//! Postcondition 15(b)(c), EC-065, EC-066, EC-068; story S-25.10 AC-006, AC-007, AC-008.
//!
//! | Story AC / red test | Test |
//! |---|---|
//! | AC-006 / T1 (golden log, EVERY byte offset, exhaustive) | `..._EC065_truncation_at_every_byte_offset_of_the_golden_log_is_exhaustive` |
//! | AC-006 / T1 (three-record log, EVERY byte offset, exhaustive) | `..._EC065_truncation_at_every_byte_offset_of_a_three_record_log_is_exhaustive` |
//! | AC-006 / T1 (property test, 2000 random logs) | `..._EC065_truncation_at_a_random_offset_with_random_tail_yields_the_written_prefix_proptest` |
//! | AC-006 / T3 | `..._EC065_garbage_nul_partial_marker_partial_end_and_invalid_utf8_tails_yield_the_prefix` |
//! | AC-006 / A-1 | `..._EC066_valid_prefix_then_garbage_then_a_valid_record_is_mid_log_corruption_not_silently_dropped` |
//! | AC-007 / T5 | `..._15b_grammar_strictness_each_violation_makes_the_record_invalid` |
//! | AC-007 / T5 | `..._15b_value_rules_are_enforced_by_the_reader_even_when_the_checksum_matches` |
//! | AC-007 / T15 | `..._EC068_log_invariants_l1_to_l4_fail_closed_with_first_violating_offset` |
//! | AC-007 / T15 | `..._EC068_identical_duplicate_intents_are_accepted` |
//! | AC-008 / T2 | `..._EC065_torn_tail_is_truncated_durably_before_the_first_append_and_r1_r3_survive` |
//! | AC-008 | `..._EC065_open_on_mid_log_corruption_or_invariant_violation_mutates_nothing` |
//! | AC-008 | `..._EC065_open_never_creates_the_file_and_a_clean_log_is_not_truncated` |
//!
//! Every byte sequence below is built by the INDEPENDENT reference encoder of
//! `s2510_support/mod.rs` (SHA-256 over the exact first nine lines, ADR-054 Decision 1.4),
//! never by the module under test, so a defect in `encode_record` cannot mask a reader
//! defect. The module under test does not exist yet: this file fails to COMPILE
//! (unresolved `shard_manager::intent_log`), which is the accepted Red.

#[path = "s2510_support/api.rs"]
mod api;
#[path = "s2510_support/mod.rs"]
mod support;

use api::{IntentLogWriter, Invariant, LogReadError, read_log, to_intent, to_intents};
use factory_dispatcher::shard_manager::BcIndexMigrationError;
use factory_dispatcher::shard_manager::migration_fs::StdFs;
use proptest::prelude::*;
use support::{
    GOLDEN_DONE, GOLDEN_INTENT, GOLDEN_TXN_ID, Rec, TXN, assemble, assert_no_failures, concat,
    encode, ends, nine_lines, sha256_hex,
};

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// For EVERY cut offset of `log`, the reader must return Ok with exactly the records whose
/// encoding ends at or before the cut and `valid_prefix_len` = the end of the last of them.
fn check_every_cut(
    label: &str,
    log: &[u8],
    parts: &[Vec<u8>],
    recs: &[Rec],
    txn: &str,
) -> Vec<String> {
    let boundaries = ends(parts);
    let mut failures = Vec::new();
    for cut in 0..=log.len() {
        let complete = boundaries.iter().filter(|&&e| e <= cut).count();
        let want_len = if complete == 0 {
            0
        } else {
            boundaries[complete - 1]
        };
        match read_log(&log[..cut], txn) {
            Ok(lr) => {
                if lr.records != to_intents(&recs[..complete]) || lr.valid_prefix_len != want_len {
                    failures.push(format!(
                        "[{label}] cut {cut}/{}: expected {complete} record(s) and \
                         valid_prefix_len {want_len}, got {} record(s) and valid_prefix_len {}",
                        log.len(),
                        lr.records.len(),
                        lr.valid_prefix_len
                    ));
                }
            }
            Err(e) => failures.push(format!(
                "[{label}] cut {cut}/{}: a truncation must NEVER be an error (a torn tail is \
                 absent, ADR-054 1.7 (iii)); got {e:?}",
                log.len()
            )),
        }
    }
    failures
}

fn three_records() -> Vec<Rec> {
    let i0 = Rec::sample(TXN, 0, "INTENT", 1);
    let i1 = Rec::sample(TXN, 1, "INTENT", 1);
    let d0 = i0.done_of(2, "2026-10-08T12:00:09Z");
    vec![i0, i1, d0]
}

fn tempdir_log() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("intent-gen-1.log");
    (dir, log)
}

fn mid_log_err(r: Result<api::LogRead, LogReadError>) -> Option<(usize, usize)> {
    match r {
        Err(LogReadError::MidLogCorruption {
            first_bad_offset,
            later_valid_offset,
        }) => Some((first_bad_offset, later_valid_offset)),
        _ => None,
    }
}

// ===========================================================================
// AC-006 / T1 -- truncation at EVERY byte offset (exhaustive over the golden log)
// ===========================================================================

/// BC-1.18.011 EC-065 / ADR-054 1.7 (i)(ii)(iii): truncating the ADR-054 GOLDEN log
/// (INTENT 579 bytes + DONE 577 bytes) at EVERY byte offset 0..=len yields exactly the fully
/// written prefix -- never an error, never a partial record. Exhaustive, not sampled.
#[test]
fn test_BC_1_18_011_EC065_truncation_at_every_byte_offset_of_the_golden_log_is_exhaustive() {
    let parts = vec![
        GOLDEN_INTENT.as_bytes().to_vec(),
        GOLDEN_DONE.as_bytes().to_vec(),
    ];
    let log = concat(&parts);
    assert_eq!(
        log.len(),
        1156,
        "fixture sanity: the 579-byte golden INTENT and the 577-byte golden DONE"
    );
    // The golden bytes are the ADR vector, and the reference encoder reproduces them
    // (guards the oracle itself, independent of the module under test).
    assert_eq!(encode(&Rec::golden_intent()), parts[0]);
    assert_eq!(encode(&Rec::golden_done()), parts[1]);
    let recs = vec![Rec::golden_intent(), Rec::golden_done()];
    let failures = check_every_cut("golden", &log, &parts, &recs, GOLDEN_TXN_ID);
    assert_no_failures("golden-log truncation at every offset", failures);
}

/// Same exhaustive property over a three-record log (INTENT, INTENT, DONE) so a cut can
/// fall in the middle record as well as the first and last.
#[test]
fn test_BC_1_18_011_EC065_truncation_at_every_byte_offset_of_a_three_record_log_is_exhaustive() {
    let recs = three_records();
    let parts: Vec<Vec<u8>> = recs.iter().map(encode).collect();
    let log = concat(&parts);
    let failures = check_every_cut("three-record", &log, &parts, &recs, TXN);
    assert_no_failures("three-record truncation at every offset", failures);
}

prop_compose! {
    fn arb_rec(idx: usize)(
        path in "/[a-zA-Z0-9_.=|-][a-zA-Z0-9_.=| -]{0,58}[a-zA-Z0-9_.=|-]",
        spath in "/[a-zA-Z0-9_.=|-][a-zA-Z0-9_.=| -]{0,58}[a-zA-Z0-9_.=|-]",
        post in "[0-9a-f]{64}",
        pre in prop_oneof![Just("MISSING".to_string()), "[0-9a-f]{64}"],
        yy in 0u32..100, mm in 1u32..=12, dd in 1u32..=31,
        hh in 0u32..24, mi in 0u32..60, ss in 0u32..60,
    ) -> Rec {
        Rec {
            txn_id: "t-1.x_y".into(),
            fencing: 1 + idx as u64,
            rtype: "INTENT".into(),
            target: format!("/p{idx}{path}"),
            staging: format!("/s{idx}{spath}"),
            post,
            pre,
            ts: format!("20{yy:02}-{mm:02}-{dd:02}T{hh:02}:{mi:02}:{ss:02}Z"),
        }
    }
}

fn arb_log() -> impl Strategy<Value = Vec<Rec>> {
    (1usize..=5).prop_flat_map(|n| (0..n).map(arb_rec).collect::<Vec<_>>())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    /// ADR-054 1.7 properties (i)-(vi), property-test form (>= 1000 cases): random valid logs
    /// of 1-5 records, a random cut, optional random garbage appended after the cut. The
    /// reader returns exactly the records fully written before the cut -- Ok, never an error,
    /// never a partial record -- and `valid_prefix_len` is the end of the last of them.
    #[test]
    fn test_BC_1_18_011_EC065_truncation_at_a_random_offset_with_random_tail_yields_the_written_prefix_proptest(
        recs in arb_log(),
        frac in 0.0f64..=1.0,
        garbage in prop::collection::vec(any::<u8>(), 0..64),
    ) {
        let parts: Vec<Vec<u8>> = recs.iter().map(encode).collect();
        let log = concat(&parts);
        let cut = ((log.len() as f64) * frac) as usize;
        let cut = cut.min(log.len());
        let boundaries = ends(&parts);
        let complete = boundaries.iter().filter(|&&e| e <= cut).count();
        let want_len = if complete == 0 { 0 } else { boundaries[complete - 1] };

        let mut bytes = log[..cut].to_vec();
        bytes.extend_from_slice(&garbage);
        let got = read_log(&bytes, "t-1.x_y");
        prop_assert!(got.is_ok(), "truncation + garbage must never be an error: {got:?}");
        let lr = got.unwrap();
        prop_assert_eq!(lr.records, to_intents(&recs[..complete]));
        prop_assert_eq!(lr.valid_prefix_len, want_len);
    }
}

// ===========================================================================
// AC-006 / T3 -- torn tails of every kind are ABSENT (no error, prefix returned)
// ===========================================================================

/// BC-1.18.011 EC-065: a tail of garbage, 512 NUL bytes, a partial `INTENT_LOG_RECORD_V1`
/// start marker, a partial END line, an unterminated record, an invalid UTF-8 byte, or a
/// self-consistent record whose path is not UTF-8 yields the valid prefix; the earlier
/// records are returned and there is NO error (the old reader failed the whole file on one
/// invalid UTF-8 byte -- A-2).
#[test]
fn test_BC_1_18_011_EC065_garbage_nul_partial_marker_partial_end_and_invalid_utf8_tails_yield_the_prefix()
 {
    let r1 = Rec::sample(TXN, 0, "INTENT", 1);
    let r2 = Rec::sample(TXN, 1, "INTENT", 1);
    let r3 = Rec::sample(TXN, 2, "INTENT", 1);
    let prefix = concat(&[encode(&r1), encode(&r2)]);
    let enc3 = encode(&r3);
    let end_line = "END_INTENT_LOG_RECORD\n".len();

    let mut non_utf8_path_nine = nine_lines(&r3);
    non_utf8_path_nine[4] = b"target_canonical=/p/\xff/x".to_vec();

    let tails: Vec<(&str, Vec<u8>)> = vec![
        ("garbage line", b"this is not a record\n".to_vec()),
        ("garbage, no LF", b"zzzz".to_vec()),
        ("512 NUL bytes", vec![0u8; 512]),
        ("partial start marker", b"INTENT_LOG_RECO".to_vec()),
        ("start marker line only", b"INTENT_LOG_RECORD_V1\n".to_vec()),
        ("partial END line", {
            let mut t = enc3[..enc3.len() - end_line].to_vec();
            t.extend_from_slice(b"END_INTENT_LOG_");
            t
        }),
        (
            "complete record minus its final LF",
            enc3[..enc3.len() - 1].to_vec(),
        ),
        (
            "record missing its END line",
            enc3[..enc3.len() - end_line].to_vec(),
        ),
        ("single invalid UTF-8 byte 0xFF", vec![0xff]),
        ("invalid UTF-8 sequence C3 28", vec![0xc3, 0x28, b'\n']),
        (
            "self-consistent record with a non-UTF-8 path",
            assemble(&non_utf8_path_nine),
        ),
    ];

    let mut failures = Vec::new();
    for (label, tail) in tails {
        let mut bytes = prefix.clone();
        bytes.extend_from_slice(&tail);
        match read_log(&bytes, TXN) {
            Ok(lr) => {
                if lr.records != to_intents(&[r1.clone(), r2.clone()])
                    || lr.valid_prefix_len != prefix.len()
                {
                    failures.push(format!(
                        "[{label}] expected exactly the 2-record prefix (valid_prefix_len {}), \
                         got {} record(s), valid_prefix_len {}",
                        prefix.len(),
                        lr.records.len(),
                        lr.valid_prefix_len
                    ));
                }
            }
            Err(e) => failures.push(format!(
                "[{label}] a torn tail must be ABSENT, not an error; got {e:?}"
            )),
        }
    }
    assert_no_failures("torn-tail kinds", failures);
}

/// ADR-054 1.7 (iv) / A-1: bytes after the valid prefix FOLLOWED by a valid record are
/// corruption in the MIDDLE and the reader FAILS CLOSED -- the later valid record is never
/// silently dropped.
#[test]
fn test_BC_1_18_011_EC066_valid_prefix_then_garbage_then_a_valid_record_is_mid_log_corruption_not_silently_dropped()
 {
    let r1 = Rec::sample(TXN, 0, "INTENT", 1);
    let r3 = Rec::sample(TXN, 2, "INTENT", 1);
    let enc1 = encode(&r1);
    let enc2_partial_lines = {
        let e = encode(&Rec::sample(TXN, 1, "INTENT", 1));
        // The first five complete lines of R2 (ends with LF), no checksum, no END.
        let mut cut = 0usize;
        let mut seen = 0;
        for (i, b) in e.iter().enumerate() {
            if *b == b'\n' {
                seen += 1;
                if seen == 5 {
                    cut = i + 1;
                    break;
                }
            }
        }
        e[..cut].to_vec()
    };
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("garbage line", b"garbage\n".to_vec()),
        ("invalid UTF-8 line", vec![0xff, 0xfe, b'\n']),
        ("512 NULs then LF", {
            let mut v = vec![0u8; 512];
            v.push(b'\n');
            v
        }),
        (
            "a partial record that ends at a line boundary",
            enc2_partial_lines,
        ),
    ];
    let mut failures = Vec::new();
    for (label, middle) in cases {
        let log = concat(&[enc1.clone(), middle.clone(), encode(&r3)]);
        let want = (enc1.len(), enc1.len() + middle.len());
        match mid_log_err(read_log(&log, TXN)) {
            Some(got) if got == want => {}
            other => failures.push(format!(
                "[{label}] expected Err(MidLogCorruption {{ first_bad_offset: {}, \
                 later_valid_offset: {} }}); got {other:?} (raw result: {:?})",
                want.0,
                want.1,
                read_log(&log, TXN).map(|l| l.records.len())
            )),
        }
    }
    assert_no_failures("mid-log corruption (A-1 sandwich)", failures);
}

/// ADR-054 1.7 (iv) v1.1: "followed by a valid record" is decided at ANY BYTE POSITION, not
/// only at line starts. Garbage with no trailing LF followed by a complete valid record puts
/// the record's marker mid-line; it must still be `MidLogCorruption`, with `first_bad_offset`
/// = the end of the valid prefix and `later_valid_offset` = the R3 byte start.
#[test]
fn test_BC_1_18_011_EC066_garbage_without_trailing_lf_then_a_valid_record_is_mid_log_corruption_at_any_byte_position()
 {
    let r1 = Rec::sample(TXN, 0, "INTENT", 1);
    let r2 = Rec::sample(TXN, 1, "INTENT", 1);
    let r3 = Rec::sample(TXN, 2, "INTENT", 1);
    let (e1, e2, e3) = (encode(&r1), encode(&r2), encode(&r3));
    let end_line = "END_INTENT_LOG_RECORD\n".len();

    let mut cases: Vec<(String, Vec<u8>)> = vec![
        ("zzzz (no LF)".into(), b"zzzz".to_vec()),
        (
            "partial start marker INTENT_LOG_RECO".into(),
            b"INTENT_LOG_RECO".to_vec(),
        ),
        ("a single byte".into(), b"x".to_vec()),
        ("a lone 0xff byte".into(), vec![0xff]),
        ("one NUL".into(), vec![0u8]),
        (
            "start marker text without LF".into(),
            b"INTENT_LOG_RECORD_V1".to_vec(),
        ),
        (
            "R2 torn inside its END line".into(),
            e2[..e2.len() - end_line + 5].to_vec(),
        ),
        (
            "R2 minus only its final LF".into(),
            e2[..e2.len() - 1].to_vec(),
        ),
        (
            "R2 missing its whole END line".into(),
            e2[..e2.len() - end_line].to_vec(),
        ),
    ];
    // Every proper, non-empty cut of R2 followed by R3: the probe finds R3 at len(R1)+cut.
    for cut in 1..e2.len() {
        cases.push((
            format!("R2 cut at byte {cut}/{}", e2.len()),
            e2[..cut].to_vec(),
        ));
    }

    let mut failures = Vec::new();
    for (label, middle) in cases {
        let log = concat(&[e1.clone(), middle.clone(), e3.clone()]);
        let want = (e1.len(), e1.len() + middle.len());
        match mid_log_err(read_log(&log, TXN)) {
            Some(got) if got == want => {}
            other => failures.push(format!(
                "[{label}] expected Err(MidLogCorruption {{ first_bad_offset: {}, \
                 later_valid_offset: {} }}); got {other:?} (raw: {:?})",
                want.0,
                want.1,
                read_log(&log, TXN).map(|l| l.records.len())
            )),
        }
    }
    assert_no_failures("byte-position mid-log corruption", failures);
}

/// ADR-054 1.7 v1.1 explicit vectors: `R1 + "zzzz" + R3`, `R1 + "INTENT_LOG_RECO" + R3` and
/// `R1 + (R2 torn inside its END line) + R3`, plus the same with garbage ALSO at the front
/// of an otherwise empty valid prefix (prefix length 0).
#[test]
fn test_BC_1_18_011_EC066_explicit_byte_position_vectors_report_the_exact_offsets() {
    let r1 = Rec::sample(TXN, 0, "INTENT", 1);
    let r2 = Rec::sample(TXN, 1, "INTENT", 1);
    let r3 = Rec::sample(TXN, 2, "INTENT", 1);
    let (e1, e2, e3) = (encode(&r1), encode(&r2), encode(&r3));

    let log = concat(&[e1.clone(), b"zzzz".to_vec(), e3.clone()]);
    assert_eq!(
        mid_log_err(read_log(&log, TXN)),
        Some((e1.len(), e1.len() + 4)),
        "R1 + zzzz + R3"
    );

    let log = concat(&[e1.clone(), b"INTENT_LOG_RECO".to_vec(), e3.clone()]);
    assert_eq!(
        mid_log_err(read_log(&log, TXN)),
        Some((e1.len(), e1.len() + "INTENT_LOG_RECO".len())),
        "R1 + INTENT_LOG_RECO + R3: later_valid_offset is R3's byte start"
    );

    let torn_end = &e2[..e2.len() - "END_INTENT_LOG_RECORD\n".len() + 10];
    let log = concat(&[e1.clone(), torn_end.to_vec(), e3.clone()]);
    assert_eq!(
        mid_log_err(read_log(&log, TXN)),
        Some((e1.len(), e1.len() + torn_end.len())),
        "R1 + R2 torn inside its END line + R3"
    );

    let log = concat(&[b"zzzz".to_vec(), e3.clone()]);
    assert_eq!(
        mid_log_err(read_log(&log, TXN)),
        Some((0, 4)),
        "garbage at byte 0 then a valid record: prefix length 0"
    );
}

/// ADR-054 1.7 v1.1 re-check of properties (i)-(iii) under byte-position probing: a torn
/// tail with NO later valid record is still ABSENT (never an error), for every cut of every
/// record AND with a stored valid record that sits entirely BEFORE the cut (so the probe has
/// bytes to scan but finds nothing). Also: a record whose bytes occur inside a PATH value
/// (the marker text in a path) must not be taken for a later record.
#[test]
fn test_BC_1_18_011_EC065_byte_position_probe_does_not_turn_a_lone_torn_tail_into_an_error() {
    let recs = three_records();
    let parts: Vec<Vec<u8>> = recs.iter().map(encode).collect();
    let log = concat(&parts);
    let bounds = ends(&parts);
    let mut failures = Vec::new();
    for cut in 0..=log.len() {
        let complete = bounds.iter().filter(|&&e| e <= cut).count();
        let want_len = if complete == 0 {
            0
        } else {
            bounds[complete - 1]
        };
        // torn tail followed by various garbage that never forms a complete record
        for tail in [
            &b""[..],
            b"zzzz",
            b"\r\n",
            b"INTENT_LOG_RECORD_V1\n",
            b"INTENT_LOG_RECORD_V1\ntxn_id=",
        ] {
            let mut bytes = log[..cut].to_vec();
            bytes.extend_from_slice(tail);
            match read_log(&bytes, TXN) {
                Ok(lr)
                    if lr.valid_prefix_len == want_len
                        && lr.records == to_intents(&recs[..complete]) => {}
                other => failures.push(format!(
                    "cut {cut}, tail {:?}: a torn tail without a later valid record is absent; \
                     got {:?}",
                    String::from_utf8_lossy(tail),
                    other.map(|l| (l.records.len(), l.valid_prefix_len))
                )),
            }
        }
    }
    assert_no_failures("lone torn tails under byte-position probing", failures);

    // Marker text inside a path value is not a forged record.
    let mut rec = Rec::sample(TXN, 0, "INTENT", 1);
    rec.target = "/x/INTENT_LOG_RECORD_V1".into();
    let mut bytes = encode(&rec);
    let torn = bytes.len();
    bytes.extend_from_slice(&encode(&rec)[..40]);
    match read_log(&bytes, TXN) {
        Ok(lr) if lr.records.len() == 1 && lr.valid_prefix_len == torn => {}
        other => panic!(
            "marker text inside a path must not forge a later record; got {:?}",
            other.map(|l| (l.records.len(), l.valid_prefix_len))
        ),
    }
}

// ===========================================================================
// AC-007 / T5 -- grammar strictness
// ===========================================================================

/// `assemble` with the checksum computed over the first nine of `lines` and the `lines`
/// beyond nine written BEFORE the checksum line (so the checksum line is not line 10).
fn assemble_with_extra(lines: &[Vec<u8>]) -> Vec<u8> {
    let mut nine_bytes = Vec::new();
    for l in &lines[..9] {
        nine_bytes.extend_from_slice(l);
        nine_bytes.push(b'\n');
    }
    let mut out = nine_bytes.clone();
    for l in &lines[9..] {
        out.extend_from_slice(l);
        out.push(b'\n');
    }
    out.extend_from_slice(
        format!(
            "record_checksum={}\nEND_INTENT_LOG_RECORD\n",
            sha256_hex(&nine_bytes)
        )
        .as_bytes(),
    );
    out
}

fn with_line(base: &Rec, idx: usize, replacement: &str) -> Vec<u8> {
    let mut nine = nine_lines(base);
    nine[idx] = replacement.as_bytes().to_vec();
    assemble(&nine)
}

/// ADR-054 1.2 rule 1 and 1.7: duplicate key, unknown key, blank line, CRLF, reordered
/// keys, uppercase hex and extra whitespace each make the record INVALID. Every mutated
/// record below carries a checksum that MATCHES ITS OWN BYTES, so only the grammar can
/// reject it. Two assertions per vector: alone it is a torn tail (zero records, no error);
/// followed by a valid record it is mid-log corruption (proving it was judged invalid, not
/// merely ignored).
#[test]
fn test_BC_1_18_011_15b_grammar_strictness_each_violation_makes_the_record_invalid() {
    let base = Rec::sample(TXN, 0, "INTENT", 1);
    let nine = nine_lines(&base);
    let mut vectors: Vec<(&str, Vec<u8>)> = Vec::new();

    // duplicate key
    vectors.push((
        "duplicate key (txn_id twice, replacing fencing_generation)",
        with_line(&base, 2, &format!("txn_id={TXN}")),
    ));
    {
        let mut lines = nine.clone();
        lines.push(format!("txn_id={TXN}").into_bytes());
        vectors.push((
            "duplicate key as an extra 10th line",
            assemble_with_extra(&lines),
        ));
    }
    // unknown key
    vectors.push((
        "unknown key replacing timestamp_utc",
        with_line(&base, 8, "timestamp=2026-10-08T12:00:00Z"),
    ));
    {
        let mut lines = nine.clone();
        lines.push(b"unexpected_key=1".to_vec());
        vectors.push((
            "unknown key as an extra 10th line",
            assemble_with_extra(&lines),
        ));
    }
    // blank line
    {
        let mut lines = nine.clone();
        lines.insert(4, Vec::new());
        // 10 pre-checksum lines: the blank is inside the first nine bytes of the checksum
        let mut out = Vec::new();
        for l in &lines[..9] {
            out.extend_from_slice(l);
            out.push(b'\n');
        }
        let sum = sha256_hex(&out);
        out.extend_from_slice(lines[9].as_slice());
        out.push(b'\n');
        out.extend_from_slice(format!("record_checksum={sum}\nEND_INTENT_LOG_RECORD\n").as_bytes());
        vectors.push(("blank line inside the record", out));
    }
    // CRLF
    {
        let mut out = Vec::new();
        for l in &nine {
            out.extend_from_slice(l);
            out.extend_from_slice(b"\r\n");
        }
        let sum = sha256_hex(&out);
        out.extend_from_slice(
            format!("record_checksum={sum}\r\nEND_INTENT_LOG_RECORD\r\n").as_bytes(),
        );
        vectors.push(("CRLF line endings throughout", out));
    }
    // reordered keys
    {
        let mut lines = nine.clone();
        lines.swap(2, 3);
        vectors.push((
            "reordered keys (fencing_generation / record_type swapped)",
            assemble(&lines),
        ));
    }
    // uppercase hex
    {
        let mut r = base.clone();
        r.post = r.post.to_uppercase();
        vectors.push(("uppercase expected_post_hash", encode(&r)));
        let mut r = Rec::sample(TXN, 1, "INTENT", 1); // odd => hex pre-state
        r.pre = r.pre.to_uppercase();
        vectors.push(("uppercase expected_pre_state", encode(&r)));
    }
    {
        let mut out = assemble(&nine);
        // uppercase the stored checksum value while keeping every other byte
        let text = String::from_utf8(out.clone()).unwrap();
        let line = text.lines().nth(9).unwrap().to_string();
        let upper = format!(
            "record_checksum={}",
            line["record_checksum=".len()..].to_uppercase()
        );
        out = text.replace(&line, &upper).into_bytes();
        vectors.push(("uppercase record_checksum value", out));
    }
    // extra whitespace
    vectors.push((
        "leading space on a line",
        with_line(&base, 1, &format!(" txn_id={TXN}")),
    ));
    vectors.push((
        "trailing space on a line",
        with_line(&base, 2, "fencing_generation=1 "),
    ));
    vectors.push((
        "spaces around '='",
        with_line(&base, 1, &format!("txn_id = {TXN}")),
    ));
    vectors.push((
        "tab inside a line",
        with_line(&base, 1, &format!("txn_id=\t{TXN}")),
    ));
    // markers
    {
        let mut lines = nine.clone();
        lines[0] = b"INTENT_LOG_RECORD_V2".to_vec();
        vectors.push(("start marker V2", assemble(&lines)));
        let mut lines = nine.clone();
        lines[0] = b"INTENT_LOG_RECORD_V1 ".to_vec();
        vectors.push(("start marker with a trailing space", assemble(&lines)));
        let mut ok = assemble(&nine);
        let n = ok.len();
        ok.truncate(n - "END_INTENT_LOG_RECORD\n".len());
        ok.extend_from_slice(b"END_INTENT_LOG_RECORD \n");
        vectors.push(("END marker with a trailing space", ok));
        let mut ok = assemble(&nine);
        let n = ok.len();
        ok.truncate(n - "END_INTENT_LOG_RECORD\n".len());
        ok.extend_from_slice(b"END_RECORD\n");
        vectors.push(("END marker replaced by the OLD v1.23 spelling", ok));
    }

    let control = encode(&Rec::sample(TXN, 5, "INTENT", 1));
    let mut failures = Vec::new();
    // Control: the unmutated record parses, so the harness itself is sound.
    match read_log(&encode(&base), TXN) {
        Ok(lr) if lr.records == vec![to_intent(&base)] => {}
        other => failures.push(format!(
            "[control] the unmutated record must parse: {:?}",
            other.map(|l| l.records.len())
        )),
    }
    for (label, bytes) in vectors {
        // (1) alone: a torn tail, i.e. zero records, valid_prefix_len 0, no error
        match read_log(&bytes, TXN) {
            Ok(lr) if lr.records.is_empty() && lr.valid_prefix_len == 0 => {}
            other => failures.push(format!(
                "[{label}] alone: the record must be INVALID (zero records, valid_prefix_len 0, \
                 no error); got {:?}",
                other.map(|l| (l.records.len(), l.valid_prefix_len))
            )),
        }
        // (2) followed by a valid record: mid-log corruption at offset 0
        let log = concat(&[bytes.clone(), control.clone()]);
        match mid_log_err(read_log(&log, TXN)) {
            Some((0, later)) if later == bytes.len() => {}
            other => failures.push(format!(
                "[{label}] followed by a valid record: expected Err(MidLogCorruption {{ \
                 first_bad_offset: 0, later_valid_offset: {} }}); got {other:?}",
                bytes.len()
            )),
        }
    }
    assert_no_failures("grammar strictness", failures);
}

/// ADR-054 1.2 value rules 2-7 apply at READ time too (the file is untrusted): a record
/// whose bytes carry a MATCHING checksum but whose value breaks a rule is invalid.
#[test]
fn test_BC_1_18_011_15b_value_rules_are_enforced_by_the_reader_even_when_the_checksum_matches() {
    let base = Rec::sample(TXN, 0, "INTENT", 1);
    let mut vectors: Vec<(&str, Vec<u8>)> = Vec::new();
    let mut put = |label: &'static str, idx: usize, line: Vec<u8>| {
        let mut nine = nine_lines(&base);
        nine[idx] = line;
        vectors.push((label, assemble(&nine)));
    };
    put("empty txn_id", 1, b"txn_id=".to_vec());
    put("txn_id with a slash", 1, b"txn_id=a/b".to_vec());
    put("txn_id with a space", 1, b"txn_id=a b".to_vec());
    put(
        "txn_id of 129 characters",
        1,
        format!("txn_id={}", "a".repeat(129)).into_bytes(),
    );
    put(
        "fencing_generation with a leading zero",
        2,
        b"fencing_generation=01".to_vec(),
    );
    put(
        "fencing_generation = u64::MAX + 1",
        2,
        b"fencing_generation=18446744073709551616".to_vec(),
    );
    put(
        "negative fencing_generation",
        2,
        b"fencing_generation=-1".to_vec(),
    );
    put(
        "signed fencing_generation",
        2,
        b"fencing_generation=+1".to_vec(),
    );
    put(
        "empty fencing_generation",
        2,
        b"fencing_generation=".to_vec(),
    );
    put("lowercase record_type", 3, b"record_type=intent".to_vec());
    put("unknown record_type", 3, b"record_type=COMMIT".to_vec());
    put("empty target_canonical", 4, b"target_canonical=".to_vec());
    put(
        "target_canonical ending in a space (v1.1 rule 2)",
        4,
        b"target_canonical=/p/x ".to_vec(),
    );
    put(
        "target_canonical beginning with a space (v1.1 rule 2)",
        4,
        b"target_canonical= /p/x".to_vec(),
    );
    put(
        "staging_path ending in a space (v1.1 rule 2)",
        5,
        b"staging_path=/s/x ".to_vec(),
    );
    put(
        "staging_path beginning with a space (v1.1 rule 2)",
        5,
        b"staging_path= /s/x".to_vec(),
    );
    put(
        "target_canonical with a C0 control (0x1f)",
        4,
        b"target_canonical=/p/\x1f/x".to_vec(),
    );
    put(
        "target_canonical with DEL",
        4,
        b"target_canonical=/p/\x7f/x".to_vec(),
    );
    put(
        "target_canonical with a non-UTF-8 byte",
        4,
        b"target_canonical=/p/\xff/x".to_vec(),
    );
    put(
        "target_canonical with an overlong UTF-8 sequence",
        4,
        b"target_canonical=/p/\xc0\xaf/x".to_vec(),
    );
    put(
        "target_canonical with a UTF-16 surrogate encoded as UTF-8",
        4,
        b"target_canonical=/p/\xed\xa0\x80/x".to_vec(),
    );
    put(
        "target_canonical of 4097 bytes",
        4,
        format!("target_canonical=/{}", "a".repeat(4096)).into_bytes(),
    );
    put(
        "staging_path with a C0 control",
        5,
        b"staging_path=/s/\x01".to_vec(),
    );
    put(
        "63-digit expected_post_hash",
        6,
        format!("expected_post_hash={}", &base.post[..63]).into_bytes(),
    );
    put(
        "65-digit expected_post_hash",
        6,
        format!("expected_post_hash={}0", base.post).into_bytes(),
    );
    put(
        "non-hex expected_post_hash",
        6,
        format!("expected_post_hash=g{}", &base.post[1..]).into_bytes(),
    );
    put(
        "lowercase missing sentinel",
        7,
        b"expected_pre_state=missing".to_vec(),
    );
    put(
        "empty expected_pre_state",
        7,
        b"expected_pre_state=".to_vec(),
    );
    put(
        "timestamp with an offset",
        8,
        b"timestamp_utc=2026-10-08T12:00:00+00:00".to_vec(),
    );
    put(
        "timestamp month 13",
        8,
        b"timestamp_utc=2026-13-08T12:00:00Z".to_vec(),
    );
    put(
        "timestamp hour 24",
        8,
        b"timestamp_utc=2026-10-08T24:00:00Z".to_vec(),
    );
    put(
        "timestamp second 60",
        8,
        b"timestamp_utc=2026-10-08T12:00:60Z".to_vec(),
    );
    put(
        "timestamp lowercase z",
        8,
        b"timestamp_utc=2026-10-08T12:00:00z".to_vec(),
    );
    put(
        "timestamp with 10 fractional digits",
        8,
        b"timestamp_utc=2026-10-08T12:00:00.1234567890Z".to_vec(),
    );

    let mut failures = Vec::new();
    for (label, bytes) in vectors {
        match read_log(&bytes, TXN) {
            Ok(lr) if lr.records.is_empty() && lr.valid_prefix_len == 0 => {}
            other => failures.push(format!(
                "[{label}] the record breaks a Decision 1.2 value rule and must be invalid; got {:?}",
                other.map(|l| (l.records.len(), l.valid_prefix_len))
            )),
        }
    }
    // Controls: values at the edge of each rule are VALID.
    let mut accept: Vec<(&str, Vec<u8>)> = Vec::new();
    {
        let mut put_ok = |label: &'static str, idx: usize, line: Vec<u8>| {
            let mut nine = nine_lines(&base);
            nine[idx] = line;
            accept.push((label, assemble(&nine)));
        };
        put_ok(
            "128-character txn_id of token characters",
            1,
            format!("txn_id={}", "a-_.".repeat(32)).into_bytes(),
        );
        put_ok(
            "fencing_generation = u64::MAX",
            2,
            b"fencing_generation=18446744073709551615".to_vec(),
        );
        put_ok(
            "fencing_generation = 0",
            2,
            b"fencing_generation=0".to_vec(),
        );
        put_ok("record_type ABORTED", 3, b"record_type=ABORTED".to_vec());
        put_ok(
            "target_canonical of exactly 4096 bytes",
            4,
            format!("target_canonical=/{}", "a".repeat(4095)).into_bytes(),
        );
        put_ok(
            "target_canonical with '=', '|' and an inner space",
            4,
            b"target_canonical=/p/a=b|c d".to_vec(),
        );
        put_ok(
            "target_canonical containing the marker text",
            4,
            b"target_canonical=/p/END_INTENT_LOG_RECORD".to_vec(),
        );
        put_ok(
            "target_canonical with a space adjacent to '/' (interior, valid)",
            4,
            b"target_canonical=/p/a /b".to_vec(),
        );
        put_ok(
            "target_canonical with multi-byte UTF-8",
            4,
            "target_canonical=/p/\u{e9}\u{65e5}\u{672c}"
                .as_bytes()
                .to_vec(),
        );
        put_ok(
            "timestamp with 9 fractional digits",
            8,
            b"timestamp_utc=2026-10-08T12:00:00.123456789Z".to_vec(),
        );
    }
    for (label, bytes) in accept {
        // The 128-character vector carries its own txn id (the reader checks L1 against the
        // live txn's id); every other vector uses the fixture's.
        let txn = if label.starts_with("128-character txn_id") {
            "a-_.".repeat(32)
        } else {
            TXN.to_string()
        };
        match read_log(&bytes, &txn) {
            Ok(lr) if lr.records.len() == 1 && lr.valid_prefix_len == bytes.len() => {}
            other => failures.push(format!(
                "[control: {label}] a record at the edge of the rule is VALID; got {:?}",
                other.map(|l| (l.records.len(), l.valid_prefix_len))
            )),
        }
    }
    assert_no_failures("reader value rules", failures);
}

// ===========================================================================
// AC-007 / T15 -- log invariants L1-L4
// ===========================================================================

/// ADR-054 1.8 / BC-1.18.011 EC-068: each invariant violation over otherwise VALID records
/// fails closed with the invariant and the START OFFSET of the FIRST violating record in
/// file order. (For L2 the violating record is the LATER INTENT that disagrees with an
/// earlier one: that is where an in-order scan first sees the conflict.)
#[test]
fn test_BC_1_18_011_EC068_log_invariants_l1_to_l4_fail_closed_with_first_violating_offset() {
    let i0 = Rec::sample(TXN, 0, "INTENT", 1);
    let i1 = Rec::sample(TXN, 1, "INTENT", 1);
    let i2 = Rec::sample(TXN, 2, "INTENT", 1);
    let d0 = i0.done_of(2, "2026-10-08T12:00:09Z");
    let d1 = i1.done_of(2, "2026-10-08T12:00:09Z");
    let foreign = |r: &Rec| Rec {
        txn_id: "other-txn".into(),
        ..r.clone()
    };

    struct V {
        label: &'static str,
        recs: Vec<Rec>,
        want: Invariant,
        at: usize, // index of the first violating record
    }
    let v = |label, recs: Vec<Rec>, want, at| V {
        label,
        recs,
        want,
        at,
    };
    let vectors = vec![
        v(
            "L1 foreign txn_id on a middle record",
            vec![i0.clone(), foreign(&i1), i2.clone()],
            Invariant::L1,
            1,
        ),
        v(
            "L1 foreign txn_id on the first record",
            vec![foreign(&i0), i1.clone()],
            Invariant::L1,
            0,
        ),
        v(
            "L1 foreign txn_id on a DONE",
            vec![i0.clone(), foreign(&d0)],
            Invariant::L1,
            1,
        ),
        v(
            "L2 conflicting expected_post_hash",
            vec![
                i0.clone(),
                i1.clone(),
                Rec {
                    post: sha256_hex(b"other"),
                    ..i0.clone()
                },
            ],
            Invariant::L2,
            2,
        ),
        v(
            "L2 conflicting staging_path",
            vec![
                i0.clone(),
                Rec {
                    staging: "/proj/other/staging.md".into(),
                    ..i0.clone()
                },
            ],
            Invariant::L2,
            1,
        ),
        v(
            "L2 conflicting expected_pre_state",
            vec![
                i0.clone(),
                Rec {
                    pre: sha256_hex(b"other-pre"),
                    ..i0.clone()
                },
            ],
            Invariant::L2,
            1,
        ),
        v(
            "L3 DONE with no INTENT for its target",
            vec![i0.clone(), d1.clone()],
            Invariant::L3,
            1,
        ),
        v(
            "L3 DONE before its INTENT in file order",
            vec![d0.clone(), i0.clone()],
            Invariant::L3,
            0,
        ),
        v(
            "L3 DONE disagreeing on expected_post_hash",
            vec![
                i0.clone(),
                Rec {
                    post: sha256_hex(b"other"),
                    ..d0.clone()
                },
            ],
            Invariant::L3,
            1,
        ),
        v(
            "L3 DONE disagreeing on staging_path",
            vec![
                i0.clone(),
                Rec {
                    staging: "/proj/other/staging.md".into(),
                    ..d0.clone()
                },
            ],
            Invariant::L3,
            1,
        ),
        v(
            "L3 DONE disagreeing on expected_pre_state",
            vec![
                i0.clone(),
                Rec {
                    pre: sha256_hex(b"other-pre"),
                    ..d0.clone()
                },
            ],
            Invariant::L3,
            1,
        ),
        v(
            "L4 fencing_generation decreases between INTENTs",
            vec![
                Rec {
                    fencing: 2,
                    ..i0.clone()
                },
                Rec {
                    fencing: 1,
                    ..i1.clone()
                },
            ],
            Invariant::L4,
            1,
        ),
        v(
            "L4 DONE lower than its INTENT",
            vec![
                Rec {
                    fencing: 3,
                    ..i0.clone()
                },
                d0.clone(),
            ],
            Invariant::L4,
            1,
        ),
        // v1.1 "first violating record": the LATER conflicting INTENT is the L2 offset, the
        // first-seen INTENT is the reference even if a later one repeats the conflict.
        v(
            "L2 offset is the first conflicting INTENT, not the earlier reference nor a repeat",
            vec![
                i0.clone(),
                Rec {
                    post: sha256_hex(b"other"),
                    ..i0.clone()
                },
                Rec {
                    post: sha256_hex(b"other"),
                    ..i0.clone()
                },
            ],
            Invariant::L2,
            1,
        ),
        // Several checks fail on the SAME record: the first in the order L1, L2, L3, L4.
        v(
            "same record fails L1 and L4: L1 is reported",
            vec![
                Rec {
                    fencing: 5,
                    ..i0.clone()
                },
                Rec {
                    fencing: 1,
                    txn_id: "foreign-txn".into(),
                    ..i1.clone()
                },
            ],
            Invariant::L1,
            1,
        ),
        v(
            "same record fails L2 and L4: L2 is reported",
            vec![
                Rec {
                    fencing: 5,
                    ..i0.clone()
                },
                Rec {
                    fencing: 1,
                    post: sha256_hex(b"other"),
                    ..i0.clone()
                },
            ],
            Invariant::L2,
            1,
        ),
        v(
            "same record fails L3 and L4: L3 is reported",
            vec![
                Rec {
                    fencing: 5,
                    ..i0.clone()
                },
                Rec {
                    fencing: 1,
                    ..d1.clone()
                },
            ],
            Invariant::L3,
            1,
        ),
        // Earliest violation wins, whatever its class.
        v(
            "first violation in file order is reported (L3 before the later L1)",
            vec![i0.clone(), d1.clone(), foreign(&i2)],
            Invariant::L3,
            1,
        ),
        v(
            "first violation in file order is reported (L1 before the later L3)",
            vec![i0.clone(), foreign(&i1), d1.clone()],
            Invariant::L1,
            1,
        ),
    ];

    let mut failures = Vec::new();
    for V {
        label,
        recs,
        want,
        at,
    } in vectors
    {
        let parts: Vec<Vec<u8>> = recs.iter().map(encode).collect();
        let log = concat(&parts);
        let want_off = if at == 0 { 0 } else { ends(&parts)[at - 1] };
        match read_log(&log, TXN) {
            Err(LogReadError::InvariantViolation { invariant, offset })
                if invariant == want && offset == want_off => {}
            other => failures.push(format!(
                "[{label}] expected Err(InvariantViolation {{ invariant: {want:?}, offset: \
                 {want_off} }}); got {:?}",
                other.map(|l| l.records.len())
            )),
        }
    }
    assert_no_failures("log invariants L1-L4", failures);
}

/// ADR-054 1.8 L2 / BC-1.18.011 EC-068 control: a STAGING re-run appends duplicate IDENTICAL
/// INTENTs; they are accepted (the most recent is canonical). Also: a fully consistent
/// INTENT...DONE log with non-decreasing (equal) fencing is accepted.
#[test]
fn test_BC_1_18_011_EC068_identical_duplicate_intents_are_accepted() {
    let i0 = Rec::sample(TXN, 0, "INTENT", 1);
    let i1 = Rec::sample(TXN, 1, "INTENT", 1);
    let i2 = Rec::sample(TXN, 2, "INTENT", 1);
    let d0 = i0.done_of(2, "2026-10-08T12:00:09Z");
    let d1 = i1.done_of(2, "2026-10-08T12:00:10Z");
    let d2 = i2.done_of(2, "2026-10-08T12:00:11Z");

    let mut failures = Vec::new();
    let logs: Vec<(&str, Vec<Rec>)> = vec![
        ("identical duplicate INTENTs", vec![i0.clone(), i0.clone()]),
        (
            "duplicate INTENT batch (STAGING re-run) then DONEs",
            vec![
                i0.clone(),
                i1.clone(),
                i0.clone(),
                i1.clone(),
                d0.clone(),
                d1.clone(),
            ],
        ),
        (
            "full INTENT batch then DONE batch, equal fencing within a phase",
            vec![i0.clone(), i1.clone(), i2.clone(), d0, d1, d2],
        ),
    ];
    for (label, recs) in logs {
        let log = concat(&recs.iter().map(encode).collect::<Vec<_>>());
        match read_log(&log, TXN) {
            Ok(lr) if lr.records == to_intents(&recs) && lr.valid_prefix_len == log.len() => {}
            other => failures.push(format!(
                "[{label}] must be accepted in full; got {:?}",
                other.map(|l| (l.records.len(), l.valid_prefix_len))
            )),
        }
    }
    assert_no_failures("duplicate-INTENT control", failures);
}

// ===========================================================================
// AC-008 / T2 -- tail repair under the writer
// ===========================================================================

/// BC-1.18.011 EC-065 / ADR-054 1.9 step 2: valid R1 + a torn tail, then the writer is
/// opened and R3 appended. Without repair R3 would be sandwiched behind garbage and the
/// reader would report mid-log corruption (A-1); with repair the file is EXACTLY
/// R1 ++ R3 and both are returned.
#[test]
fn test_BC_1_18_011_EC065_torn_tail_is_truncated_durably_before_the_first_append_and_r1_r3_survive()
{
    let r1 = Rec::sample(TXN, 0, "INTENT", 1);
    let r3 = Rec::sample(TXN, 2, "INTENT", 1);
    let enc1 = encode(&r1);
    let enc3 = encode(&r3);
    let end_line = "END_INTENT_LOG_RECORD\n".len();
    let tails: Vec<(&str, Vec<u8>)> = vec![
        ("garbage line", b"this is not a record\n".to_vec()),
        ("garbage, no LF", b"zzzz".to_vec()),
        ("512 NUL bytes", vec![0u8; 512]),
        ("partial start marker", b"INTENT_LOG_RECO".to_vec()),
        ("partial END line", {
            let e = encode(&Rec::sample(TXN, 1, "INTENT", 1));
            let mut t = e[..e.len() - end_line].to_vec();
            t.extend_from_slice(b"END_INTENT_LOG_");
            t
        }),
        ("invalid UTF-8 bytes", vec![0xff, 0xfe, 0x80, 0xc3, 0x28]),
    ];

    let mut failures = Vec::new();

    // The pre-repair hazard, shown by control: R1 + garbage + R3 as a raw concatenation is
    // mid-log corruption, so a writer that appended without repairing would lose R1.
    let sandwiched = concat(&[enc1.clone(), b"garbage\n".to_vec(), enc3.clone()]);
    if mid_log_err(read_log(&sandwiched, TXN)).is_none() {
        failures.push("[control] R1 + garbage + R3 must be mid-log corruption".to_string());
    }

    for (label, tail) in tails {
        let (_d, log) = tempdir_log();
        let mut original = enc1.clone();
        original.extend_from_slice(&tail);
        std::fs::write(&log, &original).unwrap();

        let outcome = (|| -> Result<(), BcIndexMigrationError> {
            let mut w = IntentLogWriter::open(&StdFs, &log, TXN)?;
            w.append_batch(&[to_intent(&r3)])
        })();
        if let Err(e) = outcome {
            failures.push(format!(
                "[{label}] open + append must succeed after repair: {e}"
            ));
            continue;
        }
        let bytes = std::fs::read(&log).unwrap();
        let expect = concat(&[enc1.clone(), enc3.clone()]);
        if bytes != expect {
            failures.push(format!(
                "[{label}] on-disk bytes must be EXACTLY R1 ++ R3 ({} bytes: the torn tail \
                 truncated, nothing sandwiched); got {} bytes",
                expect.len(),
                bytes.len()
            ));
        }
        match read_log(&bytes, TXN) {
            Ok(lr) if lr.records == to_intents(&[r1.clone(), r3.clone()]) => {}
            other => failures.push(format!(
                "[{label}] R1 and R3 must both be returned; got {:?}",
                other.map(|l| l.records.len())
            )),
        }
    }
    assert_no_failures("torn-tail repair", failures);
}

/// ADR-054 1.9 step 2: `MidLogCorruption` or an invariant violation at open is
/// `INTENT_LOG_CORRUPT` with NOTHING mutated (no truncation, no append).
#[test]
fn test_BC_1_18_011_EC065_open_on_mid_log_corruption_or_invariant_violation_mutates_nothing() {
    let r1 = Rec::sample(TXN, 0, "INTENT", 1);
    let r2 = Rec::sample(TXN, 1, "INTENT", 1);
    let r3 = Rec::sample(TXN, 2, "INTENT", 1);
    let foreign = Rec {
        txn_id: "other-txn".into(),
        ..r2.clone()
    };
    let enc1 = encode(&r1);

    let cases: Vec<(&str, Vec<u8>, &str, usize)> = vec![
        (
            "mid-log corruption",
            concat(&[enc1.clone(), b"garbage\n".to_vec(), encode(&r3)]),
            "mid_log_corruption",
            enc1.len(),
        ),
        (
            "L1 invariant violation (foreign txn_id)",
            concat(&[enc1.clone(), encode(&foreign), encode(&r3)]),
            "log_invariant_violation",
            enc1.len(),
        ),
    ];
    let mut failures = Vec::new();
    for (label, original, want_kind, want_offset) in cases {
        let (_d, log) = tempdir_log();
        std::fs::write(&log, &original).unwrap();
        match IntentLogWriter::open(&StdFs, &log, TXN) {
            Err(BcIndexMigrationError::IntentLogCorrupt { path, kind, offset }) => {
                if kind != want_kind || offset != want_offset as u64 || path != log {
                    failures.push(format!(
                        "[{label}] expected IntentLogCorrupt {{ kind: {want_kind}, offset: \
                         {want_offset} }} for {}; got kind {kind:?} offset {offset} path {}",
                        log.display(),
                        path.display()
                    ));
                }
            }
            Err(e) => failures.push(format!("[{label}] expected IntentLogCorrupt, got {e}")),
            Ok(_) => failures.push(format!("[{label}] open must fail closed, it succeeded")),
        }
        if std::fs::read(&log).unwrap() != original {
            failures.push(format!(
                "[{label}] the log must be byte-identical (nothing mutated)"
            ));
        }
    }
    assert_no_failures("open fails closed", failures);
}

/// ADR-054 1.9 step 2: `open` on an ABSENT log succeeds and does not create the file (the
/// first append does); `open` on a CLEAN log (ends at a record boundary) performs no
/// truncation; a batch of two records is appended contiguously.
#[test]
fn test_BC_1_18_011_EC065_open_never_creates_the_file_and_a_clean_log_is_not_truncated() {
    let r1 = Rec::sample(TXN, 0, "INTENT", 1);
    let r2 = Rec::sample(TXN, 1, "INTENT", 1);
    let r3 = Rec::sample(TXN, 2, "INTENT", 1);
    let mut failures = Vec::new();

    // absent
    let (_d, log) = tempdir_log();
    match IntentLogWriter::open(&StdFs, &log, TXN) {
        Ok(_) => {
            if log.exists() {
                failures.push("open on an absent log must not create the file".to_string());
            }
        }
        Err(e) => failures.push(format!("open on an absent log must succeed: {e}")),
    }
    // absent -> first batch creates it with exactly the two records
    let outcome = (|| -> Result<(), BcIndexMigrationError> {
        let mut w = IntentLogWriter::open(&StdFs, &log, TXN)?;
        w.append_batch(&[to_intent(&r1), to_intent(&r2)])
    })();
    match outcome {
        Ok(()) => {
            if std::fs::read(&log).unwrap() != concat(&[encode(&r1), encode(&r2)]) {
                failures
                    .push("the first batch must create the log holding exactly R1 ++ R2".into());
            }
        }
        Err(e) => failures.push(format!("first batch must succeed: {e}")),
    }
    // clean log: reopen + append one more
    let outcome = (|| -> Result<(), BcIndexMigrationError> {
        let mut w = IntentLogWriter::open(&StdFs, &log, TXN)?;
        w.append_batch(&[to_intent(&r3)])
    })();
    match outcome {
        Ok(()) => {
            if std::fs::read(&log).unwrap() != concat(&[encode(&r1), encode(&r2), encode(&r3)]) {
                failures.push("a clean log must be extended in place: R1 ++ R2 ++ R3".into());
            }
        }
        Err(e) => failures.push(format!("append to a clean log must succeed: {e}")),
    }
    assert_no_failures("open/append on absent and clean logs", failures);
}
