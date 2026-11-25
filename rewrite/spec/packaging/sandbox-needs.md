---
kind: packaging
id: sandbox-needs
package: T1
sources: [packaging.md#2.10, BLUEPRINT.md#5.5]
---

# What a sandboxed Bash tool must allow

The tool and the hook runner need these, and `vsdd doctor` ([[command:doctor]]), run through the Bash tool by
`setup`, tests each one and names the one that fails:

| Need | Why |
|---|---|
| Write access to `.factory/` | State, records, the event log |
| Write access to `.worktrees/` | Story, integration and named-spike worktrees |
| Write access to `.git/worktrees/` | Git's own files for every linked worktree (index, `HEAD`, logs): a commit in `.factory/`, in a story worktree or in the integration worktree, and `git worktree add` |
| Where commits are signed: the gpg home and its agent socket | The local merge commit and any commit made in the product, which the person's git signs |
| Write access to the `uv` cache directory (`uv cache dir`) | `uv run` opens it on every hook and tool call |
| Write access to `~/.cache/vsdd-factory/` | Python bytecode, kept out of the installed plugin |
| Network to the git remote | Fetch, push of story branches and of the state branch |
| Network to `api.github.com` | `gh`: pull requests, CI status, the merge |

A sandboxed session lacks the `.git/worktrees/` row by default: with the sandbox on and nothing granted, every
write there is refused with `Operation not permitted` (a signed commit fails too), and no setting that grants
it has been shown to work ([[topic:platform-proven-2]], item 41). Until one is, a factory run needs the Bash sandbox off
([[doc:platform]], "Headless runs"). The doctor tests the row by taking the index lock of an existing linked
worktree, the `.factory` mount's, as a commit does. Adding a worktree is not the test: a refused
`git worktree add -b` leaves its branch behind.

A missing Python 3.11 or newer is not something the plugin fixes. `vsdd doctor` starts through the launcher with
`--no-python-downloads`, so it cannot start on the very machine that lacks one; the launcher's message names the
terminal command, `uv python install 3.12`, and that is the one fix ([[packaging:cli-entry]]). There is no
`vsdd doctor --fix`.

Built and tested by package T1, "the sandbox needs each tested" ([[package:T1]]).
