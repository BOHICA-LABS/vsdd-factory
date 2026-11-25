---
kind: topic
id: seams-packaging
sources: [packaging.md#4, packaging.md#from-the-hooks-design, packaging.md#from-the-state-design, packaging.md#from-the-agents-skills-design, packaging.md#from-the-pipeline-design]
---

# Seams of the packaging design

What the packaging design provides to, and needs from, the other four designs. Each interface is linked to the
thing that now defines it. A mismatch between the two sides shows up in the place named after each need.

## What the packaging design provides

| # | Interface | Defined in |
|---|---|---|
| I1 | Paths: the repository layouts; during the rewrite every path is under `next/`; the plugin is always loaded for testing with `--plugin-dir <root>/plugins/vsdd-factory` | [[packaging:repo-layout]], [[packaging:rewrite-layout]] |
| I2 | Manifest facts: plugin name `vsdd-factory`, floor `>=2.1.331`, the three `userConfig` keys reaching hook processes as `CLAUDE_PLUGIN_OPTION_*` and `.mcp.json` as `${user_config.<key>}` | [[packaging:plugin-manifest]] |
| I3 | The launcher: `sh "<plugin>/py/launch.sh" <entry> [args]` with entry `hook` or `cli`; uv discovery, the exit-2 message, `PYTHONPYCACHEPREFIX`, one download policy | [[packaging:cli-entry]]; the `hook` entry's behaviour in [[topic:hook-launcher]] |
| I4 | The `vsdd` command: operator subcommands and shared conventions; on `PATH` for the Bash tool; in a person's terminal through the shim | [[topic:command-conventions]], [[command:doctor]], [[command:events]], [[command:obs]], [[command:repos-scan]], [[command:export-pdf]], [[command:render-diagram]], [[command:version]], [[packaging:terminal-shim]] |
| I5 | Prelude stamping: markers, layout rule, constraints, `tools/build.py [--check]` | [[packaging:build-step]], content of the prelude in [[topic:prelude]] |
| I6 | The workflow harness: `runWorkflow(name, { args, agents, workflows, maxAgents })` returning `{ result, error, calls, logs, phases }`, and the universal tests | [[packaging:tests-workflows]] |
| I7 | Lint: a file that fails a check cannot merge | [[table:lint-checks]], [[packaging:tests-lint]] |
| I8 | The roster format | [[table:roster]] |
| I9 | Test homes: `tests/unit/`, `tests/hooks/` with `payloads/`, `tests/workflows/`, `tests/e2e/` with `fixtures/<scenario>/`; one command | [[packaging:tests-python]], [[packaging:tests-workflows]], [[packaging:tests-e2e]], [[packaging:check-command]] |
| I10 | End-to-end probes: prompt, fixture, allowed tools, budget, assertions; other subsystems add probes by adding files | [[packaging:tests-e2e]] |
| I11 | Generated reference: `tools/gen_reference.py` reads agent and skill frontmatter, workflow `meta`, the rule registry, the `vsdd` argument parser and the configuration schema | [[packaging:reference-pages]] |
| I12 | Release and cutover commands: `tools/release.py prepare <version>`; `tools/cutover.py plan`, `apply <project> [--retire] [--force]` | [[packaging:plugin-release]], [[packaging:cutover-steps]], [[packaging:cutover-projects]] |
| I13 | Observability: the registry `~/.config/vsdd-factory/watched-factories`, compose project name `observability`, the collector's reading of the event log | [[packaging:observability-stack]] |

## From the hooks design

| # | Need | Mismatch shows as |
|---|---|---|
| H1 | The runner as an importable standard-library package inside the plugin, one entry script, and agreement that `bin/vsdd` uses the same launcher. Proposed paths: `py/launch.sh`, `py/hook.py`, `py/vsdd/`; if the hooks design names them differently, the three paths are needed | L-HOOK; the launcher tests in B2. Runner: [[topic:hook-runner]], [[topic:hook-launcher]] |
| H2 | The Python floor: 3.11, for `tomllib` | The `unit` matrix and the roster's `python_floor` |
| H3 | The event log: path pattern and fields (daily files, one JSON object per line, `ts`, `type`, `severity`, `rule`, `reason`, `session_id`, `agent_type`, `tool`), one in-process writer, on by default at a fixed path or redirected by an environment variable | `vsdd events` tests; the collector configuration; the dashboards; every end-to-end assertion. [[topic:event-log]], [[schema:event]] |
| H4 | The inert rule, plus one addition: a project with pre-1.0 state and no 1.0 state counts as not initialised, and the briefing adds one line naming the migration step | Smoke probe 1; step X0. [[topic:hooks-inert]] |
| H5 | A rule that blocks a bare `gh pr merge` in an initialised project and names the governed command (met by `merge-guard`) | The story tier's "merged commit is the reviewed commit" assertion. [[rule:merge-guard]] |
| H6 | A way to list rules: name, event, blocks or advises, one-line description | `docs/reference/rules.md` generation; the meta-test in `tests/hooks/`. [[table:rules]] |
| H7 | The format of a recorded payload and of a rule test case | `tests/hooks/` cannot be laid out. [[packaging:tests-python]] |

## From the state design

| # | Need | Mismatch shows as |
|---|---|---|
| S1 | How code tells the three cases apart: no state, pre-1.0 state, 1.0 state; one function in the shared core | `vsdd doctor`, `tools/cutover.py plan`, smoke probe 1 |
| S2 | The project configuration file: path and schema as data; the keys for test command, build command, base branch, merge strategy, and where the event log goes | `vsdd story deliver`, the sample project's configuration, `docs/reference/configuration-keys.md`. [[topic:project-configuration]], [[table:config-keys]] |
| S3 | Artifact frontmatter schemas and size caps as data for `vsdd validate`; no per-artifact staleness and no `vsdd stale` — an approval binds to a digest of what was approved | `vsdd validate` on the fixtures; the story tier's size-cap assertion. [[topic:artifact-rules]], [[record:decision]] |
| S4 | The lock: where it is, what it records, who takes it; if none, `vsdd lock` is dropped | `vsdd lock` tests. [[topic:run-lock]] |
| S5 | The status function that produces the briefing text, callable from `vsdd status` | `vsdd status` and the briefing would disagree. [[topic:status-report]] |
| S6 | Whether state is pushed anywhere on its own, apart from the product's commits (decides whether anything replaces the old CAS push) | A missing or needless subcommand. [[topic:state-commits]] |
| S7 | The answer to the importer question, and the name of the command a person runs after `cutover.py apply` | Step 4 of `apply` prints the wrong thing. [[topic:importer]] |
| S8 | Templates live at `<plugin>/templates/`; no research cache in 1.0 | L-XREF. [[topic:templates]] |

## From the agents-skills design

| # | Need | Mismatch shows as |
|---|---|---|
| A1 | The list of bundled servers (default: Perplexity, Tavily, Context7, Playwright); `.mcp.json`, `userConfig` and `network.hosts` follow it | L-MCP. [[topic:research-and-browser-tools]] |
| A2 | The roster as data: each agent with its tier, each skill with who may invoke it, the four budgets | L-ROSTER, L-AGENT, L-SKILL, L-BUDGET fail on the first file. [[table:roster]] |
| A3 | The standard agent template: the sections every agent has | L-AGENT. [[topic:agent-file]] |
| A4 | How agents name research tools, given that a server's tool names change when the user already has the same server; recommended that agents do not pin those names in `tools:` | An agent with no research tool on some machines; no test here can see that. [[topic:research-and-browser-tools]] |
| A5 | The names of the initialise and status skills (`setup`, `status`), the decision and driver skills (`decide`, `drive`), and the spike command (the `spike` workflow) | The smoke probes; `docs/getting-started.md`. [[skill:setup]], [[skill:status]], [[skill:decide]], [[skill:drive]], [[workflow:spike]] |
| A6 | If a skill ships a Node application: installs into a cache directory on first use, keeps its `package.json` out of the plugin root | L-MAN |

## From the pipeline design

| # | Need | Mismatch shows as |
|---|---|---|
| P1 | The prelude's content, within the constraints of the build step | `tools/build.py --check`; the harness load test. [[topic:prelude]] |
| P2 | Workflow names and which calls which | L-ROSTER; L-XREF's one-level check. [[table:roster]] |
| P3 | Cheap models for tests: settled — when `VSDD_TEST_TIER` is set, `vsdd survey` returns a tier override and the step wrapper uses it | The cost of the smoke tier |
| P4 | The story worktree convention (path and branch name) and the default merge strategy | The worktree and merge functions inside `vsdd survey` and `vsdd story deliver`. [[topic:worktrees]] |
| P5 | Settled: no subcommand. Story delivery is one command, `vsdd story deliver`, run by a clerk; worktree and merge code are library functions inside it and inside `vsdd survey` | [[command:story-deliver]] |
| P6 | Whether delivery can run with no GitHub remote; if so the end-to-end tests use that mode; if not, the `gh` test double, which is the packaging default | None; this only removes work. [[packaging:sample-project]] |
| P7 | The typical number of agent starts for one story, for the lean budget | The story tier's budget assertion has no number. [[packaging:e2e-story-tier]] |
| P8 | The scenario list for each workflow: resume points, gates, the convergence cap, spike limits | Only the universal tests would exist. [[packaging:tests-workflows]] |
