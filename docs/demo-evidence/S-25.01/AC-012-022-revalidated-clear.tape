# S-25.01 — Named-plugin PASS re-validation clears the marker (REVALIDATED)
# AC-012 / AC-022, BC-1.18.003 postcondition 1, BC-3.08.001 Event 9 clear_mode=REVALIDATED.
#
# Seeds a durable marker (as a prior INDETERMINATE outcome would have left it), then
# drives the REAL execute_tiers path with the SAME named plugin now returning PASS on
# the SAME artifact. Shows the marker file deleted and the marker.cleared(REVALIDATED)
# audit event -- carrying the ORIGINAL plugin.indeterminate's trace_id, not the new
# dispatch's trace_id, proving provenance linkage back to the event that quarantined it.
#
# NOTE: the locally installed `vhs` (v0.11.0) `Wait`/`Wait+Line` command hangs
# indefinitely against this shell's PS1 in this environment (reproduced with a
# minimal repro tape — Sleep-based timing works, Wait does not). Sleep durations
# below are sized generously against measured command runtimes.

Output docs/demo-evidence/S-25.01/AC-012-022-revalidated-clear.gif
Output docs/demo-evidence/S-25.01/AC-012-022-revalidated-clear.webm

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

Type "./target/debug/examples/demo_s2501_marker_lifecycle revalidated-clear"
Enter
Sleep 3s

Sleep 2s
