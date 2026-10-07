---
document_type: behavioral-contract
level: L3
version: "1.14"
status: active
bc_id: BC-5.39.010
section: "5.39"
last_amended: "2026-08-07 (v1.14) — test fixture: production-scale fuel-budget gate (F-S2107-P8-013)"
modified:
  - "2026-08-07"
  - "2026-08-07 (v1.14)"
---

# BC-5.39.010: test fixture for production-scale fuel-budget gate (F-S2107-P8-013)

Fixture: BC frontmatter version "1.14" matches BC-INDEX.md row (v1.14 is the last chain entry).
Expected: Arm A1 finds the row, verifies version matches, exits 0 — CLEAN happy path.

This fixture uses the live BC-INDEX.md (576 KB, ~1985 rows) as the production-scale index
to prove the WASM plugin completes without exhausting its fuel budget.
The live index is copied at fixture-creation time; row count is comparable to the production
corpus (F-S2107-P8-013 closure: sandbox × production-scale combination, previously untested).
