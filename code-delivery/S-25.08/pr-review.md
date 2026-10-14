---
document_type: pr-review-record
level: ops
title: "S-25.08 PR #848 pr-reviewer review (verbatim, durable record)"
producer: pr-reviewer
persisted_by: state-manager
timestamp: 2026-10-13T00:00:00Z
phase: per-story-step-pr-review
story: S-25.08
pr: 848
covered_sha: "0387b33ba30134bd627ae5d89e142f69d40671dd"
verdict: REQUEST_CHANGES
date: 2026-10-13
provenance: "Written by pr-manager for PR #848. The GitHub review-comment post was permission-denied, so this file is the durable record. The body below is verbatim; only this frontmatter was added by the state-manager (D-1264)."
traces_to: S-25.08
---

## PR review: S-25.08 shared admission core, B2 conformance, VP-147 v1.4 re-baseline

**Verdict: REQUEST_CHANGES** (1 BLOCKING, about the description only; the code itself is not blocked)
**covered_sha:** `0387b33ba30134bd627ae5d89e142f69d40671dd`

Scope: every changed file in the diff (68 files, +14,161 / -773, 79 commits). I read the production changes line by line: `main.rs`, `executor.rs`, `shard_manager.rs`, the new `shard_manager/admission.rs`, `payload.rs`, `invoke.rs`, `registry.rs`, the four workflow files, `CHANGELOG.md`, the `vp147-mutants` crate and `mutants.toml`. I spot-checked the test suites (48 black-box tests, none `#[ignore]`d, with real assertions such as the T7/T8 release-on-block cases). I checked the demo evidence (README, AC map, Kani summary, testrun logs) against the PR body. As instructed, I did not re-examine the human acceptance of the 14-pass LOCAL cascade.

CI was still pending on every check when I reviewed (`kani`, both `cargo-host`, bats, deny, and the others). The PR correctly does **not** claim T-6/T-14, and the Kani claim boundary is stated as LOCAL until the `kani` job is green. Merge still has to wait for CI whatever this review says.

### Findings

| # | Severity | Category | Finding | Suggestion |
|---|----------|----------|---------|------------|
| B1 | **BLOCKING** | description | The "Review convergence statement (honest)" says "no code-level findings since pass 5". That is wrong. Pass 6 finding **F-S2508-L6-004** produced red tests `faa9d9ef` and the production fix `f34e349c`, "fix(S-25.08): release leg checks tool_use_id before any stat". That fix changes `bc_index_migration_reservation_release` in `executor.rs`: the order of evaluation and when it touches the filesystem. `9e936d4c` (pass 9) also changed a production panic string. This section is the one a reviewer and the human rely on for the convergence picture, so it needs to be exact. | Change it to "no code-level findings since pass 6" (or whatever the pass reports say exactly) and keep the rest. The story's own human-acceptance record (S-25.08 v1.15 `last_amended`) has the same sentence. Route that to story-writer/state-manager so the two stay consistent. |
| N1 | NON-BLOCKING | missing / ops | The reservation namespace is now unconditional (EC-020). Every protected `Edit`/`Write`/`MultiEdit` therefore creates `.factory/migration-state/reservations/<id>.reservation`. Release-on-block in `main.rs` only covers exits this process decides (`code == 2`). A reservation leaks until the 3,600 s TTL when the tool never runs for a reason outside the dispatcher: another PreToolUse hook denies, the user rejects the permission prompt, or the session is interrupted. As far as I know, none of those emit `PostToolUse`/`PostToolUseFailure`. That has two effects. (a) In a factory whose `.factory/` is the `factory-artifacts` worktree, which has no `.gitignore`, a leaked reservation is an untracked file that a broad `git add` could commit, and it trips the "factory-artifacts dirty" check. (b) For up to an hour, any coordinator drain hits `DRAIN_TIMEOUT_ABORT` (30 s drain timeout). (b) is the spec's retention-biased trade-off, so it is not blocking. (a) is not addressed anywhere I can see. | Add an ignore rule for `migration-state/reservations/`, for example a `.gitignore` written when the namespace is created, or an entry on the factory branch. Also document the leak and stall case for operators. Route to architect if the namespace layout is spec-owned. |
| N2 | NON-BLOCKING | coherence / diagnostics | The new registry-independent admission early return (`main.rs`, `return Ok(exit_code_for_native_gate_verdict(..))` right after admission) prints only the indented `plugins_run=… block_intent=true … block_reason=…` continuation line. The `factory-dispatcher trace=<id> event=… tool=…` header is printed only after `Registry::load`. An `E-MAINTENANCE-001/002` block therefore reaches stderr with no trace id, and it leaves no InternalLog record. Before this PR the admission verdict was reported after the header. The trace UUID is the documented diagnostic handle. | Print the header (with `sync_plugins=0 async_plugins=0` or an explicit marker) before the early return. If S-25.09 (AC-022 InternalLog admission events) is meant to own this, say so in the PR body and add a test pin. |
| N3 | NON-BLOCKING | API hygiene | `bc_index_migration_admission_precheck` is documented as the "verdict-only form", but it calls the full core. When an admission with a `tool_use_id` succeeds, it **creates a reservation and drops its path**, so any future production caller would leak a reservation until TTL. Only tests call it today. | Fix the doc comment (state the side effect and that the caller can't release it), or restrict it with `#[doc(hidden)]` or `#[cfg(test)]`-style gating so it can't become a production call site. |
| N4 | NON-BLOCKING | evidence | T-12 claims "Its run gave 6/6 mutants KILLED", but the PR adds no artifact for that run and does not say it was local or name the SHA. Also, `vp147-mutants.yml` is `workflow_dispatch`-only, and GitHub cannot dispatch it until the file is on the default branch (`gh run list --workflow vp147-mutants.yml` returns 404 "not found on the default branch"). It will not run in CI until a release reaches `main`. | Say "local run at `<sha>`". Commit the `VP147-SUMMARY …` line and the per-mutant `VERDICT=` lines under `docs/demo-evidence/S-25.08/`. Note the dispatch limitation. |
| N5 | NON-BLOCKING | evidence freshness | The demo README "Notes" call a once-failing `..._no_check_then_reserve_race` run "load-induced timing, not a code defect". Later, `3eb7a6b7` changed that test to sync on seam markers and `b89cbb6d` changed the seam to a hang bound with a loud failure. The README and `testrun-*.log` were recorded at `9e936d4c`, so the admission testrun log predates the test-file change. | Update the note to say what was fixed and where. Ideally re-capture `testrun-s2508_admission_blackbox_test.log` at `0387b33b`. |
| N6 | NON-BLOCKING | description | "T-11(d): satisfied by the local Kani evidence above plus CI green" sits under "Open pre-merge tasks (not claimed done)", but the word "satisfied" reads as a claim. | Change it to "T-11(d): pending, needs CI `kani` green (local evidence attached)." |
| N7 | NON-BLOCKING | correctness (future-proofing) | `BcIndexMigrationError::admission_failure_cause` ends in `_ => "state_integrity"`. The PR's registry mapping deliberately avoids a wildcard. Here, a future I/O-like variant would silently be reported as `state_integrity` instead of `io`. | List the variants explicitly, the same way as `registry_error_exit_code`. |
| N8 | NIT | docs | The comment on the `test-support` feature in `crates/factory-dispatcher/Cargo.toml` (not in the diff) still says "Sole wired consumer: VSDD_ASYNC_DRAIN_WINDOW_MS". The feature now also compiles in the `VSDD_TEST_ADMISSION_SEAM_DIR` admission seam (`admission.rs` `cfg`). | Update the comment, because the release-build guarantee (DI-019) depends on knowing exactly what this feature turns on. |
| N9 | NIT | size | +14,161 lines, far above the ~500-line guideline. About 8.6k are tests, about 1.3k are Kani harnesses and about 1.3k are the mutant-runner crate; the production dispatcher change is about 2.5k. That is acceptable for an atomic story, but I'm noting it. | None required. |

### What I verified with no findings

- **Evaluation order (`main.rs`).** Admission and release run immediately after the payload parse and `resolve_project_cwd`, before `CLAUDE_PLUGIN_ROOT` tiering and `Registry::load`. `shard_cap_precheck` is reached only when admission did not block, so the destructive roll can't run for a blocked protected write. Every exit of `run` funnels into the one `code == 2` release point.
- **Reserve-then-verify (`admit_protected_write`).** W1 (an atomic reservation write) comes strictly before W2 (the gate and txn reads). A failed verification or an `Err` removes the admitter's own reservation. Reconciliation runs only under `try_lock` on `exclusive.lock`, and EWOULDBLOCK means no action.
- **Branches A/B/C.** Branch B rewrites the raw object in place (fields preserved) with `write_atomic_strict_durable`, txn before gate. Branch C can't finalize while verification inputs are hard-wired to unverified; the unreachable `FinalizeThenOpenGate` arm fails closed instead of panicking. A terminal-record read error is `Io`, never treated as absent. More than one live txn fails loud.
- **Path classification.** Component-wise and case-folded (never a substring test). It uses the real or lexical union with an as-given `CLAUDE_PROJECT_DIR` alias (absolute only), resolves symlinks left to right with a 40-hop limit, treats an unresolvable path as lexical (fail-closed), and leaves `.factory` itself and look-alikes out of scope.
- **`tool_use_id`.** The grammar check blocks separators, a leading dot, NUL, and more than 128 bytes, so the id can't escape `reservations/`. Absent or `null` means check-only; a present but invalid id fails closed. The release leg validates the id before any filesystem access.
- **Staleness.** The predicate is pure: `created_at` first, 300 s skew tolerance, unknown age kept rather than reclaimed, no clamping. The TTL floor is enforced at compile time and at the top of `run_bc_index_migration` before any mutation. The drain now fails closed on a `reservations/` read error other than ENOENT.
- **`registry_error_exit_code`.** The match has no wildcard, and the 2/0 mapping per variant matches the comments.
- **Lenient `tool_name`.** Accepting non-string `tool_name` values is a safe widening: an empty name matches no protected writer or tool filter.
- **`kani.yml`.** The exact-name pin, declared-count check, per-name-ran check, cover-satisfied gate and UNREACHABLE-in-proof-file gate are all sound. The summary evidence adds up to 48/48 covers (8+1+2+20+2+2+3+2+6+2) and 10 SUCCESSFUL.
- **Kani evidence vs head SHA.** The Kani evidence is at `9e936d4c`, while the head is `0387b33b`. The only source change between them is the test-only seam in `admission.rs`, which the proofs don't touch, so the LOCAL proof claim still applies to the head SHA.
- **`ci.yml` failpoints step.** It adds per-target `assert_floor` checks (39 and 1), so a target that compiles to nothing can no longer pass green.
- **`release.yml`.** It excludes `vp147-mutants` and skips its artifacts by name, so the crate is not bundled as a hook plugin.
- **`vp147-mutants` verdict logic.** A compile error or a run where nothing executed is INVALID, never KILLED. A SURVIVED verdict needs every requested harness to report SUCCESSFUL, and the run summary passes only if every mutant is KILLED.
- **Description honesty, other than B1.** 3-CLEAN is not claimed, T-6/T-14 are not claimed, and the claim boundary lists what Kani does not prove. The CHANGELOG states the limitation that there is no operator-visible diagnostics channel until S-25.09.
- **Commits.** All 79 use the conventional format with `(S-25.08)`, and none carry AI attribution.
- **Dependencies.** `depends_on: []`. The stacking note for S-25.09 is clear.
