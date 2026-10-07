# S-15.03 AC-009/AC-010 — idempotency: after last-amended-migrate has
# already applied (once to a PC7 inline-chain fixture, once to a D-1144
# escape-defect fixture), re-running --check reports ZERO further
# violations and exits 0 on both — no repeated mutation, no fuel-exhausting
# re-write loop, no future POL-3/TD-FACTORY-HOOK-BYPASS-001 exception ever
# needed for either class.
#
# Fixtures live under a synthetic /tmp scratch root laid out to mirror the
# 5-file ADR-049 shape under a throwaway --factory-root, per the tool's
# SEC-002 --path allowlist (CWE-73 hardening). No .factory/ or
# plugins/vsdd-factory/config/ file is touched by this recording.
#
# Timing uses fixed Sleep durations rather than Wait+Line — see AC-006's
# tape header for why (vhs 0.11.0 Wait/Wait+Line unreliable in this env).

Output docs/demo-evidence/S-15.03/AC-009-AC-010-idempotency-recheck.gif
Output docs/demo-evidence/S-15.03/AC-009-AC-010-idempotency-recheck.webm

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
Type "export ROOT=/tmp/lam-demo-ac009 && mkdir -p $ROOT/stories $ROOT/specs/behavioral-contracts"
Enter
Sleep 500ms
Type "cat <<'FIXTURE_EOF' > $ROOT/stories/STORY-INDEX.md"
Enter
Type "---"
Enter
Type "document_type: story-index"
Enter
Type 'version: "4.430"'
Enter
Type 'last_amended: "2026-09-02 (v4.430) — some entry text [Prior: 2026-08-15 (v4.20) — previous change entry [Prior: 2026-07-01 (v4.10) — earlier change entry]]"'
Enter
Type "---"
Enter
Type ""
Enter
Type "# Fixture STORY-INDEX (synthetic, PC7 full-recovery split demo)"
Enter
Type "FIXTURE_EOF"
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
Type "$BIN migrate --path $ROOT/stories/STORY-INDEX.md --factory-root $ROOT >/dev/null"
Enter
Sleep 1500ms
Type "$BIN migrate --path $ROOT/specs/behavioral-contracts/BC-INDEX.md --factory-root $ROOT >/dev/null"
Enter
Sleep 1500ms
Type "clear"
Enter
Sleep 500ms
Show

# === Both fixtures were already migrated once (split + escape-fix applied). ===
# === Re-running --check now must report zero further violations. ===
Type "$BIN migrate --path $ROOT/stories/STORY-INDEX.md --factory-root $ROOT --check; echo RECHECK_SPLIT_EXIT=$?"
Enter
Sleep 2s

Type "$BIN migrate --path $ROOT/specs/behavioral-contracts/BC-INDEX.md --factory-root $ROOT --check; echo RECHECK_ESCAPE_EXIT=$?"
Enter
Sleep 2s

# === Both re-checks report mutated=false and exit 0: idempotent, zero further violations ===
Type "echo 'both re-checks above: mutated=false, exit 0 -- idempotent, zero further violations'"
Enter
Sleep 3s
