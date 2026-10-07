---
document_type: story-index
version: "4.278"
last_amended: "2026-07-31 (v4.278)"
---

# STORY-INDEX

<!-- Fixture: T-037 — B3-only mismatch. B1 (story) == B2 (catalog) == "47a65c9",     -->
<!-- but B3 (blockquote) == "deadbee" (diverges from story).                          -->
<!-- Expected: Class B Arm1 fires (B3 ≠ B1), exit 2                                  -->
<!-- BC-5.39.010 v1.3 §Class B: three-way check B1==B2==B3; any mismatch blocks       -->
<!--                                                                                  -->
<!-- F-S2107-P1B-003 RED GATE: parse_story_index_blockquote_hash uses                 -->
<!-- starts_with("> S-21.07=") which never matches the production prose line.          -->
<!-- B3 = None (permanently inert). B1==B2 → no violation → EXIT 0.                  -->
<!-- Test expects EXIT 2 → FAILS → RED gate (post-implementation, pre-B3-fix).         -->
<!-- After B3 fix: B3=Some("deadbee") ≠ B1="47a65c9" → violation → EXIT 2 ✓          -->

| Story ID | Title | Epic | Points | Priority | depends_on | blocks | status | Notes |
|----------|-------|------|--------|----------|------------|--------|--------|-------|
| S-21.01 | validate-factory-path-staging WASM guard | E-21 | 11 | P1 | [] | [] | merged | [BC-4.16.001 v1.8] (wave 1; subsystems SS-04; tdd_mode strict; input-hash 32aaccc) |
| S-21.07 | validate-cross-site-correspondence WASM hook — five-arm PostToolUse cross-site value-correspondence gate (Classes A/B/D/E; BC-5.39.010 v1.2) | E-21 | 11 | P1 | [] | [] | draft | [BC-5.39.010 v1.2] (wave 4; subsystems SS-04, SS-05; tdd_mode strict; input-hash 47a65c9; AC-001..AC-021) |

> **E-21 delivery:** W1 (parallel): {S-21.01 (11 pts), S-21.02 (3 pts), S-21.03 (3 pts)} → W2 (parallel): {S-21.04 (5 pts), S-21.05 (5 pts)} → W3 (sequential): {S-21.06 (8 pts; depends_on [S-21.01])} → W4 (independent): {S-21.07 (11 pts; depends_on [])}. 7 stories total. 46 pts. DAG has no cycles. Input-hashes: S-21.01=32aaccc; S-21.02=8bd32e5; S-21.03=59e687e; S-21.04=a1b2c3d; S-21.05=c9265f0; S-21.06=b807086; S-21.07=deadbee. All 7 distinct. [Fixture: catalog=47a65c9 matches story; blockquote=deadbee diverges → B3-only mismatch]
