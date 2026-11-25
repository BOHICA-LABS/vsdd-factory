---
kind: topic
id: tool-sets
sources: [agents-skills.md#2.5, BLUEPRINT.md#4.3]
---

# Tool sets

Tool lists are written out in each agent file. Named sets keep the roster readable; the agent-file lint
checks each entry of a file's tools line against the union of these sets and rejects the tools no set holds
(the per-agent assignment below is prose, not a second table the lint reads).

| Set | Tools |
|---|---|
| READ | `Read`, `Grep`, `Glob` |
| EDIT | `Write`, `Edit` |
| REPORT | `Write` only. The hook runner limits it to creating new files in the agent's own report directory. |
| SHELL | `Bash` |
| VSDD | `Bash`, limited by the hook runner to single calls of the `vsdd` tool (`tool_only` in the agent's wall row), in the command groups the row grants. It is a way to finish a step with a check, not a shell: no pipe, no redirect, no other program. |
| SPAWN | `Agent`. The hook runner limits the callee to `vsdd-factory:spike-runner`. A workflow step never has this tool, whatever the file says. |
| RESULT | `StructuredOutput`. Listed defensively so that an explicit tool list cannot hide the tool a workflow step returns its result with (probe V2). |
| WEB | `WebSearch`, `WebFetch` |
| DOCS | Context7: `resolve-library-id`, `query-docs` |
| LOOKUP | Perplexity: `perplexity_ask`, `perplexity_search` |
| DEEP | Perplexity: `perplexity_research`, `perplexity_reason`. Tavily: `tavily_search`, `tavily_extract`, `tavily_crawl`, `tavily_map`, `tavily_research` |
| BROWSER | Playwright: `browser_navigate`, `browser_navigate_back`, `browser_snapshot`, `browser_take_screenshot`, `browser_click`, `browser_type`, `browser_fill_form`, `browser_select_option`, `browser_hover`, `browser_press_key`, `browser_wait_for`, `browser_resize`, `browser_console_messages`, `browser_close` |
| STORYBOOK | Storybook MCP: `list-all-documentation`, `get-documentation`, `get-documentation-for-story`, `get-storybook-story-instructions`, `preview-stories` |

## Who gets what

| Agent | Tool sets | What it deliberately lacks |
|---|---|---|
| `product-owner` | READ, EDIT, VSDD, SPAWN, RESULT, WEB, LOOKUP | No general shell: it writes documents. It ends a fix step with `vsdd gate spec --only <part>`. |
| `architect` | READ, EDIT, VSDD, SPAWN, RESULT, WEB, LOOKUP, DOCS | No general shell: an experiment is a spike. It ends the delta step with `vsdd feature route`, and a fix step with `vsdd gate spec --only <part>`. |
| `ux-designer` | READ, EDIT, VSDD, SPAWN, RESULT, WEB, STORYBOOK | No general shell. |
| `story-writer` | READ, EDIT, VSDD, SPAWN, RESULT | No web: stories derive from the specs only. It ends a story draft with `vsdd gate plan --story <id>`, which admits the story when it passes. |
| `codebase-analyzer` | READ, EDIT, SHELL, SPAWN, RESULT, DOCS | No web search: its claims must come from the code. |
| `researcher` | READ, EDIT, SPAWN, RESULT, WEB, LOOKUP, DEEP, DOCS | No shell: it reads untrusted web content, so it cannot run commands. |
| `spike-runner` | READ, EDIT, SHELL, RESULT, WEB, LOOKUP, DOCS | No SPAWN: a spike cannot start a spike. |
| `test-writer` | READ, EDIT, SHELL, SPAWN, RESULT, DOCS, STORYBOOK | |
| `implementer` | READ, EDIT, SHELL, SPAWN, RESULT, DOCS, STORYBOOK | |
| `verifier` | READ, EDIT, SHELL, SPAWN, RESULT, DOCS | |
| `demo-recorder` | READ, EDIT, SHELL, SPAWN, RESULT, BROWSER | |
| `technical-writer` | READ, EDIT, SHELL, SPAWN, RESULT | |
| `devops` | READ, EDIT, SHELL, SPAWN, RESULT, DOCS | |
| `adversary` | READ, REPORT, SPAWN, RESULT | No shell, no Edit, no web. Read-only by tool list, except for its one report file. |
| `code-reviewer` | READ, REPORT, SPAWN, RESULT | The same as the adversary. A spike runner it dispatches, or one run for its spike request in a workflow, inherits its walls. |
| `security-reviewer` | READ, REPORT, SPAWN, RESULT, WEB, LOOKUP, DOCS | No shell: scanners are run by the gate tool, and it reads their output. |
| `holdout-evaluator` | `Read`, SHELL, SPAWN, RESULT, BROWSER | No `Grep` or `Glob`, so it cannot trawl the source tree. No Write: its evaluation file is written by code from its result. A spike it asks for inherits its walls. |
| `ui-reviewer` | READ, REPORT, SHELL, SPAWN, RESULT, BROWSER, STORYBOOK | No Edit. |
| `scenario-reviewer` | READ, SPAWN, RESULT | No shell, no Write, no web. It returns findings only. |
| `clerk` | SHELL, RESULT | Everything else. The hook runner lets it run nothing but `vsdd`. |

No agent has `Skill`, `AskUserQuestion`, `NotebookEdit` or the task-list tools. A subagent cannot talk to the
person, and none of the others is needed.

## Shells and spawns

The four planners have VSDD. Without it every check of their work needed a clerk, and a failed check started a
new planner with no context: a fast-route feature spent four of its six non-story agent starts on clerks. The
cost is that planners are no longer free of the Bash tool, which slightly widens what a prompt-injected
document could reach; the hook limits it to read and measure commands whose inputs come from `factory.toml`,
which no tool call can write. The researcher, which reads the most untrusted text, keeps no shell. And every
agent except the spike runner and the clerk has SPAWN, so that a walled reviewer dispatched in a normal session
asks for a spike itself, and the hook records who asked.

A "`vsdd` only" shell (BLUEPRINT.md 4.3) means the agent has the Bash tool and the hook lets it run nothing but
a single `vsdd` call. The principle for the MCP sets: deep research tools (DEEP) are held only by
`researcher`; quick lookups (LOOKUP, DOCS, WEB) go to the roles that make technology or domain claims; builders
get library documentation (DOCS) so they do not rely on memory for an API; browser tools go to the three roles
that must look at a running product. An agent that needs more than its lookups returns a spike request.
