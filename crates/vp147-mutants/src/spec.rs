//! Mutant spec: the checked-in `mutants.toml` (one `[[mutant]]` table each).

use serde::Deserialize;
use std::collections::BTreeSet;

/// Spec file location, relative to the repo root.
pub const DEFAULT_SPEC_PATH: &str = "crates/vp147-mutants/mutants.toml";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SpecError {
    #[error("spec parse error: {0}")]
    Parse(String),
    #[error("spec has no [[mutant]] entries")]
    Empty,
    #[error("duplicate mutant id {0}")]
    DuplicateId(String),
    #[error("mutant {id}: {reason}")]
    Invalid { id: String, reason: String },
}

/// The only verdict VP-147 permits for a mutant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Expected {
    #[serde(rename = "KILLED")]
    Killed,
}

/// How a mutant is expected to be killed.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Killer {
    /// `cargo kani -p <package> --exact --harness <h>...`
    Kani {
        package: String,
        /// Fully-qualified harness names (matched with `--exact`).
        harnesses: Vec<String>,
    },
    /// `cargo test <args...>` (args typically `-p`, `--test`, `--no-fail-fast`).
    Test { args: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Mutant {
    pub id: String,
    pub description: String,
    /// Path relative to the repo root.
    pub file: String,
    /// Exact text; must occur exactly once in `file`.
    pub find: String,
    pub replace: String,
    pub expected: Expected,
    pub killer: Killer,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Spec {
    #[serde(rename = "mutant")]
    pub mutants: Vec<Mutant>,
}

impl Spec {
    /// Parse and structurally validate a spec document.
    pub fn parse(text: &str) -> Result<Spec, SpecError> {
        let spec: Spec = toml::from_str(text).map_err(|e| SpecError::Parse(e.to_string()))?;
        spec.validate()?;
        Ok(spec)
    }

    fn validate(&self) -> Result<(), SpecError> {
        if self.mutants.is_empty() {
            return Err(SpecError::Empty);
        }
        let mut seen = BTreeSet::new();
        for m in &self.mutants {
            let bad = |reason: &str| SpecError::Invalid {
                id: m.id.clone(),
                reason: reason.to_string(),
            };
            if m.id.trim().is_empty() {
                return Err(bad("empty id"));
            }
            if !seen.insert(m.id.clone()) {
                return Err(SpecError::DuplicateId(m.id.clone()));
            }
            if m.file.trim().is_empty() || m.file.starts_with('/') || m.file.contains("..") {
                return Err(bad("file must be a repo-relative path without `..`"));
            }
            if m.find.is_empty() {
                return Err(bad("empty find text"));
            }
            if m.find == m.replace {
                return Err(bad("replace is identical to find (no-op mutant)"));
            }
            match &m.killer {
                Killer::Kani { package, harnesses } => {
                    if package.is_empty() || harnesses.is_empty() {
                        return Err(bad("kani killer needs a package and >= 1 harness"));
                    }
                }
                Killer::Test { args } => {
                    if args.is_empty() {
                        return Err(bad("test killer needs args"));
                    }
                }
            }
        }
        Ok(())
    }

    /// Select mutants by id (empty `only` selects all). Unknown ids are an error.
    pub fn select<'a>(&'a self, only: &[String]) -> Result<Vec<&'a Mutant>, SpecError> {
        if only.is_empty() {
            return Ok(self.mutants.iter().collect());
        }
        let mut out = Vec::new();
        for id in only {
            match self.mutants.iter().find(|m| &m.id == id) {
                Some(m) => out.push(m),
                None => {
                    return Err(SpecError::Invalid {
                        id: id.clone(),
                        reason: "no such mutant in spec".to_string(),
                    });
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    const GOOD: &str = r#"
[[mutant]]
id = "A1"
description = "d"
file = "src/x.rs"
find = '''
foo
'''
replace = '''
bar
'''
expected = "KILLED"
killer = { kind = "kani", package = "p", harnesses = ["m::h1", "m::h2"] }

[[mutant]]
id = "A2"
description = "d"
file = "src/y.rs"
find = "a"
replace = "b"
expected = "KILLED"
killer = { kind = "test", args = ["-p", "p", "--no-fail-fast"] }
"#;

    #[test]
    fn parses_both_killer_kinds_and_preserves_multiline_text() {
        let s = Spec::parse(GOOD).unwrap();
        assert_eq!(s.mutants.len(), 2);
        assert_eq!(s.mutants[0].find, "foo\n");
        assert_eq!(s.mutants[0].replace, "bar\n");
        assert!(
            matches!(&s.mutants[0].killer, Killer::Kani { harnesses, .. } if harnesses.len() == 2)
        );
        assert!(matches!(&s.mutants[1].killer, Killer::Test { args } if args.len() == 3));
    }

    #[test]
    fn rejects_non_killed_expectation() {
        let t = GOOD.replacen("expected = \"KILLED\"", "expected = \"SURVIVED\"", 1);
        assert!(matches!(Spec::parse(&t), Err(SpecError::Parse(_))));
    }

    #[test]
    fn rejects_duplicate_ids_empty_find_noop_and_bad_paths() {
        let dup = GOOD.replace("id = \"A2\"", "id = \"A1\"");
        assert_eq!(Spec::parse(&dup), Err(SpecError::DuplicateId("A1".into())));

        let noop = GOOD.replace(
            "find = \"a\"\nreplace = \"b\"",
            "find = \"a\"\nreplace = \"a\"",
        );
        assert!(matches!(Spec::parse(&noop), Err(SpecError::Invalid { .. })));

        let empty_find = GOOD.replace("find = \"a\"", "find = \"\"");
        assert!(matches!(
            Spec::parse(&empty_find),
            Err(SpecError::Invalid { .. })
        ));

        let abs = GOOD.replace("file = \"src/y.rs\"", "file = \"/etc/passwd\"");
        assert!(matches!(Spec::parse(&abs), Err(SpecError::Invalid { .. })));

        let dotdot = GOOD.replace("file = \"src/y.rs\"", "file = \"../y.rs\"");
        assert!(matches!(
            Spec::parse(&dotdot),
            Err(SpecError::Invalid { .. })
        ));
    }

    #[test]
    fn rejects_empty_spec_and_empty_killers() {
        assert!(matches!(
            Spec::parse(""),
            Err(SpecError::Parse(_) | SpecError::Empty)
        ));
        let no_h = GOOD.replace("harnesses = [\"m::h1\", \"m::h2\"]", "harnesses = []");
        assert!(matches!(Spec::parse(&no_h), Err(SpecError::Invalid { .. })));
    }

    #[test]
    fn select_filters_and_rejects_unknown() {
        let s = Spec::parse(GOOD).unwrap();
        assert_eq!(s.select(&[]).unwrap().len(), 2);
        assert_eq!(s.select(&["A2".into()]).unwrap()[0].id, "A2");
        assert!(s.select(&["Z9".into()]).is_err());
    }

    #[test]
    fn checked_in_spec_is_valid_and_complete() {
        let text = include_str!("../mutants.toml");
        let s = Spec::parse(text).unwrap();
        let ids: Vec<&str> = s.mutants.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["M1", "M2", "M3", "M4-model", "M4-production", "M5"]);
    }
}
