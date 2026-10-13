//! Exactly-once textual substitution.

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SubstError {
    #[error("find text is empty")]
    EmptyFind,
    #[error("find text matched 0 times (expected exactly 1)")]
    NoMatch,
    #[error("find text matched {0} times (expected exactly 1)")]
    MultipleMatches(usize),
}

/// Count occurrences of `needle` in `haystack`, *including overlapping ones*
/// (so a pattern that could match at shifted offsets is never mistaken for a
/// unique site).
fn count_overlapping(haystack: &str, needle: &str) -> usize {
    let mut count = 0;
    let mut start = 0;
    while let Some(rel) = haystack[start..].find(needle) {
        count += 1;
        let abs = start + rel;
        // advance by one char (not byte) to stay on a UTF-8 boundary
        let step = haystack[abs..].chars().next().map_or(1, char::len_utf8);
        start = abs + step;
    }
    count
}

/// Replace the single occurrence of `find` with `replace`; fail hard unless
/// `find` occurs exactly once.
pub fn apply_single(content: &str, find: &str, replace: &str) -> Result<String, SubstError> {
    if find.is_empty() {
        return Err(SubstError::EmptyFind);
    }
    match count_overlapping(content, find) {
        0 => Err(SubstError::NoMatch),
        1 => Ok(content.replacen(find, replace, 1)),
        n => Err(SubstError::MultipleMatches(n)),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn single_match_is_replaced() {
        assert_eq!(apply_single("a b c", "b", "X").unwrap(), "a X c");
    }

    #[test]
    fn multiline_match_is_replaced() {
        let src = "fn f() {\n    if x {\n        y\n    }\n}\n";
        let out = apply_single(src, "    if x {\n        y\n    }\n", "    // gone\n").unwrap();
        assert_eq!(out, "fn f() {\n    // gone\n}\n");
    }

    #[test]
    fn zero_matches_fails_hard() {
        assert_eq!(apply_single("abc", "z", "y"), Err(SubstError::NoMatch));
    }

    #[test]
    fn multiple_matches_fail_hard() {
        assert_eq!(
            apply_single("a a", "a", "b"),
            Err(SubstError::MultipleMatches(2))
        );
    }

    #[test]
    fn overlapping_matches_count_as_multiple() {
        assert_eq!(
            apply_single("aaa", "aa", "b"),
            Err(SubstError::MultipleMatches(2))
        );
    }

    #[test]
    fn empty_find_is_rejected() {
        assert_eq!(apply_single("abc", "", "y"), Err(SubstError::EmptyFind));
    }

    #[test]
    fn non_ascii_content_is_handled() {
        assert_eq!(apply_single("é→é x", "→", "->").unwrap(), "é->é x");
        assert_eq!(
            apply_single("éé", "é", "e"),
            Err(SubstError::MultipleMatches(2))
        );
    }
}
