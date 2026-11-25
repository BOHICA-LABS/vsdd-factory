---
kind: packaging
id: cli-entry
package: B2
sources: [packaging.md#2.10, hooks.md#2.2, packaging.md#4]
---

# How `vsdd` reaches the PATH

All supporting tools are one command, `vsdd`, implemented in `<plugin>/py/vsdd/cli/`, one module per subcommand, on
the same shared core as the hook runner. `<plugin>/bin/vsdd` puts it on the Bash tool's `PATH` in every session
where the plugin is enabled:

```sh
#!/bin/sh
root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
exec sh "$root/py/launch.sh" cli "$@"
```

A bare command name matters because a workflow script's prompts cannot contain the plugin's path: scripts get no
path substitution. (The survey also returns the plugin root, which prompts use for method file paths.)

The launcher. `sh "<plugin>/py/launch.sh" <entry> [args]`, where the entry is `hook` or `cli`. It finds `uv` (on
`PATH`, then in `~/.local/bin`, `~/.cargo/bin`, `/opt/homebrew/bin`, `/usr/local/bin`), exits 2 with a one-line
message that carries a terminal command if there is none, sets `PYTHONPYCACHEPREFIX`, and runs
`uv run --quiet --no-config --offline --no-python-downloads --script "<plugin>/py/<entry>.py"`. Neither entry
downloads a Python: nothing in the plugin installs one, and the launcher's message names the terminal command that
does ([[packaging:sandbox-needs]]).

The `cli` entry shares the uv discovery and the environment hygiene of the hook entry. Like the hook entry it does
not `exec` uv, because a launcher that has replaced itself cannot print the fix when uv fails. It maps uv's exit
code as follows:

- 0 and 1 are `vsdd`'s own answers ("yes" and "no", [[topic:command-conventions]]) and pass through with nothing
  added.
- 2 is ambiguous: `vsdd` exits 2 for a usage or environment error, and uv exits 2 when it finds no Python at the
  floor. The launcher runs the interpreter probe of [[topic:hook-failure]]; only when that finds no Python 3.11 or
  newer does it add the `uv python install 3.12` line. It exits 2 either way.
- Any other code means the tool did not run: the launcher prints that, with the terminal commands to check, and
  passes the code on.

The `hook` entry takes a mode argument and turns a launch failure into the mode's behaviour — that behaviour belongs
to the hooks design ([[topic:hook-launcher]]).

Conventions shared by every subcommand — `--json`, the exit codes 0, 1 and 2, the global option `-C <dir>`, and the
detached long commands — are the command conventions ([[topic:command-conventions]], [[topic:long-commands]]).
Which subcommands exist is fixed by [[topic:command-conventions]]: the pipeline-facing commands owned by the state design, plus the
operator tools this design owns ([[command:doctor]], [[command:events]], [[command:obs]], [[command:repos-scan]],
[[command:export-pdf]], [[command:render-diagram]], [[command:version]]). Calling the Python package directly
instead of `bin/vsdd` is blocked.
