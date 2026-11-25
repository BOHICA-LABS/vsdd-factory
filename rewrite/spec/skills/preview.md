---
kind: skill
id: preview
package: K3a
sources: [agents-skills.md#2.12, BLUEPRINT.md#4.5]
---

# preview

Starts or stops the local preview server that shows mockups and diagrams in a browser while designing.

Invoked by: person.

Frontmatter beyond `name` and `description`:

```
disable-model-invocation: true
argument-hint: "<start or stop>"
```

## What it does

Instructs the main session to run the server shipped in `<plugin>/skills/preview/app/`. The app keeps its
`package.json` there, not at the plugin root, and installs its dependencies into the plugin's data directory on
first use.
