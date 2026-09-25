# PR #842 Consolidated Review Trail — S-25.02 Cluster-5 (Mechanism-B2 BC-INDEX Sharding, BC-1.18.010/011)

**Persisted:** 2026-09-25 (state-manager, single-commit TD-VSDD-053; D-1240 post-merge burst)
**Status:** MERGED — `develop` @ `ddd99212` (base `ebd16f79`), feature branch `feature/S-25.02-b2-sharding` deleted

> **Why this file exists.** PR #842's review history spans two distinct phases across
> multiple pr-reviewer cycles and security-review passes. The full trail previously lived
> only in session/conversation history, not in `.factory/` — a traceability gap identified
> and closed this burst (see `cycles/v1.0-brownfield-backfill/lessons.md`
> `L-BB-D1240-review-trail-persistence`). This file is the consolidated, persisted record.
> Individual per-cycle artifacts already on disk under this directory (e.g.
> `pr-review-cluster-5-cycle-1.md`, `pr-description-cluster-5.md`) remain the detailed
> per-cycle record for the cycle(s) they cover; this file is the cross-cycle index and
> narrative summary, not a replacement for them.

## Timeline Summary

| Phase | Scope | Cycles | Converged head | Outcome |
|-------|-------|--------|-----------------|---------|
| Phase 1 | Rename-retry mechanism (B2 first-level + second-level sub-shard addressing, governed migration) | 5 pr-reviewer cycles + 3 dedicated security-review passes | `4cbcbf3d` | APPROVE, 0 BLOCKING remaining |
| Phase 2 | Windows CI re-failure → root-cause correction → Finding C spec-conflict adjudication | continuation from Phase 1's converged head through the Windows fix + revert + Finding C resolution | `6863611f` | CI fully green, merged |

Final state: CI all-17-jobs-green; PR #842 squash-merged into `develop` as `ddd99212`.

## Phase 1 — Rename-Retry Mechanism Convergence (→ `4cbcbf3d`)

Five pr-reviewer review cycles plus three dedicated security-review passes were run against
the B2 addressing + governed-migration mechanism (BC-1.18.010/BC-1.18.011) before the
Windows CI issue surfaced. Two security regressions were caught and fixed before any code
shipped past review:

- **SEC-001 (Finding A) — v1: permanent-deadlock regression.** An early hardening pass on
  the migration's writer-exclusion / admission-gate logic introduced a code path that could
  permanently deadlock the gate under a specific interleaving (a coexisting-transaction /
  lock-acquisition ordering issue in the same family as the crash-recovery arms hardened
  under `[D-1232-OBL-1]`). Caught by a security-review pass before merge; fixed.
- **SEC-001 (Finding B) — v2: resume-path data-loss regression.** The FIX for the v1
  permanent-deadlock issue itself introduced a second regression on the crash-resume path —
  a data-loss-risk window where a resumed migration could proceed past a point it should
  have blocked at, if the v1 fix's ordering interacted with an in-flight resume. Caught by a
  subsequent security-review pass; fixed. (This v1→v2 sequence — a fix for one defect
  introducing a second, related defect on an adjacent code path — is the same fix-induced-
  regression pattern already codified multiple times for the ADR-052 concurrency core; see
  `decision-log.md` D-1225/D-1228/D-1229 and `lessons.md` L-BB-D1229.)
- **Cycle-5 (final Phase-1 cycle): APPROVE.** After both SEC-001 arms were closed and the
  B2 sub-shard chunk-boundary algorithm (ADR-051 §Decision 18) and OBL-1 crash-recovery
  discharge (Kani 7/7 PROVED + 30/30 fault-injection, D-1239) landed, the fresh-eyes
  pr-reviewer cycle converged with an APPROVE verdict and 0 BLOCKING findings remaining.
  The persisted `pr-review-cluster-5-cycle-1.md` artifact in this directory (dated
  2026-09-23, pre-dating the Windows CI discovery) reflects this converged, APPROVE state:
  no BLOCKING findings; 2 SUGGESTIONs (WAL-append durability asymmetry vs. staging writes,
  scoped to the already-tracked `[D-1232-OBL-2]` APFS durability obligation — not a merge
  blocker; a config-sourced `ss_id` field lacking the sibling `artifact_stem` field's
  SEC-002 traversal guard, mitigated by the field having no production caller yet) and 2
  NITs (a provably-safe `.expect()` pair; a `kani.yml` cache-path minor CI-time waste).
  Self-approval was environmentally blocked (`gh` identity == PR author `Zious11`), so the
  verdict was recorded via a `gh pr review --comment` review event carrying the APPROVE
  verdict, escalated to the human merge authority.

Phase 1 converged at commit `4cbcbf3d` with all BLOCKING findings closed and the security
regressions (SEC-001 v1 + v2) both caught and fixed pre-merge — the review process working
exactly as intended (catch-before-ship), at the cost of two additional fix-and-re-review
cycles.

## Phase 2 — Windows CI Re-Failure Through Merge (`4cbcbf3d` → `6863611f`)

After Phase 1's convergence, `develop` CI re-ran and the `build-dispatcher (windows-x64)`
job failed (5 integration tests, `write_txn_record → Io{PermissionDenied, os error 5}`).
This phase's own sequence, fully codified in `decision-log.md` D-1240:

1. **Initial misdiagnosis (superseded).** The session-wrap-pause checkpoint
   (D-1239/STATE.md v10.73) attributed the failure to `write_atomic_strict_durable` holding
   a temp-file handle open across `std::fs::rename`. During the fix session this was
   further (mis)theorized as a Windows Defender antivirus scan-handle race, and a
   speculative 8-primitive retry/backoff hardening sweep was written against that theory,
   landing as local commit `465585e3` (never pushed).
2. **Root-cause correction (D-1240(a)).** A fresh-eyes pass pulled the actual CI failure
   log and read the exact failing code path, finding the TRUE, fully deterministic defect:
   `StdFs::fsync_file` opened the target file **read-only**, then called `FlushFileBuffers`
   on that handle — Windows requires a write-access (`GENERIC_WRITE`) handle for
   `FlushFileBuffers`; a read-only handle fails on every invocation, with zero timing
   dependency. Not an antivirus race.
3. **Human scope decision (D-1240(b)).** The speculative 8-primitive AV-retry sweep
   (`465585e3`) was REVERTED in favor of the minimal, targeted fix (correct `fsync_file`'s
   handle-open mode), using the hook-safe local-revert recipe (`git stash` →
   `git reset --soft <base>` → `git stash`) since `destructive-command-guard` blocks a raw
   `git reset --hard`.
4. **Finding C (D-1240(c)) — BC-1.18.002 fail-open posture.** A PR-diff-level review,
   walled off from `.factory/` spec content by design, flagged
   `indeterminate_marker::block_if_marker_check`'s fail-open behavior on a non-`NotFound`
   marker-read I/O error as CWE-703 (Improper Check for Exceptional Conditions). An
   implementer change ("Item 3") tightened the behavior to fail-closed. Adjudicated
   INCORRECT on review: BC-1.18.002 v1.8 Invariant 2 / EC-030 **deliberately** requires
   fail-open here to avoid an unclearable self-lock (the same permission fault that makes
   the marker unreadable could also make the operator's `rm` escape hatch unusable,
   violating INV6's Ungated-Escape guarantee). Per the CLAUDE.md Standing Rule
   ("for code-vs-spec conflicts, the spec wins"), **Item 3 was REVERTED** pre-merge. The
   underlying security-vs-availability tradeoff is a genuine, legitimate question — filed as
   the human-directed follow-up story **S-25.07** ("Revisit BC-1.18.002 INV2/EC-030
   Marker-Read I/O-Error Fail-Open Posture"; E-25; 8 pts; P2; draft).
5. **Shipped fixes.** Two net code changes landed: the `fsync_file` read-only-handle fix
   (item 2 above) and a second, related Windows-path fix in `rotate_changelog_at`
   (discovered during the same comprehensive windows-fs audit). Item 3 (the marker
   fail-open→fail-closed change) did NOT ship — reverted per Finding C.
6. **Convergence.** With both shipped fixes green and Item 3 reverted, CI reached
   **all 17 jobs green**, converging at commit `6863611f`.

## Final Disposition

| ID | Description | Disposition |
|----|-------------|-------------|
| SEC-001 (Finding A, v1) | Permanent-deadlock regression in writer-exclusion/admission-gate logic | FIXED pre-merge (Phase 1) |
| SEC-001 (Finding B, v2) | Resume-path data-loss regression introduced by the v1 fix | FIXED pre-merge (Phase 1) |
| Finding C | `indeterminate_marker::block_if_marker_check` fail-open flagged as CWE-703; "Item 3" fail-closed change | REVERTED — BC-1.18.002 v1.8 INV2/EC-030 fail-open REAFFIRMED (spec wins); follow-up filed as **S-25.07** |
| Windows CI (`build-dispatcher windows-x64`) | Root cause: `StdFs::fsync_file` read-only-handle + `FlushFileBuffers` write-access requirement (NOT an antivirus race — that was an intervening misdiagnosis) | FIXED (`fsync_file` handle-mode correction + `rotate_changelog_at` fix); speculative AV-retry sweep (`465585e3`) REVERTED, never shipped |

**Merge:** PR #842 squash-merged into `develop` as `ddd99212` (base `ebd16f79`) 2026-09-25.
Feature branch `feature/S-25.02-b2-sharding` deleted. POL-14 auto-promotion: BC-1.18.010 +
BC-1.18.011 `status`/`lifecycle_status` draft→active (both v1.10 UNCHANGED). `merged_count`
122→123.

## Cross-References

- Full sub-decision codification: `cycles/v1.0-brownfield-backfill/decision-log.md` D-1240
  (and D-1240(a)/(b)/(c))
- Lessons: `cycles/v1.0-brownfield-backfill/lessons.md`
  `L-BB-D1240-diagnosis-discipline-pull-log-before-theorizing`,
  `L-BB-D1240-sibling-sweep-all-fresh-file-operations`,
  `L-BB-D1240-spec-before-fix-verify-deliberate-invariant`,
  `L-BB-D1240-tempfile-hygiene-use-scratchpad-not-shared-tmp`,
  `L-BB-D1240-destructive-command-guard-local-revert-recipe`,
  `L-BB-D1240-review-trail-persistence`
- Prior-arc obligations (unaffected by this merge, activation-boundary-scoped, do NOT gate
  this merge — migration ships DORMANT): `[D-1232-OBL-2]`/`[D-1232-OBL-3]`/`[D-1232-OBL-4]`
  in `STATE.md` `## Blocking Issues`
- Follow-up story: `.factory/stories/S-25.07-revisit-bc-1-18-002-marker-read-io-error-fail-open-posture.md`
- Per-cycle artifacts (this directory): `pr-description-cluster-5.md`,
  `pr-review-cluster-5-cycle-1.md`
