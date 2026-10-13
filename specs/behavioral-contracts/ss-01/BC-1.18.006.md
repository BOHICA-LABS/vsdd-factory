---
document_type: behavioral-contract
level: L3
version: "1.12"
status: active
producer: product-owner
timestamp: 2026-09-07T00:00:00Z
phase: F2
inputs:
  - .factory/specs/architecture/decisions/ADR-051-layer-2-two-mechanism-size-triggered-shard-rotation-append-logs-and-bc-index-sharding.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.005.md
  - crates/hook-sdk/src/result.rs
  - .factory/cycles/v1.0-brownfield-backfill/S-25.02-f2-architecture-delta.md
input-hash: "cd637ec"
traces_to: .factory/specs/prd.md
origin: greenfield
extracted_from: null
subsystem: "SS-01"
capability: "CAP-043"
lifecycle_status: active
introduced: v1.0-brownfield-backfill
modified: []
deprecated: null
deprecated_by: null
replacement: null
retired: null
removed: null
removal_reason: null
---

# BC-1.18.006: Roll-Before-Write via Block-and-Retry (Not Transparent Redirection) Plus Same-Invocation Atomic Shard-Index Publication

## Description

When BC-1.18.005's size-trigger fires, the dispatcher performs the roll (publish a sealed shard
copy of the current content as a NEW file, then atomically REPLACE the canonical file's content
with empty — see the CORRECTED mechanism below — then atomically publish the updated shard index)
and THEN returns `HookResult::Block` with an explicit, actionable retry instruction — never a
silent transparent redirect, which `HookResult`'s three-variant contract (`Continue`/
`Block { reason }`/`Error { message }`) makes structurally impossible. The blocked call is never
applied, so no shard is ever observed over cap by any downstream reader, and the sealed shard plus
its index update land in the SAME native-gate invocation, guaranteeing they are staged in the same
subsequent factory-artifacts commit (TD-VSDD-053 alignment).

**CORRECTED (fix-burst pass-2, F-P2-003, HIGH) — the seal step is COPY-then-ATOMIC-TRUNCATE-
IN-PLACE, NEVER a rename of the canonical path away.** The v1.0/v1.1 text above ("seal by rename")
described the seal as `rename(canonical, sealed)` followed by a separate `create(canonical)` — two
distinct filesystem operations with an interstitial window, between the rename completing and the
fresh-file create completing, during which the canonical path DOES NOT EXIST ON DISK AT ALL. Any
shard-unaware reader (the ~76 fail-open production plugins with directory-scoped `path_allow`
globs, `check_d_chain_currency`, a human `cat`) that happens to `open()` the canonical path inside
that window observes `ENOENT` — a hard failure, not a stale-but-valid read — directly contradicting
this BC's own AC-007-derived "zero-code-change transparency" guarantee and this BC's own Invariant
3 text ("the canonical filename is NEVER renamed away; only its CONTENT is replaced"), which the
withdrawn rename-based mechanism structurally could not satisfy. See Postcondition 1's corrected
text below for the exact replacement sequence: steps (c) (canonical truncate) and (d) (index
publish) reuse ONLY the already-established `write_atomic`
(`crates/last-amended-migrate/src/atomic_write.rs`) temp-file-then-rename primitive — no
reimplementation there.

**CORRECTED (cluster-2 LOCAL adversary pass-8, F-C2-P8-001, MEDIUM) — step (b) itself (the
seal-publish) is the ONE exception to "no new atomic-write primitive," and this text's prior
"reuses ONLY `write_atomic`" claim overreached.** The v1.2-v1.8 text describing step (b) as
`write_atomic`'s rename-based create was accurate only through v1.7; it stopped being accurate
once Postcondition 8 (v1.8) REQUIRED step (b) to refuse to complete if the destination already
exists. A plain `rename(2)` unconditionally OVERWRITES an existing destination — it structurally
cannot express a "refuse if it exists" contract on its own — so Postcondition 8's write-once
guarantee could only be satisfied by a genuinely new, no-clobber primitive, `write_exclusive`
(`std::fs::hard_link` onto a not-yet-existing destination, which fails atomically with
`ErrorKind::AlreadyExists` rather than silently overwriting), introduced specifically for step (b)
and used by `publish_sealed_shard`. This is a NARROW exception, scoped to step (b) alone: steps (c)
and (d) are unaffected and remain exactly as described above.

## Preconditions

1. BC-1.18.005's size-trigger has determined `projected_size > shard_cap_bytes` for a matched
   `Edit`/`Write`/`MultiEdit` tool call.
2. The current shard file for the matched artifact exists (or is treated as a zero-byte current
   shard per BC-1.18.005 EC-004 if this is the artifact's first-ever write).
3. `crates/hook-sdk/src/result.rs`'s `HookResult` enum exposes exactly three variants (`Continue`,
   `Block { reason }`, `Error { message }`) with no redirect or tool-input-mutation capability —
   this precondition is a structural SDK fact, not a runtime state, and is what makes this BC's
   block-and-retry design the ONLY implementable option for roll-before-write under the current
   dispatcher contract.

4. **NEW (F2 spec-evolution, closing BC-1.18.005 v1.11 Postcondition 3's deferred `replace_all: true`
   occurrence-multiplicity gap; S2502-CLUSTER1-PASS5 STATE.md Drift Item).** BC-1.18.005's
   Postcondition 3 `Edit`/`MultiEdit` formula computes `net_delta_bytes` as a SINGLE-occurrence
   `len(new_string) - len(old_string)` and does not multiply by
   `occurrence_count(old_string, current_file_content)` for a `replace_all: true` call — an
   explicit, adjudicated, DEFERRED gap (BC-1.18.005 Postcondition 3's "Known formula gap"
   sub-paragraph) that this BC MUST NOT treat its roll/block outcome as production-ready against
   until closed. This BC closes the gap WITHOUT modifying BC-1.18.005's already-ACTIVE, already-
   shipped trigger contract — it adopts Option (b) of BC-1.18.005's own pre-authorized closure fork
   ("BC-1.18.006's own roll-execution path... independently re-validates the post-apply size...
   using the ACTUAL applied content length, catching an under-counted trigger before or immediately
   after write"), not Option (a) (amending BC-1.18.005's formula itself). BC-1.18.005's PreToolUse
   single-occurrence estimate remains EXACTLY as BC-1.18.005 specifies it, unchanged; this BC
   instead adds an independent, narrowly-scoped POST-WRITE reconciliation check (Postcondition 7)
   that catches and corrects any resulting under-projection using the artifact's ACTUAL on-disk
   size — no occurrence counting is performed anywhere by this BC. This closure scope applies ONLY
   to an `Edit` call carrying `replace_all: true` and to a `MultiEdit` call containing at least one
   edit block with `replace_all: true`; a plain `Edit`/`MultiEdit`/`Write` without `replace_all:
   true` is unaffected by this Precondition or by Postcondition 7 (BC-1.18.005's formula is already
   exact for those cases, per its Postcondition 3's own UNCHANGED-leg text).

## Postconditions

1. **CORRECTED (fix-burst pass-2, F-P2-003/F-P2-004, HIGH/MEDIUM) — the roll sequence is a
   STAGED, four-step, crash-recoverable operation that executes BEFORE the block is returned:**
   (a) **read** the canonical file's current full content (a one-time, roll-only read — the cheap
   per-write TRIGGER check, BC-1.18.005 Postcondition 2, remains `stat()`-only; content is read
   ONLY once a roll is already confirmed necessary); (b) **publish the sealed shard as a brand-NEW
   file** at `<stem>.<seq:04>.md` (e.g. `decision-log.0001.md`) via `publish_sealed_shard`'s
   exclusive-create primitive, `write_exclusive` — **CORRECTED (cluster-2 LOCAL adversary pass-8,
   F-C2-P8-001, MEDIUM): NEVER a `rename()`.** A plain `rename()` unconditionally OVERWRITES an
   existing destination and therefore cannot itself enforce a write-once, no-clobber guarantee;
   `write_exclusive` instead creates the sealed file via `std::fs::hard_link` onto a not-yet-existing
   destination, which fails atomically (`ErrorKind::AlreadyExists`) if the destination is already
   occupied — closing the TOCTOU window a separate stat()-then-rename check would leave open. This
   never interrupts any reader of the canonical path, since sealed filenames are never read by
   shard-unaware code. **write-once: `publish_sealed_shard` MUST refuse to complete, and MUST NOT
   overwrite, if the destination path already exists on disk and is non-empty — which is precisely
   why step (b) uses this exclusive-create primitive rather than `write_atomic`'s rename-based
   create (see Postcondition 8, its 0-byte-destination exception, and the corrected Invariant 10)**;
   (c) **atomically REPLACE
   the canonical file's content with empty**, via the SAME `write_atomic` temp-file-then-rename
   primitive — write an empty temp file, then `rename(temp, canonical)`, which is an atomic
   directory-entry REPLACEMENT of an EXISTING destination (POSIX `rename(2)`; the dispatcher's
   Windows target uses `MoveFileEx` with `MOVEFILE_REPLACE_EXISTING`), never a delete-then-create
   — the canonical path resolves to SOME valid file (old content, then instantaneously the new
   empty content) at every observable instant, never absent; (d) **atomically publish the updated
   shard-index TOML** (temp-file-then-rename, the same pattern already established by
   `write_indeterminate_marker` and `write_atomic` — no new atomic-write primitive is introduced).
   Only after (a)-(d) complete does the gate return `HookResult::Block`. **This WITHDRAWS the
   v1.0/v1.1 "seal by rename-away, then create a fresh file" mechanism**, which opened a real
   ENOENT window between the rename and the create (see Description above) — step (b) is a
   `rename()` that CREATES a new sealed path (never vacates the canonical one), and step (c) is a
   `rename()` ONTO the existing canonical path (an atomic replace-in-place), so the canonical path
   is NEVER, at any instant, absent from disk.

   **CORRECTED (cluster-2 LOCAL adversary pass-4, F-C2-P4-004, ADVISORY) — step (a)'s read and step
   (b)'s seal-publish write MUST be BYTE-LEVEL, never UTF-8-fallible.** The mechanism above reads the
   canonical file's content and publishes it as a sealed shard using raw bytes (`std::fs::read` /
   `std::fs::write`, or equivalent), NOT a UTF-8-fallible `read_to_string`/`&str` path. A non-UTF-8
   byte sequence in the canonical file (however it arrived there — a corrupted transfer, a pasted
   binary fragment) must not surface as an `E-SHD-001` failure that PERMANENTLY blocks all future
   writes to that artifact merely because `execute_roll` chose a string-typed read where a byte-typed
   one would have succeeded. This BC's own self-heal reconciliation (F-C2-P1-001, Postcondition 1's
   `E-SHD-006`/`E-SHD-007` recovery, and Invariant 9's 0-byte guard) already reads sealed-shard
   candidates at the byte level; `execute_roll`'s own prospective/retroactive read-and-seal path must
   be consistent with that established baseline, not a UTF-8-fallible exception to it. **CODE CHANGE
   ROUTED (→ implementer):** change `execute_roll`'s canonical-file read and its sealed-shard write to
   byte-level I/O. **TEST ROUTED (→ test-writer):** a fixture writing a non-UTF-8 byte sequence into a
   matched artifact's canonical file (below cap, so no trigger fires on write) followed by a
   triggering call that pushes it over cap, asserting the roll succeeds (byte-for-byte seal,
   byte-for-byte canonical truncate) rather than failing `E-SHD-001` on decode.

   **NEW (cluster-2 LOCAL adversary pass-3, F-C2-P3-001, MAJOR) — the four-step sequence above
   applies ONLY when the canonical file's content is non-empty at the moment of step (a)'s read;
   an empty canonical short-circuits to `Ok(None)` and skips ALL FOUR steps.** When BC-1.18.005's
   size-trigger fires (Precondition 1: `projected_size > shard_cap_bytes`) but the canonical
   file's content is exactly 0 bytes at the moment step (a) would read it, `execute_roll` performs
   NONE of steps (b)-(d) and returns `Ok(None)` rather than executing a zero-content roll. This is
   not an omission but a deliberate, correct short-circuit: a 0-byte canonical means the trigger
   fired solely because the INCOMING call's own payload (e.g. a `Write`'s `content`, or an `Edit`'s
   `new_string` applied against an already-empty shard per BC-1.18.005 EC-004) exceeds
   `shard_cap_bytes` on its own — there is no PRE-EXISTING history to preserve, so publishing a
   sealed shard would create a `<stem>.<seq:04>.md` file with zero informational content, permanently
   consuming a `seq` slot in the shard-index for nothing, and appending a `[[shard]]` entry with
   `bytes_at_seal = 0` that documents no actual sealed history (see Invariant 9's general ban on
   zero-byte index entries, which this short-circuit is what makes universally true for THIS BC's
   own write path — Invariant 9 also binds the self-heal reconciliation paths, which do not go
   through `execute_roll` at all). Concretely: NO shard file is published (step (b) never runs), the
   canonical file is left untouched at its existing 0 bytes (step (c) never runs — there is nothing
   to truncate), and NO `[[shard]]` index entry is appended (step (d) never runs). The trigger-fire
   branch still returns `HookResult::Block` (the oversized call is still never applied — Invariant 1
   is unaffected), but via the SEPARATE, distinct template specified in Postcondition 2's
   "Empty-canonical retry template" clause, not the unified rotate-and-retry template, because there
   is nothing to "rotate": the caller's own payload, not accumulated history, is what exceeds the
   cap, and the unified template's "the current shard is now empty; retry against the current
   (post-roll, empty) file" framing would be actively misleading (the shard was ALREADY empty before
   this call; retrying unchanged content against it would simply exceed the cap again). See
   EC-021 and its matching Canonical Test Vector.

   **Partial-failure postconditions — one named `E-SHD-NNN` code per crash point (ADR-051
   Decision 11), because this composite three-write operation (steps b/c/d) has THREE distinct
   crash points, not one — none of these crash points can occur for the `Ok(None)` empty-canonical
   short-circuit above, since it performs zero writes:**
   - **Steps (a)-(b) fail (`E-SHD-001`, description REFINED from "seal-rename failure" to
     "shard-seal-write failure" to match the corrected copy-based mechanism — the CODE and
     observable contract are unchanged: `HookResult::Error`, canonical file completely
     untouched):** the canonical file is left in its exact pre-roll state (still over cap, still
     holding its full original content) — safe, no data loss, no duplicate; the next dispatch
     attempt re-evaluates the trigger and re-attempts the FULL sequence from step (a).
   - **Step (c) fails after step (b) succeeded (NEW `E-SHD-006`):** the sealed shard now durably
     exists (a byte-for-byte copy of the pre-roll content) AND the canonical file STILL holds that
     same content too (not yet truncated) — a transient, DETECTABLE duplicate-content state, not a
     data-loss state. **Recovery (self-healing, no operator intervention):** on the NEXT dispatch
     attempt for this artifact, BEFORE evaluating any new trigger, the gate checks whether a
     sealed shard exists at the index's next-expected `seq` path whose content is byte-identical
     to the canonical file's CURRENT content; if so, this is recognized as "seal published,
     truncate did not," and the gate resumes from step (c) alone (re-attempting ONLY the truncate
     + index publish, never re-writing the already-correct sealed shard) — idempotent by
     construction: the recovery logic never reissues step (b) at all. **CORRECTED (cluster-2 LOCAL
     adversary pass-8, F-C2-P8-001, MEDIUM):** this idempotency does NOT additionally rest on step
     (b)'s own create being "a no-op if reissued against identical content" — that characterization
     was true only under the withdrawn `write_atomic`-rename description of step (b) and is FALSE
     since Postcondition 8 (v1.8): step (b) is now `write_exclusive`, an exclusive-create primitive
     that refuses ANY re-publish attempt against an already-occupied, non-empty destination
     (byte-identical or not), so a hypothetical reissue of step (b) here would now surface
     `E-SHD-009`, never silently no-op. The recovery path's correctness has never depended on step
     (b) being reissue-safe — it depends only on step (b) never being reissued, which remains true.
     **This recovery premise — that the SAME artifact's next dispatch attempt
     reconciles the orphan BEFORE any new roll can be attempted — MUST hold across EVERY
     roll-triggering entry point, not only the PreToolUse Flat arm (CORRECTED, cluster-2 LOCAL
     adversary pass-4, F-C2-P4-001): see Postcondition 7 catch point (i)'s corrected mechanism text
     below, which previously omitted this pre-pass for its own retroactive-roll entry point, and the
     corrected Invariant 10.**
   - **Step (d) fails after step (c) succeeded (NEW `E-SHD-007`):** the canonical file is
     CORRECTLY fresh and empty (safe for all future writes — no over-cap risk, no data loss) and
     the sealed shard file exists correctly on disk, but `<artifact-stem>.shard-index.toml` has
     not yet recorded the new `[[shard]]` entry — a discoverability-METADATA gap only:
     whole-corpus glob-based readers (`<stem>*.md`) still find the sealed file regardless of index
     membership, so no reader-visible data loss occurs. **Recovery (self-healing):** on the next
     dispatch attempt, the gate reconciles the index by scanning the filesystem for sealed-shard
     files matching the artifact's naming convention that are absent from the index, and appends
     the missing entries before evaluating any new trigger.

     **CORRECTED (cluster-2 LOCAL adversary pass-1, F-C2-P1-001, MAJOR) — the reconciled entry MUST
     recover `sealed_retroactively` by inference, never hardcode it `false`.** Postcondition 5
     designates `sealed_retroactively` the SOLE audit trail distinguishing a legitimately-over-cap
     sealed shard from one guaranteed `<= shard_cap_bytes`. If the orphaned entry belonged to a
     RETROACTIVE roll (Postcondition 7 catch point (i) or (ii), which crashed after its OWN step (c)
     but before its OWN step (d)), a filesystem-only reconciliation that is blind to which roll
     produced the file and defaults the field to `false` would mislabel a genuinely over-cap shard
     as cap-guaranteed — the exact ambiguity Postcondition 5 exists to prevent, and the only record
     of the roll's retroactiveness (the in-flight roll's own knowledge of which path invoked it) is
     lost the moment step (d) fails to durably record it. The value is, however, deterministically
     INFERRABLE from data already available to the reconciliation scan, with no in-memory roll
     context required: `bytes_at_seal > shard_cap_bytes ⇒ the seal must have been retroactive`,
     because a PROSPECTIVE (Postcondition 1, non-retroactive) roll can never seal over-cap content —
     Postcondition 3's unconditional guarantee ensures every prospectively-sealed shard's
     `bytes_at_seal <= shard_cap_bytes`, while only a retroactive roll's documented exception
     (Postcondition 7's "Sealed-shard cap exception") permits `bytes_at_seal > shard_cap_bytes`. This
     BC therefore REQUIRES: when reconciling an orphaned, un-indexed sealed shard, the gate MUST
     compute `bytes_at_seal` from the sealed file's actual on-disk size (already required to
     populate the recovered `[[shard]]` entry's `bytes_at_seal` field) and set
     `sealed_retroactively = true` if `bytes_at_seal > shard_cap_bytes`, else `false` (or omit, per
     Postcondition 5's default) — never a hardcoded `false` regardless of size. See Invariant 7 for
     the general form of this inference rule.
   - **All four steps succeed:** normal `Block` outcome (Postcondition 2), no error.

2. **CORRECTED (fix-burst pass-2, F-P2-002/F-P2-003, HIGH) — the observable outcome of an over-cap
   write is `HookResult::Block` with a specific, actionable, UNIFIED retry-instruction message —
   NOT a silent transparent redirect, and NOT a tool-divergent instruction.** The block reason
   text MUST include: the artifact name, the cap that was reached (in bytes), the fact that the
   current shard is now empty, and a SINGLE unified retry instruction (the SAME wording regardless
   of the original tool, since both branches now converge on "recompute against the current,
   post-roll state"):

   > "Shard `<artifact>` rotated (cap `<N>` bytes reached); the current shard is now empty. Retry
   > your write against the CURRENT (post-roll, empty) file — do not resubmit your original
   > payload unchanged: if you used `Edit` or `MultiEdit`, your `old_string` will no longer match
   > (the content it targeted is now in `<sealed-path>`) — reissue as a fresh `Write` containing
   > ONLY your new entry; if you used `Write`, recompute `content` to contain ONLY your new entry
   > (not your original full pre-roll payload, which reflects discarded state and will exceed the
   > cap again if resubmitted)."

   **Empty-canonical retry template (NEW, cluster-2 LOCAL adversary pass-3, F-C2-P3-001, MAJOR) —
   a SECOND, distinct, sanctioned template for the Postcondition 1 empty-canonical short-circuit;
   see Invariant 4's adjudicated scoping.** When Postcondition 1's `Ok(None)` short-circuit fires
   (canonical already 0 bytes; the incoming call's own payload alone exceeds the cap), the gate
   returns `HookResult::Block` via a SEPARATE template, `build_empty_roll_retry_block_reason`,
   whose wording communicates a DIFFERENT fact than the unified rotate-and-retry template: there is
   no history to rotate away from, so splitting the payload — not merely recomputing against a
   freshly-emptied shard — is the only viable retry path. The block reason text MUST include: the
   artifact name, the cap that was reached (in bytes), the fact that the shard is ALREADY empty
   (not "now empty as a result of this call"), and an instruction to recompute or split the payload
   itself, distinct from the unified template's "retry against the current post-roll state"
   framing:

   > "Shard `<artifact>` is already empty; your own payload alone (`<N>` bytes) exceeds the cap
   > (`<shard_cap_bytes>` bytes). Recompute or split your payload into multiple smaller calls — no
   > roll was performed, because there is no existing content to rotate away; the shard remains
   > exactly as it was before this call."

   This is deliberately WORDED DIFFERENTLY from the unified template (Invariant 4, as adjudicated
   below) because it tells the agent something the unified template cannot: retrying with
   "the same payload against the current empty file" (the unified template's advice) is not new
   information here — the file was ALREADY the empty target, and the SAME oversized payload will
   fail identically on retry unless the agent actually reduces or splits it. Conflating the two
   messages would either omit the split-guidance the empty-canonical case uniquely needs, or
   falsely imply a rotation occurred when it did not.

   **Double-fire exception — Case B2, a THIRD sanctioned template (NEW, cluster-2 LOCAL adversary
   pass-8, F-C2-P8-004, MINOR).** The wording immediately above ("no roll was performed... the
   shard remains exactly as it was before this call") is accurate ONLY when the canonical file was
   ALREADY 0 bytes at the START of this dispatch, with no roll of any kind occurring during it —
   the "pure" empty-canonical case (Case B1). It OVERCLAIMS on the "double-fire" path: Postcondition
   7 catch point (ii)'s leading backstop probe can retroactively seal+truncate a PRE-EXISTING,
   orphaned, over-cap canonical BEFORE this dispatch's own trigger is even evaluated; if this call's
   OWN payload then also exceeds the cap against that now-freshly-emptied (not originally-empty)
   canonical, Postcondition 1's `Ok(None)` short-circuit fires a SECOND time within the SAME
   dispatch. On that path, a roll DID occur (the backstop's retroactive roll) and the shard does NOT
   "remain exactly as it was before this call" (it was over cap before this call started, and is
   empty now only because of a roll THIS SAME dispatch performed) — asserting otherwise is false.
   The gate MUST select a distinct, THIRD template — `build_empty_roll_retry_block_reason`
   parameterized with `preceded_by_backstop_roll: true` — dropping the "no roll was performed"/
   "remains exactly as it was before this call" clauses while keeping the identical, actionable
   split-payload guidance:

   > "Shard `<artifact>` is now empty (a prior over-cap shard was retroactively rotated by this
   > same call before your payload was evaluated); your own payload alone (`<N>` bytes) exceeds
   > the cap (`<shard_cap_bytes>` bytes). Recompute or split your payload into multiple smaller
   > calls."

   Selection between Case B1's and Case B2's wording is a pure function of whether Postcondition 7
   catch point (ii) executed a retroactive roll earlier in THIS SAME dispatch — never randomized,
   never combined, never conflated with Case A. **CODE CHANGE ROUTED (→ implementer):**
   `build_empty_roll_retry_block_reason` gains a fourth parameter, `preceded_by_backstop_roll:
   bool`, threading through whether catch point (ii) fired during this dispatch (a fact the gate
   already has, from its own dispatch-local control flow — no new tracking state introduced).
   **TEST ROUTED (→ test-writer):** a fixture reproducing the double-fire sequence (a pre-existing
   orphaned over-cap canonical reclaimed by catch point (ii), immediately followed by this SAME
   call's own over-cap payload against the now-empty canonical), asserting Case B2's wording is
   emitted verbatim (never Case B1's "no roll was performed" wording, never the unified Case A
   template) — see EC-026 and its matching Canonical Test Vector.

   **ADJUDICATED (cluster-2 LOCAL adversary pass-4, F-C2-P4-003, MINOR) — the `(<N> bytes)` payload
   size in this template is REQUIRED, not decorative, and `build_empty_roll_retry_block_reason` MUST
   accept the incoming payload's length as a parameter to produce it.** The shipped implementation's
   `build_empty_roll_retry_block_reason(artifact_stem, shard_cap_bytes)` signature cannot emit `<N>`
   (it has no payload-length input), which diverges from this Postcondition's own message text and
   from EC-021's Canonical Test Vector — both already specify the payload size inline. Adjudicated
   under CLAUDE.md's production-grade lens: Option (b) (thread the payload length through) is ADOPTED
   over Option (a) (drop `<N>` from the spec to match the narrower signature), because telling the
   agent its OWN payload size relative to the cap is genuinely useful, actionable UX — the agent
   learns immediately that splitting (not merely retrying) is required, without separately
   determining its own payload's byte count. This Postcondition's message text above is UNCHANGED by
   this decision (it was already correct); only the shipped function signature is brought into
   alignment (CLAUDE.md precedence rule 12: spec wins on a code-vs-spec conflict). **CODE CHANGE
   ROUTED (→ implementer):** `build_empty_roll_retry_block_reason` gains a third parameter,
   `payload_len_bytes: usize` (the incoming call's own payload length — `len(content)` for `Write`,
   the post-`replace_all`-substitution projected length for `Edit`/`MultiEdit`, matching whichever
   length BC-1.18.005's own trigger already computed to fire in the first place — no new computation,
   reuse the value already in hand), interpolated at the exact `<N>` position shown above. **TEST
   ROUTED (→ test-writer):** pin the exact expected message text shown above verbatim, table-driven
   over at least one `Write` case and one `Edit{replace_all: true}` case, asserting `<N>` reflects
   each call's own actual payload length precisely.

   **This WITHDRAWS the v1.0/v1.1 "if you used `Write`, simply retry unchanged" wording**, which
   was UNSOUND under BOTH the withdrawn rename mechanism and the corrected per-tool
   `projected_size` formula (BC-1.18.005 Postcondition 3, F-P2-002): because the canonical file is
   now EMPTY after a roll, and because a blocked `Write`'s own `content` parameter was composed by
   the agent BEFORE the roll (typically by reading the OLD, over-cap file and appending one new
   entry), retrying that SAME `content` unchanged would resubmit content that is STILL over cap
   relative to the fresh empty shard (since `projected_size = len(content)` for `Write`, per the
   corrected formula, and `len(content)` has not shrunk) — producing a permanent block/retry
   deadlock, not a duplicate. This is a structural consequence of `HookResult`'s three-variant
   contract (Precondition 3): no design that assumes transparent write-redirection is
   implementable, so the postcondition describes the message an agent WILL see, not a hypothetical
   silent success.

3. **No shard is ever observed in an over-cap state by any downstream reader.** Because the seal
   happens before the block (Postcondition 1), and the blocked call is never applied to any file,
   the sealed shard's final size is always `<= shard_cap_bytes` (BC-1.18.005's cap, evaluated at
   seal time) and the new current shard starts at exactly 0 bytes. This is a structural guarantee,
   not a convention: it holds even if the agent never retries (the sealed shard is already
   durably capped; only the RETRY's content is lost if the agent abandons the operation).

4. **Shard index and sealed shard land in the same native-gate invocation.** The shard-index TOML
   write (Postcondition 1 step (d)) and the sealed-shard publish (step (b)) are both filesystem
   writes issued by the SAME PreToolUse invocation, before any `git add`/`git commit` occurs downstream.
   This makes TD-VSDD-053's single-commit-per-burst hold STRUCTURALLY for shard+index atomicity
   (not merely by state-manager discipline), because both writes are guaranteed to be present in
   the working tree before the next `git commit` regardless of which agent or skill issued the
   original tool call.

5. **Shard-index schema (one file per sharded mechanism-A artifact):**
   ```toml
   # .factory/cycles/<cycle>/<artifact-stem>.shard-index.toml
   schema_version = 1
   artifact_stem = "decision-log"
   current_shard = "decision-log.md"
   shard_cap_bytes = 49152           # calibrated per BC-1.18.005; locked at F4
   max_single_record_bytes = 16384
   safety_margin_bytes = 8192
   practical_fuel_ceiling = 8000000
   worst_case_fuel_per_byte = 106.36

   [[shard]]
   seq = 1
   path = "decision-log.0001.md"
   sealed_at = "2026-09-10T00:00:00Z"
   bytes_at_seal = 49087
   ```
   Every seal event appends exactly one new `[[shard]]` table entry with `seq` incrementing
   monotonically from 1, `path` naming the sealed file, `sealed_at` in UTC ISO-8601, and
   `bytes_at_seal` recording the sealed shard's exact final byte count (always `<= shard_cap_bytes`
   per Postcondition 3).

   **NEW field (F2 spec-evolution, replace_all closure) — `sealed_retroactively` (boolean, OPTIONAL,
   default `false` when omitted — backward compatible with every `[[shard]]` entry produced before
   Postcondition 7 existed).** A seal produced by Postcondition 7's retroactive reconciliation path
   (either catch point (i) or catch point (ii)) MUST set `sealed_retroactively = true`; every seal
   produced by Postcondition 1's normal pre-write block-and-retry path MUST omit the field or set it
   `false`. This is the sole audit trail distinguishing a shard whose `bytes_at_seal` may
   legitimately exceed `shard_cap_bytes` (Postcondition 7's documented, narrowly-scoped exception)
   from one that is guaranteed `<= shard_cap_bytes` (Postcondition 3's normal, unconditional
   guarantee). **CORRECTED (cluster-2 LOCAL adversary pass-1, F-C2-P1-001, MAJOR):** when this field
   must be RECOVERED during self-heal reconciliation of an orphaned, un-indexed sealed shard
   (Postcondition 1's `E-SHD-007` recovery path), the gate MUST NOT default it to `false`
   unconditionally — it MUST apply the deterministic inference rule specified in Invariant 7
   (`bytes_at_seal > shard_cap_bytes ⇒ sealed_retroactively = true`).

6. **Stable-current-filename addressing is a consequence of this BC's seal mechanism, not a
   separate lookup step.** Because the seal PUBLISHES a copy of the old content as a NEW sealed
   file and ATOMICALLY REPLACES the canonical file's content with empty (CORRECTED, F-P2-003 —
   never renames the canonical path away), any shard-unaware reader or validator that opens the
   canonical filename always sees the current/latest shard, with zero code change required on the
   reader's part. Whole-corpus readers use the glob `<stem>*.md` (e.g. `decision-log*.md`), which
   sorts sealed shards ascending before the current file — the deciding byte-comparison one
   position past the shared `<stem>` prefix is a digit (`0`-`9`, from a sealed shard's `.NNNN.md`
   suffix) vs. `m` (from the current file's own `.md` suffix); since every digit byte is
   numerically less than `m`, sealed shards sort first (ADR-051 §Decision 3 fix-burst-corrected
   sort-order rationale — the "current file sorts last" conclusion is unchanged) — no
   special-casing needed in a `sort`-fed pipeline.

7. **NEW (F2 spec-evolution, closing BC-1.18.005 v1.11 Postcondition 3's deferred `replace_all:
   true` occurrence-multiplicity gap, S2502-CLUSTER1-PASS5) — bounded post-write reconciliation for
   `replace_all: true` `Edit`/`MultiEdit` calls.**

   **Why a post-write check, not a pre-write formula fix.** BC-1.18.005's Postcondition 3 PreToolUse
   trigger is, by Precondition 4 above, left unchanged: it evaluates `projected_size` using the
   single-occurrence delta BEFORE the write is applied. When the call carries `replace_all: true`
   and the TRUE occurrence-multiplied delta would have pushed `projected_size` over
   `shard_cap_bytes` while the single-occurrence estimate did not, the trigger returns `Continue`
   and the tool call is applied normally by the underlying editor — this BC's own block-and-retry
   roll (Postcondition 1) is a downstream CONSEQUENCE of BC-1.18.005's trigger firing (Precondition
   1) and is therefore structurally never invoked for a call the trigger itself failed to flag.
   Correcting this requires either changing BC-1.18.005's own trigger formula (Option (a) of
   BC-1.18.005's pre-authorized closure fork — amending an ALREADY-ACTIVE, already-shipped BC's
   contract, NOT taken here) or catching the resulting under-count independently, after the fact,
   using the artifact's true on-disk state (Option (b), specified below). Option (b) needs NO
   occurrence counting at all: a `stat()` of the ACTUAL post-apply file gives the TRUE size
   directly, exactly as it will for the artifact's own next trigger evaluation; no
   `len(new_string)`/`len(old_string)`/occurrence-count arithmetic is required or performed by this
   Postcondition.

   **Scope — catch point (i) fires only for the narrow `replace_all` case; catch point (ii)
   backstops EVERY subsequent matched dispatch, INCLUDING `Write` (CORRECTED, cluster-2 LOCAL
   adversary pass-1, F-C2-P1-002).** Catch point (i) (the immediate post-write reconciliation
   check) runs ONLY for an `Edit` call whose `replace_all` field is `true`, or a `MultiEdit` call
   containing at least one edit block whose `replace_all` field is `true`, AND whose target path
   matches a `[[shard]]` config entry (the SAME match BC-1.18.005 Precondition 3/Postcondition 1
   already performs — no additional config lookup); a plain `Edit`/`MultiEdit` without
   `replace_all: true`, a `Write`, or any call against an unmatched path never triggers catch point
   (i) itself, and pays zero added cost from it. Catch point (ii), by contrast, MUST run before
   EVERY subsequent matched dispatch — `Edit`, `Write`, or `MultiEdit` alike — because its job is
   to detect a crash-orphaned over-cap canonical file left behind by ANY failure of catch point (i),
   regardless of what tool the NEXT call happens to use; scoping catch point (ii) to `replace_all`
   calls only would leave exactly the data-loss gap F-C2-P1-002 identified (an intervening `Write`
   could destroy the orphaned history before a `replace_all` call ever came along to trigger a
   check). The withdrawn "a `Write` is completely unaffected" claim applied correctly to catch
   point (i) alone but was over-generalized to imply Postcondition 7 as a whole imposes zero cost on
   `Write` — corrected: `Write` pays one dedicated, bounded `stat()` per matched dispatch for catch
   point (ii) (see catch point (ii) above), while remaining entirely exempt from catch point (i).

   **Mechanism — two redundant catch points, bounding the observable-over-cap window to at most one
   subsequent matched dispatch:**
   - **(i) Immediate post-write check (primary).** In the SAME native dispatcher handling that
     already distinguishes PreToolUse (BC-1.18.005 Precondition 1) from PostToolUse for this tool
     call, once a `replace_all: true` call has been applied, this BC's gate performs a fresh
     `stat()` of the canonical file. If `actual_size <= shard_cap_bytes`, no action is taken (the
     single-occurrence estimate was conservative or exactly correct; `Continue`'s outcome stands
     unmodified). If `actual_size > shard_cap_bytes`, this BC executes Postcondition 1's EXACT
     four-step sequence (read the now-over-cap canonical content, publish it as a sealed shard,
     atomically truncate the canonical file to empty, publish the updated shard-index)
     RETROACTIVELY — i.e., against content that is ALREADY on disk, not content about to be
     written.

     **CORRECTED (cluster-2 LOCAL adversary pass-4, F-C2-P4-001, MAJOR) — catch point (i) MUST run
     the self-heal reconciliation pass (`run_self_heal_if_plausible`, the SAME pass the PreToolUse
     Flat arm already runs per BC-1.18.005 Precondition 1) BEFORE calling `execute_roll`, never
     after.** The WITHDRAWN claim that catch point (i) is "safe without a self-heal pre-pass, by
     construction" (the WITHDRAWN Invariant 10; see the corrected Invariant 10 below) is FALSE: it
     examined only catch point (i)'s over/under-cap DETERMINATION (which an unreconciled orphan
     indeed cannot corrupt), but never catch point (i)'s own SEAL-PUBLISH step, whose `next_seal_seq`
     is derived from the shard-index — and an UNRECONCILED index can be missing an orphan's entry
     entirely. A REACHABLE counterexample: a prior prospective roll crashes as `E-SHD-006`
     (`decision-log.0001.md` holds durable content X; canonical also still holds X, untruncated; the
     shard-index remains EMPTY, since `E-SHD-006` crashes strictly before step (d)). The next matched
     dispatch is a `replace_all: true` `Edit` whose already-applied edit changes the canonical's
     content from X to X′ (X′ > `shard_cap_bytes`, X′ ≠ X). Catch point (i) fires, `stat()`s the
     over-cap canonical, and — under the WITHDRAWN, self-heal-pre-pass-free design — calls
     `execute_roll` directly: `next_seal_seq` is derived ONLY from the (still-empty) index, yielding
     `seq=1`, and `publish_sealed_shard` OVERWRITES the durably-sealed `decision-log.0001.md` (X)
     with X′, permanently destroying the sealed history. Running self-heal FIRST indexes the
     `E-SHD-006` (and, symmetrically, any `E-SHD-007`) orphan before `next_seal_seq` is computed, so
     `next_seal_seq` correctly advances past it (`seq=2`), and the prior seal is preserved. This is
     now a REQUIRED symmetry with the PreToolUse Flat arm, not an intentional asymmetry to preserve.
     Catch point (ii) is UNAFFECTED by this correction — it already executes within the same
     PreToolUse handling path, after `run_self_heal_if_plausible` has already run per BC-1.18.005
     Precondition 1, so it already inherits this protection.

     The three per-step partial-failure codes (`E-SHD-001`/`E-SHD-006`/`E-SHD-007`) apply
     identically, self-healing exactly as Postcondition 1 already specifies. **NEW `E-SHD-009`
     (Postcondition 8) backstops this as defense-in-depth**, in case self-heal ever fails to run or
     fails to reconcile an orphan for any reason: `publish_sealed_shard` independently refuses to
     overwrite an existing destination regardless of whether self-heal ran, converting any residual
     seq-collision into a loud, fail-safe `HookResult::Error` rather than a silent overwrite. See
     EC-023 and its matching Canonical Test Vector.
   - **(ii) Next-dispatch backstop (defense-in-depth, covers a crash between apply and (i)).**
     Before evaluating BC-1.18.005's own trigger for ANY subsequent `Edit`, `Write`, or `MultiEdit`
     against a matched artifact, this BC's gate first checks whether the CURRENT on-disk size
     already exceeds `shard_cap_bytes` — a state that is structurally impossible to persist under
     the normal pre-write block-and-retry flow, and should already have been resolved by (i), but
     can transiently survive a dispatcher crash between the `replace_all` write's completion and
     (i)'s own execution. If so, this BC executes the SAME retroactive four-step sequence BEFORE
     applying the new call or evaluating its own trigger.

     **CORRECTED (cluster-2 LOCAL adversary pass-1, F-C2-P1-002, MAJOR) — the backstop's probe cost
     is per-tool, not uniformly free; a `Write` requires a DEDICATED stat, not a reused one.** The
     prior text above claimed the backstop reuses "the same `stat()` BC-1.18.005 Postcondition 2
     already performs" for EVERY subsequent tool call. This is TRUE for `Edit`/`MultiEdit` — whose
     Postcondition 3 formula, `projected_size = current_shard_bytes + net_delta_bytes`, already
     requires `stat()`-reading `current_shard_bytes`; the backstop reuses that already-read value,
     adding zero new `stat()` calls. It was WRONG for `Write`: `Write`'s Postcondition 3 formula is
     `projected_size = len(content)` alone, with NO `stat()` of the canonical file anywhere in
     BC-1.18.005's own `Write` trigger path — there is no existing stat-read for this backstop to
     reuse. Left uncorrected, this was a real DATA-LOSS gap, not merely a wording inaccuracy: after a
     dispatcher crash leaves the canonical file over-cap and un-sealed (EC-015), a subsequent `Write`
     whose own `content` is under-cap would apply directly against the canonical file — REPLACING,
     and permanently DESTROYING, the un-sealed over-cap history — with no roll ever having started
     for self-heal to catch (self-heal reconciles orphaned ROLLS; it has nothing to reconcile if no
     roll was ever attempted). Under CLAUDE.md's production-grade default (no version of this gate
     may lose history), this backstop therefore performs its OWN dedicated, bounded `stat()` of the
     canonical file BEFORE applying a `Write` — an ADDITIONAL probe specific to the `Write` arm, NOT
     a change to BC-1.18.005's stat-free `Write` trigger formula (which is UNCHANGED: this dedicated
     stat feeds ONLY this backstop's over-cap check, never `projected_size`). For `Edit`/`MultiEdit`,
     the backstop remains cost-free (reuses the existing stat), unchanged from the original design
     intent. Net cost: one bounded `stat()` call added per `Write` against a matched artifact
     (independent of artifact size, since `stat()` never reads file content); zero added cost for
     `Edit`/`MultiEdit`.

     **Backstop probe `stat()`-failure disposition — fail LOUD on any non-`NotFound` error, never
     fail-open (cluster-2 LOCAL adversary pass-2, F-C2-P2-003, MINOR).** This dedicated `Write`-arm
     probe has exactly two non-success outcomes: (1) `NotFound` — the canonical file does not yet
     exist (the artifact's first-ever write, BC-1.18.005 EC-004's zero-byte-shard case); this is NOT
     a probe failure, and the gate proceeds to apply the `Write` normally, exactly as if this
     backstop did not exist. (2) any OTHER `stat()` error (`ELOOP`, `EACCES`, `EIO`, or any other
     failure) — this BC REQUIRES the gate to fail LOUD, returning `HookResult::Error` (NEW
     `E-SHD-008`, "backstop probe stat failure" — **mechanism label, not literal `Display` text;
     see `prd-supplements/error-taxonomy.md`'s `E-SHD-008` row for the exact emitted message,
     RESOLVED cluster-2 LOCAL adversary pass-10, F-C2-P10-004**) WITHOUT applying the `Write`, identically to how
     `Edit`/`MultiEdit`'s own pre-existing stat-failure disposition already behaves. **This WITHDRAWS
     a fail-open disposition** (log a warning, let the `Write` proceed) that an earlier
     implementation adopted by analogy to cluster-1's already-adjudicated rule that "a `Write` must
     never be blocked by a stat failure irrelevant to its own formula" (the `ELOOP` fixture test) —
     that rule correctly governs BC-1.18.005's OWN stat-free `Write` trigger formula (left unchanged
     by this BC's Precondition 4), but does NOT extend to THIS BC's own dedicated backstop probe,
     whose entire purpose is to detect a crash-orphaned over-cap canonical file before a `Write` can
     destroy it (F-C2-P1-002). Fail-open is UNSOUND here, not merely under-specified: a canonical
     path whose FINAL path component is a symlink loop confined to itself fails a dereferencing
     `stat()` (`ELOOP`) while `write_atomic`'s `rename(temp, canonical)` — which does not need to
     dereference the destination's final symlink component — can still succeed, silently replacing
     the symlink and applying the `Write`'s under-cap `content`, with the true, possibly-over-cap
     content the symlink pointed at never examined, never sealed, and now unreferenced by the
     canonical name: a real, specific data-loss path, not a hypothetical one. (An `EACCES` arising
     from directory-search-permission denial IS symmetric — both `stat()` and `write_atomic`'s
     temp-file-create/rename traverse the same path prefix and fail together — but this BC does not
     rely on errno-by-errno reasoning to decide safety, because one confirmed asymmetric case is
     sufficient to make blanket fail-open unsound, and enumerating every future filesystem/errno
     combination as safe is not a claim this BC is willing to make.) Fail-loud costs nothing in the
     common case (the probe almost always succeeds) and, in the rare failure case, produces an
     explicit, actionable `HookResult::Error` rather than a silent, unverified pass-through of a
     `Write` whose safety could not be confirmed — consistent with Invariant 1's "no version of this
     check may return `Continue`" principle extended to this probe's own failure mode. See Invariant
     8 for the general form of this uniform-disposition rule, and EC-019/`E-SHD-008` below.

   **Bounded-window postcondition (testable, not "known limitation" prose).** For a `replace_all:
   true` call whose true occurrence-multiplied delta was under-projected by BC-1.18.005's
   single-occurrence trigger estimate, the canonical file MAY be observed in an over-cap state ONLY
   during the window between that call's completed write and the EARLIER of: (i) firing for the
   SAME tool invocation, or (ii) the artifact's NEXT matched `Edit`/`Write`/`MultiEdit` dispatch
   (whichever occurs first) — never indefinitely, never spanning more than one subsequent matched
   dispatch. This is the SOLE, explicitly bounded exception to Postcondition 3's "no shard is ever
   observed in an over-cap state by any downstream reader" structural guarantee, and it applies
   ONLY to this narrow `replace_all: true` case — every other call class this BC governs retains
   Postcondition 3's unconditional guarantee.

   **Sealed-shard cap exception (documented, scoped only to retroactive rolls).** Because a
   retroactive roll seals content that has ALREADY been written (its size could not be prevented
   pre-write), the resulting sealed shard's `bytes_at_seal` MAY exceed `shard_cap_bytes` — a
   documented, narrow exception to Postcondition 3's per-shard cap guarantee, scoped ONLY to shards
   produced by this Postcondition (marked `sealed_retroactively: true`, Postcondition 5). The
   CANONICAL file's own guarantee is UNAFFECTED and holds unconditionally once reconciliation
   completes: step (c)'s atomic truncate-to-empty never accepts an exception, so the canonical file
   is always exactly 0 bytes immediately after either catch point (i) or (ii) fires — see
   Invariant 6.

8. **NEW (cluster-2 LOCAL adversary pass-4, F-C2-P4-001/002, MAJOR/MINOR) — `publish_sealed_shard`
   is WRITE-ONCE for NON-EMPTY destinations: it MUST refuse to overwrite an existing, non-empty
   destination, and a collision is a loud error, never a silent data-loss overwrite.** A sealed
   shard, once published, is immutable content — Postcondition 3's guarantee that a prospective
   seal's `bytes_at_seal <= shard_cap_bytes` (or, for a retroactive seal, Postcondition 7's
   documented exception) and Invariant 3's "canonical filename never moves" guarantee both assume
   the sealed file at a given `seq` never changes after publication. Before every seal-publish
   attempt this BC's gate makes (Postcondition 1 step (b), and both of Postcondition 7's catch
   points (i)/(ii) when they execute the same step retroactively), `publish_sealed_shard` attempts
   `write_exclusive`'s atomic exclusive-create (never a `write_atomic` rename — CORRECTED, cluster-2
   LOCAL adversary pass-8, F-C2-P8-001, MEDIUM; see the Description's corrected note and this
   Postcondition's own mechanism text). If the destination already exists AND is non-empty, the
   gate MUST NOT overwrite it — it returns `HookResult::Error` (NEW `E-SHD-009`, "refusing to
   overwrite an already-sealed shard at '`<path>`' for artifact_stem \"`<artifact_stem>`\" — sealed
   shards are write-once/immutable; this seq already has durable content on disk") and applies
   NEITHER the seal NOR any of the roll's other steps (no truncate, no index publish) for this
   attempt; the pre-existing destination file is left completely untouched.

   **0-byte-destination exception (NEW, cluster-2 LOCAL adversary pass-8, F-C2-P8-002, MEDIUM) —
   resolving the Invariant 9 interaction.** A 0-byte file at the destination `seq` path is, by
   Invariant 9's own reasoning, structurally NEVER a genuine orphan of this BC's own write paths —
   `execute_roll`'s prospective and retroactive rolls alike only ever seal non-empty content
   (Postcondition 1's empty-canonical short-circuit and Postcondition 3's cap guarantee together
   ensure this), so a 0-byte file at a sealed-shard path can only be an external anomaly (a human
   `touch`, a botched manual recovery, a corrupted transfer) with NO durable sealed history to
   protect. Left un-addressed, this interacts with Invariant 9's self-heal skip-and-warn guard to
   produce a PERMANENT DEADLOCK: self-heal never indexes the 0-byte file (Invariant 9), so
   `next_seal_seq` (computed as `max(indexed seq) + 1`) never advances past it, so every future
   roll attempt for this artifact recomputes the SAME colliding `seq` and collides with the SAME
   0-byte file forever. **REQUIRED resolution — the write-once refusal above applies ONLY to a
   NON-EMPTY pre-existing destination; a 0-byte pre-existing destination is safe to replace, and
   `publish_sealed_shard` MUST reclaim it rather than refuse:** if `write_exclusive`'s exclusive-
   create fails with `AlreadyExists`, `publish_sealed_shard` `stat()`s the pre-existing destination
   exactly once; if its size is exactly 0 bytes, it unlinks the 0-byte file and retries
   `write_exclusive` exactly ONCE (never in a loop — bounded to a single reclaim attempt, so a
   racing concurrent writer cannot induce unbounded retries). If the pre-existing destination's
   size was non-zero to begin with, the gate fails loud with `E-SHD-009` exactly as specified
   above, without ever unlinking anything — the 0-byte exception never weakens the non-empty case's
   write-once guarantee.

   **CORRECTED (cluster-2 LOCAL adversary pass-10, F-C2-P10-002, ADVISORY, human-authorized
   CONVERGE-TO-PR path) — the reclaim's `stat()` → `unlink()` → retry sequence has TWO distinct
   concurrent-writer sub-windows with DIFFERENT outcomes; the withdrawn text above overclaimed that
   BOTH fail loud.** The reclaim is three sequential steps — `stat()` observes 0 bytes, then
   `unlink()` removes the 0-byte file, then the retry calls `write_exclusive()` — and a concurrent
   writer landing real content in either of the two resulting sub-windows produces a structurally
   DIFFERENT result:
   - **`unlink()`-to-retry sub-window (content lands AFTER this gate's own `unlink()` completes,
     before the retry executes):** the retry's `write_exclusive()` finds the destination occupied
     again and fails `AlreadyExists` a SECOND time; the gate correctly fails loud with `E-SHD-009`,
     and the concurrent writer's content is left byte-identical, untouched. This is the retry-
     collision arm, and the ONLY sub-window for which "fails loud" is an accurate description.
   - **`stat()`-to-`unlink()` sub-window (content lands AFTER this gate's own `stat()` observed 0
     bytes, but BEFORE this gate's own `unlink()` executes):** `unlink()` does not re-verify size
     before removing — it unconditionally deletes whatever currently occupies the path, INCLUDING
     the concurrent writer's just-landed real content. The retry's `write_exclusive()` then finds
     the destination vacant and SUCCEEDS: no `E-SHD-009` is ever raised, the reclaim silently
     completes as if the destination had genuinely been an orphaned 0-byte file, and the concurrent
     writer's content is permanently, silently lost with no error signal to any party. This
     sub-window is a genuine, residual, UNCLOSED race — not merely a wording inaccuracy in the
     withdrawn text above.

   This `stat()`-to-`unlink()` sub-window is explicitly ACCEPTED, not fixed in this burst, on the
   human-authorized converge-to-PR path (CLAUDE.md Rule 3): under the current per-dispatch
   single-process execution model (no two dispatches for the same artifact execute their
   filesystem operations concurrently within the same process), triggering it requires a genuinely
   concurrent OUT-OF-PROCESS actor racing the microsecond-scale window between two syscalls issued
   back-to-back by the same gate invocation — near-impossible in practice, which is the
   accepted-residual rationale. **Full hardening of this sub-window is OWED to Phase F6 targeted-
   hardening (architect/formal-verifier), attached to the OWED §4.4 F6-owed list**, mirroring this
   BC's own established VP-owed-to-F6 precedent (Postcondition 7's VP-NNN pending row): the concrete
   future dependency is an `O_EXCL`-based re-create-then-swap reclaim primitive (atomically
   re-creating the destination via `O_CREAT | O_EXCL` and swapping only if the observed pre-image is
   still exactly 0 bytes at swap time, which structurally cannot delete non-empty content regardless
   of interleaving) PLUS a dedicated concurrent-race integration test (fault-inject a writer landing
   real content strictly between this gate's own `stat()` and `unlink()` calls, asserting the
   writer's content survives and is never silently destroyed) — both deferred to Phase F6, not
   enacted in this burst. Until that hardening lands, this sub-window's risk is accepted as
   documented, bounded, and near-impossible under the current execution model — never silently
   unacknowledged. A successful 0-byte reclaim (either sub-window's non-colliding case) emits a
   `tracing::warn!` diagnostic naming the reclaimed path (mirroring Invariant 9's own self-heal
   diagnostic convention) but does NOT block or fail the current dispatch. See EC-025 and its
   matching Canonical Test Vectors (including the corrected race-variant CTV); EC-024 is
   correspondingly scoped to the non-empty case.

   **Why this is defense-in-depth, not the primary fix:** the primary, ROOT-CAUSE fix is
   Postcondition 7 catch point (i)'s corrected self-heal-first sequencing (above), which prevents
   `next_seal_seq` from ever being computed against an unreconciled index in the first place —
   under that fix, a well-formed dispatch should never attempt to publish at a colliding `seq`.
   This write-once guard is the SECOND, independent layer: it converts any residual seq-collision —
   whether from a self-heal reconciliation bug, an external actor placing a same-named file on disk,
   or a future code path this BC has not yet anticipated — into a loud, actionable, fail-safe error
   rather than a silent overwrite of durably-sealed history, EXCEPT for the narrow, deliberately-
   carved-out 0-byte case above, which has no durably-sealed history to protect in the first place.
   This is consistent with Invariant 1's "no version of this check may silently proceed past a
   condition it cannot verify safe" principle, extended to the seal-publish step itself. **CODE
   CHANGE ROUTED (→ implementer):** `publish_sealed_shard` gains the existence-check-before-rename
   guard described above (already implemented as `write_exclusive`'s exclusive-create), PLUS the
   bounded, one-retry 0-byte-reclaim path (`stat()` the collision once; if 0 bytes, unlink and retry
   `write_exclusive` exactly once; else fail loud). **TEST ROUTED (→ test-writer):** (i) a fixture
   that pre-creates a NON-EMPTY file at the destination `seq` path before a roll (prospective or
   retroactive) attempts to publish at that same `seq`, asserting `HookResult::Error`/`E-SHD-009` is
   returned, the pre-existing file's content is BYTE-IDENTICAL after the attempt (never overwritten),
   and no truncate or index-publish occurs (EC-024); (ii) a fixture that pre-creates a 0-BYTE file at
   the destination `seq` path, asserting the roll SUCCEEDS (the 0-byte file is reclaimed, the new
   seal content is durably published at that exact path, no `E-SHD-009`), and a fixture where a
   concurrent writer places real content at the path between the probe and the retry, asserting the
   race case still fails loud with `E-SHD-009` (EC-025).

## Invariants

1. **`HookResult::Block` is the ONLY variant this BC's gate returns on an over-cap write.** No
   version of this check may return `Continue` after determining `projected_size >
   shard_cap_bytes` (that would silently permit an over-cap write, violating BC-1.18.005
   Postcondition 3's contract), and no version may return `Error` for a normal (non-crash)
   over-cap condition (a normal rotation is not an error condition — it is the mechanism working
   as designed).

2. **CORRECTED (fix-burst pass-2, F-P2-003, HIGH) — the read-publish-truncate-publish sequence is
   never reordered.** Publishing the shard index before the sealed shard is published, or
   truncating the canonical file's content before the sealed shard's content is durably published,
   would risk a window where NEITHER a valid sealed copy NOR a valid canonical-with-full-content
   state exists for the pre-roll content. The four sub-steps in Postcondition 1 ((a) read, (b)
   publish sealed shard, (c) atomic-truncate canonical, (d) publish index) execute in the stated
   order; each of (b), (c), and (d) is its own independent filesystem write — step (b) via
   `publish_sealed_shard`'s exclusive-create primitive (`write_exclusive`, a `std::fs::hard_link`-
   based no-clobber create — CORRECTED, cluster-2 LOCAL adversary pass-8, F-C2-P8-001, MEDIUM: NOT
   `write_atomic`, since a plain rename cannot express step (b)'s write-once refuse-if-exists
   contract), and steps (c) and (d) each via `write_atomic`'s temp-file-then-rename primitive —
   there is no OS-level atomicity spanning multiple steps, which is exactly why
   Postcondition 1's per-step partial-failure codes (`E-SHD-001`/`E-SHD-006`/`E-SHD-007`) exist:
   a crash between any two steps is a distinct, named, self-healing-recoverable state, never an
   unspecified one.

3. **The canonical filename never moves.** At every observable point in time — before a roll,
   during a roll's execution, and after a roll completes — the canonical filename (e.g.
   `decision-log.md`) refers to SOME valid file: either the not-yet-full current shard (before a
   roll), or the fresh empty current shard (after a roll completes). It is NEVER renamed away to a
   sealed name; only its CONTENT is replaced, via an atomic `write_atomic` rename-ONTO-existing-
   destination (Postcondition 1 step (c)) — never a rename-OUT-of the canonical path. This
   invariant is now structurally, not merely conventionally, true: the withdrawn v1.0/v1.1
   rename-away mechanism could not satisfy it (see Description's ENOENT-window analysis); the
   corrected copy-then-atomic-truncate mechanism can, because `rename()` onto an EXISTING
   destination never leaves that destination absent.

4. **Retry-instruction wording is a single, fixed template PER OVER-CAP CASE — never divergent per
   original tool name, but deliberately divergent between the over-cap-WITH-content case and the
   over-cap-WITHOUT-content (empty-canonical) case.** CORRECTED (fix-burst pass-2, F-P2-002): the
   withdrawn v1.0/v1.1 design chose between two DIFFERENT wordings based on the original blocked
   tool's name (a `Write`-specific "simply retry unchanged" branch that was later found unsound).
   The corrected design (Postcondition 2) uses ONE unified message template for the over-cap-with-
   content case that names both tool cases within the SAME text — that template itself never
   varies by tool, and its content is never randomized, never omitted, and never generic ("write
   failed, try again" without the specific per-tool guidance embedded in the unified template is
   insufficient).

   **ADJUDICATED (cluster-2 LOCAL adversary pass-3, F-C2-P3-001, MAJOR) — this invariant is scoped
   to "one template per case," and the empty-canonical short-circuit (Postcondition 1) is a SECOND,
   DISTINCT, sanctioned case, not a violation of "single fixed template."** Two top-level cases
   exist, each tool-name-invariant WITHIN itself:
   - **Case A — over-cap-with-content** (Postcondition 1's four-step roll fires): the unified
     rotate-and-retry template (Postcondition 2's main text), identical regardless of whether the
     blocked call was `Edit`, `Write`, or `MultiEdit`.
   - **Case B — over-cap-without-content** (Postcondition 1's `Ok(None)` empty-canonical
     short-circuit fires): the `build_empty_roll_retry_block_reason` template family
     (Postcondition 2's "Empty-canonical retry template" clause), likewise never tool-name-
     dependent — but see the Case B1/B2 split below (NEW, cluster-2 LOCAL adversary pass-8,
     F-C2-P8-004, MINOR).
   This is DELIBERATE, ADOPTED (Option (a) of the two options the adversary posed, per this BC's
   own recommendation): the two cases describe structurally different facts to the agent (history
   was rotated away vs. no history existed to rotate), and unifying them into one wording would
   either omit Case B's split-payload guidance or falsely claim a rotation occurred in Case B.

   **Case B splits into two sanctioned sub-templates, B1 and B2 (NEW, cluster-2 LOCAL adversary
   pass-8, F-C2-P8-004, MINOR) — THREE templates total exist across this BC, not two.** Case B's
   own selection is a pure function of whether Postcondition 7 catch point (ii) executed a
   retroactive roll earlier in the SAME dispatch:
   - **Case B1 — pure empty-canonical** (no roll of any kind occurred during this dispatch; the
     canonical was already 0 bytes when the dispatch began): the "no roll was performed... the
     shard remains exactly as it was before this call" wording (Postcondition 2's Empty-canonical
     retry template main text).
   - **Case B2 — double-fire empty-canonical** (catch point (ii)'s backstop retroactively rolled a
     PRE-EXISTING orphaned over-cap canonical earlier in THIS SAME dispatch, and this call's own
     payload then also exceeds the cap against the now-freshly-emptied canonical): the distinct
     wording that drops the "no roll was performed"/"remains exactly as it was before this call"
     clauses (which would be FALSE on this path — a roll did occur), while keeping the identical
     split-payload guidance (Postcondition 2's "Double-fire exception" clause).
   Selection between B1 and B2 is a pure function of dispatch-local state (whether catch point (ii)
   fired earlier in this same dispatch) — never randomized, never combined, and never conflated
   with Case A. Three, and only three, templates exist (A, B1, B2) — every over-cap `Block` this BC
   returns selects exactly one of them. This invariant is VIOLATED only if (i) any template varies
   by tool name within its own case, (ii) a fourth distinct wording appears, or (iii) any
   template's selection logic depends on anything other than which Postcondition 1 code path
   executed (for A vs. B) or whether catch point (ii) fired earlier in the same dispatch (for B1
   vs. B2).

5. **NEW (fix-burst pass-2, F-P2-005, MEDIUM) — the append-only-tail assumption is explicit,
   never silently relied upon.** This BC's gate has NO semantic understanding of WHERE within a
   file an `Edit`/`Write`/`MultiEdit` lands — it computes `projected_size` from a pure byte-delta/
   length formula (BC-1.18.005 Postcondition 3) only. The roll+block+retry wording (Postcondition
   2) is phrased for the common case this gate exists to serve: a pure APPEND of one new record at
   the file's end. The four mechanism-A artifacts are, by construction, POLICY-1
   (`append_only_numbering`) governed append-only records — POLICY-1 already forbids renumbering
   or rewriting historical entries, so legitimate `Edit`/`MultiEdit` mutations against these
   artifacts are, by that SAME policy, already expected to be either (a) a pure append of a
   brand-new record at file end, or (b) a narrow amendment to a STILL-MUTABLE, recently-added
   record near the tail — never an edit to arbitrarily old, already-sealed, or deep-mid-file
   historical content. See EC-012 below for the caller-responsibility failure mode when this
   assumption is violated, and its sanctioned escape hatch.

6. **NEW (F2 spec-evolution, replace_all closure) — the canonical file's zero-bytes-after-roll
   guarantee holds unconditionally, even under Postcondition 7's retroactive path; only the SEALED
   shard's per-shard cap guarantee is exceptionally relaxed, and only when `sealed_retroactively:
   true`.** Postcondition 1 step (c) (atomic truncate-to-empty) is IDENTICAL code whether invoked
   from the normal pre-write block-and-retry sequence or from Postcondition 7's retroactive
   reconciliation (catch point (i) or (ii)) — there is no code path in which the canonical file is
   left non-empty after either catch point completes successfully. Invariant 3's "canonical filename
   never moves" guarantee is likewise unaffected: Postcondition 7 reuses the exact same
   rename-ONTO-existing-destination truncate, never a rename-away.

7. **NEW (cluster-2 LOCAL adversary pass-1, F-C2-P1-001, MAJOR) — `sealed_retroactively` is
   deterministically inferrable from `bytes_at_seal`, and self-heal reconciliation MUST use that
   inference, never a hardcoded default.** For any `[[shard]]` entry, `bytes_at_seal >
   shard_cap_bytes` (the value recorded in that SAME entry) if and only if the seal was produced by
   Postcondition 7's retroactive path — a PROSPECTIVE roll (Postcondition 1) can never seal
   over-cap content (Postcondition 3's unconditional per-shard cap guarantee), so `bytes_at_seal >
   shard_cap_bytes` occurring at all is proof-by-construction of retroactivity; conversely, a
   RETROACTIVE roll seals content that has already been written and whose size could not be
   prevented pre-write, so `bytes_at_seal <= shard_cap_bytes` from a retroactive roll is possible
   (the under-projection may have been small) but never contradicts the inference in the direction
   that matters — the inference is used only in the `>` case, which is unambiguous. Postcondition
   1's `E-SHD-007` self-heal reconciliation (which appends an index entry for a sealed shard found
   on disk but missing from the index, with no other context available about which roll produced
   it) MUST apply this inference — `sealed_retroactively = (bytes_at_seal > shard_cap_bytes)` —
   rather than defaulting the recovered entry's `sealed_retroactively` to `false` unconditionally.
   This closes the mislabeling gap that would otherwise arise when a retroactive roll's own step
   (d) fails (`E-SHD-007`) before the roll's own retroactive-context could be durably recorded.

   **STABLE-CAP PRECONDITION (cluster-2 LOCAL adversary pass-2, F-C2-P2-004, MINOR) — the inference
   above is exact ONLY under a stable `shard_cap_bytes`; a re-calibration spanning a shard's
   seal-to-reconciliation window bounds, but does not eliminate, a documented mislabel risk in BOTH
   directions.** BC-1.18.005 Postcondition 6/AC-004 permits the F4 calibration harness to re-lock
   `shard_cap_bytes` to a new value when `DEFAULT_FUEL_CAP` changes (the shard-index schema's own
   comment, Postcondition 5, already documents the value as "locked at F4" — a rare, deliberate,
   phase-gated administrative event, never a continuously runtime-mutable parameter). If an
   orphaned, un-indexed shard produced by Postcondition 1's `E-SHD-007` crash point (its own step
   (c) succeeded, step (d) did not) SURVIVES across an F4 cap re-calibration boundary before its own
   reconciliation runs, the inference compares `bytes_at_seal` (fixed under the OLD cap regime)
   against the NEW, live `shard_cap_bytes` — producing two possible mislabels, bounded to this
   single narrow window:
   - **Cap LOWERED** (`old_cap >= bytes_at_seal > new_cap`): a legitimately PROSPECTIVE seal
     (compliant with the cap in force at its own seal time) is inferred `sealed_retroactively =
     true` — a FALSE POSITIVE. This is the direction the adversary's finding text identified.
   - **Cap RAISED** (`old_cap < bytes_at_seal <= new_cap`): a genuinely RETROACTIVE seal (it exceeded
     the cap in force at ITS OWN trigger time, which is what caused catch point (i)/(ii) to fire in
     the first place) is inferred `sealed_retroactively = false`/omitted — a FALSE NEGATIVE. This
     direction was NOT named by the adversary's finding text; it is added here under CLAUDE.md's
     "fix in scope when found" default rather than left for a future pass to independently
     rediscover.

   Both mislabels are ACCEPTED, DOCUMENTED, and judged operationally negligible under the
   production-grade lens, for three independent reasons: (1) **scope** — the field mislabeled is
   `sealed_retroactively`, a historical AUDIT-TRAIL annotation only; it gates no hard structural
   guarantee this BC specifies — Invariant 6's unconditional canonical zero-bytes-after-roll
   guarantee and the sealed-shard content-preservation guarantee are computed independently of this
   flag and are unaffected in either mislabel direction. (2) **rarity** — the mislabel requires the
   CONJUNCTION of an already-rare event (a crash precisely between a roll's own step (c) and step
   (d), `E-SHD-007`) surviving specifically across an already-rare, deliberately-gated F4
   recalibration boundary, before that SAME orphaned entry's own reconciliation runs — two
   independently rare events must coincide, not a routine occurrence. (3) **testable bound** — a
   reconciled entry can be mislabeled by this mechanism if and only if `shard_cap_bytes` differs
   between the shard's own `sealed_at` timestamp and the reconciliation event; `sealed_retroactively`
   is GUARANTEED correct whenever `shard_cap_bytes` was unchanged throughout
   `[sealed_at, reconciliation_time]`. A structurally complete fix (persisting `shard_cap_bytes`-at-
   seal-time in the sealed shard itself, or gating F4 recalibration on first draining all pending
   `E-SHD-007` orphans) would require amending BC-1.18.005's already-ACTIVE calibration-harness
   contract or extending the shard-index schema beyond this BC's own scope — out of THIS BC's
   modification boundary per its own established Precondition-4/Postcondition-7 precedent of not
   amending BC-1.18.005's shipped contract within this BC's own closure bursts — and is flagged as a
   candidate FUTURE BC-1.18.005 spec-evolution item, NOT enacted here. This is a documented, bounded,
   accepted design tradeoff, not a deferred defect: no further code change follows from this
   correction; the current inference implementation is already correct under the now-explicit
   stable-cap precondition.

8. **NEW (cluster-2 LOCAL adversary pass-2, F-C2-P2-003, MINOR) — the Postcondition 7 catch point
   (ii) backstop probe's `stat()`-failure disposition is uniform fail-loud, never tool-divergent.**
   No version of this backstop probe may fail OPEN (silently permit the `Write` to proceed) on a
   `stat()` error other than `NotFound`, for the same reason Invariant 4 forbids tool-divergent
   retry wording: an unadjudicated per-tool asymmetry between `Edit`/`MultiEdit` (fail-loud, by
   virtue of reusing BC-1.18.005's own already-fail-loud trigger stat) and `Write` (this BC's own
   dedicated probe) is itself a defect class, independent of whether any single errno happens to be
   safe to ignore. `NotFound` is the sole exception — it signals "no canonical file yet exists,"
   the artifact's legitimate first-ever-write case (BC-1.18.005 EC-004), not a probe failure — and
   is unaffected by this invariant.

9. **NEW (cluster-2 LOCAL adversary pass-3, F-C2-P3-002, MINOR) — no `[[shard]]` index entry may
   ever have `bytes_at_seal = 0`, and this binds ALL paths that append `[[shard]]` entries, not
   only `execute_roll`.** ADJUDICATED: this IS a real BC invariant, not merely a property of the
   normal write path. `execute_roll` already satisfies it structurally for its own two entry points
   — the four-step roll (Postcondition 1) can only run when the canonical file is non-empty
   (Postcondition 1's `Ok(None)` short-circuit, F-C2-P3-001, guarantees this), and Postcondition 3's
   unconditional per-shard cap guarantee bounds `bytes_at_seal` above `shard_cap_bytes` but never
   requires it to be zero-or-more without also being non-zero-content — a prospective seal always
   copies REAL, non-empty pre-roll content. The gap the adversary identified is in the SELF-HEAL
   index paths (`self_heal_reconcile_missing_index_entries`, the `E-SHD-007` recovery; and
   `self_heal_resume_from_truncate`, the `E-SHD-006` recovery), which append `[[shard]]` entries by
   inspecting whatever sealed-shard-shaped file (`<stem>.<seq:04>.md`) is ALREADY on disk, with no
   equivalent non-empty guard. **REQUIRED:** both self-heal paths MUST refuse to append a
   `[[shard]]` entry for a candidate sealed-shard file whose on-disk size is exactly 0 bytes; they
   MUST instead SKIP that candidate (append no entry for it, and continue reconciling any other
   genuinely-orphaned entries in the same pass) and emit a diagnostic (a `tracing::warn!` per this
   project's structured-logging convention — not a `HookResult::Error`, since a stale externally-
   created 0-byte orphan is unrelated to the CURRENT dispatch's own write and must not block it).
   **Why this is reachable only via external anomaly, and why that does not make it exemptable:** a
   0-byte file matching the sealed-shard naming convention cannot be produced by ANY path this BC's
   own write mechanisms control — `execute_roll`'s prospective and retroactive rolls alike only ever
   seal non-empty content (per F-C2-P3-001's short-circuit and Postcondition 3's cap guarantee), and
   `write_atomic`/`write_indeterminate_marker` never create a zero-byte destination as a side effect
   of any operation this BC specifies. Such a file can only arise from an external actor (a human
   `touch`, a botched manual recovery, a corrupted transfer) creating a same-shaped file directly on
   disk — outside this BC's write-path guarantees, exactly as EC-012's sealed-shard direct-edit
   escape hatch already establishes sealed filenames as ordinary, ungated files once created.
   Low reachability does not exempt this from being specified: self-heal's job is to reconcile
   genuine orphans of THIS BC's own crash-recovery paths, and per the structural argument above,
   every genuine orphan it will ever encounter is guaranteed non-empty — so a 0-byte candidate
   self-heal encounters is BY DEFINITION not one of the orphans self-heal exists to recover, and
   indexing it as though it were would silently fabricate an audit-trail entry (`bytes_at_seal = 0`)
   for content that documents no actual sealed history, contradicting Postcondition 5's own
   description of `bytes_at_seal` as "the sealed shard's exact final byte count." **CODE CHANGE
   ROUTED (→ implementer):** both self-heal functions require the added 0-byte guard described
   above. **TEST ROUTED (→ test-writer):** a fixture placing a 0-byte file at a self-heal-eligible
   `<stem>.<seq:04>.md` path, asserting the reconciliation pass skips it (no `[[shard]]` entry
   appended for it) and does not fail the current dispatch. See EC-022 and its matching Canonical
   Test Vector.

   **Interaction with Postcondition 8's write-once guard, resolved (NEW, cluster-2 LOCAL adversary
   pass-8, F-C2-P8-002, MEDIUM).** This invariant's "self-heal never indexes a 0-byte candidate"
   rule, combined UNMODIFIED with Postcondition 8's original write-once refusal, would deadlock: if
   a 0-byte candidate sits at the artifact's `next_seal_seq` path, self-heal's refusal to index it
   means `next_seal_seq` never advances past it, so `publish_sealed_shard` would collide with that
   SAME 0-byte file on every future roll attempt, forever. Postcondition 8's 0-byte-destination
   exception resolves this: `publish_sealed_shard` itself reclaims (unlinks and overwrites) a
   0-byte pre-existing destination rather than refusing it, since a 0-byte file has no durable
   sealed history to protect — the SAME structural guarantee this invariant already relies on (a
   genuine orphan of this BC's own write paths is never 0 bytes). This invariant's own
   skip-and-warn behavior for self-heal's INDEX-reconciliation paths is UNCHANGED by that
   resolution — self-heal still never appends a `[[shard]]` entry for a 0-byte candidate; the fix
   lives entirely in `publish_sealed_shard`'s write path (Postcondition 8), not in either self-heal
   function. See Postcondition 8's 0-byte-destination exception and EC-025.

10. **CORRECTED (cluster-2 LOCAL adversary pass-4, F-C2-P4-001, MAJOR) — Postcondition 7 catch
    point (i) is safe ONLY BECAUSE it runs a self-heal reconciliation pass BEFORE calling
    `execute_roll`, and this pre-pass MUST NOT be removed.** The WITHDRAWN claim (originally
    cluster-2 LOCAL adversary pass-3 observation O-C2-P3-001, ADVISORY) asserted catch point (i)
    was safe to run WITHOUT a self-heal pre-pass, "by construction," on the theory that neither an
    `E-SHD-006` nor an `E-SHD-007` orphan could corrupt catch point (i)'s own `stat()`-based
    over/under-cap determination. That theory is TRUE as far as it goes — an unreconciled orphan
    does not change what catch point (i)'s own `stat()` observes — but it is INCOMPLETE: it
    examined only catch point (i)'s over/under-cap DETERMINATION, never catch point (i)'s own
    SEAL-PUBLISH step, which depends on `next_seal_seq` being computed from a RECONCILED index.

    **A REACHABLE counterexample (cluster-2 LOCAL adversary pass-4, F-C2-P4-001):** a prospective
    roll crashes as `E-SHD-006` (`decision-log.0001.md` holds durable content X; canonical also
    still holds X, untruncated; the shard-index remains EMPTY, since `E-SHD-006` crashes strictly
    before step (d)). The next matched dispatch is a `replace_all: true` `Edit` whose already-applied
    edit changes the canonical's content from X to X′ (X′ > `shard_cap_bytes`, X′ ≠ X). Catch point
    (i) fires, `stat()`s the over-cap canonical, and — under the WITHDRAWN, self-heal-pre-pass-free
    design — calls `execute_roll` directly: `next_seal_seq` is derived ONLY from the (still-empty)
    index, yielding `seq=1`, and `publish_sealed_shard` OVERWRITES the durably-sealed
    `decision-log.0001.md` (X) with X′, permanently destroying the sealed history. The original
    theory's premises — "it re-seals the SAME bytes into a NEW `seq+1` file" — are both false in
    this counterexample: the bytes are NOT the same (the intervening `replace_all` changed them),
    and the seal targets the SAME `seq`, not `seq+1`, because the unreconciled index has no record
    of the orphan occupying `seq=1`.

    **CORRECTED requirement (see Postcondition 7 catch point (i), Postcondition 1's `E-SHD-006`
    recovery text, and Postcondition 8):** catch point (i) MUST run the self-heal reconciliation
    pass (`run_self_heal_if_plausible`) BEFORE calling `execute_roll`, matching the PreToolUse Flat
    arm (BC-1.18.005 Precondition 1) exactly — this is now a REQUIRED symmetry, not an intentional
    asymmetry to be preserved. Running self-heal first indexes any pre-existing `E-SHD-006`/
    `E-SHD-007` orphan, so `next_seal_seq` correctly advances past it, and the prior seal is
    preserved rather than overwritten. Catch point (ii) is UNAFFECTED — it already runs within the
    same PreToolUse handling path, after `run_self_heal_if_plausible` has already run per
    BC-1.18.005 Precondition 1, so it already inherits this protection; this correction closes the
    gap ONLY for catch point (i)'s separate PostToolUse entry point.

    **Defense-in-depth (Postcondition 8):** independently of this ordering fix, `publish_sealed_shard`
    is now REQUIRED to be write-once — it refuses to overwrite an existing destination and fails
    loud (`E-SHD-009`) on any residual collision, so even a future regression that reintroduces a
    self-heal-pre-pass omission fails SAFE (a loud error) rather than fails SILENT (data loss).

    **Test obligation (closing the "by-construction, no test needed" gap this withdrawn invariant
    left):** this is no longer a bare design-time construction argument; it is discharged by a
    concrete fault-injection test (see EC-023's Canonical Test Vector and the extended VP-NNN
    (pending)/VP-119 rows below) that reproduces the exact `E-SHD-006`-then-content-changing-
    `replace_all` sequence above and asserts the prior seal survives byte-identical and the new
    content seals to the correctly-advanced `seq`. A future call-order change to catch point (i)
    MUST re-run this test before removing the self-heal-first requirement.

## Edge Cases

| ID | Description | Expected Behavior |
|----|-------------|-------------------|
| EC-001 | Single appended block exceeds shard cap (story's own EC-001) | Roll to new shard BEFORE the write is applied (Postcondition 1); the write itself is blocked (Postcondition 2), never partially applied to either the sealed or fresh shard |
| EC-002 | Agent issues `Edit` against a just-rolled artifact without reading the block message | `Edit`'s `old_string` will not match the now-empty fresh shard — the `Edit` tool call itself fails with a standard "old_string not found" error; this is the exact confusing-failure class Postcondition 2's explicit retry wording exists to prevent for the FIRST block, but a repeated blind-retry `Edit` after that still fails at the tool layer (this BC's contract covers the dispatcher's OWN block message, not enforcement of agent compliance) |
| EC-003 | Postcondition 1 step (a)-(b) fails (e.g., filesystem permission error, disk full) before the sealed shard is durably published | Fail-loud: the gate returns `HookResult::Error` (`E-SHD-001`, description refined to "shard-seal-write failure"), not `Block` and not `Continue` — the canonical file is left in its exact pre-roll state; see EC-005 of the S-25.02 story draft ("Shard index unavailable or corrupt") for the sibling dispatch-blocked case |
| EC-004 | Concurrent dispatch attempts to write to the same artifact's shard (two agents/sessions racing) | Atomic temp+rename for the index publish ensures no torn shard-index write; TD-VSDD-053 single-commit-per-burst and the project's factory-lock discipline (ADR-025) prevent concurrent factory-artifacts commits from landing interleaved |
| EC-005 | `MultiEdit` with one edit block that alone exceeds the cap even against a freshly-rolled (0-byte) shard | Roll triggers as normal (BC-1.18.005 EC-004: current_shard_bytes=0), but if `payload_bytes` alone exceeds `shard_cap_bytes - MAX_SINGLE_RECORD_BYTES`'s margin, this indicates a single record larger than the calibrated `MAX_SINGLE_RECORD_BYTES` assumption — the write still blocks per Postcondition 2, and the retry-then-still-too-large condition surfaces to the agent as a repeated block, which is the correct fail-loud signal that the record itself needs to be split by the caller, not silently truncated |
| EC-006 | `/compact-state`'s own `Edit`/`Write` calls against a sharded artifact trigger a mid-extraction roll | Gets shard-awareness for free (ADR-051 Decision 5) — the skill receives the same `Block`-with-retry-instruction message any other caller would; no amendment to the gate mechanism itself is required for this (a small documentation-only note to `compact-state/SKILL.md`'s own retry-loop guidance is recommended, per ADR-051 Decision 5, but is out of this BC's and this burst's write scope — SKILL.md is not a `.factory/specs/` artifact) |
| EC-010 (fix-burst pass-2, F-P2-004) | Postcondition 1 step (c) (atomic-truncate) fails AFTER step (b) (sealed-shard publish) succeeded | `E-SHD-006`: sealed shard durably exists AND canonical file still holds the same pre-roll content (transient, detectable duplicate-content state, not data loss); self-healing recovery resumes from step (c) alone on the next dispatch attempt (Postcondition 1) |
| EC-011 (fix-burst pass-2, F-P2-004) | Postcondition 1 step (d) (index publish) fails AFTER step (c) (atomic-truncate) succeeded | `E-SHD-007`: canonical file is correctly fresh/empty and the sealed shard exists correctly on disk, but the shard-index has not yet recorded the new `[[shard]]` entry (discoverability-metadata gap only, no reader-visible data loss); self-healing index reconciliation runs on the next dispatch attempt (Postcondition 1) |
| EC-012 (fix-burst pass-2, F-P2-005) | An `Edit`/`MultiEdit`'s target content was ALREADY relocated to a SEALED shard by an EARLIER roll (a policy-violating attempt to amend deep-historical content, or a caller operating on stale in-memory state) | The retry-instruction text ("reissue as a fresh `Write` containing only your new entry") is INAPPLICABLE — there is no "new entry" to reissue against the canonical file, because that content no longer lives there. **Sanctioned escape hatch:** a sealed shard file (`<stem>.<seq:04>.md`) is an ORDINARY file that does NOT match any `[[shard]]` config entry's canonical-path pattern (BC-1.18.005 Postcondition 1's zero-cost bypass for unmatched paths) — it is entirely UNGATED by this BC's gate, and a caller with a genuine, policy-sanctioned need to touch historical content addresses the sealed file DIRECTLY by its own on-disk filename, exactly as it would edit any other ordinary file. This gate makes no attempt to detect, permit, or forbid such an edit — that is POLICY-1's concern (enforced at the `consistency-validator`/adversary-prompt agent level), entirely orthogonal to this gate's byte-size-triggered rotation concern. |
| EC-013 (fix-burst pass-2, F-P2-005) | A net-positive `Edit`/`MultiEdit` targets a STILL-MUTABLE tail record (e.g., a same-burst typo fix to an entry not yet sealed away) and pushes `projected_size` over cap | Triggers the SAME generic roll+block+retry sequence as any other over-cap write (Postcondition 1/2) — Invariant 5's append-only-tail assumption is not violated by this case (the target is still-mutable tail content, not deep-historical content); EC-002's `old_string`-mismatch failure mode applies identically |
| EC-014 (NEW, F2 spec-evolution, replace_all closure) | An `Edit{replace_all: true}` against a matched artifact has multiple occurrences of `old_string`; BC-1.18.005's single-occurrence estimate keeps `projected_size <= shard_cap_bytes`, but the TRUE occurrence-multiplied delta would have exceeded it | BC-1.18.005's trigger returns `Continue` (under-projected); the `Edit` is applied, producing an actual over-cap canonical file; Postcondition 7 catch point (i) fires immediately post-write, detects `actual_size > shard_cap_bytes` via `stat()`, and executes the retroactive roll: canonical truncated to 0 bytes, sealed shard published with `bytes_at_seal` equal to the TRUE (over-cap) size and `sealed_retroactively: true` (Postcondition 5) |
| EC-015 (NEW, F2 spec-evolution, replace_all closure) | Postcondition 7 catch point (i) fails to execute at all (e.g., dispatcher process crash between the `replace_all` write's completion and (i)'s own invocation) | The canonical file is left over cap on disk with no seal; catch point (ii) — the leading `stat()`-based backstop probe on the SAME artifact's NEXT matched `Edit`/`Write`/`MultiEdit` dispatch — detects the pre-existing over-cap state BEFORE evaluating that new call's own trigger, and executes the same retroactive roll; Postcondition 7's bounded-window guarantee ("at most one subsequent matched dispatch") holds |
| EC-016 (NEW, F2 spec-evolution, replace_all closure) | Agent's `Edit{replace_all: true}` tool call returns SUCCESS (`Continue`, not `Block`) to the agent, and a retroactive roll subsequently occurs via catch point (i) with no block/retry message ever shown to the agent | Expected and unchanged from this BC's existing EC-002 scope ("this BC's contract covers the dispatcher's OWN block message, not enforcement of agent compliance"): the agent has no proactive signal that a roll occurred; its next `Edit` against the (now-empty) canonical file fails at the tool layer with a standard "old_string not found" error, identically to EC-002's already-specified outcome — no new agent-facing contract is introduced by Postcondition 7 |
| EC-017 (NEW, cluster-2 LOCAL adversary pass-1, F-C2-P1-002) | Catch point (i) crashes without running (as EC-015), and the artifact's NEXT matched dispatch is specifically a `Write` (not an `Edit`/`MultiEdit`) whose own `content` is under-cap | Catch point (ii)'s DEDICATED `stat()` of the canonical file (CORRECTED, F-C2-P1-002 — a probe added specifically because `Write`'s own Postcondition-3 trigger formula performs no `stat()` of its own) detects `actual_size > shard_cap_bytes` BEFORE the `Write` is applied, and executes the retroactive four-step roll (sealing the orphaned over-cap content with `sealed_retroactively: true`) BEFORE the `Write`'s under-cap `content` is ever written to the canonical file — closing the data-loss path where an unguarded `Write` would otherwise silently replace and destroy the un-sealed over-cap history |
| EC-018 (NEW, cluster-2 LOCAL adversary pass-1, F-C2-P1-001) | A retroactive roll (Postcondition 7 catch point (i) or (ii)) crashes after its own step (c) but before its own step (d), i.e. `E-SHD-007` occurs for a RETROACTIVE (not prospective) roll | Self-heal reconciliation (Postcondition 1's `E-SHD-007` recovery) infers `sealed_retroactively` from the recovered entry's own `bytes_at_seal` vs. `shard_cap_bytes` (Invariant 7) rather than hardcoding `false`; since the roll was retroactive, `bytes_at_seal > shard_cap_bytes` and the recovered entry correctly sets `sealed_retroactively=true` |
| EC-019 (NEW, cluster-2 LOCAL adversary pass-2, F-C2-P2-003) | Catch point (ii)'s dedicated `Write`-arm backstop `stat()` of the canonical file fails with a non-`NotFound` error (e.g. `ELOOP` from a symlink loop confined to the canonical path's own final component, or `EACCES`/`EIO`) | The gate fails LOUD: returns `HookResult::Error` (NEW `E-SHD-008`, "backstop probe stat failure" — **mechanism label, not literal `Display` text; RESOLVED cluster-2 LOCAL adversary pass-10, F-C2-P10-004, see `error-taxonomy.md`'s `E-SHD-008` row for the exact emitted message**) WITHOUT applying the `Write` — never fail-open. This closes the asymmetric case where a dereferencing `stat()` fails on a final-component symlink loop while a non-dereferencing `rename(temp, canonical)` would have silently succeeded, applying the `Write` without ever examining (or sealing) the true, possibly-over-cap content the symlink pointed at. `NotFound` remains the legitimate first-ever-write case (BC-1.18.005 EC-004) and is unaffected — the `Write` proceeds normally |
| EC-020 (NEW, cluster-2 LOCAL adversary pass-2, F-C2-P2-004) | An `E-SHD-007`-orphaned, un-indexed sealed shard survives across an F4 `shard_cap_bytes` re-calibration boundary (BC-1.18.005 Postcondition 6/AC-004) before its own reconciliation runs | Invariant 7's Stable-Cap Precondition governs: if the cap was LOWERED, a legitimately prospective seal is mislabeled `sealed_retroactively=true` (false positive); if the cap was RAISED, a genuinely retroactive seal is mislabeled `sealed_retroactively=false`/omitted (false negative). Both are documented, ACCEPTED, operationally negligible under the production-grade lens (audit-trail-only field; Invariant 6's canonical zero-bytes guarantee and the sealed-shard content-preservation guarantee are unaffected in either direction; the window requires two independently rare events to coincide) — no code change follows; the existing inference implementation is already correct under the now-explicit stable-cap precondition |
| EC-021 (NEW, cluster-2 LOCAL adversary pass-3, F-C2-P3-001) | BC-1.18.005's size-trigger fires (`projected_size > shard_cap_bytes`) but the canonical file's content is exactly 0 bytes at the moment `execute_roll`'s step (a) would read it (the incoming call's own payload alone exceeds the cap against an already-empty shard) | `execute_roll` short-circuits to `Ok(None)`: no shard file is published, the canonical file is left untouched at 0 bytes, and no `[[shard]]` index entry is appended (Postcondition 1's empty-canonical short-circuit). The gate still returns `HookResult::Block`, but via the distinct `build_empty_roll_retry_block_reason` template (Postcondition 2's "Empty-canonical retry template"), instructing the agent to recompute or split its payload rather than retry against a "now-empty" shard that was, in fact, already empty before this call |
| EC-022 (NEW, cluster-2 LOCAL adversary pass-3, F-C2-P3-002) | A 0-byte file matching the sealed-shard naming convention (`<stem>.<seq:04>.md`) exists on disk at a self-heal-eligible path — e.g. produced by an external actor outside this BC's write-path guarantees, since no path this BC's own write mechanisms control can ever produce a genuine 0-byte sealed-shard orphan (Invariant 9) | Both self-heal paths (`self_heal_reconcile_missing_index_entries` / `E-SHD-007`-style reconciliation, and `self_heal_resume_from_truncate` / `E-SHD-006`-style recovery) MUST refuse to append a `[[shard]]` entry for this candidate: skip it, emit a `tracing::warn!` diagnostic, and continue reconciling any other genuinely-orphaned entries in the same pass — never index it with `bytes_at_seal = 0`, and never fail the CURRENT dispatch because of it (Invariant 9) |
| EC-023 (NEW, cluster-2 LOCAL adversary pass-4, F-C2-P4-001) | A prior prospective roll crashed as `E-SHD-006` (seal published at `seq=N`, canonical NOT yet truncated, index entry never recorded); the artifact's NEXT matched dispatch is a `replace_all: true` `Edit`/`MultiEdit` whose already-applied edit CHANGES the canonical's content (not byte-identical to the orphaned seal) and pushes it over cap | Catch point (i) runs the self-heal reconciliation pass BEFORE calling `execute_roll` (CORRECTED, Invariant 10/Postcondition 7): the orphaned `seq=N` entry is indexed first, so `next_seal_seq` correctly resolves to `N+1`; the prior seal at `seq=N` is preserved byte-identical, and the new over-cap content seals to `seq=N+1` |
| EC-024 (NEW, cluster-2 LOCAL adversary pass-4, F-C2-P4-002; SCOPED to non-empty destinations, cluster-2 LOCAL adversary pass-8, F-C2-P8-002) | `publish_sealed_shard` is invoked (prospectively or retroactively) with a target `seq` path that ALREADY exists on disk AND is NON-EMPTY (e.g. a residual self-heal gap, or an external actor placing a same-named file with real content) | `publish_sealed_shard`'s `write_exclusive` exclusive-create attempt collides; the gate refuses to overwrite, returns `HookResult::Error` (NEW `E-SHD-009`, "sealed-shard immutability violation"), and applies NEITHER the seal nor any subsequent step (no truncate, no index publish) for this attempt; the pre-existing file at that path is left byte-identical, untouched (Postcondition 8). See EC-025 for the sibling 0-byte-destination case, which does NOT refuse |
| EC-025 (NEW, cluster-2 LOCAL adversary pass-8, F-C2-P8-002; race sub-windows CORRECTED, cluster-2 LOCAL adversary pass-10, F-C2-P10-002) | `publish_sealed_shard` is invoked with a target `seq` path that ALREADY exists on disk but is EXACTLY 0 bytes (e.g. an external anomaly, or a `next_seal_seq` that self-heal's Invariant 9 skip-and-warn guard left permanently un-indexed) | `publish_sealed_shard`'s `write_exclusive` attempt collides with `AlreadyExists`; the gate `stat()`s the destination once, finds it is 0 bytes, unlinks it, and retries `write_exclusive` exactly once — the retry succeeds, the new seal content is durably published at that exact path, and a `tracing::warn!` diagnostic names the reclaimed path. No `E-SHD-009`, no dispatch failure. This closes the permanent-deadlock interaction between Postcondition 8's original write-once refusal and Invariant 9's self-heal skip-and-warn guard (Postcondition 8's 0-byte-destination exception). **A concurrent writer racing this reclaim has TWO distinct sub-windows, not one** (Postcondition 8's corrected text): landing content AFTER the gate's own `unlink()` (before the retry) correctly fails loud `E-SHD-009`; landing content BEFORE the gate's own `unlink()` (after the gate's `stat()`) is silently deleted by that `unlink()`, and the retry then SUCCEEDS with no `E-SHD-009` — an accepted, documented residual race under the current per-dispatch single-process model, hardening OWED to Phase F6 (OWED §4.4 F6-owed list) |
| EC-026 (NEW, cluster-2 LOCAL adversary pass-8, F-C2-P8-004) | "Double-fire" sequence: Postcondition 7 catch point (ii)'s leading backstop probe retroactively seals+truncates a PRE-EXISTING orphaned over-cap canonical BEFORE this dispatch's own trigger is evaluated; this SAME dispatch's own payload then ALSO exceeds the cap against the now-freshly-emptied canonical, so Postcondition 1's `Ok(None)` empty-canonical short-circuit fires a SECOND time within the SAME dispatch | The gate selects the distinct Case B2 template (`build_empty_roll_retry_block_reason` with `preceded_by_backstop_roll: true`), which OMITS the "no roll was performed"/"remains exactly as it was before this call" clauses (false on this path — a roll DID occur) while keeping the identical split-payload guidance; Case B1's wording and the unified Case A template are NOT emitted (Postcondition 2's "Double-fire exception", Invariant 4's Case B1/B2 split) |

## Canonical Test Vectors

| Input | Expected Output | Category |
|-------|----------------|----------|
| **CORRECTED (cluster-2 LOCAL adversary pass-1, F-C2-P1-004 — replaces a stale carry-over from the WITHDRAWN `current + payload` formula, which this row's prior numeric example silently still assumed).** `Write` to `decision-log.md`, current shard 45,000 bytes (irrelevant to the `Write` trigger per BC-1.18.005 Postcondition 3's `projected_size = len(content)` formula), `content` length 50,000 bytes, cap 49,152 (`50,000 > 49,152` → trigger fires) | Publish `decision-log.0001.md` (a NEW file, byte-copy of the 45,000-byte PRE-ROLL canonical content — the sealed shard's size is always the pre-roll canonical content, never the blocked `Write`'s own `content`, which is never applied); atomically replace `decision-log.md`'s content with empty (0 bytes, canonical path never absent); publish index `[[shard]] seq=1 path="decision-log.0001.md" bytes_at_seal=45000`; return `Block{reason: "Shard decision-log rotated (cap 49152 bytes reached)... recompute content to contain ONLY your new entry..."}` (unified wording) | happy-path |
| **CORRECTED (fix-burst pass-2, F-P2-002).** `Edit` to `burst-log.md` causing an over-cap projection | Roll executes identically to the `Write` case (copy-then-atomic-truncate); `Block` reason text uses the SAME unified template as the `Write` case (naming both tool branches within one message, per Postcondition 2) | happy-path |
| Second roll on the same artifact within the same cycle | `[[shard]]` gains a SECOND entry with `seq=2`; `seq=1`'s entry is untouched (append-only index) | edge-case |
| Postcondition 1 step (a)-(b) fails (simulated permission error before sealed shard is published) | `HookResult::Error` (`E-SHD-001`); no shard-index entry published; canonical file left in its pre-roll state | error |
| **NEW (fix-burst pass-2, F-P2-004).** Postcondition 1 step (c) fails after step (b) succeeded (simulated crash between sealed-shard publish and canonical truncate) | `E-SHD-006`: sealed shard `decision-log.0001.md` exists AND `decision-log.md` still holds the same 45,000-byte content; next dispatch attempt detects the byte-identical duplicate and resumes from step (c) alone (truncate + index publish only) | error |
| **NEW (fix-burst pass-2, F-P2-004).** Postcondition 1 step (d) fails after step (c) succeeded (simulated crash between canonical truncate and index publish) | `E-SHD-007`: `decision-log.md` is correctly empty and `decision-log.0001.md` exists on disk, but the shard-index has no `[[shard]] seq=1` entry; next dispatch attempt scans for un-indexed sealed shards and appends the missing entry before evaluating any new trigger | error |
| Whole-corpus `grep -n "D-1234" decision-log*.md` after 2 rolls | Glob matches `decision-log.0001.md`, `decision-log.0002.md`, `decision-log.md` in that lexicographic (and chronological) order | happy-path |
| **NEW (fix-burst pass-2, F-P2-005).** `Edit` targets content already relocated to `decision-log.0001.md` by an earlier roll | Retry-instruction text is inapplicable against the (now-unrelated) canonical file; caller addresses `decision-log.0001.md` directly by filename — ungated, since it matches no `[[shard]]` config entry | edge-case |
| **NEW (F2 spec-evolution, replace_all closure, EC-014).** `Edit{replace_all: true}` to `decision-log.md`; current shard 48,900 bytes; `old_string` (50 bytes) occurs 3 times; `new_string` 250 bytes (per-occurrence delta +200); single-occurrence projected size = 49,100 (`<= 49,152` cap → BC-1.18.005 trigger returns `Continue`, write applied); TRUE post-apply size = 48,900 + 3×200 = 49,500 (`> 49,152`) | Postcondition 7 catch point (i) `stat()`s the canonical file post-write, detects 49,500 > 49,152, executes retroactive roll: publishes `decision-log.0001.md` (49,500 bytes, `bytes_at_seal=49500`, `sealed_retroactively=true`); atomically truncates `decision-log.md` to 0 bytes; publishes the index entry | edge-case |
| **NEW (F2 spec-evolution, replace_all closure, EC-015).** Same over-cap `replace_all` write as above, but Postcondition 7 catch point (i) is simulated as crashed/never-run | Canonical file remains at 49,500 bytes (over cap) until the artifact's next matched dispatch; catch point (ii)'s leading probe detects `actual_size (49,500) > shard_cap_bytes (49,152)` BEFORE evaluating the new call's own trigger, and executes the retroactive roll identically to the row above | edge-case |
| **NEW (F2 spec-evolution, replace_all closure).** `Edit{replace_all: true}` where `old_string` occurs exactly once (`occurrence_count=1`) | Single-occurrence estimate equals the TRUE delta exactly (the identity case) — BC-1.18.005's trigger is exact; Postcondition 7 catch point (i)'s `stat()` finds `actual_size <= shard_cap_bytes` and takes no action | happy-path |
| **NEW (cluster-2 LOCAL adversary pass-1, F-C2-P1-002, EC-017).** Catch point (i) crashes (as in the EC-015 vector above); canonical file left at 49,500 bytes (over cap, un-sealed, no roll ever started); artifact's NEXT matched dispatch is a `Write` with `content` length 3,000 bytes (under cap on its own) | Catch point (ii) performs its OWN dedicated `stat()` of the canonical file (no reuse of a `Write`-side stat, since `Write`'s trigger performs none) BEFORE applying the `Write`; finds `actual_size (49,500) > shard_cap_bytes (49,152)`; executes the retroactive roll: publishes `decision-log.0001.md` (`bytes_at_seal=49500`, `sealed_retroactively=true`), atomically truncates `decision-log.md` to 0 bytes, publishes the index entry — ONLY THEN is the `Write`'s 3,000-byte `content` applied, against the now-empty canonical file, with no history lost | edge-case |
| **NEW (cluster-2 LOCAL adversary pass-1, F-C2-P1-001, EC-018).** Retroactive roll (Postcondition 7 catch point (i), over-cap `replace_all` write, `bytes_at_seal=49,500`, `shard_cap_bytes=49,152`) crashes after its own step (c) (canonical truncated to 0 bytes) but before its own step (d) (index publish) | `E-SHD-007`: on the next dispatch attempt, the gate finds `decision-log.0001.md` (49,500 bytes) on disk and absent from the index; reconciliation computes `bytes_at_seal=49,500 > shard_cap_bytes=49,152` (Invariant 7) and appends the recovered `[[shard]]` entry with `sealed_retroactively=true` — never a hardcoded `false` | error |
| **NEW (cluster-2 LOCAL adversary pass-2, F-C2-P2-003, EC-019).** Catch point (ii)'s dedicated `Write`-arm `stat()` of `decision-log.md` fails with `ELOOP` (canonical path is a symlink loop confined to its own final component) | Gate fails LOUD: returns `HookResult::Error` naming `E-SHD-008` — the exact emitted message is `E-SHD-008: Write-arm backstop stat() failed for artifact_stem "decision-log" at '<path>' — cannot confirm whether the canonical file is a crash-orphaned, over-cap shard; refusing to let this Write proceed until the underlying I/O condition is resolved: ELOOP` (this row's prior "backstop probe stat failure for decision-log: ELOOP" gloss was a **mechanism label, not literal `Display` text — RESOLVED cluster-2 LOCAL adversary pass-10, F-C2-P10-004**, corrected here to the verbatim message per `error-taxonomy.md`'s `E-SHD-008` row); the `Write`'s `content` is NEVER applied; no `rename` onto the canonical path is attempted — closing the asymmetric path where a non-dereferencing `write_atomic` rename could otherwise have silently succeeded and destroyed the un-examined content the symlink pointed at | error |
| **NEW (cluster-2 LOCAL adversary pass-2, F-C2-P2-004, EC-020).** `E-SHD-007`-orphaned sealed shard `decision-log.0001.md` (`bytes_at_seal=49,500`, sealed while `shard_cap_bytes=49,152`) awaits reconciliation; before reconciliation runs, the F4 calibration harness re-locks `shard_cap_bytes` to 50,000 (RAISED) | Reconciliation computes `49,500 > 50,000` → FALSE → recovered entry sets `sealed_retroactively=false`/omitted, even though the seal was genuinely retroactive under the OLD cap (49,152) — a documented, ACCEPTED false negative (Invariant 7 Stable-Cap Precondition); no hard invariant violated, no data loss, no code change required | edge-case |
| **NEW (cluster-2 LOCAL adversary pass-3, F-C2-P3-001, EC-021).** `Write` to `decision-log.md`; canonical file is currently 0 bytes (first-ever write, or the shard was already emptied by a prior roll and never appended to since); `content` length 60,000 bytes, cap 49,152 (`60,000 > 49,152` → BC-1.18.005's trigger fires) | `execute_roll` returns `Ok(None)`: no `decision-log.NNNN.md` is published, `decision-log.md` remains untouched at 0 bytes, no `[[shard]]` entry is appended; gate returns `Block{reason: "Shard decision-log is already empty; your own payload alone (60000 bytes) exceeds the cap (49152 bytes). Recompute or split your payload..."}` (the DISTINCT `build_empty_roll_retry_block_reason` template, not the unified rotate-and-retry template) | edge-case |
| **NEW (cluster-2 LOCAL adversary pass-3, F-C2-P3-002, EC-022).** A 0-byte file `decision-log.0003.md` (matching the sealed-shard naming convention but never produced by any roll `execute_roll` performed) exists on disk at a `seq` value absent from the shard-index; a self-heal reconciliation pass runs (triggered by an unrelated `E-SHD-007` recovery elsewhere, or an operator-invoked sweep) | Self-heal detects the candidate's on-disk size is 0 bytes, skips it (appends NO `[[shard]]` entry for `decision-log.0003.md`), emits a `tracing::warn!` diagnostic naming the anomalous path, and proceeds to reconcile any other genuinely-orphaned entries in the same pass without failing the current dispatch (Invariant 9) | edge-case |
| **NEW (cluster-2 LOCAL adversary pass-4, F-C2-P4-001, EC-023).** `E-SHD-006` orphan: `decision-log.0001.md` holds durable content X (10,000 bytes); `decision-log.md` (canonical) ALSO still holds X (untruncated); the shard-index has NO `[[shard]]` entry (empty). Next matched dispatch: `Edit{replace_all: true}` on `decision-log.md` whose applied edit changes content to X′ (60,000 bytes, `> shard_cap_bytes=49,152`, `X′ ≠ X`) | Catch point (i) fires: runs self-heal reconciliation FIRST, which indexes the orphan (`seq=1`, `path="decision-log.0001.md"`, `bytes_at_seal=10000`, `sealed_retroactively` inferred per Invariant 7); THEN computes `next_seal_seq=2` from the now-reconciled index; publishes `decision-log.0002.md` (60,000 bytes, `bytes_at_seal=60000`, `sealed_retroactively=true`); atomically truncates `decision-log.md` to 0 bytes; publishes the `seq=2` index entry. `decision-log.0001.md` is UNCHANGED (still X, 10,000 bytes) — no data loss | edge-case |
| **NEW (cluster-2 LOCAL adversary pass-4, F-C2-P4-002, EC-024; SCOPED to non-empty, cluster-2 LOCAL adversary pass-8, F-C2-P8-002).** A roll (prospective or retroactive) computes `next_seal_seq=3`, but `decision-log.0003.md` ALREADY exists on disk, holding 500 bytes of unrelated content (e.g. a residual self-heal gap not covered by the EC-023 fix, or an external actor) | `publish_sealed_shard`'s `write_exclusive` attempt detects the destination already exists before completing the exclusive-create, `stat()`s it, finds it non-empty (500 bytes), and refuses to overwrite: returns `HookResult::Error` (`E-SHD-009`, "refusing to overwrite an already-sealed shard at 'decision-log.0003.md' for artifact_stem \"decision-log\" — sealed shards are write-once/immutable; this seq already has durable content on disk"); `decision-log.0003.md`'s pre-existing content is verified byte-identical after the attempt (never overwritten); no truncate, no index publish for this attempt | error |
| **NEW (cluster-2 LOCAL adversary pass-8, F-C2-P8-002, EC-025).** A roll computes `next_seal_seq=3`, but `decision-log.0003.md` ALREADY exists on disk at exactly 0 bytes (an external anomaly; self-heal's Invariant 9 guard never indexed it, so `next_seal_seq` never advanced past it) | `publish_sealed_shard`'s `write_exclusive` attempt collides with `AlreadyExists`; `stat()` finds the pre-existing file is 0 bytes; the gate unlinks it and retries `write_exclusive` exactly once — the retry succeeds, `decision-log.0003.md` now durably holds the new seal content, and a `tracing::warn!` diagnostic names the reclaimed path; no `E-SHD-009`, roll completes normally (truncate + index publish proceed) | edge-case |
| **NEW (cluster-2 LOCAL adversary pass-8, F-C2-P8-002, EC-025 race variant); NARROWED (cluster-2 LOCAL adversary pass-10, F-C2-P10-002) to the `unlink()`-to-retry sub-window specifically.** Same as above, but a concurrent writer places 20 bytes of real content at `decision-log.0003.md` AFTER the gate's own `unlink()` of the 0-byte file has completed, but BEFORE the gate's single reclaim retry executes | The retry's `write_exclusive` fails `AlreadyExists` a SECOND time; the gate does NOT retry again (bounded to exactly one reclaim attempt) — it fails loud, returning `HookResult::Error` (`E-SHD-009`); the concurrent writer's 20 bytes are left byte-identical, untouched; no truncate, no index publish for this attempt | error |
| **NEW (cluster-2 LOCAL adversary pass-10, F-C2-P10-002, EC-025 accepted-residual sub-window — documents current behavior, NOT a required test in this burst; fault-injection test OWED to Phase F6 per Postcondition 8's OWED §4.4 note).** A concurrent writer places 20 bytes of real content at `decision-log.0003.md` AFTER the gate's own `stat()` observed it as 0 bytes, but BEFORE the gate's own `unlink()` call executes | The gate's `unlink()` unconditionally removes whatever currently occupies the path — including the concurrent writer's 20 bytes, which are silently destroyed with no re-verification of size. The retry's `write_exclusive` then finds the destination vacant and SUCCEEDS: the new seal content is durably published, no `E-SHD-009` is raised, and the concurrent writer's content is permanently, silently lost. Accepted as a documented, bounded, near-impossible-under-the-current-per-dispatch-single-process-execution-model residual risk (Postcondition 8); full closure (an `O_EXCL` re-create-then-swap primitive) is OWED to Phase F6 targeted-hardening, not enacted here | error |
| **NEW (cluster-2 LOCAL adversary pass-8, F-C2-P8-004, EC-026).** Catch point (ii) retroactively seals+truncates a pre-existing orphaned canonical (60,000 bytes, `> shard_cap_bytes=49,152`) BEFORE evaluating this dispatch's own trigger; this SAME dispatch is itself a `Write` whose own `content` is 55,000 bytes (`> 49,152`), so against the now-freshly-emptied (0-byte) canonical, Postcondition 1's `Ok(None)` short-circuit fires a second time in this dispatch | The gate returns `Block{reason: "Shard decision-log is now empty (a prior over-cap shard was retroactively rotated by this same call before your payload was evaluated); your own payload alone (55000 bytes) exceeds the cap (49152 bytes). Recompute or split your payload..."}` — Case B2's wording (`preceded_by_backstop_roll: true`), never Case B1's "no roll was performed" wording, never the unified Case A template | edge-case |

## Verification Properties

| VP-NNN | Property | Proof Method |
|--------|----------|-------------|
| VP-118 | Seal-then-block ordering invariant — the shard-index TOML always contains the seal entry for a given roll BEFORE (or atomically with) the corresponding `HookResult::Block` is observed by the caller | integration test (dispatcher harness: assert index file content immediately upon receiving the Block result) |
| VP-119 | No-over-cap invariant (no sealed shard file's byte size, sampled at any point after this BC's gate executes, ever exceeds its recorded `shard_cap_bytes` at seal time); Stable-current-filename invariant (the canonical filename is never itself renamed to a sealed name across any sequence of rolls; `stat(canonical_path)` always succeeds — CORRECTED, F-P2-003: verified against the copy-then-atomic-truncate mechanism, which structurally cannot vacate the canonical path, rather than the withdrawn rename-away mechanism). **EXTENDED (cluster-2 LOCAL adversary pass-4, F-C2-P4-002, Postcondition 8) — sealed-shard write-once/immutability invariant, scoped to NON-EMPTY destinations (cluster-2 LOCAL adversary pass-8, F-C2-P8-002)**: no `[[shard]]`-eligible NON-EMPTY destination path is ever overwritten by a subsequent seal-publish attempt; a `seq` collision against non-empty content surfaces as `HookResult::Error`/`E-SHD-009`, never a silent overwrite (EC-024). **NEW facet (F-C2-P8-002) — 0-byte-destination reclaim**: a `seq` collision against a 0-byte pre-existing destination is reclaimed (unlinked and overwritten) via exactly one bounded retry, never refused, and never loops — closing the Invariant-9/Postcondition-8 permanent-deadlock interaction (EC-025) | proptest — two facets, arbitrary sequence of writes / roll sequence against a simulated artifact: every sealed shard's `bytes_at_seal <= shard_cap_bytes`; AND `stat(canonical_path)` always succeeds and is never the sealed inode from a prior roll. **NEW facet (F-C2-P4-002):** fixture pre-creating a NON-EMPTY file at a computed `next_seal_seq` destination path before a roll attempts to publish there; assert `HookResult::Error`/`E-SHD-009`, byte-identical preservation of the pre-existing file, and no truncate/index-publish side effect (EC-024). **NEW facet (F-C2-P8-002):** fixture pre-creating a 0-BYTE file at a computed `next_seal_seq` destination; assert the roll succeeds (reclaim, no `E-SHD-009`), AND a race-injection variant (content appears between the probe and the single retry) asserting exactly one retry is attempted and the race case fails loud with `E-SHD-009` (EC-025) |
| VP-120 | Retry-wording determinism (the block reason's retry instruction is the SAME fixed unified template regardless of the original tool name — CORRECTED, F-P2-002: no longer a per-tool-name choice between two divergent wordings); Fail-loud shard-seal-write-failure invariant (`E-SHD-001`, Postcondition 1 steps (a)-(b) failure, `HookResult::Error`, canonical file left untouched, per EC-003). **EXTENDED (cluster-2 LOCAL adversary pass-3, F-C2-P3-001, Invariant 4 adjudication):** two-template determinism — the over-cap-with-content case always selects the unified rotate-and-retry template and the over-cap-without-content (empty-canonical, `Ok(None)`) case always selects the distinct `build_empty_roll_retry_block_reason` template (EC-021); template selection is a pure function of which Postcondition 1 code path executed, never tool-name-dependent WITHIN either case. **EXTENDED (cluster-2 LOCAL adversary pass-8, F-C2-P8-004, Invariant 4 Case B1/B2 split):** three-template determinism — Case B itself splits into B1 (pure empty-canonical, "no roll was performed" wording) and B2 (double-fire empty-canonical, wording that OMITS the false "no roll was performed"/"remains exactly as it was" clauses); selection between B1 and B2 is a pure function of whether Postcondition 7 catch point (ii) fired earlier in the SAME dispatch; no fourth wording ever appears | unit test — three facets: table-driven over both tool-name classes for the over-cap-with-content case, asserting identical template with tool-specific guidance embedded within it; injected shard-seal-write-failure FS asserting `E-SHD-001` + pre-roll-state preservation; **NEW facet (F-C2-P3-001):** table-driven over the `Ok(None)` empty-canonical short-circuit (both `Edit`/`MultiEdit` and `Write` triggers), asserting the DISTINCT empty-canonical template is selected, the unified template is NOT selected, and no shard file or index entry is produced (EC-021). **NEW facet (F-C2-P8-004):** fault-inject the double-fire sequence (a pre-existing orphaned over-cap canonical reclaimed by catch point (ii), immediately followed by this SAME call's own over-cap payload); assert Case B2's wording is emitted verbatim, never Case B1's, never Case A's (EC-026) |
| VP-138 | Truncate-after-seal self-heal invariant (`E-SHD-006`) — a crash between Postcondition 1 step (b) (sealed-shard publish) and step (c) (atomic-truncate) resolves, on the NEXT dispatch attempt, to a byte-identity check against the sealed shard followed by resume-from-step-(c)-only recovery (truncate + index publish only; the already-correct sealed shard is never rewritten), per EC-010. **EXTENDED (cluster-2 LOCAL adversary pass-3, F-C2-P3-002, Invariant 9):** `self_heal_resume_from_truncate` MUST refuse to append a `[[shard]]` entry for a candidate sealed-shard file whose on-disk size is 0 bytes — skip it, emit a diagnostic, and never index it with `bytes_at_seal = 0` (EC-022) | integration test (fault-injection across two dispatches: simulate a crash between steps (b) and (c); assert post-recovery state is exactly one sealed shard, one index entry, and an empty canonical file). **NEW facet (F-C2-P3-002):** fixture placing a 0-byte file at a self-heal-eligible path; assert the reconciliation pass skips it, appends no `[[shard]]` entry, and does not fail the current dispatch (EC-022) |
| VP-139 | Index-after-truncate self-heal invariant (`E-SHD-007`) — a crash between Postcondition 1 step (c) (atomic-truncate) and step (d) (index publish) resolves, on the NEXT dispatch attempt, to a filesystem scan for un-indexed sealed shards followed by an append-only reconciliation of the missing `[[shard]]` entry, per EC-011 and Postcondition 5's schema. **EXTENDED (cluster-2 LOCAL adversary pass-3, F-C2-P3-002, Invariant 9):** `self_heal_reconcile_missing_index_entries` MUST refuse to append a `[[shard]]` entry for a candidate sealed-shard file whose on-disk size is 0 bytes — skip it, emit a diagnostic, and never index it with `bytes_at_seal = 0` (EC-022) | integration test (fault-injection across two dispatches: simulate a crash between steps (c) and (d); assert the reconciled index gains exactly the missing entry, existing entries untouched, idempotent on repeat). **NEW facet (F-C2-P3-002):** fixture placing a 0-byte file at a self-heal-eligible, un-indexed path; assert the reconciliation pass skips it, appends no `[[shard]]` entry for it, still reconciles any other genuinely-orphaned entries in the same pass, and does not fail the current dispatch (EC-022) |
| VP-NNN (pending) | **NEW (F2 spec-evolution, replace_all closure; EXTENDED cluster-2 LOCAL adversary pass-1, F-C2-P1-001/002).** Bounded post-write reconciliation for under-projected `replace_all: true` writes (Postcondition 7) — actual-size `stat()` check at catch point (i)/(ii), retroactive four-step roll reuse (`E-SHD-001`/`E-SHD-006`/`E-SHD-007` self-healing applies identically), the `sealed_retroactively` audit flag (Postcondition 5), and the bounded-window guarantee (over-cap observable for at most one subsequent matched dispatch, never indefinitely), per EC-014/EC-015/EC-016. **EXTENDED scope (F-C2-P1-002):** catch point (ii)'s per-tool cost split — `Edit`/`MultiEdit` reuse BC-1.18.005's existing `stat()`, `Write` performs a NEW dedicated `stat()` before applying — and the data-loss-closure property that a `Write` following a crash-orphaned, un-sealed, over-cap canonical file NEVER applies directly against it (EC-017). **EXTENDED scope (F-C2-P1-001):** the `sealed_retroactively` recovery-by-inference rule (Invariant 7) — `E-SHD-007` self-heal reconciliation of an orphaned sealed shard MUST set `sealed_retroactively = (bytes_at_seal > shard_cap_bytes)`, never a hardcoded `false` (EC-018). **EXTENDED scope (cluster-2 LOCAL adversary pass-2, F-C2-P2-003/004):** (i) the catch point (ii) backstop probe's uniform fail-loud `stat()`-failure disposition (Invariant 8) — a non-`NotFound` error MUST return `HookResult::Error`/`E-SHD-008` and MUST NOT apply the `Write`, never fail-open (EC-019); (ii) Invariant 7's Stable-Cap Precondition — `E-SHD-007` reconciliation of an orphaned shard that survived a `shard_cap_bytes` re-calibration boundary MAY mislabel `sealed_retroactively` in either direction (false positive on a cap decrease, false negative on a cap increase), an accepted, bounded, audit-trail-only risk with no hard-invariant impact (EC-020). **EXTENDED scope (cluster-2 LOCAL adversary pass-4, F-C2-P4-001, CORRECTED Invariant 10) — catch point (i)'s REQUIRED self-heal-first sequencing**: catch point (i) MUST run `run_self_heal_if_plausible` BEFORE calling `execute_roll`, so `next_seal_seq` is always computed against a reconciled index and never collides with, and overwrites, an orphaned prior seal (EC-023); this facet is the discharge of the corrected Invariant 10's test obligation | integration test (fault-injection: simulate a `replace_all` write whose true occurrence-multiplied size exceeds cap while BC-1.18.005's single-occurrence estimate does not; assert catch point (i) fires and reconciles; separately, simulate catch point (i) crashing/never-running and assert catch point (ii)'s next-dispatch backstop reconciles instead — table-driven over BOTH a following `Edit`/`MultiEdit` and a following `Write`, asserting the `Write` case performs its own dedicated `stat()` call and never applies its `content` against an over-cap canonical file (EC-017); separately, fault-inject a crash between a retroactive roll's own step (c) and step (d), and assert the self-heal-recovered `[[shard]]` entry's `sealed_retroactively` field equals `bytes_at_seal > shard_cap_bytes` (EC-018); separately, table-driven over a simulated `stat()` error class (`NotFound` vs. `ELOOP`/`EACCES`/`EIO`), asserting `NotFound` proceeds to apply the `Write` and every other error returns `HookResult::Error`/`E-SHD-008` without applying it (EC-019); separately, fault-inject a `shard_cap_bytes` re-calibration strictly between an orphaned shard's `sealed_at` and its own `E-SHD-007` reconciliation, table-driven over cap-lowered and cap-raised directions, asserting the resulting mislabel matches Invariant 7's Stable-Cap Precondition exactly and that no other invariant is violated (EC-020)). **NEW facet (F-C2-P4-001):** fault-inject an `E-SHD-006` orphan (seal at `seq=N`, canonical untruncated, index empty) followed by a content-CHANGING `replace_all` write that pushes the canonical over cap; assert catch point (i) reconciles the orphan BEFORE publishing the new seal, the orphaned `seq=N` file is preserved byte-identical, and the new content is published at `seq=N+1` (EC-023). **Allocation OWED to Phase F6 targeted-hardening (architect/formal-verifier), mirroring BC-1.18.005 EC-017's own VP-owed-to-F6 precedent (S2502-CLUSTER1-PASS5 STATE.md Drift Item) — product-owner does NOT self-allocate a VP number in this burst.** |

**Fix-burst note (fix-burst pass-3, F-P3-006):** VP-118's and VP-119's previously-separate rows are
each collapsed to ONE row (multi-facet convention). The prior separate VP-118 "partial-failure
self-healing invariant" row (spanning `E-SHD-001`/`E-SHD-006`/`E-SHD-007`) is REMOVED — it
duplicated and OVER-CLAIMED coverage that VP-INDEX v3.04 authoritatively assigns per-code:
`E-SHD-001`'s fail-loud leg is VP-120's (already in this table, "fail-loud shard-seal-write error
`E-SHD-001`" facet — see VP Anchors below), `E-SHD-006` is VP-138's, and `E-SHD-007` is VP-139's
(both already listed as separate rows in this table). No test coverage is lost; only the
mis-attributed BC-side claim under VP-118 is removed. VP-118's remaining single row (seal-then-block
ordering) is its genuine, non-overlapping scope.

## Related BCs

- BC-1.18.005 — owns the size-trigger and cap formula that determines when this BC's roll fires (depends on)
- BC-1.18.007 — retention/compaction policy consumes this BC's shard-index schema to identify archival candidates (composes with)
- BC-1.18.008 — the one-time backfill-split reuses this BC's exact seal+create+index-publish sequence, applied retroactively (composes with)
- BC-1.18.009 — mechanism B1 (BC-INDEX changelog rotation) is triggered by the SAME native gate this BC defines, with a different artifact-shape case (composes with)
- BC-1.18.010 — mechanism B2 (BC-INDEX per-subsystem sharding) is triggered by the SAME native gate, keyed by a manifest instead of the stable-filename trick (composes with)
- BC-1.18.012 — the governed one-time B1 changelog backfill migration reuses this BC's staging discipline pattern (temp-file-then-rename atomic-write) analogous to BC-1.18.008's relationship to this BC (related to)

## Architecture Anchors

- `crates/factory-dispatcher/src/shard_manager.rs` (new module) — publish-sealed-shard+atomic-truncate-canonical+index-publish sequence (CORRECTED, F-P2-003: copy-then-atomic-truncate, not rename-away), `HookResult::Block` construction with the unified retry-instruction text
- `crates/hook-sdk/src/result.rs` — `HookResult` enum (`Continue`/`Block { reason }`/`Error { message }`), the structural contract motivating block-and-retry over transparent redirection
- `crates/factory-dispatcher/src/indeterminate_marker.rs` — `write_indeterminate_marker`'s temp-file-then-rename atomic-write pattern, reused for shard-index publication
- `crates/last-amended-migrate/src/atomic_write.rs` — `write_atomic`, the alternative existing atomic-write primitive this BC's implementation may reuse instead of duplicating `indeterminate_marker.rs`'s
- `crates/factory-dispatcher/src/shard_manager.rs` (extends the existing module, F2 spec-evolution, replace_all closure) — Postcondition 7's post-write reconciliation: catch point (i), a native PostToolUse-side `stat()` check alongside the existing native PreToolUse handling BC-1.18.005 Precondition 1 already establishes; and catch point (ii), a leading `stat()`-comparison probe within the existing native PreToolUse handling path, before BC-1.18.005's own trigger evaluation. Both reuse Postcondition 1's existing seal+atomic-truncate+index-publish sequence verbatim — no new atomic-write primitive, no new `HookResult` variant, no new `hooks-registry.toml` entry (this remains native dispatcher code, not a WASM plugin, per the existing "why native, not WASM" rationale, BC-1.18.005 Postcondition 2)
- `crates/factory-dispatcher/src/shard_manager.rs` `execute_roll` (NEW, cluster-2 LOCAL adversary pass-3, F-C2-P3-001) — the `Ok(None)` empty-canonical short-circuit (Postcondition 1) and its distinct `build_empty_roll_retry_block_reason` message-construction function (Postcondition 2's "Empty-canonical retry template"), invoked from the SAME trigger-fire branch that constructs the unified rotate-and-retry template, selecting between the two per Invariant 4's adjudicated case scoping. **CORRECTED (cluster-2 LOCAL adversary pass-4, F-C2-P4-003) — `build_empty_roll_retry_block_reason` takes a THIRD parameter, `payload_len_bytes: usize`**, alongside `artifact_stem` and `shard_cap_bytes`, so the message can interpolate `<N>` (Postcondition 2's Empty-canonical retry template, EC-021). **CORRECTED (F-C2-P4-004) — `execute_roll`'s canonical-file read (step (a)) and its sealed-shard write (step (b)) are BYTE-LEVEL** (`std::fs::read`/`std::fs::write`), never a UTF-8-fallible `read_to_string`/`&str` path. **CORRECTED (cluster-2 LOCAL adversary pass-8, F-C2-P8-004, MINOR) — `build_empty_roll_retry_block_reason` gains a FOURTH parameter, `preceded_by_backstop_roll: bool`**, selecting between Case B1's and Case B2's wording (Postcondition 2's "Double-fire exception", Invariant 4's Case B1/B2 split, EC-026).
- `crates/factory-dispatcher/src/shard_manager.rs` `execute_roll` / Postcondition 7 catch point (i) (`reconcile_post_write_replace_all_overcap`) — **CORRECTED (cluster-2 LOCAL adversary pass-4, F-C2-P4-001, MAJOR) — catch point (i) MUST call `run_self_heal_if_plausible` BEFORE calling `execute_roll`**, matching the PreToolUse Flat arm's existing call order (BC-1.18.005 Precondition 1), so `next_seal_seq` is always computed against a reconciled shard-index and never collides with an orphaned prior seal (corrected Invariant 10, EC-023). This WITHDRAWS the prior self-heal-pre-pass-free design.
- `crates/factory-dispatcher/src/shard_manager.rs` `publish_sealed_shard` / `write_exclusive` (CORRECTED, cluster-2 LOCAL adversary pass-4, F-C2-P4-002, MINOR) — gains a write-once, no-clobber exclusive-create guard (`write_exclusive`, `std::fs::hard_link`-based, never a `write_atomic` rename — CORRECTED, cluster-2 LOCAL adversary pass-8, F-C2-P8-001, MEDIUM); refuses to overwrite an existing NON-EMPTY destination and returns `HookResult::Error` (NEW `E-SHD-009`) on collision, applying neither the seal nor any subsequent roll step (Postcondition 8, EC-024). **EXTENDED (cluster-2 LOCAL adversary pass-8, F-C2-P8-002, MEDIUM) — bounded 0-byte-destination reclaim**: on an `AlreadyExists` collision, `stat()`s the pre-existing destination exactly once; if 0 bytes, unlinks it and retries `write_exclusive` exactly once (never in a loop); a second collision on retry still fails loud with `E-SHD-009` (Postcondition 8's 0-byte-destination exception, EC-025)
- `crates/factory-dispatcher/src/shard_manager.rs` `self_heal_reconcile_missing_index_entries` / `self_heal_resume_from_truncate` (NEW guard, cluster-2 LOCAL adversary pass-3, F-C2-P3-002) — the 0-byte-orphan skip-and-warn guard required by Invariant 9, applied before either function appends a `[[shard]]` entry for a filesystem-discovered candidate

## SDK Grounding Evidence

Literal stable-anchor greps substantiating this BC's external-artifact claims (POLICY 5;
no `grep -n` / no file:line citations per TD-VSDD-091):

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

Confirms the exact three-variant `HookResult` contract this BC's Precondition 3 and Invariant 1
depend on — no fourth "redirect" variant exists, grounding the "transparent redirection is
structurally impossible" claim in the Description.

```
$ grep -oE "^pub fn write_indeterminate_marker|^pub fn block_if_marker_check|^pub fn should_write_marker" crates/factory-dispatcher/src/indeterminate_marker.rs
pub fn block_if_marker_check
pub fn should_write_marker
pub fn write_indeterminate_marker
```

Confirms `write_indeterminate_marker`'s existence as the temp-file-then-rename atomic-write
precedent this BC's Postcondition 1 steps (b)-(d) and Architecture Anchors cite.

```
$ grep -oE "fn write_exclusive\(path: &Path, content: &\[u8\]\) -> io::Result<\(\)>" crates/factory-dispatcher/src/shard_manager.rs
fn write_exclusive(path: &Path, content: &[u8]) -> io::Result<()>
```

```
$ grep -oE "std::fs::hard_link" crates/factory-dispatcher/src/shard_manager.rs
std::fs::hard_link
std::fs::hard_link
```

Confirms `write_exclusive`'s existence as a genuinely SEPARATE, `std::fs::hard_link`-based
exclusive-create primitive (NOT `write_atomic`'s rename) — grounding this BC's corrected step (b)
mechanism text (Description, Postcondition 1 step (b), Postcondition 8, Invariant 2; cluster-2
LOCAL adversary pass-8, F-C2-P8-001).

```
$ grep -oE "^pub fn write_atomic" crates/last-amended-migrate/src/atomic_write.rs
pub fn write_atomic
```

Confirms the alternative atomic-write primitive (`write_atomic`) this BC's Architecture Anchors
name as a reuse candidate.

## Story Anchor

S-25.02 — Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Artifacts

## VP Anchors

- VP-118, VP-119, VP-120 — allocated by formal-verifier (S-25.02 F2 verification-property extension burst; VP-INDEX v3.02). VP-118 (integration; publish-sealed-shard→atomic-truncate-canonical→atomic-index-publish before Block + same-invocation atomicity + NEW partial-failure self-healing invariant per fix-burst pass-2), VP-119 (proptest; no-over-cap + stable-current-filename — re-verified against the corrected copy-then-atomic-truncate mechanism), VP-120 (unit-test; retry-wording determinism — re-verified as a single unified template, not a per-tool-name choice — + fail-loud shard-seal-write error E-SHD-001 + NEW E-SHD-006/E-SHD-007 partial-failure codes). Formal-verifier should review VP-118/119/120 bodies against this fix-burst's corrected mechanics (copy-then-truncate instead of rename-then-create; unified retry wording; staged 4-step sequence) — not yet actioned in this burst.
- VP-138, VP-139 — allocated by formal-verifier (S-25.02 F2 verification-property fix-burst pass-2; VP-INDEX v3.04, F-P2-004 partial-failure-code symmetry: every E-SHD code now has a VP leg). VP-138 (integration; Postcondition 1 step (c)/EC-010/Invariant 2/Invariant 3 — E-SHD-006 self-healing resume-from-truncate), VP-139 (integration; Postcondition 1 step (d)/EC-011/Postcondition 5 — E-SHD-007 self-healing index reconciliation). Back-references added S-25.02 F2 residual-cleanup micro-burst (formal-verifier's VP-138/VP-139 bodies already cited this BC in `source_bc`; this BC's own Verification Properties table and VP Anchors list did not yet cite them back — gap closed here, reference-only, no behavior change). **EXTENDED (cluster-2 LOCAL adversary pass-3, F-C2-P3-002, Invariant 9) — VP-138 and VP-139 each gain a NEW facet covering the self-heal 0-byte-orphan guard** (`self_heal_resume_from_truncate` and `self_heal_reconcile_missing_index_entries` respectively MUST skip, never index, a candidate sealed-shard file whose on-disk size is 0 bytes, per EC-022). **VP citation change — routed to architect:** formal-verifier/architect must propagate this extension to VP-INDEX, `verification-architecture.md`, and `verification-coverage-matrix.md` per `vp_index_is_vp_catalog_source_of_truth` (POLICY 9).
- VP-120 — allocated by formal-verifier (see above, VP-INDEX v3.02/v3.04). **EXTENDED (cluster-2 LOCAL adversary pass-3, F-C2-P3-001, Invariant 4 adjudication) — VP-120 gains a NEW two-template-determinism facet**: the over-cap-with-content case selects the unified rotate-and-retry template and the over-cap-without-content (`Ok(None)` empty-canonical short-circuit) case selects the distinct `build_empty_roll_retry_block_reason` template, selection being a pure function of the Postcondition 1 code path taken, per EC-021. **VP citation change — routed to architect:** formal-verifier/architect must propagate this extension to VP-INDEX, `verification-architecture.md`, and `verification-coverage-matrix.md` per `vp_index_is_vp_catalog_source_of_truth` (POLICY 9).
- VP-NNN (pending, F2 spec-evolution, replace_all closure; EXTENDED cluster-2 LOCAL adversary pass-1, F-C2-P1-001/002; EXTENDED cluster-2 LOCAL adversary pass-2, F-C2-P2-003/004; EXTENDED cluster-2 LOCAL adversary pass-4, F-C2-P4-001) — Postcondition 7's bounded post-write reconciliation for under-projected `replace_all: true` writes (catch point (i)/(ii) `stat()`-based reconciliation, retroactive four-step roll reuse, `sealed_retroactively` audit flag, bounded-window guarantee); EXTENDED to cover catch point (ii)'s per-tool `stat()` cost split and `Write`-path data-loss closure (EC-017, Invariant 7's sibling scope) and the `sealed_retroactively` recovery-by-inference rule for `E-SHD-007` self-heal (EC-018, Invariant 7). **EXTENDED (pass-2):** catch point (ii)'s uniform fail-loud backstop-probe `stat()`-failure disposition — `NotFound` proceeds, every other error returns `HookResult::Error`/`E-SHD-008` without applying the `Write` (EC-019, Invariant 8); Invariant 7's Stable-Cap Precondition governing the bounded, accepted, bidirectional `sealed_retroactively` mislabel risk under a `shard_cap_bytes` re-calibration spanning a shard's seal-to-reconciliation window (EC-020). **EXTENDED (pass-4, F-C2-P4-001, REPLACES the withdrawn O-C2-P3-001-derived scope of the prior Invariant 10):** catch point (i)'s REQUIRED self-heal-first sequencing — `run_self_heal_if_plausible` MUST run BEFORE `execute_roll` is called, so `next_seal_seq` is always computed against a reconciled index (EC-023, corrected Invariant 10); this facet is the concrete test obligation that discharges the corrected Invariant 10's by-construction claim. Allocation routed to architect/formal-verifier at Phase F6 targeted-hardening, mirroring BC-1.18.005 EC-017's own VP-owed-to-F6 precedent (S2502-CLUSTER1-PASS5 STATE.md Drift Item) — NOT self-allocated by product-owner in this burst. **VP citation change — routed to architect:** formal-verifier/architect must propagate this extension to VP-INDEX, `verification-architecture.md`, and `verification-coverage-matrix.md` per `vp_index_is_vp_catalog_source_of_truth` (POLICY 9).
- VP-119 — **EXTENDED (cluster-2 LOCAL adversary pass-4, F-C2-P4-002, Postcondition 8)** with a NEW sealed-shard write-once/immutability facet: no `[[shard]]`-eligible destination path is ever overwritten by a subsequent seal-publish attempt; a `seq` collision surfaces as `HookResult::Error`/`E-SHD-009`, never a silent overwrite (EC-024). **VP citation change — routed to architect:** formal-verifier/architect must propagate this extension to VP-INDEX, `verification-architecture.md`, and `verification-coverage-matrix.md` per `vp_index_is_vp_catalog_source_of_truth` (POLICY 9).

## Traceability

| Field | Value |
|-------|-------|
| L2 Capability | CAP-043 |
| Capability Anchor Justification | CAP-043 ("Artifact Sharding Layer 2: Size-Triggered Shard Rotation for Cycle Append-Logs and BC-INDEX Structured-Catalog Sharding") per capabilities.md §CAP-043 — this BC specifies CAP-043's roll-before-write mechanics: "performs a roll-before-write (publish a sealed shard copy as a new file, then atomically replace the canonical file's content with empty, then atomically publish the updated shard index) and returns `HookResult::Block` with an explicit, actionable retry instruction (transparent write-redirection is not implementable under `HookResult`'s ... contract)." (CORRECTED, fix-burst pass-2, F-P2-003, from the withdrawn "seal the current shard by rename, create a fresh empty current file" wording). |
| L2 Domain Invariants | none (dispatcher runtime architectural invariant, not an L2 domain-spec DI-NNN) |
| Architecture Module | SS-01 (Hook Dispatcher Core — `shard_manager.rs` roll/block sequence) |
| ADR | ADR-051 §Decision 1 (block-and-retry mechanism, full algorithm, v1.2 per-tool `projected_size` correction; PreToolUse-only scope for the core roll — Postconditions 1-6 remain governed here); §Decision 3 (stable-current-filename addressing, v1.2 copy-then-atomic-truncate correction); §Decision 4 (shard-index schema, extended v1.4 with `sealed_retroactively`); §Decision 11 (staged partial-failure sequence + E-SHD-006/007, fix-burst addition); §Decision 12 (append-only-tail assumption + sealed-shard escape hatch, fix-burst addition); **§Decision 15 (NEW, ADR-051 v1.8→v1.9, architect addendum) — governs Postcondition 7's catch point (i): the PostToolUse-side native check leg, which §Decision 1 did NOT originally scope (§Decision 1 is PreToolUse-only and had no retroactive-roll semantics); §Decision 15 documents that this leg performs no `HookResult` signaling (a pure side-effect check per EC-016) and specifies its reuse of the retroactive four-step roll**; **§Decision 17 (NEW, PR #824 cycle-3 MAJOR-3, architect addendum) — governs the REACHABILITY of Postconditions 1-6's PreToolUse roll/block sequence: the upstream BC-1.18.005 `shard_cap_precheck` gate this BC's roll executes on behalf of is now invoked from `main::run`, before the `sync_tiers.is_empty() && partition.async_group.is_empty()` early-return guard, rather than from inside `executor.rs::execute_tiers` — closing the prior gap where `main::run` skipped `execute_tiers` (and therefore this BC's own roll/block outcome) entirely whenever no registry-driven plugin matched. §Decision 17 does not alter this BC's own roll mechanism, block-message templates, or partial-failure/self-heal behavior (Postconditions 1-8 unchanged) — it is a reachability/placement correction upstream of this BC's own trigger, mirroring §Decision 1's existing placement rule**; §Context (`HookResult`'s three-variant SDK constraint). Catch point (ii) (the next-dispatch backstop probe) remains governed by §Decision 1's existing PreToolUse scope, since it runs as a leading step within that same existing handling path. |
| Stories | S-25.02 |
| Cycle | v1.0-brownfield-backfill (F2 — product-owner spec-evolution burst) |
| Feature | E-25 — Validation Integrity and Large-Artifact Resilience |
| Deferred-Gap Closure | v1.4 CLOSES the `replace_all: true` occurrence-multiplicity gap BC-1.18.005 v1.11 Postcondition 3 explicitly DEFERRED to this BC's own cluster-2 F2 spec-evolution burst (S2502-CLUSTER1-PASS5 STATE.md Drift Item). |

## Changelog

| Version | Date | Author | Change |
|---------|------|--------|--------|
| 1.12 | 2026-09-08 | product-owner | Citation refresh only (PR #824 cycle-3 MAJOR-3 gate-hoist, ADR-051 §Decision 17; architect adjudication, human-directed). **No postcondition/invariant/EC/CTV content change.** The architect's MAJOR-3 fix (feature/S-25.02-roll tip `6d3d96e7`) hoisted the UPSTREAM BC-1.18.005 `shard_cap_precheck` gate's invocation from inside `crates/factory-dispatcher/src/executor.rs::execute_tiers` to `crates/factory-dispatcher/src/main.rs::main::run`, called before the `sync_tiers.is_empty() && partition.async_group.is_empty()` early-return guard — closing the gap where `main::run` skipped `execute_tiers` (and therefore this BC's own PreToolUse roll/block outcome, Postconditions 1-6) entirely whenever no plugin matched. UPDATED the Traceability ADR row to cite ADR-051 §Decision 17 (NEW, architect addendum) alongside the existing §Decision 1/3/4/11/12/15/§Context citations, documenting that §Decision 17 governs REACHABILITY of this BC's roll/block sequence (an upstream placement/reachability correction) and does not alter this BC's own roll mechanism, block-message templates, or partial-failure/self-heal behavior. No code change, no test change, no story-body propagation required beyond this citation (S-25.02's existing BC table/AC citations for this BC remain accurate as-is). `status` remains `draft`. |
| 1.11 | 2026-09-08 | product-owner | Spec-text-only correction resolving three findings from S-25.02 cluster-2 LOCAL adversary pass-10 (F-C2-P10-001 MINOR, F-C2-P10-002 ADVISORY, F-C2-P10-004 ADVISORY; human-authorized CONVERGE-TO-PR path, asymptotic acceptance, per D-386 Option C). **F-C2-P10-001 (MINOR) — E-SHD-001 taxonomy drift + exhaustive E-SHD sweep.** `error-taxonomy.md`'s `E-SHD-001` Message Format cell drifted from the shipped `ShardRollError::SealWriteFailed` `Display` (missing the `E-SHD-001:` prefix, the `artifact_stem "..."` quoting, and the "canonical file left in its exact pre-roll state, still over cap" clause) — corrected in `error-taxonomy.md` v1.7→v1.8 (see its own changelog). Performed the EXHAUSTIVE E-SHD-001..009 sweep this cluster had been doing one-at-a-time (pass-6/7/8/9): every `ShardRollError` variant's `#[error(...)]` Display in `shard_manager.rs` was read and diffed against the taxonomy table; `E-SHD-001` was the sole remaining drift, now closed; `E-SHD-006`/`E-SHD-007`/`E-SHD-008`/`E-SHD-009` all already match verbatim. `E-SHD-002`..`E-SHD-005` are not `ShardRollError` variants (owned by BC-1.18.007/008/009/011's own error surfaces, not yet implemented in `shard_manager.rs`) — out of this file's sweep scope, no claim made. No change to this BC's own body from F-C2-P10-001 beyond this changelog entry — the drift lived entirely in `error-taxonomy.md`. **F-C2-P10-004 (ADVISORY) — Postcondition 7 catch point (ii) E-SHD-008 gloss reconciled/annotated, not deferred again.** The pass-9 changelog (v1.10) explicitly flagged the abbreviated `E-SHD-008` gloss ("backstop probe stat failure") in Postcondition 7 catch point (ii), EC-019, and the matching Canonical Test Vector as "flagged for a future pass' consideration, not enacted" — a defer-pattern smell under CLAUDE.md Rule 1/6, answerable in scope. RESOLVED with a SPLIT treatment: Postcondition 7's narrative text and EC-019's table row now carry an explicit **"(mechanism label, not literal `Display` text; see `error-taxonomy.md`'s `E-SHD-008` row for the exact emitted message)"** annotation, matching the accepted E-SHD-006/E-SHD-007 mechanism-label precedent (pass-6/8 changelog); the Canonical Test Vector row (which asserts a specific expected message, not merely narrative prose) is instead RECONCILED to the verbatim emitted `E-SHD-008` text (with the row's prior "backstop probe stat failure for decision-log: ELOOP" gloss called out as superseded), since a CTV asserting non-literal text would mislead test-writer. **F-C2-P10-002 (ADVISORY) — EC-025/Postcondition 8 0-byte-reclaim race wording narrowed to be truthful.** Postcondition 8's 0-byte-destination-exception text claimed a concurrent writer racing the `stat()`→`unlink()`→retry `write_exclusive` reclaim sequence "between the `stat()` and the retry" uniformly "fails loud with `E-SHD-009`" — this OVERCLAIMED: the sequence has TWO distinct sub-windows with DIFFERENT outcomes. Content landing in the `unlink()`-to-retry sub-window correctly fails loud (`E-SHD-009`, content preserved) — the only sub-window the withdrawn claim was accurate for. Content landing in the `stat()`-to-`unlink()` sub-window is SILENTLY DELETED by the gate's own unconditional `unlink()` (which does not re-verify size), and the retry then SUCCEEDS with no `E-SHD-009` — a genuine, previously-mischaracterized residual race, not merely inaccurate wording. CORRECTED Postcondition 8's prose to describe both sub-windows precisely; NARROWED the existing EC-025 race-variant Canonical Test Vector to specify it covers the `unlink()`-to-retry sub-window only; ADDED a new Canonical Test Vector documenting the `stat()`-to-`unlink()` sub-window's accepted-residual behavior (silent deletion, no error) — documentary, not a required test in this burst; CORRECTED EC-025's own edge-case table row to name both sub-windows. Also fixed an unrelated PRE-EXISTING defect discovered while editing this row: the EC-025 race-variant Canonical Test Vector row was missing its trailing Category cell (`error`) — added (TD-VSDD-059-adjacent mechanical table-integrity fix, caught by `validate-table-cell-count`, fixed in-scope per CLAUDE.md Rule 4 rather than filed separately). **Full hardening of the `stat()`-to-`unlink()` sub-window (an `O_EXCL`-based re-create-then-swap reclaim primitive that structurally cannot delete non-empty content) PLUS its dedicated concurrent-race fault-injection test are explicitly OWED to Phase F6 targeted-hardening (architect/formal-verifier), attached to the OWED §4.4 F6-owed list** — human-authorized deferral on the converge-to-PR path per CLAUDE.md Rule 3 (explicit human direction: this burst's converge-to-PR authorization; concrete future dependency: the `O_EXCL` re-create-then-swap primitive; anchor: Phase F6 targeted-hardening), NOT self-enacted in this burst; under the current per-dispatch single-process execution model this race is accepted as near-impossible residual risk, not silently unacknowledged. **No story propagation required** — these are message-format/wording-precision/gloss-annotation corrections only, touching no Postcondition/Invariant/EC *scope*, no `bcs:` frontmatter array, and no story AC boundary; S-25.02's existing AC-006 (Postcondition 8) and AC-025 (Postcondition 7/EC-019) citations remain accurate as-is. No change to Postconditions 1-6, Precondition, Invariants, or any Edge Case/Canonical Test Vector beyond those named above. `status` remains `draft`. **Input-hash recompute owed to state-manager** for both this file and `error-taxonomy.md` (both changed in this burst, cross-referencing each other's new content). |
| 1.10 | 2026-09-08 | product-owner | Spec-text-only correction (S-25.02 cluster-2 LOCAL adversary pass-9, F-C2-P9-002, MINOR): Postcondition 8's quoted `E-SHD-009` message ("sealed-shard immutability violation: refusing to overwrite an existing seal at `<path>`") did NOT match the actual emitted `Display` text — the shipped `ShardRollError::SealedShardAlreadyExists` (`crates/factory-dispatcher/src/shard_manager.rs`) actually emits `E-SHD-009: refusing to overwrite an already-sealed shard at '{sealed_path}' for artifact_stem "{artifact_stem}" — sealed shards are write-once/immutable; this seq already has durable content on disk`. **JUDGMENT: this quote is DESCRIPTIVE, not a pinned hard contract** — unlike Postcondition 2's Empty-canonical retry template (F-C2-P4-003, v1.8), which was explicitly ADJUDICATED and pinned as "the exact expected message text," Postcondition 8's quote carries no such pinning language; it illustrates the postcondition's behavior inline, consistent with how error-taxonomy.md v1.6's F-C2-P8-003 precedent treated `E-SHD-006`/`E-SHD-007`'s message wording as descriptive. Reconciling BOTH artifacts (rather than flagging for human authorization) is therefore the correct treatment, matching the F-C2-P8-003 precedent's spirit. CORRECTED Postcondition 8's quoted `E-SHD-009` text (this paragraph) and EC-024's matching Canonical Test Vector row to reproduce the actual emitted text verbatim (with the EC-024 row's concrete example values substituted for `{artifact_stem}`/`{sealed_path}`). error-taxonomy.md's `E-SHD-008`/`E-SHD-009` Message Format cells are corrected in the same burst (v1.6→v1.7) — see its changelog, which also retracts its own v1.6 false attestation that all rows already matched their emitted text. Postcondition 7 catch point (ii)'s abbreviated `E-SHD-008` gloss ("backstop probe stat failure") and EC-019's matching CTV gloss are UNCHANGED — they are short mechanism labels, not literal quoted `Display` text, and F-C2-P9-002 did not scope them; flagged for a future pass' consideration, not enacted here. No change to Postconditions 1-7, Invariants, or any other Edge Case/Canonical Test Vector beyond EC-024's quote correction named above. `status` remains `draft`. |
| 1.9 | 2026-09-08 | product-owner | Fix-burst amendment resolving three findings from S-25.02 cluster-2 LOCAL adversary pass-8 (F-C2-P8-001 MEDIUM, F-C2-P8-002 MEDIUM, F-C2-P8-004 MINOR; the fourth pass-8 finding, F-C2-P8-003, is error-taxonomy-only and closed in `prd-supplements/error-taxonomy.md` v1.6, not here). **F-C2-P8-001 (MEDIUM, spec-internal contradiction) — reconciled stale seal-mechanism prose with the already-shipped exclusive-create.** Postcondition 1 step (b), the Description's CORRECTED note, Invariant 2, and the E-SHD-006 recovery text's "no-op if reissued" claim all still described step (b)'s seal-publish as reusing `write_atomic`'s rename-based create, "no new atomic-write primitive introduced." That was accurate only through v1.7; Postcondition 8 (v1.8) already sanctions — and the shipped `publish_sealed_shard` → `write_exclusive` (a `std::fs::hard_link`-based no-clobber exclusive-create) already implements — a genuinely NEW, no-clobber primitive for step (b) specifically, because a plain `rename(2)` unconditionally overwrites and structurally cannot express "refuse to overwrite if destination exists." REWROTE all four stale sites to describe step (b) as `write_exclusive` (never `write_atomic`), scoped the "no new atomic-write primitive" claim to steps (c)/(d) only (genuinely rename-based, unchanged), and corrected the E-SHD-006 recovery text's idempotency argument (idempotency comes from step (b) never being reissued during recovery, NOT from step (b) being reissue-safe — which is no longer true post-Postcondition-8). ADDED an SDK Grounding Evidence entry confirming `write_exclusive`/`std::fs::hard_link`'s existence. This is propagating an already-ratified newer clause (Postcondition 8) to its stale siblings — not a code-driven spec amendment; the shipped code was already correct. No code change follows. **F-C2-P8-002 (MEDIUM, permanent-deadlock interaction) — defined the 0-byte-destination policy.** Invariant 9's self-heal skip-and-warn guard (never indexes a 0-byte sealed-shard candidate) combined with Postcondition 8's original unconditional write-once refusal would deadlock permanently: a 0-byte orphan at an artifact's `next_seal_seq` path is never indexed, so `next_seal_seq` never advances past it, so every future roll for that artifact collides with the SAME 0-byte file forever (`E-SHD-009` on every attempt), with no in-band recovery. RESOLVED: Postcondition 8's write-once refusal now applies ONLY to a NON-EMPTY pre-existing destination; a 0-byte pre-existing destination has no durable sealed history to protect and is safe to reclaim. `publish_sealed_shard` discharges this: on an `AlreadyExists` collision, it `stat()`s the destination exactly once; if 0 bytes, it unlinks the file and retries `write_exclusive` exactly ONCE (never a loop — a second collision on retry still fails loud with `E-SHD-009`, preserving safety against a genuine concurrent race); if non-zero, it fails loud immediately, unchanged from v1.8. ADDED a 0-byte-destination exception to Postcondition 8, a cross-reference in Invariant 9 resolving the interaction, EC-025 (reclaim + race-variant Canonical Test Vectors), and SCOPED EC-024/its Canonical Test Vector to the non-empty case explicitly. EXTENDED VP-119 with the reclaim/bounded-retry facet. **CODE CHANGE ROUTED (→ implementer):** `publish_sealed_shard` gains the bounded, one-retry 0-byte-reclaim path described above. **TEST ROUTED (→ test-writer):** a fixture asserting the 0-byte case reclaims successfully (EC-025), and a race-injection fixture asserting exactly one retry is attempted before failing loud (EC-025 race variant). **F-C2-P8-004 (MINOR, message overclaim) — the empty-canonical Block template no longer asserts "no roll was performed" on the double-fire path.** On the backstop-then-double-fire path (Postcondition 7 catch point (ii) retroactively seals+truncates a pre-existing orphaned over-cap canonical BEFORE this dispatch's own trigger is evaluated, and this SAME dispatch's own over-cap payload then re-triggers Postcondition 1's `Ok(None)` short-circuit against the now-empty canonical), the existing Case B1 wording's "no roll was performed... the shard remains exactly as it was before this call" is FALSE (a roll DID occur; the shard was emptied by it). ADDED a THIRD sanctioned template, Case B2 (`build_empty_roll_retry_block_reason` parameterized with `preceded_by_backstop_roll: true`), which drops the false clauses while keeping the identical, actionable split-payload guidance; RESCOPED Invariant 4 from a two-template to a three-template (A/B1/B2) determinism property, with selection between B1 and B2 a pure function of whether catch point (ii) fired earlier in the SAME dispatch. Case B1's wording is UNCHANGED and remains accurate for the pure empty-canonical case (no roll of any kind during the dispatch). ADDED EC-026 and a matching Canonical Test Vector. EXTENDED VP-120 with the Case B2 facet. **CODE CHANGE ROUTED (→ implementer):** `build_empty_roll_retry_block_reason` gains a fourth parameter, `preceded_by_backstop_roll: bool`, threading through whether catch point (ii) fired during this dispatch (dispatch-local control-flow state already available — no new tracking). **TEST ROUTED (→ test-writer):** a fixture reproducing the double-fire sequence, asserting Case B2's wording is emitted verbatim (never Case B1's, never Case A's). **Stories affected by BC changes (→ story-writer, per `bc_array_changes_propagate_to_body_and_acs`):** S-25.02 AC-006 (Postcondition 1/8/Invariant 2/9 citations — propagate the 0-byte-reclaim mechanism and the corrected step-(b)-mechanism prose; add EC-025) and AC-007 (Postcondition 2/Invariant 4 citations — propagate the Case B1/B2 split; add EC-026) should be reviewed for propagation of these corrections into story body content. No change to Postconditions 3-7, Invariants 1/3/5-8/10, or any Edge Case/Canonical Test Vector predating this entry beyond the additions/scopings named above. `status` remains `draft`. |
| 1.8 | 2026-09-07 | product-owner | Fix-burst amendment resolving four findings from S-25.02 cluster-2 LOCAL adversary pass-4 (F-C2-P4-001 MAJOR, F-C2-P4-002 MINOR, F-C2-P4-003 MINOR, F-C2-P4-004 ADVISORY). **F-C2-P4-001 (MAJOR, data-loss) — Invariant 10 (v1.7, originally cluster-2 LOCAL adversary pass-3 observation O-C2-P3-001) is FALSE; catch point (i) is NOT safe without a self-heal pre-pass.** A fresh-context review found a REACHABLE counterexample: (1) a prospective roll crashes as `E-SHD-006` — `decision-log.0001.md` holds durable content X, canonical still holds X (truncate never ran), the shard-index is EMPTY (step (d) never ran); (2) the next matched dispatch is a `replace_all: true` `Edit` whose already-applied edit changes the canonical from X to X′ (X′ > cap, X′ ≠ X); (3) catch point (i) fires WITHOUT a self-heal pre-pass and calls `execute_roll` directly — `next_seal_seq` derives from the (empty) index alone → `seq=1` → `publish_sealed_shard` OVERWRITES the durably-sealed `decision-log.0001.md` (X) with X′, permanently destroying it. Invariant 10's premises ("re-seals the SAME bytes into a NEW `seq+1` file") were both false: the bytes changed, and the unindexed orphan occupies `seq=1`, not `seq+1`. **REPLACED Invariant 10** with the corrected property: catch point (i) MUST run the self-heal reconciliation pass (`run_self_heal_if_plausible`) BEFORE calling `execute_roll`, matching the PreToolUse Flat arm exactly (this is now a REQUIRED symmetry, not an intentional asymmetry) — closes the root cause. Catch point (ii) is UNAFFECTED (it already runs within the PreToolUse Flat-arm sequence, after self-heal). ADDED cross-reference from Postcondition 1's `E-SHD-006` recovery text and REWROTE Postcondition 7 catch point (i)'s mechanism paragraph to require the self-heal-first ordering. ADDED EC-023 and a matching Canonical Test Vector reproducing the exact counterexample and asserting the prior seal survives byte-identical. **CODE CHANGE ROUTED (→ implementer):** catch point (i) must call `run_self_heal_if_plausible` before `execute_roll`. **TEST ROUTED (→ test-writer):** fault-injection fixture reproducing the `E-SHD-006`-then-content-changing-`replace_all` sequence, asserting the orphaned seal survives untouched and the new content seals to the correctly-advanced `seq`. **F-C2-P4-002 (MINOR, defense-in-depth) — sealed-shard write-once immutability.** ADDED NEW Postcondition 8: `publish_sealed_shard` MUST refuse to overwrite an existing destination path, returning `HookResult::Error` (NEW `E-SHD-009`, "sealed-shard immutability violation") rather than silently overwriting — a second, independent layer beneath the F-C2-P4-001 root-cause fix, converting any residual seq-collision (a self-heal bug, an external actor, a future code path) into a loud, fail-safe error. Decided a NEW error code (`E-SHD-009`) is warranted rather than reusing an existing `E-SHD-*` code, since this is a distinct failure semantics (a refused write-once violation) from the existing partial-failure/self-heal codes (`E-SHD-001`/`E-SHD-006`/`E-SHD-007`/`E-SHD-008`), none of which describe "attempted overwrite of an already-sealed, immutable destination." ADDED EC-024 and a matching Canonical Test Vector. **CODE CHANGE ROUTED (→ implementer):** `publish_sealed_shard` gains an existence-check-before-rename guard (an atomic exclusive-create primitive preferred, to avoid a TOCTOU race). **TEST ROUTED (→ test-writer):** fixture pre-creating a file at the destination `seq` path, asserting `E-SHD-009` and byte-identical preservation of the pre-existing file. **F-C2-P4-003 (MINOR) — EC-021 CTV vs. implementation divergence, `(<N> bytes)` in the empty-canonical Block message.** ADJUDICATED Option (b) (thread the payload length through) over Option (a) (drop `<N>` from the spec): `<N>` is genuinely useful, actionable UX (the agent learns its own payload is the problem without separately computing its byte count) — the spec's existing wording (Postcondition 2's Empty-canonical retry template, EC-021's Canonical Test Vector) is ALREADY correct and is UNCHANGED by this decision; only the shipped `build_empty_roll_retry_block_reason(artifact_stem, shard_cap_bytes)` signature is out of alignment (CLAUDE.md precedence rule 12: spec wins, code is brought into alignment). ADDED an ADJUDICATED callout to Postcondition 2 pinning this decision and the exact expected message text. **CODE CHANGE ROUTED (→ implementer):** add a third parameter, `payload_len_bytes: usize`, to `build_empty_roll_retry_block_reason`, reusing the length value BC-1.18.005's own trigger already computed — no new computation. **TEST ROUTED (→ test-writer):** pin the exact expected message text, table-driven over a `Write` case and an `Edit{replace_all: true}` case, asserting `<N>` reflects each call's own actual payload length. **F-C2-P4-004 (ADVISORY) — `execute_roll` step (a)'s UTF-8-fallible read is inconsistent with the byte-safe self-heal paths.** ADJUDICATED: byte-level read/write is REQUIRED for consistency (recommended option, adopted) — a non-UTF-8 byte in the canonical file must not permanently block all future writes via `E-SHD-001` merely because `execute_roll` chose a string-typed read where a byte-typed one would succeed; this BC's own self-heal reconciliation (F-C2-P1-001) already establishes byte-level handling as this artifact class's baseline. ADDED a CORRECTED callout to Postcondition 1 requiring step (a)'s read and step (b)'s seal-publish write to be byte-level (`std::fs::read`/`std::fs::write`), never UTF-8-fallible (`read_to_string`/`&str`). **CODE CHANGE ROUTED (→ implementer):** change `execute_roll`'s canonical-file read and sealed-shard write to byte-level I/O. **TEST ROUTED (→ test-writer):** fixture with a non-UTF-8 byte sequence in a matched artifact, asserting a subsequent over-cap roll succeeds byte-for-byte rather than failing `E-SHD-001` on decode. **Test obligation closing the "by-construction, no test needed" gap:** the corrected Invariant 10 is no longer a bare design-time construction argument — EC-023's Canonical Test Vector and the extended VP-NNN (pending)/VP-119 rows below discharge it with a concrete fault-injection assertion; a future call-order change to catch point (i) MUST re-run this test before removing the self-heal-first requirement. **Stories affected by BC changes (→ story-writer, per `bc_array_changes_propagate_to_body_and_acs`):** S-25.02 AC-006 (Postcondition 1/Invariant 2/3/9 citations — add Postcondition 8, EC-023, EC-024) and AC-024 (Postcondition 7/Invariant 6/7 citations — add the corrected Invariant 10, EC-023) should be reviewed for propagation of these frontmatter-level corrections into story body content; AC-025 (catch point (ii) backstop) is UNAFFECTED (catch point (ii) already runs within the self-heal-protected PreToolUse Flat-arm sequence). **VP citations changed (→ architect, per `vp_index_is_vp_catalog_source_of_truth`, POLICY 9):** the pending VP-NNN row (Postcondition 7 reconciliation) gains an EC-023/corrected-Invariant-10 facet; VP-119 (no-over-cap / stable-current-filename invariant) gains a NEW sealed-shard write-once/immutability facet (Postcondition 8, EC-024). **ADR note (→ architect):** ADR-051 likely needs an addendum (mirroring §Decision 15's precedent) documenting catch point (i)'s corrected self-heal-first sequencing and the new write-once `publish_sealed_shard` contract — not enacted here (out of this BC's and product-owner's modification scope). No change to Postconditions 2-6, or to any Edge Case/Canonical Test Vector predating this entry beyond the additions named above. `status` remains `draft`. |
| 1.7 | 2026-09-07 | product-owner | Fix-burst amendment resolving two findings plus one advisory from S-25.02 cluster-2 LOCAL adversary pass-3 (F-C2-P3-001 MAJOR, F-C2-P3-002 MINOR, O-C2-P3-001 ADVISORY). **F-C2-P3-001 (empty-canonical roll-skip behavior + its distinct Block message, previously undocumented and in tension with Postcondition 1/Postcondition 2/Invariant 4):** the SHIPPED implementation correctly short-circuits `execute_roll` to `Ok(None)` (no sealed shard, no index row) when a size-trigger fires but the canonical file's content is 0 bytes (the caller's own payload alone exceeds the cap against an already-empty shard), returning `Block` via a SEPARATE template (`build_empty_roll_retry_block_reason`) rather than the unified rotate-and-retry template. ADJUDICATED (Option (a), the BC's own recommendation, adopted): this is a SECOND, DISTINCT, sanctioned Block template — desirable UX, since it tells the agent something the unified template cannot (splitting the payload, not merely recomputing against a freshly-emptied shard, is the only viable retry, because there is no history to rotate away from). Brought the spec into line with this correct shipped behavior (CLAUDE.md precedence rule 12): ADDED a new empty-canonical-short-circuit clause to Postcondition 1 (sanctioning the zero-step `Ok(None)` case, scoping the four-step sequence to the non-empty-canonical case only); ADDED the "Empty-canonical retry template" clause to Postcondition 2 (the distinct message text); RE-SCOPED Invariant 4 from "a single, fixed template" to "one fixed template PER OVER-CAP CASE," explicitly sanctioning the two-case (with-content / without-content) structure while preserving the no-tool-divergence guarantee WITHIN each case. ADDED EC-021 and a matching Canonical Test Vector. EXTENDED VP-120 with a new two-template-determinism facet (routed to architect per `vp_index_is_vp_catalog_source_of_truth`, POLICY 9). Spec-only; no code change follows (the shipped behavior is already correct — this closes a documentation gap). **F-C2-P3-002 (bytes_at_seal=0 guard does not bind the self-heal index paths):** `execute_roll` refuses to publish a 0-byte seal (F-C2-P2-006), but `self_heal_reconcile_missing_index_entries` and `self_heal_resume_from_truncate` — which also append `[[shard]]` index rows, from filesystem-discovered candidates rather than from `execute_roll` itself — carried no equivalent guard. ADJUDICATED: YES, "no `[[shard]]` entry may have `bytes_at_seal=0`" IS a real BC invariant, binding ALL paths that append index entries, not only `execute_roll` — a genuine orphan of THIS BC's own crash-recovery paths is, by construction, guaranteed non-empty (per F-C2-P3-001's short-circuit and Postcondition 3's cap guarantee), so a 0-byte candidate self-heal encounters is necessarily an external anomaly outside this BC's write-path guarantees, and indexing it would fabricate an audit-trail entry for content that documents no actual sealed history. ADDED NEW Invariant 9 requiring both self-heal paths to skip (never index) a 0-byte candidate and emit a diagnostic rather than failing the current dispatch. ADDED EC-022 and a matching Canonical Test Vector. EXTENDED VP-138 and VP-139 with a new 0-byte-orphan-guard facet each (routed to architect per POLICY 9). **CODE CHANGE ROUTED (→ implementer):** add the 0-byte guard to both self-heal functions. **TEST ROUTED (→ test-writer):** a fixture asserting the guard skips a 0-byte candidate without failing the dispatch. **O-C2-P3-001 (catch point (i) runs `execute_roll` without a self-heal pre-pass, asymmetric with the PreToolUse Flat arm):** the adversary verified this is BENIGN BY CONSTRUCTION (an `E-SHD-007` orphan leaves the canonical empty, which does not change catch point (i)'s own `stat()`-based determination; an `E-SHD-006` orphan is a byte-duplicate, so a duplicate re-seal wastes a shard but loses no unique content). ADJUDICATED: document as explicit BC text (not code-comment-only), since this is a call-order safety guarantee that a future change could silently break without realizing it depends on both orphan classes' specific recovery semantics. ADDED NEW Invariant 10 documenting the by-construction safety argument for both orphan classes and requiring re-verification before this asymmetry is ever removed. No code change; no new VP (the property is a design-time construction argument already covered by VP-138/VP-139's existing self-heal fault-injection tests, which independently exercise both orphan classes' recovery paths). **Stories affected by BC changes (→ story-writer, per `bc_array_changes_propagate_to_body_and_acs`):** S-25.02 — its BC table and AC traces referencing this BC's Postcondition 1/2/Invariant 4 (empty-canonical short-circuit and template selection) and its self-heal-path ACs (Invariant 9's 0-byte guard) should be reviewed for propagation of these frontmatter-level clarifications into story body content. No change to Postconditions 3-6, or to any Edge Case/Canonical Test Vector predating this entry beyond the additions named above. `status` remains `draft`. |
| 1.6 | 2026-09-07 | product-owner | Fix-burst amendment resolving two MINOR findings from S-25.02 cluster-2 LOCAL adversary pass-2 (F-C2-P2-003, F-C2-P2-004). **F-C2-P2-003 (Write-arm backstop `stat()`-failure disposition, unadjudicated fail-open):** catch point (ii)'s dedicated `Write`-arm probe (added F-C2-P1-002, v1.5) was silently implemented to fail OPEN (log a warning, let the `Write` proceed) on any `stat()` error other than `NotFound`, while `Edit`/`MultiEdit` fail LOUD on the same class of error — an unadjudicated asymmetry this BC was silent on. ADJUDICATED: fail-open is UNSOUND, not merely under-specified — a canonical path whose final path component is a symlink loop confined to itself fails a dereferencing `stat()` (`ELOOP`) while `write_atomic`'s `rename(temp, canonical)` (which does not need to dereference the destination's final symlink component) can still succeed, silently replacing the symlink and applying the `Write`'s under-cap `content` with the true, possibly-over-cap content the symlink pointed at never examined or sealed — a real, specific data-loss path, not a hypothetical one; `EACCES`-class directory-traversal failures ARE symmetric between `stat()` and `write_atomic`, but one confirmed asymmetric errno is sufficient to make blanket fail-open unsound, and this BC declines to enumerate every future filesystem/errno pair as safe. CORRECTED Postcondition 7 catch point (ii) to REQUIRE fail-loud (`HookResult::Error`, NEW `E-SHD-008`, "backstop probe stat failure") on any non-`NotFound` stat() error for the `Write` arm, matching `Edit`/`MultiEdit`'s existing disposition; `NotFound` remains the legitimate first-ever-write case and is unaffected. ADDED NEW Invariant 8 (uniform fail-loud backstop-probe disposition, never tool-divergent), EC-019, and a matching Canonical Test Vector. **CODE CHANGE REQUIRED (→ implementer):** the existing fail-open branch for this specific dedicated backstop probe must be changed to fail-loud for the `Write` arm. **F-C2-P2-004 (Invariant 7 inference exactness vs. `shard_cap_bytes` re-calibration):** Invariant 7's `bytes_at_seal > shard_cap_bytes ⇒ sealed_retroactively = true` inference is exact only while `shard_cap_bytes` is STABLE across a shard's seal-to-reconciliation window; BC-1.18.005 Postcondition 6/AC-004 permits the F4 calibration harness to re-lock the cap when `DEFAULT_FUEL_CAP` changes. ADJUDICATED (Option (a), documented bounded risk, chosen over Option (b) full schema-level re-architecture) under the production-grade lens: ADDED a STABLE-CAP PRECONDITION addendum to Invariant 7 documenting BOTH mislabel directions — a cap LOWERED between seal and reconciliation produces a false positive (a legitimately prospective seal inferred retroactive); a cap RAISED produces a false negative (a genuinely retroactive seal inferred prospective) — the false-negative direction was NOT identified by the adversary's finding text and is added here under CLAUDE.md's "fix in scope when found" default rather than left for a future pass. Both directions are judged operationally negligible and explicitly ACCEPTED because: (1) the mislabeled field is a historical audit-trail annotation only, gating no hard structural guarantee (Invariant 6's canonical zero-bytes guarantee and the sealed-shard content-preservation guarantee are unaffected in either direction); (2) the window requires two independently rare events to coincide (an `E-SHD-007` orphan surviving across a deliberately-gated, phase-locked F4 recalibration boundary before its own reconciliation runs); (3) the bound is precisely testable (`shard_cap_bytes` unchanged across `[sealed_at, reconciliation_time]` ⇒ inference is guaranteed correct). A structurally complete fix (persisting `shard_cap_bytes`-at-seal-time, or gating F4 recalibration on draining all pending `E-SHD-007` orphans first) would require amending BC-1.18.005's already-ACTIVE calibration-harness contract or the shard-index schema itself — out of THIS BC's modification scope per its own established Precondition-4/Postcondition-7 precedent of not amending BC-1.18.005's shipped contract within this BC's own closure bursts — and is flagged as a candidate FUTURE BC-1.18.005 spec-evolution item, not enacted here. ADDED EC-020 and a matching Canonical Test Vector illustrating both mislabel directions. Spec-only; no code change follows from F-C2-P2-004 (the current inference implementation is already correct under the now-explicit stable-cap precondition — this closes a documentation gap, not a code defect). Extended the pending VP-NNN row and VP Anchors entry (EXTENDED cluster-2 LOCAL adversary pass-2, F-C2-P2-003/004) to cover EC-019/EC-020/Invariant 8/Invariant 7's addendum; allocation remains routed to Phase F6 (not self-allocated). No change to Postconditions 1-6, Invariants 1-6, or any Edge Case/Canonical Test Vector predating this entry beyond the additions named above. `status` remains `draft`. |
| 1.5 | 2026-09-07 | product-owner | Fix-burst amendment resolving three MAJOR findings from S-25.02 cluster-2 LOCAL adversary pass-1 (F-C2-P1-001, F-C2-P1-002, F-C2-P1-004). **F-C2-P1-002 (catch point (ii) internal contradiction + real data-loss path):** catch point (ii)'s claim that it reuses "the same `stat()` BC-1.18.005 Postcondition 2 already performs" for EVERY subsequent `Edit`/`Write`/`MultiEdit` was TRUE for `Edit`/`MultiEdit` but WRONG for `Write` (whose Postcondition 3 trigger, `projected_size = len(content)`, performs no `stat()` at all) — left uncorrected, this allowed a subsequent under-cap `Write` to apply directly against a crash-orphaned, un-sealed, over-cap canonical file, destroying that history with no roll ever having started for self-heal to catch. ADJUDICATED under the production-grade default (no version of this gate may lose history): catch point (ii) now performs its OWN dedicated, bounded `stat()` of the canonical file specifically for the `Write` arm (BC-1.18.005's stat-free `Write` trigger formula is UNCHANGED — this is an additional backstop probe, not a change to the trigger); `Edit`/`MultiEdit` remain cost-free (reuse the existing stat). Corrected the contradictory wording in catch point (ii) and in Postcondition 7's "Scope" paragraph (which had over-generalized catch point (i)'s `replace_all`-only scope to imply `Write` pays zero cost anywhere in Postcondition 7). ADDED EC-017 and a matching Canonical Test Vector for the `Write`-next-dispatch backstop case. **F-C2-P1-001 (`sealed_retroactively` mislabeled on a crashed retroactive roll):** if a RETROACTIVE roll (catch point (i)/(ii)) crashes between its own step (c) and step (d), `E-SHD-007`'s self-heal reconciliation (scanning the filesystem for un-indexed sealed shards with no other context) would otherwise hardcode the recovered entry's `sealed_retroactively` to `false`, mislabeling a genuinely over-cap shard as cap-guaranteed. ADDED NEW Invariant 7 specifying the deterministic inference rule (`bytes_at_seal > shard_cap_bytes ⇒ sealed_retroactively = true`, since a prospective roll can never seal over-cap content) and REQUIRING `E-SHD-007` self-heal reconciliation to apply it. Cross-referenced from Postcondition 1's `E-SHD-007` recovery text and Postcondition 5's `sealed_retroactively` field description. ADDED EC-018 and a matching Canonical Test Vector. **F-C2-P1-004 (stale Canonical Test Vector, POLICY 12):** the first CTV row's numeric example (`Write`, current shard 45,000 bytes, `content` length 5,000 bytes, cap 49,152 → `Block`) was a stale carry-over from the WITHDRAWN `current + payload` formula and was inconsistent with the corrected `projected_size = len(content)` Write formula (5,000 <= 49,152 would `Continue`, not `Block`). REPLACED with a self-consistent example (`content` length 50,000 bytes, cap 49,152) that actually triggers the roll under the current formula, clarifying that the sealed shard's size is always the pre-roll canonical content (45,000 bytes), never the blocked `Write`'s own `content`. Extended the pending VP-NNN row and VP Anchors entry to cover the extended scope (EC-017/EC-018/Invariant 7); allocation remains routed to Phase F6 (not self-allocated). No change to Postconditions 1-6, 3-6, or any other Invariant, Edge Case, or Canonical Test Vector beyond those named above. `status` remains `draft`. |
| 1.4 | 2026-09-07 | product-owner | F2 spec-evolution burst (S-25.02 cluster-2 pre-TDD): CLOSES BC-1.18.005 v1.11 Postcondition 3's deferred `replace_all: true` occurrence-multiplicity gap (S2502-CLUSTER1-PASS5 STATE.md Drift Item), adopting Option (b) of BC-1.18.005's own pre-authorized closure fork — BC-1.18.005's PreToolUse trigger formula is left UNCHANGED (Option (a), amending BC-1.18.005's already-ACTIVE/shipped contract, was NOT taken). ADDED Precondition 4 (closure-scope statement, `Edit`/`MultiEdit` `replace_all: true` calls only). ADDED Postcondition 7 (bounded post-write reconciliation): two redundant catch points — (i) an immediate post-write `stat()`-based check reusing Postcondition 1's exact four-step roll sequence retroactively, and (ii) a next-dispatch leading-probe backstop covering an (i)-crash scenario; a precise, testable bounded-window postcondition (canonical file over-cap observable for at most one subsequent matched dispatch, never indefinitely); and a documented, narrowly-scoped exception allowing a RETROACTIVELY-sealed shard's `bytes_at_seal` to exceed `shard_cap_bytes`, while the CANONICAL file's own zero-bytes-after-roll guarantee remains unconditional (codified in NEW Invariant 6). EXTENDED Postcondition 5's shard-index schema with an optional `sealed_retroactively` boolean field (default `false`, backward compatible) as the audit trail distinguishing the two shard-cap guarantee regimes. ADDED EC-014 (under-projected `replace_all` triggers catch point (i)), EC-015 (catch point (i) failure triggers catch point (ii) backstop), EC-016 (no agent-facing block/retry signal for a retroactive roll — unchanged from existing EC-002 scope) and three matching Canonical Test Vectors. ADDED one new Verification Property row, VP-NNN (pending) — allocation explicitly routed to architect/formal-verifier at Phase F6 targeted-hardening (NOT self-allocated), mirroring BC-1.18.005 EC-017's own VP-owed-to-F6 precedent. Architecture Anchors extended with the two new catch-point call sites (both native dispatcher code reusing Postcondition 1's existing mechanism; no new `HookResult` variant, no new atomic-write primitive, no new `hooks-registry.toml` entry). Traceability gained a "Deferred-Gap Closure" row citing the closed drift item. `status` remains `draft` (cluster-2 not yet shipped). **SAME-BURST CORRECTION (architect implementability-gate finding, post-initial-v1.4-draft):** this entry originally claimed "no new ADR decision required — contained within ADR-051's existing native-check pattern" for Postcondition 7. That OVERCLAIMED: ADR-051 §Decision 1 scoped the native check to PreToolUse ONLY, with no PostToolUse leg and no retroactive-roll semantics — catch point (i) (the PostToolUse-side check) was NOT already covered. The architect added **ADR-051 §Decision 15** (ADR-051 v1.8→v1.9) as a new addendum documenting the PostToolUse-side native-check leg, its pure-side-effect (no `HookResult` signaling, per EC-016) contract, and its reuse of the retroactive four-step roll. The Traceability ADR row is corrected accordingly: Postcondition 7 catch point (i) is now cited to §Decision 15; the core roll mechanism (Postconditions 1-6) and catch point (ii) remain under §Decision 1/§Decision 11. No postcondition, invariant, edge-case, or mechanism text changed — citation-accuracy correction only; BC stays at v1.4 (same-burst fix, not a new increment). |
| 1.3 | 2026-09-05 | product-owner | Fix-burst amendment (adversary pass-3 finding F-P3-006 LOW): collapsed the §Verification Properties table's separate VP-118 (×2) and VP-119 (×2) rows into one row each (multi-facet convention). CRITICALLY, REMOVED the second VP-118 "partial-failure self-healing invariant" row entirely — it duplicated and OVER-CLAIMED coverage that VP-INDEX v3.04 authoritatively assigns per-code to VP-120 (`E-SHD-001`), VP-138 (`E-SHD-006`), and VP-139 (`E-SHD-007`) respectively; folded the `E-SHD-001` fail-loud facet explicitly into VP-120's row to match VP Anchors' existing description. No test coverage lost, no postcondition/invariant/edge-case content change — table presentation and mis-attribution correction only. |
| 1.2 | 2026-09-05 | product-owner | Fix-burst amendment (adversary pass-2 findings F-P2-003 HIGH + F-P2-002 HIGH + F-P2-004 MEDIUM + F-P2-005 MEDIUM, ADR-051 v1.2 Decisions 1/3/11/12): (1) REWROTE Postcondition 1(a)/Invariant 2/Invariant 3 from the WITHDRAWN rename-away seal mechanism (which opened a real ENOENT window on the canonical path) to the CORRECTED copy-then-atomic-truncate-in-place mechanism — publish the sealed shard as a new file, then atomically replace the canonical file's content with empty via `write_atomic`'s temp-file-then-rename primitive; the canonical path is never absent. (2) REWROTE Postcondition 2/Invariant 4 to a SINGLE UNIFIED retry-instruction template, withdrawing the divergent `Write`-"simply retry unchanged" wording (unsound: could permanently deadlock a blocked `Write` whose stale pre-roll `content` remains over cap under the corrected `projected_size = len(content)` formula). (3) ADDED the staged 4-step per-write roll sequence (read/publish-sealed/atomic-truncate/publish-index) with two NEW partial-failure error codes `E-SHD-006` (seal published, canonical not yet truncated — self-healing resume-from-truncate) and `E-SHD-007` (canonical truncated, index not yet updated — self-healing index reconciliation), plus new EC-010/EC-011 and a new VP-118 fault-injection property. (4) ADDED Invariant 5 (append-only-tail assumption, explicit) and two new edge cases EC-012 (sealed-shard direct-edit escape hatch for already-relocated content) and EC-013 (still-mutable tail edit, unaffected by Invariant 5). Updated Canonical Test Vectors, Architecture Anchors, SDK Grounding cross-references, VP Anchors, and Traceability's Capability Anchor Justification quote and ADR citation accordingly. Added BC-1.18.012 to Related BCs. |
| 1.1 | 2026-09-05 | product-owner | Fix-burst amendment (F-S2502-F2-007, POLICY 5): added `## SDK Grounding Evidence` section with literal stable-anchor grep output for `HookResult`'s three-variant enum, `write_indeterminate_marker`/`block_if_marker_check`, and `write_atomic`. No postcondition/invariant/VP content change — this BC's contract was confirmed unaffected by the sibling BC-1.18.009 BLOCKER fix (architect: "No change required," F2 architecture-delta §4a). |
| 1.0 | 2026-09-05 | product-owner | Initial creation. F2 spec-evolution burst, S-25.02 activation. Encodes the CORRECTED block-and-retry roll semantics (per ADR-051's finding that `HookResult`'s Continue/Block/Error contract forbids transparent write-redirection) rather than a transparent-redirect fiction; shard-index schema; stable-current-filename addressing as a structural consequence of the seal mechanism. CAP-043 capability anchor. ADR-051 §D1/§D3/§D4 citations. |
