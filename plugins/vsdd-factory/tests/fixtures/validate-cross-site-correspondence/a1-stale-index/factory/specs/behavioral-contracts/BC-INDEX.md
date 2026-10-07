---
document_type: bc-index
version: "4.43"
last_amended: "2026-07-31 (v4.43)"
changelog:
  - date: 2026-07-31
    change: "v4.43 (2026-07-31): BC-5.39.010 v1.5→v1.6; version cell updated v1.5|v1.6."
  - date: 2026-07-17
    change: "v4.09 (2026-07-17): BC-1.13.001 v1.11→v1.12; version cell v1.11|v1.12."
total_bcs: 1983
---

# BC-INDEX

<!-- Fixture: AC-001 mutant — BC-5.39.010 row cites v1.6 but BC frontmatter says version "1.5" -->
<!-- INDEX-NEWER direction (PC2b): index (v1.6) > primary (v1.5) → anomalous → BLOCK -->
<!-- Expected: Class A Arm1 fires (PC2b), exit 2, "index is newer than primary", "POLICY 14 leg 5" -->
<!-- BC-5.39.010 v1.11 TV: "A Arm1 — index-newer | BC v1.5; INDEX v1.6 | Block [Class A Arm1]" -->
<!--                                                                                -->
<!-- PRODUCTION SHAPE: frontmatter has changelog: array with entries citing BC IDs   -->
<!-- and containing pipe characters in YAML string values.                           -->
<!--                                                                                -->
<!-- F-S2107-P1B-007 exposure: extract_bc_index_version scans ALL lines for          -->
<!-- line.contains('|') && line.contains("BC-5.39.010"). The frontmatter changelog  -->
<!-- entry "... BC-5.39.010 v1.5→v1.6; version cell updated v1.5|v1.6." has BOTH   -->
<!-- conditions: pipe char in "v1.5|v1.6" AND contains "BC-5.39.010". The extractor -->
<!-- hits this BEFORE the body table, extracts wrong version from frontmatter.       -->
<!-- Current code: returns v4.43 (first token in first cell). "4.43" ≠ "1.6" →     -->
<!-- violation → EXIT 2 (correct exit, wrong source). After fix: frontmatter lines  -->
<!-- are skipped; body table row v1.5 is found → "1.5" ≠ "1.6" → EXIT 2 ✓          -->

| BC ID | Title | Status | Capabilities | Stories | Version History |
|-------|-------|--------|-------------|---------|-----------------|
| [BC-1.13.001](ss-01/BC-1.13.001.md) | Dispatcher MUST load resolvers-registry.toml at startup | active | CAP-002 | S-12.03, S-12.04, S-12.06, S-12.08, S-18.14 | v1.3 \| v1.4 \| v1.5 \| v1.6 \| v1.7 \| v1.8 \| v1.9 \| v1.10 \| v1.11 \| v1.12 |
| [BC-5.39.010](ss-05/BC-5.39.010.md) | validate-cross-site-correspondence WASM hook — five-arm PostToolUse cross-site value-correspondence gate | draft | CAP-032 | S-21.07 | v1.6 |
