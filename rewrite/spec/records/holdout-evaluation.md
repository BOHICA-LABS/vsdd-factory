---
kind: record
id: holdout-evaluation
package: S3
sources: [BLUEPRINT.md#4.7, state.md#2.5]
---

# Holdout evaluation

Tracked. Written inside the holdout wall: the evaluation file is in the `holdout-evaluations` path class,
which only code writes. The machine row - path, content fields, writer, cap and lifetime - is the
`[holdout-evaluation]` row of [[table:artifacts]]; walled findings keep their full form here
([[topic:walled-findings]]). Closing a cycle removes `demo/<cycle>/` from the tree the same way it removes
`holdout/evaluations/<cycle>/` ([[table:artifacts]], the `[demo-evidence]` and `[holdout-evaluation]` rows).
