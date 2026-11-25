---
kind: config
id: paths
package: S1
sources: [state.md#2.12, BLUEPRINT.md#4.12, BLUEPRINT.md#4.7]
---
# [paths]

Owned by the state design. Globs relative to a worktree root. Seven classes cover the product tree; a path
that matches more than one belongs to the first in the precedence of [[table:path-classes]].
`vsdd init` detects the globs, `vsdd doctor` lists tracked files that fall in
no class, and a write to such a file is blocked with the line to add under `[paths]`. Keys, types and
defaults: [[table:config-keys]].
