---
document_type: architecture-decision-record
level: L3
version: "1.1"
status: proposed
producer: architect
timestamp: 2026-09-12T00:00:00Z
phase: F1
subsystems_affected:
  - SS-01
traces_to: .factory/specs/architecture/ARCH-INDEX.md
inputs:
  - .factory/cycles/v1.0-brownfield-backfill/adv-cv-dir-cluster5-F1-direction-2026-09-12.md
  - .factory/cycles/v1.0-brownfield-backfill/adv-cv-adr052-cluster5-F1-2026-09-12.md
  - .factory/cycles/v1.0-brownfield-backfill/research-adr-052-assumption-validation-2026-09-12.md
  - .factory/stories/S-25.06-append-log-backfill-split-executor.md
  - .factory/specs/behavioral-contracts/ss-01/BC-1.18.011.md
  - .factory/cycles/v1.0-brownfield-backfill/s2502-cluster5-f1-delta-analysis.md
  - CLAUDE.md
  - .claude/settings.json
  - crates/factory-dispatcher/src/main.rs
  - plugins/vsdd-factory/hooks-registry.toml
input-hash: "b9cf57d"
# input-hash: run compute-input-hash --update at state-manager registration burst
---

# ADR-052: Native Migration CLI — Sanctioned Execution Path for Governed One-Time Shard Migrations (v1.1 Revision)

## Status

PROPOSED — v1.0 NOT ratified per D-1214 (Codex RATIFY-WITH-CHANGES verdict + research-agent
convergence). This v1.1 revision addresses all 7 Codex findings and 6 research-agent verdicts.
Human POLICY 22 ratification required before this ADR is treated as accepted. Cluster-5 TDD
dispatch remains BLOCKED until ratification.

## Context

Two governed one-time shard migrations are specified in S-25.02:

1. **Mechanism-A append-log backfill-split** (BC-1.18.008) — wired and executed by S-25.06.
   `run_mechanism_a_backfill_split` exists in `shard_manager.rs` with zero production callers.
   S-25.06 adds the CLI entry point and executes the migration against the four live cycle
   append-log files.

2. **Mechanism-B2 BC-INDEX body-split migration** (BC-1.18.011) — cluster-5 of S-25.02.
   The `run_mechanism_b2_bc_index_split` function (to be authored in cluster-5 TDD) is the
   native migration entry point in `shard_manager.rs`.

Both migrations share a structural challenge: they perform filesystem writes against `.factory/`
paths but cannot satisfy two simultaneously imposed constraints as originally stated in S-25.06
Rule 7:

- **Constraint A:** The executor MUST be a native binary (not WASM; WASM fuel budgets are
  insufficient for multi-MB files per ADR-051 §Decision 1 / CLAUDE.md §WASM plugin fuel
  budgets).
- **Constraint B (original Rule 7 text):** The invocation must be "logged as an Edit/Write
  tool call" — implying it goes through the agent's Edit/Write surface that hooks validate.

These constraints are mutually exclusive: a native binary invoked from the shell writes to the
filesystem OUTSIDE the Edit/Write tool surface.

Three project-level governance constraints apply:

- **POL-3 / TD-FACTORY-HOOK-BYPASS-001 P0:** Agents must NOT use Python/sed/echo/shell to
  bypass the hook-validated `.factory/` write surface. This ADR declares a narrowly-authorized
  exception under POLICY 22 (§Decision 8); it does NOT claim the migration is "not a bypass."
- **D-449(a):** Mechanical gates in burst-log Dim-2 require literal shell execution with
  captured stdout, not pseudocode narrative.
- **F4 activation gate:** Both migrations are one-time operations that require explicit human
  authorization and must be executed at a known activation boundary — not triggered
  automatically.

**Correction from v1.0:** `.claude/settings.json` (project-level) currently contains ONLY
`enabledPlugins`; there are no existing Bash permission entries. The v1.0 claim about "existing
project Bash permissions" establishing precedent was false (confirmed by Codex CV-ADR052-F2 and
research Q3).

**Dispatcher guard constraint (NEW — not in v1.0):** `plugins/vsdd-factory/hooks-registry.toml`
registers 13 entries with `tool = "^Bash$"`, of which 9 are PreToolUse. A settings.json Bash
allowlist suppresses the Claude Code permission prompt but does NOT suppress any PreToolUse
hook — dispatcher hooks fire regardless of allow-rules. Any mechanism that invokes the migration
binary via the Bash tool must address the 9 PreToolUse `^Bash$` guards or the command will be
blocked before execution.

**Option A viability (correction of v1.0 conflation):** Anthropic docs confirm hooks execute
with full user permissions. The dispatcher already runs a native mutation as a PreToolUse
side-effect via `shard_cap_precheck` (in `executor.rs`) whose fired verdict routes to
`shard_manager::execute_roll` — a destructive seal-and-truncate operation — proving the
hook-internal native mutation pattern works end-to-end in this codebase. The v1.0 rejection of
hook-driven migration evaluated ONLY unconditional, detection-based ("automatically-triggered")
activation; explicitly-armed one-shot hook activation was not evaluated. This v1.1 corrects that
and evaluates all three options honestly (§Rationale).

## Decision

### Decision 1 — Mechanism selection: one-time interactive Bash approval (Option B)

Governed one-time shard migrations are invoked via the **`Bash` tool with one-time interactive
human approval at F4 activation time — NO standing allowlist entry in settings.json.**

The agent invokes `{project-root}/target/release/factory-dispatcher migrate-bc-index` at the
F4 activation step. The Claude Code harness shows a permission prompt; the human approves once.
The migration runs, completes, and the permission expires — no entry is added to settings.json;
no standing permission persists. This is Option B from the three-way evaluation.

See §Rationale for the honest head-to-head evaluation of Options A (hook-driven), B
(one-time-interactive), and C (hardened standing allowlist). Option A is viable; its rejection
is on operational complexity grounds only. Option C fails F3 (standing permission ≠
activation authorization).

### Decision 2 — Binary placement for both migration entry points (unchanged from v1.0)

Both migration entry points live in the `factory-dispatcher` binary
(`crates/factory-dispatcher/`), co-located with `shard_manager.rs`. Rationale:

- `shard_manager.rs` already owns the migration algorithm for mechanism A
  (`run_mechanism_a_backfill_split`). B2's entry point (`run_mechanism_b2_bc_index_split`)
  follows the same co-location model.
- `last-amended-migrate` is BC-10.13.001-scoped; extending it for body-structure migrations
  requires a BC-10.13.001 amendment without commensurate benefit.
- `factory-dispatcher` already has a CLI dispatch layer; adding `migrate-bc-index` and
  `backfill-append-logs` subcommands is additive and consistent with the existing pattern.

### Decision 3 — Invocation: absolute-path-pinned, closed argument grammar, no standing allowlist

The agent MUST invoke the migration binary at its **absolute trusted path** derived from the
project root, not via shell PATH resolution:

```
{project-root}/target/release/factory-dispatcher migrate-bc-index
{project-root}/target/release/factory-dispatcher backfill-append-logs
{project-root}/target/release/factory-dispatcher migrate-bc-index --census
{project-root}/target/release/factory-dispatcher backfill-append-logs --census
```

The accepted argument grammar is CLOSED: exactly the above four forms, no additional flags,
no path arguments to migration targets (targets are config-driven), no shell metacharacters,
no compound commands. Any other argument form is REJECTED by the binary with a non-zero exit
and must not be allowed by the activation approval manifest (§Decision 4).

No entry is added to `.claude/settings.json`. The one-time interactive approval at F4
activation IS the permission mechanism; it is temporally bound to the exact activation moment,
not pre-granted.

### Decision 4 — Activation authorization decoupled from permission grant (CV-ADR052-F3)

A standing permission grant MUST NOT serve as activation authorization. Authorization is a
separately-recorded approval bound to the specific migration, repository state, and readiness
checks. Mechanism:

1. **Armed-activation manifest** is written by state-manager to
   `.factory/activation/migrate-bc-index-YYYY-MM-DD.json` (or
   `backfill-append-logs-YYYY-MM-DD.json`) at the F4 human-directed activation step.
   The manifest MUST contain:
   - `migration_id`: `"migrate-bc-index"` or `"backfill-append-logs"`
   - `repo_root_sha`: the SHA of the factory-artifacts commit that was HEAD at approval time
   - `approved_arch_index_sha`: the ARCH-INDEX committed SHA the config snapshot was generated
     from (§Decision 10)
   - `expected_total_bcs`: integer read from BC-INDEX frontmatter at approval time (B2 only;
     corroborates census oracle)
   - `approved_by`: `"human-F4-interactive"`
   - `timestamp_utc`: ISO-8601 timestamp of manifest creation
   - `expires_after_hours`: `24` (migration must complete within 24 hours of approval)

2. **Binary pre-mutation validation**: the migration binary reads and validates the manifest
   before performing ANY filesystem mutation:
   - Manifest exists at the expected path (fails CLOSED if absent)
   - `migration_id` matches the running subcommand (fails CLOSED on mismatch)
   - Current UTC time is within `expires_after_hours` of `timestamp_utc` (fails CLOSED on
     expiry; prevents replay against a later repository state)
   - Current ARCH-INDEX SHA matches `approved_arch_index_sha` (fails CLOSED on mismatch —
     §Decision 10 parity check)
   - All readiness preconditions (§Decision 5 guard checks) pass

3. **Post-activation manifest disposal**: after the migration reports success (COMMITTED phase
   marker written — §Decision 7), state-manager removes the manifest file from
   `.factory/activation/`. The migration binary rejects re-runs against an absent or expired
   manifest (idempotency: it detects the already-migrated state and exits 0 without re-splitting
   per BC-1.18.011 EC-006).

### Decision 5 — Dispatcher PreToolUse guard stack (CRITICAL — not in v1.0)

A settings.json allowlist suppresses the Claude Code permission prompt; it does NOT suppress
dispatcher PreToolUse hooks. The 9 `^Bash$` PreToolUse guards in
`plugins/vsdd-factory/hooks-registry.toml` fire on every Bash tool call regardless of
settings.json allow-rules. Each must be addressed before the migration can execute:

| Guard | Expected behavior on migration command | Required action |
|-------|----------------------------------------|-----------------|
| `block-ai-attribution` | Inspects Bash input for AI-attribution patterns; migration command contains none | No amendment needed |
| `check-factory-commit` | Detects multi-commit chain in `.factory/` git log; migration runs within a single burst | No amendment needed provided burst discipline is maintained |
| `destructive-command-guard` | Likely flags migration command as potentially destructive (writes to `.factory/`) | **MUST be amended** to recognize absolute-path-pinned `factory-dispatcher migrate-bc-index` and `factory-dispatcher backfill-append-logs` as governed migration commands exempt from the guard |
| `protect-secrets` | Scans Bash input for secret patterns; migration command contains no secrets | No amendment needed |
| `verify-git-push` | Only fires on `git push` patterns; irrelevant | No amendment needed |
| `validate-factory-path-staging` | Validates that Bash commands staging `.factory/` paths do so through proper channels | **MUST be amended** to recognize the governed migration commands as the POLICY 22-authorized exception; keyed to absolute trusted path; writer-exclusion lock check (§Downstream Amendment 5) added here |
| `verify-factory-lock-bash` | Checks factory lock state | Activation sequence MUST acquire factory lock before invoking migration; guard should then pass |
| `validate-heavy-op-delegation` | May flag migration as a heavy operation requiring orchestrator delegation confirmation | **MUST be amended** or configured to allow the migration command when the armed-activation manifest is present |
| `validate-unvalidated-mutation-marker-git` | Checks for unvalidated `.factory/` mutations staged via git | Devops-engineer factory-artifacts commit step handles staging via normal git; guard does not conflict with binary-level writes |

**Guard amendment deliverable:** devops-engineer amends `destructive-command-guard`,
`validate-factory-path-staging`, and `validate-heavy-op-delegation` in hooks-registry.toml
and the underlying hook scripts/plugins to recognize the governing absolute-path-pinned
migration command pattern, validated against the presence of the armed-activation manifest.
These amendments are PR-reviewable and part of cluster-5 activation.

### Decision 6 — Audit trail: durable, tamper-evident record (NIST AU-9) (replaces v1.0 Decision 4)

Census-to-stdout is correctness evidence for D-449(a); it is NOT an adequate audit record.
NIST SP 800-53r5 AU-9 requires audit records to be tamper-evident and protected from
modification or deletion, independent of the current session. A census printed to stdout can be
lost on context compaction, session end, or tab close. Two independent audit artifacts are
required:

1. **D-449(a) evidence (stdout capture):** The migration CLI MUST write a structured census
   report to stdout on completion (success or idempotent no-op). Minimum required fields:
   - Source file path(s) operated on
   - Pre-migration record count (from `total_bcs` oracle for B2; from known file count for A)
   - Independent-census record count (fresh enumeration of actual rows/entries)
   - Per-shard record counts and file paths (including sub-shards for SS-05/SS-06 in B2)
   - Content-preservation hash (SHA-256 of source body before migration)
   - Idempotency status (`FIRST_RUN` | `ALREADY_MIGRATED` | `RESUMED_FROM_CHECKPOINT`)
   - Migration binary version and build SHA
   - Exit code taxonomy (0 = success, non-zero with E-SHD-* error code on failure)

   State-manager captures the stdout verbatim as the D-449(a) burst-log Dim-2 attestation.

2. **Durable audit record (NIST AU-9):** Immediately after the migration completes (COMMITTED
   phase marker present), state-manager commits to factory-artifacts:
   - The captured census stdout as `migration-audit/migrate-bc-index-YYYY-MM-DD-census.txt`
     (or `backfill-append-logs-...`)
   - The armed-activation manifest (copied from `.factory/activation/`)
   - The COMMITTED phase marker file
   - The migration binary's `--version` output

   The factory-artifacts commit hash is recorded in STATE.md's burst-log for the activation
   burst. This factory-artifacts commit is the tamper-evident record: it is immutable in git
   history and not subject to session compaction. It satisfies NIST AU-9's requirement for
   records that are "protected from unauthorized access, modification, and deletion."

### Decision 7 — Crash-atomicity phase markers and TOCTOU guard (addresses research Q5)

The migration binary MUST implement a durable three-phase state machine to satisfy BC-1.18.011
Invariant 3's all-or-nothing guarantee across crashes:

**Phase markers** written to `.factory/migration-state/{migration-id}-state.json`:
- `PREPARED`: all staging writes complete; pre-commit source-fingerprint recorded; all
  verification checks (content-preservation + census) pass. BC-INDEX.md original body is
  UNTOUCHED at this point.
- `COMMITTED`: all atomic file replacements (rename(2) + mandatory dir-fsync on parent
  directory) complete for ALL target files as one group. COMMITTED is written AFTER the last
  rename, making it the single commit-pointer for the multi-file atomic operation.
- `CLEANED`: staging temporary files removed.

**EC-003 resume logic:** On restart, if PREPARED marker exists but COMMITTED does not, resume
directly to the atomic-replace step (do not re-run the full split from scratch). If COMMITTED
exists, report `ALREADY_MIGRATED` and exit 0.

**Pre-commit TOCTOU guard:** Immediately before the atomic-replace step (PREPARED → COMMITTED
transition), the binary MUST re-verify that the source file has not changed since the census.
It re-computes the SHA-256 of the source and compares against the fingerprint recorded at
census time. If they differ, the migration ABORTS; the PREPARED marker is deleted; the source
is left untouched per BC-1.18.011 Postcondition 4 (rollback on verification failure).

**Dir-fsync requirement:** Each rename(2) call in the atomic-replace loop MUST be followed by
fsync on the parent directory of the target file. This ensures directory entries survive system
crashes (POSIX rename(2) semantics; Pillai et al. OSDI'14 establishes that missing dir-fsync
is a prevalent crash-recovery failure mode).

### Decision 8 — Policy exception declaration (CV-ADR052-F1 — explicit, with enumerated controls)

This ADR declares a **narrowly authorized exception** to CLAUDE.md's TD-FACTORY-HOOK-BYPASS-001
"Use Edit/Write tools ONLY for `.factory/` mutations" constraint.

The migration binary writes to `.factory/specs/behavioral-contracts/shards/` (B2) and
`.factory/cycles/*/` (A) outside the Edit/Write tool surface. This IS a bypass of the standard
hook-validated write path in the mechanical sense; it is authorized only because POLICY 22
human ratification establishes that the migration binary's own internal controls are equivalent
to or stronger than the skipped controls.

#### Skipped controls and their equivalents or waivers

| Skipped control | What it enforces on Edit/Write path | Equivalent enforcement in migration binary |
|-----------------|------------------------------------|--------------------------------------------|
| `brownfield-discipline` PreToolUse (Edit\|Write\|MultiEdit) | Verifies write is consistent with brownfield discipline (existing file shapes, incremental changes) | Migration binary's content-preservation pre-check (SHA-256 of source before any split) enforces that source content is correctly inventoried; idempotency check prevents double-split |
| `validate-factory-path-staging` PreToolUse (Bash) | Validates `.factory/` writes via Bash use proper channels | Guard amended (§Decision 5) to recognize governed migration commands; the amendment IS the equivalent enforcement |
| `validate-factory-path-staged` PostToolUse (Bash) | Verifies `.factory/` mutations from Bash are properly staged for factory-artifacts | **WAIVER (human-approved):** devops-engineer performs factory-artifacts commit manually as the post-migration burst step, verified by state-manager; this guard is bypassed at the PostToolUse level by the one-time-interactive invocation path |
| `validate-count-propagation.sh` PostToolUse behavior | Verifies count propagation across BC-INDEX | Migration binary's independent census cross-check against `total_bcs` oracle provides equivalent verification before any write; count propagation is a post-migration steady-state concern |
| POL-3 "NEVER use Python/sed/echo bypass" | Prevents unstructured, unvalidated `.factory/` writes | **EXPLICIT WAIVER (POLICY 22 authorized):** The migration binary is VSDD-authored, TDD-covered (VP-132/VP-133/VP-134), adversarially-reviewed. It implements the full BC-1.18.008/BC-1.18.011 specification. This waiver is narrowly scoped to the two governed migration subcommands and expires when the migration completes. |

#### CLAUDE.md amendment text (for human application at ratification)

The human MUST apply the following amendment to `CLAUDE.md` as part of POLICY 22 ratification.
Do NOT apply this text now; it takes effect only when the human ratifies ADR-052.

**Location:** `## Conventions (Code-Level)` section, `### Forbidden patterns` table, the row for
`TD-FACTORY-HOOK-BYPASS-001 P0`.

**Current row text (Reason column):**
```
TD-FACTORY-HOOK-BYPASS-001 P0 — Use Edit/Write tools ONLY for `.factory/` mutations. NEVER use Python/sed/echo bypass. Enforced by POL-3.
```

**Replacement row text (Reason column):**
```
TD-FACTORY-HOOK-BYPASS-001 P0 — Use Edit/Write tools ONLY for `.factory/` mutations. NEVER
use Python/sed/echo bypass. Enforced by POL-3.
ADR-052 v1.1 EXCEPTION (POLICY 22 ratified): The governed one-time shard migration binary
(`{project-root}/target/release/factory-dispatcher migrate-bc-index` and
`backfill-append-logs`) may write to `.factory/specs/behavioral-contracts/shards/` and
`.factory/cycles/*/` ONLY when ALL of the following hold simultaneously:
(a) Invoked via Bash tool with one-time interactive human approval at F4 activation boundary
    (no standing allowlist entry in `.claude/settings.json`).
(b) Armed-activation manifest at `.factory/activation/` is present, unexpired, and validated
    by the binary before any mutation — containing repo root SHA, ARCH-INDEX revision, and
    expected total_bcs count (ADR-052 §Decision 4).
(c) Durable factory-artifacts commit records the census output, activation manifest, and
    COMMITTED phase marker as the NIST AU-9 tamper-evident audit record (ADR-052 §Decision 6).
(d) Migration binary is invoked at its absolute trusted path, with the closed argument grammar
    defined in ADR-052 §Decision 3 — no trailing wildcard, no shell metacharacters.
(e) The 3 dispatcher guards requiring amendment (`destructive-command-guard`,
    `validate-factory-path-staging`, `validate-heavy-op-delegation`) have been amended per
    ADR-052 §Decision 5 and are in the deployed hooks-registry.
All other `.factory/` writes by agents remain subject to the Edit/Write-only constraint.
```

### Decision 9 — Resolving the S-25.06 Rule 7 contradiction (updated from v1.0 Decision 5)

S-25.06 Rule 7's original text ("agent-executed CLI invocation logged as an Edit/Write tool
call") is WITHDRAWN and replaced with:

> "The activation step (T-10 for mechanism A, the equivalent activation step for mechanism B2)
> is executed via the `Bash` tool with one-time interactive human approval at F4 activation
> (no standing settings.json allowlist). The migration binary is invoked at its absolute trusted
> path: `{project-root}/target/release/factory-dispatcher backfill-append-logs` (or
> `migrate-bc-index`). The invocation is NOT an Edit/Write tool call. It is a Bash execution of
> the native migration binary under ADR-052 §Decision 3's closed argument grammar, activated
> under ADR-052 §Decision 4's armed-activation manifest, producing a census report captured per
> ADR-052 §Decision 6."

Story-writer updates S-25.06 Rule 7 text accordingly in the next burst.

### Decision 10 — BC-1.18.010 Invariant 2: ARCH-INDEX mapping with revision binding (updates v1.0 Decision 6)

The BC-S-prefix→SS-NN mapping used for first-level addressing is implemented as a
**config-embedded snapshot with a revision binding and two parity checks**:

- The mapping is embedded in the `[[shard]]` TOML config entry for the BC-INDEX artifact as a
  `subsystem_prefixes` table: `{ "BC-1" = "SS-01", "BC-2" = "SS-02", ..., "BC-10" = "SS-10" }`.
- The config entry MUST also contain `arch_index_sha`: the git SHA of the ARCH-INDEX.md commit
  from which the snapshot was generated. This binding is non-optional.
- **CI parity test** (`cargo test --test arch_index_parity`): greps ARCH-INDEX.md's Subsystem
  Registry `BC-S Prefix` column at HEAD and diffs it against the config snapshot, failing on any
  divergence. This test runs on every commit to `develop` and `main`.
- **Activation-time parity check** (§Decision 4): at migration activation, the binary verifies
  the live ARCH-INDEX SHA matches `approved_arch_index_sha` in the armed-activation manifest.
  If the manifest SHA does not match the current ARCH-INDEX commit, the migration FAILS CLOSED
  before reading the snapshot. This closes the gap between "CI-validated at commit N" and "live
  at activation time" — CI parity is NECESSARY-BUT-NOT-SUFFICIENT; the activation check is the
  sufficient condition.
- **Stale-installed-config test**: a separate test (`cargo test --test stale_installed_config`)
  verifies that the binary's embedded snapshot `arch_index_sha` matches a known-valid reference.
  This test detects stale binaries built against an older ARCH-INDEX before a subsystem
  renumbering.

`shard_manager.rs` reads the mapping from the deserialized config entry only, never from
ARCH-INDEX.md at runtime. This is consistent with BC-1.18.010 Invariant 2's prohibition on
independent hardcoding: the config snapshot is CI-validated against ARCH-INDEX at every commit
AND activation-time validated against the exact ARCH-INDEX revision the binary was built for.

Product-owner amends BC-1.18.010 Invariant 2 to cite this decision per §Downstream to
Product-Owner.

## Rationale

### Head-to-head mechanism evaluation

#### Option A — Hook-driven / dispatcher-internal explicitly-armed one-shot native action

**Viability: CONFIRMED.** v1.0's rejection of this option evaluated ONLY unconditional,
detection-based ("automatically-triggered") hook activation. That was a strawman. Corrected
evaluation:

- Anthropic docs (hooks page) explicitly state: "Hooks are shell commands that execute with
  your full user permissions." The WASM sandbox applies to hook plugins running inside the
  dispatcher's WASM executor, NOT to shell commands a hook executes via `exec_subprocess`.
- The dispatcher already executes native mutations as a PreToolUse side-effect via
  `shard_cap_precheck` (in `executor.rs`) whose fired verdict routes to
  `shard_manager::execute_roll` — a destructive seal-and-truncate-to-0 operation — as a
  PreToolUse side-effect. This proves the hook-internal native mutation pattern works
  end-to-end in this codebase.
- An explicitly-armed, one-shot hook that checks an armed-approval marker before mutating DOES
  satisfy the F4 human-intent gate — the marker itself is written at human direction at F4.
- Recovery (PREPARED/COMMITTED/CLEANED markers) is achievable.
- A read-only census subcommand exposes stdout evidence for D-449(a).

**Why Option A is not chosen (operational complexity, not viability):**

1. Requires a new `[[hooks]]` entry in `hooks-registry.toml` and a new WASM or bash-adapter
   hook plugin — additional infrastructure for a one-time operation.
2. The "explicitly-armed" check requires designing and implementing an armed-marker reader
   inside the hook plugin — more new code than Option B, which needs no new hook
   infrastructure.
3. Census evidence path is less direct: the census must be surfaced via a separate read-only
   subcommand captured by the agent, rather than as natural stdout from the migration
   invocation.
4. Option B achieves the same governance properties (explicit human approval at activation, no
   standing permission, durable audit trail, D-449(a) stdout) with lower implementation
   overhead.

**Option A is NOT rejected on viability grounds.** A future recurring migration use case (if
one arises) should reconsider Option A; its encapsulation within the dispatcher hook chain is
architecturally cleaner for ongoing operations.

#### Option B — One-time interactive Bash approval at F4 (CHOSEN)

- **Least privilege:** No standing permission in settings.json; the permission expires after
  the one-time interactive approval. Zero persistent attack surface after migration completes.
- **F4 gate:** Human sees the permission prompt AT the exact activation moment — human intent
  is captured by design, not by inference.
- **D-449(a):** Stdout capture is direct; no separate census subcommand needed.
- **Guard stack:** Requires the same 3 guard amendments as Option C; no additional dispatcher
  infrastructure vs. Option A.
- **Operational cost:** Lowest of the three options — no new hook registry entries, no
  armed-marker infrastructure needed beyond the activation manifest (which is needed for
  authorization decoupling regardless of mechanism choice).
- **Audit trail:** Factory-artifacts commit provides the durable, tamper-evident record
  (§Decision 6).
- **Config binding:** Activation-time ARCH-INDEX parity check (§Decision 10) closes the
  "CI-only ≠ live parity" gap.

#### Option C — Hardened standing allowlist (absolute-path-pinned, closed grammar)

**Security viability: CONDITIONAL.** Absolute-path pin eliminates CWE-88 (Codex CV-ADR052-F2's
leading-wildcard objection). Closed argument grammar eliminates CWE-78/CWE-22
(trailing-wildcard objection). The original v1.0 patterns were insecure; hardened patterns
would be security-viable.

**Why Option C is not chosen:**

1. **Fails F3 (standing permission ≠ activation authorization):** A settings.json entry
   persists indefinitely after migration completes. It can be invoked against a different
   repository state (different ARCH-INDEX revision, different total_bcs count, different cycle
   boundary) without the armed-activation manifest's temporal binding and expiry.
2. **NECESSARY-BUT-NOT-SUFFICIENT:** A settings.json entry only suppresses the Claude Code
   permission prompt. The 9 `^Bash$` PreToolUse dispatcher guards fire regardless. Option C
   requires the same guard amendments as Option B; it adds the standing-permission surface
   with no corresponding governance benefit.
3. **Post-migration cleanup burden:** The entry must be actively removed from settings.json
   after migration, or it becomes an indefinite permission with no associated authorization.

#### Rejected-alternatives summary

| Criterion | Option A (hook-driven) | Option B (one-time-interactive) [CHOSEN] | Option C (hardened allowlist) |
|-----------|----------------------|----------------------------------------|------------------------------|
| Least privilege | Best (no permission change) | Best (no standing entry) | Weakest (standing entry persists) |
| F4 human gate | Via armed-marker (indirect but valid) | Direct (permission prompt at activation) | Via allowlist install (F3 fails) |
| D-449(a) evidence | Via census subcommand | Direct stdout | Direct stdout |
| New infrastructure | New hook plugin + registry entry | None | None |
| Guard amendments | Same 3 as Option B | Same 3 | Same 3 |
| Viability | Confirmed (conflation corrected) | Confirmed | Conditional (security-viable if hardened) |
| Selected? | No (complexity) | **Yes** | No (F3 fails) |

### Why this is a narrowly-authorized exception, not "the opposite of a bypass"

v1.0's "OPPOSITE of a bypass" framing was incorrect (Codex CV-ADR052-F1). A bypass is defined
by the MECHANISM (writing to `.factory/` outside the Edit/Write hook surface), not by the
quality of the bypassing code. The migration binary writes to `.factory/` outside the
Edit/Write surface — that IS a bypass, mechanically. What makes it authorized is:
1. POLICY 22 human ratification
2. The binary's own internal controls are enumerated and verified as equivalent to the skipped
   controls (§Decision 8)
3. The exception is narrowly scoped (two subcommands, one activation, expires on completion)

This framing is both more honest and more defensible under adversarial review.

## Consequences

### Positive

- Resolves the internally contradictory execution path in S-25.06 Rule 7.
- Option B provides the lowest standing-permission surface of any viable mechanism: zero
  persistent Bash permissions after migration completes.
- D-449(a) census-report-to-stdout satisfies the burst-log Dim-2 literal-shell-execution-
  evidence obligation with zero additional tooling.
- Factory-artifacts commit provides a tamper-evident audit record satisfying NIST AU-9.
- Activation-time ARCH-INDEX parity check closes the CI-vs-live parity gap (Codex F7).
- Crash-atomicity phase markers (§Decision 7) satisfy BC-1.18.011 Invariant 3's all-or-nothing
  guarantee across crashes.
- Explicit policy exception (§Decision 8) is honest, auditable, and bounded.

### Negative

- One-time interactive approval requires operator presence at F4 activation; cannot be
  automated without reverting to a standing allowlist (which is intentionally excluded).
- Guard amendments to `destructive-command-guard`, `validate-factory-path-staging`, and
  `validate-heavy-op-delegation` add implementation work to cluster-5 activation.
- The migration binary produces no PostToolUse hook event (it is a Bash invocation, not an
  Edit/Write). Mitigations: (a) census stdout IS correctness evidence; (b) the factory-artifacts
  commit IS the tamper-evident audit record; (c) `validate-factory-path-staged` PostToolUse
  guard is explicitly waivered with devops-engineer manual commit as equivalent.
- The CLAUDE.md amendment must be applied by the human at ratification — it cannot be applied
  by any agent (CLAUDE.md is human-mandated-edit-only).

### Neutral

- BC-1.18.011's crash-atomicity hardening (§Downstream to Product-Owner) is required before
  cluster-5 TDD implementation to ensure the spec the implementer works from is
  production-grade.
- S-25.06 Rule 7 text change is a correction of a false constraint, not a behavioral change.

## Downstream to Product-Owner (BC-1.18.011 and BC-1.18.010 hardening)

The architect specifies the following amendments; the product-owner writes all BC body changes.
Do NOT modify BC content directly; route to product-owner.

### BC-1.18.011 required amendments (ordered by precedence)

**Amendment 1 — Precondition 4 (Codex F6 / A/B2 scheduling coupling removal):**

Replace the current Precondition 4 text with:

> "The migration is independently gated on the F4 activation boundary. It has NO timing or
> ordering dependency on BC-1.18.008's mechanism-A backfill-split; the two migrations activate
> independently (each via its own armed-activation manifest per ADR-052 §Decision 4) and may
> run in any order. They share an F4 activation window by operational convenience, not by
> specification."

**Amendment 2 — Postcondition 6 (Codex F6 / A/B2 coupling in PC6):**

In the final sentence of Postcondition 6, replace "at the SAME F4 activation moment mechanism
A's own backfill (BC-1.18.008) runs" with "at F4 activation, as part of the same one-time B2
migration operation (independently of mechanism A's activation schedule)." The requirement that
SS-05/SS-06 sub-split occurs WITHIN the same B2 operation (not a separate follow-on) is
PRESERVED; only the A/B2 simultaneous-activation coupling is removed.

**Amendment 3 — Postcondition 7 scope clarification (Codex F6 / Invariant 4 attribution):**

Append to the end of Postcondition 7:

> "Note: this postcondition governs B2/Cohort-B independence only. A/B2 scheduling independence
> (that mechanism A and B2 activate independently at F4) is governed by Precondition 4 [as
> amended by ADR-052 v1.1]."

**Amendment 4 — New Precondition 5: Phase markers (research Q5 Gap 3):**

Insert after Precondition 4 (now amended):

> "5. A durable phase-marker file at `.factory/migration-state/migrate-bc-index-state.json`
>    is written at each phase transition and survives crashes:
>    - PREPARED: all staging writes complete, source fingerprint recorded, all verification
>      checks pass — BC-INDEX.md original body untouched.
>    - COMMITTED: all atomic file replacements complete; this marker is the single commit-pointer
>      for the multi-file atomic operation (ADR-052 §Decision 7).
>    - CLEANED: staging temporaries removed.
>    EC-003 resume logic reads this marker to determine the correct entry point without
>    re-running the full split from scratch."

**Amendment 5 — New Precondition 6: Writer exclusion / maintenance boundary (research Q5 Gap 5
and Codex F4):**

Insert after Amendment 4:

> "6. A WRITER-EXCLUSION maintenance boundary is in force during migration execution. The
>    migration binary acquires an exclusive advisory lock file at
>    `.factory/migration-state/exclusive.lock` before any read of source files. Ordinary governed
>    writers (Edit/Write tool calls validated by the `validate-factory-path-staging` dispatcher
>    guard) check for this lock file and fail with E-MAINTENANCE before writing to BC-INDEX paths
>    during a migration window. The lock is released only after the COMMITTED phase marker is
>    written (success) or on migration abort (cleanup). The `validate-factory-path-staging` guard
>    amendment (ADR-052 §Decision 5) implements the lock-check side of this invariant."

**Amendment 6 — Postcondition 3: Dir-fsync mandate (research Q5 Gap 2):**

Append to the existing Postcondition 3 atomicity text:

> "Each atomic file replacement (rename(2) call) MUST be followed by fsync on the parent
> directory of the target file before proceeding to the next replacement, ensuring directory
> entries survive a system crash (POSIX rename(2) + dir-fsync semantics per Pillai et al.
> OSDI'14). The dir-fsync is mandatory, not best-effort."

**Amendment 7 — New Postcondition 3a: TOCTOU pre-commit guard (research Q5 Gap 4):**

Insert immediately after Postcondition 3 (renumber subsequent postconditions if needed, or
insert as a named paragraph within Postcondition 3):

> "3a. Pre-commit source-fingerprint recheck (TOCTOU guard). Immediately before the
>     atomic-replace step (PREPARED → COMMITTED transition), the migration binary re-verifies
>     that BC-INDEX.md's source content has not changed since the census (Postcondition 2). It
>     re-computes SHA-256 of the source and compares against the fingerprint recorded at census
>     time. If they differ, the migration ABORTS: Postcondition 4's rollback applies, the
>     PREPARED marker is deleted, and BC-INDEX.md's original body is left untouched. This closes
>     the TOCTOU window between census and write introduced by concurrent governed writers or
>     human edits."

**Amendment 8 — Invariant 3: commit-pointer specification (research Q5 Gap 1):**

Append to the end of existing Invariant 3 text:

> "The all-or-nothing guarantee is implemented via the single COMMITTED phase marker
> (Precondition 5 [as amended]): before COMMITTED exists, the state is PREPARED (original body
> untouched, staging in progress) or CLEAN (no migration in progress); after COMMITTED is
> written, the state is the split end-state. The COMMITTED marker is the sole commit-point for
> the multi-file atomic operation; composing N independent `write_atomic` calls without this
> commit-pointer does not satisfy this invariant."

**Amendment 9 — Architecture Anchors additions:**

Add the following anchors to the Architecture Anchors section:

> "- ADR-052 §Decision 4 — armed-activation manifest governing pre-mutation authorization
>  - ADR-052 §Decision 7 — crash-atomicity phase markers (PREPARED/COMMITTED/CLEANED)
>  - ADR-052 §Decision 8 — POLICY 22 exception declaration with enumerated skipped controls"

### BC-1.18.010 required amendment

**Invariant 2 amendment (Codex F7 / research Q5 mapping gap):**

Replace the current Invariant 2 text with:

> "The BC-S-prefix→SS-NN mapping used for first-level addressing is validated against
> ARCH-INDEX's Subsystem Registry at CI time and at migration activation time, never
> independently hardcoded. At runtime, `shard_manager.rs` reads the mapping from the config
> entry only (a snapshot embedding the ARCH-INDEX commit SHA it was generated from, per
> ADR-052 §Decision 10). Two parity checks enforce that the snapshot never diverges from
> ARCH-INDEX: (a) a CI test (`arch_index_parity`) diffs the snapshot against HEAD ARCH-INDEX
> on every commit; (b) at migration activation, the binary verifies the live ARCH-INDEX matches
> the `approved_arch_index_sha` in the armed-activation manifest and fails CLOSED on mismatch.
> A future ARCH-INDEX subsystem renumbering propagates to this BC's addressing logic by
> regenerating the config snapshot from the new ARCH-INDEX commit, triggering a CI parity
> failure and requiring an explicit config update with the new SHA. This satisfies the 'never
> independently hardcoded' constraint: the snapshot is CI-validated, not independent."

## References

- `adv-cv-adr052-cluster5-F1-2026-09-12.md` — Codex RATIFY-WITH-CHANGES verdict (7 findings)
  that prompted this v1.1 revision
- `research-adr-052-assumption-validation-2026-09-12.md` — research-agent Q1-Q5 validation
  (converged with Codex on DO-NOT-RATIFY)
- `adv-cv-dir-cluster5-F1-direction-2026-09-12.md` — CV-DIR-F2 finding that prompted v1.0
- `s2502-cluster5-f1-delta-analysis.md` §4 — original options evaluation (v1.0 basis)
- `BC-1.18.011` — migration BC; §Downstream to Product-Owner specifies amendments
- `BC-1.18.010` Invariant 2 — amended per §Downstream to Product-Owner
- `S-25.06-append-log-backfill-split-executor.md` Rule 7 — corrected per §Decision 9
- `ADR-051` §Decision 1 (WASM fuel-budget constraint), §Decision 7 (B2 end-state),
  §Decision 10 (governed one-time B2 migration)
- `CLAUDE.md` §WASM plugin fuel budgets, §TD-FACTORY-HOOK-BYPASS-001 P0 (see §Decision 8
  for amendment text); human-only edit target — not modified by this ADR
- `executor.rs` `shard_cap_precheck` → `shard_manager::execute_roll` — existing native
  PreToolUse mutation pattern that proves Option A viability
- `plugins/vsdd-factory/hooks-registry.toml` — 13 `^Bash$` hook entries (9 PreToolUse);
  §Decision 5 enumerates required amendments
- NIST SP 800-53r5 AU-9 (audit trail tamper-evidence requirement)
- Pillai et al. OSDI'14 "All File Systems Are Not Created Equal" (dir-fsync requirement)
- CWE-88 (argument injection), CWE-78 (OS command injection), CWE-22 (path traversal)

## Files to Change

| File | Change | Owner |
|------|--------|-------|
| `CLAUDE.md` | Apply §Decision 8 amendment text to `TD-FACTORY-HOOK-BYPASS-001 P0` row in Forbidden patterns table | **Human only** (human-mandated-edit-only; NOT agent-editable) |
| `.claude/settings.json` | No change — no Bash allowlist entry added (Option B requires no settings.json mutation) | — |
| `.factory/specs/behavioral-contracts/ss-01/BC-1.18.011.md` | Apply §Downstream to Product-Owner Amendments 1-9 | product-owner |
| `.factory/specs/behavioral-contracts/ss-01/BC-1.18.010.md` | Apply §Downstream to Product-Owner Invariant 2 amendment | product-owner |
| `plugins/vsdd-factory/hooks-registry.toml` | Amend `destructive-command-guard`, `validate-factory-path-staging`, `validate-heavy-op-delegation` per §Decision 5; add writer-exclusion lock-check to `validate-factory-path-staging` per §Downstream Amendment 5 | devops-engineer (cluster-5 activation) |
| `plugins/vsdd-factory/hooks/destructive-command-guard.sh` (or equivalent WASM) | Add exception for absolute-path-pinned `factory-dispatcher migrate-bc-index` and `backfill-append-logs` | devops-engineer |
| `plugins/vsdd-factory/hooks/validate-factory-path-staging.sh` (or equivalent WASM) | Add pass-through for governed migration commands; add writer-exclusion lock-presence check | devops-engineer |
| `ADR-051` | Add §Decision 18 for activation-time ARCH-INDEX revision parity check mechanism | architect (same burst as ADR-052 ratification) |
| `S-25.06-append-log-backfill-split-executor.md` | Correct Rule 7 text per §Decision 9 | story-writer |
| `crates/factory-dispatcher/src/shard_manager.rs` | Add `run_mechanism_b2_bc_index_split` entry point; add `migrate-bc-index`/`backfill-append-logs` CLI subcommands + census stdout; implement phase markers and COMMITTED commit-pointer (§Decision 7); implement TOCTOU guard and dir-fsync; implement armed-activation manifest validation (§Decision 4); implement writer-exclusion lock acquire/release | implementer (cluster-5 TDD) |
| `crates/factory-dispatcher/tests/` | Add `arch_index_parity` CI test; add `stale_installed_config` test (§Decision 10); add activation-manifest-validation tests; add phase-marker resume tests; add TOCTOU-guard tests; add concurrent-write interlock tests | implementer (cluster-5 TDD) |
| `.factory/activation/` (new directory) | Created at F4 by state-manager when writing the armed-activation manifest | state-manager (F4 activation burst) |
| `.factory/migration-state/` (new directory) | Created by migration binary at runtime for phase markers and exclusion lock | migration binary (runtime) |
| `.factory/migration-audit/` (new directory under factory-artifacts worktree) | Created by state-manager when committing the durable audit record (§Decision 6) | state-manager (post-migration burst) |

## Changelog

| Version | Date | Author | Change |
|---------|------|--------|--------|
| 1.1 | 2026-09-12 | architect | Full revision per D-1214 (Codex RATIFY-WITH-CHANGES 7 findings + research Q1-Q5). Mechanism changed from Option C (hardened standing allowlist) to Option B (one-time interactive Bash approval at F4, no settings.json change). Option A re-evaluated honestly: explicitly-armed hook-driven alternative IS viable — `shard_cap_precheck` → `shard_manager::execute_roll` native PreToolUse mutation pattern in `executor.rs` proves the hook-internal execution model works; rejected only on operational-complexity grounds. Declares explicit narrowly-authorized policy exception (§Decision 8) with enumerated skipped controls and waivers — replaces false "OPPOSITE of a bypass" framing. CLAUDE.md amendment text specified for human application. Activation-manifest authorization mechanism added (§Decision 4) decoupling permission from authorization. 9 PreToolUse `^Bash$` dispatcher guards enumerated with required amendment status (§Decision 5). Audit trail upgraded to factory-artifacts commit satisfying NIST AU-9 (§Decision 6). Crash-atomicity phase markers, TOCTOU pre-commit guard, dir-fsync mandate added (§Decision 7). Config snapshot bound to ARCH-INDEX revision with activation-time parity check and stale-installed-config test (§Decision 10). BC-1.18.011 Amendments 1-9 and BC-1.18.010 Invariant 2 amendment specified for product-owner. Corrects false claim about existing project Bash permissions (settings.json contains only enabledPlugins, verified). |
| 1.0 | 2026-09-12 | architect | Initial authoring. Resolves CV-DIR-F2 (impossible execution path). Specifies Bash-tool-with-allowlist as sanctioned invocation path for both mechanism-A (S-25.06) and mechanism-B2 (BC-1.18.011) migrations. NOT RATIFIED per D-1214. |
