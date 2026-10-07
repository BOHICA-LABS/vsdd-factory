# S-25.01 — 24h deadman TTL auto-clear (TTL_EXPIRED)
# AC-021, BC-1.18.001 postcondition 4, BC-1.18.003 postcondition 4,
# BC-3.08.001 Event 9 clear_mode=TTL_EXPIRED, ADR-048 Decision 2.
#
# Seeds a marker whose expires_at has already elapsed, then calls the REAL dispatcher-
# native check_and_clear_expired_marker (the same pre-check executor.rs runs before
# every Arm 1/Arm 2 plugin invocation). Shows the marker auto-deleted (idempotent,
# fail-open on stale quarantine) and the resulting marker.cleared(TTL_EXPIRED,
# actor_type=deadman) audit event.
#
# NOTE: the locally installed `vhs` (v0.11.0) `Wait`/`Wait+Line` command hangs
# indefinitely against this shell's PS1 in this environment (reproduced with a
# minimal repro tape — Sleep-based timing works, Wait does not). Sleep durations
# below are sized generously against measured command runtimes.

Output docs/demo-evidence/S-25.01/AC-021-ttl-deadman-expiry.gif
Output docs/demo-evidence/S-25.01/AC-021-ttl-deadman-expiry.webm

Require cargo

Set Shell "bash"
Set FontFamily "Menlo"
Set FontSize 18
Set Width 1200
Set Height 800
Set Theme "Catppuccin Mocha"
Set WindowBar Colorful
Set BorderRadius 8
Set Padding 20
Set Framerate 30
Set TypingSpeed 40ms

Hide
Type "cd $(git rev-parse --show-toplevel)"
Enter
Sleep 1s
Type "cargo build -q -p factory-dispatcher --example demo_s2501_marker_lifecycle"
Enter
Sleep 3s
Type "clear"
Enter
Sleep 500ms
Show

Type "./target/debug/examples/demo_s2501_marker_lifecycle ttl-expiry"
Enter
Sleep 3s

Sleep 2s
