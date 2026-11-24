<!-- Documentation-backed facts about the Claude Code plugin platform (read 2026-11-23). Captured 2026-11-23 by a read-only survey of branch develop @ 150bc36c. Historical reference for the 1.0 rewrite; not maintained. -->

# Claude Code plugin platform: documented facts for the rebuild

Sources were read on 2026-11-23. Where a page was truncated or silent, the item is marked UNCONFIRMED. Three pages were only partly readable: hooks (the per-event payload schemas), sub-agents (the tail of the page), and skills (the tail). The Agent SDK TypeScript page was too long to reach the Workflow tool entry, so Q2's invocation details are unconfirmed.

## 1. Plugin components and plugin.json

**Components** (sources: [components](https://code.claude.com/docs/en/plugins/components.md), [manifest reference](https://code.claude.com/docs/en/plugins/manifest-reference.md#standard-layout))

| Component | Default location | Manifest key | Replace or add |
|---|---|---|---|
| Manifest | `.claude-plugin/plugin.json` | n/a | n/a |
| Skills | `skills/<name>/SKILL.md` | `skills` | Adds to default scan |
| Commands (legacy) | `commands/*.md` | `commands` | Replaces default |
| Subagents | `agents/*.md` (subfolders allowed) | `agents` | Replaces default |
| Hooks | `hooks/hooks.json` (top-level `"hooks"` key) | `hooks` | Merges |
| MCP servers | `.mcp.json` | `mcpServers` | Merges (later name wins) |
| LSP servers | `.lsp.json` | `lspServers` | Merges |
| Output styles | `output-styles/*.md` | `outputStyles` | Replaces default |
| Workflows | `workflows/*.js` | `workflows` | Replaces default |
| Themes | `themes/*.json` | `experimental.themes` | Replaces default |
| Monitors | `monitors/monitors.json` | `experimental.monitors` | Replaces default |
| Executables | `bin/` (added to Bash PATH) | none | n/a |
| Channels | none (bound to an MCP server) | `channels` | n/a |
| Mods | `modules` key in hooks.json | `types` (`.d.ts`) | n/a |

- `settings`: UNCONFIRMED as a plugin component. The manifest field table has no `settings` key.
- `plugin.json` fields: `$schema`, `name` (only required field), `displayName`, `version`, `description`, `author` (`name` required), `homepage`, `repository`, `license`, `keywords`, `metadata`, `icon`, `documentationUrl`, `supportUrl`, `privacyPolicyUrl`, `termsOfServiceUrl`, `defaultEnabled`, `engines` (`{"claude-code": "<semver range>"}`, requires v2.1.323+), `network`, `dependencies`, `userConfig`, `types`, `channels`, `skills`, `commands`, `agents`, `hooks`, `mcpServers`, `lspServers`, `outputStyles`, `workflows`, `experimental` (`themes`, `monitors`, `evals`).
- Path rules: component paths must start with `./` and stay inside the plugin root.
- Plugin-root `CLAUDE.md` is not loaded. Put instructions in a skill.
- Substitution variables: `${CLAUDE_PLUGIN_ROOT}`, `${CLAUDE_PLUGIN_DATA}` (`~/.claude/plugins/data/<id>/`, survives updates), `${CLAUDE_PROJECT_DIR}`.

## 2. Workflows

Source: [workflows](https://code.claude.com/docs/en/workflows.md)

- **Can a plugin bundle workflows?** Yes. Put scripts in `workflows/` at the plugin root, or set the `workflows` manifest key.
- **Namespacing:** `/<plugin>:<meta.name>`. Example: plugin `acme-tools` with `meta.name: 'release-audit'` runs as `/acme-tools:release-audit`.
- **Saved (non-plugin) workflows** live in `.claude/workflows/` (project, shared via repo) or `~/.claude/workflows/` (personal). A project workflow wins over a personal one with the same name.
- **Script API:** `export const meta = {name, description}` (literal values only), plus `agent()`, `parallel()`, `pipeline()`, `phase()`, `log()`, and the `args` global. Top-level `await` is allowed.
- **Permissions:** in `-p` and the Agent SDK, `Workflow` or `Workflow(<name>)` allow rules, auto mode, bypass mode, or a `PreToolUse` hook returning `allow` are needed. In plan mode a hook alone does not start a run.
- **Skill or slash command triggering a workflow:** the docs show a saved workflow itself becoming a `/<name>` command and a human-typed `ultracode` keyword opting into a workflow. Whether a skill's instructions can make Claude call the Workflow tool is UNCONFIRMED. The `ultracode` keyword is explicitly not honored in scheduled-task prompts or relayed payloads. Test this directly.
- **Documented limits:**
  - 16 concurrent agents by default (`CLAUDE_CODE_WORKFLOW_MAX_CONCURRENT_AGENTS`, range 1–256).
  - 4,096 items per `parallel()` or `pipeline()` call.
  - 1,000 agents per run.
  - No mid-run user input (only permission prompts and usage-limit waits pause a run).
  - No `import()`, and no direct filesystem or shell access from the script.
  - Structured-output `schema` retries: 5 (`MAX_STRUCTURED_OUTPUT_RETRIES`).
  - Stalled agents restart up to 5 times (6 attempts total).
  - `Date.now()`, `Math.random()`, and no-arg `new Date()` throw, so relaunches replay deterministically.
- **Resume:** runs resume within the same session. Completed agents return cached results, and the first changed prompt reruns everything after it. Failed agents rerun along with every agent the script starts after them. Relaunch needs a `Workflow` call (`/workflows` view, key `p`).
- **Whether `agent()` can select a named plugin subagent type:** UNCONFIRMED. The examples show only prompt and `label`/`schema`/`stallMs` options.
- **Whether workflow agents may call `Agent` (nest):** UNCONFIRMED.

## 3. Subagent frontmatter

Source: [sub-agents](https://code.claude.com/docs/en/sub-agents.md#supported-frontmatter-fields)

- **Required:** `name` (max 256 chars, no `:`, no leading `-`) and `description`.
- **Supported fields:** `tools`, `disallowedTools`, `model`, `effort`, `permissionMode`, `maxTurns`, `autoCompactWindow` (int ≥ 100000, v2.1.296+), `skills`, `mcpServers`, `hooks`, `memory` (`user|project|local`), `background`, `omitClaudeMd` (v2.1.271+), `isolation` (`worktree` only), `color` (red, blue, green, yellow, purple, orange, pink, cyan), `initialPrompt`, `experimental.cacheTtl` (`5m|1h`).
- **`model`:** `sonnet`, `opus`, `haiku`, `fable`, `inherit`, or a full model ID (the docs' example is `claude-opus-5-5`). Omitted means the subagent model order applies.
- **`effort`:** `low`, `medium`, `high`, `xhigh`, `max`. Available levels depend on the model. It does not override `CLAUDE_CODE_EFFORT_LEVEL`.
- **`permissionMode`:** `default`, `acceptEdits`, `auto`, `dontAsk`, `bypassPermissions`, `plan`, or `manual` (alias for `default`).
- **Ignored in plugin-shipped agents:** `permissionMode`, `hooks`, `mcpServers`, `initialPrompt`. Ship hooks in `hooks/hooks.json` and MCP servers in `.mcp.json` instead.
- **Unknown fields** are silently ignored. Unparseable frontmatter loads the agent with all fields ignored.
- **`tools`:** if no entry resolves to a tool, the subagent usually fails to launch (relevant for 35 agents).
- **Nesting:** subagents can spawn subagents, up to three layers below the main conversation by default. `CLAUDE_CODE_MAX_SUBAGENT_SPAWN_DEPTH=1` disables nesting. Concurrency: 20 by default (`CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`). Total per session is unlimited.

## 4. Skill frontmatter

Source: [skills](https://code.claude.com/docs/en/skills.md#frontmatter-reference)

- **Fields:** `name` (defaults to directory name), `description`, `when_to_use`, `argument-hint`, `arguments`, `disable-model-invocation` (default false), `user-invocable` (default true), `allowed-tools`, `disallowed-tools`, `model` (`/model` values or `inherit`, current turn only), `effort`, `context` (only `fork` is documented), `agent` (used with fork; defaults to `general-purpose`), `background` (fork only; default true), `hooks`, `paths` (glob list; not allowed in command files), `shell` (`bash` or `powershell`), `metadata`, `license`, `compatibility`.
- The docs do not give an explicit type for every field.
- `description` plus `when_to_use` is truncated at 1,536 characters in the skill listing.
- Unknown keys are ignored. Outside Claude Code (claude.ai, Skills API), only a small set of keys is accepted.
- **`commands/`:** still supported and merged into skills, not removed. The components page calls it the older format and recommends skills for new work, with `commands/` kept for files being moved over.
- **Collisions:** plugin skills are namespaced as `/plugin-name:skill-name`, so a plugin skill and a local skill both load.

## 5. Plugin hooks

Sources: [hooks](https://code.claude.com/docs/en/hooks.md), [components](https://code.claude.com/docs/en/plugins/components.md#hooks)

- **File format:** `hooks/hooks.json` with a top-level `"hooks"` key. The inner shape is the same as `hooks` in settings.json. The manifest `hooks` key accepts a path, an inline object, or an array of either. A file with no `"hooks"` wrapper fails to load.
- **Structure:** event → matcher group (`matcher` + `hooks` array) → handler.
- **Events** (full list from the hooks page): `SessionStart`, `Setup`, `UserPromptSubmit`, `UserPromptExpansion`, `PreToolUse`, `PermissionRequest`, `PermissionDenied`, `PostToolUse`, `PostToolUseFailure`, `PostToolBatch`, `Notification`, `MessageDisplay`, `SubagentStart`, `SubagentStop`, `TaskCreated`, `TaskCompleted`, `Stop`, `StopFailure`, `TeammateIdle`, `InstructionsLoaded`, `ConfigChange`, `CwdChanged`, `DirectoryAdded`, `FileChanged`, `WorktreeCreate`, `WorktreeRemove`, `PreCompact`, `PostCompact`, `PreModelSwitch`, `PostModelSwitch`, `Elicitation`, `ElicitationResult`, `SessionEnd`.
- **Handler types:** `command`, `http`, `mcp_tool`, `prompt`, `agent` (experimental, may change).
- **Common handler fields:** `type`, `if` (permission-rule expression; only on tool events), `timeout`, `statusMessage`, `once`. `onFailure` (`continue` or `block`, v2.1.295+) applies to `PreToolUse` timeouts.
- **Matcher syntax:** `"*"`, empty, or omitted matches everything. Letters, digits, `_`, `-`, spaces, `,`, and `|` form an exact name or list (`Edit|Write`). Any other character makes it an unanchored JavaScript regex (`^Notebook`, `mcp__memory__.*`). Plugin MCP tools match as `mcp__plugin_<plugin>_<server>__<tool>`. A bare server name matches nothing.
- **stdin:** JSON with `session_id`, `cwd`, `permission_mode`, `hook_event_name`, and for tool events `tool_name`, `tool_input`, `tool_use_id`. Inside subagents, `agent_id` and `agent_type`.
- **Exit codes:**
  - 0: success. stdout is parsed as JSON if it is a JSON object.
  - 2: blocking error. stderr is the reason. On non-blocking events (`PostToolUse`, `Notification`, `SessionStart`) it only reaches the user or Claude.
  - Other: non-blocking error, action proceeds.
  - Exit 1 without valid JSON is non-blocking. Enforced policy should use exit 2.
- **JSON output:** universal `continue`, `stopReason`, `suppressOutput`, `systemMessage`. Top-level `decision` and `reason`. `hookSpecificOutput` requires `hookEventName`. `PreToolUse` supports `permissionDecision` (`allow|deny|ask|defer`).
- **`async: true`:** command hooks only, runs in background, no timeout enforced, and cannot use `onFailure`. `asyncRewake: true` wakes Claude on exit 2.
- **Timeout defaults:** 600s for command, http, and mcp_tool; 30s for prompt; 60s for agent. Shorter defaults apply on some events (30s on `UserPromptSubmit`, `PreModelSwitch`, `PostModelSwitch`; 10s on `MessageDisplay`).
- **Variables:** `${CLAUDE_PLUGIN_ROOT}`, `${CLAUDE_PLUGIN_DATA}`, `${CLAUDE_PROJECT_DIR}`. Hooks also get `CLAUDE_PLUGIN_OPTION_<KEY>` for each user config value. Use exec form (`command` + `args`) for paths. Shell form needs quoting.
- **Python hook scripts:** the docs have no dedicated guidance. Confirmed facts: shell form runs through `sh`, Git Bash, or PowerShell; exec form on Windows needs a real `.exe`, so `.cmd`/`.bat` shims won't run; `"command": "node", "args": [...]` is the documented workaround. Calling `python3` explicitly is the natural pattern but is not documented. `uv run` is UNCONFIRMED.

## 6. Subagent nesting and workflows

- Subagents nest up to three layers below the main conversation by default (see Q3).
- Workflow `agent()` calls run in a separate runtime, not in the main conversation. Whether a workflow agent may spawn further subagents is UNCONFIRMED.
- The 20-concurrent Agent-tool limit and the 16-concurrent workflow limit are separate. The docs say the session's concurrent-subagent limit is not enforced for Agent-tool subagents when ultracode is on.
- Subagent `tools` and `disallowedTools` apply to nested spawns. Omitting `Agent` from `tools` stops a given subagent from spawning.

## 7. Distribution, versioning, dependencies

Sources: [publish](https://code.claude.com/docs/en/plugins/publish.md), [marketplace reference](https://code.claude.com/docs/en/plugins/marketplace-reference.md), [dependencies](https://code.claude.com/docs/en/plugins/dependencies.md)

- **Marketplace file:** `.claude-plugin/marketplace.json`. Required top-level fields: `name`, `owner` (`name` required), `plugins`. Optional: `description`, `version`, `metadata` (`pluginRoot`), `forceRemoveDeletedPlugins`, `allowCrossMarketplaceDependenciesOn`, `renames`.
- **Plugin entries:** `name` and `source` are required. Also `description`, `version`, `category`, `tags`, `strict` (default true), `dependencies`, `engines`, `defaultEnabled`, `displayName`, `metadata`, `headers`, `headersHelper`. Entries also accept any `plugin.json` field.
- **Source types:**
  - relative path (`"./plugins/x"`)
  - `github` (`repo`, `ref`, `sha`)
  - `url` (`url`, `ref`, `sha`)
  - `git-subdir` (`url`, `path`, `ref`, `sha`)
  - `npm` (`package`, `version`, `registry`)
  - `archive` (`url`, `sha256`)
  - `command` (`command`, `timeout`, `mode` copy|link)
- **Strict mode:** when `plugin.json` exists and `strict` is not false, the entry's `commands`, `agents`, `skills`, `hooks`, `outputStyles`, and `themes` are appended. Hook matchers replace the manifest's per event. With `strict: false`, any of those fields in the entry is a conflict and the plugin fails to load.
- **Versioning:** `version` in `plugin.json` wins over the entry's. Setting it pins users until you bump it. If you set it and don't bump it, `claude plugin update` reports "already at the latest version." Omit `version` in a git-hosted marketplace and Claude Code uses the commit SHA. Do not rename a published plugin; use the `renames` map.
- **Install and update commands:** `claude plugin marketplace add owner/repo`, `claude plugin install name@marketplace`, `claude plugin update name@marketplace`. `claude plugin validate --strict`, `claude plugin tag --push`.
- **Plugin dependencies:** yes, supported. The `dependencies` array in `plugin.json` takes `"name"`, `"name@marketplace"`, or `{name, marketplace, version}`. `version` is a semver range (`^2.0`, `~2.1.0`). Constraints resolve against git tags named `<plugin-name>--v<version>`. Overlapping constraints resolve to the highest satisfying version. Non-overlapping constraints fail the install. Cross-marketplace dependencies require the root marketplace's `allowCrossMarketplaceDependenciesOn` list.
- **Dependency lifecycle:** a dependency that another enabled plugin needs starts enabled regardless of its own `defaultEnabled`. `claude plugin prune` removes auto-installed dependencies nothing needs. Disabling a dependency disables plugins that require it.
- **Installing plugins from a directory, or from a set of plugins:** `--plugin-dir` takes a directory or `.zip`. A folder with no `.claude-plugin/` but with plugin subfolders loads each subfolder as a plugin (v2.1.265+).

## Gaps and suggestions

- **UNCONFIRMED:** whether a skill can trigger the Workflow tool; whether `agent()` can pick a named plugin subagent; whether workflow agents can nest; the `settings` component; `uv run` for Python hooks. Test these in a scratch plugin before depending on them.
- **Truncated sources:** the hooks per-event payload schemas and the sub-agents page tail were not read. Check those pages directly if you need per-event fields.
- **Rebuild-risk items from the docs:** a plugin's skills default to `description` truncated at 1,536 characters, and the skill listing has a budget (`skillListingBudgetFraction` exists in your settings). With 127 skills, this matters. Fewer, grouped skills or `disable-model-invocation: true` on rarely used ones reduces listing cost.
- **Also consider `claude plugin eval`**: the evals docs at https://code.claude.com/docs/en/plugin-evals.md cover testing the plugin's behavior before a release.

If you find a discrepancy between these docs and your build's behavior, report it at https://github.com/anthropics/claude-code/issues.
