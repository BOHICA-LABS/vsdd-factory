---
document_type: behavioral-contract
level: L3
version: "1.5"
status: draft
producer: codebase-analyzer
timestamp: 2026-04-25T00:00:00
phase: 1.4b
inputs: [bc-id-mapping.md, pass-3-behavioral-contracts.md]
input-hash: "127adec"
traces_to: bc-id-mapping.md
origin: brownfield
extracted_from: ".factory/phase-0-ingestion/pass-3-behavioral-contracts.md:536"
subsystem: "SS-01"
capability: "CAP-002"
lifecycle_status: active
introduced: v1.0.0-beta.4
modified: [v1.3-fix-burst-35-2026-05-09, v1.4-ereg-fail-closed-set-2026-10-07]
last_amended: "2026-10-07 (v1.4 — Invariant 2 exception set widened to E-REG-001/002/003 to match BC-1.14.001 / BC-7.06.001)"
deprecated: null
deprecated_by: null
replacement: null
retired: null
removed: null
removal_reason: null
---

# Behavioral Contract BC-1.08.001: dispatcher exits 0 on registry/payload/engine errors (non-blocking)

## Description

For any startup-side error (registry, payload, or engine), the dispatcher emits an `internal.dispatcher_error` event and the process exits with code 0. The dispatcher does NOT block Claude Code on its own internal failures.

## Preconditions

1. A startup-side error occurs (registry, payload, or engine).

## Postconditions

1. `internal.dispatcher_error` event is emitted.
2. Process exits with code 0 (does not block Claude Code).

## Invariants

1. Dispatcher errors are non-blocking; the harness flow continues.

2. **Exception — registry invariant violations are fail-closed (exit 2)**: Three registry-load errors are NOT fail-open; the dispatcher exits with code 2 (blocking), emits a structured `dispatcher.*` event, and writes an explicit stderr diagnostic: (a) `E-REG-001` — `hooks-registry.toml` `schema_version != 2` (`RegistryError::SchemaVersion`; BC-1.14.001 EC-006); (b) `E-REG-002` — an entry with a blocking `on_error` AND `async = true` (`RegistryError::AsyncBlockConflict`; BC-1.14.001 EC-008, BC-7.06.001 Invariant 1); (c) `E-REG-003` — duplicate `(name, event, tool)` hook entry (`RegistryError::DuplicateEntry`; BC-7.06.001 Invariant 7). Fail-open on these reproduces the silent-failure root cause ADR-019 §Decision 2 was created to eliminate. **All other** registry, payload, and engine errors (registry file not found, I/O failure, TOML parse failure, invalid tool regex, payload, engine) retain fail-open (exit 0) semantics per Invariant 1. Enforced by BC-1.14.001 Error Paths and BC-7.06.001 Postcondition 1 / §Fail-Closed Symmetry. (v1.4: the exception set was previously stated as schema-version mismatch only, which contradicted BC-1.14.001 EC-008 and BC-7.06.001 Invariant 7.)

## Edge Cases

| ID | Description | Expected Behavior |
|----|-------------|-------------------|
| EC-001 | (No edge cases captured in Phase 0 extraction; to be added in Phase 1.5/test-writer pass) | TBD |

## Canonical Test Vectors

| Input | Expected Output | Category |
|-------|----------------|----------|
| Bad registry on startup | exit 0; `internal.dispatcher_error` emitted | error |
| TBD | TBD | happy-path |
| TBD | TBD | edge-case |

## Verification Properties

| VP-NNN | Property | Proof Method |
|--------|----------|-------------|
| (TBD — to be assigned in Phase 1.6b) | | |

## Traceability

| Field | Value |
|-------|-------|
| L2 Capability | CAP-002 ("Hook Claude Code tool calls with sandboxed WASM plugins") per capabilities.md §CAP-002 |
| Capability Anchor Justification | CAP-002 ("Hook Claude Code tool calls with sandboxed WASM plugins") per capabilities.md §CAP-002 — this BC contracts the dispatcher's fail-safe non-blocking behavior on startup errors, which is a core invariant for the WASM dispatcher to never block Claude Code on its own internal failures |
| L2 Domain Invariants | TBD |
| Architecture Module | SS-01 — `crates/factory-dispatcher/src/main.rs` |
| Stories | S-2.07 (Wave 9 SS-01 straggler re-anchor); S-15.01 (single story per ADR-019 §6); S-25.08 (AC-027 delivers Invariants 1/2 — registry fail-closed exit-code mapping; stays in S-25.08 at the S-25.08/S-25.09 split, D-1252(f)) |

### Source Evidence

| Property | Value |
|----------|-------|
| **Path** | `main.rs::run` error branches all return Ok(0); `emit_dispatcher_error` writes the event |
| **Confidence** | HIGH |
| **Extraction Date** | 2026-04-25 |
| **Extracted from** | `.factory/phase-0-ingestion/pass-3-behavioral-contracts.md` line `536` |

#### Evidence Types Used

- assertion (error-branch return values)

#### Purity Classification

| Property | Assessment |
|----------|-----------|
| **I/O operations** | TBD (Phase 1.6b will refine) |
| **Global state access** | TBD |
| **Deterministic** | TBD |
| **Thread safety** | TBD |
| **Overall classification** | TBD |

#### Refactoring Notes

(TBD — to be assessed in Phase 1.6b verification properties pass)

## Amendment 2026-10-07 (v1.4 → v1.5 — story-anchor: S-25.08 AC-027 named; S-25.08/S-25.09 split)

Documentary only; no Precondition/Postcondition/Invariant/EC change. After the human-approved split of S-25.08 into S-25.08 + NEW S-25.09 (D-1252(f), amended so AC-027 stays in S-25.08), the Traceability `Stories` row now names S-25.08 as the story whose AC-027 delivers Invariants 1/2 (registry fail-closed exit-code mapping). S-25.09 does not anchor this BC.

## Amendment 2026-10-07 (v1.3 → v1.4 — Invariant 2 exception set reconciled)

Invariant 2 named schema-version mismatch (E-REG-001) as the one fail-closed startup error. BC-1.14.001 EC-008 (E-REG-002, async+block) and BC-7.06.001 Invariant 7 / §Fail-Closed Symmetry (E-REG-003, duplicate entry) are also fail-closed (exit 2), and the dispatcher (`main.rs::run`) implements all three. Invariant 2 widened to the three-code set; Invariant 1 (all other errors fail-open) unchanged. No postcondition change.

## Amendment 2026-05-09 (v1.2 → v1.3 — F5 fix-burst-35 F-P36-001: Traceability Stories TBD→S-15.01)

**F-P36-001 (BC body vs BC-INDEX Stories drift):** Traceability `Stories` row updated: replaced `TBD — single story per ADR-019 §6 (cycle v1.0-feature-plugin-async-semantics-pass-1)` with `S-15.01 (single story per ADR-019 §6)`; S-2.07 (Wave 9 SS-01 straggler re-anchor) retained. BC-INDEX row (v1.28) already listed S-2.07, S-15.01; source body TBD was pre-F3. F3 story decomposition (PR #106 merged 2026-05-07) is canonical.

## Amendment 2026-05-07 (v1.2 — F2 pass-2 fix burst)

Addresses adversary pass-2 finding F-P2-017.

**F-P2-017 (Stories field unattached)**: BC-1.08.001 was amended this cycle (v1.1 added the schema-version mismatch exception) but the Stories field remained pinned to only S-2.07 (a prior wave story). Appended "TBD — single story per ADR-019 §6 (cycle v1.0-feature-plugin-async-semantics-pass-1)" to link this cycle's story once assigned.

## Amendment 2026-05-07 (v1.1 — F2 pass-1 fix burst)

Addresses adversary pass-1 findings F-P1-004 / F-P1-011 (schema-version mismatch fail-closed exception).

**F-P1-004 / F-P1-011**: Invariant 2 added. BC-1.08.001's fail-open rule (exit 0 on startup errors) now has an explicit named exception: `hooks-registry.toml` schema-version mismatch exits 2 (fail-closed). Previously, BC-1.14.001 and BC-7.06.001 referenced this BC's "fail-open convention" for schema mismatch exit behavior, creating a contradiction: "hard error" + "exit 0 per fail-open" is observationally identical to a clean run — a silent failure. The exception is motivated by ADR-019 §Decision 5 and the user's stated principle ("no silent failures").
