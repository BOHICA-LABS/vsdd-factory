---
document_type: behavioral-contract
level: L3
version: "1.5"
status: active
bc_id: BC-5.39.010
section: "5.39"
last_amended: "2026-01-01 (v1.5) — test fixture: AC-001 index-newer-than-primary mutant"
modified:
  - "2026-01-01"
  - "2026-01-01 (v1.5)"
---

# BC-5.39.010: test fixture for AC-001 index-newer-than-primary mutant

Fixture: BC frontmatter version "1.5"; BC-INDEX.md row cites v1.6 (index newer than primary).
Expected: Class A Arm1 fires (PC2b — index-newer anomalous), exit 2, [Class A Arm1] in block
reason, message contains "index is newer than primary" and "POLICY 14 leg 5".

Derives from BC-5.39.010 v1.11 PC2b: index_version > fm_version → anomalous → BLOCK.
(PC2a, the opposite direction, is covered by AC-022 / T-P6A: primary-newer → advisory.)
