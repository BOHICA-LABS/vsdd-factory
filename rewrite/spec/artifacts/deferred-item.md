---
kind: artifact
id: deferred-item
package: S6
sources: [state.md#2.5]
---

# Deferred item
The machine row for this kind - path, identifier, frontmatter keys, cap, writer and lifecycle - is its row in [[table:artifacts]].

Waivers and advisory findings are not stored here: `vsdd report debt` ([[command:report-debt]]) derives them
from closure records, together with the debt a baseline recorded ([[record:baseline]]). The old
tech-debt register's open entries import as deferred items with `target: unscheduled`
([[topic:importer]]).

Register limits are warnings and never fail a gate ([[topic:artifact-rules]]).

Decision D31, built when the build widens: `vsdd checkpoint` writes one item for each live temporary
feature flag and retires it with the flag ([[topic:flag-lifecycle]]).
