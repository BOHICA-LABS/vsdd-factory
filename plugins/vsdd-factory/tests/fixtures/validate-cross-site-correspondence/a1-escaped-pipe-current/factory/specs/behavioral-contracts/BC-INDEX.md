---
document_type: bc-index
version: "4.43"
last_amended: "2026-07-31 (v4.43)"
changelog:
  - date: 2026-07-31
    change: "v4.43 (2026-07-31): BC-1.13.001 v1.11→v1.12; version cell updated v1.11|v1.12."
total_bcs: 1983
---

# BC-INDEX

<!-- Fixture: T-039 — escaped-pipe version chain CONTROL (F-S2107-P1B-006)           -->
<!-- BC-1.13.001 at version "1.12". INDEX row has version history:                   -->
<!--   v1.3 \| v1.4 \| v1.5 \| v1.6 \| ... \| v1.12                                 -->
<!-- Expected: Class A Arm1 passes, exit 0 (current version 1.12 found in chain)    -->
<!--                                                                                  -->
<!-- F-S2107-P1B-006 RED GATE: extract_bc_index_version splits on '|' (which also   -->
<!-- splits at escaped pipes '\|') and returns FIRST version token = "1.3".          -->
<!-- "1.3" ≠ "1.12" → spurious violation → EXIT 2.                                  -->
<!-- Test expects EXIT 0 → FAILS → RED gate (post-implementation, pre-B006-fix).     -->
<!-- After fix: LAST token from version chain is "1.12" → "1.12" == "1.12" → EXIT 0 ✓ -->
<!--                                                                                  -->
<!-- NOTE: frontmatter changelog also mentions BC-1.13.001 with "v1.11|v1.12" pipe. -->
<!-- This ALSO exposes F-S2107-P1B-007 (frontmatter false-match). Either bug alone  -->
<!-- causes the CONTROL to fail. Both must be fixed for this test to pass.           -->

| BC ID | Title | Status | Capabilities | Stories | Version History |
|-------|-------|--------|-------------|---------|-----------------|
| [BC-1.13.001](ss-01/BC-1.13.001.md) | Dispatcher MUST load resolvers-registry.toml at startup | active | CAP-002 | S-12.03, S-12.04, S-12.06, S-12.08, S-18.14 | v1.3 \| v1.4 \| v1.5 \| v1.6 \| v1.7 \| v1.8 \| v1.9 \| v1.10 \| v1.11 \| v1.12 |
