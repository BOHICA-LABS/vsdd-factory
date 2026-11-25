---
kind: packaging
id: plugin-manifest
package: B1
sources: [packaging.md#2.4, BLUEPRINT.md#2.3]
---

# `plugin.json`

The complete file, delivered by package B1:

```json
{
  "name": "vsdd-factory",
  "displayName": "VSDD Factory",
  "version": "1.0.0",
  "description": "Verified Spec-Driven Development for Claude Code: specifications, stories, test-first delivery, adversarial review, holdout evaluation and release gating, run by workflows and enforced by hooks.",
  "author": { "name": "Bohica Labs", "url": "https://github.com/bohica-labs" },
  "homepage": "https://github.com/bohica-labs/vsdd-factory#readme",
  "repository": "https://github.com/bohica-labs/vsdd-factory",
  "license": "MIT",
  "keywords": ["vsdd", "spec-driven", "tdd", "sdlc", "workflows", "adversarial-review"],
  "icon": "./.claude-plugin/icon.png",
  "documentationUrl": "https://github.com/bohica-labs/vsdd-factory/tree/main/docs",
  "supportUrl": "https://github.com/bohica-labs/vsdd-factory/issues",
  "engines": { "claude-code": ">=2.1.331" },
  "network": {
    "hosts": [
      "api.perplexity.ai", "mcp.tavily.com", "mcp.context7.com", "registry.npmjs.org",
      "github.com", "api.github.com", "*.githubusercontent.com",
      "*.docker.io", "*.docker.com"
    ]
  },
  "userConfig": {
    "perplexity_api_key": {
      "type": "string", "sensitive": true,
      "title": "Perplexity API key",
      "description": "Lets research agents search with Perplexity. Optional: leave empty to work without it."
    },
    "tavily_api_key": {
      "type": "string", "sensitive": true,
      "title": "Tavily API key",
      "description": "Lets research agents search and extract with Tavily. Optional."
    },
    "context7_api_key": {
      "type": "string", "sensitive": true,
      "title": "Context7 API key",
      "description": "Lets agents fetch current library documentation from Context7. Optional."
    }
  }
}
```

Every field, and every field left out:

- `name` — `vsdd-factory`. Unchanged. It is the namespace of every agent, skill and workflow, and must never be
  renamed.
- `displayName` — `VSDD Factory`. Shown in the interface only.
- `version` — `1.0.0`. Changed only by a release pull request ([[packaging:plugin-release]]). While it is
  unchanged, installed copies do not update, by platform design.
- `description` — rewritten: it no longer promises brownfield-to-release in one sentence of jargon.
- `author`, `homepage`, `repository` — `bohica-labs`. Decision D29 of [[doc:decisions]]: use "Bohica Labs" and
  `bohica-labs` URLs; no directory submission for 1.0, so the icon can be a placeholder.
- `license` — `MIT`. Unchanged.
- `keywords` — six. Discovery only.
- `icon` — `./.claude-plugin/icon.png`. A generated square 512 px PNG. Read only by Anthropic's directory, never by
  Claude Code.
- `documentationUrl`, `supportUrl` — GitHub links. Read only by the directory. Harmless otherwise.
- `engines` — `>=2.1.331`. The floor (D10). Claude Code versions before 2.1.76 reject a manifest that has this key
  at all; they are far below the floor. The same number is recorded in `tests/lint/roster.toml` under
  `[platform]`, and the CI matrix reads it from there.
- `network.hosts` — nine entries. Descriptive only: the platform says this field allows and blocks nothing. The
  table below says what reaches each host.
- `userConfig` — three optional secrets. `sensitive: true` stores each value in the system credential store, once
  per user, so all projects share them. Each is exported to hook processes as
  `CLAUDE_PLUGIN_OPTION_PERPLEXITY_API_KEY` and so on, and is substituted into `.mcp.json` as
  `${user_config.perplexity_api_key}`. None is `required`, so the plugin works with no keys.
- `$schema` — omitted. No verified URL for it, and Claude Code ignores it.
- `defaultEnabled`, `dependencies`, `metadata`, `privacyPolicyUrl`, `termsOfServiceUrl`, `channels`, `types`,
  `lspServers`, `outputStyles`, `commands`, `experimental` — omitted. Defaults are right or the feature is unused.
- `agents`, `skills`, `workflows`, `hooks`, `mcpServers` — omitted. The default locations are used. Setting `agents`
  or `workflows` replaces the default scan and gains nothing.

Hosts and what reaches them:

| Host | Reached by | When |
|---|---|---|
| `api.perplexity.ai`, `mcp.tavily.com`, `mcp.context7.com` | the three research servers in `.mcp.json` | when an agent researches |
| `registry.npmjs.org` | `npx`, which starts the Perplexity and Playwright servers | first start of each |
| `github.com`, `api.github.com` | `git` and `gh` commands that agents run for pull requests | story delivery |
| `*.githubusercontent.com`, and `github.com` again | `uv`, only if it has to download a Python interpreter because none at the floor is installed | at most once per machine |
| `*.docker.io`, `*.docker.com` | `vsdd obs up` pulling images | only if the observability stack is started |

The shipped Python reaches no host. A lint test forbids it from importing `socket`, `ssl`, `http`, `urllib.request`,
`ftplib` and `smtplib`, so this is true by construction. Browser tools reach whatever site the project under test
names; that cannot be declared.

## `.mcp.json`

`.mcp.json` is packaged by this design, but its list of servers belongs to the agents-skills design
([[topic:research-and-browser-tools]]). The default, taken from the configuration the user already runs:

```json
{
  "mcpServers": {
    "perplexity": { "command": "npx", "args": ["-y", "@perplexity-ai/mcp-server@<exact>"],
                    "env": { "PERPLEXITY_API_KEY": "${user_config.perplexity_api_key}" } },
    "tavily":     { "type": "http", "url": "https://mcp.tavily.com/mcp/",
                    "headers": { "Authorization": "Bearer ${user_config.tavily_api_key}" } },
    "context7":   { "type": "http", "url": "https://mcp.context7.com/mcp",
                    "headers": { "CONTEXT7_API_KEY": "${user_config.context7_api_key}" } },
    "playwright": { "command": "npx", "args": ["-y", "@playwright/mcp@<exact>"] }
  }
}
```

The Playwright entry carries no option. The pinned server (`@playwright/mcp@0.0.86`) refuses `file:` addresses and
file access outside the workspace roots by default, so the bundled server cannot be used to read a file around an
information wall; its one documented option on the subject, `--allow-unrestricted-file-access`, loosens that, and
L-MCP rejects it ([[table:lint-checks]]). Probe 33 ([[doc:platform]]) confirms on the running server that a `file:`
address is refused. The hook's own check of navigation addresses applies whichever server a session ends up with, so
the wall does not rest on the server's default alone.

Three things the agents-skills designer must know, all from the platform documentation. First, a plugin server's
tools are named `mcp__plugin_vsdd-factory_<server>__<tool>`. Second, if the user or the project already has a server
at the same command or URL, the plugin's copy is dropped as a duplicate and the surviving tools carry the other
name, so tool names are not stable across machines. Whether the user's own Playwright server survives beside the
bundled one, and with which options, is probe 33 of [[doc:platform]]. Third, for a project-scoped install, each
bundled server needs a one-time approval per project.

Package versions are pinned exactly; a lint test rejects `@latest`. Lint L-MAN checks the manifest fields
([[table:lint-checks]]); `claude plugin validate --strict` is clean at package B1.
