# S-25.01 Demo Evidence — Dispatcher INDETERMINATE Outcome Layer 1

Story: `S-25.01` — Dispatcher INDETERMINATE Outcome Layer 1: Fail-Loud on Cannot-Complete
(durable `.factory/unvalidated-mutation.marker` + two-arm next-advance gate).
Branch: `feature/S-25.01`. Recorded post-3-CLEAN, pre-PR finalization sweep.

This is a backend/dispatcher story (WASM hook marker lifecycle + INDETERMINATE outcome
classification in `crates/factory-dispatcher` and `crates/hook-plugins/validate-unvalidated-mutation-marker`),
not a UI product. Evidence below is terminal-based (VHS recordings + raw stdout/JSONL
transcripts), per the CLI product-type routing.

## How the evidence was produced

All four VHS recordings and all four raw transcripts invoke **the real production code
path** — not a reconstruction or a mock:

- `crates/factory-dispatcher/examples/demo_s2501_marker_lifecycle.rs` drives the exact
  `execute_tiers` dispatch path that `crates/factory-dispatcher/tests/marker_integration.rs`
  exercises, against real WAT-compiled WASM fixtures, and prints the resulting
  `.factory/unvalidated-mutation.marker` TOML file plus the durable
  `dispatcher-internal-<date>.jsonl` audit lines the dispatcher actually wrote.
- `crates/hook-plugins/validate-unvalidated-mutation-marker/examples/demo_s2501_block_gate.rs`
  calls the fully-`pub` `on_pre_tool_use(payload: HookPayload) -> HookResult` entry point —
  the identical function the compiled WASM plugin exports as its `PreToolUse` hook — so the
  printed block message is byte-for-byte what Claude Code's dispatcher would receive.

Both `examples/` files are demo-generation scratch tooling only. They are **not** committed
to the repository (additive-only constraint: no production code, test, or spec files were
modified) — only their *output* (the recordings and transcripts below) is committed.

VHS is used per the demo-recorder CLI-product convention (not plain-text `cargo test`
captures). One caveat: the locally installed `vhs` (v0.11.0) `Wait`/`Wait+Line` directive
hung indefinitely against this environment's shell in a minimal reproduction (confirmed
independent of tape content); all four `.tape` files use generously-sized `Sleep` directives
instead, calibrated against measured command runtimes. This is noted inline in each `.tape`
file.

## Evidence artifacts

| Artifact | Scenario | AC(s) | BC(s) | Format |
|---|---|---|---|---|
| `AC-001-005-006-indeterminate-marker-write.{gif,webm,tape}` | Fuel-exhaustion on a `PostToolUse` fail-closed named plugin (`validate-factory-path-staging`) classifies `DispatchOutcome::Indeterminate{cause="fuel"}`, atomically writes the 6-field marker TOML, and durably emits `plugin.indeterminate` (Event 8) + `marker.written` (Event 10) | AC-001, AC-005, AC-006 | BC-1.18.001, BC-3.08.001 | VHS gif+webm+tape |
| `AC-007-008-009-010-block-gate.{gif,webm,tape}` | Two-arm next-advance gate: Agent dispatch blocked (Arm 1) and `git commit`/`git push` Bash dispatch blocked (Arm 2) while the marker is active, with the full T1→T2→T3 recovery-guidance block message (no `rm` instruction given to the agent — human-operator-only); `git status` (non-advancing) passes through the filter unchecked; marker-absent allows both arms; post-`rm` break-glass unblocks both arms | AC-007, AC-008, AC-009, AC-010 | BC-1.18.002, BC-1.18.003 (PC3 escape hatch) | VHS gif+webm+tape |
| `AC-012-022-revalidated-clear.{gif,webm,tape}` | The SAME named plugin (`validate-factory-path-staging`) re-run on the SAME artifact now returns PASS → marker deleted, `marker.cleared(clear_mode=REVALIDATED)` (Event 9) emitted carrying the *original* `plugin.indeterminate`'s `trace_id` (provenance linkage), not the new dispatch's trace_id | AC-012, AC-022 | BC-1.18.003 (PC1), BC-3.08.001 (Event 9) | VHS gif+webm+tape |
| `AC-021-ttl-deadman-expiry.{gif,webm,tape}` | A marker whose `expires_at` (24h deadman TTL) has already elapsed is auto-deleted by `check_and_clear_expired_marker` — the same native pre-check `executor.rs` runs before every Arm 1/Arm 2 plugin invocation — emitting `marker.cleared(clear_mode=TTL_EXPIRED, actor_type=deadman)` | AC-021 | BC-1.18.001 (PC4), BC-1.18.003 (PC4), BC-3.08.001 (Event 9), ADR-048 §Decision 2 | VHS gif+webm+tape |
| `transcripts/scenario-A-fuel-indeterminate.txt` | Raw stdout capture of the fuel-indeterminate scenario (grep-able marker TOML + JSONL) | AC-001, AC-005, AC-006 | BC-1.18.001, BC-3.08.001 | plain text |
| `transcripts/scenario-B-revalidated-clear.txt` | Raw stdout capture of the REVALIDATED-clear scenario | AC-012, AC-022 | BC-1.18.003, BC-3.08.001 | plain text |
| `transcripts/scenario-C-ttl-expiry.txt` | Raw stdout capture of the TTL-expiry scenario | AC-021 | BC-1.18.001, BC-1.18.003, BC-3.08.001 | plain text |
| `transcripts/scenario-D-block-gate.txt` | Raw stdout capture of all 7 block-gate sub-scenarios (Agent/Bash × marker present/absent/removed) | AC-007, AC-008, AC-009, AC-010 | BC-1.18.002, BC-1.18.003 | plain text |
| `transcripts/cargo-test-marker-integration.txt` | `cargo test -p factory-dispatcher --test marker_integration --test bc_1_18_002_block_if_marker` — 13/13 passing, driving the identical production `execute_tiers` path (regression backstop for the recordings above, incl. SUPERSEDED cross-pair overwrite and artifact-scoped clear coverage not separately recorded) | AC-001–AC-025 (full marker-lifecycle Red Gate suite) | BC-1.18.001, BC-1.18.002, BC-1.18.003 | plain text |
| `transcripts/cargo-test-block-gate-lib.txt` | `cargo test -p validate-unvalidated-mutation-marker --lib` — 19/19 passing (block-message field/escape-hatch assertions, `is_git_commit_or_push` Phase 1–4 vectors, TTL/INV6 tests) | AC-007, AC-008, AC-009 | BC-1.18.002 | plain text |

## Note on scope

Per the finalization-sweep instructions, this evidence sweep covers the four core
INDETERMINATE-outcome lifecycle behaviors (fuel-cause classification + marker write +
Event 8/10; next-advance block with recovery guidance; REVALIDATED clear; TTL deadman
expiry). The SUPERSEDED cross-pair overwrite path (AC-024) and the epoch/OutputTooLarge
classification causes (AC-002/AC-003) are covered by the existing `marker_integration.rs`
and `bc_1_18_002_block_if_marker.rs` regression suites referenced above (13/13 passing)
but were not separately re-recorded as VHS demos, since they exercise the same
`execute_tiers`/`classify_outcome` machinery already demonstrated live in
`AC-001-005-006-indeterminate-marker-write`.
