---
kind: skill
id: export-pdf
package: K3b
sources: [agents-skills.md#2.12, BLUEPRINT.md#4.5, packaging.md#2.10]
---

# export-pdf

Converts a Markdown report to PDF with a neutral stylesheet, or one the project supplies.

Invoked by: person.

Frontmatter beyond `name` and `description`:

```
disable-model-invocation: true
argument-hint: "<file.md> [--css path]"
```

## What it does

Instructs the main session to run `vsdd export pdf` ([[command:export-pdf]]), which the packaging design
provides: Markdown to HTML, then headless Chromium through the pinned Playwright package. It needs Node;
`vsdd doctor` reports whether it is available. It was one of the two tools missing from the old plugin (with
the diagram renderer), and this skill is its face.
