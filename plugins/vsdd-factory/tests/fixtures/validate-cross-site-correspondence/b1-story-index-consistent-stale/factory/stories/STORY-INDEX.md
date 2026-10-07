---
document_type: story-index
version: "4.999"
last_amended: "2026-08-05 (v4.999)"
---

# STORY-INDEX

<!-- Fixture: AC-023 / T-P6C PC13a — STORY-INDEX internally consistent, story just rewritten -->
<!-- B1 = story frontmatter input-hash = "47a65c9"                                          -->
<!-- B2 = catalog row input-hash = "4be9d21"  (7-char hex, B2 ≠ B1)                        -->
<!-- B3 = blockquote S-21.07=4be9d21 (7-char hex, B3 == B2)                                 -->
<!-- B2 == B3 (STORY-INDEX internally consistent): catalog and blockquote AGREE.             -->
<!-- B1 ≠ B2: story frontmatter hash differs from STORY-INDEX (story was just rewritten).   -->
<!--                                                                                         -->
<!-- BC-5.39.010 v1.11 PC13a: B2==B3 AND B1≠B2 → advisory + Continue (NOT block).          -->
<!-- POLICY 3 ordering: state-manager STORY-INDEX update MUST happen AFTER story write.     -->
<!-- Advisory MUST contain: "[Class B] advisory:", "STORY-INDEX sites agree with each       -->
<!-- other", "State-manager STORY-INDEX update pending; Class B BLOCK suspended".           -->
<!--                                                                                         -->
<!-- RED GATE (v1.10 implementation): any B1≠B2 → block (exit 2).                          -->
<!-- Test expects exit 0 → FAILS under current implementation.                              -->

| Story ID | Title | Epic | Points | Priority | depends_on | blocks | status | Notes |
|----------|-------|------|--------|----------|------------|--------|--------|-------|
| S-21.07 | validate-cross-site-correspondence WASM hook | E-21 | 11 | P1 | [] | [] | draft | [BC-5.39.010 v1.11] (wave 4; input-hash 4be9d21; AC-001..AC-023) |

> **E-21 delivery:** W4 (independent): {S-21.07 (11 pts; depends_on [])}. Input-hashes: S-21.07=4be9d21.
