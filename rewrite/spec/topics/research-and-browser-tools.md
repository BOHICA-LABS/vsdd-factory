---
kind: topic
id: research-and-browser-tools
sources: [agents-skills.md#2.10, BLUEPRINT.md#5.3, packaging.md#2.4]
---

# Research and browser tools

A plugin agent cannot declare its own MCP servers. Three facts make the tools reachable anyway.

1. MCP servers belong to the session. A session gets them from the plugin's `<plugin>/.mcp.json` or from the
   user's own configuration.
2. A tool from a plugin's server is named `mcp__plugin_vsdd-factory_<server>__<tool>`. The same tool from a
   server the user registered is named `mcp__<server>__<tool>`.
3. An agent with an explicit tool list gets an MCP tool only if the list names it.

## The design

The plugin bundles four servers under fixed keys: `perplexity`, `context7`, `tavily` and `playwright` (need
N-PKG-1; the `.mcp.json` itself and its key wiring belong to the packaging design). Every agent that needs a
tool lists both spellings, for example:

```
mcp__plugin_vsdd-factory_context7__resolve-library-id, mcp__context7__resolve-library-id,
mcp__plugin_vsdd-factory_context7__query-docs, mcp__context7__query-docs
```

Whichever server the session has, the name resolves; a name that does not resolve is dropped. This matters
because the platform drops the plugin's copy of a server when the user or the project already has the same
one, so the surviving tool name differs from machine to machine. A user who already runs these servers keeps
using them, and a user who has none enters the keys once in the plugin's configuration. The choice of "name
both" over "bundle only" and "user registers only" needs no setup in either case and no agent file changes
when a user moves from one to the other.

Who gets what is the table in [[topic:tool-sets]]. The principle: deep research tools (DEEP) are held only by
`researcher`; quick lookups (LOOKUP, DOCS, WEB) go to the roles that make technology or domain claims;
builders get library documentation (DOCS) so they do not rely on memory for an API; browser tools go to the
three roles that must look at a running product. An agent that needs more than its lookups returns a spike
request.

## When a server is missing

Nothing fails. The `research` method tells the researcher to prefer Perplexity, Context7 and Tavily, to fall
back to `WebSearch` and `WebFetch`, and to record which tools it used. Its result carries `sources_count` and
`tools_used`, so the workflow can refuse a report with no sources. Browser roles fall back to the Playwright
command line through `Bash`.

## Storybook

Storybook is a server that belongs to a project, not to the plugin. The `setup` skill tells the person to
register it under the name `storybook` when the project has a UI, and the four agents with the STORYBOOK set
list `mcp__storybook__<tool>`. Without it, the UI methods read Storybook's `index.json` instead.

## A limit of this choice

A project's own MCP servers (a database, a design tool) do not reach builders, because tools are listed
explicitly. This is a recorded risk ([[topic:risks-agents]]).

## Why the names are pinned at all

The packaging draft written alongside the design recommends that agents do not pin MCP tool names in `tools`,
because the names are not stable across machines. They are pinned, in both spellings, for two reasons. An
agent with no `tools` list inherits every tool in the session, including the user's unrelated servers, which
is the fixed cost and the exposure that rule 3 exists to remove. And listing both spellings is exactly what
makes an unstable name harmless. Probe V3 tests it; if it fails, the fallback is that the agents with MCP
sets, and only those, drop their `tools` list ([[topic:risks-agents]]).

The bundled Playwright server refuses `file:` addresses by default and is shipped without the option that
loosens that ([[packaging:plugin-manifest]]), so it cannot be used to read a file around an information wall;
the hook's own check of navigation addresses applies
whichever server a session ends up with, so the wall does not rest on this option alone. For the three agents
with the BROWSER set, a `browser_navigate` call to a `file:` address is checked against the caller's read
walls like a `Read` (need N-HOOK-9).
