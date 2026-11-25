---
kind: artifact
id: holdout-scenario
package: C2
sources: [state.md#2.5]
---

# Holdout scenario
The machine row for this kind - path, identifier, frontmatter keys, cap, writer and lifecycle - is its row in [[table:artifacts]].

Scenarios are hidden from builders: they live in the `holdout` class, and the wall that keeps them out of
builder eyes is the walls table's ([[table:walls]], [[topic:information-walls]]). Evaluations of scenarios
are written by code ([[record:holdout-evaluation]]); a walled finding keeps its full form inside the holdout
wall ([[topic:walled-findings]]).
