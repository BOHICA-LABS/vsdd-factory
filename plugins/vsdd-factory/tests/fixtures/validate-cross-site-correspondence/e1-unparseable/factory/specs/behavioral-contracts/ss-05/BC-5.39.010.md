---
document_type: behavioral-contract
level: L3
version: "1.6"
status: active
bc_id: BC-5.39.010
section: "5.39"
last_amended: "Active — companion amendment; version tracking deferred"
modified:
  - "2026-07-29"
---

# BC-5.39.010: test fixture for AC-015 unparseable last_amended

Fixture: last_amended has no YYYY-MM-DD (vX.X) prefix pattern.
extract_last_amended_outer_version returns None.
Expected: advisory-only (not block), exit 0.

Derives from BC-5.39.010 precondition 37 last sentence:
"if last_amended does not match the regex → emit advisory, return Continue".
