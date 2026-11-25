---
kind: artifact
id: adr
package: C2
sources: [state.md#2.5]
---

# Architecture decision record
The machine row for this kind - path, identifier, frontmatter keys, cap, writer and lifecycle - is its row in [[table:artifacts]].

A decision record is written once and not changed: it is one of the kinds that stay frozen even under an
amendment, because a change to it is a new artifact (`artifact_locked`; [[topic:spec-lock]]).

Decision records are distinct from the person's decision records and packets that a cycle keeps
([[record:decision]]).

Decision D31, built when the build widens: the choice of a product's flag system is recorded in a decision
record with fixed headings, which `vsdd gate spec` checks ([[topic:flag-system-choice]]).
