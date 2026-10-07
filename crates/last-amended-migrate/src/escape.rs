//! D-1144 YAML double-quote escape remediation (BC-10.13.001 PC3).
//!
//! # S-15.03 SEC-001 remediation — control-character escaping
//!
//! A literal `"` is not the only byte sequence that is significant inside a
//! YAML double-quoted scalar: a raw (unescaped) newline, carriage return,
//! tab, or any other C0 control character (`U+0000`..`U+001F`) is ALSO
//! illegal there and breaks strict `safe_load` parsing exactly like an
//! unescaped quote does (CWE-116, improper output neutralization). Both
//! `needs_escaping` and `escape_value` therefore treat "needs escaping" as
//! "contains an unescaped `\"` OR a raw control character below `U+0020`,
//! OR a literal backslash that isn't already part of a recognized escape
//! token" — see the S1 note below for the third case.
//!
//! # S-15.03 pr-reviewer S1 remediation — literal backslash escaping
//!
//! A literal backslash (e.g. a Windows-style path fragment like `C:\Users`,
//! or a value ending in a bare trailing `\`) is ALSO significant inside a
//! YAML double-quoted scalar: per the YAML spec, `\` may only be followed by
//! one of a fixed set of recognized escape characters (`\`, `"`, `n`, `r`,
//! `t`, or `x` plus 2 hex digits, for the subset this tool ever emits) — any
//! other following character (or a `\` at the very end of the value) is
//! invalid and `yaml_guard`'s pre-write `safe_load` gate correctly refuses
//! to let it reach disk. Before this fix, neither `needs_escaping` nor
//! `escape_value` ever inspected a literal backslash at all, so a value
//! containing one could never be remediated by this tool — it would always
//! be rejected by the yaml_guard gate (fail-closed, not corrupting, but a
//! genuine functional gap: a real file containing a backslash could never
//! be fixed).
//!
//! `needs_escaping`/`escape_value` now walk the value with 1-token
//! lookahead: at each `\`, they check whether it begins a token this
//! module's own `escape_value` could have emitted (`\\`, `\"`, `\n`, `\r`,
//! `\t`, or `\xHH`) — if so, the whole token is treated as already-escaped
//! and copied/skipped verbatim (this is what makes the scheme idempotent,
//! PC4); otherwise the `\` is a bare/literal backslash and is escaped to
//! `\\`. This lookahead-token-consumption is exactly equivalent to the
//! previous backslash-run-parity check for the quote case (a run of paired
//! backslashes is consumed left-to-right as N `\\` tokens, leaving at most
//! one dangling backslash immediately before the quote to decide whether
//! that quote is escaped) while additionally covering the bare-backslash
//! case the parity check never considered.
//!
//! ## Known limitation — coincidental escape-shaped literal text
//!
//! Because a raw `\` followed by `n`/`r`/`t`/`x`+hex/`\`/`"` is
//! indistinguishable from an already-escaped token without external
//! provenance tracking (which this crate deliberately does not carry — see
//! PC4's "detect already-compliant shape, do nothing" convention), a
//! genuinely literal backslash immediately followed by one of those
//! characters (e.g. a path fragment like `\name` or `\tmp`) is treated as
//! an already-escaped token and left untouched, rather than escaped. This
//! is the same fundamental ambiguity every escaping scheme without a
//! separate "was this pre-escaped" marker has; it is inherent, not a
//! regression from this fix. In practice, real corpus content (burst
//! summaries, decision text) essentially never contains a raw backslash
//! immediately followed by exactly one of those specific letters where a
//! literal (non-escape) reading was intended, so this does not affect any
//! known real `.factory/` fixture.

/// Whether `value` contains at least one literal `"` that is not already
/// escaped as `\"`, at least one raw (unescaped-by-construction) control
/// character below `U+0020` (`\n`, `\r`, `\t`, and anything else in that
/// range), or at least one literal backslash that does not begin a
/// recognized escape token (`\\`, `\"`, `\n`, `\r`, `\t`, `\xHH`) — the full
/// D-1144/SEC-001/S1 defect class that breaks strict YAML `safe_load` on
/// `BC-INDEX.md`/`ARCH-INDEX.md`/`STATE.md`'s current `last_amended`
/// entries, on any `changelog:` item text this tool emits, and on any raw
/// literal text (e.g. a Windows-style path) a PC7 split relocates.
///
/// Must be idempotent-safe: an already-fully-escaped value returns `false`
/// (PC4) — this function is the detection half of that guarantee, and MUST
/// agree with `escape_value` on what counts as "already escaped." A single
/// forward pass with up to 3-character lookahead (`recognized_escape_len`)
/// — never a look-behind re-scan — classifies each `\` as either the start
/// of an already-recognized escape token (skipped verbatim) or a bare
/// literal backslash (flags `needs_escaping`).
pub fn needs_escaping(value: &str) -> bool {
    let chars: Vec<char> = value.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' {
            match recognized_escape_len(&chars, i) {
                Some(len) => {
                    i += len;
                    continue;
                }
                None => return true,
            }
        }
        if c == '"' {
            return true;
        }
        if is_raw_control(c) {
            return true;
        }
        i += 1;
    }
    false
}

/// Whether `c` is a raw C0 control character (`U+0000`..=`U+001F`) that is
/// illegal, unescaped, inside a YAML double-quoted scalar — covers `\n`
/// (`U+000A`), `\r` (`U+000D`), `\t` (`U+0009`), and every other control
/// character in that range.
fn is_raw_control(c: char) -> bool {
    (c as u32) < 0x20
}

/// If `chars[pos]` is `\` and begins a token this module's own
/// `escape_value` could have emitted — `\\`, `\"`, `\n`, `\r`, `\t` (2
/// chars), or `\xHH` with `HH` two ASCII hex digits (4 chars) — returns that
/// token's length in `char`s. Returns `None` when `chars[pos]` is `\` but is
/// followed by anything else (including nothing, i.e. `\` is the final
/// character of the value): this is the bare/literal-backslash case S1
/// identifies, which `needs_escaping`/`escape_value` must treat as needing
/// escaping rather than silently passing through.
///
/// Bounded lookahead only (at most 3 characters past `pos`), never a
/// look-behind re-scan, so this stays a single linear (`O(n)`) pass overall
/// when called once per character from `needs_escaping`/`escape_value`.
fn recognized_escape_len(chars: &[char], pos: usize) -> Option<usize> {
    match chars.get(pos + 1)? {
        '\\' | '"' | 'n' | 'r' | 't' => Some(2),
        'x' => {
            let h1 = chars.get(pos + 2)?;
            let h2 = chars.get(pos + 3)?;
            if h1.is_ascii_hexdigit() && h2.is_ascii_hexdigit() {
                Some(4)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Unconditionally escape every literal `"`, every raw C0 control character,
/// and every literal backslash in `value` for embedding in a YAML
/// double-quoted scalar — with NO "is this already an escape token"
/// lookahead/detection (unlike [`escape_value`]).
///
/// # Why this exists separately from `escape_value`
///
/// `escape_value`'s lookahead treats a `\` immediately followed by `n`, `r`,
/// `t`, `x` + 2 hex digits, `\`, or `"` as an ALREADY-ESCAPED token and
/// copies it verbatim — this is required for [`escape_value`]'s idempotency
/// contract (PC4): re-running migration against a file this tool already
/// wrote must not double-escape already-escaped prose it round-trips through
/// `last_amended`/`changelog:` text fields.
///
/// A raw filesystem path is a fundamentally different kind of input: it is
/// ALWAYS freshly computed (via `Path::join`, never read back out of a
/// previously-escaped YAML value), so there is no idempotency concern and
/// therefore no legitimate reason to treat any backslash in it as
/// "already escaped." Reusing `escape_value`'s ambiguous heuristic on a raw
/// path is actively wrong: on Windows (and on any path whose `\`-separated
/// component happens to start with `n`/`r`/`t`/`x`+hex/`\`/`"` — e.g. a
/// cycle name like `test-cycle`, or a username like `runner`), the
/// separator's `\` would collide with one of those lookahead tokens and be
/// left unescaped, so `serde_norway`'s strict `safe_load` would silently
/// decode it back as an actual tab/newline/CR byte instead of the original
/// `\`+letter two-character sequence — a silent path-corruption defect, not
/// a parse failure (since `\t`/`\n`/`\r` are all syntactically legal
/// escapes, `yaml_guard`'s pre-write gate would never catch this).
///
/// `rotate.rs` is this function's only real caller, embedding
/// `archive_path.display()` — a raw, always-fresh path — into the
/// `changelog_archive:` pointer line.
pub fn escape_raw_value(value: &str) -> String {
    let mut result = String::with_capacity(value.len() + 8);
    for c in value.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if is_raw_control(c) => result.push_str(&format!("\\x{:02X}", c as u32)),
            c => result.push(c),
        }
    }
    result
}

/// Escape every unescaped literal `"` in `value` to `\"`, every raw C0
/// control character to its standard YAML double-quoted-scalar escape
/// sequence (`\n`, `\r`, `\t` for those three; `\xHH` for any other control
/// byte below `U+0020`), and every literal (not-already-recognized-escape)
/// backslash to `\\`, preserving every other character verbatim
/// (BC-10.13.001 PC3; S-15.03 SEC-001, S1). Idempotent: calling this on an
/// already-escaped value must not double-escape (PC4) — `recognized_escape_len`
/// classifies each `\` identically to `needs_escaping`, so a token this
/// function already emitted is always recognized and copied verbatim on a
/// second pass, and control characters are idempotent by construction (a
/// raw control byte always becomes a multi-character printable escape
/// sequence, so a second pass never observes a raw control byte again).
pub fn escape_value(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    let mut result = String::with_capacity(value.len() + 8);
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' => {
                if let Some(len) = recognized_escape_len(&chars, i) {
                    result.extend(&chars[i..i + len]);
                    i += len;
                } else {
                    result.push_str("\\\\");
                    i += 1;
                }
            }
            '"' => {
                result.push_str("\\\"");
                i += 1;
            }
            '\n' => {
                result.push_str("\\n");
                i += 1;
            }
            '\r' => {
                result.push_str("\\r");
                i += 1;
            }
            '\t' => {
                result.push_str("\\t");
                i += 1;
            }
            c if is_raw_control(c) => {
                result.push_str(&format!("\\x{:02X}", c as u32));
                i += 1;
            }
            _ => {
                result.push(c);
                i += 1;
            }
        }
    }
    result
}
