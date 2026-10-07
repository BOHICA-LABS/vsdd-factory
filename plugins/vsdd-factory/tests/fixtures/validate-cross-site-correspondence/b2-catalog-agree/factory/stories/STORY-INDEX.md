---
document_type: story-index
version: "4.278"
last_amended: "2026-07-31 (v4.278)"
---

# STORY-INDEX

<!-- Fixture: B2 control — STORY-INDEX internal: catalog 47a65c9, blockquote 47a65c9 (agree) -->
<!-- Trigger: this file is the PRIMARY target (PostToolUse on STORY-INDEX.md) -->
<!-- Expected: Class B Arm2 passes, exit 0 -->
<!-- BC-5.39.010 EC-007: STORY-INDEX written; blockquote S-21.07=47a65c9; catalog 47a65c9 → Continue -->
<!--                                                                                                  -->
<!-- PRODUCTION SHAPE: blockquote is ONE prose line with semicolon-separated entries.                 -->
<!-- F-S2107-P1B-004: run_arm_b2 calls rest.find('=') finding FIRST '=' in the prose line.           -->
<!-- This produces a garbage story_id (the entire prefix before "S-21.01="), triggering an           -->
<!-- "orphaned blockquote entry" violation → spurious EXIT 2 on this CONTROL.                        -->
<!-- Current code: EXIT 2 (spurious block). Test expects EXIT 0 → FAILS → RED gate.                  -->
<!-- After fix: correctly extracts S-21.07=47a65c9 from prose line → no violation → EXIT 0.          -->

| Story ID | Title | Epic | Points | Priority | depends_on | blocks | status | Notes |
|----------|-------|------|--------|----------|------------|--------|--------|-------|
| S-21.01 | validate-factory-path-staging WASM guard | E-21 | 11 | P1 | [] | [] | merged | [BC-4.16.001 v1.8] (wave 1; subsystems SS-04; tdd_mode strict; input-hash 32aaccc) |
| S-21.07 | validate-cross-site-correspondence WASM hook — five-arm PostToolUse cross-site value-correspondence gate (Classes A/B/D/E; BC-5.39.010 v1.2) | E-21 | 11 | P1 | [] | [] | draft | [BC-5.39.010 v1.2] (wave 4; subsystems SS-04, SS-05; tdd_mode strict; input-hash 47a65c9; AC-001..AC-021) |

> **E-21 delivery:** W1 (parallel): {S-21.01 (11 pts), S-21.02 (3 pts), S-21.03 (3 pts)} → W2 (parallel): {S-21.04 (5 pts), S-21.05 (5 pts)} → W3 (sequential): {S-21.06 (8 pts; depends_on [S-21.01])} → W4 (independent): {S-21.07 (11 pts; depends_on [])}. 7 stories total. 46 pts. DAG has no cycles. Input-hashes: S-21.01=32aaccc; S-21.02=8bd32e5; S-21.03=59e687e; S-21.04=a1b2c3d; S-21.05=c9265f0; S-21.06=b807086; S-21.07=47a65c9. All 7 distinct. [Fixture: all values agree]
