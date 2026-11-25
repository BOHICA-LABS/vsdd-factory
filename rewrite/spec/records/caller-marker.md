---
kind: record
id: caller-marker
package: H1
sources: [BLUEPRINT.md#4.7, BLUEPRINT.md#4.9, hooks.md#2.11.10]
---

# Caller marker

`.factory/local/hooks/calls/<key>.json`. Not tracked. The one-time marker the hook runner writes for each
`vsdd` call, so the tool knows which session and agent called. Fields in [[schema:caller-marker]].

## How it works

A `vsdd` process cannot see the hook payload, so before the runner lets a `vsdd` Bash call through it writes
a caller marker, holding the payload's `session_id`, `agent_id`, `agent_type` and `cwd` and the time; the key
is random and used once. And it returns `updatedInput` for the call, with the command rewritten to begin
`VSDD_CALLER=<key> `, so the tool can find its own marker and no other.

- `vsdd survey` refuses to start a run when its own call has no marker, so a workflow cannot run with the
  hooks disabled.
- The lock records the session that actually called.
- `vsdd spike open` learns the runner's `agent_id`, so parallel runners cannot be swapped.
- `vsdd env start` records which agent started a product, so it can be stopped when that agent stops.
- `vsdd doctor`, run through the Bash tool, finds its marker and reports the hooks as live.

A caller cannot forge it: `gate-guard` blocks a command that sets a `VSDD_*` variable itself, and the marker
directory is in a class only code writes. A marker is deleted when the tool reads it, and unread markers
older than ten minutes are pruned.

If `updatedInput` is not honoured for a call, the marker is found by a hash of the call's arguments instead
of a key ([[topic:caller-identity]]).

Written by the hook runner, before the call is judged for permission (the runner adds the prefix after every
rule has judged the command as the caller typed it). The tool reads the marker back with `caller(key)`
([[topic:state-api]]).
