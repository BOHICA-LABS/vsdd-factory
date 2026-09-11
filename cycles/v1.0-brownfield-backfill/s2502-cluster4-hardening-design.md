---
document_type: cycle-general-artifact
artifact_type: hardening-design
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
  - .factory/specs/behavioral-contracts/ss-10/BC-10.13.001.md
  - .factory/specs/architecture/decisions/ADR-051-layer-2-two-mechanism-size-triggered-shard-rotation-append-logs-and-bc-index-sharding.md
  - .factory/cycles/v1.0-brownfield-backfill/s2502-cluster4-f1-delta-analysis.md
  - crates/last-amended-migrate/src/rotate.rs
  - crates/factory-dispatcher/src/shard_manager.rs
input-hash: "2e9bf16"
---

# S-25.02 Cluster-4 (Mechanism B1) — Hardening Design + Spec-Impact Assessment

Observations A and B were surfaced during the LOCAL 3-CLEAN adversary cascade (non-blocking,
classified as OBSERVATIONS). The human has directed both be fixed before demo/PR. This document
is DESIGN-ONLY: it resolves the production-grade answer for each observation, specifies exact
spec-change intent for downstream specialists, and assigns routing. No BC, ADR, code, test,
index, or STATE.md file is edited here.

---

## Observation A: report.mutated / No-Op-After-Fired-Trigger

### Problem Statement (grounded in code)

The B1 handler in `shard_manager.rs` has a structural unchecked assumption:

```rust
match last_amended_migrate::rotate::rotate_changelog_at(...) {
    Ok(_report) => HookResult::Block { reason: build_b1_block_reason(...) },
    Err(e)      => HookResult::Error { message: format!("E-SHD-004: ...") },
}
```

`Ok(_report)` is matched unconditionally, discarding `RotationReport`. The report exposes:

```rust
pub struct RotationReport {
    pub path: PathBuf,
    pub archive_path: PathBuf,
    pub items_moved: usize,
    pub mutated: bool,   // false iff the no-op guard fired (total <= keep_recent)
}
```

`rotate_changelog_at` returns `Ok(RotationReport { mutated: false, items_moved: 0 })` when
its line-scan counter (`doc.changelog_items_raw.len()`, from `parse_frontmatter`) finds
`total <= keep_recent` — the EC-004 no-op guard at `rotate.rs` line 150-158.

The item-count trigger uses an entirely different mechanism: `read_changelog_item_count`
(serde deserialization via `serde_norway` of the YAML array, `shard_manager.rs` ~L1506-1523).

**Divergence scenario:** Trigger fires (serde count >= N). `rotate_changelog_at` is called.
Its own `parse_frontmatter` line-scan counts `total <= keep_recent`. It returns
`Ok(RotationReport { mutated: false })`. The handler emits `HookResult::Block` with a retry
message claiming "BC-INDEX.md's `changelog:` sequence was rotated to make room... now has
`keep_recent` items. Retry your write." This is a FALSE claim: the file was not mutated.

The agent retries. The trigger re-fires (serde count is still >= N — nothing changed). Another
false Block is emitted. **Self-DoS on BC-INDEX.md for the duration of the session** — every
Edit/Write/MultiEdit against BC-INDEX.md's frontmatter produces a spurious block+retry loop
that never converges.

**Provability caveat (from observation text):** On canonical `  - date:` format today, the
two counting methods agree exactly, so this scenario is latent, not currently triggerable.
However, the divergence is an architectural coupling (two independent parsing paths) that
has no load-bearing test preventing it from becoming real.

### Production-Grade Design Decision

**Option (b) — unify the two counters** is rejected. `read_changelog_item_count` uses a
dedicated bounded-read ceiling (`MAX_CHANGELOG_TARGET_READ_BYTES`), opportunistic CRLF
tolerance, malformed-frontmatter fail-loud path (EC-021), and a permissive-Ok(0) escape for
fenceless files (EC-014/EC-020). Replacing it with `parse_frontmatter` would drop all of
those carefully-spec'd behaviors. The correct fix targets the handler, not the trigger.

**Option (a) — check `report.mutated` after `Ok(report)` and return `Error` if false** is the
production-grade choice:

```rust
match last_amended_migrate::rotate::rotate_changelog_at(...) {
    Ok(report) if !report.mutated => HookResult::Error {
        message: format!(
            "E-SHD-008: rotate_changelog_at returned Ok(mutated=false) after item-count \
             trigger fired for \"{}\": trigger (serde) counts >= N but rotate_changelog_at \
             line-scan found total <= keep_recent — counter-method divergence; \
             manual inspection of {} required",
            entry.artifact_stem,
            target_path.display()
        ),
    },
    Ok(_report) => HookResult::Block {
        reason: build_b1_block_reason(&entry.artifact_stem, &archive_path, keep_recent),
    },
    Err(e) => HookResult::Error {
        message: format!("E-SHD-004: rotate_changelog invocation failed for \"{}\": {e}",
            entry.artifact_stem),
    },
}
```

This is minimal (one `if` guard), self-documenting, and fail-loud — the production-grade
default per CLAUDE.md. It surfaces the counter divergence immediately rather than papering
over it with a false Block.

### RotationReport Field Assessment

`RotationReport` **already has** `mutated: bool` (rotate.rs line 20). No new field is needed.
The `items_moved: usize` field (line 19) could provide additional diagnostic depth in the error
message, but the boolean gate alone is sufficient for correctness. The implementer MAY include
`items_moved` in the error message for diagnostics; the BC's contractual obligation is the
`Error` return, not the specific message body.

### Does This Warrant a New Error Code?

YES. `E-SHD-004` is already allocated for "`rotate_changelog` invocation FAILED" (Err arm,
I/O or validation error). A no-op-after-trigger is semantically distinct: `rotate_changelog_at`
returned `Ok`, not `Err`. Assigning the same code to two different failure surfaces would
prevent operators from distinguishing a filesystem failure from a counter-divergence bug.

New code: **`E-SHD-008`** = "`rotate_changelog_at` returned Ok(mutated=false) after
item-count trigger fired for `<artifact>`: counter-method divergence".

(`E-SHD-006` and `E-SHD-007` are already allocated by ADR-051 Decision 11 for mechanism-A
crash-atomicity partial failures. `E-SHD-008` is the next available slot.)

### Spec Change Intent

**BC-1.18.009 v1.6 amendments (routes to product-owner):**

1. **New EC-008** (add row to Edge Cases table):

   | EC-008 (NEW — hardening, counter-divergence guard) | Item-count trigger fires (`read_changelog_item_count` serde count `>= N`) but `rotate_changelog_at` returns `Ok(report)` where `report.mutated == false` (its line-scan `parse_frontmatter` count `total <= keep_recent` — counter-method divergence between trigger and rotate) | `HookResult::Error` with `E-SHD-008` ("counter-method divergence — trigger fired but rotate returned no-op; manual inspection required"); frontmatter unchanged; the self-DoS retry loop is structurally prevented |

2. **New Invariant Inv-5** (add to Invariants section after existing Inv-4):

   Intent text: "When `rotate_changelog_at` returns `Ok(report)` after the trigger fires,
   the gate MUST inspect `report.mutated`. If `report.mutated == false`, the gate MUST return
   `HookResult::Error` (`E-SHD-008`), never `HookResult::Block`. Emitting `Block` on a
   `mutated=false` report would falsely claim rotation occurred, sending the retrying agent
   into an infinite block+retry loop (self-DoS) for the duration of the session on
   BC-INDEX.md. This invariant gives the `RotationReport.mutated` field its first load-bearing
   use in BC-1.18.009's gate; the field has been present since BC-10.13.001/rotate.rs shipped."

3. **Postcondition 6 addendum**: Add one sentence noting that the fail-loud contract also
   covers `mutated=false` — EC-008's `HookResult::Error(E-SHD-008)` is the third fail-loud
   outcome of the rotation step, alongside `E-SHD-004` (Err arm). No structural change to PC6.

**error-taxonomy.md amendment (routes to product-owner, per ADR-051 Decision 11 routing
precedent: "product-owner adds rows to error-taxonomy.md — architect does not edit that file"):**

   New row: `E-SHD-008 | rotate_changelog_at returned Ok(mutated=false) after item-count
   trigger fired for "<artifact>": trigger (serde) and rotate (line-scan) item counts diverged
   — manual inspection required (BC-1.18.009 EC-008, Inv-5)`

**ADR-051 Decision 7 amendment (routes to architect):**

   Add one clarifying bullet to the B1 handler description, after the `Ok(_report) => Block`
   / `Err(e) => Error` summary:

   Intent: "Counter-divergence guard (EC-008): if `Ok(report)` carries `report.mutated ==
   false` after the trigger fired, the handler returns `Error(E-SHD-008)` instead of `Block`.
   This is distinct from `E-SHD-004` (Err arm, I/O failure). It closes the latent self-DoS
   the unconditional `Ok(_) => Block` match would produce if the serde trigger count and the
   `rotate_changelog_at` line-scan count ever diverged."

### Routing

| Work item | Assignee | Scope |
|-----------|----------|-------|
| BC-1.18.009 v1.6 amendment (EC-008 + Inv-5 + PC6 addendum) | **product-owner** | BC file + BC-INDEX changelog row |
| error-taxonomy.md new row E-SHD-008 | **product-owner** | `prd-supplements/error-taxonomy.md` |
| ADR-051 Decision 7 clarifying bullet | **architect** | ADR file + ARCH-INDEX changelog row |
| Code: `match Ok(report) if !report.mutated => Error(E-SHD-008)` guard in B1 handler | **implementer** | `shard_manager.rs` B1 match arm |
| Test: trigger-fires + `rotate_changelog_at` no-op returns `Error(E-SHD-008)` | **test-writer** | `bc_1_18_009_b1_rotate_test.rs` new test case (inject a fixture where serde count >= N but `changelog_items_raw.len() <= keep_recent`) |

**No BC-10.13.001 amendment needed.** The primitive's return type already exposes `mutated`;
the behavioral change is entirely in the caller (BC-1.18.009's gate handler). BC-10.13.001
PC5/Invariant 2 are unaffected.

**Human adjudication required: none.** The error-code choice (E-SHD-008 vs reusing E-SHD-004)
is resolved in-scope by the semantic-distinctness argument above. The option-a vs option-b
choice is resolved by the spec-preservation argument above. Both are answerable in current scope.

---

## Observation B: Archive-Write-Before-Source-Write / Double-Fault Duplicate

### Problem Statement (grounded in code)

`rotate_changelog_at` (rotate.rs ~L180-227) has two sequential `write_atomic` calls:

1. **Archive write** (~L213): `write_atomic(archive_path, &archive_content)` — appends the
   overflow tail to the single evergreen archive
2. **Source write** (~L220): `write_atomic(path, &new_raw)` — rewrites the frontmatter to
   hold only the `keep_recent` newest items

If the process crashes or the source write fails AFTER the archive write succeeds, the state
is:
- Archive: already contains the rotated-out tail items
- Source (BC-INDEX.md frontmatter): unchanged at N items (write_atomic uses temp-file-rename,
  so a failure leaves the original content intact — no partial write)

On the next dispatch, the item-count trigger re-fires (source still at N items). The gate
calls `rotate_changelog_at` again. The function re-reads the source (same N items), determines
the same overflow tail items to move, reads the archive (already has those items), and **appends
them a second time**. The archive now contains duplicate copies of the rotated-out items.

**PC5 analysis:** BC-1.18.009 PC5 states "no prior appended content is ever overwritten,
truncated, or deleted." Duplication by addition is NOT a PC5 violation under the current text.
BC-10.13.001 PC5 similarly guarantees "No `changelog:` item's `date:`/`change:` text content
is altered by rotation — only its location changes" — again, no violation.

**However:** BC-1.18.009 PC5 and the Related BCs description both claim full history
reconstructability ("reading the archive file followed by the current frontmatter `changelog:`
sequence reproduces full history with no gaps or duplicates"). "No duplicates" is already an
implicit correctness claim. The current crash scenario violates this implicit claim.
Production-grade requires making the behavior correct.

### Option Analysis

**Option (c): Source-first-then-archive.**
Source is trimmed first; archive is written second. If the archive write fails after the
source write succeeds: items are gone from source AND not in archive — **permanent data loss**.
This is strictly worse. Rejected per observation guidance.

**Option (b): Staged/journaled two-phase commit.**
Write archive to a temp location, atomically replace source, then rename temp to archive path.
Problem: `write_atomic` for the archive already writes to a temp file and renames; the issue
is that two separate `write_atomic` calls cannot be made jointly atomic. A journaled approach
would require a recovery-sentinel (a temporary file or a frontmatter marker) to signal
"archive committed, source pending." Callers (both the B1 gate and the CLI rotation
subcommand) would need recovery-detection logic. This adds complexity to a shared primitive
used by multiple callers (cli.rs + cycle-scoped callers), increases the surface area, and
introduces a new partial-state that all callers must handle.

**Option (a): Idempotent archive append (tail-match deduplication).**
Before writing to the archive, check whether the items to be archived are already present at
the archive's tail (byte-level, normalized for boundary `\n` handling). If they match, skip the
archive write entirely. Proceed unconditionally to the source rewrite.

This is the production-grade choice. Analysis:

- **Normal path (no prior crash):** Archive is empty OR archive tail does not match `move_items`
  (because `move_items` in this invocation are different from the last rotation's items). Dedup
  check is false. Archive write proceeds normally. Source write proceeds. `mutated: true`.
- **Crash-recovery path (archive written, source failed):** Source is unchanged (same N items).
  `move_items` are identical to the prior invocation (same oldest items, same split point).
  Dedup check finds these exact items already at archive tail. Archive write is skipped. Source
  write proceeds. `mutated: true`. The net result is identical to the normal path: archive has
  the items once, source is trimmed. No duplicates.
- **Callers unaffected:** All existing callers of `rotate_changelog` (which delegates to
  `rotate_changelog_at`) pass a literal `cycle_name`. Their test fixtures do not exercise the
  crash-recovery path. The dedup check is a no-op for them on the normal path (archive is
  written fresh each time, tail never coincidentally matches the next rotation's items).

**Write ordering is unchanged (archive-first, source-second).** This is load-bearing: reversing
to source-first would cause data loss on archive-write failure (option c).

### Deduplication Implementation Guidance (for implementer)

The dedup check must be tail-anchored, not substring-anchored (`String::contains` would match
items appearing anywhere in the archive, producing false positives if an item reappears in a
later rotation). The check is:

```
let move_combined: String = move_items.iter().map(|s| s.as_str()).collect();
// Normalize boundary: the archive may or may not end with '\n' after the boundary-
// normalization push in the current code (the push is conditional on !ends_with('\n')).
// Strip trailing '\n' on both sides for comparison.
let archive_trimmed = archive_content.trim_end_matches('\n');
let move_trimmed = move_combined.trim_end_matches('\n');
let already_archived = !move_trimmed.is_empty() && archive_trimmed.ends_with(move_trimmed);
```

If `already_archived == true`: skip the `yaml_guard` + `write_atomic` for the archive.
Proceed to `rewrite_source_after_rotation` and the source `write_atomic` unconditionally.

`RotationReport.mutated` should still be `true` when the source was rewritten, even if the
archive write was skipped — because the source WAS mutated (the file state changed).
`items_moved` reflects the semantic count (items logically moved), not whether the archive
write was physically performed. No new `RotationReport` field is needed.

### Spec Change Intent

**BC-1.18.009 v1.6 amendments (routes to product-owner):**

1. **New Invariant Inv-6** (add to Invariants section after new Inv-5 from Observation A):

   Intent text: "The B1 rotation is crash-idempotent at the archive boundary. If
   `rotate_changelog_at` completes the archive write but fails at the source rewrite, the
   agent's next Edit/Write/MultiEdit against BC-INDEX.md re-fires the item-count trigger
   (source unchanged). The subsequent `rotate_changelog_at` invocation is self-healing:
   `rotate_changelog_at`'s idempotent-append logic detects that the overflow items are already
   present at the archive's tail and skips the archive write, proceeding only with the source
   rewrite. The archive NEVER accumulates duplicate items from a partial rotation cycle.
   This invariant is the B1 analogue of Decision 11's mechanism-A `E-SHD-006` self-healing
   recovery for the 'sealed shard published, canonical-file truncate not yet complete' crash
   point — the vocabulary and recovery pattern are the same, applied to append-to-archive
   instead of copy-then-truncate."

2. **PC5 addendum**: Add one sentence to PC5 making the no-duplicate guarantee explicit:
   "The archive NEVER contains duplicate items across rotation cycles; a partial rotation
   cycle (archive written, source-write failed) is detected and resolved idempotently on the
   next invocation (see Invariant Inv-6)."

**BC-10.13.001 v1.4 amendment (routes to product-owner):**

   Extend **PC5** (or add new **PC8 — crash-recovery idempotency for the archive write**):

   Intent text: "If `rotate_changelog_at` is re-invoked after a crash that succeeded at the
   archive write but failed at the source write, the subsequent invocation is self-healing:
   the archive append is idempotent — items already present at the archive's tail (matching
   the current invocation's `move_items`, detected by a byte-level tail-match) are not
   re-appended. Only the source rewrite is reattempted. The archive NEVER accumulates
   duplicate items from partial rotation cycles. This extends Invariant 2's idempotency
   guarantee to the partial-failure state where the source file is unchanged (write failed)
   but the archive file has already been updated (write succeeded)."

   Invariant 2 should also be updated to explicitly cross-reference this crash-recovery
   property: "Re-running rotation immediately after a successful rotation against an
   unchanged source is a verified no-op (PC4 idempotency). Re-running after an
   archive-write-success + source-write-failure crash is also a no-op for the archive write
   (dedup detects the already-archived tail) — only the source rewrite is reattempted
   (PC5/PC8 crash-recovery clause)."

**ADR-051 Decision 7 amendment (routes to architect):**

   Add a crash-recovery sub-bullet to Decision 7's B1 handler description, in the location
   where Decision 11 is cross-referenced for steps 3-4, paralleling Decision 11's `E-SHD-006`
   treatment for mechanism A:

   Intent text: "B1 crash-recovery partial failure (archive-write-success + source-write-
   failure, no E-SHD-NNN, transparent self-healing): the source is left at pre-rotation count
   (safe for retry — the trigger will re-fire on the agent's next dispatch). The archive has
   the overflow items from this rotation cycle. Self-healing mechanism: `rotate_changelog_at`'s
   idempotent-append guard detects that the items to be archived are already present at the
   archive's tail and skips the archive write, reattempting only the source rewrite. Block+retry
   sequence completes normally. The archive NEVER accumulates duplicates from this crash point.
   This is the B1 analogue of Decision 11's E-SHD-006 (mechanism A: sealed-shard-published +
   canonical-file-truncate-pending), applying the same 'detection-and-resume, not rollback'
   recovery philosophy to B1's append-and-trim two-write sequence."

**No new error code needed.** The dedup is transparent behavior on the normal caller path.
The crash-recovery completes silently and correctly. No operator-visible error is emitted for
what is a self-healing internal state.

### Routing

| Work item | Assignee | Scope |
|-----------|----------|-------|
| BC-1.18.009 v1.6 amendment (Inv-6 + PC5 addendum) | **product-owner** | BC file + BC-INDEX changelog row |
| BC-10.13.001 v1.4 amendment (PC5/PC8 crash-recovery + Inv-2 extension) | **product-owner** | BC file + BC-INDEX changelog row |
| ADR-051 Decision 7 crash-recovery sub-bullet | **architect** | ADR file + ARCH-INDEX changelog row |
| Code: tail-match dedup in `rotate_changelog_at` (rotate.rs) | **implementer** | `crates/last-amended-migrate/src/rotate.rs` — add dedup check before archive `write_atomic` |
| Test: crash-recovery scenario (archive write succeeds, source write fails, retry produces no duplicate) | **test-writer** | `crates/last-amended-migrate/tests/bc_10_13_001_pc5_rotation_test.rs` new test; optionally `bc_1_18_009_b1_rotate_test.rs` for integration-level coverage |

### Sibling-Sweep Scope for the Shared Primitive

`rotate_changelog_at` is the only location where the dedup logic is added. Per TD-VSDD-060,
all callers must be verified safe with the modified behavior:

| Caller | Location | Impact |
|--------|----------|--------|
| `rotate_changelog` wrapper | `rotate.rs` | Thin delegation — inherits dedup transparently, no change |
| `bc_10_13_001_pc5_rotation_test.rs` | test fixtures | Normal-path only (fresh archive each test); dedup check is `false` → normal write proceeds; all existing tests stay green without modification |
| `bc_10_13_001_sec003_atomic_write_test.rs` | test fixture | Normal path; dedup check false → normal; unchanged |
| `shard_manager.rs` B1 handler | dispatcher | Calls `rotate_changelog_at` — inherits dedup transparently; no handler-level change for this observation |
| `cli.rs` (rotation subcommand) | last-amended-migrate binary | Calls `rotate_changelog` → delegates to `rotate_changelog_at` — inherits dedup transparently; crash-recovery now correct; no CLI-level change |

Zero existing callers need modification. The dedup is a pure additive behavior on the
crash-recovery path; the normal path is structurally identical (dedup check evaluates false,
code proceeds through the same branches as today).

**Human adjudication required: none.** The option-a vs option-b/c choice is resolved by
the data-loss analysis (option c rejected on first principles) and complexity analysis (option
b rejected due to multi-caller sentinel complexity). Option a is the only choice that is
simultaneously correct, minimal, and non-breaking across all callers. The dedup keying strategy
(tail-match) is answerable in current scope by examining the archive's append order.

---

## Combined Spec Amendment Summary

Both observations can and should be addressed in a single product-owner dispatch (one spec
amendment burst covers both). The BC-1.18.009 version advance from v1.5 to v1.6 covers BOTH
observations in one amendment. BC-10.13.001 advances from v1.3 to v1.4 for Observation B only.
ADR-051 receives two separate additions (one per observation) in one architect dispatch.

| Artifact | Change type | Assignee | Observations |
|----------|-------------|----------|-------------|
| `BC-1.18.009.md` v1.5 → v1.6 | New EC-008 + Inv-5 (Obs A); New Inv-6 + PC5 addendum (Obs B) | product-owner | A + B |
| `prd-supplements/error-taxonomy.md` | New row E-SHD-008 (Obs A) | product-owner | A |
| `BC-10.13.001.md` v1.3 → v1.4 | PC5/PC8 crash-recovery + Inv-2 extension (Obs B) | product-owner | B |
| `ADR-051-*.md` | Decision 7: counter-divergence bullet (Obs A) + crash-recovery sub-bullet (Obs B) | architect | A + B |
| `crates/last-amended-migrate/src/rotate.rs` | Add tail-match dedup before archive `write_atomic` | implementer | B |
| `crates/factory-dispatcher/src/shard_manager.rs` | `Ok(report) if !report.mutated => Error(E-SHD-008)` guard in B1 match | implementer | A |
| `bc_1_18_009_b1_rotate_test.rs` | New test: no-op-after-trigger returns `Error(E-SHD-008)` (Obs A); crash-recovery integration test (Obs B) | test-writer | A + B |
| `bc_10_13_001_pc5_rotation_test.rs` | New test: crash-recovery no-duplicate (Obs B) | test-writer | B |

**Ordering:** product-owner and architect dispatch can proceed in parallel (their artifacts
are independent). Implementer and test-writer proceed after spec amendments land (BC-1.18.009
v1.6 must be the red-gate source of truth before test-writer writes the EC-008 test).

---

*Produced by `vsdd-factory:architect`. This document is DESIGN-ONLY — no BC, ADR, code,
test, or index file was modified. State-manager does NOT need to advance STATE.md for this
document alone; it is an architect-domain design artifact. Downstream agents receive exact
text-intent specifications above — they do not need to re-derive the design.*
