---
kind: record
id: spike-admission
package: H8
sources: [BLUEPRINT.md#4.7, BLUEPRINT.md#6, hooks.md#2.11.6]
---

# Spike admission

`.factory/local/hooks/spikes/`. Not tracked. The hook's record of which agent type dispatched a spike runner.
Fields in [[schema:spike-admission]].

Who asked is a fact the hook records, never something the runner says. For a dispatch with the Agent tool,
the hook records an admission (the session and the dispatching caller's agent type, empty for the main
session) when it sees the dispatch, and `spike_attach` pairs it with the oldest unclaimed admission of that
session when the runner starts, so `vsdd spike open` reads it (`spike_admit` and `spike_attach` in
[[topic:state-api]]). If more than one admission is pending and the platform does not say which
dispatch started which runner, the runner is given the union of their walls, which is the stricter result.
Inside a workflow there is no dispatch: `spike open` reads the asker from the stored `needs_spike` step
record instead.

Written by the `spike-bounds` rule ([[rule:spike-bounds]]) on `PreToolUse` for `Agent` and on
`SubagentStart`. `--asker-type` on `vsdd spike open` is a cross-check only, and a mismatch is refused.
