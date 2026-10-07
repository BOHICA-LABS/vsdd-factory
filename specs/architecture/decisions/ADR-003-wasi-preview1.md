---
document_type: adr
adr_id: ADR-003
version: "1.1"
status: accepted
date: 2026-04-24
last_amended: "2026-10-07 (v1.1) — architect: wasmtime reference updated 44.0 -> 48.x per security fix (ADR-035 §Decision 6 v1.2); WASI p1 decision unchanged. [Prior: 2026-04-24 (v1.0) — initial ruling.]"
modified:
  - "2026-04-24 (v1.0)"
  - "2026-10-07 (v1.1)"
subsystems_affected: [SS-02, SS-04]
supersedes: null
superseded_by: null
---

# ADR-003: WASI Preview 1 for v1.0; Preview 2 Deferred to v2.0

## Context

WASM plugins need a system interface to do anything useful beyond pure computation.
Two WASI generations exist: WASI preview 1 (p1) — stable, widely supported,
provides stdin/stdout/stderr/env/file-via-preopens — and WASI preview 2 (p2) /
the Component Model, which adds wasi-http, wasi-sockets, wasi-command, and a
richer type system but was still maturing at the time of v1.0 design.

The dispatcher's host function surface (`exec_subprocess`, `read_file`, `emit_event`,
`env`, context getters) covers the plugin use-cases that p2 would enable natively.
Network access is the primary gap: p1 plugins that need HTTP must shell out via
`exec_subprocess` (cap-gated). This is accepted as a known constraint for v1.0.

## Decision

Use WASI preview 1 (`wasm32-wasip1` target triple) for all plugins in v1.0. Migrate
to WASI preview 2 / Component Model in v2.0 when the ecosystem matures.

## Rationale

WASI p1 is stable and has been shipping in production wasmtime deployments for years.
p2 is the correct long-term direction but introduces breaking ABI changes; adopting
p2 in v1.0 would mean building on a moving target. The host function surface covers
the remaining capability gaps (network via `exec_subprocess`, FS via `read_file`,
env via `env`). The cost — plugins must shell out for HTTP — is acceptable during
the beta/rc window since most current hooks are bash scripts anyway.

`wasmtime = "48.0.4"` (>= 48.0.4; was `"44.0"` at v1.0) supports both p1 and p2; the p2
migration is a controlled upgrade path, not a rewrite. In the 48.x line the p1 preopen surface is
`WasiCtxBuilder::preopened_dir(host_path, guest_path, FsPerms)` (3 args; the 46.x form took separate
`DirPerms` + `FilePerms` and was 4 args). The dispatcher grants the full-permission equivalent, so the
p1 filesystem capability model this ADR relies on is unchanged.

## Consequences

### Positive
- Stable, well-tested WASI surface with no in-flight spec churn.
- Plugin target triple `wasm32-wasip1` is directly supported by stable Rust toolchain.
- `wasmtime = "48.0.4"` fully supports p1 (verified across the 44 -> 46 -> 48 moves); no experimental feature flags needed.

### Negative / Trade-offs
- Plugins cannot make HTTP requests natively; must use cap-gated `exec_subprocess`
  (shells out to `curl`/`wget`/`gh`). This is acknowledged as a known gap until v2.0.
- p2 migration will require a plugin rebuild (target triple changes) and likely a
  `HOST_ABI_VERSION` bump.

### Status as of v1.0.0-beta.4
IN-EFFECT. All plugins target `wasm32-wasip1`. The constraint is documented in the
authoring guide (`docs/guide/authoring-hooks.md`). NFR-SEC-014 explicitly cites
ADR-003 for the WASI p1 boundary.

## Alternatives Considered

- **WASI preview 2 now:** Rejected — spec was still maturing at design time; would
  build on an unstable foundation.
- **No WASI (raw WASM imports only):** Rejected — plugins need at minimum stdin/stdout
  for the payload/result protocol; pure WASM imports would require reimplementing that.

## Changelog

| Version | Date | Change |
|---------|------|--------|
| 1.1 | 2026-10-07 | wasmtime reference updated 44.0 -> 48.x (>= 48.0.4) per ADR-035 §Decision 6 v1.2 (RUSTSEC-2026-0316/0314/0321/0322/0323/0324/0327); documented the 3-arg `preopened_dir`/`FsPerms` p1 preopen surface. Decision itself unchanged. |
| 1.0 | 2026-04-24 | Initial ruling (WASI p1 for v1.0; p2 deferred to v2.0) |

## Source / Origin

`/Users/jmagady/Dev/vsdd-factory/.factory/legacy-design-docs/2026-04-24-v1.0-factory-plugin-kit-design.md`
lines 436–444 (ADR-003 section). Also explicitly cited in `pass-4-nfr-catalog.md`
NFR-SEC-014.
