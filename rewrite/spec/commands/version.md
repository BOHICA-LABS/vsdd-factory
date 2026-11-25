---
kind: command
id: version
package: B2
sources: [BLUEPRINT.md#4.9, packaging.md#2.10]
---

# `vsdd --version`

Operator tool: `vsdd --version`, one of the commands that run outside an initialised project. Outside an
initialised project every command exits 2 except `init`, `import`, `doctor`, `obs`, `hooks selftest` and
`--version` ([[topic:command-conventions]]).

The smoke tier uses it as the Tool probe: `vsdd --version` runs from the main session and from a workflow step
([[packaging:tests-e2e]]). The same check, in each of the four calling contexts, is conformance probe N1
([[packaging:packaging-assumptions]]).

Built by package B2 ([[package:B2]]): `py/cli.py` with `--version` is part of the launcher and entries package.
