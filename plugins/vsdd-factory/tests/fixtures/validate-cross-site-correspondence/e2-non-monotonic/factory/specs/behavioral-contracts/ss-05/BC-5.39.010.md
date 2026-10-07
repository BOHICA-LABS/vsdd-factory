---
document_type: behavioral-contract
level: L3
version: "1.3"
status: active
bc_id: BC-5.39.010
section: "5.39"
last_amended: "2026-07-29 (v1.3) — test fixture: non-monotonic modified[]"
modified:
  - "2026-05-14"
  - "2026-05-18 (v1.1)"
  - "2026-05-15"
---

# BC-5.39.010: test fixture for AC-016 non-monotonic modified[]

Fixture: modified[] entries have dates 2026-05-14, 2026-05-18, 2026-05-15.
After stripping annotation suffix from "2026-05-18 (v1.1)" → "2026-05-18".
Dates: 2026-05-14 → 2026-05-18 → 2026-05-15 — NOT strictly ascending.
Expected: Class E2 fires, exit 2.

Derives from BC-5.39.010 postcondition 22: "non-ascending modified[] → Block [Class E2]".
