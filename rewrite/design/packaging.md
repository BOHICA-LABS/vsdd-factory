# vsdd-factory 1.0 design: packaging, repository layout, build, tests, release, cutover and supporting tools

> Amended by `rewrite/BLUEPRINT.md` on 2026-11-23, and again on 2026-11-24 after the second review pass
> (blueprint section 11). Where this document and the blueprint differ, the blueprint wins.

## Amendments from the blueprint

The text below has been edited in place where a table, a path or a command changed. This list is the complete
set of changes; where a passage further down still reads differently, this list and section 4 of the blueprint
apply. Items 1 to 15 are the first pass. Items 16 to 30 are the second pass, and they replace parts of items 3,
5, 6, 8, 10 and 12.

1. **The build step generates four things**, all checked by `tools/build.py --check`: the prelude copy in every
   workflow; the schema literals in the prelude, from `<plugin>/schemas/*.json`; the `TIER` and `ROLE` tables in
   the prelude, from `tests/lint/roster.toml`; and `hooks/hooks.json`, from `py/vsdd/hooks/registry.py`
   (SEAM-15, SEAM-16).
2. **A conformance test layer.** Shared JSON fixtures in `tests/conformance/` run through `judge()` and
   `converge()` in Node and through the tool's Python implementation, and must agree. The route rule exists only
   in Python (SEAM-16).
3. **Layout.** The plugin gains `methods/`, `schemas/`, `pipeline/` (`stages.toml`, `routes.toml`), `assets/`
   (`design-system/`, `demo/`, `scaffold/`), and under `py/vsdd/` the packages `hooks/`, `policy/`, `gates/` and
   `delivery/` and the module `schema.py`. Lint gains rows for method files, templates against the registry, the
   agents' shared block and severity table, and method existence (SEAM-39, COV-12).
4. **Roster.** `tests/lint/roster.toml` has four tiers (the clerk tier is Sonnet at low effort), 20 agents, 13
   skills, 12 workflows, and `model_visible_skills = 1`. L-AGENT allows `effort: low` for the clerk tier only and
   requires `omitClaudeMd: true` (SEAM-02, SEAM-06, OPS-19).
5. **The command table.** The state design owns the pipeline-facing commands (`state.md` 2.16; blueprint 4.9),
   and this design owns the operator tools. Removed: `vsdd pr-merge` and `vsdd worktree` (library functions
   inside `vsdd story deliver` and `vsdd survey`; the answer to need P5 is no), `vsdd cache` (no caller has a
   shell), and `vsdd stale --update`. Changed: `vsdd status [--fix] [--json] [--wave W]`. Added:
   `vsdd doctor --fix` and `--install-shim`, `vsdd export pdf`, `vsdd render diagram` (SEAM-01, SEAM-09, SEAM-26,
   SEAM-32, COV-02, COV-09, COV-12).
6. **Exit codes.** Outside an initialised project every command exits 2 except `init`, `import`, `doctor`,
   `obs`, `hooks selftest` and `--version`. Exit 3 is used only by `vsdd hooks canary` (SEAM-26).
7. **The person's terminal.** `vsdd doctor --install-shim`, offered by `setup`, writes `~/.local/bin/vsdd`, a
   shim that finds the active plugin and runs its launcher. `vsdd doctor` reports whether it is present. A probe
   checks whether `! vsdd` typed in a session works. Every message to a person names a form that works
   (SEAM-25, COV-13, OPS-22).
8. **One download policy.** Neither entry of the launcher downloads a Python. `vsdd doctor --fix` runs
   `uv python install`. The documentation states what a sandboxed Bash tool must allow, and `setup` runs
   `vsdd doctor` through the Bash tool to check it (OPS-22).
9. **Harness.** It models the agent counter shared by a parent and its children. Universal tests are added:
   every step names an existing method owned by the step's agent; with every agent returning an empty result
   the run still reaches its checkpoint; no reviewer prompt contains a fixer's free text; an unknown limit key
   fails (SEAM-04, SEAM-36, COV-18, OPS-09).
10. **End-to-end.** The story tier also kills a run in the middle of a step and of a CI wait. The conformance
    tier adds an eleven-minute gate job and the probes of the blueprint's section 10. The smoke tier adds a run
    in the default permission mode that counts prompts. Tests use cheap models through `VSDD_TEST_TIER`, which
    `vsdd survey` turns into a tier override. The story budget is set from measured counts, starting from 12
    agent starts for a story (SEAM-41, COV-03, COV-04, OPS-06, OPS-25).
11. **Scheduled runs.** `docs/running-the-pipeline.md` carries a tested recipe for running `maintain` and
    `discover` from cron or launchd with `claude -p` and the Workflow tool allowed (pipeline need P6, COV-09).
12. **Setup settings.** `setup` offers to write, into the project's local settings, the permission allow rules
    of decision D4 and the setting that disables Claude Code's commit and pull-request attribution; `vsdd
    doctor` checks the second (COV-04, OPS-31).
13. **Build plan.** Section 9 of the blueprint replaces section 2.12 here. It adds packages C1 to C3 (schemas,
    policy tables, stages and routes), S3 to S6, G1 to G7 (jobs, gates, verdicts, delivery, release) and T5
    (export and render), and renames some others.
14. **Not yet proven.** Section 10 of the blueprint is the full list; items N1 to N10 here are part of it.
15. **Open questions.** Section 6 is superseded by section 2 of the blueprint (decisions D25 to D30), whose
    recommendations the user adopted on 2026-11-23.

Second pass (2026-11-24):

16. **Layout.** There is no `pipeline/` directory and no `gates.toml`: the stage registry and the route file
    are gone, and the policy tables are three (`artifacts.toml`, `classes.toml`, `walls.toml`). The two shared
    agent texts live in `workflow-lib/` (section 2.3; P2LEAN-06, P2LEAN-13, P2COLD-22).
17. **The build step** also stamps the shared agent block and the severity table into agent files and
    generates the `OVERRIDE` table. It generates no step-name table for the hook runner (section 2.5;
    P2SEAM-10, P2COLD-10, P2LEAN-10).
18. **Commands.** Removed from the operator table: `vsdd doctor --fix`, `vsdd events emit`, `vsdd hooks
    canary` and exit code 3, `vsdd lock status`, `vsdd stale`. Section 2.10 follows blueprint 4.9
    (P2LEAN-07, P2LEAN-16, P2OPS-16).
19. **The terminal shim** resolves the plugin version enabled for the current project from
    `installed_plugins.json` and checks that it has `bin/vsdd` (section 2.10; P2COV-09).
20. **Sandbox needs** are listed and each is tested by `vsdd doctor`. A missing Python is fixed by a terminal
    command, not by the plugin (section 2.10; P2OPS-16).
21. **`setup` and decision D4.** `setup` asks before `vsdd init` and passes `--auto-allow`; the hook permit is
    the mechanism and static settings rules are only the fallback of blueprint section 10 (item 12 is
    replaced; P2COV-02, P2OPS-02, P2COLD-09).
22. **The bundled browser server** carries the option that refuses `file:` addresses, and L-MCP checks it
    (section 2.4; P2OPS-14).
23. **Lint.** L-KIND is added: every story kind's implement agent can write every class its method's output
    needs. L-AGENT checks the stamped regions. L-TEMPLATE covers `<plugin>/assets/` (section 2.6; P2COLD-02,
    P2SEAM-10, P2COV-10).
24. **Python tests.** Fixture tests are added for the route rows, the preconditions, the decision grammar,
    approve then survey then plan, assess then brief then spec, and import then one feature (section 2.6;
    P2OPS-05, P2SEAM-05, P2SEAM-06, P2SEAM-13, P2REQ-04).
25. **Harness.** Universal tests are added for walled findings in prompts, percent-encoded free text and
    argument text; scenario tests for every fast route, the amendment run and a weak-test fix (section 2.6;
    P2SEAM-02, P2COLD-06, P2REQ-12, P2REQ-09, P2COLD-04).
26. **Conformance fixtures** now state the amended behaviours, so the build cannot freeze the earlier ones
    (section 2.6; P2OPS-20, P2SEAM-07).
27. **End to end.** The sample project ships a standard-library JUnit runner. The smoke tier gains an inert
    case with the launcher missing, and prompt counts for a builder step and a spike. The story budget is 12
    agent starts. The proof is two packages, E3 (greenfield from an empty repository, then a trivial feature
    after release) and E4 (brownfield: ingest then feature, no tests or CI, import then feature, a hub)
    (section 2.6; P2COLD-28, P2COV-07, P2REQ-08, P2OPS-17, P2COLD-13, P2REQ-09).
28. **The weekly conformance job** records the model ID behind each alias and runs the story tier when one
    changes (P2REQ-13).
29. **Cutover.** The cutover report points each project at the `setup` offer to archive old process sections
    of its `CLAUDE.md` (section 2.9; P2OPS-15).
30. **Build plan.** Section 9 of the blueprint has 74 packages after the second pass: five merged, six split,
    B4 split in two, and E4 added (P2LEAN-14, P2COLD-16, P2COLD-17, P2COLD-18).

Designer area: packaging. Written 2026-11-23 on branch `rewrite/v1`. Companion designs: `pipeline.md`, `state.md`,
`agents-skills.md`, `hooks.md`.

Terms used throughout:

- **`<plugin>/`** is the plugin's root directory. After cutover it is `plugins/vsdd-factory/` in the repository, and
  `~/.claude/plugins/cache/claude-mp/vsdd-factory/<version>/` on a user's machine.
- **Integration branch** is the one long-lived branch that pull requests target (see open question 1 for its name).
- **Prelude** is the shared JavaScript that every workflow script needs (result schemas and the step wrapper that
  handles spike requests). Workflow scripts cannot import, so the prelude is copied into each one.
- **Stamping** is the act of copying the prelude into a marked region of each workflow file.
- **Roster** is a small TOML file that lists the agents, skills and workflows the plugin is expected to ship. Tests
  compare the plugin directory against it.
- **Harness** is a Node program that runs a workflow script with stubbed agents, outside Claude Code.
- **Probe** is one headless `claude -p` run with one mechanical assertion.
- **Floor** is the lowest version of something that is supported and tested.

---

## 1. Summary

In 1.0 the plugin is plain text: Markdown agents and skills, JavaScript workflows, one `hooks.json`, standard-library
Python, and two small `sh` launchers. Nothing is compiled, so the Rust workspace, the five-platform binaries, the bot
commit and the bats suite all go. The repository becomes one plugin directory (`plugins/vsdd-factory/`, the only thing
installed on a user's machine) plus the tests, tools and docs around it.

There is one build step: the shared workflow prelude is stamped into every workflow script and committed, and a check
fails when any copy drifts. Tests come in four layers: pytest for Python, a Node harness that runs workflow scripts
with stubbed agents, a lint suite over every shipped file, and headless end-to-end runs against a sample project.
Continuous integration takes minutes instead of an hour.

A release is two merges: a version pull request here, and a one-entry pull request in the marketplace repository that
pins an immutable tag. The 21 shell tools become one Python command, `vsdd`. The eight existing installs move with a
planned, idempotent cutover tool.

---

## 2. The design

### 2.1 Facts found on disk that change the brief

These were checked on this machine on 2026-11-23. They are not in `rewrite/STATE.md`.

| Fact | Consequence |
|---|---|
| The marketplace repository is `bohica-labs/claude-mp`, not `drbothen/claude-mp`. The plugin remote is `BOHICA-LABS/vsdd-factory`. A rebrand commit moved both. The marketplace entry's URL already says `bohica-labs`. | All URLs in 1.0 use `bohica-labs`. GitHub redirects the old names. |
| Tag `v1.0.0-rc.25`, `origin/main` and the installed commit (`51023185`) are the same commit. | The marketplace can be pinned to the tag with no change in content (cutover step X1). |
| `origin/main` is an ancestor of `origin/develop` (17 commits behind, 0 ahead). | The two branches can be collapsed into one by a fast-forward. |
| The marketplace entry is `git-subdir`, `path: plugins/vsdd-factory`, `ref: main`. `plugin.json`'s `version` takes precedence over the entry's. | Today, whatever is on `main` is what `claude plugin update` installs, and the marketplace pull request only changes the number shown in the catalog. 1.0 reverses that (section 2.8). |
| All eight installs are project-scoped and share one cache directory, `…/vsdd-factory/1.0.0-rc.25/`. `claude-mp` has auto-update off (the default for third-party marketplaces). | Nothing updates until someone runs `claude plugin update` in a project. But see risk 6 on cache cleanup. |
| Every one of the eight has `"agent": "vsdd-factory:orchestrator:orchestrator"` in `.claude/settings.local.json`, written by the old `activate` skill. | If 1.0 has no agent of that name, the setting dangles. The cutover removes it. |
| Every one of the eight has its own project `.mcp.json`. `prism` and `pregolya` have `CLAUDE.md` files of about 600 lines with 67 to 81 mentions of old process terms. `pregolya` has a project `SessionStart` hook that calls `.factory/hooks/ensure-heartbeat.sh`. | The cutover tool reports this residue. It does not edit a project's `CLAUDE.md`. |
| `~/.config/vsdd-factory/watched-factories` exists and lists five project paths for the observability stack. | The 1.0 observability command keeps this file and its format, so nothing has to be re-registered. |
| `claude plugin validate --strict --profile directory` on the spike plugin asks for: an `author`, a `network.hosts` declaration (it cannot read hosts out of hooks or skills), and an icon (square PNG or JPEG, 512 to 2048 px, under 2 MB, not SVG). It takes the README and licence from the repository root when the plugin folder has none. | Section 2.4. The plugin folder gets its own README and licence because only that folder is installed. |
| The platform documents `engines` (needs Claude Code 2.1.323 or later), `network` (2.1.328 or later), a `bin/` folder that is added to the Bash tool's `PATH`, `claude plugin tag` (creates a `<name>--v<version>` tag after checking the manifest), `claude plugin eval`, and a 14-day cleanup of superseded cache directories. | Used below. Each one that the spike did not exercise is listed in section 2.13. |

### 2.2 Decisions at a glance

| # | Decision | Why, in one line |
|---|---|---|
| D1 | The plugin stays at `plugins/vsdd-factory/` and stays a `git-subdir` source. | The marketplace path does not change, and tests, tools and docs are not copied to users' machines. |
| D2 | During the rewrite the whole new tree is built under a staging root, `next/`, that mirrors the final repository root. | Every new top-level name (`tests/`, `docs/`, `README.md`, the CI files) collides with an old one. One staging root means one mechanical promotion commit and no path edits. |
| D3 | The root `CLAUDE.md` is replaced on the rewrite branch first, before any building. | It is 47 KB of superseded process rules, and it is loaded into every builder and reviewer the rewrite starts. |
| D4 | Generated content is committed and checked for drift. There is no install-time build. | A marketplace install copies the git tree. Nothing runs at install. |
| D5 | All shipped Python is one package, launched through one guarded `sh` launcher. All supporting tools are one command, `vsdd`. | One guard, one interpreter pin, one name on `PATH`, one place to look. |
| D6 | Test helpers are Python (pytest, via `uv`) and Node built-ins only. No npm packages, no `package.json`. | Workflow scripts are JavaScript and need a JavaScript engine. Nothing else justifies a third toolchain. |
| D7 | A release is an immutable tag, and the marketplace entry pins that tag and its commit. | The marketplace pull request becomes the one switch that publishes, and reverting it is the rollback. |
| D8 | The tag is `vsdd-factory--v<version>`, the platform's own format. | `claude plugin tag` validates it, and plugin dependency resolution looks for it. |
| D9 | No release branch, no bot commit, no back-merge, no guardrail workflow. | With nothing compiled there is nothing to commit after the tag. |
| D10 | The minimum Claude Code version is the version the platform spike ran on, 2.1.331. | Every platform fact the design relies on was verified there and nowhere lower. |
| D11 | `userConfig` holds only per-user secrets (three API keys). Everything per-project lives in the project's own configuration file. | One home for each kind of setting. Keys go to the system credential store instead of plaintext `.mcp.json` files. |
| D12 | The eight projects keep project scope and are migrated one at a time by an idempotent tool that plans before it acts. | No change to which projects load the plugin, and a failed step can simply be re-run. |

Where `rewrite/STATE.md` gave the assistant's reading, I changed three things and say so here. It says the wrapper is
"stamped into each workflow file at build time **or** checked by a test": I do both, because a stamp without a check
drifts and a check without a stamp gives authors nothing to run (D4). It says cutover will "replace the old plugin
directory": I stage the entire new repository root, not just the plugin directory (D2), and I pin the marketplace to
the old tag before `main` changes (section 2.9). It carries the old git rules over: I keep them and add one, "a
release tag is never moved", because the old release bot force-moved every tag.

### 2.3 Repository layout

#### During the rewrite (branch `rewrite/v1`)

```
vsdd-factory/
├── CLAUDE.md                  REPLACED by a 15-line rewrite-mode file (see 2.11). The only old file touched.
├── rewrite/                   process files: STATE.md, reference/, design/, BLUEPRINT.md, workflows/, spike/
├── next/                      the entire 1.0 repository root, in its final relative shape
│   ├── plugins/vsdd-factory/
│   ├── workflow-lib/  tools/  tests/  docs/  .github/
│   ├── pyproject.toml  uv.lock  .nvmrc  .gitignore  .semgrep.yml
│   └── README.md  CLAUDE.md  CONTRIBUTING.md  RELEASING.md  CHANGELOG.md  LICENSE
└── (everything else)          the old system: read for reference, never edited
```

Builders work only under `next/`. To try the new plugin anywhere, including inside one of the eight projects, start
Claude Code with `--plugin-dir <repo>/next/plugins/vsdd-factory`. The platform documents that a `--plugin-dir` plugin
silently replaces an installed plugin of the same manifest name for that session only, so this needs no uninstall.

The checks run from `next/`: `cd next && uv run python tools/check.py`. Continuous integration is not needed during
the rewrite because nothing has been pushed. If the branch is pushed, one extra file, `.github/workflows/next.yml`
at the real root, runs the same command with `working-directory: next`. It is deleted at promotion.

Alternative rejected: building the plugin at `plugins/vsdd-factory-next/` and the rest in new root directories. Ten
top-level names collide, so the cutover would be a hand-edited merge instead of a rename.

#### After cutover

```
vsdd-factory/
├── plugins/vsdd-factory/          THE PLUGIN. The only directory a user's machine receives.
│   ├── .claude-plugin/plugin.json
│   ├── .claude-plugin/icon.png
│   ├── .mcp.json                  research and browser servers (list owned by agents-skills design)
│   ├── README.md   LICENSE        short README; LICENSE byte-identical to the root one (lint)
│   ├── agents/*.md                (agents-skills design) 20 files
│   ├── skills/<name>/SKILL.md     (agents-skills design) 13 skills
│   ├── methods/*.md               (agents-skills design) one procedure per file
│   ├── workflows/*.js             (pipeline design) 12 scripts, each with the stamped prelude
│   ├── schemas/*.schema.json      (pipeline design) the single source of every result schema
│   │                              (no pipeline/ directory: stage structure is in the scripts, and the route
│   │                              table and the preconditions are code in py/vsdd/state/)
│   ├── hooks/hooks.json           (hooks design) generated from the rule table, checked in, no variants
│   ├── py/
│   │   ├── launch.sh              the one guarded launcher: finds uv, fails loudly, runs an entry
│   │   ├── find-root.sh           project discovery, sourced by the launcher
│   │   ├── hook.py                entry for hooks (hooks design)
│   │   ├── cli.py                 entry for the vsdd command
│   │   └── vsdd/                  one standard-library package
│   │       ├── project.py config.py frontmatter.py events.py schema.py   shared core
│   │       ├── hooks/  rules/     (hooks design) the runner and its 17 rules
│   │       ├── policy/            walls.toml classes.toml artifacts.toml   (three tables)
│   │       ├── state/             (state design) stores, survey, checkpoint, routes, status, briefing
│   │       ├── gates/  delivery/  (state design) gate commands and jobs; story delivery and release
│   │       └── cli/               one module per operator subcommand (2.10)
│   ├── bin/vsdd                   3-line sh shim; puts `vsdd` on the Bash tool's PATH
│   ├── templates/                 (state design) one skeleton per artifact kind
│   ├── assets/                    design-system/, demo/, scaffold/: files that methods copy into a project
│   └── observability/             docker-compose.yml, collector/Loki/Prometheus config, Grafana dashboards
├── workflow-lib/                  not shipped: prelude.js (the canonical prelude), agent-shared.md (the block
│                                  every agent file carries), severity.md (the reviewers' severity table)
├── tools/                         build.py  check.py  gen_reference.py  release.py  cutover.py   (not shipped)
├── tests/
│   ├── unit/                      pytest: shared core and vsdd subcommands
│   ├── hooks/                     pytest: rules, in-process and through the real launcher; payloads/
│   ├── lint/                      pytest: checks over every shipped file; roster.toml
│   ├── workflows/                 node --test: harness.mjs, universal tests, scenarios
│   ├── conformance/               shared fixtures for the convergence rule, run in Node and in Python
│   └── e2e/                       pytest -m e2e: probes, sample-project/, fixtures/, fakebin/gh, conformance/
├── docs/                          user and contributor documentation (2.11)
├── .github/workflows/             ci.yml  e2e.yml  release.yml  semgrep.yml
├── pyproject.toml  uv.lock        development only: pytest, ruff, pyyaml. `[tool.uv] package = false`
├── .nvmrc                         `24`. Node is used only to run tests/workflows. No package.json.
└── README.md  CLAUDE.md  CONTRIBUTING.md  RELEASING.md  CHANGELOG.md  LICENSE  .gitignore  .semgrep.yml
```

Rules for the plugin directory, each enforced by a lint test (2.6):

- No `package.json` at the plugin root. Claude Code runs a package install at plugin install time when it finds one
  with a lockfile, and 1.0 wants no install-time behaviour. A skill that ships a Node application (the old
  `visual-companion` did) keeps its `package.json` inside its own skill folder and installs into a cache directory on
  first use.
- No `CLAUDE.md` at the plugin root. The platform does not load it and the validator warns.
- No file the plugin writes at run time. The plugin root changes with every version. Python bytecode goes to
  `${XDG_CACHE_HOME:-~/.cache}/vsdd-factory/pycache` (the launcher sets `PYTHONPYCACHEPREFIX`).
- Total size under 3 MB and no single file over 256 KB except the icon. The old plugin directory was 102 MB.

The Python layout (`py/launch.sh`, `py/hook.py`, `py/vsdd/`) is my proposal for a part the hooks design also owns.
What I need is in section 5, H1. If the hooks design names these differently, the names follow that design and
nothing else here changes.

#### Deleted at cutover, and kept

| Item | Size | Action |
|---|---|---|
| `crates/` (55 crates), `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `Cross.toml`, `deny.toml` | about 195,000 lines of Rust | Deleted. Nothing is compiled in 1.0. |
| Old `plugins/vsdd-factory/` in full, including `hooks/dispatcher/bin/` and `hook-plugins/` | 102 MB, 79 MB of it binaries | Deleted and replaced by `next/plugins/vsdd-factory/`. Content the other designs keep is ported, not moved. |
| `plugins/vsdd-factory/tests/` and root `tests/` | 2,279 bats tests | Deleted. The cases for rules and tools that port are harvested first (2.6). |
| `scripts/`, `ci/`, `benches/` | 7 scripts, 1 YAML, 1 script | Deleted. See the mapping in section 3. |
| `.github/workflows/` `ci.yml`, `kani.yml`, `release.yml`, `release-branch-guardrail.yml`; `.github/release-notes/` | 2,100 lines | Deleted and replaced. `semgrep.yml` is kept with its branch names updated. |
| `docs/demo-evidence/` | 110 MB, 1,313 files | Deleted (open question 6). |
| `docs/guide/` (30 files), `docs/lessons-learned/`, three top-level docs | 476 KB | Deleted and replaced by the 1.0 set (2.11). |
| `CHANGELOG.md` | 364 KB | Replaced by a new file that starts at 1.0.0 with one line pointing at tag `v1.0.0-rc.25` for history. |
| `README.md`, `CONTRIBUTING.md`, `RELEASING.md`, `CLAUDE.md` | | Rewritten. |
| `.claude/settings.json`, `.claude/scheduled_tasks.lock`, `.claude/pr-reviews/`, `.lazyclaude/` (a gitlink with no `.gitmodules`) | | Deleted. They enable the old plugin for this repository or are stray session files. |
| `.mcp.json` at the root (untracked, holds live keys) | | Not in git. Delete it locally once the keys are in the plugin configuration. Never copy it into the plugin. |
| `LICENSE`, `.semgrep.yml`, git history, all `v1.0.0-rc.*` tags | | Kept. |
| `origin/factory-artifacts` branch | | Kept as a read-only archive. No job mounts it. |
| `rewrite/` | | Kept until the last project is migrated, because it holds the resume record. Then deleted in one commit. |

Before the promotion commit, an annotated tag `archive/rc-line` is put on the tip of `origin/develop`, so the last
state of the old source has a permanent name.

### 2.4 `plugin.json`

The complete file:

```json
{
  "name": "vsdd-factory",
  "displayName": "VSDD Factory",
  "version": "1.0.0",
  "description": "Verified Spec-Driven Development for Claude Code: specifications, stories, test-first delivery, adversarial review, holdout evaluation and release gating, run by workflows and enforced by hooks.",
  "author": { "name": "Bohica Labs", "url": "https://github.com/bohica-labs" },
  "homepage": "https://github.com/bohica-labs/vsdd-factory#readme",
  "repository": "https://github.com/bohica-labs/vsdd-factory",
  "license": "MIT",
  "keywords": ["vsdd", "spec-driven", "tdd", "sdlc", "workflows", "adversarial-review"],
  "icon": "./.claude-plugin/icon.png",
  "documentationUrl": "https://github.com/bohica-labs/vsdd-factory/tree/main/docs",
  "supportUrl": "https://github.com/bohica-labs/vsdd-factory/issues",
  "engines": { "claude-code": ">=2.1.331" },
  "network": {
    "hosts": [
      "api.perplexity.ai", "mcp.tavily.com", "mcp.context7.com", "registry.npmjs.org",
      "github.com", "api.github.com", "*.githubusercontent.com",
      "*.docker.io", "*.docker.com"
    ]
  },
  "userConfig": {
    "perplexity_api_key": {
      "type": "string", "sensitive": true,
      "title": "Perplexity API key",
      "description": "Lets research agents search with Perplexity. Optional: leave empty to work without it."
    },
    "tavily_api_key": {
      "type": "string", "sensitive": true,
      "title": "Tavily API key",
      "description": "Lets research agents search and extract with Tavily. Optional."
    },
    "context7_api_key": {
      "type": "string", "sensitive": true,
      "title": "Context7 API key",
      "description": "Lets agents fetch current library documentation from Context7. Optional."
    }
  }
}
```

Every field, and every field left out:

| Field | Value | Note |
|---|---|---|
| `name` | `vsdd-factory` | Unchanged. It is the namespace of every agent, skill and workflow, and must never be renamed. |
| `displayName` | `VSDD Factory` | Shown in the interface only. |
| `version` | `1.0.0` | Changed only by a release pull request (2.8). While it is unchanged, installed copies do not update, by platform design. |
| `description` | as above | Rewritten: it no longer promises brownfield-to-release in one sentence of jargon. |
| `author`, `homepage`, `repository` | `bohica-labs` | The old file says `drbothen`. See open question 5. |
| `license` | `MIT` | Unchanged. |
| `keywords` | six | Discovery only. |
| `icon` | `./.claude-plugin/icon.png` | A generated square 512 px PNG. Read only by Anthropic's directory, never by Claude Code. See open question 5. |
| `documentationUrl`, `supportUrl` | GitHub links | Read only by the directory. Harmless otherwise. |
| `engines` | `>=2.1.331` | The floor (D10). Claude Code versions before 2.1.76 reject a manifest that has this key at all; they are far below the floor. The same number is recorded in `tests/lint/roster.toml` under `[platform]`, and the CI matrix reads it from there. |
| `network.hosts` | nine entries | Descriptive only: the platform says this field allows and blocks nothing. The table below says what reaches each host. |
| `userConfig` | three optional secrets | `sensitive: true` stores each value in the system credential store, once per user, so all projects share them. Each is exported to hook processes as `CLAUDE_PLUGIN_OPTION_PERPLEXITY_API_KEY` and so on, and is substituted into `.mcp.json` as `${user_config.perplexity_api_key}`. None is `required`, so the plugin works with no keys. |
| `$schema` | omitted | No verified URL for it, and Claude Code ignores it. |
| `defaultEnabled`, `dependencies`, `metadata`, `privacyPolicyUrl`, `termsOfServiceUrl`, `channels`, `types`, `lspServers`, `outputStyles`, `commands`, `experimental` | omitted | Defaults are right or the feature is unused. |
| `agents`, `skills`, `workflows`, `hooks`, `mcpServers` | omitted | The default locations are used. Setting `agents` or `workflows` replaces the default scan and gains nothing. |

Hosts and what reaches them:

| Host | Reached by | When |
|---|---|---|
| `api.perplexity.ai`, `mcp.tavily.com`, `mcp.context7.com` | the three research servers in `.mcp.json` | when an agent researches |
| `registry.npmjs.org` | `npx`, which starts the Perplexity and Playwright servers | first start of each |
| `github.com`, `api.github.com` | `git` and `gh` commands that agents run for pull requests | story delivery |
| `*.githubusercontent.com`, and `github.com` again | `uv`, only if it has to download a Python interpreter because none at the floor is installed | at most once per machine |
| `*.docker.io`, `*.docker.com` | `vsdd obs up` pulling images | only if the observability stack is started |

The shipped Python reaches no host. A lint test forbids it from importing `socket`, `ssl`, `http`, `urllib.request`,
`ftplib` and `smtplib`, so this is true by construction. Browser tools reach whatever site the project under test
names; that cannot be declared.

`.mcp.json` is packaged by this design, but its list of servers belongs to the agents-skills design. My default, taken
from the configuration the user already runs, is below. Package versions are pinned exactly; a lint test rejects
`@latest`.

```json
{
  "mcpServers": {
    "perplexity": { "command": "npx", "args": ["-y", "@perplexity-ai/mcp-server@<exact>"],
                    "env": { "PERPLEXITY_API_KEY": "${user_config.perplexity_api_key}" } },
    "tavily":     { "type": "http", "url": "https://mcp.tavily.com/mcp/",
                    "headers": { "Authorization": "Bearer ${user_config.tavily_api_key}" } },
    "context7":   { "type": "http", "url": "https://mcp.context7.com/mcp",
                    "headers": { "CONTEXT7_API_KEY": "${user_config.context7_api_key}" } },
    "playwright": { "command": "npx", "args": ["-y", "@playwright/mcp@<exact>", "<the option that refuses file: addresses>"] }
  }
}
```

The Playwright entry carries the server's option that refuses `file:` addresses, so that the bundled server
cannot be used to read a file around an information wall. The placeholder is deliberate. Package B1 fills it
with the option that the version it pins documents for this, and probe 33 (blueprint section 10) confirms on
the running server that a `file:` address is then refused, or names the option that does. The confirmed name,
recorded in `rewrite/STATE.md`, is the string L-MCP requires. The hook's own check of navigation addresses
(`hooks.md` 2.11.2) applies whichever server a session ends up with, so the wall does not rest on this option
alone.

Three things the agents-skills designer must know, all from the platform documentation. First, a plugin server's
tools are named `mcp__plugin_vsdd-factory_<server>__<tool>`. Second, if the user or the project already has a server
at the same command or URL, the plugin's copy is dropped as a duplicate and the surviving tools carry the other name,
so tool names are not stable across machines. Whether the user's own Playwright server survives beside the bundled
one, and with which options, is blueprint section 10, item 33. Third, for a project-scoped install, each bundled
server needs a one-time approval per project.

### 2.5 The build step

A marketplace install copies the committed tree and runs nothing. So there is no build at install time, and anything
generated is committed. `tools/build.py` generates six things inside the plugin, and `tools/gen_reference.py`
generates the reference pages. `--check` on either fails on drift.

| Generated | From | Where it lands |
|---|---|---|
| The workflow prelude | `workflow-lib/prelude.js` | The stamped region of every workflow file |
| Schema literals (`S_FINDING`, `S_GATE` and the rest) | `<plugin>/schemas/*.schema.json` | A marked block inside the prelude, so scripts, tool and hook runner validate the same shapes |
| The `TIER`, `ROLE` and `OVERRIDE` tables | `tests/lint/roster.toml` (`[tiers]`, `[agents]`, `[overrides]`) | A marked block inside the prelude, so a workflow cannot use a model or an agent name the roster does not have, and the only step that runs off its agent's tier is one the roster lists |
| The agents' shared block | `workflow-lib/agent-shared.md` | A marked region of every agent file (`spike-runner` and `clerk` take the variants the source file defines) |
| The severity table | `workflow-lib/severity.md` | A marked region, under `## Severity`, of every reviewer file. It is the table of `pipeline.md` 2.7; the blueprint's 4.10 reproduces it |
| `hooks/hooks.json` | `py/vsdd/hooks/registry.py` | The checked-in file, so a rule cannot be registered and left unwired. Built by package B4b, after the rule registry exists |

The build generates nothing for the hook runner from the workflows. The first pass had it derive a table of
step names and the agent types allowed to return each, but step names are built at run time, so the hook
stores the agent type with each record and the tool checks it when a pass is counted (`hooks.md` 2.11.7).
`[reveals]` in `walls.toml` is generated from the class of each artifact kind in `artifacts.toml`, by the
same build, under the same `--check`.

**1. The workflow prelude.** `workflow-lib/prelude.js` is the single source. `tools/build.py` copies it into a marked
region of every file in `<plugin>/workflows/`:

```js
export const meta = {
  name: 'deliver-story',
  description: '...',
  phases: [{ title: 'Recon' }, { title: 'Build' }, { title: 'Review' }],
}

// >>> stamped:prelude  (generated from workflow-lib/prelude.js by tools/build.py; do not edit between the markers)
...the prelude, verbatim...
// <<< stamped:prelude

// the workflow's own code starts here
```

Layout rule, kept simple so no JavaScript parser is needed: line 1 is `export const meta = {`, the object ends at the
first line that is exactly `}`, and the opening marker is the next non-blank line.

`uv run python tools/build.py` rewrites the region in every workflow file, inserting it if absent.
`uv run python tools/build.py --check` changes nothing and exits 1, naming each file, if any of these is false:

- every `.js` file in `<plugin>/workflows/` has exactly one opening and one closing marker, in that order and place;
- the text between them equals `workflow-lib/prelude.js` byte for byte;
- no file outside `<plugin>/workflows/` contains a marker.

The check runs in `tools/check.py`, in the CI `static` job, and again in the release workflow, so a release cannot
carry a drifted copy.

Constraints on the prelude, which the pipeline designer writes: declarations only (`const` and `function`); no
`import` or `export`; no `Date.now()`, `Math.random()` or argument-less `new Date()`; no reference to anything a
single workflow defines; a header comment listing every top-level name it introduces; at most 400 lines. A workflow
that redeclares one of those names fails to load in the harness with a syntax error, which is how collisions are
caught. Script text is executed by the platform and is not put into any model's context, so stamping the whole prelude
into every workflow costs no tokens.

Alternative rejected: a separate source tree with an include directive and fully generated output. It leaves two
copies of every workflow in the repository and makes the shipped file something authors must not edit. With marked
regions the shipped file is the file you edit, and only the region is machine-owned.

**2. The reference pages.** `tools/gen_reference.py` writes `docs/reference/*.md` from the shipped files themselves
(2.11). `--check` fails on drift. This replaces six hand-maintained reference pages whose counts were all stale.

Nothing else is built. There is no per-platform variant and no version stamping: the version
is edited by hand in one place, `plugin.json`, in a release pull request.

### 2.6 Test strategy

One command runs everything that does not cost money:

```
uv run python tools/check.py            # static + unit + workflows, about a minute
uv run python tools/check.py static     # build --check, ruff, lint suite, shellcheck, gen_reference --check
uv run python tools/check.py e2e-smoke  # starts real model sessions; needs credentials
```

#### Layer 1: Python tests (pytest through `uv`)

`pyproject.toml` sets `pythonpath = ["plugins/vsdd-factory/py"]`, so tests import the shipped package directly.

| Directory | What it tests | Rules for the tests |
|---|---|---|
| `tests/unit/` | The shared core (project detection, configuration loader, frontmatter reader, event writer and reader) and every `vsdd` subcommand. | Git behaviour is tested against real temporary repositories. `gh` is a test double on `PATH` (`tests/e2e/fakebin/gh`, shared with layer 4). Docker calls sit behind one function that tests replace. |
| `tests/hooks/` | Every rule, two ways: in-process (rule function, payload, temporary project, decision) and **through the real launcher** (`sh py/launch.sh hook` with the payload on stdin, asserting exit code, stderr and stdout). | A meta-test walks the rule registry and fails if any blocking rule lacks a through-the-launcher case that blocks and one that allows. This is the test that would have caught the old `protect-bc`, which decided to block and never told Claude Code. |
| `tests/hooks/` failure modes | `uv` missing from `PATH` exits 2 with the message; a rule that raises behaves as the hooks design says; an unknown tool (`Workflow`, `StructuredOutput`) passes; a project that is not initialised gets exit 0 and no output. | These run on Linux and macOS, on the Python floor and on the newest stable CPython. |
| `tests/hooks/` latency | Twenty launches of the hook entry in an uninitialised project. | Fails if the median exceeds 400 ms; warns above 150 ms. Every tool call pays this cost. |

The frontmatter reader is a small subset parser that ships in the plugin. A differential test parses every
frontmatter block in the plugin (agents, skills, templates) with both the shipped reader and PyYAML, and fails on any
difference. PyYAML is a development dependency only.

The payload format, the rule registry and the per-rule cases belong to the hooks design. This design supplies the
directories, the launcher test, the meta-test and the matrix.

Fixture tests in `tests/unit/`, each a small state tree and one call. They hold the sequences that the first pass
got wrong and the second pass fixed, so they cannot come back.

| Fixture | Asserts | Package |
|---|---|---|
| Approve, survey, plan | A `spec` decision is settled; the next `plan` survey finds the approval current and the specification locked. Settling wrote the decision record and nothing else | S4 |
| Assess, brief approved, spec | `discover` in assess mode ends; the brief is approved; the `spec` survey sees an open cycle | S3 |
| The route table, one fixture per row | Among them: a brownfield project with a queued request gives `feature` with that request; a greenfield project after its release gives done, not `verify`; a standing `ideas` decision plus a queued request gives `feature`; a wave gate at its cap gives "human" with that decision on the first try; a `spec-gap` settled `amend` with nothing else runnable gives `feature` with `amend`; a brief in draft gives the brief skill; missing commands give the lines to add; a fast route whose story is delivered gives `verify`, not `plan`; a maintenance cycle with delivered fixes gives `verify`; a standard-route feature whose delta specification is approved, with no run records on disk (a fresh clone), gives `plan`, because the route is read from `cycle.json` | S3 |
| The preconditions, one fixture per predicate | `pipeline.md` 2.5 | S3 |
| The decision grammar | Every documented choice parses to the right values: repeated `key=`, `item=`, `name=`, `stage=`, text after `--`, a qualified id; an ambiguous bare id is refused with the choices | S4 |
| Import, then one fast-route feature | After `vsdd import` on a fixture cut from a real project, the route is `feature` with the oldest queued request when an epic was in flight, and steady state when none was; either way `feature` starts | S5 |
| Class precedence | `src/parser/tests.rs` is in `tests`; a path in no class is reported | S1 |
| The baseline comparison | A test that failed at the baseline does not fail the green gate; a new failure does; an inherited test that fails once and passes on the second run does not, and is recorded as flaky in the evidence and then in the unit's closure record, where `vsdd report debt` lists it | G2 |
| Holdout thresholds | A must-pass scenario under its floor and a mean under the threshold each produce a blocking finding | G3 |
| Re-attaching jobs | The same long command run twice starts one job; a second process re-attaches | G1 |

Harvesting. The old bats suites are deleted, but the cases in them are data. For each rule or tool that ports, its
builder converts the old cases into table rows before the old file goes: for example the command strings in
`destructive-guard.bats` and `hooks.bats`, and the 81 cases in `factory-obs.bats`.

#### Layer 2: workflow tests (Node built-ins only)

`tests/workflows/harness.mjs` runs a workflow script the way the platform does, with stubs. It reads the file,
rewrites `export const meta` to `const meta`, wraps the body in an async function, and runs it in a fresh `node:vm`
context that contains only the platform's globals. Skeleton of a scenario test:

```js
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { runWorkflow } from './harness.mjs'

test('deliver-story stops at the review cap and escalates', async () => {
  const run = await runWorkflow('deliver-story', {
    args: { story: 'S-1.01' },
    agents: {                                   // looked up by label, then by agentType, then '*'
      'vsdd-factory:adversary': () => ({ verdict: 'findings', findings: [{ severity: 'high', title: 'x' }] }),
      '*': (prompt, opts) => minimal(opts.schema),
    },
    maxAgents: 80,
  })
  assert.equal(run.error, null)
  assert.equal(run.result.outcome, 'escalated')
  assert.ok(run.calls.filter((c) => c.agentType === 'vsdd-factory:adversary').length <= 5)
})
```

`run` carries `result`, `error`, `calls` (label, agent type, model, effort, isolation, phase, prompt, schema, the
stubbed result), `logs` and `phases`.

The harness copies the platform's documented behaviour, and each line below is a place where a script that works in
the harness must also work for real:

| Platform behaviour | In the harness |
|---|---|
| No file, shell, module or network access | The context has no `require`, `process`, `fetch` or `import()`. |
| `Date.now()`, `Math.random()`, argument-less `new Date()` throw | They throw. |
| `agent()` with a schema returns a validated object; without one, a string; `null` on a dead agent | The stub's return value is checked against the schema by a small built-in validator (type, properties, required, enum, items). A stub may return `null`. |
| A schema needs `type: 'object'` and `properties` at the root, and `required` within `properties` | Checked on every call; a bad schema throws, as it does on the platform. |
| `parallel()` turns a throwing thunk into `null` and never rejects | Same. |
| `pipeline()` drops an item to `null` when a stage throws | Same. |
| `workflow()` runs a child once; a child that calls `workflow()` throws | The child file is loaded and run for real unless the test overrides it; a second level throws the platform's message. |
| At most 1,000 agents in a run | Counted; a test can set a lower `maxAgents` to prove a loop ends. |
| `meta` is a pure literal at the top; phase titles match | Phase titles are checked. Purity is checked in layer 3, by the platform's own validator. |

Universal tests need no authoring. They run against every file in `<plugin>/workflows/` automatically:

1. It loads in the harness (syntax, no forbidden global, no clash with a prelude name).
2. Its file name equals `meta.name`, and every `phase()` title is in `meta.phases`.
3. Every `agentType` it passes exists in `<plugin>/agents/`, and every `workflow()` target exists.
4. With every agent answered by the smallest object its schema allows (empty strings, `false`, empty arrays, the
   first enum value), the run ends without an error and under the agent cap.
5. With every agent answered "the other way" (`true`, one-item arrays, the last enum value), which includes every
   agent asking for a spike every time, the run still ends under the cap. This is the mechanical proof that
   convergence loops and spike limits terminate.
6. Every step that is not a clerk step begins its prompt with `Method: <name>`, the method file exists, and the
   step's agent owns that method.
7. With every agent returning `{status: 'done'}` and empty data, and again with one agent throwing, the run still
   ends by calling the checkpoint command.
8. No prompt sent to a reviewer contains a fixer's `note` or summary text, and no claim carries a title or a quote.
9. A survey that returns a limit key the prelude does not know fails the run.
10. No prompt sent to an agent that is denied `holdout` contains text from a walled finding: the harness plants a
    marker string in a scenario-reviewer finding and in a finding located under `.factory/holdout/`, and fails if
    the marker appears in any prompt to another agent type.
11. Every free-text value in a clerk's command went through the percent-encoder: the harness passes a request
    containing spaces, quotes and a semicolon, and checks that the command is one segment with no raw text.
12. Every workflow accepts its documented argument text as well as an object, through `parseArgs()`, and a
    `key=value` word whose key is not one of its arguments stays in the main argument.
13. Every child story a workflow starts receives `limits.agents` equal to `story_agents`, and a child that throws
    is charged that amount.

The harness counts agents the way the platform does: a parent and the children it calls share one counter and the
1,000-agent stop, so a wave whose children overspend is caught here.

#### Layer 2b: conformance fixtures

The convergence rule exists twice: `judge()`, `converge()` and `holdoutFindings()` in the prelude, and the Python
the tool's verdict function uses. `tests/conformance/` holds JSON cases (findings, claims, coverage, rulings, the
expected verdict). The Node tests run each case through the prelude functions, the pytest suite runs the same files
through the tool, and both must produce the expected verdict. The route rule is not duplicated: only the tool
computes it.

Blueprint 4.14 names the prelude's `judge()` and `converge()` as the single source of the rule, and the only
rendering of that code in the design is the skeleton in `pipeline.md` 2.11. So the behaviours both review passes
fixed are stated here as fixtures, each with its expected verdict, and package G3 writes them before it ports the
code. A port of an older reading fails them.

| Fixture | Expected |
|---|---|
| A clean first pass | `converged`, `clean`, one pass, `rounds_used` 1 |
| A pass with no blocking finding and one item not marked checked | Not clean: the gap is a finding of category `coverage` owned by the test-writer, the fixer is called, and the next pass converges |
| A finding still open at the cap, never disputed, arbiter rules `not_defect` | `blocked`, `cap`; nothing dismissed |
| The same, disputed by the builder | `converged`; one dismissal by the arbiter |
| The same, disputed, but `critical` or category `security` | `blocked`, `cap` |
| Two rounds used in earlier runs, cap 3 | One more round is allowed, then `blocked`, `cap`, `rounds_used` 3 |
| A reviewer returns blocked | `blocked`, `reviewer_blocked`; not `failed`, `no_result` |
| A must-pass scenario at 0.5 (floor 0.6) | One finding of category `holdout`, key `holdout:<HS id>`, no location: blocking |
| A mean of 0.8 (threshold 0.85) | One finding `holdout:<unit>:mean`: blocking |
| A severe finding with no location and a category other than `holdout` | Advisory |
| A reviewer pass stored with a builder's agent type | The tool does not count it (Python only) |
| Claims after a fix | Each has `key`, `path`, `line_start`, `line_end`, `resolution`, `commit` and nothing else; a `holdout` claim has no path |

Scenario tests, written by whoever builds each workflow, cover what the universal tests cannot: resuming (the recon
stub reports steps already done, and the test asserts no agent ran for them), each gate, each human stop, and the
spike round trip. The second pass names these, because each was a path that could not run as first designed: one
scenario for each fast route of `feature` (`trivial`, `bugfix`, `hotfix`), in which the story is written after
the survey and admitted by `vsdd gate plan --story`; a fast route with two stories in which the second fails its
plan gate once, where the step is retried and both stories are delivered; the amendment run inside an open cycle;
a weak-test finding fixed by the test-writer and recorded with `vsdd gate red --refreeze`, where the strengthened
test fails until the implementer changes the code; a fix round in `verify` that does not fit beside the reserve
and returns `partial`; a named spike opened, advanced and closed by the runner; and a
maintenance run that delivers fixes and opens a maintenance cycle.

#### Layer 3: lint over every shipped file (pytest, `tests/lint/`)

Lint is ordinary pytest so that there is one runner and one report format.

| ID | Check |
|---|---|
| L-MAN | `plugin.json` has exactly the fields in 2.4. The `engines` floor equals `[platform] min_claude_code` in the roster. README, LICENSE (identical to the root one) and the icon exist. No plugin-root `CLAUDE.md`, `package.json` or `settings.json`. |
| L-MCP | Every `${user_config.X}` in `.mcp.json` is declared in `userConfig`. Every remote host is in `network.hosts`. Every `npx` package is pinned to an exact version. No value looks like a literal credential. The Playwright entry carries the option that refuses `file:` addresses. |
| L-HOOK | `hooks.json` has the top-level `hooks` key. Every command goes through the launcher. Every handler has a `timeout`, and it is 60 or less: the unit is seconds, and the old value of 10000 was read as nearly three hours. Event names are real. Referenced files exist. |
| L-AGENT | The file name equals `name`. `description` is present. `model` and `effort` equal what the roster's tier says (`low` is accepted for the clerk tier only). `omitClaudeMd: true` is set. None of `permissionMode`, `hooks`, `mcpServers`, `initialPrompt` appears, because the platform silently ignores them in plugin agents. Every entry in `tools` is a known tool. The body is within the size cap and has the sections the standard agent template requires. The shared block is present byte for byte, and reviewer files carry the severity table and the finding key rule byte for byte. Both are stamped by the build from `workflow-lib/agent-shared.md` and `workflow-lib/severity.md`, so the lint compares each file with one source and no hand-written copy exists to drift. Every agent except `spike-runner` and `clerk` lists the Agent tool. Every agent has a row in `walls.toml` and every row has an agent. |
| L-METHOD | Every method file is within its line cap. Every method an agent file or a workflow names exists, and every method file is named by at least one agent file. Every method in the table of `pipeline.md` 2.9.13 is listed by the agent the table gives it to. |
| L-TEMPLATE | Each skeleton in `<plugin>/templates/` agrees with its entry in `artifacts.toml`: the same frontmatter keys and required headings. Every path under `<plugin>/assets/` (`design-system/`, `demo/`, `scaffold/`) that a method cites exists; nothing is copied from a skill's folder. |
| L-KIND | For every story kind in `artifacts.toml`, the agent of its implement stage (and of its tests stage, where it has one) holds, in `walls.toml`, a write grant for every class the method's output needs: the implementer `source` and `manifests`; the test-writer `tests`; `devops` `ci`, `repo-config`, `manifests`, and for `repo-setup` new files in `source` and `tests`; the verifier `harness` and `tests`; the technical-writer `docs`. This is the check that would have caught the scaffold story being given to an agent that could write only `.github/**`. |
| L-POLICY | `walls.toml` has `[callers]` and `[reveals]`; every subcommand under `[callers]` exists; `[reveals]` has one entry per artifact kind and equals what the build generates; no agent row carries `relaxed_by`; `classes.toml` has the product-class precedence. There is no `gates.toml`. |
| L-SKILL | The directory name equals `name`. `description` plus `when_to_use` is at most 1,536 characters, the platform's truncation point. The invocation mode equals the roster's. The file is within its size cap. Supporting files it names exist. |
| L-ROSTER | The agents, skills and workflows on disk are exactly those in `tests/lint/roster.toml`: none missing, none extra. |
| L-BUDGET | The number of model-visible skills, and the total characters of all descriptions that are listed in every agent's context, are within the roster's budgets. |
| L-XREF | Every `vsdd-factory:<name>` written in any shipped file resolves to a real agent, skill or workflow. Every `vsdd <subcommand>` written anywhere is a real subcommand. Every `${CLAUDE_PLUGIN_ROOT}/...` path exists. A workflow that another workflow calls does not itself call `workflow()`. |
| L-DEAD | No shipped file contains old-system vocabulary: `.lobster`, `state-manager`, `factory-dispatcher`, `hooks-registry`, `openclaw`, `LiteLLM`, `GPT-5`, `Gemini`, `Corverax`, `dark-factory`, `/vsdd-factory:activate`, and names of agents and skills the roster dropped. |
| L-PY | Shipped Python imports only standard-library modules (checked against `sys.stdlib_module_names`) and the `vsdd` package, and none of the network modules. It compiles on the floor. `ruff` is clean. |
| L-SH | `shellcheck` and `sh -n` pass on `py/launch.sh` and `bin/vsdd`. Both start with `#!/bin/sh`. |
| L-SIZE | The plugin directory is under 3 MB; no file over 256 KB except the icon. |
| L-DOC | Generated reference pages are current. `CHANGELOG.md` headings descend. If `plugin.json`'s version has no tag yet, the changelog has a heading for it. |
| L-OBS | The compose file and the YAML configuration parse. Dashboards are valid JSON and refer to data sources by the pinned identifiers. Every image has a version tag. |

The roster, `tests/lint/roster.toml`, is about sixty lines:

```toml
[platform]
min_claude_code = "2.1.331"
python_floor = "3.11"

[tiers]                         # the user's model policy, as data; the one source for agent files and the prelude
planner  = { model = "fable",  effort = "xhigh" }
builder  = { model = "sonnet", effort = "xhigh" }
reviewer = { model = "opus",   effort = "xhigh" }
clerk    = { model = "sonnet", effort = "low" }     # steps that run one vsdd command

[agents]                        # name = tier        (20 names from agents-skills design)
adversary = "reviewer"
implementer = "builder"
clerk = "clerk"

[skills]                        # name = "person" | "model" | "both"

[workflows]                     # name = { calls = [...] }

[overrides]                     # the only steps that run off their agent's tier (decision D5); lint rejects any other
"ingest/map" = "builder"
"ingest/analyze" = "builder"

[budgets]
model_visible_skills = 0        # numbers from agents-skills design
listed_description_chars = 0
agent_file_lines = 0
skill_file_lines = 0
```

It is an expectation, not a registry: nothing reads it at run time, and it changes only when the roster changes. Its
job is to make the user's model policy and the context budget into failing tests. It is the one place where this
design asks for a hand-kept list, and I think it earns that: each agent that starts pays for every listed skill.

#### Layer 4: end to end, headless

Each probe starts a real session:

```
claude -p "<prompt>" \
  --plugin-dir <repo>/plugins/vsdd-factory \
  --setting-sources project,local --strict-mcp-config \
  --allowedTools "Workflow Agent Skill Read Write Edit Bash Glob Grep" \
  --output-format json --max-budget-usd <cap> --no-session-persistence
```

It runs in a temporary copy of `tests/e2e/sample-project/` with a scenario overlay from `tests/e2e/fixtures/` copied
on top. `--setting-sources project,local` keeps the developer's own plugins, hooks and servers out of the run.
Assertions are made on the exit code, the JSON result (`is_error`, cost, turns), files on disk, the sample project's
git log and the hook runner's event log. No assertion reads model prose, except for a fixed token that a prompt asks
for.

The sample project, `tally`, is a Python package of one module with standard-library `unittest` tests. Python is
chosen over the old Rust fixture because `uv` is already required, so the sample needs no extra toolchain. The red
gate reads per-test results from a JUnit report, and `unittest` writes none, so the sample ships its own runner,
`tools/junit_runner.py`: about sixty lines of standard library that run the suite and write JUnit XML. Its test
command is `uv run python tools/junit_runner.py` and its `test_report` is `junit:build/junit.xml`. A
standard-library runner is chosen over `pytest --junitxml` because the end-to-end tiers must not need a package
download: a PyPI fetch failed in the sandboxed shell during the platform spike. Its origin
is a local bare repository. The sample exists in four shapes, as fixture overlays: an empty repository with a
brief (E3); the package with its tests and one known lint failure and no factory state (E4, ingest); the package
with no tests and no CI (E4); and two small repositories under a hub (E4). Pull requests go through `tests/e2e/fakebin/gh`, a Python stand-in for the `gh` commands
the pipeline uses, backed by that bare repository and a JSON file. A command it does not implement fails with
"unsupported in the test double" and is logged, so a gap shows up as a clear failure. Each fixture overlay is checked
with `vsdd validate` in the unit tests, so fixtures cannot rot as the state schema changes.

| Tier | Runs | Probes | Rough cost |
|---|---|---|---|
| **Smoke** | Every pull request that touches `plugins/`, `workflow-lib/` or `tests/e2e/`. Weekly on macOS. | (1) Inert: in a project that is not initialised, a command the gates would block is allowed, and nothing is written. (2) Guard: with `uv` hidden, a tool call is blocked with the `uv` message. (3) Briefing: in an initialised fixture, the session can repeat the exact next command its briefing gave. (4) Wall: a builder-tier agent is blocked from a holdout file and a reviewer-tier agent reads it. (5) Workflow: the smallest workflow runs with `Workflow` allowed and returns its structured result. (6) Tool: `vsdd --version` runs from the main session and from a workflow step. (7) Prompts: in the default permission mode, with `[hooks.rules.auto-allow] scope = "extended"` and no pre-allowed tools, one builder step (it edits a file, runs one test by name, runs `ls` and `git status`, commits and runs its gate) and one experiment spike requested by a step raise no permission prompt; with `scope = "core"` the count is recorded, not asserted. The count over a whole wave is taken in the greenfield proof, which has the wave workflow. (8) Liveness: with hooks disabled, `vsdd survey` refuses to start a run, because its call has no caller marker. (9) Inert with a broken install: with the launcher file removed, a guarded tool call in a project that is not initialised is allowed and prints nothing, and the same call in an initialised project is blocked with the reinstall message. (10) Caller identity: two sessions in one project; each session's `vsdd survey` reads its own marker, and the lock records the session that called. All probes run on cheap models through `VSDD_TEST_TIER`. | a few dollars |
| **Story** | In the release workflow, weekly, and on demand. | From the `story-ready` fixture, deliver one story ("add `median()`"). Then the same run killed three ways and started again as a new process with the same command: at a step boundary, in the middle of a step (the worktree holds uncommitted work), and in the middle of a CI wait (a pull request is open). The second process must not review a converged story again. | tens of dollars; measured in step 4 of the plan |
| **Conformance** | Weekly, against the newest Claude Code. | Tiny workflows that assert, on the real platform, each row of the harness table above, plus the items in 2.13 and the platform facts the design now rests on (the hook sees and can block a step's result; the decide event; the hook permit; the command rewrite that carries the caller marker). It also records the model ID behind each alias (`fable`, `sonnet`, `opus`) and triggers the story tier when one has changed since the last run. Built from the spike plugin. | about a dollar |

What the story tier asserts, one group per design goal:

- Enforceable: the failing-test commit precedes the implementation commit; the merged commit is the one the reviewer
  approved; no builder-tier agent read a holdout path.
- Low touch: the run reached its first designed human stop with no input.
- Restartable: after the kill, the second process finishes; the end state satisfies the same assertions; and it
  starts fewer agents than a full run, counted from the event log.
- Lean: agent starts and total cost are recorded and fail the run above a budget; every state file is under its
  cap. The budget is 12 agent starts for a story until package E2 has measured a real figure. The pipeline design's
  planning figure is 8 to 12 starts for a realistic story and 70 to 90 for a six-story wave; its "4 to 6" is the
  floor of a simulation in which every step succeeded first time, and a budget of 1.5 times that would fail
  ordinary runs.

Results (cost, turns, agent starts per probe) are uploaded as a JSON artifact on every run. A probe that fails is
retried once; a second failure fails the job.

Step 4 of the rewrite plan, the proof on a small project, is the story tier plus two proofs, and cutover waits for
both.

| Proof | Runs | Asserts |
|---|---|---|
| E3, greenfield | From an empty repository and `tests/e2e/briefs/tally.md`, started by a person and driven by `/vsdd-factory:drive`: brief, `spec`, `plan`, the waves, `verify`, `release`. Then one trivial `feature` after the release | Three typed decisions and no edit of `factory.toml` by a person (the toolchain arrives with the `spec` approval); the scaffold story runs alone in the first wave; the driver resumes after each decision without being typed again; after the release the route is "done", and the later feature runs in delta scope; the prompt count for a wave in the default permission mode; the share of findings first caught at a later perimeter |
| E4, brownfield | Four runs. (1) `tally` with its tests and one known lint failure and no factory state: `setup`, `ingest`, approval of the recovered baseline, a standard-route feature through to release, then a bug fix. (2) The same package with no tests and no CI: the first run schedules the harness and CI chore stories. (3) An import fixture cut from `vantage` or `opnrynkr`: `vsdd import`, then one feature. (4) A two-repository hub with a change that crosses both | The inherited lint failure does not fail any gate, and a new one does; `verify` runs in delta scope; plan coverage asks for no story for an untouched recovered contract; a fast-route story is admitted and delivered; after the import the route is `feature` with the request the importer queued for an epic in flight (steady state if it queued none); the cross-repository change yields one story per repository and the wave closes when both repositories' parts close |

The user asked whether the factory covers adding features to an existing project and to a project whose first
build is finished. E3's last step and E4 are the proof that it does, and they are why the cutover of the eight
real projects, which are all brownfield, is not gated on a greenfield run alone.

Alternative considered: `claude plugin eval`, the platform's eval runner. It gives isolation and graders for free, but
it caps a run at one hour, hides project configuration from the run, and has not been tried with workflows. The spike
proved plain `claude -p --plugin-dir`, so that is the harness. Evals are a reasonable later addition for one narrow
question, whether each model-invocable skill triggers on the prompts it should, and are not part of 1.0.

### 2.7 Continuous integration

`ci.yml` runs on pushes to the integration branch and on pull requests. All jobs except the last need no secret.

| Job | Runner | Does | Time |
|---|---|---|---|
| `static` | Ubuntu | `uv sync --frozen`, then `tools/check.py static`. | under 1 min |
| `unit` | Ubuntu and macOS, each on the Python floor and the newest stable CPython | `uv run --python <v> pytest tests/unit tests/hooks`. | 1 to 2 min |
| `workflows` | Ubuntu, Node from `.nvmrc` | `node --test tests/workflows/`. | under 1 min |
| `validate` | Ubuntu, Claude Code at the floor and at latest | `claude plugin validate --strict plugins/vsdd-factory`, and a comparison of what the validator says the plugin provides (`--json`) against the roster. `--profile directory` runs as a non-blocking report. | under 1 min |
| `e2e-smoke` | Ubuntu, after the four above, only when the changed paths call for it | The smoke tier. Needs the `ANTHROPIC_API_KEY` secret. On a fork, where there is no secret, it is skipped with a visible notice. In this repository a missing secret fails the job. | 5 to 10 min |

`e2e.yml` holds the story and conformance tiers, on a weekly schedule and on manual dispatch, and is callable from the
release workflow. `semgrep.yml` is kept as it is, with the branch names updated; it now scans Python, JavaScript and
shell.

What each old job becomes:

| Old job | 1.0 |
|---|---|
| `validate` (bats subset, shell syntax, manifest JSON, `hooks.json` variants, parse `.lobster`) | `static` and `validate`. |
| `cargo-host` ×2, `build-dispatcher` ×5, `platforms-drift`, `deny-advisories`, `kani.yml` | Gone with the Rust workspace. The plugin has no third-party run-time dependency, so there is no advisory database to check; `uv.lock` covers the three development packages. |
| `policy-15-attestation-location`, `attestation-gate-non-vacuity-controls` | Gone. They policed the old bookkeeping formats. |
| `bats-full-suite`, `bats-wave-handoff (macos)`, `bats-darwin-leg (macos, bash 3.2)` | `unit` on both systems. The macOS leg still matters: the two `sh` files run under a different shell there. |
| `release-branch-guardrail.yml` | Gone. There are no release branches. |
| `semgrep.yml` | Kept. |

A human step at cutover: if branch protection names the old jobs as required checks, the required list must be
changed to `static`, `unit`, `workflows` and `validate` in the repository settings.

### 2.8 Release

A release has no binaries, no bot commit, no release branch and no back-merge. A person does two things: merges a
version pull request, and merges the marketplace pull request.

```
1. uv run python tools/release.py prepare 1.0.1
      edits plugins/vsdd-factory/.claude-plugin/plugin.json  ("version")
      moves the "## [Unreleased]" entries in CHANGELOG.md under "## 1.0.1 — <date>"
   Open a normal pull request with those two files. CI must pass. Merge it.            <- decision 1: cut it

2. release.yml runs on every push to the integration branch. Its first step asks:
   does tag vsdd-factory--v<version in plugin.json> exist?  If yes, it stops in seconds.
   If no:
      verify      tools/check.py static + unit + workflows; the story tier from e2e.yml
      tag         claude plugin tag plugins/vsdd-factory --push      (creates vsdd-factory--v1.0.1)
      release     GitHub Release, notes cut from the CHANGELOG section
      marketplace pull request on bohica-labs/claude-mp setting, for the vsdd-factory entry:
                    "version": "1.0.1"
                    "source.ref": "vsdd-factory--v1.0.1"
                    "source.sha": "<the tag's 40-character commit>"

3. In bohica-labs/claude-mp the pull request runs install-check (below). Review it and merge it.   <- decision 2: publish
```

Properties:

- The marketplace entry is the only switch. Until it merges, no user can receive the release. Reverting it is the
  rollback, and needs no change here.
- The tag and its commit are pinned together, and a tag is never moved. If a release is bad, the next one has a new
  number.
- Every job is idempotent: an existing tag, release or pull request is detected and skipped. If `verify` fails, the
  fix is a normal pull request, and the push that merges it runs the release again at the new tip.
- The story tier is a model-driven gate and can fail for no good reason. A manual run of `release.yml` accepts
  `skip_e2e: true`. That is a decision by a person and is visible in the run's inputs.
- A missing `CLAUDE_MP_PAT` secret fails the `marketplace` job with the two-line manual edit in its message. The old
  job skipped itself quietly.

`install-check` is a new workflow in `bohica-labs/claude-mp`, run on every pull request and push there. It is the
check that would have caught the five releases (rc.2 to rc.6) that installed nothing and reported success:

```sh
export CLAUDE_CONFIG_DIR="$(mktemp -d)"                 # a clean room; the runner's own state is untouched
claude plugin validate .
claude plugin marketplace add "$PWD"
for p in $(jq -r '.plugins[].name' .claude-plugin/marketplace.json); do
  claude plugin install "$p@claude-mp"
  manifest=$(find "$CLAUDE_CONFIG_DIR/plugins/cache/claude-mp/$p" -path '*/.claude-plugin/plugin.json' | head -1)
  [ -n "$manifest" ] || { echo "install of $p populated nothing"; exit 1; }
  claude plugin validate "$(dirname "$(dirname "$manifest")")"
done
```

It also asserts that the installed manifest's `version` equals the entry's. It needs no API key if plugin commands
work without credentials, which is to be confirmed (2.13).

The first release under this procedure is `1.0.0`. There are no pre-releases of 1.0: until cutover, the new plugin is
tried with `--plugin-dir`, which needs no publishing. After 1.0.0 the version follows semantic versioning over what a
project depends on: the state layout and project configuration, the names of skills and workflows, and the `vsdd`
commands.

### 2.9 Cutover

#### The repository and the marketplace

| Step | Who | What |
|---|---|---|
| X0 | builder | Rehearse. In `~/Dev/vantage` (its old state file is 109 bytes, so it is effectively empty), run `claude --plugin-dir <repo>/next/plugins/vsdd-factory`. Confirm the hooks stay inert on old state and the session starts cleanly with the stale `agent` setting present. Nothing is installed or changed. |
| X1 | person | Merge a pull request in `claude-mp` that changes the entry's `ref` from `main` to `v1.0.0-rc.25`. This is the same commit, so nothing changes for anyone, but installs no longer follow `main`. |
| X2 | person | Tag `archive/rc-line` at the tip of `origin/develop` and push it. |
| X3 | builder | The promotion commit on `rewrite/v1`: delete the old tree, `git mv next/* .`, delete `.github/workflows/next.yml`. Then `uv run python tools/check.py` must pass from the root. `git diff --stat -M` should show renames and deletions only. |
| X4 | person | Open the pull request to the integration branch. Update the required checks. Merge. |
| X5 | person | Release 1.0.0 (2.8). The marketplace pull request also rewrites the entry's description. Merge it after `install-check` passes. |
| X6 | person | If the answer to open question 1 is yes: fast-forward `main` to the release commit, make it the default branch, delete `develop`, and close the open dependency pull requests and feature branches that target Rust code. |
| X7 | person, with the tool | Migrate the projects (below). |
| X8 | builder | When the last project is done and `rewrite/STATE.md` says so, delete `rewrite/` in one commit. |

#### The eight projects

`tools/cutover.py` lives in this repository and is not shipped in the plugin; it is needed eight times on one
machine. It reads `~/.claude/plugins/installed_plugins.json`, so it finds the installs itself.

`uv run python tools/cutover.py plan` changes nothing and prints one row per project:

- the installed version and scope;
- the old state: branch, size of `STATE.md`, uncommitted changes, unpushed commits;
- story worktrees, and how many have uncommitted work (`vantage` has 98 worktrees, `prism` has 10);
- the stale `agent` setting;
- residue it will not touch: old process terms in the project's `CLAUDE.md`, project hooks that call into `.factory/`,
  and servers in the project's `.mcp.json` that the plugin now bundles;
- whether the project is already done.

`uv run python tools/cutover.py apply <project>` does the mechanical part. It is idempotent, so a failed run is
simply run again:

1. Stop, with a list, if the old state has uncommitted changes or any story worktree has uncommitted work. In-flight
   work is finished or abandoned on rc.25 first. `--force` overrides.
2. Back up `.claude/settings.local.json` beside itself and remove the `agent` key from it.
3. Run `claude plugin update vsdd-factory@claude-mp --scope project` in the project, and confirm 1.0.0 is recorded.
4. Print the residue report and the one command to run next in a session there: `/vsdd-factory:setup`, which
   shows the import dry run and then gives the person the `vsdd import` command. Where the project's `CLAUDE.md`
   holds old process sections (`prism` and `pregolya` have about 600 lines of them), the report says so and
   points at the offer `setup` makes: with the person's confirmation, those sections move to
   `docs/archive/CLAUDE-pre-1.0.md`, so the session that drives the pipeline is not loaded with rules for agents
   and commands that no longer exist. The cutover tool itself still does not edit a project's `CLAUDE.md`.

`apply <project> --retire` instead uninstalls the plugin for that project and removes the two settings. It leaves the
project's old state alone.

The safety property that makes this low-risk is one the hooks and state designs must provide (section 5, H4 and S1):
**a project that has pre-1.0 state and no 1.0 state is treated as not initialised**. The hooks do nothing there
except add one line to the session briefing that names the migration step. Then a project that reaches 1.0 by
accident, for example through a fresh clone, is inert and says why.

Order, from the facts above: `vantage` first as the pilot, then `opnrynkr` (14 KB of old state), `vitrand`,
`secops-factory`, `prism`, `hylo`, then `flowloom` (281 KB) and `pregolya` (342 KB) last. All eight should be done
within fourteen days of the first, for the reason in risk 6.

Things a person will see once and should expect: a prompt for the three optional API keys the first time the plugin
is enabled (they are stored per user, so once covers all eight), and an approval prompt per project for each bundled
server.

Not offered by default: a second marketplace entry, `vsdd-factory-legacy`, pinned to the rc.25 tag, for a project
that must stay on the old line for a long time. The platform allows an entry name that differs from the manifest
name. It is a fallback to add only if open question 3 is answered that way.

Other machines. Anyone else on a release candidate follows `docs/migrating-from-rc.md`: remove the `agent` setting,
update the plugin, run the import or initialise command. The briefing line above points them there.

### 2.10 Supporting tools

All supporting tools are one command, `vsdd`, implemented in `<plugin>/py/vsdd/cli/`, one module per subcommand, on
the same shared core as the hook runner. `<plugin>/bin/vsdd` puts it on the Bash tool's `PATH` in every session where
the plugin is enabled:

```sh
#!/bin/sh
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
exec sh "$root/py/launch.sh" cli "$@"
```

A bare command name matters because a workflow script's prompts cannot contain the plugin's path: scripts get no path
substitution. (The survey also returns the plugin root, which prompts use for method file paths.) Conventions for
every subcommand: `--json` on anything that reads; exit 0 for success or "yes", 1 for a negative answer (a check
failed, a lock is held, a verdict is stale), 2 for a usage or environment error, including a missing `uv`; outside an
initialised project, everything except `init`, `import`, `doctor`, `obs`, `hooks selftest` and `--version` says so
and exits 2. There is no exit code 3: the canary command that used it is gone. Every command takes the global
option `-C <dir>`. A long command (a gate that builds or tests, `story deliver`, `release`) runs its work as a
detached job and re-attaches: it prints its result, or `state: "pending"` and the same command to run again.

Ownership. This table lists the operator tools, which this design owns. The commands the pipeline calls (survey,
checkpoint, every gate, jobs, verdicts, story delivery, release, the registers) belong to the state design and are
listed in `state.md` section 2.16 and in section 4.9 of the blueprint. Rows below that name a state command are
thin faces kept here for the mapping from the old tools.

The person's terminal. `<plugin>/bin/` is on `PATH` only for the Bash tool, and the plugin's cache path changes with
every version. `vsdd doctor --install-shim`, which `setup` offers with the person's confirmation, writes
`~/.local/bin/vsdd`: a few lines of `sh` that find the plugin directory to use and run its `bin/vsdd`. It does not
use a recorded path, which changes with every update, or the newest directory in the plugin cache, which may be
another project's version: projects can pin different tags, and during the cutover the rc.25 directories have no
`bin/vsdd` at all. It reads `~/.claude/plugins/installed_plugins.json` and takes the `vsdd-factory` entry whose
project path is the current directory or an ancestor of it, and failing that the user-scope entry. It then checks
that the install path has `bin/vsdd`; if it does not, it prints the command that reinstalls or updates the plugin
for this project and exits 2. So the person-only commands (`vsdd decide`, `vsdd import`, `vsdd init --fresh`,
`vsdd config set`, `vsdd lock release --force`) always run the code of the version this project uses. With it,
`vsdd decide`, `vsdd lock release`, `vsdd config set`, `vsdd explain` and `vsdd doctor` work in a terminal, which
is where the designs send a person when a session cannot help. Every message to a person names the in-session
form first (`/vsdd-factory:status --fix`) and the terminal form second, because the first needs no shim.

What a sandboxed Bash tool must allow. The tool and the hook runner need these, and `vsdd doctor`, run through the
Bash tool by `setup`, tests each one and names the one that fails:

| Need | Why |
|---|---|
| Write access to `.factory/` | State, records, the event log |
| Write access to `.worktrees/` | Story, integration and named-spike worktrees |
| Write access to the `uv` cache directory (`uv cache dir`) | `uv run` opens it on every hook and tool call |
| Write access to `~/.cache/vsdd-factory/` | Python bytecode, kept out of the installed plugin |
| Network to the git remote | Fetch, push of story branches and of the state branch |
| Network to `api.github.com` | `gh`: pull requests, CI status, the merge |

A missing Python 3.11 or newer is not something the plugin fixes. `vsdd doctor` starts through the launcher with
`--no-python-downloads`, so it cannot start on the very machine that lacks one; the launcher's message names the
terminal command, `uv python install 3.12`, and that is the one fix. There is no `vsdd doctor --fix`.

| Command | Does | Replaces |
|---|---|---|
| `vsdd doctor` | Checks `uv`, a Python at the floor that `uv` can find without the network, git, `gh` and its login, the Claude Code version against the floor, the project's state (none, pre-1.0, 1.0), and that the event log is writable. Warns about Docker and Node, which only the observability stack and the `npx` servers need. | New. It is the first thing `docs/troubleshooting.md` tells a person to run, in place of the diagnostics section of the old `CLAUDE.md`. |
| `vsdd doctor --install-shim` | Writes the terminal shim. `doctor` also checks each sandbox need in the table above (it is run through the Bash tool by `setup`), the attribution setting, that `commands.run` takes its port from `{port}` or `$VSDD_PORT`, tracked product files that fall in no path class, Node for the export tools, and whether the hooks are live: run through the Bash tool it looks for the caller marker of its own call, as the survey does; run in a terminal it says the hooks cannot be checked from there. Only the main session may run `doctor` through the Bash tool. | New. |
| `vsdd status [--json] [--wave W]`; `vsdd status --fix` | Prints the pipeline status: the same text as the session briefing, plus wave readiness, the lock and its holder. It is computed from the records and writes nothing. `--fix` repairs and commits, and only the main session and the clerk may run it. A thin face on the state design's status function. | `factory-dashboard`, `wave-state`, `factory-lock-status.sh` |
| `vsdd events top`, `recent`, `grep`, `rules`, `stats`, `reasons` | Canned queries over the event log. | `factory-query` |
| `vsdd events replay [SESSION]` | Plays back one session's hook activity in order. | `factory-replay` |
| `vsdd events report daily`, `weekly`, `range` | Markdown reports. | `factory-report` |
| `vsdd events durations`, `summary`, `open` | Agent run times from start and stop pairs. | `factory-sla` |
| (no successor) `emit-event` | There is no `vsdd events emit`. Nothing in 1.0 called it: the hook runner and the tool write events in process, through one writer. With no shell entry point, only they can write the log, so no list of reserved event types is needed to protect it. | `emit-event` |
| `vsdd obs up`, `down`, `status`, `logs`, `dashboard`, `register`, `unregister`, `list`, `reset`, `telemetry` | Runs the observability stack (below). | `factory-obs` |
| (library, no subcommand) the governed merge | A function inside `vsdd story deliver`. It passes `--match-head-commit <sha>` to `gh pr merge` with the branch's current head, and only when that head's `git patch-id` equals the one the converged verdict covers. So a clean rebase, which changes the commit and not the patch, still merges, and GitHub itself refuses if the branch moved after the tool looked. It takes the merge strategy from the project configuration. It is not a subcommand, because a public `vsdd pr-merge` would let any agent with a shell merge with only a head-commit check. | `check-stale-verdict.sh`, `enforce-merge-strategy.sh` |
| (library, no subcommand) worktrees | One library, used by `vsdd survey` (create the story's worktree and branch, or return the existing one), by the two commands that admit a story written after the survey (`vsdd gate plan --story`, `vsdd story new-fix`), and by `vsdd story deliver` (verify the head, remove the worktree). One creator, so paths cannot disagree. It is built in package S3. | `resolve-worktree-identity.sh`, and the git commands the old workflows had an agent type by hand |
| `vsdd export pdf <file.md> [--css <path>]` | Converts a Markdown report to PDF: Markdown to HTML, then headless Chromium through the pinned Playwright package. Needs Node; `vsdd doctor` reports whether it is available. Package T5 proves the engine first. | The missing script of `generate-pdf` |
| `vsdd render diagram <file>` | Renders a Mermaid file to SVG or PNG through a pinned package run with `npx`; exports Excalidraw and draw.io files when their command-line tools are installed, and otherwise says the source file is the deliverable. | The missing render script of `excalidraw-export` |
| `vsdd lock release [--force]` | Breaks the project lock that stops two sessions changing pipeline state at once. `vsdd status` shows the lock, and `vsdd status --fix` releases one whose holder is dead, so there is no `lock status`. Taking the lock is automatic and belongs to the state design. | `factory-lock-status.sh`, `factory-unlock-decide.sh`, `factory-lock-acquire-precheck.sh`, `factory-lock-write.sh` |
| `vsdd validate [PATH...]` | Checks artifact frontmatter and size caps against the state design's schemas. The same module backs the hook rule on writes, and it can run in a project's own CI. A register over its limit (deferred items, on-demand reports) is a warning with exit 0, in every mode, so it can never turn a project's CI red. | `validate-template-compliance.sh` |
| (no successor) `compute-input-hash` | There is no `vsdd stale`. No input hash is stored per artifact or per stage. An approval is bound to a digest of what was approved, so a changed specification makes its decision stale; after approval a contract changes only through an amendment, whose change analysis names the stories it affects. | `compute-input-hash` |
| `vsdd repos scan [--list] [--count]` | Finds the repositories of a multi-repo project and prints their dependency graph as JSON, reading `Cargo.toml`, `package.json`, `pyproject.toml` and `go.mod`. | `multi-repo-scan` |

`research-cache` has no successor command. The `researcher` has no shell, so it could not call a cache tool; the
dated reports under `.factory/research/` are the cache, and it reads them first. The spike runner, which does have
a shell, is answered from the spike ledger when its question was asked before.

Two other old tools have no successor. `lobster-parse` read `.lobster` files, which no longer exist. `factory-cas-push.sh`
pushed the old state branch safely; whether anything like it is needed depends on where the state design keeps state
(section 5, S6).

One honest limit. The governed merge only governs if a hook rule blocks a bare `gh pr merge` in an initialised
project; the hook design's `merge-guard` does. The events that matter are written whether or not an agent
co-operates: blocks and agent starts and stops by the hook runner, and gate runs, measured commits, pull requests
and merges by the tool.

#### The observability stack

It ports whole, to `<plugin>/observability/`, and stays opt-in.

| Part | 1.0 |
|---|---|
| `docker-compose.yml`: Loki, the readiness helper, the OpenTelemetry collector, Prometheus, Grafana and its image renderer | Ported. Image versions are re-pinned at build time. `vsdd obs` always passes `-p observability`, the project name the old stack got by accident from its folder name, so the user's existing data volumes carry over. |
| `otel-collector-config.yaml` | Ported, with its file receiver pointed at the 1.0 event log's location and field names (section 5, H3). |
| `loki-config.yaml`, `prometheus-config.yaml`, three Grafana provisioning files | Ported unchanged. |
| `claude-cost.json`, `claude-overview.json` | Ported unchanged. They read Claude Code's own telemetry, not factory events. |
| `factory-overview.json`, `factory-today.json`, `factory-prs.json`, `factory-subagents.json`, `factory-roi.json` | Ported, with queries re-pointed at the 1.0 event names. |
| `factory-obs` (482 lines of bash) | Ported to Python as `vsdd obs`. |
| The registry, `~/.config/vsdd-factory/watched-factories` | Kept, same path and format. `vsdd obs up` inside an initialised project registers it automatically, which removes a manual step. |
| The generated compose override | Moved from inside the plugin directory, which is replaced on every update, to `~/.config/vsdd-factory/obs/`. |
| Sending events elsewhere (the old Datadog, Honeycomb, HTTP and gRPC sink crates) | Those crates were linked into the dispatcher and never switched on, so no user capability is lost. In 1.0 the hook runner writes one local file and never opens a connection. Anyone who wants events elsewhere adds an exporter to the collector, which is what a collector is for. `docs/observability.md` shows how. |
| Turning on Claude Code's own telemetry for a project | New: `vsdd obs telemetry` writes the five environment settings into the project's `.claude/settings.local.json`. The old guide had people do it by hand, and only `prism` has it. |

### 2.11 Documentation

Principles: hand-written pages never state a count of agents, skills, hooks or anything else; everything that lists
shipped parts is generated from those parts; nothing documents a mechanism that does not exist.

| File | For | Content | Replaces |
|---|---|---|---|
| `README.md` | everyone | What it is, prerequisites, install, the first command, a map of the docs. Under 150 lines. | The old README, whose version and five counts were all wrong. |
| `plugins/vsdd-factory/README.md` | someone looking at an installed copy | Thirty lines and a link. | |
| `docs/getting-started.md` | new user | Claude Code at the floor, `uv`, git, `gh`. Install. Initialise a project. What the session briefing tells you. `vsdd doctor`. | `getting-started.md`, the activation steps |
| `docs/concepts.md` | new user | The method in 1.0: the specification chain, stories and waves, tests before code, adversarial review and the convergence rule, holdout scenarios, spikes, and a glossary. | `VSDD.md`, `CONVERGENCE.md`, `AGENT-SOUL.md`, `FACTORY.md` (1,092 lines about a different platform), `glossary.md` |
| `docs/running-the-pipeline.md` | user | Each workflow command, when to use it, where it stops for a decision, how to resume, how to start a spike, the `drive` skill, and a tested recipe for running `maintain` and `discover` on a schedule (cron or launchd running `claude -p "/vsdd-factory:maintain"` in the project with the Workflow tool allowed). | `workflow-modes.md`, `pipeline-paths.md`, `pipeline-overview.md`, seven phase guides, `cross-cutting-skills.md` |
| `docs/configuration.md` | user | The project configuration file, the three API keys, turning a rule off or tuning it. | `configuration.md`, `policy-reference.md` |
| `docs/observability.md` | user | The event log, `vsdd events`, `vsdd obs`, the dashboards, sending events elsewhere. | `observability.md`, `observability-sinks.md` (an empty skeleton) |
| `docs/troubleshooting.md` | user | "A tool call was blocked": how to read the reason. `uv` missing. The plugin did not load. Old state found. | `troubleshooting.md`, the hook diagnostics section of the old `CLAUDE.md` |
| `docs/migrating-from-rc.md` | release-candidate users | The steps in 2.9. | `migrating-from-0.79.md`, `semver-commitment.md` |
| `docs/reference/agents.md`, `skills.md`, `workflows.md`, `rules.md`, `cli.md`, `configuration-keys.md` | lookup | **Generated** by `tools/gen_reference.py` from agent and skill frontmatter, workflow `meta`, the rule registry, the `vsdd` argument parser and the configuration schema. Checked for drift. | `agents-reference.md`, `commands-reference.md`, `hooks-reference.md`, `templates-reference.md`, `id-reference.md` |
| `docs/architecture.md` | contributor | The parts, how one story travels through them, and a short list of the decisions that shaped 1.0, each with its reason. Distilled from the blueprint. | `plugin-marketplace-architecture.md` (its lesson survives as four lines in `RELEASING.md`), `authoring-hooks.md`, `porting-bash-hook-to-wasm.md`, `dispatch-package-authoring.md`, two audit documents |
| `CONTRIBUTING.md` | contributor | The layout, the one check command, how to add an agent, a skill, a workflow, a rule or a subcommand, and the four test layers. | The old file (Rust toolchain, bats) |
| `RELEASING.md` | releaser | Section 2.8, about fifty lines, and why the marketplace is a separate repository. | 326 lines |
| `CHANGELOG.md` | everyone | Starts at 1.0.0. | 364 KB |
| `CLAUDE.md` | agents working on this repository | Below. | 47 KB |

Dropped with no successor: `visual-companion.md` and `bash-portability.md` (their content, if kept, lives in the
skill), `not-portable.md`, `v1.0-index.md`, `lessons-learned/`, `demo-evidence/`.

#### The replacement `CLAUDE.md`

The old file is 47 KB and is loaded into every agent that starts in this repository. Almost all of it is process: the
principle essay, a 35-row routing table, the commit choreography, the decision-number chains, hook diagnostics. None
of that applies to a repository that is a plugin with tests. The replacement is about forty lines. Skeleton:

```markdown
# vsdd-factory

A Claude Code plugin. The plugin is `plugins/vsdd-factory/`. Everything else in this repository builds,
tests, documents or releases it.

## Check your work
    uv run python tools/check.py        # static checks, unit tests, workflow tests; about a minute
Run the end-to-end tests only when asked. They start real model sessions and cost money.

## Where things are
| Path | What |
| plugins/vsdd-factory/ | the plugin: agents/, skills/, workflows/, hooks/, py/, bin/, templates/, observability/ |
| workflow-lib/prelude.js | shared workflow code, stamped into every workflow |
| tests/ | unit/, hooks/, lint/ (pytest); workflows/ (node --test); e2e/ |
| tools/ | build.py, check.py, gen_reference.py, release.py |
| docs/ | documentation; docs/reference/ is generated |

## Rules the checks enforce
- Shipped Python uses the standard library only and never opens a network connection.
- Do not edit between `// >>> stamped:prelude` and `// <<< stamped:prelude`. Edit workflow-lib/prelude.js,
  then run `uv run python tools/build.py`.
- Do not edit docs/reference/. Run `uv run python tools/gen_reference.py`.
- Adding or removing an agent, skill or workflow also means editing tests/lint/roster.toml.
- A hook timeout is in seconds. A gate blocks with exit code 2 and its reason on stderr.
- A workflow script cannot import, read files, run commands, or call Date.now() or Math.random().

## Git
Commit or push only when asked. No AI attribution in commit messages. Never use --no-verify.
Never force-push main. Never move a release tag.

## Releasing
See RELEASING.md.

## What this repository is not
It is not itself a factory project. There is no pipeline state here to update after a change.
```

The rewrite-mode version, used at the root from the first work package until promotion, is fifteen lines: read
`rewrite/STATE.md` first; work only under `next/`; everything else is the system being replaced, to be read and not
edited or obeyed; the check command; the git rules.

### 2.12 Build plan

**Superseded by section 9 of the blueprint**, which holds the build plan for all five subsystems, including the
packages this table did not have: schemas and policy tables (C1 and C2), survey, decisions, importer and registers
(S3 to S6), the measuring tool (G1 to G7), and export and render (T5). After the second pass the blueprint's plan
has 74 packages. It differs from this table in these ways: B4 is split into B4a (the prelude build) and B4b (the
`hooks.json` build, after H1), which removes a dependency cycle; there is no C3, because the route and the
preconditions are part of S3; T2 is merged into T3, T4 into W12, D1 into D2 and W8 into W7; S2, G4, G6, A2, A6 and
K3 are split; B8 is limited to what a toy probe can settle; and E4, the brownfield proof, is added and gates the
cutover with E3. The table below is kept for the detail of the packaging packages, whose ids the blueprint reuses
where it can.

Packages are in dependency order. Each is sized for one builder and one reviewer. Packages in another designer's area
are shown coarsely, so that the dependencies are visible; their designs split them further.

| ID | Package | Delivers | Depends on | Area |
|---|---|---|---|---|
| B0 | Rewrite-mode root | The fifteen-line root `CLAUDE.md`. The `next/` skeleton: `pyproject.toml`, `uv.lock`, `.nvmrc`, `.gitignore`, and a `tools/check.py` that runs and passes with nothing to check. | blueprint approved | packaging |
| B1 | Manifest and shell | `plugin.json` as in 2.4, plugin README, LICENSE, icon, `.mcp.json`, empty component folders. Lint L-MAN and L-MCP. `claude plugin validate --strict` is clean. | B0; the server list (A1 in section 5) | packaging |
| B2 | Launcher and entries | `py/launch.sh`, a `py/hook.py` that exits 0, `py/cli.py` with `--version`, `bin/vsdd`. Tests: `uv` missing, `uv` found off `PATH`, exit codes. Lint L-SH. | B1 | packaging with hooks |
| B3 | Shared core | `vsdd/project.py`, `config.py`, `frontmatter.py`, `events.py`, with unit tests and the differential frontmatter test. Lint L-PY. | B2; schemas from the state and hooks designs | shared |
| B4 | Prelude build | `workflow-lib/prelude.js` from the pipeline design, `tools/build.py`, tests for write and `--check`. | B0 | packaging with pipeline |
| B5 | Workflow harness | `tests/workflows/harness.mjs`, the schema validator, the two answer generators, the universal tests, and self-tests on two toy workflows. | B4 | packaging |
| B6 | Lint suite and roster | The remaining lint tests and `tests/lint/roster.toml`, filled in from the agents-skills design. | B1 | packaging |
| B7 | CI | `ci.yml` with `static`, `unit`, `workflows`, `validate`. | B2, B5, B6 | packaging |
| B8 | Conformance probes | The spike plugin's checks rebuilt as `tests/e2e/conformance/`, plus every item in 2.13 that a probe can settle. Done early because its answers can change other packages. | B2, B5 | packaging |
| R0 | Marketplace install check | `install-check` in `bohica-labs/claude-mp`. It has no dependency and protects step X1, so it goes first. | none | packaging |
| H1 | Hook runner core | Dispatch, the inert check, failure behaviour, the event writer in use. | B3 | hooks |
| H2+ | Rule groups | Each group of rules with its cases, harvested from the old suites. | H1 | hooks |
| S1 | State layout | Configuration schema, artifact schemas, templates, initialise. | B3 | state |
| S2 | Status, briefing, flush, lock | | S1, H1 | state |
| A1+ | Agents, in batches of about six | | B6 | agents-skills |
| K1+ | Skills, in batches | | B6, the agents they start | agents-skills |
| W1+ | Workflows, one package each, with scenario tests | | B5, the agents each uses, S1 | pipeline |
| T1 | `vsdd doctor` | | B3 | packaging |
| T2 | `vsdd events` | Query, replay, report, durations, emit. About 1,200 lines of bash become about 500 of Python. | B3, H1 | packaging |
| T3 | Observability | The stack moved and re-pointed, `vsdd obs`, the five factory dashboards, lint L-OBS. | T2 | packaging |
| T4 | (moved) | The merge and worktree functions are built inside the delivery package, G4 in the blueprint, with the `gh` test double. | | state |
| T5 | `vsdd status`, `lock`, `validate`, `stale` | Thin commands over state modules; built with S1 and S2 in the blueprint. The blueprint's T5 is export and render. | S1, S2 | state |
| T6 | `vsdd repos scan` | T4 in the blueprint. | B3 | packaging |
| E1 | End-to-end harness and smoke tier | Sample project with its JUnit runner, fixtures, probe runner, the ten smoke probes of 2.6, the `e2e-smoke` job. | H1, S1, one workflow, B8 | packaging |
| E2 | Story tier | Story delivery, kill and resume, the budgets, `e2e.yml`. | the story-delivery workflows, T4, E1 | packaging |
| D1 | Generated reference | `tools/gen_reference.py` and lint L-DOC. | A, K, W, H, T packages | packaging |
| D2 | User documentation | The hand-written pages under `docs/` listed in 2.11. | D1 | packaging |
| D3 | Contributor documentation | Final `CLAUDE.md`, `CONTRIBUTING.md`, `RELEASING.md`, `CHANGELOG.md`, `README.md`. | B7, R1 | packaging |
| R1 | Release tooling | `tools/release.py`, `release.yml`. | B7, E2 | packaging |
| C1 | Cutover tooling | `tools/cutover.py` and `docs/migrating-from-rc.md`. | S1; the answer to the importer question in the state design | packaging |
| X0 to X8 | Cutover | The steps in 2.9. | everything above; step 4 of the plan passed | packaging |

B0 to B8 and R0 are the "conventions" package of step 3 in the plan. B4, B5 and R0 can run alongside B1 to B3. Once
B3, B5 and B6 are in, the H, S, A, K, W and T packages run in parallel.

### 2.13 Not yet proven

Each item is documented by the platform or assumed here, and was not exercised by the first spike. B8 or R0
settles each. A second round of probes ran on 2026-11-23, and blueprint section 10 records its results: N1 is
proven in the main session, a dispatched agent and a workflow step, headless (`! vsdd` typed by a person is not,
and the shim covers the person); the others below are still open. Package B8 is limited to the items a toy probe
can settle; the rest belong to R0, the end-to-end packages and T5.
Section 10 of the blueprint is the full list across all five designs; it adds, among others, the Bash timeout and
detached jobs, hook approval of permission prompts inside agents, the prompt-expansion event for `decide`, a skill
starting a workflow, hooks on MCP tools, compaction inside a step, and `! vsdd` in a session.

| # | Assumption | How to test | If it fails |
|---|---|---|---|
| N1 | A file in `<plugin>/bin/` is on `PATH` for Bash calls in the main session, in an Agent-tool subagent, in a workflow step, and under headless `--plugin-dir`. | Conformance probe: `vsdd --version` in each of the four. | Every agent file names the tool by `${CLAUDE_PLUGIN_ROOT}/bin/vsdd`, which the platform substitutes in agent and skill bodies. |
| N2 | A Claude Code below the `engines` range refuses the plugin with a clear message. | Install an older version in a clean configuration directory and load the plugin. | `vsdd doctor` and the session briefing check the version themselves. |
| N3 | `claude plugin validate`, `marketplace add`, `install` and `tag` run in CI with no credentials, and honour `CLAUDE_CONFIG_DIR`. | R0 is the test. | `install-check` runs with a key, or is run by hand before each marketplace merge. |
| N4 | A `git-subdir` entry whose `ref` is a tag and which also sets `sha` installs correctly. | R0, on the X1 pull request, which pins `v1.0.0-rc.25`. | Pin by `sha` alone. |
| N5 | `claude plugin tag` works for a plugin in a subdirectory and produces `vsdd-factory--v1.0.0`. | `--dry-run` on a clean tree. Today it was only seen refusing a dirty one. | `git tag -a vsdd-factory--v1.0.0`. |
| N6 | A bundled server whose key is not configured stays quiet. | Load the plugin with no keys set and read the session's errors. | Remove that server from `.mcp.json` and document a project-level entry instead. |
| N7 | A session starts cleanly when the `agent` setting names an agent that no longer exists. | Step X0. | The cutover tool removes the setting before the update, which it does anyway. |
| N8 | Updating one project does not strand the other seven on a cache directory marked for cleanup. | After the first `apply`, look for `.orphaned_at` in the rc.25 cache directory. | Finish all eight inside fourteen days. The inert-on-old-state property covers a miss. |
| N9 | End-to-end runs can use cheap models. Either `CLAUDE_CODE_SUBAGENT_MODEL` and `CLAUDE_CODE_EFFORT_LEVEL` override a step's own `model` and `effort`, or the workflows accept a test switch. | Conformance probe. | Section 5, P3. Without either, the smoke tier costs several times more. |
| N10 | The harness's copy of the platform's behaviour is right. | The conformance tier, one probe per row of the table in 2.6. | Fix the harness. A wrong row is a harness bug, not a design problem. |

---

## 3. Mapping from the old system

Marks: **ported** (same capability, new form), **merged** (folded into a named 1.0 item), **replaced** (a different
mechanism now does the job), **dropped** (gone, with the reason).

### Packaging and manifests

| Old item | Mark | 1.0 |
|---|---|---|
| `.claude-plugin/plugin.json` (8 fields, version written by a bot) | ported | Section 2.4. The version is edited in a release pull request. |
| Marketplace in a separate repository, `git-subdir`, `ref: main` | ported | Same repository and source type. `ref` becomes a tag, with `sha`. |
| Install and update commands | ported | Unchanged, with `bohica-labs/claude-mp`. |
| `hooks.json.template`, five `hooks.json.<platform>` files, a git-ignored `hooks.json`, `scripts/generate-hooks-json.sh` | replaced | One checked-in `hooks/hooks.json` (hooks design). |
| The `activate` and `deactivate` steps, and the `agent` setting they wrote | dropped | Plugins need no activation, and workflows do the orchestrating. The cutover removes the setting. |
| `hooks/dispatcher/bin/<platform>/` and `hook-plugins/*.wasm` | dropped | Nothing is compiled. |
| `hooks-registry.toml`, `resolvers-registry.toml` | replaced | The rule registry in Python (hooks design). |
| `rules/` (10 files that no mechanism ever loaded) | merged | Content worth keeping goes into agent and skill files (agents-skills design) and `docs/concepts.md`. |
| `config/artifact-path-registry.yaml` | replaced | State design. |
| `fixtures/smoke-project/` (a Rust crate) | replaced | `tests/e2e/sample-project/` (Python). |
| Plugin `docs/` (6 files) | merged | `docs/concepts.md`. `FACTORY.md` and `not-portable.md` describe another platform and are dropped. |
| Repo `.claude/settings.json`, `scheduled_tasks.lock`, `pr-reviews/`, `.lazyclaude/` | dropped | Stray or obsolete. |
| Repo `.mcp.json` with live keys | replaced | Keys go into `userConfig`, in the system credential store. |

### `bin/` (21 tools)

| Old tool | Mark | 1.0 |
|---|---|---|
| `check-stale-verdict.sh` | merged | `vsdd story deliver`, which recomputes the verdict and uses `gh pr merge --match-head-commit`. |
| `compute-input-hash` | dropped | No stored input hashes and no `vsdd stale`. The decision digest makes an approval stale when the approved content changes. |
| `emit-event` | dropped | The hook runner and the tool write events in process; there is no `vsdd events emit`. |
| `enforce-merge-strategy.sh` | merged | `vsdd story deliver`. |
| `factory-cas-push.sh` | dropped | It served the orphan state branch. If the state design keeps a separately pushed branch it returns as a subcommand (S6). |
| `factory-dashboard` | merged | `vsdd status`. |
| `factory-lock-acquire-precheck.sh` | merged | Taking the lock is automatic (state design). |
| `factory-lock-status.sh` | merged | `vsdd status`, which shows the lock and its holder. |
| `factory-lock-write.sh` | merged | Automatic taking and renewal (state design); `vsdd lock release`. |
| `factory-obs` | ported | `vsdd obs`. |
| `factory-query` | ported | `vsdd events top`, `recent`, `grep`, `rules`, `stats`, `reasons`. |
| `factory-replay` | ported | `vsdd events replay`. |
| `factory-report` | ported | `vsdd events report`. |
| `factory-sla` | ported | `vsdd events durations`, `summary`, `open`. |
| `factory-unlock-decide.sh` | merged | `vsdd lock release [--force]`. |
| `lobster-parse` | dropped | There are no `.lobster` files. Workflows are checked by the harness and the platform's validator. |
| `multi-repo-scan` | ported | `vsdd repos scan`. |
| `research-cache` | dropped | No researching agent has a shell to call it. The dated reports under `.factory/research/` are the cache. |
| `resolve-worktree-identity.sh` | merged | Library functions inside `vsdd survey` and `vsdd story deliver`, which create, verify and remove story worktrees. |
| `validate-template-compliance.sh` | merged | `vsdd validate`. |
| `wave-state` | merged | `vsdd status --wave`. |

### `tools/observability/` (15 files)

All ported; see the table in 2.10. The sink crates were never switched on and are dropped.

### `scripts/`, `ci/`, `benches/`

| Old item | Mark | 1.0 |
|---|---|---|
| `scripts/bump-version.sh` | ported | `tools/release.py prepare`, which also sets the version. |
| `scripts/check-changelog-monotonicity.sh` | merged | Lint L-DOC. |
| `scripts/check-platforms-drift.py`, `ci/platforms.yaml` | dropped | There is no platform matrix. |
| `scripts/check-shakedown-window.sh` | dropped | A gate of the release-candidate line. |
| `scripts/generate-hooks-json.sh`, `generate-registry-from-hooks-json.sh`, `legacy/` | dropped | One checked-in `hooks.json`. |
| `benches/legacy-adapter-latency.sh` | replaced | The hook latency test in `tests/hooks/`. |

### Tests

| Old item | Mark | 1.0 |
|---|---|---|
| 2,279 bats tests; over 1,000 Rust tests | dropped as code | They test deleted code. Their cases for ported rules and tools are harvested (2.6). |
| Parse check of every `.lobster` file | replaced | The universal workflow tests. |
| No test ever ran the plugin inside Claude Code | new | Layer 4. |

### CI and release

| Old item | Mark | 1.0 |
|---|---|---|
| `ci.yml` (10 required checks, up to 50 minutes) | replaced | `ci.yml` (4 required checks, minutes) and `e2e.yml`. |
| `kani.yml`, `release-branch-guardrail.yml` | dropped | No Rust; no release branches. |
| `semgrep.yml` | ported | Unchanged apart from branch names. |
| `release.yml`: validate, build-binaries, commit-binaries, release, bump-marketplace, sync-develop | replaced | verify, tag, release, marketplace. Three of the six jobs existed only because of binaries. |
| `RELEASING.md`: release branch, `--merge` never `--squash`, tag at `main`, bot retag, back-merge, cache-schema check | replaced | Two merges (2.8). The invariants that had to be remembered are gone, not documented. |
| `1.0.0-rc.N` versions | replaced | `1.0.0`, then semantic versioning. |
| `CLAUDE_MP_PAT` | ported | Still needed. Its absence now fails the job. |

### Documentation

See the last column of the table in 2.11.

---

## 4. Interfaces I provide

| # | Interface | Detail |
|---|---|---|
| I1 | Paths | The layouts in 2.3. During the rewrite every path is under `next/`. The plugin is always loaded for testing with `--plugin-dir <root>/plugins/vsdd-factory`. |
| I2 | Manifest facts | Plugin name `vsdd-factory`. Floor `>=2.1.331`. Three `userConfig` keys, reaching hook processes as `CLAUDE_PLUGIN_OPTION_PERPLEXITY_API_KEY`, `CLAUDE_PLUGIN_OPTION_TAVILY_API_KEY`, `CLAUDE_PLUGIN_OPTION_CONTEXT7_API_KEY`, and `.mcp.json` as `${user_config.<key>}`. |
| I3 | The launcher | `sh "<plugin>/py/launch.sh" <entry> [args]`, where the entry is `hook` or `cli`. It finds `uv` (on `PATH`, then in `~/.local/bin`, `~/.cargo/bin`, `/opt/homebrew/bin`, `/usr/local/bin`), exits 2 with a one-line message that carries a terminal command if there is none, sets `PYTHONPYCACHEPREFIX`, and runs `uv run --quiet --no-config --offline --no-python-downloads --script "<plugin>/py/<entry>.py"`. The `hook` entry takes a mode and does not `exec`, as `hooks.md` section 2.2 specifies; the `cli` entry may `exec`. Neither downloads a Python. |
| I4 | The `vsdd` command | The operator subcommands of 2.10 and the pipeline-facing subcommands of `state.md` 2.16, `--json`, the global option `-C <dir>`, and exit codes 0, 1 and 2. It is on `PATH` for the Bash tool wherever the plugin is enabled, and in a person's terminal through the shim. Prompts, agents and skills may name any subcommand; lint L-XREF checks each mention. |
| I5 | Prelude stamping | The markers, the layout rule for `meta`, the constraints on the prelude, and `tools/build.py [--check]` (2.5). |
| I6 | The workflow harness | `runWorkflow(name, { args, agents, workflows, maxAgents })` returning `{ result, error, calls, logs, phases }`, and the five universal tests every workflow must pass (2.6). |
| I7 | Lint | The checks in 2.6. A file that fails one cannot merge. |
| I8 | The roster format | `tests/lint/roster.toml` as sketched in 2.6. |
| I9 | Test homes | `tests/unit/`, `tests/hooks/` with `payloads/`, `tests/workflows/`, `tests/e2e/` with `fixtures/<scenario>/`. One command: `uv run python tools/check.py`. |
| I10 | End-to-end probes | A probe is a prompt, a fixture name, the allowed tools, a budget and a Python assertion over the result, the files, the git log and the event log. Other subsystems add probes by adding files. |
| I11 | Generated reference | `tools/gen_reference.py` reads agent and skill frontmatter, workflow `meta`, the rule registry, the `vsdd` argument parser and the configuration schema. |
| I12 | Release and cutover commands | `tools/release.py prepare <version>`; `tools/cutover.py plan`, `apply <project> [--retire] [--force]`. |
| I13 | Observability | The registry `~/.config/vsdd-factory/watched-factories`, compose project name `observability`, and the collector's reading of the event log. |

---

## 5. Interfaces I need

Each line says what I need and how a mismatch would show up.

### From the hooks design

| # | Need | Mismatch shows as |
|---|---|---|
| H1 | The runner as an importable standard-library package inside the plugin, with one entry script, and agreement that `bin/vsdd` uses the same launcher. My proposal is `py/launch.sh`, `py/hook.py`, `py/vsdd/`. If the hooks design names them differently, I need the three paths. | L-HOOK (a command in `hooks.json` points at a missing file); the launcher tests in B2. |
| H2 | The Python floor. I assume 3.11, for `tomllib`. | The `unit` matrix and the roster's `python_floor`. |
| H3 | The event log: its path pattern inside a project, and its fields. I assume one JSON object per line in daily files, with at least `ts` (ISO 8601 with offset), `type` (dotted), `severity`, `rule`, `reason`, `session_id`, `agent_type`, `tool`; and that there is one writer, a function the hook runner and the tool call in process. I also need the log on by default at a fixed path, or an environment variable that redirects it, so tests can read it. | `vsdd events` tests; the collector configuration; the dashboards; every end-to-end assertion. |
| H4 | The inert rule, and one addition: a project with pre-1.0 state and no 1.0 state counts as not initialised, and the briefing adds one line naming the migration step. | Smoke probe 1; step X0. |
| H5 | A rule that blocks a bare `gh pr merge` in an initialised project and names the governed command. Met by `merge-guard`, which names `vsdd story deliver`. | The story tier's "merged commit is the reviewed commit" assertion. |
| H6 | A way to list rules: name, event, blocks or advises, one-line description. | `docs/reference/rules.md` generation; the meta-test in `tests/hooks/`. |
| H7 | The format of a recorded payload and of a rule test case. | `tests/hooks/` cannot be laid out. |

### From the state design

| # | Need | Mismatch shows as |
|---|---|---|
| S1 | How code tells the three cases apart: no state, pre-1.0 state, 1.0 state. One function in the shared core. | `vsdd doctor`, `tools/cutover.py plan`, smoke probe 1. |
| S2 | The project configuration file: its path and a schema as data in the shared package. I need these keys to exist, under whatever names: test command, build command, base branch, merge strategy, and where the event log goes. | `vsdd story deliver`, the sample project's configuration, `docs/reference/configuration-keys.md`. |
| S3 | Artifact frontmatter schemas and size caps as data, for `vsdd validate`. Staleness is not recorded per artifact or per stage, and there is no `vsdd stale`: the state design binds an approval to a digest of what was approved. | `vsdd validate` on the fixtures; the story tier's size-cap assertion. |
| S4 | The lock: where it is, what it records, who takes it. If there is no lock because concurrent change is prevented some other way, `vsdd lock` is dropped and that mechanism carries the capability. | `vsdd lock` tests. |
| S5 | The status function that produces the briefing text, callable from `vsdd status`. | `vsdd status` and the briefing would disagree. |
| S6 | Whether state is pushed anywhere on its own, apart from the product's commits. This decides whether anything replaces `factory-cas-push.sh`. | A missing or needless subcommand. |
| S7 | The answer to the importer question, and the name of the command a person runs after `cutover.py apply`. | Step 4 of `apply` prints the wrong thing. |
| S8 | Where templates live inside the plugin (`<plugin>/templates/`). There is no research cache in 1.0. | L-XREF. |

### From the agents-skills design

| # | Need | Mismatch shows as |
|---|---|---|
| A1 | The list of bundled servers. My default is Perplexity, Tavily, Context7 and Playwright. `.mcp.json`, `userConfig` and `network.hosts` follow this list. | L-MCP. |
| A2 | The roster as data: each agent with its tier; each skill with who may invoke it; and the four budgets. | L-ROSTER, L-AGENT, L-SKILL, L-BUDGET fail on the first file. |
| A3 | The standard agent template: the sections every agent has. | L-AGENT. |
| A4 | How agents name research tools, given that a server's tool names change when the user already has the same server (2.4). I recommend agents do not pin those names in `tools:`. | An agent with no research tool on some machines. No test here can see that; it needs a decision. |
| A5 | The names of the initialise and status skills (`setup`, `status`), the decision and driver skills (`decide`, `drive`), and the spike command (the `spike` workflow). | The smoke probes; `docs/getting-started.md`. |
| A6 | If a skill ships a Node application, that it installs into a cache directory on first use and keeps its `package.json` out of the plugin root. | L-MAN. |

### From the pipeline design

| # | Need | Mismatch shows as |
|---|---|---|
| P1 | The prelude's content, within the constraints in 2.5. | `tools/build.py --check`; the harness load test. |
| P2 | Workflow names and which calls which. | L-ROSTER; L-XREF's one-level check. |
| P3 | A way to run workflows on cheap models for tests. Settled by the blueprint: when `VSDD_TEST_TIER` is set, `vsdd survey` returns a tier override and the step wrapper uses it. | The cost of the smoke tier. |
| P4 | The story worktree convention (path and branch name) and the default merge strategy. | The worktree and merge functions inside `vsdd survey` and `vsdd story deliver`. |
| P5 | Settled by the blueprint: no. Story delivery is one command, `vsdd story deliver`, run by a clerk; the worktree and merge code are library functions inside it and inside `vsdd survey`, not subcommands. | |
| P6 | Whether delivery can run with no GitHub remote. If it can, the end-to-end tests use that mode. If not, they use the `gh` test double, which is my default and needs nothing from the pipeline design. | None; this only removes work. |
| P7 | The typical number of agent starts for one story, for the lean budget. | The story tier's budget assertion has no number. |
| P8 | The scenario list for each workflow: resume points, gates, the convergence cap, spike limits. | Only the universal tests would exist. |

---

## 6. Open questions for the user

These questions are carried into section 2 of the blueprint as decisions D25 to D30.

1. **One long-lived branch?** Today `develop` is the default branch and `main` holds releases. With releases as tags
   and the marketplace pinning a tag, `main` has no job. *Recommendation: yes.* At cutover, fast-forward `main` (it
   is already an ancestor of `develop`), make it the default, and delete `develop`. If you prefer to keep `develop`,
   nothing in the design changes except the branch name CI watches, and `main` simply stops being used.

2. **Pay for end-to-end tests in CI?** They need an `ANTHROPIC_API_KEY` secret and are billed to the API, not to a
   subscription. Rough figures, to be measured in step 4: a few dollars per smoke run, tens of dollars per story run.
   *Recommendation: yes*, with smoke on pull requests that touch the plugin and the story run at each release and
   weekly. If no, the same tests run on your machine before each release and the release gate becomes a checklist
   item.

3. **What does each of the eight projects do?** Migrate now, finish in-flight work on rc.25 first, or retire.
   *Recommendation:* run `tools/cutover.py plan`, pilot on `vantage`, migrate all eight within two weeks, and add no
   legacy marketplace entry. Whether old state is imported or re-initialised is the state design's question.

4. **Should this repository run the factory on itself after 1.0?** The old engine did, and that is where the
   bookkeeping swamped the work. *Recommendation: no for 1.0.* Develop the plugin with ordinary pull requests and the
   checks above. Revisit once a few real projects have run on 1.0.

5. **Names, and Anthropic's directory.** Use `Bohica Labs` as author and `bohica-labs` URLs? The old manifest says
   `drbothen`, but the remotes and the marketplace already moved. *Recommendation: yes.* And do you intend to submit
   the plugin to Anthropic's directory? *Recommendation: not for 1.0.* Then the icon can be a generated placeholder.
   If you ever do submit, choose a real icon first, because the directory takes it only once.

6. **Delete old history from the tree?** `docs/demo-evidence/` is 110 MB and the changelog is 364 KB. Both stay in
   git history and under the `archive/rc-line` tag. *Recommendation: delete both from the tree, and keep the
   `factory-artifacts` branch on the remote as a read-only archive.*

---

## 7. Risks

1. **Silent marketplace install failure.** It happened five times in the release-candidate line, and the validator
   passed each time. The mitigation is `install-check` in the marketplace repository, which rests on N3 and N4.
2. **The floor is one version.** Everything was verified on 2.1.331 only, and workflows are a young platform feature.
   An upstream change can break the plugin with no change here. The weekly conformance tier against the newest
   Claude Code is the early warning; it costs about a dollar a week.
3. **`vsdd` on `PATH` inside workflow agents is documented, not tested** (N1). The whole tool design assumes a bare
   command name. The fallback works but puts a long cache path into every agent file.
4. **End-to-end tests are model-driven.** As a release gate they can fail for no reason, and they cost money. The
   override exists and is visible; the temptation will be to use it by habit.
5. **The harness is a second implementation of the platform's rules.** If it is wrong, a workflow passes here and
   fails for real. The conformance tier pins each rule to the real platform, and the smoke tier always runs one real
   workflow.
6. **The shared rc.25 cache directory.** When the first project updates, Claude Code may mark the old directory for
   deletion in fourteen days while seven projects still use it. A project that loses it fetches the current release
   at its next session and lands on 1.0 with old state. That is safe only if the inert-on-old-state property (H4,
   S1) is built. This is the most important dependency this design has on the others.
7. **Residue in migrated projects.** `prism` and `pregolya` each have a `CLAUDE.md` of about 600 lines of old process
   rules, loaded into every agent there; the cutover tool reports this and does not fix it. Until a person edits
   those files, 1.0's agents in those projects are told to follow rules that no longer exist.
8. **Bundled servers.** All eight projects already configure the same servers themselves, so the plugin's copies are
   dropped as duplicates there or run alongside them, and tool names differ between machines (A4). `npx` servers need
   Node, which Claude Code no longer brings with it. The behaviour with no key set is unknown (N6).
9. **The promotion commit deletes about 200,000 lines and 190 MB.** It is mechanical and reversible only through git.
   The archive tag and pinning the marketplace first (X1, X2) are what make it safe; neither may be skipped.
10. **`uv` is a hard prerequisite.** If Claude Code starts with a `PATH` that lacks it, the launcher's fallback
    locations cover the common installs. Beyond that every tool call is blocked with the install message, which is
    the intended loud failure but will be someone's first experience.
11. **Builders will read the old tree** to port content and may carry its vocabulary across. L-DEAD catches the
    listed terms only; reviewers need to watch for old process ideas in new words.
12. **A plugin with a `bin/` folder cannot be installed in claude.ai or Cowork.** This is accepted: 1.0 targets
    Claude Code on macOS and Linux, and the hooks and workflows would not run there anyway.
13. **Required checks.** If branch protection names the old jobs, the cutover pull request cannot merge until a
    person changes the list. It is a one-minute step that is easy to forget and blocks everything behind it.
