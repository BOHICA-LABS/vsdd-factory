---
kind: packaging
id: repo-layout
package: B0
sources: [packaging.md#1, packaging.md#after-cutover, BLUEPRINT.md#4.1]
---

# The repository after cutover

In 1.0 the plugin is plain text: Markdown agents and skills, JavaScript workflows, one `hooks.json`,
standard-library Python, and two small `sh` launchers. Nothing is compiled, so the Rust workspace, the
five-platform binaries, the bot commit and the bats suite all go. The repository becomes one plugin directory
(`plugins/vsdd-factory/`, the only thing installed on a user's machine) plus the tests, tools and docs around it.

```
vsdd-factory/
├── plugins/vsdd-factory/          THE PLUGIN. The only directory a user's machine receives.
│   ├── .claude-plugin/plugin.json
│   ├── .claude-plugin/icon.png
│   ├── .mcp.json                  research and browser servers (list owned by agents-skills design)
│   ├── README.md   LICENSE        short README; LICENSE byte-identical to the root one (lint)
│   ├── agents/*.md                (agents-skills design) 20 files
│   ├── skills/<name>/SKILL.md     (agents-skills design) 13 skills
│   ├── methods/*.md               (agents-skills design) one procedure per file
│   ├── workflows/*.js             (pipeline design) 12 scripts, each with the stamped prelude
│   ├── schemas/*.schema.json      (pipeline design) the single source of every result schema
│   │                              (no pipeline/ directory: stage structure is in the scripts, and the route
│   │                              table and the preconditions are code in py/vsdd/state/)
│   ├── hooks/hooks.json           (hooks design) generated from the rule table, checked in, no variants
│   ├── py/
│   │   ├── launch.sh              the one guarded launcher: finds uv, fails loudly, runs an entry
│   │   ├── find-root.sh           project discovery, sourced by the launcher
│   │   ├── hook.py                entry for hooks (hooks design)
│   │   ├── cli.py                 entry for the vsdd command
│   │   └── vsdd/                  one standard-library package ([[packaging:python-package]])
│   ├── bin/vsdd                   3-line sh shim; puts `vsdd` on the Bash tool's PATH
│   ├── templates/                 (state design) one skeleton per artifact kind
│   ├── assets/                    design-system/, demo/, scaffold/: files that methods copy into a project
│   └── observability/             docker-compose.yml, collector/Loki/Prometheus config, Grafana dashboards
├── workflow-lib/                  not shipped: prelude.js (the canonical prelude), agent-shared.md (the block
│                                  every agent file carries), severity.md (the reviewers' severity table)
├── tools/                         build.py  check.py  gen_reference.py  release.py  cutover.py   (not shipped)
├── tests/
│   ├── unit/                      pytest: shared core and vsdd subcommands
│   ├── hooks/                     pytest: rules, in-process and through the real launcher; payloads/
│   ├── lint/                      pytest: checks over every shipped file; roster.toml
│   ├── workflows/                 node --test: harness.mjs, universal tests, scenarios
│   ├── conformance/               shared fixtures for the convergence rule, run in Node and in Python
│   └── e2e/                       pytest -m e2e: probes, sample-project/, fixtures/, fakebin/gh, conformance/
├── docs/                          user and contributor documentation ([[packaging:documentation]])
├── .github/workflows/             ci.yml  e2e.yml  release.yml  semgrep.yml
├── pyproject.toml  uv.lock        development only: pytest, ruff, pyyaml. `[tool.uv] package = false`
├── .nvmrc                         `24`. Node is used only to run tests/workflows. No package.json.
└── README.md  CLAUDE.md  CONTRIBUTING.md  RELEASING.md  CHANGELOG.md  LICENSE  .gitignore  .semgrep.yml
```

Rules for the plugin directory, each enforced by a lint test ([[table:lint-checks]]):

- No `package.json` at the plugin root. Claude Code runs a package install at plugin install time when it finds one
  with a lockfile, and 1.0 wants no install-time behaviour. A skill that ships a Node application (the old
  `visual-companion` did) keeps its `package.json` inside its own skill folder and installs into a cache directory
  on first use.
- No `CLAUDE.md` at the plugin root. The platform does not load it and the validator warns.
- No file the plugin writes at run time. The plugin root changes with every version. Python bytecode goes to
  `${XDG_CACHE_HOME:-~/.cache}/vsdd-factory/pycache` (the launcher sets `PYTHONPYCACHEPREFIX`).
- Total size under 3 MB and no single file over 256 KB except the icon. The old plugin directory was 102 MB.

The Python layout (`py/launch.sh`, `py/hook.py`, `py/vsdd/`) is the packaging design's proposal for a part the
hooks design also owns; if the hooks design names these differently, the names follow that design
([[topic:hook-launcher]]).
