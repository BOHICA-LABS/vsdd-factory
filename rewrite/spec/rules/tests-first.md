---
kind: rule
id: tests-first
package: H5
sources: [hooks.md#2.10, hooks.md#2.11.4, hooks.md#2.5, BLUEPRINT.md#5.4]
---

# tests-first

The replacement for the old `red-gate`; implements `pipeline.md` guard G1 and wall W4 and the
write-scope split of `agents-skills.md` (its need N-HOOK-4). The old hook trusted a JSON list of
"currently red" files that agents edited by hand.

| Registration | |
|---|---|
| Kind | gate |
| Runs on | `PreToolUse`, tools `Edit`, `Write`, `MultiEdit`, `NotebookEdit`, `Bash` |
| Result | `Block` |
| Fails | closed — "the gate is the point of the method" |
| Relax | yes (`strength = "advise"` to trial, `off` to skip; see [[topic:rule-tuning]]) |
| Spike exemption | yes |

There is no declaration decorator: the `RuleEntry` row in `registry.RULES` is the rule's one
declaration (HD1, [[topic:rule-model]]), and the module holds `check` and nothing else.
[[topic:rule-model]] shows `tests-first`'s row as its example, with `spike_exempt = True`.

## Evidence split

These delivery-order rules follow the split in `pipeline.md` section 2.2: scripts steer, the tool
measures, hooks enforce. The tool's gate command runs the project's configured commands and writes an
evidence record (gate, subject, commit, passed, measurements) that no agent tool call can write. A hook
then refuses the action the evidence does not support. Nothing here reads a prompt; the old hooks found
the story by running `grep` over the dispatch prompt and the project by looking for `cd <path> &&` in
it.

Evidence must come from a command that sees the real exit code: watching test commands in
`PostToolUse` does not work, because agents routinely pipe test output through `tail`, which hides the
exit code, and a false "green" is the dangerous direction.

Which story a call belongs to: `ctx.story` is the story whose worktree contains the target path, or for
a `Bash` segment its effective directory ([[topic:rule-model]]). The
pipeline creates story worktrees at `.worktrees/<story-id>/`; the rule also accepts any worktree whose
checked-out branch matches the story branch pattern (need P3), so it holds when a worktree was made
another way.

## What it checks

- It applies when `ctx.story` is set, the work is not in a spike, and the story's kind has a red gate
  (`kind_has_red_gate`, read from the story-kind rows of [[table:story-kinds]]).
- Which class a path is in follows the precedence of [[table:path-classes]], as in
  [[rule:write-scopes]]. A file that both a source glob and a tests glob match is a test file. So the
  test-writer is not caught by `red_already_recorded` when it fixes `src/parser/tests.rs` after red,
  and the implementer cannot edit that file under its `source` grant. Three cases hold it: the
  test-writer edits `src/<m>/tests.rs` after red (allowed); the implementer edits it (blocked by
  `write-scopes`); the red gate counts the tests in it.
- A frozen test the test-writer changes after red is recorded by `vsdd gate red --refreeze`, which
  [[rule:gate-guard]] allows for the test-writer's row only. This rule has nothing to do there: the
  green gate compares frozen tests with the latest red or re-freeze record.
- "A passing `red` record for the story" means the latest passing red record on any commit of the story
  branch since its base, which is what the state design's `evidence` function returns for the `red`
  gate. It is not tied to the head commit: the red gate passes on the test writer's commit, and the
  record must keep holding as the implementer commits once per test. The case tables include an
  implementer edit after its own commit (allowed) and a test-writer source edit after red at a later
  commit (blocked).
- For a path in the project's `source` class: a caller whose grant is plain `source` (the implementer,
  and the main session) is blocked until the evidence store holds a passing `red` record for the story
  (`no_red_evidence`). A caller whose grant is `source:before-red` (the test writer, for stubs) is
  blocked once that record exists (`red_already_recorded`).
- For a test file that the red record lists as frozen: only a caller whose grant is `tests:frozen` (the
  test writer) may change it (`red_test_frozen`). Other builders that may write tests, such as the
  verifier in its harness directories, cannot touch the tests that defined the story.
- The implementer cannot weaken any test, because its row does not grant `tests` at all. That is
  `write-scopes`, not this rule.
- An invalid `factory.toml` falls back to the built-in defaults (the invalid-configuration row of
  `topics/hook-failure.md`). For this rule the fallback empties the path-class table, so no write in a
  red-gated story worktree can be classified; instead of letting every write pass unjudged, the rule
  blocks it with `config_invalid` — naming the configuration error and the fix — until the file parses
  again. The scope is the rule's own: one write inside a story worktree while the governing project's
  `factory.toml` is invalid, whichever caller made it. Outside a story worktree the rule does not apply,
  and the other rules' defaults keep the gates at full strength.
- A `Bash` command is held to the same two checks for every path it writes: a redirect target, and the
  path arguments of the shell write commands that [[rule:write-scopes]] lists in its check 4 — `tee`,
  `rm`, `touch`, `truncate`, the in-place files of `sed` and `perl`, the destination of `cp`, `mv` and
  `ln` and the sources of `mv`, and `dd`'s `of=`. So `sed -i` on product source before red, or `cp` over
  a frozen test, is blocked like the edit it stands for. Words that expand at run time (`$`, a backtick)
  name no guessable file and are left out; glob words are expanded, 200 paths at most; a relative path in
  a segment whose directory is unknowable is left out, because the green gate still holds: it needs a red
  record and unchanged frozen files. What a program writes when it runs cannot be seen by a hook; the
  gates cover that: the green gate needs a red record and unchanged frozen files
  ([[command:gate-green]]).

The case tables also hold the `Bash` forms: `sed -i` on a source file before red by the implementer
(blocked), a redirect into a frozen test by the implementer (blocked), and the same `sed -i` after red
(allowed).

## Wave order needs no hook of its own

The old `validate-wave-gate-prerequisite` blocked a dispatch for a later wave while an earlier wave's
gate was pending. In 1.0 the tool creates a story's worktree only when the story is ready (in the
survey, or when it admits a story written after the survey), a builder can write project code only
inside a story worktree (`story_only`), and the tool refuses to close a wave without its evidence. An
agent started out of order has nowhere to write.
