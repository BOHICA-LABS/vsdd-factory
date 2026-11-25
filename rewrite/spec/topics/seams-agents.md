---
kind: topic
id: seams-agents
sources: [agents-skills.md#4, agents-skills.md#5, agents-skills.md#2.9]
---

# Seams of the agents and skills design

The interfaces the design provides to, and needs from, the other designs. Each need links to the thing that
now defines it.

## What the design provides

| ID | To | What |
|---|---|---|
| P-1 | all | The 20 agent type names, exactly as `vsdd-factory:<name>`, with their tiers ([[table:roster]]). |
| P-2 | pipeline | For each agent: what it must be given and the role fields it returns (each agent file's "Is given / writes / returns"); the method names ([[table:methods]]). |
| P-3 | pipeline | Reviewers and the holdout evaluator return raw findings and scores. They never return a pass or fail decision. |
| P-4 | pipeline | The spike agent `vsdd-factory:spike-runner`; the spike rule ([[topic:agent-shared-block]] carries the rule agents carry); the fact that the spike runner records its own ledger entry through `vsdd spike open` and `vsdd spike record`. The spike command is the pipeline's `spike` workflow. |
| P-5 | hooks | The read walls, write scopes, dispatch table and proposed command scopes, keyed on `agent_type`, as the content of `walls.toml` (in full under "The walls content (P-5)" below). |
| P-6 | state | Which agent type writes each path class (the write-scope rows of [[table:walls]]). Reviewers write a report only in an on-demand review; builders' record is their commit. No agent maintains an index, a count or any status. |
| P-7 | packaging | The layout `<plugin>/agents/<name>.md` (flat, 20 files), `<plugin>/skills/<name>/SKILL.md` (13), `<plugin>/methods/<name>.md` (flat), `<plugin>/assets/` (design-system starter, demo templates, scaffold files). The preview server lives in `<plugin>/skills/preview/app/`. |
| P-8 | packaging | The lint rules, listed in full under "The lint rules (P-8)" below. |
| P-9 | packaging | Reserved skill names: `status`, `setup`, `brief`, `decide`, `drive`, `research`, `review`, `author`, `retro`, `defer`, `observe`, `preview`, `export-pdf`. The initialise skill is `setup`, the status skill is `status`, the decision command is `decide`, and the driver is `drive`. `spike` is a workflow name. |
| P-10 | packaging | The MCP tool names agents list, in both spellings ([[topic:research-and-browser-tools]]), which the agent lint must accept as known tools; the bundled server list (Perplexity, Tavily, Context7, Playwright); and the probes ([[topic:risks-agents]]). |
| P-11 | packaging | The roster as data, in the format of `tests/lint/roster.toml`, with the budgets ([[table:roster]]). |

## The walls content (P-5)

The read walls, write scopes, dispatch table and proposed command scopes are keyed on `agent_type`, including
the rows for the main session and for any other type, and are the content of `walls.toml` in the format of the
hook design ([[table:walls]], [[table:command-families]]). The spike agent is `vsdd-factory:spike-runner`, its
file sets `isolation: worktree`, and the spike command is `/vsdd-factory:spike`.

## The lint rules (P-8)

**Agent files.**

- Frontmatter keys only from {`name`, `description`, `model`, `effort`, `tools`, `color`, `isolation`,
  `omitClaudeMd`}; the first five required.
- `model` and `effort` equal the agent's tier in `roster.toml` (`xhigh` everywhere except the clerk tier's
  `low`).
- `omitClaudeMd: true` on every agent.
- Reviewer files contain the severity table and the key rule byte for byte.
- Body at most 60 lines and no line over 120 characters except `description` and `tools`.
- The shared block present byte for byte; sections in order; description at most 200 characters.
- Every method named exists; every agent has a row in the hook runner's wall data and every row has an agent.
- Forbidden words (OpenClaw, dark-factory, Corverax, LiteLLM, GPT, Gemini, T1 to T3, `apply_patch`, `../../`).

**Skill files.**

- Exactly one without `disable-model-invocation: true`, and it is `status`; description at most 400 characters
  for it.
- Only documented frontmatter keys; `SKILL.md` at most 150 lines; a forked skill names an existing agent; no
  skill has a workflow's name.

**Method files.**

- At most 150 lines.

## What the design needs

**From the pipeline design.**

| ID | Need |
|---|---|
| N-PIPE-1 | The final workflow names. None may equal a reserved skill name (P-9). Each "assumed name" of the skill design must have a workflow that covers it. |
| N-PIPE-2 | Every step names an agent type from P-1. No step runs without an agent type; steps that run one command use `clerk`. Steps set `model` and `effort` from the prelude tables that the build generates from `roster.toml`. The only overrides are those the roster lists under `[overrides]`, as the row below states. |

Rest of N-PIPE-2: `step()` reads the overrides from the generated `OVERRIDE` table (`ingest/map` and
`ingest/analyze` on the builder tier); a script cannot mark one of its own.
| N-PIPE-3 | Step prompts name a method from [[table:methods]] and give absolute paths. They do not restate the procedure. Reviewer prompts follow the three prompt rules (see below). |
| N-PIPE-4 | The shared result schemas contain: a status with values for done, blocked and spike needed; a summary; an artifact list; a spike request with at least the question, why it blocks, a suggested method and a time box; and the role fields under the names given in the agent files. If a schema has a verdict field, the script computes it. |
| N-PIPE-5 | The shared wrapper runs `vsdd-factory:spike-runner` with worktree isolation for a spike request and re-runs the step with the finding. |
| N-PIPE-6 | The holdout gate (mean, minimum for must-pass scenarios, spread) is computed by the tool and the script from `scores`, as findings of category `holdout` with no location. Fixers receive `symptoms` and the cited contracts only, never scenario text. |

**From the state design.**

| ID | Need |
|---|---|
| N-STATE-1 | A glob for every path class in the walls ([[table:path-classes]]), and the configuration keys `paths.source`, `paths.tests`, `paths.docs`, `paths.ci`. Each class in the state directory must be a directory no other class shares (apart from `brief`, one file inside `spec-product`), so that write scopes can be checked by path. |
| N-STATE-2 | One canonical absolute location for spec artifacts that is the same from inside a story worktree. The old adversary needed six paragraphs to avoid reading a stale copy. |
| N-STATE-3 | Naming for review reports and evaluation reports that lets a reviewer be given a new path without reading the directory, and a CLI call that prints the next free path for an on-demand review. |
| N-STATE-4 | The spike ledger's location and entry format, written through `vsdd spike open` and `vsdd spike record`. The format of `deferred`. There is no `lessons` format: a retrospective is a research report. |
| N-STATE-5 | A path for the project's review rules file, which reviewers are given. It replaces the policy registry. |
| N-STATE-6 | No artifact that requires an agent to keep an index, a count or a version citation in step with another file. Gate evidence and diff patches are written by code, and the status is computed. |
| N-STATE-7 | The behaviour of `status` and what `setup` must create for a project to count as initialised. The templates' location and names, which method files cite. |
| N-STATE-8 | Walled content must not surface through a recursive search from the project root. Either the `holdout` class lives outside the working tree, or N-HOOK-1 covers searches. |

**From the hook design.**

| ID | Need |
|---|---|
| N-HOOK-1 | Enforcement of the read walls for `Read`, `Grep` and `Glob`, including a search whose root is an ancestor of a walled path, and for a `Bash` command that names a walled path. Enforcement of the write scopes for `Write`, `Edit` and `NotebookEdit`, including create-only for the reviewers. These rules fail closed. |
| N-HOOK-2 | The dispatch table on `PreToolUse` for `Agent`. |
| N-HOOK-3 | For `spike-runner`: a dispatch without worktree isolation is blocked; pipeline gates such as tests-before-code do not apply inside a spike worktree; no push. |
| N-HOOK-4 | The tests-before-code rule honours the split in the write-scope table: `test-writer` may write `source` only before the story's failing-test gate is recorded, and `implementer` only after. |
| N-HOOK-5 | Writes into the state directory through `Bash` (redirection, `sed -i`, scripts) are blocked for every agent, except through the plugin CLI. |
| N-HOOK-6 | `Read` under `${CLAUDE_PLUGIN_ROOT}/methods` and `/templates`, and calls to the plugin CLI, are allowed without a permission prompt, so that an agent is never stopped from reading its own procedure. |
| N-HOOK-7 | A block message names the correct route (for example "contracts are changed by `/vsdd-factory:author contract`"). |
| N-HOOK-8 | For `holdout-evaluator`: no `git`, and no command that names a `source`, `tests` or spec path. |
| N-HOOK-9 | The browser tools can open `file:` addresses, which reads a local file without the `Read` tool. For the three agents with the BROWSER set, a `browser_navigate` call to a `file:` address is checked against the caller's read walls like a `Read`. Every other MCP tool this design gives a walled agent cannot read project files. |

**From the packaging design.**

| ID | Need |
|---|---|
| N-PKG-1 | `<plugin>/.mcp.json` with servers keyed exactly `perplexity`, `context7`, `tavily`, `playwright`, their keys taken from user configuration. A server with no key must not break session start. |
| N-PKG-2 | A plugin CLI on the `Bash` path, named `vsdd`. Settled by the blueprint: the state design owns the pipeline-facing commands. The three things this design asked for exist, as the row below states. |

Rest of N-PKG-2: they exist as `vsdd gate <name>` (runs the project's gate commands and writes the result to
`evidence`; it is the only such command), `vsdd review diff` and the diff patch the survey and the green gate
write, and `vsdd review path <lens>` for an on-demand review (N-STATE-3). There is no `vsdd worktree`,
`vsdd pr-merge` or `vsdd cache`. `vsdd validate` must also cover identifiers that do not resolve and traces
that are missing, which the old consistency skills checked.
| N-PKG-3 | The lint rules of P-8 in the test suite, and the probes run before the agent files are written. |
| N-PKG-4 | The tools three skills drive: the observability command (`observe`; `vsdd obs`), a Markdown-to-PDF converter (`export-pdf`), and a diagram renderer (`author diagram`). The last two were missing from the old plugin and are not in the packaging draft. The contributor documentation receives the content of the old `writing-skills`. |

## Three rules for prompts

Hooks cannot check these and workflow authors must follow them (need N-PIPE-3):

- A reviewer's prompt contains the method, target paths, evidence paths, scope and the items to account for
  (and a report path in an on-demand review), and nothing else; it never contains a builder's summary or an
  earlier finding, with one stated exception: from the second pass of a convergence loop it carries claims as
  structured data (the finding key, the path and line range of its location but never the quote, `fixed` or
  `disputed`, and the commit) with none of the fixer's free text, and a wave reviewer receives the findings
  that story reviews deferred as scope to check, not as conclusions.
- No prompt to an agent that may not read `holdout` contains text from a walled finding (one from the scenario
  review, or located under `.factory/holdout/`), which a universal harness test checks.
- When a finding is resolved by a deliberate decision, the decision is written into the spec or a decision
  record, so the next fresh reviewer learns it from the artifact and not from the review trail.
