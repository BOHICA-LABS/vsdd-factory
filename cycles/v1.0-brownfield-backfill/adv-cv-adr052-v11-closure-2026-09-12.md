---
document_type: cross-vendor-closure-review
adversary_model: openai-codex
type: cross-vendor-closure-review
streak_impact: NONE
reviewed_at: 2026-09-12
reviewed_version: ADR-052 v1.1
verdict: RATIFY-WITH-CHANGES
ratify: false
note: "BC-5.39.001 cycle streak 3/3 UNCHANGED — cross-vendor closure reviews are NON-STREAK (decision-support only)."
---

# 2nd Cross-Vendor Codex Closure Review: ADR-052 v1.1

**Date:** 2026-09-12
**Adversary model:** openai-codex (decision-support; NON-STREAK)
**Prior review:** `adv-cv-adr052-cluster5-F1-2026-09-12.md` (1st Codex CV, 7 findings, D-1214)
**Reviewed package:** ADR-052 v1.1 + BC-1.18.011 v1.1 + BC-1.18.010 v1.3 + error-taxonomy.md v1.18

## Verdict

**RATIFY-WITH-CHANGES — DO NOT ratify v1.1 as written.**

The v1.1 revision package partially addressed the 1st Codex review's findings but did not achieve ratification quality. The single most important residual risk is that the proposed maintenance and commit-marker protocol does not establish an atomic publication boundary: an interrupted publication (crash after any rename but before COMMITTED is written) leaves an incomplete BC-INDEX.md exposed to governed writes and readers with no recovery path that can distinguish a partial publish from a crash-before-any-publish.

## Closure Status vs Prior 7 Findings (1st Codex CV)

| Finding | Closure Status | Notes |
|---------|---------------|-------|
| F1 — allowlist unsafe (leading/trailing wildcard) | **PARTIAL** | Explicit exception declared in CLAUDE.md; controls remain mischaracterized or omitted — executable trust and shell-boundary enforcement unspecified |
| F2 — insufficient guard coverage (14 `^Bash$` PreToolUse guards) | **PARTIAL** | Option B confirmed + no standing settings entry; executable trust and shell-boundary enforcement still unspecified |
| F3 — manifest authorization incomplete | **PARTIAL** | Manifest exists conceptually; required authorization comparisons and lifecycle rules are incomplete |
| F4 — hook-driven alternative evaluated and rejected | **CLOSED** | ADR-052 lines 379–412 explicitly acknowledge hook-driven viability and reject it for operational complexity; supported by main.rs:417 and executor.rs:408–413 |
| F5 — crash atomicity / publication boundary | **OPEN** | dir-fsync + fingerprints improve durability but writer exclusion and interrupted publication remain unsafe |
| F6 — A/B2 coupling | **CLOSED** | BC-1.18.011 lines 63–67 and 145–154 remove A/B2 coupling while preserving B2-internal atomicity; lines 156–162 and 195–198 correctly distinguish Cohort-B independence |
| F7 — config parity / live vs approved comparison | **PARTIAL** | Live-versus-approved ARCH-INDEX comparison is required; loaded-config-versus-approved comparison is missing |

## New Findings (8 total: 7 HIGH + 1 MED)

### Finding 1 — Place maintenance enforcement before every governed mutation and drain admitted writers
**Severity:** HIGH | **Category:** concurrency | **Confidence:** HIGH
**Location:** BC-1.18.011:79–86; plugins/vsdd-factory/hooks-registry.toml:1023–1029

**Evidence:** BC Precondition 6 says "Edit/Write tool calls validated by the validate-factory-path-staging dispatcher guard" check the lock. The actual registry declares that guard with `tool = "^Bash$"`. error-taxonomy.md:84 repeats the erroneous Edit/Write enforcement claim. Moreover, main.rs:417 calls `shard_cap_precheck` before registry execution; executor.rs:367 accepts Edit/Write/MultiEdit and :408 invokes `shard_cap_gate_check`, which reaches `execute_roll` at shard_manager.rs:2034. A registry-level check is therefore too late for native hook mutations. Even a corrected presence check allows a writer admitted before lock creation to write after the migration's fingerprint recheck. Searches across the three dispatcher files found no `exclusive.lock` or migration-state enforcement.

**Recommendation:** Specify a native maintenance admission gate before all hook side effects, covering Edit/Write/MultiEdit, native rolls/reconciliation, both migrations' targets, and recovery. Require a protocol that drains previously admitted writes and retains exclusion through their actual completion; a PreToolUse existence check alone is insufficient. Add interleaving tests for admission-before-lock, native mutation-before-plugin, and mutation-after-fingerprint-check.

### Finding 2 — Define a real atomic publication and partial-rename recovery protocol
**Severity:** HIGH | **Category:** concurrency | **Confidence:** HIGH
**Location:** ADR-052:245–262

**Evidence:** ADR says COMMITTED is written "AFTER the last rename" and PREPARED recovery must "resume directly to the atomic-replace step". BC-1.18.011:190–191 nevertheless asserts that before COMMITTED the "original body [is] untouched". A crash after replacing BC-INDEX but before writing COMMITTED directly contradicts this assertion; the source fingerprint will now differ from the original and trigger the prescribed abort. A crash earlier in the rename loop may also leave staging files consumed by rename. No per-target recovery procedure is specified. BC-1.18.010:75–79 directs readers straight to canonical shard paths without consulting a commit pointer.

**Recommendation:** Specify immutable staged generations with one atomically published authoritative pointer and reader integration, or a complete journaled publication protocol with enforced reader/writer quiescence. Define recovery after every target rename and marker write, preserve recoverable staging, verify target hashes, and treat CLEANED as terminal success. Extend fault injection beyond staging to every publication and durability boundary.

### Finding 3 — Specify crash-safe lock ownership and recovery admission
**Severity:** HIGH | **Category:** concurrency | **Confidence:** HIGH
**Location:** BC-1.18.011:69–86; error-taxonomy.md:84

**Evidence:** The BC calls `exclusive.lock` an "exclusive advisory lock file", while the taxonomy blocks whenever "the lock file exists". Release is specified only on COMMITTED or abort. A process crash can release an OS advisory lock while leaving its file present; alternatively, treating file presence as ownership leaves permanent maintenance after a crash. PREPARED is created only after staging, so a crash between lock acquisition and PREPARED has no defined ownership/recovery state. ADR:254–256 provides only PREPARED and COMMITTED restart branches.

**Recommendation:** Separate persistent maintenance intent from OS lock ownership. Specify atomic acquisition, stable lock-file identity, owner-independent crash recovery, and admission rules for missing/corrupt markers and pre-PREPARED crashes. Retain maintenance after interrupted publication until an exclusive recovery process restores consistency; never infer active ownership solely from file existence.

### Finding 4 — Validate all approval bindings under exclusion and define expiry-safe recovery
**Severity:** HIGH | **Category:** security | **Confidence:** HIGH
**Location:** ADR-052:151–179

**Evidence:** The manifest requires `repo_root_sha` and `expected_total_bcs` at lines 156–160, but the exhaustive pre-mutation checks at 165–173 compare only existence, migration_id, time, ARCH-INDEX SHA and readiness. No comparison to approved repository HEAD or approved `total_bcs` is required. Line 163 says the migration "must complete within 24 hours", but time is checked only before mutation. Lines 175–179 delegate manifest deletion to state-manager and simultaneously say absent manifests are rejected and already-migrated reruns exit 0. Validation is required "before ANY filesystem mutation", placing it before acquisition of the newly created lock unless explicitly reordered.

**Recommendation:** Under exclusion, validate canonical repository/worktree identity, approved input hashes/revision, `expected_total_bcs`, migration identity, readiness, and a bounded timestamp. Define permitted worktree changes caused by manifest creation. Bind checkpoints to a unique activation ID and input/config hashes. Recheck authorization before publication, atomically record consumption, and distinguish fresh activation from authorized crash recovery after expiry and read-only idempotent inspection.

### Finding 5 — Compare the loaded snapshot revision with the approved live revision
**Severity:** HIGH | **Category:** consistency | **Confidence:** HIGH
**Location:** ADR-052:347–368

**Evidence:** Lines 349–350 require `config.arch_index_sha`, but lines 354–359 only compare live ARCH-INDEX with `manifest.approved_arch_index_sha`, declaring this "the sufficient condition". Lines 360–363 move the embedded snapshot check into a test against a "known-valid reference". BC-1.18.010:150–160 repeats the same two-way comparison. A stale installed config at revision A still passes when live ARCH-INDEX and the manifest both identify revision B.

**Recommendation:** Require activation-time equality of `loaded config.arch_index_sha`, `manifest.approved_arch_index_sha`, and the authoritative live ARCH-INDEX revision, with mapping content tied to that revision. Test the actual activation path using an old deployed snapshot and a mutually matching new live revision and manifest; it must fail before staging.

### Finding 6 — Replace the claimed control equivalence with a complete verified inventory
**Severity:** HIGH | **Category:** purity-boundary | **Confidence:** HIGH
**Location:** ADR-052:280–288

**Evidence:** The table describes `brownfield-discipline` as validating "existing file shapes, incremental changes" and substitutes source hashing. Its actual script, lines 4–9 and 34–38, protects `.reference/` from writes. The table omits `factory-branch-guard`, registered for Edit/Write/MultiEdit at hooks-registry.toml:815 onward; its script:67–82 checks the worktree and factory-artifacts branch before mutation. The table also says `validate-factory-path-staged` is "bypassed … by the one-time-interactive invocation path", but the registry:1454–1456 explicitly registers it for PostToolUse Bash. Interactive approval does not remove that event.

**Recommendation:** Inventory every skipped hook and native check against the actual migration target set. Record accurate applicability, equivalent pre-mutation enforcement, or a specific ratified waiver for each. Include worktree/branch and path validation. Correct the Bash PostToolUse model and retain applicable hooks rather than treating interactive approval as a waiver implementation.

### Finding 7 — Enforce exact shell invocation and executable provenance before granting guard exceptions
**Severity:** HIGH | **Category:** security | **Confidence:** HIGH
**Location:** ADR-052:124–139

**Evidence:** The ADR pins `target/release/factory-dispatcher` and says shell metacharacters and compound commands are "REJECTED by the binary". A binary receives argv after shell parsing; it cannot reject a trailing shell command it never receives. Decision 5:200–203 instead describes a migration "command pattern" exception validated against manifest "presence". The ADR supplies no pre-execution executable digest/provenance verification or canonical target-containment rule; line 137 makes targets config-driven. `.claude/settings.json` contains only `enabledPlugins`, so the standing-allowlist defect is closed, but these residual boundaries are not.

**Recommendation:** Require all three guard exceptions to share an exact full-command classifier that rejects wrappers, substitutions, redirections, chains, extra arguments and alternate executable paths before shell execution. Validate the manifest rather than its presence. Bind approval to verified executable provenance/digest and canonical repository identity; validate config-derived targets and symlink containment. Add negative tests demonstrating that unrelated destructive commands never inherit the exception.

### Finding 8 — Make the proposed policy exception cover its necessary writes and sequence
**Severity:** MED | **Category:** consistency | **Confidence:** HIGH
**Location:** ADR-052:303–323

**Evidence:** The replacement permits writes ONLY to `.factory/specs/behavioral-contracts/shards/` and `.factory/cycles/*/`. B2 must also replace BC-INDEX.md itself (BC-1.18.011:117), and the binary creates `.factory/migration-state/` (ADR:672), neither of which is allowed. It requires ALL conditions to hold "simultaneously", including an audit commit recording COMMITTED, although Decision 6:227–233 creates that commit only after migration. The specified Forbidden patterns target row also does not contain the quoted current text; the actual Edit/Write-only rule is CLAUDE.md:231.

**Recommendation:** Amend the actual governing rule, enumerate BC-INDEX.md plus narrowly bounded state/lock/staging paths and the four mechanism-A targets, and distinguish preconditions from mandatory post-success audit obligations. Keep every other mutation outside the exception. Make application of the corrected policy text an explicit ratification deliverable.

## Redesign Mandate (8-Finding Summary for v1.2 Architect Pass)

On resume, the architect ADR-052 v1.2 redesign must address all 8 findings above:
1. **[HIGH]** Native maintenance-admission gate must precede ALL side-effects including native rolls via `executor.rs` `shard_cap_precheck`→`execute_roll` + drain admitted writers; lock mis-layered: guard is `^Bash$` but lock specced on Edit/Write.
2. **[HIGH]** Real atomic publication + partial-rename recovery — immutable staged generations + single published pointer + reader integration OR journaled protocol + quiescence; "COMMITTED-after-last-rename" contradicts "original untouched"; readers don't consult a commit pointer.
3. **[HIGH]** Crash-safe lock ownership + recovery (separate maintenance-intent from OS-lock; owner-independent recovery; pre-PREPARED crash undefined).
4. **[HIGH]** Full manifest validation UNDER exclusion + expiry-safe recovery (compare approved repo HEAD + `total_bcs`; recheck before publication; atomic consumption; distinguish fresh vs crash-recovery-after-expiry).
5. **[HIGH]** Three-way config parity (`loaded config.arch_index_sha` == `manifest.approved` == live ARCH-INDEX; stale installed config passes today).
6. **[HIGH]** Accurate skipped-control inventory (`brownfield-discipline` protects `.reference/` not "file shapes"; ADD `factory-branch-guard`; `validate-factory-path-staged` PostToolUse-Bash NOT bypassed by interactive approval).
7. **[HIGH]** Exact full-command classifier in the guard pre-shell + executable provenance/digest ("binary rejects shell metacharacters" is impossible — argv arrives post-shell-parse).
8. **[MED]** Exception must cover BC-INDEX.md itself + `.factory/migration-state/` dir + 4 mech-A targets; separate preconditions from post-success audit obligations; fix the CLAUDE.md:231 anchor.

## Post-Redesign Path

After v1.2 redesign:
1. product-owner re-hardens BC-1.18.011 + BC-1.18.010
2. state-manager commits the revised package
3. 3rd cross-vendor Codex re-review (NON-STREAK; decision-support)
4. Human POLICY 22 ratification

Cluster-5 TDD remains BLOCKED until human POLICY 22 ratification of a clean Codex re-review.

## Decision Reference

D-1216 (STATE.md Decisions Log): ADR-052 v1.1 NOT POLICY-22-ratifiable per this 2nd Codex closure verdict. Human authorized redesign cycle. Architect v1.2 redesign dispatched this session but ABANDONED mid-file-reading phase at session wrap (wrote NOTHING to disk; ADR-052 remains at v1.1 committed @ 18a748f7 clean). BC-5.39.001 cycle streak 3/3 UNCHANGED.
