---
document_type: behavioral-contract
level: L3
version: "1.5"
status: active
bc_id: BC-5.39.010
section: "5.39"
last_amended: "2026-07-29 (v1.33) — test fixture: AC-018 combined A1-index-newer + E1 mismatch"
modified:
  - "2026-01-01"
  - "2026-07-29 (v1.33)"
---

# BC-5.39.010: test fixture for AC-018 combined violations (A1 + E1)

Fixture: BC version "1.5"; BC-INDEX has "v1.6" (index-newer, PC2b → A1 block);
last_amended "(v1.33)" (version 1.5 ≠ last_amended v1.33 → E1 block).

Both Class A Arm1 AND Class E1 violations fire independently.
Expected: exit 2, single combined block containing both [Class A Arm1] and [Class E1].

Derives from BC-5.39.010 postcondition 23: "multiple violations combined into ONE block_with_fix".
v1.11 rationale: the A1 component uses index-newer-than-primary (PC2b → block). The prior
v1.10 fixture (primary-newer direction) would produce a PC2a advisory from A1 under v1.11,
not a block — the combined-violations path requires BOTH arms to independently produce blocks.
PC2b (index-newer) + E1 mismatch ensures both arms block simultaneously.
