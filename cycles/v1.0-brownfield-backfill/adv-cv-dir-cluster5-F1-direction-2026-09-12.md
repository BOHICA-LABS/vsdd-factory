---
document_type: cross-vendor-direction-review
level: ops
version: "1.0"
status: complete
producer: adversary
adversary_model: openai-codex (CROSS-VENDOR — OpenAI Codex)
review_type: DIRECTION
review_scope: "cluster-5 F1 kickoff — compaction-deferral plan + S-25.06 v1.0 + S-25.02 sequencing + cluster-5 direction"
timestamp: 2026-09-12T00:00:00Z
phase: F1
review_date: 2026-09-12
verdict: "DIRECTION CONDITIONALLY CORRECT — cluster-5 F1 sensible; DO NOT proceed to activation under current deferral; S-25.06 dep cycle FIXED (HIGH-1); HIGH-2 + MED-3 + MED-4 open (see findings)"
novelty: HIGH
finding_count: 4
finding_breakdown: "HIGH-1 dep cycle [FIXED this burst via S-25.06 v1.1]; HIGH-2 impossible execution path [OPEN cluster-5 F1 architect]; MED-3 unbounded deferral [Drift Item tightened this burst]; MED-4 hook cwd path [S-12.14]"
streak_impact: NONE
# CRITICAL: This is a CROSS-VENDOR DIRECTION review, NOT a BC-5.39.001 cascade pass.
# It reviews the ORCHESTRATION DECISION (proceed to cluster-5 F1?) and the
# compaction-deferral plan (S-25.06 v1.0 + S-25.02 sequencing).
# It does NOT advance or reset the cycle-level BC-5.39.001 convergence streak.
# BC-5.39.001 streak stays 3/3 UNCHANGED.
# Finding IDs use CV-DIR prefix to distinguish from BC-5.39.001 cascade pass IDs.
inputs_reviewed:
  - .factory/STATE.md
  - .factory/stories/S-25.06-append-log-backfill-split-executor.md
  - ".factory/stories/S-25.02-artifact-sharding-layer2.md (targeted sections)"
  - ".factory/stories/STORY-INDEX.md (S-25.06 registration)"
  - ".factory/specs/behavioral-contracts/ss-01/BC-1.18.010.md (frontmatter)"
  - ".factory/specs/behavioral-contracts/ss-01/BC-1.18.011.md (preconditions and migration requirements)"
  - CLAUDE.md
  - plugins/vsdd-factory/hooks-registry.toml
  - "crates/last-amended-migrate/src/migrate.rs (target list and orchestration)"
  - "crates/factory-dispatcher/src/shard_manager.rs (backfill entry point)"
  - "crates/ (backfill caller and fuel-cap searches)"
  - "crates/factory-dispatcher/src/executor.rs (indeterminate marker handling)"
  - "crates/factory-dispatcher/src/registry.rs (failure-policy defaults)"
  - plugins/vsdd-factory/hooks/postcompact-reanchor.sh
  - "plugins/vsdd-factory/hooks/update-cargo-audit-cache.sh (path search)"
  - ".factory/cycles/v1.0-brownfield-backfill/ (four append-log size measurements)"
  - "factory-artifacts commits b4da8480, 71b7ddb8, dd934e99 (log/show)"
  - "dd934e99 archive sidecars (headers and initial content)"
  - .factory/ working-tree status
verified_claims:
  - "migrate TARGET_FILES = 4 indexes + STATE.md only (confirmed)"
  - "run_mechanism_a_backfill_split has zero production callers (confirmed)"
source_file: "/tmp/codex-review-direction-1789227407/codex-review.json"
---

# adv-cv-dir-cluster5-F1-direction-2026-09-12 — Cross-Vendor Direction Review (OpenAI Codex)

> **CROSS-VENDOR DIRECTION review of the cluster-5 F1 kickoff decisions.** This is NOT a
> BC-5.39.001 cascade adversarial pass and DOES NOT affect the convergence streak (stays 3/3
> UNCHANGED). OpenAI Codex reviewed the compaction-deferral plan, S-25.06 v1.0 sequencing,
> and the overall direction decision (proceed to cluster-5 F1 spec) from the raw JSON at
> `/tmp/codex-review-direction-1789227407/codex-review.json`, rendered in VSDD Part-A format
> matching the cluster-3 pass-6 cross-vendor convention
> (`s2502-cluster3-local-adversary-pass-6.md`). Finding IDs use CV-DIR prefix.
>
> **BC-5.39.001 streak: 3/3 UNCHANGED.** Out-of-band direction review; cluster-4's LOCAL
> cascade was 3/3 CONVERGED (D-1211). Cluster-5 LOCAL cascade has not started.

## Verified Claims

Codex independently confirmed two orchestrator claims:
1. **`migrate TARGET_FILES` = 4 indexes + STATE.md only** — confirmed via
   `crates/last-amended-migrate/src/migrate.rs` target list; the append-log artifact class
   (decision-log.md, burst-log.md, lessons.md, session-checkpoints.md) is NOT in scope.
2. **`run_mechanism_a_backfill_split` has zero production callers** — confirmed via grep of
   `crates/`; no caller exists outside tests; this is the same gap as F-006/D-1191 that
   S-25.06 is the sanctioned fix for.

## Verdict

**DIRECTION CONDITIONALLY CORRECT.** Proceeding to cluster-5 F1/spec convergence is
sensible. Proceeding through Cohort-B activation under the current deferral plan is NOT
safe until HIGH-2 is resolved. Summary:

| Finding | Severity | Status |
|---------|----------|--------|
| CV-DIR-F1 — S-25.06 dependency cycle | HIGH | **FIXED** (S-25.06 v1.0→v1.1 by story-writer) |
| CV-DIR-F2 — Impossible execution path in S-25.06 AC-006 | HIGH | **OPEN** → cluster-5 F1 architect |
| CV-DIR-F3 — Unbounded deferral [S-25.06-DRIFT-001] | MED | **TIGHTENED** (STATE.md v10.43 this burst) |
| CV-DIR-F4 — Hook cwd-relative path defect | MED | **TRACKED** → S-12.14 (story authored + registered) |

## Part A — Findings

### CV-DIR-F1 (HIGH) — FIXED — Break the backfill dependency cycle before scheduling activation

**Severity:** HIGH  
**Category:** consistency  
**Confidence:** HIGH  
**Location:** `.factory/stories/S-25.06-append-log-backfill-split-executor.md:27`  
**Status:** **FIXED this burst (S-25.06 v1.0→v1.1 by story-writer)**

**Finding:** S-25.06 v1.0 declared `depends_on: [S-25.02]` and `blocks: []`. S-25.02:1522
requires "Cohort B fail-closed flip ONLY after BC-1.18.008's backfill-split has COMPLETED"
and says it is "never deployed in the same release as, or before, the backfill-split."
BC-1.18.011:62-65 additionally schedules B2 migration "at the SAME moment BC-1.18.008's
mechanism-A backfill-split runs." STATE.md:277 identifies S-25.06 as the production
call-site story. This creates a circular dependency: S-25.02's Cohort-B activation
(cluster-7) cannot dispatch until the backfill runs, yet S-25.06 was blocking on S-25.02
completing.

**Fix applied:** S-25.06 v1.1 by story-writer: `depends_on: []` (mechanism delivered in
S-25.02 cluster-3, PR #831 MERGED — no unmet story-level prerequisite), `blocks: [S-25.02]`
(cluster-7 MUST NOT dispatch until S-25.06 backfill completes), `wave: "W3"`,
`risk_mitigations: [R-S25.06-001, R-S25.06-002, R-S25.06-003]`, Rule 7 rewritten as OPEN
activation gate. STORY-INDEX v4.470→v4.471 + E-25 blockquote updated (this burst,
state-manager). BC-1.18.011 simultaneous-activation reconciliation vs corrected sequencing
→ cluster-5 F1 tracked follow-up (a).

---

### CV-DIR-F2 (HIGH) — OPEN — Specify an executable authorization path for the native migration

**Severity:** HIGH  
**Category:** spec-gap  
**Confidence:** HIGH  
**Location:** `.factory/stories/S-25.06-append-log-backfill-split-executor.md:372`  
**Status:** **OPEN → cluster-5 F1 architect**

**Finding:** S-25.06 Rule 7 requires "agent-executed CLI invocation logged as an Edit/Write
tool call." AC-006:185-188 requires a native CLI and says "No WASM plugin or hook intercepts
the invocation itself." Lines 129-132 reject even a "human-authorized shell cp" and promise
a CLI "through the same Edit/Write tool surface that hooks validate." The existing function
at `shard_manager.rs:5657` performs native filesystem work; its interface does not provide
an Edit/Write tool bridge. This is an internally contradictory execution path — it cannot
be implemented as specified.

**Recommendation:** Architect must specify a real invocation mechanism and its permission
policy: either a narrowly sanctioned native command with explicit validation, audit, and
authorization rules, or an actual tool adapter. Remove the requirement to relabel CLI
execution as Edit/Write. Validate that the chosen route can execute under the operator's
permission classifier before committing to this story as the sole compaction remedy.

**Disposition:** Cluster-5 F1 tracked follow-up (c): architect to spec the sanctioned
invocation mechanism and permission policy for S-25.06 T-10. Do NOT proceed to activation
under the current spec.

---

### CV-DIR-F3 (MED) — TIGHTENED — Bound the deferral and require evidence of restored validation

**Severity:** MED  
**Category:** completeness  
**Confidence:** HIGH  
**Location:** `.factory/STATE.md:277`  
**Status:** **TIGHTENED this burst (STATE.md v10.42→v10.43)**

**Finding:** [S-25.06-DRIFT-001] closed merely "when S-25.06 transitions to merged";
S-25.06:40-47 was P1 with `wave: null` and empty `risk_mitigations`. Measured current files
total 3,706,165 bytes (`decision-log.md` 1,233,463 B + `session-checkpoints.md` 1,228,323
B). `hooks-registry.toml:1075-1081` configures `validate-burst-log` as PostToolUse with
`on_error='continue'` — validation is silently LOST on oversized files.
CLAUDE.md:375 explicitly says 20M fuel "does not eliminate exhaustion" and that rc.23 still
embeds 10M.

**Fix applied:** [S-25.06-DRIFT-001] tightened (STATE.md v10.43): OWNER added
(state-manager owns tracking; story-writer owns S-25.06 spec), LATEST-PERMISSIBLE
checkpoint (before W3 gate closes, i.e., before S-25.06 transitions to `ready`), STOP
CONDITION added (if validators cannot complete under INSTALLED runtime OR append-log growth
rate exceeds mutation tooling capacity), closure criterion changed to evidence-based: live
census + actual size reduction + ShardRegistry enrollment + successful validation under
INSTALLED runtime (rc.23, 10M fuel — NOT the develop 20M bump). S-25.02 Cohort-B flip
(cluster-7) explicitly BLOCKED until all pass.

---

### CV-DIR-F4 (MED) — TRACKED — Track the nested-path producer defect

**Severity:** MED  
**Category:** completeness  
**Confidence:** HIGH  
**Location:** `.factory/STATE.md:281`  
**Status:** **TRACKED this burst — [D-1207] re-classified as defect, S-12.14 authored + registered**

**Finding:** D-1207 anchored to "next artifact-path-registry touch" with "no story ID
allocated yet" and labels the files "harmless." Current `.factory/` working-tree status
still shows `?? .factory/hooks/` and `?? .factory/logs/postcompact-reanchor-2026-09-11.jsonl`.
`update-cargo-audit-cache.sh:7` sets `OUTPUT_FILE='.factory/hooks/cargo-audit-cache.json'`,
a cwd-relative destination — when the hook runs with cwd inside `.factory/`, it re-prefixes
`.factory/` and lands one level too deep. The consumer read-path has not been verified;
calling the misplaced cache "harmless" is unsubstantiated.

**Fix applied:** [D-1207] re-classified from "harmless" to "defect → S-12.14" (STATE.md
v10.43). Story-writer authored S-12.14 (`S-12.14-cargo-audit-cache-cwd-relative-path-fix.md`,
E-12, 3 pts, P2, draft, hash 63c0b70). Registered in STORY-INDEX E-12 table v4.471 (this
burst, state-manager). Ignore-pattern registration (artifact-path-registry) kept separate
from the producer-path fix per finding recommendation.
