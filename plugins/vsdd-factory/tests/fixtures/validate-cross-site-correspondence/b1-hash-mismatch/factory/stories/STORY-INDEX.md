---
document_type: story-index
version: "4.278"
last_amended: "2026-07-31 (v4.278)"
---

# STORY-INDEX

<!-- Fixture: AC-009 mutant — B2≠B3 (STORY-INDEX internal inconsistency → PC13b BLOCK) -->
<!-- B1 = story frontmatter input-hash = "47a65c9" -->
<!-- B2 = catalog row input-hash = "4be9d21"  (STORY-INDEX catalog) -->
<!-- B3 = blockquote S-21.07=c3f9811 (MUTANT — B3 ≠ B2; STORY-INDEX internally inconsistent) -->
<!-- B2 ≠ B3: catalog and blockquote disagree → no burst-ordering explanation → BLOCK -->
<!-- Expected: Class B Arm1 fires (PC13b), exit 2, [Class B], POLICY 18 (D-923), -->
<!-- provenance categories (stale/fabricated/algorithm-divergent) enumerated without asserting one -->
<!-- BC-5.39.010 v1.11 PC13b TV: "B Arm1 — B2≠B3 | B1=47a65c9; B2=4be9d21; B3=c3f9811 | Block" -->

| Story ID | Title | Epic | Points | Priority | depends_on | blocks | status | Notes |
|----------|-------|------|--------|----------|------------|--------|--------|-------|
| S-21.01 | validate-factory-path-staging WASM guard | E-21 | 11 | P1 | [] | [] | merged | [BC-4.16.001 v1.8] (wave 1; subsystems SS-04; tdd_mode strict; input-hash 32aaccc) |
| S-21.07 | validate-cross-site-correspondence WASM hook — six-arm PostToolUse cross-site value-correspondence gate (BC-5.39.010 v1.11) | E-21 | 11 | P1 | [] | [] | draft | [BC-5.39.010 v1.11] (wave 4; subsystems SS-04, SS-05; tdd_mode strict; input-hash 4be9d21; AC-001..AC-023) |

> **E-21 delivery:** W1 (parallel): {S-21.01 (11 pts), S-21.02 (3 pts), S-21.03 (3 pts)} → W2 (parallel): {S-21.04 (5 pts), S-21.05 (5 pts)} → W3 (sequential): {S-21.06 (8 pts; depends_on [S-21.01])} → W4 (independent): {S-21.07 (11 pts; depends_on [])}. 7 stories total. 46 pts. DAG has no cycles. Input-hashes: S-21.01=32aaccc; S-21.02=8bd32e5; S-21.03=59e687e; S-21.04=a1b2c3d; S-21.05=c9265f0; S-21.06=b807086; S-21.07=c3f9811. All 7 distinct. [Fixture: AC-009 B2≠B3 mutant: catalog=4be9d21; blockquote=c3f9811 → STORY-INDEX internal inconsistency]
