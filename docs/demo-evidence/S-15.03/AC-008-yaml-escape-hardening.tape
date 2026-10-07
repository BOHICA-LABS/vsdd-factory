# S-15.03 AC-008 — D-1144 YAML-escape hardening: a synthetic fixture whose
# last_amended entry contains an unescaped literal double-quote (the
# BC-INDEX.md/ARCH-INDEX.md/STATE.md defect class) is correctly escaped by
# last-amended-migrate. --check detects the defect (dry-run, no write);
# apply rewrites the value with the quote escaped as \" so the frontmatter
# parses cleanly under strict YAML.
#
# The fixture lives under a synthetic /tmp scratch root laid out to mirror
# the 5-file ADR-049 shape (specs/behavioral-contracts/BC-INDEX.md under a
# throwaway --factory-root) because the tool's SEC-002 --path allowlist
# (CWE-73 hardening) requires --path to canonicalize to exactly one of the 5
# BC-10.13.001 target files under --factory-root. No .factory/ or
# plugins/vsdd-factory/config/ file is touched by this recording.
#
# Timing uses fixed Sleep durations rather than Wait+Line — see AC-006's
# tape header for why (vhs 0.11.0 Wait/Wait+Line unreliable in this env).

Output docs/demo-evidence/S-15.03/AC-008-yaml-escape-hardening.gif
Output docs/demo-evidence/S-15.03/AC-008-yaml-escape-hardening.webm

Require cargo

Set Shell "bash"
Set FontFamily "Menlo"
Set FontSize 18
Set Width 1200
Set Height 700
Set Theme "Catppuccin Mocha"
Set WindowBar Colorful
Set BorderRadius 8
Set Padding 20
Set Framerate 30
Set TypingSpeed 40ms

Sleep 1s
Hide
Type "cd /Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-15.03"
Enter
Sleep 500ms
Type "cargo build -p last-amended-migrate --release --quiet"
Enter
Sleep 1500ms
Type "export BIN=$(pwd)/target/release/last-amended-migrate"
Enter
Sleep 300ms
Type "export ROOT=/tmp/lam-demo-ac008 && mkdir -p $ROOT/specs/behavioral-contracts"
Enter
Sleep 500ms
Type "cat <<'FIXTURE_EOF' > $ROOT/specs/behavioral-contracts/BC-INDEX.md"
Enter
Type "---"
Enter
Type "document_type: behavioral-contract-index"
Enter
Type 'version: "5.41"'
Enter
Type 'last_amended: "2026-09-02 (v5.41) — fixed the "quoted term" defect"'
Enter
Type "changelog:"
Enter
Type '  - date: "2026-08-01"'
Enter
Type '    change: "an older entry"'
Enter
Type "---"
Enter
Type ""
Enter
Type "# Fixture BC-INDEX (synthetic, D-1144 escape-hardening demo)"
Enter
Type "FIXTURE_EOF"
Enter
Sleep 500ms
Type "clear"
Enter
Sleep 500ms
Show

# === Show the synthetic fixture: unescaped literal quote in last_amended ===
Type "cat $ROOT/specs/behavioral-contracts/BC-INDEX.md"
Enter
Sleep 2s

# === --check: dry-run detection of the D-1144 escape defect ===
Type "$BIN migrate --path $ROOT/specs/behavioral-contracts/BC-INDEX.md --factory-root $ROOT --check; echo CHECK_EXIT=$?"
Enter
Sleep 2s

# === apply: rewrites last_amended with the quote escaped, changelog: untouched ===
Type "$BIN migrate --path $ROOT/specs/behavioral-contracts/BC-INDEX.md --factory-root $ROOT; echo APPLY_EXIT=$?"
Enter
Sleep 2s

# === Resulting file: embedded quote now escaped as \" ===
Type "cat $ROOT/specs/behavioral-contracts/BC-INDEX.md"
Enter
Sleep 3s
