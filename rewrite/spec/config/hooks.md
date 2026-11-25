---
kind: config
id: hooks
package: S1
sources: [state.md#2.12, hooks.md#2.6, BLUEPRINT.md#4.12]
---
# [hooks] and [hooks.rules.<rule>]

Owned by the hook design; only a person edits it after init. Locked rules (`read-walls`, `write-scopes`,
`gate-guard`, `decisions`, `step-results`, `session-briefing`) ignore `strength`; setting it is reported as a
configuration error. Nothing is relaxed silently: every rule that is not at full strength is listed in every
session briefing. See [[topic:rule-tuning]] and [[topic:tool-call-approval]]. `hooks.rules.auto-allow.scope`
(off, core or extended) is the person's answer to decision D4; `vsdd init --auto-allow <scope>` writes it
once, from the answer `setup` collected before it ran `init`, and no `vsdd` command an agent can run changes
it afterwards. One approval is outside the key (decision D32): the hook runner approves an agent's own single
`vsdd` call with the rewrite that carries the caller marker even when the scope is `off`, because the
platform applies that rewrite only with an approval ([[topic:caller-identity]]). Keys, types and defaults: [[table:config-keys]], which is the whole list: `main_session_wall`,
`log_retention_days`, `strength` for each rule, and the keys four rules take of their own (`bash-git`
`attribution`, `bash-destructive` `allow_pr_close`, `secrets` `secret_globs`, `purity-advice` `pure_paths`
and `side_effect_patterns`). The loader accepts exactly these and reports any other key under `[hooks]`.
