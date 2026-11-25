---
kind: rule
id: write-scopes
package: H3
sources: [hooks.md#2.10, hooks.md#2.11.2, hooks.md#2.7, BLUEPRINT.md#5.4, BLUEPRINT.md#4.3]
---

# write-scopes

Enforces the write half of the information walls, keyed on the calling agent's type, from
[[table:walls]] and [[table:path-classes]].

| Registration | |
|---|---|
| Kind | gate |
| Runs on | `PreToolUse`, tools `Edit`, `Write`, `MultiEdit`, `NotebookEdit`, `Bash` |
| Result | `Block` |
| Fails | closed — "the write would land; the agent can simply retry once the fault is fixed" |
| Relax | locked (decision D12) |

## The checks

`write-scopes` evaluates, for a target path P and the caller's row:

1. P is in a class whose `write` is `none` or `code`: block. This makes reference code read-only, and
   makes evidence, the decision and closure records and the project configuration unforgeable by a tool
   call.
2. P's class is not one the row's `write` list grants, and P is not in `tmp`: block, with the class's
   `route` sentence as the fix. This is what makes a reviewer read-only apart from its report and stops
   an architect from rewriting a contract. For a path in the product tree, "P's class" is the one class
   the precedence of [[table:path-classes]] gives it, not every class whose globs match. So
   `src/parser/tests.rs` is in `tests`: the test-writer may write it, before and after red, and the
   implementer, whose row grants `source` and `manifests`, may not. A product path in no class is
   blocked for every agent with the `[paths]` line to add. A grant with `@<method>` counts only in a
   story whose resolved method for the caller's own stage (its `impl_method` for the implement
   agent, `tests_method` for the tests agent) is that one.
3. The grant is `:create` and P already exists: block.
4. The call is a `Bash` command that writes into the state directory (a redirect, `tee`, `sed -i`,
   `perl -i`, `cp`, `mv`, `rm`, `touch`, `truncate`, `dd of=`, `ln`): block for every caller, with the
   fix "use the Edit or Write tool, or the vsdd command". A call to the `vsdd` command is not a shell
   write.
5. P contains a state-directory segment but does not resolve to the project's one state directory (the
   classic case is a relative `.factory/...` path written from inside a worktree): block, and print the
   correct absolute path.
6. The row has `story_only` and P is project code outside a story worktree: block. A builder cannot
   change the main checkout or the integration worktree.
7. P is in the state directory and the project's condition is `partial` because the mount test fails
   (the state directory is not a worktree on the state branch): block, and name
   `/vsdd-factory:status --fix`. State is never written into a directory that is not mounted.

Check 5 applies to reads as well: [[rule:read-walls]] blocks a `Read`, `Grep` or `Glob` of a `.factory`
path inside a linked worktree and prints the correct path, so an agent cannot read a stray copy.

## Grant suffixes

Complete list ([[table:walls]]): `:create` (new files only); `:frozen` on `tests`; `:before-red` on
`source`; and `@<method>`, which limits a grant to a story whose resolved method for the caller's own
stage is the one named (`source:create@repo-setup` on the implementer side, `tests:create@repo-setup`
likewise — for `devops`, whose own stage is the implement stage, both match the story's
`impl_method`). `source:create@repo-setup` is what lets `devops` create the first source and test files
of an empty repository in the scaffold chore story, and nowhere else.

## Case table

Cases in `tests/hooks/cases/write-scopes.toml`, because the first wave of every greenfield project
depends on them:

- `devops` creates `Cargo.toml` and `src/main.rs` in a chore story with method `repo-setup` (allowed).
- `devops` creates `src/main.rs` in a chore story with method `ci` (blocked).
- `devops` creates `tests/e2e/smoke.sh` in the chore story with method `repo-setup` (allowed:
  `tests:create@repo-setup` matches the story's resolved implement method, which is the caller's own
  stage, although a chore has no tests stage of its own).
- the spike runner writes any file into a story worktree (blocked: the story worktree is neither the
  worktree that holds its working directory nor one its ledger entry records — the `own-worktree`
  definition in [[table:path-classes]]).
- the implementer adds a dependency line to `Cargo.toml` in a story worktree (allowed; the dependency
  decision comes at delivery).
- the implementer edits `.github/workflows/ci.yml` (blocked, route: a chore story).
- the verifier creates `fuzz/fuzz_targets/parse.rs` in a harness story (allowed).
- the test-writer edits `src/parser/tests.rs` after red (allowed).
- the implementer edits `src/parser/tests.rs` (blocked).
- any agent writes `scripts/build.sh` when no class matches it (blocked, with the line to add).

Limits of the walls are stated plainly in [[topic:information-walls]] and [[topic:risks-hooks]]: the
shell is not sandboxed, so the `Bash` checks are heuristics, and tools the runner cannot inspect are
handled by keeping them off walled agents.

Spike containment (the spike runner may write only its own worktree and `tmp`) is part of
[[rule:spike-bounds]] and the spike row of [[table:walls]].
