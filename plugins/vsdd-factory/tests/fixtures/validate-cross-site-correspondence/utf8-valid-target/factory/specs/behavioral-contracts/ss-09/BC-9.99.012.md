---
document_type: behavioral-contract
level: L3
version: "1.0"
status: draft
bc_id: BC-9.99.012
section: "9.99"
last_amended: "2026-07-30 (v1.0) — test fixture: ADV-RECON-007 control (valid UTF-8)"
modified:
  - "2026-07-30"
---

# BC-9.99.012: fixture BC with a valid UTF-8 byte sequence (ADV-RECON-007 control)

This line intentionally contains a valid multi-byte UTF-8 character — an
em dash (—) — to prove the decode path succeeds on genuine UTF-8 content,
in contrast to the ADV-RECON-007 mutant fixture's invalid 0xFF/0xFE byte.

Fixture: BC file version "1.0", no BC-INDEX.md present (mirrors a1-no-bc-index
NotFound-is-advisory-only shape). path_allow includes behavioral-contracts/,
so host::read_file on BC-INDEX.md → NotFound → advisory, not a block.
Expected: exit 0 (no false block from the v1.20 UTF-8 fail-closed path).

Derives from BC-5.39.010 v1.20 precondition 15a / postcondition 25 (ADV-RECON-007):
a primary target that decodes successfully as UTF-8 is unaffected by the
fail-closed decode-failure path — it proceeds to normal Arm A1 dispatch.
