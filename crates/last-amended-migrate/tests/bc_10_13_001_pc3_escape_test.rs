// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! BC-10.13.001 PC3 (D-1144 unescaped-double-quote remediation on
//! `BC-INDEX.md`/`ARCH-INDEX.md`/`STATE.md`) + Invariant 4 / BC-5.45.001
//! Invariant 3 (every value the tool emits parses under strict YAML
//! `safe_load`) + PC4 idempotency of the escape step specifically.
//!
//! Backslash/quote sequences are constructed via `\u{5c}`/`\u{22}` escapes
//! rather than nested Rust string-escaping, to keep the exact byte sequence
//! under test unambiguous (this is precisely the class of off-by-one the
//! module under test — `src/escape.rs` — exists to get right, so the test's
//! own fixture construction must not be a second source of ambiguity).

mod common;

use last_amended_migrate::escape::{escape_raw_value, escape_value, needs_escaping};
use last_amended_migrate::migrate::{MigrationMode, MigrationOptions};
use last_amended_migrate::migrate_file;

const BACKSLASH: char = '\u{5c}';
const QUOTE: char = '\u{22}';

// ── Direct `needs_escaping`/`escape_value` unit tests ───────────────────────

#[test]
fn test_BC_10_13_001_PC3_needs_escaping_true_on_unescaped_quote() {
    let raw = format!("fixed the {QUOTE}quoted term{QUOTE} defect");
    assert!(needs_escaping(&raw));
}

#[test]
fn test_BC_10_13_001_PC3_needs_escaping_false_on_plain_text() {
    let raw = "fixed the quoted term defect (no quotes at all)";
    assert!(!needs_escaping(raw));
}

/// Already-escaped `\"` (one backslash, then quote) must NOT be flagged —
/// this is the idempotency half of PC4 for the escape step specifically.
#[test]
fn test_BC_10_13_001_PC3_needs_escaping_false_on_already_escaped_quote() {
    let raw = format!("abc{BACKSLASH}{QUOTE}def");
    assert!(
        !needs_escaping(&raw),
        "an already-escaped \\\" must not be re-flagged: {raw:?}"
    );
}

/// The tricky case the module doc calls out explicitly: `\\"` (an ESCAPED
/// BACKSLASH — two backslashes — immediately followed by an UNESCAPED
/// quote) IS an unescaped quote, not a double-escape. A naive
/// look-behind-one-char check would get this wrong.
#[test]
fn test_BC_10_13_001_PC3_needs_escaping_true_after_escaped_backslash() {
    let raw = format!("abc{BACKSLASH}{BACKSLASH}{QUOTE}def");
    assert!(
        needs_escaping(&raw),
        "an escaped backslash (\\\\) followed by a bare quote must still \
         count as an unescaped quote: {raw:?}"
    );
}

#[test]
fn test_BC_10_13_001_PC3_escape_value_escapes_unescaped_quotes() {
    let raw = format!("fixed the {QUOTE}quoted term{QUOTE} defect");
    let escaped = escape_value(&raw);
    assert!(!needs_escaping(&escaped), "result must be fully escaped");
    assert_eq!(
        escaped,
        format!("fixed the {BACKSLASH}{QUOTE}quoted term{BACKSLASH}{QUOTE} defect")
    );
}

/// PC4 idempotency: escaping an already-escaped value must not double-
/// escape it (a naive `value.replace('"', "\\\"")` fails this).
#[test]
fn test_BC_10_13_001_PC4_escape_value_is_idempotent() {
    let raw = format!("fixed the {QUOTE}quoted term{QUOTE} defect");
    let once = escape_value(&raw);
    let twice = escape_value(&once);
    assert_eq!(
        once, twice,
        "escaping an already-escaped value must be a no-op (PC4)"
    );
}

/// The `\\"` case's escape output: only the bare trailing quote gains an
/// escape; the pre-existing escaped backslash is preserved verbatim.
#[test]
fn test_BC_10_13_001_PC3_escape_value_preserves_escaped_backslash() {
    let raw = format!("abc{BACKSLASH}{BACKSLASH}{QUOTE}def");
    let escaped = escape_value(&raw);
    assert!(!needs_escaping(&escaped));
    assert_eq!(
        escaped,
        format!("abc{BACKSLASH}{BACKSLASH}{BACKSLASH}{QUOTE}def"),
        "must preserve the pre-existing escaped backslash (2 chars) and add \
         exactly one new escaping backslash before the formerly-bare quote"
    );
}

// ── S-15.03 SEC-001: control-character escaping (CWE-116) ──────────────────
//
// `needs_escaping`/`escape_value` previously only detected/escaped a literal
// `"`. A raw newline, carriage return, or tab passed through unescaped and
// got spliced verbatim into a double-quoted YAML scalar, breaking strict
// `safe_load` parsing exactly like an unescaped quote does.

#[test]
fn test_BC_10_13_001_SEC001_needs_escaping_true_on_raw_newline() {
    let raw = "line one\nline two";
    assert!(
        needs_escaping(raw),
        "a raw embedded newline must be flagged"
    );
}

#[test]
fn test_BC_10_13_001_SEC001_needs_escaping_true_on_raw_carriage_return() {
    let raw = "line one\rline two";
    assert!(
        needs_escaping(raw),
        "a raw embedded carriage return must be flagged"
    );
}

#[test]
fn test_BC_10_13_001_SEC001_needs_escaping_true_on_raw_tab() {
    let raw = "column1\tcolumn2";
    assert!(needs_escaping(raw), "a raw embedded tab must be flagged");
}

#[test]
fn test_BC_10_13_001_SEC001_needs_escaping_true_on_other_control_char() {
    // U+0007 BEL — an arbitrary C0 control character with no dedicated named
    // escape (`\n`/`\r`/`\t`), exercising the `\xHH` fallback branch.
    let raw = "abc\u{7}def";
    assert!(
        needs_escaping(raw),
        "any C0 control character below U+0020 must be flagged, not just \\n/\\r/\\t"
    );
}

#[test]
fn test_BC_10_13_001_SEC001_escape_value_escapes_newline_tab_cr() {
    let raw = "line one\nline\ttwo\rdone";
    let escaped = escape_value(raw);
    assert!(
        !needs_escaping(&escaped),
        "escaped output must contain no raw control characters left: {escaped:?}"
    );
    assert_eq!(escaped, "line one\\nline\\ttwo\\rdone");
}

#[test]
fn test_BC_10_13_001_SEC001_escape_value_control_chars_round_trip_strict_yaml() {
    // The exact defect class the security review named: an embedded raw
    // newline/tab/CR in a value this tool is about to splice into a
    // double-quoted YAML scalar.
    let raw = "fixed the bug\nadded a\ttab\rand a CR";
    let escaped = escape_value(raw);
    let wrapped = format!("last_amended: \"{escaped}\"\n");
    let parsed: common::MinimalFrontmatter =
        serde_norway::from_str(&wrapped).expect("escaped control chars must parse as valid YAML");
    assert_eq!(
        parsed.last_amended, raw,
        "the YAML-decoded value must round-trip to the exact original text \
         (semantics preserved, only the raw encoding changed)"
    );
}

#[test]
fn test_BC_10_13_001_SEC001_escape_value_control_chars_idempotent() {
    let raw = "one\ntwo\tthree\rfour";
    let once = escape_value(raw);
    let twice = escape_value(&once);
    assert_eq!(
        once, twice,
        "escaping an already-escaped control-character value must be a no-op (PC4)"
    );
}

/// End-to-end: a `last_amended` value carrying a raw embedded carriage
/// return is escaped by `migrate_file`, and the resulting file parses
/// cleanly under strict YAML with the original text preserved EXACTLY
/// (including the control character itself).
///
/// # Why carriage return, not a raw newline, for this end-to-end fixture
///
/// `src/frontmatter.rs`'s hand-rolled reader (`extract_last_amended`)
/// deliberately bounds its scan for `last_amended:`'s value to a single
/// physical line via `.find('\n')` (see that module's own doc comment: every
/// field this tool reads is assumed single-line, which is what keeps the
/// scan `O(n)`-bounded against the D-1149 mega-line calibration ceiling). A
/// raw embedded `\n` therefore can never successfully round-trip THROUGH
/// `migrate_file`'s read step at all — extraction itself fails before
/// eligibility/escaping ever runs (proven separately: such a fixture yields
/// `Err(NotEligible)`, not a corrupt write), so it cannot reach this
/// end-to-end path. A raw `\r`, by contrast, does NOT terminate that
/// `\n`-bounded scan — it is read successfully as ordinary line content, so
/// it genuinely reaches `escape_value` through the real production code
/// path, making it the right control character to exercise here. (The `\n`
/// case's own escaping correctness is proven directly at the pure-function
/// level by `test_BC_10_13_001_SEC001_escape_value_control_chars_round_trip_strict_yaml`
/// above, which does not depend on the hand-rolled reader's single-line
/// assumption.)
///
/// A raw (unescaped) carriage return inside a YAML double-quoted scalar is
/// NOT a strict-parse error either — the YAML spec's flow-scalar line
/// folding rule silently collapses it to a single space instead. That is
/// arguably a WORSE defect than an outright parse failure (CWE-116): the
/// unfixed file "looks fine" (parses without error) while quietly losing
/// information, which a `--check`-style validity gate alone would never
/// catch. This test proves both halves: (1) the PRE-fix content already
/// silently loses the carriage return under strict parsing (the corruption
/// `escape_value` exists to prevent), and (2) `migrate_file` closes that gap
/// — the POST-fix content round-trips to the exact original text, carriage
/// return included.
#[test]
fn test_BC_10_13_001_SEC001_migrate_file_prevents_silent_cr_corruption() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended = "2026-09-02 (v5.42) — fixed the bug\rsecond note".to_string();
    let content = common::frontmatter_file(
        "behavioral-contract-index",
        "5.42",
        &last_amended,
        Some(&[common::changelog_item_block("2026-08-01", "an older entry")]),
        "# Fixture BC-INDEX\n",
    );
    let path = common::write_file(dir.path(), "BC-INDEX.md", &content);

    // Fixture sanity: PRE-fix, strict YAML parsing SUCCEEDS (no error) but
    // silently folds the raw carriage return into a space — proving the
    // defect is real corruption, not merely a hypothetical parse failure.
    let pre_fix_parsed = common::strict_yaml_parse(&content)
        .expect("a raw embedded carriage return does not itself cause a YAML parse error");
    assert_ne!(
        pre_fix_parsed.last_amended, last_amended,
        "fixture sanity: pre-fix, the raw carriage return must already be \
         silently folded away by strict YAML parsing (the CWE-116 defect \
         this fix closes): {:?}",
        pre_fix_parsed.last_amended
    );
    assert!(
        !pre_fix_parsed.last_amended.contains('\r'),
        "fixture sanity: pre-fix decoded value must have lost the carriage \
         return: {:?}",
        pre_fix_parsed.last_amended
    );

    let report = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default())
        .expect("migrate_file must succeed");
    assert!(report.mutated);
    assert!(report.escape_fixed, "report must record the escape fix");

    let after = common::read_file(&path);
    let parsed = common::strict_yaml_parse(&after)
        .expect("post-fix frontmatter must parse under strict YAML safe_load");
    assert_eq!(
        parsed.last_amended, last_amended,
        "post-fix, the original text INCLUDING its carriage return must \
         round-trip exactly — no silent corruption: {:?}",
        parsed.last_amended
    );
}

/// End-to-end: a `last_amended` value carrying an arbitrary C0 control
/// character with no dedicated named YAML escape (here, `U+0007` BEL) is a
/// genuine strict-parse ERROR before the fix (unlike `\n`/`\r`, which fold
/// silently instead — see the sibling test above) — `migrate_file` must
/// escape it via the `\xHH` fallback and produce a file that parses cleanly.
#[test]
fn test_BC_10_13_001_SEC001_migrate_file_escapes_other_control_char_hard_parse_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended = "2026-09-02 (v5.43) — fixed the bug\u{7}beeped".to_string();
    let content = common::frontmatter_file(
        "architecture-index",
        "5.43",
        &last_amended,
        Some(&[common::changelog_item_block("2026-08-01", "an older entry")]),
        "# Fixture ARCH-INDEX\n",
    );
    let path = common::write_file(dir.path(), "ARCH-INDEX.md", &content);

    assert!(
        common::strict_yaml_parse(&content).is_err(),
        "fixture sanity: a raw BEL control character must break strict YAML \
         parsing before the fix is applied (unlike \\n/\\r, this one has no \
         line-folding escape hatch)"
    );

    let report = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default())
        .expect("migrate_file must succeed");
    assert!(report.mutated);
    assert!(report.escape_fixed, "report must record the escape fix");

    let after = common::read_file(&path);
    let parsed = common::strict_yaml_parse(&after)
        .expect("post-fix frontmatter must parse under strict YAML safe_load");
    assert_eq!(
        parsed.last_amended, last_amended,
        "the original text, including the control character, must survive \
         the fix semantically unchanged: {:?}",
        parsed.last_amended
    );
}

// ── `migrate_file` end-to-end integration + strict YAML round trip ─────────

/// BC-10.13.001 PC3 canonical edge-case test vector: `BC-INDEX.md`-shaped
/// fixture with `changelog:` present and a D-1144-defective `last_amended`.
/// After migration: `changelog:` UNCHANGED; `last_amended` rewritten with
/// the `"` escaped to `\"`; report says 1 file mutated, 1 escape fix.
#[test]
fn test_BC_10_13_001_PC3_migrate_file_fixes_bc_index_quote_defect() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended = common::quote_defect_current_entry("2026-09-02", "v5.41");
    let content = common::frontmatter_file(
        "behavioral-contract-index",
        "5.41",
        &last_amended,
        Some(&[common::changelog_item_block("2026-08-01", "an older entry")]),
        "# Fixture BC-INDEX\n",
    );
    let path = common::write_file(dir.path(), "BC-INDEX.md", &content);

    // Fixture sanity: the PRE-fix content genuinely reproduces the D-1144
    // defect — it must NOT parse under strict YAML.
    assert!(
        common::strict_yaml_parse(&content).is_err(),
        "fixture sanity: an unescaped embedded quote must break strict YAML \
         parsing before the fix is applied"
    );

    let report = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default())
        .expect("migrate_file must succeed");

    assert!(report.mutated, "escaping a defective entry is a mutation");
    assert!(report.escape_fixed, "report must record the escape fix");

    let after = common::read_file(&path);
    assert_eq!(
        after.matches("changelog:").count(),
        1,
        "changelog: must remain UNCHANGED per the canonical edge-case vector"
    );
    assert!(
        after.contains("an older entry"),
        "pre-existing changelog item must survive untouched: {after:?}"
    );

    // Invariant 4 / BC-5.45.001 Invariant 3: the POST-fix content parses
    // cleanly under strict YAML.
    let parsed = common::strict_yaml_parse(&after)
        .expect("post-fix frontmatter must parse under strict YAML safe_load");
    assert!(
        parsed.last_amended.contains("quoted term"),
        "the entry's substantive text must be preserved through the fix: {:?}",
        parsed.last_amended
    );
    assert!(
        parsed.last_amended.contains('"'),
        "the YAML-decoded value must contain the literal quote character \
         (semantics preserved, only the raw escaping changed): {:?}",
        parsed.last_amended
    );
}

/// PC4: running the migration a second time on an already-escaped file is a
/// verified-clean no-op — byte-identical output.
#[test]
fn test_BC_10_13_001_PC4_migrate_file_escape_fix_is_idempotent_across_runs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended = common::quote_defect_current_entry("2026-09-02", "v4.11");
    let content = common::frontmatter_file(
        "architecture-index",
        "4.11",
        &last_amended,
        Some(&[common::changelog_item_block("2026-08-01", "an older entry")]),
        "# Fixture ARCH-INDEX\n",
    );
    let path = common::write_file(dir.path(), "ARCH-INDEX.md", &content);

    let first = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default())
        .expect("first migrate_file call");
    assert!(first.mutated && first.escape_fixed);
    let after_first = common::read_file(&path);

    let second = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default())
        .expect("second migrate_file call");
    assert!(
        !second.mutated,
        "PC4: a second run against an already-migrated file must report zero mutations"
    );
    assert!(!second.escape_fixed);

    let after_second = common::read_file(&path);
    assert_eq!(
        after_first, after_second,
        "PC4: second run must produce byte-identical output"
    );
}

/// PC3 scope: `VP-INDEX.md`/`STORY-INDEX.md` are explicitly UNAFFECTED by
/// the D-1144 defect set — a clean fixture for either must never be
/// (falsely) flagged as needing an escape fix.
#[test]
fn test_BC_10_13_001_PC3_migrate_file_clean_entry_never_flags_escape_fixed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended = common::clean_current_entry("2026-09-02", "v2.99", "no quotes here");
    let content = common::frontmatter_file(
        "verification-property-index",
        "2.99",
        &last_amended,
        Some(&[common::changelog_item_block("2026-08-01", "an older entry")]),
        "# Fixture VP-INDEX\n",
    );
    let path = common::write_file(dir.path(), "VP-INDEX.md", &content);

    let report = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default())
        .expect("migrate_file must succeed");

    assert!(!report.escape_fixed);
    assert!(!report.mutated, "fully compliant file is a verified no-op");
}

// ── S-15.03 pr-reviewer S1: literal-backslash escaping ──────────────────────
//
// Before this fix, `needs_escaping`/`escape_value` never inspected a literal
// backslash at all — a value containing one (e.g. a Windows-style path, or a
// bare trailing `\`) would fail `yaml_guard`'s strict `safe_load` pre-write
// gate with no way for this tool to remediate it. This is a genuine
// functional gap (the tool refuses to touch such a file, fail-closed, rather
// than corrupting it), not merely a cosmetic omission.

#[test]
fn test_BC_10_13_001_S1_needs_escaping_true_on_windows_style_path_backslash() {
    let raw = r"fixed the C:\Users\config\config.yaml path bug";
    assert!(
        needs_escaping(raw),
        "a literal backslash not part of a recognized escape token must be flagged: {raw:?}"
    );
}

#[test]
fn test_BC_10_13_001_S1_needs_escaping_true_on_bare_trailing_backslash() {
    let raw = "value ending in a trailing backslash: \\";
    assert!(
        needs_escaping(raw),
        "a bare backslash with nothing recognized after it (including at \
         the very end of the value) must be flagged: {raw:?}"
    );
}

#[test]
fn test_BC_10_13_001_S1_escape_value_doubles_windows_style_path_backslashes() {
    let raw = r"C:\Users\config";
    let escaped = escape_value(raw);
    assert!(
        !needs_escaping(&escaped),
        "escaped output must contain no un-recognized backslashes left: {escaped:?}"
    );
    assert_eq!(escaped, r"C:\\Users\\config");
}

#[test]
fn test_BC_10_13_001_S1_escape_value_backslash_is_idempotent() {
    let raw = r"C:\Users\config and a trailing \";
    let once = escape_value(raw);
    let twice = escape_value(&once);
    assert_eq!(
        once, twice,
        "escaping an already-escaped literal-backslash value must be a no-op (PC4)"
    );
}

/// S1's own acceptance bar: a real backslash (Windows-style path AND a
/// literal trailing `\`) must now round-trip through STRICT YAML instead of
/// being rejected by `yaml_guard` — proving the fix closes the functional
/// gap, not just that the two pure functions agree with each other.
#[test]
fn test_BC_10_13_001_S1_escape_value_backslash_round_trips_strict_yaml() {
    let raw = r"backed up to C:\Users\config\ before the fix";
    let escaped = escape_value(raw);
    let wrapped = format!("last_amended: \"{escaped}\"\n");
    let parsed: common::MinimalFrontmatter = serde_norway::from_str(&wrapped)
        .expect("escaped literal backslashes must parse as valid strict YAML");
    assert_eq!(
        parsed.last_amended, raw,
        "the YAML-decoded value must round-trip to the exact original text \
         (semantics preserved, only the raw encoding changed)"
    );
}

/// `escape_value`'s lookahead ambiguity (documented as a "known limitation"
/// on prose text, where it is harmless in practice) becomes a genuine
/// silent-corruption bug when misapplied to a raw filesystem path: a `\`
/// immediately followed by `r` (as in a literal path component like
/// `runner`) is indistinguishable from an already-escaped `\r` (carriage
/// return) token, so `escape_value` would leave it untouched — and strict
/// YAML `safe_load` would then decode that untouched `\r` as an actual CR
/// byte, silently corrupting the path rather than raising a parse error.
/// `rotate.rs` was fixed (S-15.03 windows-x64 CI failure, this fix) to use
/// `escape_raw_value` instead of `escape_value` for exactly this reason —
/// this test proves `escape_raw_value` has no such collision: every literal
/// backslash is unconditionally escaped, with no "already escaped" lookahead
/// to be ambiguous about.
#[test]
fn test_BC_10_13_001_S1_escape_raw_value_no_lookahead_collision_on_windows_style_path() {
    let raw = r"C:\Users\runner\.factory\cycles\test-cycle\BC-INDEX-changelog-archive.md";

    // Fixture sanity: this exact input is the case `escape_value` gets
    // wrong, because `\r` in `\runner` matches its recognized-escape-token
    // lookahead and is left as a literal (unescaped) `\r` — which is itself
    // syntactically valid inside a YAML double-quoted scalar (it's just the
    // WRONG two bytes: a real CR byte instead of backslash+`r`), so
    // `yaml_guard`'s strict-parse gate cannot catch this class of defect.
    let wrongly_escaped = escape_value(raw);
    let wrapped_wrong = format!("last_amended: \"{wrongly_escaped}\"\n");
    let decoded_wrong: common::MinimalFrontmatter = serde_norway::from_str(&wrapped_wrong)
        .expect("fixture sanity: escape_value's output must still parse as valid YAML");
    let decoded_wrong_value = decoded_wrong.last_amended.clone();
    assert_ne!(
        decoded_wrong_value, raw,
        "fixture sanity: escape_value must NOT round-trip this path correctly \
         (that is the exact defect escape_raw_value fixes) — decoded: {decoded_wrong_value:?}"
    );

    let escaped = escape_raw_value(raw);
    let wrapped = format!("changelog_archive: \"{escaped}\"\n");
    let value: serde_norway::Value = serde_norway::from_str(&wrapped)
        .expect("escape_raw_value's output must parse as valid strict YAML");
    let decoded = value
        .get("changelog_archive")
        .and_then(serde_norway::Value::as_str)
        .expect("changelog_archive key must be present and a string");
    assert_eq!(
        decoded, raw,
        "escape_raw_value must round-trip the exact original path, every \
         backslash included, with no lookahead-collision corruption"
    );
}

/// End-to-end: a `last_amended` value carrying a literal Windows-style path
/// backslash is escaped by `migrate_file`, and the resulting file parses
/// cleanly under strict YAML with the original text preserved exactly.
///
/// PRE-fix sanity: such a fixture would previously have made `migrate_file`
/// (in `Apply` mode) fail `yaml_guard`'s pre-write gate with
/// `Err(MigrateError::InvalidYamlProduced)` — this fixture's raw value
/// contains a `\U` sequence, an invalid YAML escape lead, and the tool's
/// pre-S1 `escape_value` never touched the backslash, so the invalid
/// sequence would have been written verbatim into `doc.raw` and caught (not
/// silently corrupted, but also never fixed) by the gate. This test proves
/// the gap is closed: `migrate_file` now succeeds and produces valid,
/// round-trippable output.
#[test]
fn test_BC_10_13_001_S1_migrate_file_fixes_windows_style_path_backslash_defect() {
    let dir = tempfile::tempdir().expect("tempdir");
    let last_amended =
        r"2026-09-02 (v5.43) — recovered state from C:\Users\config\backup".to_string();
    let content = common::frontmatter_file(
        "behavioral-contract-index",
        "5.43",
        &last_amended,
        Some(&[common::changelog_item_block("2026-08-01", "an older entry")]),
        "# Fixture BC-INDEX\n",
    );
    let path = common::write_file(dir.path(), "BC-INDEX.md", &content);

    // Fixture sanity: PRE-fix, the raw content as written is NOT valid
    // strict YAML — `\U` is not a value-following-backslash this tool ever
    // emits, and Rust's `\u{5c}U` here is a literal backslash-then-U, which
    // strict `safe_load` rejects outright (an 8-hex-digit `\U` escape is
    // expected but "sers\\config..." isn't hex).
    assert!(
        common::strict_yaml_parse(&content).is_err(),
        "fixture sanity: the raw pre-fix content must NOT already be valid \
         strict YAML — that is the exact defect this fix closes"
    );

    let report = migrate_file(&path, MigrationMode::Apply, MigrationOptions::default())
        .expect("migrate_file must succeed — S1's whole point is that this no longer fails");

    assert!(
        report.escape_fixed,
        "the backslash defect must be flagged as fixed"
    );
    assert!(report.mutated);

    let after = common::read_file(&path);
    let parsed = common::strict_yaml_parse(&after)
        .expect("post-fix content must parse cleanly under strict YAML safe_load");
    assert_eq!(
        parsed.last_amended, last_amended,
        "the decoded value must round-trip to the exact original text: {:?}",
        parsed.last_amended
    );
}
