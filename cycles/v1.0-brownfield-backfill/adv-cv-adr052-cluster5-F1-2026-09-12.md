---
document_type: cross-vendor-adr-review
level: ops
version: "1.0"
status: complete
producer: adversary
adversary_model: openai-codex (CROSS-VENDOR — OpenAI Codex)
review_type: ADR
review_scope: "ADR-052 — native migration CLI + Bash-tool allowlist sanctioned execution path; POLICY 22 ratification assessment for cluster-5 F1"
timestamp: 2026-09-12T00:00:00Z
phase: F1
review_date: 2026-09-12
verdict: "RATIFY-WITH-CHANGES — do not ratify current text; 7 findings (4 HIGH + 3 MED); direction sound but mechanism unsafe+insufficient; human decision: sent back for architect revision (D-1214)"
novelty: HIGH
finding_count: 7
finding_breakdown: "HIGH-1 policy-exception undeclared [architect]; HIGH-2 allowlist over-broad CWE-88 [architect]; HIGH-3 standing permission ≠ activation auth [architect]; HIGH-4 writer exclusion incomplete [product-owner/architect]; MED-5 hook-driven alternative not evaluated [architect]; MED-6 cross-migration coupling clauses inconsistent [product-owner]; MED-7 snapshot not bound to deployed revision [architect]"
streak_impact: NONE
# CRITICAL: This is a CROSS-VENDOR ADR review, NOT a BC-5.39.001 cascade pass.
# It reviews ADR-052 for POLICY 22 ratification readiness.
# It does NOT advance or reset the cycle-level BC-5.39.001 convergence streak.
# BC-5.39.001 streak stays 3/3 UNCHANGED.
# Finding IDs use CV-ADR052 prefix.
human_decision: "DO NOT RATIFY — sent back for architect revision + product-owner BC hardening (D-1214, 2026-09-12)"
inputs_reviewed:
  - .factory/specs/architecture/decisions/ADR-052-native-migration-cli-bash-tool-allowlist-sanctioned-execution-path.md
  - .factory/cycles/v1.0-brownfield-backfill/s2502-cluster5-f1-delta-analysis.md
  - .factory/cycles/v1.0-brownfield-backfill/adv-cv-dir-cluster5-F1-direction-2026-09-12.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.010.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.011.md
  - CLAUDE.md
  - .claude/settings.json
  - ".factory/policies.yaml (targeted policy lookup)"
  - "crates/factory-dispatcher/src/shard_manager.rs (migration, publication, recovery, and symbol searches)"
  - "crates/factory-dispatcher/src/main.rs (native hook integration)"
  - "crates/last-amended-migrate/src/migrate.rs"
  - "crates/ (migration caller search)"
  - "https://code.claude.com/docs/en/permissions"
source_file: "/tmp/codex-review-adr052-1789233449/codex-review.json"
---

# adv-cv-adr052-cluster5-F1-2026-09-12 — Cross-Vendor ADR Review (OpenAI Codex)

> **CROSS-VENDOR ADR review of ADR-052 (native migration CLI + Bash-tool allowlist) for POLICY 22
> ratification.** This is NOT a BC-5.39.001 cascade adversarial pass and DOES NOT affect the
> convergence streak (stays 3/3 UNCHANGED). OpenAI Codex reviewed ADR-052 and related artifacts
> from the raw JSON at `/tmp/codex-review-adr052-1789233449/codex-review.json`.
> Finding IDs use CV-ADR052 prefix.
>
> **BC-5.39.001 streak: 3/3 UNCHANGED.**
>
> **Human decision (D-1214):** ADR-052 NOT RATIFIED. Codex verdict RATIFY-WITH-CHANGES (7 findings)
> + research-agent validation CONVERGED on DO-NOT-RATIFY. ADR-052 sent back for architect revision.
> Cluster-5 TDD remains BLOCKED pending re-ratification.

## Codex Summary

> "RATIFY-WITH-CHANGES; do not ratify the current text. Native migration commands in
> factory-dispatcher are a reasonable placement, and independent activation of A and B2 appears
> technically sound while preserving B2's own atomic migration boundary. However, the ADR
> rationalizes an exception to the existing write policy without defining equivalent controls,
> overstates its allowlist confinement, and unfairly dismisses explicitly armed hook execution. The
> most important POLICY 22 decision is whether to authorize a new agent-accessible governed-write
> path whose permission currently identifies only a command name, not a trusted executable, approved
> activation, or validated input state."

## Verdict

**RATIFY-WITH-CHANGES — DO NOT ratify current text.** 4 HIGH + 3 MED findings requiring resolution
before POLICY 22 ratification. Direction (native migration in factory-dispatcher, independent A/B2
activation) is sound; mechanism (standing Bash allowlist as sole control) is unsafe and insufficient.

| Finding | Severity | Category | Status |
|---------|----------|----------|--------|
| CV-ADR052-F1 — Policy exception undeclared; CLAUDE.md not amended | HIGH | purity-boundary | **OPEN → architect** |
| CV-ADR052-F2 — Allowlist over-broad; leading wildcard admits untrusted executables (CWE-88) | HIGH | security | **OPEN → architect** |
| CV-ADR052-F3 — Standing permission ≠ activation authorization; no per-run binding | HIGH | security | **OPEN → architect** |
| CV-ADR052-F4 — Writer exclusion unspecified across migration + checkpoint recovery | HIGH | concurrency | **OPEN → product-owner/architect** |
| CV-ADR052-F5 — Explicitly armed hook alternative not evaluated (only unconditional evaluated) | MED | completeness | **OPEN → architect** |
| CV-ADR052-F6 — Cross-migration coupling clauses inconsistent (Postcondition 6 not amended) | MED | consistency | **OPEN → product-owner** |
| CV-ADR052-F7 — Mapping snapshot not bound to deployed artifact revision | MED | consistency | **OPEN → architect** |

## Part A — Findings

### CV-ADR052-F1 (HIGH) — OPEN — Define an explicit policy exception and amend CLAUDE.md

**Severity:** HIGH
**Category:** purity-boundary
**Confidence:** HIGH
**Location:** `.factory/specs/architecture/decisions/ADR-052-...:199`

**Finding:** CLAUDE.md:231 says "Use Edit/Write tools ONLY for `.factory/` mutations." ADR-052:199-205
characterizes the prohibition as targeting "unstructured, unvalidated writes" and argues that
implementing the full BC makes the binary "the OPPOSITE of a bypass." ADR-052:230-234 explicitly
acknowledges that "the standard hook-validation chain does not fire" and substitutes census evidence.
Atomic writes and census checks establish migration integrity; they do not establish equivalence to
every skipped governance check. ADR-052's Files to Change table also omits CLAUDE.md.

**Recommendation:** Describe this as a narrowly authorized exception. Amend the governing no-bypass
instruction in the same ratification package. Enumerate the skipped controls. For each, require
equivalent enforcement in the CLI or record an explicit human-approved waiver. Gate activation on
those controls passing, rather than treating spec implementation or stdout as blanket equivalence.

---

### CV-ADR052-F2 (HIGH) — OPEN — Bind permission to a trusted executable and exact invocation

**Severity:** HIGH
**Category:** security
**Confidence:** HIGH
**Location:** `.factory/specs/architecture/decisions/ADR-052-...:108`

**Finding:** The proposed rules are `Bash(*/factory-dispatcher migrate-bc-index*)` and
`Bash(*/factory-dispatcher backfill-append-logs*)`. The leading wildcard admits an executable with
that basename in any directory (e.g., `/tmp/untrusted/factory-dispatcher` satisfies the pattern).
The trailing wildcard leaves arguments and subcommand suffixes unrestricted. Lines 115-117 defer
exact permission semantics until implementation despite calling the pattern narrow. Current
`.claude/settings.json:1-5` contains only `enabledPlugins` — no existing Bash patterns exist to
support ADR:212's claim about project Bash permissions. Anthropic's permission docs describe wildcard
command matching as fragile and not a security boundary.

**Recommendation:** Pin the trusted executable location and provenance. Specify the complete accepted
argument grammar and canonical repository root. Reject unknown arguments and paths outside the
governed target set. Before activation, test the actual installed permission classifier with alternate
executable directories, subcommand suffixes, extra arguments, wrappers, and compound commands. Do not
assume shell-chain escalation is blocked without those tests.

---

### CV-ADR052-F3 (HIGH) — OPEN — Separate persistent execution permission from activation authorization

**Severity:** HIGH
**Category:** security
**Confidence:** HIGH
**Location:** `.factory/cycles/v1.0-brownfield-backfill/s2502-cluster5-f1-delta-analysis.md:198`

**Finding:** The F1 analysis states "the permission grant in settings.json IS the authorization; no
ad-hoc permission prompt is needed at execution time." ADR-052:64-66 requires "explicit human
authorization" at a "known activation boundary," but its permission entries encode no activation
identity, input revision, readiness state, or expiration. ADR:119-120 assigns installation to
devops-engineer before execution. A standing command permission therefore remains usable against
later or different repository states.

**Recommendation:** Specify a separately recorded activation approval bound to the repository,
migration, approved input/config revision, and readiness checks. Require the CLI to validate that
approval before mutation and define safe retry semantics. Remove or expire the temporary allowlist
after successful activation; installing an execution rule must not itself certify F4 readiness.

---

### CV-ADR052-F4 (HIGH) — OPEN — Specify writer exclusion across migration and checkpoint recovery

**Severity:** HIGH
**Category:** concurrency
**Confidence:** MEDIUM
**Location:** `.factory/specs/behavioral-contracts/ss-01/BC-1.18.011.md:164`

**Finding:** EC-003 requires a rerun to "resume directly to the atomic-replace step" from verified
staging. Postcondition 2 captures the census before splitting; Postcondition 3 publishes after
verification. Neither BC-1.18.011 nor ADR-052 specifies writer exclusion or source-generation
validation. Searches for lock/exclusive/concurrency/quiescence terms found no such requirement.
The native A implementation illustrates the integration risk: `shard_manager.rs:5711-5715` reads
the source, then `:6061-6065` overwrites it from an earlier partition buffer. Per-file atomic
replacement does not prevent a concurrent ordinary edit from being lost.

**Recommendation:** Require a migration-wide exclusion mechanism respected by ordinary governed
writers, or an enforced maintenance boundary that drains and blocks them. Bind staged checkpoints
to source and configuration hashes and revalidate before publication under that exclusion. Add tests
for concurrent edits, competing migrations, and source changes between crash and resume.

---

### CV-ADR052-F5 (MED) — OPEN — Evaluate an explicitly armed native hook alternative

**Severity:** MED
**Category:** completeness
**Confidence:** HIGH
**Location:** `.factory/specs/architecture/decisions/ADR-052-...:173`

**Finding:** ADR:175-190 rejects a migration "triggered automatically" because it is
timing-uncontrollable, cannot satisfy human intent, and has no stdout evidence path. That only
evaluates unconditional detection-based activation. The existing dispatcher already executes native
mutation logic inside the hook path: `main.rs:386-417` describes the native PreToolUse gate and
its `execute_roll` call. CLAUDE.md:239 requires literal shell evidence for mechanical gates; it
does not require that the mutation itself originate in Bash.

**Recommendation:** Compare the CLI against (a) an explicitly armed, one-shot native dispatcher
action that verifies approval/readiness, blocks and requests retry after migration, and exposes a
read-only census command for shell evidence; and (b) a one-time-interactive per-session Bash
invocation (no standing allowlist). Evaluate recovery, event transport, and operational cost
concretely. The present rejection does not establish that the added permission surface is necessary.

---

### CV-ADR052-F6 (MED) — OPEN — Remove all cross-migration coupling clauses consistently

**Severity:** MED
**Category:** consistency
**Confidence:** HIGH
**Location:** `.factory/cycles/v1.0-brownfield-backfill/s2502-cluster5-f1-delta-analysis.md:156`

**Finding:** The prescribed replacement changes only Precondition 4 to say the migrations are
independently gated. BC-1.18.011 Postcondition 6:111-119 still requires second-level sub-sharding
"at the SAME F4 activation moment mechanism A's own backfill ... runs." The analysis:137 also
attributes the phrase "independent one-time operations that happen to be scheduled at the same F4
activation boundary" to Postcondition 7; actual Postcondition 7:121-125 concerns Cohort-B
consumers — the quoted wording occurs in Invariant 4:153-156 and refers to the Cohort-B flip.

**Recommendation:** Amend Postcondition 6 together with Precondition 4 to remove only the A/B2
scheduling dependency. Preserve the requirement that all B2 subsystem and second-level splits form
one atomic operation. Correct the Postcondition 7 attribution and explicitly distinguish A/B2
independence from B2/Cohort-B independence.

---

### CV-ADR052-F7 (MED) — OPEN — Bind the mapping snapshot to the deployed artifact revision

**Severity:** MED
**Category:** consistency
**Confidence:** HIGH
**Location:** `.factory/specs/architecture/decisions/ADR-052-...:158`

**Finding:** ADR:158-166 promises a CI parity test and says the config snapshot is validated "at
every commit," while explicitly forbidding runtime ARCH-INDEX reads. BC-1.18.010 Invariant 2:145-149
requires future renumbering to propagate through the authoritative source. CLAUDE.md:249 states that
develop source edits do not affect the cached plugin and require a release. CI parity at one revision
therefore does not establish parity between the installed config and the live artifact revision used
at activation.

**Recommendation:** Define which committed ARCH-INDEX revision the snapshot represents and verify
that binding during activation and config loading. Either generate and deploy the snapshot with a
checked source hash or fail closed on runtime parity mismatch. Add a stale-installed-config test;
a CI-only comparison must not be described as guaranteeing live parity.
