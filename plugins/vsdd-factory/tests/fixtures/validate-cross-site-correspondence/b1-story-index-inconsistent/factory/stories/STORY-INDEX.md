---
document_type: story-index
version: "4.999"
last_amended: "2026-08-05 (v4.999)"
---

# STORY-INDEX

<!-- Fixture: T-P6D PC13b — STORY-INDEX internally inconsistent (B2 ≠ B3)                  -->
<!-- B1 = story frontmatter input-hash = "abc1234" (7-char hex)                             -->
<!-- B2 = catalog row input-hash = "def4567"  (7-char hex)                                  -->
<!-- B3 = blockquote S-21.07=deadb00 (7-char hex, B3 ≠ B2)                                 -->
<!-- B2 ≠ B3 (STORY-INDEX is internally inconsistent): catalog and blockquote DISAGREE.     -->
<!-- No POLICY 3 burst-ordering argument explains this: both catalog row and blockquote are  -->
<!-- written by state-manager in the same commit. B2≠B3 indicates partial-write, wrong-story -->
<!-- editing, or a state-manager bug.                                                        -->
<!--                                                                                         -->
<!-- BC-5.39.010 v1.11 PC13b: B2≠B3, regardless of B1 → BLOCK with three-provenance msg.   -->
<!-- Block message (normative):                                                              -->
<!--   "validate-cross-site-correspondence [Class B]: Story S-21.07 input-hash three-way    -->
<!--    mismatch: frontmatter=abc1234 STORY-INDEX-catalog=def4567                           -->
<!--    STORY-INDEX-blockquote=deadb00. STORY-INDEX catalog and blockquote disagree —        -->
<!--    this is anomalous and has no burst-ordering explanation..."                          -->
<!--                                                                                         -->
<!-- RED GATE (v1.10 implementation): blocks on B1≠B2 OR B1≠B3 but with a DIFFERENT        -->
<!-- message that does NOT say "catalog and blockquote disagree" and lacks the normative     -->
<!-- "has no burst-ordering explanation" phrasing. Test asserts v1.11 PC13b substring →     -->
<!-- FAILS under current implementation.                                                     -->
<!-- After v1.11 implementation: PC13b block with normative text → PASSES.                  -->

| Story ID | Title | Epic | Points | Priority | depends_on | blocks | status | Notes |
|----------|-------|------|--------|----------|------------|--------|--------|-------|
| S-21.07 | validate-cross-site-correspondence WASM hook | E-21 | 11 | P1 | [] | [] | draft | [BC-5.39.010 v1.11] (wave 4; input-hash def4567; AC-001..AC-021) |

> **E-21 delivery:** W4 (independent): {S-21.07 (11 pts; depends_on [])}. Input-hashes: S-21.07=deadb00.
