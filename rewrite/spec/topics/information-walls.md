---
kind: topic
id: information-walls
sources: [agents-skills.md#2.9, BLUEPRINT.md#4.3, hooks.md#2.11.2, hooks.md#2.7]
---

# The information walls

The walls are the design's answer to "who may know what": they keep hidden scenarios out of builder
context, keep reviewers independent of each other, and keep every agent inside its write scope. They
are enforced by the hook runner from data and records, keyed on the calling agent's type. This topic
is the map; the definitions live in the tables and rules.

## The four tables, and where they live

| Table | Defined in | Enforced by |
|---|---|---|
| Path classes | [[table:path-classes]] (the content of `classes.toml`, which is authoritative on names) | [[rule:read-walls]], [[rule:write-scopes]], [[rule:tests-first]] |
| Wall rows, one per caller | [[table:walls]] (the content of `walls.toml`) | [[rule:read-walls]], [[rule:write-scopes]] |
| Dispatch | the `dispatch` key of a wall row | `read-walls` on `PreToolUse` for `Agent` |
| Command scopes | the `commands` and `vsdd` keys of a wall row, with [[table:command-families]] and [[table:walls]] | [[rule:bash-git]], [[rule:gate-guard]] |

What each wall key means (`read_deny`, `read_allow`, `write`, `story_only`, `dispatch`, `commands`,
`tool_only`, `vsdd`, `spike`, `relaxed_by`) and the write-grant suffixes (`:create`, `:frozen`,
`:before-red`, `@<method>`) are documented in [[table:walls]].

## The walls, in short

From BLUEPRINT.md#4.3 (authoritative for the roster's walls):

- **Holdout.** `holdout` (hidden scenarios, their evaluations, and review findings that quote them) is
  readable only by `product-owner`, `scenario-reviewer` and `holdout-evaluator`.
- **Reviews.** Reviewers cannot read `reviews`.
- **Runs.** No caller can read `runs` (stored step records), the main session included; what a step
  needs from an earlier step arrives in its prompt, and people read findings through `vsdd review
  show`.
- **Code-reviewer.** Reads nothing in the state directory except `evidence` and `guidance` (the
  conventions and review rules it is given).
- **Holdout evaluator.** Cannot read source, tests, CI, stories, or any specification except the
  brief.
- **Scenario reviewer.** Cannot read source or tests.
- **Outside.** Agents with no shell or a `vsdd`-only shell, the clerk and the spike-runner cannot read
  outside the project, the plugin and the temporary directory.
- **The main session** is walled like an agent (decision D6): it cannot read `holdout` or `runs`;
  inside the state directory it writes only `brief` and `deferred`; and it cannot commit on the
  integration or release branch. When it dispatches an agent that can read hidden scenarios, the hook
  checks that agent's final message for scenario text before the message reaches the main session.
  `[hooks] main_session_wall = false`, which only a person can set, relaxes this row alone and is
  announced in every briefing.

## Reads, writes and searches

- "May not read" covers the `Read`, `Grep` and `Glob` tools and any `Bash` command that names a path
  in the class, a `vsdd` read command that prints the class ([[table:walls]] `[reveals]`), and a
  browser tool opening a `file:` address in it. Wherever a row says `holdout`, it covers
  `holdout-evaluations` too.
- How `read-walls` finds the paths a call would reach, per tool, is in [[rule:read-walls]], including
  the ancestor-search case that lets the main session search the project root.
- The seven checks `write-scopes` makes, including shell writes into the state directory, stray
  `.factory` paths in worktrees and the `partial` mount condition, are in [[rule:write-scopes]].
- Builders write project code only inside a story worktree, and only in the classes their row grants.
  A path in the product tree belongs to exactly one class (the precedence of
  [[table:path-classes]]).

## What the walls rest on

- Spike containment: a spike runner inherits the asker's walls, on hook-written facts ([[rule:spike-bounds]]). This is what lets `code-reviewer`, `holdout-evaluator` and `scenario-reviewer` ask safely.
- Step results store walled findings so that derived outputs carry only the walled key ([[rule:step-results]]).
- Every agent with a `read_deny` entry has only tools the runner can inspect, plus tools that cannot
  read project files; a lint test checks this against `walls.toml` (need A2).

## Limits

Stated plainly in [[topic:risks-hooks]]: the shell is not sandboxed, so the
`Bash` checks are heuristics, and tools the runner cannot inspect are handled by keeping them off
walled agents (need A2). A real sandbox is out of scope for 1.0.
