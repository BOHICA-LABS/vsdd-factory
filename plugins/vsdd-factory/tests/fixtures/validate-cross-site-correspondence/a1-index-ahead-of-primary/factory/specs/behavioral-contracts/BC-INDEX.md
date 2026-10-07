---
document_type: bc-index
version: "4.47"
last_amended: "2026-08-05 (v4.47)"
total_bcs: 1983
---

# BC-INDEX

<!-- Fixture: T-P6B PC2b — index newer than primary (anomalous)                             -->
<!-- BC frontmatter version: "1.10"; BC-INDEX row cites v1.11.                               -->
<!-- The INDEX has a HIGHER version than the BC file itself.                                 -->
<!--                                                                                         -->
<!-- BC-5.39.010 v1.11 PC2b: index newer than primary → BLOCK (anomalous).                  -->
<!-- No burst-ordering argument explains the INDEX carrying a higher version than the        -->
<!-- BC file it cites. This indicates out-of-burst INDEX update or wrong BC path.           -->
<!--                                                                                         -->
<!-- Block message (normative):                                                              -->
<!--   "validate-cross-site-correspondence [Class A Arm1]: BC-INDEX.md body-table row for   -->
<!--    BC-5.39.010 cites v1.11 but frontmatter version: is \"1.10\" — index is newer than  -->
<!--    primary. This is anomalous: the index cannot legitimately advance ahead of the BC    -->
<!--    it cites. Verify no index row was updated out-of-burst or under the wrong BC path.  -->
<!--    Update per POLICY 14 leg 5."                                                         -->
<!--                                                                                         -->
<!-- RED GATE: v1.10 implementation blocks on any mismatch with a DIFFERENT message.        -->
<!-- Test asserts the EXACT v1.11 PC2b block message text → FAILS under current impl.       -->
<!-- After v1.11 implementation: directional block with prescribed text → PASSES.           -->

| BC ID | Title | Status | Capabilities | Stories | Version History |
|-------|-------|--------|-------------|---------|-----------------|
| [BC-5.39.010](ss-05/BC-5.39.010.md) | validate-cross-site-correspondence WASM hook | draft | E-12 | S-21.07 | v1.9 \| v1.10 \| v1.11 |
