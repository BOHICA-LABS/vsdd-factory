---
document_type: behavioral-contract
bc_id: BC-5.39.010
version: "2"
status: draft
last_amended: "2026-07-30 (v2)"
modified:
  - "2026-06-01"
  - "2026-07-30 (v2)"
---

# BC-5.39.010: validate-cross-site-correspondence

Test fixture for T-045 (NOT the trigger file — this BC file is present but unused).

NOTE: T-045 triggers a VP file write (VP-9999.md), not this BC file. Arm A1 cannot
handle single-integer versions like "v2" in BC-INDEX (extract_version_token requires
vN.N format with a decimal point). Using a VP file instead isolates Class E1.

This BC file remains in the fixture for historical context only. It is NOT read by
the dispatcher in T-045 because the write event points to VP-9999.md, not BC-5.39.010.md.
VP-039.md was a dead residue (renamed to VP-9999.md in pass-2) — deleted by F-S2107-P3-020.
