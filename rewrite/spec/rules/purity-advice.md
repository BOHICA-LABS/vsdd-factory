---
kind: rule
id: purity-advice
package: H9
sources: [hooks.md#2.10, hooks.md#2.11.10]
---

# purity-advice

Advises on side-effect patterns in files on configured pure-core paths.

| Registration | |
|---|---|
| Kind | advisor |
| Runs on | `PreToolUse`, tools `Edit`, `Write`, `MultiEdit` |
| Result | `Advise` (exit 0 with `hookSpecificOutput.additionalContext`) |
| Fails | open — advice only |
| Relax | yes |
| Spike exemption | yes |

## What it does

For files on the configured pure-core paths (`pure_paths`, default: `*/pure/**`, `*/core/**`,
`*/kernel/**`, `*_pure.rs`, `*.pure.ts`) it advises when the new content contains one of the configured
side-effect patterns (`side_effect_patterns`, default: the list built into the rule). Both keys are under
`[hooks.rules.purity-advice]` ([[table:config-keys]]).
