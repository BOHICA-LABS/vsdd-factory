---
kind: artifact
id: domain-spec
package: C2
sources: [state.md#2.5]
---

# Domain specification
The machine row for this kind - path, identifier, frontmatter keys, cap, writer and lifecycle - is its row in [[table:artifacts]]. Item identifiers are the domain schemes of [[table:identifiers]] (`CAP-`, `ENT-`, `DI-`,
`EVT-`, `DEC-`, `ASM-`, `R-`, `FM-`, sequential).

Domain files are in the class `spec-product` and are locked with the whole specification set, whether or
not they carry a status ([[topic:spec-lock]]).
