---
document_type: behavioral-contract
level: L3
version: "1.14"
status: draft
producer: product-owner
timestamp: 2026-05-07T00:00:00Z
last_amended: 2026-10-07
phase: F2
inputs:
  - .factory/cycles/v1.0-feature-plugin-async-semantics-pass-1/F1-delta-analysis.md
  - plugins/vsdd-factory/hooks-registry.toml
  - crates/factory-dispatcher/src/registry.rs
input-hash: "5226893"
traces_to: .factory/specs/prd.md
origin: greenfield
extracted_from: null
subsystem: "SS-01"
capability: "CAP-002"
lifecycle_status: active
introduced: v1.0-feature-plugin-async-semantics-pass-1
modified: []
deprecated: null
deprecated_by: null
replacement: null
retired: null
removed: null
removal_reason: null
---

# BC-7.06.001: hooks-registry.toml schema_version 2 — per-plugin `async: bool` field with CI lint invariant `on_error = "block"` implies `async = false`

## Description

`hooks-registry.toml` is upgraded to `schema_version = 2`. Each `[[hooks]]` entry gains an optional `async: bool` field (default: `false`; absence is equivalent to `async = false`). The dispatcher hard-refuses any registry with `schema_version != 2`. There is no backward compatibility, no downgrade path, and no migration tooling — the schema bump is a hard cut. A CI lint invariant enforces that no entry may have both `on_error = "block"` and `async = true`; violation is a hard error at registry-load time and in CI. This BC formalizes the registry-layer obligation introduced by ADR-019.

## Preconditions

1. `hooks-registry.toml` is the source of truth for plugin classification (Layer 2 of the dual-routing-tables pattern per ADR-011).
2. The dispatcher is version 2 (with `REGISTRY_SCHEMA_VERSION = 2` in `registry.rs`).
3. The operator is updating the registry file to declare per-plugin async classification.

## Postconditions

1. **Schema version**: `hooks-registry.toml` top-level header specifies `schema_version = 2`. A registry with any other value is rejected by the dispatcher at load time with `E-REG-001` (`dispatcher.schema_mismatch` event logged); **dispatcher exits with exit code 2 (fail-closed)**; explicit stderr diagnostic is emitted. This is an explicit exception to BC-1.08.001 fail-open semantics — schema-version mismatch must not be a silent failure.

2. **Per-plugin `async` field**: Each `[[hooks]]` entry MAY include `async = true` or `async = false`. Absence of the field is equivalent to `async = false`. The field is type-checked at parse time: non-boolean values (e.g., `async = "true"`) produce a `RegistryError::ParseError`. The `async` field is NOT a global default — it is per-entry only.

3. **Default-false semantics**: Within a `schema_version = 2` registry, entries that do not declare `async` are parsed as `async = false`. Operators migrating from v1 must bump the schema_version header; per-entry `async` field absence is acceptable and does not require explicit `false`.

4. **No backward compatibility**: A v2 dispatcher receiving a `schema_version = 1` registry produces a deterministic error and halts. There is no compat shim, no silent acceptance of v1 fields, and no migration tooling provided. The operator must bump `schema_version = 2` in the registry file before deploying the v2 dispatcher. Migration is mechanical: the `async` field defaults to `false` when absent, so no content changes are required beyond the header bump.

5. **Lint invariant (load-time)**: `registry.rs::validate()` checks that no entry has both `on_error = "block"` and `async = true`. If this constraint is violated, the dispatcher emits `E-REG-002` and refuses to start. The error message must name the offending plugin.

6. **Lint invariant (CI)**: VP-078 is a CI integration test that scans `hooks-registry.toml` and asserts no entry has both `on_error = "block"` and `async = true`. CI fails if this invariant is violated. This prevents future regressions where a new plugin is added with conflicting classification.

7. **Lint invariant (edit-time)**: A Claude Code PostToolUse Edit|Write hook scans `hooks-registry.toml` for the `on_error = "block"` ⇒ `async = false` invariant. On violation, the hook returns `block_intent = true` and the tool call exits with code 2, blocking the edit before it lands in the working tree. This is the first defense layer — it catches violations at edit time, before any git commit attempt. Edit-time, registry-load-time, and CI-PR are the three defense layers per ADR-019 §Decision 4. Note: ADR-019 §Decision 4 describes this layer as "pre-commit hook" using that term generically (meaning "fires before any commit attempt is finalized"); the actual technical mechanism is the Claude Code PostToolUse Edit|Write hook lifecycle, which enforces the invariant even earlier — at the moment of file edit.

## Invariants

1. **`on_error = "block"` implies `async = false`**: An entry with `on_error = "block"` and `async = true` simultaneously is structurally invalid. Enforcement is at registry-load time (hard error `E-REG-002`), edit-time PostToolUse hook (Invariant 5 layer 1), and CI (VP-078 layer 3). This invariant exists because a blocking plugin that is classified async would have its block verdict silently discarded by the dispatcher's fire-and-forget execution model.

2. **`async` field absence is equivalent to `async = false`**: The field is optional with a `serde(default)` of `false`. This means all legacy entries (authored before schema v2) are implicitly sync. No registry edit is required for plugins that should remain synchronous.

3. **Schema version is a hard gate**: `REGISTRY_SCHEMA_VERSION` in `registry.rs` is the single authoritative value. The dispatcher accepts only this exact version. No range check, no "≥ 2" logic, no version negotiation.

4. **Per-plugin `async` field does not affect the Claude Code envelope**: The envelope (hooks.json.template and per-platform variants) is uniformly synchronous per ADR-019. The `async` field in the registry is the dispatcher's internal classification; it has no representation in `hooks.json`. The Claude Code harness never sees per-plugin async flags.

5. **Three-layer defense in depth for `on_error=block ⇒ async=false`**: The invariant is enforced at three independent layers:
   - **Layer 1 (edit-time)**: Claude Code PostToolUse Edit|Write hook scans `hooks-registry.toml`; on violation returns `block_intent = true` and exits code 2, blocking the tool call before the edit lands in the working tree. Conceptually "pre-commit" enforcement (per ADR-019 §Decision 4 which uses "pre-commit hook" generically); technically the Claude Code PostToolUse Edit|Write hook lifecycle — fires even earlier than git pre-commit. Developer learns at edit time, before CI.
   - **Layer 2 (registry-load-time)**: `registry.rs::validate()` hard-errors with `E-REG-002` if invariant is violated; dispatcher refuses to start.
   - **Layer 3 (CI-PR)**: VP-078 integration test scans the registry file; CI fails before merge.
   Per ADR-019 §Decision 4. Missing any single layer is a spec violation.

6. **Specific plugins MUST be `async = true`**: The following telemetry-only plugins MUST be classified `async = true` in the v2 registry: `capture-commit-activity`, `capture-pr-activity`, `session-start-telemetry`, `session-end-telemetry`, `worktree-hooks`, `tool-failure-hooks`, `track-agent-start`, `track-agent-stop`, `session-learning`. All validator and governance plugins with `on_error = "block"` MUST remain `async = false`. The following plugins are SYNC (on_error=continue but user-visible stderr warnings require reliable delivery): `warn-pending-wave-gate`, `regression-gate`. This is an invariant (not merely a postcondition) because a future engineer flipping any of these to `async = false` would silently degrade user-facing latency, or flipping a warn plugin to async would silently drop stderr warnings. Positive classification is verified by VP-078 Harness 3.

7. **(`name`, `event`, `tool`) tuple is unique across all `[[hooks]]` entries.** Two entries MAY share `name` and `event` if they bind to different `tool` regex values — this permits a single named plugin to enforce against multiple tool surfaces (e.g., `protect-secrets` running on both `Bash` and `Read` PreToolUse events). The `(name, event, tool)` uniqueness is enforced at registry-load time by `registry.rs::validate()`. Violations produce `RegistryError::DuplicateEntry { name, event, tool }` (`dispatcher.registry_invalid` with reason `duplicate_hook_registration`); dispatcher exits non-zero (fail-closed). Note: `tool` in this tuple is the raw regex string value (or `None` for "all tools"); two entries with `tool = None` and the same `name`+`event` are duplicates. **String equality, not regex equivalence:** Comparison of the `tool` field uses raw-string equality (Rust `Option<String>` equality). Two entries with `tool = '^Bash$'` and `tool = 'Bash'` are DISTINCT entries despite matching the same tool surface — the registry does NOT perform regex-equivalence detection. Operators MUST normalize their `tool` regex strings if they intend semantic deduplication.

   **[fail-closed] F-P8-001 — DuplicateEntry is fail-closed with E-REG-003.** Hard-error with `RegistryError::DuplicateEntry { name, event, tool }` on the first violation. Implementation MUST: (a) emit `dispatcher.registry_invalid` structured event with `error_code = "E-REG-003"`, `violation = "duplicate_hook_registration"`, and the offending plugin's `name`/`event`/`tool` fields; (b) write the error to stderr with prefix `[E-REG-003]`; (c) exit with non-zero status (2). The dispatcher MUST NOT silently fall through; this is a fail-closed semantic invariant identical in classification to E-REG-001 (SchemaVersion) and E-REG-002 (AsyncBlockConflict). Error code E-REG-003 is reserved for DuplicateEntry exclusively (see §E-REG-NNN Error Code Table).

## Implementation Notes

### (name, event, tool) Tuple Uniqueness Check — validate() Obligation (F-P2-011)

Implementations MUST add a load-time uniqueness check for the `(name, event, tool)` tuple in `crates/factory-dispatcher/src/registry.rs::validate()` (or equivalent). The check must:

1. Iterate all `[[hooks]]` entries and collect `(name, event, tool)` tuples.
2. Detect any duplicate tuple (same `name`, same `event`, same `tool` regex string or both `None`).
3. Hard-error with `RegistryError::DuplicateEntry { name, event, tool }` on the first violation; dispatcher refuses to start.
4. The error message MUST name the offending plugin `name`, `event`, and `tool` value to aid operator debugging.

The existing CI lint plugin (`lint-registry-async-invariant`, VP-078) MAY also enforce this at edit time; verify and extend its check set as needed to cover the `(name, event, tool)` tuple constraint in addition to the `on_error=block ⇒ async=false` invariant.

**Rationale:** `protect-secrets` legitimately appears twice with `event = "PreToolUse"` — once with `tool = "Bash"` and once with `tool = "Read"`. Under the prior `(name, event)` uniqueness check, this valid configuration would have produced a false-positive duplicate error. The `(name, event, tool)` tuple correctly allows this pattern while still rejecting true duplicates (identical name+event+tool).

### Equality Semantics — Raw-String, Not Regex-Equivalence (F-P3-003)

`registry.rs::validate()` implements the uniqueness check using `HashSet<(String, String, Option<String>)>` with derived `PartialEq`. This is the canonical implementation surface for the equality semantics specified in Invariant 7. Comparison is byte-for-byte string equality on the raw regex string value — the registry does NOT parse, compile, or normalize regex patterns before comparison. Consequently, `tool = '^Bash$'` and `tool = 'Bash'` are treated as distinct keys and will NOT raise a `DuplicateEntry` error even though both patterns match the same tool surface. Operators who intend semantic deduplication MUST normalize their regex strings to a canonical form before authoring registry entries.

### Fail-Closed Symmetry Across E-REG-NNN Error Codes (F-P8-001)

All three E-REG-NNN error codes share the same fail-closed semantic — load failure produces stderr eprintln, structured event emission via host emit, and dispatcher exit 2. Implementations MUST NOT route any `RegistryError` variant through a catch-all that returns exit 0. Specifically:

- `E-REG-001` (`RegistryError::SchemaVersion`) — catch-all `_ => 0` silently exits 0; MUST exit 2.
- `E-REG-002` (`RegistryError::AsyncBlockConflict`) — exits 2 via the `RegistryError::AsyncBlockConflict { name }` arm in `factory_dispatcher::main::run` (the arm carrying `eprintln! + 2` above the catch-all in `main.rs::run`); correct.
- `E-REG-003` (`RegistryError::DuplicateEntry`) — fixed at F-P8-001 by adding the explicit `RegistryError::DuplicateEntry` arm in `factory_dispatcher::main::run` returning exit code 2 with `[E-REG-003]` stderr prefix and `dispatcher.registry_invalid` event emission. This arm precedes the catch-all `_ => 0` arm. Future `RegistryError` variants MUST receive an explicit non-zero exit branch and MUST NOT fall through to the catch-all (per TD-028 process-gap codification). (F-P13-002: migrated from stale line numbers 148–151/143–145 to stable symbol anchors per TD-VSDD-091.)

Any future `RegistryError` variant added to `registry.rs` MUST receive an explicit exit-code branch in `main.rs` before the catch-all. The catch-all applies ONLY to unknown/future variants (variants not enumerated in `main.rs::run`): it MUST map them to a non-zero exit code (e.g., `_ => 1`), never 0. The four existing operational variants — `RegistryError::NotFound`, `RegistryError::Io`, `RegistryError::Toml`, `RegistryError::ToolRegex` — are NOT subject to this rule: they carry no E-REG code and remain fail-open (exit 0) per BC-1.08.001 Invariant 1, and an implementation MUST route them through explicit exit-0 arms (not through the catch-all) so that the catch-all's non-zero mapping for unknown variants is not defeated.

## Error Paths

| Condition | Behavior |
|-----------|----------|
| Registry `schema_version = 1` (or any non-2 value) | Dispatcher hard-errors at load time; `dispatcher.schema_mismatch` logged; **exit code 2 (fail-closed)** — explicit exception to BC-1.08.001 fail-open; explicit stderr diagnostic emitted; no plugins executed |
| `async = true` (non-boolean) in a `[[hooks]]` entry | `RegistryError::ParseError` at TOML parse time; dispatcher refuses to start |
| Entry has `on_error = "block"` AND `async = true` | `E-REG-002` at `validate()` time; `dispatcher.registry_invalid` logged with plugin name; dispatcher refuses to start |
| Two entries share identical `(name, event, tool)` tuple | **[fail-closed] E-REG-003** — `RegistryError::DuplicateEntry { name, event, tool }` at `validate()` time; `dispatcher.registry_invalid` event emitted with `error_code = "E-REG-003"` and `violation = "duplicate_hook_registration"`; stderr prefixed `[E-REG-003]`; **exit code 2** (F-P8-001 — MUST NOT silently exit 0 via catch-all) |
| Registry file missing | Existing behavior per BC-1.01.014 (`RegistryError::NotFound`); unchanged |

## Related BCs

- BC-1.14.001 — depends on: the `async` field in each `RegistryEntry` is read by the dispatcher partition logic defined in BC-1.14.001
- BC-1.01.001 — amends: "Registry rejects unknown schema version" must now accept only schema_version=2 (not 1); amended separately per this cycle
- BC-1.01.007 — amends: "parses_minimal_registry with schema_version=1" test vector is now invalid; must be updated to schema_version=2
- BC-7.02.006 — sibling: dispatcher routing binds hooks via `[[hooks]]` entry fields; the new `async` field is added to this entry shape

## Architecture Anchors

- `plugins/vsdd-factory/hooks-registry.toml` — schema_version bumped to 2; `async = true` added for telemetry plugins; all block-capable validators remain `async = false`
- `crates/factory-dispatcher/src/registry.rs` — `RegistryEntry.async_flag: bool` field with `#[serde(rename = "async", default)]`; `REGISTRY_SCHEMA_VERSION = 2`; `validate()` enforcing Invariant 1
- CI lint hook / bats test — VP-078 harness scanning hooks-registry.toml for `on_error=block ⇒ async=false`

## Story Anchor

TBD — single story per ADR-019 §6 (no phased rollout, user decision 2026-05-07); S-25.08 AC-027 delivers the Invariant 7 / §Fail-Closed Symmetry registry fail-closed exit-code mapping (E-REG-001/002/003 exit 2; operational variants exit 0). S-25.09 does NOT anchor this BC.

## VP Anchors

- VP-078 — CI lint: no registry entry has both `on_error = "block"` and `async = true`; integration test method; P1

## Edge Cases

| ID | Description | Expected Behavior |
|----|-------------|-------------------|
| EC-001 | `[[hooks]]` entry has no `async` key | Deserialized as `async = false` (serde default); plugin is synchronous |
| EC-002 | `[[hooks]]` entry has `async = false` (explicit) | Same behavior as absent; plugin is synchronous |
| EC-003 | `[[hooks]]` entry has `async = true` AND `on_error = "continue"` | Valid: a non-blocking plugin may be async. Dispatcher places it in async_group. |
| EC-004 | `[[hooks]]` entry has `async = true` AND `on_error = "block"` | Invalid at `validate()` time; `E-REG-002`; dispatcher refuses to start |
| EC-005 | Registry has `schema_version = 1` (operator forgot to bump) | Hard error `E-REG-001`; clear error message tells operator to update `schema_version = 2` |
| EC-006 | Registry has `schema_version = 3` (future version loaded by v2 dispatcher) | Hard error `E-REG-001`; dispatcher rejects unknown versions; forward-compatibility is not provided in v2 |
| EC-007 | `capture-commit-activity` is `async = true`; `on_error = "continue"` | Valid; classified into async_group; telemetry events fire-and-forget; no block verdict possible |
| EC-008 | `validate-template-compliance` is `async = false`; `on_error = "block"` | Valid; classified into sync_group; block verdicts reach Claude Code |
| EC-009 | `async = "yes"` (string instead of bool) in a `[[hooks]]` entry | TOML type error at parse time; `RegistryError::ParseError`; dispatcher refuses to start |
| EC-010 | Registry with zero `[[hooks]]` entries (empty plugin list); `schema_version = 2` | Valid; parses successfully; dispatcher returns exit 0 on any event (no plugins to run) |

## Canonical Test Vectors

| Input | Expected Output | Category |
|-------|----------------|----------|
| Registry v2; `async` absent | `entry.async = false` (default); plugin placed in sync_group by dispatcher | happy-path (default) |
| Registry v2; `async = true`; `on_error = "continue"` | Valid parse; plugin placed in async_group by dispatcher | happy-path (async telemetry) |
| Registry v2; `async = true`; `on_error = "block"` | `E-REG-002` at `validate()` time; dispatcher refuses to start | invariant-violation |
| Registry with `schema_version = 1` | `E-REG-001` (`RegistryError::SchemaVersion { got: 1, expected: 2 }`); no plugins run | schema-mismatch |
| Registry with `schema_version = 99` | `E-REG-001` (`RegistryError::SchemaVersion { got: 99, expected: 2 }`); no plugins run | schema-mismatch |
| `async = "true"` (string) in entry | `RegistryError::ParseError`; dispatcher refuses to start | parse-error |
| Registry v2; `capture-commit-activity` with `async = true` + `on_error = "continue"` | Valid; `capture-commit-activity` in async_group; no CI lint violation | telemetry-classification |
| Registry v2; `validate-template-compliance` with `async = false` + `on_error = "block"` | Valid; `validate-template-compliance` in sync_group; block verdict can reach Claude Code | validator-classification |
| CI lint scan: registry with `on_error = "block"` AND `async = true` | VP-078 bats test fails; CI reports violation with plugin name | ci-lint-failure |
| CI lint scan: fully valid v2 registry | VP-078 bats test passes; CI continues | ci-lint-pass |
| Registry v2; entry `(name="protect-secrets", event="PreToolUse", tool="Bash")` AND entry `(name="protect-secrets", event="PreToolUse", tool="Read")` — different `tool` values | Valid: `(name, event, tool)` tuples are distinct; registry loads successfully; both entries dispatched independently per event+tool match | tuple-uniqueness-allowed-multi-tool |
| Registry v2; two entries with identical `(name="protect-secrets", event="PreToolUse", tool="Bash")` | **[fail-closed E-REG-003 per F-P8-001]** `RegistryError::DuplicateEntry { name: "protect-secrets", event: "PreToolUse", tool: Some("Bash") }`; `dispatcher.registry_invalid` event emitted with `error_code = "E-REG-003"` and `violation = "duplicate_hook_registration"`; stderr line prefixed `[E-REG-003]`; **exit code 2** (NOT silent exit 0) | tuple-uniqueness-violation |
| Registry v2; two entries with `name="x"`, `event="PreToolUse"`, both with `tool = None` (absent) | **[fail-closed E-REG-003 per F-P8-001]** `RegistryError::DuplicateEntry { name: "x", event: "PreToolUse", tool: None }`; `dispatcher.registry_invalid` event emitted with `error_code = "E-REG-003"` and `violation = "duplicate_hook_registration"`; stderr line prefixed `[E-REG-003]`; **exit code 2** (NOT silent exit 0) | tuple-uniqueness-violation-no-tool |

## Verification Properties

| VP-NNN | Property | Proof Method |
|--------|----------|-------------|
| VP-078 | No `[[hooks]]` entry in `hooks-registry.toml` has both `on_error = "block"` and `async = true`; scanned at CI time | integration |

## E-REG-NNN Error Code Table

Canonical error codes for all registry-validation failures in `registry.rs::validate()` and dispatcher load path. All three are fail-closed: each produces a structured `dispatcher.registry_invalid` or `dispatcher.schema_mismatch` event, a stderr line with the prefixed code, and dispatcher exit code 2.

| Code | Error Variant | Event Type | Violation / Reason | Fail-Closed | Notes |
|------|--------------|------------|-------------------|-------------|-------|
| E-REG-001 | `RegistryError::SchemaVersion { got, expected }` | `dispatcher.schema_mismatch` | `schema_version` mismatch | YES — exit 2 | Existing; Postcondition 1 |
| E-REG-002 | `RegistryError::AsyncBlockConflict { name }` | `dispatcher.registry_invalid` | `async_block_conflict` | YES — exit 2 | Existing; Postcondition 5 / Invariant 1 |
| E-REG-003 | `RegistryError::DuplicateEntry { name, event, tool }` | `dispatcher.registry_invalid` | `duplicate_hook_registration` | YES — exit 2 | **NEW per F-P8-001**; Invariant 7 |

**E-REG-003 event payload** (for `dispatcher.registry_invalid`):
```json
{
  "type": "dispatcher.registry_invalid",
  "trace_id": "<uuid-v4>",
  "offending_plugin": "<name>",
  "offending_event": "<event>",
  "offending_tool": "<tool regex string or null>",
  "violation": "duplicate_hook_registration",
  "timestamp": "<ISO-8601>",
  "error_code": "E-REG-003"
}
```

**Sibling BC-3.08.001 cross-reference**: BC-3.08.001 v1.8 Event 3 (`dispatcher.registry_invalid`) enumerates two valid `error_code` values: `E-REG-002` (`violation: "async_block_conflict"`) and `E-REG-003` (`violation: "duplicate_hook_registration"`). The E-REG-003 wire-format payload defined here is the authoritative schema; BC-3.08.001 v1.8 Event 3 E-REG-003 wire-format section is the SS-03 catalog mirror (updated from v1.7 per F-P14-001 Path B to include `offending_event` and `offending_tool`).

## Traceability

| Field | Value |
|-------|-------|
| L2 Capability | CAP-002 ("Hook Claude Code tool calls with sandboxed WASM plugins") per capabilities.md §CAP-002 |
| Capability Anchor Justification | CAP-002 ("Hook Claude Code tool calls with sandboxed WASM plugins") per capabilities.md §CAP-002 — this BC contracts the registry schema that classifies each WASM plugin as sync or async, which is the mechanism enabling per-plugin async semantics while preserving `on_error = "block"` enforcement |
| L2 Domain Invariants | DI-014 — Schema version mismatch is a hard load error (the fail-closed schema_version=2 enforcement here is the BC-7 enforcement arm of DI-014, complementing the BC-1 arm in BC-1.14.001; DI-014 enforcement-owner now extends to BC-7) |
| Architecture Module | SS-01 (primary — runtime enforcement via `crates/factory-dispatcher/src/registry.rs::validate()`) + SS-07 (registry file shape — `plugins/vsdd-factory/hooks-registry.toml`). Frontmatter `subsystem` is SS-01 per F-P1-006 resolution: the runtime enforcement is the failure-loud mechanism and is housed in SS-01. |
| ADR | ADR-019 — Async Semantics at Registry Layer, Not Envelope Layer |
| Stories | S-15.01 (single story per ADR-019 §6); S-25.08 (AC-027 delivers the Invariant 7 / §Fail-Closed Symmetry registry fail-closed exit-code mapping; stays in S-25.08 at the S-25.08/S-25.09 split, D-1252(f)) |
| Cycle | v1.0-feature-plugin-async-semantics-pass-1 (F2) |
| VP-077 Harness 1 Amendment Obligation | VP-077 Harness 1 `kani::assume` precondition for entry uniqueness must be updated from `(name, event)` to `(name, event, tool)` tuple uniqueness to remain consistent with Invariant 7 v1.4. **Architect handles VP-077 v1.8** — this row is a PO sync note for handoff. If VP-077 v1.8 has not already addressed this, architect must amend the `kani::assume` precondition in the Harness 1 fixture to reflect `(name, event, tool)` tuple uniqueness. |

### Source Evidence

| Property | Value |
|----------|-------|
| **Path** | `.factory/cycles/v1.0-feature-plugin-async-semantics-pass-1/F1-delta-analysis.md` Section 3 (Change 2 and Change 4) and Section 5 (New BCs Needed) |
| **Confidence** | HIGH — F1 explicitly called for this BC; schema_version bump and `async` field are the direct fix for 55 silently-discarded block verdicts |
| **Extraction Date** | 2026-05-07 |

#### Purity Classification

| Property | Assessment |
|----------|-----------|
| **I/O operations** | Schema validation: reads registry file from disk. CI lint: reads registry file in CI. No I/O during `validate()` itself (pure check on already-parsed struct). |
| **Global state access** | No global state in `validate()`. |
| **Deterministic** | YES — given same registry content, always produces same validation result. |
| **Thread safety** | YES — `validate()` is a pure check on an immutable parsed struct. |
| **Overall classification** | Deterministic with filesystem I/O at load time only; `validate()` is a pure fn. |

## Amendment 2026-10-07 (v1.13 → v1.14 — story-anchor: S-25.08 AC-027 named; S-25.08/S-25.09 split)

Documentary only; no Precondition/Postcondition/Invariant/EC change. After the human-approved split of S-25.08 into S-25.08 + NEW S-25.09 (D-1252(f), amended so AC-027 stays in S-25.08), the Story Anchor and Traceability `Stories` row now name S-25.08 as the story whose AC-027 delivers Invariant 7 / §Fail-Closed Symmetry. S-25.09 does not anchor this BC. No delivery-gated promotion change (this BC's POL-14 status is unaffected by S-25.09).

## Amendment 2026-10-07 (v1.12 → v1.13 — catch-all scope vs BC-1.08.001 Invariant 1 reconciled)

§Fail-Closed Symmetry said the `RegistryError` catch-all "MUST map to a non-zero exit code", which read as contradicting BC-1.08.001 Invariant 1 (registry file not found / I/O / TOML parse / invalid tool regex are fail-open, exit 0). Clarified: the non-zero rule applies to unknown/future variants only; the four enumerated fail-open variants exit 0 via explicit arms. No postcondition or invariant change. Conformance note: at develop ce2421be `main.rs::run` ends the `match &e` with a single `_ => 0` arm covering both the four fail-open variants and any future variant, so a future variant would silently exit 0 — a code/spec gap routed to implementer (add explicit `NotFound | Io | Toml | ToolRegex => 0` arm; make the catch-all non-zero).

## Amendment 2026-05-09 (v1.11 → v1.12 — F5 fix-burst-35 F-P36-001: Traceability Stories TBD→S-15.01)

**F-P36-001 (BC body vs BC-INDEX Stories drift):** Traceability `Stories` row updated from `TBD — single story per ADR-019 §6 (no phased rollout, user decision 2026-05-07)` to `S-15.01 (single story per ADR-019 §6)`. BC-INDEX row (v1.28) already listed S-15.01; source body was pre-F3. F3 story decomposition (PR #106 merged 2026-05-07) is canonical.

## Amendment 2026-05-09 (v1.10 → v1.11 — F-P24-002/003/004: RegistryEntry.async → RegistryEntry.async_flag in §Architecture Anchors)

**Driver:** F-P24-002/003/004 corpus-wide sweep for all historical fabricated symbols. The §Architecture Anchors second bullet cited `RegistryEntry.async: bool` — the fabricated field name. The real Rust field name is `async_flag` (because `async` is a reserved keyword in Rust); it is serialized as `"async"` in TOML/JSON via `#[serde(rename = "async", default)]`.

**Change made:**
- §Architecture Anchors second bullet: `RegistryEntry.async: bool` → `RegistryEntry.async_flag: bool` with `#[serde(rename = "async", default)]` noted inline.
- Frontmatter `version:` bumped `"1.10"` → `"1.11"`.

**Verification:** `grep -n "async_flag" crates/factory-dispatcher/src/registry.rs` confirms field name at `registry.rs:245` per the F5 cycle verified symbol table.

**POLICY 1 verification:** No content removed. Fabricated field name corrected to real Rust field name; serde rename annotation added for TOML/JSON round-trip clarity.

## Amendment 2026-05-08 (v1.9 → v1.10 — F-P20-005: §Amendment v1.7→v1.8 grep-verification reworded for clarity)

**Driver:** F-P20-005 — The grep verification line at the end of the §Amendment 2026-05-08 (v1.7→v1.8) block read "Symbol anchors are stable across future refactors," which was ambiguous: a reader could interpret the entire line (including `main.rs:139` and `main.rs:173`) as stable anchors rather than point-in-time evidence. The intent of the original author was that the symbol names are stable, while the line numbers are historical evidence only.

**Change made:**
- §Amendment 2026-05-08 (v1.7→v1.8) final line: added `"at amendment time"` qualifier and explicit disambiguation — line numbers are point-in-time evidence; symbol names (`RegistryError::AsyncBlockConflict` arm, catch-all `_ => 0` arm in `factory_dispatcher::main::run`) are the stable references.
- Frontmatter `version:` bumped `"1.9"` → `"1.10"`.

**NOTE:** This edit is inside the §Amendment v1.7→v1.8 block, which is exempt from the validate-stable-anchors hook. The embedded `main.rs:139` and `main.rs:173` cites are intentionally preserved as historical point-in-time evidence. No line numbers were added or removed; only the explanatory framing language was clarified.

**POLICY 1 verification:** No content removed. The line numbers and symbol names are preserved verbatim; only the trailing sentence was reworded to add "at amendment time" and explicit disambiguation.

## Amendment 2026-05-08 (v1.8 → v1.9 — F-P14-005: §Fail-Closed Symmetry E-REG-003 wording clarified)

**Driver:** F-P14-005 — §Fail-Closed Symmetry Across E-REG-NNN Error Codes E-REG-003 bullet had grammatically misleading wording. The phrase "silently exits 0; MUST exit 2 (F-P8-001 fix)" was written in present-tense defect language ("silently exits 0") even though the F-P8-001 fix was already in place as of that amendment. A reader encountering the BC post-fix could interpret the bullet as describing the current (broken) state rather than the resolved state.

**Changes made:**

1. **E-REG-003 bullet in §Fail-Closed Symmetry rewritten** (F-P14-005): Defect-framing language ("the catch-all `_ => 0` arm ... silently exits 0; MUST exit 2") replaced with resolved-state framing: "fixed at F-P8-001 by adding the explicit `RegistryError::DuplicateEntry` arm in `factory_dispatcher::main::run` returning exit code 2 with `[E-REG-003]` stderr prefix and `dispatcher.registry_invalid` event emission." Forward obligation added: "Future `RegistryError` variants MUST receive an explicit non-zero exit branch and MUST NOT fall through to the catch-all (per TD-028 process-gap codification)." The F-P13-002 stable-anchor parenthetical is preserved verbatim.

2. **Frontmatter:** `version: "1.8"` → `"1.9"`.

**POLICY 1 verification:** All prior changelog entries preserved verbatim. The v1.5→v1.6 changelog (historical defect description) is untouched — it accurately records what was defective at that time and is a historical record. Only the live Implementation Notes bullet is rewritten.

**POLICY 7 verification:** H1 heading unchanged.

**TD-028 citation:** The forward obligation ("Future `RegistryError` variants MUST receive an explicit non-zero exit branch") cites TD-028 (process-gap codification) as the governing policy that prevents the same catch-all pattern from reappearing with future `RegistryError` additions.

---

## Amendment 2026-05-08 (v1.7 → v1.8 — F-P13-002: §Fail-Closed Symmetry line citations refreshed post-EC-012 refactor)

**Driver:** F-P13-002 — §Fail-Closed Symmetry Across E-REG-NNN Error Codes (added in v1.6, F-P8-001) cited stale `main.rs` line numbers that drifted after the EC-012 partial-drain refactor: the catch-all `_ => 0` was cited at `main.rs:148–151` (actual: line 173 post-refactor), and the `AsyncBlockConflict` explicit-exit-2 arm was cited at `lines 143–145` (actual: arm spans lines 139–152 post-refactor, with arguments to the emit call occupying lines 143–145). This is the same drift class as F-P10-002. EC-012 shifted all these line numbers.

**Changes made:**

1. **§Fail-Closed Symmetry — E-REG-002 bullet refreshed** (F-P13-002): Stale `lines 143–145` citation replaced with stable symbol anchor: the `RegistryError::AsyncBlockConflict { name }` arm in `factory_dispatcher::main::run` (the arm carrying `eprintln! + 2` above the catch-all in `main.rs::run`). Migrated from line numbers to symbol anchors per TD-VSDD-091.

2. **§Fail-Closed Symmetry — E-REG-003 bullet refreshed** (F-P13-002 primary fix): Stale `main.rs:148–151` catch-all citation and stale `lines 143–145` AsyncBlockConflict reference replaced with stable symbol anchors: the catch-all `_ => 0` arm is identified as `factory_dispatcher::main::run` (the arm guarding unmatched `RegistryError::*` variants); the AsyncBlockConflict reference uses `RegistryError::AsyncBlockConflict` arm name. Per TD-VSDD-091 stable-anchor convention, symbol-based anchors are preferred over line numbers for implementation references subject to refactor drift.

3. **Frontmatter:** `version: "1.7"` → `"1.8"`.

**POLICY 1 verification:** Historical changelog references at the former lines 257 and 263 (now in §Amendment 2026-05-08 v1.5 → v1.6 historical text) are preserved verbatim and untouched — those are records of the implementation state at v1.6 amendment time.

**POLICY 7 verification:** H1 heading unchanged.

**grep verification (HEAD e5108a2 at amendment time):** `RegistryError::AsyncBlockConflict` arm at `main.rs:139`; catch-all `_ => 0` at `main.rs:173`. The line numbers above are point-in-time evidence captured at amendment time; the stable references for future readers are the symbol anchors (`RegistryError::AsyncBlockConflict` arm and catch-all `_ => 0` arm in `factory_dispatcher::main::run`), not the line numbers.

## Amendment 2026-05-08 (v1.6 → v1.7 — F-P9-001: stale sibling note cleaned; E-REG-002 violation string canonicalized)

**Driver:** F-P9-001 — The sibling BC-3.08.001 PO sync note authored at line 204 during the v1.6 burst cited "BC-3.08.001 v1.6" and stated that E-REG-003 "MUST be added" — both facts were stale. BC-3.08.001 was amended to v1.7 in the same fix-burst-7 that produced BC-7.06.001 v1.6, and that amendment (a) added E-REG-003 to Event 3 and (b) canonicalized the E-REG-002 violation string from `"on_error_block_with_async_true"` to `"async_block_conflict"`. Additionally, BC-7.06.001's own E-REG-NNN Error Code Table at the former line 187 still carried the legacy violation string `"on_error_block_with_async_true"` for E-REG-002.

**Changes made:**

1. **Sibling note at former line 204 replaced** (F-P9-001 primary fix): Stale "BC-3.08.001 v1.6 / MUST be added" text replaced with a cross-reference reflecting the completed state: BC-3.08.001 v1.7 enumerates both E-REG-002 (`async_block_conflict`) and E-REG-003 (`duplicate_hook_registration`); the E-REG-003 payload defined here is authoritative; BC-3.08.001 v1.7 lines 107-117 are the SS-03 catalog mirror.

2. **E-REG-NNN Error Code Table E-REG-002 violation string canonicalized**: `"on_error_block_with_async_true"` updated to `"async_block_conflict"` to match BC-3.08.001 v1.7's canonical value. This was a stale legacy string in this BC's own table; the v1.5→v1.6 changelog at line 259 (historical record of BC-3.08.001 v1.6 state) is preserved unchanged per POLICY 1 append-only.

3. **§PO Sync Notes audit**: No standalone §PO Sync Notes section exists in this file — the sync note was inline at former line 204 and in the v1.5→v1.6 amendment changelog (line 259). The inline note at line 204 is now replaced (change 1 above). The v1.5→v1.6 changelog entry at line 259 is a historical record of what was outstanding at v1.6 amendment time and is preserved verbatim per POLICY 1 — it accurately describes the state of BC-3.08.001 at the time of that amendment. The VP-077 Harness 1 Amendment Obligation row in Traceability remains active (architect scope, not resolved by this amendment).

4. **Frontmatter:** `version: "1.6"` → `"1.7"`.

**POLICY 7 verification:** H1 heading unchanged.

**POLICY 1 verification:** No content removed. The stale note replacement is a factual correction of a cross-reference, not removal of an invariant or ID. The v1.5→v1.6 changelog text at line 259 (historical record) is untouched.

## Amendment 2026-05-08 (v1.5 → v1.6 — F-P8-001: Invariant 7 explicitly classified as [fail-closed] with E-REG-003 reservation)

**Driver:** F-P8-001 — Spec-implementation drift: `main.rs:148–151` catch-all `_ => 0` silently exits 0 for `RegistryError::DuplicateEntry`, whereas `AsyncBlockConflict` correctly exits 2 at lines 143–145. The existing Invariant 7 wording ("dispatcher exits non-zero (fail-closed)") was correct in intent but not explicit enough to prevent asymmetric implementation — "refuses to start" was interpreted differently for DuplicateEntry vs. AsyncBlockConflict.

**Changes made:**

1. **Invariant 7 amended (append-only per POLICY 1):** Appended `[fail-closed] F-P8-001` classification block explicitly naming E-REG-003 as the canonical error code, requiring `dispatcher.registry_invalid` event with `error_code = "E-REG-003"` and `violation = "duplicate_hook_registration"`, stderr prefix `[E-REG-003]`, and exit code 2. States that DuplicateEntry is fail-closed identical in classification to E-REG-001 and E-REG-002.

2. **`## Implementation Notes` extended** with `### Fail-Closed Symmetry Across E-REG-NNN Error Codes (F-P8-001)` subsection: enumerates all three E-REG-NNN variants, names the specific `main.rs` catch-all at lines 148–151 as the implementation defect, and states that any future `RegistryError` variant must receive an explicit exit-code branch (never 0) before the catch-all.

3. **Error Paths table extended** with a new row for `RegistryError::DuplicateEntry` naming E-REG-003, `dispatcher.registry_invalid` event, `[E-REG-003]` stderr prefix, and exit code 2 with explicit F-P8-001 note that it MUST NOT silently exit 0.

4. **Canonical Test Vectors updated** (POLICY 12 — bc_tv_emitter_consistency): both DuplicateEntry test vector rows now specify the full fail-closed semantics — `dispatcher.registry_invalid` event with `error_code = "E-REG-003"`, stderr prefix `[E-REG-003]`, and exit code 2 (NOT silent exit 0).

5. **`## E-REG-NNN Error Code Table` section added:** Defines the authoritative three-row table (E-REG-001, E-REG-002, E-REG-003), their variants, event types, violation strings, and fail-closed status. Includes E-REG-003 wire-format payload example with `offending_plugin`, `offending_event`, `offending_tool`, and `violation` fields.

6. **Frontmatter:** `version: "1.5"` → `"1.6"`; `last_amended:` value unchanged (already `2026-05-08`).

**POLICY 7 verification:** H1 heading unchanged — title remains authoritative as authored in v1.0.

**POLICY 1 verification:** No content removed. All amendments are additive.

**BC-3.08.001 sibling check (PO sync note):** BC-3.08.001 v1.6 Event 3 (`dispatcher.registry_invalid`) enumerates only `error_code: "E-REG-002"` / `violation: "on_error_block_with_async_true"`. E-REG-003 (`violation: "duplicate_hook_registration"`) is not listed as a valid value. A sibling amendment to BC-3.08.001 is required to add E-REG-003 to the Event 3 payload schema enumeration. This is tagged as a follow-up architect amendment.

## Amendment 2026-05-08 (v1.4 → v1.5 — F-P3-003: Invariant 7 clarified — comparison is raw-string equality, not regex equivalence)

**Driver:** F-P3-003 — Invariant 7 (v1.4) stated that the `tool` field comparison uses the "raw regex string value" but did not explicitly specify that comparison is raw-string equality rather than regex equivalence. A future operator could reasonably assume the dispatcher performs regex-equivalence detection before deciding whether two entries are duplicates. The implementation uses `HashSet<(String, String, Option<String>)>` with derived `PartialEq`, meaning comparison is byte-for-byte string equality — no regex parsing, no normalization.

**Changes made:**

1. **Invariant 7 amended (append-only per POLICY 1):** Added clarifying sentence after the existing `tool = None` note: "**String equality, not regex equivalence:** Comparison of the `tool` field uses raw-string equality (Rust `Option<String>` equality). Two entries with `tool = '^Bash$'` and `tool = 'Bash'` are DISTINCT entries despite matching the same tool surface — the registry does NOT perform regex-equivalence detection. Operators MUST normalize their `tool` regex strings if they intend semantic deduplication."

2. **Implementation Notes section extended** with `### Equality Semantics — Raw-String, Not Regex-Equivalence (F-P3-003)` subsection: names `HashSet<(String, String, Option<String>)>` with derived `PartialEq` as the canonical implementation surface; explains that no regex compilation occurs before comparison; restates operator normalization obligation.

3. **Frontmatter:** `version: "1.4"` → `"1.5"`; `last_amended:` value unchanged (already 2026-05-08 from the same session day).

**POLICY 7 verification:** H1 heading unchanged — title remains authoritative as authored in v1.0.

**POLICY 1 verification:** No content removed. Invariant 7 text extended by appending the new sentence; Implementation Notes extended by appending a new subsection. All prior wording preserved verbatim.

## Amendment 2026-05-08 (v1.3 → v1.4 — F5 fix-burst-2 F-P2-011: Invariant 7 amended to (name, event, tool) tuple uniqueness — USER-APPROVED PATH A)

**Driver:** F5 pass-2 finding F-P2-011 — Invariant 7 defined `(name, event)` tuple uniqueness, which incorrectly prohibited a single named plugin (`protect-secrets`) from binding to two tool regex values (`Bash` and `Read`) on the same event (`PreToolUse`). This is a valid and intentional configuration: same purpose, two tool surfaces. Renaming the plugin entries would create artificial divergence. User approved Path A: amend the uniqueness predicate rather than rename plugins.

**Changes made:**

1. **Invariant 7 amended (append-only per POLICY 1 — no renumbering):** uniqueness constraint changed from `(name, event)` to `(name, event, tool)` tuple. Two entries MAY share `name` and `event` if they bind to different `tool` regex values. Load-time error updated to `RegistryError::DuplicateEntry { name, event, tool }`. Added note that `tool = None` (absent) is a distinct value for comparison purposes.

2. **`## Implementation Notes` section added** with `### (name, event, tool) Tuple Uniqueness Check — validate() Obligation` subsection: explicit guidance that `registry.rs::validate()` must implement the `(name, event, tool)` tuple uniqueness check with `RegistryError::DuplicateEntry`; CI lint plugin (`lint-registry-async-invariant`) should be verified/extended to cover this constraint.

3. **Canonical Test Vectors table extended** with three new rows: (a) protect-secrets-Bash + protect-secrets-Read (different `tool`) — valid; (b) identical `(name, event, tool=Bash)` — `DuplicateEntry` error; (c) identical `(name, event, tool=None)` — `DuplicateEntry` error. Per POLICY 12 (bc_tv_emitter_consistency).

4. **Traceability table:** VP-077 Harness 1 Amendment Obligation row added — flags that VP-077 Harness 1 `kani::assume` precondition for entry uniqueness must be updated to `(name, event, tool)` tuple. Architect handles VP-077 v1.8; this row is a PO sync note.

5. **Frontmatter:** `version: "1.3"` → `"1.4"`; `last_amended:` updated to `2026-05-08`.

**POLICY 4 verification (semantic anchoring):** Confirmed via `registry.rs` line 168 that `pub tool: Option<String>` is a first-class field on `RegistryEntry`. The `(name, event, tool)` tuple uniqueness check is semantically grounded in the existing data model. No new field is required.

## Amendment 2026-05-07 (v1.2 → v1.3 — F3 pass-2 fix burst follow-up)

Addresses F3 adversarial pass-2 finding: Postcondition 7 and Invariant 5 Layer 1 described a "pre-commit hook ... fails commit" mechanism that implied a Git `.git/hooks/pre-commit` file. The actual Layer 1 mechanism is a **Claude Code PostToolUse Edit|Write hook** registered in `hooks-registry.toml` that blocks the tool call at edit time (returning `block_intent = true`, exit code 2), before the edit lands in the working tree.

**Original wording implied:** Git pre-commit hook (`.git/hooks/pre-commit` or lefthook) that fails a commit.

**Actual mechanism:** Claude Code PostToolUse Edit|Write hook (same lifecycle as existing block-mode hooks in `hooks-registry.toml`, e.g., `validate-template-compliance`, `validate-factory-path-root`, `brownfield-discipline`). These hooks fire on every Edit or Write tool call and can block the tool call before any file change occurs.

**Conceptual intent preserved:** The intent of "catches violations before they reach CI" is identical — PostToolUse Edit|Write enforcement is even earlier than git pre-commit, blocking edits before they reach git's working tree at all. ADR-019 §Decision 4 uses "pre-commit hook" as a generic term for "fires before any commit attempt is finalized"; Postcondition 7 and Invariant 5 Layer 1 now reflect the correct technical mechanism.

**WASM migration note:** Per user decision 2026-05-07, new plugins must be implemented as native WASM plugins. The lint plugin for this invariant will be implemented as a WASM plugin registered as PostToolUse Edit|Write with `on_error = "block"` — the same registration pattern as legacy bash block-mode hooks, but using native WASM instead of the legacy-bash-adapter.

**Changes:** Postcondition 7 rewritten; Invariant 1 citation updated from "pre-commit" to "edit-time PostToolUse hook"; Invariant 5 Layer 1 bullet rewritten with full mechanism description and ADR-019 terminology note.

**ADR-019 §Decision 4 status:** ADR-019 uses "pre-commit hook" as a generic/conceptual label. The ADR does NOT require Git-native pre-commit hooks — it describes the intent (defense before CI) and defers to the implementation. No ADR-019 amendment is needed; the ADR's conceptual intent is fulfilled by the Claude Code PostToolUse Edit|Write hook mechanism.

## Amendment 2026-05-07 (v1.2 — F2 pass-2 fix burst)

Addresses adversary pass-2 findings F-P2-001, F-P2-006, F-P2-013.

**F-P2-001 (Plugin-name uniqueness invariant)**: Added Invariant 7. The correct uniqueness constraint is on the (`name`, `event`) tuple, not `name` alone. The actual registry has intentional duplicate names across different events (e.g., `worktree-hooks` for WorktreeCreate and WorktreeRemove, `protect-secrets` for Bash and Read). Invariant 7 scopes uniqueness to the tuple and documents this correctly. Architect must update VP-077 to cite Invariant 7 (not Invariant 1) for the name-uniqueness property.

**F-P2-006 (Incomplete Invariant 6)**: Added `track-agent-start`, `track-agent-stop`, and `session-learning` to the async=true required list — all three are telemetry-only and always return Continue. `warn-pending-wave-gate` (Stop) and `regression-gate` (PostToolUse) are classified SYNC: both emit human-visible stderr warnings that must reach the user reliably; async classification would silently truncate those warnings at dispatcher process exit. Classification determined by reading plugin source (`lib.rs`) — both call `write_stderr`/`eprint!` and always return `HookResult::Continue`. ADR-019 Consequences should be updated to note that `warn-pending-wave-gate` and `regression-gate` are deliberately SYNC with `on_error=continue`. (Flag for architect to sync ADR-019.)

**F-P2-013 (Postcondition 3 framing)**: Postcondition 3 reworded to remove "preserves the behavior of every validator and governance plugin" framing. ADR-019 prohibits backward compat shim. New wording: "Within a `schema_version = 2` registry, entries that do not declare `async` are parsed as `async = false`. Operators migrating from v1 must bump the schema_version header."

## Amendment 2026-05-07 (v1.1 — F2 pass-1 fix burst)

Addresses adversary pass-1 findings F-P1-003, F-P1-004, F-P1-005, F-P1-006, F-P1-010, F-P1-016.

**F-P1-006 (Subsystem anchor)**: Frontmatter `subsystem` changed from `SS-07` to `SS-01`. The runtime enforcement (`registry.rs::validate()`) lives in SS-01. SS-07 remains as secondary subsystem in the Architecture Module field. Per adversary option (b): keep one BC, choose primary subsystem reflecting the runtime enforcement mechanism. Architecture Module field updated to document both subsystems with rationale.

**F-P1-004 / F-P1-011 (Fail-closed)**: Postcondition 1 and Error Paths row 1 amended. Schema-version mismatch is now explicitly **exit code 2 (fail-closed)** with explicit stderr diagnostic. Previous text "exit per BC-1.08.001 fail-open convention" was contradictory with "hard error." Same fix applied in BC-1.14.001 and BC-1.08.001.

**F-P1-005 (Pre-commit layer)**: Postcondition 7 (pre-commit) added. Previously BCs documented only two defense layers (load-time + CI); ADR-019 §Decision 4 mandated three. Invariant 5 added to enumerate all three layers (pre-commit, registry-load-time, CI-PR) explicitly.

**F-P1-016 (Classification as invariant)**: Postcondition 7 (old classification table) promoted to Invariant 6. The specific named plugins that MUST be `async = true` are now invariant-level (catching future flips), not just a postcondition. Added `worktree-hooks` and `tool-failure-hooks` to the list per F-P1-016 adversary finding. Note to architect: VP-078 needs a positive-classification verification harness (Harness 3) per F-P1-016 — this is a cross-burst dependency on architect's VP-078 work.

**F-P1-010 (Story Anchor)**: Story Anchor updated from "Story A + Story B + Story D" to "TBD — single story per ADR-019 §6 (no phased rollout, user decision 2026-05-07)". Same change in Traceability Stories field.

**F-P1-003 (DI-014 citation)**: L2 Domain Invariants updated from "TBD" to DI-014 with explanation of the BC-7 enforcement arm.
