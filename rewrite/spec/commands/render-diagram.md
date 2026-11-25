---
kind: command
id: render-diagram
package: T5
sources: [BLUEPRINT.md#4.9, packaging.md#2.10, BLUEPRINT.md#2.1]
---

# `vsdd render diagram`

Operator tool. `vsdd render diagram <file>` renders a Mermaid file to SVG or PNG through a pinned package run with
`npx`; exports Excalidraw and draw.io files when their command-line tools are installed, and otherwise says the
source file is the deliverable.

## Who may call it

Run by the main session.

The design behind it is decision D14 ([[doc:decisions]]): keep the diagram renderer as a `vsdd` command, with
Node as an optional prerequisite that `vsdd doctor` checks, and package T5 starts by proving the engines
([[package:T5]]).

Built by package T5.
