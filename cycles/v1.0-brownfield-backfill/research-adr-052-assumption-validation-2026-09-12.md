---
document_type: research-assumption-validation
level: ops
version: "1.0"
status: complete
producer: research-agent
timestamp: 2026-09-12T00:00:00Z
scope: "ADR-052 assumption validation — 5 questions (Q1 allowlist mechanics; Q2 dispatcher PreToolUse guards; Q3 hook-driven alternative viability; Q4 security standards; Q5 crash-atomicity vs existing write_atomic)"
streak_impact: NONE
commissioned_by: "orchestrator — ADR-052 POLICY 22 ratification gate"
human_decision: "ADR-052 NOT RATIFIED (D-1214) — research findings converged with Codex: DO NOT ratify"
sources:
  - "docs.anthropic.com/en/docs/claude-code/permissions"
  - "docs.anthropic.com/en/docs/claude-code/hooks"
  - "docs.anthropic.com/en/docs/claude-code/security"
  - "docs.anthropic.com/en/docs/claude-code/settings"
  - "NIST SP 800-53r5 (AC-5/6, AU-9, CM-3)"
  - "CWE-88 (argument injection), CWE-78 (OS command injection), CWE-22 (path traversal)"
  - "OWASP: Command Injection, Logging Cheat Sheet"
  - "SLSA (supply-chain integrity)"
  - "man7 rename(2)/fsync(2)"
  - "sqlite.org/atomiccommit.html"
  - "PostgreSQL file_utils.c"
  - "OSDI'14 Pillai et al. — All File Systems Are Not Created Equal"
  - "crates/last-amended-migrate/src/atomic_write.rs"
  - "crates/factory-dispatcher/src/shard_manager.rs"
  - "plugins/vsdd-factory/hooks-registry.toml"
  - ".claude/settings.json"
---

# research-adr-052-assumption-validation-2026-09-12 — Research-Agent Assumption Validation

> **Commissioned by orchestrator** to validate 5 key assumptions in ADR-052 before POLICY 22
> ratification. Findings converge with the cross-vendor Codex review
> (`adv-cv-adr052-cluster5-F1-2026-09-12.md`): **DO NOT RATIFY ADR-052 as-is.**
>
> Human decision: **ADR-052 NOT RATIFIED (D-1214)** — sent back for architect revision.

## Bottom-Line Verdicts

| Question | Verdict | Confidence |
|----------|---------|------------|
| Q1 — Native binary, not WASM | **HOLDS** | HIGH |
| Q2 — Standing settings.json Bash allowlist is sufficient | **DOES NOT HOLD** (weak on necessity AND sufficiency) | MED-HIGH |
| Q3 — Narrow allowlist patterns provide confinement | **DOES NOT HOLD** (over-broad; CWE-88) | HIGH |
| Q4 — Hook-driven no-agent alternative non-viable | **DOES NOT HOLD** (IS viable; rejected on governance only) | HIGH |
| Q4b — Census-to-stdout is adequate audit trail | **DOES NOT HOLD** (incomplete; not tamper-evident) | HIGH |
| Q5 — BC-1.18.011 crash-atomicity complete | **DOES NOT HOLD** (multiple gaps vs existing `write_atomic`) | HIGH |

---

## Q1 — Native binary placement (not WASM) — HOLDS (HIGH)

**Verdict: HOLDS.**

Anthropic docs confirm that `Bash(...)` allows shell execution of native binaries. WASM plugins run
inside a sandboxed fuel-limited executor (hooks-registry.toml; CLAUDE.md fuel-cap discussion). A
native binary invoked via `Bash(...)` escapes the WASM sandbox entirely and runs with full user
permissions. This is consistent with ADR-052's claim that a native binary is the correct placement
for a multi-file migration that exceeds WASM fuel budgets. WASM placement would be technically
impossible at current fuel-cap (rc.23: 10M; CLAUDE.md:375 confirms 20M develop bump not yet
effective at operator level).

**Conclusion for ADR-052:** The native-binary placement rationale is sound. This finding does not
block ratification.

---

## Q2 — Standing Bash allowlist weak on necessity AND sufficiency — DOES NOT HOLD (MED-HIGH)

**Verdict: DOES NOT HOLD.**

### Necessity: partially undermined

Anthropic docs (hooks page) state: hooks "execute shell commands with your full user permissions and
can modify, delete, or access any files." A PreToolUse hook that fires on `^Bash$` and executes a
native binary to perform the migration is **technically viable**. ADR-052 rejected this alternative
on control/authorization grounds, not viability — and conflated the two. A hook-driven approach IS
possible; whether it is preferred is a governance decision.

A third option identified (see Q3): agent invokes the binary via `Bash(...)` approved interactively
**once at F4**, with NO standing allowlist. This is zero persistent permission for a one-time
migration and is viable.

### Sufficiency: CRITICAL failure

**Now CONFIRMED by orchestrator registry check:** `plugins/vsdd-factory/hooks-registry.toml`
registers **14** `^Bash$` PreToolUse guards including `destructive-command-guard` and
`validate-factory-path-staging`. A settings.json Bash allowlist suppresses the Claude Code
**permission prompt** but a blocking PreToolUse hook fires **regardless of allow rules**. Therefore:

- A settings.json Bash allowlist is NECESSARY-BUT-NOT-SUFFICIENT.
- The 14 `^Bash$` dispatcher guards must ALSO be configured to permit the migrate command.
- This likely explains the original `cp` block that motivated ADR-052.

ADR-052 does not address the dispatcher PreToolUse guard stack at all. The proposed allowlist alone
cannot make the migration command executable.

---

## Q3 — Allowlist patterns not narrow; CWE-88 over-broad — DOES NOT HOLD (HIGH)

**Verdict: DOES NOT HOLD.**

**Anthropic docs (permissions page, verbatim):** The permission system matches against allow-rules
with shell-glob semantics. A leading `*` in a Bash rule matches any prefix, including spaces and
directory separators. A trailing `*` matches any suffix, including additional arguments,
subcommands, and shell metacharacters.

**Pattern analysis of proposed rules (`Bash(*/factory-dispatcher migrate-bc-index*)`):**

- Leading `*` — matches any executable path ending in `factory-dispatcher`. An executable at
  `/tmp/untrusted/factory-dispatcher` satisfies the rule. **CWE-88 (Argument Injection via
  unvalidated executable path).**
- Trailing `*` — matches any argument suffix: `migrate-bc-index --dry-run`, `migrate-bc-index;
  rm -rf`, `migrate-bc-index-all-shards-then-drop-table`. No sub-command or argument is rejected.
- Shell-chain escalation: shell-chain splitting (`;`, `&&`, `|`) behavior under the Bash allow-rule
  is each sub-command evaluated independently; a no-`*` exact rule requires exact match; but the
  proposed patterns use both wildcards and therefore do NOT restrict chaining.

**Narrowest safe pattern:** absolute path pin to the known install location, no trailing `*`, exact
closed argument list. Even then, pin should include source provenance (SLSA).

**OWASP/CWE conclusions:** CWE-88 (argument injection), CWE-22 (path traversal via args), CWE-78
(OS command injection via trailing wildcard + shell metacharacters). Anthropic docs describe
arg-patterns as "fragile / not a security boundary" — this is accurate for wildcard patterns.

---

## Q4 — Hook-driven and one-time-interactive alternatives ARE viable — DOES NOT HOLD (HIGH)

**Verdict: DOES NOT HOLD (alternatives viable).**

ADR-052's rejection of alternatives conflates viability with governance preference:

### Hook-driven native alternative (viable)

Anthropic docs (hooks page): "Hooks are shell commands that execute with your full user permissions."
`main.rs:386-417` shows the dispatcher already executes native mutation logic (`execute_roll`) inside
a PreToolUse hook path. A PreToolUse hook that (a) verifies F4 readiness/activation approval,
(b) executes the native migration, (c) exposes a read-only census sub-command for stdout evidence,
is architecturally consistent with the existing dispatcher design. Recovery semantics (resume from
PREPARED/COMMITTED markers) are achievable. The rejection in ADR-052:175-190 evaluated only
*unconditional* hook activation ("triggered automatically") — explicitly armed, one-shot hook
activation was not evaluated.

### One-time-interactive Bash alternative (viable, preferred for zero-standing-permission)

Agent invokes `factory-dispatcher migrate-bc-index` via `Bash(...)` approved interactively **at F4
activation time** — human sees the permission prompt, approves once, no entry in settings.json. The
migration runs, completes, and the permission is never granted again. This provides:
- Human intent verification at the exact activation boundary (satisfies ADR-052:64-66)
- Zero standing permission surface (eliminates F2/F3 concerns)
- Stdout evidence path (satisfies CLAUDE.md:239)
- Requires only dispatcher PreToolUse guard amendment (same as other approaches)

**Note:** This alternative was NOT evaluated in ADR-052. It is the lowest-risk mechanism of the
three.

### Audit trail — census-to-stdout NOT adequate (NIST AU-9)

NIST SP 800-53r5 AU-9 requires audit records to be tamper-evident and protected from modification
or deletion. A census printed to stdout is observable but not persisted, not signed, and not
protected against loss (context compaction, session end, tab close). An adequate audit trail
requires durable persistence independent of the current session (e.g., factory-artifacts commit
with the census output, or a signed manifest). Census-to-stdout alone does not satisfy AU-9.

---

## Q5 — BC-1.18.011 crash-atomicity incomplete vs `write_atomic` — DOES NOT HOLD (HIGH)

**Verdict: DOES NOT HOLD — multiple gaps identified.**

### What `crates/last-amended-migrate/src/atomic_write.rs` does well (single-file scope)

The existing `write_atomic` implementation: dual content-preservation (orig + staged copy),
independent ID-census vs `total_bcs` oracle cross-check, fixed SS-01..SS-10 processing order,
fail-loud rollback. Correct for its single-file scope.

### Gaps in BC-1.18.011 for multi-file B2 migration

**Gap 1 — No commit-pointer / all-or-nothing marker (Invariant 3 hole)**

B2 writes across multiple SS-NN shards + BC-INDEX. There is no single durable marker recording that
ALL target files have been atomically replaced as one unit. A crash after some shards are replaced
but before others leaves the index in a permanently inconsistent state with no deterministic resume
point. `write_atomic` handles one file; Invariant 3 requires the multi-file group be treated as
one atomic unit — this is not achievable by composing N independent `write_atomic` calls without
a commit-pointer.

**Gap 2 — Dir-fsync best-effort, not mandatory**

`write_atomic` calls `fsync` on the file but does not mandate `fsync` on the parent directory
(required by POSIX rename(2) to guarantee the directory entry survives a crash). Pillai et al.
(OSDI'14) document that missing dir-fsync is a common crash-recovery failure mode. BC-1.18.011
does not require dir-fsync.

**Gap 3 — No durable PREPARED/COMMITTED/CLEANED phase marker for resume**

EC-003 requires a rerun to "resume directly to the atomic-replace step from verified staging."
There is no specified durable marker (e.g., a `.migration-state` file or factory-artifacts commit)
recording which phase the migration reached before a crash. Without this, "resume from verified
staging" is unimplementable — the resuming process cannot distinguish "staging complete, not yet
committed" from "partially committed" from "never started."

**Gap 4 — No pre-commit source-fingerprint recheck (TOCTOU)**

The migration reads source content at census time (Postcondition 2) and writes at atomic-replace
time (Postcondition 3). There is no requirement to re-verify that the source has not changed
between those two points. Concurrent governed writers (or a human edit) can modify BC-INDEX
between census and write. `shard_manager.rs:5711-5715` → `:6061-6065` illustrates the same gap
in the existing A implementation.

**Gap 5 — No writer exclusion / maintenance-window boundary**

See CV-ADR052-F4. BC-1.18.011 does not specify a mechanism to prevent concurrent governed writes
during migration. Ordinary BC updates (Edit/Write tool calls validated by `validate-factory-path-staging`)
can interleave with the migration steps. PostgreSQL's file_utils.c uses `LockRelationForExtension`
for this class of problem; a factory equivalent is not specified.

### Additional BC-1.18.011 gaps identified (from Codex + research convergence)

- **PC4/PC6 amendment incomplete:** ADR-052's F1 delta analysis prescribes Precondition 4 amendment
  to remove A/B2 scheduling dependency; Postcondition 6 (which still requires same-moment activation)
  is not correspondingly amended. Inconsistent.
- **PC7 misattribution:** The "independent one-time operations" language attributed to Postcondition 7
  in the F1 analysis actually appears in Invariant 4:153-156 (per CV-ADR052-F6). BC-INDEX body
  amendment must target the correct location.
- **BC-1.18.010 Invariant 2 mapping gap:** BC-1.18.010 Invariant 2:145-149 requires future
  renumbering to propagate through the authoritative source. The ADR-052 config-snapshot mechanism
  does not specify which committed ARCH-INDEX revision the snapshot represents at activation time
  (CV-ADR052-F7).

---

## Summary: What Must Change Before ADR-052 Can Be Ratified

1. **CLAUDE.md exception declared** — explicitly name this as a narrowly authorized exception to the
   Edit/Write-only rule; enumerate skipped governance controls; require human-approved waivers.
2. **Allowlist patterns pinned** — absolute path, no leading wildcard, closed argument grammar; or
   abandon the standing-allowlist mechanism entirely in favour of one-time-interactive.
3. **Dispatcher PreToolUse guard stack addressed** — the 14 `^Bash$` guards must be amended or
   bypassed under a named policy; the allowlist alone does not make the command executable.
4. **Activation authorization decoupled from permission grant** — per-run activation approval
   bound to migration ID, input revision, readiness checks, and expiration.
5. **Writer exclusion specified** — maintenance boundary or lock mechanism preventing concurrent
   governed writes during migration.
6. **Commit-pointer / phase-marker specified** — durable PREPARED/COMMITTED/CLEANED states for
   crash-resume.
7. **Source-fingerprint recheck at commit point** — TOCTOU guard between census and atomic-replace.
8. **Three mechanisms evaluated head-to-head** in ADR-052: (a) standing Bash allowlist; (b) explicitly
   armed one-shot hook action; (c) one-time-interactive per-session Bash invocation. Current ADR
   evaluated only a strawman version of (b).
9. **BC-1.18.011 Postcondition 6 amended** — remove A/B2 same-moment coupling clause.
10. **Audit trail hardened beyond stdout** — durable, tamper-evident record (NIST AU-9).

## Routing (D-1214)

- **Architect:** Revise ADR-052 — evaluate 3 mechanisms head-to-head; address F1/F2/F3/F5/F7.
- **Product-owner:** Harden BC-1.18.011 — writer exclusion, phase-markers, Postcondition 6,
  Invariant 3/4 precision; harden BC-1.18.010 Invariant 2 — Gaps 1-5 above.
- **Cluster-5 TDD:** Remains BLOCKED until ADR-052 is revised and re-ratified (POLICY 22).
