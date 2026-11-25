---
kind: rule
id: read-walls
package: H3
sources: [hooks.md#2.10, hooks.md#2.11.2, hooks.md#2.3, hooks.md#2.7, BLUEPRINT.md#5.4, BLUEPRINT.md#4.3]
---

# read-walls

Enforces the read half of the information walls, keyed on the calling agent's type, from
[[table:walls]], [[table:path-classes]] and [[table:callers]].

| Registration | |
|---|---|
| Kind | gate |
| Runs on | `PreToolUse` for `Read`, `Grep`, `Glob`, `Bash`, `Agent`; also the browser navigation, tab and upload tools in both spellings (they are in the matcher, [[table:events]]); and `SubagentStop` for the final-message check |
| Result | `Block` |
| Fails | closed — "a secret or a holdout scenario cannot be unread" |
| Relax | locked (decision D12) |

## Mechanics

`read-walls` blocks when a path the call would read is in a class listed in the row's `read_deny`,
unless a class that contains the path is listed in `read_allow`.

How "the paths the call would read" is found:

- **`Read`** — `file_path`, literal and with symlinks resolved.
- **`Grep`, `Glob`** — the search root (`path`, default `cwd`). Blocked if the root is inside a denied
  class. If the root is an ancestor of a denied class: the platform probe of 2026-11-23 confirmed that
  `Grep` skips a directory listed in `.git/info/exclude` (the state directory is listed there), so the
  call is allowed for every row whose denied classes all lie under `.factory/`. The weekly conformance
  tier re-checks it; if it ever fails, the call is blocked with the fix "narrow the path". A row that
  denies product classes (the holdout evaluator, the scenario reviewer) is always blocked from an
  ancestor search.
- **`Bash`** — every token that resolves into a denied class; any occurrence of the class's literal
  path prefix in the command text; any token with glob characters that expands (capped at 200 results)
  to a path in the class; a `vsdd` read subcommand whose `[reveals]` entry names a denied class; and
  `npx playwright` or a browser command given a `file:` address in a denied class.
- **Browser tools** (`browser_navigate`, `browser_navigate_back`, `browser_tabs`, `browser_file_upload`,
  in both spellings) — a `file:` address is treated as a `Read` of the path it names, in a navigation
  and in a new tab. An upload from a path in a denied class is blocked. The plugin's own Playwright
  server also refuses `file:` addresses by default, and lint (L-MCP) checks that the option which loosens
  that is absent ([[packaging:plugin-manifest]]). If the user runs their own Playwright server the platform
  may keep theirs and drop the bundled one ([[doc:platform]], item 33); the address check applies to
  both spellings, so the wall does not depend on which server survives. `browser_click` is not matched:
  a Chromium page loaded over `http` cannot navigate to a `file:` address by a click, so there is
  nothing for the rule to see.
- **`Agent`** — the row's `dispatch` list. A factory agent may start only the spike runner, and the
  spike runner may start nothing, so a walled agent cannot send an unwalled helper to read for it. The
  spike runner it starts inherits its walls.
- **The final message of a dispatched agent (`SubagentStop`)** — only when the main session dispatched
  an agent whose row may read `holdout` (`product-owner`, `scenario-reviewer`, `holdout-evaluator`),
  and `main_session_wall` is on. The rule runs the tool's scenario-text check on the agent's final
  message: a message that quotes a hidden scenario is refused with "say what you did without quoting
  scenario text", every time and not only the first, and each refusal is logged. There is no
  second-attempt pass here, unlike the spike stop check, because the text would then reach the main
  session. This closes the path by which the main session, which may not read scenarios, could receive
  them in a reply. A dispatched agent returns text, and the platform probe showed that a refused stop
  holds such an agent.

## The main session's ancestor searches

The main session may not read `holdout`, but it searches from the project root all the time, and
blocking that would be unusable. The options, in order of preference, are: the state design keeps
`holdout` where a recursive search from the project root does not reach it (agents-skills.md
N-STATE-8); or a `.ignore` file in the state directory hides it from ripgrep; or the rule rewrites the
search through `updatedInput` to exclude it. The first is what the state design does, and the probe
confirmed it works: the state directory is in `.git/info/exclude` and `Grep` skips it. The main
session's project-wide searches are therefore allowed.

## Notes

- A `vsdd` read subcommand is treated as a read of the classes [[table:walls]] `[reveals]` lists for
  it, so `vsdd list holdout-scenario` by an implementer is blocked exactly as a `Read` of a scenario file is.
- Check 5 of [[rule:write-scopes]] (a `.factory` path inside a linked worktree) applies to reads too:
  this rule blocks a `Read`, `Grep` or `Glob` of it and prints the correct absolute path.
- The walls as a whole, including their limits (the shell is not sandboxed), are explained in
  [[topic:information-walls]]. How rules are tested, including the case tables, is
  [[topic:rule-testing]].
