---
document_type: blocking-issues-resolved
level: ops
version: "1.3"
status: archive
producer: state-manager
timestamp: 2026-08-26T18:30:00Z
cycle: "v1.0-brownfield-backfill"
inputs: [STATE.md]
input-hash: "e3703fa"
traces_to: STATE.md
---

# Resolved Blocking Issues — v1.0-brownfield-backfill

<!-- Blocking issues that were resolved and archived from STATE.md.
     Open blocking issues remain in STATE.md. -->

| ID | Issue | Severity | Blocked Phase | Owner | Resolution | Resolved Date |
|----|-------|----------|--------------|-------|------------|---------------|
| [D-1248-CARGO-DENY] | cargo-deny advisories gate failed on develop (wasmtime 46.x RUSTSEC-2026-0316/0314/0321/0322/0323/0324/0327) — blocked every PR's CI | HIGH — blocked all PR CI | all PR merges | devops-engineer / implementer | RESOLVED D-1250 — PR #844 (wasmtime/wasmtime-wasi 46.0.3→48.0.5; component model disabled; floor guards >=48.0.4 <49) squash-merged to develop as ce2421be; 18/18 CI checks green incl. deny-advisories; 49.x deferred to S-6.02. | 2026-10-07 |
| D-989 | PR #775 review refresh + full CI green (windows-x64 portability fix `c20cf2fe` needed re-verification) | HIGH — human-gated next step | S-21.09 merge readiness | pr-reviewer / code-reviewer / CI | PR #775 head `c20cf2fe` confirmed `mergeable: MERGEABLE`, `mergeStateStatus: CLEAN`; CI fully green all platforms incl. `build-dispatcher (windows-x64)` (13 pass + 1 correctly-skipped release-branch-guardrail); security-reviewer CLEAR (C-1 non-implicated, sole production diff is a `#[cfg(test)]` unit test in `registry.rs`); pr-reviewer APPROVE merge-ready against actual head `c20cf2fe`. Superseded by `[D-990] PR #775 MERGEABLE/CLEAN, awaiting human merge via GitHub UI` (STATE.md Blocking Issues). | 2026-08-13 |
| [D-1057] | S-21.13 depends_on redirect `[S-21.10,S-21.11]`→`[S-21.10,S-21.22]` OWED | LOW — bookkeeping accuracy | Wave 7 story-graph accuracy | story-writer | Redirect EXECUTED at D-1070 (story-layer 'story-remediation' burst): S-21.13's own catalog row + frontmatter now read `depends_on [S-21.10, S-21.22]`, correctly targeting S-21.22 (owner of `validate-cross-site-correspondence`) instead of the superseded S-21.11. | 2026-08-22 |
| [D-1057] | S-21.16 depends_on redirect `[S-21.11]`→`[S-21.24]` OWED | LOW — bookkeeping accuracy | Wave-null story-graph accuracy | story-writer | Redirect EXECUTED at D-1070 (story-layer 'story-remediation' burst): S-21.16's own catalog row + frontmatter now read `depends_on [S-21.24]`, correctly gating CI-lint hardening on the capstone's completed flip instead of the now-superseded S-21.11. | 2026-08-22 |
| [D-1056] | HUMAN CONVERGENCE + SIZING-DECISION gate — S-21.11 v2.11 spec bundle CONVERGED, Phase-3 TDD entry pending human approval | HIGH — human-gated | Phase-3 TDD entry | human / orchestrator | RESOLVED D-1057 — operator OVERRODE to SPLIT. BC-5.39.001 3-CLEAN achieved (passes 14/15/16). Human resolved all three gate items: (a) sizing OVERRIDDEN to split (not keep-unified); (b) cosmetic sweep RESOLVED-BY-SUPERSESSION (S-21.11 frozen/historical); (c) Phase-3 TDD entry now requires each of 7 new split stories to independently reach 3-CLEAN. | 2026-08-20 |
| [P0] | POLICY 15 ATTESTATION-LOCATION GATE — BOTH HALVES CLOSED D-1015 | P0 SECURITY | POLICY 15 gate enforcement | devops-engineer / CI | CLOSED D-1015. Crate (PR #777) + CI-wiring (PR #778) merged; gate deployed. PR #779 closes empty-range residual (ADR-040 v1.19). `[D-969]`/`[F-S2107-P10-001]` fully CLOSED. Branch-protection gap tracked separately as `[P0-followup]` (still OPEN in STATE.md). | 2026-08-16 |
| [ADR-045] | HUMAN RATIFICATION GATE — stable-anchor cross-reference architecture proposed at v1.0 (D-1081); Wave-7 pre-TDD cascade HELD pending ratification | HIGH — human-gated | Wave-7 pre-TDD cascade unblock | human / architect | RATIFIED — human ACCEPTED ADR-045, pivoting the design from the originally-proposed stable-anchor/strip model to a FROZEN-PROVENANCE + suspect-link architecture; landed at v1.3. Superseded by `[ADR-045] v1.3 ACCEPTED but ratification-recording burst OWED` (STATE.md Blocking Issues) — the amendment-application burst (POLICY 7/8/14/17/19 → policies.yaml, decision-log D-NNN, BC-INDEX/ARCH-INDEX rows) was not yet performed as of the 2026-08-26 session wrap; E-23 (built for the abandoned strip model) requires re-scope to the frozen-provenance model. | 2026-08-26 (recorded at wrap; exact ratification date within the 2026-08-24..2026-08-26 window not independently pinned — see architect's next ADR-045 touch for precise dating) |
