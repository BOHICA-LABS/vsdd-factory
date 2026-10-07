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

<!-- Fixture: A1 control — BC-5.39.010 row cites v1.6 matching BC frontmatter version "1.6" -->
<!-- Expected: Class A Arm1 passes, exit 0 -->
<!-- BC-5.39.010 TV: "A Arm1 — match | BC v1.6; INDEX v1.6 | Continue" -->
<!--                                                                    -->
<!-- PRODUCTION SHAPE: frontmatter changelog: array with pipe characters in YAML string values -->
<!--                                                                    -->
<!-- F-S2107-P1B-007 RED GATE: extract_bc_index_version finds the frontmatter changelog    -->
<!-- line "... BC-5.39.010 v1.5→v1.6; version cell updated v1.5|v1.6." BEFORE body table. -->
<!-- Splits on '|' → first cell contains "v4.43" (the changelog version prefix).           -->
<!-- Returns "4.43" (wrong source). "4.43" ≠ "1.6" → spurious violation → EXIT 2.         -->
<!-- Test expects EXIT 0 → FAILS → RED gate (exposes F-S2107-P1B-007).                     -->
<!-- After fix: frontmatter lines skipped; body table row v1.6 found → "1.6" == "1.6" → EXIT 0 ✓ -->

| BC ID | Title | Status | Capabilities | Stories | Version History |
|-------|-------|--------|-------------|---------|-----------------|
| [BC-1.13.001](ss-01/BC-1.13.001.md) | Dispatcher MUST load resolvers-registry.toml at startup | active | CAP-002 | S-12.03, S-12.04, S-12.06, S-12.08, S-18.14 | v1.3 \| v1.4 \| v1.5 \| v1.6 \| v1.7 \| v1.8 \| v1.9 \| v1.10 \| v1.11 \| v1.12 |
| [BC-5.39.010](ss-05/BC-5.39.010.md) | validate-cross-site-correspondence WASM hook — five-arm PostToolUse cross-site value-correspondence gate | draft | CAP-032 | S-21.07 | v1.6 |
