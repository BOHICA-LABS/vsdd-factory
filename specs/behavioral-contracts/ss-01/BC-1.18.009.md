---
document_type: behavioral-contract
level: L3
version: "1.6"
status: draft
producer: product-owner
timestamp: 2026-09-06T00:00:00Z
phase: F2
inputs:
  - .factory/specs/architecture/decisions/ADR-051-layer-2-two-mechanism-size-triggered-shard-rotation-append-logs-and-bc-index-sharding.md
  - .factory/specs/architecture/decisions/ADR-049-last-amended-write-path-durable-fix-current-entry-plus-changelog-sequence.md
  - crates/last-amended-migrate/src/rotate.rs
  - crates/last-amended-migrate/src/changelog.rs
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.006.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.005.md
  - .factory/cycles/v1.0-brownfield-backfill/S-25.02-f2-architecture-delta.md
input-hash: "bccc10a"
traces_to: .factory/specs/prd.md
origin: greenfield
extracted_from: null
subsystem: "SS-01"
capability: "CAP-043"
lifecycle_status: draft
introduced: v1.0-brownfield-backfill
modified:
  - "2026-09-11 (v1.6)"
deprecated: null
deprecated_by: null
replacement: null
retired: null
removed: null
removal_reason: null
---

# BC-1.18.009: BC-INDEX Frontmatter `changelog:` Array Auto-Rotation (Mechanism B1, Reusing `rotate_changelog`) — Single-Actor Block-and-Retry

## Description

`BC-INDEX.md`'s frontmatter `changelog:` YAML sequence is, structurally, an append-only log
embedded inside an otherwise-structured document — the SAME shape as the four mechanism-A
artifacts, just YAML-list-item-shaped instead of markdown-section-shaped, and direct measurement
(2026-09-05) shows it is BC-INDEX's DOMINANT size driver at 177,305 of 539,713 total bytes. This BC
extends the SAME native gate BC-1.18.006 defines with one additional artifact-shape case: when the
matched artifact is "frontmatter changelog array" rather than "flat append-only file," and
BC-1.18.005 Postcondition 8's item-count trigger fires, the gate performs ONLY a rotate/trim step
via the already-shipped `rotate_changelog` (`crates/last-amended-migrate/src/rotate.rs`, ADR-049
§Decision 6) — automating what ADR-049 could previously only do via manual CLI invocation — THEN
returns `HookResult::Block` with a retry instruction, per the CORRECTED single-actor contract below.
**This BC's Postconditions 2 and 6 and Invariant 1 were rewritten in this fix-burst
(F-S2502-F2-001, BLOCKER, ADR-051 v1.1 Decision 7 "CORRECTED" subsection) to withdraw the v1.0
"gate rotates AND prepends, then Continues" design, which was unsound (double-actor prepend hazard;
for `Write`/`MultiEdit`, a stale pre-rotation payload could silently re-introduce the
just-archived tail item, permanently undoing the rotation).** The corrected contract makes this BC a
strict structural mirror of BC-1.18.006's already-accepted block-and-retry contract: one actor per
write, no exception carved out for B1.

## Preconditions

1. The dispatcher's native shard-cap gate (BC-1.18.005/BC-1.18.006) is live and has been extended
   with a "frontmatter changelog array" artifact-shape case (BC-1.18.005 Postcondition 8) that
   matches `BC-INDEX.md` (and, by the same generalized mechanism, `ARCH-INDEX.md` and `VP-INDEX.md`,
   which ADR-049's own audit finding 3 confirms carry the identical `changelog:` shape — though this
   BC's own scope, per S-25.02, is `BC-INDEX.md` specifically).
2. An `Edit`/`Write`/`MultiEdit` call targets `BC-INDEX.md`'s frontmatter, constructing its payload
   per the existing ADR-049 §Decision 2 discipline (the displaced `last_amended` entry becomes the
   new `changelog:` item) — this BC introduces no change to how the AGENT composes its own payload.
3. `rotate_changelog` (`crates/last-amended-migrate/src/rotate.rs`) and `prepend_changelog_item`
   (`crates/last-amended-migrate/src/changelog.rs`) are available as library functions callable
   from `shard_manager.rs` (a new workspace-internal dependency edge:
   `factory-dispatcher` → `last-amended-migrate`, confirmed acyclic per ADR-051 §8's dependency
   check — `last-amended-migrate` has no dependency back on `factory-dispatcher`).
4. **CORRECTED (fix-burst pass-7, F-P7-001, LOW).** `rotate_changelog`'s own implementation
   (`rewrite_source_after_rotation` in `rotate.rs`) is verified to be a PURE TRIM — it keeps only
   `keep_items` (the caller-supplied `keep_recent` most-recent items — for B1, the configured
   `low_water_mark`, per Postcondition 1/2; **NEVER a hardcoded `N-1`**, superseding the withdrawn
   pass-2 "retained N-1 most-recent items" framing this Precondition previously carried) and
   writes a `changelog_archive:` discoverability pointer; it never calls `prepend_changelog_item`
   itself and has no parameter for a "new item to insert." This is a structural SDK/library fact,
   not a runtime state — it is what makes the corrected single-actor contract (Postcondition 2)
   the only sound design, mirroring the reasoning BC-1.18.006 Precondition 3 applies to
   `HookResult`'s three-variant contract.

## Postconditions

1. **REWRITTEN (fix-burst pass-3, F-P3-005, ADR-051 v1.3 Decision 14) — the sequence NEVER EXCEEDS
   N items at any observable instant, but is NOT capped at exactly N most-recent items in steady
   state.** `N` (the maximum retained `changelog:` item count before the trigger fires) is read
   from the same shard-config source BC-1.18.005 Postcondition 6 introduces for mechanism A's cap
   constants — this BC's implementation MUST NOT hardcode `N` into `shard_manager.rs`. Rotation
   trims the live sequence to the **low-water floor** `low_water_mark` (BC-1.18.005 Postcondition
   8's rotation-target config, default `floor(N/2)`), **NEVER to `N-1`** — so the live sequence
   fluctuates between `low_water_mark + 1` and `N` across a batch of writes between rotations, not
   a fixed `N-1`/`N` pair. **Withdrawn (this fix-burst):** the prior "capped at N most-recent
   items" framing (trim-to-`N-1` every rotation) is UNSOUND as a steady-state description — it left
   the live sequence back at the trigger boundary (`N`) immediately after the very next successful
   prepend, so the VERY NEXT write re-evaluated `current_item_count + 1 > N` against a count
   already at `N` and rotated again, producing a block+retry round-trip on essentially every write
   to `BC-INDEX.md` (F-P3-005) — directly contradicting ADR-051's own "zero added latency for ~99%
   of writes" framing for the single hottest 4-index-discipline artifact. The corrected
   `low_water_mark`-targeted trim amortizes rotation to once per `N - low_water_mark` writes.

2. **CORRECTED (fix-burst pass-1, F-S2502-F2-001, BLOCKER; archive-path citation further
   CORRECTED fix-burst pass-2, F-P2-001, HIGH) — the gate performs ONLY the rotate/trim step,
   NEVER the prepend; the prepend is EXCLUSIVELY the responsibility of whichever `Edit`/`Write`/
   `MultiEdit` call ultimately lands.** On overflow (the item-count trigger fires per BC-1.18.005
   Postcondition 8: `current_item_count + 1 > N`), the gate invokes `rotate_changelog` — and ONLY
   `rotate_changelog` — to trim the live `changelog:` sequence down to `low_water_mark` items
   (**CORRECTED, fix-burst pass-3, F-P3-005, ADR-051 v1.3 Decision 14 — NEVER a hardcoded `N-1`**;
   `low_water_mark` is BC-1.18.005 Postcondition 8's rotation-target config, default `floor(N/2)`,
   fail-loud-validated `0 <= low_water_mark < N` — see Postcondition 1's rewrite above), APPENDING
   the overflowing TAIL (the oldest items beyond the retained `low_water_mark`) to a SINGLE,
   EVERGREEN archive file
   at `.factory/specs/behavioral-contracts/BC-INDEX-changelog-archive.md`. **CORRECTED (F-P2-001):
   this is NEVER a per-`seq` sealed-shard-DIRECTORY layout** (the withdrawn v1.1 text's
   `BC-INDEX-changelog-shards/BC-INDEX-changelog.<seq:04>.md` citation) — direct inspection of the
   SHIPPED `rotate_changelog`/`resolve_archive_path` implementation
   (`crates/last-amended-migrate/src/rotate.rs`) shows it derives exactly ONE fixed destination
   path per invocation and APPENDS to that same destination on every call; it has no
   `<seq:04>`-numbering capability at all. Because `rotate_changelog`'s existing
   `cycle_name`-derived path convention is correct only for its EXISTING cycle-scoped callers, and
   `BC-INDEX.md` is a `.factory/specs/behavioral-contracts/` catalog artifact with no natural
   `cycle_name`, this BC's gate calls a small, NAMED, bounded extension to the primitive's
   path-resolution surface — a generalized `archive_path: &Path` parameter (or equivalently-named
   `resolve_archive_path_at`) that a non-cycle caller supplies DIRECTLY, in place of deriving one
   from `cycle_name` — pre-computed by the dispatcher's B1 handler as the FIXED, non-cycle,
   BC-INDEX-sibling path above. Every other line of `rotate_changelog`'s logic (frontmatter
   parsing, `keep_recent` split, archive-content accumulation, `yaml_guard` validation,
   `write_atomic` for both files) is REUSED VERBATIM, unmodified; existing cycle-scoped callers
   are unaffected (they continue to use the existing `resolve_archive_path(path, cycle_name)`
   helper). The gate NEVER calls `prepend_changelog_item` itself. Sequence when rotation is needed:
   1. Agent issues its `Edit`/`Write`/`MultiEdit` call against `BC-INDEX.md`'s frontmatter,
      already containing its own prepend of the new `changelog:` item (standard ADR-049 §Decision 2
      discipline, unchanged).
   2. The gate's item-count trigger evaluates `current_item_count + 1 > N` against the file's
      CURRENT (pre-write) state. If false: `Continue` — no rotation; the agent's own prepend lands
      normally, unmodified (EC-002, unchanged from v1.0).
   3. If true: the gate invokes `rotate_changelog` (via the generalized, explicit-`archive_path`
      call surface) to trim the live sequence to `low_water_mark` items (**CORRECTED, fix-burst
      pass-3, F-P3-005 — NEVER `N-1`**) and append the overflow tail to the
      SINGLE evergreen archive file, THEN returns `HookResult::Block` with an explicit retry
      instruction: "BC-INDEX.md's `changelog:` sequence was rotated to make room (oldest item(s)
      appended to `.factory/specs/behavioral-contracts/BC-INDEX-changelog-archive.md`); the
      frontmatter now has `low_water_mark` items. Retry your write: if you used `Edit`, reissue as a fresh
      `Write` or a fresh `Edit` re-read against the current (post-rotation) file, since your
      original `old_string`/`new_string` pair may no longer match; if you used `Write`, recompute
      your `content` payload against the current (post-rotation) file before retrying — do not
      resubmit your original payload unchanged, since it reflects pre-rotation state." This is a
      shape-appropriate specialization of BC-1.18.006 Postcondition 2's retry-wording contract, not
      a divergent one.
   4. Agent retries, reading/recomputing against the now-rotated file; its retried prepend lands via
      `Continue` (item count is now `low_water_mark + 1`, not exceeding N — **CORRECTED, fix-burst
      pass-3, F-P3-005**, superseding the withdrawn `N-1+1 = N` framing) — SINGLE ACTOR, exactly
      once. Under the corrected trim target, the next `N - low_water_mark - 1` writes land via
      plain `Continue` before the sequence next reaches `N` and re-triggers rotation (the amortized
      cadence Postcondition 1's rewrite describes).
   **This WITHDRAWS the v1.0 design** in which the gate performed BOTH the rotate/trim AND the
   prepend (via `prepend_changelog_item`), then returned `Continue`, letting the originating agent's
   own already-composed call land on top. That design was unsound for two independent reasons: (a)
   **double-actor prepend** — two writers (the gate and the agent) both perform the identical
   logical action in the same operation, producing either a literal duplicate or an order-dependent
   race; (b) **stale-payload clobber for `Write`/`MultiEdit`** — a `Write` call's `content` is the
   agent's own complete, pre-computed payload built from PRE-rotation frontmatter state; if the gate
   rotates and then returns `Continue`, the agent's stale full-file `Write` payload lands OVER the
   just-rotated file, silently RE-INTRODUCING the just-archived tail item and guaranteeing the very
   next write re-triggers rotation again (an infinite churn loop) — exactly the hazard
   BC-1.18.006's Description names as "structurally forbidden" for mechanism A (a PreToolUse hook
   cannot safely mutate a file underneath an in-flight `Edit`/`Write`).

3. **This is the SAME native gate as BC-1.18.006, with a different artifact-shape case — not a
   separate mechanism.** The dispatcher's PreToolUse handling path (BC-1.18.005 Precondition 1)
   dispatches to one of (at least) two artifact-shape handlers based on the `[[shard]]` config
   entry's declared `shape` field: `"flat"` (mechanism A, BC-1.18.006) or
   `"frontmatter-changelog-array"` (this BC, mechanism B1). Both handlers share the SAME
   config-match/no-match entry point (BC-1.18.005 Postcondition 1's zero-cost bypass for unmatched
   paths applies identically to both shapes).

4. **The rotation is triggered by the SAME `Edit`/`Write`/`MultiEdit` PreToolUse gate, not a
   separate scheduled or manually-invoked process.** Unlike ADR-049's pre-this-BC state (a manual,
   operator-invoked safety net), this BC's rotation fires automatically, size-triggered, as part
   of the SAME dispatcher-native check that mechanism A's roll-before-write uses — closing the gap
   ADR-051's Context section identifies ("that tool is operator-invoked, not size-triggered or
   dispatcher-mediated").

5. **CORRECTED (fix-burst pass-2, F-P2-001, HIGH) — the archived changelog history is a SINGLE,
   evergreen, append-only file, never a set of per-`seq` sealed shards, and never loses history.**
   Every `rotate_changelog` invocation APPENDS to the SAME single destination file,
   `.factory/specs/behavioral-contracts/BC-INDEX-changelog-archive.md` — no prior appended content
   is ever overwritten, truncated, or deleted by a subsequent rotation (`rotate_changelog` reads
   any pre-existing archive content at that path before appending, per its shipped
   implementation). The full changelog history remains reconstructable by reading the archive
   file (oldest-appended content first, in append order) followed by the current frontmatter
   `changelog:` sequence. **Accepted trade-off, documented not hidden (ADR-051 §Decision 7):** this
   archive file is itself append-only and unbounded across BC-INDEX's lifetime — the SAME shape
   ADR-049's original manual `rotate_changelog` tool already accepted as sufficient. This is sound
   here because (a) the archive is small per-append (individual `changelog:` items, not whole BC
   rows), (b) it is read by NEITHER this BC's own item-count trigger (which inspects only the LIVE
   frontmatter sequence) NOR any Cohort B validator (`regression-gate`/`convergence-tracker` do not
   read `BC-INDEX.md` at all, per the F2 architecture-delta doc §5 migration-impact map), so it
   sits entirely outside Layer 2's own bounded-artifact concern; and (c) if it later becomes large
   enough to be a new forensic contributor in its own right, that is a follow-up
   Layer-2-on-Layer-2 story, not a defect of this design.
   **The archive NEVER contains duplicate items across rotation cycles; a partial rotation
   cycle (archive written, source-write failed) is detected and resolved idempotently on the
   next invocation (see Invariant Inv-6).**

6. **CORRECTED (fix-burst, F-S2502-F2-001, BLOCKER; error-code citation further CORRECTED
   fix-burst pass-3, F-P3-001, HIGH) — the block-and-retry observable outcome
   (BC-1.18.006 Postcondition 2) applies IDENTICALLY to this artifact shape; there is NO
   "Continue after rotation" divergence.** If the rotation itself cannot complete (e.g.,
   `rotate_changelog` errors), the gate returns `HookResult::Error` (**`E-SHD-004`,
   `rotate_changelog` invocation failure — this BC's OWN error code, per `error-taxonomy.md`'s
   Error Catalog and VP-INDEX's authoritative `004→VP-131` mapping; NEVER `E-SHD-001`**, which
   names BC-1.18.006's DIFFERENT mechanism-A shard-seal-write failure — the prior v1.3
   residual-cleanup micro-burst mistakenly "corrected" this citation to the wrong sibling code
   `E-SHD-001`), the fail-loud PATTERN mirroring BC-1.18.006's EC-003
   contract for mechanism A (fail-loud, no partial state left authoritative) — but with this BC's
   OWN distinct error code. If the rotation completes
   successfully, the gate returns `HookResult::Block` (Postcondition 2 above) — NEVER `Continue` —
   because `Continue` after a successful rotation would let the agent's own already-composed,
   PRE-rotation payload land on top of the just-rotated file, which is exactly the double-actor /
   stale-payload hazard Postcondition 2 withdraws. **This supersedes the v1.0 text of this
   postcondition, which incorrectly asserted that mechanism A's block-and-retry contract does not
   apply to B1 ("this write CAN proceed after rotation makes room... a deliberate divergence") — that
   assertion is WITHDRAWN.** B1's block-and-retry wording differs from mechanism A's only in the
   artifact-name and rotation-mechanism details (Postcondition 2 step 3's exact text), never in the
   actor-ownership shape.
   **Additionally, the fail-loud contract covers the `mutated=false` outcome (EC-008): when
   `rotate_changelog_at` returns `Ok(report)` with `report.mutated == false` after the trigger
   fired, the gate returns `HookResult::Error(E-SHD-014)` — the `Error` VARIANT, never the `Block`
   VARIANT — making EC-008's `Error` the third fail-loud outcome of the rotation step, alongside
   `E-SHD-004` (the Err arm `rotate_changelog_at` invocation failure). `E-SHD-014` is a BLOCKING
   fail-loud Error (`block_intent=true`, `exit_code=2`, dispatched with the same `on_error=Block`
   treatment as `E-SHD-004`); "never `Block`" refers to the `HookResult::Block` variant (the
   rotation-succeeded retry instruction), NOT to dispatch-blocking. The unrotatable over-N write
   is BLOCKED and MUST NOT proceed; a non-blocking/advisory `E-SHD-014` that lets the write
   proceed is NON-COMPLIANT.**

## Invariants

1. **CORRECTED (fix-burst pass-1, F-S2502-F2-001, BLOCKER; refined fix-burst pass-2, F-P2-001,
   HIGH) — the gate's rotation step calls `rotate_changelog` ONLY; `prepend_changelog_item` is
   NEVER invoked by the gate — only by the agent's own write (original or retried).** No duplicate
   changelog-rotation, trim, validation, or write logic exists in `shard_manager.rs`;
   `rotate_changelog` is imported from `crates/last-amended-migrate` as a library call. Post-fix,
   `prepend_changelog_item` appears in NEITHER `shard_manager.rs` NOR any other dispatcher-native
   code path this BC introduces — it remains exclusively a function the AGENT's own tool-call
   composition logic follows (per ADR-049 §Decision 2), never a function the gate calls on the
   agent's behalf. **Refined (F-P2-001): "no reimplementation" is NOT an unqualified claim — it
   means no new ROTATION/TRIM/VALIDATE/WRITE logic is introduced. It does NOT mean zero new code:
   this BC's gate reaches `rotate_changelog` via a small, NAMED, bounded extension to the
   primitive's path-resolution surface (an explicit `archive_path` parameter, in place of deriving
   one from `cycle_name` — see Postcondition 2), because `BC-INDEX.md` has no natural `cycle_name`
   value. This bounded, additive extension is fully consistent with the invariant's intent (no
   duplicated business logic) and does not reintroduce any reimplementation risk.** This supersedes
   the v1.0 invariant text ("`rotate_changelog` and `prepend_changelog_item` are called, never
   reimplemented"), which incorrectly implied the gate itself calls both functions.

2. **Existing `changelog:` items are never mutated in place, only relocated.** Rotation moves the
   oldest N+1th-and-beyond items out of the live frontmatter into the single evergreen archive file
   (`.factory/specs/behavioral-contracts/BC-INDEX-changelog-archive.md`, Postcondition 5 — CORRECTED,
   F-P2-001: never a per-`seq` sealed shard); it never edits the content of a surviving item, and it
   never edits the content of a rotated-out item (the rotated-out items are appended byte-for-byte
   to the archive file, never overwritten or truncated).

3. **`ARCH-INDEX.md`/`VP-INDEX.md`'s identical `changelog:` shape is NOT auto-rotated by this BC.**
   This BC's scope is `BC-INDEX.md` specifically, per S-25.02's own scope. Extending the same
   mechanism to `ARCH-INDEX.md`/`VP-INDEX.md` is a future BC-authorship decision, not silently
   implied by this BC's implementation reusing the same underlying `rotate_changelog` function.

4. **Block-and-retry is the ONLY observable outcome of a successful rotation; `Continue`-after-
   rotation is structurally forbidden (fix-burst, F-S2502-F2-001).** No implementation of this BC
   may return `Continue` after `rotate_changelog` completes successfully — doing so reintroduces
   the withdrawn double-actor/stale-payload hazard Postcondition 2 and Postcondition 6 describe.
   This invariant makes BC-1.18.009 a structural mirror of BC-1.18.006 Invariant 1's "the gate
   returns `Block` on every over-cap/over-N-item write, never `Continue`" contract.

5. **`Ok(report)` with `report.mutated == false` after a fired trigger → `Error(E-SHD-014)`,
   never `Block` (EC-008 counter-divergence guard).** "Never `Block`" means never the
   `HookResult::Block` VARIANT (the rotation-succeeded retry instruction) — NOT "never block the
   dispatch." `E-SHD-014` is a BLOCKING fail-loud Error (`block_intent=true`, `exit_code=2`,
   `on_error=Block` — the same blocking treatment as `E-SHD-004`). The unrotatable over-N write
   MUST be blocked and MUST NOT proceed; an implementation that returns a non-blocking/advisory
   `E-SHD-014` (`block_intent=false`, `exit_code=1`, `on_error=Continue`) and allows the write
   to proceed is NON-COMPLIANT with this invariant.
   When `rotate_changelog_at` returns `Ok(report)` after the item-count trigger fires, the gate
   MUST inspect `report.mutated`. If `report.mutated == false`, the gate MUST return
   `HookResult::Error` (`E-SHD-014`), never `HookResult::Block`. Emitting `Block` on a
   `mutated=false` report would falsely claim rotation occurred, sending the retrying agent into
   an infinite block+retry loop (self-DoS) for the duration of the session on `BC-INDEX.md`: the
   item-count trigger re-fires on every subsequent dispatch (source is unchanged) and the gate
   would again return `Block` claiming rotation — permanently. Using `Error` (not `Block`) breaks
   the retry loop because `Error` carries NO retry instruction, while still failing loud and
   blocking the write (fail-closed). This is the correct fail-closed behavior for a
   "should never happen" counter-divergence; the taxonomy message already says "manual inspection
   required."
   This invariant gives the `RotationReport.mutated` field its first load-bearing use in
   BC-1.18.009's gate; the field has been present since BC-10.13.001/`rotate.rs` shipped.
   This is distinct from Invariant 4 (which governs the `mutated=true` `Ok` case) — both apply
   to the `Ok` arm, with different guard conditions.

6. **The B1 rotation is crash-idempotent at the archive boundary — the archive NEVER accumulates
   duplicate items even under an archive-write-success + source-write-failure double-fault + retry
   (Obs B crash-recovery).** `rotate_changelog_at`'s write sequence is archive-first, source-second
   (write ordering is load-bearing: reversing to source-first would cause data loss if the archive
   write then failed). If the process crashes or the source rewrite fails AFTER the archive write
   succeeds, the source is unchanged (`write_atomic`'s temp-file-rename failure leaves the original
   intact — no partial write). The item-count trigger re-fires on the next dispatch. The subsequent
   `rotate_changelog_at` invocation self-heals: its idempotent-append logic detects that the
   overflow items (`move_items`) are already present at the archive's tail via a byte-level
   tail-match check (the combined `move_items` string is compared against the normalized trailing
   bytes of the existing archive content, stripped of trailing `\n` on both sides — tail-anchored,
   never `String::contains`, to prevent false positives from recurrent content) and skips the
   archive write, proceeding only with the source rewrite. `RotationReport.mutated` is `true` when
   the source was rewritten, even if the archive write was skipped (the source WAS mutated);
   `items_moved` reflects the semantic count (items logically moved), not whether the archive write
   was physically performed. This invariant is the B1 analogue of ADR-051 Decision 11's mechanism-A
   `E-SHD-006` self-healing recovery for the "sealed shard published, canonical-file truncate not
   yet complete" crash point — the same "detection-and-resume, not rollback" recovery philosophy
   applied to B1's archive-append-then-source-trim two-write sequence.

## Edge Cases

| ID | Description | Expected Behavior |
|----|-------------|-------------------|
| EC-001 | `changelog:` sequence is exactly at N items; a new prepend would make N+1 | Rotation triggers: the oldest items beyond the retained `low_water_mark` are appended to the single evergreen archive file (`BC-INDEX-changelog-archive.md`), live sequence trimmed to `low_water_mark` items (**CORRECTED, fix-burst pass-3, F-P3-005 — NEVER `N-1`**), THEN `HookResult::Block` returned with a retry instruction (Postcondition 2) — the agent's retried write lands the new item, bringing the sequence to `low_water_mark + 1` |
| EC-002 | `changelog:` sequence is well under N items | No rotation; the new item is simply prepended by the agent's own original call (`Continue`, standard ADR-049 §Decision 2 discipline, unmodified) |
| EC-003 | `rotate_changelog` itself fails (e.g., cannot write to the archive file — disk full) | `HookResult::Error` (**`E-SHD-004`, `rotate_changelog` invocation failure — CORRECTED fix-burst pass-3, F-P3-001; NEVER `E-SHD-001`, which is BC-1.18.006's distinct mechanism-A shard-seal-write-failure code**); the triggering prepend is NOT applied; the frontmatter is left in its pre-rotation state (fail-loud, the SAME PATTERN as BC-1.18.006 EC-003's `E-SHD-001` mechanism-A precedent, but with this BC's own distinct error code) |
| EC-004 | A single `changelog:` item is itself extremely large (e.g., the 16,521-byte line ADR-051 directly measured) | That item still counts as exactly ONE item toward the N-item cap — rotation is item-count-triggered for this mechanism, per BC-1.18.005 Postcondition 8, not byte-size-triggered per item |
| EC-005 | Two concurrent sessions both attempt to prepend a `changelog:` item near the N-item boundary | TD-VSDD-053 single-commit-per-burst and the project's factory-lock discipline (ADR-025) prevent concurrent factory-artifacts commits from landing interleaved; the second session's dispatch re-reads the (now-rotated) frontmatter state before its own prepend is evaluated |
| EC-006 (fix-burst, F-S2502-F2-001) | An implementer mistakenly has the gate call `prepend_changelog_item` after a successful `rotate_changelog`, then returns `Continue` | This is the withdrawn v1.0 design and a direct violation of Postcondition 2/6 and Invariant 1/4 — a static-analysis check (VP-126) MUST detect any `prepend_changelog_item` call site inside `shard_manager.rs`'s B1 handler as a defect, not a valid implementation choice |
| EC-007 (NEW, fix-burst pass-3, F-P3-005) | `current_item_count` reaches `N` again after a prior rotation (the normal, expected steady-state re-trigger boundary post-fix, occurring once per `N - low_water_mark` writes) | This is NOT a defect — the SAME single-actor block-and-retry contract (Postconditions 2/3/4/6) applies identically to this re-trigger as to the first-ever rotation; the amortized cadence is the intended behavior of the `low_water_mark` correction, not a regression to the withdrawn every-write rotation pathology |
| EC-008 (NEW — cluster-4 hardening, counter-divergence guard) | Item-count trigger fires (`read_changelog_item_count` serde count `>= N`) but `rotate_changelog_at` returns `Ok(report)` where `report.mutated == false` (its line-scan `parse_frontmatter` count `total <= keep_recent` — counter-method divergence between the B1 handler's serde deserialization trigger path and `rotate_changelog_at`'s own `parse_frontmatter` line-scan path; provably latent on canonical `  - date:` format today because the two counters agree exactly, but an architectural coupling with no load-bearing gate preventing divergence) | `HookResult::Error` with `E-SHD-014` message naming the artifact and describing counter-method divergence; frontmatter `changelog:` unchanged from pre-attempt state; the self-DoS retry loop (infinite `Block`+retry on `BC-INDEX.md` for the session duration) is structurally prevented — the `Ok(report) if !report.mutated => Error(E-SHD-014)` guard intercepts it before `Block` is emitted (Invariant Inv-5); `E-SHD-014` is a BLOCKING Error (`block_intent=true`, `exit_code=2`, `on_error=Block`) — the over-N write is BLOCKED, not allowed to proceed; "never Block" refers to the `HookResult::Block` variant (the retry instruction), NOT to dispatch-blocking; a non-blocking/advisory `E-SHD-014` that allows the write to proceed is NON-COMPLIANT |

## Canonical Test Vectors

| Input | Expected Output | Category |
|-------|----------------|----------|
| **CORRECTED (fix-burst pass-3, F-P3-005 — supersedes the withdrawn pass-2 N-1 framing).** `changelog:` at N=50 items, `low_water_mark=25` (config), agent's `Edit` call already contains its own new-item prepend | Items #50 down through #26 (the 25 oldest beyond the retained 25) appended to the SINGLE evergreen archive `.factory/specs/behavioral-contracts/BC-INDEX-changelog-archive.md`; live sequence trimmed to 25 items; gate returns `Block{reason: "...rotated to make room (oldest item(s) appended to BC-INDEX-changelog-archive.md)... frontmatter now has 25 items. Retry your write..."}`; agent's retry lands the new item, sequence at 26 total | happy-path |
| **NEW (fix-burst pass-3, F-P3-005) — demonstrates the amortized rotation cadence.** `N=50`, `low_water_mark=25`; sequence at 50 items triggers rotation, trims to 25, retry lands at 26 | The next 24 ordinary writes (items 27 through 50) land via plain `Continue` — NO block/retry round-trip — before the 50-item boundary is reached again and re-triggers rotation (EC-007); this is the amortized-to-once-per-`N-low_water_mark`-writes cadence Postcondition 1 describes, replacing the withdrawn design's every-write rotation | edge-case |
| `changelog:` at 10 items, N=50, agent's `Write` call already contains its own new-item prepend | No rotation; `Continue`; the agent's original call's new item is simply prepended (11 items total) — single actor, no retry needed | happy-path |
| `rotate_changelog` fails mid-operation (simulated disk-full) | `HookResult::Error`; frontmatter `changelog:` unchanged from its pre-attempt state | error |
| Agent retries a blocked `Write` WITHOUT recomputing its payload against the post-rotation file (resubmits stale pre-rotation content) | The stale `Write` either fails at the tool layer (content mismatch with a validator expecting post-rotation shape) or, if it lands, re-introduces the just-rotated tail item — this is a caller-compliance failure, not a gate defect; the gate's own retry-instruction text (Postcondition 2 step 3) explicitly warns against this | error |
| **CORRECTED (fix-burst pass-2, F-P2-001).** Reading full changelog history: evergreen archive `BC-INDEX-changelog-archive.md` + current frontmatter `changelog:` sequence | Concatenation (archive file content in append order, oldest-to-newest, followed by current frontmatter sequence, newest-first per existing convention) reproduces full history with no gaps or duplicates — NEVER a multi-shard-directory glob (the withdrawn v1.1 model), since the archive is a single file | edge-case |
| Static scan of `shard_manager.rs`'s B1 handler for `prepend_changelog_item` call sites | ZERO call sites found (Invariant 1) — the function is imported/used ONLY by agent-side tooling, never by the gate | happy-path |
| **NEW (cluster-4 hardening, Obs A, EC-008).** Fixture: serde `read_changelog_item_count` returns count `>= N` (trigger fires) but `rotate_changelog_at` stub returns `Ok(RotationReport { mutated: false, items_moved: 0 })` (inject a fixture where `changelog_items_raw.len() <= keep_recent` in `parse_frontmatter` — the counter-divergence scenario) | `HookResult::Error` with `E-SHD-014` message naming the artifact and the counter-method divergence; frontmatter `changelog:` byte-identical to pre-attempt state; outcome is a BLOCKING Error (`block_intent=true`, `exit_code=2`) — the write is BLOCKED, not advisory/non-blocking; `HookResult::Block` variant is NOT returned (the Error variant breaks the self-DoS loop because it carries no retry instruction, while still blocking the write — Invariant Inv-5); distinguish the outcome by variant (`HookResult::Error` vs `HookResult::Block`) and by message (E-SHD-014 counter-divergence text), NOT by block_intent alone (both variants block the dispatch when configured with `on_error=Block`) | error (counter-divergence guard) |
| **NEW (cluster-4 hardening, Obs B, crash-recovery).** Fixture: prior execution completed the archive write (overflow tail appended to `BC-INDEX-changelog-archive.md`) but failed the source rewrite (simulated `write_atomic` failure leaves source unchanged at N items); next dispatch re-fires the trigger | `rotate_changelog_at` self-heals: detects overflow `move_items` already present at archive tail via byte-level tail-match (normalized for boundary `\n`); archive write skipped; source `changelog:` rewrite completes normally; `HookResult::Block` returned with retry instruction; `RotationReport { mutated: true }`; archive contains the overflow items ONCE, not twice; Invariant Inv-6 no-duplicate guarantee satisfied | edge-case (crash-recovery) |

## Verification Properties

| VP-NNN | Property | Proof Method |
|--------|----------|-------------|
| VP-126 | No-reimplementation-and-no-gate-side-prepend invariant — `shard_manager.rs`'s B1 handler contains no changelog-rotation logic other than a call into `rotate_changelog`, and contains ZERO call sites for `prepend_changelog_item` (fix-burst-strengthened per F-S2502-F2-001); **cluster-4 hardening extension: guard-arm-presence facet (EC-008, Invariant Inv-5)** — static analysis must additionally confirm the `Ok(report) if !report.mutated => Error(E-SHD-014)` guard arm is present in the B1 `Ok` match arm and routes to `HookResult::Error`, never `HookResult::Block` | code-review / static-analysis check (grep for duplicated rotation logic AND for any `prepend_changelog_item` call site inside the B1 handler; additionally: presence of `if !report.mutated` guard arm routing to `Error` variant confirmed; `Block` on `mutated=false` confirmed absent) |
| VP-125 | Bounded-live-sequence invariant (after any sequence of prepends, the live frontmatter `changelog:` sequence never exceeds N items, and the post-rotation floor is `low_water_mark` — a config-varied value, NEVER a hardcoded `N-1` — per fix-burst pass-3, F-P3-005); No-history-loss invariant (every `changelog:` item ever prepended remains recoverable, live in the frontmatter or appended to the single evergreen archive file) | proptest — two facets, arbitrary prepend sequences: `len(changelog) <= N` after every operation AND the post-rotation trim floor equals the configured `low_water_mark`; total recoverable item count is monotonically non-decreasing and equals the total prepend count |
| VP-131 | **Two-facet fail-loud invariant (cluster-4 hardening extension):** (a) **Err-arm** — a `rotate_changelog` invocation failure (`Err`) returns `HookResult::Error` (`E-SHD-004`), never `Block`/`Continue`, and leaves the live `changelog:` sequence byte-identical to its pre-rotation state (EC-003, Postcondition 6); (b) **Ok-arm** — `rotate_changelog_at` returning `Ok(report)` with `report.mutated == false` after the trigger fired returns `HookResult::Error` (`E-SHD-014`), never `Block` — the Ok-arm sibling of the Err-arm `E-SHD-004` leg (EC-008, Invariant Inv-5) | unit test — two cases: (a) injected `rotate_changelog`-failure FS (disk-full/permission); assert `Error(E-SHD-004)` naming artifact + failing op, pre-rotation state preserved; (b) injected `Ok(mutated=false)` stub after trigger fires; assert `Error(E-SHD-014)`, `Block` NOT returned, pre-rotation state byte-identical |
| VP-112 | **Cluster-4 hardening: crash-idempotent archive boundary (Invariant Inv-6)** — the `rotate_changelog_at` primitive's idempotent-append tail-match dedup ensures the archive NEVER accumulates duplicate items across a partial rotation cycle (archive-write-success + source-write-failure crash + retry); discharges BC-1.18.009 Invariant Inv-6 via the shared `rotate_changelog_at` implementation in `crates/last-amended-migrate/src/rotate.rs` — see BC-10.13.001 VP-112 for the shared primitive's primary losslessness property; this BC's citation covers the crash-recovery no-duplicate facet as applied to the B1 gate's archive path | unit-test: crash-recovery fixture in `crates/last-amended-migrate/tests/bc_10_13_001_pc5_rotation_test.rs` (new fixture, cluster-4 hardening) — archive write succeeds, source write fails (injected), retry invocation detects already-archived tail via byte-level tail-match, skips archive write, completes source rewrite; archive contains overflow items once not twice |

**Fix-burst note (F-S2502-F2-001):** formal-verifier should review VP-125/VP-126 against this BC's
corrected contract — the "bounded-live-sequence" and "no-history-loss" properties still hold
structurally, but any proptest/static-analysis harness must now model TWO tool-call events (block,
then retry) for the over-N case, not one, and VP-126's static scan must additionally assert zero
`prepend_changelog_item` call sites in the gate's own code (not yet re-verified against VP bodies
in this burst — VP body edits are formal-verifier's domain). VP-131 (new in the S-25.02 F2
verification-property fix-burst, VP-INDEX v3.03) closes the EC-003/Postcondition-6 fail-loud gap
symmetric with VP-120/VP-122/VP-124's mechanism-A fail-loud legs (F-S2502-F2-004).

**Fix-burst note (fix-burst pass-3, F-P3-006):** VP-125's two previously-separate rows are collapsed
to ONE row listing its two facets (multi-facet convention — VP-125 is a single allocated VP covering
both properties, not two separate VPs). No property content or coverage change, only table
presentation; the `low_water_mark`-floor facet is a genuinely new assertion (F-P3-005) folded into
this same collapsed row rather than added as a third separate row.

## Related BCs

- BC-1.18.006 — this BC extends the SAME native gate with a different artifact-shape case, and (post-fix) is a strict structural mirror of its block-and-retry actor-ownership contract (depends on)
- BC-1.18.005 — owns the item-count trigger condition (Postcondition 8) this BC's rotation responds to (depends on)
- BC-1.18.010 — sibling mechanism B2 (per-subsystem body sharding) targets BC-INDEX's OTHER size driver; both mechanisms are triggered by the same gate (composes with)
- BC-1.18.011 — the B2 migration BC's governed one-time-migration pattern is architecturally parallel to (but independent of) this BC's ongoing per-write rotation (related to)
- BC-1.18.012 — the governed one-time B1 changelog BACKFILL migration for BC-INDEX's cold-start (~1,997-item) `changelog:` sequence; this BC specifies the ongoing STEADY-STATE per-write rotation contract that BC-1.18.012's migration establishes the precondition for (depended on by)
- BC-10.13.001 — the sanctioned migration/rotation tool whose `rotate_changelog` primitive this BC automates the invocation of (depends on)

## Architecture Anchors

- `crates/factory-dispatcher/src/shard_manager.rs` — the "frontmatter changelog array" artifact-shape handler, calling ONLY `rotate_changelog` from `last-amended-migrate` (never `prepend_changelog_item`), via the generalized `archive_path`-parameterized call surface (F-P2-001)
- `crates/last-amended-migrate/src/rotate.rs` — `rotate_changelog` / `rewrite_source_after_rotation` / `resolve_archive_path` (reused, not reimplemented; verified pure-trim, no prepend call). CORRECTED (F-P2-001): the SHIPPED `resolve_archive_path(path, cycle_name)` derives exactly ONE fixed, cycle-scoped destination and has NO `<seq:04>`-numbering capability — this BC's gate requires a small, additive, explicit-`archive_path`-parameter extension to this function (or an equivalent `resolve_archive_path_at`) not yet present in the shipped code, to be implemented at F4 (see SDK Grounding Evidence below for the current-state grep).
- `crates/last-amended-migrate/src/changelog.rs` — `prepend_changelog_item` (agent-side tooling only, per the corrected contract — never called by the gate)
- `crates/hook-sdk/src/result.rs` — `HookResult` enum, whose `Block`/`Continue`/`Error` three-variant contract is what makes the single-actor correction structurally necessary (same SDK fact BC-1.18.006 Precondition 3 cites)

## SDK Grounding Evidence

Literal stable-anchor greps substantiating this BC's external-artifact claims (POLICY 5;
no `grep -n` / no file:line citations per TD-VSDD-091):

```
$ grep -oE "^pub fn rotate_changelog" crates/last-amended-migrate/src/rotate.rs
pub fn rotate_changelog
```

```
$ grep -oE "^fn rewrite_source_after_rotation" crates/last-amended-migrate/src/rotate.rs
fn rewrite_source_after_rotation
```

```
$ grep -oE "^fn resolve_archive_path\(path: &Path, cycle_name: &str\)" crates/last-amended-migrate/src/rotate.rs
fn resolve_archive_path(path: &Path, cycle_name: &str)
```

**CORRECTED (fix-burst pass-2, F-P2-001) — current-state grounding, not yet the target state.**
This confirms the SHIPPED `resolve_archive_path` signature derives its destination EXCLUSIVELY
from `cycle_name` (no `archive_path` parameter exists yet, no `<seq:04>` numbering capability
anywhere in this file) — grounding Postcondition 2/Invariant 1's finding that the withdrawn v1.1
per-`seq` sealed-shard-directory citation was internally impossible against this function as
shipped. The generalized, explicit-`archive_path`-parameterized call surface this BC's gate
requires (Architecture Anchors above) is a TO-BE-IMPLEMENTED, additive extension at F4 — this grep
intentionally shows its ABSENCE today, not its presence; implementer re-runs this grep at F4 and
replaces this block with confirmation of the new parameter once added.

```
$ grep -oE "^pub fn prepend_changelog_item|^pub fn ensure_changelog_field" crates/last-amended-migrate/src/changelog.rs
ensure_changelog_field
prepend_changelog_item
```

(second line shown without the `pub fn` prefix reflects grep's own match ordering; both functions
are confirmed present as separate, independently-callable library functions — `rotate_changelog`
has no parameter for inserting a new item, and `rewrite_source_after_rotation` performs the pure
trim, grounding this BC's Precondition 4 and the corrected Postcondition 2's actor-separation
argument.)

```
$ grep -oE "^pub enum HookResult" crates/hook-sdk/src/result.rs
pub enum HookResult
```

```
$ grep -oE "^\s*(Continue|Block \{[^}]*\}|Error \{[^}]*\})" crates/hook-sdk/src/result.rs | sed -E 's/^\s+//' | sort -u
Block { reason: String }
Continue
Error { message: String }
```

Confirms `HookResult`'s three-variant contract, grounding this BC's corrected Postcondition 6 claim
that no fourth "mutate-then-continue" variant exists — `Continue` and mutation-before-return are
mutually exclusive only insofar as `Continue` cannot itself carry the agent's own pending payload,
which is the actual structural reason a second gate-side write is unsafe.

## Story Anchor

S-25.02 — Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Artifacts

## VP Anchors

- VP-125, VP-126, VP-131 — allocated by formal-verifier (VP-125/126: S-25.02 F2 verification-property extension burst, VP-INDEX v3.02; VP-131: S-25.02 F2 verification-property fix-burst, VP-INDEX v3.03, F-S2502-F2-004). VP-125 (proptest; bounded live changelog: sequence + no-history-loss against the SINGLE evergreen archive model), VP-126 (static-check; rotate_changelog reuse — no reimplemented rotation logic, and post-fix-burst, no gate-side `prepend_changelog_item` call site; **cluster-4 hardening extension: guard-arm-presence facet for `Ok(mutated=false) => Error(E-SHD-014)` guard**), VP-131 (unit-test; **two-facet fail-loud, cluster-4 hardening**: Err-arm EC-003/Postcondition 6 `E-SHD-004` leg + Ok-arm EC-008/Invariant Inv-5 `E-SHD-014` leg — symmetric with VP-120/VP-122/VP-124's mechanism-A fail-loud legs). VP-125/126/131 bodies updated by formal-verifier (cluster-4 hardening burst, same commit as BC v1.6). **VP-112 (shared primitive's crash-recovery no-duplicate facet)** — discharges Invariant Inv-6 (crash-idempotent archive boundary) via the shared `rotate_changelog_at` implementation; primary registration is BC-10.13.001 VP-112 (PC5 losslessness + PC8 crash-recovery); this BC cites it for the Inv-6 facet specifically (see §Verification Properties VP-112 row above).

## Traceability

| Field | Value |
|-------|-------|
| L2 Capability | CAP-043 |
| Capability Anchor Justification | CAP-043 ("Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Append-Logs and BC-INDEX Structured-Catalog Sharding") per capabilities.md §CAP-043 — this BC specifies mechanism B1 exactly as CAP-043 describes it: "B1 reuses the already-shipped `rotate_changelog` primitive (CAP-042) to automatically rotate the frontmatter `changelog:` array (BC-INDEX's dominant size driver...) once it overflows a configured item count." |
| L2 Domain Invariants | none (dispatcher runtime architectural invariant, not an L2 domain-spec DI-NNN) |
| Architecture Module | SS-01 (Hook Dispatcher Core — `shard_manager.rs` B1 artifact-shape handler) |
| ADR | ADR-051 §Decision 7 (BC-INDEX two-level structured-catalog sharding, B1 sub-mechanism, CORRECTED single-actor block-and-retry subsection; v1.2 single-evergreen-archive-file + bounded path-resolution-extension correction); ADR-049 §Decision 6 (`rotate_changelog` primitive, reused); §Decision 2 (`last_amended`/`changelog:` prepend discipline, respected unmodified) |
| Stories | S-25.02 |
| Cycle | v1.0-brownfield-backfill (F2 — product-owner spec-evolution burst) |
| Feature | E-25 — Validation Integrity and Large-Artifact Resilience |

## Changelog

| Version | Date | Author | Change |
|---------|------|--------|--------|
| 1.6 | 2026-09-11 | product-owner | Cluster-4 hardening amendments (S-25.02 mechanism B1, observations A and B per `s2502-cluster4-hardening-design.md`). **Obs A (counter-divergence guard):** Added EC-008 (item-count trigger fires but `rotate_changelog_at` returns `Ok(mutated=false)` — counter-method divergence between the B1 handler's serde `read_changelog_item_count` trigger path and `rotate_changelog_at`'s `parse_frontmatter` line-scan; provably latent on canonical `  - date:` format today but unguarded — `HookResult::Error(E-SHD-014)` structurally prevents the infinite block+retry self-DoS on `BC-INDEX.md`); added Invariant Inv-5 (`Ok(report)` with `!report.mutated` after fired trigger → `Error(E-SHD-014)`, never `Block` — first load-bearing use of `RotationReport.mutated` in this gate); added PC6 addendum (the `mutated=false` Error is the third fail-loud outcome of the rotation step, alongside `E-SHD-004` Err arm); added canonical test vector for EC-008. **Obs B (crash-idempotent archive boundary):** Added Invariant Inv-6 (crash-idempotent at archive boundary — archive NEVER accumulates duplicate items under archive-write-success + source-write-failure double-fault + retry; `rotate_changelog_at`'s idempotent-append tail-match dedup detects already-archived overflow items and skips archive write on retry; `RotationReport.mutated` remains `true` when source was rewritten even if archive write skipped; B1 analogue of ADR-051 D11 `E-SHD-006` mechanism-A self-heal, same "detection-and-resume, not rollback" philosophy); added PC5 addendum (no-duplicate guarantee explicit, cross-references Inv-6); added canonical test vector for crash-recovery no-duplicate case. **Error code note:** the hardening design doc (`s2502-cluster4-hardening-design.md`) cited `E-SHD-008` as the "next free slot"; however, `E-SHD-008` through `E-SHD-013` are already allocated in `error-taxonomy.md` v1.14 (`E-SHD-008` = `BackstopProbeFailed` BC-1.18.006 EC-019, allocated at taxonomy v1.3; codes run through `E-SHD-013` = `CanonicalWriteVerificationFailed`). Actual next free slot assigned: **`E-SHD-014`** (counter-method divergence guard). **VP reconciliation (same burst, formal-verifier):** §Verification Properties table updated: VP-126 extended with guard-arm-presence facet (EC-008/Inv-5 `mutated=false` guard arm); VP-131 extended to two-facet (Err-arm `E-SHD-004` + Ok-arm `E-SHD-014`); VP-112 added (crash-idempotent archive boundary, Inv-6, shared primitive). §VP Anchors updated to name all four VPs. **E-SHD-014 block-semantics clarification (same burst, coordinator):** Inv-5, EC-008, PC6, and EC-008 CTV clarified to state explicitly that `E-SHD-014` is a BLOCKING fail-loud Error (`block_intent=true`, `exit_code=2`, `on_error=Block`) and that "never `Block`" refers to the `HookResult::Block` VARIANT (retry instruction), NOT to dispatch-blocking; an implementation returning non-blocking/advisory `E-SHD-014` that allows the write to proceed is NON-COMPLIANT; the over-N write MUST be blocked. |
| 1.5 | 2026-09-06 | product-owner | Surgical residual-sweep fix-burst (adversary pass-7 finding F-P7-001, LOW — the last live N-1 straggler): Precondition 4's parenthetical describing `rotate_changelog`'s pure-trim retention as "the retained N-1 most-recent items" was a STALE descriptor of the WITHDRAWN v1.0/pass-2 trim target, contradicting this BC's own Postcondition 1/2 ("NEVER to N-1"; trims to the configured `low_water_mark`, BC-1.18.005 Postcondition 8's rotation-target config). CORRECTED to "the caller-supplied `keep_recent` most-recent items — for B1, the configured `low_water_mark`" — a generic, correct characterization matching Postcondition 1/2. No postcondition, invariant, or contract-behavior change; wording reconciliation only. Sibling-sweep (POLICY 5) confirmed no other positive (non-negated, non-changelog) `N-1`-as-trim-floor descriptor remains anywhere in this BC's live contract text. |
| 1.4 | 2026-09-05 | product-owner | Fix-burst amendment (adversary pass-3 findings F-P3-001 HIGH + F-P3-005 MEDIUM + F-P3-006 LOW, ADR-051 v1.3 Decision 14): **(F-P3-001)** CORRECTED Postcondition 6 and EC-003's error-code citation from `E-SHD-001` (BC-1.18.006's DIFFERENT mechanism-A shard-seal-write-failure code) to `E-SHD-004` (this BC's OWN `rotate_changelog`-invocation-failure code, per `error-taxonomy.md`'s Error Catalog and VP-INDEX's authoritative `004→VP-131` mapping — the v1.3 residual-cleanup micro-burst had mistakenly "corrected" this citation to the wrong sibling code). **(F-P3-005)** REWROTE Postcondition 1 (the sequence never exceeds N but is NOT capped at exactly N-1/N — rotation trims to the `low_water_mark` floor, BC-1.18.005 Postcondition 8's new rotation-target config, default `floor(N/2)`, never a hardcoded `N-1`) and Postcondition 2's opening sentence/step 3/step 4 (retry-instruction text and post-retry item count corrected from `N-1`/`N-1+1=N` to `low_water_mark`/`low_water_mark+1`); no change to the single-actor block-and-retry contract itself (Postconditions 3/4, Invariants 2-4 unchanged). Corrected EC-001's rotation-target wording; added EC-007 (the normal, amortized re-trigger boundary is not a defect) and a matching Canonical Test Vector demonstrating the amortized cadence; corrected the existing rotation Canonical Test Vector's numbers to `low_water_mark=25`. **(F-P3-006)** Collapsed the §Verification Properties table's two separate VP-125 rows into one multi-facet row (folding in the new `low_water_mark`-floor assertion); no coverage change beyond the genuinely new facet, presentation-only for the rest. |
| 1.3 | 2026-09-05 | product-owner | Residual-cleanup micro-burst (S-25.02 F2, formal-verifier-flagged gap ahead of the re-run adversary; no ADR/postcondition/invariant-set change — surgical wording reconciliation only). Invariant 2 and EC-001 still described the B1 rotation destination as "a sealed shard file" / "a new sealed shard" — a withdrawn per-`seq` shape v1.2 had already corrected everywhere else; both now read "the single evergreen archive file (`BC-INDEX-changelog-archive.md`)" to match Postcondition 2/5's v1.2 corrected mechanics. Postcondition 6 and EC-003's cross-reference to BC-1.18.006's EC-003 corrected from the withdrawn "seal-rename-failure" label to the current `E-SHD-001` "shard-seal-write failure" label (BC-1.18.006 v1.2's copy+atomic-truncate mechanism has no rename step). No behavior, postcondition-count, or invariant-count change — pure terminology reconciliation. |
| 1.2 | 2026-09-05 | product-owner | Fix-burst amendment (adversary pass-2 finding F-P2-001, HIGH, ADR-051 v1.2 Decision 7 CORRECTED): REWROTE Postcondition 2/5/Invariant 1's archive-path citations from the internally-impossible per-`seq` sealed-shard-DIRECTORY layout (`BC-INDEX-changelog-shards/BC-INDEX-changelog.<seq:04>.md` — a layout the shipped `rotate_changelog`/`resolve_archive_path` cannot produce under any call pattern) to the single, evergreen, append-only archive file `.factory/specs/behavioral-contracts/BC-INDEX-changelog-archive.md`, reached via a small, NAMED, bounded extension to `rotate_changelog`'s path-resolution surface (an explicit `archive_path` parameter, not yet implemented — grounded via a current-state SDK grep showing the shipped `resolve_archive_path(path, cycle_name)` signature's absence of this parameter). Refined Invariant 1's "no reimplementation" framing to acknowledge this bounded, additive extension without weakening the no-duplicated-logic guarantee. Updated the retry-instruction text, Canonical Test Vectors, Related BCs (added BC-1.18.012), Architecture Anchors, SDK Grounding Evidence, and VP Anchors accordingly. No change to the single-actor block-and-retry contract itself (Postconditions 1/3/4/6, Invariants 2-4 unchanged from v1.1). |
| 1.1 | 2026-09-05 | product-owner | **BLOCKER fix-burst amendment (F-S2502-F2-001, ADR-051 v1.1 Decision 7 "CORRECTED" subsection).** Postconditions 2 and 6 REWRITTEN and Invariant 1 REWRITTEN (with a new Invariant 4) to withdraw the v1.0 double-actor "gate rotates AND prepends, then Continues" design, which was unsound: (a) double-actor prepend hazard (gate and agent both perform the identical insert action); (b) stale-payload clobber for `Write`/`MultiEdit` (a pre-rotation full-file payload landing over a just-rotated file silently re-introduces the archived tail, causing infinite rotation churn) — grounded in `rotate_changelog`'s own verified pure-trim signature (no `prepend_changelog_item` call, no new-item parameter). CORRECTED contract: the gate performs ONLY the rotate/trim step, THEN returns `HookResult::Block` with a retry instruction; the new item's prepend is EXCLUSIVELY the responsibility of whichever `Edit`/`Write`/`MultiEdit` call ultimately lands (original or retried) — a strict structural mirror of BC-1.18.006's block-and-retry contract. Added EC-006 (static-detection of a reintroduced gate-side prepend as a defect) and updated Canonical Test Vectors/VP table notes accordingly. H1 title amended to "... — Single-Actor Block-and-Retry" to make the corrected contract load-bearing in the title per BC H1 Title Authority. Added `## SDK Grounding Evidence` section (F-S2502-F2-007). VP-125/126 flagged for formal-verifier re-review (not actioned this burst — VP bodies are formal-verifier's domain). |
| 1.0 | 2026-09-05 | product-owner | Initial creation (NEW BC, not in the original F1 enumeration — mechanism B1, added per D-1166 human widest-scope decision covering BC-INDEX.md). Automatic size-triggered invocation of the already-shipped `rotate_changelog`/`prepend_changelog_item` primitives via the same native gate BC-1.18.006 defines, with a "Continue after rotation" divergence from mechanism A's block-and-retry (justified: rotation always succeeds in making room for a bounded-size changelog item prepend). CAP-043 capability anchor. ADR-051 §D7 + ADR-049 §D2/§D6 citations. **[WITHDRAWN by v1.1 — see above; this row is retained per POLICY 1 append-only history, not as a currently-valid contract description.]** |
