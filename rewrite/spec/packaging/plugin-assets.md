---
kind: packaging
id: plugin-assets
package: T5
sources: [state.md#2.13, BLUEPRINT.md#4.1]
---

# Plugin assets

`<plugin>/assets/` holds the files that methods copy into a project, in three directories (part of the plugin layout
in `BLUEPRINT.md#4.1`):

- `assets/design-system/`
- `assets/demo/`
- `assets/scaffold/`

They are what remains, in 1.0, of the old system's "something copied into the product" templates: the design-system
starter (23 files), the demo tape, the demo browser test, the demo CI job and the justfile. No skill copies them.
Method files cite them by path, and the lint rule L-TEMPLATE checks that every cited path exists and that nothing
is copied from a skill's folder ([[table:lint-checks]]).

They are distinct from the artifact skeletons in `<plugin>/templates/`, one per agent-written artifact kind, which
stay inside the plugin and are stamped by `vsdd new <kind>` ([[topic:templates]]).

Built by package T5 ([[package:T5]]).
