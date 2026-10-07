---
document_type: behavioral-contract
bc_id: BC-5.39.010
version: "1.6"
status: draft
last_amended: "2026-08-05 (v1.6) — test fixture: AC-022 / T-P6A PC2a primary-newer-than-index"
modified:
  - "2026-08-04"
  - "2026-08-05 (v1.6)"
---

# BC-5.39.010: test fixture for AC-022 / T-P6A PC2a (primary-newer-than-index advisory)

Fixture for BC-5.39.010 v1.11 PC2a directional carve-out (AC-022 / T-P6A).

BC frontmatter version: "1.6".
BC-INDEX row cites v1.5 (one version behind).

At the PostToolUse instant after this BC file is written, the INDEX row (secondary site)
has not yet been updated by state-manager — this is the POLICY 3 guaranteed ordering
artefact. The primary site (this file) is NEWER than the index.

BC-5.39.010 v1.11 PC2a: primary newer than index → advisory + Continue (NOT block).
Advisory message MUST match (verbatim, with <id>, <index_version>, <fm_version> substituted):
"validate-cross-site-correspondence [Class A Arm1] advisory: BC-INDEX.md body-table row
for <id> cites v<index_version> but frontmatter version: is \"<fm_version>\" — primary
newer than index; state-manager index update pending; Class A BLOCK suspended."
