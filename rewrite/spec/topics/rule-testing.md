---
kind: topic
id: rule-testing
sources: [hooks.md#2.14]
---

# How the rules are tested

Tests are pytest, in `tests/hooks/` ([[packaging:tests-python]]). The runner stays stdlib-only; pytest is a
development dependency only. [[packaging:tests-python]] supplies the directories, the launcher test, the
meta-test and the matrix; this topic supplies the formats and the cases, and the step-result and
decision-record formats its need H7 asks for are [[schema:step-result]] and [[schema:decision-record]].

## Recorded payloads

A recorded payload is the JSON Claude Code sent on stdin, captured by setting
`VSDD_FACTORY_HOOKS_CAPTURE=<dir>`, with three substitutions so it can be replayed anywhere: the
project root becomes `{root}`, the home directory `{home}`, the plugin root `{plugin}`. Payloads are
stored as `tests/hooks/payloads/<Event>/<name>.json`; `tests/hooks/payloads/_meta.json` records the
Claude Code version and the capture date. The set must contain:

- every wired event and tool ([[table:events]]);
- the main session, a plugin agent, a workflow step and a worktree-isolated agent;
- a `StructuredOutput` call from a workflow step, with and without findings;
- a typed `/vsdd-factory:decide` command;
- a `SubagentStop` for a workflow step;
- each `SessionStart` source;
- manual and automatic `PreCompact`.

## Rule cases

A rule case is one table in `tests/hooks/cases/<rule>.toml`:

```toml
[[case]]
name    = "implementer cannot read a holdout scenario"
payload = "PreToolUse/read-plugin-agent.json"
set     = { "agent_type" = "vsdd-factory:implementer", "tool_input.file_path" = "{root}/.factory/holdout/HS-001.md" }
project = "walled"                       # a fixture under tests/hooks/projects/, copied to a temporary directory
expect  = { outcome = "block", rule = "read-walls", code = "class_denied" }
launcher = true                          # also run through sh py/launch.sh and assert exit 2 and stderr
```

## The layers

| Layer | What it does |
|---|---|
| Rule cases | Each case runs in-process (rule function, payload, temporary project, outcome and reason code). Every rule has cases that must block and cases that must allow. The cases of the old `destructive-guard.bats`, `protect-secrets.bats` and `hooks.bats` are harvested into these tables. |
| Launcher cases | Cases marked `launcher = true` also run `sh py/launch.sh hook <mode>` as a subprocess and assert on the exit code, stdout and stderr, which is exactly what Claude Code sees. The meta-test fails if any gate lacks one that blocks and one that allows. These run under both `dash` and `bash`. |
| Wiring tests | `hooks.json` equals the generated file; it has the top-level `hooks` key; every timeout is at most 60; no handler with a gate rule is `async`; every rule's event and tool is reachable through a matcher; every wired event has a rule. |
| Failure tests | `PATH` without uv; a runner that raises on import; a rule that raises; a rule that sleeps past the deadline; `UV_PYTHON=3.9` in the environment; an unwritable log directory; invalid configuration; a missing policy table. Each asserts the row of the failure table in [[topic:hook-failure]]. |
| Inert tests | In a directory with no configuration file: exit 0, empty stdout and stderr, no file created, and uv never invoked (a stub `uv` on `PATH` records calls). In a directory with only pre-1.0 state: the same, plus the one fixed line on `SessionStart` (see [[topic:hooks-inert]]). |
| Location tests | The whole suite runs with the plugin copied to a temporary directory outside the repository, so no rule can depend on a path that exists only in this repository. |
| Policy tests | Every agent file has a row in `walls.toml` and the reverse; every class named in a row exists in `classes.toml`; every walled class matches at least one path in the sample project. |
| Performance test | The median `gate` call stays under 120 ms. |
| Live probes | In the end-to-end tier, [[packaging:tests-e2e]]: one call that each of `bash-destructive`, `read-walls` and `tests-first` must block, and one briefing check, asserted from the event log. The same run re-captures the payloads, so a change in Claude Code's payload shape shows up as a diff. |
