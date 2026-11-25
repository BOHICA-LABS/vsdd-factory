---
kind: config
id: flags
package: F1
sources: [decisions.md#D31]
---
# [flags]

Built when the build widens past the thin slice ([[packaging:build-plan]]). Decision D31; the capability
from end to end is [[topic:feature-flags]].

Whether the product uses feature flags, when a story must ship behind one, and where the checks look.
Keys, types and defaults: [[table:flag-config-keys]]. The checks that read them: [[table:flag-checks]],
[[table:flag-story-checks]] and [[table:flag-release-checks]].

## Who writes it

Two tables of `factory.toml`, each with one writer.

- `[flags]` is a person's, like the rest of the file: `enabled`, `require`, and any override.
- `[flags.approved]` is the survey's and nobody else's. What the survey puts there, from the approved
  proposal and from what the person typed when approving it, is F-FILL ([[table:flag-checks]]). This
  widens the third case of [[topic:project-configuration]]; the table is code's own, so no value a person
  set is ever rewritten.

A check reads a key from `[flags]` when a person wrote it there and from `[flags.approved]` otherwise. So a
flag system that a later approved proposal changes reaches the gates at the next survey, and a person can
still correct one value in a line. `vsdd status` and the briefing list every override; what the next
`spec` packet shows of them is in [[topic:flag-system-choice]]. No agent can set a key: no tool call can
write the file.

## The three states

Computed from the two tables, never written.

- **Off.** `enabled` is false, or nothing has given it a value. No flag check runs, no story may name a
  flag (F-REQUIRE), and nothing stops or asks. This is every project that never asks for flags.
- **Requested.** A person wrote `enabled = true` and no approved proposal names a flag system. Everything
  is as in off, so no gate fails and an open cycle finishes. `vsdd status`, the briefing and the release
  packet say "flags requested, flag system not chosen", and the architect proposes the system the next
  time it writes architecture, in a `spec`, `ingest` or `feature` run. A person who wants it sooner files
  the change request; it runs when the open cycle has closed, like any request.
- **On.** `enabled` is true and the approved proposal names the system (`flags.decision`). Every check
  runs.

The project has answered once `enabled` has a value from either table. An answer of false stands: no
agent proposes flags again until a person asks for them in the brief, in a request, or by writing
`enabled = true`.

## `require`: when a story must ship behind a flag

The story-writer applies it to each story it writes; `vsdd gate plan` holds it to the result
(F-REQUIRE). "The product exists" means the cycle's baseline is not empty: the code was ingested,
imported or already released ([[record:baseline]]).

| Value | A story must name a flag when |
|---|---|
| `marked` | Never. A story is behind a flag only where the story-writer judges it risky or part of work that merges before it is complete. |
| `high-risk` | It is a `feature` or `port` story, its `criticality` is `critical` or `high`, and the product exists. |
| `existing` | It is a `feature` or `port` story and the product exists. |
| `all` | It is a `feature` or `port` story. |

A story the setting requires may carry `flag_exempt` with a reason instead of a flag
([[artifact:flag]]). Under every value the story-writer may put a story behind a flag that the setting
does not require.

## Patterns

Five keys hold regular expressions: `read_patterns`, `bypass_patterns`, `test_patterns`,
`definition_patterns` and `key_patterns`. The architect writes them for the project's languages and a
person approves them with the toolchain file, as `[paths]` is approved. The tool holds no language list
and no vendor name.

- **Syntax.** Python's `re` module, which is the tool's own. A named group is written `(?P<key>...)`.
- **Where.** A pattern is matched against the whole text of a file, so a match may span lines: read and
  bypass patterns in files of the path class `source`, test patterns in files of the class `tests`,
  definition patterns in files that match `flags.definitions`, key patterns in the lines a diff adds.
- **Added lines.** A match belongs to a story's added lines when its span includes at least one
  character of a line the story's diff adds ([[command:gate-green]]).
- **Compiling.** The loader compiles every pattern when it reads the section. One that does not compile
  is a configuration error that names its key.
- **Time.** The scan of one file is stopped after five seconds. The scanner runs outside the gate's own
  process so that it can be stopped.

What a check does with a pattern that does not compile, or a file whose scan was stopped, is the "cannot
measure" rule in the header of [[table:flag-checks]].

What a pattern cannot see is stated once, in the `unmeasured` cell of each check that uses it.

## Keys and local runs

`key_env` names the variables that hold the flag source's secret keys, and `key_patterns` gives the shape
of those keys. The names have three readers: the gates (F-KEY, [[table:flag-story-checks]]); the hook rule
`secrets` ([[rule:secrets]]); and `vsdd status` and the briefing, which warn while one of the variables is
set in the session's environment, because an agent's own shell inherits it.

`override_env` names the one variable through which flag values are given from outside: to the product's
wiring for a local run, and to the suite's setup for a test run. Who sets it, and to what, is F-KEY.

## More than one component or repository

One project has one flag system and one register. In a repository with more than one toolchain
([[config:component]]) the pattern and glob lists are unions: each component adds its own entries, and
a pattern written for one language matches nothing in another. In a hub the proposal may give a member
repository ([[config:repo]]) its own values of some keys: which keys, and where a person overrides them,
is the row `flags.repo.<name>` of [[table:flag-config-keys]]. Every other key is the project's. A story's
flag is checked in that story's repository, and work on one flag in two repositories is two stories joined
by `depends_on`, as any cross-repository change is ([[topic:multi-repo]]).
