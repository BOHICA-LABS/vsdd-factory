---
kind: topic
id: hook-launcher
sources: [hooks.md#2.2, BLUEPRINT.md#4.1]
---

# The launch chain

```
Claude Code ── hooks.json ──> sh py/launch.sh hook <mode> ──> uv run --script py/hook.py <mode> ──> vsdd.hooks.main
                                 │                                 │
                                 │ not a factory project           │ exit 0  = allow (stdout may carry one JSON object)
                                 └─ exit 0, nothing else           │ exit 10 = a rule blocked (reason already on stderr)
                                                                   └ anything else = the launch failed
```

`packaging.md` proposes one launcher for the hook entry and the `vsdd` command (its interface I3); the
paths are accepted from it. Neither entry may `exec` uv: each must see uv's exit code to print the right
message. The `cli` entry shares the uv discovery and the environment hygiene; what it does with the exit
code is in [[packaging:cli-entry]].

The `hook` entry takes one more argument, the mode, which says what a launch failure must turn into.

| Mode | Used by | If the runner cannot run |
|---|---|---|
| `gate` | `PreToolUse`, `UserPromptExpansion` | Exit 2 with a one-line reason and the fix. The action is blocked. |
| `brief` | `SessionStart` | Exit 0 and print a fixed JSON briefing that says the gates are down and why, to the person and to the model. |
| `notify` | `SubagentStart`, `SubagentStop`, `PreCompact`, `SessionEnd` | Exit 0. Nothing is being guarded on these events, and the session start has already announced the fault. |

## Why each part is there

| Choice | Reason |
|---|---|
| The launcher does not `exec` uv. | A Python crash or syntax error leaves through `uv run` as exit 1 (measured), which Claude Code treats as non-blocking. The launcher must see the exit code to turn it into a block. |
| The runner blocks with exit 10, not 2. | uv itself exits 2 when it finds no interpreter or cannot open its cache (measured). With a private code the launcher can tell "a rule blocked" from "uv failed" and print the right message. Claude Code still receives 2 in both cases. |
| `--python '>=3.11'` on the command line as well as in the script header. | `UV_PYTHON=3.9` in the environment overrides the inline `requires-python` and ran the script on Python 3.9 (measured). The explicit flag wins over the variable (measured). The launcher also unsets the variable, and `hook.py` exits 13 if the interpreter is older than 3.11. |
| `--no-config --offline --no-python-downloads`. | Never touch the network; see below. |
| `PYTHONSAFEPATH=1`, `PYTHONPATH` unset. | The project being worked on must not be able to shadow a standard-library module. `hook.py` adds its own directory to `sys.path` explicitly. |
| `PYTHONPYCACHEPREFIX` in the user's cache directory. | Bytecode is cached without writing into the installed plugin (a rule in `packaging.md`). Measured: 57 ms warm with the cache, 70 ms without. |
| uv is looked for on `PATH` and then in four usual places. | Taken from `packaging.md`: a session started from a desktop launcher can have a short `PATH`. |
| `here="${0%/*}"` instead of `CLAUDE_PLUGIN_ROOT`. | The launcher works even if that variable is missing, and a parameter expansion costs no process. |
| `VSDD_FACTORY_HOOKS=off`. | The one emergency switch. Only a person can set it, because a tool call cannot change the environment of the Claude Code process. The session briefing announces it. |

Expanded row for `--no-config --offline --no-python-downloads`: the hook must behave the same in every
project and must never touch the network. A stdlib-only script has nothing to download, and an
interpreter download inside a hook call would be slow and would fail in a sandbox. If no Python 3.11
or newer is installed, uv exits 2 and the launcher prints the fix (`uv python install 3.12`). The
three flags cost nothing (measured: 35 to 37 ms with or without them).

Illustrative skeleton of the `hook` entry (the real file is about 70 lines):

```sh
#!/bin/sh
# <plugin>/py/launch.sh hook <gate|brief|notify>     stdin: hook JSON, passed through untouched
mode="${2:-gate}"; here="${0%/*}"
. "$here/find-root.sh"                      # sets FACTORY_ROOT, or FACTORY_LEGACY=1, or neither
if [ -z "$FACTORY_ROOT" ]; then             # not a 1.0 factory project: do nothing
  [ "$mode" = brief ] && [ -n "$FACTORY_LEGACY" ] && print_migration_line
  exit 0
fi

fail() {                                    # $1 = code, $2 = fixed message (no quotes inside)
  append_event runner.launch_failed "$1"    # one printf into the day's event file; never fails the caller
  case "$mode" in
    gate)  echo "vsdd-factory: $2 Guarded tool calls are blocked until this is fixed. [$1]" >&2; exit 2 ;;   # $2 carries the fix as a command a person can run in their own terminal
    brief) print_down_briefing "$2"; exit 0 ;;
    *)     exit 0 ;;
  esac
}

if [ "${VSDD_FACTORY_HOOKS:-on}" = off ]; then
  [ "$mode" = brief ] && print_disabled_briefing
  append_event runner.disabled hooks_off; exit 0       # the launcher writes this line: Python never starts
fi

uv=$(command -v uv 2>/dev/null) || for d in "$HOME/.local/bin" "$HOME/.cargo/bin" /opt/homebrew/bin /usr/local/bin; do
  [ -x "$d/uv" ] && { uv="$d/uv"; break; }
done
[ -n "$uv" ] || fail uv_missing "uv is not installed, so the factory gates cannot run. In your own terminal run: brew install uv (or: curl -LsSf https://astral.sh/uv/install.sh | sh), then restart Claude Code."

unset UV_PYTHON PYTHONPATH PYTHONHOME PYTHONSTARTUP
export PYTHONSAFEPATH=1 PYTHONUTF8=1 VSDD_FACTORY_ROOT="$FACTORY_ROOT"
export PYTHONPYCACHEPREFIX="${XDG_CACHE_HOME:-$HOME/.cache}/vsdd-factory/pycache"

"$uv" run --quiet --no-config --offline --no-python-downloads --python '>=3.11' --script "$here/hook.py" "$mode"
rc=$?
case "$rc" in
  0)  exit 0 ;;
  10) exit 2 ;;                             # a rule blocked; the runner wrote the reason to stderr
  2)  if interpreter_ok; then fail uv_unusable "uv could not run (exit 2), although a Python 3.11 or newer is installed. ..."
      else fail py_floor "uv found no Python 3.11 or newer. In your own terminal run: uv python install 3.12, ..."; fi ;;
  13) fail py_floor "..." ;;                # hook.py's own floor check
  *)  fail "launch_rc_$rc" "the hook runner could not run (exit $rc)." ;;
esac
```

`interpreter_ok` is the probe that separates the two causes of uv's exit 2 ([[topic:hook-failure]]). A mode
other than `gate`, `brief` or `notify` exits 2 with a message, because an unknown mode would make every
launch failure silent.

The two event lines the launcher writes itself, `runner.launch_failed` and `runner.disabled`, are written
in shell before any Python runs. Their fields, and the directory they go to, are in [[topic:event-log]].

`find-root.sh` is sourced by the launcher for both entries, so there is one implementation of project
discovery ([[topic:hooks-inert]]).

## Messages to a person

A person's own terminal does not have the plugin's `bin/` on its `PATH`, and when `uv` is missing
`vsdd doctor` cannot run at all. Every fix line therefore names something that works where the person
is: a terminal command that needs nothing from the plugin (install `uv`; `uv python install 3.12`), a
slash command, or `vsdd ...` once `setup` has installed the shim at `~/.local/bin/vsdd`. A message
gives the in-session form first and the terminal form second. Both entries use one download policy:
neither the hook path nor the `cli` path ever downloads a Python, and nothing in the plugin installs
one. The fallback briefing also sets `systemMessage`, so the person sees that the gates are down even
if the model says nothing.

Time budgets per mode, and the failure table, are in [[topic:hook-failure]]. The self-guarding
command in `hooks.json` is in [[topic:hook-wiring]].
