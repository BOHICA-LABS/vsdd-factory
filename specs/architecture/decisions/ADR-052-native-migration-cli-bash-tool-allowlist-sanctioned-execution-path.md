---
document_type: architecture-decision-record
level: L3
version: "1.0"
status: proposed
producer: architect
timestamp: 2026-09-12T00:00:00Z
phase: F1
subsystems_affected:
  - SS-01
traces_to: .factory/specs/architecture/ARCH-INDEX.md
inputs:
  - .factory/cycles/v1.0-brownfield-backfill/adv-cv-dir-cluster5-F1-direction-2026-09-12.md
  - .factory/stories/S-25.06-append-log-backfill-split-executor.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.011.md
  - .factory/cycles/v1.0-brownfield-backfill/s2502-cluster5-f1-delta-analysis.md
  - CLAUDE.md
input-hash: "e7f60f5"
# input-hash: run compute-input-hash --update at state-manager registration burst
---

# ADR-052: Native Migration CLI — Bash-Tool-with-Allowlist as Sanctioned Execution Path for Governed One-Time Shard Migrations

## Status

PROPOSED — Human ratification required per POLICY 22 before this ADR's design is
treated as accepted. This ADR is proposed concurrent with cluster-5 F1 delta-analysis
(`s2502-cluster5-f1-delta-analysis.md`). POLICY 22 ratification gates cluster-5 TDD
dispatch.

## Context

Two governed one-time shard migrations are specified in S-25.02:

1. **Mechanism-A append-log backfill-split** (BC-1.18.008) — wired and executed by S-25.06.
   `run_mechanism_a_backfill_split` exists in `shard_manager.rs` with zero production callers.
   S-25.06 adds the CLI entry point and executes the migration against the four live cycle
   append-log files.

2. **Mechanism-B2 BC-INDEX body-split migration** (BC-1.18.011) — cluster-5 of S-25.02.
   The `run_mechanism_b2_bc_index_split` function (to be authored in cluster-5 TDD) is the
   native migration entry point in `shard_manager.rs`.

Both migrations share a structural challenge: they perform filesystem writes against
`.factory/` paths but cannot satisfy two simultaneously imposed constraints as originally
stated in S-25.06 Rule 7:

- **Constraint A:** The executor MUST be a native binary (not WASM; WASM fuel budgets are
  insufficient for multi-MB files per ADR-051 §Decision 1 / CLAUDE.md §WASM plugin fuel
  budgets).
- **Constraint B (original Rule 7 text):** The invocation must be "logged as an Edit/Write
  tool call" — implying it goes through the agent's Edit/Write surface that hooks validate.

These constraints are mutually exclusive: a native binary invoked from the shell writes
to the filesystem OUTSIDE the Edit/Write tool surface. A native binary invocation cannot
simultaneously be an Edit/Write tool call.

Additionally, three project-level governance constraints apply:

- **POL-3 / TD-FACTORY-HOOK-BYPASS-001 P0:** Agents must NOT use Python/sed/echo/shell to
  bypass the hook-validated `.factory/` write surface.
- **D-449(a):** Mechanical gates in burst-log Dim-2 require literal shell execution with
  captured stdout, not pseudocode narrative.
- **F4 activation gate:** Both migrations are one-time operations that require explicit human
  authorization and must be executed at a known activation boundary — not triggered
  automatically by a hook during normal dispatch.

The cross-vendor direction review (CV-DIR-F2, `adv-cv-dir-cluster5-F1-direction-2026-09-12.md`)
identified the contradiction in S-25.06 Rule 7 and routed resolution to the architect.

## Decision

**Decision 1 — Sanctioned invocation path for both native migration CLIs:**

Governed one-time shard migrations are invoked via the **`Bash` tool with a narrowly-scoped,
pre-authorized permission allowlist entry** in `.claude/settings.json` (project-level).

The native migration binary IS the sanctioned migration path — it implements the full
BC-1.18.008 / BC-1.18.011 specification including content-preservation verification,
independent census, staging + verify + atomic replace (BC-1.18.006 primitives), fail-loud
rollback on failure, and idempotency. This is NOT a POL-3 bypass; it is the VSDD-specified,
adversarially-reviewed implementation of the spec. POL-3 targets unstructured bypasses
(raw `cp`, `python -c open()`, `sed`); a purpose-built migration binary implementing the
spec is the opposite of an unstructured bypass.

**Decision 2 — Binary placement for both migration entry points:**

Both migration entry points live in the `factory-dispatcher` binary (`crates/factory-dispatcher/`),
co-located with `shard_manager.rs`. Rationale:

- `shard_manager.rs` already owns the migration algorithm for mechanism A
  (`run_mechanism_a_backfill_split`). B2's entry point (`run_mechanism_b2_bc_index_split`)
  follows the same co-location model.
- `last-amended-migrate` is BC-10.13.001-scoped (frontmatter `changelog:`/`last_amended`
  mutations; `TARGET_FILES` = 5 index files + STATE.md). Extending it for body-structure
  migrations requires a BC-10.13.001 amendment without commensurate benefit.
- `factory-dispatcher` already has a CLI dispatch layer. Adding `migrate-bc-index` and
  `backfill-append-logs` subcommands is additive and consistent with the existing pattern.

**Decision 3 — Permission policy (allowlist entry format):**

Two named entries are added to `.claude/settings.json` (project-level, NOT user-level):

```json
{
  "permissions": {
    "allow": [
      "Bash(*/factory-dispatcher migrate-bc-index*)",
      "Bash(*/factory-dispatcher backfill-append-logs*)"
    ]
  }
}
```

(Exact syntax subject to the Claude Code permission schema in force at implementation time;
the principle is a narrow pattern matching only the specific subcommand, not a wildcard over
arbitrary binary invocations.)

Responsibility: devops-engineer adds these entries as part of cluster-5 activation, before
any activation-step execution. The entries are PR-reviewable (project-level settings.json).

**Decision 4 — Audit-trail obligation (census report to stdout):**

Each migration CLI invocation MUST write a structured census report to stdout on completion
(success or idempotent no-op). Minimum required fields:

- Source file path(s) operated on
- Pre-migration record count (from authoritative frontmatter oracle, e.g. `total_bcs` for
  the B2 case)
- Independent-census record count (fresh enumeration of actual rows)
- Per-shard record counts and file paths (including sub-shards for SS-05/SS-06 in the B2 case)
- Content-preservation hash (SHA-256 of source body before migration)
- Idempotency status (`FIRST_RUN` | `ALREADY_MIGRATED` | `RESUMED_FROM_CHECKPOINT`)
- Exit code (0 = success, non-zero with error taxonomy code on failure)

The state-manager records the captured stdout verbatim in the burst-log's Dim-2 attestation
for the activation burst. This IS the D-449(a)-required literal-shell-execution evidence
for the activation gate. No separate log file is required.

**Decision 5 — Resolving the S-25.06 Rule 7 contradiction:**

S-25.06 Rule 7's original text ("agent-executed CLI invocation logged as an Edit/Write tool
call") is WITHDRAWN and replaced with: "The activation step (T-10 for mechanism A, the
equivalent activation step for mechanism B2) is executed via the `Bash` tool under the
pre-authorized allowlist entry defined by ADR-052 §Decision 3. The invocation is NOT an
Edit/Write tool call. It is a Bash execution of the native migration binary, subject to the
project-level allowlist permission and producing a census report captured per ADR-052
§Decision 4." Story-writer updates S-25.06 Rule 7 text accordingly in the next burst.

**Decision 6 — BC-1.18.010 Invariant 2: ARCH-INDEX mapping read mechanism:**

The BC-S-prefix→SS-NN mapping that BC-1.18.010 Invariant 2 mandates be "READ from
ARCH-INDEX's Subsystem Registry, never independently hardcoded" is implemented as a
**config-embedded snapshot with a CI parity test** (Option C from the delta-analysis §2.1):

- The mapping is embedded in the `[[shard]]` TOML config entry for the BC-INDEX artifact
  as a `subsystem_prefixes` table: `{ "BC-1" = "SS-01", "BC-2" = "SS-02", ..., "BC-10" = "SS-10" }`.
- A CI test (`cargo test --test arch_index_parity`) greps ARCH-INDEX.md's Subsystem Registry
  `BC-S Prefix` column and diffs it against the config snapshot, failing on any divergence.
- Rationale: (a) Zero dispatch-path overhead — no runtime file parse; (b) The mapping changes
  only when ARCH-INDEX's subsystem numbering changes (extremely rare, bounded by POLICY 1);
  (c) The CI test provides the governance guarantee Invariant 2 requires.
- `shard_manager.rs` reads the mapping from the deserialized config entry only, never from
  ARCH-INDEX.md at runtime. This is fully consistent with BC-1.18.010 Invariant 2's
  prohibition on independent hardcoding: the config snapshot is not independent — it is
  CI-validated against ARCH-INDEX at every commit.

This decision supersedes the ambiguity in BC-1.18.010 Invariant 2 about the runtime
read mechanism. Product-owner amends Invariant 2 to cite this decision.

## Rationale

### Why NOT dispatcher-internal hook-driven (Option A)

A one-time migration triggered automatically inside a PreToolUse hook is:

1. **Timing-uncontrollable:** The hook fires on any `Edit`/`Write` against BC-INDEX.md.
   Before BC-1.18.010's end-state addressing is fully implemented and validated, this would
   trigger the migration during ordinary tool operations — potentially corrupting BC-INDEX.md
   before the shard files have a valid addressing scheme.

2. **Not re-runnable:** Hook-internal state is not interactive. An operator cannot run
   the migration in dry-run mode, inspect the census, and confirm before committing.

3. **Cannot satisfy the F4 activation gate:** BC-1.18.011 requires explicit human
   authorization for the "F4 activation" boundary. An automatically-triggered hook cannot
   satisfy this — it fires based on detection logic, not human intent.

4. **No D-449(a) evidence path:** Burst-log Dim-2 requires literal shell stdout capture.
   A hook-internal migration produces no capturable stdout for the session.

### Why NOT a real tool adapter / MCP server (Option C)

Disproportionate operational complexity (MCP server process management, authentication,
port allocation) for a one-time batch operation with no recurring invocation need.

### Why the Bash-tool allowlist IS consistent with POL-3

POL-3 / TD-FACTORY-HOOK-BYPASS-001 P0 prohibits:
- (a) "Python/sed/echo bypass" — unstructured, unvalidated writes to `.factory/` paths
- (b) Agent-role violations — wrong specialist touching STATE.md

The native migration binary is neither:
- It implements the FULL BC specification (content-preservation, census, crash-atomicity,
  rollback). This is the OPPOSITE of a bypass.
- It is authored under VSDD's standard TDD/adversarial-convergence process.
- It writes via BC-1.18.006's `write_atomic` primitives — the same sanctioned path every
  `.factory/` write uses.
- Its invocation path is pre-authorized in project-level settings.json — reviewable in PRs.

The allowlist entry constrains the permission to the specific subcommand pattern, not a
broad bash privilege. This is narrower than many existing project `Bash` permissions.

## Consequences

### Positive

- Resolves the internally contradictory execution path in S-25.06 Rule 7.
- Provides a fully auditable, crash-safe, spec-compliant invocation path for both
  mechanism-A and mechanism-B2 migrations.
- D-449(a) census-report-to-stdout requirement satisfies the burst-log Dim-2
  literal-shell-execution-evidence obligation with zero additional tooling.
- Config-embedded ARCH-INDEX mapping (Decision 6) eliminates a runtime file-parse
  overhead on every dispatch while maintaining the governance invariant.

### Negative

- Adds two Bash allowlist entries to `.claude/settings.json` that require maintenance
  if the binary or subcommand names change.
- The migration binary produces no PostToolUse hook event (it's a Bash invocation, not
  an Edit/Write), so the standard hook-validation chain does not fire for the migration
  itself. Mitigation: the census report stdout IS the equivalent validation evidence, and
  the migration's own internal verification (content-preservation check + census) is
  the substantive correctness guarantee.

### Neutral

- BC-1.18.011's invocation is now fully specified; the product-owner amendment (§3.2 of
  the cluster-5 F1 delta-analysis) can proceed.
- S-25.06 Rule 7 text change is a correction of a false constraint, not a behavioral change.

## References

- `adv-cv-dir-cluster5-F1-direction-2026-09-12.md` — CV-DIR-F2 finding that prompted this ADR
- `s2502-cluster5-f1-delta-analysis.md` §4 — full options evaluation
- `BC-1.18.011` §Preconditions (4), §Architecture Anchors — amended to cite this ADR
- `BC-1.18.010` Invariant 2 — amended per Decision 6
- `S-25.06-append-log-backfill-split-executor.md` Rule 7 — corrected per Decision 5
- `ADR-051` §Decision 1 (WASM fuel-budget constraint), §Decision 7 (B2 end-state),
  §Decision 10 (governed one-time B2 migration)
- `CLAUDE.md` §WASM plugin fuel budgets, §TD-FACTORY-HOOK-BYPASS-001 P0, §POL-3

## Files to Change

| File | Change |
|------|--------|
| `.claude/settings.json` | Add two Bash allowlist entries per Decision 3 (devops-engineer) |
| `.factory/specs/behavioral-contracts/ss-01/BC-1.18.011.md` | Amend Precondition 4 + add Architecture Anchor for ADR-052 (product-owner, per delta-analysis §3.2) |
| `.factory/specs/behavioral-contracts/ss-01/BC-1.18.010.md` | Amend Invariant 2 to cite Decision 6's config-snapshot implementation (product-owner) |
| `ADR-051` | Add §Decision 17 for config-embedded snapshot mechanism (architect, same burst) |
| `S-25.06-append-log-backfill-split-executor.md` | Correct Rule 7 text per Decision 5 (story-writer) |
| `crates/factory-dispatcher/src/shard_manager.rs` | Add `run_mechanism_b2_bc_index_split` entry point; add `migrate-bc-index` CLI subcommand + census-report stdout (implementer, cluster-5 TDD) |
| `crates/factory-dispatcher/tests/` | Add `arch_index_parity` CI test for config-snapshot vs ARCH-INDEX (implementer, cluster-5 TDD) |

## Changelog

| Version | Date | Author | Change |
|---------|------|--------|--------|
| 1.0 | 2026-09-12 | architect | Initial authoring. Resolves CV-DIR-F2 (impossible execution path). Specifies Bash-tool-with-allowlist as sanctioned invocation path for both mechanism-A (S-25.06) and mechanism-B2 (BC-1.18.011) migrations. Defines permission policy, audit-trail design, binary placement, and ARCH-INDEX mapping read mechanism. See cluster-5 F1 delta-analysis for full options evaluation. |
