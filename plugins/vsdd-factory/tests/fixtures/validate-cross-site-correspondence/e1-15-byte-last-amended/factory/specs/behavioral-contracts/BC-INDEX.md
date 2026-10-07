---
document_type: bc-index
version: "4.43"
last_amended: "2026-07-31 (v4.43)"
total_bcs: 1983
---

# BC-INDEX

<!-- Supporting fixture for T-045. T-045 triggers on a VP file write (VP-9999.md),    -->
<!-- NOT a BC file write. Arm A1 fires only on BC file writes — it does NOT run here.  -->
<!-- The BC-INDEX row with "v2" has no decimal point: extract_bc_index_version_state   -->
<!-- returns RowPresentNoVersion (the vN.N regex (\bv[0-9]+\.[0-9]+\b) requires a      -->
<!-- decimal), NOT a version match. This is immaterial because A1 never fires for VP   -->
<!-- writes. BC-INDEX is present only to satisfy the hook's read_file capability path. -->
<!-- F-P6-014: prior comment incorrectly said "matches BC frontmatter '2'" — that is   -->
<!-- wrong on two counts: (1) A1 does not fire on VP writes; (2) RowPresentNoVersion,  -->
<!-- not a version match, because "v2" fails the vN.N decimal-point requirement.       -->

| BC ID | Title | Status | Capabilities | Stories | Version History |
|-------|-------|--------|-------------|---------|-----------------|
| [BC-5.39.010](ss-05/BC-5.39.010.md) | validate-cross-site-correspondence WASM hook | draft | CAP-032 | S-21.07 | v2 |
