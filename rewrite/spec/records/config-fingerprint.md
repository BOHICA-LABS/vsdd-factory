---
kind: record
id: config-fingerprint
package: H1
sources: [BLUEPRINT.md#4.7, hooks.md#2.6, BLUEPRINT.md#5.4]
---

# Configuration fingerprint

Under `.factory/local/hooks/`. Not tracked. The hook runner's fingerprint of every gate-bearing configuration
section, used to announce a change. Fields in [[schema:config-fingerprint]].

The runner keeps a fingerprint of every section that bears on a gate: `[hooks]`, `[commands]`, `[paths]`,
`[branches]`, `[delivery]`, `[convergence]`, `[holdout]`, `[mutation]`, `[limits]`, `[[component]]`,
`[stops]`, `[release]`, `[spikes]`, `[caps]`. A change since the previous session is announced once, to the
model and through `systemMessage` to the person, and the briefing announces a change to any gate-bearing
section. `[[component]]` matters most: its commands override `[commands]`, the file travels with the pushed
state branch, and a `test = "true"` arriving from another machine would otherwise make the green gate pass in
silence. A write by code (the toolchain an approval applied, a `repos split`) is announced the same way.

Written by the hook runner. The configuration keys themselves are [[table:config-keys]] and the per-section
files under [[config:project]].

## Team use (decision D33; built when the build widens)

`[team]` joins the fingerprinted sections ([[config:team]]).
