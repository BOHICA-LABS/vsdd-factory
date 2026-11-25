---
kind: config
id: commands
package: S1
sources: [state.md#2.12, BLUEPRINT.md#4.12, BLUEPRINT.md#4.6]
---
# [commands] and [commands.env]

Owned by the state design. Each command is one shell string, run from the root of the worktree under test.
Missing commands are a state, not an error: see [[topic:project-configuration]]. Keys, types, defaults
and meanings: [[table:config-keys]].
