---
document_type: behavioral-contract
level: L3
version: "1.6"
status: active
bc_id: BC-5.39.010
section: "5.39"
last_amended: "2026-07-29 (v1.6) — test fixture: BC-INDEX.md absent (NotFound)"
modified:
  - "2026-01-01"
  - "2026-07-29 (v1.6)"
---

# BC-5.39.010: test fixture for AC-003 NotFound (no BC-INDEX.md file)

Fixture: BC file version "1.6" exists, but BC-INDEX.md is ABSENT (not on disk).
path_allow includes behavioral-contracts/, so host::read_file → NotFound.
Expected: advisory only, exit 0.

Derives from BC-5.39.010 precondition 8: "NotFound on BC-INDEX.md → Advisory + Continue".
