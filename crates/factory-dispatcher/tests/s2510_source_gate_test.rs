// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! S-25.10 RED tests -- SOURCE GATES (sibling sweep, TD-VSDD-060) for AC-001: the shared
//! module is the ONLY implementation of the intent-log format and the B2 originals are
//! DELETED, not wrapped.
//!
//! Authority: ADR-054 v1.0 Decision 4 ("`INTENT_LOG_RECORD_V1` / `END_INTENT_LOG_RECORD`
//! outside this module and its tests" is a literal-shell grep gate in S-25.10); BC-1.18.011
//! v1.21 Postcondition 15(a); story S-25.10 AC-001, AC-005 (`Path::to_str`, no lossy
//! conversion at plan-build), Architecture Compliance Rule 1 and the T-9 verifier checks.
//! Style: the S-25.09 `..._EC033_no_dot_factory_join_literal_outside_the_resolver_source_gate`.
//!
//! | Story AC | Test |
//! |---|---|
//! | AC-001 | `..._15a_intent_log_markers_only_in_shared_module_source_gate` |
//! | AC-001 | `..._15a_four_b2_format_originals_are_deleted_source_gate` |
//! | AC-001 / AC-008 | `..._15a_fs_append_is_replaced_by_append_durable_and_truncate_durable_in_every_impl_source_gate` |
//! | AC-001 / AC-011 | `..._15a_migration_code_uses_the_shared_module_and_never_discards_an_append_source_gate` |
//! | AC-005 | `..._EC067_plan_construction_never_converts_paths_lossily_source_gate` |
//!
//! Every gate below fails today (the module does not exist and `shard_manager.rs` still holds
//! the originals); none passes vacuously. This file needs nothing from the not-yet-existing
//! module, so it compiles and fails by ASSERTION.

use std::path::{Path, PathBuf};

use regex::Regex;

#[path = "s2510_support/mod.rs"]
mod support;

const MARKERS: [&str; 2] = ["INTENT_LOG_RECORD_V1", "END_INTENT_LOG_RECORD"];

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn crates_dir() -> PathBuf {
    manifest_dir().parent().expect("crates/").to_path_buf()
}

/// Every `.rs` file under `crates/` (skipping `target`), as (path, text).
fn all_crate_sources() -> Vec<(PathBuf, String)> {
    fn walk(dir: &Path, out: &mut Vec<(PathBuf, String)>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n == "target") {
                    continue;
                }
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs")
                && let Ok(text) = std::fs::read_to_string(&p)
            {
                out.push((p, text));
            }
        }
    }
    let mut out = Vec::new();
    walk(&crates_dir(), &mut out);
    out
}

fn is_s2510_suite_file(p: &Path) -> bool {
    p.file_name()
        .is_some_and(|n| n.to_string_lossy().starts_with("s2510_"))
        || p.components().any(|c| c.as_os_str() == "s2510_support")
}

fn module_path() -> PathBuf {
    manifest_dir().join("src/shard_manager/intent_log.rs")
}

fn rel(p: &Path) -> String {
    p.strip_prefix(crates_dir())
        .unwrap_or(p)
        .display()
        .to_string()
}

// ===========================================================================
// AC-001: markers
// ===========================================================================

/// BC-1.18.011 Postcondition 15(a): no source or test under `crates/` spells
/// `INTENT_LOG_RECORD_V1` / `END_INTENT_LOG_RECORD` outside `shard_manager/intent_log.rs`
/// and the S-25.10 suite itself; and the module exists and DOES own both markers.
#[test]
fn test_BC_1_18_011_15a_intent_log_markers_only_in_shared_module_source_gate() {
    let module = module_path();
    let module_text = std::fs::read_to_string(&module).unwrap_or_else(|e| {
        panic!(
            "AC-001 / ADR-054 Decision 4: {} must exist and own the format ({e})",
            module.display()
        )
    });
    for m in MARKERS {
        assert!(
            module_text.contains(m),
            "intent_log.rs must define the marker `{m}`"
        );
    }
    let offenders: Vec<String> = all_crate_sources()
        .into_iter()
        .filter(|(p, _)| *p != module && !is_s2510_suite_file(p))
        .filter(|(_, text)| MARKERS.iter().any(|m| text.contains(m)))
        .map(|(p, text)| {
            let which: Vec<&str> = MARKERS
                .iter()
                .copied()
                .filter(|m| text.contains(m))
                .collect();
            format!("{} spells {which:?}", rel(&p))
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "the intent-log format markers must appear ONLY in shard_manager/intent_log.rs and its \
         tests (A-6: a format shared by copy drifts); offenders:\n  - {}",
        offenders.join("\n  - ")
    );
}

// ===========================================================================
// AC-001: the four B2 originals are deleted (not wrapped); no stale call sites
// ===========================================================================

/// ADR-054 Decision 4 / story AC-001: `append_intent_log_record`, `intent_log_checksum_input`,
/// `read_intent_log`, `parse_intent_log_block` and the old `IntentLogRecord` /
/// `IntentLogRecordType` are DELETED -- and no definition or CALL SITE of any of them
/// survives anywhere under `crates/` (production code or test; TD-VSDD-060 sibling sweep).
/// (`decide_intent_log_recovery` / `IntentLogRecoveryDecision` stay until S-25.11.)
#[test]
fn test_BC_1_18_011_15a_four_b2_format_originals_are_deleted_source_gate() {
    let names = Regex::new(
        r"\b(append_intent_log_record|intent_log_checksum_input|read_intent_log|parse_intent_log_block|IntentLogRecord|IntentLogRecordType|INTENT_LOG_RECORD_START|INTENT_LOG_RECORD_END|intent_log_record_type_str)\b",
    )
    .unwrap();
    let mut hits = Vec::new();
    for (p, text) in all_crate_sources() {
        if is_s2510_suite_file(&p) {
            continue;
        }
        for (n, line) in text.lines().enumerate() {
            // Historical prose in comments is not a definition or a call site.
            let code = line.split("//").next().unwrap_or("");
            if let Some(m) = names.find(code) {
                hits.push(format!("{}:{}: `{}`", rel(&p), n + 1, m.as_str()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "the four B2 format originals (and the old record types/constants) must be DELETED, not \
         wrapped, with every call site converted; {} remaining reference(s):\n  - {}",
        hits.len(),
        hits.iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n  - ")
    );
}

// ===========================================================================
// AC-008 sibling sweep: every `impl Fs for` carries the two new methods, none keeps `append`
// ===========================================================================

/// ADR-054 "`Fs` seam additions": `append_durable` and `truncate_durable` REPLACE the old
/// `append`. Every `impl Fs for <X>` anywhere under `crates/` (StdFs, the Kani model in
/// `obl1_kani_proofs.rs`, every test double) defines both and none still defines `append`;
/// the trait itself declares both and no `fn append`.
#[test]
fn test_BC_1_18_011_15a_fs_append_is_replaced_by_append_durable_and_truncate_durable_in_every_impl_source_gate()
 {
    let impl_re = Regex::new(r"\bimpl\s+(?:[\w:]+::)?Fs\s+for\b").unwrap();
    let old_append = Regex::new(r"\bfn\s+append\s*\(\s*&self\b").unwrap();
    let mut failures = Vec::new();
    let mut impls_seen = 0usize;
    for (p, text) in all_crate_sources() {
        if is_s2510_suite_file(&p) {
            continue;
        }
        let n_impls = impl_re.find_iter(&text).count();
        if n_impls == 0 {
            continue;
        }
        impls_seen += n_impls;
        let n_append_durable = text.matches("fn append_durable(").count();
        let n_truncate = text.matches("fn truncate_durable(").count();
        if n_append_durable < n_impls || n_truncate < n_impls {
            failures.push(format!(
                "{}: {n_impls} `impl Fs for` but {n_append_durable} `fn append_durable(` and \
                 {n_truncate} `fn truncate_durable(`",
                rel(&p)
            ));
        }
        if old_append.is_match(&text) {
            failures.push(format!(
                "{}: still defines the removed `fn append(&self, ..)`",
                rel(&p)
            ));
        }
    }
    // The trait declaration lives in migration_fs.rs.
    let fs_src =
        std::fs::read_to_string(manifest_dir().join("src/shard_manager/migration_fs.rs")).unwrap();
    let trait_part = fs_src
        .split("pub trait Fs")
        .nth(1)
        .and_then(|rest| rest.split("\n}\n").next())
        .expect("migration_fs.rs must declare `pub trait Fs`");
    if !trait_part.contains("fn append_durable(") || !trait_part.contains("fn truncate_durable(") {
        failures.push(
            "`pub trait Fs` must declare `fn append_durable(` and `fn truncate_durable(`".into(),
        );
    }
    if old_append.is_match(trait_part) {
        failures.push("`pub trait Fs` must no longer declare `fn append(&self, ..)`".into());
    }
    if !trait_part.contains("-> Result<bool,") {
        failures.push("`Fs::append_durable` must return `Result<bool, _>` (created)".into());
    }
    assert!(
        impls_seen >= 3,
        "scanner sanity: expected the StdFs, Kani and test-double impls, saw {impls_seen}"
    );
    assert!(
        failures.is_empty(),
        "every Fs implementation must move to append_durable / truncate_durable:\n  - {}",
        failures.join("\n  - ")
    );
}

// ===========================================================================
// AC-001 / AC-011: the migration code uses the module and never discards an append
// ===========================================================================

/// `shard_manager.rs` declares `pub mod intent_log;`, calls the module's reader and writer,
/// and no `let _ =` discards an intent-log append / batch / open (a dropped append is the
/// TD-VSDD-059 paper-fix class; the WAL boundary is only reached when the append returned Ok).
#[test]
fn test_BC_1_18_011_15a_migration_code_uses_the_shared_module_and_never_discards_an_append_source_gate()
 {
    let prod_owned = support::production_source("src/shard_manager.rs");
    let prod = prod_owned.as_str();
    let mut failures = Vec::new();
    if !prod.contains("pub mod intent_log;") {
        failures.push("shard_manager.rs must declare `pub mod intent_log;`".to_string());
    }
    for needle in ["IntentLogWriter::open(", ".append_batch("] {
        if !prod.contains(needle) {
            failures.push(format!(
                "shard_manager.rs production code must call `{needle}`"
            ));
        }
    }
    let discard = Regex::new(
        r"(?s)let\s+_\s*=\s*[^;]*?(append_durable|append_batch|IntentLogWriter|truncate_durable)",
    )
    .unwrap();
    if let Some(m) = discard.find(prod) {
        failures.push(format!(
            "a discarded intent-log operation: `{}`",
            m.as_str().replace('\n', " ")
        ));
    }
    // The converted DONE append must not be a bare `let _ = ...append...` either; and no
    // `.ok();` swallow of a writer result.
    let swallow =
        Regex::new(r"(?s)(append_batch|IntentLogWriter::open)\([^;]*?\)\s*\.ok\(\)\s*;").unwrap();
    if swallow.is_match(prod) {
        failures.push("an intent-log writer result is swallowed with `.ok();`".to_string());
    }
    assert!(
        failures.is_empty(),
        "shared-module wiring:\n  - {}",
        failures.join("\n  - ")
    );
}

// ===========================================================================
// AC-005: no lossy path conversion at plan construction
// ===========================================================================

/// ADR-054 Decision 1.9 step 1 / story AC-005: plan paths are validated through
/// `Path::to_str` (a non-UTF-8 path is rejected, never rewritten). Every construction of a
/// plan element (`PendingCanonicalMove { .. }`, the OLD spelling this story keeps) in
/// `shard_manager.rs` production code must therefore NOT convert with `to_string_lossy`.
#[test]
fn test_BC_1_18_011_EC067_plan_construction_never_converts_paths_lossily_source_gate() {
    let prod_owned = support::production_source("src/shard_manager.rs");
    let prod = prod_owned.as_str();
    let lit = Regex::new(r"(?s)PendingCanonicalMove\s*\{[^{}]*\}").unwrap();
    let mut offenders = Vec::new();
    for m in lit.find_iter(prod) {
        let body = m.as_str();
        if body.contains("to_string_lossy") {
            let line = prod[..m.start()].matches('\n').count() + 1;
            offenders.push(format!(
                "shard_manager.rs:{line}: {}",
                body.replace('\n', " ")
            ));
        }
    }
    assert!(
        offenders.is_empty(),
        "plan construction must use Path::to_str (reject non-UTF-8), never to_string_lossy; \
         {} lossy construction site(s):\n  - {}",
        offenders.len(),
        offenders.join("\n  - ")
    );
}
