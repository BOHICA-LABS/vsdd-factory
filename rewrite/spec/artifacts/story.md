---
kind: artifact
id: story
package: C2
sources: [state.md#2.5, BLUEPRINT.md#4.6]
---

# Story
The machine row for this kind - path, identifier, frontmatter keys, cap, writer and lifecycle - is its row in [[table:artifacts]]; the kinds, with the tests and implement stages of each and whether it has a red
gate, are the story-kind rows of the same shipped table ([[table:story-kinds]]). The frontmatter carries
the eighteen keys of the state design, with its single optional `method` key split into the two
stage-method keys: `impl_method` (another method of the kind's implement agent) and `tests_method`
(another method of the test-writer for the tests stage, `ui-tests` or `e2e-tests`), so a UI story can
name `tests_method: ui-tests` and `impl_method: ui-implementation`. The survey and the admission
commands resolve kind and override into the story unit; a write-grant suffix `@<method>` in the walls
matches the resolved method of the caller's own stage ([[rule:write-scopes]]).

These keys give the tool every story fact the pipeline design asked for in its S10. The wave is the one fact
not in the story file: `vsdd plan schedule` computes it and stores it in the plan's closure record
([[topic:waves]]).

## Acceptance criteria

An acceptance criterion names one contract in its heading, `### AC-2 (BC-2.03.007)`
([[topic:artifact-rules]]). Identifiers `AC-N` and `EC-N` are local to the story
([[table:identifiers]]).

A story that does not come from the plan is admitted by the tool ([[topic:story-admission]]). Story history
is not collected in the file: review findings and convergence live in records, which is why story files stay
small ([[topic:artifact-rules]]).

## Feature flags (decision D31; built when the build widens)

Not on the thin slice path, and the slice's story row is unchanged. Package F1 adds the optional keys
whose names begin `flag`; they are defined in [[artifact:flag]].
