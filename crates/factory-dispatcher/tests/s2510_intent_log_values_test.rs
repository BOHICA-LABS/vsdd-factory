// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-25.10 RED tests -- the WRITER value rules (module level).
//!
//! Authority: ADR-054 v1.0 Decision 1.2 rules 1-7 and Decision 1.9 step 1 ("validate before
//! any byte"); BC-1.18.011 v1.21 Postcondition 15(c), EC-067; error-taxonomy v1.41 row
//! `INTENT_LOG_VALUE_REJECTED` (closed `<field>` / `<reason>` domains); story S-25.10 AC-004.
//!
//! | Story AC / red test | Test |
//! |---|---|
//! | AC-004 / T6 (path rule 2, every reason) | `..._EC067_validate_path_returns_the_closed_domain_reason_for_each_hostile_path` |
//! | AC-004 / T6 (record fields, zero bytes appended) | `..._EC067_writer_rejects_each_hostile_value_zero_bytes_appended` |
//! | AC-004 / T6 (rules 5-6, timestamp / hash / txn_id edges) | `..._EC067_timestamp_hash_and_txn_id_grammar_edges` |
//! | AC-004 / T6 controls | `..._EC067_benign_paths_with_equals_pipe_space_and_marker_text_round_trip` |
//!
//! The plan-builder half of EC-067 (AC-005: `run_bc_index_migration` prologue validation,
//! nothing staged, pre-commit abort path) drives the real binary and lives in
//! `s2510_coordinator_blackbox_test.rs`.
//!
//! Not representable, so not tested at the type level: `fencing_generation` is a `u64`
//! (reason `not_canonical_u64` can only arise when READING a log, covered by
//! `s2510_intent_log_reader_test.rs`) and `record_type` is an enum (`unknown_record_type`
//! likewise reader-only). NUL and over-4096-byte plan paths cannot exist on a real
//! filesystem, so they are covered here through `validate_path` / `encode_record`.
//!
//! The module under test does not exist yet: this file fails to COMPILE (the accepted Red).

#[path = "s2510_support/api.rs"]
mod api;
#[path = "s2510_support/mod.rs"]
mod support;

use std::ffi::OsString;
use std::path::PathBuf;

use api::{IntentLogWriter, IntentRecord, encode_record, read_log, to_intent, validate_path};
use factory_dispatcher::shard_manager::BcIndexMigrationError;
use factory_dispatcher::shard_manager::migration_fs::StdFs;
use support::{Rec, TXN, assert_no_failures, encode};

fn good() -> IntentRecord {
    to_intent(&Rec::sample(TXN, 0, "INTENT", 1))
}

fn set_path(r: &mut IntentRecord, field: &str, p: PathBuf) {
    match field {
        "target_canonical" => r.target_canonical = p,
        "staging_path" => r.staging_path = p,
        other => panic!("test fixture error: not a path field: {other}"),
    }
}

fn rejected(e: BcIndexMigrationError) -> Option<(String, String)> {
    match e {
        BcIndexMigrationError::IntentLogValueRejected { field, reason } => Some((field, reason)),
        _ => None,
    }
}

#[cfg(unix)]
fn non_utf8(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    PathBuf::from(OsString::from_vec(bytes.to_vec()))
}

// ===========================================================================
// path rule 2
// ===========================================================================

/// ADR-054 1.2 rule 2: `validate_path` returns the closed-domain reason token for each
/// hostile path (and Ok for every legal one). LF, CR, NUL, EVERY other C0 control and DEL
/// are `contains_control_character`; non-UTF-8 (never converted lossily) is `not_utf8`;
/// empty is `empty`; the limit is 4096 BYTES, not characters.
#[test]
fn test_BC_1_18_011_EC067_validate_path_returns_the_closed_domain_reason_for_each_hostile_path() {
    let mut cases: Vec<(String, PathBuf, Result<(), &'static str>)> = Vec::new();
    cases.push(("empty".into(), PathBuf::new(), Err("empty")));
    for c in 0u8..=0x1f {
        cases.push((
            format!("C0 control 0x{c:02x}"),
            PathBuf::from(format!("/a{}b", c as char)),
            Err("contains_control_character"),
        ));
    }
    cases.push((
        "DEL 0x7f".into(),
        PathBuf::from("/a\u{7f}b"),
        Err("contains_control_character"),
    ));
    cases.push((
        "LF at the very end".into(),
        PathBuf::from("/a/b\n"),
        Err("contains_control_character"),
    ));
    cases.push((
        "4097 ASCII bytes".into(),
        PathBuf::from(format!("/{}", "a".repeat(4096))),
        Err("over_4096_bytes"),
    ));
    cases.push((
        "2049 two-byte characters (4098 bytes, only 2049 chars): the limit is BYTES".into(),
        PathBuf::from("\u{e9}".repeat(2049)),
        Err("over_4096_bytes"),
    ));
    // Legal paths.
    cases.push((
        "exactly 4096 ASCII bytes".into(),
        PathBuf::from(format!("/{}", "a".repeat(4095))),
        Ok(()),
    ));
    cases.push((
        "2048 two-byte characters (exactly 4096 bytes)".into(),
        PathBuf::from("\u{e9}".repeat(2048)),
        Ok(()),
    ));
    cases.push((
        "'=', '|', spaces and marker text".into(),
        PathBuf::from("/a=b|c d/END_INTENT_LOG_RECORD/INTENT_LOG_RECORD_V1/record_checksum=0"),
        Ok(()),
    ));
    cases.push((
        "multi-byte UTF-8".into(),
        PathBuf::from("/\u{65e5}\u{672c}\u{8a9e}/\u{e9}"),
        Ok(()),
    ));
    // ADR-054 1.2 rule 2 forbids exactly U+0000..U+001F and U+007F; a C1 control such as
    // U+0085 (NEL) is a well-formed scalar outside those ranges and is therefore LEGAL.
    cases.push((
        "C1 control U+0085 (outside the rule's ranges)".into(),
        PathBuf::from("/a\u{85}b"),
        Ok(()),
    ));
    cases.push(("lone '/'".into(), PathBuf::from("/"), Ok(())));
    // ADR-054 v1.1 rule 2: a leading or trailing U+0020 is rejected (never trimmed); interior
    // spaces, including one adjacent to '/', stay legal.
    for (label, p) in [
        ("leading space", " /a"),
        ("trailing space", "/a "),
        ("a single space", " "),
        ("two spaces", "  "),
        ("leading and trailing", " /a "),
        ("two trailing spaces", "/a  "),
    ] {
        cases.push((
            label.into(),
            PathBuf::from(p),
            Err("leading_or_trailing_space"),
        ));
    }
    for (label, p) in [
        ("interior space (control)", "/a b"),
        ("space adjacent to '/' (control)", "/a /b"),
        ("space after the leading slash (control)", "/ a"),
        ("U+00A0 trailing is ordinary text, not U+0020", "/a\u{a0}"),
    ] {
        cases.push((label.into(), PathBuf::from(p), Ok(())));
    }
    // v1.1 reason precedence: empty, over_4096_bytes, not_utf8, contains_control_character,
    // leading_or_trailing_space (the first listed wins).
    cases.push((
        "NUL + trailing space: control beats space".into(),
        PathBuf::from("/a\0 "),
        Err("contains_control_character"),
    ));
    cases.push((
        "leading space + LF: control beats space".into(),
        PathBuf::from(" /a\nb"),
        Err("contains_control_character"),
    ));
    cases.push((
        "trailing TAB is a control, not a space".into(),
        PathBuf::from("/a\t"),
        Err("contains_control_character"),
    ));
    cases.push((
        "4097 bytes ending in a space: over_4096_bytes beats space".into(),
        PathBuf::from(format!("/{} ", "a".repeat(4095))),
        Err("over_4096_bytes"),
    ));
    cases.push((
        "4097 bytes with a NUL: over_4096_bytes beats control".into(),
        PathBuf::from(format!("/{}\0", "a".repeat(4095))),
        Err("over_4096_bytes"),
    ));
    cases.push((
        "exactly 4096 bytes ending in a space: space (length is fine)".into(),
        PathBuf::from(format!("/{} ", "a".repeat(4094))),
        Err("leading_or_trailing_space"),
    ));
    #[cfg(unix)]
    {
        cases.push((
            "4097 bytes with a non-UTF-8 byte: over_4096_bytes beats not_utf8".into(),
            non_utf8(&[b"/".as_slice(), &[b'a'; 4095], b"\xff"].concat()),
            Err("over_4096_bytes"),
        ));
        cases.push((
            "NUL + non-UTF-8: not_utf8 beats control".into(),
            non_utf8(b"/a\0\xff"),
            Err("not_utf8"),
        ));
        cases.push((
            "non-UTF-8 + trailing space: not_utf8 beats space".into(),
            non_utf8(b"/a\xff "),
            Err("not_utf8"),
        ));
        cases.push((
            "non-UTF-8 + leading space + NUL: not_utf8 beats both".into(),
            non_utf8(b" /a\0\xff"),
            Err("not_utf8"),
        ));
    }
    #[cfg(unix)]
    {
        cases.push((
            "non-UTF-8 byte 0xff".into(),
            non_utf8(b"/tmp/\xff"),
            Err("not_utf8"),
        ));
        cases.push((
            "overlong UTF-8 c0 af".into(),
            non_utf8(b"/tmp/\xc0\xaf"),
            Err("not_utf8"),
        ));
        cases.push((
            "truncated sequence e2 82".into(),
            non_utf8(b"/tmp/\xe2\x82"),
            Err("not_utf8"),
        ));
        cases.push((
            "UTF-16 surrogate encoded as UTF-8 (ed a0 80)".into(),
            non_utf8(b"/tmp/\xed\xa0\x80"),
            Err("not_utf8"),
        ));
    }

    let mut failures = Vec::new();
    for (label, path, want) in cases {
        let got = validate_path(&path);
        if got != want {
            failures.push(format!(
                "[{label}] validate_path -> {got:?}, expected {want:?}"
            ));
        }
    }
    assert_no_failures("validate_path", failures);
}

// ===========================================================================
// writer: every hostile value, zero bytes appended
// ===========================================================================

/// BC-1.18.011 EC-067 / ADR-054 1.9 step 1: the writer validates EVERY field of EVERY
/// record BEFORE any byte is appended and rejects with `INTENT_LOG_VALUE_REJECTED`, naming
/// the field and the closed-domain reason. Both `encode_record` and the writer are
/// exercised; for the writer the rejected record is the SECOND of a two-record batch and
/// the log must not exist afterwards (the first, valid record must NOT have been appended
/// either: "no record of this batch was appended").
#[test]
fn test_BC_1_18_011_EC067_writer_rejects_each_hostile_value_zero_bytes_appended() {
    type Mutate = Box<dyn Fn(&mut IntentRecord)>;
    let mut cases: Vec<(String, Mutate, &str, &str)> = Vec::new();
    let mut add = |label: &str, m: Mutate, field: &'static str, reason: &'static str| {
        cases.push((label.to_string(), m, field, reason));
    };

    for field in ["target_canonical", "staging_path"] {
        let mut path_case = |label: &str, p: PathBuf, reason: &'static str| {
            add(
                &format!("{field}: {label}"),
                Box::new(move |r| set_path(r, field, p.clone())),
                field,
                reason,
            );
        };
        path_case("LF", PathBuf::from("/a\nb"), "contains_control_character");
        path_case("CR", PathBuf::from("/a\rb"), "contains_control_character");
        path_case("NUL", PathBuf::from("/a\0b"), "contains_control_character");
        path_case(
            "other C0 (0x01)",
            PathBuf::from("/a\u{1}b"),
            "contains_control_character",
        );
        path_case(
            "ESC (0x1b)",
            PathBuf::from("/a\u{1b}[31mb"),
            "contains_control_character",
        );
        path_case(
            "DEL",
            PathBuf::from("/a\u{7f}b"),
            "contains_control_character",
        );
        path_case("empty", PathBuf::new(), "empty");
        path_case(
            "leading space",
            PathBuf::from(" /a"),
            "leading_or_trailing_space",
        );
        path_case(
            "trailing space",
            PathBuf::from("/a "),
            "leading_or_trailing_space",
        );
        path_case(
            "NUL + trailing space (control wins)",
            PathBuf::from("/a\0 "),
            "contains_control_character",
        );
        path_case(
            "4097 bytes",
            PathBuf::from(format!("/{}", "a".repeat(4096))),
            "over_4096_bytes",
        );
        #[cfg(unix)]
        path_case("non-UTF-8 byte", non_utf8(b"/tmp/\xff/x"), "not_utf8");
    }

    for (label, v) in [
        (
            "uppercase",
            "DD2CE8A1000FD7B33E333408ED7DC137D3C8358B5602DCC14BF9919EA00FE2BC".to_string(),
        ),
        ("63 digits", "a".repeat(63)),
        ("65 digits", "a".repeat(65)),
        ("non-hex digit", format!("g{}", "a".repeat(63))),
        ("empty", String::new()),
        ("0x prefix", format!("0x{}", "a".repeat(62))),
    ] {
        add(
            &format!("expected_post_hash: {label}"),
            Box::new(move |r| r.expected_post_hash = v.clone()),
            "expected_post_hash",
            "not_64_lowercase_hex",
        );
    }
    for (label, v) in [
        ("uppercase hex", "A".repeat(64)),
        (
            "the literal MISSING is spelled as None, not as a string",
            "MISSING".to_string(),
        ),
        ("lowercase 'missing'", "missing".to_string()),
        ("63 digits", "a".repeat(63)),
        ("empty string", String::new()),
    ] {
        add(
            &format!("expected_pre_state: {label}"),
            Box::new(move |r| r.expected_pre_state = Some(v.clone())),
            "expected_pre_state",
            "not_64_lowercase_hex",
        );
    }
    for (label, v) in [
        ("empty", String::new()),
        ("129 characters", "a".repeat(129)),
        ("space", "a b".to_string()),
        ("slash", "a/b".to_string()),
        ("equals sign", "a=b".to_string()),
        ("newline", "a\nb".to_string()),
        ("non-ASCII", "t\u{e9}".to_string()),
    ] {
        add(
            &format!("txn_id: {label}"),
            Box::new(move |r| r.txn_id = v.clone()),
            "txn_id",
            "not_1_to_128_token_characters",
        );
    }
    for (label, v) in [
        ("empty", ""),
        ("date only", "2026-10-08"),
        ("offset instead of Z", "2026-10-08T12:00:00+00:00"),
        ("month 13", "2026-13-08T12:00:00Z"),
        ("month 00", "2026-00-08T12:00:00Z"),
        ("day 32", "2026-10-32T12:00:00Z"),
        ("day 00", "2026-10-00T12:00:00Z"),
        ("hour 24", "2026-10-08T24:00:00Z"),
        ("minute 60", "2026-10-08T12:60:00Z"),
        ("second 60", "2026-10-08T12:00:60Z"),
        ("lowercase t", "2026-10-08t12:00:00Z"),
        ("lowercase z", "2026-10-08T12:00:00z"),
        ("10 fractional digits", "2026-10-08T12:00:00.1234567890Z"),
        ("empty fraction", "2026-10-08T12:00:00.Z"),
        ("trailing newline", "2026-10-08T12:00:00Z\n"),
    ] {
        add(
            &format!("timestamp_utc: {label}"),
            Box::new(move |r| r.timestamp_utc = v.to_string()),
            "timestamp_utc",
            "malformed_timestamp",
        );
    }

    let mut failures = Vec::new();
    for (label, mutate, field, reason) in cases {
        // (1) encode_record
        let mut bad = good();
        mutate(&mut bad);
        match encode_record(&bad) {
            Err(e) => match rejected(e) {
                Some((f, r)) if f == field && r == reason => {}
                other => failures.push(format!(
                    "[{label}] encode_record: expected IntentLogValueRejected {{ field: {field}, \
                     reason: {reason} }}; got {other:?}"
                )),
            },
            Ok(_) => failures.push(format!("[{label}] encode_record accepted a hostile value")),
        }
        // (2) writer: the bad record is the SECOND of the batch; nothing at all is appended
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("intent-gen-1.log");
        let first = to_intent(&Rec::sample(TXN, 1, "INTENT", 1));
        let outcome = (|| -> Result<(), BcIndexMigrationError> {
            let mut w = IntentLogWriter::open(&StdFs, &log, TXN)?;
            w.append_batch(&[first.clone(), bad.clone()])
        })();
        match outcome {
            Err(e) => match rejected(e) {
                Some((f, r)) if f == field && r == reason => {}
                other => failures.push(format!(
                    "[{label}] writer: expected IntentLogValueRejected {{ field: {field}, reason: \
                     {reason} }}; got {other:?}"
                )),
            },
            Ok(()) => failures.push(format!(
                "[{label}] writer appended a batch holding a hostile value"
            )),
        }
        if log.exists() && std::fs::metadata(&log).map(|m| m.len()).unwrap_or(0) != 0 {
            failures.push(format!(
                "[{label}] ZERO bytes of the rejected batch may be appended (the valid first record \
                 included); the log holds {} bytes",
                std::fs::metadata(&log).map(|m| m.len()).unwrap_or(0)
            ));
        }
    }
    assert!(failures.len() < 10_000); // keep the failure message bounded
    assert_no_failures("writer value rules", failures);
}

/// ADR-054 1.2 rules 3, 5 and 6 at the accept/reject boundary: what is LEGAL must encode
/// (and read back), so the validators are neither too lax nor too strict.
#[test]
fn test_BC_1_18_011_EC067_timestamp_hash_and_txn_id_grammar_edges() {
    let mut failures = Vec::new();
    let mut check_ok = |label: &str, m: &dyn Fn(&mut IntentRecord)| {
        let mut r = good();
        m(&mut r);
        match encode_record(&r) {
            Ok(bytes) => match read_log(&bytes, &r.txn_id) {
                Ok(lr) if lr.records == vec![r.clone()] => {}
                other => failures.push(format!(
                    "[{label}] encodes but does not read back: {:?}",
                    other.map(|l| l.records.len())
                )),
            },
            Err(e) => failures.push(format!("[{label}] must be accepted, got {e}")),
        }
    };
    for ts in [
        "0000-01-01T00:00:00Z",
        "9999-12-31T23:59:59Z",
        "2026-02-31T12:00:00Z", // rule 6: day 01-31, no month-length semantics
        "2026-10-08T12:00:00.1Z",
        "2026-10-08T12:00:00.123456789Z",
    ] {
        check_ok(&format!("timestamp {ts}"), &|r| {
            r.timestamp_utc = ts.to_string()
        });
    }
    check_ok("txn_id of 128 token characters", &|r| {
        r.txn_id = "aZ0-_.".repeat(21) + "ab";
    });
    check_ok("txn_id of 1 character", &|r| r.txn_id = "x".into());
    check_ok("txn_id that is a UUID", &|r| {
        r.txn_id = "0f8e4c1a-6b7d-4e2f-9a3c-5d1b2e7f8a90".into();
    });
    check_ok("expected_pre_state None (MISSING)", &|r| {
        r.expected_pre_state = None
    });
    check_ok("fencing_generation u64::MAX", &|r| {
        r.fencing_generation = u64::MAX
    });
    check_ok("fencing_generation 0", &|r| r.fencing_generation = 0);
    assert_no_failures("grammar edges", failures);
}

// ===========================================================================
// controls: benign paths round-trip exactly
// ===========================================================================

/// BC-1.18.011 EC-067 controls: paths containing `=`, the pipe character, spaces and the
/// text `END_INTENT_LOG_RECORD` (or `INTENT_LOG_RECORD_V1`) are VALID and round-trip
/// EXACTLY -- through `encode_record` (byte-identical to the independent reference
/// encoder), through the writer to disk, and back through `read_log`.
#[test]
fn test_BC_1_18_011_EC067_benign_paths_with_equals_pipe_space_and_marker_text_round_trip() {
    let paths = [
        "/a=b",
        "/a|b",
        "/a b/c d",
        "/a /b",
        "/ a",
        "/x/END_INTENT_LOG_RECORD",
        "/x/INTENT_LOG_RECORD_V1",
        "/x/record_checksum=0123",
        "/=/=/=",
        "/\u{65e5}\u{672c}\u{8a9e}/\u{e9}/\u{f1}",
        "/a\\b",
    ];
    let mut failures = Vec::new();
    for p in paths {
        let mut rec = Rec::sample(TXN, 0, "INTENT", 1);
        rec.target = p.to_string();
        rec.staging = format!("{p}.staged");
        let want_bytes = encode(&rec);
        let intent = to_intent(&rec);

        match encode_record(&intent) {
            Ok(b) if b == want_bytes => {}
            Ok(b) => failures.push(format!(
                "[{p}] encode_record is not byte-identical to the reference encoding ({} vs {} bytes)",
                b.len(),
                want_bytes.len()
            )),
            Err(e) => failures.push(format!("[{p}] must be accepted, got {e}")),
        }

        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("intent-gen-1.log");
        let outcome = (|| -> Result<(), BcIndexMigrationError> {
            let mut w = IntentLogWriter::open(&StdFs, &log, TXN)?;
            w.append_batch(std::slice::from_ref(&intent))
        })();
        if let Err(e) = outcome {
            failures.push(format!("[{p}] writer must accept: {e}"));
            continue;
        }
        let on_disk = std::fs::read(&log).unwrap();
        if on_disk != want_bytes {
            failures.push(format!(
                "[{p}] on-disk bytes differ from the reference encoding"
            ));
        }
        match read_log(&on_disk, TXN) {
            Ok(lr)
                if lr.records == vec![intent.clone()] && lr.valid_prefix_len == on_disk.len() => {}
            other => failures.push(format!(
                "[{p}] must read back as exactly the same record; got {:?}",
                other.map(|l| l.records.len())
            )),
        }
    }
    // 4096-byte and 2048-two-byte-character paths at the limit
    for p in [format!("/{}", "a".repeat(4095)), "\u{e9}".repeat(2048)] {
        let mut rec = Rec::sample(TXN, 0, "INTENT", 1);
        rec.target = p.clone();
        match encode_record(&to_intent(&rec)) {
            Ok(b) if b == encode(&rec) => {}
            other => failures.push(format!(
                "[{} bytes at the limit] must be accepted and byte-identical; got ok={}",
                p.len(),
                other.is_ok()
            )),
        }
    }
    assert_no_failures("benign path controls", failures);
}
