// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-25.10 RED tests -- the two new coordinator errors `INTENT_LOG_CORRUPT` and
//! `INTENT_LOG_VALUE_REJECTED`: exact `Display`, closed placeholder domains, sanitizer,
//! exit-code table rows, exhaustive classification (no wildcard arm).
//!
//! Authority: ADR-054 v1.0 Decision 3 "New named errors" and Decision 3.1 (normative stderr
//! text, corrections C-1..C-3); BC-1.18.011 v1.21 Postcondition 15(e), EC-066..EC-068,
//! Precondition 6(f)(vi); error-taxonomy v1.41 rows `INTENT_LOG_CORRUPT`,
//! `INTENT_LOG_VALUE_REJECTED` (closed `<kind>` / `<field>` / `<reason>` domains); story
//! S-25.10 AC-010. The by-name exit-code TABLE rows live in the S-25.09 table test, which
//! this story extends (`s2509_v123_exit_codes_and_txn_schema_inprocess_test.rs`: the
//! `ruled` and `instances` tables gain the two variants).
//!
//! | Story AC | Test |
//! |---|---|
//! | AC-010 | `..._15e_display_equals_the_adr_054_3_1_line_for_fixed_inputs_two_codes` |
//! | AC-010 | `..._15e_every_placeholder_domain_token_renders` |
//! | AC-010 | `..._15e_hostile_values_are_truncated_and_escaped_one_physical_line` |
//! | AC-010 | `..._15e_exit_code_classification_of_the_two_variants_exit_2_state_integrity` |
//! | AC-010 | `..._15e_exit_code_table_lists_the_two_variants_by_name_exit_2` |
//!
//! `Display` is `INTENT_LOG_... (exit 2): ...` WITHOUT a subcommand label: the coordinator
//! prints `migrate-bc-index: ` + the variant text (and there is NO `BC-INDEX migration: `
//! label because the shared module serves both subcommands). The whole-line, real-binary
//! assertions are in `s2510_coordinator_blackbox_test.rs`.
//!
//! The variants do not exist yet: this file fails to COMPILE (the accepted Red).

use std::path::PathBuf;

#[path = "s2510_support/mod.rs"]
mod support;

use factory_dispatcher::shard_manager::{
    AdmissionFailureCause, BcIndexMigrationError, BcIndexMigrationOutcome,
    migration_process_exit_code, sanitize_diagnostic_id,
};

const KINDS: [&str; 2] = ["mid_log_corruption", "log_invariant_violation"];
const FIELDS: [&str; 10] = [
    "staging_path",
    "canonical_path",
    "txn_id",
    "fencing_generation",
    "record_type",
    "target_canonical",
    "staging_path",
    "expected_post_hash",
    "expected_pre_state",
    "timestamp_utc",
];
const REASONS: [&str; 9] = [
    "contains_control_character",
    "not_utf8",
    "empty",
    "over_4096_bytes",
    "not_64_lowercase_hex",
    "malformed_timestamp",
    "not_1_to_128_token_characters",
    "not_canonical_u64",
    "unknown_record_type",
];

fn corrupt(path: &str, kind: &str, offset: u64) -> BcIndexMigrationError {
    BcIndexMigrationError::IntentLogCorrupt {
        path: PathBuf::from(path),
        kind: kind.to_string(),
        offset,
    }
}

fn rejected(field: &str, reason: &str) -> BcIndexMigrationError {
    BcIndexMigrationError::IntentLogValueRejected {
        field: field.to_string(),
        reason: reason.to_string(),
    }
}

/// The ADR-054 3.1 line, `<subcommand>: ` omitted.
fn corrupt_text(log_file: &str, kind: &str, offset: u64) -> String {
    format!(
        "INTENT_LOG_CORRUPT (exit 2): intent log {log_file} is corrupt ({kind}) at byte offset \
         {offset}; no further move or append was made and no completion was recorded; operator \
         investigation required"
    )
}

fn rejected_text(field: &str, reason: &str) -> String {
    format!(
        "INTENT_LOG_VALUE_REJECTED (exit 2): intent-log field {field} rejected: {reason}; no \
         record of this batch was appended"
    )
}

/// ADR-054 3.1 "Final text": `Display` equals the normative single line for fixed inputs.
/// `<log_file>` is the log FILE NAME only (correction C-3), never the full path.
#[test]
fn test_BC_1_18_011_15e_display_equals_the_adr_054_3_1_line_for_fixed_inputs_two_codes() {
    let c = corrupt(
        "/proj/.factory/migration-state/intent-7c1d2e3f-4a5b-4c6d-8e7f-9a0b1c2d3e4f.log",
        "mid_log_corruption",
        579,
    );
    assert_eq!(
        c.to_string(),
        "INTENT_LOG_CORRUPT (exit 2): intent log intent-7c1d2e3f-4a5b-4c6d-8e7f-9a0b1c2d3e4f.log \
         is corrupt (mid_log_corruption) at byte offset 579; no further move or append was made \
         and no completion was recorded; operator investigation required"
    );
    let c2 = corrupt("intent-gen-1.log", "log_invariant_violation", 0);
    assert_eq!(
        c2.to_string(),
        corrupt_text("intent-gen-1.log", "log_invariant_violation", 0)
    );

    let r = rejected("staging_path", "contains_control_character");
    assert_eq!(
        r.to_string(),
        "INTENT_LOG_VALUE_REJECTED (exit 2): intent-log field staging_path rejected: \
         contains_control_character; no record of this batch was appended"
    );

    // Negative guards from the ratified corrections.
    for (label, text) in [
        ("INTENT_LOG_CORRUPT", c.to_string()),
        ("INTENT_LOG_VALUE_REJECTED", r.to_string()),
    ] {
        assert!(
            !text.contains("BC-INDEX migration:"),
            "[{label}] no `BC-INDEX migration: ` label"
        );
        assert!(!text.contains('\n'), "[{label}] one physical line");
        assert!(
            !text.contains("nothing was moved or appended"),
            "[{label}] C-1: the false claim"
        );
        assert!(
            !text.contains("nothing was staged"),
            "[{label}] C-2: the false claim"
        );
    }
    assert!(
        !c.to_string().contains("/proj/"),
        "C-3: the directory part of the path is never echoed"
    );
}

/// Every token of every closed placeholder domain renders verbatim (the sanitizer must not
/// mangle a legitimate token such as `not_1_to_128_token_characters`).
#[test]
fn test_BC_1_18_011_15e_every_placeholder_domain_token_renders() {
    let mut failures = Vec::new();
    for kind in KINDS {
        let got = corrupt("/x/intent-abc.log", kind, 42).to_string();
        let want = corrupt_text("intent-abc.log", kind, 42);
        if got != want {
            failures.push(format!("kind {kind}: {got:?} != {want:?}"));
        }
    }
    for field in FIELDS {
        for reason in REASONS {
            let got = rejected(field, reason).to_string();
            let want = rejected_text(field, reason);
            if got != want {
                failures.push(format!(
                    "field {field} / reason {reason}: {got:?} != {want:?}"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} token rendering failure(s):\n  - {}",
        failures.len(),
        failures.join("\n  - ")
    );
}

/// ADR-054 3.1 placeholder rules: every placeholder goes through the SAME sanitizer as the
/// ADR-052 admission diagnostic (`sanitize_diagnostic_id`: control characters escaped, at
/// most 64 characters, escaped form truncated); a hostile value cannot forge terminal
/// output. Hostile inputs: a log file name carrying LF / ESC / CR, a 300-character name,
/// hostile `kind` / `field` / `reason` strings. The rendering is checked against the
/// production sanitizer AND by invariants that do not depend on it.
#[test]
fn test_BC_1_18_011_15e_hostile_values_are_truncated_and_escaped_one_physical_line() {
    let hostile_names = [
        "intent-\n[FORGED] migrate-bc-index: all clear.log".to_string(),
        "intent-\u{1b}[2J\u{1b}[31m.log".to_string(),
        "intent-\r\n.log".to_string(),
        format!("intent-{}.log", "x".repeat(300)),
        "intent-\u{0}\u{7}\u{7f}.log".to_string(),
    ];
    let mut failures = Vec::new();
    for name in &hostile_names {
        let path = PathBuf::from(format!("/proj/.factory/migration-state/{name}"));
        let got = corrupt(&path.to_string_lossy(), "mid_log_corruption", 7).to_string();
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();
        let want = corrupt_text(&sanitize_diagnostic_id(&file_name), "mid_log_corruption", 7);
        if got != want {
            failures.push(format!("log_file {name:?}: {got:?} != {want:?}"));
        }
        if got.chars().any(char::is_control) {
            failures.push(format!(
                "log_file {name:?}: the line contains a raw control character: {got:?}"
            ));
        }
        let rendered = sanitize_diagnostic_id(&file_name);
        if rendered.chars().count() > 64 {
            failures.push(format!(
                "log_file {name:?}: rendered file name exceeds 64 characters"
            ));
        }
    }
    let hostile_tokens = [
        "a\nb".to_string(),
        "\u{1b}[31mred".to_string(),
        "z".repeat(200),
        "tab\there".to_string(),
    ];
    for tok in &hostile_tokens {
        let got = rejected(tok, tok).to_string();
        let want = rejected_text(&sanitize_diagnostic_id(tok), &sanitize_diagnostic_id(tok));
        if got != want {
            failures.push(format!("field/reason {tok:?}: {got:?} != {want:?}"));
        }
        if got.chars().any(char::is_control) {
            failures.push(format!(
                "field/reason {tok:?}: raw control character in {got:?}"
            ));
        }
        let got = corrupt("/x/intent-a.log", tok, 1).to_string();
        let want = corrupt_text("intent-a.log", &sanitize_diagnostic_id(tok), 1);
        if got != want {
            failures.push(format!("kind {tok:?}: {got:?} != {want:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} hostile-rendering failure(s):\n  - {}",
        failures.len(),
        failures.join("\n  - ")
    );
}

/// Postcondition 15(e) / Precondition 6(f)(vi) / Invariant 5: both variants are exit 2 (NOT
/// exit 1, which is reserved for `EXPIRY_ABORT` and `MIGRATION_LOCK_CONTENTION`), classified
/// `StateIntegrity` in `admission_failure_cause`, and named by their taxonomy code token.
#[test]
fn test_BC_1_18_011_15e_exit_code_classification_of_the_two_variants_exit_2_state_integrity() {
    type Make = fn() -> BcIndexMigrationError;
    let errors: [(&str, Make, &str); 2] = [
        (
            "IntentLogCorrupt",
            || corrupt("/x/intent-a.log", "mid_log_corruption", 0),
            "INTENT_LOG_CORRUPT",
        ),
        (
            "IntentLogValueRejected",
            || rejected("staging_path", "empty"),
            "INTENT_LOG_VALUE_REJECTED",
        ),
    ];
    for (name, make, token) in errors {
        let e = make();
        assert_eq!(e.process_exit_code(), 2, "{name}: exit 2");
        assert_ne!(e.process_exit_code(), 1, "{name}: never exit 1");
        assert_eq!(
            e.admission_failure_cause(),
            AdmissionFailureCause::StateIntegrity,
            "{name}: classified StateIntegrity (Invariant 5)"
        );
        assert_eq!(e.code_token(), Some(token), "{name}: taxonomy code token");
        assert!(
            e.to_string().starts_with(token),
            "{name}: Display leads with the code token"
        );
        assert!(
            e.to_string().contains("(exit 2)"),
            "{name}: Display carries the literal `exit 2`"
        );
        assert_eq!(
            migration_process_exit_code(&Err::<BcIndexMigrationOutcome, _>(make())),
            2,
            "{name}: the CLI exit code is 2"
        );
    }
}

// ---------------------------------------------------------------------------
// source gate: listed BY NAME in the exhaustive matches, no wildcard
// ---------------------------------------------------------------------------

/// Precondition 6(f)(vi) / Postcondition 15(e): the two variants are LISTED BY NAME in
/// `process_exit_code`, `admission_failure_cause` and `code_token` -- exhaustive matches
/// with no wildcard arm, so an unclassified variant is a compile error.
#[test]
fn test_BC_1_18_011_15e_exit_code_table_lists_the_two_variants_by_name_exit_2() {
    let src = support::production_source("src/shard_manager.rs");
    let mut failures = Vec::new();
    for header in [
        "pub fn process_exit_code(&self)",
        "pub fn admission_failure_cause(&self)",
        "pub fn code_token(&self)",
    ] {
        let body = support::fn_body(&src, header);
        for variant in ["IntentLogCorrupt", "IntentLogValueRejected"] {
            if !body.contains(&format!("BcIndexMigrationError::{variant}")) {
                failures.push(format!(
                    "`{header}` does not name `BcIndexMigrationError::{variant}`"
                ));
            }
        }
        let squashed: String = body.split_whitespace().collect::<Vec<_>>().join(" ");
        if squashed.contains("_ =>") || squashed.contains("_ |") {
            failures.push(format!("`{header}` has a wildcard arm"));
        }
    }
    // process_exit_code: the two variants are in the exit-2 arm, not the exit-1 arm.
    let body = support::fn_body(&src, "pub fn process_exit_code(&self)");
    let one = body.find("=> 1").expect("an exit-1 arm");
    let exit1_arm = &body[..one];
    for variant in ["IntentLogCorrupt", "IntentLogValueRejected"] {
        if exit1_arm.contains(variant) {
            failures.push(format!("{variant} must not be in the exit-1 arm"));
        }
    }
    assert!(
        failures.is_empty(),
        "exhaustive classification:\n  - {}",
        failures.join("\n  - ")
    );
}
