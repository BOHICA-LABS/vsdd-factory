# Changelog

## [Unreleased]

<!-- Story PRs add their changelog rows here under ### Added / ### Changed /
     ### Fixed. Drained into the next `## <version>` section at release time
     (RELEASING.md Step 2). Keep this heading in place and empty after a drain. -->

### Security

- **wasmtime + wasmtime-wasi 46.0.3 → 48.0.4 (resolves 48.0.5)**: clears RUSTSEC-2026-0316, RUSTSEC-2026-0314, RUSTSEC-2026-0321, RUSTSEC-2026-0322, RUSTSEC-2026-0323, RUSTSEC-2026-0324 and RUSTSEC-2026-0327 so the `cargo deny check advisories` CI gate passes with `ignore = []` (no deny.toml ignores added). Adapts `factory-dispatcher` to the wasmtime-wasi 48 API (`DirPerms`/`FilePerms` unified into `FsPerms`; `preopened_dir` takes 3 arguments). The project-directory preopen keeps identical read+write semantics (`FsPerms::ReadWrite`); no sandbox widening or tightening. Also updates `chacha20` 0.10.1 → 0.10.2 (yanked release). Wasmtime 49.x requires rustc 1.96 and is out of scope for the pinned 1.95 toolchain.

- **Dispatcher engine: component model explicitly disabled (ADR-003)**: `build_engine` now calls `config.wasm_component_model(false)` so component binaries are rejected at compile time (defence in depth against the wasmtime 48 default); the dispatcher runs WASI preview-1 core modules only.

### Added

- **S-25.04 — Close `validate-factory-path-staging` zero-enforcement gap: real Layer-1 production trigger** (BC-4.16.002, BC-1.18.001 v1.7, BC-1.18.004 v1.4): Layer-1 validation-integrity: real production enforcement trigger for `validate-factory-path-staging` via new PostToolUse companion validator `validate-factory-path-staged` (closes S-25.01/ADR-047 zero-enforcement gap). New WASM plugin crate `crates/hook-plugins/validate-factory-path-staged/` — the post-hoc detective mirror of `validate-factory-path-staging` (BC-4.16.001): fires unconditionally on every completed `PostToolUse ^Bash$` dispatch (BROAD trigger scope, no command-text pre-filter) and inspects the actual git index/branch state (`git diff --cached --name-only` then `git branch --show-current`) to detect a `.factory/` path staged on a product branch that evaded the preventive PreToolUse guard (git plumbing, alias, wrapper script, under-matched invocation text). Registered `priority = 161`, `failure_policy = "fail-closed"`, `on_error = "continue"` — this is the first Layer-1 validator whose fail-closed assignment reaches a structurally-reachable PostToolUse trigger for `write_indeterminate_marker` (S-25.01's marker/gate machinery, reused verbatim — no new marker file or gate mechanism). `validate-factory-path-staging`'s own PreToolUse registration is unchanged. Layer-1 effective fail-closed count increases from ZERO to N≥1, attributable entirely to `validate-factory-path-staged`; `validate-factory-path-staging` remains at ZERO observable enforcement effect per its own unchanged PreToolUse-only registration (BC-1.18.004 v1.4 PC4; S-25.04 AC-003).

### Fixed

- **S-25.08 — Shared PreToolUse admission core for governed migrations wired on the production path + B2 conformance + VP-147 Kani re-baseline** (BC-1.18.011 v1.18, BC-1.18.013 v1.10, BC-3.08.001 v1.35; ADR-052 §5a, §7e and §Error Code Semantics, v1.21 as provenance; error-taxonomy `E-MAINTENANCE-001/002`, `RESERVATION_TTL_BELOW_FLOOR`, `FACTORY_ROOT_NOT_FOUND`, `MIGRATION_STATE_INTEGRITY_FAILURE`): The native writer-admission gate for `Edit`/`Write`/`MultiEdit` under `.factory/specs/behavioral-contracts/` and `.factory/cycles/` is now ONE shared core (`shard_manager::admit_protected_write`) evaluated exactly once per PreToolUse event, before `Registry::load`, `resolve_registry_path`, the `CLAUDE_PLUGIN_ROOT` tiering, `shard_cap_precheck` and every registry plugin, so a missing, unparseable or schema-mismatched registry can no longer let protected writers run unreserved. `Bash` is left unprocessed (no decision, no reservation, no state write).
  - **Reserve-then-verify and release.** The `reservations/<tool_use_id>.reservation` file is created first, then the gate and txn records are read, then the step-3.5 reconciliation runs and the decision is re-evaluated; no admitter lock. A blocked admission removes its own reservation, the dispatcher removes it when any later stage (shard-cap, plugin, registry fail-closed exit) ends the dispatch with exit 2, and the reservation is released on BOTH `PostToolUse` and `PostToolUseFailure` for the same `tool_use_id` (a failed tool call is delivered only as `PostToolUseFailure`). The reservation namespace is unconditional: admission no longer skips when `migration-state/` does not yet exist (first-activation race closed).
  - **Scope.** The gate guards the session's own `factory_root` (`<CLAUDE_PROJECT_DIR>/.factory`, must exist, never created); migration-state is always `<factory_root>/migration-state`. Classification is component-wise and case-insensitive over the shared `resolve_target_path` `(T_real, T_lex)` pair (POSIX left-to-right symlink walk, 40-hop limit, nonexistent tail applied lexically, unresolvable falls back to lexical; `\` is a separator only on Windows; relative paths join the payload `cwd`; no `~` expansion). A write is in scope when either form matches under either lexical root spelling (canonical `project_root/.factory` or the as-given absolute `CLAUDE_PROJECT_DIR`). Paths outside the root (another project's `.factory`, a nested project, a look-alike) are an admitted no-op: no reservation, no directory, no state read.
  - **Messages and errors.** `E-MAINTENANCE-001` is keyed on the written path family (`BC-INDEX` / `.factory/cycles/`) with the em-dash completion-record-mismatch suffix only for Branch C failures. NEW `E-MAINTENANCE-002: writer-admission check failed (<cause>)` (fail-closed, no reservation) with causes `invalid_tool_use_id` (a present but non-string/empty/grammar-violating id; byte length only, never the raw value), `io` (an OS read/create failure) and `state_integrity` (malformed or non-UTF-8 gate/txn record, non-string `migration_id`, more than one live txn). Gate and txn files are read as raw bytes then decoded, so invalid UTF-8 is `state_integrity`, not `io`. Untrusted ids and error text in admission diagnostics are escaped and length-capped.
  - **B2 conformance.** The stale-gate reconciliation is wired on the production path under `flock(exclusive.lock)`: Branch A (stuck gate, no live txn), Branch B (STAGING with a null `generation_id` is aborted in place with `abort_reason: "null_generation"` regardless of gate state, then the gate opens) and Branch C (pure `decide_terminal_record_reconciliation`; "foreign" means `migration_id` is not one of the two known migrations, and a live txn of either known migration is decided against its own record; the effectful verify-then-finalize remains a fail-closed seam pending S-25.06). `admit_or_block_bc_index_writer` delegates to the shared core (no check-then-reserve).
  - **Reservation TTL.** Default 3,600 s (was 120 s) with a 1,800 s production floor: below-floor configuration is `ReservationTtlBelowFloor` / `RESERVATION_TTL_BELOW_FLOOR`, raised at the very top of `run_bc_index_migration` before any lock or mutation, plus a compile-time guard. Staleness (`reservation_is_stale`) is `created_at`-first with a 300 s clock-skew tolerance: an unparseable, pre-epoch or far-future `created_at` falls back to mtime, a future mtime is retained, and an unknown age is never reclaimed (every ambiguity resolves toward retention, with a `reason`-tagged warning). The drain now fails closed when `reservations/` cannot be read, instead of reporting quiescence.
  - **Registry exit codes.** `registry_error_exit_code` maps every `RegistryError` variant explicitly (no wildcard arm: schema-version, async+block and duplicate-entry fail closed with exit 2; not-found, I/O, TOML and tool-regex fail open with exit 0) and `main.rs::run` delegates to it. The `executor::resolve_shard_gate_precedence` helper and its unit tests were removed: the ordering is now structural in `main::run` and pinned by a real-binary test.
  - **One anchoring rule.** `resolve_session_project_root` (a present non-empty `CLAUDE_PROJECT_DIR` canonicalized, falling back to the as-given path and never the cwd; else the process cwd; no ancestor walk) feeds `resolve_factory_root` for the dispatcher admission, the release leg AND the `migrate-bc-index` coordinator, which previously operated on the bare process cwd (so reservations and the drain could land in different directories). The coordinator derives every `.factory/…` path from the resolved real factory root, never creates `.factory`, and a missing `.factory` is the fail-closed `FACTORY_ROOT_NOT_FOUND` (exit 2; one stderr line `migrate-bc-index: FACTORY_ROOT_NOT_FOUND: no .factory directory under project root <root> (resolved from <source>)`, nothing created). `run_bc_index_migration` validates the production TTL as its first statement through a crate-private injectable seam (never public/env/argv).
  - **Diagnostics are events, not `tracing`.** The dispatcher installs no `tracing` subscriber, so admission/release/reconciliation diagnostics are returned as data by the shared core and written by `main.rs` to the dispatcher-internal JSONL log before the early return: `migration.admission_blocked` (one per `E-MAINTENANCE-001`; `branch` ∈ gate_only / live_txn / foreign_migration / live_coordinator / completion_record_mismatch), `migration.admission_failed` (one per `E-MAINTENANCE-002`; `cause` + `kind`) and `migration.admission_advisory` (Branch A gate re-open, Branch B txn discard, Branch C finalize seam, reservation-release failure). No event carries a raw `tool_use_id`, record content or `plugin_name`; disk-derived ids are escaped and capped at 64 characters. Coordinator anomalies go to stderr.
  - **Error classification.** Admission-side integrity failures raise a dedicated `AdmissionStateIntegrity { kind }` (gate/txn record malformed, non-string `migration_id`, multiple live txns, reservation serialization) instead of reusing `BinaryIntegrityFailure`; `admission_failure_cause` is an exhaustive match over every error variant. Verdicts are labelled `blocking_plugins=migration-admission` (a closed `NativeGate`), a shard-cap block keeps `shard-cap-gate`; the shared entry points have migration-neutral names (`migration_writer_admission`, `migration_writer_admission_precheck`, `migration_reservation_release`).
  - **VP-147.** The B2 Kani suite is re-baselined to 10 harnesses (CI `EXPECTED_PROOFS` 10).

## 1.0.0-rc.25 — dispatcher INDETERMINATE outcome layer-1 + wasmtime 46.0.3 sandbox-escape fix (2026-09-04)

**SECURITY (HIGH):** wasmtime + wasmtime-wasi bumped 46.0.2 → 46.0.3, clearing RUSTSEC-2026-0269 (HIGH — a sandbox-escape in the WASM hook-plugin runtime) and RUSTSEC-2026-0268 (MEDIUM). This is the primary reason to upgrade promptly. Alongside the security fix, this release ships Layer 1 of the dispatcher's new INDETERMINATE outcome class (ADR-047/ADR-048) — a durable, fail-loud marker that blocks the next Agent dispatch and `git commit`/`git push` when a fail-closed hook plugin cannot complete (fuel/epoch/output-too-large), the `last-amended-migrate` CLI for the durable `last_amended` write-path fix (S-15.03), the `factory-lock` identity-aware renewal chain (S-17.05/06/07, ADR-046), the `vsdd-factory:wrap` session-pause skill, and a CPU-runaway fix in the count-propagation validation hook.

### Added

- **S-25.01 — Dispatcher INDETERMINATE outcome class + durable mutation marker + next-advance gate** (BC-1.18.001–004, BC-3.08.001 Event 8, ADR-047 Layer 1, ADR-048): Dispatcher: INDETERMINATE outcome class for fail-closed plugins — fuel/epoch/OutputTooLarge cannot-complete events now write a durable `.factory/unvalidated-mutation.marker` (atomic write + 24h TTL deadman expiry); `validate-unvalidated-mutation-marker` gate blocks next Agent dispatch and `git commit`/`git push` while a non-expired marker exists (quote-aware, compound-command-aware `git commit`/`push` detection covering `-C`/`-c`/`--namespace`/`--git-dir`/`--work-tree`/`--config-env` global options and `&&`/`;`/`|`/`&` chaining); successful re-validation clears the marker (artifact-scoped: only clears for the same `artifact_path` the marker recorded, or unconditionally when the marker is non-artifact-scoped); existing ~76 fail-open plugins unchanged (advisory `plugin.indeterminate` event only, all 8 BC-3.08.001 Event-8 mandatory fields including `artifact_path`); Cohort A validators assigned fail-closed (1 effective today: `validate-factory-path-staging`; 2 latent: `validate-pr-merge-prerequisites` + `validate-wave-gate-prerequisite`, pending S-21.24 calibration per ADR-047 §D8a). New `on_error = "block_if_marker"` registry variant (ADR-048) added for the native crash/timeout path — 5 hooks-registry.toml entries now use the new INDETERMINATE-aware error semantics.

- **S-15.03 — `last_amended` write-path durable fix** (BC-10.13.001): New `last-amended-migrate` CLI crate (`crates/last-amended-migrate`) providing check-mode and register-mode migration of the legacy inlined `last_amended`-string frontmatter field toward the structured `changelog:` array, with YAML-injection gating (SEC-001), a path allowlist restricted to governed spec directories (SEC-002), atomic-write-with-rotation semantics (SEC-003), eligibility classification, and idempotency guarantees. Full bats + integration test coverage (14 dedicated test files) validates changelog bootstrap, escape hardening, rotation, registry parity, and STATE.md-split recovery paths.

- **S-24.01 — `vsdd-factory:wrap` skill** (BC-6.28.001): 7-step procedure for safe factory session pause, checkpoint, and lock release; resume guidance cites `/vsdd-factory:rehydrate-wave` before `/vsdd-factory:next-step`.

- **S-17.05 — `stamp-state-timestamp` PostToolUse WASM hook** (BC-4.17.001, ADR-046): New `stamp-state-timestamp` WASM hook plugin crate that stamps STATE.md's freshness timestamp on qualifying PostToolUse writes, sharing `TTL_SECONDS`/`STATE_MD_MAX_BYTES` constants promoted into `factory-lock-parse`.

- **S-17.06 — factory-lock shared identity-renewal functions** (BC-4.17.001): `renew_lock_if_holder` (6-outcome decision tree: Renewed / AlreadyExpired / NotHolder / NoOp / IdentityResolutionFailed / Malformed), `IdentityResolution`/`SkipReason` types, `classify_identity_resolution`, and `trim_git_email` added to `crates/factory-lock` as shared functions consumed by S-17.05/S-17.07, plus doc-comment corrections.

### Changed

- **S-17.07 — precompact-flush Step-4 identity gate** (BC-7.07.001 v1.40, S-17.07 v1.2, ADR-046 Decision 3/4): precompact-flush Step-4 lock renewal upgraded from the identity-blind `renew_lock` to `renew_lock_if_holder` — renewal is now skipped when the lock is expired (`AlreadyExpired`), when the current committer is not the lock holder (`NotHolder`), or when git identity resolution fails; `factory.lock.renewal_indeterminate` event emitted with 5-field payload (`plugin`, `holder`, `locked_at`, `expires_at`, `resolution_error`) on identity-resolution failure; `Malformed` lock blocks are advisory-logged and flush proceeds unblocked. `step4_renewal_gate<RI,WS,LW,EE,NF>` wired into `run_plugin_with_mock_and_cwd` with injectable `log_warn_fn`/`emit_event_fn` closures; production path delegates to `host::log_warn`/`host::emit_event`.

### Fixed

- **CPU-runaway CI/hook hang — count-propagation ID-strip + awk length guard** (#803): `validate-count-propagation`'s `_extract_counts` used an extglob substitution (`${line//+([A-Za-z])-+([0-9.])/}`) exhibiting O(n·k) backtracking behavior against the ~195KB single-line `last_amended:` blob in BC-INDEX.md (n = line length, k ≈ 8000 ID matches), pegging a full CPU core for 12+ seconds as a PostToolUse process before orphaning to PPID 1 and running indefinitely. Replaced with two O(n) whole-file pre-processing passes fed via process substitution: `awk 'length <= 8192'` drops the mega-line in one linear pass, then `sed -E 's/[A-Za-z]+-[0-9.]+//g'` strips ID tokens via a linear NFA with no backtracking possible.

- **precompact-flush TTL single-source** (F-WG5-P2-001, #801): Removed an independent TTL literal from precompact-flush; TTL is now single-sourced from `factory_lock_parse::TTL_SECONDS`, eliminating a drift vector between the lock crate and the flush plugin.

- **Mis-bundled `policy15-attestation-gate.wasm` orphan removed** (#786): `policy15-attestation-gate` is a native CLI binary invoked directly in CI (`cargo build --release -p policy15-attestation-gate`, executed as a host process) with no legitimate WASM target. It was accidentally swept into the rc.24 release bundle because the crate was missing from the `--exclude` list in `release.yml`'s workspace-wide `wasm32-wasip1` build step. The orphan `.wasm` (zero registry entries in either `hooks-registry.toml` or `resolvers-registry.toml`) is removed, and `release.yml` gains recurrence prevention so future workspace-wide wasm builds correctly exclude native-only crates.

### Security

- **wasmtime / wasmtime-wasi 46.0.2 → 46.0.3** (RUSTSEC-2026-0269 HIGH sandbox-escape; RUSTSEC-2026-0268 MEDIUM): Bumps the WASM sandbox runtime used by every hook plugin invocation. RUSTSEC-2026-0269 is a HIGH-severity sandbox-escape advisory in wasmtime's guest isolation; RUSTSEC-2026-0268 is a MEDIUM-severity companion advisory in the same dependency line. Both are cleared by this bump. Caught by the `cargo deny check advisories` CI gate introduced in rc.24.

### Operational

- **Bundled dispatcher binaries + registry re-synced for `block_if_marker`**: This release rebuilds the operator-level bundled dispatcher binaries (all 5 platforms) to support the new `on_error = "block_if_marker"` registry variant, now present in 5 `hooks-registry.toml` entries. Operators running the pre-rc.25 cached dispatcher binary against a post-rc.25 registry (or vice versa) would fail registry-load validation — this release re-syncs the shipped binary and registry atomically, as required whenever the registry schema grows a new `on_error` variant.

### Migration

No breaking changes. Existing fail-open hook plugins are unaffected by the INDETERMINATE marker gate (advisory-only). Operators should update via `/plugin update vsdd-factory@claude-mp` once the marketplace bump PR merges to pick up the wasmtime security fix.

## 1.0.0-rc.24 — wasmtime 46.0.2 + RUSTSEC clearances + POLICY 15 gate (2026-08-25)

Ships the E-21 wave-5 security and governance hardening set: the wasmtime sandbox runtime is bumped to 46.0.2 clearing five outstanding RUSTSEC advisories (including a `FilePerms` capability-bypass CVE), a new `cargo deny check advisories` CI gate closes the detection gap that let those advisories sit undetected across multiple release candidates, and the new POLICY 15 attestation-location gate (crate + required CI job) lands with an in-cycle false-FAIL fix. Also ships the `validate-cross-site-correspondence` cross-document consistency guard, the `validate-factory-path-staging` WASM artifact restore, the `failure_policy` dispatcher registry schema extension (ADR-039 Phase 1), and CI reliability hardening that restores full-workspace test execution in the release pipeline.

### Added

- **S-21.10 — `failure_policy` dispatcher registry schema extension** (BC-1.01.016 v1.3, E-21 W5, ADR-039 Phase 1): Dispatcher registry: new per-plugin `failure_policy` TOML field (`fail-closed` | `fail-open`); absent field defaults to `fail-open`; unknown values rejected at parse time (ADR-039 Decision 1+2 Phase 1).

- **POLICY 15 — attestation-location gate** (ADR-040 v1.18, PRs #777, #778): New `policy15-gate` crate implementing the ATTESTATION-LOCATION gate with four-outcome exit classification, validated via adversary spec-conformance review, 0-missed cargo-mutants, and execution-verified false-FAIL/false-negative controls (H-1 merge-commit, H-2 quotePath). Wired into CI as a dedicated unconditional required-check job (`policy-15-attestation-location`, fetch-depth:0, evaluated against the PR's head SHA and base ref) with attestation-gate-non-vacuity controls. Marking the job required in branch protection is a follow-up admin step.

- **S-21.07 — validate-cross-site-correspondence WASM hook plugin** (BC-5.39.010 v1.13, E-21 W4): New `validate-cross-site-correspondence` PostToolUse WASM hook plugin enforcing value-equivalence invariants across BC-INDEX.md, STORY-INDEX.md, and BC/story frontmatter after every Edit/Write on a governed file. Three mechanically-gateable classes ship: **Class A Arm1** fires on BC file writes and validates the BC-INDEX version-chain cite matches the BC frontmatter version (directional: primary-newer-than-index → advisory per PC2a; index-newer-than-primary → block per PC2b); **Class A Arm2** fires on story file writes and validates every BC version cite in the story body against the cited BC's current frontmatter; **Class B** fires on STORY-INDEX writes and enforces three-way input-hash equality per POLICY 18 (directional: B2==B3 AND B1≠B2 → advisory per PC13a; B2≠B3 → block per PC13b; half-present → advisory per PC13c); **Class E** fires on BC/story writes and enforces frontmatter version↔last_amended text-prefix parity and modified[] date monotonicity per POLICY 14 leg 4 / POLICY 17. New crate `crates/hook-plugins/validate-cross-site-correspondence`; 51-test bats acceptance suite (`plugins/vsdd-factory/tests/validate-cross-site-correspondence.bats`); `validate-cross-site-correspondence` entry added to `plugins/vsdd-factory/hooks-registry.toml`.

### Fixed

- **POLICY 15 — empty/unresolvable commit range false-FAIL** (ADR-040 v1.19 Ruling 9(f), PR #779): The attestation-location gate previously hard-blocked (exit 2, `EmptyOrUnreachable(EmptyRange)`) on every post-merge push to `develop` where the merge-base yields no commits, and on any unresolvable base ref — false-failing CI on push events with no PR diff to evaluate. Replaced with a new `GateOutcome::SkippedEmptyRange` inert-skip outcome (exit 0, `is_pass=true`); `UnreachableCause::EmptyRange` retired (both callsites migrated per TD-VSDD-060 sibling sweep). Guard ordering preserved — `StalePin` still beats `SkippedEmptyRange`.

- **S-21.07 — validate-cross-site-correspondence pass-7 adversarial fix burst** (BC-5.39.010 v1.13, E-21 W4): Closes adversarial findings from pass 7. **BC v1.12 (F-P6-001 Option 1 propagation; story-writer routing):** EC table realigned to v1.11 normative postconditions: EC-002 corrected (primary-newer-than-index direction → advisory + Continue per PC2a; prior text erroneously prescribed Block); EC-005 updated (block trigger is B2≠B3 per PC13b, not B2≠B1); EC-019 updated (Class A Arm1 primary-newer direction is advisory per PC2a, not a block violation); EC-034 added (PC13a advisory path: B2==B3 AND B1≠B2 → advisory + Continue); EC-035 added (PC2b block path: index-newer-than-primary → Block). VP table updated: "A Arm1 Stale-Index Block" renamed to "A Arm1 Index-Newer-than-Primary Block (PC2b)"; two new pending VP entries added. **BC v1.13 (ADR-038 §Decisions 1/4/5):** PC5/PC6 version-chain extraction algorithm replaced — rightmost-of-field[5] superseded by first-token-of-last-chain-entry with `fields[5..].join("|")` reassembly per ADR-038; rightmost-of-field[5] declared NON-CONFORMING. PC13c added (ADR-038 §Decision 3): half-present case — exactly one of {B2, B3} present and differing from B1 — dispositioned as advisory + Continue; live instances S-18.11/S-18.12. PC13 Phase 2 algorithm replaced (ADR-038 §Decision 5): reverse-field (rightmost-first) → BC-ID-anchored first-v-token; reverse-field declared NON-CONFORMING. EC-036 added. Corpus figures re-verified at 2026-08-06: 5-field 1943 / 39 six-field + 1 nine-field (n≥6 = 40) / total 1983. ADR-038 governs the algorithm changes (three amendments this burst: v1.1 histogram correction, v1.2 population predicate correction, v1.3 prescribed-text correction). ADR-039 governs the validator failure policy (per-plugin `failure_policy` field, fail-closed default for authorization-class validators, safe migration ordering). **F-S2107-P7-003 (CI corpus gate):** `cargo test` ran before `.factory/` was mounted in both `cargo-host` and `build-dispatcher` CI jobs, causing all 11 corpus tests to silently skip on every CI run since the plugin shipped; mount step moved to precede `cargo test`; `CI_REQUIRE_ARTIFACTS=1` added to both jobs converting absent-corpus from silent skip to hard failure. **F-S2107-P7-003b (VSDD_CORPUS_ROOT fail-open):** `VSDD_CORPUS_ROOT` fell back silently to parent-walk when set to an invalid path; previously untested `CI_REQUIRE_ARTIFACTS=1` panic branch now covered by `test_corpus_fail_hard_panics` and `test_corpus_invalid_corpus_root_override_panics_nonexistent`. **Production-file validator restoration:** `VSDD_SKIP_PRODUCTION_STATE_MD_TEST` flag removed; `validate_production_state_md_no_false_positive` and `validate_production_edp1_index_md_no_false_positive` now run on every PR run with coverage active.

- **S-21.07 — validate-cross-site-correspondence pass-6 adversarial fix burst** (BC-5.39.010 v1.12, E-21 W4): Closes adversarial findings from pass 6 implementing BC-5.39.010 v1.11 directional logic. **F-P4-003 (full-string equality — BLOCKER):** `RowMalformed` advisory message replaced with exact verbatim PC4a text per BC-5.39.010 v1.11: prefix changed from `[Class A Arm1] advisory:` to `[Class A Arm1]:`, "non-empty fields found" corrected to "fields found", all extra trailing explanation clauses removed. Pass-5 entry had added two MUST-verbatim subclauses but retained the wrong prefix and extra trailing text — the full-string equality gate requires exact verbatim conformance. **F-P6-001 / PC2 directional (HIGH):** `arm_a1` `Version` arm replaced with directional comparison per BC-5.39.010 v1.11 PC2: `bc_version > index_version` → PC2a advisory "primary newer than index; state-manager index update pending; Class A BLOCK suspended"; `index_version >= bc_version` OR parse failure → PC2b block "index is newer than primary. This is anomalous: the index cannot legitimately advance ahead of the BC it cites." Prior undirected block on any version mismatch was NON-CONFORMING under v1.11 POLICY 3 burst-ordering guarantee. **F-P6-001 / PC13 directional (HIGH):** `arm_b` `(Some(b2), Some(b3))` arm replaced with two-sub-case logic per BC-5.39.010 v1.11 PC13: `B2==B3` AND `B1!=B2` → PC13a advisory "STORY-INDEX sites agree with each other; story frontmatter differs; Class B BLOCK suspended"; `B2!=B3` → PC13b block "STORY-INDEX catalog and blockquote disagree — this is anomalous and has no burst-ordering explanation." Prior undirected block on any B1≠B2 was NON-CONFORMING under v1.11 PC13a carve-out. **F-P6-020 (volatile advisory prefix):** `run_arm_b1` volatile-input advisory prefix corrected to `validate-cross-site-correspondence [Class B] advisory:` (was missing entirely). **F-P6-013 (dead parameter):** `_is_strip: bool` parameter removed from `collect_block_scalar_body` — silently ignored since both clip and strip modes strip trailing blanks for field-value extraction; call site updated. **F-P6-019 normalization (seven sites, two sub-classes):** `extract_frontmatter_field` returns raw version strings (e.g., "v1.3") while structured extractors strip the leading 'v' prefix — comparison fails silently. Six live defects fixed: 019a (`arm_a1` false BLOCK on "v"-prefixed BC frontmatter versions), 019e (`arm_a2` false violation), 019f (`arm_e` false Class E1 block), 019-lib (`lib.rs` latent trap), 019b/019c closed by existing shadow absorber and new `_with_index_result` extension respectively. `strip_version_prefix` normalization accessor added at all call sites; F-P6-019-GUARD escape-scope guard added to prevent regression. Version cite sweep: `BC-5.39.010 v1.10` → `v1.12` in Cargo.toml description, arm_a1.rs RowMalformed BC trace, arm_a2.rs two BC trace comments, hooks-registry.toml §Gate Spec comment.

- **S-21.07 — validate-cross-site-correspondence pass-3/4/5 adversarial fix burst** (BC-5.39.010 v1.10, E-21 W4): Closes adversarial findings from passes 3, 4, and 5. **Pass-3 (F-S2107-P3-001 BLOCKER):** `extract_bc_index_version` rearchitected to three-state `BcIndexVersionState` enum (`RowAbsent` / `RowPresentNoVersion` / `Version(String)`) with escape-aware column-count-anchored split; prior two-state `Option<String>` conflated absent-row and no-version-cell, causing ≥1,712 false BLOCKs on 5-column BC-INDEX rows. **F-S2107-P3-002 (HIGH):** `arm_b::is_volatile_path` rewritten with 8-concrete-form three-branch implementation per ADR-037 §Decision 2 (pattern 1: `.factory/STATE.md` equality; patterns 2–5: `.factory/cycles/`-contains check filtered by `VOLATILE_PATTERNS_CYCLES_NAMED` 4-element named-filename array; patterns 6–8: path-equality for `ARCH-INDEX.md`, `BC-INDEX.md`, `STORY-INDEX.md`). ARCH-INDEX.md inclusion is normative conformance per ADR-037 §Decision 2, not a widening. **F-S2107-P3-004 (HIGH):** `arm_a2` BC-ID detection replaced with word-boundary check. **F-S2107-P3-009 (HIGH):** `dispatch::is_frontmatter_parity_target` epic branch path-component validation hardened. **F-S2107-P3-011 (MEDIUM):** `is_canonical_vp_filename` gains VP-INDEX.md early-return guard. **F-S2107-P3-015 (MEDIUM):** `arm_b::extract_story_id_from_table_row` migrated to `parse_story_id_len` canonical validator. Two-phase PC13 version-extraction replaces prior left-to-right optional-v scanner. **Pass-4 (F-S2107-P4-001):** `BcIndexVersionState` extended to four-state with `RowMalformed(usize)` advisory arm; `RowAbsent` now exclusively means no candidate line found; RowMalformed advisory message per postcondition 4a. **Pass-5 (F-S2107-P4-003/025 HIGH):** `RowMalformed` advisory message updated with MUST-verbatim postcondition 4a clauses ("Registration status cannot be determined from this line" + "Verify BC-INDEX body-table registration manually") — note: prefix and trailing-text full-string conformance required a further fix in pass-6 (F-P4-003 full-string equality gate). **F-S2107-P4-004 (HIGH):** `extract_frontmatter_field` block-scalar support (`|`, `|-`, `>`, `>-`); returning "|-" was NON-CONFORMING per PC36. **F-S2107-P4-005 (HIGH):** `extract_bc_index_version_state` rewritten to full-file scan; prior first-match-wins on malformed lines was NON-CONFORMING per v1.10 PC5. **F-S2107-P4-006 (HIGH):** `classify_provenance` deleted; arm_b.rs mismatch violation now enumerates all three provenance categories (STALE/FABRICATED/ALGORITHM-DIVERGENT) per postcondition 13/invariant 11 without asserting which applies. **F-S2107-P4-008 (sibling sweep):** arm_a2 stale-citation violation message POLICY 14 leg corrected (leg 3 → leg 5). **F-S2107-P4-015 (MEDIUM):** `CapabilityDenied` violation in arm_a2 now includes cited BC versions for actionable diagnostics. Stale `BC-5.39.010 v1.2/v1.8/v1.9` cites updated to v1.10 across Cargo.toml, lib.rs, arm_a1.rs.

- **S-21.09 — WASM artifact restore + registry parity gate** (BC-4.16.001, PR pending): Commits `validate-factory-path-staging.wasm` to `plugins/vsdd-factory/hook-plugins/` via `git add -f` (directory is gitignored; consistent with the `validate-heavy-op-delegation.wasm` precedent at `a85e6e05`). Adds T-012..T-049 plus two fixtures, four helpers, `detect_ungated_declarations`, and module-level `lex_norm` to `crates/factory-dispatcher/tests/bundle_orphan_check.rs`: asserts every WASM declared in either registry is also tracked in git (declared-set ⊆ tracked-set, the inverse direction of the existing T-009 standing gate); detects gate-3-escaped and gate-1-escaped declarations (UNGATED-DECLARATION); detects declarations resolving outside the worktree root (OUTSIDE-REPO-DECLARATION, T-047); adds git-fixture controls killing mutants M15 (git ls-files → fs scan), M18 (ls-tree HEAD → ls-files), and M16 (tracked/committed arg swap). `detect_ungated_declarations` predicate is total: every declaration either resolves correctly under `hook-plugins/`, fires `UNGATED-DECLARATION`, or fires `OUTSIDE-REPO-DECLARATION` — no silent drops. Pass-11: `detect_ungated_declarations` refactored to call `extract_hook_plugin_name` instead of the duplicate `is_hook_plugins` block (single-copy design); T-048 property test (18 candidates) kills M2/M4/M1+M4 composite and identifier swap; T-049 EC-005a control; `Registry::parse_str` validation and resolvers schema_version check added to `run_t012_gate` (HIGH-3); T-013/T-014 fixture assertions filter comment lines (HIGH-1); T-008 M22 NOT-orphan assertion added. Post-fix: declared count = tracked count = 36; `declared − tracked` = empty.

### Security

- **S-21.12 — dependency advisory clearances + `cargo deny` CI gate**: Five advisories cleared in-scope.
  - **wasmtime / wasmtime-wasi 44.0.x → 46.0.2** (RUSTSEC-2026-0188 / CVE-2026-58494; RUSTSEC-2026-0222): Bumps the wasmtime sandbox runtime and wasmtime-wasi adapter used by the hook plugin executor. RUSTSEC-2026-0188 / CVE-2026-58494 describes a `FilePerms` bypass allowing a WASM guest to read or overwrite host files via hard-link and rename operations that circumvented the capability check. RUSTSEC-2026-0222 describes a cross-engine type-index confusion that could cause the host to misinterpret WASM type indices when a module is shared across independently-created `Engine` instances. Both are fixed in wasmtime 46.0.2.
  - **crossbeam-epoch 0.9.18 → 0.9.20** (RUSTSEC-2026-0204): Clears an invalid pointer dereference in crossbeam-epoch's garbage-collection epoch mechanism, reachable under concurrent workloads. crossbeam-epoch is a transitive dependency of the dispatcher's async runtime.
  - **anyhow → 1.0.104** (RUSTSEC-2026-0190): Clears an unsoundness in `anyhow::Error::downcast_mut` that could produce a mutable reference to an incorrect concrete type under specific trait-object casting sequences. anyhow is used throughout the dispatcher crates for ergonomic error propagation.
  - **httpmock 0.7 → 0.8.3** (dev-dependency; RUSTSEC-2025-0052): httpmock 0.7 pulled in `async-std`, which is unmaintained and carries RUSTSEC-2025-0052. The 0.8.x line drops `async-std` entirely. This affects the test binary only; no production path is involved.
  - **SHA-pinned `cargo deny check advisories` CI job added**: A new CI job runs `cargo deny check advisories` on every PR to `develop` or `main`, gating merges when new advisories are detected against the workspace dependency tree. The deny configuration is SHA-pinned to prevent silent advisory-list drift. This closes the gap where the five advisories above were present in the tree for multiple release candidates without automated detection.

- **h2 0.4.14 → 0.4.19** (RUSTSEC-2026-0258): Clears an unbounded empty DATA frames DoS in the h2 HTTP/2 implementation, reachable via the sink crates' `reqwest`/`tonic` → `hyper` → `h2` chain in the production factory-dispatcher binary; a peer could queue empty DATA frames without limit, driving unbounded memory growth if streams were not actively drained. Fixed upstream in h2 0.4.16+; this bump was caught by the `cargo deny check advisories` CI gate added above, which failed clean against 0.4.14 in `release.yml`'s fresh dependency resolution. Lockfile-only change (`cargo update -p h2 --precise 0.4.19`); no Cargo.toml edit required.

### Operational

- **CI reliability**: eliminated an async e2e wall-clock flake (widened e2e deadlines; narrowed bats T-012 off `--all-targets`) and restored full-workspace test execution + corpus-artifact env parity in release.yml (#782, #783).

## 1.0.0-rc.23 — E-19 operator hardening — host ABI fixes + read_prefix FFI (2026-07-18)

Ships the complete E-19 post-rc.22 operator-hardening epic (9 stories, 55 story points, 3 waves): pr-manager stale-verdict pinning and merge-strategy enforcement, `verify-factory-lock` and `verify-state-timestamp-refresh` 256 KiB cap raise, `read_file NOT_FOUND(-5)` host ABI constant and `file_not_found` telemetry, registry and bundle hygiene, async plugin completion telemetry with `VSDD_SINK_FILE` release-mode opt-in, the new `host::read_prefix` bounded partial-read FFI entry point, Phase-B migration of `verify-factory-lock` to `read_prefix`, and four CRITICAL host ABI production-path gap fixes (D19–D22). All 5 platform dispatcher binaries (darwin-arm64, darwin-x86_64, linux-x86_64, linux-musl, windows-x86_64) are rebuilt from source this release, closing the rc.22-era stale-binary gap where linux/windows binaries pre-dated the S-19.06 hook-sdk changes.

### Added

- **S-19.01 — pr-manager stale-verdict detection + merge-strategy enforcement** (BC-5.42.001 v1.7, PR #613): `pr-manager-completion-guard.wasm` pins the covered SHA at READY time; `check-stale-verdict.sh` detects SHA drift between the READY verdict and merge. `enforce-merge-strategy.sh` is now the sole gateway for all `gh pr merge` invocations, enforcing the squash/merge/rebase strategy declared by the PR author and blocking strategy-smuggling. Includes a darwin-leg `bats-darwin-leg-macos` CI job running under `/bin/bash 3.2`. Introduces BC-5.42.001 and the no-new-.sh standing policy for future work.

- **S-19.03 — read_file NOT_FOUND(-5) host ABI constant + file_not_found telemetry** (BC-2.07.001 v1.5, BC-2.02.011 v1.7, PR #611): Adds `codes::NOT_FOUND = -5` to the hook-sdk `pub mod codes` module and the `HostError::NotFound` named variant, giving plugin authors a stable constant for missing-file detection without numeric literals. Adds the `internal.file_not_found` internal telemetry event emitted on NOT_FOUND paths. `warn-pending-wave-gate` now handles absent `wave-state.yaml` gracefully (returns `Continue`) with zero `capability_denied` events.

- **S-19.05 — async plugin completion telemetry + VSDD_SINK_FILE release-mode opt-in** (BC-3.08.001 v1.23, PR #640): Async plugins now emit `plugin.completed` (9 mandatory fields) and `plugin.abandoned` (7 mandatory fields, emitted when the drain window expires) events. `VSDD_SINK_FILE` is honored in both debug and release builds — operators can capture all observable plugin events as JSONL by setting `VSDD_SINK_FILE=/tmp/disp-events.jsonl` before launching Claude (SEC-003 path constraint: no traversal sequences or null bytes).

- **S-19.06 — host::read_prefix bounded partial read — additive FFI entry point** (BC-1.17.001 v1.6, VP-101, PR #657): New `host::read_prefix(path, max_bytes, timeout_ms)` host function in the hook-sdk. Implements head-c semantics: reads at most `max_bytes` bytes from the file start; `max_bytes = 0` returns an empty payload without opening the file. Architecturally guaranteed to never return `OUTPUT_TOO_LARGE (-3)`. Capability schema added to the `[hooks.capabilities.read_prefix]` preamble block in `hooks-registry.toml`.

- **S-19.07 — verify-factory-lock Phase-B migration to read_prefix** (BC-4.13.001 v1.17, VP-095 v1.5, PR #670): Migrates `verify-factory-lock` from `host::read_file` (subject to `OUTPUT_TOO_LARGE` at >256 KiB) to `host::read_prefix` with a ~35 KB real-shape frontmatter fixture. Removes the `STATE_MD_MAX_BYTES` constant from the plugin. The `verify-factory-lock-bash` registry entry is also migrated to `capabilities.read_prefix`.

### Fixed

- **S-19.02 — verify-factory-lock OUTPUT_TOO_LARGE fix — 64 KiB cap raised to 256 KiB** (BC-4.13.001 v1.15 Phase-A, PR #610): `STATE_MD_MAX_BYTES` raised from 65536 to 262144. Adds `extract_frontmatter(bytes: &[u8]) -> &[u8]` to `crates/factory-lock-parse`; `verify-factory-lock` now passes only the frontmatter slice to lock parsing. Soft-warning `state_md_approaching_cap` emitted when byte count is in (200000, 262144]; reads succeed at 262144-exact; fail at 262145+. Closes the rc.22 FINDING-1 smoke-test blocker.

- **S-19.04 — registry/bundle hygiene — tool-filter regex anchoring + orphan WASM exclusion** (VP-099, PR #639): All tool-filter regex entries in `hooks-registry.toml` are fully anchored (`^...$`); unanchored entries that previously matched unintended tool names (e.g., `Edit` matching `MultiEdit`) are corrected. Three orphan WASM build artifacts excluded from the release bundle. `hello-hook.wasm` BUILD/COPY steps removed from `release.yml`. Adds `crates/factory-dispatcher/tests/bundle_orphan_check.rs` Rust workspace test and the `cargo test stage_release_bundle` validation path to the release gate.

- **S-19.08 — verify-state-timestamp-refresh 256 KiB cap raise + extract_frontmatter wiring** (BC-5.40.001 v1.3, PR #646): Same `STATE_MD_MAX_BYTES = 262144` fix applied to `verify-state-timestamp-refresh` (sibling plugin carrying the identical defect class as S-19.02). Wires `extract_frontmatter` from `crates/factory-lock-parse` for byte-exact boundary semantics; reuses the VP-096 proptest. Zero `output_too_large` events on 70 KiB reads.

- **S-19.09 — host ABI production-path gap fixes D19–D22** (BC-1.17.001 v1.7, BC-3.08.001 v1.24, PR #659): Four CRITICAL host ABI fixes: **D19** — `read_prefix` registered in `setup_host_on_store_data` (the production dispatch path, `Linker<StoreData>`); S-19.06 had registered it only in `setup_linker` (test path), making every operator-visible `read_prefix` call non-functional. **D20** — `timeout_ms` doc comments corrected from "epoch interruption" (aspirational) to "structurally unenforced" in `read_file.rs` and `read_prefix.rs`. **D21** — all bare event-type string literals (`"internal.file_not_found"`, `"plugin.abandoned"`, `"plugin.completed"`, `"plugin.timeout"`) in production code replaced with named constants from `internal_log.rs`. **D22** — `emit_plugin_completed_async` now emits `fields["timestamp"]` (was absent; BC-3.08.001 §Postconditions Event 6 Mandatory-fields now satisfied).

### Operational

- **All 5 platform dispatcher binaries rebuilt from source** (darwin-arm64, darwin-x86_64, linux-x86_64, linux-musl, windows-x86_64): The rc.22 release cycle left linux-x86_64, linux-musl, and windows-x86_64 binaries at the rc.22 bundle commit; they did not yet include the S-19.06 hook-sdk changes. The `release.yml` `build-binaries` job rebuilds all 5 platform binaries from source at tag-push, automatically resolving this gap. Operators on `/plugin update vsdd-factory@claude-mp` receive the complete E-19 implementation across all platforms.
- **POLICY 20 bundle hygiene verified:** 34 WASMs on disk = 33 hooks-registry entries + 1 resolvers-registry entry; 0 orphans; 0 dev samples. Registry count unchanged from rc.22.
- **New BC promoted to active:** BC-5.42.001 (pr-manager stale-verdict enforcement), BC-1.17.001 v1.6 (read_prefix FFI contract), BC-5.40.001 v1.3 (STATE.md freshness guard 256 KiB).
- **VSDD_SINK_FILE** is now a supported release-mode diagnostic tool for capturing plugin events as JSONL (see S-19.05 and `CLAUDE.md` §VSDD_SINK_FILE diagnostic capture).
- No breaking changes to hook API, registry schema, or skill invocation signatures.

### Deferred

- **E-20 candidates from E-19 scope** (D-846, D-847 follow-ups): timeout_ms epoch-interrupt structural enforcement (architect follow-up story required — ADR-025 §Decisions 17–18 amendment); hook-sdk timeout_ms wrapper doc alignment (blocked on architect decision); POLICY 21 `.sh` migration (D-846 — `plugins/vsdd-factory/bin/check-stale-verdict.sh` and `enforce-merge-strategy.sh` pending architect migration ADR). Full E-20 roster in `.factory/cycles/v1.0-brownfield-backfill/e-19-sw-post-epic-report.md` Section 5.
- **SEC-003 CWE-833 re-open gate**: currently LOW severity because `timeout_ms` is structurally unenforced; risk profile changes when epoch-interrupt implementation lands (see architect follow-up from D19).
- **release-config.yaml vestigial reference** (FINDING-A from smoke test): `global_version_sources` cites nonexistent root `.claude-plugin/marketplace.json` (removed at rc.7; benign — `release.yml` reads version from the git tag). E-20 devops cleanup.
- **Dispatcher version telemetry** (FINDING-B from smoke test): binary emits `"dispatcher_version": "0.0.1"` in internal logs rather than the actual release semver. No functional impact. E-20 polish.

## 1.0.0-rc.22 — context durability epic + release hardening (2026-07-02)

Ships the complete E-18 context-durability epic (18 stories + 2 prereqs): full PreCompact/PostCompact dispatcher routing, the precompact-flush native WASM plugin, postcompact-reanchor advisory hook, wave-handoff/rehydrate-wave/wave-reset skills, validate-wave-handoff-completeness and validate-heavy-op-delegation WASM gates, pure-parse invariant consistency gate, F2 process-gap lesson gates, check-state-health autocompact verification, sprint-state per-story producer migration, portability-lint guard extension with bash-portability.md, and release-pipeline hardening (orphan WASM removal + registry-vs-staged assertion). Operators on `/plugin update vsdd-factory@claude-mp` receive the complete context-durability hook chain and a cleaner 33-plugin registry bundle.

### Added

- **S-18.00 — Dispatcher PreCompact/PostCompact routing + check-harness-version.sh** (ADR-026, ADR-028, ADR-029, PR #191): Dispatcher routes `PreCompact` and `PostCompact` harness events through the hook chain for the first time. `check-harness-version.sh` validates harness ABI at startup.

- **S-18.01 — wave-handoff skill** (BC-6.23.001, PR #193): `/vsdd-factory:wave-handoff` skill produces HANDOFF.md + wave-state.yaml atomically at wave boundaries, capturing the canonical pre-compact state snapshot for downstream rehydration.

- **S-18.02 — validate-wave-handoff-completeness WASM PostToolUse gate** (BC-4.12.001, PR #195): WASM plugin enforcing structural completeness of HANDOFF.md before any PostToolUse write is allowed to proceed past a wave boundary.

- **S-18.03 — rehydrate-wave skill + wave-reset SKILL.md** (BC-6.24.001, PR #270): `/vsdd-factory:rehydrate-wave` reconstructs wave context from the git-tracked HANDOFF.md/wave-state.yaml pair after a context compaction or session clear. `/vsdd-factory:wave-reset` resets wave state without full rehydration.

- **S-18.04a — precompact-flush native WASM plugin core** (ADR-028, PR #249): `precompact-flush.wasm` — a native (non-exec) WASM plugin that fires on `PreCompact` events and persists the current wave-boundary state to the `factory-artifacts` branch before context is lost.

- **S-18.04b — exec-free PreCompact exemption + dispatcher git_context payload injection** (ADR-029, PR #262, #264): Dispatcher injects `git_context` payload (branch, HEAD SHA, worktree path) into every hook invocation. `PreCompact` exemption allows exec-free WASM plugins to bypass the dispatch guard that otherwise requires an active factory session.

- **S-18.05 — postcompact-reanchor advisory hook** (BC-7.07.002, PR #271): `PostCompact` hook emits a `[PostCompact Re-anchor]` block to stdout so the LLM session can re-ground itself after automatic context compaction.

- **S-18.06 — validate-heavy-op-delegation WASM gate** (BC-4.15.001, ADR-026 §D12, PR #284): WASM PreToolUse gate that blocks heavy operations (multi-file writes, large Agent dispatches) from running directly in a context-durability-sensitive wave without first producing a HANDOFF.md.

- **S-18.07 — E-18 terminology disambiguation docs** (ADR-026 §D7, PR #301): Clarifies the three independent compaction mechanisms (`/compact-state` skill, `PreCompact` hook event, `PostCompact` hook event) and their non-overlapping triggers in CLAUDE.md and ADR-026.

- **S-18.08 — pure-parse invariant consistency gate** (BC-4.14.001/BC-4.15.001, ADR-026 §D14, PR #303): WASM gate that enforces the pure-parse invariant — spec parsers may not perform side effects — as a structural consistency check across the dispatcher hook chain.

- **S-18.09 — F2 process-gap lesson gate checks** (ADR-026, D-576, PR #307): Terminal wave-8 gate: checks that the F2 adversary cycle's process-gap lessons (L-EDP1-NNN) are codified before allowing the wave-8 terminal commit to proceed.

- **S-18.10 — check-state-health CLAUDE_AUTOCOMPACT_PCT_OVERRIDE verification** (BC-6.25.001 v1.1, ADR-026 §F-11, PR #315): `/vsdd-factory:check-state-health` now verifies `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE` is set, within the canonical ceiling, and numeric; emits ADVISORYs rather than hard blocks (non-breaking for operators without the env var set).

- **S-18.11 — sprint-state per-story producer migration + def-b ordering** (BC-5.41.004, PR #340): Migrates sprint-state YAML emission from the wave-level producer to per-story producers; enforces def-b (dependency-before) ordering in the emitted YAML. Adds 14 new bats acceptance tests.

- **S-18.12 — portability-lint guard extension + bash-portability.md** (E-18 wave-9, PR #384, #385): Extends the portability-lint WASM guard with array detection (`typeset -A`), while/until wrapper detection, and pip-dotted-version detection. Adds `plugins/vsdd-factory/docs/bash-portability.md` as the canonical portability reference for bash hook authors.

- **wave-scheduling skill** (`/vsdd-factory:wave-scheduling`, delivered with S-18.11): New skill for scheduling and ordering story waves with def-b dependency enforcement.

### Fixed

- **S-18.13 — wave-handoff Write-tool gate-trigger fix** (PR #196, closes F-S1802-02): PostToolUse gate now correctly fires on HANDOFF.md writes. Previously the gate only triggered on Edit tool calls, missing Write-tool productions entirely.

- **S-18.14 — dispatcher resolver WASM path resolution + cross-platform CI hardening** (PR #201): Fixes resolver WASM path resolution when the dispatcher is invoked from a non-root cwd; adds cross-platform CI matrix hardening for darwin-x64 and linux-musl.

- **write_file.rs: resolve relative paths under cwd** (PR #198, facade parity with read_file + invoke.rs): `write_file` now resolves relative paths against the session cwd, matching the behavior of `read_file` and `invoke.rs`. Eliminates silent writes to the wrong directory when cwd ≠ repo root.

- **compute-input-hash: awk exit-condition bug + repo-root-relative path resolution** (PR #189): Fixes awk early-exit that caused drift detection to report false-positive hash mismatches on files with no trailing newline; adds repo-root-relative path resolution.

- **develop CI restoration** (PR #200, closes #197): Restores develop CI after the `validate-state-structure` validator emitted a false-positive block on `pure-parse` schema; fixes the flaky `F-P3-008` timing lower bound.

- **S-18.12 detector-parity gaps** (PR #385, adversary MAJOR-1 + MINOR-2/4/5): Post-merge adversary review of the portability-lint guard found comment-stripping parity gaps in the `has_arrays` trigger loop (MAJOR-1) and three broadening mismatches (typeset -A, while/until, pip-dotted). Fixed in-cycle before rc.22.

- **F-P3-008 timing flake** (PR #431): Replaces the wall-clock lower bound in the F-P3-008 timeout test with an InternalLog JSONL event check, eliminating CI flakiness on slow runners.

### Operational

- **11 orphan underscore-named stub WASMs removed from the bundle** (PR #431): `block_ai_attribution.wasm`, `capture_commit_activity.wasm`, `capture_pr_activity.wasm`, `lint_registry_async_invariant.wasm`, `validate_burst_log.wasm`, `validate_closes_completeness.wasm`, `validate_dispatch_advance.wasm`, `validate_index_cite_refresh.wasm`, `validate_policies_schema.wasm`, `validate_state_structure.wasm`, `validate_trajectory_tail_cell_completeness.wasm`. These were lib-target build artifacts (underscore naming, 75-byte stubs) that were never invoked by the dispatcher — the registry references their hyphen-named equivalents. Operators get a cleaner hook-plugins directory with no dead weight.
- **Registry-vs-staged WASM assertion in release pipeline** (PR #431, #438): Both the `build` job (asserts artifact/ dir before upload) and `commit-binaries` job (asserts hook-plugins/ dir before bundle commit) now verify every registry-declared WASM is present. Parse guard: registry parse yielding < 30 entries aborts as a regex failure. Prevents silent partial-bundle publishes.
- **Hook plugin registry count: 33** (hooks-registry.toml). Down from 44 after stub removal; the 11 removed entries were never dispatched.
- **Dependabot #202** (PR merged to develop before this release): bumps vite and @vitejs/plugin-react in `plugins/vsdd-factory/skills/visual-companion` to current safe versions.
- No breaking changes to hook API, registry schema, or skill invocation signatures.

### Deferred

- **D-749 merge-race process-gap story** (pr-manager HEAD-pinning): The pr-manager race where a story's HEAD SHA changes between adversary review and merge is tracked as D-749; deferred to next wave.
- **vsdd-context-resolvers naming duality** (architect adjudication pending): The resolver binary uses hyphen-naming (`vsdd-context-resolvers.wasm`) while all other hook plugins use hyphen names from the registry; the duality between the resolver-registry and hooks-registry is tracked for adjudication.
- **release.yml ADVISORY-1/2 guard floor tuning**: The registry-count guard floor (≥ 30) is conservative; a tighter floor tied to the exact declared count is deferred to post-rc.22 once the count stabilizes.
- **Dependabot #192** (dompurify 3.4.6 → 3.4.11 in visual-companion): Deferred; no runtime security impact on the plugin's CLI surface.

## 1.0.0-rc.21 — factory-lock subsystem + dispatcher hardening + STATE.md freshness guard (2026-06-12)

Ships the complete S-17 factory-lock wave (S-17.01 through S-17.04), three dispatcher/adversary/pr-manager hardening fixes, and the write-time STATE.md freshness guard. This release brings the lock-heartbeat mechanism, the `/factory-lock` and `/factory-unlock` skills, the `verify-factory-lock` and `verify-state-timestamp-refresh` WASM guards, and the new `factory-lock-parse` crate onto the operator-level cache — the first production-grade implementation of coordinated factory-state locking.

### Added

- **S-17.01 — factory_lock STATE.md schema + state-burst fetch-then-CAS push** (BC-5.40.001 PC1, PR #181, Closes #170): Adds `factory_lock:` block to STATE.md frontmatter schema (owner, acquired_at, lock_id, heartbeat_at, reason). State-burst uses fetch-then-compare-and-swap push to prevent blind overwrites under concurrent factory sessions.

- **S-17.02 — verify-factory-lock WASM PreToolUse guard** (BC-4.13.001, PR #182, part of #170): New WASM hook plugin (`verify-factory-lock`) that fires PreToolUse on Agent/Edit/Write tools; blocks state mutations when STATE.md is locked by a different session. Ships new `factory-lock-parse` crate.

- **S-17.03 — /factory-lock + /factory-unlock skills + health lock status** (BC-6.23.001, PR #183, Closes #170): Adds the `/vsdd-factory:factory-lock` and `/vsdd-factory:factory-unlock` skills plus lock-status reporting in the `/vsdd-factory:factory-health` skill. Operators can now acquire, release, and inspect factory-state locks from the Claude Code UI.

- **S-17.04 — verify-state-timestamp-refresh WASM guard (mid-burst heartbeat renewal wiring)** (BC-5.40.001 PC4, PR #184): New `verify-state-timestamp-refresh` WASM PostToolUse guard that enforces timestamp and lock-heartbeat advance on every factory state write. Ensures STATE.md `last_updated` and `heartbeat_at` fields advance monotonically; blocks stale writes that would regress the timestamp.

### Fixed

- **Dispatcher: recursive .factory/.factory/logs/ shadow + fail-loud plugin-root** (PR #179, Closes #130): Resolves the recursive log-directory shadow where `.factory/logs/` was nested inside itself under certain cwd configurations. Dispatcher now fails loudly on bad plugin-root rather than silently emitting to a shadow path. Adds guard shadow-exception for the legitimate factory-artifacts worktree layout.

- **Adversary: worktree-identity engine fix — eliminate phantom findings** (PR #180, Closes #169, #176): Fixes the adversarial-review agent emitting findings scoped to the wrong worktree (phantom findings in the engine root when the adversary was dispatched from a story worktree). Worktree-identity resolution is now correct; fresh-context adversary reviews no longer bleed across worktree boundaries.

- **PR-manager: verify remote branch deletion after merge** (PR #178, Closes #128): The pr-manager now verifies the remote feature branch is actually deleted after a squash-merge rather than assuming the `--delete-branch` flag succeeded. Prevents stale remote branches from accumulating silently.

### Operational

- rc.21 delivers the complete factory-lock subsystem to the operator-level cache. After the marketplace PR merges, operators on `/plugin update vsdd-factory@claude-mp` will receive the new `verify-factory-lock` WASM guard (PreToolUse), the `verify-state-timestamp-refresh` WASM guard (PostToolUse), the `factory-lock-parse` crate, and the `/factory-lock` + `/factory-unlock` skills.
- Hook plugin count advances: `verify-factory-lock` + `verify-state-timestamp-refresh` join the registry (operator count was 53 as of rc.20; rc.21 brings it to 55).
- Pre-existing advisory RUSTSEC-2026-0149 (wasmtime-wasi) is tracked in STATE.md and is not a release blocker; no operator action required.

### Deferred

- 6 open Dependabot PRs (#157 openssl, #156/#152/#125/#3/#2 npm visual-companion) remain deferred to a future maintenance sweep.

## 1.0.0-rc.20 — trajectory-tail completeness hook + MCP fleet-sweep (2026-06-01)

Ships the S-15.17 validate-trajectory-tail-cell-completeness WASM hook to the operator-level cache, completes the S-15.03 PRIORITY-A lint-hook automation wave, hardens a flaky resolver-timeout test, and sweeps MCP tool-guidance across the agent fleet (research-agent now biases toward Perplexity deep-research).

### Added

- **validate-trajectory-tail-cell-completeness WASM hook** (BC-5.39.009 v1.9, ADR-023, hooks-registry priority 158): enforces STATE.md trajectory-tail cell completeness. New dispatcher hook plugin — operator plugin count now 53 (was 52). (S-15.17, PR #164)

### Fixed

- **F-P3-008 resolver-timeout flake** (TD #67): de-flaked the resolver-timeout wall-clock assertion in the integration suite, stabilizing CI on ubuntu runners. (PR #165)
- **MCP tool-guidance fleet-sweep**: fleet-wide MCP tool-guidance corrections; research-agent now biases toward Perplexity deep-research and enforces MCP usage. (PR #163)

### Operational

- This release brings the S-15.17 hook and the MCP fleet-sweep agent guidance onto the operator-level marketplace cache. Develop-branch-only changes prior to this release did not affect the cached plugin; after rc.20 ships and the marketplace PR merges, operators on `/plugin update vsdd-factory@claude-mp` pick up the new hook + guidance.

### Deferred

- 2 S-15.17 code-reviewer suggestion-level findings (S-15.17-CR-001/002) remain ACCEPTED-DEFERRED (advisory-arm only, no production reachability).
- 6 open Dependabot PRs (#157 openssl, #156/#152/#125/#3/#2 npm visual-companion) deferred to a future maintenance sweep — NOT in this release.

## 1.0.0-rc.19 — S-15.03 PRIORITY-A complete — 7 new WASM validation hooks + dispatcher hardening (2026-05-27)

**rc.19 ships the complete S-15.03 PRIORITY-A automation wave** — the
mechanical-gate infrastructure that closes the META-LEVEL-24 false-green
class identified during the engine-discipline F5 cycle. Eleven stories
landed across three milestones (M1: foundation; M2: core validation hooks;
M3: extended validation + cross-document agreement) bringing 7 new WASM
hook plugins into the registry plus dispatcher stderr surfacing, test
infrastructure de-flake, security migration, and CI hardening.

This is the largest rc since rc.15 by both PR count (18) and feature
surface (7 net-new WASM plugins, 21 total native-WASM plugins now in the
registry). It represents the post-asymptotic-acceptance ship: both
E-10 (brownfield cycle adversarial review) and F5 (engine-discipline
cycle adversarial review) reached their respective asymptotic floors and
the human authorized cutting the release.

### Added

- **7 new WASM validation hook plugins** (S-15.03 PRIORITY-A M1+M2+M3
  waves; priorities 152-157 in the PostToolUse Edit|Write tier):

  - `validate-index-cite-refresh` (PR #145, S-15.07, priority 151):
    enforces D-429(b) cross-cell version cite refresh when any of the
    4 canonical indexes (BC/VP/STORY/ARCH) bumps. Catches stale version
    citations in STATE.md and sibling docs.

  - `validate-burst-log` (PR #146, S-15.11, priority 152): enforces
    burst-log h2 entry structural completeness (8 D-444(c) blocks per
    entry); detects missing Dim-2/5/6/7 attestation blocks before
    commit.

  - `validate-state-structure` Phase 1+2 (PRs #147+#154, S-15.09+S-15.10,
    priority 153): enforces STATE.md structural invariants — tally sync,
    monotonic D-NNN, row coalescence, Phase Progress completeness, and
    D-434(e) frontmatter parity.

  - `validate-dispatch-advance` (PR #148, S-15.14, priority 154):
    enforces TD-VSDD-097-EXT — all 5 BC-5.39.006 PCs simultaneously
    present in STATE.md `current_step:` field at dispatch-side advance.

  - `validate-closes-completeness` Phase 1+2 (PRs #155+#159, S-15.12+
    S-15.13, priority 156, new crate): enforces D-411(c) Closes-block
    completeness; Phase 1 trigger-file Closes-set must agree with
    pass-N Part A finding set; Phase 2 extends cross-document agreement
    to burst-log + decision-log + lessons.md + STATE.md secondary
    citation sites via ADR-022 pointer-file mechanism
    (`.factory/current-adversary-pass.txt`).

  - `validate-policies-schema` (PR #158, S-15.15, priority 157, new
    crate): enforces policies.yaml schema integrity per F-PASS14-004
    closure — frontmatter required (`document_type`, `version`,
    `last_amended`), bare integer policy IDs, required per-policy
    fields, and ADR-021 Option (b) cargo-audit cache reader. Closes
    F-PASS14-004/006 + D-472 POLICY 13+16.

- **`validate-stable-anchors`** (PreToolUse, priority 155): companion
  hook ensuring anchor references in cross-document citations remain
  stable across edits.

- **`dim2-gates` bash template library** (PR #144, S-15.08): 11
  canonical bash scripts at `plugins/vsdd-factory/hooks/dim2-gates/`
  for fix-burst Dim-2 attestation — directly addresses D-449(a)
  literal-shell-execution-evidence requirement (META-LEVEL-24 closure).
  Each script follows `--help` + `set -euo pipefail` + structured
  PASS/FAIL stdout pattern. 12 bats integration tests + 25 fixtures
  exercise the library against representative artifacts.

- **`lessons.md` size gate** (PR #153, S-15.16-Part-B): extends
  `validate-state-size` hook to enforce D-442(e) lessons.md budget
  (≤3500 soft / ≤4000 hard). Closes the WASM-fuel-exhaustion class
  where oversized lessons.md silently failed PostToolUse validators.

- **`wait_for_log_event` test helper** (PR #143, S-15.05): async
  helper in `full_stack_plugin_invocation.rs` that observes
  dispatcher internal-log events with 100ms polling instead of
  wall-clock timing. Used to rewrite TC-4/5/7/9 from
  timing-assertion to event-observation form. Resolves the recurring
  TC-9 ubuntu flake (F-P3-008 root cause; architect Strategy B).

### Changed

- **Dispatcher stderr now surfaces `blocking_plugins` + `block_reason`**
  (PR #138, TD #71): blocking dispatches now include the plugin name(s)
  that fired and the block message inline in the dispatcher's stderr
  summary line. Previously operators had to grep the daily internal-log
  jsonl by trace UUID to discover *why* a dispatch was blocked; the
  reason is now visible at the failure point. Reduces diagnostic
  time-to-root-cause from minutes to seconds. 5 bats integration tests
  cover the surfacing path. CLAUDE.md "Factory Hook Diagnostics" §Step 2
  updated to reflect the change.

- **Workspace serde migration: serde_yaml 0.9.34 → serde_norway 0.9**
  (PR #139, TD #72): 13 files migrated. Initially recommended target
  was serde_yml, but cargo-audit during security review caught
  RUSTSEC-2025-0068 (serde_yml unsoundness) + RUSTSEC-2025-0067 (libyml
  UB) — pivoted in-scope to serde_norway 0.9. 2 Critical security
  findings resolved in 1 fix cycle. The pivot established TD #74
  shift-left codification pattern (`docs/dispatch-package-authoring.md`).

- **CI cache hardening** (PR #140, TD #70): `Swatinem/rust-cache` SHA-
  pinned at 3 sites (ci.yml cargo-host + ci.yml cross-compile matrix
  + release.yml cross-compile) at commit `c19371144df3bb44fab255c43d04cbc2ab54d1c4`.
  `cache-on-failure=true` added for resilience. Note: floating `@v2` was
  already present; TD #70 hardened the pin, did not introduce caching.
  10/10 green on 7 runners.

- **trace_id field-name canonicalization** (PR #142, S-15.04): bats
  internal-log assertions tightened from `'"(dispatcher_)?trace_id":"'`
  to `'"trace_id":"'` with negative `dispatcher_trace_id` assertion
  enforcing BC-3.08.001 v1.7 Invariant 5 zero-occurrence contract.
  Plugin stdin envelope `dispatcher_trace_id` field stays (intentionally
  distinct). Closes TD #66.

- **All 7 new WASM hooks fail-open on `host::read_file` errors with
  524 KiB cap** (PR #160, F-PASS15 sibling sweep): full TD-VSDD-060
  sibling-site sweep raised `host::read_file` byte cap from 65536 to
  524288 across 7 hook plugin crates (`validate-index-cite-refresh`,
  `validate-burst-log`, `lint-registry-async-invariant`,
  `session-start-telemetry`, `update-wave-state-on-merge`,
  `validate-artifact-path`, `validate-per-story-adversary-convergence`,
  `warn-pending-wave-gate`). Each crate now declares a named
  `MAX_BYTES = 524_288` constant with doc-comment; the 2 crates with
  material behavioral impact (`validate-burst-log`,
  `validate-index-cite-refresh`) carry compile-time
  `const _: () = assert!(MAX_BYTES >= 524_288)` regression assertions.
  Previously the 65536 cap silently rendered validators inert against
  production STATE.md (95 KiB) — D-429(b) cross-cell version sweep
  was functionally dead before this fix. Closes F-PASS15-001 (HIGH),
  F-PASS15-002 (HIGH), F-PASS15-004 (MEDIUM).

### Operational

- **S-15.03 PRIORITY-A complete**: all 11 stories shipped across M1
  (foundation: S-15.06 + S-15.08 + S-15.16-Part-A), M2 (core validation:
  S-15.07 + S-15.11 + S-15.09 + S-15.14), and M3 (extended validation:
  S-15.16-Part-B + S-15.10 + S-15.12 + S-15.15 + S-15.13). 40 story
  points across M3 alone. E-10 (brownfield adversarial cycle) resumption
  was gated on this work landing — NOW UNBLOCKED.

- **E-10 pass-15 + fix-burst PR #160 shipped**: first cycle-level
  adversarial review since the S-15.03 wave. Verdict
  MEDIUM-HIGH 8 findings (0C+2H+4M+2L); trajectory holds at 8 from
  pass-14. Character SHIFTED from governance-process META-class to
  implementation-correctness — the automation wave WORKED. Fix-burst
  closed the 2 HIGHs + 1 MEDIUM via the 65536→524288 sweep. Remaining
  5 findings ACCEPTED-AT-ASYMPTOTIC-FLOOR per D-471 model.

- **F5 pass-75 fix-burst + META-LEVEL-30 asymptotic-acceptance**: first
  F5 cycle review since the 14-day pause at pass-74. Verdict HIGH 11
  findings at META-LEVEL-30 CANDIDATE-CONFIRMED via 3 routes
  (closure-burst literal-shell command-vs-interpretation path mismatch;
  codified-without-runtime-gate degraded over pause; paused-cycle-INDEX
  staleness). 6 mechanical findings closed in-burst (POL-14 leg-5
  propagation, banner literal count, lessons.md corrigendum, F5
  INDEX.md paused-frozen frontmatter, story status auto-promotion).
  4 structural findings accepted at META-30 asymptotic floor per
  D-386 Option C extension. HIGH-002 (codified-without-runtime-gate)
  anchored to new forward-story S-15.17 per Canonical Principle Rule 3
  concrete future dependency.

- **TD #66/67/70/71/72/74 all closed in this release window**: six
  technical debt items resolved across PRs #138-143 (TD #66 trace_id
  + TD #67 de-flake + TD #70 cache + TD #71 stderr + TD #72 serde
  migration + TD #74 cargo-audit codification).

- **dim2-gates source instantiation** (PR #137 + D-454(c) preview):
  `plugins/vsdd-factory/hooks/dim2-gates/` registered in artifact-path
  registry and instantiated with template scaffolding before the full
  library landed in S-15.08. Tier-B priority-1 closure for
  ADV-EDP1-P74-HIGH-002.

- **POLICY 14 5-leg quintuple parity gate**: extended at D-490 from
  3-leg (BC frontmatter + body Changelog + modified[] array) to 5-leg
  (adds frontmatter `last_amended:` text-prefix + upstream-index body-
  table cells). Extended again at D-494 with `verification_step 7`
  literal-shell 4-index self-application gate. Production-validated
  through ~46 BC-006-parity-sweep conversions during M3 BC cascade.

- **Cache implications**: operators upgrading from rc.18 will see 7 new
  WASM plugins in their dispatcher cache: `validate-index-cite-refresh`,
  `validate-burst-log`, `validate-state-structure`,
  `validate-dispatch-advance`, `validate-closes-completeness`,
  `validate-policies-schema`, `validate-stable-anchors`. All register
  with `on_error = "continue"` (soft-launch mode); crash/timeout
  produces warn-level log line, not block. Dispatcher binary changed
  schema-compatibly via TD #71 (stderr summary additions); no manual
  cache fix required.

### Migration

No breaking changes for end-users. Hook authors writing new plugins
should adopt the `MAX_BYTES = 524_288` convention for `host::read_file`
calls — the 65536 default is now considered an anti-pattern (silent
fail-open on production-sized files). See PR #160 for canonical examples.

### Deferred

- **F-PASS15-003** (3 hooks hardcode `.factory/cycles/v1.0-brownfield-backfill/`)
  remains open at asymptotic floor; cycle-resolver future work has no
  story yet. Will surface on cycle rotation.

- **F-PASS15-005** (validate-closes-completeness Phase 2 missing
  INDEX.md as secondary citation site) accepted at floor.

- **F-PASS15-006** (all 7 new hooks use `on_error=continue`) intentional
  soft-launch; promote individual hooks to `block` after production
  stabilization period.

- **F-PASS15-007** (CI WASM plugin count assertion stale: `>=16` but
  21 plugins exist) low-priority cosmetic.

- **F-PASS15-008** (`find_part_a_start` off-by-one in pos tracking)
  guarded by `.min(text.len())`; no current bug.

- **S-15.17** (validate-trajectory-tail-cell-completeness, F5 pass-75
  HIGH-002 anchor) authored as draft; awaits PO authorship of
  BC-5.39.009 then per-story TDD delivery.

- **TD-VSDD-101** (CI env-var paper-fix for production STATE.md read
  in bats test) closed in S-15.15 ADR-021 implementation path.

## 1.0.0-rc.18 — research-agent MCP fix + S-12 convergence context migration (rc.17 retry) (2026-05-13)

**rc.18 is a retry of rc.17.** Tag `v1.0.0-rc.17` was pushed on
2026-05-13 but its `Pre-release Validation` job failed on a single
sanity test in `resolver-host-abi-context-injection.bats` — the test
capped HOST_ABI.md's `## Context Injection Contract` section at 400
lines, but the section had legitimately settled at ~475 lines once
the S-12.x ABI documentation shipped. No GitHub Release was created
for rc.17; the source content (the research-agent MCP fix and the
S-12.x resolver work) is on main but operators never received the
binaries or marketplace bump.

rc.18 carries the same content as rc.17 plus a one-line fix to the
sanity test ceiling (400 → 600) so the release pipeline can complete.
A latent CI gap was also noted: `ci.yml`'s `validate` job only runs
three bats files, while `release.yml`'s `Pre-release Validation` runs
the full `run-all.sh`. The S-12.06 sanity test belonged to the latter
only, so develop CI was green through merge. A separate follow-up
should mirror `run-all.sh` into `ci.yml` to close the gap.

The substantive headline content (carried over from rc.17):

- **research-agent MCP fix** — Perplexity tool names in the agent's
  allowlist now match what `@perplexity-ai/mcp-server` publishes;
  Tavily tools are now allowed for cross-validation.
- **S-12.x convergence context migration** — closes F-P2-001
  (critical-path terminus). The `validate-per-story-adversary-convergence`
  hook now consumes wave context through `WaveContextResolver`
  instead of a side-channel fallback. New `vsdd-context-resolvers`
  crate ships as a first-class WASM plugin; `ContextResolver` trait,
  `ResolverRegistry`, and hook-sdk resolver-authoring macros enter
  the public SDK surface.

### Fixed

- **`plugins/vsdd-factory/tests/resolver-host-abi-context-injection.bats`**
  (PR #134): `sanity: Context Injection Contract section is between
  100 and 400 lines` raised to 100–600. The 400-line ceiling was an
  early S-12.06 estimate, not a spec invariant; the section is
  currently 475 lines (legitimate ABI documentation including
  resolver lifecycle, host fns, registry schema, error envelope,
  and worked examples). Caught by `release.yml`'s `Pre-release
  Validation` during the rc.17 cut.

- **`plugins/vsdd-factory/agents/research-agent.md`** (PR #127,
  closes #126; carried over from rc.17): four Perplexity MCP tools
  now resolve. The allowlist previously named
  `mcp__perplexity__deep_research` (and three similarly-stale
  names) but `@perplexity-ai/mcp-server` publishes
  `mcp__perplexity__perplexity_research`. Every research-agent
  dispatch silently lost Perplexity access; Context7 tools were
  unaffected because their names already matched. The fix updates
  the allowlist, the Perplexity section body docs, the new Tavily
  section, the Research Methods table, and the Tool Access list,
  with an inline note on the `perplexity_` prefix to prevent the
  stale-name pattern from creeping back in.

### Added

- **Tavily MCP allowlist** (PR #127; carried over from rc.17): five
  Tavily tools (`tavily_search`, `tavily_research`, `tavily_extract`,
  `tavily_crawl`, `tavily_map`) are now permitted for the
  research-agent, enabling cross-validation against a second
  research backend when both `@perplexity-ai/mcp-server` and the
  Tavily MCP server are configured in the operator's `.mcp.json`.

- **`crates/vsdd-context-resolvers`** (PR #122, S-12.07; carried
  over from rc.17): new workspace crate housing
  `WaveContextResolver`, the first concrete resolver implementation.
  Provides wave-context extraction (active wave, story list, schema
  validation) used by the convergence hook to ground its decisions.

- **`hook-sdk` resolver-authoring extensions** (PR #119, S-12.05;
  carried over from rc.17): new types and macros in the public SDK
  surface so hook authors can declare resolver dependencies, type
  their needs, and receive schema-validated context payloads. Backs
  the `needs_context` pattern adopted by
  `validate-per-story-adversary-convergence`.

- **`ContextResolver` trait + `ResolverRegistry`** (PR #120,
  S-12.03; carried over from rc.17): trait abstraction and registry
  that lets the dispatcher load resolver WASM plugins, route context
  requests by name, and return strongly-typed payloads to consuming
  hooks.

- **WASM resolver loading + lifecycle + error isolation** (PR #121,
  S-12.04; carried over from rc.17): dispatcher infrastructure to
  load resolver plugins, manage their lifecycle (init, invoke,
  teardown), and isolate errors so a bad resolver cannot poison the
  dispatcher process. Includes determinism property tests and
  load-time error tests.

### Changed

- **`validate-per-story-adversary-convergence` hook** (PR #123,
  S-12.08; carried over from rc.17): now consumes wave context
  through the resolver pipeline instead of the legacy side-channel
  fallback. The hook's `needs_context` declaration wires it to
  `WaveContextResolver` at registration time; convergence decisions
  now use the schema-validated wave payload directly. The old
  fallback path is removed — there is one path now, not two.

- **`hooks-registry.toml`** (carried over from rc.17): registers the
  new `vsdd-context-resolvers.wasm` plugin and updates the
  convergence hook entry to declare its resolver dependency.

### Operational

- **rc.17 left as a dead tag**: `v1.0.0-rc.17` exists in the repo's
  tag list but has no GitHub Release attached (the `release` job in
  the workflow was skipped after `validate` failed). The tag's
  commit on main (`193bf9b5`) carries the source content rc.17 was
  trying to ship; rc.18 is the actual ship. Operators on
  `claude-mp` resolve to rc.18 directly — there is no rc.17 to skip
  over because no marketplace bump PR was opened.

- **F-P2-001 (critical-path terminus) closed**: with rc.18, the
  S-12.x resolver epic is functionally complete. Operators no
  longer need to worry about the legacy fallback masking missing
  wave context — the convergence hook fails closed if its resolver
  cannot return a valid payload.

- **Cache implications**: operators upgrading from rc.16 will see
  one new plugin in their cache (`vsdd-context-resolvers.wasm`) and
  an updated `validate-per-story-adversary-convergence.wasm`. The
  dispatcher binary is unchanged in schema; no manual cache fix is
  required (unlike rc.14).

- **CI gap identified, not fixed in this rc**: `ci.yml`'s `validate`
  job runs only `hooks.bats`, `block-helper.bats`, and `bin.bats`,
  while `release.yml`'s `Pre-release Validation` runs the full
  `run-all.sh`. Tests added under `run-all.sh` only (like the S-12.06
  sanity test) are not exercised on develop PRs. A follow-up should
  mirror `run-all.sh` into `ci.yml` so this class of failure is
  caught before tag time.

### Migration

No breaking changes for end-users. Hook authors writing new plugins
can now opt into resolver-based context via the `needs_context`
declaration; existing plugins continue to work without modification.

### Deferred

- Additional resolver implementations beyond `WaveContextResolver`
  (cycle, sprint, project context) remain on the S-12.x backlog and
  will ship in subsequent rcs as they pass per-story delivery.

- Mirror `bats run-all.sh` into `ci.yml`'s `validate` job (or split
  it into a dedicated `run-all-bats` job) so develop CI catches
  full-suite failures before release time. Tracked as a follow-up.

## 1.0.0-rc.17 — research-agent MCP fix + S-12 convergence context migration (2026-05-12)

The headline of rc.17 is a small but high-impact bug fix to the
`research-agent` agent definition: the Perplexity MCP tool names in
its allowlist did not match the names that `@perplexity-ai/mcp-server`
actually publishes, so every research dispatch silently lost Perplexity
access and fell back to `WebSearch`/`WebFetch` without surfacing an
error. The four real tool names (`perplexity_ask`, `perplexity_search`,
`perplexity_reason`, `perplexity_research`) are now wired correctly,
and the five Tavily MCP tools (`tavily_search`, `tavily_research`,
`tavily_extract`, `tavily_crawl`, `tavily_map`) join the allowlist so
research-agent can use Tavily as a cross-validation source.

Riding alongside the agent fix is the S-12.x **convergence context
migration**, which closes F-P2-001 — the critical-path terminus of
the resolver work. The convergence hook
(`validate-per-story-adversary-convergence`) now consumes its wave
context through the new `WaveContextResolver` rather than a fragile
side-channel fallback, completing the contract-resolver architecture
introduced over S-12.03 → S-12.08. The new `vsdd-context-resolvers`
crate ships as a first-class WASM plugin, the `ContextResolver` trait
and `ResolverRegistry` are now part of the public hook-sdk surface,
WASM resolver loading/lifecycle/error-isolation is in place, and the
hook-sdk macro extensions for authoring resolvers ship. The old
fallback path is removed; all convergence decisions now flow through
the resolver pipeline.

### Fixed

- **`plugins/vsdd-factory/agents/research-agent.md`** (PR #127,
  closes #126): four Perplexity MCP tools now resolve. The allowlist
  previously named `mcp__perplexity__deep_research` (and three
  similarly-stale names) but `@perplexity-ai/mcp-server` publishes
  `mcp__perplexity__perplexity_research`. Every research-agent
  dispatch silently lost Perplexity access; Context7 tools were
  unaffected because their names already matched. The fix updates
  the allowlist, the Perplexity section body docs, the new Tavily
  section, the Research Methods table, and the Tool Access list,
  with an inline note on the `perplexity_` prefix to prevent the
  stale-name pattern from creeping back in.

### Added

- **Tavily MCP allowlist** (PR #127): five Tavily tools
  (`tavily_search`, `tavily_research`, `tavily_extract`,
  `tavily_crawl`, `tavily_map`) are now permitted for the
  research-agent, enabling cross-validation against a second
  research backend when both `@perplexity-ai/mcp-server` and the
  Tavily MCP server are configured in the operator's `.mcp.json`.

- **`crates/vsdd-context-resolvers`** (PR #122, S-12.07): new
  workspace crate housing `WaveContextResolver`, the first concrete
  resolver implementation. Provides wave-context extraction
  (active wave, story list, schema validation) used by the
  convergence hook to ground its decisions.

- **`hook-sdk` resolver-authoring extensions** (PR #119, S-12.05):
  new types and macros in the public SDK surface so hook authors can
  declare resolver dependencies, type their needs, and receive
  schema-validated context payloads. Backs the `needs_context`
  pattern adopted by `validate-per-story-adversary-convergence`.

- **`ContextResolver` trait + `ResolverRegistry`** (PR #120,
  S-12.03): trait abstraction and registry that lets the dispatcher
  load resolver WASM plugins, route context requests by name, and
  return strongly-typed payloads to consuming hooks.

- **WASM resolver loading + lifecycle + error isolation** (PR #121,
  S-12.04): dispatcher infrastructure to load resolver plugins,
  manage their lifecycle (init, invoke, teardown), and isolate
  errors so a bad resolver cannot poison the dispatcher process.
  Includes determinism property tests and load-time error tests.

### Changed

- **`validate-per-story-adversary-convergence` hook** (PR #123,
  S-12.08): now consumes wave context through the resolver pipeline
  instead of the legacy side-channel fallback. The hook's
  `needs_context` declaration wires it to `WaveContextResolver` at
  registration time; convergence decisions now use the
  schema-validated wave payload directly. The old fallback path is
  removed — there is one path now, not two.

- **`hooks-registry.toml`**: registers the new
  `vsdd-context-resolvers.wasm` plugin and updates the convergence
  hook entry to declare its resolver dependency.

### Operational

- **F-P2-001 (critical-path terminus) closed**: with rc.17, the S-12.x
  resolver epic is functionally complete. Operators no longer need to
  worry about the legacy fallback masking missing wave context — the
  convergence hook fails closed if its resolver cannot return a valid
  payload.

- **Cache implications**: operators upgrading from rc.16 will see one
  new plugin in their cache (`vsdd-context-resolvers.wasm`) and an
  updated `validate-per-story-adversary-convergence.wasm`. The
  dispatcher binary is unchanged in schema; no manual cache fix is
  required (unlike rc.14).

- **No procedural changes** to RELEASING.md — rc.17 is cut by
  following rc.16's procedure verbatim. This is the second release
  governed entirely by RELEASING.md and the release-branch guardrail.

### Migration

No breaking changes for end-users. Hook authors writing new plugins
can now opt into resolver-based context via the `needs_context`
declaration; existing plugins continue to work without modification.

### Deferred

- Additional resolver implementations beyond `WaveContextResolver`
  (cycle, sprint, project context) remain on the S-12.x backlog and
  will ship in subsequent rcs as they pass per-story delivery.

## 1.0.0-rc.16 — RELEASING.md procedural infrastructure + TD #69 guardrail (2026-05-10)

The headline of rc.16 is procedural infrastructure: it ships
`RELEASING.md` (the canonical release procedure that will govern every
future rc.X release), a CI guardrail enforcing the
`release/v1.0.0-rc.X → main` branch convention, and a release-skill
update that defers to RELEASING.md (and prompts the human to create
one when missing). This release is the first to be cut by following
RELEASING.md step-by-step — dogfooding the very procedure it ships.

Combined with rc.15's source-sync fix, rc.16 closes both halves of
the "rc.14 marketplace E-REG-001" root cause: rc.15 fixed the content
drift, rc.16 fixes the procedural drift that allowed the content
drift to accumulate.

### Added

- **`RELEASING.md`** (project root, ~250 lines) — single source of
  truth for the release procedure. Captures the two-branch model
  rationale, mandatory invariants table (8 invariants with violation
  consequences), step-by-step procedure (Step 0 preflight → Step 9
  operator install verification), recovery procedures for 5 known
  failure modes, what-gets-automated breakdown, and explicit rules
  for AI agents executing a release.

- **`.github/workflows/release-branch-guardrail.yml`** — PR-trigger
  GitHub Actions workflow that fails any PR with a `release/*` head
  branch targeting anything other than `main`. Runs on every PR
  event; skips when head doesn't match `release/*`. Failure
  annotation includes a one-line `gh pr edit` fix command and links
  to RELEASING.md.

- **`CLAUDE.md`** (project root) — minimal project conventions
  auto-loaded by Claude Code at session start. References
  RELEASING.md, the branching model, commit-message rules, testing
  commands, and the self-referential nature of this repo (engine
  and product are the same).

### Changed

- **`plugins/vsdd-factory/skills/release/SKILL.md`** — the release
  skill now reads `RELEASING.md` first when present and uses it as
  authoritative. If `RELEASING.md` is missing, the skill prompts the
  human to either (1) create one before cutting the release
  (recommended), or (2) proceed with the generic config-driven flow.
  Generic fallback preserved for projects that explicitly opt in.

### Fixed

- **`release-branch-guardrail.yml` YAML parse error** (PR #117): the
  initial workflow used bash backslash line continuations inside
  YAML's literal block scalar, which broke YAML parsing. Every push
  to develop produced a misleading "failed" run with empty job list.
  Fixed by collapsing the multi-line bash echo to one line.

- **`release-branch-guardrail.yml` shell-injection hardening**
  (PR #117): Semgrep flagged direct `${{ github.head_ref }}` /
  `${{ github.base_ref }}` interpolations inside the `run:` block as
  a run-shell-injection risk. Channeled through `env:` per the
  standard GitHub Actions security pattern; no behavior change.

### Operational

- This release is the **live exercise** of the new RELEASING.md
  procedure. The release branch (`release/v1.0.0-rc.16`) was created
  from develop, the PR targets `main` (enforced by the guardrail
  itself, which fires on this very PR), and the merge will use
  `--merge` (not `--squash`) to preserve develop's commits as
  ancestors of main.

- Future release branches must follow RELEASING.md or fail the
  guardrail's check. Operators no longer need to remember the
  convention from session memory.

### Deferred

- TD #66 — `trace_id` / `dispatcher_trace_id` field-name canonicalization.
- TD #67 — 4 timing-flaky e2e tests in `full_stack_plugin_invocation.rs`.
- TD #68 — sync-develop binary auto-resolve (already fixed but watch
  for any edge cases on the next release where develop has merged
  new WASM source files since rc.16).

## 1.0.0-rc.15 — Marketplace source-sync fix (main backfill) (2026-05-10)

The headline of rc.15 is operational: it restores the rc.X release-branch
pattern (release/v1.0.0-rc.X → main) that was the canonical sync mechanism
through rc.11 but was inadvertently abandoned starting rc.13 (PR #111
targeted develop instead of main). The drift accumulated for 3 release
cycles produced a catastrophic mismatch in rc.14: the marketplace tarball
shipped with `hooks-registry.toml schema_version = 1` against a dispatcher
binary built from develop expecting `schema_version = 2`. Every operator
who ran `/plugin update vsdd-factory@claude-mp` to rc.14 hit `E-REG-001`
fail-closed on every tool call until manually patching the cache.

rc.15 ships:

- **main backfilled with 92 files of develop source** (3954 insertions /
  880 deletions) — every fix from rc.12, rc.13, rc.14 that was in develop
  but never reached main, and therefore never reached the marketplace
  tarball. Notably:
  - `hooks-registry.toml` `schema_version = 2` (S-15.01 T-3a)
  - `scripts/generate-registry-from-hooks-json.sh` schema-bump (PR #112)
  - 3 bats test files with portable path resolution (PR #112)
  - `ci.yml` develop trigger (PR #112)
  - `release.yml` TD #68 binary auto-resolve (PR #114)
  - `regression-v1.0.bats` trace_id field tolerance (PR #113)
  - All S-15.01 plugin async semantics work (rc.13)
  - F5 convergence work (rc.13)
  - HOST_ABI documentation expansion (rc.13)
  - 33 new integration tests (rc.13)
  - Crashed/timed-out gate fail-closed (PR #110)

- **Identical dispatcher binaries to rc.14** — the binary build was
  correct in rc.14; only the source-file portion of the tarball was
  stale. rc.15 rebuilds them anyway to ensure all platforms are
  reproducible at the rc.15 tag.

### Operational

- rc.14 stays as a tagged release but is functionally **broken on the
  marketplace** (E-REG-001 on every tool call). Operators should update
  to rc.15.
- The rc.X release-branch convention is restored: future releases must
  target `main`, not `develop`. Documented in this entry as the canonical
  pattern.
- The release branch was merged with `--merge` (not `--squash`) to
  preserve develop's commits as ancestors of main, which means future
  rc.X releases see `develop ⊇ main` correctly (TD #68 auto-resolve in
  sync-develop becomes a no-op when nothing has diverged).

### Deferred

- TD #69 — guardrail enforcing release branches target main (lint hook
  or branch-protection rule). Defer to S-15.02.
- TD #66 — `trace_id` / `dispatcher_trace_id` field-name canonicalization.
- TD #67 — 4 timing-flaky e2e tests in `full_stack_plugin_invocation.rs`.

## 1.0.0-rc.14 — Release CI unblocker (supersedes rc.11–rc.13) (2026-05-09)

The headline of rc.14 is operational: it is the first release tag in 5 days
to flow cleanly through `validate` → `build-binaries` → `commit-binaries` →
`release` → `bump-marketplace`. Every tag from `v1.0.0-rc.11` (2026-05-05)
through `v1.0.0-rc.13` (2026-05-09) was tagged but never reached the
`drbothen/claude-mp` marketplace because four bats suites were silently
broken in `release.yml`'s `validate` job — and `ci.yml` only ran on `main`,
so the regression was never caught at PR time.

rc.14 carries **all the rc.12 and rc.13 deliverables forward** plus the
release-CI fixes. Operators on `/plugin update vsdd-factory@claude-mp`
will jump from rc.10 directly to rc.14.

### Fixed

- **Release CI validate-job unblock** (PR #112):
  - `scripts/generate-registry-from-hooks-json.sh` now emits
    `schema_version = 2` (matches `REGISTRY_SCHEMA_VERSION` bump landed by
    T-3a / S-15.01 on 2026-05-07). Pre-fix the dispatcher rejected
    generator output with `E-REG-001 SchemaVersion { got: 1, expected: 2 }`.
  - `tests/resolver-host-abi-context-injection.bats` and
    `tests/per-story-adversary-workflow.bats` no longer hardcode developer-
    machine worktree paths (`/Users/jmagady/.../worktrees/S-12.06` etc).
    They now resolve `REPO_ROOT` portably from `BATS_TEST_DIRNAME`.
  - `tests/perf-baseline.bats` now correctly canonicalizes the
    `git --git-common-dir` output (which is relative to the `-C` target,
    not CWD) by `cd`'ing into `BATS_TEST_DIRNAME` first and `pwd`'ing.

- **`ci.yml` now triggers on `develop`** (PR #112): adds `develop` to push
  and pull_request branch lists. Closes the gap that let release-CI-blocking
  bugs land for 4 days without surfacing on PR.

### Carried forward from rc.12 (Slice 3 canonical reason codes)

- Tier A canonical block-message migrations (slice 3): wave-gate-prerequisite,
  pr-merge-prerequisites, factory-path-root, factory-branch-guard,
  destructive-command-guard, verify-git-push, validate-finding-format,
  validate-vp-consistency, validate-subsystem-names, validate-bc-title,
  validate-story-bc-sync. All use canonical Why/Fix/Code block-message
  pattern with HOST_ABI Tier A reason codes.
- `tests/release-ci-bats.bats` aligned to Slice 3 canonical reason codes.
- `lib/block.sh` source robustness for hook-side helpers.

### Carried forward from rc.13 (Plugin async semantics)

- **Plugin async semantics** (S-15.01, ADR-019): registry-layer partition
  splits matched-event hooks into `sync_group` / `async_group` based on each
  hook's `async = true|false` declaration in `hooks-registry.toml`. Async
  hooks fire-and-forget via `executor.rs::spawn_async_plugin`; the dispatcher
  returns to Claude Code after sync-group completion plus an
  `ASYNC_DRAIN_WINDOW_MS` (DI-019, 100 ms) drain window. Pre-fix, `async: true`
  in `hooks.json` silenced every plugin's exit code for a given event.
- **10 telemetry hooks marked `async = true`**: `session-start-telemetry`,
  `session-end-telemetry`, `capture-commit-activity`, `capture-pr-activity`,
  `tool-failure-hooks`, `convergence-tracker`, `track-agent-start`,
  `track-agent-stop`, `session-learning`, `update-wave-state-on-merge`.
- **HOST_ABI documentation expanded** (+1,549 words / +331 lines): five new
  sections covering Async Hook Semantics, Registry `async` field schema,
  plugin-author async guidance, async failure modes, and the
  `dispatcher.registry_invalid` wire-format split.
- **Type-safe `registry-invalid` emitters** (B-3): two dedicated functions
  replace the prior single emitter. E-REG-002 (intra-entry) and E-REG-003
  (inter-entry) distinction is now enforced by the type system.
- **Crashed/timed-out sync gate hooks fail-closed** (PR #110, ADR-019
  Decision 2): `executor.rs::plugin_fail_closed()` returns true when
  `result.kind ∈ {Crashed, Timeout}` AND `on_error == OnError::Block`.
  Pre-fix these failed open silently.
- **Hook silent-bypass on absolute paths** (PR #108):
  `validate-stable-anchors::is_spec_target` and
  `validate-artifact-path::matches_canonical` + `hook_logic` now accept both
  relative and absolute path forms.
- **VP-070 Kani harness assumption hardened** to remain sound against
  absolute-path matching introduced by `validate-artifact-path`.

### Refactored

- `cargo fmt --all` sweep across 11 files (PR #112). Pre-existing fmt drift
  caught by ci.yml's new develop coverage. No behavioral changes — only
  whitespace/wrapping. Restores PR-time fmt enforcement.

### Operational notes

- `v1.0.0-rc.11`, `v1.0.0-rc.12`, `v1.0.0-rc.13` remain as git tags but
  never reached `drbothen/claude-mp`. They are functionally superseded by
  rc.14.
- Filed TD #66: `regression-v1.0` test 7 expects field `dispatcher_trace_id`
  but S-15.01 emits `trace_id`. CI skips this test (preflight artifacts
  missing in `validate` job which doesn't `cargo build` before bats), so
  it doesn't gate release. Defer to S-15.02.

### F5 Cycle Convergence (carried from rc.13)

Cycle `v1.0-feature-plugin-async-semantics-pass-1` reached ADR-013
convergence at pass-57 (3 consecutive NITPICK_ONLY: 55, 56, 57).
- 40 adversary passes (P18–P57); 49 fix-bursts
- 19 L-P28-001 META-self-application failure instances codified
- 14+ lessons codified (L-P18-001 through L-P28-001)

### Deferred

- S-15.02: dispatcher cold-start optimization (P2)
- S-15.03: mechanical hook enforcement WASM hooks (P2)
- TD #66: trace_id / dispatcher_trace_id field-name canonicalization

## 1.0.0-rc.13 — Plugin async semantics (S-15.01) + absolute-path bypass fix + F5 convergence (2026-05-09)

The headline delivery for rc.13 is the S-15.01 registry-layer async partition —
the fix that makes `on_error = "block"` validators actually block.
Telemetry hooks are classified `async = true` so they never gate the user.
Two silent-bypass bugs in absolute-path matching are closed. The F5
adversarial cycle converged at pass-57 (ADR-013 protocol satisfied).

### Added

- **Plugin async semantics** (S-15.01, ADR-019): the registry-layer partition
  splits matched-event hooks into `sync_group` / `async_group` based on each
  hook's `async = true|false` declaration in `hooks-registry.toml`. Async hooks
  fire-and-forget via `executor.rs::spawn_async_plugin`; the dispatcher returns
  to Claude Code after the sync group completes plus an `ASYNC_DRAIN_WINDOW_MS`
  (DI-019, 100 ms) drain window. Prior to this change, `async: true` in
  `hooks.json` silenced every plugin's exit code for a given event, causing all
  55 block decisions recorded in the 2026-05-07 prism audit to be discarded
  silently.

- **10 telemetry hooks marked `async = true`** (`feat/async-flag-telemetry-hooks`,
  commit `31c30a75`): `session-start-telemetry`, `session-end-telemetry`,
  `capture-commit-activity`, `capture-pr-activity`, `tool-failure-hooks`,
  `convergence-tracker`, `track-agent-start`, `track-agent-stop`,
  `session-learning`, `update-wave-state-on-merge`. These hooks emit
  telemetry / audit / tracking output and no longer block Claude Code's tool
  flow.

- **HOST_ABI documentation expanded** (`crates/hook-sdk/HOST_ABI.md`, +1,549
  words / +331 lines, commit `3e034a37` on `docs/host-abi-async-semantics`):
  five new sections covering Async Hook Semantics, Registry `async` field
  schema, plugin-author async guidance, async failure modes, and the
  `dispatcher.registry_invalid` wire-format split (B-3).

- **Type-safe `registry-invalid` emitters** (B-3, PR #109): two dedicated
  functions replace the prior single emitter. `emit_registry_invalid_e_reg002`
  takes 3 params (context, plugin name, violation) for intra-entry violations
  (E-REG-002); `emit_registry_invalid_e_reg003` takes 5 params (context, plugin
  name, violation, offending event, `offending_tool: Option<&str>`) for
  inter-entry violations (E-REG-003). Wire format is unchanged; the distinction
  is now enforced by the type system rather than a documentation-only invariant.

### Fixed

- **Hook silent-bypass on absolute paths** (PR #108): `validate-stable-anchors::is_spec_target`
  and `validate-artifact-path::matches_canonical` + `hook_logic` now accept
  both relative (`.factory/...`) and absolute (`/path/to/.factory/...`) forms
  via leading-slash discipline. Pre-fix, both hooks returned `Continue` for
  Claude Code's absolute-path envelopes — a silent bypass. Verified by 10/10
  integration tests in `tests/absolute_path_hook_engagement.rs`
  (`validation/absolute-path-hook-engagement`, commit `264fa2ee`).

- **VP-070 Kani harness assumption hardened** (commit `cc5a016b`): tightened
  `kani::assume(!path.starts_with(".factory/") && !path.contains("/.factory/"))`
  to remain sound against the absolute-path matching introduced by the
  `validate-artifact-path` fix above.

### Refactored

- `emit_dispatcher_registry_invalid` split into two type-safe variants
  `emit_registry_invalid_e_reg002` / `emit_registry_invalid_e_reg003` (B-3,
  PR #109). The documentation-only invariant requiring `(None, None)` for
  E-REG-002 callers is now enforced by the type system.

### F5 Cycle Convergence

Cycle `v1.0-feature-plugin-async-semantics-pass-1` reached ADR-013 convergence
criteria at pass-57 (3 consecutive NITPICK_ONLY passes: 55, 56, 57).

- 40 adversary passes (P18–P57); 49 fix-bursts
- 19 L-P28-001 META-self-application failure instances codified
- 14+ lessons codified (L-P18-001 through L-P28-001)
- Spec corpus repairs: 1,000+ edits across BC-INDEX, VP-INDEX, STORY-INDEX,
  ARCH-INDEX, BC bodies, and lessons.md
- Final index versions: BC-INDEX v1.63, VP-INDEX v1.40, STORY-INDEX v2.64,
  ARCH-INDEX v1.44

### Deferred

- S-15.02: dispatcher cold-start optimization (P2)
- S-15.03: mechanical hook enforcement — `validate-symbol-cite`,
  `validate-index-cite-refresh`, `validate-lesson-retroactive-sweep` WASM
  hooks; structurally-convergent path codified by L-P28-001 series (P2)

## 1.0.0-rc.11 — Single-commit burst protocol + TD-020 sweep + retag-round-2 (2026-05-04)

Three improvements bundled (rc.11 required two retag rounds to ship; root
causes documented as TD-VSDD-054 + TD-VSDD-055 in the engine register).

### Single-commit burst protocol (TD-VSDD-053)

Resolves external **TD-VSDD-044**: the self-referential factory-artifacts
HEAD cite in STATE.md / SESSION-HANDOFF.md "current state" sections that
caused 6× recurrence loops in real-world dogfood (5+ force-pushes per
session). Retires the two-commit burst protocol; replaces with a
single-commit protocol.

The loop existed because STATE.md sits ON the factory-artifacts branch,
so committing STATE.md changed HEAD, instantly staling any HEAD-SHA cite
inside the same content. The two-commit workaround (Stage 1 placeholder
→ commit → Stage 2 backfill SHA → commit) had to update the SHA in 8
specific cite locations in lockstep; missing any one created a
"fix-the-fix" loop. The structural fix removes the cite altogether:
`git -C .factory log -1` returns the current HEAD; STATE.md no longer
claims it. Historical SHA references in changelog rows, decisions log,
and cycle manifests remain valid (immutable past burst SHAs).

#### Changed

- **`templates/verify-sha-currency.sh`** — removed factory-arts cite
  extraction (~80 LOC). Preserved: develop SHA cite check (cross-branch,
  no loop), `MULTI_COMMIT_CHAIN_NOT_ALLOWED` regression guard,
  wave-state.yaml ↔ STATE.md cross-record check, tense-flip detection.
- **`agents/state-manager.md`** — protocol references updated; current
  factory-artifacts HEAD is `git -C .factory log -1`, not a string in
  any artifact (operational guidance in agent doc, not in STATE.md prose).
- **`skills/state-burst/SKILL.md`** — full rewrite to single-commit;
  Stage 1/2 sections removed; `15fa97e6` placeholder pattern removed;
  commit message must NOT contain `backfill`.
- **`templates/state-manager-checklist-template.md`** — full rewrite to
  single-commit; two acceptable `remediation_sha:` patterns documented.

#### Preserved (not touched)

- `validate-input-hash.sh` — artifact-level drift detection
- `validate-state-pin-freshness.sh` — version-pin freshness
- Historical SHA references throughout (changelog/decisions-log/cycle-
  manifest/TL;DR History/BC/story/spec IDs)
- `verify-sha-currency.sh` wave-state ↔ STATE cross-record check

### Retag-round-2 fixes (TD-VSDD-054 + TD-VSDD-055)

The first rc.11 release attempt failed at pre-release validation — TD-020
sweep had un-skipped 4 bats suites with "passes locally" claims that hit
two CI-environment regressions:

- **TD-VSDD-054** (PR #85+#86): `generate-registry.bats` invoked the
  generator script which read `git show 7b4b774^:plugins/vsdd-factory/hooks/hooks.json`.
  CI uses shallow clone (default `actions/checkout` fetch-depth: 1) so
  the historical SHA wasn't available. Fixed by vendoring the historical
  hooks.json as `scripts/legacy/hooks-json-pre-templating.json` (94 lines,
  exact byte-for-byte copy) and updating the generator to read the
  static file. No more git-history dependency.
- **TD-VSDD-055** (PR #87): `state-health.bats` setup ran `git commit`
  which exits status 128 in CI because user.email/name aren't globally
  configured on GitHub-hosted runners. Fixed by adding three
  `git config --local` calls in the test's setup() (user.email,
  user.name, commit.gpgsign false). Local config doesn't pollute
  operator's global config.

### TD-020 sweep — bats SKIP_SUITES cleanup

Resolved the four bats suites permanently excluded from the
release-validation gate via `SKIP_SUITES` in
`plugins/vsdd-factory/tests/run-all.sh`. Each suite either now passes
when un-skipped or had its broken assertions removed.

#### Tests removed (with rationale)

- **`codify-lessons.bats` — BC-5.36.007** ("all three agents updated
  in the delivery branch"): asserted that the
  `.worktrees/codify-lessons` delivery worktree contained diff-vs-main
  changes to 3 agents. The worktree no longer exists post-merge, so
  the assertion was structurally impossible. The same contract
  (agents updated) is still validated by BC-5.36.001–006 which inspect
  the merged files.
- **`novelty-assessment.bats` — "validates adversarial-delta-review
  files"**: asserted that the hook would block
  `.factory/phase-f5-adversarial/adversarial-delta-review.md`. The
  current case-statement matcher in
  `validate-novelty-assessment.sh` only covers
  `.factory/specs/adversarial-*review*.md`. Per TD-020 (no new
  functionality to make tests pass), the test asserted behavior the
  hook does not implement.
- **`novelty-assessment.bats` — "validates story adversarial review
  files"**: asserted that `.factory/stories/adversarial-reviews/`
  paths are validated. That layout is not used anywhere in the
  current plugin (zero grep hits). Per-story adversarial review is
  not part of the current artifact tree.
- **`novelty-assessment.bats` — "valid delta review passes"**: passed
  only because the case-statement matcher does not cover
  `.factory/phase-f5-adversarial/` and so falls through to exit 0.
  With the matching negative-path test removed, keeping this
  happy-path test gave a misleading signal.

#### Suites un-skipped (no test changes needed)

- **`generate-registry.bats`** — all 6 invariant tests now pass; the
  generator stabilized after the TD-016 glob refactor that originally
  surfaced the breakage.

## 1.0.0-rc.10 — Skill-body version-ref cleanup (2026-05-04)

Hotfix release. Removes vsdd-factory release-version references
(`v0.X`, `v1.X`, `beta.N`, `rc.N`) from skill and agent prose. The
version refs were causing Claude Code's plugin resolver to pull from
older cached plugin versions during installation when it found those
version strings inside skill bodies — a real install-time blocker
that affected operators on fresh checkouts.

Pure prose-only release. No code, test, or behavioral changes; the
9 modified skill/agent .md files now describe BEHAVIOR rather than
which release first introduced each capability.

### Changed

- **Skill bodies cleaned of vsdd version refs** (PR #80):
  - `skills/activate/SKILL.md` — 5 refs removed (v1.0+, v1.0, v0.79.x,
    v2.1.84+ Claude Code, S-* IDs, dated design-doc path)
  - `skills/deactivate/SKILL.md` — 5 refs removed (v1.0, v0.79.x,
    v1.0-beta)
  - `skills/factory-obs/SKILL.md` — "Since v0.78.0" → timeless prose
  - `skills/claude-telemetry/SKILL.md` — 3 refs removed (v0.76.0,
    v0.72.0+)
  - `skills/onboard-observability/SKILL.md` — pre-v0.76.0 ref
    removed from comment block
  - `skills/release/SKILL.md` — 4 hardcoded version examples in
    dry-run output replaced with `<NEXT>` / `<CURRENT>` placeholders
  - `skills/adversarial-review/SKILL.md` — cycle-name example
    parametrized
  - `skills/factory-cycles-bootstrap/SKILL.md` — cycle-name examples
    parametrized
  - `agents/orchestrator/steady-state.md` — 2 refs to "v1.0.0
    release" generalized to "first stable release"

### Intentionally NOT changed (legitimate retentions)

- Spec artifact IDs (BC-X.X, S-X.X, FR-X) — stable design-artifact
  identifiers, intended to be permanent
- Spec/template versions in spec-versioning skill (that skill IS
  about versioning; examples are the point)
- Generic semver guidance in devops-engineer (about releasing OTHER
  projects)
- Third-party tool versions (`excalidraw-brute-export-cli v0.4.0+`)
- Illustrative example filenames in sdk-generation
  (`openapi-v1.0.0.yaml` etc.)

## 1.0.0-rc.9 — PowerShell siblings for activate-skill helpers (2026-05-04)

Release candidate 9 closes the activate-skill platform-coverage gap that
opened in Claude Code v2.1.84+ on native Windows hosts without Git for
Windows installed (where Claude Code falls back to PowerShell and cannot
execute the bash `detect-platform.sh` / `apply-platform.sh` helpers).

Beyond the PowerShell sibling work, two adjacent threads landed:
- a regression-v1.0 test was rewritten to use a still-adapter-routed
  bash hook (`purity-check.sh`) instead of the WASM-migrated
  `capture-commit-activity` (the original test premise was invalidated
  by commit 818fb95)
- a research investigation surfaced a separate, larger telemetry-pipeline
  issue (WASM `host::emit_event` routes events to
  `dispatcher-internal-*.jsonl` instead of `events-*.jsonl`, silently
  losing every WASM-emitted event from every downstream consumer). That
  work is captured by ADR-015 and the new E-TELEMETRY epic — NOT shipped
  in rc.9.

### Added

- **`plugins/vsdd-factory/skills/activate/detect-platform.ps1`** — PowerShell
  sibling of `detect-platform.sh`. Identical JSON contract, exit codes
  (0/1/2), and `MOCK_UNAME_S`/`MOCK_UNAME_M` test-override envvars.
  Implements case-sensitive platform mapping via explicit if/elseif chains
  (PowerShell `switch` is case-insensitive by default and runs all matching
  arms — bash `case` is first-match-wins). Hand-built JSON output for
  byte-identical parity with the bash sibling's `jq` output. Honors
  `Set-StrictMode -Version Latest` for `-u`-equivalent strictness.
- **`plugins/vsdd-factory/skills/activate/apply-platform.ps1`** — PowerShell
  sibling of `apply-platform.sh`. Identical exit codes (0/1/2/3/4), `-Check`
  verify-only mode, `VSDD_PLUGIN_ROOT_OVERRIDE` test override, diagnostic
  strings. Accepts both PowerShell-style flags (`-Check`, `-Help`) and
  bash-style aliases (`--check`, `--help`, `-h`) so cross-shell muscle
  memory does not produce exit-4 surprises. Guards `$PSScriptRoot` empty-
  state (when invoked via `pwsh -Command`). Narrows the UnixFileMode
  exception handling to `PlatformNotSupportedException` so genuine I/O
  errors surface instead of silently downgrading exit-3.

### Changed

- **`plugins/vsdd-factory/skills/activate/SKILL.md`** — steps 2 and 6 now
  dispatch to the right helper (`.sh` vs `.ps1`) based on active shell.
  JSON output and exit codes are byte-for-byte identical between the two
  implementations; only invocation syntax differs by host shell convention.
  Notes section explicitly documents flag-syntax conventions and the
  TD-019 deferral for full Pester test parity.

### Fixed

- **`plugins/vsdd-factory/tests/regression-v1.0.bats` test 9** ("bash-side
  events land in events-*.jsonl when routed through the adapter") —
  rewritten to use `purity-check.sh` (a bash hook still routed through
  legacy-bash-adapter on `PostToolUse/Edit|Write`) instead of
  `capture-commit-activity` (which migrated to native WASM in commit
  818fb95 and no longer goes through the adapter). Test now correctly
  pins the bash-via-adapter path's contract that side-effecting bash
  hooks land their events in `events-*.jsonl`. The WASM-side gap (events
  landing in `dispatcher-internal-*.jsonl` instead) is captured by
  ADR-015 and the E-TELEMETRY epic.

### Tech debt logged

- **TD-019 (P2, v1.1)** — Pester test suite + Rust consolidation of
  `apply-platform.{sh,ps1}` into a `factory-dispatcher activate
  --platform <p>` subcommand
- **TD-019a (P2, v1.0.1)** — Lightweight `pwsh` syntactic-validation
  CI gate as the v1.0-cycle stopgap before the full TD-019(a) Pester
  suite lands
- **LESSON-2026-05-04-001 [process-gap]** — Story marked "shipped"
  without AC verification (S-3.04 AC-001 false-shipped); follow-up via
  S-7.04
- **LESSON-2026-05-04-002 [process-gap]** — Dashboards query fields no
  plugin emits (`pr.opened` vs `pr.created`; `open_to_merge_seconds`
  never emitted); follow-up via S-7.05
- **TD-007 amendment** — original "S-3.04 AC-003 deferred" claim
  amended; AC-001 was actually never wired (Router::submit absent from
  main.rs since April 24, 2026); E-TELEMETRY epic addresses it via
  ADR-015 and a new wave of stories

### Not shipped (deferred to E-TELEMETRY epic)

- WASM `host::emit_event` routing fix (currently lands events in
  `dispatcher-internal-*.jsonl`; should land in `events-*.jsonl`)
- Per-project / per-worktree / per-instance event-attribute drill-down
- OTel-aligned schema with full Resource attributes
- `pr.opened`/`pr.created` reconciliation
- `plugin_version` correctness fix (currently always equals dispatcher
  version, not the plugin's actual version)
- Full bash↔WASM emit_event field parity

These are tracked under ADR-015 and decomposed into stories under the
new E-TELEMETRY epic. Adversarial review of ADR-015 is in flight.

## 1.0.0-rc.8 — Hooks-test migration to hooks-registry.toml (2026-05-04)

Release candidate 8 finishes the S-0.4 hooks.json untracking properly.
The rc.7 hotfix #2 re-tracked hooks.json as a workaround when the
bats suite broke without it; rc.8 migrates those tests to assert
against `hooks-registry.toml` (the actual source of truth) and
removes the "dual tracked + gitignored" hack.

### Removed

- **`plugins/vsdd-factory/hooks/hooks.json` — untracked cleanly** (was
  in the repo as both tracked and gitignored, a contradictory state
  introduced in rc.7 hotfix #2 to satisfy 11 bats suites that asserted
  against this file). The file is now per-machine only — populated by
  `/vsdd-factory:activate` copying the right `hooks.json.<platform>`
  variant into place. S-0.4 design intent fully restored.

### Fixed

- **13 bats suites migrated to assert against `hooks-registry.toml`
  instead of `hooks.json`.** Pre-rc.8 pattern checked the per-machine
  activation output:
  ```bash
  jq -e '.hooks.PostToolUse[].hooks[] | select(.command | contains("X"))' hooks.json
  ```
  Post-rc.8 pattern checks the canonical source of truth (what the
  dispatcher actually loads):
  ```bash
  registry_has_hook "X" "PostToolUse"
  ```
  27 individual `@test` blocks updated across `corpus-lint`,
  `convergence-tracker`, `destructive-guard`, `finding-format`,
  `input-hash`, `novelty-assessment`, `policy-enforcement`, `policy9`,
  `pr-lifecycle-hooks`, `protect-secrets`, `state-health`,
  `template-compliance`, `wave-gate-hooks`. All migrated tests pass
  locally and are stable across the activate boundary.
- **BC-9.01.005 Rust test** (asserts hooks.json IS gitignored)
  continues to pass — `.gitignore` entry intact; only the
  dual-tracked state was removed.

### Added

- **`plugins/vsdd-factory/tests/helpers/registry.bash`** — shared
  bats helper with two assertion functions:
  - `registry_has_hook NAME [EVENT] [TOOL]` — verifies a hook is
    registered with optional event + tool matcher specificity.
  - `registry_has_script SCRIPT_PATH` — verifies a legacy bash hook
    is wired through `legacy-bash-adapter.wasm` (matches `script_path`
    field).

### Migration

No user-facing changes. The plugin install + activate flow is
unchanged. Existing rc.7 users:

```
/plugin marketplace update claude-mp
/plugin update vsdd-factory@claude-mp
/reload-plugins
```

After update, the cache will NOT contain a `hooks/hooks.json` file
(correct per S-0.4). The file gets created on `/vsdd-factory:activate`
from the appropriate `hooks.json.<platform>` variant. If you have an
old `hooks.json` from a pre-rc.8 install, it will be overwritten by
the next activate. No manual cleanup required.

## 1.0.0-rc.7 — Marketplace split to drbothen/claude-mp (2026-05-03)

Release candidate 7 finishes what rc.6 attempted: splits the marketplace
out of this repo into a separate `drbothen/claude-mp` repository so
`/plugin install` actually populates the cache and surfaces all 120
`/vsdd-factory:*` slash commands. The rc.6 attempt with `git-subdir +
ref=main` was schema-correct but empirically still broken in our
self-referential same-repo layout — the cache-populate step silently
failed identically to the schema-invalid `github + path` form shipped in
rc.2-rc.5. Moving the marketplace to a separate repo puts vsdd-factory
on the well-tested cross-repo install path used by every working sibling
plugin (Semgrep, dclaude, wclaude, zclaude, etc.).

### Removed

- **`.claude-plugin/marketplace.json`** — deleted from this repo. The
  marketplace catalog now lives at
  [drbothen/claude-mp](https://github.com/drbothen/claude-mp), pointing
  back at this repo via `git-subdir + ref=main`. claude-mp owns its own
  version field; it gets bumped out-of-band when this repo ships a new
  release.
- **release.yml `marketplace.json` write step** — `commit-binaries` job
  no longer writes a version into a file that doesn't exist anymore.
  `plugin.json` is still written from the tag atomically with the
  bundled binaries (the cache-staleness fix from beta.4 still applies).
- **`scripts/bump-version.sh` marketplace.json references** — the script
  no longer reads or displays a `marketplace.json` version. Only
  `plugin.json` is referenced as display-only context.

### Fixed

- **`/plugin install vsdd-factory@claude-mp` now correctly populates
  `~/.claude/plugins/cache/claude-mp/vsdd-factory/<version>/`** with all
  120 SKILL.md files, 44 agent files, 36 hook entries, and the LSP
  server. Verified via clean-room install: cache directory is created,
  skill count > 0, all `/vsdd-factory:*` slash commands appear in the
  picker.
- **Untracked `plugins/vsdd-factory/hooks/hooks.json`** per S-0.4 design
  intent. The base `hooks.json` is per-machine (created by
  `/vsdd-factory:activate` copying the right `hooks.json.<platform>`
  variant into place); only the template + 5 per-platform variants
  belong in the repo. Some prior commit re-tracked the base file, and
  it was never updated as bash hooks were ported to native WASM in
  S-8.x — accumulating 3 stale references to deleted .sh files
  (`session-learning.sh`, `handoff-validator.sh`, `track-agent-stop.sh`)
  that fired Stop / SubagentStop hooks as "No such file or directory"
  errors. Fix: `git rm --cached` the base file. New installs will not
  have hooks active until the user runs `/vsdd-factory:activate`.
- **The `sync-develop` job in `release.yml`** still runs to keep develop
  in sync with main's binary-bundle commits, but its rationale is
  documented now as "git workflow consistency" rather than the
  pre-rc.7 "marketplace.json freshness" reason that no longer applies.
- **`block-ai-attribution` and `capture-pr-activity` WASM hook plugins
  now load successfully.** Both crates were silently crashing on every
  fire with `plugin file not found` — Cargo's default `[lib]` name
  conversion produced underscored output filenames
  (`block_ai_attribution.wasm`, `capture_pr_activity.wasm`) but
  `hooks-registry.toml` references the hyphenated forms
  (`block-ai-attribution.wasm`, `capture-pr-activity.wasm`). Cargo
  doesn't allow hyphens in `[lib]` names, so the only way to produce
  hyphenated wasm output is via `[[bin]]` (which does allow hyphens).
  Migrated both crates to the canonical `[[bin]]` WASI command pattern
  matching `capture-commit-activity` and `handoff-validator`: added
  `src/main.rs` with a thin `on_hook` wrapper that calls into the
  lib's existing logic, removed `crate-type = ["cdylib", "rlib"]`
  from `[lib]`, added `[[bin]] name = "<hyphenated>"`. Empirically
  verified: PreToolUse fires now show `block-ai-attribution: exit 0`
  (was crash); PostToolUse with `gh pr create` now emits a real
  `pr.created` event for the first time. Both crates' existing
  unit-test suites continue to pass (43 tests for block-ai-attribution,
  71 tests for capture-pr-activity) since the lib logic was untouched.

### Added (CI)

- **`bump-marketplace` job in `release.yml`** — auto-opens a PR against
  `drbothen/claude-mp` after each release tag, bumping the
  `vsdd-factory` plugin entry's `version` field. Self-skips when
  `CLAUDE_MP_PAT` secret is not configured. To enable: create a PAT
  with `contents: write` + `pull-requests: write` scopes on
  `drbothen/claude-mp` and add it as `CLAUDE_MP_PAT` in this repo's
  Actions secrets. The job opens a PR (not a direct push) so the
  operator gets a review gate.

### Added

- **`docs/guide/plugin-marketplace-architecture.md`** — fully rewritten
  to capture the actual rc.2 → rc.6 → rc.7 saga, the empirical
  observation that `git-subdir` is only verified to work in cross-repo
  installs (NOT same-repo with `path:`), and the validation discipline
  (clean-room install + cache directory check) that catches install
  bugs `claude plugin validate` doesn't.
- **README + getting-started + migrating-from-0.79 install instructions**
  updated to use the new `drbothen/claude-mp` marketplace.
- **Migration note in `migrating-from-0.79.md`** flagging the
  marketplace change for operators upgrading from v0.79.x and skipping
  past rc.6.

### Migration

**Action required for ALL users on rc.6 or earlier.** The marketplace
identifier changed from `vsdd-factory@vsdd-factory` to
`vsdd-factory@claude-mp`.

```bash
# In a Claude Code session
/plugin marketplace remove vsdd-factory       # remove the old (broken) marketplace
/plugin marketplace add drbothen/claude-mp    # add the new marketplace
/plugin install vsdd-factory@claude-mp        # install via new marketplace
/vsdd-factory:activate                        # required: populate hooks.json from
                                              #   the right per-platform variant
/reload-plugins                               # pick up new state
```

> **`/vsdd-factory:activate` is REQUIRED** in v1.0+ to populate the
> per-machine `hooks/hooks.json` from the right per-platform variant.
> Without activation, no hooks fire (this is correct behavior — the
> plugin is installed but inactive). Pre-rc.7 a buggy committed base
> `hooks.json` masked this requirement and shipped broken `.sh`
> references; rc.7 removes that file from the repo.

After reload, `/vsdd-factory:*` slash commands should appear in the
picker for the first time since rc.1. Verify the cache directory is
populated:

```bash
ls ~/.claude/plugins/cache/claude-mp/vsdd-factory/1.0.0-rc.7/
# should list: skills/ agents/ hooks/ hook-plugins/ bin/ ...
find ~/.claude/plugins/cache/claude-mp -name SKILL.md | wc -l
# should be 120
```

If the cache directory is empty after install, refer to
`docs/guide/plugin-marketplace-architecture.md` §8 (Validation
discipline) for the clean-room reinstall procedure.

**For repo maintainers only** — after this release ships and the bot
publishes the v1.0.0-rc.7 tag with bundled binaries, manually open a
PR against `drbothen/claude-mp` bumping the `vsdd-factory` plugin
entry's `version` field from `1.0.0-rc.6` to `1.0.0-rc.7`. Future
releases should automate this; for now it's a manual one-liner.

## 1.0.0-rc.6 — Marketplace install fix (2026-05-03)

Release candidate 6 fixes a silent install-time bug shipped in rc.4 and
rc.5 that prevented Claude Code from populating the plugin cache, which
in turn prevented `/vsdd-factory:*` slash commands and skills from
loading at all. The marketplace clone was correct on disk and
`claude plugin validate` reported success — but the plugin loader found
zero skills because the cache install path recorded in
`installed_plugins.json` never actually got created.

### Fixed

- **`marketplace.json` `source` field now uses the documented
  `git-subdir` form with explicit `ref: "main"`.** Previous releases
  shipped a schema-violating combination — `source: "github"` with a
  `path:` field — that no Claude Code source type officially supports.
  The install code path silently skipped the cache copy step when
  encountering the unrecognized field combination, leaving
  `~/.claude/plugins/cache/vsdd-factory/vsdd-factory/<version>/`
  empty even though `installed_plugins.json` recorded a successful
  install. Replaced with:
  ```json
  "source": {
    "source": "git-subdir",
    "url": "https://github.com/drbothen/vsdd-factory.git",
    "path": "plugins/vsdd-factory",
    "ref": "main"
  }
  ```
  `git-subdir` is the documented source type for "plugin at a subpath
  of an external git repo," and `ref: "main"` pins installs to the
  release-stable branch independent of the marketplace clone's
  checked-out ref. The latter matters because we use Git Flow — the
  GitHub default branch is `develop` (in-flight), but releases live on
  `main`. Without explicit ref pinning, future re-registrations of the
  marketplace could silently start pulling pre-release work from
  develop instead of release-stable code from main.

### Added

- **`docs/guide/plugin-marketplace-architecture.md`** — comprehensive
  reference for the plugin packaging architecture. Documents:
  - All 5 valid `source` types (`relative-path string`, `github`,
    `url`, `git-subdir`, `npm`) with when-to-use guidance
  - A decision tree that accounts for ref-pinning needs under Git Flow
  - The full root cause of the rc.4/rc.5 bug and why
    `claude plugin validate` failed to catch it
  - Repository layout today and future evolution paths (central
    marketplace, release channels for stable vs edge)
  - Complete `SKILL.md` frontmatter field reference
  - Common pitfalls catalogued during the rc.5 audit (blank lines in
    YAML frontmatter, comma-separated `allowed-tools` instead of
    space-separated, `disable-model-invocation` interactions with
    auto-loading)
  - Validation discipline checklist for future marketplace changes

### Migration

No breaking user-facing changes for installed plugins. Users who
installed rc.5 (or earlier) and saw the "0 skills" issue should see
slash commands appear after upgrading to rc.6 and running
`/plugin update vsdd-factory@vsdd-factory` followed by
`/reload-plugins`. Users on a clean install should see the issue go
away on first install.

If `/plugin update` reports already-installed at a non-rc.6 version but
the cache directory is empty, force a fresh install:

```bash
claude plugin uninstall vsdd-factory@vsdd-factory --scope project
# (or --scope user, depending on how it was originally installed)
# then /plugin install vsdd-factory@vsdd-factory
```

If maintaining a marketplace registration that points at this repo,
no action needed — the marketplace.json shipped on `main` is now
self-correcting.

## 1.0.0-rc.5 — Skills-only plugin surface (2026-05-03)

Release candidate 5 collapses the dual command + skill plugin surface
into a single skills-only surface and closes the model-auto-invocation
gap on the activate/deactivate lifecycle skills. Both changes follow
the marketplace pattern shipped by sibling plugins (dclaude, wclaude,
zclaude), which expose `/<plugin>:<skill>` slash commands directly
from skill frontmatter without parallel command shim files.

### Removed

- **`plugins/vsdd-factory/commands/` directory (111 files)** — every
  command was a thin shim of the form `Use the vsdd-factory:<name>
  skill via the Skill tool`. The Claude Code plugin loader already
  exposes plugin skills as `/vsdd-factory:<name>` slash commands from
  SKILL.md frontmatter; the shim files were pure duplication. All
  command descriptions had drifted from their skill counterparts (111
  of 111 differed in wording), making the directory a maintenance
  liability with no user-facing benefit.
- **8 stale "command file exists" bats assertions** —
  `tests/input-hash.bats`, `tests/skills.bats`, and
  `tests/state-health.bats` each had assertions checking that a
  parallel command shim existed alongside its skill. Removed alongside
  the shims; the sibling "skill exists" assertions remain and are the
  real existence check.

### Fixed

- **`activate` and `deactivate` skills now set
  `disable-model-invocation: true`** — both skills write to
  `.claude/settings.local.json` (activate flips the default
  main-thread agent to `orchestrator` and persists the host platform;
  deactivate reverses it). Without the flag, the plugin loader
  exposed both as ambient tools the model could choose to call
  unprompted. With the flag set, Claude's tool catalog no longer
  lists them, but `/vsdd-factory:activate` and
  `/vsdd-factory:deactivate` continue to work as user-typed slash
  commands.

### Added

- **`Examples` block in `skills/create-adr/SKILL.md`** — ported from
  the deleted `commands/create-adr.md`. The skill already carried the
  Arguments table; only the Examples block was unique to the command
  shim and worth preserving.

### Migration

No breaking user-facing changes. All 120 skills remain invokable via
`/vsdd-factory:<skill-name>`. Argument passing is unchanged: skills
that reference `$ARGUMENTS` in their body continue to substitute
inline; skills that don't continue to receive an auto-appended
`ARGUMENTS:` line from the plugin loader. The 9 skills that never had
a command shim (`phase-0-codebase-ingestion` through
`phase-7-convergence` and `state-burst`) are unaffected — they were
already orchestrator-dispatched.

If any out-of-tree tooling shelled out to `plugins/vsdd-factory/commands/<name>.md`
directly (rather than invoking via `/vsdd-factory:<name>`), update it
to read `plugins/vsdd-factory/skills/<name>/SKILL.md` instead. No
in-repo references to the commands directory remain.

## 1.0.0-rc.4 — Pre-W16 hardening sprint (2026-05-03)

Release candidate 4 ships pre-flight hardening for the W-16 (Tier 2 native
WASM) wave. Addresses orphan-reference fragility surfaced by W-15's release
recovery cycles, sweeps clippy debt, restores main branch protection with bot
bypass, and removes legacy bash hook dupes that have native WASM equivalents.

### Added

- **bats-orphan-detection CI gate (TD-017)** — `.github/workflows/ci.yml`
  validate job now runs `plugins/vsdd-factory/tests/check-bats-orphans.sh`,
  which scans every `tests/*.bats` for `hooks/<name>.sh` references and fails
  if any referenced hook is missing from `plugins/vsdd-factory/hooks/`.
  Detects deletion-driven orphans pre-merge.
- **`plugins/vsdd-factory/tests/check-bats-orphans.sh`** — standalone shell
  script enforcing the gate above; invokable locally as `bash
  plugins/vsdd-factory/tests/check-bats-orphans.sh`.

### Changed

- **`plugins/vsdd-factory/tests/run-all.sh` glob discovery (TD-016)** —
  Replaced 23-line hardcoded suite enumeration with 7-line `nullglob` loop.
  New `.bats` files are picked up automatically; deleted ones don't break CI
  with stale references. Also: continues past first-failure suite, accumulates
  failed-suite list, and reports total fail count at the end (was: aborted on
  first failure under `set -e`).
- **main branch protection restored (TD-013)** — `enforce_admins=false`,
  `required_pull_request_reviews=null` (allows `github-actions[bot]` direct
  push for release.yml bundle commits), `required_status_checks=null`,
  `allow_force_pushes=false`, `allow_deletions=false`. Replaces fully-disabled
  state from W-15 recovery cycles.
- **Workspace clippy debt swept (TD-018)** — `cargo clippy --workspace
  --all-targets -- -D warnings` is now clean. Side effect: 6 pre-existing
  test failures in `generate-registry.bats` fixed (the migration generator
  was clobbering hand-maintained `hooks-registry.toml` during test runs).

### Removed

- **3 legacy bash hook dupes deleted** — `block-ai-attribution.sh`,
  `capture-commit-activity.sh`, `capture-pr-activity.sh`. All have native
  WASM equivalents shipped in W-15. Their dedicated `.bats` files removed;
  references in `pr-lifecycle-hooks.bats` trimmed; `hooks.json` entries
  removed.

### Fixed

- **Orphan references in `factory-obs.bats` and `regression-v1.0.bats`** —
  Tests asserting deleted .sh executables removed; replaced with sentinel
  comments explaining the WASM port.
- **`check-bats-orphans.sh` regex coverage** — broadened from `$HOOKS_DIR/`
  pattern to all `hooks/<name>.sh` reference forms (catches
  `${BATS_TEST_DIRNAME}/../hooks/`, `$PLUGIN_ROOT/hooks/`, `${CLAUDE_PLUGIN_ROOT}/hooks/`,
  hardcoded paths, etc.).

### Tech Debt Backlog (post-rc.4)

- TD-019 — `ci.yml` triggers on `pull_request: [main]` only; PRs targeting
  develop don't run CI gates. Will be addressed as a follow-up before rc.5
  or v1.0.0 GA.

## 1.0.0-rc.3 — W-15 convergence + wave-gate fixes (2026-05-02)

Release candidate 3 ships the completed W-15 wave gate (12 stories merged)
plus three post-merge fix-bursts (PRs #59, #60, #61) that closed all CRIT/HIGH
findings before tagging. First rc that fully exercises the native WASM plugin
pipeline with dispatcher binary bundling across all 5 platforms.

### Fixed

- **[W-15-gate / PR #59]** Critical and high pre-rc.3 findings: dispatcher
  AND-gate logic, tool_input field regression, dead fixture cleanup, and
  HOST_ABI.md alignment.
- **[W-15-gate-2 / PR #60]** Dispatcher AND-gate re-verification + tool_input
  regression confirmed resolved; integration test coverage added.
- **[W-15-gate-3 / PR #61]** HOST_ABI.md spec alignment with implementation;
  removed stale test fixtures that were masking true coverage gaps.

### Added

- **W-15 wave convergence (12 stories merged)** — Native WASM port of all
  SubagentStop and Stop hook plugins complete; legacy bash adapter retained as
  routing shim. All 16 registry entries now route through the native dispatcher.
- **`host::write_file` SDK API** — hook plugins can write to host-allowlisted
  paths via `WriteFileCaps` capability schema; HOST_ABI_VERSION stays at 1
  (additive-only, D-6 Option A). See BC-2.02.011.
- **advisory-block-mode pattern** documented in HOST_ABI.md — dispatchers
  can surface advisory blocks without hard-failing the hook chain.

### Migration

No breaking changes. All slash commands, auto-invocation, and orchestrator
dispatch continue to work unchanged. The native dispatcher replaces bash
adapter execution paths transparently; no operator configuration required.

## [0.2.0] - 2026-05-02

### Added
- `host::write_file(path, contents, max_bytes, timeout_ms) -> Result<(), HostError>` SDK API for hook plugins to write to host-allowlisted paths. New WriteFileCaps capability schema (`path_allow`, `max_bytes_per_call`). HOST_ABI_VERSION unchanged at 1 (D-6 Option A additive-only). See BC-2.02.011 for full invariants.

### Security
- Fixed path-traversal vulnerability in `path_allowed()` for both `read_file` and `write_file` dispatcher bindings: paths are now canonicalized before the allowlist prefix check, preventing `../` escapes from bypassing capability gates. (BC-2.02.011 EC-001 / BC-2.02.001 EC-001 — same bug existed in both host functions.)

## 1.0.0-rc.2 — Release Candidate 2 (skill cleanup) (2026-05-01)

Bug-fix release focused on plugin frontmatter cleanup and CI/CD reliability.
The headline change: 29 skills had `disable-model-invocation: true` set, which
was preventing the orchestrator (which runs as the model) from dispatching
those skills via the Skill tool — forcing fallback to ToolSearch and the
"load multiple times then search" UX friction. This release removes that
flag from all 29 skills. Real safety lives in the skill body, not in the flag.

### Fixed

- **commands/claude-telemetry.md** — quoted the description so YAML parses;
  the unquoted "Default: on" fragment was a YAML mapping-separator bug that
  caused `claude plugin validate` to fail.
- **skills/generate-pdf** — renamed `tools:` to `allowed-tools:` (the
  former was silently ignored, meaning the skill ran without its declared
  tool pre-approvals).
- **agents/session-reviewer.md → agents/session-review.md** — renamed the
  file to match its `name: session-review` field and the matching skill
  convention. Workflow refs already used `session-review`.
- **9 orchestrator reference docs** — removed meaningless
  `disable-model-invocation: true` from sequence playbooks (the flag is
  skill-only; agents are invoked via `Agent(subagent_type=...)`).

### Added

- **`argument-hint` on 4 skills** — `claude-telemetry`, `factory-obs`,
  `run-phase`, `validate-workflow` now show their accepted arguments in
  slash-command autocomplete.
- **Marketplace JSON sync to develop** (release.yml `sync-develop` job and
  inline best-effort path in `commit-binaries`) — closes a Git Flow gap
  discovered during rc.1: Claude Code's plugin marketplace reads
  `marketplace.json` from the GitHub default branch (develop), but the
  post-tag bot writes version bumps only to main. Without these safeguards,
  develop's `marketplace.json` was always one release behind. This rc.2
  release is the first real-world test of the new path.
- **Marketplace `ref: "main"` pin** — `marketplace.json` plugin source now
  explicitly pulls from `main` so end-user installs always get released
  code regardless of which branch was read.

### Changed

- **29 skills no longer have `disable-model-invocation: true`** — affects
  pipeline ops (state-update, track-debt, check-state-health, factory-health,
  validate-consistency, convergence-check, wave-gate, holdout-eval,
  formal-verify, perf-check, spec-drift, dtu-validate, worktree-manage,
  pr-create, record-demo, setup-env), phase playbooks (create-prd,
  create-architecture, create-story, create-domain-spec, create-brief,
  create-adr, decompose-stories, research), and a handful of user-action
  skills. Orchestrator and sub-agents can now dispatch all of these via
  the Skill tool without "Unknown skill" fallback.

### Migration

No breaking changes for end-user plugin invocations. Slash commands,
auto-invocation, and orchestrator dispatch all continue to work; the
skill cleanup eliminates failure modes rather than introducing new ones.

For developers extending the plugin: if you'd added new skills with
`disable-model-invocation: true` for safety, consider whether real safety
should instead live in the skill body (explicit confirmation prompts,
prerequisite checks). The flag is not the right tool for orchestration
gates — see PR #45 for rationale.

## 1.0.0-rc.1 — Release Candidate 1 (2026-04-29)

First release candidate for v1.0.0 GA. Closes the 4-month beta cycle by shipping all
remaining v1.0 epics (Wave 11/12/13/14). WASM plugin ecosystem fully wired (DRIFT-006
closed; 4/4 lifecycle events). Multi-sink observability stack (file/HTTP/Datadog/Honeycomb
with retry/circuit-breaker/DLQ/routing/tag-enrichment) complete. Operator migration guide
and semver commitment documentation shipped.

### Added

- **Wave 11** — WASM plugin ports: capture-commit-activity (S-3.01), capture-pr-activity
  (S-3.02), block-ai-attribution (S-3.03); sink-http retry/jitter (S-4.09);
  cross-sink internal.sink_error events (S-4.10).
- **Wave 12** — Dead Letter Queue with byte-counted rotation (S-4.05); per-sink routing
  filters + tag enrichment (S-4.06); 40-test E2E observability integration suite (S-4.07);
  rc.1 release gate spec with 20 ACs (S-4.08).
- **Wave 13** — SessionStart hook (S-5.01); SessionEnd hook (S-5.02);
  WorktreeCreate/Remove hooks (S-5.03); PostToolUseFailure hook (S-5.04).
  DRIFT-006 fully closed.
- **Wave 14** — Operator migration guide v0.79.x → v1.0 (S-5.05);
  semver commitment documentation (S-5.06).
- Semgrep SAST workflow (`.github/workflows/semgrep.yml`), image pinned to 1.61.0.
- `deny.toml` cargo-deny 0.19 schema with permissive license allow-list (AC-Q4).

### Fixed

- deny.toml: removed invalid `unmaintained = "warn"` field incompatible with cargo-deny 0.19.
- Semgrep: 19 false-positive findings suppressed inline with justified `nosemgrep` comments.
- Stale `loads_legacy_registry` tests removed (v0.79.x migration complete).
- `sinks_file_integration` test: corrected sink type from `datadog` to `splunk` (S-4.07 drift).

### Migration

No breaking changes from v1.0.0-beta.7. Operators on v0.79.x: see
`docs/guide/migrating-from-0.79.md` for the full upgrade guide.

## 1.0.0-beta.7 — S-7.03 TDD Discipline Hardening (2026-04-26)

This release ships the third E-7 self-referential dogfooding round: a 4-layer
structural defense against the stub-as-implementation anti-pattern, codified
into the plugin's own dispatcher prompts, hooks, and templates.

The motivating evidence: during Prism Wave 2, 3 of 5 parallel stub-architect
dispatches produced stubs with full business logic instead of `todo!()` bodies.
Root cause was precedent cascade — earlier merged DTU crates contained
pre-implemented stubs that Wave 2 stub-architects loaded as contextual
templates, silently bypassing the TDD Iron Law's Red Gate. This release
codifies four interlocking defenses so future cycles cannot recur.

### Added

- **Layer 1 — Stub obligations + GREEN-BY-DESIGN / WIRING-EXEMPT protocol.**
  `agents/stub-architect.md` (+187 LOC) gains explicit `todo!()` obligation
  citing BC-5.38.001 for non-trivial function bodies in `tdd_mode: strict`
  stories, plus a verbatim self-check rule (BC-5.38.005 invariant 1):
  "Before including any non-todo!() function body, apply self-check: 'If I
  include this real implementation, will the test for this function pass
  trivially without any implementer work?'" The agent now reports
  inline-implemented functions as `GREEN-BY-DESIGN` (pure data mappings —
  enum labels, zero-logic accessors, body ≤ 3 lines) or `WIRING-EXEMPT`
  (framework integration scaffolding like Tower `Service::poll_ready`).
  Both classes are excluded from the Red Gate denominator.

- **Layer 2 — Anti-precedent guard text in dispatch files.**
  `skills/deliver-story/SKILL.md` and `workflows/phases/per-story-delivery.md`
  Step 2 sections now include a verbatim guard block citing four canonical
  Prism SHAs: anti-precedent commits `aa706543`, `6d2d005e`, `20b4a12a`
  (Wave 2 violations) and model precedent `e86d03f2` (S-2.06 datasource-trait
  with 5 genuine `todo!()` macros). Both files updated atomically per
  BC-5.38.006 invariant 1.

- **Layer 3 — Red Gate Density Check (BLOCKING before Step 4).**
  New section in `per-story-delivery.md` between Step 3 and Step 4 enforcing
  `RED_RATIO ≥ 0.5` where `RED_RATIO = RED_TESTS / (TOTAL_NEW_TESTS - EXEMPT_TESTS)`
  and `EXEMPT_TESTS = GREEN-BY-DESIGN + WIRING-EXEMPT`. Two remediation
  options when ratio falls below threshold: Option A rolls back the stub
  and re-dispatches with stricter prompt (default for automated orchestrators
  per BC-8.29.003 EC-001); Option B accepts with `mutation_testing_required:
  true` frontmatter, electing mutation testing at the wave gate as the
  compensating control. New PostToolUse hook `hooks/validate-red-ratio.sh`
  (+119 LOC) enforces the threshold mechanically when red-gate-log files
  are written; logs go to `.factory/logs/red-gate-log-<story-id>.md` with
  per-test rationale categories (PURE-DATA / FRAMEWORK-WIRING /
  STRUCTURAL-ASSERTION / PRE-EXISTING-BEHAVIOR / OTHER-JUSTIFIED /
  UNJUSTIFIED). Hook registered in `hooks-registry.toml`.

- **Layer 4 — `tdd_mode` frontmatter contract.**
  `templates/story-template.md` gains a `tdd_mode: strict | facade` field
  with default-to-strict semantics (BC-8.30.001 invariant 2 — absent or
  unrecognized values default to `strict` so no existing story silently
  promotes to facade mode). Strict mode enforces the full TDD Iron Law
  with the new Red Gate density check; facade mode (intended for DTU
  clones and mock infrastructure where the scaffold IS the implementation)
  permits combined scaffold+impl commits and bypasses the Red Gate, with
  the cargo-mutants wave gate as the compensating control. New facade-mode
  delivery flow section in `per-story-delivery.md` documents the
  modifications to Steps 2/3/4 and the wave gate. `agents/story-writer.md`
  frontmatter checklist updated to require `tdd_mode` declaration.

- **Layer 5 — Mutation testing wave-gate.**
  `skills/wave-gate/SKILL.md` (+94 LOC) gains a Mutation Testing section
  that scans all stories in the wave for `tdd_mode: facade` and
  `mutation_testing_required: true` (Option B election), runs
  `cargo mutants -p <crate> --jobs $(nproc) --timeout 300`, and gates on
  ≥80% kill rate. Survivors above the 20% allowance enter a disposition
  table (A: new test committed + re-run kills mutant; B: dead-code-equivalent
  with execution condition; C: explicit waiver naming mutant + file + line
  + mutation type — no blanket waivers). 60-minute wave-level mutation
  budget. Empty-wave path: explicit "no facade stories — mutation step
  skipped" log entry, never silent omission.

- **Verification suite.** `tests/tdd-discipline-gate.bats` (+242 LOC, 18
  tests across 5 layers) covers all 11 acceptance criteria with grep-based
  assertions plus 3 hook-invocation tests for `validate-red-ratio.sh`
  (low-ratio block, sufficient-ratio pass, Option B election pass).

### Spec Convergence

S-7.03 spec converged through **17 adversarial passes** — the longest in
project history (vs S-6.01: 8 passes, E-7: 7 passes). Trajectory:
25 → 12 → 5 → 2 → 1 → 0 → 0 → 1 → 2 → 4 → 3 → 1 → 1 → 2 → 0 → 0 → 0.
Pass-8 reset on F-301 (Architecture Compliance Rules cited wrong Task
numbers) was BC-5.36.005/006 partial-fix-regression discipline working
as designed. Pass-11 caught a deepest-yet propagation gap: a pass-1
re-anchor of BC-8.30.002 from SS-08 to SS-05 had propagated to BC-INDEX
but not to VP-064 scope/traceability (3 findings F-601/602/603 — 2 HIGH,
1 MED). The exhaustive enumerate-axes-upfront methodology (Path C) was
introduced at pass-13 and produced the final clean-pass triple at
passes 15/16/17.

### Process Notes

- **Bootstrap Exemption pattern formalized.** S-7.03 implements gates
  (Red Gate density, mutation testing) that cannot apply to its own
  delivery cycle — captured explicitly in the story's "Bootstrap Exemption"
  section and recorded as a methodology pattern for future
  self-referential codification stories.
- **Self-validation withdrawal as convergence indicator.** The adversary's
  withdrawal-via-re-arithmetic of hallucinated findings appeared
  pass-by-pass (2 → 2 → 5 → 11 → 6 across passes 13-17), peaking at
  pass-16 then stabilizing — a useful corollary signal alongside finding
  count for declaring true convergence.

### Identifier Counts (post-release)

| Type | Pre-beta.7 | Post-beta.7 |
|------|-----------|-------------|
| BCs (total) | 1,878 | 1,891 (+13) |
| VPs | 62 | 64 (+2 — VP-063 integration, VP-064 manual) |
| FRs | 42 | 43 (+1 — FR-043) |
| Epics | 8 | 8 |
| Stories | 44 | 45 (+1 — S-7.03 status: completed) |
| ADRs | 13 | 13 |

### Migration

No breaking changes. The `tdd_mode` field defaults to `strict` for any
existing or future story that doesn't declare it explicitly — no existing
story is silently promoted to facade mode. The new Red Gate density check
applies only to `tdd_mode: strict` stories (which is everything by default).
The mutation testing wave gate applies only when at least one story in
a wave declares `tdd_mode: facade` or `mutation_testing_required: true`;
otherwise the step is explicitly skipped with a log entry.

The new `validate-red-ratio.sh` hook is read-only — it only inspects
red-gate-log files written by the orchestrator and never modifies any
file. Existing development flows that don't write red-gate-log files
will not invoke the hook.

Per beta.4 cache-staleness fix: this commit only touches CHANGELOG.md.
The release workflow bot will write plugin.json:version + binaries
atomically when v1.0.0-beta.7 tag is pushed.

## 1.0.0-beta.6 — S-6.01 create-adr skill + E-7 process codification (2026-04-26)

This release ships two CONVERGED feature deliveries (S-6.01 + E-7) plus
substantial spec backfill (10 ADRs, 27 BCs, 5 VPs, 2 FRs, 2 epics, 3 stories).
The standout meta-property is self-referential dogfooding: vsdd-factory used
its own VSDD process to find and codify gaps in vsdd-factory itself.

### Added

- **`/vsdd-factory:create-adr` skill (S-6.01).** Closes the per-artifact
  `create-*` skill family gap (alongside `create-prd`, `create-story`,
  `create-architecture`, `create-domain-spec`, `create-brief`). Scaffolds
  new ADR records with: collision-free ID allocation against `decisions/`
  filesystem AND ARCH-INDEX, frontmatter from `templates/adr-template.md`
  with `status=proposed` always at creation, optional bidirectional
  supersession patch (new ADR `supersedes:` ↔ old ADR `superseded_by:`),
  ARCH-INDEX row insertion in numeric order, optional `--brownfield`
  annotation, optional `--dry-run` flag, atomic-or-nothing rollback on
  any failure. Spec converged through 8 adversarial passes (trajectory
  19 → 4 → 2 → 1 → 1 → 0 → 0 → 0); 26 bats tests cover 8 ACs, 12 BCs
  (BC-6.20.001-012), 3 VPs (VP-058/059/060).
- **Process codification (E-7) — 8 lessons codified into plugin source:**
  - **Spec-First Gate** in `agents/story-writer.md`: blocks
    `status=ready` without canonical `BC-N.NN.NNN` pattern in
    `behavioral_contracts:` and AC↔BC bidirectional traces.
    (BC-5.36.001-002)
  - **Capability Anchor Justification** in `agents/product-owner.md`:
    every BC must verbatim-cite `capabilities.md` for its CAP.
    (BC-5.36.003-004)
  - **Partial-Fix-Regression discipline** in `agents/adversary.md`:
    every pass after pass-1 checks prior-pass fix propagation to
    bodies, sibling files, prose; flags `[process-gap]` for
    codification follow-up. (BC-5.36.005-006)
  - **Defensive Sweep discipline** in `agents/state-manager.md`:
    corpus-wide grep before declaring count-changing update complete.
    (BC-5.37.001-002)
  - **Cycle-Closing Checklist** in `agents/orchestrator/orchestrator.md`:
    references `rules/lessons-codification.md` before declaring
    sub-cycle CONVERGED. (BC-8.28.002)
  - **NEW hook `validate-count-propagation.sh`** (BC-7.05.001-002):
    PostToolUse drift detector across PRD / STATE.md / ARCH-INDEX /
    BC-INDEX / VP-INDEX. Exits 2 on drift, runs <500ms.
  - **NEW rule `rules/lessons-codification.md`** (BC-8.28.001):
    meta-discipline — every novel adversary process catch (not content
    defect) MUST trigger codification follow-up before sub-cycle closure.
  - **`validate-template-compliance.sh` extended** (BC-7.05.003) to
    enforce VP multi-BC `source_bc=primary, bcs[]=full list` convention.
  - **`hooks-registry.toml`** registers new hook (BC-7.05.004).

  Spec converged through 7 adversarial passes (trajectory
  12 → 5 → 1 → 2 → 2 → 0 → 0); 16 bats tests cover both stories.
- **Spec backfill — 10 ADRs (ADR-004..013):** TOML config; multi-sink
  observability natively in dispatcher; HOST_ABI_VERSION as separate
  semver constant; always-on dispatcher self-telemetry;
  parallel-within-tier sequential-between-tier execution;
  activation-skill-driven platform binary selection; StoreData-typed
  linker for host functions; dual hooks.json + hooks-registry.toml
  during migration; legacy-bash-adapter as universal current router;
  cycle-keyed adversarial review structure.
- **27 new behavioral contracts** across SS-05/06/07/08 (12 for
  create-adr, 15 for E-7 codification surfaces).
- **5 new verification properties** (VP-058..062 — atomicity, ID
  monotonicity proptest, bidirectional supersession, agent prompt
  static-check, multi-axis process-codification surface invariant).
- **2 new functional requirements** (FR-041, FR-042).
- **2 new epics** (E-6 VSDD Self-Improvement / Tooling Backlog;
  E-7 Process Codification — Self-Improvement).
- **L4-verification-property-template.md schema note** pinning
  `source_bc=primary, bcs[]=full list` convention for multi-BC VPs.

### Fixed

- **`validate-novelty-assessment.sh` false-positive on filename matchers.**
  Hook was matching `ADR-013-adversarial-review-structure.md` as a
  review file. Tightened the matcher to anchor on
  `.factory/cycles/<key>/adversarial-reviews/` directory paths; added
  explicit skip for `*/architecture/decisions/ADR-*.md`.

### Process Notes

The pass-4 reset on the e7-spec sub-cycle (F-020 SS-09 mis-citation
across two artifacts) was BC-5.36.005/006 partial-fix-regression
discipline working as designed: a fresh-perspective adversary caught
a 2-artifact mis-citation that 3 prior passes missed. The pass-1
review of E-7 specs caught a meta dogfood failure: PRD body still
cited "1,863-BC catalog" while the new BCs were being authored to
codify the very defensive-sweep tooling that detects this drift class.
Cumulative findings closed: 27 across 15 spec passes total.

### Identifier Counts (post-release)

| Type | Pre-beta.6 | Post-beta.6 |
|------|-----------|-------------|
| BCs (total) | 1,851 | 1,878 (+27) |
| VPs | 57 | 62 (+5) |
| FRs | 40 | 42 (+2) |
| Epics | 6 | 8 (+2) |
| Stories | 41 | 44 (+3) |
| ADRs | 3 | 13 (+10) |

### Migration

No breaking changes. Codified disciplines apply to NEW spec sub-cycles;
existing artifacts are not retroactively re-validated.

## 1.0.0-beta.5 — ADR template + identifier canonicalization (2026-04-25)

Same-day patch on top of beta.4. Two coupled improvements emerged from
the brownfield onboarding of vsdd-factory itself: a missing ADR
validation template and identifier-convention drift between the
plugin source and prism's working real-world usage.

### Added

- **`templates/adr-template.md`.** A canonical Architecture Decision
  Record template declaring `document_type: adr` plus the canonical H2
  sections (Context, Decision, Rationale, Consequences with
  Positive/Negative/Status sub-headings, Alternatives Considered,
  Source / Origin). The `validate-template-compliance.sh` hook's
  primary lookup-by-document_type now matches ADR files against this
  template instead of falling through to `architecture-section-template`
  (which had been validating ADRs against the wrong schema and
  silently blocking ADR writes during plugin self-onboarding).

### Changed

- **Identifier conventions canonicalized to match prism's working
  pattern.** Three coupled changes across 26 plugin source files
  (templates + rules + 12 skills):
  - **BC shard layout:** flat `behavioral-contracts/BC-*.md` →
    sharded `behavioral-contracts/ss-NN/BC-*.md`. Required for any
    project with more than ~50 BCs.
  - **Story IDs:** `STORY-NNN` → `S-N.MM` (section.story zero-padded;
    e.g., `S-1.01`, `S-3.15`). Section grouping aligns with epics
    (`E-N`) and gives BC-anchoring traversal natural shape.
  - **Epic IDs:** `EPIC-NNN` → `E-N` (single-digit, matches story
    section number).

  Story `N` (section/epic) and BC `S` (subsystem number) are
  intentionally different hierarchies — a story can implement BCs
  from multiple subsystems via its `subsystems: [SS-NN, ...]`
  frontmatter array. This separation is now documented in
  `rules/spec-format.md`.

### Notes

- Pre-rc.1 projects using `STORY-NNN` continue to work — the
  validate-template-compliance hook keys on `document_type`, not ID
  format. New projects from beta.5 onward should use `S-N.MM` from
  the start.
- Phase 2 follow-up (test fixtures, workflows, agents) deferred to a
  future beta; not in this release.

### How discovered

vsdd-factory underwent its own brownfield onboarding cycle
(`v1.0-brownfield-backfill`) on 2026-04-25, producing 1,851 BCs
across 10 subsystems. The flat BC layout broke down at that scale,
exposing the plugin's stale assumption. Phase 1d adversarial review
converged in 6 passes (3 consecutive NITPICK), validating both the
new template and the canonicalized conventions.

## 1.0.0-beta.4 — cache fix + stderr capture + SHA-currency gate (2026-04-25)

Same-day patch on top of beta.3. Three follow-ups from the prior beta
loop, plus the new state-burst skill suite that codifies the wave-gate
bookkeeping defect class. None of these change the dispatcher's hot
path; they each close a paper-cut surface the prior iterations
exposed.

### Fixed

- **Plugin-cache staleness.** Claude Code's plugin-version cache keys
  on `plugin.json:version`. Until now, the operator's chore commit
  bumped that field but the matching binaries didn't exist yet — the
  bot's binary-bundle commit landed ~2 min later. Consumers fetching
  in that window cached "version X with X-1 binaries", and the cache
  never refreshed under that key. Fix: the chore commit no longer
  touches `plugin.json` or `marketplace.json`. The release workflow's
  bot commit reads the tag name and writes both JSON fields atomically
  with the binaries, so consumers never observe `version=X` without
  the matching binaries. **Operator workflow now commits only
  `CHANGELOG.md` for the release chore commit.**

### Added

- **Plugin stderr capture on lifecycle events.**
  `plugin.completed` / `plugin.crashed` / `plugin.timeout` events now
  carry the captured wasm plugin stderr (truncated to 4 KiB with an
  explicit `(stderr truncated)` marker). Empty stderr is omitted from
  the JSON to avoid noise on well-behaved plugins. Diagnoses plugin
  failures without requiring a manual re-run with stdout/stderr
  capture.
- **`vsdd-factory:state-burst` skill** + canonical
  `verify-sha-currency.sh` template + `state-manager-checklist-template.md`
  + agent-prompt updates + `docs/lessons-learned/wave-gate-bookkeeping.md`.
  Codifies the Single Canonical SHA + Two-Commit Protocol that breaks
  a recurring SHA-drift / narrative-staleness defect class observed
  across six consecutive remediation passes in real-world dogfood.
  Operators opt in by copying `templates/verify-sha-currency.sh` into
  their project's `.factory/hooks/` and instantiating
  `templates/state-manager-checklist-template.md` as their checklist.
- **Adversary SHA-currency gate.** The
  `validate-wave-gate-prerequisite.sh` Claude Code hook now branches
  on subagent type. Adversary dispatches additionally call
  `.factory/hooks/verify-sha-currency.sh` (when present) and block on
  FAIL. Without this, the adversary could dispatch against a dirty
  factory-artifacts state and report stale-cite drift as "false
  positive" findings. No-op if the project hasn't installed the
  verify-sha-currency hook.

### Migration

No breaking changes for plugin authors. **Maintainers cutting
releases**: `bump-version.sh` no longer modifies `plugin.json` or
`marketplace.json`. Stage only `CHANGELOG.md` for the chore commit.
The Release workflow's bot commit handles the JSON bumps.

## 1.0.0-beta.3 — hook tool_response shape fix (2026-04-25)

Same-day patch on top of beta.2. With the dispatcher's
`hook_event_name` alias in place, the harness was finally invoking
hooks — but three PostToolUse:Bash hooks were silently no-op'ing
because their guards expected an `exit_code` field that Claude Code
doesn't send.

### Fixed

- **`tool_response` shape mismatch in three bash hooks.** Claude Code's
  harness sends `tool_response` for Bash with `interrupted`, `stdout`,
  `stderr`, `isImage`, `noOutputExpected` — NOT the `exit_code` field
  the documented schema implied. The affected hooks bailed when
  `exit_code` defaulted to `-1`, so even after the dispatcher started
  parsing envelopes correctly (beta.2), the events never landed.
  - `capture-commit-activity`, `capture-pr-activity`: prefer
    `exit_code` when the host sends it (back-compat for any future
    schema change), fall back to `interrupted` for Claude Code's
    actual shape.
  - `regression-gate`: same fallback chain; `interrupted=true` →
    fail, `interrupted=false` → pass when `exit_code` is absent.
- **`capture-commit-activity` first-line-only stdout parser.**
  Compound bash commands (e.g. `echo BEFORE; git commit ...`) put
  earlier output before the git-commit preamble. The new parser
  scans every stdout line for the `[<branch> <sha>] <message>`
  pattern and validates the bracket's last token as a 7–40 char hex
  SHA before treating the line as a commit.

### Verified end-to-end

Real harness `git commit` invocations produce `commit.made` events
in `factory-events-*.jsonl`. Confirmed against four prism repo
commits (4fd662ab, 400fedb5, 7617214d, 3fe36e4b) — first time
`commit.made` has fired through the v1.0 pipeline against the real
Claude Code harness.

### Migration

No breaking changes from beta.2. Operators on beta.1 or beta.2
should `/plugin update vsdd-factory@vsdd-factory:1.0.0-beta.3` and
re-run `/vsdd-factory:activate`. If Claude Code's plugin cache
serves stale beta.2 binaries (a known limitation around the
"chore + bot retag" window), wipe
`~/.claude/plugins/cache/vsdd-factory/vsdd-factory/1.0.0-beta.2/`
manually and restart the session.

## 1.0.0-beta.2 — harness payload schema fix (2026-04-25)

Same-day patch to v1.0.0-beta.1 that closes the gate-4 dogfood
criterion. Beta.1 architecturally worked but couldn't talk to the
real Claude Code harness because of a field-name mismatch in the
input envelope schema.

### Fixed

- **Dispatcher rejected real Claude Code harness envelopes.** The
  dispatcher's `HookPayload` struct deserialized `event_name`, but
  Claude Code's documented hooks payload uses `hook_event_name`.
  Every real harness invocation failed at parse time with "missing
  field `event_name` at line 1 column 320" → zero plugins ran →
  zero events emitted. Synthetic tests passed because we authored
  them with `event_name` (the SDK's canonical name) — they didn't
  catch the harness/dispatcher schema mismatch. Fixed via
  `#[serde(alias = "hook_event_name")]`. Both spellings now parse;
  canonical name stays `event_name` so the SDK's payload shape is
  unchanged.

### Added

- Regression test
  `payload::tests::accepts_hook_event_name_alias_from_real_harness`
  pinning the alias against a real-shape envelope. Smoke verified
  end-to-end: feeding the harness-shape payload to the dispatcher
  binary produces a clean PostToolUse/Bash dispatch into 3 plugins
  with `exit_code:0`.

### Migration

No breaking changes from beta.1. Operators on beta.1 should
`/plugin update vsdd-factory@vsdd-factory:1.0.0-beta.2` and re-run
`/vsdd-factory:activate` if their `hooks.json` is platform-stale.

## 1.0.0-beta.1 — Factory Plugin Kit beta (2026-04-25)

First v1.0 pre-release. v0.79.x's bash-hook dispatch model ran into an
unfixable upstream bug: Claude Code's `PostToolUse Bash` matcher
silently de-duplicated identical hook entries across events, so
`commit.made` emissions stopped reaching the observability pipeline.
The fix path was a full rewrite of the hook layer.

This beta replaces the v0.79.x bash-dispatch shape with a Rust
dispatcher (`factory-dispatcher`) that loads WASM hook plugins and
routes Claude Code events through them, plus a `legacy-bash-adapter`
WASM plugin that runs unported v0.79.x bash hooks unchanged on
Linux/macOS so the full hook set keeps working through the migration.

### Architecture

- New Rust workspace (`Cargo.toml`) with 9 member crates:
  - `factory-dispatcher` — the binary Claude Code now invokes per
    hook event. Loads `hooks-registry.toml`, matches plugins by
    event/tool, runs them in priority tiers (parallel within a tier
    via tokio + spawn_blocking around wasmtime), enforces wasmtime
    epoch + fuel budgets per plugin.
  - `vsdd-hook-sdk` + `vsdd-hook-sdk-macros` — the `#[hook]` proc-
    macro and host-fn wrappers external authors use to write WASM
    hooks. Targets `wasm32-wasip1`. SDK is publish-ready at 0.1.0;
    real `cargo publish` to crates.io ships separately.
  - `legacy-bash-adapter` — WASM plugin that bridges to existing
    bash hooks via `exec_subprocess("bash", [<script_path>], ...)`
    with the Q4 safety envelope (`shell_bypass_acknowledged`).
  - `capture-commit-activity` — workspace member crate; native
    WASM port lands in S-3.1 (Tier E).
  - `sink-core` + `sink-file` + `sink-otel-grpc` — observability
    sinks. File sink mirrors v0.79.x JSONL behavior so existing
    Grafana dashboards keep rendering with no changes.
- 5-platform cross-build matrix: darwin-arm64, darwin-x64
  (macos-15-intel after macOS-13 retirement), linux-x64, linux-arm64
  (cross-rs/cross), windows-x64. Pinned in `ci/platforms.yaml` with a
  drift gate (`scripts/check-platforms-drift.py`).
- HOST_ABI v1 frozen — see `crates/hook-sdk/HOST_ABI.md`.

### Migration

The migration is opt-in: marketplace `/plugin install
vsdd-factory@vsdd-factory` continues to default to `0.79.4`.
Operators upgrading to `1.0.0-beta.1` follow
`docs/guide/migrating-from-0.79.md`.

After install, run the new `/vsdd-factory:activate` skill — it
detects platform, copies `hooks.json.<platform>` over `hooks.json`,
and verifies the dispatcher binary is present + executable.

### Fixed

- The `PostToolUse Bash` matcher de-dup bug in Claude Code that
  motivated the rewrite (v0.79.4 was a partial workaround; this beta
  is the real fix because dispatch happens out-of-process now).
- `factory-dispatcher`'s `exec_subprocess` host fn now writes the
  result envelope into a guest-pre-allocated buffer instead of wasm
  memory offset 0 (which the SDK previously short-circuited to an
  empty Vec, making the legacy adapter pipeline unusable). Caught
  during S-2.7 regression validation.
- Three latent wiring gaps: `HostContext.plugin_root` was never
  populated from `${CLAUDE_PLUGIN_ROOT}`; `HostContext.env_view` was
  empty so `env_allow` produced empty subprocess env; bash subprocess
  cwd defaulted to the dispatcher's cwd instead of
  `${CLAUDE_PROJECT_DIR}`. All three needed for round-trip parity
  with v0.79.x bash hook behavior.

### Added

- `plugins/vsdd-factory/hooks-registry.toml` — auto-generated by
  `scripts/generate-registry-from-hooks-json.sh` from the historical
  v0.79.x hooks.json (45 entries, all routed through
  `legacy-bash-adapter.wasm`). Idempotent regeneration.
- `plugins/vsdd-factory/tests/regression-v1.0.bats` — 11 regression
  guards for the dispatcher → adapter → bash pipeline. Pins the
  post-fix end-state so future refactors can't silently regress.
- `crates/factory-dispatcher/tests/loads_legacy_registry.rs` — Rust
  integration test that the generated registry parses through
  `Registry::load`.
- `benches/legacy-adapter-latency.sh` — direct-vs-adapter latency
  benchmark scaffolding (median of 10 per hook). Adapter overhead
  budget validation lands when we re-baseline post-beta.
- `docs/guide/migrating-from-0.79.md` — operator-facing migration
  guide (skeleton flushes out in S-5.5; v1.0-beta.1 ships the
  Resolved-during-S-2.7 history + platform validation table).
- Platform-specific `hooks.json.{darwin-arm64,darwin-x64,linux-x64,
  linux-arm64,windows-x64}` variants (CI-generated from
  `hooks.json.template`); the activate skill picks the right one per
  install.

### Tests

87 dispatcher lib + 14 adapter + 18 host integration + 20 sdk + 17
sink + 13 macros + 11 regression-v1.0 bats + 6 generate-registry bats
+ 1245 baseline bats — all green at this tag. Adapter wasm builds
cleanly on every CI platform. SDK publish dry-run clean.

### Known limitations (non-blocking for beta)

- Plugin stderr is allocated as a `MemoryOutputPipe` but not
  forwarded to the dispatcher-internal log. Plugin failures show up
  as `exit_code:1` on `plugin.completed` without the stderr text.
  Diagnostic-only; tracked for post-beta.
- Windows: `legacy-bash-adapter` requires git-bash. Native WASM hooks
  (capture-commit-activity, capture-pr-activity, block-ai-attribution
  in Phase 3) will work without it.
- Real `cargo publish` of `vsdd-hook-sdk` to crates.io happens at the
  release workflow's discretion; the workspace dry-run gate in CI is
  the standing guard.

## 0.79.4 — drop matcher field entirely on PostToolUse Bash hooks (Claude Code harness workaround)

v0.79.3 replaced `"matcher": "Bash"` with `"matcher": "*"` and still
saw zero `commit.made` emissions in prism post-restart. The `"*"`
literal apparently isn't treated as match-any by Claude Code's
resolver — it's either rejected silently or hits the same dedup bug.

Reference repo [shanraisshan/claude-code-hooks](https://github.com/shanraisshan/claude-code-hooks)
uses **no `matcher` field at all** on any of its 27 hook event
entries (`PreToolUse`, `PostToolUse`, `SubagentStop`, etc.). Their
dispatcher self-dispatches based on `hook_event_name` and `tool_name`
from the stdin JSON payload. Verified in production.

### Changed

- **`plugins/vsdd-factory/hooks/hooks.json`** — remove the `matcher`
  field from the `PostToolUse` Bash-intent entry. The three hooks
  (`regression-gate`, `capture-pr-activity`, `capture-commit-activity`)
  already self-filter on `tool_name` internally, so matching every
  tool invocation (which is what absent-matcher means per the
  reference implementation) and early-exiting for non-Bash is a
  no-op for behavior.
- **`plugins/vsdd-factory/tests/capture-commit-activity.bats`**,
  **`plugins/vsdd-factory/tests/factory-obs.bats`** — assertions
  relaxed to "hook is registered under PostToolUse," agnostic to
  the matcher field (already made this change in v0.79.3 for
  `select(.matcher == "Bash")`; this confirms they remain correct
  when the field is absent entirely).

### Rationale

This is the last configuration-only workaround we can try. If this
doesn't restore dispatch, the bug is upstream harness state that no
plugin-side change can fix. Any further attempts would require
rearchitecting the hook system — which is scoped for **v1.0.0**
(Rust dispatcher, cross-platform support, per-hook toggles, full
Claude Code event coverage). Design doc to follow.

### Migration

No migration needed. Install and **fully restart the Claude Code
session** (hot-upgrades don't reliably rewire hooks).

## 0.79.3 — replace PostToolUse "Bash" matcher with "*" (Claude Code harness workaround)

v0.79.2 reordered the `PostToolUse` array to test whether the bug was
positional — it wasn't. With `Bash` as the *first* entry and
`Edit|Write` as the second, `PostToolUse:Bash` was still silent while
`PostToolUse:Edit|Write` fired normally. Post-restart test in prism
produced zero `commit.made` events despite the reorder.

Refined bug theory: the Claude Code harness can't wire both
`(PreToolUse, "Bash")` AND `(PostToolUse, "Bash")` in the same plugin
— one of them is always dropped. Matches upstream
[claude-code #52715](https://github.com/anthropics/claude-code/issues/52715)
where the opposite direction is silent
(`PreToolUse:Bash` silent, `PostToolUse:Bash` fires). Looks like
`matcher: "Bash"` gets deduplicated across event types.

### Changed

- **`plugins/vsdd-factory/hooks/hooks.json`** — change the
  `PostToolUse` Bash-intent matcher from `"Bash"` to `"*"`. The
  three hooks under that entry (`regression-gate`,
  `capture-pr-activity`, `capture-commit-activity`) already
  self-filter on `tool_name` (see `capture-commit-activity.sh:40-43`,
  `capture-pr-activity.sh:33-35`, `regression-gate.sh:21-22`), so
  matching every tool and early-exiting for non-Bash is a behavioral
  no-op — just trades ~6ms per non-Bash tool call for the chance to
  escape the harness dedup bug. `PreToolUse:Bash` (block-ai-attribution,
  destructive-command-guard, protect-secrets, verify-git-push,
  check-factory-commit) stays as-is since it fires fine.

### Migration

No migration needed. Install and **fully restart the Claude Code
session** (hot-upgrades don't reliably rewire hooks). If `commit.made`
events appear in `.factory/logs/events-*.jsonl` after the restart,
the dedup theory is confirmed and the Factory ROI dashboard becomes
honest.

## 0.79.2 — reorder PostToolUse matchers (Claude Code harness workaround)

Speculative fix for an observed Claude Code harness bug where the
`PostToolUse` hooks registered under `matcher: "Bash"` were silently
never dispatched in live sessions — zero emissions across two full
days despite dozens of real `git commit` and `gh pr` invocations.
Manual invocation of the hook scripts (piping synthetic payloads into
them) produced correct output, and `emit-event` wrote events
correctly. The silence was specific to the `(PostToolUse, Bash)` pair
while `(PreToolUse, Bash)` and `(PostToolUse, Edit|Write)` both fired
normally from the same `hooks.json`.

The only meaningful structural difference was position in the
`PostToolUse` array — `Edit|Write` was the first entry, `Bash` the
second. Upstream issue
[claude-code #52715](https://github.com/anthropics/claude-code/issues/52715)
reports the mirror-image symptom (`PreToolUse` + `Bash` silent while
`PostToolUse` + `Bash` fires), suggesting a harness-level registration
bug that drops one `(EventType, "Bash")` pair when multiple matcher
entries coexist.

### Changed

- **`plugins/vsdd-factory/hooks/hooks.json`** — reordered
  `PostToolUse` matcher entries so `Bash` is first and `Edit|Write`
  second. Hook behavior is unchanged; only the array order differs.
  If this does not restore `capture-commit-activity` /
  `capture-pr-activity` / `regression-gate` emissions, the fallback
  plan is to collapse to a single `"*"` matcher entry with internal
  per-script tool filtering (all three Bash hooks already filter on
  `tool_name`, so this is a no-op for them).

### Fixed

- **`plugins/vsdd-factory/tests/input-hash.bats`**,
  **`plugins/vsdd-factory/tests/template-compliance.bats`** —
  replaced `.hooks.PostToolUse[0]` index lookups with
  `.hooks.PostToolUse[]` (iterate all entries) so assertions survive
  future matcher-array ordering changes.

### Migration

No migration needed. Install the new version and **restart the
Claude Code session** — plugin hot-upgrades mid-session do not
reliably rewire hooks (that's how we got here).

## 0.79.1 — bump-version.sh idempotent CHANGELOG guard

Tiny tooling fix. Three releases in a row (v0.76.1 / v0.78.1 /
v0.79.0) shipped with a stub "TODO: fill in release title" heading
in their GitHub release notes. Root cause: `scripts/bump-version.sh`
unconditionally prepended a stub heading to `CHANGELOG.md`, which
raced with my workflow of writing the real entry beforehand (the
stub edit collided with my in-progress Edit of the pre-written
entry, and the stub won).

### Fixed

- **`scripts/bump-version.sh`** — added a guard that checks for an
  existing `## <new-version>` heading before prepending. If the
  user (or tooling) already wrote the entry, the script skips the
  stub injection, preserves the real content, and prints a status
  line saying the entry was already present. No more racy stub
  overwrites.

- Also fixed a pre-existing shellcheck SC2059 in the `printf`
  format-string usage so the script is clean under `shellcheck`
  (no behavior change, just hygiene).

### Migration

No behavior change for the preferred flow (write CHANGELOG first,
then run bump-version.sh). The legacy "bump first, edit after" flow
still works but prints a TODO stub as before — users just need to
remember to fill it in before committing.

## 0.79.0 — Observability onboarding: model-invocable skills + one-step setup

Before this release, "register this project with the observability
stack" was not discoverable to an orchestrator agent — both
`factory-obs` and `claude-telemetry` skills had
`disable-model-invocation: true` in their frontmatter, so the model
couldn't auto-match the user's request to the skill, and a user had
to explicitly type `/vsdd-factory:factory-obs register` and
`/vsdd-factory:claude-telemetry on`. This release makes the
onboarding flow discoverable and automatable.

### Added

- **New `/vsdd-factory:onboard-observability` skill + command alias**
  (`plugins/vsdd-factory/skills/onboard-observability/SKILL.md`,
  `plugins/vsdd-factory/commands/onboard-observability.md`).
  One-command first-time setup that:
  1. Runs `factory-obs register` on the current project (adds its
     `.factory/logs/` to the collector's watch list).
  2. Writes the 5 `OTEL_*` env vars to `.claude/settings.local.json`
     (same operation as `claude-telemetry on`, inlined so the skill
     is self-contained).
  3. Prints a concise summary + next-step reminder (restart Claude
     to pick up OTel env vars; run `factory-obs up` if the stack
     isn't already running).

  Idempotent — safe to re-run. Model-invocable — the skill's
  description matches phrases like "register this project with
  observability", "set up observability here", "onboard the
  observability stack".

- **Getting-started guide** (`docs/guide/getting-started.md`) gained
  a "5. (Optional) Wire the project into the observability stack"
  step in the first-time setup section, documenting the new skill
  and the underlying two-step workflow.

- **Commands reference** (`docs/guide/commands-reference.md`) gained
  an "Infrastructure" row for each of the three observability
  commands (`factory-obs`, `claude-telemetry`,
  `onboard-observability`) so they're discoverable in the
  documented command catalog.

### Changed

- **`factory-obs` skill frontmatter** — removed
  `disable-model-invocation: true`. The model can now auto-invoke
  `factory-obs` when the user says things like "start the
  observability stack", "open the Grafana dashboards", "register
  this factory", etc. Description copy tightened to cover the new
  invocation cases.

- **`claude-telemetry` skill frontmatter** — removed
  `disable-model-invocation: true`. Auto-invocation matches on
  phrases like "enable Claude telemetry", "turn on Claude OTel
  export", "why aren't Claude's metrics showing up". Description
  copy tightened.

### Why this matters for orchestration

An orchestrator agent spun up in a fresh project can now handle the
observability onboarding request end-to-end with a single user
phrase. Before: the user had to know the exact slash-command syntax
and run two separate commands. After: "register this project with
the observability stack" or "set up observability here" both match
`onboard-observability` directly, and the agent executes both steps.

### Safety

Flipping `disable-model-invocation` slightly increases noise —
mentions of "observability" or "telemetry" in an unrelated context
could now match these skills. Mitigated by the descriptions being
specific about the use cases ("Use when the user asks to …"). None
of these skills take destructive actions without explicit user
intent, and both existing sub-skills already ran without invocation
gating before — flipping the frontmatter only changes *discovery*.

### Tests

- New `skills.bats` assertions: `factory-obs`, `claude-telemetry`,
  and `onboard-observability` must NOT have
  `disable-model-invocation: true` (regression guard). The
  `onboard-observability` skill must describe both halves of the
  workflow and state idempotency.
- Full suite: 1177 tests, all passing.

## 0.78.2 — Docs refresh for the observability stack (v0.66 → v0.78)

Documentation-only release. No runtime code or config changes — safe
to skip from a deployment standpoint. Every piece of observability
documentation was stale by 10+ releases:

- The top-level `README.md` and `docs/guide/observability.md`
  described the original "3-container" stack from v0.66.0 (OTel +
  Loki + Grafana) even though v0.78.1 ships 5 services + an init
  container.
- `plugins/vsdd-factory/skills/factory-obs/SKILL.md` listed only
  the original 6 subcommands; missed the 4 new ones from v0.78.0
  (`register`, `unregister`, `list`, `regenerate`).
- `plugins/vsdd-factory/tools/observability/README.md`'s
  architecture diagram still showed the v0.66 topology without
  Prometheus, the renderer sidecar, or the OTLP metrics path.
- `docs/guide/hooks-reference.md` was missing `capture-pr-activity.sh`
  (v0.73.1) and `capture-commit-activity.sh` (v0.77.0) entirely.
- The observability Roadmap table stopped at v0.68.0.

### Changed

- **`docs/guide/observability.md`** — opening paragraph now reflects
  v0.78.1 state (5-service stack, 7 dashboards, multi-factory,
  Prometheus). The "factory-obs + Docker stack" section expanded
  with the full service list, lifecycle, multi-factory workflow,
  dashboard catalog, and the `capture-pr-activity` / 
  `capture-commit-activity` companion-signal hooks. Roadmap table
  updated with phases 7–13 through v0.78.0 + planned items.

- **`plugins/vsdd-factory/tools/observability/README.md`** —
  architecture ASCII diagram redrawn to show OTel collector →
  Loki + Prometheus split and the Grafana + renderer sidecar
  topology. "What ships" list now covers all 5 services, 7
  dashboards, and the companion `loki-config.yaml` +
  `prometheus-config.yaml` artifacts.

- **`plugins/vsdd-factory/skills/factory-obs/SKILL.md`** —
  description frontmatter updated to name all 5 services and
  mention multi-factory. Subcommand table now includes `register`,
  `unregister`, `list`, and `regenerate`. Env-var list expanded
  with `VSDD_OBS_PROMETHEUS_PORT`, `VSDD_OBS_RENDERER_PORT`, and
  `VSDD_OBS_REGISTRY`. New "When to use" entries for the registry
  commands. Non-goals clarified to explain that registration is
  explicit by design.

- **`docs/guide/hooks-reference.md`** — added rows to the Hook
  Summary table and new Hook Details sections for
  `capture-pr-activity.sh` (PR event capture with open→merge
  duration pairing) and `capture-commit-activity.sh` (commit
  event capture backing the rebuilt Cost per commit panel).

- **`README.md`** — Observability row in the docs index now
  mentions the local stack and multi-factory registry, not just
  the event-log schema.

### Migration

No breaking changes. No runtime behavior change.

## 0.78.1 — Shellcheck hint fix on v0.78.0 factory-obs

Tiny fix release — v0.78.0's Release workflow failed its shellcheck
contract test because the `_find_factory_root` helper takes an
optional `$1` that's never passed explicitly (all call sites rely on
the `$PWD` default). Shellcheck's SC2120/SC2119 info-level checks
flagged it.

### Fixed

- `bin/factory-obs` — added `# shellcheck disable=SC2120` on
  `_find_factory_root` with an inline note explaining why the
  optional-arg pattern is intentional. No behavior change.

### Migration

No breaking changes. Identical runtime behavior to v0.78.0.

## 0.78.0 — Multi-factory watch: register/list/unregister + dynamic compose override

The observability stack can now aggregate events from any number of
factory projects, wherever they live on the filesystem. This fixes a
blind spot surfaced during the 2026-04-23 live-stack audit: the stack
was only watching its own `.factory/logs/` because the bind mount was
hard-coded relative to the plugin's install location. Projects
installed anywhere else (e.g., `~/Dev/prism`, `~/work/api`, `/opt/team/…`)
had their hook events silently dropped even while their Claude OTel
telemetry flowed normally.

### Fixed

- **Silent data loss on additional factories**: any factory project
  outside the plugin install tree was previously invisible to Loki
  because the otel-collector's bind mount was anchored to the plugin's
  own `.factory/logs/`. Now every registered factory is mounted
  explicitly at `/var/log/factory/<safe-name>/`, and the collector's
  filelog receiver globs `/var/log/factory/*/events-*.jsonl` to pick
  them all up.

### Added

- **`bin/factory-obs register [PATH]`** — add a factory to the watched
  list. With no argument, autoresolves the nearest ancestor directory
  containing `.factory/` from `cwd`. Refuses relative paths and
  non-factory directories. Dedups if the absolute path is already
  registered.

- **`bin/factory-obs unregister [PATH]`** — remove a factory from the
  watched list. Same `cwd` autoresolution as register. No-op (not an
  error) when the path isn't registered.

- **`bin/factory-obs list`** — print all registered factories with
  their safe-names and filesystem status. Flags paths whose
  `.factory/` subdirectory has gone missing so users can clean up
  stale entries.

- **`bin/factory-obs regenerate`** — rewrite
  `tools/observability/docker-compose.override.yml` from the current
  registry without starting or restarting the stack. Primarily for
  tests and scripting; `up` always calls it implicitly.

- **Registry file** at `${XDG_CONFIG_HOME:-~/.config}/vsdd-factory/watched-factories`.
  One absolute path per line, `#` comments allowed. Managed via the
  CLI; users generally shouldn't edit by hand.
  Override via `VSDD_OBS_REGISTRY=<path>` (primarily for tests).

### Changed

- **`tools/observability/docker-compose.yml`** — the hard-coded
  `${VSDD_FACTORY_LOGS:-…}:/var/log/factory:ro` bind mount is removed
  from the base compose. Per-factory mounts are now injected by the
  generated override file. The base compose is no longer usable on
  its own — always use `factory-obs up` (or
  `docker compose -f base.yml -f override.yml …` with the override
  already generated).

- **`tools/observability/otel-collector-config.yaml`** — filelog
  `include` glob changed from `/var/log/factory/events-*.jsonl` to
  `/var/log/factory/*/events-*.jsonl` to match the per-factory
  subdirectory layout.

- **`tools/observability/README.md`** — new "Watching multiple
  factories" section documenting the registry workflow.

### Migration

No action required for single-factory users: running `factory-obs up`
from inside a factory project still works exactly as before (empty
registry → autoresolve `cwd`). The `VSDD_FACTORY_LOGS` single-path
fallback is preserved.

For multi-factory users, the recommended setup is:

```bash
cd ~/Dev/project-a && factory-obs register
cd ~/Dev/project-b && factory-obs register
factory-obs list         # verify
factory-obs up           # apply
```

### Notes

- The generated `docker-compose.override.yml` is `.gitignore`-d so
  each dev machine maintains its own multi-factory layout without
  leaking into commits.
- Safe-name format is `<basename>-<8-char-sha>`. Two projects sharing
  a basename (e.g., `api`) get distinct mounts because the sha
  derives from the full absolute path.

### Tests

- New `tests/factory-obs-registry.bats` — 19 tests covering register
  (success, cwd autoresolve, dedup, invalid inputs), unregister,
  list, regenerate (mount shapes, fallbacks, safe-name disambiguation).
- Updated `tests/factory-obs.bats` — asserts base compose no longer
  hard-codes a `.factory/logs` volume, and that the collector's
  filelog glob uses the multi-factory subdirectory pattern.
- Full suite: **1193 tests**, all passing.

## 0.77.1 — Factory ROI: Cost per active minute + Cost per active second

Additive re-units for the Cost per active hour panel shipped in v0.77.0.
Same underlying Claude SDK signal (`claude_code_active_time_seconds_total`),
different time scales so the displayed value reads naturally for typical
Claude Code usage patterns.

### Added

- **`grafana-dashboards/factory-roi.json` — Cost per active minute panel
  (id=13)** — `cost / (active_time / 60)`. Sits alongside Cost per active
  hour at y=8. Typical reading: tens of dollars per sustained-compute
  minute with Opus 4.7.

- **`grafana-dashboards/factory-roi.json` — Cost per active second panel
  (id=14)** — `cost / active_time`. The rawest reading of the same
  signal. Typical: ~$0.50–$2.00 per second of sustained compute. Useful
  as a sanity-check — this is roughly what the LLM is charging you in
  the moment.

All three panels (hour, minute, second) render identical underlying
math; only the display unit differs. Descriptions on each clarify that
Claude's `active_time` measures **sustained compute windows**, not
wall-clock engagement, so these are burst-rate signals rather than
actual per-hour / per-minute / per-second spend.

### Migration

No breaking changes. Layout unchanged — the two new panels fill empty
grid slots at x=8, y=8 and x=16, y=8 on the existing row.

Total bats tests: 1154 (up from 1153 at v0.77.0 — one new test covers
both re-unit panels).

## 0.77.0 — Fix Cost per commit (real signal) + add Cost per active hour

Replaces a ghost panel that had been showing N/A since v0.74.0, and adds
one complementary Prom-native ROI panel.

### The bug we uncovered

The Factory ROI dashboard's "Cost per commit" and "Commits" stat panels
both depended on `claude_code_commit_count_total` — a Prometheus counter
that's listed in Claude Code's OTel metric reference but **never
actually emitted by the SDK in our sessions**. Across the full 30-day
Prometheus retention, the series was empty. The panels silently read
"N/A" and "0" from v0.74.0 through v0.76.1 even while sessions were
actively committing. Our test only verified the panels *existed*, not
that their metrics resolved.

### Fixed

- **`hooks/capture-commit-activity.sh`** — new `PostToolUse` Bash hook,
  modeled on `capture-pr-activity.sh`. Matches `git commit` invocations
  (with word-boundary anchoring so `git commit-tree`, echoed text, and
  aliases don't spuriously trigger) and emits a structured
  `commit.made` event on success:
  - `commit_sha` — parsed from the `[<branch> <sha>]` preamble git
    prints on successful commits.
  - `branch` — first token of the same preamble; supports `feature/x`,
    `HEAD detached at <prev> <new>`, and `(root-commit)` shapes.
  - `message_subject` — first-line text after the preamble.
  - `amended="true"` — flagged when `--amend` was in the command, so
    downstream panels can exclude amends if desired.

  Failed commits are no-ops (pre-commit hook rejections shouldn't
  count against cost-per-commit). `git commit --dry-run` is a no-op.
  Absent/malformed `[<branch> <sha>]` preambles are no-ops. 21 bats
  tests cover structural, positive, filter-tightness, malformed-stdout,
  and graceful-degradation paths.

- **`grafana-dashboards/factory-roi.json` — Commits panel (id=3)** —
  datasource switched from Prometheus to Loki, query switched to
  `sum(count_over_time({service_name="vsdd-factory",event_type="commit.made"}[$__range]))`.
  Now reports the count that actually happened in the range.

- **`grafana-dashboards/factory-roi.json` — Cost per commit panel
  (id=6)** — rebuilt using the same cross-datasource pattern we
  proved out in v0.76.0:
  - refA = `sum(increase(claude_code_cost_usage_USD_total[$__range]))`
    (Prometheus instant).
  - refB = `sum(count_over_time({service_name="vsdd-factory",event_type="commit.made"}[$__range]))`
    (Loki instant).
  - refC = `__expr__` math with `expression: "$A / $B"`.
  - `filterByRefId` transform keeps only C for display.

  Verified end-to-end via the renderer sidecar: rendered **$0.3263**
  over a 24h window against live data. First time this panel has
  computed a real number since v0.74.0 shipped.

### Added

- **`grafana-dashboards/factory-roi.json` — Cost per active hour
  panel (id=12)** — new Prom-native stat, independent of git
  workflow:
  `cost / (active_time / 3600)`. Uses
  `claude_code_active_time_seconds_total`, which Claude's SDK DOES
  emit reliably. The panel description flags an important subtlety:
  `active_time` measures **sustained compute time** (streaming-response
  and tool-execution windows), not wall-clock session engagement. A
  typical session accumulates seconds-to-minutes of "active time", so
  the headline number represents the **sustained-compute hourly rate**,
  not an actual hourly spend. Useful as a stable rate signal,
  complementing cost-per-output-unit (PR / commit / story) panels.

### Notes

- Layout shift on Factory ROI: the new Cost-per-active-hour row pushes
  the `Cost over time` / `Output over time` timeseries from y=8 to
  y=12, and the `Cost by query source` / `Subagent dispatch efficiency`
  row from y=16 to y=20. No panels deleted.

- Total bats tests: **1153** (up from 1129 at v0.76.1 — 21 new for
  the commit hook, 3 new for the ROI panel shapes, 1 adjusted for the
  rebuilt Cost per commit).

## 0.76.1 — CI test-infra fix for Release workflow

Tiny release that unblocks the `Release` GitHub Actions workflow. No
runtime code changes; safe to consume alongside v0.76.0.

### Fixed

- `tests/emit-event.bats` — the worktree-resolution test at
  `@test "emit-event: resolves to main worktree's .factory/logs…"`
  was making a `git commit --allow-empty` inside a fresh `mktemp -d`
  repo, which fails on GitHub Actions runners because they don't
  have a global git identity. Set `GIT_AUTHOR_*` / `GIT_COMMITTER_*`
  env vars at test scope so the commit succeeds without touching
  the runner's git config. This had been silently failing the
  `Release` workflow since v0.75.0 — both v0.75.0 and v0.76.0 have
  valid tags but no GitHub Release artifact as a result. v0.76.1
  is the first release that should produce a GitHub Release
  automatically again.

### Migration

No breaking changes. If you're on v0.76.0, the only difference at
v0.76.1 is the test file. No dashboard, skill, hook, or collector
config changes.

## 0.76.0 — Observability stack upgrade: Grafana v13 + OTel 0.149 + Loki 3.6 + Prom v3

Coordinated major-version bump of the entire local observability stack.
The headline outcome: **Factory ROI's `Cost per PR merged` and `Cost per
story touched` panels now compute real values end-to-end** — the first
time cross-datasource Prometheus ÷ Loki math has worked since we tried
it in v0.74.0. Along the way: simplified Claude telemetry UX, closed
two backlog items, and fixed several breaking changes that the stack
upgrade surfaced.

### Version pins (docker-compose.yml)

- **Grafana** `10.4.2` → `13.0.1` — unlocks server-side expression
  queries (`datasource: { type: "__expr__" }`), which is the canonical
  v11+ pattern for cross-datasource math. This is what finally makes
  cost-per-X panels computable.
- **OTel Collector Contrib** `0.94.0` → `0.149.0` — unlocks the
  `deltatocumulative` processor (added in 0.115). With it wired into
  the metrics pipeline, Claude's DELTA-temporality counters are
  converted in-flight before Prometheus's `remote_write` receiver
  sees them, so `OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE`
  is no longer required at the Claude side.
- **Loki** `3.0.0` → `3.6.10` — the image is now full distroless (no
  shell, no `wget`, no probes). We replaced the Docker `HEALTHCHECK`
  with a one-shot `loki-ready` init container that polls `/ready`
  from a busybox image on the shared network; downstream services
  (otel-collector, grafana) wait on `loki-ready` via
  `condition: service_completed_successfully`.
- **Prometheus** `v2.54.0` → `v3.5.2` — `remote_write` receive +
  PromQL query APIs we use are stable across v2→v3.

### Added

- **`tools/observability/loki-config.yaml`** — previously used Loki's
  built-in defaults. Now we ship an explicit config that:
  - Raises `reject_old_samples_max_age` to 30 days so the collector
    can backfill historical `.factory/logs/events-*.jsonl` entries
    across a stack restart without Loki 400-ing whole batches. This
    was a hidden data-loss path under the old default (~2h window).
  - Declares the Loki-side `otlp_config.resource_attributes`
    promotion list (`service.name`, `event_type`, `hook`, `reason`,
    `severity`). Previously we relied on the collector's standalone
    `loki` exporter honoring a `loki.resource.labels` hint attribute.
    That exporter was removed in collector-contrib ~0.112, so we
    ship the labels via Loki's native OTLP ingester and let Loki
    own the label decision.

- **`tools/observability/docker-compose.yml` `renderer` service** —
  Grafana Image Renderer 3.11.0 sidecar is now part of the default
  stack. Enables `/render/d-solo/...` for programmatic PNG/PDF export
  of any panel. Shared auth token with Grafana
  (`GF_RENDERING_RENDERER_TOKEN` ↔ `AUTH_TOKEN`) since Grafana v11+
  refuses the library default. Closes backlog task #104.

- **`tools/observability/docker-compose.yml` `loki-ready` service** —
  tiny busybox init container that polls Loki's HTTP `/ready`
  endpoint and exits 0. Downstream services gate on it via
  `service_completed_successfully`. Replaces the broken
  `wget`-based `HEALTHCHECK` that Loki 3.6's distroless image can
  no longer run.

### Fixed

- **`otel-collector-config.yaml` metrics pipeline** — wired the
  `deltatocumulative` processor (ordered before
  `prometheusremotewrite`). Claude's metrics (cost, token counts,
  session durations) now land in Prometheus without the temporality
  env-var workaround.

- **`otel-collector-config.yaml` logs exporter** — switched from the
  removed `loki:` exporter to `otlphttp/loki:` targeting
  `http://loki:3100/otlp`. Loki 3.x's native OTLP ingester appends
  `/v1/logs` automatically.

- **`otel-collector-config.yaml` move operators** — added
  `if: 'attributes.type != nil'` guard on the `move` that promotes
  the event `type` to `resource.event_type`. collector-contrib 0.149
  is strictly-failing where 0.94 was lenient: a single malformed
  event with no `type` field (e.g. one written by an
  incorrectly-invoked emit-event) would otherwise halt the entire
  filelog tailer for that cycle. Matches the existing guards on
  `hook`, `reason`, `severity`.

- **`grafana-dashboards/factory-roi.json`** — the two markdown
  fallback panels are **back to real stat panels**, each doing a
  live Prometheus-cost / Loki-count division via a server-side
  expression query. Structure:
  - refA: Prometheus instant query (`sum(increase(...[$__range]))`)
  - refB: Loki instant query (`sum(count_over_time(...[$__range]))`)
  - refC: `__expr__` math query with `expression: "$A / $B"`
  - `filterByRefId` transform keeps only C for display

  This pattern works on every Grafana version from 9.x forward —
  the reason we weren't using it earlier is that nobody had reached
  for server-side expressions. Unblocks real cost-per-X visibility
  in the ROI dashboard without requiring task #105 (Prom-native
  counters) to land.

### Removed

- **`otel-collector-config.yaml` `attributes/loki-label-hint`
  processor** — dead code after the `loki` exporter's removal. Loki
  now owns label promotion via its own `otlp_config`. Pipeline is
  simpler: `[batch, resource]` instead of
  `[batch, resource, attributes/loki-label-hint]`.

### Changed

- **`skills/claude-telemetry/SKILL.md`** — the env var list is back
  down to 5 (from 6). `OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE`
  is obsolete now that `deltatocumulative` runs in the collector.
  The `on` path prunes the legacy key on re-run for users who
  configured it via an earlier release; `off` also cleans it up.
  `status` flags the legacy key as stale if present.

### Notes

- Historical events in `.factory/logs/events-*.jsonl` older than
  about 6 hours at the time of stack restart may not be re-indexed
  into Loki. The collector's file_storage remembers the last offset,
  and Loki's 30d acceptance window doesn't apply retroactively to
  events that were briefly rejected on a prior run. Fresh events
  flow normally.

- Total bats tests: **1129** (up from 1117 at v0.75.0 — 12 new
  tests cover the upgrade surface).

## 0.75.0 — Subagent dashboard + PR duration pairing + honest ROI fallback

Three backlog items closed in one release, plus a honest admission on a
known limitation.

### Added

- **`tools/observability/grafana-dashboards/factory-subagents.json`** —
  new "Factory Subagents — Usage & Exit Classes" dashboard. 8 panels:
  total dispatches, unique subagent types, ok/non-ok stop counts,
  top-15 invocation bargauge, exit-class distribution stacked bars,
  per-subagent activity timeline, and a recent-events log feed.
  Closes backlog task #91.

- **`hooks/capture-pr-activity.sh`** — PR duration pairing. On
  `gh pr merge` success, the hook now scans recent
  `.factory/logs/events-*.jsonl` files for a matching `pr.opened`
  event with the same `pr_number`, computes the elapsed time, and
  attaches `open_to_merge_seconds` to the emitted `pr.merged` event.
  Bounded to last 7 days of log files for runtime. Absurd durations
  (>30 days, negative) are rejected to protect against PR-number
  collisions across weeks. 3 new bats tests (18 total for this hook).

- **`tools/observability/grafana-dashboards/factory-prs.json`** —
  two new panels using the new duration data:
  - "Avg open→merge duration" (stat, seconds) — uses LogQL `unwrap
    attributes_open_to_merge_seconds` + sum/count division to compute
    mean.
  - "PRs with duration recorded" (stat) — how many `pr.merged`
    events actually carried a duration, so users can see data
    coverage without being misled by a tiny sample.

### Fixed / Changed

- **`tools/observability/grafana-dashboards/factory-roi.json`** —
  replaced the two broken cross-datasource derived panels
  ("Cost per PR merged" and "Cost per story touched") with
  markdown text panels that explain how to compute the ratio from
  the raw stats above. The underlying problem: Grafana v10.4.2's
  `calculateField` binary transform doesn't reliably resolve field
  references across mixed Prometheus+Loki frames. Multiple patterns
  were tried (joinByField, merge, seriesToColumns, legendFormat
  rename, Value #A naming) — none produced a reliable divide.
  "Cost per commit" (Prom-native, real division) is unchanged and
  remains the one working derived ratio.

### Known limitation & follow-up

- Cross-datasource cost-per-X panels deferred to task #105: emit
  Prometheus-native counters for factory events (PRs merged,
  stories touched). Once those metrics exist, divisions become
  Prom-native and the text panels can become real stat panels.

- Task #104: adding `grafana/grafana-image-renderer` to the
  observability stack. Makes panel verification programmatic for
  future debugging (this release's ROI-transform debugging would
  have been ~5× faster with it).

### Verified live

All 7 dashboards load in Grafana's "VSDD Factory" folder. The new
duration panel renders "N/A" until a PR open→merge cycle completes
with both events captured. All 82 bats tests pass (suite grew with
3 new PR-duration cases + 2 updated ROI-dashboard cases).

### Migration

`factory-obs down && factory-obs up` (or `docker restart
vsdd-obs-grafana`) to reload the new dashboards. Existing hook events
are untouched — the duration pairing only affects future `pr.merged`
emissions.

## 0.74.0 — Factory ROI dashboard (Phase C — cost/ROI series complete)

Completes the three-phase cost-and-ROI build-out. Phase A (v0.72.0)
laid the Prometheus foundation and Factory Today. Phase B (v0.73.0)
shipped the Claude Cost dashboard. Phase C (this release) answers the
third question in the original trio: **"what was our ROI?"**

### Added

- **`tools/observability/grafana-dashboards/factory-roi.json`** —
  new "Factory ROI — Cost vs Output" dashboard. 11 panels combining
  Prometheus (cost) with Loki (output event counts) via Grafana
  mixed-datasource transformations:

  Top row — raw totals (4 stats):
  - Total cost (USD, Prometheus)
  - PRs merged (Loki pr.merged count)
  - Commits (Prometheus claude_code_commit_count_total)
  - Stories touched (distinct attributes_story_id on agent.start)

  Derived ratios (3 stats, noValue=N/A):
  - **Cost per PR merged** — cross-datasource; `joinByField` +
    `calculateField` Grafana transforms divide Prom cost by Loki count.
  - **Cost per commit** — Prom-native division
    (claude_code_cost_usage_USD_total / claude_code_commit_count_total).
  - **Cost per story touched** — cross-datasource, Prom cost / Loki
    distinct-story count.

  Trends (2 timeseries):
  - Cost over time
  - Output over time (agent.start + pr.opened + pr.merged bars)
    — designed to read side-by-side with "Cost over time" so
    expensive-but-unproductive windows stand out visually.

  Breakdown (2):
  - Cost by query_source (pie: main / subagent / auxiliary) —
    where the spend actually goes.
  - Subagent dispatch efficiency (bargauge: top 10 subagents by
    invocation count).

### Cross-datasource mechanic

Grafana "Mixed" datasource with two targets:
- `refId: COST` (Prometheus)
- `refId: PRS` (Loki)

Transformations chain:
1. `joinByField { byField: "Time", mode: "outer" }` — aligns the
   two series on the time axis.
2. `calculateField { binary: { left: COST, operator: "/", right: PRS },
   replaceFields: true }` — divides and emits a single value field.

Result renders as a stat panel showing a dollar amount. `noValue: "N/A"`
covers the division-by-zero case for quiet periods.

### ROI story complete

Three dashboards now answer the original questions:
- **"What did the factory do today?"** — Factory Today (v0.72.0)
- **"How much did it cost us?"** — Claude Cost & Usage (v0.73.0)
- **"What was our ROI?"** — Factory ROI (v0.74.0 — this release)

Six dashboards total in the bundle:
1. Factory Overview (hook events focused)
2. Claude Code Overview (Claude OTel log events)
3. Factory Today — Unified Activity
4. Factory PRs
5. Claude Cost & Usage
6. Factory ROI — Cost vs Output

### Added tests

- **`tests/factory-obs.bats`** — 6 new tests covering the ROI
  dashboard file, UID stability, derived-panel presence, and mixed-
  datasource transformation plumbing. Suite now at 79 tests.

### Verified

Dashboard loads in Grafana, joins the 6-dashboard list under the VSDD
Factory folder. Derived ratio panels render "N/A" until denominator
events appear (expected — the prism session hasn't produced commits
or PR merges in the current window yet). Cost-over-time timeseries
populated with real spend ($0.83+).

### Known gap — queued

Open→merge PR duration measurement requires LogQL pairing of
`pr.opened` and `pr.merged` events by `pr_number`. Backlog (see
project memory). Approach will likely mirror factory-sla's
agent-start/stop pairing.

### Migration

`factory-obs down && factory-obs up` (or `docker restart
vsdd-obs-grafana`) to pick up the new dashboard. No breaking changes.

## 0.73.1 — PR signal capture hook + cost dashboard metric name fixes + temporality fix

Big follow-up to v0.73.0. Closes backlog task #90 (richer PR signals
via direct Bash capture) and fixes two interacting issues that were
keeping the Claude Cost dashboard empty despite real Claude activity.

### Added

- **`hooks/capture-pr-activity.sh`** (new PostToolUse Bash hook) —
  watches for `gh pr create` and `gh pr merge` invocations and emits
  direct structured events when they succeed. No more inferring PR
  activity from subagent message text; the signal comes from the
  actual command.
  - `type=pr.opened` with fields: `pr_url`, `pr_number`, `pr_repo`,
    `title` (if `--title` flag present).
  - `type=pr.merged` with fields: `pr_url`, `pr_number`, `pr_repo`,
    `merge_strategy` (squash/rebase/merge from flag).
  - Catches PRs opened or merged *outside* the factory pr-manager
    workflow (e.g., human-driven merges).
  - Exit 0 always — advisory, never blocks. Filters on stdout URL
    presence + command-boundary regex match to avoid false positives
    (e.g., `echo "gh pr create is useful"` is correctly ignored).

- **`tests/capture-pr-activity.bats`** (new) — 15 bats tests covering
  create/merge paths, title extraction, merge-strategy flags, the
  positional-PR-number form (`gh pr merge 42`), regex
  tightness (echoed mentions don't match), failure-path no-ops, and
  graceful degradation when `CLAUDE_PLUGIN_ROOT` or emit-event is
  missing.

- **Factory PRs dashboard** — new "PRs opened" stat panel (id 2). Top
  row resized to 5 × w=5 + 1 × w=4 to fit. "PRs merged" stat (id 3)
  and "PR workflows blocked / incomplete" (id 5) rebound to the new
  grid positions.

### Fixed

- **`skills/claude-telemetry/SKILL.md`** — telemetry `on` subcommand
  now writes a SIXTH env var:
  `OTEL_EXPORTER_OTLP_METRICS_TEMPORALITY_PREFERENCE=cumulative`.
  Without this, Claude's OTel SDK defaults to DELTA temporality, and
  the collector's `prometheusremotewrite` exporter rejects every sample
  with `invalid temporality and type combination` — dropping 7-11
  metrics per minute silently. The `deltatocumulative` processor that
  would do the conversion in-flight is not available in
  collector-contrib v0.94.0; normalizing at source is the only fix.

- **`tools/observability/grafana-dashboards/claude-cost.json`** —
  every metric-name query updated to match the real Prometheus
  surface, which includes unit suffixes between the metric name and
  the `_total` counter marker:
  - `claude_code_cost_usage_total` → `claude_code_cost_usage_USD_total`
  - `claude_code_token_usage_total` → `claude_code_token_usage_tokens_total`
  - `claude_code_active_time_total` → `claude_code_active_time_seconds_total`
  
  Verified live against the running Prometheus label index.

- **`tools/observability/grafana-dashboards/factory-today.json`** and
  **`factory-prs.json`** — PRs-merged and PRs-opened queries now
  target the direct `event_type=pr.opened` / `event_type=pr.merged`
  labels instead of the inferred `hook=update-wave-state-on-merge`
  signal. More accurate for dashboard aggregation; the existing
  update-wave-state-on-merge hook continues doing its wave-state.yaml
  work untouched.

### Verified live

- New hook smoke-tested with manual JSON input; emits the correct
  event shape.
- Real Claude cost now visible in Prometheus: $0.83 today; token
  breakdown input=107, output=2610, cacheRead=1.2M, cacheCreation=26K.
  Cache hit ratio is excellent.
- All 73 bats tests pass (suite grew from 58 → 73 with the new
  hook's 15 cases).

### Migration

- Users with existing `claude-telemetry on` state: run
  `/vsdd-factory:claude-telemetry on` again to write the new sixth
  env var, then **restart your Claude Code session**. Without the
  restart, metrics continue to get dropped.
- `factory-obs down && factory-obs up` (or `docker restart
  vsdd-obs-grafana`) to reload dashboard provisioning.

### Known gap — queued for follow-up

- Open→merge duration needs LogQL pairing of `pr.opened` and
  `pr.merged` events by `pr_number`. Non-trivial in LogQL; likely
  needs a factory-sla-style pairing tool. Queued.

## 0.73.0 — Claude Cost & Usage dashboard (Phase B of cost/ROI series)

## 0.73.0 — Claude Cost & Usage dashboard (Phase B of cost/ROI series)

Phase B of the three-phase cost-and-ROI build-out. Uses the Prometheus
infrastructure laid in v0.72.0 to answer "how much did Claude cost us?"
with real dollar figures from Claude's OTel cost metric. ROI dashboard
lands next in Phase C.

### Added

- **`tools/observability/grafana-dashboards/claude-cost.json`** — new
  "Claude Cost & Usage" dashboard, 12 Prometheus-backed panels:
  - Top stats (5): Total cost (USD), Sessions, Input tokens, Output
    tokens, Cache hit ratio (0-1 gauge).
  - Cost timeseries by model — spotting which model drives cost.
  - Cost by model (donut pie) + Token usage by type (horizontal bar
    chart, input/output/cacheRead/cacheCreation).
  - Output-count stats (4): Lines added, Commits via Claude, Active
    session time, Cost per commit (derived).
  - All stats use `noValue: "0"` / `"N/A"` so empty ranges render
    cleanly.

### Metrics referenced

Documented Claude metric surface as translated to Prometheus (dots →
underscores, counters → `_total`):

- `claude_code_cost_usage_total` (USD; model / query_source / speed / effort)
- `claude_code_token_usage_total` (tokens; type / model / ...)
- `claude_code_session_count_total`
- `claude_code_lines_of_code_count_total` (by type added/removed)
- `claude_code_commit_count_total`
- `claude_code_active_time_total` (seconds)

Full metric shape captured in the project's Claude-OTel reference
memory for future dashboard work.

### `claude_code.pull_request.count` distinction

Claude emits this counter for PRs created via its `/install-github-app`
feature — NOT the factory's `pr-manager` subagent dispatches tracked
on the Factory PRs dashboard. Different concepts; dashboards do not
correlate them.

### Verified

`claude-cost.json` parses; all 12 panels reference the Prometheus
datasource (`uid: prometheus`); metric-name assertions pass in the
58-test bats suite. End-to-end data flow was verified in v0.72.0 via
a synthetic OTLP push — dashboard ships empty and populates as real
Claude activity accumulates.

### Migration

`factory-obs down && factory-obs up` to reload the dashboard
provisioning. Dashboard appears under the "VSDD Factory" folder in
Grafana alongside the other four.

### Next: Phase C (ROI)

Will cross-reference factory output signals (PRs merged, wave gates
passed, agent exit classes) with cost data to produce "cost per X"
views. Depends on richer PR signals (backlog) for the X to be
meaningful.

## 0.72.5 — Fix broken gauge query on Factory PRs dashboard

Factory PRs shipped in v0.72.3 with a Completion Ratio gauge whose
LogQL query was malformed. The query tried `1 - (blocked/dispatched)`
to invert the ratio, but LogQL doesn't accept a scalar on the left
of a top-level binary operation. Grafana rendered
`parse error at line 1, col 1: syntax error: unexpected IDENTIFIER`.

### Fixed

- **`tools/observability/grafana-dashboards/factory-prs.json`** —
  - Renamed "Completion ratio" → "Incomplete rate" (panel id 4).
    Semantics flipped: low is good (0 = every dispatch finished all
    10 steps), high is bad (1 = every dispatch blocked before step
    10). Color thresholds inverted accordingly: green/yellow/red at
    0 / 0.3 / 0.7.
  - Query rewritten as `blocked / dispatched` — pure vector/vector
    division, LogQL-compatible. Verified live with real data: returns
    `0.5` for the current state (1 blocked / 2 dispatched).
  - `noValue: "0"` added so fresh installs with no PR activity render
    "0%" instead of "No data".

### Why not just fix the subtraction?

LogQL does support binary operations between two aggregated queries,
but not with a literal scalar on the left at top level. Possible
rewrites that work:
- `(dispatched - blocked) / dispatched`
- `scalar(blocked) / scalar(dispatched)` wrapped differently

Both are more fragile than just showing the blocked ratio directly.
"Incomplete rate" is the same information with cleaner semantics and
a simpler query.

### Migration

Restart Grafana or `factory-obs down && up`. Browser may cache the
old dashboard — hard-reload if the panel still shows the error.

## 0.72.4 — Stat panels show "0" instead of "No data" for zero counts

User-reported: the "PRs merged" stat on Factory Today rendered
"No data" even when the answer was unambiguously "zero PRs merged
today." Grafana treats a Loki `count_over_time` that returns no
matching streams as null, and null → "No data" by default. But a
zero-count is a meaningful observation, not missing data — the
display should say "0."

### Fixed

- All stat panels across `factory-overview.json`,
  `factory-today.json`, `factory-prs.json`, and `claude-overview.json`
  now set `fieldConfig.defaults.noValue: "0"`. Zero-result queries
  render as "0" instead of "No data", making the semantic
  distinction honest: "we know the count is zero" vs. "the query
  errored / no data exists."

Doesn't affect gauge, bargauge, or logs panels — only stat panels,
which have a binary "show a number" mode where zero-vs-null
ambiguity matters.

### Migration

Restart Grafana or `factory-obs down && up` to pick up the dashboard
changes. No query or data-model changes.

## 0.72.3 — Factory PRs dashboard + Factory Today cross-reference

Dedicated dashboard for PR workflow signals, built on the events we
already capture. Complements Factory Today (general activity) and
feeds the upcoming cost/ROI story in Phases B and C.

### Added

- **`tools/observability/grafana-dashboards/factory-prs.json`** — new
  "Factory PRs" dashboard at URL path `factory-prs`, 7 panels:
  - PR workflows dispatched (stat) — `agent.start` events with
    `attributes_subagent="vsdd-factory:pr-manager"`
  - PR workflows blocked / incomplete (stat) — `pr-manager-completion-guard`
    block events
  - PRs merged (stat) — `update-wave-state-on-merge` hook events
  - Completion ratio (gauge) — approximate, 1 - (blocked / dispatched)
  - PR workflow activity timeline (timeseries)
  - Where PR workflows stall (bargauge) — distribution of `attributes_last_step`
    from completion-guard blocks; shows which of the 10 PR workflow
    steps the pr-manager is most likely to stop at
  - Recent PR workflow events (logs) — filtered to pr-manager activity

- **`tools/observability/grafana-dashboards/factory-today.json`** — new
  "PRs merged" stat panel (id 10) in the top row, so the top-line
  number appears on the daily overview without needing to switch
  dashboards. Top-row stats resized from `w=6 × 4` to `w=5 × 4 + w=4`
  to fit.

- **`tests/factory-obs.bats`** — 7 new tests covering the PRs
  dashboard file, UID stability, panel count, Loki-only queries, and
  the cross-reference stat on Factory Today. Suite now 52 tests.

### Backlog items captured (not shipped here)

Two signal-coverage gaps surfaced during build that are deferred to
after Phase B (cost) and Phase C (ROI):
1. Richer PR instrumentation — explicit "PR opened" events (new hook
   on `gh pr create` Bash invocations), PR URL/number capture, paired
   open-to-merge duration. Needed to make "cost per PR" truly
   meaningful.
2. Per-subagent usage dashboard — ranked panel showing every subagent
   type by invocation count, avg duration, and exit class mix. Uses
   existing `agent.start` / `agent.stop` events.

### Migration

Restart Grafana or `factory-obs down && up` to load the new dashboard.
The PRs-merged stat on Factory Today will populate as soon as the
`update-wave-state-on-merge` hook fires in any instrumented worktree.

## 0.72.2 — Factory Today bargauge number + label correctness

Two follow-on fixes for the Factory Today dashboard based on live
review. The panels were showing wildly inflated numbers (e.g.,
`block-ai-attribution = 214` when Loki had only 1 event for that hook)
and the Top Tools / Stories panels weren't showing their bar labels.

### Root causes

1. **Inflated counts**: all bargauge panels used
   `"reduceOptions.calcs": ["sum"]`. Grafana range queries against Loki
   emit a time series of evaluations — each bucket at time T re-runs
   `count_over_time([$__range])` and returns the range-wide count. With
   `sum` as the reducer, the panel summed ~180 buckets × actual_count,
   inflating each bar by 100-200×. Stat panels don't have this issue
   because they use `lastNotNull`, which takes the most recent bucket
   (which IS the actual range-wide count).

2. **Missing tool / story labels**: the Top Hooks panel renders labels
   correctly because it groups by `hook` — a promoted Loki stream label
   (via the `loki.resource.labels` hint added in v0.70.2). Top Tools
   and Stories group by `attributes_tool_name` and `attributes_story_id`
   — fields extracted at query time via `| json`. Grafana's bargauge
   shows real stream labels automatically but needs an explicit
   `displayName` override to surface query-extracted fields.

### Fixed

- **`tools/observability/grafana-dashboards/factory-today.json`** —
  - All three bargauge panels (Top Hooks, Top Tools, Stories): changed
    `reduceOptions.calcs` from `["sum"]` to `["lastNotNull"]`. Bar
    values now match the actual event counts in Loki.
  - Top Tools panel: added
    `fieldConfig.defaults.displayName: "${__field.labels.attributes_tool_name}"`
    so bars render with the tool name instead of the full label set.
  - Stories panel: same pattern with `attributes_story_id`.

### Verified live

Reloaded the dashboard after applying the fix. Numbers now align with
`wc -l .factory/logs/events-*.jsonl` and with direct Loki queries.
Stories panel shows `S-1.07` as its bar label; Tools shows `Bash`.

### Migration

Restart Grafana or `factory-obs down && up` to reload the dashboard
JSON. No data changes — only visual correctness.

## 0.72.1 — Fix Factory Today stories panel + metrics pipeline verified

Follow-up to v0.72.0. Same class of bug as v0.71.0's Claude-dashboard
queries: field name mismatch between the raw event JSON and Loki's
`| json` flattened output.

### Fixed

- **`tools/observability/grafana-dashboards/factory-today.json`** —
  stories-touched query now uses `attributes_story_id` (Loki's
  flattened form), not bare `story_id`. Verified live: returns
  `S-1.07 = 2` against today's prism activity.

### Verified

- **Metrics pipeline end-to-end**: synthetic OTLP push to
  `http://localhost:4318/v1/metrics` → HTTP 200 → metric appeared in
  Prometheus as `claude_code_token_usage_synthetic_total` within
  seconds. The empty Prometheus state in a fresh install is purely a
  "no Claude session actively emitting" artifact, not a wiring bug.

- **Top tools (Claude) panel**: working as designed. Renders a single
  large bar when Claude has used only one tool (Bash) in the range;
  the tool name label sits left of the bar and can crop at narrow
  window widths.

### Migration

Restart Grafana or `factory-obs down && up` to pick up the dashboard
fix.

## 0.72.0 — Prometheus + Factory Today dashboard (Phase A of cost/ROI series)

Phase A of the three-phase cost-and-ROI observability build-out. Lays
the metrics-storage foundation (Prometheus) and ships the first new
dashboard (unified daily activity view). Cost dashboard lands in
v0.72.1; ROI in v0.72.2.

### Added

- **`tools/observability/prometheus-config.yaml`** — minimal Prometheus
  config. Acts as a remote_write sink only; no scrape jobs beyond a
  self-scrape. 30-day retention set on the command line in compose.

- **`tools/observability/docker-compose.yml`** — new `prometheus`
  service (`prom/prometheus:v2.54.0`), port 9090 (env-overridable via
  `VSDD_OBS_PROMETHEUS_PORT`). Started with
  `--web.enable-remote-write-receiver` so the collector can push
  metrics over OTLP. Health-checked. `otel-collector` and `grafana`
  both depend on it being healthy before starting — prevents the
  collector's first push getting a connection-refused.

- **`tools/observability/otel-collector-config.yaml`** — new
  `prometheusremotewrite` exporter pushing to
  `http://prometheus:9090/api/v1/write`.
  `resource_to_telemetry_conversion.enabled: true` so resource
  attributes (`service.name`, session id, model) surface as labels on
  every metric. Metrics pipeline now exports to both Prometheus AND
  the existing `debug` exporter — stdout visibility retained alongside
  persistence.

- **`tools/observability/grafana-provisioning/datasources/prometheus.yaml`** —
  Prometheus datasource with pinned `uid: prometheus` (same convention
  as the loki datasource).

- **`tools/observability/grafana-dashboards/factory-today.json`** —
  unified daily activity dashboard. 9 panels, all Loki-backed
  (Prometheus-backed cost/ROI panels live on dedicated dashboards in
  v0.72.1+):
  - Events today (total across both streams)
  - Hook blocks (factory guards)
  - Tool calls (Claude)
  - Unique Claude sessions
  - Activity timeline stacked by `service_name`
  - Top hooks fired
  - Top tools used
  - Stories touched today (parsed from `agent.start.story_id`)
  - Recent significant events (`hook.block` + `hook.action`)

  Default time range is "today" (`now/d` to `now`) — wall-clock
  midnight-to-now rather than the typical rolling 24h window.

- **`bin/factory-obs`** — help text and `up` confirmation now mention
  Prometheus URL and `VSDD_OBS_PROMETHEUS_PORT`.

- **`tests/factory-obs.bats`** — 14 new tests covering the Prometheus
  service definition, collector wiring, datasource provisioning, and
  the factory-today dashboard. Total suite size now 45 tests.

### Verified live

Stack restarted against prism's active event stream; confirmed all four
containers healthy, both datasources provisioned, Prometheus
remote_write accepting pushes (invalid-samples counter = 0), collector
emitting no errors. Claude metrics populate on next Claude-session
activity (metrics batch on a 60s default interval).

### Migration

Existing installs: `factory-obs down && factory-obs up` to pick up the
new service. First start pulls the Prometheus image (~250MB). If port
9090 is in use, set `VSDD_OBS_PROMETHEUS_PORT=<alternate>` before `up`.

No breaking changes. Logs-to-Loki continues working even if Prometheus
is down — pipelines are independent.

## 0.71.1 — Claude dashboard queries fixed against real Claude OTel shape

v0.71.0 shipped the `claude-overview.json` dashboard with queries based
on docs-level assumptions about Claude Code's OTel shape. Real data
showed the shape differs: Claude emits attribute keys with dots
(`attributes.session.id`, `attributes.event.name`, `attributes.tool_name`),
and Loki's `| json` operator flattens them with underscores
(`attributes_session_id`, `attributes_event_name`, `attributes_tool_name`).
Queries that assumed bare `session_id` / `tool_name` returned empty.

### Fixed

- **`tools/observability/grafana-dashboards/claude-overview.json`** —
  queries now use the flattened attribute paths Loki actually produces:
  - Panel 2 (Unique sessions): `attributes_session_id` — resolved to 2.
  - Panel 3 (Tool invocations): filters on
    `attributes_event_name = "tool_result"` — resolved to 2.
  - Panel 4 (API requests): filters on
    `attributes_event_name = "api_request"` — resolved to 3.
  - Panel 5 (Events over time): breakdown by `attributes_event_name`.
  - Panel 6 (Top tools): `attributes_tool_name` — resolved to Bash = 3.

Verified live against real prism-session Claude data.

### Event names observed in real Claude data (v2.1.118)

For reference when building future dashboards:
`user_prompt`, `api_request`, `api_response`, `tool_result`,
`tool_decision`, `hook_execution_start`, `hook_execution_complete`,
`mcp_server_connection`.

Memory reference `claude-code-otel-reference` updated with the full
attribute shape.

### Migration

Restart Grafana (or `factory-obs down && up`) to pick up the corrected
dashboard JSON.

## 0.71.0 — Claude dashboard + CI parity check + release script

Bundles three pieces of release-hygiene + observability work that have
been on the backlog since the v0.68.1/v0.69.0 release-workflow incident
and v0.69.0's introduction of Claude OTel ingestion without a matching
dashboard:

1. **CI parity check** — the missing gate that would have caught v0.68.1
   and v0.69.0 before tag push.
2. **`scripts/bump-version.sh`** — atomic release helper so the lockstep
   rule (plugin.json ↔ marketplace.json ↔ CHANGELOG) can't be forgotten
   by accident.
3. **`claude-overview.json` Grafana dashboard** — first-pass dashboard
   for the `claude-code` service pipeline wired in v0.69.0. Queries
   don't assume Claude's full label schema yet — they use `| json`
   parsing so they work against whatever structured-metadata shape
   Claude's OTel SDK produces.

### Added

- **`.github/workflows/ci.yml`** — new step `Verify version parity across
  plugin.json and marketplace.json`. Runs on every push to main + PRs.
  Fails fast with an actionable message pointing at
  `scripts/bump-version.sh` if `plugin.json.version` and
  `marketplace.json.plugins[0].version` drift. This catches the failure
  mode that bit v0.68.1 (silent Release workflow failure on tag push)
  at the earliest possible moment — on the commit that introduces the
  drift, not at the tag two commits later.

- **`scripts/bump-version.sh`** — one-command version bump that updates
  plugin.json, marketplace.json, and prepends a CHANGELOG heading stub
  with today's date. Validates semver format. Refuses to run if either
  version file has uncommitted changes (prevents clobbering
  work-in-progress). Does **not** commit or tag — the operator reviews
  the diff and stages manually. This release dogfoods the script.

- **`tools/observability/grafana-dashboards/claude-overview.json`** —
  new Grafana dashboard at URL path `claude-code-overview`, auto-
  provisioned under the "VSDD Factory" folder. Seven panels:
  - Total events (Claude)
  - Unique sessions (parsed from `session_id` via `| json`)
  - Tool invocations (matches `tool_result` in body)
  - API requests (matches `api_request` in body)
  - Events over time
  - Top tools used (top 10 by `tool_name`)
  - Recent events (last 50, raw log)
  
  Queries are intentionally forgiving — only `service_name` is assumed
  to be a promoted Loki label; everything else goes through `| json` so
  the panels work whatever label schema Claude's SDK produces. Tune to
  promoted labels later once real data accumulates.

### Migration

No breaking changes. The new dashboard auto-provisions on next
`factory-obs up`. Existing installs may need `factory-obs down && up`
to pick up the new `claude-overview.json` file if Grafana's dashboard
provider doesn't hot-reload for them.

To see Claude telemetry in the new dashboard:
1. `factory-obs up`
2. `/vsdd-factory:claude-telemetry on`
3. Restart your Claude Code session so env vars load.

## 0.70.3 — Blocks (hard) threshold: yellow at 1+, red at 10+

Cosmetic tweak to the bundled `factory-overview` dashboard. Previously
any non-zero count in the `Blocks (hard)` stat panel turned it red,
which over-reacted to routine hook fires like `block-ai-attribution`
(normal during AI-assisted commits). New threshold scheme distinguishes
"some expected activity" from "unusual volume worth investigating":

- `0` → green
- `1-9` → yellow (expected during active sessions)
- `10+` → red (elevated, investigate)

### Fixed

- **`tools/observability/grafana-dashboards/factory-overview.json`** —
  `Blocks (hard)` panel thresholds updated. No query change; just the
  color mapping. Other panels (`Blocks (warn)`, `Actions`, `Total
  events`) unchanged.

### Migration

Dashboard is provisioned from this JSON on Grafana startup. Existing
installs need `factory-obs down && factory-obs up` to reload the
dashboard file (or `factory-obs reset` if other state is stale).

## 0.70.2 — Loki label hint: move to a log-record processor (fixes v0.70.1)

v0.70.1 placed the `loki.resource.labels` hint on the `resource`
processor — which silently did nothing. The Loki exporter reads that
hint off the **log record's** attributes, not the resource's. Verified
live against the running stack: before this fix, Loki labels were
only `service_name`, `exporter`, `job`. After this fix, new events
ship with the full set — `event_type`, `hook`, `reason`, `severity`
— and the dashboard's filter panels resolve.

### Fixed

- **`tools/observability/otel-collector-config.yaml`** — introduced a
  new `attributes/loki-label-hint` processor (log-record scope) that
  sets `loki.resource.labels` on each log record. Wired into the
  hook-events pipeline after `resource` and before `loki` export.
  Removed the dead hint from the `resource` processor. Verified
  label-promotion works in Loki exporter v0.94.0.

### Migration

Existing installs need `factory-obs down && factory-obs up` to reload
config. Events ingested under v0.70.1 (or earlier) are stuck as
structured metadata — Loki doesn't retroactively relabel chunks. If
the old `No data` panels need to clear, `factory-obs reset` wipes the
Loki volume; `.factory/logs/events-*.jsonl` remains the source of
truth and the collector re-ingests with correct labels on next start.

## 0.70.1 — Promote `event_type` / `hook` / `reason` / `severity` to Loki labels

Completes the dashboard-resolution work started in v0.69.2. With the
service.name fix applied, the `Total events` and `Recent events` panels
started working (confirmed via a screenshot showing 66 events). But
the filter panels — `Blocks (hard)`, `Blocks (warn)`, `Actions`,
`Events over time (by type)`, `Top block reasons`, `Blocks by hook` —
all still showed "No data" because their LogQL queries use
`event_type`, `hook`, `reason`, and `severity` as Loki **labels** in
stream selectors and `by()` groupings, but the collector was storing
them as **structured metadata**. Different syntax; the queries found
nothing.

### Fixed

- **`tools/observability/otel-collector-config.yaml`** — added the
  `loki.resource.labels` hint (`"event_type, hook, reason, severity"`)
  on the `resource` processor. This is the documented Loki-exporter
  mechanism for promoting resource attributes to stream labels.
  Cardinality is bounded: ~4 event_types × ~40 hooks × ~70 reasons ×
  ~4 severities keeps local Loki comfortable.

The `resource/claude` processor is deliberately NOT touched — Claude's
native telemetry has its own attribute shape and shouldn't be forced
into our label schema.

### Migration

Existing installs need `factory-obs down && factory-obs up` for the
collector to reload config. Historical events in Loki stay as
structured metadata under their old streams (Loki doesn't re-label
historical chunks); new events start flowing with the promoted labels
immediately. Run `factory-obs reset` if you want a clean slate —
`.factory/logs/events-*.jsonl` remains the source of truth.

## 0.70.0 — Worktree-aware log dir: events aggregate into main repo's `.factory/logs/`

Previously, hooks running inside a git worktree wrote their events to
`<worktree>/.factory/logs/`, so the activity stream fragmented across
every linked worktree. If the observability stack was started from
inside a worktree, it tailed only that one worktree's events — and
other worktrees' activity was invisible. Debugging this consumed real
time (prism's main repo had 4 events; its active worktree had 95).

This release fixes the fragmentation by making every event-producing
and event-consuming tool resolve to the **main worktree's**
`.factory/logs/` by default.

### Changed

- **`bin/emit-event`** — resolves the main worktree via
  `git worktree list --porcelain | awk '/^worktree /{print $2; exit}'`
  (first entry is always the main worktree). Events from any linked
  worktree now land in `<main-worktree>/.factory/logs/`. Explicit
  `VSDD_LOG_DIR` still wins; fallback to cwd's `.factory/logs/` if
  git is unavailable or cwd isn't a repo.

- **`bin/factory-query`**, **`bin/factory-report`**, **`bin/factory-replay`**,
  **`bin/factory-sla`** — same resolution so queries see the unified
  event stream from every worktree regardless of which one you're
  sitting in.

- **`bin/factory-dashboard`** — when `--factory PATH` is passed
  explicitly, log resolution honors that path (caller scoped the run).
  When `--factory` is default, uses the main-worktree resolution. This
  preserves the existing escape hatch.

- **`bin/factory-obs`** — the Docker stack now mounts the main
  worktree's `.factory/logs/` by default, so the collector tails the
  aggregated stream. Restart required: `factory-obs down && up`.

### Added (tests)

- **`tests/emit-event.bats`** — 3 new tests covering the resolution
  matrix: `VSDD_LOG_DIR` explicit override wins; main-worktree
  resolution fires inside a git worktree; graceful fallback to cwd
  when git is unavailable. Each test builds a temp repo + worktree
  via `git worktree add` so it doesn't rely on the surrounding
  repo's state.

### Migration

Existing installs need a one-time migration to see the aggregated
stream:

```bash
# Stop the stack, then rebind to the main worktree's logs
factory-obs down
cd <main-repo-root>  # or simply re-run `factory-obs up` from any worktree
factory-obs up
```

Historical events in `<worktree>/.factory/logs/events-*.jsonl` files
are not moved automatically — they stay where they are. If you want
them aggregated, copy or symlink them into
`<main-repo>/.factory/logs/` before starting the stack (the filelog
receiver will ingest them like any other event file).

Setting `VSDD_LOG_DIR` or `VSDD_FACTORY_LOGS` preserves the pre-v0.70
behavior for anyone who needs per-worktree isolation. The fallback to
cwd for non-git directories is unchanged.

## 0.69.2 — Observability stack: fix service.name label so dashboards resolve

Patch release. Fixes a silent-data bug in the local observability stack
that has been present since v0.66.0: all hook events land in Loki with
the label `service_name=unknown_service` instead of
`service_name=vsdd-factory`, so every panel in the bundled
`factory-overview` dashboard returns "No data" even though events are
flowing through the collector correctly.

### Root cause

`tools/observability/otel-collector-config.yaml` set the resource
attribute key as `service_name` (underscore). The Loki exporter's label
promotion rule looks for `service.name` (dotted — the OTel semantic
convention). When it doesn't find the dotted key, it falls back to
`service_name=unknown_service` as the Loki label. Our underscore
attribute was still emitted, but as structured metadata inside the log
body — not as a queryable Loki label.

### Fixed

- **`tools/observability/otel-collector-config.yaml`** — both `resource`
  and `resource/claude` processors now set `service.name` (dotted) with
  `upsert`. Hook events now land with
  `{service_name="vsdd-factory"}` and Claude's native telemetry with
  `{service_name="claude-code"}`.

### Migration

Existing installs must `factory-obs down && factory-obs up` to reload
the collector config. Data already in Loki under `unknown_service`
remains there (Loki doesn't re-label historical chunks), but new events
will land under the correct label immediately. If you want the old
data to disappear, also run `factory-obs reset` — this wipes Loki's
volume, which is fine for local-dev observability since events are
authoritative in `.factory/logs/events-*.jsonl`.

## 0.69.1 — Release workflow fix: sync marketplace.json version

Patch release. Unblocks the GitHub Release workflow, which has been
silently failing on every tag push since v0.68.1 because
`.claude-plugin/marketplace.json` was not bumped alongside
`plugins/vsdd-factory/.claude-plugin/plugin.json`. The plugin itself
still installs correctly via the tag — only the Releases-page entry
was missing.

### Fixed

- **`.claude-plugin/marketplace.json`** — bumped `plugins[0].version`
  from `0.68.0` to `0.69.1` to match `plugin.json`. The release
  workflow validates tag ↔ marketplace.json AND tag ↔ plugin.json;
  any drift fails the job and skips the GitHub Release creation step.

### Process note

Future releases MUST bump all three version fields in lockstep:
- `plugins/vsdd-factory/.claude-plugin/plugin.json` — plugin itself
- `.claude-plugin/marketplace.json` — marketplace entry
- `CHANGELOG.md` — new section heading

No v0.68.1 / v0.69.0 GitHub Release entries will be backfilled; the
tags remain the source of truth and installs continue to work
unchanged.

## 0.69.0 — Claude Code native telemetry → factory observability stack

Turns the local observability stack into a two-stream feed. Hook events
(`service_name=vsdd-factory`) remain the interpretation layer. Claude
Code's native OpenTelemetry export (`service_name=claude-code`) now
flows into the same Loki, giving a complete "what did the factory do
today?" activity feed — every tool call, token count, and API event —
without requiring us to instrument more hooks.

### Fixed

- **`tools/observability/grafana-provisioning/datasources/loki.yaml`** —
  pinned the Loki datasource UID to `loki`. Without this, Grafana
  auto-generates a random UID, and the bundled `factory-overview`
  dashboard (which references the datasource by `"uid": "loki"` in all
  16 panels) errors with "datasource loki not found" on every panel.
  Affects existing installs; users should run `factory-obs reset &&
  factory-obs up` to re-provision the datasource with the pinned UID.

### Added

- **`tools/observability/otel-collector-config.yaml`** — added an `otlp`
  receiver (HTTP on 4318, gRPC on 4317) and three new pipelines:
  - `logs/claude` — Claude Code OTel logs → Loki, tagged
    `service_name=claude-code` via a new `resource/claude` processor.
    Query in Grafana with `{service_name="claude-code"}`.
  - `metrics` — Claude's metrics → `debug` exporter (basic verbosity,
    no persistent backend). Pipelines must exist or the OTLP receiver
    rejects the signal type. Swap for Prometheus if metrics querying
    becomes important.
  - `traces` — symmetric to metrics, opt-in on Claude's side.

- **`skills/claude-telemetry/SKILL.md`** + **`commands/claude-telemetry.md`** —
  new `/vsdd-factory:claude-telemetry` slash command with three
  subcommands:
  - `on` (default) — writes five OTEL env vars
    (`CLAUDE_CODE_ENABLE_TELEMETRY`, `OTEL_METRICS_EXPORTER`,
    `OTEL_LOGS_EXPORTER`, `OTEL_EXPORTER_OTLP_PROTOCOL`,
    `OTEL_EXPORTER_OTLP_ENDPOINT`) into
    `.claude/settings.local.json` under the `env` block via `jq` merge.
    Preserves all other `env` entries and top-level keys.
  - `off` — removes exactly those five keys and deletes `env` if empty.
  - `status` — prints which of the five keys are currently set.
  
  Reversible, per-project, respects `settings.local.json` (gitignored
  per convention). `disable-model-invocation: true` — fires on explicit
  request only, matching the `factory-obs` / `activate` pattern for
  configuration side-effects.

### Defaults and rationale

- Protocol is pinned to `http/protobuf` because `docker-compose.yml`
  only host-exposes port 4318. Users who want gRPC can remap 4317 and
  override `OTEL_EXPORTER_OTLP_PROTOCOL` manually.
- Endpoint is `http://localhost:4318`. If `VSDD_OBS_OTLP_HTTP_PORT` is
  remapped in compose, users override the endpoint manually — the skill
  does not read environment variables from the factory-obs layer
  because the skill writes to user settings, not collector settings.
- The skill does **not** toggle sensitive-content gates
  (`OTEL_LOG_USER_PROMPTS`, `OTEL_LOG_TOOL_DETAILS`, `OTEL_LOG_TOOL_CONTENT`).
  Users who want those must add them manually. The skill's
  documentation calls this out explicitly with a privacy note.

### Migration

No breaking changes. Installs on v0.68.x continue to work — the new
OTLP receiver and pipelines are additive. To start using Claude's
native telemetry:

1. `factory-obs up` — ensure the stack is current and accepting OTLP.
2. `/vsdd-factory:claude-telemetry on` — write the env vars.
3. Restart your Claude Code session.
4. Query in Grafana with `{service_name="claude-code"}`.

If you're upgrading from <v0.69.0 and your collector container was
started against the old config, recreate it: `factory-obs down && factory-obs up`.

## 0.68.1 — Observability stack: first-run fixes + /factory-obs skill

Two shippable fixes found while smoke-testing `factory-obs up` on a clean
machine, plus a convenience skill so the stack can be driven from a slash
command.

### Fixed

- **`tools/observability/docker-compose.yml`** — the otel-collector crashed
  with `permission denied` on `/var/lib/otelcol/receiver_filelog_` on first
  run. The `otel/opentelemetry-collector-contrib:0.94.0` image runs as UID
  10001, but Docker creates the `collector-state` named volume root-owned,
  so the collector couldn't persist its filelog offsets and entered a
  crash loop. Added `user: "0:0"` to the otel-collector service. This is a
  local-only dev stack and the volume is isolated from the host, so it
  doesn't grant host privileges.

- **`tools/observability/otel-collector-config.yaml`** — the `move`
  operators for `attributes.hook` and `attributes.reason` threw
  `field does not exist` errors on events that don't carry those fields
  (`agent.start`, `agent.stop`). Added `if: 'attributes.X != nil'` guards
  matching the pattern already used for `severity`. Errors are gone from
  collector logs; hook.block / hook.action events continue to promote
  `hook` and `reason` to resource attributes as before.

### Added

- **`skills/factory-obs/SKILL.md`** + **`commands/factory-obs.md`** — new
  `/vsdd-factory:factory-obs` slash command. Accepts the same subcommand
  surface as the binary (`up` / `down` / `reset` / `status` / `logs` /
  `dashboard` / `help`) and defaults to `up` when called with no args.
  `disable-model-invocation: true` — only fires on explicit user request,
  matching the `factory-dashboard` pattern for infrastructure commands
  that side-effect Docker.

### Migration

No breaking changes. Users who already have the stack running with the
old compose should run `factory-obs reset && factory-obs up` to recreate
the collector-state volume with the new `user:` directive. The reset
loses the collector's filelog read-offsets, so events already in the
`.factory/logs/events-*.jsonl` files will be re-ingested (duplicates in
Loki for the short pre-reset window). Fresh installs are unaffected.

## 0.68.0 — Observability Phase 6.2: agent SLO tracking + factory-sla

Derives per-invocation subagent durations by pairing new `agent.start` and
`agent.stop` telemetry events. Also closes a test-side papercut where
`factory-obs dashboard` was opening a browser during BATS runs.

### Added

- **`hooks/track-agent-start.sh`** (PreToolUse on Agent matcher) — emits
  one `agent.start` event per subagent dispatch. Never blocks; exits 0
  on every path. Fields: `subagent`, `session_id` (auto from env),
  `story_id` (best-effort extracted from prompt).

- **`hooks/track-agent-stop.sh`** (SubagentStop, all agents) — emits one
  `agent.stop` event per subagent completion. Classifies `exit_class`
  into `ok` / `empty` / `blocked` from the subagent's result text.
  Fields: `subagent`, `session_id`, `exit_class`, `result_len`.

- **`bin/factory-sla`** — duration analysis CLI. Three subcommands:
  - `durations` — list each matched (start, stop) pair with
    session / subagent / timestamps / duration_sec / exit class /
    story_id. Flags: `--session`, `--subagent`, `--days`, `--limit`,
    `--tsv`.
  - `summary` — per-subagent aggregate: count, min, p50, p90, p99,
    max, mean (seconds).
  - `open` — starts without matching stops (in-flight or orphaned).

  Pairing rule: per `(session_id, subagent)` tuple, each `agent.stop`
  pairs with the most recent unpaired `agent.start`. Nested dispatches
  of the same subagent in one session stack correctly.

- **`emit-event`** — new `ts_epoch` field (Unix seconds) alongside `ts`.
  Additive schema change. Tools computing durations should prefer
  `ts_epoch` since tz-offset `ts` parsing varies across platforms.

### Fixed

- **`bin/factory-obs dashboard`** — no longer launches a browser in
  non-interactive contexts (BATS runs, CI). Gated by `[ -t 1 ]` TTY
  check, with an explicit `VSDD_OBS_OPEN_BROWSER` env var override
  (`1` forces, `0` suppresses, unset = auto). Tests now pass
  `VSDD_OBS_OPEN_BROWSER=0` explicitly.

  Side benefit: the dashboard URL was being opened even when the Docker
  stack wasn't running — browser would show connection-refused. The
  TTY gate prevents both issues.

### Changed

- **`hooks/hooks.json`** — wires the two new telemetry hooks into the
  Agent / SubagentStop matchers. They run alongside existing validators
  (wave-gate-prerequisite, pr-merge-prerequisites, handoff-validator,
  etc.), not replacing anything.

### Added (tests)

- **`tests/agent-tracking.bats`** (new) — 14 tests: track-agent-start
  (emission on Agent, story_id extraction, non-Agent no-op,
  CLAUDE_PLUGIN_ROOT resilience), track-agent-stop (emission,
  exit_class classification for ok/empty/blocked, resilience), and
  hooks.json wiring.

- **`tests/factory-sla.bats`** (new) — 21 tests: structural, durations
  with session/subagent filters, TSV output, exit class display,
  story_id propagation, summary percentile correctness, open-unmatched
  detection, empty-state graceful handling.

- **`tests/emit-event.bats`** — 1 new test (41 total) asserting
  `ts_epoch` is an integer in a sane range.

- 1073 tests across 35 suites, 0 failures.

### Deferred to Phase 6.3

- **Pipeline flame graphs** — would need a shift from logs-based
  emission to OTel tracing spans (parent/child relationships for nested
  subagent dispatches), plus adding Tempo to the Docker stack. Bigger
  architectural move; worth its own release.

## 0.67.0 — Observability Phase 6.1: session ID injection + factory-replay

Foundation for session-scoped analysis. Events now carry `session_id`
auto-injected from Claude Code's `$CLAUDE_SESSION_ID` env var, enabling
replay of what a specific session fired. Unblocks Phase 6.2 (agent SLOs)
and 6.3 (flame graphs) — both need session grouping to be meaningful.

### Changed

- **`bin/emit-event`** — auto-injects `session_id` field from env vars
  with priority order:
  1. Caller-provided `session_id=` arg (highest priority)
  2. `$VSDD_SESSION_ID` (test / override)
  3. `$CLAUDE_SESSION_ID` (Claude Code native env var)
  4. No session_id (event is orphan, still valid)

  Purely additive schema change. Events emitted before 0.67 lack
  `session_id` and simply group under `(no-session)` in replay views.

### Added

- **`bin/factory-replay`** — session replay CLI. Three subcommands:
  - `sessions [--days N] [--limit N] [--tsv]` — list distinct sessions
    with event count + first/last timestamps, sorted by recency.
  - `show <session_id> [--limit N]` — chronological playback of a
    specific session. Pass `(no-session)` to see orphan events.
  - `latest [--limit N]` — replay the most recent session with a
    session_id.

  Output is a readable ts / severity / hook / reason / context table.
  Gracefully handles missing log dir, empty log dir, and "only orphan
  events" cases.

### Updated (schema)

- **`docs/guide/observability.md`** — event schema table now includes
  the optional `session_id` field with priority rules.

### Added (tests)

- **`tests/emit-event.bats`** — 5 new tests (40 total) covering
  session_id auto-injection priority: `VSDD_SESSION_ID` override,
  `CLAUDE_SESSION_ID` fallback, env-var precedence (VSDD wins), caller
  override beats env, absence when neither set.
- **`tests/factory-replay.bats`** (new) — 16 tests: subcommand
  structure, session listing + sort order, session filtering, orphan
  event handling, chronological ordering in `show`, latest with/without
  sessions, empty-state graceful handling.
- 1038 tests across 33 suites, 0 failures.

### Deferred to Phase 6.2 / 6.3

- Agent duration tracking — needs hooks at both PreToolUse Agent
  (dispatch) and SubagentStop (return), plus a correlation ID so start
  and end events can be paired. Planned for Phase 6.2.
- Flame graph visualization — needs OTel tracing spans, which means
  adding Tempo to the Docker stack and changing the emission layer
  from logs to traces. Bigger architectural shift. Planned for 6.3.

## 0.66.0 — Observability Phase 5: local Docker stack (Grafana dashboards)

The visual layer on top of the event log. Opt-in Docker stack that tails
`.factory/logs/events-*.jsonl` and renders preconfigured Grafana
dashboards. Three containers: OTel Collector, Loki, Grafana. No cloud
services, no telemetry leaves your machine.

### Added

- **`plugins/vsdd-factory/tools/observability/`** — ships with vsdd-factory:
  - `docker-compose.yml` — 3-service stack. Health-checked. Configurable
    ports via env vars. Default: Grafana 3000, Loki 3100, OTLP HTTP 4318.
  - `otel-collector-config.yaml` — `filelog` receiver tails
    `events-*.jsonl`, JSON-parses each line, promotes
    `type`/`hook`/`reason`/`severity` to resource attributes, exports
    to Loki. Uses the `file_storage` extension to track ingest offsets
    across restarts (no duplicate ingestion).
  - `grafana-provisioning/datasources/loki.yaml` — preconfigured Loki
    datasource.
  - `grafana-provisioning/dashboards/provider.yaml` — dashboard
    provisioning pointing at a mounted `/var/lib/grafana/dashboards`.
  - `grafana-dashboards/factory-overview.json` — starter dashboard with
    8 panels: total events / blocks (hard) / blocks (warn) / actions
    stat tiles, stacked time series of events by type, top-block-reasons
    table, blocks-by-hook bar gauge, and a live log stream.
  - `README.md` — quickstart, architecture diagram, troubleshooting,
    uninstall.

- **`bin/factory-obs`** — lifecycle CLI: `up` / `down` / `reset` / `status`
  / `logs` / `dashboard`. Auto-detects `docker compose` (v2 subcommand)
  or `docker-compose` (v1 binary). Sets `VSDD_FACTORY_LOGS` to the
  project's `.factory/logs/` if unset. `dashboard` prints the Grafana URL
  and opens the browser on macOS / Linux.

### Design decisions

- **Why not `grafana/otel-lgtm` single-container?** Separate services
  give us precise provisioning paths (Grafana provisioning APIs vary by
  distribution) and standard images that are well-documented. The
  overhead of one extra container is negligible compared to easier
  troubleshooting.
- **Why no Prometheus/metrics backend?** Loki's `count_over_time` covers
  every current visualization. If future dashboards need true time
  series (histograms, quantiles), Mimir can be added later without
  breaking anything.
- **Why anonymous admin access in Grafana?** Deliberate — this stack is
  local-only dev. The README flags what to change if exposing beyond
  localhost.

### Added (tests)

- **`tests/factory-obs.bats`** (new) — 32 tests. Verifies script syntax
  + help + error paths (no Docker required), compose file structure
  (services / images / dependencies / volume mounts / ports), collector
  config (receivers / exporters / pipelines), Grafana provisioning YAML,
  and dashboard JSON (parseable, stable UID, all panels reference Loki,
  every target has an expr).

  End-to-end Docker runs are intentionally NOT tested in CI — BATS
  environments often don't have Docker and starting real containers in
  tests is flaky. Full validation is per the README's "testing without
  docker" section.

- 1006 tests across 32 suites, 0 failures.

### Docs

- **`docs/guide/observability.md`** — new "factory-obs + Docker stack"
  section describing the opt-in flow; roadmap marks Phase 5 shipped.

### What's next

- **Phase 6** — Session replay from the event log, agent SLO tracking,
  pipeline flame graphs. Some parts (replay) can layer on top of
  `factory-query` without Docker; others (flame graphs) benefit from
  Tempo, which could be added to the stack as a future release.

## 0.65.0 — Observability Phase 4: /factory-dashboard slash command

Live pipeline dashboard as a single command. Combines STATE.md, wave-state.yaml,
and the event log into one markdown view.

### Added

- **`bin/factory-dashboard`** — shell script that produces a markdown
  pipeline dashboard. Sections:
  - Project metadata (phase, mode, status, current step, STATE.md size
    with >200/>500-line warnings)
  - Waves table (each wave with gate status glyph, merged/total stories,
    next-gate marker)
  - Recent activity (factory-query stats + top-5 block reasons for the
    lookback window)
  - Pending wave gates (scraped from the most recent log's
    `pending_wave_gate_at_session_end` events)
  - Health checks (✓/✗ for each data source)

  Every section handles "missing" gracefully — no STATE.md, no
  wave-state.yaml, no event log, missing `python3` (falls back cleanly)
  all produce clean notices rather than errors.

- **`skills/factory-dashboard/SKILL.md`** + **`commands/factory-dashboard.md`**
  — slash command `/vsdd-factory:factory-dashboard` invokes the script.

### Naming

Chose `factory-dashboard` to avoid collision with the pre-existing
`factory-health` skill (which validates the `.factory/` worktree
structure — different purpose). The two complement each other:
- `/vsdd-factory:factory-health` — is the worktree set up correctly?
- `/vsdd-factory:factory-dashboard` — what's the pipeline doing right now?

### Flags

- `--factory PATH` — point at an alternate `.factory/` (e.g., a second
  project opened in the same session).
- `--days N` — change the event-log lookback window (default 7).

### Added (tests)

- **`tests/factory-dashboard.bats`** (new) — 16 tests covering:
  structural checks, empty state, STATE-only state, wave-state parsing
  (including malformed YAML graceful degradation), event integration,
  health-check glyphs, size-threshold warnings, and both flags.
- 974 tests across 31 suites, 0 failures.

### Docs

- **`docs/guide/observability.md`** — new section on
  `/factory-dashboard` with usage examples and a note clarifying its
  distinction from `/factory-health`; roadmap marks Phase 4 shipped.

### What's next

- **Phase 5** — Opt-in Docker observability stack (Grafana LGTM
  single-container) with preconfigured dashboards.
- **Phase 6** — Session replay, agent SLO tracking, pipeline flame graphs.

## 0.64.0 — Observability Phase 3: factory-query + factory-report CLIs

First shipped tooling on top of the event log. Two new binaries query
and summarize the structured events accumulated by instrumented hooks.

### Added

- **`bin/factory-query`** — canned queries against the event log. Six
  subcommands:
  - `top [--days N] [--limit N] [--tsv]` — top block reasons and hooks.
  - `recent [--limit N] [--severity warn|block|any] [--type T] [--tsv]`
    — latest events with optional filters. Action events display as
    `severity=action` to distinguish from block/warn.
  - `grep <reason_code>` — emit matching events as JSONL.
  - `hooks [--days N] [--tsv]` — block counts per hook.
  - `stats [--days N]` — aggregate: total / blocks (hard/warn) / actions /
    unique reasons / unique hooks / log file count / first & last dates.
  - `reasons [--days N] [--tsv]` — every unique (type, severity, hook,
    reason) combination with counts.

  All subcommands default to human-readable aligned tables; `--tsv`
  produces pipe-friendly output. `--days N` filters by log file date
  stamp (portable — avoids `date -d` / `date -v` syntax differences).

- **`bin/factory-report`** — markdown summaries. Three subcommands:
  - `daily [--date YYYY-MM-DD]` — single-day summary (default: today).
  - `weekly [--end YYYY-MM-DD]` — trailing 7-day summary ending at today
    (or explicit end date).
  - `range --from YYYY-MM-DD --to YYYY-MM-DD` — arbitrary date range.

  Each report contains: summary totals, top block reasons table, hook
  activity table, wave merges table (if any `wave_merge_recorded` events
  in range), and session-end gate warnings table (if any). Output is
  clean markdown — paste directly into PRs or Slack, or pipe through
  `glow`/`mdcat` for rendered terminal output.

### Added (tests)

- **`tests/factory-query.bats`** (new) — 21 tests. Covers all
  subcommands, flag combinations, empty log dir, missing log dir,
  date filtering, TSV format, and action-type severity labeling.
- **`tests/factory-report.bats`** (new) — 17 tests. Covers daily /
  weekly / range variants, markdown structure, all section types,
  empty states, and argument validation.
- 958 tests across 30 suites, 0 failures.

### Docs

- **`docs/guide/observability.md`** — new "Querying logs" section with
  examples for both CLIs; roadmap marks Phase 3 shipped.

### Notes

- Both CLIs are read-only. They never write to the log or to `.factory/`.
- Both gracefully handle missing log dir, no events, and unparseable
  events (jq silently skips malformed lines).
- No new external dependencies beyond `jq` (already required by hooks).

### What's next

- **Phase 4** — `/factory-health` slash command: live pipeline dashboard
  from `STATE.md` + `wave-state.yaml` + recent events.
- **Phase 5** — Opt-in Docker observability stack (Grafana LGTM
  single-container) with preconfigured dashboards.
- **Phase 6** — Session replay, agent SLO tracking, pipeline flame graphs.

## 0.63.0 — Observability Phase 2e: instrument SubagentStop + Stop hooks (Phase 2 COMPLETE)

**Phase 2 of the observability plan is complete.** Every hook decision
point in vsdd-factory that carries a meaningful structured signal now
emits a `hook.block` or `hook.action` event. Registry: 70 reason codes
across 30 instrumented hooks.

### New: `hook.action` event type

Introduced for passive state-change signals that aren't anomalies.
`update-wave-state-on-merge` emits `type=hook.action` when a story merge
is recorded — the event is telemetry, not a caught issue. Dashboards
that count "blocks" filter `type=hook.block`; dashboards that visualize
pipeline activity include both types.

### Changed

- **`handoff-validator.sh`** (SubagentStop) — two reasons,
  `severity=warn`: `subagent_empty_result` (zero non-ws chars) and
  `subagent_truncated_result` (<40 non-ws chars; event carries `result_len`).
- **`pr-manager-completion-guard.sh`** (SubagentStop) — reason
  `pr_manager_incomplete_lifecycle`. Event carries `step_count`, `last_step`,
  `next_step` — aggregate to find where pr-manager most often exits early.
- **`update-wave-state-on-merge.sh`** (SubagentStop) — `type=hook.action`
  with reason `wave_merge_recorded`. Event carries `story_id`, `wave`,
  `total` (stories in wave), `merged` (merged so far), `gate_transitioned`
  (bool: whether this merge flipped `gate_status` to `pending`). Python
  body now writes structured key=value lines to stdout which the shell
  forwards to `emit-event`, keeping emission in bash.
- **`validate-pr-review-posted.sh`** (SubagentStop) — reason
  `pr_review_not_posted`.
- **`warn-pending-wave-gate.sh`** (Stop) — reason
  `pending_wave_gate_at_session_end`, `severity=warn`. Event carries
  `pending_waves` (comma-separated).

### Not instrumented (intentional)

- **`session-learning.sh`** (Stop) — passive append-only hook. Writes a
  timestamp marker to `sidecar-learning.md` every session end with no
  anomaly detection logic. Nothing structured to emit. Documented in
  observability registry.

### Added (tests)

- **`tests/stop-hooks-emission.bats`** (new) — 12 tests covering all
  five instrumented stop-family hooks, the `hook.action` event type,
  severity=warn cases, and the standard `VSDD_TELEMETRY=off` regression.
- 920 tests across 28 suites, 0 failures.

### Docs

- **`docs/guide/observability.md`** — registry grown from 64 to 70 reason
  codes; added new "SubagentStop and Stop hooks" section; documented
  `hook.action` event type alongside `hook.block`; Phase 2 roadmap fully
  marked shipped across all sub-phases (2a → 2e).
- **`docs/guide/hooks-reference.md`** — added rows for four previously
  uncounted SubagentStop hooks; Instrumented column ticked for all five.

### Phase 2 summary

| Sub-phase | Hooks | New codes | Release |
|-----------|-------|-----------|---------|
| 1 (foundation) | — | — | 0.56.0 |
| 2a (PreToolUse Bash) | 4 | 35 | 0.57.0 |
| 2b (PreToolUse Edit\|Write) | 5 | 6 | 0.58.0 |
| 2c (PreToolUse Agent) | 2 | 2 | 0.59.0 |
| 2d.1 (Policy validators) | 4 | 4 | 0.60.0 |
| 2d.2 (Structural validators) | 7 | 7 | 0.61.0 |
| 2d.3 (Workflow validators) | 10 | 10 | 0.62.0 |
| 2e (SubagentStop + Stop) | 5 | 6 | 0.63.0 |
| **Total** | **37 emission sites across 30 hooks** | **70 codes** | — |

Two hooks remain intentionally uninstrumented: `check-factory-commit.sh`
(advisory-only hint, no anomaly detection) and
`validate-index-self-reference.sh` + `session-learning.sh` (pure stderr
advisories / passive appends with no structured signal).

### What's next

- **Phase 3** — `bin/factory-query` CLI with canned queries against the
  event log; `bin/factory-report` daily/weekly/per-cycle summaries.
- **Phase 4** — `/factory-health` slash command (text dashboard from
  STATE.md + wave-state.yaml + recent events).
- **Phase 5** — Opt-in Docker observability stack (OTel Collector +
  Grafana LGTM single-container) with preconfigured dashboards.
- **Phase 6** — Session replay, agent SLO tracking, pipeline flame graphs.

## 0.62.0 — Observability Phase 2d.3: instrument workflow validators (PostToolUse complete)

Final sub-release of Phase 2d. Instruments the remaining 10 PostToolUse
validators. **21 of 22 PostToolUse hooks now emit structured events.**
Registry has grown to 64 reason codes.

### Changed

- **`purity-check.sh`** — reason `pure_core_boundary_violation`,
  `severity=warn`. Event carries `patterns` field (space-joined list of
  matched side-effect idioms) for aggregating "which stdlib boundaries
  get crossed most often."
- **`validate-input-hash.sh`** — reason `input_hash_invalid_format`
  (format-block path only; advisory drift warnings remain stderr-only).
  Event carries `stored_hash`, `hash_len`, and `issue` (`length`|`chars`).
- **`validate-novelty-assessment.sh`** — reason `novelty_assessment_incomplete`.
- **`convergence-tracker.sh`** — reason `convergence_rule_violation`
  (premature CONVERGENCE_REACHED path). Event carries `verdict` and
  `novelty_score` for trend aggregation.
- **`validate-anchor-capabilities-union.sh`** — reason
  `anchor_capabilities_mismatch`. Event carries `expected` and `actual`
  so mis-derivations are visible in one jq row.
- **`validate-demo-evidence-story-scoped.sh`** — reason
  `demo_evidence_not_story_scoped`.
- **`validate-pr-description-completeness.sh`** — reason `pr_description_incomplete`.
- **`validate-wave-gate-completeness.sh`** — reason `wave_gate_incomplete`.
- **`validate-factory-path-root.sh`** — reason `factory_path_worktree_relative`.
  Event carries `worktree` for "which stories most commonly trip this."
- **`regression-gate.sh`** — reason `regression_gate_pass_to_fail`,
  `severity=warn`. First emission on a Bash-matcher PostToolUse hook;
  previous PostToolUse emissions were all Edit|Write. Hook never blocks
  (it's pure telemetry) — the event is the whole point.

### Added (tests)

- **`tests/workflow-validators-emission.bats`** (new) — 13 tests covering
  per-hook emission, severity=warn variants, `pass→pass` no-op case,
  and the standard `VSDD_TELEMETRY=off` regression. 908 tests across 27
  suites, 0 failures.

### Docs

- **`docs/guide/observability.md`** — registry grown from 54 to 64
  reason codes; Phase 2d fully marked shipped across 2d.1/2d.2/2d.3.
- **`docs/guide/hooks-reference.md`** — Instrumented column filled for
  all 10 hooks; added rows for `validate-anchor-capabilities-union.sh`,
  `validate-demo-evidence-story-scoped.sh`,
  `validate-pr-description-completeness.sh`,
  `validate-wave-gate-completeness.sh`, and
  `validate-factory-path-root.sh` (all previously missing from the
  summary table).

### Phase 2d status

PostToolUse instrumentation is complete. The factory's hot path — every
time an agent writes a spec, story, review, PR artifact, or STATE.md —
now produces a structured audit event when a validator fires.

### Not yet instrumented

- Phase 2e: 6 SubagentStop + Stop hooks (final pass before the tooling
  / dashboard work begins).

## 0.61.0 — Observability Phase 2d.2: instrument structural validators

Instruments 7 of 8 structural PostToolUse validators. Introduces a
`severity` field on emitted events for advisory-level hooks that exit 1
rather than 2 (so dashboards can distinguish hard blocks from warnings).

### Changed

- **`validate-template-compliance.sh`** — reason `template_noncompliant`.
  Event carries `template` (template basename) and `missing_keys` for
  aggregating "which fields do agents forget most often."
- **`validate-finding-format.sh`** — reason `finding_id_legacy_format`.
- **`validate-table-cell-count.sh`** — reason `table_cell_count_mismatch`.
- **`validate-changelog-monotonicity.sh`** — reason `changelog_not_monotonic`.
- **`validate-state-size.sh`** — reason `state_bloat`. Event carries
  `line_count` and `limit` to track how far over the bound the file grew.
- **`validate-state-pin-freshness.sh`** — reason `state_version_pin_drift`.
- **`validate-state-index-status-coherence.sh`** — reason
  `state_index_status_drift`, **`severity=warn`** (hook exits 1, not 2).
  First use of the severity field — lets dashboards filter warn-vs-block.

### Not instrumented (intentional)

- **`validate-index-self-reference.sh`** — pure advisory: always exits 0,
  emits stderr only, doesn't flag a structured anomaly. Nothing
  machine-actionable to emit. Documented in the registry.

### Added (tests)

- **`tests/structural-validators-emission.bats`** (new) — 13 tests.
  895 tests across 26 suites, 0 failures.

### Docs

- **`docs/guide/observability.md`** — registry grown from 47 to 54 codes;
  roadmap marks 2d.2 shipped.
- **`docs/guide/hooks-reference.md`** — Instrumented column ticked for
  all 7 structural validators (6 exit-2 + 1 exit-1 warn); added row for
  `validate-state-index-status-coherence.sh` (was previously missing).

### Not yet instrumented

- Phase 2d.3: 10 workflow/specialized validators (purity-check, input-hash,
  novelty-assessment, convergence-tracker, anchor-capabilities-union,
  demo-evidence-story-scoped, pr-description-completeness,
  wave-gate-completeness, factory-path-root, regression-gate).
- Phase 2e: 6 SubagentStop + Stop hooks.

## 0.60.0 — Observability Phase 2d.1: instrument policy validators

Starts the PostToolUse validator instrumentation pass. Phase 2d is
sub-split into three releases (policy, structural, workflow) to keep
each diff bisectable.

### Changed

- **`validate-subsystem-names.sh`** (Policy 6) — reason
  `policy6_subsystem_name_mismatch`.
- **`validate-bc-title.sh`** (Policy 7) — reason `policy7_bc_title_mismatch`.
  Event carries `bc_id`, `h1_title`, `index_title` — an immediate diff
  of which title drifted from which authoritative source.
- **`validate-story-bc-sync.sh`** (Policy 8) — reason
  `policy8_bc_array_desync`.
- **`validate-vp-consistency.sh`** (Policy 9) — reason
  `policy9_vp_inconsistency`.

Each event carries `matcher=PostToolUse` and `file_path=<path to the
edited file>`.

### Added (tests)

- **`tests/policy-validators-emission.bats`** (new) — 12 tests covering
  per-hook emission paths, the standard failure-tolerance regressions,
  and three "clean scenario emits no event" no-op cases. 882 tests
  across 25 suites, 0 failures.

### Docs

- **`docs/guide/observability.md`** — registry grown from 43 to 47 reason
  codes; roadmap split Phase 2d into 2d.1/2d.2/2d.3 sub-phases.
- **`docs/guide/hooks-reference.md`** — Instrumented column ticked for
  all 4 policy validators.

### Not yet instrumented

- Phase 2d.2: 8 structural validators (template-compliance, finding-format,
  table-cell-count, changelog-monotonicity, state-size, state-pin-freshness,
  state-index-status-coherence, index-self-reference).
- Phase 2d.3: 10 workflow/specialized validators (purity-check, input-hash,
  novelty-assessment, convergence-tracker, anchor-capabilities-union,
  demo-evidence-story-scoped, pr-description-completeness,
  wave-gate-completeness, factory-path-root, regression-gate).
- Phase 2e: 6 SubagentStop + Stop hooks.

## 0.59.0 — Observability Phase 2c: instrument PreToolUse Agent guards

Instruments the two hooks that govern subagent dispatch. Adds 2 reason
codes. Both hooks now emit rich structured context that makes Wave gate
and PR merge failures diagnosable without scrolling logs.

### Changed

- **`validate-wave-gate-prerequisite.sh`** — reason
  `wave_gate_prerequisite_not_passed`. Event carries `subagent`,
  `story_id`, `target_wave`, `blocking_wave`, `blocking_status`. Makes
  it trivial to aggregate which wave gates are blocking which
  downstream stories.
- **`validate-pr-merge-prerequisites.sh`** — reason
  `pr_merge_evidence_missing`. Event carries `story_id`, `delivery_dir`,
  and a comma-separated `missing` field listing which evidence files
  are absent (`pr-description.md`, `pr-review.md`, `security-review.md`).
  The unrelated advisory warning (directory entirely absent) still
  fires unchanged and does not emit an event.

### Added (tests)

- **`tests/agent-guards-emission.bats`** (new) — 11 tests covering
  per-hook emission paths and the standard failure-tolerance regressions
  (`CLAUDE_PLUGIN_ROOT` unset, path broken, `VSDD_TELEMETRY=off`).
  Also tests the "passed gate" and "complete delivery" no-op cases.
- 870 tests across 24 suites, 0 failures.

### Docs

- **`docs/guide/observability.md`** — registry grown from 41 to 43
  reason codes; Phase 2c marked shipped in the roadmap.
- **`docs/guide/hooks-reference.md`** — added rows for both Agent-matcher
  hooks (previously uncounted); Instrumented column ticked.

### Not yet instrumented

- Phase 2d: 21 PostToolUse validators (bulk)
- Phase 2e: 6 SubagentStop + Stop hooks

## 0.58.0 — Observability Phase 2b: instrument PreToolUse Edit|Write guards

Continues the instrumentation pass. Five more hooks now emit structured
`hook.block` events. Adds 6 reason codes to the registry. Docs updated
inline per the Tier-2 cadence.

### Changed

- **`brownfield-discipline.sh`** — reason `reference_readonly`.
- **`protect-vp.sh`** — reason `vp_green_immutable`. Emission happens
  inside `emit_deny()`; the existing JSON-envelope `permissionDecision`
  response format is preserved (this hook emits the envelope instead of
  exiting 2).
- **`protect-bc.sh`** — reason `bc_green_immutable`. Same pattern.
- **`red-gate.sh`** — reason `red_gate_strict_violation`.
- **`factory-branch-guard.sh`** — reasons `factory_not_worktree` and
  `factory_wrong_branch`. The second event includes `current_branch` and
  `expected_branch` as extra fields to aid triage.

Each event carries `type=hook.block`, `hook=<name>`, `matcher=<Edit|Write>`
(taken from `.tool_name` in the input so emissions distinguish Edit vs.
Write), `reason=<code>`, `file_path=<path>`.

### Added (tests)

- **`tests/edit-guards-emission.bats`** (new) — 17 tests covering per-hook
  emission paths and the "still blocks when emit-event is broken"
  regressions (`CLAUDE_PLUGIN_ROOT` unset, path broken, `VSDD_TELEMETRY=off`).
- 859 tests across 23 suites, 0 failures.

### Docs

- **`docs/guide/observability.md`** — reason-code registry grown from 35
  to 41 codes; Phase 2b marked shipped in the roadmap.
- **`docs/guide/hooks-reference.md`** — Instrumented column for all 5
  Edit|Write guards ticked (9 of 22 hooks in the summary table now ticked;
  remaining 13 pending Phases 2c–2e).

### Not yet instrumented

- Phase 2c: `validate-wave-gate-prerequisite.sh`, `validate-pr-merge-prerequisites.sh`
- Phase 2d: 21 PostToolUse validators
- Phase 2e: `handoff-validator.sh`, `pr-manager-completion-guard.sh`,
  `update-wave-state-on-merge.sh`, `validate-pr-review-posted.sh`,
  `session-learning.sh`, `warn-pending-wave-gate.sh`

## 0.57.1 — Observability docs (docs-only patch)

Documentation catch-up for the observability work shipped in 0.56.0 and
0.57.0. No code or behavior changes.

### Added

- **`docs/guide/observability.md`** (new) — single-page user + developer
  reference. Covers: env-var controls, log location and rotation, event
  schema, `jq` query recipes, the full reason-code registry (35 codes as
  of 0.57.0), "how to instrument your own hook" recipe, safety
  guarantees, and the Phase 2-6 roadmap.

### Changed

- **`docs/guide/hooks-reference.md`** — added `protect-secrets.sh` row
  (missing since 0.55.0), added `block-ai-attribution.sh` row, added new
  "Instrumented" column showing which hooks emit to the event log (4
  ticked, 18 pending), added pointer to observability guide. A broader
  audit to reconcile the hook count with the current plugin is deferred
  to an end-of-Phase-2 cleanup release.
- **`docs/guide/configuration.md`** — expanded `destructive-command-guard`
  description with all v0.55.0 additions, added `protect-secrets` rows
  under both Bash and Read matcher sections, added `block-ai-attribution`,
  added new "Environment variables" section documenting `VSDD_TELEMETRY`,
  `VSDD_LOG_DIR`, and `CLAUDE_PLUGIN_ROOT`.
- **`README.md`** — added `Observability` row to the user guide table;
  retitled Hooks Reference description to mention instrumentation.

## 0.57.0 — Observability Phase 2a: instrument PreToolUse Bash guards

First use of the `emit-event` helper shipped in 0.56.0. Four PreToolUse
Bash guards now emit a structured `hook.block` event at every decision
point, tagged with a stable `reason` code for aggregation and dashboards.

### Changed

- **`destructive-command-guard.sh`** — all 27 `block()` call sites now
  carry a reason code; `block()` emits a `hook.block` event before exiting.
  Reason codes: `catastrophic_root`, `protected_path_delete`, `sot_delete`,
  `sot_clobber_redirect`, `sot_truncate_colon`, `sot_truncate_cmd`,
  `sot_clobber_cpnull`, `find_delete_protected`, `git_reset_hard`,
  `git_clean_force`, `git_checkout_dot`, `git_restore_dot`,
  `git_stash_discard`, `git_branch_d_protected`, `git_filter_history`,
  `git_reflog_expire`, `git_gc_prune_now`, `git_worktree_force`,
  `git_no_verify`, `git_no_gpg_sign`, `git_rm_protected`,
  `gh_repo_delete`, `gh_release_delete`, `gh_pr_close`, `gh_issue_delete`,
  `rce_pipe_to_shell`, `recursive_permission_change`.
- **`protect-secrets.sh`** — 6 reason codes: `env_file_read_direct`
  (Read tool), `env_file_read_shell` (cat/less/grep/etc. on .env),
  `env_file_copy` (cp/mv of real .env source), `env_file_archive`
  (tar/zip of .env), `secret_var_echo`, `secret_var_grep`.
- **`verify-git-push.sh`** — refactored inline `echo; exit 2` sites into a
  shared `block()` function; 2 reason codes: `git_push_force`,
  `git_push_protected`.
- **`block-ai-attribution.sh`** — same refactor; 2 codes:
  `ai_attribution_coauthored`, `ai_attribution_generated`.

Every event carries `type=hook.block`, `hook=<name>`, `matcher=<Bash|Read>`,
`reason=<code>`, `command=<original>`, plus the auto-injected `ts` and
`schema_version` from `bin/emit-event`.

### Safety guarantees (preserved from 0.56.0)

- Each hook still blocks identically when `emit-event` is missing, broken,
  or disabled. The emission is always `2>/dev/null || true`.
- `VSDD_TELEMETRY=off` short-circuits emission at line 1 of `emit-event`.
- Missing `CLAUDE_PLUGIN_ROOT` → silent no-op, hook still blocks.

### Added (tests)

- 8 new emission tests in `tests/destructive-guard.bats` (105 total),
  including three "hook still blocks when emit-event is broken" regressions.
- 7 new emission tests in `tests/protect-secrets.bats` (62 total).
- 842 tests across 22 suites, 0 failures.

### Not yet instrumented (coming in later sub-phases)

- `check-factory-commit.sh` — advisory-only, emits `additionalContext`
  hints but never blocks. Will get a `hook.advice` event type in a later pass.
- Phase 2b: PreToolUse Edit|Write guards (5 hooks).
- Phase 2c: PreToolUse Agent guards (2 hooks).
- Phase 2d: PostToolUse validators (21 hooks).
- Phase 2e: SubagentStop + Stop hooks (6 hooks).

## 0.56.0 — Observability Phase 1: emit-event safety scaffold

First increment of the local-first observability plan. Ships the foundation
without instrumenting any hooks yet — the guarantee being proven in this
release is that **calling the emitter cannot break vsdd-factory** under any
failure mode.

### Added

- **`bin/emit-event`** — failure-tolerant structured event emitter. Writes
  one JSON event per invocation to `.factory/logs/events-YYYY-MM-DD.jsonl`.
  Hard guarantees:
  - Exits 0 on every path (missing `jq`, missing disk, read-only log dir,
    malformed args, disk full — all silent drops).
  - No stdout/stderr on success.
  - POSIX-portable date format (macOS, Linux, WSL, git-bash).
  - Atomic append (relies on POSIX PIPE_BUF guarantee, no `flock`).
  - `VSDD_TELEMETRY=off` kill switch (line-1 short-circuit).
  - `VSDD_LOG_DIR=<path>` override (default `.factory/logs`).
  - Args are `key=value` pairs; values may contain any characters (spaces,
    quotes, newlines, `=`); jq handles escaping.
  - Auto-adds `ts` (ISO-8601 w/ tz) and `schema_version` (integer) fields.

### Added (tests)

- `tests/emit-event.bats` — 35 tests covering structural checks, exit-code
  guarantees (garbage args, binary data, 10KB values, 50 field pairs), kill
  switch, graceful degradation (missing jq, readonly log dir, unwritable
  parent, auto-creation of deep dirs), emission correctness (valid JSON,
  ISO-8601 timestamp, schema version, field preservation, quote/backslash
  escaping, `=` in values, dotted keys stay flat), and append semantics.
- 827 tests across 22 suites, 0 failures.

### Notes

- Nothing currently calls `emit-event`. Phase 2 will instrument the 21
  existing hooks to emit events at decision points.
- Logs land under `.factory/logs/` which is already gitignored on `main`.
- Target shells: bash on macOS, Linux, WSL, and git-bash. Native PowerShell
  on Windows is out of scope for Phase 1.

## 0.55.0 — Harden destructive-command-guard + add protect-secrets hook

### Added

- **`destructive-command-guard.sh` — Tier 1 guards** (new):
  - Catastrophic root targets: `rm -rf /`, `/*`, `~`, `~/`, `$HOME`, `*`, `.*` (and flag-order variants)
  - Clobbering redirects to source-of-truth files: `> STATE.md`, `: > STATE.md`, `truncate -s 0`, `cp /dev/null`. `>>` (append) and `sed -i` remain allowed.
  - `find … -delete` and `find … -exec rm` on `.factory`, `src/`, `tests/`
  - `git stash drop` / `git stash clear`
  - `git branch -D main|master|develop`
  - `git filter-branch` / `git filter-repo`
- **`destructive-command-guard.sh` — Tier 2 guards** (new):
  - `--no-verify` on `git commit|merge|rebase|cherry-pick|am`, plus `--no-gpg-sign` on commit
  - `gh repo delete`, `gh release delete`, `gh pr close`, `gh issue delete`
  - `git reflog expire --expire=now` and `git gc --prune=now`
  - `git worktree remove --force` outside `.worktrees/`
  - Pipe-to-interpreter RCE pattern: `curl|wget|fetch ... | bash|sh|zsh|python|perl|ruby`
- **`destructive-command-guard.sh` — Tier 3 guards** (new):
  - Recursive `chmod -R` / `chown -R` / `--recursive` on `.factory`, `src/`, `tests/`, `.git/`
- **`protect-secrets.sh`** (new PreToolUse hook, matches Bash and Read):
  - Blocks `Read` of `.env`, `.env.*`, `.envrc` (allows `.env.example` / `.sample` / `.template`)
  - Blocks `cat|less|more|head|tail|bat|xxd|od|strings|grep|awk|sed` on real .env files
  - Blocks `cp|mv|rsync|scp` when the **source** is a real .env (allows template-bootstrap like `cp .env.example .env`)
  - Blocks `tar|zip` that archive real .env files
  - Blocks `echo|printf` of secret-shaped env vars (`$*_TOKEN`, `$*_SECRET`, `$*_PASSWORD`, `$*_API_KEY`, `$*_PRIVATE_KEY`, `$*_ACCESS_KEY`, `$*_CREDENTIAL`, `$*_AUTH`)
  - Blocks `env|printenv|set | grep` for secret-shaped names
  - Allows existence checks (`ls .env*`, `test -f .env`, `[ -f .env ]`) and sourcing (`source .env`, `. .env`)

### Fixed

- **`destructive-command-guard.sh` — bare `.factory` match** — `rm -rf .factory` (no trailing slash) previously slipped through the substring check; now caught via regex that handles end-of-command, slash, or separator boundary.
- **`destructive-command-guard.sh` — long-form recursive flag** — `rm --recursive` is now recognized alongside `-r`, `-R`, `-rf`, `-fr`, `-Rf`, `-fR`.

### Added (tests)

- 41 new tests in `tests/destructive-guard.bats` (catastrophic roots, SoT redirection, find-delete, git-stash-drop, git-branch-D, history rewriters, --no-verify, gh destructive, curl|bash, recursive chmod/chown) — 97 tests total in suite
- `tests/protect-secrets.bats` (new) — 55 tests covering Read + Bash paths, cp source-vs-destination semantics, secret env echo/grep
- `run-all.sh` wires the new protect-secrets suite
- 792 tests across 21 suites, 0 failures

## 0.54.0 — Unify red-gate-log path + factory path root guard

### Fixed

- **Red-gate-log path unified** — three conflicting paths (`.factory/stories/`, `.factory/phase-f4-implementation/`, `.factory/cycles/**/implementation/`) consolidated to `.factory/cycles/<cycle-id>/<story-id>/implementation/red-gate-log.md` across deliver-story skill, step files, phase-3 guide, per-story-delivery orchestrator, and phase-f4 skill.

### Added

- **`validate-factory-path-root.sh`** (PostToolUse on Write/Edit) — blocks `.factory/` writes that resolve inside `.worktrees/STORY-NNN/.factory/` instead of the project root. Catches agents using relative paths from inside story worktrees. Diagnostic shows the expected absolute path.
- 8 new BATS tests for factory path root validation
- 691 tests across 21 suites, 0 failures

## 0.53.0 — Wave gate completeness enforcement + GATE_CHECK telemetry

### Added

- **`validate-wave-gate-completeness.sh`** (PostToolUse on Write to wave-state.yaml) — blocks marking gate_status: passed unless the gate report contains evidence of all 6 gates (test suite, DTU validation, adversarial review, demo evidence, holdout evaluation, state update). Accepts both human-readable "Gate N" headings and structured GATE_CHECK telemetry lines.
- **GATE_CHECK telemetry** added to wave-gate skill — structured `GATE_CHECK: gate=N name=<name> status=<pass|fail|skip> note=<reason>` lines emitted after each gate, enabling mechanical validation by the completeness hook.
- 10 new BATS tests for gate completeness validation
- 682 tests across 21 suites, 0 failures

## 0.52.0 — Wave gate enforcement + PR lifecycle hooks + AI attribution blocker

### Added

- **Wave gate enforcement** (3 hooks):
  - `validate-wave-gate-prerequisite.sh` (PreToolUse on Agent) — blocks Wave N+1 worker dispatch if Wave N gate not passed/deferred
  - `update-wave-state-on-merge.sh` (SubagentStop on pr-manager) — auto-updates wave-state.yaml when stories merge, flips gate_status to pending when all wave stories merged
  - `warn-pending-wave-gate.sh` (Stop) — session-end reminder for pending gates
  - `wave-state-template.yaml` — structured wave lifecycle tracker template

- **PR lifecycle enforcement** (3 hooks):
  - `validate-pr-description-completeness.sh` (PostToolUse on Write) — blocks incomplete PR descriptions missing required sections or containing unresolved template placeholders
  - `validate-pr-review-posted.sh` (SubagentStop on pr-reviewer) — blocks if pr-reviewer didn't write pr-review.md or used `gh pr comment` instead of `gh pr review`
  - `validate-pr-merge-prerequisites.sh` (PreToolUse on Agent) — blocks merge dispatch if evidence trail (pr-description.md, pr-review.md, security-review.md) is missing

- **`block-ai-attribution.sh`** (PreToolUse on Bash) — blocks git commits containing Co-Authored-By: Claude/GPT/Gemini, "Generated with Claude Code", or noreply@anthropic.com patterns

- 80 new BATS tests across 2 test files (37 wave-gate + 43 PR lifecycle)
- 672 tests across 21 suites, 0 failures

## 0.51.0 — PR manager completion guard hook (FM4 detection)

### Added

- **`pr-manager-completion-guard.sh`** — SubagentStop hook that detects FM4 (pr-manager exiting before completing all 9 lifecycle steps). Blocks the stop via exit 2 and injects a continuation prompt with the next step hint. Accepts 8+ STEP_COMPLETE emissions as complete (allows one N/A skip). Passes through legitimate BLOCKED exits.
- 22 new BATS tests covering: scope filtering, full completion, partial exits at steps 0/1/3/5/7/8, BLOCKED passthrough, agent name matching variants, edge cases
- 592 tests across 19 suites, 0 failures

## 0.50.0 — Replace sessions_spawn with Agent tool + pr-manager hardening

### Fixed

- **pr-manager agent** — replaced all 13 `sessions_spawn` calls with `Agent(subagent_type=...)` syntax. Added COORDINATOR RULE meta-instruction, explicit continuation clauses after every sub-agent spawn (prevents premature exit on reviewer APPROVE), merge pre-authorization (`AUTHORIZE_MERGE=yes`), and STEP_COMPLETE instrumentation for all 9 steps.
- **sessions_spawn removed project-wide** — all references across 22 files (orchestrator, FACTORY.md, pr-reviewer, security-reviewer, code-delivery, 12 skill boilerplate lines, templates) replaced with Agent tool syntax. Zero `sessions_spawn` references remain.

### Changed

- FACTORY.md Sub-Agent Delegation Rule rewritten for Agent tool pattern
- Orchestrator delegation section updated with Agent tool examples
- agent-file-review audit rule updated to reflect pr-manager as legitimate Agent tool user

## 0.49.0 — Per-story demo evidence scoping (POL-010)

### Added

- **POL-010: `demo_evidence_story_scoped`** — all demo-recorder output must live under `docs/demo-evidence/<STORY-ID>/`, preventing evidence-report.md collisions across stories
- **`validate-demo-evidence-story-scoped.sh`** — PostToolUse lint hook that blocks flat-level file creation at `docs/demo-evidence/*.md`
- Policy entry added to `policies-template.yaml` (id: 10)

### Changed

- **demo-recorder agent** — output root changed from `docs/demo-evidence/` to `docs/demo-evidence/<STORY-ID>/`, constraint added forbidding flat-level files
- **per-story-delivery orchestrator** — task prompt updated with `<STORY-ID>` subfolder and collision-prevention explanation
- **deliver-story skill + steps** — all path references updated to per-story scoped
- **code-delivery skill** — paths, gates, and tree diagram updated
- **pr-manager, pr-reviewer agents** — evidence gate paths updated
- **record-demo skill** — output paths updated to per-story scoped
- **demo-recording skill** — internal factory paths unchanged (`.factory/demo-evidence/`)
- **Templates** — demo-tape-template.tape, demo-playwright-template.spec.ts output paths updated with `{STORY_ID}` placeholder
- **Workflows** — greenfield.lobster, code-delivery.lobster commit paths updated
- **Docs** — phase-3-tdd-delivery.md, FACTORY.md, configuration.md, factory-protocol.md updated
- Removed Tally MCP dependency — all 27 references across 18 files removed. Traceability chain simplified: `BC → VP → test → src → ADV → KANI` (TALLY-xxx link was redundant)

## 0.48.0 — Semantic drift lint hooks

### Added

- **`validate-state-index-status-coherence.sh`** — PostToolUse hook detecting drift between STATE.md `convergence_status:` frontmatter and cycle INDEX.md `**Status:**` lines. Normalizes underscores/hyphens and case for comparison, trims trailing descriptions after em-dash. Warning only (exit 1) — state transitions often land at different commits within the same burst.
- **`validate-anchor-capabilities-union.sh`** — PostToolUse hook enforcing the invariant that a story's `anchor_capabilities:` must equal the sorted union of `capability:` fields across all referenced BCs. Handles dual-anchor BCs with CSV capabilities, warns (but doesn't block) on missing BC files. Block on mismatch (exit 2) with BC→CAP mapping diagnostic.
- 22 new BATS tests covering both hooks (pass, fail, edge cases, wiring)

## 0.47.0 — Glob/directory expansion for input-hash + docs update

### Added

- **Glob/directory expansion** in `compute-input-hash` — artifacts can now use `inputs: [behavioral-contracts/**]` (recursive glob) or `inputs: [domain-spec/]` (directory) instead of listing every file. Expansion uses `find` + `LC_ALL=C sort` for deterministic ordering.
- 4 new glob tests (directory expansion, ** wildcard, deterministic hash, missing glob directory)

### Changed

- README updated: 116 skills, 108 commands, 26 hooks, 99 templates, 538 tests across 17 suites. All per-suite test counts updated. Contributing section: 9 Iron Laws, 80 Red Flags.
- hooks-reference.md: updated from 19 to 26 hooks with 8 new entries
- id-reference.md: ADV producer "Phase 1d/2/3/5", FIX producer "Phase 5/6/7", enforcement table expanded with 8 new hooks
- 538 tests across 17 suites

## 0.46.1 — Strip .factory/ prefix from input paths

### Fixed

- **compute-input-hash** — inputs with `.factory/` prefix (e.g., `inputs: [.factory/specs/prd.md]`) resolved to double-nested paths that don't exist. Now strips the prefix when FACTORY_ROOT is known, falls back to original if stripping fails. 534 tests.

## 0.46.0 — Input resolution mode + partial hash safety

### Added

- **`compute-input-hash --resolve`** — per-file and batch mode to check if all inputs are resolvable, naming specific missing files
- **`--scan <dir> --resolve`** — batch find all artifacts with unresolvable inputs (TOTAL/RESOLVABLE/UNRESOLVABLE)

### Fixed

- **Refuse partial hashing** — binary no longer silently hashes incomplete input sets. Missing inputs produce PARTIAL warning; hash/update/print all refuse to proceed. Prevents false MATCH from two partial hashes agreeing.
- **Named missing files** — all warnings now list the specific missing input file names (was just a count)
- **Warn in all modes** — missing input warnings fire in check, update, and print modes (was print-only)

### Changed

- `check-input-drift` skill: Step 2 (Resolve inputs) is now MANDATORY before interpreting drift results. Documents common causes (not yet produced, renamed, wrong path, deleted) with fix actions. Steps renumbered 1-6.
- 532 tests across 17 suites

## 0.45.1 — Fix absolute vs relative path bug in compute-input-hash

### Fixed

- **compute-input-hash** — absolute paths to `--scan` produced 62 false-positive STALE results vs 2 correct with relative paths. Root cause: `FACTORY_ROOT` pattern match behaved differently with relative vs absolute file paths. Fix: canonicalize `FILE` to absolute via `cd+pwd` before resolution logic runs. Both paths now produce identical results.
- 2 regression tests added (path identity + different-cwd scan)
- 527 tests across 17 suites

## 0.45.0 — Cluster drift triage for input-hash scanning

### Added

- **Step 5: Triage cluster drift** in `check-input-drift` skill — recognizes patterns where multiple related artifacts drift simultaneously (all domain-spec shards, all BCs for a subsystem, PRD + supplements) and routes to producing agents for content review before hash bumping
- Dispatch table mapping 7 artifact types to producing agents (business-analyst, product-owner, architect, story-writer)
- Task template for dispatched agents with content review instructions
- Common Cluster Drift Patterns reference table (7 patterns)
- Warning gate: Step 4 `--update` on >3 files requires Step 5 triage first
- 6 new BATS tests — 526 tests across 17 suites

## 0.44.0 — Batch input-hash drift scanning

### Added

- **`compute-input-hash --scan <dir>`** — batch drift detection that walks all `.factory/**/*.md` artifacts in one command. Avoids Claude Code's multi-line shell auto-backgrounding issue that killed inline bash loops before output flushed.
- **`compute-input-hash --scan <dir> --update`** — batch remediation of all stale hashes
- Summary line output: `TOTAL=N MATCH=N STALE=N UNCOMPUTED=N NOINPUT=N UPDATED=N UPDATE_FAILED=N`
- 14 new BATS tests (input-hash-scan.bats) — 520 tests across 17 suites

### Changed

- `check-input-drift` skill updated: uses `--scan` instead of manual iteration, documents both batch and per-file modes
- All existing per-file invocations (`<file>`, `<file> --update`, `<file> --check`) unchanged

## 0.43.1 — Shellcheck gate + post-release fixes

### Added

- **Shellcheck gate test** in hook-robustness.bats — runs shellcheck on all hooks and bin scripts, catches SC2001/SC2012 before release (506 tests)

### Fixed

- `validate-index-self-reference.sh` — sed → parameter expansion (shellcheck SC2001)
- `validate-table-cell-count.sh` — sed → parameter expansion (shellcheck SC2001)

## 0.43.0 — Corpus lint hooks + comprehensive glossary

### Added

- **4 new corpus lint hooks** targeting recurring adversarial finding defect classes:
  - `validate-table-cell-count.sh` — blocks markdown table rows with mismatched pipe count (caught 134 BCs in one Prism pass)
  - `validate-changelog-monotonicity.sh` — blocks duplicate versions, ascending order, date inversions, frontmatter mismatch
  - `validate-state-pin-freshness.sh` — blocks STATE.md version pins that don't match actual artifact versions
  - `validate-index-self-reference.sh` — warns when INDEX/burst-log edited without current burst row
- Extended `validate-input-hash.sh` — now blocks non-7-char hashes and non-lowercase-hex characters (caught 23 files in one pass)
- 45 new BATS tests across corpus-lint.bats (35) and edge cases (10) — 505 total across 16 suites
- **Glossary expanded to 110 entries** — added all spec levels (L1-L4), all 8 phases (0-7), architecture concepts (Purity Boundary, Pure Core, Effectful Shell), convergence metrics (Confidence Score, Hallucination Fingerprinting, MVR, Satisfaction Score), operational concepts (Burst, Per-Story Delivery Flow, Model Tier), and verification tools (Kani, Semgrep)

### Fixed

- `validate-input-hash.sh` null handling — `null` is not a valid placeholder (warns), only `[live-state]` and `[pending-recompute]` silently skip
- 26 hooks total, 505 tests across 16 suites

## 0.42.0 — Iron Laws and Red Flags for all critical pipeline skills

### Added

- **5 new Iron Laws** with Red Flags tables for critical skills:
  - `decompose-stories`: NO STORY WITHOUT BC TRACEABILITY FIRST (8 flags)
  - `holdout-eval`: NO HOLDOUT EVALUATION WITHOUT INFORMATION ASYMMETRY FIRST (8 flags)
  - `convergence-check`: NO RELEASE WITHOUT ALL SEVEN DIMENSIONS CONVERGED (8 flags)
  - `formal-verify`: NO HARDENING SIGN-OFF WITHOUT ALL PROOF HARNESSES PASSING (8 flags)
  - `create-architecture`: NO ARCHITECTURE WITHOUT VERIFICATION FEASIBILITY ASSESSMENT (8 flags)
- Total: 9 Iron Laws, 80 Red Flag entries across the pipeline

### Fixed

- **Minimum pass count inconsistency** — adversarial-review SKILL.md and adversary.md both said "Minimum 2 passes", now correctly say "Minimum 3 clean passes" matching CONVERGENCE.md and all convergence hooks
- **Maximum pass escalation** — "Maximum 5" → "Maximum 10" matching convergence loop `max_iterations` in all lobster files

## 0.41.3 — Restore adversary agent model tier

### Fixed

- **adversary.md** — `model: adversary` (invalid tier) restored to `model: opus`

## 0.41.2 — Cycle file templates for state management

### Added

- **5 new templates** for cycle-scoped state files: `burst-log-template.md`, `convergence-trajectory-template.md`, `session-checkpoints-template.md`, `lessons-template.md`, `blocking-issues-resolved-template.md`
- Template references added to `compact-state` skill and `state-manager` agent (99 templates total)

## 0.41.1 — Post-release bugfixes

### Fixed

- **Command files** — `check-state-health.md` and `compact-state.md` had empty body text after frontmatter, causing `cache_control cannot be set for empty text blocks` API error when invoked
- **convergence-tracker.sh** — replaced `ls` with `find` to satisfy shellcheck SC2012

## 0.41.0 — Adversarial review hardening + state management + convergence enforcement

### Added

- **Per-story adversarial review** — full convergence loop in `code-delivery.lobster` after implementation, before demo recording. Scoped to story diff with information asymmetry walls
- **Phase 2 adversarial convergence loop** — added to `phase-2-story-decomposition.lobster` (was missing entirely)
- **Wave-level adversarial convergence** — upgraded from single-pass to full convergence loop in both `greenfield.lobster` and `feature.lobster`
- **`validate-novelty-assessment.sh` hook** (21st) — blocks adversarial review files missing the structured Novelty Assessment section (pass, novelty score, trajectory, verdict)
- **`convergence-tracker.sh` hook** (22nd) — lightweight convergence rule enforcement: trajectory monotonicity, minimum 3 clean passes, novelty ≤ 0.15 for CONVERGENCE_REACHED, zero-findings first pass warning
- **`validate-state-size.sh` hook** (20th) — warns at 200 lines, blocks at 500 lines (allows compaction writes that reduce size)
- **`check-state-health` skill + command** — 7-check diagnostic (existence, frontmatter, size, phase numbering, structure, content routing, convergence counter)
- **`compact-state` skill + command** — extracts historical content from bloated STATE.md to cycle-scoped files (burst-log, convergence-trajectory, session-checkpoints, lessons, blocking-issues-resolved)
- **Content routing rules** in state-manager agent — explicit routing tables and anti-patterns preventing STATE.md bloat
- **STATE.md health check** (check 8) wired into `factory-health` — catches bloat at every session start
- **CONVERGENCE.md Dimensions 6-7** — Visual Convergence (demo evidence) and Documentation Convergence (CHANGELOG, README, API docs)
- **Adversarial review template** updated with structured Novelty Assessment section (7 required fields)
- **State template v2.0** — Phase 0 included, size budget comment, Historical Content section, session checkpoint limit
- 68 new BATS tests across 3 new suites (state-health: 33, novelty-assessment: 18, convergence-tracker: 17)

### Fixed

- **Phase 1d step files** — rewrote 5 empty stubs into full alphabetic step files (a-e) with `_shared-context.md` containing Iron Law, Red Flags, convergence protocol, persistence protocol
- **Phase F5 step files** — enriched 8 thin files into full alphabetic step files (a-h) with `_shared-context.md` containing DF-025 walls, DF-013 multi-repo, holdout regression, security touchpoint
- **Orchestrator mandatory checks** — corrected all phase references (1d, 2, 3, 4, 5), added per-story and wave-level adversarial as mandatory steps
- **Stale phase references** swept across ~30 files: agents (7), docs (VSDD.md, FACTORY.md), templates (14), skills (4), lobster files (5), orchestrator sequences
- **CONVERGENCE.md autonomy config** — fixed phase numbering (3.5→4, 4→5, 5→6, 6→7)
- **adversary.md** — `model: opus` → `model: adversary`, "Phase 4" → "Phase 5"
- **Prompt template** renamed `phase-4-code-review.md` → `phase-5-code-review.md`
- **Template frontmatter** — 4 adversarial templates updated from `phase: 1d|2|4` to `1d|2|5`

### Changed

- 460 tests across 15 suites (was 392 across 12)
- 22 hooks (was 19)
- 116 skills (was 112), 108 commands (was 104)

## 0.40.0 — Full pipeline path verification + workflow mode documentation

### Added

- **docs/guide/pipeline-paths.md** — definitive routing reference documenting all 14 paths through the factory with step traces, gate criteria, Mermaid routing diagram, and transition logic
- **docs/guide/workflow-modes.md** — all 8 workflow modes with routing diagram, per-mode documentation, mode detection logic, greenfield→multi-repo and brownfield→multi-repo transition explanations
- **Greenfield→multi-repo transition** formalized as 4 lobster steps: topology-check, human-confirmation, repo-transition, state-migration
- **Brownfield→multi-repo auto hand-off** — `multi-repo-handoff-check` step detects if greenfield sub-workflow triggered multi-repo transition

### Fixed

- `wave-integration-gate` schema: `type: skill` → `type: compound` in greenfield + feature lobster (had no `skill:` path)
- Multi-repo `wave-0`/`wave-1` steps parameterized with `${repo.name}` and `${repo.mode}` (was hardcoded `api-server`/`frontend`)
- Brownfield `depends_on` on conditional steps: uses `wait_for_optional` for `semport-validation-gate` and `brownfield-design-system-approval`
- Maintenance description: updated from "9 sweep types" to "11 sweep types" (sweeps 10-11 were added but description not updated)
- Planning + multi-repo added to orchestrator Mode Detection and Reference Files table
- Mode-decision-guide: added multi-repo, maintenance, discovery to decision table + flowchart
- All "Phases 1-6" references updated to "Phases 1-7"

### Path Trace Results

14 paths traced. Zero broken file references. All `skill:` paths resolve, all `agent:` names resolve, all `sub_workflow:` references resolve.

## 0.39.0 — Workflow standardization + phase renumbering

### Changed

- **Phase renumbering**: Eliminated fractional Phase 3.5. Pipeline is now 8 phases (0-7): Codebase Ingestion, Spec Crystallization, Story Decomposition, TDD Implementation, Holdout Evaluation, Adversarial Refinement, Formal Hardening, Convergence.
- **Step decomposition**: All phases decomposed into step files with `_shared-context.md` and state-manager crash recovery. 30 step files across 5 work skills.
- **Three-layer architecture**: Top-level lobster → phase entry-point skill → phase sub-workflow lobster → step files.
- **Pure alphabetic step naming**: All step IDs use `step-a-`, `step-b-`, etc. No numeric or sub-step IDs.

### Added

- **8 phase entry-point skills** (`phase-0-codebase-ingestion` through `phase-7-convergence`) bridging top-level lobsters to phase sub-workflows
- **`phase-2-story-decomposition.lobster`** — previously missing phase sub-workflow
- **`rules/step-decomposition.md`** — comprehensive standard for phase numbering, step naming, workflow structure
- **8 structural BATS tests** for lobster path resolution, phase entry-point skills, no old numbering, shared-context, alphabetic naming
- Renamed `docs/guide/phase-4-adversarial-refinement.md` → `phase-5-adversarial-refinement.md`
- Renamed `docs/guide/phase-6-convergence-release.md` → `phase-7-convergence-release.md`

### Fixed

- **20 broken skill paths** in lobster workflow files (referenced non-existent phase skill directories)
- **~150 old phase number references** across 47 files
- `semport/SKILL.md` → `semport-analyze/SKILL.md` in brownfield.lobster
- README Mermaid diagram now shows all 8 phases
- Skills: 104 → 112, Workflows: 15 → 16, Rules: 8 → 9, Tests: 384 → 392

## 0.38.1 — Fix compute-input-hash path resolution

### Fixed

- **compute-input-hash** now searches `.factory/phase-0-ingestion/`, `.factory/stories/`, and `.factory/holdout-scenarios/` when resolving input paths. Previously only searched artifact dir, parent, specs/, and .factory/ root — causing silent partial hashes for artifacts referencing brownfield ingestion outputs, stories, or holdout scenarios.

### Added

- 3 new BATS tests for path resolution: phase-0-ingestion, stories, holdout-scenarios (Tests: 381 → 384)

## 0.38.0 — Orchestrator audit + workflow drift checks

### Added

- **Input-hash drift checks in lobster workflows** — phase-1, phase-3, phase-6 now include `input-hash-drift-check` step before gates/human approval
- **Orchestrator mandatory step** — input-hash drift check required at phase gates 1, 2, 3, 6
- **5 cross-cutting skills** added to orchestrator reference: `validate-template-compliance`, `conform-to-template`, `register-artifact`, `recover-state`, `factory-cycles-bootstrap`
- **validate-finding-format.sh** and **validate-input-hash.sh** detail sections in hooks-reference.md

### Changed

- README counts updated: 104 skills, 104 commands, 19 hooks, 94 templates, 5 bin helpers, 381 tests across 12 suites
- hooks-reference.md updated: 17 → 19 hooks with full detail sections
- configuration.md updated: 17 → 19 hooks in PostToolUse table

## 0.37.0 — Input-hash drift detection tooling

### Added

- **bin/compute-input-hash** — 7-char MD5 hash from `inputs:` files. `--update` writes to frontmatter, `--check` compares (exit 2 on drift).
- **validate-input-hash.sh** — PostToolUse hook warning on missing/stale input-hash. Advisory.
- **check-input-drift skill** — batch scan + report. Optional `--fix`.
- **22 BATS tests** for all three components.

### Changed

- Producer agents updated to compute input-hash after writing
- Hooks: 18 → 19, Tests: 359 → 380, Suites: 11 → 12, Skills: 103 → 105

## 0.36.0 — Strict ID enforcement + comprehensive ID reference

### Added

- **validate-finding-format.sh** — PostToolUse hook blocking legacy ADV-NNN, ADV-P[N]-NNN, and STORY-NNN-FIX formats. Only current formats accepted: `ADV-<CYCLE>-P[N]-[SEV]-NNN` and `FIX-P[N]-NNN`. 15 tests.
- **docs/guide/id-reference.md** — comprehensive reference documenting all 30 ID formats with scope rules, producer, registry, hook validation, and detailed descriptions for key IDs

### Changed

- **All migration graces removed.** Only current formats accepted:
  - `behavioral_contracts` (not `bcs`), `target_module` (not `crate`)
  - `document_type: domain-spec-index` (not `domain-spec-section`)
  - `SS-NN` IDs (not subsystem names)
  - Current table headers only (no legacy acceptance)
  - Legacy ADV/FIX ID formats removed from FACTORY.md
- **validate-story-bc-sync.sh** reads both `behavioral_contracts:` and `bcs:` functionally (finds data regardless of field name — template-compliance hook flags the wrong name separately)
- Hooks: 17 → 18, Tests: 342 → 359, Suites: 10 → 11

## 0.35.0 — Complete ID system formalization + Subsystem Registry

### Added

- **Subsystem Registry** in ARCH-INDEX template with formal SS-NN ID format, lifecycle documentation, and naming rules
- **31-entry ID Format Reference** in FACTORY.md (was 16) — every ID system in the plugin now formally documented
- **ID Scope Definitions** — Lifecycle (append-only), Cycle (resets), Local (scoped to parent)
- **Legacy ID Formats** migration table — ADV-NNN, STORY-NNN-FIX mapped to current formats
- **3 new templates:** holdout-scenario-index (HS-INDEX with WHS section), epic-index, fix
- **5 glossary terms:** Epic, Fix PR, Gap Register, ID Scope, Wave Holdout Scenario

### Changed

- **Subsystem references now use SS-NN IDs** (was human-readable names). BC `subsystem:` and story `subsystems:` fields hold SS-IDs, not names.
- **validate-subsystem-names.sh** hook matches SS-NN IDs against ARCH-INDEX registry. Error messages show `SS-01 (Core Engine)` pairs.
- **Language-agnostic terminology:** "Crate" → "module/package" across agents and templates. Story field `crate:` → `target_module:` (migration alias accepted).
- Templates: 124 → 127
- Policy 6 enforcement: validates SS-IDs, not names

## 0.34.0 — Template v1.1 schema update (Prism-validated improvements)

### Changed — Template Schema Updates

All changes are additive or header renames with migration acceptance. Existing artifacts remain valid.

- **behavioral-contract-template.md (v1.1):** Added `## Description` (required), `## Related BCs`, `## Architecture Anchors`, `## Story Anchor`, `## VP Anchors` (Recommended)
- **story-template.md (v1.1):** Added optional frontmatter: wave, crate, subsystems, estimated_days. UX Screens conditional. Table header simplifications with migration acceptance.
- **module-criticality-template.md (v1.1):** Added Module Inventory, Per-Module Risk Assessment, Dependency Graph, Implementation Priority, Cross-Cutting Concerns (Recommended), Anti-Patterns (Conditional)
- **verification-architecture-template.md (v1.1):** Sections allow inline OR reference-to-shard
- **L2-domain-spec-index-template.md (v1.1):** Clarified canonical document_type, strengthened sections: as REQUIRED

### Updated — Companion Changes

- product-owner and story-writer agents updated for v1.1 template guidance
- validate-template-compliance hook filters Recommended/Conditional sections

## 0.33.0 — Template compliance enforcement hook

### Added

- **validate-template-compliance.sh** — PostToolUse hook that automatically validates every Write to `.factory/**/*.md` against its corresponding template. Checks required frontmatter fields and H2 section headings. Resolves templates via `document_type` frontmatter or path-pattern fallback. Warning messages suggest `/vsdd-factory:conform-to-template` for fixes.
- **14 BATS tests** with 4 fixtures (compliant BC, non-compliant BC, non-compliant story, holdout with no frontmatter)

### Changed

- Hooks: 16 → 17
- Tests: 328 → 342 across 10 suites

### Context

Built in response to Prism template compliance audit showing 0% strict compliance across stories (76 FAIL) and holdout scenarios (8 FAIL). The hook catches these drift patterns at write time — agents see the warning immediately and can self-correct.

## 0.32.0 — Template compliance skills + hook integration audit

### Added

- **validate-template-compliance skill** — read-only audit checking artifact files against their templates at three levels: frontmatter fields, section headings, table column headers. Resolves templates via `document_type` frontmatter or file path patterns. Reports PASS/WARN/FAIL per file with aggregate summary.
- **conform-to-template skill** — remediation skill that fixes structural gaps by adding missing frontmatter fields, section headings, and `[TODO]` placeholders. Safety guarantees: never deletes content, always shows diff before applying, creates backup. Reports table/order mismatches for manual fix.
- **8 BATS tests** for both skills (structure, three-level check, mapping, safety guarantees, commands)

### Fixed

- **handoff-validator.sh:** Was reading `.result` / `.output` / `.tool_response` but Claude Code SubagentStop sends `last_assistant_message`. Hook was always seeing empty content. Fixed with correct field + legacy fallback.

### Changed

- Skills: 101 → 103, Commands: 101 → 103
- Tests: 319 → 328

### Hook Integration Audit Results

Verified all 16 hooks parse correct JSON fields per Claude Code documentation:
- Edit|Write hooks: `tool_input.file_path` — all 10 correct
- Bash hooks: `tool_input.command` — all 4 correct
- SubagentStop: `last_assistant_message` — fixed (was wrong)
- PostToolUse hooks correctly use exit 2 for prominent warnings (edits can't be undone)

## 0.31.0 — Template extraction + hook trigger fixes

### Added

- **12 new templates** extracted from inline format definitions:
  - HIGH: story-index, traceability-matrices, extraction-validation
  - MEDIUM: spec-drift-report, formal-verification, performance-report, implementation-readiness, design-drift, brief-validation, research-index
  - LOW: agent-file-review, consistency-validation-report
- **14 new BATS tests** for hook trigger edge cases:
  - check-factory-commit: 4 tests (was zero coverage)
  - red-gate: 3 absolute path tests
  - destructive-command-guard: 5 complex bash construct tests
  - verify-git-push: 2 edge case tests (--force-with-lease, -f at end)

### Fixed

- **verify-git-push:** `--force-with-lease` (safe force push) was incorrectly blocked because it substring-matched `--force`. Now allowed.
- **red-gate:** Absolute file paths from Claude Code tool_input never matched relative paths in `.red[]` array. Added PWD and git-root prefix stripping.

### Changed

- 13 agent/skill files updated to reference new templates instead of inline format definitions
- Templates: 112 → 124
- Tests: 305 → 319

## 0.30.2 — Generalize Policy 9 hook for multi-project portability

### Changed

- **validate-vp-consistency.sh Check (d):** Replaced hardcoded Kani/Proptest/Fuzz column detection with generic header-based discovery. Auto-detects ALL verification method columns from the Coverage by Module header row. Works for any tool names (Kani, CBMC, Hypothesis, fast-check, Stryker, etc.).
- **verification-coverage-matrix-template.md:** Method columns are now documented as project-specific with language-specific examples. Hook auto-detects — no configuration needed.
- Removed dependency on `.declared` temp file for Check (d) — uses `get_summary_total` directly with partial label matching.

## 0.30.1 — Fix Policy 9 hook defects + verification templates

### Fixed

- **validate-vp-consistency.sh Defect 1:** Multi-word Summary labels (e.g., "Integration test") triggered `set -u` abort on arithmetic comparison. All Policy 9 arithmetic enforcement (Checks c/d/e) was silently disabled. Fixed by normalizing labels to snake_case + defensive non-integer guard.
- **validate-vp-consistency.sh Defect 2:** EXIT trap clobbered non-zero exit codes to 0. Now preserves original rc through trap.
- **validate-vp-consistency.sh Defect 3:** Check (d) summed wrong columns (Criticality+Kani+Proptest, missing Fuzz). Replaced with header-detected column positions that find Kani/Proptest/Fuzz by name in the header row, handling any column layout.
- **Test fixtures** rebuilt from template format (was ad-hoc 5-column format that no project produces)

### Added

- **verification-coverage-matrix-template.md** — 7-column template with column definitions, arithmetic invariant rules, coverage gaps table, and domain invariant verification map
- **verification-architecture-template.md** — Provable Properties Catalog, P0/P1 lists, purity boundary, verification tooling tables
- **2 real-world regression tests** in policy9.bats (multi-word labels, fuzz column drift)
- **2 new test fixture sets** (policy-9-realworld, policy-9-fuzz-drift) matching template format

### Changed

- Architect agent references new templates for verification-coverage-matrix and verification-architecture sections
- Templates: 110 → 112

## 0.30.0 — Quality of life + comprehensive guardrails audit complete (Tier 5)

### Added

- **Policy reference matrix + violation playbook** (`docs/guide/policy-reference.md`) — single-page quick reference for all 9 policies with enforcement matrix (hook, criteria, agents) and per-policy step-by-step fix procedures
- **register-artifact skill** — automates INDEX file registration after creating BCs, VPs, stories, or holdout scenarios. Supports batch registration. 4 BATS tests.
- **recover-state skill** — reconstructs `.factory/STATE.md` from artifacts on disk when corrupted or missing. 9-step procedure with backup, artifact scanning, phase determination, user validation, and `--dry-run` option. 7 BATS tests.

### Changed

- Skills: 96 → 101, Commands: 96 → 101, Templates: 109 → 110
- Test count: 292 → 303 across 9 suites

### Audit Summary

This release completes the 5-tier comprehensive guardrails audit:
- **Tier 1 (v0.27.0):** 3 data safety hooks — destructive command guard, branch protection, factory branch guard
- **Tier 2 (v0.27.1):** 3 policy enforcement hooks — subsystem names, BC titles, story-BC sync
- **Tier 3 (v0.28.0):** All 33 agents standardized with Tool Access, Failure & Escalation, Remember, AGENT-SOUL.md
- **Tier 4 (v0.29.0):** 31 hook robustness + error contract tests, orchestrator FACTORY.md refs
- **Tier 5 (v0.30.0):** Policy reference docs, register-artifact skill, recover-state skill

Total impact: 16 hooks (was 11), 303 tests (was 133), 101 skills (was 96), all agents standardized.

## 0.29.0 — Testing gaps + orchestrator standardization (Tier 3-4)

### Added

- **FACTORY.md references** for all 9 orchestrator sequence files (brownfield, discovery, feature, greenfield, heartbeat, maintenance, multi-repo, per-story-delivery, steady-state)
- **hook-robustness.bats** — 31 tests verifying all 7 enforcement hooks handle: empty JSON, missing fields, nonexistent files, empty files, malformed frontmatter, and produce correct error contract (BLOCKED / POLICY N VIOLATION keywords)
- **BATS test** verifying all orchestrator files reference FACTORY.md

### Changed

- Test count: 260 → 292 across 9 suites
- README: updated test counts, suite counts, suite listings

## 0.28.0 — Agent standardization (Tier 3)

### Added

- **Tool Access sections** for 5 agents missing them: adversary (`read-only`), codebase-analyzer (`full`), holdout-evaluator (`restricted`), research-agent (`full`), validate-extraction (`full`)
- **Failure & Escalation sections** for 6 agents missing them (adversary, codebase-analyzer, holdout-evaluator, validate-extraction, implementer, plus research-agent already had one)
- **Remember + AGENT-SOUL.md closing** for all 5 newly standardized agents
- **4 structural completeness tests** — verify ALL 33 agents have: Tool Access, Failure & Escalation, Remember, AGENT-SOUL.md reference
- **5 profile tests** for newly documented agent profiles

### Changed

- Test count: 251 → 260 across 8 suites
- All 33 agents now have consistent structure: Tool Access, Failure & Escalation, Remember, AGENT-SOUL.md

## 0.27.1 — Policy enforcement hooks (Tier 2)

### Added

- **validate-subsystem-names.sh** (Policy 6) — PostToolUse hook verifying BC `subsystem:` and story `subsystems:` fields match ARCH-INDEX Subsystem Registry canonical names. Error messages list all valid names.
- **validate-bc-title.sh** (Policy 7) — PostToolUse hook verifying BC file H1 heading matches BC-INDEX title column. Shows both titles on mismatch.
- **validate-story-bc-sync.sh** (Policy 8) — PostToolUse hook verifying bidirectional BC completeness between story frontmatter `bcs:` array, body BC table, and AC trace annotations. Identifies specific missing BCs.
- **28 BATS tests** for all three hooks with shared fixtures (ARCH-INDEX, BC-INDEX, good/bad BC and story files)

### Changed

- Hook count: 13 → 16
- Test count: 223 → 251 across 8 suites
- hooks-reference.md, configuration.md: updated with all new hooks

### Analysis (Finding #6 conclusions)

Policies 1-5 remain gate-only (consistency-validator criteria). Research confirmed:
- Policy 1 (append_only_numbering): requires git history scan — too expensive for edit-time
- Policy 2 (lift_invariants_to_bcs): requires cross-file scan of all BCs — blocks intermediate states
- Policy 3 (state_manager_runs_last): temporal ordering — not a file-content check
- Policy 4 (semantic_anchoring_integrity): requires semantic judgment — only adversary can assess
- Policy 5 (creators_justify_anchors): can check presence but not correctness — adversary review is appropriate

## 0.27.0 — Data safety hooks (Tier 1)

### Added

- **destructive-command-guard.sh** — PreToolUse Bash hook blocking `rm -rf` on protected paths (.factory/, src/, tests/), `rm` on source-of-truth files (STATE.md, *-INDEX.md, prd.md), `git reset --hard`, `git clean -f`, `git checkout -- .`, `git restore .`, `git rm` on spec/story paths. Each block message includes the safe alternative. 46 BATS tests.
- **factory-branch-guard.sh** — PreToolUse Edit|Write hook blocking writes to `.factory/` when not mounted as git worktree on `factory-artifacts` branch. Also guards `.factory-project/` for multi-repo projects. Block messages include the exact recovery command. 6 BATS tests.
- **verify-git-push.sh** enhanced — now blocks direct push to protected branches (main, master, develop) in addition to force push. Block messages suggest the PR workflow. 10 BATS tests.

### Changed

- Hook count: 10 → 13
- Test count: 161 → 223 across 7 suites
- hooks-reference.md: complete rewrite covering all 13 hooks with detail sections
- configuration.md: updated hook tables with all new hooks
- README: updated hook count, test count, suite count, suite listings

## 0.26.0 — Policy registry, cycle management, scoped reviews

### Added

- **Policy registry** (`/vsdd-factory:policy-registry`) — declarative `.factory/policies.yaml` with init, list, validate, show commands. 9 baseline policies with `verification_steps` field providing step-by-step check procedures the adversary executes per-policy.
- **Policy add** (`/vsdd-factory:policy-add`) — register new governance policies mid-cycle with schema validation, sequential ID assignment, and verification steps
- **policies-template.yaml** — complete template with 9 baseline policies, verification steps, lint hook references, and scope declarations
- **Adversarial-review policy auto-loading** — orchestrator reads `policies.yaml` and injects full policy rubric (with verification steps) into every adversary dispatch, replacing manual copy-pasting
- **Cycle layout bootstrap** (`/vsdd-factory:factory-cycles-bootstrap`) — migrate from flat `specs/adversarial-review-pass-*.md` layout to cycle-keyed directories with `git mv` for history preservation
- **Scoped adversarial review** (`--scope` parameter) — `full` (default), `diff-from:<commit>` (focus on changed files), `paths:<pattern>` (target specific subsystems)
- **Cycle-prefixed finding IDs** — `ADV-<CYCLE>-P<PASS>-<SEV>-<SEQ>` format prevents ID collisions across multi-cycle projects

### Changed

- **adversarial-review** `disable-model-invocation` set to `false` — enables orchestrator-initiated invocation for autonomous convergence loops
- **Orchestrator** updated with policy rubric loading directive and new cross-cutting skills
- Finding ID format in 4 templates updated from `ADV-P[N]-NNN` to cycle-prefixed format
- README: test count 152 → 161, skills.bats 24 → 33

## 0.25.0 — Policy 9 lint hook + adversarial-review skill hardening

### Added

- **Policy 9 lint hook** (`hooks/validate-vp-consistency.sh`) — PostToolUse hook that validates VP-INDEX.md ↔ verification-architecture.md ↔ verification-coverage-matrix.md consistency on every edit. Checks: (a) every VP-INDEX VP appears in both arch docs, (b) per-tool summary totals match row counts, (c) coverage matrix Totals row matches data row sums, (d) no orphaned VP references in coverage matrix. Wired into hooks.json PostToolUse.
- **Architect source-of-truth invariants table** — consolidates ARCH-INDEX (subsystem names), BC H1 (titles), VP-INDEX (VP catalog) as the three authoritative sources with their downstream documents. Cross-links to validate-vp-consistency.sh as automated enforcer.
- **Adversarial-review post-adversary persistence** — explicit orchestrator step to capture adversary findings and dispatch state-manager to persist them to `.factory/cycles/<cycle>/adversarial-reviews/pass-<N>.md`. Prevents findings loss when adversary (read-only agent) returns chat text that disappears on session boundary.
- **Adversarial-review filename collision guard** — pre-flight check refuses to overwrite existing review files with different content. Points to cycle bootstrap skill for resolution. Warns about legacy flat-file layout.
- **11 BATS tests** for Policy 9 hook (3 fixture sets: green, canary with fuzz column drift, missing-VP)
- **3 BATS tests** for adversarial-review persistence + collision guard presence

### Changed

- README: test count 138 → 152, suite count 5 → 6
- run-all.sh: includes policy9.bats suite

## 0.24.1 — Policy 9: VP-INDEX source of truth

### Added

- **Policy 9: `vp_index_is_vp_catalog_source_of_truth`** — VP-INDEX.md is the authoritative VP enumeration; changes must propagate to verification-architecture.md and verification-coverage-matrix.md in the same burst
- **Architect agent:** VP-INDEX propagation obligation — must touch VP-INDEX AND both architecture anchor docs in the same output
- **Adversary agent:** VP-INDEX ↔ architecture document coherence review axis (4-point verification: self-consistency, catalog match, coverage matrix match, reverse check)
- **Product-owner agent:** VP citation change handoff — notes arch-doc propagation obligation when VP citations change in BCs
- **Consistency-validator criteria 78-80:** VP-INDEX self-consistency (arithmetic), VP-INDEX → verification-architecture.md completeness, VP-INDEX → verification-coverage-matrix.md completeness
- **5 BATS tests** for policy 9 presence in architect, adversary, product-owner, consistency-validator, FACTORY.md
- **Glossary term:** VP-INDEX Source of Truth

### Changed

- Consistency-validator criteria count: 77 → 80
- Session-review template: added policy 9 to governance policy audit checklist
- README: test count 133 → 138

## 0.24.0 — 8 governance policies formalized + permission model regression tests

### Added

- **Governance policy registry** in FACTORY.md — 8 top-level integrity policies with enforcing agents, validation criteria, and severity floors
- **8 governance policies** formally integrated into agent prompts:
  - `append_only_numbering` — IDs never renumbered, filename slugs immutable (product-owner, spec-steward, criterion 77)
  - `lift_invariants_to_bcs` — every DI-NNN must be cited by at least one BC (product-owner, adversary, criterion 74)
  - `state_manager_runs_last` — already present, verified across all agents
  - `semantic_anchoring_integrity` — formalized as criteria 70-73 (was unnumbered)
  - `creators_justify_anchors` — added to business-analyst for CAP-NNN creation
  - `architecture_is_subsystem_name_source_of_truth` — added product-owner BC subsystem validation, criterion 76
  - `bc_h1_is_title_source_of_truth` — added product-owner enrichment rule, adversary title sync axis, criterion 75
  - `bc_array_changes_propagate_to_body_and_acs` — story-writer, product-owner, adversary, criteria 67-69
- **permissions.bats** — new test suite (53 tests) covering agent permission model and governance policy presence
  - Profile enforcement: spec producers `coding`, code producers `full`, coordinators restricted
  - Tool-profile coherence: coding-profile agents have no shell commands in code blocks or inline backticks
  - Policy presence: each of 8 policies verified in correct agent prompts
- **Consistency-validator criteria 67-77** — 11 new validation criteria for governance policy enforcement
- **3 new adversary review axes** — BC title/subsystem sync, invariant-to-BC orphan detection, story frontmatter-body coherence
- **Glossary terms** — Append-Only Numbering, Governance Policy, Invariant Lifting

### Changed

- **accessibility-auditor** profile: `coding` → `full` (needs shell for axe-core, lighthouse, pa11y, eslint jsx-a11y)
- **FACTORY.md** permission model table: added "Tool-based reviewers" row for accessibility-auditor
- **Consistency-validator** criteria count: 66 → 77
- **Session-review template**: governance policy audit section expanded to cover all 8 policies
- **Cycle-manifest template**: added "Governance Policies Adopted" section
- **README.md**: test count 62 → 133, suite count 3 → 5

### Fixed

- **story-writer** pre-commit verification: changed "grep the story file" to "read the story file" (story-writer has `coding` profile, no shell access)
- **accessibility-auditor** tool-profile mismatch: was told to run `npx` commands but had `coding` profile (no exec)

## 0.23.0 — Comprehensive documentation update + Prism lessons + DTU taxonomy + agent permission model

### Added
- **12 Prism Phase 3 lessons** codified across agents and skills:
  - State-manager-last ordering, path-prefix verification, burst splitting (>8 artifacts), BC anchor-back in same burst
  - Fresh-context consistency audit at every gate, universal DTU integration surface taxonomy, BC retirement checklist, trajectory monotonicity, single source of truth rule
  - Structured human review questions at every gate, minimum 3 clean passes for convergence
- **Universal DTU integration surface taxonomy** — 6 mandatory categories (inbound data, outbound operations, identity & access, persistence & state, observability & export, enrichment & lookup) replacing project-specific categories
- **Agent permission model** documented in FACTORY.md, agents-reference, and configuration guide — spec producers (coding), code producers (full), coordinators (restricted), infrastructure (full)
- **Semantic Anchoring Integrity** — adversary, consistency-validator, product-owner, story-writer, architect enforce semantic correctness of all anchors
- **DTU assessment gate** mandatory in Phase 1 (P1-06) with pre-Phase 4 clone existence check
- **CI/CD deferred to post-architecture** (P1-06b) with pre-Phase 4 verification gate
- **Complete command coverage** — all 96 skills now have slash commands (was 47)
- **Activate agent ID fix** — 3-segment format (`vsdd-factory:orchestrator:orchestrator`)
- **Glossary entries** for semantic anchoring, convergence trajectory, integration surface taxonomy, single source of truth, anchor justification, trajectory monotonicity

### Changed
- **Getting-started guide** now includes `/activate` as Step 2, scaffold-claude-md as Step 5
- **Cross-cutting skills guide** expanded from ~25 to all 96 skills organized into 20 categories
- **Commands reference** expanded from ~47 to all 96 commands organized by category
- **Phase 1 guide** documents mandatory DTU assessment, CI/CD setup, anchor justification, consistency audit
- **Pipeline overview** documents DTU gate, CI/CD gate, consistency audit, pre-Phase 4 gates
- **README counts** corrected: commands 47→96, agents 34→33, templates 108→109

### Fixed
- **Command files** use colon syntax (`vsdd-factory:skill-name`) — was space syntax causing "Unknown skill" errors
- **Delegation commands** route through orchestrator instead of bouncing
- **Agent permissions** — product-owner, story-writer, architect reverted to `coding` profile (state-manager owns `.factory/` commits)
- **All project-specific references** removed from agent and skill files — generic examples only
- **Session-reviewer** agent name corrected throughout (was `session-review`)

## 0.22.0 — Semantic Anchoring Integrity

### Added
- **Adversary: Semantic Anchoring Audit** — new scan category verifying anchors are semantically correct, not just syntactically valid. 4-level severity matrix (CRITICAL/HIGH/MEDIUM/LOW). Mis-anchoring ALWAYS blocks convergence — never deferred as "Observation."
- **Consistency-validator: Anchor Semantic Audit** — verifies BC↔capability, story↔subsystem, VP↔anchor_story, and traceability table semantic correctness beyond structural ID matching
- **Product-owner: Anchor Justification Requirement** — must explicitly justify capability anchor choice citing source-of-truth when creating/modifying BCs
- **Story-writer: Anchor Justification Requirement** — must justify subsystem, dependency, and VP anchor choices with specific technical reasons
- **Architect: Anchor Justification Requirement** — must justify ADR references, subsystem assignments, and crate ownership claims. Planned-but-not-created crates must be marked `[PLANNED]`

## 0.21.0 — Orchestrator sync

### Fixed
- **Agent routing table** — added missing `codebase-analyzer` and `validate-extraction` agents, fixed `session-review` → `session-reviewer` to match agent filename, removed duplicate `product-owner` entry
- **Agents reference doc** — corrected `session-review` → `session-reviewer`

### Added
- **Cross-cutting skills reference** in orchestrator — table of 9 skills available at any pipeline point (scaffold-claude-md, visual-companion, create-excalidraw, systematic-debugging, writing-skills, validate-consistency, spec-drift, research, track-debt)

### Changed
- **State-manager delegation description** updated — orchestrator now documents that state-manager owns `.factory/` commits directly (no devops-engineer roundtrip)

## 0.20.1 — State-manager direct git commits

### Changed
- **State-manager now has shell access** for direct `.factory/` git commits. No longer spawns devops-engineer for every STATE.md update. Shell access is scoped: git commands inside `.factory/` only, no non-git commands, no source code branches.

## 0.20.0 — CI/CD deferred to post-architecture

### Changed
- **CI/CD setup moved out of repo-initialization** — repo-init no longer creates CI/CD workflows because the tech stack is unknown at init time. CI/CD is now a separate mandatory step (`phase-1-cicd-setup`) that runs after architecture determines the language, framework, and deployment topology.

### Added
- **Mandatory CI/CD setup step** (P1-06b) — devops-engineer creates `.github/workflows/` (ci.yml, release.yml, security.yml) based on architecture output, updates branch protection with CI status checks, produces `cicd-setup.md`
- **CI/CD criteria in Phase 1 gate** — ci.yml must exist, cicd-setup.md must exist, branch protection must require CI checks
- **Pre-Phase 4 CI/CD gate** — verifies CI pipeline exists and runs successfully before implementation begins
- CI/CD added to orchestrator's mandatory steps list (never skip, never conditional)

## 0.19.0 — Complete command coverage + activate agent ID fix

### Added
- **49 missing command files** — every skill now has a corresponding slash command for full autocomplete coverage
- Delegation reference commands (12) route through the orchestrator instead of bouncing
- Execution commands (37) invoke skills directly via the Skill tool

### Fixed
- **Activate skill writes correct 3-segment agent ID** (`vsdd-factory:orchestrator:orchestrator`). The 2-segment form (`vsdd-factory:orchestrator`) silently fell back to plain Claude because the orchestrator lives in a subdirectory.
- **Delegation command files** (dtu-creation, guided-brief-creation) now route through the orchestrator instead of trying to execute delegation-reference skills directly

## 0.18.0 — DTU assessment gate enforcement + command syntax fix

### Added
- **DTU assessment is now mandatory** (P1-06) — always produces `dtu-assessment.md`, even if the answer is "DTU_REQUIRED: false" with rationale. Prevents silent skip that occurred in Prism.
- **DTU checks in Phase 2 gate** — `dtu-assessment.md` must exist, fidelity classifications required if DTU_REQUIRED, rationale required if not
- **Pre-Phase 4 DTU clone existence gate** — if DTU_REQUIRED: true, verifies clones are built and validated before implementation begins
- **Mandatory steps list** in orchestrator — explicit "never skip, never conditional" list covering DTU assessment, adversarial convergence, holdout evaluation
- **DTU status in STATE.md** — state-manager writes `dtu_required`, `dtu_assessment`, `dtu_clones_built`, `dtu_services` fields for visibility across sessions

### Fixed
- **All 47 command files** now use colon syntax (`vsdd-factory:skill-name`) instead of space syntax (`vsdd-factory skill-name`). The space syntax caused "Unknown skill" errors when commands delegated to skills via the Skill tool.

## 0.17.0 — Prism Phase 3 lessons learned

### Added
- **8 lessons from Prism Phase 3 adversarial convergence** (29 passes, 46 stories, 167 BCs, 38 VPs) codified across 6 agent/skill files:
  - **story-writer:** must read source BC files (not summaries), use centralized version pins from dependency-graph.md, include forbidden dependencies section, use only existing error codes from taxonomy, pre-validate new stories against invariant list
  - **adversary:** accumulate confirmed invariants across passes (monotonically growing list)
  - **adversarial-review:** fix root causes not symptoms (rewrite from BC, don't patch lines), accumulate invariants, pre-validate new scope additions
  - **implementer:** fix root causes from BC source, read before editing and verify after editing
  - **deliver-story:** verify every fix landed correctly (read file, grep for pattern, check for side effects)
  - **create-story:** centralized version pins, forbidden dependencies section, error taxonomy compliance

## 0.16.1 — Reference manifest template + documentation fixes

### Added
- **reference-manifest-template.yaml** — standardized template for `.factory/reference-manifest.yaml` combining corverax and vsdd-factory formats (url, commit SHA, ingested date, depth, focus, status)
- End-user guide for visual companion (`docs/guide/visual-companion.md`) with Mermaid workflow diagrams

### Fixed
- Brownfield-ingest skill now references the template for manifest format
- Removed stale `/vsdd-factory:excalidraw-export` reference from visual companion See Also
- Added visual companion to README documentation table

## 0.16.0 — Excalidraw integration + visual companion testing

### Added
- **Excalidraw integration** in visual companion — `.excalidraw` files render as interactive canvases in the browser with user editing and WebSocket save-back
- **create-excalidraw skill** (`/vsdd-factory:create-excalidraw`) — generate `.excalidraw` JSON files for architecture diagrams, entity relationships, and flow charts
- **History sidebar** — collapsible panel showing all past screens (HTML and excalidraw), click to navigate
- **Composed views** — `screen.json` manifest for multi-pane layouts (split, side-by-side)
- **setup.sh** — one-time setup script installs React + excalidraw dependencies and builds the viewer
- **18 visual companion tests** — server routes, file-type detection, API endpoints, file serving
- React app scaffold (Vite + React 18 + @excalidraw/excalidraw v0.18)

### Fixed
- Server `__ACTIVE_FILE__` and `__MANIFEST__` injection now uses script tag insertion instead of string replacement
- Tiered visual tooling tables corrected across 5 files — replaced incorrect excalidraw-export reference with proper tiers (visual-companion excalidraw mode, create-excalidraw standalone, Mermaid, ASCII)

### Changed
- Visual companion server now supports `.html`, `.excalidraw`, and `screen.json` file types (was HTML-only)
- `/api/files` endpoint returns all screen files with metadata
- `/api/drawing/<name>` endpoint serves raw excalidraw JSON
- `/html/<name>` endpoint serves individual HTML files (for iframe embedding)
- Test suite now 80 tests across 4 suites (was 62 across 3)

## 0.15.0 — Systematic debugging, verification discipline, and writing-skills

### Added
- **systematic-debugging skill** — 4-phase root cause investigation process adapted from superpowers, with BC-aware debugging and "3+ fixes = architectural problem" escalation rule
- **writing-skills skill** — TDD methodology for creating and maintaining skills (RED-GREEN-REFACTOR applied to process documentation), with CSO guidance and rationalization resistance patterns
- **Verification discipline** — deliver-story and per-story-delivery now enforce independent verification of agent claims before proceeding (agent reports are claims, test output is evidence)
- **Review feedback guidance** — implementer and test-writer agents now have explicit guidance for receiving code review (verify before implementing, push back when wrong, BC is source of truth)

### Documentation
- Getting started guide now includes scaffold-claude-md as Step 4
- Cross-cutting skills guide documents visual-companion, systematic-debugging, and writing-skills
- Phase 1 guide documents visual tooling and self-review checklists
- Phase 2 guide documents scope check, plan failures, and self-review
- Phase 3 guide documents verification discipline, agent status protocol, model selection, review feedback handling, and debugging reference
- Agents reference documents the standard status protocol and self-review
- README skill count updated to 95

## 0.14.0 — Agent dispatch quality gaps

### Added
- **Standard agent status protocol** (DONE/DONE_WITH_CONCERNS/NEEDS_CONTEXT/BLOCKED) — agents-md-template, implementer, test-writer, pr-manager all report structured status codes
- **"Over your head" escalation language** — agents-md-template, implementer, test-writer explicitly encourage early escalation over bad work
- **Pre-handoff self-review checklists** — implementer (completeness, TDD, YAGNI), test-writer (coverage, behavior vs implementation, naming), pr-manager (description accuracy, traceability, demo evidence)
- **Model selection guidance** — deliver-story and per-story-delivery.md include tier mapping (fast/standard/capable) per dispatch task type
- **Extended Red Flags** — deliver-story adds 4 new dispatch anti-patterns (parallel dispatch, shared agents, skipped reviews, same-model retry)

## 0.13.0 — Story decomposition quality gaps

### Added
- **Hard gate language** — decompose-stories and create-story block premature implementation
- **Scope decomposition check** — decompose-stories verifies PRD describes a single product before breaking it down
- **"Plan Failures" anti-pattern list** — both skills explicitly ban "TBD", vague error handling, untestable ACs, and 4 other story-invalidating patterns
- **Self-review checklists** — decompose-stories checks spec coverage, consistency, and sizing; create-story checks completeness, testability, and context budget
- **Execution reference** in story template — points to `/vsdd-factory:deliver-story STORY-NNN`

## 0.12.0 — Early-phase quality gaps + visual companion

### Added
- **visual-companion skill** (`/vsdd-factory:visual-companion`) — browser-based mockups, diagrams, and interactive A/B choices during brainstorming, brief creation, and architecture design. Ported from superpowers. Optional, requires Node.js.
- **Tiered visual tooling strategy** — early-phase skills auto-detect available tools (visual-companion → excalidraw-export → Mermaid → ASCII) with no hard dependencies
- **Pre-adversarial self-review checklist** — added to create-brief, create-prd, create-architecture, and create-domain-spec to catch obvious gaps before the expensive adversary loop
- **Hard gate language** — explicit "do NOT skip to next phase" guards in brainstorming, guided-brief-creation, create-brief, create-prd, and create-architecture
- **Anti-pattern + Red Flags table** — brainstorming skill now calls out the "too simple to brainstorm" rationalization with a 7-row cognitive trap table

### Changed
- FACTORY.md documents visual companion in project tooling section
- VSDD.md references visual companion in Tooling section

## 0.11.0 — CLAUDE.md scaffolding skill

### Added
- **scaffold-claude-md skill** (`/vsdd-factory:scaffold-claude-md`) — auto-detects project language, build/test/lint commands, git workflow, and project references to generate a project-specific `CLAUDE.md`
- Activate skill now suggests `scaffold-claude-md` when no `CLAUDE.md` exists
- Optional `scaffold-claude-md` step in greenfield and brownfield workflows

### Changed
- FACTORY.md documents CLAUDE.md scaffolding in project setup section
- VSDD.md references the new skill in Tooling section

## 0.10.3 — Release infrastructure and CI/CD

### Added
- **Release workflow** (`.github/workflows/release.yml`) — tag-triggered validation + GitHub Release with CHANGELOG excerpt
- **Release config** (`.factory/release-config.yaml`) — declarative release manifest on factory-artifacts branch
- **Release skill rewrite** — config-driven, 3 modes (init/release/dry-run), quality gate spectrum
- Retroactive git tags and GitHub Releases for all 12 prior versions (v0.1.0 through v0.10.2)
- Version field in marketplace.json for release validation
- Factory-artifacts mount step in CI and release workflows

### Changed
- CI workflow renamed from `plugin-validation.yml` to `ci.yml` for cross-repo consistency
- Bump `actions/checkout` from v4 to v6 (Node.js 20 deprecation)

## 0.10.2 — Template path portability fix

Closes a portability hole that would have broken clean installs.

### The bug

Skills and agents referenced templates as `.claude/templates/<name>.md` — a path that only exists inside corverax, where the plugin was originally developed and `.claude/templates/` is pre-populated. A clean install of vsdd-factory into any other project would ship the templates at `plugins/vsdd-factory/templates/` (where they actually live) but every skill referencing `.claude/templates/...` would fail the lookup.

59 references across 24 files were affected:

- 20 skills: `research`, `semport-analyze`, `brownfield-ingest`, `create-brief`, `create-story`, `create-domain-spec`, `create-architecture`, `create-prd`, `adversarial-review`, `holdout-eval`, `state-update`, `record-demo`, `pr-create`, `decompose-stories`, `track-debt`, `convergence-check`, `validate-consistency`, `deliver-story`, `dtu-validate`, `formal-verify`
- 4 agents: `validate-extraction`, `research-agent`, `adversary`, `holdout-evaluator`

### The fix

All 59 references rewritten from `.claude/templates/<name>` to `${CLAUDE_PLUGIN_ROOT}/templates/<name>` — the Claude Code canonical environment variable for the plugin root directory. Agents shell-expand the variable when reading via bash, and the path resolves to the real template location that ships with the plugin regardless of install target.

### Regression guards (3 new tests)

`tests/skills.bats` grew a "Template path portability" section with three tests:

- `no skill references the non-portable .claude/templates/ path` — grep-based regression guard
- `no agent references the non-portable .claude/templates/ path` — same
- `every referenced template actually exists in plugin templates/` — extracts every `${CLAUDE_PLUGIN_ROOT}/templates/<file>` reference from skills and agents, strips the prefix, and asserts the file exists at `plugins/vsdd-factory/templates/<file>`. Caught zero dangling references on first run.

Test suite now **62 tests**, all pass.

### Note for future skill authors

When citing a template in a new skill or agent, use:

```
- `${CLAUDE_PLUGIN_ROOT}/templates/<name>.md` — <description>
```

The `.claude/templates/` prefix is never portable and is now a test failure.

## 0.10.1 — Step-file content fill

Closes the last deferred item from 0.9.0: empty `steps/` placeholder stubs in three skills now carry real per-step playbooks.

### 17 step files expanded (1566 LOC)

The three facilitation / inspection skills (`brainstorming`, `artifact-detection`, `guided-brief-creation`) had `## Step-File Decomposition` tables referencing per-step files that were 3-6 line placeholders. The parent SKILL.md carried the high-level flow; the step files existed only as stubs.

Each step file is now a 58-130 line self-contained playbook the orchestrator can load on demand when executing that specific step. Structure per file:

- **Inputs** — what previous steps produced, files to read, expected state
- **Outputs** — exact artifact paths and formats
- **Procedure** — specific moves, exact elicitation questions (for facilitation skills), exact commands and glob patterns (for inspection skills)
- **Decision points** — branches with criteria, where applicable
- **Failure modes** — step-level failures (distinct from whole-skill failures in parent SKILL.md)
- **Quality gate** — short checklist before advancing
- **Hand-off** — what to pass to the next step

**brainstorming (6 files, 487 LOC):** session setup, technique selection, facilitated ideation, synthesis, direction selection, report writing. Includes exact opening questions, transition phrases, SCAMPER/reverse-brainstorming/mind-mapping/constraint-removal scripts, and the verbatim markdown template for `brainstorming-report.md`.

**artifact-detection (5 files, 510 LOC):** scan, classify, validate, gap analysis, route decision. Includes exact glob patterns per artifact type, explicit validation checklists (rewritten from the SKILL.md prose as iterable rules), DF-020/DF-021 format-migration handling, and verbatim templates for `artifact-inventory.md`, `gap-analysis.md`, and `routing-decision.md`.

**guided-brief-creation (6 files, 569 LOC):** understand intent, contextual discovery, guided elicitation, draft review, adversarial review, finalize. Includes exact section-by-section elicitation questions, research-agent / adversary dispatch criteria, market-intel integration points, and verbatim structures for `product-brief.md` and `elicitation-notes.md`.

### Cross-step dependencies surfaced

Step files make explicit several dependencies that were implicit in the prose:

- **artifact-detection format flags propagate** — format detection in step 1b (FR-NNN vs BC-S.SS.NNN, old vs DF-021-sharded architecture) flows into step 3 validation rules and step 5 routing decisions
- **guided-brief-creation market-intel reference** — `market-intel.md` is read in step 3 and again in step 5 adversarial review for differentiation and risk signals
- **guided-brief-creation adversarial loopback** — step 5 feedback can send the agent back to step 3 (re-elicitation) or step 4 (redraft)

### Meta

- No SKILL.md files modified. Step-file decomposition tables unchanged.
- All 59 tests still pass. No new tests added for step-file content (content is prose, not behavior).
- Full analysis report at `.factory/semport/STEPS-REPORT.md`.

## 0.10.0 — Deferred remediation: commands, hook envelopes, structural tests

Closes out the remaining P1/P2 items deferred from 0.9.0.

### Commands directory (47 files)

Prior versions exposed skills only — many with `disable-model-invocation: true`, which meant users had no slash-command entry point for phase transitions, health checks, or delivery. This release ships `plugins/vsdd-factory/commands/` with **47 thin slash-command wrappers**, one per user-facing skill.

Each command is 15-30 lines: frontmatter (description + optional `argument-hint` mirrored from the skill) and a body that delegates via `Use the <skill-name> skill via the Skill tool`. Commands are entry points; skills remain the source of truth.

Coverage: all Phase 0-6 lifecycle skills (brownfield-ingest, semport-analyze, create-brief through release), cross-cutting ops (factory-health, track-debt, worktree-manage), and UI/design skills (design-system-bootstrap, ui-quality-gate, etc.).

### Hook upgrade: permissionDecision envelopes (POC on spec-steward)

`hooks/protect-vp.sh` and `hooks/protect-bc.sh` now emit `PreToolUse` JSON envelopes with `permissionDecision` + `permissionDecisionReason` instead of bare exit codes. The denial reasons are richer and instruct the agent to create a superseding artifact rather than just blocking the edit.

This is a POC on the two spec-steward hooks. The other hooks (`brownfield-discipline`, `red-gate`, `purity-check`, etc.) still use exit codes. Upgrading them requires per-hook design — deferred until a specific need motivates each one.

Tests updated: the two "blocks edit to green X" tests now assert `status -eq 0` with `permissionDecision:deny` in stdout, replacing the old `status -eq 2` stderr check.

### Structural tests for Iron Laws and Red Flags (18 new tests)

New `tests/skills.bats` enforces that the four discipline skills carry their behavior-shaping scaffolding. A discipline skill missing its Iron Law, "Announce at start" line, or Red Flags table is now a test failure — empirically load-bearing content cannot silently rot.

Test coverage per skill:
- `deliver-story`, `brownfield-ingest`, `adversarial-review`, `wave-gate`: Iron Law token + `## The Iron Law` section + `## Announce at Start` section + `## Red Flags` table with ≥8 rows
- `brownfield-ingest` specifically: Honest Convergence clause, Known Round-1 Hallucination Classes, Subagent Delivery Protocol (`=== FILE:` delimiter), Behavioral vs Metric split, Priority-ordered Lessons mandate
- `validate-extraction` agent: Behavioral vs Metric operating mode with Phase 1 / Phase 2 sections

Total suite: **59 tests** (41 pre-existing + 18 new). All pass.

### Name collision fix

`agents/session-review.md` renamed to `agents/session-reviewer.md` to disambiguate from the `skills/session-review/` directory. Non-breaking — no referring files use the old basename (verified via grep).

### Deferred

Placeholder `steps/` stubs in `brainstorming`, `artifact-detection`, and `guided-brief-creation` skills are still empty. These need real content (not a mechanical fix); tracked for a scoped content PR.

Non-spec-steward hooks remain on exit-code semantics until a per-hook motivation exists for the envelope upgrade.

## 0.9.0 — Self-ingest remediation: apply lessons from claude-code + superpowers

Applies the P0/P1 lessons from running the plugin's own `brownfield-ingest` protocol against `anthropics/claude-code` and `obra/superpowers` in the `.factory/semport/` analysis. The ingest caught 3 round-1 hallucinations via strict-binary novelty, which validated both the protocol and specific gaps in the plugin itself.

### Agent frontmatter remediation (Group A — 46 files)

- **26 agent descriptions rewritten** from the boilerplate stub `VSDD factory agent: <name>` to one-sentence "Use when..." triggers drawn from each agent's body, following superpowers' CSO rule (third-person, when-not-what, <1024 chars).
- **`model:` field added to 28 agents.** Defaults to `sonnet`. Exceptions on `opus`: `adversary`, `holdout-evaluator`, `formal-verifier`, `pr-reviewer`, `spec-reviewer` — terminal reviewers where reasoning quality dominates call volume.
- **`color:` field added to all 33 root agents**, grouped by function: reviewers=red, builders=green, planners=blue, ops=yellow, research=purple.
- **`implementer.md` description** fixed (was truncated mid-sentence).
- **9 `agents/orchestrator/` include files** gained YAML frontmatter with `disable-model-invocation: true` so strict loaders no longer trip on them.
- **`excalidraw-export` and `jira` SKILLs** gained frontmatter (reference-only, `disable-model-invocation: true`).
- **`state-update` skill** marked `disable-model-invocation: true` (internal).

### deliver-story dispatch rewrite (Group B)

`skills/deliver-story/SKILL.md` was a single-context script that quietly drifted from the `agents/orchestrator/per-story-delivery.md` workflow it was supposed to use. Rewritten as a thin dispatcher:

- Declares itself a dispatcher, not an implementer, via `EXTREMELY-IMPORTANT` block.
- Iron Law: `NO IMPLEMENTATION WITHOUT RED GATE VERIFICATION FIRST`.
- Prerequisites check that STOPs on failure (no silent bypass).
- 9-step dispatch sequence: devops-engineer → test-writer (stubs) → test-writer (tests) → **independent Red Gate verification** → implementer → demo-recorder → implementer (push) → pr-manager → devops-engineer (cleanup) → state update.
- Context discipline table naming which files each specialist receives (prevents topic drift from passing whole-story context to every agent).
- Story split recovery flow for oversized PRs.
- 10-row Red Flags table targeting the rationalizations that lead back to single-context execution.
- `agents/orchestrator/per-story-delivery.md` header marked as canonical source.

### brownfield-ingest self-improvements (Group C)

Codifies the 5 lessons the ingest protocol taught itself when applied to real reference repos:

- **Honest Convergence clause** — mandatory verbatim text in every round prompt: "<3 substantive → declare converged, emit no file." Stops agents from fabricating findings under pressure to produce SUBSTANTIVE output.
- **Known Round-1 Hallucination Classes** — 5 named failure modes (over-extrapolated token lists, miscounted enumerations, named pattern conflation, same-basename artifact conflation, inflated/deflated metrics) with verbatim examples from superpowers round 1 (persuasion matrix, Pressure Taxonomy, writing-plans forbidden tokens). Round 2+ prompts must audit round 1 against these classes.
- **Subagent Delivery Protocol (inline-by-default)** — `=== FILE: <name> ===` delimiter pattern that works around sandbox Write denials. Explicit override of subagent default system prompts that forbid "inline fallback."
- **Behavioral vs Metric split** in Phase B.6 — mandatory two-phase validation: Phase 1 samples contracts/entities for CONFIRMED/INACCURATE/HALLUCINATED (judgment); Phase 2 independently recounts every numeric claim via `find` + `wc -l` (arithmetic, not judgment). Empirical anchor: superpowers Pass 0 round 1 claimed 32 files / 5279 LOC; recount showed 23 files / 3859 LOC.
- **Priority-ordered Lessons mandate** in Phase C — synthesis MUST include a `## Lessons for <target-project>` section with P0/P1/P2/P3 buckets, each lesson naming (a) what target does today, (b) what reference does, (c) gap, (d) specific action items with file paths. Makes the synthesis a directly actionable backlog.
- **`agents/validate-extraction.md`** updated with matching operating-mode split and two-table output format.

### Iron Laws and Red Flags rollout (Group D)

Applies superpowers' empirically-anchored behavior-shaping scaffolding to the 4 highest-stakes discipline skills. Iron Laws follow the canonical form `NO <verb> <scope> WITHOUT <prerequisite> FIRST`. Each skill gained an "Announce at Start" verbatim line and a Red Flags table enumerating the rationalizations observed during pressure testing.

- **`deliver-story`** — `NO IMPLEMENTATION WITHOUT RED GATE VERIFICATION FIRST` (+ 10 Red Flags, included in Group B rewrite)
- **`brownfield-ingest`** — `NO ROUND COMPLETION WITHOUT HONEST CONVERGENCE CHECK FIRST` (+ 10 Red Flags)
- **`adversarial-review`** — `NO APPROVAL WITHOUT FRESH-CONTEXT REVIEW FIRST` (+ 8 Red Flags targeting information-asymmetry violations)
- **`wave-gate`** — `NO WAVE ADVANCE WITHOUT ALL SIX GATES PASSING FIRST` (+ 8 Red Flags targeting threshold rounding, gate skipping, flake handling)

### AGENT-SOUL pragmatism footnote (Group E)

`docs/AGENT-SOUL.md` §8 "Pragmatism Over Ceremony" gained a footnote distinguishing **principled pragmatism** (design-time, human-in-loop, ROI-reasoning, documented) from **rationalization** (execution-time, bypass-a-rule). References superpowers' Pressure Taxonomy and the Meincke 2025 empirical anchor (N=28000, compliance 33%→72% under persuasion pressure) — which names "I'm just being pragmatic" as a first-class attack vector on discipline skills. This is the principle most easily weaponized to justify skipping Iron Laws; the footnote exists to stop that.

### Meta

- Reference analysis artifacts live in `.factory/semport/claude-code/` and `.factory/semport/superpowers/` (Phase A + B + C complete, validated).
- `TAKEAWAYS.md` and `PLUGIN-INVENTORY.md` in `.factory/semport/` document the analysis → remediation trace.
- No behavior changes to hooks, workflows, or bin helpers.
- No new tests yet — Group F bookkeeping only. Test coverage for the new Iron Law / Red Flags content is deferred.

## 0.8.0 — Wave 7: Validation infrastructure

Ships the test harness that validates the enforcement layer actually works. Previously, Wave 4's hooks and Wave 5/6's bin helpers had only smoke tests ("does it run without crashing"). Wave 7 adds allow/block path coverage.

- **41 TAP tests** across two bats files:
  - `tests/hooks.bats` (28 tests) — allow and block paths for every hook: brownfield-discipline (4), protect-vp (4), protect-bc (3), red-gate (6), purity-check (3), handoff-validator (3), regression-gate (3), session-learning (2)
  - `tests/bin.bats` (13 tests) — lobster-parse (3, including all 15 workflow files parse), research-cache (4, round-trip + determinism + normalization), multi-repo-scan (3), wave-state (3)
- **Smoke fixture** `fixtures/smoke-project/` — minimal Rust crate with one passing test, for future hook integration tests
- **Test runner** `tests/run-all.sh` — syntax checks + hook tests + bin tests; tool-guarded per `bash.md`
- **GitHub Actions CI** `.github/workflows/plugin-validation.yml` — runs on push/PR to main: installs bats/jq/yq, syntax-checks shell scripts, runs both test suites, validates all JSON manifests, parses every workflow file

All 41 tests pass on first run locally.

## 0.7.0 — Wave 6: Runtime helpers and not-portable documentation

Finishes the runtime-extension port. Ships bin helpers for the extensions that map to bash+jq+yq, wraps them in skills, and documents the four that cannot be ported.

**New bin helpers** (`plugins/vsdd-factory/bin/`):

- `research-cache` — SHA-keyed disk cache for Perplexity/Context7 query results at `.factory/research-cache/`. Subcommands: `get`, `put`, `has`, `key`, `clear`, `stats`. Ports `research-cache.ts`.
- `wave-state` — read-only query of `.factory/stories/sprint-state.yaml`. Subcommands: `current`, `stories`, `ready`, `summary`. Read-only slice of `wave-orchestrator.ts`.
- `multi-repo-scan` — detects multi-repo layouts from `.worktrees/`, reports repos with manifest types. Read-only slice of `multi-repo-orchestrator.ts`.

**New skill wrappers**:

- `research-cache-ops` — operates the research cache from within a session
- `wave-status` — reports wave readiness with recommendations
- `multi-repo-health` — detects multi-repo layouts and cross-checks against `.factory/stories/`

**Not-portable documentation** (`docs/not-portable.md`):

Documents why four dark-factory extensions cannot port to Claude Code's plugin primitives:

- `cost-tracker.ts` — no `PreModelCall` hook
- `attention-heatmap.ts` — no read-event hooks
- `tiered-context.ts` — Claude Code manages context natively
- `sidecar-learning.ts` (full synthesis) — `Stop` hook has no transcript access; partial marker-only port shipped in Wave 4

All bin helpers follow `bash.md`: `set -euo pipefail`, stderr guards, STDERR-EXEMPT tags, tool availability checks. Pass `bash -n` syntax checks and basic smoke tests.

Total skills: 91. Total bin helpers: 4.

## 0.6.0 — Wave 5: Orchestrator + workflow data (Lobster replacement)

Replaces dark-factory's Lobster workflow DSL with "Lobster-as-data" driven by the orchestrator agent and a bash helper.

- **Workflow corpus** — shipped all 15 `.lobster` files as data under `plugins/vsdd-factory/workflows/`:
  - Mode workflows: greenfield, brownfield, feature, maintenance, discovery, planning, multi-repo, code-delivery
  - Phase sub-workflows: phase-0-codebase-ingestion, phase-1-spec-crystallization, phase-3-tdd-implementation, phase-3.5-holdout-evaluation, phase-4-adversarial-refinement, phase-5-formal-hardening, phase-6-convergence
- **`bin/lobster-parse`** — bash helper wrapping `yq` + `jq` that emits workflow files as JSON with optional jq expressions. Lobster files parse cleanly as YAML.
- **Orchestrator agent updated** — added a Workflow Data section that points at the `workflows/` corpus and documents the lobster-parse helper with worked examples.
- **Five new skills** in `skills/`:
  - `run-phase` — execute a phase by reading its Lobster file and spawning declared sub-agents in dependency order
  - `next-step` — read `.factory/STATE.md` + active workflow, propose next action (does not execute)
  - `validate-workflow` — static schema check: required fields, agent/skill existence, depends_on graph, cycles, duplicate step names
  - `activate` — per-project opt-in that writes `{"agent": "vsdd-factory:orchestrator"}` to `.claude/settings.local.json`
  - `deactivate` — removes the agent override; leaves plugin enabled

Opt-in design (vs hijacking default persona on plugin enable) chosen per earlier decision — activation is always an explicit user action, per-project.

Total skills: 88.

## 0.5.0 — Wave 4: Enforcement layer (hooks)

Ported dark-factory's OpenClaw runtime extensions to Claude Code hooks. This is the "make the wrong thing impossible" wave — recovering the enforcement discipline that was missing from the initial extract.

**New hooks** (in `plugins/vsdd-factory/hooks/`):

- `brownfield-discipline.sh` (PreToolUse) — blocks edits to `.reference/**`
- `protect-bc.sh` (PreToolUse) — blocks edits to green Behavioral Contracts
- `red-gate.sh` (PreToolUse) — enforces TDD red-before-green when `.factory/red-gate-state.json` declares strict mode; opt-in per project
- `purity-check.sh` (PostToolUse, warn) — flags side-effect patterns in pure-core paths (`*/pure/**`, `*/core/**`, `*_pure.rs`, `*.pure.ts`, `*/kernel/*`)
- `regression-gate.sh` (PostToolUse) — records cargo/pytest/npm/go test outcomes to `.factory/regression-state.json`, warns on pass→fail transitions
- `handoff-validator.sh` (SubagentStop) — warns on empty/truncated subagent output
- `session-learning.sh` (Stop) — appends session-end markers to `.factory/sidecar-learning.md`

**Wired existing hooks**:

- `protect-vp.sh` (PreToolUse, Edit|Write) — already shipped, now registered
- `verify-git-push.sh` (PreToolUse, Bash) — registered
- `check-factory-commit.sh` (PreToolUse, Bash) — registered

All hooks follow `.claude/rules/bash.md`: `set -euo pipefail`, jq-based JSON parsing with stderr guards, no `eval`, tool availability checks, STDERR-EXEMPT tags where stderr suppression is intentional. All 10 hooks pass `bash -n` syntax checks and basic smoke tests.

**Not portable** (needs API-level integration Claude Code doesn't expose):

- Cost tracker, attention heatmap, tiered-context enforcement, full sidecar-learning synthesis — will ship as doc stubs in Wave 6.

## 0.4.0 — Wave 3: Design system, UX, and market intelligence

- Ported 13 skills for UI-heavy projects and product-intelligence workflows
- **Design & UX:** `design-drift-detection`, `design-system-bootstrap`, `multi-variant-design`, `storybook-mcp-integration`, `responsive-validation`, `ui-completeness-check`, `ui-quality-gate`, `ux-heuristic-evaluation`
- **Market & customer:** `competitive-monitoring`, `customer-feedback-ingestion`, `intelligence-synthesis`, `market-intelligence-assessment`, `analytics-integration`
- `templates/design-system/` already present from initial extraction

Total skills: 83.

## 0.3.0 — Wave 2: Skill coverage (feature-mode + maintenance)

- Ported 39 skills from dark-factory workflow catalogue
- **Feature-mode (F1–F7):** `phase-f1-delta-analysis`, `phase-f2-spec-evolution`, `phase-f3-incremental-stories`, `phase-f4-delta-implementation`, `phase-f5-scoped-adversarial`, `phase-f6-targeted-hardening`, `phase-f7-delta-convergence`
- **Maintenance & discovery:** `maintenance-sweep`, `discovery-engine`, `planning-research`, `post-feature-validation`, `pr-review-triage`, `fix-pr-delivery`
- **Mode routing:** `mode-decision-guide`, `quick-dev-routing`, `feature-mode-scoping-rules`, `implementation-readiness`, `validate-brief`
- **Infrastructure:** `model-routing`, `repo-initialization`, `toolchain-provisioning`, `wave-scheduling`, `spec-versioning`, `traceability-extension`, `sdk-generation`
- **Session & consistency:** `consistency-validation`, `convergence-tracking`, `artifact-detection`, `phase-1-prd-revision`, `phase-1d-adversarial-spec-review`, `multi-repo-phase-0-synthesis`, `factory-worktree-health`, `dtu-creation`, `brainstorming`, `agent-file-review`, `code-delivery`, `demo-recording`, `session-review`, `guided-brief-creation`
- Replaced Corverax's `release` skill with dark-factory's authoritative version per merge rules

Total skills: 70 (was 31).

## 0.2.0 — Wave 1: Foundation

- Shipped `docs/VSDD.md`, `docs/FACTORY.md`, `docs/CONVERGENCE.md` — the methodology documents
- Shipped `docs/AGENT-SOUL.md` — shared engine-wide agent principles
- Ported 28 dark-factory agents into single-file triune format (`## Identity` + `## Operating Procedure`) with synthesized frontmatter:
  `accessibility-auditor, architect, business-analyst, code-reviewer, consistency-validator, data-engineer, demo-recorder, devops-engineer, dtu-validator, dx-engineer, e2e-tester, formal-verifier, github-ops, implementer, performance-engineer, pr-manager, pr-reviewer, product-owner, security-reviewer, session-review, spec-reviewer, spec-steward, state-manager, story-writer, technical-writer, test-writer, ux-designer, visual-reviewer`
- Shipped `agents/orchestrator/` as a directory containing `orchestrator.md` plus 9 mode-sequence sub-files (greenfield, brownfield, feature, maintenance, discovery, multi-repo, per-story-delivery, steady-state, heartbeat)
- Kept Corverax's enhanced versions of the 5 overlapping agents (adversary, codebase-analyzer, holdout-evaluator, research-agent, validate-extraction) unchanged

Total agents: 34 (33 dark-factory + 1 Corverax addition).

## 0.1.0 — Initial marketplace

- Extracted Corverax `.claude/` VSDD pipeline into a shareable plugin marketplace
- 5 agents, 31 skills, 3 hooks, rules, templates
