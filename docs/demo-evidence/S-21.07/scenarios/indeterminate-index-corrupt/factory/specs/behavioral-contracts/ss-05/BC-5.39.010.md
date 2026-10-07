---
document_type: behavioral-contract
level: L3
version: "1.6"
status: active
bc_id: BC-5.39.010
section: "5.39"
last_amended: "2026-01-01 (v1.6) — demo fixture: S-21.07 secondary-index UTF-8 decode failure"
modified:
  - "2026-01-01"
  - "2026-01-01 (v1.6)"
---

# BC-5.39.010: demo fixture for IndexUnreadable (secondary-index UTF-8 decode failure)

Primary target (this file) decodes fine and cites version "1.6" — the primary read path
(precondition 15a / postcondition 25) is NOT under test here. The secondary read target,
`../BC-INDEX.md`, is corrupted with an invalid UTF-8 byte sequence by the demo harness
before the dispatcher is invoked. That corruption is what should trigger the
`IndexUnreadable` disposition (precondition 15b / postcondition 26): a distinct advisory
+ Continue, not a BLOCK.
