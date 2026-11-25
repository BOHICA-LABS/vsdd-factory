---
kind: artifact
id: demo-evidence
package: C2
sources: [state.md#2.5, BLUEPRINT.md#2.2]
---

# Demo evidence
The machine row for this kind - path, identifier, frontmatter keys, cap, writer and lifecycle - is its row in [[table:artifacts]].

The script and the text transcript are the evidence, because they can be run and read again. Rendered video
and GIF files go to `.factory/local/media/` and are never committed ([[topic:state-layout]]). Decision D22:
a demo script plus its text transcript is enough as committed demo evidence, with screenshots capped and no
committed video; the old repository carries 110 MB of committed recordings.
