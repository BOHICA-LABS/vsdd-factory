---
kind: topic
id: rule-model
sources: [hooks.md#2.5, hooks.md#2.1]
---

# The rule model

What a rule is: one Python function with a declared identity, events, tools and failure behaviour. A
rule is declared once, next to its code: the module holds `check` and nothing else, and one
`RuleEntry` row in the registry's `RULES` table is its declaration (HD1):

```python
# vsdd/rules/tests_first.py — the whole module:
def check(ctx: Ctx, cfg: RuleConfig) -> Outcome | list[Outcome] | None: ...

# vsdd/hooks/registry.py, in RULES — the row is the declaration:
RuleEntry(id="tests-first",
          on=(("PreToolUse", ("Edit", "Write", "MultiEdit", "NotebookEdit", "Bash")),),
          kind="gate",            # gate | advisor | recorder
          fail="closed",          # what a crash or the deadline turns into: closed = block, open = log and continue
          locked=False,           # locked rules cannot be relaxed in project configuration
          spike_exempt=True,      # skipped when the work is inside a spike
          summary="Product source in a story is written only after a failing test run is recorded.")
```

The individual declarations are in each rule file: [[rule:gate-guard]], [[rule:secrets]],
[[rule:read-walls]], [[rule:write-scopes]], [[rule:artifacts]], [[rule:tests-first]],
[[rule:merge-guard]], [[rule:bash-destructive]], [[rule:bash-git]], [[rule:spike-bounds]],
[[rule:decisions]], [[rule:step-results]], [[rule:session-briefing]], [[rule:compaction]],
[[rule:lifecycle-log]], [[rule:purity-advice]], [[rule:auto-allow]].

## What a rule receives

`Ctx` is built once per call; the derived fields are computed on first use.

| Field | Content |
|---|---|
| `event`, `tool`, `tool_input`, `raw` | The payload as Claude Code sent it. A missing payload string is `""`; `event` and `tool` are the payload's `hook_event_name` and `tool_name`, and `permission_mode` and the launch mode ride along as `permission_mode` and `mode`. |
| `session_id`, `agent_type`, `agent_id`, `cwd` | Who is calling and from where. `agent_type` is empty for the main session, `vsdd-factory:<name>` for a plugin agent, `workflow-subagent` for a workflow step with no agent type (all verified by the spike). |
| `row` | The wall-table row for `agent_type`: the agent's own row, `main` for the main session, `other` for any type that is not listed — `workflow-subagent` is `other`. The row carries `.name` (main, other or the agent type), `.data` (the raw walls.toml row) and `.spike` (the row's `spike` flag). |
| `project` | The governing project: root, state directory, parsed configuration, policy tables. Found by walking up from the path being touched to the nearest state directory, falling back to the session's project; `ctx.project_at(path)` resolves another path. Story worktrees and the `repo` targets of a multi-repository project resolve correctly. What the object adds is listed below the table. |
| `target` | For file tools: the absolute path the call reads or writes, normalised, with symlinks resolved. The literal is `file_path` (`Read`, `Edit`, `Write`, `MultiEdit`), `notebook_path` (`NotebookEdit`) and `path` or `cwd` (`Grep`, `Glob`); `target` has symlinks resolved and `target_literal` does not. Class matching tests both the literal and the resolved path. |
| `classes` | The path classes of `target` — see "Path classes" below. `()` when the call names no file. |
| `command` | For `Bash`: the parsed command (segments, tokens, redirect targets, `cd` and `git -C` prefixes, command families). |
| `new_content` | For `Write`, `Edit` and `MultiEdit`: the text the file would contain after the call. This is what lets validation run before the write. Built by H4; reading it before then raises `ContextUnavailable`. |
| `where` | The call's effective directory, defined below. A `Bash` command has one per segment. |
| `story` | The story whose branch is checked out in the worktree that contains `where`, if any; `story_at(dir)` answers for one segment's directory, `branch_at(dir)` reads that directory's branch. |
| `in_spike` | True when the caller's row is the spike row, or the worktree that contains `where` is a spike (`spike_at(dir)`, by `vsdd.project.detect`: worktree name or `spike/*` branch). |
| `log(type, severity="info", **fields)` | Adds an event to the session project's event log, adding `agent_type`, `agent_id`, `event`, `tool` and `ms` (the call's age in milliseconds). A field the caller names wins over the automatic one. Write errors are swallowed: the event log cannot take a call down with it. |

The project object is a `vsdd.project.Project` subclass, so a state function that takes a project
takes it, and adds:

- `.hooks_dir` — `<state>/local/hooks`, the runner's own store,
- `.config` — a `Config`; the defaults when the file does not parse, with `.config_error` saying why,
- `.policy` — the shipped tables; raises `PolicyInvalid` when one is missing or invalid,
- `.rule_config(id)` — the `[hooks.rules.<id>]` table as a `RuleConfig`; a rule whose own table is
  faulty resets to its defaults, and any other fault already reset the whole file.

Path classes. `classes` and `classes_of(path)` come from the hook runner's `paths.classes_of`, with
[[table:path-classes]]: in the state directory every class whose globs match, innermost first (a
contained class before its container, so `brief` before `spec-product`); elsewhere the product class
by S1's `path_class`, `unclassified` when no glob matches; plus the computed classes `project`,
`own-worktree` (by the caller's working directory — the payload `cwd` — or the directory passed to `classes_of`), `tmp` and `outside`. A policy table
that does not load raises `PolicyInvalid`, which is the reading rule's crash.

The effective directory. A workflow step that is not isolated starts every call in the project root, so the
payload's `cwd` is the same for every story running at once and says nothing about which worktree a call
works in. A rule therefore never tests `cwd` alone. `where` is, for a file tool, the directory of `target`;
for a `Bash` segment, the directory that a `cd` earlier in the command, a `git -C` or a `vsdd -C` names,
resolved against `cwd`, and `cwd` when the segment has none. "The caller's worktree", "inside a story
worktree" and "while a branch is checked out" in any rule mean the worktree that contains `where`; the one
exception is `own-worktree` of [[table:path-classes]], which answers where this caller may work and is
defined in that table ([[rule:spike-bounds]]). Two cases
hold it: `cd <story worktree> && git commit` while the main checkout is on the integration branch is
allowed, and a bare `git commit` there is blocked ([[rule:merge-guard]]).

## What a rule can return

| Outcome | Meaning | How `emit.py` delivers it |
|---|---|---|
| `None` | Allow; the rule has no opinion. | Nothing. |
| `Block(code, reason, fix)` | Stop the action. `reason` says what was attempted and why it is not allowed; `fix` names the correct route. | Exit 10 from the runner, which the launcher turns into exit 2, with the message on stderr. Nothing on stdout. This is the path the spike proved, and it is the same for every blocking event. |
| `Advise(code, message)` | Allow, and tell the model something. | Exit 0 with `hookSpecificOutput.additionalContext`. |
| `Context(text)` | Add context (briefing, compaction instructions). | `additionalContext` on `SessionStart`, `UserPromptExpansion` and `SubagentStart`; plain stdout on `PreCompact`, where Claude Code uses it as instructions for the summary; dropped on the other events. A context's `system` becomes the output's `systemMessage`. |
| `Permit(reason)` | `PreToolUse` only: skip the permission prompt for this call. | `permissionDecision: "allow"`. Honoured only if no rule returned `Block` or `Advise`. Only `auto-allow` returns it. |
| `Rewrite(tool_input, reason)` | Return `updatedInput` for the call; `tool_input` is the whole replacement input. | On `PreToolUse`, inside the one `hookSpecificOutput` object, paired with a permission decision (below). A rule may return it only when its row sets `rewrites` — otherwise it is that rule's crash. |

All rules selected for a call run, even after one blocks, so the model sees every reason at once.

## What the runner itself says

Every block is printed with its own id (`b-` and six hexadecimal characters), one on stderr and the
same in its `rule.block` event, so `vsdd explain` and the survey can refer to one call's reason. The
advice of an `Advise` outcome is a line of its own, `vsdd-factory advice [<rule>/<code>]: <message>`
one per advising rule, and a permit is honoured only when no rule blocked or advised. A `PreToolUse`
call that nothing has to say about prints nothing at all.

The runner raises three blocks of its own, under the rule id `runner`: `runner_bad_input` for a
payload that is not a JSON object or carries no `hook_event_name` (a block in gate mode, the fixed
fallback briefing in brief, logged in notify, under the session id `unknown`), `caller_forged` for a
Bash command that sets `VSDD_CALLER=` itself, and `caller_marker_unwritable` when the marker of
[[record:caller-marker]] cannot be written — a `vsdd` call that passed unmarked would report dead
hooks, the wrong fault. The other two name the rule concerned: `rule_error` for a crash and
`runner_timeout` for the deadline; in gate mode a fail-closed rule's crash and any deadline are
blocks, and a policy fault carries `policy_invalid` for the rules that read the table.

The decision sent with a rewrite is `allow` when a `Permit` stands, `agent_id` is set (an agent or a
workflow step), the tool is `StructuredOutput`, or `permission_mode` is `auto` or `bypassPermissions`;
otherwise `ask` — the main session in a mode that prompts. An `ask` paired with a rewrite never runs
unattended, and `allow` for the main session would widen the project's decision D4.
With no rewrite, `allow` needs a `Permit`. Reasons begin `vsdd-factory hook:`, so one is not taken
for command output. The pairing is never dropped: the platform applies a rewrite whether or not a
decision travels with it, so a rewrite that reached the output step unpaired would silently change
what runs — the runner refuses one instead, as the rewriting rule's crash (probed in round 4, Q4).

On `SessionStart`, until H9's briefing exists, the core says four things, each with a `systemMessage`:
an invalid configuration (logged there only as `config.invalid`), an invalid policy table
(`policy.invalid`), a quarantine in force, and the rules below full strength. A healthy project says
nothing.

## The main loop

Illustrative skeleton of the main loop (HD1, HD4, HD5: the row is the declaration; one alarm per
call, its budget the event row's, raising a `BaseException` a rule's own `except Exception` cannot
swallow; the applicability test runs inside the rule's `try`):

```python
def main(mode: str) -> int:
    payload = read_stdin_json()                     # unreadable input on a gate becomes a Block
    ctx = Ctx(payload, os.environ, mode, os.environ["VSDD_FACTORY_ROOT"])
    outcomes = []
    with alarm(EVENT_ROW[ctx.event].deadline) as d:  # one alarm per call, the event row's budget
        for entry in registry.select(ctx.event, ctx.tool):      # table rows; no rule module is imported yet
            ctx.git_budget_reset()                              # one git subprocess per rule
            d.current = entry.id
            try:
                if not applies(entry, ctx):                     # quarantine, then strength off, then the spike
                    continue                                    #   exemption, and a locked row always blocks
                outcomes += run_rule(entry, ctx)                # imports this rule's module, inside its own try
            except Deadline:                                    # the BaseException the alarm raises
                outcomes.append(timeout_outcome(entry))         # runner_timeout; a block in gate mode only
                break                                           # the rules after it did not run
            except Exception as exc:                            # a failed import is this rule's crash too
                outcomes.append(crash_outcome(entry, exc))      # closed -> Block, open -> logged only
    if mode == "brief" and (a rule crashed or timed out):
        outcomes.append(fallback_context())                     # never a silent session start
    return emit.finish(ctx, outcomes, mode)                      # logs, writes stdout/stderr, returns 0 or 10
```

## The message format

Fixed, for every block:

```
vsdd-factory blocked this Edit call. [tests-first/no_red_evidence b-7f3a2c]
Why: src/lib.rs is product source in story S-1.02, and no failing test run is recorded for that story.
Instead: write the failing test, then run the red gate: vsdd gate red --story S-1.02
More: vsdd explain b-7f3a2c
```

The runner prints the fix text as the rule gave it, adding no quoting and no trailing punctuation. The
platform wraps the message — the hook command line before it, a line naming the plugin after it — so a
block message must read correctly inside that wrapper and must not rely on being the first characters
the agent sees.

## Rules do not write files

With these named exceptions: every rule may log; `step-results` stores the step record and `decisions`
settles a decision, both through the state design's own functions; `spike-bounds` keeps start times and
admissions in the runner's own directory; `compaction` calls the state flush; and the runner itself
writes a caller marker before it lets a `vsdd` call through (see [[topic:caller-identity]]). `Permit`
and `Context` aside, one more thing can leave a rule: `updatedInput`, in two cases and no other. The runner
returns it for a `vsdd` Bash call — what counts as one is stated once in [[topic:caller-identity]]: an
allow-list match of the whole command text, plus the parsed shape — to add the marker's key to the
command; and `step-results` returns it
for a `StructuredOutput` call, to mark findings that stayed ungrounded ([[rule:step-results]]).

## Selection

Registration is an explicit table in `registry.py`, not directory scanning. `main.py` imports only the
rule modules listed for the event and tool at hand, so a `Read` call does not load the Bash parser. Each
module is imported inside its own rule's `try`, so a module that cannot be imported is that rule's error,
logged as `rule.error` like any other crash, and the other rules still run.
Failure behaviour per rule is [[topic:hook-failure]]; the full rule list as data is [[table:rules]];
how a call is dispatched is [[topic:hook-runner]].
