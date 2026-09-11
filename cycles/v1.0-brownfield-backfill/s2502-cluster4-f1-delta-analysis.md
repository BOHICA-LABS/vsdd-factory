---
document_type: cycle-general-artifact
artifact_type: phase-delta-analysis
story_id: "S-25.02"
cluster: "4"
mechanism: "B1"
bc: "BC-1.18.009"
version: "1.0"
status: ready
producer: architect
timestamp: 2026-09-11T00:00:00Z
cycle: v1.0-brownfield-backfill
phase: F4
inputs:
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.009.md
  - .factory/specs/architecture/decisions/ADR-051-layer-2-two-mechanism-size-triggered-shard-rotation-append-logs-and-bc-index-sharding.md
  - crates/last-amended-migrate/src/rotate.rs
  - crates/last-amended-migrate/src/changelog.rs
  - crates/last-amended-migrate/src/lib.rs
  - crates/last-amended-migrate/Cargo.toml
  - crates/factory-dispatcher/src/shard_manager.rs
  - crates/factory-dispatcher/Cargo.toml
  - .factory/stories/S-25.02-artifact-sharding-layer2.md
  - .factory/specs/prd-supplements/error-taxonomy.md
input-hash: "f567563"
---

# S-25.02 Cluster-4 (Mechanism B1 Rotation, BC-1.18.009) — F1 Delta Analysis / Implementation-Readiness Assessment

## Executive Summary

**Readiness verdict: READY-FOR-TDD.**

BC-1.18.009 v1.5 is implementation-ready without a product-owner spec-refresh. The contract is
unambiguous across all postconditions, invariants, edge cases, and canonical test vectors. The
known prerequisite (additive library extension to `rotate_changelog`) is fully described in the
BC's own Architecture Anchors and SDK Grounding Evidence sections. The gate integration point is
pre-scaffolded in `shard_manager.rs` as a clearly labelled placeholder awaiting this cluster.

No routed findings of the NEEDS-SPEC-REFRESH class were found. One carry-forward annotation
for the **formal-verifier** is recorded in Section 4 (VP body re-review obligation deferred since
pass-2/pass-3 per the BC's own fix-burst notes — not a blocker for TDD, not architect domain to
fix).

The `factory-dispatcher → last-amended-migrate` workspace dependency edge is ALREADY SHIPPED
(cluster-2, PR #824, Cargo.toml comment documents this explicitly). No new Cargo.toml change is
needed for cluster-4.

---

## Section 1: Library Extension Design

### 1.1 Current shipped state (verified by code read)

`crates/last-amended-migrate/src/rotate.rs` exports one public function:

```
pub fn rotate_changelog(
    path: &Path,
    cycle_name: &str,
    keep_recent: usize,
    mode: MigrationMode,
) -> Result<RotationReport, MigrateError>
```

The private `fn resolve_archive_path(path: &Path, cycle_name: &str) -> Result<PathBuf, MigrateError>`
derives the archive destination as:
```
<nearest .factory ancestor>/cycles/<cycle_name>/<file-stem>-changelog-archive.md
```

This path derivation is correct for every existing cycle-scoped caller (all call with a literal
`"test-cycle"` or a real cycle name). It is WRONG for `BC-INDEX.md`, which has no natural
`cycle_name` — its archive belongs at a fixed sibling path
`.factory/specs/behavioral-contracts/BC-INDEX-changelog-archive.md`, not under any
`.factory/cycles/` subdirectory.

The BC's SDK Grounding Evidence section explicitly confirms this gap (the grep for the
`resolve_archive_path(path: &Path, cycle_name: &str)` signature confirms its ABSENCE of an
`archive_path` parameter) and states: "the generalized, explicit-`archive_path`-parameterized
call surface this BC's gate requires is a TO-BE-IMPLEMENTED, additive extension at F4."

### 1.2 Extension design — production-grade decision

**Decision: add a new public function `rotate_changelog_at`, refactor `rotate_changelog` as a
thin wrapper delegating to it.**

```rust
/// Rotate `path`'s `changelog:` sequence using a caller-supplied, pre-resolved
/// `archive_path` directly — for callers that cannot derive an archive path from
/// a `cycle_name` (e.g. BC-INDEX.md, which is a catalog artifact, not a
/// cycle-scoped log file).
///
/// Identical behaviour to `rotate_changelog` in all other respects: moves the
/// oldest items past `keep_recent` verbatim into `archive_path`, removes exactly
/// those items from `path`, and leaves a `changelog_archive:` discoverability
/// pointer (BC-10.13.001 PC5). No-op (EC-004) when the sequence does not exceed
/// `keep_recent`. Creates `archive_path`'s parent directory if it does not
/// already exist (EC-005 precedent).
pub fn rotate_changelog_at(
    path: &Path,
    archive_path: &Path,
    keep_recent: usize,
    mode: MigrationMode,
) -> Result<RotationReport, MigrateError>
```

`rotate_changelog` becomes a thin wrapper:

```rust
pub fn rotate_changelog(
    path: &Path,
    cycle_name: &str,
    keep_recent: usize,
    mode: MigrationMode,
) -> Result<RotationReport, MigrateError> {
    let archive_path = resolve_archive_path(path, cycle_name)?;
    rotate_changelog_at(path, &archive_path, keep_recent, mode)
}
```

`rotate_changelog_at` contains the current body of `rotate_changelog` from the point AFTER
`resolve_archive_path` is called (lines ~144–208 in rotate.rs). The entire `RotationReport`
construction, `doc` parse, `total`/`keep_items`/`move_items` split, `Check` mode early return,
`create_dir_all`, archive-read-and-append, `yaml_guard` validation, `write_atomic` for both
files, and the discoverability pointer — all reused verbatim. Zero new rotation/trim/validate/
write logic is introduced anywhere.

**Rationale for this design over alternatives:**

- Changing `rotate_changelog`'s own signature (replacing `cycle_name: &str` with `archive_path:
  &Path`) would break all existing callers: the integration tests in
  `crates/last-amended-migrate/tests/` call `rotate_changelog(&path, "test-cycle", ...)` — all
  would fail to compile. Ruled out.
- Adding `archive_path: Option<&Path>` alongside `cycle_name: &str` creates a mixed-purpose
  signature requiring callers to pass `None` explicitly; the BC's own framing ("a generalized
  `archive_path: &Path` parameter or equivalently-named `resolve_archive_path_at`") favours
  the clean-name variant. Ruled out.
- Adding a new standalone public function with a clear name preserves full backward compatibility,
  makes the new entry-point's intent explicit at every call site, and avoids any new `Option`-
  threading in the existing function's signature.

### 1.3 Existing callers — confirmed unaffected

All existing callers of `rotate_changelog` pass a literal `cycle_name: &str`:

- `crates/last-amended-migrate/tests/bc_10_13_001_pc5_rotation_test.rs`: four call sites, all with
  `"test-cycle"` or `"brand-new-cycle"` — unchanged, compile against the thin-wrapper signature
  which is byte-identical in parameter types.
- `crates/last-amended-migrate/tests/bc_10_13_001_sec003_atomic_write_test.rs`: one call site,
  `"test-cycle"` — unchanged.
- No callers exist outside `last-amended-migrate/tests/`.

### 1.4 `lib.rs` export addition required

`crates/last-amended-migrate/src/lib.rs` line 50 currently reads:

```rust
pub use rotate::{RotationReport, rotate_changelog};
```

This must become:

```rust
pub use rotate::{RotationReport, rotate_changelog, rotate_changelog_at};
```

`shard_manager.rs` imports `last_amended_migrate::rotate::rotate_changelog_at` (or via the
crate-root re-export). Either import path is valid; using the crate-root re-export is consistent
with how `write_atomic` is imported in the existing cluster-2 code
(`last_amended_migrate::atomic_write::write_atomic`).

---

## Section 2: Workspace Dependency Edge

**The `factory-dispatcher → last-amended-migrate` dependency edge is ALREADY SHIPPED.**

`crates/factory-dispatcher/Cargo.toml` already contains:

```toml
last-amended-migrate = { path = "../last-amended-migrate", version = "0.0.1" }
```

This edge was added in cluster-2 (PR #824) for `write_atomic` reuse. The comment in that
Cargo.toml explicitly references BC-1.18.006 and includes the ADR-051 §8 dependency-acyclicity
acknowledgement: "factory-dispatcher → last-amended-migrate is the ONLY allowed direction of this
workspace-internal edge; last-amended-migrate MUST NOT gain a reverse dependency on
factory-dispatcher."

**Acyclicity verification:** `crates/last-amended-migrate/Cargo.toml` depends only on `clap`,
`thiserror`, `serde`, and `serde_norway` (all workspace dependencies). It has no dependency on
`factory-dispatcher` or any of the `sink-*` crates. The edge is provably acyclic.

**No new Cargo.toml change is required for cluster-4.** The existing edge already makes
`last_amended_migrate::rotate::rotate_changelog_at` (once added) available in
`shard_manager.rs` without any further workspace configuration.

---

## Section 3: Gate Integration Point

### 3.1 Where in `shard_manager.rs` cluster-4 wires in

The `FrontmatterChangelogArray` shape handler is in the `check_shard_cap` function's
`match entry.shape_resolved` block, starting at line ~2023. The current trigger-fired
branch (lines ~2081–2102) is:

```rust
if item_count_trigger_fires(current_item_count, n) {
    tracing::warn!(
        artifact_stem = %entry.artifact_stem,
        current_item_count,
        n,
        "BC-1.18.005: item-count shard-cap trigger fired; rotate/block outcome is \
         owned by BC-1.18.009 (not yet implemented in this cluster) — allowing \
         the call to proceed"
    );
}

HookResult::Continue
```

The module-level doc comment (lines 55–75) explicitly flags this as the pending cluster-4
hand-off: "BC-1.18.009 (the observable rotate/block-and-retry outcome once the item-count
trigger fires) and BC-1.18.012 (the one-time changelog backfill migration) remain LATER
clusters and are still explicitly OUT OF SCOPE here."

**Cluster-4 replaces the entire trigger-fired branch with:**

```rust
if item_count_trigger_fires(current_item_count, n) {
    // BC-1.18.009 Postcondition 2: rotate-then-block-and-retry.
    // The archive path is the fixed, non-cycle sibling determined by the BC's
    // contract: .factory/specs/behavioral-contracts/BC-INDEX-changelog-archive.md.
    // The dispatcher pre-computes this from the canonical target path's parent
    // directory — it is NOT derived from cycle_name (BC-1.18.009 Architecture
    // Anchors: "BC-INDEX.md has no natural cycle_name value").
    let archive_path = target_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("BC-INDEX-changelog-archive.md");

    let keep_recent = resolved_low_water_mark(n, entry.low_water_mark) as usize;

    match last_amended_migrate::rotate::rotate_changelog_at(
        target_path,
        &archive_path,
        keep_recent,
        last_amended_migrate::MigrationMode::Apply,
    ) {
        Ok(_report) => HookResult::Block {
            reason: build_b1_block_reason(
                &entry.artifact_stem,
                &archive_path,
                keep_recent,
            ),
        },
        Err(e) => HookResult::Error {
            message: format!(
                "E-SHD-004: rotate_changelog invocation failed for \"{}\": {e}",
                entry.artifact_stem
            ),
        },
    }
} else {
    HookResult::Continue
}
```

A `build_b1_block_reason` helper (pure, analogous to `build_roll_retry_block_reason` for
mechanism A) constructs the retry message prescribed by BC-1.18.009 Postcondition 2 step 3:

> "BC-INDEX.md's `changelog:` sequence was rotated to make room (oldest item(s) appended to
> `<archive_path>`); the frontmatter now has `<keep_recent>` items. Retry your write: if you
> used `Edit`, reissue as a fresh `Write` or a fresh `Edit` re-read against the current
> (post-rotation) file, since your original `old_string`/`new_string` pair may no longer
> match; if you used `Write`, recompute your `content` payload against the current
> (post-rotation) file before retrying — do not resubmit your original payload unchanged,
> since it reflects pre-rotation state."

### 3.2 Shape-field dispatch path (from cluster-1)

The dispatch on `entry.shape_resolved` (a `ShardShape` enum) already has both arms:

- `ShardShape::Flat` — cluster-2's roll-before-write path (fully implemented, tests green)
- `ShardShape::FrontmatterChangelogArray` — cluster-4's rotate-and-retry path (currently stub)

The `item_count_trigger_fires(current_item_count, n)` call, `read_changelog_item_count(target_path)`
call, `resolved_low_water_mark(n, entry.low_water_mark)` call, and the `validate_entry` guard
(which fail-louds on a missing `n` for this shape — `ShardConfigError::MissingN`) are ALL already
implemented and tested. Cluster-4 adds only the trigger-fired consequence: the rotate call and
the Block/Error result. No new trigger-boundary or validation logic is introduced.

### 3.3 Trigger condition cross-check (BC-1.18.005 Postcondition 8)

The shipped `item_count_trigger_fires` function (line ~1497):

```rust
pub fn item_count_trigger_fires(current_item_count: u64, n: u64) -> bool {
    current_item_count.saturating_add(1) > n
}
```

This correctly implements BC-1.18.005 Postcondition 8's `current_item_count + 1 > N` condition
(EC-008 off-by-one: N-1 items → false; exactly N items → true). Cluster-4 uses this function as-
is — no change.

### 3.4 Error code `E-SHD-004`

Per `error-taxonomy.md` v1.13: `E-SHD-004 | rotate_changelog invocation failed for "<artifact>": <io-error>`
(BC-1.18.009 EC-003). The v1.13 changelog explicitly confirms: "E-SHD-004/E-SHD-005 NOT YET
IMPLEMENTED anywhere in the workspace — `grep -rn "E-SHD-004\|E-SHD-005" --include="*.rs"
crates/` returns zero hits." Cluster-4 introduces `E-SHD-004` for the first time. No new error
enum variant is required in `shard_manager.rs` — the error message is composed inline as a
`HookResult::Error { message: format!("E-SHD-004: rotate_changelog invocation failed for
\"{}\": {e}", ...) }`, consistent with how BC-1.18.006's fail-loud paths compose their messages
before they were refactored into `ShardRollError` variants. If the implementer judges a dedicated
`ShardB1RotateError` enum variant warranted for uniformity with the `ShardRollError` pattern, that
is an implementation-level decision within scope — the message format from `error-taxonomy.md` is
the authoritative contract, not the enum shape.

---

## Section 4: Spec-Refresh Gate

**Verdict: NO SPEC REFRESH NEEDED — BC-1.18.009 v1.5 is implementation-ready AS-IS.**

Assessed against the D-1174 D-1170 precedent (cluster-2's product-owner spec-refresh from
BC-1.18.006 v1.3→v1.4 before TDD):

1. **Postconditions** — all unambiguous. Postcondition 1 (low_water_mark trim, never N-1),
   Postcondition 2 (rotate-only, Block, retry instruction), Postcondition 5 (single evergreen
   archive), Postcondition 6 (E-SHD-004 on failure) are each fully specified with no conditional
   placeholders or "pending review" language.

2. **Invariants** — all unambiguous. Invariant 1 (zero `prepend_changelog_item` call sites in
   gate), Invariant 4 (Block is the ONLY outcome after successful rotation — never Continue) are
   load-bearing, static-checkable, and directly verifiable by VP-126.

3. **Edge cases** — EC-001 through EC-007 are concrete and cover every scenario the TDD chain
   needs to drive tests.

4. **Canonical test vectors** — the five CTVs supply exact numeric inputs
   (N=50, `low_water_mark=25`) and expected outcomes, including the amortized-cadence vector
   (24 writes via plain Continue after a rotation).

5. **SDK Grounding Evidence** — the grep-based evidence in the BC correctly describes the
   CURRENT shipped state (no `archive_path` parameter exists) and explicitly defers confirmation
   of the extension to F4 ("implementer re-runs this grep at F4 and replaces this block with
   confirmation"). No stale or contradictory citation was found.

6. **The BC's fix-burst notes** flag that VP-125/VP-126/VP-131 bodies have not been re-reviewed
   against the v1.5 corrected contract since the v1.2 fix-burst (VP body edits are
   formal-verifier's domain per the BC's own notation). This is a **carry-forward annotation
   for formal-verifier** only, not a blocker for TDD: the VP-INDEX allocations (VP-125/126/131)
   are in place, the VP-to-BC traces are correct, and the proof methods (proptest/static-
   analysis/unit-test) are correctly specified. The implementer can write red-gate tests directly
   from the BC contract and CTVs without waiting for VP body reconciliation.

**No routed finding to product-owner.** The one architectural clarification required for
implementation (the precise archive path derivation for `BC-INDEX.md`) is fully answered in
Section 3.1 of this document and in the BC's Architecture Anchors — the dispatcher pre-computes
it as `target_path.parent() / "BC-INDEX-changelog-archive.md"`, i.e. the fixed, non-cycle
sibling path the BC specifies.

**Carry-forward annotation for formal-verifier (not a TDD blocker):**
VP-125, VP-126, and VP-131 body files should be re-reviewed against BC-1.18.009 v1.5 before
the cluster-4 local adversary gate run to confirm the proptest strategy (VP-125), static-analysis
grep pattern (VP-126), and unit-test fault-injection wiring (VP-131) correctly reflect the
v1.5 corrected contract (single-actor block-and-retry, `low_water_mark` trim target, E-SHD-004
error code). Route to `vsdd-factory:formal-verifier` at the formal-hardening step, not before.

---

## Section 5: Impact Boundary and Scope

### 5.1 Files touched by cluster-4

| File | Change type | Description |
|------|-------------|-------------|
| `crates/last-amended-migrate/src/rotate.rs` | Modify | Add `pub fn rotate_changelog_at(path, archive_path, keep_recent, mode)` containing the current body of `rotate_changelog` from post-`resolve_archive_path` onward; refactor `rotate_changelog` as a thin wrapper calling `resolve_archive_path` then `rotate_changelog_at` |
| `crates/last-amended-migrate/src/lib.rs` | Modify | Add `rotate_changelog_at` to the `pub use rotate::{...}` re-export line |
| `crates/factory-dispatcher/src/shard_manager.rs` | Modify | Replace the `FrontmatterChangelogArray` trigger-fired branch's `tracing::warn! + Continue` stub with the `rotate_changelog_at` call, `build_b1_block_reason` construction, and `Block`/`Error` return; update module-level doc "Scope note" to mark BC-1.18.009 as implemented (analogous to how cluster-2 updated the note for BC-1.18.006) |
| `crates/factory-dispatcher/tests/bc_1_18_009_b1_rotate_test.rs` | New | Integration test suite for the B1 rotate/block/retry outcome, paralleling `bc_1_18_006_roll_test.rs` |
| Optionally: `crates/last-amended-migrate/tests/bc_10_13_001_pc5_rotation_test.rs` | Modify | Add tests for `rotate_changelog_at` with an explicit archive path (parallel to the existing `rotate_changelog` tests) |

**No Cargo.toml changes are needed.** The `factory-dispatcher → last-amended-migrate` edge is
already in `crates/factory-dispatcher/Cargo.toml`. The `last-amended-migrate` crate's own
Cargo.toml is unchanged.

### 5.2 ACs discharged by cluster-4

| AC | Title | Traces to |
|----|-------|-----------|
| AC-015 | B1 single-actor block-and-retry via `rotate_changelog` only — never a gate-side prepend | BC-1.18.009 PC2, PC3, Inv-1, Inv-4 |
| AC-016 | B1 fail-loud rotation failure and no-history-loss archive | BC-1.18.009 PC5, PC6, EC-003 |

ACs NOT in scope for cluster-4 (later clusters):
- AC-019 (BC-1.18.012 cold-start backfill migration — T-9, later cluster)
- AC-017/AC-018 (mechanism B2 — T-10/T-11, later clusters)
- AC-020/AC-021 (Cohort B fail-closed flip — T-12, final cluster)

### 5.3 Verification properties in scope

| VP | Property | Proof method | Status |
|----|----------|-------------|--------|
| VP-125 | Bounded-live-sequence (never exceeds N) + no-history-loss (every item recoverable from live or single archive) | proptest — two facets; arbitrary prepend sequences | Bodies flagged for formal-verifier re-review against v1.5 contract (see §4) |
| VP-126 | No-reimplementation + zero `prepend_changelog_item` call sites in B1 handler | static-analysis / CI grep | VP-126's grep pattern MUST include the B1 handler scope specifically; CI-enforceable after cluster-4 |
| VP-131 | Fail-loud `rotate_changelog` failure returns `HookResult::Error (E-SHD-004)`, never Block/Continue; frontmatter byte-identical to pre-rotation state | unit test — injected `rotate_changelog_at` failure (disk-full/permission) | Bodies flagged for formal-verifier re-review |

### 5.4 RED-Gate obligations for test-writer

The test-writer must produce red-gate tests for every non-trivial function introduced or
modified by cluster-4. `tdd_mode: strict` is in effect across all of S-25.02.

Key red-gate scenarios (from BC-1.18.009 edge cases and CTVs):

| Scenario | Expected RED behavior (pre-implementation) |
|----------|---------------------------------------------|
| EC-001: `changelog:` at exactly N items, agent write arrives | `item_count_trigger_fires` returns true; `rotate_changelog_at` not yet called → stub fails |
| EC-002: `changelog:` well under N | `Continue` — passes via existing cluster-1 logic; no new stub needed |
| EC-003: `rotate_changelog_at` fails (injected error) | `HookResult::Error` with `E-SHD-004` prefix |
| EC-007: item count reaches N again after a prior rotation (amortized re-trigger) | Same Block-and-retry as EC-001 — same code path |
| CTV happy-path: N=50, `low_water_mark=25`, rotate fires | Archive has 25 items; live sequence has 25; `Block` returned |
| CTV amortized-cadence: 24 writes after rotation land via `Continue` | No block; sequence grows 26→49 without re-triggering |
| `rotate_changelog_at` called with `archive_path = sibling location` | `RotationReport.archive_path == expected_sibling` |

Stub-architect scope: stub `rotate_changelog_at` in `rotate.rs` with `todo!()` body; stub the
trigger-fired branch replacement in `shard_manager.rs`'s `FrontmatterChangelogArray` arm with
`todo!()`.

**GREEN-BY-DESIGN exception** (inherits the cluster-2 precedent): `build_b1_block_reason` is
a pure, zero-branching string template function. The stub-architect may provide a real
implementation directly, analogous to cluster-2's `build_roll_retry_block_reason`. The
test-writer must still write a test verifying the message format against the Postcondition 2
step-3 contract, even for a GREEN-BY-DESIGN function.

---

## Section 6: BC-INDEX.md Self-Referential Hazard

BC-INDEX.md is simultaneously the artifact being rotated by this BC AND the live catalog that
state-manager mutates on every burst (BC-1.18.009 Precondition 2 and BC-1.18.012's dependency
relationship).

**Bootstrapping concern — ACKNOWLEDGED, NOT BLOCKING:**

Between cluster-4's merge and BC-1.18.012's cold-start migration run (~1,997 existing
`changelog:` items not yet trimmed), every state-manager write to BC-INDEX.md's frontmatter
that hits the item-count trigger (N = configured value, provisional ~50) will fire the B1
rotation path. This means the very first write after cluster-4 deploys will almost certainly
trigger rotation and return `Block` with a retry instruction — since 1,997 >> N for any
reasonable N.

This is NOT a defect of cluster-4's design. It is the explicitly acknowledged pre-BC-1.18.012
cold-state behavior the story documents: "Three governed one-time migrations are MANDATORY
companions, not follow-ups." The retry round-trip is deterministic and recoverable; agents
following the BC-1.18.009 retry instruction (re-read the post-rotation file, recompute payload,
resubmit) will succeed on the second attempt.

**Ordering recommendation (not a cluster-4 gate):** the cluster ordering in the story schedule
(cluster-4 merged, then cluster-9 implementing BC-1.18.012) should be respected. Cluster-4
MUST NOT be held waiting for cluster-9 — they are sequential by design — but operators should
be aware that in the window between cluster-4 merge and the BC-1.18.012 migration run, every
BC-INDEX.md write involving a large `changelog:` will block-and-retry at least once.

**No bootstrapping deadlock exists.** The `rotate_changelog_at` call writes directly to the
filesystem using `write_atomic`, not through the dispatcher's own hook chain. The gate fires on
the AGENT's write tool call (Edit/Write/MultiEdit), not on the rotation's own internal writes.
There is no recursive trigger.

**Self-referential integrity note for stub-architect/test-writer:** integration tests for
cluster-4 should NOT use the live `.factory/specs/behavioral-contracts/BC-INDEX.md` fixture —
they should use a `tempfile`-based fixture, as all existing shard-manager integration tests do.
The single-evergreen-archive invariant (Postcondition 5) means the test must verify that
repeated rotations APPEND to the same archive path rather than creating new files.

---

## Section 7: Regression Safety

**Develop base commit:** `08ad44b5` (PR #831, cluster-3 merged — BC-1.18.007+008 mechanism-A
backfill).

**Branching instruction:** the cluster-4 TDD worktree MUST branch from `origin/develop`. Verify
`git rev-parse origin/develop` matches `08ad44b5` (or its successor if develop has advanced)
before stub-architect commit.

**Regression net:**
- `cargo test --workspace --all-targets` — full workspace test suite, must stay green
- `cargo clippy --workspace --all-targets -- -D warnings` — zero new warnings
- `cargo fmt --check --all` — no formatting changes in the diff
- `plugins/vsdd-factory/tests/run-all.sh` — Bats integration suite

**Zero-regression confidence on cluster-1/2/3 suites:**
- `bc_1_18_005_shard_cap_trigger_test.rs` — no changes to `item_count_trigger_fires`,
  `read_changelog_item_count`, or `validate_entry`. Zero impact.
- `bc_1_18_006_roll_test.rs` — no changes to `execute_roll`, `ShardRollError`, or the `Flat`
  shape arm. Zero impact.
- `bc_10_13_001_pc5_rotation_test.rs` — `rotate_changelog`'s public signature is unchanged (same
  parameters, same return type). The function body becomes a thin wrapper delegating to
  `rotate_changelog_at`; all existing tests exercise the wrapper via the same call shape and
  must produce the same results.

---

## Section 8: Implementation Checklist for TDD Chain

The following checklist captures the exact implementation obligations for each agent in the
cluster-4 TDD chain, derived from all analysis above. Pass this to stub-architect/test-writer/
implementer verbatim.

### Stub-architect obligations

1. In `crates/last-amended-migrate/src/rotate.rs`:
   - Add `pub fn rotate_changelog_at(path: &Path, archive_path: &Path, keep_recent: usize, mode: MigrationMode) -> Result<RotationReport, MigrateError>` with `todo!()` body.
   - Refactor `pub fn rotate_changelog` to call `resolve_archive_path(path, cycle_name)?` and then `rotate_changelog_at(path, &archive_path, keep_recent, mode)` — this is NOT a `todo!()` body since it is pure delegation wiring (WIRING-EXEMPT precedent from BC-1.18.006 cluster-2).

2. In `crates/last-amended-migrate/src/lib.rs`:
   - Add `rotate_changelog_at` to the `pub use rotate::{...}` re-export line.

3. In `crates/factory-dispatcher/src/shard_manager.rs`:
   - Add `fn build_b1_block_reason(artifact_stem: &str, archive_path: &Path, keep_recent: usize) -> String` — GREEN-BY-DESIGN (pure string template, zero branching), real implementation provided directly.
   - Replace the `FrontmatterChangelogArray` trigger-fired branch's `tracing::warn! + HookResult::Continue` with a `todo!()` placeholder that compiles. The outer `HookResult::Continue` after the trigger block remains until the implementer replaces the entire branch.
   - Update the module-level "Scope note" section (lines ~47–75) to reflect that BC-1.18.009 is now in-cluster scope (mirror how cluster-2 updated the scope note for BC-1.18.006).

4. Red Gate density check: the trigger-fired branch stub (once expanded) must drive at least one new failing test per BC-5.38.001's `>=0.5` density requirement.

### Test-writer obligations

New test file `crates/factory-dispatcher/tests/bc_1_18_009_b1_rotate_test.rs`:

- **Test 1 (happy-path, EC-001/CTV):** fixture `BC-INDEX.md` with N=50 items, `low_water_mark=25`
  configured; simulate an Edit/Write/MultiEdit dispatch; assert `HookResult::Block` returned
  with message containing `low_water_mark` item count and archive path name; assert archive file
  exists and contains the expected overflow items; assert live sequence trimmed to 25 items.
- **Test 2 (below-threshold, EC-002):** fixture with 10 items, N=50; assert `HookResult::Continue`.
- **Test 3 (rotation failure, EC-003/CTV):** inject an unwritable archive path (permission-denied
  temp dir); assert `HookResult::Error` with message starting `"E-SHD-004: rotate_changelog
  invocation failed for"`; assert frontmatter unchanged.
- **Test 4 (amortized re-trigger, EC-007/CTV):** after a rotation that trims to 25 items, assert
  that 24 subsequent writes land via `Continue`; assert the 25th write (bringing count to 50)
  re-triggers `Block`.
- **Test 5 (no `prepend_changelog_item` call in B1 handler):** VP-126 static check — assert
  `grep -c "prepend_changelog_item" shard_manager.rs` returns `0` from within the B1 handler
  scope (can be implemented as a doc-level assertion that the implementer confirms, or as a
  build-time CI check noted in the PR template).
- **Test 6 (`build_b1_block_reason` format):** assert the retry message matches Postcondition 2
  step 3's prescribed text (archive path name, `low_water_mark` item count, Edit/Write
  differentiated instruction).

Existing tests in `bc_10_13_001_pc5_rotation_test.rs` must stay green without modification —
no test file changes unless a new `rotate_changelog_at`-specific test is added (optional but
recommended for completeness: one test calling `rotate_changelog_at` with an explicit
`archive_path` that is a non-cycles sibling path, verifying the report's `archive_path` field
matches).

### Implementer obligations

1. `rotate_changelog_at` body: extract from current `rotate_changelog` body (lines ~144–208),
   replacing `archive_path` variable from `resolve_archive_path(path, cycle_name)?` with the
   caller-supplied `archive_path: &Path` parameter. Every other line verbatim — no new logic.
2. `shard_manager.rs` `FrontmatterChangelogArray` trigger-fired branch: implement per Section 3.1
   above. Specifically:
   a. Compute `archive_path` as `target_path.parent().unwrap_or(Path::new(".")).join("BC-INDEX-changelog-archive.md")`.
   b. Compute `keep_recent` as `resolved_low_water_mark(n, entry.low_water_mark) as usize`.
   c. Call `last_amended_migrate::rotate::rotate_changelog_at(target_path, &archive_path, keep_recent, last_amended_migrate::MigrationMode::Apply)`.
   d. On `Ok(_)`: return `HookResult::Block { reason: build_b1_block_reason(&entry.artifact_stem, &archive_path, keep_recent) }`.
   e. On `Err(e)`: return `HookResult::Error { message: format!("E-SHD-004: rotate_changelog invocation failed for \"{}\": {e}", entry.artifact_stem) }`.
3. Update the module scope-note doc comment to mark BC-1.18.009 as implemented.
4. VP-126 structural enforcement: after implementation, confirm `grep -c "prepend_changelog_item" crates/factory-dispatcher/src/shard_manager.rs` returns 0 (i.e. the function is imported in `last-amended-migrate` only, never referenced in dispatcher code). This grep is the VP-126 static-analysis assertion.

---

## Appendix: Verified Current-State Greps

The following greps were run during analysis to ground this document's claims. They record
the AS-OF-`08ad44b5` state, not target state.

```
$ grep -oE "^pub fn rotate_changelog" crates/last-amended-migrate/src/rotate.rs
pub fn rotate_changelog
```

Confirms: only ONE public rotation function exists; `rotate_changelog_at` is absent.

```
$ grep -oE "^fn resolve_archive_path" crates/last-amended-migrate/src/rotate.rs
fn resolve_archive_path
```

Confirms: `resolve_archive_path` is private (`fn`, not `pub fn`); no `archive_path`-parameterized
variant exists.

```
$ grep -c "last-amended-migrate" crates/factory-dispatcher/Cargo.toml
1
```

Confirms: the `factory-dispatcher → last-amended-migrate` dependency edge is present exactly once.

```
$ grep -c "factory-dispatcher" crates/last-amended-migrate/Cargo.toml
0
```

Confirms: `last-amended-migrate` has no reverse dependency on `factory-dispatcher` — acyclic.

```
$ grep -c "rotate_changelog" crates/factory-dispatcher/src/shard_manager.rs
0
```

Confirms: no call to `rotate_changelog` or `rotate_changelog_at` exists yet in
`shard_manager.rs` — cluster-4 introduces the first such call site.

```
$ grep -c "E-SHD-004" crates/factory-dispatcher/src/shard_manager.rs
0
```

Confirms: `E-SHD-004` is not yet emitted anywhere in the workspace; cluster-4 introduces it.

---

*Produced by `vsdd-factory:architect`. Do NOT edit STATE.md, BC/ADR files, BC-INDEX/STORY/VP/ARCH
indexes, code, or tests — those are other agents' domains. State-manager records the phase
transition after this document is committed.*
