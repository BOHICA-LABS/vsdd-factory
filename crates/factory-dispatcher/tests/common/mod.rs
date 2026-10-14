//! Helpers shared by the S-25.08 dispatcher integration tests
//! (`mod common;` from each test file). Not a test target itself.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// `Path::canonicalize` minus the Windows `\\?\` verbatim prefix (dunce-style;
/// no new dependency). On Windows `canonicalize` returns a `\\?\C:\...`
/// verbatim path in which `/` is NOT a separator and `.`/`..` are NOT
/// normalized, so a `format!("{}/x", ..)` built from it never exercises the
/// alias/traversal it claims to (PR #848 round-3 BLOCKING-2, CI run
/// 38485890751). The prefix is stripped for drive paths only (`\\?\UNC\...`
/// is left alone). Identity of `canonicalize` on every other platform.
pub fn simplified_canonical(p: &Path) -> PathBuf {
    let c = p.canonicalize().unwrap();
    #[cfg(windows)]
    {
        let s = c.to_string_lossy().into_owned();
        if let Some(rest) = s.strip_prefix(r"\\?\")
            && !rest.starts_with("UNC\\")
        {
            return PathBuf::from(rest);
        }
    }
    c
}
