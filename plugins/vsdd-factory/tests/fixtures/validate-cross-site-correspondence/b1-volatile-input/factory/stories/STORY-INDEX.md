---
document_type: story-index
version: "4.999"
last_amended: "2026-08-04 (v4.999)"
---

# STORY-INDEX

<!-- Fixture: PC40 volatile-input test (T-047)                                              -->
<!-- B1 = story frontmatter input-hash = "abc123" (story file — 6-char hex, valid per      -->
<!--   arm_b.rs parse_story_input_hash which has no length bound)                           -->
<!-- B2 = catalog row input-hash = "def4567" (7-char hex, valid for {7,40} extractor)      -->
<!-- B3 = blockquote S-21.07=deadb00 (7-char hex, valid; B2 ≠ B3 → PC13b without PC40)     -->
<!--                                                                                         -->
<!-- F-P6-003 fix: original fixture used "def456" (6 chars) which fails {7,40} extraction  -->
<!-- → B2=None, B3=None → (None, None) arm fires → advisory regardless of volatile check.  -->
<!-- The original test passed for the WRONG REASON (not because PC40 fired, but because     -->
<!-- the extractors returned None). "def4567"/"deadb00" (7-char) ensures B2 and B3 are      -->
<!-- extracted. Without volatile: B2≠B3 → PC13b → block (exit 2). With volatile: PC40 →    -->
<!-- advisory + exit 0. Discrimination is now genuine.                                       -->
<!--                                                                                         -->
<!-- T-047 CONTROL (added per F-P6-003): a companion test uses the b1-story-index-          -->
<!-- inconsistent fixture (B2=def4567, B3=deadb00, no volatile inputs) to prove that        -->
<!-- WITHOUT volatile, the same B2≠B3 mismatch blocks (exit 2). This control test           -->
<!-- disambiguates: PC40 is what prevents the block, not some other code path.              -->
<!--                                                                                         -->
<!-- BC-5.39.010 v1.6 PC40: volatile inputs suppress the three-way comparison.             -->

| S-21.07 | validate-cross-site-correspondence | E-21 | S-21.06 | [] | input-hash def4567 |

> **E-21 S-21 delivery inputs:** S-21.07=deadb00.
