---
kind: topic
id: governed-delivery
package: G4b
sources: [BLUEPRINT.md#2.2, BLUEPRINT.md#4.11, BLUEPRINT.md#4.12, state.md#2.8, pipeline.md#2.9.1]
---

# When a delivered story waits for a person

Defaults (decision D15): "Yes: `delivery.merge = "auto"`. Three things always wait for you: a story that
touches a protected path (`.github/**`, `.env*`, `CODEOWNERS`, release configuration); a change that adds a
dependency or upgrades one beyond what the approved architecture lists; and anything a project configures."

The keys live in `[delivery]` of the project configuration ([[config:delivery]]): `mode` (`pr` or `local`),
`ci` (`required` or `none`), `merge` (`auto` or `human`), `merge_strategy`, `protected_paths`,
`dependency_changes` (`human` or `auto`), `ci_wait_minutes`.

## The merge decision

"A delivered story waits for a person: protected path, dependency change, or `delivery.merge = "human"`"
opens a `merge` decision ([[record:decision]], [[table:decision-kinds]]). Its choices:

- `approve`: "the next `story deliver` performs the merge, and a dependency it names is recorded as
  approved."
- `reject`: "the pull request is closed and the story is descoped."

"While a product cycle is open" a dependency a person approves in a `merge` decision cannot be written into
`dependencies.toml` during a build: "Dependency detection in `vsdd story deliver` therefore reads
`dependencies.toml` and the `dependencies` of every approved `merge` decision that is newer than the latest
approved `spec` decision. The next `spec` or amendment run has the architect fold them into
`dependencies.toml`." ([[artifact:dependencies]])

## The governed merge

The merge itself is a library inside [[command:story-deliver]], not a subcommand: "It passes
`--match-head-commit <sha>` to `gh pr merge` with the branch's current head, and only when that head's
`git patch-id` equals the one the converged verdict covers. So a clean rebase, which changes the commit and
not the patch, still merges, and GitHub itself refuses if the branch moved after the tool looked. It takes
the merge strategy from the project configuration. It is not a subcommand, because a public `vsdd pr-merge`
would let any agent with a shell merge with only a head-commit check." A bare `gh pr merge` is blocked in an
initialised project by the hook rule `merge-guard` ([[rule:merge-guard]]).

## The hotfix branch

"A critical bug fix may open a second cycle beside the main one. Its stories are cut from the release tag
and delivered to `hotfix/<version>`; its `next` carries `cycle`, which `verify` and `release` accept; and
when it is released the merge back into the integration branch waits for the integration lock, opening a
`merge` decision if it conflicts." ([[command:release]])

## Feature flags (decision D31; built when the build widens)

A feature flag adds nothing to the list of deliveries that always wait: a story that ships behind a
flag, turns one on or removes one merges as `delivery.merge` says. Whether the removal of a flag is asked
of a person first is a setting the project configures, `flags.removal`, and the question is put before the
removal is built ([[topic:flag-lifecycle]]).

## Team use (decision D33; built when the build widens)

On a team every story is delivered through a pull request and CI, and the closure records who merged
(`delivery.merged_by`). A pull request the host will merge by itself later is `queued` and opens no
decision; one merged by the host or outside the tool is adopted only when the change that landed is the
reviewed one ([[table:team-delivery-checks]], T-DELIVERY). In `pr` mode the merge of a hotfix branch
back into the integration branch is a pull request as well ([[command:release]]). Where this file names
one host's tool, that tool is reached through the delivery's one host module
([[command:story-deliver]]). The factory enforces nobody's right to merge: the repository's rules do.
