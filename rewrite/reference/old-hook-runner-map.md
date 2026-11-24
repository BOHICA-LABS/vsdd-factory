<!-- Architecture map of the pre-rewrite Rust/WASM hook dispatcher and its 79 registered hooks. Captured 2026-11-23 by a read-only survey of branch develop @ 150bc36c. Historical reference for the 1.0 rewrite; not maintained. -->

I mapped the whole hook system without changing anything. A few findings matter for a rewrite (section 0). They come from my reading of the code; I didn't run the dispatcher.

# Hook system architecture map: vsdd-factory

## 0. Findings to know before replacing it

These are what the code actually does, which sometimes differs from the docs and comments.

1. **The dispatcher never writes to stdout.** Claude Code only ever gets an exit code (0 or 2) and stderr. No JSON hook output (`permissionDecision`, `additionalContext`, `systemMessage`) can reach Claude Code.
   - `protect-bc.sh` and `protect-vp.sh` deny through `block_pre_json`, which prints JSON to stdout and exits 0.
   - The adapter (`crates/hook-plugins/legacy-bash-adapter/src/lib.rs`) maps exit 0 to Continue and only logs stdout.
   - So those two denies don't block when run through the dispatcher. They only fall back to exit 2 when `jq` is missing.
2. **Event wiring doesn't match the registry.**
   - The registry has entries for `PreCompact` (2) and `PostCompact` (1), but `hooks.json.*` doesn't wire either event. `precompact-flush`, `check-harness-version` and `postcompact-reanchor` can't fire from Claude Code.
   - `PermissionRequest` is wired but has zero registry entries.
3. **`on_error` is not the "can this block" switch.**
   - Any sync plugin that writes `"outcome":"block"` to stdout blocks, whatever its `on_error`.
   - `on_error="block"` adds two things: exit 2 is also honoured, and a crash or timeout fails closed.
   - A clean plugin exit 1 (`HookResult::Error`, e.g. bash missing) never blocks, even with `on_error=block`.
   - `HOST_ABI.md` says "block and continue identical", which is out of date.
4. **The sink crates are compiled in but never used.** `Router`/`SinkRegistry` are never built in `main.rs` (ADR-005 was superseded by ADR-015 for this reason). Plugin events only go to:
   - `dispatcher-internal-YYYY-MM-DD.jsonl`
   - the file named by `VSDD_SINK_FILE`, if set
5. **The WASM sandbox doesn't confine the filesystem.** Every plugin gets the project root preopened read-write (`invoke.rs` `preopened_dir(... FsPerms::ReadWrite)`). The `read_file`/`write_file`/`read_prefix` path allow-lists only limit those host functions, not native WASI `std::fs`. `HOST_ABI.md` "Filesystem Access Model" says so.
6. **The registry header comment is stale.**

   | | Header says | Actual |
   |---|---|---|
   | Native WASM entries | 21 | 42 (37 unique `.wasm`) |
   | Legacy-bash-adapter entries | 35 | 37 (36 unique scripts; `protect-secrets` is registered twice) |

7. **`hooks.json` timeout is 10000, and `hooks.json` isn't in the checkout.** Claude Code reads the hook `timeout` in seconds, so 10000 is about 2.8 hours, not 10 s. `plugins/vsdd-factory/hooks/hooks.json` is gitignored and absent; it's only created by `/vsdd-factory:activate`.
8. **About 16.5k of the dispatcher's 42.7k source lines are dormant shard rotation.** `shard_manager.rs` only activates if `.factory/shard-config.toml` exists, and it doesn't exist anywhere, including the `factory-artifacts` branch. The `migrate-bc-index` argv subcommand also lives in `main.rs`.

---

## 1. Cargo workspace (`/Users/jmagady/dev2/vsdd-factory/Cargo.toml`)

Edition 2024, rust-version 1.95, `wasmtime`/`wasmtime-wasi` 48.0.6. 55 members. Line counts are `.rs` LOC including tests.

| Crate | LOC | Purpose | Key deps |
|---|---|---|---|
| `crates/factory-dispatcher` | 84,458 (src 42,710, tests 41,748) | The binary: Claude Code hook stdin → registry → wasmtime plugins → exit code | wasmtime, wasmtime-wasi, tokio (current_thread), toml, serde_norway, regex, sha2, uuid, chrono; internal: vsdd-hook-sdk, factory-lock-parse, last-amended-migrate (for `write_atomic`), sink-* ×6; optional `fail` (failpoints) |
| `crates/hook-plugins/*` (40 crates) | 82,428 total | WASM plugins (section 5) | vsdd-hook-sdk, serde_json, regex, serde_norway; some use factory-lock / factory-lock-parse |
| `crates/hook-sdk` (`vsdd-hook-sdk`) | 2,899 | `#[hook]` macro re-export, `HookPayload`/`HookResult`, `host::*` FFI wrappers, resolver ABI. `HOST_ABI.md` (1,275 lines) is the contract | serde, serde_json, vsdd-hook-sdk-macros |
| `crates/hook-sdk-macros` | 313 | proc-macros `#[hook]` and resolver macro | syn, quote, proc-macro2 |
| `crates/hook-sdk/examples/wasm_resolver_export` | (in hook-sdk) | Example resolver export; excluded from the bundle | — |
| `crates/factory-lock` | 1,974 | Pure lib: acquire/renew/clear the `factory_lock` frontmatter in STATE.md (ADR-028) | factory-lock-parse, chrono |
| `crates/factory-lock-parse` | 919 | Pure lib: `parse_factory_lock`, `LockState`, ISO-8601 parse | chrono |
| `crates/last-amended-migrate` | 7,348 | Native CLI: one-time `last_amended`/changelog migration on 5 index/state files (ADR-049). Not a hook | clap, serde_norway |
| `crates/policy15-attestation-gate` | 2,343 | Native CLI used by `ci.yml` (POLICY 15 attestation gate, ADR-040). Not a hook | clap, thiserror |
| `crates/vsdd-context-resolvers` | 1,264 | WASM resolver: reads `.factory/wave-state.yaml` + STATE.md and injects `wave_context` into `plugin_config` | vsdd-hook-sdk, serde_norway |
| `crates/sink-core` | 3,316 | Sink trait and routing-filter primitives | tokio, rand, chrono |
| `crates/sink-file` | 1,183 | JSONL file sink with date rotation | sink-core, tokio |
| `crates/sink-http` | 3,124 | HTTP batch POST sink | reqwest, tokio |
| `crates/sink-datadog` | 1,202 | Datadog Logs sink (wraps sink-http) | sink-http |
| `crates/sink-honeycomb` | 1,395 | Honeycomb sink (wraps sink-http) | sink-http, reqwest |
| `crates/sink-otel-grpc` | 1,483 | OTLP/gRPC log forwarder | opentelemetry-proto 0.31, tonic 0.14 |

All six `sink-*` crates are linked into the dispatcher, but `main.rs` never builds the sink registry (see section 0, item 4).

---

## 2. factory-dispatcher: end-to-end flow

Source: `/Users/jmagady/dev2/vsdd-factory/crates/factory-dispatcher/src/`.

**Entry: `main.rs` `main()` → `run()`**
1. If `argv[1] == "migrate-bc-index"`, it runs `shard_manager::run_migrate_bc_index_cli` and exits. No hook handling.
2. `InternalLog::new(resolve_log_dir())`, then `prune_old(30 days)`.
   - Log dir is resolved by `log_dir.rs`, levels A–G, first match wins:
     - A: `VSDD_LOG_DIR`
     - B: `FACTORY_ROOT/logs`
     - C–E: an enclosing or child `.factory/logs`
     - F: `git worktree list` (200 ms timeout)
     - G: `./.factory/logs`
   - A "mount gate" avoids creating `.factory/` before the worktree is mounted.
3. `HookPayload::from_reader(stdin)` (`payload.rs`). It accepts `hook_event_name` as an alias of `event_name`, and keeps `tool_name`, `session_id`, `tool_input`, `tool_response` and the rest in `extra`.
4. Registry load (ADR-024, two-tier):
   - If `CLAUDE_PLUGIN_ROOT` is empty, it logs `internal.dispatcher_error`, uses an empty registry and exits 0.
   - Otherwise it calls `Registry::load(${CLAUDE_PLUGIN_ROOT}/hooks-registry.toml)` (`registry.rs`). Validation:
     - `schema_version == 2`; mismatch is E-REG-001 → **exit 2**
     - each tool regex must compile
     - `async=true` combined with `on_error ∈ {block, block_if_marker}` is E-REG-002 → **exit 2**
     - a duplicate `(name, event, tool)` is E-REG-003 → **exit 2**
     - other load errors (missing file, bad TOML) → exit 0 (fail-open)
   - Fields (serde `deny_unknown_fields`): `name`, `event`, `tool` (regex), `plugin`, `priority` (default 500), `enabled`, `timeout_ms` (5000), `fuel_cap` (20M, `DEFAULT_FUEL_CAP`), `on_error` (`continue` / `block` / `block_if_marker`), `capabilities`, `config` (TOML table → `plugin_config`), `async`, `needs_context`, `failure_policy` (`fail-open` / `fail-closed`). An optional `[defaults]` table is supported.
5. `routing::match_plugins`: keeps entries that are enabled, whose `event == payload.event_name` exactly, and whose `tool` regex matches `tool_name`. The match is a regex **search**, not a fullmatch; the registry header explains the anchoring rule.
6. `partition::partition_plugins` splits into a sync group and an async group. `group_by_priority` puts the sync group into tiers by ascending priority, keeping registry order inside a tier.
7. Stderr line: `factory-dispatcher trace=<uuid> event=… tool=… host_abi=1 sync_plugins=N async_plugins=M`.
8. Native, non-WASM pre-checks run every time, before the "nothing matched" early return:
   - `invoke::reconcile_replace_all_overcap_if_qualifying` (shard janitor)
   - `executor::bc_index_migration_admission_precheck` / `bc_index_migration_reservation_release`
   - `shard_cap_precheck`, which is a no-op unless `.factory/shard-config.toml` exists

   If nothing matched and no shard verdict fired, it returns 0.
9. Engine setup (`engine.rs`):
   - `build_engine()` turns on epoch interruption and fuel, with no component model.
   - `EpochTicker` is a thread that increments the epoch every `EPOCH_TICK_MS = 10` ms. A plugin's deadline is `timeout_ms / 10` epochs.
   - `PluginCache` (`plugin_loader.rs`) compiles each `.wasm` once per process.
10. `HostContext` (`host/mod.rs`):
    - `cwd` = canonicalized `CLAUDE_PROJECT_DIR`
    - `plugin_root`
    - `env_view` = the whole process environment
    - a shared event queue
11. Payload shaping:
    - adds `dispatcher_trace_id`
    - `invoke::inject_git_context_if_qualifying` adds `git_context` (7 string fields) on PostToolUse Bash `git commit` events aimed at the factory-artifacts worktree (ADR-029)
12. Resolvers (`resolver_loader.rs`, `resolver.rs`):
    - `ResolverLoader::load_registry(${plugin_root}/resolvers-registry.toml)`
    - A missing file means an empty set. A parse or compile error is fatal.
    - Only one resolver exists: `wave_context` → `hook-plugins/vsdd-context-resolvers.wasm`, `path_allow = [.factory/wave-state.yaml, .factory/STATE.md]`.
    - For entries with `needs_context`, `executor::build_plugin_config` runs the resolver and merges its output into `plugin_config`.
13. Sync execution: `executor::execute_tiers` → `execute_tier`.
    - Tiers run one after another. Plugins inside a tier run in parallel (`spawn_blocking` each).
    - All tiers always run; a block doesn't stop the later tiers.
    - For each plugin, `invoke::invoke_plugin`:
      - fresh `Store`, `set_fuel(fuel_cap)`, `set_epoch_deadline`
      - WASI p1 with the payload JSON on stdin, stdout/stderr captured in 64 KiB pipes (stderr truncated to 4 KiB)
      - preopen of the project dir as `.` (read-write)
      - `vsdd::*` host imports, then calls `_start`
    - Results:
      - `I32Exit(n)` → `Ok{exit_code}`
      - `Trap::Interrupt` → `Timeout{Epoch}`
      - `Trap::OutOfFuel` → `Timeout{Fuel}`
      - any other trap → `Crashed`
    - `classify_outcome` / `indeterminate_marker.rs` (ADR-047/048):
      - a PostToolUse INDETERMINATE result from a `fail-closed` plugin writes `.factory/unvalidated-mutation.marker` (24 h TTL)
      - a PASS from the named plugin in PostToolUse clears it
      - `block_if_marker` entries run TTL expiry and raw-delete reconciliation first
14. Block aggregation (`execute_tiers`, `aggregator::aggregate_exit_code`). The result is exit 2 if **any** sync plugin hits one of these:
    - `plugin_requests_block`: stdout contains `"outcome":"block"`, under any `on_error`
    - `plugin_fail_closed`: crashed or timed out with `on_error=block`
    - `plugin_block_if_marker`: crashed or timed out with `block_if_marker` while a live marker exists
    - `aggregate_exit_code`: exit code 2 with `on_error=block`
    - a fired native shard or migration gate

    PostCompact is forced to exit 0 (`EventType::is_advisory_only`).
15. Async group:
    - each plugin is `tokio::spawn`'d through `spawn_async_plugin`
    - results are collected through an mpsc channel inside `ASYNC_DRAIN_WINDOW_MS = 100` ms (`lib.rs`, DI-019)
    - unfinished ones get `plugin.abandoned`
    - any block verdict is discarded and logged as `plugin.async_block_discarded`
    - async plugins never change the exit code and their stderr isn't relayed
16. Output:
    - each sync plugin's captured stderr is copied to the dispatcher's stderr
    - then a summary line: `  plugins_run=N total_ms=T block_intent=<bool> exit_code=<0|2>`, plus `blocking_plugins=a,b block_reason="…"` when it blocks (`extract_block_info`; the reason is the plugin's `reason` field)
    - if `VSDD_SINK_FILE` is set, `vsdd_sink::flush_sink_file` writes every non-`internal.*` queued event as JSONL (rejects `..`)
    - `std::process::exit(code)`
    - most internal errors exit 0 (fail-open)

**Host ABI** (`invoke.rs` `setup_host_on_store_data`; mirrored in `host/*.rs`; `HOST_ABI_VERSION = 1`). Module `vsdd`:

| Function | Notes |
|---|---|
| `log` | |
| `emit_event` | reserved identity fields are dropped; goes to the internal log + event queue via `HostContext::emit_internal` |
| `session_id`, `dispatcher_trace_id`, `plugin_root`, `plugin_version`, `cwd` | context getters |
| `env` | gated by `capabilities.env_allow` |
| `read_file` | gated by `capabilities.read_file.path_allow` |
| `read_prefix` | separate `capabilities.read_prefix.path_allow`; never returns -3 |
| `write_file` | `path_allow`, `max_bytes_per_call` |
| `exec_subprocess` | `binary_allow`, `env_allow`, `cwd_allow`; `shell_bypass_acknowledged` is required for `bash`; result is `[exit i32 | stdout_len | stdout | stderr_len | stderr]` |

Error codes: -1 denied, -2 timeout, -3 output too large, -4 invalid, -5 not found, -99 internal. Denials emit `internal.capability_denied`.

**SDK plugin contract** (`crates/hook-sdk/src/__internal.rs`, `result.rs`):
- `#[hook] fn(HookPayload) -> HookResult`
- stdout is one JSON line, `{"outcome":"continue"|"block","reason"|"error","message"}`
- exit codes: 0 continue, 2 block, 1 error
- `block_with_fix` formats "BLOCKED by X: … Fix: … Code: …"
- several plugins use the "advisory block" pattern: return Continue but print an `outcome:block` line

**Legacy bash adapter** (`crates/hook-plugins/legacy-bash-adapter/src/lib.rs`, ADR-012):
1. Reads `plugin_config.script_path` and resolves it under `plugin_root`.
2. Strips `plugin_config` and runs `exec_subprocess("bash", [script])` with the original envelope on stdin.
3. Maps the result: exit 0 → Continue; exit 2 → `Block{reason = trimmed stderr ≤ 4 KiB}`; anything else → `Error`. The script's stdout is only logged.

**Internal logs** (`internal_log.rs`):
- `<log_dir>/dispatcher-internal-YYYY-MM-DD.jsonl`, 30-day retention; dead-letter files `dead-letter-*.jsonl` (7 days)
- Event types: `dispatcher.started`, `plugin.invoked/completed/timeout/crashed/indeterminate/abandoned`, `marker.written/cleared`, `resolver.*`, `internal.*`
- `internal.dispatcher_error` is de-duplicated
- Bash hooks log separately through `plugins/vsdd-factory/bin/emit-event`, which writes `.factory/logs/events-YYYY-MM-DD.jsonl`

**Env vars:**

| Variable | Status |
|---|---|
| `CLAUDE_PLUGIN_ROOT`, `CLAUDE_PROJECT_DIR`, `VSDD_LOG_DIR`, `FACTORY_ROOT`, `VSDD_SINK_FILE` | honoured in release builds |
| `VSDD_ASYNC_DRAIN_WINDOW_MS`, `VSDD_FORCE_ENGINE_BUILD_FAILURE` | debug or `test-support` feature only |

---

## 3. `plugins/vsdd-factory/hooks-registry.toml` (1,708 lines, `schema_version = 2`): 79 entries

All entries have `fuel_cap` unset (default 20M) and `enabled` unset (true).

Common capability sets used below:
- **L** (most bash-adapter entries): `env_allow` + `exec_subprocess` with `binary_allow=[bash, jq]` and `shell_bypass_acknowledged`, `env_allow=[PATH, HOME, TMPDIR, CLAUDE_PROJECT_DIR, CLAUDE_PLUGIN_ROOT, VSDD_SESSION_ID]`.
- **G**: `exec_subprocess` with `binary_allow=[git]`, `env_allow=[HOME, GIT_CONFIG_GLOBAL, XDG_CONFIG_HOME]`.
- **EWM** = `^(Edit|Write|MultiEdit)$`.

In the tables, "prio" is priority; "def" means the field is unset and defaults apply (priority 500, `on_error=continue`).

**SessionStart / SessionEnd / Worktree / PostToolUseFailure** (all WASM, async, default `on_error`):

| Name | Event | Timeout | Capabilities |
|---|---|---|---|
| `session-start-telemetry` | SessionStart | 8000 | `read_file[.claude/settings.local.json]`, `exec[factory-health]` |
| `session-end-telemetry` | SessionEnd | 5000 | — |
| `worktree-hooks` | WorktreeCreate | 5000 | — |
| `worktree-hooks` | WorktreeRemove | 5000 | — |
| `tool-failure-hooks` | PostToolUseFailure | 5000 | — |

**PreToolUse (22 entries)**

| prio | Name | Tool | Target | on_error | Timeout | Notes / capabilities |
|---|---|---|---|---|---|---|
| 20 | `block-ai-attribution` | `^Bash$` | wasm | block | 5000 | `env_allow=[]` |
| 30 | `brownfield-discipline` | EWM | bash | block | 5000 | L |
| 40 | `check-factory-commit` | `^Bash$` | bash | block | 5000 | L + git |
| 50 | `destructive-command-guard` | `^Bash$` | bash | block | 5000 | L + git, gh, curl |
| 60 | `factory-branch-guard` | EWM | bash | block | 5000 | L + git |
| 70 | `protect-bc` | EWM | bash | block | 5000 | L |
| 80 | `protect-secrets` | `^Bash$` | bash | block | 5000 | L |
| 80 | `protect-secrets` | `^Read$` | bash | block | 5000 | L |
| 90 | `protect-vp` | EWM | bash | block | 5000 | L |
| 100 | `red-gate` | EWM | bash | block | 5000 | L + git |
| 110 | `track-agent-start` | `^Agent$` | wasm | continue, **async** | 5000 | — |
| 120 | `validate-pr-merge-prerequisites` | `^Agent$` | bash | block | 10000 | L + gh; `failure_policy=fail-closed` |
| 130 | `validate-wave-gate-prerequisite` | `^Agent$` | bash | block | 10000 | L; fail-closed |
| 140 | `verify-git-push` | `^Bash$` | bash | block | 10000 | L + git, gh |
| 140 | `validate-factory-path-staging` | `^Bash$` | wasm | continue | 5000 | G; fail-closed |
| 142 | `verify-factory-lock` | `^(Edit|Write|MultiEdit|Agent)$` | wasm | continue | 5000 | `read_prefix[.factory/STATE.md]` + G |
| 142 | `verify-factory-lock-bash` | `^Bash$` | wasm (same) | continue | 5000 | same |
| 150 | `validate-artifact-path` | EWM | wasm | continue | 5000 | `read_file[plugins/vsdd-factory/config/artifact-path-registry.yaml]` |
| 155 | `validate-stable-anchors` | EWM | wasm | block | 5000 | `read_file[.factory/specs]` |
| def | `validate-heavy-op-delegation` | `^Bash$` | wasm | continue | 5000 | `config.patterns` = cargo test --release, grep -r/-R, find . -name/-type, ./run-all.sh, ./run-bats.sh |
| def | `validate-unvalidated-mutation-marker` | `^Agent$` | wasm | block_if_marker | default | fail-open |
| def | `validate-unvalidated-mutation-marker-git` | `^Bash$` | wasm | block_if_marker | default | fail-open |

**PostToolUse (41 entries)**

- **No tool filter (fire on every tool):**
  - `capture-commit-activity` (110, async, continue, `env_allow=[]`, exec[git])
  - `capture-pr-activity` (120, async, continue, exec[gh])
  - `regression-gate` (230, continue; `read_file` + `write_file` [.factory/regression-state.json])
- **Bash adapter, EWM, L caps, async=false unless noted** (prio: name [on_error / timeout]):
  - 210 `convergence-tracker` [continue / 10000, **async**]
  - 220 `purity-check` [continue / 5000]
  - 240 `validate-anchor-capabilities-union` [continue / 10000]
  - 250 `validate-bc-title` [continue / 5000]
  - 260 `validate-changelog-monotonicity` [continue / 5000]
  - 270 `validate-demo-evidence-story-scoped` [continue / 5000]
  - 280 `validate-factory-path-root` [**block** / 5000]
  - 290 `validate-finding-format` [continue / 5000]
  - 300 `validate-index-self-reference` [continue / 5000]
  - 310 `validate-input-hash` [**block** / 10000]
  - 320 `validate-novelty-assessment` [continue / 5000]
  - 330 `validate-pr-description-completeness` [continue / 5000]
  - 340 `validate-state-index-status-coherence` [continue / 10000]
  - 350 `validate-state-pin-freshness` [continue / 10000]
  - 360 `validate-state-size` [continue / 5000, + git]
  - 370 `validate-story-bc-sync` [continue / 5000]
  - 380 `validate-subsystem-names` [continue / 5000]
  - 390 `validate-table-cell-count` [continue / 5000]
  - 400 `validate-template-compliance` [**block** / 10000]
  - 410 `validate-vp-consistency` [continue / 10000]
  - 420 `validate-wave-gate-completeness` [continue / 10000]
  - 430 `validate-count-propagation` [continue / 5000]
  - 440 `validate-red-ratio` [continue / 5000]
- **WASM, EWM:**
  - 450 `validate-wave-handoff-completeness` (`read_file[.factory/HANDOFF.md]`)
  - 460 `validate-cross-site-correspondence` (8000 ms; `read_file[BC/VP/stories dirs]`)
  - 470 `stamp-state-timestamp` (`read_file` + `write_file[.factory/STATE.md]`, G)
  - 151 `validate-index-cite-refresh` (`read_file` of BC-INDEX, VP-INDEX, STORY-INDEX, ARCH-INDEX, STATE.md, `.factory/cycles/v1.0-brownfield-backfill/INDEX.md`)
  - 152 `validate-burst-log` (EWM **and** `^Bash$`; `read_file[.factory/cycles, .factory/hooks]`)
  - 153 `validate-state-structure` (`read_file[.factory]`)
  - 154 `validate-dispatch-advance` (EWM **and** `^Bash$`; `read_file[.factory]`)
  - 156 `validate-closes-completeness` (`read_file[.factory]`)
  - 157 `validate-policies-schema` (`read_file[.factory, plugins/vsdd-factory]`)
  - 158 `validate-trajectory-tail-cell-completeness` (`read_file[.factory]`)
  - 160 `lint-registry-async-invariant` (**block**; `read_file[plugins/vsdd-factory/hooks-registry.toml]`)
- **WASM, `^Bash$`:**
  - 161 `validate-factory-path-staged` (G; fail-closed)
  - 159 `verify-state-timestamp-advisory` (no caps)

All of these are `continue` unless marked **block**.

**Stop:**
- `session-learning` (910, async, continue)
- `warn-pending-wave-gate` (920, `read_file[.factory/wave-state.yaml]`)

**SubagentStop (all WASM):**
- 910 `handoff-validator`
- 920 `pr-manager-completion-guard`
- 930 `track-agent-stop` (async)
- 940 `update-wave-state-on-merge` (async, 10000 ms, read + write `wave-state.yaml`, `max_bytes_per_call` 65536)
- 950 `validate-pr-review-posted`
- 960 `validate-per-story-adversary-convergence` (10000 ms, `needs_context=["wave_context"]`, `read_file[.factory/cycles]`)

**PreCompact / PostCompact** (not wired in `hooks.json`):
- 50 `check-harness-version` (bash; caps: bash, `CLAUDE_CODE_VERSION`)
- 100 `precompact-flush` (wasm, 30000 ms, read + write `.factory/`, exec[git] with `SSH_AUTH_SOCK`)
- 100 `postcompact-reanchor` (bash, PostCompact, + `write_file[.factory/logs/]`)

**Counts**

| Breakdown | Values |
|---|---|
| By event | PostToolUse 41 (23 bash / 18 wasm); PreToolUse 22 (12 / 10); SubagentStop 6; Stop 2; PreCompact 2; PostCompact 1; SessionStart, SessionEnd, WorktreeCreate, WorktreeRemove, PostToolUseFailure 1 each |
| Kind | 37 bash-adapter / 42 native WASM |
| `on_error` | continue 54 + default 5; block 18 (15 bash, 3 wasm); block_if_marker 2 |
| `async` | 12 true (11 wasm + bash `convergence-tracker`); 67 sync |
| `failure_policy` | fail-closed 4; fail-open explicit 2 |

**Block-capable vs advisory.** This is by what the code emits, not by `on_error`:
- **Can block, sync WASM (26 entries):** plugins whose source has `HookResult::block` or an `"outcome":"block"` emit.
- **Advisory sync WASM (5):** `stamp-state-timestamp`, `warn-pending-wave-gate`, `validate-heavy-op-delegation`, `verify-state-timestamp-advisory`, `regression-gate`. None has a block site.
- **Async (12):** always advisory.
- **Bash (36 sync entries):** a script blocks only by exiting 2. These never do:
  - `check-factory-commit.sh` ("Exit 0 always")
  - `purity-check.sh` (warn-only)
  - `protect-bc` / `protect-vp`, which use JSON deny with exit 0 (section 0, item 1)

---

## 4. `plugins/vsdd-factory/hooks/` (62 files)

**How Claude Code calls the dispatcher**
- `hooks.json.template` uses `{{PLATFORM}}` and `{{EXE_SUFFIX}}`. `scripts/generate-hooks-json.sh` renders `hooks.json.{darwin-arm64, darwin-x64, linux-arm64, linux-x64, windows-x64}`. CI's `--check` mode fails on drift.
- Each variant registers one command per event, `${CLAUDE_PLUGIN_ROOT}/hooks/dispatcher/bin/<platform>/factory-dispatcher[.exe]` with `timeout 10000`, for:
  - PreToolUse, PostToolUse, PermissionRequest, Stop, SubagentStop
  - SessionStart and SessionEnd (both `once: true`)
  - WorktreeCreate, WorktreeRemove, PostToolUseFailure
- No matchers are used; all routing happens in the dispatcher.
- `/vsdd-factory:activate` picks the platform (ADR-009). `skills/activate/apply-platform.sh` (and `.ps1`) copies `hooks.json.<platform>` to `hooks/hooks.json`, which is gitignored and currently absent, and checks the binary is executable.
- Old pre-template format: `scripts/legacy/hooks-json-pre-templating.json`.

**`dispatcher/bin/<platform>/`:** 5 prebuilt binaries, git-tracked, 13.8–19.4 MB each (about 82 MB total).

**Live bash hooks, registered via the legacy adapter (36 scripts):**

*Guards and protection*

| Script | What it does |
|---|---|
| `brownfield-discipline.sh` | Makes `.reference/**` read-only |
| `check-factory-commit.sh` | Advisory STATE.md reminder after `.factory/` commits. The header says PostToolUse, but it's registered PreToolUse |
| `destructive-command-guard.sh` | Blocks destructive rm/git/etc. against `.factory/`, `src/`, `tests/` (435 lines) |
| `factory-branch-guard.sh` | `.factory/` writes only when `.factory` is a worktree on the `factory-artifacts` branch |
| `protect-bc.sh` | Blocks edits to BC files with Status: green (JSON deny) |
| `protect-vp.sh` | Blocks edits to VP files with Status: green (JSON deny) |
| `protect-secrets.sh` | Blocks Bash/Read that would leak `.env` files or secret env vars |
| `red-gate.sh` | Strict TDD: blocks source edits not declared "currently red" |
| `verify-git-push.sh` | Blocks force push and protected-branch pushes |
| `validate-pr-merge-prerequisites.sh` | Before a github-ops merge dispatch, requires the evidence trail in `.factory/code-delivery/STORY-NNN/` |
| `validate-wave-gate-prerequisite.sh` | Blocks Wave N+1 dispatch while the Wave N gate is pending |

*Artifact validators (PostToolUse)*

| Script | What it does |
|---|---|
| `convergence-tracker.sh` | Novelty Assessment convergence rules (async, so it can't block) |
| `purity-check.sh` | Warns on side effects in `*/pure/**`, `*/core/**`, `*_pure.rs` |
| `validate-anchor-capabilities-union.sh` | Story `anchor_capabilities` = union of its BCs' `capability` |
| `validate-bc-title.sh` | BC H1 matches BC-INDEX |
| `validate-changelog-monotonicity.sh` | Changelog rows ordered newest-first |
| `validate-count-propagation.sh` | Cross-document count drift across the INDEX/STATE/PRD/SS files |
| `validate-demo-evidence-story-scoped.sh` | Evidence must live under `docs/demo-evidence/<STORY>/` |
| `validate-factory-path-root.sh` | `.factory/` written inside a worktree instead of the project root |
| `validate-finding-format.sh` | ADV finding/fix ID format |
| `validate-index-self-reference.sh` | INDEX/burst-log references the current pass from STATE `current_step` |
| `validate-input-hash.sh` | `input-hash` drift / placeholder |
| `validate-novelty-assessment.sh` | Required Novelty Assessment fields |
| `validate-pr-description-completeness.sh` | PR template sections present |
| `validate-red-ratio.sh` | RED_RATIO ≥ 0.5 |
| `validate-state-index-status-coherence.sh` | STATE `convergence_status` vs INDEX status |
| `validate-state-pin-freshness.sh` | STATE version pins vs actual artifact versions |
| `validate-state-size.sh` | STATE.md warns >200 lines, blocks >500 |
| `validate-story-bc-sync.sh` | Story BC frontmatter ↔ body table |
| `validate-subsystem-names.sh` | SS-NN names match ARCH-INDEX |
| `validate-table-cell-count.sh` | Markdown table pipe counts |
| `validate-template-compliance.sh` | Required frontmatter/sections from the matching template |
| `validate-vp-consistency.sh` | VP-INDEX ↔ verification-architecture ↔ coverage matrix |
| `validate-wave-gate-completeness.sh` | `gate_status: passed` requires a complete gate report |

*Compact events (unwired)*

| Script | What it does |
|---|---|
| `check-harness-version.sh` | PreCompact: Claude Code ≥ 2.1.105 |
| `postcompact-reanchor.sh` | PostCompact re-anchor advisory |

**Not hooks:**
- `precompact-flush-prune.sh` — prunes `precompact-flush-log` when it passes 1000 entries; called by tooling
- `update-cargo-audit-cache.sh` — writes `.factory/hooks/cargo-audit-cache.json` for `validate-policies-schema` (ADR-021)
- `lib/block.sh` — `block_pre` (stderr + exit 2) and `block_pre_json` (stdout JSON + exit 0) helpers
- `dim2-gates/*.sh` (11 scripts + README) — manual tools for "Dim-2" burst-log attestation:
  - `active-branches-sha-currency`, `banner-wc-l`, `block-label-canonical-form`, `decision-log-monotonic-rows`
  - `dim1-file-count-arithmetic`, `dim7-dispatched-count-sweep`, `freshness-literal-stdout`
  - `layer-ordinal-dual-direction`, `meta-level-ack-grep`, `propagation-completeness`, `trajectory-tail-cell-grep`

  The README says they are not registered. They exist purely for this repo's engine discipline.

**Bash hooks replaced by WASM ports** (deleted in git history):
- `block-ai-attribution`, `capture-commit-activity`, `capture-pr-activity`
- `handoff-validator`, `pr-manager-completion-guard`, `regression-gate`
- `session-learning`, `track-agent-start`, `track-agent-stop`
- `update-wave-state-on-merge`, `validate-pr-review-posted`, `warn-pending-wave-gate`

No current `.sh` file has a matching WASM crate.

---

## 5. WASM plugins

There are 40 crates in `crates/hook-plugins/` and 39 tracked `.wasm` files in `plugins/vsdd-factory/hook-plugins/` (37 registered hooks + `legacy-bash-adapter` + `vsdd-context-resolvers`; about 14.4 MB).

Not bundled:
- `read-prefix-fixture` (a link test, 30 LOC)
- `verify-state-timestamp-refresh` (5,102 LOC; deregistered, replaced by `stamp-state-timestamp` per ADR-046)

Flags: **[ED]** = tied to this repo's own self-referential engine-discipline bookkeeping (hardcoded `plugins/vsdd-factory/...` paths, the `v1.0-brownfield-backfill` cycle, D-NNN codification rules, the burst-log/trajectory format). **[VSDD]** = depends on the VSDD `.factory/` artifact model, so useful to VSDD consumer projects. **[GEN]** = generally useful anywhere.

**A. `.factory` write protection and state integrity**

| Plugin | Flag | Rule |
|---|---|---|
| `validate-artifact-path` | ED/VSDD | Blocks `.factory/` writes to paths not listed in `plugins/vsdd-factory/config/artifact-path-registry.yaml`. That path is project-relative, so it only exists in this repo |
| `validate-factory-path-staging` | VSDD | PreToolUse Bash: blocks `git add` of `.factory/` paths on product branches (uses git) |
| `validate-factory-path-staged` | VSDD | PostToolUse detective version of the above |
| `verify-factory-lock` | VSDD | Reads the STATE.md `factory_lock` prefix; blocks mutating tools while another session holds an unexpired lock |
| `stamp-state-timestamp` | VSDD | PostToolUse: rewrites the STATE.md `timestamp:` and renews `factory_lock.expires_at`. Has side effects; never blocks |
| `verify-state-timestamp-advisory` | VSDD | Advisory when a factory-artifacts commit doesn't advance STATE `timestamp:` |
| `validate-unvalidated-mutation-marker` | VSDD | Blocks Agent dispatches and git commit/push while `.factory/unvalidated-mutation.marker` exists |
| `precompact-flush` | VSDD | Commits STATE.md to `factory-artifacts` before compaction and renews the lock (unwired) |

**B. Spec / BC / VP / story consistency**

| Plugin | Flag | Rule |
|---|---|---|
| `validate-cross-site-correspondence` | VSDD/ED | BC version citations, story input-hash three-way equality, frontmatter version/`last_amended` parity, `modified[]` monotonicity. Largest plugin, 11,651 LOC |
| `validate-stable-anchors` | VSDD | Blocks spec text citing volatile `file.ext:NNN` line numbers |
| `validate-index-cite-refresh` | ED | Stale 4-index version citations in ARCH-INDEX/STATE/INDEX; hardcodes the cycle `v1.0-brownfield-backfill` |

**C. Engine-discipline bookkeeping** (all ED)

| Plugin | Rule |
|---|---|
| `validate-burst-log` | burst-log h2 format, 9-block completeness, Dim-1 parity |
| `validate-closes-completeness` | `lessons.md` `**Closes:**` rules |
| `validate-dispatch-advance` | STATE `current_step` meta-commentary, 4-index cites, trajectory-tail cardinality, D-chain cites, INDEX review-row schema |
| `validate-state-structure` | STATE banner line-count / dual-margin / trajectory-tail |
| `validate-trajectory-tail-cell-completeness` | `trajectory_tail` present at every prescribed site |
| `validate-policies-schema` | `.factory/policies.yaml` structure plus the cargo-audit cache check; reads `plugins/vsdd-factory` |
| `lint-registry-async-invariant` | Blocks edits to this repo's `plugins/vsdd-factory/hooks-registry.toml` that break schema v2 or the block⇒sync rule |
| `validate-heavy-op-delegation` | Advisory stderr when a Bash command matches the heavy-operation patterns; never blocks |

**D. Wave / convergence / adversary gates**

| Plugin | Flag | Rule |
|---|---|---|
| `validate-per-story-adversary-convergence` | VSDD | SubagentStop: blocks wave-gate dispatch unless each story has `passes_clean ≥ 3` and NITPICK_ONLY; uses the `wave_context` resolver |
| `warn-pending-wave-gate` | VSDD | Stop: warns if any wave has `gate_status: pending` |
| `update-wave-state-on-merge` | VSDD | SubagentStop: updates `wave-state.yaml` after a pr-manager merge |
| `validate-wave-handoff-completeness` | VSDD | Blocks an incomplete `.factory/HANDOFF.md` (ADR-026 fields) |
| `regression-gate` | VSDD/GEN | Records test pass/fail in `.factory/regression-state.json` and warns on pass→fail |

**E. Git / commit / PR / subagent guards**

| Plugin | Flag | Rule |
|---|---|---|
| `block-ai-attribution` | GEN | Blocks git commits containing AI-attribution trailers |
| `pr-manager-completion-guard` | VSDD | Advisory-blocks a pr-manager SubagentStop before all 9 lifecycle steps are done |
| `validate-pr-review-posted` | VSDD | pr-reviewer must write `pr-review.md` and post a `gh pr review` |
| `handoff-validator` | GEN-ish | Advisory-blocks empty or truncated subagent output |

**F. Session lifecycle and telemetry** (emit events only; never block)

| Plugin | Flag | Event emitted / effect |
|---|---|---|
| `session-start-telemetry` | GEN | `session.started` (runs `factory-health`) |
| `session-end-telemetry` | GEN | `session.ended` |
| `session-learning` | VSDD | Appends a marker to `.factory/sidecar-learning.md` |
| `worktree-hooks` | GEN | `worktree.created` / `worktree.removed` |
| `tool-failure-hooks` | GEN | `tool.error` |
| `track-agent-start` / `track-agent-stop` | GEN | `agent.start` / `agent.stop` |
| `capture-commit-activity` | GEN | `commit.made` |
| `capture-pr-activity` | GEN | `pr.created` / `merged` / `closed` via `gh` |

**Infrastructure:**
- `legacy-bash-adapter` (538 LOC) — the bash bridge
- `vsdd-context-resolvers` — the `wave_context` resolver
- `read-prefix-fixture` — test fixture only

---

## 6. How binaries are built and shipped

**`.github/workflows/release.yml`** (868 lines; runs on tag `v*`):
- `validate`: changelog and version checks.
- `build-binaries` matrix: aarch64/x86_64-apple-darwin, x86_64/aarch64-unknown-linux-gnu (aarch64 through cross-rs), x86_64-pc-windows-msvc.
  - `cargo build --release -p factory-dispatcher`, **without** the `test-support` feature.
  - wasm32-wasip1 release build of the workspace, excluding the dispatcher, sinks, `policy15-attestation-gate`, `last-amended-migrate` and `verify-state-timestamp-refresh`. `legacy-bash-adapter` is built with `--bin`; `update-wave-state-on-merge` with `--no-default-features`.
  - Skips underscore-named lib stubs and `read-prefix-fixture`.
  - Asserts `vsdd-context-resolvers.wasm` exists in `hook-plugins/` (it's committed, not rebuilt) and that every registry-declared `.wasm` is present.
- `commit-binaries`:
  - stages the binaries into `hooks/dispatcher/bin/<platform>/` and the `.wasm` files (from the linux-x64 artifact) into `plugins/vsdd-factory/hook-plugins/`
  - runs `scripts/generate-hooks-json.sh` and bumps `plugin.json` from the tag
  - `git add -f`, commits "chore: bundle dispatcher binaries for $TAG" on `main` and re-points the tag

**Committed artifacts:** the 5 dispatcher binaries and 39 `.wasm` files are git-tracked. `.gitignore` lists `plugins/vsdd-factory/hook-plugins/`; the files got in by force-add. `hooks/hooks.json` is gitignored.

**`ci.yml`** (1,091 lines):
- `validate` job: shell syntax, `hooks.json` variant drift, bats hook/bin tests
- `cargo-host`: fmt, clippy `-D warnings`, `cargo test --workspace`, wasm builds, staging into `hook-plugins/`, release dispatcher for end-to-end bats
- `build-dispatcher` platform matrix (`cargo test --release --features factory-dispatcher/test-support`), plus `platforms-drift` and `policy-15-attestation-location`

**Also:** `kani.yml` (Kani proofs, VP-077), `semgrep.yml`, `release-branch-guardrail.yml`.

---

## 7. Tests tied to hooks

**`plugins/vsdd-factory/tests/`:** 646 files — 258 `.bats` (65,726 lines, about 2,279 `@test`), 326 `.md` fixtures, about 380 files under `fixtures/`. Runner: `tests/run-all.sh`.

I classified files with grep heuristics, so these counts are approximate:

| Group | Files | Tests | What they test | Coupling |
|---|---|---|---|---|
| Dispatcher/WASM-driven | 173 | ~670 | Mostly per-plugin directories (`validate-closes-completeness/` 28, `validate-trajectory-tail-cell-completeness/` 32, `validate-state-structure/` 25, `validate-policies-schema/` 24, `validate-dispatch-advance/` 19, `dim2-gates/` 12, …) plus top-level suites (`precompact-*`, `resolver-*`, `td-71-stderr-block-reason`, `s21-12-*`, `vp093-git-context-injection`, `stamp-state-timestamp`, `validate-unvalidated-mutation-marker`, …) | Tightly coupled. Each test writes a temporary `hooks-registry.toml` pointing at `hook-plugins/<x>.wasm`, pipes an envelope into `target/release/factory-dispatcher` with `CLAUDE_PLUGIN_ROOT`/`CLAUDE_PROJECT_DIR`, and checks exit code, stderr, the summary line and sometimes `VSDD_SINK_FILE` JSONL. 161 files `skip` when the binary or wasm isn't built. Helpers: `helpers/dispatcher-provenance.bash`, `helpers/registry.bash` (`registry_has_hook name [event] [tool]`) |
| Direct bash-hook | 45 | ~842 | `hooks.bats` (62), `destructive-guard.bats`, `protect-secrets.bats`, `hook-robustness.bats`, `policy-enforcement.bats`, `*-emission.bats`, `dim2-gates/*`, … | Behaviour-level. Pipes JSON straight into `hooks/<x>.sh` and checks exit code and stderr. Doesn't depend on the dispatcher, but some tests check that hooks are registered via the registry helper |
| Other | 40 | ~767 | `bin/` tools (factory-obs 81, emit-event 50, factory-query, factory-lock-*), skills content, permissions (70), docs/semver, `registry-tool-filter-anchoring` (a registry lint) | Mostly not about hooks |

**Rust tests:**
- `factory-dispatcher`: 41,748 lines in `tests/`, about 1,003 `#[test]`. Covers the executor, host functions, routing, sinks, internal log, resolvers (including proptest), marker, shard/BC-1.18.* migration with failpoints, latency canary, async partition.
- Plugin crates: about 1,900 `#[test]` total, inline or under `tests/`. Largest: `cross-site-correspondence` 231, `factory-path-staging` 138, `artifact-path` 94, `dispatch-advance` 81.
- Others: `last-amended-migrate` 107, `hook-sdk` 65, `sink-core` 55; the other sink crates 16–36 each.
- WAT fixtures: `crates/factory-dispatcher/tests/fixtures/*.wat`.

---

## 8. ADRs and docs on why the dispatcher/WASM design exists

The ADRs live on the `factory-artifacts` branch at `specs/architecture/decisions/` (read with `git show origin/factory-artifacts:<path>`). That branch isn't checked out here.

| ADR | One-line summary |
|---|---|
| ADR-001-rust-dispatcher | One compiled Rust binary per platform, committed by release CI. Motivation: Claude Code 0.79.x `PostToolUse:Bash` matcher dedup bug (claude-code#52715) and no Windows support for bash-only hooks |
| ADR-002-wasm-plugin-abi | Plugins as `wasm32-wasip1` modules on wasmtime: language-agnostic, sandboxed, small (avoids ~300 MB of native binaries per platform), no dispatcher rebuild per new hook |
| ADR-003-wasi-preview1 | WASI p1 now; p2 deferred |
| ADR-004-toml-config | TOML for all config |
| ADR-005-multi-sink-observability | **Superseded** by ADR-015; Router/SinkRegistry never wired |
| ADR-006 / ADR-010 | `HOST_ABI_VERSION`; StoreData-typed linker |
| ADR-007-always-on-telemetry | `dispatcher-internal-*.jsonl` |
| ADR-008-parallel-within-tier | Parallel within a tier, sequential between tiers |
| ADR-009-activation-platform-selection | `/activate` copies `hooks.json.<platform>` |
| ADR-011-dual-hook-routing-tables | `hooks.json` + registry during migration |
| ADR-012-legacy-bash-adapter-router | One adapter `.wasm` runs all unported bash hooks (~44 at design time) |
| ADR-014-tier-2-native-wasm-migration | Proposed rewrite-clean port of 23 `validate-*.sh` hooks |
| ADR-015 | Single-stream, OTel-aligned events |
| ADR-018 | Context resolvers |
| ADR-019 | Async semantics at the registry layer; block⇒sync; fail-closed exit 2 |
| ADR-020 | Latency budget classes |
| ADR-021 | cargo-audit cache |
| ADR-024 | Log-dir resolution and `CLAUDE_PLUGIN_ROOT` fail-loud |
| ADR-028 | Native precompact-flush |
| ADR-029 | git_context injection |
| ADR-035 / ADR-042 | Fuel budget, 20M cap |
| ADR-039 | `failure_policy` |
| ADR-043 | exec `binary_allow` resolution |
| ADR-046 | stamp-state-timestamp |
| ADR-047 / ADR-048 | INDETERMINATE result, mutation marker, `block_if_marker` |
| ADR-051 / ADR-052 / ADR-053 / ADR-054 | Sharding and the BC-INDEX migration CLI |

**In-repo docs:**

| Doc | Summary |
|---|---|
| `crates/hook-sdk/HOST_ABI.md` | The plugin↔host contract. Partly out of date about `on_error` |
| `docs/guide/hooks-reference.md` | Operator reference for each hook |
| `docs/guide/migrating-from-0.79.md` | Bash `hooks.json` → dispatcher migration |
| `docs/guide/plugin-marketplace-architecture.md` | Plugin packaging and install |
| `docs/guide/authoring-hooks.md`, `porting-bash-hook-to-wasm.md`, `observability-sinks.md` | Skeletons, bodies empty |
| `docs/E-8-tier1-native-audit.md` | Tier-1 WASM port audit |
| `docs/oq-6-regression-gate-security-audit.md` | regression-gate security audit |
| `plugins/vsdd-factory/hooks/dim2-gates/README.md` | Says those scripts are not hooks |
| `plugins/vsdd-factory/docs/` | VSDD, FACTORY, CONVERGENCE, bash-portability; general, not dispatcher-specific |
