---
kind: topic
id: hook-runner
sources: [hooks.md#1, hooks.md#2.4, hooks.md#2.3, BLUEPRINT.md#5.4]
---

# The hook runner

In 1.0 the hook subsystem is one small Python program that uses only the standard library. A 70-line
POSIX shell launcher starts it through `uv`, and one checked-in `hooks/hooks.json` wires it to seven
events. It replaces the Rust dispatcher, 37 WebAssembly plugins and 36 bash hooks (79 registry
entries) with 17 rules.

A rule is a plain Python function. It receives the hook payload plus facts about the project and
returns allow, block, advise, add context, or permit. Gates run before the action, so an invalid write
never lands.

In a project with no factory configuration file the launcher exits before Python starts (about 8 ms).
Every failure of the launcher or of a gate rule blocks and says why. A gate can be relaxed only by a
person, and every relaxation is announced at each session start.

The rules cover general safety, information walls keyed on the calling agent, artifact validity, tests
before code, the governed merge, decisions only a person can record, spike bounds and containment,
storing every workflow step result, the session briefing and the pre-compaction flush. One JSON-lines
event log feeds the observability stack and the `vsdd explain` command.

## Layout

The layout follows [[packaging:python-package]]: one standard-library package, `vsdd`, shared by the
hook entry and the `vsdd` command.

```
<plugin>/hooks/hooks.json     generated from the rule table, checked in
<plugin>/py/
  launch.sh                   the launcher (see [[topic:hook-launcher]])
  find-root.sh                project discovery, sourced by launch.sh
  hook.py                     hook entry: inline metadata, version check, sys.path, call vsdd.hooks.main
  cli.py                      entry for the vsdd command (packaging design)
  vsdd/
    project.py                shared core: project root, state directory, worktrees, story and spike detection
    config.py                 shared core: loads and validates the project configuration, including [hooks]
    frontmatter.py            shared core: the small frontmatter reader (a strict YAML subset)
    events.py                 shared core: the one event writer and reader (see [[topic:event-log]])
    hooks/
      main.py                 read stdin, build the context, select rules, run them, emit, log
      model.py                Ctx, RuleEntry, the outcome types (the registry row is the declaration)
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

The frontmatter reader accepts one `---` fenced block at the top of a file containing scalars
(string, integer, boolean, null, a bare ISO date), flow lists (`[a, b]`), block lists of scalars, and
one level of nested mapping. Anything else raises an error with a line number, which the `artifacts`
rule reports as a block.

Its semantics are fixed by one rule: refused, not misread. On every input it accepts, its value equals
what YAML 1.1 as PyYAML's `safe_load` reads, because that is what the differential test compares with
([[packaging:tests-python]]). Every plain scalar that YAML 1.1 would read as something other than it
looks is refused, with the line number and the instruction to quote it: `yes`, `no`, `on`, `off` and
any other spelling of a boolean or null than `true`, `false`, `null` and `~`; numbers with a leading
zero, a sign, an underscore or a base prefix; floats; `12:30`; and a plain scalar holding `#` or `: `.
An agent's artifact is therefore never read differently from how it looks.

## Dispatch

Selection and the loop: for the event and tool at hand, `registry.select` yields the rules; each runs
inside a deadline; a rule crash or the deadline turns into the rule's declared failure behaviour
([[topic:hook-failure]]); `emit.finish` logs and writes stdout and stderr, returning 0 or 10. The
model of what a rule is and may return is [[topic:rule-model]]. The wiring is [[topic:hook-wiring]].

Interpreter floor: Python 3.11, the first release with `tomllib` in the standard library. The runner
has no third-party imports.

Budgets: a rule may start at most one `git` subprocess, with a 1.5 s timeout, and no rule makes a
network call. Git facts that can be read from files (`HEAD`, worktree pointers) are read from files.
Gate calls are expected in 55 to 70 ms warm; the performance test fails if the median `gate` call
exceeds 120 ms, stricter than the 400 ms ceiling of [[packaging:tests-python]].

Time budget per call:

| Path | Claude Code timeout | Runner's own deadline | Expected time |
|---|---|---|---|
| Not a factory project | n/a | n/a | about 8 ms (measured), no uv, no Python |
| `gate` | 10 s | 4 s, then a block that names the rule that was running | 55 to 70 ms warm (measured 57 ms on a prototype with uv start, JSON, TOML and four module imports); about 190 ms on the first call after an install |
| `brief` | 15 s | 8 s, then the fixed fallback briefing | 60 to 90 ms |
| `notify` on `PreCompact` | 30 s | 25 s, of which the state flush may use 20 s | 80 ms plus the flush |
| `notify` elsewhere | 5 s (1.5 s on `SessionEnd`) | 1 s | about 55 ms |
