//! `changelog:` sequence handling (BC-10.13.001 PC1) and the BC-5.45.001 PC2
//! prepend discipline this tool's migration output must conform to
//! (BC-10.13.001 Invariant 4).

use crate::frontmatter::FrontmatterDoc;

/// One `changelog:` sequence item, as constructed by this tool when
/// prepending a newly displaced `last_amended` entry during migration —
/// distinct from `FrontmatterDoc::changelog_items_raw`, which holds
/// pre-existing items' raw text verbatim for lossless rotation (PC5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangelogItem {
    /// `date:` field. Nominally `YYYY-MM-DD`, but NOT guaranteed to be
    /// date-shaped: a PC7-recovered legacy entry that doesn't match the
    /// `"{date} ({version}) — {text}"` convention can carry an arbitrary
    /// leading token here (e.g. a decision-reference prefix like
    /// `D-1149:`) — MUST be passed through `crate::escape::escape_value`
    /// when `crate::escape::needs_escaping` reports it needs it, same as
    /// `summary`/`version` (S-15.03 B1).
    pub date: String,
    /// `version:` field, when the target file's own convention includes
    /// one. MUST be passed through `crate::escape::escape_value` when
    /// `crate::escape::needs_escaping` reports it needs it (S-15.03
    /// SEC-001).
    pub version: Option<String>,
    /// `summary:`/`change:` field text — MUST be passed through
    /// `crate::escape::escape_value` before being written (PC3).
    pub summary: String,
}

/// Outcome of `ensure_changelog_field` (BC-10.13.001 PC1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangelogMutation {
    /// `changelog:` was absent and has been added (expected only for
    /// `STORY-INDEX.md`, EC-002).
    Added,
    /// `changelog:` already existed — verified no-op.
    AlreadyPresent,
    /// Target is `STATE.md`, which never gains a `changelog:` field
    /// (ADR-049 Decision 4, EC-006) — not attempted.
    SkippedStateFile,
}

/// Ensure `doc` carries a `changelog:` top-level sequence, adding an empty
/// one only when absent and the file is not `STATE.md` (BC-10.13.001 PC1).
pub fn ensure_changelog_field(doc: &mut FrontmatterDoc, is_state_file: bool) -> ChangelogMutation {
    if is_state_file {
        return ChangelogMutation::SkippedStateFile;
    }
    if doc.changelog_present {
        return ChangelogMutation::AlreadyPresent;
    }
    match crate::frontmatter::frontmatter_bounds(&doc.raw) {
        Ok((_, fm_end)) => {
            // `fm_end` is the byte offset of the `\n` that both terminates the
            // last existing frontmatter field's line AND immediately precedes
            // the closing `---` fence — insert the new key right after that
            // newline (i.e. before `---`), so it lands as the final
            // frontmatter field, on its own line.
            doc.raw.insert_str(fm_end + 1, "changelog:\n");
        }
        Err(_) => {
            // Defensive fallback for an already-malformed `raw` (should not
            // occur on a `FrontmatterDoc` that parsed successfully or was
            // hand-constructed with a valid fence, per this crate's own
            // invariant) — never silently drop the mutation.
            doc.raw.push_str("\nchangelog:\n");
        }
    }
    doc.changelog_present = true;
    ChangelogMutation::Added
}

/// Prepend exactly one newly displaced entry to `doc`'s `changelog:`
/// sequence, newest-first, leaving every existing item byte-for-byte
/// untouched (BC-5.45.001 PC2 discipline that this tool's own migration
/// output must satisfy per BC-10.13.001 Invariant 4).
pub fn prepend_changelog_item(doc: &mut FrontmatterDoc, item: ChangelogItem) {
    let block = render_item_block(&item);
    const KEY_MARKER: &str = "changelog:\n";
    match doc.raw.find(KEY_MARKER) {
        Some(pos) => {
            doc.raw.insert_str(pos + KEY_MARKER.len(), &block);
        }
        None => {
            // Defensive fallback: `changelog:` key not yet present at all.
            // `ensure_changelog_field` should always have run first in this
            // crate's own orchestration, but an external caller of this
            // public primitive may not have — bootstrap the key in the same
            // write rather than silently dropping the item.
            match crate::frontmatter::frontmatter_bounds(&doc.raw) {
                Ok((_, fm_end)) => {
                    let mut combined = String::with_capacity(KEY_MARKER.len() + block.len());
                    combined.push_str(KEY_MARKER);
                    combined.push_str(&block);
                    doc.raw.insert_str(fm_end + 1, &combined);
                }
                Err(_) => {
                    doc.raw.push_str("\nchangelog:\n");
                    doc.raw.push_str(&block);
                }
            }
        }
    }
    doc.changelog_items_raw.insert(0, block);
    doc.changelog_present = true;
}

/// Render one `ChangelogItem` as its raw YAML sequence-item block text —
/// `"  - date: \"...\"\n    version: \"...\"\n    change: \"...\"\n"` (the
/// `version:` line omitted when `item.version` is `None`). `item.date`,
/// `item.version`, and `item.summary` are all written verbatim into their own
/// double-quoted YAML scalar: callers are responsible for having already
/// passed each through `crate::escape::escape_value` when
/// `crate::escape::needs_escaping` reports it needs it (see
/// `ChangelogItem::summary`'s own doc comment, and S-15.03 B1: `item.date` is
/// NOT guaranteed to be an actual `YYYY-MM-DD` token — a PC7-recovered
/// historical entry that predates the `"{date} ({version}) — {text}"`
/// convention can carry an arbitrary leading token, such as a decision-
/// reference prefix like `D-1149:`, as its "date" — so this field is quoted
/// unconditionally, exactly like `version`/`summary`, rather than emitted as
/// a bare unquoted plain scalar. An unquoted `date: D-1149:` is invalid YAML
/// (an unescaped colon inside a plain scalar reads as a nested mapping
/// indicator); a quoted `date: "D-1149:"` is not.
fn render_item_block(item: &ChangelogItem) -> String {
    let mut block = format!("  - date: \"{}\"\n", item.date);
    if let Some(version) = &item.version {
        block.push_str(&format!("    version: \"{version}\"\n"));
    }
    block.push_str(&format!("    change: \"{}\"\n", item.summary));
    block
}
