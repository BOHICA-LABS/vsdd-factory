# S-15.03 AC-006 — last-amended-migrate: --check (dry-run) then apply on a
# synthetic fixture whose last_amended holds an old-style inline
# [Prior: ...] bracket chain. Demonstrates the BC-10.13.001 v1.1 PC7
# full-recovery split: the current entry stays in last_amended, and each
# chained prior entry is relocated into changelog: as a new item, newest
# first.
#
# The fixture lives under a synthetic /tmp scratch root laid out to mirror
# the 5-file ADR-049 shape (stories/STORY-INDEX.md under a throwaway
# --factory-root) because the tool's SEC-002 --path allowlist (CWE-73
# hardening) requires --path to canonicalize to exactly one of the 5
# BC-10.13.001 target files under --factory-root. No .factory/ or
# plugins/vsdd-factory/config/ file is touched by this recording.
#
# Timing uses fixed Sleep durations rather than Wait+Line: the installed
# vhs (0.11.0) does not reliably support Wait/Wait+Line pattern-matching in
# this environment (verified empirically — even a trivial `echo` + Wait+Line
# times out), so this tape follows the same Sleep-based convention already
# used by this repo's other CLI demo tapes (e.g. docs/demo-evidence/S-12.02).

Output docs/demo-evidence/S-15.03/AC-006-migrate-full-recovery-split.gif
Output docs/demo-evidence/S-15.03/AC-006-migrate-full-recovery-split.webm

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
Type "export ROOT=/tmp/lam-demo-ac006 && mkdir -p $ROOT/stories"
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
Type "clear"
Enter
Sleep 500ms
Show

# === Show the synthetic fixture: old-style inline [Prior: ...] chain ===
Type "cat $ROOT/stories/STORY-INDEX.md"
Enter
Sleep 2s

# === --check: dry-run detection, no write, nonzero exit on drift ===
Type "$BIN migrate --path $ROOT/stories/STORY-INDEX.md --factory-root $ROOT --check; echo CHECK_EXIT=$?"
Enter
Sleep 2s

# === Confirm --check made no changes to the fixture ===
Type "cat $ROOT/stories/STORY-INDEX.md"
Enter
Sleep 1500ms

# === apply (default mode, no --check): performs the PC7 full-recovery split ===
Type "$BIN migrate --path $ROOT/stories/STORY-INDEX.md --factory-root $ROOT; echo APPLY_EXIT=$?"
Enter
Sleep 2s

# === Resulting file: current-entry-only last_amended + 2 new changelog: items ===
Type "cat $ROOT/stories/STORY-INDEX.md"
Enter
Sleep 3s
