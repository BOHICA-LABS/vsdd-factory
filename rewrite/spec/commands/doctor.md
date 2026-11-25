---
kind: command
id: doctor
package: T1
sources: [BLUEPRINT.md#4.9, packaging.md#2.10, BLUEPRINT.md#5.5]
---

# `vsdd doctor`

Operator tool ([[topic:command-conventions]]), run by the main session.

`vsdd doctor` checks `uv`, a Python at the floor that `uv` can find without the network, git, `gh` and its login,
the Claude Code version against the floor, the project's state (none, pre-1.0, 1.0), and that the event log is
writable. Warns about Docker and Node, which only the observability stack and the `npx` servers need. It is the
first thing `docs/troubleshooting.md` tells a person to run, in place of the diagnostics section of the old
`CLAUDE.md`.

`vsdd doctor --install-shim` writes the terminal shim ([[packaging:terminal-shim]]). `doctor` also checks each
sandbox need ([[packaging:sandbox-needs]]) — it is run through the Bash tool by `setup` — the attribution setting,
that `commands.run` takes its port from `{port}` or `$VSDD_PORT`, tracked product files that fall in no path class,
Node for the export tools, and whether the hooks are live: run through the Bash tool it looks for the caller marker
of its own call, as the survey does ([[topic:caller-identity]]); run in a terminal it says the hooks cannot be
checked from there. Only the main session may run `doctor` through the Bash tool.

A missing Python 3.11 or newer is not something the plugin fixes. `vsdd doctor` starts through the launcher with
`--no-python-downloads`, so it cannot start on the very machine that lacks one; the launcher's message names the
terminal command, `uv python install 3.12`, and that is the one fix. There is no `vsdd doctor --fix`.

Built by package T1 ([[package:T1]]), which depends on B3 and H9.

## Team use (decision D33; built when the build widens)

With team use on, `doctor` prints the lines of the preflight ([[table:team-checks]]): the git floor, the
plugin floor, a shallow clone, the identity git would use, the upstream of each factory branch, and
whether the remote answers a listing. It also prints the address each factory branch is pushed to, with
any user or password part removed, for people to compare between clones; the length of the state
branch; and what the factory needs of the repository's own rules ([[topic:team-sync]], "A remote with
its own rules"). Whether the clone may push stays unmeasured, and `doctor` says so.
