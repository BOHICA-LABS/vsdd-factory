---
kind: topic
id: hook-failure
sources: [hooks.md#2.9, hooks.md#2.3]
---

# Failure behaviour

The principle: when the hook stands between the model and an action, a failure blocks. When blocking
would only strand the session (briefing, compaction, logging), a failure lets the session continue and
is announced at the next opportunity.

## What each failure turns into

| Failure | On a gate event | On `SessionStart` | On other events |
|---|---|---|---|
| `uv` is not installed | Exit 2: "uv is not installed, so the factory gates cannot run", with the fix | Fixed JSON briefing to the person and the model | Exit 0 |
| No Python 3.11 or newer (uv exits 2) | Exit 2, with the fix `uv python install 3.12` | Same | Exit 0 |
| uv cannot run (cache not writable, unknown flag on an old uv) | Exit 2 naming the exit code and three terminal commands that need nothing from the plugin (below) | Same | Exit 0 |
| The plugin's files are missing (the launcher cannot be read) | In a factory project: exit 2 from the self-guarding command, with the reinstall message. In any other project: exit 0, nothing printed | In a factory project: the fixed down-briefing with `systemMessage` | Exit 0 |
| Runner crashes before rules run (an import or syntax error in the runner's own modules after a bad update) | Exit 2 through the launcher's exit-code mapping | Same | Exit 0 |
| Runner killed (signal, out of memory) | Exit 2 | Same | Exit 0 |
| Hook input is not valid JSON | Block `runner_bad_input` | Fallback briefing | Logged |
| A gate rule raises an exception, or its module cannot be imported | Block `rule_error` naming the rule and the exception type, with the restart line below; the other rules still run | n/a | n/a |
| The runner's deadline passes | Block `runner_timeout` naming the rule that was running | Fallback briefing | Logged, exit 0 |
| Claude Code's own timeout passes, or the hook process dies before it can exit | Blocked by `onFailure: "block"` | Claude Code shows a hook error | Ignored |
| The event log cannot be written | The decision is unchanged; the block message is still printed | Briefing adds "event log not writable" | Ignored |
| Project configuration is invalid | Built-in defaults (the strictest setting) | Briefing says so | Defaults |
| A policy table is missing or invalid | Block `policy_invalid` for the rules that need it | Briefing says so | Logged |

Both of those rows reach the launcher as uv's exit code 2. The launcher tells them apart on that failure
path only, so a successful call pays nothing: it runs `uv python find --no-python-downloads '>=3.11'` with a
cache directory of its own, so that a broken session cache cannot be mistaken for a missing interpreter. If
that finds an interpreter the fault is uv's (code `uv_unusable`); otherwise it is the missing Python (code
`py_floor`), whose message also names `uv --version` and `uv cache dir` in case the probe itself could not
run.

The three terminal commands in the "uv cannot run" row: `uv --version` (is it at least the tested
version), `uv cache dir` (is that directory writable), and the upgrade or reinstall command
(`uv self update`, or `brew upgrade uv`). It does not name `vsdd doctor`, which starts through the
same launcher and would fail the same way.

A rule that crashes every time blocks every call it guards, and the agent that sees the block cannot
clear it. So the `rule_error` block's `Instead:` line is written for a person: it says to restart Claude
Code with `VSDD_FACTORY_HOOKS_QUARANTINE=<rule>` set and to report the fault. The session briefing and
`vsdd status` list each rule with repeated `rule.error` events, with the same line, so the person meets
it without reading an agent's output. When the quarantine applies is in [[topic:rule-tuning]].

## Which rules fail closed, and why

| Rule | Fails | Reason |
|---|---|---|
| `bash-destructive`, `bash-git`, `merge-guard` | closed | The actions are irreversible. |
| `secrets`, `read-walls` | closed | A secret or a holdout scenario cannot be unread. |
| `write-scopes`, `gate-guard`, `artifacts` | closed | The write would land; the agent can simply retry once the fault is fixed. |
| `tests-first` | closed | The gate is the point of the method. |
| `decisions` | closed | A decision that could not be recorded must not look recorded; the person sees the reason and retypes. |
| `step-results` | closed | A step result that is not on disk breaks resume, and an ungrounded finding must go back to its author. The platform's retry limit bounds the loop. |
| `spike-bounds` | closed, except at stop | A spike with no ledger entry or past its time box must not continue. A finish without a record is refused once; the second attempt is allowed and logged, so an agent is never trapped. |
| `session-briefing` | open, announced | Blocking a session start helps nobody; the fallback text says the briefing failed. |
| `compaction` | open, announced | Blocking automatic compaction can fail the request outright (hooks documentation). The next briefing reports the failed flush. |
| `lifecycle-log`, `purity-advice` | open | Recording and advice only. |
| `auto-allow` | open | No permit means the normal permission prompt. |

## How "no gate switches off silently" is met, in one place

1. Every launch failure on a gate event becomes a block with a reason (launcher), backed by
   `onFailure: "block"` (platform).
2. A rule crash or timeout blocks for every gate rule.
3. Rule settings cannot be changed by a tool call, relaxations are listed in every briefing, and
   changes are announced.
4. The emergency switch is an environment variable only a person can set, and it is announced.
5. `hooks.json` is generated from the rule table, so a rule cannot be registered and unwired, which is
   what happened to the old pre-compaction hooks.
6. Every gate rule has a case that must produce exit 2 through the real launcher, and one that must
   pass. The meta-test in `packaging.md` enforces that such cases exist; `vsdd hooks selftest` runs
   them against an installed plugin.
7. If hooks are switched off at the platform level, which the runner cannot see, a workflow still
   finds out: the runner writes a caller marker for every `vsdd` call, and `vsdd survey` refuses to
   start a run when its own call has none (see [[topic:caller-identity]]). `vsdd doctor`, run through
   the Bash tool, makes the same test on its own call, so there is no separate canary command.
8. The command in `hooks.json` guards itself against a missing launcher, in factory projects only
   (see [[topic:hook-wiring]]), and `gate-guard` blocks the tool calls that would disable the plugin
   or set the emergency switch in a settings file.

Time budgets per mode (the deadline a rule crash or overrun collides with): `gate` 10 s Claude Code /
4 s runner; `brief` 15 s / 8 s; `PreCompact` 30 s / 25 s of which the flush may use 20 s; `notify`
elsewhere 5 s (1.5 s on `SessionEnd`) / 1 s.
