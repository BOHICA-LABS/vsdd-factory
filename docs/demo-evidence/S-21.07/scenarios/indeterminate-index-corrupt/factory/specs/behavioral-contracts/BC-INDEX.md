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

<!-- Demo fixture (S-21.07 demo-evidence, not a bats fixture): secondary-index UTF-8 -->
<!-- decode-failure scenario. BC-5.39.010 §precondition 15b / postcondition 26      -->
<!-- (v1.22 / ADV-RECON11-001), `BcIndexVersionState::IndexUnreadable`.             -->
<!--                                                                                 -->
<!-- Everything below this comment block is well-formed UTF-8. A corrupted byte     -->
<!-- sequence (an invalid UTF-8 continuation byte, 0xFF 0xFE) is appended AFTER     -->
<!-- this file's otherwise-valid content by the demo harness (see                  -->
<!-- ../../../../../run-scenario.sh), simulating a mis-encoded save of BC-INDEX.md. -->
<!-- host::read_file succeeds (bytes ARE readable) but Rust's                      -->
<!-- std::str::from_utf8() decode fails inside extract_bc_index_version_state(),    -->
<!-- which is exactly the precondition 15b trigger: the row-location scan cannot   -->
<!-- run against undecodable bytes, so the row state for BC-5.39.010 is genuinely   -->
<!-- INDETERMINATE (not confirmed-absent).                                          -->
<!--                                                                                 -->
<!-- Expected disposition: distinct advisory naming BC-INDEX.md, emitted via        -->
<!-- host::log_warn, and Continue (exit 0) — NEVER the RowAbsent BLOCK path         -->
<!-- (postcondition 4), which would misreport index-file corruption as a dropped    -->
<!-- BC registration.                                                               -->

| BC ID | Title | Status | Capabilities | Stories | Version History |
|-------|-------|--------|-------------|---------|-----------------|
| [BC-1.13.001](ss-01/BC-1.13.001.md) | Dispatcher MUST load resolvers-registry.toml at startup | active | CAP-002 | S-12.03, S-12.04, S-12.06, S-12.08, S-18.14 | v1.3 \| v1.4 \| v1.5 \| v1.6 \| v1.7 \| v1.8 \| v1.9 \| v1.10 \| v1.11 \| v1.12 |
| [BC-5.39.010](ss-05/BC-5.39.010.md) | validate-cross-site-correspondence WASM hook — five-arm PostToolUse cross-site value-correspondence gate | draft | CAP-032 | S-21.07 | v1.6 |
