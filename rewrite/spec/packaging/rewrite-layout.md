---
kind: packaging
id: rewrite-layout
package: B0
sources: [packaging.md#during-the-rewrite-branch-rewrite-v1, BLUEPRINT.md#9]
---

# The repository during the rewrite

During the rewrite the whole new tree is built under a staging root, `next/`, that mirrors the final repository
root (decision D2 in [[packaging:packaging-decisions]]):

```
vsdd-factory/
├── CLAUDE.md                  REPLACED by a 15-line rewrite-mode file (see 2.11). The only old file touched.
├── rewrite/                   process files: STATE.md, reference/, design/, BLUEPRINT.md, workflows/, spike/
├── next/                      the entire 1.0 repository root, in its final relative shape
│   ├── plugins/vsdd-factory/
│   ├── workflow-lib/  tools/  tests/  docs/  .github/
│   ├── pyproject.toml  uv.lock  .nvmrc  .gitignore  .semgrep.yml
│   └── README.md  CLAUDE.md  CONTRIBUTING.md  RELEASING.md  CHANGELOG.md  LICENSE
└── (everything else)          the old system: read for reference, never edited
```

`next/.gitignore` becomes the root file at promotion, so it is written for the root: bytecode and caches,
`node_modules/`, `.claude/worktrees/`, `/.worktrees/`, `.claude/settings.local.json`, and `/.mcp.json` followed by
the negation `!/plugins/vsdd-factory/.mcp.json`. The root `.mcp.json` holds live keys and stays untracked; the
plugin's own `.mcp.json` is shipped and must be committable. The negation is needed during the rewrite as well,
because the old root `.gitignore` ignores `.mcp.json` at every depth and would drop the staged plugin's file from a
commit.

Builders work only under `next/`. To try the new plugin anywhere, including inside one of the eight projects, start
Claude Code with `--plugin-dir <repo>/next/plugins/vsdd-factory`. The platform documents that a `--plugin-dir`
plugin silently replaces an installed plugin of the same manifest name for that session only, so this needs no
uninstall.

The checks run from `next/`: `cd next && uv run python tools/check.py` ([[packaging:check-command]]). Continuous
integration is not needed during the rewrite because nothing has been pushed. If the branch is pushed, one extra
file, `.github/workflows/next.yml` at the real root, runs the same command with `working-directory: next`. It is
deleted at promotion.

The build plan itself, and why packages are sized and ordered as they are, is [[packaging:build-plan]].

Alternative rejected: building the plugin at `plugins/vsdd-factory-next/` and the rest in new root directories. Ten
top-level names collide, so the cutover would be a hand-edited merge instead of a rename.
