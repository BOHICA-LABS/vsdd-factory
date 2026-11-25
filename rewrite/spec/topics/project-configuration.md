---
kind: topic
id: project-configuration
package: S1
sources: [state.md#2.12, BLUEPRINT.md#4.12, BLUEPRINT.md#4.6]
---

# Project configuration

`.factory/factory.toml` is the project configuration file. It exists if and only if the project is
initialised for 1.0. The file is in the `config` path class: a person creates and edits it, and no agent
tool call can write it, so an agent blocked by a gate cannot relax the gate.

## Who may write it, and the three cases in which code does

`vsdd init` creates it by detection and a person edits it afterwards. Code writes it in three stated cases,
by appending or by filling a key that is unset, never by rewriting a value a person set; each is announced in
the next briefing:

1. `vsdd init` and `vsdd import` when they create it.
2. `vsdd repos split`, which appends `[[repo]]` entries ([[config:repo]]).
3. The survey, when an approved `spec` decision covers a toolchain file ([[artifact:toolchain]]) and
   `[commands]` or `[paths]` keys are still unset: it fills them from the toolchain file the `spec` decision
   approved.

Otherwise code only reads it, because the standard library can read TOML and cannot write it back with
comments intact. Every other machine-written file is JSON. Unknown keys and wrong types are errors that the
briefing reports, and the hook design's rule applies: an invalid value never relaxes a gate. The one key
list is [[table:config-keys]]; the sections have a file each, starting at [[config:project]]. A workflow
script cannot read the file: the survey returns the values a run needs in its `config` and `limits` fields
([[schema:survey]]), and `vsdd config --json` prints the whole file as data.

## Detection by init

`vsdd init` writes the first version by detection (`Cargo.toml`, `package.json` scripts, `pyproject.toml`,
`go.mod`, a `Makefile` or `justfile`, the current branch names, the release configuration, whether there is a
remote and whether there is CI) and then runs each detected command once to see that it starts. It detects
the globs of the seven product classes and lists any tracked file that falls in none. `README.md` and
`CLAUDE.md` always land in `paths.docs`. It detects a JUnit
report for the test command (nextest, `pytest --junitxml`, jest-junit, go-junit-report). It checks that
`commands.run`, when set, takes its port from `{port}` or `$VSDD_PORT`. A command it cannot detect is left
empty and listed in the briefing. The `setup` skill shows the detected values to the person, who edits the
file where they are wrong. `--auto-allow <scope>` and `--hub` carry the answers `setup` collected before it
ran `init`, because `init` is the only moment the plugin writes those keys.

## Missing commands are a state, not an error

When `test` or `test_report` is not set, `vsdd status` says "commands not configured", workflows that would
measure (`wave`, `story`, `verify`, `maintain`) refuse to start, and the route says what supplies them: the
`spec` or `ingest` approval for a project that has none yet, or the line to add. The values arrive by
approval: the architect writes `specs/architecture/toolchain.toml` in `spec` (greenfield) or in the
`synthesize` stage of `ingest` (existing code), the `spec` decision approves it with the rest of the
specification, its packet shows the exact lines, and the first survey after the approval writes them into the
unset keys. A project never needs a person to edit `factory.toml` between `spec` and `wave`. Where the
commands cannot work yet, a chore story makes them work: the scaffold story for an empty repository, a test
harness chore story in `E-0` for a codebase with no suite or no JUnit report, and a CI chore story where
`delivery.ci = "none"`.

## The whole key list, as an annotated example

The example below is the whole key list: every section and every key the loader accepts appears in it, with
its default. It is the one key list; an unknown key is an error and two lists would reject each other's keys.

```toml
schema = 1

[project]
name = "acme-api"
languages = ["rust"]
ui = false
layout = "single"                    # single | hub (2.14)

[branches]
integration = "develop"              # stories merge here
release = "main"
protected = ["main"]
story = "story/{id}"                 # branch patterns the hook runner maps back to stories and spikes
spike = "spike/{name}"

[commands]                           # each is one shell string, run from the root of the worktree under test
build = "cargo build --workspace"
test = "cargo nextest run --workspace --profile ci"
test_report = "junit:target/nextest/ci/junit.xml"   # required: the red gate needs per-test results
lint = "cargo clippy --workspace --all-targets -- -D warnings"
format_check = "cargo fmt --check --all"
run = ""                             # how to start the product, for demos and hidden scenarios. It must take
                                     # its port from {port} or $VSDD_PORT; vsdd init and vsdd doctor check that
ui_check = ""
docs_check = ""                      # the documentation check the green gate runs for a docs story
mutation = ""
scan = ""
audit = ""
bench = ""
proofs = ""

[commands.env]                       # optional: environment for every configured command, e.g. a shared build cache
# SCCACHE_DIR = "~/.cache/sccache"

[paths]                              # globs relative to a worktree root. Seven classes cover the product tree.
                                     # A path that matches more than one belongs to the first in this order:
                                     # tests, harness, ci, repo_config, manifests, docs, source (2.4)
tests = ["tests/**", "crates/*/tests/**", "**/tests.rs"]   # so src/parser/tests.rs is a test file
harness = ["fuzz/**", "benches/**", "proofs/**", "**/proofs.rs"]   # proofs, fuzz targets, benchmarks, in-source proof files
ci = [".github/**"]
repo_config = ["rust-toolchain.toml", ".gitignore", ".editorconfig", "clippy.toml", "rustfmt.toml", "deny.toml"]
manifests = ["Cargo.toml", "Cargo.lock", "crates/*/Cargo.toml"]   # build manifests and lock files
docs = ["docs/**", "README.md", "CLAUDE.md"]
source = ["src/**", "crates/*/src/**", "build.rs"]

[delivery]
mode = "pr"                          # pr | local. vsdd init sets local when the repository has no remote
ci = "required"                      # required | none. vsdd init sets none when the repository has no CI; only
                                     # then does "no checks" count as a pass, and a CI chore story is scheduled
merge = "auto"                       # auto | human
merge_strategy = "squash"
protected_paths = [".github/**", ".env*", "CODEOWNERS", "CHANGELOG.md"]   # always wait for a person. vsdd init
                                     # adds the files [release] names: version files, the change log, the
                                     # release workflow
dependency_changes = "human"         # human | auto: a new dependency, or an upgrade beyond dependencies.toml
                                     # and the dependencies approved merge decisions list
ci_wait_minutes = 30                 # the total wait for CI, taken as one job in slices of nine minutes

[convergence]                        # loops; the defaults are the pipeline design's
passes = 3                           # review passes at the story and spec perimeters
rounds = 3                           # review rounds at the wave and release perimeters

[limits]
agents_per_run = 900
agents_per_story = 40                # the budget each child story receives
stories_per_wave_run = 24
step_attempts = 2
delivery_rounds = 3
gate_minutes = 60                    # the longest a gate job may run
parallel_gates = 4                   # how many gate jobs run at once
failed_runs_before_stuck = 2

[spikes]
per_step = 2
per_run = 6
per_day = 20
minutes = 20                         # the time box
max_open = 3                         # named spikes open at once; enforced by vsdd spike open --name
retain_days = 14                     # how long a finished spike's worktree is kept

[holdout]
mean_min = 0.85
must_pass_min = 0.6
stddev_max = 0.15
rotation = 0.8
batch = 12                           # scenarios per evaluator step

[mutation]
kill_rate = { critical = 0.95, high = 0.90, medium = 0.80, low = 0.70 }   # by module criticality

[stops]
plan = "auto"                        # auto | always: when the plan decision opens
plan_ceiling_agents = 600            # with auto, the plan decision opens when the estimate passes this

[evidence]
wave_demo = false                    # a demo at the wave gate
final_demo = false                   # a demo in the release UI review

[maintain]
sweeps = []                          # empty means every sweep that applies; or a list of sweep names
fixes_per_run = 10

[release]                            # detected by vsdd init; run by vsdd release
version_files = []                   # e.g. [{ path = "Cargo.toml", key = "package.version" }]
changelog = "CHANGELOG.md"
tag_format = "v{version}"
publish = []                         # commands, e.g. ["cargo publish"]
github_release = true

# [[discover.source]]                # optional: feedback and analytics pulled by vsdd discover collect
# name = "github-issues"
# command = "gh issue list --state open --json number,title,body,labels --limit 200"

[state]
push = true
lock_stale_minutes = 20
retain_runs_days = 30
prune_closed_cycles = true

[caps]                               # optional: raise a cap to at most twice its default
# story = 20000
# waived = ["specs/architecture/decisions/ADR-052.md"]   # files over their cap after an import (2.15)

[notify]
channel = "none"                     # none | github-issue | command
command = ""

# [[component]]                      # optional: a repository with more than one toolchain
# name = "web"
# paths = ["web/**"]
# commands = { test = "pnpm -C web test", lint = "pnpm -C web lint" }

# [[reference]]                      # optional: repositories studied during ingestion, cloned under .reference/
# name = "superpowers"
# url = "https://github.com/obra/superpowers"

# [[repo]]                           # hub layout only (2.14); written by vsdd init --hub or vsdd repos split
# name = "api-server"
# path = "repos/api-server"
# url = "git@github.com:acme/api-server.git"
# # optional per repository, with the same keys as the top level: branches, commands, paths, release, delivery

[hooks]                              # owned by the hook design; only a person edits it after init
main_session_wall = true             # decision D6. false relaxes the main session's row only, and is announced
log_retention_days = 30              # event files older than this are deleted at session end
# [hooks.rules.<rule>] takes strength, and three rules take keys of their own; all are in the key table

[hooks.rules.auto-allow]
scope = "off"                        # decision D4: off | core | extended. vsdd init writes the answer setup collected
```

## Sections this design owns and sections it hosts

The state design defines and owns: `schema`, `[project]`, `[branches]`, `[commands]`, `[paths]`, `[state]`,
`[caps]`, `[[component]]`, `[[reference]]`, `[[repo]]`.

Sections whose keys and defaults belong to another design, which this file only hosts and validates:

- `[delivery]`, `[convergence]`, `[holdout]`, `[mutation]`, `[limits]`, `[stops]`, `[evidence]`, `[notify]`,
  `[maintain]`, `[discover]` — the pipeline design. As shown above:
  `delivery.{mode, ci, merge, merge_strategy, protected_paths, dependency_changes, ci_wait_minutes}`;
  `convergence.{passes, rounds}`; `holdout.{mean_min, must_pass_min, stddev_max, rotation, batch}`;
  `mutation.kill_rate` by criticality; `limits.*`; `stops.{plan, plan_ceiling_agents}`;
  `evidence.{wave_demo, final_demo}`; `maintain.{sweeps, fixes_per_run}`.
- `[spikes]` — the pipeline and hooks designs: `per_step`, `per_run`, `per_day`, `minutes`, `retain_days`,
  `max_open`. `per_day` and `max_open` are enforced by `vsdd spike open`; the time box by the hook runner.
- `[release]`, `[[discover.source]]` — the pipeline design. As shown above.
- `[hooks]` — the hooks design: `main_session_wall`, `log_retention_days`, and `[hooks.rules.<rule>]` tables
  ([[config:hooks]]). `vsdd init` and `vsdd import` write the `auto-allow` scope once, from `--auto-allow`;
  after that no `vsdd` command an agent can run changes it.

## Feature flags (decision D31; built when the build widens)

Not on the thin slice path. Decision D31 adds one section, `[flags]` ([[config:flags]]). It is not in the
example above, and the loader rejects it as an unknown key, until package F1 adds its keys
([[table:flag-config-keys]]). From then the survey's third case also keeps the table `[flags.approved]`,
which only code writes; what it holds is in [[config:flags]].

## Team use (decision D33; built when the build widens)

Not on the thin slice path. Decision D33 adds one section, `[team]` ([[config:team]]). It is not in the
example above, and the loader rejects it as an unknown key, until package M6 adds its keys
([[table:team-config-keys]]). Code never writes it. Every clone shares the whole file, so nothing that
exists on one machine only may be a value in it.
