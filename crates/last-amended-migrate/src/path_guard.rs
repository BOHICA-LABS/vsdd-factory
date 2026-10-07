//! CLI-argument path allowlisting (S-15.03 SEC-002, CWE-73 external control
//! of file name/path).
//!
//! `cli.rs`'s `Migrate { path: Some(p), .. }`, `Rotate { path, .. }`, and
//! `Command::Register { registry }` used to accept an arbitrary `PathBuf`
//! with zero validation — no canonicalization, no check the path resolves
//! under the intended scope, no rejection of `..` traversal. Only
//! `migrate_all`'s hardcoded `TARGET_FILES` loop (never user-path-controlled)
//! had any allowlist. This module closes that gap for every CLI-boundary
//! path argument, BEFORE the corresponding subcommand handler performs any
//! read/write of the target's *content*.
//!
//! # Why the allowlist lives here, not in the library functions
//!
//! `migrate_file`, `rotate_changelog`, and `register_artifact_paths` remain
//! deliberately path-agnostic: this crate's own Red Gate test suite exercises
//! them directly against arbitrary tempdir fixture paths (POLICY 11 — tests
//! must call the crate's real public API, not a reimplementation, and a
//! tempdir is the only safe place for a test to write). Baking an allowlist
//! into the library functions would make them untestable without physically
//! recreating `.factory/`'s exact directory shape inside every tempdir
//! fixture. The CLI boundary (`cli.rs::run`) is where an argument actually
//! originates from an external, potentially-untrusted source (an operator's
//! shell invocation, or an orchestrator-templated command line) — so this is
//! the correct, and sufficient, enforcement point.

use crate::error::MigrateError;
use crate::migrate::TARGET_FILES;
use std::path::{Path, PathBuf};

fn canonicalize(path: &Path) -> Result<PathBuf, MigrateError> {
    std::fs::canonicalize(path).map_err(|source| MigrateError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// `migrate --path` allowlist: the canonicalized path must be exactly one of
/// the 5 `TARGET_FILES` entries, resolved relative to `factory_root` — the
/// same resolution `migrate_all` already performs unconditionally, just
/// verified here for the user-supplied single-file form too.
pub fn validate_migrate_path(path: &Path, factory_root: &Path) -> Result<(), MigrateError> {
    let canonical = canonicalize(path)?;
    for rel in TARGET_FILES {
        if let Ok(candidate) = canonicalize(&factory_root.join(rel))
            && candidate == canonical
        {
            return Ok(());
        }
    }
    Err(MigrateError::PathNotAllowed {
        path: path.to_path_buf(),
        reason: format!(
            "--path must resolve to one of the 5 BC-10.13.001 target files \
             under --factory-root ({}); this path is outside that allowlist",
            factory_root.display()
        ),
    })
}

/// `rotate --path` allowlist: the canonicalized path must reside under a
/// `.factory/` ancestor directory (BC-10.13.001 PC5's own governed scope —
/// every real rotation target lives under `.factory/`).
pub fn validate_rotate_path(path: &Path) -> Result<(), MigrateError> {
    let canonical = canonicalize(path)?;
    let under_factory = canonical
        .ancestors()
        .any(|a| a.file_name().is_some_and(|n| n == ".factory"));
    if under_factory {
        Ok(())
    } else {
        Err(MigrateError::PathNotAllowed {
            path: path.to_path_buf(),
            reason: "--path must reside under a .factory/ ancestor directory".to_string(),
        })
    }
}

/// `register --registry` allowlist: the canonicalized path's trailing
/// component SHAPE — not merely its basename — must match
/// `plugins/vsdd-factory/config/artifact-path-registry.yaml` (S-15.03
/// AC-006). A basename-only check (this function's pre-S-15.03-pr-review
/// form) would accept ANY file named `artifact-path-registry.yaml`
/// anywhere on the filesystem, with none of `validate_migrate_path`'s
/// rigor about WHERE it lives; this checks the full 4-component relative
/// suffix instead, matching that same rigor. As with `validate_migrate_path`,
/// this only requires the trailing shape to match — it does not require an
/// absolute root — so this function stays testable against an arbitrary
/// tempdir fixture that recreates just that trailing directory structure
/// (see this module's own doc comment on why the CLI boundary, not the
/// library function, is where allowlisting belongs).
pub fn validate_registry_path(path: &Path) -> Result<(), MigrateError> {
    let canonical = canonicalize(path)?;
    const EXPECTED_SUFFIX: [&str; 4] = [
        "plugins",
        "vsdd-factory",
        "config",
        "artifact-path-registry.yaml",
    ];
    let components: Vec<&std::ffi::OsStr> = canonical.iter().collect();
    let shape_ok = components.len() >= EXPECTED_SUFFIX.len()
        && components[components.len() - EXPECTED_SUFFIX.len()..]
            .iter()
            .zip(EXPECTED_SUFFIX.iter())
            .all(|(actual, expected)| *actual == std::ffi::OsStr::new(expected));
    if shape_ok {
        Ok(())
    } else {
        Err(MigrateError::PathNotAllowed {
            path: path.to_path_buf(),
            reason: "--registry must resolve to a \
                plugins/vsdd-factory/config/artifact-path-registry.yaml-shaped \
                path (matching the real repository location) — a file that \
                merely shares that basename in a different directory is not \
                accepted"
                .to_string(),
        })
    }
}
