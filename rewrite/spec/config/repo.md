---
kind: config
id: repo
package: S1
sources: [state.md#2.12, state.md#2.14]
---
# [[repo]]

Hub layout only. Member repositories, written by `vsdd init --hub` or `vsdd repos split`; cloned at `path`
(by default `repos/<name>/`, excluded from the hub through `.git/info/exclude`). Each entry may carry its own
`branches`, `commands`, `paths`, `release` and `delivery`, with the same keys as the top level, so the hub
release can ship each repository by its own rules. See [[topic:multi-repo-state]]. Keys, types and defaults:
[[table:config-keys]].

Decision D31, built when the build widens: an entry may also carry `flags`, with the path and pattern keys
of [[config:flags]].
