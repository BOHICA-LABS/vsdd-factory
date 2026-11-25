---
kind: artifact
id: architecture-overview
package: C2
sources: [state.md#2.5]
---

# Architecture overview
The machine row for this kind - path, identifier, frontmatter keys, cap, writer and lifecycle - is its row in [[table:artifacts]].

The overview is in the class `spec-arch` and is locked with the whole specification set, whether or not it
carries a status ([[topic:spec-lock]]). The subsystem list is not an index: it is the subsystem files
themselves ([[artifact:subsystem]], [[topic:stored-and-derived]]).
