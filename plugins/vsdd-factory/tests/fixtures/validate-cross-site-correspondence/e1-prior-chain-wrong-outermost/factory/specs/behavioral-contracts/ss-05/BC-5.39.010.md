---
document_type: behavioral-contract
level: L3
version: "1.6"
status: active
bc_id: BC-5.39.010
section: "5.39"
last_amended: "2026-07-29 (v1.5) — Active. [Prior: 2026-01-01 (v1.4) — initial.]"
modified:
  - "2026-01-01"
  - "2026-07-29 (v1.5)"
---

# BC-5.39.010: test fixture for over-broad E1 exclusion (outermost version wrong + Prior chain)

Fixture: version "1.6" but outermost last_amended says "(v1.5)".
[Prior: ... (v1.4) ...] is present. A naive impl that skips E1 when [Prior:] is present
would incorrectly pass this. E1 must still fire.

Expected: exit 2, [Class E1] in output.

Derives from BC-5.39.010 precondition 37 + arm_e.rs `test_BC_5_39_010_class_e1_outermost_wrong_prior_chain_present_still_blocks`.
