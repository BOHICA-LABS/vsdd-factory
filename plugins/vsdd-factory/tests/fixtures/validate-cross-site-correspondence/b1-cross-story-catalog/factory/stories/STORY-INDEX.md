---
document_type: story-index
version: "4.278"
last_amended: "2026-07-31 (v4.278)"
---

# STORY-INDEX

<!-- Fixture: T-038 — cross-story catalog false-match (F-S2107-P1B-008)              -->
<!-- Story: S-18.01-test.md with input-hash "1b4ea21" (B1)                           -->
<!-- STORY-INDEX: S-18.00 row appears BEFORE S-18.01 and has "S-18.01" in blocks.    -->
<!-- Naive contains("S-18.01") matches S-18.00 row first, returns "e5bc551" (B2).    -->
<!-- B1="1b4ea21" ≠ B2="e5bc551" → spurious violation → EXIT 2.                     -->
<!-- Expected: EXIT 0 (correct row for S-18.01 with hash "1b4ea21" should be found). -->
<!--                                                                                  -->
<!-- F-S2107-P1B-008 RED GATE: current code uses naive contains(story_id) →          -->
<!-- finds S-18.00 row → returns wrong hash → spurious EXIT 2.                       -->
<!-- Test expects EXIT 0 → FAILS → RED gate (post-implementation, pre-fix).           -->
<!-- After fix: exact prefix match on story ID → S-18.01 row found → EXIT 0 ✓        -->

| Story ID | Title | Epic | Points | Priority | depends_on | blocks | status | Notes |
|----------|-------|------|--------|----------|------------|--------|--------|-------|
| S-18.00 | Dispatcher PreCompact/PostCompact Routing | E-18 | 8 | P0 | [] | [S-18.01, S-18.04a, S-18.05] | merged | [BC-1.15.001 v1.6] (wave 1; subsystems SS-01; tdd_mode strict; input-hash e5bc551) |
| S-18.01 | HANDOFF.md Schema + wave-handoff Skill | E-18 | 13 | P0 | [S-18.00] | [S-18.02, S-18.08] | merged | [BC-5.41.001 v1.18, BC-5.41.002 v1.20] (wave 2; subsystems SS-05; tdd_mode strict; input-hash 1b4ea21) |

> **E-18 delivery:** W1: {S-18.00 (8 pts)} → W2: {S-18.01 (13 pts)} → ... Input-hashes: S-18.00=e5bc551; S-18.01=1b4ea21. [Fixture: S-18.00 row has S-18.01 in blocks column to expose F-S2107-P1B-008 naive-contains false match]
