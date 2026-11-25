---
kind: packaging
id: python-package
package: B3
sources: [BLUEPRINT.md#4.1, hooks.md#2.4, packaging.md#after-cutover]
---

# The Python package

All shipped Python is one standard-library package, `py/vsdd/`, shared by the hook entry and the `vsdd` command
(decision D5 in [[packaging:packaging-decisions]]). The blueprint's module list is authoritative:

- Shared core: `project.py`, `config.py`, `frontmatter.py`, `events.py`, `schema.py` (a small JSON Schema checker).
- Sub-packages: `hooks/`, `rules/`, `policy/`, `state/`, `gates/`, `delivery/`, `cli/`.

Who owns what:

- `py/vsdd/hooks/` and `py/vsdd/rules/` — the hooks design: the runner (`main.py`, `model.py`, `registry.py`,
  `emit.py`, `policy.py`, `paths.py`, `shell.py`, `edits.py`, `gitfs.py`, `records.py`) and one module per rule
  ([[topic:hook-runner]], [[table:rules]]).
- `py/vsdd/policy/` — the three policy tables `walls.toml`, `classes.toml`, `artifacts.toml`
  ([[table:walls]], [[table:path-classes]], [[table:artifacts]]).
- `py/vsdd/state/`, `py/vsdd/gates/`, `py/vsdd/delivery/` — the state design: stores, survey, checkpoint, routes,
  status, briefing; gate commands and jobs; story delivery and release.
- `py/vsdd/cli/` — one module per operator subcommand, owned by this design
  ([[command:doctor]], [[command:events]], [[command:obs]], [[command:repos-scan]], [[command:export-pdf]],
  [[command:render-diagram]], [[command:version]]).

Entry files beside the package: `py/hook.py` for hooks (hooks design) and `py/cli.py`, the entry for the `vsdd`
command ([[packaging:cli-entry]]), both started through the one launcher.

Shipped Python imports only standard-library modules and the `vsdd` package, and none of the network modules
(lint L-PY, [[table:lint-checks]]). The `hooks.json` file is generated from `py/vsdd/hooks/registry.py`
([[topic:single-sources]]).
