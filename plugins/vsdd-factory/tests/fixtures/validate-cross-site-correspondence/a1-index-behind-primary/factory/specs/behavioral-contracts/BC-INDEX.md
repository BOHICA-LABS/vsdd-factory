---
document_type: bc-index
version: "4.47"
last_amended: "2026-08-05 (v4.47)"
total_bcs: 1983
---

# BC-INDEX

<!-- Fixture: AC-022 / T-P6A PC2a — primary newer than index                                  -->
<!-- BC frontmatter version: "1.6"; BC-INDEX row cites v1.5 (one version behind).            -->
<!-- B.C. PRIMARY (BC-5.39.010.md) has been written first (v1.6);                           -->
<!-- INDEX (this file) has not yet been updated by state-manager (POLICY 3 ordering).       -->
<!--                                                                                         -->
<!-- BC-5.39.010 v1.11 PC2a: primary newer than index → advisory + Continue (exit 0).       -->
<!-- The POLICY 3 "state_manager_runs_last" guarantee means "index behind primary" is the   -->
<!-- EXPECTED state at the primary-write instant — it is a burst-ordering artefact.         -->
<!-- This direction MUST NOT block. Blocking produces a spurious exit 2 on every correct    -->
<!-- BC authoring burst, making the gate unreliable.                                        -->
<!--                                                                                         -->
<!-- RED GATE (v1.10 implementation): v1.10 blocks on ANY version mismatch → exits 2.       -->
<!-- Test expects exit 0 (advisory) → FAILS under current implementation.                   -->
<!-- After v1.11 implementation: directional carve-out → advisory + exit 0 → PASSES.        -->

| BC ID | Title | Status | Capabilities | Stories | Version History |
|-------|-------|--------|-------------|---------|-----------------|
| [BC-5.39.010](ss-05/BC-5.39.010.md) | validate-cross-site-correspondence WASM hook | draft | E-12 | S-21.07 | v1.5 |
