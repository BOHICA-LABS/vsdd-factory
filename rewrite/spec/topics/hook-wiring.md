---
kind: topic
id: hook-wiring
sources: [hooks.md#2.3, BLUEPRINT.md#5.4]
---

# Hook wiring

`hooks/hooks.json` is generated from the rule table (`vsdd/hooks/registry.py`) by `tools/build.py` and
checked in. `tools/build.py --check` fails if the checked-in file differs from the generated one.
There are no per-platform variants and no activation step. The runner starts only for the events and
tools in [[table:events]]; every other tool never reaches it. Seven events are wired.

Which event carries which rule is the table's content ([[table:events]]); the rule list itself is
[[table:rules]].

## The command guards itself

The `PreToolUse` handler's shell command guards itself, and it stays inert where it should. If the
launcher is missing (a broken install, or a cache directory that was cleaned up under a running
session), `sh` would exit 127, which Claude Code treats as non-blocking, and every gate would switch
off silently. The guard prevents that, but only in a factory project. `find-root.sh`, which decides
whether a project is a factory project, lives beside the launcher and is unreadable in exactly this
case, so the guard carries the same test inline: it walks up from `CLAUDE_PROJECT_DIR` (or the working
directory) to the home directory or the root, looking for `.factory/factory.toml`. If it finds one it
exits 2 with a reason and the reinstall command. If it finds none it exits 0, so a broken install
never blocks a tool call in a project that was not initialised (decision D24). In `notify` mode the
same guard exits 0. In `brief` mode, in a factory project, it prints a fixed down-briefing as JSON
with `systemMessage`, saying the plugin files are missing and the gates are blocking, so the person
sees why every guarded call fails. The guard does not test linked worktrees or pre-1.0 state; it is
the smallest test that keeps unrelated projects untouched, and a session started inside a linked
worktree of a factory project falls back to exit 0, which the next start from the project root
corrects.

The self-guarding command, as generated into `hooks.json`:

```json
{ "matcher": "Bash|Edit|Write|MultiEdit|NotebookEdit|Read|Grep|Glob|Agent|Task|StructuredOutput|Workflow|WebFetch|WebSearch|mcp__(plugin_vsdd-factory_)?playwright__browser_(navigate|navigate_back|tabs|file_upload)|mcp__(plugin_vsdd-factory_)?(perplexity|tavily|context7)__.*",
  "hooks": [ { "type": "command",
    "command":
    "sh -c 'f=\"${CLAUDE_PLUGIN_ROOT}/py/launch.sh\"; [ -r \"$f\" ] && exec sh \"$f\" hook gate; d=\"${CLAUDE_PROJECT_DIR:-$PWD}\"; while [ -n \"$d\" ] && [ \"$d\" != \"$HOME\" ] && [ \"$d\" != / ]; do [ -f \"$d/.factory/factory.toml\" ] && { echo \"vsdd-factory: plugin files are missing, so the factory gates cannot run. Reinstall the plugin.\" >&2; exit 2; }; d=\"${d%/*}\"; done; exit 0'",
    "timeout": 10, "onFailure": "block" } ] }
```

## Notes on the wiring

- Timeouts are in seconds; a test asserts that no timeout in the file exceeds 60.
- `onFailure: "block"` (Claude Code 2.1.295 or later) makes Claude Code itself treat a hook that
  cannot start, times out, or exits with an unexpected code as a block. It is the second line of
  defence behind the launcher, for a hook that hangs or dies. It is not the defence for missing plugin
  files: the self-guarding command above is, and it is the one that knows whether the project is a
  factory project.
- `StructuredOutput` is a guarded tool, because that call is how a workflow step returns its result
  and `step-results` stores it (`pipeline.md` need H1). A call that carries no step identity passes
  untouched. It is also where a spike runner that is a workflow step is held until it has recorded its
  finding.
- `Workflow` reaches the runner for `auto-allow` only.
- `SubagentStop` is synchronous because `spike-bounds` and `read-walls` can refuse a stop. Its mode is
  `notify`, so a launch failure never traps an agent that is trying to finish. The platform probe
  showed that a refused stop holds an agent that returns text and does not hold a workflow step that
  has already returned a schema result; the rules that need to hold a step act on its
  `StructuredOutput` call.
- `PostToolUse` is not wired. In 1.0 no pipeline agent opens a pull request (the tool does), and a
  recorder there would cost every commit about 55 ms. The tool writes `pr.created` and `pr.merged`,
  and the gates write `commit.made` for the commits they measure.
- `PostToolUseFailure`, `Stop`, `PostCompact`, `PermissionRequest`, `WorktreeCreate` and
  `WorktreeRemove` are deliberately not wired. In short: validation moved before the write, the
  briefing after compaction arrives through `SessionStart` with source `compact` (proven by the
  spike), and a `WorktreeCreate` command hook replaces Claude Code's own worktree creation, so a
  logging hook there would break worktree isolation.
- `async: true` is used only for recorders. The generator refuses to put `async` on a handler that has
  a gate rule, because Claude Code ignores decisions from background hooks.
- The shell form of the command is used because the spike proved it. The path is quoted.
