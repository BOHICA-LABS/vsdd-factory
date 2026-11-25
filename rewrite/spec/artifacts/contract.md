---
kind: artifact
id: contract
package: C2
sources: [state.md#2.5, state.md#2.4]
---

# Behavioral contract

The machine row of the kind — path, identifier, frontmatter keys, writer, cap and lifecycle — is the
`[contract]` row of [[table:artifacts]]; identifier schemes are [[table:identifiers]].

References are checked for syntax at the write and for existence at the gates and at approval, so a contract
may name a subsystem that a later stage writes ([[topic:artifact-rules]]). A retired contract is replaced
with `status: retired` and `replaced_by`, never deleted ([[topic:artifact-rules]]).
