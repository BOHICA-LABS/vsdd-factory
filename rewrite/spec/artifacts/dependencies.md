---
kind: artifact
id: dependencies
package: C2
sources: [state.md#2.5, state.md#2.8, state.md#layout, BLUEPRINT.md#5.2]
---

# Dependencies
The machine row for this kind - path, identifier, frontmatter keys, cap, writer and lifecycle - is its row in [[table:artifacts]].

`dependencies.toml` is part of the locked specification set, so a dependency a person approves in a `merge`
decision cannot be written into it during a build. Dependency detection in `vsdd story deliver` therefore
reads `dependencies.toml` and the `dependencies` of every approved `merge` decision that is newer than the
latest approved `spec` decision. The next `spec` or amendment run has the architect fold them into
`dependencies.toml` ([[record:decision]]).

The file sits in the layout beside the other architecture files and is locked with the set whether or not it
carries a status ([[topic:spec-lock]], [[topic:state-layout]]).
