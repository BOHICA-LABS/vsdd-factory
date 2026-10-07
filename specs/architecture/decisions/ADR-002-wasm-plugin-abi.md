---
document_type: adr
adr_id: ADR-002
version: "1.1"
status: accepted
date: 2026-04-24
last_amended: "2026-10-07 (v1.1) — architect: wasmtime / wasmtime-wasi pin updated 44.0 -> 48.x (>= 48.0.4) per security fix (RUSTSEC-2026-0316/0314/0321/0322/0323/0324/0327); see ADR-035 §Decision 6 v1.2. [Prior: 2026-04-24 (v1.0) — initial ruling.]"
modified:
  - "2026-04-24 (v1.0)"
  - "2026-10-07 (v1.1)"
subsystems_affected: [SS-01, SS-02, SS-04]
supersedes: null
superseded_by: null
---

# ADR-002: WASM (wasmtime) Plugin ABI

## Context

vsdd-factory needed a plugin execution model that is language-agnostic, sandboxed
by default, small to distribute, and fast to instantiate (~1ms per hook event).

Native binaries would require per-platform cross-compilation: ~2MB × 30 hooks × 5
platforms = ~300MB shipped alongside the dispatcher — prohibitive. A monolithic
dispatcher (all hooks compiled in) means every new hook requires a dispatcher
rebuild and release, blocking third-party experimentation.

## Decision

Each hook plugin is a `.wasm` module compiled to `wasm32-wasip1`. The dispatcher
loads and executes plugins via `wasmtime = "48.0.4"` + `wasmtime-wasi = "48.0.4"`
(minimum 48.0.4, resolving to the latest 48.0.x patch; pinned lockstep in
`Cargo.toml [workspace.dependencies]`; the concrete line is governed by ADR-035
§Decision 6 v1.2 — the pin was `"44.0"` at v1.0, then `"46.0.3"` before the 48.x security move). Rust is the reference
plugin authoring language via the `hook-sdk` crate and the `#[hook]` proc-macro.
The plugin ABI is the WASI preview-1 stdio boundary plus `vsdd::*` host function
imports registered by the dispatcher's wasmtime `Linker`.

## Rationale

| Option | Size | Sandbox | Lang-agnostic | Instantiation |
|--------|------|---------|---------------|---------------|
| WASM (wasmtime) | ~100–200KB/plugin | Default | Yes | ~1ms |
| Native binaries | ~2MB × 5 platforms | No | Yes | ~1–5ms |
| Monolithic Rust | 0 extra | No | No | n/a |
| Embedded scripting (Rhai/Lua) | Small | Partial | New language | Fast |

One `.wasm` runs on every OS. ~6MB total for ~30 plugins vs ~300MB for native.
Sandbox is the default posture: wasmtime provides epoch interruption, fuel metering,
and WASI capability restriction without extra effort. wasmtime is production-grade
(Fastly, Shopify). ~1ms instantiation satisfies the latency budget. Language-agnostic
boundary means future plugin authors can use any `wasm32-wasip1`-targeting language.

Embedded scripting (Rhai/Lua) was rejected: introduces an unfamiliar language
surface for plugin authors and adds cognitive overhead.

## Consequences

### Positive
- One `.wasm` runs on all 5 supported platforms without recompilation.
- Sandbox by default: epoch interruption (10ms tick), fuel metering (10M units), WASI
  capability restriction.
- Adding a hook requires no dispatcher rebuild — drop `.wasm`, update `hooks-registry.toml`.
- Language-agnostic: any `wasm32-wasip1` target language can author plugins.

### Negative / Trade-offs
- Plugins cannot access the network directly; must use cap-gated `exec_subprocess`
  to shell out. WASI preview-2 (wasi-http, wasi-sockets) deferred to v2.0 per ADR-003.
- Exact-line wasmtime pinning (`"48.0.4"`; was `"44.0"` at v1.0) creates upgrade friction when wasmtime makes
  breaking changes — realized in the 46 -> 48 move (WASI `FilePerms` renamed `FsPerms`;
  `WasiCtxBuilder::preopened_dir` 4 -> 3 args; sandbox semantics unchanged — see ADR-035 §Decision 6 v1.2).
- Plugin debugging requires WASI-aware tooling.

### Status as of v1.0.0-beta.4
IN-EFFECT. All 45 registry entries load `legacy-bash-adapter.wasm` via this ABI
(45 entries per `hooks-registry.toml` head as of beta.4; see ARCH-INDEX SS-04 + SS-07 BCs for breakdown).
`capture-commit-activity.wasm` is a 20-LOC stub (S-3.1 in flight). Plugin target
triple `wasm32-wasip1` confirmed in `crates/hook-sdk/` and `crates/hook-plugins/`.

## Alternatives Considered

- **Native binaries per platform:** Rejected on distribution size (~300MB).
- **Monolithic Rust:** Rejected: inhibits third-party hooks; requires dispatcher rebuild
  per hook change.
- **Embedded scripting (Rhai/Lua):** Rejected: unfamiliar language for plugin authors.

## Changelog

| Version | Date | Change |
|---------|------|--------|
| 1.1 | 2026-10-07 | Pin updated to `wasmtime`/`wasmtime-wasi` 48.x (>= 48.0.4) — clears RUSTSEC-2026-0316/0314/0321/0322/0323/0324/0327 (46.x is EOL, no patch). 49.x deferred to a separate toolchain-upgrade story (requires rustc 1.96; pinned 1.95). Decision rationale: ADR-035 §Decision 6 v1.2. Plugin ABI (`wasm32-wasip1` + `vsdd::*` imports) and `HOST_ABI_VERSION` unchanged. |
| 1.0 | 2026-04-24 | Initial ruling; `wasmtime = "44.0"` + `wasmtime-wasi = "44.0"` |

## Source / Origin

`/Users/jmagady/Dev/vsdd-factory/.factory/legacy-design-docs/2026-04-24-v1.0-factory-plugin-kit-design.md`
lines 418–434 (ADR-002 section).
