---
kind: rule
id: auto-allow
package: H6
sources: [hooks.md#2.10, hooks.md#2.11.10, hooks.md#2.6, hooks.md#2.3, BLUEPRINT.md#5.4]
---

# auto-allow

New rule (decision D4). Approves the plugin's own routine tool calls so that unattended runs do not
stop on permission prompts. It is the one mechanism for decision D4; static permission rules in a
settings file are only the fallback if a platform change breaks the hook permit, because static rules
cannot be scoped by agent role.

| Registration | |
|---|---|
| Kind | gate-shaped, but returns `Permit` |
| Runs on | `PreToolUse` for `Read`, `Bash`, `Edit`, `Write`, `Workflow`, and for the `extended` scope the web and research tools (they are in the matcher, [[table:events]]) |
| Result | `Permit` (`permissionDecision: "allow"`). Honoured only if no rule returned `Block` or `Advise`; the rule returns `Permit` only when every other rule returned nothing, so a call a wall or a gate would block is never approved. |
| Fails | open — no permit means the normal permission prompt |
| Relax | a scope, not a strength: `off`, `core` or `extended`, written by `vsdd init --auto-allow` from the answer `setup` collected (decision D4). See [[topic:tool-call-approval]]. |

The platform probe of 2026-11-23 proved the permit in the default permission mode, in the main session,
a dispatched agent and a workflow step. In the `auto` permission mode, which the user's own sessions
use, the platform allowed the unapproved control call as well, so there the rule changes little, and it
 matters most for the default mode and for scheduled headless runs. Whether the permit is honoured for
the `Workflow` tool is [[doc:platform]] item 30; if it is not, `setup` writes one static allow rule
for this plugin's workflows.

## Part one: the built-in permit (always on, narrow)

- a `Read` of a file under the plugin's `methods/`, `templates/` or `assets/` directory;
- a `Bash` command that is exactly one call of the `vsdd` command (`vsdd ...` or `vsdd -C <dir> ...`)
  with no pipe, redirect or second segment, whose subcommand the caller's row grants. For the main
  session only read-only subcommands are permitted this way; anything that changes state still asks.

Tool commands take their inputs as arguments or as a file under the temporary directory, never on
standard input, so a clerk's command never needs a redirect. Free text travels percent-encoded in an
argument, and the spike commands take files the runner writes. It exists so that an agent is never
stopped by a permission prompt from reading its own procedure or calling the plugin's own command
(`agents-skills.md` N-HOOK-6).

## Part two: the project's scope (decision D4)

The answer is `[hooks.rules.auto-allow] scope` in `factory.toml`. `setup` asks the person before it
runs `vsdd init`, and passes the answer as `vsdd init --auto-allow <scope>`, because `init` is the one
moment the plugin writes that file; on a project that is already initialised, `setup` shows the two
lines to add.

### `off`

Only the built-in part above.

### `core`

What decision D4 lists, made to work unattended.

- Edits inside story and spike worktrees and the state directory by an agent whose row may write them.
- The project's configured commands (`[commands]` and any `[[component]]` commands), matched as a
  prefix: the configured string followed by further arguments, so that a builder running one test by
  name is approved, with no pipe to a shell and no redirect outside the worktree.
- `git add`, `commit`, `status`, `diff` and `log` inside those worktrees, that is, with the call's
  effective directory in one ([[topic:rule-model]]).
- A `vsdd` call the caller's row grants, including for the main session `vsdd driver`,
  `vsdd checkpoint brief` and `vsdd note`, which the driver and the brief skill run.
- A `Workflow` call by the main session that starts one of this plugin's workflows (`vsdd-factory:*`),
  so the driver is not stopped at each workflow boundary.

### `extended`

`core`, and what a real unattended run also does.

- A fixed list of read-only commands inside the caller's own worktree: `ls`, `cat`, `head`, `tail`,
  `wc`, `rg`, `grep`, `find` without `-exec`, `-delete` or `-ok`, and the `git-read` family.
- A `curl`, `wget` or browser call to the loopback port of a product that `vsdd env start` started
  for the run, by a row that holds the `env` group.
- Any Bash command by the spike runner whose working directory and every path lie inside its own spike
  worktree, and which [[rule:bash-destructive]], [[rule:bash-git]] and [[rule:secrets]] pass: this is
  what lets an agent-requested experiment run without a person.
- `WebFetch`, `WebSearch` and the research tools for the rows whose tool list has them, still under
  `secrets`.

`extended` goes beyond the list in decision D4, so the plugin never turns it on by itself: `setup`
describes both scopes and the person chooses. With `core` alone a wave in the default permission mode
still raises prompts for builders' read-only commands and for every spike experiment. Without any
scope, plugin agents (which cannot set a permission mode) prompt for every such call and an
unattended run stalls. The smoke tier counts prompts in the default permission mode for one builder
step and one experiment spike, and the greenfield proof counts them for a wave; with `extended` both
expect none.

## Interaction with the caller marker

The caller-marker rewrite is one more thing `auto-allow` (and `tool_only`) must accept: the runner adds
the `VSDD_CALLER=<key>` prefix after every rule has judged the command as the caller typed it. See
[[topic:caller-identity]].
