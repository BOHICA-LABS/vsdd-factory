---
kind: artifact
id: asset
package: C2
sources: [state.md#2.5]
---

# Asset
The machine row for this kind - path, identifier, frontmatter keys, cap, writer and lifecycle - is its row in [[table:artifacts]].

Assets live in whichever specification directory includes them, so they take that directory's path class and
lock ([[topic:spec-lock]], [[table:path-classes]]).
