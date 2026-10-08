---
document_type: behavioral-contract
level: L3
version: "1.38"
last_amended: "2026-10-08 (v1.38) — ADR-052 v1.25 item 11(g) (closes the Event 12 / operator-line cap conflation; independent of any wire-format change): the 64-character sanitization cap on Events 11–13 data-derived substrings is stated as InternalLog-event-only (Event 12 `detail` field semantics and Invariant 7 gain \"(InternalLog event only; the operator stderr rendering of the same fact is capped at 256 per BC-1.18.013 Postcondition 10)\"); EC-018 and the Event 12 Canonical Test Vectors gain a ~100-character-path vector (truncated to 64 in the event, untruncated on stderr). NO new event, NO new field, NO event-count change (still thirteen), `cause`/`kind` domains, mandatory fields and sink target unchanged. [Prior: 2026-10-07 (v1.37) — ADR-052 v1.23 \"third binary-leg extension\" (item 10(c); independent validation `.factory/research/adr-052-v123-architect-calls-validation.md`): Event 12 `migration.admission_failed` field `kind` closed domain gains the sixth token `txn_record_newer_schema` (a txn record whose `schema_version` is an integer >= 2, written by a newer build; reachable on the admission surface through the Branch B / Branch C version gate, BC-1.18.013 Precondition 6(c) rule 9); additive wire-domain token only — NO new event, NO new field, NO event-count change, `cause` domain unchanged. [Prior: 2026-10-07 (v1.36) — story-anchor re-attribution after the human-approved S-25.08/S-25.09 split (D-1252(f)): Events 11–13 are delivered by S-25.09 (Story Anchor + Traceability Stories rows; the S-25.08-seam prose sites re-pointed); documentary only, no wire-format/field-shape change. [Prior: 2026-10-07 (v1.35) — ADR-052 §Decision 5a v1.21 \"Admission diagnostics channel\" (S-25.08 local adversary pass 2 D-1; ADR-052 v1.21 §Downstream item 25; architect-ratified): POLICY 9 — three NEW dispatcher-native events catalogued as Events 11–13: `migration.admission_blocked`, `migration.admission_failed`, `migration.admission_advisory` (the admission/release/reconciliation diagnostics of BC-1.18.011 v1.18 / BC-1.18.013 v1.10 Postcondition 10, previously mandated as `tracing::*!` calls that the subscriber-less dispatcher discards in production). Wire-format/field-shape authority only; triggering semantics are BC-1.18.013 Postcondition 10 / BC-1.18.011 Postcondition 10. Emission path is `InternalLog::write` DIRECTLY by `main.rs` (NOT `HostContext::emit_internal` — no HostContext exists at the registry-independent admission position; recorded deviation) so Events 11–13 land ONLY in `dispatcher-internal-{date}.jsonl` (no `ctx.events` queue, no `plugin_name`). H1, §Description, §Common Fields, §Invariants 1/3 (+ new Invariant 7), §Related BCs, §Architecture Anchors, §VP Anchors (VP-079 v1.24 / VP-028 v1.1 / VP-133 v1.6 / VP-143 v1.5 cited; VP-079 staleness flag closed), §Edge Cases (new EC-017..EC-019), §Canonical Test Vectors, §Traceability and Amendment section updated. Events 1–10 and their wire formats UNCHANGED. [Prior: 2026-09-01 (v1.34) — ADR-048 §Decision 4 v1.5 Emission-Point Correction (S-25.01
  adversary pass 9 F-P9-001 MEDIUM; architect adjudication; HUMAN-RATIFIED 2026-09-01, POLICY 22,
  D-1142): POLICY 9 — Event 9 SUPERSEDED emission-point correction: `emit_superseded_if_cross_pair`
  relocated from the pre-overwrite READ to inside `write_indeterminate_marker`'s `Ok(())` arm,
  symmetric with Event 10's v1.33 `marker.written` emit-only-after-success rule; on `Err(_)`, NEITHER
  event is emitted. Event 9 Trigger bullet (PC5), `clear_mode`/`actor_type` correspondence table row,
  Emission path paragraph, Event 10 Trigger paragraph, Architecture Anchors (Event 9 + Event 10
  bullets), EC-014 (SUPERSEDED ordering) + EC-016 (write-failure symmetric note), Canonical Test
  Vectors row (`marker-cleared-superseded`), Verification Properties (new VP-108 PC8 row), and
  Traceability ADR row all updated. Event count unchanged at ten; SUPERSEDED/system/non-null-reason
  wire contract otherwise unchanged — this is an emission-point correction only. Amendment section
  appended. [Prior: 2026-09-01 (v1.33) — ADR-048 §Decision 4 v1.4 Reconciliation-Premise Correction (S-25.01
  adversary pass 6 F-P6-001 MEDIUM; architect adjudication; HUMAN-RATIFIED 2026-09-01, POLICY 22,
  D-1141); POLICY 9: new Event 10 `marker.written` catalog entry — the positive
  marker-creation record emitted by `write_indeterminate_marker`'s caller ONLY after the atomic
  write returns `Ok(())`, via `HostContext::emit_internal`, making `reconcile_raw_delete`'s
  OPERATOR_OVERRIDE reconciliation (Event 9) sound by construction against unmatched `marker.written`
  rather than unmatched `plugin.indeterminate`. Full triggering-condition/semantics authority is
  BC-1.18.001 §PC4 v1.4; this catalog entry registers the wire-format/field-shape authority only,
  the same split already established for Events 7/8/9. Count-phrase sweep: nine→ten event types
  throughout (H1, §Description, §Common Fields, §Invariants 1+3, §VP Anchors VP-079+VP-028,
  §Verification Properties VP-079 row, §Traceability DI-017). Event 9's OPERATOR_OVERRIDE Trigger
  bullet, `clear_mode`/`actor_type` correspondence table row, `trace_id` semantics paragraph, EC-013,
  and the `marker-cleared-operator-override` Canonical Test Vectors row all retargeted from unmatched
  `plugin.indeterminate` to unmatched `marker.written`. §VP Anchors VP-079 staleness flag added for
  Event 10. §Related BCs, §Architecture Anchors, §Edge Cases (new EC-015/EC-016), §Canonical Test
  Vectors (new rows), §Verification Properties, and §Traceability ADR row all extended. Amendment
  section appended. [Prior: 2026-09-01 (v1.32) — ADR-048 §Decision 4 v1.3 Emission-Mechanism Precision Correction (S-25.01 LOCAL adversary pass 3 F-P3-001 MEDIUM) + SUPERSEDED Clear Mode (F-P3-002 LOW); POLICY 9; human-ratified 2026-09-01 (POLICY 22, D-1140, per ADR-048 v1.3 Status): Event 9 `clear_mode` enum gains `SUPERSEDED`, `actor_type` enum gains `system`, `reason`-mandatory condition extended to include SUPERSEDED (wire format + mandatory-fields + clear_mode/actor_type correspondence table + trace_id/reason semantics all updated); §Sink destination prose corrected — it previously claimed all nine events are 'NOT routed to the dispatcher-internal debug stream,' which is INACCURATE for the actual dispatcher-native mechanism: all nine route via `HostContext::emit_internal`, which durably writes `dispatcher-internal-{date}.jsonl` unconditionally (not gated behind the opt-in DI-007 flag) AND pushes onto `ctx.events` for eventual FileSink drain; Events 8 and 9's §Emission path paragraphs corrected to name `HostContext::emit_internal` explicitly (not a raw `InternalLog::write`) plus the `dispatcher-internal-{date}.jsonl` durability property, matching the seven sibling BC-3.08.001 events; new §Durable sink target note explains the deliberate scan-target divergence from the literal `events-{date}.jsonl` filename pending S-4.07. §Trigger bullets, §Architecture Anchors, EC-013 (scan-target wording) + new EC-014 (SUPERSEDED), new Canonical Test Vectors row (`marker-cleared-superseded`), §VP Anchors + §Verification Properties VP-079 staleness-flag extension, and §Traceability ADR row all updated. Event count unchanged at nine — `SUPERSEDED` is a new VALUE of the existing `clear_mode` enum, not a tenth event. [Prior: 2026-08-31 (v1.31) — ADR-048 §Decision 4 v1.2 Emission-Point Correction (S-25.01 LOCAL adversary pass 2 F-P2-002 HIGH + F-P2-003 MED; POLICY 9; human-ratified): Event 9 `marker.cleared` clear_mode/actor_type correspondence table's TTL_EXPIRED and OPERATOR_OVERRIDE 'Emission point' cells re-attributed from the WASM gate plugin to dispatcher-native `check_and_clear_expired_marker` / `reconcile_raw_delete` (both in `indeterminate_marker.rs`); `evaluate_gate` becomes a pure marker-presence check. §Emission path paragraph, §Trigger bullets (PC4/PC3), §Architecture Anchors marker.cleared bullet, EC-012/EC-013, and the two Canonical Test Vectors rows (marker-cleared-ttl-expired / marker-cleared-operator-override) corrected to the same dispatcher-native locus. No wire-format/field-shape change; VP-108 v1.1 and VP-106 v1.5 are the architect-side companion corrections. [Prior: 2026-08-31 (v1.30) — ADR-048 §Decision 4: Event 9 `marker.cleared` added to the SS-03 event catalog (BC-1.18.003 triggering-condition/clear-path authority; this BC provides wire-format/field-shape catalog authority). Count-phrase sweep: eight→nine event types throughout (H1, §Description, §Common Fields, §Sink destination, §Invariants 1+3, §VP Anchors VP-079+VP-028, §Verification Properties VP-079 row). §VP Anchors VP-079 staleness flag added for Event 9 — architect must propagate to VP-079 SITE_9, mandatory-fields table, and Property Statement. §Related BCs, §Architecture Anchors, §Traceability ADR row extended. Amendment section + Changelog row added. [Prior: 2026-08-30 (v1.29) — consistency-audit closure (product-owner): §VP Anchors VP-079 staleness flag for Event 8 closed — VP-079 v1.22 covers Event 8 (`plugin.indeterminate`) in its Property Statement (eight events), mandatory-fields table, and SITE_8. [Prior: 2026-08-30 (v1.28) — F2 validation-integrity-layer1 spec burst (product-owner): Event 8 `plugin.indeterminate` added to the SS-03 event catalog (BC-1.18.001 triggering-condition/semantics authority; this BC provides the wire-format/field-shape catalog authority — same pattern as Event 7/BC-1.03.019). Count-phrase sweep: seven→eight event types throughout. §VP Anchors and §Verification Properties extended with VP-102/VP-103/VP-104/VP-105/VP-106 (BC-1.18.001–004 verifiers). §Traceability ADR row extended with ADR-047; Stories row extended with S-25.01. Amendment section appended. [Prior: 2026-08-21 (v1.27) — D-1064 Wave-6 pass-6 remediation: F-S2125-P6-001 (HIGH, POLICY 19 adr_version_cite_volatile_pin_prohibition) sibling-sweep — Traceability ADR row's load-bearing `ADR-039 v1.15 §Decision 5 Mitigation 1` pin replaced with stable form `ADR-039 §Decision 5 Mitigation 1 (E-006)`; F-S2125-P6-002 (LOW) — live §VP Anchors VP-079 closure bullet annotated `(VP-079 v1.20 at closure; now v1.21)` to keep the bare version cite from reading as current. [Prior: 2026-08-20 (v1.26) — F-S2125-P2-003 (MEDIUM, S-21.25 adversarial pass-2, v1.0-brownfield-backfill): false VP-079-staleness flag closed at 3 sites (§VP Anchors bullet, Amendment changes-made item 11, standalone Amendment paragraph) — VP-079 v1.20 already registers Event 7, the architect follow-up was already done, the flag was simply never cleared. Sibling-site sweep: `emit_fuel_headroom_warning` → `emit_plugin_fuel_headroom_warning` (2 occurrences in the v1.25 Amendment section) to match BC-1.03.019 v1.2's F-S2125-P2-002 emitter rename. [Prior: 2026-08-20 (v1.25) — F-S2125-P1-003 (MEDIUM, S-21.25 adversarial pass-1, v1.0-brownfield-backfill): Event 7 `plugin.fuel_headroom_warning` added to the SS-03 event catalog — new dispatcher wire event for ADR-039 §Decision 5 Mitigation 1 (fuel-headroom early-warning signal); full triggering-condition/semantics authority remains BC-1.03.019, this BC registers the wire-format/field-shape catalog entry. Count-phrase sweep: six→seven event types throughout. [Prior: 2026-07-15 (v1.24) — F-P8-001 (LOW, S-19.09 D22): Event 6 wire format and mandatory-fields enumeration amended to include `timestamp` field (ISO-8601 alias of `ts`, matching Events 1–5 sibling parity). [Prior: 2026-07-13 (v1.23) — S-19.05 pass-13 fix-burst discovery: frontmatter status: draft / lifecycle_status: active mismatch — missed POL-14 auto-promotion at S-15.01 PR-106 merge (2026-05-08, merge_sha=453eee1). [Prior: 2026-07-13 (v1.22) — S-19.05 pass-13 F-P13-001: five stale count phrases corrected (§Common Fields intro; session_id row; §Architecture Anchors FileSink row; §Traceability CAJ; §Traceability DI-017); §Traceability ADR row disambiguated (original-ADR four scope explicitly stated; Events 5–6 provenance noted). Whole-file count-phrase sweep conducted. [Prior: 2026-07-10 (v1.21) — F-P43-003: §VP VP-100 row verbatim-derived from VP-INDEX (cardinality+mutual-exclusivity form); F-P43-005: v1.19 Changelog row backfilled + v1.20 Amendment section authored; O-P43-001: last_amended canonicalized to chain form. [Prior: 2026-07-09 (v1.20) — D-798 pre-pass-43 consistency sweep.]]]]]]]]]]]]]]]"
status: active
producer: product-owner
timestamp: 2026-05-07T00:00:00Z
phase: F2
inputs:
  - .factory/cycles/v1.0-feature-plugin-async-semantics-pass-1/adversary-pass-1.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.14.001.md
  - .factory/specs/behavioral-contracts/ss-07/BC-7.06.001.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.03.019.md
input-hash: "8d4ced9"
traces_to: .factory/specs/prd.md
origin: greenfield
extracted_from: null
subsystem: "SS-03"
capability: "CAP-003"
lifecycle_status: active
introduced: v1.0-feature-plugin-async-semantics-pass-1
modified:
  - "2026-07-06 (v1.15)"
  - "2026-07-06 (v1.16)"
  - "2026-07-06 (v1.17)"
  - "2026-07-07 (v1.18)"
  - "2026-07-07 (v1.19)"
  - "2026-07-09 (v1.20)"
  - "2026-07-10 (v1.21)"
  - "2026-07-13 (v1.22)"
  - "2026-07-13 (v1.23)"
  - "2026-07-15 (v1.24)"
  - "2026-08-20 (v1.25)"
  - "2026-08-20 (v1.26)"
  - "2026-08-21 (v1.27)"
  - "2026-08-30 (v1.28)"
  - "2026-08-30 (v1.29)"
  - "2026-08-31 (v1.30)"
  - "2026-08-31 (v1.31)"
  - "2026-09-01 (v1.32)"
  - "2026-09-01 (v1.33)"
  - "2026-09-01 (v1.34)"
  - "2026-10-07 (v1.35)"
  - "2026-10-07 (v1.36)"
  - "2026-10-07 (v1.37)"
  - "2026-10-08 (v1.38)"
deprecated: null
deprecated_by: null
replacement: null
retired: null
removed: null
removal_reason: null
---

# BC-3.08.001: dispatcher async-semantics event types are catalogued and emitted via FileSink — `plugin.async_block_discarded`, `dispatcher.schema_mismatch`, `dispatcher.registry_invalid`, `plugin.timeout` (async path), `plugin.abandoned`, `plugin.completed` (async path), `plugin.fuel_headroom_warning`, `plugin.indeterminate`, `marker.cleared`, `marker.written`, `migration.admission_blocked`, `migration.admission_failed`, `migration.admission_advisory`

## Description

ADR-019 F2 introduces four new event-type strings as part of the async-semantics feature; `plugin.abandoned` is added by the F5 E-19 pass-1 fix burst (F-P1-013) to cover the async drain-window expiry path; `plugin.completed` (async path) is added by the F5 E-19 pass-5 fix burst (F-P5-003) to cover async plugins that complete within the drain window; `plugin.fuel_headroom_warning` is added by the S-21.25 adversarial pass-1 fix burst (F-S2125-P1-003) to register the ADR-039 §Decision 5 Mitigation 1 fuel-headroom early-warning wire event whose full triggering-condition/semantics authority is BC-1.03.019; `plugin.indeterminate` is added by the F2 validation-integrity-layer1 spec burst (product-owner, 2026-08-30) to register the ADR-047 INDETERMINATE outcome wire event whose full triggering-condition/semantics authority is BC-1.18.001; `marker.cleared` is added by the ADR-048 §Decision 4 spec burst (product-owner, 2026-08-31) to register the audited marker-clearance wire event whose full clear-path/lifecycle authority is BC-1.18.003 (REVALIDATED/TTL_EXPIRED/OPERATOR_OVERRIDE/SUPERSEDED clear modes); `marker.written` is added by the ADR-048 §Decision 4 v1.4 Reconciliation-Premise Correction spec burst (product-owner, 2026-09-01, S-25.01 adversary pass 6 F-P6-001 MEDIUM) to register the audited marker-CREATION wire event whose full triggering-condition/semantics authority is BC-1.18.001 §PC4 v1.4 — it is the positive creation record that makes `marker.cleared`'s OPERATOR_OVERRIDE reconciliation (Event 9, BC-1.18.003 §PC3) sound by construction. These events are referenced in BC-1.14.001, BC-7.06.001, BC-1.03.019, BC-1.18.001, and BC-1.18.003 but require SS-03 catalog authority to define their payload schemas and wire format. Each event is a JSON line written to `events-*.jsonl` via the standard FileSink path (Events 8/9/10 additionally durably land in `dispatcher-internal-{date}.jsonl` via `HostContext::emit_internal` — see Event 8's, Event 9's, and Event 10's §Emission path paragraphs below). This BC provides the catalog entry for all ten, establishing the authoritative field set, wire format, and sink-fan-out obligation per DI-007 (amended: opt-in debug stream) and the VP-028 sink-fan-out invariant. **Events 11–13 (v1.35)** — `migration.admission_blocked`, `migration.admission_failed`, `migration.admission_advisory` — are specified by the S-25.08 local adversary pass-2 fix burst (D-1; delivered by S-25.09 after the S-25.08/S-25.09 split, D-1252(f); ADR-052 §Decision 5a "Admission diagnostics channel (v1.21)") to register the structured diagnostics of the governed-migration writer-admission gate whose triggering-condition/semantics authority is BC-1.18.013 Postcondition 10 (and BC-1.18.011 Postcondition 10). They differ from Events 1–10 in two load-bearing ways: they are written by `main.rs` directly through `InternalLog::write` (no `HostContext` exists at the registry-independent admission position), so they land ONLY in `dispatcher-internal-{date}.jsonl` and are NOT pushed onto `ctx.events` / FileSink fan-out; and they carry no `plugin_name` (dispatcher-native, no plugin context). The thirteen-event catalog is therefore: ten host-`emit_internal` events plus these three direct-`InternalLog` events.

## Preconditions

1. Dispatcher is running with `schema_version = 2` registry (BC-7.06.001).
2. FileSink is initialized and the `events-YYYY-MM-DD.jsonl` file is writable.
3. The triggering condition for each event type has occurred (see Postconditions for per-event triggers).

## Common Fields

All ten event types carry the following dispatcher-owned fields on the wire. These fields are injected by the host (see `emit_event.rs` enrichment path) and are never supplied by plugins (they are RESERVED_FIELDS — see §Implementation Notes):

| Field | Type | Description |
|-------|------|-------------|
| `trace_id` | UUID v4 string | Trace correlation value from the invoking hook envelope (DI-017). Canonical wire-format name; `dispatcher_trace_id` must NOT appear on wire (Invariant 5). |
| `session_id` | UUID v4 string | Claude Code session identifier from the hook envelope context (`ctx.session_id`). Present on all ten event types (O-P15-001). |
| `plugin_name` | string | Name of the plugin registry entry, injected by the host. Present on plugin-context events (1, 4, 5, 6, 7, and 8) only; absent from dispatcher-startup events (2 + 3) which have no plugin context. Events 9 and 10 also carry `plugin_name`, but sourced from the marker's own fields (constructed dispatcher-native, not host-injected from the invoking plugin's own `ctx.plugin_name`) — see Event 9's and Event 10's own wire formats. |
| `ts` | string | Emission timestamp (internal format). |
| `ts_epoch` | integer | Emission timestamp as Unix epoch milliseconds. |
| `schema_version` | integer | Registry schema version at emission time. |
| `type` | string | The event type string (e.g. `"plugin.async_block_discarded"`). |

The §Common Fields appear on the wire for ALL ten event types except where noted. Wire-format examples in §Postconditions show:
- **Plugin-context events (1, 4, 5, 7, and 8):** `plugin_name` explicitly shown (these are plugin-instance events). `plugin_version` is NOT emitted by Events 1, 4, 5, 7, and 8 — the original BC-3.08.001 emit functions for these events (in `crates/factory-dispatcher/src/host/emit_event.rs`) do not call `with_plugin_version()`.
- **Event 6 (`plugin.completed` async path):** mirrors the sync-path `emit_lifecycle` call chain in `crates/factory-dispatcher/src/executor.rs`, which includes `with_plugin_version()`. Both `plugin_name` and `plugin_version` are explicitly shown in the Event 6 wire example.
- **Dispatcher-startup events (2 + 3):** `plugin_name` OMITTED from examples (no plugin context at dispatcher startup).
- **Events 9 and 10 (`marker.cleared` / `marker.written`):** `plugin_name` and `artifact_path` explicitly shown, sourced from the marker's own TOML fields (or, for Event 9 OPERATOR_OVERRIDE, from the matched `marker.written` scan record) rather than the host's per-invocation `ctx.plugin_name` injection — see each event's own §Postconditions entry.
- All ten event types: `trace_id` + `session_id` explicitly shown (verified by VP-079 payload conformance).
- **Events 11–13 (`migration.admission_blocked` / `_failed` / `_advisory`; v1.35):** carry the `InternalEvent` envelope fields `type`, `ts`, `ts_epoch`, `schema_version` plus `trace_id` and `session_id` (set by `main.rs` via `.with_trace_id(..).with_session_id(..)` from the dispatch's hook envelope context, DI-017); they carry NO `plugin_name` and NO `plugin_version` (dispatcher-native admission events with no plugin context) and, like Event 10, no distinct top-level `timestamp` (the common `ts` is the only time field). Their event-specific fields are defined in each event's own §Postconditions entry.
- Common fields shown only in summary: `ts`, `ts_epoch`, `schema_version` (always emitted; not in examples for readability).

## Postconditions

### Event 1: `plugin.async_block_discarded`

**Trigger**: An async group plugin (one with `async = true` in the registry) returns exit code 2 (which would be a block verdict if it were in the sync group). Because async plugins cannot have `on_error = "block"` (BC-1.14.001 Invariant 4), the block intent is structurally invalid and is discarded rather than reaching Claude Code.

**Wire format** (JSON line in `events-*.jsonl`):

```json
{
  "type": "plugin.async_block_discarded",
  "trace_id": "<uuid-v4>",
  "session_id": "<uuid-v4>",
  "plugin_name": "<string — registry entry name>",
  "exit_code": 2,
  "timestamp": "<ISO-8601>",
  "reason": "async_plugin_block_verdict_discarded"
}
```

**Mandatory fields**: `type`, `trace_id`, `session_id`, `plugin_name`, `exit_code`, `timestamp`, `reason`.

### Event 2: `dispatcher.schema_mismatch`

**Trigger**: Dispatcher loads `hooks-registry.toml` and finds `schema_version != 2` (e.g., `schema_version = 1` or any unknown value). Dispatcher hard-errors and emits this event before exiting.

**Wire format** (JSON line in `events-*.jsonl`):

```json
{
  "type": "dispatcher.schema_mismatch",
  "trace_id": "<uuid-v4>",
  "session_id": "<uuid-v4>",
  "found_version": <integer or null>,
  "expected_version": 2,
  "timestamp": "<ISO-8601>",
  "error_code": "E-REG-001"
}
```

**Mandatory fields**: `type`, `trace_id`, `session_id`, `found_version`, `expected_version`, `timestamp`, `error_code`.

### Event 3: `dispatcher.registry_invalid`

**Trigger**: Dispatcher's `registry.rs::validate()` detects a registry invariant violation. Two distinct violation conditions trigger this event, each with its own `error_code`:

| `error_code` | `violation` | Triggering condition |
|---|---|---|
| `E-REG-002` | `async_block_conflict` | A registry entry has both `on_error = "block"` AND `async = true` simultaneously |
| `E-REG-003` | `duplicate_hook_registration` | Two or more registry entries share the same hook name (duplicate registration) |

Dispatcher emits this event and hard-errors (non-zero exit) for either condition.

**Wire format — E-REG-002 variant** (JSON line in `events-*.jsonl`):

```json
{
  "type": "dispatcher.registry_invalid",
  "trace_id": "<uuid-v4>",
  "session_id": "<uuid-v4>",
  "offending_plugin": "<string — name of the plugin entry that violates the invariant>",
  "violation": "async_block_conflict",
  "timestamp": "<ISO-8601>",
  "error_code": "E-REG-002"
}
```

**Wire format — E-REG-003 variant** (JSON line in `events-*.jsonl`):

```json
{
  "type": "dispatcher.registry_invalid",
  "trace_id": "<uuid-v4>",
  "session_id": "<uuid-v4>",
  "offending_plugin": "<name>",
  "offending_event": "<event>",
  "offending_tool": "<tool regex string or null>",
  "violation": "duplicate_hook_registration",
  "error_code": "E-REG-003",
  "timestamp": "<ISO-8601>"
}
```

**Mandatory fields for E-REG-003**: `type`, `trace_id`, `session_id`, `offending_plugin` (string, required), `offending_event` (string, required), `offending_tool` (string or null, required — null when the duplicating entry has no `tool` filter), `violation`, `error_code`, `timestamp`.

**E-REG-002 vs E-REG-003 field asymmetry (F-P14-001):** E-REG-002 (`AsyncBlockConflict`) does NOT include `offending_event` or `offending_tool` in its payload — the violation is intra-entry (a single entry simultaneously has `on_error = "block"` and `async = true`; no second entry is involved). E-REG-003 (`DuplicateEntry`) DOES include `offending_event` and `offending_tool` because the violation is inter-entry: the specific `(name, event, tool)` tuple uniquely identifies which entry is the duplicator. The data is already present in `RegistryError::DuplicateEntry { name, event, tool }` and MUST be propagated to the event payload. Implementations MUST propagate all three tuple fields; omitting `offending_event` or `offending_tool` is a BC violation.

The `error_code` field is an enum with exactly two valid values: `"E-REG-002"` and `"E-REG-003"`. The `violation` field value is determined by the `error_code` per the table above — no other combinations are valid.

### Event 4: `plugin.timeout` (async path)

**Note**: `plugin.timeout` is emitted for both sync and async plugin timeouts. The sync-path behavior is governed by BC-1.14.001. This entry covers the **async-path variant** only: when an async group plugin exceeds its `timeout_ms` and is terminated.

**Wire format** (JSON line in `events-*.jsonl`):

```json
{
  "type": "plugin.timeout",
  "trace_id": "<uuid-v4>",
  "session_id": "<uuid-v4>",
  "plugin_name": "<string>",
  "execution_group": "async",
  "timeout_ms": <integer>,
  "timestamp": "<ISO-8601>"
}
```

**Mandatory fields**: `type`, `trace_id`, `session_id`, `plugin_name`, `execution_group`, `timeout_ms`, `timestamp`.

### Event 5: `plugin.abandoned`

**Trigger**: The async drain timer fires (`tokio::select!` timer arm in `crates/factory-dispatcher/src/main.rs`, EC-011) while the plugin's forwarding task has not yet delivered a result to the drain channel. The plugin was dispatched but did not complete within the `ASYNC_DRAIN_WINDOW_MS` budget (DI-019). One `plugin.abandoned` event is emitted per in-flight plugin at drain expiry.

**Wire format** (JSON line in `events-*.jsonl`):

```json
{
  "type": "plugin.abandoned",
  "trace_id": "<uuid-v4>",
  "session_id": "<uuid-v4>",
  "plugin_name": "<string>",
  "entry_index": <u32>,
  "drain_window_ms": <integer>,
  "timestamp": "<ISO-8601>"
}
```

**Mandatory fields**: `type`, `trace_id`, `session_id`, `plugin_name`, `entry_index`, `drain_window_ms`, `timestamp`.

**`entry_index` semantics**: The ordinal position (0-based, from `enumerate()`) of this plugin's registry entry in the async partition at the time of dispatch. The `plugin_name` field in `plugin.abandoned` events is the registry entry `name` field verbatim — it is not derived from the WASM binary path or a logical plugin grouping label. The registry schema does NOT enforce `name` uniqueness across entries: it is possible to register two entries with identical `name` values for different event types or tool filters. If both such entries are in-flight at drain time, both emit `plugin.abandoned` events with the same `plugin_name`, making name-only keying ambiguous. Name-only keying collapses those distinct invocations; consumers need the `(plugin_name, entry_index)` tuple to unambiguously identify which registry entry was abandoned. (Note on the production registry: the two `verify-factory-lock` entries carry DIFFERENT `name` values — `verify-factory-lock` and `verify-factory-lock-bash` respectively — and would therefore produce distinct `plugin_name` values in any `plugin.abandoned` events; `entry_index` disambiguation is a schema-level invariant for future-proof consumers of any registry that does not enforce `name` uniqueness.)

**Schema-level defense, not a runtime dispatch gate (F-P7-007):** The concurrent-same-`plugin_name` scenario has no production dispatch path — the production registry has no entries sharing the same `name` value AND overlapping tool-filter patterns that would co-occur in the same async partition invocation (confirmed: `registry.rs` enforces `(name, event, tool)` tuple uniqueness; two same-named entries require different `event` or `tool` values, and co-occurrence in a single async partition dispatch requires both entries to match the same event+tool pair simultaneously, which no production entry pair does). `entry_index` is a **schema-level defense**: its correctness is verified by serialization/property tests over the `plugin.abandoned` event struct — asserting that the 0-based ordinal from `enumerate()` of the async partition is correctly marshalled into the `entry_index` field — not by a runtime concurrent-dispatch fixture. The runtime path (single unique `plugin_name` per async partition in any production dispatch) is covered by normal integration tests.

**`drain_window_ms` semantics**: The effective drain window value at the time the timer fired — `ASYNC_DRAIN_WINDOW_MS` in release builds, or the debug-override value from `VSDD_ASYNC_DRAIN_WINDOW_MS` in debug builds (SEC-003). This is the dispatcher-level drain window, distinct from the per-plugin `timeout_ms` carried by `plugin.timeout` events. Both may apply to the same plugin: `plugin.timeout` fires when the plugin exceeds its per-plugin budget; `plugin.abandoned` fires when the drain window expires regardless of per-plugin timeout status.

**Abandoned-vs-late-completion semantics (F-P1-013)**: See Invariant 6.

**Sink destination (corrected v1.32, ADR-048 §D4 v1.3 — S-25.01 LOCAL adversary pass 3 F-P3-001 MEDIUM; extended v1.33 for Event 10):** Events 1–7 (the original seven, per `host/emit_event.rs`'s established `emit_*` functions) are routed via each function's own emission call — historically described here as "`events-*.jsonl` via FileSink" and "NOT routed to the dispatcher-internal debug stream." This description was INACCURATE for the actual dispatcher-native mechanism, and Events 8, 9, and 10 make the inaccuracy load-bearing: ALL TEN events are emitted via `HostContext::emit_internal`, a DUAL-sink primitive that (a) writes durably to `InternalLog` (`dispatcher-internal-{date}.jsonl`) whenever `ctx.internal_log` is `Some` — this durable write is NOT gated behind the opt-in DI-007 debug-stream flag; it is the ONLY log every default production dispatcher run writes unconditionally — AND (b) pushes onto the `ctx.events` stub queue, drained today by the opt-in `VSDD_SINK_FILE` diagnostic flush and, once S-4.07 wires it into `main.rs`, by the `sinks::Router`/`FileSink` apparatus that would produce a literal `events-{date}.jsonl` file. Until S-4.07 lands (or an operator sets `VSDD_SINK_FILE`), NO default production run durably writes a file literally named `events-{date}.jsonl` — `dispatcher-internal-{date}.jsonl` is the durable ground truth for all ten events today. The VP-028 sink-fan-out invariant applies unchanged: if multiple sinks are configured, all ten events must fan out to all applicable sinks — `emit_internal`'s dual-write IS that fan-out mechanism. **Scope note (v1.35):** this paragraph governs Events 1–10 only; Events 11–13 are written by `main.rs` directly through `InternalLog::write` (not `emit_internal`), land only in `dispatcher-internal-{date}.jsonl`, and are not subject to the VP-028 fan-out (see each event's §Emission path).

### Event 6: `plugin.completed` (async path)

**Note**: `plugin.completed` is emitted for both sync and async plugin completions. The sync-path behavior is governed by BC-1.14.001 and the internal log. This entry covers the **async-path variant** only: when an async group plugin completes within the drain window with a non-block exit code.

**Trigger**: An async group plugin's result arrives on the drain channel receiver (`rx`) before the `tokio::select!` timer arm fires (EC-011), and the result is `PluginResult::Ok` with a non-block exit code. The plugin completed within the `ASYNC_DRAIN_WINDOW_MS` budget (DI-019) without timing out or being abandoned.

**Wire format** (JSON line in `events-*.jsonl`):

```json
{
  "type": "plugin.completed",
  "trace_id": "<uuid-v4>",
  "session_id": "<uuid-v4>",
  "plugin_name": "<string — registry entry name>",
  "plugin_version": "<string>",
  "entry_index": <u32>,
  "exit_code": <integer>,
  "elapsed_ms": <integer>,
  "fuel_consumed": <integer>,
  "timestamp": "<ISO-8601>"
}
```

`stderr` is present only when non-empty (matching sync-path behavior per `extra_fields.retain(|(k, v)| k != "stderr" || ...)` in `crates/factory-dispatcher/src/executor.rs`).

**Mandatory fields**: `type`, `trace_id`, `session_id`, `plugin_name`, `plugin_version`, `entry_index`, `exit_code`, `elapsed_ms`, `fuel_consumed`, `timestamp`.

**`entry_index` semantics**: Mirrors Event 5 (`plugin.abandoned`) — the ordinal position (0-based, from `enumerate()`) of this plugin's registry entry in the async partition at the time of dispatch. The `(plugin_name, entry_index)` tuple unambiguously identifies which registry entry completed, enabling correlation with the corresponding `plugin.invoked` event and exclusion under Invariant 6. See Event 5 `entry_index` semantics paragraph for the full disambiguation rationale. The same schema-level defense applies: correctness of `entry_index` in `plugin.completed` events is verified by property/serialization tests over the event struct, not by a runtime concurrent-dispatch fixture (F-P7-007).

**Invariant 6 interplay**: `plugin.completed` (async path) and `plugin.abandoned` are mutually exclusive for any given `(trace_id, plugin_name, entry_index)` tuple (Invariant 6). When the drain timer fires, the `rx` channel receiver is dropped, precluding completion delivery for abandoned plugins. Conversely, a plugin that delivers its result to `rx` before the timer arm fires cannot subsequently emit `plugin.abandoned` for the same invocation.

### Event 7: `plugin.fuel_headroom_warning`

**Trigger**: `invoke_plugin` (`crates/factory-dispatcher/src/invoke.rs`) completes a WASM plugin invocation with `PluginResult::Ok` and the invocation consumed `fuel_consumed > 0.9 × fuel_cap` (strict, integer-exact comparison). This is a Phase-2 near-term observability mitigation from ADR-039 §Decision 5 Mitigation 1, fully independent of the `failure_policy`/`on_error` fail-closed enforcement machinery — it fires uniformly for every plugin, calibrated or not, and never influences the dispatcher's block decision. **Full behavioral governance (triggering-condition semantics, boundary controls, `headroom_ratio` formula, independence guarantees) is BC-1.03.019 (PC1–PC10); this catalog entry registers the wire-format/field-shape authority only.**

**Wire format** (JSON line in `events-*.jsonl`):

```json
{
  "type": "plugin.fuel_headroom_warning",
  "trace_id": "<uuid-v4>",
  "session_id": "<uuid-v4>",
  "plugin_name": "<string — registry entry name>",
  "fuel_consumed": <integer>,
  "fuel_cap": <integer>,
  "headroom_ratio": <float — fraction of budget REMAINING, clamped [0.0, 1.0]>,
  "level": "warn",
  "message": "fuel-headroom-warning: plugin consumed >90% of budget; next larger input may trap — recalibrate fuel_cap",
  "timestamp": "<ISO-8601>"
}
```

**Mandatory fields**: `type`, `trace_id`, `session_id`, `plugin_name`, `fuel_consumed`, `fuel_cap`, `headroom_ratio`, `level`, `message`, `timestamp`.

**`level` and `message` semantics**: `level` is the literal string `"warn"` — `InternalEvent` has no dedicated severity field of its own, so `level` is carried as an explicit `fields` entry per BC-1.03.019 PC8. `message` reproduces ADR-039 v1.15 §Decision 5's mandated text verbatim (corrected by ADR-039 E-006 from an earlier `≥90%` draft wording to strict `>90%` to match the strict-greater-than trigger predicate) — implementations MUST NOT paraphrase, truncate, or interpolate the plugin name into this string.

**`headroom_ratio` semantics**: The fraction of budget REMAINING, `1.0 - (fuel_consumed / fuel_cap)`, not the fraction consumed — see BC-1.03.019 PC7 for the full rationale.

**Does not affect block decision**: Emitting this event never reads `RegistryEntry.on_error` or `RegistryEntry.failure_policy`, never sets `block_intent`, and never alters the dispatcher's exit code (BC-1.03.019 PC9; Invariant 3 below).

### Event 8: `plugin.indeterminate`

**Trigger**: `invoke_plugin` (`crates/factory-dispatcher/src/executor.rs`) classifies a plugin's outcome as INDETERMINATE — meaning the plugin could not complete. Three mutually exclusive INDETERMINATE causes exist (ADR-047 §Decision 1): (a) `Trap::OutOfFuel` (fuel exhaustion); (b) `Trap::Interrupt` (epoch timeout); (c) `PluginResult::Ok{exit_code:0}` with `host_output_too_large_seen == true` (output too large). This event is emitted for BOTH `failure_policy = "fail-closed"` (marker path, BC-1.18.001) AND `failure_policy = "fail-open"` (advisory-only path, BC-1.18.004) — the `failure_policy` field distinguishes the two on the wire. **Full behavioral governance (outcome classification, marker lifecycle, gate arms, fail-open vs fail-closed routing) is BC-1.18.001 (and BC-1.18.002–004); this catalog entry registers the wire-format/field-shape authority only.**

**Wire format** (JSON line; durably persisted to `dispatcher-internal-{date}.jsonl` via `HostContext::emit_internal` — see §Emission path below):

```json
{
  "type": "plugin.indeterminate",
  "trace_id": "<uuid-v4>",
  "session_id": "<uuid-v4>",
  "plugin_name": "<string — registry entry name>",
  "artifact_path": "<string — path of the artifact being written, from hook envelope>",
  "cause": "<string — one of: fuel | epoch | output-too-large>",
  "failure_policy": "<string — one of: fail-closed | fail-open>",
  "timestamp": "<ISO-8601>"
}
```

**Mandatory fields**: `type`, `trace_id`, `session_id`, `plugin_name`, `artifact_path`, `cause`, `failure_policy`, `timestamp`.

**`cause` semantics**: The exact string `"fuel"` for `Trap::OutOfFuel`; `"epoch"` for `Trap::Interrupt`; `"output-too-large"` for the `host_output_too_large_seen == true` + exit_code=0 path. These are the canonical string forms per ADR-047 §Decision 3 marker `cause` field — the event reuses the same vocabulary.

**`failure_policy` semantics**: The `failure_policy` field value reflects the plugin's registry entry `failure_policy` setting at invocation time: `"fail-closed"` or `"fail-open"` (including the absent/default case, which is coerced to `"fail-open"` per `FailurePolicy::default()`). Consumers can use this field to distinguish marker-writing dispatches from advisory-only dispatches without consulting the registry.

**`artifact_path` semantics**: The filesystem path of the artifact that triggered the hook (from the hook envelope `file_path` or equivalent context field). Used as the marker's `artifact_path` field for fail-closed dispatches (BC-1.18.001 PC4). May be empty string for hooks without an artifact context (e.g., PreToolUse on a non-file-writing tool); implementations MUST emit the empty string rather than omitting the field.

**Does not affect block decision**: The event itself is observational. For `failure_policy = "fail-closed"`, the BLOCK decision comes from the marker-gate mechanism (BC-1.18.002), not from this event. For `failure_policy = "fail-open"`, no block occurs (BC-1.18.004).

**Relationship to `plugin.timeout`**: `plugin.timeout` events are still emitted for epoch and fuel-exhaustion timeouts (sync and async paths per BC-1.14.001). `plugin.indeterminate` is a SEPARATE, ADDITIONAL event carrying the INDETERMINATE classification context (cause enum, artifact_path, failure_policy). Both may fire for the same dispatch; consumers should expect both events on the same `trace_id` when a fuel or epoch timeout maps to INDETERMINATE.

**Emission path (corrected v1.3, ADR-048 §D4 v1.3 — S-25.01 LOCAL adversary pass 3 F-P3-001 MEDIUM):** `emit_indeterminate` (`executor.rs`) emits via `base_ctx.emit_internal(ev)` (`base_ctx: &HostContext`) — the SAME dual-sink helper (durable `InternalLog` write when `ctx.internal_log` is `Some`, AND push onto `ctx.events`) that every OTHER dispatcher-native event in this catalog already uses — NOT a raw `InternalLog::write` call, which is what the frozen S-25.01 implementation used prior to this correction. The `InternalLog` write durably lands in `dispatcher-internal-{date}.jsonl` — the ONLY log every default production dispatcher run writes unconditionally, independent of `observability-config.toml`. See Event 9's §Durable sink target note for the full rationale (identical for both events).

### Event 9: `marker.cleared`

**Trigger**: The `.factory/unvalidated-mutation.marker` file is cleared by one of four paths defined in BC-1.18.003:
- **PC1 REVALIDATED**: `delete_marker_if_pass` in `crates/factory-dispatcher/src/indeterminate_marker.rs` removes the marker after a successful re-validation PASS.
- **PC4 TTL_EXPIRED**: The dispatcher-native `check_and_clear_expired_marker` pre-check (called from `executor.rs`'s tier-execution loop, before the Arm 1/Arm 2 WASM gate plugin runs) auto-deletes an expired marker (TTL-loudness — previously silent; now audited per ADR-048 §Decision 4; emission point corrected v1.2).
- **PC3 OPERATOR_OVERRIDE**: Retroactive reconciliation via RAW_DELETE_DETECTED: the dispatcher-native `reconcile_raw_delete` (same pre-check's marker-absent branch) finds the marker absent on next evaluation with an unmatched `marker.written` (Event 10, new in v1.33) in `dispatcher-internal-{date}.jsonl` (emission point corrected v1.2 — never the WASM gate plugin; scan-target corrected v1.3 — see §Durable sink target below; scan MATCH TYPE corrected v1.33, ADR-048 §D4 v1.4 F-P6-001 — retargeted from an unmatched `plugin.indeterminate`, which is unsound because it fires for every INDETERMINATE outcome regardless of whether a marker was ever written; see Event 10 below).
- **PC5 SUPERSEDED** (v1.3, ADR-048 §D4 v1.3 — F-P3-002; emission point corrected v1.34, ADR-048 §D4 v1.5 — F-P9-001): cross-pair marker overwrite: `write_indeterminate_marker`'s caller in `executor.rs` reads the existing marker's fields, if they belong to a DIFFERENT `(plugin_name, artifact_path)` pair than the new INDETERMINATE event, BEFORE the temp+rename overwrite (BC-1.18.001 INV3 last-writer-wins) — this read is unavoidable, since the write itself overwrites the file the fields are read from — but emits `marker.cleared(SUPERSEDED)` for the superseded pair ONLY AFTER `write_indeterminate_marker` returns `Ok(())`, in the SAME write-success arm as Event 10's `marker.written` emission (SUPERSEDED fires first, then `marker.written`). On `Err(_)`, NEITHER event is emitted (see EC-014/EC-016 below).

**Full behavioral governance (clear predicates, scoping rules, audited-clear obligations, RAW_DELETE_DETECTED reconciliation, SUPERSEDED cross-pair trigger) is BC-1.18.003 (PC1, PC3, PC4, PC5) + BC-1.18.001 (INV3 corollary); this catalog entry registers the wire-format/field-shape authority only.**

**Wire format** (JSON line; durably persisted to `dispatcher-internal-{date}.jsonl` via `HostContext::emit_internal` — see §Emission path below):

```json
{
  "type": "marker.cleared",
  "trace_id": "<uuid-v4 — matches trace_id of the originating plugin.indeterminate event>",
  "session_id": "<uuid-v4>",
  "plugin_name": "<string — from marker TOML plugin_name field>",
  "artifact_path": "<string — from marker TOML artifact_path field>",
  "clear_mode": "<string — one of: REVALIDATED | TTL_EXPIRED | OPERATOR_OVERRIDE | SUPERSEDED>",
  "actor_type": "<string — one of: validator | deadman | operator | system>",
  "reason": "<string | null — mandatory when clear_mode=OPERATOR_OVERRIDE or clear_mode=SUPERSEDED; null otherwise>",
  "timestamp": "<ISO-8601 UTC — time of the clear event, not the original INDETERMINATE event>"
}
```

**Mandatory fields**: `type`, `trace_id`, `session_id`, `plugin_name`, `artifact_path`, `clear_mode`, `actor_type`, `reason` (conditional: mandatory for OPERATOR_OVERRIDE and SUPERSEDED, null/omitted otherwise), `timestamp`.

**`clear_mode` / `actor_type` correspondence:**

| `clear_mode` | `actor_type` | Trigger | Emission point |
|---|---|---|---|
| `REVALIDATED` | `validator` | `delete_marker_if_pass` removes marker after PASS (BC-1.18.003 PC1) | `emit_marker_cleared` in `indeterminate_marker.rs`, called from `delete_marker_if_pass` after `remove_file` succeeds, emitting via `ctx.emit_internal` |
| `TTL_EXPIRED` | `deadman` | Dispatcher-native `check_and_clear_expired_marker` pre-check auto-delete (BC-1.18.003 PC4; ADR-048 §D4 v1.2) | `check_and_clear_expired_marker` in `indeterminate_marker.rs` — dispatcher-native, called from `executor.rs`'s tier-execution loop BEFORE the Arm 1/Arm 2 WASM gate plugin runs, emitting via `ctx.emit_internal`; NOT emitted from inside `evaluate_gate` |
| `OPERATOR_OVERRIDE` | `operator` | Retroactive: dispatcher-native `reconcile_raw_delete` finds marker absent + unmatched `marker.written` (Event 10; BC-1.18.003 PC3; ADR-048 §D4 v1.2/v1.4) | `reconcile_raw_delete` in `indeterminate_marker.rs` — dispatcher-native, invoked from the same pre-check's marker-absent branch, emitting via `ctx.emit_internal`; NOT emitted from inside the WASM gate plugin. Scan match-type corrected v1.33 (ADR-048 §D4 v1.4) — matches unmatched `marker.written`, not unmatched `plugin.indeterminate` |
| `SUPERSEDED` (v1.3) | `system` | Cross-pair marker overwrite: `write_indeterminate_marker`'s caller detects the existing marker belongs to a DIFFERENT `(plugin_name, artifact_path)` pair than the new INDETERMINATE event (BC-1.18.001 INV3 last-writer-wins; BC-1.18.003 PC5; ADR-048 §D4 v1.3 — F-P3-002) | `write_indeterminate_marker`'s caller in `executor.rs` reads the superseded pair's fields BEFORE the temp+rename overwrite (unavoidable — the write overwrites the file the fields are read from) and calls `emit_marker_cleared(ctx, ..., SUPERSEDED, system, Some(reason))`, emitting via `ctx.emit_internal`, ONLY AFTER `write_indeterminate_marker` returns `Ok(())` — the SAME write-success arm as Event 10's `marker.written` emission, SUPERSEDED first then `marker.written` (**corrected v1.34, ADR-048 §D4 v1.5, F-P9-001** — previously emitted unconditionally at the pre-overwrite read, before the write was even attempted); on `Err(_)`, NEITHER event is emitted. NOT emitted for a same-pair re-INDETERMINATE (no cross-pair change) |

**`trace_id` semantics**: Links to the originating `plugin.indeterminate` event. For REVALIDATED and TTL_EXPIRED, the `trace_id` is read from the marker TOML `trace_id` field written at INDETERMINATE time. For OPERATOR_OVERRIDE, the `trace_id` is taken from the unmatched `marker.written` record found via the bounded `dispatcher-internal-{date}.jsonl` scan (corrected v1.33, ADR-048 §D4 v1.4 — previously sourced from the unmatched `plugin.indeterminate` record; same field, corrected source). For SUPERSEDED, the `trace_id` is the SUPERSEDED (old) pair's own `trace_id`, read from its marker TOML fields before the overwrite — NEVER the new (overwriting) pair's `trace_id`.

**`reason` semantics**: Null (or omitted) for REVALIDATED and TTL_EXPIRED. For OPERATOR_OVERRIDE, MUST be the literal string: `"RAW_DELETE_DETECTED: marker absent without prior marker.cleared event; inferred operator out-of-band clear"`. For SUPERSEDED (v1.3), MUST be the literal string: `"SUPERSEDED: marker overwritten by a new plugin.indeterminate event for a different (plugin_name, artifact_path) pair before being cleared; last-writer-wins (BC-1.18.001 INV3)"`.

**Proportionality**: No signed digests or dual-control required. Append-only, durably-persisted event is proportionate for the cooperating-agent threat model (VSDD baseline). GitHub branch-protection and the `factory-artifacts` worktree provide tamper-evidence at the VCS layer (ADR-048 §Decision 4 rationale).

**Emission path (corrected v1.3, ADR-048 §D4 v1.3 — S-25.01 LOCAL adversary pass 3 F-P3-001 MEDIUM):** `emit_marker_cleared` emits via `HostContext::emit_internal(ev)` (`ctx: &HostContext`) — the SAME dual-sink helper (writes durable `InternalLog` when `ctx.internal_log` is `Some`, AND pushes onto the `ctx.events` stub queue) that every OTHER dispatcher-native BC-3.08.001 event already uses (`emit_dispatcher_schema_mismatch`, `emit_registry_invalid_e_reg002`/`_e_reg003`, `emit_plugin_abandoned`, `emit_plugin_async_block_discarded`, `emit_plugin_completed_async`, `emit_plugin_timeout_async`) — NOT a raw `InternalLog::write` call, which is what the frozen S-25.01 implementation used prior to this correction. `plugin.indeterminate` (Event 8, `emit_indeterminate` in `executor.rs`) is corrected the same way. `emit_internal` does NOT re-enrich or overwrite the event's fields (unlike the WASM `emit_event` host ABI's RESERVED_FIELDS wall), so the marker-derived `trace_id`/`plugin_name` set before the call are preserved exactly. Per ADR-048 §D4 v1.2, emission is called exclusively from dispatcher-native code (`delete_marker_if_pass`, `check_and_clear_expired_marker`, `reconcile_raw_delete`, and — v1.3 — `write_indeterminate_marker`'s caller for SUPERSEDED, all in `indeterminate_marker.rs`/`executor.rs`); never from inside the WASM gate plugin via the `emit_event` host ABI. The crash-path native TTL check in the dispatcher (`block_if_marker_check`, Decision 1 — crash + expired marker → Allow) does NOT emit `marker.cleared(TTL_EXPIRED)` — only the dispatcher-native pre-check (`check_and_clear_expired_marker`) emits it. **Emission-point correction (v1.34, ADR-048 §D4 v1.5 — F-P9-001):** the SUPERSEDED call is no longer invoked unconditionally at the pre-overwrite read; `write_indeterminate_marker`'s caller now calls it only after confirming `Ok(())`, in the same write-success arm as the `emit_marker_written` call (Event 10) — SUPERSEDED first, then `marker.written`. On `Err(_)`, `write_indeterminate_marker`'s caller emits neither event, closing a fabricated-audit-record class identical to the one the v1.33 `marker.written` "emit only after `Ok(())`" rule already closed for Event 10.

**Durable sink target (v1.3 precision correction):** `HostContext::emit_internal`'s `InternalLog` write lands in `<InternalLog log_dir>/dispatcher-internal-{date}.jsonl` — the ONLY log every default production dispatcher run writes unconditionally, independent of `observability-config.toml`. This is a DELIBERATE correction from the literal `events-{date}.jsonl` filename this catalog entry (and ADR-048 v1.1/v1.2 prose) previously named: no default production dispatcher run durably writes a file literally named `events-{date}.jsonl` today — the `sinks::Router`/`FileSink` apparatus (`crates/factory-dispatcher/src/sinks/mod.rs`) that would produce one is not yet wired into `main.rs` (pending S-4.07), and `VSDD_SINK_FILE` is an opt-in diagnostic flag, absent by default. `reconcile_raw_delete`'s bounded scan for OPERATOR_OVERRIDE reconciliation (above) targets this SAME `dispatcher-internal-{date}.jsonl` file. `HostContext::emit_internal` still ALSO reaches `ctx.events`, so once S-4.07 wires the Router/FileSink into `main.rs`, or an operator sets `VSDD_SINK_FILE`, Events 8/9 additionally appear at an `events-*.jsonl`-shaped path, with no further code change. This correction applies to Events 8 and 9 only (the two events this ADR governs); the other seven events' emission sites and sink destinations are unaffected.

### Event 10: `marker.written`

**Trigger**: `write_indeterminate_marker` (`crates/factory-dispatcher/src/indeterminate_marker.rs`) returns `Ok(())` — the atomic temp-file-then-`rename` write of `.factory/unvalidated-mutation.marker` succeeded. Its caller in `crates/factory-dispatcher/src/executor.rs` (the same INDETERMINATE-outcome callsite that also performs the Invariant 3 SUPERSEDED check, BC-1.18.001 EC-009/BC-1.18.003 PC5) emits `marker.written` immediately after the `Ok(())` return — AFTER the Event 9 SUPERSEDED emission, if applicable (**corrected v1.34, ADR-048 §D4 v1.5 — F-P9-001**: SUPERSEDED is now gated on the SAME `Ok(())` return rather than firing unconditionally at the pre-overwrite read, so both write-tied events fire back-to-back in this one arm — SUPERSEDED for the superseded pair, then `marker.written` for the new pair). `marker.written` is NEVER emitted when `write_indeterminate_marker` returns `Err(_)` (BC-1.18.001 EC-007 — atomic rename failure, e.g. disk full or permissions) and is NEVER emitted for a PreToolUse INDETERMINATE (BC-1.18.001 Invariant 4 — marker write is PostToolUse-only, so `write_indeterminate_marker` is never called on the PreToolUse path at all).

**Full behavioral governance (write-success trigger, ONLY-after-`Ok(())` constraint, reconciliation-soundness rationale) is BC-1.18.001 §PC4 (v1.4 addition); this catalog entry registers the wire-format/field-shape authority only — the same split already established for Events 7/8/9.**

**Wire format** (JSON line; durably persisted to `dispatcher-internal-{date}.jsonl` via `HostContext::emit_internal` — see §Emission path below):

```json
{
  "type": "marker.written",
  "trace_id": "<uuid-v4 — the marker's own trace_id, i.e. the current dispatch's trace_id>",
  "session_id": "<uuid-v4>",
  "plugin_name": "<string — the marker's own plugin_name (the plugin whose INDETERMINATE outcome caused the write)>",
  "artifact_path": "<string — the marker's own artifact_path field>",
  "cause": "<string — one of: fuel | epoch | output-too-large>",
  "expires_at": "<ISO-8601 UTC — the marker's own expires_at field>",
  "ts": "<ISO-8601 UTC — emission timestamp; reconcile_raw_delete reads this common field, matching its existing convention for plugin.indeterminate, not a distinct top-level 'timestamp' field>"
}
```

**Mandatory fields**: `type`, `trace_id`, `session_id`, `plugin_name`, `artifact_path`, `cause`, `expires_at`, `ts` (the common emission-timestamp field — see note below; `marker.written` does NOT carry a separate top-level `timestamp` field the way Events 8 and 9 do).

**`cause` semantics**: Identical vocabulary to Event 8's `cause` field — the exact string `"fuel"`, `"epoch"`, or `"output-too-large"` — copied verbatim from the marker's own `cause` field at the moment of the write (ADR-047 §Decision 3 marker `cause` field; same canonical strings reused per Event 8).

**`expires_at` semantics**: Copied verbatim from the marker's own `expires_at` TOML field (BC-1.18.001 PC4 — `timestamp + UNVALIDATED_MUTATION_MARKER_TTL_SECONDS`, 86400s). This field lets `reconcile_raw_delete` reconstruct the full `MarkerFields` needed for a retroactive `marker.cleared(OPERATOR_OVERRIDE)` emission (Event 9) directly from the matched `marker.written` scan record, without a second read of the (now-absent) marker file.

**`ts` field note**: Unlike Events 8 and 9, which additionally carry a distinct top-level `timestamp` field alongside the common `ts`/`ts_epoch` fields, `marker.written`'s only time field is the common `ts` (see §Common Fields). `reconcile_raw_delete`'s scan reads `ts` from the matched `marker.written` record when reconstructing fields — this matches the scan's existing convention for reading `plugin.indeterminate` records (which also rely on the common `ts` field, not a distinct `timestamp`).

**Relationship to `marker.cleared` (Event 9) OPERATOR_OVERRIDE reconciliation**: `marker.written` is the positive creation record that makes `reconcile_raw_delete`'s RAW_DELETE_DETECTED inference sound BY CONSTRUCTION rather than inferred from a proxy signal (ADR-048 §D4 v1.4 — S-25.01 adversary pass 6 F-P6-001 MEDIUM). The pre-v1.33 catalog matched Event 9's OPERATOR_OVERRIDE reconciliation against an unmatched `plugin.indeterminate` (Event 8) — but Event 8 is emitted for EVERY INDETERMINATE outcome regardless of hook phase (fail-closed and fail-open, PreToolUse and PostToolUse alike), while the marker write itself is conditionally gated (PostToolUse AND fail-closed AND the atomic write actually succeeding). Those two are not equivalent: a PreToolUse fail-closed INDETERMINATE (BC-1.18.001 Invariant 4) or a PostToolUse marker-write I/O failure (BC-1.18.001 EC-007) both produce an unmatched `plugin.indeterminate` with no marker ever written, which a scan matching on `plugin.indeterminate` would misread as a raw delete and fabricate `marker.cleared(OPERATOR_OVERRIDE)` for — a false NIST AU-3/AU-10 non-repudiation record. `marker.written` exists if and only if a marker was actually, durably written, so an unmatched `marker.written` is proof-by-construction of a genuine raw delete. See Event 9's `clear_mode`/`actor_type` correspondence table (above) and §Edge Cases EC-015/EC-016 (below) for the corrected reconciliation behavior.

**Does not affect block decision**: Like Events 8 and 9, `marker.written` is a pure observability/audit event — it never influences the marker-gate BLOCK/Allow decision (BC-1.18.002), which is driven solely by marker file presence.

**Emission path**: `emit_marker_written(ctx, &fields)` (`ctx: &HostContext`) in `indeterminate_marker.rs` emits via `ctx.emit_internal(ev)` — the SAME dual-sink helper (durable `InternalLog` write to `dispatcher-internal-{date}.jsonl` when `ctx.internal_log` is `Some`, AND push onto `ctx.events`) every other dispatcher-native BC-3.08.001 event already uses, including its own sibling Events 8 and 9 (post-v1.32/v1.3 correction). Called from `write_indeterminate_marker`'s caller in `executor.rs`, on the `Ok(())` arm only, immediately after the SUPERSEDED check (Event 9, if applicable) and BEFORE control returns from the INDETERMINATE-outcome handling block. Never called on the `Err(_)` arm.

**Durable sink target**: Identical to Events 8 and 9 — `dispatcher-internal-{date}.jsonl` via `HostContext::emit_internal`'s `InternalLog` write, the ONLY log every default production dispatcher run writes unconditionally. `reconcile_raw_delete`'s bounded scan (Event 9, above) targets this SAME file. See Event 9's §Durable sink target note for the full rationale (identical for all three of Events 8/9/10).

### Event 11: `migration.admission_blocked`

**Trigger** (v1.35; ADR-052 §Decision 5a "Admission diagnostics channel (v1.21)"): the dispatcher's governed-migration writer-admission gate returns an `E-MAINTENANCE-001` block verdict for a protected-path `Edit`/`Write`/`MultiEdit` PreToolUse (BC-1.18.013 Precondition 6(b); BC-1.18.011 Precondition 6(b)/(d)). The shared core (`executor::migration_writer_admission*`) returns the verdict's `AdmissionDiagnostic` as DATA; the `main.rs` shell writes exactly ONE `migration.admission_blocked` event per `E-MAINTENANCE-001` verdict immediately after the call and BEFORE the early return, so a blocked dispatch is logged. Never emitted for an admitted write, for an out-of-scope (non-protected / out-of-root) path, or for an `E-MAINTENANCE-002` error (Event 12). Full triggering-condition/semantics authority is BC-1.18.013 Postcondition 10 (and BC-1.18.011 Postcondition 10); this catalog entry registers the wire-format/field-shape authority only.

**Wire format** (JSON line; durably persisted to `dispatcher-internal-{date}.jsonl` — see §Emission path below):

```json
{
  "type": "migration.admission_blocked",
  "trace_id": "<uuid-v4>",
  "session_id": "<uuid-v4>",
  "scope": "<string — BC-INDEX | .factory/cycles/ (the E-MAINTENANCE-001 <scope> token, keyed on the written path family)>",
  "family": "<string — bc_index | cycles>",
  "branch": "<string — one of: gate_only | live_txn | foreign_migration | live_coordinator | completion_record_mismatch>",
  "gate_state": "<string — OPEN | DRAINING | LOCKED>",
  "migration_id": "<string | null — sanitized live-txn migration_id; null when no live txn>",
  "txn_id": "<string | null — sanitized live-txn id; null when no live txn>",
  "check": "<string | null — non-null iff branch=completion_record_mismatch, then EXACTLY one of the closed nine tokens: staging_with_terminal_record | terminal_record_unparseable | terminal_record_schema_mismatch | txn_id_mismatch | generation_id_mismatch | canonical_paths_count_mismatch | canonical_hash_mismatch | terminal_record_unverified | finalize_unwired; else null>",
  "reconciliation": "<string — closed six-token domain of the effectful reconciliation outcome: live_coordinator | nothing_to_reconcile | gate_reopened | null_generation_txn_aborted | foreign_migration_refused | completion_record_mismatch>",
  "ts": "<ISO-8601 UTC>",
  "ts_epoch": 0,
  "schema_version": 1
}
```

**Mandatory fields**: `type`, `trace_id`, `session_id`, `scope`, `family`, `branch`, `gate_state`, `migration_id`, `txn_id`, `check`, `reconciliation`, `ts`, `ts_epoch`, `schema_version` (the nullable fields are always PRESENT, with value `null` when not applicable). `plugin_name` MUST NOT appear.

**Field semantics**: `scope`/`family` pair 1:1 (`BC-INDEX`/`bc_index`, `.factory/cycles/`/`cycles`). `branch`: `gate_only` = block with no live txn (gate DRAINING/LOCKED); `live_txn` = live txn (STAGING/COMMITTING) with the terminal record absent or no repair possible; `foreign_migration` = live txn whose `migration_id ∉ K`; `live_coordinator` = `exclusive.lock` held (EWOULDBLOCK); `completion_record_mismatch` = Branch C verification failure, or a VERIFIED Branch C finalize that the S-25.09 seam cannot yet perform (`check = "finalize_unwired"`; retired when S-25.06 delivers the finalize — a verified finalize is then admitted with no Event 11) — the verdict message carries the mismatch suffix in both cases; **`check` value domain (CLOSED, nine tokens; normative text BC-1.18.013 Postcondition 10(a); v1.35 follow-up 2):** verification checks, evaluated in this order with the first failure winning: `staging_with_terminal_record` (STAGING + own terminal record; always fail-closed), `terminal_record_unparseable`, `terminal_record_schema_mismatch`, `txn_id_mismatch`, `generation_id_mismatch`, `canonical_paths_count_mismatch`, `canonical_hash_mismatch`; plus the S-25.09-seam tokens `terminal_record_unverified` (COMMITTING + record present, verifier not delivered) and `finalize_unwired`, BOTH retired once S-25.06 delivers the verifier and finalize. The values are compile-time constants (never on-disk content); a terminal-record/canonical-file read CALL failure is `E-MAINTENANCE-002 (io)` and emits Event 12, not Event 11; any value outside the nine is a conformance failure (a verification failure is ONE `migration.admission_blocked` with `check` = the failing check and NO `migration.admission_advisory`; the `finalize_unwired` case is ONE `migration.admission_blocked` plus the `branch_c_finalize_unwired` Event 13). **`branch` derivation (total; ADR-052 §Downstream item 33(b)):** `branch = live_coordinator` ⇔ `reconciliation = live_coordinator`; `branch = foreign_migration` ⇔ `reconciliation = foreign_migration_refused`; `branch = completion_record_mismatch` ⇔ `reconciliation = completion_record_mismatch`; otherwise `branch = gate_only` if no live txn remains, else `branch = live_txn`. **`reconciliation` (closed six-token domain, no `none` — reconciliation always runs before a block):** `live_coordinator` = `exclusive.lock` held (EWOULDBLOCK), no repair; `nothing_to_reconcile` = the reconciliation core returned `NoOp` (nothing to repair); `gate_reopened` = Branch A repaired a stuck gate; `null_generation_txn_aborted` = Branch B discarded a STAGING `generation_id=null` txn; `foreign_migration_refused` = live txn with `migration_id ∉ K` refused (never finalized/aborted); `completion_record_mismatch` = Branch C verification failed. **Sanitization (load-bearing):** `migration_id`, `txn_id` and `check` are derived from on-disk record content and are therefore SANITIZED — control characters escaped, truncated to 64 characters (BC-1.18.013 EC-029). No field carries a raw `tool_use_id` or record content.

**Does not affect block decision**: the event is written after the verdict is computed; the verdict, message and exit code are unchanged by it (Invariant 3).

**Emission path**: `main.rs` writes `internal_log.write(&InternalEvent::now(MIGRATION_ADMISSION_BLOCKED).with_trace_id(..).with_session_id(..).with_field(..))` DIRECTLY via `InternalLog::write` — NOT `HostContext::emit_internal` (no `HostContext` exists at the registry-independent admission position, ADR-052 §Decision 5a "Evaluation position"); this is the one deliberate deviation from the dual-sink pattern of Events 1–10, recorded here. The event is NOT pushed onto `ctx.events`. Constant `MIGRATION_ADMISSION_BLOCKED` in `crates/factory-dispatcher/src/internal_log.rs`.

**Durable sink target**: `dispatcher-internal-{date}.jsonl` (ADR-024 log-dir resolution) — the sole sink; unconditional, not gated behind the opt-in DI-007 debug-stream flag.

### Event 12: `migration.admission_failed`

**Trigger** (v1.35): the writer-admission check returns an `E-MAINTENANCE-002` error verdict — it could not complete and FAILED CLOSED (BC-1.18.013 Precondition 6(c) "`E-MAINTENANCE-002` `<cause>` classification"; BC-1.18.011 Precondition 6(c)/(d)). `main.rs` writes exactly ONE `migration.admission_failed` event per `E-MAINTENANCE-002` verdict, before the early return. Never emitted for an `E-MAINTENANCE-001` block (Event 11) or an admitted write.

**Wire format** (JSON line; durably persisted to `dispatcher-internal-{date}.jsonl`):

```json
{
  "type": "migration.admission_failed",
  "trace_id": "<uuid-v4>",
  "session_id": "<uuid-v4>",
  "cause": "<string — one of: invalid_tool_use_id | io | state_integrity>",
  "kind": "<string | null — the AdmissionStateIntegrityKind token when cause=state_integrity (closed SIX-token domain: gate_record_malformed | txn_record_malformed | txn_migration_id_not_string | multiple_live_txns | reservation_serialization | txn_record_newer_schema), else null>",
  "detail": "<string — path / io::Error kind+message / byte length; sanitized, never a raw tool_use_id and never record content>",
  "ts": "<ISO-8601 UTC>",
  "ts_epoch": 0,
  "schema_version": 1
}
```

**Mandatory fields**: `type`, `trace_id`, `session_id`, `cause`, `kind`, `detail`, `ts`, `ts_epoch`, `schema_version` (`kind` always present, `null` unless `cause = state_integrity`). `plugin_name` MUST NOT appear.

**Field semantics**: `cause` is the `AdmissionFailureCause::token()` of the closed classification (BC-1.18.013 Invariant 7); `kind` is the `AdmissionStateIntegrityKind` token carried by `BcIndexMigrationError::AdmissionStateIntegrity { kind, detail }`, drawn from the CLOSED six-token domain above (v1.37: `txn_record_newer_schema` = a txn record whose `schema_version` is an integer >= 2, written by a newer build; additive token, emitted through the existing writer unchanged; no other token may appear). `detail` carries the underlying detail the single-line operator message deliberately omits: for `invalid_tool_use_id`, the byte length of the offending value ONLY; for `io`, the path and the `io::Error` kind and message; for `state_integrity`, the path and parse message — NEVER a raw `tool_use_id` and NEVER record content; sanitized (control characters escaped, truncated to 64 characters for any data-derived substring) (InternalLog event only; the operator stderr rendering of the same fact is capped at 256 per BC-1.18.013 Postcondition 10). The `AdmissionStateIntegrity` / `Io` facts carry their data-derived slots RAW and are sanitized at render time by the surface that renders them (64 per substring here, 256 on stderr — ADR-052 v1.25 item 11(g)); the `detail` string is never re-truncated after escaping.

**Emission path / Durable sink target**: identical to Event 11 (`InternalLog::write` directly from `main.rs`; constant `MIGRATION_ADMISSION_FAILED`; `dispatcher-internal-{date}.jsonl` only; not pushed onto `ctx.events`). Does not affect the verdict, message or exit code.

### Event 13: `migration.admission_advisory`

**Trigger** (v1.35): a non-verdict anomaly in the admission/reconciliation/release legs that the operator should be able to query but that does not change the dispatch outcome (BC-1.18.013 Postcondition 10(c)): a non-ENOENT reservation-release error (`reservation_release_failed`), a Branch A gate re-open (`branch_a_gate_reopened`), a Branch B null-generation txn discard (`branch_b_txn_aborted`), the Branch C finalize leg when `decide_terminal_record_reconciliation` returns `FinalizeThenOpenGate` and the finalize effect is undelivered (`branch_c_finalize_unwired` — S-25.09 seam; written IN ADDITION to the one `migration.admission_blocked`, which carries `reconciliation="completion_record_mismatch"`, `branch="completion_record_mismatch"`, `check="finalize_unwired"` (item 33(g)); retired when S-25.06 delivers the finalize), and S-25.06's successful verify-then-finalize (`branch_c_finalized` — the write is then admitted, so no `_blocked`; neither S-25.08 nor S-25.09 emits it). The five reservation-timestamp tokens (`created_at_unparseable`, `created_at_pre_epoch`, `created_at_future`, `mtime_future`, `age_unknown`) are NOT advisory reasons: they are coordinator-drain STDERR tokens (only the drain GC calls `reservation_is_stale`; admission never reads reservation files). `main.rs` writes exactly ONE `migration.admission_advisory` event per anomaly (the release leg returns its own diagnostics list). An admitted write with no anomaly writes none.

**Wire format** (JSON line; durably persisted to `dispatcher-internal-{date}.jsonl`):

```json
{
  "type": "migration.admission_advisory",
  "trace_id": "<uuid-v4>",
  "session_id": "<uuid-v4>",
  "reason": "<string — one of: reservation_release_failed | branch_a_gate_reopened | branch_b_txn_aborted | branch_c_finalize_unwired | branch_c_finalized>",
  "ts": "<ISO-8601 UTC>",
  "ts_epoch": 0,
  "schema_version": 1
}
```

**Mandatory fields**: `type`, `trace_id`, `session_id`, `reason`, `ts`, `ts_epoch`, `schema_version`. Optional context fields are the CLOSED set {`migration_id`, `txn_id`, `check`, `detail`, `tool_use_id_len`} — any other field is forbidden; each string field is sanitized as in Event 11, `tool_use_id_len` is the byte length, and a `tool_use_id` is NEVER logged raw. `plugin_name` MUST NOT appear.

**Emission path / Durable sink target**: identical to Event 11 (`InternalLog::write` directly from `main.rs`; constant `MIGRATION_ADMISSION_ADVISORY`; `dispatcher-internal-{date}.jsonl` only). Does not affect the dispatch outcome. (The two coordinator binaries, being CLI processes, additionally write their own anomalies as single stderr lines — ADR-052 §Decision 5a "Admission diagnostics channel" (5); that stderr surface is not an event.)

## Invariants

1. **`trace_id` is mandatory on all thirteen event types**: Per DI-017, every emitted event carries the UUID v4 from the invoking hook envelope. These thirteen events are no exception (Events 11–13 are set by `main.rs` via `.with_trace_id(..)` from the dispatch's envelope context).
2. **Events are write-once, no retry**: These are diagnostic events; partial emission is acceptable (emit-then-crash). They are never retried on FileSink write failure.
3. **Events do not affect dispatcher exit code**: All thirteen are observability-only (Events 11–13 accompany an `E-MAINTENANCE-001` / `E-MAINTENANCE-002` verdict or an advisory anomaly but never cause it; the verdict's message and exit code are computed before, and independent of, the event write). `plugin.async_block_discarded`, `plugin.timeout (async)`, `plugin.abandoned`, `plugin.completed` (async path), `plugin.fuel_headroom_warning`, `plugin.indeterminate`, `marker.cleared`, and `marker.written` are logged and forgotten. `dispatcher.schema_mismatch` and `dispatcher.registry_invalid` accompany a hard exit (non-zero) but the event itself does not cause the exit — the validation failure does. For `plugin.indeterminate` with `failure_policy=fail-closed`, the block is issued by the marker-gate mechanism (BC-1.18.002), not by this event emission. `marker.cleared` and `marker.written` are pure audit events: they record a state change (or creation) but never influence block decisions.
4. **`plugin.async_block_discarded` reason field is the literal string `"async_plugin_block_verdict_discarded"`**: Not an error code; a diagnostic reason string for human-readable log inspection.
5. **`trace_id` is the exclusive wire-format field name for the trace correlation value**: The dispatcher's structured-event wire format uses field name `trace_id` exclusively. The legacy field name `dispatcher_trace_id` MUST NOT appear in the serialized wire output. Plugins MUST NOT emit a `trace_id` field via `with_field()` — `trace_id` is reserved for the dispatcher (see §Implementation Notes). Reference: DI-017 (amended per F-P1-007).
6. **`plugin.abandoned` and `plugin.completed` (async path) are mutually exclusive terminal outcomes per invocation**: When the async drain timer fires (EC-011), `plugin.abandoned` is the last observable event for each in-flight plugin in this invocation. No `plugin.completed` event fires after `plugin.abandoned` for the same `trace_id` + `plugin_name` + `entry_index` tuple. Conversely, a plugin that emits `plugin.completed` (async path — Event 6) within the drain window cannot subsequently emit `plugin.abandoned` for the same tuple; the drain timer fires only for plugins still in-flight at expiry. Rationale for semantics option (a) abort-at-drain (F-P1-013): the dispatcher exits shortly after the drain window expires; in-flight Tokio tasks that complete after the `break` have no live FileSink to emit from. The `rx` channel receiver is dropped at drain timer fire, so any late result send is silently discarded. Option (b) (both events may fire) is structurally impossible under the current single-process lifecycle — the process exits before abandoned tasks can complete and write to a live sink. Option (c) (suppress `plugin.completed` after `plugin.abandoned`) is mechanically equivalent to (a) without benefit; no suppression logic is needed when the emission path is already closed. **Schema-level note (F-P7-007):** The `(trace_id, plugin_name, entry_index)` mutual-exclusivity key is a schema-level predicate — its correctness as a disambiguation key is verifiable by property/serialization tests over the event structs (asserting both events carry the same tuple fields with consistent types and ordinal derivation), not by a runtime concurrent-dispatch fixture that exercises two same-named entries simultaneously.
7. **Events 11–13 are exactly-once per verdict/anomaly, InternalLog-only, and never carry a raw `tool_use_id` or record content (v1.35)**: each `E-MAINTENANCE-001` verdict produces exactly one Event 11, each `E-MAINTENANCE-002` verdict exactly one Event 12, each non-verdict anomaly exactly one Event 13 (BC-1.18.013 Postcondition 10); they are written by `main.rs` through `InternalLog::write` directly, are NOT pushed onto `ctx.events` (so they do not reach the opt-in `VSDD_SINK_FILE` flush or a future FileSink), and carry no `plugin_name`. No field of any of the three carries a raw `tool_use_id` (at most its byte length) or on-disk record content; string fields derived from disk/payload data are sanitized (control characters escaped, truncated to 64 characters for any data-derived substring) (InternalLog event only; the operator stderr rendering of the same fact is capped at 256 per BC-1.18.013 Postcondition 10).

## Implementation Notes

### RESERVED_FIELDS and `trace_id` (F-P1-007, O-P15-003)

Implementations MUST add `trace_id` to the host-side reserved-fields filter (e.g., `RESERVED_FIELDS` in `crates/factory-dispatcher/src/host/emit_event.rs`). This prevents plugins from spoofing the dispatcher's trace correlation value via `with_field("trace_id", ...)`.

The reserved-fields filter MUST also retain `dispatcher_trace_id` for backward defense (defense-in-depth): even though the dispatcher no longer emits `dispatcher_trace_id` on the wire, plugins must not be allowed to inject it. Both names are reserved regardless of which name the dispatcher currently uses as the canonical field.

**Full `RESERVED_FIELDS` enumeration** (O-P15-003 — authoritative source: the `RESERVED_FIELDS` constant in `crates/factory-dispatcher/src/host/emit_event.rs`):

| Field name | Reason reserved |
|------------|----------------|
| `trace_id` | Canonical wire-format trace correlation value; dispatcher-owned per DI-017 and Invariant 5 of this BC (added by F-P1-007) |
| `dispatcher_trace_id` | Legacy field name; retained for defense-in-depth per F-P1-007; dispatcher no longer emits this on the wire but plugins must not inject it |
| `session_id` | Claude Code session identifier; dispatcher-owned; injected from `ctx.session_id` (see §Common Fields) |
| `plugin_name` | Plugin identity; dispatcher-owned; injected from `ctx.plugin_name` |
| `plugin_version` | Plugin version; dispatcher-owned; injected from `ctx.plugin_version` |
| `ts` | Emission timestamp (internal format); dispatcher-owned |
| `ts_epoch` | Emission timestamp as Unix epoch milliseconds; dispatcher-owned |
| `schema_version` | Registry schema version; dispatcher-owned |
| `type` | Event type string; provided by the emitting call site (not the plugin's field buffer); reserved to prevent shadowing |

Plugins that attempt to set any of these fields via `with_field()` MUST have the field silently stripped by the host-side filter before serialization. The full set is tested by the `reserved_fields_rejected` integration test in `crates/factory-dispatcher/src/host/emit_event.rs`.

For canonical HOST_ABI documentation of which fields the dispatcher enriches automatically, see `crates/hook-sdk/HOST_ABI.md` §`emit_event`. Note: HOST_ABI.md uses the legacy name `dispatcher_trace_id` in its enrichment description (§`emit_event` enrichment description; source-line carve-out per TD-VSDD-091: line 267 is unstable, stable anchor is §`emit_event` section) — the actual wire-format name is `trace_id` per BC-3.08.001 Invariant 5. RESERVED_FIELDS in `emit_event.rs` is the authoritative implementation reference.

## Error Paths

| Condition | Behavior |
|-----------|----------|
| FileSink write fails during event emission | Emission silently dropped; no retry; dispatcher continues (or exits if the triggering error requires exit) |
| `trace_id` unavailable at emission time | Event emitted with `trace_id: null`; this is a last-resort fallback; per DI-017 all events should have trace_id |

## Related BCs

- BC-1.14.001 — events `plugin.async_block_discarded` and `plugin.timeout` (async) originate from the dispatch loop defined there; this BC provides the SS-03 catalog authority
- BC-7.06.001 — events `dispatcher.schema_mismatch` and `dispatcher.registry_invalid` originate from the registry validation path defined there; this BC provides the SS-03 catalog authority
- BC-3.07.002 — sibling: `internal.sink_error` event catalog; same pattern (SS-03 catalogues events emitted by other subsystems)
- BC-1.03.019 — event `plugin.fuel_headroom_warning` (Event 7) originates from `invoke_plugin` (`crates/factory-dispatcher/src/invoke.rs`) as defined there; this BC provides the SS-03 wire-format/field-shape catalog authority, while BC-1.03.019 remains the triggering-condition/semantics authority (threshold predicate, boundary controls, `headroom_ratio` formula, independence from `on_error`/`failure_policy`)
- BC-1.18.001 — event `plugin.indeterminate` (Event 8) triggering-condition/semantics authority (outcome classification, INDETERMINATE causes, fail-closed marker path) AND event `marker.written` (Event 10, v1.33 addition) triggering-condition/semantics authority (§PC4 v1.4 — emitted only after a confirmed successful marker write, never before, never on write failure); this BC provides the SS-03 wire-format/field-shape catalog authority only for both
- BC-1.18.002 — gate behavior when marker exists (follow-on from Event 8 fail-closed path)
- BC-1.18.003 — event `marker.cleared` (Event 9) clear-path/lifecycle authority (REVALIDATED/TTL_EXPIRED/OPERATOR_OVERRIDE/SUPERSEDED clear predicates, audited-clear obligations, RAW_DELETE_DETECTED reconciliation now sound against unmatched `marker.written`); this BC provides the SS-03 wire-format/field-shape catalog authority only
- BC-1.18.004 — fail-open advisory-only path for Event 8 (`failure_policy=fail-open` INDETERMINATE)
- BC-1.18.013 — events `migration.admission_blocked` / `migration.admission_failed` / `migration.admission_advisory` (Events 11–13, v1.35) triggering-condition/semantics authority (Postcondition 10 — exactly one event per `E-MAINTENANCE-001` verdict / `E-MAINTENANCE-002` verdict / non-verdict anomaly, hygiene rules, the `main.rs`-writes-the-returned-`AdmissionDiagnostic`-data purity split); this BC provides the SS-03 wire-format/field-shape catalog authority only
- BC-1.18.011 — sibling of BC-1.18.013 for the B2 migration; its Postcondition 10 carries the same obligation over the same shared admission core (Events 11–13, v1.35); wire-format/field-shape authority is this BC

## Architecture Anchors

- `crates/factory-dispatcher/src/main.rs` (call sites) + `crates/factory-dispatcher/src/host/emit_event.rs` (emit fns) — async block discard path; timeout termination path; `plugin.abandoned` emission path (drain timer arm, EC-011 break); `plugin.completed` (async path) emission path (drain result arm, mirroring sync `emit_lifecycle` in `crates/factory-dispatcher/src/executor.rs`)
- `crates/factory-dispatcher/src/invoke.rs::invoke_plugin` + `crates/factory-dispatcher/src/host/emit_event.rs` — `plugin.fuel_headroom_warning` emission path: single centralized check after `invoke_plugin`'s internal `match` produces its final `PluginResult`, gated on `PluginResult::Ok` with `fuel_consumed > 0.9 × fuel_cap`; full site detail in BC-1.03.019 §Architecture Anchors
- `crates/factory-dispatcher/src/executor.rs` + `crates/factory-dispatcher/src/host/emit_event.rs` — `plugin.indeterminate` (Event 8) emission path: `classify_outcome()` in `executor.rs` produces `DispatchOutcome::Indeterminate`; `emit_plugin_indeterminate()` in `emit_event.rs` writes the JSON line; same call chain as Event 7 but with additional `artifact_path`, `cause`, and `failure_policy` fields
- `crates/factory-dispatcher/src/indeterminate_marker.rs` + `crates/factory-dispatcher/src/host/mod.rs` (`HostContext::emit_internal`) — `marker.cleared` (Event 9) emission path: `emit_marker_cleared(ctx, clear_mode, marker, trace_id, reason)` (`ctx: &HostContext`, v1.3 signature) in `indeterminate_marker.rs` writes the JSON line via `ctx.emit_internal(ev)` — the dual-sink primitive that durably writes `dispatcher-internal-{date}.jsonl` (via `InternalLog`) AND pushes onto `ctx.events`, NOT a raw `InternalLog::write` call (corrected v1.3, ADR-048 §D4 v1.3 F-P3-001); called from `delete_marker_if_pass` (REVALIDATED clear), from the dispatcher-native `check_and_clear_expired_marker` pre-check (TTL_EXPIRED clear), from the dispatcher-native `reconcile_raw_delete` (OPERATOR_OVERRIDE retroactive — bounded scan of `dispatcher-internal-{date}.jsonl` for an unmatched `marker.written`, not a literal `events-*.jsonl` and not, as of v1.33, an unmatched `plugin.indeterminate`; scan-target corrected v1.3, scan match-type corrected v1.33 ADR-048 §D4 v1.4 F-P6-001), and — v1.3 — from `write_indeterminate_marker`'s caller in `executor.rs` (SUPERSEDED, cross-pair overwrite) — all four call sites are dispatcher-native per ADR-048 §D4 v1.2/v1.3; none is inside the WASM gate plugin. `plugin.indeterminate` (Event 8, `emit_indeterminate` in `executor.rs`) is corrected the same way — `base_ctx.emit_internal(ev)`, not a raw `log.write(&ev)`. **v1.34 correction (ADR-048 §D4 v1.5, F-P9-001):** the SUPERSEDED call site is now gated identically to Event 10's `emit_marker_written` call — both fire only inside `write_indeterminate_marker`'s `Ok(())` arm (SUPERSEDED first, then `marker.written`), never at the pre-overwrite read and never on `Err(_)`.
- `crates/factory-dispatcher/src/indeterminate_marker.rs` (`emit_marker_written`) — `marker.written` (Event 10, v1.33 addition) emission path: `emit_marker_written(ctx, &fields)` (`ctx: &HostContext`), called from `write_indeterminate_marker`'s caller in `executor.rs` immediately after `write_indeterminate_marker` returns `Ok(())` (and after the Event 9 SUPERSEDED check, if applicable), emitting via `ctx.emit_internal(ev)` — the same dual-sink primitive as every sibling dispatcher-native event. NOT called on the `Err(_)` arm (write failure, BC-1.18.001 EC-007). `reconcile_raw_delete`'s bounded scan (above) is the consumer of this event's durable `dispatcher-internal-{date}.jsonl` record.
- `crates/factory-dispatcher/src/registry.rs` — schema_mismatch and registry_invalid emission sites
- `crates/factory-dispatcher/src/internal_log.rs` (`MIGRATION_ADMISSION_BLOCKED`, `MIGRATION_ADMISSION_FAILED`, `MIGRATION_ADMISSION_ADVISORY`) + `crates/factory-dispatcher/src/main.rs` (`run`) + `crates/factory-dispatcher/src/executor.rs` / `crates/factory-dispatcher/src/shard_manager/admission.rs` (`AdmissionDiagnostic` data) — `migration.admission_blocked` / `_failed` / `_advisory` (Events 11–13, v1.35) emission path: the shared admission core (`executor::migration_writer_admission*`) stays free of the log and returns `AdmissionDiagnostic` data (`MigrationAdmission.diagnostics`; `AdmissionOutcome::{Admitted,Blocked}` and the release leg carry the same); `main.rs` writes each via `internal_log.write(&InternalEvent::now(<type>).with_trace_id(..).with_session_id(..).with_field(..))` immediately after the call and BEFORE the early return on a verdict. NOT `HostContext::emit_internal` — no `HostContext` exists at the registry-independent admission position (recorded deviation from the Events 1–10 dual-sink pattern)
- `crates/sink-core/src/` — FileSink fan-out path for the first ten event types (Events 1–10; Events 11–13 are InternalLog-only and do not fan out)
- VP-028 — sink fan-out invariant verification

## Story Anchor

TBD — single story per ADR-019 §6 (no phased rollout, user decision 2026-05-07); S-21.25 anchors Event 7 (`plugin.fuel_headroom_warning`); S-25.09 anchors Events 11–13 (`migration.admission_blocked` / `migration.admission_failed` / `migration.admission_advisory`; v1.36 — implemented via `InternalLog::write` in the `main.rs` shell by S-25.09 AC-002; moved from S-25.08 at the S-25.08/S-25.09 split, D-1252(f)). POL-14: Events 11–13 land with S-25.09, so any draft→active promotion tied to their delivery occurs only after S-25.09 merges (this BC is already `lifecycle_status: active` for Events 1–10).

## VP Anchors

- VP-079 — Payload schema conformance for all ten event types including `plugin.abandoned`, `plugin.completed` (async path), `plugin.fuel_headroom_warning`, `plugin.indeterminate`, `marker.cleared`, and `marker.written`: each mandatory field is
  present, non-null, and the `type` string matches the catalogued value; verified via
  fault-injection integration test per event-type triggering scenario (integration method, bats).
  **Staleness flag CLOSED (F-S2125-P2-003, pass-2 fix burst):** VP-079 v1.20 already registers Event 7 (`plugin.fuel_headroom_warning`) in its Property Statement (seven events), mandatory-fields table, and SITE_7 — the architect-owned amendment this bullet previously demanded was already performed; this flag is retained only as a closure record, not an open action item. (VP-079 v1.20 at closure; now v1.21 — this bullet is a historical closure record and is not re-verified against the current VP-079 version; see v1.27 Changelog, F-S2125-P6-002.) **Staleness flag CLOSED (same-burst 2026-08-30):** VP-079 v1.22 covers Event 8 (`plugin.indeterminate`) in its Property Statement (eight events), mandatory-fields table, and SITE_8 — the architect-owned amendment was performed in the same burst as Event 8's addition; this flag is retained only as a closure record, not an open action item. **Staleness flag OPEN (v1.30 — 2026-08-31):** Event 9 (`marker.cleared`) is added to this BC. VP-079 must be amended to cover Event 9 in its Property Statement (nine events), mandatory-fields table (add `clear_mode`, `actor_type`, `reason` fields), and SITE_9. Architect must propagate to VP-079 + VP-INDEX + verification-architecture.md + verification-coverage-matrix.md under `vp_index_is_vp_catalog_source_of_truth` policy. **Staleness flag OPEN (v1.32 — 2026-09-01, ADR-048 §D4 v1.3 F-P3-002):** Event 9's `clear_mode` enum gains a fourth value `SUPERSEDED` and `actor_type` gains a fourth value `system` (no new event — Event count stays at nine). VP-079's mandatory-fields table and SITE_9 fixture MUST be amended to assert coverage of the `SUPERSEDED`/`system` combination in addition to REVALIDATED/validator, TTL_EXPIRED/deadman, OPERATOR_OVERRIDE/operator. Architect must propagate to VP-079 + VP-INDEX + verification-architecture.md + verification-coverage-matrix.md under `vp_index_is_vp_catalog_source_of_truth` policy. **Staleness flag OPEN (v1.33 — 2026-09-01, ADR-048 §D4 v1.4 F-P6-001):** Event 10 (`marker.written`) is added to this BC (a genuine new event — event count nine→ten). VP-079 must be amended to cover Event 10 in its Property Statement (ten events), mandatory-fields table (add `cause`, `expires_at`; note `marker.written` has no distinct top-level `timestamp` field, relying on the common `ts` field only), and a new SITE_10. VP-108 (BC-1.18.001/BC-1.18.003's emission-correctness VP) also requires its PC3 fixture retargeted from seeding a raw `plugin.indeterminate` line to seeding a `marker.written` line, and gains new PC6 (marker.written emission) and PC7 (negative-control regression test) postconditions — see BC-1.18.001 §PC4 v1.4 and BC-1.18.003 §PC3 v1.6/EC-017. Architect must propagate to VP-079 + VP-108 + VP-INDEX + verification-architecture.md + verification-coverage-matrix.md under `vp_index_is_vp_catalog_source_of_truth` policy. **Staleness flag CLOSED (v1.35 — 2026-10-07, ADR-052 §Decision 5a v1.21 D-1; ADR-052 §Downstream item 33(f)):** Events 11–13 (`migration.admission_blocked` / `_failed` / `_advisory`) were added to this BC (three genuine new events — event count ten→thirteen) and VP-079 v1.25 (current as of v1.36; was v1.24 at v1.35) now carries their wire-format rows (`dispatcher-internal-*.jsonl` only; Property 6 SITES not extended — they are not fan-out events); no new VP, VP counts unchanged. Emission semantics (exactly one per verdict/anomaly, no raw `tool_use_id`/record content) are verified by VP-133 v1.7 facet 7(e) and VP-143 v1.6 (current as of v1.36; were v1.6 / v1.5 at v1.35) (BC-1.18.013 EC-037..EC-039). This flag is retained only as a closure record, not an open action item.
- VP-028 — Sink fan-out invariant: once emitted, all ten event types (Events 1–10; v1.35 scope note, mirrored by VP-028 v1.1: Events 11–13 are not SinkEvents — they are InternalLog-only by design and out of fan-out scope) (`plugin.async_block_discarded`, `dispatcher.schema_mismatch`, `dispatcher.registry_invalid`, `plugin.timeout` (async path), `plugin.abandoned`, `plugin.completed` (async path), `plugin.fuel_headroom_warning`, `plugin.indeterminate`, `marker.cleared`, `marker.written`) reach every configured accepting sink (independent of VP-079's payload conformance check)
- VP-100 — Drain-Timer Expiry Emits Exactly One plugin.abandoned Per In-Flight (plugin_name, entry_index); No plugin.completed Follows for Same Trace (integration; S-19.05; Invariant 6; DI-019)
- VP-TBD (per BC-1.03.019) — fuel-headroom warning triggering-condition/semantics properties (threshold predicate, boundary controls, independence from `on_error`/`failure_policy`); architect to assign a real VP-NNN and propagate to VP-INDEX/verification-architecture.md/verification-coverage-matrix.md per `vp_index_is_vp_catalog_source_of_truth` — this BC's Event 7 entry is the wire-format/field-shape catalog authority only, not the triggering-condition VP owner
- VP-102 — Fuel/Epoch Timeout → INDETERMINATE for fail-closed (BC-1.18.001 PC1); unit-test harness `classify_outcome(PluginResult, FailurePolicy, bool) -> DispatchOutcome`
- VP-103 — Host OutputTooLarge + Ok(exit:0) → INDETERMINATE for fail-closed; per-invocation flag reset (BC-1.18.001 PC1); unit-test
- VP-104 — INDETERMINATE marker write contains all required fields at expected path (BC-1.18.001 PC4); unit-test with tempdir
- VP-105 — Next-advance gate blocks `^Agent$` and `git commit/push` dispatches while marker exists; passes when absent (BC-1.18.002 PC1–PC4); bats integration + Rust unit-test
- VP-106 — Successful re-validation deletes marker; fail-open INDETERMINATE writes no marker; backward-compat guard `test_BC_1_18_004_fail_open_default_preserves_advisory_behavior` preserved (BC-1.18.003 PC1 + BC-1.18.004 PC2); unit-test

## Edge Cases

| ID | Description | Expected Behavior |
|----|-------------|-------------------|
| EC-001 | Async plugin returns exit code 2 (block_intent false by definition — on_error != block) | `plugin.async_block_discarded` emitted with `reason: "async_plugin_block_verdict_discarded"`; dispatcher exit code unchanged |
| EC-002 | Registry with schema_version = 1 loaded | `dispatcher.schema_mismatch` emitted with `found_version: 1`, `expected_version: 2`, `error_code: "E-REG-001"` |
| EC-003 | Registry with schema_version = null (malformed TOML) | `dispatcher.schema_mismatch` emitted with `found_version: null`; dispatcher hard-errors |
| EC-004a | Registry entry has on_error=block AND async=true (AsyncBlockConflict) | `dispatcher.registry_invalid` emitted with `error_code: "E-REG-002"`, `violation: "async_block_conflict"`, `offending_plugin` named; dispatcher refuses to start |
| EC-004b | Two or more registry entries share the same `(name, event, tool)` tuple (DuplicateEntry) | `dispatcher.registry_invalid` emitted with `error_code: "E-REG-003"`, `violation: "duplicate_hook_registration"`, `offending_plugin`/`offending_event`/`offending_tool` set to the duplicating entry's tuple; dispatcher refuses to start |
| EC-005 | Async plugin times out | `plugin.timeout` emitted with `execution_group: "async"`; plugin process terminated; dispatcher exit code unaffected |
| EC-006 | Multiple async plugins time out in same invocation | One `plugin.timeout` event per timed-out plugin (not a single batch event) |
| EC-007 | Drain timer fires with N async plugins still in-flight | N `plugin.abandoned` events emitted (one per in-flight plugin), each with `drain_window_ms` set to the effective drain window value and `entry_index` identifying the registry partition position; no `plugin.completed` events follow for the abandoned plugins in this invocation (Invariant 6) |
| EC-008 | Async plugin completes within drain window with exit code 0 (non-block) | `plugin.completed` (async path) emitted with all mandatory fields present including `entry_index`; no `plugin.abandoned` follows for the same `(trace_id, plugin_name, entry_index)` tuple; dispatcher exit code unchanged (Invariant 3 and Invariant 6) |
| EC-009 | `invoke_plugin` completes `PluginResult::Ok` with `fuel_consumed > 0.9 × fuel_cap` | `plugin.fuel_headroom_warning` emitted with `plugin_name`, `fuel_consumed`, `fuel_cap`, `headroom_ratio`, `level="warn"`, verbatim `message`, `timestamp`; dispatcher exit code and block decision unaffected (Invariant 3; BC-1.03.019 PC9). Full boundary/negative-control edge cases (exact-90%, non-`Ok` outcomes, etc.) are BC-1.03.019's EC-001–EC-007. |
| EC-010 | `invoke_plugin` produces INDETERMINATE (any cause) | `plugin.indeterminate` emitted with `plugin_name`, `artifact_path`, `cause` (one of: fuel\|epoch\|output-too-large), `failure_policy` (fail-closed or fail-open), `trace_id`, `session_id`, `timestamp`; dispatcher exit code unaffected by the event itself. For `failure_policy=fail-closed`, marker also written and gate armed (BC-1.18.001/BC-1.18.002). For `failure_policy=fail-open`, only the event (BC-1.18.004). Full behavioral governance: BC-1.18.001–004. |
| EC-011 | `delete_marker_if_pass` removes marker on PASS (clear_mode=REVALIDATED) | `marker.cleared` emitted with `clear_mode="REVALIDATED"`, `actor_type="validator"`, `trace_id` from marker TOML, `plugin_name` from marker TOML, `artifact_path` from marker TOML, `reason=null`, `timestamp` = time of clear. Emitted after `remove_file` succeeds. Full behavioral governance: BC-1.18.003 PC1. |
| EC-012 | Dispatcher-native `check_and_clear_expired_marker` pre-check determines marker `expires_at ≤ now` (TTL deadman; ADR-048 §D4 v1.2) | `marker.cleared` emitted with `clear_mode="TTL_EXPIRED"`, `actor_type="deadman"`, `trace_id` from marker TOML, `plugin_name` from marker TOML, `artifact_path` from marker TOML, `reason=null`, `timestamp` = time of auto-delete. Crash-path native TTL-allow does NOT emit this event (BC-1.18.003 PC4). Full behavioral governance: BC-1.18.003 PC4. |
| EC-013 | Dispatcher-native `reconcile_raw_delete`'s `RAW_DELETE_DETECTED` reconciliation path fires (marker absent, unmatched `marker.written` in `dispatcher-internal-{date}.jsonl`; ADR-048 §D4 v1.2/v1.3/v1.4 — scan match-type corrected v1.33, previously described as unmatched `plugin.indeterminate`) | `marker.cleared` emitted retroactively with `clear_mode="OPERATOR_OVERRIDE"`, `actor_type="operator"`, `trace_id`/`plugin_name`/`artifact_path` from the matched `marker.written` event, `reason` field MUST be non-null (human-readable explanation), `timestamp` = reconciliation time. Best-effort: only possible when `dispatcher-internal-{date}.jsonl` is available (`ctx.internal_log` is `Some`). Full behavioral governance: BC-1.18.003 PC3. |
| EC-014 | `write_indeterminate_marker`'s caller (`executor.rs`) overwrites a marker for pair A `(plugin_a, artifact_a)` with a NEW INDETERMINATE event for a DIFFERENT pair B `(plugin_b, artifact_b)` (BC-1.18.001 INV3 last-writer-wins; ADR-048 §D4 v1.3 F-P3-002) AND `write_indeterminate_marker(&pair_b, ...)` returns `Ok(())` | `marker.cleared` emitted for A ONLY AFTER B's `write_indeterminate_marker` call returns `Ok(())` (**corrected v1.34, ADR-048 §D4 v1.5, F-P9-001** — previously described as emitted BEFORE B's write completes), in the same write-success arm as B's `marker.written` (Event 10; SUPERSEDED fires first, then `marker.written`), with `clear_mode="SUPERSEDED"`, `actor_type="system"`, `trace_id`/`plugin_name`/`artifact_path` = A's OWN fields (not B's), `reason` MUST be non-null (`"SUPERSEDED: marker overwritten by a new plugin.indeterminate event for a different (plugin_name, artifact_path) pair before being cleared; last-writer-wins (BC-1.18.001 INV3)"`), `timestamp` = time of the overwrite. Same-pair re-INDETERMINATE (A overwritten by a new event for the SAME pair) emits nothing. See EC-016 for the `Err(_)` negative control. Full behavioral governance: BC-1.18.003 PC5. |
| EC-015 | `write_indeterminate_marker` returns `Ok(())` (v1.33, ADR-048 §D4 v1.4 — F-P6-001) | `marker.written` (Event 10) emitted with `trace_id`/`plugin_name`/`artifact_path`/`cause`/`expires_at` = the marker's own values (the values just written to the TOML file); `ts` = time of the write. Full behavioral governance: BC-1.18.001 PC4 v1.4. |
| EC-016 | `write_indeterminate_marker` returns `Err(_)` — atomic rename fails (v1.33, ADR-048 §D4 v1.4 — F-P6-001; sibling of BC-1.18.001 EC-007) | NO `marker.written` emitted — this is load-bearing, not an oversight: a write failure producing the same positive creation record a write success produces would make `reconcile_raw_delete`'s OPERATOR_OVERRIDE inference (EC-013) unsound again for the write-failure case. `plugin.indeterminate` (Event 8) is still emitted per BC-1.18.001 EC-007. **v1.34 addition (ADR-048 §D4 v1.5 — F-P9-001):** when the `Err(_)` occurs during a cross-pair overwrite (EC-014's scenario — an existing marker for a DIFFERENT pair A was about to be superseded), `marker.cleared(SUPERSEDED)` for A is likewise NOT emitted — NEITHER write-tied event fires on failure, symmetric by construction. Full behavioral governance: BC-1.18.001 PC4 v1.4 + BC-1.18.003 PC5. |
| EC-017 | The writer-admission gate returns an `E-MAINTENANCE-001` block verdict for a protected-path write, in each of the five branches (gate-only, live txn, live coordinator, foreign migration, Branch C verification failure) (v1.35, ADR-052 §5a "Admission diagnostics channel"; BC-1.18.013 Postcondition 10(a)) | Exactly ONE `migration.admission_blocked` (Event 11) is appended to `dispatcher-internal-{date}.jsonl` per blocked dispatch, written by `main.rs` before the early return, with `scope`/`family`/`branch`/`gate_state`/`migration_id`/`txn_id`/`check`/`reconciliation` populated per its wire format (nullable fields present as `null` when not applicable); `migration_id`/`txn_id`/`check` sanitized (control characters escaped, ≤ 64 chars); no `plugin_name`; no raw `tool_use_id` or record content in any field; the dispatcher exit code (2) and stderr message are those of the verdict, unchanged. NO event for an admitted or out-of-scope write |
| EC-018 | The writer-admission check returns an `E-MAINTENANCE-002` error verdict (causes `invalid_tool_use_id`, `io`, `state_integrity`) (v1.35; BC-1.18.013 Postcondition 10(b)) | Exactly ONE `migration.admission_failed` (Event 12) per failed dispatch with `cause` = the cause token, `kind` = the `AdmissionStateIntegrityKind` token (one of the SIX closed-domain tokens, v1.37) when `cause=state_integrity` else `null`, `detail` = path / `io::Error` kind+message / byte length (never the raw `tool_use_id`, never record content); the single-line operator message carries only the cause token, unchanged. **Cap-surface vector (v1.38; ADR-052 v1.25 item 11(g)):** a `state_integrity` / `io` fact whose subject is a ~100-character path ⇒ the event `detail` carries that path truncated to 64 characters (per data-derived substring), while the coordinator's operator stderr line / `BcIndexMigrationError` Display renders the SAME path untruncated (100 < 256; BC-1.18.013 Postcondition 10); a 108-character macOS path is cut at 64 in the event and intact on stderr |
| EC-019 | A non-verdict admission anomaly occurs (non-ENOENT release error, Branch A re-open, Branch B discard, Branch C finalize undelivered, S-25.06 successful finalize) (v1.35; BC-1.18.013 Postcondition 10(c)); and, separately, a reservation-timestamp fallback at drain GC | Exactly ONE `migration.admission_advisory` (Event 13) per anomaly with `reason` ∈ {`reservation_release_failed`, `branch_a_gate_reopened`, `branch_b_txn_aborted`, `branch_c_finalize_unwired`, `branch_c_finalized`} (closed five-token domain; `branch_c_finalize_unwired` is IN ADDITION to the one Event 11, `branch_c_finalized` is admitted with no Event 11 and neither S-25.08 nor S-25.09 emits it); optional fields only from {`migration_id`, `txn_id`, `check`, `detail`, `tool_use_id_len`}; the dispatch outcome of the release/Branch A/Branch B cases is unchanged; no raw `tool_use_id`. The timestamp fallback writes NO Event 13 (coordinator-drain stderr token only) |

## Canonical Test Vectors

| Input | Expected Output | Category |
|-------|----------------|----------|
| Async plugin exits 2 | `plugin.async_block_discarded` event in events-*.jsonl; dispatcher exit 0 | async-block-discard |
| Registry schema_version=1 loaded | `dispatcher.schema_mismatch` event in events-*.jsonl; dispatcher exits non-zero | schema-mismatch |
| Registry entry on_error=block + async=true | `dispatcher.registry_invalid` event in events-*.jsonl; `error_code: "E-REG-002"`, `violation: "async_block_conflict"`; dispatcher refuses to start | registry-invalid-E-REG-002 |
| Registry with duplicate hook name entries | `dispatcher.registry_invalid` event in events-*.jsonl; `error_code: "E-REG-003"`, `violation: "duplicate_hook_registration"`; dispatcher refuses to start | registry-invalid-E-REG-003 |
| Async plugin times out (timeout_ms exceeded) | `plugin.timeout` with `execution_group: "async"` in events-*.jsonl; no impact on dispatcher exit | async-timeout |
| All four original events emitted; FileSink running | All four appear as JSON lines in events-YYYY-MM-DD.jsonl; `trace_id` present on all | fan-out-happy-path |
| Async plugin exits 0 within drain window (non-block) | `plugin.completed` event in events-*.jsonl; `type = "plugin.completed"`; `entry_index` present and correct; `plugin_version` present; no `plugin.abandoned` for same `(trace_id, plugin_name, entry_index)` tuple; dispatcher exit code unchanged | async-completed |
| All async plugins complete before drain timer | No `plugin.abandoned` events in events-*.jsonl | abandoned-none |
| One async plugin still in-flight at drain timer expiry | `plugin.abandoned` event with `drain_window_ms` set, `plugin_name` correct, `entry_index` correct, `trace_id` + `session_id` present; no `plugin.completed` follows for that plugin (Invariant 6) | abandoned-one |
| `PluginResult::Ok`, `fuel_consumed=18_500_000`, `fuel_cap=20_000_000` (92.5%) | `plugin.fuel_headroom_warning` event in events-*.jsonl; `headroom_ratio=0.075`; `level="warn"`; `message="fuel-headroom-warning: plugin consumed >90% of budget; next larger input may trap — recalibrate fuel_cap"`; dispatcher exit code and block decision unaffected | fuel-headroom-warning |
| fail-closed plugin fuel-exhausts (`Trap::OutOfFuel`) | `plugin.indeterminate` event in events-*.jsonl; `cause="fuel"`, `failure_policy="fail-closed"`, all mandatory fields present; `.factory/unvalidated-mutation.marker` written; subsequent `^Agent$` and `git commit/push` dispatches blocked (BC-1.18.002) | indeterminate-fail-closed-fuel |
| fail-open plugin epoch-times-out | `plugin.indeterminate` event in events-*.jsonl; `cause="epoch"`, `failure_policy="fail-open"`, all mandatory fields present; no marker written; no gate triggered | indeterminate-fail-open-epoch |
| Marker deleted via T1 re-validation (Edit/Write → plugin PASS → `delete_marker_if_pass`) | `marker.cleared` event in `dispatcher-internal-{date}.jsonl` (via `ctx.emit_internal`); `clear_mode="REVALIDATED"`, `actor_type="validator"`, `trace_id` matches originating `plugin.indeterminate` `trace_id`, `reason=null`; all mandatory fields present | marker-cleared-revalidated |
| Marker TTL expires; dispatcher-native `check_and_clear_expired_marker` pre-check auto-deletes marker (before the Arm 1/Arm 2 WASM gate plugin runs) | `marker.cleared` event in `dispatcher-internal-{date}.jsonl` (via `ctx.emit_internal`); `clear_mode="TTL_EXPIRED"`, `actor_type="deadman"`, `trace_id` from marker TOML, `reason=null`; all mandatory fields present; no event if native crash-path TTL-allow fires (no marker delete on crash path) | marker-cleared-ttl-expired |
| Operator deletes marker out-of-band; dispatcher-native `reconcile_raw_delete` detects RAW_DELETE (marker absent + unmatched `marker.written` in `dispatcher-internal-{date}.jsonl`; v1.33 — previously described as unmatched `plugin.indeterminate`) | `marker.cleared` event in `dispatcher-internal-{date}.jsonl` (via `ctx.emit_internal`); `clear_mode="OPERATOR_OVERRIDE"`, `actor_type="operator"`, `reason` non-null; emitted retroactively at reconciliation time | marker-cleared-operator-override |
| Marker for pair A `(plugin_a, artifact_a)` overwritten by a NEW INDETERMINATE event for a DIFFERENT pair B `(plugin_b, artifact_b)`; `write_indeterminate_marker(&pair_b, ...)` returns `Ok(())` (v1.3; ordering corrected v1.34) | `marker.cleared` event in `dispatcher-internal-{date}.jsonl` (via `ctx.emit_internal`), emitted for A ONLY AFTER B's marker write returns `Ok(())` (**corrected v1.34, ADR-048 §D4 v1.5, F-P9-001** — previously described as before B's write completes), in the same write-success arm as B's `marker.written`; `clear_mode="SUPERSEDED"`, `actor_type="system"`, `trace_id`/`plugin_name`/`artifact_path` = A's own fields, `reason` non-null; all mandatory fields present | marker-cleared-superseded |
| Marker for pair A `(plugin_a, artifact_a)` overwritten by a NEW INDETERMINATE event for a DIFFERENT pair B `(plugin_b, artifact_b)`; `write_indeterminate_marker(&pair_b, ...)` returns `Err(_)` (v1.34, ADR-048 §D4 v1.5 — F-P9-001 negative control) | NO `marker.cleared(SUPERSEDED)` emitted for A and NO `marker.written` emitted for B — neither write-tied event fabricated on write failure | marker-cleared-superseded-failure-no-emit |
| `write_indeterminate_marker` returns `Ok(())` (v1.33) | `marker.written` event in `dispatcher-internal-{date}.jsonl` (via `ctx.emit_internal`); `trace_id`/`plugin_name`/`artifact_path`/`cause`/`expires_at` = the marker's own values; all mandatory fields present | marker-written-success |
| `write_indeterminate_marker` returns `Err(_)` — atomic rename fails (v1.33) | NO `marker.written` event emitted; `plugin.indeterminate` still emitted (BC-1.18.001 EC-007) | marker-written-failure-no-emit |
| PreToolUse fail-closed INDETERMINATE — no marker write ever attempted (v1.33, F-P6-001 negative control) | `plugin.indeterminate` emitted with NO `marker.written`; subsequent `reconcile_raw_delete` finds no unmatched `marker.written` for that pair and does NOT emit `marker.cleared(OPERATOR_OVERRIDE)` | marker-written-negative-control-pretooluse |
| Real dispatcher PreToolUse `Write` under `.factory/cycles/c1/a.md` (`tool_use_id=toolu_ABC123`) with a live txn (`migration_id="backfill-append-logs"`, STAGING gen-1, terminal record absent); then read `dispatcher-internal-{date}.jsonl` (v1.35) | Exit 2 `E-MAINTENANCE-001`; exactly ONE line with `type="migration.admission_blocked"`, `scope=".factory/cycles/"`, `family="cycles"`, `branch="live_txn"`, `gate_state` present, `migration_id="backfill-append-logs"`, `txn_id` non-null, `check=null`, `reconciliation="nothing_to_reconcile"`, and `trace_id`/`session_id`/`ts`/`ts_epoch`/`schema_version` present; no `plugin_name`; the string `toolu_ABC123` appears in no field of the line | migration-admission-blocked |
| Real dispatcher PreToolUse `Write` to a protected path with `tool_use_id=12345` (a JSON number); and separately with `gate-state.json` containing `"BOGUS"` (v1.35) | Exit 2 `E-MAINTENANCE-002`; exactly ONE `migration.admission_failed` line each: (`cause="invalid_tool_use_id"`, `kind=null`, `detail` = byte length only) and (`cause="state_integrity"`, `kind="gate_record_malformed"`, `detail` = path + parse message, no record content) | migration-admission-failed |
| Real dispatcher PreToolUse `Write` to a protected path (v1.37) with a live known STAGING txn `{"state":"STAGING","migration_id":"migrate-bc-index","generation_id":null,"schema_version":2}` (and separately `"schema_version":4294967296`), gate DRAINING, `exclusive.lock` free, valid `tool_use_id` | Exit 2 `E-MAINTENANCE-002` (`state_integrity`); exactly ONE `migration.admission_failed` line each: (`cause="state_integrity"`, `kind="txn_record_newer_schema"`, `detail` = path + sanitized message naming the newer schema_version, no record content); NO `migration.admission_blocked`; the txn bytes are unchanged (not discarded); the line parses and `kind` is a member of the closed SIX-token domain `gate_record_malformed \| txn_record_malformed \| txn_migration_id_not_string \| multiple_live_txns \| reservation_serialization \| txn_record_newer_schema` | migration-admission-failed |
| Closed-domain assertion over every `migration.admission_failed` line produced by the Event 12 fixtures (v1.37) | `kind` is `null` when `cause != state_integrity`, and otherwise exactly one of the SIX tokens above; a seventh token or a token spelled differently fails the test | migration-admission-failed |
| Real dispatcher PostToolUse release of an admitted reservation whose unlink fails with a non-ENOENT error (read-only `reservations/` directory); and a dispatch that performs a Branch A repair (gate `LOCKED`, no live txn) (v1.35) | Exactly ONE `migration.admission_advisory` line each (`reason="reservation_release_failed"`; `reason="branch_a_gate_reopened"`); dispatch outcome unchanged (exit 0 / admitted); no raw `tool_use_id` | migration-admission-advisory |
| Real dispatcher PreToolUse `Write` to a protected path whose txn record (or `.factory` directory) lives under a ~100-character project path and is malformed / unreadable (`state_integrity`, `io`) (v1.38, ADR-052 v1.25 item 11(g)); and, separately, the coordinator `migrate-bc-index` run over the same on-disk state | Dispatcher: exactly ONE `migration.admission_failed` whose `detail` carries the path truncated to 64 characters (escaped, per data-derived substring) and the single-line operator message carries the cause token only. Coordinator: the stderr failure line / `BcIndexMigrationError` Display carries the SAME path UNTRUNCATED (< 256), so the operator learns which file is malformed; the event is never the source of the stderr text (two renderings of one raw slot) | migration-admission-failed-cap-surface |
| COMMITTING txn with a verifying own terminal record while the finalize effect is undelivered (S-25.09 seam), protected `Write`; and (S-25.06 only) the same fixture once the finalize is delivered (v1.35 follow-up, item 33(c)) | S-25.09: exit 2 with exactly ONE `migration.admission_blocked` (`reconciliation="completion_record_mismatch"`, `branch="completion_record_mismatch"`, `check="finalize_unwired"`) AND exactly ONE `migration.admission_advisory` with `reason="branch_c_finalize_unwired"`; S-25.06: the write is admitted, exactly ONE `migration.admission_advisory` with `reason="branch_c_finalized"` and NO `migration.admission_blocked`; `reason` is always one of the five-token domain, optional fields only from {`migration_id`, `txn_id`, `check`, `detail`, `tool_use_id_len`} | migration-admission-advisory-branch-c |
| Drain GC over a reservation with unparseable `created_at` (v1.35 follow-up) | NO `migration.admission_*` line (the five timestamp tokens are coordinator-drain stderr tokens only) | migration-admission-timestamp-stderr-only |
| One real-dispatcher/unit vector per `reconciliation` token (v1.35 follow-up, item 33(b)): `live_coordinator` (lock held); `nothing_to_reconcile` (live txn own record absent; and gate-only DRAINING); `gate_reopened` (Branch A then blocked); `null_generation_txn_aborted` (Branch B then blocked); `foreign_migration_refused` (`migration_id ∉ K`); `completion_record_mismatch` (STAGING + terminal record) | `migration.admission_blocked.reconciliation` equals the token; `branch` derived: `live_coordinator`; `live_txn` / `gate_only`; `gate_only` if no live txn remains else `live_txn` (same for Branch B); `foreign_migration`; `completion_record_mismatch` (with `check` non-null); `none` never appears | migration-admission-reconciliation-tokens |
| Admitted protected-path write with no anomaly; and a write to `.factory/STATE.md` or another project's `.factory/cycles/…` (v1.35) | NO `migration.admission_*` line written for either | migration-admission-no-event |

## Verification Properties

| VP-NNN | Property | Proof Method |
|--------|----------|-------------|
| VP-028 | Sink fan-out invariant — all events reach all configured sinks | integration |
| VP-079 | Payload schema conformance for all ten event types including `plugin.abandoned`, `plugin.completed` (async path), `plugin.fuel_headroom_warning`, `plugin.indeterminate`, `marker.cleared`, and `marker.written` — mandatory fields present, non-null, type string correct. **VP-079 Event 9 staleness flag OPEN:** architect must add SITE_9 (`marker.cleared`), update mandatory-fields table (add `clear_mode`, `actor_type`, `reason`), update Property Statement to "nine events" per `vp_index_is_vp_catalog_source_of_truth`. **v1.32 staleness flag OPEN (ADR-048 §D4 v1.3):** `clear_mode` gains `SUPERSEDED`, `actor_type` gains `system` — SITE_9 and mandatory-fields table must assert coverage of this fourth combination; event count stays at nine at that point. **v1.33 staleness flag OPEN (ADR-048 §D4 v1.4 — F-P6-001):** new Event 10 `marker.written` — architect must add SITE_10, update mandatory-fields table (add `cause`, `expires_at`), update Property Statement to "ten events" per `vp_index_is_vp_catalog_source_of_truth`. | integration |
| VP-108 | (v1.33, ADR-048 §D4 v1.4 — F-P6-001) `write_indeterminate_marker` returning `Ok(())` emits `marker.written` (Event 10) via `emit_marker_written`, carrying the marker's own trace_id/plugin_name/artifact_path/cause/expires_at; returning `Err(_)` emits zero `marker.written` events (BC-1.18.001 PC4 v1.4) | unit-test |
| VP-108 | (v1.33, ADR-048 §D4 v1.4 — F-P6-001 regression test) negative control: a fixture with an unmatched `plugin.indeterminate` but no `marker.written` record (simulating a PreToolUse fail-closed INDETERMINATE or a PostToolUse write failure) → `reconcile_raw_delete` emits zero `marker.cleared(OPERATOR_OVERRIDE)` events for that pair (BC-1.18.003 PC3/EC-017) | unit-test |
| VP-108 | (v1.34, ADR-048 §D4 v1.5 — F-P9-001 regression test) negative control: existing marker for pair A, `write_indeterminate_marker(&pair_b, ...)` FORCED to return `Err(_)` during a cross-pair overwrite → zero `marker.cleared(SUPERSEDED)` events for A AND zero `marker.written` events for B are emitted (BC-1.18.001 PC4/INV3; BC-1.18.003 PC5/EC-018) | unit-test |
| VP-100 | Drain-timer expiry emits exactly one plugin.abandoned per in-flight (plugin_name, entry_index); no plugin.completed follows for same trace_id+plugin_name+entry_index (Invariant 6; DI-019) | integration (S-19.05) |
| VP-102 | Fuel/epoch timeout produces INDETERMINATE for fail-closed plugin; `classify_outcome(Timeout{Fuel}, FailClosed, _) == Indeterminate` (BC-1.18.001 PC1) | unit-test |
| VP-103 | OTL flag + exit_code=0 produces INDETERMINATE for fail-closed; per-invocation reset of `host_output_too_large_seen` (BC-1.18.001 PC1) | unit-test |
| VP-104 | INDETERMINATE marker write contains all 5 required fields (plugin_name, artifact_path, cause, trace_id, timestamp) at expected path (BC-1.18.001 PC4) | unit-test |
| VP-105 | Next-advance gate blocks `^Agent$` and `git commit/push` dispatches while marker exists; passes when marker absent (BC-1.18.002 PC1–PC4) | integration + unit-test |
| VP-106 | Successful re-validation (PASS) deletes marker; fail-open INDETERMINATE writes no marker; backward-compat guard test preserved (BC-1.18.003 PC1 + BC-1.18.004 PC2) | unit-test |

## Traceability

| Field | Value |
|-------|-------|
| L2 Capability | CAP-003 ("Stream observability events to multiple configurable sinks") per capabilities.md §CAP-003 |
| Capability Anchor Justification | CAP-003 ("Stream observability events to multiple configurable sinks") per capabilities.md §CAP-003 — these seven event types are observability events that operators and the VSDD engine consume to diagnose async plugin behavior and dispatcher resource-budget health; cataloguing them here fulfills the "stream observability events" promise by defining the wire format and sink-fan-out obligation |
| L2 Domain Invariants | DI-017 — `trace_id` present on every emitted event; all thirteen event types must carry `trace_id`; Invariant 5 of this BC enforces DI-017's requirement that `trace_id` be the canonical wire-field name (not `dispatcher_trace_id`); DI-019 — `ASYNC_DRAIN_WINDOW_MS` (the `plugin.timeout` async path and `plugin.async_block_discarded` events are emitted by tasks running within the drain window bounded by DI-019; VP-079 fixture timing for these events must account for the DI-019 drain window value) |
| Architecture Module | SS-03 — `crates/sink-core/` (event routing); SS-01 — `crates/factory-dispatcher/src/main.rs` + `crates/factory-dispatcher/src/host/emit_event.rs` (emission sites); SS-01 — `crates/factory-dispatcher/src/registry.rs` (schema_mismatch + registry_invalid emission sites); SS-01 — `crates/factory-dispatcher/src/invoke.rs` (`plugin.fuel_headroom_warning` emission site, Event 7); SS-01 — `crates/factory-dispatcher/src/executor.rs` + `crates/factory-dispatcher/src/indeterminate_marker.rs` (`plugin.indeterminate` emission site and marker path, Event 8); SS-01 — `crates/factory-dispatcher/src/indeterminate_marker.rs` `emit_marker_cleared` (REVALIDATED/TTL_EXPIRED/OPERATOR_OVERRIDE/SUPERSEDED, all dispatcher-native via `HostContext::emit_internal` per ADR-048 §D4 v1.2/v1.3) + `crates/factory-dispatcher/src/host/mod.rs` (`HostContext::emit_internal`/`InternalLog` — the durable `dispatcher-internal-{date}.jsonl` sink) (`marker.cleared` emission sites, Event 9; corrected v1.32 — the pre-v1.2 attribution of TTL_EXPIRED/OPERATOR_OVERRIDE emission to the SS-04 `validate-unvalidated-mutation-marker` plugin crate was superseded by the v1.2 Emission-Point Correction: `evaluate_gate` is a pure marker-presence check with no emission logic of its own); SS-01 — `crates/factory-dispatcher/src/indeterminate_marker.rs` `emit_marker_written` (v1.33 addition, ADR-048 §D4 v1.4 — `marker.written`, Event 10, called from `write_indeterminate_marker`'s caller in `executor.rs` on the `Ok(())` arm only). Note: SS-07 owns `plugins/vsdd-factory/hooks-registry.toml` (the file format) but the emission sites in registry.rs are SS-01 Rust modules per ARCH-INDEX. |
| ADR | ADR-019 — Async Semantics at Registry Layer; introduces the original four ADR-019 async-semantics events (Events 1–4); Events 5 (`plugin.abandoned`) and 6 (`plugin.completed` async path) were added by E-19 fix bursts F-P1-013 and F-P5-003 respectively; Event 7 (`plugin.fuel_headroom_warning`) is added by ADR-039 §Decision 5 Mitigation 1 (E-006) (S-21.25 adversarial pass-1 fix burst, F-S2125-P1-003); Event 8 (`plugin.indeterminate`) is added by ADR-047 §Decision 1 (INDETERMINATE outcome class) + ADR-047 §Decision 2 (`failure_policy` field reuse routing) (F2 validation-integrity-layer1 spec burst, 2026-08-30); Event 9 (`marker.cleared`) is added by ADR-048 §Decision 4 (audited clear event + TTL-loudness — REVALIDATED/TTL_EXPIRED/OPERATOR_OVERRIDE clear modes, RAW_DELETE_DETECTED reconciliation, proportionate design: append-only durable log, no signed digests/dual-control per cooperating-agent threat model) (product-owner spec burst, 2026-08-31); ADR-048 §Decision 4 v1.2 Emission-Point Correction (S-25.01 LOCAL adversary pass 2 F-P2-002 HIGH + F-P2-003 MED; human-ratified) moves TTL_EXPIRED/OPERATOR_OVERRIDE emission entirely dispatcher-native; **ADR-048 §Decision 4 v1.3 Emission-Mechanism Precision Correction + SUPERSEDED Clear Mode** (S-25.01 LOCAL adversary pass 3 F-P3-001 MEDIUM + F-P3-002 LOW; human-ratified) — Events 8/9 emission corrected from a raw `InternalLog::write` to `HostContext::emit_internal` (the dual-sink helper every sibling event uses); `reconcile_raw_delete`'s scan target corrected to `dispatcher-internal-{date}.jsonl` (via `InternalLog`), not a literal `events-{date}.jsonl`; new `clear_mode = "SUPERSEDED"` / `actor_type = "system"` added to close a false-`OPERATOR_OVERRIDE`-attribution gap on cross-pair marker overwrite (BC-1.18.001 INV3); **ADR-048 §Decision 4 v1.4 Reconciliation-Premise Correction** (S-25.01 adversary pass 6 F-P6-001 MEDIUM; architect adjudication; HUMAN-RATIFIED 2026-09-01, POLICY 22, D-1141) — new Event 10 `marker.written` added to the SS-03 event catalog: emitted via `HostContext::emit_internal` by `write_indeterminate_marker`'s caller ONLY after the atomic write returns `Ok(())`, never before, never on `Err(_)` (BC-1.18.001 §PC4 v1.4); Event 9's OPERATOR_OVERRIDE reconciliation (`reconcile_raw_delete`) is retargeted from scanning for an unmatched `plugin.indeterminate` (unsound — fires for every INDETERMINATE outcome regardless of whether a marker was ever written) to scanning for an unmatched `marker.written` (sound by construction — exists iff a marker was actually written); closes the false-`OPERATOR_OVERRIDE`-attribution gap reachable via a PreToolUse fail-closed INDETERMINATE (BC-1.18.001 INV4) or a PostToolUse marker-write I/O failure (BC-1.18.001 EC-007); **ADR-048 §Decision 4 v1.5 Emission-Point Correction** (S-25.01 adversary pass 9 F-P9-001 MEDIUM; architect adjudication; HUMAN-RATIFIED 2026-09-01, POLICY 22, D-1142) — Event 9's SUPERSEDED emission (`emit_superseded_if_cross_pair`) is relocated from the unconditional pre-overwrite read to inside `write_indeterminate_marker`'s `Ok(())` arm, alongside the (unchanged) Event 10 `marker.written` call — SUPERSEDED fires first, then `marker.written`; on `Err(_)`, NEITHER event is emitted; symmetric with the v1.4 `marker.written`-only-after-success rule, closing the identical fabricated-audit-record class (NIST AU-3/AU-10) for the SUPERSEDED clear_mode that v1.4 closed for the OPERATOR_OVERRIDE reconciliation premise; **ADR-052 §Decision 5a v1.21 "Admission diagnostics channel"** (S-25.08 local adversary pass 2 D-1; ADR-052 v1.21 §Downstream item 25) — Events 11–13 (`migration.admission_blocked` / `_failed` / `_advisory`) added: the dispatcher installs no `tracing` subscriber, so the governed-migration admission/release/reconciliation diagnostics are `InternalEvent`s written by `main.rs` through `InternalLog::write` directly (not `HostContext::emit_internal` — no `HostContext` at the registry-independent admission position; recorded deviation), durable in `dispatcher-internal-{date}.jsonl`, exactly one per verdict/anomaly, no raw `tool_use_id` or record content |
| Stories | S-15.01 (single story per ADR-019 §6); S-19.05 (Event 6 — async plugin.completed telemetry); S-21.25 (Event 7 — fuel-headroom warning telemetry); S-25.01 (Events 8 + 9 + 10 — INDETERMINATE outcome + marker + gate + audited clear + audited creation, validation-integrity Layer 1); S-25.09 (Events 11 + 12 + 13 — `migration.admission_blocked` / `_failed` / `_advisory` admission diagnostics via `InternalLog`, ADR-052 v1.21 scope; moved from S-25.08 at the S-25.08/S-25.09 split) |
| Cycle | v1.0-feature-plugin-async-semantics-pass-1 (F2); v1.0-brownfield-backfill (Event 7 addition, S-21.25 adversarial pass-1 fix burst) |

### Source Evidence

| Property | Value |
|----------|-------|
| **Path** | Adversary pass-1 F-P1-008: BC-1.14.001 + BC-7.06.001 introduce ≥3 new event-type strings without SS-03 catalog amendment; sink consumers may silently drop these events; VP-028 sink-fan-out invariant bypassed |
| **Confidence** | HIGH — all four event types are referenced in F2 BCs but were missing SS-03 catalog authority |
| **Extraction Date** | 2026-05-07 |

#### Purity Classification

| Property | Assessment |
|----------|-----------|
| **I/O operations** | Event emission: writes to events-*.jsonl via FileSink (filesystem I/O). |
| **Global state access** | FileSink holds shared write state (file handle). |
| **Deterministic** | Event content is deterministic given same inputs; file timestamps vary. |
| **Thread safety** | FileSink is designed for concurrent writes (per BC-3.x contracts). |
| **Overall classification** | Effectful (filesystem I/O); emission is fire-and-once (no retry). |

## Amendment 2026-07-06 (v1.16 → v1.17 — F-P3-013: `entry_index` semantics paragraph corrected; derivation rule stated explicitly; verify-factory-lock example replaced)

**Driver:** Adversary finding F-P3-013 (E-19 pass-3) — the `entry_index` semantics paragraph in Event 5 (`plugin.abandoned`) claimed that "the registry idiom permits multiple entries per `plugin_name`" and cited `verify-factory-lock` as an example (one entry for `Edit|Write|MultiEdit|Agent`, one for `Bash`). Ground truth: those two production registry entries carry DIFFERENT `name` field values (`verify-factory-lock` and `verify-factory-lock-bash`), so they produce distinct `plugin_name` values in any `plugin.abandoned` events and do NOT constitute a name-duplication example. The paragraph also failed to state the derivation rule for `plugin_name` explicitly.

**Changes made:**

1. **Event 5 `entry_index` semantics paragraph** (F-P3-013): Replaced the erroneous `verify-factory-lock` duplication example with: (a) explicit derivation rule — `plugin_name` in `plugin.abandoned` events is the registry entry `name` field verbatim; (b) the registry schema does NOT enforce `name` uniqueness across entries, making name-only keying potentially ambiguous for future registries; (c) a parenthetical note that the production `verify-factory-lock` / `verify-factory-lock-bash` pair carries DIFFERENT names and does not require `entry_index` disambiguation today, but the mechanism exists as a schema-level invariant for future-proof consumers.
2. **Invariant 6** (unchanged): Terminal-semantics key remains `trace_id + plugin_name + entry_index` — no change to the disambiguation tuple.
3. **Frontmatter**: `version: "1.16"` → `"1.17"`; `modified[]` entry added.

**POLICY 1 verification:** All prior content preserved verbatim except the `entry_index` semantics paragraph replacement above.
**POLICY 7 verification:** H1 heading unchanged.
**TD-031 verification:** No new line-number citations introduced.

---

## Amendment 2026-07-06 (v1.15 → v1.16 — F-P2-008: `entry_index` field for `plugin.abandoned`; Invariant 6 tuple key extended)

**Driver:** Adversary finding F-P2-008 (E-19 pass-2) — the `plugin.abandoned` event schema used `plugin_name` alone as the per-entry identifier, but the registry idiom permits multiple entries per `plugin_name` (e.g., `verify-factory-lock` has two entries). Consumers disambiguating abandoned plugins by `(trace_id, plugin_name)` would collapse two distinct in-flight invocations into one signal, making root-cause analysis impossible when both entries abandon in the same drain window.

**Changes made:**

1. **Event 5 wire format** (F-P2-008): `entry_index: <u32>` field added between `plugin_name` and `drain_window_ms`. Mandatory fields list updated to include `entry_index`.
2. **Event 5 `entry_index` semantics paragraph** (F-P2-008): Added after mandatory fields. States: 0-based ordinal from `enumerate()` of the async partition at spawn time; registry allows multiple entries per `plugin_name`; consumers need `(plugin_name, entry_index)` tuple for unambiguous identification.
3. **Invariant 6** (F-P2-008): Terminal-semantics key extended from `trace_id + plugin_name` pair to `trace_id + plugin_name + entry_index` tuple.
4. **EC-007** (F-P2-008): Per-event description updated to mention `entry_index` alongside `drain_window_ms`.
5. **Canonical Test Vectors `abandoned-one` row** (F-P2-008): Updated to assert `entry_index` correct.
6. **Frontmatter** (F-P2-008): `version` bumped `"1.15"` → `"1.16"`; `modified[]` entry added; `last_amended` date retained 2026-07-06.

**POLICY 1 verification:** All prior content preserved verbatim except the six changes listed above.
**POLICY 7 verification:** H1 heading unchanged (entry_index is a field extension, not a new event type).
**TD-031 verification:** No new line-number citations introduced.

---

## Amendment 2026-07-06 (v1.14 → v1.15 — F-P1-013: `plugin.abandoned` event catalog + drain-terminal semantics codified)

**Driver:** Adversary finding F-P1-013 (E-19 pass-1) — the async drain timer arm in `crates/factory-dispatcher/src/main.rs` fires with plugins still in-flight (EC-011) but no SS-03 catalog event covered this condition. Telemetry consumers had no observable signal for abandoned plugins; the abandoned-vs-late-completion race semantics were unspecified, creating implementation ambiguity.

**Semantics decision: option (a) abort-at-drain — abandoned is terminal.**

Three options were evaluated (F-P1-013):

- **(a) abort-at-drain**: `plugin.abandoned` is terminal for this invocation. No `plugin.completed` fires after it. **Selected.**
- **(b) both events may fire**: `plugin.abandoned` then `plugin.completed` (late completion), correlated by `trace_id + plugin_name`. **Rejected.**
- **(c) suppress `plugin.completed`**: Track abandoned set; suppress late completions. **Rejected.**

Option (b) rejected: the dispatcher process exits within milliseconds of the drain timer firing. In Tokio, dropping a `JoinHandle` does not cancel the underlying task — the spawned tasks continue — but the `tokio::main` runtime shutdown drops all pending tasks before they can write to a live FileSink. Additionally, the `rx` channel receiver is dropped at drain timer `break`, so any late result send on `tx_for_task` is silently discarded even in the narrow pre-exit window. No emission path exists for late completions. Option (c) rejected: it is mechanically equivalent to (a) without benefit — the emission path is already closed, so suppression logic would be dead code.

**Changes made:**

1. Frontmatter: `version` bumped `"1.14"` → `"1.15"`; `last_amended` updated to 2026-07-06; `modified[]` array entry added.
2. H1 title: `, \`plugin.abandoned\`` appended to the event list.
3. §Description: "four new event-type strings" replaced with "four new event-type strings (plus `plugin.abandoned` added by F5 E-19 pass-1)"; "catalog entry for all four" → "all five".
4. §Common Fields: table `session_id` row updated "all four" → "all five"; `plugin_name` row updated "(1 + 4)" → "(1, 4, and 5)"; closing paragraph updated — "Plugin-context events (1 + 4)" → "(1, 4, and 5)"; "none of the four" → "none of the five"; "All four event types" → "All five event types".
5. §Postconditions: Event 5 (`plugin.abandoned`) section added after Event 4, including trigger, wire format, mandatory fields, `drain_window_ms` semantics, and forward-reference to Invariant 6. "Sink destination" paragraph: "All four events" → "All five events".
6. §Invariants: Invariant 6 added — `plugin.abandoned` is terminal (option a), with full rationale for rejecting options (b) and (c).
7. §Edge Cases: EC-007 added (drain timer fires with N in-flight plugins).
8. §Canonical Test Vectors: `abandoned-none` and `abandoned-one` rows added.
9. §VP Anchors: VP-079 scope updated to "all five event types including `plugin.abandoned`".
10. §Verification Properties table: VP-079 row scope updated to "all five event types including `plugin.abandoned`".
11. §Architecture Anchors: first bullet extended with `plugin.abandoned` drain timer arm; FileSink bullet "four event types" → "five event types".

**POLICY 1 verification:** All prior content preserved verbatim except the changes listed above.
**POLICY 7 verification:** H1 heading updated to include `plugin.abandoned`; BC-INDEX row updated atomically (BC-INDEX v3.58→v3.59).
**TD-031 verification:** No `main.rs:[0-9]+` line-number citations introduced; stable function/section anchors used throughout (drain timer arm identified by `tokio::select!` section and EC-011 label).

---

## Amendment 2026-05-09 (v1.13 → v1.14 — F5 fix-burst-35 F-P36-001: Traceability Stories TBD→S-15.01)

**F-P36-001 (BC body vs BC-INDEX Stories drift):** Traceability `Stories` row updated from `TBD — single story per ADR-019 §6 (no phased rollout, user decision 2026-05-07)` to `S-15.01 (single story per ADR-019 §6)`. BC-INDEX row (v1.28) already listed S-15.01; source body was pre-F3. F3 story decomposition (PR #106 merged 2026-05-07) is canonical.

## Amendment 2026-05-09 (v1.12 → v1.13 — F-P25-003: emission-site anchors corrected; F-P25-006: duplicate last_amended removed)

**Drivers:**
- **F-P25-003** — §Architecture Anchors bullet 1 cited `engine.rs` as the async block discard path / timeout termination path. Post-merge, the call sites are in `main.rs` (lines 46/423/550) and the emit function registrations are in `host/emit_event.rs`. The §Traceability Architecture Module row similarly cited `engine.rs (emission sites)`. Both corrected to `main.rs + host/emit_event.rs (emission sites)`.
- **F-P25-006** — Frontmatter contained duplicate `last_amended:` fields (lines 5 and 9 both had `last_amended: 2026-05-08`). The duplicate (line 9) was removed; only line 5 retained.

**Changes made:**
1. **§Architecture Anchors bullet 1** (F-P25-003): `crates/factory-dispatcher/src/engine.rs` → `crates/factory-dispatcher/src/main.rs (call sites) + crates/factory-dispatcher/src/host/emit_event.rs (emit fns)`
2. **§Traceability Architecture Module row** (F-P25-003): `SS-01 — crates/factory-dispatcher/src/engine.rs (emission sites)` → `SS-01 — crates/factory-dispatcher/src/main.rs + crates/factory-dispatcher/src/host/emit_event.rs (emission sites)`
3. **Frontmatter** (F-P25-006): duplicate `last_amended:` field removed; `version: "1.12"` → `"1.13"`, `last_amended:` updated to 2026-05-09.

**POLICY 1 verification:** All prior content preserved verbatim.
**POLICY 7 verification:** H1 heading unchanged.
**TD-031 verification:** No line-number citations introduced.

---

## Amendment 2026-05-08 (v1.10 → v1.11 — F-P17-002: §Common Fields plugin_version removed)

**Driver:** **F-P17-002** — §Common Fields table listed `plugin_version` as a field present on all four event types. This was incorrect. None of the four BC-3.08.001 emit functions (`emit_dispatcher_schema_mismatch`, `emit_dispatcher_registry_invalid`, `emit_plugin_async_block_discarded`, `emit_plugin_timeout_async`) in `crates/factory-dispatcher/src/host/emit_event.rs` call `with_plugin_version()`. Only the generic plugin `emit_event` host function (which handles arbitrary plugin-emitted events) enriches with `plugin_version` — and that function is not used by any of the four structured events catalogued in this BC.

The v1.10 amendment (F-P16-005) introduced this error: it rewrote the §Common Fields closing paragraph to state `plugin_name + plugin_version explicitly shown` for Events 1+4. The actual wire examples for Events 1 and 4 show only `plugin_name`; neither shows `plugin_version`. The bats tests do not assert `plugin_version`. The v1.10 amendment paragraph was therefore internally inconsistent with the wire-format examples it purported to describe.

**POLICY 4 verification:** `grep -n "with_plugin_version" crates/factory-dispatcher/src/host/emit_event.rs` finds only one call site: inside the generic `emit_event` host function registration (line 46). None of the four named emit functions for BC-3.08.001 events call `with_plugin_version`.

**Changes made:**

1. **§Common Fields table** (F-P17-002): `plugin_version` row removed. `plugin_name` row description updated to note it is present on plugin-context events (1 + 4) only; absent from dispatcher-startup events (2 + 3).
2. **§Common Fields closing paragraph** (F-P17-002): Corrected to state `plugin_name` only (not `plugin_name + plugin_version`) for Events 1+4; added explicit note that `plugin_version` is NOT emitted by any of the four BC-3.08.001 emit functions.
3. **Frontmatter version:** `"1.10"` → `"1.11"`.

**POLICY 1 verification:** All prior content preserved verbatim except the two §Common Fields changes above. No event IDs renumbered. No wire-format examples changed (they never showed `plugin_version` — this removes the erroneous table row that contradicted them).

**POLICY 7 verification:** H1 heading unchanged.

**TD-031 verification:** No `emit_event.rs:[0-9]+` or `main.rs:[0-9]+` patterns introduced.

**Sync notes for story-writer:** No story task or AC changes required — the wire examples were already correct; only the §Common Fields prose is corrected to match them.

**Sync notes for test-writer:** No bats changes required. Bats tests never asserted `plugin_version` for these events; this amendment aligns the spec with the existing correct test behavior.

---

## Amendment 2026-05-08 (v1.9 → v1.10 — F-P16-002 + F-P16-005: emit_event.rs line citations migrated to stable symbol anchors; §Common Fields paragraph rewritten)

**Drivers:**
- **F-P16-002** — The v1.9 amendment narrative cited specific line numbers within `emit_event.rs` (for the four `.with_session_id` call sites and for the `reserved_fields_rejected` test). Per TD-031, `emit_event.rs:[0-9]+` patterns are prohibited in body text because line numbers drift as code evolves. The §Implementation Notes section similarly cited `emit_event.rs` lines 62-78 for the `RESERVED_FIELDS` constant.
- **F-P16-005** — §Common Fields closing paragraph stated that `plugin_name` and `plugin_version` were "omitted from examples for readability" across all four events. This contradicted Events 1 and 4 wire examples, which DO include `plugin_name` explicitly. The paragraph failed to differentiate between plugin-context events (1 + 4, which carry plugin identity) and dispatcher-startup events (2 + 3, which have no plugin context at startup).

**Changes made:**

1. **v1.9 amendment narrative — O-P15-001 driver bullet** (F-P16-002): Replaced `"lines 162, 193, 243, 289"` with the stable symbol anchor: `"all four emit_dispatcher_* and emit_plugin_*_async functions in crates/factory-dispatcher/src/host/emit_event.rs"`.

2. **§Implementation Notes — RESERVED_FIELDS table heading** (F-P16-002): Replaced `"emit_event.rs lines 62-78"` with `"the RESERVED_FIELDS constant in crates/factory-dispatcher/src/host/emit_event.rs"`.

3. **§Implementation Notes — reserved_fields_rejected sentence** (F-P16-002): Replaced `"(test at line 348)"` with the stable description: `"the reserved_fields_rejected integration test in crates/factory-dispatcher/src/host/emit_event.rs"`.

4. **§Common Fields closing paragraph** (F-P16-005): Rewritten to differentiate plugin-context events (1 + 4, `plugin_name` + `plugin_version` explicitly shown) from dispatcher-startup events (2 + 3, those fields omitted from examples), while clarifying that `trace_id` + `session_id` appear in all four examples per VP-079 and that `ts`/`ts_epoch`/`schema_version` are always emitted but not shown for readability.

5. **Frontmatter:** `version: "1.9"` → `"1.10"`.

**POLICY 1 verification:** All prior content preserved verbatim. No event IDs renumbered. No wire-format examples changed.

**POLICY 7 verification:** H1 heading unchanged.

**TD-031 verification:** No `emit_event.rs:[0-9]+` or `main.rs:[0-9]+` patterns remain in body text.

**Sync notes for story-writer:** No story task or AC changes required — these are spec-narrative and §Common Fields prose fixes only. No mandatory field lists were changed.

---

## Amendment 2026-05-08 (v1.8 → v1.9 — O-P15-001: session_id on wire for all four event types; O-P15-003: RESERVED_FIELDS full enumeration)

**Drivers:**
- **O-P15-001** — Wire-format examples for Events 1, 2, 3-E-REG-002, and 4 did not show the `session_id` field. All four `emit_dispatcher_*` and `emit_plugin_*_async` functions in `crates/factory-dispatcher/src/host/emit_event.rs` call `.with_session_id(&ctx.session_id)`, so `session_id` IS on the wire for all four event types. Only the E-REG-003 example (introduced in v1.8) showed `session_id`. This was a spec omission, not an implementation gap.
- **O-P15-003** — §Implementation Notes discussed only `trace_id` and `dispatcher_trace_id` from the reserved-fields filter. The full `RESERVED_FIELDS` constant in `crates/factory-dispatcher/src/host/emit_event.rs` contains 9 fields; the remaining 7 (`session_id`, `plugin_name`, `plugin_version`, `ts`, `ts_epoch`, `schema_version`, `type`) were invisible to spec readers.

**Changes made:**

1. **§Common Fields section added** (O-P15-001, O-P15-003): New section between Preconditions and Postconditions. Enumerates all 8 dispatcher-owned fields present on every event (`trace_id`, `session_id`, `plugin_name`, `plugin_version`, `ts`, `ts_epoch`, `schema_version`, `type`). States that `session_id` and `trace_id` appear explicitly in wire-format examples because VP-079 verifies them; remaining fields present but omitted from examples for readability.

2. **Event 1 (`plugin.async_block_discarded`) wire-format example** (O-P15-001): `session_id` field added between `trace_id` and `plugin_name`. Mandatory fields paragraph updated: `session_id` added.

3. **Event 2 (`dispatcher.schema_mismatch`) wire-format example** (O-P15-001): `session_id` field added between `trace_id` and `found_version`. Mandatory fields paragraph updated: `session_id` added.

4. **Event 3 E-REG-002 (`dispatcher.registry_invalid`) wire-format example** (O-P15-001): `session_id` field added between `trace_id` and `offending_plugin`. (E-REG-003 example already showed `session_id` per v1.8; unchanged.)

5. **Event 4 (`plugin.timeout` async path) wire-format example** (O-P15-001): `session_id` field added between `trace_id` and `plugin_name`. Mandatory fields paragraph updated: `session_id` added.

6. **§Implementation Notes — RESERVED_FIELDS subsection extended** (O-P15-003): Subsection heading updated to cite O-P15-003. Full 9-field enumeration table added. Cross-reference to HOST_ABI.md and implementation test added.

7. **Frontmatter:** `version: "1.8"` → `"1.9"`.

**POLICY 1 verification:** All prior content preserved verbatim. No event IDs renumbered. E-REG-003 wire-format example unchanged.

**POLICY 7 verification:** H1 heading unchanged.

**POLICY 12 (TV emitter consistency):** Canonical Test Vectors table rows assert `trace_id present on all` (fan-out-happy-path row) — this coverage already included `session_id` implicitly via the Common Fields definition. No TV row changes required: VP-079's payload conformance check now has the §Common Fields section as the authoritative field list, and the test vectors continue to exercise the same triggering scenarios.

**Sync notes for story-writer:** BC-3.08.001 v1.9 changes are spec-only (wire examples and notes); no story task changes required. If any story task currently references "mandatory fields" from the pre-v1.9 wire examples, those task bodies should be updated to include `session_id` in each event's field list.

**Sync notes for test-writer:** Bats tests that assert `plugin.async_block_discarded`, `dispatcher.schema_mismatch`, `dispatcher.registry_invalid` (E-REG-002 path), and `plugin.timeout` events MUST be updated to assert `session_id` is present and non-null in the emitted JSON. The E-REG-003 path already required `session_id` per v1.8 — only the other three event types need test updates.

---

## Amendment 2026-05-08 (v1.7 → v1.8 — F-P14-001 Path B: E-REG-003 wire schema extended with offending_event + offending_tool)

**Driver:** F-P14-001 Path B — Cross-BC contradiction since fix-burst-7: BC-7.06.001 v1.8 §E-REG-NNN Error Code Table declares the authoritative E-REG-003 `dispatcher.registry_invalid` payload as including `offending_plugin`, `offending_event`, `offending_tool`, `violation`, `timestamp`, `error_code`. BC-3.08.001 v1.7 Event 3 E-REG-003 wire-format example only enumerated `offending_plugin`, omitting `offending_event` and `offending_tool`. The data already exists in `RegistryError::DuplicateEntry { name, event, tool }` — the divergence was a spec-level omission, not an implementation gap.

**User-decided resolution:** Path B — extend BC-3.08.001 wire schema to match BC-7.06.001's authoritative enrichment. (Path A would have stripped the fields from BC-7.06.001 v1.8; Path B was chosen because the extra fields carry diagnostic value and are already present in the Rust error variant.)

**Changes made:**

1. **Event 3 E-REG-003 wire-format example extended** (F-P14-001): JSON example now includes `session_id`, `offending_event` (string, required), and `offending_tool` (string or null, required). Field ordering normalized to match BC-7.06.001 v1.8 authoritative schema.

2. **Mandatory fields paragraph for E-REG-003 updated**: `offending_event` (string, required) and `offending_tool` (string or null, required) added to the mandatory fields list. `session_id` added. Null semantics for `offending_tool` clarified (null when duplicating entry has no `tool` filter).

3. **E-REG-002 vs E-REG-003 asymmetry note added** (F-P14-001): Explains why E-REG-002 does NOT carry `offending_event`/`offending_tool` (intra-entry violation) while E-REG-003 does (inter-entry violation — the tuple identifies the duplicating entry). Cites implementation anchor: `RegistryError::DuplicateEntry { name, event, tool }`.

4. **EC-004b updated**: "offending_plugin set to first duplicate entry name" replaced with "offending_plugin/offending_event/offending_tool set to the duplicating entry's tuple" (F-P14-001 POLICY 12 TV emitter consistency).

5. **Frontmatter:** `version: "1.7"` → `"1.8"`.

**Sync notes for implementer**: The event emission site for E-REG-003 in `factory_dispatcher::main::run` (or `registry.rs::validate()`) must accept the full `RegistryError::DuplicateEntry { name, event, tool }` destructure and propagate all three fields to the structured event. The emit call signature must accept `event: &str` and `tool: Option<&str>` (already present in the `DuplicateEntry` variant). No new fields are required in `RegistryError` itself — only the event emission path needs updating to include `offending_event` and `offending_tool` alongside the existing `offending_plugin`.

**Sync notes for test-writer**: Bats S8 tests that assert `dispatcher.registry_invalid` for E-REG-003 MUST be updated to assert all three fields: `offending_plugin`, `offending_event`, and `offending_tool`. A test vector that asserts only `offending_plugin` is now insufficient. The `offending_tool` field must be `null` when the duplicating entry has no `tool` filter and a regex string otherwise.

**POLICY 1 verification:** All prior content preserved verbatim. E-REG-002 wire-format example unchanged (asymmetry note explains why it remains field-minimal). No event IDs renumbered.

**POLICY 7 verification:** H1 heading unchanged.

**F-P14-001 cross-reference:** BC-7.06.001 v1.8 §Sibling BC-3.08.001 cross-reference at line 204 referenced "BC-3.08.001 v1.7 lines 107-117" as the SS-03 catalog mirror. That note is now superseded: BC-3.08.001 v1.8 Event 3 (E-REG-003 wire format section) is the updated SS-03 catalog mirror, and the two BCs are now consistent.

---

## Amendment 2026-05-08 (v1.7 — O-P10-001: phase frontmatter corrected F8 → F2)

**Driver:** O-P10-001 metadata defect — frontmatter field `phase:` contained `F8`, a value that appears nowhere else in the F-series BC corpus. All sibling BCs produced in the same feature cycle (BC-7.06.001, BC-1.14.001, BC-9.01.006) declare `phase: F2`. This was a typo introduced when the frontmatter was last written.

**Change:** `phase: F8` → `phase: F2` (line 9). No body content altered. No version bump — pure metadata correction per project convention for frontmatter-only fixes.

---

## Amendment 2026-05-08 (v1.6 → v1.7 — F-P8-001 sibling: Event 3 E-REG-003 added; violation string canonicalized)

**Driver:** F-P8-001 PO sync — BC-7.06.001 v1.6 (amended in the same burst) establishes that `dispatcher.registry_invalid` has two valid error codes: `E-REG-002` (async block conflict) and `E-REG-003` (duplicate hook registration). BC-3.08.001 v1.6 only enumerated E-REG-002 in Event 3, omitting E-REG-003 entirely. Additionally, the canonical violation string for E-REG-002 was normalized in BC-7.06.001 from the legacy value `"on_error_block_with_async_true"` to `"async_block_conflict"`; BC-3.08.001 v1.6 still carried the legacy string.

**Changes made:**

1. **Event 3 wire-format section expanded** — trigger description now states both violation conditions. A two-row enum table lists `E-REG-002 / async_block_conflict` and `E-REG-003 / duplicate_hook_registration` with their triggering conditions. Two wire-format examples are provided (one per error code). The mandatory fields paragraph clarifies that `error_code` is an enum with exactly these two valid values and that `violation` is determined by `error_code`.

2. **Canonical violation string normalized** — `"on_error_block_with_async_true"` replaced by `"async_block_conflict"` throughout Event 3 (wire format example, EC table, test vectors). This matches BC-7.06.001 v1.6 as the canonical authority. **Bats tests for S8 and any test file asserting `"on_error_block_with_async_true"` MUST be updated to `"async_block_conflict"` before delivery.**

3. **Edge Cases table** — EC-004 split into EC-004a (AsyncBlockConflict / E-REG-002) and EC-004b (DuplicateEntry / E-REG-003).

4. **Canonical Test Vectors table** — the single `registry-invalid` row split into `registry-invalid-E-REG-002` and `registry-invalid-E-REG-003` rows.

5. **Frontmatter** — `version:` bumped `"1.6"` → `"1.7"`.

No other postconditions, invariants, or verification properties were modified. POLICY 1 (append-only, no event renumbering) and POLICY 7 (H1 unchanged) observed.

## Amendment 2026-05-08 (v1.5 → v1.6 — F5 fix-burst-2 F-P2-015: last_amended frontmatter format normalized)

**Driver:** F5 pass-2 finding F-P2-015 — the `last_amended:` frontmatter field contained an embedded parenthetical annotation `(v1.5 — F5 pass-1 fix-burst F-P1-007)` appended to the date string. This non-standard format may break date-parsing tooling that expects a bare ISO-8601 date value in this field.

**Changes made:**
- Frontmatter `last_amended:` cleaned to bare date `2026-05-08` (parenthetical removed).
- Parenthetical content ("v1.5 — F5 pass-1 fix-burst F-P1-007") is preserved in the §Amendment 2026-05-08 (v1.4 → v1.5) changelog entry below, where it already appeared as the section title.
- Frontmatter `version:` bumped `"1.5"` → `"1.6"`.

No behavioral, wire-format, invariant, or test-vector content was changed. This is a frontmatter hygiene fix only.

## Amendment 2026-05-08 (v1.4 → v1.5 — F5 pass-1 fix-burst F-P1-007)

Addresses adversary F5-pass-1 finding F-P1-007 (trace_id duality on wire).

**F-P1-007 (trace_id is the canonical wire-field name)**: The implementation in `crates/factory-dispatcher/src/host/emit_event.rs` (the `with_trace_id` call path) emitted both `dispatcher_trace_id` (via `with_trace_id(...)`) and `trace_id` (via `with_field("trace_id", ...)`) on the wire. This created an ABI inconsistency: sink consumers may parse one or the other; RESERVED_FIELDS included `dispatcher_trace_id` but not `trace_id`, allowing plugins to spoof the trace correlation field.

User-approved resolution (option b): code emits only `trace_id`; `dispatcher_trace_id` removed from wire output; `trace_id` added to RESERVED_FIELDS.

The wire format examples in §Postconditions (Events 1-4) already showed `trace_id` only — no change needed there. The spec was correct; the implementation deviated.

**Changes made:**
- Frontmatter `version:` bumped to `"1.5"`; `last_amended:` updated to `2026-05-08`
- Invariant 5 added: `trace_id` is the exclusive wire-format field name; `dispatcher_trace_id` MUST NOT appear on wire; plugins MUST NOT emit `trace_id` via `with_field()` (it is dispatcher-reserved)
- `## Implementation Notes` section added with `### RESERVED_FIELDS and trace_id` subsection: explicit guidance that `trace_id` must be added to RESERVED_FIELDS; `dispatcher_trace_id` retained in RESERVED_FIELDS for defense-in-depth; plugins attempting to set either field are silently stripped
- Traceability L2 Domain Invariants: DI-017 entry expanded to note that Invariant 5 enforces the canonical wire-field-name requirement

**Canonical Test Vectors**: All existing test vectors show `trace_id` only (no `dispatcher_trace_id`); no changes required. Wire format examples in Postconditions §Events 1-4 already compliant.

## Amendment 2026-05-07 (v1.3 → v1.4 — F2 pass-10 NIT-P10-001 cleanup; sibling fix to F-P7-004)

**Redundant `(per DI-019)` parenthetical removed from Traceability L2 Domain Invariants cell.** The DI-019 entry in the cell already began with `DI-019 —` prefix, making the trailing `(per DI-019)` redundant. Pass-7 F-P7-004 fixed exactly this pattern in the sibling BC-1.14.001 (v1.5→v1.6) but the parallel fix in BC-3.08.001 was not applied at that time.

This is a cosmetic-only change. No postconditions, wire formats, invariants, test vectors, or verification properties were modified.

## Amendment 2026-05-07 (v1.2 → v1.3 — F2 pass-6 F-P6-003: inline literal removed; sibling-fix to F-P4-005)

**DI-019 inline literal removed from Traceability L2 Domain Invariants cell.** The text `ASYNC_DRAIN_WINDOW_MS = 100 ms` was an inlined value literal that violated the DI-019 canonical-value rule (same pattern removed from BC-1.14.001 v1.4 → v1.5 in the F2 pass-4 burst). Replaced with `ASYNC_DRAIN_WINDOW_MS (per DI-019)` — DI-019 is the single source of truth for the numeric value.

This is a traceability-text-only change. No postconditions, wire formats, invariants, test vectors, or verification properties were modified.

## Amendment 2026-05-07 (v1.1 → v1.2 — F2 pass-3 user-correction: DI-019 cross-reference added)

**DI-019 traceability cross-reference added** per user-directed structural correction (same burst as BC-1.14.001 v1.3 → v1.4 and invariants.md v1.4 → v1.5).

DI-019 (`ASYNC_DRAIN_WINDOW_MS = 100 ms`) was lifted from BC-1.14.001 v1.3's inline "Constant Definitions" table to a domain invariant. Two of the four event types catalogued in this BC (`plugin.timeout` async path and `plugin.async_block_discarded`) are emitted by async tasks running within the drain window bounded by DI-019. The Traceability L2 Domain Invariants field now cites DI-019 alongside DI-017 to make this dependency explicit.

This is a traceability-only change. No postconditions, wire formats, invariants, or test vectors were modified.

**Architect obligation:** VP-079 fixture timing (which verifies that these events reach FileSink before dispatcher exit) must anchor to DI-019 for the drain window budget. This is unchanged from the VP-079 obligation noted in BC-1.14.001 v1.3.

## Amendment 2026-05-07 (v1.0 → v1.1 — F2 pass-2 fix burst)

Addresses adversary pass-2 finding F-P2-010.

**F-P2-010 (Architecture Module misclassification)**: Traceability Architecture Module field previously listed "SS-07 — `crates/factory-dispatcher/src/registry.rs`" for the schema_mismatch + registry_invalid emission sites. Per ARCH-INDEX, `crates/factory-dispatcher/src/registry.rs` is owned by SS-01 (the SS-01 row in ARCH-INDEX explicitly lists `{main,registry,routing,executor,invoke,engine,plugin_loader,payload}.rs`). SS-07 owns `plugins/vsdd-factory/hooks/*.sh` and `hooks-registry.toml` (the file format) — not `registry.rs` (the Rust module that reads it). Updated to "SS-01 — `crates/factory-dispatcher/src/registry.rs`" with a clarifying note that SS-07 still owns the TOML file format. This is a POLICY 6 (architecture_is_subsystem_name_source_of_truth) HIGH severity fix.

## Amendment 2026-05-08 (v1.11 → v1.12 — F-P23-002: cross-subsystem source-line-cite migrated to stable symbol anchor)

**Driver:** F-P23-002 pass-23 cross-subsystem corpus sweep (per L-P20-001 / L-P22-001 broadest scope mandate) — §Implementation Notes §RESERVED_FIELDS note cited `HOST_ABI.md (line 267)`. This references a line number in `crates/hook-sdk/HOST_ABI.md` which drifts as the doc evolves. Per TD-VSDD-091, source-file line cites must migrate to stable symbol anchors. The §`emit_event` section is the stable anchor.

**Changes made:**
- §Implementation Notes §RESERVED_FIELDS cross-reference note: `(line 267)` → `(§\`emit_event\` enrichment description; source-line carve-out per TD-VSDD-091: line 267 is unstable, stable anchor is §\`emit_event\` section)`.
- Frontmatter `version:` bumped `"1.11"` → `"1.12"`.

**Changelog:**

| 1.38 | 2026-10-08 | product-owner | ADR-052 v1.25 item 11(g) (closes the Event 12 / operator-line cap conflation; independent of any wire-format change). Sanitization caps are per SURFACE: the 64-character cap on data-derived substrings applies to the `InternalLog` events 11–13 ONLY; the operator stderr rendering of the same facts (the coordinator failure line and the `Display` of `BcIndexMigrationError`) is capped at 256 per BC-1.18.013 v1.14 Postcondition 10 "Coordinators" clause. Edited: Event 12 `detail` field semantics and Invariant 7 keep "truncated to 64 characters for any data-derived substring" and add "(InternalLog event only; the operator stderr rendering of the same fact is capped at 256 per BC-1.18.013 Postcondition 10)"; Invariant 7 additionally states the raw-slot / render-time-sanitization rule (never re-truncated after escaping); EC-018 and the Event 12 Canonical Test Vectors gain a ~100-character-path vector (64 in the event, untruncated on stderr) and a 108-character macOS path vector. NO new event, NO new field, NO event-count change (still thirteen), `cause`/`kind` domains, mandatory fields, sink target and Invariant numbering unchanged. Coordinator-only advisory emission is mirrored in BC-1.18.013 v1.14; this BC records only that any dispatcher-reachable advisory condition is an Event 13 (already the Event 13 catalog). `modified:` gains the v1.38 entry. **Stories affected by BC changes:** none (no `bcs:` array change). **VP citations changed in:** none. Sibling sweep: BC-1.18.013 v1.14, BC-1.18.011 v1.22, error-taxonomy v1.42. |
| 1.37 | 2026-10-07 | product-owner | ADR-052 v1.23 "third binary-leg extension" (§Error Code Semantics item 10(c); independent validation `.factory/research/adr-052-v123-architect-calls-validation.md`; POLICY 13 count-phrase sweep). Event 12 `migration.admission_failed` field `kind` closed domain grows from FIVE to SIX tokens: adds `txn_record_newer_schema` (a txn record whose `schema_version` is an integer >= 2, written by a newer build; reachable on the admission surface through the Branch B / Branch C version gate, BC-1.18.013 Precondition 6(c) rule 9, and surfaced by the coordinator as `MIGRATION_STATE_INTEGRITY_FAILURE` exit 2). Edited: Event 12 wire-format `kind` comment and Field-semantics paragraph (closed six-token domain), EC-018 expected output, Canonical Test Vectors (NEW newer-schema row for `schema_version` 2 and 4294967296; NEW closed-domain assertion row). Additive wire-domain token ONLY: NO new event, NO new field, NO event-count change (still thirteen), `cause` domain {`invalid_tool_use_id`, `io`, `state_integrity`} unchanged, mandatory fields and sink target unchanged, no Invariant change. `modified:` frontmatter gains the v1.36 and v1.37 entries (v1.36 had omitted its own). Sweep: no remaining "five kinds" / "five-token" prose for the `kind` domain in this BC (the surviving "five branches" in EC-017 and "five-token `reason` domain" of Event 13 are different domains). VP propagation (architect, `vp_index_is_vp_catalog_source_of_truth`): VP-079 Event 12 vectors and VP-133 facet 7(e) gain the token. BC-INDEX BC-3.08.001 version cell += v1.37 (state-manager). |
| 1.36 | 2026-10-07 | product-owner | Story-anchor re-attribution after the human-approved split of S-25.08 into S-25.08 + NEW S-25.09 (D-1252(f), amended so AC-027 stays in S-25.08). Documentary only — NO wire-format, field-shape, Invariant or event-count change. Events 11–13 (`migration.admission_blocked` / `_failed` / `_advisory`) are implemented by S-25.09 AC-002 (`InternalLog::write` in the `main.rs` shell): Story Anchor and Traceability Stories row now name S-25.09; the "S-25.08 seam" / "S-25.08-seam tokens" / "S-25.08 never emits it" / "S-25.08: exit 2" prose sites (Event 11 `branch` semantics, Event 13 Trigger, EC-019, the finalize-unwired test vector, and the §Description Events 11–13 provenance sentence) re-pointed to S-25.09. The historical v1.35 changelog row and v1.35 Amendment section are immutable and still say "S-25.08 seam"; those references are superseded by S-25.09. POL-14 note: Events 11–13 land with S-25.09. Also: live VP cites in §VP Anchors refreshed to VP-079 v1.25, VP-133 v1.7, VP-143 v1.6 (architect VP-INDEX v3.31); `branch_c_finalized` prose now reads "neither S-25.08 nor S-25.09 emits it" (only S-25.06 emits it); ADR-052 v1.22 read-failure ruling (non-ENOENT canonical/terminal read failure ⇒ `E-MAINTENANCE-002 (io)`, `migration.admission_failed`, never an Event 11 `check`) is consistent with Event 11/12 semantics — no Event field change. Sibling sweep: BC-1.18.013 v1.11, BC-1.18.011 v1.19. |
| 1.35 | 2026-10-07 | product-owner | ADR-052 §Decision 5a v1.21 "Admission diagnostics channel" (S-25.08 local adversary pass 2 D-1; ADR-052 v1.21 §Downstream item 25). Three NEW events catalogued — Event 11 `migration.admission_blocked`, Event 12 `migration.admission_failed`, Event 13 `migration.admission_advisory` — the structured admission/release/reconciliation diagnostics that BC-1.18.011 v1.18 / BC-1.18.013 v1.10 Postcondition 10 mandate in place of `tracing::*!` calls (the dispatcher installs no `tracing` subscriber, so those were discarded in production). Trigger = the `main.rs` shell writing the `AdmissionDiagnostic` data returned by the shared core; fields per ADR §5a (no raw `tool_use_id`, no record content); emission path = `InternalLog::write` directly (NOT `HostContext::emit_internal`; recorded deviation); durable sink = `dispatcher-internal-{date}.jsonl`; no `plugin_name`. H1/§Description/§Common Fields/§Invariants 1 and 3 (+ new Invariant 7)/§Related BCs/§Architecture Anchors/§VP Anchors/§Edge Cases (EC-017..EC-019)/§Canonical Test Vectors/§Traceability updated; Events 1–10 unchanged. VP citations changed in this BC (VP-079 v1.24 extended to Events 11–13 and its staleness flag closed; VP-028 v1.1 scope note; emission semantics cite VP-133 v1.6 facet 7(e) / VP-143 v1.5 — architect already propagated, no new VP). Folded-in follow-up (ADR-052 v1.21 §Downstream item 33 (a)–(f)): Event 11 `reconciliation` is the closed six-token effectful domain with the total `branch` derivation (no `none`); Event 13 `reason` is the closed five-token domain (`reservation_release_failed` / `branch_a_gate_reopened` / `branch_b_txn_aborted` / `branch_c_finalize_unwired` / `branch_c_finalized`), the five timestamp tokens being coordinator-drain stderr tokens; Event 13 optional fields are the closed set {`migration_id`, `txn_id`, `check`, `detail`, `tool_use_id_len`}. Item 33(g): an undelivered Branch C finalize (S-25.08 seam) writes the one Event 11 with `reconciliation="completion_record_mismatch"`, `branch="completion_record_mismatch"`, `check="finalize_unwired"` plus Event 13 `reason="branch_c_finalize_unwired"`; after S-25.06 delivers the finalize there is no Event 11, Event 13 `reason="branch_c_finalized"`, and `finalize_unwired` is retired. **Folded-in follow-up 2 (S-25.08 implementation review @ f8726c34, same version, uncommitted):** Event 11 `check` is a CLOSED nine-token domain (`staging_with_terminal_record`, `terminal_record_unparseable`, `terminal_record_schema_mismatch`, `txn_id_mismatch`, `generation_id_mismatch`, `canonical_paths_count_mismatch`, `canonical_hash_mismatch`, plus seam tokens `terminal_record_unverified` and `finalize_unwired`, both retired by S-25.06); read-CALL failures stay Event 12 (`io`). Amendment section appended. |
| 1.34 | 2026-09-01 | product-owner | ADR-048 §Decision 4 v1.5 Emission-Point Correction (S-25.01 adversary pass 9 F-P9-001 MEDIUM; architect adjudication; HUMAN-RATIFIED 2026-09-01, POLICY 22, D-1142). Event 9's SUPERSEDED `marker.cleared` emission is relocated from an unconditional pre-overwrite read to inside `write_indeterminate_marker`'s `Ok(())` arm, symmetric with Event 10's v1.33 `marker.written` "emit only after `Ok(())`" rule — both write-tied events now fire back-to-back in the same write-success arm (SUPERSEDED first, then `marker.written`); on `Err(_)`, NEITHER event is emitted. This closes F-P9-001: the pre-v1.34 formulation emitted SUPERSEDED unconditionally at the pre-overwrite field-read, so a subsequent write failure (EC-007) left the OLD marker still durably present and enforcing while a SUPERSEDED record had already been emitted falsely claiming it was overwritten — a fabricated audit record (NIST AU-3/AU-10), the un-swept sibling of the v1.33 `marker.written` fix. (1) Event 9 PC5 Trigger bullet corrected: read-before-overwrite is unavoidable and unchanged, but emission is now gated on `Ok(())`. (2) `clear_mode`/`actor_type` correspondence table SUPERSEDED row corrected to the same ordering. (3) Emission path paragraph gains the v1.34 correction sentence. (4) Event 10 Trigger paragraph corrected — no longer claims `marker.written` fires "BEFORE ANY other marker-lifecycle event for this write" (stale/misleading now that SUPERSEDED shares the same arm); states the SUPERSEDED-then-`marker.written` order explicitly. (5) Architecture Anchors: Event 9 bullet and Event 10 bullet both gain the v1.34 correction note. (6) EC-014 corrected to AFTER-`Ok(())` ordering; EC-016 extended with the symmetric SUPERSEDED-withholding-on-`Err(_)` clause. (7) Canonical Test Vectors: `marker-cleared-superseded` row corrected to AFTER-`Ok(())` ordering; new `marker-cleared-superseded-failure-no-emit` row added. (8) Verification Properties: new VP-108 row for the v1.34/F-P9-001 negative-control regression test (PC8). (9) Traceability ADR row gains the ADR-048 §Decision 4 v1.5 citation. No wire-format/field-shape change — `clear_mode`, `actor_type`, and the `reason`-mandatory-for-SUPERSEDED rule are UNCHANGED; this is purely an emission-point/ordering correction. Companion architect-side correction: VP-108 v1.3 → v1.4 (PC5 fixture reordered to write-before-emit; new PC8 negative control). |
| 1.33 | 2026-09-01 | product-owner | ADR-048 §Decision 4 v1.4 Reconciliation-Premise Correction (S-25.01 adversary pass 6 F-P6-001 MEDIUM; architect adjudication; HUMAN-RATIFIED 2026-09-01, POLICY 22, D-1141). New Event 10 `marker.written` added to the SS-03 event catalog — the positive marker-creation record emitted by `write_indeterminate_marker`'s caller (`executor.rs`) ONLY after the atomic write returns `Ok(())`, via `emit_marker_written(ctx, &fields)` → `ctx.emit_internal`, never before the write, never on `Err(_)`. Full triggering-condition/semantics authority: BC-1.18.001 §PC4 v1.4; this BC registers the wire-format/field-shape catalog entry only (same split as Events 7/8/9). Count-phrase sweep: nine→ten event types throughout (H1, §Description, §Common Fields, §Invariants 1+3, §VP Anchors VP-079+VP-028, §Verification Properties VP-079 row, §Traceability DI-017). (1) H1 title: `marker.written` appended. (2) §Description: provenance sentence added; "all nine" → "all ten"; note added that Events 8/9/10 durably land in `dispatcher-internal-{date}.jsonl`. (3) §Common Fields: "All nine" → "All ten" (×2); `plugin_name` row and bullet list extended with Events 9/10 sourcing note. (4) New `### Event 10: marker.written` section added under §Postconditions (full wire format, mandatory fields, `cause`/`expires_at`/`ts` semantics, relationship to Event 9 OPERATOR_OVERRIDE reconciliation, emission path, durable sink target). (5) Event 9 retargeted: OPERATOR_OVERRIDE Trigger bullet, `clear_mode`/`actor_type` correspondence table row, `trace_id` semantics paragraph all corrected from unmatched `plugin.indeterminate` to unmatched `marker.written` (ADR-048 §D4 v1.4, F-P6-001). (6) §Invariants 1+3: "all nine"/"All nine" → "all ten"/"All ten"; `marker.written` added to Invariant 3's event enumeration. (7) §Related BCs: BC-1.18.001 bullet extended to cover Event 10 authorship; BC-1.18.003 bullet notes RAW_DELETE_DETECTED now sound against `marker.written`. (8) §Architecture Anchors: new `emit_marker_written` bullet added; `reconcile_raw_delete` bullet's scan-target prose extended with the v1.33 match-type correction. (9) §VP Anchors: VP-079 gains a new "Staleness flag OPEN (v1.33)" for Event 10 (Property Statement→ten events, mandatory-fields table gains `cause`/`expires_at`, new SITE_10); VP-108 staleness noted (PC3 fixture retarget + new PC6/PC7). VP-028 list gains `marker.written`; "all nine" → "all ten". (10) §Edge Cases: EC-013 retargeted to `marker.written`; new EC-015 (`Ok(())` → emission) and EC-016 (`Err(_)` → no emission) added. (11) §Canonical Test Vectors: OPERATOR_OVERRIDE row retargeted; three new rows added (`marker-written-success`, `marker-written-failure-no-emit`, `marker-written-negative-control-pretooluse`). (12) §Verification Properties: VP-079 row → "ten event types"; two new VP-108 rows added (write-path emission, negative-control regression test). (13) §Traceability: DI-017 "all nine" → "all ten"; Architecture Module row gains `emit_marker_written` bullet; ADR row gains ADR-048 §Decision 4 v1.4 citation; Stories row extended to "Events 8 + 9 + 10". (14) Amendment section + Changelog row added. No wire-format/field-shape change to Events 1–9; this is purely additive — a new, tenth event type. |
| 1.32 | 2026-09-01 | product-owner | ADR-048 §Decision 4 v1.3 Emission-Mechanism Precision Correction (S-25.01 LOCAL adversary pass 3 F-P3-001 MEDIUM) + SUPERSEDED Clear Mode (F-P3-002 LOW); human-ratified 2026-09-01, POLICY 22, D-1140, per ADR-048 v1.3 Status. (1) Event 9 wire format: `clear_mode` enum gains `SUPERSEDED`; `actor_type` enum gains `system`; `reason`-mandatory condition extended to "mandatory for OPERATOR_OVERRIDE and SUPERSEDED". `clear_mode`/`actor_type` correspondence table gains a SUPERSEDED/system row citing BC-1.18.001 INV3 + BC-1.18.003 PC5. `trace_id`/`reason` semantics paragraphs extended with the SUPERSEDED case (superseded pair's own trace_id, never the new pair's; literal SUPERSEDED reason string). (2) §Sink destination prose corrected: the claim that all nine events are "NOT routed to the dispatcher-internal debug stream" was INACCURATE — all nine route via `HostContext::emit_internal`, whose `InternalLog` write is unconditional (not gated behind the opt-in DI-007 flag) and durably lands in `dispatcher-internal-{date}.jsonl`; `emit_internal` also pushes onto `ctx.events` for eventual FileSink drain once S-4.07 lands. (3) Events 8 and 9's §Emission path paragraphs corrected to name `HostContext::emit_internal` (`ctx.emit_internal(ev)`/`base_ctx.emit_internal(ev)`) explicitly instead of a raw `InternalLog::write`, matching how the seven sibling BC-3.08.001 dispatcher-native events are already described; new §Durable sink target note (Event 9) explains the deliberate scan-target divergence from the literal `events-{date}.jsonl` filename — `dispatcher-internal-{date}.jsonl` is the only log every default production run writes unconditionally, pending S-4.07's Router/FileSink wiring. (4) §Trigger bullets: PC3 bullet's scan-target reworded to `dispatcher-internal-{date}.jsonl`; new PC5 SUPERSEDED bullet added; "Full behavioral governance" sentence extended to cite BC-1.18.003 PC5 + BC-1.18.001 INV3. (5) §Architecture Anchors: `marker.cleared` emission-path bullet rewritten — `emit_marker_cleared` signature widened to `ctx: &HostContext`, all four call sites (including the new SUPERSEDED trigger in `write_indeterminate_marker`'s caller) named, emission mechanism corrected to `ctx.emit_internal`; `plugin.indeterminate` bullet's `emit_indeterminate` corrected the same way; §Traceability Architecture Module row's stale "SS-04 plugin crate TTL_EXPIRED branch" attribution (predating the v1.2 correction) also swept to the current dispatcher-native-only state. (6) EC-013 scan-target wording corrected; new EC-014 (SUPERSEDED cross-pair overwrite) added. (7) Canonical Test Vectors: `events-*.jsonl`/`FileSink log` phrasing in the three existing marker.cleared rows corrected to `dispatcher-internal-{date}.jsonl`; new `marker-cleared-superseded` row added. (8) §VP Anchors + §Verification Properties: VP-079 staleness flag extended (v1.32) — SITE_9 and mandatory-fields table must assert SUPERSEDED/system coverage; event count stays at nine. (9) §Traceability ADR row: ADR-048 §Decision 4 v1.2 Emission-Point Correction citation added (previously missing) + new v1.3 citation. No new event added — `SUPERSEDED` is a new VALUE of the existing Event 9 `clear_mode` enum. |
| 1.31 | 2026-08-31 | product-owner | ADR-048 §Decision 4 v1.2 Emission-Point Correction (S-25.01 LOCAL adversary pass 2 F-P2-002 HIGH + F-P2-003 MED; human-ratified): Event 9 `clear_mode`/`actor_type` correspondence table's TTL_EXPIRED and OPERATOR_OVERRIDE "Emission point" cells re-attributed from the WASM gate plugin to dispatcher-native `check_and_clear_expired_marker`/`reconcile_raw_delete`. §Emission path paragraph, §Trigger bullets, §Architecture Anchors bullet, EC-012/EC-013, and the two `marker-cleared-ttl-expired`/`marker-cleared-operator-override` Canonical Test Vectors rows corrected to match. Event count unchanged at nine; no count-phrase sweep required. |
| 1.30 | 2026-08-31 | product-owner | ADR-048 §Decision 4: Event 9 `marker.cleared` added to SS-03 event catalog. Count-phrase sweep: eight→nine throughout (H1, §Description, §Common Fields, §Sink destination, §Invariants 1+3, §VP Anchors VP-079+VP-028, §Verification Properties VP-079 row, §Traceability DI-017+ADR+Architecture Module). Event 9 wire format section added with `clear_mode`/`actor_type` table, `trace_id`/`reason` semantics. EC-011/EC-012/EC-013 added (REVALIDATED/TTL_EXPIRED/OPERATOR_OVERRIDE clear edge cases). Canonical test vectors `marker-cleared-revalidated`, `marker-cleared-ttl-expired`, `marker-cleared-operator-override` added. §Related BCs: BC-1.18.003 added as Event 9 clear-path authority. §Architecture Anchors: `emit_marker_cleared` emission path + plugin crate paths added. §VP Anchors VP-079: staleness flag OPEN for Event 9 (architect must propagate to VP-079 SITE_9, mandatory-fields table, Property Statement). §VP-028 list: `marker.cleared` added. §Traceability ADR: ADR-048 §Decision 4 added; Stories row extended. Amendment section appended. |
| 1.29 | 2026-08-30 | product-owner | Consistency-audit finding 7 closure (MINOR): §VP Anchors VP-079 staleness flag for Event 8 (`plugin.indeterminate`) closed — VP-079 v1.22 covers Event 8 in its Property Statement (eight events), mandatory-fields table, and SITE_8. Sentinel sentence replaced with closure annotation per v1.26 Event 7 pattern (F-S2125-P2-003). |
| 1.28 | 2026-08-30 | product-owner | F2 validation-integrity-layer1 spec burst: Event 8 `plugin.indeterminate` catalog entry added — triggering-condition/semantics authority is BC-1.18.001; wire-format/field-shape catalog entry follows Event 7/BC-1.03.019 pattern (ADR-047 + BC-1.18.001/002/003/004). seven→eight count-phrase sweep throughout (§Description, §Common Fields, §Postconditions, §Sink destination, §Invariants 1+3, §Architecture Anchors). BC-1.18.001/002/003/004 added to §Related BCs. EC-010 (`plugin.indeterminate`) added. Canonical test vectors `indeterminate-fail-closed-fuel` and `indeterminate-fail-open-epoch` added. VP-102/103/104/105/106 added to §VP Anchors and §Verification Properties. ADR-047 added to §Traceability ADR row; S-25.01 added to §Traceability Stories row. H1 title updated per POLICY 7 (`plugin.indeterminate` appended). |
| 1.27 | 2026-08-21 | product-owner | D-1064 Wave-6 pass-6 remediation burst. **F-S2125-P6-001** (HIGH, POLICY 19 `adr_version_cite_volatile_pin_prohibition`, sibling-sweep of BC-1.03.019's same-burst fix): Traceability ADR row carried a load-bearing `ADR-039 v1.15 §Decision 5 Mitigation 1` version pin. Swept to the stable section-anchor form `ADR-039 §Decision 5 Mitigation 1 (E-006)` — the `(E-006)` parenthetical is informational only, naming the erratum that corrected the WARN-message text, not a version gate on the citation. **F-S2125-P6-002** (LOW): the live §VP Anchors VP-079 closure bullet ("Staleness flag CLOSED...") cited a bare `VP-079 v1.20`, which is now stale (VP-079 has advanced to v1.21) even though the bullet is a historical closure record rather than a live re-verification claim. Chose annotation over silent carry-forward (production-grade default: the bullet documents what was true AT closure time, 2026-08-20, so restating it as "v1.21" would misrepresent what was actually verified then) — appended `(VP-079 v1.20 at closure; now v1.21)` to the bullet so it reads accurately as a dated historical record without implying VP-079 v1.21 was independently checked. No §Postconditions/§Common Fields/§Description/VP-079 content itself changed. |
| 1.26 | 2026-08-20 | product-owner | F-S2125-P2-003 (MEDIUM, S-21.25 adversarial pass-2, `v1.0-brownfield-backfill`): closed the false VP-079-staleness flag at all 3 sites (§VP Anchors VP-079 bullet; Amendment 2026-08-20 v1.24→v1.25 changes-made item 11; Amendment 2026-08-20 v1.24→v1.25 standalone "VP-079 staleness flag" paragraph) — VP-079 v1.20 already registers Event 7 (`plugin.fuel_headroom_warning`) in its Property Statement, mandatory-fields table, and SITE_7; the architect follow-up the flag demanded was already done, the flag was simply never cleared. Sibling-site sweep (TD-VSDD-060): `emit_fuel_headroom_warning` → `emit_plugin_fuel_headroom_warning` at the 2 remaining occurrences in the v1.25 Amendment section (§Schema derivation, TD-031 verification line), matching BC-1.03.019 v1.1→v1.2's F-S2125-P2-002 emitter rename to the `emit_plugin_*` sibling convention. Does not touch VP-079 itself (already correct) or any §Postconditions/§Common Fields/§Description content — precision-only fix to stale meta-commentary. |
| 1.25 | 2026-08-20 | product-owner | F-S2125-P1-003 (MEDIUM, S-21.25 adversarial pass-1, `v1.0-brownfield-backfill`): Event 7 `plugin.fuel_headroom_warning` catalog entry added — dispatcher wire event for ADR-039 v1.15 §Decision 5 Mitigation 1 (fuel-headroom early-warning signal on `PluginResult::Ok`); full triggering-condition/semantics authority is BC-1.03.019 (PC1–PC10), this BC registers the wire-format/field-shape catalog entry only. Sibling parity: `plugin.fuel_headroom_warning` carries `timestamp` identically to Events 1/4/5/6, closing the gap the finding identified (BC-1.03.019's own PC6 previously omitted it). H1 title, §Description, §Common Fields (plugin_name/plugin_version presence rows extended to include Event 7), §Postconditions (Event 7 section added), §Sink destination paragraph, Invariants 1 and 3, §Architecture Anchors, §Story Anchor, §VP Anchors (VP-079/VP-028 count-phrase + enumeration updated; VP-079 staleness flag raised for architect routing per the v1.18 precedent; VP-TBD bullet added per BC-1.03.019), EC-009, Canonical Test Vectors (`fuel-headroom-warning` row), §Verification Properties (VP-079 row text), §Related BCs (BC-1.03.019 cross-reference), §Traceability (CAJ, DI, Architecture Module, ADR, Stories, Cycle rows) all updated — six→seven event-type count-phrase sweep performed throughout per the file's own established convention (cf. v1.18/F-P5-003's four→six sweep on the prior Event-6 addition). |
| 1.24 | 2026-07-15 | product-owner | F-P8-001 (LOW, S-19.09 D22): Event 6 wire format and mandatory-fields enumeration amended to include `timestamp` field (ISO-8601 alias of `ts`, byte-consistent with Events 1–5 sibling form). SDK grounding: `emit_plugin_completed_async` in `crates/factory-dispatcher/src/host/emit_event.rs` chains `.with_field("timestamp", ts.as_str())`. |
| 1.23 | 2026-07-13 | product-owner | S-19.05 pass-13 fix-burst discovery: frontmatter status: draft / lifecycle_status: active mismatch — missed POL-14 auto-promotion. Verification: S-15.01 (behavioral_contracts includes BC-3.08.001) carries status=merged, merged_at=2026-05-08, merged_in=PR-106, merge_sha=453eee1; POL-14 mandates draft→active on PR merge; lifecycle_status was already active. |
| 1.22 | 2026-07-13 | product-owner | S-19.05 pass-13 F-P13-001: five stale count phrases corrected — §Common Fields intro "All five event types" → "All six event types"; session_id row "all five event types" → "all six event types"; §Architecture Anchors FileSink row "all five event types" → "all six event types"; §Traceability CAJ "these four event types" → "these six event types"; §Traceability DI-017 "all four event types" → "all six event types". §Traceability ADR row disambiguated: four-count scoped to original ADR-019 events (Events 1–4); Events 5–6 provenance noted. Whole-file count-phrase sweep conducted per F-P13-001 method; all remaining hits classified. |
| v1.21 | 2026-07-10 | product-owner | F-P43-003: §VP VP-100 row verbatim-derived from VP-INDEX SoT (cardinality+mutual-exclusivity form; replaces latency-paraphrase). F-P43-005: v1.19 Changelog row backfilled; Amendment 2026-07-09 (v1.19→v1.20) prose section authored for structural parity. O-P43-001: last_amended canonicalized to chain form. |
| v1.20 | 2026-07-09 | product-owner | orchestrator pre-pass-43 consistency sweep: (a) §Verification Properties VP-100 row added (missing-row; integration S-19.05; Invariant 6; DI-019); §VP Anchors VP-100 bullet added. (b) §VP Anchors VP-028 stale-count fixed — "all four event types" → "all six event types" with enumeration of all six async-semantics event types. |
| v1.19 | 2026-07-07 | product-owner | F-P7-007: `entry_index` semantics clarified as schema-level defense, not runtime dispatch gate. Event 5 `entry_index` semantics paragraph extended with schema-level-defense note (concurrent-same-`plugin_name` scenario has no production dispatch path; correctness verified by property/serialization tests). Event 6 `entry_index` semantics final sentence added mirroring schema-level defense. Invariant 6 schema-level predicate note added. Closes F-P7-007. |
| v1.18 | 2026-07-07 | product-owner | E-19 pass-5 PO fix burst (F-P5-003): Event 6 `plugin.completed` (async path) added to catalog — async-path variant for async plugins that complete within the drain window. Wire schema mirrors sync-path `emit_lifecycle` in `crates/factory-dispatcher/src/executor.rs` (`trace_id`, `session_id`, `plugin_name`, `plugin_version`, `elapsed_ms`, `fuel_consumed`, `exit_code`, `stderr` conditional) plus `entry_index: u32` for Invariant 6 tuple parity with Event 5. Invariant 6 updated to state mutual exclusivity: abandoned↔completed are terminal alternatives for any (trace_id, plugin_name, entry_index) tuple. Invariants 1 and 3 count corrected four→six. §Common Fields updated: plugin_name present on events (1,4,5,6); plugin_version note split — Events 1/4/5 do NOT emit plugin_version (original emit_event.rs fns); Event 6 DOES (mirrors executor.rs emit_lifecycle). §Description, §Sink destination, §EC-008, §async-completed test vector, §VP Anchors, §Verification Properties, §Architecture Anchors, §Traceability Stories updated. BC-INDEX v3.70→v3.71. |
| v1.17 | 2026-07-06 | product-owner | E-19 pass-3 PO finalization (F-P3-013): `entry_index` semantics paragraph in Event 5 corrected — derivation rule stated explicitly: `plugin_name` in `plugin.abandoned` events = registry entry `name` field verbatim. Example replaced: the two production `verify-factory-lock` entries carry DIFFERENT `name` values (`verify-factory-lock` vs `verify-factory-lock-bash`) so they do NOT constitute a name-duplication example. Paragraph now cites the schema-level invariant: registry does NOT enforce `name` uniqueness, so future registries may have duplicate `name` entries that make name-only keying ambiguous — `entry_index` is the disambiguation mechanism. Invariant 6 terminal-semantics key (`trace_id + plugin_name + entry_index`) UNCHANGED. |
| v1.16 | 2026-07-06 | product-owner | F-P2-008 fix burst (product-owner): Event 5 `plugin.abandoned` schema extended with mandatory field `entry_index: u32` (ordinal position of registry entry in async partition at spawn time; enumerate() order; disambiguates multiple entries per plugin_name — e.g., verify-factory-lock has 2 entries). Mandatory fields list updated. Invariant 6 terminal-semantics key extended: `trace_id+plugin_name` → `trace_id+plugin_name+entry_index`. EC-007 and abandoned-one test vector updated. Closes F-P2-008. BC-INDEX v3.60→v3.61. |
| v1.15 | 2026-07-06 | product-owner | F-P1-013: Event 5 `plugin.abandoned` added to catalog; drain-terminal semantics codified (option a); Invariant 6; EC-007; abandoned-none/one test vectors; VP-079 scope updated; §Architecture Anchors updated. Closes F-P1-013. |
| v1.14 | 2026-05-09 | state-manager | F-P36-001: Traceability Stories row updated TBD → S-15.01 (F3 story decomposition propagation). |
| v1.13 | 2026-05-09 | implementer | F-P25-003: §Architecture Anchors bullet 1 corrected to main.rs + host/emit_event.rs (emission sites); §Traceability Architecture Module row corrected. F-P25-006: duplicate last_amended frontmatter field removed. |
| v1.12 | 2026-05-08 | state-manager | F-P23-002 cross-subsystem sweep: HOST_ABI.md line cite migrated to stable §`emit_event` section anchor per TD-VSDD-091. |

---

## Amendment 2026-07-07 (v1.17 → v1.18 — F-P5-003: `plugin.completed` (async path) Event 6 catalog entry added)

**Driver:** Adversary finding F-P5-003 (E-19 pass-5) — S-19.05 emits a new `plugin.completed` event for async plugins that complete within the drain window, but BC-3.08.001 v1.17 catalogues no such event. The story's AC-001 BC Trace cited "BC-3.08.001 v1.17 (Event 3 schema)" — an error on two counts: Event 3 is `dispatcher.registry_invalid`, and `plugin.completed` was absent from the catalog entirely. The story's Architecture Mapping stated "plugin.completed already exists in the sync path; confirm async variant uses same schema" without citing an owning BC. Ground-truth search (`grep -rl "plugin.completed" .factory/specs/behavioral-contracts/`) found only BC-3.02.008 (test fixture references) and BC-3.08.001 (Invariant 6 reference) — neither owned the wire schema for the async variant.

**Schema derivation:** Field set derived from `crates/factory-dispatcher/src/executor.rs::emit_lifecycle` for `PluginResult::Ok { exit_code, elapsed_ms, fuel_consumed, stderr }`: `trace_id`, `session_id`, `plugin_name`, `plugin_version`, `elapsed_ms`, `fuel_consumed`, `exit_code`, `stderr` (conditional on non-empty). `entry_index: u32` added for Invariant 6 tuple parity: the abandoned↔completed mutual-exclusion check requires the same `(trace_id, plugin_name, entry_index)` key on both events.

**Option chosen:** Option (a) — amend BC-3.08.001 v1.17→v1.18 to add `plugin.completed` (async path) as Event 6. No other existing BC owns this wire schema; BC-3.08.001 is the SS-03 catalog authority for async-semantics events; Event 6 is the natural complement to Event 5 under Invariant 6.

**Changes made:**

1. **Frontmatter** (F-P5-003): `version` bumped `"1.17"` → `"1.18"`; `last_amended` updated to 2026-07-07; `modified[]` entry added.
2. **H1 title** (F-P5-003): `, \`plugin.completed\` (async path)` appended to event list. (POLICY 7: H1 updated; BC-INDEX row mirrors H1.)
3. **§Description** (F-P5-003): `plugin.completed` (async path) cite added for F-P5-003 fix burst; "five" → "six" in catalog-entry count.
4. **§Common Fields** (F-P5-003): `plugin_name` row updated — present on events (1, 4, 5, and 6). Closing paragraph split: Events 1/4/5 do NOT emit `plugin_version` (original emit_event.rs fns); Event 6 DOES (mirrors executor.rs emit_lifecycle). "All five" → "All six" in trace_id+session_id line.
5. **§Sink destination paragraph** (F-P5-003): "All five events" → "All six events".
6. **Event 6 postcondition section added** (F-P5-003): trigger, wire format (JSON), mandatory fields list, `entry_index` semantics, Invariant 6 interplay note.
7. **Invariant 1** (backfill — stale since v1.15): "all four event types" → "all six event types".
8. **Invariant 3** (backfill — stale since v1.15): "All four" → "All six"; enumeration extended to include `plugin.abandoned` and `plugin.completed` (async path).
9. **Invariant 6** (F-P5-003): Updated to explicitly state mutual exclusivity in both directions: `plugin.abandoned` and `plugin.completed` (async path) are terminal alternatives; reference to Event 6 added.
10. **EC-008** (F-P5-003): Edge case for async plugin completing within drain window.
11. **§Canonical Test Vectors — `async-completed` row** (F-P5-003): Test vector for async plugin exit 0 within drain window.
12. **§VP Anchors** (F-P5-003): VP-079 scope updated to "all six event types including `plugin.abandoned` and `plugin.completed` (async path)".
13. **§Verification Properties table** (F-P5-003): VP-079 row scope updated.
14. **§Architecture Anchors** (F-P5-003): First bullet extended with `plugin.completed` (async path) emission path anchor.
15. **§Traceability Stories** (F-P5-003): S-19.05 added as anchor story for Event 6.

**POLICY 1 verification:** All prior content preserved verbatim except the changes listed above. No event IDs renumbered. Events 1–5 wire-format examples unchanged.
**POLICY 7 verification:** H1 heading updated to include `plugin.completed` (async path); BC-INDEX row mirrors H1 (BC-INDEX v3.71).
**TD-031 verification:** No `executor.rs:[0-9]+` or `main.rs:[0-9]+` line-number citations introduced; stable function/section anchors used throughout.

**VP-079 staleness flag (architect routing required):** VP-079 currently enumerates "five event types" and its mandatory-fields table does not include `plugin.completed` (async path). Adding Event 6 makes VP-079 stale on: (a) event-count prose ("five" → "six"); (b) mandatory-fields table (missing Event 6 row); (c) production caller-site list (missing the async drain result arm). Orchestrator should route VP-079 amendment to `vsdd-factory:architect`.

**VP-100 cite staleness flag (architect routing required):** VP-100 `source_bc:` field cites "BC-3.08.001 v1.16 Invariant 6" and `input-hash: "1072e05"` will be stale once this amendment is persisted. The conceptual correctness of VP-100 is unaffected (Invariant 6 strengthened, not changed); only the version pin and input-hash need updating. Orchestrator should route VP-100 frontmatter update to `vsdd-factory:architect`.

---

## Amendment 2026-07-07 (v1.18 → v1.19 — F-P7-007: `entry_index` clarified as schema-level defense, not runtime dispatch gate)

**Driver:** Adversary finding F-P7-007 (E-19 pass-7) — the `entry_index` semantics paragraphs in Event 5 and Event 6, and Invariant 6, described `entry_index` disambiguation for the concurrent-same-`plugin_name` scenario without clarifying that this scenario has no production dispatch path and that `entry_index` correctness is a schema-level property verified by serialization/property tests, not a runtime concurrent-dispatch fixture. S-19.05's AC-002 concurrent clause implied a runtime test fixture path that does not exist.

**Ruling: schema-level defense.** The dispatcher's `registry.rs` enforces `(name, event, tool)` tuple uniqueness (E-REG-003). Two same-named entries require different `event` or `tool` values; co-occurring in a single async partition dispatch requires both to match the same event+tool invocation simultaneously — no production registry entry pair does this. `entry_index` is the natural 0-based ordinal from `enumerate()` of the async partition slice, emitted by the spawn loop without any additional runtime gate. Its correctness as a consumer disambiguation key is a schema-level invariant: the only thing that needs verifying is that the ordinal is correctly marshalled from the spawn loop's `enumerate()` into the event payload — a property test over the struct. No runtime concurrent-dispatch fixture exercises two same-named entries because that scenario is not reachable in any registered production plugin pair.

**S-19.05 AC-002 concurrent clause ruling for story-writer:** Replace any runtime concurrent-dispatch fixture for same-`plugin_name` disambiguation with: "Property test over the `plugin.completed` and `plugin.abandoned` event structs asserting (a) `entry_index` field type is `u32`, (b) the ordinal value in the emitted event equals the `enumerate()` index of the entry in the async partition slice, verified by constructing a synthetic partition of N entries and asserting each emitted event's `entry_index` matches its position."

**Changes made:**

1. **Frontmatter** (F-P7-007): `version` bumped `"1.18"` → `"1.19"`; `last_amended` updated to 2026-07-07; `modified[]` entry added.
2. **Event 5 `entry_index` semantics paragraph** (F-P7-007): Added explicit schema-level-defense paragraph after the existing parenthetical — states the concurrent-same-`plugin_name` scenario has no production dispatch path; `entry_index` is a schema-level defense verified by serialization/property tests, not a runtime concurrent-dispatch fixture; runtime path covered by normal integration tests.
3. **Event 6 `entry_index` semantics paragraph** (F-P7-007): Added final sentence mirroring the schema-level defense principle for `plugin.completed`.
4. **Invariant 6** (F-P7-007): Added schema-level note at end — the `(trace_id, plugin_name, entry_index)` mutual-exclusivity key is a schema-level predicate verifiable by property/serialization tests, not a runtime concurrent-dispatch fixture.

**POLICY 1 verification:** All prior content preserved verbatim except the four additions above. No event IDs renumbered. No wire-format examples changed.
**POLICY 7 verification:** H1 heading unchanged (no new event type; this is a semantics clarification).
**TD-031 verification:** No `registry.rs:[0-9]+` line-number citations introduced; stable function/symbol anchors used (`registry.rs` validation function, `(name, event, tool)` uniqueness invariant, E-REG-003 error code).

---

## Amendment 2026-07-09 (v1.19 → v1.20 — D-798 pre-pass-43 consistency sweep)

**Driver:** Orchestrator pre-pass-43 consistency sweep (D-798) — two defects found and fixed.

**(a) §Verification Properties VP-100 row missing (D-798):** BC-3.08.001 v1.19 had no VP-100 row in the §Verification Properties table. VP-100 was created in the E-19 VP package (architect, pass-42 sibling sweep) covering BC-3.08.001 §Invariant 6 drain-timer expiry. The §VP Anchors bullet for VP-100 was present (added in the same D-798 sweep) but the §Verification Properties table row was absent, creating inconsistency between the two sections.

**(b) §VP Anchors VP-028 stale count (D-798):** The VP-028 bullet in §VP Anchors still referenced "all four event types" — stale since v1.15 when Event 5 (`plugin.abandoned`) and v1.18 when Event 6 (`plugin.completed` async path) were added. Updated to "all six event types" with explicit enumeration of all six: `plugin.async_block_discarded`, `dispatcher.schema_mismatch`, `dispatcher.registry_invalid`, `plugin.timeout` (async path), `plugin.abandoned`, `plugin.completed` (async path).

**Changes made:**

1. **§Verification Properties table** (D-798): VP-100 row added — property: drain-timer expiry guarantees plugin.abandoned emission (Invariant 6; DI-019); proof-method: integration (S-19.05).
2. **§VP Anchors** (D-798): VP-100 bullet added — Drain-Timer Expiry Emits Exactly One plugin.abandoned Per In-Flight (plugin_name, entry_index); No plugin.completed Follows for Same Trace (integration; S-19.05; Invariant 6; DI-019). VP-028 bullet stale-count updated: "all four event types" → enumeration of all six async-semantics event types.
3. **Frontmatter** (D-798): `version: "1.19"` → `"1.20"`; `last_amended: 2026-07-09`; `modified[]` entry added.

**POLICY 1 verification:** All prior content preserved verbatim except the changes listed above.
**POLICY 7 verification:** H1 heading unchanged.
**TD-031 verification:** No new line-number citations introduced.

---

## Amendment 2026-07-10 (v1.20 → v1.21 — F-P43-003: VP-100 row verbatim-derived; F-P43-005: v1.19 changelog backfill + v1.20 amendment section; O-P43-001: last_amended chain form)

**Driver:** Adversarial findings F-P43-003 (MEDIUM), F-P43-005 (MEDIUM), and O-P43-001 (LOW, fix-in-scope) from E-19 pass-43.

**F-P43-003 — §Verification Properties VP-100 row paraphrase drift:** The VP-100 row in §Verification Properties described a latency property ("guarantees plugin.abandoned emission within configured deadline_ms") rather than the cardinality+mutual-exclusivity property defined in VP-INDEX v2.56 (SoT) and VP-100.md H1. VP-INDEX row (line 531) and VP-100.md `title:` field both define VP-100 as: "Drain-Timer Expiry Emits Exactly One plugin.abandoned Per In-Flight (plugin_name, entry_index); No plugin.completed Follows for Same Trace." The §VP Anchors bullet (added by D-798) already used the correct exactly-one form — only the §Verification Properties table row carried the paraphrase.

**F-P43-005 — Changelog leg-2 gap:** The body Changelog table skipped v1.19 (the `modified[]` frontmatter array included it; a dedicated `## Amendment 2026-07-07 (v1.18 → v1.19 — F-P7-007...)` prose section exists at end-of-file). The v1.19 Changelog row has been backfilled between v1.20 and v1.18. Additionally, no `## Amendment 2026-07-09 (v1.19 → v1.20 — D-798...)` prose section existed, creating structural asymmetry with v1.15..v1.19 which each have both a Changelog row AND a prose section. The v1.19→v1.20 prose section has been authored above, sourced from the D-798 Changelog row.

**O-P43-001 — last_amended chain form:** Frontmatter `last_amended: 2026-07-09` used a bare date — the only E-19 BC not using the `"(vN.M) — description [Prior: ...]"` chain form, making POLICY 14 leg-4 unverifiable. Canonicalized to standard chain form (production-grade default per CANONICAL PRINCIPLE).

**Changes made:**

1. **Frontmatter** (O-P43-001 + POLICY 14): `last_amended` canonicalized from bare `2026-07-09` to `"2026-07-10 (v1.21) — ..."` chain form. `version: "1.20"` → `"1.21"`. `modified[]` entry `"2026-07-10 (v1.21)"` added.
2. **§Verification Properties VP-100 row** (F-P43-003): Property cell replaced — paraphrase ("guarantees plugin.abandoned emission within configured deadline_ms") replaced with verbatim-derived cardinality+mutual-exclusivity form sourced from VP-INDEX v2.56 SoT and VP-100.md H1: "Drain-timer expiry emits exactly one plugin.abandoned per in-flight (plugin_name, entry_index); no plugin.completed follows for same trace_id+plugin_name+entry_index (Invariant 6; DI-019)". Proof method `integration (S-19.05)` unchanged.
3. **Changelog table** (F-P43-005): v1.21 row added at top; v1.19 row backfilled between v1.20 and v1.18 rows.
4. **Amendment 2026-07-09 (v1.19 → v1.20) section** (F-P43-005): Prose section added for structural parity with v1.15..v1.19.

**POLICY 1 verification:** All prior content preserved verbatim except the changes listed above.
**POLICY 7 verification:** H1 heading unchanged (no new event type; this is a spec-consistency fix).
**TD-031 verification:** No new line-number citations introduced.

---

## Amendment 2026-07-15 (v1.23 → v1.24 — F-P8-001: Event 6 wire format and mandatory fields updated to include `timestamp`)

**Driver:** Adversary finding F-P8-001 (LOW, S-19.09 D22) — Event 6 (`plugin.completed` async path) was the only event in BC-3.08.001 whose `timestamp` field was absent from both the wire-format JSON example and the mandatory-fields enumeration. Events 1–5 all document `timestamp` as a mandatory ISO-8601 field (alias of the internal `ts` wire field, set via `.with_field("timestamp", ts.as_str())`). The implementation (`emit_plugin_completed_async` in `crates/factory-dispatcher/src/host/emit_event.rs`) already emits `timestamp` — the post-D22 (S-19.09) fix burst added `.with_field("timestamp", ts.as_str())` to align with sibling emitters. The spec omission was a documentation gap; the wire reality was correct.

**SDK grounding (POLICY 5):** Literal grep capture from the S-19.09 worktree (`/Users/zious/Documents/GITHUB/vsdd-factory/.worktrees/S-19.09/crates/factory-dispatcher/src/host/emit_event.rs`), stable function anchor `emit_plugin_completed_async`:

```
.with_field("timestamp", ts.as_str())
```

Call chain: `emit_plugin_completed_async` captures `let ts = ev.ts.clone()` before moving `ev` into the builder chain, then chains `.with_field("timestamp", ts.as_str())` as the third call (after `.with_trace_id` and `.with_session_id`), mirroring all sibling emitters in the same file (`emit_plugin_async_block_discarded`, `emit_dispatcher_schema_mismatch`, `emit_dispatcher_registry_invalid`, `emit_plugin_timeout_async`, `emit_plugin_abandoned`).

**Changes made:**

1. **Frontmatter** (POLICY 14 legs 1–4): `version: "1.23"` → `"1.24"`; `last_amended` chain form prepended with v1.24 entry; `modified[]` entry `"2026-07-15 (v1.24)"` added.
2. **Event 6 wire-format JSON example**: `"timestamp": "<ISO-8601>"` field added after `"fuel_consumed"`, consistent with Event 4 and Event 5 placement (timestamp as final non-conditional field before closing brace).
3. **Event 6 mandatory fields**: `timestamp` appended to the mandatory-fields enumeration.
4. **Changelog table**: v1.24 row added at top with F-P8-001 citation and SDK grounding reference.

**POLICY 1 verification:** All prior content preserved verbatim except the four changes listed above. No event IDs renumbered. Events 1–5 wire-format examples unchanged.
**POLICY 7 verification:** H1 heading unchanged (no new event type; this is a documentation parity fix).
**TD-031 verification:** No `emit_event.rs:[0-9]+` line-number citations introduced; stable function anchor (`emit_plugin_completed_async`) used throughout.

---

## Amendment 2026-08-20 (v1.24 → v1.25 — F-S2125-P1-003: Event 7 `plugin.fuel_headroom_warning` catalog entry added)

**Driver:** S-21.25 adversarial pass-1 finding F-S2125-P1-003 (MEDIUM), cycle `v1.0-brownfield-backfill`. The new `plugin.fuel_headroom_warning` dispatcher wire event, defined by BC-1.03.019 (fuel-headroom WARN early-warning signal per ADR-039 §Decision 5 Mitigation 1), was not registered in this BC's SS-03 event catalog — the authority BC-3.08.001 is for every dispatcher-emitted `plugin.*`/`dispatcher.*` wire event. The finding also noted that BC-1.03.019's own PC6 field enumeration omitted the `timestamp` common field every sibling `plugin.*` emitter carries (`emit_plugin_timeout_async`, `emit_plugin_abandoned`, `emit_plugin_completed_async` in `crates/factory-dispatcher/src/host/emit_event.rs`) — the exact gap S-19.09 T-013/F-WG-003 (this BC's own v1.24 amendment, immediately above) fixed on `emit_plugin_completed_async`. The adversary's preferred resolution was sibling parity (add `timestamp`) plus registration in this catalog, over a documented exemption; both are performed in this amendment and in BC-1.03.019's companion v1.0→v1.1 fix burst.

**Schema derivation:** Field set derived from BC-1.03.019 PC6 (as corrected in the same burst): `plugin_name`, `fuel_consumed`, `fuel_cap`, `headroom_ratio`, `level`, `message`, `timestamp`, plus the standard envelope fields (`type`, `trace_id`, `session_id`, `schema_version`). No `plugin_version` — mirrors Events 1/4/5 (the original `emit_event.rs` functions do not call `with_plugin_version()`); Event 7's emitter (`emit_plugin_fuel_headroom_warning`, proposed per BC-1.03.019 §Architecture Anchors, renamed from an earlier `emit_fuel_headroom_warning` draft at BC-1.03.019 v1.2 to match the `emit_plugin_*` sibling convention) follows the same convention, not Event 6's `emit_lifecycle`-mirroring exception.

**Option chosen:** Option (a) — amend BC-3.08.001 v1.24→v1.25 to add `plugin.fuel_headroom_warning` as Event 7. No other existing BC owns this wire schema; BC-3.08.001 is the SS-03 catalog authority for dispatcher-emitted events; BC-1.03.019 remains the triggering-condition/semantics authority (threshold predicate, boundary controls, `headroom_ratio` formula, independence from `on_error`/`failure_policy`) and is cross-referenced rather than duplicated.

**Changes made:**

1. **Frontmatter** (POLICY 14 legs 1–4): `version: "1.24"` → `"1.25"`; `last_amended` chain form prepended with v1.25 entry; `modified[]` entry `"2026-08-20 (v1.25)"` added.
2. **H1 title** (F-S2125-P1-003): `, \`plugin.fuel_headroom_warning\`` appended to event list. (POLICY 7: H1 updated; BC-INDEX row must mirror H1 — routed to state-manager/index-crossref, out of this BC's own edit scope.)
3. **§Description**: `plugin.fuel_headroom_warning`/S-21.25/F-S2125-P1-003 cite added; "six" → "seven" in catalog-entry count.
4. **§Common Fields**: `session_id` row, intro sentence, and closing bullets — "all six" → "all seven"; `plugin_name` presence row extended to "(1, 4, 5, 6, and 7)"; `plugin_version`-absence bullet extended to "(1, 4, 5, and 7)" since Event 7 does not emit `plugin_version` (mirrors Events 1/4/5, not Event 6).
5. **§Sink destination paragraph**: "All six events" → "All seven events".
6. **Event 7 postcondition section added**: trigger, wire format (JSON), mandatory fields list, `level`/`message` semantics, `headroom_ratio` semantics, block-decision-independence note — each cross-referencing BC-1.03.019 as the full behavioral authority rather than duplicating its ten postconditions.
7. **Invariant 1**: "all six event types" → "all seven event types"; "These six events" → "These seven events".
8. **Invariant 3**: "All six" → "All seven"; enumeration extended to include `plugin.fuel_headroom_warning`.
9. **EC-009 added**: Edge case for the `>90%` fuel-headroom trigger, cross-referencing BC-1.03.019's EC-001–EC-007 for full boundary/negative-control detail.
10. **§Canonical Test Vectors — `fuel-headroom-warning` row added**: Test vector for `PluginResult::Ok` at 92.5% fuel consumption.
11. **§VP Anchors**: VP-079 bullet — "all six event types" → "all seven event types", enumeration extended, staleness flag raised at v1.25 authoring time (architect routing required, same pattern as the v1.18/F-P5-003 Event-6 precedent); **CLOSED at v1.26 (F-S2125-P2-003)** — VP-079 v1.20 already registers Event 7, so the flag was stale-and-uncleared, not a genuine open gap. VP-028 bullet — "all six event types" → "all seven event types", enumeration extended. VP-TBD bullet added, cross-referencing BC-1.03.019's own VP-TBD placeholder (architect to assign a real VP-NNN for the triggering-condition properties — this VP-TBD flag is unrelated to the VP-079 staleness flag and remains open).
12. **§Verification Properties table**: VP-079 row scope text updated — "all six event types" → "all seven event types", enumeration extended. No new VP-NNN row added (Event 7's triggering-condition VP remains BC-1.03.019's VP-TBD, an architect-owned assignment, not duplicated here).
13. **§Related BCs**: BC-1.03.019 cross-reference bullet added, stating the wire-format-vs-triggering-condition authority split explicitly.
14. **§Architecture Anchors**: New bullet for `crates/factory-dispatcher/src/invoke.rs::invoke_plugin` (Event 7 emission site); FileSink bullet "all six" → "all seven".
15. **§Story Anchor**: S-21.25 cite added for Event 7.
16. **§Traceability**: CAJ, L2 Domain Invariants, Architecture Module, ADR, Stories, and Cycle rows all updated to reflect Event 7 / S-21.25 / ADR-039 v1.15 / `v1.0-brownfield-backfill`.
17. **Changelog table**: v1.25 row added at top with F-S2125-P1-003 citation.

**POLICY 1 verification:** All prior content preserved verbatim except the seventeen changes listed above. No event IDs renumbered (Events 1–6 unchanged; Event 7 is a genuinely new addition, not a renumbering). Events 1–6 wire-format examples unchanged.
**POLICY 7 verification:** H1 heading updated to include `plugin.fuel_headroom_warning`; BC-INDEX row must mirror H1 in the same burst per this file's own POLICY 7 convention — routed to state-manager/index-crossref (out of product-owner's edit scope for this task).
**TD-031 verification:** No `invoke.rs:[0-9]+` or `emit_event.rs:[0-9]+` line-number citations introduced; stable function anchors (`invoke_plugin`, `emit_plugin_fuel_headroom_warning`) used throughout.

**VP-079 staleness flag — CLOSED (F-S2125-P2-003, pass-2 fix burst, 2026-08-20):** This flag, raised at v1.25 authoring time, asserted VP-079 was stale on event-count prose, mandatory-fields table, and production caller-site list for Event 7. That assertion is now FALSE: VP-079 v1.20 already registers Event 7 (`plugin.fuel_headroom_warning`) in its Property Statement (seven events), mandatory-fields table, and SITE_7 (whose mutation-counter-proof grep predicate searches the emitter name directly). The architect follow-up this flag demanded was already completed; the flag was simply never cleared here. Retained as a closure record only — no further architect routing needed for this item.

---

## Amendment 2026-08-30 (v1.27 → v1.28 — Event 8 `plugin.indeterminate`: INDETERMINATE outcome wire-format catalog entry)

**Driver:** F2 validation-integrity-layer1 spec burst (product-owner, 2026-08-30). BC-1.18.001 introduces the INDETERMINATE outcome class for fail-closed WASM plugins that cannot complete (fuel exhaustion, epoch timeout, or OutputTooLarge). BC-1.18.004 defines the advisory-only fail-open path. Both paths emit a `plugin.indeterminate` event — the triggering-condition/semantics authority is BC-1.18.001; SS-03 catalog authority for the wire-format/field-shape is this BC. Pattern follows Event 7/BC-1.03.019 exactly.

**Changes made:**

1. **Frontmatter** (v1.28): `version` bumped `"1.27"` → `"1.28"`; `last_amended` updated; `modified[]` entry `"2026-08-30 (v1.28)"` added.
2. **H1 title**: `, \`plugin.indeterminate\`` appended to the event list.
3. **§Description**: "seven" → "eight" throughout; Event 8 provenance sentence added (ADR-047 + BC-1.18.001 authority); BC-1.18.001 added to referenced-BCs list; "all seven" → "all eight" in the catalog sentence.
4. **§Common Fields**: "all seven" → "all eight"; `session_id` row "all seven" → "all eight"; `plugin_name` row presence list extended to "(1, 4, 5, 6, 7, and 8)"; closing bullets updated — "(1, 4, 5, and 7)" → "(1, 4, 5, 7, and 8)" for plugin_version-absence; "all seven" → "all eight" in both closing sentences.
5. **Event 8 `plugin.indeterminate` section added** in §Postconditions (after Event 7): trigger, wire format (JSON), mandatory fields, `cause` semantics, `failure_policy` semantics, `artifact_path` semantics, block-decision note, `plugin.timeout` co-emission note.
6. **§Postconditions sink destination paragraph**: "All seven events" → "All eight events".
7. **§Invariants 1 and 3**: "all seven" / "These seven" / "All seven" → eight; Invariant 3 enumeration extended to include `plugin.indeterminate`; note added that block (for fail-closed INDETERMINATE) comes from BC-1.18.002 marker-gate, not the event itself.
8. **§Related BCs**: BC-1.18.001, BC-1.18.002, BC-1.18.003, BC-1.18.004 bullets added.
9. **§Architecture Anchors**: New bullet for `executor.rs` + `indeterminate_marker.rs` (Event 8 emission site); FileSink bullet "all seven" → "all eight".
10. **§Edge Cases**: EC-010 added for `plugin.indeterminate`.
11. **§Canonical Test Vectors**: Two rows added: `indeterminate-fail-closed-fuel` and `indeterminate-fail-open-epoch`.
12. **§VP Anchors**: VP-079 scope updated to "all eight event types including `plugin.indeterminate`"; VP-028 enumeration extended; VP-102/103/104/105/106 bullets added.
13. **§Verification Properties table**: VP-079 row scope updated; VP-102/103/104/105/106 rows added.
14. **§Traceability ADR row**: ADR-047 §Decision 1 + §Decision 2 cite added for Event 8.
15. **§Traceability Stories row**: S-25.01 added.
16. **§Traceability Architecture Module row**: `executor.rs` + `indeterminate_marker.rs` added for Event 8.

**POLICY 1 verification:** All prior content preserved verbatim except the sixteen changes listed above. No event IDs renumbered (Events 1–7 unchanged; Event 8 is a genuinely new addition).
**POLICY 7 verification:** H1 heading updated to include `plugin.indeterminate`; BC-INDEX row must mirror H1 in the same burst — routed to state-manager/index-crossref per standard convention.
**TD-031 verification:** No line-number citations introduced; stable function anchors (`classify_outcome`, `emit_plugin_indeterminate`, `delete_marker_if_pass`) used throughout.

**VP-TBD assignment flag (architect routing required):** BC-1.03.019's VP-TBD placeholder (fuel-headroom warning triggering-condition properties: threshold predicate PC1–PC3, non-`Ok`-outcome exclusion PC4, uniform sub-shape coverage PC5, required-fields shape PC6–PC8, independence PC9, exactly-once semantics PC10) needs a real VP-NNN assignment and propagation to VP-INDEX/verification-architecture.md/verification-coverage-matrix.md per `vp_index_is_vp_catalog_source_of_truth`. This BC's own VP-TBD bullet (§VP Anchors) mirrors that placeholder for cross-reference purposes only — it is not a duplicate VP, it is a pointer to BC-1.03.019's pending assignment.

---

## Amendment 2026-08-30 (v1.28 → v1.29 — Finding 7: VP-079 Event 8 staleness flag closed)

**Driver:** Consistency-audit finding 7 (MINOR, 2026-08-30). The §VP Anchors VP-079 bullet added in v1.28 (Event 8 addition burst) ended with an open action-item flag: "VP-079 scope must be extended to cover Event 8 (`plugin.indeterminate`) in a follow-up architect pass per `vp_index_is_vp_catalog_source_of_truth`." This was a resolved false-positive: VP-079 v1.22 covers Event 8 in its Property Statement (eight events), mandatory-fields table, and SITE_8 — the architect-owned amendment was performed in the same burst as Event 8's addition. The flag was never cleared, leaving a stale open action item.

**Pattern:** Identical to v1.26 (F-S2125-P2-003) which closed the analogous Event 7 staleness flag at the same bullet location.

**Changes made:**

1. **Frontmatter** (POLICY 14 legs 1–4): `version: "1.28"` → `"1.29"`; `last_amended` chain form prepended with v1.29 entry; `modified[]` entry `"2026-08-30 (v1.29)"` added.
2. **§VP Anchors VP-079 bullet** (Finding 7): Final sentence replaced — open flag "VP-079 scope must be extended to cover Event 8 (`plugin.indeterminate`) in a follow-up architect pass per `vp_index_is_vp_catalog_source_of_truth`." replaced with closure annotation "**Staleness flag CLOSED (same-burst 2026-08-30):** VP-079 v1.22 covers Event 8 (`plugin.indeterminate`) in its Property Statement (eight events), mandatory-fields table, and SITE_8 — the architect-owned amendment was performed in the same burst as Event 8's addition; this flag is retained only as a closure record, not an open action item."
3. **Changelog table**: v1.29 row added at top (between column headers and prior v1.27 entry).

**POLICY 1 verification:** All prior content preserved verbatim except the §VP Anchors sentence replacement above. No event IDs renumbered. No wire-format examples changed. No invariants, edge cases, or test vectors changed.
**POLICY 7 verification:** H1 heading unchanged (no new event type; this is a meta-commentary staleness-flag closure).
**TD-031 verification:** No new line-number citations introduced.

## Amendment 2026-08-31 (v1.29 → v1.30 — ADR-048 §Decision 4: Event 9 `marker.cleared` catalog entry)

**Driver:** ADR-048 §Decision 4 (audited clear event + TTL-loudness). Every marker-clearance path must now emit an audited `marker.cleared` event so operators can correlate a clear with its originating `plugin.indeterminate` event via `trace_id`. Three clear modes: REVALIDATED (T1 re-validation via Edit/Write), TTL_EXPIRED (deadman TTL auto-delete), OPERATOR_OVERRIDE (retroactive RAW_DELETE_DETECTED reconciliation). Pattern follows Event 7 (BC-1.03.019 + BC-3.08.001) and Event 8 (BC-1.18.001 + BC-3.08.001): BC-1.18.003 is the triggering-condition/clear-path authority; this BC registers the wire-format/field-shape catalog entry.

**Changes made:**

1. **Frontmatter** (POLICY 14 legs 1–4): `version: "1.29"` → `"1.30"`; `last_amended` chain form prepended with v1.30 entry; `modified[]` entry `"2026-08-31 (v1.30)"` added.
2. **H1 title** (POLICY 7): `marker.cleared` appended to event list.
3. **§Description**: "all eight" → "all nine"; `marker.cleared` provenance sentence added (ADR-048 §Decision 4, clear-path/lifecycle authority = BC-1.18.003).
4. **§Common Fields** (3 occurrences): "All eight event types" → "All nine event types"; "all eight event types" → "all nine event types".
5. **§Sink destination paragraph**: "All eight" → "All nine".
6. **Invariant 1**: "all eight" → "all nine".
7. **Invariant 3**: "All eight" → "All nine"; `marker.cleared` added to the event enumeration list.
8. **Event 9 section added** (new `### Event 9: marker.cleared` under §Postconditions): full wire format with `clear_mode`/`actor_type` table, `trace_id` link semantics (matches originating `plugin.indeterminate` `trace_id`), `reason` obligation (mandatory for OPERATOR_OVERRIDE, null otherwise).
9. **§Related BCs**: BC-1.18.003 entry added as Event 9 clear-path authority.
10. **§Architecture Anchors**: `emit_marker_cleared` emission path (SS-01 `crates/factory-dispatcher/src/indeterminate_marker.rs`) + SS-04 plugin crate paths for TTL_EXPIRED and OPERATOR_OVERRIDE reconciliation added; FileSink row "all eight" → "all nine".
11. **§VP Anchors VP-079**: "eight" → "nine" in count phrase; `marker.cleared` added to enumeration; **Staleness flag OPEN** added: architect must propagate Event 9 to VP-079 SITE_9, mandatory-fields table (`clear_mode`, `actor_type`, `reason`), Property Statement ("nine events"), VP-INDEX, verification-architecture.md, verification-coverage-matrix.md under `vp_index_is_vp_catalog_source_of_truth`.
12. **§VP Anchors VP-028**: `marker.cleared` added to "all nine event types" list.
13. **§Verification Properties VP-079 row**: "all nine event types including … `marker.cleared`" and staleness flag added.
14. **§Traceability DI-017**: "all seven" → "all nine".
15. **§Traceability Architecture Module**: Event 9 emission sites added (SS-01 `emit_marker_cleared` + SS-04 plugin crate paths).
16. **§Traceability ADR**: ADR-048 §Decision 4 added as Event 9 provenance; Stories row extended with Events 8+9 combined scope.
17. **§Edge Cases**: EC-011 (REVALIDATED clear), EC-012 (TTL_EXPIRED), EC-013 (OPERATOR_OVERRIDE) added.
18. **§Canonical Test Vectors**: Three rows added: `marker-cleared-revalidated`, `marker-cleared-ttl-expired`, `marker-cleared-operator-override`.
19. **Changelog table**: v1.30 row added at top.

**POLICY 1 verification:** All prior content preserved verbatim except the count-phrase sweep items above (each is a targeted replacement). No event IDs renumbered. No wire-format examples for Events 1–8 changed.
**POLICY 7 verification:** H1 heading updated per POLICY 7 (new event type `marker.cleared` appended).
**TD-031 verification:** No new file:line-number citations introduced; emission paths identified by function name + source file only.
**VP citation change handoff:** VP citations changed in: BC-3.08.001 (VP-079 gains Event 9 staleness flag OPEN, VP-028 gains `marker.cleared`). Architect must propagate to VP-079 SITE_9, mandatory-fields table, Property Statement, VP-INDEX, verification-architecture.md, and verification-coverage-matrix.md under `vp_index_is_vp_catalog_source_of_truth` policy.

## Amendment 2026-08-31 (v1.30 → v1.31 — ADR-048 §Decision 4 v1.2 Emission-Point Correction: TTL_EXPIRED/OPERATOR_OVERRIDE emission moved dispatcher-native)

**Driver:** ADR-048 §Decision 4 v1.2 Emission-Point Correction (S-25.01 LOCAL adversary pass 2 F-P2-002 HIGH + F-P2-003 MED; human-ratified per orchestrator "proceed" directive). The v1.30 catalog entry attributed TTL_EXPIRED and OPERATOR_OVERRIDE `marker.cleared` emission to the WASM gate plugin's `evaluate_gate`. This is structurally impossible: the `emit_event` host ABI's RESERVED_FIELDS enrichment unconditionally overwrites `trace_id`/`plugin_name` with the CURRENT invoking plugin's own dispatch identity, never a foreign identity such as the marker's own `trace_id`/`plugin_name` that Event 9's wire contract (§Postconditions Event 9) requires. The architect's ADR-048 v1.2 fix moves TTL detection, auto-delete, and emission entirely into a new dispatcher-native pre-check (`check_and_clear_expired_marker`, `indeterminate_marker.rs`), called from `executor.rs`'s tier-execution loop before every Arm 1/Arm 2 (`on_error = "block_if_marker"`) plugin invocation; OPERATOR_OVERRIDE reconciliation is added to the same pre-check's marker-absent branch (`reconcile_raw_delete`). `evaluate_gate` is simplified to a pure marker-presence check with no TTL parsing, deletion, or emission logic of its own.

**Changes made:**

1. **Frontmatter** (POLICY 14 legs 1–4): `version: "1.30"` → `"1.31"`; `last_amended` chain form prepended with v1.31 entry; `modified[]` entry `"2026-08-31 (v1.31)"` added.
2. **§Postconditions Event 9 `clear_mode`/`actor_type` correspondence table**: TTL_EXPIRED row's "Trigger" and "Emission point" cells, and OPERATOR_OVERRIDE row's "Trigger" and "Emission point" cells, re-attributed from the WASM gate plugin to `check_and_clear_expired_marker` / `reconcile_raw_delete` (dispatcher-native, `indeterminate_marker.rs`).
3. **§Postconditions Event 9 Trigger bullets** (PC4 TTL_EXPIRED, PC3 OPERATOR_OVERRIDE): re-attributed to the same dispatcher-native functions; "emission point corrected v1.2" annotation added.
4. **§Postconditions Event 9 "Emission path" paragraph**: crash-path-exclusion note "only the gate plugin's normal-path auto-delete emits it" → "only the dispatcher-native pre-check (`check_and_clear_expired_marker`) emits it"; explicit statement added that `emit_marker_cleared` is called exclusively from dispatcher-native code, never from inside the WASM gate plugin.
5. **§Architecture Anchors**: `marker.cleared` (Event 9) emission path bullet — "gate plugin's TTL-check branch" / "gate plugin's RAW_DELETE_DETECTED reconciliation path" → `check_and_clear_expired_marker` / `reconcile_raw_delete`, both dispatcher-native.
6. **§Edge Cases**: EC-012 ("Gate plugin's TTL-check branch...") and EC-013 ("Gate plugin's RAW_DELETE_DETECTED...") re-attributed to the dispatcher-native pre-check functions.
7. **§Canonical Test Vectors**: `marker-cleared-ttl-expired` and `marker-cleared-operator-override` rows' "Input" column re-attributed from "gate plugin" to the dispatcher-native functions.
8. **Changelog table**: v1.31 row added at top.

**POLICY 1 verification:** All prior content preserved verbatim except the emission-locus corrections above (each is a targeted attribution fix, not a wire-format or field-shape change). No event IDs renumbered. Event count unchanged at nine. `marker.cleared` wire format (§Postconditions Event 9 JSON example, mandatory-fields list) is UNCHANGED — this amendment corrects WHERE the event is emitted from, not its shape.
**POLICY 7 verification:** H1 heading unchanged — no title-affecting content changed.
**POLICY 8 note:** No `bcs:` frontmatter array changes induced; S-25.01 story body/ACs are out of scope for this amendment per orchestrator routing (story-writer handles propagation).
**Companion architect-side corrections:** VP-108 v1.0 → v1.1 (Postcondition 2/3 re-attributed to `check_and_clear_expired_marker`/`reconcile_raw_delete`; proof harness rewritten to remove the fictional cross-WASM-boundary `evaluate_gate_with_sink`) and VP-106 v1.4 → v1.5 (Postcondition F/G retargeted from `evaluate_gate` to `check_and_clear_expired_marker`) were produced by the architect in the same burst; this BC amendment is the product-owner-side sibling correction for the same ADR-048 §D4 v1.2 finding.

## Amendment 2026-09-01 (v1.32 → v1.33 — ADR-048 §Decision 4 v1.4 Reconciliation-Premise Correction: new Event 10 `marker.written` catalog entry)

**Driver:** ADR-048 §Decision 4 v1.4 Reconciliation-Premise Correction (S-25.01 adversary pass 6 F-P6-001 MEDIUM; architect adjudication; HUMAN-RATIFIED 2026-09-01, POLICY 22, D-1141). F-P6-001 found that `reconcile_raw_delete`'s pre-v1.4 RAW_DELETE_DETECTED inference — "an unmatched fail-closed `plugin.indeterminate` proves a marker was durably written and later raw-deleted out-of-band" — is FALSE in two reachable cases: (1) a PreToolUse fail-closed INDETERMINATE (BC-1.18.001 Invariant 4 — marker write is PostToolUse-only) never attempts a marker write at all; (2) a PostToolUse marker-write I/O failure (BC-1.18.001 EC-007, swallowed best-effort) leaves the identical no-marker-ever-existed footprint. Both cases fabricate `marker.cleared(clear_mode=OPERATOR_OVERRIDE, actor_type=operator)` — a false NIST AU-3/AU-10 non-repudiation record attributing a human out-of-band action that never happened. The architect's fix (Option A — positive marker-creation record, per ADR-048 §D4 v1.4) introduces a new dispatcher-native audit event, `marker.written`, emitted by `write_indeterminate_marker`'s caller ONLY immediately after the atomic write returns `Ok(())` — never before, never on failure. `reconcile_raw_delete`'s scan retargets from an unmatched `plugin.indeterminate` (with its now-redundant `failure_policy=="fail-closed"` filter, removed) to an unmatched `marker.written`, making the reconciliation premise TRUE BY CONSTRUCTION. This is a genuinely NEW event (event count nine→ten), unlike v1.32's `SUPERSEDED`, which was a new VALUE of the existing Event 9 `clear_mode` enum.

**Changes made:**

1. **Frontmatter** (POLICY 14 legs 1–4): `version: "1.32"` → `"1.33"`; `last_amended` chain form prepended with v1.33 entry; `modified[]` entry `"2026-09-01 (v1.33)"` added.
2. **H1 title** (POLICY 7): `marker.written` appended to the event-type list.
3. **§Description**: `marker.written` provenance sentence added (ADR-048 §D4 v1.4, F-P6-001, triggering-condition/semantics authority = BC-1.18.001 §PC4 v1.4); "all nine" → "all ten"; note added that Events 8/9/10 durably land in `dispatcher-internal-{date}.jsonl`.
4. **§Common Fields**: "All nine event types" → "All ten event types" (×2 occurrences); `plugin_name` table row and the plugin-context-events bullet list extended with a note on Events 9/10's marker-sourced (not host-ctx-injected) `plugin_name`.
5. **New `### Event 10: marker.written` section** added under §Postconditions (immediately after Event 9's §Durable sink target paragraph): full JSON wire format, mandatory-fields list, `cause`/`expires_at`/`ts` field semantics, relationship-to-Event-9 rationale (the reconciliation-soundness argument), does-not-affect-block-decision note, emission path (`emit_marker_written`), and durable sink target (shared with Events 8/9).
6. **Event 9 retargeted** (§Postconditions): the OPERATOR_OVERRIDE Trigger bullet, the `clear_mode`/`actor_type` correspondence table's OPERATOR_OVERRIDE row, and the `trace_id` semantics paragraph all corrected from "unmatched `plugin.indeterminate`" to "unmatched `marker.written`" (ADR-048 §D4 v1.4, F-P6-001).
7. **§Invariants 1 and 3**: "all nine"/"All nine" → "all ten"/"All ten"; Invariant 3's event-type enumeration gains `marker.written`.
8. **§Related BCs**: BC-1.18.001 bullet extended to cover Event 10 triggering-condition/semantics authorship (alongside its existing Event 8 authorship); BC-1.18.003 bullet's clear-predicate list extended with SUPERSEDED and a note that RAW_DELETE_DETECTED is now sound against `marker.written`.
9. **§Architecture Anchors**: new bullet added for `emit_marker_written` (`indeterminate_marker.rs`); the existing `marker.cleared` bullet's `reconcile_raw_delete` clause extended with the v1.33 scan match-type correction.
10. **§VP Anchors**: VP-079 gains a new "Staleness flag OPEN (v1.33)" entry for Event 10 (Property Statement → ten events; mandatory-fields table gains `cause`/`expires_at`; new SITE_10 required); a note is added that VP-108 (BC-1.18.001/BC-1.18.003's emission-correctness VP) requires its PC3 fixture retargeted plus new PC6/PC7 postconditions. VP-028's event-type list gains `marker.written`; "all nine" → "all ten".
11. **§Edge Cases**: EC-013 retargeted from unmatched `plugin.indeterminate` to unmatched `marker.written`; new EC-015 (`write_indeterminate_marker` → `Ok(())` → `marker.written` emitted) and EC-016 (`Err(_)` → no emission, sibling of BC-1.18.001 EC-007) added.
12. **§Canonical Test Vectors**: the `marker-cleared-operator-override` row's Input column retargeted to unmatched `marker.written`; three new rows added — `marker-written-success`, `marker-written-failure-no-emit`, `marker-written-negative-control-pretooluse`.
13. **§Verification Properties**: VP-079 row updated to "ten event types" + Event 10 mention; two new VP-108 rows added (write-path emission correctness; F-P6-001 negative-control regression test).
14. **§Traceability**: DI-017 row "all nine" → "all ten"; Architecture Module row gains the `emit_marker_written` (Event 10) emission-site bullet; ADR row gains the ADR-048 §Decision 4 v1.4 citation; Stories row extended from "Events 8 + 9" to "Events 8 + 9 + 10".
15. **Changelog table**: v1.33 row added at top (existing table location, per established pattern — this file's Changelog table lives under the v1.11→v1.12 Amendment section's "**Changelog:**" label, not at the file's physical end).
16. **This Amendment section** appended at the true end of the file, following the v1.30/v1.31 precedent (v1.32's SUPERSEDED change recorded only a Changelog row, no dedicated Amendment section — this v1.33 addition, being a genuinely new event rather than a new enum value, warrants the fuller documentation).

**POLICY 1 verification:** All prior content preserved verbatim except the count-phrase sweep and Event 9 OPERATOR_OVERRIDE retarget items above (each a targeted, cited correction). No event IDs renumbered. Events 1–8's wire formats are UNCHANGED. Event 9's wire format (JSON example, mandatory-fields list, `clear_mode`/`actor_type` enum values) is UNCHANGED — only its OPERATOR_OVERRIDE reconciliation's INPUT SOURCE (which record type the scan matches) is corrected; the `marker.cleared` event Event 9 itself emits is identical in shape to v1.32.
**POLICY 7 verification:** H1 heading updated per POLICY 7 (new event type `marker.written` appended).
**TD-031/TD-VSDD-091 verification:** No new file:line-number citations introduced; the new Event 10 section identifies emission sites by function name (`write_indeterminate_marker`, `emit_marker_written`) + module path only.
**VP citation change handoff:** VP citations changed in: BC-3.08.001 (VP-079 gains Event 10 staleness flag OPEN; VP-028 gains `marker.written`; new VP-108 rows referenced for PC6/PC7). Architect must propagate to VP-079 SITE_10, mandatory-fields table, Property Statement, VP-108 (PC3 fixture retarget + new PC6/PC7), VP-INDEX, verification-architecture.md, and verification-coverage-matrix.md under `vp_index_is_vp_catalog_source_of_truth` policy.
**Companion product-owner-side siblings (same burst):** BC-1.18.001 v1.3 → v1.4 (§PC4 gains the `marker.written` write-path emission postcondition; this BC's Event 10 catalog entry is its wire-format-authority counterpart) and BC-1.18.003 v1.5 → v1.6 (§PC3 OPERATOR_OVERRIDE reconciliation retargeted to unmatched `marker.written`; new EC-017 negative control) were amended in the same burst as this BC's Event 10 addition — all three amendments jointly implement ADR-048 §D4 v1.4 / F-P6-001's fix.

## Amendment 2026-09-01 (v1.33 → v1.34 — ADR-048 §Decision 4 v1.5 Emission-Point Correction: Event 9 SUPERSEDED emission relocated to after write-success)

**Driver:** ADR-048 §Decision 4 v1.5 Emission-Point Correction (S-25.01 adversary pass 9 F-P9-001 MEDIUM; architect adjudication; HUMAN-RATIFIED 2026-09-01, POLICY 22, D-1142). F-P9-001 found that `emit_superseded_if_cross_pair` (added by §Decision 4 v1.3 to close F-P3-002) emitted `marker.cleared(clear_mode=SUPERSEDED, actor_type=system, reason=non-null)` for the OLD `(plugin_name, artifact_path)` pair UNCONDITIONALLY, BEFORE `write_indeterminate_marker` attempted the overwrite that the SUPERSEDED record purports to describe. If that write then returned `Err(_)` (EC-007, swallowed best-effort), the OLD marker was still durably present on disk and still enforcing its quarantine, yet a SUPERSEDED record had already been emitted falsely claiming it was overwritten — a fabricated audit record (NIST AU-3/AU-10 non-repudiation defect), the exact class ADR-048 v1.3/v1.4 exist to eliminate. This is the un-swept sibling of the v1.33 `marker.written` "emit only after `Ok(())`" discipline (TD-VSDD-060 sibling-sweep miss): v1.33 established that `marker.written` — the write's POSITIVE creation record — may be emitted only after `write_indeterminate_marker` confirms success; the identical discipline was never applied to `marker.cleared(SUPERSEDED)` — the SAME write's side-effect clearance record for the marker it displaces. Fix: `emit_superseded_if_cross_pair`'s call MOVES to inside the `Ok(())` arm of `write_indeterminate_marker`'s result, alongside the (unchanged) `emit_marker_written` call the v1.33 amendment already placed there. The OLD marker's fields are still read BEFORE the write (unavoidable — the write overwrites them), but the EMISSION now fires only once the overwrite is confirmed durable. On `Err(_)`, NEITHER `marker.cleared(SUPERSEDED)` NOR `marker.written` is emitted.

**Changes made:**

1. **Frontmatter** (POLICY 14 legs 1–4): `version: "1.33"` → `"1.34"`; `last_amended` chain form prepended with v1.34 entry; `modified[]` entry `"2026-09-01 (v1.34)"` added.
2. **§Postconditions Event 9 PC5 SUPERSEDED Trigger bullet**: corrected — the pre-overwrite field read is unavoidable and unchanged, but emission is now stated as occurring ONLY AFTER `write_indeterminate_marker` returns `Ok(())`, in the same write-success arm as Event 10's `marker.written`; NEITHER event emitted on `Err(_)`.
3. **§Postconditions Event 9 `clear_mode`/`actor_type` correspondence table** SUPERSEDED row: corrected to the same AFTER-`Ok(())` ordering, with an explicit "corrected v1.34" citation.
4. **§Postconditions Event 9 Emission path paragraph**: gains a v1.34 correction sentence naming the relocation and the on-`Err(_)`-neither-emits rule.
5. **§Postconditions Event 10 Trigger paragraph**: the pre-existing claim that `marker.written` fires "BEFORE ANY other marker-lifecycle event for this write" is corrected — it is now stated as firing AFTER the (now equally `Ok(())`-gated) SUPERSEDED emission, matching the ordering already documented in §Architecture Anchors' Event 10 bullet.
6. **§Architecture Anchors**: Event 9's `marker.cleared` emission-path bullet and Event 10's `emit_marker_written` bullet both gain a v1.34 correction sentence stating the two write-tied events are now gated identically.
7. **§Edge Cases**: EC-014 (SUPERSEDED cross-pair overwrite) corrected to AFTER-`Ok(())` ordering, with a forward reference to EC-016; EC-016 (`write_indeterminate_marker` returns `Err(_)`) extended with a clause stating SUPERSEDED is likewise withheld on failure.
8. **§Canonical Test Vectors**: `marker-cleared-superseded` row corrected to AFTER-`Ok(())` ordering; new `marker-cleared-superseded-failure-no-emit` row added for the `Err(_)` negative control.
9. **§Verification Properties**: new VP-108 row added citing the v1.34/F-P9-001 negative-control regression test (VP-108 PC8).
10. **§Traceability**: ADR row gains the ADR-048 §Decision 4 v1.5 citation.
11. **Changelog table**: v1.34 row added at top.
12. **This Amendment section** appended at the true end of the file, following the v1.30/v1.31/v1.33 precedent.

**POLICY 1 verification:** All prior content preserved verbatim except the ordering-correction items above (each a targeted, cited correction). No event IDs renumbered; event count remains ten. `marker.cleared`'s wire format (JSON example, mandatory-fields list, `clear_mode`/`actor_type` enum values, the `reason`-mandatory-for-SUPERSEDED rule) is UNCHANGED — this amendment corrects WHEN Event 9's SUPERSEDED value is emitted relative to the marker write, not its shape. `marker.written`'s wire format (Event 10) is likewise UNCHANGED — v1.33's `Ok(())`-only rule for it was already correct and is unaffected.
**POLICY 7 verification:** H1 heading unchanged — no title-affecting content changed (no new event, no clear_mode/actor_type value change).
**POLICY 8 note:** No `bcs:` frontmatter array changes induced; S-25.01 story body/ACs are out of scope for this amendment per orchestrator routing (story-writer handles propagation).
**VP citation change handoff:** VP citations changed in: BC-3.08.001 (new VP-108 PC8 regression-test row referenced). Architect must propagate to VP-108 (already corrected to v1.4 in the same burst — PC5 fixture reordered, new PC8 added), VP-INDEX, verification-architecture.md, and verification-coverage-matrix.md under `vp_index_is_vp_catalog_source_of_truth` policy.
**Companion product-owner-side siblings (same burst):** BC-1.18.001 v1.4 → (bump) (§PC4/Invariant 3 SUPERSEDED-corollary ordering corrected to AFTER-`Ok(())`) and BC-1.18.003 v1.6 → (bump) (§PC5 SUPERSEDED emission-point corrected; new edge case for cross-pair + `Err(_)` non-fabrication) were amended in the same burst as this BC's Event 9/Event 10 ordering correction — all three amendments jointly implement ADR-048 §D4 v1.5 / F-P9-001's fix, ground-truthed against the architect's ratified ADR-048 v1.5 and VP-108 v1.4.

## Amendment 2026-10-07 (v1.34 → v1.35 — ADR-052 §Decision 5a v1.21 "Admission diagnostics channel": Events 11–13 `migration.admission_blocked` / `_failed` / `_advisory` added)

**Driver:** ADR-052 v1.21 §Downstream item 25 (S-25.08 local adversary pass 2 finding D-1; architect-ratified). Fact check recorded by the architect: the dispatcher binary installs NO `tracing` subscriber (`factory-dispatcher/Cargo.toml` has `tracing` but not `tracing-subscriber`; no `set_global_default`), so every `tracing::warn!`/`error!`/`info!` in the dispatcher — the governed-migration writer-admission gate's included — is discarded in production. The durable, spec-owned diagnostic channel is the dispatcher-internal JSONL log (`InternalLog` → `dispatcher-internal-YYYY-MM-DD.jsonl`), which `main.rs::run` already holds at the admission call site. Installing a global subscriber was explicitly rejected (a dispatcher-wide logging-policy change, not needed).

**Changes made:**

1. **Frontmatter** (POLICY 14 legs 1–4): `version: "1.34"` → `"1.35"`; `last_amended` chain prepended with the v1.35 entry; `modified[]` entry `"2026-10-07 (v1.35)"` added.
2. **H1**: the three new event-type strings appended (POLICY 7; BC-INDEX title column updated in the same burst).
3. **§Description**: Events 11–13 provenance sentence added (S-25.08 pass-2 D-1; triggering authority BC-1.18.013 Postcondition 10 / BC-1.18.011 Postcondition 10) with the two load-bearing differences from Events 1–10 (direct `InternalLog::write`; no `plugin_name`).
4. **§Common Fields**: new bullet stating the Events 11–13 envelope (`type`, `ts`, `ts_epoch`, `schema_version`, `trace_id`, `session_id`; no `plugin_name`/`plugin_version`).
5. **§Postconditions**: new `### Event 11`, `### Event 12`, `### Event 13` sections (trigger, wire format, mandatory fields, field semantics/sanitization, emission path, durable sink target) appended after Event 10.
6. **§Postconditions Sink destination paragraph**: scope note — governs Events 1–10 only.
7. **§Invariants**: 1 and 3 extended ten→thirteen; NEW Invariant 7 (exactly-once, InternalLog-only, no raw `tool_use_id`/record content).
8. **§Related BCs**: BC-1.18.013 and BC-1.18.011 bullets added. **§Architecture Anchors**: new bullet for the Events 11–13 emission path (`internal_log.rs` constants, `main.rs` drain of `AdmissionDiagnostic` data, the shared-core purity split); `sink-core` bullet scoped to Events 1–10.
9. **§VP Anchors**: VP-079 staleness flag CLOSED with VP-079 v1.24 cited (Events 11–13 wire-format rows; Property 6 SITES not extended); VP-028 v1.1 scope note (Events 11–13 are not SinkEvents, out of fan-out scope); emission semantics cite VP-133 v1.6 facet 7(e) / VP-143 v1.5. Follow-up item 33: Event 11 `reconciliation` six-token domain + `branch` derivation; Event 13 five-token `reason` domain + closed optional-field set; Events 11–13 field semantics sections and EC-019/vectors updated accordingly.
10. **§Edge Cases**: EC-017, EC-018, EC-019 added (one per event type). **§Canonical Test Vectors**: four rows added (`migration-admission-blocked`, `migration-admission-failed`, `migration-admission-advisory`, `migration-admission-no-event`).
11. **§Traceability**: DI-017 row ten→thirteen; ADR row gains ADR-052 §Decision 5a v1.21. (Stories row UNCHANGED: S-25.08 implements Events 11–13 but its `bcs:` frontmatter does not yet list BC-3.08.001; the `bcs:` array change and its body/AC propagation are the story-writer's under POLICY 8 — see the handoff note below.)
12. **Changelog table**: v1.35 row added at top. **This Amendment section** appended at the true end of the file.

**POLICY 1 verification:** All prior content preserved verbatim except the count-phrase extensions and scope notes listed above. No event IDs renumbered (Events 1–10 unchanged; Events 11–13 are genuinely new). Events 1–10 wire formats UNCHANGED. The count-phrase sweep was deliberately NOT applied to §Common Fields' "All ten event types" sentences, §Verification Properties' VP-079/VP-028 rows or VP-079's Property Statement wording (they remain scoped to Events 1–10 pending the architect's decision on VP-079/VP-028).
**POLICY 7 verification:** H1 heading changed (three event-type strings appended); BC-INDEX title column updated to match the H1 verbatim in the same burst.
**POLICY 8 note:** No `bcs:` frontmatter array is changed by this burst (the product-owner does not edit stories). Stories affected by BC changes: S-25.08 (its `bcs:` array does not list BC-3.08.001, which now catalogues the Events 11–13 that S-25.08 AC-022 implements — story-writer must decide and propagate under `bc_array_changes_propagate_to_body_and_acs`), S-25.06 (cites BC-1.18.013 v1.10). Story-writer handles propagation.
**VP citation change handoff:** VP citations changed in: BC-3.08.001 (VP-079 v1.24, VP-028 v1.1, VP-133 v1.6, VP-143 v1.5 cited; staleness flag closed). The architect has already propagated these (VP-INDEX; no VP counts change, so verification-architecture.md and verification-coverage-matrix.md are untouched).
**Companion product-owner-side siblings (same burst):** BC-1.18.013 v1.9 → v1.10 and BC-1.18.011 v1.17 → v1.18 (Postcondition 10 of each is the triggering-condition authority for Events 11–13), and error-taxonomy v1.38 → v1.39 (`E-MAINTENANCE-001`/`E-MAINTENANCE-002` rows cite `migration.admission_blocked` / `migration.admission_failed`).

## Amendment 2026-10-08 (v1.37 → v1.38 — ADR-052 v1.25 item 11(g): the 64-character cap is InternalLog-event-only)

**Driver:** ADR-052 v1.25 item 11(g) ("Sanitization caps are per SURFACE, and a data-derived value is sanitized at RENDER time, never at construction"). Two surfaces render the same `AdmissionStateIntegrity` / `Io` facts and have different caps: the `InternalLog` events 11–13 (64 characters per data-derived substring, the log-injection hygiene bound for a structured, queryable field) and the operator stderr (the coordinator `MIGRATION_STATE_INTEGRITY_FAILURE` / failure line and the Display of any `BcIndexMigrationError`: 256 characters per data-derived substring, because a 64-character cap would cut a macOS project path of ~108 characters before its file name). BC-1.18.013 v1.14 Postcondition 10 "Coordinators" clause states the 256 operator cap; this BC keeps 64 and says so.

**Changes made:**

1. **Frontmatter** (POLICY 14 legs 1–4): `version: "1.37"` → `"1.38"`; `last_amended` chain prepended with the v1.38 entry; `modified[]` entry `"2026-10-08 (v1.38)"` added; Changelog table v1.38 row added at top.
2. **Event 12 `detail` field semantics**: the existing "truncated to 64 characters for any data-derived substring" is KEPT and "(InternalLog event only; the operator stderr rendering of the same fact is capped at 256 per BC-1.18.013 Postcondition 10)" is added.
3. **Invariant 7**: same parenthetical added after the 64-character rule; states the raw-slot / render-time-sanitization rule (the event `detail` is built from the same raw fields as the stderr Display; never re-truncated after escaping).
4. **EC-018** and **§Canonical Test Vectors**: a ~100-character-path vector (64 in the event, untruncated on stderr) and a 108-character macOS path vector.

**POLICY 1 verification:** all prior content preserved verbatim except the parenthetical additions above. Events 1–13 wire formats UNCHANGED; no event renumbered; count stays thirteen.
**POLICY 8 note:** no `bcs:` frontmatter array is changed. Stories affected by BC changes: none.
**VP citation change handoff:** VP citations changed in: none.
**Companion product-owner-side siblings (same burst):** BC-1.18.013 v1.14 (same-version extension: per-slot 256 rule, 256/64 surface split, coordinator-only advisory emission; EC-058/EC-061 vectors), BC-1.18.011 v1.22 and error-taxonomy v1.42 (`detail` sanitization wording marked event-only 64 / stderr 256).
