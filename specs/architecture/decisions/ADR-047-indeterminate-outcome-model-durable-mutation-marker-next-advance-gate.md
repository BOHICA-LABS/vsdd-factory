---
document_type: architecture-decision-record
level: L3
adr_id: ADR-047
version: "1.7"
title: "ADR-047: INDETERMINATE Outcome Model — First-Class Cannot-Complete Outcome, Durable Mutation Marker, and Next-Advance Gate (Three-Layer Validation Integrity Architecture)"
status: accepted
date: 2026-08-30
producer: architect
timestamp: 2026-08-30T00:00:00Z
deciders:
  - architect
subsystems_affected: [SS-01, SS-03, SS-04, SS-07]
supersedes: null
superseded_by: ADR-048
extends: ADR-039
traces_to: .factory/specs/architecture/ARCH-INDEX.md
last_amended: "2026-10-07 (v1.7) — Erratum-class factual refresh (architect; wasmtime security move per ADR-035 v1.2): Decision 1 implementation note's version statement updated (project now on wasmtime 48.x, >= 48.0.4; the `Trap::OutOfFuel`/`Trap::Interrupt` variants are unchanged). No decision altered. [Prior: 2026-09-05 (v1.6) — Erratum-class plugin-name correction to §Decision 8a's Cohort B table (S-25.02 F2 architecture elaboration, architect-authored per orchestrator dispatch, co-amended in the same burst as ADR-051; NOT a POLICY 22 design/security-model change — no decision content, threshold, or normative prescription is altered, using the same erratum-class amendment process ADR-047 v1.5 itself set as precedent): the non-existent plugin name `validate-burst-log-structure` is corrected to the actual registered plugin `validate-burst-log` (specifically its `event=\"PostToolUse\" tool=\"^(Edit|Write|MultiEdit)$\"` arm, which scans burst-log.md content; the plugin's separate `tool=\"^Bash$\"` arm is an unrelated, exec-free git-commit chain-detection gate and is explicitly out of Cohort B scope). S-25.02's own AC-006 carries the same citation drift and is flagged as a product-owner BC-authorship input in ADR-051 and the companion S-25.02 F2 architecture-delta doc; this ADR does not modify the story body.]"
modified:
  - "2026-08-30 (v1.0) — Initial authoring"
  - "2026-08-30 (v1.1) — Human ratification: D9 extended gate to git commit/push Bash arm; D8a confirmed as-authored"
  - "2026-08-30 (v1.2) — Status section correction: ADR acceptance does not open F4 gate; Cohort A reduced to three human-confirmed validators; validate-cross-site-correspondence moved to Cohort B"
  - "2026-08-30 (v1.3) — Cohort A A-immediate/A-deferred partition; Integration Ordering corrected; tool pattern factual fix for validate-factory-path-staging; consistent with S-25.01 AC-016 v1.1"
  - "2026-09-03 (v1.4) — Factual correction: Layer-1 effective fail-closed count corrected from ONE to ZERO for validate-factory-path-staging (structural PreToolUse/PostToolUse marker-write mismatch, BC-1.18.001 INV4); EFFECTIVE-NOW label softened to avoid overclaiming enforcement; v1.3 tool-pattern-fix changelog entry completed (event-type half was already correct in the body, only the changelog description was incomplete); ZERO-enforcement gap anchored to recommended follow-up story S-25.04. Not a POLICY 22 change — pure factual correction, no design content altered."
  - "2026-09-04 (v1.5) — Erratum-class clarification note appended to §Decision 8a's 'Known Gap' paragraph: corrects the loose 'artifact-write side' phrasing to cite BC-4.16.001 as validate-factory-path-staging's authoritative narrow git-staging-exclusivity behavior (not a content validator), and identifies the Known Gap's closure as new companion validate-factory-path-staged (BC-4.16.002, SS-04/CAP-034) under the human-ratified BROAD (unconditional, no command-text pre-filter) trigger scope. Not a POLICY 22 change — no decision content altered, wording/citation correction only."
  - "2026-09-05 (v1.6) — Erratum-class plugin-name correction to §Decision 8a's Cohort B table: `validate-burst-log-structure` (non-existent) corrected to `validate-burst-log` (Edit/Write arm only). Discovered during S-25.02 F1 delta analysis; co-amended alongside new ADR-051 (Layer-2 two-mechanism shard-rotation architecture). Not a POLICY 22 change — no decision content, threshold, or normative prescription altered, citation correction only."
  - "2026-10-07 (v1.7) — Erratum-class factual refresh: Decision 1 implementation note wasmtime version statement 46.0.2 -> 48.x (>= 48.0.4) per ADR-035 v1.2 security move (RUSTSEC-2026-0316/0314/0321/0322/0323/0324/0327); no decision altered"
---

# ADR-047: INDETERMINATE Outcome Model — First-Class Cannot-Complete Outcome, Durable Mutation Marker, and Next-Advance Gate

## Status

**ACCEPTED — Human-Ratified 2026-08-30** (POLICY 22 — ratification complete. D9 amended by
human decision to extend the gate to `git commit`/`git push` Bash dispatches, closing the
durable-propagation boundary. D8a confirmed as-authored per v1.1.)

ADR-047 acceptance clears the ARCHITECTURE decision only. It does NOT open the F4 (TDD)
implementation gate for S-25.01. Layer-1 (S-25.01) may proceed to F4 ONLY after ALL of:
(a) the full spec package (BC-1.18.001–BC-1.18.004 + BC-3.08.001 amendment + VP-102–VP-106
+ story S-25.01) is authored; (b) a fresh-context consistency audit passes; AND (c) the
orchestrator explicitly confirms the F3 spec gate. ADR ratification is a prerequisite for
F4, not a sufficient condition.

POLICY 22 ratification (2026-08-30) is to be recorded authoritatively in the decision-log
(D-NNN) by state-manager in the spec-burst commit. The ADR frontmatter `status: accepted`
reflects the architectural decision; the decision-log entry is the authoritative ratification
record.

This ADR EXTENDS ADR-039 (Validator failure policy for resource exhaustion). ADR-039 §Decision 1
established the `failure_policy` field and the resource-exhaustion enforcement axis. ADR-047 adds:
(1) a named INDETERMINATE outcome class on top of the existing detection infrastructure;
(2) a durable mutation marker for fail-closed INDETERMINATE events; and
(3) a next-advance gate that blocks state-advancing dispatch while the marker exists.

ADR-047 does NOT amend ADR-039's normative content. ADR-039 remains the authoritative source for
the `failure_policy` field schema, the calibration prerequisites (§Decision 3), and the
axes-independence invariant (§Decision 1). The two ADRs are read together.

---

## Context

### Operational Forensics

PostToolUse WASM validators run in fuel-bounded and epoch-bounded sandboxes. Forensic analysis of
the dispatcher event log reveals:

- Approximately 11,262 `plugin.timeout { cause: Fuel }` events (fuel exhaustion)
- Approximately 480 `plugin.timeout { cause: Epoch }` events (wall-clock timeout)
- 167 `host_fn_returned_output_too_large` events (host function returning `OutputTooLarge = -3`)
- Approximately 455 events where the entire validator suite wiped out together on a single artifact edit

Because all these hooks are PostToolUse with `failure_policy = "fail-open"` (current default for
all ~76 live plugins), a non-completing validator is treated as PASS. **State mutates UNVALIDATED,
silently.** The `regression-gate` plugin failed to persist its own state file 22 times due to
OutputTooLarge returns.

Current mitigations (fuel cap 10M→20M per ADR-042; CLAUDE.md prose size budgets; agent-side
compaction awareness) are symptom treatment and LLM-compensation. The human has directed a
mechanistic fix where the runtime and data structures enforce integrity — never the agent.

### The OutputTooLarge Detection Gap

`OutputTooLarge (-3)` is returned from HOST FUNCTIONS to the plugin. The plugin then decides its own
exit code. With the current implementation the dispatcher cannot distinguish "plugin saw
OutputTooLarge and correctly returned exit_code=1 to block" from "plugin saw OutputTooLarge and
silently returned exit_code=0, completing without blocking." This means a plugin that fails to read
a required input — not because it validated the write and found it clean, but because it could not
read the input at all — registers as PASS. This is exactly the CWE-636 (Fail Open) failure mode
that ADR-039 identified for fuel exhaustion, now applied to the OutputTooLarge axis.

### Terminology Reconciliation with ADR-039

ADR-039's existing codebase uses:
- Registry field: `failure_policy = "fail-closed"` (TOML) / `FailurePolicy::FailClosed` (Rust)
- Existing plugin result variants: `PluginResult::Timeout { cause: TimeoutCause::Fuel }`,
  `PluginResult::Timeout { cause: TimeoutCause::Epoch }`, `PluginResult::Crashed { .. }`

ADR-047 introduces **INDETERMINATE as a SEMANTIC OUTCOME LABEL**, not a new `PluginResult` variant
and not a new registry field. INDETERMINATE is a classification the executor applies AFTER observing
a cannot-complete signal from the underlying `PluginResult`. No new `on_indeterminate` field is
introduced — the existing `failure_policy` field already covers the routing semantics.

### Three-Layer Architecture Summary

Layer 1 (this ADR; S-25.01): Make cannot-complete fail-LOUD. INDETERMINATE outcome class,
durable marker, next-advance gate.

Layer 2 (S-25.02; REGISTERED BACKLOG): Continuous size-triggered sharding of append-only cycle
artifacts into capped shards with fuel-derived shard cap. Removes the dark zone BY CONSTRUCTION.

Layer 3 (S-25.03; REGISTERED BACKLOG): Bounded validator windows reading from shards. Regression-
gate state-file bounded rotation.

---

## Decisions

### Decision 1 — Outcome Trichotomy: PASS / FAIL / INDETERMINATE

The dispatcher's outcome classification is extended from a binary (PASS / FAIL) to a trichotomy:

| Outcome | Meaning | Triggering conditions |
|---------|---------|----------------------|
| **PASS** | Plugin completed; found no blocking condition | `PluginResult::Ok { exit_code: 0, .. }` AND NOT `host_output_too_large_seen` (or `failure_policy = fail-open` and OutputTooLarge seen) |
| **FAIL** | Plugin completed; found a blocking condition | `PluginResult::Ok { exit_code: non-zero, .. }` (per AMD-003 on `on_error=Block` path) OR `PluginResult::Crashed` / `PluginResult::Timeout` with `on_error=Block` (existing semantics — unchanged) |
| **INDETERMINATE** | Plugin could NOT complete | Any of: (a) `PluginResult::Timeout { cause: TimeoutCause::Fuel }`, (b) `PluginResult::Timeout { cause: TimeoutCause::Epoch }`, (c) `PluginResult::Ok { exit_code: 0, .. }` WITH `host_output_too_large_seen == true` |

INDETERMINATE is **mutually exclusive** with PASS and FAIL for the purposes of `failure_policy`
routing:

- INDETERMINATE + `failure_policy = fail-closed` → durable marker + next-advance gate block
- INDETERMINATE + `failure_policy = fail-open` → advisory `plugin.indeterminate` event only

PASS and FAIL semantics and all existing `on_error` block/continue semantics are UNCHANGED.
INDETERMINATE is a third classification layer that does not alter the PASS/FAIL paths.

### Decision 2 — Registry Field Reuse: `failure_policy`; No New Field

The registry field introduced by ADR-039 §Decision 1 is the complete specification for routing
INDETERMINATE outcomes. There is NO new `on_indeterminate` field.

`failure_policy = "fail-closed"` means: the plugin is an authorization-class gate; if it cannot
complete (INDETERMINATE), the dispatcher must treat the unvalidated mutation as suspect and
activate the durable marker + next-advance gate.

`failure_policy = "fail-open"` (default if field is absent) means: INDETERMINATE is advisory-only
— a `plugin.indeterminate` event is emitted but no marker is written and no gate is triggered.

This reuse preserves backward compatibility exactly: all existing plugins with absent
`failure_policy` field (currently all ~76 production plugins) remain fail-open, producing no
observable behavior change beyond an additional advisory event for INDETERMINATE outcomes where
they previously only emitted `plugin.timeout`.

The axes-independence invariant from ADR-039 §Decision 1 is preserved: `on_error` and
`failure_policy` govern orthogonal failure modes and are not unified.

### Decision 3 — Durable Unvalidated-Mutation Marker

When a plugin with `failure_policy = "fail-closed"` produces an INDETERMINATE outcome on a
PostToolUse hook, the dispatcher MUST write an unvalidated-mutation marker file.

**Path:** `.factory/unvalidated-mutation.marker`

**Write mechanism:** Direct file write by the dispatcher's `indeterminate_marker` module
(a new module at `crates/factory-dispatcher/src/indeterminate_marker.rs`). Write is atomic
(write to a temp file in the same directory, then rename).

**Required fields (TOML-formatted content):**

```toml
timestamp = "<ISO-8601 UTC timestamp of the INDETERMINATE event>"
plugin_name = "<name field from the [[hook]] entry in hooks-registry.toml>"
artifact_path = "<absolute path of the artifact that was written (from PostToolUse payload)>"
cause = "<one of: fuel | epoch | output-too-large>"
trace_id = "<dispatcher_trace_id of the event that produced INDETERMINATE>"
```

**Single-marker policy:** Only ONE marker file exists at a given time. If a second
INDETERMINATE+fail-closed event fires while a marker already exists, the dispatcher overwrites
the existing marker with the new event's details (last writer wins; a human who ignored the first
marker will see the most recent unvalidated mutation).

**Rationale:** A per-plugin marker file scheme (`.factory/unvalidated-mutation-<plugin>.marker`)
was considered but rejected at F1 as over-engineering for Layer 1. Layer 3 (bounded validator
windows) will reduce INDETERMINATE frequency so substantially that simultaneous multi-plugin
INDETERMINATE events will become rare. The single-marker policy is simpler and correct for the
immediate threat model.

### Decision 4 — Next-Advance Gate: `validate-unvalidated-mutation-marker` Plugin

A new native WASM plugin `validate-unvalidated-mutation-marker` is introduced.

**Registration in `hooks-registry.toml`:**
```toml
# Arm 1: Agent dispatch gate
[[hook]]
name = "validate-unvalidated-mutation-marker"
plugin = "hook-plugins/validate-unvalidated-mutation-marker.wasm"
event = "PreToolUse"
tool = "^Agent$"
on_error = "block"
async = false
failure_policy = "fail-open"  # the gate itself is fail-open; its PRESENCE is the protection

# Arm 2: git commit/push durable-propagation gate (D9 — human ratification amendment)
[[hook]]
name = "validate-unvalidated-mutation-marker-git"
plugin = "hook-plugins/validate-unvalidated-mutation-marker.wasm"
event = "PreToolUse"
tool = "^Bash$"
on_error = "block"
async = false
failure_policy = "fail-open"  # same fail-open posture as Arm 1; command-content filter in plugin
```

**Behavior:**
- Reads `.factory/unvalidated-mutation.marker`
- If marker EXISTS: return exit_code=2 (block) with a human-readable message containing:
  - The plugin that produced INDETERMINATE (from marker `plugin_name`)
  - The artifact path (from marker `artifact_path`)
  - The cause (from marker `cause`)
  - The recommended re-validation command: `cargo test --test <plugin_name>_integration` (or equivalent)
  - The manual escape hatch: `rm .factory/unvalidated-mutation.marker`
- If marker ABSENT: return exit_code=0 (pass)

**Arm 2 behavior (Bash dispatch, `validate-unvalidated-mutation-marker-git` entry):**
For Bash PreToolUse events, the plugin reads the `command` payload parameter:
- If `command` does NOT match `\bgit\b.*\b(commit|push)\b` (case-insensitive): return
  `exit_code=0` immediately (pass — only git commit/push Bash dispatches are gated)
- If `command` matches AND marker EXISTS: return `exit_code=2` (block) with the same recovery
  message as the Agent arm (plugin name, artifact path, cause, recommended recovery command,
  and the manual escape hatch: `rm .factory/unvalidated-mutation.marker`)
- If `command` matches AND marker ABSENT: return `exit_code=0` (pass)

The command pattern intentionally does NOT match `git status`, `git log`, `git diff`,
`git fetch`, or other non-mutating subcommands. Only `git commit` and `git push` trigger
the marker check — the exact operations that propagate mutations to the durable branch.

**Scope constraint:** This gate applies to `^Agent$` dispatches (Arm 1) and to Bash dispatches
whose `command` matches the `git (commit|push)` pattern (Arm 2). Read, Edit, Write, MultiEdit,
and non-git-commit/push Bash dispatches are NOT gated by this plugin. The gate is designed to
block the two state-advancing PIPELINE ACTIONS that advance durable state: Agent pipeline dispatch
and durable-branch propagation via git.

**Self-lock hazard:** This gate, when active, blocks ALL Agent dispatches including those that
would dispatch a re-validator. The escape hatch (`rm .factory/unvalidated-mutation.marker`)
is intentionally simple: an operator with shell/process-environment access can unblock the
session without requiring another Agent dispatch. This is the authenticated-by-possession
break-glass pattern from ADR-039 §Decision 3 — the same posture, applied to the marker gate.

**`failure_policy` for the gate itself:** The gate plugin is registered `failure_policy = "fail-open"`.
If the gate itself cannot complete (e.g., its fuel is exhausted reading the marker check), the
dispatch proceeds rather than self-locking unconditionally. This is deliberate: the INDETERMINATE
outcome model protects WRITES (PostToolUse), not reads; a fail-open gate that guards Agent
dispatch must not introduce a new unconditional self-lock class. The architectural invariant being
protected is write-integrity, not dispatch-integrity.

### Decision 5 — Marker Clear Protocol

The marker is cleared (deleted) by the dispatcher under either of two conditions:

**Condition A — Successful re-validation:**
When the dispatcher runs the same plugin that previously produced INDETERMINATE and that plugin
now produces PASS (exit_code=0, no `host_output_too_large_seen`) on the same artifact, the
dispatcher deletes `.factory/unvalidated-mutation.marker` for that plugin entry. The deletion is
idempotent: if the marker is absent at delete-time, the operation is a no-op (not an error).

**Condition B — Manual operator escape hatch:**
`rm .factory/unvalidated-mutation.marker` is a fully supported operator action. No special command
or credential is required. The marker's absence is sufficient to unblock the next Agent dispatch.
This matches the break-glass spirit of ADR-039 §Decision 3.

**Clear-on-different-plugin:** If a different plugin (not the one that wrote the marker) subsequently
runs on the same artifact and produces PASS, the marker is NOT cleared. Clearance is scoped to
the plugin that produced INDETERMINATE, not to the artifact in general.

**Rationale for not clearing on FAIL:** If the re-validating plugin produces FAIL (exit_code=1),
the write is blocked and the marker remains. The unvalidated state persists until either a
successful re-validation or manual clearance.

### Decision 6 — OutputTooLarge Detection: Per-Invocation Store Flag

**Problem statement:** `OutputTooLarge (-3)` is a return code from host functions to the plugin
WASM module. The plugin receives this code and decides whether to propagate it as a block
(exit_code=1) or swallow it (exit_code=0). The dispatcher, observing only the final exit code,
cannot distinguish "plugin validated successfully" from "plugin saw OutputTooLarge and silently
passed."

**Mechanism:**

A boolean flag `host_output_too_large_seen: bool` is added to the wasmtime `StoreData` struct
(or equivalent per-invocation dispatcher state that the executor can read after plugin completion).
This flag is initialized to `false` at the start of each plugin invocation.

Any host function that returns `OutputTooLarge = -3` to the plugin MUST set
`host_output_too_large_seen = true` on the Store before returning. Affected host functions
include at minimum: `read_file`, `read_prefix`, and any future host function that returns
i32 values to plugins.

After plugin completion, the executor checks: if `host_output_too_large_seen == true` AND
`plugin exit_code == 0` AND `failure_policy == FailClosed`, the outcome is classified
INDETERMINATE (not PASS). The cause field in the marker is set to `output-too-large`.

If `host_output_too_large_seen == true` AND `plugin exit_code == 0` AND
`failure_policy == FailOpen`, the outcome is still PASS (advisory `plugin.indeterminate` event
emitted with cause `output-too-large`, but no marker written — consistent with Decision 2).

If `host_output_too_large_seen == true` AND `plugin exit_code != 0`, the plugin correctly
propagated the error; outcome is FAIL per existing AMD-003 semantics.

**Per-invocation reset REQUIRED:** The `host_output_too_large_seen` flag MUST be reset to `false`
IMMEDIATELY BEFORE each guest (WASM module) invocation — not only at Store creation time. wasmtime
Stores record historical state for the lifetime of the Store; if a single Store is reused across
multiple plugin invocations (e.g., in a hypothetical future multi-invocation Store pool), a flag
set to `true` by invocation N would misclassify invocation N+1 as INDETERMINATE even if N+1's
host functions all returned successfully. The correct implementation: `store.data_mut().host_output_too_large_seen = false;`
immediately before `func.call(&mut store, ..)`. This is not a future-proofing concern only — it
is a correctness requirement for any implementation that reuses Store context across calls.

**Scope of StoreData extension:** The `host_output_too_large_seen` flag is per-invocation state.
It is NOT shared across invocations or persisted between dispatches. Each plugin invocation
starts with a fresh `false` flag (enforced by the pre-invocation reset above).

**SDK/ABI stability:** This flag lives inside the dispatcher's `StoreData` (the Rust struct that
the dispatcher uses when linking the WASM module). It is NOT part of the hook-sdk ABI
(`crates/hook-sdk/`). Plugin WASM binaries do NOT read or write this flag directly. The flag is
dispatcher-internal state only. Therefore this change requires NO host ABI version bump
(HOST_ABI_VERSION unchanged; no new host function; additive dispatcher-internal state).

**Why not defer OutputTooLarge to Layer 3?** Layer 3 reforms validator read patterns so validators
read bounded windows from sharded artifacts, eliminating the root cause of OutputTooLarge events.
However, deferring detection to Layer 3 would leave a known integrity gap in the Layer 1 release:
a keystone that fails-loud on fuel and epoch but silently passes on read-failure is not
production-grade. The Store-flag mechanism closes this gap at Layer 1 with minimal complexity
(one boolean flag per invocation, two lines of host-fn wrapper code, one executor-side check).

### Decision 7 — Backward-Compatibility Contract

| Existing Behavior | Behavior Under Layer 1 |
|------------------|----------------------|
| `on_error = "block"` semantics | UNCHANGED — crash + `on_error=block` still blocks current dispatch per AMD-003 |
| `on_error = "continue"` semantics | UNCHANGED |
| `failure_policy = "fail-open"` (current default for all ~76 plugins) | UNCHANGED — INDETERMINATE for fail-open plugins emits advisory `plugin.indeterminate` event; NO marker, NO gate |
| PASS outcome for `PluginResult::Ok { exit_code: 0 }` | UNCHANGED when `host_output_too_large_seen == false` (the common case for all current plugins) |
| FAIL semantics | UNCHANGED — FAIL is not redefined; INDETERMINATE is a THIRD class |
| `PluginResult::Timeout { .. }` and `PluginResult::Crashed { .. }` existing event emission | UNCHANGED — existing events still emitted; `plugin.indeterminate` is an ADDITIVE new event type |
| ADR-039 axes-independence invariant | PRESERVED — `on_error` and `failure_policy` remain orthogonal; INDETERMINATE does not interact with `on_error` |
| ADR-039 §Decision 6 test `fail_closed_timeout_with_on_error_continue_is_open` | UNCHANGED behavior — INDETERMINATE+fail-open remains advisory-only; this test still must not be deleted per ADR-039 §Decision 6 |

**Regression guard test:** `test_BC_1_18_004_fail_open_default_preserves_advisory_behavior` (new;
authored in S-25.01) explicitly asserts that a plugin with absent or `fail-open` `failure_policy`
that produces INDETERMINATE writes NO marker and triggers NO gate.

### Decision 8 — Fail-Closed Validator Assignments and Registered Future Phases

#### 8a — Fail-Closed Rollout Schedule: Partitioned by Input-Size Class

**CRITICAL CONSTRAINT (research-validated, F2 gate):** Fail-closed assignments are partitioned
into TWO cohorts based on whether the validator's inputs are BOUNDED or LARGE/UNBOUNDED.
Setting a large-artifact validator to `fail-closed` in Layer 1 — before Layer 2 sharding bounds
its inputs — would cause INDETERMINATE+fail-closed to fire immediately and repeatedly on normal
pipeline work, requiring constant manual marker-deletion. This replicates the human-as-cron
toil pattern (Google SRE: toil is repetitive work that does not yield permanent improvement)
that this feature is designed to eliminate. It would be a self-inflicted operational DoS on
our own pipeline.

**COHORT A — S-25.01 Layer 1 candidates (bounded-input; ADR-039 §D3 calibration also required):**

**Human-confirmed Cohort A membership (v1.1 ratification, corrected in v1.2): EXACTLY THREE validators —
`validate-pr-merge-prerequisites`, `validate-wave-gate-prerequisite`, `validate-factory-path-staging`.
No other validator is in Cohort A unless explicitly confirmed by the human at the F3 spec gate.**

Cohort A is further partitioned into two operational sub-groups (v1.3 correction; consistent with
S-25.01 AC-016):

**Cohort A-immediate** — ASSIGNED-NOW in S-25.01 (fail-closed config bit set unconditionally; no
S-21.24 dependency for the assignment — corrected v1.4; previously labeled "EFFECTIVE-NOW",
which overclaimed live enforcement. The assignment is safely SET; it is NOT currently ENFORCED.
See "Enforcement note" below the table and "Why the partition?" for why):

| Plugin | Event/Tool | Input Size Class | ADR-039 §D3 Calibration Status | Layer-1 Safe to SET? |
|--------|-----------|------------------|-------------------------------|---------------------|
| `validate-factory-path-staging` | PreToolUse `^Bash$` | **Bounded** — reads file path from Bash tool params | Calibration confirmed; `on_error = continue` (no self-lock risk even after S-21.24 enforcement) | YES — unconditional |

**Enforcement note (v1.4 correction):** "Safe to SET" is not the same claim as "effective." This
assignment currently produces ZERO observable enforcement effect — see "Why the partition?" and
"Layer-1 effective fail-closed count" immediately below, and the "Known Gap" note at the end of
this subsection.

**Cohort A-deferred** — SET-BUT-LATENT in S-25.01 (fail-closed SET in registry with a comment;
enforcement conditional on S-21.24 confirming calibration + S-21.23 break-glass before wiring
ADR-039 Phase 4 executor enforcement):

| Plugin | Event/Tool | Input Size Class | ADR-039 §D3 Calibration Status | Layer-1 Safe to SET? |
|--------|-----------|------------------|-------------------------------|---------------------|
| `validate-wave-gate-prerequisite` | PreToolUse `^Agent$` | **Bounded** — reads STATE.md header/phase only | **Gated on S-21.24** (HELD Wave-7 cascade; S-21.21 calibration values + S-21.23 break-glass required) | YES to SET with latent comment; NO to enforce before S-21.24 |
| `validate-pr-merge-prerequisites` | PreToolUse `^Agent$` | **Bounded** — reads PR metadata only | **Gated on S-21.24** (HELD Wave-7 cascade; S-21.21 calibration values + S-21.23 break-glass required) | YES to SET with latent comment; NO to enforce before S-21.24 |

**Why the partition?** `validate-factory-path-staging` has `on_error = "continue"` — even when
S-21.24 wires the ADR-039 Phase 4 executor enforcement, this validator cannot self-lock the session
(a fuel-exhausted `on_error=continue` validator never blocks the current dispatch). The two
`^Agent$` validators have `on_error = "block"`: once S-21.24 wires enforcement, a fuel-exhausted
`^Agent$` PreToolUse validator blocks the current Agent dispatch, risking self-lock if inputs are
not bounded. S-21.24's own dependency chain (S-21.19 → S-21.21 calibration → S-21.23 break-glass)
ensures this risk is resolved before enforcement is wired — but that only happens at S-21.24 merge,
not at S-25.01 merge.

**Layer-1 effective fail-closed count at S-25.01 merge: ZERO** (corrected v1.4; misstated as ONE
in v1.0–v1.3). `validate-factory-path-staging` sets `failure_policy = "fail-closed"`
unconditionally and is safe to set (no self-lock risk, per "Why the partition?" above), but the
assignment is inert: the durable-marker write path (Decision 3; BC-1.18.001 invariant 4) fires
ONLY on a PostToolUse dispatch, and `validate-factory-path-staging` is registered PreToolUse
`^Bash$` — it structurally can never reach `write_indeterminate_marker`. Combined with
`on_error = "continue"` (which, as established above, means it can never block the current
dispatch either, present or future) and its absence from the ADR-039 §Decision 2 six-validator
exhaustion-leg current-dispatch-blocking roadmap (§8b's S-21.19→S-21.24 chain does not name this
validator), the assignment produces NO observable enforcement effect different from
`failure_policy = "fail-open"` — an INDETERMINATE outcome on this validator degrades to the same
advisory-only `plugin.indeterminate` event either way. The two `^Agent$` validators are present
in the registry with `failure_policy = "fail-closed"` and a latent comment but likewise produce
no enforcement effect until S-21.24 activates it — so, at S-25.01 merge, all three Cohort A
validators currently deliver ZERO effective enforcement. The distinction between Cohort A-immediate
and Cohort A-deferred is WHEN (or, for `validate-factory-path-staging`, WHETHER AT ALL) that
changes — not whether either enforces today.

**Known Gap — `validate-factory-path-staging` has no live or currently-scoped enforcement path:**
This is a genuine coverage gap in Layer 1's "make cannot-complete fail-LOUD" guarantee, for the
one Cohort A validator originally advertised as immediately effective. The human has directed
that this gap be closed via a dedicated follow-up story rather than silently accepted (CLAUDE.md
Canonical Principle Rule 3 — a tech-debt-register entry requires an explicit future-story anchor,
not a bare deferral). Closure is tracked to follow-up story **"Close validate-factory-path-staging
zero-enforcement gap — real Layer-1 production trigger"** (recommended ID S-25.04, the next
available slot under Epic E-25 as of this amendment; story-writer owns final ID allocation and
authoring). Closure options for that story to evaluate include: (a) a new PostToolUse companion
validator that actually reaches `write_indeterminate_marker` for `.factory/` path-staging
mutations; (b) changing this validator's `on_error` from `continue` to a blocking variant (with a
self-lock-risk re-analysis, since the "no self-lock risk" finding above depends on `continue`);
and/or (c) extending the ADR-039 §Decision 2 six-validator exhaustion-leg roadmap to add this
validator as a seventh member. This ADR does not select among these options; that is the
follow-up story's F1/F2 scope.

**Clarification (added at S-25.04 F2 architecture elaboration — erratum-class, no decision
content altered):** The phrase "artifact-write side" used above to describe option (a)'s
companion validator is informal shorthand and should not be read as implying
`validate-factory-path-staging` validates the *content* of a `.factory/` artifact write.
`validate-factory-path-staging`'s authoritative behavior is defined by **BC-4.16.001**: a narrow
git-staging exclusivity guard (INV-E21-001, CAP-034) that blocks `git add`/`git stage` of
`.factory/` paths on product branches at PreToolUse time. It has no artifact-write-content
validation role. Option (a)'s companion validator is correspondingly a **post-hoc git-staging
detective mirror**: it inspects the actual git index and current-branch state after a Bash
command completes (`git diff --cached --name-only` + `git branch --show-current`, the same
`host::exec_subprocess` pattern `validate-factory-path-staging` itself already uses) — not
artifact-write content. This closure is delivered by the new companion plugin
`validate-factory-path-staged` (BC-4.16.002, SS-04, CAP-034), registered `event = "PostToolUse"`,
`tool = "^Bash$"`, running its internal `git diff --cached`/`git branch --show-current` check
unconditionally on every completed Bash dispatch — the human-ratified BROAD trigger scope (no
command-text pre-filter), selected over a narrower text-gated alternative precisely because a
text pre-filter would reintroduce the same under-match residual-risk class (BC-4.16.001's own
Accepted Residuals) this detective mirror exists to close. See BC-4.16.002 (SS-04, CAP-034) for
the companion's formal contract and S-25.04's F1 Delta Analysis §0 for the full correction record.

**COHORT B — Layer 2 S-25.02 candidates (large-artifact or boundedness-unconfirmed; flip ONLY after sharding bounds inputs OR explicit human Cohort A confirmation at F3 gate):**

These validators either scan large, append-only cycle artifacts (decision-log.md, burst-log.md,
lessons.md, STATE.md body) OR have not received explicit human confirmation of bounded inputs.
Setting them fail-closed before Layer 2 sharding — or without explicit human confirmation —
risks an INDETERMINATE+marker feedback loop on normal pipeline writes:

| Plugin | Event/Tool | Reason for Layer-2 Deferral |
|--------|-----------|---------------------------|
| `validate-burst-log` (Edit/Write/MultiEdit arm only — corrected v1.6, erratum: was misnamed `validate-burst-log-structure`, which does not exist in `hooks-registry.toml`; the plugin's separate `^Bash$` arm is an unrelated git-commit chain-detection gate, NOT part of Cohort B) | PostToolUse `^(Edit\|Write\|MultiEdit)$` | Scans burst-log.md — fuel-exhausts at current sizes; Layer 2 shard cap eliminates this |
| `regression-gate` | PostToolUse `^(Edit\|Write)$` | Scans STATE.md body + large cycle artifacts; failed 22× writing its own state file; Layer 2 bounds both the input and the state-file write |
| `convergence-tracker` | PostToolUse `^(Edit\|Write)$` | Scans convergence history across multiple large cycle files; Layer 2 shard cap required first |
| `validate-cross-site-correspondence` | PostToolUse `^(Edit\|Write\|MultiEdit)$` | **Demoted from Cohort A in v1.2:** Human-confirmed Cohort A set is exactly three validators; this validator was NOT in the human-confirmed set. Boundedness relative to large cycle artifacts (decision-log.md/burst-log.md/lessons.md) is not verified in F2 scope — the validator scans `modified[]` arrays and cross-site parity across spec files and fired on E-9/19/21/22 this cycle, but whether those scan paths reach the large growing cycle artifacts is unconfirmed. Demoted per default-safer rule (when in doubt, Cohort B). Re-evaluate at F3 spec gate: if human confirms inputs are bounded to spec-file frontmatter only and exclude large cycle artifacts, promote to Cohort A. |

**Monitoring (all cohorts):** The `plugin.indeterminate` event MUST be explicitly EXCLUDED from
any pass-rate metric computation in the dispatcher telemetry layer. Including INDETERMINATE events
in a pass-rate metric replicates the silent-success failure mode at the metrics layer (the INDETERMINATE
outcome exists precisely because the pass rate was falsely inflated). A non-completing invocation
is NOT a pass; it is a data gap. Absence-of-completion is itself alertable — the ~455 whole-suite-
wipeout events show that absence-of-validation can signal an entire write being unguarded.

**Retry guard:** Any auto re-validation retry triggered by successful re-validation detection MUST
be bounded and backed-off. An unbounded retry loop on large-artifact writes defeats the Layer 1
protection model by draining the session fuel budget.

**F2 Gate Human Decisions Required:**
1. Confirm the Cohort A / Cohort B partition (S-25.01 assigns Cohort A post-calibration; S-25.02 flips Cohort B).
2. Confirm single-marker-file policy (Decision 3) vs. per-plugin markers.
3. Confirm OutputTooLarge Store-flag detection (Decision 6) is accepted for Layer 1 complexity.
4. DECIDED (human ratification 2026-08-30) — gate extended to git commit/push Bash arm per human D9 amendment; see Decision 9 — Extended Gate Scope.
5. Confirm independent delivery (S-25.01 depends only on S-21.10; Cohort A PreToolUse Agent gates additionally require S-21.24 calibration).

#### 8b — Ratified Future Phases

The following are RATIFIED FUTURE PHASES of the three-layer validation-integrity architecture.
They are REGISTERED BACKLOG stories under Epic E-25, not tech-debt-register entries. They MUST NOT
be silently deferred.

**Layer 2 — S-25.02 — Artifact Sharding:**
Continuous size-triggered sharding of append-only cycle artifacts (decision-log, burst-log,
lessons) into capped shards plus an index.

Shard cap derivation (correct, production-grade formulation):
`shard_cap_bytes <= (PRACTICAL_FUEL_CEILING / WORST_CASE_FUEL_PER_BYTE) - MAX_SINGLE_RECORD_BYTES - SAFETY_MARGIN`
This accounts for: (a) super-linear validators (monotonicity-checking, cross-reference checks);
(b) per-shard metadata overhead; (c) margin for worst-case input. Average-case fuel-per-byte
is NOT the correct denominator — worst-case is.

Per-file bounding: the shard cap is applied PER FILE, not to an aggregate across all cycle
artifacts. A decision-log shard, a burst-log shard, and a lessons shard are independently
bounded; a write to one does not affect the shard state of the others.

Honest shard count accounting: shard COUNT is unbounded absent a retention/compaction policy.
Whole-corpus validators operating across shards are O(shards), NOT O(1). This is the correct
claim: the shard cap ensures each shard-level validation completes within fuel budget; the
total corpus validation time is proportional to shard count. A separate retention/compaction
policy for old shards (e.g., archive shards older than N cycles) is a REQUIRED companion to
Layer 2 to prevent unbounded shard accumulation. This companion policy is documented here as
a mandatory S-25.02 subtask, not deferred.

Roll-before-write policy: if a single appended block exceeds the shard cap, the append helper
MUST roll to a new shard BEFORE writing the block (not write an oversized shard). This is the
Kafka log-segment roll-before-write pattern (RFC 9162 — certificate transparency log
checkpoint discipline provides the same precedent for append-only bounded segments).

Rotation trigger: a deterministic size rule (PreToolUse hook checking shard size, or an append
helper that checks size pre-write). No LLM-side awareness required. Dependencies: [S-25.01].

**Layer 3 — S-25.03 — Bounded Validator Windows:**
Validators read BOUNDED WINDOWS from shards rather than whole files. Two categories:

**Window-local validators (safe — no cross-shard state needed):**
- `validate-closes-completeness` → current burst shard only (Closes blocks are local to a burst)
- `validate-factory-path-staging` → tool params (already bounded at Layer 1 Cohort A)

**Whole-corpus validators requiring trusted-boundary-checkpoint carry-forward:**
Cross-shard invariants (e.g., changelog monotonicity — sequence numbers must increase across
shard boundaries; trajectory tail — the banner must match the prior shard's last entry) are
sound for WHOLE-CORPUS invariants ONLY with a trusted-boundary-checkpoint carried forward from
the prior SEALED shard. Each sealed shard's checkpoint record contains:
`(last_validated_seq, last_shard_id, last_shard_digest, summary)`.
The validator for shard N reads the checkpoint from shard N-1 (sealed) and validates the seam.
This follows RFC 9162 (Certificate Transparency checkpoint-carry-forward) and the RocksDB
two-layer model: per-file local check + separate global manifest check.

Retroactive edits: our cycle performs retroactive remediations. If a remediation edits a
SEALED shard, that shard must be re-validated and its checkpoint recomputed; downstream shard
seam validations also re-run. This MUST be explicit in the S-25.03 spec — a "sealed shard"
is not immutable in our cycle, and the validator design must handle re-sealing.

Specific validator migrations:
- `regression-gate` → bounded state file per-rotation window + state-file write reformatted
  to bounded/rotated + fail-loud (state file itself is a shard under Layer 3 rules)
- `validate-changelog-monotonicity` → head rows of the current shard + checkpoint carry-forward
- `validate-trajectory-tail` → banner of STATE.md (bounded by Section 1; already bounded)

Dependencies: [S-25.02].

### Decision 9 — Extended Gate Scope: Agent Dispatch AND git commit/push

The `validate-unvalidated-mutation-marker` gate is extended to block BOTH durable-propagation
paths while `.factory/unvalidated-mutation.marker` exists:

**(a) `^Agent$` PreToolUse dispatch** (Decision 4 Arm 1): blocks all Agent tool dispatches while
the marker is present. This is the primary quarantine-forward mechanism for pipeline state-advance
actions.

**(b) `git commit` and `git push` via `^Bash$` PreToolUse** (Decision 4 Arm 2 — added by this
D9 human ratification amendment): blocks Bash dispatches whose `command` parameter matches
`\bgit\b.*\b(commit|push)\b` while the marker is present. Non-matching Bash dispatches proceed
unconditionally.

**Rationale:** `.factory/` artifacts reach the `factory-artifacts` durable branch exclusively via
`git commit` and `git push` issued through the Bash tool. An INDETERMINATE-tainted artifact CAN be
committed and pushed via Bash BEFORE re-validation if the gate covers only Agent dispatch. Extending
the gate to the `git commit`/`git push` path closes the actual propagation boundary. Layer 1
remains a DETECTION and QUARANTINE-FORWARD layer (PostToolUse is inherently detective — the write
has occurred before PostToolUse fires; NIST SA-8(24) "Fail Secure" requires blocking until
validation is confirmed), and the quarantine now covers the complete state-advancing surface:
Agent dispatch AND durable-branch propagation.

**Unblocked tool surface:** The following dispatches are NOT gated and proceed unconditionally
regardless of marker state:
- Bash dispatches whose `command` does NOT match `\bgit\b.*\b(commit|push)\b` (reads,
  diagnostics, non-mutating git subcommands such as `git status`, `git log`, `git diff`,
  `git fetch`)
- Read, Edit, Write, MultiEdit dispatches
- All other tool types

Routine diagnostic and authoring work is not frozen while the marker is active.

**Marker-clear protocol** (Decision 5 — unchanged): `rm .factory/unvalidated-mutation.marker`
unblocks both gate arms simultaneously. The successful-re-validation clear (Condition A) also
unblocks both arms. No separate clear action is required per arm.

**BC-1.18.002 anchor:** BC-1.18.002 (to be authored by product-owner in F3) MUST state
postconditions covering BOTH gate arms: (a) marker present → Agent dispatch blocked; (b) marker
present AND `git commit`/`git push` command → Bash dispatch blocked; (c) marker absent → both
arms pass. This ADR is the authoritative anchor for BC-1.18.002's scope. VP-105 provides the
mechanically verifiable property specification covering both arms (Rust unit + bats).

**Prior v1.0 "accepted residual risk" framing superseded:** The v1.0 draft designated the Bash
commit/push path as accepted residual risk (Layer 1.5 future story). That framing is superseded
by the D9 human ratification amendment. No Layer 1.5 backlog story is registered for this
capability; the Bash arm is delivered in S-25.01 alongside the Agent arm.

### Implementation Note — wasmtime Trap Variant Dispatch (Decision 1 Mechanics)

The executor's classification of `PluginResult::Timeout` causes (fuel vs epoch) MUST use
the wasmtime `Trap` variant downcast, NOT a post-invocation `get_fuel()` check:

- **Fuel exhaustion:** `trap.downcast_ref::<Trap>() == Some(Trap::OutOfFuel)` (wasmtime
  `Trap::OutOfFuel` variant, stable across wasmtime 20.x–48.x; the project is on 48.x (>= 48.0.4) per ADR-035 §Decision 6 v1.2, formerly 46.0.2).
- **Epoch timeout:** `trap.downcast_ref::<Trap>() == Some(Trap::Interrupt)` (wasmtime
  `Trap::Interrupt` variant; corresponds to epoch deadline exceeded).
- **Neither:** Any other `Trap` variant is NOT an INDETERMINATE signal for the
  fuel/epoch axis; route to existing `on_error` handling.

`get_fuel()` is NOT authoritative for fuel-exhaustion detection when both fuel and epoch are
enabled simultaneously — the remaining fuel counter is only supplementary in that mode. The Trap
variant is the authoritative signal.

`Trap` is `#[non_exhaustive]` (24 variants at time of writing). The match on Trap variants
MUST include a wildcard arm `_ => { /* not an INDETERMINATE Trap; route to on_error */ }` so
a future wasmtime upgrade cannot silently misbucket a new Trap variant as INDETERMINATE.

---

## Rationale

### Prior Art and Theoretical Grounding

**Saltzer and Schroeder (1975) — Fail-Safe Defaults:** Security systems should default to lack
of access, not access. Applied here: when a validator cannot determine whether a mutation is
safe, the safe default is "not confirmed safe" — manifested as the INDETERMINATE outcome and
the blocking marker. Note: fail-safe defaults are not universally fail-closed; for THIS integrity
gate the safe direction is deny/quarantine-forward because the protected invariant is
write-integrity, not availability.

**NIST SP 800-53 SA-8(23) — Fail-Safe Default (Fail-Secure):** "Implement security design
principles including fail-safe defaults." This principle requires that the system enters a secure
state on failure, not an open state.

**NIST SP 800-53 SA-8(24) — Fail Secure:** When security mechanisms fail, the system falls
back to a state where access is denied until the failure is resolved. Layer 1's marker+gate
mechanism is the operational realization: when a validator cannot complete, future state-advancing
actions are blocked until resolution.

**OWASP — Fail Securely:** When a security decision (allow/disallow) fails with an exception,
the exception MUST follow the "disallow" path, not the "allow" path. Applied to validators:
the exception case (cannot-complete) follows the disallow path for fail-closed plugins.

**SMT-LIB UNKNOWN (formal methods analogy):** An SMT solver returns `sat` (PASS), `unsat` (FAIL),
or `unknown` (resource limit exceeded, undecidable). `unknown` is not `sat`. INDETERMINATE maps
exactly to `unknown` in SMT-LIB: the tool exhausted its resources without reaching a conclusion.
Treating `unknown` as `sat` is a soundness violation.

**CWE-754 — Improper Check for Exceptional Conditions:** Failure to properly detect or handle
exceptional conditions. The pre-Layer-1 behavior (treating cannot-complete as PASS) is an
instance of CWE-754.

**CWE-400 — Uncontrolled Resource Consumption:** Continuously raising the fuel cap to
accommodate larger and larger cycle artifacts without bounding the artifact size is an
unbounded resource consumption pattern. Layer 2 (bounded shard cap derived from fuel cap)
resolves the root cause. Layer 1 makes the failure mode loud.

**Google SRE — Toil:** Toil is work that is manual, repetitive, automatable, and does not yield
permanent improvement. The pre-Layer-1 pattern of "agent manually avoids large artifacts" is
toil. Layer 1+2+3 convert toil into permanent mechanical guarantees.

### Why INDETERMINATE as a Named Outcome?

The alternative would be to treat `PluginResult::Timeout { cause: Fuel }` + `failure_policy = fail-closed`
directly as FAIL. ADR-039 explicitly rejected this framing: INDETERMINATE means "could not determine"
— the plugin neither passed nor failed; it failed to complete. Treating it as FAIL would cause the
next dispatch to be blocked even after the root cause (large artifact) has been remediated. Naming
INDETERMINATE separately allows the marker/gate mechanism to distinguish "we don't know" from
"we checked and it's bad."

### Why a Durable Marker File Rather Than an In-Memory Flag?

An in-memory flag is lost on dispatcher restart or context compaction — the canonical failure
modes that follow large-artifact writes. A marker file survives session restarts and context
clears, ensuring the "we couldn't validate" signal outlasts the session that produced it.

### Why a Single Marker File?

Per-plugin marker files were considered at F1. Rejected for Layer 1: the extra complexity of
selective re-validation and multi-marker coordination is not warranted at this stage. Layer 3's
bounded windows will reduce concurrent INDETERMINATE events to near-zero, making the
multi-marker scenario increasingly rare. The single-file policy is production-grade for Layer 1.

### Why Gate Agent Dispatch AND git commit/push Rather Than Edit/Write?

Edit/Write are PostToolUse-only in terms of blocking; the writes have already happened by the time
the dispatcher evaluates failure policy. The two state-advancing ACTIONS that must be quarantined
are: (a) the next Agent dispatch (which loads the latest specs, plans the next step, and
potentially mutates more state), and (b) `git commit`/`git push` which propagate `.factory/`
mutations to the `factory-artifacts` durable branch. Blocking both is the complete set of
durable-propagation paths that follow a PostToolUse INDETERMINATE event. Read, Edit, Write,
and non-git-commit/push Bash dispatches are not propagation paths for the unvalidated mutation
and need not be gated.

### ADR-047 ↔ ADR-039 Relationship

ADR-039 defines the `failure_policy` field and its semantics (what fail-closed means for a
resource-exhaustion event). ADR-047 adds: what happens AFTER the event is classified as
INDETERMINATE — specifically, the durable marker and the next-advance gate. The two ADRs are
complementary layers of the same mechanism. ADR-039 owns the PER-DISPATCH block semantics;
ADR-047 owns the CROSS-DISPATCH persistence + gate semantics.

---

## Integration Ordering Recommendation

**S-25.01 can be delivered independently of S-21.19–S-21.24** (the HELD Wave-7 enforcement seams).
The marker mechanism and next-advance gate work correctly regardless of whether the current-dispatch
block (ADR-039 §Decision 3 Phase 4 enforcement) is wired.

Full operational effect — INDETERMINATE both blocks the CURRENT dispatch (via fail-closed
`plugin_fail_closed` enforcement) AND writes the durable marker (via S-25.01) — requires BOTH
S-25.01 AND S-21.24 to be merged.

Partial operational effect available now (S-25.01 only): INDETERMINATE from fail-closed plugins
writes the durable marker and blocks the NEXT Agent dispatch. The current PostToolUse dispatch is
not blocked (the write already happened before PostToolUse fires anyway).

**S-25.01 Cohort A registry assignments:** S-25.01 sets `failure_policy = "fail-closed"` for all
three Cohort A validators in `hooks-registry.toml`. However the assignments are not equivalent:

- **`validate-factory-path-staging` (Cohort A-immediate):** set unconditionally; no S-21.24 dependency.
  `on_error = "continue"` ensures no self-lock risk when S-21.24 later wires enforcement.
- **`validate-pr-merge-prerequisites` and `validate-wave-gate-prerequisite` (Cohort A-deferred):**
  S-25.01 PREPARES the field in the registry with a comment:
  `# ACTIVATION CONDITIONAL on S-21.24 calibration per ADR-047 §D8a Cohort A-deferred`
  The field is INERT until S-21.24 wires the ADR-039 Phase 4 executor enforcement (S-21.10 is
  schema-only). S-21.24's dependency chain (S-21.19 → S-21.21 calibration → S-21.23 break-glass)
  resolves the self-lock risk before enforcement activates. Do NOT treat these two validators as
  actively fail-closed until S-21.24 merges.

**Layer-1 effective fail-closed count at S-25.01 merge: ZERO** (corrected v1.4 — see §8a "Why the
partition?" for the full rationale: `validate-factory-path-staging`'s PreToolUse registration
means the PostToolUse-only marker-write path can never fire for it, and `on_error = "continue"`
means it can never block the current dispatch either; the two `^Agent$` validators remain inert
pending S-21.24). The "3 Cohort A validators" is the correct Cohort A membership; the immediate
enforcement is 0, not 1. Closure of the `validate-factory-path-staging` gap is tracked to a
dedicated follow-up story — see §8a "Known Gap" above (recommended ID S-25.04).

---

## Consequences

### Positive

1. "Cannot validate" is no longer silent — it is fail-LOUD with a human-readable artifact.
2. The next state-advancing dispatch is automatically blocked until the situation is resolved —
   covering both Agent pipeline dispatch (Arm 1) and durable-branch propagation via git (Arm 2).
3. OutputTooLarge events that produce a silent false-PASS are detected and classified correctly.
4. The mechanism is self-contained — no changes to the hook-sdk ABI or HOST_ABI_VERSION.
5. Existing fail-open behavior is completely unchanged; zero regression risk for the ~76 current
   production plugins until explicit `failure_policy = "fail-closed"` assignments are made.
6. The durable-propagation boundary is fully closed in Layer 1: an INDETERMINATE-tainted artifact
   cannot be committed or pushed to `factory-artifacts` while the marker is present.

### Negative

1. The OutputTooLarge Store-flag mechanism adds a boolean field to StoreData and two lines of
   wrapping code per host function — minimal but non-zero complexity.
2. A single marker file cannot distinguish WHICH of multiple concurrent INDETERMINATE events
   produced the unvalidated mutation (last-writer-wins). This is acceptable for Layer 1.
3. The next-advance gate creates a recovery path that requires either re-running the validator
   (potentially impossible while the artifact is large) or manual marker deletion. This is
   mitigated by: Layer 2 (sharding eliminates the large-artifact root cause) and the manual
   escape hatch (`rm .factory/unvalidated-mutation.marker`).
4. The marker file persists across session boundaries. A stale marker from a previous session
   will block the first Agent dispatch of a new session. Human must recognize the marker's
   block message and respond appropriately.

### Risk: Marker Accumulation in CI

If CI environments exit without re-validation, marker files may persist across CI runs. Mitigation:
CI pipelines should add `rm -f .factory/unvalidated-mutation.marker` to their setup step, or the
S-25.01 story should include a CI fixture that verifies no marker is present at test-start.

---

## Alternatives Considered

### A — On-Demand Re-Validation (No Persistent Marker)

Design: when INDETERMINATE fires, the dispatcher immediately attempts to re-run the plugin with
the same inputs. If re-run passes, treat as PASS. Only if re-run also fails treat as INDETERMINATE.
Rejected: the re-run may exhaust the same fuel budget (the artifact is still large); this masks
the root cause. The point is to make CANNOT-VALIDATE visible, not to hide it behind a retry.

### B — Amend ADR-039 Rather Than Author ADR-047

Design: extend ADR-039 with §AMD-005 covering the INDETERMINATE outcome and marker mechanism.
Rejected (per F1 Open Question 5, architect recommendation accepted by human): ADR-039 is focused
on resource-exhaustion calibration and fail-closed enforcement within a single dispatch. ADR-047's
concern is cross-dispatch persistence. Keeping them separate makes each ADR readable without
the other. ADR-047 explicitly cites ADR-039 as its foundation.

### C — Per-Plugin Marker Files

Design: `.factory/unvalidated-mutation-<plugin_name>.marker` to allow selective re-validation.
Rejected for Layer 1: complexity not warranted. See Decision 3 rationale. Revisit in Layer 3 if
bounded windows do not eliminate multi-plugin INDETERMINATE events.

---

## Source / Origin

- **F1 Delta Analysis:** `.factory/feature-delta/validation-integrity-layer1/F1-delta-analysis.md` — authoritative scope document; problem statistics, three-layer architecture, decision table, VP proposals, fail-closed validator candidates, impact boundary.
- **ADR-039:** `decisions/ADR-039-validator-failure-policy-resource-exhaustion-fail-closed.md` — the framework this ADR extends; normative for `failure_policy` field schema, calibration prerequisites (§Decision 3), and axes-independence invariant (§Decision 1). v1.16 ratified.
- **ADR-042:** `decisions/ADR-042-validate-cross-site-correspondence-fuel-budget-raise-and-loud-exhaustion-signaling.md` — motivation for OutputTooLarge loud detection; `DEFAULT_FUEL_CAP` raise from 10M to 20M.
- **CAP-041 (new):** Validation Integrity: INDETERMINATE Outcome, Durable Mutation Marker, and Next-Advance Gate — new capability under SS-01 authored in F2 burst.
- **Architecture as-built (crates affected):** `crates/factory-dispatcher/src/executor.rs` (primary — INDETERMINATE classification logic, OutputTooLarge Store-flag check); new `crates/factory-dispatcher/src/indeterminate_marker.rs` (marker write/clear logic); `crates/hook-sdk/src/host.rs` (host function wrappers — add `host_output_too_large_seen = true` set); new `crates/hook-plugins/validate-unvalidated-mutation-marker/src/lib.rs` (next-advance gate plugin); `plugins/vsdd-factory/hooks-registry.toml` (new gate entry; fail-closed assignments); `plugins/vsdd-factory/tests/validate-unvalidated-mutation-marker.bats` (integration test).
- **BCs (proposed, pending product-owner F3):** BC-1.18.001–BC-1.18.004 (SS-01) + BC-3.08.001 amendment (`plugin.indeterminate` event type, SS-03).
- **VPs (authored this F2 burst):** VP-102–VP-106 in `.factory/specs/verification-properties/`.
