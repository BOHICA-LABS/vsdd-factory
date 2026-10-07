---
document_type: behavioral-contract
bc_id: BC-1.13.001
version: "1.12"
status: active
last_amended: "2026-07-31 (v1.12)"
modified:
  - "2025-01-01"
  - "2026-07-31 (v1.12)"
---

# BC-1.13.001: Dispatcher MUST load resolvers-registry.toml at startup

Test fixture for T-039: escaped-pipe version chain CONTROL.
BC-INDEX has version history `v1.3 \| v1.4 \| ... \| v1.12`.
extract_bc_index_version("BC-1.13.001", ...) should return the LAST token "1.12".
Current code returns the FIRST token "1.3" (F-S2107-P1B-006).
"1.3" ≠ "1.12" → spurious violation → exit 2.
Expected (post-fix): exit 0 (current version 1.12 correctly found).
