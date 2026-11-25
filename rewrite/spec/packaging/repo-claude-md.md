---
kind: packaging
id: repo-claude-md
package: D3
sources: [packaging.md#the-replacement-claude-md]
---

# The replacement `CLAUDE.md`

The old file is 47 KB and is loaded into every agent that starts in this repository. Almost all of it is process: the
principle essay, a 35-row routing table, the commit choreography, the decision-number chains, hook diagnostics. None
of that applies to a repository that is a plugin with tests. The replacement is about forty lines. Skeleton:

```markdown
# vsdd-factory

A Claude Code plugin. The plugin is `plugins/vsdd-factory/`. Everything else in this repository builds,
tests, documents or releases it.

## Check your work
    uv run python tools/check.py        # static checks, unit tests, workflow tests; about a minute
Run the end-to-end tests only when asked. They start real model sessions and cost money.

## Where things are
| Path | What |
| plugins/vsdd-factory/ | the plugin: agents/, skills/, workflows/, hooks/, py/, bin/, templates/, observability/ |
| workflow-lib/prelude.js | shared workflow code, stamped into every workflow |
| tests/ | unit/, hooks/, lint/ (pytest); workflows/ (node --test); e2e/ |
| tools/ | build.py, check.py, gen_reference.py, release.py |
| docs/ | documentation; docs/reference/ is generated |

## Rules the checks enforce
- Shipped Python uses the standard library only and never opens a network connection.
- Do not edit between `// >>> stamped:prelude` and `// <<< stamped:prelude`. Edit workflow-lib/prelude.js,
  then run `uv run python tools/build.py`.
- Do not edit docs/reference/. Run `uv run python tools/gen_reference.py`.
- Adding or removing an agent, skill or workflow also means editing tests/lint/roster.toml.
- A hook timeout is in seconds. A gate blocks with exit code 2 and its reason on stderr.
- A workflow script cannot import, read files, run commands, or call Date.now() or Math.random().

## Git
Commit or push only when asked. No AI attribution in commit messages. Never use --no-verify.
Never force-push main. Never move a release tag.

## Releasing
See RELEASING.md.

## What this repository is not
It is not itself a factory project. There is no pipeline state here to update after a change.
```

The rewrite-mode version, used at the root from the first work package until promotion, is fifteen lines: read
`rewrite/STATE.md` first; work only under `next/`; everything else is the system being replaced, to be read and not
edited or obeyed; the check command ([[packaging:check-command]]); the git rules
([[packaging:rewrite-layout]]).

Decision D3 in [[packaging:packaging-decisions]]: the root `CLAUDE.md` is replaced on the rewrite branch first,
before any building. Delivered by package D3 ([[package:D3]]).
