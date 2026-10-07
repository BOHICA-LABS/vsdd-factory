---
document_type: behavioral-contract
level: L3
version: "1.6"
status: draft
bc_id: BC-5.39.010
section: "5.39"
last_amended: "2026-07-29 (v1.6) — test fixture: RowMalformed advisory trigger"
modified:
  - "2026-07-29"
  - "2026-07-29 (v1.6)"
---

# BC-5.39.010: test fixture for a1-row-malformed

Fixture: BC version "1.6". The BC-INDEX row for this BC has only 2 non-empty fields
(RowMalformed(2)) — it is a notes/changelog table row, not a body-table row.

Expected: Arm A1 emits advisory + Continue (exit 0). The advisory must contain:
  "Registration status cannot be determined from this line"
  "Verify BC-INDEX body-table registration manually"

BC-5.39.010 v1.10 PC5 postcondition 4a: RowMalformed → advisory-only, NEVER blocks.
