---
kind: rule
id: gate-guard
package: H6
sources: [hooks.md#2.10, hooks.md#2.11.10, hooks.md#2.11.1, hooks.md#2.7, BLUEPRINT.md#5.4]
---

# gate-guard

Protects the gates themselves. New rule; it closes the "agent disables its own gate" hole.

| Registration | |
|---|---|
| Kind | gate |
| Runs on | `PreToolUse`, tools `Edit`, `Write`, `MultiEdit`, `Bash` |
| Result | `Block` |
| Fails | closed — "the write would land; the agent can simply retry once the fault is fixed" |
| Relax | locked (decision D12) |

## What it blocks

- a write under the installed plugin directory (only when that directory lies outside the project, so
  developing the plugin in its own repository still works);
- a write to the runner's own directory;
- a write to a Claude Code settings file whose result sets `disableAllHooks`, disables or removes this
  plugin, or sets any `VSDD_*` environment variable (which is how a tool call could otherwise switch
  the hooks off for the next session);
- a Bash call of `claude plugin disable`, `claude plugin uninstall` or `claude config`;
- a call of a tool subcommand on the person-only list (the `HUMAN_ONLY` constant, carried by
  [[table:callers]] until [[package:H6]] moves it into `vsdd/hooks/model.py`);
- a call of a tool subcommand (or a flag such as `status --fix` or `--include-walled`) in a group the
  caller's row does not grant, from `[callers]` in `walls.toml`;
- a Bash command that assigns a `VSDD_*` variable itself (for every caller, because the runner adds the
  caller marker's key that way, see [[topic:caller-identity]]);
- a Bash call that reaches the tool's Python other than through `bin/vsdd` (it names `py/vsdd`,
  `cli.py`, `hook.py` or `launch.sh`, or imports the package from Python), for every caller, so the
  checks on subcommands cannot be stepped around.

There is no canary command.

The project configuration and the evidence, decision and step-record stores are covered by
[[rule:write-scopes]], because their classes cannot be written by a tool call.

## How subcommand groups work

`shell.py` classifies each Bash segment into at most one command family ([[table:command-families]]);
a call of the plugin's own tool is in no git family, and this rule decides it instead: the subcommand
must be in a group the caller's row grants (`[callers]` in `walls.toml`, see [[table:walls]]), and
the person-only subcommands are refused for every caller, including the main session's model.

A row with `tool_only = true` (the clerk and the four planners) may run nothing but calls of the tool,
written as `vsdd ...` or `vsdd -C <dir> ...` in a single segment, with no `cd` in front, no pipe and no
redirect. `-C <dir>` is a global option of the tool ([[packaging:cli-entry]]), which is why a clerk
never needs a `cd`.

An agent that is blocked by a gate therefore cannot switch the gate off, and cannot weaken a gate
indirectly by changing the test command, the source globs or a threshold: only a person can change the
`[hooks]` table (see [[topic:rule-tuning]]).

Related: the person-only `decide` subcommand is what makes [[rule:decisions]] "only by a person" true.

## Team use (decision D33; built when the build widens)

`claim take` joins the person-only list ([[command:claim-take]]).
