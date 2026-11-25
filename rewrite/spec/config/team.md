---
kind: config
id: team
package: M6
sources: [decisions.md#D33]
---
# [team]

Built when the build widens past the thin slice ([[packaging:build-plan]]). Decision D33; the capability
from end to end is [[topic:team-use]].

Whether more than one clone runs the factory on this repository, and the oldest plugin version that may
write its state. Keys, types and defaults: [[table:team-config-keys]]. The checks that read them are in
[[table:team-checks]], [[table:sync-checks]], [[table:work-claim-checks]] and
[[table:team-delivery-checks]].

## Who writes it

A person, like the rest of `factory.toml`. Code never fills either key, so the cases in which code writes
the file stay three ([[topic:project-configuration]]). `setup` shows the line to add once the team
proof has passed ([[packaging:proof-team]]); until then nothing a person is shown offers it.

## On and off

- **Off** (`enabled` is false or absent). One clone. No survey fetches the state branch, no claim is
  made, no clone record is written and nothing waits on the network that did not wait before. Some
  checks act all the same. The head of [[table:team-checks]] is the one list of them, with the reason
  for each, and this file does not repeat it.
- **On.** Every check of the four tables runs. What the project must have set first is T-SWITCH's list.

Turning it on is one line by a person on the clone that has the project, followed by
`/vsdd-factory:status --fix`, which syncs. That sync gives the clone its record and claims the stories it
has in flight (T-SWITCH). A project with no remote cannot turn it on, because there is nothing to share
through. A clone that was already mounted when the team was formed joins by itself at its next push,
and claims what it has in flight in the same way (T-SOLO).

## What is shared and what belongs to one clone

| Shared, on the state branch | Belongs to one clone, never pushed |
|---|---|
| Every key of `factory.toml`, this section included | The name the clone gives the remote, and the upstream of each branch (git's own configuration) |
| Every artifact and every tracked record ([[topic:machine-records]]) | The person's identity, as git gives it in that clone |
| The work claims ([[record:work-claim]]), with the holder's name, email and clone label | Credentials. The tool never stores, prints or asks for one |
| | The clone record ([[record:clone]]) and everything else under `.factory/local/`: runs, step records, evidence, jobs, the lock, caller markers, the event log |

This table is the one list of what stays on a machine; T-LOCAL is the check that keeps it there, and
that refuses a `factory.toml` holding a path that works on one machine only.

The hook runner already announces a change to a gate-bearing section that arrives from another clone
([[record:config-fingerprint]]); `[team]` joins that list.

## Versions

The state format is the `schema` key of `factory.toml`: a plugin that does not know the number refuses to
load the file, so every command and every gate on that machine refuses. That is built. The plugin
version against `min_plugin`, the git floor, and the version each state commit names are T-VERSION.
