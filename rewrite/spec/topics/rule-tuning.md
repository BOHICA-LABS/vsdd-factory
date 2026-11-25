---
kind: topic
id: rule-tuning
sources: [hooks.md#2.6, BLUEPRINT.md#2.1, BLUEPRINT.md#2.2]
---

# Tuning a rule or turning it off

Settings live in the `[hooks]` table of the project configuration file. Only a person can change this
table: the file is in the `config` path class, which no tool call may write, and the `vsdd` command
offers no subcommand an agent can run that changes it (`vsdd config set` is on the person-only list,
need K7). An agent that is blocked by a gate therefore cannot switch the gate off, and cannot weaken a
gate indirectly by changing the test command, the source globs or a threshold.

```toml
[hooks]
log_retention_days = 30
main_session_wall = true            # decision D6; false relaxes the main session's row only

[hooks.rules.auto-allow]
scope = "core"                      # decision D4: off | core | extended; written by vsdd init --auto-allow

[hooks.rules.tests-first]
strength = "advise"                 # block (default) | advise | off

[hooks.rules.bash-git]
attribution = "block"               # commit messages and pull request text

[hooks.rules.bash-destructive]
allow_pr_close = true
```

- `strength = "advise"` turns every block from that rule into advice that begins "would have blocked",
  and logs `rule.would_block`. It is the way to trial a rule. `off` skips the rule.
- Locked rules ([[rule:read-walls]], [[rule:write-scopes]], [[rule:gate-guard]], [[rule:decisions]],
  [[rule:step-results]], [[rule:session-briefing]]) ignore `strength`; setting it is reported as a
  configuration error. Decision D12: lock the walls, write scopes, gate-guard, decision capture and
  step storage; let a person set any other gate to advise or off by editing the project file.
- One exception to the lock is stated, because the user adopted both decision D12 (the walls are
  locked) and decision D6 (the main session is walled, "with a switch in project configuration").
  `main_session_wall = false` relaxes the `main` row of [[table:walls]] and nothing else: the main
  session may then read `holdout` and write the specification classes. Every agent row stays locked
  whatever the key says. While it is `false`, every briefing says so.
- `auto-allow` has a scope, not a strength: `off`, `core` or `extended` (see
  [[topic:tool-call-approval]]). `vsdd init --auto-allow <scope>` writes it from the answer `setup`
  collected before it ran `init`, because that is the one moment the plugin writes `[hooks]`. This is
  how the person's yes to decision D4 is stored.

## Nothing is relaxed silently

Every rule that is not at full strength is listed in every session briefing. The runner keeps a
fingerprint of every section that bears on a gate (`[hooks]`, `[commands]`, `[paths]`, `[branches]`,
`[delivery]`, `[convergence]`, `[holdout]`, `[mutation]`, `[limits]`, `[[component]]`, `[stops]`,
`[release]`, `[spikes]`, `[caps]`), and a change since the previous session is announced once, to the
model and through `systemMessage` to the person. `[[component]]` matters most: its commands override
`[commands]`, the file travels with the pushed state branch, and a `test = "true"` arriving from
another machine would otherwise make the green gate pass in silence. A write by code (the toolchain an
approval applied, a `repos split`) is announced the same way.

## Quarantine and defaults

One rule can be quarantined: `VSDD_FACTORY_HOOKS_QUARANTINE=<rule id>`, an environment variable only a
person can set, skips that rule, locked or not, but only when the event log shows repeated
`rule.error` events for it. It exists so that a crash in one rule (for example `step-results`, which
would stop every workflow step) does not force a person to switch every gate off. Every briefing
announces it.

An unknown rule id, an unknown key or a bad value under `[hooks]` does not disable anything: the
runner uses the built-in defaults for that rule, logs `config.invalid`, and says so in the briefing.

Project-specific paths and limits that rules read (the integration and release branches, source and
test globs, the spike time box) are ordinary project configuration owned by the state design (need
S3), not `[hooks]` settings.
