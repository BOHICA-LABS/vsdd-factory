---
kind: topic
id: hook-platform-facts
sources: [hooks.md#2.16]
---

# Platform facts the hook design relies on

The status of each fact, and what the design does if it turns out false. Beyond these, the platform
facts common to all designs are in [[doc:platform]].

| Fact | Status | If it turns out false |
|---|---|---|
| Exit 2 from a uv-launched Python hook blocks a tool call and the model sees stderr | Proven by the spike | n/a |
| `agent_type` is in the payload for calls inside a subagent and empty in the main session | Proven by the spike | n/a |
| `SessionStart` `additionalContext` reaches the model on startup, resume and compact | Proven by the spike | n/a |
| `PreCompact` fires; hooks fire for the `Workflow` and `StructuredOutput` tools | Proven by the spike | n/a |

Six facts share one status and one fallback. They are `onFailure: "block"`; `PreToolUse`
`additionalContext`; `PreCompact` stdout becomes summary instructions; the plugin `SessionEnd` budget
of 1.5 s; a `WorktreeCreate` hook replaces worktree creation; and `UserPromptSubmit` also fires for
prompts no person typed. Status: hooks documentation, read 2026-11-23. If they turn out false: the
launcher blocks without `onFailure`, and the self-guarding command covers missing plugin files;
`onFailure` is not relied on for either. Advice would move to `systemMessage`. The compaction
instruction is a bonus.

| Fact | Status | If it turns out false |
|---|---|---|
| `UserPromptExpansion` fires for `/vsdd-factory:decide`, carries `command_name` and `command_args`, can add context, and a block ends the prompt before any model reads it | Proven by the probe of 2026-11-23, headless | n/a |
| `PreToolUse` on `StructuredOutput` carries the whole result in `tool_input`, and a block makes the agent correct the result and call again | Proven by the probe of 2026-11-23 | n/a. If a later release changes it: the fallback in [[rule:step-results]]. |
| Exit 2 on `SubagentStop` keeps an agent running | Probed 2026-11-23: it holds an agent that returns text; it does not hold a workflow step that has already returned a schema result | The design follows the observation: a workflow step is held by blocking its `StructuredOutput` call, a dispatched agent by refusing its stop ([[rule:spike-bounds]]). |
| A hook's `permissionDecision: "allow"` is honoured in the main session, a dispatched agent and a workflow step | Proven by the probe of 2026-11-23 in the default permission mode. In the `auto` mode the unapproved control call was allowed as well | Static allow rules written by `setup`, which cannot be scoped by role. |
| A hook's `updatedInput` rewrites a Bash command, in every context | Proven with allow: auto mode in the main session, a dispatched agent and a step ([[topic:platform-proven]], item 27); default mode in a step ([[topic:platform-proven-2]], item 38) | n/a. If a later release changes it: the caller marker is named by a hash of the call's arguments ([[topic:caller-identity]]). |
| Tool calls inside a workflow step carry the main session's `session_id` | Proven ([[topic:platform-proven]], item 28) | n/a. If a later release changes it: the lock is kept alive by calls that name the run or touch its worktrees ([[topic:run-lock]]). |
| The permit is honoured for the `Workflow` tool | Proven in the default mode ([[topic:platform-proven]], item 30) | n/a. If a later release changes it: one static allow rule for this plugin's workflows. |
| The payload `cwd` of a worktree-isolated agent is its worktree | Proven by the spike | n/a |
| `Grep` skips a directory listed in `.git/info/exclude` | Proven by the probe of 2026-11-23 | n/a. If a later release changes it, ancestor searches are blocked for walled callers ([[rule:read-walls]]). |
| `isolation: worktree` in an agent file | Probed 2026-11-23: honoured for a dispatch with the Agent tool, ignored for a workflow step | Every workflow step that needs isolation passes it itself, which the spike wrapper does. |
