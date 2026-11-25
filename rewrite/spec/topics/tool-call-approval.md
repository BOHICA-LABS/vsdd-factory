---
kind: topic
id: tool-call-approval
sources: [hooks.md#2.11.10, hooks.md#2.6, BLUEPRINT.md#5.4, BLUEPRINT.md#2.4, BLUEPRINT.md#2.1]
---

# Tool-call approval (decision D4)

Decision D4, adopted: the plugin may approve its own routine tool calls, so that unattended runs do
not stop on permission prompts. Plugin agents cannot set a permission mode, so in the default mode
every edit, build, test and commit inside a workflow asks the person. A call is approved only when
every other hook rule passes.

## The question and the answer

- The question (BLUEPRINT.md#2.1, D4): "May the plugin approve its own routine tool calls, so that
  unattended runs do not stop on permission prompts?"
- The answer, scoped: edits inside story and spike worktrees and the state directory by the agent
  whose role may write them; the project's configured build and test commands; `git add`, `commit`,
  `status`, `diff` and `log` inside those worktrees; and `vsdd` commands the caller's role allows.
- Where the answer is built (BLUEPRINT.md#2.4): `auto-allow` (build package H6); the storage of the
  answer is part of `vsdd init`.

## The mechanism

The one mechanism is [[rule:auto-allow]] returning `Permit`; static permission rules in a settings
file are only the fallback if a platform change breaks the hook permit, because static rules cannot be
scoped by agent role. The rule has two parts: a narrow built-in permit (the plugin's own method,
template and asset files, and a single `vsdd` call the caller's row grants), and the project's chosen
scope. What each scope approves (`off`, `core`, `extended`) is the scope table in
[[rule:auto-allow]].

## The switch

The answer is stored as `[hooks.rules.auto-allow] scope` in `factory.toml`. `setup` asks the person
before it runs `vsdd init`, and passes the answer as `vsdd init --auto-allow <scope>`, because `init`
is the one moment the plugin writes that file; on a project that is already initialised, `setup` shows
the two lines to add. `vsdd config set` is person-only, so an agent cannot change the scope (see
[[topic:rule-tuning]]). `extended` goes beyond the list in decision D4, so the plugin never turns it
on by itself: `setup` describes both scopes and the person chooses.

The smoke tier counts prompts in the default permission mode for one builder step and one experiment
spike, and the greenfield proof counts them for a wave; with `extended` both expect none
([[rule:auto-allow]] has the rationale).

## Platform facts

The probe of 2026-11-23 proved the permit in the default permission mode, in the main session, a
dispatched agent and a workflow step; in the `auto` permission mode the platform allowed the
unapproved control call as well. Whether the permit is honoured for the `Workflow` tool, and the
fallback if it is not, are in [[topic:hook-platform-facts]].
