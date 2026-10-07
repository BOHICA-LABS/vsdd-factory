---
document_type: story-index
version: "4.278"
last_amended: "2026-07-31 (v4.278)"
---

# STORY-INDEX

<!-- Fixture: B2 mutant — STORY-INDEX internal: catalog 47a65c9 ≠ blockquote 4be9d21 -->
<!-- Trigger: this file is the PRIMARY target (PostToolUse on STORY-INDEX.md) -->
<!-- Expected: Class B Arm2 fires, exit 2, [Class B] in block reason -->
<!-- BC-5.39.010 EC-008: STORY-INDEX written; blockquote S-21.07=4be9d21; catalog 47a65c9 | Block -->
<!--                                                                                               -->
<!-- PRODUCTION SHAPE: blockquote is ONE prose line. F-S2107-P1B-004: run_arm_b2 with              -->
<!-- production shape produces a garbage story_id via rest.find('=') on the prose line →          -->
<!-- "orphaned blockquote entry" violation → EXIT 2 (but for wrong reason: spurious block          -->
<!-- rather than the real catalog/blockquote mismatch). After B2 fix: real mismatch detected.     -->
<!-- Either way, EXIT 2 → test PASSES (exit code correct throughout).                             -->

| Story ID | Title | Epic | Points | Priority | depends_on | blocks | status | Notes |
|----------|-------|------|--------|----------|------------|--------|--------|-------|
| S-21.01 | validate-factory-path-staging WASM guard | E-21 | 11 | P1 | [] | [] | merged | [BC-4.16.001 v1.8] (wave 1; subsystems SS-04; tdd_mode strict; input-hash 32aaccc) |
| S-21.07 | validate-cross-site-correspondence WASM hook — five-arm PostToolUse cross-site value-correspondence gate (Classes A/B/D/E; BC-5.39.010 v1.2) | E-21 | 11 | P1 | [] | [] | draft | [BC-5.39.010 v1.2] (wave 4; subsystems SS-04, SS-05; tdd_mode strict; input-hash 47a65c9; AC-001..AC-021) |

> **E-21 delivery:** W1 (parallel): {S-21.01 (11 pts), S-21.02 (3 pts), S-21.03 (3 pts)} → W2 (parallel): {S-21.04 (5 pts), S-21.05 (5 pts)} → W3 (sequential): {S-21.06 (8 pts; depends_on [S-21.01])} → W4 (independent): {S-21.07 (11 pts; depends_on [])}. 7 stories total. 46 pts. DAG has no cycles. Input-hashes: S-21.01=32aaccc; S-21.02=8bd32e5; S-21.03=59e687e; S-21.04=a1b2c3d; S-21.05=c9265f0; S-21.06=b807086; S-21.07=4be9d21. All 7 distinct. [Fixture: blockquote has 4be9d21 but catalog row has 47a65c9 → catalog/blockquote mismatch]
