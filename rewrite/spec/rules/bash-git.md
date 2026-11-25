---
kind: rule
id: bash-git
package: H2
sources: [hooks.md#2.10, hooks.md#2.11.1, hooks.md#2.11.6, BLUEPRINT.md#2.1]
---

# bash-git

Carries over the old `verify-git-push` and `block-ai-attribution`, with the same reason codes so the
old test cases can be harvested.

| Registration | |
|---|---|
| Kind | gate |
| Runs on | `PreToolUse`, tool `Bash` |
| Result | `Block` |
| Fails | closed — "the actions are irreversible" |
| Relax | yes; `attribution = "block"` (commit messages and pull request text) is the shown key (hooks.md 2.6) |
| Spike exemption | no (the safety rules still apply inside a spike) |

Adopted decision D11: the old git rules hold as default gates — no AI attribution in commits or pull
requests, no `--no-verify`, no force push, no direct push or commit to protected branches — on by
default, each relaxable per project by a person.

## What it checks

Blocked:

- `git push --force` and `-f` (with `--force-with-lease` allowed on branches other than the integration
  and release branches).
- `--no-verify`, `-n` on commit, `--no-gpg-sign`, and `-c core.hooksPath=`.
- AI attribution (the eleven old patterns) in a commit message or in pull request text.
- A command family the caller's row does not grant (see [[table:command-families]] and
  [[table:walls]]).
- For spikes: a `spike/*` branch leaving its sandbox — nobody may merge, cherry-pick or rebase a
  `spike/*` ref onto another branch, or open a pull request whose head is one; the spike runner's row
  grants no push, no merge and no pull request.

Pushes and merges onto the integration and release branches belong to [[rule:merge-guard]].

## Parsing

Like [[rule:bash-destructive]], it works on a parsed command. It reads what the command points at: a
commit made with `git commit -F <file>` has its message file scanned for attribution; the old hook
scanned only the command string.
