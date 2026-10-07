---
document_type: behavioral-contract
level: L3
version: "1.33"
status: active
bc_id: BC-5.39.010
section: "5.39"
last_amended: "2026-07-29 (v1.31) — test fixture: E1 version-mismatch mutant"
modified:
  - "2026-01-01"
  - "2026-07-29 (v1.31)"
---

# BC-5.39.010: test fixture for E1 version-mismatch mutant

Fixture: frontmatter version "1.33" but last_amended outermost prefix says "(v1.31)".
BC-INDEX.md has row v1.33 (A1 passes). Only E1 fires.
Expected: Class E1 fires, exit 2, [Class E1] in block reason.

Derives from BC §Canonical Test Vectors: "E1 — match | version 1.6; last_amended (v1.6) | Continue"
(inverted: version "1.33" vs last_amended "(v1.31)" → block)
Also BC EC-015: version "1.33", last_amended "(v1.31)" → Block: Class E1.
