# Hook runner and rules: design for vsdd-factory 1.0

> Amended by `rewrite/BLUEPRINT.md` on 2026-11-23, and again on 2026-11-24 after the second review pass
> (blueprint section 11). Where this document and the blueprint differ, the blueprint wins.

## Amendments from the blueprint

The text below has been edited in place where a table, a rule or an example changed. This list is the complete
set of changes; where a passage further down still reads differently, this list and section 4 of the blueprint
apply. Items 1 to 23 are the first pass. The second pass replaced parts of items 1, 4, 7, 8, 12, 14, 15 and
22, as the second-pass list below them (items 24 to 43) says.

1. **A self-guarding hook command.** The command in `hooks.json` first tests that `py/launch.sh` is readable and,
   if it is not, exits 2 with "plugin files missing, reinstall" (exit 0 in `notify` mode). `onFailure` stays as a
   second layer (COV-16).
2. **`hooks.json` is generated** by `tools/build.py` from `registry.py`, under `--check` (SEAM-16).
3. **More tools reach the runner.** The `PreToolUse` matcher adds, in both spellings, the Playwright tools
   `browser_navigate`, `browser_navigate_back` and `browser_file_upload`, plus `WebFetch`, `WebSearch` and the
   research tools. `read-walls` treats a `file:` address as a read of that path and blocks an upload from a
   walled path. Web calls are logged (`web.call`), and one whose address or query contains a known secret value
   or a common token format is blocked. The lint for need A2 counts these tools as inspected. The Bash
   heuristics cover `npx playwright` with `file:` addresses (SEAM-11, OPS-16, OPS-17).
4. **Policy tables.** `classes.toml` is exactly `state.md` 2.4. `walls.toml` has a row for each of the 20 agents,
   including `clerk` (`tool_only = true`, `write = []`) and `scenario-reviewer`; every row denies `runs`; rows of
   agents without a shell, and the clerk, deny reading outside the project, the plugin and the temporary
   directory. `gates.toml` gains `[callers]` (which `vsdd` subcommand groups each row may run) and `[reveals]`
   (which classes each read subcommand prints); `human_only` is `decide`, `config set`, `cycle close`,
   `lock release --force`, `init --fresh`, `import`; `exempt_kinds` is `facade`, `docs`, `chore`, `harness`
   (SEAM-02, SEAM-10, SEAM-35, OPS-15).
5. **`gate-guard`** also blocks: a `vsdd` subcommand the caller's row does not grant; a Bash call that names
   `py/vsdd`, `cli.py`, `hook.py` or `launch.sh` other than through `bin/vsdd`; a settings write that sets a
   `VSDD_*` environment variable or removes this plugin from the enabled plugins; and `claude plugin disable`,
   `claude plugin uninstall` and `claude config` (OPS-14, OPS-15).
6. **Configuration.** `factory.toml` is class `config`: no tool call may write it. The runner fingerprints every
   gate-bearing section, not only `[hooks]`, and the briefing announces a change (SEAM-08, OPS-14).
7. **State's needs.** `artifacts` honours `code_values` and `dir`, names the next free identifier, and checks a
   reference for syntax only (existence is checked at gates). `write-scopes` blocks state writes while the
   mount test fails and blocks reads as well as writes of a `.factory` path inside a linked worktree.
   `lifecycle-log` refreshes the lock's `seen` (at most once a minute, on any guarded call from the holder's
   session) and releases the lock on `SessionEnd`. The five conditions of `state.md` 2.3 replace the four here.
   No flush on agent stop or session end (SEAM-17, OPS-04, OPS-07).
8. **Session marker.** Before a `vsdd` call is allowed the runner writes
   `.factory/local/hooks/sessions/<session id>.json`. `vsdd survey` refuses to start a run without a fresh one,
   so no workflow runs with hooks off. `vsdd hooks canary` remains for `vsdd doctor` and exits 3 (SEAM-22,
   SEAM-26, OPS-14).
9. **`tests-first`** uses the per-gate evidence lookup: the latest passing red record on the story branch since
   its base. Cases are added for an implementer edit after a commit and a test-writer source edit after red
   (SEAM-05).
10. **`merge-guard`** also blocks `git commit`, `--amend` and a branch-moving `reset` while the integration or
    release branch is checked out; for the main session the message names `/vsdd-factory:feature`. `vsdd
    story deliver` and `vsdd release` are the only governed routes; there is no `vsdd pr-merge`
    (SEAM-09, OPS-30).
11. **`decisions`** parses `<id> <choice> [key=value ...] [-- note]` and needs the person-only `decide` skill
    (SEAM-23, SEAM-37, COV-01).
12. **`step-results`** stores a record only when the run is live and the calling agent type is one the step
    name allows (a table generated from the workflows), so a builder cannot forge a review. Grounding resolves
    `location.path` against the unit's tree (`unit_tree`), allows three lines of drift, blocks once per step,
    and then stores ungrounded findings marked advisory. Holdout symptoms are not grounded. The tool validates
    every record against its schema before it counts (SEAM-33, SEAM-43, OPS-10, OPS-15).
13. **`spike-bounds`** keys a spike by `agent_id`; lets a runner read plugin method and template files before
    `spike open`; treats the worktree in the open ledger entry as the runner's own; and applies the read walls
    of the asker recorded in the entry (SEAM-19, SEAM-20, COV-11, COV-17).
14. **`compaction-snapshot`** branches on `agent_type`. For a step agent the instruction keeps the step
    identity, worktree, story, closing command and findings word for word, and does not promise a briefing
    (OPS-08).
15. **`auto-allow`.** The built-in part permits a `vsdd` call (including `vsdd -C <dir>`) when the caller's row
    grants the subcommand; for the main session only read-only subcommands. The project allow-list follows
    decision D4 of the blueprint (COV-04, OPS-12, OPS-15).
16. **`secrets`** adds `.mcp.json`, home credential files (aws, ssh, netrc, docker, gh, npmrc, Claude
    credentials, `*.pem`, `*.key`), `gh auth token`, `git credential fill`, bare `env` and `printenv`, and
    `/proc/*/environ`. `bash-destructive` blocks `curl` and `wget` upload flags for agents (OPS-16).
17. **Messages to a person** name a form that works: terminal commands to install `uv` or Python, the
    `vsdd` shim, or a slash command. The down-briefing also sets `systemMessage` (SEAM-25, OPS-22).
18. **Quarantine.** `VSDD_FACTORY_HOOKS_QUARANTINE=<rule>` skips one rule, only when that rule has logged
    repeated `rule.error` events, and is announced in every briefing (OPS-24).
19. **Multi-repository.** One state directory, in the hub. A member repository has no state of its own; the
    governing project is found by walking up from the path being touched (SEAM-07).
20. **Ancestor searches** are allowed for a row whose denied classes all lie under `.factory/`, once a probe
    shows the search tool skips the excluded state directory (SEAM-42).
21. **Mapping.** Rows 16 and 20 of section 3.1 are re-marked (COV-15).
22. **Reserved event types.** `vsdd events emit` cannot write `rule.*`, `runner.*`, `decision.*`, `step.*`,
    `spike.*` or `pr.merged` (OPS-15).
23. **Open questions.** Section 6 is superseded by section 2 of the blueprint (decisions D4, D6, D7, D11, D12,
    D24), whose recommendations the user adopted on 2026-11-23.

Second pass (2026-11-24):

24. **The guard stays inert.** When the launcher cannot be read, the `hooks.json` command walks up from the
    project directory for `.factory/factory.toml` and blocks only if it finds one; in a project that is not
    initialised it exits 0. In brief mode it prints the down-briefing. `onFailure` is no longer described as
    the defence for missing files (sections 2.3, 2.8, 2.9, 2.16; P2COV-07).
25. **Caller markers.** Before a `vsdd` call is allowed the runner writes a marker for that one call
    (`local/hooks/calls/<key>.json`: session, agent id, agent type, working directory) and rewrites the command
    through `updatedInput` to carry the key. This replaces the per-session marker of item 8. The survey, the
    lock, `spike open`, `env start` and `doctor` read it. Probe: blueprint section 10, items 27 to 29
    (section 2.11.10; P2SEAM-15, P2OPS-01, P2COLD-05, P2COV-08).
26. **Three policy tables.** `[callers]` and `[reveals]` are in `walls.toml`; `[reveals]` is generated from the
    class of each artifact kind; an unknown list kind reveals every walled class. The person-only commands,
    the governed commands and the exempt story kinds are constants in code (the last derived from the
    story-kind table). There is no `gates.toml` (section 2.7; P2LEAN-13, P2SEAM-12).
27. **Walls corrected.** Every row, the main session's included, denies `runs`. The four planners have
    `tool_only = true` with the `read` and `measure` groups (and `route` for the architect). The spike runner
    denies `outside`. The main row grants `read`, `author` and `operator`, not `pipeline` or `env`. `status
    --fix` and `doctor` are out of the `read` group (section 2.7; P2SEAM-11, P2LEAN-03, P2OPS-12, P2OPS-13,
    P2COLD-21).
28. **Product path classes.** Seven classes with one precedence (`tests`, `harness`, `ci`, `repo-config`,
    `manifests`, `docs`, `source`), used by `write-scopes`, `tests-first` and the red gate. Cases are added
    for a scaffold create, a manifest edit, and `src/<m>/tests.rs` (sections 2.7, 2.11.2, 2.11.4; P2SEAM-01,
    P2COLD-02, P2OPS-03).
29. **`artifacts`.** No `code_values` check. The specification lock is one rule over the whole set
    (`spec_locked(project)`), plus `artifact_locked(project, path)` for accepted decision records, proven
    properties and done stories (section 2.11.3; P2LEAN-01, P2SEAM-19, P2COLD-20).
30. **`decisions`.** Named choice forms, repeated keys, qualified identifiers. The rule's settle call writes
    the decision record only, so it stays inside the deadline. If the driver was paused at that decision in
    this session, the returned context tells the main session to resume it (section 2.11.5; P2SEAM-13,
    P2OPS-05, P2OPS-06, P2COLD-29).
31. **`step-results`.** The rule stores the calling agent's type and the worktree head with every record and
    no longer refuses by step name; the tool checks the type when it counts a pass. There is no generated
    step-name table. Walled findings go to the holdout class. Findings of category `holdout` are not grounded.
    The platform probe passed, so there is no `step record` fallback (section 2.11.7; P2LEAN-10, P2COLD-27,
    P2SEAM-02, P2SEAM-03, P2SEAM-07).
32. **`spike-bounds`.** Every check matches the runner by agent id. The asker's type comes from the asking
    step's stored record or from an admission the rule records at `PreToolUse` on `Agent`. A workflow step is
    held by blocking its `StructuredOutput` call; a dispatched runner by refusing its stop. `spike record
    --increment` satisfies the check for a named spike (section 2.11.6; P2SEAM-04, P2COV-01, P2COV-08,
    P2REQ-10).
33. **`auto-allow`.** The one mechanism for decision D4. `[hooks.rules.auto-allow] scope` is `off`, `core` or
    `extended`, written by `vsdd init --auto-allow`. It is not "off by default pending a question" (sections
    2.6, 2.10, 2.11.10; P2COV-02, P2OPS-02, P2COLD-09, P2REQ-08).
34. **The main-session wall switch.** `[hooks] main_session_wall = false` relaxes the `main` row only; every
    agent row stays locked. When the main session dispatches an agent that may read `holdout`, the rule checks
    that agent's final message for scenario text (sections 2.6, 2.11.2; P2COLD-11, P2REQ-14).
35. **`secrets`.** Upload flags are allowed to a loopback port that `vsdd env start` recorded for the run.
    Web addresses in `curl`, `wget` and `http` commands are logged and checked like web tool calls. `export
    -p` and `declare -x` are blocked (section 2.11.1; P2COLD-15, P2OPS-13).
36. **No `PostToolUse` wiring.** Seven events are wired. The tool writes `pr.created` and `pr.merged`, and the
    gates write `commit.made` (sections 2.3, 2.11.10, 2.12; P2LEAN-15).
37. **`compaction`.** The rule is renamed from `compaction-snapshot`. It flushes and instructs the summariser
    and writes no snapshot file; the briefing adds no snapshot lines (sections 2.11.8, 2.11.9; P2LEAN-12).
38. **No canary, no `events emit`, no reserved-type list.** `vsdd doctor` proves the hooks are live through its
    own caller marker (sections 2.9, 2.12, 2.13; P2LEAN-07, P2LEAN-16).
39. **Fingerprint.** It also covers `[[component]]`, `[stops]`, `[release]`, `[spikes]` and `[caps]`, and the
    briefing announces a change to any gate-bearing section (sections 2.6, 2.11.8; P2OPS-11).
40. **`lifecycle-log`.** It refreshes the lock for calls from the holder's session or from a worktree the run
    owns, and on `SubagentStop` stops a product the agent started (section 2.11.10; P2OPS-01, P2OPS-10).
41. **Browser.** The bundled server's entry carries the option that refuses `file:` addresses; `browser_tabs`
    is matched (section 2.11.2; P2OPS-14).
42. **Messages.** The "uv cannot run" row names commands that need nothing from the plugin; `doctor` has no
    `--fix` (sections 2.2, 2.9; P2SEAM-16, P2OPS-16).
43. **Interfaces.** Section 5, S7 and P5, and the conditions table of 2.8 are aligned with `state.md`
    (P2SEAM-20, P2COV-08, P2COLD-32).

Author: hooks designer, 2026-11-23. Scope: the hook runner, its rules, its event log and its tests.

Three sibling designs, `agents-skills.md`, `packaging.md` and `pipeline.md`, were on disk before I finished,
so this document uses their names where they overlap with mine (agent names, path classes, the `py/` layout,
the evidence, decision and step-record stores) and section 4.2 shows where each need they placed on hooks is
met. `state.md` was not yet written; names that belong to it are working names here and are listed as needs
in section 5. The plugin's command-line tool is called `vsdd` in `packaging.md` and `factory` in
`pipeline.md`. I write `vsdd`; the rules recognise the tool by its resolved path, not by its name.

## 1. Summary

In 1.0 the hook subsystem is one small Python program that uses only the standard library. A 70-line POSIX
shell launcher starts it through `uv`, and one checked-in `hooks/hooks.json` wires it to seven events. It
replaces the Rust dispatcher, 37 WebAssembly plugins and 36 bash hooks (79 registry entries) with 17 rules.

A rule is a plain Python function. It receives the hook payload plus facts about the project and returns
allow, block, advise, add context, or permit. Gates run before the action, so an invalid write never lands.

In a project with no factory configuration file the launcher exits before Python starts (about 8 ms). Every
failure of the launcher or of a gate rule blocks and says why. A gate can be relaxed only by a person, and
every relaxation is announced at each session start.

The rules cover general safety, information walls keyed on the calling agent, artifact validity, tests
before code, the governed merge, decisions only a person can record, spike bounds and containment, storing
every workflow step result, the session briefing and the pre-compaction flush. One JSON-lines event log
feeds the observability stack and the `vsdd explain` command.

## 2. The design

### 2.1 Terms

| Term | Meaning |
|---|---|
| Runner | The Python code under `<plugin>/py/vsdd/` that evaluates rules for one hook event and exits. |
| Launcher | `<plugin>/py/launch.sh`, the shell script Claude Code executes. It decides whether to start the runner and turns every launch failure into a defined result. |
| Rule | One Python function with a declared identity, events, tools and failure behaviour. |
| Gate | A rule that can block. An advisor can only add text. A recorder only writes records. |
| Outcome | What a rule returns: nothing (allow), `Block`, `Advise`, `Context` or `Permit`. |
| Row | The line of the wall table selected by the calling agent's type (`agent_type` in the payload). |
| Path class | A named set of paths, as defined in `agents-skills.md` section 2.9 (for example `holdout`, `reviews`, `source`). |
| Factory project | A project whose root holds the project configuration file (working name `.factory/factory.toml`). |
| Evidence record | A small JSON file written by code, not by a model, that proves something happened (a failing test run, a passing test run). |

### 2.2 The launch chain

```
Claude Code ── hooks.json ──> sh py/launch.sh hook <mode> ──> uv run --script py/hook.py <mode> ──> vsdd.hooks.main
                                 │                                 │
                                 │ not a factory project           │ exit 0  = allow (stdout may carry one JSON object)
                                 └─ exit 0, nothing else           │ exit 10 = a rule blocked (reason already on stderr)
                                                                   └ anything else = the launch failed
```

`packaging.md` proposes one launcher for the hook entry and the `vsdd` command (its interface I3). I accept
the paths and specify the behaviour of the `hook` entry here, because it differs from I3 in one important
way: it must not `exec` uv. The `cli` entry shares the uv discovery and the environment hygiene and may
`exec`.

The `hook` entry takes one more argument, the mode, which says what a launch failure must turn into.

| Mode | Used by | If the runner cannot run |
|---|---|---|
| `gate` | `PreToolUse`, `UserPromptExpansion` | Exit 2 with a one-line reason and the fix. The action is blocked. |
| `brief` | `SessionStart` | Exit 0 and print a fixed JSON briefing that says the gates are down and why, to the person and to the model. |
| `notify` | `SubagentStart`, `SubagentStop`, `PreCompact`, `SessionEnd` | Exit 0. Nothing is being guarded on these events, and the session start has already announced the fault. |

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

[ "${VSDD_FACTORY_HOOKS:-on}" = off ] && { [ "$mode" = brief ] && print_disabled_briefing; exit 0; }

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
  *)  fail "launch_rc_$rc" "the hook runner could not run (exit $rc)." ;;
esac
```

Why each part is there. The facts marked "measured" come from experiments I ran on the user's Mac
(uv 0.12.23) while writing this design; the rest are from `rewrite/STATE.md`.

| Choice | Reason |
|---|---|
| The launcher does not `exec` uv for hooks. | A Python crash or syntax error leaves through `uv run` as exit 1 (measured), which Claude Code treats as non-blocking. The launcher must see the exit code to turn it into a block. |
| The runner blocks with exit 10, not 2. | uv itself exits 2 when it finds no interpreter or cannot open its cache (measured). With a private code the launcher can tell "a rule blocked" from "uv failed" and print the right message. Claude Code still receives 2 in both cases. |
| `--python '>=3.11'` on the command line as well as in the script header. | `UV_PYTHON=3.9` in the environment overrides the inline `requires-python` and ran the script on Python 3.9 (measured). The explicit flag wins over the variable (measured). The launcher also unsets the variable, and `hook.py` exits 13 if the interpreter is older than 3.11. |
| `--no-config --offline --no-python-downloads`. | The hook must behave the same in every project and must never touch the network. A stdlib-only script has nothing to download, and an interpreter download inside a hook call would be slow and would fail in a sandbox. If no Python 3.11 or newer is installed, uv exits 2 and the launcher prints the fix (`uv python install 3.12`). The three flags cost nothing (measured: 35 to 37 ms with or without them). |
| `PYTHONSAFEPATH=1`, `PYTHONPATH` unset. | The project being worked on must not be able to shadow a standard-library module. `hook.py` adds its own directory to `sys.path` explicitly. |
| `PYTHONPYCACHEPREFIX` in the user's cache directory. | Bytecode is cached without writing into the installed plugin (a rule in `packaging.md`). Measured: 57 ms warm with the cache, 70 ms without. |
| uv is looked for on `PATH` and then in four usual places. | Taken from `packaging.md`: a session started from a desktop launcher can have a short `PATH`. |
| `here="${0%/*}"` instead of `CLAUDE_PLUGIN_ROOT`. | The launcher works even if that variable is missing, and a parameter expansion costs no process. |
| `VSDD_FACTORY_HOOKS=off`. | The one emergency switch. Only a person can set it, because a tool call cannot change the environment of the Claude Code process. The session briefing announces it. See question 3 in section 6. |

`find-root.sh` is sourced by the launcher for both entries, so there is one implementation of project
discovery (section 2.8).

Messages to a person. A person's own terminal does not have the plugin's `bin/` on its `PATH`, and when
`uv` is missing `vsdd doctor` cannot run at all. Every fix line therefore names something that works where
the person is: a terminal command that needs nothing from the plugin (install `uv`; `uv python install
3.12`), a slash command, or `vsdd ...` once `setup` has installed the shim at `~/.local/bin/vsdd`. A message
gives the in-session form first and the terminal form second. Both entries use one download policy: neither
the hook path nor the `cli` path ever downloads a Python, and nothing in the plugin installs one. The first
pass gave that job to `vsdd doctor --fix`, which starts through this same launcher and so cannot start on
the very machine that lacks a Python; the fix is the terminal command the message names. The fallback
briefing also sets `systemMessage`, so the person sees that the gates are down even if the model says
nothing.

Interpreter floor: Python 3.11, the first release with `tomllib` in the standard library. The runner has no
third-party imports. `hook.py` carries the inline metadata `requires-python = ">=3.11"` and
`dependencies = []`, as the spike proved. The test suite runs on 3.11 and on the newest release.

### 2.3 Event wiring and time budgets

`hooks/hooks.json` is generated from the rule table (`vsdd/hooks/registry.py`) by `tools/build.py` and
checked in. `tools/build.py --check` fails if the checked-in file differs from the generated one. There are no per-platform variants
and no activation step. The runner starts only for the events and tools below. Every other tool (MCP tools
other than those the matcher names) never reaches it. Seven events are wired; the first pass also wired
`PostToolUse`, and the second pass removed it (below).

| Event | Matcher | Mode | `timeout` (seconds) | Other fields | Rules that run |
|---|---|---|---|---|---|
| `SessionStart` | none (all sources: startup, resume, clear, compact, fork) | `brief` | 15 | | `session-briefing` |
| `UserPromptExpansion` | `^vsdd-factory:decide$` | `gate` | 10 | `onFailure: "block"` | `decisions` |
| `PreToolUse` | the exact-name list below | `gate` | 10 | `onFailure: "block"` | all gates, the advisor, `auto-allow` |
| `SubagentStart` | `^vsdd-factory:\|^workflow-subagent$` | `notify` | 5 | `async: true` | `lifecycle-log`; `spike-bounds` (pairs a starting spike runner with its admission) |
| `SubagentStop` | same | `notify` | 5 | | `lifecycle-log`, `spike-bounds`, `read-walls` (the final-message check of 2.11.2) |
| `PreCompact` | none (manual and auto) | `notify` | 30 | | `compaction` |
| `SessionEnd` | none | `notify` | platform budget of 1.5 | | `lifecycle-log` (log, prune, and release a run lock the session holds) |

The `PreToolUse` matcher is one name list, and the runner routes by `tool_name`. Besides the built-in tools
it names the tools that can read a local file or carry data out: the Playwright navigation, tab and upload
tools in both spellings (the plugin's server and the user's own), `WebFetch`, `WebSearch`, and the research
tools. It also names `Workflow`, for one purpose: `auto-allow` may approve a start of one of this plugin's
workflows by the main session, so the driver is not stopped by a prompt. No gate rule applies to `Workflow`,
so hooks still never decide when work starts.

```json
{ "matcher": "Bash|Edit|Write|MultiEdit|NotebookEdit|Read|Grep|Glob|Agent|Task|StructuredOutput|Workflow|WebFetch|WebSearch|mcp__(plugin_vsdd-factory_)?playwright__browser_(navigate|navigate_back|tabs|file_upload)|mcp__(plugin_vsdd-factory_)?(perplexity|tavily|context7)__.*",
  "hooks": [ { "type": "command",
               "command": "sh -c 'f=\"${CLAUDE_PLUGIN_ROOT}/py/launch.sh\"; [ -r \"$f\" ] && exec sh \"$f\" hook gate; d=\"${CLAUDE_PROJECT_DIR:-$PWD}\"; while [ -n \"$d\" ] && [ \"$d\" != \"$HOME\" ] && [ \"$d\" != / ]; do [ -f \"$d/.factory/factory.toml\" ] && { echo \"vsdd-factory: plugin files are missing, so the factory gates cannot run. Reinstall the plugin.\" >&2; exit 2; }; d=\"${d%/*}\"; done; exit 0'",
               "timeout": 10, "onFailure": "block" } ] }
```

The command guards itself, and it stays inert where it should. If the launcher is missing (a broken install,
or a cache directory that was cleaned up under a running session), `sh` would exit 127, which Claude Code
treats as non-blocking, and every gate would switch off silently. The guard prevents that, but only in a
factory project. `find-root.sh`, which decides whether a project is a factory project, lives beside the
launcher and is unreadable in exactly this case, so the guard carries the same test inline: it walks up from
`CLAUDE_PROJECT_DIR` (or the working directory) to the home directory or the root, looking for
`.factory/factory.toml`. If it finds one it exits 2 with a reason and the reinstall command. If it finds
none it exits 0, so a broken install never blocks a tool call in a project that was not initialised
(decision D24). In `notify` mode the same guard exits 0. In `brief` mode, in a factory project, it prints a
fixed down-briefing as JSON with `systemMessage`, saying the plugin files are missing and the gates are
blocking, so the person sees why every guarded call fails. The guard does not test linked worktrees or
pre-1.0 state; it is the smallest test that keeps unrelated projects untouched, and a session started inside
a linked worktree of a factory project falls back to exit 0, which the next start from the project root
corrects.

Notes on the wiring.

- Timeouts are in seconds. The old file said 10000, which Claude Code read as 2.8 hours. A test asserts
  that no timeout in the file exceeds 60.
- `onFailure: "block"` (Claude Code 2.1.295 or later) makes Claude Code itself treat a hook that cannot
  start, times out, or exits with an unexpected code as a block. It is the second line of defence behind
  the launcher, for a hook that hangs or dies. It is not the defence for missing plugin files: the
  self-guarding command above is, and it is the one that knows whether the project is a factory project.
- `StructuredOutput` is a guarded tool, because that call is how a workflow step returns its result and
  `step-results` stores it (`pipeline.md` need H1). A call that carries no step identity passes untouched.
  It is also where a spike runner that is a workflow step is held until it has recorded its finding.
  `Workflow` reaches the runner for `auto-allow` only, as said above.
- `SubagentStop` is synchronous because `spike-bounds` and `read-walls` can refuse a stop. Its mode is
  `notify`, so a launch failure never traps an agent that is trying to finish. The platform probe showed
  that a refused stop holds an agent that returns text and does not hold a workflow step that has already
  returned a schema result; the rules that need to hold a step act on its `StructuredOutput` call.
- `PostToolUse` is not wired. The first pass had two handlers there, filtered to `git commit` and `gh pr`,
  to record commits and pull requests. In 1.0 no pipeline agent opens a pull request (the tool does), the
  filter was documented as best effort, and each commit paid about 55 ms for a recorder. The tool now writes
  `pr.created` and `pr.merged`, and the gates write `commit.made` for the commits they measure.
- `PostToolUseFailure`, `Stop`, `PostCompact`, `PermissionRequest`, `WorktreeCreate` and `WorktreeRemove`
  are deliberately not wired. Section 3 gives the reason for each old hook that used them. In short:
  validation moved before the write, the briefing after compaction arrives through `SessionStart` with
  source `compact` (proven by the spike), and a `WorktreeCreate` command hook replaces Claude Code's own
  worktree creation, so a logging hook there would break worktree isolation.
- `async: true` is used only for recorders. The generator refuses to put `async` on a handler that has a
  gate rule, because Claude Code ignores decisions from background hooks.
- The shell form of the command is used because the spike proved it. The path is quoted.

Time budget per call.

| Path | Claude Code timeout | Runner's own deadline | Expected time |
|---|---|---|---|
| Not a factory project | n/a | n/a | about 8 ms (measured), no uv, no Python |
| `gate` | 10 s | 4 s, then a block that names the rule that was running | 55 to 70 ms warm (measured 57 ms on a prototype with uv start, JSON, TOML and four module imports); about 190 ms on the first call after an install |
| `brief` | 15 s | 8 s, then the fixed fallback briefing | 60 to 90 ms |
| `notify` on `PreCompact` | 30 s | 25 s, of which the state flush may use 20 s | 80 ms plus the flush |
| `notify` elsewhere | 5 s (1.5 s on `SessionEnd`) | 1 s | about 55 ms |

Rules have a budget too: a rule may start at most one `git` subprocess, with a 1.5 s timeout, and no rule
makes a network call. Git facts that can be read from files (`HEAD`, worktree pointers) are read from files.
A performance test fails if the median `gate` call on the test machine exceeds 120 ms, which is stricter
than the 400 ms ceiling in `packaging.md`.

### 2.4 Python layout inside the plugin

The layout follows `packaging.md` section 2.3: one standard-library package, `vsdd`, shared by the hook
entry and the `vsdd` command.

```
<plugin>/hooks/hooks.json     generated from the rule table, checked in
<plugin>/py/
  launch.sh                   the launcher (section 2.2)
  find-root.sh                project discovery, sourced by launch.sh
  hook.py                     hook entry: inline metadata, version check, sys.path, call vsdd.hooks.main
  cli.py                      entry for the vsdd command (packaging design)
  vsdd/
    project.py                shared core: project root, state directory, worktrees, story and spike detection
    config.py                 shared core: loads and validates the project configuration, including [hooks]
    frontmatter.py            shared core: the small frontmatter reader (a strict YAML subset)
    events.py                 shared core: the one event writer and reader (section 2.12)
    hooks/
      main.py                 read stdin, build the context, select rules, run them, emit, log
      model.py                Ctx, Rule, the outcome types, the @rule decorator
      registry.py             the rule table: (event, tool) -> rule modules; generates hooks.json
      emit.py                 the only module that writes a hook's stdout, stderr and exit code
      policy.py               loads the three policy tables
      paths.py                path normalisation, symlink resolution, path-class matching
      shell.py                splits a Bash command into segments, tokens, redirect targets, command families
      edits.py                computes the content a file would have after a Write, Edit or MultiEdit
      gitfs.py                git facts from files; one subprocess helper with a timeout
      records.py              reads the evidence store and the spike ledger; calls the state design's
                              functions that store a step record and settle a decision
    rules/                    one module per rule (17 modules)
    policy/                   walls.toml  classes.toml  artifacts.toml
    state/  cli/              (state and packaging designs)
```

Registration is an explicit table in `registry.py`, not directory scanning. `main.py` imports only the rule
modules listed for the event and tool at hand, so a `Read` call does not load the Bash parser.

The frontmatter reader accepts one `---` fenced block at the top of a file containing scalars (string,
integer, boolean, null), flow lists (`[a, b]`), block lists of scalars, and one level of nested mapping.
Anything else raises an error with a line number, which the `artifacts` rule reports as a block. It is
about 120 lines. The differential test against PyYAML in `packaging.md` covers it.

### 2.5 The rule model

A rule is declared once, next to its code:

```python
@rule(id="tests-first",
      on=[("PreToolUse", ("Edit", "Write", "MultiEdit", "NotebookEdit"))],
      kind="gate",            # gate | advisor | recorder
      fail="closed",          # what a crash or the deadline turns into: closed = block, open = log and continue
      locked=False,           # locked rules cannot be relaxed in project configuration
      spike_exempt=True,      # skipped when the work is inside a spike
      summary="Product source in a story is written only after a failing test run is recorded.")
def check(ctx: Ctx, cfg: RuleConfig) -> Outcome | list[Outcome] | None: ...
```

What a rule receives. `Ctx` is built once per call; the derived fields are computed on first use.

| Field | Content |
|---|---|
| `event`, `tool`, `tool_input`, `raw` | The payload as Claude Code sent it. |
| `session_id`, `agent_type`, `agent_id`, `cwd` | Who is calling and from where. `agent_type` is empty for the main session, `vsdd-factory:<name>` for a plugin agent, `workflow-subagent` for a workflow step with no agent type (all verified by the spike). |
| `row` | The wall-table row for `agent_type`: the agent's own row, `main` for the main session, `other` for any type that is not listed. |
| `project` | The governing project: root, state directory, parsed configuration, policy tables. It is found by walking up from the path being touched to the nearest state directory, and falls back to the session's project. Story worktrees and the `repo` targets of a multi-repository project therefore resolve correctly (`pipeline.md` need H7). |
| `target` | For file tools: the absolute path the call reads or writes, normalised, with symlinks resolved. Class matching tests both the literal and the resolved path. |
| `classes` | The path classes that contain `target`. |
| `command` | For `Bash`: the parsed command (segments, tokens, redirect targets, `cd` and `git -C` prefixes, command families). |
| `new_content` | For `Write`, `Edit` and `MultiEdit`: the text the file would contain after the call. This is what lets validation run before the write. |
| `story` | The story whose branch is checked out in the worktree that contains `target` or `cwd`, if any. |
| `in_spike` | True when the caller's row is the spike row, or the enclosing worktree is on a `spike/*` branch, or its directory name starts with `spike-`. |
| `log(type, **fields)` | Adds an event to the event log. |

What a rule can return.

| Outcome | Meaning | How `emit.py` delivers it |
|---|---|---|
| `None` | Allow; the rule has no opinion. | Nothing. |
| `Block(code, reason, fix)` | Stop the action. `reason` says what was attempted and why it is not allowed; `fix` names the correct route. | Exit 10 from the runner, which the launcher turns into exit 2, with the message on stderr. Nothing on stdout. This is the path the spike proved, and it is the same for every blocking event. |
| `Advise(code, message)` | Allow, and tell the model something. | Exit 0 with `hookSpecificOutput.additionalContext`. |
| `Context(text)` | Add context (briefing, compaction instructions). | `additionalContext` on `SessionStart`; plain stdout on `PreCompact`, where Claude Code uses it as instructions for the summary. |
| `Permit(reason)` | `PreToolUse` only: skip the permission prompt for this call. | `permissionDecision: "allow"`. Honoured only if no rule returned `Block` or `Advise`. Only `auto-allow` returns it. |

All rules selected for a call run, even after one blocks, so the model sees every reason at once. The
message format is fixed:

```
vsdd-factory blocked this Edit call. [tests-first/no_red_evidence b-7f3a2c]
Why: src/lib.rs is product source in story S-1.02, and no failing test run is recorded for that story.
Instead: write the failing test, then run the red gate: `vsdd gate red --story S-1.02`.
More: vsdd explain b-7f3a2c
```

Rules do not write files, with these named exceptions: every rule may log; `step-results` stores the step
record and `decisions` settles a decision, both through the state design's own functions; `spike-bounds`
keeps start times and admissions in the runner's own directory; `compaction` calls the state flush; and the
runner itself writes a caller marker before it lets a `vsdd` call through (section 2.11.10). `Permit` and
`Context` aside, one more thing can leave a rule: the runner may return `updatedInput` for a `vsdd` Bash
call, to add the marker's key to the command, and for nothing else.

Illustrative skeleton of the main loop:

```python
def main(mode: str) -> int:
    payload = read_stdin_json()                     # unreadable input on a gate becomes a Block
    ctx = Ctx.from_payload(payload, Project(os.environ["VSDD_FACTORY_ROOT"]))
    outcomes = []
    with deadline(BUDGET[mode]) as d:
        for r in registry.select(ctx.event, ctx.tool):          # imports only these modules
            if not r.applies(ctx):                              # strength off, or spike exemption
                continue
            d.current = r.id
            try:
                outcomes += as_list(r.fn(ctx, ctx.project.rule_config(r.id)))
            except Exception as exc:                            # includes the deadline
                outcomes.append(r.failure_outcome(exc))         # closed -> Block, open -> logged only
    return emit.finish(ctx, outcomes)                           # logs, writes stdout/stderr, returns 0 or 10
```

### 2.6 How a project tunes a rule or turns it off

Settings live in the `[hooks]` table of the project configuration file.

```toml
[hooks]
log_retention_days = 30
main_session_wall = true            # decision D6; false relaxes the main session's row only

[hooks.rules.auto-allow]
scope = "core"                      # decision D4: off | core | extended; written by vsdd init --auto-allow

[hooks.rules.tests-first]
strength = "advise"                 # block (default) | advise | off

[hooks.rules.bash-git]
attribution = "block"               # commit messages and pull request text

[hooks.rules.bash-destructive]
allow_pr_close = true
```

- `strength = "advise"` turns every block from that rule into advice that begins "would have blocked", and
  logs `rule.would_block`. It is the way to trial a rule. `off` skips the rule.
- Locked rules (`read-walls`, `write-scopes`, `gate-guard`, `decisions`, `step-results`,
  `session-briefing`) ignore `strength`; setting it is reported as a configuration error.
- One exception to the lock is stated, because the user adopted both decision D12 (the walls are locked) and
  decision D6 (the main session is walled, "with a switch in project configuration"). `main_session_wall =
  false` relaxes the `main` row of the wall table and nothing else: the main session may then read `holdout`
  and write the specification classes. Every agent row stays locked whatever the key says. While it is
  `false`, every briefing says so.
- `auto-allow` has a scope, not a strength: `off`, `core` or `extended` (section 2.11.10). `vsdd init
  --auto-allow <scope>` writes it from the answer `setup` collected before it ran `init`, because that is the
  one moment the plugin writes `[hooks]`. On a project that is already initialised, `setup` shows the person
  the two lines to add. This is how the person's yes to decision D4 is stored; the first pass had no key for
  it.
- Only a person can change this table. The project configuration file is in the `config` path class, which
  no tool call may write (`state.md` section 2.4), and the `vsdd` command offers no subcommand an agent can
  run that changes it: `vsdd config set` is on the person-only list (need K7). A person edits the file. An
  agent that is blocked by a gate therefore cannot switch the gate off, and cannot weaken a gate indirectly
  by changing the test command, the source globs or a threshold.
- Nothing is relaxed silently. Every rule that is not at full strength is listed in every session briefing.
  The runner keeps a fingerprint of every section that bears on a gate (`[hooks]`, `[commands]`, `[paths]`,
  `[branches]`, `[delivery]`, `[convergence]`, `[holdout]`, `[mutation]`, `[limits]`, `[[component]]`,
  `[stops]`, `[release]`, `[spikes]`, `[caps]`), and a change since the previous session is announced once,
  to the model and through `systemMessage` to the person. `[[component]]` matters most: its commands
  override `[commands]`, the file travels with the pushed state branch, and a `test = "true"` arriving from
  another machine would otherwise make the green gate pass in silence. A write by code (the toolchain an
  approval applied, a `repos split`) is announced the same way.
- One rule can be quarantined. `VSDD_FACTORY_HOOKS_QUARANTINE=<rule id>`, an environment variable only a
  person can set, skips that rule, locked or not, but only when the event log shows repeated `rule.error`
  events for it. It exists so that a crash in one rule (for example `step-results`, which would stop every
  workflow step) does not force a person to switch every gate off. Every briefing announces it.
- An unknown rule id, an unknown key or a bad value under `[hooks]` does not disable anything: the runner
  uses the built-in defaults for that rule, logs `config.invalid`, and says so in the briefing.

Project-specific paths and limits that rules read (the integration and release branches, source and test
globs, the spike time box) are ordinary project configuration owned by the state design (need S3), not
`[hooks]` settings.

### 2.7 Policy tables

Three small TOML files under `<plugin>/py/vsdd/policy/` hold the facts that other designers own:
`classes.toml`, `walls.toml` and `artifacts.toml`. I own the format and the loader. A lint test checks them
against the plugin: every agent file has a row and every row has an agent file (`agents-skills.md` P-8),
every path class has globs, every subcommand named under `[callers]` exists in the tool. The first pass had
a fourth file, `gates.toml`. Its two tables that are data, `[callers]` and `[reveals]`, moved into
`walls.toml`, where the rows that use them are; its three lists that never change without the code changing
are constants (end of this section).

`classes.toml` maps each path class to real paths and adds two facts the runner needs. Its content is the
class table of `state.md` section 2.4 and nothing else; the examples below use that table's names.

```toml
[holdout]
globs = [".factory/holdout/**"]          # relative to the project root
route = "Holdout scenarios are written by the product owner: /vsdd-factory:author holdout"

[config]
globs = [".factory/factory.toml"]
write = "none"                           # tools (default) | code | none
                                         # tools: Edit and Write by agents whose row grants the class, never the shell
                                         # code:  only the vsdd command and the runner
                                         # none:  no tool call (a person edits it; reference code)
route = "Project configuration is edited by a person. Show them the line to change."

[control]
globs = [".factory/notes.json", ".factory/cycles/**"]
write = "code"

[evidence]
globs = [".factory/local/evidence/**"]
write = "code"

[runs]
globs = [".factory/local/runs/**"]
write = "code"                           # and every agent row denies reading it

[reference]
globs = [".reference/**"]
write = "none"

[source]
from_config = "paths.source"             # project-specific globs, matched relative to the worktree root

[manifests]
from_config = "paths.manifests"          # build manifests and lock files

[repo-config]
from_config = "paths.repo_config"        # toolchain and repository configuration files

[harness]
from_config = "paths.harness"            # proofs, fuzz targets, benchmarks

[_product]
precedence = ["tests", "harness", "ci", "repo-config", "manifests", "docs", "source"]
                                         # a product path belongs to the first class that matches, so
                                         # src/parser/tests.rs is in tests although src/** is a source glob
```

The evidence store, the decision store, the step-record store, the spike ledger and the runner's own
directory are all `write = "code"`: the tool and the runner write them, and no agent tool call can
(`pipeline.md` guard G4).

Classes inside the state directory are matched relative to the project root. The seven project-code classes
(`tests`, `harness`, `ci`, `repo-config`, `manifests`, `docs`, `source`) are matched relative to the root of
the git worktree that contains the path, so they work the same in the main checkout and in every story
worktree. A product path has exactly one class: the first in the `precedence` list whose globs match.
`paths.py` resolves it once, and `write-scopes`, `tests-first` and the tool's red gate all use that answer.
A product path that matches no class is `unclassified`: no agent may write it, and the block message gives
the `[paths]` line to add. `route` is the sentence a block message uses to name the correct way to change
something in that class.

Three classes are computed, not listed: `own-worktree` is the git worktree that contains the caller's `cwd`;
`project` is every path outside the state directory; `tmp` is the system temporary directory and is always
writable.

`walls.toml` has one table per agent type and encodes the four tables of `agents-skills.md` section 2.9:
read walls, write scopes, dispatch, and command scopes. Two extra rows cover callers that are not factory
agents: `main` for the main session and `other` for any type that is not listed, which gets the strictest
treatment.

```toml
[main]
read_deny = ["holdout", "runs"]                # runs is denied to every caller; findings are read with vsdd review show
write     = ["brief", "deferred", "project"]
dispatch  = ["*"]
commands  = ["*"]                              # merge-guard still blocks commits and pushes on the integration and release branches
vsdd      = ["read", "author", "operator"]     # not pipeline or env: survey, checkpoint, deliver, release and env start are the clerk's
relaxed_by = "hooks.main_session_wall"         # false drops holdout from read_deny and adds the spec classes to write (D6)

[other]
read_deny = ["holdout", "reviews", "runs"]
write     = ["project"]

["vsdd-factory:implementer"]
read_deny  = ["holdout", "runs"]
write      = ["source", "manifests"]           # never tests
story_only = true                              # project code only inside a story worktree
dispatch   = ["vsdd-factory:spike-runner"]
commands   = ["git-commit"]
vsdd       = ["read", "measure"]               # it finishes with vsdd gate green

["vsdd-factory:test-writer"]
read_deny  = ["holdout", "runs"]
write      = ["tests:frozen", "manifests", "source:before-red"]   # may change frozen tests; stubs only until red exists
story_only = true
dispatch   = ["vsdd-factory:spike-runner"]
commands   = ["git-commit"]
vsdd       = ["read", "measure", "refreeze"]   # vsdd gate red, and gate red --refreeze, which only this row may run

["vsdd-factory:verifier"]
read_deny  = ["holdout", "reviews", "runs"]
write      = ["harness", "tests"]              # not the tests frozen at a story's red gate
story_only = true
dispatch   = ["vsdd-factory:spike-runner"]
vsdd       = ["read", "measure"]

["vsdd-factory:devops"]
read_deny  = ["holdout", "runs"]
write      = ["ci", "repo-config", "manifests", "source:create@repo-setup", "tests:create@repo-setup"]
                                               # @repo-setup: only in a chore story whose method is repo-setup (the scaffold)
story_only = true
dispatch   = ["vsdd-factory:spike-runner"]
vsdd       = ["read", "measure"]

["vsdd-factory:architect"]
tool_only  = true                              # a vsdd-only shell
read_deny  = ["holdout", "runs", "outside"]
write      = ["spec-arch"]
dispatch   = ["vsdd-factory:spike-runner"]
vsdd       = ["read", "measure", "route"]      # gate spec --only <part>; feature route

["vsdd-factory:story-writer"]
tool_only  = true
read_deny  = ["holdout", "runs", "outside"]
write      = ["stories"]
dispatch   = ["vsdd-factory:spike-runner"]
vsdd       = ["read", "measure"]               # gate plan --story, which admits the story

["vsdd-factory:adversary"]
read_deny = ["holdout", "reviews", "runs", "outside"]
write     = ["reviews:create"]                 # may create its report in an on-demand review; may not change an existing file
dispatch  = ["vsdd-factory:spike-runner"]

["vsdd-factory:holdout-evaluator"]
read_deny  = ["spec-product", "spec-arch", "source", "tests", "reviews", "runs", "outside"]   # shortened here
read_allow = ["brief"]                         # a class named here wins over a containing class that is denied
write      = []                                # its evaluation file is written by code
commands   = []                                # no git, no gh
dispatch   = ["vsdd-factory:spike-runner"]     # the runner then inherits this row's read_deny
vsdd       = ["env"]                           # the only vsdd subcommands it may run

["vsdd-factory:scenario-reviewer"]
read_deny = ["source", "tests", "ci", "reviews", "stories", "analysis", "spec-arch", "spec-ux", "runs", "outside"]
write     = []                                 # it reads holdout and spec-product and returns findings
dispatch  = ["vsdd-factory:spike-runner"]

["vsdd-factory:clerk"]
tool_only = true                               # every Bash command must be a call of the vsdd tool
read_deny = ["holdout", "reviews", "runs", "outside"]
write     = []
dispatch  = []
vsdd      = ["read", "pipeline", "measure", "route", "env"]

["vsdd-factory:spike-runner"]
spike     = true                               # the spike row: exemption and containment apply
read_deny = ["holdout", "reviews", "runs", "outside"]   # plus the read_deny of the asker recorded in its ledger entry
write     = ["own-worktree"]                   # the ledger is written by vsdd spike open and record
dispatch  = []
vsdd      = ["read", "spike"]

[callers]                                      # group = the vsdd subcommands in it
read     = ["status", "list", "refs", "coverage", "path", "validate", "config", "explain", "rules", "review show"]
measure  = ["gate"]
refreeze = ["gate red --refreeze"]             # held by the test-writer's row only
route    = ["feature route"]
pipeline = ["survey", "checkpoint", "plan schedule", "story", "release", "idea", "report", "discover", "repos split", "status --fix"]
spike    = ["spike"]
env      = ["env"]
author   = ["new", "note", "defer", "retire", "driver", "lock release", "review prepare", "init", "import --dry-run", "checkpoint brief", "status --fix"]
operator = ["doctor", "events", "obs", "export", "render", "hooks", "repos scan", "--version"]

[reveals]                                      # generated at build time from the class of each artifact kind
"list scenario" = ["holdout"]
"list story"    = ["stories"]
"list contract" = ["spec-product"]
"refs --include-walled"        = ["holdout", "reviews"]
"coverage --include-walled"    = ["holdout"]
"review show"                  = ["reviews"]   # rendered findings count as review content, so a reviewer cannot read an earlier pass this way
"review show --include-walled" = ["reviews", "holdout"]
```

`outside` is a computed class: every path outside the project, the plugin directory and the temporary
directory. Rows of agents that have no shell or a `vsdd`-only shell, the clerk and the spike runner deny it,
so a prompt-injected page cannot make the researcher, a planner or a spike runner read a credential file.
Builders keep it, because they read toolchain files.

Every row denies `runs`, the main session's included (wall W3). A group in `[callers]` is matched on the
subcommand and, where the entry names one, the flag: plain `status` is in `read`, and `status --fix`, which
releases locks and removes worktrees, is only in `pipeline` and `author`. In the same way `gate red
--refreeze` is only in `refreeze`, which the test-writer's row alone holds, so the `measure` group that
every builder, every planner and the clerk hold does not carry the flag. `doctor` is in `operator`, which only
the `main` row holds. A caller with no `vsdd` key may run the `read` group only.

`[reveals]` has one entry per artifact kind, generated from the kind's class in `artifacts.toml`, so the
kind vocabulary is the registry's and cannot drift from it. `vsdd list <kind>` with a kind that has no entry
is treated as revealing every walled class, so a misspelt or new kind is refused for a walled caller instead
of passing. `refs`, `coverage` and `review show` leave walled content out by default, so they reveal nothing
walled without the flag; with `--include-walled` they reveal the classes listed, and the rule blocks the
flag for a caller that is denied any of them.

| Key | Meaning |
|---|---|
| `read_deny` | Path classes the caller may not read. |
| `read_allow` | Classes that stay readable even when a class that contains them is denied. |
| `write` | The complete list of classes the caller may write. `:create` allows new files only. `:before-red` allows the class only until the story has red evidence; a plain `source` grant inside a story requires that evidence. `:frozen` on `tests` also allows changing the test files frozen at the red gate; a plain `tests` grant does not (section 2.11.4). |
| `story_only` | The caller may write project code only inside a story worktree (`pipeline.md` wall W5). |
| `dispatch` | Agent types the caller may start with the `Agent` tool. `"*"` means any. Absent means none, so every agent row except the spike runner's and the clerk's names the spike runner. |
| `commands` | The git and GitHub command families the caller may run (section 2.11.1). `"*"` means any, and is the default when the key is absent. |
| `tool_only` | Every Bash command the caller runs must be a call of the plugin's command-line tool (the `clerk`, need H8). |
| `vsdd` | The groups of `vsdd` subcommands the caller may run, from `[callers]` in this file. Absent means `read` only. |
| `spike` | Marks the spike agent's row. |
| `relaxed_by` | On the `main` row only: the `[hooks]` key that relaxes it (decision D6). No agent row may carry it; lint checks that. |

Write-grant suffixes, complete: `:create` (new files only); `:frozen` on `tests`; `:before-red` on `source`;
and `@<method>`, which limits a grant to a story whose method is the one named. `source:create@repo-setup`
is what lets `devops` create the first source and test files of an empty repository in the scaffold chore
story, and nowhere else.

`artifacts.toml` has one table per artifact kind (section 2.11.3), and a table of story kinds with whether
each has a red gate.

Three lists that the first pass kept as data are constants in `vsdd/hooks/model.py`, because they change
only when the tool's code changes and a second copy could drift:

| Constant | Value |
|---|---|
| `HUMAN_ONLY` | `decide`, `config set`, `cycle close`, `lock release --force`, `init --fresh`, `import` (`import --dry-run` is allowed) |
| `GOVERNED` | `story deliver`, `release`: the only routes onto the integration and release branches |
| Kinds exempt from tests before code | Not a list at all: `kind_has_red_gate(kind)` reads the story-kind table, which today exempts `facade`, `docs`, `chore` and `harness` |

`gate-guard` refuses a `vsdd` call whose subcommand is in a group the caller's row does not grant. A
builder row grants `read` and `measure` (and `refreeze` for the test-writer); a planner row the same, with
`route` for the architect; the clerk row `read`, `pipeline`, `measure`, `route` and `env`; the main session's
row `read`, `author` and `operator`; the rows of the demo-recorder, the ui-reviewer and the holdout-evaluator
hold `env`. The person-only commands are refused for every caller. `read-walls` treats a read subcommand as
a read of the classes `[reveals]` lists for it, so `vsdd list scenario` by an implementer is blocked exactly
as a `Read` of a scenario file is.

### 2.8 Staying inert

`find-root.sh` decides, in shell and without starting any process:

1. Start at `CLAUDE_PROJECT_DIR` (the project root where the session started), or the working directory if
   that is unset.
2. At each directory, test for `.factory/factory.toml`. If it exists, that directory is the project root.
3. If the directory's `.git` is a file that points into `<main>/.git/worktrees/`, also test the main
   checkout. This covers a session started inside a linked worktree.
4. Otherwise move to the parent. Stop at the user's home directory or the filesystem root, so a stray file
   there can never turn every project into a factory project.

If no root is found the launcher exits 0 immediately: no uv, no Python, no output, no file created. The old
dispatcher needed a "mount gate" to avoid creating `.factory/logs` in unrelated projects; here nothing is
written before the root is known. The Python side receives the root in `VSDD_FACTORY_ROOT` as the session's
project. For each call it then resolves the governing project from the path being touched (section 2.5).
A multi-repository project has one state directory, in its hub (`state.md` section 2.14): a member
repository cloned under the hub has no state of its own, and walking up from a path inside it reaches the
hub's state, which governs it. The one case not covered is a session started outside any factory project
that reaches into one: the launcher has already exited.

A project is in one of the five conditions that `vsdd.state.detect` returns; `state.md` section 2.3 is the
one definition, and this table is the same five as the hook runner meets them. The first three reconcile two
requirements in `rewrite/STATE.md` that pull against each other ("hooks do nothing in a project that has not
been initialised" and "the briefing says whether the project is initialised"); decision D24 confirmed that
reading.

| Condition (`detect` value) | How it is recognised | Behaviour |
|---|---|---|
| Not a factory project (`none`) | Neither `.factory/factory.toml` nor `.factory/STATE.md` exists | Completely inert and silent. |
| Pre-1.0 state (`legacy`) | `.factory/STATE.md` exists and `.factory/factory.toml` does not | Inert: no gate runs. On `SessionStart` only, the launcher prints one fixed line naming the migration step. No Python starts. |
| State exists but is not mounted (`unmounted`) | `.factory/` is absent and the state branch exists locally or on the remote | Inert and silent, because there is no configuration file to find. `vsdd doctor` and `setup` report it. |
| Partial (`partial`) | `factory.toml` parses with `schema = 1`, and the mount test fails: `.factory/` is not a worktree on the state branch. A missing status record is no longer a cause, because there is no status record | All gates run. State writes are blocked (`write-scopes` check 7). The briefing says so and names `/vsdd-factory:status --fix`. |
| Ready (`ready`) | The configuration parses and the mount test passes | Normal operation. |

An unknown `schema` value in the configuration is not a sixth condition: it is an invalid configuration, and
the rules run on their built-in defaults (section 2.9).

### 2.9 Failure behaviour

The principle: when the hook stands between the model and an action, a failure blocks. When blocking would
only strand the session (briefing, compaction, logging), a failure lets the session continue and is
announced at the next opportunity.

| Failure | On a gate event | On `SessionStart` | On other events |
|---|---|---|---|
| `uv` is not installed | Exit 2: "uv is not installed, so the factory gates cannot run", with the fix | Fixed JSON briefing to the person and the model | Exit 0 |
| No Python 3.11 or newer (uv exits 2) | Exit 2, with the fix `uv python install 3.12` | Same | Exit 0 |
| uv cannot run (cache not writable, unknown flag on an old uv) | Exit 2, naming the exit code and three terminal commands that need nothing from the plugin: `uv --version` (is it at least the tested version), `uv cache dir` (is that directory writable), and the upgrade or reinstall command (`uv self update`, or `brew upgrade uv`). It does not name `vsdd doctor`, which starts through the same launcher and would fail the same way | Same | Exit 0 |
| The plugin's files are missing (the launcher cannot be read) | In a factory project: exit 2 from the self-guarding command, with the reinstall message. In any other project: exit 0, nothing printed (section 2.3) | In a factory project: the fixed down-briefing with `systemMessage` | Exit 0 |
| Runner crashes before rules run (import error, syntax error after a bad update) | Exit 2 through the launcher's exit-code mapping | Same | Exit 0 |
| Runner killed (signal, out of memory) | Exit 2 | Same | Exit 0 |
| Hook input is not valid JSON | Block `runner_bad_input` | Fallback briefing | Logged |
| A gate rule raises an exception | Block `rule_error` naming the rule and the exception type; the other rules still run | n/a | n/a |
| The runner's deadline passes | Block `runner_timeout` naming the rule that was running | Fallback briefing | Logged, exit 0 |
| Claude Code's own timeout passes, or the hook process dies before it can exit | Blocked by `onFailure: "block"` | Claude Code shows a hook error | Ignored |
| The event log cannot be written | The decision is unchanged; the block message is still printed | Briefing adds "event log not writable" | Ignored |
| Project configuration is invalid | Built-in defaults (the strictest setting) | Briefing says so | Defaults |
| A policy table is missing or invalid | Block `policy_invalid` for the rules that need it | Briefing says so | Logged |

Which rules fail closed, and why.

| Rule | Fails | Reason |
|---|---|---|
| `bash-destructive`, `bash-git`, `merge-guard` | closed | The actions are irreversible. |
| `secrets`, `read-walls` | closed | A secret or a holdout scenario cannot be unread. |
| `write-scopes`, `gate-guard`, `artifacts` | closed | The write would land; the agent can simply retry once the fault is fixed. |
| `tests-first` | closed | The gate is the point of the method. |
| `decisions` | closed | A decision that could not be recorded must not look recorded; the person sees the reason and retypes. |
| `step-results` | closed | A step result that is not on disk breaks resume, and an ungrounded finding must go back to its author. The platform's retry limit bounds the loop. |
| `spike-bounds` | closed, except at stop | A spike with no ledger entry or past its time box must not continue. A finish without a record is refused once; the second attempt is allowed and logged, so an agent is never trapped. |
| `session-briefing` | open, announced | Blocking a session start helps nobody; the fallback text says the briefing failed. |
| `compaction` | open, announced | Blocking automatic compaction can fail the request outright (hooks documentation). The next briefing reports the failed flush. |
| `lifecycle-log`, `purity-advice` | open | Recording and advice only. |
| `auto-allow` | open | No permit means the normal permission prompt. |

How "no gate switches off silently" is met, in one place:

1. Every launch failure on a gate event becomes a block with a reason (launcher), backed by
   `onFailure: "block"` (platform).
2. A rule crash or timeout blocks for every gate rule.
3. Rule settings cannot be changed by a tool call, relaxations are listed in every briefing, and changes are
   announced.
4. The emergency switch is an environment variable only a person can set, and it is announced.
5. `hooks.json` is generated from the rule table, so a rule cannot be registered and unwired, which is what
   happened to the old pre-compaction hooks.
6. Every gate rule has a case that must produce exit 2 through the real launcher, and one that must pass.
   The meta-test in `packaging.md` enforces that such cases exist; `vsdd hooks selftest` runs them against
   an installed plugin.
7. If hooks are switched off at the platform level, which the runner cannot see, a workflow still finds
   out: the runner writes a caller marker for every `vsdd` call, and `vsdd survey` refuses to start a run
   when its own call has none (section 2.11.10). `vsdd doctor`, run through the Bash tool, makes the same
   test on its own call, so there is no separate canary command (section 4.1, item 10).
8. The command in `hooks.json` guards itself against a missing launcher, in factory projects only (section
   2.3), and `gate-guard` blocks the tool calls that would disable the plugin or set the emergency switch
   in a settings file.

### 2.10 The 1.0 rule list

All gates run on `PreToolUse` unless stated. "Relax" says whether project configuration may lower the
rule's strength.

| # | Rule | Tools or event | What it checks | Result | Relax | Old hooks it replaces |
|---|---|---|---|---|---|---|
| 1 | `gate-guard` | Edit, Write, MultiEdit, Bash | Writes to the installed plugin, to the runner's own records, and to Claude Code settings that would disable hooks or this plugin. Tool subcommands only a person may run, and subcommands the caller's row does not grant. A command that sets a `VSDD_*` variable itself. | Block | Locked | new (closes the "agent disables its own gate" hole) |
| 2 | `secrets` | Read, Grep, Bash | Reading, copying, archiving or printing `.env` files and secret-shaped variables | Block | Yes | `protect-secrets` (both entries) |
| 3 | `read-walls` | Read, Grep, Glob, Bash, Agent | The caller's row may not read the path class the call reaches; the dispatch table | Block | Locked | the 49 unenforced `context.exclude` walls in the old workflow files |
| 4 | `write-scopes` | Edit, Write, MultiEdit, NotebookEdit, Bash | The caller's row may write this class; code-only and read-only classes; shell writes into the state directory; spike containment | Block | Locked | `brownfield-discipline`, `validate-factory-path-root` |
| 5 | `artifacts` | Edit, Write, MultiEdit | The file's content after the call is a valid artifact of its kind: path, identifier, frontmatter, headings, references, size cap, frozen status, forbidden patterns | Block | Yes | `validate-template-compliance`, `protect-bc`, `protect-vp`, `validate-artifact-path`, `validate-state-size`, `validate-stable-anchors` and seven more (section 3) |
| 6 | `tests-first` | Edit, Write, MultiEdit, NotebookEdit | In a story, product source is written by the implementer only after red evidence exists, and by the test writer (stubs) only before; tests frozen at the red gate stay frozen for other builders | Block | Yes | `red-gate` |
| 7 | `merge-guard` | Bash | A direct merge into, or push to, the integration or release branch is refused, and the governed tool command is named | Block | Yes | `validate-pr-merge-prerequisites` |
| 8 | `bash-destructive` | Bash | Destructive shell, git and GitHub commands | Block | Yes | `destructive-command-guard` |
| 9 | `bash-git` | Bash | Force push, `--no-verify`, AI attribution, command families by agent, spike branches leaving their sandbox | Block | Yes | `verify-git-push`, `block-ai-attribution` |
| 10 | `spike-bounds` | Any guarded tool called by a spike runner, including its `StructuredOutput` call; `Agent` (to record who dispatched a spike runner); `SubagentStart`; `SubagentStop` | A spike runner works only with an open ledger entry, only inside its time box, under the walls of whoever asked, and may not finish without a recorded finding | Block | Time box tunable | new |
| 11 | `decisions` | `UserPromptExpansion` | Nothing to refuse except a malformed or stale decision; records the decision a person typed | Block or Context | Locked | the 37 unenforced approval steps |
| 12 | `step-results` | StructuredOutput | A review finding must point at a real file, line range and quote; then the step result is stored | Block | Locked | `handoff-validator` in part; about sixty old state-manager "backup" steps |
| 13 | `session-briefing` | `SessionStart` | Nothing; composes the briefing | Context | Locked | `session-start-telemetry`, `postcompact-reanchor`, `warn-pending-wave-gate` |
| 14 | `compaction` | `PreCompact` | Nothing; flushes state and tells the summariser what to keep. It writes no snapshot file | Context | No | `precompact-flush` |
| 15 | `lifecycle-log` | `SubagentStart`, `SubagentStop`, `SessionEnd`, and every guarded call for the lock's liveness | Nothing; records agent and session lifecycle; keeps the run lock alive and releases it at session end; stops a product an agent left running; prunes old logs. Commit and pull-request events are written by the tool | Record | No | `track-agent-start`, `track-agent-stop`, `session-end-telemetry`; `capture-commit-activity` and `capture-pr-activity` move to the tool |
| 16 | `purity-advice` | Edit, Write, MultiEdit | Side-effect patterns in files on configured pure-core paths | Advise | Yes | `purity-check` |
| 17 | `auto-allow` | Read, Bash, Edit, Write, Workflow, and for the `extended` scope the web and research tools | Built-in: the plugin reading its own method and template files and calling its own command. By the project's answer to decision D4: the `core` or `extended` scope of section 2.11.10 | Permit | Scope set at setup: `off`, `core` or `extended` | new (decision D4) |

### 2.11 The rules in detail

#### 2.11.1 General safety: `bash-destructive`, `bash-git`, `secrets`

These three carry over the rules of the old `destructive-command-guard.sh`, `verify-git-push.sh`,
`block-ai-attribution` and `protect-secrets.sh`, with the same reason codes so the old test cases can be
harvested as `packaging.md` describes. Two things change.

First, they work on a parsed command, not on the whole string. `shell.py` splits the command into segments
at `;`, `&&`, `||`, `|` and newlines, tokenises each with `shlex`, and records redirect targets and any
`cd <dir>` or `git -C <dir>` prefix. This removes the old false positives (a commit message that mentions
`git reset --hard` was blocked) and the old misses (`rm -r -f`, `rm --recursive`).

Second, they read what the command points at. A commit made with `git commit -F <file>` has its message
file scanned for attribution; the old hook scanned only the command string.

| Rule | Blocks |
|---|---|
| `bash-destructive` | Recursive `rm` of `/`, `~`, `$HOME`, `*`, `.*`. Recursive delete (`rm -r`, `find -delete`, `find -exec rm`) under the state directory or the project's `source` and `tests` classes, except build output and worktree directories. Delete, truncate or clobbering redirect of a state file. `git reset --hard`, `git clean -f` (not a dry run), `git checkout -- .`, `git restore .`, `git stash drop` and `clear`. `git branch -D` of a protected branch, `git filter-branch`, `git filter-repo`, `git reflog expire --expire=now`, `git gc --prune=now`, `git worktree remove --force` outside the worktree roots. `gh repo delete`, `gh release delete`, `gh issue delete`, `gh pr close`. `curl`, `wget` or `fetch` piped into a shell or interpreter. Recursive `chmod` or `chown` on protected paths. |
| `bash-git` | `git push --force` and `-f` (with `--force-with-lease` allowed on branches other than the integration and release branches). `--no-verify`, `-n` on commit, `--no-gpg-sign`, and `-c core.hooksPath=`. AI attribution (the eleven old patterns) in a commit message or in pull request text. A command family the caller's row does not grant. For spikes: see 2.11.6. Pushes and merges onto the integration and release branches belong to `merge-guard` (2.11.4). |
| `secrets` | `Read` or `Grep` of `.env`, `.env.*` and `.envrc` (templates named `.example`, `.sample`, `.template` are allowed), of `.mcp.json` (it holds live keys in most projects), of home credential files (`~/.aws/**`, `~/.ssh/**`, `~/.netrc`, `~/.docker/config.json`, `~/.config/gh/**`, `~/.npmrc`, Claude Code's credential files), of `*.pem` and `*.key`, of `/proc/*/environ`, and of the configured `secret_globs`. Shell reads, copies and archives of the same files. `gh auth token` and `git credential fill`. `echo` or `printf` of a secret-shaped variable. Bare `env`, `printenv` and `set`, and the same piped to `grep`. Also `export -p` and `declare -x`, which print the environment as `env` does. For the web tools the matcher names (`WebFetch`, `WebSearch`, the research tools): every call is logged as `web.call` with its host, and a call is blocked when its address or query contains the value of a secret-shaped environment variable or a common token format. The same log and the same check apply to every web address in a `curl`, `wget` or `http` command an agent row runs in Bash, because a `GET` address can carry data out as well as an upload can; this is what bounds a spike runner, which has both web tools and a shell. For agents, `curl` and `wget` with an upload flag (`-d`, `--data*`, `-F`, `-T`, `--upload-file`, `--post-file`) are blocked, with one exception: every target address is loopback (`localhost`, `127.0.0.1`, `[::1]`) on a port that `vsdd env start` recorded for the current run. That exception is what lets the holdout evaluator and the demo recorder exercise the `POST` and `PUT` endpoints of the product the tool started; without it, scenarios for an API product would all score as not run. The case table has an allowed loopback upload, a blocked upload to another host, and a blocked upload to a loopback port the run did not start. |

Command families. `agents-skills.md` proposes command scopes by agent; `bash-git` implements them. `shell.py`
classifies each segment into at most one family, and a row's `commands` list grants families.

| Family | Commands |
|---|---|
| `git-commit` | `git commit`, `git add`, `git rm`, `git mv` |
| `git-push` | `git push`, `git tag` with a push |
| `git-integrate` | `git merge`, `git rebase`, `git cherry-pick` |
| `git-worktree` | `git worktree add`, `remove`, `prune` |
| `gh-pr` | `gh pr create`, `edit`, `merge` |
| `git-read` | every other `git` and `gh` command |

A row with `commands = []` may run none of them, which is how the holdout evaluator gets "no git". Its other
restriction, no command that names a source, test or specification path, is `read-walls` applied to Bash.
A call of the plugin's own tool is in no git family. `gate-guard` decides it instead: the subcommand must
be in a group the caller's row grants (`[callers]` in `walls.toml`), and the person-only subcommands are
refused for every caller. A row with `tool_only = true` (the clerk and the four planners) may run nothing
but calls of the tool, written as `vsdd ...` or `vsdd -C <dir> ...` in a single segment, with no `cd` in
front, no pipe and no redirect. `-C <dir>` is a global option of the tool (`state.md` 2.16), which is why a
clerk never needs a `cd`. A command that assigns a `VSDD_*` variable itself is blocked for every caller,
because the runner adds the caller marker's key that way (section 2.11.10). A Bash command that reaches the tool another way (it
names `py/vsdd`, `cli.py`, `hook.py` or `launch.sh`, or imports the package from Python) is blocked for
every caller, so the checks on subcommands cannot be stepped around.

Inside a spike worktree the "discard work" commands (`git reset --hard`, `git clean -f`, `git checkout -- .`,
`git restore .`, `git stash drop`) are allowed when the command has no `cd`, `-C`, `--git-dir` or
`--work-tree` that points elsewhere, because a spike worktree is disposable.

#### 2.11.2 Information walls: `read-walls` and `write-scopes`

The walls are enforced from the two tables in section 2.7 and keyed on the calling agent's type, which the
spike proved is present in the payload.

`read-walls` blocks when a path the call would read is in a class listed in the row's `read_deny`, unless a
class that contains the path is listed in `read_allow`. How "the paths the call would read" is found:

| Tool | Paths considered |
|---|---|
| `Read` | `file_path`, literal and with symlinks resolved. |
| `Grep`, `Glob` | The search root (`path`, default `cwd`). Blocked if the root is inside a denied class. If the root is an ancestor of a denied class: the platform probe of 2026-11-23 confirmed that `Grep` skips a directory listed in `.git/info/exclude` (the state directory is listed there), so the call is allowed for every row whose denied classes all lie under `.factory/`. The weekly conformance tier re-checks it; if it ever fails, the call is blocked with the fix "narrow the path". A row that denies product classes (the holdout evaluator, the scenario reviewer) is always blocked from an ancestor search. |
| `Bash` | Every token that resolves into a denied class; any occurrence of the class's literal path prefix in the command text; any token with glob characters that expands (capped at 200 results) to a path in the class; a `vsdd` read subcommand whose `[reveals]` entry names a denied class; and `npx playwright` or a browser command given a `file:` address in a denied class. |
| Browser tools (`browser_navigate`, `browser_navigate_back`, `browser_tabs`, `browser_file_upload`, in both spellings) | A `file:` address is treated as a `Read` of the path it names, in a navigation and in a new tab. An upload from a path in a denied class is blocked. The plugin's own Playwright entry in `.mcp.json` also carries the server's option that refuses `file:` addresses, and lint (L-MCP) checks that the option is present. If the user runs their own Playwright server the platform may keep theirs and drop the bundled one (blueprint section 10, item 33); the address check above applies to both spellings, so the wall does not depend on which server survives. `browser_click` is not matched: a Chromium page loaded over `http` cannot navigate to a `file:` address by a click, so there is nothing for the rule to see. |
| `Agent` | The row's `dispatch` list. A factory agent may start only the spike runner, and the spike runner may start nothing, so a walled agent cannot send an unwalled helper to read for it. The spike runner it starts inherits its walls (section 2.11.6). |
| The final message of a dispatched agent (`SubagentStop`) | Only when the main session dispatched an agent whose row may read `holdout` (`product-owner`, `scenario-reviewer`, `holdout-evaluator`), and `main_session_wall` is on. The rule runs the tool's scenario-text check on the agent's final message: a message that quotes a hidden scenario is refused with "say what you did without quoting scenario text", every time and not only the first, and each refusal is logged. There is no second-attempt pass here, unlike the spike stop check, because the text would then reach the main session. This closes the path by which the main session, which may not read scenarios, could receive them in a reply. A dispatched agent returns text, and the platform probe showed that a refused stop holds such an agent. |

Note on searches from an ancestor by the main session. The main session may not read `holdout`, but it
searches from the project root all the time, and blocking that would be unusable. The options, in order of
preference, are: the state design keeps `holdout` where a recursive search from the project root does not
reach it (`agents-skills.md` N-STATE-8); or a `.ignore` file in the state directory hides it from ripgrep;
or the rule rewrites the search through `updatedInput` to exclude it. The first is what the state design
does, and the probe confirmed it works: the state directory is in `.git/info/exclude` and `Grep` skips it.
The main session's project-wide searches are therefore allowed.

`write-scopes` evaluates, for a target path P and the caller's row:

1. P is in a class whose `write` is `none` or `code`: block. This makes reference code read-only, and makes
   evidence, the decision and closure records and the project configuration unforgeable by a tool call.
2. P's class is not one the row's `write` list grants, and P is not in `tmp`: block, with the class's
   `route` sentence as the fix. This is what makes a reviewer read-only apart from its report and stops an
   architect from rewriting a contract. For a path in the product tree, "P's class" is the one class the
   precedence of section 2.7 gives it, not every class whose globs match. So `src/parser/tests.rs` is in
   `tests`: the test-writer may write it, before and after red, and the implementer, whose row grants
   `source` and `manifests`, may not. A product path in no class is blocked for every agent with the
   `[paths]` line to add. A grant with `@<method>` counts only in a story whose method is that one.
3. The grant is `:create` and P already exists: block.
4. The call is a `Bash` command that writes into the state directory (a redirect, `tee`, `sed -i`,
   `perl -i`, `cp`, `mv`, `rm`, `touch`, `truncate`, `dd of=`, `ln`): block for every caller, with the fix
   "use the Edit or Write tool, or the vsdd command". A call to the `vsdd` command is not a shell write.
5. P contains a state-directory segment but does not resolve to the project's one state directory (the
   classic case is a relative `.factory/...` path written from inside a worktree): block, and print the
   correct absolute path.
6. The row has `story_only` and P is project code outside a story worktree: block. A builder cannot change
   the main checkout or the integration worktree.
7. P is in the state directory and the project's condition is `partial` because the mount test fails (the
   state directory is not a worktree on the state branch): block, and name `/vsdd-factory:status --fix`.
   State is never written into a directory that is not mounted.

Check 5 applies to reads as well: `read-walls` blocks a `Read`, `Grep` or `Glob` of a `.factory` path
inside a linked worktree and prints the correct path, so an agent cannot read a stray copy.

Cases the second pass added to `tests/hooks/cases/write-scopes.toml`, because the first wave of every
greenfield project depends on them: `devops` creates `Cargo.toml` and `src/main.rs` in a chore story with
method `repo-setup` (allowed); `devops` creates `src/main.rs` in a chore story with method `ci` (blocked);
the implementer adds a dependency line to `Cargo.toml` in a story worktree (allowed; the dependency decision
comes at delivery); the implementer edits `.github/workflows/ci.yml` (blocked, route: a chore story); the
verifier creates `fuzz/fuzz_targets/parse.rs` in a harness story (allowed); the test-writer edits
`src/parser/tests.rs` after red (allowed); the implementer edits `src/parser/tests.rs` (blocked); any agent
writes `scripts/build.sh` when no class matches it (blocked, with the line to add).

Limits of the walls are stated plainly in section 7: the shell is not sandboxed, so the `Bash` checks are
heuristics, and tools the runner cannot inspect are handled by keeping them off walled agents (need A2).

#### 2.11.3 Artifact validity: `artifacts`

The rule looks up the target path in `artifacts.toml` and validates `ctx.new_content`, the text the file
would contain after the call. Because it runs on `PreToolUse`, an invalid artifact never reaches the disk.
The old validators ran after the write, could only complain, and needed a marker file and two more hooks to
cope when they timed out.

```toml
[story]                                      # illustrative; the content is state.md section 2.5
glob        = ".factory/stories/E-*/S-*.md"
id          = '^S-(\d+)\.(\d{2})$'           # must equal the file stem and the frontmatter id
dir         = "E-{1}"                        # the parent directory is derived from the identifier
frontmatter = { id = "str", title = "str",
                status = ["draft", "dropped"],        # "ready" and "done" are computed from closure records, not written
                kind = ["feature", "fix", "port", "facade", "docs", "chore", "harness"],
                contracts = "list[ref:contract]" }   # syntax is checked here; existence at the gates
headings    = ["Acceptance criteria"]
max_bytes   = 16000
lock        = "done"                         # frozen once its delivery closure exists (artifact_locked)
forbid      = ['\b[\w./-]+\.(rs|py|ts|tsx|js|go|java):\d+\b']   # volatile file:line citations
tables      = true                           # every table row has as many cells as its header

[_unknown]
policy = "block"                             # a write under the state directory that matches no kind
```

| Check | Reason code |
|---|---|
| Path matches no known kind | `unknown_artifact_path` |
| The file is locked. Either it is in a specification class (`spec-product`, `spec-arch`, `spec-ux`), new or existing, while the specification set is locked: the latest `spec` decision is approved and no live run holds an amendment (`spec_locked(project)`; `pipeline.md` guard G3). The lock is over the whole set and does not look at the file's status, so it also covers the files that never had one. The `brief` class is outside it, and outside the `spec` decision's digest, because a brief has its own decision. The two guidance files, `conventions.md` and `review-rules.md`, are locked with the set while a cycle is open and writable by the architect between cycles (`guidance_locked(project)`), so a retrospective's proposal can be adopted without an amendment run and the rules cannot change in the middle of a build; they are outside the `spec` decision's digest (`state.md` 2.8), so that edit leaves the approval current. Or the file is individually frozen whatever the amendment state: a decision record an approved `spec` decision accepted, a proven property, a done story (`artifact_locked(project, path)`). One record read per guarded write | `artifact_frozen` |
| Frontmatter missing, outside the supported subset, or a required key absent or of the wrong type | `frontmatter_invalid` |
| Identifier does not match the pattern, the file name, or the directory `dir` derives from it; or the identifier is already taken (the message names the next free one) | `id_mismatch` |
| The status value is not one the kind's frontmatter lists. The lists hold only values an agent may set (`draft`, `retired`, `dropped`, and so on); `approved`, `ready`, `done`, `proven` and `accepted` are computed from records and are in no list, so an agent that writes one is refused by the ordinary frontmatter check. There is no separate `code_only_value` check | `frontmatter_invalid` |
| A reference is not a well-formed identifier of the referenced kind. Whether the artifact exists is not checked at the write, because stages write in an order (contracts before subsystems) and in parallel (stories by epic); `vsdd gate spec`, `vsdd gate plan`, `vsdd validate` and approval check existence | `reference_malformed` at the write; `dangling_reference` at a gate |
| A required heading is missing | `heading_missing` |
| Content exceeds the size cap (a write that makes the file smaller is always allowed) | `size_cap` |
| A forbidden pattern is present | `forbidden_pattern` |
| A table row has the wrong number of cells | `table_malformed` |

The same module backs `vsdd validate` (`packaging.md` section 2.10), which workflows and CI call at gates.
That is the backstop for anything that reached the disk some other way.

The content of `artifacts.toml` belongs to the state design. `packaging.md` asks the state design for the
same schemas as data (its need S3); there must be one copy, and this rule reads whichever file that is.

#### 2.11.4 Delivery order: `tests-first` and `merge-guard`

These rules follow the split in `pipeline.md` section 2.2: scripts steer, the tool measures, hooks enforce.
The tool's gate command runs the project's configured commands and writes an evidence record (gate,
subject, commit, passed, measurements) that no agent tool call can write. A hook then refuses the action
the evidence does not support. Nothing here reads a prompt; the old hooks found the story by running `grep`
over the dispatch prompt and found the project by looking for `cd <path> &&` in it.

That split is also the right answer to a question I had settled the same way before reading it: evidence
must come from a command that sees the real exit code. Watching test commands in `PostToolUse` does not
work, because agents routinely pipe test output through `tail`, which hides the exit code, and a false
"green" is the dangerous direction.

Which story a call belongs to: `ctx.story` is the story whose worktree contains the target path. The
pipeline creates story worktrees at `.worktrees/<story-id>/`; the rule also accepts any worktree whose
checked-out branch matches the story branch pattern (need P3), so it holds when a worktree was made another
way.

`tests-first` is the replacement for `red-gate` and implements `pipeline.md` guard G1 and wall W4 and the
write-scope split of `agents-skills.md` (its need N-HOOK-4). The old hook trusted a JSON list of "currently
red" files that agents edited by hand.

- It applies when `ctx.story` is set, the work is not in a spike, and the story's kind has a red gate
  (`kind_has_red_gate`, read from the story-kind table: every kind except `facade`, `docs`, `chore` and
  `harness`).
- Which class a path is in follows the precedence of section 2.7, as in `write-scopes`. A file that both a
  source glob and a tests glob match is a test file. So the test-writer is not caught by
  `red_already_recorded` when it fixes `src/parser/tests.rs` after red, and the implementer cannot edit that
  file under its `source` grant. Three cases hold it: the test-writer edits `src/<m>/tests.rs` after red
  (allowed); the implementer edits it (blocked by `write-scopes`); the red gate counts the tests in it.
- A frozen test the test-writer changes after red is recorded by `vsdd gate red --refreeze`, which
  `gate-guard` allows for the test-writer's row only. This rule has nothing to do there: the green gate
  compares frozen tests with the latest red or re-freeze record.
- "A passing `red` record for the story" means the latest passing red record on any commit of the story
  branch since its base, which is what the state design's `evidence` function returns for the `red` gate. It
  is not tied to the head commit: the red gate passes on the test writer's commit, and the record must keep
  holding as the implementer commits once per test. The case tables include an implementer edit after its
  own commit (allowed) and a test-writer source edit after red at a later commit (blocked).
- For a path in the project's `source` class: a caller whose grant is plain `source` (the implementer, and
  the main session) is blocked until the evidence store holds a passing `red` record for the story
  (`no_red_evidence`). A caller whose grant is `source:before-red` (the test writer, for stubs) is blocked
  once that record exists (`red_already_recorded`).
- For a test file that the red record lists as frozen: only a caller whose grant is `tests:frozen` (the
  test writer) may change it (`red_test_frozen`). Other builders that may write tests, such as the verifier
  in its harness directories, cannot touch the tests that defined the story.
- The implementer cannot weaken any test, because its row does not grant `tests` at all. That is
  `write-scopes`, not this rule.

Wave order needs no hook of its own. The old `validate-wave-gate-prerequisite` blocked a dispatch for a
later wave while an earlier wave's gate was pending. In 1.0 the tool creates a story's worktree only when
the story is ready (in the survey, or when it admits a story written after the survey), a builder can
write project code only inside a story worktree (`story_only`),
and the tool refuses to close a wave without its evidence. An agent started out of order has nowhere to
write.

`merge-guard` implements `pipeline.md` guard G2 and `packaging.md` need H5: no agent merges into, or pushes
to, the integration or release branch except through the tool. The rule blocks a tool call that runs
`gh pr merge`; `git merge`, `git rebase` or `git cherry-pick` while the integration or release branch is
checked out; `git commit` (including `--amend`) and a `git reset` that moves the branch while one of those
branches is checked out, which matters most in local delivery mode, where the integration branch is the
product branch; and `git push` whose target is one of those branches, including `HEAD:<branch>` forms and a
bare `git push` from one of them. The fix line names the governed subcommands (the constant `GOVERNED`:
`story deliver`, `release`), and for the main session it names `/vsdd-factory:feature`. There is
no `vsdd pr-merge` subcommand: merging is a library function inside `story deliver`, so no caller can merge
with only a head-commit check. The checks that make a merge legitimate (green evidence on the head
commit, a converged verdict that covers it, the demo gate, green CI, the reviewed commit matching the
branch head) are the tool's, in code. A `gh` or `git` process the tool starts is not a tool call and is not
seen by hooks.

#### 2.11.5 Decisions only a person can record: `decisions`

`pipeline.md` section 2.6 defines human stops as decisions. A decision is opened only by the tool's
checkpoint and settled only by a person. This rule and `gate-guard` are what make "only by a person" true
(its need H5).

1. A person types `/vsdd-factory:decide <id> <choice> [key=value ...] [-- note]`. `decide` is a person-only
   skill in the agents-skills roster, so the command is registered. `UserPromptExpansion` fires before any
   model reads the message, with `command_name` and `command_args` in the payload.
2. The rule parses the arguments by one grammar: the id (which may be qualified, `release@hotfix-1.2.1`),
   the choice, then `key=value` pairs, then the note after `--`. Every value is named and a key may repeat;
   nothing is positional and nothing is quoted. Examples, each a parser fixture: `spec approve variant=B
   topology=split`; `spec revise stage=contracts -- cover the offline case`; `ideas select
   pursue=ID-003,ID-007 reject=ID-004`; `finding.S-7.04 waive key=code:src/auth.rs:lockout
   key=test:tests/auth.rs:lockout_race -- accepted for the beta`; `finding.W-02 waive-coverage item=AC-3 --
   covered by the manual check`; `blocked.S-7.04 open-spike name=lockout-timing`; `release hold -- wait for
   the security fix`. A finding key contains colons and slashes but no spaces and no `=`, so it is a legal
   value. The rule calls the state design's settle function (need S7) with the id, the choice, the pairs,
   the note and the session id. That function checks that the decision is open, that the id is not
   ambiguous, that the choice and its keys are valid for the kind, and that the digest of what is being
   approved still matches. Then it writes the decision record, and nothing else: it promotes no file and
   runs no git, so the call takes milliseconds on a specification of any size and stays far inside the
   rule's four-second deadline. The next survey commits the record.
3. If the function refuses (no such decision, an ambiguous id, already settled, stale digest, unknown choice
   or key), the rule blocks. The person sees the reason, with the choices where the id was ambiguous, and
   nothing reaches the model.
4. If it succeeds, the rule returns context for the model: the decision id, the choice, and the next command
   from the status function. If `local/driver.json` says the driver is paused at this decision in this
   session, the context adds: "The driver was paused at this decision. Resume the driver loop: start the
   command in `next` with the Workflow tool and continue." That is what lets a person approve and walk away
   without typing `/vsdd-factory:drive` again. It logs `decision.recorded`.

An agent cannot do any of this. The decision store is a code-only path class. `gate-guard` refuses every
tool call that runs a subcommand on the `human_only` list (`decide`), whoever the caller is, including the
main session's model. A person can still run it in their own terminal, where no hook applies, through the
`~/.local/bin/vsdd` shim that `setup` installs.

I use `UserPromptExpansion`, where `pipeline.md` names `UserPromptSubmit`, for one reason: the hooks
documentation says `UserPromptSubmit` also fires for scheduled tasks, background subagent reports and
messages from other sessions, so it does not prove that a person typed anything. `UserPromptExpansion`
fires when a typed command expands. It requires `decide` to be a registered plugin command (need P4). The
platform probe of 2026-11-23 proved this headless: the event fires with `command_name` and `command_args`,
a hook can add context, and a block ends the prompt with no model turn. The `UserPromptSubmit` fallback is
therefore not built.

#### 2.11.6 Spikes: bounds, exemption and containment

The spike agent is `vsdd-factory:spike-runner`. `pipeline.md` section 2.8 defines the protocol: the
runner's first command is the tool's `spike open`, which checks the ledger and the count limits and writes
a ledger entry; its last is `spike record`. Counts per step, per run and per day are enforced by the
workflow wrapper and by `spike open`, in code. The hook enforces what only a hook can (its need H6).

| Bound | Where the rule acts | What it does |
|---|---|---|
| No work without an open ledger entry | Every guarded tool call by a spike runner | Until the ledger holds an open entry for the runner, every call is blocked except `spike open`, a `Write` of the question or finding file in the temporary directory, and a `Read` of a file under the plugin's `methods/` or `templates/` (the agent file tells it to read its method first). A runner that was refused by `spike open` can therefore do nothing but return its refusal. The entry is matched by the runner's `agent_id`. The tool learns that id from the caller marker the hook writes for the runner's `spike open` call (section 2.11.10), so two runners that open at the same moment cannot be swapped, and `spike open` writes the id into the entry. For a named spike, whose entry already exists, the runner's first `spike open --name` attaches its id to that entry. Every spike is isolated, research spikes included, so no two runners share a working directory either. |
| Time box (default 20 minutes, from project configuration) | Every guarded tool call by a spike runner | The rule notes when it first saw each spike runner's `agent_id`. After the time box, every call is blocked except `spike record`, with the message "time box over: record what you have". |
| No nesting | `PreToolUse` on `Agent` | The spike runner's `dispatch` list is empty. |
| Who asked | `PreToolUse` on `Agent`, when the target is the spike runner; `SubagentStart` for a spike runner | The rule records an admission through the state design's `spike_admit`: the session and the dispatching caller's agent type (empty for the main session). When a spike runner starts, `spike_attach` pairs it with the oldest unclaimed admission of that session. If more than one admission is pending and the platform does not say which dispatch started which runner (blueprint section 10, item 35), the runner is given the union of their walls, which is the stricter result. Inside a workflow there is no dispatch: `spike open` reads the asker from the stored `needs_spike` step record instead. Either way the asker's type is a fact the hook wrote, and `spike open` refuses when it finds none. |
| No finish without a finding | The spike runner's `StructuredOutput` call when it is a workflow step; `SubagentStop` when it was dispatched with the Agent tool | If the ledger entry for the runner's `agent_id` has no verdict (or, for a named spike, no increment recorded in this run), the finish is refused once with "run `spike record` first". The platform probe showed why there are two places: a refused stop holds an agent that returns text, and does not hold a workflow step that has already returned its schema result, so for a step the rule acts on the result call, which the probe showed can be blocked and retried. If a finish was already refused once (`stop_hook_active`, or a second result call), it is allowed and `spike.stop_unrecorded` is logged, so a runner is never trapped. |

`SubagentStart` cannot block, which is why the first two bounds act on the runner's tool calls and not on
its start. Every row matches the runner by `agent_id`; the worktree is recorded in the entry and is used for
containment, not for identity.

Exemption. `ctx.in_spike` is defined in section 2.5. Rules declared `spike_exempt` are skipped when it is
true: `tests-first`, `artifacts` (a spike writes no pipeline artifacts) and `purity-advice`. The safety
rules, the walls and `gate-guard` still apply inside a spike.

Containment, so that nothing leaves a spike except through the pipeline:

- `write-scopes`: the spike runner may write only its own worktree and `tmp`. The ledger is written by the
  tool, not by a tool call. For the spike row, `own-worktree` is the worktree that holds the caller's
  working directory, and also the worktree recorded in the caller's open ledger entry: a named spike works
  in `.worktrees/spike-<name>/`, which `vsdd spike open --name` records, while the platform gives the
  runner a throwaway worktree as its working directory. `own-worktree` never means the main checkout: a
  spike runner whose working directory is the main checkout and whose entry names no worktree has none of
  its own, so every write is blocked. This is how need N-HOOK-3 of
  `agents-skills.md` is met. The hook cannot see at dispatch time whether isolation will apply, so it
  enforces the effect instead. The agent file's `isolation: worktree` makes the normal case work.
- `bash-git`: the spike runner's row grants no push, no merge and no pull request. Nobody may merge,
  cherry-pick or rebase a `spike/*` ref onto another branch, or open a pull request whose head is one.
- `read-walls`: the spike runner is denied `holdout`, `reviews`, `runs` and `outside`, and also every class
  denied to the agent type that asked. The ledger entry records that type (`asker_type`) and the resulting
  deny set (`read_deny`), and both come from hook-written records, never from an argument the runner types.
  Before the entry exists the runner can read only its method file, so there is no window in which it is
  unwalled. A spike therefore cannot be used to read around a wall, whoever asks, and the `code-reviewer`,
  the `holdout-evaluator` and the `scenario-reviewer` can ask safely. The cached-finding rule completes it:
  `spike open` returns an earlier finding only to an asker whose deny set the earlier entry's `read_deny`
  covers, so a finding produced for the adversary is never handed to the holdout evaluator.
- No relay. The first pass let the main session run a spike "on behalf of" a walled reviewer that had been
  dispatched without the Agent tool. That recorded the main session as the asker and so applied the wrong
  walls. Every agent except the spike runner and the clerk now has the Agent tool, limited to the spike
  runner, and asks for itself.

#### 2.11.7 Step results: `step-results`

A workflow script cannot write a file, so in the old design an extra agent "backed up" state after almost
every step. In 1.0 the hook does it (`pipeline.md` needs H1 and H2). A workflow step returns its result by
calling the `StructuredOutput` tool, and the rule runs on `PreToolUse` for that call.

1. If the result carries no step identity (`step.run`), the rule returns nothing and the call passes.
2. Sender. A record is accepted only when `step.run` names a live run. The rule stores the payload's
   `agent_type`, and the head commit of the unit's worktree, with every record. It does not refuse by step
   name. The first pass had it check the caller against a table of step names generated from the workflows,
   but many step names are built at run time (`review-${lens}-p${pass}`), so that table could not be
   generated. The check sits where the record is used: the tool counts a review pass toward a verdict only
   when the stored agent type is the reviewer that owns the pass's method (`review-story-diff` from the
   adversary, `security-review` from the security reviewer, and so on). A builder that stores a clean
   review under a reviewer's step name therefore changes nothing: the verdict computed before a merge
   ignores it. The stored type is also what `spike open` reads as the asker of a `needs_spike` result.
3. Grounding. For each finding in a review result, the rule checks `location`. The path is relative to the
   root of the reviewed tree, which the rule gets from the state design's `unit_tree(project, workflow,
   subject)`: the story's worktree, the integration worktree, or the project root for specification files.
   The path must exist; `line_start` and `line_end` must lie inside the file; and the `quote`, if given,
   must occur within three lines of that range, comparing with whitespace normalised. Holdout symptoms have
   no location and are not checked, and neither is a finding of category `holdout`, which the tool builds
   from scenario scores with no location (`pipeline.md` 2.7). If any finding fails, the rule blocks the call once per step, with one
   line per finding (`finding <key>: quote not found in <path>`), so the reviewer can correct it. On the
   next return it does not block again: it stores the result with each still-ungrounded finding marked
   `grounded: false`, which makes it advisory. One bad citation therefore cannot use up the platform's
   retries and lose the pass.
4. Storing. The rule writes the step record through the state design's store function (need S7), keyed by
   run, workflow, subject, name and attempt, before the call completes. It logs `step.recorded`. The tool
   validates each record against its schema, and applies the same grounding rule, before it counts the
   record toward a verdict.
5. Walled findings. A finding in a result from the `scenario-reviewer`, and any finding whose
   `location.path` lies under `.factory/holdout/`, quotes or names a hidden scenario. The store function
   writes its full form to the evaluation file in the `holdout-evaluations` class. The step record keeps it
   too, in `runs`, which no caller can read. Everything derived later (a decision packet, a closure record,
   a claim, the output of `vsdd review show`) carries its walled key (a hash of the key, `pipeline.md` 2.7)
   and severity only, unless the reader may read `holdout`.
6. Spike runners. When the caller is a spike runner and the result is its spike finding, `spike-bounds`
   runs on this same call (section 2.11.6).

The rule does not re-validate the whole result against its JSON Schema: the workflow runtime already does
that, and the standard library has no schema validator. It reads only the fields it uses, whose names come
from `<plugin>/schemas/step-result.schema.json` and `finding.schema.json`.

The two platform facts under this rule were proven by the probe of 2026-11-23: the payload carries the
whole result as `tool_input`, and a block makes the agent correct the result and return again. The fallback
the first pass kept ready, an agent running `vsdd step record` as its last act, is removed from the design.
It could not have worked: nine of the twenty agents have no shell, every pipeline reviewer among them. If a
later platform release changes the behaviour, the fallback is one that needs no tool in the agent: the
runner stores the result at `SubagentStop` from the subagent's transcript (`agent_transcript_path`), and
ungrounded findings are marked advisory when the tool computes the verdict. The weekly conformance tier
re-checks the two facts.

#### 2.11.8 The session briefing: `session-briefing`

The state design owns what the briefing says and its length limit, and `packaging.md` asks that the same
function back `vsdd status` (its need S5). This rule owns how the briefing is produced and delivered.

- It runs on every `SessionStart` source: startup, resume, clear, compact and fork. The spike proved that
  `additionalContext` reaches the model for startup, resume and compact.
- It calls the state design's briefing function (need S4), which computes the status from the records
  (there is no stored status record) and returns the briefing lines: phase, what is in flight, open
  decisions and named spikes, the exact next command, and whether the driver is active or paused.
- After a compaction it adds nothing of its own about what was running. The briefing's "Running" line names
  the live workflow from the lock and the run record, and the survey reports worktrees with uncommitted
  work when the workflow continues, so the first pass's compaction snapshot repeated both and is gone.
- It adds runner lines only when there is something to say: gates that are relaxed; the main-session wall
  switched off; the `auto-allow` scope; a change to any gate-bearing section of the configuration since the
  last session (the fingerprint list of section 2.6, not only `[hooks]`), including a write the tool made;
  a configuration or policy fault; another session that started in this project in the last 30 minutes and
  has not ended. When all is normal it adds nothing.
- It enforces the state design's length limit, and a hard ceiling of 2,000 characters of its own.
- It logs `session.start`.

In a project with no configuration file none of this runs (section 2.8).

#### 2.11.9 The pre-compaction flush: `compaction`

The old pre-compaction and post-compaction hooks were registered and never wired, so this is the first
version that runs. The rule was named `compaction-snapshot` in the first pass, when it also wrote a snapshot
file; the lock, the run record and the survey already hold what the snapshot repeated, and collecting it
cost a `git status` per worktree inside the same budget the flush needs, so the file and its writer are
gone. On `PreCompact` (manual or automatic) the rule does two things.

1. It runs the state flush (need S8), with a 20 second limit.
2. It prints a short instruction for the summariser on stdout, which Claude Code uses as instructions for
   the compaction summary. The instruction depends on who is compacting. For the main session (empty
   `agent_type`): keep the project root, the current phase, the in-flight item and the exact next command,
   and note that a fresh briefing will follow. For a subagent or workflow step, which receives no briefing:
   keep word for word the step identity block, the worktree path, the story id, the closing gate command and
   the findings or claims the step was given. The workflow prompt also repeats the identity and the closing
   command at its end, so they survive even if this event does not fire for a step.

It never blocks compaction. If the flush fails, the event `compaction.flush_failed` is logged and the next
briefing says so. `PostCompact` is not wired: the briefing after compaction arrives through `SessionStart`
with source `compact`, which also logs `compaction.done`.

#### 2.11.10 The remaining rules

`gate-guard` protects the gates themselves. It blocks: a write under the installed plugin directory (only
when that directory lies outside the project, so developing the plugin in its own repository still works);
a write to the runner's own directory; a write to a Claude Code settings file whose result sets
`disableAllHooks`, disables or removes this plugin, or sets any `VSDD_*` environment variable (which is how
a tool call could otherwise switch the hooks off for the next session); a Bash call of `claude plugin
disable`, `claude plugin uninstall` or `claude config`; a call of a tool subcommand on the person-only
list; a call of a tool subcommand (or a flag such as `status --fix` or `--include-walled`) in a group the
caller's row does not grant; a Bash command that assigns a `VSDD_*` variable itself; and a Bash call that
reaches the tool's Python other than through `bin/vsdd`. There is no canary command. The project
configuration and the evidence, decision and step-record stores are covered by `write-scopes`, because
their classes cannot be written by a tool call.

Caller markers and hook liveness. A `vsdd` process cannot see the hook payload, so it cannot know which
session or which agent called it. Before the runner lets a `vsdd` Bash call through, it does two things. It
writes a caller marker, `.factory/local/hooks/calls/<key>.json`, holding the payload's `session_id`,
`agent_id`, `agent_type` and `cwd` and the time; the key is random and used once. And it returns
`updatedInput` for the call, with the command rewritten to begin `VSDD_CALLER=<key> `, so the tool can find
its own marker and no other. The first pass wrote one marker per session, which a `vsdd` process had no way
to pick out: with two sessions in one project, which the state design supports, the survey of a session
whose hooks were off passed on the other session's marker, and the lock could be recorded under the wrong
session and released when that other session ended. With a marker per call:

- `vsdd survey` refuses to start a run when its own call has no marker, so a workflow cannot run with the
  hooks disabled at the platform level, with the plugin's hooks switched off, or with
  `VSDD_FACTORY_HOOKS=off`, whatever another session is doing.
- The lock records the session that actually called.
- `vsdd spike open` learns the runner's `agent_id`, so parallel runners cannot be swapped.
- `vsdd env start` records which agent started a product, so it can be stopped when that agent stops.
- `vsdd doctor`, run through the Bash tool, finds its marker and reports the hooks as live; run in a
  terminal it says the hooks cannot be checked from there.

A marker is deleted when the tool reads it, and unread markers older than ten minutes are pruned. The
rewrite is one more thing `auto-allow` and `tool_only` must accept: the runner adds the prefix after every
rule has judged the command as the caller typed it. Two things are not yet proven (blueprint section 10,
items 27 and 28): that `updatedInput` is honoured for a Bash call inside a workflow step, and that calls
inside a step carry the main session's `session_id`. If the first fails, the marker is keyed by the Claude
Code process that both the hook and the tool descend from, and matched to an agent by `cwd`, which is unique
for every isolated agent (item 29). For the second, the lock's liveness does not depend on the session id
alone (below).

`lifecycle-log` records `agent.start` and `agent.stop` for factory agents and workflow steps, and
`session.end`. It also keeps the run lock alive: on any guarded call from a session that holds the lock, or
from a working directory inside a worktree that the locked run owns, it refreshes the lock's `seen` time
through the state design's `lock_seen`, at most once a minute, and on `SessionEnd` it releases a lock that
session holds through `lock_release`. On `SubagentStop` it stops any product process that agent started
with `vsdd env start` and left running; the checkpoint stops whatever is left. It records no commit or
pull-request event: the tool writes `pr.created` and `pr.merged` when it delivers a story, and the gates
write `commit.made` for the commit they measure. On `SessionEnd` it also deletes event files older than the
retention period. Claude Code gives plugin `SessionEnd` hooks 1.5 seconds, which
a directory listing fits easily.

`purity-advice` ports the old warning. For files on the configured pure-core paths (default: `*/pure/**`,
`*/core/**`, `*/kernel/**`, `*_pure.rs`, `*.pure.ts`) it advises when the new content contains one of the
configured side-effect patterns (default: the old list).

`auto-allow` has two parts. The built-in part is always on and is narrow: a `Read` of a file under the
plugin's `methods/`, `templates/` or `assets/` directory, and a `Bash` command that is exactly one call of
the `vsdd` command (`vsdd ...` or `vsdd -C <dir> ...`) with no pipe, redirect or second segment, whose
subcommand the caller's row grants. For the main session only read-only subcommands are permitted this
way; anything that changes state still asks. Tool commands take their inputs as arguments or as a file
under the temporary directory, never on standard input, so a clerk's command never needs a redirect. Free
text travels percent-encoded in an argument, and the spike commands take files the runner writes. It exists
so that an agent is never stopped by a permission prompt from reading its own procedure or calling the
plugin's own command (`agents-skills.md` N-HOOK-6).

The second part is the project's answer to decision D4, which the user adopted: the plugin may approve its
own routine tool calls so that unattended runs do not stop on permission prompts. It is the one mechanism
for that; static permission rules in a settings file are only the fallback if a platform change breaks the
hook permit, because static rules cannot be scoped by agent role. The answer is `[hooks.rules.auto-allow]
scope` in `factory.toml`. `setup` asks the person before it runs `vsdd init`, and passes the answer as
`vsdd init --auto-allow <scope>`, because `init` is the one moment the plugin writes that file; on a
project that is already initialised, `setup` shows the two lines to add. The rule returns `Permit` only
when every other rule returned nothing, so a call a wall or a gate would block is never approved. The
platform probe of 2026-11-23 proved the permit in the default permission mode, in the main session, a
dispatched agent and a workflow step.

| Scope | What is approved |
|---|---|
| `off` | Only the built-in part above. |
| `core` | What decision D4 lists, made to work unattended. Edits inside story and spike worktrees and the state directory by an agent whose row may write them. The project's configured commands (`[commands]` and any `[[component]]` commands), matched as a prefix: the configured string followed by further arguments, so that a builder running one test by name is approved, with no pipe to a shell and no redirect outside the worktree. `git add`, `commit`, `status`, `diff` and `log` inside those worktrees. A `vsdd` call the caller's row grants, including for the main session `vsdd driver`, `vsdd checkpoint brief` and `vsdd note`, which the driver and the brief skill run. A `Workflow` call by the main session that starts one of this plugin's workflows (`vsdd-factory:*`), so the driver is not stopped at each workflow boundary. |
| `extended` | `core`, and what a real unattended run also does. A fixed list of read-only commands inside the caller's own worktree: `ls`, `cat`, `head`, `tail`, `wc`, `rg`, `grep`, `find` without `-exec`, `-delete` or `-ok`, and the `git-read` family. A `curl`, `wget` or browser call to the loopback port of a product that `vsdd env start` started for the run, by a row that holds the `env` group. Any Bash command by the spike runner whose working directory and every path lie inside its own spike worktree, and which `bash-destructive`, `bash-git` and `secrets` pass: this is what lets an agent-requested experiment run without a person. `WebFetch`, `WebSearch` and the research tools for the rows whose tool list has them, still under `secrets`. |

`extended` goes beyond the list in decision D4, so the plugin never turns it on by itself: `setup` describes
both scopes and the person chooses. Two second-pass reviewers showed that with `core` alone a wave in the
default permission mode still raises prompts for builders' read-only commands and for every spike
experiment. Without any scope, plugin agents (which cannot set a permission mode) prompt for every such call
and an unattended run stalls. The smoke tier counts prompts in the default permission mode for one builder
step and one experiment spike, and the greenfield proof counts them for a wave; with `extended` both expect
none. One observation from the probe belongs here: in the `auto` permission mode, which the user's own
sessions use, the platform allowed the unapproved control call as well, so there the rule changes little,
and it matters most for the default mode and for scheduled headless runs. Whether the permit is honoured for
the `Workflow` tool is blueprint section 10, item 30; if it is not, `setup` writes one static allow rule for
this plugin's workflows.

### 2.12 The event log, and explaining a block

Location: daily files, one JSON object per line, in the directory the project configuration names (working
name `.factory/local/events/<UTC date>.jsonl`; `VSDD_EVENTS_DIR` overrides it for tests). `vsdd/events.py`
is the one writer, used by the runner and by the tool, in process. There is no `vsdd events emit`: no 1.0
agent, skill or workflow used it, and the only reason for the first pass's list of reserved event types was
to protect the log from that command. With no shell entry point, only the runner and the tool can write the
log, whose directory is in a class only code writes, so the quarantine switch can trust the `rule.error`
events it reads. Each line is one `write` on a file opened
with `O_APPEND`, so concurrent agents do not interleave. Lines are capped at 4 KB. Calls from worktrees log
to the main project's file, because the root is resolved to the main checkout.

```json
{"v":1,"ts":"2026-11-23T21:40:03.512Z","type":"rule.block","severity":"block","id":"b-7f3a2c",
 "session_id":"0c1d…","agent_type":"vsdd-factory:implementer","agent_id":"agent-41",
 "event":"PreToolUse","tool":"Edit","rule":"tests-first","code":"no_red_evidence","story":"S-1.02",
 "target":".worktrees/S-1.02/src/lib.rs","reason":"…","fix":"…","ms":41}
```

Every line has `v`, `ts` (ISO 8601, UTC), `type` (dotted), `severity` (`info`, `warn`, `block`, `error`) and
`session_id`. Lines about a tool call add `agent_type`, `agent_id`, `event`, `tool` and `ms`. These are the
field names `packaging.md` assumes in its need H3.

| Type | Written when | Extra fields |
|---|---|---|
| `session.start`, `session.end` | Session start and end | `source` or `reason` |
| `agent.start`, `agent.stop` | A factory agent or workflow step starts or stops | `agent_type`, `agent_id`, `cwd` |
| `rule.block`, `rule.advise`, `rule.would_block` | A rule returns `Block` or `Advise`, or would have blocked at `advise` strength | `id`, `rule`, `code`, `reason`, `fix`, `target` or `command` |
| `rule.error`, `rule.timeout` | A rule crashed or ran out of time | `rule`, `error`, `fail` |
| `runner.launch_failed`, `runner.disabled`, `runner.slow` | The launcher could not start the runner; the emergency switch is set; a call took over 300 ms | `code`, `mode`, `ms` |
| `config.invalid`, `config.changed`, `policy.invalid` | Configuration faults and changes | `detail` |
| `decision.recorded`, `decision.refused` | A person typed the decide command | `decision`, `choice` |
| `step.recorded`, `step.rejected` | A workflow step's result was stored, or sent back for ungrounded findings | `run`, `workflow`, `subject`, `name`, `attempt` |
| `spike.start`, `spike.timeout`, `spike.stop_unrecorded` | A spike runner was first seen; its time box ended; it stopped without a finding | `agent_id`, `worktree` |
| `compaction.start`, `compaction.done`, `compaction.flush_failed` | Around a compaction | `trigger` |
| `commit.made`, `pr.created`, `pr.merged` | Written by the tool, not by a hook: a gate measured a commit (`commit.made`, with the head it measured); `vsdd story deliver` opened or merged a pull request | `head`, `branch`, `story`, `url` |
| `web.call` | A web tool call, or a web address in a shell command of an agent row | `host`, `tool` |

The tool writes its own events (gates run, evidence recorded, decisions opened) through the same writer;
their types belong to the tool's design.

Allowed calls are not logged one by one; that would be most of the file and nobody reads it. Generic tool
and cost figures come from Claude Code's own telemetry. Setting `VSDD_FACTORY_HOOKS_DEBUG=1` logs every call
as `runner.call` with its duration, for diagnosis.

Hygiene: file contents are never logged. Commands are cut to 200 characters and passed through a redactor
that masks `NAME=value` pairs with secret-shaped names and common token formats. Files older than
`log_retention_days` (default 30) are deleted at session end. If a day's file passes 20 MB, only blocks and
errors are written for the rest of the day.

Explaining a block. Every block has an id that appears in the message the model sees and in the log.

```
$ vsdd explain b-7f3a2c
Blocked: Edit of .worktrees/S-1.02/src/lib.rs by vsdd-factory:implementer at 21:40:03
Rule:    tests-first (gate, can be relaxed in [hooks.rules.tests-first])
Why:     no failing test run is recorded for story S-1.02
Fix:     write the failing test, then run the red gate: vsdd gate red --story S-1.02
Looked at: evidence store, gate "red", subject S-1.02 (no passing record); paths.source = ["src/**"]
```

`vsdd explain --last` shows the most recent block of the current project. `vsdd events schema` prints the
JSON Schema that the writer validates against, so the documented schema cannot drift from the code.

### 2.13 Command-line entry points

`packaging.md` defines one command, `vsdd`, with one module per subcommand. The hook subsystem contributes
these subcommands; the logic is in the modules of section 2.4 and the argument parsing sits with the rest of
`vsdd/cli/`.

| Subcommand | Purpose | Used by |
|---|---|---|
| `vsdd explain <id>`, `vsdd explain --last` | Explain a block | People, the status skill |
| `vsdd rules [--json]` | The rule table: id, events, tools, kind, one-line summary, current strength in this project | People; reference generation and the meta-test (`packaging.md` H6) |
| `vsdd events schema` | Print the event log's JSON Schema | Observability stack, reference generation |
| `vsdd hooks selftest` | Run every gate's blocking case through the installed launcher and confirm exit 2 | `vsdd doctor`, CI |

There is no `vsdd hooks canary` and no exit code 3. The first pass had a command that `gate-guard` always
blocked, so that `vsdd doctor` could tell whether hooks were live. `vsdd doctor`, run through the Bash tool,
now looks for the caller marker of its own call, exactly as the survey does, which answers the same question
with no extra command and no special branch in `gate-guard`.

`vsdd validate` and `vsdd doctor` are defined in `packaging.md`. The first uses the `artifacts` module. The
second should run `vsdd hooks selftest` and the exact launch command of section 2.2, instead of comparing uv
version numbers.

### 2.14 How the rules are tested

Tests are pytest, in `tests/hooks/` (`packaging.md` I9). The runner stays stdlib-only; pytest is a
development dependency only. `packaging.md` supplies the directories, the launcher test, the meta-test and
the matrix; this section supplies the formats it asks for (its need H7) and the cases.

A recorded payload is the JSON Claude Code sent on stdin, captured by setting
`VSDD_FACTORY_HOOKS_CAPTURE=<dir>`, with three substitutions so it can be replayed anywhere: the project
root becomes `{root}`, the home directory `{home}`, the plugin root `{plugin}`. It is stored as
`tests/hooks/payloads/<Event>/<name>.json`. `tests/hooks/payloads/_meta.json` records the Claude Code
version and the capture date. The set must contain every wired event and tool; the main session, a plugin
agent, a workflow step and a worktree-isolated agent; a `StructuredOutput` call from a workflow step, with
and without findings; a typed `/vsdd-factory:decide` command; a `SubagentStop` for a workflow step; each
`SessionStart` source; manual and automatic `PreCompact`.

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

| Layer | What it does |
|---|---|
| Rule cases | Each case runs in-process (rule function, payload, temporary project, outcome and reason code). Every rule has cases that must block and cases that must allow. The cases of the old `destructive-guard.bats`, `protect-secrets.bats` and `hooks.bats` are harvested into these tables. |
| Launcher cases | Cases marked `launcher = true` also run `sh py/launch.sh hook <mode>` as a subprocess and assert on the exit code, stdout and stderr, which is exactly what Claude Code sees. The meta-test fails if any gate lacks one that blocks and one that allows. These run under both `dash` and `bash`. |
| Wiring tests | `hooks.json` equals the generated file; it has the top-level `hooks` key; every timeout is at most 60; no handler with a gate rule is `async`; every rule's event and tool is reachable through a matcher; every wired event has a rule. |
| Failure tests | `PATH` without uv; a runner that raises on import; a rule that raises; a rule that sleeps past the deadline; `UV_PYTHON=3.9` in the environment; an unwritable log directory; invalid configuration; a missing policy table. Each asserts the row of the table in section 2.9. |
| Inert tests | In a directory with no configuration file: exit 0, empty stdout and stderr, no file created, and uv never invoked (a stub `uv` on `PATH` records calls). In a directory with only pre-1.0 state: the same, plus the one fixed line on `SessionStart`. |
| Location tests | The whole suite runs with the plugin copied to a temporary directory outside the repository, so no rule can depend on a path that exists only in this repository. |
| Policy tests | Every agent file has a row in `walls.toml` and the reverse; every class named in a row exists in `classes.toml`; every walled class matches at least one path in the sample project. |
| Performance test | The median `gate` call stays under 120 ms. |
| Live probes | In the end-to-end tier of `packaging.md`: one call that each of `bash-destructive`, `read-walls` and `tests-first` must block, and one briefing check, asserted from the event log. The same run re-captures the payloads, so a change in Claude Code's payload shape shows up as a diff. |

### 2.15 Old defects and what prevents each one here

| Old defect | Prevention |
|---|---|
| The dispatcher never wrote stdout, so `protect-bc` and `protect-vp`, which denied through JSON with exit 0, never blocked. | One module (`emit.py`) produces all hook output. Blocks use exit 2 and stderr only. Each gate has a launcher case that asserts exit 2. |
| The hook timeout of 10000 was read as seconds. | Timeouts are generated in seconds; a test fails above 60. |
| `PreCompact` and `PostCompact` were registered but not wired; `PermissionRequest` was wired with nothing registered. | `hooks.json` is generated from the rule table and tested in both directions. |
| `hooks.json` was git-ignored and created by an activation step from five platform variants. | One checked-in file. No activation. |
| Nearly every bash hook began "if `jq` is missing, exit 0", and two exited 0 without `python3`. Missing tools switched gates off silently. | No optional dependencies. Missing uv or Python blocks and says so. |
| A clean exit 1 from a plugin never blocked, even with `on_error = block`. | The launcher maps every exit code other than 0 and 10 to a block on gate events. |
| Validators ran after the write and "blocked" what had already landed; a marker file and two more hooks covered the case where a validator timed out. | Validation runs before the write on the content the file would have. |
| `validate-artifact-path` read its table from a path that existed only in this repository. | Policy tables are resolved from the plugin directory; the suite runs from a copy outside the repository. |
| Hooks found the story and the project by scraping the dispatch prompt. | Rules use the payload's `cwd` and `agent_type`, git facts and evidence records. |
| Whole-string matching blocked commit messages that mentioned a dangerous command and missed `git commit -F`. | Commands are parsed into segments; message files are read. |
| Advisory hooks used an "advisory block" pattern (print `outcome: block`, return continue), and async hooks' blocks were discarded. | Three explicit kinds. The generator refuses `async` on a gate. |
| A crashed bash hook (`convergence-tracker`) blocked edits to unrelated files; large files exhausted WASM fuel. | A crashing rule names itself in the block and does not hide the other rules' results; artifact size caps apply before the write; there is no fuel. |
| The registry header and the ABI document were out of date. | The rule table is the documentation source: `vsdd rules` prints it and the reference page is generated from it. |
| Bash hooks logged through a separate `emit-event` tool into a second log. | One log, one writer. |

### 2.16 Platform facts this design relies on

| Fact | Status | If it turns out false |
|---|---|---|
| Exit 2 from a uv-launched Python hook blocks a tool call and the model sees stderr | Proven by the spike | n/a |
| `agent_type` is in the payload for calls inside a subagent and empty in the main session | Proven by the spike | n/a |
| `SessionStart` `additionalContext` reaches the model on startup, resume and compact | Proven by the spike | n/a |
| `PreCompact` fires; hooks fire for the `Workflow` and `StructuredOutput` tools | Proven by the spike | n/a |
| `onFailure: "block"`; `PreToolUse` `additionalContext`; `PreCompact` stdout becomes summary instructions; plugin `SessionEnd` budget of 1.5 s; a `WorktreeCreate` hook replaces worktree creation; `UserPromptSubmit` also fires for prompts no person typed | Hooks documentation, read 2026-11-23 | The launcher blocks without `onFailure`, and the self-guarding command covers missing plugin files; `onFailure` is not relied on for either. Advice would move to `systemMessage`. The compaction instruction is a bonus. |
| `UserPromptExpansion` fires for `/vsdd-factory:decide`, carries `command_name` and `command_args`, can add context, and a block ends the prompt before any model reads it | Proven by the probe of 2026-11-23, headless | n/a |
| `PreToolUse` on `StructuredOutput` carries the whole result in `tool_input`, and a block makes the agent correct the result and call again | Proven by the probe of 2026-11-23 | n/a. If a later release changes it: section 2.11.7. |
| Exit 2 on `SubagentStop` keeps an agent running | Probed 2026-11-23: it holds an agent that returns text; it does not hold a workflow step that has already returned a schema result | The design follows the observation: a workflow step is held by blocking its `StructuredOutput` call, a dispatched agent by refusing its stop (section 2.11.6). |
| A hook's `permissionDecision: "allow"` is honoured in the main session, a dispatched agent and a workflow step | Proven by the probe of 2026-11-23 in the default permission mode. In the `auto` mode the unapproved control call was allowed as well | Static allow rules written by `setup`, which cannot be scoped by role. |
| A hook's `updatedInput` rewrites a Bash command, in every context | Hooks documentation; not verified (blueprint section 10, item 27) | The caller marker is keyed by the Claude Code process and matched by working directory (section 2.11.10). |
| Tool calls inside a workflow step carry the main session's `session_id` | Not verified (item 28) | The lock is also kept alive by calls from a worktree the run owns. |
| The permit is honoured for the `Workflow` tool | Not verified (item 30) | One static allow rule for this plugin's workflows. |
| The payload `cwd` of a worktree-isolated agent is its worktree | Proven by the spike | n/a |
| `Grep` skips a directory listed in `.git/info/exclude` | Proven by the probe of 2026-11-23 | n/a. If a later release changes it, ancestor searches are blocked for walled callers (section 2.11.2). |
| `isolation: worktree` in an agent file | Probed 2026-11-23: honoured for a dispatch with the Agent tool, ignored for a workflow step | Every workflow step that needs isolation passes it itself, which the spike wrapper does. |

The first build task for this subsystem is to capture payloads for the rows still unverified.

### 2.17 Where this design departs from the readings in STATE.md

`rewrite/STATE.md` marks some items as the assistant's reading. I changed these:

- "Guard the hook command so a missing uv fails loudly" is widened to every launch failure, because a
  Python crash (exit 1) and a `UV_PYTHON` override are equally silent.
- "The hook runner sees every spike start, so limits can be enforced in code" needed a mechanism, since
  `SubagentStart` cannot block. Count limits are enforced by the tool's `spike open` command, as
  `pipeline.md` designs it, and the hook enforces at the spike runner's tool calls that no work happens
  without an open ledger entry or after the time box.
- "Tests-before-code must not block work inside a spike worktree" is generalised to a per-rule
  `spike_exempt` flag with a precise definition of "inside a spike", and paired with containment.
- The old registry's post-compaction hook is not ported as a `PostCompact` handler; `SessionStart` with
  source `compact` already delivers the briefing.
- The briefing's "initialised or not" is read as partial versus ready; an uninitialised project gets silence.

## 3. Mapping from the old system

### 3.1 All 79 registry entries

Totals: 8 ported, 21 merged into a 1.0 rule, 21 replaced (11 by a workflow gate, 6 by the state design, 4
by another source), 29 dropped. "Workflow gate" includes the gates the pipeline's command-line tool measures
on a workflow's behalf.

"Bookkeeping" below means the hook policed a format of the old pipeline's own records (burst log, decision
log, trajectory tail, index citations, hand-kept counts and version pins) that does not exist in 1.0.

| # | Old entry (event; tool) | Outcome | Where it went, or why it was dropped |
|---|---|---|---|
| 1 | `session-start-telemetry` (SessionStart) | Merged | `session-briefing` logs `session.start`. |
| 2 | `session-end-telemetry` (SessionEnd) | Merged | `lifecycle-log` logs `session.end`. |
| 3 | `worktree-hooks` (WorktreeCreate) | Dropped | A command hook on this event replaces Claude Code's own worktree creation. Worktree use is visible in `agent.start` (`cwd`). |
| 4 | `worktree-hooks` (WorktreeRemove) | Dropped | Fires only for worktrees a `WorktreeCreate` hook made; no consumer. |
| 5 | `tool-failure-hooks` (PostToolUseFailure) | Replaced, other source | Claude Code's own telemetry reports tool results. |
| 6 | `capture-commit-activity` (PostToolUse; all) | Replaced, other source | The gates write `commit.made` for the commit they measure. No hook is wired on `PostToolUse`; the old one ran on every tool call. |
| 7 | `capture-pr-activity` (PostToolUse; all) | Replaced, other source | The tool writes `pr.created` and `pr.merged` when it delivers a story. No pipeline agent opens a pull request. |
| 8 | `convergence-tracker` (PostToolUse; edits) | Replaced, workflow gate | Convergence is computed in the workflow script from structured verdicts. The old hook was async and could not block. |
| 9 | `purity-check` (PostToolUse; edits) | Ported | `purity-advice`, now before the write. |
| 10 | `regression-gate` (PostToolUse; all) | Replaced, other source | The tool's gate command writes evidence from the real exit code; delivery requires green evidence for the head commit. |
| 11 | `validate-anchor-capabilities-union` | Dropped | Policed a stored field that is a function of other fields. Derived data is computed, not stored. |
| 12 | `validate-bc-title` | Dropped | Policed a title duplicated between a file and a hand-kept index. |
| 13 | `validate-changelog-monotonicity` | Dropped | Policed hand-kept changelog tables; history is in git. |
| 14 | `validate-demo-evidence-story-scoped` | Merged | `artifacts`: the evidence kind's path glob requires the story directory. |
| 15 | `validate-factory-path-root` | Merged | `write-scopes` check 5, before the write. |
| 16 | `validate-finding-format` | Replaced, workflow gate | Findings are not stored as files. Their shape is `finding.schema.json`, validated by the workflow runtime when a step returns and again by the tool before a record counts toward a verdict. |
| 17 | `validate-index-self-reference` | Dropped | Bookkeeping. |
| 18 | `validate-input-hash` | Replaced, state design | Staleness is reported by `vsdd stale` at a gate, not policed on every write. |
| 19 | `validate-novelty-assessment` | Replaced, workflow gate | The review result schema carries those fields and the workflow runtime validates it. |
| 20 | `validate-pr-description-completeness` | Replaced, other source | The pull request description is not an artifact: `vsdd story deliver` renders it from a fixed template, and a unit test asserts the template's required sections. |
| 21 | `validate-state-index-status-coherence` | Dropped | Bookkeeping: one status kept in two files. |
| 22 | `validate-state-pin-freshness` | Dropped | Bookkeeping: version pins. |
| 23 | `validate-state-size` | Merged | `artifacts` size caps, before the write. |
| 24 | `validate-story-bc-sync` | Merged | `artifacts` reference check. The duplicated body table is gone. |
| 25 | `validate-subsystem-names` | Merged | `artifacts` reference check against the subsystem registry. |
| 26 | `validate-table-cell-count` | Merged | `artifacts` table check. |
| 27 | `validate-template-compliance` | Ported | The core of `artifacts`, driven by one table instead of 136 templates. |
| 28 | `validate-vp-consistency` | Dropped | Policed three hand-synchronised copies of the verification property list. |
| 29 | `validate-wave-gate-completeness` | Replaced, workflow gate | The wave gate is measured by the tool, which writes the evidence and refuses to close a wave without it. |
| 30 | `validate-count-propagation` | Dropped | Bookkeeping: counts repeated across documents. |
| 31 | `validate-red-ratio` | Replaced, workflow gate | The ratio is a condition over the test writer's structured result; the hook side is the red evidence in `tests-first`. |
| 32 | `validate-wave-handoff-completeness` | Dropped | The handoff file is gone; resume reads the closure and decision records. |
| 33 | `validate-cross-site-correspondence` | Dropped | Bookkeeping: version citations, `last_amended` parity, three-way hash equality. |
| 34 | `stamp-state-timestamp` | Dropped | No hook mutates pipeline state in 1.0. |
| 35 | `block-ai-attribution` (PreToolUse; Bash) | Merged | `bash-git`. |
| 36 | `brownfield-discipline` (PreToolUse; edits) | Merged | `write-scopes`: class `reference` with `write = "none"`. |
| 37 | `check-factory-commit` (PreToolUse; Bash) | Dropped | A reminder to update the old state file; it always exited 0 and was registered on the wrong event. |
| 38 | `destructive-command-guard` (PreToolUse; Bash) | Ported | `bash-destructive`. |
| 39 | `factory-branch-guard` (PreToolUse; edits) | Replaced, state design | Meaningful only if state stays on a separate branch mounted as a worktree. If it does, it becomes a precondition in `write-scopes` (need S9). |
| 40 | `protect-bc` (PreToolUse; edits) | Merged | `artifacts` frozen status. It now blocks. |
| 41 | `protect-secrets` (PreToolUse; Bash) | Ported | `secrets`. |
| 42 | `protect-secrets` (PreToolUse; Read) | Ported | `secrets`. |
| 43 | `protect-vp` (PreToolUse; edits) | Merged | `artifacts` frozen status. It now blocks. |
| 44 | `red-gate` (PreToolUse; edits) | Ported | `tests-first`, on code-written evidence. |
| 45 | `track-agent-start` (PreToolUse; Agent) | Merged | `lifecycle-log` on `SubagentStart`, which also sees workflow steps. |
| 46 | `validate-pr-merge-prerequisites` (PreToolUse; Agent) | Ported | `merge-guard` forces the governed route, and the evidence check runs in code inside the tool, instead of on a dispatch prompt. |
| 47 | `validate-wave-gate-prerequisite` (PreToolUse; Agent) | Replaced, workflow gate | The tool creates a story's worktree only when the story is ready, and `write-scopes` keeps builders inside story worktrees (section 2.11.4). Its second arm, a check of commit-hash citations before adversary dispatch, is bookkeeping and is dropped. |
| 48 | `verify-git-push` (PreToolUse; Bash) | Merged | `bash-git`. |
| 49 | `validate-factory-path-staging` (PreToolUse; Bash) | Replaced, state design | Same condition as entry 39. |
| 50 | `validate-artifact-path` (PreToolUse; edits) | Merged | `artifacts` unknown-path check, with the table shipped in the plugin. |
| 51 | `validate-burst-log` (PostToolUse; edits) | Dropped | Bookkeeping. |
| 52 | `validate-burst-log` (PostToolUse; Bash) | Dropped | Bookkeeping. |
| 53 | `validate-index-cite-refresh` | Dropped | Bookkeeping; hard-coded one cycle name. |
| 54 | `validate-state-structure` | Dropped | Bookkeeping. |
| 55 | `validate-dispatch-advance` (PostToolUse; edits) | Dropped | Bookkeeping. |
| 56 | `validate-dispatch-advance` (PostToolUse; Bash) | Dropped | Bookkeeping. |
| 57 | `validate-closes-completeness` | Dropped | Bookkeeping. |
| 58 | `validate-policies-schema` | Dropped | The policy registry file is gone. Hook settings are validated by `config.py`. |
| 59 | `validate-trajectory-tail-cell-completeness` | Dropped | Bookkeeping. |
| 60 | `session-learning` (Stop) | Dropped | It appended a marker line at every stop and captured nothing. Lessons are an artifact written by a workflow step. |
| 61 | `warn-pending-wave-gate` (Stop) | Merged | The briefing shows pending gates and open decisions. |
| 62 | `handoff-validator` (SubagentStop) | Replaced, workflow gate | Every workflow step returns a schema-validated result, so an empty or truncated result cannot pass; `step-results` stores it. |
| 63 | `pr-manager-completion-guard` (SubagentStop) | Replaced, workflow gate | The pull request lifecycle is workflow steps in code order. |
| 64 | `track-agent-stop` (SubagentStop) | Merged | `lifecycle-log`. |
| 65 | `update-wave-state-on-merge` (SubagentStop) | Replaced, workflow gate | State is written at the step boundary by the workflow. |
| 66 | `validate-pr-review-posted` (SubagentStop) | Replaced, workflow gate | The review step's result, plus the governed merge requiring the verdict record. |
| 67 | `validate-per-story-adversary-convergence` (SubagentStop) | Replaced, workflow gate | The convergence rule in the story workflow, plus the governed merge. |
| 68 | `validate-stable-anchors` (PreToolUse; edits) | Merged | `artifacts` forbidden-pattern check. |
| 69 | `lint-registry-async-invariant` | Dropped | Linted the old registry file. The generated `hooks.json` test covers the same invariant. |
| 70 | `validate-factory-path-staged` (PostToolUse; Bash) | Replaced, state design | Same condition as entry 39. |
| 71 | `verify-factory-lock` (PreToolUse; edits, Agent) | Replaced, state design | If the state design keeps a lock, its own code takes it when it writes; no hook rewrites a timestamp on every edit. The briefing reports another live session from the event log. |
| 72 | `verify-factory-lock-bash` (PreToolUse; Bash) | Replaced, state design | Same. |
| 73 | `check-harness-version` (PreCompact) | Replaced, other source | The minimum Claude Code version in `plugin.json`, and `vsdd doctor`. |
| 74 | `precompact-flush` (PreCompact) | Ported | `compaction`, wired for the first time. |
| 75 | `postcompact-reanchor` (PostCompact) | Merged | `session-briefing` on source `compact`. |
| 76 | `validate-heavy-op-delegation` (PreToolUse; Bash) | Dropped | It advised the main session to delegate heavy commands. In 1.0 the main session does not run pipeline work. |
| 77 | `validate-unvalidated-mutation-marker` (PreToolUse; Agent) | Dropped | It existed because validators ran after the write and could time out. Validation now runs before the write and fails closed. |
| 78 | `validate-unvalidated-mutation-marker-git` (PreToolUse; Bash) | Dropped | Same. |
| 79 | `verify-state-timestamp-advisory` (PostToolUse; Bash) | Dropped | Bookkeeping. |

### 3.2 The old runner and its supporting pieces

| Old item | Outcome | Reason |
|---|---|---|
| `factory-dispatcher` (Rust, about 84,000 lines with tests), wasmtime, fuel and epoch limits | Replaced | `launch.sh` and `vsdd/hooks/main.py`. The sandbox did not confine the filesystem, and fuel exhaustion was a recurring failure. |
| `hooks-registry.toml` (1,708 lines), priorities, tiers, capabilities | Replaced | `registry.py`, an explicit table. Rules run in table order; there are no capabilities because rules are trusted plugin code. |
| `hooks.json.template`, five platform variants, `generate-hooks-json.sh`, `/vsdd-factory:activate` | Replaced | One generated, checked-in `hooks.json`. |
| Five prebuilt dispatcher binaries (about 82 MB) and 39 `.wasm` files (about 14 MB) in git | Dropped | No binaries. |
| `legacy-bash-adapter` and the 36 bash hooks | Replaced | Python rules. |
| `hook-sdk`, `hook-sdk-macros`, `HOST_ABI.md` | Dropped | The rule model in section 2.5 is the whole contract. |
| `vsdd-context-resolvers` (`wave_context`) | Dropped | No rule needs wave state injected; rules read the evidence store. |
| The six `sink-*` crates | Dropped | Never wired. The event log is a file; shipping it elsewhere belongs to the observability stack. |
| `dispatcher-internal-*.jsonl`, `VSDD_SINK_FILE`, `bin/emit-event` and its second log | Replaced | The one event log in section 2.12. |
| Unvalidated-mutation marker (ADR-047, ADR-048) | Dropped | Not needed when validation precedes the write. |
| Shard manager and `migrate-bc-index` (16,500 dormant lines) | Dropped | Never activated; index sharding is a state-design question. |
| `factory-lock`, `factory-lock-parse` crates | Replaced | See entries 71 and 72. |
| `hooks/lib/block.sh` | Replaced | `emit.py`. The why, fix and code elements of the message are kept. |
| `hooks/dim2-gates/*` (11 scripts) | Dropped | Manual tools for the old burst-log attestations. |
| `precompact-flush-prune.sh`, `update-cargo-audit-cache.sh` | Dropped | Served a log and a cache that no longer exist. |
| `verify-state-timestamp-refresh`, `read-prefix-fixture` crates | Dropped | Deregistered and a test fixture. |
| About 173 dispatcher-driven bats files and 45 direct bash-hook bats files | Replaced | The pytest layers in section 2.14. The behaviour cases for the destructive, secrets and git guards are harvested into case tables. |
| `last-amended-migrate`, `policy15-attestation-gate` | Not hooks | Left to the packaging design's tool mapping. |

## 4. Interfaces I provide

### 4.1 What this subsystem offers

1. **The hook entry.** `<plugin>/hooks/hooks.json` as in section 2.3, and
   `sh "<plugin>/py/launch.sh" hook <gate|brief|notify>` with the exit-code contract of section 2.2. Nobody
   else adds entries to `hooks.json`; a new hook is a new rule in the rule table.
2. **The rule model** of section 2.5, for anyone who needs a new rule: one module, one `@rule` declaration,
   one row in `registry.py`, case tables.
3. **Project configuration keys** under `[hooks]` (section 2.6). I read, and do not own, the paths, limits
   and commands listed in need S3.
4. **Policy table formats** (section 2.7): `walls.toml` (with `[callers]` and `[reveals]`), `classes.toml`
   (with the product-class precedence), `artifacts.toml` (with the story kinds).
5. **Agent-keyed enforcement.** Every row of `walls.toml` gets its read walls, write scope, dispatch list
   and command families enforced on `Read`, `Grep`, `Glob`, `Edit`, `Write`, `MultiEdit`, `NotebookEdit`,
   `Agent` and, heuristically, `Bash`. The main session uses the `main` row and any unlisted type the
   `other` row.
6. **Step records and grounded findings.** Every `StructuredOutput` call that carries a step identity is
   stored as a step record before the call completes, and a review result whose finding cites a missing
   file, a line range outside the file or a quote that is not there is sent back to its author
   (section 2.11.7).
7. **Decision capture.** `/vsdd-factory:decide <id> <choice> [key=value ...] [-- note]`, typed by a person,
   settles the decision through the state design's function before any model reads the message. No tool
   call can run the `decide` subcommand or write the decision store (section 2.11.5).
8. **Evidence-backed guards.** Source in a story is unwritable before red evidence, frozen tests stay
   frozen, and the integration and release branches are reachable only through the tool's governed
   subcommands (section 2.11.4). The evidence itself is the tool's.
9. **Spike enforcement.** No work without an open ledger entry, the time box, no nesting, no finish without
   a recorded finding, the admission that records who asked, the asker's walls, containment, and the
   `spike_exempt` flag (section 2.11.6).
10. **Caller identity and the hook liveness check.** A caller marker under `.factory/local/hooks/calls/`,
    written for each `vsdd` call with the session, agent id, agent type and working directory, and the key
    added to the command. `vsdd survey` requires its own, as `vsdd doctor` does when it is run through the
    Bash tool. There is no canary command.
11. **Artifact validation as a module**, used by the `artifacts` rule before a write and by `vsdd validate`
    over the whole state directory. One finding per line: `<path>: <code>: <message>`.
12. **The event log**: the writer in `vsdd/events.py`, the line format and type list of section 2.12,
    schema version 1, `vsdd events schema`, and the `VSDD_EVENTS_DIR` override for tests. Allowed tool calls
    are not in it; generic tool and cost figures come from Claude Code's own telemetry.
13. **Block explanation**: the block id in every message, and `vsdd explain`.
14. **Session briefing delivery** on startup, resume, clear, compact and fork, with the state design's
    content and limit, and runner lines only when something is wrong or relaxed.
15. **Pre-compaction**: the state flush is run with a 20 second limit before every compaction, and the
    summariser is told what to keep. No snapshot file is written.
16. **`vsdd rules`** and **`vsdd hooks selftest`**.
17. **Environment switches**: `VSDD_FACTORY_HOOKS=off`, `VSDD_FACTORY_HOOKS_DEBUG=1`,
    `VSDD_FACTORY_HOOKS_CAPTURE=<dir>`, `VSDD_EVENTS_DIR=<dir>`.

### 4.2 Needs the three designs on disk placed on this one

| Their id | What they asked for | Where it is met |
|---|---|---|
| pipeline H1 | Store the step result, with the calling agent's type, when a `StructuredOutput` call carries `step.run`; pass other `StructuredOutput` calls through; store walled findings in the holdout class | `step-results` (2.11.7). `Workflow` reaches the runner for `auto-allow` only. |
| pipeline H2 | Reject a review result whose finding names a missing file, a bad line range or a quote not found | `step-results` grounding (2.11.7). The probe proved it. |
| pipeline H3 | Walls W1 to W6 by agent type | W1, W2, W3: `read_deny` rows over the `holdout`, step-record and other classes (2.7, 2.11.2). W4: `tests:frozen` (2.11.4). W5: `story_only` (2.11.2 check 6). W6: spike containment (2.11.6). |
| pipeline H4 | Guards G1 to G4 | G1: `tests-first`, exempting the kinds with no red gate (`facade`, `docs`, `chore`, `harness`). G2: `merge-guard`. G3: the whole-set specification lock in `artifacts`. G4: code-only classes in `write-scopes`. |
| pipeline H5 | Decision capture from a typed command; block `decide` for every agent | `decisions` and `gate-guard` (2.11.5). I use `UserPromptExpansion` first and `UserPromptSubmit` only as the fallback. |
| pipeline H6 | Spike rules: exemption, time box, no dispatch, no stop without a ledger record | `spike-bounds` (2.11.6). |
| pipeline H7 | Resolve the governing project from the path being touched | `ctx.project` (2.5, 2.8). |
| pipeline H8 | A `clerk` may run only the tool | `tool_only` (2.7, 2.11.1). |
| agents N-HOOK-1 | Read walls on `Read`, `Grep`, `Glob` and Bash, including ancestor search roots; write scopes including create-only; fail closed | 2.11.2; `:create` in 2.7; 2.9. The main-session ancestor search is the one open point (2.11.2 note). |
| agents N-HOOK-2 | The dispatch table on `PreToolUse` for `Agent` | `dispatch` in 2.7, enforced by `read-walls`. |
| agents N-HOOK-3 | Spike runner: no dispatch without isolation; gates do not apply in a spike; no push | 2.11.6. Isolation is enforced by its effect: without a worktree of its own the spike runner can write nothing. |
| agents N-HOOK-4 | Tests-before-code honours the test-writer and implementer split | `:before-red` in 2.7; 2.11.4. |
| agents N-HOOK-5 | Shell writes into the state directory are blocked, except through the plugin command | `write-scopes` check 4. |
| agents N-HOOK-6 | No permission prompt for reading methods and templates or calling the plugin command | Built-in part of `auto-allow` (2.11.10). |
| agents N-HOOK-7 | A block message names the correct route | `route` in `classes.toml`; the `Instead:` line. |
| agents N-HOOK-8 | Holdout evaluator: no git, no command naming source, test or spec paths | `commands = []` (2.11.1) and `read-walls` on Bash. |
| packaging H1 | Runner as a standard-library package with one entry script, sharing the launcher with `bin/vsdd` | 2.2 and 2.4: `py/launch.sh`, `py/hook.py`, `py/vsdd/`. One change to its interface I3: the `hook` entry takes a mode and must not `exec` uv. |
| packaging H2 | The Python floor | 3.11. |
| packaging H3 | Event log path and fields; one writer; a fixed path or an override | 2.12. There is no `vsdd events emit`: the runner and the tool write in process. |
| packaging H4 | Inert rule, and pre-1.0 state counts as not initialised with one briefing line | 2.8, the Legacy condition. |
| packaging H5 | A rule that blocks a bare `gh pr merge` and names the governed command | `merge-guard` (2.11.4). |
| packaging H6 | A way to list rules | `vsdd rules --json`, and `registry.RULES` for the meta-test. |
| packaging H7 | The format of a recorded payload and of a rule case | 2.14. |

Needs the state design placed on this one (its section 5), which was written after this document, and
where the blueprint has them met:

| Their id | What they asked for | Where it is met |
|---|---|---|
| state H-1 | Flush on `SubagentStop` and `SessionEnd` | Withdrawn. Flush runs on `PreCompact` and inside `vsdd` commands only, which fits the one-second deadline of notify events. |
| state H-2 | Keep the run lock's `seen` fresh; release the lock at `SessionEnd`; let `vsdd` learn who called; stop products an agent left running | `lifecycle-log` and the caller marker (2.11.10). |
| state H-3 | Honour `dir` and the lock; name the next free identifier | `artifacts` (2.11.3). There is no `code_values` column to honour. |
| state H-9 | Store the agent type with each step record; record and pair spike admissions | `step-results` (2.11.7) and `spike-bounds` (2.11.6). |
| state H-4 | Block state writes while the mount test fails | `write-scopes` check 7 (2.11.2). |
| state H-5 | Block reads and writes of a `.factory` path inside a linked worktree | `write-scopes` check 5 and `read-walls` (2.11.2). |
| state H-6, H-7 | Step records only through `store_step`; decisions only through `settle_decision` | `step-results` and `decisions`. |
| agents N-HOOK-9 | A browser `file:` address is checked like a `Read` | The matcher (2.3) and `read-walls` (2.11.2). |

The three mismatches between those designs that touched this one are settled by the blueprint: the tool is
`vsdd` and the governed commands are `story deliver` and `release`; `clerk` and `scenario-reviewer`
are roster agents with rows; and the tool writes the spike ledger. They are kept below as written.

Three mismatches between those designs that touch this one, for the synthesis to settle:

- The tool is `vsdd` in `packaging.md` and `factory` in `pipeline.md`, and the governed merge is
  `vsdd pr-merge` in one and `factory story deliver` in the other. The rules read the subcommand names from
  `gates.toml`, so either answer is one line of data.
- `pipeline.md` names agent types (`clerk`, `spec-reviewer`) that the roster in `agents-skills.md` does not
  have. Every agent type a workflow uses needs a row in `walls.toml`; an unlisted type gets the `other` row,
  which can write nothing in the state directory.
- `agents-skills.md` has the spike runner write its own ledger entry; `pipeline.md` has the tool write it.
  The second fits code-only stores better, and the `spike-bounds` rule assumes it.

One correction to a test in `packaging.md`: a `StructuredOutput` call does reach the runner. It passes when
it carries no step identity.

## 5. Interfaces I need

From the state designer.

| Id | Need |
|---|---|
| S1 | The project configuration file is TOML, sits at a fixed path relative to the project root (I assume `.factory/factory.toml`), exists if and only if the project is initialised for 1.0, and has an integer `schema` key. The file test that recognises pre-1.0 state (I assume `.factory/STATE.md`). If either path differs, one constant in `find-root.sh` changes. |
| S2 | One git-ignored directory inside the state directory for the runner's own records (caller markers, spike start times and admissions, the configuration fingerprint), and the event log directory. Both must be in a path class with `write = "code"`. |
| S3 | These configuration keys: the seven product classes `paths.tests`, `paths.harness`, `paths.ci`, `paths.repo_config`, `paths.manifests`, `paths.docs`, `paths.source`, with their precedence (as `agents-skills.md` N-STATE-1); `hooks.main_session_wall` and `hooks.rules.auto-allow.scope`; `branches.integration` and `branches.release` (as `pipeline.md` S1); the spike time box in minutes; where the event log goes. The `[hooks]` table is reserved for this subsystem. |
| S4 | The briefing function, which computes the status from the decision and closure records each time (there is no stored status record) and returns the briefing lines, importable from `vsdd/state/`, returning well inside the eight-second briefing deadline, plus the briefing's length limit. |
| S5 | The content of `artifacts.toml` for every artifact kind (section 2.11.3 shows the columns), as the one copy that `vsdd validate` also reads, with frontmatter kept inside the supported subset (section 2.4). Confirmation that indexes, counts and input hashes are derived, which is what lets entries 11, 12, 18, 28 and 30 go. |
| S6 | The content of `classes.toml`: globs for every path class of `agents-skills.md` section 2.9, each class in the state directory being a directory no other class shares, plus classes for the step-record store (with earlier passes and builders' records separable, for `pipeline.md` wall W3), the evidence store, the decision store and the spike ledger. An answer to `agents-skills.md` N-STATE-8 for `holdout`. |
| S7 | The stores of `pipeline.md` S3 to S6 as importable functions in `vsdd/state/`, each returning within one second. This is the same list as `state.md` section 4, S7, and the two are built to one signature each: `store_step(project, result, agent_type, head)`, which also writes walled findings to the holdout class; `settle_decision(project, id, choice, values, note, session_id)`, which writes the decision record only and raises with a reason when it refuses; `evidence(project, gate, subject)`, returning the passing record or nothing, with the frozen test files listed in the `red` or re-freeze record; `spike_entry(project, agent_id, worktree)`, matched by agent id, returning the ledger entry with its `opened` time, `verdict`, `asker_type` and `read_deny`; `spike_admit(project, session_id, dispatcher_type)` and `spike_attach(project, agent_id, cwd)`; `story_kind(project, story)` and `kind_has_red_gate(kind)`; `spec_locked(project)`, true when the latest `spec` decision is approved and no live run holds an amendment, over the whole specification set; `artifact_locked(project, path)`, true for an accepted decision record, a proven property and a done story; `guidance_locked(project)`, true while a cycle is open and no live run holds an amendment, for the two guidance files; `unit_tree(project, workflow, subject)`, the root a unit's finding paths are relative to; `lock_seen` and `lock_release`; and `status(root)` and `briefing(root, session_id, source)`, computed from the records. |
| S8 | The pre-compaction flush as a function or a command, idempotent, at most 20 seconds, no network; or the statement that nothing needs flushing. |
| S9 | Whether the state directory is a single instance at the project root. `write-scopes` check 5 assumes it is. If state stays on a separate branch mounted as a worktree, the expected branch name, so that entries 39, 49 and 70 become a precondition. |

From the agent and skill designer.

| Id | Need |
|---|---|
| A1 | The content of `walls.toml`, which is their section 2.9 in the format of my section 2.7: the `main` and `other` rows; a `commands` list per agent using the six families of section 2.11.1; `story_only` on the builders; `tests:frozen` and `source:before-red` on the test writer. Every agent type a workflow uses has a row. |
| A2 | Every agent with a `read_deny` entry has only tools the runner can inspect (`Read`, `Grep`, `Glob`, `Edit`, `Write`, `MultiEdit`, `NotebookEdit`, `Bash`, `Agent`) plus tools that cannot read project files. A lint test will check this against `walls.toml`. |
| A3 | Agents that write artifacts have `Edit` or `Write`, because shell writes into the state directory are blocked. |
| A4 | The spike runner's file sets `isolation: worktree` (already in their design), and `decide` exists as a plugin command a person can type, with `disable-model-invocation: true`. |

From the pipeline designer.

| Id | Need |
|---|---|
| P1 | The person-only tool subcommands and the governed subcommands for the integration and release branches, as constants; and the story-kind table in `artifacts.toml`, from which the kinds exempt from tests-before-code are read. There is no `gates.toml`. |
| P2 | Every pipeline agent step names an agent type (also `agents-skills.md` N-PIPE-2), so that `workflow-subagent` falls under the `other` row. |
| P3 | Story worktrees at `.worktrees/<story-id>/` (their S2) and branch naming for stories and spikes as patterns (I assume `story/<id>` and `spike/<name>`). |
| P4 | The decide command is `/vsdd-factory:decide <id> <choice> [key=value ...] [-- note]`, with the id and the choice as the first two whitespace-separated arguments, every further value named, and free text after `--`. |
| P5 | `spike open` writes the ledger entry with the runner's `agent_id` (from the caller marker) and its worktree path at open time, so the hook can match a spike runner to its entry by agent id; for a named spike it attaches the id to the open entry. `spike record` (with `--increment` for a named spike) and `spike open` are the exact subcommand names, and both take files, not standard input. |
| P6 | Settled by the blueprint: no canary step and no canary command. `vsdd survey`, the first step of every workflow, refuses to start a run when the caller marker for its own call is missing. |
| P7 | The field names the hook reads stay as in their section 2.10: `step.run`, `step.workflow`, `step.subject`, `step.name`, `step.attempt`, and each finding's `key` and `location` (`path`, `line_start`, `line_end`, `quote`). Where findings sit inside a review result (I assume `data.findings`). |

From the packaging designer.

| Id | Need |
|---|---|
| K1 | `engines.claude-code` of at least 2.1.331 in `plugin.json` (already its I2). The runner makes no network calls, so the hooks add nothing to `network.hosts`. |
| K2 | The launcher contract of section 2.2 in place of its I3 for the `hook` entry: a mode argument, no `exec`, the exit-code mapping, the environment hygiene, and `find-root.sh` beside it. |
| K3 | `hooks/hooks.json` regenerated by `tools/build.py` from `registry.py`, checked by `tools/build.py --check`, and never edited by hand. |
| K4 | The subcommands of section 2.13 wired into the tool: `explain`, `rules`, `events schema`, `hooks selftest`. `doctor` runs `hooks selftest` and checks its own caller marker. |
| K5 | The observability stack reads the event log of section 2.12 (schema version 1) and takes generic tool and cost figures from Claude Code's telemetry. |
| K6 | `uv` documented as the one prerequisite, and project initialisation running `doctor`. |
| K7 | No subcommand an agent can run changes the `[hooks]` table of the project configuration once the file exists. `vsdd init --auto-allow` and `vsdd import --auto-allow` write the `auto-allow` scope once, when they create the file; `vsdd config set` is person-only. |
| K8 | The tool's governed merge writes the `pr.merged` event, and every gate subcommand takes the exit code from the command it ran, never from an agent. |

## 6. Open questions for the user

These questions are carried into section 2 of the blueprint as decisions D11, D4, D12, D6, D7 and D24.

1. **Do the old git rules still hold as default gates?** `rewrite/STATE.md` lists them as an unconfirmed
   assumption: no AI attribution in commits, no `--no-verify`, no force push, no direct push to protected
   branches. Recommendation: yes, on by default in every factory project, each one relaxable per project,
   and extend the attribution check to pull request titles and bodies.
2. **May the plugin approve its own routine operations?** A hook can answer a permission prompt with
   "allow". That is the only lever a plugin has to cut permission prompts during long runs, and it is a
   security decision. Recommendation: keep the narrow built-in part (the plugin reading its own method
   files and calling its own command). Ship the project allow-list off, and offer it at project
   initialisation with a short list: edits inside story and spike worktrees and the state directory, and
   the project's configured build and test commands, applied only when every other rule passes.
3. **Which gates may a project relax, and is an emergency switch acceptable?** Recommendation: the walls,
   write scopes, `gate-guard`, decision capture and step storage are locked; every other gate can be set to
   `advise` or `off` by a person editing the project file, never by a tool call, and is then listed in
   every briefing.
   One environment variable, `VSDD_FACTORY_HOOKS=off`, turns everything off for a session started with it
   and is announced; without it, a runner bug could lock a project until the plugin is uninstalled.
4. **Is the main session walled off from holdout scenarios too?** This is also open question 3 of
   `agents-skills.md`. Recommendation: yes. The model you talk to cannot read them; you can still open the
   files in an editor. The cost is that project-wide searches by the main session need holdout kept out of
   their reach (section 2.11.2 note).
5. **Is a typed command the only way to settle a decision inside a session?** With `decisions`, an approval
   is recorded only when a person types `/vsdd-factory:decide <id> <choice>`, or runs the tool in their own
   terminal. Saying "approved, go ahead" in prose records nothing, and the model is refused if it tries to
   record it for you. Recommendation: accept this. It is the one act a model must not be able to perform,
   and it is one short line to type.
6. **Silent, or a one-line hint, in projects that are not initialised?** Recommendation: silent, except
   for the one migration line in a project that has pre-1.0 state. The plugin may be enabled for every
   project, and unrelated projects should pay nothing.

## 7. Risks

1. **Unverified platform behaviour.** The three facts the first spike did not cover were probed on
   2026-11-23. `step-results` and `decisions` are proven as designed. `spike-bounds` is proven in part: a
   refused `SubagentStop` does not hold a workflow step that has already returned its result, so the rule
   holds a step at its `StructuredOutput` call instead. What is still unverified is listed in section 2.16:
   the command rewrite that carries the caller marker's key, the session id inside workflow steps, and the
   permit for the `Workflow` tool. Each has a named fallback, and in every case the tool's own checks at the
   point of no return still hold.
2. **The shell is not sandboxed.** Walls and write scopes over `Bash` are heuristics. A command such as
   `python -c "open(...)"` passes them, and so could a write to the evidence or decision store. Mitigation:
   walled agents get the narrowest tool set; the tool re-measures at delivery instead of trusting a stored
   record alone (`pipeline.md` risk R5); `vsdd validate` re-checks artifacts at gates. A real sandbox is
   out of scope for 1.0.
3. **Fail-closed lockout.** A runner bug, a broken uv, or a missing interpreter blocks every guarded tool in
   every factory project. Mitigation: `vsdd hooks selftest` in CI and in `vsdd doctor`, messages that carry
   the fix, and the emergency switch.
4. **False positives cause workarounds.** A guard that blocks legitimate commands teaches agents to route
   around it. Mitigation: parsed commands, the harvested old cases, `advise` strength for trials, and
   `rule.would_block` events to measure a rule before enforcing it.
5. **Latency and load.** About 55 to 70 ms is added to every guarded call, including every `Read`, and up
   to 16 agents run at once. Mitigation: the inert path costs 8 ms, imports are per event, bytecode is
   cached, and the performance test guards the budget. If `Read` volume proves costly, the `if` field can
   narrow that matcher at the price of weaker walls.
6. **Hooks disabled outside the runner's sight.** `disableAllHooks`, a disabled plugin or a bare headless
   run switches every gate off without any code of ours running. Mitigation: `gate-guard` blocks tool
   edits that would do it, and the survey of every workflow detects it, because its call has no caller
   marker.
7. **Policy tables have three owners.** A row that names no agent file, or a class glob that does not match
   the state layout, silently protects nothing. Mitigation: the policy tests in section 2.14.
8. **The main session and ancestor searches.** Until one of the three options in section 2.11.2 is
   confirmed, project-wide searches by the main session are blocked in any project that has holdout
   scenarios, which will be felt at once. This needs an answer from the state design before the build.
9. **uv changes.** The launch relies on four uv flags and on uv's exit codes. Mitigation: `vsdd doctor`
   runs the exact launch command instead of comparing version numbers, and the launcher blocks on any
   unexpected exit.
10. **A hook now sits on every step boundary.** `step-results` runs when each workflow step returns. A bug
    there stalls every workflow, and a slow grounding check on a large review adds to every pass.
    Mitigation: it reads only the files the findings name, each at most once, under the same 4 second
    deadline, and it has launcher cases with real recorded results.
11. **Scope pressure.** `artifacts.toml` could regrow the old validator family one check at a time.
    Mitigation: a check is added only for a stored fact that cannot be derived, and each kind has a size
    cap so the table itself stays small.
