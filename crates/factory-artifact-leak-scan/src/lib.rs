//! factory-artifact-leak-scan — content-based factory-artifact leak detector (#515).
//!
//! Rust replacement for the retired `bin/factory-artifact-leak-scan.sh`
//! (POLICY 21 `no_new_shell_scripts`: new tooling must be platform-agnostic).
//! The detection contract is unchanged from the shell implementation, with the
//! #729 review fixes carried forward:
//!
//! A tracked file is a leaked factory artifact iff ALL hold:
//!   1. its path is NOT under a `.factory/` (or `.factory-project/`) directory;
//!   2. its path is NOT plugin machinery (`templates/`, `tests/`, `skills/`,
//!      `rules/` under `plugins/vsdd-factory/` carry `document_type:`
//!      frontmatter as examples/fixtures, not live artifacts);
//!   3. its frontmatter `document_type` is a factory-produced type — declared
//!      by some template under `${PLUGIN_ROOT}/templates/`, searched
//!      RECURSIVELY (#729 M1: subdirectory templates like
//!      `adversary-prompt-templates/*.md` are part of the universe);
//!   4. its `(document_type, path)` pair is NOT exempted by
//!      [`PRODUCT_TRACKED_HOMES`] — product-shipped doctypes are exempt ONLY
//!      under their canonical home directory (#729 M2: the exemption is
//!      path-scoped, not doctype-global).
//!
//! Frontmatter parsing anchors the opening fence to line 1 (#729 M3): a `---`
//! thematic break mid-document no longer opens a phantom frontmatter block,
//! and non-frontmatter files cost exactly one line of read.

use std::collections::BTreeSet;
use std::io::BufRead;
use std::path::{Path, PathBuf};

/// Factory doctypes the project intentionally tracks on the product branch,
/// each exempt ONLY under its canonical home directory (path prefix). These
/// are exactly the template-backed `document_type` values with NO entry in
/// the artifact-path-registry (no `.factory/` canonical home). Extend only
/// with the same evidence, with justification in the PR.
pub const PRODUCT_TRACKED_HOMES: &[(&str, &str)] = &[
    ("demo-evidence-report", "docs/demo-evidence/"),
    ("demo-evidence-index", "docs/demo-evidence/"),
];

/// A leaked factory artifact: its doctype and repo-relative tracked path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leak {
    pub doctype: String,
    pub path: String,
}

/// Extract `document_type:` from a file's FIRST frontmatter block.
///
/// #729 M3 contract: the opening `---` fence MUST be line 1 (trailing
/// whitespace tolerated). Anything else returns `None` after reading a single
/// line. Within the block, the first `document_type:` line wins; the scan
/// stops at the closing fence. A block left unclosed at EOF still counts
/// (sensitivity-preserving: this is a leak GUARD — prefer flagging).
pub fn frontmatter_doctype<R: BufRead>(reader: R) -> Option<String> {
    let mut lines = reader.lines();

    let first = lines.next()?.ok()?;
    if !is_fence(&first) {
        return None;
    }

    for line in lines {
        let line = line.ok()?;
        if is_fence(&line) {
            return None; // closing fence before any document_type
        }
        if let Some(rest) = line.strip_prefix("document_type:") {
            let value: String = rest.chars().filter(|c| *c != '"' && *c != '\'').collect();
            let value = value.trim();
            if value.is_empty() {
                return None;
            }
            return Some(value.to_string());
        }
    }
    None
}

fn is_fence(line: &str) -> bool {
    let t = line.trim_end();
    t == "---"
}

/// Read a file's frontmatter doctype; I/O errors and non-UTF-8 content are
/// treated as "no doctype" (the guard scans only text artifacts).
pub fn file_doctype(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    frontmatter_doctype(std::io::BufReader::new(file))
}

/// Build the factory doctype universe from every `*.md` template under `dir`,
/// RECURSIVELY (#729 M1). Returns a sorted set for deterministic behavior.
pub fn collect_factory_doctypes(dir: &Path) -> std::io::Result<BTreeSet<String>> {
    let mut doctypes = BTreeSet::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d)? {
            let entry = entry?;
            let path = entry.path();
            let ft = entry.file_type()?;
            if ft.is_dir() {
                stack.push(path);
            } else if ft.is_file()
                && path.extension().is_some_and(|e| e == "md")
                && let Some(dt) = file_doctype(&path)
            {
                doctypes.insert(dt);
            }
        }
    }
    Ok(doctypes)
}

/// Skip rule 1: any path with a `.factory` / `.factory-project` component is
/// the artifact worktree, not the product tree.
pub fn in_factory_worktree(rel: &str) -> bool {
    rel.split('/')
        .any(|c| c == ".factory" || c == ".factory-project")
}

/// Skip rule 2: plugin machinery whose frontmatter is examples/fixtures.
pub fn is_plugin_machinery(rel: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "plugins/vsdd-factory/templates/",
        "plugins/vsdd-factory/tests/",
        "plugins/vsdd-factory/skills/",
        "plugins/vsdd-factory/rules/",
    ];
    PREFIXES.iter().any(|p| rel.starts_with(p))
}

/// Exemption rule 4 (#729 M2): a product-tracked doctype is exempt iff the
/// path sits under that doctype's canonical home. Anywhere else it is a leak.
pub fn is_product_tracked_at_home(doctype: &str, rel: &str) -> bool {
    PRODUCT_TRACKED_HOMES
        .iter()
        .any(|(dt, home)| *dt == doctype && rel.starts_with(home))
}

/// Scan `tracked_md_paths` (repo-relative, as emitted by `git ls-files`)
/// against the doctype universe. `repo_root` anchors file reads.
pub fn scan(
    repo_root: &Path,
    tracked_md_paths: &[String],
    factory_doctypes: &BTreeSet<String>,
) -> Vec<Leak> {
    let mut leaks = Vec::new();
    for rel in tracked_md_paths {
        if !rel.ends_with(".md") {
            continue;
        }
        if in_factory_worktree(rel) || is_plugin_machinery(rel) {
            continue;
        }
        let abs: PathBuf = repo_root.join(rel);
        if !abs.is_file() {
            continue;
        }
        let Some(dt) = file_doctype(&abs) else {
            continue;
        };
        if !factory_doctypes.contains(&dt) {
            continue;
        }
        if is_product_tracked_at_home(&dt, rel) {
            continue;
        }
        leaks.push(Leak {
            doctype: dt,
            path: rel.clone(),
        });
    }
    leaks
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn doctype_of(s: &str) -> Option<String> {
        frontmatter_doctype(Cursor::new(s))
    }

    #[test]
    fn test_parses_doctype_from_line1_frontmatter() {
        let s = "---\ndocument_type: red-gate-log\nstory_id: S-1\n---\nbody\n";
        assert_eq!(doctype_of(s), Some("red-gate-log".into()));
    }

    /// #729 M3 regression: a `---` thematic break mid-document must NOT open
    /// a phantom frontmatter block.
    #[test]
    fn test_m3_thematic_break_is_not_frontmatter() {
        let s = "# Title\n\n---\ndocument_type: red-gate-log\n---\nmore\n";
        assert_eq!(doctype_of(s), None);
    }

    /// #729 M3: `document_type:` AFTER the closing fence is body text.
    #[test]
    fn test_m3_doctype_after_closing_fence_ignored() {
        let s = "---\nstatus: ok\n---\ndocument_type: red-gate-log\n";
        assert_eq!(doctype_of(s), None);
    }

    /// Sensitivity-preserving: an unclosed line-1 block still yields the
    /// doctype (a malformed leaked artifact must not evade the guard).
    #[test]
    fn test_unclosed_frontmatter_still_detected() {
        let s = "---\ndocument_type: red-gate-log\nno closing fence\n";
        assert_eq!(doctype_of(s), Some("red-gate-log".into()));
    }

    #[test]
    fn test_quotes_stripped_and_trimmed() {
        assert_eq!(
            doctype_of("---\ndocument_type: \"story\"\n---\n"),
            Some("story".into())
        );
        assert_eq!(
            doctype_of("---\ndocument_type:   'adr'  \n---\n"),
            Some("adr".into())
        );
    }

    #[test]
    fn test_empty_doctype_is_none() {
        assert_eq!(doctype_of("---\ndocument_type:\n---\n"), None);
        assert_eq!(doctype_of(""), None);
    }

    #[test]
    fn test_fence_tolerates_trailing_whitespace_only() {
        assert_eq!(
            doctype_of("---   \ndocument_type: story\n---\n"),
            Some("story".into())
        );
        assert_eq!(doctype_of("--- x\ndocument_type: story\n---\n"), None);
    }

    /// #729 M1 regression: templates in SUBDIRECTORIES are in the universe.
    #[test]
    fn test_m1_collects_doctypes_recursively() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("story-template.md"),
            "---\ndocument_type: story\n---\n",
        )
        .unwrap();
        let sub = dir.path().join("adversary-prompt-templates");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(
            sub.join("phase-5.md"),
            "---\ndocument_type: adversary-prompt-template\n---\n",
        )
        .unwrap();

        let set = collect_factory_doctypes(dir.path()).unwrap();
        assert!(set.contains("story"));
        assert!(
            set.contains("adversary-prompt-template"),
            "subdirectory template doctypes must be in the universe (M1)"
        );
    }

    /// #729 M2 regression: exemption is path-scoped, not doctype-global.
    #[test]
    fn test_m2_product_tracked_exempt_only_under_home() {
        assert!(is_product_tracked_at_home(
            "demo-evidence-report",
            "docs/demo-evidence/S-1/report.md"
        ));
        assert!(!is_product_tracked_at_home(
            "demo-evidence-report",
            "evidence-report.md"
        ));
        assert!(!is_product_tracked_at_home(
            "demo-evidence-report",
            "docs/reports/report.md"
        ));
        assert!(!is_product_tracked_at_home(
            "red-gate-log",
            "docs/demo-evidence/x.md"
        ));
    }

    #[test]
    fn test_worktree_and_machinery_skips() {
        assert!(in_factory_worktree(".factory/cycles/x.md"));
        assert!(in_factory_worktree("sub/.factory-project/x.md"));
        assert!(!in_factory_worktree("docs/factory-notes.md"));
        assert!(is_plugin_machinery("plugins/vsdd-factory/templates/t.md"));
        assert!(!is_plugin_machinery("plugins/vsdd-factory/bin/README.md"));
    }

    #[test]
    fn test_scan_end_to_end() {
        let repo = tempfile::tempdir().unwrap();
        let root = repo.path();
        std::fs::create_dir_all(root.join("docs/demo-evidence")).unwrap();
        std::fs::write(
            root.join("leaked.md"),
            "---\ndocument_type: red-gate-log\n---\n",
        )
        .unwrap();
        std::fs::write(
            root.join("docs/demo-evidence/report.md"),
            "---\ndocument_type: demo-evidence-report\n---\n",
        )
        .unwrap();
        std::fs::write(root.join("plain.md"), "# nothing\n").unwrap();

        let universe: BTreeSet<String> = ["red-gate-log", "demo-evidence-report"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let tracked = vec![
            "leaked.md".to_string(),
            "docs/demo-evidence/report.md".to_string(),
            "plain.md".to_string(),
        ];
        let leaks = scan(root, &tracked, &universe);
        assert_eq!(
            leaks,
            vec![Leak {
                doctype: "red-gate-log".into(),
                path: "leaked.md".into()
            }]
        );
    }
}
