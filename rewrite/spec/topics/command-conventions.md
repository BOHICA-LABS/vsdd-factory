---
kind: topic
id: command-conventions
package: S1
sources: [BLUEPRINT.md#4.9, state.md#2.16, packaging.md#2.10]
---

# What every `vsdd` command shares

Ownership. "One tool, two owners. The state design owns everything the pipeline calls (`py/vsdd/state/`,
`gates/`, `delivery/`); the pipeline design supplies each gate's pass condition. The packaging design owns
the operator tools (`py/vsdd/cli/`)." All are subcommands of `vsdd`, one module per subcommand, on the
shared core. `<plugin>/bin/vsdd` puts the command on the Bash tool's `PATH` in every session where the
plugin is enabled.

Output. Every command prints one JSON object with `ok` under `--json`.

Exit codes. 0 for success or "yes", 1 for a negative answer (a check failed, a lock is held, a verdict is
stale), 2 for a usage or environment error, including a missing `uv`. There is no exit code 3.

Outside an initialised project. "Outside an initialised project every command exits 2 except `init`,
`import`, `doctor`, `obs`, `hooks selftest` and `--version`."

Global option. "Every command also takes the global option `-C <dir>`, which names the project directory,
so a clerk's command stays one segment with no `cd` in front."

Callers. The hook runner enforces the last caller column from the `[callers]` table in `walls.toml` (which
groups of subcommands each row may run, with `status --fix` and `doctor` granted apart from the read group)
and treats a read command as a read of the classes `[reveals]` gives for it
([[table:walls]], [[table:command-families]]). Calling the Python package directly instead of `bin/vsdd`
is blocked. "The commands only a person may run, the two governed merge commands and the story kinds exempt
from tests before code are constants in code."

Who is calling. A command knows nothing about its caller on its own; it reads the caller marker the hook
runner wrote for that one call. See [[topic:caller-identity]].

Long commands. A gate that builds or tests, `story deliver` and `release` run detached and re-attach; see
[[topic:long-commands]].

Inputs. "Tool commands take their inputs as arguments or as a file under the temporary directory, never on
standard input, so a clerk's command never needs a redirect. Free text travels percent-encoded in an
argument, and the spike commands take files the runner writes."
