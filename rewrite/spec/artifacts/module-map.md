---
kind: artifact
id: module-map
package: C2
sources: [state.md#2.5]
---

# Module map
The machine row for this kind - path, identifier, frontmatter keys, cap, writer and lifecycle - is its row in [[table:artifacts]].

The module map is in the class `spec-arch` and is locked with the whole specification set
([[topic:spec-lock]]). Criticality feeds the kill-rate thresholds of the mutation check
([[config:mutation]]).
