---
kind: artifact
id: toolchain
package: C2
sources: [state.md#2.5, state.md#2.12, BLUEPRINT.md#4.6]
---

# Toolchain
The machine row for this kind - path, identifier, frontmatter keys, cap, writer and lifecycle - is its row in [[table:artifacts]].

From the blueprint's story kinds: an empty repository has the architect record the toolchain (`[commands]`,
the JUnit report, `[paths]`) in this file, which is part of what the `spec` decision approves. The first
survey after that approval writes those values into the keys of `factory.toml` that are still unset, and the
briefing announces it. `vsdd plan schedule` puts the scaffold chore story alone in the first wave, and its
green gate is the first run of those commands. A project never needs a person to edit `factory.toml`
between `spec` and `wave`.

Missing commands are a state, not an error: the workflows that measure have "[commands] is configured" as a
precondition, and the values arrive by approval of this file ([[topic:project-configuration]]).

Decision D31, built when the build widens: the file gains a `[flags]` table, which holds the flag settings
the project will run with and the proposal of a flag system that a `spec` decision approves
([[topic:flag-system-choice]], [[table:flag-config-keys]]). Its pattern lists need room, so the row's cap
rises to 12,000 bytes with it.
