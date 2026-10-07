# vsdd-factory Host ABI Reference

This document describes the runtime contract between WASM hook plugins
and the `factory-dispatcher` host. Plugin authors who use `vsdd-hook-sdk`
do not call this ABI directly — the SDK's [`host`](src/host.rs) module
wraps every function in safe, ergonomic Rust. This file exists for:

- ABI version negotiation (`HOST_ABI_VERSION`)
- Implementers writing alternate-language SDKs (Go, AssemblyScript, etc.)
- Forensic debugging when a plugin / dispatcher pairing misbehaves

The ABI is **stable as of `HOST_ABI_VERSION = 1`**. Breaking changes
require a major bump on both `vsdd-hook-sdk` and `factory-dispatcher`
per the semver commitment (S-5.6).

---

## Negotiation

Every plugin exports a constant `HOST_ABI_VERSION: u32`. The dispatcher
reads it before any host call and refuses to invoke plugins whose
version it does not understand. SDK 0.0.x → ABI 1; SDK 1.x → ABI 1;
SDK 2.x → ABI 2 (when it lands).

The dispatcher's compile-time `HOST_ABI_VERSION` constant is the source
of truth for what it speaks. The SDK's `vsdd_hook_sdk::HOST_ABI_VERSION`
constant is the source of truth for what plugins built against that SDK
expect.

---

## Plugin entry point

WASI preview-1 command convention. The dispatcher invokes the
auto-generated `_start` function. The `#[hook]` macro emits a `fn main()`
that drives the user function:

1. Reads stdin to a `Vec<u8>`.
2. Deserializes JSON into `HookPayload`.
3. Calls the user's `fn(HookPayload) -> HookResult` with a
   `std::panic::catch_unwind` boundary so a panicking plugin terminates
   cleanly rather than aborting the wasmtime store.
4. Serializes the `HookResult` JSON to stdout.
5. Calls `std::process::exit` with the result's exit code.

### Stdin envelope (host → plugin)

UTF-8 JSON. Schema:

```json
{
  "event_name": "PreToolUse",
  "tool_name": "Bash",
  "session_id": "abc-123",
  "dispatcher_trace_id": "uuidv4",
  "tool_input": { "...": "tool-specific" },
  "tool_response": null
}
```

`tool_response` is `null` for `PreToolUse` and most pre-call events;
present for `PostToolUse` and lifecycle events that carry a result.

### SubagentStop stdin envelope

When Claude Code fires a `SubagentStop` event the dispatcher writes the
following shape to plugin stdin.  The four SubagentStop-specific fields
(`agent_type`, `subagent_name`, `last_assistant_message`, `result`) are
**top-level** — not nested inside `tool_input` or `tool_response`.

```json
{
  "event_name": "SubagentStop",
  "session_id": "abc-123",
  "dispatcher_trace_id": "uuidv4",
  "tool_name": "",
  "tool_input": null,
  "tool_response": null,
  "agent_type": "product-owner",
  "subagent_name": "product-owner-fallback",
  "last_assistant_message": "Wrote the story spec and committed.",
  "result": "fallback-result-string"
}
```

**Field presence semantics**

| Field | Type in `HookPayload` | Present on | Absent behaviour |
|---|---|---|---|
| `agent_type` | `Option<String>` | SubagentStop only | `None` (via `#[serde(default)]`) |
| `subagent_name` | `Option<String>` | SubagentStop only | `None` |
| `last_assistant_message` | `Option<String>` | SubagentStop only | `None` |
| `result` | `Option<String>` | SubagentStop only | `None` |

JSON `null` deserializes to `None` for all four fields — identical to jq's
`//` null-advance semantics.

**Canonical fallback chains (BC-2.02.012 Postconditions 5–6)**

```rust
// Agent identity — mirrors: jq -r '.agent_type // .subagent_name // "unknown"'
let agent = payload.agent_type.as_deref()
    .or(payload.subagent_name.as_deref())
    .unwrap_or("unknown");

// Assistant message — mirrors: jq -r '.last_assistant_message // .result // empty'
let message = payload.last_assistant_message.as_deref()
    .or(payload.result.as_deref())
    .unwrap_or("");
```

Both chains are normative.  Plugin implementations for S-8.01, S-8.02,
S-8.03, and S-8.05 MUST use these expressions; divergence requires an
explicit rationale (see BC-2.02.012 EC-004 for the `handoff-validator`
three-stage `output` fallback).

**BC reference:** BC-2.02.012 — "HookPayload SubagentStop fields: top-level
envelope schema for agent_type, subagent_name, last_assistant_message,
result."

**ABI version:** These fields are an additive `HookPayload` extension
under D-6 Option A and D-183.  `HOST_ABI_VERSION` remains `1`.

---

### Stdout envelope (plugin → host)

UTF-8 JSON, single-line, terminated with `\n`. One of:

```json
{ "outcome": "continue" }
{ "outcome": "block",    "reason":  "<short string>" }
{ "outcome": "error",    "message": "<diagnostic>" }
```

### Exit code contract

| `outcome` | Exit code | Meaning to dispatcher |
|---|---|---|
| `continue` | `0` | Allow the tool call / non-blocking event |
| `block`    | `2` | Block (PreToolUse / PermissionRequest only) |
| `error`    | `1` | Plugin failed; non-blocking unless `on_error = "block"` |

---

## Advisory block-mode pattern

Plugins that need to advise the dispatcher to block downstream actions (e.g., a
pr-manager subagent that has not completed all 9 steps) emit a JSON line to stdout:

    {"outcome":"block","reason":"<short_machine_string>"}

Optionally with `"stderr":"<operator-visible message>"` to inform the operator.

The dispatcher checks for this line BEFORE returning to the parent agent. If
`outcome=block`, the dispatcher exits with a non-zero status and propagates the
reason.

The `on_error` field in `hooks-registry.toml` controls how the dispatcher reacts
to a plugin that *crashes* (returns a non-zero exit code, panics, or otherwise
fails). The advisory-block mechanism is independent of `on_error` — any plugin
that writes `{"outcome":"block","reason":"..."}` to stdout records a
dispatcher-level block intent regardless of `on_error`. As of v1.0,
`on_error="block"` and `on_error="continue"` produce identical behavior for
block_intent aggregation; the field is preserved for forward-compatibility with
potential future crash-to-block escalation policies (W-16+).

The SDK's `HookResult::Block(reason)` variant is reserved for future hard-block
semantics (planned W-16 / v1.1). All v1.0 plugins use `HookResult::Continue` +
stdout outcome line.

**Affected plugins (canonical v1.0 advisory-block-mode users):**

- `handoff-validator` — emits `hook.block` event + writes
  `{"outcome":"block","reason":"handoff_empty_result"}` (empty path) or
  `{"outcome":"block","reason":"handoff_truncated_result"}` (truncated path) to
  stdout + returns `HookResult::Continue`
- `pr-manager-completion-guard` — emits `hook.block` event + writes
  `{"outcome":"block","reason":"pr_manager_incomplete_lifecycle"}` to stdout +
  returns `HookResult::Continue`
- `validate-pr-review-posted` — emits `hook.block` event + writes
  `{"outcome":"block","reason":"pr_review_invalid"}` to stdout when any
  pre-merge check fails + returns `HookResult::Continue`

**Decision record:** D-W15-gate-003 — W-15 gate fix: canonical advisory-block-mode
pattern chosen (stdout emit, not HookResult::Block); HookResult::Block SDK
extension deferred to W-16. See `CRIT-W15-002 + HIGH-W15-003` in the W-15 gate
adversary review.

---

## Async Hook Semantics (S-15.01)

### Registry-layer partition

When the dispatcher loads `hooks-registry.toml` and receives a Claude Code hook event, it
calls `partition::partition_plugins` to split the matched entries into two disjoint groups:

| Group | Criterion | Behaviour |
|-------|-----------|-----------|
| `sync_group` | `async` field absent or `async = false` | Awaited to completion; block verdicts gate Claude Code |
| `async_group` | `async = true` | Fire-and-forget tokio tasks; verdicts never gate Claude Code |

Every matched entry appears in exactly one group (disjoint and exhaustive — BC-1.14.001
Postcondition 1; VP-077 Kani proof).

Source: `crates/factory-dispatcher/src/partition.rs::partition_plugins`

### Sync execution

`executor.rs::execute_tiers(&sync_group)` drives sync-group plugins through the ADR-008
tier model: plugins within the same priority tier execute concurrently (each wrapped in
`tokio::task::spawn_blocking` so that wasmtime's synchronous execution does not block the
async runtime); tiers themselves are sequential (lower priority value fires first).

The dispatcher awaits all sync-group completions before computing a verdict. A `Block`
outcome from any sync-group plugin sets `block_intent = true` and the dispatcher exits with
code 2 — Claude Code reads that exit code and enforces the gate.

Source: `crates/factory-dispatcher/src/executor.rs::execute_tiers`

### Async execution

After `sync_group` completes, `executor.rs::spawn_async_plugin` spawns each async-group
entry as an independent `tokio::spawn` task. Async-group plugins are NOT passed through
`execute_tiers` and NOT subject to the ADR-008 tier ordering model — they are unordered
and fire-and-forget relative to each other and to sync-group execution (BC-1.14.001
Invariant 3).

Source: `crates/factory-dispatcher/src/executor.rs::spawn_async_plugin`

### Drain window

After spawning all async tasks, the dispatcher waits up to `ASYNC_DRAIN_WINDOW_MS` (defined
in DI-019) for async tasks to emit terminal events before the dispatcher process exits. The
drain is implemented via `tokio::select!` over per-task result channels and a drain timer.
The drain window is a bounded constant — dispatcher latency is bounded by:

```
max(sync_plugin_durations_in_slowest_tier) + ASYNC_DRAIN_WINDOW_MS
```

Tasks that complete within the drain window emit their terminal events (`plugin.timeout`,
`plugin.async_block_discarded`) to the FileSink cleanly. Tasks still executing when the
drain timer fires are forcibly terminated; their terminal events may be lost (truncated
telemetry is an accepted cost for async plugins — BC-1.14.001 EC-011).

Refs: DI-019 (canonical value for `ASYNC_DRAIN_WINDOW_MS`); BC-1.14.001 PC4.

### Dispatcher exit semantics

The dispatcher returns to Claude Code only after:

1. All sync-group hooks complete (every tier in priority order).
2. The drain window expires (or all async tasks finish, whichever comes first).

The dispatcher exit code is determined solely by sync-group results:

- `0` — all sync-group plugins continued (or sync_group is empty).
- `2` — at least one sync-group plugin emitted `{"outcome":"block","reason":"..."}`.

Async-group block verdicts are structurally discarded and logged as
`plugin.async_block_discarded` events. They never produce exit code 2.

Ref: ADR-019 §Decision 3

---

## Registry Entry Schema

### `async` field (S-15.01)

Each `[[hooks]]` entry in `hooks-registry.toml` may declare its async classification:

```toml
[[hooks]]
name    = "session-start-telemetry"
event   = "SessionStart"
plugin  = "hook-plugins/session-start-telemetry.wasm"
async   = true                     # fire-and-forget; does not gate Claude Code

[[hooks]]
name    = "validate-stable-anchors"
event   = "PreToolUse"
matcher = "Edit|Write"
plugin  = "hook-plugins/validate-stable-anchors.wasm"
on_error = "block"                 # async = false (default) — sync gate
```

**TOML wire field:** `async = true | false`. Absent field is equivalent to `async = false`
via `#[serde(default)]`.

**Rust field:** `RegistryEntry.async_flag: bool` with `#[serde(default, rename = "async")]`.
`async` is a Rust reserved keyword; the field is renamed to `async_flag` in the Rust source
while preserving the `async` key on the TOML wire format.

**Per-hook granularity:** Each entry declares independently. The same plugin binary may be
registered as both async and sync hooks for different events:

```toml
[[hooks]]
name  = "telemetry-post"
event = "PostToolUse"
plugin = "hook-plugins/capture-activity.wasm"
async = true      # observation only

[[hooks]]
name  = "gate-pre"
event = "PreToolUse"
plugin = "hook-plugins/validate-anchors.wasm"
# async = false (default) — sync gate
```

**Schema version:** The `async` field was introduced with `schema_version = 2`
(REGISTRY_SCHEMA_VERSION = 2). Registries with `schema_version != 2` are rejected at load
time with E-REG-001.

Refs: BC-7.06.001 (registry invariants); ADR-019 §Decision 2.

---

## Plugin Author Async Guidance

Use this decision matrix when classifying a hook plugin:

| Hook category | `async` setting | Rationale |
|---------------|-----------------|-----------|
| Telemetry / OTel emission | `true` | Pure side-effect; no decision returned |
| Audit logging | `true` | Append-only; no gate |
| Activity tracking | `true` | Observation only |
| Learning extraction | `true` | Async-friendly; no caller dependency |
| State updates (post-merge etc.) | `true` if not blocking | Caller does not depend on completion |
| Validation gates | `false` | Block decision must reach Claude Code |
| Security / secrets / artifact-path checks | `false` | Block decision must enforce |
| Anti-fabrication / discipline guards | `false` | Block decision required |
| Worktree / filesystem mutations Claude Code depends on | `false` | Side-effect must complete before Claude Code proceeds |

**Warning: async plugins cannot block.** Async hooks return `HookResult` from the plugin's
perspective, but the dispatcher does NOT propagate a `Block` verdict to Claude Code's gate —
the `async_group` spawn is fire-and-forget and the block decision arrives after the gate
window has already closed. An async plugin that emits `{"outcome":"block","reason":"..."}`:

- Records a `plugin.async_block_discarded` event in `events-*.jsonl` for observability.
- Does NOT prevent the tool call from proceeding.
- Does NOT produce dispatcher exit code 2.

Treat `Block` from an async plugin as advisory-only diagnostic output. If enforcement is
required, the plugin MUST be `async = false`.

Additionally, combining `on_error = "block"` with `async = true` is a **hard registry
error** (E-REG-002) — the dispatcher refuses to start. See §Async Failure Modes below.

---

## Async Failure Modes

### `plugin.timeout` (async path)

When an async-group plugin exceeds its configured `timeout_ms`, the dispatcher emits:

```
host::emit_event::emit_plugin_timeout_async(ctx, plugin_name, timeout_ms)
```

Wire schema (JSON line in `events-*.jsonl`):

```json
{
  "type": "plugin.timeout",
  "trace_id": "<uuid-v4>",
  "session_id": "<uuid-v4>",
  "plugin_name": "<registry entry name>",
  "execution_group": "async",
  "timeout_ms": <integer>,
  "timestamp": "<ISO-8601>"
}
```

Mandatory fields: `type`, `trace_id`, `session_id`, `plugin_name`, `execution_group`,
`timeout_ms`, `timestamp`.

Note: `plugin.timeout` is also emitted for sync-path timeouts (governed by BC-1.14.001).
The `execution_group: "async"` field distinguishes the async-path variant. The dispatcher
exit code is NOT affected by an async-path timeout.

Note: `timeout_ms` is the per-plugin budget from the registry entry, not the drain window.
The drain window is `ASYNC_DRAIN_WINDOW_MS` (DI-019) — a separate, independent constant.

Source: `crates/factory-dispatcher/src/host/emit_event.rs::emit_plugin_timeout_async`

Ref: BC-3.08.001 Event 4; DI-019.

### E-REG-002 AsyncBlockConflict

A registry entry that declares both `on_error = "block"` and `async = true`
simultaneously is an E-REG-002 violation. This combination is contradictory: a hook whose
crash policy is "block Claude Code" structurally cannot be async (the async execution path
discards block verdicts). The dispatcher enforces this at registry-load time:

- `registry.rs::Registry::load` calls `validate_async_block_invariant()` during parsing.
- On violation: `RegistryError::AsyncBlockConflict { name }` is returned.
- Dispatcher emits `dispatcher.registry_invalid` (E-REG-002) and exits with code 2
  (fail-closed per ADR-019 §Decision 2; explicit exception to BC-1.08.001 fail-open).

The error message names the offending plugin entry.

To resolve: either remove `on_error = "block"` (if the plugin should be async) or remove
`async = true` (if the plugin must enforce a block gate).

Source: `crates/factory-dispatcher/src/registry.rs::RegistryEntry` (field validation),
`crates/factory-dispatcher/src/registry.rs::Registry::load`

Refs: BC-7.06.001 Invariant 1; BC-1.14.001 Invariant 4; ADR-019 §Decision 4.

---

## `dispatcher.registry_invalid` Wire Format (B-3)

The `dispatcher.registry_invalid` event is emitted when a registry entry violates a
load-time invariant. Two error codes trigger this event, each with a distinct wire schema.
The dispatcher provides separate type-safe emit functions for each — compile-time
enforcement replaces documentation-only invariants (PR #109, B-3).

### E-REG-002 variant (`emit_registry_invalid_e_reg002`)

```rust
host::emit_event::emit_registry_invalid_e_reg002(ctx, plugin_name, violation)
```

E-REG-002 is an **intra-entry violation**: a single entry simultaneously has
`on_error = "block"` and `async = true`. No second entry is involved, so no
`offending_event` or `offending_tool` fields apply.

Wire schema:

```json
{
  "type": "dispatcher.registry_invalid",
  "trace_id": "<uuid-v4>",
  "session_id": "<uuid-v4>",
  "offending_plugin": "<registry entry name>",
  "violation": "async_block_conflict",
  "error_code": "E-REG-002",
  "timestamp": "<ISO-8601>"
}
```

Fields `offending_event` and `offending_tool` are **absent** from E-REG-002 payloads. This
is intentional — the intra-entry nature of the violation means no event/tool tuple applies.

### E-REG-003 variant (`emit_registry_invalid_e_reg003`)

```rust
host::emit_event::emit_registry_invalid_e_reg003(
    ctx,
    plugin_name,
    violation,
    offending_event,       // &str — mandatory
    offending_tool,        // Option<&str> — None for wildcard ("all tools") binding
)
```

E-REG-003 is an **inter-entry violation**: two registry entries share the same
`(name, event, tool)` tuple (DuplicateEntry). The specific tuple is propagated to the event
payload because it uniquely identifies which entry is the duplicator.

Wire schema with wildcard tool binding (`offending_tool = None` → JSON `null`):

```json
{
  "type": "dispatcher.registry_invalid",
  "trace_id": "<uuid-v4>",
  "session_id": "<uuid-v4>",
  "offending_plugin": "<duplicate entry name>",
  "offending_event": "PreToolUse",
  "offending_tool": null,
  "violation": "duplicate_hook_registration",
  "error_code": "E-REG-003",
  "timestamp": "<ISO-8601>"
}
```

Wire schema with explicit tool binding (`offending_tool = Some("Bash")` → JSON string):

```json
{
  "type": "dispatcher.registry_invalid",
  "trace_id": "<uuid-v4>",
  "session_id": "<uuid-v4>",
  "offending_plugin": "protect-secrets",
  "offending_event": "PreToolUse",
  "offending_tool": "Bash",
  "violation": "duplicate_hook_registration",
  "error_code": "E-REG-003",
  "timestamp": "<ISO-8601>"
}
```

`offending_tool: null` means the duplicating entry matched all tools (no `tool` filter
declared). `offending_tool: "<regex>"` means it matched a specific tool pattern.

Implementations MUST propagate all three tuple fields (`offending_plugin`,
`offending_event`, `offending_tool`) for E-REG-003. Omitting `offending_event` or
`offending_tool` is a BC violation (BC-3.08.001 Invariant per F-P14-001 Path B).

### Field asymmetry summary

| Field | E-REG-002 | E-REG-003 |
|-------|-----------|-----------|
| `offending_plugin` | present | present |
| `offending_event` | **absent** | present (mandatory) |
| `offending_tool` | **absent** | present (null or string) |
| `violation` | `"async_block_conflict"` | `"duplicate_hook_registration"` |
| `error_code` | `"E-REG-002"` | `"E-REG-003"` |

Source: `crates/factory-dispatcher/src/host/emit_event.rs::emit_registry_invalid_e_reg002`,
`crates/factory-dispatcher/src/host/emit_event.rs::emit_registry_invalid_e_reg003`

Refs: BC-3.08.001 v1.8 (Event 3); BC-7.06.001 Invariants 1, 7; F-P14-001 Path B.

---

## Filesystem Access Model

### WASI preopened directories

All plugins receive a single WASI preopened directory: the project root
(the dispatcher's `host_ctx.cwd`, derived from `CLAUDE_PROJECT_DIR`), mounted
as `.` in the guest with `FsPerms::ReadWrite` (wasmtime-wasi 48; formerly
`DirPerms::all() | FilePerms::all()` — semantics unchanged). No other
directory is preopened. This means any plugin can read and write within the
project root using native WASI filesystem calls (`std::fs::read`,
`std::fs::write`, etc.) — no capability declaration required.

### host::write_file capability

The `host::write_file` host function provides an **additional** bounded-write
mechanism with BC-2.02.011 enforcement (`max_bytes_per_call`, `path_allow`
list). Plugins that declare a `write_file` capability block in
`hooks-registry.toml` use this path for guarded writes.

### Relationship

WASI preopened access is the **sandbox boundary**. The `host::write_file`
capability gate controls only the host function — it does NOT constrain native
WASI filesystem calls. A plugin with no `write_file` capability declared can
still read and write the preopened directories via standard Rust `std::fs`.

### v1.1 roadmap

Future releases (v1.1) will tighten preopens to read-only by default; write
access will require an explicit capability declaration. This closes the gap
between WASI preopened access and the `host::write_file` allow-list boundary.
See `CRIT-W15-003 / SEC-001` in the W-15 gate security review and the comment
in `crates/factory-dispatcher/src/invoke.rs` near `preopened_dir(...)`.

**Decision record:** D-W15-gate-004 — W-15 gate fix: WASI preopened_dir vs
`write_file` capability model documented; capability tightening deferred to v1.1.

---

## Host functions

All host functions are imported under the WASI module name `vsdd`.
Strings are passed as `(ptr: u32, len: u32)` UTF-8 byte tuples; out-strings
follow the `(out_ptr: u32, out_cap: u32) -> u32 bytes_written` convention.
Negative return codes signal errors:

| Code | Meaning |
|---:|---|
| `-1` | Capability denied (caller lacks the registry-declared capability) |
| `-2` | Timeout exceeded the call's `timeout_ms` |
| `-3` | Output exceeded `max_output_bytes`; truncated |
| `-4` | Invalid argument (path traversal, unknown env name, etc.) |
| Other | Reserved for future categorized errors |

### `log(level, msg_ptr, msg_len) -> ()`

Write `msg` (UTF-8) to the dispatcher's internal log at `level`.

| Level | Value |
|---|---|
| Trace | 0 |
| Debug | 1 |
| Info  | 2 |
| Warn  | 3 |
| Error | 4 |

Always succeeds. No return value, no error path.

### `emit_event(type_ptr, type_len, fields_ptr, fields_len) -> ()`

Emit a structured event. `type` is the event name (e.g. `"commit.made"`).
`fields` is a length-prefixed sequence of UTF-8 key/value pairs:

```
[ key_len: u32 LE | key: u8 × key_len | value_len: u32 LE | value: u8 × value_len ]+
```

The dispatcher enriches the event with `dispatcher_trace_id`,
`session_id`, `plugin_name`, `plugin_version`, `ts`, `ts_epoch`, and
`schema_version` automatically.

### `read_file(path_ptr, path_len, max_bytes, timeout_ms, out_ptr_out, out_len_out) -> i32`

Read a file under the dispatcher's read allow-list. The host writes the
returned buffer's `(ptr, len)` into the two out-params; the SDK copies
the bytes and surfaces them to the plugin.

`max_bytes` and `timeout_ms` are mandatory. Return: `0` on success or a
negative error code.

### `exec_subprocess(cmd_ptr, cmd_len, args_ptr, args_len, stdin_ptr, stdin_len, timeout_ms, max_output_bytes, result_buf_ptr, result_buf_cap) -> i32`

Run a subprocess against the dispatcher's binary allow-list. `cmd` is the
basename or absolute path; `args` is a length-prefixed sequence of
arguments (same encoding as `emit_event`'s fields, key-only).

`(stdin_ptr, stdin_len)` is an optional payload written to the
subprocess's stdin. Pass `(_, 0)` for no stdin (the host wires
`Stdio::null()` and skips the write). The legacy-bash-adapter (S-2.1)
uses this to forward the Claude Code hook envelope unchanged to bash
hooks.

`(result_buf_ptr, result_buf_cap)` points to a guest-pre-allocated
buffer. The host writes the result envelope into it:

```
[ exit_code: i32 LE | stdout_len: u32 LE | stdout: u8 × stdout_len | stderr_len: u32 LE | stderr: u8 × stderr_len ]
```

Callers should size the buffer at `max_output_bytes + 16` (12 bytes of
length headers + 4 bytes slack). The SDK wrapper does this automatically.

Return:

- `> 0` — number of bytes written to the result buffer (the SDK reads
  exactly that many bytes and decodes the envelope).
- `0` — never returned in practice; the envelope always carries at
  least the exit-code header.
- `< 0` — error code: `-1` capability denied, `-2` timeout, `-3` output
  larger than `max_output_bytes` (or buffer too small), `-4` invalid
  argument.

`timeout_ms` and `max_output_bytes` are mandatory.

### `session_id(out_ptr, out_cap) -> u32`
### `dispatcher_trace_id(out_ptr, out_cap) -> u32`
### `plugin_root(out_ptr, out_cap) -> u32`
### `plugin_version(out_ptr, out_cap) -> u32`
### `cwd(out_ptr, out_cap) -> u32`

Retrieve a fixed string from the host. The host writes UTF-8 bytes into
`out_ptr` (up to `out_cap`) and returns the number of bytes written. If
the value is longer than `out_cap`, the host returns the **required
capacity** without writing partial data; the SDK re-calls with a larger
buffer.

### `env(name_ptr, name_len, out_ptr, out_cap) -> i32`

Read a single environment variable. Returns:

- `>= 0` — number of bytes written (`0` = variable unset)
- `< 0`  — error code (typically `-1` / capability denied if the name is
  not on the dispatcher's env allow-list)

### `write_file(path_ptr, path_len, contents_ptr, contents_len, max_bytes, timeout_ms) -> i32`

Write a guest-owned byte slice to the filesystem through the dispatcher's
bounded host function (BC-2.02.011 — additive ABI extension, D-6 Option A;
`HOST_ABI_VERSION` stays at 1).

**Protocol:** input-pointer — the SDK passes guest-owned bytes;
the dispatcher copies them via `read_wasm_bytes`. This is the **inverse**
of `read_file`'s output-pointer protocol and the two must not be confused.

**Parameters:**

| Parameter | Type | Description |
|---|---|---|
| `path_ptr` | `u32` | Pointer to UTF-8 path in guest memory |
| `path_len` | `u32` | Byte length of path |
| `contents_ptr` | `u32` | Pointer to content bytes in guest memory |
| `contents_len` | `u32` | Byte length of content |
| `max_bytes` | `u32` | Mandatory byte cap; content exceeding this returns `-3` |
| `timeout_ms` | `u32` | Mandatory timeout budget; accepted for ABI stability (epoch interruption enforced in S-1.5) |

**Return values:**

| Code | Meaning |
|---|---|
| `0` | Success; full byte slice durably written to `path` |
| `-1` | Capability denied: path not in `capabilities.write_file.path_allow`, path traversal attempt, or no `write_file` capability block present |
| `-2` | Timeout exceeded `timeout_ms` |
| `-3` | Content length exceeded `max_bytes` cap; **no bytes written to disk** |
| `-4` | Invalid argument (e.g. UTF-8 path decoding failure) |
| `-99` | Filesystem I/O error or missing parent directory |

**Safety policy:** path must be within the plugin's declared
`capabilities.write_file.path_allow` list. Traversal attempts (`..`) return
`-1` (same as `read_file`). Deny-by-default: no capability block → `-1`.

---

## Future ABI versions

**ABI 2** (post-1.0, tentative) is expected to land alongside the WASI
preview-2 / component-model migration:

- Component-model exports replace `_start`
- Strings as Component Model `string` instead of byte tuples
- Async host functions where it makes sense

ABI 2 will not be back-compatible with ABI 1 plugins; the dispatcher
will load both during a transition window per the semver commitment.

---

## Block-message convention (canonical Why/Fix/Code)

All blocking hooks (bash + WASM) emit a single-line stderr message or
`permissionDecisionReason` in the form:

    BLOCKED by <hook-name>: <reason>. Fix: <recommendation>. Code: <code>.

This format survives the legacy-bash-adapter's stderr-capture path
(which reads only the first stderr line up to a 4 KiB cap) and ensures
every blocking hook tells the agent (a) why the block fired and (b) what
to do next.

### Bash hooks

Source `${CLAUDE_PLUGIN_ROOT}/hooks/lib/block.sh` and call one of:

```bash
# Standard PreToolUse / PostToolUse block (exit 2 + single-line stderr):
source "${CLAUDE_PLUGIN_ROOT}/hooks/lib/block.sh"
block_pre <hook-name> "<reason>" "<recommendation>" "<code>"

# JSON-envelope deny (for hooks that use permissionDecision: deny):
source "${CLAUDE_PLUGIN_ROOT}/hooks/lib/block.sh"
block_pre_json <hook-name> "<reason>" "<recommendation>" "<code>"
```

`block_pre` automatically:
- Strips trailing periods from reason and recommendation to avoid double periods.
- Emits telemetry via `bin/emit-event` if available (silent no-op otherwise).
- Writes the canonical line to stderr and exits 2.

`block_pre_json` does the same but emits a JSON envelope to stdout with
`permissionDecision: "deny"` and `permissionDecisionReason` set to the
canonical line, then exits 0. Falls back to `block_pre` if `jq` is absent.

### WASM hooks

Use `HookResult::block_with_fix(hook, reason, recommendation, code)`
instead of `HookResult::block(reason)`:

```rust
use vsdd_hook_sdk::HookResult;

// Preferred (canonical Why/Fix/Code format):
HookResult::block_with_fix(
    "my-hook",
    "Descriptive reason why the block fired",
    "Actionable recommendation for how to fix it",
    "snake_case_code",
)

// Avoid (no recommendation, no code — harder to diagnose):
HookResult::block("bare reason string")
```

### Rules for hook authors

1. `<hook-name>` must be the kebab-case hook filename without `.sh` /
   `.wasm` suffix (e.g., `verify-git-push`, `block-ai-attribution`).
2. `<reason>` answers "why did this block fire?" — be specific, include
   the offending value where safe to do so.
3. `<recommendation>` answers "what should I do to unblock?" — be
   actionable (include a command or file to edit where possible).
4. `<code>` is a `snake_case` identifier used for telemetry bucketing.
   Use a stable, descriptive code (e.g., `git_push_force`, `bc_green_immutable`).
5. Do NOT end `<reason>` or `<recommendation>` with a period — the
   helper appends exactly one period to each field.

### Related rules

- `plugins/vsdd-factory/hooks/lib/block.sh` — the bash helper implementation.
- `crates/vsdd-hook-sdk/src/lib.rs` — the `HookResult::block_with_fix` Rust API.
- `tests/integration/hooks/block-helper.bats` — unit tests for the bash helper.
- `tests/integration/hooks/canonical-format-invariant.bats` — per-hook regression tests.

---

## Context Injection Contract

This section documents the **context injection** platform: a factory-agnostic mechanism by
which the dispatcher enriches a hook's `plugin_config` with runtime context values before
each dispatch. Context is produced by **resolver plugins** — separate WASM artifacts that
read domain-specific data and return a structured output that the dispatcher merges into
`plugin_config`. The hook plugin receives the enriched `plugin_config` and has no visibility
into whether its configuration came from the static registry or from resolver output.

The resolver ABI is versioned independently from the hook ABI:

```
RESOLVER_ABI_VERSION = 1
```

`RESOLVER_ABI_VERSION` and `HOST_ABI_VERSION` evolve on separate tracks. A bump to one
does not imply a bump to the other. (BC-4.12.002)

---

### Overview

The context injection flow is:

1. At dispatcher startup, the dispatcher loads `resolvers-registry.toml` (a separate file
   from `hooks-registry.toml`) and compiles each declared resolver WASM artifact into a
   `Module` held in the resolver cache.
2. When a hook dispatch is triggered, the dispatcher reads the `needs_context` field of the
   matched hooks-registry entry.
3. For each resolver name declared in `needs_context`, the dispatcher creates a fresh
   `Store`, invokes the resolver's `resolve()` export with a `ResolverInput`, and receives a
   `ResolverOutput`.
4. Each resolver's output is merged additively into `plugin_config` under the resolver's
   declared key.
5. The hook plugin is dispatched with the enriched `plugin_config`.

A hook with an empty or absent `needs_context` field skips all resolver invocations
entirely. This is the **zero-overhead path**: no resolver code executes, no Store is
created, and hook dispatch latency is unaffected. (BC-1.13.001)

---

### Resolver Registration (`resolvers-registry.toml`)

Resolvers are registered in `resolvers-registry.toml`. This file is **distinct** from
`hooks-registry.toml`: it has a different schema, a different lifecycle role (pre-dispatch
data providers, not event handlers), and is versioned independently. The dispatcher loads
both files at startup. (BC-1.13.001 INV7, ADR-018 OD-2)

Example entry:

```toml
[[resolvers]]
name        = "my-context"
plugin      = "hook-plugins/my-context-resolver.wasm"
path_allow  = [".factory/"]
```

Fields:

| Field | Required | Description |
|-------|----------|-------------|
| `name` | yes | The resolver's unique name. Matches the string declared in `needs_context`. This name is the key under which the resolver's output is written in `plugin_config`. |
| `plugin` | yes | Relative path to the `.wasm` artifact (relative to the registry file's directory). |
| `path_allow` | yes | List of path prefixes the resolver may read via `host::read_file`. Empty list = no reads allowed (deny-by-default). |
| `fail_closed` | no (default: `true`) | Controls startup behavior if the resolver `.wasm` artifact fails to compile. `true` (default): dispatcher emits `resolver.load_error` and fails startup — fail-loud semantics. `false`: the entry is skipped with a warning and the dispatcher starts with the remaining resolvers — fail-open for optional resolvers. (BC-4.12.001 PC6) |

**Critical constraint (BC-1.13.001 PC1):** If `resolvers-registry.toml` is absent at
startup, the dispatcher initializes with zero resolvers configured — this is NOT a startup
error. An absent `resolvers-registry.toml` yields zero resolvers and MUST NOT be treated
as a startup error. Existing deployments without the file behave identically to before this
feature. No error is emitted and no hook dispatch is blocked. Only when the file is present
but malformed does the dispatcher fail loudly.

Duplicate `name` entries in `resolvers-registry.toml` are a **startup error** (fail-loud):
the dispatcher emits `resolver.load_error` and does not start with a partial resolver set.
(BC-4.12.005 PC6, EC-004)

---

### `needs_context` Field in `hooks-registry.toml`

Each hooks-registry entry may declare which resolvers it requires:

```toml
[[hooks]]
name         = "my-hook"
plugin       = "hook-plugins/my-hook.wasm"
needs_context = ["my-context"]
```

The `needs_context` field is a `Vec<String>` that defaults to `[]` when absent. All
existing hooks-registry entries that omit the field parse correctly (backward-compatible
via `#[serde(default)]`).

**Semantics:**

- If `needs_context = []` or the field is absent, the dispatcher skips resolver invocation
  entirely and dispatches the hook with the unmodified `plugin_config` (zero-overhead path).
- If `needs_context = ["my-context"]`, the dispatcher invokes the `my-context` resolver
  before dispatch and merges its output into `plugin_config["my-context"]`.
- If a `needs_context` entry names a resolver that is not registered, the dispatcher emits
  a `resolver.not_found` event and dispatch proceeds without context injection for that
  entry. The hook receives a `plugin_config` that lacks the expected key. (BC-1.13.001 PC6)

**`resolver.not_found` event fields:**

| Field | Type | Description |
|-------|------|-------------|
| `resolver_name` | string | The name of the resolver that was requested but not found in the registry. |
| `trace_id` | string | Dispatcher trace ID for the dispatch event. |
| `session_id` | string | Claude Code session identifier. |
| `plugin_name` | string | The hook plugin name that declared this resolver in `needs_context`. |

Emitted when a hook entry's `needs_context` list references a resolver name not registered
in `resolvers-registry.toml`. The dispatcher continues dispatch (the missing resolver
contributes no key to `plugin_config`); see BC-1.13.001 PC6.

**`resolver.registry_loaded` event fields:**

| Field | Type | Description |
|-------|------|-------------|
| `resolver_count` | integer | Number of resolver modules successfully compiled at startup. |
| `registry_path` | string | Path to the `resolvers-registry.toml` file that was loaded. |
| `trace_id` | string | Dispatcher trace ID for the startup event. |
| `session_id` | string | Claude Code session identifier. |

Emitted at dispatcher startup after `ResolverLoader::load_registry` succeeds and at least
one resolver compiled successfully. Not emitted when the registry is absent (zero-resolver
case is not an error — see BC-1.13.001 PC1).

**`resolver.load_warning` event fields:**

| Field | Type | Description |
|-------|------|-------------|
| `resolver_name` | string | Name of the resolver entry that was skipped (`fail_closed = false`). |
| `error_detail` | string | Reason the resolver was skipped (compile error, missing file, etc.). |
| `trace_id` | string | Dispatcher trace ID for the startup event. |
| `session_id` | string | Claude Code session identifier. |

Emitted at registry-load time when a resolver entry has `fail_closed = false` and fails to
compile or load. The dispatcher continues startup with the remaining resolvers (fail-open
semantics). One event is emitted per skipped resolver entry. (BC-4.12.001 PC6)

**`resolver.load_error` event fields:**

| Field | Type | Description |
|-------|------|-------------|
| `error_detail` | string | Description of the registry-load failure (TOML parse error, fail-closed compile failure, duplicate name, schema violation). |
| `trace_id` | string | Dispatcher trace ID for the startup event. |
| `session_id` | string | Claude Code session identifier. |

Emitted when `ResolverLoader::load_registry` returns `Err` — i.e. when a fail-closed
resolver fails to compile, the registry TOML is malformed, or a duplicate `name` entry is
detected. The dispatcher does NOT start with a partial resolver set; startup is aborted
after emitting this event. (BC-4.12.001 PC6, BC-4.12.005 PC6)

---

### Resolver Lifecycle (BC-4.12.001)

The resolver lifecycle follows the same compile-once, instantiate-per-call pattern as hook
plugins:

**Startup (Module compilation):**
- The dispatcher loads each resolver WASM artifact listed in `resolvers-registry.toml` and
  compiles it into a `Module` at dispatcher startup.
- Module compilation is amortized across all dispatches within the process lifetime.
- The module cache is keyed by `(canonical_path, mtime)` — the same mtime-based cache
  invalidation pattern used by `plugin_loader.rs`.

**Per-dispatch (Store creation):**
- Each resolver invocation creates a fresh `Store<HostContext>` (per-dispatch Store
  isolation).
- The `Store` is created, used for one `resolve()` invocation, and then dropped. No state
  persists between invocations via the Store.
- Multiple resolvers in a single dispatch each get their own independent `Store`.

**Mtime-based cache invalidation:**
- On each dispatch that needs a resolver, the dispatcher checks the current mtime of the
  resolver's `.wasm` file.
- If mtime has changed since the module was cached, the old module is evicted and the
  artifact is recompiled into a new Module before invocation.
- This ensures that a resolver update takes effect on the next dispatch after the file
  changes, without requiring a dispatcher restart.

---

### Resolver ABI Types (BC-4.12.002)

The resolver ABI uses distinct types from the hook ABI. `ResolverInput` and `ResolverOutput`
are NOT the same as `HookPayload` and `HookResult`.

**Exported function signature:**

```
resolve(input_ptr: i32, input_len: i32) -> i64
```

The `i64` return value encodes a packed `(ptr: i32, len: i32)` pair:
`((ptr as i64) << 32) | (len as i64)`.

**`ResolverInput` (dispatcher → resolver):**

```json
{
  "event_type":      "SubagentStop",
  "hook_event_name": "my-hook",
  "agent_type":      "my-agent-type",
  "project_dir":     "/absolute/path/to/project",
  "plugin_config":   { "static_key": "static_value" }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `event_type` | `String` | The host platform's event-type string. For Claude Code dispatch events, common values include `PreToolUse`, `PostToolUse`, `SubagentStop`, `UserPromptSubmit`, `Stop`. The dispatcher passes this through unchanged; resolvers may treat it as an opaque key for branching. Consult the host platform's reference for the canonical list. |
| `hook_event_name` | `String` | The name of the hook being dispatched. |
| `agent_type` | `Option<String>` | The agent type if present in the dispatch context; `null` if absent. |
| `project_dir` | `String` | Absolute path to the factory project root. |
| `plugin_config` | `Value` | The hook's static `plugin_config` from `hooks-registry.toml` (read-only; resolver outputs are not yet merged). |

**`ResolverOutput` (resolver → dispatcher):**

```json
{
  "key":   "my-context",
  "value": { "data": "runtime-value" }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `key` | `String` | Informational self-documentation for the resolver author. **Does NOT determine where the value is merged into `plugin_config`.** |
| `value` | `Option<Value>` | The context payload; `null` means the key is absent (not written to `plugin_config`). |

**Merge key convention (F-P2-002):** The merge key — i.e. the key under which this resolver's
`value` is stored in `plugin_config` — is determined by the **registry-declared `context_key`**
in `resolvers-registry.toml`, NOT by `ResolverOutput.key`. Resolvers MAY include `key` in their
output for self-documentation (e.g., for logging or debugging), but it does not affect merging.
This decouples the resolver's internal naming from the registry schema.

#### Zero-Length Result Convention

A packed return value of `(0, 0)` — i.e. the `i64` result equals `0` — indicates the resolver
intends `Ok(None)`: it has no context to contribute for this dispatch. Resolvers SHOULD return
`ResolverOutput { value: None }` explicitly when a structured return is desired (allows the
dispatcher to log the output key for diagnostics). The `(0, 0)` shortcut is for resolvers that
have no allocation to return and wish to signal absence without writing any bytes. The dispatcher
treats both forms identically: neither writes to `plugin_config`. (F-P2-008)

Resolvers do NOT return block/continue decisions. They return data only. A resolver cannot
block a hook dispatch.

**No inter-resolver dependencies (OD-5):** Resolvers cannot observe or depend on other
resolvers' outputs. Each resolver receives only the static `plugin_config` from
`hooks-registry.toml` — the value that existed before any resolver has run. Resolver outputs
are merged after ALL resolvers have completed their invocations. A resolver that attempts to
design around another resolver's output is a design error. (BC-4.12.002 INV4)

---

### SDK Authoring Surface (BC-4.12.002 PC5, PC8)

Resolver plugins are authored using the `resolver-authoring` feature flag on `vsdd-hook-sdk`.
Enable it in the resolver crate's `Cargo.toml`:

```toml
[dependencies]
vsdd-hook-sdk = { version = "...", features = ["resolver-authoring"] }
```

Use the `#[resolver]` proc-macro to annotate the implementation function:

```rust
use vsdd_hook_sdk::{ResolverInput, ResolverOutput};

#[resolver]
fn resolve_impl(input: ResolverInput) -> ResolverOutput {
    // Read domain-specific data from project_dir using host::read_file,
    // then return the computed context as a ResolverOutput.
    ResolverOutput {
        key: "my-context".to_string(),
        value: Some(serde_json::json!({ "data": "runtime-value" })),
    }
}
```

The `#[resolver]` macro generates the WASM-compatible `resolve()` export that:
1. Reads the input byte slice from WASM memory.
2. Deserializes it from JSON into `ResolverInput`.
3. Calls the user's `resolve_impl` function.
4. Serializes the `ResolverOutput` to JSON.
5. Writes the output to a WASM memory allocation and returns the ptr+len pair.

The user function MUST be named `resolve_impl` and MUST have the exact signature
`fn resolve_impl(input: ResolverInput) -> ResolverOutput`.

Hook crates that are NOT resolvers MUST NOT enable the `resolver-authoring` feature.
The feature flag ensures `ResolverInput`, `ResolverOutput`, and `#[resolver]` are only
available to crates that explicitly opt in.

---

### Capability Model (BC-4.12.003)

Resolvers are **read-only** by design. The dispatcher's host linker for resolver execution
exposes a restricted set of host functions:

| Host function | Available to resolvers | Notes |
|---------------|----------------------|-------|
| `host::read_file` | Yes (capability-gated) | Subject to `path_allow` declarations. |
| `host::log(level, msg_ptr, msg_len)` | Yes, always | Single host function with level argument (Trace=0, Debug=1, Info=2, Warn=3, Error=4). The SDK exposes ergonomic wrappers `log_info(msg)`, `log_warn(msg)`, `log_error(msg)` over this base function for resolver authors. Available at all times for diagnostics. |
| `host::write_file` | **No** — absent from resolver linker | Resolvers are read-only; `host::write_file` is not available to resolvers. |
| `host::exec_subprocess` | **No** — absent from resolver linker | Resolvers cannot execute subprocesses. |
| `host::emit_event` | **No** — absent from resolver linker | Resolvers cannot emit telemetry events directly. |

**Deny-by-default (DI-004):** A resolver without an explicit `path_allow` entry (or with
`path_allow = []`) cannot read any files. Every `host::read_file` call returns
`CapabilityDenied`. This is valid registry configuration for a resolver that computes its
output purely from `ResolverInput` fields without filesystem I/O.

**`CapabilityDenied` return code:** A resolver that attempts to read a path outside its
declared `path_allow` prefixes receives `CapabilityDenied` as a return code from
`host::read_file` — not a WASM trap. The resolver can observe the error in Rust code and
handle it gracefully (e.g., returning `value: None`). (BC-4.12.003 INV3)

**Telemetry on capability denial:** When a resolver receives `CapabilityDenied` from
`host::read_file`, the dispatcher emits a `resolver.capability_denied` telemetry event
(BC-4.12.003 PC2) with three fields: (1) the **resolver name**, (2) the **denied path**
(the path the resolver passed to the host function), and (3) the **resolved path that was
attempted** (the canonicalized path the host computed before failing the prefix check). The
third field is forensically valuable: it lets operators detect path-traversal attempts where
the user-supplied path looks innocent but the resolved path is not. Capability denials are
also surfaced via the `resolver.error` event with `error_kind: "capability_denied"` if the
denial causes the resolver to fail (BC-4.12.004 PC2). The `resolver.capability_denied`
event fires at the host-function boundary; the `resolver.error` event fires if the resolver
subsequently returns an error result. Both may fire for the same denial depending on
dispatcher implementation. Cross-reference: BC-4.12.003 PC2, BC-4.12.004 PC2.

**Path-prefix matching:** `path_allow` entries are matched as path prefixes. A
`path_allow = [".factory/"]` entry allows reading any path that starts with `.factory/`
relative to `project_dir`. Matching is case-sensitive on case-sensitive filesystems.

A resolver module that references `host::write_file` fails at instantiation time (linker
error), not at runtime. This prevents resolvers from accidentally or maliciously writing
files. (BC-4.12.003 INV2)

**Per-resolver isolation:** The capability restrictions for resolver A do NOT apply to
resolver B and vice versa. Each resolver's `Store` is initialized with that resolver's own
`path_allow` list from its registry entry. (BC-4.12.003 PC6)

---

### Error and Crash Isolation (BC-4.12.004)

The WASM sandbox provides hard isolation between a resolver's execution and the dispatcher
process. A resolver panic, WASM trap, execution timeout (fuel/epoch budget exceeded), or
ABI violation (e.g., invalid output JSON) MUST NOT propagate to the dispatcher.

**`resolver.error` telemetry event:**

When a resolver fails, the dispatcher emits a `resolver.error` event with the following
fields:

| Field | Description |
|-------|-------------|
| `resolver_name` | The registry name of the failed resolver. |
| `error_kind` | One of: `"trap"`, `"timeout"`, `"abi_violation"`, `"capability_denied"`, `"not_found"`, `"load_error"`. |
| `error_detail` | Human-readable description of the specific error. |
| `event_type` | The Claude Code envelope event type (e.g., `'PreToolUse'`, `'PostToolUse'`) that triggered this resolver dispatch. |
| `trace_id`     | Dispatcher trace ID for the dispatch event. |
| `session_id`   | Claude Code session identifier. |
| `plugin_name`  | The hook plugin name that declared this resolver in `needs_context`. |

In addition to the telemetry event, the dispatcher writes an error-level log entry at the configured log path with the same fields (BC-4.12.004 PC7).

**Isolation guarantees:**

- A resolver crash does NOT propagate to the dispatcher; `invoke_resolver` (host-side,
  implemented in factory-dispatcher — not exported from hook-sdk) returns a Result in all
  cases. `ResolverError` is a host-side type; it is NOT part of the hook-sdk public API.
  The dispatcher continues executing normally after handling the error.
- The failed resolver's key is NOT written to `plugin_config`. No partial output, no null
  value, no default value — the key is simply absent.
- Dispatch proceeds without the missing context: the hook receives a `plugin_config` that
  lacks the failed resolver's key and must decide how to handle the absent context.
- If multiple resolvers are declared in `needs_context` and one fails, the remaining
  resolvers still execute. A failure in resolver A does not skip resolver B.

---

### Merging Contract (BC-4.12.005)

Context injection uses **additive overlay** merge semantics:

- The final `plugin_config` passed to the hook is the union of:
  - All fields from the static hooks-registry `plugin_config` (preserved as-is).
  - All fields from successful resolver outputs (one field per resolver:
    `plugin_config[key] = value`).
- Resolvers are merged in the order they are declared in `needs_context`.

**Whole-value replacement (no deep merge):** If a resolver returns a value for a key
that already exists in `plugin_config`, the resolver's value **replaces the key wholesale**.
There is no deep merge of nested objects. The static value at that key is gone after the
overlay.

**`value: None` → key absent:** If a resolver returns `ResolverOutput { key: "foo",
value: None }`, the key `"foo"` is NOT written to `plugin_config`. The key remains absent
from the merged `plugin_config`. The hook reading `plugin_config["foo"]` sees a missing
key, not a null value. This is distinct from `value: Some(null)` — a resolver returns
`Option<Value>`, and returning `None` means the resolver has no output for this dispatch.
(BC-4.12.005 PC2)

**`resolver.merge_collision` event:** If a resolver outputs a key that already exists in
the static `plugin_config`, the dispatcher emits a `resolver.merge_collision` telemetry
event with the key name, static value, and resolver value. The resolver's output wins.
This is not an error; it is an expected enrichment pattern.

**`resolver.merge_collision` event fields:**

| Field | Type | Description |
|-------|------|-------------|
| `key` | string | The colliding output key. |
| `static_value` | any (JSON) | The static-config value being overwritten. |
| `resolver_value` | any (JSON) | The resolver-output value overwriting the static. |
| `resolver_name` | string | The registry name of the resolver whose output produced the collision. |
| `plugin_name` | string | The hook plugin name being dispatched. |
| `trace_id` | string | Dispatch trace ID. |
| `session_id` | string | Claude Code session identifier. |

**`needs_context` is the merge scope:** Only resolvers named in the `needs_context` field
of the hooks-registry entry contribute to the merge for that dispatch. Other registered
resolvers (in `resolvers-registry.toml` but not in `needs_context`) are NOT invoked and do
NOT contribute any keys. This zero-overhead path ensures registry entries without
`needs_context` skip resolver invocation entirely. (BC-4.12.005 INV5, BC-1.13.001 PC3)

**Merge example:**

Before merge (static config from hooks-registry entry):
```json
{ "threshold": 3 }
```

After merge (resolver returns `{ "key": "my-context", "value": { "items": ["a", "b"] } }`):
```json
{ "threshold": 3, "my-context": { "items": ["a", "b"] } }
```

If the resolver returns `{ "key": "my-context", "value": null }`, the result is:
```json
{ "threshold": 3 }
```
(key absent — not written to `plugin_config`)

---

### Cross-References

- **ADR-018** — WASM-Plugin Context Resolvers: Design and Layering. Codifies the
  factory-agnostic invariant, separate registry file (OD-2), resolver ABI types (OD-3),
  load-once lifecycle (OD-1), read-only capability model, and explicit registration (OD-6).
- **BC-1.13.001** — Dispatcher loads `resolvers-registry.toml` at startup and injects
  resolver context into `plugin_config` before each hook dispatch. Contains the absent-file
  = zero resolvers constraint (PC1) and `needs_context` semantics.
- **BC-4.12.001** — Resolver lifecycle: load-once with mtime-based cache invalidation;
  per-dispatch Store isolation.
- **BC-4.12.002** — Resolver ABI types (`ResolverInput`, `ResolverOutput`),
  `RESOLVER_ABI_VERSION = 1`, `#[resolver]` macro contract, `resolver-authoring` feature.
- **BC-4.12.003** — Resolver capability model: `path_allow` declarations, deny-by-default,
  `CapabilityDenied` return code, read-only linker (no `write_file`).
- **BC-4.12.004** — Resolver error and crash isolation: `resolver.error` event,
  `resolver_name` and `error_kind` fields, dispatch continues after resolver failure.
- **BC-4.12.005** — Context-injection merging: additive overlay, whole-value replacement,
  `None` → key absent, `resolver.merge_collision` event, duplicate `context_key` = startup
  error.
