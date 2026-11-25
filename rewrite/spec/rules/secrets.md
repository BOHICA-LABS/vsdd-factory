---
kind: rule
id: secrets
package: H2
sources: [hooks.md#2.10, hooks.md#2.11.1, BLUEPRINT.md#5.4]
---

# secrets

Carries over the old `protect-secrets` hook (both entries), with the same reason codes so the old test
cases can be harvested as [[topic:rule-testing]] describes.

| Registration | |
|---|---|
| Kind | gate |
| Runs on | `PreToolUse`, tools `Read`, `Grep`, `Bash`; plus the web tools the matcher names (`WebFetch`, `WebSearch`, the research tools) |
| Result | `Block` |
| Fails | closed — "a secret or a holdout scenario cannot be unread" |
| Relax | yes (`[hooks.rules.secrets]`) |
| Spike exemption | no (the safety rules still apply inside a spike) |

## What it checks

Works on a parsed command, not on the whole string: `shell.py` splits the command into segments at `;`,
`&&`, `||`, `|` and newlines, tokenises each with `shlex`, and records redirect targets and any
`cd <dir>` or `git -C <dir>` prefix. It also reads what the command points at: a commit made with
`git commit -F <file>` has its message file scanned.

Blocked:

- `Read` or `Grep` of `.env`, `.env.*` and `.envrc` (templates named `.example`, `.sample`, `.template`
  are allowed), of `.mcp.json` (it holds live keys in most projects), of home credential files
  (`~/.aws/**`, `~/.ssh/**`, `~/.netrc`, `~/.docker/config.json`, `~/.config/gh/**`, `~/.npmrc`, Claude
  Code's credential files), of `*.pem` and `*.key`, of `/proc/*/environ`, and of the configured
  `secret_globs` (`[hooks.rules.secrets]`, [[table:config-keys]]).
- Shell reads, copies and archives of the same files.
- `gh auth token` and `git credential fill`.
- `echo` or `printf` of a secret-shaped variable. Bare `env`, `printenv` and `set`, and the same piped
  to `grep`. Also `export -p` and `declare -x`, which print the environment as `env` does.

Web tools: every call is logged as `web.call` with its host, and a call is blocked when its address or
query contains the value of a secret-shaped environment variable or a common token format.

Shell web addresses: the same log and the same check apply to every web address in a `curl`, `wget` or
`http` command an agent row runs in Bash, because a `GET` address can carry data out as well as an
upload can; this is what bounds a spike runner, which has both web tools and a shell.

Uploads: for agents, `curl` and `wget` with an upload flag (`-d`, `--data*`, `-F`, `-T`,
`--upload-file`, `--post-file`) are blocked, with one exception — every target address is loopback
(`localhost`, `127.0.0.1`, `[::1]`) on a port that `vsdd env start` recorded for the current run. That
exception lets the holdout evaluator and the demo recorder exercise the `POST` and `PUT` endpoints of
the product the tool started; without it, scenarios for an API product would all score as not run. The
case table has an allowed loopback upload, a blocked upload to another host, and a blocked upload to a
loopback port the run did not start.

## Messages

Covered by the shared block format and event log: see [[topic:rule-model]] and [[table:event-types]]
(`web.call` and the `rule.block` fields).

## Context

- Adopted decision D11 puts the old git rules on by default; `secrets` is part of the general-safety
  triple with [[rule:bash-destructive]] and [[rule:bash-git]].
- The blueprint amendment (sources, above): `secrets` covers `.mcp.json`, home credential files, key
  files, `gh auth token`, `git credential fill`, bare `env` and `printenv`; web tool calls are logged,
  and a URL or query that contains a known secret value or token format is blocked; loopback uploads
  are allowed as above.
- `secret_globs` is this rule's own key under `[hooks.rules.secrets]`; like every key of `[hooks]` only a
  person can change it (see [[topic:rule-tuning]]).

## Feature flags (decision D31; built when the build widens)

Built with package F2, not with this rule's own package. In a project that uses feature flags, the
variables that `flags.key_env` names are secret variables whatever their names look like: the rule blocks
a shell segment that names one (`$NAME`, `${NAME}`), in addition to what it blocks for every
secret-shaped variable ([[config:flags]]).
