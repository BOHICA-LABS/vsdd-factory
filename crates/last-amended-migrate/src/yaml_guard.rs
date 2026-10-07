//! Pre-write strict YAML validity gate (BC-10.13.001 Invariant 4 / PC3;
//! S-15.03 SEC-001).
//!
//! `MigrateError::InvalidYamlProduced` was declared in `src/error.rs` from
//! this crate's stub stage onward (per its own doc comment, "intended to
//! gate an `safe_load`-validity check before writing") but was never
//! constructed anywhere — dead code, no actual pre-write validation ran. This
//! module closes that gap: every content this tool is about to write to one
//! of the 5 BC-10.13.001 target files, or to a `rotate_changelog` archive
//! file, is parsed under strict YAML `safe_load` semantics FIRST, via
//! `serde_norway` — an independent, general-purpose YAML parser, deliberately
//! NOT the hand-rolled bounded scan `src/frontmatter.rs` uses for its
//! production hot-path parse (see that module's own doc comment for why that
//! parse must stay hand-rolled: bounded-resource safety against the D-1149
//! calibration ceiling). Using a real YAML library here, on the OUTPUT side
//! only, for a bounded-size validity check (never the multi-hundred-KB
//! hot-path read), carries none of that risk.
//!
//! Returns `MigrateError::InvalidYamlProduced` instead of allowing the write
//! to proceed when validation fails — this is a defense-in-depth backstop:
//! `src/escape.rs`'s control-character + quote escaping (SEC-001's other
//! half) is expected to make every value this tool itself constructs valid,
//! but any gap in that escaping (present or future) is still caught here,
//! before a corrupt file ever reaches disk.

use crate::error::MigrateError;
use std::path::Path;

/// Deserialization target for a frontmatter-block validity check — mirrors
/// `tests/common::MinimalFrontmatter`'s shape but lives in production code
/// (this module), not test-only code, since it is now used as a real gate
/// rather than purely for test verification. Extra frontmatter fields are
/// ignored by default serde struct deserialization (no `deny_unknown_fields`),
/// so this only needs the 2 fields this tool operates on.
#[derive(serde::Deserialize)]
struct MinimalFrontmatter {
    #[allow(dead_code)]
    last_amended: String,
    #[serde(default)]
    #[allow(dead_code)]
    changelog: Option<Vec<serde_norway::Value>>,
}

/// Deserialization target for a bare `changelog:` sequence validity check
/// (used for `rotate_changelog`'s archive-file write, which has no
/// `last_amended` field or `---` frontmatter fences of its own — just the
/// relocated `changelog:` sequence items).
#[derive(serde::Deserialize)]
struct ChangelogOnly {
    #[allow(dead_code)]
    changelog: Vec<serde_norway::Value>,
}

/// Extract the `---`-fenced frontmatter block (without the fences) from a
/// full file's raw content, so `serde_norway::from_str` parses only the YAML
/// region, not the markdown body below it.
fn frontmatter_block(file_content: &str) -> Option<&str> {
    let after_open = file_content.strip_prefix("---\n")?;
    let end = after_open.find("\n---")?;
    Some(&after_open[..end])
}

/// Validate that `raw` (a full target-file's content, as this tool is about
/// to write it) has a well-formed `---`-fenced frontmatter block that parses
/// cleanly under strict YAML `safe_load` semantics (BC-10.13.001 Invariant
/// 4). Called immediately before every `std::fs::write` of a frontmatter
/// document in `migrate.rs::migrate_file` and `rotate.rs::rotate_changelog`.
pub fn validate_frontmatter_yaml(path: &Path, raw: &str) -> Result<(), MigrateError> {
    let block = frontmatter_block(raw).ok_or_else(|| MigrateError::InvalidYamlProduced {
        reason: format!(
            "{}: produced content has no valid --- frontmatter fence to validate",
            path.display()
        ),
    })?;
    serde_norway::from_str::<MinimalFrontmatter>(block)
        .map(|_| ())
        .map_err(|source| MigrateError::InvalidYamlProduced {
            reason: format!("{}: {source}", path.display()),
        })
}

/// Validate that `items_raw` (the concatenated raw `changelog:` sequence-item
/// blocks this tool is about to write to a rotation archive file) parses
/// cleanly as a YAML sequence under a synthetic `changelog:` wrapper key —
/// the archive file itself has no frontmatter fences of its own (BC-10.13.001
/// PC5), so this validates the sequence shape directly rather than via
/// `validate_frontmatter_yaml`.
pub fn validate_changelog_sequence_yaml(path: &Path, items_raw: &str) -> Result<(), MigrateError> {
    let synthetic = format!("changelog:\n{items_raw}");
    serde_norway::from_str::<ChangelogOnly>(&synthetic)
        .map(|_| ())
        .map_err(|source| MigrateError::InvalidYamlProduced {
            reason: format!("{}: {source}", path.display()),
        })
}
