---
document_type: architecture-decision-record
level: L3
adr_id: ADR-035
version: "1.2"
title: "ADR-035: Cross-site correspondence validation — three-tier architecture, fuel error taxonomy, and wasmtime version target"
status: proposed
date: 2026-07-30
producer: architect
timestamp: 2026-07-30T00:00:00Z
deciders:
  - architect
supersedes: null
superseded_by: null
traces_to: .factory/specs/architecture/ARCH-INDEX.md
related_adrs:
  - ADR-034 (CI gate product-branch operand isolation — §Decision 5 Permitted/Forbidden pattern for cross-branch reads governs Tier 3 placement; this ADR establishes the architecture that §Decision 5's "Spec-validation: Permitted" class belongs to)
  - ADR-027 (factory-artifacts worktree path discipline — canonical .factory/ mount convention referenced by Tier 3)
  - ADR-003 (WASI preview-1 — WASM plugin host function surface referenced by Tier 1/2A)
  - ADR-042 (fuel budget raise to 20M and loud exhaustion signaling — discharges D-945; raises InvokeLimits::default() fuel_cap 10M→20M; corrects §Decision 5 O(n) confirmation and ~100KB threshold)
anchors:
  - SS-01
  - SS-05
subsystems_affected:
  - SS-01
  - SS-05
last_amended: "2026-10-07 (v1.2) — AMENDED (architect; wasmtime security move): §Decision 6 rewritten — target 48.x (>= 48.0.4; 48 is an LTS major) for BOTH wasmtime and wasmtime-wasi, clearing RUSTSEC-2026-0316/0314/0321/0322/0323/0324/0327 (46.x EOL, no patch; cargo-deny gate failing on every PR); 49.x deferred to a separate planned toolchain-upgrade (needs rustc 1.96 vs pinned 1.95); the v1.0/v1.1 \"47\" immediate target is superseded (workspace actually moved 44 -> 46.0.2/46.0.3 via S-21.12/E-22); §Alternatives LTS-pin entry re-targeted from 47.0.x to 48.x. [Prior: 2026-08-08 (v1.1) — AMENDED (architect, ADR-042): §Decision 5 fuel error taxonomy corrected — O(n) linear scan confirmed (R²=0.998790); ~100KB threshold corrected to ~136KB (10M cap) / ~327KB (20M cap); fuel budget raised to 20M per ADR-042 §Decision 1; loud exhaustion signaling mandated per ADR-042 §Decision 3; D-945 discharged. [Prior: 2026-07-30 (v1.0) — initial ruling (architect): three-tier cross-site correspondence validation architecture; fuel error taxonomy; wasmtime version target. Addresses 19 production failures across passes 28-30.]"
modified:
  - "2026-07-30 (v1.0)"
  - "2026-08-08 (v1.1)"
  - "2026-10-07 (v1.2)"
---

# ADR-035: Cross-site correspondence validation — three-tier architecture, fuel error taxonomy, and wasmtime version target

## Context

Nineteen production failures across six adversarial passes (28–30) share one shape: a datum was
updated at its authoritative site and left stale at one or more secondary sites. In every failure
the secondary site existed; only its value was wrong. The cluster per bump:

- `.factory/specs/behavioral-contracts/BC-INDEX.md` version cell
- `.factory/stories/STORY-INDEX.md` story-version cell, BC-version-pin cell, input-hash catalog
  row, and aggregation blockquote
- Story body Token Budget version cite

`lint_hook: null` on the governing policies (POLICY 14 leg 5, POLICY 18) meant no mechanical
gate fired. BC-5.39.010 v1.1 (product-owner, pre-pass-30 fix-burst, uncommitted) specifies WASM
PostToolUse hooks for Classes A and B. This ADR rules on the overall three-tier architecture
that BC-5.39.010 fits into, and on two implementation concerns raised during architectural review.

**Prior research summary (inputs, not gospel):** No mature documentation system validates
replicated scalars — they eliminate replication via single-source-plus-projection (Sphinx,
Antora, OpenAPI `$ref`, Sphinx-Needs generated tables). Mature systems layer an incremental
path AND a full-revalidation escape hatch (Sphinx `-E`/`-a`, TypeScript `--force`). The
incremental-only failure (sphinx#10416: a build with an invalid reference omits the warning
because the file was not rebuilt) is the canonical staleness hazard. WASI `fd_readdir` exists
in preview-1 but paginated enumeration is O(n²) in fuel; a narrow `list_matching(glob)` import
is the POLA-correct alternative if directory enumeration becomes necessary.

## Decision

### Decision 1 — Three-tier architecture for cross-site correspondence validation

Three tiers, ordered by scope and trigger:

| Tier | Mechanism | Scope |
|------|-----------|-------|
| 1 | WASM PostToolUse — file-local | Class E: frontmatter `version:` ↔ `last_amended:` text-prefix ↔ `modified[]` monotonicity |
| 2A | WASM PostToolUse — cross-site validation | Classes A and B: BC-INDEX version-cite + story BC-citation currency (Arm1/Arm2); STORY-INDEX three-way input-hash equality |
| 2B | Rust binary, operator-triggered | Future generation layer: regenerates BC-INDEX version cells and STORY-INDEX derived cells from authoritative BC/story frontmatter — complements 2A, never replaces it |
| 3 | bats spec-validation suite (CI-mounted `.factory/`) | Class C: count/enumeration parity; Class D-semantic: finding-ID existence; whole-corpus invariants |

Tier 2A is what BC-5.39.010 v1.1 implements. It is the correct mechanism for the immediate
problem: it fires within the edit loop, catches staleness before commit, and does not require
generator infrastructure. Tier 2B is an architectural commitment to a future generation layer
that reduces the manual obligation. Its absence does not degrade Tier 2A.

### Decision 2 — Authoritative source declarations for derivable cells

The following sources are unambiguously authoritative over their downstream cells. This
establishes the standing rule implicit in POLICY 14 leg 5 for code-generation purposes:

| Authoritative source | Derivable secondary cell |
|---------------------|--------------------------|
| BC file frontmatter `version:` | BC-INDEX body-table version column for that BC |
| BC file H1 heading | BC-INDEX body-table title column (verbatim, per POLICY 7) |
| Story frontmatter `version:` | STORY-INDEX catalog row story-version cell |
| Story frontmatter `input-hash:` | STORY-INDEX catalog row input-hash cell AND aggregation blockquote `S-NNN=` |

**NOT derivable (authored records):**
- STORY-INDEX BC-version-pin cell (records which BC version the story's ACs trace to; updated by
  story-writer at reconciliation time, not auto-derived from current BC frontmatter — see
  Rationale section)
- STORY-INDEX pass-refs column (append-only adversary pass record)
- Story body Token Budget narrative (only the version token within a row is a derivable cell;
  the surrounding narrative is authored)

This distinction governs Tier 2B scope: the generator produces only derivable cells; authored
records remain manual.

### Decision 3 — Tier 2B: generation mechanism, trigger, and scope

When built, Tier 2B MUST be a Rust binary (POLICY 21 compliance) named `generate-index-rows`.
State-manager invokes it explicitly at burst commit-preparation time before staging `.factory/`
artifacts. It is NOT a PostToolUse hook — WASM hooks are read-only validators per BC-5.39.010
Invariant 1; the generation step is a write operation.

**Scope:**
- Full BC-INDEX body-table: regenerate all rows from BC frontmatter + H1 + subsystem directory
  structure. Preserves section ordering and row ordering from the section directory. Row format
  defined by BC-INDEX's own column schema. One-shot regeneration (O(n) file reads; 1,983 BCs ×
  ~5KB ≈ ~10MB; fast for a Rust binary).
- STORY-INDEX story-version cells and input-hash cells: targeted in-place update of only touched
  stories' catalog rows and blockquote tokens.

**Out of scope for Tier 2B:** story Token Budget row version tokens (mixed authored/derived rows
require per-story parsing; Tier 2A Arm2 already catches staleness here at edit time).

**Incremental adoption:** Tier 2B is incrementally adoptable — regenerate only rows for
artifacts touched in a burst. Full-corpus regeneration as a pre-push escape hatch MUST always
be available.

### Decision 4 — Tier 3: home, scope, and staleness mitigation

Tier 3 MUST live in a bats spec-validation suite that reads `.factory/` via the `Mount factory
artifacts` step, consistent with ADR-034 §Decision 5's "Spec-validation: Permitted"
classification. The test subject is spec document invariants (counts, enumerations, finding-ID
existence) whose authoritative content lives on `factory-artifacts` by design.

**Scope:**
- Class C (count/enumeration parity): parameterized Rust workspace integration test invoked by
  bats. Inputs: per-file fixtures. Assertions: (i) count enumeration items by structural anchor;
  (ii) extract stated count by stable anchor; (iii) assert equality. POLICY 21 compliant.
- Class D-semantic (finding-ID existence): scan `adv-cycle-pass-*.md` files for cited `F-` IDs;
  assert each cited ID appears in at least one pass record. POLICY 21 compliant.

**Staleness mitigation (sphinx#10416 analog):** bats does not compile per-file incremental
caches, so the sphinx staleness hazard does not apply in its exact form. However:

1. The Tier 3 suite MUST be written as a full-corpus scan, not per-file incremental.
2. Any future incremental optimization MUST include a force-full escape hatch
   (`FORCE_FULL=1` environment variable or equivalent) that re-runs all invariant checks
   from scratch.
3. The `bats-full-suite` CI job that mounts `origin/factory-artifacts` at tip MUST always run
   Tier 3. The mount is unpinned to tip by design (spec content evolves; CI validates current
   state).

### Decision 5 — Fuel error taxonomy

**The defect:** Large artifacts (BC-INDEX.md, `lessons.md` >3,000 lines) can exhaust the
default 10M-instruction fuel cap during WASM plugin execution. The `handle_plugin_err`
function in `crates/factory-dispatcher/src/invoke.rs` maps `Trap::OutOfFuel` to
`PluginResult::Timeout { cause: TimeoutCause::Fuel }`. For plugins with `on_error = "block"`,
this `Timeout` surfaces as `block_intent=true exit_code=2` — a resource-policy failure
appearing as a validation blocking signal. For `on_error = "continue"` plugins, the hook
silently produces no findings (no advisory emitted to indicate validation was skipped).

**Taxonomy ruling:**

`TimeoutCause::Fuel` is a **resource-policy error**, not a validation result. Two required
fixes:

1. **`on_error = "block"` audit**: Any hook with `on_error = "block"` that reads `.factory/`
   artifacts larger than ~100KB is at risk of spurious fuel-exhaustion blocks. The two current
   blocking hooks are `validate-factory-path-root` and `validate-input-hash`
   (both legacy-bash-adapter). Route to implementer: audit whether either reads large artifacts
   during the bash subprocess; if so, either raise their per-plugin fuel cap (via registry
   `fuel_cap` field — see next point) or set them to `on_error = "continue"` with explicit
   fuel-exhaustion advisory.

2. **Fuel exhaustion advisory emission**: When any plugin produces `TimeoutCause::Fuel`, the
   dispatcher MUST emit a host-level advisory log (distinct from the plugin's own output):
   `"[fuel-exhausted] plugin <name> ran out of fuel after <N> instructions — validation
   skipped; this is a resource-policy event, not a finding."`. This makes silent validation
   bypass observable. Route to implementer.

**Registry per-plugin fuel_cap:** `InvokeLimits::default()` in
`crates/factory-dispatcher/src/invoke.rs` sets `fuel_cap: 10_000_000`. This field exists in
code but is not exposed in `hooks-registry.toml`. The registry schema MUST be extended with
an optional `fuel_cap` integer field per `[[hooks]]` entry so operators can tune per-plugin
fuel without source changes. Route to implementer.

**New validate-cross-site-correspondence hook (BC-5.39.010):** Already correctly designed —
`on_error = "continue"` in the Gate Spec, and `max_bytes` limits sized to keep artifact reads
within the 10M fuel budget. No registry change needed for this new hook; the `fuel_cap` field
exposure is for future tuning and the existing `on_error = "continue"` handles the fuel-
exhaustion case correctly.

---

**AMENDED 2026-08-08 (v1.1 — architect, ADR-042):**

Three corrections apply to §Decision 5, based on measured benchmarks from S-21.07 pass-9 (performance-engineer, 2026-08-08):

1. **O(n) confirmed for `validate-cross-site-correspondence`, O(n²) concern resolved.**
   The O(n²) concern in §Context was about directory enumeration via `fd_readdir` (a rejected
   alternative). The implemented `read_file` linear scan is confirmed **O(n) in input bytes**:
   regression `fuel = 2,585,970 + 53.18 × var_bytes`, R²=0.998790 over 24 measured points.
   Quadratic coefficient 5.42×10⁻⁵ adds only +0.075% R² — fuel cost is linear, not superlinear.

2. **~100KB threshold corrected to ~136KB (10M cap) / ~327KB (20M cap).**
   At 53.18 fuel/byte marginal, the breakeven (budget consumed at cap): `(cap − 2,585,970) / 53.18`.
   At 10M: ≈139,388 bytes ≈ 136KB. At 20M: ≈326,782 bytes ≈ 327KB.
   The measured exhaustion threshold for `validate-cross-site-correspondence` was ~415KB prefix
   (BC-5.39.010 at BC-INDEX line 1464, ~415,278 byte prefix). The ~100KB guidance understated
   the risk zone by ~4×. The new ~136KB / ~327KB figures are derived from the measured model.

3. **Fuel budget raised to 20M; exhaustion signaling mandated; D-945 discharged.**
   `InvokeLimits::default()` `fuel_cap` has been raised from 10,000,000 to 20,000,000 (ADR-042
   §Decision 1). The "no registry change needed for this new hook" note is superseded: a
   per-plugin `fuel_cap` SHOULD be set once ADR-039 Phase 1 (registry schema extension) ships
   (ADR-042 §Decision 2). Loud exhaustion signaling via dispatcher stderr summary line +
   `advisories[]` payload is mandated (ADR-042 §Decision 3). Standing drift item [D-945]
   "ADR-035 §Decision 5 fuel budget advisory — OPEN 2026-07-30 — 'May need revision after
   S-21.07 benchmarks'" is **DISCHARGED** by ADR-042. See ADR-042 §Decision 4 for full
   corrections and derivation evidence.

**Preferred remedy order (per prior art):** (1) host-side input cap (`max_bytes` in
`read_file`) — already implemented in BC-5.39.010; (2) epoch deadline as outer bound —
already implemented via `EpochTicker` at 10ms/tick in `crates/factory-dispatcher/src/engine.rs`;
(3) per-plugin `fuel_cap` in registry — for tuning after the field is exposed.

### Decision 6 — wasmtime version target

**v1.2 (2026-10-07) — current ruling.** Workspace `wasmtime` and `wasmtime-wasi` move to the
**48.x line, minimum 48.0.4** (`"48.0.4"`, resolving to the latest 48.0.x patch; lockstep for both
crates). 48 is an LTS major (LTS cadence: majors divisible by 12 at 24-month intervals — 24.x, 36.x,
48.x), so the dispatcher's release-bundle longevity requirement is met.

**Why:** 46.x is end-of-life with no patch release, and it carries open advisories
RUSTSEC-2026-0316, -0314, -0321, -0322, -0323, -0324 and -0327. The `cargo-deny` advisories CI gate
(added by S-21.12) fails on every PR while 46.x is pinned. 48.0.4+ clears all seven. Moving to 48
rather than 47 skips a non-LTS interim major, which would itself reach EOL within the cadence window
and require another forced move.

**Why not 49.x:** 49.x requires rustc 1.96; the workspace toolchain is pinned at 1.95
(`rust-toolchain.toml` / per-crate `rust-version`; single-workspace-MSRV rule). Moving the toolchain
is a cross-cutting change (CI images, MSRV, ASM-02 operator toolchain floor, hook-plugin WASM
builds) and is a **separate planned toolchain-upgrade story**, not part of this security fix.

**API migration (46 -> 48), sandbox semantics MUST be unchanged:**
- `wasmtime_wasi::FilePerms` is renamed `FsPerms`.
- `WasiCtxBuilder::preopened_dir` takes 3 args (`host_path`, `guest_path`, `FsPerms`) instead of 4
  (`host_path`, `guest_path`, `DirPerms`, `FilePerms`).
- Behavior-preservation rule: the preopen grant remains the exact equivalent of the prior
  `DirPerms::all()` + `FilePerms::all()` (full read/write on the preopened directory). Widening or
  narrowing the plugin filesystem capability is NOT part of this change; any future SEC-001 preopen
  hardening (read-only preopens) is a separate decision and builds on the `FsPerms` API.
- Fuel/epoch surface (`build_engine`, `invoke_plugin`, `Trap::OutOfFuel` / `Trap::Interrupt`
  classification per ADR-047) must be re-verified green; no ABI change (`HOST_ABI_VERSION` stays 1).

Route to implementer (dependency bump + mechanical API rename) with the existing S-21.12
`Cargo.lock` version-gate test floor raised to `>= 48.0.4` (story-writer amends AC-008 / floor
references). Required verification before merge: `cargo test --workspace --all-targets`, the bats
suite, and `cargo deny check advisories` all green.

**History (superseded rulings).** v1.0/v1.1 ruled "bump `44.0` to `47`, then 48 LTS when released"
(RUSTSEC-2026-0149 out-of-support exposure). The workspace instead moved `44.0` -> `46.0.2` under
S-21.12 / E-22 (RUSTSEC-2026-0188 + RUSTSEC-2026-0222) and then `46.0.3`; the 47 target was never
adopted. This v1.2 ruling realizes the "migrate to 48 LTS" half of the original decision and makes
it the target directly.

## Rationale

**Why not replace Tier 2A (WASM validation) with Tier 2B (generation)?**
The research finding that mature systems prefer generation over validation applies to systems
DESIGNING their artifact structure from scratch. BC-5.39.010 v1.1 was authored against an
existing 1,983-row artifact structure. Replacing validation with generation before the
generator is built and proven would leave the failure class unguarded during the transition.
The correct sequence is: (1) ship Tier 2A validation now; (2) build Tier 2B generation later;
(3) keep Tier 2A as a backstop — a validator that catches generation failures is strictly safer
than a lone generator. The "layer, don't choose" prior art finding directly supports this
sequence.

**Why is the STORY-INDEX BC-version-pin excluded from generation?**
The pin records the BC version against which the story's ACs were RECONCILED — it is a quality
assertion, not a datum copy. Generating it from current BC frontmatter would erase the
reconciliation semantics: a story could silently appear reconciled against BC v1.18 while its
ACs still trace to v1.15 patterns. The pin must be updated by story-writer during actual
reconciliation work, making it an authored record.

**Why Tier 3 in bats rather than a standalone CI job?**
ADR-034 §Decision 5 establishes that spec-validation suites reading `.factory/` for spec
content are a Permitted cross-branch read pattern. The existing `bats-full-suite` CI job
already mounts `.factory/` and runs on every PR. Running Tier 3 via the existing harness
avoids introducing a second CI mechanism with independent mount logic. Rust workspace
integration tests are POLICY 21 compliant and are already the pattern for Class C
(see BC-5.39.010 §Honest Gap).

**On the fuel error taxonomy:** `TimeoutCause::Fuel` is already a distinct enum variant in
the dispatcher source — the fix is NOT architectural restructuring but (a) audit of
`on_error = "block"` hooks for large-artifact fuel risk, and (b) advisory emission on fuel
exhaustion. The underlying design (epoch + fuel dual-mechanism per `build_engine`) is correct
and requires no change.

## Consequences

### Positive

- BC-5.39.010 v1.1 Parts A, B, and E are confirmed as the correct Tier 2A + Tier 1
  implementation. No behavioral revision required from this ADR.
- Tier 2B generation provides a future path to eliminating the manual obligation entirely
  without requiring it to be built before shipping the validator.
- Fuel exhaustion becomes observable (advisory log) rather than a silent validation bypass.
- wasmtime upgrade to 48.x (LTS) resolves out-of-support exposure (and, per v1.2, RUSTSEC-2026-0316/0314/0321/0322/0323/0324/0327) and aligns the dispatcher with the LTS
  release track.
- STORY-INDEX BC-version-pin exclusion from generation preserves reconciliation semantics,
  preventing a class of false "reconciled" appearances.

### Negative / Trade-offs

- Tier 2B requires future development effort (Rust binary, story TBD).
- Tier 3 requires POLICY 21-compliant Rust workspace tests alongside gate specifications
  (Class C per-file fixtures, Class D-semantic scan logic).
- wasmtime 46→48 migration (v1.2; originally 44→47) requires the `FilePerms`→`FsPerms` /
  3-arg `preopened_dir` source update in `crates/factory-dispatcher` and verification that sandbox
  semantics are unchanged; 49.x remains blocked on a separate rustc 1.96 toolchain upgrade.
- Per-plugin `fuel_cap` registry field requires a TOML schema extension before it can be used
  without source changes.

## Alternatives Considered

**Replace Tier 2A (WASM validation) entirely with Tier 2B (generation):**
Ship only the generator; remove the validation hook. Rejected. The generator does not yet exist
and building it correctly for 1,983 BCs requires non-trivial development effort. Shipping
BC-5.39.010 v1.1 now and building the generator later is strictly safer. A validator that
catches generation failures is more valuable than a lone generator with no backstop. Deferred,
not rejected permanently — Tier 2B is a planned future layer.

**Put Tier 3 in a standalone CI job (not bats-full-suite):**
A dedicated CI job mounting `.factory/` for Tier 3 only. Rejected as unnecessary duplication.
`bats-full-suite` already mounts `.factory/` on every PR. Adding a second mount mechanism
creates maintenance overhead with no additional correctness guarantee. Tier 3 tests invoked by
the existing bats harness are sufficient.

**Set `on_error = "block"` on `validate-cross-site-correspondence` to catch fuel exhaustion
loudly:** Fuel exhaustion would then surface as `block_intent=true`, making validation bypass
visible. Rejected. Blocking on fuel exhaustion conflates resource-policy failures with
validation results. Operators would have no way to distinguish "validator found a real
violation" from "validator ran out of fuel." The correct fix is observable advisory emission
(§Decision 5), not a severity escalation that misrepresents the failure type.

**Pin to wasmtime LTS 36.0.x instead of upgrading to 48.x (v1.2: re-targeted from 47.0.x):**
Downgrade to the prior LTS for maximum support window. Rejected. A downgrade introduces API
regression risk (multiple majors of WASI preview-1 surface change) with uncertain gain — 36.0.x
support is already well into its 24-month window and would not clear the RUSTSEC-2026-0316/0314/
0321/0322/0323/0324/0327 advisories on the upgrade path the workspace already took (44 -> 46).
Upgrade to 48.0.4+ (the current LTS major) is the lower-risk path and provides the long-term
stable track directly.

**Use a narrow `list_matching(glob)` WASM host import for directory enumeration in Tier 2A:**
Allow the WASM hook to enumerate BC files directly for Arm2. Rejected. BC-5.39.010 v1.1 Arm2
derives BC file paths deterministically from BC IDs in the story's `behavioral_contracts:`
frontmatter — no directory enumeration needed. The `list_matching(glob)` pattern is the correct
solution IF enumeration were required, but it is not required for the current design.

## Downstream Routing

| Artifact | Change | Route |
|----------|--------|-------|
| BC-5.39.010 v1.1 Honest Gap section | Add note: fuel exhaustion silences the hook silently via `on_error = "continue"` at registry level; `max_bytes` limits are calibrated to keep artifact reads within 10M fuel budget for typical artifacts; fuel-exhausted invocation produces `TimeoutCause::Fuel` (no finding emitted, no block) — advisory emission added by implementer per §Decision 5 | product-owner (assess whether warrants v1.2 bump) |
| `crates/factory-dispatcher/src/invoke.rs` | In `handle_plugin_err`, when `TimeoutCause::Fuel` is produced, emit host-level advisory log before returning `PluginResult::Timeout` | implementer |
| `plugins/vsdd-factory/hooks-registry.toml` | (1) Audit `validate-factory-path-root` and `validate-input-hash` for large-artifact fuel risk; (2) add optional `fuel_cap` integer field to `[[hooks]]` schema | implementer |
| Root `Cargo.toml` workspace dependencies | Bump `wasmtime` and `wasmtime-wasi` to `"48.0.4"` (v1.2; was `"44.0"` -> `"47"` in v1.0/v1.1; workspace currently `"46.0.3"`), with `FilePerms`→`FsPerms` + 3-arg `preopened_dir` migration | implementer |
| Tier 2B generator | New story: `generate-index-rows` Rust binary for BC-INDEX + STORY-INDEX derived cells | story-writer (TBD story ID) |
| Tier 3 bats suite | New story: Class C count/enumeration + Class D-semantic finding-ID Rust workspace tests invoked by bats-full-suite | story-writer (TBD story ID) |
| ARCH-INDEX.md | Insert ADR-035 row in decisions table | state-manager (post-ruling burst) |

## Source / Origin

- Passes 28–30 adversarial cascade on S-21.04: 19 production failures, single shape
- BC-5.39.010 v1.1 (product-owner, uncommitted, 2026-07-30): specifies WASM validation for
  Classes A, B, D, E; honest gap for Class C
- ADR-034 §Decision 5: Permitted/Forbidden/Advisory classification for cross-branch reads in
  bats; Tier 3's placement is consistent with the Permitted (spec-validation) class
- `handle_plugin_err` function in `crates/factory-dispatcher/src/invoke.rs`: maps
  `Trap::OutOfFuel` to `PluginResult::Timeout { cause: TimeoutCause::Fuel }` — fuel
  exhaustion already distinct in code from wall-clock timeout
- `InvokeLimits::default()` in `crates/factory-dispatcher/src/invoke.rs`:
  `fuel_cap: 10_000_000` default — per-plugin tuning not yet exposed in registry
- `[workspace.dependencies]` in root `Cargo.toml`: `wasmtime = "44.0"` (v1.0 observation; out of support at the time — workspace subsequently moved to 46.0.3 and, per v1.2, to 48.0.4+)
- `plugins/vsdd-factory/hooks-registry.toml`: `on_error = "block"` on
  `validate-factory-path-root` and `validate-input-hash`
