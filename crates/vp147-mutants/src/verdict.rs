//! Verdict classification from a killer's exit status and parsed result lines.
//!
//! Classification never substring-matches the word "error". Only
//! line-anchored, structured markers are used:
//!
//! * Kani: `VERIFICATION:- SUCCESSFUL | FAILED | UNDETERMINED`, `Checking harness <name>...`
//! * cargo test: `test <name> ... FAILED`, `test result: ok|FAILED. N passed; M failed; ...`
//! * compile failure: a line *starting with* `error[E....]` or `error: could not compile`.
//!
//! cargo's `error: test failed, to rerun pass ...` and `error: N targets
//! failed:` lines are ordinary test-failure noise and are NOT compile errors.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The killer detected the mutant (the desired outcome).
    Killed { evidence: Vec<String> },
    /// The killer passed against the mutated tree: the property suite missed it.
    Survived,
    /// The experiment is void (compile failure, spec mismatch, tool failure,
    /// nothing ran). Never counts as a kill.
    Invalid { reason: String },
}

impl Verdict {
    pub fn label(&self) -> &'static str {
        match self {
            Verdict::Killed { .. } => "KILLED",
            Verdict::Survived => "SURVIVED",
            Verdict::Invalid { .. } => "INVALID",
        }
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

fn invalid(reason: impl Into<String>) -> Verdict {
    Verdict::Invalid {
        reason: reason.into(),
    }
}

/// True for a line that is a rustc/cargo *build* failure marker.
fn is_compile_error_line(line: &str) -> bool {
    let l = line.trim_start();
    if l.starts_with("error: could not compile") {
        return true;
    }
    // `error[E0425]: ...`
    if let Some(rest) = l.strip_prefix("error[E") {
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        return !digits.is_empty() && rest[digits.len()..].starts_with("]:");
    }
    false
}

/// Classify a `cargo kani` run.
///
/// `expected_harnesses` is the number of harnesses requested; a SURVIVED
/// verdict requires every one of them to report SUCCESSFUL.
pub fn classify_kani(exit_code: Option<i32>, output: &str, expected_harnesses: usize) -> Verdict {
    let mut current: Option<String> = None;
    let mut failed: Vec<String> = Vec::new();
    let mut successful = 0usize;
    let mut undetermined = 0usize;
    let mut compile_error = false;

    for line in output.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("Checking harness ") {
            current = Some(rest.trim_end_matches("...").to_string());
        } else if t.starts_with("VERIFICATION:- FAILED") {
            failed.push(
                current
                    .clone()
                    .unwrap_or_else(|| "<unknown harness>".into()),
            );
        } else if t.starts_with("VERIFICATION:- SUCCESSFUL") {
            successful += 1;
        } else if t.starts_with("VERIFICATION:- UNDETERMINED") {
            undetermined += 1;
        } else if is_compile_error_line(line) {
            compile_error = true;
        }
    }

    let Some(code) = exit_code else {
        return invalid("cargo kani terminated by signal (no exit code)");
    };
    if compile_error && failed.is_empty() {
        return invalid("compile error building the mutated tree (not a kill)");
    }
    if undetermined > 0 {
        return invalid(format!("{undetermined} harness(es) UNDETERMINED"));
    }
    match (code, failed.is_empty()) {
        (0, true) => {
            if successful == expected_harnesses {
                Verdict::Survived
            } else {
                invalid(format!(
                    "exit 0 but {successful} SUCCESSFUL verdict(s) for {expected_harnesses} requested harness(es)"
                ))
            }
        }
        (0, false) => invalid("exit 0 contradicts VERIFICATION:- FAILED lines"),
        (_, false) => Verdict::Killed {
            evidence: failed
                .into_iter()
                .map(|h| format!("VERIFICATION:- FAILED {h}"))
                .collect(),
        },
        (c, true) => invalid(format!(
            "exit {c} without any VERIFICATION:- FAILED line (tool failure or build error)"
        )),
    }
}

/// Classify a `cargo test` run.
pub fn classify_test(exit_code: Option<i32>, output: &str) -> Verdict {
    let mut failed_tests: Vec<String> = Vec::new();
    let mut passed_total = 0u64;
    let mut result_lines = 0usize;
    let mut compile_error = false;

    for line in output.lines() {
        if is_compile_error_line(line) {
            compile_error = true;
        }
        if let Some(rest) = line.strip_prefix("test ")
            && let Some(name) = rest.strip_suffix(" ... FAILED")
        {
            failed_tests.push(name.to_string());
        }
        if let Some(rest) = line.strip_prefix("test result: ") {
            result_lines += 1;
            // "ok. 3 passed; 0 failed; ..." / "FAILED. 1 passed; 2 failed; ..."
            if let Some(n) = rest
                .split(';')
                .next()
                .and_then(|seg| seg.split_whitespace().rev().nth(1))
                .and_then(|tok| tok.parse::<u64>().ok())
            {
                passed_total += n;
            }
        }
    }

    let Some(code) = exit_code else {
        return invalid("cargo test terminated by signal (no exit code)");
    };
    if compile_error {
        return invalid("compile error building the mutated tree (not a kill)");
    }
    match (code, failed_tests.is_empty()) {
        (0, true) => {
            if result_lines == 0 || passed_total == 0 {
                invalid("exit 0 but no tests ran (filter matched nothing?)")
            } else {
                Verdict::Survived
            }
        }
        (0, false) => invalid("exit 0 contradicts `... FAILED` test lines"),
        (_, false) => {
            failed_tests.sort();
            failed_tests.dedup();
            Verdict::Killed {
                evidence: failed_tests
                    .into_iter()
                    .map(|t| format!("test {t} ... FAILED"))
                    .collect(),
            }
        }
        (c, true) => invalid(format!(
            "exit {c} without any failing `test ... FAILED` line (build error or tool failure)"
        )),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    const KANI_FAILED: &str = "\
Kani Rust Verifier 0.68.0
Checking harness shard_manager::obl1_kani_proofs::proof_a...
SUMMARY:
 ** 1 of 90 failed
Failed Checks: assertion failed: x
VERIFICATION:- FAILED
Checking harness shard_manager::obl1_kani_proofs::proof_b...
VERIFICATION:- SUCCESSFUL
Complete - 1 successfully verified harnesses, 1 failures, 2 total.
";

    const KANI_OK: &str = "\
Checking harness m::a...
VERIFICATION:- SUCCESSFUL
Checking harness m::b...
VERIFICATION:- SUCCESSFUL
";

    #[test]
    fn kani_failed_harness_is_killed_with_harness_name() {
        let v = classify_kani(Some(1), KANI_FAILED, 2);
        match v {
            Verdict::Killed { evidence } => {
                assert_eq!(
                    evidence,
                    ["VERIFICATION:- FAILED shard_manager::obl1_kani_proofs::proof_a"]
                );
            }
            other => panic!("expected Killed, got {other:?}"),
        }
    }

    #[test]
    fn kani_all_successful_is_survived() {
        assert_eq!(classify_kani(Some(0), KANI_OK, 2), Verdict::Survived);
    }

    #[test]
    fn kani_success_count_mismatch_is_invalid_not_survived() {
        assert!(matches!(
            classify_kani(Some(0), KANI_OK, 3),
            Verdict::Invalid { .. }
        ));
    }

    #[test]
    fn kani_compile_error_is_invalid() {
        let out = "error[E0425]: cannot find value `x` in this scope\n --> src/a.rs:1:1\n";
        assert!(matches!(
            classify_kani(Some(1), out, 1),
            Verdict::Invalid { .. }
        ));
        let out = "error: could not compile `factory-dispatcher` (lib) due to 1 previous error\n";
        assert!(matches!(
            classify_kani(Some(101), out, 1),
            Verdict::Invalid { .. }
        ));
    }

    #[test]
    fn kani_nonzero_exit_without_verdict_is_invalid() {
        assert!(matches!(
            classify_kani(Some(2), "boom\n", 1),
            Verdict::Invalid { .. }
        ));
        assert!(matches!(
            classify_kani(None, KANI_FAILED, 2),
            Verdict::Invalid { .. }
        ));
    }

    #[test]
    fn kani_undetermined_is_invalid() {
        let out = "Checking harness m::a...\nVERIFICATION:- UNDETERMINED\n";
        assert!(matches!(
            classify_kani(Some(1), out, 1),
            Verdict::Invalid { .. }
        ));
    }

    const TEST_FAILED_WITH_CARGO_ERROR_NOISE: &str = "\
   Compiling factory-dispatcher v0.0.0
     Running tests/s2508_admission_blackbox_test.rs (target/debug/deps/s2508-abc)
running 3 tests
test a_ok ... ok
test b_reservation_present ... FAILED
test c_ok ... ok

failures:

---- b_reservation_present stdout ----
thread 'b_reservation_present' panicked at tests/x.rs:9:5:
assertion failed

test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.10s

error: test failed, to rerun pass `-p factory-dispatcher --test s2508_admission_blackbox_test`
error: 1 target failed:
    `-p factory-dispatcher --test s2508_admission_blackbox_test`
";

    #[test]
    fn cargo_error_test_failed_noise_is_a_kill_not_invalid() {
        // Regression: the scratchpad runner substring-matched "error" and
        // reported this exact output as INVALID.
        match classify_test(Some(101), TEST_FAILED_WITH_CARGO_ERROR_NOISE) {
            Verdict::Killed { evidence } => {
                assert_eq!(evidence, ["test b_reservation_present ... FAILED"]);
            }
            other => panic!("expected Killed, got {other:?}"),
        }
    }

    #[test]
    fn test_all_pass_is_survived() {
        let out = "running 2 tests\ntest a ... ok\ntest b ... ok\n\ntest result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n";
        assert_eq!(classify_test(Some(0), out), Verdict::Survived);
    }

    #[test]
    fn test_zero_tests_ran_is_invalid() {
        let out = "running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out\n";
        assert!(matches!(
            classify_test(Some(0), out),
            Verdict::Invalid { .. }
        ));
        assert!(matches!(
            classify_test(Some(0), ""),
            Verdict::Invalid { .. }
        ));
    }

    #[test]
    fn test_compile_error_is_invalid_even_with_nonzero_exit() {
        let out = "error[E0308]: mismatched types\n --> tests/x.rs:3:5\nerror: could not compile `factory-dispatcher` (test \"x\") due to 1 previous error\n";
        assert!(matches!(
            classify_test(Some(101), out),
            Verdict::Invalid { .. }
        ));
    }

    #[test]
    fn test_nonzero_exit_without_failed_test_is_invalid() {
        assert!(matches!(
            classify_test(Some(101), "error: no matching package\n"),
            Verdict::Invalid { .. }
        ));
    }

    #[test]
    fn indented_test_output_mentioning_error_is_not_a_compile_error() {
        let out = "running 1 test\ntest e ... FAILED\nthread panicked: error[E9999]: fake in a message\n\ntest result: FAILED. 0 passed; 1 failed; 0 ignored\n";
        assert!(matches!(
            classify_test(Some(101), out),
            Verdict::Killed { .. }
        ));
    }
}
