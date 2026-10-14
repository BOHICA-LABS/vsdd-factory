# S-25.08 demo evidence (per-AC)

Story: S-25.08 shared admission core + B2 conformance + VP-147 re-baseline. Branch `feature/S-25.08`, recorded at HEAD `9e936d4c`.
Product type: CLI/dispatcher (no UI), so evidence is VHS terminal recordings (`.tape` source + `.gif` + `.webm`).

How the recordings were made:
- Real binary: `factory-dispatcher` built from this worktree (`cargo build -p factory-dispatcher`, debug profile).
- Real fixtures in fresh temp dirs (`tempfile`-style `s2508-demo-*`), built by `demo-driver.py` (evidence-only driver, lives only in this folder; no `.sh` files added). It never touches the repo's own `.factory/`.
- `demo-driver.py` talks to the dispatcher exactly as Claude Code does: a hook-envelope JSON on stdin, `CLAUDE_PROJECT_DIR` / `CLAUDE_PLUGIN_ROOT` env, an empty registry (`schema_version = 2`, zero hooks) so only the native admission gate can act. A live coordinator is modelled by holding `flock` on `exclusive.lock`.
- Named test runs (`cargo test -p factory-dispatcher --test <suite>`) are the authoritative proof where a demo adds nothing. Full-suite logs are in `testrun-*.log` (all green at HEAD: admission 48, diagnostics 9, pure-core/TTL 16, registry exit code 5, release-stat-error 4, b2 migration 56).
- Regenerate any recording from the repo root: `vhs docs/demo-evidence/S-25.08/<name>.tape`.

## AC to artifact map

Legend: **DEMO** = real-binary recording; **TEST** = named cargo test run (recording would add nothing, reason given); **KANI** = formal proof run.

| AC | Behaviour | Artifact | Kind / why |
|----|-----------|----------|-----------|
| AC-001 | Gate wired on the production PreToolUse path: STAGING/COMMITTING block protected writes with E-MAINTENANCE-001 (`<scope>` keyed on path family), out-of-union path and OPEN gate admitted | `AC-001-002-013-014-admission-block-verdicts.{tape,gif,webm}`; `testrun-s2508_admission_blackbox_test.log` (`..PC6b_gate_wired_on_production_path_blackbox`) | DEMO + TEST |
| AC-002 | One shared core evaluated once, ahead of the shard-cap precheck | same recording shows the verdict (`blocking_plugins` path is the native gate); ordering is asserted by `..PC6b_single_shared_core_evaluated_once_ahead_of_shard_cap_precheck` in `testrun-s2508_admission_blackbox_test.log` | TEST (call ordering inside one process is not visible at the process boundary) |
| AC-003 | Reservation created on PreToolUse, removed on PostToolUse | `AC-003-004-reserve-on-pre-release-on-completion.*` | DEMO |
| AC-004 | Bash left unprocessed: no reservation, no state write | `AC-003-004-reserve-on-pre-release-on-completion.*` (last step) | DEMO |
| AC-005 | Reserve-then-verify, all interleavings (mechanism-A + B2) | `AC-005-006-reserve-then-verify-and-release-on-block.*` (suite run) | TEST: interleavings need the `VSDD_TEST_ADMISSION_SEAM_DIR` test seam to pause between W1 and W2; not reproducible from a plain terminal |
| AC-006 | Release-on-block: a later-stage block releases the reservation | same recording (`..EC018_later_stage_block_releases_reservation_blackbox`) | TEST: needs a second blocking stage plus a seam to observe the reservation mid-flight |
| AC-007 | B2 reconciliation branches A/B/C wired on the production path | `AC-007-009-015-020-b2-coordinator-conformance.*` (Branch A self-heal DRAINING to OPEN; Branch B STAGING+null generation to ABORTED `null_generation`; blocked COMMITTING) | DEMO + TEST (Branch C terminal-record cases are in `..PC6d_reconciliation_branches_ABC..`) |
| AC-008 | Production TTL constant/floor and `created_at`-first staleness | `AC-008-016-017-019-ttl-timestamps-drain-failclosed.*`; `testrun-s2508_pure_core_and_ttl_test.log` | TEST: pure predicate, no observable process behaviour at the boundary |
| AC-009 | B2 delegates to the shared core, no check-then-reserve race | `AC-007-009-015-020-*` (BC-INDEX path blocked/admitted through the same core); `..PC6c_b2_delegates_to_shared_core..` in the admission log | DEMO + TEST |
| AC-010 | VP-147 v1.4 re-baseline: Kani 10/10 | `AC-010-kani-10-of-10.*`, `AC-010-kani-summary.txt`, `AC-010-kani-output.log.gz` | KANI. A real full run (kani 0.68.0, CBMC 6.11.0): `cargo kani -p factory-dispatcher --exact` over the 10 pinned harnesses, exit 0, 10 VERIFICATION:- SUCCESSFUL, covers 10/10 summaries all satisfied, 0 UNREACHABLE in the proof file, 10 declared = 10 pinned. The ~15 min solve is too long for a recording, so the tape replays the captured summary (stated in the tape header) |
| AC-011 | Release on PostToolUse AND PostToolUseFailure; release-leg evaluation order | `AC-003-004-*` (PostToolUseFailure releases the reservation) and `AC-011-031-release-leg-evaluation-order.*` | DEMO + TEST |
| AC-012 | Gate evaluated before `Registry::load`, registry-independent | every admission recording runs with an EMPTY registry (zero hooks) yet still blocks; explicit test `..EC022_gate_is_registry_independent_blackbox` | DEMO (implicit) + TEST |
| AC-013 | `factory_root` anchoring, component-wise classification, out-of-root | out-of-scope path admitted in the verdicts recording; symlink/alias/out-of-root vectors in `..EC023..` tests (admission log) | TEST: alias and out-of-root vectors need relative `CLAUDE_PROJECT_DIR`, `t_real`-unavailable and symlinked roots |
| AC-014 | Shared `resolve_target_path` canonicalization | `..EC024..` tests (admission log) and `..EC024_resolve_target_path..` (pure-core log) | TEST: path-spelling matrix (k)/(l) is table-driven |
| AC-015 | Foreign `migration_id` (not in K) | `AC-007-009-015-020-*` (foreign migration, lock held: plain E-MAINTENANCE-001, no mismatch suffix); `..EC027_029_030_migration_id_knownness..` | DEMO + TEST |
| AC-016 | `reservation_is_stale` v1.20 timestamp table, never-clamped retention | `AC-008-016-017-019-*`; pure-core log | TEST (pure table) |
| AC-017 | `RESERVATION_TTL_BELOW_FLOOR` named error, nothing mutated | same | TEST: the production floor is a compile-time/validated constant; no CLI knob exists to set a sub-floor TTL |
| AC-018 | `E-MAINTENANCE-002` `<cause>`: `invalid_tool_use_id`, `state_integrity`, `io`; wire text | `AC-018-028-030-emaintenance-002-cause-tokens.*`; `..EC026..`, `..EC032_gate_state_fixture_matrix..` in the admission log | DEMO + TEST |
| AC-019 | Drain fails closed on a reservations read error | `AC-008-016-017-019-*` (`..PC6c_drain_fails_closed_on_reservations_read_error`) | TEST: the drain is the coordinator side, driven through the pure core with an injected read error |
| AC-020 | Terminal-record read fails closed, never Branch B | `AC-007-009-015-020-*` suite run (`..PC6d_terminal_record_stat_error_fails_closed_never_branch_b_regression_pin`) | TEST: needs an EACCES/EISDIR record path; shown as a named run beside the B2 demo |
| AC-021..AC-026 | MOVED to S-25.09 (AC-001..AC-006) | n/a | Out of this story's scope |
| AC-027 | Registry error to exit code: E-REG-001 (schema mismatch) exits 2, missing/TOML-parse exit 0 | `AC-027-registry-error-exit-codes.*`; `testrun-s2508_registry_exit_code_test.log` | DEMO + TEST (E-REG-002/003 are not reachable from a fixture, covered by the exhaustive-variant tests) |
| AC-028 | Three-way `.factory` stat: Found / Absent / Unstatable | `AC-018-028-030-*` (regular-file `.factory` admitted out of scope; self-loop symlink fails closed `io`) | DEMO + TEST |
| AC-029 | Tier 0 / Tier 1 txn-record interpretation, "absent is not null" | `..ADR052_v123_tier1_branch_b_..`, `..EC029_minimal_shape_foreign_txn..` in the admission log | TEST: record-shape matrix (absent key vs number/bool/array/object) is table-driven |
| AC-030 | Evaluation order: `.factory` stat before `tool_use_id` check, id check only for in-scope writes | `AC-018-028-030-*` (ELOOP + `../x` gives `io`, out-of-scope + `../x` admitted, in-scope + `../x` gives `invalid_tool_use_id`) | DEMO + TEST |
| AC-031 | Release leg: ENOENT vs other stat error; valid-id-only warn | `AC-011-031-release-leg-evaluation-order.*` | DEMO (process boundary) + TEST (the warn itself) |

## Release-leg evaluation order, stated precisely

Order on the release leg: completion-event check, then `tool_use_id` presence + grammar (payload only), then `resolve_factory_root`, then the `migration-state` probe, then unlink.

- Absent, non-string, `../x`, empty or over-length id: FULLY SILENT (no filesystem access, no warn), even when `migration-state` is a self-looping symlink.
- Valid id + unstatable `migration-state` (ELOOP/EACCES/...): exactly ONE non-fatal `tracing::warn!` (target `bc_1_18_011_migration`, `error_kind` not `NotFound`, `path` names `migration-state`); dispatch outcome unchanged (exit 0).
- Valid id + ENOENT: silent no-op.

The warn is a `tracing` event and the production dispatcher installs no subscriber (S-25.09 replaces that channel), so the warn-versus-silent split is NOT observable at the process boundary. The recording therefore shows the process-boundary half (exit 0 for all four ids, identical) and the captured-subscriber tests that assert the split:
`..L6_004_release_{absent,traversal,empty_overlong_nonstring}_id_is_silent_before_any_stat`, `..L6_004_release_absent_or_invalid_id_with_unstatable_dot_factory_is_silent`, `..L5_001_release_stat_error_emits_exactly_one_warn` (valid id + ELOOP, both completion events), `..L5_001_release_enoent_migration_state_emits_no_warn`, plus the 4 probe tests in `bc_1_18_013_release_stat_error_test` (`probe_{absent,present,eloop,eacces}`).

## Notes

- `test_BC_1_18_011_PC6c_b2_delegates_to_shared_core_no_check_then_reserve_race` failed once (18 s) while a Kani solve was saturating the machine. That failure was a wall-time dependency in the test, not a code defect, and it is resolved: the race test now synchronizes on the admission seam's markers instead of wall time, and the seam's wait on resume has a hang bound instead of a fixed 30 s cap. The run recorded in `testrun-s2508_admission_blackbox_test.log` (48 passed) is the evidence for the admission suite.
- `bc_1_18_011_b2_migration_crash_injection_test` compiles to 0 tests without its feature gate and is not cited as evidence.
