---
kind: artifact
id: spike-ledger-entry
package: S6
sources: [state.md#2.5, BLUEPRINT.md#6]
---

# Spike ledger entry
The machine row for this kind - path, identifier, frontmatter keys, cap, writer and lifecycle - is its row in [[table:artifacts]].

Body: the answer and up to ten evidence lines. These are the fields of [[schema:spike-finding]].

Named spikes also keep the user's existing convention inside their own worktree: `.spike/STATE.md` and
`.spike/TASKS.md` on branch `spike/<name>` in `.worktrees/spike-<name>/`. Those two files belong to the
spike and are never read by the pipeline. The project ledger entry is the only thing that crosses over.
The spike protocol and its bounds are [[topic:spikes]]; named spikes are [[topic:named-spikes]].
