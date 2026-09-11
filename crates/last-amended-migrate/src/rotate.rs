//! Rotation subcommand orchestration (BC-10.13.001 PC5, PC6).

use crate::error::MigrateError;
use crate::migrate::MigrationMode;
use std::path::{Path, PathBuf};

/// Tool-default number of most-recent `changelog:` items to retain in the
/// source file when a `--keep-recent` count is not supplied
/// (BC-10.13.001 PC5 "all-but-the-most-recent-K by tool default").
pub const DEFAULT_KEEP_RECENT: usize = 20;

/// Outcome of a single rotation invocation.
#[derive(Debug, Clone)]
pub struct RotationReport {
    pub path: PathBuf,
    pub archive_path: PathBuf,
    /// Number of `changelog:` items moved out of `path` into the archive.
    /// `0` for a below-threshold no-op (EC-004).
    pub items_moved: usize,
    pub mutated: bool,
}

/// Derive the archive destination path: the nearest `.factory/` ancestor
/// directory of `path` (walking upward — `Path::ancestors` yields the most
/// specific ancestor first), joined with
/// `cycles/<cycle_name>/<file-basename>-changelog-archive.md` (PC5's own
/// literal naming convention). Falls back to a `.factory/` sibling of
/// `path`'s own parent directory when no `.factory` ancestor component is
/// found at all (defensive — every real target file lives under `.factory/`,
/// but `rotate_changelog` has no separate `factory_root` parameter to lean
/// on, unlike `migrate_all`).
fn resolve_archive_path(path: &Path, cycle_name: &str) -> Result<PathBuf, MigrateError> {
    let basename = path.file_stem().and_then(|s| s.to_str()).ok_or_else(|| {
        MigrateError::FrontmatterParse {
            path: path.to_path_buf(),
            reason: "cannot derive a file stem for the archive naming convention".to_string(),
        }
    })?;
    let archive_filename = format!("{basename}-changelog-archive.md");

    let factory_root = path
        .ancestors()
        .find(|a| a.file_name().is_some_and(|n| n == ".factory"))
        .map(Path::to_path_buf)
        .unwrap_or_else(|| {
            path.parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".factory")
        });

    Ok(factory_root
        .join("cycles")
        .join(cycle_name)
        .join(archive_filename))
}

/// Remove every line starting with `prefix` from `text` — used to drop a
/// pre-existing `changelog_archive:` discoverability-pointer line before
/// writing a fresh one, so repeated genuine rotations never accumulate
/// duplicate pointer lines.
fn remove_lines_with_prefix(text: &str, prefix: &str) -> String {
    text.split_inclusive('\n')
        .filter(|line| !line.starts_with(prefix))
        .collect()
}

/// Rewrite `raw`'s `changelog:` sequence to hold only `keep_items` (in
/// order), followed by a `changelog_archive:` discoverability pointer line
/// naming `archive_path` (PC5) — every other byte of `raw` is left
/// untouched.
fn rewrite_source_after_rotation(
    path: &Path,
    raw: &str,
    keep_items: &[String],
    archive_path: &Path,
) -> Result<String, MigrateError> {
    let (seq_start, seq_end) =
        crate::frontmatter::changelog_sequence_bounds(raw).ok_or_else(|| {
            MigrateError::FrontmatterParse {
                path: path.to_path_buf(),
                reason: "changelog: sequence not found while rewriting after rotation".to_string(),
            }
        })?;

    let mut new_seq = String::new();
    for item in keep_items {
        new_seq.push_str(item);
    }

    let tail = remove_lines_with_prefix(&raw[seq_end..], "changelog_archive:");
    // S-15.03 Windows-CI fix: `archive_path.display()` is a raw filesystem
    // path, not a pre-escaped YAML scalar body. On Windows it renders with
    // `\`-separated components, which are illegal unescaped inside a YAML
    // double-quoted scalar (YAML permits `\` only immediately before `\`,
    // `"`, `n`, `r`, `t`, or `xHH`) — route it through this crate's own
    // `escape::escape_raw_value` before embedding it.
    //
    // Deliberately NOT `escape::escape_value`: that function's "is this
    // already an escaped token" lookahead is designed for idempotently
    // re-processing prose text this tool may have escaped on a PRIOR run
    // (PC4) — a raw, always-freshly-computed filesystem path has no such
    // concern, and reusing that heuristic here is actively wrong. A `\`
    // path separator immediately followed by a component that starts with
    // `n`/`r`/`t`/`x`+hex/`\`/`"` (e.g. a cycle name like `test-cycle`, or a
    // username like `runner` — thoroughly ordinary on Windows) would
    // collide with `escape_value`'s recognized-escape-token lookahead and be
    // left unescaped, so strict YAML `safe_load` would silently decode it
    // back as an actual tab/newline/CR byte instead of the literal
    // backslash — silent path corruption, not a parse failure (see
    // `escape_raw_value`'s doc comment for the full analysis).
    let pointer_line = format!(
        "changelog_archive: \"{}\"\n",
        crate::escape::escape_raw_value(&archive_path.display().to_string())
    );

    let mut result = String::with_capacity(raw.len() + pointer_line.len());
    result.push_str(&raw[..seq_start]);
    result.push_str(&new_seq);
    result.push_str(&pointer_line);
    result.push_str(&tail);
    Ok(result)
}

/// Rotate `path`'s `changelog:` sequence using a caller-supplied,
/// pre-resolved `archive_path` directly — for callers that cannot derive an
/// archive path from a `cycle_name` (e.g. `BC-INDEX.md`, which is a catalog
/// artifact, not a cycle-scoped log file).
///
/// Identical behaviour to [`rotate_changelog`] in all other respects: moves
/// the oldest items past `keep_recent` verbatim into `archive_path`, removes
/// exactly those items from `path`, and leaves a `changelog_archive:`
/// discoverability pointer (BC-10.13.001 PC5). No-op (EC-004) when the
/// sequence does not exceed `keep_recent`. Creates `archive_path`'s parent
/// directory if it does not already exist (EC-005 precedent).
///
/// BC-1.18.009 Architecture Anchors: "a small, NAMED, bounded extension to
/// the primitive's path-resolution surface — a generalized `archive_path:
/// &Path` parameter... pre-computed by the dispatcher's B1 handler as the
/// FIXED, non-cycle, BC-INDEX-sibling path."
pub fn rotate_changelog_at(
    path: &Path,
    archive_path: &Path,
    keep_recent: usize,
    mode: MigrationMode,
) -> Result<RotationReport, MigrateError> {
    let doc = crate::frontmatter::parse_frontmatter(path)?;
    let total = doc.changelog_items_raw.len();

    if total <= keep_recent {
        // EC-004: below-threshold no-op.
        return Ok(RotationReport {
            path: path.to_path_buf(),
            archive_path: archive_path.to_path_buf(),
            items_moved: 0,
            mutated: false,
        });
    }

    let items_moved = total - keep_recent;
    // `changelog_items_raw` is newest-first: keep the newest `keep_recent`
    // items in the source, move the rest (the oldest) to the archive.
    let (keep_items, move_items) = doc.changelog_items_raw.split_at(keep_recent);

    if mode == MigrationMode::Check {
        return Ok(RotationReport {
            path: path.to_path_buf(),
            archive_path: archive_path.to_path_buf(),
            items_moved,
            mutated: false,
        });
    }

    if let Some(parent) = archive_path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| MigrateError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let mut archive_content = if archive_path.exists() {
        std::fs::read_to_string(archive_path).map_err(|source| MigrateError::Io {
            path: archive_path.to_path_buf(),
            source,
        })?
    } else {
        String::new()
    };

    // Inv-6 crash-recovery sentinel: a dotfile sibling of `archive_path`
    // written AFTER both the archive write and the source write succeed.
    // Its presence on a subsequent call with the same `archive_path` marks
    // the prior rotation as fully committed — meaning any tail-match is a
    // coincidental byte-identical second rotation (VP-125 scenario), not
    // a crash recovery. Its absence means the prior rotation never completed
    // (archive written, source write crashed) — crash-recovery dedup fires.
    let file_name = archive_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("archive");
    let sentinel_path = archive_path.with_file_name(format!(".{file_name}.written"));

    // Inv-6 crash-recovery tail-match dedup (BC-1.18.009 Invariant 6 /
    // ADR-051 §Decision 7 self-heal bullet): detect the crash scenario where a
    // prior rotation attempt completed the archive write but failed on the
    // source rewrite. On retry, `move_items` is byte-identical to the content
    // already present at the archive's tail. A tail-anchored `ends_with` suffix
    // comparison (NEVER `String::contains` — L-BB-D1179) combined with a
    // sentinel-absent check detects this and skips the redundant archive write,
    // preventing the 25→50 item doubling. The source rewrite still proceeds
    // so `mutated=true` is returned.
    //
    // The sentinel distinguishes crash recovery (sentinel absent) from a
    // normal second rotation that coincidentally produces byte-identical
    // `move_items` (sentinel present from completed prior rotation).
    let to_append: String = move_items.concat();
    let skip_archive_write = !archive_content.is_empty()
        && archive_content.ends_with(&to_append)
        && !sentinel_path.exists();

    if !skip_archive_write {
        // Boundary normalization: the last `changelog:` item in a frontmatter
        // document is captured verbatim by `changelog_items_raw`, and because
        // `frontmatter_bounds` sets `fm_end` at the `\n` character that
        // immediately precedes the closing `---` fence, that terminal newline
        // is NOT included in the last item's raw text. This means that if the
        // existing archive is non-empty (i.e., a prior rotation already
        // appended items to it), the last byte of the archive content may be a
        // non-newline character such as `"`. Appending the next rotation's
        // first `  - ` item directly would concatenate it mid-line, creating
        // an invalid YAML block sequence entry ("block sequence entries are not
        // allowed in this context"). Ensuring a separator newline before
        // appending is the correct fix: it is a no-op when the archive already
        // ends with `\n`, and it repairs the boundary only when needed.
        if !archive_content.is_empty() && !archive_content.ends_with('\n') {
            archive_content.push('\n');
        }
        archive_content.push_str(&to_append);
        // S-15.03 SEC-001 (BC-10.13.001 Invariant 4): validate the archive's
        // relocated `changelog:` sequence content parses cleanly before writing.
        crate::yaml_guard::validate_changelog_sequence_yaml(archive_path, &archive_content)?;
        // S-15.03 SEC-003: write-then-rename, not a direct in-place write.
        crate::atomic_write::write_atomic(archive_path, &archive_content)?;
    }

    let new_raw = rewrite_source_after_rotation(path, &doc.raw, keep_items, archive_path)?;
    // S-15.03 SEC-001: validate the rewritten source file's frontmatter
    // before writing it back.
    crate::yaml_guard::validate_frontmatter_yaml(path, &new_raw)?;
    // S-15.03 SEC-003: write-then-rename for the source rewrite too.
    crate::atomic_write::write_atomic(path, &new_raw)?;

    // Mark this rotation as fully committed. The sentinel's presence prevents
    // the next call's crash-recovery dedup from incorrectly classifying a
    // coincidental byte-identical second rotation as a crash recovery.
    crate::atomic_write::write_atomic(&sentinel_path, "")?;

    Ok(RotationReport {
        path: path.to_path_buf(),
        archive_path: archive_path.to_path_buf(),
        items_moved,
        mutated: true,
    })
}

/// Rotate `path`'s `changelog:` sequence: move the oldest items past
/// `keep_recent` verbatim into
/// `.factory/cycles/<cycle_name>/<file-basename>-changelog-archive.md`,
/// removing exactly those items from `path` and leaving a discoverability
/// pointer (BC-10.13.001 PC5).
///
/// No-op (EC-004) when the sequence does not exceed `keep_recent`. Creates
/// `.factory/cycles/<cycle_name>/` if it does not already exist (EC-005).
/// Every `changelog:` item's `date:`/`summary:` text is preserved verbatim —
/// only its location (source vs. archive) changes (PC5). Re-running rotation
/// immediately after a successful rotation, before the threshold is
/// exceeded again, is a verified-clean no-op (Invariant 2).
///
/// This is a thin wrapper around [`rotate_changelog_at`] that derives the
/// archive path from `cycle_name` using `resolve_archive_path`. Callers that
/// supply an explicit archive path (e.g. the BC-1.18.009 B1 gate in
/// `shard_manager.rs`) should call [`rotate_changelog_at`] directly.
pub fn rotate_changelog(
    path: &Path,
    cycle_name: &str,
    keep_recent: usize,
    mode: MigrationMode,
) -> Result<RotationReport, MigrateError> {
    let archive_path = resolve_archive_path(path, cycle_name)?;
    rotate_changelog_at(path, &archive_path, keep_recent, mode)
}
