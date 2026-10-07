---
document_type: prd
level: L3
version: "1.6"
status: draft
producer: product-owner
timestamp: 2026-10-07T00:00:00Z
phase: 1.5
origin: brownfield
inputs:
  - .factory/specs/domain-spec/L2-INDEX.md
  - .factory/specs/domain-spec/capabilities.md
  - .factory/specs/domain-spec/invariants.md
  - .factory/specs/domain-spec/business-rules.md
  - .factory/specs/domain-spec/edge-cases.md
  - .factory/specs/architecture/ARCH-INDEX.md
  - .factory/specs/dtu-assessment.md
  - .factory/phase-0-ingestion/pass-4-nfr-catalog.md
  - .factory/phase-0-ingestion/pass-8-final-synthesis.md
  - .factory/legacy-design-docs/2026-04-24-v1.0-factory-plugin-kit-design.md
input-hash: "e75f6e1"
traces_to: .factory/specs/domain-spec/L2-INDEX.md
supplements: []
# Supplements deferred — PRD body contains summary versions:
# interface-definitions → §3 (Functional Requirements, interface contracts per FR)
# error-taxonomy       → §5 (Behavioral Constraints, error handling rules)
# test-vectors         → §6 (Quality Attributes, acceptance criteria)
# nfr-catalog          → §4 (Non-Functional Requirements)
# Detailed extractions to prd-supplements/ are TBD.
---

# Product Requirements Document: vsdd-factory

> **Context Engineering — Extended ToC Pattern:**
> This PRD is an index document for Phase 1.5 brownfield spec backfill.
> It synthesizes the 1,943-BC catalog (1,863 pre-E-7 baseline + 15 E-7 process
> codification + 13 S-7.03 TDD hardening + 2 Wave 11 SS-03 + 5 S-5.01 foundation + 2 S-5.01 pass-2 SS-01 host fns + 5 S-5.02 Wave 13 + 6 F2 engine-discipline-pass-1 + 6 F2-amendment resolver platform) into a formal L3 requirements artifact. Section 2 is the
> primary machine-consumed surface: it groups BCs by functional requirement (FR-NNN)
> and provides subsystem-level traceability. Agents needing deep BC content load
> individual `.factory/specs/behavioral-contracts/ss-NN/BC-S.SS.NNN.md` files on demand.
> Sections 3-5 point to supplement files (DF-021 context discipline).

> **BC Index Model:** 1,943 individual BC files live under
> `.factory/specs/behavioral-contracts/ss-NN/`. Section 2 groups them into
> 48 logical FRs. Do NOT inline full contract details here — cross-reference only.

---

## 1. Product Overview

### 1.1 Problem Statement

Solo developers and small teams using Claude Code face two persistent problems.
First, ad-hoc hook scripting is brittle: bash scripts are untyped, platform-fragile
(Windows), hard to test, and invisible to observability tooling. Second, autonomous
agent-driven SDLC workflows lack discipline: there is no standard for how LLM
sub-agents should receive specs, verify behavioral contracts, produce tested code, or
pass adversarial review. The result is that teams either avoid automation (losing
productivity) or automate unsafely (accumulating silent technical debt and spec drift).

vsdd-factory solves both problems simultaneously in a single Claude Code marketplace
plugin: it provides a typed, sandboxed, observable hook dispatcher (Subsystem A) and
a 119-skill orchestration framework that encodes the full Verified Spec-Driven
Development (VSDD) methodology (Subsystem B).

### 1.2 Solution Vision

vsdd-factory is a **self-referential, two-subsystem Claude Code plugin** — the engine
and the product are one. Subsystem A (compiled Rust) intercepts every Claude Code tool
call, routes events through sandboxed WASM plugins with capability-gated host
functions, and fans out structured telemetry to multiple configurable sinks. Subsystem B
(declarative markdown + Lobster workflows) drives 34 specialist LLM sub-agents through
an 8-phase SDLC pipeline: brief → domain-spec → PRD → architecture → stories → TDD
delivery → adversarial review → convergence.

The product was built with itself. Phase 0 ingestion of this very codebase produced the
1,863-BC pre-E-7 baseline that this PRD synthesizes (catalog has since grown to 1,900
with additions of 15 BCs in E-7, 13 BCs in S-7.03, 2 BCs in Wave 11 SS-03, 5 BCs in S-5.01 Wave 13, and 2 BCs in S-5.01 pass-2). This self-referential loop is the ultimate dogfooding
test: every architectural decision (WASM sandbox, capability deny-by-default,
parallel-within-tier execution, always-on telemetry) was enacted in Rust and then
analyzed by the framework's own brownfield-ingest skill.

**Current shipping milestone:** `1.0.0-beta.7` shipped 2026-04-26 at commit `b08e085`.
Tiers A through D (26 merged + 3 partial stories; 29 total) are substantially closed.
Tiers E through H (18 draft stories) are the active backlog for rc.1 and 1.0 GA.

### 1.3 Key Differentiators

| ID | Differentiator | Description |
|----|---------------|-------------|
| KD-001 | Self-orchestrating LLM pipeline | The orchestrator agent reads `.lobster` workflow files and autonomously dispatches specialist sub-agents through all 8 SDLC phases — the human only approves gates |
| KD-002 | Sandbox-aware WASM plugin execution | wasmtime + WASI preview-1 + capability deny-by-default + bounded fuel + epoch interruption — plugins can be blocked before they run dangerous code |
| KD-003 | Always-on multi-sink observability | `dispatcher-internal-YYYY-MM-DD.jsonl` is written for every hook invocation, independent of sink config; file and OTel/gRPC sinks fan out in parallel |
| KD-004 | Cross-platform native dispatcher | A compiled Rust binary ships for 5 platforms (darwin-arm64, darwin-x64, linux-x64, linux-arm64, windows-x64) with per-platform hooks.json variants activated at install time |
| KD-005 | Strict-binary novelty assessment | Adversarial review and convergence use binary SUBSTANTIVE / NITPICK severity — no gradient, no half-measures |
| KD-006 | Brownfield ingest as first-class workflow | A 7-pass structured analysis produces a verified BC catalog from any existing codebase; this PRD was produced by that pipeline |

### 1.4 Target Users

| Persona | Description | Volume | Pain Level |
|---------|-------------|--------|------------|
| Solo developer / Claude Code power user | Individual using Claude Code for autonomous coding; wants guarded, observable automation without managing infra | Primary | HIGH — currently managing raw bash hook scripts with no typing or observability |
| Small team (2–8 engineers) | Teams standardizing on Claude Code for feature delivery; wants shared VSDD discipline and BC-traceable acceptance criteria | Secondary | HIGH — spec drift and lack of traceability are recurring review complaints |
| Plugin author | Rust or other-language developer who wants to write typed WASM hooks for Claude Code tool calls | Tertiary | MEDIUM — today's alternative is raw bash scripting with no type safety |
| Platform engineer / dogfooder | vsdd-factory itself — the pipeline that produces this PRD is a vsdd-factory product | Meta | VERY HIGH — this is the closed-loop self-referential test of every feature |

### 1.5 Out of Scope

> **Machine-consumed.** The adversary and consistency-validator check that no story AC
> implements features listed here. Be explicit and unambiguous.

- Plugin signing and marketplace discovery infrastructure (declared non-goal for v1.0; design doc)
- WASI preview-2 (wasi-http, wasi-sockets, component model) — deferred to v2.0 per ADR-003
- seccomp / AppArmor / Landlock sandboxing on Linux — declared v2+ in design doc
- Argument-level filtering for `exec_subprocess` (binary_allow only at v1.0 per design Q4)
- Multi-tenant / multi-organization deployment (vsdd-factory is a developer-workstation tool)
- A web dashboard or UI — all observability is via log files, OTel collector, or CLI bin tools
- Project management tooling (Jira, Linear, Asana) — vsdd-factory is an SDLC spec+delivery pipeline, not a PM tool
- Code review as a service — adversarial review is an internal agent-to-agent quality gate, not a SaaS product
- Hosted / SaaS deployment of the dispatcher — it runs locally on the developer's machine only
- Plugin marketplace listing management — installation is via Claude Code's built-in plugin mechanism
- Complex authentication at the dispatcher boundary (Claude Code is the trust boundary; no dispatcher auth)

---

## 2. Behavioral Contracts Index

> BCs are grouped into 48 logical FRs. Each FR maps to one or more CAP-NNN
> capabilities, one or more SS-NN subsystems, and the specific BC prefix ranges
> that implement it. Full BC files live in
> `.factory/specs/behavioral-contracts/ss-NN/`. Status = shipped / partial / pending
> reflects Pass-8 story-coverage rollup.

### 2.1 Hook Dispatcher Core (SS-01)

#### FR-001 — Registry loading and schema validation

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-1.01.001 | Registry rejects unknown schema version | P0 |
| BC-1.01.002 | Registry rejects invalid tool regex at load time | P0 |
| BC-1.01.003 | Registry rejects unknown entry fields (typo guard) | P0 |
| BC-1.01.004 | Relative plugin paths resolve against registry file's parent directory | P0 |
| BC-1.01.005 | Plugin filter requires event match AND (no tool OR tool regex matches) | P0 |
| BC-1.01.006 | Tiers ordered ascending by priority, registry order preserved within tier | P0 |
| BC-1.01.007–015 | Registry parsing, defaults, capabilities block, overrides, not-found handling | P0 |

Source BCs: `ss-01/BC-1.01.001.md` through `BC-1.01.015.md` (15 BCs).
Enforces: DI-014. Status: **shipped** (S-1.02).

#### FR-002 — Hook payload parsing and event routing

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-1.02.001 | HookPayload requires non-empty event_name and session_id | P0 |
| BC-1.02.002 | HookPayload accepts both `event_name` and `hook_event_name` | P0 |
| BC-1.02.003–005 | PreToolUse / PostToolUse / SessionStart envelope deserialization | P0 |

Source BCs: `ss-01/BC-1.02.001.md` through `BC-1.02.005.md` (5 BCs).
Enforces: DI-017 (trace ID propagation). Status: **shipped** (S-1.02).

#### FR-003 — Plugin execution lifecycle (epoch, fuel, crash, block-intent)

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-1.03.001 | Plugin in infinite loop times out via epoch interruption | P0 |
| BC-1.03.002 | Plugin in tight arithmetic loop runs out of fuel | P0 |
| BC-1.03.003 | Plugin trap reports as Crashed | P0 |
| BC-1.03.004 | Normal plugin returns Ok with exit_code 0 + fuel recorded | P0 |
| BC-1.03.005 | stderr captured per plugin and truncated at 4 KiB with marker | P0 |
| BC-1.03.006 | Empty stderr is omitted from lifecycle events | P1 |
| BC-1.03.007 | Tier execution preserves between-tier order | P0 |
| BC-1.03.008 | Plugins within a tier execute concurrently | P0 |
| BC-1.03.009 | block_intent set only when on_error=block AND plugin asks to block | P0 |
| BC-1.03.010 | Per-plugin plugin_config spliced into HookPayload before invocation | P0 |
| BC-1.03.011 | WASI exit(N) maps to PluginResult::Ok with exit_code N | P0 |
| BC-1.03.012–016 | Integration: 5-plugin happy path, crash isolation, timeout non-cascade, multi-tier order, empty tier | P0 |

Source BCs: `ss-01/BC-1.03.001.md` through `BC-1.03.016.md` (16 BCs).
Enforces: DI-001 (tier ordering), DI-002 (crash isolation), DI-003 (block-intent), DI-010 (stderr cap), DI-016 (per-plugin config). Status: **shipped** (S-1.05, S-1.06).

#### FR-004 — Engine construction: epoch ticker and fuel

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-1.04.001 | Engine builds with epoch interruption + fuel + reference types | P0 |
| BC-1.04.002 | Epoch ticker advances epoch every 10ms; cooperative shutdown | P0 |
| BC-1.04.003 | timeout_ms_to_epochs rounds up | P0 |

Source BCs: `ss-01/BC-1.04.001.md` through `BC-1.04.003.md` (3 BCs).
Enforces: CAP-011. Status: **shipped** (S-1.05).

#### FR-005 — Host function surface (capability-gated exec, env, read_file, log, emit_event)

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-1.05.001 | exec_subprocess denies when no exec_subprocess capability | P0 |
| BC-1.05.002 | exec_subprocess denies binaries not on allow-list | P0 |
| BC-1.05.003 | exec_subprocess denies shell interpreters without shell_bypass_acknowledged | P0 |
| BC-1.05.004 | exec_subprocess refuses setuid/setgid binaries categorically (Unix) | P0 |
| BC-1.05.005 | exec_subprocess returns OUTPUT_TOO_LARGE when result exceeds buffer | P0 |
| BC-1.05.006 | exec_subprocess result envelope binary wire format | P0 |
| BC-1.05.007 | env host fn denies env var not on allow-list | P0 |
| BC-1.05.008 | env host fn returns 0 when var allowed but unset | P0 |
| BC-1.05.009 | read_file at StoreData-typed linker is currently CAPABILITY_DENIED stub | P0 |
| BC-1.05.010 | Context getters always return current value | P0 |
| BC-1.05.011 | log host fn emits plugin.log event with level mapped | P0 |
| BC-1.05.012 | emit_event filters out reserved field names | P0 |
| BC-1.05.013–034 | emit_event decode / encode unit tests; read_file full impl; exec integration tests | P0–P1 |

Source BCs: `ss-01/BC-1.05.001.md` through `BC-1.05.034.md` (34 BCs).
Enforces: DI-004 (denial co-event), DI-005 (shell-bypass), DI-006 (setuid refusal), DI-017 (trace ID on every event). Status: **shipped** for deny paths; **partial** for read_file (DRIFT-001, L-P0-001).

#### FR-006 — Internal log (always-on self-telemetry)

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-1.06.001 | Internal log writes are best-effort; never panic; never propagate | P0 |
| BC-1.06.002 | Daily rotation by event timestamp produces separate files per UTC date | P0 |
| BC-1.06.003 | Internal log auto-creates missing parent directories | P0 |
| BC-1.06.004 | prune_old removes only dispatcher-internal-*.jsonl files older than threshold | P0 |
| BC-1.06.005 | prune_old is no-op when log dir missing | P0 |
| BC-1.06.006 | InternalEvent fields flatten to top-level JSON | P0 |
| BC-1.06.007–010 | JSONL shape tests; None-field omission; startup-flow round-trip; best-effort on bad path | P0 |

Source BCs: `ss-01/BC-1.06.001.md` through `BC-1.06.010.md` (10 BCs).
Enforces: DI-007 (always-on), DI-008 (timestamp-derived rotation), DI-009 (30-day retention). Status: **shipped** (S-1.07).

#### FR-007 — Legacy hook routing compatibility and dispatcher main

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-1.07.001 | All 30+ existing bash hooks fire via legacy-bash-adapter on Linux/macOS | P0 |
| BC-1.07.002 | commit.made events fire reliably on real git commit | P0 |
| BC-1.07.003 | Generated hooks-registry.toml round-trips through Registry::load | P0 |
| BC-1.07.004 | Registry-generation script is idempotent | P0 |
| BC-1.07.005–006 | Every production entry routes through legacy-bash-adapter.wasm; carries script_path | P0 |
| BC-1.08.001 | Dispatcher exits 0 on registry/payload/engine errors (non-blocking) | P0 |
| BC-1.08.002 | Dispatcher exit code is 2 iff at least one block_intent recorded | P0 |
| BC-1.08.003–006 | Runtime: current_thread, CLAUDE_PROJECT_DIR cwd, plugin_root injection, env projection | P0 |

Source BCs: `ss-01/BC-1.07.001.md` through `BC-1.08.006.md` (10 BCs).
Enforces: CAP-002 (hook integration), DI-001 (execution model). Status: **shipped** (S-2.01, S-2.02, S-2.07).
<!-- Wave 9 SS-01 straggler re-anchor: S-2.07 (regression-test-validation) anchors BC-1.07.001/002 + BC-1.08.001/002 as the primary integration-validation story for FR-007. -->

#### FR-008 — Plugin cache (mtime-driven, process-lifetime)

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-1.09.001 | PluginCache key is path only; invalidation is mtime-driven | P1 |
| BC-1.09.002 | PluginCache.get_or_compile is thread-safe via Mutex<HashMap> | P1 |
| BC-1.09.003 | PluginCache has no eviction — entries live for dispatcher process lifetime | P1 |
| BC-1.09.004 | Missing plugin path returns NotFound; corrupt returns Compile; IO errors carry context | P1 |

Source BCs: `ss-01/BC-1.09.001.md` through `BC-1.09.004.md` (4 BCs).
Status: **shipped** (S-1.05, S-1.06).

> Full contracts: `.factory/specs/behavioral-contracts/ss-01/` (99 BCs total)

---

### 2.2 Hook SDK and Plugin ABI (SS-02)

#### FR-009 — Plugin ABI types and host function SDK

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-2.01.001 | HookResult serialization is tagged with `outcome` field | P0 |
| BC-2.01.002 | HookResult exit codes Continue=0 / Block=2 / Error=1 | P0 |
| BC-2.01.003 | HOST_ABI_VERSION is 1 in both crates | P0 |
| BC-2.01.004 | SDK HookPayload has `plugin_config` field defaulting to Null | P0 |
| BC-2.02.001 | Plugin-author API surface is `vsdd_hook_sdk::host::*`; raw FFI is private | P0 |
| BC-2.02.002 | Bounded host calls are mandatory — read_file and exec_subprocess REQUIRE explicit limits | P0 |
| BC-2.02.003 | HostError code mapping: -1=CapabilityDenied, -2=Timeout, -3=OutputTooLarge, -4=InvalidArgument | P0 |
| BC-2.02.004 | SubprocessResult envelope decoding is paranoid — rejects truncated input rather than panicking | P0 |
| BC-2.02.005 | SDK-side read_string re-call protocol — host returns required size; SDK reallocates | P0 |
| BC-2.02.006–010 | FFI target-conditional; encode_fields; encode_args; decode_subprocess_result; log level stability | P0 |
| BC-2.04.001–005 | SDK payload: PreToolUse, PostToolUse, SessionStart deserialization; round-trip; plugin_config pass-through | P0 |
| BC-2.05.001–003 | Panic hook extracts message for all payload types | P1 |

Source BCs: `ss-02/BC-2.01.001.md` through `BC-2.05.003.md` (22 BCs total).
Enforces: CAP-009. Status: **shipped** (S-1.03); crates.io publish **partial** (S-2.05, DRIFT on publish step).

> Full contracts: `.factory/specs/behavioral-contracts/ss-02/` (22 BCs total)

---

### 2.3 Event Emission (OTel-Aligned) (SS-03)

#### FR-010 — Sink registry, routing filter, and config loading

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-3.01.001 | Empty SinkRegistry submit/flush/shutdown is no-op | P0 |
| BC-3.01.002 | Unknown sink type warns to stderr but does not fail config load | P0 |
| BC-3.01.003 | Sink schema_version != 1 is a hard error | P0 |
| BC-3.01.004 | RoutingFilter empty=pass-through, allow non-empty=whitelist, deny applied after allow | P0 |
| BC-3.01.005 | SinkEvent serializes flat (transparent over Map) | P0 |
| BC-3.06.001–006 | sink-core unit: routing filter defaults, event_type accessor, config defaults, case-sensitivity | P0 |

Source BCs: `ss-03/BC-3.01.001.md` through `BC-3.01.005.md` + `BC-3.06.001.md` through `BC-3.06.006.md` (11 BCs in this group).
Enforces: DI-011 (non-blocking submit), DI-012 (failure isolation), DI-013 (unknown type), DI-014 (schema version). Status: **shipped** (S-1.08, S-1.09).

#### FR-011 — File sink (daily-rotated JSONL with path templates, tags, routing)

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-3.01.006 | File sink path template substitutes {date}, {name}, {project} and rejects unknown placeholders | P0 |
| BC-3.01.007 | File sink mpsc bounded at default 1000; submit is non-blocking via try_send | P0 |
| BC-3.01.008 | File sink failures recorded into Mutex<Vec<SinkFailure>> | P0 |
| BC-3.02.001–016 | File sink template variants; mkdir-p; JSONL append; routing filter; tag enrichment; disabled sink; backpressure; shutdown drain; TOML config deserialization | P0 |

Source BCs: `ss-03/BC-3.01.006.md` through `BC-3.01.008.md` + `BC-3.02.001.md` through `BC-3.02.016.md` (19 BCs in this group).
Enforces: DI-011, DI-012. Status: **shipped** (S-1.08). Tag enrichment per-sink wiring: **partial** (S-4.06, DRIFT partial).

#### FR-012 — OTel gRPC sink (OTLP/gRPC batching, self-healing reconnect)

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-3.01.009 | OTel-gRPC sink loads with unreachable endpoint (lazy connect) | P0 |
| BC-3.03.001–013 | Batch trigger thresholds; send-failure self-healing; connection lifecycle; dedicated runtime; non-blocking submit; flush; shutdown; LogRecord mapping; OTLP attribute flatten | P0 |
| BC-3.04.001–002 | Router pass-through wrapper; extension point for S-4.x | P1 |
| BC-3.05.001–003 | Integration: builds file sink from parsed config; fans events to file sinks; 10-event OTLP attribute mapping | P0 |

Source BCs: `ss-03/BC-3.01.009.md` + `BC-3.03.001.md` through `BC-3.05.003.md` (19 BCs in this group).
Status: **shipped** (S-1.09). Retry/circuit-breaker: **pending** (S-4.04).

#### FR-044 — Per-sink resilience: retry, circuit breaker, dead-letter queue

Sink drivers carry per-instance resilience: configurable retry policy with
exponential backoff (BC-3.NN.NNN-retry-policy-exponential-backoff —
v1.1 candidate); per-sink circuit breaker state machine that opens/half-opens
based on failure rates (BC-3.NN.NNN-circuit-breaker-state-machine —
v1.1 candidate); dead-letter queue for messages whose sinks have exhausted
retries (BC-3.NN.NNN-dlq-on-retry-exhaustion — v1.1 candidate).

Source BCs (currently anchored): BC-3.01.008 (failure recording);
BC-3.03.002 (otel-grpc reconnect-on-error pattern);
BC-3.07.001 (sink-http retry backoff — promoted from v1.1 candidate, anchored by S-4.09 in Wave 11).
v1.1 BC candidates pending: 8 (S-4.04: retry policy, CB state machine, CB
event emission, retry isolation; S-4.05: DLQ on exhaustion, DLQ on overflow,
DLQ event shape, DLQ disk-full no-crash, DLQ daily rotation).
Source CAP: CAP-024 (per-sink retry, circuit breaker, dead-letter queue).
Status: **partial** — BC-3.07.001 anchored (S-4.09); remaining v1.1 BC creation pending (S-4.04, S-4.05).

#### FR-045 — Structured `internal.sink_error` event emission on each sink failure

**Source CAP:** CAP-003 — Internal events fan-out to all sinks
**Behavioral Contracts:** BC-3.07.002
**Stories:** S-4.10
**Status:** pending
**Subsystem(s):** SS-03

When any sink driver records a `SinkFailure`, the dispatcher emits an
`internal.sink_error` structured event into the same fan-out pipeline
that consumes plugin and dispatcher events. This makes sink failures
observable through the same JSONL/OTel streams as normal events,
enabling alerting and triage without requiring separate failure-log
inspection. The event is additive to BC-3.01.008's failure recording —
recording remains in `Mutex<Vec<SinkFailure>>`; emission is
fire-and-forget via `try_send` so failure events never block the
dispatcher.

Conforms to `INTERNAL_EVENT_SCHEMA_VERSION = 1` (PRD §3.1); the type
literal `internal.sink_error` is reserved.

> Full contracts: `.factory/specs/behavioral-contracts/ss-03/` (49 BCs total)

---

### 2.4 Plugin Ecosystem (SS-04)

#### FR-013 — Legacy bash adapter plugin (multi-instance, bash hook bridge)

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-4.01.001 | Legacy-bash-adapter requires non-empty plugin_config.script_path | P0 |
| BC-4.01.002 | Legacy-bash-adapter strips plugin_config to Null before piping payload to bash | P0 |
| BC-4.01.003 | Legacy-bash-adapter maps bash exit codes to HookResult | P0 |
| BC-4.01.004 | Legacy-bash-adapter caps combined output at 1 MiB | P0 |
| BC-4.01.005 | Legacy-bash-adapter caps wall-clock at 60_000ms (backstop only) | P0 |
| BC-4.01.006 | payload bytes reach bash with plugin_config stripped, event_name + trace_id preserved | P0 |
| BC-4.02.001–006 | stdout/stderr forwarding; exit-code mapping; script_path validation order; plugin_config strip; relative path resolution; wall-clock backstop relationship | P0 |

Source BCs: `ss-04/BC-4.01.001.md` through `BC-4.02.006.md` (12 BCs in this group).
Enforces: CAP-008, CAP-013. Status: **shipped** (S-2.01).

#### FR-014 — Capture-commit-activity plugin (PostToolUse event capture)

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-4.03.001 | capture-commit-activity on_hook stub returns 0 (pre-S-3.01 placeholder) | P1 |

Source BCs: `ss-04/BC-4.03.001.md` (1 BC).
Status: **pending** full implementation (S-3.01 stub).

#### FR-046 — New Claude Code lifecycle hook events: SessionStart/SessionEnd/WorktreeCreate/WorktreeRemove/PostToolUseFailure

**Source CAP:** CAP-002 — Hook Claude Code tool calls with sandboxed WASM plugins
**Behavioral Contracts:** BC-4.04.001..005 (session-start, S-5.01), BC-4.05.001..005 (session-end, S-5.02), BC-4.07.001..004 (worktree, S-5.03), BC-4.08.001..003 (tool-failure, S-5.04)
**Stories:** S-5.01, S-5.02, S-5.03, S-5.04
**Status:** in-progress (S-5.01 + S-5.02 + S-5.03 + S-5.04 BCs allocated; implementation pending)
**Subsystem(s):** SS-04, SS-01 (hooks.json registry inclusion)

The vsdd-factory dispatcher routes five new Claude Code lifecycle event types to dedicated WASM plugins:
SessionStart (session.started telemetry + factory-health brief),
SessionEnd (session.ended with duration/tool-call count),
WorktreeCreate (worktree.created with path/branch metadata),
WorktreeRemove (worktree.removed with cleanup status),
PostToolUseFailure (tool.error with tool_name + error_message context).

Closes DRIFT-006 (Phase 5 events not wired). Each plugin is registered in hooks.json.template
with appropriate once/async semantics. Dispatcher routing reuses existing event-name match logic
(BC-1.01.001 registry schema, BC-1.02.005 envelope parsing tolerates missing tool_name on
lifecycle events).

**Reserved event-name literals (per this FR):** The following event-name strings are reserved by vsdd-factory and must not be reused by user plugins or third-party extensions:
- `session.started` — emitted by S-5.01 (BC-4.04.001); reserved for session-start telemetry
- `session.ended` — emitted by S-5.02 (BC-4.05.*); reserved for session-end telemetry
- `worktree.created` — emitted by S-5.03 (BC-4.07.001); reserved for worktree creation events
- `worktree.removed` — emitted by S-5.03 (BC-4.07.002); reserved for worktree removal events
- `tool.error` — emitted by S-5.04 (BC-4.08.001); reserved for PostToolUseFailure events

These mirror the `internal.sink_error` reservation pattern in FR-045. Any plugin emitting an event with a reserved name that it does not own is a protocol violation.

**Canonical Tier F BC family pattern:** Each Tier F hook story produces a 4-5 BC family following this canonical pattern established by S-5.01's BC-4.04.001..005:
1. Plugin emits structured event with telemetry payload (e.g., BC-4.04.001)
2. Plugin invokes subsystem subprocess if applicable; fail-open (e.g., BC-4.04.002)
3. Plugin idempotency / deduplication contract (e.g., BC-4.04.003)
4. `hooks.json.template` registration (Claude Code-side routing) (e.g., BC-4.04.004)
5. `hooks-registry.toml` registration (dispatcher-side routing) (e.g., BC-4.04.005)

Sibling stories BC-4.05.*, BC-4.07.*, BC-4.08.* mirror this shape. Implementers and reviewers should use BC-4.04.001..005 as the canonical reference family.

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-4.04.001 | session-start plugin emits session.started event with session telemetry on SessionStart event | P1 |
| BC-4.04.002 | session-start plugin invokes factory-health subprocess; emits session.started even if check fails | P1 |
| BC-4.04.003 | session-start plugin is idempotent on duplicate SessionStart events within the same session_id | P1 |
| BC-4.04.004 | hooks.json.template registers SessionStart event with `command` field routing to dispatcher binary; once:true and async:true | P1 |
| BC-4.04.005 | hooks-registry.toml registers SessionStart event routing to hook-plugins/session-start-telemetry.wasm with read_file + exec_subprocess capability tables and timeout_ms:8000 | P1 |
| BC-4.05.001 | session-end plugin emits session.ended event with session telemetry on SessionEnd event | P1 |
| BC-4.05.002 | session-end plugin emits without subprocess invocation; fast-path completion | P1 |
| BC-4.05.003 | session-end plugin is unconditionally stateless; idempotency delegated to Layer 1 once:true | P1 |
| BC-4.05.004 | hooks.json.template registers SessionEnd event with `command` field routing to dispatcher binary; once:true and async:true | P1 |
| BC-4.05.005 | hooks-registry.toml registers SessionEnd event routing to hook-plugins/session-end-telemetry.wasm with timeout_ms:5000 | P1 |
| BC-4.07.001 | worktree-hooks plugin emits worktree.created event with {worktree_path, worktree_name} on WorktreeCreate event; zero-capability (Option A); 10-field wire payload | P1 |
| BC-4.07.002 | worktree-hooks plugin emits worktree.removed event with {worktree_path} on WorktreeRemove event; unknown-worktree no-op; zero-capability; 9-field wire payload | P1 |
| BC-4.07.003 | hooks.json.template registers WorktreeCreate and WorktreeRemove events with `command` field routing to dispatcher binary; once key ABSENT (can re-fire); async:true; timeout:10000 | P1 |
| BC-4.07.004 | hooks-registry.toml registers WorktreeCreate and WorktreeRemove routing to hook-plugins/worktree-hooks.wasm; single crate, two entries; ZERO capability tables; timeout_ms:5000 | P1 |
| BC-4.08.001 | tool-failure-hooks plugin emits tool.error event with {tool_name, error_message} on PostToolUseFailure event; tool_name="unknown" if absent; error_message truncated to 1000 chars; 10-field wire payload; RESERVED_FIELDS not set by plugin | P1 |
| BC-4.08.002 | hooks.json.template registers PostToolUseFailure with `command` routing to dispatcher binary; once key ABSENT (fires per-failure); async:true; timeout:10000 | P1 |
| BC-4.08.003 | hooks-registry.toml registers PostToolUseFailure with name="tool-failure-hooks", event="PostToolUseFailure", plugin="hook-plugins/tool-failure-hooks.wasm", timeout_ms=5000; ZERO capability tables; NO once field | P1 |

Source BCs: `ss-04/BC-4.04.001.md` through `BC-4.04.005.md` (S-5.01), `ss-04/BC-4.05.001.md` through `BC-4.05.005.md` (S-5.02), `ss-04/BC-4.07.001.md` through `BC-4.07.004.md` (S-5.03), `ss-04/BC-4.08.001.md` through `BC-4.08.003.md` (S-5.04) — 17 BCs anchored; all Tier F stories BCs allocated.
Status: **in-progress** (S-5.01 + S-5.02 + S-5.03 + S-5.04 BCs allocated; implementation pending).

> **S-5.04 foundation burst (2026-04-28):** BC-4.08.001–003 promoted from v1.1 candidates to real BCs. Option A (zero-capability) selected: tool-failure plugin emits `tool.error` ONLY — no subprocess, no file reads. 2 plugin-set fields: `tool_name` (fallback "unknown" if absent) + `error_message` (truncated to 1000 chars). `once` key ABSENT (per-failure event; mirrors S-5.03 worktree pattern). session_id is RESERVED_FIELDS host-enriched (per BC-1.05.012); plugin MUST NOT set it. Event-name corrected from PRD placeholder `tool.failed` to `tool.error` (matching story ACs). Closes DRIFT-006: 4/4 Tier F events wired (S-5.01/02/03/04 BCs all allocated). S-5.01 (14-pass) + S-5.02 (9-pass) + S-5.03 (14-pass) lessons applied up-front.

> **S-5.03 foundation burst (2026-04-28):** BC-4.07.001–004 promoted from v1.1 candidates to real BCs. Option A (zero-capability, no filesystem write) selected: worktree plugin emits events ONLY — no filesystem auto-registration (deferred to BC-4.07.005 candidate in v1.1 when `write_file` host fn is available). Single `worktree-hooks.wasm` crate handles both events via internal dispatch; two separate `[[hooks]]` registry entries. `once` key ABSENT for both events — worktree events can re-fire on reconnect, unlike session events. S-5.01 (14-pass) + S-5.02 (9-pass) lessons applied up-front; expected adversarial pass count: 5-7.

> **Pass-4 architectural simplification (2026-04-28):** BC-1.10.001 (host fn `vsdd::activated_platform()` — over-engineered; canonical `read_file` host fn used instead) and BC-1.10.002 (dispatcher-side dedup — over-engineered; Claude Code Layer 1 `once:true` directive enforces idempotency at hooks.json.template) retired. BC count for this FR reverts to 5 anchored BCs (BC-4.04.001–005). Scope reverts to SS-04 plugin work; story S-5.01 remains 3-pt as originally budgeted.

> Full contracts: `.factory/specs/behavioral-contracts/ss-04/` (30 BCs total — 27 prior + BC-4.08.001/002/003 added by S-5.04 foundation burst)

> **DI coverage note (F-12):** No dedicated L2 domain invariant covers the lifecycle event class (SessionStart/SessionEnd/WorktreeCreate/WorktreeRemove/PostToolUseFailure) as a first-class invariant. The applicable DIs are DI-004 (capability denial), DI-007 (always-on self-telemetry), DI-011 (sink submit non-blocking), DI-014 (schema version), DI-015 (activation gate), and DI-017 (trace_id). A dedicated DI for "lifecycle event plugins must always emit their target event regardless of subsystem health" is a **v1.1 DI candidate** — the fail-open semantics are currently enforced at BC level (BC-4.04.002 Invariant 1, BC-4.04.001 Postcondition 4) without a formal DI backing. Flag for Domain Spec v1.1 revision.

---

#### FR-047 (SS-04 slice) — Engine Governance: WASM Hooks for Convergence and Path Governance

**Source CAPs:** CAP-009 ("Author and publish WASM hook plugins using the Rust SDK"), CAP-008 (PreToolUse gating)
**Behavioral Contracts:** BC-4.10.001, BC-4.10.002, BC-4.11.001
**Stories:** Story B (validate-per-story-adversary-convergence), Story C (validate-artifact-path)
**Status:** pending (v1.0-feature-engine-discipline-pass-1)
**Subsystem(s):** SS-04

Two new WASM plugins are introduced by this FR in the SS-04 Plugin Ecosystem:

**1. `validate-per-story-adversary-convergence`** (Story B): Fires on `SubagentStop` at wave-gate dispatch.
Reads `.factory/cycles/<cycle-id>/<story-id>/adversary-convergence-state.json` for each story in the wave.
Blocks if any story is missing the file, has `passes_clean < 3`, or has `last_classification != "NITPICK_ONLY"`.
Gracefully degrades (returns Continue) when invoked outside wave-gate context (BC-4.10.002).

**2. `validate-artifact-path`** (Story C): Fires on `PreToolUse` for `Write`/`Edit` tool calls targeting `.factory/`.
Reads `plugins/vsdd-factory/config/artifact-path-registry.yaml` as the single source of truth.
Blocks writes whose paths match no registered pattern (`code: ARTIFACT_PATH_UNREGISTERED`).
Respects `enforcement_level: block | warn | advisory` per registry entry.
Ships in immediate `block` mode from registration (no phased rollout of the hook itself; registry entries start at `warn` per F1 design).

Both hooks use `HOST_ABI_VERSION = 1`, injectable-callback pure logic functions, and
`HookResult::block_with_fix(hook, reason, recommendation, code)` per the canonical Why/Fix/Code pattern.
Both follow the `handoff-validator` / `regression-gate` structural template.

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-4.10.001 | validate-per-story-adversary-convergence WASM hook MUST block wave-gate dispatch when any story lacks convergence clearance | P1 |
| BC-4.10.002 | validate-per-story-adversary-convergence WASM hook MUST gracefully degrade (exit 0) when invoked outside wave-gate context or when cycle directory is absent | P1 |
| BC-4.11.001 | validate-artifact-path WASM hook MUST consult artifact-path-registry.yaml as single source of truth and block (immediate mode) writes whose paths do not match a registered pattern | P1 |

Source BCs: `ss-04/BC-4.10.001.md`, `ss-04/BC-4.10.002.md`, `ss-04/BC-4.11.001.md` — 3 BCs (SS-04 slice of FR-047).
Verification Properties: VP-069 (registry load never panics — proptest), VP-070 (path matching deterministic — kani), VP-071 (adversary convergence gate block always emitted — kani), VP-072 (single-source-of-truth invariant — integration).

#### FR-048 (SS-01 + SS-04 slices) — Factory-Agnostic Runtime Context Injection via Sandboxed WASM-Plugin Resolvers

**Source CAPs:** CAP-009 ("Author and publish WASM hook plugins using the Rust SDK"), CAP-002 ("Hook Claude Code tool calls with sandboxed WASM plugins")
**Behavioral Contracts:** BC-1.13.001 (SS-01), BC-4.12.001 through BC-4.12.005 (SS-04)
**Stories:** S-12.03 (ContextResolver trait + ResolverRegistry), S-12.04 (WASM resolver loading + lifecycle), S-12.05 (hook-sdk resolver authoring), S-12.06 (HOST_ABI.md contract section), S-12.07 (vsdd-context-resolvers crate + WaveContextResolver), S-12.08 (convergence hook migration; closes F-P2-001)
**ADR:** ADR-018 (WASM-Plugin Context Resolvers — Design and Layering)
**Status:** pending (v1.0-feature-engine-discipline-pass-1 F2-amendment)
**Subsystem(s):** SS-01, SS-04

The dispatcher MUST support a resolver registry (`resolvers-registry.toml`) containing
zero or more WASM resolver plugins. When a hooks-registry entry declares
`needs_context = [resolver-name, ...]`, the dispatcher MUST invoke the named resolvers
before hook dispatch and merge their outputs into `plugin_config` under each resolver's
declared key. Resolvers MUST run with capability-restricted filesystem access
(`path_allow` declarations; no `write_file`, `exec_subprocess`, or `emit_event`). A
resolver crash, trap, or capability denial MUST NOT prevent hook dispatch. The
dispatcher core MUST have zero compile-time dependency on any per-factory resolver crate.

**SS-01 slice (BC-1.13.001):** Resolver-registry loading and pre-dispatch context
injection contract. The dispatcher loads `resolvers-registry.toml` at startup (same
mtime-cache pattern as `hooks-registry.toml`). Absent `resolvers-registry.toml` is
treated as zero resolvers (not an error). Before each hook dispatch, the dispatcher
calls registered resolvers for entries that declare `needs_context`; resolver outputs
are merged into `plugin_config` before `invoke_plugin`.

**SS-04 slice (BC-4.12.001 through BC-4.12.005):** Resolver lifecycle (load-once,
mtime-cache), ABI (`ResolverInput`/`ResolverOutput`, `RESOLVER_ABI_VERSION = 1`),
capability model (`path_allow` deny-by-default), error/crash isolation semantics
(fail-loud: absent key, `resolver.error` event), and context-injection merging contract
(additive overlay, first-declared-resolver wins on collision with `resolver.merge_collision`
event).

**Factory-agnostic layering:** `factory-dispatcher` is resolver-agnostic.
Per-factory resolver crates (`crates/vsdd-context-resolvers/`) compile to `.wasm` and
are referenced by `resolvers-registry.toml` entries only. The first concrete resolver
— `WaveContextResolver` — reads `.factory/wave-state.yaml` and returns the active
wave's story list as `plugin_config["wave_context"]`. This resolver closes F-P2-001
(convergence hook inert in production) when S-12.08 ships.

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-1.13.001 | Resolver-registry loading and pre-dispatch context injection (SS-01) | P1 |
| BC-4.12.001 | Resolver lifecycle invariant: load at startup, mtime-cache, per-dispatch fresh Store | P1 |
| BC-4.12.002 | Resolver ABI and payload schema: `resolve()` export, `ResolverInput`/`ResolverOutput`, `RESOLVER_ABI_VERSION = 1` | P1 |
| BC-4.12.003 | Resolver capability model: `path_allow` deny-by-default; read-only resolvers; `CapabilityDenied` return | P1 |
| BC-4.12.004 | Resolver error and crash isolation: no propagation to dispatcher; `resolver.error` event; absent key in `plugin_config` | P1 |
| BC-4.12.005 | Context-injection merging contract: additive overlay, registry-order collision resolution, `resolver.merge_collision` event | P1 |

Source BCs: `ss-01/BC-1.13.001.md` (1 BC SS-01 slice), `ss-04/BC-4.12.001.md` through
`ss-04/BC-4.12.005.md` (5 BCs SS-04 slice) — 6 BCs total.
Verification Properties: VP-073 (resolver-load purity — integration), VP-074 (resolver-error
isolation — kani + integration), VP-075 (context-injection determinism — proptest),
VP-076 (resolver-capability confinement — integration).
Decision: ADR-018.

---

### 2.5 Pipeline Orchestration (SS-05)

#### FR-015 — Lobster workflow format (YAML-based DAG with typed step kinds)

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-5.01.001 | A .lobster file is YAML with a single workflow: key | P0 |
| BC-5.01.002 | Workflow defaults: block sets step defaults | P0 |
| BC-5.01.003 | Step type enumeration: skill, agent, gate, loop, human-approval, sub-workflow, parallel, compound | P0 |
| BC-5.01.004 | Step ordering by depends_on topological resolution, NOT array position | P0 |
| BC-5.01.005 | Steps declare condition: for conditional execution | P1 |
| BC-5.01.006 | Failure handling — on_failure: escalate is the default | P0 |
| BC-5.01.007 | loop: blocks bounded; require max_iterations and exit_condition | P0 |
| BC-5.01.008 | human-approval steps declare approval: {prompt, artifacts, timeout} | P0 |
| BC-5.01.009 | agent steps with model_tier: override default agent model assignment | P1 |
| BC-5.01.010 | agent steps declare context: {include: [...], exclude: [...]} for information walls | P0 |
| BC-5.01.011 | Sub-workflow invocation via type: sub-workflow + sub_workflow: filename | P0 |

Source BCs: `ss-05/BC-5.01.001.md` through `BC-5.01.011.md` (11 BCs).
Enforces: CAP-001. Status: **shipped** (workflows present; orchestrator active).

#### FR-016 — Orchestrator agent behavioral contracts

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-5.02.001–013 | Orchestrator: no-write discipline; no self-delegation; no-skip delivery sub-steps; no PR body composition; state-manager last; min timeout 300s; input-hash drift check; absolute paths; workspace resolution; 3-clean-pass adversary convergence; burst splitting; heartbeat read-only; pipeline resume precondition | P0 |

Source BCs: `ss-05/BC-5.02.001.md` through `BC-5.02.013.md` (13 BCs).
Enforces: CAP-001, CAP-005. Status: **shipped** (orchestrator active).

#### FR-017 — Review and quality-gate agents (adversary, spec-reviewer, visual-reviewer)

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-5.04.001 | adversary: cannot see prior adversarial reviews (information wall) | P0 |
| BC-5.04.002 | adversary: every finding tagged with HIGH/MEDIUM/LOW confidence | P0 |
| BC-5.04.003 | adversary: mis-anchoring always blocks convergence | P0 |
| BC-5.04.004 | adversary: min 3 clean passes, max 10 before human escalation | P0 |
| BC-5.04.005 | adversary: max 3 self-validation iterations per pass | P1 |
| BC-5.04.006 | adversary: returns findings as chat text, never writes files | P0 |
| BC-5.04.007 | spec-reviewer: never re-reports adversary findings | P1 |

Source BCs: `ss-05/BC-5.04.001.md` through `BC-5.04.007.md` (7 BCs).
Enforces: CAP-005. Status: **shipped** (adversary active; SHA-currency gate beta.4).

#### FR-018 — Architecture, spec-steward, and consistency-validator agents

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-5.05.001 | architect: every module gets a purity boundary classification | P0 |
| BC-5.05.002 | architect: every VP has viable proof strategy and feasibility note | P0 |
| BC-5.05.003 | architect: ARCH-INDEX must declare deployment_topology | P0 |
| BC-5.05.004 | architect: DTU assessment is mandatory and covers all 6 categories | P0 |
| BC-5.05.005 | architect: VP-INDEX changes propagate in same burst | P0 |
| BC-5.05.006 | architect: VP-locking is 5-step protocol, after which VP is immutable | P0 |
| BC-5.05.007 | consistency-validator: 80 criteria, none skipped | P0 |
| BC-5.05.008 | consistency-validator: index-first validation discipline | P0 |
| BC-5.05.009 | consistency-validator: gate fails when blocking findings exist | P0 |
| BC-5.05.010 | consistency-validator: mis-anchoring is never an Observation | P0 |
| BC-5.05.011–021 | formal-verifier, spec-reviewer, spec-steward, technical-writer behavioral contracts | P0–P1 |

Source BCs: `ss-05/BC-5.05.001.md` through `BC-5.05.021.md` (21 BCs).
Enforces: CAP-004, CAP-018. Status: **shipped** (agents active).

#### FR-019 — Business analyst, product-owner, and story-writer agent contracts

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-5.06.001 | business-analyst: never invents capabilities — must ground in product brief | P0 |
| BC-5.06.002 | business-analyst: produces sharded L2 (L2-INDEX + section files), never monolithic | P0 |
| BC-5.06.003 | business-analyst: include all template sections | P0 |
| BC-5.06.004 | business-analyst: every ASM has a validation method; every R-NNN has a mitigation | P0 |
| BC-5.06.005 | product-owner: BC-S.SS.NNN numbering scheme | P0 |
| BC-5.06.006 | product-owner: BC H1 heading is title source of truth | P0 |
| BC-5.06.007 | product-owner: append-only IDs and slugs | P0 |
| BC-5.06.008 | product-owner: every domain invariant lifted to a BC | P0 |
| BC-5.06.009 | product-owner: same-burst anchor-back when creating BCs | P0 |
| BC-5.06.010 | product-owner: subsystem ID from ARCH-INDEX, never names | P0 |
| BC-5.06.011 | story-writer: one file per story, never monolithic | P0 |
| BC-5.06.012 | story-writer: every AC traces to a BC clause; 6 context-engineering sections mandatory | P0 |
| BC-5.06.013 | story-writer: no story exceeds 13 points or 20-30% agent context window | P0 |
| BC-5.06.014 | story-writer: BC array changes propagate to body and ACs in same atomic commit | P0 |
| BC-5.06.015 | story-writer: dependency graph must be acyclic | P0 |

Source BCs: `ss-05/BC-5.06.001.md` through `BC-5.06.015.md` (15 BCs).
Enforces: CAP-014 (BC decomposition). Status: **shipped** (agents active).

#### FR-020 — Delivery agent contracts (implementer, code-reviewer, test-writer, e2e-tester, security-reviewer)

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-5.07.028 | implementer: never writes code without a failing test (Red Gate) | P0 |
| BC-5.07.029 | implementer: minimum code per test (TDD discipline) | P0 |
| BC-5.07.030 | implementer: micro-commit per passing test, squash before PR | P0 |
| BC-5.07.031 | implementer: respects purity boundary map | P0 |
| BC-5.07.032 | implementer: HALT only on blocker, impossibility, or 3 consecutive failures | P0 |
| BC-5.07.033 | implementer: status in {DONE, DONE_WITH_CONCERNS, NEEDS_CONTEXT, BLOCKED} | P0 |
| BC-5.07.044–049 | test-writer: no implementation code; BC-NNN-traceable naming; Red Gate; no vacuous tests; PBT ≥1000 cases; canonical test vectors | P0 |
| BC-5.07.038–043 | security-reviewer: CWE/CVE citations; 4-tier severity; information wall; no dismissals; supply-chain-ANY-blocks-install; gh pr review (not comment) | P0 |
| BC-5.07.001–023 | code-reviewer, codebase-analyzer, dtu-validator, dx-engineer, e2e-tester, formal-verifier, holdout-evaluator behavioral contracts | P0–P1 |

Source BCs: `ss-05/BC-5.07.001.md` through `BC-5.07.049.md` (49 BCs).
Enforces: CAP-016 (TDD gate). Status: **shipped** (agents active).

#### FR-021 — Infrastructure agents (devops-engineer, PR-manager, state-manager, github-ops)

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-5.08.001–024 | Data-engineer migration discipline; devops-engineer git hygiene; github-ops execute-only; PR-manager 9-step coordinator; PR-reviewer information wall + gh pr review | P0 |
| BC-5.09.001–019 | DTU-validator; research-agent citation discipline; session-reviewer 8-dimensional; validate-extraction behavioral + metrics split | P0 |
| BC-5.10.001–005 | state-manager .factory/ scope; no spec writes; STATE.md 200-line cap; worktree preconditions; wave-gate two-commit protocol | P0 |

Source BCs: `ss-05/BC-5.08.001.md` through `BC-5.10.005.md` (33 BCs).
Status: **shipped** (agents active).

#### FR-022 — Phase workflow DAG contracts (Phase 0–3)

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-5.20.001–020 | phase-0-codebase-ingestion: identity, entry-point, terminal-step, DAG integrity, failure semantics, 7 workflow steps + backups, gate, drift-check, human-approval | P0 |
| BC-5.21.001–019 | phase-1-spec-crystallization: identity through human-approval | P0 |
| BC-5.22.001–019 | phase-2-story-decomposition: identity through human-approval | P0 |
| BC-5.23.001–015 | phase-3-tdd-implementation: identity through record-demos | P0 |

Source BCs: `ss-05/BC-5.20.001.md` through `BC-5.23.015.md` (53 BCs, with remaining phase workflow BCs).
Enforces: CAP-001 (self-orchestrating pipeline). Status: phases 0-1-2-3 are **shipped** as workflows.

#### FR-043 (SS-05 slice) — TDD Discipline Hardening — Pipeline Orchestration layer

Anti-precedent guard, Red Gate density check gate, and facade-mode per-story-delivery semantics (Layer 1, 2, and 3 of FR-043).

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-5.38.001 | stub-architect commit must contain todo!()/unimplemented!() bodies for all non-trivial function implementations | P0 |
| BC-5.38.002 | pure data mappings in stub commits may be implemented inline and must be flagged GREEN-BY-DESIGN | P1 |
| BC-5.38.003 | framework integration wiring may have minimal real code for cargo check; handler business logic must be todo!() | P1 |
| BC-5.38.004 | stub-architect must not use pre-implemented sibling crates as stub templates | P0 |
| BC-5.38.005 | stub-architect applies self-check before committing any non-todo!() function body | P1 |
| BC-5.38.006 | deliver-story SKILL.md and per-story-delivery.md Step 2 must contain anti-precedent guard text verbatim | P0 |
| BC-8.29.001 | RED_RATIO = RED_TESTS / TOTAL_NEW_TESTS must be ≥ 0.5 before Step 4 implementer dispatch (BLOCKING) | P0 |
| BC-8.29.002 | each non-RED test must be documented in red-gate-log with rationale before threshold relaxation | P1 |
| BC-8.29.003 | on RED_RATIO < 0.5 without GREEN-BY-DESIGN justification, orchestrator must choose remediation option A or B | P0 |
| BC-8.30.002 | tdd_mode=facade modifies per-story-delivery semantics and mandates mutation testing at wave gate | P1 |

Note: BC-8.29.001–003 and BC-8.30.002 have BC-ID prefixes historically assigned to SS-08, but are authoritatively SS-05 per subsystem reanchor in adversarial pass-1 review. See individual BC Notes sections for the historical artifact explanation.

Source BCs: `ss-05/BC-5.38.001–006.md` (6 BCs), `ss-08/BC-8.29.001–003.md` (3 BCs, SS-05 reanchored), `ss-08/BC-8.30.002.md` (1 BC, SS-05 reanchored).
Enforces: CAP-016. Status: **pending** (E-7 story S-7.03 not yet implemented).

> Full contracts: `.factory/specs/behavioral-contracts/ss-05/` (646 BCs total + 2 new FR-047 BCs = 648 BCs)
> Demo-recorder, accessibility-auditor, ux-designer agents: BC-5.03.001–018 (18 BCs)

#### FR-047 (SS-05 slice) — Engine Governance: Per-Story Adversary Workflow Gate

**Source CAP:** CAP-005 ("Run adversarial review with information asymmetry")
**Behavioral Contracts:** BC-5.39.001, BC-5.39.002
**Stories:** Story A (v1.0-feature-engine-discipline-pass-1)
**Status:** pending (v1.0-feature-engine-discipline-pass-1)
**Subsystem(s):** SS-05

The per-story adversary workflow gate formalizes the adversarial review loop that runs at
Step 4.5 of the per-story-delivery Lobster workflow (between Step 4 Implement and Step 5 Record demos).
The adversary agent (defined in `agents/adversary.md`) must achieve minimum 3 consecutive clean passes
where `last_classification == "NITPICK_ONLY"` before the demo recording step executes.
Convergence state is persisted in `.factory/cycles/<cycle-id>/<story-id>/adversary-convergence-state.json`
per the schema in BC-5.39.001 Postcondition 2 (OQ3 resolution).

Adversary scope is strictly bounded (BC-5.39.002): the adversary reads only the story worktree diff
vs `develop`, the story spec, and the anchored BCs. Out-of-scope findings (cross-story, integration,
system-level, architectural) are deferred to `deferred_findings` and routed to wave-gate or Phase-5.
Deferred findings do not block per-story convergence (OQ6: adversary agent owns convergence assessment).

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-5.39.001 | Per-story adversarial convergence loop MUST achieve minimum 3 clean passes (NITPICK_ONLY) before demo recording | P0 |
| BC-5.39.002 | Per-story adversary scope MUST be limited to story diff, spec, and anchored BCs; out-of-scope findings MUST be deferred | P0 |

Source BCs: `ss-05/BC-5.39.001.md`, `ss-05/BC-5.39.002.md` — 2 BCs (SS-05 slice of FR-047).
Enforces: CAP-005. Status: **pending** (Story A not yet implemented).

---

### 2.6 Skill Catalog (SS-06)

> SS-06 contains 585 BCs across 119 skills. FRs below group skills by functional
> family. All BCs in `ss-06/` follow the `BC-6.NN.NNN` prefix scheme.

#### FR-023 — Brownfield ingestion skill

| BC Group | Description | Priority |
|----------|-------------|----------|
| BC-6.01.001–NNN | brownfield-ingest: 7-pass multi-phase analysis; convergence tracking; novelty assessment; subsystem coverage audit; extraction validation | P1 |

Enforces: CAP-015. Status: **shipped** (produced this PRD's BC catalog). Approx. 15–20 BCs.

#### FR-024 — Spec crystallization skills (create-brief, create-domain-spec, create-prd, create-architecture)

| BC Group | Description | Priority |
|----------|-------------|----------|
| BC-6.02.001–NNN | create-brief: product brief creation with competitive research | P1 |
| BC-6.03.001–NNN | create-domain-spec: L2 domain spec synthesis (entities, invariants, capabilities, events) | P1 |
| BC-6.04.001–NNN | create-prd: PRD + 4 supplement files; FR-NNN + NFR-NNN | P1 |
| BC-6.05.001–NNN | create-architecture: ARCH-INDEX + 8 section files + 13+ ADRs | P1 |

Enforces: CAP-019, CAP-020. Status: **shipped** (skills active; PRD-this-file produced by create-prd). Approx. 40–60 BCs across the family.

#### FR-025 — Story decomposition and wave-scheduling skills

| BC Group | Description | Priority |
|----------|-------------|----------|
| BC-6.06.001–NNN | decompose-stories: story decomposition with BC traceability and tier assignment | P1 |
| BC-6.07.001–NNN | wave-scheduling: dependency-graph resolution + parallel-tier scheduling | P1 |
| BC-6.08.001–NNN | wave-gate: wave bookkeeping, quality gate enforcement, two-commit protocol | P0 |

Enforces: CAP-006. Status: wave-gate **shipped** (beta.4 state-burst skill); decompose-stories and wave-scheduling **shipped**. Approx. 30–40 BCs.

#### FR-026 — TDD delivery skills (deliver-story, red-gate, green-gate, refactor)

| BC Group | Description | Priority |
|----------|-------------|----------|
| BC-6.09.001–NNN | deliver-story: 9-phase TDD lifecycle; red-gate enforcement; green-gate; refactor + review + demo + merge | P0 |

Enforces: CAP-016. Status: **shipped** (deliver-story active). Approx. 30–40 BCs.

#### FR-027 — Adversarial review skills (adversarial-review, phase-F5-scoped-adversarial, convergence-check)

| BC Group | Description | Priority |
|----------|-------------|----------|
| BC-6.10.001–NNN | adversarial-review: cycle-keyed structure; information asymmetry; SHA-currency gate; 3-clean-pass convergence | P0 |

Enforces: CAP-005. Status: **shipped** (beta.4 SHA-currency gate). Approx. 20–30 BCs.

#### FR-028 — Formal verification skill

| BC Group | Description | Priority |
|----------|-------------|----------|
| BC-6.11.001–NNN | formal-verify: Kani harness generation; proptest; fuzz targets ≥5 minutes; VP catalog; purity boundary audit | P1 |

Enforces: CAP-021. Status: **shipped** (skill present). Approx. 15–20 BCs.

#### FR-029 — Activation and plugin management skills

| BC Group | Description | Priority |
|----------|-------------|----------|
| BC-6.12.001–NNN | activate: platform detection; hooks.json variant copy; binary verification; re-activation drift warning | P0 |
| BC-6.13.001–NNN | deactivate, release, spec-versioning, check-input-drift, maintenance-sweep | P1 |

Enforces: CAP-007, CAP-028. Status: **shipped** (activate active since beta.1). Approx. 20–30 BCs.

#### FR-030 — Factory observability skills (factory-obs, factory-dashboard, factory-health)

| BC Group | Description | Priority |
|----------|-------------|----------|
| BC-6.14.001–NNN | factory-obs, factory-dashboard, factory-health, factory-query, factory-replay: CLI observability over internal logs and sink files | P1 |

Enforces: CAP-003 (observability). Status: **shipped** (bin tools present). Approx. 15–20 BCs.

#### FR-031 — Remaining skill catalog (semport, multi-repo, DTU, holdout, scaffold, UI skills)

| BC Group | Description | Priority |
|----------|-------------|----------|
| BC-6.15–6.99.NNN | semport-analyze, multi-repo-health, dtu-creation, dtu-validate, holdout-eval, scaffold-claude-md, consistency-validation, research, session-review, brainstorming, competitive-monitoring, responsive-validation, UI-quality-gate, storybook, excalidraw, record-demo, policy-registry, and 80+ other skills | P1–P2 |

Enforces: CAP-022–027. Status: shipped (skills present; WIP on ports). Approx. 370 BCs.

#### FR-043 (SS-06 slice) — TDD Discipline Hardening — Mutation Testing Wave Gate

Mutation testing at wave gate for facade-mode stories (Layer 4 of FR-043).

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-6.21.001 | wave-gate skill must run cargo mutants for every story with tdd_mode=facade in the wave | P1 |
| BC-6.21.002 | mutation kill rate floor is 80%; surviving mutants must be addressed via test, dead-code confirmation, or explicit waiver | P1 |

Enforces: CAP-016. Status: **pending** (E-7 story S-7.03 not yet implemented).

> Full contracts: `.factory/specs/behavioral-contracts/ss-06/` (585 BCs total)

---

### 2.7 Hook Bash Layer (SS-07)

#### FR-032 — PreToolUse gate hooks (protect-secrets, destructive-command-guard, branch-guard, BC/VP protection)

| BC Group | Description | Priority |
|----------|-------------|----------|
| BC-7.01.001–NNN | protect-secrets.sh: pattern-based secret detection on Bash+Read; blocks with reason; emits block event | P0 |
| BC-7.02.001–NNN | destructive-command-guard.sh: blocks dangerous bash patterns (rm -rf, git push --force on main) | P0 |
| BC-7.03.001–NNN | protect-bc.sh / protect-vp.sh: PreToolUse Edit or Write gates preventing direct BC/VP modification | P0 |
| BC-7.04.001–NNN | factory-branch-guard.sh, brownfield-discipline.sh, red-gate.sh: factory state machine gates | P0 |

Source BCs: `ss-07/BC-7.01.001.md` through (approx. 50 BCs in PreToolUse family).
Enforces: CAP-008, DI-004. Status: **shipped** (44 hooks active).

#### FR-033 — PostToolUse capture hooks (commit capture, PR activity, audit emission)

| BC Group | Description | Priority |
|----------|-------------|----------|
| BC-7.05.001–NNN | capture-commit-activity.sh (PostToolUse:Bash): extracts commit metadata; emits commit.made event | P0 |
| BC-7.06.001–NNN | capture-pr-activity.sh (PostToolUse:Bash): PR open/merge events | P1 |
| BC-7.07.001–NNN | block-ai-attribution.sh: strips AI attribution from commits | P0 |

Source BCs: `ss-07/BC-7.05.001.md` through (approx. 40 BCs in PostToolUse family).
Enforces: CAP-013. Status: **shipped**.

#### FR-034 — SubagentStop / lifecycle hooks and validate-* gate family

| BC Group | Description | Priority |
|----------|-------------|----------|
| BC-7.08.001–NNN | validate-* family (24 hooks): validate-novelty-assessment, validate-state-pin-freshness, validate-anchor-capabilities-union, validate-wave-gate-completeness, validate-template-compliance, etc. — each is a standalone gate contract | P0 |
| BC-7.09.001–NNN | SubagentStop hooks: validate-state-index-status-coherence, convergence-tracker | P0 |
| BC-7.10.001–NNN | verify-sha-currency.sh (opt-in adversary SHA-currency gate) | P0 |

Source BCs: `ss-07/BC-7.08.001.md` through (approx. 80 BCs in validate/lifecycle family).
Enforces: CAP-004 (BC traceability enforcement). Status: **shipped**. Note: 24 validate-* hooks lack formal BC backfill per L-P0-003 — this is the gap being addressed by this Phase 1.5 PRD effort.

> Full contracts: `.factory/specs/behavioral-contracts/ss-07/` (196 BCs total)

---

### 2.8 Templates and Rules (SS-08)

#### FR-035 — Spec artifact templates (BC, PRD, architecture, ADR, story templates)

| BC Group | Description | Priority |
|----------|-------------|----------|
| BC-8.01.001–NNN | behavioral-contract-template.md: canonical frontmatter; BC-S.SS.NNN heading; required sections (Description, Preconditions, Postconditions, Invariants, Edge Cases, Test Vectors, Verification Properties, Traceability, Related BCs) | P0 |
| BC-8.02.001–NNN | prd-template.md: L3 frontmatter; supplements field; 7 required sections | P0 |
| BC-8.03.001–NNN | story-template.md: 6 context-engineering sections; BC-array frontmatter; AC format | P0 |
| BC-8.04.001–NNN | architecture templates: ARCH-INDEX, SS-NN section, verification-architecture, ADR | P0 |
| BC-8.05.001–NNN | PRD supplement templates: interface-definitions, error-taxonomy, test-vectors, nfr-catalog | P0 |

Source BCs: `ss-08/BC-8.01.001.md` through (approx. 60 BCs in template family).
Enforces: CAP-014. Status: **shipped** (105 template files active).

#### FR-036 — Rules and cross-cutting policy documents (9 rule files)

| BC Group | Description | Priority |
|----------|-------------|----------|
| BC-8.06.001–NNN | Rules files: AGENT-SOUL.md, FACTORY.md, VSDD.md cross-cutting policies; append-only ID policy; BC array propagation handoff policy; VP citation change handoff | P0 |

Source BCs: `ss-08/BC-8.06.001.md` through (approx. 70 BCs in rules family).
Enforces: CAP-014 (methodology discipline). Status: **shipped**.

<!-- F-008 (Wave 8 pass-1): docs-stories S-0.05/S-5.05/S-5.06 anchored to CAP-014 via SS-08 BCs (BC-8.22.001 Conventional Commits, BC-8.26.001 story-completeness audit, BC-8.26.006 user-facing-docs deliverable) — methodology-anchor pattern. 6 v1.1 BC candidates BC-8.31.003-008 registered in story v1.1 candidate tables (S-0.05 + S-5.05 + S-5.06; story-table-only per Wave 8 SS-08 re-anchor pattern; no on-disk BC files yet — promotion to v1.0 BCs deferred to v1.0.1). BC-8.31.001-002 not in use; gap-numbered for future expansion. (Updated Wave 14 pass-2: BC-8.31.008 added by S-5.05 v1.5; count corrected from 7 to 6.) -->

#### FR-043 (SS-08 slice) — TDD Discipline Hardening — Story Template tdd_mode field

Story template contract for the `tdd_mode` frontmatter field (Layer 3 template artifact of FR-043). This is the only FR-043 BC that remains in SS-08 after adversarial pass-1 reanchor (BC-8.30.002 moved to SS-05; BC-8.29.001–003 moved to SS-05).

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-8.30.001 | story template must include tdd_mode field with strict\|facade enum and strict default | P1 |

Enforces: CAP-016. Status: **pending** (E-7 story S-7.03 not yet implemented).

> Full contracts: `.factory/specs/behavioral-contracts/ss-08/` (218 BCs total)

---

### 2.9 Configuration and Activation (SS-09)

#### FR-037 — Platform-aware activation and hooks.json variant management

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-9.01.001 | bump-version.sh accepts semver release format (stable N.N.N + prerelease 1.0.0-beta.N / 1.0.0-rc.N) | P0 |
| BC-9.01.002 | chore commit (operator-staged) modifies only CHANGELOG.md | P0 |
| BC-9.01.003 | release workflow's bot commit atomically writes binaries + plugin.json + marketplace.json | P0 |
| BC-9.01.004 | 5-platform CI matrix is the build matrix; drift gated by check-platforms-drift.py | P0 |
| BC-9.01.005 | hooks.json is gitignored; hooks.json.template + per-platform variants are committed | P0 |

Source BCs: `ss-09/BC-9.01.001.md` through `BC-9.01.005.md` (5 BCs total).
Enforces: DI-015 (activation gate), CAP-007 (BC-9.01.004/005), CAP-028 (BC-9.01.001-003 — release-tooling discipline). Status: **all 5 BCs anchored to capabilities (BC frontmatter status:draft pending Phase 1.6b verification)** as of Wave 6 SS-09 re-anchor; Wave 7 SS-10 cross-wave anchors added. BC-9.01.004/005 → CAP-007: S-0.03, S-0.04, S-2.02, S-2.03, S-2.06, S-2.08. BC-9.01.001-003 → CAP-028: S-0.01, S-0.02, S-2.04, S-2.08, S-4.08, S-5.07.
<!-- Wave 7 SS-10 cross-wave anchor: BC-9.01.001 anchored to S-0.02 (Release.yml prerelease detection is the Release-side complement to bump-version.sh tag format), S-4.08 (rc.1 release gate invokes bump-version.sh 1.0.0-rc.1), S-5.07 (1.0.0 release gate invokes bump-version.sh 1.0.0). BC-9.01.003 anchored to S-0.02 (the Release.yml workflow triggers the bot commit contracted by BC-9.01.003). -->

> **Scope note (Wave 5 pass-1 CRIT-001 fix):** FR-037 covers Platform-aware activation hooks.json variant management AND release-tooling discipline (version bump, chore commit cleanliness, release-bot atomicity). The 5 BCs split across: activation-gate prerequisites (BC-9.01.004, BC-9.01.005) and release-pipeline discipline (BC-9.01.001, BC-9.01.002, BC-9.01.003). The activation-gate narrative in §FR-037 heading still holds — BC-9.01.004/005 are the gate-readiness contracts; BC-9.01.001-003 are release-tooling discipline contracts that govern how gate-ready artifacts are produced.

> Full contracts: `.factory/specs/behavioral-contracts/ss-09/` (5 BCs total)

---

### 2.10 CLI Tools and Bin (SS-10)

#### FR-038 — Event emission CLI tool (bin/emit-event)

| BC Group | Description | Priority |
|----------|-------------|----------|
| BC-10.01.001–NNN | emit-event: normalizes bash-hook event emission; writes to internal log and configured sinks; stdin-driven; exit-code passthrough | P0 |

Source BCs: `ss-10/BC-10.01.001.md` through (approx. 10 BCs).
Enforces: CAP-027. Status: **partial** (shell tool present; host-fn refactor S-3.04 partial, DRIFT on refactor).

#### FR-039 — Factory observability bin tools (factory-dashboard, factory-obs, factory-query, factory-replay, factory-report, factory-sla)

| BC Group | Description | Priority |
|----------|-------------|----------|
| BC-10.02.001–NNN | factory-{dashboard,obs,query,replay,report,sla}: CLI tools over dispatcher-internal-*.jsonl and factory-events-*.jsonl; query by trace-id, event type, date range; output as table or JSON | P1 |

Source BCs: `ss-10/BC-10.02.001.md` through (approx. 30 BCs in obs-bin family).
Enforces: CAP-003, CAP-010. Status: **shipped** (bin tools present).

#### FR-040 — Workflow infrastructure CLI tools (wave-state, lobster-parse, compute-input-hash)

| BC Group | Description | Priority |
|----------|-------------|----------|
| BC-10.03.001–NNN | wave-state: reads/writes STATE.md wave bookkeeping; lobster-parse: validates workflow files; compute-input-hash: generates and updates frontmatter input-hash fields | P0 |

Source BCs: `ss-10/BC-10.03.001.md` through (approx. 18 BCs in workflow-infra family).
Status: **shipped**.

> Full contracts: `.factory/specs/behavioral-contracts/ss-10/` (58 BCs total)

#### FR-041 — Skill-driven ADR authoring workflow

The `create-adr` skill (`/vsdd-factory:create-adr`) provides a collision-free, validated, atomic workflow for creating Architecture Decision Records. It allocates the next sequential ADR-NNN by dual-source scan (filesystem + ARCH-INDEX), scaffolds the ADR file from `adr-template.md` with correct frontmatter and verbatim placeholder section bodies, optionally applies bidirectional supersession patches, inserts an ARCH-INDEX row, annotates the Source/Origin section for brownfield context, and runs `validate-template-compliance.sh` as a final gate. All side-effects are atomic: any failure rolls back the repository to its pre-invocation state. This skill eliminates the manual ADR authoring process that was re-exposed during the 10-ADR brownfield backfill burst.

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-6.20.001 | create-adr allocates next sequential ADR-NNN by scanning filesystem and ARCH-INDEX | P1 |
| BC-6.20.002 | create-adr refuses explicit --id override that already exists | P1 |
| BC-6.20.003 | create-adr blocks on filesystem-vs-ARCH-INDEX ID mismatch | P1 |
| BC-6.20.004 | create-adr writes frontmatter with status=proposed (always at creation) | P1 |
| BC-6.20.005 | create-adr validates subsystems_affected against ARCH-INDEX Subsystem Registry | P1 |
| BC-6.20.006 | create-adr validates --supersedes ADR-NNN exists before proceeding | P1 |
| BC-6.20.007 | create-adr bidirectionally patches old ADR's superseded_by on supersession | P1 |
| BC-6.20.008 | create-adr inserts ARCH-INDEX row in numeric order, pipe-aligned | P1 |
| BC-6.20.009 | create-adr scaffolds placeholder section bodies verbatim from template (no ghost-writing) | P1 |
| BC-6.20.010 | create-adr annotates Source/Origin section under --brownfield or implicit-brownfield | P1 |
| BC-6.20.011 | create-adr runs validate-template-compliance.sh as final gate, blocks on non-zero | P1 |
| BC-6.20.012 | create-adr is atomic — any partial-state failure rolls back all side-effects | P0 |

Source BCs: `ss-06/BC-6.20.001.md` through `ss-06/BC-6.20.012.md` (12 BCs).
Maps to: SS-06 (Skill Catalog), SS-08 (Templates and Rules), SS-10 (CLI Tools and Bin).
Acceptance: S-6.01 (all 8 ACs satisfied).
Status: **pending** (S-6.01 not yet implemented).

> Full contracts: `.factory/specs/behavioral-contracts/ss-06/BC-6.20.*.md`

#### FR-042 — Process self-improvement enforcement

The vsdd-factory pipeline enforces lessons learned from adversarial reviews at the structural level: agent prompts include mandatory spec-first gates and capability anchor justification requirements; a new lint hook detects count drift across index files; a meta-rule requires codification follow-up for every novel process gap; and the validate-template-compliance script enforces VP structural conventions. These controls prevent the class of findings that generated 8+ adversarial passes in S-6.01 (empty BCs, missing anchor justification, incomplete propagation) from recurring in future cycles.

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-5.36.001 | story-writer agent rejects status=ready when behavioral_contracts is empty | P0 |
| BC-5.36.002 | story-writer requires AC↔BC bidirectional traces before marking a story ready | P0 |
| BC-5.36.003 | product-owner agent requires Capability Anchor Justification cell on every BC | P1 |
| BC-5.36.004 | product-owner cites capabilities.md verbatim in every capability anchor justification | P1 |
| BC-5.36.005 | adversary explicitly checks partial-fix-regression for every finding closed in a prior pass | P1 |
| BC-5.36.006 | adversary checks fix propagation to bodies, sibling files, and prose — not just frontmatter | P1 |
| BC-5.36.007 | all three agent prompts updated atomically in single delivery; no partial update | P1 |
| BC-5.37.001 | state-manager runs corpus-wide grep before declaring count change complete | P1 |
| BC-5.37.002 | state-manager logs sweep results before declaring count-change complete | P1 |
| BC-7.05.001 | validate-count-propagation.sh detects count drift across index files and exits non-zero | P1 |
| BC-7.05.002 | validate-count-propagation.sh runs in under 200ms and is deterministic | P1 |
| BC-7.05.003 | validate-template-compliance.sh enforces VP multi-BC source_bc convention | P1 |
| BC-8.28.001 | rules/lessons-codification.md requires codification follow-up for every novel process catch | P1 |
| BC-8.28.002 | orchestrator cycle-closing checklist references lessons-codification.md rule | P1 |
| BC-7.05.004 | hooks-registry.toml registers validate-count-propagation.sh as PostToolUse on index file writes | P1 |

Source BCs: `ss-05/BC-5.36.001-007.md`, `ss-05/BC-5.37.001-002.md`, `ss-07/BC-7.05.001-004.md`, `ss-08/BC-8.28.001-002.md` (15 BCs total).
Maps to: SS-05 (Pipeline Orchestration), SS-07 (Hook Bash Layer), SS-08 (Templates and Rules).
Acceptance: S-7.01 (5 ACs), S-7.02 (6 ACs).
Status: **pending** (E-7 stories not yet implemented).

> Full contracts: `.factory/specs/behavioral-contracts/ss-05/BC-5.36.*.md`, `ss-05/BC-5.37.*.md`, `ss-07/BC-7.05.*.md`, `ss-08/BC-8.28.*.md`

#### FR-043 — TDD Discipline Hardening — Prevent Stub-as-Implementation Anti-Pattern

A real downstream factory run (Prism project, Wave 2, parallel batch of 5 stories) revealed that 3 of 5 stub-architect agents pre-implemented business logic instead of writing `todo!()` bodies. By Step 3 (Red Gate), most tests already passed — meaning no genuine Red→Green TDD cycle occurred and Step 4 (implementer dispatch) became a no-op. The root cause was a precedent cascade: earlier merged DTU clones contained stubs with full implementations, and Wave 2 stub-architects used those crates as templates and reproduced the violation. This FR codifies four hardening layers to prevent recurrence:

**Layer 1 — Anti-precedent guard (SS-05 Pipeline Orchestration):** The deliver-story SKILL.md and per-story-delivery.md Step 2 must contain verbatim anti-precedent guard text (with SHA citations aa706543, 6d2d005e, 20b4a12a as anti-patterns and e86d03f2 as model precedent). Stub-architect must use `todo!()` for all non-trivial function bodies; only pure-data mappings (GREEN-BY-DESIGN) and framework wiring minimums (WIRING-EXEMPT) are exempt, and both must be flagged in the stub report.

**Layer 2 — Red Gate density check (SS-05 Pipeline Orchestration):** A BLOCKING gate between Step 3 and Step 4 enforces RED_RATIO = RED_TESTS / TOTAL_EFFECTIVE_TESTS ≥ 0.5. If the gate fires with unjustified GREEN tests, the orchestrator must choose: (A) roll back stub and re-dispatch with stricter prompt, or (B) accept with mandatory mutation testing at wave gate plus PR disclosure.

**Layer 3 — tdd_mode frontmatter contract (SS-08 Templates and Rules for story-template field; SS-05 Pipeline Orchestration for facade-mode delivery semantics):** The story template gains `tdd_mode: strict|facade` (default: strict). Facade mode allows scaffold+impl combined for third-party API clone / structural facade work, bypassing the stub discipline and Red Gate density check while mandating mutation testing at wave gate as the compensating control.

**Layer 4 — Mutation testing wave-gate (SS-06 Skill Catalog):** The wave-gate skill must run `cargo mutants -p <crate> --jobs N --timeout 300` for any story with `tdd_mode: facade` (or `mutation_testing_required: true` from Layer 2 Option B). Kill rate floor is 80%. Surviving mutants must be addressed via new test (A), dead-code-equivalent confirmation (B), or explicit waiver (C).

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-5.38.001 | stub-architect commit must contain todo!()/unimplemented!() bodies for all non-trivial function implementations | P0 |
| BC-5.38.002 | pure data mappings in stub commits may be implemented inline and must be flagged GREEN-BY-DESIGN | P1 |
| BC-5.38.003 | framework integration wiring may have minimal real code for cargo check; handler business logic must be todo!() | P1 |
| BC-5.38.004 | stub-architect must not use pre-implemented sibling crates as stub templates | P0 |
| BC-5.38.005 | stub-architect applies self-check before committing any non-todo!() function body | P1 |
| BC-5.38.006 | deliver-story SKILL.md and per-story-delivery.md Step 2 must contain anti-precedent guard text verbatim | P0 |
| BC-8.29.001 | RED_RATIO = RED_TESTS / TOTAL_NEW_TESTS must be ≥ 0.5 before Step 4 implementer dispatch (BLOCKING) | P0 |
| BC-8.29.002 | each non-RED test must be documented in red-gate-log with rationale before threshold relaxation | P1 |
| BC-8.29.003 | on RED_RATIO < 0.5 without GREEN-BY-DESIGN justification, orchestrator must choose remediation option A or B | P0 |
| BC-8.30.001 | story template must include tdd_mode field with strict or facade enum and strict default | P1 |
| BC-8.30.002 | tdd_mode=facade modifies per-story-delivery semantics and mandates mutation testing at wave gate | P1 |
| BC-6.21.001 | wave-gate skill must run cargo mutants for every story with tdd_mode=facade in the wave | P1 |
| BC-6.21.002 | mutation kill rate floor is 80%; surviving mutants must be addressed via test, dead-code confirmation, or explicit waiver | P1 |

Source BCs: `ss-05/BC-5.38.001–006.md` (6 BCs), `ss-08/BC-8.29.001–003.md` (3 BCs, SS-05 reanchored), `ss-08/BC-8.30.001.md` (1 BC, SS-08), `ss-08/BC-8.30.002.md` (1 BC, SS-05 reanchored), `ss-06/BC-6.21.001–002.md` (2 BCs) — 13 BCs total.
Maps to: SS-05 (Pipeline Orchestration), SS-06 (Skill Catalog), SS-08 (Templates and Rules — BC-8.30.001 only).
Verification Properties: VP-063 (RED_RATIO computation correctness — integration/bats), VP-064 (facade-mode mutation gate enforcement — manual).
Acceptance: S-7.03 (TDD Discipline Hardening).
Status: **pending** (E-7 story not yet implemented).

> Full contracts: `.factory/specs/behavioral-contracts/ss-05/BC-5.38.*.md`, `ss-06/BC-6.21.*.md`, `ss-08/BC-8.29.*.md`, `ss-08/BC-8.30.*.md`

#### FR-047 (SS-06 slice) — Engine Governance: Artifact Path Governance — relocate-artifact Skill

**Source CAP:** CAP-018 ("Validate spec consistency across all artifact layers")
**Behavioral Contracts:** BC-6.22.001
**Stories:** Story C (v1.0-feature-engine-discipline-pass-1)
**Status:** pending (v1.0-feature-engine-discipline-pass-1)
**Subsystem(s):** SS-06

The `relocate-artifact` skill (`/vsdd-factory:relocate-artifact`) is a new skill that
enforces artifact path consistency across `.factory/`. It reads `plugins/vsdd-factory/config/artifact-path-registry.yaml`
as the single source of truth and detects files whose current paths violate canonical patterns.

Default mode is dry-run (reports violations, no filesystem changes). The `--apply` flag executes
`git mv` and updates cross-references. The skill MUST run to zero violations before
`validate-artifact-path` is registered in `hooks-registry.toml` (OQ5 hard delivery prerequisite).

The 9 creation skills (`create-adr`, `create-architecture`, `create-brief`, `create-domain-spec`,
`create-excalidraw`, `create-prd`, `create-story`, `register-artifact`, `conform-to-template`) also
receive a registry-read step as part of this FR's Story C scope: each skill reads the registry at
the start of its procedure and resolves target paths before calling `Write`.

| BC ID | Title | Priority |
|-------|-------|----------|
| BC-6.22.001 | relocate-artifact skill MUST scan .factory/ for registry violations, propose canonical destinations, perform git mv, and update cross-references; MUST run to zero violations before validate-artifact-path hook is registered | P0 |

Source BCs: `ss-06/BC-6.22.001.md` — 1 BC (SS-06 slice of FR-047).
Verification Properties: VP-072 (single-source-of-truth invariant — no embedded path lists in skill or hook source).
Enforces: CAP-018. Status: **pending** (Story C not yet implemented).

> Full contracts: `.factory/specs/behavioral-contracts/ss-06/BC-6.22.001.md`

---

## 3. Interface Definition

> **Supplement:** Full interface definitions are in
> `.factory/specs/prd-supplements/interface-definitions.md`.

### 3.1 Primary Interfaces Summary

**Dispatcher invocation interface (SS-01)**
- Binary: `factory-dispatcher[.exe]` invoked by Claude Code via `hooks.json`
- Input: JSON envelope on stdin (Claude Code hook payload format)
- Output: exit code (0 = continue, 2 = block), JSON summary on stdout
- Config files: `hooks-registry.toml` (plugin routing), `observability-config.toml` (sinks)
- Schema version: `schema_version = 2` on `hooks-registry.toml` (post-ADR-019) and `observability-config.toml` (post-ADR-015); mismatch = hard error

**Plugin ABI (SS-02)**
- Module target: `wasm32-wasip1`
- Entry point: `__hook_entry(ptr: i32, len: i32) -> i32`
- Host import module: `vsdd` namespace
- ABI version constant: `HOST_ABI_VERSION = 1`
- Payload format: JSON (HookPayload struct) + plugin_config injection

**Skill invocation interface (SS-06, SS-10)**
- Claude Code slash command: `/vsdd-factory:<skill-name> [args]`
- All 110 slash commands map 1:1 to skills in `plugins/vsdd-factory/skills/`
- Arguments passed as plain text; skills parse internally

**Observability output interface (SS-03, SS-10)**
- Always-on log: `${CLAUDE_PROJECT_DIR}/.factory/logs/dispatcher-internal-YYYY-MM-DD.jsonl`
- Configured file sink: `${CLAUDE_PROJECT_DIR}/.factory/logs/factory-events-YYYY-MM-DD.jsonl` (default path)
- OTel endpoint: `http://localhost:4317` (default gRPC)
- Event schema version: `INTERNAL_EVENT_SCHEMA_VERSION = 1`

See `.factory/specs/prd-supplements/interface-definitions.md` for complete CLI help
text, exit code table, JSON schema, config schema, and flag interaction rules.

---

## 4. Non-Functional Requirements

> **Supplement:** Full NFR catalog (76 items) is in
> `.factory/specs/prd-supplements/nfr-catalog.md`.
> Source: `.factory/phase-0-ingestion/pass-4-nfr-catalog.md`.

### 4.1 NFR Summary by Category

| Category | Count | Top NFRs |
|----------|-------|----------|
| Performance | 16 | Epoch tick ≤10ms; default timeout 5,000ms; fuel cap 10,000,000; sink queue depth 1,000; STDERR_CAP_BYTES = 4,096; OTLP batch size 100 |
| Security | 18 | Capability deny-by-default; shell-bypass-acknowledged required; setuid refused categorically; env_allow per-plugin; WASI p1 sandbox; adversary SHA-currency gate |
| Observability | 16 | Always-on self-telemetry; 30-day retention; daily rotation by event timestamp; 17 event-type constants; dispatcher_trace_id propagated on every event; multi-sink fan-out |
| Reliability | 15 | Non-blocking errors (exit 0 on registry/payload/engine error); plugin failure isolated; sink failure isolated; cooperative shutdown; atomic binary release |
| Scalability | 6 | Parallel-within-tier sequential-between; single-shot dispatcher process; ~3GB binary ceiling over 3 years |
| Maintainability | 8 | Workspace dependency pinning; Edition 2024 + rust-version 1.95; schema versioning at every config boundary; deny_unknown_fields universal; 185 Rust tests + ~1,262 bats |
| Compatibility | 5 | 5-platform support; HOST_ABI_VERSION = 1 frozen at v1.0; SDK accepts both event_name and hook_event_name; cross-host re-activation drift warning |
| Auditability | 4 | Every capability denial recorded; dispatcher_trace_id; durable audit trail (JSONL); bin/emit-event normalization |
| **Total** | **76** | |

### 4.2 Top 5 Priority NFRs (must hold for v1.0 ship)

| NFR-ID | Category | Requirement | Target |
|--------|----------|-------------|--------|
| NFR-SEC-001 | Security | Capability deny-by-default | All host fns return CAPABILITY_DENIED (-1) when no cap block configured |
| NFR-OBS-001 | Observability | Always-on dispatcher self-telemetry | `dispatcher-internal-*.jsonl` written on every invocation regardless of sink config |
| NFR-REL-001 | Reliability | Non-blocking errors | Exit 0 on registry/payload/engine errors; misconfiguration never blocks user's tool call |
| NFR-MAINT-004 | Maintainability | Schema versioning at every config boundary | REGISTRY_SCHEMA_VERSION = 2 (post-ADR-019), INTERNAL_EVENT_SCHEMA_VERSION = 1, HOST_ABI_VERSION = 1; mismatch = hard error |
| NFR-COMPAT-002 | Compatibility | HOST_ABI_VERSION = 1 frozen at v1.0 | Breaking ABI change requires major version bump on both dispatcher AND SDK |

See `.factory/specs/prd-supplements/nfr-catalog.md` for the complete 76-NFR catalog with numerical targets and validation methods.

---

## 5. Error Taxonomy

> **Supplement:** Full error taxonomy is in
> `.factory/specs/prd-supplements/error-taxonomy.md`.

### 5.1 Error Category Summary

| Category | Prefix | Severity | Exit Code | Examples |
|----------|--------|----------|-----------|---------|
| Registry errors | E-REG-NNN | broken → exit 0 | 0 | E-REG-001: schema_version mismatch; E-REG-002: invalid tool regex; E-REG-003: unknown fields |
| Payload errors | E-PAY-NNN | broken → exit 0 | 0 | E-PAY-001: missing event_name; E-PAY-002: invalid JSON |
| Capability denial | E-CAP-NNN | blocked → exit 2 | 2 | E-CAP-001: exec_subprocess denied; E-CAP-002: shell-bypass not acknowledged; E-CAP-003: setuid refused |
| Plugin execution | E-PLG-NNN | degraded → exit 0 | 0 | E-PLG-001: timeout (epoch); E-PLG-002: timeout (fuel); E-PLG-003: crash (trap) |
| Sink errors | E-SNK-NNN | degraded → non-blocking | 0 | E-SNK-001: queue full (event dropped); E-SNK-002: write failure; E-SNK-003: unknown driver type |
| Activation errors | E-ACT-NNN | broken → explicit error | non-zero | E-ACT-001: unsupported platform; E-ACT-002: binary missing; E-ACT-003: hooks.json write failure |
| Hook gate blocks | E-HK-NNN | blocked → exit 2 | 2 | E-HK-001: secret detected; E-HK-002: destructive command; E-HK-003: branch protection |
| Migration binary exit codes | Named codes (EXPIRY_ABORT, etc.) | non-error sentinel (exit 0/1) or blocked (exit 2) | 0, 1, or 2 | EXPIRY_ABORT: manifest expired/absent/null-generation sub-state (exit 1); CENSUS_MISMATCH_ABORT: pre-pivot census failure (exit 2); ALREADY_MIGRATED: idempotent sentinel (exit 0) |
| Migration window guards | E-MAINTENANCE-NNN | blocked → HookResult (native admission gate); exit 2 | 2 | E-MAINTENANCE-001: BC-INDEX write blocked — txn record in STAGING or COMMITTING state (ADR-052 §Decision 5a) |

All dispatcher-level errors except E-CAP and E-PLG exit 0 (non-blocking per NFR-REL-001).

See `.factory/specs/prd-supplements/error-taxonomy.md` for the complete error catalog.

---

## 5b. Test Vectors

> **Supplement:** Canonical test vectors are in
> `.factory/specs/prd-supplements/test-vectors.md`.

Key test vector families (each grounded in existing Rust unit/integration tests):

- **Registry parsing vectors** (BC-1.01): minimal registry round-trip, schema version mismatch, unknown fields, capabilities block, per-entry priority override, load-not-found
- **Plugin execution vectors** (BC-1.03): timeout-via-epoch, fuel-exhaustion, crash-trap, crash-does-not-affect-siblings, multi-tier-order
- **Host function vectors** (BC-1.05): exec_subprocess deny-no-cap, deny-not-on-allow-list, deny-shell-no-bypass, deny-setuid, allow-with-bypass; env deny-not-on-allow, return-0-unset; emit_event reserved-field-rejected, decode round-trips
- **Internal log vectors** (BC-1.06): 10-event JSONL round-trip; startup-flow 4-event; write-is-best-effort-on-bad-path; prune-removes-only-dispatcher-files
- **Sink vectors** (BC-3.02, BC-3.03): template substitution variants; JSONL append 3-events; routing filter allow-list; tag enrichment no-overwrite; OTLP 10-event attribute mapping

See `.factory/specs/prd-supplements/test-vectors.md` for tables with explicit inputs, expected outputs, and reference Rust test names.

---

## 6. Competitive Differentiator Traceability

> Maps each KD from Section 1.3 to the BCs that implement it.

### 6.1 KD-001 — Self-Orchestrating LLM Pipeline

| BC Group | Contribution |
|----------|-------------|
| BC-5.01.001–011 | Lobster workflow format defines the DAG that the orchestrator reads to drive phases |
| BC-5.02.001–013 | Orchestrator behavioral constraints ensure autonomous dispatch without human intervention per-step |
| BC-5.20.001–020 | Phase-0 workflow delivers brownfield ingestion from a single entry-point |
| BC-5.21.001–019 | Phase-1 workflow delivers spec crystallization autonomously |
| BC-5.22.001–019 | Phase-2 delivers story decomposition and wave scheduling |
| BC-5.23.001+ | Phase-3 delivers TDD implementation with red-gate enforcement |
| BC-6.09.001+ | deliver-story skill enforces the full 9-phase TDD lifecycle |

### 6.2 KD-002 — Sandbox-Aware WASM Plugin Execution

| BC Group | Contribution |
|----------|-------------|
| BC-1.03.001–002 | Epoch + fuel limits enforce bounded execution — no runaway plugins |
| BC-1.05.001–004 | Deny-by-default capability gates: exec_subprocess, shell-bypass, setuid refusal |
| BC-2.01.001–004 | Plugin ABI types enforce typed output (Continue/Block/Error) |
| BC-2.02.001–002 | SDK API design requires explicit bounds on every blocking call |
| BC-4.01.001–006 | Legacy-bash-adapter provides backward compat without bypassing WASM sandbox |

### 6.3 KD-003 — Always-On Multi-Sink Observability

| BC Group | Contribution |
|----------|-------------|
| BC-1.06.001–010 | Internal log always written regardless of sink config |
| BC-3.01.001–006 | SinkRegistry multi-sink fan-out; RoutingFilter; unknown-driver graceful skip |
| BC-3.02.001–016 | File sink with daily rotation, tag enrichment, path templates |
| BC-3.03.001–013 | OTel/gRPC sink with batch delivery and self-healing reconnect |

### 6.4 KD-004 — Cross-Platform Native Dispatcher

| BC Group | Contribution |
|----------|-------------|
| BC-9.01.001–005 | Activation writes correct platform binary path; CI generates 5 variants |
| BC-1.08.004–005 | Dispatcher uses CLAUDE_PROJECT_DIR for cwd; CLAUDE_PLUGIN_ROOT for plugin root |

### 6.5 KD-005 — Strict-Binary Novelty Assessment

| BC Group | Contribution |
|----------|-------------|
| BC-5.07.007–008 | codebase-analyzer enforces binary novelty (SUBSTANTIVE / NITPICK); min 2 rounds, max 5 |
| BC-5.04.001–006 | adversary: min 3 clean passes; information wall; mis-anchoring always blocks |
| BC-7.08–7.09 | validate-novelty-assessment.sh enforces the binary format at SubagentStop |

### 6.6 KD-006 — Brownfield Ingest as First-Class Workflow

| BC Group | Contribution |
|----------|-------------|
| BC-5.20.001–020 | phase-0-codebase-ingestion workflow is the entry-point for brownfield projects |
| BC-6.01.001+ | brownfield-ingest skill implements the 7-pass multi-phase analysis |
| BC-5.09.014–019 | validate-extraction agent enforces 97%+ confirmation rate threshold |

---

## 7. Requirements Traceability Matrix

> This matrix provides FR-level traceability. Each row summarizes one FR group.
> For BC-level granularity, load the individual BC files in `ss-NN/`.

| FR ID | Title | CAPs | Subsystem(s) | BC Prefix / Range | Approx BC Count | Status | Epic |
|-------|-------|------|-------------|-------------------|-----------------|--------|------|
| FR-001 | Registry loading and schema validation | CAP-002 | SS-01 | BC-1.01.001–015 | 15 | shipped | E-1 |
| FR-002 | Hook payload parsing and event routing | CAP-002 | SS-01 | BC-1.02.001–005 | 5 | shipped | E-1 |
| FR-003 | Plugin execution lifecycle (epoch/fuel/crash/block) | CAP-002, CAP-011 | SS-01 | BC-1.03.001–016 | 16 | shipped | E-1 |
| FR-004 | Engine construction (epoch ticker, fuel) | CAP-011 | SS-01 | BC-1.04.001–003 | 3 | shipped | E-1 |
| FR-005 | Host function surface (cap-gated exec/env/read_file/log/emit) | CAP-002, CAP-008 | SS-01 | BC-1.05.001–034 | 34 | shipped (CAP-008 deny gates); partial (CAP-002 read_file per DRIFT-001) | E-1 |
| FR-006 | Internal log (always-on self-telemetry) | CAP-010 | SS-01, SS-03 | BC-1.06.001–010 | 10 | shipped | E-1 |
| FR-007 | Legacy hook routing compatibility + dispatcher main | CAP-002, CAP-008 | SS-01, SS-04 | BC-1.07.001–BC-1.08.006 | 10 | shipped | E-2 |<!-- Wave 9: S-2.07 anchors BC-1.07.001/002+BC-1.08.001/002 (regression gate) --><!-- F-004 (Wave 9 pass-1): SS-07 omission from FR-007 Subsystem(s) is intentional. S-2.07 declares subsystems:[SS-01, SS-04, SS-07] because BC-1.07.002 (Architecture Module includes SS-07 bash-layer) is anchored to S-2.07 — but SS-07 touch is via cross-subsystem regression coverage (bats tests invoke SS-07 hooks transitively via legacy-bash-adapter), NOT primary FR-007 implementation. Per Wave 7 F-002 secondary-subsystem disclosure pattern. -->
| FR-008 | Plugin cache (mtime-driven, process-lifetime) | CAP-002 | SS-01 | BC-1.09.001–004 | 4 | shipped | E-1 |
| FR-009 | Plugin ABI types and host function SDK | CAP-009 | SS-02 | BC-2.01.001–BC-2.05.003 | 22 | shipped/partial | E-1 |
| FR-010 | Sink registry, routing filter, config loading | CAP-003 | SS-03 | BC-3.01.001–005 + BC-3.06.001–006 | 11 | shipped | E-1 |
| FR-011 | File sink (JSONL, daily rotation, tags, routing) | CAP-003 | SS-03 | BC-3.01.006–008 + BC-3.02.001–016 | 19 | shipped | E-1 |
| FR-012 | OTel gRPC sink (OTLP/gRPC batch, self-healing) | CAP-003 | SS-03 | BC-3.01.009 + BC-3.03.001–BC-3.05.003 | 18 | shipped | E-1 |
| FR-013 | Legacy bash adapter plugin | CAP-008, CAP-013 | SS-04 | BC-4.01.001–BC-4.02.006 | 12 | shipped | E-2 |
| FR-014 | Capture-commit-activity plugin (PostToolUse) | CAP-013 | SS-04 | BC-4.03.001 | 1 | pending | E-3 |
| FR-015 | Lobster workflow format (DAG, typed step kinds) | CAP-001 | SS-05 | BC-5.01.001–011 | 11 | shipped | E-0, E-1 |
| FR-016 | Orchestrator agent behavioral contracts | CAP-001, CAP-005 | SS-05 | BC-5.02.001–013 | 13 | shipped | E-1 |
| FR-017 | Review and quality-gate agents (adversary, spec-reviewer) | CAP-005 | SS-05 | BC-5.04.001–007 | 7 | shipped | E-1 |
| FR-018 | Architecture, spec-steward, consistency-validator agents | CAP-004, CAP-018 | SS-05 | BC-5.05.001–021 | 21 | shipped | E-1 |
| FR-019 | Business analyst, product-owner, story-writer contracts | CAP-014 | SS-05 | BC-5.06.001–015 | 15 | shipped | E-1 |
| FR-020 | Delivery agent contracts (implementer, test-writer, security-reviewer) | CAP-016 | SS-05 | BC-5.07.001–049 | 49 | shipped | E-1 |
| FR-021 | Infrastructure agents (devops, PR-manager, state-manager, github-ops) | CAP-001 | SS-05 | BC-5.08.001–BC-5.10.005 | 33 | shipped | E-1 |
| FR-022 | Phase workflow DAG contracts (Phase 0–3) | CAP-001 | SS-05 | BC-5.20.001–BC-5.23.NNN | ~80 | shipped | E-0, E-1 |
| FR-023 | Brownfield ingestion skill | CAP-015 | SS-06 | BC-6.01.001+ | ~20 | shipped | E-0 |
| FR-024 | Spec crystallization skills (brief/domain-spec/PRD/arch) | CAP-019, CAP-020 | SS-06, SS-08 | BC-6.02–6.05.NNN | ~50 | shipped | E-1 |
| FR-025 | Story decomposition and wave-scheduling skills | CAP-006 | SS-05, SS-06 | BC-6.06–6.08.NNN | ~40 | shipped | E-1 |
| FR-026 | TDD delivery skill (deliver-story) | CAP-016 | SS-05, SS-06 | BC-6.09.NNN | ~35 | shipped | E-1 |
| FR-027 | Adversarial review skills | CAP-005 | SS-05, SS-06 | BC-6.10.NNN | ~25 | shipped | E-1 |
| FR-028 | Formal verification skill | CAP-021 | SS-06 | BC-6.11.NNN | ~20 | shipped | E-1 |
| FR-029 | Activation and plugin management skills | CAP-007, CAP-028 | SS-06, SS-09 | BC-6.12–6.13.NNN | ~30 | shipped | E-0, E-2 |
| FR-030 | Factory observability skills | CAP-003 | SS-06, SS-10 | BC-6.14.NNN | ~20 | shipped | E-1 |
| FR-031 | Remaining skill catalog (semport, multi-repo, DTU, holdout, UI skills) | CAP-022–027 | SS-06, SS-08 | BC-6.15–6.99.NNN | ~370 | shipped | E-1, E-3 |
| FR-032 | PreToolUse gate hooks (secrets, destructive, branch, BC/VP guard) | CAP-008 | SS-07 | BC-7.01–7.04.NNN | ~50 | shipped | E-2 |
| FR-033 | PostToolUse capture hooks (commit, PR, attribution) | CAP-013 | SS-07 | BC-7.05–7.07.NNN | ~40 | shipped | E-2, E-3 |
| FR-034 | SubagentStop / lifecycle hooks and validate-* gate family | CAP-004 | SS-07 | BC-7.08–7.10.NNN | ~80 | shipped | E-2 |
| FR-035 | Spec artifact templates | CAP-014, CAP-016 | SS-08 | BC-8.01–8.05.NNN | ~60 | shipped | E-1 |
| FR-036 | Rules and cross-cutting policy documents | CAP-014 | SS-08 | BC-8.06.NNN + BC-8.22.001, BC-8.26.001, BC-8.26.006 (Wave 8 docs anchors: S-0.05, S-5.05, S-5.06) | ~70 | shipped | E-1, E-5 |
<!-- F-201 (Wave 8 pass-3): BC-8.26.006 stories list within this row reads "S-0.05, S-5.05, S-5.06" as a UNION across the 3 cited BCs. BC-8.26.006 specifically EXCLUDES S-0.05 per F-003 Wave 8 pass-1 — actual BC-8.26.006 coverage is {S-5.05, S-5.06}. Mirrors F-101 disclosure at §8 CAP-014 row. -->
| FR-037 | Platform-aware activation and hooks.json variant management | CAP-007, CAP-028 | SS-09 | BC-9.01.001–005 | 5 | shipped | E-0, E-2 |
| FR-038 | Event emission CLI tool (bin/emit-event) | CAP-027 | SS-07, SS-10 | BC-10.01.NNN | ~10 | partial | E-3 |
| FR-039 | Factory observability bin tools | CAP-003, CAP-010 | SS-10 | BC-10.02.NNN | ~30 | shipped | E-1 |
| FR-040 | Workflow infrastructure CLI tools (wave-state, lobster-parse, compute-input-hash) | CAP-001 | SS-10 | BC-10.03.NNN | ~18 | shipped | E-1 |
| FR-041 | Skill-driven ADR authoring workflow (create-adr skill) | CAP-017 | SS-06, SS-08, SS-10 | BC-6.20.001–012 | 12 | pending | E-6 |
| FR-042 | Process self-improvement enforcement (agent prompt discipline + count-propagation hook + lessons-codification rule) | CAP-001 | SS-05, SS-07, SS-08 | BC-5.36.001–007, BC-5.37.001–002, BC-7.05.001–004, BC-8.28.001–002 | 15 | pending | E-7 |
| FR-043 | TDD Discipline Hardening — Prevent Stub-as-Implementation Anti-Pattern (anti-precedent guard + Red Gate density check + tdd_mode contract + mutation wave-gate) | CAP-016 | SS-05, SS-06, SS-08 | BC-5.38.001–006, BC-8.29.001–003, BC-8.30.001–002, BC-6.21.001–002 | 13 | pending | E-7 |
| FR-044 | Per-sink resilience: retry, circuit breaker, dead-letter queue | CAP-024 | SS-03 | BC-3.01.008, BC-3.03.002, BC-3.07.001 + v1.1 candidates (8 pending) | 3 anchored + 8 v1.1 candidates | partial | E-4 |
| FR-045 | Emit `internal.sink_error` structured event on each sink failure | CAP-003 | SS-03 | BC-3.07.002 | 1 | pending | E-4 |
| FR-046 | New Claude Code lifecycle hook events: SessionStart/SessionEnd/WorktreeCreate/WorktreeRemove/PostToolUseFailure | CAP-002, CAP-013 | SS-04, SS-01 | BC-4.04.001–005 (S-5.01 anchored); BC-4.05.001–005 (S-5.02 anchored); BC-1.10.001–002 (retired pass-4); BC-4.07.001–004 (S-5.03 anchored); BC-4.08.001–003 (S-5.04 anchored) | 17 anchored; all Tier F BCs allocated | in-progress | E-5 |
| FR-047 | Engine governance: per-story adversarial convergence loop + artifact path discipline (validate-artifact-path WASM hook + artifact-path-registry.yaml + relocate-artifact skill + per-story adversary workflow gate + validate-per-story-adversary-convergence WASM hook) | CAP-005, CAP-009, CAP-018 | SS-04, SS-05, SS-06 | BC-5.39.001–002 (Stories A workflow); BC-4.10.001–002 (Story B WASM hook); BC-4.11.001 (Story C path hook); BC-6.22.001 (Story C relocate skill) | 6 | pending | v1.0-feature-engine-discipline-pass-1 |
| FR-048 | Factory-agnostic runtime context injection for hooks via sandboxed WASM-plugin resolvers (ContextResolver trait + ResolverRegistry + resolver ABI + WaveContextResolver; closes F-P2-001) | CAP-009, CAP-002 | SS-01, SS-04 | BC-1.13.001 (SS-01 pre-dispatch injection); BC-4.12.001–005 (SS-04 resolver lifecycle/ABI/capability/isolation/merge) | 6 | pending | v1.0-feature-engine-discipline-pass-1 |

**Total: 48 FRs across 10 subsystems**

---

## 8. CAP-to-BC Anchoring

> For each of the 28 CAPs, this section cites the BC prefix ranges that implement it.
> This provides the `traceability:` anchoring for `capabilities.md`.

| CAP-ID | Title | Primary BC Ranges | Subsystems |
|--------|-------|-------------------|-----------|
| CAP-001 | Run self-orchestrating LLM-driven SDLC pipeline | BC-5.01–5.23 (workflow/orchestration); BC-6.09 (deliver-story); BC-5.02 (orchestrator) | SS-05, SS-06 |
| CAP-002 | Hook Claude Code tool calls with sandboxed WASM plugins | BC-1.01–1.09 (dispatcher); BC-4.01–4.02 (legacy-bash-adapter) | SS-01, SS-02, SS-04 |
| CAP-003 | Stream observability events to multiple configurable sinks | BC-3.01–3.06 (sink registry/routing); BC-10.02 (factory-obs bin) | SS-01, SS-03, SS-10 |
| CAP-004 | Enforce per-PR behavioral contract traceability | BC-5.05.007–010 (consistency-validator); BC-7.08–7.09 (validate-* hooks) | SS-05, SS-06 |
| CAP-005 | Run adversarial review with information asymmetry | BC-5.04.001–007 (adversary); BC-6.10 (adversarial-review skill); BC-7.10 (SHA-currency); BC-5.39.001–002 (FR-047: per-story convergence loop + scope bounding) | SS-05, SS-06 |
| CAP-006 | Decompose specs into wave-scheduled stories with parallel execution | BC-6.06–6.08 (decompose-stories, wave-scheduling, wave-gate); BC-5.22 (phase-2 workflow) | SS-05, SS-06 |
| CAP-007 | Deploy and activate the plugin on any supported platform | BC-6.01.003–006 (activation skill); BC-6.03.001–006 (activate behavior); BC-9.01.004-005 (CI matrix + hooks.json gitignore — activation-gate prerequisites) | SS-06, SS-09 |
| CAP-008 | Gate tool calls with pre-execution behavioral checks (PreToolUse) | BC-1.05.001–004 (host fn deny gates); BC-7.01–7.04 (bash PreToolUse hooks) | SS-01, SS-02, SS-04, SS-07 |
<!-- F-208 (Wave 6 pass-3): BC-list cites SS-01 + SS-07 BCs only; SS-02 enforcer-BC is BC-2.01.002 (HookResult exit-code contract) per capabilities.md:51 defensive comment; SS-04 enforcer-BCs TBD pending plugin-ecosystem BC backfill (task #108). -->
| CAP-009 | Author and publish WASM hook plugins using the Rust SDK | BC-2.01–2.05 (SDK types, ABI, proc-macro, payload); BC-4.10.001–002 (FR-047: validate-per-story-adversary-convergence hook); BC-4.11.001 (FR-047: validate-artifact-path hook); BC-4.12.001–005 (FR-048: resolver ABI, lifecycle, capability, isolation, merge); BC-1.13.001 (FR-048: pre-dispatch injection) | SS-01, SS-02, SS-04 |
| CAP-010 | Always-on dispatcher self-telemetry independent of sink config | BC-1.06.001–010 (internal log); BC-10.02 (factory-obs bin) | SS-01, SS-03, SS-10 |
| CAP-011 | Enforce fuel and epoch budgets on plugin execution | BC-1.03.001–002 (timeout/fuel BCs); BC-1.04.001–003 (engine/ticker) | SS-01 |
| CAP-012 | Recover from workflow interruption (crash recovery) | BC-5.10.001–005 (state-manager); BC-5.23 (phase-3 resume semantics) | SS-05 |
| CAP-013 | Capture post-execution activity (PostToolUse hooks) | BC-4.01–4.02 (legacy-bash-adapter); BC-7.05–7.07 (PostToolUse hooks) | SS-01, SS-04, SS-07 |
<!-- F-207 (Wave 6 pass-3): BC-list cites SS-04 + SS-07 BCs only; SS-01 enforcer-BC pending — dispatcher routing of PostToolUseFailure (and PostToolUse) lives in SS-01. Specific BC IDs TBD when SS-01 dispatcher-routing BC backfill closes (deferred to task #108 28-CAP audit). Mirrors capabilities.md:71 defensive comment. -->
| CAP-014 | Decompose product specs into verified behavioral contracts | BC-5.06.001–015 (product-owner/story-writer agents); BC-8.01–8.05 (spec templates); BC-8.22.001, BC-8.26.001, BC-8.26.006 (docs-deliverable and commit-convention rules — Wave 8 SS-08 re-anchor; S-0.05, S-5.05, S-5.06) | SS-05, SS-06, SS-08 |
<!-- F-101 (Wave 8 pass-2): BC-8.26.006 stories list within this row reads "S-0.05, S-5.05, S-5.06" as a UNION across the 3 cited BCs. BC-8.26.006 specifically EXCLUDES S-0.05 (skeleton-only stories are not user-facing-docs deliverables) per F-003 Wave 8 pass-1 — actual BC-8.26.006 coverage is {S-5.05, S-5.06}. See BC-INDEX:1885 + BC-8.26.006.md:81 for canonical scope. -->
| CAP-015 | Ingest brownfield codebases via structured multi-pass analysis | BC-5.20.001–020 (phase-0 workflow); BC-6.01 (brownfield-ingest skill) | SS-06 |
| CAP-016 | Drive TDD delivery with red/green/refactor gate enforcement | BC-5.07.028–033 (implementer agent); BC-6.09 (deliver-story skill); BC-5.23 (phase-3 workflow); BC-5.38.001–006, BC-8.29.001–003, BC-8.30.001–002, BC-6.21.001–002 (TDD hardening — S-7.03) | SS-05, SS-06, SS-08 |
| CAP-017 | Create and manage formal ADR records | BC-6.05 (create-architecture skill); BC-8.04 (ADR templates); BC-6.20.001–012 (create-adr skill) | SS-06, SS-08, SS-10 |
<!-- F-301 (Wave 6 pass-4): SS-10 added to match capabilities.md:123 (single source of truth). BC-list cites SS-06+SS-08 BCs only; SS-10 enforcer-BC pending — ADR-INDEX maintenance involves CLI tools (SS-10). Specific BC IDs TBD when SS-10 BC backfill closes (deferred to task #112). -->
| CAP-018 | Validate spec consistency across all artifact layers | BC-5.05.007–010 (consistency-validator agent); BC-6 (consistency-validation skill); BC-6.22.001 (FR-047: relocate-artifact skill — path consistency enforcement) | SS-05, SS-06 |
<!-- F-302 (Wave 6 pass-4): capabilities.md:128 expanded to match (SS-05 added to source of truth). -->
| CAP-019 | Generate domain specs from product briefs | BC-6.03 (create-domain-spec skill) | SS-06, SS-08 |
| CAP-020 | Produce and maintain a PRD with NFR catalog | BC-6.04 (create-prd skill); BC-8.02 (prd-template) | SS-06 |
| CAP-021 | Perform formal verification of pure domain logic | BC-5.07.019–023 (formal-verifier agent); BC-6.11 (formal-verify skill) | SS-06 |
| CAP-022 | Port hook plugins from bash to native WASM | BC-4.03.001 (capture-commit-activity stub); BC-6 (code-delivery skill) | SS-04, SS-06 |
| CAP-023 | Ship advanced observability sinks (HTTP, Datadog, Honeycomb) | BC-3.04.001–002 (Router extension point) | SS-03 |
| CAP-024 | Per-sink retry, circuit breaker, and dead-letter queue | BC-3.04.001–002 (Router extension); declared constants in BC-3.01 | SS-03 |
| CAP-025 | Generate semantic port translations between language implementations | BC-6 (semport-analyze skill); BC-8 (semport templates) | SS-06, SS-08 |
| CAP-026 | Manage multi-repo health and cross-repo traceability | BC-6 (multi-repo-health skill) | SS-06 |
| CAP-027 | Emit structured events from bash hooks via CLI tool | BC-10.01 (emit-event bin tool); BC-7 (hooks using _emit helper) | SS-07, SS-10 |
| CAP-028 | Install and update the plugin via Claude Code marketplace | BC-9.01.001 (semver release format — stable + prerelease — in bump-version.sh), BC-9.01.002 (chore commit discipline), BC-9.01.003 (atomic bot commit — binaries + plugin.json + marketplace.json) | SS-06, SS-09 |
<!-- F-101 (Wave 6 pass-2): BC-list column cites SS-09 BCs only; SS-06 enforcer-BC pending — install/update flows via SS-06 activate skill (BC-6.12.x family per FR-029). Specific BC IDs TBD when SS-06 backfill closes. -->
<!-- F-002 (Wave 7 pass-1): SS-10 target-module declarations on Wave 7 release-gate stories (S-0.02, S-4.08, S-5.07) are SECONDARY architectural module NOT primary CAP-028 subsystem; primary remain SS-06,SS-09. -->

---

## 9. Constraints and Assumptions

### 9.1 Technical Constraints

- **Rust compiler:** Edition 2024, rust-version 1.95 minimum (enforced in `rust-toolchain.toml`)
- **wasmtime:** pinned at the 48.x LTS line, minimum 48.0.4 (`wasmtime` and `wasmtime-wasi` in lockstep) per ADR-035 v1.2 Decision 6, clearing RUSTSEC-2026-0316/0314/0321/0322/0323/0324/0327; WASI preview-1 only for v1.0 (preview-2 deferred to v2.0 per ADR-003)
- **opentelemetry-* crates:** pinned in lockstep at 0.31 (documented in Cargo.toml)
- **Schema versioning:** `hooks-registry.toml` carries `schema_version = 2` (post-ADR-019; was 1 pre-2026-05-07); `observability-config.toml` carries `schema_version = 2` (post-ADR-015); `INTERNAL_EVENT_SCHEMA_VERSION = 1`; mismatch is a hard error
- **HOST_ABI_VERSION = 1:** frozen at v1.0; breaking ABI change requires major version bump on dispatcher AND SDK simultaneously
- **Claude Code harness:** The dispatcher runs as a Claude Code hook; the Claude Code harness itself is not owned by vsdd-factory; its hook envelope format and tool call lifecycle are external contracts

### 9.2 DTU Assessment

DTU_REQUIRED: **false** per `.factory/specs/dtu-assessment.md` (Phase 1.6a). vsdd-factory has no third-party SaaS service dependencies that require behavioral cloning. All 10 subsystems implement first-party behavior. Re-assessment is triggered when Datadog sink (S-4.02) or Honeycomb sink (S-4.03) ship (Tier E).

### 9.3 Platform Constraints

Supported platforms: darwin-arm64, darwin-x64, linux-x64, linux-arm64, windows-x64. All bash hooks (SS-07) require git-bash on Windows until Tier E native WASM ports (S-3.01, S-3.02, S-3.03) complete. Windows users have partial hook coverage until rc.1 (DRIFT-010).

### 9.4 Assumptions

- **ASM-01:** Claude Code hook envelope format remains stable at PreToolUse/PostToolUse/SessionStart/Stop/SubagentStop event schema. The SDK's dual `event_name`/`hook_event_name` alias (BC-1.02.002) handles the known v0.79.x/v1.x spelling difference; future format changes require dispatcher update.
- **ASM-02:** Operators have a working Rust toolchain ≥1.95 for plugin authoring; they do NOT need Rust to use the dispatcher (binaries are pre-built).
- **ASM-03:** The wasmtime 48.x API surface (minimum 48.0.4) is stable for the plugin ABI contract (HOST_ABI_VERSION = 1). A major wasmtime upgrade requires an ABI version bump. Rationale: ADR-035 v1.2 Decision 6 moves both `wasmtime` and `wasmtime-wasi` to the 48.x LTS line to clear RUSTSEC-2026-0316/0314/0321/0322/0323/0324/0327 (46.x is EOL with no patch).
- **ASM-04:** The legacy bash hooks will eventually be ported to native WASM; until then, `legacy-bash-adapter` remains the routing backbone. The two-routing-table state (DRIFT-004) is an accepted transitional anti-pattern resolved at 1.0 GA.

---

## 10. Known Issues and Open Risks

### 10.1 Drift Items (P0/P1 — require resolution before rc.1 or 1.0)

| ID | Severity | Description | Target Fix |
|----|----------|-------------|-----------|
| DRIFT-001 | P0 | `read_file` host fn at StoreData-typed linker returns CAPABILITY_DENIED stub; full impl in `host/read_file.rs` not wired | Must-fix before rc.1 (L-P0-001) |
| DRIFT-002 | P1 | `internal.sink_*` event constants declared in `internal_log.rs` but never emitted; SinkFailure recorded but not surfaced | S-4.10 (Tier E — primary resolver per FR-045) |
| DRIFT-003 | P2 | Per-sink dedicated OS threads vs design's promised shared tokio runtime; S-1.06 shipped but swap not made | Acceptable for 1.0; planned post-1.0 |
| DRIFT-004 | P1 | Two parallel hook-routing tables (`hooks.json` + `hooks-registry.toml`); source-of-truth ambiguity | Decision + fix at 1.0 GA (L-P0-002) |
| DRIFT-005 | P2 | HTTP/Datadog/Honeycomb sinks declared in design; warn-and-skip in `sinks/mod.rs::from_config` | Planned rc.1 (Tier E, S-4.01–4.03) |
| DRIFT-006 | P2 | SessionStart/SessionEnd/WorktreeCreate/PostToolUseFailure events not wired to plugin reactions | **BCs allocated — implementation pending** — FR-046 anchors S-5.01–5.04; BC-4.04.001–005 (S-5.01), BC-4.05.001–005 (S-5.02), BC-4.07.001–004 (S-5.03), BC-4.08.001–003 (S-5.04) all allocated 2026-04-28. Closes when S-5.04 ships (4/4 events wired). |
| DRIFT-007 | P3 | `dispatcher.shutting_down` constant defined but never emitted | Acceptable for 1.0 OR 1-line add |
| DRIFT-008 | P3 | `plugin.loaded` / `plugin.load_failed` constants declared but never emitted from plugin_loader | Acceptable for 1.0; 1-line emit call |
| DRIFT-009 | P2 | Adversary SHA-currency gate is opt-in (template only; `hooks/verify-sha-currency.sh` not auto-installed) | Documented as opt-in; CHANGELOG beta.4 |
| DRIFT-010 | P0 for Windows | 26 unported bash hooks require git-bash on Windows; native ports not yet complete | rc.1 (Tier E, S-3.01–3.03) |
| DRIFT-011 | P3 | Concurrent self-modification risk (vsdd-factory dogfooding): stories that edit hook scripts or routing config can race with active dispatcher invocations. | Acceptable for v1.0; mitigated by registry per-invocation read + PluginCache mtime + atomic story-edit commits (see KL-005). Re-evaluate if Phase 3 TDD encounters race issues. **Source:** Phase 1d pass 1 F-014; pass 2 F-018/F-021/F-025 corrections. |

### 10.2 Pending Milestones (stories not yet shipped)

| Tier | Stories | Key Work |
|------|---------|----------|
| E — Native ports + advanced sinks | S-3.01, S-3.02, S-3.03, S-3.04 (merged), S-4.01–4.07, S-4.09, S-4.10 | Port 3 bash hooks to WASM; ship HTTP/Datadog/Honeycomb sinks; retry + circuit breaker + DLQ; emit-event refactor; E2E observability tests |
| F — rc.1 gate | S-4.08 | `1.0.0-rc.1` release gate |
| G — New hook events + docs | S-5.01–5.06 (S-5.05 partial) | Wire SessionStart/SessionEnd/WorktreeCreate/PostToolUseFailure; migration guide; semver commitment docs |
| H — 1.0 GA gate | S-5.07 | `1.0.0` release gate |

### 10.3 P0 Lessons Not Yet Fully Addressed

| ID | Description | Status |
|----|-------------|--------|
| L-P0-001 | Wire `read_file` impl through to StoreData-typed linker (DRIFT-001) | Pending (must-fix before rc.1) |
| L-P0-002 | Resolve parallel hook-routing table state (DRIFT-004) | Decision pending before 1.0 |
| L-P0-003 | Backfill BCs for 24+ validate-* hooks (they are contracts without formal BC files) | In-progress (this Phase 1.5 backfill effort) |
| L-P0-004 | Resolve bin/emit-event shell tool vs host::emit_event host fn duplication (S-3.04 partial) | Pending S-3.04 completion |

### 10.4 Known Limitations (v1.0)

### KL-001 — verification-architecture.md and verification-coverage-matrix.md deferred
VP-INDEX is the authoritative VP catalog for v1.0. The 2 supplement files referenced in
ARCH-INDEX Document Map are deferred to v1.1 (cross-cutting documentation extraction).
POLICY 9 (`vp_index_is_vp_catalog_source_of_truth`) verification step "VP appears in
verification-architecture.md" is treated as vacuously satisfied while supplements are deferred.
**Source:** Phase 1d pass 1 F-004.

### KL-002 — All 57 VPs are unit-test or manual; no kani/proptest at v1.0
KD-002 (sandbox security via WASI capabilities) relies on unit-test verification at v1.0.
VP-020, VP-023, VP-042 are flagged as kani upgrade candidates for v1.1+. The current
verification posture is sufficient for v1.0 GA but not for high-assurance security claims.
**Source:** Phase 1d pass 1 F-008.

### KL-003 — Orphan VPs (VP-024, VP-048, VP-053..VP-057) lack DI parents
7 of 57 VPs are not derived from domain invariants. They cover cross-cutting concerns
(PluginCache mtime, workflow DAG validation) that don't fit the DI lattice cleanly.
This is acknowledged structural diversity, not a defect.
**Source:** Phase 1d pass 1 F-009.

### KL-004 — Workflow VPs (VP-053..VP-057) are manual-only
5 workflow-domain VPs are manual proof method. Could be automated via lobster-parse
linter in v1.1. No blocker for v1.0.
**Source:** Phase 1d pass 1 F-015.

### KL-006 — NFR catalog not lifted to L3 spec tree

The 76 NFRs are summarized in PRD §4 ("Non-Functional Requirements") and stored in `prd-supplements/nfr-catalog.md`.
Individual VPs (e.g., VP-005, VP-021, VP-044) reference specific
NFR-SEC-NNN and NFR-PERF-NNN identifiers, but no addressable L3 document enumerates
those IDs. A reader cannot look up "NFR-SEC-001" in the L3 spec tree.
**Plan:** Lift NFR catalog to `specs/nfr-catalog.md` with addressable NFR-NNN IDs in v1.1.
For v1.0, treat the prd-supplement as the authoritative NFR source.
**Source:** Phase 1d pass 3 F-032.

### KL-005 — Concurrent self-modification race (vsdd-factory dogfooding)
**Risk:** vsdd-factory IS its own product. A story modifying hooks-registry.toml or hook scripts can race with active dispatcher invocations.
**Mitigations actively in place (not via DI; via system architecture):**
- The dispatcher process reads `hooks-registry.toml` and resolves `script_path` on each invocation — not at vsdd-factory startup. Stale registry rows cannot persist across invocation boundaries.
- PluginCache (BC-1.09.001) mtime-invalidates plugin .wasm artifacts. A story that modifies a plugin .wasm forces recompile on the next dispatcher invocation.
- Story-writer agent commits story-edit transactions atomically (via state-manager Single Canonical SHA + Two-Commit Protocol). The dispatcher's next invocation observes either the pre- or post-commit registry state, never an in-flight intermediate.
**Why not a DI:** A domain invariant requires an enforcing BC + verification mechanism. The above mitigations are emergent properties of the architecture, not contract-level guarantees. Promoting this to a DI would be aspirational without an enforcing BC.
**Status for v1.0:** Acceptable. Re-evaluate if Phase 3 TDD encounters race issues.
**Source:** Phase 1d pass 1 F-014; pass 2 F-018/F-021/F-025 corrections.

---

## 11. Milestones

### 11.1 Release Ladder

| Milestone | Tag | Status | Epic(s) | Gate |
|-----------|-----|--------|---------|------|
| beta.1 | `1.0.0-beta.1` | shipped | E-0, E-1, E-2 | Tier A+B+C(6/7)+D closed; 22 stories |
| beta.2 | `1.0.0-beta.2` | shipped | E-2 | Follow-up fixes; legacy adapter + regression |
| beta.3 | `1.0.0-beta.3` | shipped | E-2 | Cache fix; stderr capture |
| beta.4 | `1.0.0-beta.4` | shipped | E-2 | Adversary SHA-currency gate; atomic binary release; state-burst skill |
| rc.1 | `1.0.0-rc.1` | **pending** | E-3, E-4 | S-4.08 gate: Tier E complete; read_file fix; advanced sinks; native ports |
| 1.0 GA | `1.0.0` | **pending** | E-5 | S-5.07 gate: Tier G complete; semver commitment; migration guide |

### 11.2 Epic-to-Milestone Map

| Epic | Title | Stories | Milestone |
|------|-------|---------|-----------|
| E-0 | Infrastructure Prep | S-0.01–S-0.05 (5) | beta.1 |
| E-1 | Dispatcher Foundation | S-1.01–S-1.09 (9) | beta.1 |
| E-2 | Legacy Adapter and Beta | S-2.01–S-2.08 (8) | beta.1–beta.4 |
| E-3 | WASM Port High-Value Hooks | S-3.01–S-3.04 (4) | rc.1 |
| E-4 | Observability Sinks RC | S-4.01–S-4.10 (10) | rc.1 |
| E-5 | New Hook Events v1.0 | S-5.01–S-5.07 (7) | 1.0 GA |

---

## 12. Acceptance Criteria and Verification Approach

### 12.1 Behavioral Contract Verification

All 1,905 BCs in `ss-01/` through `ss-10/` are verifiable. Verification is stratified:

| Test Type | Coverage Target | Primary Tools |
|-----------|----------------|---------------|
| Rust unit tests | BC-1, BC-2, BC-3, BC-4 (Subsystem A) | `cargo test`; 185+ existing tests |
| Rust integration tests | BC-1.03 execution scenarios; BC-3.05 sink integration | `crates/factory-dispatcher/tests/*.rs` |
| BATS tests | BC-7, BC-9, BC-10 (hook layer, activation, CLI tools) | ~1,262 baseline bats + 11 regression-v1.0.bats |
| Property-based tests | BC-1.05 (host fn decode round-trips) | proptest (planned CAP-021 skill) |
| Agent behavioral assertions | BC-5, BC-6 (SS-05, SS-06 agent contracts) | Session-level adversarial review; consistency-validator |
| Holdout evaluation | Phase-0 BC catalog trust basis | validate-extraction: 97.6% confirmation (122/125 sampled) |

### 12.2 Phase 0 Validation Provenance

The 1,863-BC pre-E-7 baseline catalog was validated by `extraction-validation.md` at 97.6% confirmation rate (122/125 BCs sampled at 6.8% sample rate):
- 122 CONFIRMED
- 2 INACCURATE (corrected: BC-AUDIT-067 PostToolUse→PreToolUse; BC-AUDIT-1007 "4 hooks" → "3")
- 1 HALLUCINATED (removed: "13 trybuild tests" in pass-0 inventory — actual is 0)

This establishes the trust basis for the BC catalog. Future changes to the catalog should trigger spot-check re-validation.

### 12.3 Gate Criteria

| Gate | Threshold |
|------|-----------|
| Adversarial convergence | 3 consecutive clean passes (no HIGH confidence findings) |
| Holdout evaluation | Mean satisfaction score ≥ 0.85; every critical scenario ≥ 0.60 |
| Consistency validation | 0 blocking findings across 80 criteria |
| TDD red gate | All tests fail before any implementation exists for a story |
| BC novelty assessment | SUBSTANTIVE / NITPICK binary; NITPICK = convergence reached |

---

## 13. Out-of-Scope Summary (Machine-Readable Reference)

The following features must NOT appear in any story acceptance criteria or implementation:

1. Plugin signing or plugin marketplace discovery beyond Claude Code's standard install mechanism
2. WASI preview-2 (wasi-http, wasi-sockets, wasi-command, component model)
3. seccomp, AppArmor, or Landlock process sandboxing on Linux
4. Argument-level filtering in exec_subprocess (binary_allow-list only)
5. Multi-tenant deployment or hosted SaaS mode
6. Web dashboard or browser-based UI for observability
7. Integration with project management systems (Jira, Linear, Asana, etc.)
8. Code review as an external SaaS product
9. Authentication at the dispatcher stdin boundary
10. Direct LLM provider API calls (vsdd-factory uses Claude Code's Agent tool abstraction only)

---

## 14. Document Status and Maintenance

| Field | Value |
|-------|-------|
| Phase | 1.5 (brownfield spec backfill) + F2-amendment feature delta (v1.0-feature-engine-discipline-pass-1 WASM-plugin context resolver platform) |
| BC catalog version | 1,943 BCs (1,863 pre-E-7 baseline + 15 E-7 process codification + 13 S-7.03 TDD hardening + 2 Wave 11 SS-03 + 5 S-5.01 Wave 13 + 2 S-5.01 pass-2 + 5 S-5.02 Wave 13 + 4 S-5.03 foundation burst + [various E-10 amendments] + 6 F2 engine-discipline-pass-1: BC-5.39.001/002, BC-4.10.001/002, BC-4.11.001, BC-6.22.001 + 6 F2-amendment resolver platform: BC-1.13.001, BC-4.12.001–005) |
| Validation basis | extraction-validation.md (97.6% confirmation) |
| Current release | 1.0.0-beta.7 (commit b08e085, 2026-04-26) |
| Next gate | rc.1 (S-4.08, pending Tier E) |
| DRIFT items open | 11 (DRIFT-001 through DRIFT-011) |
| Stories shipped (merged) | 26 (Tier A–D + S-3.04 + S-6.01 + S-7.01 + S-7.02 fully merged) |
| Stories partial | 3 (S-2.05, S-4.06, S-5.05) |
| Stories pending (draft) | 18 (Tiers E–H draft) + 3 (v1.0-feature-engine-discipline-pass-1 F3 pending) + 6 (F2-amendment resolver platform S-12.03–S-12.08 pending F3-amendment authoring) |
| CAPs covered | 28 / 28 |
| FRs defined | 48 |
| NFRs cataloged | 76 |
| DTU status | DTU_REQUIRED: false |
| PRD version | 1.6 (2026-10-07 — wasmtime currency: §9.1 pin and ASM-03 updated from stale 44.x to the 48.x LTS line (min 48.0.4) per ADR-035 v1.2 Decision 6 / RUSTSEC-2026-0316/0314/0321/0322/0323/0324/0327. inputs: dropped two downstream entries (stories, non-path; behavioral-contracts/BC-INDEX.md) to restore a computable acyclic input-hash. Previous: 1.5 (2026-09-13 — D-1222-DRIFT-001 pre-ratification cleanup: added Migration binary exit codes (MIG) and Migration window guards (MAINTENANCE) categories to §5.1 Error Category Summary to match prd-supplements/error-taxonomy.md. Previous: 1.4 (2026-05-13 — D-466 E-10 pass-12 fix burst F-3+F-6 closure (HH-4 regex-alternation discipline): §2.3 heading `Observability Sinks (SS-03)` → `Event Emission (OTel-Aligned) (SS-03)` per POLICY 6 canonical-name SoT). |

This PRD should be updated when:
- A Tier E/F/G story ships and its FR status changes from `pending` to `shipped`
- A DRIFT item is resolved (update Section 10.1)
- A new CAP is added to `capabilities.md` (add corresponding FR + CAP-to-BC row)
- The BC catalog is amended (update FR approximate counts and traceability matrix)
