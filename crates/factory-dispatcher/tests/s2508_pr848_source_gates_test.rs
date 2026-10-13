// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-25.08 PR #848 fix-batch source gates.
//!
//! * CR-005: `BcIndexMigrationError::admission_failure_cause` must be an
//!   exhaustive `match` with NO catch-all arm, so a new error variant forces a
//!   deliberate `<cause>` token choice (compile error) instead of silently
//!   becoming `state_integrity`.
//! * CR-003: `executor::bc_index_migration_admission_precheck` had no
//!   production caller and, by discarding `MigrationAdmission::reservation`,
//!   leaked a reservation on the admitted path. Chosen fix: REMOVE it. Callers
//!   (tests) use `bc_index_migration_admission(..).verdict`. The gate asserts
//!   the function no longer exists in non-test source anywhere in the crate.

use std::path::{Path, PathBuf};

fn src_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn read_src(rel: &str) -> String {
    std::fs::read_to_string(src_root().join(rel)).expect("read source file")
}

/// Strip `//` line comments (incl. doc comments) so gates only see code.
fn strip_line_comments(src: &str) -> String {
    src.lines()
        .map(|l| match l.find("//") {
            Some(i) => &l[..i],
            None => l,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Body of `fn <name>` (brace-balanced) from comment-stripped source.
fn fn_body(code: &str, name: &str) -> Option<String> {
    let start = code.find(&format!("fn {name}"))?;
    let open = start + code[start..].find('{')?;
    let mut depth = 0usize;
    for (i, ch) in code[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(code[open..open + i + 1].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

/// True if any `match` arm in `body` is a catch-all: a bare `_` or a bare
/// lowercase binding pattern (e.g. `other =>`), optionally with a guard-less
/// form. Variant arms (`Type::Variant {..}`) are not catch-alls.
fn has_wildcard_arm(body: &str) -> bool {
    body.lines().any(|line| {
        let t = line.trim();
        let Some(idx) = t.find("=>") else {
            return false;
        };
        let pat = t[..idx].trim();
        pat == "_"
            || (!pat.is_empty()
                && pat.chars().all(|c| c.is_ascii_lowercase() || c == '_')
                && pat != "match")
            || pat.ends_with("| _")
    })
}

#[test]
fn test_S_25_08_CR005_admission_failure_cause_match_has_no_wildcard_arm() {
    let code = strip_line_comments(&read_src("shard_manager.rs"));
    let body = fn_body(&code, "admission_failure_cause")
        .expect("admission_failure_cause must exist in shard_manager.rs");
    assert!(
        body.contains("match self"),
        "admission_failure_cause must dispatch via `match self`; body: {body}"
    );
    assert!(
        !has_wildcard_arm(&body),
        "CR-005: admission_failure_cause has a catch-all arm; every \
         BcIndexMigrationError variant must be listed explicitly so a new variant \
         fails to compile. Body: {body}"
    );
}

/// Positive control: the detector flags the known-bad shape.
#[test]
fn test_S_25_08_CR005_control_detector_flags_wildcard_and_binding_catch_alls() {
    assert!(has_wildcard_arm(
        "match self {\n A::X => \"a\",\n _ => \"b\",\n}"
    ));
    assert!(has_wildcard_arm(
        "match self {\n A::X => \"a\",\n other => \"b\",\n}"
    ));
    assert!(has_wildcard_arm(
        "match self {\n A::X => \"a\",\n A::Y | _ => \"b\",\n}"
    ));
}

/// Negative control: the detector accepts an exhaustive, wildcard-free match.
#[test]
fn test_S_25_08_CR005_control_detector_accepts_exhaustive_match() {
    let ok = "match self {\n A::X { .. } => \"a\",\n A::Y(..) => \"b\",\n A::Z => \"c\",\n}";
    assert!(!has_wildcard_arm(ok));
}

/// The gate extractor itself finds a real function body (guards against the
/// CR-005 gate passing vacuously on an empty/misparsed body).
#[test]
fn test_S_25_08_CR005_control_fn_body_extraction_is_non_vacuous() {
    let code = strip_line_comments(&read_src("shard_manager.rs"));
    let body = fn_body(&code, "admission_failure_cause").expect("found");
    assert!(body.contains("InvalidToolUseId") && body.contains("\"io\""));
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).expect("read_dir").flatten() {
        let p = e.path();
        if p.is_dir() {
            rs_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn test_S_25_08_CR003_bc_index_migration_admission_precheck_is_removed() {
    let mut files = Vec::new();
    rs_files(&src_root(), &mut files);
    let offenders: Vec<String> = files
        .iter()
        .filter(|f| {
            strip_line_comments(&std::fs::read_to_string(f).unwrap())
                .contains("bc_index_migration_admission_precheck")
        })
        .map(|f| f.display().to_string())
        .collect();
    assert!(
        offenders.is_empty(),
        "CR-003: bc_index_migration_admission_precheck (no production caller; leaks \
         the reservation it creates) must be removed from non-comment source: {offenders:?}"
    );
}

/// Control: the shared core the precheck wrapped is still present and still
/// returns the reservation, so removal loses no behavior (and the scan above
/// is looking at the right files).
#[test]
fn test_S_25_08_CR003_control_shared_admission_core_still_exposed_with_reservation() {
    let code = strip_line_comments(&read_src("executor.rs"));
    assert!(code.contains("pub fn bc_index_migration_admission("));
    assert!(code.contains("pub reservation: Option<std::path::PathBuf>"));
}
