# Step 3 plan: hook runner core and hook wiring (H1, then B4b)

Paths are under `next/` unless they start with `rewrite/`. `P` is `plugins/vsdd-factory`. Spec files are under
`rewrite/spec/`.

## 1. Scope

This unit replaces the stub `P/py/hook.py` with the real runner: it reads one call, builds a context, runs the
rules registered for the event and tool under a deadline, and turns their outcomes into the exit code, stdout
and stderr Claude Code sees. It builds the runner's own duties (the caller marker with the command rewrite, a
block on a typed `VSDD_CALLER=`, four session-start notices), the failure paths of `topics/hook-failure.md`
that start in Python, the environment switches, and `P/hooks/hooks.json` generated from the registry with a
drift check and the self-guarding command. It ships no rule: fixture rules under `tests/` prove the core in
process and through the real launcher.

The second part of H1 is its own plan, `hook-runner-b.md`, built after S1 and records: path classes
(`ctx.classes`) and lock liveness with the release at session end.

Left for when the build widens:

- Every rule (H2 to H9) and `vsdd explain`, `rules`, `hooks selftest` (H9).
- `Ctx.new_content` and `hooks/edits.py` (H4); `hooks/records.py` (H5 or H7); command families (H2); the
  main-session wall switch (H3); a spike runner's `own-worktree` from its ledger entry (H8).
- The configuration fingerprint and the full briefing (the rest of H1, with H9); the agent records and
  event-file pruning of `lifecycle-log` (H9); the argument-hash marker fallback (item 27 is proven);
  recorded payloads (E2; hand-written here).

## 2. Files

Owned (only this unit writes them), all new:

- `P/py/vsdd/hooks/`: `__init__.py` (docstring only), `model.py` (the types and `Ctx`), `registry.py` (3.3),
  `main.py` (loop, deadline, failure, strength, quarantine, switches, notices), `emit.py` (the only writer of
  stdout, stderr and the exit code), `caller.py` (3.5), `shell.py`, `gitfs.py`, `policy.py` (3.6).
- `P/py/vsdd/rules/__init__.py` (docstring only),
  `P/schemas/caller-marker.schema.json`, `P/hooks/hooks.json` (generated; `P/hooks/.gitkeep` goes).
- `tests/hooks/runner_helpers.py` (3.7), `fixture_rules/*.py`, `payloads/**`, `fixture_cases/probe.toml`, the
  test files of section 7, `tests/unit/fixtures/schemas/caller-marker/*`, `.../event/valid_rule-block.json`.

Shared, edited before records starts. `P/py/vsdd/project.py` is not among them: S1 makes its `home`
change (state-core.md, section 2), and this unit passes `home` itself to `find_project_root`.

| File | Other writer | What this unit changes |
|---|---|---|
| `P/py/hook.py` | B2 | After the floor check the stub body becomes `return vsdd.hooks.main.main(argv)`. |
| `P/schemas/event.schema.json` | C1 | Adds the string properties `cwd` and `story`. |
| `tools/build.py` | B4a, then records | A fifth step in `build()`, for `hooks.json` (3.7); `caller-marker.schema.json` joins `NON_PRELUDE_SCHEMAS`. |
| `tools/check.py` | B2, B6, then records | `caller-marker.schema.json` joins `required` in `schemas_are_the_single_source`; the gate payload in `launcher_modes_on_a_live_project` becomes a full `PreToolUse` payload (HD11). |

Records then adds its seven schema names to the same two sets.

## 3. Interfaces this unit provides

### 3.1 The hook entry

`sh P/py/launch.sh hook <gate|brief|notify>` is unchanged (`topics/hook-launcher.md`). `hook.py` exits 0 (allow;
stdout holds at most one JSON object, or plain text on `PreCompact`), 10 (blocked; reasons on stderr, nothing
on stdout) or 13 (floor). Any other code is the runner's own crash, which the launcher maps.

```python
# vsdd/hooks/main.py
def main(argv: list[str]) -> int            # argv[0] is the mode
def run(mode: str, payload_text: str, env: Mapping[str, str], *,
        entries: Sequence[RuleEntry] | None = None, stdout=None, stderr=None) -> int   # 0 or 10
```

`entries=None` means `registry.RULES`. `run` returns 0 silently when `VSDD_FACTORY_ROOT` is unset, or its real
path is the real home directory or `/`.

### 3.2 The rule interface

A rule is one module `vsdd/rules/<id with underscores>.py` exposing `check`, plus one row in `registry.RULES`.
The row is the declaration (HD1).

```python
def check(ctx: Ctx, cfg: RuleConfig) -> Outcome | list[Outcome] | None

@dataclass(frozen=True)
class RuleEntry:
    id: str
    on: tuple[tuple[str, tuple[str, ...]], ...]  # (event, tool atoms); () means every call of the event
    kind: str                                    # "gate" | "advisor" | "recorder"
    fail: str                                    # "closed" | "open"
    locked: bool = False
    spike_exempt: bool = False
    rewrites: bool = False                       # may return Rewrite; step-results only
    summary: str = ""
    module: str = ""                             # default "vsdd.rules." + id.replace("-", "_")
    def load(self) -> Callable                   # imports the module, returns check

Block(code, reason, fix)  Advise(code, message)  Context(text, system="")  Permit(reason)
Rewrite(tool_input: dict, reason)  RuleConfig(strength, options)      # frozen dataclasses
```

`Rewrite.tool_input` is the whole replacement input. `strength` is `block`, `advise` or `off`; a locked rule
always gets `block`. `options` holds the rule's other keys under `[hooks.rules.<id>]`. `Context.system`
becomes `systemMessage`. Anything `check` raises is the rule's crash (3.4). `Ctx` has the fields of
`topics/rule-model.md`, derived ones computed on first use. The spec leaves open:

| Field | Decided here |
|---|---|
| payload fields | `event` and `tool` are `hook_event_name` and `tool_name`; a missing string is `""`. Added: `permission_mode`, `mode`. |
| `project` | A subclass of `vsdd.project.Project`, so a state function that takes a project takes it; `.root` is the main checkout. Added: `.hooks_dir` (`<state>/local/hooks`), `.config` (a `Config`; the defaults when invalid), `.config_error`, `.policy` (raises `PolicyInvalid`), `.rule_config(id)`. Found from `target`, else `cwd`, else the session root; `ctx.project_at(path)` for another path. |
| `target`, `target_literal` | `file_path` (`Read`, `Edit`, `Write`, `MultiEdit`), `notebook_path` (`NotebookEdit`), `path` or `cwd` (`Grep`, `Glob`); else `None`. Absolute; `target` has symlinks resolved. |
| `where`, `command` | The directory of `target`, else `cwd`. `command` is `shell.parse(command, cwd)` for `Bash`, whose rules read `segment.where`. |
| `story`, `in_spike` | `story_at(where)`; `row.spike or spike_at(where)`. For a segment: `ctx.story_at(dir)`, `spike_at(dir)` (by `vsdd.project.detect`), `branch_at(dir)` (`gitfs.branch`). |
| `row` | `.name` (`main`, `other` or the agent type), `.data` (the `walls.toml` row), `.spike`. `workflow-subagent` is `other`. |
| `classes` | Built by the second part (`hook-runner-b.md`); until then reading it raises `ContextUnavailable`. |
| `new_content` | Not built (H4): reading it raises `ContextUnavailable`, the rule's crash. |
| `log(type, severity="info", **fields)` | Adds `agent_type`, `agent_id`, `event`, `tool`, `ms`; the session project's log; write errors are swallowed. |

### 3.3 The registry

```python
# vsdd/hooks/registry.py
RULES: tuple[RuleEntry, ...]                      # rows sit between "# >>> rules" and "# <<< rules"
def select(event: str, tool: str, entries=RULES) -> list[RuleEntry]     # table order; imports no rule module
def wiring(entries=RULES) -> dict
def hooks_json(entries=RULES) -> str              # json.dumps(wiring, indent=2) + "\n"
def guard_command(mode: str) -> str
```

A row is selected when it names the event and lists no tools, or one of its atoms matches the whole tool name
as a regular expression. `EVENTS` holds the seven rows of `tables/events.toml` in order (mode, matcher,
`timeout`, deadline, `onFailure`, `async`). `TOOL_ATOMS` is the ordered list of the `PreToolUse` matcher's
atoms; a rule names an atom by its text. `wiring` raises `WiringError` for an unknown event, a tool that is no
atom, or `async` on an event with a gate rule.

### 3.4 Output and failure

`emit.finish(ctx, outcomes, mode) -> int` runs once per call, on `(rule id, outcome)` pairs, to
`topics/rule-model.md` and the failure table of `topics/hook-failure.md`. Decided here:

- Any `Block`, in any mode: each is printed with its own id (`b-` and six hex characters) and logged as
  `rule.block`; the exit is 10.
- `PreToolUse` without a block prints one `hookSpecificOutput` object holding `additionalContext` (a line per
  `Advise`: `vsdd-factory advice [<rule>/<code>]: <message>`), `updatedInput`, and `permissionDecision` with
  `permissionDecisionReason` (HD7). With nothing to say it prints nothing.
- `Context` is `additionalContext` on `SessionStart`, `UserPromptExpansion` and `SubagentStart`, plain stdout on
  `PreCompact`, dropped elsewhere.
- The runner's own codes: `runner_bad_input`, `caller_forged`, `caller_marker_unwritable` (rule id `runner`),
  and `runner_timeout`, `rule_error`, `policy_invalid` (the id of the rule concerned).

### 3.5 The caller marker (the seam with S2b, and through it S3 and G1)

For a `vsdd` call (HD8) that no rule blocked, the runner writes `<root>/.factory/local/hooks/calls/<key>.json`
and returns the call's whole `tool_input` with `command` set to `VSDD_CALLER=<key> ` and the command as typed.

- `<root>` is `vsdd.project.find_project_root(d)`, where `d` is the directory `vsdd -C <dir>` names, resolved
  against the payload `cwd`, else `cwd`. The tool must find its project the same way. With no such project
  the call passes as typed, unmarked.
- `<key>` is `secrets.token_hex(16)`, new for every call.
- The file is written as `<key>.json.tmp` and renamed, so a reader never sees half of it. It is
  `{"v": 1, "key", "session_id", "agent_id", "agent_type", "cwd", "ts"}`: all required, all strings but `v`.
  `agent_id` and `agent_type` are `""` for the main session; `ts` is as in the event log. A reader needs only
  the four names the spec gives.
- The runner only writes markers. Reading, deleting and pruning them (file time over ten minutes) is
  records' `vsdd.state.callers.caller(key, project=<root>)`.

### 3.6 Helpers step 4 builds on

```python
def shell.parse(text: str, cwd: str) -> Command    # never raises
Command(text, segments: tuple[Segment, ...], opaque: bool)
Segment(text, joiner, assignments: tuple[tuple[str, str], ...], program, args: tuple[str, ...],
        redirects: tuple[Redirect, ...], where, opaque: bool)
Redirect(op, target)
def gitfs.worktree_root(path) -> Path | None       # nearest directory holding .git
def gitfs.branch(path) -> str | None               # the commit when detached
def gitfs.head_sha(path) -> str | None             # read from files
def gitfs.git(ctx, args, cwd) -> CompletedProcess  # 1.5 s; a second call in one rule raises
def policy.tables() -> Policy                      # .walls, .classes, .row(agent_type); raises PolicyInvalid
```

`joiner` is the operator before the segment (`""`, `;`, `&&`, `||`, `|`, `&`, newline). `where` is absolute: a
`cd <dir>` sets it for later segments, `git -C` and `vsdd -C` for their own; `""` when unknowable. `opaque`
marks a substitution, subshell, here-document, `eval`, `sh -c` or `xargs` the parser does not enter.

### 3.7 The build (B4b), and test support

`tools/build.py` writes `registry.hooks_json()` to `P/hooks/hooks.json` after its four existing steps. `--check`
prints `DRIFT plugins/vsdd-factory/hooks/hooks.json` and exits 1 when the file differs. After this unit the
file wires `SessionStart` (no matcher, the `brief` guard, timeout 15) and `PreToolUse` (matcher `Bash`, the
`gate` guard, timeout 10, `onFailure: "block"`); the second part adds `StructuredOutput` and `SessionEnd`.

For step 4, `runner_helpers.py` gives `make_project(tmp, *, story=None, branch=None)` (a `factory.toml`
only; a test that needs state uses S1's `tests/unit/_repo.make_project`), `payload(name, **set)`,
`run_hook(payload, mode, entries=..., env=...)` and `launch_copy(tmp, rules=[...])` (rules go in at the
`# <<< rules` anchor); `test_rule_cases.py` runs every `tests/hooks/cases/*.toml`.

## 4. Interfaces this unit needs

- B2: `launch.sh` and `find-root.sh` as built. B3: `vsdd.events.EventLog(directory, session_id)` with
  `.log(type, severity, **fields)` and `.read(day)`, `events_dir_for(root)`; `vsdd.project.find_project_root`,
  `detect`, `Project`, `read_branch`; `vsdd.schema.validate` (tests).
- `vsdd.config` as it stands (S1's changes keep these): `load(path) -> Config`, `loads(text)`,
  `ConfigError`, `Config.get("hooks", "rules")`, `Config.hook_problems`. The defaults are
  `loads("schema = 1\n")`.
- S1: its `-C` finds the project as 3.5 says, and it makes the `home` default change in `project.py`.
  This unit writes no path-class resolver or glob matcher: the second part uses S1's.
- C2 (built): `vsdd.policy.load("walls")` and `load("classes")`; `.factory/local/**` is written by code only.
- Records: `vsdd.state.callers.caller(key, project=<root>)` reads the marker of 3.5. This unit calls
  nothing in `vsdd.state`; the second part calls `vsdd.state.locks`.

## 5. Design decisions

- **HD1. The registry row is the rule's one declaration; there is no `@rule` decorator.** An import crash
  must become the rule's declared failure, which needs `kind` and `fail` before the module loads.
- **HD2. Wiring is derived.** An event is wired when a rule or a core duty names it; the `PreToolUse` matcher
  is the atoms they name, in `TOOL_ATOMS` order. Two duties are wired with no rule: `PreToolUse` on `Bash`
  (the marker) and `SessionStart` (the launcher's and the guard's fixed lines, and HD6). The second part
  adds lock liveness as two more.
- **HD3. A crash or the deadline blocks only in `gate` mode.** There a fail-closed rule's crash is `rule_error`,
  and the deadline is always `runner_timeout`, because the rules after the running one did not run. In `brief`
  and `notify` the exit is 0 and `brief` prints a fixed fallback line. Every mode logs `rule.error` or
  `rule.timeout`.
- **HD4. Deadlines are per event, one alarm per call:** 4 s on `PreToolUse`, `UserPromptExpansion` and
  `SubagentStop`; 8 s on `SessionStart`; 25 s on `PreCompact`; 1 s on `SubagentStart` and `SessionEnd`. The
  alarm raises a `BaseException`, so a rule's `except Exception` cannot swallow it. `SessionEnd` gets no
  `timeout` key: the platform's budget governs it.
- **HD5. `applies` runs inside the rule's `try` and tests quarantine, then strength `off`, then the spike
  exemption.** Quarantine needs `VSDD_FACTORY_HOOKS_QUARANTINE` to name the rule and two or more `rule.error`
  or `rule.timeout` events for it in the two newest daily log files (the spec says only "repeated").
- **HD6. Until H9's briefing exists the core says four things on `SessionStart`,** each a `Context` with
  `system` set: an invalid configuration; an invalid policy table; a quarantine in force; the rules below
  full strength. Otherwise step 4's gates could be relaxed in silence. `config.invalid` and `policy.invalid`
  are logged there only.
- **HD7. The decision sent with a rewrite** is `allow` when a `Permit` stands, `agent_id` is set (an agent or a
  workflow step), the tool is `StructuredOutput`, or `permission_mode` is `auto` or `bypassPermissions`;
  otherwise `ask` (the main session in a mode that prompts). `ask` with a rewrite never runs unattended
  (PR3-3), and `allow` for the main session would widen the project's decision D4. Until H6 nothing refuses a
  subcommand an agent's row does not grant. With no rewrite, `allow` needs a `Permit`. Reasons begin
  `vsdd-factory hook:`, so one is not taken for command output. **Open question for the person**
  (`ORDER.md`): an agent's single-segment `vsdd` call is approved even when
  `hooks.rules.auto-allow.scope` is `off`, though D4 makes approving the plugin's own calls an opt-in.
- **HD8. A `vsdd` call is a command of exactly one segment, not opaque, with no redirect and no leading
  assignment, whose program's last path part is `vsdd`** (the form `rules/auto-allow.md` permits). The decision
  sent with the rewrite approves the whole command, so it may hold nothing else. Any other command that runs
  `vsdd` passes unmarked, and a subcommand that needs its caller refuses.
- **HD9. The runner blocks a `Bash` command whose text contains `VSDD_CALLER=`** (`caller_forged`), which
  otherwise waits for `gate-guard` (H6); writes into the marker directory stay open until H3 and H6. A marker
  that cannot be written blocks too (`caller_marker_unwritable`): let through, the survey would report dead
  hooks, the wrong fault.
- **HD10. `Rewrite` is a fifth outcome type**, taken once per call from a rule whose row sets `rewrites`, else
  that rule's crash. The spec gives a rule no way to return `updatedInput`.
- **HD11. A payload that is not a JSON object, or lacks `hook_event_name`, is bad input:** `runner_bad_input` in
  `gate` mode, the fallback line in `brief`, logged in `notify`, under the session id `unknown`.
- **HD12. The guard takes `find-root.sh`'s two precautions:** it walks only from an absolute path (the spec's
  loop never ends on a relative one) and strips a trailing slash from `HOME`.
- **HD13. The marker's time key is `ts` and its schema file ships,** because two units meet at it and the
  spec's schema names four fields and no types. Pruning is the reader's (records.md).

## 6. Findings

| Id | Outcome |
|---|---|
| P3COV-07, PR3-3 | HD7. `test_decision_pairing`. Static allow rules for the prefixed command: not at slice depth, H6. |
| P3BLD-26 | Not taken: with hooks off, a copied command could replay plain facts, not a one-time key. `test_marker_and_prefix`. |
| PR3-2 (H1, B4b) | The guard is built as designed, with HD12. `test_guard`. |
| P3OPS-18 | `SubagentStop` gets the 4 s deadline: `test_shape`. The platform-timeout probe and any watchdog: not at slice depth, B8 then B2. Announcing `rule.timeout`: H9. |
| SL1-48, SL1-14, SL1-40 | S1 makes the `project.py` change; this unit passes `home` itself. Both resolvers share `tests/root_cases.py`, relative `gitdir` included. The guard normalises `HOME`; the runner refuses a root that is really home. `test_project_resolution`, `test_guard`, `test_root_at_home_is_inert`. |
| SL1-47 | `config.py` owns the check: a fault in one rule's table resets that rule, any other fault resets all; the runner announces it. `test_applies_and_strength`, `test_degraded_inputs`. |
| SL1-50, AU-P2-10 | `story` and `cwd` join the shipped event schema; the design's example line is the fixture `valid_rule-block.json`. `test_block_format`. |
| SL1-21, SL1-17, SL1-18 | Already in `next/`, unchanged; `tests/unit/` holds them. |
| P3BLD-14, P3OPS-20, P3SEAM-24 | The second part (`hook-runner-b.md`). |
| Not at slice depth | L2-catchup-fix-3, AU-S2-09: T3 (`vsdd events schema`); here each type is one constant in `model.py`. P3OPS-23, P3REQ-13: the rest of H1 with H9 (a launcher line for an unmounted state branch), and S1 (`vsdd status`). AU-S2-06, AU-S2-10: spec text, when H1's spec files shrink to pointers. |

## 7. Tests

Under `tests/hooks/` unless a path is given. Launcher tests run under sh, bash and dash.

`test_runner.py`
- `test_select`: table order; whole-name atom match; an unselected rule's module is never imported.
- `test_all_rules_run_after_a_block`: two reasons, exit 10.
- `test_applies_and_strength`: quarantine (one error is too few), `off` and the spike exemption skip; `advise`
  logs `rule.would_block`; a faulty rule table resets that rule only.
- `test_bad_input_blocks_via_launcher`: garbage on stdin in `gate` mode exits 2 with `runner_bad_input`
  (the loud failure). `test_import_crash_blocks_via_launcher`: a broken `emit.py` exits 2 in `gate`, 0 in
  `notify`.
- `test_rule_crash`: closed blocks with the quarantine line, open is logged, only `gate` blocks.
- `test_deadline`: a sleeping rule that catches `Exception` still gives `runner_timeout` with its id.
- `test_degraded_inputs`: an unwritable log still prints the block; a broken `factory.toml` relaxes nothing;
  a missing policy table is `policy_invalid` for the rule that reads it.
- `test_session_notices`: HD6's four lines, each with `systemMessage`; none on a healthy project.
- `test_root_at_home_is_inert`; `test_env_switches` (`runner.call`, `runner.slow`, a captured payload).

`test_emit.py`
- `test_block_format`: four lines; one id on stderr and in the event, which has the design's fields.
- `test_stdout_by_outcome`: advice; context per event; a permit only with no block or advice; a `Rewrite`
  from an unflagged rule is its crash.

`test_ctx.py`, `test_shell.py`, `test_gitfs.py`
- `test_payload_target_and_where`: three caller shapes; literal and resolved paths; `where` per segment.
- `test_project_resolution`: a story worktree; an outside path gets the session's project, a
  `vsdd.project.Project`; state at home adopts nothing; story and spike by worktree name and by branch.
- `test_row`: `main`, `other`, an agent; `new_content` and, in this part, `classes` raise.
- `test_parse_table`: joiners, quoting, redirects, `cd`, `-C`, an unknown directory, each opaque construct.
- `test_gitfs`: branch and head in a main checkout and a linked worktree; a second `git` call raises.

`test_caller.py`
- `test_marker_and_prefix`: a schema-valid file, a new key per call, other input fields kept, `-C` honoured.
- `test_decision_pairing`: HD7's cases; each reason has the fixed prefix.
- `test_no_marker`: none for a compound command, a redirect, a blocked call, or a directory in no project.
- `test_typed_caller_blocked`: in process and through the launcher, exit 2.
- `test_write_failure`: a read-only directory blocks; an old marker is left for the reader.

`tests/lint/test_hooks_wiring.py`
- `test_current_and_drift`: the file is current; a changed byte makes `build.py --check` exit 1.
- `test_shape`: a top-level `hooks` key; timeouts at most 60; gate handlers carry `onFailure`; `EVENTS` holds
  HD4's values; with every atom named, the design's matcher.
- `test_generator_refusals`: `async` with a gate rule; an unknown tool; an unknown event.
- `test_reachability`: every rule is reachable; every wired event has a rule or a duty.
- `test_guard`: launcher missing: exit 2 with the reinstall message in a factory project, silent 0 elsewhere,
  `systemMessage` JSON for `brief`, 0 for `notify`; launcher present: its code comes back; a trailing slash
  on `HOME`, state at home and a relative start adopt nothing.

`test_rule_cases.py`, `test_latency.py`
- `test_case_table_both_ways`: the probe table, in process and through the launcher.
- `test_gate_case_coverage`: every gate row has a blocking and an allowing launcher case; modules match rows.
- `test_gate_latency`: the median of twenty real launches is under 120 ms.

## 8. Order of work

Before the builder starts: the catch-up unit has landed and C2's tables and loader exist; nothing is needed
from B6. The unit runs beside S1 and jobs (`ORDER.md`) and edits no file of theirs. Records waits for it,
because of steps 5 and 7.

1. `model.py` and `registry.py` (`EVENTS`, `TOOL_ATOMS`, empty `RULES`, `select`).
2. `emit.py`, `main.py`, the `hook.py` change, the helpers, fixture rules and payloads; `test_runner.py`,
   `test_emit.py`.
3. `shell.py` and `gitfs.py` with their tests.
4. The `Ctx` fields that need no policy table.
5. `caller.py`; the marker schema, its fixtures and, in the same step, its name in the two sets of
   `check.py` and `build.py`, so the shared check never sees the file without its name; `test_caller.py`.
6. `policy.py`: `row`.
7. B4b: `wiring`, `guard_command`, `hooks_json`; the `build.py` step; `hooks.json`; the `check.py` payload
   and event schema edits; the wiring tests.
8. `test_rule_cases.py`, `test_latency.py`; then `tools/check.py`, pytest and `tools/build.py --check`, clean.
