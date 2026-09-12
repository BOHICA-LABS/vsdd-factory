---
document_type: f1-delta-analysis
level: ops
version: "1.0"
status: complete
producer: architect
timestamp: 2026-09-12T00:00:00Z
phase: F1
scope: "S-25.02 cluster-5 — BC-1.18.010 (B2 body-table sharding) + BC-1.18.011 (governed B2 migration)"
verdict: "NEEDS-CONVERGENCE — BC-1.18.011 Precondition 4 simultaneous-activation language is an over-constraint (CV-DIR-F1a); invocation mechanism contradiction (CV-DIR-F2) requires ADR-052 before TDD can start. BC-1.18.010 is near-ready pending one ARCH-INDEX-mapping-runtime gap noted below."
inputs_reviewed:
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.010.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.011.md
  - .factory/stories/S-25.02-artifact-sharding-layer2.md
  - .factory/stories/S-25.06-append-log-backfill-split-executor.md
  - .factory/cycles/v1.0-brownfield-backfill/adv-cv-dir-cluster5-F1-direction-2026-09-12.md
  - .factory/specs/architecture/ARCH-INDEX.md
  - crates/factory-dispatcher/src/shard_manager.rs
  - crates/last-amended-migrate/src/migrate.rs
adr_produced: ADR-052
---

# S-25.02 Cluster-5 F1 Delta-Analysis

> Architect-authored F1 analysis for cluster-5 (BC-1.18.010 + BC-1.18.011). Covers impact
> boundary, spec-readiness gaps, CV-DIR-F1a simultaneous-activation reconciliation (with exact
> BC-1.18.011 amendment for product-owner), CV-DIR-F2 invocation-mechanism architecture
> (ADR-052 authored), CV-DIR-F1b regression-risk flag, and ordered downstream routing plan.
> NOT committed. NOT an INDEX/STATE edit. Hand-off to state-manager for commit.

---

## 1. Impact Boundary

### 1.1 Crates Touched

| Crate | Change | Regression Risk |
|-------|--------|----------------|
| `crates/factory-dispatcher/src/shard_manager.rs` | NEW: B2 artifact-shape handler (per-subsystem body-table splitting, first-level addressing, second-level sub-sharding trigger), plus the one-time migration entry point (`run_mechanism_b2_bc_index_split`) reusing BC-1.18.006's atomic-write primitives | HIGH — shard_manager.rs is the hot path for every `Edit`/`Write`/`MultiEdit` PreToolUse dispatch; any regression here blocks the entire hook chain |
| `crates/factory-dispatcher` (config TOML) | NEW `[[shard]]` entry with `shape = "per-subsystem-body-table"` for `BC-INDEX.md`; populates the B2 artifact-shape case path | MEDIUM — config deserialization error triggers fail-loud for ALL dispatches matching that artifact |
| `crates/last-amended-migrate` | NO CHANGE — `TARGET_FILES` const (5 files: STATE.md, BC-INDEX.md, STORY-INDEX.md, ARCH-INDEX.md, VP-INDEX.md) is BC-10.13.001-scoped; the B2 migration is a body-structure split, not a frontmatter `changelog:`/`last_amended` mutation, and does NOT belong in this binary (see §3.2 below for the binary-decision rationale) | NONE — confirmed no touch |

**Zero-production-callers note (confirmed by CV direction review):** `run_mechanism_a_backfill_split` (the mechanism-A function shipped in cluster-3, PR #831) has zero production callers — confirmed by grep. The B2 migration entry point to be authored in cluster-5 will share this characteristic until S-25.06 and BC-1.18.011's one-time activation steps are executed. This is expected and correct — both are one-time migration functions that are wired but not triggered until explicit F4 activation.

### 1.2 Spec Artifacts Touched

| Artifact | Change |
|----------|--------|
| `.factory/specs/behavioral-contracts/BC-INDEX.md` | Body stripped post-migration: zero per-BC rows remain; only `§Summary`, `§Subsystem Shard Manifest`, and cross-cutting invariants |
| `.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-NN.md` (×10) | NEW files created by BC-1.18.011 migration; each holds one subsystem's BC rows verbatim |
| `.factory/specs/behavioral-contracts/shards/BC-INDEX.shard-manifest.toml` | NEW; top-level manifest for the 10 subsystem shards |
| `.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-05.manifest.toml` | NEW; second-level sub-shard manifest for SS-05 (661 BCs, ~88,695 bytes, over cap) |
| `.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-06.manifest.toml` | NEW; second-level sub-shard manifest for SS-06 (592 BCs, ~85,407 bytes, over cap) |
| `.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-05.{a,b,...}.md` | NEW; SS-05 sub-shards |
| `.factory/specs/behavioral-contracts/shards/BC-INDEX-SS-06.{a,b,...}.md` | NEW; SS-06 sub-shards |

### 1.3 Tooling / Validator Touchpoints (Migration Surface)

Per BC-1.18.010 Postcondition 5's enumerated bounded migration surface:

| Touchpoint | Required Change | Owner |
|------------|----------------|-------|
| Product-owner BC authorship workflow | Write target becomes per-subsystem shard file, not `BC-INDEX.md` body | product-owner (documentation + process) |
| State-manager POLICY 7/8 title-sync + count-propagation bursts | `§Summary` count aggregation must sum across shard files' actual row counts; `validate-count-propagation.sh`'s `_extract_counts` needs a companion pass across the shard set | state-manager / devops-engineer |
| Consistency-validator cross-reference checks | Any check that currently globs or full-text-scans `BC-INDEX.md` for a specific ID must instead consult the shard manifest or glob `shards/BC-INDEX-SS-*.md` | consistency-validator |
| Adversarial-review POLICY auto-load | Confirmed NOT to require migration (loads `.factory/policies.yaml`, not BC-INDEX.md body) | no action |

### 1.4 Regression Risk Surface Summary

| Risk Area | Level | Notes |
|-----------|-------|-------|
| shard_manager.rs hot path | HIGH | New B2 artifact-shape dispatch must not regress existing flat-shape or frontmatter-changelog-array-shape dispatch paths |
| BC-INDEX.md post-migration body | HIGH | Any validator, tool, or agent that still reads BC-INDEX.md body for per-BC rows will find zero rows — a data-corruption signal, not a soft error |
| POLICY 7 title-authority invariant | MEDIUM | Row's file path changes from BC-INDEX.md body to shard file; authority relationship is unchanged, but any grep-based title check targeting BC-INDEX.md body breaks silently |
| VP-127/VP-128/VP-132/VP-133/VP-134 coverage | MEDIUM | Five new VPs with unit/integration/fault-injection proof obligations; test-writer must author all before TDD dispatch |
| S-25.06↔S-25.02 blocks edge | LOW (flagged for follow-up, §5) | Consistency-validator sweep required; does not block cluster-5 TDD |

---

## 2. Spec-Readiness Assessment

### 2.1 BC-1.18.010 (B2 End-State) — NEAR-READY

**Assessment:** BC-1.18.010 v1.2 is substantively correct and well-specified for the end-state. One gap requires an ADR-051 amendment before the implementer can write compliant code.

**Gap 1 (MEDIUM) — ARCH-INDEX mapping read mechanism unspecified.**
BC-1.18.010 Invariant 2 states: "The BC-S-prefix→SS-NN mapping used for first-level addressing is READ from ARCH-INDEX's Subsystem Registry, never independently hardcoded or duplicated in `shard_manager.rs`."

This invariant is correct as a governance constraint but does not specify HOW `shard_manager.rs` accesses this mapping at runtime. Options with distinct implementation and performance implications:
- (a) Startup-loaded parse of ARCH-INDEX.md (355KB file; acceptable if cached after first parse, expensive if re-read per dispatch)
- (b) Build-time code-generation extracting the mapping into a Rust const (correct for a rarely-changing table; invalidated by ARCH-INDEX rebuild but acceptable with a CI check)
- (c) Config-embedded snapshot in the `[[shard]]` TOML with a CI test verifying parity against ARCH-INDEX at build time

**Recommended resolution (architect decision):** Option (c) — a config-embedded snapshot with a CI parity test. This avoids runtime ARCH-INDEX parsing entirely, has zero dispatch-path overhead, and the parity test provides the governance guarantee Invariant 2 requires. The CI test greps ARCH-INDEX's Subsystem Registry `BC-S Prefix` column and diffs it against the config snapshot.

**Required action:** ADR-051 addendum (§Decision 17) specifying Option (c). Product-owner also amends BC-1.18.010 Invariant 2 to name the mechanism: "READ from ARCH-INDEX's Subsystem Registry — implemented as a config-embedded snapshot with a CI parity test verifying the snapshot matches ARCH-INDEX's committed state." This is a product-owner BC amendment (one sentence addition to Invariant 2).

**Gap 2 (LOW) — VP-128 appears twice in the Verification Properties table.**
BC-1.18.010 lists two rows with VP-128 (single-authoritative-row invariant AND mapping-source-of-truth invariant). Per the fix-burst note in the BC, this is intentional: VP-128 is a single VP covering multiple integration-test properties per the single-method-per-VP convention. This is consistent with VP-INDEX v3.02's assignment. No change needed; this is a documentation style note only.

**Conclusion:** BC-1.18.010 is implementation-ready once Gap 1's ADR-051 §Decision 17 addendum is authored and the corresponding one-line Invariant 2 amendment is applied by the product-owner.

### 2.2 BC-1.18.011 (B2 Governed Migration) — NEEDS CONVERGENCE

**Assessment:** BC-1.18.011 v1.0 has two gaps that prevent TDD dispatch. Both are addressed in this analysis.

**Gap 1 (HIGH) — Precondition 4 simultaneous-activation over-constraint (CV-DIR-F1a).**
See §3 below for full reconciliation and exact amendment text.

**Gap 2 (HIGH) — Invocation mechanism unspecified (CV-DIR-F2).**
BC-1.18.011 Postcondition 3 and Architecture Anchor name `shard_manager.rs` as the "one-time migration entry point" but do not specify HOW this entry point is invoked. The CV direction review (CV-DIR-F2) identified this same gap for S-25.06 and routed it to architect. The same gap applies here: BC-1.18.011's migration performs filesystem writes, yet there is no specified invocation path that satisfies both the "no agent shell bypass" constraint (POL-3) and the "no WASM fuel-budget exhaustion" constraint (ADR-051).

**Resolution:** ADR-052 (authored as part of this delta-analysis, §4 below) specifies the sanctioned `Bash`-tool-with-explicit-allowlist invocation path. Product-owner must amend BC-1.18.011 to reference ADR-052 in its Architecture Anchors and in Precondition 4 (§3 below).

**Gap 3 (LOW) — E-SHD-005 error code referenced without explicit taxonomy citation.**
BC-1.18.011 Canonical Test Vectors reference `E-SHD-005` for "fail-loud error surfaced." Verify this error code is present in `prd-supplements/error-taxonomy.md` before implementation. If it is absent, product-owner must add it. (BC-1.18.008's analogous code is `E-SHD-003`; BC-1.18.011's test vector implies a distinct code for census-failure in the B2 case — confirm or align.)

**Conclusion:** BC-1.18.011 status: NEEDS-CONVERGENCE. Blocked on Gap 1 amendment (§3) + Gap 2 ADR-052 reference (§4). Gap 3 is a verification task for the product-owner during the same amendment burst.

---

## 3. CV-DIR-F1a Reconciliation — Simultaneous-Activation Language

### 3.1 The Over-Constraint

BC-1.18.011 Precondition 4 states:

> "The migration is scheduled to execute at F4 activation, at the SAME moment BC-1.18.008's
> mechanism-A backfill-split runs (both are one-time migrations gated on the same F4 activation
> boundary, though they operate on independent artifact sets and have no ordering dependency on
> each other per Postcondition 7)."

This language was written when BC-1.18.008's mechanism-A backfill execution was expected to be performed by S-25.02 itself. Since then, a mechanistic-coverage gap was discovered (CV-DIR-F1, fixed to S-25.06): `run_mechanism_a_backfill_split` has zero production callers and requires a separate story (S-25.06) to wire and execute it. S-25.06 carries `blocks: [S-25.02]` — meaning cluster-7 cannot dispatch until S-25.06 completes.

The "SAME moment" language creates a false ordering dependency: it implies BC-1.18.011's B2 migration cannot proceed until S-25.06's mechanism-A backfill also runs. But:

1. BC-1.18.011 Postcondition 7 already correctly states the two are "independent one-time operations that happen to be scheduled at the same F4 activation boundary."
2. Postcondition 7 further confirms "this migration has NO Cohort-B sequencing dependency."
3. The two migrations operate on entirely different artifact sets (BC-INDEX.md body for B2 vs. the four cycle append-log files for mechanism A).
4. No technical dependency exists: BC-1.18.011 depends on BC-1.18.010's end-state spec and BC-1.18.006's atomic-write primitives — both already delivered. It does NOT depend on BC-1.18.008's execution.

**Conclusion:** The "SAME moment" / "at the same F4 activation boundary" language in Precondition 4 is an over-constraint introduced by the original simultaneous-scheduling assumption, which has since been invalidated by the S-25.06 separation. Postcondition 7 already records the correct model. Precondition 4 must be amended to remove the coupling.

### 3.2 Exact BC-1.18.011 Amendment (for product-owner)

**File:** `.factory/specs/behavioral-contracts/ss-01/BC-1.18.011.md`

**Section:** `## Preconditions`, item 4

**Current text:**
> "The migration is scheduled to execute at F4 activation, at the SAME moment BC-1.18.008's
> mechanism-A backfill-split runs (both are one-time migrations gated on the same F4 activation
> boundary, though they operate on independent artifact sets and have no ordering dependency on
> each other per Postcondition 7)."

**Replacement text:**
> "The migration is scheduled to execute at F4 activation. It is an INDEPENDENT one-time
> operation and has no ordering dependency on BC-1.18.008's mechanism-A backfill-split
> (confirmed by Postcondition 7 and by the sequencing architecture: mechanism-A execution is
> wired and executed by S-25.06, a separate story with `blocks: [S-25.02]`; the B2 migration
> governed by this BC executes under its own cluster-5 delivery track). Both migrations are
> gated on F4 activation independently — neither is a precondition for the other."

**Additionally:** Add a new bullet to the `## Architecture Anchors` section:
> "- `ADR-052` — sanctioned `Bash`-tool invocation mechanism and permission policy for
>   native migration CLI execution (this migration's entry point in `shard_manager.rs` is
>   invoked via the `Bash`-tool-with-allowlist path specified in ADR-052)"

**Changelog entry** (product-owner to add):
> "| 1.1 | 2026-09-12 | product-owner | Cluster-5 F1 delta-analysis amendment (architect, `s2502-cluster5-f1-delta-analysis.md`): Precondition 4 simultaneous-activation over-constraint removed — replaced with independent-operation framing consistent with Postcondition 7 and the S-25.06 separation (CV-DIR-F1a). Architecture Anchor for ADR-052 added (CV-DIR-F2). E-SHD-005 taxonomy presence verified (Gap 3). |"

---

## 4. CV-DIR-F2 Architecture — Sanctioned Invocation Mechanism

### 4.1 The Contradiction (re-stated for completeness)

S-25.06 Rule 7 (and by extension BC-1.18.011's unstated invocation path) present an internally contradictory set of constraints:

- **Constraint A (correctness):** The migration executor must be a native binary (not WASM, not a hook plugin) — WASM fuel budgets are insufficient for multi-MB files (ADR-051; CLAUDE.md §WASM plugin fuel budgets).
- **Constraint B (auditability, original Rule 7 text):** The invocation must be "logged as an Edit/Write tool call" — implying it runs through the agent's Edit/Write surface that hooks validate.
- **Contradiction:** A native binary invoked from the CLI writes to the filesystem OUTSIDE the Edit/Write tool surface. A native binary invocation cannot simultaneously be an Edit/Write tool call. These constraints cannot both hold.
- **POL-3 scope:** POL-3/TD-FACTORY-HOOK-BYPASS-001 P0 prohibits Python/sed/echo bypasses and agent-role violations. It does NOT prohibit a purpose-built native binary that implements the full BC specification including content-preservation, census, crash-atomicity, and rollback.

### 4.2 Options Evaluated

**Option A — Dispatcher-internal hook-driven (migration fires inside a PreToolUse hook automatically):**
The migration logic runs as part of the dispatcher's native PreToolUse handling path, triggered by detecting the un-migrated state (e.g., BC-INDEX.md body still contains per-BC rows and the `shards/` directory is absent).

- Pro: No agent invocation needed; no permission-classifier question.
- Con (BLOCKING): One-time migrations require explicit human authorization at F4 activation. An automatically-triggered hook cannot satisfy the "F4 activation gate" that BC-1.18.011 mandates. A hook that fires automatically during normal dispatch could trigger the migration unexpectedly during any `Edit`/`Write` against BC-INDEX.md — including before BC-1.18.010's end-state is fully implemented and validated. This makes timing of the migration uncontrollable.
- Con: Not re-runnable interactively without triggering another tool call against BC-INDEX.md.
- Con: The state-manager's burst-log Dim-2 evidence requirement (D-449(a)) cannot be satisfied by a hook-internal migration — there is no stdout capture path.

**Option B — Narrowly-allowlisted Bash invocation with explicit permission grant (RECOMMENDED):**
The migration CLI (`factory-dispatcher migrate-bc-index` or equivalent subcommand) is invoked via the `Bash` tool. An explicit, narrowly-scoped permission entry in `.claude/settings.json` pre-authorizes this specific invocation under the operator's auto-mode permission classifier.

- Pro: Explicit human-authorized activation (the permission grant in settings.json IS the authorization; no ad-hoc permission prompt is needed at execution time).
- Pro: Auditable — the Bash command's stdout (census report) is captured by the session and recorded in the burst-log.
- Pro: Re-runnable and interactive — the operator can inspect the census report, re-run in dry-run mode, and confirm before committing.
- Pro: Satisfies the `D-449(a)` literal-shell-execution-evidence requirement — the census report stdout IS the evidence.
- Pro: Does NOT violate POL-3 — the native binary IS the hook-runtime's own shard management code, implementing the full BC-1.18.008/BC-1.18.011 specification; it is not a raw `cp` or `python -c` bypass.
- Pro: No new WASM concern — the binary runs natively outside any fuel budget.
- The `Bash` tool in Claude Code is NOT hook-validated by the same PreToolUse/PostToolUse chain as Edit/Write — it is validated by the session's permission classifier instead. A pre-authorized specific pattern bypasses the runtime permission prompt while preserving the audit trail.

**Option C — Real tool adapter (MCP server wrapping the migration CLI):**
An MCP server wraps the migration subcommand and exposes it as an agent tool.

- Con: Operational complexity (MCP server process management, authentication, port allocation) disproportionate to a one-time batch operation.
- Con: No existing MCP server infrastructure for internal factory binary operations.
- **Not recommended.**

### 4.3 Decision: Option B — Bash Tool with Allowlist

**Recommended:** Option B. ADR-052 (authored separately as `.factory/specs/architecture/decisions/ADR-052-native-migration-cli-bash-tool-allowlist-sanctioned-execution-path.md`) specifies this decision in full. Key elements:

**Permission policy:**
1. A named permission entry is added to `.claude/settings.json` (project-level) of the form:
   ```json
   { "type": "bash", "pattern": "*/factory-dispatcher migrate-bc-index*" }
   { "type": "bash", "pattern": "*/factory-dispatcher backfill-append-logs*" }
   ```
   (Exact pattern syntax to be confirmed against the installed version's allowlist schema; the principle is a narrow pattern matching only the specific subcommand, not a wildcard over arbitrary binary invocations.)
2. The allowlist entries are added by the devops-engineer as part of cluster-5 activation (T-1 equivalent) — they do NOT pre-exist the cluster-5 spec.
3. The allowlist is project-level (`.claude/settings.json`), not user-level (`~/.claude/settings.json`), so the authorization is repo-scoped and reviewable in PRs.

**Audit-trail design:**
1. The migration CLI MUST write a structured census report to stdout on successful completion. Minimum required fields: source file path, pre-split BC count (from `total_bcs` frontmatter), independent-census BC count, per-shard BC counts (including SS-05/SS-06 sub-shards), content-preservation hash (SHA-256 of source body), and idempotency status (first-run vs. already-migrated).
2. The state-manager records the captured stdout verbatim in the burst-log's Dim-2 attestation for the activation burst. This IS the D-449(a)-required literal-shell-execution evidence.
3. No separate log file is required — the session's stdout capture + burst-log entry is the canonical audit record.

**POL-3 compatibility:**
The native migration binary is NOT a raw bypass. It:
- Implements the FULL BC-1.18.011 specification: content-preservation verification, independent census, staging + verify + atomic replace, fail-loud rollback on failure, idempotency.
- Was written specifically for this purpose under VSDD's standard TDD/adversarial-convergence process.
- Performs filesystem writes via BC-1.18.006's atomic-write primitives (`write_atomic`, `write_indeterminate_marker`), the same sanctioned path every other .factory write uses.
The distinction: POL-3 targets unstructured, unvalidated writes (shell `cp`, `python -c open()`, `echo > file`) that bypass the spec. This binary IS the spec.

**Binary placement decision (for product-owner BC + story-writer story):**
The B2 migration entry point lives in `crates/factory-dispatcher`, co-located with `shard_manager.rs`. Rationale:
- `shard_manager.rs` already contains `run_mechanism_a_backfill_split`; the B2 migration follows the same code-ownership model.
- `last-amended-migrate` is governed by BC-10.13.001 (frontmatter `changelog:`/`last_amended` mutations only; TARGET_FILES = 5 indexes + STATE.md). Extending it for body-structure migrations would violate BC-10.13.001's scope without a BC amendment — unnecessary complexity.
- The `factory-dispatcher` binary already has a CLI dispatch layer; adding a `migrate-bc-index` subcommand is additive and follows the existing pattern.
This placement decision is an ARCHITECT DECISION and must be recorded in ADR-052 + reflected in BC-1.18.011's Architecture Anchor amendment (§3.2 above).

### 4.4 ADR-052 Status

ADR-052 has been authored at:
`.factory/specs/architecture/decisions/ADR-052-native-migration-cli-bash-tool-allowlist-sanctioned-execution-path.md`

State-manager must add the ARCH-INDEX row for ADR-052 in the same burst that registers this delta-analysis.

---

## 5. CV-DIR-F1b — S-25.06↔S-25.02 Blocks-Edge Regression Flag

Per the CV direction review findings and the task scope, the following is flagged for consistency-validator and story-writer follow-up. This flag does NOT block cluster-5 TDD.

**Finding:** S-25.06 v1.1 declares `blocks: [S-25.02]` (correct: cluster-7 Cohort-B flip is blocked on S-25.06 completion). S-25.02 v4.3 declares `blocks: [S-25.03]` in its frontmatter — which reflects S-25.02's outward dependency to S-25.03, not its inward blocking from S-25.06. The S-25.06 `blocks: [S-25.02]` edge is asymmetric: S-25.02's frontmatter does not reflect that it is blocked by S-25.06 for the cluster-7 sub-scope.

**Why this is acceptable in the current state:**
- S-25.02 is partially-merged (clusters 1-4 done). Its frontmatter `blocks: [S-25.03]` reflects the story's overall outward dependency chain, not intra-story cluster sequencing.
- The cluster-7-is-blocked-on-S-25.06 relationship is documented in S-25.06's body `Dependency Sequencing` block and flagged via the story-writer's FLAG comment.
- The state-manager was tasked (per S-25.06's FLAG) to add a reciprocal note to S-25.02's body in the same burst that registers S-25.06.

**Requested follow-up (no blocker):**
1. **Consistency-validator**: Verify that the S-25.06 `blocks: [S-25.02]` edge is consistent with S-25.02's cluster-7 task block's stated precondition. Flag any STORY-INDEX row that conflicts.
2. **Story-writer**: Confirm the reciprocal note in S-25.02's cluster-7 task block has been added per S-25.06's FLAG (if not yet done, add it in a follow-up burst). The note: "Cluster-7 (Cohort-B fail-closed flip) is blocked on S-25.06 completion. S-25.06 must execute the live backfill (Task T-10) and have its PR merged before cluster-7 can be dispatched."

---

## 6. Routing Plan — Ordered Downstream Actions

The following actions must be completed in the order listed before cluster-5 TDD dispatch. Gate-blocking actions are labeled **GATE**.

### Step 1 — Architect: Register ADR-052 [THIS BURST, state-manager handles]

State-manager adds ADR-052 row to ARCH-INDEX.md in the same commit burst that registers this delta-analysis. ADR-052 total_adrs counter: 51 → 52.

### Step 2 — Product-owner: Amend BC-1.18.011 [GATE — blocks TDD dispatch]

Apply the exact amendment from §3.2:
- Replace Precondition 4 text (remove simultaneous-activation language; substitute independent-operation framing)
- Add ADR-052 to Architecture Anchors
- Verify E-SHD-005 is present in `prd-supplements/error-taxonomy.md`; if absent, add it
- Bump BC-1.18.011 version: 1.0 → 1.1
- Add changelog entry (per §3.2)

### Step 3 — Product-owner: Amend BC-1.18.010 Invariant 2 [GATE — blocks TDD dispatch]

One-sentence addition to Invariant 2 specifying the config-embedded-snapshot-with-CI-parity-test mechanism (per §2.1 Gap 1). This is a minor clarifying amendment; no postcondition or behavioral change.

### Step 4 — Architect: ADR-051 §Decision 17 addendum [GATE — blocks TDD dispatch]

Add §Decision 17 to ADR-051 specifying the config-embedded snapshot implementation for the ARCH-INDEX BC-S-prefix→SS-NN mapping (per §2.1 Gap 1 recommendation). This is a new decision section in ADR-051, not a correction to an existing decision. Bump ADR-051 version accordingly.

### Step 5 — Devops-engineer: Add Bash allowlist entries to `.claude/settings.json` [GATE — blocks activation step T-10 equivalent]

Add the two narrowly-scoped Bash permission entries for `factory-dispatcher migrate-bc-index` and `factory-dispatcher backfill-append-logs` (per §4.3 Permission policy). These entries enable the operator's auto-mode permission classifier to execute the migration without a runtime prompt.

### Step 6 — Story-writer: Update S-25.02 cluster-5 task blocks + S-25.02 body amendment [non-blocking, do in same burst as Step 2]

1. Add the cluster-7 reciprocal note to S-25.02's body (the S-25.06 FLAG task, if not yet done).
2. Add a cluster-5 task block to S-25.02's `## Tasks` section listing the TDD scope: B2 artifact-shape handler, migration entry point, config snapshot + CI test, VP-127/VP-128/VP-132/VP-133/VP-134 test stubs.

### Step 7 — Formal-verifier: Allocate VP-127, VP-128, VP-132, VP-133, VP-134 in VP-INDEX (if not already allocated) [GATE — blocks test-writer dispatch]

Verify these VPs are present in VP-INDEX v3.03+ (they were allocated per BC-1.18.010/011 VP Anchors sections). If any are missing, allocate them and propagate per POLICY 9 to `verification-architecture.md` and `verification-coverage-matrix.md`.

### Step 8 — Test-writer: Author Red Gate stubs for VP-127, VP-128, VP-132, VP-133, VP-134 [GATE — blocks implementer dispatch]

Per story tdd_mode: strict, covering:
- VP-127: zero-manifest-read unit test (mock filesystem read-call counter for non-sub-sharded subsystem lookups)
- VP-128: single-authoritative-row integration test + ARCH-INDEX-sourced prefix mapping cross-reference
- VP-132: content-preservation proptest / golden-file round-trip
- VP-133: independent-census integrity + crash-atomicity fault-injection + idempotency + SS-05/SS-06 sub-split census (four obligations, consolidated per single-method-per-VP convention)
- VP-134: static-check — migration completion not referenced in hooks-registry.toml Cohort-B gating

### Step 9 — Implementer: B2 artifact-shape handler + migration entry point [TDD phase]

Per standard per-story Phase 3 sub-workflow. Must NOT begin before Steps 2-8 are complete. Key constraints:
- New `run_mechanism_b2_bc_index_split` function in `shard_manager.rs` using BC-1.18.006 atomic-write primitives
- Config-embedded ARCH-INDEX snapshot + CI parity test (per ADR-051 §Decision 17 / ADR-052)
- Census report to stdout (per ADR-052 audit-trail design)
- Second-level sub-sharding for SS-05/SS-06 in the SAME migration operation (BC-1.18.011 Postcondition 6)

### Step 10 — Activation: Execute migration via Bash tool [post-TDD, pre-cluster-7]

The actual one-time B2 migration execution, per the allowlisted Bash invocation path (ADR-052). State-manager records the census report stdout in the burst-log. This is analogous to S-25.06's T-10 but for the B2 case.

---

## 7. Spec-Readiness Verdict

| BC | Verdict | Blocking Gaps |
|----|---------|--------------|
| BC-1.18.010 v1.2 | **NEAR-READY** — blocked on ADR-051 §Decision 17 addendum + one-sentence Invariant 2 amendment (Steps 3-4). Both are minor. | §2.1 Gap 1 (MEDIUM) |
| BC-1.18.011 v1.0 | **NEEDS-CONVERGENCE** — blocked on Precondition 4 amendment (§3.2) + ADR-052 Architecture Anchor addition (Steps 2, 4, 5). | §2.2 Gap 1 (HIGH), §2.2 Gap 2 (HIGH) |

**Overall cluster-5 verdict: NEEDS-CONVERGENCE.** TDD dispatch is blocked until Steps 2-5 complete. Steps 6-7 can proceed in parallel with Steps 2-5. Steps 8-9 are gated on all prior steps.

---

## 8. Commit / INDEX / STATE Discipline

Per task scope and VSDD routing:
- This analysis file: authored by architect, NOT committed in this burst.
- ADR-052: authored by architect at `.factory/specs/architecture/decisions/ADR-052-native-migration-cli-bash-tool-allowlist-sanctioned-execution-path.md`, NOT committed in this burst.
- Neither ARCH-INDEX.md nor STATE.md nor any INDEX file was edited.
- State-manager handles all commits, ARCH-INDEX ADR-052 row, and STATE.md advancement in the downstream burst.
