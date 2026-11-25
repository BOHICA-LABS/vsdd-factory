---
kind: topic
id: multi-repo-state
sources: [state.md#2.14, BLUEPRINT.md#5.2]
---

# State for a multi-repository project

All state for a multi-repo project, shared or not, lives in one state directory, in the hub repository.

- The hub is the git repository whose `factory.toml` has `layout = "hub"` and `[[repo]]` entries
  ([[config:repo]]). It may be a repository made for the purpose or one of the code repositories. Member
  repositories are cloned at the `path` of their entry (by default `repos/<name>/`, excluded from the hub
  through `.git/info/exclude`).
- Member repositories hold code and nothing from the factory. A session runs from the hub root, and the
  hook runner resolves the project by walking up from the path being touched, so work inside
  `repos/api-server/` is governed by the hub's state.
- There is one identifier space for the whole project. A subsystem and a story carry `repo`. A story
  belongs to one repository; work that spans two is two stories joined by `depends_on`. Cross-repository
  contracts are ordinary contracts, and interface definitions sit in `specs/product/supplements/`.
- Each `[[repo]]` entry may carry its own `branches`, `commands`, `paths`, `release` and `delivery`, with
  the same keys as the top level, so the hub release can ship each repository by its own rules
  ([[config:release]], [[config:delivery]]).
- The order in which repositories are built is not stored. It follows from story dependencies, as waves do
  ([[topic:waves]]).
- Waves are project-wide, and a wave is built and gated one repository at a time, because a `wave` run
  takes the lock of one repository and branch. So a wave has one closure record per repository that has
  stories in it (`units/wave.<W-NN>.<repo>.json`), each with its own gate rounds, and one integration
  worktree per repository (`.worktrees/_integration/<repo>/`). The wave is closed when every one of those
  records is closed.
- A hub is set up deliberately. `vsdd init` does not guess one. `setup --hub` shows what `vsdd repos scan`
  found and passes the confirmed list to `vsdd init --hub`, which writes the `[[repo]]` entries when it
  creates `factory.toml`. On a project that already has a `factory.toml`, setup shows the block for the
  person to paste. `vsdd repos split` ([[command:repos-split]]) remains the case in which a `spec` decision
  creates the members.
- On a fresh clone of the hub the member repositories are missing. `vsdd status` reports each one, and
  `vsdd status --fix` clones it from the `url` in its entry.
- A change that crosses repositories is one story per repository, joined by `depends_on`, on every route.

One state directory is chosen over the old arrangement (a `.factory/` in every repository plus a
`.factory-project/` above them) because it keeps the single-directory rule the hook and pipeline designs
rely on, and because it is what the user's own nine-repository project already does in practice: its state
sits in one directory with a list of repositories. The cost is that a member repository cloned on its own
has no factory state. That is accepted.

How the workflows run across repositories — the hub, the `repo` argument and stories that cross
repositories — is [[topic:multi-repo]].
