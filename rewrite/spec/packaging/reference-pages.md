---
kind: packaging
id: reference-pages
package: D2
sources: [packaging.md#2.5, packaging.md#2.11]
---

# Generated reference pages

`tools/gen_reference.py` writes `docs/reference/*.md` from the shipped files themselves. `--check` fails on drift.
This replaces six hand-maintained reference pages whose counts were all stale.

It reads agent and skill frontmatter, workflow `meta`, the rule registry, the `vsdd` argument parser and the
configuration schema. The generated pages:

- `docs/reference/agents.md`
- `docs/reference/skills.md`
- `docs/reference/workflows.md`
- `docs/reference/rules.md`
- `docs/reference/cli.md`
- `docs/reference/configuration-keys.md`

Lint L-DOC checks that generated reference pages are current ([[table:lint-checks]]). Contributors never edit
`docs/reference/` by hand; the replacement `CLAUDE.md` says to run the generator instead
([[packaging:repo-claude-md]]).

Built by package D2 ([[package:D2]]).
