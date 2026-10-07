---
document_type: story-index
version: "4.278"
last_amended: "2026-07-31 (v4.278)"
---

# STORY-INDEX

<!-- Fixture: B1 control — all three sites agree (PRODUCTION blockquote shape) -->
<!-- B1 = story frontmatter input-hash = "47a65c9" -->
<!-- B2 = catalog row input-hash = "47a65c9" -->
<!-- B3 = blockquote S-21.07=47a65c9 (embedded in production prose line) -->
<!-- Expected: Class B Arm1 passes, exit 0 -->
<!-- BC-5.39.010 TV: "B Arm1 — match | story 47a65c9; catalog 47a65c9; blockquote 47a65c9 | Continue" -->
<!--                                                                                               -->
<!-- PRODUCTION SHAPE: blockquote is ONE prose line with semicolon-separated entries.              -->
<!-- A line `> S-21.07=47a65c9` does NOT exist in production. The current                         -->
<!-- parse_story_index_blockquote_hash looks for starts_with("> S-21.07=") — this shape            -->
<!-- exposes F-S2107-P1B-003 (B3 leg permanently inert on production STORY-INDEX).                 -->
<!-- After fix: B3 is correctly extracted from the prose line → three-way match confirmed.         -->

| Story ID | Title | Epic | Points | Priority | depends_on | blocks | status | Notes |
|----------|-------|------|--------|----------|------------|--------|--------|-------|
| S-21.01 | validate-factory-path-staging WASM guard | E-21 | 11 | P1 | [] | [] | merged | [BC-4.16.001 v1.8] (wave 1; subsystems SS-04; tdd_mode strict; input-hash 32aaccc) |
| S-21.07 | validate-cross-site-correspondence WASM hook — five-arm PostToolUse cross-site value-correspondence gate (Classes A/B/D/E; BC-5.39.010 v1.2) | E-21 | 11 | P1 | [] | [] | draft | [BC-5.39.010 v1.2] (wave 4; subsystems SS-04, SS-05; tdd_mode strict; input-hash 47a65c9; AC-001..AC-021) |

> **E-21 delivery:** W1 (parallel): {S-21.01 (11 pts), S-21.02 (3 pts), S-21.03 (3 pts)} → W2 (parallel): {S-21.04 (5 pts), S-21.05 (5 pts)} → W3 (sequential): {S-21.06 (8 pts; depends_on [S-21.01])} → W4 (independent): {S-21.07 (11 pts; depends_on [])}. 7 stories total. 46 pts. DAG has no cycles. Input-hashes: S-21.01=32aaccc; S-21.02=8bd32e5; S-21.03=59e687e; S-21.04=a1b2c3d; S-21.05=c9265f0; S-21.06=b807086; S-21.07=47a65c9. All 7 distinct. [Fixture: hash values chosen for test isolation; S-21.04 uses a1b2c3d to avoid alias with S-21.07=47a65c9]
