---
kind: command
id: export-pdf
package: T5
sources: [BLUEPRINT.md#4.9, packaging.md#2.10, BLUEPRINT.md#2.1]
---

# `vsdd export pdf`

Operator tool. `vsdd export pdf <file.md> [--css <path>]` converts a Markdown report to PDF: Markdown to HTML,
then headless Chromium through the pinned Playwright package. Needs Node; `vsdd doctor` reports whether it is
available. Package T5 proves the engine first.

## Who may call it

Run by the main session.

The design behind it is decision D14 ([[doc:decisions]]): two skills needed tools the old plugin never shipped, a
Markdown-to-PDF converter and a diagram renderer; both need Node (`npx`), which the bundled servers already need;
keep both, with Node as an optional prerequisite that `vsdd doctor` checks, and package T5 starts by proving the
engines ([[package:T5]]).

Skill `export-pdf` ([[skill:export-pdf]]) is the model-facing side of this command. Built by package T5.
