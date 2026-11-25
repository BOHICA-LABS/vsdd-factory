---
kind: topic
id: multi-repo
sources: [pipeline.md#2.9.11, BLUEPRINT.md#5.1, BLUEPRINT.md#3.4]
---

# Multi-repository

How work runs across several repositories: the hub, the `repo` argument and stories that cross
repositories. The hub model is defined in [[workflow:multi]], which owns the `[[repo]]` entries,
per-repository work, per-repository waves and gates, and the hub setup; the state-layout side of the
hub model is [[topic:multi-repo-state]].

In one view: the hub repository holds the single state directory and one `[[repo]]` entry per member
repository; member repositories hold code and nothing from the factory; there is one cycle, one
identifier space and one status report. Per-repository work runs the ordinary workflows with
`repo: "<name>"` from the hub. A story belongs to one repository and carries `repo`, so work that
spans two repositories is two stories joined by `depends_on`
([[topic:story-admission]]). Every workflow also accepts `repo`
([[topic:workflow-arguments]]).

## Team use (decision D33; built when the build widens)

Several people on one repository is a different feature from several repositories, and decision D33
covers the first only ([[topic:team-use]]). How the two combine at a hub is not designed and not
proved, so team use cannot be turned on in a project whose `factory.toml` holds a `[[repo]]` entry
([[table:work-claim-checks]], T-SWITCH).
